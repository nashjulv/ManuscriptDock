use crate::workspace::WorkspaceSummary;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, error::Error, fmt};

pub const KNOWLEDGE_BODY_SCHEMA_VERSION: u32 = 1;
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
    pub network: KnowledgeBodyNetwork,
    pub external_transmission: String,
}

impl AcademicKnowledgeBodySnapshot {
    pub fn validate(&self) -> Result<(), KnowledgeBodyError> {
        if self.schema_version != KNOWLEDGE_BODY_SCHEMA_VERSION || self.snapshot_version == 0 {
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
        for assertion in &self.network.assertions {
            assertion.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnowledgeBodyError {
    InvalidSnapshot,
    InvalidAiReviewHistory,
    UnresolvedAiReviewReference,
    InvalidNetworkAssertion,
}

impl fmt::Display for KnowledgeBodyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSnapshot => write!(formatter, "知识体快照版本无效"),
            Self::InvalidAiReviewHistory => write!(formatter, "AI 审核报告版本链无效"),
            Self::UnresolvedAiReviewReference => {
                write!(formatter, "知识体快照引用的 AI 审核报告版本不存在")
            }
            Self::InvalidNetworkAssertion => {
                write!(formatter, "关联知识体声明缺少成立依据或协议类型无效")
            }
        }
    }
}

impl Error for KnowledgeBodyError {}

pub fn local_knowledge_body_snapshot(
    workspace: &WorkspaceSummary,
) -> AcademicKnowledgeBodySnapshot {
    let knowledge_body_id = format!("kb:{}", workspace.id);
    let claim_id = format!("{knowledge_body_id}:claim:primary");
    let object = |suffix: &str, object_type, version| VersionedObjectReference {
        object_id: format!("{knowledge_body_id}:{suffix}"),
        object_type,
        version,
    };
    let claim = VersionedObjectReference {
        object_id: claim_id,
        object_type: KnowledgeObjectType::Claim,
        version: 1,
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
    let method = object("method:primary", KnowledgeObjectType::Method, 0);
    let snapshot = AcademicKnowledgeBodySnapshot {
        schema_version: KNOWLEDGE_BODY_SCHEMA_VERSION,
        knowledge_body_id: knowledge_body_id.clone(),
        snapshot_version: workspace.snapshot_version,
        manuscript: object(
            "artifact:manuscript",
            KnowledgeObjectType::ArtifactVersion,
            workspace.snapshot_version,
        ),
        claim: ClaimFiveTuple {
            claim: claim.clone(),
            proposition: ClaimElementReference {
                reference: object("proposition:primary", KnowledgeObjectType::Proposition, 0),
                state: ElementState::Pending,
            },
            conditions: ClaimElementReference {
                reference: object("scope:primary", KnowledgeObjectType::Scope, 0),
                state: ElementState::Pending,
            },
            evidence: ClaimElementReference {
                reference: object("evidence:primary", KnowledgeObjectType::Evidence, 0),
                state: ElementState::Pending,
            },
            sources: ClaimElementReference {
                reference: sources.clone(),
                state: ElementState::Established,
            },
            status: ClaimElementReference {
                reference: object("status:building", KnowledgeObjectType::Status, 1),
                state: ElementState::Established,
            },
        },
        objects: KnowledgeBodyObjectSet {
            artifact_version: object(
                "artifact:manuscript",
                KnowledgeObjectType::ArtifactVersion,
                workspace.snapshot_version,
            ),
            claim: claim.clone(),
            scope: object("scope:primary", KnowledgeObjectType::Scope, 0),
            method: method.clone(),
            result: object("result:primary", KnowledgeObjectType::Result, 0),
            evidence_relation: object(
                "evidence-relation:primary",
                KnowledgeObjectType::EvidenceRelation,
                0,
            ),
            source_anchor: sources.clone(),
            ai_review_report: None,
            provenance: object("provenance:primary", KnowledgeObjectType::Provenance, 1),
            knowledge_body_snapshot: object(
                "snapshot:current",
                KnowledgeObjectType::KnowledgeBodySnapshot,
                workspace.snapshot_version,
            ),
        },
        ai_review_report: None,
        ai_review_history: AiReviewReportHistory {
            report_id: format!("{knowledge_body_id}:ai-review:primary"),
            current_version: None,
            versions: Vec::new(),
        },
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
    use crate::{ManuscriptKind, ManuscriptSummary};

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
        let mut snapshot = local_knowledge_body_snapshot(&workspace());
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
        let snapshot = local_knowledge_body_snapshot(&workspace());
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
    }
}
