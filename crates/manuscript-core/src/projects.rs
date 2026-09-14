use crate::{
    inspect_docx, inspect_pdf, AppError, AuthorDecision, CompiledPackage, DocumentFacts,
    ExportReceipt, JobEvent, JobRecord, JobStatus, Material, Project, ProjectWorkspace,
    SourceSnapshot, TargetOrigin, TargetSelection, TaskKind, PROJECT_SCHEMA_VERSION,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone)]
pub struct ProjectStore {
    root: PathBuf,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestResult<T> {
    schema_version: u32,
    request_id: String,
    operation: String,
    result: T,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct FolderBinding {
    schema_version: u32,
    binding_id: String,
    project_id: String,
    canonical_path: PathBuf,
}

struct SourceInspection {
    sha256: String,
    size_bytes: u64,
    format: String,
    feature_profile: String,
    facts: DocumentFacts,
}

struct CleanupDirectory {
    path: PathBuf,
    armed: bool,
}

impl CleanupDirectory {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn preserve(&mut self) {
        self.armed = false;
    }
}

impl Drop for CleanupDirectory {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

struct CleanupFile {
    path: PathBuf,
    armed: bool,
}

impl CleanupFile {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn preserve(&mut self) {
        self.armed = false;
    }
}

impl Drop for CleanupFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

impl ProjectStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, AppError> {
        let root = root.into();
        fs::create_dir_all(root.join("projects"))
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
        fs::create_dir_all(root.join("packages"))
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
        fs::create_dir_all(root.join("receipts"))
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
        fs::create_dir_all(root.join("requests"))
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
        fs::create_dir_all(root.join("jobs"))
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
        fs::create_dir_all(root.join("locks"))
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
        fs::create_dir_all(root.join("folders"))
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
        Ok(Self { root })
    }

    pub fn execute_idempotent_job<T, F>(
        &self,
        request_id: &str,
        operation: &str,
        project_id: Option<&str>,
        context_hash: Option<&str>,
        action: F,
    ) -> Result<T, AppError>
    where
        T: Clone + DeserializeOwned + Serialize,
        F: FnOnce() -> Result<T, AppError>,
    {
        self.execute_idempotent(request_id, operation, || {
            self.start_job(request_id, operation, project_id, context_hash)?;
            match action() {
                Ok(result) => {
                    self.transition_job(request_id, JobStatus::Succeeded, None)?;
                    Ok(result)
                }
                Err(error) => {
                    let _ = self.transition_job(
                        request_id,
                        if error.retryable {
                            JobStatus::NeedsInput
                        } else {
                            JobStatus::Failed
                        },
                        Some(error.clone()),
                    );
                    Err(error)
                }
            }
        })
    }

    pub fn reserve_job(
        &self,
        request_id: &str,
        operation: &str,
        project_id: Option<&str>,
        context_hash: Option<&str>,
    ) -> Result<(JobRecord, bool), AppError> {
        validate_request_id(request_id)?;
        if operation.trim().is_empty() {
            return Err(AppError::new("REQUEST_ID_CONFLICT", false));
        }
        if let Some(project_id) = project_id {
            validate_id(project_id)?;
        }
        let lock = job_lock(request_id)?;
        let _guard = lock
            .lock()
            .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
        let _file_guard = self.acquire_file_lock("job", request_id)?;
        let path = self.job_path(request_id);
        if path.exists() {
            let mut job = self.read_job(request_id)?;
            if job.operation != operation
                || job.project_id.as_deref() != project_id
                || job.context_hash.as_deref() != context_hash
            {
                return Err(AppError::new("REQUEST_ID_CONFLICT", false));
            }
            if job.status == JobStatus::Succeeded && job.result.is_none() {
                if let Some(result) = self.read_request_value(request_id, operation)? {
                    job.result = Some(result);
                    self.save_job(&job)?;
                }
            }
            return Ok((job, false));
        }
        let now = now_ms()?;
        let job = JobRecord {
            schema_version: 1,
            id: request_id.to_owned(),
            project_id: project_id.map(str::to_owned),
            request_id: request_id.to_owned(),
            operation: operation.to_owned(),
            context_hash: context_hash.map(str::to_owned),
            status: JobStatus::Queued,
            cancel_requested: false,
            events: vec![JobEvent {
                seq: 1,
                phase: operation.to_owned(),
                status: JobStatus::Queued,
                created_at_unix_ms: now,
                completed_units: None,
                total_units: None,
                error: None,
            }],
            result: None,
            updated_at_unix_ms: now,
        };
        self.save_job(&job)?;
        Ok((job, true))
    }

    pub fn execute_reserved_job<T, F, N>(
        &self,
        request_id: &str,
        operation: &str,
        action: F,
        mut notify: N,
    ) -> Result<T, AppError>
    where
        T: Clone + DeserializeOwned + Serialize,
        F: FnOnce() -> Result<T, AppError>,
        N: FnMut(&JobRecord),
    {
        self.execute_idempotent(request_id, operation, || {
            let running = self.transition_job(request_id, JobStatus::Running, None)?;
            notify(&running);
            match action() {
                Ok(result) => {
                    let value = serde_json::to_value(&result)
                        .map_err(|_| AppError::new("REQUEST_RECORD_INVALID", false))?;
                    let completed = self.complete_job(request_id, value)?;
                    notify(&completed);
                    Ok(result)
                }
                Err(error) => {
                    if let Ok(failed) = self.transition_job(
                        request_id,
                        if error.retryable {
                            JobStatus::NeedsInput
                        } else {
                            JobStatus::Failed
                        },
                        Some(error.clone()),
                    ) {
                        notify(&failed);
                    }
                    Err(error)
                }
            }
        })
    }

    pub fn get_job(&self, project_id: &str, job_id: &str) -> Result<JobRecord, AppError> {
        validate_id(project_id)?;
        let job = self.read_job(job_id)?;
        if job.project_id.as_deref().is_some_and(|id| id != project_id) {
            return Err(AppError::new("JOB_NOT_FOUND", true));
        }
        Ok(job)
    }

    pub fn get_job_unscoped(&self, job_id: &str) -> Result<JobRecord, AppError> {
        self.read_job(job_id)
    }

    pub fn cancel_job(&self, project_id: &str, job_id: &str) -> Result<JobRecord, AppError> {
        validate_id(project_id)?;
        self.cancel_job_with_scope(Some(project_id), job_id)
    }

    pub fn cancel_job_unscoped(&self, job_id: &str) -> Result<JobRecord, AppError> {
        self.cancel_job_with_scope(None, job_id)
    }

    fn cancel_job_with_scope(
        &self,
        project_id: Option<&str>,
        job_id: &str,
    ) -> Result<JobRecord, AppError> {
        let lock = job_lock(job_id)?;
        let _guard = lock
            .lock()
            .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
        let _file_guard = self.acquire_file_lock("job", job_id)?;
        let mut job = self.read_job(job_id)?;
        if project_id
            .is_some_and(|project_id| job.project_id.as_deref().is_some_and(|id| id != project_id))
        {
            return Err(AppError::new("JOB_NOT_FOUND", true));
        }
        if job.status.is_terminal() {
            return Ok(job);
        }
        job.cancel_requested = true;
        append_job_event(&mut job, JobStatus::Cancelled, None)?;
        self.save_job(&job)?;
        Ok(job)
    }

    pub fn recover_interrupted_jobs(&self) -> Result<usize, AppError> {
        let mut recovered = 0;
        for entry in fs::read_dir(self.root.join("jobs"))
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?
        {
            let Ok(entry) = entry else {
                continue;
            };
            let Ok(encoded) = fs::read(entry.path()) else {
                continue;
            };
            let Ok(mut job) = serde_json::from_slice::<JobRecord>(&encoded) else {
                continue;
            };
            if matches!(job.status, JobStatus::Queued | JobStatus::Running) {
                append_job_event(&mut job, JobStatus::Interrupted, None)?;
                self.save_job(&job)?;
                recovered += 1;
            }
        }
        Ok(recovered)
    }
    pub fn execute_idempotent<T, F>(
        &self,
        request_id: &str,
        operation: &str,
        action: F,
    ) -> Result<T, AppError>
    where
        T: Clone + DeserializeOwned + Serialize,
        F: FnOnce() -> Result<T, AppError>,
    {
        validate_request_id(request_id)?;
        if operation.trim().is_empty() {
            return Err(AppError::new("REQUEST_ID_CONFLICT", false));
        }
        let lock = request_lock(request_id)?;
        let _guard = lock
            .lock()
            .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
        let _file_guard = self.acquire_file_lock("request", request_id)?;
        let record_path = self
            .root
            .join("requests")
            .join(format!("{request_id}.json"));
        if record_path.exists() {
            let encoded = fs::read(&record_path)
                .map_err(|_| AppError::new("REQUEST_RECORD_INVALID", true))?;
            let record: RequestResult<serde_json::Value> = serde_json::from_slice(&encoded)
                .map_err(|_| AppError::new("REQUEST_RECORD_INVALID", true))?;
            if record.schema_version != 1
                || record.request_id != request_id
                || record.operation != operation
            {
                return Err(AppError::new("REQUEST_ID_CONFLICT", false));
            }
            return serde_json::from_value(record.result)
                .map_err(|_| AppError::new("REQUEST_RECORD_INVALID", true));
        }
        let result = action()?;
        let record = RequestResult {
            schema_version: 1,
            request_id: request_id.to_owned(),
            operation: operation.to_owned(),
            result: result.clone(),
        };
        let encoded = serde_json::to_vec_pretty(&record)
            .map_err(|_| AppError::new("REQUEST_RECORD_INVALID", false))?;
        atomic_write(&record_path, &encoded)?;
        Ok(result)
    }

    fn read_request_value(
        &self,
        request_id: &str,
        operation: &str,
    ) -> Result<Option<serde_json::Value>, AppError> {
        let record_path = self
            .root
            .join("requests")
            .join(format!("{request_id}.json"));
        if !record_path.exists() {
            return Ok(None);
        }
        let encoded =
            fs::read(record_path).map_err(|_| AppError::new("REQUEST_RECORD_INVALID", true))?;
        let record: RequestResult<serde_json::Value> = serde_json::from_slice(&encoded)
            .map_err(|_| AppError::new("REQUEST_RECORD_INVALID", true))?;
        if record.schema_version != 1
            || record.request_id != request_id
            || record.operation != operation
        {
            return Err(AppError::new("REQUEST_ID_CONFLICT", false));
        }
        Ok(Some(record.result))
    }
    pub fn create_from_docx(&self, path: &Path, task: TaskKind) -> Result<Project, AppError> {
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_none_or(|value| !value.eq_ignore_ascii_case("docx"))
        {
            return Err(AppError::new("FORMAT_UNSUPPORTED", true));
        }
        self.create_from_manuscript(path, task)
    }

    pub fn create_from_manuscript(&self, path: &Path, task: TaskKind) -> Result<Project, AppError> {
        self.create_from_manuscript_with_id(
            path,
            task,
            uuid::Uuid::new_v4().to_string(),
            None,
            None,
        )
    }
    fn create_from_manuscript_with_id(
        &self,
        path: &Path,
        task: TaskKind,
        project_id: String,
        workspace: Option<ProjectWorkspace>,
        display_name: Option<String>,
    ) -> Result<Project, AppError> {
        validate_id(&project_id)?;
        if self.project_dir(&project_id).exists() {
            return self.get(&project_id);
        }
        let inspection = inspect_source(path)?;
        let source_id = uuid::Uuid::new_v4().to_string();
        let final_dir = self.project_dir(&project_id);
        let temporary_dir = self
            .root
            .join("projects")
            .join(format!(".{project_id}.tmp-{}", uuid::Uuid::new_v4()));
        let sources_dir = temporary_dir.join("sources").join(&source_id);
        fs::create_dir_all(&sources_dir).map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
        let file_name = safe_file_name(path)?;
        let copied_source = sources_dir.join(&file_name);
        if let Err(error) = atomic_copy(path, &copied_source) {
            let _ = fs::remove_dir_all(&temporary_dir);
            return Err(error);
        }
        let copied_bytes = match fs::read(&copied_source) {
            Ok(bytes) => bytes,
            Err(_) => {
                let _ = fs::remove_dir_all(&temporary_dir);
                return Err(AppError::new("STORAGE_UNAVAILABLE", true));
            }
        };
        if copied_bytes.len() as u64 != inspection.size_bytes
            || hex::encode(Sha256::digest(&copied_bytes)) != inspection.sha256
        {
            let _ = fs::remove_dir_all(&temporary_dir);
            return Err(AppError::new("SOURCE_CHANGED_DURING_IMPORT", true)
                .recover("choose_manuscript_again"));
        }
        let now = now_ms()?;
        let project = Project {
            id: project_id,
            schema_version: PROJECT_SCHEMA_VERSION,
            revision: 1,
            display_name: display_name.unwrap_or_else(|| {
                inspection.facts.title.clone().unwrap_or_else(|| {
                    Path::new(&file_name)
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or(&file_name)
                        .to_owned()
                })
            }),
            active_source: SourceSnapshot {
                id: source_id,
                file_name,
                sha256: inspection.sha256,
                format: inspection.format,
                size_bytes: inspection.size_bytes,
                created_at_unix_ms: now,
                feature_profile: inspection.feature_profile,
            },
            facts: inspection.facts,
            materials: Vec::new(),
            target: None,
            author_decisions: Vec::new(),
            workspace,
            last_task: task,
            updated_at_unix_ms: now,
        };
        let result = (|| {
            let data = serde_json::to_vec_pretty(&project)
                .map_err(|_| AppError::new("PROJECT_INVALID", false))?;
            atomic_write(&temporary_dir.join("manifest.json"), &data)?;
            let original = path
                .canonicalize()
                .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
            let parent = original
                .parent()
                .ok_or_else(|| AppError::new("INPUT_UNREADABLE", true))?;
            atomic_write(
                &temporary_dir.join("input-directory.json"),
                &serde_json::to_vec(parent).map_err(|_| AppError::new("PROJECT_INVALID", false))?,
            )?;
            fs::rename(&temporary_dir, &final_dir)
                .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
            Ok(project)
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&temporary_dir);
        }
        result
    }

    pub fn open_or_create_folder_project(
        &self,
        folder: &Path,
        manuscript: &Path,
        task: TaskKind,
    ) -> Result<Project, AppError> {
        let canonical_folder = canonical_folder(folder)?;
        let canonical_manuscript = manuscript
            .canonicalize()
            .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
        if canonical_manuscript.parent() != Some(canonical_folder.as_path()) {
            return Err(AppError::new("FOLDER_MANUSCRIPT_OUTSIDE", false));
        }
        let binding_id = folder_binding_id(&canonical_folder);
        let _folder_guard = self.acquire_file_lock("folder", &binding_id)?;
        let binding_path = self.folder_binding_path(&binding_id);
        if binding_path.exists() {
            let binding = self.read_folder_binding(&binding_id)?;
            if binding.canonical_path != canonical_folder {
                return Err(AppError::new("FOLDER_BINDING_CONFLICT", false));
            }
            self.refresh_folder_project(&binding.project_id, &canonical_manuscript, task)?;
            return self.restore_recent_project(&binding.project_id);
        }

        let project_id = uuid::Uuid::new_v4().to_string();
        let folder_name = canonical_folder
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("ManuscriptDock")
            .to_owned();
        let workspace = ProjectWorkspace {
            kind: "folder".into(),
            name: folder_name.clone(),
            binding_id: binding_id.clone(),
        };
        let project = self.create_from_manuscript_with_id(
            &canonical_manuscript,
            task,
            project_id.clone(),
            Some(workspace),
            Some(folder_name),
        )?;
        let binding = FolderBinding {
            schema_version: 1,
            binding_id: binding_id.clone(),
            project_id,
            canonical_path: canonical_folder,
        };
        if let Err(error) = self.write_folder_binding(&binding) {
            let _ = fs::remove_dir_all(self.project_dir(&project.id));
            return Err(error);
        }
        Ok(project)
    }

    pub fn folder_path_for_project(&self, project_id: &str) -> Result<PathBuf, AppError> {
        let project = self.get(project_id)?;
        let workspace = project
            .workspace
            .as_ref()
            .filter(|workspace| workspace.kind == "folder")
            .ok_or_else(|| AppError::new("PROJECT_FOLDER_NOT_BOUND", true))?;
        let binding = self.read_folder_binding(&workspace.binding_id)?;
        if binding.project_id != project.id {
            return Err(AppError::new("FOLDER_BINDING_CONFLICT", false));
        }
        let canonical = canonical_folder(&binding.canonical_path)?;
        if folder_binding_id(&canonical) != binding.binding_id
            || canonical != binding.canonical_path
        {
            return Err(AppError::new("FOLDER_BINDING_CONFLICT", false));
        }
        Ok(canonical)
    }

    fn refresh_folder_project(
        &self,
        project_id: &str,
        manuscript: &Path,
        task: TaskKind,
    ) -> Result<Project, AppError> {
        let project_lock = project_lock(project_id)?;
        let _guard = project_lock
            .lock()
            .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
        let _file_guard = self.acquire_file_lock("project", project_id)?;
        let mut project = self.get(project_id)?;
        let inspection = inspect_source(manuscript)?;
        let mut source_cleanup = None;
        if inspection.sha256 != project.active_source.sha256 {
            let source_id = uuid::Uuid::new_v4().to_string();
            let file_name = safe_file_name(manuscript)?;
            let source_directory = self
                .project_dir(project_id)
                .join("sources")
                .join(&source_id);
            let cleanup = CleanupDirectory::new(source_directory.clone());
            let destination = source_directory.join(&file_name);
            atomic_copy(manuscript, &destination)?;
            let copied =
                fs::read(&destination).map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
            if copied.len() as u64 != inspection.size_bytes
                || hex::encode(Sha256::digest(&copied)) != inspection.sha256
            {
                return Err(AppError::new("SOURCE_CHANGED_DURING_IMPORT", true)
                    .recover("choose_manuscript_again"));
            }
            project.active_source = SourceSnapshot {
                id: source_id,
                file_name,
                sha256: inspection.sha256,
                format: inspection.format,
                size_bytes: inspection.size_bytes,
                created_at_unix_ms: now_ms()?,
                feature_profile: inspection.feature_profile,
            };
            project.facts = inspection.facts;
            project.author_decisions.clear();
            project.revision += 1;
            source_cleanup = Some(cleanup);
        }
        if project.last_task != task {
            project.last_task = task;
            project.revision += 1;
        }
        project.updated_at_unix_ms = now_ms()?;
        self.save(&project)?;
        if let Some(mut cleanup) = source_cleanup {
            cleanup.preserve();
        }
        Ok(project)
    }

    fn read_folder_binding(&self, binding_id: &str) -> Result<FolderBinding, AppError> {
        validate_folder_binding_id(binding_id)?;
        let encoded = fs::read(self.folder_binding_path(binding_id))
            .map_err(|_| AppError::new("FOLDER_BINDING_INVALID", true))?;
        let binding: FolderBinding = serde_json::from_slice(&encoded)
            .map_err(|_| AppError::new("FOLDER_BINDING_INVALID", true))?;
        if binding.schema_version != 1 || binding.binding_id != binding_id {
            return Err(AppError::new("FOLDER_BINDING_INVALID", true));
        }
        validate_id(&binding.project_id)?;
        Ok(binding)
    }

    fn write_folder_binding(&self, binding: &FolderBinding) -> Result<(), AppError> {
        let encoded = serde_json::to_vec_pretty(binding)
            .map_err(|_| AppError::new("FOLDER_BINDING_INVALID", false))?;
        atomic_write(&self.folder_binding_path(&binding.binding_id), &encoded)
    }

    fn folder_binding_path(&self, binding_id: &str) -> PathBuf {
        self.root.join("folders").join(format!("{binding_id}.json"))
    }
    pub fn get(&self, project_id: &str) -> Result<Project, AppError> {
        validate_id(project_id)?;
        let bytes = fs::read(self.project_dir(project_id).join("manifest.json")).map_err(|_| {
            AppError::new("PROJECT_NOT_FOUND", true).recover("open_another_project")
        })?;
        let project: Project = serde_json::from_slice(&bytes)
            .map_err(|_| AppError::new("PROJECT_INVALID", true).recover("open_another_project"))?;
        if project.schema_version != PROJECT_SCHEMA_VERSION || project.id != project_id {
            return Err(AppError::new("PROJECT_INVALID", true));
        }
        Ok(project)
    }
    pub fn recent(&self) -> Result<Vec<Project>, AppError> {
        let hidden = self.read_hidden_recent()?;
        let mut projects = Vec::new();
        for entry in fs::read_dir(self.root.join("projects"))
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?
        {
            let Ok(entry) = entry else {
                continue;
            };
            let id = entry.file_name().to_string_lossy().into_owned();
            if !hidden.contains(&id) {
                if let Ok(project) = self.get(&id) {
                    projects.push(project);
                }
            }
        }
        projects.sort_by(|left, right| right.updated_at_unix_ms.cmp(&left.updated_at_unix_ms));
        projects.truncate(12);
        Ok(projects)
    }
    pub fn removed_recent(&self) -> Result<Vec<Project>, AppError> {
        let hidden = self.read_hidden_recent()?;
        let mut projects = Vec::new();
        for id in hidden {
            if let Ok(project) = self.get(&id) {
                projects.push(project);
            }
        }
        projects.sort_by(|left, right| right.updated_at_unix_ms.cmp(&left.updated_at_unix_ms));
        Ok(projects)
    }
    pub fn hide_recent_project(&self, project_id: &str) -> Result<Project, AppError> {
        let project = self.get(project_id)?;
        let _guard = self.acquire_file_lock("recent", "hidden")?;
        let mut hidden = self.read_hidden_recent()?;
        hidden.insert(project_id.to_owned());
        self.write_hidden_recent(&hidden)?;
        Ok(project)
    }
    pub fn restore_recent_project(&self, project_id: &str) -> Result<Project, AppError> {
        let project = self.get(project_id)?;
        let _guard = self.acquire_file_lock("recent", "hidden")?;
        let mut hidden = self.read_hidden_recent()?;
        hidden.remove(project_id);
        self.write_hidden_recent(&hidden)?;
        Ok(project)
    }
    fn read_hidden_recent(&self) -> Result<BTreeSet<String>, AppError> {
        let path = self.root.join("recent-hidden.json");
        if !path.exists() {
            return Ok(BTreeSet::new());
        }
        let encoded = fs::read(path).map_err(|_| AppError::new("RECENT_INDEX_INVALID", true))?;
        let hidden = serde_json::from_slice::<BTreeSet<String>>(&encoded)
            .map_err(|_| AppError::new("RECENT_INDEX_INVALID", true))?;
        if hidden.iter().any(|id| validate_id(id).is_err()) {
            return Err(AppError::new("RECENT_INDEX_INVALID", true));
        }
        Ok(hidden)
    }
    fn write_hidden_recent(&self, hidden: &BTreeSet<String>) -> Result<(), AppError> {
        let encoded = serde_json::to_vec_pretty(hidden)
            .map_err(|_| AppError::new("RECENT_INDEX_INVALID", false))?;
        atomic_write(&self.root.join("recent-hidden.json"), &encoded)
    }
    pub fn select_target(
        &self,
        project_id: &str,
        expected_revision: u64,
        selection: TargetSelection,
    ) -> Result<Project, AppError> {
        let project_lock = project_lock(project_id)?;
        let _guard = project_lock
            .lock()
            .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
        let _file_guard = self.acquire_file_lock("project", project_id)?;
        let mut project = self.get(project_id)?;
        ensure_revision(&project, expected_revision)?;
        if matches!(selection.origin, TargetOrigin::Recommendation)
            && selection
                .recommendation_ref
                .as_deref()
                .is_none_or(str::is_empty)
        {
            return Err(AppError::new("RECOMMENDATION_REFERENCE_REQUIRED", true));
        }
        if matches!(selection.origin, TargetOrigin::Catalog)
            && selection.recommendation_ref.is_some()
        {
            return Err(AppError::new("RECOMMENDATION_REFERENCE_NOT_ALLOWED", true));
        }
        project.target = Some(selection);
        project.author_decisions.clear();
        project.revision += 1;
        project.last_task = TaskKind::PreparePackage;
        project.updated_at_unix_ms = now_ms()?;
        self.save(&project)?;
        Ok(project)
    }
    pub fn update_facts(
        &self,
        project_id: &str,
        expected_revision: u64,
        facts: DocumentFacts,
    ) -> Result<Project, AppError> {
        let project_lock = project_lock(project_id)?;
        let _guard = project_lock
            .lock()
            .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
        let _file_guard = self.acquire_file_lock("project", project_id)?;
        let mut project = self.get(project_id)?;
        ensure_revision(&project, expected_revision)?;
        if facts.source_hash != project.active_source.sha256 {
            return Err(AppError::new("CONTEXT_CHANGED", true).recover("reload_project"));
        }
        project.facts = facts;
        for index in 0..project.materials.len() {
            if project.materials[index].kind == "anonymized_manuscript" {
                let check = crate::documents::inspect_anonymity(
                    &self.material_path(&project.id, &project.materials[index]),
                    &project.facts,
                )?;
                project.materials[index].anonymity_check = Some(check);
            }
        }
        project.author_decisions = author_decisions(&project)?;
        project.revision += 1;
        project.updated_at_unix_ms = now_ms()?;
        self.save(&project)?;
        Ok(project)
    }
    pub fn add_material(
        &self,
        project_id: &str,
        expected_revision: u64,
        path: &Path,
        kind: &str,
    ) -> Result<Project, AppError> {
        let file_name = safe_file_name(path)?;
        self.add_material_named(project_id, expected_revision, path, kind, &file_name, false)
    }
    pub fn use_workspace_material(
        &self,
        project_id: &str,
        expected_revision: u64,
        path: &Path,
        kind: &str,
    ) -> Result<Project, AppError> {
        let file_name = safe_file_name(path)?;
        self.add_material_named(project_id, expected_revision, path, kind, &file_name, true)
    }
    fn add_material_named(
        &self,
        project_id: &str,
        expected_revision: u64,
        path: &Path,
        kind: &str,
        file_name: &str,
        replace_same_name: bool,
    ) -> Result<Project, AppError> {
        validate_stored_file_name(file_name)?;
        validate_material_kind(kind)?;
        let link_metadata =
            fs::symlink_metadata(path).map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
        if link_metadata.file_type().is_symlink() {
            return Err(
                AppError::new("INPUT_SYMLINK_NOT_ALLOWED", true).recover("choose_original_file")
            );
        }
        let is_anonymized = kind == "anonymized_manuscript";
        let is_editable_manuscript = kind == "editable_manuscript";
        let project_lock = project_lock(project_id)?;
        let _guard = project_lock
            .lock()
            .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
        let _file_guard = self.acquire_file_lock("project", project_id)?;
        let mut project = self.get(project_id)?;
        ensure_revision(&project, expected_revision)?;
        let metadata = path
            .metadata()
            .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
        if !metadata.is_file() {
            return Err(AppError::new("INPUT_UNREADABLE", true));
        }
        let bytes = fs::read(path).map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
        let size_bytes =
            u64::try_from(bytes.len()).map_err(|_| AppError::new("LIMIT_EXCEEDED", true))?;
        let hash = hex::encode(Sha256::digest(&bytes));
        if project.materials.iter().any(|material| {
            material.included
                && material.file_name == file_name
                && material.sha256 == hash
                && material.kind == kind
        }) {
            return Ok(project);
        }
        if project.materials.len() >= crate::MAX_MATERIAL_FILES {
            return Err(AppError::new("LIMIT_EXCEEDED", true));
        }
        let total = project.materials.iter().map(|m| m.size_bytes).sum::<u64>();
        if total.saturating_add(size_bytes) > crate::MAX_MATERIAL_TOTAL_BYTES {
            return Err(AppError::new("LIMIT_EXCEEDED", true));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let material_directory = self.project_dir(project_id).join("materials").join(&id);
        let destination = material_directory.join(file_name);
        let mut material_cleanup = CleanupDirectory::new(material_directory);
        atomic_write(&destination, &bytes)?;
        if is_anonymized || is_editable_manuscript {
            inspect_docx(&destination)?;
        }
        let anonymity_check = if is_anonymized {
            Some(crate::documents::inspect_anonymity(
                &destination,
                &project.facts,
            )?)
        } else {
            None
        };
        {
            for material in &mut project.materials {
                if material.kind == kind
                    && (is_anonymized
                        || is_editable_manuscript
                        || (replace_same_name && material.file_name == file_name))
                {
                    material.included = false;
                }
            }
        }
        project.materials.push(Material {
            id,
            file_name: file_name.to_owned(),
            sha256: hash,
            size_bytes,
            kind: kind.to_owned(),
            included: kind != "unclassified",
            anonymity_check,
        });
        project.revision += 1;
        project.updated_at_unix_ms = now_ms()?;
        self.save(&project)?;
        material_cleanup.preserve();
        Ok(project)
    }
    pub fn source_path(&self, project: &Project) -> PathBuf {
        self.project_dir(&project.id)
            .join("sources")
            .join(&project.active_source.id)
            .join(&project.active_source.file_name)
    }
    pub fn package_workspace_path(&self, project_id: &str) -> Result<PathBuf, AppError> {
        let project = self.get(project_id)?;
        let target = project
            .target
            .ok_or_else(|| AppError::new("TARGET_REQUIRED", true))?;
        let parent = self.package_parent_directory(project_id)?;
        let journal = crate::catalog()?
            .journals
            .into_iter()
            .find(|journal| journal.id == target.journal_id)
            .ok_or_else(|| AppError::new("JOURNAL_NOT_FOUND", true))?;
        let stem = Path::new(&project.active_source.file_name)
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Manuscript");
        let safe = |value: &str| {
            value
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || matches!(c, '-' | '_' | ' ') {
                        c
                    } else {
                        '-'
                    }
                })
                .take(65)
                .collect::<String>()
        };
        // A short project suffix prevents different tasks with the same filename sharing editable files.
        Ok(parent
            .join(format!(
                "{} - Submission Package - {}",
                safe(stem),
                &project.id[..8]
            ))
            .join(safe(&journal.display_name)))
    }

    pub fn package_parent_directory(&self, project_id: &str) -> Result<PathBuf, AppError> {
        let project = self.get(project_id)?;
        let override_path = self.project_dir(project_id).join("package-directory.json");
        if override_path.exists() {
            let parent: PathBuf = serde_json::from_slice(
                &fs::read(&override_path)
                    .map_err(|_| AppError::new("PACKAGE_LOCATION_REQUIRED", true))?,
            )
            .map_err(|_| AppError::new("PROJECT_INVALID", false))?;
            return canonical_folder(&parent);
        }
        if project.workspace.is_some() {
            return self.folder_path_for_project(project_id);
        }
        let path = self.project_dir(project_id).join("input-directory.json");
        if !path.exists() {
            return Err(AppError::new("PACKAGE_LOCATION_REQUIRED", true));
        }
        let parent: PathBuf = serde_json::from_slice(
            &fs::read(&path).map_err(|_| AppError::new("PACKAGE_LOCATION_REQUIRED", true))?,
        )
        .map_err(|_| AppError::new("PROJECT_INVALID", false))?;
        canonical_folder(&parent)
    }

    pub fn set_package_parent_directory(
        &self,
        project_id: &str,
        parent: &Path,
    ) -> Result<(), AppError> {
        self.get(project_id)?;
        let parent = canonical_folder(parent)?;
        atomic_write(
            &self.project_dir(project_id).join("package-directory.json"),
            &serde_json::to_vec(&parent).map_err(|_| AppError::new("PROJECT_INVALID", false))?,
        )
    }
    pub fn previous_package_workspace_path(&self, project_id: &str) -> Result<PathBuf, AppError> {
        let project = self.get(project_id)?;
        let target = project
            .target
            .ok_or_else(|| AppError::new("TARGET_REQUIRED", true))?;
        Ok(self
            .project_dir(project_id)
            .join("package-workspace")
            .join(hex::encode(Sha256::digest(target.journal_id.as_bytes()))))
    }
    pub fn material_path(&self, project_id: &str, material: &Material) -> PathBuf {
        self.project_dir(project_id)
            .join("materials")
            .join(&material.id)
            .join(&material.file_name)
    }
    pub fn staging_path(&self, project_id: &str, package_id: &str) -> PathBuf {
        self.project_dir(project_id)
            .join("staging")
            .join(package_id)
    }
    pub fn save_compiled_package(&self, package: &CompiledPackage) -> Result<(), AppError> {
        validate_id(&package.id)?;
        validate_id(&package.project_id)?;
        let encoded = serde_json::to_vec_pretty(package)
            .map_err(|_| AppError::new("PACKAGE_INVALID", false))?;
        atomic_write(
            &self
                .root
                .join("packages")
                .join(format!("{}.json", package.id)),
            &encoded,
        )
    }
    pub fn get_compiled_package(&self, package_id: &str) -> Result<CompiledPackage, AppError> {
        validate_id(package_id)?;
        let encoded = fs::read(
            self.root
                .join("packages")
                .join(format!("{package_id}.json")),
        )
        .map_err(|_| AppError::new("PACKAGE_NOT_FOUND", true))?;
        let mut package: CompiledPackage =
            serde_json::from_slice(&encoded).map_err(|_| AppError::new("PACKAGE_INVALID", true))?;
        if package.id != package_id {
            return Err(AppError::new("PACKAGE_INVALID", true));
        }
        package.staging_dir = self
            .staging_path(&package.project_id, package_id)
            .to_string_lossy()
            .into_owned();
        if !Path::new(&package.staging_dir).is_dir() {
            return Err(AppError::new("PACKAGE_NOT_FOUND", true));
        }
        Ok(package)
    }
    pub fn save(&self, project: &Project) -> Result<(), AppError> {
        validate_id(&project.id)?;
        let dir = self.project_dir(&project.id);
        fs::create_dir_all(&dir).map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
        let data = serde_json::to_vec_pretty(project)
            .map_err(|_| AppError::new("PROJECT_INVALID", false))?;
        atomic_write(&dir.join("manifest.json"), &data)
    }
    pub fn record_export_receipt(&self, receipt: &mut ExportReceipt) -> Result<(), AppError> {
        validate_id(&receipt.id)?;
        validate_id(&receipt.package_id)?;
        if receipt
            .request_id
            .as_deref()
            .is_some_and(|request_id| validate_request_id(request_id).is_err())
        {
            return Err(AppError::new("REQUEST_ID_INVALID", false));
        }
        let mut recorded = receipt.clone();
        recorded.record_persisted = true;
        let data = serde_json::to_vec_pretty(&recorded)
            .map_err(|_| AppError::new("PROJECT_INVALID", false))?;
        atomic_write(
            &self
                .root
                .join("receipts")
                .join(format!("{}.json", receipt.id)),
            &data,
        )?;
        receipt.record_persisted = true;
        Ok(())
    }
    pub fn export_receipt_for_request(
        &self,
        request_id: &str,
    ) -> Result<Option<ExportReceipt>, AppError> {
        validate_request_id(request_id)?;
        for entry in fs::read_dir(self.root.join("receipts"))
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?
        {
            let Ok(entry) = entry else {
                continue;
            };
            let Ok(encoded) = fs::read(entry.path()) else {
                continue;
            };
            let Ok(receipt) = serde_json::from_slice::<ExportReceipt>(&encoded) else {
                continue;
            };
            if receipt.request_id.as_deref() == Some(request_id) {
                return Ok(Some(receipt));
            }
        }
        Ok(None)
    }
    fn start_job(
        &self,
        request_id: &str,
        operation: &str,
        project_id: Option<&str>,
        context_hash: Option<&str>,
    ) -> Result<JobRecord, AppError> {
        validate_request_id(request_id)?;
        if let Some(project_id) = project_id {
            validate_id(project_id)?;
        }
        let lock = job_lock(request_id)?;
        let _guard = lock
            .lock()
            .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
        let _file_guard = self.acquire_file_lock("job", request_id)?;
        let path = self.job_path(request_id);
        let mut job = if path.exists() {
            self.read_job(request_id)?
        } else {
            let now = now_ms()?;
            JobRecord {
                schema_version: 1,
                id: request_id.to_owned(),
                project_id: project_id.map(str::to_owned),
                request_id: request_id.to_owned(),
                operation: operation.to_owned(),
                context_hash: context_hash.map(str::to_owned),
                status: JobStatus::Queued,
                cancel_requested: false,
                events: vec![JobEvent {
                    seq: 1,
                    phase: operation.to_owned(),
                    status: JobStatus::Queued,
                    created_at_unix_ms: now,
                    completed_units: None,
                    total_units: None,
                    error: None,
                }],
                result: None,
                updated_at_unix_ms: now,
            }
        };
        if job.operation != operation
            || job.project_id.as_deref() != project_id
            || job.context_hash.as_deref() != context_hash
        {
            return Err(AppError::new("REQUEST_ID_CONFLICT", false));
        }
        if job.status == JobStatus::Cancelled {
            return Err(AppError::new("JOB_CANCELLED", false));
        }
        append_job_event(&mut job, JobStatus::Running, None)?;
        self.save_job(&job)?;
        Ok(job)
    }
    fn transition_job(
        &self,
        job_id: &str,
        status: JobStatus,
        error: Option<AppError>,
    ) -> Result<JobRecord, AppError> {
        let lock = job_lock(job_id)?;
        let _guard = lock
            .lock()
            .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
        let _file_guard = self.acquire_file_lock("job", job_id)?;
        let mut job = self.read_job(job_id)?;
        if job.status == JobStatus::Cancelled {
            return Err(AppError::new("JOB_CANCELLED", false));
        }
        append_job_event(&mut job, status, error)?;
        self.save_job(&job)?;
        Ok(job)
    }

    fn complete_job(&self, job_id: &str, result: serde_json::Value) -> Result<JobRecord, AppError> {
        let lock = job_lock(job_id)?;
        let _guard = lock
            .lock()
            .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
        let _file_guard = self.acquire_file_lock("job", job_id)?;
        let mut job = self.read_job(job_id)?;
        if job.status == JobStatus::Cancelled || job.cancel_requested {
            return Err(AppError::new("JOB_CANCELLED", false));
        }
        job.result = Some(result);
        append_job_event(&mut job, JobStatus::Succeeded, None)?;
        self.save_job(&job)?;
        Ok(job)
    }
    fn read_job(&self, job_id: &str) -> Result<JobRecord, AppError> {
        validate_request_id(job_id)?;
        let encoded =
            fs::read(self.job_path(job_id)).map_err(|_| AppError::new("JOB_NOT_FOUND", true))?;
        let job: JobRecord = serde_json::from_slice(&encoded)
            .map_err(|_| AppError::new("JOB_RECORD_INVALID", true))?;
        if job.schema_version != 1 || job.id != job_id || job.request_id != job_id {
            return Err(AppError::new("JOB_RECORD_INVALID", true));
        }
        Ok(job)
    }
    fn save_job(&self, job: &JobRecord) -> Result<(), AppError> {
        let encoded = serde_json::to_vec_pretty(job)
            .map_err(|_| AppError::new("JOB_RECORD_INVALID", false))?;
        atomic_write(&self.job_path(&job.id), &encoded)
    }
    fn job_path(&self, job_id: &str) -> PathBuf {
        self.root.join("jobs").join(format!("{job_id}.json"))
    }
    pub fn acquire_ai_lock(&self, project_id: &str) -> Result<impl Drop + Send, AppError> {
        self.acquire_file_lock("ai-request", project_id)
    }

    pub(crate) fn acquire_file_lock(
        &self,
        kind: &str,
        id: &str,
    ) -> Result<FileLockGuard, AppError> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join("locks").join(format!("{kind}-{id}.lock")))
            .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
        fs2::FileExt::try_lock_exclusive(&file)
            .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
        Ok(FileLockGuard { file })
    }
    pub(crate) fn project_dir(&self, id: &str) -> PathBuf {
        self.root.join("projects").join(id)
    }
}

