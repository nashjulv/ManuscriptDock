import { OfficialSourceAccess, type OfficialFetchOptions, type OfficialFetchResult, type DiscoverOfficialSource } from "./OfficialSourceAccess";
import { invoke, isTauri } from "@tauri-apps/api/core";
import type { CSSProperties, KeyboardEvent as ReactKeyboardEvent, ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import manuscriptDockLogo from "./assets/manuscriptdock-logo.svg";
import { PRODUCT_VERSION } from "./version";
import { I18nProvider, localize, localizeBackendText, localizeSourceLabel, useI18n, type Locale } from "./i18n";

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

interface WorkspaceStorageSummary {
  defaultLocation: string;
  storageMode: "application_managed_local_library";
  sourcePolicy: "immutable_versioned_copy";
}

interface WorkspaceCopyExport {
  folderName: string;
  workspaceId: string;
  manuscriptVersion: number;
  fileCount: number;
  exportedUnixMs: number;
  externalTransmission: "not_performed";
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
type JournalMetricScheme = "cas_partition" | "clarivate_jcr" | "emerging_partition";
interface JournalDirectoryEvidence { scheme: JournalMetricScheme; releaseYear: number; metricYear: number | null; issn: string | null; eissn: string | null; partition: number | null; top: boolean | null; openAccess: boolean | null; jifTenths: number | null; category: string | null; }
interface JournalDirectorySummary { schemaVersion: number; available: boolean; sourceCount: number; recordCount: number; distinctJournalCount: number; issnCount: number; eissnCount: number; publisherCount: number; scopeCount: number; annualVolumeCount: number; reviewProcessCount: number; reviewSpeedCount: number; publicationCycleCount: number; circulationCount: number; oaStatusCount: number; latestReleaseYear: number | null; recordsByScheme: Record<string, number>; partitionCounts: Record<string, number>; topCount: number; openAccessCount: number; formulaCellCount: number; catalogFingerprint: string | null; updatedUnixMs: number; sourceFiles: string[]; warnings: string[]; }
interface JournalDirectoryImportResult { importedSourceCount: number; importedRecordCount: number; unchangedSourceCount: number; summary: JournalDirectorySummary; }
interface JournalProfileDiscoveryRecord { schemaVersion: number; discoveryId: string; workspaceId: string; targetSelectionId: string; journalId: string; journalName: string; issn: string | null; eissn: string | null; publisher: string | null; scopeSummary: string | null; reportedPrintCirculation: number | null; averageReviewDays: number | null; submissionToPublicationDays: number | null; publicationFrequency: string | null; apcStatus: string | null; openAccessStatus: string | null; officialHomepageUrl: string | null; aimsScopeUrl: string | null; authorInstructionsUrl: string | null; sourceUrls: string[]; missingFields: string[]; evidenceStatus: "local_profile_available" | "candidate_requires_official_verification"; sourceMode: "local_directory" | "configured_model_candidate"; providerLabel: string | null; model: string | null; externalTransmission: string; createdUnixMs: number; }
interface JournalRecommendation { id: string; name: string; nameEn: string; region: "domestic" | "international"; publisher: string; publisherEn?: string; rankSystem: string; rankTier: string; deadlineStatus: string; institutionEligibility: string; rankingSourceUrl: string; homepageUrl: string; openAccessStatus: string; directoryEvidence: JournalDirectoryEvidence[]; }
interface JournalRecommendationPortfolio { sprint: JournalRecommendation[]; matching: JournalRecommendation[]; safeguard: JournalRecommendation[]; }
interface JournalRecommendationRun { schemaVersion: number; runId: string; workspaceId: string; manuscriptVersion: number; catalogVersion: string; catalogVerifiedDate: string; resolvedArticleType?: ArticleTypePreference; evaluatedUnixMs: number; recommendationProfile: JournalRecommendationProfileSummary; deadlineDaysRemaining: number; domestic: JournalRecommendationPortfolio; international: JournalRecommendationPortfolio; schoolRuleStatus: string; institutionDirectoryStatus: string; journalDirectoryVersion: string | null; limitations: string[]; externalTransmission: string; }

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
type KnowledgeCandidateDecision = "included" | "excluded";
interface ExtractedKnowledgeElement { object: VersionedObjectReference; state: ElementState; candidates: KnowledgeCandidateContent[]; }
interface KnowledgeExtractionLayer { decompositionId: string; decompositionHash: string; analysisVersion: number; sourceSnapshotVersion: number; generatedBy: string; confirmationPolicy: string; claim: ExtractedKnowledgeElement; scope: ExtractedKnowledgeElement; method: ExtractedKnowledgeElement; result: ExtractedKnowledgeElement; evidence: ExtractedKnowledgeElement; }
type PublicationContactKind = "email" | "orcid" | "correspondence";
interface PublicationContact { kind: PublicationContactKind; value: string; sourceLabel: string; sourceFragmentId: string | null; }
interface SourceIdentityVersion { version: number; title: string | null; authors: string[]; affiliations: string[]; contacts: PublicationContact[]; sourceArtifact: VersionedObjectReference; status: "awaiting_extraction" | "not_detected" | "extracted"; disclosureBasis: "source_document_declared_metadata"; localVisibility: "visible_in_local_workspace"; externalModelPolicy: "excluded_from_default_model_projection"; }
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
interface AcademicKnowledgeBodySnapshot { schemaVersion: number; knowledgeBodyId: string; snapshotVersion: number; manuscript: VersionedObjectReference; claim: ClaimFiveTuple; objects: KnowledgeBodyObjectSet; aiReviewReport: VersionedObjectReference | null; aiReviewHistory: AiReviewReportHistory; sourceIdentity?: SourceIdentityVersion; extraction?: KnowledgeExtractionLayer; serviceArchitecture?: KnowledgeBodyServiceArchitecture; network: { bodies: KnowledgeBodyNode[]; assertions: NetworkAssertion[]; supportedRelations: RelationKind[] }; externalTransmission: string; }

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
  schemaVersion?: number;
  submissionId: string;
  workspaceId: string;
  manuscriptVersion: number;
  attestationId: string;
  targetSelectionId?: string | null;
  target: string;
  publisher?: string | null;
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

type SubmissionMaterialKind = "source_project" | "blinded_manuscript" | "figure" | "table" | "bibliography" | "supplementary" | "cover_letter" | "title_page" | "declaration" | "other";
interface SubmissionMaterial { materialId: string; kind: SubmissionMaterialKind; originalName: string; extension: string; sizeBytes: number; contentHash: string; importedUnixMs: number; manuscriptVersion: number; targetSelectionId: string | null; requirementSnapshotId: string | null; checklistItemId: string | null; included: boolean; validationStatus: "passed" | "warning" | "blocked" | ""; validationIssues: string[]; detectedMediaType: string | null; }
type SubmissionChecklistStatus = "passed" | "missing" | "recommended" | "author_confirmation" | "manual_verification";
interface SubmissionMaterialChecklistItem { id: string; label: string; labelEn: string; group: "target" | "manuscript" | "files" | "declarations"; requirement: "required" | "recommended"; status: SubmissionChecklistStatus; detail: string; verification: "automatic" | "file" | "author" | "manual"; materialKind: SubmissionMaterialKind | null; blocking: boolean; confirmable: boolean; sourceUrl: string | null; evidenceExcerpt: string | null; capturedUnixMs: number | null; freshUntilUnixMs: number | null; requiredCount: number; matchedMaterialIds: string[]; }
type SubmissionWorkflowStatus = "manuscript_received" | "preliminary_recommendation" | "target_verified" | "materials_required" | "materials_complete_check_required" | "submission_ready";
interface SubmissionMaterialCatalog { schemaVersion: number; workspaceId: string; manuscriptVersion: number; materials: SubmissionMaterial[]; checklist: SubmissionMaterialChecklistItem[]; recommendationReady: boolean; targetVerified: boolean; requiredComplete: boolean; targetCheckReady: boolean; workflowStatus: SubmissionWorkflowStatus; requiredTotal: number; requiredCompleted: number; detectedFigureCount?: number; detectedTableCount?: number; }
interface SubmissionTargetSelection { schemaVersion: number; selectionId: string; workspaceId: string; selectedAgainstManuscriptVersion: number; recommendationRunId: string; journalId: string; name: string; nameEn: string; publisher: string; region: "domestic" | "international"; rankSystem: string; rankTier: string; homepageUrl: string; articleType?: ArticleTypePreference; planRole: "primary" | "backup"; priority: number; selectedUnixMs: number; recordHash: string; externalTransmission: "not_performed"; }
interface SubmissionTargetPlan { schemaVersion: number; workspaceId: string; primary: SubmissionTargetSelection | null; backups: SubmissionTargetSelection[]; updatedUnixMs: number; }
type JournalRequirementStatus = "official_sources_captured" | "author_attested_official" | "requires_manual_review";
type JournalRequirementObligation = "required" | "recommended" | "verify";
interface JournalRequirementSource { url: string; title: string; contentHash: string; capturedUnixMs: number; officialHostMatched: boolean; }
interface JournalRequirementItem { id: string; category: string; label: string; labelEn: string; obligation: JournalRequirementObligation; detail: string; sourceUrl: string; evidenceExcerpt: string; }
interface JournalRequirementSnapshot { schemaVersion: number; snapshotId: string; workspaceId: string; targetSelectionId: string; journalId: string; journalName: string; sourceMode: "official_network_fetch" | "author_provided_official_text"; status: JournalRequirementStatus; sources: JournalRequirementSource[]; requirements: JournalRequirementItem[]; limitations: string[]; capturedUnixMs: number; freshUntilUnixMs: number; recordHash: string; externalTransmission: string; }
interface TargetSubmissionExport { packageName: string; manuscriptVersion: number; targetSelectionId: string; targetName: string; files: string[]; warnings: string[]; exportedUnixMs: number; externalTransmission: "not_performed"; }
interface TargetSubmissionPackageFile { materialId: string | null; displayName: string; relativePath: string; role: string; materialKind: SubmissionMaterialKind | null; checklistItemId: string | null; checklistLabel: string | null; required: boolean; included: boolean; sizeBytes: number; contentHash: string; validationStatus: string; validationIssues: string[]; }
interface TargetSubmissionPackagePlan { schemaVersion: number; workspaceId: string; manuscriptVersion: number; targetSelectionId: string; targetName: string; anonymousReview: boolean; ready: boolean; files: TargetSubmissionPackageFile[]; warnings: string[]; blockers: string[]; createdUnixMs: number; externalTransmission: "not_performed"; }

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
  submissionMaterials: SubmissionMaterialCatalog;
  submissionTarget: SubmissionTargetSelection | null;
  submissionTargetPlan: SubmissionTargetPlan;
  journalRequirements: JournalRequirementSnapshot | null;
}

type SelectionState = "idle" | "selecting" | "selected" | "error";
type WorkspaceStage = "source" | "materials" | "check" | "revision" | "versions" | "journals" | "attestation" | "submission" | "knowledge";
type WorkspaceSection = "overview" | "materials" | "journals" | "prepare" | "submission";
type IconName = "workspace" | "upload" | "lock" | "file" | "check" | "close" | "versions" | "structure" | "target" | "format" | "review" | "package" | "knowledge" | "arrow" | "warning" | "more" | "archive" | "trash" | "restore" | "folder" | "settings";
interface LocalizedCopy { zhCN: string; en: string; }
type LocalizedNotice = LocalizedCopy | string;

const WORKSPACE_STAGES: Array<{ id: WorkspaceStage; zh: string; en: string; shortZh: string; shortEn: string }> = [
  { id: "source", zh: "概览", en: "Overview", shortZh: "概览", shortEn: "Overview" },
  { id: "materials", zh: "投稿资料", en: "Materials", shortZh: "资料", shortEn: "Materials" },
  { id: "check", zh: "检查", en: "Check", shortZh: "检查", shortEn: "Check" },
  { id: "revision", zh: "修订", en: "Revise", shortZh: "修订", shortEn: "Revise" },
  { id: "versions", zh: "版本", en: "Version", shortZh: "版本", shortEn: "Version" },
  { id: "journals", zh: "目标期刊", en: "Target Journal", shortZh: "期刊", shortEn: "Target" },
  { id: "attestation", zh: "存证", en: "Attest", shortZh: "存证", shortEn: "Attest" },
  { id: "submission", zh: "投稿包", en: "Package", shortZh: "投稿包", shortEn: "Package" },
  { id: "knowledge", zh: "个人知识体", en: "Personal Knowledge Body", shortZh: "知识", shortEn: "Knowledge" },
];

const WORKSPACE_SECTIONS: Array<{ id: WorkspaceSection; stage: WorkspaceStage; zh: string; en: string; icon: IconName }> = [
  { id: "overview", stage: "source", zh: "概览", en: "Overview", icon: "workspace" },
  { id: "journals", stage: "journals", zh: "目标期刊", en: "Target journal", icon: "target" },
  { id: "materials", stage: "materials", zh: "投稿资料", en: "Materials", icon: "folder" },
  { id: "prepare", stage: "check", zh: "检查与修订", en: "Check/revise", icon: "review" },
  { id: "submission", stage: "submission", zh: "投稿包", en: "Package", icon: "package" },
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
    close: <><circle cx="12" cy="12" r="9" /><path d="m9 9 6 6M15 9l-6 6" /></>,
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
    folder: <><path d="M3 6a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" /><path d="M3 9h18" /></>,
    settings: <><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.83 2.83-.06-.06A1.7 1.7 0 0 0 15 19.4a1.7 1.7 0 0 0-1 .6 1.7 1.7 0 0 0-.4 1.1V21H9.6v-.1A1.7 1.7 0 0 0 8.5 19.3a1.7 1.7 0 0 0-1.88.34l-.06.06-2.83-2.83.06-.06A1.7 1.7 0 0 0 4.1 15a1.7 1.7 0 0 0-.6-1 1.7 1.7 0 0 0-1.1-.4H2.3V9.6h.1A1.7 1.7 0 0 0 4 8.5a1.7 1.7 0 0 0-.34-1.88l-.06-.06 2.83-2.83.06.06A1.7 1.7 0 0 0 8.4 4.1a1.7 1.7 0 0 0 1-.6 1.7 1.7 0 0 0 .4-1.1V2.4h4v.1A1.7 1.7 0 0 0 15 4.1a1.7 1.7 0 0 0 1.88-.34l.06-.06 2.83 2.83-.06.06A1.7 1.7 0 0 0 19.4 8.5a1.7 1.7 0 0 0 .6 1 1.7 1.7 0 0 0 1.1.4h.1v4h-.1A1.7 1.7 0 0 0 19.4 15Z" /></>,
  };
  return <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">{paths[name]}</svg>;
}

