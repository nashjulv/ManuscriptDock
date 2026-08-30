use crate::{
    dialogue::{
        KnowledgeAnswerRecord, KnowledgeDialogueItem, KnowledgeDialogueLedger,
        KnowledgeInquiryOrigin, KnowledgeInquiryRecord, KnowledgeInquiryStance,
        KnowledgeInquiryTarget, KNOWLEDGE_DIALOGUE_SCHEMA_VERSION,
    },
    inspect_manuscript,
    journal_match::{
        deadline_days_remaining, recommend_journals, InstitutionRuleEvidence,
        InstitutionRuleStatus, JournalMatchPreferences, JournalRecommendationProfile,
        JournalRecommendationProfileInput, JournalRecommendationRun,
        JOURNAL_PROFILE_SCHEMA_VERSION,
    },
    knowledge::{
        discipline_catalog_item, local_knowledge_body_snapshot, AcademicKnowledgeBodySnapshot,
        DisciplineClassification, KnowledgeBodyError, DISCIPLINE_INDEX_SCHEME,
        DISCIPLINE_INDEX_VERSION,
    },
    readiness::{
        evaluate_readiness, render_readiness_html, ReadinessError, READINESS_REPORT_VERSION,
    },
    revision::{apply_revision, extract_revision_fields},
    structure::{extract_structure, StructureError, STRUCTURE_ANALYSIS_VERSION},
    ManuscriptSummary, ReadinessOutcome, ReadinessReport, RevisionApplication, RevisionChangeInput,
    RevisionDraft, RevisionError, RevisionSet, StructureReport,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
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
    pub archived_workspaces: Vec<WorkspaceSummary>,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAttestation {
    pub attestation_id: String,
    pub workspace_id: String,
    pub manuscript_version: u32,
    pub manuscript_hash: String,
    pub readiness_report_id: String,
    pub readiness_output_snapshot_version: u32,
    pub readiness_outcome: ReadinessOutcome,
    pub attested_unix_ms: u64,
    pub statement: String,
    pub record_hash: String,
    pub external_transmission: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionRecord {
    pub submission_id: String,
    pub workspace_id: String,
    pub manuscript_version: u32,
    pub attestation_id: String,
    pub target: String,
    pub receipt: Option<String>,
    pub submitted_unix_ms: u64,
    pub statement: String,
    pub record_hash: String,
    pub external_transmission: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionExport {
    pub package_name: String,
    pub manuscript_version: u32,
    pub attestation_id: String,
    pub files: Vec<String>,
    pub exported_unix_ms: u64,
    pub external_transmission: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBodyRecord {
    pub record_id: String,
    pub workspace_id: String,
    pub manuscript_version: u32,
    pub attestation_id: String,
    pub submission_id: String,
    pub finalized_unix_ms: u64,
    #[serde(default)]
    pub discipline_classification: Option<DisciplineClassification>,
    pub snapshot: AcademicKnowledgeBodySnapshot,
    pub record_hash: String,
    pub external_transmission: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLifecycle {
    pub workspace_id: String,
    pub current_version: u32,
    pub structure_report: Option<StructureReport>,
    pub readiness_report: Option<ReadinessReport>,
    pub attestation: Option<LocalAttestation>,
    pub submission: Option<SubmissionRecord>,
    pub knowledge_body: Option<KnowledgeBodyRecord>,
}

#[derive(Debug)]
pub enum WorkspaceError {
    Io(io::Error),
    InvalidWorkspaceId,
    WorkspaceNotFound,
    WorkspaceDestinationExists,
    InvalidManifest(String),
    Structure(StructureError),
    Readiness(ReadinessError),
    Revision(RevisionError),
    Knowledge(KnowledgeBodyError),
    SourceChangedDuringImport,
    VersionNotFound(u32),
    VersionFormatMismatch,
    VersionNoteTooLong,
    InvalidJournalProfile,
    InvalidInstitutionRuleEvidence,
    JournalProfileNotFound,
    MissingCurrentReadiness,
    AuthorConfirmationRequired,
    InvalidSubmissionTarget,
    MissingCurrentAttestation,
    MissingCurrentSubmission,
    InvalidDisciplineClassification,
    MissingCurrentKnowledgeBody,
    InvalidKnowledgeInquiry,
    KnowledgeInquiryNotFound,
    InvalidKnowledgeAnswer,
    InvalidExportDestination,
    ExportDestinationExists,
    TimeBeforeUnixEpoch,
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "本地工作区写入失败：{error}"),
            Self::InvalidWorkspaceId => write!(formatter, "本地工作区标识无效"),
            Self::WorkspaceNotFound => write!(formatter, "未找到需要管理的本地工作区"),
            Self::WorkspaceDestinationExists => {
                write!(formatter, "目标位置已存在同一工作区，未移动任何文件")
            }
            Self::InvalidManifest(message) => write!(formatter, "本地工作区记录无效：{message}"),
            Self::Structure(error) => write!(formatter, "{error}"),
            Self::Readiness(error) => write!(formatter, "{error}"),
            Self::Revision(error) => write!(formatter, "{error}"),
            Self::Knowledge(error) => write!(formatter, "{error}"),
            Self::SourceChangedDuringImport => {
                write!(formatter, "导入期间源稿件发生变化，请重新选择后再试")
            }
            Self::VersionNotFound(version) => write!(formatter, "未找到论文版本 v{version}"),
            Self::VersionFormatMismatch => write!(
                formatter,
                "新版本必须与当前稿件保持相同文件类型；格式转换应作为投稿输出保存"
            ),
            Self::VersionNoteTooLong => write!(formatter, "版本说明不能超过 200 个字符"),
            Self::InvalidJournalProfile => write!(
                formatter,
                "请完整填写姓名、学校、专业、论文用途和有效的未来投稿截止日期"
            ),
            Self::InvalidInstitutionRuleEvidence => write!(
                formatter,
                "学校要求抽取结果缺少可追溯来源、有效规则版本或合法的分区条件"
            ),
            Self::JournalProfileNotFound => {
                write!(
                    formatter,
                    "未找到已保存的投稿背景档案，请先保存后再计算推荐"
                )
            }
            Self::MissingCurrentReadiness => {
                write!(formatter, "当前论文版本尚未完成投稿检查，请先重新检查")
            }
            Self::AuthorConfirmationRequired => {
                write!(formatter, "需要作者明确确认后才能创建记录")
            }
            Self::InvalidSubmissionTarget => {
                write!(formatter, "投稿目标不能为空，且不能超过 200 个字符")
            }
            Self::MissingCurrentAttestation => {
                write!(formatter, "当前论文版本尚未完成本地存证")
            }
            Self::MissingCurrentSubmission => {
                write!(formatter, "当前论文版本尚未登记投稿记录")
            }
            Self::InvalidDisciplineClassification => {
                write!(formatter, "请选择有效的学科索引分类后再固化知识体")
            }
            Self::MissingCurrentKnowledgeBody => {
                write!(formatter, "当前论文版本尚未固化知识体，不能建立问答记录")
            }
            Self::InvalidKnowledgeInquiry => {
                write!(formatter, "知识体问题不能为空，且不能超过 4000 个字符")
            }
            Self::KnowledgeInquiryNotFound => write!(formatter, "未找到当前知识体对应的问题记录"),
            Self::InvalidKnowledgeAnswer => {
                write!(
                    formatter,
                    "模型回答、模型名称和提供方不能为空，且长度必须在限制内"
                )
            }
            Self::InvalidExportDestination => write!(formatter, "请选择可写入的导出文件夹"),
            Self::ExportDestinationExists => {
                write!(formatter, "目标文件夹中已存在同名投稿包，未覆盖任何文件")
            }
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
            Self::Knowledge(error) => Some(error),
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

impl From<KnowledgeBodyError> for WorkspaceError {
    fn from(error: KnowledgeBodyError) -> Self {
        Self::Knowledge(error)
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AttestationPayload<'a> {
    attestation_id: &'a str,
    workspace_id: &'a str,
    manuscript_version: u32,
    manuscript_hash: &'a str,
    readiness_report_id: &'a str,
    readiness_output_snapshot_version: u32,
    readiness_outcome: ReadinessOutcome,
    attested_unix_ms: u64,
    statement: &'a str,
    external_transmission: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmissionPayload<'a> {
    submission_id: &'a str,
    workspace_id: &'a str,
    manuscript_version: u32,
    attestation_id: &'a str,
    target: &'a str,
    receipt: &'a Option<String>,
    submitted_unix_ms: u64,
    statement: &'a str,
    external_transmission: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyKnowledgeBodyPayload<'a> {
    record_id: &'a str,
    workspace_id: &'a str,
    manuscript_version: u32,
    attestation_id: &'a str,
    submission_id: &'a str,
    finalized_unix_ms: u64,
    snapshot: &'a AcademicKnowledgeBodySnapshot,
    external_transmission: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeBodyPayload<'a> {
    record_id: &'a str,
    workspace_id: &'a str,
    manuscript_version: u32,
    attestation_id: &'a str,
    submission_id: &'a str,
    finalized_unix_ms: u64,
    discipline_classification: &'a DisciplineClassification,
    snapshot: &'a AcademicKnowledgeBodySnapshot,
    external_transmission: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeInquiryPayload<'a> {
    schema_version: u32,
    inquiry_id: &'a str,
    workspace_id: &'a str,
    knowledge_body_record_id: &'a str,
    knowledge_body_hash: &'a str,
    snapshot_version: u32,
    origin: KnowledgeInquiryOrigin,
    stance: KnowledgeInquiryStance,
    target: KnowledgeInquiryTarget,
    question: &'a str,
    external_actor_label: &'a Option<String>,
    created_unix_ms: u64,
    external_transmission: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeAnswerPayload<'a> {
    schema_version: u32,
    answer_id: &'a str,
    inquiry_id: &'a str,
    workspace_id: &'a str,
    knowledge_body_record_id: &'a str,
    model_slot: &'a str,
    provider_label: &'a str,
    model: &'a str,
    answer: &'a str,
    source_anchors: &'a [crate::VersionedObjectReference],
    created_unix_ms: u64,
    external_transmission: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmissionPackageManifest<'a> {
    schema_version: u32,
    workspace_id: &'a str,
    manuscript_version: u32,
    manuscript_hash: &'a str,
    readiness_report_id: &'a str,
    attestation_id: &'a str,
    attestation_hash: &'a str,
    created_unix_ms: u64,
    files: &'a [String],
    external_transmission: &'a str,
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
        let archived_projects_root = self.archived_projects_root();
        let mut workspaces = Vec::new();
        let mut archived_workspaces = Vec::new();
        let mut warnings = Vec::new();
        for (collection_root, collection, label) in [
            (&projects_root, &mut workspaces, "工作区"),
            (
                &archived_projects_root,
                &mut archived_workspaces,
                "归档工作区",
            ),
        ] {
            if !collection_root.exists() {
                continue;
            }
            for entry in fs::read_dir(collection_root)? {
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
                        collection.push(manifest.workspace);
                    }
                    Ok(_) => {
                        warnings.push(format!("{label} {directory_name} 的标识不一致，已跳过"))
                    }
                    Err(_) => warnings.push(format!("{label} {directory_name} 无法读取，已跳过")),
                }
            }
        }

        workspaces.sort_by(|left, right| right.imported_unix_ms.cmp(&left.imported_unix_ms));
        archived_workspaces
            .sort_by(|left, right| right.imported_unix_ms.cmp(&left.imported_unix_ms));
        Ok(WorkspaceCatalog {
            workspaces,
            archived_workspaces,
            warnings,
        })
    }

    pub fn archive_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<WorkspaceCatalog, WorkspaceError> {
        let (source_root, manifest) = self.workspace_for_management(workspace_id, false)?;
        let archived_root = self.archived_projects_root();
        fs::create_dir_all(&archived_root)?;
        let destination_root = archived_root.join(workspace_id);
        if destination_root.exists() {
            return Err(WorkspaceError::WorkspaceDestinationExists);
        }
        append_audit_event(
            &source_root.join("audit.jsonl"),
            "workspace_archived",
            &manifest.workspace,
            unix_time_ms()?,
        )?;
        fs::rename(source_root, destination_root)?;
        self.list()
    }

    pub fn restore_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<WorkspaceCatalog, WorkspaceError> {
        let (source_root, manifest) = self.workspace_for_management(workspace_id, true)?;
        let projects_root = self.projects_root();
        fs::create_dir_all(&projects_root)?;
        let destination_root = projects_root.join(workspace_id);
        if destination_root.exists() {
            return Err(WorkspaceError::WorkspaceDestinationExists);
        }
        append_audit_event(
            &source_root.join("audit.jsonl"),
            "workspace_restored",
            &manifest.workspace,
            unix_time_ms()?,
        )?;
        fs::rename(source_root, destination_root)?;
        self.list()
    }

    pub fn delete_workspace(
        &self,
        workspace_id: &str,
        archived: bool,
        author_confirmed: bool,
    ) -> Result<WorkspaceCatalog, WorkspaceError> {
        if !author_confirmed {
            return Err(WorkspaceError::AuthorConfirmationRequired);
        }
        let (workspace_root, _) = self.workspace_for_management(workspace_id, archived)?;
        remove_generated_directory(&workspace_root)?;
        self.list()
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

    pub fn save_journal_recommendation_profile(
        &self,
        workspace_id: &str,
        input: JournalRecommendationProfileInput,
    ) -> Result<JournalRecommendationProfile, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let input = input
            .normalized()
            .map_err(|_| WorkspaceError::InvalidJournalProfile)?;
        let saved_unix_ms = unix_time_ms()?;
        if deadline_days_remaining(&input.submission_deadline, saved_unix_ms).unwrap_or(0) == 0 {
            return Err(WorkspaceError::InvalidJournalProfile);
        }
        let encoded = serde_json::to_vec(&(workspace_id, &input))
            .map_err(|error| WorkspaceError::InvalidManifest(error.to_string()))?;
        let profile_id = format!(
            "jmp-{}",
            hex::encode(Sha256::digest(encoded))
                .chars()
                .take(20)
                .collect::<String>()
        );
        let analysis_root = workspace_root.join("analysis");
        fs::create_dir_all(&analysis_root)?;
        let profile_path = analysis_root.join(format!("journal-profile-{profile_id}.json"));
        if profile_path.exists() {
            return read_json(&profile_path);
        }
        let profile_version = fs::read_dir(&analysis_root)?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("journal-profile-jmp-")
            })
            .count() as u32
            + 1;
        let profile = JournalRecommendationProfile {
            schema_version: JOURNAL_PROFILE_SCHEMA_VERSION,
            profile_id,
            profile_version,
            workspace_id: workspace_id.to_owned(),
            author_name: input.author_name,
            institution: input.institution,
            specialty: input.specialty,
            manuscript_purpose: input.manuscript_purpose,
            submission_deadline: input.submission_deadline,
            saved_unix_ms,
            institution_rule_evidence: InstitutionRuleEvidence::default(),
            external_transmission: "not_performed".into(),
        };
        let temporary_path = analysis_root.join(format!(".{}.tmp", Uuid::new_v4()));
        write_json(&temporary_path, &profile)?;
        fs::rename(&temporary_path, &profile_path)?;
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "journal_recommendation_profile_saved",
            &manifest.workspace,
            saved_unix_ms,
        )?;
        Ok(profile)
    }

    pub fn recommend_journals(
        &self,
        workspace_id: &str,
        profile_id: &str,
        preferences: JournalMatchPreferences,
    ) -> Result<JournalRecommendationRun, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        if !profile_id.starts_with("jmp-")
            || profile_id.len() != 24
            || !profile_id[4..].bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(WorkspaceError::JournalProfileNotFound);
        }
        let workspace_root = self.projects_root().join(workspace_id);
        let profile_path = workspace_root
            .join("analysis")
            .join(format!("journal-profile-{profile_id}.json"));
        let profile: JournalRecommendationProfile = if profile_path.exists() {
            read_json(&profile_path)?
        } else {
            return Err(WorkspaceError::JournalProfileNotFound);
        };
        let evaluated_unix_ms = (unix_time_ms()? / 86_400_000) * 86_400_000;
        if profile.workspace_id != workspace_id
            || deadline_days_remaining(&profile.submission_deadline, evaluated_unix_ms).unwrap_or(0)
                == 0
        {
            return Err(WorkspaceError::InvalidJournalProfile);
        }
        let report = self.analyze_structure(workspace_id)?;
        let run = recommend_journals(&report, profile, preferences, evaluated_unix_ms);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        let analysis_root = workspace_root.join("analysis");
        fs::create_dir_all(&analysis_root)?;
        let run_path = analysis_root.join(format!("journal-match-{}.json", run.run_id));
        if !run_path.exists() {
            let temporary_path = analysis_root.join(format!(".{}.tmp", Uuid::new_v4()));
            write_json(&temporary_path, &run)?;
            match fs::rename(&temporary_path, &run_path) {
                Ok(()) => {}
                Err(_) if run_path.exists() => {
                    let _ = fs::remove_file(temporary_path);
                }
                Err(error) => return Err(WorkspaceError::Io(error)),
            }
        }
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "journal_recommendations_computed",
            &manifest.workspace,
            unix_time_ms()?,
        )?;
        Ok(run)
    }

    pub fn save_institution_rule_evidence(
        &self,
        workspace_id: &str,
        base_profile_id: &str,
        evidence: InstitutionRuleEvidence,
    ) -> Result<JournalRecommendationProfile, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        if !base_profile_id.starts_with("jmp-")
            || base_profile_id.len() != 24
            || !base_profile_id[4..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(WorkspaceError::JournalProfileNotFound);
        }
        let valid_hash = evidence.source_text_hash.as_ref().is_some_and(|hash| {
            hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
        let valid_official_url = evidence
            .source_urls
            .iter()
            .all(|url| url.starts_with("https://") && url.chars().count() <= 1_000);
        let has_source = (!evidence.source_urls.is_empty() && valid_official_url) || valid_hash;
        let verification_valid =
            evidence.status != InstitutionRuleStatus::Verified || evidence.author_attested_official;
        let conditions_valid = evidence
            .minimum_cas_partition
            .is_none_or(|zone| (1..=4).contains(&zone))
            && evidence.extracted_conditions.len() <= 40
            && evidence
                .extracted_conditions
                .iter()
                .all(|condition| !condition.trim().is_empty() && condition.chars().count() <= 500);
        if !matches!(
            evidence.status,
            InstitutionRuleStatus::Verified | InstitutionRuleStatus::CandidateSourcesFound
        ) || evidence.rule_set_id.as_deref().is_none_or(str::is_empty)
            || evidence
                .rule_set_version
                .as_deref()
                .is_none_or(str::is_empty)
            || !has_source
            || !verification_valid
            || !conditions_valid
        {
            return Err(WorkspaceError::InvalidInstitutionRuleEvidence);
        }
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let analysis_root = workspace_root.join("analysis");
        let base_path = analysis_root.join(format!("journal-profile-{base_profile_id}.json"));
        let base: JournalRecommendationProfile = if base_path.exists() {
            read_json(&base_path)?
        } else {
            return Err(WorkspaceError::JournalProfileNotFound);
        };
        if base.workspace_id != workspace_id {
            return Err(WorkspaceError::JournalProfileNotFound);
        }
        let encoded = serde_json::to_vec(&(base_profile_id, &evidence))
            .map_err(|error| WorkspaceError::InvalidManifest(error.to_string()))?;
        let profile_id = format!(
            "jmp-{}",
            hex::encode(Sha256::digest(encoded))
                .chars()
                .take(20)
                .collect::<String>()
        );
        let profile_path = analysis_root.join(format!("journal-profile-{profile_id}.json"));
        if profile_path.exists() {
            return read_json(&profile_path);
        }
        let profile_version = fs::read_dir(&analysis_root)?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("journal-profile-jmp-")
            })
            .count() as u32
            + 1;
        let saved_unix_ms = unix_time_ms()?;
        let profile = JournalRecommendationProfile {
            schema_version: JOURNAL_PROFILE_SCHEMA_VERSION,
            profile_id,
            profile_version,
            workspace_id: base.workspace_id,
            author_name: base.author_name,
            institution: base.institution,
            specialty: base.specialty,
            manuscript_purpose: base.manuscript_purpose,
            submission_deadline: base.submission_deadline,
            saved_unix_ms,
            institution_rule_evidence: evidence,
            external_transmission:
                "performed_to_configured_model_institution_and_redacted_rule_text".into(),
        };
        let temporary_path = analysis_root.join(format!(".{}.tmp", Uuid::new_v4()));
        write_json(&temporary_path, &profile)?;
        fs::rename(&temporary_path, &profile_path)?;
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "institution_rule_evidence_saved",
            &manifest.workspace,
            saved_unix_ms,
        )?;
        Ok(profile)
    }

    pub fn journal_recommendation_profile(
        &self,
        workspace_id: &str,
        profile_id: &str,
    ) -> Result<JournalRecommendationProfile, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        if !profile_id.starts_with("jmp-")
            || profile_id.len() != 24
            || !profile_id[4..].bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(WorkspaceError::JournalProfileNotFound);
        }
        let profile_path = self
            .projects_root()
            .join(workspace_id)
            .join("analysis")
            .join(format!("journal-profile-{profile_id}.json"));
        let profile: JournalRecommendationProfile = if profile_path.exists() {
            read_json(&profile_path)?
        } else {
            return Err(WorkspaceError::JournalProfileNotFound);
        };
        if profile.workspace_id != workspace_id {
            return Err(WorkspaceError::JournalProfileNotFound);
        }
        Ok(profile)
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

    pub fn lifecycle(&self, workspace_id: &str) -> Result<WorkspaceLifecycle, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let structure_report = read_current_structure_report(&workspace_root, &manifest.workspace)?;
        let readiness_report = read_current_readiness_report(&workspace_root, &manifest.workspace)?;
        let attestation = match &readiness_report {
            Some(report) => read_current_attestation(&workspace_root, &manifest.workspace, report)?,
            None => None,
        };
        let submission = match &attestation {
            Some(attestation) => {
                read_current_submission(&workspace_root, &manifest.workspace, attestation)?
            }
            None => None,
        };
        let knowledge_body = match &submission {
            Some(submission) => {
                read_current_knowledge_body(&workspace_root, &manifest.workspace, submission)?
            }
            None => None,
        };
        Ok(WorkspaceLifecycle {
            workspace_id: workspace_id.to_owned(),
            current_version: manifest.workspace.snapshot_version,
            structure_report,
            readiness_report,
            attestation,
            submission,
            knowledge_body,
        })
    }

    pub fn create_local_attestation(
        &self,
        workspace_id: &str,
        author_confirmed: bool,
    ) -> Result<LocalAttestation, WorkspaceError> {
        if !author_confirmed {
            return Err(WorkspaceError::AuthorConfirmationRequired);
        }
        let lifecycle = self.lifecycle(workspace_id)?;
        let report = lifecycle
            .readiness_report
            .ok_or(WorkspaceError::MissingCurrentReadiness)?;
        if let Some(existing) = lifecycle.attestation {
            return Ok(existing);
        }
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        let attestation_id = Uuid::new_v4().to_string();
        let attested_unix_ms = unix_time_ms()?;
        let statement = "作者确认当前稿件版本、检查报告及待确认事项构成本次本地存证边界；该记录不证明研究结论为真。".to_owned();
        let external_transmission = "not_performed".to_owned();
        let payload = AttestationPayload {
            attestation_id: &attestation_id,
            workspace_id,
            manuscript_version: manifest.workspace.snapshot_version,
            manuscript_hash: &manifest.workspace.content_hash,
            readiness_report_id: &report.report_id,
            readiness_output_snapshot_version: report.output_snapshot_version,
            readiness_outcome: report.outcome,
            attested_unix_ms,
            statement: &statement,
            external_transmission: &external_transmission,
        };
        let record_hash = hash_serializable(&payload)?;
        let record = LocalAttestation {
            attestation_id: attestation_id.clone(),
            workspace_id: workspace_id.to_owned(),
            manuscript_version: manifest.workspace.snapshot_version,
            manuscript_hash: manifest.workspace.content_hash.clone(),
            readiness_report_id: report.report_id,
            readiness_output_snapshot_version: report.output_snapshot_version,
            readiness_outcome: report.outcome,
            attested_unix_ms,
            statement,
            record_hash,
            external_transmission,
        };
        write_immutable_record(
            &workspace_root.join("attestations"),
            &attestation_id,
            "attestation.json",
            &record,
        )?;
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "local_attestation_created",
            &manifest.workspace,
            attested_unix_ms,
        )?;
        Ok(record)
    }

    pub fn export_submission_package(
        &self,
        workspace_id: &str,
        destination: &Path,
    ) -> Result<SubmissionExport, WorkspaceError> {
        if !destination.is_dir() {
            return Err(WorkspaceError::InvalidExportDestination);
        }
        let lifecycle = self.lifecycle(workspace_id)?;
        let report = lifecycle
            .readiness_report
            .ok_or(WorkspaceError::MissingCurrentReadiness)?;
        let attestation = lifecycle
            .attestation
            .ok_or(WorkspaceError::MissingCurrentAttestation)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        let package_name = format!(
            "ManuscriptDock-{}-v{}",
            &workspace_id[..8],
            manifest.workspace.snapshot_version
        );
        let final_root = destination.join(&package_name);
        if final_root.exists() {
            return Err(WorkspaceError::ExportDestinationExists);
        }
        let temporary_root = destination.join(format!(".manuscriptdock-{}.tmp", Uuid::new_v4()));
        fs::create_dir(&temporary_root)?;
        let exported_unix_ms = unix_time_ms()?;
        let files = vec![
            format!("manuscript.{}", manifest.workspace.manuscript.extension),
            "readiness-report.json".to_owned(),
            "readiness-preview.html".to_owned(),
            "local-attestation.json".to_owned(),
            "submission-manifest.json".to_owned(),
        ];
        let result = (|| {
            let snapshot = self.source_snapshot_path(workspace_id)?;
            verify_snapshot(&snapshot, &manifest.workspace)?;
            fs::copy(&snapshot, temporary_root.join(&files[0]))?;
            let report_root = readiness_output_root(&workspace_root, &report.report_id);
            fs::copy(
                report_root.join(format!("readiness-v{}.json", report.report_version)),
                temporary_root.join(&files[1]),
            )?;
            fs::copy(
                report_root.join("preview.html"),
                temporary_root.join(&files[2]),
            )?;
            fs::copy(
                workspace_root
                    .join("attestations")
                    .join(&attestation.attestation_id)
                    .join("attestation.json"),
                temporary_root.join(&files[3]),
            )?;
            let package_manifest = SubmissionPackageManifest {
                schema_version: 1,
                workspace_id,
                manuscript_version: manifest.workspace.snapshot_version,
                manuscript_hash: &manifest.workspace.content_hash,
                readiness_report_id: &report.report_id,
                attestation_id: &attestation.attestation_id,
                attestation_hash: &attestation.record_hash,
                created_unix_ms: exported_unix_ms,
                files: &files[..4],
                external_transmission: "not_performed",
            };
            write_json(&temporary_root.join(&files[4]), &package_manifest)?;
            fs::rename(&temporary_root, &final_root)?;
            append_audit_event(
                &workspace_root.join("audit.jsonl"),
                "submission_package_exported",
                &manifest.workspace,
                exported_unix_ms,
            )?;
            Ok(SubmissionExport {
                package_name,
                manuscript_version: manifest.workspace.snapshot_version,
                attestation_id: attestation.attestation_id,
                files,
                exported_unix_ms,
                external_transmission: "not_performed".to_owned(),
            })
        })();
        if temporary_root.exists() {
            let _ = remove_generated_directory(&temporary_root);
        }
        result
    }

    pub fn record_manual_submission(
        &self,
        workspace_id: &str,
        target: &str,
        receipt: Option<&str>,
        author_confirmed: bool,
    ) -> Result<SubmissionRecord, WorkspaceError> {
        if !author_confirmed {
            return Err(WorkspaceError::AuthorConfirmationRequired);
        }
        let target = target.trim();
        if target.is_empty() || target.chars().count() > 200 {
            return Err(WorkspaceError::InvalidSubmissionTarget);
        }
        let receipt = receipt
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);
        if receipt
            .as_ref()
            .is_some_and(|value| value.chars().count() > 200)
        {
            return Err(WorkspaceError::InvalidSubmissionTarget);
        }
        let lifecycle = self.lifecycle(workspace_id)?;
        let attestation = lifecycle
            .attestation
            .ok_or(WorkspaceError::MissingCurrentAttestation)?;
        if let Some(existing) = lifecycle.submission {
            return Ok(existing);
        }
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        let submission_id = Uuid::new_v4().to_string();
        let submitted_unix_ms = unix_time_ms()?;
        let statement =
            "作者确认已在外部投稿系统完成提交；ManuscriptDock 仅保存本地登记，不执行网络投稿。"
                .to_owned();
        let external_transmission = "not_performed".to_owned();
        let payload = SubmissionPayload {
            submission_id: &submission_id,
            workspace_id,
            manuscript_version: manifest.workspace.snapshot_version,
            attestation_id: &attestation.attestation_id,
            target,
            receipt: &receipt,
            submitted_unix_ms,
            statement: &statement,
            external_transmission: &external_transmission,
        };
        let record_hash = hash_serializable(&payload)?;
        let record = SubmissionRecord {
            submission_id: submission_id.clone(),
            workspace_id: workspace_id.to_owned(),
            manuscript_version: manifest.workspace.snapshot_version,
            attestation_id: attestation.attestation_id,
            target: target.to_owned(),
            receipt,
            submitted_unix_ms,
            statement,
            record_hash,
            external_transmission,
        };
        write_immutable_record(
            &workspace_root.join("submissions"),
            &submission_id,
            "submission.json",
            &record,
        )?;
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "manual_submission_recorded",
            &manifest.workspace,
            submitted_unix_ms,
        )?;
        Ok(record)
    }

    pub fn finalize_knowledge_body(
        &self,
        workspace_id: &str,
        discipline_code: &str,
    ) -> Result<KnowledgeBodyRecord, WorkspaceError> {
        let discipline = discipline_catalog_item(discipline_code.trim())
            .ok_or(WorkspaceError::InvalidDisciplineClassification)?;
        let lifecycle = self.lifecycle(workspace_id)?;
        let attestation = lifecycle
            .attestation
            .ok_or(WorkspaceError::MissingCurrentAttestation)?;
        let submission = lifecycle
            .submission
            .ok_or(WorkspaceError::MissingCurrentSubmission)?;
        let existing = lifecycle.knowledge_body;
        if let Some(existing_record) = &existing {
            if existing_record
                .discipline_classification
                .as_ref()
                .is_some_and(|classification| classification.code == discipline.code)
            {
                return Ok(existing_record.clone());
            }
        }
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        let snapshot = existing
            .as_ref()
            .map(|record| record.snapshot.clone())
            .unwrap_or_else(|| local_knowledge_body_snapshot(&manifest.workspace));
        snapshot.validate()?;
        let previous_classification = existing
            .as_ref()
            .and_then(|record| record.discipline_classification.as_ref());
        let discipline_classification = DisciplineClassification {
            assignment_id: previous_classification
                .map(|classification| classification.assignment_id.clone())
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            version: previous_classification
                .map(|classification| classification.version + 1)
                .unwrap_or(1),
            scheme: DISCIPLINE_INDEX_SCHEME.to_owned(),
            scheme_version: DISCIPLINE_INDEX_VERSION.to_owned(),
            code: discipline.code,
            label: discipline.label,
            label_en: discipline.label_en,
            status: "author_confirmed".to_owned(),
            basis: "author_selection".to_owned(),
        };
        let record_id = Uuid::new_v4().to_string();
        let finalized_unix_ms = unix_time_ms()?.max(
            existing
                .as_ref()
                .map(|record| record.finalized_unix_ms.saturating_add(1))
                .unwrap_or(0),
        );
        let external_transmission = "not_performed".to_owned();
        let payload = KnowledgeBodyPayload {
            record_id: &record_id,
            workspace_id,
            manuscript_version: manifest.workspace.snapshot_version,
            attestation_id: &attestation.attestation_id,
            submission_id: &submission.submission_id,
            finalized_unix_ms,
            discipline_classification: &discipline_classification,
            snapshot: &snapshot,
            external_transmission: &external_transmission,
        };
        let record_hash = hash_serializable(&payload)?;
        let record = KnowledgeBodyRecord {
            record_id: record_id.clone(),
            workspace_id: workspace_id.to_owned(),
            manuscript_version: manifest.workspace.snapshot_version,
            attestation_id: attestation.attestation_id,
            submission_id: submission.submission_id,
            finalized_unix_ms,
            discipline_classification: Some(discipline_classification),
            snapshot,
            record_hash,
            external_transmission,
        };
        write_immutable_record(
            &workspace_root.join("knowledge"),
            &record_id,
            "knowledge-body.json",
            &record,
        )?;
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "knowledge_body_finalized",
            &manifest.workspace,
            finalized_unix_ms,
        )?;
        Ok(record)
    }

    pub fn knowledge_dialogue(
        &self,
        workspace_id: &str,
    ) -> Result<KnowledgeDialogueLedger, WorkspaceError> {
        let lifecycle = self.lifecycle(workspace_id)?;
        let knowledge_body = lifecycle
            .knowledge_body
            .ok_or(WorkspaceError::MissingCurrentKnowledgeBody)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let mut inquiries = read_nested_records::<KnowledgeInquiryRecord>(
            &workspace_root.join("dialogue/inquiries"),
            "inquiry.json",
        )?;
        for inquiry in &inquiries {
            verify_knowledge_inquiry(inquiry)?;
        }
        inquiries.retain(|inquiry| {
            inquiry.workspace_id == workspace_id
                && inquiry.knowledge_body_record_id == knowledge_body.record_id
                && inquiry.knowledge_body_hash == knowledge_body.record_hash
        });
        inquiries.sort_by_key(|inquiry| inquiry.created_unix_ms);

        let mut answers = read_nested_records::<KnowledgeAnswerRecord>(
            &workspace_root.join("dialogue/answers"),
            "answer.json",
        )?;
        for answer in &answers {
            verify_knowledge_answer(answer)?;
        }
        answers.retain(|answer| {
            answer.workspace_id == workspace_id
                && answer.knowledge_body_record_id == knowledge_body.record_id
        });
        answers.sort_by_key(|answer| answer.created_unix_ms);

        let items = inquiries
            .into_iter()
            .map(|inquiry| KnowledgeDialogueItem {
                answers: answers
                    .iter()
                    .filter(|answer| answer.inquiry_id == inquiry.inquiry_id)
                    .cloned()
                    .collect(),
                inquiry,
            })
            .collect();
        Ok(KnowledgeDialogueLedger {
            workspace_id: workspace_id.to_owned(),
            knowledge_body_record_id: knowledge_body.record_id,
            knowledge_body_hash: knowledge_body.record_hash,
            items,
        })
    }

    pub fn create_owner_inquiry(
        &self,
        workspace_id: &str,
        stance: KnowledgeInquiryStance,
        target: KnowledgeInquiryTarget,
        question: &str,
        author_confirmed_model_projection: bool,
    ) -> Result<KnowledgeInquiryRecord, WorkspaceError> {
        if !author_confirmed_model_projection {
            return Err(WorkspaceError::AuthorConfirmationRequired);
        }
        let question = question.trim();
        if question.is_empty() || question.chars().count() > 4_000 {
            return Err(WorkspaceError::InvalidKnowledgeInquiry);
        }
        let lifecycle = self.lifecycle(workspace_id)?;
        let knowledge_body = lifecycle
            .knowledge_body
            .ok_or(WorkspaceError::MissingCurrentKnowledgeBody)?;
        let inquiry_id = Uuid::new_v4().to_string();
        let created_unix_ms = unix_time_ms()?;
        let external_actor_label = None;
        let external_transmission = "author_confirmed_model_projection".to_owned();
        let payload = KnowledgeInquiryPayload {
            schema_version: KNOWLEDGE_DIALOGUE_SCHEMA_VERSION,
            inquiry_id: &inquiry_id,
            workspace_id,
            knowledge_body_record_id: &knowledge_body.record_id,
            knowledge_body_hash: &knowledge_body.record_hash,
            snapshot_version: knowledge_body.snapshot.snapshot_version,
            origin: KnowledgeInquiryOrigin::Owner,
            stance,
            target,
            question,
            external_actor_label: &external_actor_label,
            created_unix_ms,
            external_transmission: &external_transmission,
        };
        let record_hash = hash_serializable(&payload)?;
        let record = KnowledgeInquiryRecord {
            schema_version: KNOWLEDGE_DIALOGUE_SCHEMA_VERSION,
            inquiry_id: inquiry_id.clone(),
            workspace_id: workspace_id.to_owned(),
            knowledge_body_record_id: knowledge_body.record_id,
            knowledge_body_hash: knowledge_body.record_hash,
            snapshot_version: knowledge_body.snapshot.snapshot_version,
            origin: KnowledgeInquiryOrigin::Owner,
            stance,
            target,
            question: question.to_owned(),
            external_actor_label,
            created_unix_ms,
            record_hash,
            external_transmission,
        };
        let workspace_root = self.projects_root().join(workspace_id);
        write_immutable_record(
            &workspace_root.join("dialogue/inquiries"),
            &inquiry_id,
            "inquiry.json",
            &record,
        )?;
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "knowledge_inquiry_created",
            &manifest.workspace,
            created_unix_ms,
        )?;
        Ok(record)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_model_answer(
        &self,
        workspace_id: &str,
        inquiry_id: &str,
        model_slot: &str,
        provider_label: &str,
        model: &str,
        answer: &str,
        source_anchors: &[crate::VersionedObjectReference],
    ) -> Result<KnowledgeAnswerRecord, WorkspaceError> {
        let provider_label = provider_label.trim();
        let model = model.trim();
        let answer = answer.trim();
        if !matches!(model_slot, "primary" | "fallback_1" | "fallback_2")
            || provider_label.is_empty()
            || provider_label.chars().count() > 100
            || model.is_empty()
            || model.chars().count() > 200
            || answer.is_empty()
            || answer.chars().count() > 24_000
        {
            return Err(WorkspaceError::InvalidKnowledgeAnswer);
        }
        let ledger = self.knowledge_dialogue(workspace_id)?;
        if !ledger
            .items
            .iter()
            .any(|item| item.inquiry.inquiry_id == inquiry_id)
        {
            return Err(WorkspaceError::KnowledgeInquiryNotFound);
        }
        let answer_id = Uuid::new_v4().to_string();
        let created_unix_ms = unix_time_ms()?;
        let external_transmission = "performed_to_configured_model".to_owned();
        let payload = KnowledgeAnswerPayload {
            schema_version: KNOWLEDGE_DIALOGUE_SCHEMA_VERSION,
            answer_id: &answer_id,
            inquiry_id,
            workspace_id,
            knowledge_body_record_id: &ledger.knowledge_body_record_id,
            model_slot,
            provider_label,
            model,
            answer,
            source_anchors,
            created_unix_ms,
            external_transmission: &external_transmission,
        };
        let record_hash = hash_serializable(&payload)?;
        let record = KnowledgeAnswerRecord {
            schema_version: KNOWLEDGE_DIALOGUE_SCHEMA_VERSION,
            answer_id: answer_id.clone(),
            inquiry_id: inquiry_id.to_owned(),
            workspace_id: workspace_id.to_owned(),
            knowledge_body_record_id: ledger.knowledge_body_record_id,
            model_slot: model_slot.to_owned(),
            provider_label: provider_label.to_owned(),
            model: model.to_owned(),
            answer: answer.to_owned(),
            source_anchors: source_anchors.to_vec(),
            created_unix_ms,
            record_hash,
            external_transmission,
        };
        let workspace_root = self.projects_root().join(workspace_id);
        write_immutable_record(
            &workspace_root.join("dialogue/answers"),
            &answer_id,
            "answer.json",
            &record,
        )?;
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "knowledge_model_answer_recorded",
            &manifest.workspace,
            created_unix_ms,
        )?;
        Ok(record)
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

    fn archived_projects_root(&self) -> PathBuf {
        self.root.join("archived-projects")
    }

    fn workspace_for_management(
        &self,
        workspace_id: &str,
        archived: bool,
    ) -> Result<(PathBuf, WorkspaceManifest), WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let collection_root = if archived {
            self.archived_projects_root()
        } else {
            self.projects_root()
        };
        let workspace_root = collection_root.join(workspace_id);
        let metadata =
            fs::symlink_metadata(&workspace_root).map_err(|error| match error.kind() {
                io::ErrorKind::NotFound => WorkspaceError::WorkspaceNotFound,
                _ => WorkspaceError::Io(error),
            })?;
        if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        Ok((workspace_root, manifest))
    }
}

