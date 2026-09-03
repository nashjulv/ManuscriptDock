use crate::{
    dialogue::{
        KnowledgeAnswerRecord, KnowledgeDialogueItem, KnowledgeDialogueLedger,
        KnowledgeInquiryOrigin, KnowledgeInquiryRecord, KnowledgeInquiryStance,
        KnowledgeInquiryTarget, KNOWLEDGE_DIALOGUE_SCHEMA_VERSION,
    },
    inspect_manuscript,
    journal_directory::{
        JournalDirectoryCatalog, JournalDirectoryImportResult, JournalDirectoryProfile,
        JournalDirectoryStore, JournalDirectorySummary, JournalProfileDiscoveryRecord,
        JOURNAL_PROFILE_DISCOVERY_SCHEMA_VERSION,
    },
    journal_match::{
        deadline_days_remaining, recommend_journals_with_directory, ArticleTypePreference,
        InstitutionRuleEvidence, InstitutionRuleStatus, JournalMatchPreferences,
        JournalRecommendation, JournalRecommendationProfile, JournalRecommendationProfileInput,
        JournalRecommendationRun, JOURNAL_PROFILE_SCHEMA_VERSION,
    },
    journal_requirements::{
        extract_journal_requirements, JournalRequirementCategory, JournalRequirementObligation,
        JournalRequirementSnapshot, JournalRequirementSourceDocument, JournalRequirementSourceMode,
        JournalRequirementStatus, JOURNAL_REQUIREMENT_FRESHNESS_DAYS,
        JOURNAL_REQUIREMENT_SCHEMA_VERSION,
    },
    knowledge::{
        apply_candidate_decisions, discipline_catalog_item, local_knowledge_body_snapshot,
        AcademicKnowledgeBodySnapshot, DisciplineClassification, KnowledgeBodyError,
        KnowledgeCandidateDecision, DISCIPLINE_INDEX_SCHEME, DISCIPLINE_INDEX_VERSION,
    },
    readiness::{
        evaluate_readiness, render_readiness_html, ReadinessError, READINESS_REPORT_VERSION,
    },
    revision::{apply_revision, extract_revision_fields},
    structure::{
        extract_structure, DecompositionManifest, StructureError, DECOMPOSITION_SCHEMA_VERSION,
        STRUCTURE_ANALYSIS_VERSION,
    },
    ManuscriptKind, ManuscriptSummary, ReadinessOutcome, ReadinessReport, RevisionApplication,
    RevisionChangeInput, RevisionDraft, RevisionError, RevisionSet, StructureReport,
    MAX_MANUSCRIPT_SIZE_BYTES,
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
use zip::ZipArchive;

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
    #[serde(default = "default_submission_record_schema_version")]
    pub schema_version: u32,
    pub submission_id: String,
    pub workspace_id: String,
    pub manuscript_version: u32,
    pub attestation_id: String,
    #[serde(default)]
    pub target_selection_id: Option<String>,
    pub target: String,
    #[serde(default)]
    pub publisher: Option<String>,
    pub receipt: Option<String>,
    pub submitted_unix_ms: u64,
    pub statement: String,
    pub record_hash: String,
    pub external_transmission: String,
}

