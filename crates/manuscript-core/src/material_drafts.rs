use crate::{AppError, LocalizedText, Project, ProjectStore};
use serde::Serialize;
use std::{fs, path::Path};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialTask {
    pub id: String,
    pub label: LocalizedText,
    pub description: LocalizedText,
    pub relative_path: String,
    pub required: bool,
    pub status: String,
    pub can_generate: bool,
    pub can_generate_template: bool,
    pub template_present: bool,
    pub existing_file_path: Option<String>,
    pub review_path: Option<String>,
    pub manuscript_kind: Option<String>,
    pub provided_file_name: Option<String>,
}

fn planned_tasks(project: &Project) -> Result<Vec<(MaterialTask, Vec<String>)>, AppError> {
    let target = project
        .target
        .as_ref()
        .ok_or_else(|| AppError::new("TARGET_REQUIRED", true))?;
    let rules = crate::rules_for(&target.journal_id)?;
    let preparation = crate::prepare(project)?;
    let confirmed = |key: &str| {
        rules
            .iter()
            .filter(|r| r.fact_key.as_deref() == Some(key))
            .any(|rule| {
                preparation
                    .ready_items
                    .iter()
                    .any(|item| item.requirement_id == rule.id)
            })
    };
    let task = |id: &str,
                label: LocalizedText,
                description: LocalizedText,
                path: &str,
                required: bool,
                possible: bool| MaterialTask {
        id: id.into(),
        label,
        description,
        relative_path: path.into(),
        required,
        status: if possible {
            "ready_to_generate"
        } else {
            "manual_required"
        }
        .into(),
        can_generate: possible,
        can_generate_template: id != "manuscript",
        template_present: false,
        existing_file_path: None,
        review_path: None,
        manuscript_kind: None,
        provided_file_name: None,
    };
    let needs_anonymous = rules
        .iter()
        .any(|r| r.required_file_kind.as_deref() == Some("anonymized_manuscript"));
    let main_ready = if needs_anonymous {
        preparation
            .ready_items
            .iter()
            .any(|item| item.requirement_id.ends_with("anonymized-manuscript"))
    } else {
        project.active_source.format == "docx"
            || project
                .materials
                .iter()
                .any(|m| m.included && m.kind == "editable_manuscript")
    };
    let mut main = task("manuscript", LocalizedText::new(if needs_anonymous { "匿名主稿 DOCX" } else { "可编辑主稿 DOCX" }, if needs_anonymous { "Anonymized manuscript DOCX" } else { "Editable manuscript DOCX" }), LocalizedText::new("主稿须由作者提供和核对，不能自动生成或匿名化。", "The author must supply and verify the manuscript. It cannot be generated or anonymized automatically."), "submission/manuscript.docx", true, false);
    if main_ready {
        main.status = "ready".into();
    }
    let kind = if needs_anonymous {
        "anonymized_manuscript"
    } else {
        "editable_manuscript"
    };
    main.manuscript_kind = Some(kind.into());
    main.provided_file_name = project
        .materials
        .iter()
        .find(|material| material.included && material.kind == kind)
        .map(|material| material.file_name.clone())
        .or_else(|| {
            (!needs_anonymous && project.active_source.format == "docx")
                .then(|| project.active_source.file_name.clone())
        });
    let title = project.facts.title.as_deref().unwrap_or("");
    let journal = crate::catalog()?
        .journals
        .into_iter()
        .find(|j| j.id == target.journal_id)
        .ok_or_else(|| AppError::new("JOURNAL_NOT_FOUND", true))?;
    let mut tasks = vec![(main, vec![]), (
        task("title_page", LocalizedText::new("标题页", "Title page"), LocalizedText::new("确认标题、作者、单位和通讯邮箱后可生成；生成后仍需作者核对。", "Confirm the title, authors, affiliations, and corresponding email to generate a draft for author review."), "author-tools/title-page-DRAFT.docx", needs_anonymous, confirmed("title") && confirmed("authors") && confirmed("affiliations") && confirmed("corresponding_email")),
        crate::compiler::title_page_paragraphs(project)
    ), (
        task("cover_letter", LocalizedText::new("投稿信草稿", "Cover letter draft"), LocalizedText::new("可按论文标题和目标期刊生成投稿信初稿；创新性和投稿承诺仍需作者补充。", "Generate an initial cover letter from the manuscript title and target journal. The author must complete novelty and submission declarations."), "author-tools/cover-letter-DRAFT.docx", false, !title.trim().is_empty()),
        vec!["Dear Editor,".into(), format!("Please consider our manuscript, \"{title}\", for {}.", journal.display_name), "[AUTHOR REVIEW REQUIRED: add the study's contribution and confirm any submission declarations.]".into(), "Sincerely,".into(), project.facts.authors.first().cloned().unwrap_or_else(|| "[CORRESPONDING AUTHOR]".into())]
    )];
    for rule in rules.iter().filter(|rule| {
        matches!(
            rule.fact_key.as_deref(),
            Some(
                "conflict_of_interest"
                    | "funding"
                    | "data_availability"
                    | "highlights"
                    | "credit_contributions"
                    | "generative_ai_disclosure"
            )
        )
    }) {
        let key = rule.fact_key.as_deref().unwrap_or_default();
        let value = crate::projects::confirmed_fact_value(&project.facts, key).unwrap_or_default();
        tasks.push((
            task(
                key,
                rule.label.clone(),
                rule.description.clone(),
                &format!("author-tools/{}-DRAFT.docx", key.replace('_', "-")),
                rule.required,
                confirmed(key),
            ),
            value.split('\u{1f}').map(String::from).collect(),
        ));
    }
    Ok(tasks)
}