fn read_current_structure_report(
    workspace_root: &Path,
    workspace: &WorkspaceSummary,
) -> Result<Option<StructureReport>, WorkspaceError> {
    let hash_prefix = workspace
        .content_hash
        .get(..12)
        .ok_or_else(|| WorkspaceError::InvalidManifest("内容指纹长度无效".to_owned()))?;
    let path = workspace_root.join("analysis").join(format!(
        "structure-v{STRUCTURE_ANALYSIS_VERSION}-{hash_prefix}.json"
    ));
    if !path.is_file() {
        return Ok(None);
    }
    let report: StructureReport = read_json(&path)?;
    if report.workspace_id != workspace.id
        || report.source_content_hash != workspace.content_hash
        || report.source_snapshot_version != workspace.snapshot_version
    {
        return Ok(None);
    }
    Ok(Some(report))
}

fn read_current_readiness_report(
    workspace_root: &Path,
    workspace: &WorkspaceSummary,
) -> Result<Option<ReadinessReport>, WorkspaceError> {
    let mut reports = read_nested_records::<ReadinessReport>(
        &workspace_root.join("outputs"),
        &format!("readiness-v{READINESS_REPORT_VERSION}.json"),
    )?;
    reports.retain(|report| {
        report.workspace_id == workspace.id
            && report.source_content_hash == workspace.content_hash
            && report.source_snapshot_version == workspace.snapshot_version
    });
    reports.sort_by_key(|report| report.generated_unix_ms);
    Ok(reports.pop())
}