fn inspect_source(path: &Path) -> Result<SourceInspection, AppError> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("docx") => {
            let inspection = inspect_docx(path)?;
            Ok(SourceInspection {
                sha256: inspection.sha256,
                size_bytes: inspection.size_bytes,
                format: "docx".into(),
                feature_profile: format!("{:?}", inspection.profile).to_lowercase(),
                facts: inspection.facts,
            })
        }
        Some("pdf") => {
            let inspection = inspect_pdf(path)?;
            Ok(SourceInspection {
                sha256: inspection.sha256,
                size_bytes: inspection.size_bytes,
                format: "pdf".into(),
                feature_profile: if inspection
                    .warnings
                    .iter()
                    .any(|warning| warning == "pdf_has_no_extractable_text")
                {
                    "pdf_scan".into()
                } else {
                    "pdf_text".into()
                },
                facts: inspection.facts,
            })
        }
        _ => Err(AppError::new("FORMAT_UNSUPPORTED", true)
            .param("formats", "pdf,docx")
            .recover("choose_another_file")),
    }
}

fn validate_material_kind(kind: &str) -> Result<(), AppError> {
    if [
        "supplementary",
        "supplement",
        "figure",
        "reporting_checklist",
        "anonymized_manuscript",
        "editable_manuscript",
        "image",
        "spreadsheet",
        "document",
        "other",
        "unclassified",
    ]
    .contains(&kind)
    {
        Ok(())
    } else {
        Err(AppError::new("MATERIAL_KIND_INVALID", false))
    }
}

