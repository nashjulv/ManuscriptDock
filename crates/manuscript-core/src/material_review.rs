use crate::{AppError, LocalizedText, Project, ProjectStore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRecord {
    pub material_id: String,
    pub relative_path: String,
    pub sha256: String,
    pub context_hash: String,
    pub confirmed_at: u64,
    pub snapshot_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialCheck {
    pub material_id: String,
    pub relative_path: String,
    pub sha256: String,
    pub context_hash: String,
    pub issues: Vec<LocalizedText>,
    pub author_checks: Vec<LocalizedText>,
}

fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn context(project: &Project) -> Result<String, AppError> {
    let rules = project
        .target
        .as_ref()
        .map(|target| crate::rules_hash_for(&target.journal_id))
        .transpose()?;
    Ok(hash(
        &serde_json::to_vec(&(
            &project.target,
            &project.active_source.sha256,
            &project.facts,
            rules,
        ))
        .map_err(|_| AppError::new("PROJECT_INVALID", false))?,
    ))
}
fn directory(store: &ProjectStore, project: &Project, id: &str) -> Result<PathBuf, AppError> {
    let target = project
        .target
        .as_ref()
        .ok_or_else(|| AppError::new("TARGET_REQUIRED", true))?;
    // IDs are hashed as well as target IDs; presentation input never forms a path component.
    Ok(store
        .project_dir(&project.id)
        .join("material-review")
        .join(hash(target.journal_id.as_bytes()))
        .join(hash(id.as_bytes())))
}
fn read_bytes(path: &Path) -> Result<Vec<u8>, AppError> {
    let file = fs::File::open(path).map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
    let mut bytes = Vec::new();
    file.take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err(AppError::new("LIMIT_EXCEEDED", true));
    }
    Ok(bytes)
}
fn record(
    store: &ProjectStore,
    project: &Project,
    id: &str,
) -> Result<Option<ReviewRecord>, AppError> {
    let path = directory(store, project, id)?.join("confirmed.json");
    if !path.exists() {
        return Ok(None);
    }
    serde_json::from_slice(&read_bytes(&path)?)
        .map(Some)
        .map_err(|_| AppError::new("PROJECT_INVALID", false))
}
pub(crate) fn track_generated(
    store: &ProjectStore,
    project: &Project,
    id: &str,
    relative: &str,
) -> Result<(), AppError> {
    let root = store.package_workspace_path(&project.id)?;
    let path = crate::package_workspace::workspace_path(&root, relative)?;
    crate::projects::atomic_write(
        &directory(store, project, id)?.join(format!("baseline-{}.txt", hash(relative.as_bytes()))),
        hash(&read_bytes(&path)?).as_bytes(),
    )
}
pub(crate) fn state(
    store: &ProjectStore,
    project: &Project,
    id: &str,
    relative: &str,
) -> Result<Option<String>, AppError> {
    let root = store.package_workspace_path(&project.id)?;
    let current = hash(&read_bytes(&crate::package_workspace::workspace_path(
        &root, relative,
    )?)?);
    if let Some(record) = record(store, project, id)? {
        return Ok(Some(
            if record.relative_path == relative
                && record.sha256 == current
                && record.context_hash == context(project)?
            {
                "confirmed"
            } else {
                "modified"
            }
            .into(),
        ));
    }
    let baseline =
        directory(store, project, id)?.join(format!("baseline-{}.txt", hash(relative.as_bytes())));
    if baseline.exists() && read_bytes(&baseline)? != current.as_bytes() {
        return Ok(Some("modified".into()));
    }
    Ok(None)
}
fn task(
    store: &ProjectStore,
    project: &Project,
    id: &str,
) -> Result<crate::material_drafts::MaterialTask, AppError> {
    crate::material_drafts::list_material_tasks(store, project)?
        .into_iter()
        .find(|task| task.id == id && task.review_path.is_some())
        .ok_or_else(|| AppError::new("MATERIAL_REVIEW_UNAVAILABLE", true))
}
pub fn check(store: &ProjectStore, project: &Project, id: &str) -> Result<MaterialCheck, AppError> {
    let task = task(store, project, id)?;
    let relative = task
        .review_path
        .clone()
        .ok_or_else(|| AppError::new("MATERIAL_REVIEW_UNAVAILABLE", true))?;
    let root = store.package_workspace_path(&project.id)?;
    let path = crate::package_workspace::workspace_path(&root, &relative)?;
    let bytes = read_bytes(&path)?;
    let mut issues = Vec::new();
    // Inspect exactly the captured bytes, so concurrent external edits cannot mix checked text and hash.
    let temporary =
        directory(store, project, id)?.join(format!("check-{}.docx", uuid::Uuid::new_v4()));
    crate::projects::atomic_write(&temporary, &bytes)?;
    let inspection = crate::inspect_docx(&temporary);
    let _ = fs::remove_file(&temporary);
    if inspection.is_err() {
        issues.push(LocalizedText::new(
            "文件不是可读取的 DOCX，请修复后重新检查。",
            "The file is not a readable DOCX. Repair it and check again.",
        ));
    } else {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes))
            .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
        let mut xml = String::new();
        archive
            .by_name("word/document.xml")
            .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?
            .take(32 * 1024 * 1024)
            .read_to_string(&mut xml)
            .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
        let mut lines = crate::documents::paragraphs(&xml);
        let text = lines.join("\n").to_lowercase();
        if [
            "[author",
            "[corresponding",
            "[insert",
            "[todo",
            "[tbd",
            "待填写",
            "待补充",
            "template only",
            "author review required",
        ]
        .iter()
        .any(|marker| text.contains(marker))
        {
            issues.push(LocalizedText::new("仍含待填占位符或模板说明，请填写并删除这些提示后重新检查。", "Placeholders or template instructions remain. Complete them and remove the instructions before checking again."));
        }
        lines.retain(|line| {
            !line.trim().is_empty()
                && line != &task.label.en
                && line != &task.label.zh_cn
                && !line.starts_with("Target journal:")
                && !line.starts_with("Requirement:")
                && !line.ends_with("— Template")
                && !(id == "highlights"
                    && matches!(
                        line.trim().to_lowercase().as_str(),
                        "highlights" | "article highlights" | "highlights draft" | "研究亮点"
                    ))
        });
        if lines
            .iter()
            .map(|line| line.trim().chars().count())
            .sum::<usize>()
            < 10
        {
            issues.push(LocalizedText::new(
                "缺少有效正文，请补充材料内容。",
                "Substantive content is missing. Complete the material text.",
            ));
        }
        if id == "title_page"
            && !lines
                .iter()
                .any(|line| line.contains('@') && line.contains('.'))
        {
            issues.push(LocalizedText::new(
                "标题页缺少可识别的通讯邮箱。",
                "The title page is missing a recognizable corresponding email.",
            ));
        }
        if let Some(target) = &project.target {
            for rule in crate::rules_for(&target.journal_id)?
                .iter()
                .filter(|rule| rule.fact_key.as_deref() == Some(id))
            {
                if let Some(c) = &rule.constraint {
                    if c.min_items.is_some_and(|min| lines.len() < min)
                        || c.max_items.is_some_and(|max| lines.len() > max)
                        || c.max_chars_per_item
                            .is_some_and(|max| lines.iter().any(|line| line.chars().count() > max))
                        || c.max_words.is_some_and(|max| {
                            lines
                                .iter()
                                .map(|line| line.split_whitespace().count())
                                .sum::<usize>()
                                > max
                        })
                    {
                        issues.push(LocalizedText::new(format!("未满足期刊条目数量、长度或字数要求：{}", rule.description.zh_cn), format!("Journal item-count, length, or word-limit requirements are not met: {}", rule.description.en)));
                    }
                }
            }
        }
    }
    Ok(MaterialCheck { material_id: id.into(), relative_path: relative, sha256: hash(&bytes), context_hash: context(project)?, issues, author_checks: vec![
        LocalizedText::new("我已按目标期刊要求核对材料的必填内容、作者信息和格式。", "I have verified the required content, author details, and format against the target journal requirements."),
        LocalizedText::new("我确认研究事实与声明真实、完整，并愿意将此版本加入投稿包。", "I confirm that the research facts and declarations are accurate and complete, and approve this version for the submission package."),
    ] })
}
pub fn confirm(
    store: &ProjectStore,
    project: &Project,
    id: &str,
    expected_hash: &str,
    expected_context: &str,
    author_confirmed: bool,
) -> Result<ReviewRecord, AppError> {
    if !author_confirmed {
        return Err(AppError::new("MATERIAL_AUTHOR_CONFIRMATION_REQUIRED", true));
    }
    let _guard = store.acquire_file_lock("material-generation", &project.id)?;
    let checked = check(store, project, id)?;
    if checked.sha256 != expected_hash || checked.context_hash != expected_context {
        return Err(AppError::new("MATERIAL_REVIEW_CHANGED", true));
    }
    if !checked.issues.is_empty() {
        return Err(AppError::new("MATERIAL_REVIEW_FAILED", true));
    }
    let root = store.package_workspace_path(&project.id)?;
    let bytes = read_bytes(&crate::package_workspace::workspace_path(
        &root,
        &checked.relative_path,
    )?)?;
    if hash(&bytes) != checked.sha256 {
        return Err(AppError::new("MATERIAL_REVIEW_CHANGED", true));
    }
    if let Some(existing) = record(store, project, id)? {
        if existing.sha256 == checked.sha256
            && existing.context_hash == checked.context_hash
            && existing.relative_path == checked.relative_path
        {
            return Ok(existing);
        }
    }
    let snapshot_id = uuid::Uuid::new_v4().to_string();
    let record = ReviewRecord {
        material_id: id.into(),
        relative_path: checked.relative_path,
        sha256: checked.sha256,
        context_hash: checked.context_hash,
        confirmed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        snapshot_id,
    };
    let dir = directory(store, project, id)?;
    crate::projects::atomic_write(&dir.join(format!("{}.docx", record.snapshot_id)), &bytes)?;
    let json =
        serde_json::to_vec_pretty(&record).map_err(|_| AppError::new("PROJECT_INVALID", false))?;
    crate::projects::atomic_write(&dir.join(format!("{}.json", record.snapshot_id)), &json)?;
    crate::projects::atomic_write(&dir.join("confirmed.json"), &json)?;
    Ok(record)
}