fn read_current_attestation(
    workspace_root: &Path,
    workspace: &WorkspaceSummary,
    report: &ReadinessReport,
) -> Result<Option<LocalAttestation>, WorkspaceError> {
    let mut records = read_nested_records::<LocalAttestation>(
        &workspace_root.join("attestations"),
        "attestation.json",
    )?;
    for record in &records {
        verify_attestation_record(record)?;
    }
    records.retain(|record| {
        record.workspace_id == workspace.id
            && record.manuscript_version == workspace.snapshot_version
            && record.manuscript_hash == workspace.content_hash
            && record.readiness_report_id == report.report_id
    });
    records.sort_by_key(|record| record.attested_unix_ms);
    Ok(records.pop())
}

fn read_current_submission(
    workspace_root: &Path,
    workspace: &WorkspaceSummary,
    attestation: &LocalAttestation,
) -> Result<Option<SubmissionRecord>, WorkspaceError> {
    let mut records = read_nested_records::<SubmissionRecord>(
        &workspace_root.join("submissions"),
        "submission.json",
    )?;
    for record in &records {
        verify_submission_record(record)?;
    }
    records.retain(|record| {
        record.workspace_id == workspace.id
            && record.manuscript_version == workspace.snapshot_version
            && record.attestation_id == attestation.attestation_id
    });
    records.sort_by_key(|record| record.submitted_unix_ms);
    Ok(records.pop())
}