pub fn list_material_tasks(
    store: &ProjectStore,
    project: &Project,
) -> Result<Vec<MaterialTask>, AppError> {
    let root = store.package_workspace_path(&project.id)?;
    planned_tasks(project)?
        .into_iter()
        .map(|(mut task, _)| {
            // A workspace file is a candidate, never proof of author verification.
            if task.id == "manuscript"
                && crate::package_workspace::workspace_path(&root, &task.relative_path)?.is_file()
            {
                task.existing_file_path = Some(task.relative_path.clone());
            }
            if task.can_generate_template {
                task.template_present =
                    crate::package_workspace::workspace_path(&root, &template_path(&task))?
                        .is_file();
                task.can_generate_template = !task.template_present;
            }
            if task.id != "manuscript" {
                let name = if task.id == "conflict_of_interest" {
                    "declaration-of-competing-interests".into()
                } else {
                    task.id.replace('_', "-")
                };
                for relative in [
                    task.relative_path.clone(),
                    format!("submission/{name}.docx"),
                ] {
                    if crate::package_workspace::workspace_path(&root, &relative)?.is_file() {
                        task.status = "draft_present".into();
                        task.can_generate = false;
                        task.can_generate_template = false;
                        task.existing_file_path = Some(relative);
                        break;
                    }
                }
            }
            if task.id != "manuscript" {
                task.review_path = task
                    .existing_file_path
                    .clone()
                    .or_else(|| task.template_present.then(|| template_path(&task)));
                if let Some(relative) = &task.review_path {
                    if let Some(status) =
                        crate::material_review::state(store, project, &task.id, relative)?
                    {
                        task.status = status;
                        if task.status == "confirmed" {
                            task.can_generate = false;
                            task.can_generate_template = false;
                        }
                    }
                }
            }
            Ok(task)
        })
        .collect()
}

fn template_path(task: &MaterialTask) -> String {
    task.relative_path.replace("-DRAFT.docx", "-TEMPLATE.docx")
}

pub fn generate_material_templates(
    store: &ProjectStore,
    project: &Project,
    ids: &[String],
) -> Result<Vec<String>, AppError> {
    let _guard = store.acquire_file_lock("material-generation", &project.id)?;
    let tasks = list_material_tasks(store, project)?;
    if ids.iter().any(|id| {
        !tasks
            .iter()
            .any(|task| task.id == *id && task.can_generate_template)
    }) {
        return Err(AppError::new("MATERIAL_NOT_GENERATABLE", true));
    }
    let mut paths = Vec::new();
    for task in tasks
        .iter()
        .filter(|task| task.can_generate_template && (ids.is_empty() || ids.contains(&task.id)))
    {
        paths.extend(write_material_template(store, project, &task.id)?);
    }
    Ok(paths)
}

pub fn generate_material_template(
    store: &ProjectStore,
    project: &Project,
    material_id: &str,
) -> Result<Vec<String>, AppError> {
    generate_material_templates(store, project, &[material_id.into()])
}

