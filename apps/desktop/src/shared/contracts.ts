import type { LocalizedText } from "../i18n";
export type TaskKind = "find_journals" | "prepare_package";
export interface SelectedInput {
  token: string;
  name: string;
  extension: string;
  sizeBytes: number;
  kind: string;
}
export interface SelectedFolder {
  token: string;
  name: string;
}
export type SelectionResponse =
  { status: "selected"; items: SelectedInput[]; folder?: SelectedFolder } | {
    status: "cancelled";
  };
export interface AppError {
  code: string;
  params?: Record<string, string>;
  retryable: boolean;
  recoveryAction?: string;
  diagnosticId: string;
}
export type JobStatus =
  | "queued"
  | "running"
  | "succeeded"
  | "needs_input"
  | "failed"
  | "cancelled"
  | "interrupted";
export interface JobRecord {
  schemaVersion: number;
  id: string;
  projectId?: string;
  requestId: string;
  operation: string;
  contextHash?: string;
  status: JobStatus;
  cancelRequested: boolean;
  events: Array<{
    seq: number;
    phase: string;
    status: JobStatus;
    createdAtUnixMs: number;
    completedUnits?: number;
    totalUnits?: number;
    error?: AppError;
  }>;
  result?: unknown;
  updatedAtUnixMs: number;
}
export interface DocumentFacts {
  sourceHash: string;
  extractorVersion: string;
  title: string | null;
  abstractText: string | null;
  keywords: string[];
  language: string | null;
  articleType: string | null;
  authors: string[];
  affiliations: string[];
  correspondingEmail: string | null;
  conflictOfInterest: string | null;
  funding: string | null;
  dataAvailability: string | null;
  ethicsStatement: string | null;
  highlights: string[];
  creditContributions: string | null;
  generativeAiDisclosure: string | null;
  authorConfirmedFields: string[];
}
export interface TargetSelection {
  id: string;
  journalId: string;
  articleType: string;
  stage: string;
  origin: "catalog" | "recommendation" | "generic";
  recommendationRef?: string;
  rulesHash: string;
}
export interface Project {
  id: string;
  schemaVersion: number;
  revision: number;
  displayName: string;
  activeSource: {
    id: string;
    fileName: string;
    sha256: string;
    format: string;
    sizeBytes: number;
    createdAtUnixMs: number;
    featureProfile: string;
  };
  facts: DocumentFacts;
  materials: Array<{
    id: string;
    fileName: string;
    sha256: string;
    sizeBytes: number;
    kind: string;
    included: boolean;
    anonymityCheck?: {
      identityContextHash: string;
      detectedCategories: string[];
    };
  }>;
  target: TargetSelection | null;
  authorDecisions: Array<{
    requirementId: string;
    value: string;
    evidenceRef: string;
    contextHash: string;
    confirmedAtUnixMs: number;
  }>;
  workspace?: {
    kind: "folder";
    name: string;
    bindingId: string;
  };
  lastTask: TaskKind;
  updatedAtUnixMs: number;
}
export interface JournalRecord {
  id: string;
  displayName: string;
  publisher: string;
  issn: string | null;
  eissn: string | null;
  aliases: string[];
  languages: string[];
  articleTypes: string[];
  topics: string[];
  indexing: string[];
  publicationRoute: string;
  apcAmount: number | null;
  apcCurrency: string | null;
  firstDecisionDays: number | null;
  generationCoverage: string;
  status: string;
  verifiedAt: string;
  evidence: Array<{
    sourceUrl: string;
    label: LocalizedText;
    verifiedAt: string;
  }>;
}
export interface AuthorConstraints {
  requiredLanguage: string | null;
  articleType: string | null;
  topicKeywords: string[];
  requiredIndexing: string[];
  maximumApc: number | null;
  currency: string | null;
  requireOpenAccess: boolean;
  preferFastFirstDecision: boolean;
}
export interface Recommendation {
  journal: JournalRecord;
  role: "best_overall_fit" | "ambitious_option" | "less_preparation_needed";
  reasons: LocalizedText[];
  risks: LocalizedText[];
  preparation: LocalizedText[];
  constraints: Array<{
    constraintId: string;
    status: "pass" | "fail" | "unknown";
    explanation: LocalizedText;
  }>;
  evidenceCoverage: number;
}
export interface RecommendationResult {
  runId: string;
  catalogVersion: string;
  recommendations: Recommendation[];
  needsVerification: JournalRecord[];
  excludedCount: number;
}
export interface PreparationItem {
  requirementId: string;
  label: LocalizedText;
  description: LocalizedText;
  action: string;
  status: string;
  required: boolean;
  evidence: {
    sourceUrl: string;
    label: LocalizedText;
    verifiedAt: string;
  };
}
export interface PreparationView {
  projectId: string;
  revision: number;
  contextHash: string;
  status: "needs_input" | "draft_ready" | "ready_to_export";
  blockers: PreparationItem[];
  warnings: PreparationItem[];
  readyItems: PreparationItem[];
  allowedActions: string[];
  packagePlan: {
    id: string;
    contextHash: string;
    filePlan: Array<{
      relativePath: string;
      operation: string;
      publisherFile: boolean;
    }>;
    transformPlan: string[];
    capabilities: string[];
    status: string;
  };
}
export interface CompiledPackage {
  id: string;
  projectId: string;
  contextHash: string;
  mode: "draft" | "final";
  files: Array<{
    id: string;
    relativePath: string;
    purpose: LocalizedText;
    sha256: string;
    sizeBytes: number;
    publisherFile: boolean;
  }>;
  validationPassed: boolean;
  warnings: LocalizedText[];
}
export interface ExportReceipt {
  id: string;
  requestId?: string;
  packageId: string;
  outputDirectory: string;
  fileCount: number;
  packageHash: string;
  finishedAtUnixMs: number;
  recordPersisted: boolean;
}