#[derive(Debug)]
pub(crate) struct FileLockGuard {
    file: fs::File,
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.file);
    }
}

fn append_job_event(
    job: &mut JobRecord,
    status: JobStatus,
    error: Option<AppError>,
) -> Result<(), AppError> {
    let seq = job.events.last().map_or(1, |event| event.seq + 1);
    let now = now_ms()?;
    job.events.push(JobEvent {
        seq,
        phase: job.operation.clone(),
        status,
        created_at_unix_ms: now,
        completed_units: None,
        total_units: None,
        error,
    });
    job.status = status;
    job.updated_at_unix_ms = now;
    Ok(())
}

fn author_decisions(project: &Project) -> Result<Vec<AuthorDecision>, AppError> {
    let Some(target) = project.target.as_ref() else {
        return Ok(Vec::new());
    };
    let confirmed_at_unix_ms = now_ms()?;
    crate::rules_for(&target.journal_id)?
        .into_iter()
        .filter_map(|rule| {
            let fact_key = rule.fact_key.clone()?;
            project
                .facts
                .author_confirmed_fields
                .iter()
                .any(|confirmed| confirmed == &fact_key)
                .then_some((rule, fact_key))
        })
        .filter_map(|(rule, fact_key)| {
            confirmed_fact_value(&project.facts, &fact_key).map(|value| (rule, fact_key, value))
        })
        .map(|(rule, _fact_key, value)| {
            Ok(AuthorDecision {
                context_hash: author_decision_context_hash(
                    &project.active_source.sha256,
                    &target.rules_hash,
                    &rule.id,
                    &value,
                )?,
                requirement_id: rule.id,
                value,
                evidence_ref: rule.evidence.source_url,
                confirmed_at_unix_ms,
            })
        })
        .collect()
}