fn read_current_knowledge_body(
    workspace_root: &Path,
    workspace: &WorkspaceSummary,
    submission: &SubmissionRecord,
) -> Result<Option<KnowledgeBodyRecord>, WorkspaceError> {
    let mut records = read_nested_records::<KnowledgeBodyRecord>(
        &workspace_root.join("knowledge"),
        "knowledge-body.json",
    )?;
    for record in &records {
        verify_knowledge_body_record(record)?;
    }
    records.retain(|record| {
        record.workspace_id == workspace.id
            && record.manuscript_version == workspace.snapshot_version
            && record.submission_id == submission.submission_id
    });
    records.sort_by_key(|record| record.finalized_unix_ms);
    let record = records.pop();
    if let Some(record) = &record {
        record.snapshot.validate()?;
    }
    Ok(record)
}

fn read_nested_records<T: DeserializeOwned>(
    root: &Path,
    file_name: &str,
) -> Result<Vec<T>, WorkspaceError> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() || entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path().join(file_name);
        if path.is_file() {
            records.push(read_json(&path)?);
        }
    }
    Ok(records)
}

fn readiness_output_root(workspace_root: &Path, report_id: &str) -> PathBuf {
    workspace_root.join("outputs").join(report_id)
}

fn write_immutable_record(
    collection_root: &Path,
    record_id: &str,
    file_name: &str,
    value: &impl Serialize,
) -> Result<(), WorkspaceError> {
    fs::create_dir_all(collection_root)?;
    let temporary_root = collection_root.join(format!(".{record_id}.tmp"));
    let final_root = collection_root.join(record_id);
    if final_root.exists() {
        return Err(WorkspaceError::InvalidManifest(
            "不可变记录标识已存在，未覆盖任何文件".to_owned(),
        ));
    }
    fs::create_dir(&temporary_root)?;
    let result = (|| {
        let path = temporary_root.join(file_name);
        write_json(&path, value)?;
        set_readonly(&path)?;
        fs::rename(&temporary_root, &final_root)?;
        Ok(())
    })();
    if temporary_root.exists() {
        let _ = remove_generated_directory(&temporary_root);
    }
    result
}

