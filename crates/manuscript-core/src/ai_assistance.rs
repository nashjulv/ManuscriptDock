use crate::{AppError, LocalizedText, Project, ProjectStore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiTask {
    DraftCoverLetter,
    DraftHighlights,
    ReviewMaterial,
    ReviewConsistency,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSource {
    pub id: String,
    pub label: LocalizedText,
    pub text: String,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiInput {
    pub task: AiTask,
    pub material_id: Option<String>,
    pub project_id: String,
    pub context_hash: String,
    pub sources: Vec<AiSource>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    pub source_id: String,
    pub quote: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftParagraph {
    pub text: String,
    pub evidence: Vec<Citation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub message: LocalizedText,
    pub evidence: Vec<Citation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiOutput {
    pub paragraphs: Vec<DraftParagraph>,
    pub findings: Vec<Finding>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiRun {
    pub id: String,
    pub task: AiTask,
    pub material_id: Option<String>,
    pub context_hash: String,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub created_at: u64,
    pub sources: Vec<(String, String)>,
    pub output: Option<AiOutput>,
    pub error_code: Option<String>,
    pub accepted_path: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub current: bool,
}
pub fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn source(id: &str, label: LocalizedText, text: String) -> AiSource {
    AiSource {
        id: id.into(),
        label,
        sha256: hash(text.as_bytes()),
        text,
    }
}
pub fn prepare(
    store: &ProjectStore,
    project: &Project,
    task: AiTask,
    material_id: Option<String>,
) -> Result<AiInput, AppError> {
    let target = project
        .target
        .as_ref()
        .ok_or_else(|| AppError::new("TARGET_REQUIRED", true))?;
    let journal = crate::catalog()?
        .journals
        .into_iter()
        .find(|j| j.id == target.journal_id)
        .ok_or_else(|| AppError::new("JOURNAL_NOT_FOUND", true))?;
    let rules = crate::rules_for(&target.journal_id)?;
    let tasks = crate::material_drafts::list_material_tasks(store, project)?;
    if matches!(task, AiTask::DraftCoverLetter | AiTask::DraftHighlights) {
        let id = if task == AiTask::DraftCoverLetter {
            "cover_letter"
        } else {
            "highlights"
        };
        if material_id.as_deref() != Some(id)
            || !tasks.iter().any(|item| {
                item.id == id && item.existing_file_path.is_none() && item.status != "confirmed"
            })
        {
            return Err(AppError::new("MATERIAL_NOT_GENERATABLE", true));
        }
        if project
            .facts
            .title
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
            || project
                .facts
                .abstract_text
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
        {
            return Err(AppError::new("AI_SOURCE_REQUIRED", true));
        }
    }
    let facts = serde_json::json!({"title":project.facts.title,"abstract":project.facts.abstract_text,"keywords":project.facts.keywords,"authors":project.facts.authors,"affiliations":project.facts.affiliations,"correspondingEmail":project.facts.corresponding_email,"funding":project.facts.funding,"conflictOfInterest":project.facts.conflict_of_interest,"dataAvailability":project.facts.data_availability,"highlights":project.facts.highlights,"authorConfirmedFields":project.facts.author_confirmed_fields});
    let mut sources = vec![
        source(
            "manuscript-facts",
            LocalizedText::new(
                "稿件摘要与作者填写的信息（不含主稿全文）",
                "Manuscript abstract and author-entered facts (not the full manuscript)",
            ),
            serde_json::to_string_pretty(&facts)
                .map_err(|_| AppError::new("PROJECT_INVALID", false))?,
        ),
        source(
            "journal-requirements",
            LocalizedText::new(
                "目标期刊与内置官方要求",
                "Target journal and bundled official requirements",
            ),
            serde_json::to_string_pretty(
                &serde_json::json!({"journal":journal,"target":target,"rules":rules}),
            )
            .map_err(|_| AppError::new("PROJECT_INVALID", false))?,
        ),
    ];
    {
        let root = store.package_workspace_path(&project.id)?;
        for item in tasks.iter().filter(|item| {
            task == AiTask::ReviewConsistency || material_id.as_deref() == Some(&item.id)
        }) {
            if let Some(relative) = &item.review_path {
                let path = crate::package_workspace::workspace_path(&root, relative)?;
                if fs::metadata(&path)
                    .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?
                    .len()
                    > 32_000_000
                {
                    return Err(AppError::new("AI_INPUT_LIMIT", true));
                }
                let before =
                    fs::read(&path).map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
                let text = crate::documents::material_text(&path)?;
                let after = fs::read(&path).map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
                if before != after {
                    return Err(AppError::new("AI_CONTEXT_CHANGED", true));
                }
                let mut content =
                    source(&format!("material-{}", item.id), item.label.clone(), text);
                // Bind reviews to file bytes, including edits that preserve extracted text.
                content.sha256 = hash(&before);
                sources.push(content);
            }
        }
        if matches!(task, AiTask::ReviewMaterial | AiTask::ReviewConsistency) && sources.len() < 3 {
            return Err(AppError::new("AI_SOURCE_REQUIRED", true));
        }
    }
    let text_size: usize = sources.iter().map(|s| s.text.len()).sum();
    if text_size > 100_000 {
        return Err(AppError::new("AI_INPUT_LIMIT", true));
    }
    let binding = serde_json::to_vec(&(
        &task,
        &material_id,
        &project.active_source.sha256,
        &project.facts,
        &sources,
        crate::rules_hash_for(&target.journal_id)?,
    ))
    .map_err(|_| AppError::new("PROJECT_INVALID", false))?;
    Ok(AiInput {
        task,
        material_id,
        project_id: project.id.clone(),
        context_hash: hash(&binding),
        sources,
    })
}
pub fn validate_output(input: &AiInput, output: &AiOutput) -> Result<(), AppError> {
    let bad = || AppError::new("AI_OUTPUT_INVALID", true);
    let cite = |evidence: &[Citation]| {
        evidence.len() <= 8
            && !evidence.is_empty()
            && evidence.iter().all(|citation| {
                citation.quote.trim().chars().count() >= 4
                    && citation.quote.len() <= 4000
                    && input
                        .sources
                        .iter()
                        .any(|s| s.id == citation.source_id && s.text.contains(&citation.quote))
            })
    };
    let drafting = matches!(
        input.task,
        AiTask::DraftCoverLetter | AiTask::DraftHighlights
    );
    if output.findings.len() > 20
        || output.paragraphs.len() > 20
        || (drafting && output.paragraphs.is_empty())
        || (!drafting && !output.paragraphs.is_empty())
    {
        return Err(bad());
    }
    for p in &output.paragraphs {
        if p.text.trim().is_empty() || p.text.len() > 4000 || !cite(&p.evidence) {
            return Err(bad());
        }
    }
    for f in &output.findings {
        if f.message.zh_cn.trim().is_empty()
            || f.message.en.trim().is_empty()
            || f.message.zh_cn.len() > 4000
            || f.message.en.len() > 4000
            || !cite(&f.evidence)
        {
            return Err(bad());
        }
    }
    if input.task == AiTask::DraftHighlights
        && (!(3..=5).contains(&output.paragraphs.len())
            || output
                .paragraphs
                .iter()
                .any(|p| p.text.chars().count() > 85))
    {
        return Err(bad());
    }
    Ok(())
}
fn directory(store: &ProjectStore, project: &Project) -> PathBuf {
    store.project_dir(&project.id).join("ai-assistance")
}
fn record_path(store: &ProjectStore, project: &Project, id: &str) -> Result<PathBuf, AppError> {
    uuid::Uuid::parse_str(id).map_err(|_| AppError::new("AI_RUN_NOT_FOUND", true))?;
    Ok(directory(store, project).join(format!("{id}.json")))
}
pub fn begin(
    store: &ProjectStore,
    project: &Project,
    input: &AiInput,
    id: &str,
    provider: &str,
    model: &str,
) -> Result<AiRun, AppError> {
    let run = AiRun {
        id: id.into(),
        task: input.task.clone(),
        material_id: input.material_id.clone(),
        context_hash: input.context_hash.clone(),
        provider: provider.into(),
        model: model.into(),
        status: "started".into(),
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        sources: input
            .sources
            .iter()
            .map(|s| (s.id.clone(), s.sha256.clone()))
            .collect(),
        output: None,
        error_code: None,
        accepted_path: None,
        input_tokens: None,
        output_tokens: None,
        current: true,
    };
    fs::create_dir_all(directory(store, project))
        .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(record_path(store, project, id)?)
        .map_err(|_| AppError::new("AI_RUN_ALREADY_STARTED", true))?;
    file.write_all(
        &serde_json::to_vec_pretty(&run).map_err(|_| AppError::new("PROJECT_INVALID", false))?,
    )
    .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
    file.sync_all()
        .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
    Ok(run)
}
pub fn save_run(store: &ProjectStore, project: &Project, run: &AiRun) -> Result<(), AppError> {
    crate::projects::atomic_write(
        &record_path(store, project, &run.id)?,
        &serde_json::to_vec_pretty(run).map_err(|_| AppError::new("PROJECT_INVALID", false))?,
    )
}
pub fn history(store: &ProjectStore, project: &Project) -> Result<Vec<AiRun>, AppError> {
    let dir = directory(store, project);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut runs = Vec::new();
    for entry in fs::read_dir(dir).map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))? {
        let path = entry
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?
            .path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let mut run: AiRun = serde_json::from_slice(
            &fs::read(path).map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?,
        )
        .map_err(|_| AppError::new("PROJECT_INVALID", false))?;
        run.current = prepare(store, project, run.task.clone(), run.material_id.clone())
            .is_ok_and(|input| input.context_hash == run.context_hash);
        runs.push(run);
    }
    runs.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| b.id.cmp(&a.id))
    });
    Ok(runs)
}
pub fn accept(store: &ProjectStore, project: &Project, id: &str) -> Result<String, AppError> {
    let _guard = store.acquire_file_lock("material-generation", &project.id)?;
    let mut run: AiRun = serde_json::from_slice(
        &fs::read(record_path(store, project, id)?)
            .map_err(|_| AppError::new("AI_RUN_NOT_FOUND", true))?,
    )
    .map_err(|_| AppError::new("PROJECT_INVALID", false))?;
    if run.status != "succeeded" || run.accepted_path.is_some() {
        return Err(AppError::new("AI_DRAFT_UNAVAILABLE", true));
    }
    if !matches!(run.task, AiTask::DraftCoverLetter | AiTask::DraftHighlights) {
        return Err(AppError::new("AI_DRAFT_UNAVAILABLE", true));
    }
    let input = prepare(store, project, run.task.clone(), run.material_id.clone())?;
    if input.context_hash != run.context_hash {
        return Err(AppError::new("AI_CONTEXT_CHANGED", true));
    }
    let output = run
        .output
        .as_ref()
        .ok_or_else(|| AppError::new("AI_OUTPUT_INVALID", true))?;
    validate_output(&input, output)?;
    let task_id = run
        .material_id
        .as_deref()
        .ok_or_else(|| AppError::new("AI_OUTPUT_INVALID", true))?;
    let item = crate::material_drafts::list_material_tasks(store, project)?
        .into_iter()
        .find(|t| t.id == task_id)
        .ok_or_else(|| AppError::new("AI_DRAFT_UNAVAILABLE", true))?;
    let root = crate::package_workspace::workspace_root(store, &project.id)?;
    let destination = crate::package_workspace::workspace_path(&root, &item.relative_path)?;
    let temp = directory(store, project).join(format!("{}.docx", uuid::Uuid::new_v4()));
    let result = (|| {
        crate::compiler::write_docx(
            &temp,
            &item.label.en,
            &output
                .paragraphs
                .iter()
                .map(|p| p.text.clone())
                .collect::<Vec<_>>(),
        )?;
        crate::package_workspace::copy_new(&temp, &destination)
    })();
    let _ = fs::remove_file(temp);
    result?;
    crate::material_review::track_generated(store, project, task_id, &item.relative_path)?;
    run.accepted_path = Some(item.relative_path.clone());
    save_run(store, project, &run)?;
    Ok(item.relative_path)
}