pub(crate) fn confirmed_fact_value(facts: &DocumentFacts, key: &str) -> Option<String> {
    match key {
        "title" => facts.title.clone().filter(|value| !value.trim().is_empty()),
        "authors" => (!facts.authors.is_empty()).then(|| facts.authors.join("\u{1f}")),
        "affiliations" => {
            (!facts.affiliations.is_empty()).then(|| facts.affiliations.join("\u{1f}"))
        }
        "corresponding_email" => facts
            .corresponding_email
            .clone()
            .filter(|value| value.contains('@')),
        "conflict_of_interest" => facts
            .conflict_of_interest
            .clone()
            .filter(|value| !value.trim().is_empty()),
        "funding" => facts
            .funding
            .clone()
            .filter(|value| !value.trim().is_empty()),
        "data_availability" => facts
            .data_availability
            .clone()
            .filter(|value| !value.trim().is_empty()),
        "ethics_statement" => facts
            .ethics_statement
            .clone()
            .filter(|value| !value.trim().is_empty()),
        "highlights" => (!facts.highlights.is_empty()).then(|| facts.highlights.join("\u{1f}")),
        "abstract_text" => facts
            .abstract_text
            .clone()
            .filter(|value| !value.trim().is_empty()),
        "keywords" => (!facts.keywords.is_empty()).then(|| facts.keywords.join("\u{1f}")),
        "credit_contributions" => facts
            .credit_contributions
            .clone()
            .filter(|value| !value.trim().is_empty()),
        "generative_ai_disclosure" => facts
            .generative_ai_disclosure
            .clone()
            .filter(|value| !value.trim().is_empty()),
        _ => None,
    }
}