fn default_submission_record_schema_version() -> u32 {
    1
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionMaterialKind {
    SourceProject,
    BlindedManuscript,
    Figure,
    Table,
    Bibliography,
    Supplementary,
    CoverLetter,
    TitlePage,
    Declaration,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionMaterial {
    pub material_id: String,
    pub kind: SubmissionMaterialKind,
    pub original_name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub content_hash: String,
    pub imported_unix_ms: u64,
    #[serde(default)]
    pub manuscript_version: u32,
    #[serde(default)]
    pub target_selection_id: Option<String>,
    #[serde(default)]
    pub requirement_snapshot_id: Option<String>,
    #[serde(default)]
    pub checklist_item_id: Option<String>,
    #[serde(default = "default_material_included")]
    pub included: bool,
    #[serde(default)]
    pub validation_status: String,
    #[serde(default)]
    pub validation_issues: Vec<String>,
    #[serde(default)]
    pub detected_media_type: Option<String>,
}

fn default_material_included() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionMaterialChecklistItem {
    pub id: String,
    pub label: String,
    pub label_en: String,
    pub group: String,
    pub requirement: String,
    pub status: String,
    pub detail: String,
    pub verification: String,
    pub material_kind: Option<SubmissionMaterialKind>,
    pub blocking: bool,
    pub confirmable: bool,
    pub source_url: Option<String>,
    pub evidence_excerpt: Option<String>,
    pub captured_unix_ms: Option<u64>,
    pub fresh_until_unix_ms: Option<u64>,
    pub required_count: usize,
    pub matched_material_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionMaterialCatalog {
    pub schema_version: u32,
    pub workspace_id: String,
    pub manuscript_version: u32,
    pub materials: Vec<SubmissionMaterial>,
    pub checklist: Vec<SubmissionMaterialChecklistItem>,
    pub recommendation_ready: bool,
    pub target_verified: bool,
    pub required_complete: bool,
    pub target_check_ready: bool,
    pub workflow_status: String,
    pub required_total: usize,
    pub required_completed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionTargetSelection {
    pub schema_version: u32,
    pub selection_id: String,
    pub workspace_id: String,
    pub selected_against_manuscript_version: u32,
    pub recommendation_run_id: String,
    pub journal_id: String,
    pub name: String,
    pub name_en: String,
    pub publisher: String,
    pub region: String,
    pub rank_system: String,
    pub rank_tier: String,
    pub homepage_url: String,
    #[serde(default = "default_article_type_preference")]
    pub article_type: ArticleTypePreference,
    #[serde(default = "default_primary_target_role")]
    pub plan_role: String,
    #[serde(default)]
    pub priority: u32,
    pub selected_unix_ms: u64,
    pub record_hash: String,
    pub external_transmission: String,
}

fn default_primary_target_role() -> String {
    "primary".to_owned()
}

fn default_article_type_preference() -> ArticleTypePreference {
    ArticleTypePreference::Auto
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionTargetPlan {
    pub schema_version: u32,
    pub workspace_id: String,
    pub primary: Option<SubmissionTargetSelection>,
    pub backups: Vec<SubmissionTargetSelection>,
    pub updated_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionTargetTransition {
    pub schema_version: u32,
    pub transition_id: String,
    pub workspace_id: String,
    pub from_selection_id: Option<String>,
    pub to_selection_id: String,
    pub reason: String,
    pub transitioned_unix_ms: u64,
    pub record_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSubmissionExport {
    pub package_name: String,
    pub manuscript_version: u32,
    pub target_selection_id: String,
    pub target_name: String,
    pub files: Vec<String>,
    pub warnings: Vec<String>,
    pub exported_unix_ms: u64,
    pub external_transmission: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSubmissionPackageFile {
    pub material_id: Option<String>,
    pub display_name: String,
    pub relative_path: String,
    pub role: String,
    pub material_kind: Option<SubmissionMaterialKind>,
    pub checklist_item_id: Option<String>,
    pub checklist_label: Option<String>,
    pub required: bool,
    pub included: bool,
    pub size_bytes: u64,
    pub content_hash: String,
    pub validation_status: String,
    pub validation_issues: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSubmissionPackagePlan {
    pub schema_version: u32,
    pub workspace_id: String,
    pub manuscript_version: u32,
    pub target_selection_id: String,
    pub target_name: String,
    pub anonymous_review: bool,
    pub ready: bool,
    pub files: Vec<TargetSubmissionPackageFile>,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
    pub created_unix_ms: u64,
    pub external_transmission: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCopyExport {
    pub folder_name: String,
    pub workspace_id: String,
    pub manuscript_version: u32,
    pub file_count: u32,
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
    pub submission_materials: SubmissionMaterialCatalog,
    pub submission_target: Option<SubmissionTargetSelection>,
    pub submission_target_plan: SubmissionTargetPlan,
    pub journal_requirements: Option<JournalRequirementSnapshot>,
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
    JournalDirectory(String),
    MissingCurrentReadiness,
    AuthorConfirmationRequired,
    InvalidSubmissionTarget,
    SubmissionTargetNotFound,
    StaleRecommendationRun,
    InvalidSubmissionTargetPlan,
    SubmissionTargetLockedBySubmission,
    SubmissionBackupLimitReached,
    InvalidJournalRequirementSource,
    InvalidSubmissionMaterial(String),
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
                "请检查投稿背景字段长度，并选择有效的未来投稿截止日期"
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
            Self::JournalDirectory(message) => write!(formatter, "{message}"),
            Self::MissingCurrentReadiness => {
                write!(formatter, "当前论文版本尚未完成投稿检查，请先重新检查")
            }
            Self::AuthorConfirmationRequired => {
                write!(formatter, "需要作者明确确认后才能创建记录")
            }
            Self::InvalidSubmissionTarget => {
                write!(formatter, "投稿目标不能为空，且不能超过 200 个字符")
            }
            Self::SubmissionTargetNotFound => {
                write!(formatter, "请先从推荐结果中选择当前投稿目标")
            }
            Self::StaleRecommendationRun => {
                write!(
                    formatter,
                    "该推荐属于较早的稿件版本，请按当前版本重新计算后再选择"
                )
            }
            Self::InvalidSubmissionTargetPlan => {
                write!(formatter, "投稿主线或备选支线记录无效")
            }
            Self::SubmissionTargetLockedBySubmission => write!(
                formatter,
                "当前主线已经登记投稿，不能直接取消；请通过退稿、撤稿或未投稿原因切换后续路线"
            ),
            Self::SubmissionBackupLimitReached => {
                write!(formatter, "最多可保留 8 个备选投稿支线")
            }
            Self::InvalidJournalRequirementSource => write!(
                formatter,
                "期刊要求必须来自有效的 HTTPS 官方来源，并包含可核对的原文"
            ),
            Self::InvalidSubmissionMaterial(message) => {
                write!(formatter, "投稿材料无效：{message}")
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredSubmissionMaterial {
    #[serde(flatten)]
    material: SubmissionMaterial,
    relative_path: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredSubmissionMaterialCatalog {
    schema_version: u32,
    #[serde(default)]
    materials: Vec<StoredSubmissionMaterial>,
    #[serde(default)]
    confirmations: Vec<StoredSubmissionRequirementConfirmation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredSubmissionRequirementConfirmation {
    item_id: String,
    target_selection_id: String,
    requirement_snapshot_id: String,
    confirmed_unix_ms: u64,
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
struct LegacySubmissionPayload<'a> {
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
struct SubmissionPayload<'a> {
    schema_version: u32,
    submission_id: &'a str,
    workspace_id: &'a str,
    manuscript_version: u32,
    attestation_id: &'a str,
    target_selection_id: &'a Option<String>,
    target: &'a str,
    publisher: &'a Option<String>,
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
    decomposition_id: &'a str,
    decomposition_hash: &'a str,
    readiness_report_id: &'a str,
    attestation_id: &'a str,
    attestation_hash: &'a str,
    created_unix_ms: u64,
    files: &'a [String],
    external_transmission: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmissionTargetPayload<'a> {
    schema_version: u32,
    selection_id: &'a str,
    workspace_id: &'a str,
    selected_against_manuscript_version: u32,
    recommendation_run_id: &'a str,
    journal_id: &'a str,
    name: &'a str,
    name_en: &'a str,
    publisher: &'a str,
    region: &'a str,
    rank_system: &'a str,
    rank_tier: &'a str,
    homepage_url: &'a str,
    article_type: ArticleTypePreference,
    plan_role: &'a str,
    priority: u32,
    selected_unix_ms: u64,
    external_transmission: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JournalRequirementSnapshotPayload<'a> {
    schema_version: u32,
    snapshot_id: &'a str,
    workspace_id: &'a str,
    target_selection_id: &'a str,
    journal_id: &'a str,
    journal_name: &'a str,
    source_mode: JournalRequirementSourceMode,
    status: JournalRequirementStatus,
    sources: &'a [crate::JournalRequirementSource],
    requirements: &'a [crate::JournalRequirementItem],
    limitations: &'a [String],
    captured_unix_ms: u64,
    fresh_until_unix_ms: u64,
    external_transmission: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmissionTargetTransitionPayload<'a> {
    schema_version: u32,
    transition_id: &'a str,
    workspace_id: &'a str,
    from_selection_id: &'a Option<String>,
    to_selection_id: &'a str,
    reason: &'a str,
    transitioned_unix_ms: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetSubmissionPackageManifest<'a> {
    schema_version: u32,
    workspace_id: &'a str,
    manuscript_version: u32,
    manuscript_hash: &'a str,
    target_selection: &'a SubmissionTargetSelection,
    journal_requirement_snapshot: Option<&'a JournalRequirementSnapshot>,
    submission_files: &'a [TargetSubmissionPackageFile],
    warnings: &'a [String],
    created_unix_ms: u64,
    external_transmission: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DecompositionPayload<'a> {
    schema_version: u32,
    decomposition_id: &'a str,
    workspace_id: &'a str,
    source_content_hash: &'a str,
    source_snapshot_version: u32,
    created_unix_ms: u64,
    structure: &'a StructureReport,
    declared_outputs: &'a [String],
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

    pub fn export_workspace_copy(
        &self,
        workspace_id: &str,
        archived: bool,
        destination: &Path,
    ) -> Result<WorkspaceCopyExport, WorkspaceError> {
        if !destination.is_dir() {
            return Err(WorkspaceError::InvalidExportDestination);
        }
        let (workspace_root, manifest) = self.workspace_for_management(workspace_id, archived)?;
        let destination = fs::canonicalize(destination)?;
        let workspace_root = fs::canonicalize(workspace_root)?;
        if destination.starts_with(&workspace_root) {
            return Err(WorkspaceError::InvalidExportDestination);
        }
        let manuscript_stem = Path::new(&manifest.workspace.manuscript.name)
            .file_stem()
            .and_then(|value| value.to_str())
            .map(safe_export_component)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "manuscript".to_owned());
        let folder_name = format!(
            "ManuscriptDock-{manuscript_stem}-v{}-{}",
            manifest.workspace.snapshot_version,
            &workspace_id[..8]
        );
        let final_root = destination.join(&folder_name);
        if final_root.exists() {
            return Err(WorkspaceError::ExportDestinationExists);
        }
        let temporary_root = destination.join(format!(".manuscriptdock-{}.tmp", Uuid::new_v4()));
        let exported_unix_ms = unix_time_ms()?;
        let result = (|| {
            let mut file_count = 0;
            copy_workspace_tree(&workspace_root, &temporary_root, &mut file_count)?;
            fs::rename(&temporary_root, &final_root)?;
            append_audit_event(
                &workspace_root.join("audit.jsonl"),
                "workspace_copy_exported",
                &manifest.workspace,
                exported_unix_ms,
            )?;
            Ok(WorkspaceCopyExport {
                folder_name,
                workspace_id: workspace_id.to_owned(),
                manuscript_version: manifest.workspace.snapshot_version,
                file_count,
                exported_unix_ms,
                external_transmission: "not_performed".to_owned(),
            })
        })();
        if temporary_root.exists() {
            let _ = remove_generated_directory(&temporary_root);
        }
        result
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
        let decomposition = match read_current_decomposition(&workspace_root, &manifest.workspace)?
        {
            Some(decomposition) => Some(decomposition),
            None => {
                self.analyze_structure(workspace_id)?;
                read_current_decomposition(&workspace_root, &manifest.workspace)?
            }
        };
        Ok(local_knowledge_body_snapshot(
            &manifest.workspace,
            decomposition.as_ref(),
        ))
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

        let created_unix_ms = unix_time_ms()?;
        let decomposition_id = format!(
            "decomposition:{}:v{}",
            workspace_id, manifest.workspace.snapshot_version
        );
        let declared_outputs = vec![
            "knowledge_body_candidates".to_owned(),
            "submission_readiness_inputs".to_owned(),
            "submission_package_manifest".to_owned(),
        ];
        let external_transmission = "not_performed".to_owned();
        let payload = DecompositionPayload {
            schema_version: DECOMPOSITION_SCHEMA_VERSION,
            decomposition_id: &decomposition_id,
            workspace_id,
            source_content_hash: &manifest.workspace.content_hash,
            source_snapshot_version: manifest.workspace.snapshot_version,
            created_unix_ms,
            structure: &report,
            declared_outputs: &declared_outputs,
            external_transmission: &external_transmission,
        };
        let manifest_hash = hash_serializable(&payload)?;
        let decomposition = DecompositionManifest {
            schema_version: DECOMPOSITION_SCHEMA_VERSION,
            decomposition_id,
            workspace_id: workspace_id.to_owned(),
            source_content_hash: manifest.workspace.content_hash.clone(),
            source_snapshot_version: manifest.workspace.snapshot_version,
            created_unix_ms,
            structure: report.clone(),
            declared_outputs,
            manifest_hash,
            external_transmission,
        };

        let analysis_root = workspace_root.join("analysis");
        fs::create_dir_all(&analysis_root)?;
        let hash_prefix = manifest
            .workspace
            .content_hash
            .get(..12)
            .ok_or_else(|| WorkspaceError::InvalidManifest("内容指纹长度无效".to_owned()))?;
        let report_path = analysis_root.join(format!(
            "decomposition-v{DECOMPOSITION_SCHEMA_VERSION}-a{STRUCTURE_ANALYSIS_VERSION}-{hash_prefix}.json"
        ));
        if !report_path.exists() {
            let temporary_path = analysis_root.join(format!(".{}.tmp", Uuid::new_v4()));
            write_json(&temporary_path, &decomposition)?;
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
            "manuscript_decomposed",
            &manifest.workspace,
            created_unix_ms,
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
        let directory_store = self.journal_directory_store();
        let directory = directory_store
            .load()
            .map_err(|error| WorkspaceError::JournalDirectory(error.to_string()))?;
        let directory = directory.summary().available.then_some(directory);
        let run = recommend_journals_with_directory(
            &report,
            profile,
            preferences,
            evaluated_unix_ms,
            directory.as_ref(),
        );
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

    pub fn journal_recommendation_runs(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<JournalRecommendationRun>, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let analysis_root = workspace_root.join("analysis");
        if !analysis_root.exists() {
            return Ok(Vec::new());
        }

        let mut runs = Vec::new();
        for entry in fs::read_dir(analysis_root)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if !file_name.starts_with("journal-match-jmr-") || !file_name.ends_with(".json") {
                continue;
            }
            let run = read_journal_recommendation_run(&entry.path())?;
            if run.workspace_id != workspace_id
                || file_name != format!("journal-match-{}.json", run.run_id)
            {
                return Err(WorkspaceError::InvalidManifest(
                    "期刊推荐记录与当前论文工作区不一致".to_owned(),
                ));
            }
            runs.push(run);
        }
        runs.sort_by(|left, right| {
            right
                .evaluated_unix_ms
                .cmp(&left.evaluated_unix_ms)
                .then_with(|| right.manuscript_version.cmp(&left.manuscript_version))
                .then_with(|| left.run_id.cmp(&right.run_id))
        });
        Ok(runs)
    }

    pub fn import_journal_directory(
        &self,
        paths: &[PathBuf],
    ) -> Result<JournalDirectoryImportResult, WorkspaceError> {
        self.journal_directory_store()
            .import_workbooks(paths)
            .map_err(|error| WorkspaceError::JournalDirectory(error.to_string()))
    }

    pub fn journal_directory_summary(&self) -> Result<JournalDirectorySummary, WorkspaceError> {
        self.journal_directory_store()
            .summary()
            .map_err(|error| WorkspaceError::JournalDirectory(error.to_string()))
    }

    pub fn journal_directory_catalog(&self) -> Result<JournalDirectoryCatalog, WorkspaceError> {
        self.journal_directory_store()
            .load()
            .map_err(|error| WorkspaceError::JournalDirectory(error.to_string()))
    }

    pub fn journal_directory_profile(
        &self,
        title: &str,
        issn: Option<&str>,
        eissn: Option<&str>,
    ) -> Result<Option<JournalDirectoryProfile>, WorkspaceError> {
        self.journal_directory_store()
            .profile_for_identity(title, issn, eissn)
            .map_err(|error| WorkspaceError::JournalDirectory(error.to_string()))
    }

    pub fn save_journal_profile_discovery(
        &self,
        workspace_id: &str,
        record: &JournalProfileDiscoveryRecord,
    ) -> Result<(), WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if record.schema_version != JOURNAL_PROFILE_DISCOVERY_SCHEMA_VERSION
            || record.workspace_id != workspace_id
            || !record.discovery_id.starts_with("jed-")
            || record.discovery_id.len() != 24
            || !record.discovery_id[4..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(WorkspaceError::InvalidManifest(
                "期刊外部发现记录身份无效".to_owned(),
            ));
        }
        let plan = self.submission_target_plan(workspace_id)?;
        let selected = plan
            .primary
            .iter()
            .chain(plan.backups.iter())
            .find(|target| target.selection_id == record.target_selection_id)
            .ok_or(WorkspaceError::SubmissionTargetNotFound)?;
        if selected.journal_id != record.journal_id || selected.name != record.journal_name {
            return Err(WorkspaceError::SubmissionTargetNotFound);
        }
        let valid_provenance = matches!(
            (
                record.source_mode.as_str(),
                record.evidence_status.as_str(),
                record.external_transmission.as_str()
            ),
            (
                "local_directory",
                "local_profile_available",
                "not_performed"
            ) | (
                "configured_model_candidate",
                "candidate_requires_official_verification",
                "author_confirmed_public_journal_identity_only"
            )
        );
        if !valid_provenance {
            return Err(WorkspaceError::InvalidManifest(
                "期刊外部发现记录的证据状态无效".to_owned(),
            ));
        }
        let analysis_root = workspace_root.join("analysis");
        fs::create_dir_all(&analysis_root)?;
        let path = analysis_root.join(format!(
            "journal-profile-discovery-{}.json",
            record.discovery_id
        ));
        if !path.exists() {
            write_json(&path, record)?;
        }
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            if record.external_transmission == "not_performed" {
                "journal_profile_resolved_locally"
            } else {
                "journal_profile_candidate_discovered_by_model"
            },
            &manifest.workspace,
            record.created_unix_ms,
        )?;
        Ok(())
    }

    pub fn journal_profile_discoveries(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<JournalProfileDiscoveryRecord>, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let analysis_root = workspace_root.join("analysis");
        if !analysis_root.exists() {
            return Ok(Vec::new());
        }
        let mut records = Vec::new();
        for entry in fs::read_dir(analysis_root)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if !file_name.starts_with("journal-profile-discovery-jed-")
                || !file_name.ends_with(".json")
            {
                continue;
            }
            let record: JournalProfileDiscoveryRecord = read_json(&entry.path())?;
            if record.workspace_id != workspace_id
                || file_name != format!("journal-profile-discovery-{}.json", record.discovery_id)
            {
                return Err(WorkspaceError::InvalidManifest(
                    "期刊外部发现记录与当前论文工作区不一致".to_owned(),
                ));
            }
            records.push(record);
        }
        records.sort_by(|left, right| {
            right
                .created_unix_ms
                .cmp(&left.created_unix_ms)
                .then_with(|| left.discovery_id.cmp(&right.discovery_id))
        });
        Ok(records)
    }

    fn journal_directory_store(&self) -> JournalDirectoryStore {
        JournalDirectoryStore::new(self.root.join("journal-directory"))
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

    pub fn journal_recommendation_author_names(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<String>, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let analysis_root = workspace_root.join("analysis");
        if !analysis_root.exists() {
            return Ok(Vec::new());
        }
        let mut names = BTreeSet::new();
        for entry in fs::read_dir(analysis_root)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if !file_name.starts_with("journal-profile-jmp-") || !file_name.ends_with(".json") {
                continue;
            }
            let profile: JournalRecommendationProfile = read_json(&entry.path())?;
            if profile.workspace_id != workspace_id {
                return Err(WorkspaceError::InvalidWorkspaceId);
            }
            if !profile.author_name.trim().is_empty() {
                names.insert(profile.author_name);
            }
        }
        Ok(names.into_iter().collect())
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
        let structure = match read_current_decomposition(&workspace_root, &manifest.workspace)? {
            Some(decomposition) => decomposition.structure,
            None => self.analyze_structure(workspace_id)?,
        };
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

    pub fn submission_materials(
        &self,
        workspace_id: &str,
    ) -> Result<SubmissionMaterialCatalog, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let stored = read_stored_submission_materials(&workspace_root)?;
        let structure = read_current_structure_report(&workspace_root, &manifest.workspace)?;
        let target = read_submission_target(&workspace_root)?;
        let readiness = read_current_readiness_report(&workspace_root, &manifest.workspace)?
            .filter(|report| {
                readiness_matches_target(report, target.as_ref(), &manifest.workspace)
            });
        let journal_requirements = target
            .as_ref()
            .map(|selection| {
                read_journal_requirement_snapshot(&workspace_root, &selection.selection_id)
            })
            .transpose()?
            .flatten();
        let recommendation_ready = self
            .journal_recommendation_runs(workspace_id)?
            .iter()
            .any(|run| run.manuscript_version == manifest.workspace.snapshot_version);
        Ok(build_submission_material_catalog(
            &manifest.workspace,
            stored,
            structure.as_ref(),
            readiness.as_ref(),
            target.as_ref(),
            journal_requirements.as_ref(),
            recommendation_ready,
            unix_time_ms()?,
        ))
    }

    pub fn target_submission_package_plan(
        &self,
        workspace_id: &str,
    ) -> Result<TargetSubmissionPackagePlan, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        let target = read_submission_target(&workspace_root)?
            .ok_or(WorkspaceError::SubmissionTargetNotFound)?;
        let requirement_snapshot =
            read_journal_requirement_snapshot(&workspace_root, &target.selection_id)?;
        let catalog = self.submission_materials(workspace_id)?;
        let stored = read_stored_submission_materials(&workspace_root)?;
        let now = unix_time_ms()?;
        let anonymous_review = requirement_snapshot.as_ref().is_some_and(|snapshot| {
            snapshot.requirements.iter().any(|requirement| {
                requirement.category == JournalRequirementCategory::AnonymousReview
                    && requirement.obligation == JournalRequirementObligation::Required
            })
        });
        let mut blockers = catalog
            .checklist
            .iter()
            .filter(|item| item.blocking && item.status != "passed")
            .map(|item| format!("{}：{}", item.label, item.detail))
            .collect::<Vec<_>>();
        if target.selected_against_manuscript_version != manifest.workspace.snapshot_version {
            blockers.push("目标期刊不是基于当前稿件版本选择".to_owned());
        }
        if !requirement_snapshot.as_ref().is_some_and(|snapshot| {
            snapshot.status != JournalRequirementStatus::RequiresManualReview
                && !snapshot.requirements.is_empty()
                && snapshot.fresh_until_unix_ms >= now
        }) {
            blockers.push("期刊官方投稿要求缺失、待人工核验或已过期".to_owned());
        }
        let mut warnings = catalog
            .checklist
            .iter()
            .filter(|item| !item.blocking && item.status != "passed")
            .map(|item| format!("{}：{}", item.label, item.detail))
            .collect::<Vec<_>>();
        let snapshot_id = requirement_snapshot
            .as_ref()
            .map(|snapshot| snapshot.snapshot_id.as_str());
        let current_materials = stored
            .materials
            .iter()
            .filter(|item| {
                item.material.manuscript_version == manifest.workspace.snapshot_version
                    && item.material.target_selection_id.as_deref()
                        == Some(target.selection_id.as_str())
                    && item.material.requirement_snapshot_id.as_deref() == snapshot_id
            })
            .collect::<Vec<_>>();
        let omitted_count = stored
            .materials
            .len()
            .saturating_sub(current_materials.len());
        if omitted_count > 0 {
            warnings.push(format!(
                "已自动排除 {omitted_count} 个属于旧版本、旧目标或旧要求快照的附件"
            ));
        }
        let mut files = Vec::new();
        if !anonymous_review {
            files.push(TargetSubmissionPackageFile {
                material_id: None,
                display_name: manifest.workspace.manuscript.name.clone(),
                relative_path: format!(
                    "submission/manuscript.{}",
                    manifest.workspace.manuscript.extension
                ),
                role: "main_manuscript".to_owned(),
                material_kind: None,
                checklist_item_id: Some("main-manuscript".to_owned()),
                checklist_label: Some("当前主稿".to_owned()),
                required: true,
                included: true,
                size_bytes: manifest.workspace.manuscript.size_bytes,
                content_hash: manifest.workspace.content_hash.clone(),
                validation_status: "passed".to_owned(),
                validation_issues: Vec::new(),
            });
        } else {
            warnings.push(
                "匿名评审已启用：实名源稿不会写入 submission，独立匿名稿将作为主稿".to_owned(),
            );
        }
        let checklist_by_id = catalog
            .checklist
            .iter()
            .map(|item| (item.id.as_str(), item))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut used_names = BTreeSet::new();
        for stored_material in current_materials {
            let material = &stored_material.material;
            let checklist = material
                .checklist_item_id
                .as_deref()
                .and_then(|id| checklist_by_id.get(id).copied());
            let category = material_kind_folder(material.kind);
            let mut exported_name = safe_export_file_name(&material.original_name);
            if exported_name.is_empty() {
                exported_name = format!("{}.{}", &material.material_id[..8], material.extension);
            }
            let relative_path = if anonymous_review
                && material.kind == SubmissionMaterialKind::BlindedManuscript
                && !files.iter().any(|file| file.role == "blinded_manuscript")
            {
                format!("submission/manuscript-blinded.{}", material.extension)
            } else {
                let mut unique_key = format!("{category}/{exported_name}");
                if !used_names.insert(unique_key.clone()) {
                    exported_name = format!("{}-{exported_name}", &material.material_id[..8]);
                    unique_key = format!("{category}/{exported_name}");
                    used_names.insert(unique_key);
                }
                format!("submission/{category}/{exported_name}")
            };
            for issue in &material.validation_issues {
                warnings.push(format!("{}：{}", material.original_name, issue));
            }
            files.push(TargetSubmissionPackageFile {
                material_id: Some(material.material_id.clone()),
                display_name: material.original_name.clone(),
                relative_path,
                role: material_kind_role(material.kind).to_owned(),
                material_kind: Some(material.kind),
                checklist_item_id: material.checklist_item_id.clone(),
                checklist_label: checklist.map(|item| item.label.clone()),
                required: checklist.is_some_and(|item| item.blocking),
                included: material.included,
                size_bytes: material.size_bytes,
                content_hash: material.content_hash.clone(),
                validation_status: if material.validation_status.is_empty() {
                    "legacy_unverified".to_owned()
                } else {
                    material.validation_status.clone()
                },
                validation_issues: material.validation_issues.clone(),
            });
        }
        if anonymous_review
            && !files.iter().any(|file| {
                file.role == "blinded_manuscript"
                    && file.included
                    && file.validation_status != "blocked"
            })
        {
            blockers.push("匿名评审要求独立匿名主稿，但当前没有可导出的匿名稿".to_owned());
        }
        blockers.sort();
        blockers.dedup();
        warnings.sort();
        warnings.dedup();
        Ok(TargetSubmissionPackagePlan {
            schema_version: 1,
            workspace_id: workspace_id.to_owned(),
            manuscript_version: manifest.workspace.snapshot_version,
            target_selection_id: target.selection_id,
            target_name: target.name,
            anonymous_review,
            ready: blockers.is_empty() && catalog.target_check_ready,
            files,
            warnings,
            blockers,
            created_unix_ms: now,
            external_transmission: "not_performed".to_owned(),
        })
    }

    pub fn add_submission_materials(
        &self,
        workspace_id: &str,
        kind: SubmissionMaterialKind,
        paths: &[PathBuf],
    ) -> Result<SubmissionMaterialCatalog, WorkspaceError> {
        self.add_submission_materials_for_requirement(workspace_id, kind, None, paths)
    }

    pub fn add_submission_materials_for_requirement(
        &self,
        workspace_id: &str,
        kind: SubmissionMaterialKind,
        checklist_item_id: Option<&str>,
        paths: &[PathBuf],
    ) -> Result<SubmissionMaterialCatalog, WorkspaceError> {
        if paths.is_empty() {
            return self.submission_materials(workspace_id);
        }
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let target = read_submission_target(&workspace_root)?
            .ok_or(WorkspaceError::SubmissionTargetNotFound)?;
        if target.selected_against_manuscript_version != manifest.workspace.snapshot_version {
            return Err(WorkspaceError::InvalidSubmissionMaterial(
                "投稿材料必须绑定当前稿件版本和当前目标期刊".to_owned(),
            ));
        }
        let requirement_snapshot =
            read_journal_requirement_snapshot(&workspace_root, &target.selection_id)?
                .ok_or(WorkspaceError::InvalidJournalRequirementSource)?;
        let current_catalog = self.submission_materials(workspace_id)?;
        let eligible_items = current_catalog
            .checklist
            .iter()
            .filter(|item| item.verification == "file" && item.material_kind == Some(kind))
            .collect::<Vec<_>>();
        let resolved_item_id = match checklist_item_id {
            Some(item_id) => {
                let item = eligible_items
                    .iter()
                    .find(|item| item.id == item_id)
                    .ok_or_else(|| {
                        WorkspaceError::InvalidSubmissionMaterial(
                            "所选文件类型与投稿要求不匹配".to_owned(),
                        )
                    })?;
                item.id.clone()
            }
            None if eligible_items.len() == 1 => eligible_items[0].id.clone(),
            None if eligible_items.is_empty() => {
                return Err(WorkspaceError::InvalidSubmissionMaterial(
                    "当前目标没有对应的文件要求".to_owned(),
                ))
            }
            None => {
                return Err(WorkspaceError::InvalidSubmissionMaterial(
                    "存在多个同类要求，请从具体清单项添加文件".to_owned(),
                ))
            }
        };
        let mut stored = read_stored_submission_materials(&workspace_root)?;
        let files_root = workspace_root.join("materials").join("files");
        fs::create_dir_all(&files_root)?;
        let imported_unix_ms = unix_time_ms()?;
        let mut added = 0_u32;
        for path in paths {
            let metadata = fs::metadata(path).map_err(|_| {
                WorkspaceError::InvalidSubmissionMaterial("无法读取所选文件".to_owned())
            })?;
            if !metadata.is_file()
                || metadata.len() == 0
                || metadata.len() > MAX_MANUSCRIPT_SIZE_BYTES
            {
                return Err(WorkspaceError::InvalidSubmissionMaterial(
                    "文件必须为 250 MB 以内的非空普通文件".to_owned(),
                ));
            }
            let original_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| WorkspaceError::InvalidSubmissionMaterial("文件名无效".to_owned()))?
                .to_owned();
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.to_ascii_lowercase())
                .ok_or_else(|| {
                    WorkspaceError::InvalidSubmissionMaterial("文件缺少扩展名".to_owned())
                })?;
            if !is_allowed_submission_material_extension(&extension) {
                return Err(WorkspaceError::InvalidSubmissionMaterial(format!(
                    "不支持 .{extension} 文件"
                )));
            }
            if !is_allowed_submission_material_kind_extension(kind, &extension) {
                return Err(WorkspaceError::InvalidSubmissionMaterial(format!(
                    "{}：{}",
                    original_name,
                    submission_material_kind_extension_help(kind)
                )));
            }
            let material_id = Uuid::new_v4().to_string();
            let relative_path = format!("materials/files/{material_id}.{extension}");
            let destination = workspace_root.join(&relative_path);
            let (content_hash, copied_size) = copy_and_hash(path, &destination)?;
            if copied_size != metadata.len() {
                let _ = fs::remove_file(&destination);
                return Err(WorkspaceError::SourceChangedDuringImport);
            }
            let validation = validate_submission_material(&destination, &extension, kind)?;
            if validation.status == "blocked" {
                let _ = fs::remove_file(&destination);
                return Err(WorkspaceError::InvalidSubmissionMaterial(format!(
                    "{}：{}",
                    original_name,
                    validation.issues.join("；")
                )));
            }
            if stored.materials.iter().any(|item| {
                item.material.content_hash == content_hash
                    && item.material.kind == kind
                    && item.material.manuscript_version == manifest.workspace.snapshot_version
                    && item.material.target_selection_id.as_deref()
                        == Some(target.selection_id.as_str())
                    && item.material.requirement_snapshot_id.as_deref()
                        == Some(requirement_snapshot.snapshot_id.as_str())
                    && item.material.checklist_item_id.as_deref() == Some(resolved_item_id.as_str())
            }) {
                let _ = fs::remove_file(&destination);
                continue;
            }
            set_readonly(&destination)?;
            stored.materials.push(StoredSubmissionMaterial {
                material: SubmissionMaterial {
                    material_id,
                    kind,
                    original_name,
                    extension,
                    size_bytes: copied_size,
                    content_hash,
                    imported_unix_ms,
                    manuscript_version: manifest.workspace.snapshot_version,
                    target_selection_id: Some(target.selection_id.clone()),
                    requirement_snapshot_id: Some(requirement_snapshot.snapshot_id.clone()),
                    checklist_item_id: Some(resolved_item_id.clone()),
                    included: true,
                    validation_status: validation.status,
                    validation_issues: validation.issues,
                    detected_media_type: validation.detected_media_type,
                },
                relative_path,
            });
            added += 1;
        }
        stored.schema_version = 3;
        let catalog_path = workspace_root.join("materials").join("catalog.json");
        write_or_replace_json(&catalog_path, &stored)?;
        if added > 0 {
            append_audit_event(
                &workspace_root.join("audit.jsonl"),
                "submission_materials_added",
                &manifest.workspace,
                imported_unix_ms,
            )?;
        }
        self.submission_materials(workspace_id)
    }

    pub fn set_submission_material_included(
        &self,
        workspace_id: &str,
        material_id: &str,
        included: bool,
    ) -> Result<SubmissionMaterialCatalog, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        Uuid::parse_str(material_id).map_err(|_| {
            WorkspaceError::InvalidSubmissionMaterial("无效的投稿材料标识".to_owned())
        })?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        let target = read_submission_target(&workspace_root)?
            .ok_or(WorkspaceError::SubmissionTargetNotFound)?;
        let mut stored = read_stored_submission_materials(&workspace_root)?;
        let material = stored
            .materials
            .iter_mut()
            .find(|item| item.material.material_id == material_id)
            .ok_or_else(|| {
                WorkspaceError::InvalidSubmissionMaterial("未找到该投稿材料".to_owned())
            })?;
        if material.material.manuscript_version != manifest.workspace.snapshot_version
            || material.material.target_selection_id.as_deref()
                != Some(target.selection_id.as_str())
        {
            return Err(WorkspaceError::InvalidSubmissionMaterial(
                "只能调整当前稿件版本与当前目标期刊的材料".to_owned(),
            ));
        }
        material.material.included = included;
        stored.schema_version = 3;
        write_or_replace_json(
            &workspace_root.join("materials").join("catalog.json"),
            &stored,
        )?;
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            if included {
                "submission_material_included"
            } else {
                "submission_material_excluded"
            },
            &manifest.workspace,
            unix_time_ms()?,
        )?;
        self.submission_materials(workspace_id)
    }

    pub fn delete_submission_material(
        &self,
        workspace_id: &str,
        material_id: &str,
        author_confirmed: bool,
    ) -> Result<SubmissionMaterialCatalog, WorkspaceError> {
        if !author_confirmed {
            return Err(WorkspaceError::AuthorConfirmationRequired);
        }
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        Uuid::parse_str(material_id).map_err(|_| {
            WorkspaceError::InvalidSubmissionMaterial("无效的投稿材料标识".to_owned())
        })?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let mut stored = read_stored_submission_materials(&workspace_root)?;
        let index = stored
            .materials
            .iter()
            .position(|item| item.material.material_id == material_id)
            .ok_or_else(|| {
                WorkspaceError::InvalidSubmissionMaterial("未找到该投稿材料".to_owned())
            })?;
        let material_path =
            resolve_submission_material_path(&workspace_root, &stored.materials[index])?;
        let deleting_path = workspace_root
            .join("materials")
            .join("files")
            .join(format!(".{material_id}.deleting"));
        if deleting_path.exists() {
            return Err(WorkspaceError::InvalidSubmissionMaterial(
                "附件删除暂存位置已存在，请重新打开工作区后再试".to_owned(),
            ));
        }

        fs::rename(&material_path, &deleting_path)?;
        stored.materials.remove(index);
        stored.schema_version = 3;
        let catalog_path = workspace_root.join("materials").join("catalog.json");
        if let Err(error) = write_or_replace_json(&catalog_path, &stored) {
            let _ = fs::rename(&deleting_path, &material_path);
            return Err(error);
        }
        let metadata = fs::symlink_metadata(&deleting_path)?;
        if !metadata.file_type().is_symlink() && metadata.permissions().readonly() {
            let mut permissions = metadata.permissions();
            make_file_owner_writable(&mut permissions);
            fs::set_permissions(&deleting_path, permissions)?;
        }
        fs::remove_file(&deleting_path)?;
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "submission_material_deleted",
            &manifest.workspace,
            unix_time_ms()?,
        )?;
        self.submission_materials(workspace_id)
    }

    pub fn confirm_submission_requirement(
        &self,
        workspace_id: &str,
        item_id: &str,
        confirmed: bool,
    ) -> Result<SubmissionMaterialCatalog, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        if item_id.trim().is_empty() || item_id.chars().count() > 160 {
            return Err(WorkspaceError::InvalidSubmissionMaterial(
                "无效的投稿要求确认项".to_owned(),
            ));
        }
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        let target = read_submission_target(&workspace_root)?
            .ok_or(WorkspaceError::SubmissionTargetNotFound)?;
        let snapshot = read_journal_requirement_snapshot(&workspace_root, &target.selection_id)?
            .ok_or(WorkspaceError::InvalidJournalRequirementSource)?;
        let mut stored = read_stored_submission_materials(&workspace_root)?;
        let recommendation_ready = self
            .journal_recommendation_runs(workspace_id)?
            .iter()
            .any(|run| run.manuscript_version == manifest.workspace.snapshot_version);
        let structure = read_current_structure_report(&workspace_root, &manifest.workspace)?;
        let readiness = read_current_readiness_report(&workspace_root, &manifest.workspace)?
            .filter(|report| readiness_matches_target(report, Some(&target), &manifest.workspace));
        let catalog = build_submission_material_catalog(
            &manifest.workspace,
            stored.clone(),
            structure.as_ref(),
            readiness.as_ref(),
            Some(&target),
            Some(&snapshot),
            recommendation_ready,
            unix_time_ms()?,
        );
        if !catalog
            .checklist
            .iter()
            .any(|item| item.id == item_id && item.confirmable)
        {
            return Err(WorkspaceError::InvalidSubmissionMaterial(
                "该投稿要求不能由作者确认完成".to_owned(),
            ));
        }
        stored.confirmations.retain(|item| {
            !(item.item_id == item_id
                && item.target_selection_id == target.selection_id
                && item.requirement_snapshot_id == snapshot.snapshot_id)
        });
        let confirmed_unix_ms = unix_time_ms()?;
        if confirmed {
            stored
                .confirmations
                .push(StoredSubmissionRequirementConfirmation {
                    item_id: item_id.to_owned(),
                    target_selection_id: target.selection_id,
                    requirement_snapshot_id: snapshot.snapshot_id,
                    confirmed_unix_ms,
                });
        }
        stored.schema_version = 2;
        write_or_replace_json(
            &workspace_root.join("materials").join("catalog.json"),
            &stored,
        )?;
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            if confirmed {
                "submission_requirement_confirmed"
            } else {
                "submission_requirement_confirmation_revoked"
            },
            &manifest.workspace,
            confirmed_unix_ms,
        )?;
        self.submission_materials(workspace_id)
    }

    pub fn select_recommended_journal(
        &self,
        workspace_id: &str,
        recommendation_run_id: &str,
        journal_id: &str,
    ) -> Result<SubmissionTargetSelection, WorkspaceError> {
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        let run = self
            .journal_recommendation_runs(workspace_id)?
            .into_iter()
            .find(|run| run.run_id == recommendation_run_id)
            .ok_or(WorkspaceError::SubmissionTargetNotFound)?;
        if run.manuscript_version != manifest.workspace.snapshot_version {
            return Err(WorkspaceError::StaleRecommendationRun);
        }
        let journal =
            journal_in_run(&run, journal_id).ok_or(WorkspaceError::SubmissionTargetNotFound)?;
        let selected_unix_ms = target_change_unix_ms(&workspace_root, &manifest.workspace)?;
        let selection = build_target_selection(
            workspace_id,
            manifest.workspace.snapshot_version,
            recommendation_run_id,
            journal,
            run.resolved_article_type,
            "primary",
            0,
            selected_unix_ms,
        )?;
        let targets_root = workspace_root.join("targets");
        write_immutable_record(
            &targets_root,
            &selection.selection_id,
            "target.json",
            &selection,
        )?;
        fs::create_dir_all(&targets_root)?;
        write_or_replace_json(&targets_root.join("current.json"), &selection)?;
        let mut plan = read_submission_target_plan(&workspace_root, workspace_id)?;
        plan.primary = Some(selection.clone());
        plan.backups
            .retain(|candidate| candidate.journal_id != journal_id);
        plan.updated_unix_ms = selected_unix_ms;
        write_or_replace_json(&targets_root.join("plan.json"), &plan)?;
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "submission_target_selected",
            &manifest.workspace,
            selected_unix_ms,
        )?;
        Ok(selection)
    }

    pub fn add_backup_recommended_journal(
        &self,
        workspace_id: &str,
        recommendation_run_id: &str,
        journal_id: &str,
    ) -> Result<SubmissionTargetPlan, WorkspaceError> {
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        let run = self
            .journal_recommendation_runs(workspace_id)?
            .into_iter()
            .find(|run| run.run_id == recommendation_run_id)
            .ok_or(WorkspaceError::SubmissionTargetNotFound)?;
        if run.manuscript_version != manifest.workspace.snapshot_version {
            return Err(WorkspaceError::StaleRecommendationRun);
        }
        let journal =
            journal_in_run(&run, journal_id).ok_or(WorkspaceError::SubmissionTargetNotFound)?;
        let mut plan = read_submission_target_plan(&workspace_root, workspace_id)?;
        if plan
            .primary
            .as_ref()
            .is_some_and(|target| target.journal_id == journal_id)
            || plan
                .backups
                .iter()
                .any(|target| target.journal_id == journal_id)
        {
            return Ok(plan);
        }
        if plan.backups.len() >= 8 {
            return Err(WorkspaceError::SubmissionBackupLimitReached);
        }
        let selected_unix_ms = unix_time_ms()?;
        let selection = build_target_selection(
            workspace_id,
            manifest.workspace.snapshot_version,
            recommendation_run_id,
            journal,
            run.resolved_article_type,
            "backup",
            plan.backups.len() as u32 + 1,
            selected_unix_ms,
        )?;
        let targets_root = workspace_root.join("targets");
        write_immutable_record(
            &targets_root,
            &selection.selection_id,
            "target.json",
            &selection,
        )?;
        plan.backups.push(selection);
        plan.updated_unix_ms = selected_unix_ms;
        write_or_replace_json(&targets_root.join("plan.json"), &plan)?;
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "submission_backup_added",
            &manifest.workspace,
            selected_unix_ms,
        )?;
        Ok(plan)
    }

    pub fn remove_backup_target(
        &self,
        workspace_id: &str,
        backup_selection_id: &str,
    ) -> Result<SubmissionTargetPlan, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let mut plan = read_submission_target_plan(&workspace_root, workspace_id)?;
        let index = plan
            .backups
            .iter()
            .position(|target| target.selection_id == backup_selection_id)
            .ok_or(WorkspaceError::SubmissionTargetNotFound)?;
        plan.backups.remove(index);
        let removed_unix_ms = target_change_unix_ms(&workspace_root, &manifest.workspace)?;
        plan.updated_unix_ms = removed_unix_ms;
        write_or_replace_json(&workspace_root.join("targets").join("plan.json"), &plan)?;
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "submission_backup_removed",
            &manifest.workspace,
            removed_unix_ms,
        )?;
        Ok(plan)
    }

    pub fn clear_primary_submission_target(
        &self,
        workspace_id: &str,
        primary_selection_id: &str,
        author_confirmed: bool,
    ) -> Result<SubmissionTargetPlan, WorkspaceError> {
        if !author_confirmed {
            return Err(WorkspaceError::AuthorConfirmationRequired);
        }
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        if self.lifecycle(workspace_id)?.submission.is_some() {
            return Err(WorkspaceError::SubmissionTargetLockedBySubmission);
        }
        let mut plan = read_submission_target_plan(&workspace_root, workspace_id)?;
        if plan
            .primary
            .as_ref()
            .is_none_or(|target| target.selection_id != primary_selection_id)
        {
            return Err(WorkspaceError::SubmissionTargetNotFound);
        }
        let cleared_unix_ms = target_change_unix_ms(&workspace_root, &manifest.workspace)?;
        plan.primary = None;
        plan.updated_unix_ms = cleared_unix_ms;
        let targets_root = workspace_root.join("targets");
        write_or_replace_json(&targets_root.join("plan.json"), &plan)?;
        let current_target_path = targets_root.join("current.json");
        if current_target_path.exists() {
            fs::remove_file(current_target_path)?;
        }
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "submission_primary_target_cleared",
            &manifest.workspace,
            cleared_unix_ms,
        )?;
        Ok(plan)
    }

    pub fn promote_backup_target(
        &self,
        workspace_id: &str,
        backup_selection_id: &str,
        reason: &str,
    ) -> Result<SubmissionTargetPlan, WorkspaceError> {
        if !matches!(reason, "not_submitted" | "rejected" | "withdrawn") {
            return Err(WorkspaceError::InvalidSubmissionTargetPlan);
        }
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        let mut plan = read_submission_target_plan(&workspace_root, workspace_id)?;
        let index = plan
            .backups
            .iter()
            .position(|target| target.selection_id == backup_selection_id)
            .ok_or(WorkspaceError::SubmissionTargetNotFound)?;
        let backup = plan.backups.remove(index);
        let prepared_requirements =
            read_journal_requirement_snapshot(&workspace_root, &backup.selection_id)?;
        let transitioned_unix_ms = target_change_unix_ms(&workspace_root, &manifest.workspace)?;
        let selection = build_target_selection_from_existing(
            &backup,
            manifest.workspace.snapshot_version,
            "primary",
            0,
            transitioned_unix_ms,
        )?;
        let targets_root = workspace_root.join("targets");
        write_immutable_record(
            &targets_root,
            &selection.selection_id,
            "target.json",
            &selection,
        )?;
        write_or_replace_json(&targets_root.join("current.json"), &selection)?;
        let transition_id = Uuid::new_v4().to_string();
        let from_selection_id = plan
            .primary
            .as_ref()
            .map(|target| target.selection_id.clone());
        let payload = SubmissionTargetTransitionPayload {
            schema_version: 1,
            transition_id: &transition_id,
            workspace_id,
            from_selection_id: &from_selection_id,
            to_selection_id: &selection.selection_id,
            reason,
            transitioned_unix_ms,
        };
        let transition_record_hash = hash_serializable(&payload)?;
        let transition = SubmissionTargetTransition {
            schema_version: 1,
            transition_id: transition_id.clone(),
            workspace_id: workspace_id.to_owned(),
            from_selection_id,
            to_selection_id: selection.selection_id.clone(),
            reason: reason.to_owned(),
            transitioned_unix_ms,
            record_hash: transition_record_hash,
        };
        write_immutable_record(
            &targets_root.join("transitions"),
            &transition_id,
            "transition.json",
            &transition,
        )?;
        plan.primary = Some(selection);
        plan.updated_unix_ms = transitioned_unix_ms;
        write_or_replace_json(&targets_root.join("plan.json"), &plan)?;
        if let (Some(snapshot), Some(primary)) = (prepared_requirements, plan.primary.as_ref()) {
            rebind_journal_requirement_snapshot(&workspace_root, primary, snapshot)?;
        }
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "submission_backup_promoted",
            &manifest.workspace,
            transitioned_unix_ms,
        )?;
        Ok(plan)
    }

    pub fn submission_target_plan(
        &self,
        workspace_id: &str,
    ) -> Result<SubmissionTargetPlan, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        read_submission_target_plan(&workspace_root, workspace_id)
    }

    pub fn submission_target(
        &self,
        workspace_id: &str,
    ) -> Result<Option<SubmissionTargetSelection>, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        read_submission_target(&workspace_root)
    }

    pub fn save_journal_requirement_snapshot(
        &self,
        workspace_id: &str,
        target_selection_id: &str,
        documents: &[JournalRequirementSourceDocument],
        source_mode: JournalRequirementSourceMode,
        author_attested_official: bool,
        external_transmission: &str,
    ) -> Result<JournalRequirementSnapshot, WorkspaceError> {
        if documents.is_empty()
            || documents.iter().any(|document| {
                !document.url.starts_with("https://")
                    || document.text.trim().chars().count() < 20
                    || document.text.chars().count() > 1_000_000
            })
            || (source_mode == JournalRequirementSourceMode::AuthorProvidedOfficialText
                && !author_attested_official)
        {
            return Err(WorkspaceError::InvalidJournalRequirementSource);
        }
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        let plan = read_submission_target_plan(&workspace_root, workspace_id)?;
        let target = plan
            .primary
            .iter()
            .chain(plan.backups.iter())
            .find(|target| target.selection_id == target_selection_id)
            .ok_or(WorkspaceError::SubmissionTargetNotFound)?;
        let captured_unix_ms = unix_time_ms()?;
        let (sources, requirements) = extract_journal_requirements(documents, captured_unix_ms);
        let status = if requirements.is_empty() {
            JournalRequirementStatus::RequiresManualReview
        } else if source_mode == JournalRequirementSourceMode::AuthorProvidedOfficialText {
            JournalRequirementStatus::AuthorAttestedOfficial
        } else {
            JournalRequirementStatus::OfficialSourcesCaptured
        };
        let mut limitations =
            vec!["自动抽取只建立带来源的准备清单，不替代作者对官网原文的最终核对".to_owned()];
        if requirements.is_empty() {
            limitations
                .push("已保存官方页面指纹，但未识别到明确投稿条目；请粘贴作者指南原文".to_owned());
        }
        if documents
            .iter()
            .any(|document| !document.official_host_matched)
        {
            limitations.push("部分来源由作者确认，域名未与期刊主页自动匹配".to_owned());
        }
        let fresh_until_unix_ms = captured_unix_ms
            .saturating_add(JOURNAL_REQUIREMENT_FRESHNESS_DAYS * 24 * 60 * 60 * 1_000);
        let snapshot_id = Uuid::new_v4().to_string();
        let payload = JournalRequirementSnapshotPayload {
            schema_version: JOURNAL_REQUIREMENT_SCHEMA_VERSION,
            snapshot_id: &snapshot_id,
            workspace_id,
            target_selection_id,
            journal_id: &target.journal_id,
            journal_name: &target.name,
            source_mode,
            status,
            sources: &sources,
            requirements: &requirements,
            limitations: &limitations,
            captured_unix_ms,
            fresh_until_unix_ms,
            external_transmission,
        };
        let requirement_record_hash = hash_serializable(&payload)?;
        let snapshot = JournalRequirementSnapshot {
            schema_version: JOURNAL_REQUIREMENT_SCHEMA_VERSION,
            snapshot_id: snapshot_id.clone(),
            workspace_id: workspace_id.to_owned(),
            target_selection_id: target_selection_id.to_owned(),
            journal_id: target.journal_id.clone(),
            journal_name: target.name.clone(),
            source_mode,
            status,
            sources,
            requirements,
            limitations,
            captured_unix_ms,
            fresh_until_unix_ms,
            record_hash: requirement_record_hash,
            external_transmission: external_transmission.to_owned(),
        };
        let requirements_root = workspace_root
            .join("targets")
            .join(target_selection_id)
            .join("requirements");
        write_immutable_record(
            &requirements_root,
            &snapshot_id,
            "requirements.json",
            &snapshot,
        )?;
        write_or_replace_json(&requirements_root.join("current.json"), &snapshot)?;
        append_audit_event(
            &workspace_root.join("audit.jsonl"),
            "journal_requirements_captured",
            &manifest.workspace,
            captured_unix_ms,
        )?;
        Ok(snapshot)
    }

    pub fn journal_requirement_snapshot(
        &self,
        workspace_id: &str,
        target_selection_id: &str,
    ) -> Result<Option<JournalRequirementSnapshot>, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        Uuid::parse_str(target_selection_id)
            .map_err(|_| WorkspaceError::InvalidSubmissionTargetPlan)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        read_journal_requirement_snapshot(&workspace_root, target_selection_id)
    }

    pub fn journal_requirement_snapshots(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<JournalRequirementSnapshot>, WorkspaceError> {
        let plan = self.submission_target_plan(workspace_id)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let mut snapshots = Vec::new();
        for target in plan.primary.iter().chain(plan.backups.iter()) {
            if let Some(snapshot) =
                read_journal_requirement_snapshot(&workspace_root, &target.selection_id)?
            {
                snapshots.push(snapshot);
            }
        }
        Ok(snapshots)
    }

    pub fn lifecycle(&self, workspace_id: &str) -> Result<WorkspaceLifecycle, WorkspaceError> {
        Uuid::parse_str(workspace_id).map_err(|_| WorkspaceError::InvalidWorkspaceId)?;
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        if manifest.workspace.id != workspace_id {
            return Err(WorkspaceError::InvalidWorkspaceId);
        }
        let structure_report = read_current_structure_report(&workspace_root, &manifest.workspace)?;
        let submission_target = read_submission_target(&workspace_root)?;
        let readiness_report = read_current_readiness_report(&workspace_root, &manifest.workspace)?
            .filter(|report| {
                readiness_matches_target(report, submission_target.as_ref(), &manifest.workspace)
            });
        let attestation = match &readiness_report {
            Some(report) => read_current_attestation(&workspace_root, &manifest.workspace, report)?,
            None => None,
        };
        let submission = match &attestation {
            Some(attestation) => read_current_submission(
                &workspace_root,
                &manifest.workspace,
                attestation,
                submission_target.as_ref(),
            )?,
            None => None,
        };
        let knowledge_body = match &submission {
            Some(submission) => {
                read_current_knowledge_body(&workspace_root, &manifest.workspace, submission)?
            }
            None => None,
        };
        let submission_target_plan = read_submission_target_plan(&workspace_root, workspace_id)?;
        let journal_requirements = submission_target
            .as_ref()
            .map(|selection| {
                read_journal_requirement_snapshot(&workspace_root, &selection.selection_id)
            })
            .transpose()?
            .flatten();
        let recommendation_ready = self
            .journal_recommendation_runs(workspace_id)?
            .iter()
            .any(|run| run.manuscript_version == manifest.workspace.snapshot_version);
        let submission_materials = build_submission_material_catalog(
            &manifest.workspace,
            read_stored_submission_materials(&workspace_root)?,
            structure_report.as_ref(),
            readiness_report.as_ref(),
            submission_target.as_ref(),
            journal_requirements.as_ref(),
            recommendation_ready,
            unix_time_ms()?,
        );
        Ok(WorkspaceLifecycle {
            workspace_id: workspace_id.to_owned(),
            current_version: manifest.workspace.snapshot_version,
            structure_report,
            readiness_report,
            attestation,
            submission,
            knowledge_body,
            submission_materials,
            submission_target,
            submission_target_plan,
            journal_requirements,
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
        let decomposition = match read_current_decomposition(&workspace_root, &manifest.workspace)?
        {
            Some(decomposition) => decomposition,
            None => {
                self.analyze_structure(workspace_id)?;
                read_current_decomposition(&workspace_root, &manifest.workspace)?.ok_or_else(
                    || WorkspaceError::InvalidManifest("未生成当前论文分解资产".to_owned()),
                )?
            }
        };
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
            "decomposition-manifest.json".to_owned(),
            "readiness-report.json".to_owned(),
            "readiness-preview.html".to_owned(),
            "local-attestation.json".to_owned(),
            "submission-manifest.json".to_owned(),
        ];
        let result = (|| {
            let snapshot = self.source_snapshot_path(workspace_id)?;
            verify_snapshot(&snapshot, &manifest.workspace)?;
            fs::copy(&snapshot, temporary_root.join(&files[0]))?;
            write_json(&temporary_root.join(&files[1]), &decomposition)?;
            let report_root = readiness_output_root(&workspace_root, &report.report_id);
            fs::copy(
                report_root.join(format!("readiness-v{}.json", report.report_version)),
                temporary_root.join(&files[2]),
            )?;
            fs::copy(
                report_root.join("preview.html"),
                temporary_root.join(&files[3]),
            )?;
            fs::copy(
                workspace_root
                    .join("attestations")
                    .join(&attestation.attestation_id)
                    .join("attestation.json"),
                temporary_root.join(&files[4]),
            )?;
            let package_manifest = SubmissionPackageManifest {
                schema_version: 1,
                workspace_id,
                manuscript_version: manifest.workspace.snapshot_version,
                manuscript_hash: &manifest.workspace.content_hash,
                decomposition_id: &decomposition.decomposition_id,
                decomposition_hash: &decomposition.manifest_hash,
                readiness_report_id: &report.report_id,
                attestation_id: &attestation.attestation_id,
                attestation_hash: &attestation.record_hash,
                created_unix_ms: exported_unix_ms,
                files: &files[..5],
                external_transmission: "not_performed",
            };
            write_json(&temporary_root.join(&files[5]), &package_manifest)?;
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

    pub fn export_target_submission_package(
        &self,
        workspace_id: &str,
        destination: &Path,
    ) -> Result<TargetSubmissionExport, WorkspaceError> {
        if !destination.is_dir() {
            return Err(WorkspaceError::InvalidExportDestination);
        }
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        let target = read_submission_target(&workspace_root)?
            .ok_or(WorkspaceError::SubmissionTargetNotFound)?;
        let journal_requirements =
            read_journal_requirement_snapshot(&workspace_root, &target.selection_id)?;
        let stored_materials = read_stored_submission_materials(&workspace_root)?;
        let exported_unix_ms = unix_time_ms()?;
        let package_plan = self.target_submission_package_plan(workspace_id)?;
        if !package_plan.ready {
            return Err(WorkspaceError::InvalidSubmissionMaterial(format!(
                "投稿包预检未通过：{}",
                package_plan.blockers.join("；")
            )));
        }
        let readiness_report = read_current_readiness_report(&workspace_root, &manifest.workspace)?
            .filter(|report| readiness_matches_target(report, Some(&target), &manifest.workspace));
        let target_component = safe_export_component(if target.name_en.trim().is_empty() {
            &target.name
        } else {
            &target.name_en
        });
        let package_name = format!(
            "ManuscriptDock-{}-v{}",
            if target_component.is_empty() {
                "target"
            } else {
                &target_component
            },
            manifest.workspace.snapshot_version
        );
        let final_root = destination.join(&package_name);
        if final_root.exists() {
            return Err(WorkspaceError::ExportDestinationExists);
        }
        let temporary_root = destination.join(format!(".manuscriptdock-{}.tmp", Uuid::new_v4()));
        let submission_root = temporary_root.join("submission");
        let records_root = temporary_root.join("records");
        fs::create_dir_all(&submission_root)?;
        fs::create_dir_all(&records_root)?;
        let warnings = package_plan.warnings.clone();
        let result = (|| {
            for planned_file in package_plan.files.iter().filter(|file| file.included) {
                let source = if let Some(material_id) = planned_file.material_id.as_deref() {
                    let stored = stored_materials
                        .materials
                        .iter()
                        .find(|item| item.material.material_id == material_id)
                        .ok_or_else(|| {
                            WorkspaceError::InvalidSubmissionMaterial(
                                "组包预览引用的材料已不存在".to_owned(),
                            )
                        })?;
                    let source = resolve_snapshot_path(&workspace_root, &stored.relative_path)?;
                    verify_file_hash(&source, &planned_file.content_hash)?;
                    source
                } else {
                    let source = self.source_snapshot_path(workspace_id)?;
                    verify_snapshot(&source, &manifest.workspace)?;
                    source
                };
                let destination = temporary_root.join(&planned_file.relative_path);
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(source, destination)?;
            }

            write_json(&records_root.join("target-selection.json"), &target)?;
            if let Some(snapshot) = journal_requirements.as_ref() {
                write_json(&records_root.join("journal-requirements.json"), snapshot)?;
            }
            if let Some(report) = readiness_report.as_ref() {
                write_json(&records_root.join("readiness-report.json"), &report)?;
            }
            let package_manifest = TargetSubmissionPackageManifest {
                schema_version: 2,
                workspace_id,
                manuscript_version: manifest.workspace.snapshot_version,
                manuscript_hash: &manifest.workspace.content_hash,
                target_selection: &target,
                journal_requirement_snapshot: journal_requirements.as_ref(),
                submission_files: &package_plan.files,
                warnings: &warnings,
                created_unix_ms: exported_unix_ms,
                external_transmission: "not_performed",
            };
            write_json(
                &records_root.join("package-manifest.json"),
                &package_manifest,
            )?;
            write_text(
                &temporary_root.join("README.txt"),
                &format!(
                    "ManuscriptDock 目标期刊投稿包\n\n目标：{}\n出版社：{}\n\n请只从 submission 文件夹选择期刊系统要求上传的文件。records 文件夹仅用于本地核验，不要上传。\n{}",
                    target.name,
                    target.publisher,
                    if warnings.is_empty() {
                        "当前通用必需材料检查已通过；仍须以期刊官网最新作者指南为准。".to_owned()
                    } else {
                        format!("尚有 {} 项提示，请在上传前逐项核对。", warnings.len())
                    }
                ),
            )?;
            let mut exported_files = package_plan
                .files
                .iter()
                .filter(|file| file.included)
                .map(|file| file.relative_path.clone())
                .collect::<Vec<_>>();
            exported_files.push("records/target-selection.json".to_owned());
            if records_root.join("readiness-report.json").is_file() {
                exported_files.push("records/readiness-report.json".to_owned());
            }
            if records_root.join("journal-requirements.json").is_file() {
                exported_files.push("records/journal-requirements.json".to_owned());
            }
            exported_files.push("records/package-manifest.json".to_owned());
            exported_files.push("README.txt".to_owned());
            fs::rename(&temporary_root, &final_root)?;
            append_audit_event(
                &workspace_root.join("audit.jsonl"),
                "target_submission_package_exported",
                &manifest.workspace,
                exported_unix_ms,
            )?;
            Ok(TargetSubmissionExport {
                package_name,
                manuscript_version: manifest.workspace.snapshot_version,
                target_selection_id: target.selection_id,
                target_name: target.name,
                files: exported_files,
                warnings,
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
        let requested_target = target.trim();
        if requested_target.is_empty() || requested_target.chars().count() > 200 {
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
        let selected_target = lifecycle
            .submission_target
            .as_ref()
            .ok_or(WorkspaceError::SubmissionTargetNotFound)?;
        if selected_target.selected_against_manuscript_version != lifecycle.current_version {
            return Err(WorkspaceError::InvalidSubmissionTargetPlan);
        }
        if requested_target != selected_target.name && requested_target != selected_target.name_en {
            return Err(WorkspaceError::InvalidSubmissionTargetPlan);
        }
        let target = selected_target.name.as_str();
        let target_selection_id = Some(selected_target.selection_id.clone());
        let publisher = Some(selected_target.publisher.clone());
        let attestation = match lifecycle.attestation {
            Some(attestation) => attestation,
            None => self.create_local_attestation(workspace_id, true)?,
        };
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
            schema_version: 2,
            submission_id: &submission_id,
            workspace_id,
            manuscript_version: manifest.workspace.snapshot_version,
            attestation_id: &attestation.attestation_id,
            target_selection_id: &target_selection_id,
            target,
            publisher: &publisher,
            receipt: &receipt,
            submitted_unix_ms,
            statement: &statement,
            external_transmission: &external_transmission,
        };
        let record_hash = hash_serializable(&payload)?;
        let record = SubmissionRecord {
            schema_version: 2,
            submission_id: submission_id.clone(),
            workspace_id: workspace_id.to_owned(),
            manuscript_version: manifest.workspace.snapshot_version,
            attestation_id: attestation.attestation_id,
            target_selection_id,
            target: target.to_owned(),
            publisher,
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
        decisions: &[KnowledgeCandidateDecision],
        author_confirmed: bool,
    ) -> Result<KnowledgeBodyRecord, WorkspaceError> {
        if !author_confirmed {
            return Err(WorkspaceError::AuthorConfirmationRequired);
        }
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
        let workspace_root = self.projects_root().join(workspace_id);
        let manifest = read_manifest(&workspace_root.join("manifest.json"))?;
        let decomposition = match read_current_decomposition(&workspace_root, &manifest.workspace)?
        {
            Some(decomposition) => Some(decomposition),
            None => {
                self.analyze_structure(workspace_id)?;
                read_current_decomposition(&workspace_root, &manifest.workspace)?
            }
        };
        let mut snapshot =
            local_knowledge_body_snapshot(&manifest.workspace, decomposition.as_ref());
        apply_candidate_decisions(&mut snapshot, decisions)?;
        snapshot.validate()?;
        if let Some(existing_record) = &existing {
            if existing_record
                .discipline_classification
                .as_ref()
                .is_some_and(|classification| classification.code == discipline.code)
                && existing_record.snapshot == snapshot
            {
                return Ok(existing_record.clone());
            }
        }
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
    if let Some(decomposition) = read_current_decomposition(workspace_root, workspace)? {
        return Ok(Some(decomposition.structure));
    }
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

fn read_current_decomposition(
    workspace_root: &Path,
    workspace: &WorkspaceSummary,
) -> Result<Option<DecompositionManifest>, WorkspaceError> {
    let hash_prefix = workspace
        .content_hash
        .get(..12)
        .ok_or_else(|| WorkspaceError::InvalidManifest("内容指纹长度无效".to_owned()))?;
    let path = workspace_root.join("analysis").join(format!(
        "decomposition-v{DECOMPOSITION_SCHEMA_VERSION}-a{STRUCTURE_ANALYSIS_VERSION}-{hash_prefix}.json"
    ));
    if !path.is_file() {
        return Ok(None);
    }
    let decomposition: DecompositionManifest = read_json(&path)?;
    if decomposition.schema_version != DECOMPOSITION_SCHEMA_VERSION
        || decomposition.workspace_id != workspace.id
        || decomposition.source_content_hash != workspace.content_hash
        || decomposition.source_snapshot_version != workspace.snapshot_version
        || decomposition.structure.workspace_id != workspace.id
        || decomposition.structure.source_content_hash != workspace.content_hash
        || decomposition.structure.source_snapshot_version != workspace.snapshot_version
        || !decomposition
            .declared_outputs
            .iter()
            .any(|output| output == "knowledge_body_candidates")
        || !decomposition
            .declared_outputs
            .iter()
            .any(|output| output == "submission_readiness_inputs")
    {
        return Err(WorkspaceError::InvalidManifest(
            "当前论文分解资产与源版本不一致".to_owned(),
        ));
    }
    let payload = DecompositionPayload {
        schema_version: decomposition.schema_version,
        decomposition_id: &decomposition.decomposition_id,
        workspace_id: &decomposition.workspace_id,
        source_content_hash: &decomposition.source_content_hash,
        source_snapshot_version: decomposition.source_snapshot_version,
        created_unix_ms: decomposition.created_unix_ms,
        structure: &decomposition.structure,
        declared_outputs: &decomposition.declared_outputs,
        external_transmission: &decomposition.external_transmission,
    };
    if hash_serializable(&payload)? != decomposition.manifest_hash {
        return Err(WorkspaceError::InvalidManifest(
            "当前论文分解资产哈希校验失败".to_owned(),
        ));
    }
    Ok(Some(decomposition))
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

fn readiness_matches_target(
    report: &ReadinessReport,
    target: Option<&SubmissionTargetSelection>,
    workspace: &WorkspaceSummary,
) -> bool {
    target.is_none_or(|target| {
        target.selected_against_manuscript_version == workspace.snapshot_version
            && report.generated_unix_ms >= target.selected_unix_ms
    })
}

fn target_change_unix_ms(
    workspace_root: &Path,
    workspace: &WorkspaceSummary,
) -> Result<u64, WorkspaceError> {
    let current = unix_time_ms()?;
    let after_readiness = read_current_readiness_report(workspace_root, workspace)?
        .map(|report| report.generated_unix_ms.saturating_add(1))
        .unwrap_or(0);
    Ok(current.max(after_readiness))
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
    target: Option<&SubmissionTargetSelection>,
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
            && target.is_none_or(|target| {
                record.target_selection_id.as_deref() == Some(target.selection_id.as_str())
                    || (record.target_selection_id.is_none()
                        && (record.target == target.name || record.target == target.name_en))
            })
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
    let expected = if record.schema_version >= 2 {
        hash_serializable(&SubmissionPayload {
            schema_version: record.schema_version,
            submission_id: &record.submission_id,
            workspace_id: &record.workspace_id,
            manuscript_version: record.manuscript_version,
            attestation_id: &record.attestation_id,
            target_selection_id: &record.target_selection_id,
            target: &record.target,
            publisher: &record.publisher,
            receipt: &record.receipt,
            submitted_unix_ms: record.submitted_unix_ms,
            statement: &record.statement,
            external_transmission: &record.external_transmission,
        })?
    } else {
        hash_serializable(&LegacySubmissionPayload {
            submission_id: &record.submission_id,
            workspace_id: &record.workspace_id,
            manuscript_version: record.manuscript_version,
            attestation_id: &record.attestation_id,
            target: &record.target,
            receipt: &record.receipt,
            submitted_unix_ms: record.submitted_unix_ms,
            statement: &record.statement,
            external_transmission: &record.external_transmission,
        })?
    };
    if expected != record.record_hash {
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

fn copy_workspace_tree(
    source: &Path,
    destination: &Path,
    file_count: &mut u32,
) -> Result<(), WorkspaceError> {
    fs::create_dir(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(WorkspaceError::InvalidManifest(
                "工作区包含符号链接，已停止另存".to_owned(),
            ));
        }
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_workspace_tree(&entry.path(), &target, file_count)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target)?;
            *file_count = file_count.saturating_add(1);
        } else {
            return Err(WorkspaceError::InvalidManifest(
                "工作区包含不支持的文件类型，已停止另存".to_owned(),
            ));
        }
    }
    Ok(())
}

fn safe_export_component(value: &str) -> String {
    value
        .chars()
        .filter_map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_') {
                Some(character)
            } else if character.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .take(60)
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
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

fn is_allowed_submission_material_extension(extension: &str) -> bool {
    matches!(
        extension,
        "doc"
            | "docx"
            | "odt"
            | "rtf"
            | "tex"
            | "zip"
            | "tar"
            | "gz"
            | "tgz"
            | "bib"
            | "bbl"
            | "bst"
            | "cls"
            | "sty"
            | "ris"
            | "nbib"
            | "enw"
            | "pdf"
            | "eps"
            | "ps"
            | "svg"
            | "png"
            | "jpg"
            | "jpeg"
            | "tif"
            | "tiff"
            | "csv"
            | "tsv"
            | "xls"
            | "xlsx"
            | "ods"
            | "ppt"
            | "pptx"
            | "odp"
            | "txt"
            | "md"
            | "json"
            | "xml"
            | "mp4"
            | "mov"
            | "avi"
            | "webm"
            | "mpeg"
            | "mpg"
            | "mp3"
            | "wav"
            | "m4a"
            | "sav"
            | "dta"
            | "mat"
            | "h5"
            | "hdf5"
            | "parquet"
    )
}

fn is_allowed_submission_material_kind_extension(
    kind: SubmissionMaterialKind,
    extension: &str,
) -> bool {
    match kind {
        SubmissionMaterialKind::SourceProject => matches!(extension, "zip" | "tar" | "gz" | "tgz"),
        SubmissionMaterialKind::BlindedManuscript => {
            matches!(extension, "doc" | "docx" | "odt" | "rtf" | "tex" | "pdf")
        }
        SubmissionMaterialKind::Figure => matches!(
            extension,
            "pdf" | "eps" | "ps" | "svg" | "png" | "jpg" | "jpeg" | "tif" | "tiff"
        ),
        SubmissionMaterialKind::Table => matches!(
            extension,
            "csv" | "tsv" | "xls" | "xlsx" | "ods" | "doc" | "docx" | "odt" | "rtf" | "tex"
        ),
        SubmissionMaterialKind::Bibliography => {
            matches!(
                extension,
                "bib"
                    | "bbl"
                    | "ris"
                    | "nbib"
                    | "enw"
                    | "xml"
                    | "txt"
                    | "doc"
                    | "docx"
                    | "odt"
                    | "rtf"
            )
        }
        SubmissionMaterialKind::CoverLetter
        | SubmissionMaterialKind::TitlePage
        | SubmissionMaterialKind::Declaration => {
            matches!(
                extension,
                "doc" | "docx" | "odt" | "rtf" | "tex" | "pdf" | "txt"
            )
        }
        SubmissionMaterialKind::Supplementary | SubmissionMaterialKind::Other => {
            is_allowed_submission_material_extension(extension)
        }
    }
}

fn submission_material_kind_extension_help(kind: SubmissionMaterialKind) -> &'static str {
    match kind {
        SubmissionMaterialKind::SourceProject => "源文件工程只接受 ZIP、TAR、GZ 或 TGZ",
        SubmissionMaterialKind::BlindedManuscript => {
            "匿名主稿只接受 DOC、DOCX、ODT、RTF、TEX 或 PDF"
        }
        SubmissionMaterialKind::Figure => {
            "图片栏只接受 PDF、EPS、PS、SVG、PNG、JPG 或 TIFF；CSV/TSV/Excel 请从“可编辑表格”上传"
        }
        SubmissionMaterialKind::Table => {
            "表格栏只接受 CSV、TSV、Excel、ODS、Word、RTF 或 TEX；图片文件请从“原始图件”上传"
        }
        SubmissionMaterialKind::Bibliography => {
            "参考文献栏只接受 BIB、BBL、RIS、NBIB、ENW、XML、Word、RTF 或 TXT"
        }
        SubmissionMaterialKind::CoverLetter
        | SubmissionMaterialKind::TitlePage
        | SubmissionMaterialKind::Declaration => {
            "该文档栏只接受 DOC、DOCX、ODT、RTF、TEX、PDF 或 TXT"
        }
        SubmissionMaterialKind::Supplementary | SubmissionMaterialKind::Other => {
            "文件类型与当前投稿资料类别不匹配"
        }
    }
}

fn is_valid_utf16_text(content: &[u8]) -> bool {
    if content.len() < 4 {
        return false;
    }
    let (little_endian, payload) = if content.starts_with(&[0xff, 0xfe]) {
        (true, &content[2..])
    } else if content.starts_with(&[0xfe, 0xff]) {
        (false, &content[2..])
    } else {
        let pairs = content.chunks_exact(2).take(128).collect::<Vec<_>>();
        if pairs.len() < 4 {
            return false;
        }
        let little_zeros = pairs.iter().filter(|pair| pair[1] == 0).count();
        let big_zeros = pairs.iter().filter(|pair| pair[0] == 0).count();
        if little_zeros.max(big_zeros) * 3 < pairs.len() * 2 {
            return false;
        }
        (little_zeros >= big_zeros, content)
    };
    let units = payload
        .chunks_exact(2)
        .map(|pair| {
            if little_endian {
                u16::from_le_bytes([pair[0], pair[1]])
            } else {
                u16::from_be_bytes([pair[0], pair[1]])
            }
        })
        .collect::<Vec<_>>();
    String::from_utf16(&units).ok().is_some_and(|text| {
        !text.is_empty()
            && text
                .chars()
                .all(|character| !character.is_control() || matches!(character, '\n' | '\r' | '\t'))
    })
}

struct SubmissionMaterialValidation {
    status: String,
    issues: Vec<String>,
    detected_media_type: Option<String>,
}

fn validate_submission_material(
    path: &Path,
    extension: &str,
    kind: SubmissionMaterialKind,
) -> Result<SubmissionMaterialValidation, WorkspaceError> {
    if !is_allowed_submission_material_kind_extension(kind, extension) {
        return Err(WorkspaceError::InvalidSubmissionMaterial(
            submission_material_kind_extension_help(kind).to_owned(),
        ));
    }
    let mut file = File::open(path)?;
    let mut header = [0_u8; 16];
    let read = file.read(&mut header)?;
    let bytes = &header[..read];
    let detected = if bytes.starts_with(b"%PDF-") {
        Some("application/pdf")
    } else if bytes.starts_with(b"PK\x03\x04") {
        Some("application/zip")
    } else if bytes.starts_with(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1") {
        Some("application/x-ole-storage")
    } else if bytes.starts_with(b"\x1f\x8b") {
        Some("application/gzip")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        Some("image/tiff")
    } else if bytes.starts_with(b"%!PS-Adobe") {
        Some("application/postscript")
    } else if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        Some("video/mp4")
    } else if bytes.starts_with(b"RIFF") && bytes.len() >= 12 && &bytes[8..12] == b"WAVE" {
        Some("audio/wav")
    } else if bytes.starts_with(b"\x1a\x45\xdf\xa3") {
        Some("video/webm")
    } else if bytes.starts_with(b"ID3")
        || (bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0)
    {
        Some("audio/mpeg")
    } else {
        None
    };
    let mut issues = Vec::new();
    let mut blocked = false;
    if matches!(extension, "csv" | "tsv") && detected == Some("application/x-ole-storage") {
        blocked = true;
        issues.push(format!(
            "文件内容实际是旧版 Microsoft Excel（.xls），不是 {} 文本；请将扩展名改为 .xls 后从“可编辑表格”重新上传，或在 Excel 中另存为真正的 {}",
            extension.to_ascii_uppercase(),
            extension.to_ascii_uppercase()
        ));
    } else if matches!(extension, "csv" | "tsv") && detected == Some("application/zip") {
        blocked = true;
        issues.push(format!(
            "文件内容实际是 ZIP/Office Open XML 工作簿，不是 {} 文本；请恢复正确的 .xlsx 扩展名后从“可编辑表格”重新上传，或另存为真正的 {}",
            extension.to_ascii_uppercase(),
            extension.to_ascii_uppercase()
        ));
    }
    let signature_expected = match extension {
        "pdf" => Some("application/pdf"),
        "docx" | "xlsx" | "odt" | "ods" | "pptx" | "odp" | "zip" => Some("application/zip"),
        "doc" | "xls" | "ppt" => Some("application/x-ole-storage"),
        "gz" | "tgz" => Some("application/gzip"),
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "tif" | "tiff" => Some("image/tiff"),
        "eps" | "ps" => Some("application/postscript"),
        "mp4" | "mov" | "m4a" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "wav" => Some("audio/wav"),
        "mp3" => Some("audio/mpeg"),
        _ => None,
    };
    if let Some(expected) = signature_expected {
        if detected != Some(expected) {
            blocked = true;
            issues.push("扩展名与文件签名不一致或文件已损坏".to_owned());
        }
    }
    if !blocked && extension == "pdf" {
        match lopdf::Document::load(path) {
            Ok(document) if document.is_encrypted() => {
                blocked = true;
                issues.push("PDF 已加密，出版社系统可能无法读取".to_owned());
            }
            Ok(_) => {}
            Err(_) => {
                blocked = true;
                issues.push("PDF 结构无法解析".to_owned());
            }
        }
    }
    if !blocked
        && matches!(
            extension,
            "docx" | "xlsx" | "odt" | "ods" | "pptx" | "odp" | "zip"
        )
    {
        let mut archive = ZipArchive::new(File::open(path)?).map_err(|_| {
            WorkspaceError::InvalidSubmissionMaterial("压缩文件结构无法解析".to_owned())
        })?;
        if extension == "docx" {
            if archive.by_name("[Content_Types].xml").is_err()
                || archive.by_name("word/document.xml").is_err()
            {
                blocked = true;
                issues.push("DOCX 缺少必要的文档结构".to_owned());
            } else {
                if archive.by_name("word/comments.xml").is_ok() {
                    issues.push("DOCX 仍包含批注，请在上传前确认是否应移除".to_owned());
                }
                if let Ok(mut settings) = archive.by_name("word/settings.xml") {
                    let mut xml = String::new();
                    settings.read_to_string(&mut xml)?;
                    if xml.contains("trackRevisions") {
                        issues.push("DOCX 仍启用修订跟踪，请在上传前确认是否应接受修订".to_owned());
                    }
                }
            }
        } else if extension == "xlsx" && archive.by_name("xl/workbook.xml").is_err() {
            blocked = true;
            issues.push("XLSX 缺少必要的工作簿结构".to_owned());
        } else if extension == "pptx" && archive.by_name("ppt/presentation.xml").is_err() {
            blocked = true;
            issues.push("PPTX 缺少必要的演示文稿结构".to_owned());
        } else if matches!(extension, "odt" | "ods" | "odp") && archive.by_name("mimetype").is_err()
        {
            blocked = true;
            issues.push("OpenDocument 文件缺少必要的 mimetype 结构".to_owned());
        }
    }
    if !blocked
        && matches!(kind, SubmissionMaterialKind::Figure)
        && matches!(extension, "png" | "jpg" | "jpeg" | "tif" | "tiff" | "eps")
    {
        issues.push(
            "图片可打开性已初检；分辨率、DPI、色彩空间和版面尺寸仍需按期刊原文核验".to_owned(),
        );
    }
    if !blocked
        && matches!(
            extension,
            "rtf"
                | "tex"
                | "bib"
                | "bbl"
                | "bst"
                | "cls"
                | "sty"
                | "ris"
                | "nbib"
                | "enw"
                | "csv"
                | "tsv"
                | "txt"
                | "md"
                | "json"
                | "xml"
                | "svg"
        )
    {
        let mut content = Vec::new();
        File::open(path)?.take(4096).read_to_end(&mut content)?;
        if content.contains(&0) {
            if is_valid_utf16_text(&content) {
                issues.push(
                    "文本文件使用 UTF-16 编码；已按文本接收，投稿前请核对期刊要求的字符编码"
                        .to_owned(),
                );
            } else {
                blocked = true;
                issues.push("文本类文件包含异常二进制内容".to_owned());
            }
        }
    }
    Ok(SubmissionMaterialValidation {
        status: if blocked {
            "blocked"
        } else if issues.is_empty() {
            "passed"
        } else {
            "warning"
        }
        .to_owned(),
        issues,
        detected_media_type: detected.map(str::to_owned),
    })
}

fn material_kind_folder(kind: SubmissionMaterialKind) -> &'static str {
    match kind {
        SubmissionMaterialKind::SourceProject => "source-project",
        SubmissionMaterialKind::BlindedManuscript => "blinded-manuscript",
        SubmissionMaterialKind::Figure => "figures",
        SubmissionMaterialKind::Table => "tables",
        SubmissionMaterialKind::Bibliography => "bibliography",
        SubmissionMaterialKind::Supplementary => "supplementary",
        SubmissionMaterialKind::CoverLetter => "cover-letter",
        SubmissionMaterialKind::TitlePage => "title-page",
        SubmissionMaterialKind::Declaration => "declarations",
        SubmissionMaterialKind::Other => "other",
    }
}

fn material_kind_role(kind: SubmissionMaterialKind) -> &'static str {
    match kind {
        SubmissionMaterialKind::SourceProject => "source_project",
        SubmissionMaterialKind::BlindedManuscript => "blinded_manuscript",
        SubmissionMaterialKind::Figure => "figure",
        SubmissionMaterialKind::Table => "table",
        SubmissionMaterialKind::Bibliography => "bibliography",
        SubmissionMaterialKind::Supplementary => "supplementary_file",
        SubmissionMaterialKind::CoverLetter => "cover_letter",
        SubmissionMaterialKind::TitlePage => "title_page",
        SubmissionMaterialKind::Declaration => "declaration",
        SubmissionMaterialKind::Other => "other_supporting_file",
    }
}

fn safe_export_file_name(value: &str) -> String {
    let path = Path::new(value);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(safe_export_component)
        .unwrap_or_default();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| is_allowed_submission_material_extension(value));
    match (stem.is_empty(), extension) {
        (false, Some(extension)) => format!("{stem}.{extension}"),
        (false, None) => stem,
        _ => String::new(),
    }
}

fn read_stored_submission_materials(
    workspace_root: &Path,
) -> Result<StoredSubmissionMaterialCatalog, WorkspaceError> {
    let path = workspace_root.join("materials").join("catalog.json");
    if !path.exists() {
        return Ok(StoredSubmissionMaterialCatalog {
            schema_version: 3,
            materials: Vec::new(),
            confirmations: Vec::new(),
        });
    }
    let catalog: StoredSubmissionMaterialCatalog = read_json(&path)?;
    for item in &catalog.materials {
        let stored_path = resolve_submission_material_path(workspace_root, item)?;
        verify_file_hash(&stored_path, &item.material.content_hash)?;
    }
    Ok(catalog)
}

fn resolve_submission_material_path(
    workspace_root: &Path,
    item: &StoredSubmissionMaterial,
) -> Result<PathBuf, WorkspaceError> {
    let expected = format!(
        "materials/files/{}.{}",
        item.material.material_id, item.material.extension
    );
    if item.relative_path != expected || !is_safe_relative_path(&item.relative_path) {
        return Err(WorkspaceError::InvalidSubmissionMaterial(format!(
            "附件 {} 的本地路径无效",
            item.material.original_name
        )));
    }
    let stored_path = workspace_root.join(&item.relative_path);
    let metadata = fs::symlink_metadata(&stored_path).map_err(|_| {
        WorkspaceError::InvalidSubmissionMaterial(format!(
            "缺少已登记文件 {}",
            item.material.original_name
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(WorkspaceError::InvalidSubmissionMaterial(format!(
            "附件 {} 不是安全的本地普通文件",
            item.material.original_name
        )));
    }
    Ok(stored_path)
}

fn read_submission_target(
    workspace_root: &Path,
) -> Result<Option<SubmissionTargetSelection>, WorkspaceError> {
    let path = workspace_root.join("targets").join("current.json");
    if !path.exists() {
        return Ok(None);
    }
    let target: SubmissionTargetSelection = read_json(&path)?;
    Ok(Some(target))
}

fn read_submission_target_plan(
    workspace_root: &Path,
    workspace_id: &str,
) -> Result<SubmissionTargetPlan, WorkspaceError> {
    let path = workspace_root.join("targets").join("plan.json");
    if path.exists() {
        let plan: SubmissionTargetPlan = read_json(&path)?;
        if plan.workspace_id != workspace_id || plan.schema_version != 1 {
            return Err(WorkspaceError::InvalidSubmissionTargetPlan);
        }
        return Ok(plan);
    }
    Ok(SubmissionTargetPlan {
        schema_version: 1,
        workspace_id: workspace_id.to_owned(),
        primary: read_submission_target(workspace_root)?,
        backups: Vec::new(),
        updated_unix_ms: 0,
    })
}

fn read_journal_requirement_snapshot(
    workspace_root: &Path,
    target_selection_id: &str,
) -> Result<Option<JournalRequirementSnapshot>, WorkspaceError> {
    let path = workspace_root
        .join("targets")
        .join(target_selection_id)
        .join("requirements")
        .join("current.json");
    if !path.exists() {
        return Ok(None);
    }
    let snapshot: JournalRequirementSnapshot = read_json(&path)?;
    if snapshot.schema_version != JOURNAL_REQUIREMENT_SCHEMA_VERSION
        || snapshot.target_selection_id != target_selection_id
    {
        return Err(WorkspaceError::InvalidJournalRequirementSource);
    }
    let payload = JournalRequirementSnapshotPayload {
        schema_version: snapshot.schema_version,
        snapshot_id: &snapshot.snapshot_id,
        workspace_id: &snapshot.workspace_id,
        target_selection_id: &snapshot.target_selection_id,
        journal_id: &snapshot.journal_id,
        journal_name: &snapshot.journal_name,
        source_mode: snapshot.source_mode,
        status: snapshot.status,
        sources: &snapshot.sources,
        requirements: &snapshot.requirements,
        limitations: &snapshot.limitations,
        captured_unix_ms: snapshot.captured_unix_ms,
        fresh_until_unix_ms: snapshot.fresh_until_unix_ms,
        external_transmission: &snapshot.external_transmission,
    };
    if hash_serializable(&payload)? != snapshot.record_hash {
        return Err(WorkspaceError::InvalidJournalRequirementSource);
    }
    Ok(Some(snapshot))
}

fn rebind_journal_requirement_snapshot(
    workspace_root: &Path,
    target: &SubmissionTargetSelection,
    mut snapshot: JournalRequirementSnapshot,
) -> Result<(), WorkspaceError> {
    snapshot.snapshot_id = Uuid::new_v4().to_string();
    snapshot.target_selection_id = target.selection_id.clone();
    snapshot.journal_id = target.journal_id.clone();
    snapshot.journal_name = target.name.clone();
    snapshot
        .limitations
        .push("该要求快照由已准备的备选支线继承；正式投稿前仍应检查新鲜度".to_owned());
    let payload = JournalRequirementSnapshotPayload {
        schema_version: snapshot.schema_version,
        snapshot_id: &snapshot.snapshot_id,
        workspace_id: &snapshot.workspace_id,
        target_selection_id: &snapshot.target_selection_id,
        journal_id: &snapshot.journal_id,
        journal_name: &snapshot.journal_name,
        source_mode: snapshot.source_mode,
        status: snapshot.status,
        sources: &snapshot.sources,
        requirements: &snapshot.requirements,
        limitations: &snapshot.limitations,
        captured_unix_ms: snapshot.captured_unix_ms,
        fresh_until_unix_ms: snapshot.fresh_until_unix_ms,
        external_transmission: &snapshot.external_transmission,
    };
    snapshot.record_hash = hash_serializable(&payload)?;
    let requirements_root = workspace_root
        .join("targets")
        .join(&target.selection_id)
        .join("requirements");
    write_immutable_record(
        &requirements_root,
        &snapshot.snapshot_id,
        "requirements.json",
        &snapshot,
    )?;
    write_or_replace_json(&requirements_root.join("current.json"), &snapshot)
}

#[allow(clippy::too_many_arguments)]
fn build_target_selection(
    workspace_id: &str,
    manuscript_version: u32,
    recommendation_run_id: &str,
    journal: &JournalRecommendation,
    article_type: ArticleTypePreference,
    plan_role: &str,
    priority: u32,
    selected_unix_ms: u64,
) -> Result<SubmissionTargetSelection, WorkspaceError> {
    let region = match journal.region {
        crate::JournalRegion::Domestic => "domestic",
        crate::JournalRegion::International => "international",
    }
    .to_owned();
    finalize_target_selection(SubmissionTargetSelection {
        schema_version: 3,
        selection_id: Uuid::new_v4().to_string(),
        workspace_id: workspace_id.to_owned(),
        selected_against_manuscript_version: manuscript_version,
        recommendation_run_id: recommendation_run_id.to_owned(),
        journal_id: journal.id.clone(),
        name: journal.name.clone(),
        name_en: journal.name_en.clone(),
        publisher: journal.publisher.clone(),
        region,
        rank_system: journal.rank_system.clone(),
        rank_tier: journal.rank_tier.clone(),
        homepage_url: journal.homepage_url.clone(),
        article_type,
        plan_role: plan_role.to_owned(),
        priority,
        selected_unix_ms,
        record_hash: String::new(),
        external_transmission: "not_performed".to_owned(),
    })
}

fn build_target_selection_from_existing(
    existing: &SubmissionTargetSelection,
    manuscript_version: u32,
    plan_role: &str,
    priority: u32,
    selected_unix_ms: u64,
) -> Result<SubmissionTargetSelection, WorkspaceError> {
    let mut selection = existing.clone();
    selection.schema_version = 3;
    selection.selection_id = Uuid::new_v4().to_string();
    selection.selected_against_manuscript_version = manuscript_version;
    selection.plan_role = plan_role.to_owned();
    selection.priority = priority;
    selection.selected_unix_ms = selected_unix_ms;
    selection.record_hash.clear();
    finalize_target_selection(selection)
}

fn finalize_target_selection(
    mut selection: SubmissionTargetSelection,
) -> Result<SubmissionTargetSelection, WorkspaceError> {
    let payload = SubmissionTargetPayload {
        schema_version: selection.schema_version,
        selection_id: &selection.selection_id,
        workspace_id: &selection.workspace_id,
        selected_against_manuscript_version: selection.selected_against_manuscript_version,
        recommendation_run_id: &selection.recommendation_run_id,
        journal_id: &selection.journal_id,
        name: &selection.name,
        name_en: &selection.name_en,
        publisher: &selection.publisher,
        region: &selection.region,
        rank_system: &selection.rank_system,
        rank_tier: &selection.rank_tier,
        homepage_url: &selection.homepage_url,
        article_type: selection.article_type,
        plan_role: &selection.plan_role,
        priority: selection.priority,
        selected_unix_ms: selection.selected_unix_ms,
        external_transmission: &selection.external_transmission,
    };
    selection.record_hash = hash_serializable(&payload)?;
    Ok(selection)
}

fn journal_in_run<'a>(
    run: &'a JournalRecommendationRun,
    journal_id: &str,
) -> Option<&'a JournalRecommendation> {
    [&run.domestic, &run.international]
        .into_iter()
        .flat_map(|portfolio| {
            portfolio
                .sprint
                .iter()
                .chain(portfolio.matching.iter())
                .chain(portfolio.safeguard.iter())
        })
        .find(|journal| journal.id == journal_id)
}

#[allow(clippy::too_many_arguments)]
fn build_submission_material_catalog(
    workspace: &WorkspaceSummary,
    stored: StoredSubmissionMaterialCatalog,
    structure: Option<&StructureReport>,
    readiness: Option<&ReadinessReport>,
    target: Option<&SubmissionTargetSelection>,
    journal_requirements: Option<&JournalRequirementSnapshot>,
    recommendation_ready: bool,
    now_unix_ms: u64,
) -> SubmissionMaterialCatalog {
    let matching_material_ids = |item_id: &str, kind| {
        stored
            .materials
            .iter()
            .filter(|item| {
                item.material.kind == kind
                    && item.material.included
                    && item.material.validation_status != "blocked"
                    && item.material.manuscript_version == workspace.snapshot_version
                    && item.material.target_selection_id.as_deref()
                        == target.map(|selection| selection.selection_id.as_str())
                    && item.material.checklist_item_id.as_deref() == Some(item_id)
                    && item.material.requirement_snapshot_id.as_deref()
                        == journal_requirements.map(|snapshot| snapshot.snapshot_id.as_str())
            })
            .map(|item| item.material.material_id.clone())
            .collect::<Vec<_>>()
    };
    let figures_expected = structure.is_some_and(|report| report.figure_count > 0);
    let tables_expected = structure.is_some_and(|report| report.table_count > 0);
    let target_current = target.is_some_and(|selection| {
        selection.selected_against_manuscript_version == workspace.snapshot_version
    });
    let requirements_current =
        target
            .zip(journal_requirements)
            .is_some_and(|(selection, snapshot)| {
                snapshot.target_selection_id == selection.selection_id
                    && snapshot.status != JournalRequirementStatus::RequiresManualReview
                    && !snapshot.requirements.is_empty()
                    && snapshot.fresh_until_unix_ms >= now_unix_ms
            });
    let confirmed = |item_id: &str| {
        target
            .zip(journal_requirements)
            .is_some_and(|(selection, snapshot)| {
                stored.confirmations.iter().any(|confirmation| {
                    confirmation.item_id == item_id
                        && confirmation.target_selection_id == selection.selection_id
                        && confirmation.requirement_snapshot_id == snapshot.snapshot_id
                })
            })
    };
    let mut checklist = vec![static_submission_checklist_item(
        "target-journal",
        "目标期刊与文章类型",
        "target",
        if target_current { "passed" } else { "missing" },
        match target {
            Some(selection) if target_current => format!(
                "已选择 {}（{}）· 文章类型 {}",
                selection.name,
                selection.publisher,
                article_type_label(selection.article_type)
            ),
            Some(selection) => format!("已选择 {}，需按当前稿件版本复核", selection.name),
            None => "请先从初步推荐中选择一个主投期刊".to_owned(),
        },
    )];
    checklist.push(static_submission_checklist_item(
        "official-journal-requirements",
        "期刊官方投稿要求",
        "target",
        if requirements_current {
            "passed"
        } else {
            "missing"
        },
        match journal_requirements {
            Some(snapshot) if requirements_current => format!(
                "已保存 {} 个官方来源、{} 项带证据要求",
                snapshot.sources.len(),
                snapshot.requirements.len()
            ),
            Some(_) => "已有抓取记录，但仍需补充、更新或核对官方作者指南".to_owned(),
            None if target.is_some() => "请取得或录入该刊官方作者指南".to_owned(),
            None => "选择主投期刊后再取得其官方要求".to_owned(),
        },
    ));
    checklist.push(static_submission_checklist_item(
        "main-manuscript",
        "当前主稿",
        "manuscript",
        "passed",
        format!("已保存不可变稿件 v{}", workspace.snapshot_version),
    ));
    checklist.push(static_submission_checklist_item(
        "current-check",
        "按目标期刊重新检查",
        "target",
        if readiness.is_some() {
            "passed"
        } else {
            "missing"
        },
        if readiness.is_some() {
            "当前稿件版本已有目标专属检查报告".to_owned()
        } else {
            "补齐材料后运行一次与当前目标绑定的投稿检查".to_owned()
        },
    ));
    if workspace.manuscript.kind == ManuscriptKind::Latex {
        checklist.push(file_submission_checklist_item(
            "latex-project",
            "完整 LaTeX 工程",
            "files",
            true,
            matching_material_ids("latex-project", SubmissionMaterialKind::SourceProject),
            1,
            SubmissionMaterialKind::SourceProject,
            "请提供含图片、参考文献和自定义样式的 ZIP/TAR 工程包".to_owned(),
            None,
            journal_requirements,
        ));
    }
    if let Some(snapshot) = journal_requirements.filter(|_| requirements_current) {
        for requirement in &snapshot.requirements {
            let id = format!(
                "journal-{}",
                requirement
                    .id
                    .strip_prefix("requirement-")
                    .unwrap_or(&requirement.id)
            );
            let blocking = requirement.obligation != JournalRequirementObligation::Recommended;
            let item = match requirement.category {
                JournalRequirementCategory::AnonymousReview
                    if requirement.obligation == JournalRequirementObligation::Required =>
                {
                    file_submission_checklist_item(
                    &id,
                    &requirement.label,
                    "files",
                    blocking,
                    matching_material_ids(&id, SubmissionMaterialKind::BlindedManuscript),
                    1,
                    SubmissionMaterialKind::BlindedManuscript,
                    "如该刊采用匿名评审，请提供不含作者身份的独立主稿".to_owned(),
                    Some(requirement),
                    journal_requirements,
                    )
                }
                JournalRequirementCategory::AnonymousReview => {
                    submission_checklist_item(
                        &id,
                        &requirement.label,
                        "manuscript",
                        if blocking { "required" } else { "recommended" },
                        if blocking { "manual_verification" } else { "recommended" },
                        "原文提到匿名评审但没有识别到明确适用方式；请补充或更新官方要求，系统不会自动切换为双盲包".to_owned(),
                        "manual",
                        None,
                        blocking,
                        false,
                        Some(requirement),
                        Some(snapshot),
                    )
                }
                JournalRequirementCategory::TitlePage => file_submission_checklist_item(
                    &id,
                    &requirement.label,
                    "files",
                    blocking,
                    matching_material_ids(&id, SubmissionMaterialKind::TitlePage),
                    1,
                    SubmissionMaterialKind::TitlePage,
                    "按官方要求提供独立标题页".to_owned(),
                    Some(requirement),
                    journal_requirements,
                ),
                JournalRequirementCategory::SupplementaryFiles => file_submission_checklist_item(
                    &id,
                    &requirement.label,
                    "files",
                    blocking,
                    matching_material_ids(&id, SubmissionMaterialKind::Supplementary),
                    1,
                    SubmissionMaterialKind::Supplementary,
                    "按官方要求提供适用的补充材料".to_owned(),
                    Some(requirement),
                    journal_requirements,
                ),
                JournalRequirementCategory::CoverLetter => file_submission_checklist_item(
                    &id,
                    &requirement.label,
                    "files",
                    blocking,
                    matching_material_ids(&id, SubmissionMaterialKind::CoverLetter),
                    1,
                    SubmissionMaterialKind::CoverLetter,
                    "准备面向该刊编辑的投稿附信".to_owned(),
                    Some(requirement),
                    journal_requirements,
                ),
                JournalRequirementCategory::OtherSupportingFiles => file_submission_checklist_item(
                    &id,
                    &requirement.label,
                    "files",
                    blocking,
                    matching_material_ids(&id, SubmissionMaterialKind::Other),
                    1,
                    SubmissionMaterialKind::Other,
                    "请提供作者指南指出的清单、授权书、协议或其他支持文件".to_owned(),
                    Some(requirement),
                    journal_requirements,
                ),
                JournalRequirementCategory::Figures if figures_expected => {
                    file_submission_checklist_item(
                        &id,
                        &requirement.label,
                        "files",
                        blocking,
                        matching_material_ids(&id, SubmissionMaterialKind::Figure),
                        structure.map_or(1, |report| report.figure_count.max(1) as usize),
                        SubmissionMaterialKind::Figure,
                        "正文包含图片，请提供独立高精度原图".to_owned(),
                        Some(requirement),
                        journal_requirements,
                    )
                }
                JournalRequirementCategory::Tables if tables_expected => {
                    file_submission_checklist_item(
                        &id,
                        &requirement.label,
                        "files",
                        blocking,
                        matching_material_ids(&id, SubmissionMaterialKind::Table),
                        structure.map_or(1, |report| report.table_count.max(1) as usize),
                        SubmissionMaterialKind::Table,
                        "正文包含表格，请提供期刊要求的可编辑表格".to_owned(),
                        Some(requirement),
                        journal_requirements,
                    )
                }
                JournalRequirementCategory::Abstract => automatic_submission_checklist_item(
                    &id,
                    &requirement.label,
                    "manuscript",
                    blocking,
                    structure.is_some_and(|report| report.abstract_present),
                    "系统只核验摘要是否存在；结构、长度仍应按官方原文人工核对".to_owned(),
                    requirement,
                    snapshot,
                ),
                JournalRequirementCategory::Keywords => automatic_submission_checklist_item(
                    &id,
                    &requirement.label,
                    "manuscript",
                    blocking,
                    structure.is_some_and(|report| report.keywords_present),
                    "系统只核验关键词是否存在；数量和格式仍应按官方原文核对".to_owned(),
                    requirement,
                    snapshot,
                ),
                JournalRequirementCategory::References => confirmable_submission_checklist_item(
                    &id,
                    &requirement.label,
                    "manuscript",
                    blocking,
                    confirmed(&id),
                    if structure.is_some_and(|report| report.references_present) {
                        "已检测到参考文献；请确认引用样式与该刊要求一致"
                    } else {
                        "未可靠检测到参考文献；请检查并确认引用文件与样式"
                    }
                    .to_owned(),
                    "manual",
                    requirement,
                    snapshot,
                ),
                JournalRequirementCategory::Ethics
                | JournalRequirementCategory::ConflictOfInterest
                | JournalRequirementCategory::DataAvailability
                | JournalRequirementCategory::AuthorContributions
                | JournalRequirementCategory::Orcid => confirmable_submission_checklist_item(
                    &id,
                    &requirement.label,
                    "declarations",
                    blocking,
                    confirmed(&id),
                    "请由作者确认内容真实、完整且适用于当前论文；系统不替作者作事实声明".to_owned(),
                    "author",
                    requirement,
                    snapshot,
                ),
                JournalRequirementCategory::ManuscriptFile
                | JournalRequirementCategory::Template
                | JournalRequirementCategory::LengthLimit
                | JournalRequirementCategory::Figures
                | JournalRequirementCategory::Tables
                | JournalRequirementCategory::FeesAndOpenAccess => {
                    confirmable_submission_checklist_item(
                        &id,
                        &requirement.label,
                        if requirement.category == JournalRequirementCategory::FeesAndOpenAccess {
                            "declarations"
                        } else {
                            "manuscript"
                        },
                        blocking,
                        confirmed(&id),
                        "请对照证据原文人工核验；确认也可表示该条件对本文不适用".to_owned(),
                        "manual",
                        requirement,
                        snapshot,
                    )
                }
            };
            checklist.push(item);
        }
    }
    if figures_expected
        && !checklist
            .iter()
            .any(|item| item.material_kind == Some(SubmissionMaterialKind::Figure))
    {
        checklist.push(file_submission_checklist_item(
            "figure-originals",
            "原始图件",
            "files",
            false,
            matching_material_ids("figure-originals", SubmissionMaterialKind::Figure),
            structure.map_or(1, |report| report.figure_count.max(1) as usize),
            SubmissionMaterialKind::Figure,
            "正文包含图片；取得目标期刊要求后再确认文件格式和精度".to_owned(),
            None,
            journal_requirements,
        ));
    }
    if tables_expected
        && !checklist
            .iter()
            .any(|item| item.material_kind == Some(SubmissionMaterialKind::Table))
    {
        checklist.push(file_submission_checklist_item(
            "table-editables",
            "可编辑表格",
            "files",
            false,
            matching_material_ids("table-editables", SubmissionMaterialKind::Table),
            structure.map_or(1, |report| report.table_count.max(1) as usize),
            SubmissionMaterialKind::Table,
            "正文包含表格；请提供 CSV、Excel、Word 或 LaTeX 等可编辑文件，图片版表格不能替代源数据"
                .to_owned(),
            None,
            journal_requirements,
        ));
    }
    let required_total = checklist.iter().filter(|item| item.blocking).count();
    let required_completed = checklist
        .iter()
        .filter(|item| item.blocking && item.status == "passed")
        .count();
    let required_complete = required_completed == required_total;
    let target_verified = target_current && requirements_current;
    let target_check_ready = required_complete && target_verified && readiness.is_some();
    let workflow_status = if target_check_ready {
        "submission_ready"
    } else if target_verified {
        "target_verified"
    } else if recommendation_ready {
        "preliminary_recommendation"
    } else {
        "manuscript_received"
    };
    SubmissionMaterialCatalog {
        schema_version: 3,
        workspace_id: workspace.id.clone(),
        manuscript_version: workspace.snapshot_version,
        materials: stored
            .materials
            .into_iter()
            .map(|item| item.material)
            .collect(),
        checklist,
        recommendation_ready,
        target_verified,
        required_complete,
        target_check_ready,
        workflow_status: workflow_status.to_owned(),
        required_total,
        required_completed,
    }
}

fn static_submission_checklist_item(
    id: &str,
    label: &str,
    group: &str,
    status: &str,
    detail: String,
) -> SubmissionMaterialChecklistItem {
    SubmissionMaterialChecklistItem {
        id: id.to_owned(),
        label: label.to_owned(),
        label_en: match id {
            "target-journal" => "Target journal and article type",
            "official-journal-requirements" => "Official journal requirements",
            "main-manuscript" => "Current manuscript",
            "current-check" => "Target-specific recheck",
            _ => label,
        }
        .to_owned(),
        group: group.to_owned(),
        requirement: "required".to_owned(),
        status: status.to_owned(),
        detail,
        verification: "automatic".to_owned(),
        material_kind: None,
        blocking: true,
        confirmable: false,
        source_url: None,
        evidence_excerpt: None,
        captured_unix_ms: None,
        fresh_until_unix_ms: None,
        required_count: 0,
        matched_material_ids: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn file_submission_checklist_item(
    id: &str,
    label: &str,
    group: &str,
    blocking: bool,
    matched_material_ids: Vec<String>,
    required_count: usize,
    material_kind: SubmissionMaterialKind,
    detail: String,
    requirement: Option<&crate::JournalRequirementItem>,
    snapshot: Option<&JournalRequirementSnapshot>,
) -> SubmissionMaterialChecklistItem {
    let present = matched_material_ids.len() >= required_count;
    let mut item = submission_checklist_item(
        id,
        label,
        group,
        if blocking { "required" } else { "recommended" },
        if present {
            "passed"
        } else if blocking {
            "missing"
        } else {
            "recommended"
        },
        detail,
        "file",
        Some(material_kind),
        blocking,
        false,
        requirement,
        snapshot,
    );
    item.required_count = required_count;
    item.matched_material_ids = matched_material_ids;
    item
}

#[allow(clippy::too_many_arguments)]
fn automatic_submission_checklist_item(
    id: &str,
    label: &str,
    group: &str,
    blocking: bool,
    passed: bool,
    detail: String,
    requirement: &crate::JournalRequirementItem,
    snapshot: &JournalRequirementSnapshot,
) -> SubmissionMaterialChecklistItem {
    submission_checklist_item(
        id,
        label,
        group,
        if blocking { "required" } else { "recommended" },
        if passed {
            "passed"
        } else if blocking {
            "missing"
        } else {
            "recommended"
        },
        detail,
        "automatic",
        None,
        blocking,
        false,
        Some(requirement),
        Some(snapshot),
    )
}

#[allow(clippy::too_many_arguments)]
fn confirmable_submission_checklist_item(
    id: &str,
    label: &str,
    group: &str,
    blocking: bool,
    confirmed: bool,
    detail: String,
    verification: &str,
    requirement: &crate::JournalRequirementItem,
    snapshot: &JournalRequirementSnapshot,
) -> SubmissionMaterialChecklistItem {
    submission_checklist_item(
        id,
        label,
        group,
        if blocking { "required" } else { "recommended" },
        if confirmed {
            "passed"
        } else if !blocking {
            "recommended"
        } else if verification == "author" {
            "author_confirmation"
        } else {
            "manual_verification"
        },
        detail,
        verification,
        None,
        blocking,
        blocking,
        Some(requirement),
        Some(snapshot),
    )
}

#[allow(clippy::too_many_arguments)]
fn submission_checklist_item(
    id: &str,
    label: &str,
    group: &str,
    requirement: &str,
    status: &str,
    detail: String,
    verification: &str,
    material_kind: Option<SubmissionMaterialKind>,
    blocking: bool,
    confirmable: bool,
    source: Option<&crate::JournalRequirementItem>,
    snapshot: Option<&JournalRequirementSnapshot>,
) -> SubmissionMaterialChecklistItem {
    SubmissionMaterialChecklistItem {
        id: id.to_owned(),
        label: label.to_owned(),
        label_en: source
            .map(|item| item.label_en.clone())
            .unwrap_or_else(|| label.to_owned()),
        group: group.to_owned(),
        requirement: requirement.to_owned(),
        status: status.to_owned(),
        detail,
        verification: verification.to_owned(),
        material_kind,
        blocking,
        confirmable,
        source_url: source.map(|item| item.source_url.clone()),
        evidence_excerpt: source.map(|item| item.evidence_excerpt.clone()),
        captured_unix_ms: snapshot.map(|item| item.captured_unix_ms),
        fresh_until_unix_ms: snapshot.map(|item| item.fresh_until_unix_ms),
        required_count: 0,
        matched_material_ids: Vec::new(),
    }
}

fn article_type_label(article_type: ArticleTypePreference) -> &'static str {
    match article_type {
        ArticleTypePreference::Auto => "待确认",
        ArticleTypePreference::Research => "研究论文",
        ArticleTypePreference::Review => "综述",
        ArticleTypePreference::Application => "应用型论文",
    }
}

fn verify_file_hash(path: &Path, expected_hash: &str) -> Result<(), WorkspaceError> {
    let mut reader = BufReader::new(File::open(path)?);
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
        return Err(WorkspaceError::InvalidSubmissionMaterial(
            "已保存材料的内容指纹不一致".to_owned(),
        ));
    }
    Ok(())
}

fn write_or_replace_json(path: &Path, value: &impl Serialize) -> Result<(), WorkspaceError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        replace_json(path, value)
    } else {
        write_json(path, value)
    }
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

fn read_journal_recommendation_run(
    path: &Path,
) -> Result<JournalRecommendationRun, WorkspaceError> {
    let mut value: serde_json::Value = read_json(path)?;
    if value.get("domestic").is_none() || value.get("international").is_none() {
        let object = value.as_object_mut().ok_or_else(|| {
            WorkspaceError::InvalidManifest("期刊推荐记录不是 JSON 对象".to_owned())
        })?;
        let mut domestic = serde_json::Map::new();
        let mut international = serde_json::Map::new();
        for group in ["sprint", "matching", "safeguard"] {
            let entries = object
                .remove(group)
                .and_then(|value| value.as_array().cloned())
                .ok_or_else(|| {
                    WorkspaceError::InvalidManifest("期刊推荐记录缺少分组".to_owned())
                })?;
            let mut domestic_entries = Vec::new();
            let mut international_entries = Vec::new();
            for entry in entries {
                match entry.get("region").and_then(serde_json::Value::as_str) {
                    Some("domestic") => domestic_entries.push(entry),
                    Some("international") => international_entries.push(entry),
                    _ => {
                        return Err(WorkspaceError::InvalidManifest(
                            "期刊推荐记录包含未知地区".to_owned(),
                        ))
                    }
                }
            }
            domestic.insert(group.to_owned(), serde_json::Value::Array(domestic_entries));
            international.insert(
                group.to_owned(),
                serde_json::Value::Array(international_entries),
            );
        }
        object.insert("domestic".to_owned(), serde_json::Value::Object(domestic));
        object.insert(
            "international".to_owned(),
            serde_json::Value::Object(international),
        );
    }
    serde_json::from_value(value)
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
        make_tree_writable, read_json, read_stored_submission_materials, write_json,
        DecompositionManifest, SubmissionTargetSelection, VersionCreation, VersionOrigin,
        WorkspaceError, WorkspaceStore,
    };
    use crate::{
        ElementState, InstitutionRuleEvidence, InstitutionRuleStatus, JournalMatchPreferences,
        JournalProfileDiscoveryRecord, JournalRecommendationProfileInput,
        JournalRequirementSourceDocument, JournalRequirementSourceMode, KnowledgeBodyError,
        KnowledgeCandidateDecision, KnowledgeInquiryStance, KnowledgeInquiryTarget,
        ManuscriptPurpose, ReadinessOutcome, RevisionApplication, RevisionChangeInput,
        RevisionFieldKind, SubmissionMaterialKind, JOURNAL_PROFILE_DISCOVERY_SCHEMA_VERSION,
    };
    use std::{
        fs::{self, File},
        io::Write,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };
    use zip::{write::SimpleFileOptions, ZipWriter};

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

    fn write_synthetic_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut archive = ZipWriter::new(File::create(path).unwrap());
        for (name, content) in entries {
            archive
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            archive.write_all(content).unwrap();
        }
        archive.finish().unwrap();
    }

    fn select_synthetic_target(
        store: &WorkspaceStore,
        workspace_id: &str,
    ) -> (SubmissionTargetSelection, String, String, String) {
        let profile = store
            .save_journal_recommendation_profile(
                workspace_id,
                JournalRecommendationProfileInput {
                    author_name: "Synthetic Author".into(),
                    institution: "Synthetic University".into(),
                    specialty: "Computer vision".into(),
                    manuscript_purpose: ManuscriptPurpose::DegreeRequirement,
                    submission_deadline: "2099-12-31".into(),
                },
            )
            .unwrap();
        let run = store
            .recommend_journals(
                workspace_id,
                &profile.profile_id,
                JournalMatchPreferences::default(),
            )
            .unwrap();
        let domestic = run
            .domestic
            .sprint
            .iter()
            .chain(run.domestic.matching.iter())
            .chain(run.domestic.safeguard.iter())
            .collect::<Vec<_>>();
        let journal_id = domestic.first().unwrap().id.clone();
        let backup_journal_id = domestic
            .iter()
            .find(|journal| journal.id != journal_id)
            .unwrap()
            .id
            .clone();
        let target = store
            .select_recommended_journal(workspace_id, &run.run_id, &journal_id)
            .unwrap();
        (target, run.run_id, journal_id, backup_journal_id)
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
        assert_eq!(
            store
                .journal_recommendation_author_names(&workspace.id)
                .unwrap(),
            vec!["Synthetic Author"]
        );
        let run = store
            .recommend_journals(
                &workspace.id,
                &profile.profile_id,
                JournalMatchPreferences::default(),
            )
            .unwrap();

        assert_eq!(profile, same_profile);
        assert_eq!(run.recommendation_profile.profile_id, profile.profile_id);
        assert!(run.domestic.sprint.len() <= 2);
        assert!(!run.domestic.matching.is_empty());
        assert!(run.domestic.matching.len() <= 3);
        assert!(run.domestic.safeguard.len() <= 3);
        assert!(run.international.sprint.len() <= 2);
        assert!(!run.international.matching.is_empty());
        assert!(run.international.matching.len() <= 3);
        assert!(run.international.safeguard.len() <= 3);
        assert!(run.school_rule_status.contains("search_required"));
        let recommended = run
            .domestic
            .sprint
            .iter()
            .chain(run.domestic.matching.iter())
            .chain(run.domestic.safeguard.iter())
            .next()
            .unwrap();
        let target = store
            .select_recommended_journal(&workspace.id, &run.run_id, &recommended.id)
            .unwrap();
        assert_eq!(target.journal_id, recommended.id);
        assert_eq!(
            store.submission_target(&workspace.id).unwrap(),
            Some(target.clone())
        );
        let discovery = JournalProfileDiscoveryRecord {
            schema_version: JOURNAL_PROFILE_DISCOVERY_SCHEMA_VERSION,
            discovery_id: "jed-0123456789abcdefabcd".into(),
            workspace_id: workspace.id.clone(),
            target_selection_id: target.selection_id.clone(),
            journal_id: target.journal_id.clone(),
            journal_name: target.name.clone(),
            issn: Some("1234-5678".into()),
            eissn: Some("8765-4321".into()),
            publisher: Some(target.publisher.clone()),
            scope_summary: Some("Synthetic scope discovered from the local directory.".into()),
            reported_print_circulation: None,
            average_review_days: None,
            submission_to_publication_days: Some(120.0),
            publication_frequency: Some("monthly".into()),
            apc_status: Some("no_apc".into()),
            open_access_status: Some("hybrid".into()),
            official_homepage_url: Some(target.homepage_url.clone()),
            aims_scope_url: None,
            author_instructions_url: None,
            source_urls: vec![target.homepage_url.clone()],
            missing_fields: vec![
                "reported_print_circulation".into(),
                "average_review_days".into(),
            ],
            evidence_status: "local_profile_available".into(),
            source_mode: "local_directory".into(),
            provider_label: None,
            model: None,
            external_transmission: "not_performed".into(),
            created_unix_ms: 10,
        };
        store
            .save_journal_profile_discovery(&workspace.id, &discovery)
            .unwrap();
        assert_eq!(
            store.journal_profile_discoveries(&workspace.id).unwrap(),
            vec![discovery]
        );
        let backup_candidate = run
            .domestic
            .sprint
            .iter()
            .chain(run.domestic.matching.iter())
            .chain(run.domestic.safeguard.iter())
            .find(|candidate| candidate.id != recommended.id)
            .unwrap();
        let plan = store
            .add_backup_recommended_journal(&workspace.id, &run.run_id, &backup_candidate.id)
            .unwrap();
        assert_eq!(
            plan.primary.as_ref().unwrap().selection_id,
            target.selection_id
        );
        assert_eq!(plan.backups.len(), 1);
        let backup_selection_id = plan.backups[0].selection_id.clone();
        let plan = store
            .remove_backup_target(&workspace.id, &backup_selection_id)
            .unwrap();
        assert!(plan.backups.is_empty());
        let plan = store
            .add_backup_recommended_journal(&workspace.id, &run.run_id, &backup_candidate.id)
            .unwrap();
        assert_eq!(plan.backups.len(), 1);
        let audit = fs::read_to_string(
            store
                .projects_root()
                .join(&workspace.id)
                .join("audit.jsonl"),
        )
        .unwrap();
        assert!(audit.contains("submission_backup_removed"));
        let requirement_snapshot = store
            .save_journal_requirement_snapshot(
                &workspace.id,
                &target.selection_id,
                &[JournalRequirementSourceDocument {
                    url: "https://journal.example/guide-for-authors".to_owned(),
                    title: "Guide for authors".to_owned(),
                    text: "A separate title page is required. A blinded manuscript is required for double-blind review. Figures must be supplied at 300 dpi. A cover letter is recommended.".to_owned(),
                    official_host_matched: true,
                }],
                JournalRequirementSourceMode::AuthorProvidedOfficialText,
                true,
                "not_performed",
            )
            .unwrap();
        assert_eq!(requirement_snapshot.requirements.len(), 4);
        assert_eq!(
            store
                .journal_requirement_snapshots(&workspace.id)
                .unwrap()
                .len(),
            1
        );
        let source_project = temporary.path().join("source-project.zip");
        write_synthetic_zip(&source_project, &[("manuscript.tex", b"Synthetic source")]);
        let materials = store
            .add_submission_materials(
                &workspace.id,
                SubmissionMaterialKind::SourceProject,
                std::slice::from_ref(&source_project),
            )
            .unwrap();
        assert!(!materials.required_complete);
        assert!(!materials.target_check_ready);
        assert!(materials
            .checklist
            .iter()
            .any(|item| item.id == "latex-project" && item.status == "passed"));
        assert_eq!(materials.materials.len(), 1);
        let target_exports = temporary.path().join("target-exports");
        fs::create_dir(&target_exports).unwrap();
        assert!(matches!(
            store.export_target_submission_package(&workspace.id, &target_exports),
            Err(WorkspaceError::InvalidSubmissionMaterial(_))
        ));
        let title_page = temporary.path().join("title-page.docx");
        write_synthetic_zip(
            &title_page,
            &[
                ("[Content_Types].xml", b"<Types />"),
                (
                    "word/document.xml",
                    b"<document>synthetic title page</document>",
                ),
            ],
        );
        store
            .add_submission_materials(
                &workspace.id,
                SubmissionMaterialKind::TitlePage,
                std::slice::from_ref(&title_page),
            )
            .unwrap();
        let blinded_manuscript = temporary.path().join("blinded-manuscript.tex");
        fs::write(&blinded_manuscript, b"Synthetic blinded manuscript").unwrap();
        store
            .add_submission_materials(
                &workspace.id,
                SubmissionMaterialKind::BlindedManuscript,
                std::slice::from_ref(&blinded_manuscript),
            )
            .unwrap();
        store
            .confirm_submission_requirement(&workspace.id, "journal-figures", true)
            .unwrap();
        store.evaluate_readiness(&workspace.id, &[]).unwrap();
        let materials = store.submission_materials(&workspace.id).unwrap();
        assert!(materials.required_complete);
        assert!(materials.target_check_ready);
        let package_plan = store.target_submission_package_plan(&workspace.id).unwrap();
        assert!(package_plan.ready);
        assert!(package_plan.anonymous_review);
        assert!(package_plan
            .files
            .iter()
            .all(|file| file.content_hash.len() == 64));
        let title_page_id = materials
            .materials
            .iter()
            .find(|material| material.kind == SubmissionMaterialKind::TitlePage)
            .unwrap()
            .material_id
            .clone();
        store
            .set_submission_material_included(&workspace.id, &title_page_id, false)
            .unwrap();
        assert!(
            !store
                .target_submission_package_plan(&workspace.id)
                .unwrap()
                .ready
        );
        store
            .set_submission_material_included(&workspace.id, &title_page_id, true)
            .unwrap();
        assert!(matches!(
            store.delete_submission_material(&workspace.id, &title_page_id, false),
            Err(WorkspaceError::AuthorConfirmationRequired)
        ));
        let stored_before_delete =
            read_stored_submission_materials(&store.projects_root().join(&workspace.id)).unwrap();
        let title_page_path = store.projects_root().join(&workspace.id).join(
            &stored_before_delete
                .materials
                .iter()
                .find(|item| item.material.material_id == title_page_id)
                .unwrap()
                .relative_path,
        );
        assert!(title_page_path.is_file());
        let materials = store
            .delete_submission_material(&workspace.id, &title_page_id, true)
            .unwrap();
        assert!(!title_page_path.exists());
        assert!(materials
            .materials
            .iter()
            .all(|material| material.material_id != title_page_id));
        assert!(!materials.required_complete);
        assert!(
            !store
                .target_submission_package_plan(&workspace.id)
                .unwrap()
                .ready
        );
        let audit = fs::read_to_string(
            store
                .projects_root()
                .join(&workspace.id)
                .join("audit.jsonl"),
        )
        .unwrap();
        assert!(audit.contains("submission_material_deleted"));
        let materials = store
            .add_submission_materials(
                &workspace.id,
                SubmissionMaterialKind::TitlePage,
                std::slice::from_ref(&title_page),
            )
            .unwrap();
        assert!(materials.required_complete);
        assert!(
            store
                .target_submission_package_plan(&workspace.id)
                .unwrap()
                .ready
        );
        let target_export = store
            .export_target_submission_package(&workspace.id, &target_exports)
            .unwrap();
        let target_root = target_exports.join(&target_export.package_name);
        assert!(!target_root.join("submission/manuscript.tex").exists());
        assert!(target_root
            .join("submission/manuscript-blinded.tex")
            .is_file());
        assert!(target_root
            .join("submission/source-project/source-project.zip")
            .is_file());
        assert!(target_root
            .join("submission/title-page/title-page.docx")
            .is_file());
        assert!(target_root.join("records/target-selection.json").is_file());
        assert!(target_root
            .join("records/journal-requirements.json")
            .is_file());
        assert!(target_root.join("records/package-manifest.json").is_file());
        assert!(target_root.join("README.txt").is_file());
        let backup = plan.backups.first().unwrap();
        store
            .save_journal_requirement_snapshot(
                &workspace.id,
                &backup.selection_id,
                &[JournalRequirementSourceDocument {
                    url: "https://backup.example/author-guidelines".to_owned(),
                    title: "Backup guide".to_owned(),
                    text: "A cover letter is required and figures must be supplied separately."
                        .to_owned(),
                    official_host_matched: true,
                }],
                JournalRequirementSourceMode::AuthorProvidedOfficialText,
                true,
                "not_performed",
            )
            .unwrap();
        let promoted = store
            .promote_backup_target(&workspace.id, &backup.selection_id, "rejected")
            .unwrap();
        assert_eq!(
            promoted.primary.as_ref().unwrap().journal_id,
            backup.journal_id
        );
        assert!(promoted.backups.is_empty());
        let inherited = store
            .journal_requirement_snapshot(
                &workspace.id,
                &promoted.primary.as_ref().unwrap().selection_id,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            inherited.target_selection_id,
            promoted.primary.as_ref().unwrap().selection_id
        );
        assert!(inherited
            .limitations
            .iter()
            .any(|item| item.contains("备选支线继承")));
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
        let recovered_runs = WorkspaceStore::new(&store_root)
            .journal_recommendation_runs(&workspace.id)
            .unwrap();
        assert_eq!(recovered_runs, vec![run.clone()]);
        assert!(recovered_runs[0]
            .domestic
            .sprint
            .iter()
            .chain(recovered_runs[0].domestic.matching.iter())
            .chain(recovered_runs[0].domestic.safeguard.iter())
            .chain(recovered_runs[0].international.sprint.iter())
            .chain(recovered_runs[0].international.matching.iter())
            .chain(recovered_runs[0].international.safeguard.iter())
            .all(|recommendation| !recommendation.publisher.trim().is_empty()));
        let run_path = analysis_root.join(format!("journal-match-{}.json", run.run_id));
        let mut legacy_value = serde_json::to_value(&run).unwrap();
        let legacy_object = legacy_value.as_object_mut().unwrap();
        let domestic_value = legacy_object.remove("domestic").unwrap();
        let international_value = legacy_object.remove("international").unwrap();
        for group in ["sprint", "matching", "safeguard"] {
            let mut combined = domestic_value[group].as_array().unwrap().clone();
            combined.extend(international_value[group].as_array().unwrap().clone());
            legacy_object.insert(group.to_owned(), serde_json::Value::Array(combined));
        }
        legacy_object.insert("schemaVersion".to_owned(), serde_json::Value::from(4));
        write_json(&run_path, &legacy_value).unwrap();
        let migrated_runs = WorkspaceStore::new(&store_root)
            .journal_recommendation_runs(&workspace.id)
            .unwrap();
        assert_eq!(migrated_runs[0].run_id, run.run_id);
        assert_eq!(migrated_runs[0].domestic, run.domestic);
        assert_eq!(migrated_runs[0].international, run.international);
        let copies_root = temporary.path().join("workspace-copies");
        fs::create_dir(&copies_root).unwrap();
        let exported = store
            .export_workspace_copy(&workspace.id, false, &copies_root)
            .unwrap();
        assert!(exported.file_count > 3);
        assert!(copies_root
            .join(&exported.folder_name)
            .join("analysis")
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
        assert!(audit.contains("workspace_copy_exported"));

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
            .sprint
            .iter()
            .chain(evidence_run.international.matching.iter())
            .chain(evidence_run.international.safeguard.iter())
            .any(|item| item.scores.institution_rules == Some(100)));
    }

    #[test]
    fn clears_only_the_active_primary_pointer_and_preserves_target_history_and_backups() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("clear-primary-target.tex");
        fs::write(
            &source_path,
            r"\title{Primary Target Cancellation}
\author{Synthetic Author}
\begin{abstract}Computer vision evidence.\end{abstract}
\keywords{computer vision}
\section{Introduction}
\section{Methods}
\section{Results}
\bibliography{synthetic}",
        )
        .unwrap();
        let store = WorkspaceStore::new(temporary.path().join("store"));
        let workspace = store.create_from_source(&source_path).unwrap();
        store.analyze_structure(&workspace.id).unwrap();
        let (target, recommendation_run_id, _, backup_journal_id) =
            select_synthetic_target(&store, &workspace.id);
        let plan = store
            .add_backup_recommended_journal(
                &workspace.id,
                &recommendation_run_id,
                &backup_journal_id,
            )
            .unwrap();
        assert_eq!(plan.backups.len(), 1);
        assert!(matches!(
            store.clear_primary_submission_target(&workspace.id, &target.selection_id, false),
            Err(WorkspaceError::AuthorConfirmationRequired)
        ));

        let cleared = store
            .clear_primary_submission_target(&workspace.id, &target.selection_id, true)
            .unwrap();

        assert!(cleared.primary.is_none());
        assert_eq!(cleared.backups.len(), 1);
        assert!(store.submission_target(&workspace.id).unwrap().is_none());
        let workspace_root = store.projects_root().join(&workspace.id);
        assert!(workspace_root
            .join("targets")
            .join(&target.selection_id)
            .join("target.json")
            .is_file());
        assert!(!workspace_root.join("targets").join("current.json").exists());
        assert!(fs::read_to_string(workspace_root.join("audit.jsonl"))
            .unwrap()
            .contains("submission_primary_target_cleared"));
        let next_selection_id = cleared.backups[0].selection_id.clone();
        let promoted = store
            .promote_backup_target(&workspace.id, &next_selection_id, "not_submitted")
            .unwrap();
        assert!(promoted.primary.is_some());
        assert!(promoted.backups.is_empty());
        assert_eq!(
            store
                .submission_target(&workspace.id)
                .unwrap()
                .unwrap()
                .journal_id,
            promoted.primary.unwrap().journal_id
        );
    }

    #[test]
    fn separates_figure_and_table_formats_and_accepts_utf16_text() {
        let temporary = SyntheticDirectory::create();
        let table_path = temporary.path().join("Table1.csv");
        let mut utf16_csv = vec![0xff, 0xfe];
        for unit in "variable,value\r\nsynthetic,1\r\n".encode_utf16() {
            utf16_csv.extend_from_slice(&unit.to_le_bytes());
        }
        fs::write(&table_path, utf16_csv).unwrap();

        assert!(super::is_allowed_submission_material_kind_extension(
            SubmissionMaterialKind::Table,
            "csv"
        ));
        assert!(!super::is_allowed_submission_material_kind_extension(
            SubmissionMaterialKind::Figure,
            "csv"
        ));
        let validation =
            super::validate_submission_material(&table_path, "csv", SubmissionMaterialKind::Table)
                .unwrap();
        assert_eq!(validation.status, "warning");
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.contains("UTF-16")));
        assert!(matches!(
            super::validate_submission_material(
                &table_path,
                "csv",
                SubmissionMaterialKind::Figure
            ),
            Err(WorkspaceError::InvalidSubmissionMaterial(message))
                if message.contains("可编辑表格")
        ));

        let disguised_excel_path = temporary.path().join("disguised.csv");
        fs::write(
            &disguised_excel_path,
            b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1synthetic workbook",
        )
        .unwrap();
        let disguised_validation = super::validate_submission_material(
            &disguised_excel_path,
            "csv",
            SubmissionMaterialKind::Table,
        )
        .unwrap();
        assert_eq!(disguised_validation.status, "blocked");
        assert!(disguised_validation
            .issues
            .iter()
            .any(|issue| issue.contains("实际是旧版 Microsoft Excel（.xls）")));
    }

    #[test]
    fn preserves_detected_figure_and_table_upload_slots_with_official_requirements() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("figure-table-study.tex");
        fs::write(
            &source_path,
            r"\title{Figure and Table Study}
\begin{abstract}Synthetic evidence.\end{abstract}
\keywords{computer vision}
\section{Introduction}
\begin{figure}\caption{Synthetic figure}\end{figure}
\begin{table}\caption{Synthetic table}\end{table}
\bibliography{synthetic}",
        )
        .unwrap();
        let store = WorkspaceStore::new(temporary.path().join("store"));
        let workspace = store.create_from_source(&source_path).unwrap();
        let structure = store.analyze_structure(&workspace.id).unwrap();
        assert_eq!(structure.figure_count, 1);
        assert_eq!(structure.table_count, 1);
        let (target, _, _, _) = select_synthetic_target(&store, &workspace.id);
        store
            .save_journal_requirement_snapshot(
                &workspace.id,
                &target.selection_id,
                &[JournalRequirementSourceDocument {
                    url: "https://journal.example/guide-for-authors".to_owned(),
                    title: "Guide for authors".to_owned(),
                    text: "A separate title page is required.".to_owned(),
                    official_host_matched: true,
                }],
                JournalRequirementSourceMode::AuthorProvidedOfficialText,
                true,
                "not_performed",
            )
            .unwrap();

        let catalog = store.submission_materials(&workspace.id).unwrap();
        assert!(catalog.checklist.iter().any(|item| {
            item.id == "figure-originals"
                && item.verification == "file"
                && item.material_kind == Some(SubmissionMaterialKind::Figure)
        }));
        assert!(catalog.checklist.iter().any(|item| {
            item.id == "table-editables"
                && item.verification == "file"
                && item.material_kind == Some(SubmissionMaterialKind::Table)
        }));
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
\begin{abstract}We propose a deterministic method. Results show improved extraction accuracy by 18 percent.\end{abstract}
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
        assert!(persisted_report.contains("knowledge_body_candidates"));
        assert!(persisted_report.contains("submission_readiness_inputs"));
        assert!(persisted_report.contains("manifestHash"));
        assert!(persisted_report.contains("sourceFragments"));
        assert!(persisted_report.contains("sourceFragmentId"));
        assert!(audit.contains("manuscript_decomposed"));
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
        let audit_after_readiness = fs::read_to_string(
            store_root
                .join("projects")
                .join(&workspace.id)
                .join("audit.jsonl"),
        )
        .unwrap();
        assert_eq!(
            audit_after_readiness
                .matches("manuscript_decomposed")
                .count(),
            1
        );
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

        let (target, recommendation_run_id, journal_id, backup_journal_id) =
            select_synthetic_target(&store, &workspace.id);
        assert!(store
            .lifecycle(&workspace.id)
            .unwrap()
            .readiness_report
            .is_none());
        store.evaluate_readiness(&workspace.id, &[]).unwrap();
        let attestation = store.create_local_attestation(&workspace.id, true).unwrap();
        let export_root = temporary.path().join("exports");
        fs::create_dir(&export_root).unwrap();
        let export = store
            .export_submission_package(&workspace.id, &export_root)
            .unwrap();
        let package_root = export_root.join(&export.package_name);
        assert!(package_root.join("manuscript.tex").is_file());
        assert!(package_root.join("decomposition-manifest.json").is_file());
        assert!(package_root.join("readiness-report.json").is_file());
        assert!(package_root.join("readiness-preview.html").is_file());
        assert!(package_root.join("local-attestation.json").is_file());
        assert!(package_root.join("submission-manifest.json").is_file());

        let submission = store
            .record_manual_submission(&workspace.id, &target.name, Some("SYN-2026-001"), true)
            .unwrap();
        assert_eq!(submission.schema_version, 2);
        assert_eq!(
            submission.target_selection_id.as_deref(),
            Some(target.selection_id.as_str())
        );
        assert_eq!(
            submission.publisher.as_deref(),
            Some(target.publisher.as_str())
        );
        let candidate_decisions = store
            .knowledge_body_snapshot(&workspace.id)
            .unwrap()
            .extraction
            .unwrap()
            .all_candidates()
            .map(|candidate| KnowledgeCandidateDecision {
                candidate_id: candidate.candidate_id.clone(),
                included: true,
            })
            .collect::<Vec<_>>();
        assert!(!candidate_decisions.is_empty());
        assert!(matches!(
            store.finalize_knowledge_body(
                &workspace.id,
                "computer_information_sciences",
                &candidate_decisions,
                false,
            ),
            Err(WorkspaceError::AuthorConfirmationRequired)
        ));
        assert!(matches!(
            store.finalize_knowledge_body(
                &workspace.id,
                "computer_information_sciences",
                &candidate_decisions[..candidate_decisions.len() - 1],
                true,
            ),
            Err(WorkspaceError::Knowledge(
                KnowledgeBodyError::InvalidCandidateReview
            ))
        ));
        assert!(matches!(
            store.finalize_knowledge_body(
                &workspace.id,
                "unknown-discipline",
                &candidate_decisions,
                true,
            ),
            Err(WorkspaceError::InvalidDisciplineClassification)
        ));
        let knowledge = store
            .finalize_knowledge_body(
                &workspace.id,
                "computer_information_sciences",
                &candidate_decisions,
                true,
            )
            .unwrap();
        assert_eq!(knowledge.attestation_id, attestation.attestation_id);
        assert_eq!(knowledge.submission_id, submission.submission_id);
        assert_eq!(knowledge.snapshot.manuscript.version, 1);
        let extraction = knowledge.snapshot.extraction.as_ref().unwrap();
        for element in [
            &extraction.claim,
            &extraction.scope,
            &extraction.method,
            &extraction.result,
            &extraction.evidence,
        ] {
            if !element.candidates.is_empty() {
                assert_eq!(element.state, ElementState::Established);
                assert!(element
                    .candidates
                    .iter()
                    .all(|candidate| candidate.author_confirmed));
            }
        }
        let exported_decomposition: DecompositionManifest =
            read_json(&package_root.join("decomposition-manifest.json")).unwrap();
        let exported_package_manifest: serde_json::Value =
            read_json(&package_root.join("submission-manifest.json")).unwrap();
        assert_eq!(
            extraction.decomposition_id,
            exported_decomposition.decomposition_id
        );
        assert_eq!(
            extraction.decomposition_hash,
            exported_decomposition.manifest_hash
        );
        assert_eq!(
            exported_package_manifest["decompositionHash"],
            exported_decomposition.manifest_hash
        );
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
            .finalize_knowledge_body(
                &workspace.id,
                "engineering_technology",
                &candidate_decisions,
                true,
            )
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

        let backup_plan = store
            .add_backup_recommended_journal(
                &workspace.id,
                &recommendation_run_id,
                &backup_journal_id,
            )
            .unwrap();
        let backup_selection_id = backup_plan.backups[0].selection_id.clone();
        store
            .promote_backup_target(&workspace.id, &backup_selection_id, "rejected")
            .unwrap();
        let rerouted = store.lifecycle(&workspace.id).unwrap();
        assert!(rerouted.readiness_report.is_none());
        assert!(rerouted.attestation.is_none());
        assert!(rerouted.submission.is_none());
        assert!(rerouted.knowledge_body.is_none());

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
        assert!(matches!(
            store.select_recommended_journal(&workspace.id, &recommendation_run_id, &journal_id),
            Err(WorkspaceError::StaleRecommendationRun)
        ));
        let new_head = store.lifecycle(&workspace.id).unwrap();
        assert_eq!(new_head.current_version, 2);
        assert!(new_head.structure_report.is_none());
        assert!(new_head.readiness_report.is_none());
        assert!(new_head.attestation.is_none());
        assert!(new_head.submission.is_none());
        assert!(new_head.knowledge_body.is_none());
    }

    #[test]
    fn recording_a_real_submission_creates_the_local_attestation_when_needed() {
        let temporary = SyntheticDirectory::create();
        let source_path = temporary.path().join("automatic-attestation.tex");
        fs::write(
            &source_path,
            r"\title{Automatic Attestation}
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
        let store = WorkspaceStore::new(temporary.path().join("store"));
        let workspace = store.create_from_source(&source_path).unwrap();
        store.analyze_structure(&workspace.id).unwrap();
        let (target, _, _, _) = select_synthetic_target(&store, &workspace.id);
        store.evaluate_readiness(&workspace.id, &[]).unwrap();

        assert!(store
            .lifecycle(&workspace.id)
            .unwrap()
            .attestation
            .is_none());
        let submission = store
            .record_manual_submission(&workspace.id, &target.name, None, true)
            .unwrap();
        let lifecycle = store.lifecycle(&workspace.id).unwrap();

        assert_eq!(
            lifecycle
                .attestation
                .as_ref()
                .map(|record| record.attestation_id.as_str()),
            Some(submission.attestation_id.as_str())
        );
        assert_eq!(
            lifecycle
                .submission
                .as_ref()
                .map(|record| record.submission_id.as_str()),
            Some(submission.submission_id.as_str())
        );
        assert!(matches!(
            store.clear_primary_submission_target(&workspace.id, &target.selection_id, true),
            Err(WorkspaceError::SubmissionTargetLockedBySubmission)
        ));
    }
}
