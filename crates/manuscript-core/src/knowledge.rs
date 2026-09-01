use crate::{
    structure::{DecompositionManifest, SemanticCandidate, SemanticElementKind, SourceModality},
    workspace::WorkspaceSummary,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const KNOWLEDGE_BODY_SCHEMA_VERSION: u32 = 4;
pub const DISCIPLINE_INDEX_SCHEME: &str = "ManuscriptDock Discipline Index";
pub const DISCIPLINE_INDEX_VERSION: &str = "1.0";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisciplineCatalogItem {
    pub code: String,
    pub label: String,
    pub label_en: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisciplineClassification {
    pub assignment_id: String,
    pub version: u32,
    pub scheme: String,
    pub scheme_version: String,
    pub code: String,
    pub label: String,
    pub label_en: String,
    pub status: String,
    pub basis: String,
}

const DISCIPLINE_INDEX: [(&str, &str, &str); 12] = [
    ("multidisciplinary", "综合与跨学科", "Multidisciplinary"),
    (
        "mathematics_statistics",
        "数学与统计学",
        "Mathematics and statistics",
    ),
    (
        "computer_information_sciences",
        "计算机与信息科学",
        "Computer and information sciences",
    ),
    (
        "physical_sciences",
        "物理与天文学",
        "Physical sciences and astronomy",
    ),
    (
        "chemistry_materials",
        "化学与材料科学",
        "Chemistry and materials science",
    ),
    (
        "earth_environmental_sciences",
        "地球与环境科学",
        "Earth and environmental sciences",
    ),
    ("life_sciences", "生命科学", "Life sciences"),
    (
        "medicine_health_sciences",
        "医学与健康科学",
        "Medicine and health sciences",
    ),
    (
        "engineering_technology",
        "工程与技术",
        "Engineering and technology",
    ),
    (
        "agriculture_veterinary",
        "农业与兽医学",
        "Agriculture and veterinary sciences",
    ),
    ("social_sciences", "社会科学", "Social sciences"),
    ("humanities_arts", "人文与艺术", "Humanities and arts"),
];

pub fn discipline_catalog() -> Vec<DisciplineCatalogItem> {
    DISCIPLINE_INDEX
        .iter()
        .map(|(code, label, label_en)| DisciplineCatalogItem {
            code: (*code).to_owned(),
            label: (*label).to_owned(),
            label_en: (*label_en).to_owned(),
        })
        .collect()
}

pub fn discipline_catalog_item(code: &str) -> Option<DisciplineCatalogItem> {
    discipline_catalog()
        .into_iter()
        .find(|item| item.code == code)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeObjectType {
    KnowledgeBody,
    KnowledgeBodySnapshot,
    Claim,
    Proposition,
    Scope,
    Evidence,
    EvidenceRelation,
    SourceAnchor,
    Status,
    Method,
    Result,
    ArtifactVersion,
    AiReviewReport,
    Provenance,
    CapabilityContract,
    RuntimeProfile,
    RightsPolicy,
    ReputationRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionedObjectReference {
    pub object_id: String,
    pub object_type: KnowledgeObjectType,
    pub version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementState {
    Pending,
    Candidate,
    Established,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimElementReference {
    #[serde(flatten)]
    pub reference: VersionedObjectReference,
    pub state: ElementState,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFiveTuple {
    pub claim: VersionedObjectReference,
    pub proposition: ClaimElementReference,
    pub conditions: ClaimElementReference,
    pub evidence: ClaimElementReference,
    pub sources: ClaimElementReference,
    pub status: ClaimElementReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeCandidateContent {
    pub candidate_id: String,
    pub text: String,
    pub source_label: String,
    pub source_fragment_id: Option<String>,
    pub modality: SourceModality,
    pub confidence_percent: u8,
    pub author_confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeCandidateDecision {
    pub candidate_id: String,
    pub included: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationContactKind {
    Email,
    Orcid,
    Correspondence,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicationContact {
    pub kind: PublicationContactKind,
    pub value: String,
    pub source_label: String,
    pub source_fragment_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceIdentityStatus {
    AwaitingExtraction,
    NotDetected,
    Extracted,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceIdentityVersion {
    pub version: u32,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub affiliations: Vec<String>,
    pub contacts: Vec<PublicationContact>,
    pub source_artifact: VersionedObjectReference,
    pub status: SourceIdentityStatus,
    pub disclosure_basis: String,
    pub local_visibility: String,
    pub external_model_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedKnowledgeElement {
    pub object: VersionedObjectReference,
    pub state: ElementState,
    pub candidates: Vec<KnowledgeCandidateContent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeExtractionLayer {
    pub decomposition_id: String,
    pub decomposition_hash: String,
    pub analysis_version: u32,
    pub source_snapshot_version: u32,
    pub generated_by: String,
    pub confirmation_policy: String,
    pub claim: ExtractedKnowledgeElement,
    pub scope: ExtractedKnowledgeElement,
    pub method: ExtractedKnowledgeElement,
    pub result: ExtractedKnowledgeElement,
    pub evidence: ExtractedKnowledgeElement,
}

impl KnowledgeExtractionLayer {
    pub fn all_candidates(&self) -> impl Iterator<Item = &KnowledgeCandidateContent> {
        [
            &self.claim,
            &self.scope,
            &self.method,
            &self.result,
            &self.evidence,
        ]
        .into_iter()
        .flat_map(|element| element.candidates.iter())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBodyObjectSet {
    pub artifact_version: VersionedObjectReference,
    pub claim: VersionedObjectReference,
    pub scope: VersionedObjectReference,
    pub method: VersionedObjectReference,
    pub result: VersionedObjectReference,
    pub evidence_relation: VersionedObjectReference,
    pub source_anchor: VersionedObjectReference,
    pub ai_review_report: Option<VersionedObjectReference>,
    pub provenance: VersionedObjectReference,
    pub knowledge_body_snapshot: VersionedObjectReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeBodyLifecycleStatus {
    Active,
    Deprecated,
    Withdrawn,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityAvailability {
    Available,
    RequiresRuntime,
    Planned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeBindingPolicy {
    Replaceable,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityVersionLayer {
    pub knowledge_body: VersionedObjectReference,
    pub current_snapshot: VersionedObjectReference,
    pub source_artifact: VersionedObjectReference,
    pub creator_provenance: VersionedObjectReference,
    pub lifecycle_status: KnowledgeBodyLifecycleStatus,
    pub supersedes: Option<VersionedObjectReference>,
    pub immutable_history: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBoundaryEvidenceLayer {
    pub claims: Vec<VersionedObjectReference>,
    pub scope: VersionedObjectReference,
    pub method: VersionedObjectReference,
    pub result: VersionedObjectReference,
    pub evidence: VersionedObjectReference,
    pub evidence_relation: VersionedObjectReference,
    pub source_anchor: VersionedObjectReference,
    pub known_limitations: Vec<String>,
    pub unverified_objects: Vec<VersionedObjectReference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityContract {
    pub contract_id: String,
    pub version: u32,
    pub capability: String,
    pub input_contract: Vec<String>,
    pub output_contract: Vec<String>,
    pub preconditions: Vec<String>,
    pub refusal_conditions: Vec<String>,
    pub evidence_sources: Vec<VersionedObjectReference>,
    pub availability: CapabilityAvailability,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionRuntimeLayer {
    pub runtime_profile: VersionedObjectReference,
    pub binding_policy: RuntimeBindingPolicy,
    pub coordinator_role: String,
    pub allowed_tools: Vec<String>,
    pub per_call_authorization: bool,
    pub external_transmission: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationRightsReputationLayer {
    pub validation_records: Vec<VersionedObjectReference>,
    pub rights_policy: VersionedObjectReference,
    pub reputation_record: VersionedObjectReference,
    pub content_snapshot: VersionedObjectReference,
    pub attribution_required: bool,
    pub reputation_updates_independently: bool,
    pub reuse_control: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBodyServiceArchitecture {
    pub identity_and_version: IdentityVersionLayer,
    pub knowledge_boundary_and_evidence: KnowledgeBoundaryEvidenceLayer,
    pub capability_contracts: Vec<CapabilityContract>,
    pub interaction_runtime: InteractionRuntimeLayer,
    pub validation_rights_and_reputation: ValidationRightsReputationLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiReviewStatus {
    RevisionRequired,
    Passed,
    Unresolved,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiReviewReportVersion {
    pub report_id: String,
    pub version: u32,
    pub previous_version: Option<u32>,
    pub reviewed_claim: VersionedObjectReference,
    pub reviewer_id: String,
    pub reviewer_version: String,
    pub created_unix_ms: u64,
    pub status: AiReviewStatus,
    pub summary: String,
    pub external_transmission: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiReviewReportHistory {
    pub report_id: String,
    pub current_version: Option<u32>,
    pub versions: Vec<AiReviewReportVersion>,
}

impl AiReviewReportHistory {
    pub fn validate(&self) -> Result<(), KnowledgeBodyError> {
        let mut seen = BTreeSet::new();
        for report in &self.versions {
            if report.report_id != self.report_id || report.version == 0 {
                return Err(KnowledgeBodyError::InvalidAiReviewHistory);
            }
            if !seen.insert(report.version) {
                return Err(KnowledgeBodyError::InvalidAiReviewHistory);
            }
            if report.version == 1 && report.previous_version.is_some() {
                return Err(KnowledgeBodyError::InvalidAiReviewHistory);
            }
            if report.version > 1 && report.previous_version != Some(report.version - 1) {
                return Err(KnowledgeBodyError::InvalidAiReviewHistory);
            }
        }
        if let Some(highest) = seen.iter().next_back().copied() {
            if seen.len() != highest as usize
                || !(1..=highest).all(|version| seen.contains(&version))
            {
                return Err(KnowledgeBodyError::InvalidAiReviewHistory);
            }
        }
        match self.current_version {
            Some(current) if seen.contains(&current) => Ok(()),
            None if self.versions.is_empty() => Ok(()),
            _ => Err(KnowledgeBodyError::InvalidAiReviewHistory),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeBodyRole {
    CurrentStudy,
    OriginalResearch,
    ReproductionResearch,
    CompetingResearch,
    CrossDomainApplication,
    LaterSynthesis,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBodyNode {
    pub body: VersionedObjectReference,
    pub display_id: String,
    pub title: String,
    pub role: KnowledgeBodyRole,
    pub claim: VersionedObjectReference,
    pub source_anchor: VersionedObjectReference,
    pub method: VersionedObjectReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    Citation,
    ClaimRelation,
    EvidenceRelation,
    MethodTransfer,
    Reproduction,
    Alignment,
    VersionRelation,
    Classification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum RelationProtocol {
    CitationAssertion,
    ClaimRelationAssertion,
    EvidenceRelation,
    MethodRelationAssertion,
    ReproductionAssertion,
    AlignmentAssertion,
    VersionRelation,
    ClassificationAssignment,
}

impl RelationKind {
    pub const fn protocol(self) -> RelationProtocol {
        match self {
            Self::Citation => RelationProtocol::CitationAssertion,
            Self::ClaimRelation => RelationProtocol::ClaimRelationAssertion,
            Self::EvidenceRelation => RelationProtocol::EvidenceRelation,
            Self::MethodTransfer => RelationProtocol::MethodRelationAssertion,
            Self::Reproduction => RelationProtocol::ReproductionAssertion,
            Self::Alignment => RelationProtocol::AlignmentAssertion,
            Self::VersionRelation => RelationProtocol::VersionRelation,
            Self::Classification => RelationProtocol::ClassificationAssignment,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssertionStatus {
    Candidate,
    AuthorConfirmed,
    Verified,
    Disputed,
    Withdrawn,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssertionBasis {
    pub label: String,
    pub source: VersionedObjectReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkAssertion {
    pub assertion_id: String,
    pub version: u32,
    pub relation_kind: RelationKind,
    pub protocol_object: RelationProtocol,
    pub source: VersionedObjectReference,
    pub target: VersionedObjectReference,
    pub basis: Vec<AssertionBasis>,
    pub status: AssertionStatus,
}

impl NetworkAssertion {
    pub fn validate(&self) -> Result<(), KnowledgeBodyError> {
        if self.version == 0
            || self.assertion_id.trim().is_empty()
            || (self.source.object_id == self.target.object_id
                && self.source.version == self.target.version)
            || self.basis.is_empty()
            || self.basis.iter().any(|basis| basis.label.trim().is_empty())
            || self.protocol_object != self.relation_kind.protocol()
        {
            return Err(KnowledgeBodyError::InvalidNetworkAssertion);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBodyNetwork {
    pub bodies: Vec<KnowledgeBodyNode>,
    pub assertions: Vec<NetworkAssertion>,
    pub supported_relations: Vec<RelationKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcademicKnowledgeBodySnapshot {
    pub schema_version: u32,
    pub knowledge_body_id: String,
    pub snapshot_version: u32,
    pub manuscript: VersionedObjectReference,
    pub claim: ClaimFiveTuple,
    pub objects: KnowledgeBodyObjectSet,
    pub ai_review_report: Option<VersionedObjectReference>,
    pub ai_review_history: AiReviewReportHistory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_identity: Option<SourceIdentityVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extraction: Option<KnowledgeExtractionLayer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_architecture: Option<KnowledgeBodyServiceArchitecture>,
    pub network: KnowledgeBodyNetwork,
    pub external_transmission: String,
}

impl AcademicKnowledgeBodySnapshot {
    pub fn validate(&self) -> Result<(), KnowledgeBodyError> {
        if !(1..=KNOWLEDGE_BODY_SCHEMA_VERSION).contains(&self.schema_version)
            || self.snapshot_version == 0
        {
            return Err(KnowledgeBodyError::InvalidSnapshot);
        }
        if self.objects.artifact_version != self.manuscript
            || self.objects.claim != self.claim.claim
            || self.objects.scope != self.claim.conditions.reference
            || self.objects.source_anchor != self.claim.sources.reference
            || self.objects.ai_review_report != self.ai_review_report
            || self.objects.knowledge_body_snapshot.object_type
                != KnowledgeObjectType::KnowledgeBodySnapshot
            || self.objects.knowledge_body_snapshot.version != self.snapshot_version
            || self.objects.method.object_type != KnowledgeObjectType::Method
            || self.objects.result.object_type != KnowledgeObjectType::Result
            || self.objects.evidence_relation.object_type != KnowledgeObjectType::EvidenceRelation
            || self.objects.provenance.object_type != KnowledgeObjectType::Provenance
        {
            return Err(KnowledgeBodyError::InvalidSnapshot);
        }
        if let Some(extraction) = &self.extraction {
            let candidate_elements = [
                (&extraction.claim, &self.claim.proposition.reference),
                (&extraction.scope, &self.objects.scope),
                (&extraction.method, &self.objects.method),
                (&extraction.result, &self.objects.result),
                (&extraction.evidence, &self.claim.evidence.reference),
            ];
            if extraction.decomposition_id.trim().is_empty()
                || extraction.decomposition_hash.len() != 64
                || extraction.source_snapshot_version != self.snapshot_version
                || extraction.generated_by.trim().is_empty()
                || extraction.confirmation_policy.trim().is_empty()
                || candidate_elements.iter().any(|(element, expected)| {
                    let expected_state = if element.candidates.is_empty() {
                        ElementState::Pending
                    } else if element
                        .candidates
                        .iter()
                        .any(|candidate| candidate.author_confirmed)
                    {
                        ElementState::Established
                    } else {
                        ElementState::Candidate
                    };
                    element.object != **expected
                        || element.state != expected_state
                        || element.candidates.iter().any(|candidate| {
                            candidate.candidate_id.trim().is_empty()
                                || candidate.text.trim().is_empty()
                                || candidate.source_label.trim().is_empty()
                                || candidate.confidence_percent > 100
                        })
                })
                || extraction.claim.state != self.claim.proposition.state
                || extraction.scope.state != self.claim.conditions.state
                || extraction.evidence.state != self.claim.evidence.state
            {
                return Err(KnowledgeBodyError::InvalidSnapshot);
            }
        }
        match (&self.source_identity, self.schema_version) {
            (Some(identity), _) => {
                if identity.version != self.snapshot_version
                    || identity.source_artifact != self.manuscript
                    || identity.disclosure_basis != "source_document_declared_metadata"
                    || identity.local_visibility != "visible_in_local_workspace"
                    || identity.external_model_policy != "excluded_from_default_model_projection"
                    || identity
                        .authors
                        .iter()
                        .any(|author| author.trim().is_empty())
                    || identity
                        .affiliations
                        .iter()
                        .any(|affiliation| affiliation.trim().is_empty())
                    || identity.contacts.iter().any(|contact| {
                        contact.value.trim().is_empty() || contact.source_label.trim().is_empty()
                    })
                {
                    return Err(KnowledgeBodyError::InvalidSnapshot);
                }
            }
            (None, 1..=3) => {}
            (None, _) => return Err(KnowledgeBodyError::InvalidSnapshot),
        }
        self.ai_review_history.validate()?;
        match &self.ai_review_report {
            Some(reference)
                if reference.object_type == KnowledgeObjectType::AiReviewReport
                    && Some(reference.version) == self.ai_review_history.current_version
                    && reference.object_id == self.ai_review_history.report_id
                    && self.ai_review_history.versions.iter().any(|report| {
                        report.version == reference.version
                            && report.reviewed_claim == self.claim.claim
                    }) => {}
            None if self.ai_review_history.current_version.is_none() => {}
            _ => return Err(KnowledgeBodyError::UnresolvedAiReviewReference),
        }
        match (&self.service_architecture, self.schema_version) {
            (Some(architecture), _) => self.validate_service_architecture(architecture)?,
            (None, 1) => {}
            (None, _) => return Err(KnowledgeBodyError::InvalidServiceArchitecture),
        }
        for assertion in &self.network.assertions {
            assertion.validate()?;
        }
        Ok(())
    }

    fn validate_service_architecture(
        &self,
        architecture: &KnowledgeBodyServiceArchitecture,
    ) -> Result<(), KnowledgeBodyError> {
        let identity = &architecture.identity_and_version;
        let knowledge = &architecture.knowledge_boundary_and_evidence;
        let runtime = &architecture.interaction_runtime;
        let trust = &architecture.validation_rights_and_reputation;
        let contract_ids = architecture
            .capability_contracts
            .iter()
            .map(|contract| contract.contract_id.as_str())
            .collect::<BTreeSet<_>>();
        let contracts_are_valid = contract_ids.len() == architecture.capability_contracts.len()
            && architecture.capability_contracts.iter().all(|contract| {
                !contract.contract_id.trim().is_empty()
                    && contract.version > 0
                    && !contract.capability.trim().is_empty()
                    && !contract.input_contract.is_empty()
                    && !contract.output_contract.is_empty()
                    && !contract.preconditions.is_empty()
                    && !contract.refusal_conditions.is_empty()
                    && !contract.evidence_sources.is_empty()
            });
        let review_is_registered = self.ai_review_report.as_ref().is_none_or(|review| {
            trust
                .validation_records
                .iter()
                .any(|record| record == review)
        });
        if identity.knowledge_body.object_id != self.knowledge_body_id
            || identity.knowledge_body.object_type != KnowledgeObjectType::KnowledgeBody
            || identity.current_snapshot != self.objects.knowledge_body_snapshot
            || identity.source_artifact != self.manuscript
            || identity.creator_provenance != self.objects.provenance
            || !identity.immutable_history
            || knowledge.claims.is_empty()
            || !knowledge
                .claims
                .iter()
                .any(|claim| claim == &self.claim.claim)
            || knowledge.scope != self.objects.scope
            || knowledge.method != self.objects.method
            || knowledge.result != self.objects.result
            || knowledge.evidence != self.claim.evidence.reference
            || knowledge.evidence_relation != self.objects.evidence_relation
            || knowledge.source_anchor != self.objects.source_anchor
            || runtime.runtime_profile.object_type != KnowledgeObjectType::RuntimeProfile
            || runtime.binding_policy != RuntimeBindingPolicy::Replaceable
            || !runtime.per_call_authorization
            || trust.rights_policy.object_type != KnowledgeObjectType::RightsPolicy
            || trust.reputation_record.object_type != KnowledgeObjectType::ReputationRecord
            || trust.content_snapshot != self.objects.knowledge_body_snapshot
            || !trust.attribution_required
            || !trust.reputation_updates_independently
            || !review_is_registered
            || !contracts_are_valid
        {
            return Err(KnowledgeBodyError::InvalidServiceArchitecture);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnowledgeBodyError {
    InvalidSnapshot,
    InvalidAiReviewHistory,
    UnresolvedAiReviewReference,
    InvalidServiceArchitecture,
    InvalidNetworkAssertion,
    InvalidCandidateReview,
}

impl fmt::Display for KnowledgeBodyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSnapshot => write!(formatter, "知识体快照版本无效"),
            Self::InvalidAiReviewHistory => write!(formatter, "AI 审核报告版本链无效"),
            Self::UnresolvedAiReviewReference => {
                write!(formatter, "知识体快照引用的 AI 审核报告版本不存在")
            }
            Self::InvalidServiceArchitecture => {
                write!(formatter, "知识体五部分服务架构或能力契约无效")
            }
            Self::InvalidNetworkAssertion => {
                write!(formatter, "关联知识体声明缺少成立依据或协议类型无效")
            }
            Self::InvalidCandidateReview => {
                write!(
                    formatter,
                    "知识候选审核必须逐条选择纳入或排除，且必须对应当前论文分解"
                )
            }
        }
    }
}

impl Error for KnowledgeBodyError {}

pub fn apply_candidate_decisions(
    snapshot: &mut AcademicKnowledgeBodySnapshot,
    decisions: &[KnowledgeCandidateDecision],
) -> Result<(), KnowledgeBodyError> {
    let extraction = snapshot
        .extraction
        .as_mut()
        .ok_or(KnowledgeBodyError::InvalidCandidateReview)?;
    let decision_map = decisions
        .iter()
        .map(|decision| (decision.candidate_id.as_str(), decision.included))
        .collect::<BTreeMap<_, _>>();
    if decision_map.len() != decisions.len() {
        return Err(KnowledgeBodyError::InvalidCandidateReview);
    }

    let expected_ids = [
        &extraction.claim,
        &extraction.scope,
        &extraction.method,
        &extraction.result,
        &extraction.evidence,
    ]
    .into_iter()
    .flat_map(|element| {
        element
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_id.as_str())
    })
    .collect::<BTreeSet<_>>();
    if expected_ids.len() != decision_map.len()
        || expected_ids
            .iter()
            .any(|candidate_id| !decision_map.contains_key(candidate_id))
    {
        return Err(KnowledgeBodyError::InvalidCandidateReview);
    }

    fn apply_element(element: &mut ExtractedKnowledgeElement, decision_map: &BTreeMap<&str, bool>) {
        for candidate in &mut element.candidates {
            candidate.author_confirmed = decision_map
                .get(candidate.candidate_id.as_str())
                .copied()
                .unwrap_or(false);
        }
        element.state = if element.candidates.is_empty() {
            ElementState::Pending
        } else if element
            .candidates
            .iter()
            .any(|candidate| candidate.author_confirmed)
        {
            ElementState::Established
        } else {
            ElementState::Candidate
        };
    }

    apply_element(&mut extraction.claim, &decision_map);
    apply_element(&mut extraction.scope, &decision_map);
    apply_element(&mut extraction.method, &decision_map);
    apply_element(&mut extraction.result, &decision_map);
    apply_element(&mut extraction.evidence, &decision_map);
    snapshot.claim.proposition.state = extraction.claim.state;
    snapshot.claim.conditions.state = extraction.scope.state;
    snapshot.claim.evidence.state = extraction.evidence.state;
    snapshot.claim.status.state = ElementState::Established;

    if let Some(architecture) = snapshot.service_architecture.as_mut() {
        architecture
            .knowledge_boundary_and_evidence
            .known_limitations
            .retain(|limitation| {
                limitation != "locally_extracted_semantic_candidates_require_author_confirmation"
            });
        architecture
            .knowledge_boundary_and_evidence
            .known_limitations
            .push("author_review_completed_for_current_decomposition".to_owned());
        architecture
            .knowledge_boundary_and_evidence
            .unverified_objects = [
            (&extraction.scope, snapshot.objects.scope.clone()),
            (&extraction.method, snapshot.objects.method.clone()),
            (&extraction.result, snapshot.objects.result.clone()),
            (
                &extraction.evidence,
                snapshot.claim.evidence.reference.clone(),
            ),
        ]
        .into_iter()
        .filter_map(|(element, object)| {
            (element.state != ElementState::Established).then_some(object)
        })
        .collect();
    }
    snapshot.validate()
}

fn extracted_element(
    knowledge_body_id: &str,
    object: VersionedObjectReference,
    element: SemanticElementKind,
    candidates: &[SemanticCandidate],
) -> ExtractedKnowledgeElement {
    let candidates = candidates
        .iter()
        .filter(|candidate| candidate.element == element)
        .enumerate()
        .map(|(index, candidate)| KnowledgeCandidateContent {
            candidate_id: format!(
                "{knowledge_body_id}:candidate:{}:{}",
                semantic_element_label(element),
                index + 1
            ),
            text: candidate.text.clone(),
            source_label: candidate.source_label.clone(),
            source_fragment_id: candidate.source_fragment_id.clone(),
            modality: candidate.modality,
            confidence_percent: candidate.confidence_percent,
            author_confirmed: false,
        })
        .collect::<Vec<_>>();
    ExtractedKnowledgeElement {
        object,
        state: if candidates.is_empty() {
            ElementState::Pending
        } else {
            ElementState::Candidate
        },
        candidates,
    }
}

fn semantic_element_label(element: SemanticElementKind) -> &'static str {
    match element {
        SemanticElementKind::Claim => "claim",
        SemanticElementKind::Scope => "scope",
        SemanticElementKind::Method => "method",
        SemanticElementKind::Result => "result",
        SemanticElementKind::Evidence => "evidence",
    }
}

fn source_identity_version(
    report: Option<&crate::structure::StructureReport>,
    source_artifact: VersionedObjectReference,
    version: u32,
) -> SourceIdentityVersion {
    let mut affiliations = Vec::new();
    let mut contacts = Vec::new();
    if let Some(report) = report {
        for fragment in report.source_fragments.iter().take(120) {
            if looks_like_affiliation(&fragment.text)
                && !report
                    .authors
                    .iter()
                    .any(|author| author.eq_ignore_ascii_case(fragment.text.trim()))
            {
                push_unique_case_insensitive(&mut affiliations, &fragment.text);
            }
            for email in extract_email_addresses(&fragment.text) {
                push_unique_contact(
                    &mut contacts,
                    PublicationContact {
                        kind: PublicationContactKind::Email,
                        value: email,
                        source_label: fragment.source_label.clone(),
                        source_fragment_id: Some(fragment.fragment_id.clone()),
                    },
                );
            }
            for orcid in extract_orcid_identifiers(&fragment.text) {
                push_unique_contact(
                    &mut contacts,
                    PublicationContact {
                        kind: PublicationContactKind::Orcid,
                        value: orcid,
                        source_label: fragment.source_label.clone(),
                        source_fragment_id: Some(fragment.fragment_id.clone()),
                    },
                );
            }
            if looks_like_correspondence_line(&fragment.text)
                && !contacts.iter().any(|contact| {
                    contact.source_fragment_id.as_deref() == Some(fragment.fragment_id.as_str())
                })
            {
                push_unique_contact(
                    &mut contacts,
                    PublicationContact {
                        kind: PublicationContactKind::Correspondence,
                        value: bounded_identity_text(&fragment.text, 320),
                        source_label: fragment.source_label.clone(),
                        source_fragment_id: Some(fragment.fragment_id.clone()),
                    },
                );
            }
        }
    }
    let title = report.and_then(|report| report.title.clone());
    let authors = report
        .map(|report| report.authors.clone())
        .unwrap_or_default();
    let status = match report {
        None => SourceIdentityStatus::AwaitingExtraction,
        Some(_)
            if title.is_none()
                && authors.is_empty()
                && affiliations.is_empty()
                && contacts.is_empty() =>
        {
            SourceIdentityStatus::NotDetected
        }
        Some(_) => SourceIdentityStatus::Extracted,
    };
    SourceIdentityVersion {
        version,
        title,
        authors,
        affiliations,
        contacts,
        source_artifact,
        status,
        disclosure_basis: "source_document_declared_metadata".to_owned(),
        local_visibility: "visible_in_local_workspace".to_owned(),
        external_model_policy: "excluded_from_default_model_projection".to_owned(),
    }
}

fn looks_like_affiliation(text: &str) -> bool {
    let normalized = text.trim();
    if normalized.chars().count() < 4 || normalized.chars().count() > 320 {
        return false;
    }
    let lower = normalized.to_ascii_lowercase();
    let excluded = [
        "abstract",
        "keywords",
        "references",
        "摘要",
        "关键词",
        "参考文献",
    ];
    if excluded.iter().any(|marker| lower.starts_with(marker)) {
        return false;
    }
    [
        "university",
        "institute",
        "department",
        "school of",
        "college of",
        "laboratory",
        "centre for",
        "center for",
        "hospital",
        "academy",
        "大学",
        "学院",
        "研究院",
        "研究所",
        "实验室",
        "医院",
        "中心",
        "学系",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn looks_like_correspondence_line(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "corresponding author",
        "correspondence:",
        "contact:",
        "通讯作者",
        "通信作者",
        "联系人",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn extract_email_addresses(text: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    let mut current = String::new();
    let flush = |current: &mut String, candidates: &mut Vec<String>| {
        let candidate = current.trim_matches(['.', '-', '_']).to_owned();
        current.clear();
        let Some((local, domain)) = candidate.split_once('@') else {
            return;
        };
        if local.is_empty()
            || domain.is_empty()
            || domain.contains('@')
            || !domain.contains('.')
            || domain.starts_with('.')
            || domain.ends_with('.')
        {
            return;
        }
        push_unique_case_insensitive(candidates, &candidate);
    };
    for character in text.chars().chain(std::iter::once(' ')) {
        if character.is_ascii_alphanumeric()
            || matches!(character, '.' | '_' | '%' | '+' | '-' | '@')
        {
            current.push(character);
        } else if !current.is_empty() {
            flush(&mut current, &mut candidates);
        }
    }
    candidates
}

fn extract_orcid_identifiers(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    for token in text.split(|character: char| {
        !(character.is_ascii_alphanumeric() || matches!(character, '-' | '/' | ':' | '.'))
    }) {
        let candidate = token
            .trim_end_matches(['.', '/', ':'])
            .strip_prefix("https://orcid.org/")
            .or_else(|| {
                token
                    .trim_end_matches(['.', '/', ':'])
                    .strip_prefix("http://orcid.org/")
            })
            .unwrap_or(token.trim_end_matches(['.', '/', ':']));
        let groups = candidate.split('-').collect::<Vec<_>>();
        if groups.len() == 4
            && groups.iter().all(|group| group.len() == 4)
            && groups[..3]
                .iter()
                .all(|group| group.chars().all(|character| character.is_ascii_digit()))
            && groups[3].chars().enumerate().all(|(index, character)| {
                character.is_ascii_digit() || (index == 3 && matches!(character, 'X' | 'x'))
            })
        {
            push_unique_case_insensitive(&mut values, &candidate.to_ascii_uppercase());
        }
    }
    values
}

fn push_unique_case_insensitive(values: &mut Vec<String>, value: &str) {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if !normalized.is_empty()
        && !values
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&normalized))
    {
        values.push(normalized);
    }
}

fn push_unique_contact(contacts: &mut Vec<PublicationContact>, contact: PublicationContact) {
    if !contacts.iter().any(|existing| {
        existing.kind == contact.kind && existing.value.eq_ignore_ascii_case(&contact.value)
    }) {
        contacts.push(contact);
    }
}

fn bounded_identity_text(text: &str, maximum_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= maximum_chars {
        normalized
    } else {
        normalized.chars().take(maximum_chars).collect::<String>() + "…"
    }
}

pub fn local_knowledge_body_snapshot(
    workspace: &WorkspaceSummary,
    decomposition: Option<&DecompositionManifest>,
) -> AcademicKnowledgeBodySnapshot {
    let structure_report = decomposition.map(|manifest| &manifest.structure);
    let knowledge_body_id = format!("kb:{}", workspace.id);
    let claim_id = format!("{knowledge_body_id}:claim:primary");
    let object = |suffix: &str, object_type, version| VersionedObjectReference {
        object_id: format!("{knowledge_body_id}:{suffix}"),
        object_type,
        version,
    };
    let extracted_version = |kind: SemanticElementKind| {
        structure_report
            .filter(|report| {
                report.source_snapshot_version == workspace.snapshot_version
                    && report
                        .semantic_candidates
                        .iter()
                        .any(|candidate| candidate.element == kind)
            })
            .map(|_| workspace.snapshot_version)
            .unwrap_or(0)
    };
    let proposition_version = extracted_version(SemanticElementKind::Claim);
    let scope_version = extracted_version(SemanticElementKind::Scope);
    let method_version = extracted_version(SemanticElementKind::Method);
    let result_version = extracted_version(SemanticElementKind::Result);
    let evidence_version = extracted_version(SemanticElementKind::Evidence);
    let evidence_relation_version = if proposition_version > 0 && evidence_version > 0 {
        workspace.snapshot_version
    } else {
        0
    };
    let candidate_state = |version| {
        if version > 0 {
            ElementState::Candidate
        } else {
            ElementState::Pending
        }
    };
    let claim = VersionedObjectReference {
        object_id: claim_id,
        object_type: KnowledgeObjectType::Claim,
        version: proposition_version.max(1),
    };
    let sources = object(
        "source:manuscript",
        KnowledgeObjectType::SourceAnchor,
        workspace.snapshot_version,
    );
    let body = VersionedObjectReference {
        object_id: knowledge_body_id.clone(),
        object_type: KnowledgeObjectType::KnowledgeBody,
        version: workspace.snapshot_version,
    };
    let artifact = object(
        "artifact:manuscript",
        KnowledgeObjectType::ArtifactVersion,
        workspace.snapshot_version,
    );
    let proposition = object(
        "proposition:primary",
        KnowledgeObjectType::Proposition,
        proposition_version,
    );
    let scope = object("scope:primary", KnowledgeObjectType::Scope, scope_version);
    let method = object(
        "method:primary",
        KnowledgeObjectType::Method,
        method_version,
    );
    let result = object(
        "result:primary",
        KnowledgeObjectType::Result,
        result_version,
    );
    let evidence = object(
        "evidence:primary",
        KnowledgeObjectType::Evidence,
        evidence_version,
    );
    let evidence_relation = object(
        "evidence-relation:primary",
        KnowledgeObjectType::EvidenceRelation,
        evidence_relation_version,
    );
    let provenance = object("provenance:primary", KnowledgeObjectType::Provenance, 1);
    let snapshot_reference = object(
        "snapshot:current",
        KnowledgeObjectType::KnowledgeBodySnapshot,
        workspace.snapshot_version,
    );
    let source_identity = source_identity_version(
        structure_report,
        artifact.clone(),
        workspace.snapshot_version,
    );
    let extraction = structure_report
        .filter(|report| report.source_snapshot_version == workspace.snapshot_version)
        .map(|report| KnowledgeExtractionLayer {
            decomposition_id: decomposition
                .map(|manifest| manifest.decomposition_id.clone())
                .unwrap_or_default(),
            decomposition_hash: decomposition
                .map(|manifest| manifest.manifest_hash.clone())
                .unwrap_or_default(),
            analysis_version: report.analysis_version,
            source_snapshot_version: report.source_snapshot_version,
            generated_by: "local_deterministic_semantic_extraction".to_owned(),
            confirmation_policy: "machine_candidates_require_author_confirmation".to_owned(),
            claim: extracted_element(
                &knowledge_body_id,
                proposition.clone(),
                SemanticElementKind::Claim,
                &report.semantic_candidates,
            ),
            scope: extracted_element(
                &knowledge_body_id,
                scope.clone(),
                SemanticElementKind::Scope,
                &report.semantic_candidates,
            ),
            method: extracted_element(
                &knowledge_body_id,
                method.clone(),
                SemanticElementKind::Method,
                &report.semantic_candidates,
            ),
            result: extracted_element(
                &knowledge_body_id,
                result.clone(),
                SemanticElementKind::Result,
                &report.semantic_candidates,
            ),
            evidence: extracted_element(
                &knowledge_body_id,
                evidence.clone(),
                SemanticElementKind::Evidence,
                &report.semantic_candidates,
            ),
        });
    let snapshot = AcademicKnowledgeBodySnapshot {
        schema_version: KNOWLEDGE_BODY_SCHEMA_VERSION,
        knowledge_body_id: knowledge_body_id.clone(),
        snapshot_version: workspace.snapshot_version,
        manuscript: artifact.clone(),
        claim: ClaimFiveTuple {
            claim: claim.clone(),
            proposition: ClaimElementReference {
                reference: proposition,
                state: candidate_state(proposition_version),
            },
            conditions: ClaimElementReference {
                reference: scope.clone(),
                state: candidate_state(scope_version),
            },
            evidence: ClaimElementReference {
                reference: evidence.clone(),
                state: candidate_state(evidence_version),
            },
            sources: ClaimElementReference {
                reference: sources.clone(),
                state: ElementState::Established,
            },
            status: ClaimElementReference {
                reference: object(
                    if proposition_version > 0 {
                        "status:extracted-candidate"
                    } else {
                        "status:building"
                    },
                    KnowledgeObjectType::Status,
                    1,
                ),
                state: if proposition_version > 0 {
                    ElementState::Candidate
                } else {
                    ElementState::Established
                },
            },
        },
        objects: KnowledgeBodyObjectSet {
            artifact_version: artifact.clone(),
            claim: claim.clone(),
            scope: scope.clone(),
            method: method.clone(),
            result: result.clone(),
            evidence_relation: evidence_relation.clone(),
            source_anchor: sources.clone(),
            ai_review_report: None,
            provenance: provenance.clone(),
            knowledge_body_snapshot: snapshot_reference.clone(),
        },
        ai_review_report: None,
        ai_review_history: AiReviewReportHistory {
            report_id: format!("{knowledge_body_id}:ai-review:primary"),
            current_version: None,
            versions: Vec::new(),
        },
        source_identity: Some(source_identity),
        extraction,
        service_architecture: Some(KnowledgeBodyServiceArchitecture {
            identity_and_version: IdentityVersionLayer {
                knowledge_body: body.clone(),
                current_snapshot: snapshot_reference.clone(),
                source_artifact: artifact.clone(),
                creator_provenance: provenance,
                lifecycle_status: KnowledgeBodyLifecycleStatus::Active,
                supersedes: None,
                immutable_history: true,
            },
            knowledge_boundary_and_evidence: KnowledgeBoundaryEvidenceLayer {
                claims: vec![claim.clone()],
                scope: scope.clone(),
                method: method.clone(),
                result: result.clone(),
                evidence: evidence.clone(),
                evidence_relation: evidence_relation.clone(),
                source_anchor: sources.clone(),
                known_limitations: vec![
                    if structure_report.is_some_and(|report| !report.semantic_candidates.is_empty())
                    {
                        "locally_extracted_semantic_candidates_require_author_confirmation"
                            .to_owned()
                    } else {
                        "no_semantic_candidates_extracted_from_current_artifact".to_owned()
                    },
                    "professional_ai_review_not_performed".to_owned(),
                ],
                unverified_objects: vec![scope.clone(), method.clone(), result.clone(), evidence],
            },
            capability_contracts: vec![
                CapabilityContract {
                    contract_id: format!("{knowledge_body_id}:capability:source-traceability"),
                    version: 1,
                    capability: "source_traceability".to_owned(),
                    input_contract: vec!["knowledge_object_id".to_owned()],
                    output_contract: vec!["source_anchor_or_information_insufficient".to_owned()],
                    preconditions: vec!["finalized_snapshot".to_owned()],
                    refusal_conditions: vec!["object_has_no_confirmed_source_anchor".to_owned()],
                    evidence_sources: vec![sources.clone(), artifact.clone()],
                    availability: CapabilityAvailability::Available,
                },
                CapabilityContract {
                    contract_id: format!("{knowledge_body_id}:capability:evidence-bounded-qa"),
                    version: 1,
                    capability: "evidence_bounded_question_answering".to_owned(),
                    input_contract: vec!["question".to_owned(), "target_object".to_owned()],
                    output_contract: vec![
                        "answer".to_owned(),
                        "source_anchors".to_owned(),
                        "uncertainty".to_owned(),
                    ],
                    preconditions: vec![
                        "finalized_snapshot".to_owned(),
                        "author_configured_runtime".to_owned(),
                        "per_call_authorization".to_owned(),
                    ],
                    refusal_conditions: vec![
                        "outside_knowledge_boundary".to_owned(),
                        "insufficient_confirmed_evidence".to_owned(),
                        "runtime_unavailable".to_owned(),
                    ],
                    evidence_sources: vec![claim.clone(), scope.clone(), sources.clone()],
                    availability: CapabilityAvailability::RequiresRuntime,
                },
                CapabilityContract {
                    contract_id: format!("{knowledge_body_id}:capability:applicability-check"),
                    version: 1,
                    capability: "method_applicability_check".to_owned(),
                    input_contract: vec![
                        "data_scale".to_owned(),
                        "variable_types".to_owned(),
                        "distribution_conditions".to_owned(),
                    ],
                    output_contract: vec![
                        "applicable_or_not_or_insufficient".to_owned(),
                        "basis".to_owned(),
                    ],
                    preconditions: vec!["confirmed_scope_and_method".to_owned()],
                    refusal_conditions: vec![
                        "scope_or_method_pending".to_owned(),
                        "input_outside_declared_contract".to_owned(),
                    ],
                    evidence_sources: vec![scope, method.clone(), sources.clone()],
                    availability: CapabilityAvailability::Planned,
                },
            ],
            interaction_runtime: InteractionRuntimeLayer {
                runtime_profile: object(
                    "runtime:author-configured",
                    KnowledgeObjectType::RuntimeProfile,
                    1,
                ),
                binding_policy: RuntimeBindingPolicy::Replaceable,
                coordinator_role: "author_configured_model".to_owned(),
                allowed_tools: vec![
                    "local_knowledge_projection".to_owned(),
                    "source_anchor_lookup".to_owned(),
                ],
                per_call_authorization: true,
                external_transmission: "author_confirmed_per_request".to_owned(),
            },
            validation_rights_and_reputation: ValidationRightsReputationLayer {
                validation_records: Vec::new(),
                rights_policy: object(
                    "rights:author-controlled",
                    KnowledgeObjectType::RightsPolicy,
                    1,
                ),
                reputation_record: object(
                    "reputation:external-state",
                    KnowledgeObjectType::ReputationRecord,
                    0,
                ),
                content_snapshot: snapshot_reference,
                attribution_required: true,
                reputation_updates_independently: true,
                reuse_control: "author_controlled".to_owned(),
            },
        }),
        network: KnowledgeBodyNetwork {
            bodies: vec![KnowledgeBodyNode {
                body,
                display_id: "K-A".to_owned(),
                title: workspace.manuscript.name.clone(),
                role: KnowledgeBodyRole::CurrentStudy,
                claim,
                source_anchor: sources,
                method,
            }],
            assertions: Vec::new(),
            supported_relations: vec![
                RelationKind::Citation,
                RelationKind::ClaimRelation,
                RelationKind::EvidenceRelation,
                RelationKind::MethodTransfer,
                RelationKind::Reproduction,
                RelationKind::Alignment,
                RelationKind::VersionRelation,
                RelationKind::Classification,
            ],
        },
        external_transmission: "not_performed".to_owned(),
    };
    debug_assert!(snapshot.validate().is_ok());
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AnalysisQuality, ExtractedSourceFragment, ExtractionCoverage, ManuscriptKind,
        ManuscriptSummary, SourceModality, StructureReport,
    };

    fn workspace() -> WorkspaceSummary {
        WorkspaceSummary {
            id: "8ee3a2d2-ae7a-47e7-b9b8-8be72c90b1e7".to_owned(),
            manuscript: ManuscriptSummary {
                name: "synthetic-study.tex".to_owned(),
                extension: "tex".to_owned(),
                kind: ManuscriptKind::Latex,
                size_bytes: 128,
                modified_unix_ms: None,
            },
            content_hash: "a".repeat(64),
            imported_unix_ms: 1,
            snapshot_version: 3,
        }
    }

    fn review_version(
        version: u32,
        reviewed_claim: &VersionedObjectReference,
    ) -> AiReviewReportVersion {
        AiReviewReportVersion {
            report_id: "review:primary".to_owned(),
            version,
            previous_version: (version > 1).then_some(version - 1),
            reviewed_claim: reviewed_claim.clone(),
            reviewer_id: "pwc.review.agent".to_owned(),
            reviewer_version: format!("v{version}"),
            created_unix_ms: u64::from(version),
            status: AiReviewStatus::Passed,
            summary: format!("Synthetic review v{version}"),
            external_transmission: "recorded".to_owned(),
        }
    }

    #[test]
    fn keeps_ai_review_history_while_the_snapshot_pins_v2() {
        let mut snapshot = local_knowledge_body_snapshot(&workspace(), None);
        let reviewed_claim = snapshot.claim.claim.clone();
        snapshot.ai_review_history = AiReviewReportHistory {
            report_id: "review:primary".to_owned(),
            current_version: Some(2),
            versions: vec![
                review_version(1, &reviewed_claim),
                review_version(2, &reviewed_claim),
            ],
        };
        snapshot.ai_review_report = Some(VersionedObjectReference {
            object_id: "review:primary".to_owned(),
            object_type: KnowledgeObjectType::AiReviewReport,
            version: 2,
        });
        snapshot.objects.ai_review_report = snapshot.ai_review_report.clone();
        snapshot
            .service_architecture
            .as_mut()
            .unwrap()
            .validation_rights_and_reputation
            .validation_records = vec![snapshot.ai_review_report.clone().unwrap()];

        snapshot.validate().unwrap();
        assert_eq!(snapshot.ai_review_history.versions.len(), 2);
        assert_eq!(snapshot.ai_review_report.unwrap().version, 2);
    }

    #[test]
    fn maps_every_network_relation_to_a_first_class_protocol_object() {
        let kinds = [
            RelationKind::Citation,
            RelationKind::ClaimRelation,
            RelationKind::EvidenceRelation,
            RelationKind::MethodTransfer,
            RelationKind::Reproduction,
            RelationKind::Alignment,
            RelationKind::VersionRelation,
            RelationKind::Classification,
        ];
        let protocols = kinds.map(RelationKind::protocol);
        assert_eq!(protocols.len(), 8);
        assert_eq!(protocols[0], RelationProtocol::CitationAssertion);
        assert_eq!(protocols[7], RelationProtocol::ClassificationAssignment);
    }

    #[test]
    fn exposes_a_stable_bilingual_author_classification_catalog() {
        let catalog = discipline_catalog();
        let codes = catalog
            .iter()
            .map(|item| item.code.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(catalog.len(), 12);
        assert_eq!(codes.len(), catalog.len());
        assert!(catalog.iter().all(|item| {
            !item.code.trim().is_empty()
                && !item.label.trim().is_empty()
                && !item.label_en.trim().is_empty()
        }));
        assert_eq!(
            discipline_catalog_item("life_sciences").unwrap().label,
            "生命科学"
        );
    }

    #[test]
    fn local_snapshot_does_not_invent_reviews_or_cross_body_relations() {
        let snapshot = local_knowledge_body_snapshot(&workspace(), None);
        snapshot.validate().unwrap();
        assert!(snapshot.ai_review_report.is_none());
        assert!(snapshot.ai_review_history.versions.is_empty());
        assert_eq!(snapshot.network.bodies.len(), 1);
        assert!(snapshot.network.assertions.is_empty());
        assert_eq!(snapshot.claim.sources.reference.version, 3);
        assert_eq!(snapshot.objects.artifact_version.version, 3);
        assert_eq!(snapshot.objects.claim.version, 1);
        assert_eq!(snapshot.objects.scope.version, 0);
        assert_eq!(snapshot.objects.provenance.version, 1);
        assert_eq!(snapshot.objects.knowledge_body_snapshot.version, 3);
        let identity = snapshot.source_identity.as_ref().unwrap();
        assert_eq!(identity.status, SourceIdentityStatus::AwaitingExtraction);
        assert_eq!(identity.source_artifact, snapshot.manuscript);
        assert_eq!(identity.local_visibility, "visible_in_local_workspace");
        let architecture = snapshot.service_architecture.unwrap();
        assert_eq!(architecture.capability_contracts.len(), 3);
        assert_eq!(
            architecture.interaction_runtime.binding_policy,
            RuntimeBindingPolicy::Replaceable
        );
        assert!(
            architecture
                .validation_rights_and_reputation
                .reputation_updates_independently
        );
        assert_eq!(
            architecture
                .validation_rights_and_reputation
                .reputation_record
                .version,
            0
        );
    }

    #[test]
    fn preserves_source_declared_identity_and_contacts_in_the_local_snapshot() {
        let artifact = VersionedObjectReference {
            object_id: "kb:synthetic:artifact:manuscript".to_owned(),
            object_type: KnowledgeObjectType::ArtifactVersion,
            version: 3,
        };
        let report = StructureReport {
            analysis_version: 6,
            workspace_id: workspace().id,
            source_content_hash: "a".repeat(64),
            source_snapshot_version: 3,
            quality: AnalysisQuality::Complete,
            title: Some("Traceable Identity Study".to_owned()),
            authors: vec!["Ada Author".to_owned(), "Ben Researcher".to_owned()],
            abstract_present: true,
            abstract_text: Some("We propose a synthetic method.".to_owned()),
            keywords_present: false,
            sections: Vec::new(),
            figure_count: 0,
            table_count: 0,
            references_present: true,
            declarations: Vec::new(),
            page_count: None,
            word_count: 100,
            semantic_candidates: Vec::new(),
            source_fragments: vec![
                ExtractedSourceFragment {
                    fragment_id: "fragment:3:affiliation".to_owned(),
                    text: "Department of Computer Science, Synthetic University".to_owned(),
                    source_label: "首页作者单位".to_owned(),
                    modality: SourceModality::Text,
                },
                ExtractedSourceFragment {
                    fragment_id: "fragment:3:contact".to_owned(),
                    text: "Corresponding author: ada.author@example.edu; ORCID 0000-0002-1825-0097"
                        .to_owned(),
                    source_label: "首页通讯信息".to_owned(),
                    modality: SourceModality::Text,
                },
            ],
            extraction_coverage: ExtractionCoverage {
                text_fragments: 2,
                table_fragments: 0,
                figure_fragments: 0,
            },
            pdf_processing: None,
            warnings: Vec::new(),
        };

        let identity = source_identity_version(Some(&report), artifact.clone(), 3);

        assert_eq!(identity.status, SourceIdentityStatus::Extracted);
        assert_eq!(identity.authors, report.authors);
        assert_eq!(identity.affiliations.len(), 1);
        assert!(identity
            .contacts
            .iter()
            .any(|contact| contact.kind == PublicationContactKind::Email
                && contact.value == "ada.author@example.edu"));
        assert!(identity
            .contacts
            .iter()
            .any(|contact| contact.kind == PublicationContactKind::Orcid
                && contact.value == "0000-0002-1825-0097"));
        assert_eq!(identity.source_artifact, artifact);
        assert_eq!(
            identity.external_model_policy,
            "excluded_from_default_model_projection"
        );
    }

    #[test]
    fn keeps_legacy_v1_snapshots_readable_without_the_five_layer_architecture() {
        let mut snapshot = local_knowledge_body_snapshot(&workspace(), None);
        snapshot.schema_version = 1;
        snapshot.service_architecture = None;

        snapshot.validate().unwrap();
    }
}
