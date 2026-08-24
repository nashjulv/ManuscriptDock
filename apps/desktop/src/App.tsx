import { invoke, isTauri } from "@tauri-apps/api/core";
import type { CSSProperties, ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
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
  warnings: string[];
}

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

type KnowledgeObjectType = "knowledge_body" | "knowledge_body_snapshot" | "claim" | "proposition" | "scope" | "evidence" | "evidence_relation" | "source_anchor" | "status" | "method" | "result" | "artifact_version" | "ai_review_report" | "provenance";
interface VersionedObjectReference { objectId: string; objectType: KnowledgeObjectType; version: number; }
interface ClaimElementReference extends VersionedObjectReference { state: "pending" | "established"; }
interface ClaimFiveTuple { claim: VersionedObjectReference; proposition: ClaimElementReference; conditions: ClaimElementReference; evidence: ClaimElementReference; sources: ClaimElementReference; status: ClaimElementReference; }
interface KnowledgeBodyObjectSet { artifactVersion: VersionedObjectReference; claim: VersionedObjectReference; scope: VersionedObjectReference; method: VersionedObjectReference; result: VersionedObjectReference; evidenceRelation: VersionedObjectReference; sourceAnchor: VersionedObjectReference; aiReviewReport: VersionedObjectReference | null; provenance: VersionedObjectReference; knowledgeBodySnapshot: VersionedObjectReference; }
interface AiReviewReportVersion { reportId: string; version: number; previousVersion: number | null; reviewedClaim: VersionedObjectReference; reviewerId: string; reviewerVersion: string; createdUnixMs: number; status: string; summary: string; externalTransmission: string; }
interface AiReviewReportHistory { reportId: string; currentVersion: number | null; versions: AiReviewReportVersion[]; }
type KnowledgeBodyRole = "current_study" | "original_research" | "reproduction_research" | "competing_research" | "cross_domain_application" | "later_synthesis";
interface KnowledgeBodyNode { body: VersionedObjectReference; displayId: string; title: string; role: KnowledgeBodyRole; claim: VersionedObjectReference; sourceAnchor: VersionedObjectReference; method: VersionedObjectReference; }
type RelationKind = "citation" | "claim_relation" | "evidence_relation" | "method_transfer" | "reproduction" | "alignment" | "version_relation" | "classification";
interface NetworkAssertion { assertionId: string; version: number; relationKind: RelationKind; protocolObject: string; source: VersionedObjectReference; target: VersionedObjectReference; basis: Array<{ label: string; source: VersionedObjectReference }>; status: string; }
interface AcademicKnowledgeBodySnapshot { schemaVersion: number; knowledgeBodyId: string; snapshotVersion: number; manuscript: VersionedObjectReference; claim: ClaimFiveTuple; objects: KnowledgeBodyObjectSet; aiReviewReport: VersionedObjectReference | null; aiReviewHistory: AiReviewReportHistory; network: { bodies: KnowledgeBodyNode[]; assertions: NetworkAssertion[]; supportedRelations: RelationKind[] }; externalTransmission: string; }

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

type SelectionState = "idle" | "selecting" | "selected" | "error";
type WorkspaceStage = "source" | "versions" | "structure" | "target" | "format" | "review" | "package" | "knowledge";
type MobilePane = "operation" | "evidence";
type IconName = "workspace" | "upload" | "lock" | "file" | "check" | "versions" | "structure" | "target" | "format" | "review" | "package" | "knowledge" | "arrow" | "warning";