pub(crate) fn confirmed_records(
    store: &ProjectStore,
    project: &Project,
) -> Result<Vec<ReviewRecord>, AppError> {
    if !store
        .project_dir(&project.id)
        .join("material-review")
        .exists()
    {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    for task in crate::material_drafts::list_material_tasks(store, project)? {
        if task.status == "confirmed" {
            if let Some(record) = record(store, project, &task.id)? {
                records.push(record);
            }
        }
    }
    Ok(records)
}
pub fn prepare(
    store: &ProjectStore,
    project: &Project,
) -> Result<crate::PreparationView, AppError> {
    let mut view = crate::prepare(project)?;
    let records = confirmed_records(store, project)?;
    if records.is_empty() {
        return Ok(view);
    }
    let target = project
        .target
        .as_ref()
        .ok_or_else(|| AppError::new("TARGET_REQUIRED", true))?;
    let rules = crate::rules_for(&target.journal_id)?;
    let ready = |id: &str| {
        rules.iter().any(|rule| {
            rule.id == id
                && rule
                    .fact_key
                    .as_ref()
                    .is_some_and(|key| records.iter().any(|record| &record.material_id == key))
        })
    };
    for items in [&mut view.blockers, &mut view.warnings] {
        let mut remaining = Vec::new();
        for mut item in items.drain(..) {
            if ready(&item.requirement_id) {
                item.status = "ready".into();
                view.ready_items.push(item);
            } else {
                remaining.push(item);
            }
        }
        *items = remaining;
    }
    if view.blockers.is_empty() {
        view.status = crate::PreparationStatus::ReadyToExport;
        view.allowed_actions = vec!["build_draft".into(), "build_final".into()];
    }
    view.context_hash = hash(
        &serde_json::to_vec(&(&view.context_hash, &records))
            .map_err(|_| AppError::new("PROJECT_INVALID", false))?,
    );
    view.package_plan.context_hash = view.context_hash.clone();
    view.package_plan.id = format!("plan-{}", &view.context_hash[..16]);
    view.package_plan.status = if view.blockers.is_empty() {
        "ready"
    } else {
        "blocked"
    }
    .into();
    view.package_plan
        .capabilities
        .push("use_author_confirmed_material_snapshots".into());
    for record in &records {
        let name = if record.material_id == "conflict_of_interest" {
            "declaration-of-competing-interests".into()
        } else {
            record.material_id.replace('_', "-")
        };
        let relative = format!("submission/{name}.docx");
        view.package_plan
            .file_plan
            .retain(|file| file.relative_path != relative);
        view.package_plan.file_plan.push(crate::PlannedFile {
            relative_path: relative,
            operation: "copy_author_confirmed_material".into(),
            publisher_file: true,
        });
    }
    Ok(view)
}
pub(crate) fn copy_confirmed(
    store: &ProjectStore,
    project: &Project,
    staging: &Path,
) -> Result<(), AppError> {
    let records = confirmed_records(store, project)?;
    if records.is_empty() {
        return Ok(());
    }
    for record in &records {
        let bytes = read_bytes(
            &directory(store, project, &record.material_id)?
                .join(format!("{}.docx", record.snapshot_id)),
        )?;
        if hash(&bytes) != record.sha256 {
            return Err(AppError::new("MATERIAL_REVIEW_CHANGED", true));
        }
        let name = if record.material_id == "conflict_of_interest" {
            "declaration-of-competing-interests".into()
        } else {
            record.material_id.replace('_', "-")
        };
        // Only replaces freshly compiled staging artifacts, never an editable source or an older export.
        let draft = staging.join(format!("submission/{name}-DRAFT.docx"));
        if draft.exists() {
            fs::remove_file(draft).map_err(|_| AppError::new("WORKSPACE_IO_FAILED", true))?;
        }
        crate::projects::atomic_write(&staging.join(format!("submission/{name}.docx")), &bytes)?;
    }
    crate::projects::atomic_write(
        &staging.join("records/material-confirmations.json"),
        &serde_json::to_vec_pretty(&records)
            .map_err(|_| AppError::new("PROJECT_INVALID", false))?,
    )
}