fn hash_serializable(value: &impl Serialize) -> Result<String, WorkspaceError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| WorkspaceError::InvalidManifest(error.to_string()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn verify_attestation_record(record: &LocalAttestation) -> Result<(), WorkspaceError> {
    let payload = AttestationPayload {
        attestation_id: &record.attestation_id,
        workspace_id: &record.workspace_id,
        manuscript_version: record.manuscript_version,
        manuscript_hash: &record.manuscript_hash,
        readiness_report_id: &record.readiness_report_id,
        readiness_output_snapshot_version: record.readiness_output_snapshot_version,
        readiness_outcome: record.readiness_outcome,
        attested_unix_ms: record.attested_unix_ms,
        statement: &record.statement,
        external_transmission: &record.external_transmission,
    };
    if hash_serializable(&payload)? != record.record_hash {
        return Err(WorkspaceError::InvalidManifest(
            "本地存证记录完整性验证失败".to_owned(),
        ));
    }
    Ok(())
}

fn verify_submission_record(record: &SubmissionRecord) -> Result<(), WorkspaceError> {
    let payload = SubmissionPayload {
        submission_id: &record.submission_id,
        workspace_id: &record.workspace_id,
        manuscript_version: record.manuscript_version,
        attestation_id: &record.attestation_id,
        target: &record.target,
        receipt: &record.receipt,
        submitted_unix_ms: record.submitted_unix_ms,
        statement: &record.statement,
        external_transmission: &record.external_transmission,
    };
    if hash_serializable(&payload)? != record.record_hash {
        return Err(WorkspaceError::InvalidManifest(
            "投稿登记记录完整性验证失败".to_owned(),
        ));
    }
    Ok(())
}

fn verify_knowledge_body_record(record: &KnowledgeBodyRecord) -> Result<(), WorkspaceError> {
    record.snapshot.validate()?;
    let expected_hash = match &record.discipline_classification {
        Some(classification) => {
            let catalog_item = discipline_catalog_item(&classification.code).ok_or_else(|| {
                WorkspaceError::InvalidManifest("知识体学科索引记录无效".to_owned())
            })?;
            if classification.version == 0
                || classification.assignment_id.trim().is_empty()
                || classification.label != catalog_item.label
                || classification.label_en != catalog_item.label_en
                || classification.scheme != DISCIPLINE_INDEX_SCHEME
                || classification.scheme_version != DISCIPLINE_INDEX_VERSION
                || classification.status != "author_confirmed"
                || classification.basis != "author_selection"
            {
                return Err(WorkspaceError::InvalidManifest(
                    "知识体学科索引记录无效".to_owned(),
                ));
            }
            hash_serializable(&KnowledgeBodyPayload {
                record_id: &record.record_id,
                workspace_id: &record.workspace_id,
                manuscript_version: record.manuscript_version,
                attestation_id: &record.attestation_id,
                submission_id: &record.submission_id,
                finalized_unix_ms: record.finalized_unix_ms,
                discipline_classification: classification,
                snapshot: &record.snapshot,
                external_transmission: &record.external_transmission,
            })?
        }
        None => hash_serializable(&LegacyKnowledgeBodyPayload {
            record_id: &record.record_id,
            workspace_id: &record.workspace_id,
            manuscript_version: record.manuscript_version,
            attestation_id: &record.attestation_id,
            submission_id: &record.submission_id,
            finalized_unix_ms: record.finalized_unix_ms,
            snapshot: &record.snapshot,
            external_transmission: &record.external_transmission,
        })?,
    };
    if expected_hash != record.record_hash {
        return Err(WorkspaceError::InvalidManifest(
            "知识体快照记录完整性验证失败".to_owned(),
        ));
    }
    Ok(())
}

fn verify_knowledge_inquiry(record: &KnowledgeInquiryRecord) -> Result<(), WorkspaceError> {
    let payload = KnowledgeInquiryPayload {
        schema_version: record.schema_version,
        inquiry_id: &record.inquiry_id,
        workspace_id: &record.workspace_id,
        knowledge_body_record_id: &record.knowledge_body_record_id,
        knowledge_body_hash: &record.knowledge_body_hash,
        snapshot_version: record.snapshot_version,
        origin: record.origin,
        stance: record.stance,
        target: record.target,
        question: &record.question,
        external_actor_label: &record.external_actor_label,
        created_unix_ms: record.created_unix_ms,
        external_transmission: &record.external_transmission,
    };
    if record.schema_version != KNOWLEDGE_DIALOGUE_SCHEMA_VERSION
        || record.inquiry_id.trim().is_empty()
        || record.question.trim().is_empty()
        || record.question.chars().count() > 4_000
        || hash_serializable(&payload)? != record.record_hash
    {
        return Err(WorkspaceError::InvalidManifest(
            "知识体问题记录完整性验证失败".to_owned(),
        ));
    }
    Ok(())
}

fn verify_knowledge_answer(record: &KnowledgeAnswerRecord) -> Result<(), WorkspaceError> {
    let payload = KnowledgeAnswerPayload {
        schema_version: record.schema_version,
        answer_id: &record.answer_id,
        inquiry_id: &record.inquiry_id,
        workspace_id: &record.workspace_id,
        knowledge_body_record_id: &record.knowledge_body_record_id,
        model_slot: &record.model_slot,
        provider_label: &record.provider_label,
        model: &record.model,
        answer: &record.answer,
        source_anchors: &record.source_anchors,
        created_unix_ms: record.created_unix_ms,
        external_transmission: &record.external_transmission,
    };
    if record.schema_version != KNOWLEDGE_DIALOGUE_SCHEMA_VERSION
        || record.answer_id.trim().is_empty()
        || record.inquiry_id.trim().is_empty()
        || record.answer.trim().is_empty()
        || !matches!(
            record.model_slot.as_str(),
            "primary" | "fallback_1" | "fallback_2"
        )
        || hash_serializable(&payload)? != record.record_hash
    {
        return Err(WorkspaceError::InvalidManifest(
            "知识体模型回答记录完整性验证失败".to_owned(),
        ));
    }
    Ok(())
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

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, WorkspaceError> {
    let reader = BufReader::new(File::open(path)?);
    serde_json::from_reader(reader)
        .map_err(|error| WorkspaceError::InvalidManifest(error.to_string()))
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
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            make_tree_writable(&entry_path)?;
        } else {
            let mut permissions = fs::symlink_metadata(&entry_path)?.permissions();
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
    use super::{
        make_tree_writable, VersionCreation, VersionOrigin, WorkspaceError, WorkspaceStore,
    };
    use crate::{
        InstitutionRuleEvidence, InstitutionRuleStatus, JournalMatchPreferences,
        JournalRecommendationProfileInput, KnowledgeInquiryStance, KnowledgeInquiryTarget,
        ManuscriptPurpose, ReadinessOutcome, RevisionApplication, RevisionChangeInput,
        RevisionFieldKind,
    };
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
    fn saves_a_versioned_submission_profile_before_journal_recommendation() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("journal-profile-study.tex");
        fs::write(
            &source_path,
            "\\title{Computer vision study}\n\\author{Synthetic Author}\n\\begin{abstract}Image segmentation evidence.\\end{abstract}\n\\section{Method}\nSynthetic method.\n\\section{Results}\nSynthetic result.\n\\section{Discussion}\nSynthetic discussion.\n\\section{References}\nSynthetic reference.",
        )
        .unwrap();
        let store_root = temporary.path().join("store");
        let store = WorkspaceStore::new(&store_root);
        let workspace = store.create_from_source(&source_path).unwrap();
        let input = JournalRecommendationProfileInput {
            author_name: "Synthetic Author".into(),
            institution: "Synthetic University".into(),
            specialty: "Computer vision".into(),
            manuscript_purpose: ManuscriptPurpose::DegreeRequirement,
            submission_deadline: "2099-12-31".into(),
        };

        let profile = store
            .save_journal_recommendation_profile(&workspace.id, input.clone())
            .unwrap();
        let same_profile = store
            .save_journal_recommendation_profile(&workspace.id, input)
            .unwrap();
        let run = store
            .recommend_journals(
                &workspace.id,
                &profile.profile_id,
                JournalMatchPreferences::default(),
            )
            .unwrap();

        assert_eq!(profile, same_profile);
        assert_eq!(run.recommendation_profile.profile_id, profile.profile_id);
        assert_eq!(run.domestic.len(), 3);
        assert_eq!(run.international.len(), 3);
        assert!(run.school_rule_status.contains("search_required"));
        let analysis_root = store_root
            .join("projects")
            .join(&workspace.id)
            .join("analysis");
        assert!(analysis_root
            .join(format!("journal-profile-{}.json", profile.profile_id))
            .is_file());
        assert!(analysis_root
            .join(format!("journal-match-{}.json", run.run_id))
            .is_file());
        let audit = fs::read_to_string(
            store_root
                .join("projects")
                .join(&workspace.id)
                .join("audit.jsonl"),
        )
        .unwrap();
        assert!(audit.contains("journal_recommendation_profile_saved"));
        assert!(audit.contains("journal_recommendations_computed"));

        let evidence = InstitutionRuleEvidence {
            status: InstitutionRuleStatus::Verified,
            rule_set_id: Some("institution-rule-synthetic".into()),
            rule_set_version: Some("author-source-1".into()),
            source_text_hash: Some("b".repeat(64)),
            source_kind: Some("author_supplied_institution_requirement".into()),
            extracted_conditions: vec!["Only CCF A is recognized".into()],
            recognized_rank_tiers: vec!["CCF A".into()],
            author_attested_official: true,
            ..InstitutionRuleEvidence::default()
        };
        let evidence_profile = store
            .save_institution_rule_evidence(&workspace.id, &profile.profile_id, evidence)
            .unwrap();
        let evidence_run = store
            .recommend_journals(
                &workspace.id,
                &evidence_profile.profile_id,
                JournalMatchPreferences::default(),
            )
            .unwrap();
        assert!(evidence_profile.profile_version > profile.profile_version);
        assert_eq!(evidence_run.school_rule_status, "verified_rule_set_applied");
        assert!(evidence_run
            .international
            .iter()
            .any(|item| item.scores.institution_rules == Some(100)));
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
    fn archives_restores_and_deletes_only_author_confirmed_workspaces() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("managed-study.tex");
        fs::write(&source_path, "\\title{Managed synthetic study}").unwrap();
        let store_root = temporary.path().join("store");
        let store = WorkspaceStore::new(&store_root);
        let workspace = store.create_from_source(&source_path).unwrap();

        let archived_catalog = store.archive_workspace(&workspace.id).unwrap();
        assert!(archived_catalog.workspaces.is_empty());
        assert_eq!(
            archived_catalog.archived_workspaces,
            vec![workspace.clone()]
        );
        assert!(!store_root.join("projects").join(&workspace.id).exists());
        let archived_root = store_root.join("archived-projects").join(&workspace.id);
        assert!(archived_root.exists());

        let restored_catalog = store.restore_workspace(&workspace.id).unwrap();
        assert_eq!(restored_catalog.workspaces, vec![workspace.clone()]);
        assert!(restored_catalog.archived_workspaces.is_empty());
        let restored_root = store_root.join("projects").join(&workspace.id);
        let audit = fs::read_to_string(restored_root.join("audit.jsonl")).unwrap();
        assert!(audit.contains("workspace_archived"));
        assert!(audit.contains("workspace_restored"));

        assert!(matches!(
            store.delete_workspace(&workspace.id, false, false),
            Err(WorkspaceError::AuthorConfirmationRequired)
        ));
        assert!(restored_root.exists());
        let deleted_catalog = store.delete_workspace(&workspace.id, false, true).unwrap();
        assert!(deleted_catalog.workspaces.is_empty());
        assert!(!restored_root.exists());

        let archived_only = store.create_from_source(&source_path).unwrap();
        store.archive_workspace(&archived_only.id).unwrap();
        let final_catalog = store
            .delete_workspace(&archived_only.id, true, true)
            .unwrap();
        assert!(final_catalog.workspaces.is_empty());
        assert!(final_catalog.archived_workspaces.is_empty());
        assert!(!store_root
            .join("archived-projects")
            .join(archived_only.id)
            .exists());
    }

    #[cfg(unix)]
    #[test]
    fn deleting_a_workspace_does_not_follow_an_injected_symbolic_link() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("symlink-study.tex");
        fs::write(&source_path, "\\title{Symlink boundary study}").unwrap();
        let store_root = temporary.path().join("store");
        let store = WorkspaceStore::new(&store_root);
        let workspace = store.create_from_source(&source_path).unwrap();
        let external_file = temporary.path().join("outside-workspace.txt");
        fs::write(&external_file, "must remain untouched").unwrap();
        fs::set_permissions(&external_file, fs::Permissions::from_mode(0o400)).unwrap();
        symlink(
            &external_file,
            store_root
                .join("projects")
                .join(&workspace.id)
                .join("injected-link"),
        )
        .unwrap();

        store.delete_workspace(&workspace.id, false, true).unwrap();

        assert_eq!(
            fs::read_to_string(&external_file).unwrap(),
            "must remain untouched"
        );
        assert_eq!(
            fs::metadata(&external_file).unwrap().permissions().mode() & 0o777,
            0o400
        );
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

    #[test]
    fn completes_and_recovers_the_local_submission_lifecycle() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("lifecycle.tex");
        fs::write(
            &source_path,
            r"\title{Lifecycle Study}
\author{Synthetic Author}
\begin{abstract}Traceable evidence.\end{abstract}
\keywords{workflow}
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

        store.analyze_structure(&workspace.id).unwrap();
        let report = store.evaluate_readiness(&workspace.id, &[]).unwrap();
        let checked = store.lifecycle(&workspace.id).unwrap();
        assert_eq!(
            checked
                .readiness_report
                .as_ref()
                .map(|item| &item.report_id),
            Some(&report.report_id)
        );
        assert!(matches!(
            store.create_local_attestation(&workspace.id, false),
            Err(WorkspaceError::AuthorConfirmationRequired)
        ));

        let attestation = store.create_local_attestation(&workspace.id, true).unwrap();
        let export_root = temporary.path().join("exports");
        fs::create_dir(&export_root).unwrap();
        let export = store
            .export_submission_package(&workspace.id, &export_root)
            .unwrap();
        let package_root = export_root.join(&export.package_name);
        assert!(package_root.join("manuscript.tex").is_file());
        assert!(package_root.join("readiness-report.json").is_file());
        assert!(package_root.join("readiness-preview.html").is_file());
        assert!(package_root.join("local-attestation.json").is_file());
        assert!(package_root.join("submission-manifest.json").is_file());

        let submission = store
            .record_manual_submission(
                &workspace.id,
                "Synthetic Journal",
                Some("SYN-2026-001"),
                true,
            )
            .unwrap();
        assert!(matches!(
            store.finalize_knowledge_body(&workspace.id, "unknown-discipline"),
            Err(WorkspaceError::InvalidDisciplineClassification)
        ));
        let knowledge = store
            .finalize_knowledge_body(&workspace.id, "computer_information_sciences")
            .unwrap();
        assert_eq!(knowledge.attestation_id, attestation.attestation_id);
        assert_eq!(knowledge.submission_id, submission.submission_id);
        assert_eq!(knowledge.snapshot.manuscript.version, 1);
        let classification = knowledge.discipline_classification.as_ref().unwrap();
        assert_eq!(classification.code, "computer_information_sciences");
        assert_eq!(classification.version, 1);
        assert_eq!(classification.status, "author_confirmed");
        assert_eq!(knowledge.record_hash.len(), 64);

        let mut legacy_knowledge = knowledge.clone();
        legacy_knowledge.discipline_classification = None;
        legacy_knowledge.record_hash =
            super::hash_serializable(&super::LegacyKnowledgeBodyPayload {
                record_id: &legacy_knowledge.record_id,
                workspace_id: &legacy_knowledge.workspace_id,
                manuscript_version: legacy_knowledge.manuscript_version,
                attestation_id: &legacy_knowledge.attestation_id,
                submission_id: &legacy_knowledge.submission_id,
                finalized_unix_ms: legacy_knowledge.finalized_unix_ms,
                snapshot: &legacy_knowledge.snapshot,
                external_transmission: &legacy_knowledge.external_transmission,
            })
            .unwrap();
        super::verify_knowledge_body_record(&legacy_knowledge).unwrap();

        assert!(matches!(
            store.create_owner_inquiry(
                &workspace.id,
                KnowledgeInquiryStance::Question,
                KnowledgeInquiryTarget::Claim,
                "当前 Claim 有哪些限制？",
                false,
            ),
            Err(WorkspaceError::AuthorConfirmationRequired)
        ));
        assert!(matches!(
            store.create_owner_inquiry(
                &workspace.id,
                KnowledgeInquiryStance::Question,
                KnowledgeInquiryTarget::Claim,
                "   ",
                true,
            ),
            Err(WorkspaceError::InvalidKnowledgeInquiry)
        ));
        let inquiry = store
            .create_owner_inquiry(
                &workspace.id,
                KnowledgeInquiryStance::Challenge,
                KnowledgeInquiryTarget::Claim,
                "当前证据是否足以支持这个 Claim？",
                true,
            )
            .unwrap();
        let answer = store
            .record_model_answer(
                &workspace.id,
                &inquiry.inquiry_id,
                "fallback_1",
                "Synthetic Provider",
                "synthetic-model",
                "现有知识体只建立了来源边界，尚未形成正式 EvidenceRelation。",
                &[],
            )
            .unwrap();
        let dialogue = store.knowledge_dialogue(&workspace.id).unwrap();
        assert_eq!(dialogue.items.len(), 1);
        assert_eq!(dialogue.items[0].inquiry, inquiry);
        assert_eq!(dialogue.items[0].answers, vec![answer.clone()]);
        assert_eq!(
            answer.external_transmission,
            "performed_to_configured_model"
        );
        let mut tampered_answer = answer;
        tampered_answer.answer = "tampered".to_owned();
        assert!(super::verify_knowledge_answer(&tampered_answer)
            .unwrap_err()
            .to_string()
            .contains("完整性验证失败"));

        let reclassified = store
            .finalize_knowledge_body(&workspace.id, "engineering_technology")
            .unwrap();
        let reclassified_assignment = reclassified.discipline_classification.as_ref().unwrap();
        assert_eq!(
            reclassified_assignment.assignment_id,
            classification.assignment_id
        );
        assert_eq!(reclassified_assignment.version, 2);
        assert_eq!(reclassified_assignment.code, "engineering_technology");
        assert_ne!(reclassified.record_hash, knowledge.record_hash);

        let recovered = store.lifecycle(&workspace.id).unwrap();
        assert_eq!(recovered.attestation, Some(attestation));
        assert_eq!(recovered.submission, Some(submission));
        assert_eq!(recovered.knowledge_body, Some(reclassified.clone()));
        assert!(store
            .knowledge_dialogue(&workspace.id)
            .unwrap()
            .items
            .is_empty());
        let mut tampered_knowledge = reclassified;
        tampered_knowledge.submission_id = "tampered".to_owned();
        assert!(super::verify_knowledge_body_record(&tampered_knowledge)
            .unwrap_err()
            .to_string()
            .contains("完整性验证失败"));

        fs::write(
            &source_path,
            r"\title{Lifecycle Study Revised}
\begin{abstract}New evidence.\end{abstract}
\keywords{workflow}
\section{Introduction}
\section{Methods}",
        )
        .unwrap();
        store
            .create_version_from_source(&workspace.id, &source_path, "new head")
            .unwrap();
        let new_head = store.lifecycle(&workspace.id).unwrap();
        assert_eq!(new_head.current_version, 2);
        assert!(new_head.structure_report.is_none());
        assert!(new_head.readiness_report.is_none());
        assert!(new_head.attestation.is_none());
        assert!(new_head.submission.is_none());
        assert!(new_head.knowledge_body.is_none());
    }
}