const WORKSPACE_STAGES: Array<{ id: WorkspaceStage; zh: string; en: string; shortZh: string; shortEn: string }> = [
  { id: "source", zh: "原稿", en: "Source", shortZh: "原稿", shortEn: "Source" },
  { id: "versions", zh: "版本", en: "Versions", shortZh: "版本", shortEn: "Versions" },
  { id: "structure", zh: "结构", en: "Structure", shortZh: "结构", shortEn: "Structure" },
  { id: "target", zh: "目标", en: "Target", shortZh: "目标", shortEn: "Target" },
  { id: "format", zh: "修订", en: "Revision", shortZh: "修订", shortEn: "Revision" },
  { id: "review", zh: "检查", en: "Checks", shortZh: "检查", shortEn: "Checks" },
  { id: "package", zh: "投稿包", en: "Package", shortZh: "包", shortEn: "Package" },
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
  if (stage === "versions") return "versions";
  if (stage === "structure") return "structure";
  if (stage === "target") return "target";
  if (stage === "format") return "format";
  if (stage === "review") return "review";
  if (stage === "package") return "package";
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
  const [catalogWarnings, setCatalogWarnings] = useState<string[]>([]);
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
  const [isLoadingKnowledgeBody, setIsLoadingKnowledgeBody] = useState(false);
  const [activeStage, setActiveStage] = useState<WorkspaceStage>("source");
  const [mobilePane, setMobilePane] = useState<MobilePane>("operation");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;
    void invoke<WorkspaceCatalog>("list_workspaces")
      .then((catalog) => { setRecentWorkspaces(catalog.workspaces); setCatalogWarnings(catalog.warnings); })
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
    setIsLoadingKnowledgeBody(false);
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
        resetKnowledgeBodyState();
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
        resetKnowledgeBodyState();
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
          resetKnowledgeBodyState();
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
          resetKnowledgeBodyState();
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
          setActiveStage("structure");
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
          setActiveStage("review");
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
    resetKnowledgeBodyState();
    setActiveStage("source");
    setMobilePane("operation");
    setErrorMessage(null);
    setSelectionState("idle");
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
    if (stage === "target" && ruleCatalog.length === 0 && !isLoadingRuleCatalog) {
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
    if (stage === "format" && !isLoadingSubmissionElements) {
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
    if (stage === "knowledge" && activeWorkspace && !isLoadingKnowledgeBody && knowledgeBodySnapshot?.manuscript.version !== activeWorkspace.snapshotVersion) {
      setIsLoadingKnowledgeBody(true);
      setErrorMessage(null);
      void invoke<AcademicKnowledgeBodySnapshot>("get_knowledge_body_snapshot", { workspaceId: activeWorkspace.id })
        .then(setKnowledgeBodySnapshot)
        .catch((error: unknown) => setErrorMessage(normalizeError(error)))
        .finally(() => setIsLoadingKnowledgeBody(false));
    }
  }

  function toggleRulePack(rulePackId: string) {
    setSelectedRulePackIds((current) => current.includes(rulePackId)
      ? current.filter((id) => id !== rulePackId)
      : [...current, rulePackId]);
    setSubmissionElementCatalog(null);
    setReadinessReport(null);
  }

  function applyRevision() {
    if (!activeWorkspace || !revisionDraft || isApplyingRevision) return;
    const changes = revisionDraft.fields.filter((field) => (revisionValues[field.field] ?? field.value).trim() !== field.value).map((field) => ({ field: field.field, after: revisionValues[field.field] ?? field.value }));
    if (changes.length === 0) return;
    setIsApplyingRevision(true); setErrorMessage(null);
    void invoke<RevisionApplication>("apply_manuscript_revision", { workspaceId: activeWorkspace.id, baseVersion: revisionDraft.baseVersion, changes })
      .then((result) => {
        if (result.status === "unchanged") { setVersionNotice(localizeBackendText(locale, result.message)); return; }
        setActiveWorkspace(result.workspace);
        setRecentWorkspaces((current) => [result.workspace, ...current.filter((workspace) => workspace.id !== result.workspace.id)]);
        setRevisionResult(result.revisionSet); setStructureReport(null); setReadinessReport(null); setVersionHistory(null); resetKnowledgeBodyState();
        return invoke<RevisionDraft>("get_revision_draft", { workspaceId: result.workspace.id }).then((draft) => { setRevisionDraft(draft); setRevisionValues(Object.fromEntries(draft.fields.map((field) => [field.field, field.value]))); });
      })
      .catch((error: unknown) => setErrorMessage(normalizeError(error)))
      .finally(() => setIsApplyingRevision(false));
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
            {(["versions", "structure", "target", "format", "review", "knowledge"] as WorkspaceStage[]).map((stage) => { const item = WORKSPACE_STAGES.find((candidate) => candidate.id === stage); return <button key={stage} className="rail-button" type="button" aria-label={item ? localize(locale, item.zh, item.en) : undefined} title={text("创建工作区后可用", "Available after creating a workspace")} disabled><Icon name={getStageIcon(stage)} /></button>; })}
          </nav>

          <main id="main-content" className="landing-main">
            <header className="landing-workspace-head"><h1 id="page-title">{text("我的工作台", "My Workspace")}</h1></header>
            <div className="landing-content">
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
                <article className="track-card"><span>02</span><h2>{text("传输可见", "Transfers stay visible")}</h2><p>{text("联网、模型调用和外发均在执行前说明对象与范围。", "Network, model, and outbound actions disclose their destination and scope before execution.")}</p></article>
              </section>
              {recentWorkspaces.length > 0 || catalogWarnings.length > 0 ? <RecentWorkspaces workspaces={recentWorkspaces} warnings={catalogWarnings.map((warning) => localizeBackendText(locale, warning))} onOpen={openRecentWorkspace} /> : null}
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
            const isComplete = stage.id === "source" || stage.id === "versions" || (stage.id === "structure" && structureReport !== null) || (stage.id === "review" && readinessReport !== null) || (stage.id === "package" && readinessReport !== null);
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
              <OperationPane stage={activeStage} workspace={activeWorkspace} structureReport={structureReport} readinessReport={readinessReport} knowledgeBodySnapshot={knowledgeBodySnapshot} ruleCatalog={ruleCatalog} selectedRulePackIds={selectedRulePackIds} submissionElementCatalog={submissionElementCatalog} revisionDraft={revisionDraft} revisionValues={revisionValues} revisionResult={revisionResult} versionHistory={versionHistory} selectedVersion={selectedVersion} versionCandidate={versionCandidate} versionNote={versionNote} versionNotice={versionNotice} isLoadingRuleCatalog={isLoadingRuleCatalog} isLoadingSubmissionElements={isLoadingSubmissionElements} isLoadingKnowledgeBody={isLoadingKnowledgeBody} isApplyingRevision={isApplyingRevision} isAnalyzing={isAnalyzing} isEvaluating={isEvaluating} isSelectingVersion={isSelectingVersion} isSavingVersion={isSavingVersion} isRestoringVersion={isRestoringVersion} onAnalyze={analyzeWorkspace} onEvaluate={evaluateReadiness} onToggleRulePack={toggleRulePack} onOpenStage={openStage} onRevisionValueChange={(field, value) => setRevisionValues((current) => ({ ...current, [field]: value }))} onApplyRevision={applyRevision} onSelectVersionCandidate={selectVersionCandidate} onVersionNoteChange={setVersionNote} onSaveVersion={saveVersion} onSelectVersion={(version) => compareVersions(activeWorkspace, version, activeWorkspace.snapshotVersion)} onRestoreVersion={restoreVersion} />
            </section>
            <aside id="evidence-pane" className="evidence-pane" role="tabpanel" aria-label={`${currentStageLabel} ${text("证据", "Evidence")}`}><EvidencePane stage={activeStage} workspace={activeWorkspace} structureReport={structureReport} readinessReport={readinessReport} knowledgeBodySnapshot={knowledgeBodySnapshot} ruleCatalog={ruleCatalog} selectedRulePackIds={selectedRulePackIds} submissionElementCatalog={submissionElementCatalog} revisionDraft={revisionDraft} revisionValues={revisionValues} revisionResult={revisionResult} versionHistory={versionHistory} selectedVersion={selectedVersion} versionComparison={versionComparison} isComparingVersions={isComparingVersions} /></aside>
          </div>
          {errorMessage ? <ErrorNotice message={localizeBackendText(locale, errorMessage)} onRetry={activeStage === "review" ? evaluateReadiness : activeStage === "target" ? () => openStage("target") : activeStage === "versions" ? () => loadVersionHistory(activeWorkspace) : activeStage === "knowledge" ? () => openStage("knowledge") : analyzeWorkspace} /> : null}
        </main>
      </div>
      <LiveStatus selecting={isSelecting} analyzing={isAnalyzing} evaluating={isEvaluating} />
    </div>
  );
}

function ProductBar({ manuscriptName, onNewManuscript, isSelecting = false }: { manuscriptName?: string; onNewManuscript?: () => void; isSelecting?: boolean }) {
  const { locale, setLocale, text } = useI18n();
  return <header className="product-bar"><div className="brand" aria-label="ManuscriptDock"><span className="brand-mark" aria-hidden="true">M</span><span className="brand-name">ManuscriptDock</span><span className="brand-cn">{text("投稿舱", "Submission Dock")}</span></div>{manuscriptName ? <p className="current-manuscript" title={manuscriptName}>{manuscriptName}</p> : <span />}<div className="bar-actions"><div className="language-switch" role="group" aria-label={text("界面语言", "Interface language")}><button type="button" aria-pressed={locale === "zh-CN"} onClick={() => setLocale("zh-CN")}>中文</button><button type="button" aria-pressed={locale === "en"} onClick={() => setLocale("en")}>EN</button></div><span className="local-badge" title={text("稿件尚未离开你的设备", "The manuscript has not left your device")}><Icon name="lock" />{text("仅在本机", "Local only")}</span>{onNewManuscript ? <button className="bar-button" type="button" onClick={onNewManuscript} disabled={isSelecting}>{isSelecting ? text("正在打开…", "Opening…") : text("导入另一篇", "Import another")}</button> : null}</div></header>;
}

function SubmissionGuide() {
  const { text } = useI18n();
  const items = [
    ["1", text("导入原稿", "Import source"), text("选择 DOCX、PDF 或 TEX，建立本地只读快照", "Choose DOCX, PDF, or TEX and create a local read-only snapshot")],
    ["2", text("结构提取", "Extract structure"), text("确定性识别标题、章节、图表和投稿声明", "Deterministically identify the title, sections, figures, tables, and declarations")],
    ["3", text("目标与格式", "Target and format"), text("组合期刊规则，并生成不覆盖原稿的新版本", "Compose journal rules and create a new version without overwriting the source")],
    ["4", text("投稿检查", "Submission checks"), text("校验规则来源和完整性，逐条查看依据与待确认事项", "Verify rule sources and integrity, then review evidence and confirmations item by item")],
    ["5", text("知识体积累", "Build the knowledge body"), text("把原稿、结构、证据和版本写回同一知识对象", "Write the source, structure, evidence, and versions back to one knowledge object")],
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

function RecentWorkspaces({ workspaces, warnings, onOpen }: { workspaces: WorkspaceSummary[]; warnings: string[]; onOpen: (workspace: WorkspaceSummary) => void }) {
  const { locale, text } = useI18n();
  return <section className="recent-section" aria-labelledby="recent-heading"><div className="section-heading"><div><p className="field-kicker">{text("设备上的记录", "Records on this device")}</p><h2 id="recent-heading">{text("最近工作区", "Recent workspaces")}</h2></div><span>{locale === "zh-CN" ? `${workspaces.length} 个` : workspaces.length}</span></div>{workspaces.length > 0 ? <ul className="recent-list">{workspaces.slice(0, 5).map((workspace) => <li key={workspace.id}><button type="button" onClick={() => onOpen(workspace)}><span className="recent-file-icon" aria-hidden="true"><Icon name="file" /></span><span className="recent-file-copy"><strong>{workspace.manuscript.name}</strong><span>{text("源快照", "Source snapshot")} v{workspace.snapshotVersion} · {formatModifiedDate(workspace.importedUnixMs, locale)}</span></span><code>{workspace.contentHash.slice(0, 8)}</code><Icon name="arrow" /></button></li>)}</ul> : null}{warnings.map((warning) => <p className="catalog-warning" key={warning}>{warning}</p>)}</section>;
}

function getStageDescription(stage: WorkspaceStage, locale: Locale) {
  const descriptions: Record<WorkspaceStage, string> = {
    source: localize(locale, "确认本地只读源快照及其数据边界。", "Confirm the local read-only source snapshot and its data boundary."),
    versions: localize(locale, "像查看时间线一样保存、比较和恢复论文版本。", "Save, compare, and restore manuscript versions through a clear timeline."),
    structure: localize(locale, "确定性提取标题、章节、图表和投稿声明。", "Deterministically extract the title, sections, figures, tables, and declarations."),
    target: localize(locale, "选择国家、出版商和研究报告标准，组合本次检查规则。", "Choose national, publisher, and reporting standards for this check."),
    format: localize(locale, "按出版社要求核对投稿要素，并准备形成可追溯修改。", "Review publisher submission elements and prepare traceable changes."),
    review: localize(locale, "校验规则来源与完整性，逐条查看依据和修复方向。", "Verify rule sources and integrity, then review evidence and remedies item by item."),
    package: localize(locale, "汇总版本、声明与检查报告，形成投稿快照。", "Combine versions, declarations, and check reports into a submission snapshot."),
    knowledge: localize(locale, "查看这篇论文正在形成的作者控制知识体。", "View the author-controlled knowledge body forming around this manuscript."),
  };
  return descriptions[stage];
}

function StageStatus({ stage, structureReport, readinessReport }: { stage: WorkspaceStage; structureReport: StructureReport | null; readinessReport: ReadinessReport | null }) {
  const { text } = useI18n();
  let label = text("当前设备", "This device");
  let tone = "local";
  if (stage === "structure") label = structureReport ? text("提取完成", "Extraction complete") : text("等待提取", "Awaiting extraction");
  if (stage === "versions") label = text("本地版本库", "Local version library");
  if (stage === "target") { label = text("规则目录", "Rule catalog"); tone = "local"; }
  if (stage === "format") { label = text("出版社要素", "Publisher elements"); tone = "local"; }
  if (stage === "review") { label = readinessReport ? text("检查完成", "Checks complete") : structureReport ? text("可以检查", "Ready to check") : text("需要结构", "Structure required"); tone = readinessReport ? "local" : "warning"; }
  if (stage === "package") { label = readinessReport ? `${text("本地快照", "Local snapshot")} v${readinessReport.outputSnapshotVersion}` : text("需要检查", "Checks required"); tone = readinessReport ? "local" : "warning"; }
  if (stage === "knowledge") { label = text("持续积累", "Continuously accumulating"); tone = "info"; }
  return <span className="stage-status" data-tone={tone}><Icon name={tone === "warning" ? "warning" : "check"} />{label}</span>;
}

interface PaneProps { stage: WorkspaceStage; workspace: WorkspaceSummary; structureReport: StructureReport | null; readinessReport: ReadinessReport | null; knowledgeBodySnapshot?: AcademicKnowledgeBodySnapshot | null; ruleCatalog?: RulePackCatalogItem[]; selectedRulePackIds?: string[]; submissionElementCatalog?: SubmissionElementCatalog | null; revisionDraft?: RevisionDraft | null; revisionValues?: Record<string, string>; revisionResult?: RevisionSet | null; versionHistory?: VersionHistory | null; selectedVersion?: number | null; versionComparison?: VersionComparison | null; isComparingVersions?: boolean; }

function OperationPane({ stage, workspace, structureReport, readinessReport, knowledgeBodySnapshot = null, ruleCatalog = [], selectedRulePackIds = [], submissionElementCatalog = null, revisionDraft = null, revisionValues = {}, revisionResult = null, versionHistory, selectedVersion, versionCandidate, versionNote, versionNotice, isLoadingRuleCatalog, isLoadingSubmissionElements, isLoadingKnowledgeBody, isApplyingRevision, isAnalyzing, isEvaluating, isSelectingVersion, isSavingVersion, isRestoringVersion, onAnalyze, onEvaluate, onToggleRulePack, onOpenStage, onRevisionValueChange, onApplyRevision, onSelectVersionCandidate, onVersionNoteChange, onSaveVersion, onSelectVersion, onRestoreVersion }: PaneProps & { versionCandidate: ManuscriptSummary | null; versionNote: string; versionNotice: string | null; isLoadingRuleCatalog: boolean; isLoadingSubmissionElements: boolean; isLoadingKnowledgeBody: boolean; isApplyingRevision: boolean; isAnalyzing: boolean; isEvaluating: boolean; isSelectingVersion: boolean; isSavingVersion: boolean; isRestoringVersion: boolean; onAnalyze: () => void; onEvaluate: () => void; onToggleRulePack: (rulePackId: string) => void; onOpenStage: (stage: WorkspaceStage) => void; onRevisionValueChange: (field: string, value: string) => void; onApplyRevision: () => void; onSelectVersionCandidate: () => void; onVersionNoteChange: (note: string) => void; onSaveVersion: () => void; onSelectVersion: (version: number) => void; onRestoreVersion: (version: number) => void }) {
  const { locale, text } = useI18n();
  if (stage === "source") {
    return <><p className="workspace-created-status"><Icon name="check" />{text("本地工作区已创建", "Local workspace created")}</p><PanelHeading kicker={text("步骤 1 / 8", "Step 1 / 8")} title={text("确认当前稿件", "Confirm current manuscript")} copy={text("ManuscriptDock 已建立不可变副本。后续版本、分析与输出都不会覆盖历史稿件。", "ManuscriptDock created an immutable copy. Later versions, analysis, and outputs never overwrite manuscript history.")} /><dl className="detail-list"><div><dt>{text("文件", "File")}</dt><dd>{workspace.manuscript.name}</dd></div><div><dt>{text("格式与大小", "Format and size")}</dt><dd>{workspace.manuscript.extension.toUpperCase()} · {formatBytes(workspace.manuscript.sizeBytes)}</dd></div><div><dt>{text("当前版本", "Current version")}</dt><dd>v{workspace.snapshotVersion}</dd></div><div><dt>{text("首次导入", "First imported")}</dt><dd>{formatModifiedDate(workspace.importedUnixMs, locale)}</dd></div><div><dt>{text("数据位置", "Data location")}</dt><dd><span className="inline-status"><Icon name="lock" />{text("当前设备", "This device")}</span></dd></div></dl><BoundaryNote title={text("当前边界", "Current boundary")} copy={text("此步骤没有联网、模型调用或外部传输。页面只接收 Rust 返回的安全元数据，不接收源文件路径。", "This step uses no network, model, or external transfer. The page receives safe metadata from Rust, never the source file path.")} /><PaneAction label={text("下一步", "Next")} title={text("进入本地版本库", "Open the local version library")} copy={text("保存修改稿、查看时间线，并在不覆盖历史的情况下恢复旧版。", "Save revisions, inspect the timeline, and restore older work without overwriting history.")} buttonLabel={text("管理论文版本", "Manage manuscript versions")} onClick={() => onOpenStage("versions")} /></>;
  }
  if (stage === "versions") {
    return <VersionManager workspace={workspace} history={versionHistory ?? null} selectedVersion={selectedVersion ?? null} candidate={versionCandidate} note={versionNote} notice={versionNotice} selecting={isSelectingVersion} saving={isSavingVersion} restoring={isRestoringVersion} onSelectCandidate={onSelectVersionCandidate} onNoteChange={onVersionNoteChange} onSave={onSaveVersion} onSelectVersion={onSelectVersion} onRestore={onRestoreVersion} onContinue={onAnalyze} />;
  }
  if (stage === "structure") {
    if (!structureReport) return <EmptyStage icon="structure" kicker={text("步骤 3 / 8", "Step 3 / 8")} title={text("尚未提取论文结构", "Structure has not been extracted")} copy={text("提取过程完全在本机执行，并保留所有不确定性说明。", "Extraction runs entirely on this device and preserves every uncertainty notice.")} actionLabel={isAnalyzing ? text("正在提取…", "Extracting…") : text("开始结构提取", "Start structure extraction")} disabled={isAnalyzing} onAction={onAnalyze} />;
    const authors = structureReport.authors ?? [];
    return <><PanelHeading kicker={`${text("确定性结构提取", "Deterministic structure extraction")} · v${structureReport.analysisVersion}`} title={structureReport.title ?? text("未检测到论文标题", "No manuscript title detected")} copy={structureReport.quality === "complete" ? text("已从本地源快照完成结构提取。", "Structure extraction completed from the local source snapshot.") : text("提取受版面、编码或格式限制，需要作者结合右侧证据确认。", "Extraction is limited by layout, encoding, or format; confirm it against the evidence on the right.")} />{authors.length > 0 ? <p className="structure-authors"><span>{text("作者", "Authors")}</span>{authors.join(" · ")}</p> : null}<span className={`quality-chip quality-${structureReport.quality}`}>{structureReport.quality === "complete" ? text("完整提取", "Complete extraction") : text("受限提取", "Limited extraction")}</span><div className="metric-row" aria-label={text("结构提取统计", "Structure extraction metrics")}><Metric label={text("章节", "Sections")} value={structureReport.sections.length} /><Metric label={text("图", "Figures")} value={structureReport.figureCount} /><Metric label={text("表", "Tables")} value={structureReport.tableCount} /><Metric label={structureReport.pageCount === null ? text("词元", "Words") : text("页数", "Pages")} value={structureReport.pageCount ?? structureReport.wordCount} /></div><ul className="presence-list" aria-label={text("必要结构检测结果", "Required structure detection results")}><Presence label={text("作者", "Authors")} present={authors.length > 0} /><Presence label={text("摘要", "Abstract")} present={structureReport.abstractPresent} /><Presence label={text("关键词", "Keywords")} present={structureReport.keywordsPresent} /><Presence label={text("参考文献", "References")} present={structureReport.referencesPresent} /><Presence label={text("投稿声明", "Submission declarations")} present={structureReport.declarations.length > 0} /></ul>{structureReport.warnings.map((warning) => <p className="inline-warning" key={warning}><Icon name="warning" />{localizeBackendText(locale, warning)}</p>)}<PaneAction label={text("下一步", "Next")} title={text("选择出版标准", "Choose publication standards")} copy={text("先选择适用的国家、出版商和研究报告规则；全部在本机执行。", "Choose applicable national, publisher, and reporting rules; all run locally.")} buttonLabel={text("选择检查标准", "Choose check standards")} onClick={() => onOpenStage("target")} /></>;
  }
  if (stage === "target") {
    return <TargetRuleSelector ruleCatalog={ruleCatalog} selectedRulePackIds={selectedRulePackIds} loading={isLoadingRuleCatalog} structureReady={structureReport !== null} onToggle={onToggleRulePack} onContinue={() => onOpenStage(structureReport ? "format" : "structure")} />;
  }
  if (stage === "format") {
    return <SubmissionElementsDesk catalog={submissionElementCatalog} draft={revisionDraft} values={revisionValues} result={revisionResult} loading={isLoadingSubmissionElements} saving={isApplyingRevision} selectedPublisherCount={ruleCatalog.filter((item) => item.category === "publisher" && selectedRulePackIds.includes(item.id)).length} onValueChange={onRevisionValueChange} onSave={onApplyRevision} onContinue={() => onOpenStage("review")} />;
  }
  if (stage === "review") {
    if (!structureReport) return <EmptyStage icon="review" kicker={text("步骤 6 / 8", "Step 6 / 8")} title={text("需要先建立论文结构", "Manuscript structure is required first")} copy={text("检查规则需要结构字段作为可解释依据。", "The checks require structured fields as explainable evidence.")} actionLabel={text("返回结构提取", "Return to structure extraction")} onAction={() => onOpenStage("structure")} />;
    if (!readinessReport) return <EmptyStage icon="review" kicker={text("步骤 6 / 8", "Step 6 / 8")} title={text("检查投稿准备", "Check submission readiness")} copy={text("校验检查规则的来源与完整性，生成逐条结论和本地 HTML 预览；不会调用 AI 或发送论文。", "Verify check-rule sources and integrity, then generate itemized findings and a local HTML preview without AI calls or manuscript transmission.")} actionLabel={isEvaluating ? text("正在检查…", "Checking…") : text("开始检查", "Start checks")} disabled={isEvaluating} onAction={onEvaluate} />;
    return <><PanelHeading kicker={`${text("投稿准备报告", "Submission-readiness report")} · v${readinessReport.reportVersion}`} title={outcomeLabel(readinessReport.outcome, locale)} copy={text("每项结论均保留规则来源与论文结构定位。", "Every finding retains its rule source and manuscript location.")} /><div className="metric-row" aria-label={text("投稿检查统计", "Submission-check metrics")}><Metric label={text("通过", "Passed")} value={readinessReport.passedCount} /><Metric label={text("建议", "Suggestions")} value={readinessReport.warningCount} /><Metric label={text("阻断", "Blocked")} value={readinessReport.blockedCount} /><Metric label={text("待确认", "Confirmations")} value={readinessReport.confirmationCount} /></div><ol className="finding-list" aria-label={text("投稿检查明细", "Submission-check details")}>{readinessReport.findings.map((finding) => <li key={finding.ruleId} data-status={finding.status}><span className="finding-status">{findingLabel(finding.status, locale)}</span><div><strong>{locale === "en" && finding.messageEn ? finding.messageEn : localizeBackendText(locale, finding.message)}</strong><code>{finding.sourceLocation}</code></div></li>)}</ol><PaneAction label={text("本地输出", "Local output")} title={`${text("查看投稿快照", "View submission snapshot")} v${readinessReport.outputSnapshotVersion}`} copy={text("JSON 报告与 HTML 预览已保存，尚未发生外部传输。", "The JSON report and HTML preview are saved; no external transfer has occurred.")} buttonLabel={text("查看投稿包", "View submission package")} onClick={() => onOpenStage("package")} /></>;
  }
  if (stage === "package") {
    if (!readinessReport) return <EmptyStage icon="package" kicker={text("步骤 7 / 8", "Step 7 / 8")} title={text("投稿包尚未生成", "Submission package has not been generated")} copy={text("完成结构提取和投稿检查后，ManuscriptDock 才会建立版本化输出快照。", "ManuscriptDock creates a versioned output snapshot only after structure extraction and submission checks.")} actionLabel={text("进入检查", "Go to checks")} onAction={() => onOpenStage("review")} />;
    return <><PanelHeading kicker={text("步骤 7 / 8", "Step 7 / 8")} title={`${text("本地投稿快照", "Local submission snapshot")} v${readinessReport.outputSnapshotVersion}`} copy={text("当前快照包含机器可读报告与供作者核验的 HTML 预览。", "This snapshot contains a machine-readable report and a local HTML preview for author verification.")} /><ul className="artifact-list"><li><Icon name="file" /><div><strong>{text("投稿准备报告", "Submission-readiness report")}.json</strong><span>{text("结构化检查结果、定位与规则来源", "Structured findings, locations, and rule sources")}</span></div><em>{text("已生成", "Generated")}</em></li><li><Icon name="file" /><div><strong>{text("投稿准备预览", "Submission-readiness preview")}.html</strong><span>{text("自包含、已转义的本地核验页面", "Self-contained, escaped local verification page")}</span></div><em>{text("已生成", "Generated")}</em></li><li data-pending="true"><Icon name="package" /><div><strong>{text("期刊格式稿与附件清单", "Journal-formatted manuscript and attachment list")}</strong><span>{text("等待目标规则和排版模块", "Waiting for target rules and formatting")}</span></div><em>{text("未生成", "Not generated")}</em></li></ul><BoundaryNote title={text("外发状态", "Outbound status")} copy={text("未发生外部传输。MVP 不包含自动投稿、预印本发布或发送至 PWC。", "No external transfer has occurred. The MVP does not include automatic submission, preprint publishing, or sending to PWC.")} /><PaneAction label={text("长期资产", "Long-term asset")} title={text("写入论文知识体视图", "Write to the manuscript knowledge-body view")} copy={text("查看源稿、结构、规则证据和输出快照如何组成同一个知识对象。", "See how the source, structure, rule evidence, and output snapshots form one knowledge object.")} buttonLabel={text("查看知识体", "View knowledge body")} onClick={() => onOpenStage("knowledge")} /></>;
  }
  if (isLoadingKnowledgeBody && !knowledgeBodySnapshot) return <EmptyStage icon="package" kicker={text("步骤 8 / 8 · 知识体快照", "Step 8 / 8 · Knowledge-body snapshot")} title={text("正在读取本地知识体", "Loading the local knowledge body")} copy={text("正在校验对象版本、AI 审核引用和跨体声明。", "Verifying object versions, AI-review references, and cross-body assertions.")} />;
  const objects = knowledgeBodySnapshot?.objects;
  const aiReview = knowledgeBodySnapshot?.aiReviewReport;
  const retainedReviewCount = knowledgeBodySnapshot?.aiReviewHistory.versions.length ?? 0;
  return (
    <>
      <PanelHeading
        kicker={`${text("步骤 8 / 8 · 知识体快照", "Step 8 / 8 · Knowledge-body snapshot")} · v${knowledgeBodySnapshot?.snapshotVersion ?? 1}`}
        title={text("知识体与关联网络", "Knowledge body and relationship network")}
        copy={text("每个知识体保持自身对象和版本边界；跨体连接必须由带依据、状态和版本的声明对象承担。", "Each knowledge body preserves its own object and version boundary; every cross-body connection requires a versioned assertion with basis and status.")}
      />
      <ul className="knowledge-layers" aria-label={text("知识体核心要素", "Knowledge-body core objects")}>
        <KnowledgeLayer title={`ArtifactVersion · v${objects?.artifactVersion.version ?? workspace.snapshotVersion}`} copy={text("确定知识体所依据的论文、预印本或报告版本，是不可变来源边界", "Pins the manuscript, preprint, or report version that forms the immutable source boundary")} complete />
        <KnowledgeLayer title={`Claim · v${objects?.claim.version ?? 1}`} copy={text("核心可引用主张；表达研究者在特定条件下提出了什么", "The citable core claim: what the researchers assert under specified conditions")} complete />
        <KnowledgeLayer title={`Scope · v${objects?.scope.version ?? 0}`} copy={text("限定 Claim 成立的人群、时间、空间、参数、假设和适用范围", "Limits the Claim by population, time, place, parameters, assumptions, and applicability")} complete={(objects?.scope.version ?? 0) > 0} />
        <KnowledgeLayer title={`Method · v${objects?.method.version ?? 0}`} copy={text("记录研究设计、算法、实验流程、数据处理方式和关键参数", "Records study design, algorithms, experimental workflow, data processing, and key parameters")} complete={(objects?.method.version ?? 0) > 0} />
        <KnowledgeLayer title={`Result · v${objects?.result.version ?? 0}`} copy={text("保存论文实际报告的观察、测量、统计结果或实验输出", "Preserves the observations, measurements, statistics, or experimental outputs actually reported")} complete={(objects?.result.version ?? 0) > 0} />
        <KnowledgeLayer title={`EvidenceRelation · v${objects?.evidenceRelation.version ?? 0}`} copy={text("表达 Result 如何支持、削弱或无法支持 Claim；不是自动推理结论", "States how a Result supports, weakens, or fails to support a Claim; it is not an automated inference")} complete={(objects?.evidenceRelation.version ?? 0) > 0} />
        <KnowledgeLayer title={`SourceAnchor · v${objects?.sourceAnchor.version ?? workspace.snapshotVersion}`} copy={text("把对象精确定位到页、段、句、表、图、公式或数据位置", "Locates objects precisely to a page, paragraph, sentence, table, figure, equation, or data position")} complete />
        <KnowledgeLayer title={`AIReviewReport · ${aiReview ? `v${aiReview.version}` : "v0"}`} copy={text("审核抽取忠实性、来源锚点、结构完整性和越界推理；不裁定科学真理", "Reviews extraction fidelity, source anchors, structural completeness, and overreach; it does not decide scientific truth")} complete={aiReview !== null} />
        <KnowledgeLayer title={`Provenance · v${objects?.provenance.version ?? 1}`} copy={text("记录对象由谁、何时、使用什么模型或流程产生、审核和修订", "Records who created, reviewed, and revised an object, when, and with which model or process")} complete />
        <KnowledgeLayer title={`KnowledgeBodySnapshot · S${objects?.knowledgeBodySnapshot.version ?? knowledgeBodySnapshot?.snapshotVersion ?? 1}`} copy={text("固定组合全部对象的具体版本，形成不可变、可引用的研究记忆快照", "Pins exact versions of all objects into an immutable, citable research-memory snapshot")} complete />
      </ul>
      <section className="knowledge-object-summary" aria-labelledby="ai-review-object-heading">
        <div><span>{text("独立版本对象", "Independent versioned object")}</span><h3 id="ai-review-object-heading">AIReviewReport</h3></div>
        <strong>{aiReview ? `v${aiReview.version}` : text("尚未生成", "Not generated")}</strong>
        <p>{aiReview ? text(`当前快照固定引用 v${aiReview.version}；内部保留 ${retainedReviewCount} 个审核版本。`, `This snapshot pins v${aiReview.version}; ${retainedReviewCount} review versions remain in history.`) : text("确定性投稿检查不是 AI 审核。首次专业审核后将建立 v1，后续升级不会推进 Claim 版本。", "Deterministic submission checks are not AI review. The first professional review creates v1; later upgrades do not advance the Claim version.")}</p>
      </section>
      <details className="relation-contracts">
        <summary>{text(`关联声明协议 · ${knowledgeBodySnapshot?.network.supportedRelations.length ?? 8} 类`, `Relationship assertion protocols · ${knowledgeBodySnapshot?.network.supportedRelations.length ?? 8}`)}</summary>
        <dl>{(knowledgeBodySnapshot?.network.supportedRelations ?? ["citation", "claim_relation", "evidence_relation", "method_transfer", "reproduction", "alignment", "version_relation", "classification"]).map((kind) => <div key={kind}><dt>{relationKindLabel(kind, locale)}</dt><dd>{relationProtocolLabel(kind)}</dd></div>)}</dl>
      </details>
      <BoundaryNote title={text("研究记忆与版本边界", "Research memory and version boundary")} copy={text("单一学术知识体不是一篇论文的摘要，而是围绕一个或一组 Claim 构成的、可追溯且可版本化的研究记忆单元。快照只引用具体对象版本。", "A single academic knowledge body is not a paper abstract. It is a traceable, versioned research-memory unit organized around one or more Claims, and its snapshot pins exact object versions.")} />
    </>
  );
}

function VersionManager({ workspace, history, selectedVersion, candidate, note, notice, selecting, saving, restoring, onSelectCandidate, onNoteChange, onSave, onSelectVersion, onRestore, onContinue }: { workspace: WorkspaceSummary; history: VersionHistory | null; selectedVersion: number | null; candidate: ManuscriptSummary | null; note: string; notice: string | null; selecting: boolean; saving: boolean; restoring: boolean; onSelectCandidate: () => void; onNoteChange: (note: string) => void; onSave: () => void; onSelectVersion: (version: number) => void; onRestore: (version: number) => void; onContinue: () => void }) {
  const { locale, text } = useI18n();
  const currentVersion = history?.currentVersion ?? workspace.snapshotVersion;
  const selected = selectedVersion ?? currentVersion;
  const formatMatches = !candidate || candidate.kind === workspace.manuscript.kind;
  const versions = history ? [...history.versions].reverse() : [];
  return <>
    <PanelHeading kicker={text("步骤 2 / 8 · 本地版本库", "Step 2 / 8 · Local version library")} title={text("保存每一次值得保留的修改", "Keep every revision worth returning to")} copy={text("不需要理解 Git。选择修改后的稿件、写一句版本说明，ManuscriptDock 会保存不可变版本并自动比较变化。", "No Git knowledge is needed. Choose the revised manuscript, add a short note, and ManuscriptDock saves an immutable version and compares the changes.")} />
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
    <button className="secondary-action" type="button" onClick={onContinue}>{text("继续进行结构提取", "Continue to structure extraction")}<Icon name="arrow" /></button>
  </>;
}

function versionOriginLabel(version: ManuscriptVersionSummary, locale: Locale) {
  if (version.origin === "imported") return localize(locale, "初始导入", "Initial import");
  if (version.origin === "restored") return localize(locale, `从 v${version.restoredFromVersion ?? "?"} 恢复`, `Restored from v${version.restoredFromVersion ?? "?"}`);
  return localize(locale, "修改稿", "Revision");
}

function TargetRuleSelector({ ruleCatalog, selectedRulePackIds, loading, structureReady, onToggle, onContinue }: { ruleCatalog: RulePackCatalogItem[]; selectedRulePackIds: string[]; loading: boolean; structureReady: boolean; onToggle: (rulePackId: string) => void; onContinue: () => void }) {
  const { locale, text } = useI18n();
  const categories = [
    ["national_standard", text("中国国家标准", "Chinese national standards")],
    ["ethics", text("出版伦理与透明度", "Publishing ethics and transparency")],
    ["publisher", text("主流出版商", "Major publishers")],
    ["article_type", text("文章类型规范", "Article-type standards")],
    ["reporting_guideline", text("研究报告指南", "Research reporting guidelines")],
  ] as const;
  return <>
    <PanelHeading kicker={text("步骤 4 / 8 · 来源与完整性已校验", "Step 4 / 8 · Sources and integrity verified")} title={text("选择适用于这篇论文的标准", "Choose standards applicable to this manuscript")} copy={text("通用初投稿规则始终启用。只选择真实适用的国家标准、出版商和研究类型；具体期刊作者指南仍具有最高优先级。", "General initial-submission rules are always active. Select only applicable national, publisher, and study-type standards; the journal's own author instructions still take precedence.")} />
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
    <PaneAction label={text("当前组合", "Current composition")} title={selectedRulePackIds.length > 0 ? text(`已选择 ${selectedRulePackIds.length} 套增强规则`, `${selectedRulePackIds.length} enhanced rule pack(s) selected`) : text("仅使用通用投稿规则", "Use general submission rules only")} copy={text("规则在本机执行，不调用 AI，也不会发送论文。", "Rules run locally without AI calls or manuscript transmission.")} buttonLabel={structureReady ? text("核对投稿要素", "Review submission elements") : text("先提取结构", "Extract structure first")} onClick={onContinue} />
  </>;
}

function SubmissionElementsDesk({ catalog, draft, values, result, loading, saving, selectedPublisherCount, onValueChange, onSave, onContinue }: { catalog: SubmissionElementCatalog | null; draft: RevisionDraft | null; values: Record<string, string>; result: RevisionSet | null; loading: boolean; saving: boolean; selectedPublisherCount: number; onValueChange: (field: string, value: string) => void; onSave: () => void; onContinue: () => void }) {
  const { locale, text } = useI18n();
  if (loading || !catalog) return <EmptyStage icon="format" kicker={text("步骤 5 / 8 · 投稿优化修订台", "Step 5 / 8 · Submission Revision Desk")} title={text("正在整理投稿要素", "Preparing submission elements")} copy={text("正在本机组合已签名的出版社要求，不会调用 AI 或发送论文。", "Combining signed publisher requirements locally without AI calls or manuscript transmission.")} />;
  const groups = ["identity", "manuscript", "declarations", "files"];
  const editableCount = catalog.elements.filter((element) => element.editableField).length;
  const changedCount = draft?.fields.filter((field) => (values[field.field] ?? field.value).trim() !== field.value).length ?? 0;
  return <>
    <PanelHeading kicker={text("步骤 5 / 8 · 投稿优化修订台", "Step 5 / 8 · Submission Revision Desk")} title={selectedPublisherCount > 0 ? text("核对出版社投稿要素", "Review publisher submission elements") : text("尚未选择出版社", "No publisher selected")} copy={selectedPublisherCount > 0 ? text("相同要素已合并，具体期刊要求仍优先。先核对内容，再进入确定性检查。", "Shared elements are merged and journal-specific instructions still take precedence. Review the content before deterministic checks.") : text("返回目标步骤选择 Elsevier、Springer Nature、Wiley 或 IEEE，即可形成出版社级投稿清单。", "Return to Target and select Elsevier, Springer Nature, Wiley, or IEEE to build a publisher-level submission list.")} />
    {result ? <p className="revision-saved" role="status"><Icon name="check" />{text(`已保存为 v${result.outputVersion}，${result.changes.length} 项修改已记录来源`, `Saved as v${result.outputVersion}; provenance recorded for ${result.changes.length} change(s)`)}</p> : null}
    {draft && draft.fields.length > 0 ? <section className="revision-fields" aria-labelledby="revision-fields-heading"><header><div><span>{text(`基础版本 v${draft.baseVersion}`, `Base version v${draft.baseVersion}`)}</span><h3 id="revision-fields-heading">{text("可安全回写的字段", "Fields safe to write back")}</h3></div><strong>{draft.format.toUpperCase()}</strong></header>{draft.fields.map((field) => <div className="revision-field" key={field.field}><label htmlFor={`revision-${field.field}`}>{locale === "en" ? field.labelEn : field.label}</label>{field.field === "title" ? <input id={`revision-${field.field}`} value={values[field.field] ?? field.value} onChange={(event) => onValueChange(field.field, event.target.value)} disabled={!field.editable || saving} /> : <textarea id={`revision-${field.field}`} rows={field.field === "abstract" ? 5 : 2} value={values[field.field] ?? field.value} onChange={(event) => onValueChange(field.field, event.target.value)} disabled={!field.editable || saving} />}<small>{field.limitation ? (locale === "en" ? field.limitationEn : field.limitation) : text("作者修改 · 本机处理 · 保存前可在右侧核对差异", "Author edit · Local processing · Review the difference on the right before saving")}</small></div>)}</section> : null}
    {draft?.warnings.map((warning) => <p className="inline-warning" key={warning}><Icon name="warning" />{localizeBackendText(locale, warning)}</p>)}
    {catalog.elements.length > 0 ? <div className="submission-element-groups" aria-label={text("出版社投稿要素", "Publisher submission elements")}>{groups.map((group) => {
      const elements = catalog.elements.filter((element) => element.group === group);
      if (elements.length === 0) return null;
      return <section className="submission-element-group" key={group}><header><h3>{submissionElementGroupLabel(group, locale)}</h3><span>{elements.length}</span></header><ul>{elements.map((element) => <li key={element.id}><span className="element-state"><Icon name={element.editableField ? "format" : "check"} /></span><div><strong>{locale === "en" ? element.labelEn : element.label}</strong><p>{locale === "en" ? element.descriptionEn : element.description}</p><small>{element.editableField ? text("可进入结构化修订", "Structured revision available") : text("作者核对", "Author confirmation")}</small></div></li>)}</ul></section>;
    })}</div> : <div className="submission-elements-empty"><Icon name="target" /><p>{text("当前组合没有出版社级投稿要素；通用检查仍然可用。", "The current composition has no publisher-level elements; general checks remain available.")}</p></div>}
    <BoundaryNote title={text("可信边界", "Trust boundary")} copy={text(`共 ${catalog.elements.length} 项，其中 ${editableCount} 项已连接到后续结构化修订字段。所有来源在右侧只读显示。`, `${catalog.elements.length} elements are listed; ${editableCount} connect to structured revision fields. Every source is shown read-only on the right.`)} />
    <PaneAction label={changedCount > 0 ? text(`${changedCount} 项待保存`, `${changedCount} change(s) pending`) : text("下一步", "Next")} title={changedCount > 0 ? text(`保存为新版本 v${(draft?.baseVersion ?? 0) + 1}`, `Save as new version v${(draft?.baseVersion ?? 0) + 1}`) : text("执行投稿准备检查", "Run submission-readiness checks")} copy={changedCount > 0 ? text("原稿与历史版本不会被覆盖；修改集将随新版本保存在本机。", "The source and history will not be overwritten; the change set is stored locally with the new version.") : text("没有待保存修改，可以继续执行确定性检查。", "There are no unsaved changes; continue to deterministic checks.")} buttonLabel={changedCount > 0 ? (saving ? text("正在保存…", "Saving…") : text("保存为新版本", "Save as new version")) : text("进入投稿检查", "Continue to checks")} disabled={saving} onClick={changedCount > 0 ? onSave : onContinue} />
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

function EvidencePane({ stage, workspace, structureReport, readinessReport, knowledgeBodySnapshot = null, ruleCatalog = [], selectedRulePackIds = [], submissionElementCatalog = null, revisionDraft = null, revisionValues = {}, revisionResult = null, versionHistory = null, selectedVersion = null, versionComparison = null, isComparingVersions = false }: PaneProps) {
  const { locale, text } = useI18n();
  if (stage === "source") return <EvidenceFrame kicker={text("只读版本证据", "Read-only version evidence")} title={text("当前稿件身份", "Current manuscript identity")}><div className="document-sheet source-sheet"><span className="document-type">{workspace.manuscript.extension.toUpperCase()}</span><p className="document-title">{workspace.manuscript.name}</p><dl><div><dt>{text("内容指纹", "Content fingerprint")}</dt><dd>{workspace.contentHash}</dd></div><div><dt>{text("当前版本", "Current version")}</dt><dd>v{workspace.snapshotVersion}</dd></div><div><dt>{text("状态", "Status")}</dt><dd>{text("不可变；历史不会被覆盖", "Immutable; history is never overwritten")}</dd></div></dl></div></EvidenceFrame>;
  if (stage === "versions") return <VersionEvidence workspace={workspace} history={versionHistory} selectedVersion={selectedVersion} comparison={versionComparison} comparing={isComparingVersions} />;
  if (stage === "structure") return <EvidenceFrame kicker={text("结构证据", "Structure evidence")} title={text("论文轮廓", "Manuscript outline")}>{structureReport ? <div className="document-sheet"><p className="document-overline">DETERMINISTIC EXTRACTION · V{structureReport.analysisVersion}</p><p className="document-title">{structureReport.title ?? text("未检测到论文标题", "No manuscript title detected")}</p>{(structureReport.authors ?? []).length > 0 ? <p className="document-authors">{(structureReport.authors ?? []).join(" · ")}</p> : null}<p className="document-meta">{structureReport.pageCount ? `${structureReport.pageCount} ${text("页", "pages")}` : `${structureReport.wordCount} ${text("词元", "words")}`} · {text("源快照", "Source snapshot")} v{structureReport.sourceSnapshotVersion}</p>{structureReport.abstractText ? <section className="abstract-evidence"><h3>{text("识别到的摘要", "Detected abstract")}</h3><p>{structureReport.abstractText}</p></section> : null}{structureReport.sections.length > 0 ? <ol className="section-outline" aria-label={text("检测到的章节", "Detected sections")}>{structureReport.sections.slice(0, 16).map((section, index) => <li key={`${section.level}-${section.heading}-${index}`} style={{ "--section-level": section.level } as CSSProperties}><span>{String(index + 1).padStart(2, "0")}</span><strong>{section.heading}</strong></li>)}</ol> : <EvidenceEmpty copy={text("没有形成可靠章节轮廓，请结合警告人工确认。", "No reliable section outline was formed; review the warnings and confirm manually.")} />}</div> : <EvidenceEmpty copy={text("完成本地结构提取后，这里会显示论文标题、作者、摘要、章节层级和源快照定位。", "After local extraction, the manuscript title, authors, abstract, section hierarchy, and source-snapshot locations appear here.")} />}</EvidenceFrame>;
  if (stage === "target") {
    const selected = ruleCatalog.filter((item) => selectedRulePackIds.includes(item.id));
    return <EvidenceFrame kicker={text("规则证据", "Rule evidence")} title={text("来源与完整性", "Sources and integrity")}><div className="rule-evidence-summary"><strong>{selected.length > 0 ? text(`${selected.length} 套增强规则`, `${selected.length} enhanced rule pack(s)`) : text("通用规则", "General rules")}</strong><p>{text("所有已列出的规则包均已在本机通过数字签名完整性校验。", "Every listed rule pack passed local digital-signature integrity verification.")}</p></div>{selected.length > 0 ? <ul className="provenance-list">{selected.map((item) => <li key={item.id}><span><Icon name="check" /></span><div><strong>{locale === "en" ? item.sourceLabelEn : item.sourceLabel}</strong><p>v{item.version} · {text("覆盖等级", "Coverage")} {item.coverage} · {text("来源可信，内容未被篡改", "Trusted source; content unchanged")}</p></div></li>)}</ul> : <EvidenceEmpty copy={text("尚未选择增强规则；检查仍会应用通用论文结构和初投稿基础规则。", "No enhanced rules are selected; checks will still apply the general manuscript structure and initial-submission baseline.")} />}</EvidenceFrame>;
  }
  if (stage === "format") {
    const pending = revisionResult?.changes ?? revisionDraft?.fields.filter((field) => (revisionValues[field.field] ?? field.value).trim() !== field.value).map((field) => ({ field: field.field, before: field.value, after: revisionValues[field.field] ?? field.value, basis: "author_edit", status: "candidate" })) ?? [];
    return <EvidenceFrame kicker={text("修订证据", "Revision evidence")} title={pending.length > 0 ? text("修改前后", "Before and after") : text("要素来源与完整性", "Element sources and integrity")}>{pending.length > 0 ? <><div className="revision-diff-meta"><strong>{revisionResult ? `v${revisionResult.baseVersion} → v${revisionResult.outputVersion}` : text(`基于 v${revisionDraft?.baseVersion}`, `Based on v${revisionDraft?.baseVersion}`)}</strong><span>{revisionResult ? text("已保存", "Saved") : text("保存前预览", "Pre-save preview")}</span></div><ol className="revision-diff-list">{pending.map((change) => <li key={change.field}><strong>{revisionDraft?.fields.find((field) => field.field === change.field)?.[locale === "en" ? "labelEn" : "label"] ?? change.field}</strong><div><span>{change.before}</span><Icon name="arrow" /><span>{change.after}</span></div></li>)}</ol><BoundaryNote title={text("版本与来源", "Version and provenance")} copy={text("修改由作者确认，在本机生成新版本；没有调用 AI，也没有发生外部传输。", "Author-confirmed changes create a new local version without AI calls or external transmission.")} /></> : submissionElementCatalog && submissionElementCatalog.rulePacks.length > 0 ? <><div className="rule-evidence-summary"><strong>{text(`${submissionElementCatalog.elements.length} 项投稿要素`, `${submissionElementCatalog.elements.length} submission elements`)}</strong><p>{text("要素来自本机已验证的出版商 B 级规则包；具体期刊作者指南具有更高优先级。", "Elements come from locally verified coverage-B publisher packs; journal author instructions have higher priority.")}</p></div><ul className="provenance-list">{submissionElementCatalog.rulePacks.map((pack) => <li key={pack.id}><span><Icon name={pack.signatureVerified ? "check" : "warning"} /></span><div><strong>{locale === "en" && pack.sourceLabelEn ? pack.sourceLabelEn : pack.sourceLabel}</strong><p>v{pack.version} · {text("覆盖等级", "Coverage")} {pack.coverage} · {text("来源可信，内容未被篡改", "Trusted source; content unchanged")}</p></div></li>)}</ul></> : <EvidenceEmpty copy={text("选择出版社后，这里会显示要素来源；可安全修订的字段会显示修改前后。", "Select a publisher to see element sources; safely editable fields show before and after values here.")} />}</EvidenceFrame>;
  }
  if (stage === "review") return <EvidenceFrame kicker={text("规则证据", "Rule evidence")} title={text("来源与完整性", "Sources and integrity")}>{readinessReport ? <><div className="outcome-banner" data-outcome={readinessReport.outcome}><Icon name={readinessReport.outcome === "ready" ? "check" : "warning"} /><div><strong>{outcomeLabel(readinessReport.outcome, locale)}</strong><span>{text("输出快照", "Output snapshot")} v{readinessReport.outputSnapshotVersion}</span></div></div><ul className="provenance-list">{readinessReport.rulePacks.map((pack) => <li key={pack.id}><span><Icon name={pack.signatureVerified ? "check" : "warning"} /></span><div><strong>{locale === "en" && pack.sourceLabelEn ? pack.sourceLabelEn : localizeBackendText(locale, pack.sourceLabel)}</strong><p>v{pack.version} · {text("覆盖等级", "Coverage")} {pack.coverage} · {pack.signatureVerified ? text("来源可信，内容未被篡改", "Trusted source; content unchanged") : text("规则完整性异常", "Rule integrity issue")}</p></div></li>)}</ul><BoundaryNote title={text("传输记录", "Transfer record")} copy={text("未发生外部传输；本次检查由本机确定性规则完成。", "No external transfer occurred; deterministic local rules completed this check.")} /></> : <EvidenceEmpty copy={text("完成投稿准备检查后，这里会显示规则来源、覆盖等级和输出快照。", "After submission-readiness checks, rule sources, coverage, and the output snapshot appear here.")} />}</EvidenceFrame>;
  if (stage === "package") return <EvidenceFrame kicker={text("输出证据", "Output evidence")} title={text("投稿包预览", "Submission-package preview")}>{readinessReport ? <div className="package-preview"><span>MANUSCRIPTDOCK</span><h2>{structureReport?.title ?? workspace.manuscript.name}</h2><p>{text("本地投稿准备快照", "Local submission-readiness snapshot")}</p><dl><div><dt>{text("源快照", "Source snapshot")}</dt><dd>v{workspace.snapshotVersion}</dd></div><div><dt>{text("检查报告", "Check report")}</dt><dd>v{readinessReport.reportVersion}</dd></div><div><dt>{text("输出快照", "Output snapshot")}</dt><dd>v{readinessReport.outputSnapshotVersion}</dd></div></dl><small>{text("仅本机 · 未外发", "Local only · Not transmitted")}</small></div> : <EvidenceEmpty copy={text("投稿检查完成后，这里会显示可核验的本地快照。", "After submission checks, a verifiable local snapshot appears here.")} />}</EvidenceFrame>;
  if (stage === "knowledge") return <EvidenceFrame kicker={text("对象与声明证据", "Object and assertion evidence")} title={text("从单一知识体到关联网络", "From one knowledge body to a relationship network")}><KnowledgeSpatialMap workspace={workspace} structureReport={structureReport} readinessReport={readinessReport} knowledgeBodySnapshot={knowledgeBodySnapshot} /></EvidenceFrame>;
  return <EvidenceFrame kicker={text("版本证据", "Version evidence")} title={text("修订差异预览", "Revision-difference preview")}><EvidenceEmpty copy={text("保存修订版本后，这里将并列显示修改前后与规则依据。", "After saving a revision, before/after content and rule evidence appear here.")} /></EvidenceFrame>;
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

function KnowledgeSpatialMap({ workspace, structureReport, readinessReport, knowledgeBodySnapshot = null }: Omit<PaneProps, "stage">) {
  const { locale, text } = useI18n();
  const [view, setView] = useState<KnowledgeView>("single");
  const network = knowledgeBodySnapshot?.network;
  const bodyCount = network?.bodies.length ?? 1;
  const availableView = view === "pair" ? bodyCount >= 2 : view === "network" ? bodyCount >= 3 : true;
  const claim = knowledgeBodySnapshot?.claim;
  const objects = knowledgeBodySnapshot?.objects;
  const aiReview = knowledgeBodySnapshot?.aiReviewReport;
  const previousReviewVersions = (knowledgeBodySnapshot?.aiReviewHistory.versions ?? []).filter((report) => report.version !== aiReview?.version).map((report) => `v${report.version}`).join(" · ");
  const elements = [
    { key: "artifact-version", label: "ArtifactVersion", version: `v${objects?.artifactVersion.version ?? workspace.snapshotVersion}`, state: text("不可变来源", "Immutable source"), complete: true },
    { key: "ai-review-report", label: "AIReviewReport", version: aiReview ? `v${aiReview.version}` : "v0", state: aiReview ? (previousReviewVersions ? text(`历史 ${previousReviewVersions}`, `History ${previousReviewVersions}`) : text("当前审核", "Current review")) : text("尚未审核", "Not reviewed"), complete: aiReview !== null },
    { key: "scope", label: "Scope", version: `v${objects?.scope.version ?? 0}`, state: structureReport ? text("待作者确认", "Author confirmation") : text("待定义", "Pending definition"), complete: (objects?.scope.version ?? 0) > 0 },
    { key: "source-anchor", label: "SourceAnchor", version: `v${objects?.sourceAnchor.version ?? workspace.snapshotVersion}`, state: `Hash ${workspace.contentHash.slice(0, 8)}`, complete: true },
    { key: "provenance", label: "Provenance", version: `v${objects?.provenance.version ?? 1}`, state: text("本地生成", "Created locally"), complete: true },
    { key: "result", label: "Result", version: `v${objects?.result.version ?? 0}`, state: structureReport ? text("待结构化", "Pending structure") : text("待提取", "Pending extraction"), complete: (objects?.result.version ?? 0) > 0 },
    { key: "evidence-relation", label: "EvidenceRelation", version: `v${objects?.evidenceRelation.version ?? 0}`, state: readinessReport ? text("待建立关系", "Pending relation") : text("待建立", "Pending"), complete: (objects?.evidenceRelation.version ?? 0) > 0 },
    { key: "method", label: "Method", version: `v${objects?.method.version ?? 0}`, state: structureReport ? text("待结构化", "Pending structure") : text("待提取", "Pending extraction"), complete: (objects?.method.version ?? 0) > 0 },
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
        <div className="knowledge-space-visual" role="img" aria-label={text(`单一学术知识体空间视图。KnowledgeBodySnapshot S${objects?.knowledgeBodySnapshot.version ?? knowledgeBodySnapshot?.snapshotVersion ?? 1} 是不可变外层边界；中心是 Claim v${claim?.claim.version ?? 1} 十二面体，八条直线连接 ArtifactVersion、Scope、Method、Result、EvidenceRelation、SourceAnchor、AIReviewReport 和 Provenance 的具体版本。`, `Single academic knowledge-body view. KnowledgeBodySnapshot S${objects?.knowledgeBodySnapshot.version ?? knowledgeBodySnapshot?.snapshotVersion ?? 1} is the immutable outer boundary. A Claim v${claim?.claim.version ?? 1} dodecahedron sits at the center, with eight lines linking exact versions of ArtifactVersion, Scope, Method, Result, EvidenceRelation, SourceAnchor, AIReviewReport, and Provenance.`)}>
          <span className="knowledge-snapshot-label" aria-hidden="true">KnowledgeBodySnapshot · S{objects?.knowledgeBodySnapshot.version ?? knowledgeBodySnapshot?.snapshotVersion ?? 1}</span>
          <svg className="claim-connections" viewBox="0 0 600 420" preserveAspectRatio="none" aria-hidden="true">
            <line x1="300" y1="210" x2="300" y2="58" />
            <line x1="300" y1="210" x2="420" y2="92" />
            <line x1="300" y1="210" x2="504" y2="168" />
            <line x1="300" y1="210" x2="486" y2="302" />
            <line x1="300" y1="210" x2="372" y2="365" />
            <line x1="300" y1="210" x2="228" y2="365" />
            <line x1="300" y1="210" x2="114" y2="302" />
            <line x1="300" y1="210" x2="96" y2="168" />
          </svg>
          <div className="claim-center" aria-hidden="true">
            <ClaimDodecahedron />
            <span className="claim-core"><strong>Claim · v{claim?.claim.version ?? 1}</strong><small>{text("十二面体核心", "Dodecahedron core")}</small></span>
          </div>
          {elements.map((element) => <div className={`claim-element element-${element.key}`} data-complete={element.complete} key={element.key} aria-hidden="true"><span className="element-sphere"><strong>{element.label}</strong><small>{element.version}</small></span><em>{element.state}</em></div>)}
        </div>
      ) : availableView && network ? <KnowledgeNetworkCanvas bodies={view === "pair" ? network.bodies.slice(0, 2) : network.bodies} assertions={network.assertions} view={view} /> : null}
      <p className="knowledge-space-note">{view === "single" ? text("单一学术知识体是围绕一个或一组 Claim 构成的研究记忆单元，不是论文摘要。外圈固定各对象的具体版本；v0 表示对象尚未正式建立。", "A single academic knowledge body is a research-memory unit organized around one or more Claims, not a paper abstract. The outer snapshot pins exact object versions; v0 means an object is not yet established.") : text("圆形边界表示知识体自身边界；绿色菱形表示带依据、状态和版本的声明对象。相似度不会自动成为关系。", "Circular boundaries preserve each knowledge body; green diamonds are versioned assertions with basis and status. Similarity never becomes a relationship automatically.")}</p>
    </div>
  );
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
