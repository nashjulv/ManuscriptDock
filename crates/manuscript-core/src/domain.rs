use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};

pub const PROJECT_SCHEMA_VERSION: u32 = 1;
pub const CATALOG_SCHEMA_VERSION: u32 = 1;
pub const COMPILER_VERSION: &str = "submission-compiler-v1";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    #[serde(default)]
    pub params: BTreeMap<String, String>,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_action: Option<String>,
    pub diagnostic_id: String,
}

impl AppError {
    pub fn new(code: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            params: BTreeMap::new(),
            retryable,
            recovery_action: None,
            diagnostic_id: format!("md-{}", uuid::Uuid::new_v4().simple()),
        }
    }
    pub fn param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }
    pub fn recover(mut self, action: impl Into<String>) -> Self {
        self.recovery_action = Some(action.into());
        self
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ({})", self.code, self.diagnostic_id)
    }
}
impl std::error::Error for AppError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    NeedsInput,
    Failed,
    Cancelled,
    Interrupted,
}

impl JobStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::NeedsInput | Self::Failed | Self::Cancelled | Self::Interrupted
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobEvent {
    pub seq: u64,
    pub phase: String,
    pub status: JobStatus,
    pub created_at_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_units: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_units: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRecord {
    pub schema_version: u32,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub request_id: String,
    pub operation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_hash: Option<String>,
    pub status: JobStatus,
    pub cancel_requested: bool,
    pub events: Vec<JobEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedText {
    pub zh_cn: String,
    pub en: String,
}
impl LocalizedText {
    pub fn new(zh_cn: impl Into<String>, en: impl Into<String>) -> Self {
        Self {
            zh_cn: zh_cn.into(),
            en: en.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    FindJournals,
    PreparePackage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetOrigin {
    Catalog,
    Recommendation,
    Generic,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSnapshot {
    pub id: String,
    pub file_name: String,
    pub sha256: String,
    pub format: String,
    pub size_bytes: u64,
    pub created_at_unix_ms: u64,
    pub feature_profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DocumentFacts {
    pub source_hash: String,
    pub extractor_version: String,
    pub title: Option<String>,
    pub abstract_text: Option<String>,
    pub keywords: Vec<String>,
    pub language: Option<String>,
    pub article_type: Option<String>,
    pub authors: Vec<String>,
    pub affiliations: Vec<String>,
    pub corresponding_email: Option<String>,
    pub conflict_of_interest: Option<String>,
    pub funding: Option<String>,
    pub data_availability: Option<String>,
    pub ethics_statement: Option<String>,
    pub highlights: Vec<String>,
    #[serde(default)]
    pub credit_contributions: Option<String>,
    #[serde(default)]
    pub generative_ai_disclosure: Option<String>,
    #[serde(default)]
    pub author_confirmed_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    pub id: String,
    pub file_name: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub kind: String,
    pub included: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anonymity_check: Option<AnonymityCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymityCheck {
    pub identity_context_hash: String,
    pub detected_categories: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSelection {
    pub id: String,
    pub journal_id: String,
    pub article_type: String,
    pub stage: String,
    pub origin: TargetOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation_ref: Option<String>,
    pub rules_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorDecision {
    pub requirement_id: String,
    pub value: String,
    pub evidence_ref: String,
    pub context_hash: String,
    pub confirmed_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWorkspace {
    pub kind: String,
    pub name: String,
    pub binding_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub schema_version: u32,
    pub revision: u64,
    pub display_name: String,
    pub active_source: SourceSnapshot,
    pub facts: DocumentFacts,
    pub materials: Vec<Material>,
    pub target: Option<TargetSelection>,
    #[serde(default)]
    pub author_decisions: Vec<AuthorDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<ProjectWorkspace>,
    pub last_task: TaskKind,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthorConstraints {
    pub required_language: Option<String>,
    pub article_type: Option<String>,
    pub topic_keywords: Vec<String>,
    pub required_indexing: Vec<String>,
    pub maximum_apc: Option<u32>,
    pub currency: Option<String>,
    pub require_open_access: bool,
    pub prefer_fast_first_decision: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceRef {
    pub source_url: String,
    pub label: LocalizedText,
    pub verified_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRecord {
    pub id: String,
    pub display_name: String,
    pub publisher: String,
    pub issn: Option<String>,
    pub eissn: Option<String>,
    pub aliases: Vec<String>,
    pub languages: Vec<String>,
    pub article_types: Vec<String>,
    pub topics: Vec<String>,
    pub indexing: Vec<String>,
    pub publication_route: String,
    pub apc_amount: Option<u32>,
    pub apc_currency: Option<String>,
    pub first_decision_days: Option<u32>,
    pub generation_coverage: String,
    pub status: String,
    pub verified_at: String,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalCatalog {
    pub schema_version: u32,
    pub data_version: String,
    pub journals: Vec<JournalRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintStatus {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintResult {
    pub constraint_id: String,
    pub status: ConstraintStatus,
    pub explanation: LocalizedText,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationRole {
    BestOverallFit,
    AmbitiousOption,
    LessPreparationNeeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRecommendation {
    pub journal: JournalRecord,
    pub role: RecommendationRole,
    pub reasons: Vec<LocalizedText>,
    pub risks: Vec<LocalizedText>,
    pub preparation: Vec<LocalizedText>,
    pub constraints: Vec<ConstraintResult>,
    pub evidence_coverage: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationResult {
    pub run_id: String,
    pub catalog_version: String,
    pub recommendations: Vec<JournalRecommendation>,
    pub needs_verification: Vec<JournalRecord>,
    pub excluded_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementRule {
    pub id: String,
    pub label: LocalizedText,
    pub description: LocalizedText,
    pub required: bool,
    pub kind: String,
    pub fact_key: Option<String>,
    pub required_file_kind: Option<String>,
    #[serde(default)]
    pub constraint: Option<RequirementConstraint>,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RequirementConstraint {
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
    pub max_chars_per_item: Option<usize>,
    pub max_words: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparationItem {
    pub requirement_id: String,
    pub label: LocalizedText,
    pub description: LocalizedText,
    pub action: String,
    pub status: String,
    pub required: bool,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedFile {
    pub relative_path: String,
    pub operation: String,
    pub publisher_file: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackagePlan {
    pub id: String,
    pub context_hash: String,
    pub file_plan: Vec<PlannedFile>,
    pub transform_plan: Vec<String>,
    pub capabilities: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparationStatus {
    NeedsInput,
    DraftReady,
    ReadyToExport,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparationView {
    pub project_id: String,
    pub revision: u64,
    pub context_hash: String,
    pub status: PreparationStatus,
    pub blockers: Vec<PreparationItem>,
    pub warnings: Vec<PreparationItem>,
    pub ready_items: Vec<PreparationItem>,
    pub allowed_actions: Vec<String>,
    pub package_plan: PackagePlan,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedFile {
    pub id: String,
    pub relative_path: String,
    pub purpose: LocalizedText,
    pub sha256: String,
    pub size_bytes: u64,
    pub publisher_file: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledPackage {
    pub id: String,
    pub project_id: String,
    pub context_hash: String,
    pub mode: String,
    #[serde(skip)]
    pub staging_dir: String,
    pub files: Vec<GeneratedFile>,
    pub validation_passed: bool,
    pub warnings: Vec<LocalizedText>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReceipt {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub package_id: String,
    pub output_directory: String,
    pub file_count: usize,
    pub package_hash: String,
    pub finished_at_unix_ms: u64,
    pub record_persisted: bool,
}