fn write_material_template(
    store: &ProjectStore,
    project: &Project,
    material_id: &str,
) -> Result<Vec<String>, AppError> {
    let task = list_material_tasks(store, project)?
        .into_iter()
        .find(|task| task.id == material_id && task.can_generate_template)
        .ok_or_else(|| AppError::new("MATERIAL_NOT_GENERATABLE", true))?;
    let root = crate::package_workspace::workspace_root(store, &project.id)?;
    let relative = template_path(&task);
    let target = project
        .target
        .as_ref()
        .ok_or_else(|| AppError::new("TARGET_REQUIRED", true))?;
    let journal = crate::catalog()?
        .journals
        .into_iter()
        .find(|journal| journal.id == target.journal_id)
        .ok_or_else(|| AppError::new("JOURNAL_NOT_FOUND", true))?;
    let mut paragraphs = vec![
        format!("Target journal: {}", journal.display_name),
        "TEMPLATE ONLY — incomplete; not ready for submission.".into(),
        format!("Requirement: {}", task.description.en),
    ];
    let fields: &[&str] = match task.id.as_str() {
        "title_page" => &["Manuscript title", "Author names and affiliations", "Corresponding author and email"],
        "cover_letter" => &["Editor and journal", "Manuscript title and article type", "Study contribution and relevance", "Author-confirmed submission declarations", "Corresponding author and signature"],
        "highlights" => &["Highlight 1", "Highlight 2", "Highlight 3"],
        "credit_contributions" => &["Author name and verified CRediT roles for each author"],
        "funding" => &["Confirmed funding sources, grant identifiers, and funder role; explicitly state no funding only if confirmed"],
        "conflict_of_interest" => &["Author-confirmed competing interests for all authors"],
        "data_availability" => &["Data location, access conditions, repository identifiers, or justified restrictions"],
        "generative_ai_disclosure" => &["Actual AI tools used, purpose, and author review; confirm if no disclosure is applicable"],
        _ => return Err(AppError::new("MATERIAL_NOT_GENERATABLE", true)),
    };
    paragraphs.extend(
        fields
            .iter()
            .map(|field| format!("[AUTHOR TO COMPLETE: {field}]")),
    );
    write_new_draft(
        &crate::package_workspace::workspace_path(&root, &relative)?,
        &format!("{} — Template", task.label.en),
        &paragraphs,
    )?;
    crate::material_review::track_generated(store, project, material_id, &relative)?;
    Ok(vec![relative])
}

pub fn generate_materials(
    store: &ProjectStore,
    project: &Project,
    material_ids: &[String],
) -> Result<Vec<String>, AppError> {
    let _guard = store.acquire_file_lock("material-generation", &project.id)?;
    let tasks = list_material_tasks(store, project)?;
    if material_ids
        .iter()
        .any(|id| !tasks.iter().any(|task| task.id == *id && task.can_generate))
    {
        return Err(AppError::new("MATERIAL_NOT_GENERATABLE", true));
    }
    let root = crate::package_workspace::workspace_root(store, &project.id)?;
    let recipes = planned_tasks(project)?;
    let mut written = Vec::new();
    for task in tasks.iter().filter(|task| {
        task.can_generate && (material_ids.is_empty() || material_ids.contains(&task.id))
    }) {
        let paragraphs = &recipes
            .iter()
            .find(|(recipe, _)| recipe.id == task.id)
            .ok_or_else(|| AppError::new("MATERIAL_NOT_GENERATABLE", true))?
            .1;
        let destination = crate::package_workspace::workspace_path(&root, &task.relative_path)?;
        write_new_draft(&destination, &task.label.en, paragraphs)?;
        crate::material_review::track_generated(store, project, &task.id, &task.relative_path)?;
        written.push(task.relative_path.clone());
    }
    Ok(written)
}

fn write_new_draft(path: &Path, title: &str, paragraphs: &[String]) -> Result<(), AppError> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::new("WORKSPACE_PATH_INVALID", true))?;
    let temporary = parent.join(format!(".{}.docx", uuid::Uuid::new_v4()));
    let result = (|| {
        crate::compiler::write_docx(&temporary, title, paragraphs)?;
        crate::package_workspace::copy_new(&temporary, path)
    })();
    let _ = fs::remove_file(temporary);
    result
}
