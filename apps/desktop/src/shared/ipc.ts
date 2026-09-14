import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AppError,
  AuthorConstraints,
  CompiledPackage,
  DocumentFacts,
  ExportReceipt,
  JobRecord,
  JournalRecord,
  PreparationView,
  Project,
  RecommendationResult,
  SelectionResponse,
  TaskKind,
  MaterialTask,
  MaterialCheck,
  AiSettings, AiPreview, AiRun, AiTask,
} from "./contracts";

async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (typeof error === "object" && error !== null && "code" in error)
      throw error as AppError;
    throw {
      code: "UNKNOWN",
      retryable: true,
      diagnosticId: "frontend-unknown",
    } satisfies AppError;
  }
}

const pendingRequestIds = new Map<string, string>();

function isJobRecord(value: unknown): value is JobRecord {
  return (
    typeof value === "object" &&
    value !== null &&
    "status" in value &&
    "requestId" in value &&
    "events" in value
  );
}

function terminalResult<T>(job: JobRecord): T | undefined {
  if (job.status === "succeeded") {
    if (job.result === undefined) {
      throw {
        code: "JOB_RECORD_INVALID",
        retryable: true,
        diagnosticId: `job-result-${job.id}`,
      } satisfies AppError;
    }
    return job.result as T;
  }
  if (["needs_input", "failed", "cancelled", "interrupted"].includes(job.status)) {
    const reported = [...job.events]
      .reverse()
      .find((event) => event.error)?.error;
    throw Object.assign(
      reported ?? {
        code: job.status === "cancelled" ? "JOB_CANCELLED" : "JOB_INTERRUPTED",
        retryable: job.status !== "cancelled",
        diagnosticId: `job-${job.status}-${job.id}`,
      },
      { jobTerminal: true },
    );
  }
  return undefined;
}

async function waitForJob<T>(initial: JobRecord): Promise<T> {
  let current = initial;
  let pushed: JobRecord | undefined;
  let stopped = false;
  const unlisten = await listen<JobRecord>("manuscriptdock://job", (event) => {
    if (event.payload.id === initial.id) pushed = event.payload;
  }).catch(() => () => undefined);
  try {
    while (!stopped) {
      const terminal = terminalResult<T>(current);
      if (terminal !== undefined) return terminal;
      await new Promise((resolve) => window.setTimeout(resolve, 80));
      if (pushed) {
        current = pushed;
        pushed = undefined;
      } else {
        current = await call<JobRecord>("get_job", {
          projectId: initial.projectId,
          jobId: initial.id,
        });
      }
    }
  } finally {
    stopped = true;
    unlisten();
  }
  throw {
    code: "JOB_INTERRUPTED",
    retryable: true,
    diagnosticId: `job-loop-${initial.id}`,
  } satisfies AppError;
}

