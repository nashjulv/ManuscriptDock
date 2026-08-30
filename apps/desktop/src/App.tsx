import { invoke, isTauri } from "@tauri-apps/api/core";
import type { CSSProperties, ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import manuscriptDockLogo from "./assets/manuscriptdock-logo.svg";
import { I18nProvider, localize, localizeBackendText, useI18n, type Locale } from "./i18n";

type ManuscriptKind = "word" | "pdf" | "latex";

export interface ManuscriptSummary {
  name: string;
  extension: string;
  kind: ManuscriptKind;
  sizeBytes: number;
  modifiedUnixMs: number | null;
}

type ManuscriptSelection =
  | { status: "selected"; selectionId: string; manuscript: ManuscriptSummary }
  | { status: "cancelled" }
  | { status: "rejected"; message: string };

interface WorkspaceSummary {
  id: string;
  manuscript: ManuscriptSummary;
  contentHash: string;
  importedUnixMs: number;
  snapshotVersion: number;
}

type WorkspaceCreation =
  | { status: "created"; workspace: WorkspaceSummary }
  | { status: "rejected"; message: string };

interface WorkspaceCatalog {
  workspaces: WorkspaceSummary[];
  archivedWorkspaces: WorkspaceSummary[];
  warnings: string[];
}

type VersionOrigin = "imported" | "revision" | "restored";

interface ManuscriptVersionSummary {
  version: number;
  parentVersion: number | null;
  manuscript: ManuscriptSummary;
  contentHash: string;
  createdUnixMs: number;
  note: string;
  origin: VersionOrigin;
  restoredFromVersion: number | null;
}

interface VersionHistory {
  workspaceId: string;
  currentVersion: number;
  versions: ManuscriptVersionSummary[];
}

type VersionCreation =
  | { status: "created"; workspace: WorkspaceSummary; version: ManuscriptVersionSummary }
  | { status: "unchanged"; version: number; message: string };

interface VersionComparison {
  workspaceId: string;
  fromVersion: number;
  toVersion: number;
  identical: boolean;
  fromContentHash: string;
  toContentHash: string;
  titleBefore: string | null;
  titleAfter: string | null;
  wordCountDelta: number;
  figureCountDelta: number;
  tableCountDelta: number;
  addedSections: string[];
  removedSections: string[];
  addedDeclarations: string[];
  removedDeclarations: string[];
  externalTransmission: "not_performed";
}

type AnalysisQuality = "complete" | "limited";

interface SectionSummary {
  level: number;
  heading: string;
}
type SemanticElementKind = "claim" | "scope" | "method" | "result" | "evidence";
type SourceModality = "text" | "table" | "figure";
interface SemanticCandidate { element: SemanticElementKind; text: string; sourceLabel: string; sourceFragmentId?: string | null; modality: SourceModality; confidencePercent: number; }
interface ExtractedSourceFragment { fragmentId: string; text: string; sourceLabel: string; modality: SourceModality; }
interface ExtractionCoverage { textFragments: number; tableFragments: number; figureFragments: number; }

interface PdfProcessingSummary {
  classification: string;
  confidencePercent: number;
  nativeExtraction: string;
  pagesNeedingRecognition: number[];
  pagesWithTables: number[];
  pagesWithColumns: number[];
  hasEncodingIssues: boolean;
}

interface StructureReport {
  analysisVersion: number;
  workspaceId: string;
  sourceContentHash: string;
  sourceSnapshotVersion: number;
  quality: AnalysisQuality;
  title: string | null;
  authors: string[];
  abstractPresent: boolean;
  abstractText: string | null;
  keywordsPresent: boolean;
  sections: SectionSummary[];
  figureCount: number;
  tableCount: number;
  referencesPresent: boolean;
  declarations: string[];
  pageCount: number | null;
  wordCount: number;
  semanticCandidates: SemanticCandidate[];
  sourceFragments: ExtractedSourceFragment[];
  extractionCoverage: ExtractionCoverage;
  pdfProcessing?: PdfProcessingSummary;
  warnings: string[];
}

type ResearchTopic = "auto" | "general_ai" | "machine_learning" | "computer_vision" | "natural_language_processing" | "data_mining" | "software_systems" | "robotics_control";
type ArticleTypePreference = "auto" | "research" | "review" | "application";
type PublicationLanguagePreference = "auto" | "chinese" | "english";
type TargetStrategy = "reach" | "balanced" | "pragmatic";
type OpenAccessPreference = "no_preference" | "prefer" | "require";
type ManuscriptPurpose = "degree_requirement" | "graduation" | "professional_title" | "project_completion" | "academic_communication";
interface JournalRecommendationProfileInput { authorName: string; institution: string; specialty: string; manuscriptPurpose: ManuscriptPurpose; submissionDeadline: string; }
interface InstitutionRuleEvidence { status: "search_required" | "candidate_sources_found" | "verified" | "no_official_rule_found"; ruleSetId: string | null; ruleSetVersion: string | null; sourceUrls: string[]; verifiedAt: string | null; recognizedRankTiers: string[]; blockedRankTiers: string[]; sourceTextHash?: string | null; sourceKind?: string | null; extractionModel?: string | null; extractedConditions?: string[]; minimumCasPartition?: number | null; requiresCasTop?: boolean; authorAttestedOfficial?: boolean; casPartitionDataStatus?: string | null; }
interface JournalRecommendationProfile extends JournalRecommendationProfileInput { schemaVersion: number; profileId: string; profileVersion: number; workspaceId: string; savedUnixMs: number; institutionRuleEvidence: InstitutionRuleEvidence; externalTransmission: "not_performed"; }
interface JournalRecommendationProfileSummary { profileId: string; profileVersion: number; institution: string; specialty: string; manuscriptPurpose: ManuscriptPurpose; submissionDeadline: string; }
interface InstitutionRuleExtractionSummary { profileId: string; profileVersion: number; status: "verified" | "requires_verification" | "search_required" | "no_official_rule_found"; }
interface JournalMatchPreferences { topic: ResearchTopic; articleType: ArticleTypePreference; language: PublicationLanguagePreference; targetStrategy: TargetStrategy; openAccess: OpenAccessPreference; }
interface JournalFitScores { institutionRules: number | null; topicScope: number; specialtyFit: number; articleType: number; contentReadiness: number; language: number; targetLevel: number; openAccess: number; purposeFit: number; timeFeasibility: number; }
interface JournalRecommendation { id: string; name: string; nameEn: string; region: "domestic" | "international"; publisher: string; rankSystem: string; rankTier: string; overallFit: number; estimatedSubmissionPreparationDays: number; deadlineStatus: string; institutionEligibility: string; scores: JournalFitScores; reasons: string[]; rankingSourceUrl: string; homepageUrl: string; openAccessStatus: string; }
interface JournalRecommendationRun { schemaVersion: number; runId: string; workspaceId: string; manuscriptVersion: number; manuscriptHash: string; algorithmVersion: string; catalogVersion: string; catalogVerifiedDate: string; inferredTopic: ResearchTopic; topicBasis: string; maturityScore: number; evaluatedUnixMs: number; recommendationProfile: JournalRecommendationProfileSummary; deadlineDaysRemaining: number; preferences: JournalMatchPreferences; domestic: JournalRecommendation[]; international: JournalRecommendation[]; schoolRuleStatus: string; institutionDirectoryStatus: string; limitations: string[]; externalTransmission: string; }

type StructureAnalysis =
  | { status: "completed"; report: StructureReport }
  | { status: "rejected"; message: string };

type FindingStatus = "passed" | "warning" | "blocked" | "confirmation";
type ReadinessOutcome = "ready" | "needs_attention" | "blocked";

interface RuleFinding {
  ruleId: string;
  rulePackId: string;
  classification: "must" | "recommendation" | "author_confirmation";
  status: FindingStatus;
  message: string;
  messageEn?: string;
  sourceLocation: string;
}

interface RulePackReference {
  id: string;
  version: string;
  coverage: string;
  stage: string;
  sourceLabel: string;
  sourceLabelEn?: string;
  sourceUrls?: string[];
  verifiedAt?: string;
  signatureVerified: boolean;
}

interface RulePackCatalogItem {
  id: string;
  version: string;
  coverage: string;
  stage: string;
  region: string;
  category: string;
  sourceLabel: string;
  sourceLabelEn: string;
  description: string;
  descriptionEn: string;
  sourceUrls: string[];
  verifiedAt: string;
  signatureVerified: boolean;
}

interface RulePackCatalog { rulePacks: RulePackCatalogItem[]; }

type SubmissionElementRequirement = "required" | "recommended" | "author_confirmation";

interface SubmissionElementCatalogItem {
  id: string;
  group: string;
  label: string;
  labelEn: string;
  description: string;
  descriptionEn: string;
  requirement: SubmissionElementRequirement;
  editableField: string | null;
  rulePackIds: string[];
  sourceLabels: string[];
  sourceLabelsEn: string[];
  sourceUrls: string[];
}

interface SubmissionElementCatalog {
  elements: SubmissionElementCatalogItem[];
  rulePacks: RulePackReference[];
}

type RevisionFieldKind = "title" | "abstract" | "keywords";
interface RevisionField { field: RevisionFieldKind; label: string; labelEn: string; value: string; editable: boolean; limitation: string | null; limitationEn: string | null; }
interface RevisionDraft { workspaceId: string; baseVersion: number; format: string; fields: RevisionField[]; warnings: string[]; }
interface RevisionChange { field: RevisionFieldKind; before: string; after: string; basis: string; status: string; }
interface RevisionSet { revisionId: string; workspaceId: string; baseVersion: number; outputVersion: number; createdUnixMs: number; changes: RevisionChange[]; externalTransmission: "not_performed"; }
type RevisionApplication = { status: "created"; workspace: WorkspaceSummary; version: ManuscriptVersionSummary; revisionSet: RevisionSet } | { status: "unchanged"; version: number; message: string };

type KnowledgeObjectType = "knowledge_body" | "knowledge_body_snapshot" | "claim" | "proposition" | "scope" | "evidence" | "evidence_relation" | "source_anchor" | "status" | "method" | "result" | "artifact_version" | "ai_review_report" | "provenance" | "capability_contract" | "runtime_profile" | "rights_policy" | "reputation_record";
interface VersionedObjectReference { objectId: string; objectType: KnowledgeObjectType; version: number; }
type ElementState = "pending" | "candidate" | "established";
interface ClaimElementReference extends VersionedObjectReference { state: ElementState; }
interface ClaimFiveTuple { claim: VersionedObjectReference; proposition: ClaimElementReference; conditions: ClaimElementReference; evidence: ClaimElementReference; sources: ClaimElementReference; status: ClaimElementReference; }
interface KnowledgeBodyObjectSet { artifactVersion: VersionedObjectReference; claim: VersionedObjectReference; scope: VersionedObjectReference; method: VersionedObjectReference; result: VersionedObjectReference; evidenceRelation: VersionedObjectReference; sourceAnchor: VersionedObjectReference; aiReviewReport: VersionedObjectReference | null; provenance: VersionedObjectReference; knowledgeBodySnapshot: VersionedObjectReference; }
interface AiReviewReportVersion { reportId: string; version: number; previousVersion: number | null; reviewedClaim: VersionedObjectReference; reviewerId: string; reviewerVersion: string; createdUnixMs: number; status: string; summary: string; externalTransmission: string; }
interface AiReviewReportHistory { reportId: string; currentVersion: number | null; versions: AiReviewReportVersion[]; }
interface KnowledgeCandidateContent { candidateId: string; text: string; sourceLabel: string; sourceFragmentId: string | null; modality: SourceModality; confidencePercent: number; authorConfirmed: boolean; }
interface ExtractedKnowledgeElement { object: VersionedObjectReference; state: ElementState; candidates: KnowledgeCandidateContent[]; }
interface KnowledgeExtractionLayer { decompositionId: string; decompositionHash: string; analysisVersion: number; sourceSnapshotVersion: number; generatedBy: string; confirmationPolicy: string; claim: ExtractedKnowledgeElement; scope: ExtractedKnowledgeElement; method: ExtractedKnowledgeElement; result: ExtractedKnowledgeElement; evidence: ExtractedKnowledgeElement; }
type CapabilityAvailability = "available" | "requires_runtime" | "planned";
interface CapabilityContract { contractId: string; version: number; capability: string; inputContract: string[]; outputContract: string[]; preconditions: string[]; refusalConditions: string[]; evidenceSources: VersionedObjectReference[]; availability: CapabilityAvailability; }
interface KnowledgeBodyServiceArchitecture {
  identityAndVersion: { knowledgeBody: VersionedObjectReference; currentSnapshot: VersionedObjectReference; sourceArtifact: VersionedObjectReference; creatorProvenance: VersionedObjectReference; lifecycleStatus: string; supersedes: VersionedObjectReference | null; immutableHistory: boolean; };
  knowledgeBoundaryAndEvidence: { claims: VersionedObjectReference[]; scope: VersionedObjectReference; method: VersionedObjectReference; result: VersionedObjectReference; evidence: VersionedObjectReference; evidenceRelation: VersionedObjectReference; sourceAnchor: VersionedObjectReference; knownLimitations: string[]; unverifiedObjects: VersionedObjectReference[]; };
  capabilityContracts: CapabilityContract[];
  interactionRuntime: { runtimeProfile: VersionedObjectReference; bindingPolicy: "replaceable"; coordinatorRole: string; allowedTools: string[]; perCallAuthorization: boolean; externalTransmission: string; };
  validationRightsAndReputation: { validationRecords: VersionedObjectReference[]; rightsPolicy: VersionedObjectReference; reputationRecord: VersionedObjectReference; contentSnapshot: VersionedObjectReference; attributionRequired: boolean; reputationUpdatesIndependently: boolean; reuseControl: string; };
}
type KnowledgeBodyRole = "current_study" | "original_research" | "reproduction_research" | "competing_research" | "cross_domain_application" | "later_synthesis";
interface KnowledgeBodyNode { body: VersionedObjectReference; displayId: string; title: string; role: KnowledgeBodyRole; claim: VersionedObjectReference; sourceAnchor: VersionedObjectReference; method: VersionedObjectReference; }
type RelationKind = "citation" | "claim_relation" | "evidence_relation" | "method_transfer" | "reproduction" | "alignment" | "version_relation" | "classification";
interface NetworkAssertion { assertionId: string; version: number; relationKind: RelationKind; protocolObject: string; source: VersionedObjectReference; target: VersionedObjectReference; basis: Array<{ label: string; source: VersionedObjectReference }>; status: string; }
interface AcademicKnowledgeBodySnapshot { schemaVersion: number; knowledgeBodyId: string; snapshotVersion: number; manuscript: VersionedObjectReference; claim: ClaimFiveTuple; objects: KnowledgeBodyObjectSet; aiReviewReport: VersionedObjectReference | null; aiReviewHistory: AiReviewReportHistory; extraction?: KnowledgeExtractionLayer; serviceArchitecture?: KnowledgeBodyServiceArchitecture; network: { bodies: KnowledgeBodyNode[]; assertions: NetworkAssertion[]; supportedRelations: RelationKind[] }; externalTransmission: string; }

interface ReadinessReport {
  reportVersion: number;
  reportId: string;
  workspaceId: string;
  sourceContentHash: string;
  sourceSnapshotVersion: number;
  outputSnapshotVersion: number;
  generatedUnixMs: number;
  outcome: ReadinessOutcome;
  passedCount: number;
  warningCount: number;
  blockedCount: number;
  confirmationCount: number;
  findings: RuleFinding[];
  rulePacks: RulePackReference[];
  externalTransmission: "not_performed";
}

type ReadinessEvaluation =
  | { status: "completed"; report: ReadinessReport }
  | { status: "rejected"; message: string };

interface LocalAttestation {
  attestationId: string;
  workspaceId: string;
  manuscriptVersion: number;
  manuscriptHash: string;
  readinessReportId: string;
  readinessOutputSnapshotVersion: number;
  readinessOutcome: ReadinessOutcome;
  attestedUnixMs: number;
  statement: string;
  recordHash: string;
  externalTransmission: "not_performed";
}

interface SubmissionRecord {
  submissionId: string;
  workspaceId: string;
  manuscriptVersion: number;
  attestationId: string;
  target: string;
  receipt: string | null;
  submittedUnixMs: number;
  statement: string;
  recordHash: string;
  externalTransmission: "not_performed";
}

interface SubmissionExport {
  packageName: string;
  manuscriptVersion: number;
  attestationId: string;
  files: string[];
  exportedUnixMs: number;
  externalTransmission: "not_performed";
}

interface DisciplineCatalogItem {
  code: string;
  label: string;
  labelEn: string;
}

interface DisciplineClassification extends DisciplineCatalogItem {
  assignmentId: string;
  version: number;
  scheme: string;
  schemeVersion: string;
  status: "author_confirmed";
  basis: "author_selection";
}

interface KnowledgeBodyRecord {
  recordId: string;
  workspaceId: string;
  manuscriptVersion: number;
  attestationId: string;
  submissionId: string;
  finalizedUnixMs: number;
  disciplineClassification: DisciplineClassification | null;
  snapshot: AcademicKnowledgeBodySnapshot;
  recordHash: string;
  externalTransmission: "not_performed";
}

type KnowledgeInquiryOrigin = "owner" | "external";
type KnowledgeInquiryStance = "recognition" | "question" | "challenge";
type KnowledgeInquiryTarget = "knowledge_body" | "claim" | "scope" | "method" | "result" | "evidence_relation" | "source_anchor" | "ai_review_report" | "provenance" | "capability_contract" | "rights_reputation";

interface KnowledgeInquiryRecord {
  schemaVersion: number;
  inquiryId: string;
  workspaceId: string;
  knowledgeBodyRecordId: string;
  knowledgeBodyHash: string;
  snapshotVersion: number;
  origin: KnowledgeInquiryOrigin;
  stance: KnowledgeInquiryStance;
  target: KnowledgeInquiryTarget;
  question: string;
  externalActorLabel: string | null;
  createdUnixMs: number;
  recordHash: string;
  externalTransmission: string;
}

interface KnowledgeAnswerRecord {
  schemaVersion: number;
  answerId: string;
  inquiryId: string;
  workspaceId: string;
  knowledgeBodyRecordId: string;
  modelSlot: string;
  providerLabel: string;
  model: string;
  answer: string;
  sourceAnchors: VersionedObjectReference[];
  createdUnixMs: number;
  recordHash: string;
  externalTransmission: string;
}

interface KnowledgeDialogueItem { inquiry: KnowledgeInquiryRecord; answers: KnowledgeAnswerRecord[]; }
interface KnowledgeDialogueLedger { workspaceId: string; knowledgeBodyRecordId: string; knowledgeBodyHash: string; items: KnowledgeDialogueItem[]; }

type ModelSlotRole = "primary" | "fallback_1" | "fallback_2";
interface ModelSlotSummary { role: ModelSlotRole; enabled: boolean; providerLabel: string; baseUrl: string; model: string; hasApiKey: boolean; }
interface ModelSettingsSummary { schemaVersion: number; slots: ModelSlotSummary[]; secureStore: string; }
interface ModelSlotDraft extends ModelSlotSummary { apiKey: string; clearApiKey: boolean; }

interface WorkspaceLifecycle {
  workspaceId: string;
  currentVersion: number;
  structureReport: StructureReport | null;
  readinessReport: ReadinessReport | null;
  attestation: LocalAttestation | null;
  submission: SubmissionRecord | null;
  knowledgeBody: KnowledgeBodyRecord | null;
}

type SelectionState = "idle" | "selecting" | "selected" | "error";
type WorkspaceStage = "source" | "check" | "revision" | "versions" | "journals" | "attestation" | "submission" | "knowledge";
type MobilePane = "operation" | "evidence";
type IconName = "workspace" | "upload" | "lock" | "file" | "check" | "versions" | "structure" | "target" | "format" | "review" | "package" | "knowledge" | "arrow" | "warning" | "more" | "archive" | "trash" | "restore";

const WORKSPACE_STAGES: Array<{ id: WorkspaceStage; zh: string; en: string; shortZh: string; shortEn: string }> = [
  { id: "source", zh: "导入", en: "Import", shortZh: "导入", shortEn: "Import" },
  { id: "check", zh: "检查", en: "Check", shortZh: "检查", shortEn: "Check" },
  { id: "revision", zh: "修订", en: "Revise", shortZh: "修订", shortEn: "Revise" },
  { id: "versions", zh: "版本", en: "Version", shortZh: "版本", shortEn: "Version" },
  { id: "journals", zh: "期刊匹配", en: "Journal Match", shortZh: "期刊", shortEn: "Match" },
  { id: "attestation", zh: "存证", en: "Attest", shortZh: "存证", shortEn: "Attest" },
  { id: "submission", zh: "投稿", en: "Submit", shortZh: "投稿", shortEn: "Submit" },
  { id: "knowledge", zh: "知识体", en: "Knowledge Body", shortZh: "知识", shortEn: "Knowledge" },
];

const GOLDEN_RATIO = (1 + Math.sqrt(5)) / 2;
const INVERSE_GOLDEN_RATIO = 1 / GOLDEN_RATIO;
const DODECAHEDRON_VERTICES: Array<[number, number, number]> = [
  [-1, -1, -1], [-1, -1, 1], [-1, 1, -1], [-1, 1, 1],
  [1, -1, -1], [1, -1, 1], [1, 1, -1], [1, 1, 1],
  [0, -INVERSE_GOLDEN_RATIO, -GOLDEN_RATIO], [0, -INVERSE_GOLDEN_RATIO, GOLDEN_RATIO],
  [0, INVERSE_GOLDEN_RATIO, -GOLDEN_RATIO], [0, INVERSE_GOLDEN_RATIO, GOLDEN_RATIO],
  [-INVERSE_GOLDEN_RATIO, -GOLDEN_RATIO, 0], [-INVERSE_GOLDEN_RATIO, GOLDEN_RATIO, 0],
  [INVERSE_GOLDEN_RATIO, -GOLDEN_RATIO, 0], [INVERSE_GOLDEN_RATIO, GOLDEN_RATIO, 0],
  [-GOLDEN_RATIO, 0, -INVERSE_GOLDEN_RATIO], [-GOLDEN_RATIO, 0, INVERSE_GOLDEN_RATIO],
  [GOLDEN_RATIO, 0, -INVERSE_GOLDEN_RATIO], [GOLDEN_RATIO, 0, INVERSE_GOLDEN_RATIO],
];
const DODECAHEDRON_EDGE_LENGTH = 2 / GOLDEN_RATIO;
const DODECAHEDRON_EDGES = DODECAHEDRON_VERTICES.flatMap((from, fromIndex) => DODECAHEDRON_VERTICES.slice(fromIndex + 1).flatMap((to, offset) => {
  const distance = Math.hypot(from[0] - to[0], from[1] - to[1], from[2] - to[2]);
  return Math.abs(distance - DODECAHEDRON_EDGE_LENGTH) < 0.001 ? [[fromIndex, fromIndex + offset + 1] as const] : [];
}));

function Icon({ name }: { name: IconName }) {
  const paths: Record<IconName, ReactNode> = {
    workspace: <><path d="m3 10 9-7 9 7" /><path d="M5 9v11h14V9" /><path d="M9 20v-6h6v6" /></>,
    upload: <><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="17 8 12 3 7 8" /><line x1="12" y1="3" x2="12" y2="15" /></>,
    lock: <><rect x="5.5" y="10" width="13" height="10" rx="2" /><path d="M8.5 10V7.5a3.5 3.5 0 0 1 7 0V10" /></>,
    file: <><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v4a2 2 0 0 0 2 2h4" /><path d="M10 9H8M16 13H8M16 17H8" /></>,
    check: <><circle cx="12" cy="12" r="9" /><path d="m8 12 2.5 2.5L16 9" /></>,
    versions: <><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /><path d="M5.6 5.6 3 8.2" /></>,
    structure: <><path d="M8 6h13M8 12h13M8 18h13" /><circle cx="4" cy="6" r="1" /><circle cx="4" cy="12" r="1" /><circle cx="4" cy="18" r="1" /></>,
    target: <><circle cx="12" cy="12" r="10" /><line x1="2" y1="12" x2="22" y2="12" /><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" /></>,
    format: <><path d="M3 6h18M3 12h14M3 18h10" /></>,
    review: <><rect width="8" height="4" x="8" y="2" rx="1" /><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" /><path d="m9 14 2 2 4-4" /></>,
    package: <><path d="m7.5 4.27 9 5.15" /><path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z" /><path d="M3.3 7 12 12l8.7-5M12 22V12" /></>,
    knowledge: <><circle cx="18" cy="5" r="3" /><circle cx="6" cy="12" r="3" /><circle cx="18" cy="19" r="3" /><path d="m8.6 10.5 6.8-4M8.6 13.5l6.8 4" /></>,
    arrow: <><path d="M5 12h14M14 7l5 5-5 5" /></>,
    warning: <><path d="M12 4 3.8 19h16.4z" /><path d="M12 9v4M12 16.5v.1" /></>,
    more: <><circle cx="5" cy="12" r="1" /><circle cx="12" cy="12" r="1" /><circle cx="19" cy="12" r="1" /></>,
    archive: <><path d="M4 7h16v13H4z" /><path d="M3 3h18v4H3z" /><path d="M9 11h6" /></>,
    trash: <><path d="M4 7h16M9 7V4h6v3M7 7l1 14h8l1-14M10 11v6M14 11v6" /></>,
    restore: <><path d="M4 4v6h6" /><path d="M5.5 15a7 7 0 1 0 .5-7" /><path d="M12 9v4l3 2" /></>,
  };
  return <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">{paths[name]}</svg>;
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)} ${units[unitIndex]}`;
}

function formatModifiedDate(timestamp: number | null, locale: Locale) {
  if (timestamp === null) return localize(locale, "未知", "Unknown");
  return new Intl.DateTimeFormat(locale, { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" }).format(new Date(timestamp));
}

function normalizeError(error: unknown) {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "无法打开该文件。请确认文件可访问后重试。";
}

function getStageIcon(stage: WorkspaceStage): IconName {
  if (stage === "source") return "file";
  if (stage === "check") return "review";
  if (stage === "revision") return "format";
  if (stage === "versions") return "versions";
  if (stage === "journals") return "target";
  if (stage === "attestation") return "lock";
  if (stage === "submission") return "package";
  return "knowledge";
}

function findingLabel(status: FindingStatus, locale: Locale) {
  if (status === "passed") return localize(locale, "通过", "Passed");
  if (status === "blocked") return localize(locale, "阻断", "Blocked");
  if (status === "warning") return localize(locale, "建议", "Suggestion");
  return localize(locale, "作者确认", "Author confirmation");
}

function outcomeLabel(outcome: ReadinessOutcome, locale: Locale) {
  if (outcome === "ready") return localize(locale, "已具备基础投稿条件", "Basic submission requirements met");
  if (outcome === "blocked") return localize(locale, "存在阻断项", "Blocking issues found");
  return localize(locale, "仍有事项需要处理", "Items still need attention");
}

export default function App() {
  return <I18nProvider><ManuscriptDockApp /></I18nProvider>;
}

function ManuscriptDockApp() {
  const { locale, text } = useI18n();
  const [selectionState, setSelectionState] = useState<SelectionState>("idle");
  const [selectionId, setSelectionId] = useState<string | null>(null);
  const [manuscript, setManuscript] = useState<ManuscriptSummary | null>(null);
  const [activeWorkspace, setActiveWorkspace] = useState<WorkspaceSummary | null>(null);
  const [recentWorkspaces, setRecentWorkspaces] = useState<WorkspaceSummary[]>([]);
  const [archivedWorkspaces, setArchivedWorkspaces] = useState<WorkspaceSummary[]>([]);
  const [catalogWarnings, setCatalogWarnings] = useState<string[]>([]);
  const [workspaceManagementBusyId, setWorkspaceManagementBusyId] = useState<string | null>(null);
  const [workspaceManagementNotice, setWorkspaceManagementNotice] = useState<string | null>(null);
  const [workspaceManagementError, setWorkspaceManagementError] = useState<string | null>(null);
  const [isCreatingWorkspace, setIsCreatingWorkspace] = useState(false);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [isEvaluating, setIsEvaluating] = useState(false);
  const [isLoadingRuleCatalog, setIsLoadingRuleCatalog] = useState(false);
  const [structureReport, setStructureReport] = useState<StructureReport | null>(null);
  const [readinessReport, setReadinessReport] = useState<ReadinessReport | null>(null);
  const [ruleCatalog, setRuleCatalog] = useState<RulePackCatalogItem[]>([]);
  const [selectedRulePackIds, setSelectedRulePackIds] = useState<string[]>([]);
  const [submissionElementCatalog, setSubmissionElementCatalog] = useState<SubmissionElementCatalog | null>(null);
  const [isLoadingSubmissionElements, setIsLoadingSubmissionElements] = useState(false);
  const [revisionDraft, setRevisionDraft] = useState<RevisionDraft | null>(null);
  const [revisionValues, setRevisionValues] = useState<Record<string, string>>({});
  const [revisionResult, setRevisionResult] = useState<RevisionSet | null>(null);
  const [isApplyingRevision, setIsApplyingRevision] = useState(false);
  const [versionHistory, setVersionHistory] = useState<VersionHistory | null>(null);
  const [selectedVersion, setSelectedVersion] = useState<number | null>(null);
  const [versionComparison, setVersionComparison] = useState<VersionComparison | null>(null);
  const [versionSelectionId, setVersionSelectionId] = useState<string | null>(null);
  const [versionCandidate, setVersionCandidate] = useState<ManuscriptSummary | null>(null);
  const [versionNote, setVersionNote] = useState("");
  const [versionNotice, setVersionNotice] = useState<string | null>(null);
  const [isSelectingVersion, setIsSelectingVersion] = useState(false);
  const [isSavingVersion, setIsSavingVersion] = useState(false);
  const [isRestoringVersion, setIsRestoringVersion] = useState(false);
  const [isComparingVersions, setIsComparingVersions] = useState(false);
  const [knowledgeBodySnapshot, setKnowledgeBodySnapshot] = useState<AcademicKnowledgeBodySnapshot | null>(null);
  const [knowledgeBodyRecord, setKnowledgeBodyRecord] = useState<KnowledgeBodyRecord | null>(null);
  const [disciplineCatalog, setDisciplineCatalog] = useState<DisciplineCatalogItem[]>([]);
  const [selectedDisciplineCode, setSelectedDisciplineCode] = useState("");
  const [isLoadingDisciplineCatalog, setIsLoadingDisciplineCatalog] = useState(false);
  const [attestation, setAttestation] = useState<LocalAttestation | null>(null);
  const [submission, setSubmission] = useState<SubmissionRecord | null>(null);
  const [submissionExport, setSubmissionExport] = useState<SubmissionExport | null>(null);
  const [attestationConfirmed, setAttestationConfirmed] = useState(false);
  const [submissionConfirmed, setSubmissionConfirmed] = useState(false);
  const [submissionTarget, setSubmissionTarget] = useState("");
  const [submissionReceipt, setSubmissionReceipt] = useState("");
  const [isLoadingLifecycle, setIsLoadingLifecycle] = useState(false);
  const [isAttesting, setIsAttesting] = useState(false);
  const [isExportingSubmission, setIsExportingSubmission] = useState(false);
  const [isRecordingSubmission, setIsRecordingSubmission] = useState(false);
  const [isFinalizingKnowledge, setIsFinalizingKnowledge] = useState(false);
  const [isLoadingKnowledgeBody, setIsLoadingKnowledgeBody] = useState(false);
  const [activeStage, setActiveStage] = useState<WorkspaceStage>("source");
  const [mobilePane, setMobilePane] = useState<MobilePane>("operation");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;
    void invoke<WorkspaceCatalog>("list_workspaces")
      .then((catalog) => { setRecentWorkspaces(catalog.workspaces); setArchivedWorkspaces(catalog.archivedWorkspaces ?? []); setCatalogWarnings(catalog.warnings); })
      .catch(() => { setCatalogWarnings([text("最近的本地工作区暂时无法读取", "Recent local workspaces could not be loaded")]); });
  }, []);

  function resetVersionState() {
    setVersionHistory(null);
    setSelectedVersion(null);
    setVersionComparison(null);
    setVersionSelectionId(null);
    setVersionCandidate(null);
    setVersionNote("");
    setVersionNotice(null);
  }

  function resetKnowledgeBodyState() {
    setKnowledgeBodySnapshot(null);
    setKnowledgeBodyRecord(null);
    setSelectedDisciplineCode("");
    setIsLoadingKnowledgeBody(false);
  }

  function resetDownstreamLifecycle() {
    setAttestation(null);
    setSubmission(null);
    setSubmissionExport(null);
    setAttestationConfirmed(false);
    setSubmissionConfirmed(false);
    setSubmissionTarget("");
    setSubmissionReceipt("");
    resetKnowledgeBodyState();
  }

  function hydrateLifecycle(workspace: WorkspaceSummary) {
    setIsLoadingLifecycle(true);
    setErrorMessage(null);
    void invoke<WorkspaceLifecycle>("get_workspace_lifecycle", { workspaceId: workspace.id })
      .then((lifecycle) => {
        setStructureReport(lifecycle.structureReport);
        setReadinessReport(lifecycle.readinessReport);
        setAttestation(lifecycle.attestation);
        setSubmission(lifecycle.submission);
        setSubmissionTarget(lifecycle.submission?.target ?? "");
        setSubmissionReceipt(lifecycle.submission?.receipt ?? "");
        setKnowledgeBodyRecord(lifecycle.knowledgeBody);
        setKnowledgeBodySnapshot(lifecycle.knowledgeBody?.snapshot ?? null);
        setSelectedDisciplineCode(lifecycle.knowledgeBody?.disciplineClassification?.code ?? "");
        if (lifecycle.readinessReport) {
          setSelectedRulePackIds(lifecycle.readinessReport.rulePacks
            .map((pack) => pack.id)
            .filter((id) => id !== "core-structure-v1" && id !== "initial-submission-v1"));
        }
      })
      .catch((error: unknown) => setErrorMessage(normalizeError(error)))
      .finally(() => setIsLoadingLifecycle(false));
  }

  function loadVersionHistory(workspace: WorkspaceSummary, compareFrom?: number) {
    setErrorMessage(null);
    void invoke<VersionHistory>("get_version_history", { workspaceId: workspace.id })
      .then((history) => {
        setVersionHistory(history);
        const version = compareFrom ?? (history.currentVersion > 1 ? history.currentVersion - 1 : history.currentVersion);
        setSelectedVersion(version);
        if (version !== history.currentVersion) compareVersions(workspace, version, history.currentVersion);
        else setVersionComparison(null);
      })
      .catch((error: unknown) => setErrorMessage(normalizeError(error)));
  }

  function compareVersions(workspace: WorkspaceSummary, fromVersion: number, toVersion: number) {
    setSelectedVersion(fromVersion);
    if (fromVersion === toVersion) {
      setVersionComparison(null);
      return;
    }
    setIsComparingVersions(true);
    setErrorMessage(null);
    void invoke<VersionComparison>("compare_manuscript_versions", { workspaceId: workspace.id, fromVersion, toVersion })
      .then(setVersionComparison)
      .catch((error: unknown) => setErrorMessage(normalizeError(error)))
      .finally(() => setIsComparingVersions(false));
  }

  function selectVersionCandidate() {
    if (isSelectingVersion) return;
    setIsSelectingVersion(true);
    setVersionNotice(null);
    setErrorMessage(null);
    void invoke<ManuscriptSelection>("select_manuscript")
      .then((result) => {
        if (result.status === "selected") {
          setVersionSelectionId(result.selectionId);
          setVersionCandidate(result.manuscript);
        } else if (result.status === "rejected") setErrorMessage(result.message);
      })
      .catch((error: unknown) => setErrorMessage(normalizeError(error)))
      .finally(() => setIsSelectingVersion(false));
  }

  function saveVersion() {
    if (!activeWorkspace || !versionSelectionId || isSavingVersion) return;
    const previousVersion = activeWorkspace.snapshotVersion;
    setIsSavingVersion(true);
    setVersionNotice(null);
    setErrorMessage(null);
    void invoke<VersionCreation>("save_manuscript_version", {
      workspaceId: activeWorkspace.id,
      selectionId: versionSelectionId,
      note: versionNote,
    })
      .then((result) => {
        setVersionSelectionId(null);
        setVersionCandidate(null);
        setVersionNote("");
        if (result.status === "unchanged") {
          setVersionNotice(localizeBackendText(locale, result.message));
          return;
        }
        setActiveWorkspace(result.workspace);
        setRecentWorkspaces((current) => [result.workspace, ...current.filter((workspace) => workspace.id !== result.workspace.id)]);
        setStructureReport(null);
        setReadinessReport(null);
        resetDownstreamLifecycle();
        setVersionNotice(text(`已保存版本 v${result.version.version}`, `Version v${result.version.version} saved`));
        loadVersionHistory(result.workspace, previousVersion);
      })
      .catch((error: unknown) => setErrorMessage(normalizeError(error)))
      .finally(() => setIsSavingVersion(false));
  }

  function restoreVersion(version: number) {
    if (!activeWorkspace || isRestoringVersion || version === activeWorkspace.snapshotVersion) return;
    const previousVersion = activeWorkspace.snapshotVersion;
    setIsRestoringVersion(true);
    setVersionNotice(null);
    setErrorMessage(null);
    void invoke<VersionCreation>("restore_manuscript_version", { workspaceId: activeWorkspace.id, version })
      .then((result) => {
        if (result.status === "unchanged") {
          setVersionNotice(localizeBackendText(locale, result.message));
          return;
        }
        setActiveWorkspace(result.workspace);
        setRecentWorkspaces((current) => [result.workspace, ...current.filter((workspace) => workspace.id !== result.workspace.id)]);
        setStructureReport(null);
        setReadinessReport(null);
        resetDownstreamLifecycle();
        setVersionNotice(text(`已将 v${version} 恢复为新的 v${result.version.version}`, `Restored v${version} as new v${result.version.version}`));
        loadVersionHistory(result.workspace, previousVersion);
      })
      .catch((error: unknown) => setErrorMessage(normalizeError(error)))
      .finally(() => setIsRestoringVersion(false));
  }

  function selectManuscript() {
    setSelectionState("selecting");
    setErrorMessage(null);
    void Promise.resolve()
      .then(() => invoke<ManuscriptSelection>("select_manuscript"))
      .then((result) => {
        if (result.status === "selected") {
          setSelectionId(result.selectionId);
          setManuscript(result.manuscript);
          setActiveWorkspace(null);
          setStructureReport(null);
          setReadinessReport(null);
          setSelectedRulePackIds([]);
          setSubmissionElementCatalog(null);
          setRevisionDraft(null); setRevisionValues({}); setRevisionResult(null);
          resetVersionState();
          resetDownstreamLifecycle();
          setActiveStage("source");
          setSelectionState("selected");
        } else if (result.status === "cancelled") {
          setSelectionState(manuscript ? "selected" : "idle");
        } else {
          setErrorMessage(result.message);
          setSelectionState("error");
        }
      })
      .catch((error: unknown) => { setErrorMessage(normalizeError(error)); setSelectionState("error"); });
  }

  function createWorkspace() {
    if (!selectionId || isCreatingWorkspace) return;
    setIsCreatingWorkspace(true);
    setErrorMessage(null);
    void invoke<WorkspaceCreation>("create_workspace", { selectionId })
      .then((result) => {
        if (result.status === "created") {
          setActiveWorkspace(result.workspace);
          setStructureReport(null);
          setReadinessReport(null);
          setSelectedRulePackIds([]);
          setSubmissionElementCatalog(null);
          setRevisionDraft(null); setRevisionValues({}); setRevisionResult(null);
          resetVersionState();
          resetDownstreamLifecycle();
          setActiveStage("source");
          setMobilePane("operation");
          setRecentWorkspaces((current) => [result.workspace, ...current.filter((workspace) => workspace.id !== result.workspace.id)]);
          setSelectionId(null);
        } else {
          setErrorMessage(result.message);
          setSelectionState("error");
        }
      })
      .catch((error: unknown) => { setErrorMessage(normalizeError(error)); setSelectionState("error"); })
      .finally(() => setIsCreatingWorkspace(false));
  }

  function analyzeWorkspace() {
    if (!activeWorkspace || isAnalyzing) return;
    setIsAnalyzing(true);
    setErrorMessage(null);
    void invoke<StructureAnalysis>("analyze_workspace", { workspaceId: activeWorkspace.id })
      .then((result) => {
        if (result.status === "completed") {
          setStructureReport(result.report);
          setReadinessReport(null);
          resetDownstreamLifecycle();
          setActiveStage("check");
        } else setErrorMessage(result.message);
      })
      .catch((error: unknown) => setErrorMessage(normalizeError(error)))
      .finally(() => setIsAnalyzing(false));
  }

  function evaluateReadiness() {
    if (!activeWorkspace || isEvaluating) return;
    setIsEvaluating(true);
    setErrorMessage(null);
    void invoke<ReadinessEvaluation>("evaluate_readiness", { workspaceId: activeWorkspace.id, rulePackIds: selectedRulePackIds })
      .then((result) => {
        if (result.status === "completed") {
          setReadinessReport(result.report);
          resetDownstreamLifecycle();
          setActiveStage("check");
        } else setErrorMessage(result.message);
      })
      .catch((error: unknown) => setErrorMessage(normalizeError(error)))
      .finally(() => setIsEvaluating(false));
  }

  function openRecentWorkspace(workspace: WorkspaceSummary) {
    setActiveWorkspace(workspace);
    setManuscript(null);
    setSelectionId(null);
    setStructureReport(null);
    setReadinessReport(null);
    setSelectedRulePackIds([]);
    setSubmissionElementCatalog(null);
    setRevisionDraft(null); setRevisionValues({}); setRevisionResult(null);
    resetVersionState();
    resetDownstreamLifecycle();
    setActiveStage("source");
    setMobilePane("operation");
    setErrorMessage(null);
    setSelectionState("idle");
    hydrateLifecycle(workspace);
  }

  async function manageWorkspace(action: "archive" | "restore" | "delete", workspace: WorkspaceSummary, archived: boolean) {
    if (workspaceManagementBusyId) return false;
    setWorkspaceManagementBusyId(workspace.id);
    setWorkspaceManagementNotice(null);
    setWorkspaceManagementError(null);
    try {
      const command = action === "archive" ? "archive_workspace" : action === "restore" ? "restore_workspace" : "delete_workspace";
      const catalog = await invoke<WorkspaceCatalog>(command, action === "delete"
        ? { workspaceId: workspace.id, archived, authorConfirmed: true }
        : { workspaceId: workspace.id });
      setRecentWorkspaces(catalog.workspaces);
      setArchivedWorkspaces(catalog.archivedWorkspaces ?? []);
      setCatalogWarnings(catalog.warnings);
      setWorkspaceManagementNotice(action === "archive"
        ? text(`已归档《${workspace.manuscript.name}》`, `Archived “${workspace.manuscript.name}”`)
        : action === "restore"
          ? text(`已恢复《${workspace.manuscript.name}》`, `Restored “${workspace.manuscript.name}”`)
          : text(`已永久删除《${workspace.manuscript.name}》`, `Permanently deleted “${workspace.manuscript.name}”`));
      return true;
    } catch (error) {
      setWorkspaceManagementError(localizeBackendText(locale, normalizeError(error)));
      return false;
    } finally {
      setWorkspaceManagementBusyId(null);
    }
  }

  function openWorkspaceHome() {
    if (!activeWorkspace) return;
    setActiveWorkspace(null);
    setManuscript(null);
    setSelectionId(null);
    setActiveStage("source");
    setMobilePane("operation");
    setErrorMessage(null);
    setSelectionState("idle");
  }

  function openStage(stage: WorkspaceStage) {
    setActiveStage(stage);
    setMobilePane("operation");
    if (stage === "check" && ruleCatalog.length === 0 && !isLoadingRuleCatalog) {
      setIsLoadingRuleCatalog(true);
      setErrorMessage(null);
      void invoke<RulePackCatalog>("list_rule_packs")
        .then((catalog) => setRuleCatalog(catalog.rulePacks))
        .catch((error: unknown) => setErrorMessage(normalizeError(error)))
        .finally(() => setIsLoadingRuleCatalog(false));
    }
    if (stage === "versions" && activeWorkspace && !versionHistory) {
      loadVersionHistory(activeWorkspace);
    }
    if (stage === "revision" && !isLoadingSubmissionElements) {
      setIsLoadingSubmissionElements(true);
      setErrorMessage(null);
      void invoke<SubmissionElementCatalog>("list_submission_elements", { rulePackIds: selectedRulePackIds })
        .then(setSubmissionElementCatalog)
        .catch((error: unknown) => setErrorMessage(normalizeError(error)))
        .finally(() => setIsLoadingSubmissionElements(false));
      if (activeWorkspace && !revisionDraft) {
        void invoke<RevisionDraft>("get_revision_draft", { workspaceId: activeWorkspace.id })
          .then((draft) => { setRevisionDraft(draft); setRevisionValues(Object.fromEntries(draft.fields.map((field) => [field.field, field.value]))); })
          .catch((error: unknown) => setErrorMessage(normalizeError(error)));
      }
    }
    if (stage === "knowledge" && structureReport && activeWorkspace && !isLoadingKnowledgeBody) {
      setIsLoadingKnowledgeBody(true);
      setErrorMessage(null);
      void invoke<AcademicKnowledgeBodySnapshot>("get_knowledge_body_snapshot", { workspaceId: activeWorkspace.id })
        .then(setKnowledgeBodySnapshot)
        .catch((error: unknown) => setErrorMessage(normalizeError(error)))
        .finally(() => setIsLoadingKnowledgeBody(false));
    }
    if (stage === "knowledge" && disciplineCatalog.length === 0 && !isLoadingDisciplineCatalog) {
      setIsLoadingDisciplineCatalog(true);
      setErrorMessage(null);
      void invoke<DisciplineCatalogItem[]>("list_discipline_index")
        .then(setDisciplineCatalog)
        .catch((error: unknown) => setErrorMessage(normalizeError(error)))
        .finally(() => setIsLoadingDisciplineCatalog(false));
    }
  }

  function toggleRulePack(rulePackId: string) {
    setSelectedRulePackIds((current) => current.includes(rulePackId)
      ? current.filter((id) => id !== rulePackId)
      : [...current, rulePackId]);
    setSubmissionElementCatalog(null);
    setReadinessReport(null);
    resetDownstreamLifecycle();
  }

  async function applyRevision() {
    if (!activeWorkspace || !revisionDraft || isApplyingRevision) return;
    const changes = revisionDraft.fields.filter((field) => (revisionValues[field.field] ?? field.value).trim() !== field.value).map((field) => ({ field: field.field, after: revisionValues[field.field] ?? field.value }));
    if (changes.length === 0) return;
    setIsApplyingRevision(true);
    setErrorMessage(null);
    try {
      const result = await invoke<RevisionApplication>("apply_manuscript_revision", { workspaceId: activeWorkspace.id, baseVersion: revisionDraft.baseVersion, changes });
      if (result.status === "unchanged") {
        setVersionNotice(localizeBackendText(locale, result.message));
        return;
      }
      setActiveWorkspace(result.workspace);
      setRecentWorkspaces((current) => [result.workspace, ...current.filter((workspace) => workspace.id !== result.workspace.id)]);
      setRevisionResult(result.revisionSet);
      setStructureReport(null);
      setReadinessReport(null);
      setVersionHistory(null);
      resetDownstreamLifecycle();
      const structure = await invoke<StructureAnalysis>("analyze_workspace", { workspaceId: result.workspace.id });
      if (structure.status !== "completed") throw new Error(structure.message);
      setStructureReport(structure.report);
      const readiness = await invoke<ReadinessEvaluation>("evaluate_readiness", { workspaceId: result.workspace.id, rulePackIds: selectedRulePackIds });
      if (readiness.status !== "completed") throw new Error(readiness.message);
      setReadinessReport(readiness.report);
      const draft = await invoke<RevisionDraft>("get_revision_draft", { workspaceId: result.workspace.id });
      setRevisionDraft(draft);
      setRevisionValues(Object.fromEntries(draft.fields.map((field) => [field.field, field.value])));
      setVersionNotice(text(`已保存 v${result.version.version}，并完成当前版本复查`, `Saved v${result.version.version} and rechecked the current version`));
      loadVersionHistory(result.workspace, result.revisionSet.baseVersion);
      setActiveStage("versions");
    } catch (error) {
      setErrorMessage(normalizeError(error));
    } finally {
      setIsApplyingRevision(false);
    }
  }

  async function createAttestation() {
    if (!activeWorkspace || !attestationConfirmed || isAttesting) return;
    setIsAttesting(true); setErrorMessage(null);
    try {
      const record = await invoke<LocalAttestation>("create_local_attestation", { workspaceId: activeWorkspace.id, authorConfirmed: true });
      setAttestation(record);
      setAttestationConfirmed(false);
    } catch (error) { setErrorMessage(normalizeError(error)); }
    finally { setIsAttesting(false); }
  }

  async function exportSubmission() {
    if (!activeWorkspace || isExportingSubmission) return;
    setIsExportingSubmission(true); setErrorMessage(null);
    try {
      const result = await invoke<SubmissionExport | null>("export_submission_package", { workspaceId: activeWorkspace.id });
      if (result) setSubmissionExport(result);
    } catch (error) { setErrorMessage(normalizeError(error)); }
    finally { setIsExportingSubmission(false); }
  }

  async function recordSubmission() {
    if (!activeWorkspace || !submissionConfirmed || !submissionTarget.trim() || isRecordingSubmission) return;
    setIsRecordingSubmission(true); setErrorMessage(null);
    try {
      const record = await invoke<SubmissionRecord>("record_manual_submission", { workspaceId: activeWorkspace.id, target: submissionTarget, receipt: submissionReceipt.trim() || null, authorConfirmed: true });
      setSubmission(record);
      setSubmissionConfirmed(false);
    } catch (error) { setErrorMessage(normalizeError(error)); }
    finally { setIsRecordingSubmission(false); }
  }

  async function finalizeKnowledgeBody() {
    if (!activeWorkspace || !selectedDisciplineCode || isFinalizingKnowledge) return;
    setIsFinalizingKnowledge(true); setErrorMessage(null);
    try {
      const record = await invoke<KnowledgeBodyRecord>("finalize_knowledge_body", { workspaceId: activeWorkspace.id, disciplineCode: selectedDisciplineCode });
      setKnowledgeBodyRecord(record);
      setKnowledgeBodySnapshot(record.snapshot);
    } catch (error) { setErrorMessage(normalizeError(error)); }
    finally { setIsFinalizingKnowledge(false); }
  }

  const isSelecting = selectionState === "selecting";

  if (!activeWorkspace) {
    return (
      <div className="app-shell landing-shell">
        <a className="skip-link" href="#main-content">{text("跳到主要内容", "Skip to main content")}</a>
        <ProductBar />
        <div className="landing-review-layout">
          <nav className="task-rail landing-rail" aria-label={text("工作台导航", "Workspace navigation")}>
            <button className="rail-button rail-workspace-button" type="button" aria-current="page" aria-label={text("我的工作台", "My Workspace")} title={text("我的工作台", "My Workspace")} onClick={openWorkspaceHome}><Icon name="workspace" /></button>
            <div className="rail-divider" role="separator" aria-orientation="horizontal" />
            <button className="rail-button" type="button" aria-label={text("导入论文", "Import manuscript")} title={text("导入论文", "Import manuscript")} onClick={selectManuscript}><Icon name="upload" /></button>
            {(["check", "revision", "versions", "journals", "attestation", "submission", "knowledge"] as WorkspaceStage[]).map((stage) => { const item = WORKSPACE_STAGES.find((candidate) => candidate.id === stage); return <button key={stage} className="rail-button" type="button" aria-label={item ? localize(locale, item.zh, item.en) : undefined} title={text("创建工作区后可用", "Available after creating a workspace")} disabled><Icon name={getStageIcon(stage)} /></button>; })}
          </nav>

          <main id="main-content" className="landing-main">
            <header className="landing-workspace-head"><h1 id="page-title">{text("我的工作台", "My Workspace")}</h1></header>
            <div className="landing-content">
              <section className="brand-statement" aria-labelledby="brand-statement-title">
                <h2 id="brand-statement-title" className="brand-statement-title">
                  <span lang="zh-CN">投稿舱</span>
                  <span lang="en">ManuscriptDock</span>
                </h2>
                <div className="brand-positioning">
                  <p lang="zh-CN">本地论文投稿准备工作台</p>
                  <p lang="en">Local-first manuscript submission workspace.</p>
                </div>
                <div className="brand-slogan">
                  <p lang="zh-CN">投论文，上更好的期刊</p>
                  <p lang="en">Go for Better Journals.</p>
                </div>
              </section>
              <section className="intake-panel" aria-label={text("选择论文稿件", "Select a manuscript")}>
                {!manuscript ? (
                  <div className="intake-empty">
                    <div className="section-heading compact-heading"><div><p className="field-kicker">{text("本地投稿准备", "Local submission preparation")}</p><h2>{text("导入论文稿件", "Import a manuscript")}</h2></div><span>DOCX · PDF · TEX</span></div>
                    <p className="intake-copy">{text("建立不可变源快照，再逐步完成结构提取、规则检查与投稿包准备。", "Create an immutable source snapshot, then prepare structure, checks, and the submission package step by step.")}</p>
                    <button className="primary-button" type="button" onClick={selectManuscript} disabled={isSelecting}><Icon name="upload" />{isSelecting ? text("正在打开…", "Opening…") : text("选择论文", "Select manuscript")}</button>
                    <div className="boundary-line"><Icon name="lock" /><span>{text("没有文件会在此阶段上传", "No files are uploaded at this stage")}</span></div>
                  </div>
                ) : (
                  <div className="intake-selected">
                    <div className="file-heading">
                      <span className="file-glyph" aria-hidden="true"><Icon name="file" /></span>
                      <div><p className="status-line"><Icon name="check" /> {text("本地校验完成", "Local validation complete")}</p><h2>{manuscript.name}</h2><p>{manuscript.extension.toUpperCase()} · {formatBytes(manuscript.sizeBytes)}</p></div>
                      <button className="text-button" type="button" onClick={selectManuscript} disabled={isSelecting}>{isSelecting ? text("正在打开…", "Opening…") : text("重新选择", "Choose another")}</button>
                    </div>
                    <dl className="summary-grid">
                      <div><dt>{text("文件格式", "Format")}</dt><dd>{manuscript.extension.toUpperCase()}</dd></div>
                      <div><dt>{text("文件大小", "Size")}</dt><dd>{formatBytes(manuscript.sizeBytes)}</dd></div>
                      <div><dt>{text("最近修改", "Last modified")}</dt><dd>{formatModifiedDate(manuscript.modifiedUnixMs, locale)}</dd></div>
                      <div><dt>{text("数据位置", "Data location")}</dt><dd>{text("当前设备", "This device")}</dd></div>
                    </dl>
                    <div className="intake-action-row">
                      <div><span className="field-kicker">{text("创建工作区", "Create workspace")}</span><h3>{text("保存不可变的本地源快照", "Save an immutable local source snapshot")}</h3><p>{text("复制并计算内容指纹，源稿件不会被修改。", "Copy and fingerprint the content without modifying the source manuscript.")}</p></div>
                      <button className="primary-button" type="button" onClick={createWorkspace} disabled={!selectionId || isCreatingWorkspace}>{isCreatingWorkspace ? text("正在创建…", "Creating…") : text("创建本地工作区", "Create local workspace")}<Icon name="arrow" /></button>
                    </div>
                  </div>
                )}
                {errorMessage ? <ErrorNotice message={localizeBackendText(locale, errorMessage)} onRetry={selectManuscript} /> : null}
              </section>

              <section className="track-grid" aria-label={text("本地工作原则", "Local workspace principles")}>
                <article className="track-card"><span>01</span><h2>{text("源稿不变", "Source stays unchanged")}</h2><p>{text("所有处理基于版本化工作副本，原稿始终保持只读。", "All processing uses versioned working copies; the source remains read-only.")}</p></article>
                <article className="track-card"><span>02</span><h2>{text("传输可见", "Transfers stay visible")}</h2><p>{text("你自主决定是否联网、使用模型和外部投送。", "You decide whether to go online, use models, or send work externally.")}</p></article>
              </section>
              {recentWorkspaces.length > 0 || archivedWorkspaces.length > 0 || catalogWarnings.length > 0 || workspaceManagementNotice || workspaceManagementError ? <RecentWorkspaces workspaces={recentWorkspaces} archivedWorkspaces={archivedWorkspaces} warnings={catalogWarnings.map((warning) => localizeBackendText(locale, warning))} busyId={workspaceManagementBusyId} notice={workspaceManagementNotice} error={workspaceManagementError} onOpen={openRecentWorkspace} onManage={manageWorkspace} /> : null}
            </div>
          </main>

          <SubmissionGuide />
        </div>
        <LiveStatus selecting={isSelecting} analyzing={isAnalyzing} evaluating={isEvaluating} />
      </div>
    );
  }

  const currentStage = WORKSPACE_STAGES.find((stage) => stage.id === activeStage) ?? WORKSPACE_STAGES[0];
  const currentStageLabel = localize(locale, currentStage.zh, currentStage.en);
  return (
    <div className="app-shell workspace-shell">
      <a className="skip-link" href="#main-content">{text("跳到主要内容", "Skip to main content")}</a>
      <ProductBar manuscriptName={activeWorkspace.manuscript.name} onNewManuscript={selectManuscript} isSelecting={isSelecting} />
      <div className="workbench">
        <nav className="task-rail" aria-label={text("论文工作阶段", "Manuscript workflow stages")}>
          <button className="rail-button rail-workspace-button" type="button" aria-label={text("我的工作台", "My Workspace")} title={text("我的工作台", "My Workspace")} onClick={openWorkspaceHome}><Icon name="workspace" /></button>
          <div className="rail-divider" role="separator" aria-orientation="horizontal" />
          {WORKSPACE_STAGES.map((stage) => {
            const isCurrent = stage.id === activeStage;
            const isComplete = stage.id === "source"
              || (stage.id === "check" && readinessReport !== null)
              || (stage.id === "revision" && activeWorkspace.snapshotVersion > 1)
              || stage.id === "versions"
              || (stage.id === "attestation" && attestation !== null)
              || (stage.id === "submission" && submission !== null)
              || (stage.id === "knowledge" && knowledgeBodyRecord !== null);
            const stageLabel = localize(locale, stage.zh, stage.en);
            return <button key={stage.id} className="rail-button" type="button" aria-current={isCurrent ? "step" : undefined} aria-label={stageLabel} title={stageLabel} data-complete={isComplete} onClick={() => openStage(stage.id)}><Icon name={getStageIcon(stage.id)} /><span>{localize(locale, stage.shortZh, stage.shortEn)}</span></button>;
          })}
        </nav>

        <main id="main-content" className="workspace-main">
          <header className="workflow-header">
            <div><p className="breadcrumb">{text("工作台", "Workspace")} <span>/</span> {currentStageLabel}</p><h1>{currentStageLabel}</h1><p>{getStageDescription(activeStage, locale)}</p></div>
            <StageStatus stage={activeStage} structureReport={structureReport} readinessReport={readinessReport} />
          </header>
          <div className="pane-switcher" role="tablist" aria-label={text("工作区视图", "Workspace views")}>
            <button type="button" role="tab" aria-controls="operation-pane" aria-selected={mobilePane === "operation"} onClick={() => setMobilePane("operation")}>{text("操作", "Actions")}</button>
            <button type="button" role="tab" aria-controls="evidence-pane" aria-selected={mobilePane === "evidence"} onClick={() => setMobilePane("evidence")}>{text("证据", "Evidence")}</button>
          </div>
          <div className="workspace-panes" data-mobile-pane={mobilePane}>
            <section id="operation-pane" className="operation-pane" role="tabpanel" aria-label={`${currentStageLabel} ${text("操作", "Actions")}`}>
              <OperationPane stage={activeStage} workspace={activeWorkspace} structureReport={structureReport} readinessReport={readinessReport} knowledgeBodySnapshot={knowledgeBodySnapshot} knowledgeBodyRecord={knowledgeBodyRecord} disciplineCatalog={disciplineCatalog} selectedDisciplineCode={selectedDisciplineCode} attestation={attestation} submission={submission} submissionExport={submissionExport} ruleCatalog={ruleCatalog} selectedRulePackIds={selectedRulePackIds} submissionElementCatalog={submissionElementCatalog} revisionDraft={revisionDraft} revisionValues={revisionValues} revisionResult={revisionResult} versionHistory={versionHistory} selectedVersion={selectedVersion} versionCandidate={versionCandidate} versionNote={versionNote} versionNotice={versionNotice} attestationConfirmed={attestationConfirmed} submissionConfirmed={submissionConfirmed} submissionTarget={submissionTarget} submissionReceipt={submissionReceipt} isLoadingRuleCatalog={isLoadingRuleCatalog} isLoadingSubmissionElements={isLoadingSubmissionElements} isLoadingLifecycle={isLoadingLifecycle} isLoadingKnowledgeBody={isLoadingKnowledgeBody} isLoadingDisciplineCatalog={isLoadingDisciplineCatalog} isApplyingRevision={isApplyingRevision} isAnalyzing={isAnalyzing} isEvaluating={isEvaluating} isSelectingVersion={isSelectingVersion} isSavingVersion={isSavingVersion} isRestoringVersion={isRestoringVersion} isAttesting={isAttesting} isExportingSubmission={isExportingSubmission} isRecordingSubmission={isRecordingSubmission} isFinalizingKnowledge={isFinalizingKnowledge} onAnalyze={analyzeWorkspace} onEvaluate={evaluateReadiness} onToggleRulePack={toggleRulePack} onOpenStage={openStage} onRevisionValueChange={(field, value) => setRevisionValues((current) => ({ ...current, [field]: value }))} onApplyRevision={applyRevision} onSelectVersionCandidate={selectVersionCandidate} onVersionNoteChange={setVersionNote} onSaveVersion={saveVersion} onSelectVersion={(version) => compareVersions(activeWorkspace, version, activeWorkspace.snapshotVersion)} onRestoreVersion={restoreVersion} onAttestationConfirmed={setAttestationConfirmed} onCreateAttestation={createAttestation} onExportSubmission={exportSubmission} onSubmissionConfirmed={setSubmissionConfirmed} onSubmissionTargetChange={setSubmissionTarget} onSubmissionReceiptChange={setSubmissionReceipt} onRecordSubmission={recordSubmission} onDisciplineChange={setSelectedDisciplineCode} onFinalizeKnowledge={finalizeKnowledgeBody} />
            </section>
            <aside id="evidence-pane" className="evidence-pane" role="tabpanel" aria-label={`${currentStageLabel} ${text("证据", "Evidence")}`}><EvidencePane stage={activeStage} workspace={activeWorkspace} structureReport={structureReport} readinessReport={readinessReport} knowledgeBodySnapshot={knowledgeBodySnapshot} knowledgeBodyRecord={knowledgeBodyRecord} attestation={attestation} submission={submission} submissionExport={submissionExport} ruleCatalog={ruleCatalog} selectedRulePackIds={selectedRulePackIds} submissionElementCatalog={submissionElementCatalog} revisionDraft={revisionDraft} revisionValues={revisionValues} revisionResult={revisionResult} versionHistory={versionHistory} selectedVersion={selectedVersion} versionComparison={versionComparison} isComparingVersions={isComparingVersions} /></aside>
          </div>
          {errorMessage ? <ErrorNotice message={localizeBackendText(locale, errorMessage)} onRetry={activeStage === "check" ? (structureReport ? evaluateReadiness : analyzeWorkspace) : activeStage === "versions" ? () => loadVersionHistory(activeWorkspace) : activeStage === "knowledge" ? finalizeKnowledgeBody : () => openStage(activeStage)} /> : null}
        </main>
      </div>
      <LiveStatus selecting={isSelecting} analyzing={isAnalyzing} evaluating={isEvaluating} />
    </div>
  );
}

function ProductBar({ manuscriptName, onNewManuscript, isSelecting = false }: { manuscriptName?: string; onNewManuscript?: () => void; isSelecting?: boolean }) {
  const { locale, setLocale, text } = useI18n();
  return <header className="product-bar"><div className="brand" aria-label="投稿舱 ManuscriptDock"><span className="brand-mark" aria-hidden="true"><img src={manuscriptDockLogo} alt="" width="32" height="32" /></span><span className="brand-copy"><span className="brand-cn" lang="zh-CN">投稿舱</span><span className="brand-name" lang="en">ManuscriptDock</span></span></div>{manuscriptName ? <p className="current-manuscript" title={manuscriptName}>{manuscriptName}</p> : <span />}<div className="bar-actions"><div className="language-switch" role="group" aria-label={text("界面语言", "Interface language")}><button type="button" aria-pressed={locale === "zh-CN"} onClick={() => setLocale("zh-CN")}>中文</button><button type="button" aria-pressed={locale === "en"} onClick={() => setLocale("en")}>EN</button></div><span className="local-badge" title={text("稿件尚未离开你的设备", "The manuscript has not left your device")}><Icon name="lock" />{text("仅在本机", "Local only")}</span>{onNewManuscript ? <button className="bar-button" type="button" onClick={onNewManuscript} disabled={isSelecting}>{isSelecting ? text("正在打开…", "Opening…") : text("导入另一篇", "Import another")}</button> : null}</div></header>;
}

function SubmissionGuide() {
  const { text } = useI18n();
  const items = [
    ["1", text("导入与检查", "Import and check"), text("建立只读快照，提取结构并按目标规则生成逐条结论", "Create a read-only snapshot, extract structure, and run target-aware checks")],
    ["2", text("修订与版本", "Revise and version"), text("核对依据、保存新版本并自动复查当前稿件", "Review evidence, save a new version, and recheck the current manuscript")],
    ["3", text("本地存证", "Local attestation"), text("由作者确认稿件版本和检查报告，形成带指纹的证据记录", "Author-confirm the version and report to create a fingerprinted evidence record")],
    ["4", text("导出与投稿", "Export and submit"), text("导出交付包；在期刊网站提交后登记目标与回执", "Export a handoff package, then record the target and receipt after journal submission")],
    ["5", text("固化知识体", "Finalize knowledge body"), text("把本次稿件、证据和投稿链固化为不可变知识体快照", "Finalize the manuscript, evidence, and submission chain as an immutable knowledge-body snapshot")],
  ];
  return <aside className="submission-guide" aria-labelledby="guide-heading"><header><h2 id="guide-heading">{text("投稿指引", "Submission guide")}</h2></header><div className="guide-list">{items.map(([number, title, copy]) => <article className="guide-card" key={number}><span>{number}</span><div><h3>{title}</h3><p>{copy}</p></div></article>)}</div></aside>;
}

function ErrorNotice({ message, onRetry }: { message: string; onRetry: () => void }) {
  const { text } = useI18n();
  return <div className="error-notice" role="alert"><Icon name="warning" /><div><strong>{text("操作未完成", "Action not completed")}</strong><span>{message}</span></div><button type="button" onClick={onRetry}>{text("重试", "Retry")}</button></div>;
}

function LiveStatus({ selecting, analyzing, evaluating }: { selecting: boolean; analyzing: boolean; evaluating: boolean }) {
  const { text } = useI18n();
  return <div className="sr-only" role="status" aria-live="polite">{selecting ? text("正在打开系统文件选择器", "Opening the system file picker") : analyzing ? text("正在解析论文结构", "Analyzing manuscript structure") : evaluating ? text("正在检查投稿准备", "Checking submission readiness") : ""}</div>;
}

function RecentWorkspaces({ workspaces, archivedWorkspaces, warnings, busyId, notice, error, onOpen, onManage }: { workspaces: WorkspaceSummary[]; archivedWorkspaces: WorkspaceSummary[]; warnings: string[]; busyId: string | null; notice: string | null; error: string | null; onOpen: (workspace: WorkspaceSummary) => void; onManage: (action: "archive" | "restore" | "delete", workspace: WorkspaceSummary, archived: boolean) => Promise<boolean>; }) {
  const { locale, text } = useI18n();
  const [view, setView] = useState<"recent" | "archived">("recent");
  const [menuWorkspaceId, setMenuWorkspaceId] = useState<string | null>(null);
  const [deleteWorkspaceId, setDeleteWorkspaceId] = useState<string | null>(null);
  const archived = view === "archived";
  const visibleWorkspaces = archived ? archivedWorkspaces : workspaces;

  useEffect(() => {
    if (!menuWorkspaceId && !deleteWorkspaceId) return;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMenuWorkspaceId(null);
        setDeleteWorkspaceId(null);
      }
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [menuWorkspaceId, deleteWorkspaceId]);

  const perform = async (action: "archive" | "restore" | "delete", workspace: WorkspaceSummary) => {
    const completed = await onManage(action, workspace, archived);
    if (completed) {
      setMenuWorkspaceId(null);
      setDeleteWorkspaceId(null);
    }
  };

  return <section className="recent-section" aria-labelledby="recent-heading">
    <div className="section-heading"><div><p className="field-kicker">{text("设备上的记录", "Records on this device")}</p><h2 id="recent-heading">{text("最近工作区", "Recent workspaces")}</h2></div><span>{locale === "zh-CN" ? `${workspaces.length + archivedWorkspaces.length} 个` : workspaces.length + archivedWorkspaces.length}</span></div>
    <div className="workspace-catalog-tabs" role="tablist" aria-label={text("工作区状态", "Workspace status")}><button type="button" role="tab" aria-selected={view === "recent"} onClick={() => { setView("recent"); setMenuWorkspaceId(null); setDeleteWorkspaceId(null); }}>{text("最近工作区", "Recent workspaces")}<span>{workspaces.length}</span></button><button type="button" role="tab" aria-selected={view === "archived"} onClick={() => { setView("archived"); setMenuWorkspaceId(null); setDeleteWorkspaceId(null); }}>{text("已归档", "Archived")}<span>{archivedWorkspaces.length}</span></button></div>
    {notice ? <p className="workspace-management-message" role="status">{notice}</p> : null}
    {error ? <p className="workspace-management-message workspace-management-error" role="alert">{error}</p> : null}
    {visibleWorkspaces.length > 0 ? <ul className="recent-list">{visibleWorkspaces.slice(0, 20).map((workspace) => {
      const isBusy = busyId === workspace.id;
      const menuOpen = menuWorkspaceId === workspace.id;
      const confirmingDelete = deleteWorkspaceId === workspace.id;
      return <li key={workspace.id}>
        <div className="workspace-row">
          <button className="workspace-open-button" type="button" onClick={() => onOpen(workspace)} disabled={archived || isBusy} aria-label={archived ? text(`${workspace.manuscript.name} 已归档`, `${workspace.manuscript.name} is archived`) : text(`打开 ${workspace.manuscript.name}`, `Open ${workspace.manuscript.name}`)}><span className="recent-file-icon" aria-hidden="true"><Icon name="file" /></span><span className="recent-file-copy"><strong>{workspace.manuscript.name}</strong><span>{archived ? text("已归档 · 恢复后可继续工作", "Archived · Restore to continue") : text("源快照", "Source snapshot")} {archived ? "" : `v${workspace.snapshotVersion} · ${formatModifiedDate(workspace.importedUnixMs, locale)}`}</span></span><code>{workspace.contentHash.slice(0, 8)}</code>{!archived ? <Icon name="arrow" /> : null}</button>
          <button className="workspace-manage-button" type="button" aria-label={text(`管理 ${workspace.manuscript.name}`, `Manage ${workspace.manuscript.name}`)} aria-expanded={menuOpen} aria-controls={`workspace-menu-${workspace.id}`} disabled={isBusy} onClick={() => { setMenuWorkspaceId(menuOpen ? null : workspace.id); setDeleteWorkspaceId(null); }}><Icon name="more" /><span>{isBusy ? text("处理中", "Working") : text("管理", "Manage")}</span></button>
          {menuOpen ? <div className="workspace-management-menu" id={`workspace-menu-${workspace.id}`} role="menu" aria-label={text(`${workspace.manuscript.name} 管理操作`, `Management actions for ${workspace.manuscript.name}`)}><button type="button" role="menuitem" onClick={() => void perform(archived ? "restore" : "archive", workspace)}><Icon name={archived ? "restore" : "archive"} />{archived ? text("恢复到最近工作区", "Restore to recent") : text("归档工作区", "Archive workspace")}</button><button className="workspace-delete-action" type="button" role="menuitem" onClick={() => { setDeleteWorkspaceId(workspace.id); setMenuWorkspaceId(null); }}><Icon name="trash" />{text("永久删除…", "Delete permanently…")}</button></div> : null}
        </div>
        {confirmingDelete ? <div className="workspace-delete-confirm" role="group" aria-labelledby={`delete-title-${workspace.id}`}><div><strong id={`delete-title-${workspace.id}`}>{text("永久删除这个论文工作区？", "Permanently delete this manuscript workspace?")}</strong><p>{text("将删除全部论文版本、分析、检查、存证、投稿和知识体问答记录。此操作无法撤销。", "All manuscript versions, analyses, checks, attestations, submissions, and knowledge-body dialogue will be deleted. This cannot be undone.")}</p></div><div><button type="button" onClick={() => setDeleteWorkspaceId(null)} disabled={isBusy}>{text("取消", "Cancel")}</button><button className="confirm-delete-button" type="button" onClick={() => void perform("delete", workspace)} disabled={isBusy}><Icon name="trash" />{isBusy ? text("正在删除…", "Deleting…") : text("确认永久删除", "Delete permanently")}</button></div></div> : null}
      </li>;
    })}</ul> : <div className="workspace-catalog-empty"><Icon name={archived ? "archive" : "file"} /><p>{archived ? text("还没有归档的论文工作区。", "No archived manuscript workspaces yet.") : text("最近工作区为空。", "No recent workspaces.")}</p></div>}
    {warnings.map((warning) => <p className="catalog-warning" key={warning}>{warning}</p>)}
  </section>;
}

function getStageDescription(stage: WorkspaceStage, locale: Locale) {
  const descriptions: Record<WorkspaceStage, string> = {
    source: localize(locale, "确认本地只读源快照，然后进入检查。", "Confirm the local read-only source snapshot, then begin checks."),
    check: localize(locale, "在一个阶段完成结构提取、规则选择和逐条投稿检查。", "Extract structure, choose rules, and run itemized checks in one stage."),
    revision: localize(locale, "依据检查结果修订安全字段，并保存为新的不可变版本。", "Revise safe fields from check evidence and save an immutable new version."),
    versions: localize(locale, "核验、比较或恢复论文版本；版本变化会使下游记录待更新。", "Verify, compare, or restore versions; a new version makes downstream records stale."),
    journals: localize(locale, "基于当前版本和公开目录快照，在本机计算国内、国际投稿目标。", "Compute domestic and international submission targets locally from the current version and a public directory snapshot."),
    attestation: localize(locale, "作者确认当前版本与检查报告，建立本地加密完整性记录。", "Author-confirm the current version and report as a local integrity record."),
    submission: localize(locale, "导出投稿交付包，并在外部提交后登记目标和回执。", "Export the submission handoff, then record the target and receipt after external submission."),
    knowledge: localize(locale, "固化并查看由稿件、证据与投稿记录组成的知识体快照。", "Finalize and view the knowledge-body snapshot formed by the manuscript, evidence, and submission record."),
  };
  return descriptions[stage];
}

function StageStatus({ stage, structureReport, readinessReport }: { stage: WorkspaceStage; structureReport: StructureReport | null; readinessReport: ReadinessReport | null }) {
  const { text } = useI18n();
  let label = text("当前设备", "This device");
  let tone = "local";
  if (stage === "check") label = readinessReport ? text("当前版本已检查", "Current version checked") : structureReport ? text("等待运行规则", "Ready for rules") : text("等待提取", "Awaiting extraction");
  if (stage === "revision") label = readinessReport ? text("依据当前检查", "Based on current checks") : text("需要检查", "Checks required");
  if (stage === "versions") label = text("本地版本库", "Local version library");
  if (stage === "journals") label = text("本地计算 · 非录用概率", "Local calculation · Not acceptance odds");
  if (stage === "attestation") { label = readinessReport ? text("可创建存证", "Ready to attest") : text("需要当前检查", "Current check required"); tone = readinessReport ? "local" : "warning"; }
  if (stage === "submission") { label = text("作者控制外发", "Author-controlled handoff"); tone = "info"; }
  if (stage === "knowledge") { label = text("不可变快照", "Immutable snapshot"); tone = "info"; }
  return <span className="stage-status" data-tone={tone}><Icon name={tone === "warning" ? "warning" : "check"} />{label}</span>;
}

interface PaneProps { stage: WorkspaceStage; workspace: WorkspaceSummary; structureReport: StructureReport | null; readinessReport: ReadinessReport | null; knowledgeBodySnapshot?: AcademicKnowledgeBodySnapshot | null; knowledgeBodyRecord?: KnowledgeBodyRecord | null; disciplineCatalog?: DisciplineCatalogItem[]; selectedDisciplineCode?: string; attestation?: LocalAttestation | null; submission?: SubmissionRecord | null; submissionExport?: SubmissionExport | null; ruleCatalog?: RulePackCatalogItem[]; selectedRulePackIds?: string[]; submissionElementCatalog?: SubmissionElementCatalog | null; revisionDraft?: RevisionDraft | null; revisionValues?: Record<string, string>; revisionResult?: RevisionSet | null; versionHistory?: VersionHistory | null; selectedVersion?: number | null; versionComparison?: VersionComparison | null; isComparingVersions?: boolean; }


type OperationPaneProps = PaneProps & {
  versionCandidate: ManuscriptSummary | null; versionNote: string; versionNotice: string | null;
  attestationConfirmed: boolean; submissionConfirmed: boolean; submissionTarget: string; submissionReceipt: string;
  isLoadingRuleCatalog: boolean; isLoadingSubmissionElements: boolean; isLoadingLifecycle: boolean; isLoadingKnowledgeBody: boolean; isLoadingDisciplineCatalog: boolean;
  isApplyingRevision: boolean; isAnalyzing: boolean; isEvaluating: boolean; isSelectingVersion: boolean; isSavingVersion: boolean; isRestoringVersion: boolean;
  isAttesting: boolean; isExportingSubmission: boolean; isRecordingSubmission: boolean; isFinalizingKnowledge: boolean;
  onAnalyze: () => void; onEvaluate: () => void; onToggleRulePack: (rulePackId: string) => void; onOpenStage: (stage: WorkspaceStage) => void;
  onRevisionValueChange: (field: string, value: string) => void; onApplyRevision: () => void;
  onSelectVersionCandidate: () => void; onVersionNoteChange: (note: string) => void; onSaveVersion: () => void; onSelectVersion: (version: number) => void; onRestoreVersion: (version: number) => void;
  onAttestationConfirmed: (confirmed: boolean) => void; onCreateAttestation: () => void; onExportSubmission: () => void;
  onSubmissionConfirmed: (confirmed: boolean) => void; onSubmissionTargetChange: (target: string) => void; onSubmissionReceiptChange: (receipt: string) => void; onRecordSubmission: () => void;
  onDisciplineChange: (code: string) => void;
  onFinalizeKnowledge: () => void;
};

function OperationPane(props: OperationPaneProps) {
  const { locale, text } = useI18n();
  const { stage, workspace, structureReport, readinessReport, knowledgeBodySnapshot = null, knowledgeBodyRecord = null, disciplineCatalog = [], selectedDisciplineCode = "", attestation = null, submission = null, submissionExport = null, ruleCatalog = [], selectedRulePackIds = [], submissionElementCatalog = null, revisionDraft = null, revisionValues = {}, revisionResult = null, versionHistory, selectedVersion, versionCandidate, versionNote, versionNotice, attestationConfirmed, submissionConfirmed, submissionTarget, submissionReceipt, isLoadingRuleCatalog, isLoadingSubmissionElements, isLoadingLifecycle, isLoadingKnowledgeBody, isLoadingDisciplineCatalog, isApplyingRevision, isAnalyzing, isEvaluating, isSelectingVersion, isSavingVersion, isRestoringVersion, isAttesting, isExportingSubmission, isRecordingSubmission, isFinalizingKnowledge, onAnalyze, onEvaluate, onToggleRulePack, onOpenStage, onRevisionValueChange, onApplyRevision, onSelectVersionCandidate, onVersionNoteChange, onSaveVersion, onSelectVersion, onRestoreVersion, onAttestationConfirmed, onCreateAttestation, onExportSubmission, onSubmissionConfirmed, onSubmissionTargetChange, onSubmissionReceiptChange, onRecordSubmission, onDisciplineChange, onFinalizeKnowledge } = props;

  if (isLoadingLifecycle) return <EmptyStage icon="package" kicker={text("恢复流程", "Restore workflow")} title={text("正在恢复当前版本的流程记录", "Restoring lifecycle records for the current version")} copy={text("只读取与当前内容指纹匹配的结构、检查、存证、投稿和知识体记录。", "Only structure, checks, attestation, submission, and knowledge records matching the current fingerprint are restored.")} />;

  if (stage === "source") return <><p className="workspace-created-status"><Icon name="check" />{text("本地工作区已创建", "Local workspace created")}</p><PanelHeading kicker={text("步骤 1 / 8 · 导入", "Step 1 / 8 · Import")} title={text("确认当前稿件", "Confirm current manuscript")} copy={text("已建立不可变副本；后续动作都绑定具体版本，不覆盖历史。", "An immutable copy now exists; every later action binds to an exact version without overwriting history.")} /><dl className="detail-list"><div><dt>{text("文件", "File")}</dt><dd>{workspace.manuscript.name}</dd></div><div><dt>{text("格式与大小", "Format and size")}</dt><dd>{workspace.manuscript.extension.toUpperCase()} · {formatBytes(workspace.manuscript.sizeBytes)}</dd></div><div><dt>{text("当前版本", "Current version")}</dt><dd>v{workspace.snapshotVersion}</dd></div><div><dt>{text("内容指纹", "Fingerprint")}</dt><dd><code>{workspace.contentHash.slice(0, 16)}</code></dd></div></dl><BoundaryNote title={text("当前边界", "Current boundary")} copy={text("未联网、未调用模型、未外发；页面不接收源文件路径。", "No network, model call, or transmission occurred; the page never receives the source path.")} /><PaneAction label={text("下一步", "Next")} title={text("检查当前版本", "Check the current version")} copy={text("先提取结构，再选择适用规则并生成逐条结论。", "Extract structure, choose applicable rules, and generate itemized findings.")} buttonLabel={text("开始检查", "Start checks")} onClick={() => onOpenStage("check")} /></>;

  if (stage === "check") {
    if (!structureReport) return <EmptyStage icon="structure" kicker={text("步骤 2 / 8 · 检查", "Step 2 / 8 · Check")} title={text("先建立论文结构", "First establish manuscript structure")} copy={text("标题、作者、摘要、章节和声明只在本机确定性提取。", "Title, authors, abstract, sections, and declarations are extracted deterministically on this device.")} actionLabel={isAnalyzing ? text("正在提取…", "Extracting…") : text("提取论文结构", "Extract structure")} disabled={isAnalyzing} onAction={onAnalyze} />;
    if (!readinessReport) return <><StructureCheckSummary report={structureReport} /><TargetRuleSelector ruleCatalog={ruleCatalog} selectedRulePackIds={selectedRulePackIds} loading={isLoadingRuleCatalog} structureReady onToggle={onToggleRulePack} onContinue={onEvaluate} actionLabel={isEvaluating ? text("正在检查…", "Checking…") : text("运行投稿检查", "Run submission checks")} disabled={isEvaluating} /></>;
    return <><PanelHeading kicker={`${text("步骤 2 / 8 · 投稿检查", "Step 2 / 8 · Submission check")} · v${readinessReport.reportVersion}`} title={outcomeLabel(readinessReport.outcome, locale)} copy={text("报告与当前稿件版本、内容指纹和规则来源绑定。", "The report is bound to the current manuscript version, fingerprint, and rule sources.")} /><div className="metric-row" aria-label={text("投稿检查统计", "Submission-check metrics")}><Metric label={text("通过", "Passed")} value={readinessReport.passedCount} /><Metric label={text("建议", "Suggestions")} value={readinessReport.warningCount} /><Metric label={text("阻断", "Blocked")} value={readinessReport.blockedCount} /><Metric label={text("待确认", "Confirmations")} value={readinessReport.confirmationCount} /></div><ol className="finding-list" aria-label={text("投稿检查明细", "Submission-check details")}>{readinessReport.findings.map((finding) => <li key={finding.ruleId} data-status={finding.status}><span className="finding-status">{findingLabel(finding.status, locale)}</span><div><strong>{locale === "en" && finding.messageEn ? finding.messageEn : localizeBackendText(locale, finding.message)}</strong><code>{finding.sourceLocation}</code></div></li>)}</ol><div className="secondary-action-row"><button className="text-button" type="button" onClick={onEvaluate} disabled={isEvaluating}>{text("重新检查", "Run again")}</button></div><PaneAction label={text("下一步", "Next")} title={text("根据结论修订", "Revise from findings")} copy={text("进入安全字段修订台；每次保存都会生成新版本并自动复查。", "Open safe-field revision; every save creates a new version and automatically rechecks it.")} buttonLabel={text("进入修订", "Continue to revision")} onClick={() => onOpenStage("revision")} /></>;
  }

  if (stage === "revision") {
    if (!readinessReport) return <EmptyStage icon="format" kicker={text("步骤 3 / 8 · 修订", "Step 3 / 8 · Revise")} title={text("需要当前版本的检查报告", "A current check report is required")} copy={text("修订必须从可追溯的检查依据开始。", "Revision must start from traceable check evidence.")} actionLabel={text("返回检查", "Return to checks")} onAction={() => onOpenStage("check")} />;
    return <SubmissionElementsDesk catalog={submissionElementCatalog} draft={revisionDraft} values={revisionValues} result={revisionResult} loading={isLoadingSubmissionElements} saving={isApplyingRevision} selectedPublisherCount={ruleCatalog.filter((item) => item.category === "publisher" && selectedRulePackIds.includes(item.id)).length} onValueChange={onRevisionValueChange} onSave={onApplyRevision} onContinue={() => onOpenStage("versions")} />;
  }

  if (stage === "versions") return <VersionManager workspace={workspace} history={versionHistory ?? null} selectedVersion={selectedVersion ?? null} candidate={versionCandidate} note={versionNote} notice={versionNotice} selecting={isSelectingVersion} saving={isSavingVersion} restoring={isRestoringVersion} onSelectCandidate={onSelectVersionCandidate} onNoteChange={onVersionNoteChange} onSave={onSaveVersion} onSelectVersion={onSelectVersion} onRestore={onRestoreVersion} onContinue={() => onOpenStage(readinessReport ? "journals" : "check")} continueReady={readinessReport !== null} />;

  if (stage === "journals") return <JournalMatchStage workspace={workspace} onContinue={() => onOpenStage("attestation")} />;

  if (stage === "attestation") {
    if (!readinessReport) return <EmptyStage icon="package" kicker={text("步骤 6 / 8 · 存证", "Step 6 / 8 · Attest")} title={text("当前版本尚未检查", "The current version has not been checked")} copy={text("新版本不会继承旧版本的检查与存证。", "A new version never inherits checks or attestation from an older version.")} actionLabel={text("检查当前版本", "Check current version")} onAction={() => onOpenStage("check")} />;
    if (attestation) return <><PanelHeading kicker={text("步骤 6 / 8 · 本地存证", "Step 6 / 8 · Local attestation")} title={text(`v${attestation.manuscriptVersion} 已完成存证`, `v${attestation.manuscriptVersion} attested`)} copy={text("记录绑定稿件指纹、检查报告、作者确认和时间；不宣称上链或证明科学结论。", "The record binds the manuscript fingerprint, check report, author confirmation, and time; it does not claim blockchain notarization or scientific truth.")} /><LifecycleRecord label="Attestation" id={attestation.attestationId} hash={attestation.recordHash} timestamp={attestation.attestedUnixMs} /><PaneAction label={text("下一步", "Next")} title={text("准备投稿交付", "Prepare submission handoff")} copy={text("导出可交付文件，并在外部期刊系统提交后登记回执。", "Export the handoff files, then record the receipt after submitting in the journal system.")} buttonLabel={text("进入投稿", "Continue to submission")} onClick={() => onOpenStage("submission")} /></>;
    return <><PanelHeading kicker={text("步骤 6 / 8 · 本地存证", "Step 6 / 8 · Local attestation")} title={text("确认当前证据边界", "Confirm the current evidence boundary")} copy={text(`将绑定稿件 v${workspace.snapshotVersion}、检查报告 ${readinessReport.reportId.slice(0, 8)} 和输出快照 v${readinessReport.outputSnapshotVersion}。`, `This binds manuscript v${workspace.snapshotVersion}, check report ${readinessReport.reportId.slice(0, 8)}, and output snapshot v${readinessReport.outputSnapshotVersion}.`)} /><label className="confirmation-control"><input type="checkbox" checked={attestationConfirmed} onChange={(event) => onAttestationConfirmed(event.target.checked)} /><span>{text("我已核对当前稿件、检查结论和待作者确认事项；我理解该记录不证明研究结论为真。", "I reviewed the current manuscript, findings, and author confirmations; I understand this record does not prove scientific truth.")}</span></label><BoundaryNote title={text("存证含义", "Meaning of attestation")} copy={text("这是带 SHA-256 的本地作者确认记录，不是区块链确权、公证或第三方时间戳。", "This is a SHA-256 local author-confirmation record, not blockchain ownership, notarization, or a third-party timestamp.")} /><button className="primary-button" type="button" disabled={!attestationConfirmed || isAttesting} onClick={onCreateAttestation}>{isAttesting ? text("正在创建…", "Creating…") : text("创建本地存证", "Create local attestation")}<Icon name="arrow" /></button></>;
  }

  if (stage === "submission") {
    if (!attestation) return <EmptyStage icon="package" kicker={text("步骤 7 / 8 · 投稿", "Step 7 / 8 · Submit")} title={text("需要先完成本地存证", "Local attestation is required first")} copy={text("投稿交付包必须绑定明确的作者确认记录。", "The submission handoff must bind to an explicit author-confirmation record.")} actionLabel={text("进入存证", "Go to attestation")} onAction={() => onOpenStage("attestation")} />;
    if (submission) return <><PanelHeading kicker={text("步骤 7 / 8 · 投稿记录", "Step 7 / 8 · Submission record")} title={submission.target} copy={text("作者已确认在外部投稿系统完成提交；ManuscriptDock 只保存本地登记。", "The author confirmed submission in an external system; ManuscriptDock stores only the local record.")} /><LifecycleRecord label="Submission" id={submission.submissionId} hash={submission.recordHash} timestamp={submission.submittedUnixMs} /><dl className="detail-list"><div><dt>{text("投稿目标", "Target")}</dt><dd>{submission.target}</dd></div><div><dt>{text("回执", "Receipt")}</dt><dd>{submission.receipt ?? text("未填写", "Not provided")}</dd></div></dl><PaneAction label={text("下一步", "Next")} title={text("固化知识体快照", "Finalize the knowledge-body snapshot")} copy={text("把稿件、检查、存证和投稿记录固定在同一不可变研究记忆中。", "Pin the manuscript, checks, attestation, and submission record in one immutable research memory.")} buttonLabel={text("进入知识体", "Continue to knowledge body")} onClick={() => onOpenStage("knowledge")} /></>;
    return <><PanelHeading kicker={text("步骤 7 / 8 · 作者控制投稿", "Step 7 / 8 · Author-controlled submission")} title={text("先导出，再登记", "Export, then record")} copy={text("ManuscriptDock 不伪装成期刊网站：先生成本地交付包，由你提交后再登记结果。", "ManuscriptDock does not impersonate a journal site: export the local handoff, submit it yourself, then record the result.")} /><section className="submission-action-card"><div><span>01</span><h3>{text("导出投稿交付包", "Export submission handoff")}</h3><p>{text("包含当前稿件、JSON 检查报告、HTML 预览、存证记录和清单。", "Includes the current manuscript, JSON report, HTML preview, attestation, and manifest.")}</p></div><button className="secondary-button" type="button" onClick={onExportSubmission} disabled={isExportingSubmission}>{isExportingSubmission ? text("正在导出…", "Exporting…") : text("选择导出文件夹", "Choose export folder")}</button></section>{submissionExport ? <p className="revision-saved" role="status"><Icon name="check" />{text(`已导出 ${submissionExport.packageName}（${submissionExport.files.length} 个文件）`, `Exported ${submissionExport.packageName} (${submissionExport.files.length} files)`)}</p> : null}<section className="submission-record-form" aria-labelledby="submission-record-heading"><header><span>02</span><h3 id="submission-record-heading">{text("登记已完成的外部投稿", "Record a completed external submission")}</h3></header><label htmlFor="submission-target">{text("期刊、会议或预印本平台", "Journal, conference, or preprint platform")}</label><input id="submission-target" value={submissionTarget} maxLength={200} onChange={(event) => onSubmissionTargetChange(event.target.value)} placeholder={text("例如：Journal of …", "For example: Journal of …")} /><label htmlFor="submission-receipt">{text("稿件号或回执（可选）", "Manuscript ID or receipt (optional)")}</label><input id="submission-receipt" value={submissionReceipt} maxLength={200} onChange={(event) => onSubmissionReceiptChange(event.target.value)} /><label className="confirmation-control"><input type="checkbox" checked={submissionConfirmed} onChange={(event) => onSubmissionConfirmed(event.target.checked)} /><span>{text("我确认已经在上述外部系统完成投稿；此操作只在本机登记，不会发送文件。", "I confirm I completed submission in the external system; this action only records it locally and sends no files.")}</span></label><button className="primary-button" type="button" disabled={!submissionTarget.trim() || !submissionConfirmed || isRecordingSubmission} onClick={onRecordSubmission}>{isRecordingSubmission ? text("正在登记…", "Recording…") : text("登记投稿记录", "Record submission")}<Icon name="arrow" /></button></section></>;
  }

  if (isLoadingKnowledgeBody && !knowledgeBodySnapshot) return <EmptyStage icon="package" kicker={text("步骤 8 / 8 · 知识体", "Step 8 / 8 · Knowledge body")} title={text("正在读取知识体快照", "Loading the knowledge-body snapshot")} copy={text("正在校验对象版本和生命周期引用。", "Verifying object versions and lifecycle references.")} />;
  if (!structureReport && knowledgeBodyRecord?.disciplineClassification) return <KnowledgeBodyOperation workspace={workspace} snapshot={knowledgeBodyRecord.snapshot} record={knowledgeBodyRecord} />;
  if (!structureReport) return <EmptyStage icon="package" kicker={text("步骤 8 / 8 · 知识体", "Step 8 / 8 · Knowledge body")} title={text("需要先完成结构提取", "Structure extraction is required first")} copy={text("提取论文文本、表格与图片线索后，系统会立即建立可追溯的候选知识体。", "After extracting text, table, and figure signals, the app immediately creates a traceable candidate knowledge body.")} actionLabel={text("返回检查", "Return to check")} onAction={() => onOpenStage("check")} />;
  if (!knowledgeBodySnapshot) return <EmptyStage icon="package" kicker={text("步骤 8 / 8 · 知识体", "Step 8 / 8 · Knowledge body")} title={text("尚未生成候选知识体", "Candidate knowledge body is not available")} copy={text("请重新运行当前版本的结构提取。", "Run structure extraction again for the current version.")} actionLabel={text("重新提取", "Extract again")} onAction={() => onOpenStage("check")} />;
  if (!submission) return <><PanelHeading kicker={text("结构提取后立即生成", "Generated after structure extraction")} title={text("候选知识体已经建立", "Candidate knowledge body created")} copy={text("系统已将可识别的 Claim、Scope、Method、Result 与 Evidence 保存为带来源的候选对象；作者确认前不会冒充正式结论。", "Recognizable Claim, Scope, Method, Result, and Evidence content is stored as source-backed candidates and never presented as a confirmed conclusion before author review.")} /><KnowledgeCandidatePreview snapshot={knowledgeBodySnapshot} structureReport={structureReport} /><BoundaryNote title={text("候选与固化是两件事", "Extraction and finalization are separate")} copy={text("候选知识体现在即可查看；完成存证与投稿登记后，作者选择学科分类并固化不可变快照。", "The candidate knowledge body is available now. After attestation and submission registration, the author selects a discipline and finalizes an immutable snapshot.")} /><button className="secondary-action" type="button" onClick={() => onOpenStage("attestation")}>{text("继续完成存证与投稿", "Continue to attestation and submission")}<Icon name="arrow" /></button></>;
  const finalizedDecompositionHash = knowledgeBodyRecord?.snapshot.extraction?.decompositionHash ?? null;
  const currentDecompositionHash = knowledgeBodySnapshot.extraction?.decompositionHash ?? null;
  const requiresUpdatedSnapshot = knowledgeBodyRecord !== null && (knowledgeBodyRecord.snapshot.schemaVersion !== knowledgeBodySnapshot.schemaVersion || finalizedDecompositionHash !== currentDecompositionHash);
  if (!knowledgeBodyRecord?.disciplineClassification || requiresUpdatedSnapshot) return <><PanelHeading kicker={text("步骤 8 / 8 · 知识体", "Step 8 / 8 · Knowledge body")} title={requiresUpdatedSnapshot ? text("用统一分解资产更新知识体", "Update the knowledge body from the unified decomposition") : knowledgeBodyRecord ? text("补充学科索引分类", "Add discipline classification") : text("固化本次研究记忆", "Finalize this research memory")} copy={text("候选知识体已经从当前论文中提取；由作者确认学科分类后，再与存证和投稿记录一同固化。", "The candidate knowledge body has been extracted from the current manuscript. The author confirms a discipline before it is finalized with the attestation and submission record.")} /><KnowledgeCandidatePreview snapshot={knowledgeBodySnapshot} structureReport={structureReport} compact /><DisciplineSelector catalog={disciplineCatalog} selectedCode={selectedDisciplineCode} loading={isLoadingDisciplineCatalog} onChange={onDisciplineChange} /><BoundaryNote title={text("分类与固化边界", "Classification and finalization boundary")} copy={text("语义内容目前是本地提取候选，尚未得到作者逐条确认；学科分类完全由作者选择，不会调用大模型或发布到网络。", "Semantic content remains locally extracted and awaits item-level author confirmation. Classification is author-selected without model calls or network publishing.")} /><button className="primary-button" type="button" disabled={!selectedDisciplineCode || isLoadingDisciplineCatalog || isFinalizingKnowledge} onClick={onFinalizeKnowledge}>{isFinalizingKnowledge ? text("正在固化…", "Finalizing…") : requiresUpdatedSnapshot ? text("更新知识体快照", "Update knowledge-body snapshot") : knowledgeBodyRecord ? text("保存分类并生成新记录", "Save classification as a new record") : text("确认分类并固化知识体", "Confirm classification and finalize")}<Icon name="arrow" /></button></>;
  return <KnowledgeBodyOperation workspace={workspace} snapshot={knowledgeBodySnapshot ?? knowledgeBodyRecord.snapshot} record={knowledgeBodyRecord} structureReport={structureReport} />;
}

function JournalMatchStage({ workspace, onContinue }: { workspace: WorkspaceSummary; onContinue: () => void }) {
  const { locale, text } = useI18n();
  const [profile, setProfile] = useState<JournalRecommendationProfileInput>({ authorName: "", institution: "", specialty: "", manuscriptPurpose: "academic_communication", submissionDeadline: "" });
  const [institutionRequirementText, setInstitutionRequirementText] = useState("");
  const [institutionSourceUrl, setInstitutionSourceUrl] = useState("");
  const [officialSourceConfirmed, setOfficialSourceConfirmed] = useState(false);
  const [ruleTransmissionConfirmed, setRuleTransmissionConfirmed] = useState(false);
  const [preferences, setPreferences] = useState<JournalMatchPreferences>({ topic: "auto", articleType: "auto", language: "auto", targetStrategy: "balanced", openAccess: "no_preference" });
  const [run, setRun] = useState<JournalRecommendationRun | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const minimumDeadline = new Date(Date.now() + 86_400_000).toISOString().slice(0, 10);
  const profileComplete = profile.authorName.trim().length > 0 && profile.institution.trim().length > 0 && profile.specialty.trim().length > 0 && profile.submissionDeadline >= minimumDeadline;
  const hasInstitutionText = institutionRequirementText.trim().length > 0;
  const institutionTextReady = !hasInstitutionText || (institutionRequirementText.trim().length >= 40 && ruleTransmissionConfirmed);
  const update = <K extends keyof JournalMatchPreferences>(key: K, value: JournalMatchPreferences[K]) => { setPreferences((current) => ({ ...current, [key]: value })); setRun(null); };
  const updateProfile = <K extends keyof JournalRecommendationProfileInput>(key: K, value: JournalRecommendationProfileInput[K]) => { setProfile((current) => ({ ...current, [key]: value })); setRun(null); };
  async function calculate() {
    setBusy(true); setError(null);
    try {
      const savedProfile = await invoke<JournalRecommendationProfile>("save_journal_recommendation_profile", { workspaceId: workspace.id, profile });
      const effectiveProfileId = hasInstitutionText ? (await invoke<InstitutionRuleExtractionSummary>("extract_institution_requirements", { workspaceId: workspace.id, profileId: savedProfile.profileId, requirementText: institutionRequirementText, sourceUrl: institutionSourceUrl.trim() || null, authorAttestedOfficial: officialSourceConfirmed, authorConfirmedExternalTransmission: ruleTransmissionConfirmed })).profileId : savedProfile.profileId;
      setRun(await invoke<JournalRecommendationRun>("recommend_journals", { workspaceId: workspace.id, profileId: effectiveProfileId, preferences }));
    }
    catch (reason) { setError(localizeBackendText(locale, normalizeError(reason))); }
    finally { setBusy(false); }
  }
  const topicOptions: Array<[ResearchTopic, string, string]> = [["auto","自动识别","Auto detect"],["general_ai","通用人工智能","General AI"],["machine_learning","机器学习","Machine learning"],["computer_vision","计算机视觉","Computer vision"],["natural_language_processing","自然语言处理","Natural language processing"],["data_mining","数据挖掘","Data mining"],["software_systems","软件与系统","Software & systems"],["robotics_control","机器人与控制","Robotics & control"]];
  return <>
    <PanelHeading kicker={text("步骤 5 / 8 · 期刊匹配", "Step 5 / 8 · Journal match")} title={text("计算当前最适合的投稿组合", "Calculate the best current-fit submission set")} copy={text("每个论文新版本都会按同一维度重新计算；版本号本身不加分，只有可追溯的品质改善才会提升分数并解锁更好的匹配与冲刺期刊。", "Every manuscript revision is recalculated on the same dimensions. A higher version number earns no points by itself; only traceable quality improvements can raise scores and unlock stronger match and reach journals.")} />
    <BoundaryNote title={text("学校标准是资格依据，不是声誉分", "Institution rules are eligibility evidence, not prestige scores")} copy={text("正式规则只限定认可范围、用途和时间；学校排名、导师名气不会直接换算为期刊档次。系统结合论文适配度与当前条件给出冲刺、匹配和务实候选，保留更高目标的投稿机会。联网检索必须另行获得你的授权。", "Formal rules constrain recognition, purpose, and timing only. Institution rank and adviser fame are never converted directly into journal tiers. The system combines manuscript fit with current conditions to preserve reach, match, and pragmatic options. Online discovery requires separate authorization.")} />
    <section className="journal-profile" aria-labelledby="journal-profile-heading"><header><div><span>{text("仅保存在当前论文工作区", "Stored only in this manuscript workspace")}</span><h3 id="journal-profile-heading">{text("投稿背景档案", "Submission context profile")}</h3></div><strong>{text("必填", "Required")}</strong></header><div>
      <label htmlFor="journal-author-name">{text("投稿人姓名", "Author name")}<input id="journal-author-name" autoComplete="name" maxLength={120} value={profile.authorName} onChange={(event) => updateProfile("authorName", event.target.value)} /></label>
      <label htmlFor="journal-institution">{text("学校 / 机构", "Institution")}<input id="journal-institution" autoComplete="organization" maxLength={200} value={profile.institution} onChange={(event) => updateProfile("institution", event.target.value)} /></label>
      <label htmlFor="journal-specialty">{text("学院或专业", "Faculty or specialty")}<input id="journal-specialty" maxLength={160} value={profile.specialty} onChange={(event) => updateProfile("specialty", event.target.value)} placeholder={text("例如：计算机科学与技术", "For example: Computer Science")} /></label>
      <label htmlFor="journal-purpose">{text("论文用途", "Manuscript purpose")}<select id="journal-purpose" value={profile.manuscriptPurpose} onChange={(event) => updateProfile("manuscriptPurpose", event.target.value as ManuscriptPurpose)}><option value="academic_communication">{text("一般学术发表", "Academic communication")}</option><option value="degree_requirement">{text("学位成果要求", "Degree requirement")}</option><option value="graduation">{text("毕业要求", "Graduation")}</option><option value="professional_title">{text("职称或考核", "Professional evaluation")}</option><option value="project_completion">{text("项目结题", "Project completion")}</option></select></label>
      <label htmlFor="journal-deadline">{text("完成投稿截止日期", "Submission completion deadline")}<input id="journal-deadline" type="date" min={minimumDeadline} value={profile.submissionDeadline} onChange={(event) => updateProfile("submissionDeadline", event.target.value)} /><small>{text("用于评估投稿准备余量，不预测录用、见刊或数据库收录日期。", "Used for submission-preparation planning, not acceptance, publication, or indexing forecasts.")}</small></label>
    </div></section>
    <section className="institution-requirement" aria-labelledby="institution-requirement-heading"><header><div><span>{text("可选 · 学校要求优先", "Optional · institution requirements take priority")}</span><h3 id="institution-requirement-heading">{text("提供学校正式要求", "Provide official institution requirements")}</h3></div><strong>{text("模型结构化", "Model extraction")}</strong></header><p>{text("粘贴研究生院、科研处或学院发布的投稿与成果认定原文。学校名称和专业用于限定适用范围；模型不得凭常识补写规则。", "Paste the original submission or research-output policy issued by the graduate school, research office, or faculty. Institution name and specialty constrain applicability; the model may not invent policy from prior knowledge.")}</p><label>{text("学校要求说明文字", "Institution requirement text")}<textarea maxLength={30000} value={institutionRequirementText} onChange={(event) => { setInstitutionRequirementText(event.target.value); setRun(null); }} placeholder={text("粘贴正式文件中与期刊等级、分区、毕业、职称或结题相关的原文……", "Paste official text about journal tiers, partitions, graduation, evaluation, or project completion…")} /></label><label>{text("官方来源网址（仅保存在本机）", "Official source URL (stored locally only)")}<input type="url" inputMode="url" value={institutionSourceUrl} onChange={(event) => { setInstitutionSourceUrl(event.target.value); setRun(null); }} placeholder="https://…" /></label><div className="institution-consents"><label><input type="checkbox" checked={officialSourceConfirmed} onChange={(event) => { setOfficialSourceConfirmed(event.target.checked); setRun(null); }} />{text("我确认原文来自本校正式要求", "I confirm this text comes from an official institution policy")}</label><label><input type="checkbox" checked={ruleTransmissionConfirmed} onChange={(event) => { setRuleTransmissionConfirmed(event.target.checked); setRun(null); }} />{text("本次允许发送学校名称、学科、论文用途和脱敏后的规则原文", "Allow the institution name, discipline, manuscript purpose, and redacted policy text for this model call")}</label></div><small>{text("作者姓名、来源网址、联系方式、学号和论文正文均不发送；脱敏在 Rust 网络出口执行，并在本地记录传输范围。", "Author name, source URL, contact details, identifiers, and manuscript content are never sent. Redaction runs at the Rust network boundary and the transmission scope is audited locally.")}</small></section>
    <section className="journal-preferences" aria-labelledby="journal-preferences-heading"><h3 id="journal-preferences-heading">{text("调整推荐条件", "Adjust recommendation conditions")}</h3><div>
      <label>{text("研究方向", "Research area")}<select value={preferences.topic} onChange={(event) => update("topic", event.target.value as ResearchTopic)}>{topicOptions.map(([value,zh,en])=><option key={value} value={value}>{localize(locale,zh,en)}</option>)}</select></label>
      <label>{text("文章类型", "Article type")}<select value={preferences.articleType} onChange={(event) => update("articleType", event.target.value as ArticleTypePreference)}><option value="auto">{text("自动识别", "Auto detect")}</option><option value="research">{text("研究论文", "Research")}</option><option value="review">{text("综述", "Review")}</option><option value="application">{text("应用型", "Application")}</option></select></label>
      <label>{text("投稿语言", "Language")}<select value={preferences.language} onChange={(event) => update("language", event.target.value as PublicationLanguagePreference)}><option value="auto">{text("自动", "Auto")}</option><option value="chinese">{text("中文", "Chinese")}</option><option value="english">{text("英文", "English")}</option></select></label>
      <label>{text("目标策略", "Target strategy")}<select value={preferences.targetStrategy} onChange={(event) => update("targetStrategy", event.target.value as TargetStrategy)}><option value="reach">{text("冲刺", "Reach")}</option><option value="balanced">{text("均衡", "Balanced")}</option><option value="pragmatic">{text("务实", "Pragmatic")}</option></select></label>
      <label>{text("开放获取", "Open access")}<select value={preferences.openAccess} onChange={(event) => update("openAccess", event.target.value as OpenAccessPreference)}><option value="no_preference">{text("无偏好", "No preference")}</option><option value="prefer">{text("优先", "Prefer")}</option><option value="require">{text("必须", "Require")}</option></select></label>
    </div><button className="primary-button" type="button" disabled={busy || !profileComplete || !institutionTextReady} onClick={() => void calculate()}>{busy ? text("正在保存并计算…", "Saving and calculating…") : hasInstitutionText ? text("提取校规并计算推荐", "Extract rules and calculate") : text("保存档案并计算推荐", "Save profile and calculate")}<Icon name="arrow" /></button>{!profileComplete ? <p className="journal-profile-hint">{text("请完整填写五项背景信息，并选择未来的截止日期。", "Complete all five profile fields and choose a future deadline.")}</p> : hasInstitutionText && institutionRequirementText.trim().length < 40 ? <p className="journal-profile-hint">{text("学校要求原文至少需要 40 个字符。", "Institution requirement text must contain at least 40 characters.")}</p> : hasInstitutionText && !ruleTransmissionConfirmed ? <p className="journal-profile-hint">{text("模型抽取前需要确认本次发送范围。", "Confirm the transmission scope before model extraction.")}</p> : null}</section>
    {error ? <p className="inline-warning"><Icon name="warning" />{error}</p> : null}
    {run ? <><div className="institution-rule-status" role="status" data-verified={run.schoolRuleStatus === "verified_rule_set_applied"}><strong>{run.schoolRuleStatus === "verified_rule_set_applied" ? text("学校规则已核验并计入", "Verified institution rules applied") : run.schoolRuleStatus === "verified_rule_waiting_for_institution_directory_data" ? text("学校规则已提取 · 评价目录数据待核验", "Institution rules extracted · evaluation directory pending") : text("学校规则尚未核验 · 当前为候选初筛", "Institution rules unverified · provisional shortlist")}</strong><span>{text(`档案 v${run.recommendationProfile.profileVersion} · ${run.recommendationProfile.institution} · 剩余 ${run.deadlineDaysRemaining} 天`, `Profile v${run.recommendationProfile.profileVersion} · ${run.recommendationProfile.institution} · ${run.deadlineDaysRemaining} days remaining`)}</span></div><div className="journal-run-meta"><span>{text("本地推荐记录", "Local recommendation run")} {run.runId}</span><span>{text("稿件完备度", "Manuscript readiness")} {run.maturityScore}/100</span><span>{run.catalogVersion} · {run.catalogVerifiedDate}</span></div><div className="journal-columns"><JournalRecommendationList title={text("国内 3 家", "3 domestic journals")} items={run.domestic} locale={locale} /><JournalRecommendationList title={text("国际 3 家", "3 international journals")} items={run.international} locale={locale} /></div><PaneAction label={text("下一步", "Next")} title={text("确认版本与推荐依据", "Confirm version and recommendation basis")} copy={text("推荐记录已绑定投稿背景档案、当前稿件哈希和评分版本；机构评价目录只参与后台资格计算，不向界面公开原始目录。", "The run binds the context profile, manuscript hash, and scoring version. Institution evaluation directories are used only for backend eligibility checks; raw directory data is not exposed in the interface.")} buttonLabel={text("进入存证", "Continue to attestation")} onClick={onContinue} /></> : null}
  </>;
}

function JournalRecommendationList({ title, items, locale }: { title: string; items: JournalRecommendation[]; locale: Locale }) {
  const { text } = useI18n();
  return <section className="journal-result-group"><h3>{title}</h3><ol>{items.map((item,index)=><li key={item.id}><header><span>{String(index+1).padStart(2,"0")}</span><div><strong>{locale === "en" ? item.nameEn : item.name}</strong><p>{item.publisher}</p></div><em>{item.overallFit}</em></header><div className="journal-tags"><span>{item.rankTier}</span><span>{text("专业", "Specialty")} {item.scores.specialtyFit}</span><span>{text("用途", "Purpose")} {item.scores.purposeFit}</span><span>{text("时间", "Timing")} {item.scores.timeFeasibility}</span><span>{text(`规划 ${item.estimatedSubmissionPreparationDays} 天`, `${item.estimatedSubmissionPreparationDays}-day plan`)}</span><span>{item.scores.institutionRules === null ? text("校规待核验", "Institution rules pending") : text(`校规 ${item.scores.institutionRules}`, `Institution ${item.scores.institutionRules}`)}</span></div><p>{localizeBackendText(locale,item.reasons[0])}</p></li>)}</ol></section>;
}

function StructureCheckSummary({ report }: { report: StructureReport }) {
  const { text } = useI18n();
  return <section className="check-structure-summary"><header><div><span>{text("结构提取完成", "Structure extracted")}</span><h3>{report.title ?? text("未检测到标题", "No title detected")}</h3></div><strong>v{report.sourceSnapshotVersion}</strong></header><div className="metric-row"><Metric label={text("作者", "Authors")} value={report.authors.length} /><Metric label={text("章节", "Sections")} value={report.sections.length} /><Metric label={text("图", "Figures")} value={report.figureCount} /><Metric label={text("表", "Tables")} value={report.tableCount} /></div>{report.warnings.map((warning) => <p className="inline-warning" key={warning}><Icon name="warning" />{warning}</p>)}</section>;
}

function KnowledgeCandidatePreview({ snapshot, structureReport, compact = false }: { snapshot: AcademicKnowledgeBodySnapshot; structureReport?: StructureReport; compact?: boolean }) {
  const { text } = useI18n();
  const extraction = snapshot.extraction;
  const elements: Array<{ key: SemanticElementKind; label: string; value?: ExtractedKnowledgeElement }> = [
    { key: "claim", label: "Claim", value: extraction?.claim },
    { key: "scope", label: "Scope", value: extraction?.scope },
    { key: "method", label: "Method", value: extraction?.method },
    { key: "result", label: "Result", value: extraction?.result },
    { key: "evidence", label: "Evidence", value: extraction?.evidence },
  ];
  const candidateCount = elements.reduce((total, element) => total + (element.value?.candidates.length ?? 0), 0);
  const coverage = structureReport?.extractionCoverage ?? { textFragments: 0, tableFragments: 0, figureFragments: 0 };
  return <section className="knowledge-candidate-preview" data-compact={compact} aria-labelledby="knowledge-candidate-heading">
    <header><div><span>{text("本地确定性语义提取", "Local deterministic semantic extraction")}</span><h3 id="knowledge-candidate-heading">{text(`${candidateCount} 条知识候选`, `${candidateCount} knowledge candidates`)}</h3></div><strong>{text("待作者确认", "Author confirmation pending")}</strong></header>
    <p className="knowledge-extraction-coverage">{text(`已分析 ${coverage.textFragments} 个文本片段、${coverage.tableFragments} 个表格片段和 ${coverage.figureFragments} 个图片片段。`, `Analyzed ${coverage.textFragments} text fragments, ${coverage.tableFragments} table fragments, and ${coverage.figureFragments} figure fragments.`)}</p>
    <div className="knowledge-candidate-grid">{elements.map((element) => {
      const candidates = element.value?.candidates ?? [];
      return <article key={element.key} data-state={element.value?.state ?? "pending"}><header><h4>{element.label}</h4><span>{candidates.length > 0 ? text(`候选 v${element.value?.object.version ?? 0}`, `Candidate v${element.value?.object.version ?? 0}`) : text("未提取", "Not extracted")}</span></header>{candidates.length > 0 ? <ul>{candidates.slice(0, compact ? 1 : 3).map((candidate) => <li key={candidate.candidateId}><p>{candidate.text}</p><small>{candidate.sourceFragmentId ? `${candidate.sourceFragmentId} · ` : ""}{candidate.sourceLabel} · {candidate.modality.toUpperCase()} · {candidate.confidencePercent}%</small></li>)}</ul> : <p>{text("当前可解析内容中没有足够明确的语义片段；不会用占位文字冒充知识。", "No sufficiently explicit semantic passage was found in the parseable content; placeholders are not presented as knowledge.")}</p>}</article>;
    })}</div>
    <p className="knowledge-candidate-policy">{text("候选对象可以支持带不确定性说明的问答；只有作者逐条确认后，状态才会从 candidate 升级为 established。", "Candidates can support uncertainty-aware questions and answers. They become established only after item-level author confirmation.")}</p>
  </section>;
}

function DisciplineSelector({ catalog, selectedCode, loading, onChange }: { catalog: DisciplineCatalogItem[]; selectedCode: string; loading: boolean; onChange: (code: string) => void }) {
  const { locale, text } = useI18n();
  return <section className="discipline-selector" aria-labelledby="discipline-selector-heading"><label id="discipline-selector-heading" htmlFor="discipline-code">{text("学科索引分类", "Discipline classification")}</label><select id="discipline-code" value={selectedCode} disabled={loading} onChange={(event) => onChange(event.target.value)}><option value="">{loading ? text("正在读取分类…", "Loading classifications…") : text("请选择主要学科", "Choose the primary discipline")}</option>{catalog.map((item) => <option key={item.code} value={item.code}>{locale === "en" ? item.labelEn : `${item.label} · ${item.labelEn}`}</option>)}</select><p>{text("采用 ManuscriptDock Discipline Index v1.0；本次选择将记录为作者确认的 ClassificationAssignment。", "Uses ManuscriptDock Discipline Index v1.0; this selection is recorded as an author-confirmed ClassificationAssignment.")}</p></section>;
}

function LifecycleRecord({ label, id, hash, timestamp }: { label: string; id: string; hash: string; timestamp: number }) {
  const { locale, text } = useI18n();
  return <dl className="lifecycle-record"><div><dt>{label} ID</dt><dd>{id}</dd></div><div><dt>{text("记录指纹", "Record fingerprint")}</dt><dd>{hash}</dd></div><div><dt>{text("创建时间", "Created")}</dt><dd>{formatModifiedDate(timestamp, locale)}</dd></div><div><dt>{text("外部传输", "External transmission")}</dt><dd>{text("未发生", "None")}</dd></div></dl>;
}

function KnowledgeBodyOperation({ workspace, snapshot, record, structureReport }: { workspace: WorkspaceSummary; snapshot: AcademicKnowledgeBodySnapshot; record: KnowledgeBodyRecord; structureReport?: StructureReport }) {
  const { locale, text } = useI18n();
  const objects = snapshot.objects;
  const aiReview = snapshot.aiReviewReport;
  const architecture = snapshot.serviceArchitecture;
  const capabilityCount = architecture?.capabilityContracts.length ?? 0;
  const availableCapabilities = architecture?.capabilityContracts.filter((contract) => contract.availability !== "planned").length ?? 0;
  const reputationVersion = architecture?.validationRightsAndReputation.reputationRecord.version ?? 0;
  const classification = record.disciplineClassification;
  if (!classification) return null;
  return <><p className="workspace-created-status"><Icon name="check" />{text("知识体快照已固化", "Knowledge-body snapshot finalized")}</p><PanelHeading kicker={`${text("步骤 8 / 8 · 知识体快照", "Step 8 / 8 · Knowledge-body snapshot")} · S${snapshot.snapshotVersion}`} title={text("知识体与关联网络", "Knowledge body and relationship network")} copy={text("快照已绑定本次存证、投稿记录和作者确认的学科分类；当前仍只保存在本机。", "The snapshot binds this attestation, submission, and author-confirmed discipline and remains local.")} /><KnowledgeCandidatePreview snapshot={snapshot} structureReport={structureReport} compact /><section className="knowledge-identity-card" aria-labelledby="knowledge-identity-heading"><header><div><span>{text("稳定身份 · 不可变快照", "Stable identity · Immutable snapshot")}</span><h3 id="knowledge-identity-heading">{text("知识体哈希与学科索引", "Knowledge-body hash and discipline index")}</h3></div><strong>SHA-256</strong></header><dl><div><dt>{text("知识体哈希编码", "Knowledge-body hash")}</dt><dd><code>{record.recordHash}</code></dd></div><div><dt>{text("学科索引分类", "Discipline classification")}</dt><dd><strong>{locale === "en" ? classification.labelEn : classification.label}</strong><span>{classification.code}</span></dd></div><div><dt>{text("分类协议", "Classification protocol")}</dt><dd>ClassificationAssignment · v{classification.version}</dd></div><div><dt>{text("索引体系", "Index scheme")}</dt><dd>{classification.scheme} · v{classification.schemeVersion}</dd></div><div><dt>{text("确认状态", "Confirmation status")}</dt><dd>{text("学科分类已确认；语义候选待逐条确认", "Discipline confirmed; semantic candidates await item-level confirmation")}</dd></div><div><dt>KnowledgeBody ID</dt><dd>{snapshot.knowledgeBodyId}</dd></div><div><dt>{text("固化时间", "Finalized")}</dt><dd>{formatModifiedDate(record.finalizedUnixMs, locale)}</dd></div></dl><p>{text("该哈希覆盖知识体快照、学科分类、存证与投稿引用；身份长期稳定，内容更新形成新快照。信誉状态可独立变化，不会改写历史内容。", "This hash covers the knowledge snapshot, discipline classification, attestation, and submission references. Identity remains stable while content updates create new snapshots. Reputation may evolve independently without rewriting historical content.")}</p></section><ul className="knowledge-layers" aria-label={text("知识体五部分架构", "Five-part knowledge-body architecture")}><KnowledgeLayer title={text(`身份与版本 · Artifact v${objects.artifactVersion.version} · Snapshot S${objects.knowledgeBodySnapshot.version}`, `Identity & version · Artifact v${objects.artifactVersion.version} · Snapshot S${objects.knowledgeBodySnapshot.version}`)} copy={text("稳定 KnowledgeBody 身份、作者来源、不可变版本和替代/撤回状态", "Stable KnowledgeBody identity, creator provenance, immutable versions, and supersession or withdrawal state")} complete /><KnowledgeLayer title={text(`知识、边界与证据 · Claim v${objects.claim.version}`, `Knowledge, boundary & evidence · Claim v${objects.claim.version}`)} copy={text(`Scope v${objects.scope.version} · Method v${objects.method.version} · Result v${objects.result.version} · EvidenceRelation v${objects.evidenceRelation.version} · SourceAnchor v${objects.sourceAnchor.version}`, `Scope v${objects.scope.version} · Method v${objects.method.version} · Result v${objects.result.version} · EvidenceRelation v${objects.evidenceRelation.version} · SourceAnchor v${objects.sourceAnchor.version}`)} complete={objects.scope.version > 0 && objects.method.version > 0 && objects.result.version > 0} /><KnowledgeLayer title={text(`能力契约 · ${capabilityCount} 项`, `Capability contracts · ${capabilityCount}`)} copy={text(`${availableCapabilities} 项可用或需要运行时；明确输入、输出、前置条件、拒绝条件与证据追溯`, `${availableCapabilities} available or runtime-dependent; inputs, outputs, preconditions, refusal conditions, and evidence traceability are explicit`)} complete={capabilityCount > 0} /><KnowledgeLayer title={text("交互与执行运行时 · RuntimeProfile v1", "Interaction & execution runtime · RuntimeProfile v1")} copy={text("作者配置的模型只作为可替换协调层；每次外发单独授权", "Author-configured models are replaceable coordinators; every transmission requires per-call authorization")} complete={architecture !== undefined} /><KnowledgeLayer title={text(`验证、权利与信誉 · Reputation v${reputationVersion}`, `Validation, rights & reputation · Reputation v${reputationVersion}`)} copy={text(`AIReviewReport ${aiReview ? `v${aiReview.version}` : "v0"} · RightsPolicy v1；信誉独立于固定内容持续更新`, `AIReviewReport ${aiReview ? `v${aiReview.version}` : "v0"} · RightsPolicy v1; reputation evolves independently of fixed content`)} complete={architecture !== undefined} /></ul></>;
}

function VersionManager({ workspace, history, selectedVersion, candidate, note, notice, selecting, saving, restoring, onSelectCandidate, onNoteChange, onSave, onSelectVersion, onRestore, onContinue, continueReady }: { workspace: WorkspaceSummary; history: VersionHistory | null; selectedVersion: number | null; candidate: ManuscriptSummary | null; note: string; notice: string | null; selecting: boolean; saving: boolean; restoring: boolean; onSelectCandidate: () => void; onNoteChange: (note: string) => void; onSave: () => void; onSelectVersion: (version: number) => void; onRestore: (version: number) => void; onContinue: () => void; continueReady: boolean }) {
  const { locale, text } = useI18n();
  const currentVersion = history?.currentVersion ?? workspace.snapshotVersion;
  const selected = selectedVersion ?? currentVersion;
  const formatMatches = !candidate || candidate.kind === workspace.manuscript.kind;
  const versions = history ? [...history.versions].reverse() : [];
  return <>
    <PanelHeading kicker={text("步骤 4 / 8 · 本地版本库", "Step 4 / 8 · Local version library")} title={text("核验当前版本与历史", "Verify the current version and history")} copy={text("修订已保存为不可变版本。也可导入外部修改稿或安全恢复旧版；版本变化后必须重新检查。", "The revision is stored immutably. You can also import an external revision or safely restore an older version; any version change requires a new check.")} />
    <section className="version-save-card" aria-labelledby="version-save-heading">
      <div className="version-save-heading"><div><span>{text("新的修改稿", "New revision")}</span><h3 id="version-save-heading">{candidate ? candidate.name : text("尚未选择文件", "No file selected")}</h3>{candidate ? <p>{candidate.extension.toUpperCase()} · {formatBytes(candidate.sizeBytes)}</p> : <p>{text(`请选择与当前稿件相同类型的 ${workspace.manuscript.extension.toUpperCase()} 文件。`, `Choose a ${workspace.manuscript.extension.toUpperCase()} file matching the current manuscript type.`)}</p>}</div><button className="text-button" type="button" onClick={onSelectCandidate} disabled={selecting || saving}>{selecting ? text("正在打开…", "Opening…") : candidate ? text("重新选择", "Choose another") : text("选择修改稿", "Choose revision")}</button></div>
      {candidate ? <div className="version-note-field"><label htmlFor="version-note">{text("版本说明", "Version note")} <span>{text("可选", "Optional")}</span></label><input id="version-note" value={note} maxLength={200} onChange={(event) => onNoteChange(event.target.value)} placeholder={text("例如：补充方法与统计分析", "For example: expanded methods and statistical analysis")} /><small>{note.length} / 200</small></div> : null}
      {!formatMatches ? <p className="inline-warning" role="alert"><Icon name="warning" />{text("修改稿必须与当前稿件保持相同文件类型；格式转换应留在投稿输出中。", "The revision must use the same file type as the current manuscript; format conversion belongs in submission outputs.")}</p> : null}
      <button className="primary-button version-primary" type="button" onClick={candidate ? onSave : onSelectCandidate} disabled={selecting || saving || !formatMatches}>{saving ? text("正在保存…", "Saving…") : candidate ? text(`保存为 v${currentVersion + 1}`, `Save as v${currentVersion + 1}`) : text("选择修改后的稿件", "Choose revised manuscript")}<Icon name={candidate ? "check" : "upload"} /></button>
    </section>
    {notice ? <p className="version-notice" role="status"><Icon name="check" />{notice}</p> : null}
    <section className="version-history" aria-labelledby="version-history-heading"><header><div><span>{text("不可变时间线", "Immutable timeline")}</span><h3 id="version-history-heading">{text("论文版本", "Manuscript versions")}</h3></div><strong>{history ? text(`${history.versions.length} 个版本`, `${history.versions.length} versions`) : text("读取中…", "Loading…")}</strong></header>
      {versions.length > 0 ? <ol aria-label={text("论文版本时间线", "Manuscript version timeline")}>{versions.map((version) => {
        const isCurrent = version.version === currentVersion;
        const isSelected = version.version === selected;
        return <li key={version.version} data-current={isCurrent} data-selected={isSelected}><button className="version-row" type="button" aria-pressed={isSelected} onClick={() => onSelectVersion(version.version)}><span className="version-marker">v{version.version}</span><span className="version-copy"><strong>{version.note || versionOriginLabel(version, locale)}</strong><small>{version.manuscript.name} · {formatModifiedDate(version.createdUnixMs, locale)}</small></span><em>{isCurrent ? text("当前", "Current") : versionOriginLabel(version, locale)}</em></button>{isSelected && !isCurrent ? <button className="restore-button" type="button" onClick={() => onRestore(version.version)} disabled={restoring}>{restoring ? text("正在恢复…", "Restoring…") : text(`恢复 v${version.version} 为新版本`, `Restore v${version.version} as a new version`)}</button> : null}</li>;
      })}</ol> : <p className="version-loading">{text("正在读取本地版本记录…", "Loading local version records…")}</p>}
    </section>
    <BoundaryNote title={text("安全恢复", "Safe restoration")} copy={text("恢复旧版不会删除或覆盖任何内容，而是以当前版本为父节点创建一个新的版本。所有稿件只保存在本机。", "Restoring an older version never deletes or overwrites content. It creates a new version from the current head, and every manuscript stays on this device.")} />
    <button className="secondary-action" type="button" onClick={onContinue}>{continueReady ? text("继续创建本地存证", "Continue to local attestation") : text("重新检查当前版本", "Check the current version again")}<Icon name="arrow" /></button>
  </>;
}

function versionOriginLabel(version: ManuscriptVersionSummary, locale: Locale) {
  if (version.origin === "imported") return localize(locale, "初始导入", "Initial import");
  if (version.origin === "restored") return localize(locale, `从 v${version.restoredFromVersion ?? "?"} 恢复`, `Restored from v${version.restoredFromVersion ?? "?"}`);
  return localize(locale, "修改稿", "Revision");
}

function TargetRuleSelector({ ruleCatalog, selectedRulePackIds, loading, structureReady, onToggle, onContinue, actionLabel, disabled = false }: { ruleCatalog: RulePackCatalogItem[]; selectedRulePackIds: string[]; loading: boolean; structureReady: boolean; onToggle: (rulePackId: string) => void; onContinue: () => void; actionLabel?: string; disabled?: boolean }) {
  const { locale, text } = useI18n();
  const categories = [
    ["national_standard", text("中国国家标准", "Chinese national standards")],
    ["ethics", text("出版伦理与透明度", "Publishing ethics and transparency")],
    ["publisher", text("主流出版商", "Major publishers")],
    ["article_type", text("文章类型规范", "Article-type standards")],
    ["reporting_guideline", text("研究报告指南", "Research reporting guidelines")],
  ] as const;
  return <>
    <PanelHeading kicker={text("步骤 2 / 8 · 检查规则", "Step 2 / 8 · Check rules")} title={text("选择适用于这篇论文的标准", "Choose standards applicable to this manuscript")} copy={text("通用初投稿规则始终启用。只选择真实适用的国家标准、出版商和研究类型；具体期刊作者指南仍具有最高优先级。", "General initial-submission rules are always active. Select only applicable national, publisher, and study-type standards; the journal's own author instructions still take precedence.")} />
    {loading ? <p className="rule-catalog-loading">{text("正在校验并读取内置规则…", "Verifying and loading built-in rules…")}</p> : null}
    {!loading && ruleCatalog.length === 0 ? <BoundaryNote title={text("规则目录暂不可用", "Rule catalog unavailable")} copy={text("仍可使用通用初投稿检查；重新进入本页可再次读取内置目录。", "General initial-submission checks remain available; reopen this page to retry the built-in catalog.")} /> : null}
    <div className="rule-catalog" aria-label={text("可选出版标准", "Optional publication standards")}>
      {categories.map(([category, label]) => {
        const items = ruleCatalog.filter((item) => item.category === category);
        if (items.length === 0) return null;
        return <section className="rule-group" key={category}><header><h3>{label}</h3><span>{items.length}</span></header><div>{items.map((item) => {
          const selected = selectedRulePackIds.includes(item.id);
          return <button type="button" role="checkbox" aria-checked={selected} className="rule-option" data-selected={selected} key={item.id} onClick={() => onToggle(item.id)}>
            <span className="rule-check"><Icon name={selected ? "check" : "package"} /></span>
            <span className="rule-copy"><strong>{locale === "en" ? item.sourceLabelEn : item.sourceLabel}</strong><small>{locale === "en" ? item.descriptionEn : item.description}</small><em>v{item.version} · {text("覆盖", "Coverage")} {item.coverage} · {item.verifiedAt}</em></span>
          </button>;
        })}</div></section>;
      })}
    </div>
    <BoundaryNote title={text("覆盖边界", "Coverage boundary")} copy={text("这里内置的是标准与出版商级基线，不代表已经覆盖旗下每本期刊。报告会把不能可靠自动判断的事项列为“作者确认”。", "These are standards- and publisher-level baselines, not complete coverage of every journal. The report marks items that cannot be determined reliably as author confirmations.")} />
    <PaneAction label={text("当前组合", "Current composition")} title={selectedRulePackIds.length > 0 ? text(`已选择 ${selectedRulePackIds.length} 套增强规则`, `${selectedRulePackIds.length} enhanced rule pack(s) selected`) : text("仅使用通用投稿规则", "Use general submission rules only")} copy={text("规则在本机执行，不调用 AI，也不会发送论文。", "Rules run locally without AI calls or manuscript transmission.")} buttonLabel={structureReady ? (actionLabel ?? text("运行投稿检查", "Run submission checks")) : text("先提取结构", "Extract structure first")} disabled={disabled} onClick={onContinue} />
  </>;
}

function SubmissionElementsDesk({ catalog, draft, values, result, loading, saving, selectedPublisherCount, onValueChange, onSave, onContinue }: { catalog: SubmissionElementCatalog | null; draft: RevisionDraft | null; values: Record<string, string>; result: RevisionSet | null; loading: boolean; saving: boolean; selectedPublisherCount: number; onValueChange: (field: string, value: string) => void; onSave: () => void; onContinue: () => void }) {
  const { locale, text } = useI18n();
  if (loading || !catalog) return <EmptyStage icon="format" kicker={text("步骤 3 / 8 · 投稿优化修订台", "Step 3 / 8 · Submission Revision Desk")} title={text("正在整理投稿要素", "Preparing submission elements")} copy={text("正在本机组合已签名的出版社要求，不会调用 AI 或发送论文。", "Combining signed publisher requirements locally without AI calls or manuscript transmission.")} />;
  const groups = ["identity", "manuscript", "declarations", "files"];
  const editableCount = catalog.elements.filter((element) => element.editableField).length;
  const changedCount = draft?.fields.filter((field) => (values[field.field] ?? field.value).trim() !== field.value).length ?? 0;
  return <>
    <PanelHeading kicker={text("步骤 3 / 8 · 投稿优化修订台", "Step 3 / 8 · Submission Revision Desk")} title={selectedPublisherCount > 0 ? text("依据检查结果修订", "Revise from check findings") : text("通用投稿修订", "General submission revision")} copy={selectedPublisherCount > 0 ? text("出版社要素已合并；保存后会形成新版本并自动重做当前规则检查。", "Publisher elements are merged; saving creates a new version and automatically reruns the current checks.") : text("当前使用通用规则；可安全回写的字段仍可修订，具体期刊要求需作者继续核对。", "General rules are active; safe fields remain editable while journal-specific requirements still require author review.")} />
    {result ? <p className="revision-saved" role="status"><Icon name="check" />{text(`已保存为 v${result.outputVersion}，${result.changes.length} 项修改已记录来源`, `Saved as v${result.outputVersion}; provenance recorded for ${result.changes.length} change(s)`)}</p> : null}
    {draft && draft.fields.length > 0 ? <section className="revision-fields" aria-labelledby="revision-fields-heading"><header><div><span>{text(`基础版本 v${draft.baseVersion}`, `Base version v${draft.baseVersion}`)}</span><h3 id="revision-fields-heading">{text("可安全回写的字段", "Fields safe to write back")}</h3></div><strong>{draft.format.toUpperCase()}</strong></header>{draft.fields.map((field) => <div className="revision-field" key={field.field}><label htmlFor={`revision-${field.field}`}>{locale === "en" ? field.labelEn : field.label}</label>{field.field === "title" ? <input id={`revision-${field.field}`} value={values[field.field] ?? field.value} onChange={(event) => onValueChange(field.field, event.target.value)} disabled={!field.editable || saving} /> : <textarea id={`revision-${field.field}`} rows={field.field === "abstract" ? 5 : 2} value={values[field.field] ?? field.value} onChange={(event) => onValueChange(field.field, event.target.value)} disabled={!field.editable || saving} />}<small>{field.limitation ? (locale === "en" ? field.limitationEn : field.limitation) : text("作者修改 · 本机处理 · 保存前可在右侧核对差异", "Author edit · Local processing · Review the difference on the right before saving")}</small></div>)}</section> : null}
    {draft?.warnings.map((warning) => <p className="inline-warning" key={warning}><Icon name="warning" />{localizeBackendText(locale, warning)}</p>)}
    {catalog.elements.length > 0 ? <div className="submission-element-groups" aria-label={text("出版社投稿要素", "Publisher submission elements")}>{groups.map((group) => {
      const elements = catalog.elements.filter((element) => element.group === group);
      if (elements.length === 0) return null;
      return <section className="submission-element-group" key={group}><header><h3>{submissionElementGroupLabel(group, locale)}</h3><span>{elements.length}</span></header><ul>{elements.map((element) => <li key={element.id}><span className="element-state"><Icon name={element.editableField ? "format" : "check"} /></span><div><strong>{locale === "en" ? element.labelEn : element.label}</strong><p>{locale === "en" ? element.descriptionEn : element.description}</p><small>{element.editableField ? text("可进入结构化修订", "Structured revision available") : text("作者核对", "Author confirmation")}</small></div></li>)}</ul></section>;
    })}</div> : <div className="submission-elements-empty"><Icon name="target" /><p>{text("当前组合没有出版社级投稿要素；通用检查仍然可用。", "The current composition has no publisher-level elements; general checks remain available.")}</p></div>}
    <BoundaryNote title={text("可信边界", "Trust boundary")} copy={text(`共 ${catalog.elements.length} 项，其中 ${editableCount} 项已连接到后续结构化修订字段。所有来源在右侧只读显示。`, `${catalog.elements.length} elements are listed; ${editableCount} connect to structured revision fields. Every source is shown read-only on the right.`)} />
    <PaneAction label={changedCount > 0 ? text(`${changedCount} 项待保存`, `${changedCount} change(s) pending`) : text("下一步", "Next")} title={changedCount > 0 ? text(`保存为新版本 v${(draft?.baseVersion ?? 0) + 1}`, `Save as new version v${(draft?.baseVersion ?? 0) + 1}`) : text("核验论文版本", "Verify manuscript versions")} copy={changedCount > 0 ? text("保存后自动重提取并复查新版本；原稿与历史不会被覆盖。", "Saving automatically extracts and rechecks the new version; the source and history remain unchanged.") : text("没有待保存修改，可以继续查看当前版本与历史。", "There are no unsaved changes; continue to the current version and history.")} buttonLabel={changedCount > 0 ? (saving ? text("保存并复查中…", "Saving and rechecking…") : text("保存新版本并复查", "Save new version and recheck")) : text("进入版本", "Continue to versions")} disabled={saving} onClick={changedCount > 0 ? onSave : onContinue} />
  </>;
}

function submissionElementGroupLabel(group: string, locale: Locale) {
  const labels: Record<string, [string, string]> = {
    identity: ["作者身份", "Author identity"],
    manuscript: ["稿件正文", "Manuscript"],
    declarations: ["声明与伦理", "Declarations and ethics"],
    files: ["投稿文件", "Submission files"],
  };
  const label = labels[group] ?? [group, group];
  return localize(locale, label[0], label[1]);
}

function EvidencePane({ stage, workspace, structureReport, readinessReport, knowledgeBodySnapshot = null, knowledgeBodyRecord = null, attestation = null, submission = null, submissionExport = null, ruleCatalog = [], selectedRulePackIds = [], submissionElementCatalog = null, revisionDraft = null, revisionValues = {}, revisionResult = null, versionHistory = null, selectedVersion = null, versionComparison = null, isComparingVersions = false }: PaneProps) {
  const { locale, text } = useI18n();
  if (stage === "source") return <EvidenceFrame kicker={text("只读版本证据", "Read-only version evidence")} title={text("当前稿件身份", "Current manuscript identity")}><div className="document-sheet source-sheet"><span className="document-type">{workspace.manuscript.extension.toUpperCase()}</span><p className="document-title">{workspace.manuscript.name}</p><dl><div><dt>{text("内容指纹", "Content fingerprint")}</dt><dd>{workspace.contentHash}</dd></div><div><dt>{text("当前版本", "Current version")}</dt><dd>v{workspace.snapshotVersion}</dd></div><div><dt>{text("状态", "Status")}</dt><dd>{text("不可变；历史不会被覆盖", "Immutable; history is never overwritten")}</dd></div></dl></div></EvidenceFrame>;
  if (stage === "versions") return <VersionEvidence workspace={workspace} history={versionHistory} selectedVersion={selectedVersion} comparison={versionComparison} comparing={isComparingVersions} />;
  if (stage === "journals") return <EvidenceFrame kicker={text("目录与算法证据", "Catalog and algorithm evidence")} title={text("公开来源、本地评分", "Public sources, local scoring")}><div className="document-sheet source-sheet"><span className="document-type">LOCAL FIT V1.2</span><p className="document-title">computer-ai-2025.1</p><dl><div><dt>{text("国内目录", "Domestic directory")}</dt><dd>CCF 2025 · T1/T2/T3</dd></div><div><dt>{text("国际目录", "International directory")}</dt><dd>CCF AI · A/B/C</dd></div><div><dt>{text("机构评价目录", "Institution evaluation directory")}</dt><dd>{text("只参与后台资格判断，不公开原始目录内容", "Used only for backend eligibility checks; raw directory data is not exposed")}</dd></div><div><dt>{text("评分权重", "Weights")}</dt><dd>{text("校规 24 · 主题 18 · 专业 5 · 类型 8 · 完备度 10 · 语言 6 · 策略 8 · 用途 8 · 时间 10 · 开放获取 3", "Institution 24 · Topic 18 · Specialty 5 · Type 8 · Readiness 10 · Language 6 · Strategy 8 · Purpose 8 · Time 10 · OA 3")}</dd></div><div><dt>{text("学校规则", "Institution rules")}</dt><dd>{text("模型只能抽取客户提供的正式原文；目录条件核验后再计分", "The model extracts only supplied official text; directory conditions are scored only after verification")}</dd></div><div><dt>{text("身份公平", "Identity fairness")}</dt><dd>{text("学校排名和导师名气不参与声誉打分", "Institution rank and adviser fame are never prestige scores")}</dd></div><div><dt>{text("论文实力", "Scholarly strength")}</dt><dd>{text("当前仅有结构完备度；创新与证据强度等待 PWC 版本化审核", "Currently structural readiness only; innovation and evidence strength await versioned PWC review")}</dd></div><div><dt>{text("外部传输", "External transmission")}</dt><dd>{text("每次模型抽取单独确认、出口脱敏并审计", "Each extraction requires consent, boundary redaction, and audit")}</dd></div></dl><small>{text("当前计算的是最适合的投稿准备组合，不是录用概率。进入更高层级的学术判断必须引用可追溯的 PWC 审核档案。", "This computes the best current-fit submission-preparation set, not acceptance odds. Higher-tier scholarly judgments must cite a traceable PWC review profile.")}</small></div></EvidenceFrame>;
  if (stage === "check" && !readinessReport) return <EvidenceFrame kicker={text("结构证据", "Structure evidence")} title={text("论文轮廓", "Manuscript outline")}>{structureReport ? <div className="document-sheet"><p className="document-overline">DETERMINISTIC EXTRACTION · V{structureReport.analysisVersion}</p><p className="document-title">{structureReport.title ?? text("未检测到论文标题", "No manuscript title detected")}</p>{structureReport.authors.length > 0 ? <p className="document-authors">{structureReport.authors.join(" · ")}</p> : null}<p className="document-meta">{structureReport.pageCount ? `${structureReport.pageCount} ${text("页", "pages")}` : `${structureReport.wordCount} ${text("词元", "words")}`} · {text("源快照", "Source snapshot")} v{structureReport.sourceSnapshotVersion}</p>{structureReport.abstractText ? <section className="abstract-evidence"><h3>{text("识别到的摘要", "Detected abstract")}</h3><p>{structureReport.abstractText}</p></section> : null}{structureReport.sections.length > 0 ? <ol className="section-outline" aria-label={text("检测到的章节", "Detected sections")}>{structureReport.sections.slice(0, 16).map((section, index) => <li key={`${section.level}-${section.heading}-${index}`} style={{ "--section-level": section.level } as CSSProperties}><span>{String(index + 1).padStart(2, "0")}</span><strong>{section.heading}</strong></li>)}</ol> : <EvidenceEmpty copy={text("没有形成可靠章节轮廓，请结合警告人工确认。", "No reliable section outline was formed; review warnings and confirm manually.")} />}</div> : <EvidenceEmpty copy={text("完成本地结构提取后，这里显示标题、作者、摘要和章节证据。", "After local extraction, title, author, abstract, and section evidence appears here.")} />}</EvidenceFrame>;
  if (stage === "check") {
    const selected = ruleCatalog.filter((item) => selectedRulePackIds.includes(item.id));
    return <EvidenceFrame kicker={text("检查证据", "Check evidence")} title={text("来源、完整性与输出", "Sources, integrity, and output")}>{readinessReport ? <><div className="outcome-banner" data-outcome={readinessReport.outcome}><Icon name={readinessReport.outcome === "ready" ? "check" : "warning"} /><div><strong>{outcomeLabel(readinessReport.outcome, locale)}</strong><span>{text("输出快照", "Output snapshot")} v{readinessReport.outputSnapshotVersion}</span></div></div><ul className="provenance-list">{readinessReport.rulePacks.map((pack) => <li key={pack.id}><span><Icon name={pack.signatureVerified ? "check" : "warning"} /></span><div><strong>{locale === "en" && pack.sourceLabelEn ? pack.sourceLabelEn : localizeBackendText(locale, pack.sourceLabel)}</strong><p>v{pack.version} · {text("覆盖等级", "Coverage")} {pack.coverage} · {pack.signatureVerified ? text("完整性已校验", "Integrity verified") : text("完整性异常", "Integrity issue")}</p></div></li>)}</ul></> : selected.length > 0 ? <ul className="provenance-list">{selected.map((item) => <li key={item.id}><span><Icon name="check" /></span><div><strong>{locale === "en" ? item.sourceLabelEn : item.sourceLabel}</strong><p>v{item.version} · {text("签名完整性已校验", "Signature integrity verified")}</p></div></li>)}</ul> : <EvidenceEmpty copy={text("通用论文结构和初投稿规则仍会启用。", "General manuscript-structure and initial-submission rules remain active.")} />}</EvidenceFrame>;
  }
  if (stage === "revision") {
    const pending = revisionResult?.changes ?? revisionDraft?.fields.filter((field) => (revisionValues[field.field] ?? field.value).trim() !== field.value).map((field) => ({ field: field.field, before: field.value, after: revisionValues[field.field] ?? field.value, basis: "author_edit", status: "candidate" })) ?? [];
    return <EvidenceFrame kicker={text("修订证据", "Revision evidence")} title={pending.length > 0 ? text("修改前后", "Before and after") : text("要素来源与完整性", "Element sources and integrity")}>{pending.length > 0 ? <><div className="revision-diff-meta"><strong>{revisionResult ? `v${revisionResult.baseVersion} → v${revisionResult.outputVersion}` : text(`基于 v${revisionDraft?.baseVersion}`, `Based on v${revisionDraft?.baseVersion}`)}</strong><span>{revisionResult ? text("已保存", "Saved") : text("保存前预览", "Pre-save preview")}</span></div><ol className="revision-diff-list">{pending.map((change) => <li key={change.field}><strong>{revisionDraft?.fields.find((field) => field.field === change.field)?.[locale === "en" ? "labelEn" : "label"] ?? change.field}</strong><div><span>{change.before}</span><Icon name="arrow" /><span>{change.after}</span></div></li>)}</ol></> : submissionElementCatalog?.rulePacks.length ? <ul className="provenance-list">{submissionElementCatalog.rulePacks.map((pack) => <li key={pack.id}><span><Icon name="check" /></span><div><strong>{locale === "en" ? pack.sourceLabelEn : pack.sourceLabel}</strong><p>v{pack.version} · {text("完整性已校验", "Integrity verified")}</p></div></li>)}</ul> : <EvidenceEmpty copy={text("修改字段后，这里会显示保存前差异。", "Edit a field to preview the difference before saving.")} />}</EvidenceFrame>;
  }
  if (stage === "attestation") return <EvidenceFrame kicker={text("存证证据", "Attestation evidence")} title={attestation ? text("不可变本地存证", "Immutable local attestation") : text("将要绑定的对象", "Objects to be bound")}>{attestation ? <LifecycleEvidence id={attestation.attestationId} hash={attestation.recordHash} items={[[text("稿件版本", "Manuscript version"), `v${attestation.manuscriptVersion}`], [text("检查报告", "Check report"), attestation.readinessReportId], [text("输出快照", "Output snapshot"), `v${attestation.readinessOutputSnapshotVersion}`]]} /> : readinessReport ? <div className="package-preview"><span>LOCAL ATTESTATION</span><h2>v{workspace.snapshotVersion}</h2><p>{outcomeLabel(readinessReport.outcome, locale)}</p><dl><div><dt>{text("稿件指纹", "Manuscript fingerprint")}</dt><dd>{workspace.contentHash.slice(0, 16)}</dd></div><div><dt>{text("检查报告", "Check report")}</dt><dd>{readinessReport.reportId.slice(0, 16)}</dd></div></dl><small>{text("仅本机 · 作者确认后创建", "Local only · Created after author confirmation")}</small></div> : <EvidenceEmpty copy={text("完成当前版本检查后才能创建存证。", "Complete checks for the current version before attestation.")} />}</EvidenceFrame>;
  if (stage === "submission") return <EvidenceFrame kicker={text("投稿证据", "Submission evidence")} title={submission ? text("已登记投稿", "Submission recorded") : text("投稿交付包", "Submission handoff")}>{submission ? <LifecycleEvidence id={submission.submissionId} hash={submission.recordHash} items={[[text("目标", "Target"), submission.target], [text("回执", "Receipt"), submission.receipt ?? text("未填写", "Not provided")], [text("绑定存证", "Bound attestation"), submission.attestationId]]} /> : <div className="package-preview"><span>MANUSCRIPTDOCK</span><h2>{structureReport?.title ?? workspace.manuscript.name}</h2><p>{text("作者控制的投稿交付", "Author-controlled submission handoff")}</p><dl><div><dt>{text("稿件版本", "Manuscript version")}</dt><dd>v{workspace.snapshotVersion}</dd></div><div><dt>{text("存证", "Attestation")}</dt><dd>{attestation?.attestationId.slice(0, 12) ?? text("未完成", "Not ready")}</dd></div><div><dt>{text("导出状态", "Export status")}</dt><dd>{submissionExport ? submissionExport.packageName : text("尚未导出", "Not exported")}</dd></div></dl><small>{text("导出不等于已投稿", "Export does not mean submitted")}</small></div>}</EvidenceFrame>;
  if (stage === "knowledge") return <><EvidenceFrame kicker={text("对象与声明证据", "Object and assertion evidence")} title={knowledgeBodyRecord ? text("已固化知识体", "Finalized knowledge body") : text("知识体预览", "Knowledge-body preview")}><KnowledgeSpatialMap workspace={workspace} structureReport={structureReport} readinessReport={readinessReport} knowledgeBodySnapshot={knowledgeBodySnapshot} /></EvidenceFrame><KnowledgeDialoguePanel workspace={workspace} knowledgeBodyRecord={knowledgeBodyRecord} /></>;
  return <EvidenceFrame kicker={text("流程证据", "Lifecycle evidence")} title={text("等待当前步骤产物", "Waiting for this stage's output")}><EvidenceEmpty copy={text("完成操作后，这里显示不可变证据和来源。", "Complete the action to see immutable evidence and provenance here.")} /></EvidenceFrame>;
}

function LifecycleEvidence({ id, hash, items }: { id: string; hash: string; items: Array<[string, string]> }) {
  const { text } = useI18n();
  return <div className="document-sheet source-sheet"><span className="document-type">VERIFIED</span><p className="document-title">{id}</p><dl>{items.map(([label, value]) => <div key={label}><dt>{label}</dt><dd>{value}</dd></div>)}<div><dt>{text("记录指纹", "Record fingerprint")}</dt><dd>{hash}</dd></div><div><dt>{text("外部传输", "External transmission")}</dt><dd>{text("未发生", "None")}</dd></div></dl></div>;
}

function VersionEvidence({ workspace, history, selectedVersion, comparison, comparing }: { workspace: WorkspaceSummary; history: VersionHistory | null; selectedVersion: number | null; comparison: VersionComparison | null; comparing: boolean }) {
  const { locale, text } = useI18n();
  const currentVersion = history?.currentVersion ?? workspace.snapshotVersion;
  const selected = history?.versions.find((version) => version.version === (selectedVersion ?? currentVersion));
  if (!history) return <EvidenceFrame kicker={text("版本证据", "Version evidence")} title={text("正在读取版本库", "Loading version library")}><EvidenceEmpty copy={text("正在校验本地版本清单和内容指纹。", "Verifying the local version list and content fingerprints.")} /></EvidenceFrame>;
  if (comparing) return <EvidenceFrame kicker={text("确定性比较", "Deterministic comparison")} title={text("正在比较版本", "Comparing versions")}><EvidenceEmpty copy={text("正在本机提取两个版本的标题、章节、字数、图表和声明差异。", "Extracting title, section, word, figure, table, and declaration differences locally.")} /></EvidenceFrame>;
  if (!comparison) return <EvidenceFrame kicker={text("版本证据", "Version evidence")} title={text(`当前版本 v${currentVersion}`, `Current version v${currentVersion}`)}><div className="document-sheet source-sheet"><span className="document-type">{selected?.manuscript.extension.toUpperCase() ?? workspace.manuscript.extension.toUpperCase()}</span><p className="document-title">{selected?.manuscript.name ?? workspace.manuscript.name}</p><p className="document-meta">{selected ? versionOriginLabel(selected, locale) : text("当前版本", "Current version")} · {selected ? formatModifiedDate(selected.createdUnixMs, locale) : ""}</p><dl><div><dt>{text("内容指纹", "Content fingerprint")}</dt><dd>{selected?.contentHash ?? workspace.contentHash}</dd></div><div><dt>{text("版本状态", "Version status")}</dt><dd>{text("不可变且已校验", "Immutable and verified")}</dd></div><div><dt>{text("外部传输", "External transfer")}</dt><dd>{text("未发生", "None")}</dd></div></dl></div></EvidenceFrame>;
  const changes = comparison.addedSections.length + comparison.removedSections.length + comparison.addedDeclarations.length + comparison.removedDeclarations.length + Math.abs(comparison.wordCountDelta) + Math.abs(comparison.figureCountDelta) + Math.abs(comparison.tableCountDelta);
  return <EvidenceFrame kicker={text("确定性比较", "Deterministic comparison")} title={`v${comparison.fromVersion} → v${comparison.toVersion}`}><div className="version-diff-summary"><strong>{comparison.identical ? text("两个版本内容相同", "The versions are identical") : text("检测到稿件变化", "Manuscript changes detected")}</strong><p>{text("比较在本机完成；没有调用 AI，也没有发送论文。", "The comparison ran locally without AI calls or manuscript transmission.")}</p></div><div className="diff-title-pair"><div><span>v{comparison.fromVersion}</span><strong>{comparison.titleBefore ?? text("未检测到标题", "No title detected")}</strong></div><Icon name="arrow" /><div><span>v{comparison.toVersion}</span><strong>{comparison.titleAfter ?? text("未检测到标题", "No title detected")}</strong></div></div><div className="metric-row diff-metrics" aria-label={text("版本变化统计", "Version change metrics")}><DeltaMetric label={text("词元", "Words")} value={comparison.wordCountDelta} /><DeltaMetric label={text("图", "Figures")} value={comparison.figureCountDelta} /><DeltaMetric label={text("表", "Tables")} value={comparison.tableCountDelta} /><Metric label={text("变化量", "Change units")} value={changes} /></div><VersionChangeList title={text("新增章节", "Added sections")} items={comparison.addedSections} empty={text("无新增章节", "No added sections")} tone="added" /><VersionChangeList title={text("移除章节", "Removed sections")} items={comparison.removedSections} empty={text("无移除章节", "No removed sections")} tone="removed" />{comparison.addedDeclarations.length > 0 || comparison.removedDeclarations.length > 0 ? <VersionChangeList title={text("声明变化", "Declaration changes")} items={[...comparison.addedDeclarations.map((item) => `+ ${item}`), ...comparison.removedDeclarations.map((item) => `− ${item}`)]} empty="" tone="neutral" /> : null}<dl className="diff-hashes"><div><dt>v{comparison.fromVersion} Hash</dt><dd>{comparison.fromContentHash}</dd></div><div><dt>v{comparison.toVersion} Hash</dt><dd>{comparison.toContentHash}</dd></div></dl></EvidenceFrame>;
}

function DeltaMetric({ label, value }: { label: string; value: number }) {
  return <div><span>{label}</span><strong>{value > 0 ? `+${value}` : value}</strong></div>;
}

function VersionChangeList({ title, items, empty, tone }: { title: string; items: string[]; empty: string; tone: "added" | "removed" | "neutral" }) {
  return <section className="version-change-group" data-tone={tone}><h3>{title}</h3>{items.length > 0 ? <ul>{items.map((item) => <li key={item}><span>{tone === "added" ? "+" : tone === "removed" ? "−" : "·"}</span>{item}</li>)}</ul> : <p>{empty}</p>}</section>;
}

function PanelHeading({ kicker, title, copy }: { kicker: string; title: string; copy: string }) { return <header className="panel-heading"><p>{kicker}</p><h2>{title}</h2><span>{copy}</span></header>; }
function Metric({ label, value }: { label: string; value: number }) { return <div><span>{label}</span><strong>{value}</strong></div>; }
function Presence({ label, present }: { label: string; present: boolean }) { const { text } = useI18n(); return <li data-present={present}><Icon name={present ? "check" : "warning"} /><span>{label}</span><strong>{present ? text("已检测", "Detected") : text("待确认", "Confirm")}</strong></li>; }
function BoundaryNote({ title, copy }: { title: string; copy: string }) { return <div className="boundary-note"><Icon name="lock" /><div><strong>{title}</strong><p>{copy}</p></div></div>; }
function PaneAction({ label, title, copy, buttonLabel, disabled = false, onClick }: { label: string; title: string; copy: string; buttonLabel: string; disabled?: boolean; onClick: () => void }) { return <div className="pane-action"><div><span>{label}</span><h3>{title}</h3><p>{copy}</p></div><button className="primary-button" type="button" disabled={disabled} onClick={onClick}>{buttonLabel}<Icon name="arrow" /></button></div>; }
function EmptyStage({ icon, kicker, title, copy, actionLabel, disabled = false, onAction }: { icon: "structure" | "target" | "format" | "review" | "package"; kicker: string; title: string; copy: string; actionLabel?: string; disabled?: boolean; onAction?: () => void }) { const { text } = useI18n(); return <div className="empty-stage"><span className="empty-stage-icon" aria-hidden="true"><Icon name={icon} /></span><p>{kicker}</p><h2>{title}</h2><span>{copy}</span>{actionLabel && onAction ? <button className="primary-button" type="button" disabled={disabled} onClick={onAction}>{actionLabel}<Icon name="arrow" /></button> : <em>{text("已进入产品路线，当前版本不执行此操作", "Planned for the product roadmap; this version does not run the action")}</em>}</div>; }
function EvidenceFrame({ kicker, title, children }: { kicker: string; title: string; children: ReactNode }) { const { text } = useI18n(); return <div className="evidence-frame"><header><div><p>{kicker}</p><h2>{title}</h2></div><span className="read-only-badge"><Icon name="lock" />{text("只读", "Read-only")}</span></header>{children}</div>; }
function EvidenceEmpty({ copy }: { copy: string }) { return <div className="evidence-empty"><Icon name="file" /><p>{copy}</p></div>; }
function KnowledgeLayer({ title, copy, complete = false }: { title: string; copy: string; complete?: boolean }) { const { text } = useI18n(); return <li data-complete={complete}><span><Icon name={complete ? "check" : "package"} /></span><div><strong>{title}</strong><p>{copy}</p></div><em>{complete ? text("已进入知识体", "In knowledge body") : text("等待后续能力", "Waiting for planned capability")}</em></li>; }

function relationKindLabel(kind: RelationKind, locale: Locale) {
  const labels: Record<RelationKind, [string, string]> = {
    citation: ["显式引用", "Citation"],
    claim_relation: ["Claim 语义关系", "Claim relation"],
    evidence_relation: ["证据关联", "Evidence relation"],
    method_transfer: ["方法迁移", "Method transfer"],
    reproduction: ["复现关联", "Reproduction"],
    alignment: ["概念与身份映射", "Alignment"],
    version_relation: ["版本与更正", "Version relation"],
    classification: ["学科索引关联", "Classification"],
  };
  return localize(locale, labels[kind][0], labels[kind][1]);
}

function relationProtocolLabel(kind: RelationKind) {
  const protocols: Record<RelationKind, string> = {
    citation: "CitationAssertion",
    claim_relation: "ClaimRelationAssertion",
    evidence_relation: "EvidenceRelation",
    method_transfer: "MethodRelationAssertion",
    reproduction: "ReproductionAssertion",
    alignment: "AlignmentAssertion",
    version_relation: "VersionRelation",
    classification: "ClassificationAssignment",
  };
  return protocols[kind];
}

type KnowledgeView = "single" | "pair" | "network";

function KnowledgeSpatialMap({ workspace, knowledgeBodySnapshot = null }: Omit<PaneProps, "stage">) {
  const { locale, text } = useI18n();
  const [view, setView] = useState<KnowledgeView>("single");
  const network = knowledgeBodySnapshot?.network;
  const bodyCount = network?.bodies.length ?? 1;
  const availableView = view === "pair" ? bodyCount >= 2 : view === "network" ? bodyCount >= 3 : true;
  const claim = knowledgeBodySnapshot?.claim;
  const objects = knowledgeBodySnapshot?.objects;
  const aiReview = knowledgeBodySnapshot?.aiReviewReport;
  const architecture = knowledgeBodySnapshot?.serviceArchitecture;
  const previousReviewVersions = (knowledgeBodySnapshot?.aiReviewHistory.versions ?? []).filter((report) => report.version !== aiReview?.version).map((report) => `v${report.version}`).join(" · ");
  const capabilityContracts = architecture?.capabilityContracts ?? [];
  const layers = [
    { key: "identity", label: text("身份与版本", "Identity & version"), version: `S${objects?.knowledgeBodySnapshot.version ?? knowledgeBodySnapshot?.snapshotVersion ?? 1}`, state: text(`Artifact v${objects?.artifactVersion.version ?? workspace.snapshotVersion} · 稳定 ID`, `Artifact v${objects?.artifactVersion.version ?? workspace.snapshotVersion} · Stable ID`), complete: true },
    { key: "knowledge", label: text("知识、边界与证据", "Knowledge, boundary & evidence"), version: `Claim v${claim?.claim.version ?? 1}`, state: `Scope v${objects?.scope.version ?? 0} · Method v${objects?.method.version ?? 0} · Result v${objects?.result.version ?? 0} · EvidenceRelation v${objects?.evidenceRelation.version ?? 0} · Anchor v${objects?.sourceAnchor.version ?? workspace.snapshotVersion}`, complete: (objects?.scope.version ?? 0) > 0 && (objects?.method.version ?? 0) > 0 },
    { key: "capability", label: text("能力契约", "Capability contracts"), version: `v1 · ${capabilityContracts.length}`, state: text("输入 · 输出 · 前置 · 拒绝", "Input · Output · Preconditions · Refusal"), complete: capabilityContracts.length > 0 },
    { key: "runtime", label: text("交互与执行运行时", "Interaction & runtime"), version: "RuntimeProfile · v1", state: text("可替换模型 · 单次授权", "Replaceable model · Per-call consent"), complete: architecture !== undefined },
    { key: "trust", label: text("验证、权利与信誉", "Validation, rights & reputation"), version: `Reputation · v${architecture?.validationRightsAndReputation.reputationRecord.version ?? 0}`, state: aiReview ? text(`AIReview v${aiReview.version}${previousReviewVersions ? ` · 历史 ${previousReviewVersions}` : ""}`, `AIReview v${aiReview.version}${previousReviewVersions ? ` · History ${previousReviewVersions}` : ""}`) : text("Rights v1 · 审核待建立", "Rights v1 · Review pending"), complete: architecture !== undefined },
  ];
  return (
    <div className="knowledge-space">
      <div className="knowledge-view-switch" role="tablist" aria-label={text("知识体网络层级", "Knowledge-network level")}>
        {(["single", "pair", "network"] as KnowledgeView[]).map((item, index) => {
          const enabled = item === "single" || (item === "pair" ? bodyCount >= 2 : bodyCount >= 3);
          const labels = [text("1. 单一知识体", "1. One body"), text("2. 两体关联", "2. Two bodies"), text("3. 关联网络", "3. Network")];
          return <button key={item} type="button" role="tab" aria-selected={view === item} disabled={!enabled} title={enabled ? labels[index] : text("建立足够的声明关系后可用", "Available after enough asserted relationships exist")} onClick={() => setView(item)}>{labels[index]}</button>;
        })}
      </div>
      {view === "single" ? (
        <div className="knowledge-space-visual knowledge-service-space" role="img" aria-label={text(`单篇论文知识体空间视图。稳定 KnowledgeBody 身份包含不可变内容快照 S${objects?.knowledgeBodySnapshot.version ?? knowledgeBodySnapshot?.snapshotVersion ?? 1}，中心是 Claim v${claim?.claim.version ?? 1} 十二面体。五条空间连接分别指向身份与版本、知识边界与证据、能力契约、可替换交互运行时，以及独立变化的验证权利与信誉记录。`, `Single-paper KnowledgeBody spatial view. A stable KnowledgeBody identity contains immutable content snapshot S${objects?.knowledgeBodySnapshot.version ?? knowledgeBodySnapshot?.snapshotVersion ?? 1}, centered on a Claim v${claim?.claim.version ?? 1} dodecahedron. Five spatial links lead to identity and version, knowledge boundary and evidence, capability contracts, a replaceable interaction runtime, and independently evolving validation, rights, and reputation records.`)}>
          <span className="knowledge-snapshot-label" aria-hidden="true">KnowledgeBody · {text("稳定身份", "stable identity")} · Snapshot S{objects?.knowledgeBodySnapshot.version ?? knowledgeBodySnapshot?.snapshotVersion ?? 1}</span>
          <span className="knowledge-content-boundary" aria-hidden="true">{text("固定内容快照", "Immutable content snapshot")}</span>
          <svg className="claim-connections" viewBox="0 0 600 460" preserveAspectRatio="none" aria-hidden="true">
            <line x1="300" y1="230" x2="300" y2="68" />
            <line x1="300" y1="230" x2="112" y2="160" />
            <line x1="300" y1="230" x2="488" y2="160" />
            <line x1="300" y1="230" x2="438" y2="372" className="runtime-connection" />
            <line x1="300" y1="230" x2="162" y2="372" className="reputation-connection" />
          </svg>
          <div className="claim-center" aria-hidden="true">
            <ClaimDodecahedron />
            <span className="claim-core"><strong>Claim · v{claim?.claim.version ?? 1}</strong><small>{text("知识核心 · 边界受限", "Knowledge core · Bounded")}</small></span>
          </div>
          {layers.map((layer) => <div className={`service-layer-node service-layer-${layer.key}`} data-complete={layer.complete} key={layer.key} aria-hidden="true"><span className="service-layer-sphere"><strong>{layer.label}</strong><small>{layer.version}</small><em title={layer.state}>{layer.state}</em></span></div>)}
        </div>
      ) : availableView && network ? <KnowledgeNetworkCanvas bodies={view === "pair" ? network.bodies.slice(0, 2) : network.bodies} assertions={network.assertions} view={view} /> : null}
      <p className="knowledge-space-note">{view === "single" ? text("单一知识体是具有稳定身份、明确知识边界、可验证证据、能力契约和可替换运行时的知识服务单元。内容快照不可变；信誉记录可独立更新；v0 表示尚未正式建立。", "A single KnowledgeBody is a knowledge-service unit with stable identity, explicit boundaries, verifiable evidence, capability contracts, and a replaceable runtime. Content snapshots are immutable; reputation records evolve independently; v0 means not yet established.") : text("圆形边界表示知识体自身边界；绿色菱形表示带依据、状态和版本的声明对象。相似度不会自动成为关系。", "Circular boundaries preserve each knowledge body; green diamonds are versioned assertions with basis and status. Similarity never becomes a relationship automatically.")}</p>
    </div>
  );
}

const MODEL_SLOT_ROLES: ModelSlotRole[] = ["primary", "fallback_1", "fallback_2"];
const DEEPSEEK_PRESET = { providerLabel: "DeepSeek", baseUrl: "https://api.deepseek.com", model: "deepseek-v4-flash" } as const;
const INQUIRY_TARGETS: KnowledgeInquiryTarget[] = ["knowledge_body", "claim", "scope", "method", "result", "evidence_relation", "source_anchor", "capability_contract", "rights_reputation", "ai_review_report", "provenance"];

function emptyModelSlot(role: ModelSlotRole): ModelSlotDraft {
  return { role, enabled: false, providerLabel: "", baseUrl: "", model: "", hasApiKey: false, apiKey: "", clearApiKey: false };
}

function modelSlotLabel(role: ModelSlotRole, locale: Locale) {
  if (role === "primary") return localize(locale, "主模型", "Primary");
  if (role === "fallback_1") return localize(locale, "备选模型 1", "Fallback 1");
  return localize(locale, "备选模型 2", "Fallback 2");
}

function inquiryStanceLabel(stance: KnowledgeInquiryStance, locale: Locale) {
  if (stance === "recognition") return localize(locale, "认可", "Recognition");
  if (stance === "challenge") return localize(locale, "挑战", "Challenge");
  return localize(locale, "疑问", "Question");
}

function inquiryTargetLabel(target: KnowledgeInquiryTarget, locale: Locale) {
  const labels: Record<KnowledgeInquiryTarget, [string, string]> = {
    knowledge_body: ["知识体整体", "Knowledge body"], claim: ["Claim 主张", "Claim"], scope: ["Scope 适用范围", "Scope"], method: ["Method 方法", "Method"], result: ["Result 结果", "Result"], evidence_relation: ["EvidenceRelation 证据关系", "EvidenceRelation"], source_anchor: ["SourceAnchor 来源锚点", "SourceAnchor"], capability_contract: ["CapabilityContract 能力契约", "Capability contract"], rights_reputation: ["权利与信誉记录", "Rights and reputation"], ai_review_report: ["AIReviewReport 审核报告", "AIReviewReport"], provenance: ["Provenance 来源记录", "Provenance"],
  };
  return localize(locale, labels[target][0], labels[target][1]);
}

function isDeepSeekDocumentationUrl(value: string) {
  try {
    return new URL(value).hostname.toLowerCase() === "api-docs.deepseek.com";
  } catch {
    return false;
  }
}

function KnowledgeDialoguePanel({ workspace, knowledgeBodyRecord }: { workspace: WorkspaceSummary; knowledgeBodyRecord: KnowledgeBodyRecord | null }) {
  const { locale, text } = useI18n();
  const [activeTab, setActiveTab] = useState<"owner" | "external">("owner");
  const [ledger, setLedger] = useState<KnowledgeDialogueLedger | null>(null);
  const [settings, setSettings] = useState<ModelSettingsSummary | null>(null);
  const [slotDrafts, setSlotDrafts] = useState<ModelSlotDraft[]>(MODEL_SLOT_ROLES.map(emptyModelSlot));
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [question, setQuestion] = useState("");
  const [stance, setStance] = useState<KnowledgeInquiryStance>("question");
  const [target, setTarget] = useState<KnowledgeInquiryTarget>("knowledge_body");
  const [isLoading, setIsLoading] = useState(false);
  const [isSavingSettings, setIsSavingSettings] = useState(false);
  const [isAsking, setIsAsking] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const applySettings = (summary: ModelSettingsSummary) => {
    setSettings(summary);
    setSlotDrafts(MODEL_SLOT_ROLES.map((role) => {
      const slot = summary.slots.find((item) => item.role === role);
      return slot ? { ...slot, apiKey: "", clearApiKey: false } : emptyModelSlot(role);
    }));
  };

  useEffect(() => {
    let active = true;
    setIsLoading(true);
    setError(null);
    const dialogueRequest = knowledgeBodyRecord
      ? invoke<KnowledgeDialogueLedger>("get_knowledge_dialogue", { workspaceId: workspace.id })
      : Promise.resolve(null);
    void Promise.all([dialogueRequest, invoke<ModelSettingsSummary>("get_model_settings")])
      .then(([nextLedger, nextSettings]) => {
        if (!active) return;
        setLedger(nextLedger);
        applySettings(nextSettings);
      })
      .catch((reason) => { if (active) setError(normalizeError(reason)); })
      .finally(() => { if (active) setIsLoading(false); });
    return () => { active = false; };
  }, [workspace.id, knowledgeBodyRecord?.recordId]);

  const configuredSlots = settings?.slots.filter((slot) => slot.enabled && slot.hasApiKey && slot.providerLabel && slot.baseUrl && slot.model) ?? [];
  const invalidEnabledDraft = slotDrafts.some((slot) => slot.enabled && (
    !slot.providerLabel.trim() || !slot.baseUrl.trim() || !slot.model.trim()
    || isDeepSeekDocumentationUrl(slot.baseUrl)
    || slot.clearApiKey
    || (!slot.hasApiKey && !slot.apiKey.trim())
  ));
  const ownerItems = ledger?.items.filter((item) => item.inquiry.origin === "owner") ?? [];
  const externalItems = ledger?.items.filter((item) => item.inquiry.origin === "external") ?? [];

  const updateSlot = <K extends keyof ModelSlotDraft>(role: ModelSlotRole, field: K, value: ModelSlotDraft[K]) => {
    setSlotDrafts((current) => current.map((slot) => slot.role === role ? { ...slot, [field]: value } : slot));
  };

  const applyDeepSeekPreset = (role: ModelSlotRole) => {
    setSlotDrafts((current) => current.map((slot) => slot.role === role
      ? { ...slot, enabled: true, ...DEEPSEEK_PRESET }
      : slot));
  };

  const saveSettings = async () => {
    setIsSavingSettings(true);
    setError(null);
    setNotice(null);
    try {
      const summary = await invoke<ModelSettingsSummary>("save_model_settings", {
        slots: slotDrafts.map(({ role, enabled, providerLabel, baseUrl, model, apiKey, clearApiKey }) => ({
          role, enabled, providerLabel, baseUrl, model, apiKey: apiKey.trim() || null, clearApiKey,
        })),
      });
      applySettings(summary);
      setNotice(text("模型设置已保存；API Key 仅保存在系统凭据库。", "Model settings saved; API keys remain only in the system credential store."));
    } catch (reason) {
      setError(normalizeError(reason));
    } finally {
      setIsSavingSettings(false);
    }
  };

  const ask = async () => {
    if (!knowledgeBodyRecord || !question.trim() || configuredSlots.length === 0) return;
    setIsAsking(true);
    setError(null);
    setNotice(null);
    try {
      const nextLedger = await invoke<KnowledgeDialogueLedger>("ask_knowledge_body", {
        workspaceId: workspace.id,
        stance,
        target,
        question: question.trim(),
        authorConfirmedExternalTransmission: true,
      });
      setLedger(nextLedger);
      setQuestion("");
    } catch (reason) {
      setError(normalizeError(reason));
      try {
        setLedger(await invoke<KnowledgeDialogueLedger>("get_knowledge_dialogue", { workspaceId: workspace.id }));
      } catch { /* Keep the last verified ledger if recovery also fails. */ }
    } finally {
      setIsAsking(false);
    }
  };

  return <section className="knowledge-dialogue" aria-labelledby="knowledge-dialogue-title">
    <header className="knowledge-dialogue-header">
      <div><p>{text("知识体自动服务", "Knowledge-body assistance")}</p><h2 id="knowledge-dialogue-title">{text("向这个知识体提问", "Ask this knowledge body")}</h2></div>
      <button className="model-settings-button" type="button" aria-expanded={settingsOpen} onClick={() => setSettingsOpen((open) => !open)}>{text("模型设置", "Model settings")}<span>{text(`${configuredSlots.length}/3 可用`, `${configuredSlots.length}/3 ready`)}</span></button>
    </header>

    {settingsOpen ? <section className="model-settings" aria-labelledby="model-settings-title">
      <header><div><h3 id="model-settings-title">{text("1 个主模型，2 个备选模型", "One primary and two fallback models")}</h3><p>{text("按主模型 → 备选 1 → 备选 2 自动尝试。接口需兼容 OpenAI Chat Completions。", "Automatic order: primary → fallback 1 → fallback 2. Endpoints must support OpenAI Chat Completions.")}</p></div><span>{settings?.secureStore ?? text("系统凭据库", "System credential store")}</span></header>
      <div className="model-slot-grid">{slotDrafts.map((slot) => <fieldset className="model-slot" key={slot.role}>
        <legend>{modelSlotLabel(slot.role, locale)}</legend>
        <div className="model-slot-heading"><span className={`model-slot-status ${slot.enabled && !(!slot.providerLabel.trim() || !slot.baseUrl.trim() || !slot.model.trim() || isDeepSeekDocumentationUrl(slot.baseUrl) || slot.clearApiKey || (!slot.hasApiKey && !slot.apiKey.trim())) ? "is-ready" : ""}`}>{!slot.enabled ? text("未启用", "Disabled") : !slot.providerLabel.trim() || !slot.baseUrl.trim() || !slot.model.trim() ? text("配置不完整", "Incomplete") : isDeepSeekDocumentationUrl(slot.baseUrl) ? text("API 地址错误", "Wrong API URL") : slot.clearApiKey || (!slot.hasApiKey && !slot.apiKey.trim()) ? text("缺少 API Key", "API key required") : slot.apiKey.trim() ? text("保存后可用", "Ready after save") : text("已启用", "Enabled")}</span><button type="button" onClick={() => applyDeepSeekPreset(slot.role)}>{text("使用 DeepSeek 官方配置", "Use DeepSeek preset")}</button></div>
        <label className="model-enabled"><input type="checkbox" checked={slot.enabled} onChange={(event) => updateSlot(slot.role, "enabled", event.target.checked)} />{text("启用此槽位", "Enable this slot")}</label>
        <label>{text("提供方名称", "Provider label")}<input value={slot.providerLabel} onChange={(event) => updateSlot(slot.role, "providerLabel", event.target.value)} placeholder={text("例如：OpenAI", "e.g. OpenAI")} /></label>
        <label>{text("API 地址", "API base URL")}<input aria-label={text("API 地址", "API base URL")} value={slot.baseUrl} aria-invalid={isDeepSeekDocumentationUrl(slot.baseUrl)} onChange={(event) => updateSlot(slot.role, "baseUrl", event.target.value)} placeholder="https://api.example.com/v1" inputMode="url" />{isDeepSeekDocumentationUrl(slot.baseUrl) ? <span className="model-field-error" role="alert">{text("这是 DeepSeek 文档页，不是 API。请改用 https://api.deepseek.com", "This is the DeepSeek documentation site, not its API. Use https://api.deepseek.com")}</span> : null}</label>
        <label>{text("模型名称", "Model name")}<input value={slot.model} onChange={(event) => updateSlot(slot.role, "model", event.target.value)} placeholder="model-id" /></label>
        <label>{text("API Key", "API key")}<input aria-label={text("API Key", "API key")} type="password" autoComplete="new-password" value={slot.apiKey} aria-invalid={slot.enabled && !slot.hasApiKey && !slot.apiKey.trim()} onChange={(event) => { updateSlot(slot.role, "apiKey", event.target.value); updateSlot(slot.role, "clearApiKey", false); }} placeholder={slot.hasApiKey ? text("已安全保存；留空表示保留", "Stored securely; leave blank to retain") : text("输入后保存到系统凭据库", "Saved to the system credential store")} />{slot.enabled && !slot.hasApiKey && !slot.apiKey.trim() ? <span className="model-field-error" role="alert">{text("启用模型需要 API Key；输入后点击“保存模型设置”。", "An API key is required. Enter it, then save the model settings.")}</span> : null}</label>
        <label className="model-clear-key"><input type="checkbox" checked={slot.clearApiKey} onChange={(event) => updateSlot(slot.role, "clearApiKey", event.target.checked)} disabled={!slot.hasApiKey} />{text("删除已保存的 Key", "Delete saved key")}</label>
      </fieldset>)}</div>
      <div className="model-settings-actions"><p>{text("应用界面不会读取或回显明文 Key。仅在作者主动提问时调用模型；DeepSeek V4 问答会关闭思考模式，把输出额度保留给最终回答。", "The interface never reads or reveals plaintext keys. Models are called only after an author submits a question; DeepSeek V4 requests disable thinking mode so the output allowance remains available for the final answer.")}</p><button className="primary-button" type="button" disabled={isSavingSettings || invalidEnabledDraft} onClick={() => void saveSettings()}>{isSavingSettings ? text("保存中…", "Saving…") : invalidEnabledDraft ? text("请先补全启用项", "Complete enabled slots") : text("保存模型设置", "Save model settings")}</button></div>
    </section> : null}

    {error ? <p className="dialogue-message dialogue-error" role="alert">{error}</p> : null}
    {notice ? <p className="dialogue-message" role="status">{notice}</p> : null}

    <div className="dialogue-tabs" role="tablist" aria-label={text("知识体对话类型", "Knowledge dialogue type")}>
      <button type="button" role="tab" aria-selected={activeTab === "owner"} onClick={() => setActiveTab("owner")}>{text("我的问答", "My questions")}<span>{ownerItems.length}</span></button>
      <button type="button" role="tab" aria-selected={activeTab === "external"} onClick={() => setActiveTab("external")}>{text("外部反馈 · 预留", "External feedback · Reserved")}<span>{externalItems.length}</span></button>
    </div>

    {activeTab === "owner" ? <div className="dialogue-owner">
      <KnowledgeDialogueList items={ownerItems} loading={isLoading} />
      <form className="knowledge-composer" onSubmit={(event) => { event.preventDefault(); void ask(); }}>
        <div className="composer-controls"><label>{text("提问类型", "Stance")}<select value={stance} onChange={(event) => setStance(event.target.value as KnowledgeInquiryStance)}><option value="recognition">{text("认可", "Recognition")}</option><option value="question">{text("疑问", "Question")}</option><option value="challenge">{text("挑战", "Challenge")}</option></select></label><label>{text("针对对象", "Target")}<select value={target} onChange={(event) => setTarget(event.target.value as KnowledgeInquiryTarget)}>{INQUIRY_TARGETS.map((item) => <option value={item} key={item}>{inquiryTargetLabel(item, locale)}</option>)}</select></label></div>
        <label className="composer-question"><span>{text("问题或需求", "Question or request")}</span><textarea rows={3} maxLength={4000} value={question} onChange={(event) => setQuestion(event.target.value)} placeholder={text("例如：这个 Claim 目前缺少哪些来源锚点？", "For example: Which source anchors are still missing for this Claim?")} disabled={!knowledgeBodyRecord || isAsking} /></label>
        <div className="composer-submit"><p>{text("本次只发送脱敏后的知识体投影与问题；作者姓名、联系方式、身份标识、源文件和本机路径均不发送，回答不会自动改写知识体。", "Only the redacted knowledge-body projection and question are sent. Author names, contact details, identifiers, source files, and local paths are excluded; answers cannot automatically modify the knowledge body.")}</p><button className="primary-button" type={configuredSlots.length === 0 ? "button" : "submit"} disabled={!knowledgeBodyRecord || isAsking || (configuredSlots.length > 0 && !question.trim())} onClick={configuredSlots.length === 0 && knowledgeBodyRecord ? () => setSettingsOpen(true) : undefined}>{isAsking ? text("模型回答中…", "Model is answering…") : !knowledgeBodyRecord ? text("先固化知识体", "Finalize knowledge body first") : configuredSlots.length === 0 ? text("打开模型设置", "Open model settings") : text("询问知识体", "Ask knowledge body")}</button></div>
        {knowledgeBodyRecord && configuredSlots.length === 0 ? <p className="composer-disabled-note">{text("还没有可用模型：启用槽位、填写正确 API 地址与 Key，并保存设置。", "No model is ready: enable a slot, enter a valid API URL and key, then save.")}</p> : null}
        {!knowledgeBodyRecord ? <p className="composer-disabled-note">{text("先固化当前知识体，问答才会绑定到准确的快照与哈希。", "Finalize the current knowledge body first so dialogue can bind to its exact snapshot and hash.")}</p> : null}
      </form>
    </div> : <ExternalFeedbackReserve items={externalItems} />}
  </section>;
}

function KnowledgeDialogueList({ items, loading }: { items: KnowledgeDialogueItem[]; loading: boolean }) {
  const { locale, text } = useI18n();
  if (loading) return <p className="dialogue-empty">{text("正在读取本地问答记录…", "Loading the local dialogue ledger…")}</p>;
  if (items.length === 0) return <div className="dialogue-empty"><Icon name="knowledge" /><p>{text("还没有问题。可以从 Claim、Scope、Method 或来源锚点开始。", "No questions yet. Start with the Claim, Scope, Method, or source anchors.")}</p></div>;
  return <ol className="dialogue-ledger">{items.map((item) => <li key={item.inquiry.inquiryId}>
    <article className="inquiry-card"><header><span data-stance={item.inquiry.stance}>{inquiryStanceLabel(item.inquiry.stance, locale)}</span><strong>{inquiryTargetLabel(item.inquiry.target, locale)}</strong><time>{formatModifiedDate(item.inquiry.createdUnixMs, locale)}</time></header><p>{item.inquiry.question}</p><small>S{item.inquiry.snapshotVersion} · {item.inquiry.knowledgeBodyHash.slice(0, 12)}</small></article>
    {item.answers.length > 0 ? item.answers.map((answer) => <article className="answer-card" key={answer.answerId}><header><strong>{answer.providerLabel} · {answer.model}</strong><span>{modelSlotLabel(answer.modelSlot as ModelSlotRole, locale)}</span></header><p>{answer.answer}</p><footer><span>{text(`${answer.sourceAnchors.length} 个来源锚点`, `${answer.sourceAnchors.length} source anchor(s)`)}</span><code>{answer.recordHash.slice(0, 12)}</code></footer></article>) : <p className="answer-pending">{text("问题已保存在本机；模型尚未形成回答。", "The question is saved locally; no model answer is available yet.")}</p>}
  </li>)}</ol>;
}

function ExternalFeedbackReserve({ items }: { items: KnowledgeDialogueItem[] }) {
  const { text } = useI18n();
  return <div className="external-feedback">
    {items.length > 0 ? <KnowledgeDialogueList items={items} loading={false} /> : null}
    <div className="external-reserve-card"><span>{text("尚未开放", "Not yet available")}</span><h3>{text("为外部读者保留的论文提问窗口", "Reserved inquiry window for external readers")}</h3><p>{text("未来可让经过授权的客户针对公开知识体表达认可、疑问或挑战；身份、权限、原问题和 AI 回答都进入可追溯记录。", "Authorized readers will be able to express recognition, questions, or challenges toward a published knowledge body; identity, permission, original inquiry, and AI answer will remain traceable.")}</p><ul><li>{text("认可", "Recognition")}</li><li>{text("疑问", "Question")}</li><li>{text("挑战", "Challenge")}</li></ul><small>{text("当前版本不接收外部网络请求，也不会虚构外部反馈。", "This version accepts no external network requests and never fabricates reader feedback.")}</small></div>
  </div>;
}

function KnowledgeNetworkCanvas({ bodies, assertions, view }: { bodies: KnowledgeBodyNode[]; assertions: NetworkAssertion[]; view: Exclude<KnowledgeView, "single"> }) {
  const { locale, text } = useI18n();
  const positions = view === "pair"
    ? [{ x: 160, y: 210 }, { x: 440, y: 210 }]
    : [{ x: 100, y: 105 }, { x: 300, y: 88 }, { x: 500, y: 105 }, { x: 185, y: 320 }, { x: 415, y: 320 }];
  const visibleBodies = bodies.slice(0, positions.length);
  const findBodyIndex = (reference: VersionedObjectReference) => visibleBodies.findIndex((body) => [body.body.objectId, body.claim.objectId, body.sourceAnchor.objectId, body.method.objectId].includes(reference.objectId));
  const visibleAssertions = assertions.flatMap((assertion) => {
    const sourceIndex = findBodyIndex(assertion.source);
    const targetIndex = findBodyIndex(assertion.target);
    return sourceIndex >= 0 && targetIndex >= 0 && sourceIndex !== targetIndex ? [{ assertion, sourceIndex, targetIndex }] : [];
  }).slice(0, 10);
  const roleLabel = (role: KnowledgeBodyRole) => {
    const labels: Record<KnowledgeBodyRole, [string, string]> = {
      current_study: ["当前研究", "Current study"], original_research: ["原研究", "Original research"], reproduction_research: ["复现研究", "Reproduction"], competing_research: ["竞争研究", "Competing research"], cross_domain_application: ["跨域应用", "Cross-domain application"], later_synthesis: ["后续综合", "Later synthesis"],
    };
    return localize(locale, labels[role][0], labels[role][1]);
  };
  return <div className="knowledge-network-canvas" role="img" aria-label={text(`${visibleBodies.length} 个保持边界的知识体，通过 ${visibleAssertions.length} 个一等声明对象形成关联网络。`, `${visibleBodies.length} bounded knowledge bodies form a network through ${visibleAssertions.length} first-class assertion objects.`)}>
    <svg viewBox="0 0 600 420" preserveAspectRatio="xMidYMid meet" aria-hidden="true">
      <defs><marker id="knowledge-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" /></marker></defs>
      {visibleAssertions.map(({ assertion, sourceIndex, targetIndex }) => {
        const source = positions[sourceIndex]; const target = positions[targetIndex]; const midX = (source.x + target.x) / 2; const midY = (source.y + target.y) / 2;
        return <g className="network-assertion" key={`${assertion.assertionId}-${assertion.version}`}><line x1={source.x} y1={source.y} x2={target.x} y2={target.y} markerEnd="url(#knowledge-arrow)" /><rect x={midX - 8} y={midY - 8} width="16" height="16" transform={`rotate(45 ${midX} ${midY})`} /><text x={midX} y={midY - 14}>{relationKindLabel(assertion.relationKind, locale)}</text><text className="network-protocol" x={midX} y={midY + 24}>{assertion.protocolObject} · v{assertion.version}</text></g>;
      })}
      {visibleBodies.map((body, index) => { const position = positions[index]; const radius = view === "pair" ? 118 : 72; return <g className="network-body" key={body.body.objectId}><circle className="body-boundary" cx={position.x} cy={position.y} r={radius} /><text className="body-title" x={position.x} y={position.y - radius + 18}>{body.displayId} · {roleLabel(body.role)} · S{body.body.version}</text><line x1={position.x} y1={position.y} x2={position.x - 38} y2={position.y + 30} /><line x1={position.x} y1={position.y} x2={position.x + 38} y2={position.y + 30} /><circle className="body-claim" cx={position.x} cy={position.y} r="19" /><text x={position.x} y={position.y - 26}>Claim</text><text x={position.x} y={position.y + 4}>v{body.claim.version}</text><circle className="body-anchor" cx={position.x - 38} cy={position.y + 30} r="13" /><text x={position.x - 38} y={position.y + 52}>Anchor v{body.sourceAnchor.version}</text><circle className="body-method" cx={position.x + 38} cy={position.y + 30} r="13" /><text x={position.x + 38} y={position.y + 52}>Method v{body.method.version}</text></g>; })}
    </svg>
    {visibleAssertions.length === 0 ? <p>{text("当前只有单体边界，尚无经过声明协议确认的跨体关系。", "Only the local body boundary exists; no cross-body relationship has been confirmed through an assertion protocol.")}</p> : null}
  </div>;
}

function ClaimDodecahedron() {
  const svgRef = useRef<SVGSVGElement | null>(null);

  useEffect(() => {
    const svg = svgRef.current;
    if (!svg) return;
    const edges = Array.from(svg.querySelectorAll<SVGLineElement>(".dodeca-edge"));
    const motionPreference = window.matchMedia?.("(prefers-reduced-motion: reduce)");
    let frameId: number | null = null;

    const draw = (angle: number) => {
      const tilt = -0.38 + Math.sin(angle * 0.5) * 0.08;
      const cosY = Math.cos(angle);
      const sinY = Math.sin(angle);
      const cosX = Math.cos(tilt);
      const sinX = Math.sin(tilt);
      const points = DODECAHEDRON_VERTICES.map(([x, y, z]) => {
        const rotatedX = x * cosY + z * sinY;
        const rotatedZ = -x * sinY + z * cosY;
        const rotatedY = y * cosX - rotatedZ * sinX;
        const depth = y * sinX + rotatedZ * cosX;
        const perspective = 4.5 / (4.5 - depth);
        return { x: 60 + rotatedX * 27 * perspective, y: 60 + rotatedY * 27 * perspective, depth };
      });
      DODECAHEDRON_EDGES.forEach(([from, to], index) => {
        const line = edges[index];
        const start = points[from];
        const end = points[to];
        line.setAttribute("x1", start.x.toFixed(2));
        line.setAttribute("y1", start.y.toFixed(2));
        line.setAttribute("x2", end.x.toFixed(2));
        line.setAttribute("y2", end.y.toFixed(2));
        line.style.strokeOpacity = String(Math.max(0.24, Math.min(0.9, 0.52 + (start.depth + end.depth) * 0.1)));
      });
    };

    const start = () => {
      if (frameId !== null) window.cancelAnimationFrame(frameId);
      if (motionPreference?.matches || typeof window.requestAnimationFrame !== "function") {
        draw(0.62);
        frameId = null;
        return;
      }
      const startedAt = window.performance.now();
      draw(0);
      let lastDrawAt = startedAt;
      const animate = (time: number) => {
        if (time - lastDrawAt >= 32) {
          draw(((time - startedAt) / 48000) * Math.PI * 2);
          lastDrawAt = time;
        }
        frameId = window.requestAnimationFrame(animate);
      };
      frameId = window.requestAnimationFrame(animate);
    };

    start();
    motionPreference?.addEventListener?.("change", start);
    return () => {
      if (frameId !== null) window.cancelAnimationFrame(frameId);
      motionPreference?.removeEventListener?.("change", start);
    };
  }, []);

  return <svg ref={svgRef} className="claim-dodecahedron" viewBox="0 0 120 120" focusable="false">{DODECAHEDRON_EDGES.map(([from, to]) => <line className="dodeca-edge" key={`${from}-${to}`} />)}</svg>;
}
