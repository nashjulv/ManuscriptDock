use crate::VersionedObjectReference;
use serde::{Deserialize, Serialize};

pub const KNOWLEDGE_DIALOGUE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeInquiryOrigin {
    Owner,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeInquiryStance {
    Recognition,
    Question,
    Challenge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeInquiryTarget {
    KnowledgeBody,
    Claim,
    Scope,
    Method,
    Result,
    EvidenceRelation,
    SourceAnchor,
    AiReviewReport,
    Provenance,
    CapabilityContract,
    RightsReputation,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeInquiryRecord {
    pub schema_version: u32,
    pub inquiry_id: String,
    pub workspace_id: String,
    pub knowledge_body_record_id: String,
    pub knowledge_body_hash: String,
    pub snapshot_version: u32,
    pub origin: KnowledgeInquiryOrigin,
    pub stance: KnowledgeInquiryStance,
    pub target: KnowledgeInquiryTarget,
    pub question: String,
    pub external_actor_label: Option<String>,
    pub created_unix_ms: u64,
    pub record_hash: String,
    pub external_transmission: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeAnswerRecord {
    pub schema_version: u32,
    pub answer_id: String,
    pub inquiry_id: String,
    pub workspace_id: String,
    pub knowledge_body_record_id: String,
    pub model_slot: String,
    pub provider_label: String,
    pub model: String,
    pub answer: String,
    pub source_anchors: Vec<VersionedObjectReference>,
    pub created_unix_ms: u64,
    pub record_hash: String,
    pub external_transmission: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeDialogueItem {
    pub inquiry: KnowledgeInquiryRecord,
    pub answers: Vec<KnowledgeAnswerRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeDialogueLedger {
    pub workspace_id: String,
    pub knowledge_body_record_id: String,
    pub knowledge_body_hash: String,
    pub items: Vec<KnowledgeDialogueItem>,
}