async function mutate<T>(
  command: string,
  args: Record<string, unknown>,
): Promise<T> {
  const key = `${command}:${JSON.stringify(args)}`;
  const requestId = pendingRequestIds.get(key) ?? crypto.randomUUID();
  pendingRequestIds.set(key, requestId);
  try {
    const dispatched = await call<T | JobRecord>(command, {
      ...args,
      requestId,
    });
    const result = isJobRecord(dispatched)
      ? await waitForJob<T>(dispatched)
      : dispatched;
    pendingRequestIds.delete(key);
    return result;
  } catch (error) {
    if (
      typeof error === "object" &&
      error !== null &&
      "jobTerminal" in error
    ) {
      pendingRequestIds.delete(key);
    }
    throw error;
  }
}
export const api = {
  onJob: (handler: (job: JobRecord) => void) =>
    listen<JobRecord>("manuscriptdock://job", (event) => handler(event.payload)),
  choose: (
    kind: "manuscript" | "editable_manuscript" | "material" | "folder",
  ) =>
    call<SelectionResponse>("choose_local_inputs", { kind }),
  openProject: (
    manuscriptToken: string,
    task: TaskKind,
    folderToken?: string,
  ) => mutate<Project>("open_project", { manuscriptToken, folderToken, task }),
  project: (projectId: string) =>
    call<Project>("get_project_view", { projectId }),
  recent: () => call<Project[]>("list_recent_projects"),
  removed: () => call<Project[]>("list_removed_projects"),
  hideRecent: (projectId: string) =>
    mutate<Project>("hide_recent_project", { projectId }),
  restoreRecent: (projectId: string) =>
    mutate<Project>("restore_recent_project", { projectId }),
  journals: (query = "") => call<JournalRecord[]>("list_journals", { query }),
  recommend: (projectId: string, constraints: AuthorConstraints) =>
    mutate<RecommendationResult>("recommend_journals", {
      projectId,
      constraints,
    }),
  selectTarget: (
    project: Project,
    journalId: string,
    origin: "catalog" | "recommendation",
    recommendationRef?: string,
    articleType?: string,
  ) =>
    mutate<Project>("select_target", {
      input: {
        projectId: project.id,
        expectedRevision: project.revision,
        journalId,
        articleType: articleType ?? project.facts.articleType ?? "research_article",
        origin,
        recommendationRef,
      },
    }),
  preparation: (projectId: string) =>
    call<PreparationView>("get_preparation", { projectId }),
  workspace: (projectId: string) => call<{ rootPath: string; entries: Array<{ relativePath: string; directory: boolean; sizeBytes: number; modifiedAtUnixMs?: number }> }>("list_package_workspace", { projectId }),
  choosePackageLocation: (projectId: string) => call<boolean>("choose_package_location", { projectId }),
  materialTasks: (projectId: string) => call<MaterialTask[]>("list_material_tasks", { projectId }),
  aiSettings: () => call<AiSettings>("get_ai_settings"),
  saveAiSettings: (input: Omit<AiSettings, "hasKey"> & { apiKey?: string }) => call<AiSettings>("save_ai_settings", { input }),
  prepareAi: (projectId: string, task: AiTask, materialId?: string) => call<AiPreview>("prepare_ai_request", { projectId, task, materialId }),
  runAi: (previewId: string) => call<AiRun>("run_ai_request", { previewId, authorConsent: true }),
  aiHistory: (projectId: string) => call<AiRun[]>("list_ai_runs", { projectId }),
  acceptAiDraft: (projectId: string, runId: string) => call<string>("accept_ai_draft", { projectId, runId }),
  checkMaterial: (projectId: string, materialId: string) => call<MaterialCheck>("check_material", { projectId, materialId }),
  confirmMaterial: (projectId: string, check: MaterialCheck) => call("confirm_material", { projectId, materialId: check.materialId, expectedHash: check.sha256, expectedContext: check.contextHash, authorConfirmed: true }),
  generateMaterials: (project: Project, materialIds: string[] = []) => mutate<string[]>("generate_materials", { projectId: project.id, expectedRevision: project.revision, materialIds }),
  generateMaterialTemplate: (project: Project, materialId: string) => mutate<string[]>("generate_materials", { projectId: project.id, expectedRevision: project.revision, materialIds: [materialId], template: true }),
  generateMaterialTemplates: (project: Project) => mutate<string[]>("generate_materials", { projectId: project.id, expectedRevision: project.revision, materialIds: [], template: true }),
  importWorkspace: (projectId: string, token: string, directory: string) => call<void>("import_workspace_file", { projectId, token, directory }),
  moveWorkspace: (projectId: string, source: string, directory: string) => call<void>("move_workspace_file", { projectId, source, directory }),
  useWorkspaceMaterial: (project: Project, relativePath: string, kind: string) => call<Project>("use_workspace_material", { projectId: project.id, expectedRevision: project.revision, relativePath, kind }),
  openWorkspace: (projectId: string) => call<void>("open_package_workspace", { projectId }),
  openWorkspaceEntry: (projectId: string, relativePath: string) => call<void>("open_workspace_entry", { projectId, relativePath }),
  openSource: (journalId: string, url: string) => call<void>("open_journal_source", { journalId, url }),
  updateFacts: (project: Project, facts: DocumentFacts) =>
    mutate<Project>("update_document_facts", {
      projectId: project.id,
      expectedRevision: project.revision,
      facts,
    }),
  addMaterial: (project: Project, token: string, kind: string) =>
    mutate<Project>("add_material", {
      projectId: project.id,
      expectedRevision: project.revision,
      token,
      kind,
    }),
  build: (projectId: string, contextHash: string, mode: "draft" | "final") =>
    mutate<CompiledPackage>("build_package", { projectId, contextHash, mode }),
  chooseExport: () => call<SelectionResponse>("choose_export_folder"),
  chooseProjectExport: (projectId: string) =>
    call<SelectionResponse>("choose_project_folder_for_export", { projectId }),
  export: (pkg: CompiledPackage, destinationToken: string) =>
    mutate<ExportReceipt>("export_package", {
      packageId: pkg.id,
      contextHash: pkg.contextHash,
      destinationToken,
    }),
  job: (projectId: string | undefined, jobId: string) =>
    call<JobRecord>("get_job", { projectId, jobId }),
  cancelJob: (projectId: string | undefined, jobId: string) =>
    call<JobRecord>("cancel_job", { projectId, jobId }),
};
