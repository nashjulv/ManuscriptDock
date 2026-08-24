use crate::{
    inspect_manuscript,
    knowledge::{local_knowledge_body_snapshot, AcademicKnowledgeBodySnapshot},
    readiness::{
        evaluate_readiness, render_readiness_html, ReadinessError, READINESS_REPORT_VERSION,
    },
    revision::{apply_revision, extract_revision_fields},
    structure::{extract_structure, StructureError, STRUCTURE_ANALYSIS_VERSION},
    ManuscriptSummary, ReadinessReport, RevisionApplication, RevisionChangeInput, RevisionDraft,
    RevisionError, RevisionSet, StructureReport,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    error::Error,
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const MANIFEST_SCHEMA_VERSION: u32 = 2;
const LEGACY_MANIFEST_SCHEMA_VERSION: u32 = 1;
const SOURCE_SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub id: String,
    pub manuscript: ManuscriptSummary,
    pub content_hash: String,
    pub imported_unix_ms: u64,
    pub snapshot_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum WorkspaceCreation {
    Created { workspace: WorkspaceSummary },
    Rejected { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCatalog {
    pub workspaces: Vec<WorkspaceSummary>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionOrigin {
    Imported,
    Revision,
    Restored,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManuscriptVersionSummary {
    pub version: u32,
    pub parent_version: Option<u32>,
    pub manuscript: ManuscriptSummary,
    pub content_hash: String,
    pub created_unix_ms: u64,
    pub note: String,
    pub origin: VersionOrigin,
    pub restored_from_version: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionHistory {
    pub workspace_id: String,
    pub current_version: u32,
    pub versions: Vec<ManuscriptVersionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum VersionCreation {
    Created {
        workspace: Box<WorkspaceSummary>,
        version: Box<ManuscriptVersionSummary>,
    },
    Unchanged {
        version: u32,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionComparison {
    pub workspace_id: String,
    pub from_version: u32,
    pub to_version: u32,
    pub identical: bool,
    pub from_content_hash: String,
    pub to_content_hash: String,
    pub title_before: Option<String>,
    pub title_after: Option<String>,
    pub word_count_delta: i64,
    pub figure_count_delta: i64,
    pub table_count_delta: i64,
    pub added_sections: Vec<String>,
    pub removed_sections: Vec<String>,
    pub added_declarations: Vec<String>,
    pub removed_declarations: Vec<String>,
    pub external_transmission: &'static str,
}

#[derive(Debug)]
pub enum WorkspaceError {
    Io(io::Error),
    InvalidWorkspaceId,
    InvalidManifest(String),
    Structure(StructureError),
    Readiness(ReadinessError),
    Revision(RevisionError),
    SourceChangedDuringImport,
    VersionNotFound(u32),
    VersionFormatMismatch,
    VersionNoteTooLong,
    TimeBeforeUnixEpoch,
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "本地工作区写入失败：{error}"),
            Self::InvalidWorkspaceId => write!(formatter, "本地工作区标识无效"),
            Self::InvalidManifest(message) => write!(formatter, "本地工作区记录无效：{message}"),
            Self::Structure(error) => write!(formatter, "{error}"),
            Self::Readiness(error) => write!(formatter, "{error}"),
            Self::Revision(error) => write!(formatter, "{error}"),
            Self::SourceChangedDuringImport => {
                write!(formatter, "导入期间源稿件发生变化，请重新选择后再试")
            }
            Self::VersionNotFound(version) => write!(formatter, "未找到论文版本 v{version}"),
            Self::VersionFormatMismatch => write!(
                formatter,
                "新版本必须与当前稿件保持相同文件类型；格式转换应作为投稿输出保存"
            ),
            Self::VersionNoteTooLong => write!(formatter, "版本说明不能超过 200 个字符"),
            Self::TimeBeforeUnixEpoch => write!(formatter, "系统时间无效，无法创建审计记录"),
        }
    }
}

impl Error for WorkspaceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Structure(error) => Some(error),
            Self::Readiness(error) => Some(error),
            Self::Revision(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for WorkspaceError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<StructureError> for WorkspaceError {
    fn from(error: StructureError) -> Self {
        Self::Structure(error)
    }
}

impl From<ReadinessError> for WorkspaceError {
    fn from(error: ReadinessError) -> Self {
        Self::Readiness(error)
    }
}

impl From<RevisionError> for WorkspaceError {
    fn from(error: RevisionError) -> Self {
        Self::Revision(error)
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceStore {
    root: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceManifest {
    schema_version: u32,
    workspace: WorkspaceSummary,
    source_snapshot: SourceSnapshot,
    #[serde(default)]
    versions: Vec<StoredVersion>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceSnapshot {
    relative_path: String,
    readonly: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredVersion {
    #[serde(flatten)]
    summary: ManuscriptVersionSummary,
    relative_path: String,
    readonly: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditEvent<'a> {
    schema_version: u32,
    event_id: String,
    event_type: &'a str,
    occurred_unix_ms: u64,
    workspace_id: &'a str,
    snapshot_version: u32,
}

impl WorkspaceStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn create_from_source(
        &self,
        source_path: &Path,
    ) -> Result<WorkspaceSummary, WorkspaceError> {
        let manuscript = inspect_manuscript(source_path)
            .map_err(|error| WorkspaceError::InvalidManifest(error.to_string()))?;
        let workspace_id = Uuid::new_v4().to_string();
        let imported_unix_ms = unix_time_ms()?;
        let projects_root = self.projects_root();
        fs::create_dir_all(&projects_root)?;

        let temporary_root = projects_root.join(format!(".{workspace_id}.tmp"));
        let final_root = projects_root.join(&workspace_id);
        let source_directory = temporary_root.join("source");
        fs::create_dir_all(&source_directory)?;

        let snapshot_relative_path = format!("source/original.{}", manuscript.extension);
        let snapshot_path = temporary_root.join(&snapshot_relative_path);

        let result = (|| {
            let (content_hash, copied_size) = copy_and_hash(source_path, &snapshot_path)?;
            if copied_size != manuscript.size_bytes {
                return Err(WorkspaceError::SourceChangedDuringImport);
            }

            let workspace = WorkspaceSummary {
                id: workspace_id.clone(),
                manuscript: manuscript.clone(),
                content_hash: content_hash.clone(),
                imported_unix_ms,
                snapshot_version: SOURCE_SNAPSHOT_VERSION,
            };
            let initial_version = StoredVersion {
                summary: ManuscriptVersionSummary {
                    version: SOURCE_SNAPSHOT_VERSION,
                    parent_version: None,
                    manuscript,
                    content_hash,
                    created_unix_ms: imported_unix_ms,
                    note: String::new(),
                    origin: VersionOrigin::Imported,
                    restored_from_version: None,
                },
                relative_path: snapshot_relative_path.clone(),
                readonly: true,
            };
            let manifest = WorkspaceManifest {
                schema_version: MANIFEST_SCHEMA_VERSION,
                workspace: workspace.clone(),
                source_snapshot: SourceSnapshot {
                    relative_path: snapshot_relative_path,
                    readonly: true,
                },
                versions: vec![initial_version],
            };

            write_json(&temporary_root.join("manifest.json"), &manifest)?;
            append_audit_event(
                &temporary_root.join("audit.jsonl"),
                "workspace_created",
                &workspace,
                workspace.imported_unix_ms,
            )?;
            set_readonly(&snapshot_path)?;
            fs::rename(&temporary_root, &final_root)?;

            Ok(workspace)
        })();

        if result.is_err() {
            let _ = remove_generated_directory(&temporary_root);
        }

        result
    }

    pub fn list(&self) -> Result<WorkspaceCatalog, WorkspaceError> {
        let projects_root = self.projects_root();
        if !projects_root.exists() {
            return Ok(WorkspaceCatalog {
                workspaces: Vec::new(),
                warnings: Vec::new(),
            });
        }

        let mut workspaces = Vec::new();
        let mut warnings = Vec::new();
        for entry in fs::read_dir(projects_root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let directory_name = entry.file_name().to_string_lossy().into_owned();
            if directory_name.starts_with('.') || Uuid::parse_str(&directory_name).is_err() {
                continue;
            }

            match read_manifest(&entry.path().join("manifest.json")) {
                Ok(manifest) if manifest.workspace.id == directory_name => {
                    workspaces.push(manifest.workspace);
                }
                Ok(_) => warnings.push(format!("工作区 {directory_name} 的标识不一致，已跳过")),
                Err(_) => warnings.push(format!("工作区 {directory_name} 无法读取，已跳过")),
            }
        }

        workspaces.sort_by(|left, right| right.imported_unix_ms.cmp(&left.imported_unix_ms));
        Ok(WorkspaceCatalog {
            workspaces,
            warnings,
        })
    }

    pub fn source_snapshot_path(&self, workspace_id: &str) -> Result<PathBuf, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let versions = normalized_versions(&manifest);
        let current = versions
            .iter()
            .find(|version| version.summary.version == manifest.workspace.snapshot_version)
            .ok_or(WorkspaceError::VersionNotFound(
                manifest.workspace.snapshot_version,
            ))?;
        resolve_snapshot_path(&workspace_root, &current.relative_path)
    }

    pub fn version_history(&self, workspace_id: &str) -> Result<VersionHistory, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let versions = normalized_versions(&manifest);
        for version in &versions {
            let path = resolve_snapshot_path(&workspace_root, &version.relative_path)?;
            verify_version_snapshot(&path, &version.summary)?;
        }
        Ok(VersionHistory {
            workspace_id: workspace_id.to_owned(),
            current_version: manifest.workspace.snapshot_version,
            versions: versions
                .into_iter()
                .map(|version| version.summary)
                .collect(),
        })
    }

    pub fn knowledge_body_snapshot(
        &self,
        workspace_id: &str,
    ) -> Result<AcademicKnowledgeBodySnapshot, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let snapshot_path = self.source_snapshot_path(workspace_id)?;
        verify_snapshot(&snapshot_path, &manifest.workspace)?;
        Ok(local_knowledge_body_snapshot(&manifest.workspace))
    }

    pub fn create_version_from_source(
        &self,
        workspace_id: &str,
        source_path: &Path,
        note: &str,
    ) -> Result<VersionCreation, WorkspaceError> {
        let manuscript = inspect_manuscript(source_path)
            .map_err(|error| WorkspaceError::InvalidManifest(error.to_string()))?;
        self.commit_version(
            workspace_id,
            source_path,
            manuscript,
            note,
            VersionOrigin::Revision,
            None,
        )
    }

    pub fn restore_version(
        &self,
        workspace_id: &str,
        version: u32,
    ) -> Result<VersionCreation, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let versions = normalized_versions(&manifest);
        let selected = versions
            .iter()
            .find(|candidate| candidate.summary.version == version)
            .ok_or(WorkspaceError::VersionNotFound(version))?;
        let snapshot_path = resolve_snapshot_path(&workspace_root, &selected.relative_path)?;
        verify_version_snapshot(&snapshot_path, &selected.summary)?;
        self.commit_version(
            workspace_id,
            &snapshot_path,
            selected.summary.manuscript.clone(),
            "",
            VersionOrigin::Restored,
            Some(version),
        )
    }

    pub fn compare_versions(
        &self,
        workspace_id: &str,
        from_version: u32,
        to_version: u32,
    ) -> Result<VersionComparison, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let versions = normalized_versions(&manifest);
        let from = versions
            .iter()
            .find(|candidate| candidate.summary.version == from_version)
            .ok_or(WorkspaceError::VersionNotFound(from_version))?;
        let to = versions
            .iter()
            .find(|candidate| candidate.summary.version == to_version)
            .ok_or(WorkspaceError::VersionNotFound(to_version))?;
        let from_path = resolve_snapshot_path(&workspace_root, &from.relative_path)?;
        let to_path = resolve_snapshot_path(&workspace_root, &to.relative_path)?;
        verify_version_snapshot(&from_path, &from.summary)?;
        verify_version_snapshot(&to_path, &to.summary)?;
        let from_structure = extract_structure(
            &from_path,
            &from.summary.manuscript,
            workspace_id,
            &from.summary.content_hash,
            from.summary.version,
        )?;
        let to_structure = extract_structure(
            &to_path,
            &to.summary.manuscript,
            workspace_id,
            &to.summary.content_hash,
            to.summary.version,
        )?;
        let from_sections = from_structure
            .sections
            .iter()
            .map(|section| section.heading.clone())
            .collect::<BTreeSet<_>>();
        let to_sections = to_structure
            .sections
            .iter()
            .map(|section| section.heading.clone())
            .collect::<BTreeSet<_>>();
        let from_declarations = from_structure
            .declarations
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let to_declarations = to_structure
            .declarations
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();

        Ok(VersionComparison {
            workspace_id: workspace_id.to_owned(),
            from_version,
            to_version,
            identical: from.summary.content_hash == to.summary.content_hash,
            from_content_hash: from.summary.content_hash.clone(),
            to_content_hash: to.summary.content_hash.clone(),
            title_before: from_structure.title,
            title_after: to_structure.title,
            word_count_delta: to_structure.word_count as i64 - from_structure.word_count as i64,
            figure_count_delta: to_structure.figure_count as i64
                - from_structure.figure_count as i64,
            table_count_delta: to_structure.table_count as i64 - from_structure.table_count as i64,
            added_sections: to_sections.difference(&from_sections).cloned().collect(),
            removed_sections: from_sections.difference(&to_sections).cloned().collect(),
            added_declarations: to_declarations
                .difference(&from_declarations)
                .cloned()
                .collect(),
            removed_declarations: from_declarations
                .difference(&to_declarations)
                .cloned()
                .collect(),
            external_transmission: "not_performed",
        })
    }

    pub fn analyze_structure(&self, workspace_id: &str) -> Result<StructureReport, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let snapshot_path = self.source_snapshot_path(workspace_id)?;
        verify_snapshot(&snapshot_path, &manifest.workspace)?;
        let report = extract_structure(
            &snapshot_path,
            &manifest.workspace.manuscript,
            workspace_id,
            &manifest.workspace.content_hash,
            manifest.workspace.snapshot_version,
        )?;

        let analysis_root = workspace_root.join("analysis");
        fs::create_dir_all(&analysis_root)?;
        let hash_prefix = manifest
            .workspace
            .content_hash
            .get(..12)
            .ok_or_else(|| WorkspaceError::InvalidManifest("内容指纹长度无效".to_owned()))?;
        let report_path = analysis_root.join(format!(
            "structure-v{STRUCTURE_ANALYSIS_VERSION}-{hash_prefix}.json"
        ));
        if !report_path.exists() {
            let temporary_path = analysis_root.join(format!(".{}.tmp", Uuid::new_v4()));
            write_json(&temporary_path, &report)?;
            match fs::rename(&temporary_path, &report_path) {
                Ok(()) => {}
                Err(_) if report_path.exists() => {
                    let _ = fs::remove_file(temporary_path);
                }
                Err(error) => return Err(WorkspaceError::Io(error)),
            }
        }

        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "structure_analyzed",
            &manifest.workspace,
            unix_time_ms()?,
        )?;
        Ok(report)
    }

    pub fn evaluate_readiness(
        &self,
        workspace_id: &str,
        selected_rule_pack_ids: &[String],
    ) -> Result<ReadinessReport, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let snapshot_path = self.source_snapshot_path(workspace_id)?;
        verify_snapshot(&snapshot_path, &manifest.workspace)?;
        let structure = extract_structure(
            &snapshot_path,
            &manifest.workspace.manuscript,
            workspace_id,
            &manifest.workspace.content_hash,
            manifest.workspace.snapshot_version,
        )?;
        let generated_unix_ms = unix_time_ms()?;
        let report_id = Uuid::new_v4().to_string();
        let report = evaluate_readiness(
            &structure,
            report_id.clone(),
            generated_unix_ms,
            selected_rule_pack_ids,
        )?;
        let preview = render_readiness_html(&report, &manifest.workspace.manuscript.name);

        let outputs_root = workspace_root.join("outputs");
        fs::create_dir_all(&outputs_root)?;
        let temporary_root = outputs_root.join(format!(".{report_id}.tmp"));
        let final_root = outputs_root.join(&report_id);
        fs::create_dir(&temporary_root)?;
        let result = (|| {
            write_json(
                &temporary_root.join(format!("readiness-v{READINESS_REPORT_VERSION}.json")),
                &report,
            )?;
            write_text(&temporary_root.join("preview.html"), &preview)?;
            fs::rename(&temporary_root, &final_root)?;
            append_audit_event(
                &workspace_root.join("audit.jsonl"),
                "readiness_evaluated",
                &manifest.workspace,
                generated_unix_ms,
            )?;
            Ok(report)
        })();
        if result.is_err() {
            let _ = remove_generated_directory(&temporary_root);
        }
        result
    }

    pub fn revision_draft(&self, workspace_id: &str) -> Result<RevisionDraft, WorkspaceError> {
        let history = self.version_history(workspace_id)?;
        let current = history
            .versions
            .iter()
            .find(|version| version.version == history.current_version)
            .ok_or(WorkspaceError::VersionNotFound(history.current_version))?;
        let path = self.source_snapshot_path(workspace_id)?;
        let (fields, warnings) = extract_revision_fields(&path, &current.manuscript)?;
        Ok(RevisionDraft {
            workspace_id: workspace_id.to_owned(),
            base_version: history.current_version,
            format: current.manuscript.extension.clone(),
            fields,
            warnings,
        })
    }

    pub fn apply_revision(
        &self,
        workspace_id: &str,
        base_version: u32,
        inputs: &[RevisionChangeInput],
    ) -> Result<RevisionApplication, WorkspaceError> {
        let history = self.version_history(workspace_id)?;
        if base_version != history.current_version {
            return Err(WorkspaceError::InvalidManifest(
                "修订基础版本已不是当前版本，请重新载入后再试".to_owned(),
            ));
        }
        let current = history
            .versions
            .iter()
            .find(|version| version.version == base_version)
            .ok_or(WorkspaceError::VersionNotFound(base_version))?;
        let source = self.source_snapshot_path(workspace_id)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let temporary = workspace_root.join(format!(
            ".revision-{}.{}",
            Uuid::new_v4(),
            current.manuscript.extension
        ));
        let result = (|| {
            let changes = apply_revision(&source, &temporary, &current.manuscript, inputs)?;
            if changes.is_empty() {
                return Ok(RevisionApplication::Unchanged {
                    version: base_version,
                    message: "没有检测到字段变化，未创建重复版本".to_owned(),
                });
            }
            let creation = self.create_version_from_source(
                workspace_id,
                &temporary,
                &format!("投稿优化修订台：{} 项修改", changes.len()),
            )?;
            let VersionCreation::Created { workspace, version } = creation else {
                return Ok(RevisionApplication::Unchanged {
                    version: base_version,
                    message: "修改后内容与当前版本相同，未创建重复版本".to_owned(),
                });
            };
            let revision_set = RevisionSet {
                revision_id: Uuid::new_v4().to_string(),
                workspace_id: workspace_id.to_owned(),
                base_version,
                output_version: version.version,
                created_unix_ms: version.created_unix_ms,
                changes,
                external_transmission: "not_performed".to_owned(),
            };
            let revision_path =
                workspace_root.join(format!("versions/v{:04}/revision.json", version.version));
            write_json(&revision_path, &revision_set)?;
            set_readonly(&revision_path)?;
            append_audit_event(
                &workspace_root.join("audit.jsonl"),
                "revision_applied",
                &workspace,
                unix_time_ms()?,
            )?;
            Ok(RevisionApplication::Created {
                workspace,
                version,
                revision_set,
            })
        })();
        let _ = fs::remove_file(&temporary);
        result
    }

    fn commit_version(
        &self,
        workspace_id: &str,
        source_path: &Path,
        manuscript: ManuscriptSummary,
        note: &str,
        origin: VersionOrigin,
        restored_from_version: Option<u32>,
    ) -> Result<VersionCreation, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        if note.chars().count() > 200 {
            return Err(WorkspaceError::VersionNoteTooLong);
        }
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest_path = workspace_root.join("manifest.json");
        let mut manifest = read_manifest(&manifest_path)?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        if manuscript.kind != manifest.workspace.manuscript.kind {
            return Err(WorkspaceError::VersionFormatMismatch);
        }

        let mut versions = normalized_versions(&manifest);
        let current_version = manifest.workspace.snapshot_version;
        let next_version = versions
            .iter()
            .map(|version| version.summary.version)
            .max()
            .unwrap_or(current_version)
            .checked_add(1)
            .ok_or_else(|| WorkspaceError::InvalidManifest("版本号已达到上限".to_owned()))?;
        let temporary_root = workspace_root.join(format!(".version-{}.tmp", Uuid::new_v4()));
        fs::create_dir(&temporary_root)?;
        let temporary_snapshot =
            temporary_root.join(format!("manuscript.{}", manuscript.extension));

        let result = (|| {
            let (content_hash, copied_size) = copy_and_hash(source_path, &temporary_snapshot)?;
            if copied_size != manuscript.size_bytes {
                return Err(WorkspaceError::SourceChangedDuringImport);
            }
            if content_hash == manifest.workspace.content_hash {
                return Ok(VersionCreation::Unchanged {
                    version: current_version,
                    message: "所选稿件与当前版本内容完全一致，未创建重复版本".to_owned(),
                });
            }

            set_readonly(&temporary_snapshot)?;
            let versions_root = workspace_root.join("versions");
            fs::create_dir_all(&versions_root)?;
            let final_root = versions_root.join(format!("v{next_version:04}"));
            if final_root.exists() {
                return Err(WorkspaceError::InvalidManifest(
                    "目标版本目录已存在，未覆盖任何文件".to_owned(),
                ));
            }
            fs::rename(&temporary_root, &final_root)?;
            let relative_path = format!(
                "versions/v{next_version:04}/manuscript.{}",
                manuscript.extension
            );
            let created_unix_ms = unix_time_ms()?;
            let summary = ManuscriptVersionSummary {
                version: next_version,
                parent_version: Some(current_version),
                manuscript: manuscript.clone(),
                content_hash: content_hash.clone(),
                created_unix_ms,
                note: note.trim().to_owned(),
                origin,
                restored_from_version,
            };
            versions.push(StoredVersion {
                summary: summary.clone(),
                relative_path,
                readonly: true,
            });
            manifest.schema_version = MANIFEST_SCHEMA_VERSION;
            manifest.workspace.manuscript = manuscript;
            manifest.workspace.content_hash = content_hash;
            manifest.workspace.snapshot_version = next_version;
            manifest.versions = versions;
            replace_json(&manifest_path, &manifest)?;
            append_audit_event(
                &workspace_root.join("audit.jsonl"),
                match origin {
                    VersionOrigin::Restored => "version_restored",
                    VersionOrigin::Imported | VersionOrigin::Revision => "version_created",
                },
                &manifest.workspace,
                created_unix_ms,
            )?;
            Ok(VersionCreation::Created {
                workspace: Box::new(manifest.workspace),
                version: Box::new(summary),
            })
        })();

        if temporary_root.exists() {
            let _ = remove_generated_directory(&temporary_root);
        }
        result
    }

    fn projects_root(&self) -> PathBuf {
        self.root.join("projects")
    }
}

fn copy_and_hash(source_path: &Path, destination: &Path) -> Result<(String, u64), WorkspaceError> {
    let mut source = BufReader::new(File::open(source_path)?);
    let mut destination = BufWriter::new(
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(destination)?,
    );
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut copied_size = 0_u64;

    loop {
        let bytes_read = source.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        destination.write_all(&buffer[..bytes_read])?;
        hasher.update(&buffer[..bytes_read]);
        copied_size = copied_size
            .checked_add(bytes_read as u64)
            .ok_or_else(|| WorkspaceError::InvalidManifest("文件大小溢出".to_owned()))?;
    }
    destination.flush()?;
    destination.get_ref().sync_all()?;

    Ok((hex::encode(hasher.finalize()), copied_size))
}

fn verify_snapshot(
    snapshot_path: &Path,
    workspace: &WorkspaceSummary,
) -> Result<(), WorkspaceError> {
    verify_snapshot_integrity(
        snapshot_path,
        workspace.manuscript.size_bytes,
        &workspace.content_hash,
    )
}

fn verify_version_snapshot(
    snapshot_path: &Path,
    version: &ManuscriptVersionSummary,
) -> Result<(), WorkspaceError> {
    verify_snapshot_integrity(
        snapshot_path,
        version.manuscript.size_bytes,
        &version.content_hash,
    )
}

fn verify_snapshot_integrity(
    snapshot_path: &Path,
    expected_size: u64,
    expected_hash: &str,
) -> Result<(), WorkspaceError> {
    if fs::metadata(snapshot_path)?.len() != expected_size {
        return Err(WorkspaceError::InvalidManifest(
            "源快照完整性验证失败".to_owned(),
        ));
    }
    let mut reader = BufReader::new(File::open(snapshot_path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    if hex::encode(hasher.finalize()) != expected_hash {
        return Err(WorkspaceError::InvalidManifest(
            "源快照完整性验证失败".to_owned(),
        ));
    }
    Ok(())
}

fn set_readonly(path: &Path) -> Result<(), WorkspaceError> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), WorkspaceError> {
    let mut writer = BufWriter::new(File::create(path)?);
    serde_json::to_writer_pretty(&mut writer, value)
        .map_err(|error| WorkspaceError::InvalidManifest(error.to_string()))?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(())
}

fn replace_json(path: &Path, value: &impl Serialize) -> Result<(), WorkspaceError> {
    let parent = path
        .parent()
        .ok_or_else(|| WorkspaceError::InvalidManifest("记录路径无效".to_owned()))?;
    let temporary_path = parent.join(format!(".manifest-{}.tmp", Uuid::new_v4()));
    let backup_path = parent.join(format!(".manifest-{}.bak", Uuid::new_v4()));
    write_json(&temporary_path, value)?;
    fs::rename(path, &backup_path)?;
    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::rename(&backup_path, path);
        let _ = fs::remove_file(&temporary_path);
        return Err(WorkspaceError::Io(error));
    }
    fs::remove_file(backup_path)?;
    Ok(())
}

fn write_text(path: &Path, value: &str) -> Result<(), WorkspaceError> {
    let mut writer = BufWriter::new(OpenOptions::new().create_new(true).write(true).open(path)?);
    writer.write_all(value.as_bytes())?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(())
}

fn read_manifest(path: &Path) -> Result<WorkspaceManifest, WorkspaceError> {
    let reader = BufReader::new(File::open(path)?);
    let manifest: WorkspaceManifest = serde_json::from_reader(reader)
        .map_err(|error| WorkspaceError::InvalidManifest(error.to_string()))?;
    if manifest.schema_version != MANIFEST_SCHEMA_VERSION
        && manifest.schema_version != LEGACY_MANIFEST_SCHEMA_VERSION
    {
        return Err(WorkspaceError::InvalidManifest(
            "不支持的记录版本".to_owned(),
        ));
    }
    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_manifest(manifest: &WorkspaceManifest) -> Result<(), WorkspaceError> {
    if !manifest.source_snapshot.readonly
        || !is_safe_relative_path(&manifest.source_snapshot.relative_path)
    {
        return Err(WorkspaceError::InvalidManifest(
            "源快照引用不是安全的相对路径或未标记只读".to_owned(),
        ));
    }
    if manifest.schema_version == LEGACY_MANIFEST_SCHEMA_VERSION && manifest.versions.is_empty() {
        return Ok(());
    }
    if manifest.versions.is_empty() {
        return Err(WorkspaceError::InvalidManifest(
            "版本清单不能为空".to_owned(),
        ));
    }
    let mut seen_versions = BTreeSet::new();
    for version in &manifest.versions {
        if version.summary.version == 0
            || !seen_versions.insert(version.summary.version)
            || !version.readonly
            || !is_safe_relative_path(&version.relative_path)
            || !is_sha256(&version.summary.content_hash)
            || version
                .summary
                .parent_version
                .is_some_and(|parent| parent >= version.summary.version)
        {
            return Err(WorkspaceError::InvalidManifest(
                "版本清单包含无效或重复记录".to_owned(),
            ));
        }
    }
    let current = manifest
        .versions
        .iter()
        .find(|version| version.summary.version == manifest.workspace.snapshot_version)
        .ok_or_else(|| WorkspaceError::InvalidManifest("当前版本不存在于版本清单".to_owned()))?;
    if current.summary.manuscript != manifest.workspace.manuscript
        || current.summary.content_hash != manifest.workspace.content_hash
    {
        return Err(WorkspaceError::InvalidManifest(
            "当前版本与工作区摘要不一致".to_owned(),
        ));
    }
    Ok(())
}

fn is_safe_relative_path(value: &str) -> bool {
    let path = Path::new(value);
    !value.is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn normalized_versions(manifest: &WorkspaceManifest) -> Vec<StoredVersion> {
    if !manifest.versions.is_empty() {
        return manifest.versions.clone();
    }
    vec![StoredVersion {
        summary: ManuscriptVersionSummary {
            version: SOURCE_SNAPSHOT_VERSION,
            parent_version: None,
            manuscript: manifest.workspace.manuscript.clone(),
            content_hash: manifest.workspace.content_hash.clone(),
            created_unix_ms: manifest.workspace.imported_unix_ms,
            note: String::new(),
            origin: VersionOrigin::Imported,
            restored_from_version: None,
        },
        relative_path: manifest.source_snapshot.relative_path.clone(),
        readonly: manifest.source_snapshot.readonly,
    }]
}

fn resolve_snapshot_path(
    workspace_root: &Path,
    relative_path: &str,
) -> Result<PathBuf, WorkspaceError> {
    if !is_safe_relative_path(relative_path) {
        return Err(WorkspaceError::InvalidManifest(
            "源快照引用不是安全的相对路径".to_owned(),
        ));
    }
    let relative_path = Path::new(relative_path);
    let snapshot_path = workspace_root.join(relative_path);
    if !snapshot_path.is_file() {
        return Err(WorkspaceError::InvalidManifest(
            "源快照引用不存在".to_owned(),
        ));
    }
    Ok(snapshot_path)
}

fn append_audit_event(
    path: &Path,
    event_type: &str,
    workspace: &WorkspaceSummary,
    occurred_unix_ms: u64,
) -> Result<(), WorkspaceError> {
    let event = AuditEvent {
        schema_version: 1,
        event_id: Uuid::new_v4().to_string(),
        event_type,
        occurred_unix_ms,
        workspace_id: &workspace.id,
        snapshot_version: workspace.snapshot_version,
    };
    let mut writer = BufWriter::new(OpenOptions::new().create(true).append(true).open(path)?);
    serde_json::to_writer(&mut writer, &event)
        .map_err(|error| WorkspaceError::InvalidManifest(error.to_string()))?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(())
}

fn unix_time_ms() -> Result<u64, WorkspaceError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| WorkspaceError::TimeBeforeUnixEpoch)?;
    u64::try_from(duration.as_millis()).map_err(|_| WorkspaceError::TimeBeforeUnixEpoch)
}

fn remove_generated_directory(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    make_tree_writable(path)?;
    fs::remove_dir_all(path)
}

fn make_tree_writable(path: &Path) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry.file_type()?.is_dir() {
            make_tree_writable(&entry_path)?;
        } else {
            let mut permissions = fs::metadata(&entry_path)?.permissions();
            if permissions.readonly() {
                make_file_owner_writable(&mut permissions);
                fs::set_permissions(entry_path, permissions)?;
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_file_owner_writable(permissions: &mut fs::Permissions) {
    use std::os::unix::fs::PermissionsExt;
    permissions.set_mode(permissions.mode() | 0o200);
}

#[cfg(not(unix))]
fn make_file_owner_writable(permissions: &mut fs::Permissions) {
    permissions.set_readonly(false);
}

#[cfg(test)]
mod tests {
    use super::{make_tree_writable, VersionCreation, VersionOrigin, WorkspaceStore};
    use crate::{ReadinessOutcome, RevisionApplication, RevisionChangeInput, RevisionFieldKind};
    use std::{
        fs::{self, File},
        io::Write,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    struct SyntheticDirectory(PathBuf);

    impl SyntheticDirectory {
        fn create() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should follow the Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "manuscriptdock-workspace-test-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("synthetic directory should be created");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for SyntheticDirectory {
        fn drop(&mut self) {
            let _ = make_tree_writable(&self.0);
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn creates_an_immutable_snapshot_and_recovers_it_from_the_catalog() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("Synthetic Study.tex");
        let source_content = b"\\section{Introduction}\nSynthetic evidence.";
        File::create(&source_path)
            .and_then(|mut file| file.write_all(source_content))
            .expect("synthetic manuscript should be written");
        let store = WorkspaceStore::new(temporary.path().join("store"));

        let created = store
            .create_from_source(&source_path)
            .expect("workspace should be created");
        let snapshot_path = store
            .source_snapshot_path(&created.id)
            .expect("snapshot path should resolve internally");
        let catalog = store.list().expect("catalog should be readable");

        assert_eq!(catalog.workspaces, vec![created.clone()]);
        assert!(catalog.warnings.is_empty());
        assert_eq!(fs::read(snapshot_path).unwrap(), source_content);
        assert_eq!(fs::read(&source_path).unwrap(), source_content);
        assert_eq!(created.content_hash.len(), 64);
    }

    #[test]
    fn persisted_records_do_not_contain_the_original_absolute_path() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("private-source.pdf");
        fs::write(&source_path, b"synthetic pdf fixture").expect("fixture should be written");
        let store_root = temporary.path().join("store");
        let store = WorkspaceStore::new(&store_root);

        let created = store
            .create_from_source(&source_path)
            .expect("workspace should be created");
        let project_root = store_root.join("projects").join(created.id);
        let persisted = format!(
            "{}\n{}",
            fs::read_to_string(project_root.join("manifest.json")).unwrap(),
            fs::read_to_string(project_root.join("audit.jsonl")).unwrap()
        );

        assert!(!persisted.contains(&temporary.path().display().to_string()));
        assert!(persisted.contains("workspace_created"));
    }

    #[test]
    fn catalog_skips_a_corrupt_workspace_and_reports_a_safe_warning() {
        let temporary = SyntheticDirectory::create();
        let workspace_id = uuid::Uuid::new_v4().to_string();
        let corrupt_root = temporary.path().join("store/projects").join(&workspace_id);
        fs::create_dir_all(&corrupt_root).unwrap();
        fs::write(corrupt_root.join("manifest.json"), b"not-json").unwrap();
        let store = WorkspaceStore::new(temporary.path().join("store"));

        let catalog = store.list().expect("catalog should remain recoverable");

        assert!(catalog.workspaces.is_empty());
        assert_eq!(catalog.warnings.len(), 1);
        assert!(catalog.warnings[0].contains(&workspace_id));
        assert!(!catalog.warnings[0].contains(&temporary.path().display().to_string()));
    }

    #[test]
    fn analyzes_the_immutable_snapshot_and_versions_the_local_result() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("structured-study.tex");
        fs::write(
            &source_path,
            r"\title{Synthetic Study}
\begin{abstract}Synthetic abstract.\end{abstract}
\keywords{local, deterministic}
\section{Introduction}
\section{Methods}
\begin{figure}\end{figure}
\bibliography{synthetic}",
        )
        .unwrap();
        let store_root = temporary.path().join("store");
        let store = WorkspaceStore::new(&store_root);
        let workspace = store.create_from_source(&source_path).unwrap();

        let report = store.analyze_structure(&workspace.id).unwrap();

        assert_eq!(report.title.as_deref(), Some("Synthetic Study"));
        assert_eq!(report.sections.len(), 2);
        assert_eq!(report.figure_count, 1);
        assert!(report.references_present);
        assert_eq!(report.source_content_hash, workspace.content_hash);

        let project_root = store_root.join("projects").join(&workspace.id);
        let analysis_files = fs::read_dir(project_root.join("analysis"))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(analysis_files.len(), 1);
        let persisted_report = fs::read_to_string(analysis_files[0].path()).unwrap();
        let audit = fs::read_to_string(project_root.join("audit.jsonl")).unwrap();
        assert!(persisted_report.contains("Synthetic Study"));
        assert!(!persisted_report.contains(&temporary.path().display().to_string()));
        assert_eq!(audit.lines().count(), 2);
        assert!(audit.contains("structure_analyzed"));
    }

    #[test]
    fn rejects_a_manifest_source_path_that_traverses_outside_the_workspace() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("safe.tex");
        fs::write(&source_path, "synthetic").unwrap();
        let store_root = temporary.path().join("store");
        let store = WorkspaceStore::new(&store_root);
        let workspace = store.create_from_source(&source_path).unwrap();
        let manifest_path = store_root
            .join("projects")
            .join(&workspace.id)
            .join("manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest["sourceSnapshot"]["relativePath"] =
            serde_json::Value::String("../outside.tex".to_owned());
        manifest["versions"][0]["relativePath"] =
            serde_json::Value::String("../outside.tex".to_owned());
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let error = store.source_snapshot_path(&workspace.id).unwrap_err();

        assert!(error.to_string().contains("安全的相对路径"));
    }

    #[test]
    fn rejects_a_manifest_whose_current_version_disagrees_with_the_workspace() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("safe.tex");
        fs::write(&source_path, "synthetic").unwrap();
        let store_root = temporary.path().join("store");
        let store = WorkspaceStore::new(&store_root);
        let workspace = store.create_from_source(&source_path).unwrap();
        let manifest_path = store_root
            .join("projects")
            .join(&workspace.id)
            .join("manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest["versions"][0]["contentHash"] = serde_json::Value::String("f".repeat(64));
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let error = store.version_history(&workspace.id).unwrap_err();

        assert!(error.to_string().contains("工作区摘要不一致"));
    }

    #[test]
    fn creates_deduplicates_and_compares_local_manuscript_versions() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("study.tex");
        fs::write(
            &source_path,
            r"\title{Versioned Study}
\section{Introduction}
Initial evidence.",
        )
        .unwrap();
        let store = WorkspaceStore::new(temporary.path().join("store"));
        let workspace = store.create_from_source(&source_path).unwrap();
        let initial_history = store.version_history(&workspace.id).unwrap();
        assert_eq!(initial_history.current_version, 1);
        assert_eq!(initial_history.versions.len(), 1);
        assert_eq!(initial_history.versions[0].origin, VersionOrigin::Imported);

        let duplicate = store
            .create_version_from_source(&workspace.id, &source_path, "duplicate")
            .unwrap();
        assert!(matches!(
            duplicate,
            VersionCreation::Unchanged { version: 1, .. }
        ));

        fs::write(
            &source_path,
            r"\title{Versioned Study Revised}
\section{Introduction}
Expanded evidence and rationale.
\section{Methods}
Synthetic method.",
        )
        .unwrap();
        let created = store
            .create_version_from_source(&workspace.id, &source_path, "补充方法")
            .unwrap();
        let VersionCreation::Created {
            workspace: updated,
            version,
        } = created
        else {
            panic!("a changed manuscript should create a version")
        };
        assert_eq!(updated.snapshot_version, 2);
        assert_eq!(version.version, 2);
        assert_eq!(version.parent_version, Some(1));
        assert_eq!(version.note, "补充方法");

        let comparison = store.compare_versions(&workspace.id, 1, 2).unwrap();
        assert!(!comparison.identical);
        assert_eq!(comparison.title_before.as_deref(), Some("Versioned Study"));
        assert_eq!(
            comparison.title_after.as_deref(),
            Some("Versioned Study Revised")
        );
        assert_eq!(comparison.added_sections, vec!["Methods"]);
        assert_eq!(comparison.external_transmission, "not_performed");
    }

    #[test]
    fn applies_a_structured_revision_as_a_new_immutable_version_with_provenance() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("revision-study.tex");
        fs::write(&source_path, "\\title{Original title}\n\\begin{abstract}Original abstract\\end{abstract}\n\\keywords{one, two}\n\\section{Methods}Keep").unwrap();
        let store_root = temporary.path().join("store");
        let store = WorkspaceStore::new(&store_root);
        let workspace = store.create_from_source(&source_path).unwrap();

        let draft = store.revision_draft(&workspace.id).unwrap();
        assert_eq!(draft.base_version, 1);
        assert_eq!(draft.fields.len(), 3);
        let result = store
            .apply_revision(
                &workspace.id,
                1,
                &[RevisionChangeInput {
                    field: RevisionFieldKind::Title,
                    after: "Revised title".to_owned(),
                }],
            )
            .unwrap();
        let RevisionApplication::Created {
            workspace: revised,
            version,
            revision_set,
        } = result
        else {
            panic!("revision should create a version")
        };

        assert_eq!(revised.snapshot_version, 2);
        assert_eq!(version.parent_version, Some(1));
        assert_eq!(revision_set.base_version, 1);
        assert_eq!(revision_set.output_version, 2);
        assert_eq!(revision_set.changes[0].before, "Original title");
        assert_eq!(revision_set.changes[0].after, "Revised title");
        let project = store_root.join("projects").join(&workspace.id);
        assert!(project.join("versions/v0002/revision.json").exists());
        assert!(fs::read_to_string(project.join("audit.jsonl"))
            .unwrap()
            .contains("revision_applied"));
        assert_eq!(
            store
                .analyze_structure(&workspace.id)
                .unwrap()
                .title
                .as_deref(),
            Some("Revised title")
        );
    }

    #[test]
    fn restores_an_old_snapshot_as_a_new_version_without_rewriting_history() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("restore.tex");
        fs::write(&source_path, "\\section{First}\nVersion one.").unwrap();
        let store = WorkspaceStore::new(temporary.path().join("store"));
        let workspace = store.create_from_source(&source_path).unwrap();
        let original_hash = workspace.content_hash.clone();
        fs::write(&source_path, "\\section{Second}\nVersion two.").unwrap();
        store
            .create_version_from_source(&workspace.id, &source_path, "second")
            .unwrap();

        let restored = store.restore_version(&workspace.id, 1).unwrap();
        let VersionCreation::Created {
            workspace: restored_workspace,
            version,
        } = restored
        else {
            panic!("restoring an older version should create a new head")
        };
        assert_eq!(restored_workspace.snapshot_version, 3);
        assert_eq!(restored_workspace.content_hash, original_hash);
        assert_eq!(version.origin, VersionOrigin::Restored);
        assert_eq!(version.parent_version, Some(2));
        assert_eq!(version.restored_from_version, Some(1));

        let history = store.version_history(&workspace.id).unwrap();
        assert_eq!(history.current_version, 3);
        assert_eq!(history.versions.len(), 3);
        assert_eq!(history.versions[0].version, 1);
        assert_eq!(history.versions[1].version, 2);
        assert_eq!(history.versions[2].version, 3);
    }

    #[test]
    fn rejects_a_revision_with_a_different_document_format() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("study.tex");
        let pdf_path = temporary.path().join("study.pdf");
        fs::write(&source_path, "synthetic tex").unwrap();
        fs::write(&pdf_path, "synthetic pdf").unwrap();
        let store = WorkspaceStore::new(temporary.path().join("store"));
        let workspace = store.create_from_source(&source_path).unwrap();

        let error = store
            .create_version_from_source(&workspace.id, &pdf_path, "wrong format")
            .unwrap_err();

        assert!(error.to_string().contains("相同文件类型"));
        assert_eq!(
            store.version_history(&workspace.id).unwrap().versions.len(),
            1
        );
    }

    #[test]
    fn reads_and_migrates_a_legacy_single_snapshot_manifest() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("legacy.tex");
        fs::write(&source_path, "\\section{Legacy}\nVersion one.").unwrap();
        let store_root = temporary.path().join("store");
        let store = WorkspaceStore::new(&store_root);
        let workspace = store.create_from_source(&source_path).unwrap();
        let manifest_path = store_root
            .join("projects")
            .join(&workspace.id)
            .join("manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest["schemaVersion"] = serde_json::Value::from(1);
        manifest.as_object_mut().unwrap().remove("versions");
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let legacy_history = store.version_history(&workspace.id).unwrap();
        assert_eq!(legacy_history.versions.len(), 1);
        assert_eq!(legacy_history.versions[0].version, 1);

        fs::write(&source_path, "\\section{Migrated}\nVersion two.").unwrap();
        store
            .create_version_from_source(&workspace.id, &source_path, "migrate")
            .unwrap();
        let migrated: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        assert_eq!(migrated["schemaVersion"], 2);
        assert_eq!(migrated["versions"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn rejects_analysis_when_the_immutable_snapshot_fingerprint_has_changed() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("fingerprinted.tex");
        fs::write(&source_path, "synthetic evidence").unwrap();
        let store = WorkspaceStore::new(temporary.path().join("store"));
        let workspace = store.create_from_source(&source_path).unwrap();
        let snapshot_path = store.source_snapshot_path(&workspace.id).unwrap();
        let mut permissions = fs::metadata(&snapshot_path).unwrap().permissions();
        super::make_file_owner_writable(&mut permissions);
        fs::set_permissions(&snapshot_path, permissions).unwrap();
        fs::write(&snapshot_path, "tampered evidence!").unwrap();

        let error = store.analyze_structure(&workspace.id).unwrap_err();

        assert!(error.to_string().contains("完整性验证失败"));
    }

    #[test]
    fn writes_an_immutable_readiness_report_and_html_preview() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("readiness-study.tex");
        fs::write(
            &source_path,
            r"\title{Submission Study}
\begin{abstract}Evidence.\end{abstract}
\keywords{submission}
\section{Introduction}
\section{Methods}
\section{Conflict of Interest}
\section{Data Availability}
\bibliography{synthetic}",
        )
        .unwrap();
        let store_root = temporary.path().join("store");
        let store = WorkspaceStore::new(&store_root);
        let workspace = store.create_from_source(&source_path).unwrap();

        let report = store.evaluate_readiness(&workspace.id, &[]).unwrap();

        assert_eq!(report.outcome, ReadinessOutcome::Ready);
        assert_eq!(report.rule_packs.len(), 2);
        assert!(report.rule_packs.iter().all(|pack| pack.signature_verified));
        let snapshot_root = store_root
            .join("projects")
            .join(&workspace.id)
            .join("outputs")
            .join(&report.report_id);
        let json = fs::read_to_string(snapshot_root.join(format!(
            "readiness-v{}.json",
            super::READINESS_REPORT_VERSION
        )))
        .unwrap();
        let html = fs::read_to_string(snapshot_root.join("preview.html")).unwrap();
        let audit = fs::read_to_string(
            store_root
                .join("projects")
                .join(&workspace.id)
                .join("audit.jsonl"),
        )
        .unwrap();
        assert!(json.contains("signatureVerified"));
        assert!(html.contains("未发生外部传输"));
        assert!(!json.contains(&temporary.path().display().to_string()));
        assert!(audit.contains("readiness_evaluated"));
    }
}