function formatBytes(bytes: number, locale: Locale) {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: value >= 10 ? 0 : 1 }).format(value)} ${units[unitIndex]}`;
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

function localizedCopy(zhCN: string, en: string): LocalizedCopy {
  return { zhCN, en };
}

function resolveNotice(locale: Locale, notice: LocalizedNotice | null) {
  if (!notice) return null;
  return typeof notice === "string"
    ? localizeBackendText(locale, notice)
    : localize(locale, notice.zhCN, notice.en);
}

function getStageIcon(stage: WorkspaceStage): IconName {
  if (stage === "source") return "file";
  if (stage === "materials") return "folder";
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
  const [workspaceManagementNotice, setWorkspaceManagementNotice] = useState<LocalizedCopy | null>(null);
  const [workspaceManagementError, setWorkspaceManagementError] = useState<string | null>(null);
  const [workspaceStorage, setWorkspaceStorage] = useState<WorkspaceStorageSummary | null>(null);
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
  const [versionNotice, setVersionNotice] = useState<LocalizedNotice | null>(null);
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
  const [submissionMaterials, setSubmissionMaterials] = useState<SubmissionMaterialCatalog | null>(null);
  const [submissionTargetSelection, setSubmissionTargetSelection] = useState<SubmissionTargetSelection | null>(null);
  const [submissionTargetPlan, setSubmissionTargetPlan] = useState<SubmissionTargetPlan | null>(null);
  const [journalRequirementSnapshots, setJournalRequirementSnapshots] = useState<JournalRequirementSnapshot[]>([]);
  const [targetSubmissionExport, setTargetSubmissionExport] = useState<TargetSubmissionExport | null>(null);
  const [targetSubmissionPackagePlan, setTargetSubmissionPackagePlan] = useState<TargetSubmissionPackagePlan | null>(null);
  const [attestationConfirmed, setAttestationConfirmed] = useState(false);
  const [submissionConfirmed, setSubmissionConfirmed] = useState(false);
  const [submissionReceipt, setSubmissionReceipt] = useState("");
  const [isLoadingLifecycle, setIsLoadingLifecycle] = useState(false);
  const [isAttesting, setIsAttesting] = useState(false);
  const [isExportingSubmission, setIsExportingSubmission] = useState(false);
  const [isAddingMaterials, setIsAddingMaterials] = useState(false);
  const [confirmingRequirementId, setConfirmingRequirementId] = useState<string | null>(null);
  const [isSelectingTarget, setIsSelectingTarget] = useState(false);
  const [targetPlanBusyId, setTargetPlanBusyId] = useState<string | null>(null);
  const [requirementBusyId, setRequirementBusyId] = useState<string | null>(null);
  const [isRecordingSubmission, setIsRecordingSubmission] = useState(false);
  const [isFinalizingKnowledge, setIsFinalizingKnowledge] = useState(false);
  const [isLoadingKnowledgeBody, setIsLoadingKnowledgeBody] = useState(false);
  const [knowledgeCandidateDecisions, setKnowledgeCandidateDecisions] = useState<Record<string, KnowledgeCandidateDecision>>({});
  const [knowledgeReviewConfirmed, setKnowledgeReviewConfirmed] = useState(false);
  const [activeStage, setActiveStage] = useState<WorkspaceStage>("source");
  const [evidenceOpen, setEvidenceOpen] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;
    void invoke<WorkspaceCatalog>("list_workspaces")
      .then((catalog) => { setRecentWorkspaces(catalog.workspaces); setArchivedWorkspaces(catalog.archivedWorkspaces ?? []); setCatalogWarnings(catalog.warnings); })
      .catch(() => { setCatalogWarnings([text("最近的本地工作区暂时无法读取", "Recent local workspaces could not be loaded")]); });
    void invoke<WorkspaceStorageSummary>("get_workspace_storage_summary")
      .then(setWorkspaceStorage)
      .catch(() => setWorkspaceStorage(null));
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
    setKnowledgeCandidateDecisions({});
    setKnowledgeReviewConfirmed(false);
  }

  function resetDownstreamLifecycle(clearSubmissionAssets = false) {
    setAttestation(null);
    setSubmission(null);
    setSubmissionExport(null);
    setAttestationConfirmed(false);
    setSubmissionConfirmed(false);
    setSubmissionReceipt("");
    setTargetSubmissionExport(null);
    setTargetSubmissionPackagePlan(null);
    if (clearSubmissionAssets) {
      setSubmissionMaterials(null);
      setSubmissionTargetSelection(null);
      setSubmissionTargetPlan(null);
      setJournalRequirementSnapshots([]);
    }
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
        setSubmissionMaterials(lifecycle.submissionMaterials ?? null);
        setSubmissionTargetSelection(lifecycle.submissionTarget ?? null);
        setSubmissionTargetPlan(lifecycle.submissionTargetPlan ?? { schemaVersion: 1, workspaceId: workspace.id, primary: lifecycle.submissionTarget ?? null, backups: [], updatedUnixMs: 0 });
        setJournalRequirementSnapshots(lifecycle.journalRequirements ? [lifecycle.journalRequirements] : []);
        setSubmissionReceipt(lifecycle.submission?.receipt ?? "");
        setKnowledgeBodyRecord(lifecycle.knowledgeBody);
        setKnowledgeBodySnapshot(lifecycle.knowledgeBody?.snapshot ?? null);
        setSelectedDisciplineCode(lifecycle.knowledgeBody?.disciplineClassification?.code ?? "");
        if (lifecycle.readinessReport) {
          setSelectedRulePackIds(lifecycle.readinessReport.rulePacks
            .map((pack) => pack.id)
            .filter((id) => id !== "core-structure-v1" && id !== "initial-submission-v1"));
        }
        void invoke<JournalRequirementSnapshot[]>("get_journal_requirement_snapshots", { workspaceId: workspace.id })
          .then(setJournalRequirementSnapshots)
          .catch(() => undefined);
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
          setVersionNotice(result.message);
          return;
        }
        setActiveWorkspace(result.workspace);
        setRecentWorkspaces((current) => [result.workspace, ...current.filter((workspace) => workspace.id !== result.workspace.id)]);
        setStructureReport(null);
        setReadinessReport(null);
        resetDownstreamLifecycle();
        setVersionNotice(localizedCopy(`已保存版本 v${result.version.version}`, `Version v${result.version.version} saved`));
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
          setVersionNotice(result.message);
          return;
        }
        setActiveWorkspace(result.workspace);
        setRecentWorkspaces((current) => [result.workspace, ...current.filter((workspace) => workspace.id !== result.workspace.id)]);
        setStructureReport(null);
        setReadinessReport(null);
        resetDownstreamLifecycle();
        setVersionNotice(localizedCopy(`已将 v${version} 恢复为新的 v${result.version.version}`, `Restored v${version} as new v${result.version.version}`));
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
          resetDownstreamLifecycle(true);
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
          resetDownstreamLifecycle(true);
          setActiveStage("source");
          setEvidenceOpen(false);
          setRecentWorkspaces((current) => [result.workspace, ...current.filter((workspace) => workspace.id !== result.workspace.id)]);
          setSelectionId(null);
          refreshSubmissionMaterials(result.workspace.id);
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
          refreshSubmissionMaterials(activeWorkspace.id);
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
          refreshSubmissionMaterials(activeWorkspace.id);
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
    resetDownstreamLifecycle(true);
    setActiveStage("source");
    setEvidenceOpen(false);
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
        ? localizedCopy(`已归档《${workspace.manuscript.name}》`, `Archived “${workspace.manuscript.name}”`)
        : action === "restore"
          ? localizedCopy(`已恢复《${workspace.manuscript.name}》`, `Restored “${workspace.manuscript.name}”`)
          : localizedCopy(`已永久删除《${workspace.manuscript.name}》`, `Permanently deleted “${workspace.manuscript.name}”`));
      return true;
    } catch (error) {
      setWorkspaceManagementError(normalizeError(error));
      return false;
    } finally {
      setWorkspaceManagementBusyId(null);
    }
  }

  async function saveWorkspaceCopy(workspace: WorkspaceSummary, archived: boolean) {
    if (workspaceManagementBusyId) return false;
    setWorkspaceManagementBusyId(workspace.id);
    setWorkspaceManagementNotice(null);
    setWorkspaceManagementError(null);
    try {
      const exported = await invoke<WorkspaceCopyExport | null>("export_workspace_copy", { workspaceId: workspace.id, archived });
      if (!exported) return false;
      setWorkspaceManagementNotice(localizedCopy(`已另存完整工作区：${exported.folderName}（${exported.fileCount} 个文件）`, `Saved workspace copy: ${exported.folderName} (${exported.fileCount} files)`));
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
    setEvidenceOpen(false);
    setErrorMessage(null);
    setSelectionState("idle");
  }

  function openStage(stage: WorkspaceStage) {
    setActiveStage(stage);
    setEvidenceOpen(false);
    if (stage === "submission" && activeWorkspace) {
      refreshTargetSubmissionPackagePlan(activeWorkspace.id);
    }
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
        .then((snapshot) => {
          const sameDecomposition = knowledgeBodySnapshot?.extraction?.decompositionHash === snapshot.extraction?.decompositionHash;
          setKnowledgeBodySnapshot(snapshot);
          setKnowledgeCandidateDecisions((current) => Object.fromEntries(
            knowledgeCandidates(snapshot).flatMap((candidate) => {
              const decision = candidate.authorConfirmed ? "included" : sameDecomposition ? current[candidate.candidateId] : undefined;
              return decision ? [[candidate.candidateId, decision]] : [];
            }),
          ));
          if (!sameDecomposition) setKnowledgeReviewConfirmed(false);
        })
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
        setVersionNotice(result.message);
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
      setVersionNotice(localizedCopy(`已保存 v${result.version.version}，并完成当前版本复查`, `Saved v${result.version.version} and rechecked the current version`));
      loadVersionHistory(result.workspace, result.revisionSet.baseVersion);
      refreshSubmissionMaterials(result.workspace.id);
      setActiveStage("journals");
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

  function refreshSubmissionMaterials(workspaceId = activeWorkspace?.id) {
    if (!workspaceId) return;
    void invoke<SubmissionMaterialCatalog>("get_submission_materials", { workspaceId })
      .then(setSubmissionMaterials)
      .catch((error: unknown) => setErrorMessage(normalizeError(error)));
  }

  function refreshTargetSubmissionPackagePlan(workspaceId = activeWorkspace?.id) {
    if (!workspaceId) return;
    void invoke<TargetSubmissionPackagePlan>("get_target_submission_package_plan", { workspaceId })
      .then(setTargetSubmissionPackagePlan)
      .catch(() => setTargetSubmissionPackagePlan(null));
  }

  async function addSubmissionMaterials(kind: SubmissionMaterialKind, checklistItemId?: string) {
    if (!activeWorkspace || isAddingMaterials) return;
    setIsAddingMaterials(true);
    setErrorMessage(null);
    try {
      const catalog = await invoke<SubmissionMaterialCatalog | null>("add_submission_materials", { workspaceId: activeWorkspace.id, kind, checklistItemId: checklistItemId ?? null, locale });
      if (catalog) {
        setSubmissionMaterials(catalog);
        refreshTargetSubmissionPackagePlan(activeWorkspace.id);
      }
    } catch (error) {
      setErrorMessage(normalizeError(error));
    } finally {
      setIsAddingMaterials(false);
    }
  }

  async function setSubmissionMaterialIncluded(materialId: string, included: boolean) {
    if (!activeWorkspace || isAddingMaterials) return;
    setIsAddingMaterials(true);
    setErrorMessage(null);
    try {
      const catalog = await invoke<SubmissionMaterialCatalog>("set_submission_material_included", { workspaceId: activeWorkspace.id, materialId, included });
      setSubmissionMaterials(catalog);
      refreshTargetSubmissionPackagePlan(activeWorkspace.id);
    } catch (error) {
      setErrorMessage(normalizeError(error));
    } finally {
      setIsAddingMaterials(false);
    }
  }

  async function deleteSubmissionMaterial(materialId: string) {
    if (!activeWorkspace || isAddingMaterials) return;
    setIsAddingMaterials(true);
    setErrorMessage(null);
    try {
      const catalog = await invoke<SubmissionMaterialCatalog>("delete_submission_material", { workspaceId: activeWorkspace.id, materialId, authorConfirmed: true });
      setSubmissionMaterials(catalog);
      refreshTargetSubmissionPackagePlan(activeWorkspace.id);
    } catch (error) {
      setErrorMessage(normalizeError(error));
    } finally {
      setIsAddingMaterials(false);
    }
  }

  async function confirmSubmissionRequirement(itemId: string, confirmed: boolean) {
    if (!activeWorkspace || confirmingRequirementId) return;
    setConfirmingRequirementId(itemId);
    setErrorMessage(null);
    try {
      const catalog = await invoke<SubmissionMaterialCatalog>("confirm_submission_requirement", { workspaceId: activeWorkspace.id, itemId, confirmed });
      setSubmissionMaterials(catalog);
    } catch (error) {
      setErrorMessage(normalizeError(error));
    } finally {
      setConfirmingRequirementId(null);
    }
  }

  function syncRecommendationAnalysis() {
    if (!activeWorkspace) return;
    void invoke<WorkspaceLifecycle>("get_workspace_lifecycle", { workspaceId: activeWorkspace.id })
      .then((lifecycle) => {
        setStructureReport(lifecycle.structureReport);
        setSubmissionMaterials(lifecycle.submissionMaterials);
      })
      .catch((error: unknown) => setErrorMessage(normalizeError(error)));
  }

  async function selectSubmissionTarget(recommendationRunId: string, journalId: string) {
    if (!activeWorkspace || isSelectingTarget) return;
    setIsSelectingTarget(true);
    setErrorMessage(null);
    try {
      const target = await invoke<SubmissionTargetSelection>("select_recommended_journal", { workspaceId: activeWorkspace.id, recommendationRunId, journalId });
      setReadinessReport(null);
      resetDownstreamLifecycle();
      setSubmissionTargetSelection(target);
      setTargetSubmissionExport(null);
      setTargetSubmissionPackagePlan(null);
      const plan = await invoke<SubmissionTargetPlan>("get_submission_target_plan", { workspaceId: activeWorkspace.id });
      setSubmissionTargetPlan(plan);
      setJournalRequirementSnapshots(await invoke<JournalRequirementSnapshot[]>("get_journal_requirement_snapshots", { workspaceId: activeWorkspace.id }));
      refreshSubmissionMaterials(activeWorkspace.id);
    } catch (error) {
      setErrorMessage(normalizeError(error));
    } finally {
      setIsSelectingTarget(false);
    }
  }

  async function clearPrimarySubmissionTarget(selectionId: string) {
    if (!activeWorkspace || targetPlanBusyId) return;
    setTargetPlanBusyId(selectionId);
    setErrorMessage(null);
    try {
      const plan = await invoke<SubmissionTargetPlan>("clear_primary_submission_target", { workspaceId: activeWorkspace.id, primarySelectionId: selectionId, authorConfirmed: true });
      setReadinessReport(null);
      resetDownstreamLifecycle();
      setSubmissionTargetPlan(plan);
      setSubmissionTargetSelection(null);
      setTargetSubmissionExport(null);
      setTargetSubmissionPackagePlan(null);
      setJournalRequirementSnapshots(await invoke<JournalRequirementSnapshot[]>("get_journal_requirement_snapshots", { workspaceId: activeWorkspace.id }));
      refreshSubmissionMaterials(activeWorkspace.id);
    } catch (error) {
      setErrorMessage(normalizeError(error));
    } finally {
      setTargetPlanBusyId(null);
    }
  }

  async function addBackupSubmissionTarget(recommendationRunId: string, journalId: string) {
    if (!activeWorkspace || targetPlanBusyId) return;
    setTargetPlanBusyId(journalId);
    setErrorMessage(null);
    try {
      const existingBackup = submissionTargetPlan?.backups.find((target) => target.journalId === journalId);
      const plan = existingBackup
        ? await invoke<SubmissionTargetPlan>("remove_backup_target", { workspaceId: activeWorkspace.id, backupSelectionId: existingBackup.selectionId })
        : await invoke<SubmissionTargetPlan>("add_backup_recommended_journal", { workspaceId: activeWorkspace.id, recommendationRunId, journalId });
      setSubmissionTargetPlan(plan);
    } catch (error) {
      setErrorMessage(normalizeError(error));
    } finally {
      setTargetPlanBusyId(null);
    }
  }

  async function promoteBackupSubmissionTarget(selectionId: string, reason: string) {
    if (!activeWorkspace || targetPlanBusyId) return;
    setTargetPlanBusyId(selectionId);
    setErrorMessage(null);
    try {
      const plan = await invoke<SubmissionTargetPlan>("promote_backup_target", { workspaceId: activeWorkspace.id, backupSelectionId: selectionId, reason });
      setReadinessReport(null);
      resetDownstreamLifecycle();
      setSubmissionTargetPlan(plan);
      setSubmissionTargetSelection(plan.primary);
      setTargetSubmissionExport(null);
      setTargetSubmissionPackagePlan(null);
      setJournalRequirementSnapshots(await invoke<JournalRequirementSnapshot[]>("get_journal_requirement_snapshots", { workspaceId: activeWorkspace.id }));
      refreshSubmissionMaterials(activeWorkspace.id);
    } catch (error) {
      setErrorMessage(normalizeError(error));
    } finally {
      setTargetPlanBusyId(null);
    }
  }

  async function discoverJournalRequirements(selectionId: string, options: OfficialFetchOptions) {
    if (!activeWorkspace || requirementBusyId) return;
    setRequirementBusyId(selectionId);
    setErrorMessage(null);
    try {
      const result = await invoke<OfficialFetchResult<JournalRequirementSnapshot>>("discover_journal_requirements", { workspaceId: activeWorkspace.id, targetSelectionId: selectionId, authorConfirmedExternalTransmission: true, options });
      const snapshot = result.snapshot;
      if (snapshot) {
        setJournalRequirementSnapshots((current) => [snapshot, ...current.filter((item) => item.targetSelectionId !== selectionId)]);
        if (submissionTargetSelection?.selectionId === selectionId) refreshSubmissionMaterials(activeWorkspace.id);
      }
      return result;
    } catch (error) {
      setErrorMessage(normalizeError(error));
    } finally {
      setRequirementBusyId(null);
    }
  }

  async function saveManualJournalRequirements(selectionId: string, sourceUrl: string, requirementText: string) {
    if (!activeWorkspace || requirementBusyId) return;
    setRequirementBusyId(selectionId);
    setErrorMessage(null);
    try {
      const snapshot = await invoke<JournalRequirementSnapshot>("save_manual_journal_requirements", { workspaceId: activeWorkspace.id, targetSelectionId: selectionId, sourceUrl, requirementText, authorAttestedOfficial: true });
      setJournalRequirementSnapshots((current) => [snapshot, ...current.filter((item) => item.targetSelectionId !== selectionId)]);
      if (submissionTargetSelection?.selectionId === selectionId) refreshSubmissionMaterials(activeWorkspace.id);
    } catch (error) {
      setErrorMessage(normalizeError(error));
    } finally {
      setRequirementBusyId(null);
    }
  }

  async function exportTargetSubmission() {
    if (!activeWorkspace || isExportingSubmission) return;
    setIsExportingSubmission(true);
    setErrorMessage(null);
    try {
      const result = await invoke<TargetSubmissionExport | null>("export_target_submission_package", { workspaceId: activeWorkspace.id });
      if (result) {
        setTargetSubmissionExport(result);
        refreshTargetSubmissionPackagePlan(activeWorkspace.id);
      }
    } catch (error) {
      setErrorMessage(normalizeError(error));
    } finally {
      setIsExportingSubmission(false);
    }
  }

  async function recordSubmission() {
    if (!activeWorkspace || !submissionTargetSelection || !submissionConfirmed || isRecordingSubmission) return;
    setIsRecordingSubmission(true); setErrorMessage(null);
    try {
      const record = await invoke<SubmissionRecord>("record_manual_submission", { workspaceId: activeWorkspace.id, target: submissionTargetSelection.name, receipt: submissionReceipt.trim() || null, authorConfirmed: true });
      setSubmission(record);
      setSubmissionConfirmed(false);
    } catch (error) { setErrorMessage(normalizeError(error)); }
    finally { setIsRecordingSubmission(false); }
  }

  async function finalizeKnowledgeBody() {
    if (!activeWorkspace || !selectedDisciplineCode || !knowledgeBodySnapshot || !knowledgeReviewConfirmed || isFinalizingKnowledge) return;
    const candidates = knowledgeCandidates(knowledgeBodySnapshot);
    if (candidates.some((candidate) => !knowledgeCandidateDecisions[candidate.candidateId])) return;
    setIsFinalizingKnowledge(true); setErrorMessage(null);
    try {
      const decisions = candidates.map((candidate) => ({ candidateId: candidate.candidateId, included: knowledgeCandidateDecisions[candidate.candidateId] === "included" }));
      const record = await invoke<KnowledgeBodyRecord>("finalize_knowledge_body", { workspaceId: activeWorkspace.id, disciplineCode: selectedDisciplineCode, decisions, authorConfirmed: true });
      setKnowledgeBodyRecord(record);
      setKnowledgeBodySnapshot(record.snapshot);
      setKnowledgeReviewConfirmed(false);
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
            {WORKSPACE_SECTIONS.filter((section) => section.id !== "overview").map((section) => <button key={section.id} className="rail-button" type="button" aria-label={localize(locale, section.zh, section.en)} title={text("创建工作区后可用", "Available after creating a workspace")} disabled><Icon name={section.icon} /></button>)}
          </nav>

          <main id="main-content" className="landing-main">
            <header className="landing-workspace-head"><h1 id="page-title">{text("我的工作台", "My Workspace")}</h1></header>
            <div className="landing-content">
              <section className="brand-statement" aria-labelledby="brand-statement-title">
                <h2 id="brand-statement-title" className="brand-statement-title">
                  <span lang="zh-CN">投稿舱</span>
                  <span lang="en">ManuscriptDock</span>
                  <span className="brand-version">{PRODUCT_VERSION}</span>
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
                      <div><p className="status-line"><Icon name="check" /> {text("本地校验完成", "Local validation complete")}</p><h2>{manuscript.name}</h2><p>{manuscript.extension.toUpperCase()} · {formatBytes(manuscript.sizeBytes, locale)}</p></div>
                      <button className="text-button" type="button" onClick={selectManuscript} disabled={isSelecting}>{isSelecting ? text("正在打开…", "Opening…") : text("重新选择", "Choose another")}</button>
                    </div>
                    <dl className="summary-grid">
                      <div><dt>{text("文件格式", "Format")}</dt><dd>{manuscript.extension.toUpperCase()}</dd></div>
                      <div><dt>{text("文件大小", "Size")}</dt><dd>{formatBytes(manuscript.sizeBytes, locale)}</dd></div>
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

              <section className="storage-policy-card" aria-labelledby="storage-policy-heading"><div><p className="field-kicker">{text("默认读取与存取位置", "Default read and storage location")}</p><h2 id="storage-policy-heading">{text("ManuscriptDock 本地资料库", "ManuscriptDock local library")}</h2><code>{workspaceStorage?.defaultLocation ?? text("正在定位本地资料库…", "Locating the local library…")}</code></div><p>{text("每篇论文导入后建立独立工作区，自动保存源快照、全部版本、推荐期刊与出版社、检查和存证记录。", "Each imported manuscript gets an independent workspace that automatically stores its source snapshot, versions, recommended journals and publishers, checks, and attestations.")}</p><small>{text("“另存工作区”会复制完整档案到你选择的文件夹；当前资料库和原始稿件都不会被移动。", "Save Workspace As copies the complete record to a folder you choose; neither the current library nor the original manuscript is moved.")}</small></section>

              <section className="track-grid" aria-label={text("本地工作原则", "Local workspace principles")}>
                <article className="track-card"><span>01</span><h2>{text("源稿不变", "Source stays unchanged")}</h2><p>{text("所有处理基于版本化工作副本，原稿始终保持只读。", "All processing uses versioned working copies; the source remains read-only.")}</p></article>
                <article className="track-card"><span>02</span><h2>{text("传输可见", "Transfers stay visible")}</h2><p>{text("你自主决定是否联网、使用模型和外部投送。", "You decide whether to go online, use models, or send work externally.")}</p></article>
              </section>
              {recentWorkspaces.length > 0 || archivedWorkspaces.length > 0 || catalogWarnings.length > 0 || workspaceManagementNotice || workspaceManagementError ? <RecentWorkspaces workspaces={recentWorkspaces} archivedWorkspaces={archivedWorkspaces} warnings={catalogWarnings.map((warning) => localizeBackendText(locale, warning))} busyId={workspaceManagementBusyId} notice={resolveNotice(locale, workspaceManagementNotice)} error={workspaceManagementError ? localizeBackendText(locale, workspaceManagementError) : null} onOpen={openRecentWorkspace} onManage={manageWorkspace} onSaveCopy={saveWorkspaceCopy} /> : null}
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
  const activeSection: WorkspaceSection | null = activeStage === "source" ? "overview" : activeStage === "materials" ? "materials" : activeStage === "journals" ? "journals" : activeStage === "check" || activeStage === "revision" ? "prepare" : activeStage === "submission" ? "submission" : null;
  return (
    <div className="app-shell workspace-shell">
      <a className="skip-link" href="#main-content">{text("跳到主要内容", "Skip to main content")}</a>
      <ProductBar manuscriptName={activeWorkspace.manuscript.name} onNewManuscript={selectManuscript} isSelecting={isSelecting} />
      <div className="workbench">
        <aside className="workspace-sidebar">
          <button className="sidebar-home" type="button" onClick={openWorkspaceHome}><Icon name="workspace" /><span>{text("所有论文", "All manuscripts")}</span></button>
          <nav className="primary-task-nav" aria-label={text("投稿准备主任务", "Primary submission tasks")}>
            {WORKSPACE_SECTIONS.map((section) => {
              const complete = section.id === "overview" || (section.id === "materials" && authorMaterialInputsReady(submissionMaterials)) || (section.id === "journals" && isSubmissionTargetCurrent(activeWorkspace, submissionTargetSelection) && journalRequirementSnapshotReady(currentJournalRequirementSnapshot(submissionTargetSelection, journalRequirementSnapshots))) || (section.id === "prepare" && Boolean(readinessReport && readinessReport.blockedCount === 0)) || (section.id === "submission" && Boolean(submission));
              return <button key={section.id} type="button" title={localize(locale, section.zh, section.en)} aria-current={activeSection === section.id ? "page" : undefined} data-complete={complete} onClick={() => openStage(section.stage)}><Icon name={section.icon} /><span>{section.id === "prepare" && locale === "en" ? <>Check<br />revise</> : localize(locale, section.zh, section.en)}</span><small>{complete ? text("已完成", "Done") : text("待处理", "Pending")}</small></button>;
            })}
          </nav>
          <details className="advanced-nav" open={["versions", "attestation", "knowledge"].includes(activeStage)}>
            <summary>{text("记录与高级功能", "Records & advanced")}</summary>
            <button type="button" aria-current={activeStage === "versions" ? "page" : undefined} onClick={() => openStage("versions")}><Icon name="versions" />{text("版本历史", "Version history")}</button>
            <button type="button" aria-current={activeStage === "attestation" ? "page" : undefined} onClick={() => openStage("attestation")}><Icon name="lock" />{text("本地存证", "Local attestation")}</button>
            <button type="button" aria-current={activeStage === "knowledge" ? "page" : undefined} onClick={() => openStage("knowledge")}><Icon name="knowledge" />{text("个人知识体", "Personal knowledge body")}</button>
          </details>
        </aside>

        <main id="main-content" className="workspace-main">
          <header className="workflow-header workflow-summary-header">
            <div><p className="breadcrumb">{text("论文工作区", "Manuscript workspace")} <span>/</span> {currentStageLabel}</p><h1>{currentStageLabel}</h1><p>{getStageDescription(activeStage, locale)}</p></div>
            <div className="workflow-header-actions"><div className="workspace-context"><span>v{activeWorkspace.snapshotVersion}</span><span>{submissionTargetSelection ? (locale === "en" ? submissionTargetSelection.nameEn : submissionTargetSelection.name) : text("未选目标期刊", "No target selected")}</span></div><StageStatus stage={activeStage} structureReport={structureReport} readinessReport={readinessReport} /><button className="evidence-toggle" type="button" aria-expanded={evidenceOpen} aria-controls="evidence-pane" onClick={() => setEvidenceOpen((current) => !current)}>{evidenceOpen ? text("收起依据", "Hide evidence") : text("查看依据", "View evidence")}</button></div>
          </header>
          <SubmissionFlowRail catalog={submissionMaterials} />
          {(activeStage === "check" || activeStage === "revision") ? <nav className="prepare-subnav" aria-label={text("检查与修订", "Check and revise")}><button type="button" aria-current={activeStage === "check" ? "page" : undefined} onClick={() => openStage("check")}>{text("检查", "Check")}</button><button type="button" aria-current={activeStage === "revision" ? "page" : undefined} onClick={() => openStage("revision")}>{text("修订", "Revise")}</button></nav> : null}
          <div className="workspace-panes" data-evidence-open={evidenceOpen}>
            <section id="operation-pane" className="operation-pane" aria-label={`${currentStageLabel} ${text("操作", "Actions")}`}>
              <OperationPane stage={activeStage} workspace={activeWorkspace} structureReport={structureReport} readinessReport={readinessReport} knowledgeBodySnapshot={knowledgeBodySnapshot} knowledgeBodyRecord={knowledgeBodyRecord} disciplineCatalog={disciplineCatalog} selectedDisciplineCode={selectedDisciplineCode} knowledgeCandidateDecisions={knowledgeCandidateDecisions} knowledgeReviewConfirmed={knowledgeReviewConfirmed} attestation={attestation} submission={submission} submissionExport={submissionExport} submissionMaterials={submissionMaterials} submissionTargetSelection={submissionTargetSelection} submissionTargetPlan={submissionTargetPlan} journalRequirementSnapshots={journalRequirementSnapshots} targetSubmissionExport={targetSubmissionExport} targetSubmissionPackagePlan={targetSubmissionPackagePlan} ruleCatalog={ruleCatalog} selectedRulePackIds={selectedRulePackIds} submissionElementCatalog={submissionElementCatalog} revisionDraft={revisionDraft} revisionValues={revisionValues} revisionResult={revisionResult} versionHistory={versionHistory} selectedVersion={selectedVersion} versionCandidate={versionCandidate} versionNote={versionNote} versionNotice={resolveNotice(locale, versionNotice)} attestationConfirmed={attestationConfirmed} submissionConfirmed={submissionConfirmed} submissionReceipt={submissionReceipt} isLoadingRuleCatalog={isLoadingRuleCatalog} isLoadingSubmissionElements={isLoadingSubmissionElements} isLoadingLifecycle={isLoadingLifecycle} isLoadingKnowledgeBody={isLoadingKnowledgeBody} isLoadingDisciplineCatalog={isLoadingDisciplineCatalog} isApplyingRevision={isApplyingRevision} isAnalyzing={isAnalyzing} isEvaluating={isEvaluating} isSelectingVersion={isSelectingVersion} isSavingVersion={isSavingVersion} isRestoringVersion={isRestoringVersion} isAttesting={isAttesting} isExportingSubmission={isExportingSubmission} isAddingMaterials={isAddingMaterials} confirmingRequirementId={confirmingRequirementId} isSelectingTarget={isSelectingTarget} targetPlanBusyId={targetPlanBusyId} requirementBusyId={requirementBusyId} isRecordingSubmission={isRecordingSubmission} isFinalizingKnowledge={isFinalizingKnowledge} onAnalyze={analyzeWorkspace} onEvaluate={evaluateReadiness} onToggleRulePack={toggleRulePack} onOpenStage={openStage} onRevisionValueChange={(field, value) => setRevisionValues((current) => ({ ...current, [field]: value }))} onApplyRevision={applyRevision} onSelectVersionCandidate={selectVersionCandidate} onVersionNoteChange={setVersionNote} onSaveVersion={saveVersion} onSelectVersion={(version) => compareVersions(activeWorkspace, version, activeWorkspace.snapshotVersion)} onRestoreVersion={restoreVersion} onAttestationConfirmed={setAttestationConfirmed} onCreateAttestation={createAttestation} onExportSubmission={exportSubmission} onAddMaterial={(kind, checklistItemId) => void addSubmissionMaterials(kind, checklistItemId)} onSetMaterialIncluded={(materialId, included) => void setSubmissionMaterialIncluded(materialId, included)} onDeleteMaterial={(materialId) => void deleteSubmissionMaterial(materialId)} onConfirmSubmissionRequirement={(itemId, confirmed) => void confirmSubmissionRequirement(itemId, confirmed)} onRecommendationGenerated={syncRecommendationAnalysis} onSelectSubmissionTarget={(runId, journalId) => void selectSubmissionTarget(runId, journalId)} onClearPrimaryTarget={(selectionId) => void clearPrimarySubmissionTarget(selectionId)} onAddBackupTarget={(runId, journalId) => void addBackupSubmissionTarget(runId, journalId)} onPromoteBackupTarget={(selectionId, reason) => void promoteBackupSubmissionTarget(selectionId, reason)} onDiscoverJournalRequirements={discoverJournalRequirements} onSaveManualJournalRequirements={(selectionId, sourceUrl, requirementText) => void saveManualJournalRequirements(selectionId, sourceUrl, requirementText)} onExportTargetSubmission={() => void exportTargetSubmission()} onSubmissionConfirmed={setSubmissionConfirmed} onSubmissionReceiptChange={setSubmissionReceipt} onRecordSubmission={recordSubmission} onDisciplineChange={setSelectedDisciplineCode} onKnowledgeCandidateDecision={(candidateId, decision) => setKnowledgeCandidateDecisions((current) => ({ ...current, [candidateId]: decision }))} onKnowledgeReviewConfirmed={setKnowledgeReviewConfirmed} onFinalizeKnowledge={finalizeKnowledgeBody} />
            </section>
            <aside id="evidence-pane" className="evidence-pane" aria-label={`${currentStageLabel} ${text("证据", "Evidence")}`}><button className="evidence-close" type="button" onClick={() => setEvidenceOpen(false)} aria-label={text("关闭依据", "Close evidence")}><Icon name="close" /></button><EvidencePane stage={activeStage} workspace={activeWorkspace} structureReport={structureReport} readinessReport={readinessReport} knowledgeBodySnapshot={knowledgeBodySnapshot} knowledgeBodyRecord={knowledgeBodyRecord} attestation={attestation} submission={submission} submissionExport={submissionExport} submissionMaterials={submissionMaterials} submissionTargetSelection={submissionTargetSelection} submissionTargetPlan={submissionTargetPlan} journalRequirementSnapshots={journalRequirementSnapshots} targetSubmissionExport={targetSubmissionExport} ruleCatalog={ruleCatalog} selectedRulePackIds={selectedRulePackIds} submissionElementCatalog={submissionElementCatalog} revisionDraft={revisionDraft} revisionValues={revisionValues} revisionResult={revisionResult} versionHistory={versionHistory} selectedVersion={selectedVersion} versionComparison={versionComparison} isComparingVersions={isComparingVersions} /></aside>
          </div>
          {errorMessage ? <ErrorNotice message={localizeBackendText(locale, errorMessage)} onRetry={activeStage === "check" ? (structureReport ? evaluateReadiness : analyzeWorkspace) : activeStage === "materials" ? () => refreshSubmissionMaterials(activeWorkspace.id) : activeStage === "versions" ? () => loadVersionHistory(activeWorkspace) : () => openStage(activeStage)} /> : null}
        </main>
      </div>
      <LiveStatus selecting={isSelecting} analyzing={isAnalyzing} evaluating={isEvaluating} />
    </div>
  );
}

function ProductBar({ manuscriptName, onNewManuscript, isSelecting = false }: { manuscriptName?: string; onNewManuscript?: () => void; isSelecting?: boolean }) {
  const { locale, setLocale, text } = useI18n();
  const [modelSettingsOpen, setModelSettingsOpen] = useState(false);
  const [modelSettings, setModelSettings] = useState<ModelSettingsSummary | null>(null);
  useEffect(() => {
    if (!isTauri()) return;
    void invoke<ModelSettingsSummary>("get_model_settings")
      .then(setModelSettings)
      .catch(() => setModelSettings(null));
  }, []);
  const configuredModelCount = modelSettings?.slots?.filter((slot) => slot.enabled && slot.hasApiKey && slot.providerLabel && slot.baseUrl && slot.model).length ?? 0;
  return <>
    <header className="product-bar"><div className="brand" aria-label={`投稿舱 ManuscriptDock ${PRODUCT_VERSION}`}><span className="brand-mark" aria-hidden="true"><img src={manuscriptDockLogo} alt="" width="32" height="32" /></span><span className="brand-copy"><span className="brand-cn" lang="zh-CN">投稿舱</span><span className="brand-name" lang="en">ManuscriptDock</span><span className="brand-version">{PRODUCT_VERSION}</span></span></div>{manuscriptName ? <p className="current-manuscript" title={manuscriptName}>{manuscriptName}</p> : <span className="current-manuscript" aria-hidden="true" />}<div className="bar-actions"><div className="language-switch" role="group" aria-label={text("界面语言", "Interface language")}><button type="button" aria-pressed={locale === "zh-CN"} onClick={() => setLocale("zh-CN")}>中文</button><button type="button" aria-pressed={locale === "en"} onClick={() => setLocale("en")}>EN</button></div><button className="bar-button model-config-button" type="button" aria-label={text("模型设置", "Models")} aria-haspopup="dialog" aria-expanded={modelSettingsOpen} onClick={() => setModelSettingsOpen(true)} title={text("配置模型与 API Key", "Configure models and API keys")}><Icon name="settings" /><span>{text("模型设置", "Models")}</span>{configuredModelCount > 0 ? <b>{configuredModelCount}</b> : null}</button><span className="local-badge" title={text("稿件尚未离开你的设备", "The manuscript has not left your device")}><Icon name="lock" />{text("仅在本机", "Local only")}</span>{onNewManuscript ? <button className="bar-button" type="button" onClick={onNewManuscript} disabled={isSelecting}>{isSelecting ? text("正在打开…", "Opening…") : text("导入另一篇", "Import another")}</button> : null}</div></header>
    <GlobalModelSettingsDialog open={modelSettingsOpen} onClose={() => setModelSettingsOpen(false)} onSaved={setModelSettings} />
  </>;
}

function GlobalModelSettingsDialog({ open, onClose, onSaved }: { open: boolean; onClose: () => void; onSaved: (summary: ModelSettingsSummary) => void }) {
  const { locale, text } = useI18n();
  const dialogRef = useRef<HTMLElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const dirtyRef = useRef(false);
  const [settings, setSettings] = useState<ModelSettingsSummary | null>(null);
  const [slotDrafts, setSlotDrafts] = useState<ModelSlotDraft[]>(MODEL_SLOT_ROLES.map(emptyModelSlot));
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [confirmClose, setConfirmClose] = useState(false);
  const [notice, setNotice] = useState<LocalizedCopy | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => { dirtyRef.current = dirty; }, [dirty]);

  const applySettings = (summary: ModelSettingsSummary) => {
    setSettings(summary);
    setSlotDrafts(MODEL_SLOT_ROLES.map((role) => {
      const slot = summary.slots.find((item) => item.role === role);
      return slot ? { ...slot, apiKey: "", clearApiKey: false } : emptyModelSlot(role);
    }));
    setDirty(false);
  };

  useEffect(() => {
    if (!open) return;
    setLoading(true);
    setConfirmClose(false);
    setNotice(null);
    setError(null);
    void invoke<ModelSettingsSummary>("get_model_settings")
      .then(applySettings)
      .catch((reason) => setError(normalizeError(reason)))
      .finally(() => setLoading(false));
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const appShell = document.querySelector<HTMLElement>(".app-shell");
    appShell?.setAttribute("inert", "");
    closeButtonRef.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        if (dirtyRef.current) setConfirmClose(true);
        else onClose();
        return;
      }
      if (event.key !== "Tab") return;
      const focusable = Array.from(dialogRef.current?.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex='-1'])") ?? []);
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      appShell?.removeAttribute("inert");
    };
  }, [open, onClose]);

  if (!open) return null;
  const configuredSlots = settings?.slots.filter((slot) => slot.enabled && slot.hasApiKey && slot.providerLabel && slot.baseUrl && slot.model) ?? [];
  const invalidEnabledDraft = slotDrafts.some((slot) => slot.enabled && (
    !slot.providerLabel.trim() || !slot.baseUrl.trim() || !slot.model.trim()
    || isDeepSeekDocumentationUrl(slot.baseUrl)
    || slot.clearApiKey
    || (!slot.hasApiKey && !slot.apiKey.trim())
  ));
  const updateSlot = <K extends keyof ModelSlotDraft>(role: ModelSlotRole, field: K, value: ModelSlotDraft[K]) => {
    setSlotDrafts((current) => current.map((slot) => slot.role === role ? { ...slot, [field]: value } : slot));
    setDirty(true);
    setNotice(null);
  };
  const applyDeepSeekPreset = (role: ModelSlotRole) => {
    setSlotDrafts((current) => current.map((slot) => slot.role === role ? { ...slot, enabled: true, ...DEEPSEEK_PRESET } : slot));
    setDirty(true);
    setNotice(null);
  };
  const requestClose = () => {
    if (dirty) setConfirmClose(true);
    else onClose();
  };
  const saveSettings = async () => {
    setSaving(true);
    setError(null);
    setNotice(null);
    try {
      const summary = await invoke<ModelSettingsSummary>("save_model_settings", {
        slots: slotDrafts.map(({ role, enabled, providerLabel, baseUrl, model, apiKey, clearApiKey }) => ({ role, enabled, providerLabel, baseUrl, model, apiKey: apiKey.trim() || null, clearApiKey })),
      });
      applySettings(summary);
      onSaved(summary);
      window.dispatchEvent(new CustomEvent<ModelSettingsSummary>("manuscriptdock:model-settings-updated", { detail: summary }));
      setNotice(localizedCopy("模型设置已保存；API Key 仅保存在系统凭据库。", "Model settings saved; API keys remain only in the system credential store."));
    } catch (reason) {
      setError(normalizeError(reason));
    } finally {
      setSaving(false);
    }
  };

  return createPortal(<div className="model-settings-overlay">
    <section ref={dialogRef} className="global-model-settings-dialog" role="dialog" aria-modal="true" aria-labelledby="global-model-settings-title">
      <header className="global-model-settings-header"><div><span>{text("全局设置 · 首次模型调用前完成", "Global settings · complete before the first model call")}</span><h2 id="global-model-settings-title">{text("模型与 API Key", "Models and API keys")}</h2><p>{text("期刊资料发现、学校规则抽取和知识体问答共用这一套配置。", "Journal discovery, institution-rule extraction, and knowledge-body questions share this configuration.")}</p></div><div><strong>{text(`${configuredSlots.length}/3 可用`, `${configuredSlots.length}/3 ready`)}</strong><button ref={closeButtonRef} type="button" aria-label={text("关闭模型设置", "Close model settings")} onClick={requestClose}><Icon name="close" /></button></div></header>
      {loading ? <p className="model-settings-loading" role="status">{text("正在读取系统凭据状态…", "Reading credential status…")}</p> : <section className="model-settings global-model-settings" aria-label={text("模型槽位设置", "Model slot settings")}>
        <header><div><h3>{text("1 个主模型，2 个备选模型", "One primary and two fallback models")}</h3><p>{text("按主模型 → 备选 1 → 备选 2 自动尝试。接口需兼容 OpenAI Chat Completions。", "Automatic order: primary → fallback 1 → fallback 2. Endpoints must support OpenAI Chat Completions.")}</p></div><span>{settings?.secureStore ?? text("系统凭据库", "System credential store")}</span></header>
        <div className="model-slot-grid">{slotDrafts.map((slot) => {
          const slotIncomplete = !slot.providerLabel.trim() || !slot.baseUrl.trim() || !slot.model.trim();
          const keyMissing = slot.clearApiKey || (!slot.hasApiKey && !slot.apiKey.trim());
          const ready = slot.enabled && !slotIncomplete && !isDeepSeekDocumentationUrl(slot.baseUrl) && !keyMissing;
          return <fieldset className="model-slot" key={slot.role}>
            <legend>{modelSlotLabel(slot.role, locale)}</legend>
            <div className="model-slot-heading"><span className={`model-slot-status ${ready ? "is-ready" : ""}`}>{!slot.enabled ? text("未启用", "Disabled") : slotIncomplete ? text("配置不完整", "Incomplete") : isDeepSeekDocumentationUrl(slot.baseUrl) ? text("API 地址错误", "Wrong API URL") : keyMissing ? text("缺少 API Key", "API key required") : slot.apiKey.trim() ? text("保存后可用", "Ready after save") : text("已启用", "Enabled")}</span><button type="button" onClick={() => applyDeepSeekPreset(slot.role)}>{text("使用 DeepSeek 官方配置", "Use DeepSeek preset")}</button></div>
            <label className="model-enabled"><input type="checkbox" checked={slot.enabled} onChange={(event) => updateSlot(slot.role, "enabled", event.target.checked)} />{text("启用此槽位", "Enable this slot")}</label>
            <label>{text("提供方名称", "Provider label")}<input maxLength={100} value={slot.providerLabel} onChange={(event) => updateSlot(slot.role, "providerLabel", event.target.value)} placeholder={text("例如：OpenAI", "e.g. OpenAI")} /></label>
            <label>{text("API 地址", "API base URL")}<input maxLength={500} aria-label={`${modelSlotLabel(slot.role, locale)} ${text("API 地址", "API base URL")}`} value={slot.baseUrl} aria-invalid={isDeepSeekDocumentationUrl(slot.baseUrl)} onChange={(event) => updateSlot(slot.role, "baseUrl", event.target.value)} placeholder="https://api.example.com/v1" inputMode="url" />{isDeepSeekDocumentationUrl(slot.baseUrl) ? <span className="model-field-error" role="alert">{text("这是 DeepSeek 文档页，不是 API。请改用 https://api.deepseek.com", "This is the DeepSeek documentation site, not its API. Use https://api.deepseek.com")}</span> : null}</label>
            <label>{text("模型名称", "Model name")}<input maxLength={200} value={slot.model} onChange={(event) => updateSlot(slot.role, "model", event.target.value)} placeholder="model-id" /></label>
            <label>{text("API Key", "API key")}<input maxLength={2000} aria-label={`${modelSlotLabel(slot.role, locale)} API Key`} type="password" autoComplete="new-password" value={slot.apiKey} aria-invalid={slot.enabled && !slot.hasApiKey && !slot.apiKey.trim()} onChange={(event) => { updateSlot(slot.role, "apiKey", event.target.value); updateSlot(slot.role, "clearApiKey", false); }} placeholder={slot.hasApiKey ? text("已安全保存；留空表示保留", "Stored securely; leave blank to retain") : text("输入后保存到系统凭据库", "Saved to the system credential store")} />{slot.enabled && !slot.hasApiKey && !slot.apiKey.trim() ? <span className="model-field-error" role="alert">{text("启用模型需要 API Key；输入后点击“保存模型设置”。", "An API key is required. Enter it, then save the model settings.")}</span> : null}</label>
            <label className="model-clear-key"><input type="checkbox" checked={slot.clearApiKey} onChange={(event) => updateSlot(slot.role, "clearApiKey", event.target.checked)} disabled={!slot.hasApiKey} />{text("删除已保存的 Key", "Delete saved key")}</label>
          </fieldset>;
        })}</div>
        <div className="model-settings-actions"><p>{text("Key 由 Rust 写入操作系统凭据库，界面不会读取或回显明文。同一次应用运行中，每个已保存 Key 只向系统凭据库读取一次；点击具体模型任务即完成该次授权，不再重复勾选。", "Rust stores keys in the operating system credential store, and the interface never reads or reveals plaintext. During one app session, each stored key is read from the credential store only once; clicking a specific model task authorizes that call without another checkbox.")}</p><button className="primary-button" type="button" disabled={saving || invalidEnabledDraft} onClick={() => void saveSettings()}>{saving ? text("保存中…", "Saving…") : invalidEnabledDraft ? text("请先补全启用项", "Complete enabled slots") : text("保存模型设置", "Save model settings")}</button></div>
      </section>}
      {error ? <p className="dialogue-message dialogue-error" role="alert">{localizeBackendText(locale, error)}</p> : null}
      {notice ? <p className="dialogue-message" role="status">{localize(locale, notice.zhCN, notice.en)}</p> : null}
      {confirmClose ? <div className="model-settings-close-confirm" role="group" aria-label={text("确认关闭模型设置", "Confirm closing model settings")}><div><strong>{text("放弃尚未保存的模型设置？", "Discard unsaved model settings?")}</strong><p>{text("已经安全保存到系统凭据库的 Key 不受影响。", "Keys already stored in the system credential store are not affected.")}</p></div><div><button type="button" onClick={() => setConfirmClose(false)}>{text("继续编辑", "Keep editing")}</button><button type="button" onClick={() => { setDirty(false); setConfirmClose(false); onClose(); }}>{text("放弃更改", "Discard changes")}</button></div></div> : null}
    </section>
  </div>, document.body);
}

function SubmissionGuide() {
  const { text } = useI18n();
  const items = [
    ["1", text("建立论文工作区", "Create a manuscript workspace"), text("保存不可变源快照，所有处理绑定明确版本", "Save an immutable source snapshot and bind every action to a version")],
    ["2", text("先推荐，再选择目标期刊", "Recommend first, then choose a journal"), text("仅凭当前主稿生成初步候选；作者再确定主投与备选", "Use the current manuscript for preliminary candidates, then choose primary and backup targets")],
    ["3", text("按目标组织投稿资料", "Organize target-specific materials"), text("依据所选期刊的官方要求准备原图、表格、投稿信与声明", "Prepare figures, tables, cover letter, and declarations against the selected journal's official requirements")],
    ["4", text("检查与修订", "Check and revise"), text("运行可追溯检查，安全修订并保存新版本", "Run traceable checks, revise safely, and save a new version")],
    ["5", text("生成真实投稿包", "Create the actual package"), text("只向出版社上传 submission；records 留在本机", "Upload only submission to the publisher; keep records locally")],
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

function RecentWorkspaces({ workspaces, archivedWorkspaces, warnings, busyId, notice, error, onOpen, onManage, onSaveCopy }: { workspaces: WorkspaceSummary[]; archivedWorkspaces: WorkspaceSummary[]; warnings: string[]; busyId: string | null; notice: string | null; error: string | null; onOpen: (workspace: WorkspaceSummary) => void; onManage: (action: "archive" | "restore" | "delete", workspace: WorkspaceSummary, archived: boolean) => Promise<boolean>; onSaveCopy: (workspace: WorkspaceSummary, archived: boolean) => Promise<boolean>; }) {
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

  const saveCopy = async (workspace: WorkspaceSummary) => {
    if (await onSaveCopy(workspace, archived)) setMenuWorkspaceId(null);
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
          {menuOpen ? <div className="workspace-management-menu" id={`workspace-menu-${workspace.id}`} role="menu" aria-label={text(`${workspace.manuscript.name} 管理操作`, `Management actions for ${workspace.manuscript.name}`)}><button type="button" role="menuitem" onClick={() => void saveCopy(workspace)}><Icon name="package" />{text("另存完整工作区…", "Save Workspace As…")}</button><button type="button" role="menuitem" onClick={() => void perform(archived ? "restore" : "archive", workspace)}><Icon name={archived ? "restore" : "archive"} />{archived ? text("恢复到最近工作区", "Restore to recent") : text("归档工作区", "Archive workspace")}</button><button className="workspace-delete-action" type="button" role="menuitem" onClick={() => { setDeleteWorkspaceId(workspace.id); setMenuWorkspaceId(null); }}><Icon name="trash" />{text("永久删除…", "Delete permanently…")}</button></div> : null}
        </div>
        {confirmingDelete ? <div className="workspace-delete-confirm" role="group" aria-labelledby={`delete-title-${workspace.id}`}><div><strong id={`delete-title-${workspace.id}`}>{text("永久删除这个论文工作区？", "Permanently delete this manuscript workspace?")}</strong><p>{text("将删除全部论文版本、分析、检查、存证、投稿和知识体问答记录。此操作无法撤销。", "All manuscript versions, analyses, checks, attestations, submissions, and knowledge-body dialogue will be deleted. This cannot be undone.")}</p></div><div><button type="button" onClick={() => setDeleteWorkspaceId(null)} disabled={isBusy}>{text("取消", "Cancel")}</button><button className="confirm-delete-button" type="button" onClick={() => void perform("delete", workspace)} disabled={isBusy}><Icon name="trash" />{isBusy ? text("正在删除…", "Deleting…") : text("确认永久删除", "Delete permanently")}</button></div></div> : null}
      </li>;
    })}</ul> : <div className="workspace-catalog-empty"><Icon name={archived ? "archive" : "file"} /><p>{archived ? text("还没有归档的论文工作区。", "No archived manuscript workspaces yet.") : text("最近工作区为空。", "No recent workspaces.")}</p></div>}
    {warnings.map((warning) => <p className="catalog-warning" key={warning}>{warning}</p>)}
  </section>;
}

function getStageDescription(stage: WorkspaceStage, locale: Locale) {
  const descriptions: Record<WorkspaceStage, string> = {
    source: localize(locale, "确认当前论文、目标与准备进度。", "Review the manuscript, target, and preparation progress."),
    materials: localize(locale, "按目标期刊官方要求补齐分章节篇幅、原图、表格、声明及其他支持文件。", "Complete section-length, figure, table, declaration, and supporting-file requirements from the target journal's official guidance."),
    check: localize(locale, "在一个阶段完成结构提取、规则选择和逐条投稿检查。", "Extract structure, choose rules, and run itemized checks in one stage."),
    revision: localize(locale, "依据检查结果修订安全字段，并保存为新的不可变版本。", "Revise safe fields from check evidence and save an immutable new version."),
    versions: localize(locale, "查看、比较或恢复不可变版本；这是可选的高级记录，不会打断投稿主线。", "View, compare, or restore immutable versions; this optional record does not interrupt the submission flow."),
    journals: localize(locale, "基于当前版本和公开目录快照，在本机生成冲刺、匹配和保底投稿组合。", "Build a local reach, match, and safeguard submission portfolio from the current version and public directory snapshot."),
    attestation: localize(locale, "作者确认当前版本与检查报告，建立本地加密完整性记录。", "Author-confirm the current version and report as a local integrity record."),
    submission: localize(locale, "导出投稿交付包，并在外部提交后登记目标和回执。", "Export the submission handoff, then record the target and receipt after external submission."),
    knowledge: localize(locale, "固化并查看由稿件、证据与投稿记录组成的个人知识体快照。", "Finalize and view the personal knowledge-body snapshot formed by the manuscript, evidence, and submission record."),
  };
  return descriptions[stage];
}

function isSubmissionTargetCurrent(workspace: WorkspaceSummary, target: SubmissionTargetSelection | null) {
  return target?.selectedAgainstManuscriptVersion === workspace.snapshotVersion;
}

function currentJournalRequirementSnapshot(target: SubmissionTargetSelection | null, snapshots: JournalRequirementSnapshot[]) {
  if (!target) return null;
  return snapshots.find((snapshot) => snapshot.targetSelectionId === target.selectionId) ?? null;
}

function journalRequirementSnapshotReady(snapshot: JournalRequirementSnapshot | null) {
  return Boolean(snapshot
    && snapshot.status !== "requires_manual_review"
    && snapshot.requirements.length > 0
    && snapshot.freshUntilUnixMs >= Date.now());
}

function authorMaterialInputsReady(catalog: SubmissionMaterialCatalog | null) {
  if (!catalog) return false;
  return catalog.checklist
    .filter((item) => item.blocking && item.id !== "target-journal" && item.id !== "official-journal-requirements")
    .every((item) => item.status === "passed");
}

function SubmissionFlowRail({ catalog }: { catalog: SubmissionMaterialCatalog | null }) {
  const { text } = useI18n();
  const steps = [
    { id: "recommendation", label: text("初步推荐", "Preliminary recommendation"), copy: text("依据主稿内容", "From manuscript content"), complete: Boolean(catalog?.recommendationReady), active: catalog?.workflowStatus === "manuscript_received" },
    { id: "verified", label: text("目标已核验", "Target verified"), copy: text("期刊、文章类型与官方要求", "Journal, article type, and official rules"), complete: Boolean(catalog?.targetVerified), active: catalog?.workflowStatus === "preliminary_recommendation" },
    { id: "materials", label: text("材料已齐", "Materials complete"), copy: text("必需文件与作者确认", "Required files and author confirmations"), complete: Boolean(catalog?.requiredComplete), active: catalog?.workflowStatus === "target_verified" || catalog?.workflowStatus === "materials_required" },
    { id: "ready", label: text("投稿已就绪", "Submission ready"), copy: text("完成当前目标检查", "Current target check complete"), complete: Boolean(catalog?.targetCheckReady), active: catalog?.workflowStatus === "materials_complete_check_required" },
  ];
  return <ol className="submission-flow-rail" aria-label={text("投稿准备进度", "Submission preparation progress")}>{steps.map((step, index) => <li key={step.id} data-complete={step.complete} data-active={step.active}><span>{step.complete ? <Icon name="check" /> : String(index + 1).padStart(2, "0")}</span><div><strong>{step.label}</strong><small>{step.copy}</small></div></li>)}</ol>;
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

interface PaneProps { stage: WorkspaceStage; workspace: WorkspaceSummary; structureReport: StructureReport | null; readinessReport: ReadinessReport | null; knowledgeBodySnapshot?: AcademicKnowledgeBodySnapshot | null; knowledgeBodyRecord?: KnowledgeBodyRecord | null; disciplineCatalog?: DisciplineCatalogItem[]; selectedDisciplineCode?: string; attestation?: LocalAttestation | null; submission?: SubmissionRecord | null; submissionExport?: SubmissionExport | null; submissionMaterials?: SubmissionMaterialCatalog | null; submissionTargetSelection?: SubmissionTargetSelection | null; submissionTargetPlan?: SubmissionTargetPlan | null; journalRequirementSnapshots?: JournalRequirementSnapshot[]; targetSubmissionExport?: TargetSubmissionExport | null; targetSubmissionPackagePlan?: TargetSubmissionPackagePlan | null; ruleCatalog?: RulePackCatalogItem[]; selectedRulePackIds?: string[]; submissionElementCatalog?: SubmissionElementCatalog | null; revisionDraft?: RevisionDraft | null; revisionValues?: Record<string, string>; revisionResult?: RevisionSet | null; versionHistory?: VersionHistory | null; selectedVersion?: number | null; versionComparison?: VersionComparison | null; isComparingVersions?: boolean; }


type OperationPaneProps = PaneProps & {
  versionCandidate: ManuscriptSummary | null; versionNote: string; versionNotice: string | null;
  knowledgeCandidateDecisions: Record<string, KnowledgeCandidateDecision>; knowledgeReviewConfirmed: boolean;
  attestationConfirmed: boolean; submissionConfirmed: boolean; submissionReceipt: string;
  isLoadingRuleCatalog: boolean; isLoadingSubmissionElements: boolean; isLoadingLifecycle: boolean; isLoadingKnowledgeBody: boolean; isLoadingDisciplineCatalog: boolean;
  isApplyingRevision: boolean; isAnalyzing: boolean; isEvaluating: boolean; isSelectingVersion: boolean; isSavingVersion: boolean; isRestoringVersion: boolean;
  isAttesting: boolean; isExportingSubmission: boolean; isAddingMaterials: boolean; confirmingRequirementId: string | null; isSelectingTarget: boolean; targetPlanBusyId: string | null; requirementBusyId: string | null; isRecordingSubmission: boolean; isFinalizingKnowledge: boolean;
  onAnalyze: () => void; onEvaluate: () => void; onToggleRulePack: (rulePackId: string) => void; onOpenStage: (stage: WorkspaceStage) => void;
  onRevisionValueChange: (field: string, value: string) => void; onApplyRevision: () => void;
  onSelectVersionCandidate: () => void; onVersionNoteChange: (note: string) => void; onSaveVersion: () => void; onSelectVersion: (version: number) => void; onRestoreVersion: (version: number) => void;
  onAttestationConfirmed: (confirmed: boolean) => void; onCreateAttestation: () => void; onExportSubmission: () => void;
  onAddMaterial: (kind: SubmissionMaterialKind, checklistItemId?: string) => void; onSetMaterialIncluded: (materialId: string, included: boolean) => void; onDeleteMaterial: (materialId: string) => void; onConfirmSubmissionRequirement: (itemId: string, confirmed: boolean) => void; onRecommendationGenerated: () => void; onSelectSubmissionTarget: (recommendationRunId: string, journalId: string) => void; onClearPrimaryTarget: (selectionId: string) => void; onAddBackupTarget: (recommendationRunId: string, journalId: string) => void; onPromoteBackupTarget: (selectionId: string, reason: string) => void;
  onDiscoverJournalRequirements: DiscoverOfficialSource<JournalRequirementSnapshot>; onSaveManualJournalRequirements: (selectionId: string, sourceUrl: string, requirementText: string) => void; onExportTargetSubmission: () => void;
  onSubmissionConfirmed: (confirmed: boolean) => void; onSubmissionReceiptChange: (receipt: string) => void; onRecordSubmission: () => void;
  onDisciplineChange: (code: string) => void;
  onKnowledgeCandidateDecision: (candidateId: string, decision: KnowledgeCandidateDecision) => void;
  onKnowledgeReviewConfirmed: (confirmed: boolean) => void;
  onFinalizeKnowledge: () => void;
};

function OperationPane(props: OperationPaneProps) {
  const { locale, text } = useI18n();
  const { stage, workspace, structureReport, readinessReport, knowledgeBodySnapshot = null, knowledgeBodyRecord = null, disciplineCatalog = [], selectedDisciplineCode = "", knowledgeCandidateDecisions, knowledgeReviewConfirmed, attestation = null, submission = null, submissionExport = null, submissionMaterials = null, submissionTargetSelection = null, submissionTargetPlan = null, journalRequirementSnapshots = [], targetSubmissionExport = null, targetSubmissionPackagePlan = null, ruleCatalog = [], selectedRulePackIds = [], submissionElementCatalog = null, revisionDraft = null, revisionValues = {}, revisionResult = null, versionHistory, selectedVersion, versionCandidate, versionNote, versionNotice, attestationConfirmed, submissionConfirmed, submissionReceipt, isLoadingRuleCatalog, isLoadingSubmissionElements, isLoadingLifecycle, isLoadingKnowledgeBody, isLoadingDisciplineCatalog, isApplyingRevision, isAnalyzing, isEvaluating, isSelectingVersion, isSavingVersion, isRestoringVersion, isAttesting, isExportingSubmission, isAddingMaterials, confirmingRequirementId, isSelectingTarget, targetPlanBusyId, requirementBusyId, isRecordingSubmission, isFinalizingKnowledge, onAnalyze, onEvaluate, onToggleRulePack, onOpenStage, onRevisionValueChange, onApplyRevision, onSelectVersionCandidate, onVersionNoteChange, onSaveVersion, onSelectVersion, onRestoreVersion, onAttestationConfirmed, onCreateAttestation, onExportSubmission, onAddMaterial, onSetMaterialIncluded, onDeleteMaterial, onConfirmSubmissionRequirement, onRecommendationGenerated, onSelectSubmissionTarget, onClearPrimaryTarget, onAddBackupTarget, onPromoteBackupTarget, onDiscoverJournalRequirements, onSaveManualJournalRequirements, onExportTargetSubmission, onSubmissionConfirmed, onSubmissionReceiptChange, onRecordSubmission, onDisciplineChange, onKnowledgeCandidateDecision, onKnowledgeReviewConfirmed, onFinalizeKnowledge } = props;
  const targetCurrent = isSubmissionTargetCurrent(workspace, submissionTargetSelection);
  const targetRequirements = currentJournalRequirementSnapshot(submissionTargetSelection, journalRequirementSnapshots);
  const targetRequirementsReady = targetCurrent && journalRequirementSnapshotReady(targetRequirements);

  if (isLoadingLifecycle) return <EmptyStage icon="package" kicker={text("恢复流程", "Restore workflow")} title={text("正在恢复当前版本的流程记录", "Restoring lifecycle records for the current version")} copy={text("只读取与当前内容指纹匹配的结构、检查、存证、投稿和知识体记录。", "Only structure, checks, attestation, submission, and knowledge records matching the current fingerprint are restored.")} />;

  if (stage === "source") return <WorkspaceOverview workspace={workspace} readinessReport={readinessReport} materials={submissionMaterials} target={submissionTargetSelection} requirementSnapshots={journalRequirementSnapshots} submission={submission} onOpenStage={onOpenStage} />;

  if (stage === "materials") return <SubmissionMaterialsCenter workspace={workspace} catalog={submissionMaterials} busy={isAddingMaterials} confirmingRequirementId={confirmingRequirementId} targetReady={targetRequirementsReady} onAdd={onAddMaterial} onSetIncluded={onSetMaterialIncluded} onDelete={onDeleteMaterial} onConfirm={onConfirmSubmissionRequirement} onContinue={() => onOpenStage(targetRequirementsReady ? "check" : "journals")} />;

  if (stage === "check") {
    if (!targetRequirementsReady) return <EmptyStage icon="target" kicker={text("主任务 4 · 检查与修订", "Primary task 4 · Check & revise")} title={!targetCurrent ? text("先确认当前版本的目标期刊", "Confirm a target for the current version first") : text("先取得目标期刊的有效要求", "Capture valid target-journal requirements first")} copy={text("检查必须建立在当前稿件版本、当前出版社目标和带来源的作者指南之上。", "Checks must be based on the current manuscript version, current publisher target, and source-backed author guidance.")} actionLabel={text("返回目标期刊", "Return to target journal")} onAction={() => onOpenStage("journals")} />;
    if (!structureReport) return <EmptyStage icon="structure" kicker={text("主任务 4 · 检查与修订", "Primary task 4 · Check & revise")} title={text("先建立论文结构", "First establish manuscript structure")} copy={text("标题、作者、摘要、章节和声明只在本机确定性提取。", "Title, authors, abstract, sections, and declarations are extracted deterministically on this device.")} actionLabel={isAnalyzing ? text("正在提取…", "Extracting…") : text("提取论文结构", "Extract structure")} disabled={isAnalyzing} onAction={onAnalyze} />;
    if (!readinessReport) return <><StructureCheckSummary report={structureReport} /><TargetRuleSelector ruleCatalog={ruleCatalog} selectedRulePackIds={selectedRulePackIds} loading={isLoadingRuleCatalog} structureReady onToggle={onToggleRulePack} onContinue={onEvaluate} actionLabel={isEvaluating ? text("正在检查…", "Checking…") : text("运行投稿检查", "Run submission checks")} disabled={isEvaluating} /></>;
    return <><PanelHeading kicker={`${text("主任务 4 · 投稿检查", "Primary task 4 · Submission check")} · v${readinessReport.reportVersion}`} title={outcomeLabel(readinessReport.outcome, locale)} copy={text("报告与当前稿件版本、目标期刊、内容指纹和规则来源绑定。", "The report is bound to the current manuscript version, target journal, fingerprint, and rule sources.")} /><div className="metric-row" aria-label={text("投稿检查统计", "Submission-check metrics")}><Metric label={text("通过", "Passed")} value={readinessReport.passedCount} /><Metric label={text("建议", "Suggestions")} value={readinessReport.warningCount} /><Metric label={text("阻断", "Blocked")} value={readinessReport.blockedCount} /><Metric label={text("待确认", "Confirmations")} value={readinessReport.confirmationCount} /></div><ol className="finding-list" aria-label={text("投稿检查明细", "Submission-check details")}>{readinessReport.findings.map((finding) => <li key={finding.ruleId} data-status={finding.status}><span className="finding-status">{findingLabel(finding.status, locale)}</span><div><strong>{locale === "en" && finding.messageEn ? finding.messageEn : localizeBackendText(locale, finding.message)}</strong><code>{finding.sourceLocation}</code></div></li>)}</ol><div className="secondary-action-row"><button className="text-button" type="button" onClick={onEvaluate} disabled={isEvaluating}>{text("重新检查", "Run again")}</button></div><PaneAction label={text("下一步", "Next")} title={!authorMaterialInputsReady(submissionMaterials) ? text("补齐检查后发现的投稿资料", "Complete materials identified by the check") : text("根据结论修订", "Revise from findings")} copy={!authorMaterialInputsReady(submissionMaterials) ? text("结构提取发现了新的原图或附件要求；补齐后返回本检查结果。", "Structure extraction identified additional figure or attachment requirements; return to this check after adding them.") : text("进入安全字段修订台；每次保存都会生成新版本并自动复查。", "Open safe-field revision; every save creates a new version and automatically rechecks it.")} buttonLabel={!authorMaterialInputsReady(submissionMaterials) ? text("补齐投稿资料", "Complete materials") : text("进入修订", "Continue to revision")} onClick={() => onOpenStage(authorMaterialInputsReady(submissionMaterials) ? "revision" : "materials")} /></>;
  }

  if (stage === "revision") {
    if (!readinessReport) return <EmptyStage icon="format" kicker={text("主任务 4 · 修订", "Primary task 4 · Revise")} title={text("需要当前版本的检查报告", "A current check report is required")} copy={text("修订必须从可追溯的检查依据开始。", "Revision must start from traceable check evidence.")} actionLabel={text("返回检查", "Return to checks")} onAction={() => onOpenStage("check")} />;
    return <SubmissionElementsDesk catalog={submissionElementCatalog} draft={revisionDraft} values={revisionValues} result={revisionResult} loading={isLoadingSubmissionElements} saving={isApplyingRevision} selectedPublisherCount={ruleCatalog.filter((item) => item.category === "publisher" && selectedRulePackIds.includes(item.id)).length} onValueChange={onRevisionValueChange} onSave={onApplyRevision} onContinue={() => onOpenStage(submissionMaterials?.targetCheckReady ? "submission" : "materials")} />;
  }

  if (stage === "versions") return <VersionManager workspace={workspace} history={versionHistory ?? null} selectedVersion={selectedVersion ?? null} candidate={versionCandidate} note={versionNote} notice={versionNotice} selecting={isSelectingVersion} saving={isSavingVersion} restoring={isRestoringVersion} onSelectCandidate={onSelectVersionCandidate} onNoteChange={onVersionNoteChange} onSave={onSaveVersion} onSelectVersion={onSelectVersion} onRestore={onRestoreVersion} onContinue={() => onOpenStage(readinessReport ? "submission" : "check")} continueReady={readinessReport !== null} />;

  if (stage === "journals") return <JournalMatchStage workspace={workspace} selectedTarget={submissionTargetSelection} targetPlan={submissionTargetPlan} requirementSnapshots={journalRequirementSnapshots} selectingTarget={isSelectingTarget} targetPlanBusyId={targetPlanBusyId} requirementBusyId={requirementBusyId} onRecommendationGenerated={onRecommendationGenerated} onSelectTarget={onSelectSubmissionTarget} onClearPrimary={onClearPrimaryTarget} onAddBackup={onAddBackupTarget} onPromoteBackup={onPromoteBackupTarget} onDiscoverRequirements={onDiscoverJournalRequirements} onSaveManualRequirements={onSaveManualJournalRequirements} onContinue={() => onOpenStage("materials")} />;

  if (stage === "attestation") {
    if (!readinessReport) return <EmptyStage icon="package" kicker={text("高级记录 · 本地存证", "Advanced record · Local attestation")} title={text("当前版本尚未检查", "The current version has not been checked")} copy={text("新版本不会继承旧版本的检查与存证。", "A new version never inherits checks or attestation from an older version.")} actionLabel={text("检查当前版本", "Check current version")} onAction={() => onOpenStage("check")} />;
    if (attestation) return <><PanelHeading kicker={text("高级记录 · 本地存证", "Advanced record · Local attestation")} title={text(`v${attestation.manuscriptVersion} 已完成存证`, `v${attestation.manuscriptVersion} attested`)} copy={text("记录绑定稿件指纹、检查报告、作者确认和时间；不宣称上链或证明科学结论。", "The record binds the manuscript fingerprint, check report, author confirmation, and time; it does not claim blockchain notarization or scientific truth.")} /><LifecycleRecord label="Attestation" id={attestation.attestationId} hash={attestation.recordHash} timestamp={attestation.attestedUnixMs} /><PaneAction label={text("返回主流程", "Return to primary flow")} title={text("查看投稿包", "View submission package")} copy={text("存证是可选高级记录，不是导出投稿包的前置步骤。", "Attestation is an optional advanced record, not a prerequisite for exporting the submission package.")} buttonLabel={text("返回投稿包", "Return to package")} onClick={() => onOpenStage("submission")} /></>;
    return <><PanelHeading kicker={text("高级记录 · 本地存证", "Advanced record · Local attestation")} title={text("确认当前证据边界", "Confirm the current evidence boundary")} copy={text(`将绑定稿件 v${workspace.snapshotVersion}、检查报告 ${readinessReport.reportId.slice(0, 8)} 和输出快照 v${readinessReport.outputSnapshotVersion}。`, `This binds manuscript v${workspace.snapshotVersion}, check report ${readinessReport.reportId.slice(0, 8)}, and output snapshot v${readinessReport.outputSnapshotVersion}.`)} /><label className="confirmation-control"><input type="checkbox" checked={attestationConfirmed} onChange={(event) => onAttestationConfirmed(event.target.checked)} /><span>{text("我已核对当前稿件、检查结论和待作者确认事项；我理解该记录不证明研究结论为真。", "I reviewed the current manuscript, findings, and author confirmations; I understand this record does not prove scientific truth.")}</span></label><BoundaryNote title={text("存证含义", "Meaning of attestation")} copy={text("这是带 SHA-256 的本地作者确认记录，不是区块链确权、公证或第三方时间戳。", "This is a SHA-256 local author-confirmation record, not blockchain ownership, notarization, or a third-party timestamp.")} /><button className="primary-button" type="button" disabled={!attestationConfirmed || isAttesting} onClick={onCreateAttestation}>{isAttesting ? text("正在创建…", "Creating…") : text("创建本地存证", "Create local attestation")}<Icon name="arrow" /></button></>;
  }

  if (stage === "submission") {
    if (!submissionTargetSelection) return <EmptyStage icon="target" kicker={text("投稿包", "Submission package")} title={text("先选择目标期刊", "Choose a target journal first")} copy={text("推荐结果需要明确设为本篇论文的投稿目标，之后才能形成面向出版社的资料包。", "Set one recommendation as this manuscript's submission target before creating a publisher-facing package.")} actionLabel={text("选择目标期刊", "Choose target journal")} onAction={() => onOpenStage("journals")} />;
    if (!targetCurrent) return <EmptyStage icon="target" kicker={text("主任务 5 · 投稿包", "Primary task 5 · Submission package")} title={text("目标期刊属于较早的稿件版本", "The target belongs to an earlier manuscript version")} copy={text("请按当前版本重新计算或重新确认目标，旧选择和要求仍保留在本地历史中。", "Recalculate or reconfirm the target for the current version; the prior selection and requirements remain in local history.")} actionLabel={text("重新确认目标期刊", "Reconfirm target journal")} onAction={() => onOpenStage("journals")} />;
    if (submission) return <><PanelHeading kicker={text("主任务完成 · 投稿记录", "Primary flow complete · Submission record")} title={submission.target} copy={text("作者已确认在外部投稿系统完成提交；ManuscriptDock 只保存本地登记。", "The author confirmed submission in an external system; ManuscriptDock stores only the local record.")} /><LifecycleRecord label="Submission" id={submission.submissionId} hash={submission.recordHash} timestamp={submission.submittedUnixMs} /><dl className="detail-list"><div><dt>{text("投稿目标", "Target")}</dt><dd>{submission.target}</dd></div><div><dt>{text("回执", "Receipt")}</dt><dd>{submission.receipt ?? text("未填写", "Not provided")}</dd></div></dl><PaneAction label={text("可选高级功能", "Optional advanced feature")} title={text("固化个人知识体快照", "Finalize a personal knowledge-body snapshot")} copy={text("主投稿流程已经完成；如有需要，再把稿件、检查、存证和投稿记录固定为个人研究记忆。", "The primary submission flow is complete. Optionally pin the manuscript, checks, attestation, and submission record as personal research memory.")} buttonLabel={text("查看个人知识体", "View personal knowledge body")} onClick={() => onOpenStage("knowledge")} /></>;
    return <><PanelHeading kicker={text("主任务 5 · 作者控制外发", "Primary task 5 · Author-controlled handoff")} title={text("生成出版社可用的投稿资料包", "Create a publisher-facing submission package")} copy={text("资料包严格按照已选期刊及出版社要求组织；submission 只放主稿和真实附件，检查记录单独保存在 records。", "The package is organized against the selected journal and publisher requirements; submission contains only the manuscript and genuine attachments, while checks stay in records.")} /><section className="selected-target-card"><span>{text("当前投稿主线", "Current primary target")}</span><h3>{locale === "en" ? submissionTargetSelection.nameEn : submissionTargetSelection.name}</h3><p>{submissionTargetSelection.publisher} · {articleTypeLabel(submissionTargetSelection.articleType, locale)} · {submissionTargetSelection.rankTier} · v{submissionTargetSelection.selectedAgainstManuscriptVersion} · {targetRequirementsReady ? text(`${targetRequirements?.requirements.length ?? 0} 项官方要求`, `${targetRequirements?.requirements.length ?? 0} official requirements`) : text("官方要求待核验", "Official requirements pending")}</p></section><TargetPackageAssembly plan={targetSubmissionPackagePlan} busy={isAddingMaterials} onSetIncluded={onSetMaterialIncluded} />{!targetRequirementsReady ? <BoundaryNote title={text("尚未建立该刊专属要求", "Journal-specific requirements are not ready")} copy={text("请回到“目标期刊”，逐次授权读取官网，或粘贴并确认官方作者指南原文。", "Return to Target Journal and authorize an official-site fetch or paste confirmed official author-guide text.")} /> : !targetSubmissionPackagePlan?.ready ? <BoundaryNote title={text("投稿包预检尚未通过", "Package preflight has not passed")} copy={text("请按上方阻断项返回材料或检查环节处理。系统不会把评估建议、旧目标附件或实名源稿误作出版社投稿文件。", "Resolve the blockers above in Materials or Checks. The app does not treat assessment notes, stale-target attachments, or an identified source manuscript as publisher files.")} /> : null}<section className="submission-action-card"><div><span>01</span><h3>{text("导出真正的投稿包", "Export the actual submission package")}</h3><p>{text("submission 文件夹供上传出版社；records 文件夹仅供本地留档，不应上传。", "Upload the submission folder to the publisher; records is local-only and should not be uploaded.")}</p></div><button className="secondary-button" type="button" onClick={onExportTargetSubmission} disabled={isExportingSubmission || !targetSubmissionPackagePlan?.ready}>{isExportingSubmission ? text("正在导出…", "Exporting…") : text("选择导出文件夹", "Choose export folder")}</button></section>{targetSubmissionExport ? <div className="revision-saved" role="status"><Icon name="check" /><span>{text(`已导出 ${targetSubmissionExport.packageName}（${targetSubmissionExport.files.length} 个文件）`, `Exported ${targetSubmissionExport.packageName} (${targetSubmissionExport.files.length} files)`)}</span></div> : null}<section className="submission-record-form" aria-labelledby="submission-record-heading"><header><span>02</span><h3 id="submission-record-heading">{text("投稿后登记回执", "Record the receipt after submission")}</h3></header><label htmlFor="submission-target">{text("投稿期刊（来自当前主线）", "Target journal (from current route)")}</label><input id="submission-target" value={submissionTargetSelection.name} readOnly aria-readonly="true" /><label htmlFor="submission-receipt">{text("稿件号或回执（可选）", "Manuscript ID or receipt (optional)")}</label><input id="submission-receipt" value={submissionReceipt} maxLength={200} onChange={(event) => onSubmissionReceiptChange(event.target.value)} /><label className="confirmation-control"><input type="checkbox" checked={submissionConfirmed} onChange={(event) => onSubmissionConfirmed(event.target.checked)} /><span>{text("我确认已经向上述期刊的外部投稿系统完成提交。确认后系统会自动建立本地存证并保存这条投稿记录。", "I confirm I submitted to the journal above through its external system. The app will automatically create a local attestation and store this submission record.")}</span></label><button className="primary-button" type="button" disabled={!submissionConfirmed || isRecordingSubmission} onClick={onRecordSubmission}>{isRecordingSubmission ? text("正在登记…", "Recording…") : text("登记投稿记录", "Record submission")}<Icon name="arrow" /></button></section>{attestation && !submission ? <details className="advanced-settings"><summary>{text("内部审核档案", "Internal review archive")}</summary><p>{text("此包包含检查报告、预览和存证，只用于内部复核，不是出版社投稿包。", "This archive contains checks, previews, and attestation for internal review; it is not a publisher submission package.")}</p><button className="text-button" type="button" onClick={onExportSubmission} disabled={isExportingSubmission}>{text("导出内部审核档案", "Export internal review archive")}</button>{submissionExport ? <small>{submissionExport.packageName}</small> : null}</details> : null}</>;
  }

  if (isLoadingKnowledgeBody && !knowledgeBodySnapshot) return <EmptyStage icon="package" kicker={text("高级功能 · 个人知识体", "Advanced feature · Personal knowledge body")} title={text("正在读取知识体快照", "Loading the knowledge-body snapshot")} copy={text("正在校验对象版本和生命周期引用。", "Verifying object versions and lifecycle references.")} />;
  if (!structureReport && knowledgeBodyRecord?.disciplineClassification) return <KnowledgeBodyOperation workspace={workspace} snapshot={knowledgeBodyRecord.snapshot} record={knowledgeBodyRecord} />;
  if (!structureReport) return <EmptyStage icon="package" kicker={text("高级功能 · 个人知识体", "Advanced feature · Personal knowledge body")} title={text("需要先完成结构提取", "Structure extraction is required first")} copy={text("提取论文文本、表格与图片线索后，系统会立即建立可追溯的候选知识体。", "After extracting text, table, and figure signals, the app immediately creates a traceable candidate knowledge body.")} actionLabel={text("返回检查", "Return to check")} onAction={() => onOpenStage("check")} />;
  if (!knowledgeBodySnapshot) return <EmptyStage icon="package" kicker={text("高级功能 · 个人知识体", "Advanced feature · Personal knowledge body")} title={text("尚未生成候选知识体", "Candidate knowledge body is not available")} copy={text("请重新运行当前版本的结构提取。", "Run structure extraction again for the current version.")} actionLabel={text("重新提取", "Extract again")} onAction={() => onOpenStage("check")} />;
  if (!submission) return <><PanelHeading kicker={text("高级功能 · 个人知识体候选", "Advanced feature · Personal knowledge-body candidates")} title={text("候选知识体已经建立", "Candidate knowledge body created")} copy={text("系统已保留源稿明示的作者身份、单位、联系方式和版本，并将可识别的 Claim、Scope、Method、Result 与 Evidence 保存为带来源的候选对象。", "The app preserves source-declared authors, affiliations, contact details, and version while storing recognizable Claim, Scope, Method, Result, and Evidence content as source-backed candidates.")} /><KnowledgeSpatialMap workspace={workspace} knowledgeBodySnapshot={knowledgeBodySnapshot} /><KnowledgeDialoguePanel workspace={workspace} knowledgeBodyRecord={null} /><SourceIdentityPreview snapshot={knowledgeBodySnapshot} /><KnowledgeCandidatePreview snapshot={knowledgeBodySnapshot} structureReport={structureReport} /><BoundaryNote title={text("候选与固化是两件事", "Extraction and finalization are separate")} copy={text("身份与版本可在本机直接查看；研究语义候选仍须审核。完成投稿登记后，作者选择学科分类并固化不可变快照。", "Identity and version are immediately visible locally, while research-semantic candidates still require review. After submission registration, the author selects a discipline and finalizes an immutable snapshot.")} /><button className="secondary-action" type="button" onClick={() => onOpenStage("submission")}>{text("返回投稿主流程", "Return to submission flow")}<Icon name="arrow" /></button></>;
  const finalizedDecompositionHash = knowledgeBodyRecord?.snapshot.extraction?.decompositionHash ?? null;
  const currentDecompositionHash = knowledgeBodySnapshot.extraction?.decompositionHash ?? null;
  const requiresUpdatedSnapshot = knowledgeBodyRecord !== null && (knowledgeBodyRecord.snapshot.schemaVersion !== knowledgeBodySnapshot.schemaVersion || finalizedDecompositionHash !== currentDecompositionHash);
  if (!knowledgeBodyRecord?.disciplineClassification || requiresUpdatedSnapshot) {
    const candidates = knowledgeCandidates(knowledgeBodySnapshot);
    const decidedCount = candidates.filter((candidate) => knowledgeCandidateDecisions[candidate.candidateId]).length;
    const reviewComplete = candidates.length > 0 && decidedCount === candidates.length;
    return <>
      <PanelHeading kicker={text("高级功能 · 个人知识体", "Advanced feature · Personal knowledge body")} title={requiresUpdatedSnapshot ? text("审核并更新知识体", "Review and update the knowledge body") : text("逐条确认研究知识", "Review the research knowledge item by item")} copy={text("对每条本地提取结果选择“纳入”或“排除”，再确认学科分类；只有纳入项会升级为作者确认的知识。", "Choose Include or Exclude for every locally extracted item, then confirm the discipline. Only included items become author-confirmed knowledge.")} />
      <KnowledgeSpatialMap workspace={workspace} knowledgeBodySnapshot={knowledgeBodySnapshot} />
      <KnowledgeDialoguePanel workspace={workspace} knowledgeBodyRecord={null} />
      <SourceIdentityPreview snapshot={knowledgeBodySnapshot} />
      <KnowledgeCandidatePreview snapshot={knowledgeBodySnapshot} structureReport={structureReport} decisions={knowledgeCandidateDecisions} onDecision={onKnowledgeCandidateDecision} reviewable />
      <p className="knowledge-review-progress" role="status">{text(`已审核 ${decidedCount} / ${candidates.length} 条`, `Reviewed ${decidedCount} / ${candidates.length} items`)}</p>
      <DisciplineSelector catalog={disciplineCatalog} selectedCode={selectedDisciplineCode} loading={isLoadingDisciplineCatalog} onChange={onDisciplineChange} />
      <label className="confirmation-control knowledge-review-attestation">
        <input type="checkbox" checked={knowledgeReviewConfirmed} disabled={!reviewComplete} onChange={(event) => onKnowledgeReviewConfirmed(event.target.checked)} />
        <span>{text("我已逐条核对候选内容及来源，并确认纳入项准确表达当前论文；排除项不会成为已确认知识。", "I reviewed every candidate and its source, and confirm that included items accurately represent this manuscript; excluded items will not become confirmed knowledge.")}</span>
      </label>
      <BoundaryNote title={text("本地作者审核", "Local author review")} copy={text("审核决定绑定当前分解哈希并写入不可变知识体快照；不会调用模型或发送论文。", "Review decisions are bound to the current decomposition hash and written into the immutable knowledge-body snapshot; no model is called and no manuscript is sent.")} />
      <button className="primary-button" type="button" disabled={!selectedDisciplineCode || !reviewComplete || !knowledgeReviewConfirmed || isLoadingDisciplineCatalog || isFinalizingKnowledge} onClick={onFinalizeKnowledge}>{isFinalizingKnowledge ? text("正在固化…", "Finalizing…") : requiresUpdatedSnapshot ? text("确认审核并更新快照", "Confirm review and update snapshot") : text("确认审核并固化知识体", "Confirm review and finalize")}<Icon name="arrow" /></button>
    </>;
  }
  return <KnowledgeBodyOperation workspace={workspace} snapshot={knowledgeBodyRecord.snapshot} record={knowledgeBodyRecord} structureReport={structureReport} />;
}

function WorkspaceOverview({ workspace, readinessReport, materials, target, requirementSnapshots, submission, onOpenStage }: { workspace: WorkspaceSummary; readinessReport: ReadinessReport | null; materials: SubmissionMaterialCatalog | null; target: SubmissionTargetSelection | null; requirementSnapshots: JournalRequirementSnapshot[]; submission: SubmissionRecord | null; onOpenStage: (stage: WorkspaceStage) => void }) {
  const { locale, text } = useI18n();
  const targetReady = isSubmissionTargetCurrent(workspace, target) && journalRequirementSnapshotReady(currentJournalRequirementSnapshot(target, requirementSnapshots));
  const nextStage: WorkspaceStage = !targetReady ? "journals" : !authorMaterialInputsReady(materials) ? "materials" : !readinessReport ? "check" : "submission";
  const nextLabels: Record<WorkspaceStage, [string, string]> = { source: ["查看概览", "View overview"], materials: ["补全投稿资料", "Complete materials"], journals: ["选择目标期刊", "Choose target journal"], check: ["检查当前稿件", "Check manuscript"], revision: ["修订稿件", "Revise manuscript"], versions: ["管理版本", "Manage versions"], attestation: ["查看本地记录", "View local records"], submission: ["生成投稿包", "Create submission package"], knowledge: ["查看知识体", "View knowledge body"] };
  return <>
    <p className="workspace-created-status"><Icon name="check" />{text("论文已安全保存在本地工作区", "The manuscript is safely stored in its local workspace")}</p>
    <PanelHeading kicker={text("论文概览", "Manuscript overview")} title={workspace.manuscript.name} copy={text("从这里确认当前版本、投稿目标和准备进度。高级记录不会打断主流程。", "Review the current version, submission target, and preparation progress here. Advanced records stay out of the main flow.")} />
    <div className="overview-summary-grid">
      <article><span>{text("当前版本", "Current version")}</span><strong>v{workspace.snapshotVersion}</strong><small>{workspace.manuscript.extension.toUpperCase()} · {formatBytes(workspace.manuscript.sizeBytes, locale)}</small></article>
      <article><span>{text("投稿资料", "Materials")}</span><strong>{authorMaterialInputsReady(materials) ? text("作者资料已齐", "Author files ready") : text("待补全", "Needs files")}</strong><small>{text(`${materials?.materials.length ?? 0} 个附加文件`, `${materials?.materials.length ?? 0} additional files`)}</small></article>
      <article><span>{text("目标期刊", "Target journal")}</span><strong>{targetReady && target ? (locale === "en" ? target.nameEn : target.name) : target ? text("需要按当前版本复核", "Needs current-version review") : text("尚未选择", "Not selected")}</strong><small>{target?.publisher ?? text("先选择出版社与期刊", "Choose a publisher and journal first")}</small></article>
      <article><span>{text("准备状态", "Readiness")}</span><strong>{submission ? text("已登记投稿", "Submission recorded") : readinessReport ? outcomeLabel(readinessReport.outcome, locale) : text("尚未检查", "Not checked")}</strong><small>{readinessReport ? `v${readinessReport.outputSnapshotVersion}` : text("检查只在本机运行", "Checks run locally")}</small></article>
    </div>
    <PaneAction label={text("建议下一步", "Recommended next step")} title={localize(locale, nextLabels[nextStage][0], nextLabels[nextStage][1])} copy={text("系统根据当前论文记录引导下一项主任务，你也可以从左侧直接切换。", "The app suggests the next primary task from the current manuscript record; you can also switch from the sidebar.")} buttonLabel={localize(locale, nextLabels[nextStage][0], nextLabels[nextStage][1])} onClick={() => onOpenStage(nextStage)} />
  </>;
}

function submissionChecklistCopy(item: SubmissionMaterialChecklistItem, locale: Locale) {
  if (locale === "zh-CN") return { label: item.label, detail: item.detail };
  const labels: Record<string, string> = { "main-manuscript": "Current manuscript", "target-journal": "Target journal and article type", "official-journal-requirements": "Official journal requirements", "latex-project": "Complete LaTeX project", "figure-originals": "Original figures", "table-editables": "Editable tables", "common-title-page": "Title and author-information page", "common-cover-letter": "Cover letter", "common-declaration-files": "Declaration documents", "common-bibliography-files": "Bibliography files", "common-supplementary-files": "Supplementary materials and research data", "common-explanation-files": "Explanations, responses, and other supporting files" };
  const details: Record<string, string> = {
    "main-manuscript": "The immutable current manuscript is stored in this workspace.",
    "target-journal": item.status === "passed" ? "The target, publisher, and article type are bound to this manuscript version." : "Choose one primary journal from the preliminary recommendations.",
    "official-journal-requirements": item.status === "passed" ? "A current source-backed author-guide snapshot is stored locally." : "Capture the selected journal's official author guide.",
    "latex-project": "Provide a ZIP or TAR project containing figures, bibliography, and custom styles.",
    "figure-originals": "The manuscript contains figures; provide original image files rather than PDF screenshots.",
    "table-editables": "The manuscript contains tables; provide editable CSV, Excel, Word, or LaTeX source files.",
    "common-title-page": "Use when author, affiliation, and correspondence details must be separate from an anonymized manuscript.",
    "common-cover-letter": "Add a letter to the editor when required or useful; follow the current journal's instructions.",
    "common-declaration-files": "Add ethics, consent, conflict-of-interest, funding, data-availability, author-contribution, or AI-use declarations.",
    "common-bibliography-files": "Add editable BIB, RIS, NBIB, EndNote, XML, or other bibliography files.",
    "common-supplementary-files": "Add appendices, extended methods, data, code archives, presentations, or media allowed by the journal.",
    "common-explanation-files": "Add explanatory notes, response letters, reporting checklists, copyright or permission forms, author agreements, and other supporting documents.",
  };
  return { label: labels[item.id] ?? item.labelEn ?? item.label, detail: details[item.id] ?? (item.sourceUrl ? "Review this item against the cited official evidence before submission." : item.detail) };
}

function submissionChecklistStatusLabel(status: SubmissionChecklistStatus, locale: Locale) {
  if (status === "passed") return localize(locale, "已完成", "Complete");
  if (status === "missing") return localize(locale, "缺失", "Missing");
  if (status === "author_confirmation") return localize(locale, "作者确认", "Author confirmation");
  if (status === "manual_verification") return localize(locale, "人工核验", "Manual verification");
  return localize(locale, "建议", "Recommended");
}

function materialAcceptedFormats(kind: SubmissionMaterialKind, locale: Locale) {
  const formats: Record<SubmissionMaterialKind, [string, string]> = {
    source_project: ["ZIP、TAR、GZ、TGZ", "ZIP, TAR, GZ, TGZ"],
    blinded_manuscript: ["DOC、DOCX、ODT、RTF、TEX、PDF", "DOC, DOCX, ODT, RTF, TEX, PDF"],
    figure: ["PDF、EPS、PS、SVG、PNG、JPG、TIFF", "PDF, EPS, PS, SVG, PNG, JPG, TIFF"],
    table: ["CSV、TSV、XLS、XLSX、ODS、Word、RTF、TEX", "CSV, TSV, XLS, XLSX, ODS, Word, RTF, TEX"],
    bibliography: ["BIB、BBL、RIS、NBIB、ENW、XML、Word、RTF、TXT", "BIB, BBL, RIS, NBIB, ENW, XML, Word, RTF, TXT"],
    supplementary: ["文档、数据、演示、压缩包、音视频及常用科研数据文件", "Documents, data, presentations, archives, media, and common research-data files"],
    cover_letter: ["DOC、DOCX、ODT、RTF、TEX、PDF、TXT", "DOC, DOCX, ODT, RTF, TEX, PDF, TXT"],
    title_page: ["DOC、DOCX、ODT、RTF、TEX、PDF、TXT", "DOC, DOCX, ODT, RTF, TEX, PDF, TXT"],
    declaration: ["DOC、DOCX、ODT、RTF、TEX、PDF、TXT", "DOC, DOCX, ODT, RTF, TEX, PDF, TXT"],
    other: ["文档、数据、演示、压缩包、音视频及常用科研数据文件", "Documents, data, presentations, archives, media, and common research-data files"],
  };
  return localize(locale, ...formats[kind]);
}

type PreparationTreeTone = "complete" | "attention" | "optional";

function matchedMaterialCount(items: SubmissionMaterialChecklistItem[]) {
  return new Set(items.flatMap((item) => item.matchedMaterialIds)).size;
}

function detectedMaterialCount(catalog: SubmissionMaterialCatalog | null, kind: "figure" | "table") {
  const explicit = kind === "figure" ? catalog?.detectedFigureCount : catalog?.detectedTableCount;
  if (typeof explicit === "number") return explicit;
  return Math.max(0, ...(catalog?.checklist ?? []).filter((item) => item.materialKind === kind).map((item) => item.requiredCount));
}

function preparationComparison(uploaded: number, detected: number, locale: Locale): { tone: PreparationTreeTone; label: string } {
  if (uploaded === detected) return { tone: "complete", label: localize(locale, "数量一致", "Counts match") };
  if (uploaded < detected) return { tone: "attention", label: localize(locale, `少 ${detected - uploaded} 个`, `${detected - uploaded} missing`) };
  return { tone: "attention", label: localize(locale, `多 ${uploaded - detected} 个 · 请核对`, `${uploaded - detected} extra · Review`) };
}

function SubmissionPreparationTree({ workspace, catalog }: { workspace: WorkspaceSummary; catalog: SubmissionMaterialCatalog | null }) {
  const { locale, text } = useI18n();
  const checklist = catalog?.checklist ?? [];
  const currentMaterials = (catalog?.materials ?? []).filter((material) => material.manuscriptVersion === workspace.snapshotVersion);
  const includedMaterials = currentMaterials.filter((material) => material.included);
  const figureItems = checklist.filter((item) => item.materialKind === "figure");
  const tableItems = checklist.filter((item) => item.materialKind === "table");
  const detectedFigures = detectedMaterialCount(catalog, "figure");
  const detectedTables = detectedMaterialCount(catalog, "table");
  const uploadedFigures = matchedMaterialCount(figureItems);
  const uploadedTables = matchedMaterialCount(tableItems);
  const figureComparison = preparationComparison(uploadedFigures, detectedFigures, locale);
  const tableComparison = preparationComparison(uploadedTables, detectedTables, locale);
  const requiredFiles = checklist.filter((item) => item.blocking && item.verification === "file" && item.materialKind !== "figure" && item.materialKind !== "table");
  const requiredFileTotal = requiredFiles.reduce((total, item) => total + item.requiredCount, 0);
  const requiredFileReady = requiredFiles.reduce((total, item) => total + Math.min(item.matchedMaterialIds.length, item.requiredCount), 0);
  const confirmationItems = checklist.filter((item) => item.blocking && item.confirmable);
  const completedConfirmations = confirmationItems.filter((item) => item.status === "passed").length;
  const optionalItems = checklist.filter((item) => item.id.startsWith("common-") && item.verification === "file");
  const optionalFileCount = matchedMaterialCount(optionalItems);
  const blockedFiles = currentMaterials.filter((material) => material.validationStatus === "blocked").length;
  const requiredTotal = catalog?.requiredTotal ?? 0;
  const requiredCompleted = catalog?.requiredCompleted ?? 0;
  const progress = requiredTotal === 0 ? 100 : Math.round((requiredCompleted / requiredTotal) * 100);
  const branches: Array<{ key: string; label: string; detail: string; metric: string; tone: PreparationTreeTone }> = [
    { key: "manuscript", label: text("当前主稿", "Current manuscript"), detail: text(`不可变稿件 v${workspace.snapshotVersion} 已纳入准备包`, `Immutable manuscript v${workspace.snapshotVersion} is included`), metric: text("1 个主文件", "1 main file"), tone: "complete" },
    { key: "figures", label: text("原始图件", "Original figures"), detail: text(`正文扫描 ${detectedFigures} 幅 · 已上传 ${uploadedFigures} 个`, `Scanned ${detectedFigures} figure(s) · ${uploadedFigures} uploaded`), metric: figureComparison.label, tone: figureComparison.tone },
    { key: "tables", label: text("可编辑表格", "Editable tables"), detail: text(`正文扫描 ${detectedTables} 个 · 已上传 ${uploadedTables} 个`, `Scanned ${detectedTables} table(s) · ${uploadedTables} uploaded`), metric: tableComparison.label, tone: tableComparison.tone },
    { key: "required", label: text("期刊必需附件", "Journal-required files"), detail: text("依据当前目标期刊的官方要求计算", "Calculated from the current journal's official requirements"), metric: catalog?.targetVerified ? `${requiredFileReady}/${requiredFileTotal}` : text("待生成", "Pending"), tone: catalog?.targetVerified && requiredFileReady >= requiredFileTotal ? "complete" : "attention" },
    { key: "confirmations", label: text("声明与作者确认", "Declarations & confirmations"), detail: text("只统计当前期刊要求的必需确认", "Counts only confirmations required by the current journal"), metric: catalog?.targetVerified ? `${completedConfirmations}/${confirmationItems.length}` : text("待生成", "Pending"), tone: catalog?.targetVerified && completedConfirmations >= confirmationItems.length ? "complete" : "attention" },
    { key: "optional", label: text("可选支持资料", "Optional supporting files"), detail: text("投稿信、参考文献、补充材料、说明及其他资料", "Cover letter, bibliography, supplements, explanations, and other files"), metric: text(`${optionalFileCount} 个文件`, `${optionalFileCount} file(s)`), tone: "optional" },
  ];
  return <section className="submission-preparation-tree" aria-labelledby="submission-preparation-tree-heading">
    <header><div><span>{text("文件包总览 · 自动对照正文", "Package overview · Compared with manuscript scan")}</span><h3 id="submission-preparation-tree-heading">{text("投稿包准备树", "Submission package preparation tree")}</h3></div><strong data-ready={Boolean(catalog?.requiredComplete)}>{catalog?.requiredComplete ? text("必需项已达标", "Required items met") : text("仍有必需项未达标", "Required items incomplete")}</strong></header>
    <div className="preparation-tree-root" data-ready={Boolean(catalog?.requiredComplete)}><span className="preparation-tree-root-icon"><Icon name="folder" /></span><div><strong>{text(`当前准备包 · ${1 + includedMaterials.length} 个拟组包文件`, `Current package · ${1 + includedMaterials.length} file(s) planned`)}</strong><p>{text(`工作区另有 ${currentMaterials.length - includedMaterials.length} 个当前版本文件未纳入 · ${blockedFiles} 个文件校验阻断`, `${currentMaterials.length - includedMaterials.length} current-version file(s) excluded · ${blockedFiles} blocked by validation`)}</p><div className="preparation-progress" role="progressbar" aria-label={text("必需项完成度", "Required-item completion")} aria-valuemin={0} aria-valuemax={requiredTotal} aria-valuenow={requiredCompleted}><i style={{ width: `${progress}%` }} /></div><small>{text(`必需项 ${requiredCompleted}/${requiredTotal}`, `Required ${requiredCompleted}/${requiredTotal}`)}</small></div></div>
    <ul>{branches.map((branch) => <li key={branch.key} data-tone={branch.tone}><span><Icon name={branch.tone === "complete" ? "check" : branch.tone === "attention" ? "warning" : "file"} /></span><div><strong>{branch.label}</strong><p>{branch.detail}</p></div><b>{branch.metric}</b></li>)}</ul>
    <small className="preparation-tree-note">{text("“数量一致”只表示正文检测槽位与已绑定文件数量一致；图片精度、表格可编辑性及期刊格式仍按下方清单核验。", "“Counts match” means only that detected manuscript slots and bound files have the same count. Figure quality, table editability, and journal formatting still require the checklist below.")}</small>
  </section>;
}

type SubmissionMaterialView = "overview" | "requirements" | "upload" | "files";

function SubmissionMaterialsCenter({ workspace, catalog, busy, confirmingRequirementId, targetReady, onAdd, onSetIncluded, onDelete, onConfirm, onContinue }: { workspace: WorkspaceSummary; catalog: SubmissionMaterialCatalog | null; busy: boolean; confirmingRequirementId: string | null; targetReady: boolean; onAdd: (kind: SubmissionMaterialKind, checklistItemId?: string) => void; onSetIncluded: (materialId: string, included: boolean) => void; onDelete: (materialId: string) => void; onConfirm: (itemId: string, confirmed: boolean) => void; onContinue: () => void }) {
  const { locale, text } = useI18n();
  const [deleteMaterialId, setDeleteMaterialId] = useState<string | null>(null);
  const [activeView, setActiveView] = useState<SubmissionMaterialView>("overview");
  const groupLabels: Record<SubmissionMaterialChecklistItem["group"], [string, string]> = { target: ["目标与依据", "Target & evidence"], manuscript: ["主稿与篇幅", "Manuscript & length"], files: ["投稿支持文件", "Supporting files"], declarations: ["声明与作者确认", "Declarations & confirmations"] };
  const groups: SubmissionMaterialChecklistItem["group"][] = ["target", "manuscript", "files", "declarations"];
  const authorInputsReady = authorMaterialInputsReady(catalog);
  const figureAndTableItems = (catalog?.checklist ?? []).filter((item) => item.verification === "file" && (item.materialKind === "figure" || item.materialKind === "table"));
  const commonMaterialItems = (catalog?.checklist ?? []).filter((item) => item.verification === "file" && item.id.startsWith("common-"));
  const storedMaterialCount = catalog?.materials.length ?? 0;
  const materialViews: Array<{ id: SubmissionMaterialView; label: string; hint: string; count: string }> = [
    { id: "overview", label: text("准备概览", "Overview"), hint: text("文件包与完成度", "Package and progress"), count: `${catalog?.requiredCompleted ?? 0}/${catalog?.requiredTotal ?? 0}` },
    { id: "requirements", label: text("要求清单", "Requirements"), hint: text("期刊要求与确认", "Journal rules and confirmations"), count: String((catalog?.checklist ?? []).filter((item) => !item.id.startsWith("common-")).length) },
    { id: "upload", label: text("上传资料", "Upload"), hint: text("按文件类型选择", "Choose by file type"), count: String(figureAndTableItems.length + commonMaterialItems.length) },
    { id: "files", label: text("已存文件", "Stored files"), hint: text("纳入、替换或删除", "Include, replace, or delete"), count: String(storedMaterialCount) },
  ];
  function switchMaterialViewByKeyboard(event: ReactKeyboardEvent<HTMLButtonElement>, index: number) {
    let nextIndex: number | null = null;
    if (event.key === "ArrowRight" || event.key === "ArrowDown") nextIndex = (index + 1) % materialViews.length;
    if (event.key === "ArrowLeft" || event.key === "ArrowUp") nextIndex = (index - 1 + materialViews.length) % materialViews.length;
    if (event.key === "Home") nextIndex = 0;
    if (event.key === "End") nextIndex = materialViews.length - 1;
    if (nextIndex === null) return;
    event.preventDefault();
    const nextView = materialViews[nextIndex];
    setActiveView(nextView.id);
    window.requestAnimationFrame(() => document.getElementById(`material-view-${nextView.id}`)?.focus());
  }
  return <>
    <PanelHeading kicker={text("主任务 3 · 目标专属准备", "Primary task 3 · Target-specific preparation")} title={text("按目标期刊组织投稿资料", "Organize materials for the target journal")} copy={targetReady ? text("清单由当前主稿、文章类型和官方作者指南共同生成；每条篇幅限制和支持文件要求都保留证据。", "The checklist is generated from the current manuscript, article type, and official author guide; every length limit and supporting-file requirement keeps its evidence.") : text("请先选择主投期刊并取得有效的官方要求，再开始补充资料。", "Choose a primary journal and capture valid official requirements before adding materials.")} />
    <nav className="material-view-switcher" role="tablist" aria-label={text("投稿资料查看模式", "Submission materials view")}>
      {materialViews.map((view, index) => <button key={view.id} type="button" role="tab" id={`material-view-${view.id}`} aria-selected={activeView === view.id} aria-controls={`material-panel-${view.id}`} tabIndex={activeView === view.id ? 0 : -1} onClick={() => setActiveView(view.id)} onKeyDown={(event) => switchMaterialViewByKeyboard(event, index)}><span><strong>{view.label}</strong><small>{view.hint}</small></span><b>{view.count}</b></button>)}
    </nav>
    {activeView === "overview" ? <section id="material-panel-overview" role="tabpanel" aria-labelledby="material-view-overview" className="material-view-panel"><SubmissionPreparationTree workspace={workspace} catalog={catalog} /><BoundaryNote title={text("AI 语义与语法审计 · 后续迭代", "AI semantic and grammar audit · Planned")} copy={text("当前版本不执行语法润色或语义改写，只处理可追溯的投稿准备、格式、篇幅、声明和支持文件要求。AI 审计能力将在后续版本上线，敬请期待。", "This version does not perform grammar polishing or semantic rewriting. It focuses on traceable submission preparation, format, length, declarations, and supporting files. AI audit is planned for a later release.")} /></section> : null}
    {activeView === "upload" ? <section id="material-panel-upload" role="tabpanel" aria-labelledby="material-view-upload" className="material-view-panel">
      <p className="material-replacement-policy"><Icon name="review" /><span>{text("上传时会检查当前稿件、当前期刊和同一上传项中的同名文件；发现同名文件后，必须确认才会替换工作区副本。", "Uploads are checked for duplicate names within the current manuscript, journal, and upload slot. Replacing an existing workspace copy always requires confirmation.")}</span></p>
      {figureAndTableItems.length > 0 ? <section className="material-upload-hub" aria-labelledby="material-upload-hub-heading"><header><div><span>{text("独立上传入口", "Dedicated upload entry")}</span><h3 id="material-upload-hub-heading">{text("图表文件", "Figures and tables")}</h3></div><strong>{text("按类型分开", "Separated by type")}</strong></header><div className="material-type-grid">{figureAndTableItems.map((item) => { const copy = submissionChecklistCopy(item, locale); const isFigure = item.materialKind === "figure"; return <article key={`upload-${item.id}`}><div><Icon name="file" /><span>{item.matchedMaterialIds.length}/{item.requiredCount}</span></div><h3>{isFigure ? text("上传原始图件", "Upload original figures") : text("上传可编辑表格", "Upload editable tables")}</h3><p>{materialAcceptedFormats(item.materialKind!, locale)}</p><button className="secondary-button" type="button" disabled={busy || !targetReady} aria-label={text(`为${copy.label}上传${isFigure ? "原始图件" : "可编辑表格"}`, `Upload ${isFigure ? "original figures" : "editable tables"} for ${copy.label}`)} onClick={() => onAdd(item.materialKind!, item.id)}>{busy ? text("正在添加…", "Adding…") : isFigure ? text("选择图片文件", "Choose figure files") : text("选择表格文件", "Choose table files")}</button></article>; })}</div></section> : null}
      {commonMaterialItems.length > 0 ? <section className="material-upload-hub common-material-upload-hub" aria-labelledby="common-material-upload-heading"><header><div><span>{text("按需补充 · 不默认设为必需", "Add as needed · Not required by default")}</span><h3 id="common-material-upload-heading">{text("常见投稿附件", "Common submission attachments")}</h3></div><strong>{text(`${commonMaterialItems.length} 类`, `${commonMaterialItems.length} types`)}</strong></header><p className="material-upload-hub-copy">{text("声明、说明及其他附件在不同期刊中的要求差异很大。这里始终提供分类入口；若官方作者指南明确要求，以带官方证据的动态清单为准。", "Journals vary widely in their declaration, explanation, and supporting-file requirements. These categorized upload entries remain available; when the official author guide is explicit, follow the evidence-backed dynamic checklist.")}</p><div className="material-type-grid common-material-type-grid">{commonMaterialItems.map((item) => { const copy = submissionChecklistCopy(item, locale); return <article key={`common-upload-${item.id}`}><div><Icon name="file" /><span>{text(`${item.matchedMaterialIds.length} · 可选`, `${item.matchedMaterialIds.length} · Optional`)}</span></div><h3>{copy.label}</h3><p>{copy.detail}</p><small>{text("支持：", "Accepted: ")}{materialAcceptedFormats(item.materialKind!, locale)}</small><button className="secondary-button" type="button" disabled={busy || !targetReady} aria-label={text(`上传${copy.label}`, `Upload ${copy.label}`)} onClick={() => onAdd(item.materialKind!, item.id)}>{busy ? text("正在添加…", "Adding…") : text("选择文件", "Choose files")}</button></article>; })}</div></section> : null}
    </section> : null}
    {activeView === "requirements" ? <section id="material-panel-requirements" role="tabpanel" aria-labelledby="material-view-requirements" className="material-view-panel materials-checklist"><header><div><span>{text("动态投稿清单", "Dynamic submission checklist")}</span><h3 id="materials-checklist-heading">{authorInputsReady ? text("作者侧必需资料与确认已完成", "Author-supplied files and confirmations are complete") : text("仍有必需项待处理", "Required items still need attention")}</h3></div><strong>{catalog ? `${catalog.requiredCompleted ?? 0}/${catalog.requiredTotal ?? 0}` : `v${workspace.snapshotVersion}`}</strong></header><div className="checklist-groups">{groups.map((group) => { const items = (catalog?.checklist ?? []).filter((item) => item.group === group && !item.id.startsWith("common-")); if (items.length === 0) return null; return <section className="checklist-group" key={group} aria-labelledby={`checklist-group-${group}`}><header><h4 id={`checklist-group-${group}`}>{localize(locale, ...groupLabels[group])}</h4><span>{items.length}</span></header><ul>{items.map((item) => { const copy = submissionChecklistCopy(item, locale); const confirmed = item.confirmable && item.status === "passed"; return <li key={item.id} data-status={item.status}><Icon name={item.status === "passed" ? "check" : "warning"} /><div><div className="checklist-item-heading"><strong>{copy.label}</strong><span>{submissionChecklistStatusLabel(item.status, locale)}</span></div><p>{copy.detail}</p>{item.requiredCount > 0 ? <small className="material-slot-count">{text(`已绑定 ${item.matchedMaterialIds.length}/${item.requiredCount} 个文件`, `${item.matchedMaterialIds.length}/${item.requiredCount} file(s) bound`)}</small> : null}{item.evidenceExcerpt ? <details className="checklist-evidence"><summary>{text("查看官方依据", "View official evidence")}</summary><blockquote>{item.evidenceExcerpt}</blockquote><a href={item.sourceUrl ?? undefined} target="_blank" rel="noreferrer">{item.sourceUrl}</a>{item.capturedUnixMs ? <small>{text("获取于", "Captured")} {formatModifiedDate(item.capturedUnixMs, locale)} · {item.freshUntilUnixMs && item.freshUntilUnixMs >= Date.now() ? text("有效", "Current") : text("需更新", "Refresh needed")}</small> : null}</details> : null}{item.verification === "file" && item.materialKind ? <><small className="accepted-material-formats">{text("支持：", "Accepted: ")}{materialAcceptedFormats(item.materialKind, locale)}</small><button className="secondary-button checklist-add-file" type="button" disabled={busy || !targetReady} aria-label={text(`为${copy.label}添加文件`, `Add file for ${copy.label}`)} onClick={() => onAdd(item.materialKind!, item.id)}>{busy ? text("正在添加…", "Adding…") : text("为此要求添加文件", "Add file for this requirement")}</button></> : null}{item.confirmable ? <button className="text-button checklist-confirm" type="button" disabled={confirmingRequirementId !== null} onClick={() => onConfirm(item.id, !confirmed)}>{confirmingRequirementId === item.id ? text("正在保存…", "Saving…") : confirmed ? text("撤销确认", "Revoke confirmation") : item.verification === "author" ? text("我已确认内容真实且适用", "I confirm this is accurate and applicable") : text("我已对照原文人工核验", "I verified this against the source")}</button> : null}</div><span>{item.blocking ? text("必需", "Required") : text("建议", "Recommended")}</span></li>; })}</ul></section>; })}</div></section> : null}
    {activeView === "files" ? <section id="material-panel-files" role="tabpanel" aria-labelledby="material-view-files" className="material-view-panel">{catalog && catalog.materials.length > 0 ? <section className="material-file-list"><h3>{text("已保存的附加文件", "Stored supporting files")}</h3><ul>{catalog.materials.map((material) => {
      const current = material.manuscriptVersion === workspace.snapshotVersion;
      const confirmingDelete = deleteMaterialId === material.materialId;
      return <li key={material.materialId} data-current={current}>
        <Icon name="file" />
        <div><strong>{material.originalName}</strong><span>{formatBytes(material.sizeBytes, locale)} · {material.contentHash.slice(0, 10)} · {current ? text("当前版本", "Current version") : text("历史材料", "Historical material")}</span>{material.validationIssues.map((issue) => <small key={issue}>{issue}</small>)}</div>
        <div className="material-file-actions">
          <label className="material-include-control"><input type="checkbox" checked={material.included} disabled={busy || !current} onChange={(event) => onSetIncluded(material.materialId, event.target.checked)} /><span>{text("纳入组包", "Include")}</span></label>
          <button className="material-delete-button" type="button" disabled={busy} aria-expanded={confirmingDelete} aria-label={text(`删除附件 ${material.originalName}`, `Delete attachment ${material.originalName}`)} onClick={() => setDeleteMaterialId(material.materialId)}><Icon name="trash" />{text("删除", "Delete")}</button>
        </div>
        {confirmingDelete ? <div className="material-delete-confirm" role="group" aria-label={text(`确认删除 ${material.originalName}`, `Confirm deletion of ${material.originalName}`)}><div><strong>{text("删除工作区中的附件副本？", "Delete this workspace copy?")}</strong><p>{text("原始文件不会受影响；对应投稿要求可能重新变为缺失，可随后从同一清单项重新上传。", "The original file is not affected. Its submission requirement may become missing, and you can upload a replacement from the same checklist item.")}</p></div><div><button type="button" disabled={busy} onClick={() => setDeleteMaterialId(null)}>{text("取消", "Cancel")}</button><button className="confirm-material-delete" type="button" disabled={busy} onClick={() => onDelete(material.materialId)}><Icon name="trash" />{busy ? text("正在删除…", "Deleting…") : text("确认删除附件", "Delete attachment")}</button></div></div> : null}
      </li>;
    })}</ul></section> : <div className="material-files-empty"><Icon name="folder" /><div><strong>{text("还没有已保存附件", "No stored attachments yet")}</strong><p>{text("切换到“上传资料”，按文件类型添加投稿附件。", "Switch to Upload and add submission files by type.")}</p></div><button className="secondary-button" type="button" onClick={() => setActiveView("upload")}>{text("前往上传", "Go to upload")}</button></div>}</section> : null}
    <PaneAction
      label={text("材料之后", "After materials")}
      title={!targetReady ? text("先核验目标期刊", "Verify the target journal first") : authorInputsReady ? (catalog?.targetCheckReady ? text("查看当前目标检查", "Review the current target check") : text("进入目标检查", "Continue to target checks")) : text("先补齐清单中的必需项", "Complete the required checklist first")}
      copy={text("目标检查是材料完成后的独立步骤，不属于待上传材料。进入“检查与修订”后，系统会将检查报告绑定到当前稿件版本、目标期刊和官方要求。", "Target checks are a separate step after materials, not a file to upload. In Check/revise, the report is bound to the current manuscript version, target journal, and official requirements.")}
      buttonLabel={!targetReady ? text("返回目标期刊", "Return to target journal") : authorInputsReady ? (catalog?.targetCheckReady ? text("查看检查结果", "Review check results") : text("继续检查当前稿件", "Continue to manuscript checks")) : text("完成必需材料后继续", "Continue after required materials")}
      disabled={!targetReady || !authorInputsReady}
      onClick={onContinue}
    />
  </>;
}

function packageRoleLabel(role: string, locale: Locale) {
  const labels: Record<string, [string, string]> = {
    main_manuscript: ["实名主稿", "Main manuscript"],
    blinded_manuscript: ["匿名主稿", "Blinded manuscript"],
    source_project: ["源文件工程", "Source project"],
    figure: ["原始图件", "Figure"],
    table: ["可编辑表格", "Table"],
    bibliography: ["参考文献", "Bibliography"],
    supplementary_file: ["补充材料", "Supplementary file"],
    cover_letter: ["投稿信", "Cover letter"],
    title_page: ["标题页", "Title page"],
    declaration: ["声明", "Declaration"],
    other_supporting_file: ["其他支持文件", "Other supporting file"],
  };
  return localize(locale, ...(labels[role] ?? [role, role]));
}

function TargetPackageAssembly({ plan, busy, onSetIncluded }: { plan: TargetSubmissionPackagePlan | null; busy: boolean; onSetIncluded: (materialId: string, included: boolean) => void }) {
  const { locale, text } = useI18n();
  if (!plan) return <section className="package-assembly" aria-live="polite"><header><div><span>{text("导出前组包预览", "Pre-export assembly")}</span><h3>{text("正在生成逐文件清单", "Building the file-by-file plan")}</h3></div></header></section>;
  const includedCount = plan.files.filter((file) => file.included).length;
  return <section className="package-assembly" aria-labelledby="package-assembly-heading">
    <header><div><span>{text("导出前组包预览", "Pre-export assembly")}</span><h3 id="package-assembly-heading">{plan.ready ? text("可以导出", "Ready to export") : text("仍有阻断项", "Blockers remain")}</h3></div><strong>{text(`${includedCount} 个外发文件`, `${includedCount} outgoing file(s)`)}</strong></header>
    {plan.anonymousReview ? <p className="package-anonymous-note"><Icon name="lock" />{text("期刊明确要求匿名投稿：实名源稿不会进入 submission；匿名稿作为上传主稿。", "The journal explicitly requires anonymous submission: the identified source is excluded from submission and the blinded file becomes the upload manuscript.")}</p> : null}
    <ol className="package-file-plan">{plan.files.map((file) => <li key={file.materialId ?? file.relativePath} data-included={file.included}><Icon name={file.included ? "check" : "close"} /><div><div><strong>{file.displayName}</strong><span>{packageRoleLabel(file.role, locale)} · {file.required ? text("必需", "Required") : text("可选", "Optional")}</span></div><code>{file.relativePath}</code><small>{file.checklistLabel ? text(`对应上传项：${file.checklistLabel}`, `Upload slot: ${file.checklistLabel}`) : text("系统主稿", "System manuscript") } · SHA-256 {file.contentHash.slice(0, 12)}</small>{file.validationIssues.map((issue) => <p key={issue}>{issue}</p>)}</div>{file.materialId ? <label><input type="checkbox" checked={file.included} disabled={busy} onChange={(event) => onSetIncluded(file.materialId!, event.target.checked)} /><span>{text("纳入", "Include")}</span></label> : <span className="package-fixed-file">{text("固定", "Fixed")}</span>}</li>)}</ol>
    {plan.blockers.length > 0 ? <div className="package-preflight-messages" data-tone="blocked"><strong>{text("必须处理", "Must resolve")}</strong><ul>{plan.blockers.map((message) => <li key={message}>{message}</li>)}</ul></div> : null}
    {plan.warnings.length > 0 ? <details className="package-preflight-messages"><summary>{text(`${plan.warnings.length} 项上传前提示`, `${plan.warnings.length} pre-upload note(s)`)}</summary><ul>{plan.warnings.map((message) => <li key={message}>{message}</li>)}</ul></details> : null}
  </section>;
}

function JournalMatchStage({ workspace, selectedTarget, targetPlan, requirementSnapshots, selectingTarget, targetPlanBusyId, requirementBusyId, onRecommendationGenerated, onSelectTarget, onClearPrimary, onAddBackup, onPromoteBackup, onDiscoverRequirements, onSaveManualRequirements, onContinue }: { workspace: WorkspaceSummary; selectedTarget: SubmissionTargetSelection | null; targetPlan: SubmissionTargetPlan | null; requirementSnapshots: JournalRequirementSnapshot[]; selectingTarget: boolean; targetPlanBusyId: string | null; requirementBusyId: string | null; onRecommendationGenerated: () => void; onSelectTarget: (recommendationRunId: string, journalId: string) => void; onClearPrimary: (selectionId: string) => void; onAddBackup: (recommendationRunId: string, journalId: string) => void; onPromoteBackup: (selectionId: string, reason: string) => void; onDiscoverRequirements: DiscoverOfficialSource<JournalRequirementSnapshot>; onSaveManualRequirements: (selectionId: string, sourceUrl: string, requirementText: string) => void; onContinue: () => void }) {
  const { locale, text } = useI18n();
  const [profile, setProfile] = useState<JournalRecommendationProfileInput>(() => ({ authorName: "", institution: "", specialty: "", manuscriptPurpose: "academic_communication", submissionDeadline: new Date(Date.now() + 90 * 86_400_000).toISOString().slice(0, 10) }));
  const [institutionRequirementText, setInstitutionRequirementText] = useState("");
  const [institutionSourceUrl, setInstitutionSourceUrl] = useState("");
  const [officialSourceConfirmed, setOfficialSourceConfirmed] = useState(false);
  const [preferences, setPreferences] = useState<JournalMatchPreferences>({ topic: "auto", articleType: "auto", language: "auto", targetStrategy: "balanced", openAccess: "no_preference" });
  const [run, setRun] = useState<JournalRecommendationRun | null>(null);
  const [history, setHistory] = useState<JournalRecommendationRun[]>([]);
  const [busy, setBusy] = useState(false);
  const [historyBusy, setHistoryBusy] = useState(false);
  const [directoryBusy, setDirectoryBusy] = useState(false);
  const [directorySummary, setDirectorySummary] = useState<JournalDirectorySummary | null>(null);
  const [profileDiscoveries, setProfileDiscoveries] = useState<JournalProfileDiscoveryRecord[]>([]);
  const [profileDiscoveryBusy, setProfileDiscoveryBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const minimumDeadline = new Date(Date.now() + 86_400_000).toISOString().slice(0, 10);
  const profileComplete = profile.submissionDeadline >= minimumDeadline;
  const hasInstitutionText = institutionRequirementText.trim().length > 0;
  const institutionTextReady = !hasInstitutionText || institutionRequirementText.trim().length >= 40;
  const update = <K extends keyof JournalMatchPreferences>(key: K, value: JournalMatchPreferences[K]) => { setPreferences((current) => ({ ...current, [key]: value })); setRun(null); };
  const updateProfile = <K extends keyof JournalRecommendationProfileInput>(key: K, value: JournalRecommendationProfileInput[K]) => { setProfile((current) => ({ ...current, [key]: value })); setRun(null); };
  useEffect(() => {
    if (!isTauri()) return;
    void invoke<JournalDirectorySummary>("get_journal_directory_summary")
      .then(setDirectorySummary)
      .catch(() => setDirectorySummary(null));
    void invoke<JournalProfileDiscoveryRecord[]>("get_journal_profile_discoveries", { workspaceId: workspace.id })
      .then(setProfileDiscoveries)
      .catch(() => setProfileDiscoveries([]));
    setHistoryBusy(true);
    void invoke<JournalRecommendationRun[]>("list_journal_recommendations", { workspaceId: workspace.id })
      .then((records) => {
        setHistory(records);
        setRun((current) => current ?? records[0] ?? null);
      })
      .catch((reason: unknown) => setError(localizeBackendText(locale, normalizeError(reason))))
      .finally(() => setHistoryBusy(false));
  }, [workspace.id]);
  async function importDirectory() {
    setDirectoryBusy(true); setError(null);
    try {
      const result = await invoke<JournalDirectoryImportResult | null>("import_journal_directory");
      if (result) { setDirectorySummary(result.summary); setRun(null); }
    }
    catch (reason) { setError(localizeBackendText(locale, normalizeError(reason))); }
    finally { setDirectoryBusy(false); }
  }
  async function calculate() {
    setBusy(true); setError(null);
    try {
      const savedProfile = await invoke<JournalRecommendationProfile>("save_journal_recommendation_profile", { workspaceId: workspace.id, profile });
      const effectiveProfileId = hasInstitutionText ? (await invoke<InstitutionRuleExtractionSummary>("extract_institution_requirements", { workspaceId: workspace.id, profileId: savedProfile.profileId, requirementText: institutionRequirementText, sourceUrl: institutionSourceUrl.trim() || null, authorAttestedOfficial: officialSourceConfirmed, authorConfirmedExternalTransmission: true })).profileId : savedProfile.profileId;
      const nextRun = await invoke<JournalRecommendationRun>("recommend_journals", { workspaceId: workspace.id, profileId: effectiveProfileId, preferences });
      setRun(nextRun);
      setHistory((current) => [nextRun, ...current.filter((record) => record.runId !== nextRun.runId)]);
      onRecommendationGenerated();
    }
    catch (reason) { setError(normalizeError(reason)); }
    finally { setBusy(false); }
  }
  async function discoverSelectedJournalProfile(authorConfirmedExternalTransmission: boolean) {
    if (!selectedTarget) return;
    setProfileDiscoveryBusy(true); setError(null);
    try {
      const record = await invoke<JournalProfileDiscoveryRecord>("discover_journal_profile", { workspaceId: workspace.id, targetSelectionId: selectedTarget.selectionId, authorConfirmedExternalTransmission });
      setProfileDiscoveries((current) => [record, ...current.filter((item) => item.discoveryId !== record.discoveryId)]);
    }
    catch (reason) { setError(localizeBackendText(locale, normalizeError(reason))); }
    finally { setProfileDiscoveryBusy(false); }
  }
  const topicOptions: Array<[ResearchTopic, string, string]> = [["auto","自动识别","Auto detect"],["general_ai","通用人工智能","General AI"],["machine_learning","机器学习","Machine learning"],["computer_vision","计算机视觉","Computer vision"],["natural_language_processing","自然语言处理","Natural language processing"],["data_mining","数据挖掘","Data mining"],["software_systems","软件与系统","Software & systems"],["robotics_control","机器人与控制","Robotics & control"]];
  const backupJournalIds = new Set((targetPlan?.backups ?? []).map((target) => target.journalId));
  const selectedTargetCurrent = isSubmissionTargetCurrent(workspace, selectedTarget);
  const currentRequirements = currentJournalRequirementSnapshot(selectedTarget, requirementSnapshots);
  const currentRequirementsReady = selectedTargetCurrent && journalRequirementSnapshotReady(currentRequirements);
  const currentRunReady = run?.manuscriptVersion === workspace.snapshotVersion;
  const currentProfileDiscovery = selectedTarget ? profileDiscoveries.find((record) => record.targetSelectionId === selectedTarget.selectionId) ?? null : null;
  return <>
    <PanelHeading kicker={text("主任务 2 · 初步期刊推荐", "Primary task 2 · Preliminary journal recommendation")} title={text("先用当前主稿发现适合的期刊", "Start with journals that fit the current manuscript")} copy={text("PDF、Word 或 LaTeX 主稿足以开始内容适配；学校、专业和期限用于进一步调整，不要求先上传完整投稿资料。", "A PDF, Word, or LaTeX manuscript is enough to begin content matching. Institution, specialty, and timing refine the result; a complete submission package is not required yet.")} />
    <BoundaryNote title={text("学校规则需要正式来源", "Institution rules require an official source")} copy={text("应用显示规则是否已核验及候选是否满足要求；未核验时明确标记为候选初筛。联网检索必须另行获得你的授权。", "The app shows whether rules are verified and whether a candidate meets them. Unverified results are clearly marked as provisional. Online discovery requires separate authorization.")} />
    <JournalRecommendationArchive records={history} selectedRunId={run?.runId ?? null} loading={historyBusy} locale={locale} onSelect={setRun} />
    <details className="advanced-settings journal-directory-settings"><summary>{text("期刊数据设置", "Journal data settings")} · {directorySummary?.available ? text("离线目录已同步", "Offline directory synced") : text("可选", "Optional")}</summary><section className="journal-profile" aria-labelledby="journal-directory-heading"><header><div><span>{text("本地优先 · 不执行公式或外部链接", "Local-first · formulas and external links are not executed")}</span><h3 id="journal-directory-heading">{text("离线期刊目录", "Offline journal directory")}</h3></div><strong>{directorySummary?.available ? text("已同步", "Synced") : text("未导入", "Not imported")}</strong></header>{directorySummary?.available ? <><div className="metric-row"><Metric label={text("数据记录", "Records")} value={directorySummary.recordCount} /><Metric label={text("期刊实体", "Journal entities")} value={directorySummary.distinctJournalCount} /><Metric label="ISSN" value={directorySummary.issnCount} /><Metric label="EISSN" value={directorySummary.eissnCount} /></div><div className="metric-row"><Metric label={text("出版社", "Publishers")} value={directorySummary.publisherCount} /><Metric label={text("方向范围", "Scope profiles")} value={directorySummary.scopeCount} /><Metric label={text("出版周期", "Publication cycles")} value={directorySummary.publicationCycleCount} /><Metric label={text("最新年份", "Latest year")} value={directorySummary.latestReleaseYear ?? "—"} /></div><p>{text("中科院、JCR 与新锐分区分别保存；ISSN/EISSN 用于严格归并。审稿速度、发行量等无可靠来源的值保持未知。", "CAS, JCR, and Emerging partitions remain separate; ISSN/EISSN drive strict identity matching. Review speed and circulation remain unknown when reliable evidence is unavailable.")}</p></> : <p>{text("导入本地 .xlsx 分区资料后，推荐阶段可离线核对候选期刊，不需要每次联网查询。", "Import local .xlsx partition data to verify candidates offline without a fresh web lookup each time.")}</p>}<button className="secondary-button" type="button" onClick={() => void importDirectory()} disabled={directoryBusy}>{directoryBusy ? text("正在导入…", "Importing…") : text("选择并同步 Excel", "Select and sync Excel")}</button></section></details>
    <section className="journal-profile" aria-labelledby="journal-profile-heading"><header><div><span>{text("可选调整 · 仅保存在当前论文工作区", "Optional refinement · stored only in this manuscript workspace")}</span><h3 id="journal-profile-heading">{text("投稿背景档案", "Submission context profile")}</h3></div><strong>{text("可稍后补充", "Can be added later")}</strong></header><div>
      <label htmlFor="journal-author-name">{text("投稿人姓名（可选，不参与排序）", "Author name (optional; not ranked)")}<input id="journal-author-name" autoComplete="name" maxLength={120} value={profile.authorName} onChange={(event) => updateProfile("authorName", event.target.value)} /></label>
      <label htmlFor="journal-institution">{text("学校 / 机构（可选）", "Institution (optional)")}<input id="journal-institution" autoComplete="organization" maxLength={200} value={profile.institution} onChange={(event) => updateProfile("institution", event.target.value)} /></label>
      <label htmlFor="journal-specialty">{text("学院或专业（可选）", "Faculty or specialty (optional)")}<input id="journal-specialty" maxLength={160} value={profile.specialty} onChange={(event) => updateProfile("specialty", event.target.value)} placeholder={text("留空时依据论文内容自动识别", "Leave blank to infer from manuscript content")} /></label>
      <label htmlFor="journal-purpose">{text("论文用途", "Manuscript purpose")}<select id="journal-purpose" value={profile.manuscriptPurpose} onChange={(event) => updateProfile("manuscriptPurpose", event.target.value as ManuscriptPurpose)}><option value="academic_communication">{text("一般学术发表", "Academic communication")}</option><option value="degree_requirement">{text("学位成果要求", "Degree requirement")}</option><option value="graduation">{text("毕业要求", "Graduation")}</option><option value="professional_title">{text("职称或考核", "Professional evaluation")}</option><option value="project_completion">{text("项目结题", "Project completion")}</option></select></label>
      <label htmlFor="journal-deadline">{text("完成投稿截止日期", "Submission completion deadline")}<input id="journal-deadline" type="date" min={minimumDeadline} value={profile.submissionDeadline} onChange={(event) => updateProfile("submissionDeadline", event.target.value)} /><small>{text("用于评估投稿准备余量，不预测录用、见刊或数据库收录日期。", "Used for submission-preparation planning, not acceptance, publication, or indexing forecasts.")}</small></label>
    </div></section>
    <section className="institution-requirement" aria-labelledby="institution-requirement-heading"><header><div><span>{text("可选 · 学校要求优先", "Optional · institution requirements take priority")}</span><h3 id="institution-requirement-heading">{text("提供学校正式要求", "Provide official institution requirements")}</h3></div><strong>{text("模型结构化", "Model extraction")}</strong></header><p>{text("粘贴研究生院、科研处或学院发布的投稿与成果认定原文。学校名称和专业用于限定适用范围；模型不得凭常识补写规则。", "Paste the original submission or research-output policy issued by the graduate school, research office, or faculty. Institution name and specialty constrain applicability; the model may not invent policy from prior knowledge.")}</p><label>{text("学校要求说明文字", "Institution requirement text")}<textarea maxLength={30000} value={institutionRequirementText} onChange={(event) => { setInstitutionRequirementText(event.target.value); setRun(null); }} placeholder={text("粘贴正式文件中与期刊等级、分区、毕业、职称或结题相关的原文……", "Paste official text about journal tiers, partitions, graduation, evaluation, or project completion…")} /></label><label>{text("官方来源网址（仅保存在本机）", "Official source URL (stored locally only)")}<input type="url" inputMode="url" value={institutionSourceUrl} onChange={(event) => { setInstitutionSourceUrl(event.target.value); setRun(null); }} placeholder="https://…" /></label><div className="institution-consents"><label><input type="checkbox" checked={officialSourceConfirmed} onChange={(event) => { setOfficialSourceConfirmed(event.target.checked); setRun(null); }} />{text("我确认原文来自本校正式要求", "I confirm this text comes from an official institution policy")}</label></div><small>{text("点击生成即授权这一次模型抽取。只发送学校名称、学科、论文用途和脱敏后的规则原文；作者姓名、来源网址、联系方式、学号和论文正文均不发送。", "Generating authorizes this model extraction once. Only the institution name, discipline, manuscript purpose, and redacted policy text are sent; author name, source URL, contact details, identifiers, and manuscript content are excluded.")}</small></section>
    <section className="journal-preferences" aria-labelledby="journal-preferences-heading"><h3 id="journal-preferences-heading">{text("调整推荐条件", "Adjust recommendation conditions")}</h3><div>
      <label>{text("研究方向", "Research area")}<select value={preferences.topic} onChange={(event) => update("topic", event.target.value as ResearchTopic)}>{topicOptions.map(([value,zh,en])=><option key={value} value={value}>{localize(locale,zh,en)}</option>)}</select></label>
      <label>{text("文章类型", "Article type")}<select value={preferences.articleType} onChange={(event) => update("articleType", event.target.value as ArticleTypePreference)}><option value="auto">{text("自动识别", "Auto detect")}</option><option value="research">{text("研究论文", "Research")}</option><option value="review">{text("综述", "Review")}</option><option value="application">{text("应用型", "Application")}</option></select></label>
      <label>{text("投稿语言", "Language")}<select value={preferences.language} onChange={(event) => update("language", event.target.value as PublicationLanguagePreference)}><option value="auto">{text("自动", "Auto")}</option><option value="chinese">{text("中文", "Chinese")}</option><option value="english">{text("英文", "English")}</option></select></label>
      <label>{text("目标策略", "Target strategy")}<select value={preferences.targetStrategy} onChange={(event) => update("targetStrategy", event.target.value as TargetStrategy)}><option value="reach">{text("冲刺", "Reach")}</option><option value="balanced">{text("均衡", "Balanced")}</option><option value="pragmatic">{text("务实", "Pragmatic")}</option></select></label>
      <label>{text("开放获取", "Open access")}<select value={preferences.openAccess} onChange={(event) => update("openAccess", event.target.value as OpenAccessPreference)}><option value="no_preference">{text("无偏好", "No preference")}</option><option value="prefer">{text("优先", "Prefer")}</option><option value="require">{text("必须", "Require")}</option></select></label>
    </div><button className="primary-button" type="button" disabled={busy || !profileComplete || !institutionTextReady} onClick={() => void calculate()}>{busy ? text("正在分析主稿并计算…", "Analyzing the manuscript…") : hasInstitutionText ? text("结合校规生成初步推荐", "Generate preliminary recommendations with institution rules") : text("根据当前论文生成初步推荐", "Generate preliminary recommendations from this manuscript")}<Icon name="arrow" /></button>{!profileComplete ? <p className="journal-profile-hint">{text("请选择未来的投稿准备截止日期。", "Choose a future submission-preparation deadline.")}</p> : hasInstitutionText && institutionRequirementText.trim().length < 40 ? <p className="journal-profile-hint">{text("学校要求原文至少需要 40 个字符。", "Institution requirement text must contain at least 40 characters.")}</p> : null}</section>
    {error ? <p className="inline-warning"><Icon name="warning" />{error}</p> : null}
    {run ? <>
      {!currentRunReady ? <BoundaryNote title={text("这是较早稿件版本的推荐记录", "This recommendation belongs to an earlier manuscript version")} copy={text("历史候选仅供查阅，不能直接设为当前投稿目标。请按当前版本重新计算推荐。", "Historical candidates are read-only and cannot become the current target. Recalculate recommendations for the current manuscript version.")} /> : null}
      {selectedTarget && !selectedTargetCurrent ? <BoundaryNote title={text("当前主线需要按新版本重新确认", "The primary route needs confirmation for the new version")} copy={text("可以在当前版本的新推荐中重新选择同一期刊，或按已记录原因提升一条备选支线。", "Select the same journal from a current-version recommendation, or promote a backup route with a recorded reason.")} /> : null}
      <div className="institution-rule-status" role="status" data-verified={run.schoolRuleStatus === "verified_rule_set_applied" || run.schoolRuleStatus === "verified_rule_set_applied_with_local_directory"}><strong>{run.schoolRuleStatus === "verified_rule_set_applied" || run.schoolRuleStatus === "verified_rule_set_applied_with_local_directory" ? text("学校规则已核验", "Institution rules verified") : run.schoolRuleStatus === "verified_rule_waiting_for_institution_directory_data" ? text("学校规则已提取 · 评价目录数据待核验", "Institution rules extracted · evaluation directory pending") : text("学校规则尚未核验 · 当前为候选初筛", "Institution rules unverified · provisional shortlist")}</strong><span>{text(`档案 v${run.recommendationProfile.profileVersion} · ${run.recommendationProfile.institution} · 剩余 ${run.deadlineDaysRemaining} 天`, `Profile v${run.recommendationProfile.profileVersion} · ${run.recommendationProfile.institution} · ${run.deadlineDaysRemaining} days remaining`)}</span></div>
      <div className="journal-run-meta"><span>{text("本地推荐记录", "Local recommendation run")} {run.runId}</span><span>{text("稿件版本", "Manuscript version")} v{run.manuscriptVersion}</span><span>{run.catalogVersion} · {run.catalogVerifiedDate}</span>{run.journalDirectoryVersion ? <span>{text("离线目录", "Offline directory")} {run.journalDirectoryVersion}</span> : null}</div>
      <div className="journal-region-sections"><JournalRegionPortfolio title={text("中国期刊与出版社", "China journals & publishers")} portfolio={run.domestic} locale={locale} runId={run.runId} workspaceVersion={workspace.snapshotVersion} selectionDisabled={!currentRunReady} selectedTarget={selectedTarget} backupJournalIds={backupJournalIds} selectingTarget={selectingTarget} targetPlanBusyId={targetPlanBusyId} onSelectTarget={onSelectTarget} onClearPrimary={onClearPrimary} onAddBackup={onAddBackup} /><JournalRegionPortfolio title={text("全球期刊与出版社", "Global journals & publishers")} portfolio={run.international} locale={locale} runId={run.runId} workspaceVersion={workspace.snapshotVersion} selectionDisabled={!currentRunReady} selectedTarget={selectedTarget} backupJournalIds={backupJournalIds} selectingTarget={selectingTarget} targetPlanBusyId={targetPlanBusyId} onSelectTarget={onSelectTarget} onClearPrimary={onClearPrimary} onAddBackup={onAddBackup} /></div>
      {selectedTargetCurrent && selectedTarget ? <JournalProfileDiscoveryPanel target={selectedTarget} record={currentProfileDiscovery} busy={profileDiscoveryBusy} onDiscover={discoverSelectedJournalProfile} /> : null}
      {selectedTarget || (targetPlan?.backups.length ?? 0) > 0 ? <SubmissionRoutePlan primary={selectedTarget} backups={targetPlan?.backups ?? []} snapshots={requirementSnapshots} busyTargetId={targetPlanBusyId} busyRequirementId={requirementBusyId} onClearPrimary={onClearPrimary} onPromote={onPromoteBackup} onDiscover={onDiscoverRequirements} onSaveManual={onSaveManualRequirements} /> : null}
      <PaneAction label={text("下一步", "Next")} title={!selectedTargetCurrent ? text("为当前版本确认一个投稿目标", "Confirm a target for the current version") : currentRequirementsReady ? text("按该刊要求组织投稿资料", "Organize materials for this journal") : text("先取得该刊官方投稿要求", "Capture this journal's official requirements first")} copy={!selectedTargetCurrent ? text("推荐不会自动成为投稿计划，且较早版本的目标不能沿用。", "A recommendation does not automatically become a submission plan, and a target from an earlier version cannot be reused without confirmation.") : currentRequirementsReady ? text("主线要求快照已保存在本机；下一步将据此生成资料清单。", "The primary requirement snapshot is stored locally; the next task builds the materials checklist from it.") : text("只有建立带来源且仍在有效期内的要求快照后，才会生成期刊专属清单。", "A source-backed requirement snapshot within its validity period is required before a journal-specific checklist is created.")} buttonLabel={text("按要求准备投稿资料", "Prepare required materials")} disabled={!selectedTargetCurrent || !currentRequirementsReady} onClick={onContinue} />
    </> : null}
  </>;
}

function JournalProfileDiscoveryPanel({ target, record, busy, onDiscover }: { target: SubmissionTargetSelection; record: JournalProfileDiscoveryRecord | null; busy: boolean; onDiscover: (authorConfirmedExternalTransmission: boolean) => void }) {
  const { locale, text } = useI18n();
  const fromModel = record?.sourceMode === "configured_model_candidate";
  return <section className="journal-profile-discovery" aria-labelledby="journal-profile-discovery-heading" data-candidate={fromModel}>
    <header><div><span>{text("本地优先 · 模型仅作缺失兜底", "Local first · model only for missing data")}</span><h3 id="journal-profile-discovery-heading">{text("核对目标期刊画像", "Resolve target journal profile")}</h3></div><strong>{record ? (fromModel ? text("模型线索 · 待核验", "Model lead · verify") : text("本地资料", "Local data")) : text("尚未核对", "Not checked")}</strong></header>
    <p>{text(`先查询本机数据库中的 ${target.name}；只有本地没有足够画像时，才会调用已配置的大模型 API。`, `The local database is checked for ${target.nameEn} first. A configured model API is called only when the local profile is insufficient.`)}</p>
    <div className="profile-discovery-action"><p>{text("按钮先查本地；仅在资料缺失时调用模型，并只发送期刊名、出版社、公开主页和已有 ISSN/EISSN。不发送论文、作者、学校、评分或附件。点击即授权这一次受限发现。", "The button checks locally first. Only if data is missing, the model receives the journal name, publisher, public homepage, and known ISSN/EISSN—never the manuscript, author, institution, scores, or files. Clicking authorizes this restricted discovery once.")}</p><button className="secondary-button" type="button" disabled={busy} onClick={() => onDiscover(true)}>{busy ? text("正在核对…", "Resolving…") : record ? text("重新核对本地与模型", "Check local/model again") : text("核对本地；缺失时发现", "Check local; discover if missing")}</button></div>
    {record ? <div className="profile-discovery-result"><div className="profile-discovery-tags"><span>{record.issn ? `ISSN ${record.issn}` : "ISSN —"}</span><span>{record.eissn ? `EISSN ${record.eissn}` : "EISSN —"}</span>{record.publisher ? <span>{record.publisher}</span> : null}{record.averageReviewDays ? <span>{text(`审稿约 ${record.averageReviewDays} 天`, `Review about ${record.averageReviewDays} days`)}</span> : null}{record.submissionToPublicationDays ? <span>{text(`投稿至发表 ${record.submissionToPublicationDays} 天`, `${record.submissionToPublicationDays} days submission-to-publication`)}</span> : null}{record.reportedPrintCirculation ? <span>{text(`纸刊发行量 ${record.reportedPrintCirculation}`, `Print circulation ${record.reportedPrintCirculation}`)}</span> : null}{record.publicationFrequency ? <span>{record.publicationFrequency}</span> : null}{record.apcStatus ? <span>{text(`费用：${record.apcStatus}`, `Fee: ${record.apcStatus}`)}</span> : null}{record.openAccessStatus ? <span>{`OA: ${record.openAccessStatus}`}</span> : null}{fromModel && record.providerLabel && record.model ? <span>{record.providerLabel} · {record.model}</span> : null}</div>{record.scopeSummary ? <p>{record.scopeSummary}</p> : null}{record.missingFields.length > 0 ? <small>{text(`仍未知：${record.missingFields.map((field) => journalProfileFieldLabel(field, locale)).join("、")}`, `Still unknown: ${record.missingFields.map((field) => journalProfileFieldLabel(field, locale)).join(", ")}`)}</small> : null}{fromModel ? <p className="profile-discovery-warning"><Icon name="warning" />{text("模型输出只是待核验线索，不会写成官方事实、分区证据或推荐分数；请继续读取期刊官网。", "Model output is an unverified lead. It is not treated as an official fact, ranking record, or recommendation score; continue with the journal website verification.")}</p> : <small>{text("本次未调用模型 API。", "No model API was called.")}</small>}</div> : <small>{text("尚未核对。点击后先查本地；若需要模型兜底，本次受限外发会写入本地审计记录。", "Not checked yet. Clicking checks locally first; any restricted model fallback is recorded in the local audit log.")}</small>}
  </section>;
}

function journalProfileFieldLabel(field: string, locale: Locale) {
  const labels: Record<string, [string, string]> = {
    issn: ["ISSN", "ISSN"], eissn: ["EISSN", "EISSN"], publisher: ["出版社", "publisher"], scope_summary: ["发表方向", "publication scope"], reported_print_circulation: ["纸刊发行量", "print circulation"], average_review_days: ["真实审稿速度", "review speed"], submission_to_publication_days: ["投稿至发表周期", "submission-to-publication time"], publication_frequency: ["出版频率", "publication frequency"], apc_status: ["收费/APC", "fees/APC"], open_access_status: ["OA 开放获取", "open access"]
  };
  const label = labels[field];
  return label ? localize(locale, label[0], label[1]) : field;
}

function SubmissionRoutePlan({ primary, backups, snapshots, busyTargetId, busyRequirementId, onClearPrimary, onPromote, onDiscover, onSaveManual }: { primary: SubmissionTargetSelection | null; backups: SubmissionTargetSelection[]; snapshots: JournalRequirementSnapshot[]; busyTargetId: string | null; busyRequirementId: string | null; onClearPrimary: (selectionId: string) => void; onPromote: (selectionId: string, reason: string) => void; onDiscover: DiscoverOfficialSource<JournalRequirementSnapshot>; onSaveManual: (selectionId: string, sourceUrl: string, requirementText: string) => void }) {
  const { locale, text } = useI18n();
  const [confirmingClear, setConfirmingClear] = useState(false);
  const snapshotFor = (target: SubmissionTargetSelection) => snapshots.find((snapshot) => snapshot.targetSelectionId === target.selectionId) ?? null;
  const clearing = primary !== null && busyTargetId === primary.selectionId;
  return <section className="submission-route-plan" aria-labelledby="submission-route-heading">
    <header><div><span>{text("唯一主线 · 有序备选", "One active primary · ordered backups")}</span><h2 id="submission-route-heading">{text("投稿路线", "Submission routes")}</h2></div><strong>{text(`${primary ? 1 : 0} 条主线 · ${backups.length} 条支线`, `${primary ? 1 : 0} primary · ${backups.length} backup${backups.length === 1 ? "" : "s"}`)}</strong></header>
    {primary ? <article className="submission-route-card primary-route" aria-label={text(`当前投稿主线：${primary.name}`, `Current primary route: ${primary.nameEn}`)}><div className="route-rail"><span>01</span><i /></div><div className="route-content"><div className="route-heading"><div><span>{text("当前投稿主线", "Current primary route")}</span><h3>{locale === "en" ? primary.nameEn : primary.name}</h3><p>{primary.publisher} · {articleTypeLabel(primary.articleType, locale)} · {primary.rankTier}</p></div><div className="primary-route-actions"><strong>{text("唯一激活", "Only active")}</strong><button className="text-button" type="button" disabled={clearing} aria-expanded={confirmingClear} onClick={() => setConfirmingClear(true)}>{text("取消当前主线", "Clear primary")}</button></div></div>{confirmingClear ? <div className="primary-target-clear-confirm" role="group" aria-label={text(`确认取消主线 ${primary.name}`, `Confirm clearing primary ${primary.nameEn}`)}><div><strong>{text("取消这条当前投稿主线？", "Clear this primary route?")}</strong><p>{text("只解除当前激活关系；推荐记录、期刊要求快照、附件和历史选择仍保存在本机。之后可以重新选择主线。", "Only the active link is cleared. Recommendations, requirement snapshots, attachments, and selection history stay on this device, and you can choose another primary later.")}</p></div><div><button type="button" disabled={clearing} onClick={() => setConfirmingClear(false)}>{text("返回", "Back")}</button><button className="confirm-primary-clear" type="button" disabled={clearing} onClick={() => onClearPrimary(primary.selectionId)}><Icon name="close" />{clearing ? text("正在取消…", "Clearing…") : text("确认取消主线", "Clear primary")}</button></div></div> : null}<JournalRequirementCapture target={primary} snapshot={snapshotFor(primary)} busy={busyRequirementId === primary.selectionId} onDiscover={onDiscover} onSaveManual={onSaveManual} /></div></article> : <div className="route-empty primary-route-empty"><Icon name="target" /><div><strong>{text("当前没有激活的投稿主线", "No active primary route")}</strong><p>{backups.length > 0 ? text("备选支线仍按原顺序保留。第一条作为下一主线建议，但需要你确认后才会切换。", "Backup routes remain in their original order. The first is suggested next, but it becomes primary only after your confirmation.") : text("可以从当前版本的推荐结果中重新选择一家期刊。", "Choose a journal again from the current-version recommendations.")}</p></div></div>}
    {backups.length > 0 ? <div className="backup-route-list" aria-label={text("备选投稿支线", "Backup submission routes")}>{backups.map((backup, index) => <BackupRouteCard key={backup.selectionId} target={backup} order={index + (primary ? 2 : 1)} snapshot={snapshotFor(backup)} busy={busyTargetId === backup.selectionId} requirementBusy={busyRequirementId === backup.selectionId} primaryActive={primary !== null} suggestedNext={primary === null && index === 0} onPromote={onPromote} onDiscover={onDiscover} onSaveManual={onSaveManual} />)}</div> : <div className="route-empty"><Icon name="target" /><div><strong>{text("还没有备选支线", "No backup branch yet")}</strong><p>{text("在推荐卡片中点击“加入备选支线”。支线可提前准备，但不会登记为并行投稿。", "Use “Add backup branch” on a recommendation. A branch can be prepared early but is never recorded as a parallel submission.")}</p></div></div>}
    <small className="route-policy">{primary ? text("只有主线结束、撤稿或未投稿时，才能把备选支线提升为新的主线；每次切换都会保留原因和本地历史。", "A backup can become primary only after the current route is rejected, withdrawn, or not submitted. Every transition keeps its reason and local history.") : text("备选顺序只用于提出下一主线建议，不会自动代表作者投稿；最终选择仍需作者确认。", "Backup order only suggests the next primary and never records a submission automatically; the author must confirm the final choice.")}</small>
  </section>;
}

function BackupRouteCard({ target, order, snapshot, busy, requirementBusy, primaryActive, suggestedNext, onPromote, onDiscover, onSaveManual }: { target: SubmissionTargetSelection; order: number; snapshot: JournalRequirementSnapshot | null; busy: boolean; requirementBusy: boolean; primaryActive: boolean; suggestedNext: boolean; onPromote: (selectionId: string, reason: string) => void; onDiscover: DiscoverOfficialSource<JournalRequirementSnapshot>; onSaveManual: (selectionId: string, sourceUrl: string, requirementText: string) => void }) {
  const { locale, text } = useI18n();
  const [reason, setReason] = useState("");
  return <article className="submission-route-card backup-route" aria-label={text(`备选投稿支线：${target.name}`, `Backup route: ${target.nameEn}`)} data-suggested={suggestedNext}><div className="route-rail"><span>{String(order).padStart(2,"0")}</span><i /></div><div className="route-content"><div className="route-heading"><div><span>{text("备选投稿支线", "Backup route")}</span><h3>{locale === "en" ? target.nameEn : target.name}</h3><p>{target.publisher} · {articleTypeLabel(target.articleType, locale)} · {target.rankTier}</p></div><strong>{suggestedNext ? text("建议下一主线", "Suggested next") : text("未投稿", "Not submitted")}</strong></div><JournalRequirementCapture target={target} snapshot={snapshot} busy={requirementBusy} onDiscover={onDiscover} onSaveManual={onSaveManual} compact />{primaryActive ? <div className="route-promote"><label htmlFor={`route-reason-${target.selectionId}`}>{text("提升为主线前，记录上一主线状态", "Before promotion, record the prior route status")}<select id={`route-reason-${target.selectionId}`} value={reason} onChange={(event) => setReason(event.target.value)}><option value="">{text("请选择", "Select")}</option><option value="not_submitted">{text("尚未投稿，调整目标", "Not submitted; changing target")}</option><option value="rejected">{text("已收到拒稿结果", "Rejected")}</option><option value="withdrawn">{text("已完成撤稿", "Withdrawn")}</option></select></label><button className="text-button" type="button" disabled={!reason || busy} onClick={() => onPromote(target.selectionId, reason)}>{busy ? text("正在切换…", "Switching…") : text("提升为当前主线", "Promote to primary")}</button></div> : <div className="route-promote route-promote-without-primary"><p>{suggestedNext ? text("按现有备选顺序优先建议；选择前仍应核对当前论文版本与期刊要求。", "Suggested first by the existing backup order; verify the current manuscript version and journal requirements before choosing.") : text("也可以跳过前序备选，直接把这家设为当前主线。", "You may skip earlier backups and choose this route directly.")}</p><button className="text-button" type="button" disabled={busy} onClick={() => onPromote(target.selectionId, "not_submitted")}>{busy ? text("正在切换…", "Switching…") : text("设为当前主线", "Set as primary")}</button></div>}</div></article>;
}

function officialSourceProtocol(value: string) {
  try {
    const url = new URL(value);
    if (url.username || url.password || value.trim().length > 2000) return null;
    const protocol = url.protocol;
    return protocol === "http:" || protocol === "https:" ? protocol : null;
  } catch {
    return null;
  }
}

function JournalRequirementCapture({ target, snapshot, busy, onDiscover, onSaveManual, compact = false }: { target: SubmissionTargetSelection; snapshot: JournalRequirementSnapshot | null; busy: boolean; onDiscover: DiscoverOfficialSource<JournalRequirementSnapshot>; onSaveManual: (selectionId: string, sourceUrl: string, requirementText: string) => void; compact?: boolean }) {
  const { locale, text } = useI18n();
  const [manualUrl, setManualUrl] = useState(target.homepageUrl);
  const [manualText, setManualText] = useState("");
  const [manualConfirmed, setManualConfirmed] = useState(false);
  const targetProtocol = officialSourceProtocol(target.homepageUrl);
  const automaticFetchAvailable = targetProtocol !== null;
  const manualSourceAllowed = officialSourceProtocol(manualUrl.trim()) !== null;
  const ready = Boolean(snapshot && snapshot.status !== "requires_manual_review" && snapshot.requirements.length > 0);
  const stale = Boolean(snapshot && Date.now() > snapshot.freshUntilUnixMs);
  useEffect(() => {
    setManualUrl(target.homepageUrl);
    setManualText("");
    setManualConfirmed(false);
  }, [target.homepageUrl, target.selectionId]);
  const submitManual = () => {
    onSaveManual(target.selectionId, manualUrl.trim(), manualText.trim());
    setManualConfirmed(false);
  };
  return <section className="journal-requirement-capture" data-ready={ready}>
    <div className="requirement-status"><div><Icon name={ready ? "check" : "warning"} /><div><strong>{ready ? text("已建立期刊专属要求快照", "Journal-specific snapshot ready") : snapshot ? text("已取得页面 · 仍需人工补充", "Page captured · manual input needed") : text("尚未取得官方投稿要求", "Official requirements not captured")}</strong><span>{snapshot ? text(`${snapshot.sources.length} 个来源 · ${snapshot.requirements.length} 项要求${stale ? " · · 已超过 90 天" : ""}`, `${snapshot.sources.length} source(s) · ${snapshot.requirements.length} requirement(s)${stale ? " · older than 90 days" : ""}`) : text("不会发送论文、作者身份或本地资料", "No manuscript, author identity, or local files are sent")}</span></div></div><b>{ready ? text("可用于清单", "Checklist ready") : text("待核验", "Pending")}</b></div>
    {snapshot && snapshot.requirements.length > 0 ? <details className="requirement-evidence" open={!compact}><summary>{text("查看带来源的要求", "View source-backed requirements")}</summary><ol>{snapshot.requirements.map((item) => <li key={item.id}><div><strong>{locale === "en" ? item.labelEn : item.label}</strong><span>{requirementObligationLabel(item.obligation, locale)}</span></div><p>{item.evidenceExcerpt}</p><code title={item.sourceUrl}>{item.sourceUrl}</code></li>)}</ol></details> : null}
    <OfficialSourceAccess workspaceId={target.workspaceId} selectionId={target.selectionId} homepageUrl={target.homepageUrl} busy={busy} onDiscover={(options) => onDiscover(target.selectionId, options)} />
    {snapshot?.limitations.map((limitation) => <small key={limitation}>{localizeBackendText(locale, limitation)}</small>)}
    <details className="manual-requirement-input" open={targetProtocol !== "https:" || snapshot?.status === "requires_manual_review"}><summary>{automaticFetchAvailable ? text("网页无法读取？粘贴官方原文", "Page unavailable? Paste official text") : text("粘贴官方投稿要求，继续准备", "Paste official requirements to continue")}</summary><label>{text("官方来源网址", "Official source URL")}<input type="url" inputMode="url" value={manualUrl} onChange={(event) => setManualUrl(event.target.value)} /></label><label>{text("作者指南原文", "Author-guide text")}<textarea value={manualText} maxLength={100000} onChange={(event) => setManualText(event.target.value)} placeholder={text("粘贴与格式、匿名审稿、图表、声明、费用等有关的官方原文……", "Paste official text covering format, anonymous review, figures, declarations, fees, and related requirements…")} /></label><label className="confirmation-control"><input type="checkbox" checked={manualConfirmed} onChange={(event) => setManualConfirmed(event.target.checked)} /><span>{text("我确认这段原文来自该期刊或出版社的官方作者指南", "I confirm this text is from the journal or publisher's official author guide")}</span></label>{targetProtocol === "http:" ? <small>{text("手动粘贴时，HTTP 网址只作为本地来源记录保存，不触发联网。", "When pasting text manually, the HTTP URL is stored only as local provenance and does not trigger network access.")}</small> : null}<button className="text-button" type="button" disabled={busy || !manualConfirmed || !manualSourceAllowed || manualText.trim().length < 20} onClick={submitManual}>{text("保存并生成本地快照", "Save local snapshot")}</button></details>
  </section>;
}

function requirementObligationLabel(obligation: JournalRequirementObligation, locale: Locale) {
  if (obligation === "required") return localize(locale, "明确要求", "Required");
  if (obligation === "recommended") return localize(locale, "建议或可选", "Recommended or optional");
  return localize(locale, "需作者确认", "Author verification needed");
}

function articleTypeLabel(articleType: ArticleTypePreference | undefined, locale: Locale) {
  if (articleType === "research") return localize(locale, "研究论文", "Research article");
  if (articleType === "review") return localize(locale, "综述", "Review");
  if (articleType === "application") return localize(locale, "应用型论文", "Application article");
  return localize(locale, "文章类型待确认", "Article type pending");
}

function JournalRecommendationArchive({ records, selectedRunId, loading, locale, onSelect }: { records: JournalRecommendationRun[]; selectedRunId: string | null; loading: boolean; locale: Locale; onSelect: (record: JournalRecommendationRun) => void }) {
  const { text } = useI18n();
  return <section className="journal-history" aria-labelledby="journal-history-heading"><header><div><span>{text("随论文工作区保存在本机", "Stored locally with this manuscript workspace")}</span><h3 id="journal-history-heading">{text("已保存推荐", "Saved recommendations")} · {records.length}{locale === "zh-CN" ? " 条" : ""}</h3></div><strong>{loading ? text("读取中", "Loading") : text("自动保存", "Auto-saved")}</strong></header>{records.length > 0 ? <ol>{records.map((record) => { const publishers = journalRecommendationPublishers(record); return <li key={record.runId}><button type="button" aria-pressed={record.runId === selectedRunId} aria-label={text(`查看推荐记录 ${record.runId}`, `View recommendation ${record.runId}`)} onClick={() => onSelect(record)}><span><strong>{text(`论文 v${record.manuscriptVersion}`, `Manuscript v${record.manuscriptVersion}`)} · {articleTypeLabel(record.resolvedArticleType, locale)}</strong><small>{formatModifiedDate(record.evaluatedUnixMs, locale)} · {record.runId}</small></span><span className="journal-history-publishers"><b>{text("期刊对应出版社", "Journal publishers")}</b>{publishers.join(" · ") || "—"}</span><Icon name="arrow" /></button></li>; })}</ol> : <p>{loading ? text("正在读取这篇论文的推荐档案…", "Loading recommendation records for this manuscript…") : text("首次生成后，期刊、出版社、分组和来源状态会自动保存在这里。", "After the first run, journals, publishers, groups, and source status are saved here automatically.")}</p>}</section>;
}

function journalRecommendationPublishers(run: JournalRecommendationRun) {
  const recommendations = [run.domestic, run.international].flatMap((portfolio) => [portfolio.sprint, portfolio.matching, portfolio.safeguard].flat());
  return [...new Set(recommendations.map((recommendation) => recommendation.publisher.trim()).filter(Boolean))];
}

type JournalTargetTier = "sprint" | "matching" | "safeguard";

function JournalTargetMap({ title, portfolio, locale, focusedJournalId, onFocus }: { title: string; portfolio: JournalRecommendationPortfolio; locale: Locale; focusedJournalId: string | null; onFocus: (journalId: string) => void }) {
  const { text } = useI18n();
  const tiers: Array<{ key: JournalTargetTier; code: string; radius: number; label: string; items: JournalRecommendation[] }> = [
    { key: "sprint", code: "R", radius: 13, label: text("冲刺", "Reach"), items: portfolio.sprint },
    { key: "matching", code: "M", radius: 28, label: text("匹配", "Match"), items: portfolio.matching },
    { key: "safeguard", code: "S", radius: 42, label: text("保底", "Safeguard"), items: portfolio.safeguard },
  ];
  const flat = tiers.flatMap((tier) => tier.items.map((item, index) => ({ item, tier, code: `${tier.code}${index + 1}` })));
  const points = flat.map((point, index) => {
    const angle = -90 + (index * 360) / Math.max(flat.length, 1);
    const radians = (angle * Math.PI) / 180;
    return { ...point, left: 50 + Math.cos(radians) * point.tier.radius, top: 50 + Math.sin(radians) * point.tier.radius };
  });
  return <section className="journal-target-map" aria-label={text(`${title}推荐靶图`, `${title} recommendation target map`)}>
    <div className="journal-target-plot">
      <svg viewBox="0 0 100 100" aria-hidden="true" focusable="false">
        <circle className="target-ring target-ring-safeguard" cx="50" cy="50" r="46" />
        <circle className="target-ring target-ring-matching" cx="50" cy="50" r="33" />
        <circle className="target-ring target-ring-sprint" cx="50" cy="50" r="19" />
        {Array.from({ length: 8 }, (_, index) => { const angle = ((-90 + index * 45) * Math.PI) / 180; return <line key={index} x1="50" y1="50" x2={50 + Math.cos(angle) * 46} y2={50 + Math.sin(angle) * 46} />; })}
        <text x="50" y="51">{text("冲刺", "Reach")}</text>
        <text x="50" y="23">{text("匹配环", "Match ring")}</text>
        <text x="50" y="7">{text("保底环", "Safeguard ring")}</text>
      </svg>
      {points.map(({ item, tier, code, left, top }) => <button key={item.id} className="journal-target-point" data-tier={tier.key} type="button" aria-pressed={focusedJournalId === item.id} aria-label={`${code} · ${locale === "en" ? item.nameEn : item.name} · ${locale === "en" && item.publisherEn ? item.publisherEn : item.publisher} · ${tier.label}`} style={{ left: `${left}%`, top: `${top}%` }} onClick={() => onFocus(item.id)}><span>{code}</span></button>)}
    </div>
    <div className="journal-target-legend" aria-label={text("推荐层级图例", "Recommendation tier legend")}>{tiers.map((tier) => <span key={tier.key} data-tier={tier.key}><i />{tier.label}<b>{tier.items.length}</b></span>)}</div>
    <p>{text("每个坐标对应右侧一家期刊；环层表示推荐角色，不代表录用概率或单一分区高低。", "Each coordinate maps to one journal on the right. Rings show recommendation roles, not acceptance probability or a single ranking metric.")}</p>
  </section>;
}

function JournalRegionPortfolio({ title, portfolio, locale, runId, workspaceVersion, selectionDisabled, selectedTarget, backupJournalIds, selectingTarget, targetPlanBusyId, onSelectTarget, onClearPrimary, onAddBackup }: { title: string; portfolio: JournalRecommendationPortfolio; locale: Locale; runId: string; workspaceVersion: number; selectionDisabled: boolean; selectedTarget: SubmissionTargetSelection | null; backupJournalIds: Set<string>; selectingTarget: boolean; targetPlanBusyId: string | null; onSelectTarget: (runId: string, journalId: string) => void; onClearPrimary: (selectionId: string) => void; onAddBackup: (runId: string, journalId: string) => void }) {
  const { text } = useI18n();
  const [focusedJournalId, setFocusedJournalId] = useState<string | null>(null);
  const total = portfolio.sprint.length + portfolio.matching.length + portfolio.safeguard.length;
  const shared = { locale, runId, workspaceVersion, selectionDisabled, selectedTarget, backupJournalIds, selectingTarget, targetPlanBusyId, focusedJournalId, onHighlight: setFocusedJournalId, onSelectTarget, onClearPrimary, onAddBackup };
  return <section className="journal-region-block"><header><div><span>{text("环状推荐坐标", "Radial recommendation coordinates")}</span><h2>{title} · {total}{locale === "zh-CN" ? " 家" : ""}</h2></div><strong>{text("冲刺在内 · 匹配居中 · 保底在外", "Reach inside · Match middle · Safeguard outside")}</strong></header><div className="journal-target-layout"><JournalTargetMap title={title} portfolio={portfolio} locale={locale} focusedJournalId={focusedJournalId} onFocus={setFocusedJournalId} /><section className="journal-target-details" aria-label={text(`${title}出版社与期刊资料`, `${title} publishers and journal data`)}><header><span>{text("坐标对应清单", "Coordinate-linked list")}</span><h3>{text("出版社与期刊资料", "Publishers and journal data")}</h3></header><div className="journal-columns"><JournalRecommendationList tierCode="R" title={text(`冲刺型 · ${portfolio.sprint.length} 家`, `Reach · ${portfolio.sprint.length} journals`)} description={text("目标标准较高，竞争通常更激烈。", "Higher target standards with typically stronger competition.")} items={portfolio.sprint} {...shared} /><JournalRecommendationList tierCode="M" title={text(`匹配型 · ${portfolio.matching.length} 家`, `Match · ${portfolio.matching.length} journals`)} description={text("与论文内容、专业方向、用途和可核验要求契合度较高。", "Higher fit with the manuscript, specialty, purpose, and verified requirements.")} items={portfolio.matching} {...shared} /><JournalRecommendationList tierCode="S" title={text(`保底型 · ${portfolio.safeguard.length} 家`, `Safeguard · ${portfolio.safeguard.length} journals`)} description={text("门槛与投稿准备条件相对可行；周期和录用情况仍需核验。", "Relatively feasible thresholds and preparation conditions; timelines and acceptance still require verification.")} items={portfolio.safeguard} {...shared} /></div></section></div></section>;
}

function JournalRecommendationList({ tierCode, title, description, items, locale, runId, workspaceVersion, selectionDisabled, selectedTarget, backupJournalIds, selectingTarget, targetPlanBusyId, focusedJournalId, onHighlight, onSelectTarget, onClearPrimary, onAddBackup }: { tierCode: "R" | "M" | "S"; title: string; description: string; items: JournalRecommendation[]; locale: Locale; runId: string; workspaceVersion: number; selectionDisabled: boolean; selectedTarget: SubmissionTargetSelection | null; backupJournalIds: Set<string>; selectingTarget: boolean; targetPlanBusyId: string | null; focusedJournalId: string | null; onHighlight: (journalId: string) => void; onSelectTarget: (runId: string, journalId: string) => void; onClearPrimary: (selectionId: string) => void; onAddBackup: (runId: string, journalId: string) => void }) {
  const { text } = useI18n();
  const [confirmingPrimarySelectionId, setConfirmingPrimarySelectionId] = useState<string | null>(null);
  const hasCurrentPrimary = selectedTarget?.selectedAgainstManuscriptVersion === workspaceVersion;
  useEffect(() => { setConfirmingPrimarySelectionId(null); }, [selectedTarget?.selectionId]);
  return <section className="journal-result-group"><h3>{title}</h3><p className="journal-group-description">{description}</p><ol>{items.map((item,index)=>{ const selected = hasCurrentPrimary && selectedTarget?.journalId === item.id; const staleSameTarget = !hasCurrentPrimary && selectedTarget?.journalId === item.id; const backup = backupJournalIds.has(item.id); const clearing = selected && targetPlanBusyId === selectedTarget?.selectionId; const busy = selectingTarget || targetPlanBusyId === item.id || clearing; const confirmingClear = selected && confirmingPrimarySelectionId === selectedTarget?.selectionId; return <li key={item.id} data-selected={selected} data-backup={backup} data-focused={focusedJournalId === item.id} onFocusCapture={() => onHighlight(item.id)}><header><button className="journal-coordinate-button" type="button" aria-label={text(`在靶图中定位 ${item.name}`, `Locate ${item.nameEn} on the target map`)} onClick={() => onHighlight(item.id)}>{tierCode}{index + 1}</button><div><strong>{locale === "en" ? item.nameEn : item.name}</strong><p>{locale === "en" && item.publisherEn ? item.publisherEn : item.publisher}</p></div></header><div className="journal-tags"><span>{item.rankTier}</span><span>{item.region === "domestic" ? text("中国", "China") : text("全球", "Global")}</span>{(item.directoryEvidence ?? []).map((evidence)=><span key={`${evidence.scheme}-${evidence.releaseYear}`}>{directoryEvidenceLabel(evidence, locale)}</span>)}<span>{item.deadlineStatus === "planning_window_sufficient" ? text("准备时间可安排", "Preparation window available") : text("准备时间较紧", "Tight preparation window")}</span><span>{institutionEligibilityLabel(item.institutionEligibility, locale)}</span></div><p>{text("投稿前请在期刊官网核对最新范围、费用、周期和作者指南。", "Verify the latest scope, fees, timelines, and author guidelines on the journal website before submission.")}</p><button className={selected ? "selected-target-button selected-target-clear-button" : "secondary-button"} type="button" disabled={selectionDisabled || busy} aria-expanded={selected ? confirmingClear : undefined} onClick={() => selected && selectedTarget ? setConfirmingPrimarySelectionId(selectedTarget.selectionId) : hasCurrentPrimary ? onAddBackup(runId, item.id) : onSelectTarget(runId, item.id)}>{selectionDisabled ? text("历史推荐仅供查看", "Historical recommendation") : selected ? <><Icon name="close" />{clearing ? text("正在取消主选…", "Clearing primary…") : text("取消主选期刊", "Clear primary journal")}</> : busy ? text("正在保存…", "Saving…") : backup ? text("取消备选", "Remove backup") : staleSameTarget ? text("按当前版本重新确认", "Reconfirm for this version") : hasCurrentPrimary ? text("加入备选支线", "Add backup branch") : text("设为投稿目标", "Set as target")}</button>{confirmingClear && selectedTarget ? <div className="primary-target-clear-confirm recommendation-clear-confirm" role="group" aria-label={text(`确认取消主选期刊 ${selectedTarget.name}`, `Confirm clearing primary journal ${selectedTarget.nameEn}`)}><div><strong>{text("取消这家主选期刊？", "Clear this primary journal?")}</strong><p>{text("仅解除当前主选关系；推荐记录、期刊要求、附件和历史选择仍保存在本机。", "Only the active primary link is cleared; recommendations, journal requirements, attachments, and selection history remain on this device.")}</p></div><div><button type="button" disabled={clearing} onClick={() => setConfirmingPrimarySelectionId(null)}>{text("返回", "Back")}</button><button className="confirm-primary-clear" type="button" disabled={clearing} onClick={() => onClearPrimary(selectedTarget.selectionId)}><Icon name="close" />{clearing ? text("正在取消…", "Clearing…") : text("确认取消主选", "Clear primary")}</button></div></div> : null}</li>;})}</ol></section>;
}

function institutionEligibilityLabel(status: string, locale: Locale) {
  if (status.startsWith("blocked_by_verified")) return localize(locale, "不满足已核验要求", "Does not meet verified requirements");
  if (status.startsWith("recognized_by_verified")) return localize(locale, "满足已核验要求", "Meets verified requirements");
  if (status === "requires_local_cas_directory_data") return localize(locale, "评价目录待同步", "Evaluation directory pending");
  return localize(locale, "学校要求待核验", "Institution requirements pending");
}

function directoryEvidenceLabel(evidence: JournalDirectoryEvidence, locale: Locale) {
  const scheme = evidence.scheme === "cas_partition" ? localize(locale, "中科院", "CAS") : evidence.scheme === "clarivate_jcr" ? "JCR" : localize(locale, "新锐", "Emerging");
  const partition = evidence.partition ? `${evidence.partition}${localize(locale, "区", "")}` : localize(locale, "分区缺失", "No partition");
  return `${scheme} ${evidence.releaseYear} · ${partition}${evidence.top ? " · Top" : ""}`;
}

function StructureCheckSummary({ report }: { report: StructureReport }) {
  const { locale, text } = useI18n();
  return <section className="check-structure-summary"><header><div><span>{text("结构提取完成", "Structure extracted")}</span><h3>{report.title ?? text("未检测到标题", "No title detected")}</h3></div><strong>v{report.sourceSnapshotVersion}</strong></header><div className="metric-row"><Metric label={text("作者", "Authors")} value={report.authors.length} /><Metric label={text("章节", "Sections")} value={report.sections.length} /><Metric label={text("图", "Figures")} value={report.figureCount} /><Metric label={text("表", "Tables")} value={report.tableCount} /></div>{report.warnings.map((warning) => <p className="inline-warning" key={warning}><Icon name="warning" />{localizeBackendText(locale, warning)}</p>)}</section>;
}

function knowledgeCandidates(snapshot: AcademicKnowledgeBodySnapshot) {
  const extraction = snapshot.extraction;
  return extraction ? [extraction.claim, extraction.scope, extraction.method, extraction.result, extraction.evidence].flatMap((element) => element.candidates) : [];
}

function publicationContactLabel(kind: PublicationContactKind, locale: Locale) {
  if (kind === "email") return localize(locale, "电子邮箱", "Email");
  if (kind === "orcid") return "ORCID";
  return localize(locale, "通讯信息", "Correspondence");
}

function SourceIdentityPreview({ snapshot }: { snapshot: AcademicKnowledgeBodySnapshot }) {
  const { locale, text } = useI18n();
  const identity = snapshot.sourceIdentity;
  const extracted = identity?.status === "extracted";
  return <section className="source-identity-card" aria-labelledby="source-identity-heading">
    <header><div><span>{text("身份与版本 · 源稿明示信息", "Identity & version · Source-declared metadata")}</span><h3 id="source-identity-heading">{text("作者身份与公开联系方式", "Author identity and declared contact details")}</h3></div><strong>{text("仅在本机显示", "Visible locally")}</strong></header>
    {identity ? <dl>
      <div><dt>{text("论文标题", "Manuscript title")}</dt><dd>{identity.title ?? text("未可靠识别", "Not reliably detected")}</dd></div>
      <div><dt>{text("作者", "Authors")}</dt><dd>{identity.authors.length > 0 ? <ul>{identity.authors.map((author) => <li key={author}>{author}</li>)}</ul> : text("未可靠识别", "Not reliably detected")}</dd></div>
      <div><dt>{text("作者单位", "Affiliations")}</dt><dd>{identity.affiliations.length > 0 ? <ul>{identity.affiliations.map((affiliation) => <li key={affiliation}>{affiliation}</li>)}</ul> : text("未可靠识别", "Not reliably detected")}</dd></div>
      <div><dt>{text("明示联系方式", "Declared contacts")}</dt><dd>{identity.contacts.length > 0 ? <ul>{identity.contacts.map((contact, index) => <li key={`${contact.kind}-${contact.value}-${index}`}><strong>{publicationContactLabel(contact.kind, locale)}</strong><span>{contact.value}</span><small>{contact.sourceFragmentId ? `${contact.sourceFragmentId} · ` : ""}{localizeSourceLabel(locale, contact.sourceLabel)}</small></li>)}</ul> : text("未可靠识别", "Not reliably detected")}</dd></div>
      <div><dt>{text("身份版本", "Identity version")}</dt><dd>Artifact v{identity.sourceArtifact.version} · Snapshot S{identity.version}</dd></div>
    </dl> : <p>{text("当前快照尚未完成身份信息提取。", "Identity extraction has not been completed for this snapshot.")}</p>}
    <p className="source-identity-policy"><Icon name={extracted ? "check" : "warning"} /><span>{text("这些内容来自论文源稿已明示的身份区，不进入 Claim 等语义候选的“纳入/排除”流程。本机可以直接查看；默认不会随知识体问答发送给外部模型。", "These fields come from the manuscript's declared identity area and are not subject to the Include/Exclude flow for Claim-like semantic candidates. They remain visible locally and are excluded from external model questions by default.")}</span></p>
  </section>;
}

function KnowledgeCandidatePreview({ snapshot, structureReport, compact = false, reviewable = false, decisions = {}, onDecision }: { snapshot: AcademicKnowledgeBodySnapshot; structureReport?: StructureReport; compact?: boolean; reviewable?: boolean; decisions?: Record<string, KnowledgeCandidateDecision>; onDecision?: (candidateId: string, decision: KnowledgeCandidateDecision) => void }) {
  const { locale, text } = useI18n();
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
  const confirmedCount = knowledgeCandidates(snapshot).filter((candidate) => candidate.authorConfirmed).length;
  return <section className="knowledge-candidate-preview" data-compact={compact} data-reviewable={reviewable} aria-labelledby="knowledge-candidate-heading">
    <header><div><span>{text("本地确定性语义提取", "Local deterministic semantic extraction")}</span><h3 id="knowledge-candidate-heading">{text(`${candidateCount} 条知识候选`, `${candidateCount} knowledge candidates`)}</h3></div><strong>{confirmedCount > 0 ? text(`${confirmedCount} 条已确认`, `${confirmedCount} confirmed`) : reviewable ? text("逐条审核", "Item review") : text("待作者确认", "Author confirmation pending")}</strong></header>
    <p className="knowledge-extraction-coverage">{text(`已分析 ${coverage.textFragments} 个文本片段、${coverage.tableFragments} 个表格片段和 ${coverage.figureFragments} 个图片片段。`, `Analyzed ${coverage.textFragments} text fragments, ${coverage.tableFragments} table fragments, and ${coverage.figureFragments} figure fragments.`)}</p>
    <div className="knowledge-candidate-grid">{elements.map((element) => {
      const candidates = element.value?.candidates ?? [];
      return <article key={element.key} data-state={element.value?.state ?? "pending"}><header><h4>{element.label}</h4><span>{element.value?.state === "established" ? text(`已确认 v${element.value.object.version}`, `Established v${element.value.object.version}`) : candidates.length > 0 ? text(`候选 v${element.value?.object.version ?? 0}`, `Candidate v${element.value?.object.version ?? 0}`) : text("未提取", "Not extracted")}</span></header>{candidates.length > 0 ? <ul>{candidates.slice(0, compact ? 1 : 3).map((candidate) => {
        const decision = candidate.authorConfirmed ? "included" : decisions[candidate.candidateId];
        return <li key={candidate.candidateId} data-decision={decision ?? "pending"}><p>{candidate.text}</p><small>{candidate.sourceFragmentId ? `${candidate.sourceFragmentId} · ` : ""}{localizeSourceLabel(locale, candidate.sourceLabel)} · {candidate.modality.toUpperCase()} · {candidate.confidencePercent}%</small>{candidate.authorConfirmed ? <span className="candidate-confirmed-badge"><Icon name="check" />{text("作者已确认", "Author confirmed")}</span> : reviewable && onDecision ? <div className="candidate-decision" role="group" aria-label={text(`审核 ${element.label} 候选`, `Review ${element.label} candidate`)}><button type="button" aria-pressed={decision === "included"} onClick={() => onDecision(candidate.candidateId, "included")}><Icon name="check" />{text("纳入知识体", "Include")}</button><button type="button" aria-pressed={decision === "excluded"} onClick={() => onDecision(candidate.candidateId, "excluded")}><Icon name="close" />{text("排除", "Exclude")}</button></div> : null}</li>;
      })}</ul> : <p>{text("当前可解析内容中没有足够明确的语义片段；不会用占位文字冒充知识。", "No sufficiently explicit semantic passage was found in the parseable content; placeholders are not presented as knowledge.")}</p>}</article>;
    })}</div>
    <p className="knowledge-candidate-policy">{text("候选对象可以支持带不确定性说明的问答；只有作者选择纳入并完成整组审核后，状态才会从 candidate 升级为 established。", "Candidates can support uncertainty-aware questions and answers. They become established only after the author includes them and completes the full review.")}</p>
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
  return <><p className="workspace-created-status"><Icon name="check" />{text("个人知识体快照已固化", "Personal knowledge-body snapshot finalized")}</p><PanelHeading kicker={`${text("高级功能 · 个人知识体快照", "Advanced feature · Personal knowledge-body snapshot")} · S${snapshot.snapshotVersion}`} title={text("知识体与关联网络", "Knowledge body and relationship network")} copy={text("快照已绑定身份与版本、语义知识、存证、投稿记录和作者确认的学科分类；当前仍只保存在本机。", "The snapshot binds identity and version, semantic knowledge, attestation, submission, and author-confirmed discipline and remains local.")} /><KnowledgeSpatialMap workspace={workspace} knowledgeBodySnapshot={snapshot} /><KnowledgeDialoguePanel workspace={workspace} knowledgeBodyRecord={record} /><SourceIdentityPreview snapshot={snapshot} /><KnowledgeCandidatePreview snapshot={snapshot} structureReport={structureReport} compact /><section className="knowledge-identity-card" aria-labelledby="knowledge-identity-heading"><header><div><span>{text("稳定身份 · 不可变快照", "Stable identity · Immutable snapshot")}</span><h3 id="knowledge-identity-heading">{text("知识体哈希与学科索引", "Knowledge-body hash and discipline index")}</h3></div><strong>SHA-256</strong></header><dl><div><dt>{text("知识体哈希编码", "Knowledge-body hash")}</dt><dd><code>{record.recordHash}</code></dd></div><div><dt>{text("学科索引分类", "Discipline classification")}</dt><dd><strong>{locale === "en" ? classification.labelEn : classification.label}</strong><span>{classification.code}</span></dd></div><div><dt>{text("分类协议", "Classification protocol")}</dt><dd>ClassificationAssignment · v{classification.version}</dd></div><div><dt>{text("索引体系", "Index scheme")}</dt><dd>{classification.scheme} · v{classification.schemeVersion}</dd></div><div><dt>{text("确认状态", "Confirmation status")}</dt><dd>{text("身份与版本已保留；学科分类和纳入的语义候选已确认", "Identity and version preserved; discipline and included semantic candidates confirmed")}</dd></div><div><dt>KnowledgeBody ID</dt><dd>{snapshot.knowledgeBodyId}</dd></div><div><dt>{text("固化时间", "Finalized")}</dt><dd>{formatModifiedDate(record.finalizedUnixMs, locale)}</dd></div></dl><p>{text("该哈希覆盖源稿明示身份、知识体快照、学科分类、存证与投稿引用；身份长期稳定，内容更新形成新快照。信誉状态可独立变化，不会改写历史内容。", "This hash covers source-declared identity, the knowledge snapshot, discipline classification, attestation, and submission references. Identity remains stable while content updates create new snapshots. Reputation may evolve independently without rewriting historical content.")}</p></section><ul className="knowledge-layers" aria-label={text("知识体五部分架构", "Five-part knowledge-body architecture")}><KnowledgeLayer title={text(`身份与版本 · Artifact v${objects.artifactVersion.version} · Snapshot S${objects.knowledgeBodySnapshot.version}`, `Identity & version · Artifact v${objects.artifactVersion.version} · Snapshot S${objects.knowledgeBodySnapshot.version}`)} copy={text("稳定 KnowledgeBody 身份、源稿作者与联系方式、不可变版本和替代/撤回状态", "Stable KnowledgeBody identity, source-declared authors and contacts, immutable versions, and supersession or withdrawal state")} complete /><KnowledgeLayer title={text(`知识、边界与证据 · Claim v${objects.claim.version}`, `Knowledge, boundary & evidence · Claim v${objects.claim.version}`)} copy={text(`Scope v${objects.scope.version} · Method v${objects.method.version} · Result v${objects.result.version} · EvidenceRelation v${objects.evidenceRelation.version} · SourceAnchor v${objects.sourceAnchor.version}`, `Scope v${objects.scope.version} · Method v${objects.method.version} · Result v${objects.result.version} · EvidenceRelation v${objects.evidenceRelation.version} · SourceAnchor v${objects.sourceAnchor.version}`)} complete={objects.scope.version > 0 && objects.method.version > 0 && objects.result.version > 0} /><KnowledgeLayer title={text(`能力契约 · ${capabilityCount} 项`, `Capability contracts · ${capabilityCount}`)} copy={text(`${availableCapabilities} 项可用或需要运行时；明确输入、输出、前置条件、拒绝条件与证据追溯`, `${availableCapabilities} available or runtime-dependent; inputs, outputs, preconditions, refusal conditions, and evidence traceability are explicit`)} complete={capabilityCount > 0} /><KnowledgeLayer title={text("交互与执行运行时 · RuntimeProfile v1", "Interaction & execution runtime · RuntimeProfile v1")} copy={text("作者配置的模型只作为可替换协调层；每次外发单独授权", "Author-configured models are replaceable coordinators; every transmission requires per-call authorization")} complete={architecture !== undefined} /><KnowledgeLayer title={text(`验证、权利与信誉 · Reputation v${reputationVersion}`, `Validation, rights & reputation · Reputation v${reputationVersion}`)} copy={text(`AIReviewReport ${aiReview ? `v${aiReview.version}` : "v0"} · RightsPolicy v1；信誉独立于固定内容持续更新`, `AIReviewReport ${aiReview ? `v${aiReview.version}` : "v0"} · RightsPolicy v1; reputation evolves independently without rewriting historical content`)} complete={architecture !== undefined} /></ul></>;
}

function VersionManager({ workspace, history, selectedVersion, candidate, note, notice, selecting, saving, restoring, onSelectCandidate, onNoteChange, onSave, onSelectVersion, onRestore, onContinue, continueReady }: { workspace: WorkspaceSummary; history: VersionHistory | null; selectedVersion: number | null; candidate: ManuscriptSummary | null; note: string; notice: string | null; selecting: boolean; saving: boolean; restoring: boolean; onSelectCandidate: () => void; onNoteChange: (note: string) => void; onSave: () => void; onSelectVersion: (version: number) => void; onRestore: (version: number) => void; onContinue: () => void; continueReady: boolean }) {
  const { locale, text } = useI18n();
  const currentVersion = history?.currentVersion ?? workspace.snapshotVersion;
  const selected = selectedVersion ?? currentVersion;
  const formatMatches = !candidate || candidate.kind === workspace.manuscript.kind;
  const versions = history ? [...history.versions].reverse() : [];
  return <>
    <PanelHeading kicker={text("高级记录 · 版本历史", "Advanced record · Version history")} title={text("查看和管理不可变论文版本", "View and manage immutable manuscript versions")} copy={text("主投稿流程不会经过本页。只有需要比较、导入修改稿或安全恢复旧稿时才在这里操作。", "The primary submission flow does not pass through this page. Use it only to compare, import, or safely restore revisions.")} />
    <section className="version-confirm-card" aria-labelledby="version-confirm-heading">
      <div><span>{text("当前待投稿版本", "Current submission version")}</span><h3 id="version-confirm-heading">v{currentVersion} · {workspace.manuscript.name}</h3><p>{workspace.manuscript.extension.toUpperCase()} · {formatBytes(workspace.manuscript.sizeBytes, locale)} · {continueReady ? text("当前检查结果有效", "Current check is valid") : text("需要重新检查", "A new check is required")}</p></div>
      <button className="primary-button" type="button" onClick={onContinue}>{continueReady ? text("返回投稿包", "Return to package") : text("检查当前版本", "Check this version")}<Icon name="arrow" /></button>
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
    <details className="version-revision-option" open={candidate !== null}>
      <summary>{text("需要改用另一份修改稿？", "Need to use another revision?")}</summary>
      <p>{text(`仅在当前 v${currentVersion} 不是最终稿时使用。新文件必须保持 ${workspace.manuscript.extension.toUpperCase()} 格式，保存后系统会重新检查。`, `Use this only if v${currentVersion} is not final. The new file must remain ${workspace.manuscript.extension.toUpperCase()}, and the app will recheck it after saving.`)}</p>
      <section className="version-save-card" aria-labelledby="version-save-heading">
        <div className="version-save-heading"><div><span>{text("可选修改稿", "Optional revision")}</span><h3 id="version-save-heading">{candidate ? candidate.name : text("尚未选择文件", "No file selected")}</h3>{candidate ? <p>{candidate.extension.toUpperCase()} · {formatBytes(candidate.sizeBytes, locale)}</p> : null}</div><button className="text-button" type="button" onClick={onSelectCandidate} disabled={selecting || saving}>{selecting ? text("正在打开…", "Opening…") : candidate ? text("重新选择", "Choose another") : text("选择修改稿", "Choose revision")}</button></div>
        {candidate ? <div className="version-note-field"><label htmlFor="version-note">{text("版本说明", "Version note")} <span>{text("可选", "Optional")}</span></label><input id="version-note" value={note} maxLength={200} onChange={(event) => onNoteChange(event.target.value)} placeholder={text("例如：补充方法与统计分析", "For example: expanded methods and statistical analysis")} /><small>{note.length} / 200</small></div> : null}
        {!formatMatches ? <p className="inline-warning" role="alert"><Icon name="warning" />{text("修改稿必须与当前稿件保持相同文件类型；格式转换应留在投稿输出中。", "The revision must use the same file type as the current manuscript; format conversion belongs in submission outputs.")}</p> : null}
        {candidate ? <button className="secondary-button version-primary" type="button" onClick={onSave} disabled={selecting || saving || !formatMatches}>{saving ? text("正在保存并复查…", "Saving and rechecking…") : text(`保存为 v${currentVersion + 1} 并复查`, `Save as v${currentVersion + 1} and recheck`)}<Icon name="check" /></button> : null}
      </section>
    </details>
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
    <PanelHeading kicker={text("主任务 4 · 检查规则", "Primary task 4 · Check rules")} title={text("选择适用于这篇论文的标准", "Choose standards applicable to this manuscript")} copy={text("通用初投稿规则始终启用。只选择真实适用的国家标准、出版商和研究类型；具体期刊作者指南仍具有最高优先级。", "General initial-submission rules are always active. Select only applicable national, publisher, and study-type standards; the journal's own author instructions still take precedence.")} />
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
  if (loading || !catalog) return <EmptyStage icon="format" kicker={text("主任务 4 · 投稿优化修订台", "Primary task 4 · Submission Revision Desk")} title={text("正在整理投稿要素", "Preparing submission elements")} copy={text("正在本机组合已签名的出版社要求，不会调用 AI 或发送论文。", "Combining signed publisher requirements locally without AI calls or manuscript transmission.")} />;
  const groups = ["identity", "manuscript", "declarations", "files"];
  const editableCount = catalog.elements.filter((element) => element.editableField).length;
  const changedCount = draft?.fields.filter((field) => (values[field.field] ?? field.value).trim() !== field.value).length ?? 0;
  return <>
    <PanelHeading kicker={text("主任务 4 · 投稿优化修订台", "Primary task 4 · Submission Revision Desk")} title={selectedPublisherCount > 0 ? text("依据检查结果修订", "Revise from check findings") : text("通用投稿修订", "General submission revision")} copy={selectedPublisherCount > 0 ? text("出版社要素已合并；保存后会形成新版本并自动重做当前规则检查。", "Publisher elements are merged; saving creates a new version and automatically reruns the current checks.") : text("当前使用通用规则；可安全回写的字段仍可修订，具体期刊要求需作者继续核对。", "General rules are active; safe fields remain editable while journal-specific requirements still require author review.")} />
    {result ? <p className="revision-saved" role="status"><Icon name="check" />{text(`已保存为 v${result.outputVersion}，${result.changes.length} 项修改已记录来源`, `Saved as v${result.outputVersion}; provenance recorded for ${result.changes.length} change(s)`)}</p> : null}
    {draft && draft.fields.length > 0 ? <section className="revision-fields" aria-labelledby="revision-fields-heading"><header><div><span>{text(`基础版本 v${draft.baseVersion}`, `Base version v${draft.baseVersion}`)}</span><h3 id="revision-fields-heading">{text("可安全回写的字段", "Fields safe to write back")}</h3></div><strong>{draft.format.toUpperCase()}</strong></header>{draft.fields.map((field) => <div className="revision-field" key={field.field}><label htmlFor={`revision-${field.field}`}>{locale === "en" ? field.labelEn : field.label}</label>{field.field === "title" ? <input id={`revision-${field.field}`} value={values[field.field] ?? field.value} onChange={(event) => onValueChange(field.field, event.target.value)} disabled={!field.editable || saving} /> : <textarea id={`revision-${field.field}`} rows={field.field === "abstract" ? 5 : 2} value={values[field.field] ?? field.value} onChange={(event) => onValueChange(field.field, event.target.value)} disabled={!field.editable || saving} />}<small>{field.limitation ? (locale === "en" ? field.limitationEn : field.limitation) : text("作者修改 · 本机处理 · 保存前可在右侧核对差异", "Author edit · Local processing · Review the difference on the right before saving")}</small></div>)}</section> : null}
    {draft?.warnings.map((warning) => <p className="inline-warning" key={warning}><Icon name="warning" />{localizeBackendText(locale, warning)}</p>)}
    {catalog.elements.length > 0 ? <div className="submission-element-groups" aria-label={text("出版社投稿要素", "Publisher submission elements")}>{groups.map((group) => {
      const elements = catalog.elements.filter((element) => element.group === group);
      if (elements.length === 0) return null;
      return <section className="submission-element-group" key={group}><header><h3>{submissionElementGroupLabel(group, locale)}</h3><span>{elements.length}</span></header><ul>{elements.map((element) => <li key={element.id}><span className="element-state"><Icon name={element.editableField ? "format" : "check"} /></span><div><strong>{locale === "en" ? element.labelEn : element.label}</strong><p>{locale === "en" ? element.descriptionEn : element.description}</p><small>{element.editableField ? text("可进入结构化修订", "Structured revision available") : text("作者核对", "Author confirmation")}</small></div></li>)}</ul></section>;
    })}</div> : <div className="submission-elements-empty"><Icon name="target" /><p>{text("当前组合没有出版社级投稿要素；通用检查仍然可用。", "The current composition has no publisher-level elements; general checks remain available.")}</p></div>}
    <BoundaryNote title={text("可信边界", "Trust boundary")} copy={text(`共 ${catalog.elements.length} 项，其中 ${editableCount} 项已连接到后续结构化修订字段。所有来源在右侧只读显示。`, `${catalog.elements.length} elements are listed; ${editableCount} connect to structured revision fields. Every source is shown read-only on the right.`)} />
    <PaneAction label={changedCount > 0 ? text(`${changedCount} 项待保存`, `${changedCount} change(s) pending`) : text("下一步", "Next")} title={changedCount > 0 ? text(`保存为新版本 v${(draft?.baseVersion ?? 0) + 1}`, `Save as new version v${(draft?.baseVersion ?? 0) + 1}`) : text("当前版本可以进入投稿包", "The current version can proceed to packaging")} copy={changedCount > 0 ? text("保存后自动重提取和复查；新版本需要重新确认期刊目标与官方要求，原稿与历史不会被覆盖。", "Saving automatically extracts and rechecks. The new version must reconfirm its journal target and official requirements; the source and history remain unchanged.") : text("没有待保存修改；版本历史仍可从高级记录单独查看。", "There are no unsaved changes; version history remains available separately under advanced records.")} buttonLabel={changedCount > 0 ? (saving ? text("保存并复查中…", "Saving and rechecking…") : text("保存新版本并重新确认目标", "Save and reconfirm target")) : text("进入投稿包", "Continue to package")} disabled={saving} onClick={changedCount > 0 ? onSave : onContinue} />
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

function EvidencePane({ stage, workspace, structureReport, readinessReport, knowledgeBodySnapshot = null, knowledgeBodyRecord = null, attestation = null, submission = null, submissionExport = null, submissionMaterials = null, submissionTargetSelection = null, submissionTargetPlan = null, journalRequirementSnapshots = [], targetSubmissionExport = null, ruleCatalog = [], selectedRulePackIds = [], submissionElementCatalog = null, revisionDraft = null, revisionValues = {}, revisionResult = null, versionHistory = null, selectedVersion = null, versionComparison = null, isComparingVersions = false }: PaneProps) {
  const { locale, text } = useI18n();
  if (stage === "source") return <EvidenceFrame kicker={text("只读版本证据", "Read-only version evidence")} title={text("当前稿件身份", "Current manuscript identity")}><div className="document-sheet source-sheet"><span className="document-type">{workspace.manuscript.extension.toUpperCase()}</span><p className="document-title">{workspace.manuscript.name}</p><dl><div><dt>{text("内容指纹", "Content fingerprint")}</dt><dd>{workspace.contentHash}</dd></div><div><dt>{text("当前版本", "Current version")}</dt><dd>v{workspace.snapshotVersion}</dd></div><div><dt>{text("状态", "Status")}</dt><dd>{text("不可变；历史不会被覆盖", "Immutable; history is never overwritten")}</dd></div></dl></div></EvidenceFrame>;
  if (stage === "materials") return <EvidenceFrame kicker={text("本地资料目录", "Local materials catalog")} title={text("出版社文件与内部记录分离", "Publisher files separated from internal records")}><div className="document-sheet source-sheet"><span className="document-type">MATERIALS</span><p className="document-title">{submissionMaterials?.materials.length ?? 0} {text("个附加文件", "supporting files")}</p><dl><div><dt>{text("必需项", "Required items")}</dt><dd>{submissionMaterials?.requiredComplete ? text("已齐", "Complete") : text("待补", "Incomplete")}</dd></div><div><dt>{text("目标期刊", "Target journal")}</dt><dd>{submissionTargetSelection ? (locale === "en" ? submissionTargetSelection.nameEn : submissionTargetSelection.name) : text("未选择", "Not selected")}</dd></div><div><dt>{text("外部传输", "External transmission")}</dt><dd>{text("未发生", "None")}</dd></div></dl><small>{text("文件已复制到当前论文的本地资料目录，源文件不会被移动。", "Files are copied into this manuscript's local materials directory; originals are not moved.")}</small></div></EvidenceFrame>;
  if (stage === "versions") return <VersionEvidence workspace={workspace} history={versionHistory} selectedVersion={selectedVersion} comparison={versionComparison} comparing={isComparingVersions} />;
  if (stage === "journals") {
    const currentRequirements = submissionTargetSelection ? journalRequirementSnapshots.find((snapshot) => snapshot.targetSelectionId === submissionTargetSelection.selectionId) ?? null : null;
    return <EvidenceFrame kicker={text("候选与来源状态", "Shortlist and source status")} title={text("本地期刊推荐", "Local journal recommendations")}><div className="document-sheet source-sheet"><span className="document-type">JOURNAL ROUTES</span><p className="document-title">{submissionTargetSelection ? (locale === "en" ? submissionTargetSelection.nameEn : submissionTargetSelection.name) : "ManuscriptDock"}</p><dl><div><dt>{text("候选输出", "Shortlist output")}</dt><dd>{text("国内、国外各最多 8 家 · 不跨层级强行补足", "Up to 8 domestic and 8 international journals · no cross-tier quota filling")}</dd></div><div><dt>{text("投稿路线", "Submission routes")}</dt><dd>{submissionTargetSelection ? text(`1 条主线 · ${submissionTargetPlan?.backups.length ?? 0} 条备选支线`, `1 primary · ${submissionTargetPlan?.backups.length ?? 0} backup route(s)`) : text("尚未选择主线", "No primary selected")}</dd></div><div><dt>{text("主线官方要求", "Primary official requirements")}</dt><dd>{currentRequirements ? text(`${currentRequirements.sources.length} 个来源 · ${currentRequirements.requirements.length} 项要求`, `${currentRequirements.sources.length} source(s) · ${currentRequirements.requirements.length} requirement(s)`) : text("尚未取得", "Not captured")}</dd></div><div><dt>{text("外部传输", "External transmission")}</dt><dd>{text("推荐不联网；官网读取与模型抽取均需逐次确认", "Recommendations stay offline; site fetches and model extraction require per-call consent")}</dd></div></dl>{currentRequirements?.sources.length ? <ul className="provenance-list">{currentRequirements.sources.map((source) => <li key={source.contentHash}><span><Icon name={source.officialHostMatched ? "check" : "warning"} /></span><div><strong>{source.title}</strong><p>{source.url}</p><code>{source.contentHash}</code></div></li>)}</ul> : null}<small>{text("要求快照保留来源、时间与内容指纹；自动抽取不替代投稿前人工核对。", "Requirement snapshots retain source, time, and fingerprint; extraction does not replace final author verification.")}</small></div></EvidenceFrame>;
  }
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
  if (stage === "submission") return <EvidenceFrame kicker={text("投稿证据", "Submission evidence")} title={submission ? text("已登记投稿", "Submission recorded") : text("出版社投稿包", "Publisher submission package")}>{submission ? <LifecycleEvidence id={submission.submissionId} hash={submission.recordHash} items={[[text("目标", "Target"), submission.target], [text("回执", "Receipt"), submission.receipt ?? text("未填写", "Not provided")], [text("自动存证", "Automatic attestation"), submission.attestationId]]} /> : <div className="package-preview"><span>MANUSCRIPTDOCK</span><h2>{submissionTargetSelection ? (locale === "en" ? submissionTargetSelection.nameEn : submissionTargetSelection.name) : text("尚未选择目标", "No target selected")}</h2><p>{text("作者控制的投稿资料交付", "Author-controlled publisher handoff")}</p><dl><div><dt>{text("稿件版本", "Manuscript version")}</dt><dd>v{workspace.snapshotVersion}</dd></div><div><dt>{text("附加资料", "Supporting files")}</dt><dd>{submissionMaterials?.materials.length ?? 0}</dd></div><div><dt>{text("导出状态", "Export status")}</dt><dd>{targetSubmissionExport ? targetSubmissionExport.packageName : text("尚未导出", "Not exported")}</dd></div></dl><small>{text("submission 供出版社；records 仅供本地留档。导出不等于已投稿。", "submission is publisher-facing; records is local-only. Export does not mean submitted.")}</small></div>}</EvidenceFrame>;
  if (stage === "knowledge") return <EvidenceFrame kicker={text("知识体依据", "Knowledge-body evidence")} title={knowledgeBodyRecord ? text("已固化对象与来源", "Finalized objects and sources") : text("候选对象与来源", "Candidate objects and sources")}><div className="document-sheet source-sheet"><span className="document-type">KNOWLEDGE BODY</span><p className="document-title">{knowledgeBodySnapshot?.knowledgeBodyId ?? workspace.manuscript.name}</p><dl><div><dt>{text("快照", "Snapshot")}</dt><dd>S{knowledgeBodySnapshot?.snapshotVersion ?? workspace.snapshotVersion}</dd></div><div><dt>{text("状态", "Status")}</dt><dd>{knowledgeBodyRecord ? text("作者确认并固化", "Author-confirmed and finalized") : text("本地提取候选", "Locally extracted candidates")}</dd></div><div><dt>{text("记录指纹", "Record fingerprint")}</dt><dd>{(knowledgeBodyRecord?.recordHash ?? workspace.contentHash).slice(0, 16)}</dd></div><div><dt>{text("外部传输", "External transmission")}</dt><dd>{text("默认未发生；AI 问答逐次授权", "None by default; AI questions require per-call consent")}</dd></div></dl><small>{text("动态图、节点交互和 AI 问答已放回知识体主界面；这里仅保留可审计依据。", "The dynamic map, node interaction, and AI dialogue are available in the main knowledge-body view; this pane retains auditable evidence only.")}</small></div></EvidenceFrame>;
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
function Metric({ label, value }: { label: string; value: ReactNode }) { return <div><span>{label}</span><strong>{value}</strong></div>; }
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

function conciseKnowledgeText(value: string, limit = 128) {
  const normalized = value.replace(/\s+/g, " ").trim();
  return normalized.length > limit ? `${normalized.slice(0, limit).trimEnd()}…` : normalized;
}

function KnowledgePointCloud({ density }: { density: number }) {
  const svgRef = useRef<SVGSVGElement | null>(null);
  const pointCount = Math.max(28, Math.min(52, 28 + density * 4));

  useEffect(() => {
    const svg = svgRef.current;
    if (!svg) return;
    const points = Array.from(svg.querySelectorAll<SVGCircleElement>("circle"));
    const motionPreference = window.matchMedia?.("(prefers-reduced-motion: reduce)");
    const seeds = points.map((_, index) => {
      const longitude = index * 2.399963229728653;
      const latitude = Math.asin(-1 + (2 * (index + 0.5)) / points.length);
      return {
        x: Math.cos(latitude) * Math.cos(longitude),
        y: Math.sin(latitude),
        z: Math.cos(latitude) * Math.sin(longitude),
      };
    });
    let frameId: number | null = null;

    const draw = (angle: number) => {
      const cosY = Math.cos(angle);
      const sinY = Math.sin(angle);
      const tilt = -0.26 + Math.sin(angle * 0.7) * 0.05;
      const cosX = Math.cos(tilt);
      const sinX = Math.sin(tilt);
      seeds.forEach((seed, index) => {
        const rotatedX = seed.x * cosY + seed.z * sinY;
        const rotatedZ = -seed.x * sinY + seed.z * cosY;
        const rotatedY = seed.y * cosX - rotatedZ * sinX;
        const depth = seed.y * sinX + rotatedZ * cosX;
        const perspective = 3.8 / (3.8 - depth);
        const point = points[index];
        point.setAttribute("cx", (300 + rotatedX * 176 * perspective).toFixed(2));
        point.setAttribute("cy", (230 + rotatedY * 176 * perspective).toFixed(2));
        point.setAttribute("r", (1.45 + (depth + 1) * 0.9).toFixed(2));
        point.style.opacity = String(Math.max(0.16, Math.min(0.72, 0.38 + depth * 0.24)));
      });
    };

    const start = () => {
      if (frameId !== null) window.cancelAnimationFrame(frameId);
      if (motionPreference?.matches || typeof window.requestAnimationFrame !== "function") {
        draw(0.7);
        frameId = null;
        return;
      }
      const startedAt = window.performance.now();
      let lastDrawAt = startedAt;
      const animate = (time: number) => {
        if (time - lastDrawAt >= 40) {
          draw(((time - startedAt) / 36000) * Math.PI * 2);
          lastDrawAt = time;
        }
        frameId = window.requestAnimationFrame(animate);
      };
      draw(0);
      frameId = window.requestAnimationFrame(animate);
    };

    start();
    motionPreference?.addEventListener?.("change", start);
    return () => {
      if (frameId !== null) window.cancelAnimationFrame(frameId);
      motionPreference?.removeEventListener?.("change", start);
    };
  }, [pointCount]);

  return <svg ref={svgRef} className="knowledge-point-cloud" viewBox="0 0 600 460" preserveAspectRatio="none" aria-hidden="true">{Array.from({ length: pointCount }, (_, index) => <circle key={index} cx="300" cy="230" r="2" />)}</svg>;
}

function KnowledgeSpatialMap({ workspace, knowledgeBodySnapshot = null }: { workspace: WorkspaceSummary; knowledgeBodySnapshot?: AcademicKnowledgeBodySnapshot | null }) {
  const { locale, text } = useI18n();
  const [view, setView] = useState<KnowledgeView>("single");
  const [selectedLayerKey, setSelectedLayerKey] = useState("knowledge");
  const network = knowledgeBodySnapshot?.network;
  const bodyCount = network?.bodies.length ?? 1;
  const availableView = view === "pair" ? bodyCount >= 2 : view === "network" ? bodyCount >= 3 : true;
  const claim = knowledgeBodySnapshot?.claim;
  const objects = knowledgeBodySnapshot?.objects;
  const aiReview = knowledgeBodySnapshot?.aiReviewReport;
  const architecture = knowledgeBodySnapshot?.serviceArchitecture;
  const extraction = knowledgeBodySnapshot?.extraction;
  const sourceIdentity = knowledgeBodySnapshot?.sourceIdentity;
  const semanticElements: Array<{ key: SemanticElementKind; label: string; element?: ExtractedKnowledgeElement }> = [
    { key: "claim", label: "Claim", element: extraction?.claim },
    { key: "scope", label: "Scope", element: extraction?.scope },
    { key: "method", label: "Method", element: extraction?.method },
    { key: "result", label: "Result", element: extraction?.result },
    { key: "evidence", label: "Evidence", element: extraction?.evidence },
  ];
  const summaries = semanticElements.map((item) => {
    const candidate = item.element?.candidates.find((entry) => entry.authorConfirmed) ?? item.element?.candidates[0];
    return { ...item, candidate, summary: candidate ? conciseKnowledgeText(candidate.text) : text("当前分解未形成可靠内容", "No reliable content was formed by the current decomposition") };
  });
  const claimSummary = summaries.find((item) => item.key === "claim");
  const previousReviewVersions = (knowledgeBodySnapshot?.aiReviewHistory.versions ?? []).filter((report) => report.version !== aiReview?.version).map((report) => `v${report.version}`).join(" · ");
  const capabilityContracts = architecture?.capabilityContracts ?? [];
  const layers = [
    { key: "identity", label: text("身份与版本", "Identity & version"), version: `S${objects?.knowledgeBodySnapshot.version ?? knowledgeBodySnapshot?.snapshotVersion ?? 1}`, state: sourceIdentity?.status === "extracted" ? text(`${sourceIdentity.authors.length} 位作者 · ${sourceIdentity.contacts.length} 项联系方式 · Artifact v${sourceIdentity.sourceArtifact.version}`, `${sourceIdentity.authors.length} authors · ${sourceIdentity.contacts.length} contacts · Artifact v${sourceIdentity.sourceArtifact.version}`) : text(`Artifact v${objects?.artifactVersion.version ?? workspace.snapshotVersion} · 身份待识别`, `Artifact v${objects?.artifactVersion.version ?? workspace.snapshotVersion} · Identity pending`), complete: sourceIdentity?.status === "extracted" },
    { key: "knowledge", label: text("知识、边界与证据", "Knowledge, boundary & evidence"), version: `Claim v${claim?.claim.version ?? 1}`, state: claimSummary?.summary ?? text("等待论文分解", "Awaiting decomposition"), complete: claimSummary?.element?.state === "established" },
    { key: "capability", label: text("能力契约", "Capability contracts"), version: `v1 · ${capabilityContracts.length}`, state: text("输入 · 输出 · 前置 · 拒绝", "Input · Output · Preconditions · Refusal"), complete: capabilityContracts.length > 0 },
    { key: "runtime", label: text("交互与执行运行时", "Interaction & runtime"), version: "RuntimeProfile · v1", state: text("可替换模型 · 单次授权", "Replaceable model · Per-call consent"), complete: architecture !== undefined },
    { key: "trust", label: text("验证、权利与信誉", "Validation, rights & reputation"), version: `Reputation · v${architecture?.validationRightsAndReputation.reputationRecord.version ?? 0}`, state: aiReview ? text(`AIReview v${aiReview.version}${previousReviewVersions ? ` · 历史 ${previousReviewVersions}` : ""}`, `AIReview v${aiReview.version}${previousReviewVersions ? ` · History ${previousReviewVersions}` : ""}`) : text("Rights v1 · 审核待建立", "Rights v1 · Review pending"), complete: architecture !== undefined },
  ];
  const selectedLayer = layers.find((layer) => layer.key === selectedLayerKey) ?? layers[1];
  const establishedPointCount = semanticElements.reduce((count, item) => count + (item.element?.candidates.length ?? 0), 0);
  return (
    <section className="knowledge-space" aria-label={text("动态知识点云与关联网络", "Dynamic knowledge point cloud and relationship network")}>
      <div className="knowledge-view-switch" role="tablist" aria-label={text("知识体网络层级", "Knowledge-network level")}>
        {(["single", "pair", "network"] as KnowledgeView[]).map((item, index) => {
          const enabled = item === "single" || (item === "pair" ? bodyCount >= 2 : bodyCount >= 3);
          const labels = [text("1. 单一知识体", "1. One body"), text("2. 两体关联", "2. Two bodies"), text("3. 关联网络", "3. Network")];
          return <button key={item} type="button" role="tab" aria-selected={view === item} disabled={!enabled} title={enabled ? labels[index] : text("建立足够的声明关系后可用", "Available after enough asserted relationships exist")} onClick={() => setView(item)}>{labels[index]}</button>;
        })}
      </div>
      {view === "single" ? (
        <div className="knowledge-space-visual knowledge-service-space" aria-label={text(`单篇论文动态知识点云。稳定 KnowledgeBody 身份包含不可变内容快照 S${objects?.knowledgeBodySnapshot.version ?? knowledgeBodySnapshot?.snapshotVersion ?? 1}，中心是 Claim v${claim?.claim.version ?? 1} 十二面体；可选择五类节点查看详情。`, `Dynamic single-paper knowledge point cloud. A stable KnowledgeBody identity contains immutable content snapshot S${objects?.knowledgeBodySnapshot.version ?? knowledgeBodySnapshot?.snapshotVersion ?? 1}, centered on a Claim v${claim?.claim.version ?? 1} dodecahedron; select any of five nodes for details.`)}>
          <span className="knowledge-snapshot-label" aria-hidden="true">KnowledgeBody · {text("稳定身份", "stable identity")} · Snapshot S{objects?.knowledgeBodySnapshot.version ?? knowledgeBodySnapshot?.snapshotVersion ?? 1}</span>
          <span className="knowledge-content-boundary" aria-hidden="true">{text("固定内容快照", "Immutable content snapshot")}</span>
          <KnowledgePointCloud density={establishedPointCount} />
          <svg className="claim-connections" viewBox="0 0 600 460" preserveAspectRatio="none" aria-hidden="true">
            <line x1="300" y1="230" x2="300" y2="68" />
            <line x1="300" y1="230" x2="112" y2="160" />
            <line x1="300" y1="230" x2="488" y2="160" />
            <line x1="300" y1="230" x2="438" y2="372" className="runtime-connection" />
            <line x1="300" y1="230" x2="162" y2="372" className="reputation-connection" />
          </svg>
          <div className="claim-center" aria-hidden="true">
            <span className="claim-knowledge-summary">{claimSummary?.summary}</span>
            <ClaimDodecahedron />
            <span className="claim-core"><strong>Claim · v{claim?.claim.version ?? 1}</strong><small>{claimSummary?.element?.state === "established" ? text("作者已确认", "Author confirmed") : text("提取候选 · 待确认", "Extracted candidate · Review pending")}</small></span>
          </div>
          {layers.map((layer) => <button type="button" className={`service-layer-node service-layer-${layer.key}`} data-complete={layer.complete} data-selected={selectedLayer.key === layer.key} key={layer.key} aria-pressed={selectedLayer.key === layer.key} onClick={() => setSelectedLayerKey(layer.key)}><span className="service-layer-sphere"><strong>{layer.label}</strong><small>{layer.version}</small><em title={layer.state}>{layer.state}</em></span></button>)}
        </div>
      ) : availableView && network ? <KnowledgeNetworkCanvas bodies={view === "pair" ? network.bodies.slice(0, 2) : network.bodies} assertions={network.assertions} view={view} /> : null}
      {view === "single" ? <div className="knowledge-layer-inspector" role="status" aria-live="polite"><span>{text("当前节点", "Selected node")}</span><strong>{selectedLayer.label} · {selectedLayer.version}</strong><p>{selectedLayer.state}</p></div> : null}
      {view === "single" ? <section className="knowledge-summary-legend" aria-labelledby="knowledge-summary-heading"><header><div><span>{text("当前论文的解构知识", "Decomposed knowledge from this manuscript")}</span><h3 id="knowledge-summary-heading">{text("知识摘要与来源", "Knowledge summaries and sources")}</h3></div><strong>{text("逐项可追溯", "Traceable by item")}</strong></header><ol>{summaries.map((item) => <li key={item.key} data-state={item.element?.state ?? "pending"}><div className="knowledge-summary-label"><strong>{item.label}</strong><span>{item.element?.state === "established" ? text("作者已确认", "Confirmed") : item.candidate ? text("提取候选", "Candidate") : text("未建立", "Pending")}</span></div><p>{item.summary}</p>{item.candidate ? <small>{item.candidate.sourceLabel}{item.candidate.sourceFragmentId ? ` · ${item.candidate.sourceFragmentId}` : ""} · {item.candidate.confidencePercent}%</small> : null}</li>)}</ol></section> : null}
      <p className="knowledge-space-note">{view === "single" ? text("单一知识体是具有稳定身份、明确知识边界、可验证证据、能力契约和可替换运行时的知识服务单元。内容快照不可变；信誉记录可独立更新；v0 表示尚未正式建立。", "A single KnowledgeBody is a knowledge-service unit with stable identity, explicit boundaries, verifiable evidence, capability contracts, and a replaceable runtime. Content snapshots are immutable; reputation records evolve independently; v0 means not yet established.") : text("圆形边界表示知识体自身边界；绿色菱形表示带依据、状态和版本的声明对象。相似度不会自动成为关系。", "Circular boundaries preserve each knowledge body; green diamonds are versioned assertions with basis and status. Similarity never becomes a relationship automatically.")}</p>
    </section>
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
  const [notice, setNotice] = useState<LocalizedCopy | null>(null);
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

  useEffect(() => {
    const refreshFromGlobalSettings = (event: Event) => {
      const summary = (event as CustomEvent<ModelSettingsSummary>).detail;
      if (summary) applySettings(summary);
    };
    window.addEventListener("manuscriptdock:model-settings-updated", refreshFromGlobalSettings);
    return () => window.removeEventListener("manuscriptdock:model-settings-updated", refreshFromGlobalSettings);
  }, []);

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
      window.dispatchEvent(new CustomEvent<ModelSettingsSummary>("manuscriptdock:model-settings-updated", { detail: summary }));
      setNotice(localizedCopy("模型设置已保存；API Key 仅保存在系统凭据库。", "Model settings saved; API keys remain only in the system credential store."));
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
      <div className="model-settings-actions"><p>{text("应用界面不会读取或回显明文 Key。同一次应用运行中，每个已保存 Key 只向系统凭据库读取一次；作者点击“询问知识体”即授权该次调用。DeepSeek V4 问答会关闭思考模式，把输出额度保留给最终回答。", "The interface never reads or reveals plaintext keys. During one app session, each stored key is read from the credential store only once; clicking “Ask knowledge body” authorizes that call. DeepSeek V4 requests disable thinking mode so the output allowance remains available for the final answer.")}</p><button className="primary-button" type="button" disabled={isSavingSettings || invalidEnabledDraft} onClick={() => void saveSettings()}>{isSavingSettings ? text("保存中…", "Saving…") : invalidEnabledDraft ? text("请先补全启用项", "Complete enabled slots") : text("保存模型设置", "Save model settings")}</button></div>
    </section> : null}

    {error ? <p className="dialogue-message dialogue-error" role="alert">{localizeBackendText(locale, error)}</p> : null}
    {notice ? <p className="dialogue-message" role="status">{localize(locale, notice.zhCN, notice.en)}</p> : null}

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
    {item.answers.length > 0 ? item.answers.map((answer) => <article className="answer-card" key={answer.answerId}><header><strong>{answer.providerLabel} · {answer.model}</strong><span>{modelSlotLabel(answer.modelSlot as ModelSlotRole, locale)}</span></header><p>{answer.answer}</p><footer><span>{text(`${answer.sourceAnchors.length} 个来源锚点`, `${answer.sourceAnchors.length} ${answer.sourceAnchors.length === 1 ? "source anchor" : "source anchors"}`)}</span><code>{answer.recordHash.slice(0, 12)}</code></footer></article>) : <p className="answer-pending">{text("问题已保存在本机；模型尚未形成回答。", "The question is saved locally; no model answer is available yet.")}</p>}
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