pub(crate) fn author_decision_context_hash(
    source_hash: &str,
    rules_hash: &str,
    requirement_id: &str,
    value: &str,
) -> Result<String, AppError> {
    let encoded = serde_json::to_vec(&(source_hash, rules_hash, requirement_id, value))
        .map_err(|_| AppError::new("PROJECT_INVALID", false))?;
    Ok(hex::encode(Sha256::digest(encoded)))
}

fn project_lock(project_id: &str) -> Result<Arc<Mutex<()>>, AppError> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    let mut locks = LOCKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
    Ok(locks
        .entry(project_id.to_owned())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone())
}

fn request_lock(request_id: &str) -> Result<Arc<Mutex<()>>, AppError> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    let mut locks = LOCKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
    Ok(locks
        .entry(request_id.to_owned())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone())
}

fn job_lock(job_id: &str) -> Result<Arc<Mutex<()>>, AppError> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    let mut locks = LOCKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| AppError::new("PROJECT_LOCK_UNAVAILABLE", true))?;
    Ok(locks
        .entry(job_id.to_owned())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone())
}

fn validate_id(value: &str) -> Result<(), AppError> {
    uuid::Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| AppError::new("PROJECT_INVALID", false))
}
fn validate_request_id(value: &str) -> Result<(), AppError> {
    uuid::Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| AppError::new("REQUEST_ID_INVALID", false))
}
fn ensure_revision(project: &Project, expected: u64) -> Result<(), AppError> {
    if project.revision == expected {
        Ok(())
    } else {
        Err(AppError::new("CONTEXT_CHANGED", true)
            .param("currentRevision", project.revision.to_string())
            .recover("reload_project"))
    }
}
fn safe_file_name(path: &Path) -> Result<String, AppError> {
    let value = path
        .file_name()
        .and_then(|v| v.to_str())
        .filter(|v| !v.is_empty() && *v != "." && *v != "..")
        .map(str::to_owned)
        .ok_or_else(|| AppError::new("INPUT_UNREADABLE", true))?;
    validate_stored_file_name(&value)?;
    Ok(value)
}
fn canonical_folder(path: &Path) -> Result<PathBuf, AppError> {
    let link_metadata = fs::symlink_metadata(path)
        .map_err(|_| AppError::new("PROJECT_FOLDER_UNAVAILABLE", true))?;
    if link_metadata.file_type().is_symlink() || !link_metadata.is_dir() {
        return Err(AppError::new("PROJECT_FOLDER_UNAVAILABLE", true));
    }
    path.canonicalize()
        .map_err(|_| AppError::new("PROJECT_FOLDER_UNAVAILABLE", true))
}
fn folder_binding_id(path: &Path) -> String {
    hex::encode(Sha256::digest(path.to_string_lossy().as_bytes()))
}
fn validate_folder_binding_id(value: &str) -> Result<(), AppError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(AppError::new("FOLDER_BINDING_INVALID", false))
    }
}
fn validate_stored_file_name(value: &str) -> Result<(), AppError> {
    let mut components = Path::new(value).components();
    if value.is_empty()
        || !matches!(components.next(), Some(std::path::Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(AppError::new("INPUT_UNREADABLE", true));
    }
    Ok(())
}
fn now_ms() -> Result<u64, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::new("SYSTEM_TIME_INVALID", false))
        .and_then(|v| {
            u64::try_from(v.as_millis()).map_err(|_| AppError::new("SYSTEM_TIME_INVALID", false))
        })
}
fn atomic_copy(source: &Path, destination: &Path) -> Result<(), AppError> {
    let bytes = fs::read(source).map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
    atomic_write(destination, &bytes)
}
pub(crate) fn atomic_write(destination: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = destination
        .parent()
        .ok_or_else(|| AppError::new("STORAGE_UNAVAILABLE", false))?;
    fs::create_dir_all(parent).map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
    let temporary = parent.join(format!(
        ".{}.tmp-{}",
        destination
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("write"),
        uuid::Uuid::new_v4()
    ));
    let mut file =
        fs::File::create(&temporary).map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
    let mut temporary_cleanup = CleanupFile::new(temporary.clone());
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
    fs::rename(&temporary, destination).map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
    temporary_cleanup.preserve();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};
    #[test]
    fn catalog_target_serializes_without_recommendation_reference() {
        let target = TargetSelection {
            id: "id".into(),
            journal_id: "journal".into(),
            article_type: "research_article".into(),
            stage: "initial_submission".into(),
            origin: TargetOrigin::Catalog,
            recommendation_ref: None,
            rules_hash: "hash".into(),
        };
        let encoded = serde_json::to_string(&target).unwrap();
        assert!(encoded.contains("\"origin\":\"catalog\""));
        assert!(!encoded.contains("recommendationRef"));
    }

    #[test]
    fn request_results_are_concurrent_safe_persistent_and_operation_bound() {
        let root = std::env::temp_dir().join(format!(
            "manuscriptdock-idempotency-{}",
            uuid::Uuid::new_v4()
        ));
        let store = ProjectStore::new(&root).unwrap();
        let request_id = uuid::Uuid::new_v4().to_string();
        let starts = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let handles = (0..2)
            .map(|_| {
                let store = store.clone();
                let request_id = request_id.clone();
                let starts = starts.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    store.execute_idempotent(&request_id, "test_operation", || {
                        starts.fetch_add(1, Ordering::SeqCst);
                        Ok::<_, AppError>("stable result".to_owned())
                    })
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        let results = handles
            .into_iter()
            .map(|handle| handle.join().unwrap().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results, vec!["stable result", "stable result"]);
        assert_eq!(starts.load(Ordering::SeqCst), 1);

        let reopened = ProjectStore::new(&root).unwrap();
        let recovered: String = reopened
            .execute_idempotent(&request_id, "test_operation", || {
                panic!("a persisted request must not execute again")
            })
            .unwrap();
        assert_eq!(recovered, "stable result");
        assert_eq!(
            reopened
                .execute_idempotent(&request_id, "different_operation", || Ok(7_u32))
                .unwrap_err()
                .code,
            "REQUEST_ID_CONFLICT"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn jobs_are_queryable_cancellable_and_recovered_as_interrupted() {
        let root =
            std::env::temp_dir().join(format!("manuscriptdock-jobs-{}", uuid::Uuid::new_v4()));
        let store = ProjectStore::new(&root).unwrap();
        let project_id = uuid::Uuid::new_v4().to_string();
        let held_lock = store.acquire_file_lock("project", &project_id).unwrap();
        assert_eq!(
            store
                .acquire_file_lock("project", &project_id)
                .unwrap_err()
                .code,
            "PROJECT_LOCK_UNAVAILABLE"
        );
        drop(held_lock);
        drop(store.acquire_file_lock("project", &project_id).unwrap());

        let completed_id = uuid::Uuid::new_v4().to_string();
        let value = store
            .execute_idempotent_job(
                &completed_id,
                "recommend_journals",
                Some(&project_id),
                Some("context"),
                || Ok::<_, AppError>("result".to_owned()),
            )
            .unwrap();
        assert_eq!(value, "result");
        let completed = store.get_job(&project_id, &completed_id).unwrap();
        assert_eq!(completed.status, JobStatus::Succeeded);
        assert_eq!(completed.events.len(), 3);
        assert_eq!(completed.events[0].status, JobStatus::Queued);

        let dispatched_id = uuid::Uuid::new_v4().to_string();
        let (queued, should_start) = store
            .reserve_job(
                &dispatched_id,
                "update_document_facts",
                Some(&project_id),
                Some("facts-context"),
            )
            .unwrap();
        assert!(should_start);
        assert_eq!(queued.status, JobStatus::Queued);
        let mut notifications = Vec::new();
        let dispatched_result = store
            .execute_reserved_job(
                &dispatched_id,
                "update_document_facts",
                || Ok::<_, AppError>(serde_json::json!({ "revision": 4 })),
                |job| notifications.push(job.clone()),
            )
            .unwrap();
        assert_eq!(dispatched_result["revision"], 4);
        assert_eq!(notifications.len(), 2);
        assert_eq!(notifications[0].status, JobStatus::Running);
        assert_eq!(notifications[1].status, JobStatus::Succeeded);
        assert_eq!(notifications[1].result, Some(dispatched_result.clone()));
        let (replayed, should_start_again) = store
            .reserve_job(
                &dispatched_id,
                "update_document_facts",
                Some(&project_id),
                Some("facts-context"),
            )
            .unwrap();
        assert!(!should_start_again);
        assert_eq!(replayed.result, Some(dispatched_result));

        let cancelled_id = uuid::Uuid::new_v4().to_string();
        store
            .start_job(&cancelled_id, "build_package", Some(&project_id), None)
            .unwrap();
        let cancelled = store.cancel_job(&project_id, &cancelled_id).unwrap();
        assert_eq!(cancelled.status, JobStatus::Cancelled);
        assert!(cancelled.cancel_requested);
        assert_eq!(
            store
                .transition_job(&cancelled_id, JobStatus::Succeeded, None)
                .unwrap_err()
                .code,
            "JOB_CANCELLED"
        );

        let interrupted_id = uuid::Uuid::new_v4().to_string();
        store
            .start_job(&interrupted_id, "export_package", Some(&project_id), None)
            .unwrap();
        assert_eq!(store.recover_interrupted_jobs().unwrap(), 1);
        assert_eq!(
            store.get_job(&project_id, &interrupted_id).unwrap().status,
            JobStatus::Interrupted
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hiding_recent_projects_is_reversible_and_never_deletes_project_files() {
        let root = std::env::temp_dir().join(format!(
            "manuscriptdock-recent-hidden-{}",
            uuid::Uuid::new_v4()
        ));
        let input = root.join("input.docx");
        fs::create_dir_all(&root).unwrap();
        write_test_docx(&input);
        let store = ProjectStore::new(root.join("data")).unwrap();
        let project = store
            .create_from_docx(&input, TaskKind::FindJournals)
            .unwrap();
        let manifest = store.project_dir(&project.id).join("manifest.json");
        let source = store.source_path(&project);
        let manifest_before = fs::read(&manifest).unwrap();
        let source_before = fs::read(&source).unwrap();

        store.hide_recent_project(&project.id).unwrap();
        assert!(store.recent().unwrap().is_empty());
        assert_eq!(store.removed_recent().unwrap()[0].id, project.id);
        assert_eq!(fs::read(&manifest).unwrap(), manifest_before);
        assert_eq!(fs::read(&source).unwrap(), source_before);

        store.restore_recent_project(&project.id).unwrap();
        assert_eq!(store.recent().unwrap()[0].id, project.id);
        assert!(store.removed_recent().unwrap().is_empty());
        assert_eq!(fs::read(&manifest).unwrap(), manifest_before);
        assert_eq!(fs::read(&source).unwrap(), source_before);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn folder_projects_reuse_one_record_and_keep_immutable_source_versions() {
        let root = std::env::temp_dir().join(format!(
            "manuscriptdock-folder-project-{}",
            uuid::Uuid::new_v4()
        ));
        let folder = root.join("Study Folder");
        fs::create_dir_all(&folder).unwrap();
        let draft = folder.join("draft.docx");
        write_test_docx_with_text(&draft, "Synthetic draft manuscript");
        let store = ProjectStore::new(root.join("data")).unwrap();

        let first = store
            .open_or_create_folder_project(&folder, &draft, TaskKind::FindJournals)
            .unwrap();
        let first_source_path = store.source_path(&first);
        assert_eq!(first.display_name, "Study Folder");
        assert_eq!(first.workspace.as_ref().unwrap().kind, "folder");
        assert_eq!(
            store.folder_path_for_project(&first.id).unwrap(),
            folder.canonicalize().unwrap()
        );

        let reopened = store
            .open_or_create_folder_project(&folder, &draft, TaskKind::PreparePackage)
            .unwrap();
        assert_eq!(reopened.id, first.id);
        assert_eq!(reopened.active_source.id, first.active_source.id);
        assert_eq!(store.recent().unwrap().len(), 1);

        store.hide_recent_project(&first.id).unwrap();
        assert!(store.recent().unwrap().is_empty());
        let explicitly_reopened = store
            .open_or_create_folder_project(&folder, &draft, TaskKind::PreparePackage)
            .unwrap();
        assert_eq!(explicitly_reopened.id, first.id);
        assert_eq!(store.recent().unwrap().len(), 1);

        let final_manuscript = folder.join("final.docx");
        write_test_docx_with_text(&final_manuscript, "Synthetic final manuscript");
        let updated = store
            .open_or_create_folder_project(&folder, &final_manuscript, TaskKind::PreparePackage)
            .unwrap();
        assert_eq!(updated.id, first.id);
        assert_ne!(updated.active_source.id, first.active_source.id);
        assert!(first_source_path.exists());

        let figure = folder.join("figure.png");
        fs::write(&figure, b"synthetic figure").unwrap();
        let with_material = store
            .add_material(&updated.id, updated.revision, &figure, "figure")
            .unwrap();
        let deduplicated = store
            .add_material(&with_material.id, with_material.revision, &figure, "figure")
            .unwrap();
        assert_eq!(deduplicated.revision, with_material.revision);
        assert_eq!(deduplicated.materials.len(), 1);

        let outside = root.join("outside.docx");
        write_test_docx(&outside);
        assert_eq!(
            store
                .open_or_create_folder_project(&folder, &outside, TaskKind::FindJournals)
                .unwrap_err()
                .code,
            "FOLDER_MANUSCRIPT_OUTSIDE"
        );
        let _ = fs::remove_dir_all(root);
    }

    fn write_test_docx(path: &Path) {
        write_test_docx_with_text(path, "Synthetic manuscript");
    }

    fn write_test_docx_with_text(path: &Path, text: &str) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer.start_file("[Content_Types].xml", options).unwrap();
        writer
            .write_all(
                br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#,
            )
            .unwrap();
        writer.start_file("word/document.xml", options).unwrap();
        writer
            .write_all(
                format!(
                    r#"<w:document xmlns:w="x"><w:body><w:p><w:r><w:t>{text}</w:t></w:r></w:p></w:body></w:document>"#
                )
                .as_bytes(),
            )
            .unwrap();
        writer.finish().unwrap();
    }
}
