import { invoke, isTauri } from "@tauri-apps/api/core";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: vi.fn() }));

const invokeMock = vi.mocked(invoke);
const isTauriMock = vi.mocked(isTauri);

function makeCurrentTarget(workspace: { id: string; snapshotVersion: number }, selectionId = "target-current") {
  return { schemaVersion: 3, selectionId, workspaceId: workspace.id, selectedAgainstManuscriptVersion: workspace.snapshotVersion, recommendationRunId: "run-current", journalId: "synthetic-journal", name: "Synthetic Journal", nameEn: "Synthetic Journal", publisher: "Synthetic Publisher", region: "international", rankSystem: "JCR", rankTier: "Q1", homepageUrl: "https://example.test/journal", articleType: "research", planRole: "primary", priority: 0, selectedUnixMs: Date.UTC(2026, 7, 24, 2, 30), recordHash: "6".repeat(64), externalTransmission: "not_performed" };
}

function makeCurrentRequirements(workspace: { id: string }, target: ReturnType<typeof makeCurrentTarget>) {
  return { schemaVersion: 1, snapshotId: "requirements-current", workspaceId: workspace.id, targetSelectionId: target.selectionId, journalId: target.journalId, journalName: target.name, sourceMode: "author_provided_official_text", status: "author_attested_official", sources: [{ url: "https://example.test/journal/guide-for-authors", title: "Guide for authors", contentHash: "a".repeat(64), capturedUnixMs: Date.UTC(2026, 7, 24, 2, 40), officialHostMatched: true }], requirements: [{ id: "requirement-main", category: "main_manuscript", label: "主稿", labelEn: "Main manuscript", obligation: "required", detail: "Main manuscript required", sourceUrl: "https://example.test/journal/guide-for-authors", evidenceExcerpt: "Main manuscript is required" }], limitations: [], capturedUnixMs: Date.UTC(2026, 7, 24, 2, 40), freshUntilUnixMs: Date.UTC(2099, 0, 1), recordHash: "b".repeat(64), externalTransmission: "not_performed" };
}

describe("App", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isTauriMock.mockReset();
    isTauriMock.mockReturnValue(false);
    window.localStorage.clear();
    Object.defineProperty(window.navigator, "language", { configurable: true, value: "zh-CN" });
    Object.defineProperty(window.navigator, "languages", { configurable: true, value: ["zh-CN"] });
  });

  it("explains the local-first import step", () => {
    const { container } = render(<App />);
    expect(screen.getByRole("heading", { name: "我的工作台" })).toBeVisible();
    expect(screen.getByRole("button", { name: "我的工作台" })).toHaveAttribute("aria-current", "page");
    expect(container.querySelector(".brand-mark img")).toHaveAttribute("src", expect.stringContaining("manuscriptdock-logo.svg"));
    expect(screen.getByLabelText("投稿舱 ManuscriptDock V0.43")).toBeVisible();
    const brandStatement = within(screen.getByRole("region", { name: "投稿舱 ManuscriptDock V0.43" }));
    expect(brandStatement.getByText("V0.43")).toBeVisible();
    expect(brandStatement.getByText("本地论文投稿准备工作台")).toBeVisible();
    expect(brandStatement.getByText("Local-first manuscript submission workspace.")).toHaveAttribute("lang", "en");
    expect(brandStatement.getByText("投论文，上更好的期刊")).toBeVisible();
    expect(brandStatement.getByText("Go for Better Journals.")).toHaveAttribute("lang", "en");
    expect(screen.getByText("你自主决定是否联网、使用模型和外部投送。")).toBeVisible();
    expect(screen.getByRole("button", { name: "选择论文" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "模型设置" })).toBeEnabled();
    expect(screen.getByText("没有文件会在此阶段上传")).toBeVisible();
    expect(container.querySelector(".product-bar > .current-manuscript")).toHaveAttribute("aria-hidden", "true");
    const navigationIcons = within(screen.getByRole("navigation", { name: "工作台导航" }))
      .getAllByRole("button")
      .map((button) => button.querySelector("svg")?.innerHTML);
    expect(new Set(navigationIcons).size).toBe(navigationIcons.length);
  });

  it("switches the complete interface to English and remembers the choice", async () => {
    const user = userEvent.setup();
    const { unmount } = render(<App />);

    await user.click(screen.getByRole("button", { name: "EN" }));

    expect(screen.getByRole("heading", { name: "My Workspace" })).toBeVisible();
    expect(screen.getByRole("button", { name: "Select manuscript" })).toBeEnabled();
    expect(screen.getByRole("navigation", { name: "Workspace navigation" })).toBeVisible();
    expect(screen.getByText("No files are uploaded at this stage")).toBeVisible();
    expect(screen.getByText("You decide whether to go online, use models, or send work externally.")).toBeVisible();
    expect(document.documentElement).toHaveAttribute("lang", "en");
    expect(window.localStorage.getItem("manuscriptdock.locale")).toBe("en");

    unmount();
    render(<App />);
    expect(screen.getByRole("heading", { name: "My Workspace" })).toBeVisible();
  });

  it("keeps an HTTP-only journal target usable through author-provided official text", async () => {
    isTauriMock.mockReturnValue(true);
    const workspace = {
      id: "http-journal-workspace",
      manuscript: { name: "nlp-study.pdf", extension: "pdf", kind: "pdf", sizeBytes: 2048, modifiedUnixMs: null },
      contentHash: "8".repeat(64),
      importedUnixMs: Date.UTC(2026, 8, 4),
      snapshotVersion: 1,
    };
    const target = {
      ...makeCurrentTarget(workspace, "http-target"),
      journalId: "jcip",
      name: "中文信息学报",
      nameEn: "Journal of Chinese Information Processing",
      homepageUrl: "http://jcip.cipsc.org.cn/",
    };
    const targetPlan = { schemaVersion: 4, workspaceId: workspace.id, primary: target, backups: [], updatedUnixMs: Date.UTC(2026, 8, 4) };
    const emptyPortfolio = { sprint: [], matching: [], safeguard: [] };
    const recommendationRun = {
      schemaVersion: 6,
      runId: target.recommendationRunId,
      workspaceId: workspace.id,
      manuscriptVersion: 1,
      resolvedArticleType: "research",
      catalogVersion: "synthetic",
      catalogVerifiedDate: "2026-09-04",
      evaluatedUnixMs: Date.UTC(2026, 8, 4),
      recommendationProfile: { profileVersion: 1, institution: "", specialty: "", manuscriptPurpose: "academic_communication" },
      deadlineDaysRemaining: 90,
      domestic: emptyPortfolio,
      international: emptyPortfolio,
      schoolRuleStatus: "official_source_search_required_excluded_from_score",
      journalDirectoryVersion: null,
      limitations: [],
      externalTransmission: "not_performed",
    };
    const snapshot = {
      ...makeCurrentRequirements(workspace, target),
      snapshotId: "http-requirements",
      sources: [{ url: target.homepageUrl, title: "Author-provided guide", contentHash: "a".repeat(64), capturedUnixMs: Date.UTC(2026, 8, 4), officialHostMatched: true }],
    };
    invokeMock.mockImplementation((command, args) => {
      if (command === "list_workspaces") return Promise.resolve({ workspaces: [workspace], archivedWorkspaces: [], warnings: [] });
      if (command === "get_workspace_storage_summary") return Promise.reject(new Error("not needed"));
      if (command === "get_workspace_lifecycle") return Promise.resolve({ workspaceId: workspace.id, currentVersion: 1, structureReport: null, readinessReport: null, attestation: null, submission: null, knowledgeBody: null, submissionTarget: target, submissionTargetPlan: targetPlan, journalRequirements: null });
      if (command === "get_journal_requirement_snapshots") return Promise.resolve([]);
      if (command === "get_journal_directory_summary") return Promise.reject(new Error("not imported"));
      if (command === "get_journal_profile_discoveries") return Promise.resolve([]);
      if (command === "list_journal_recommendations") return Promise.resolve([recommendationRun]);
      if (command === "get_submission_materials") return Promise.resolve(null);
      if (command === "save_manual_journal_requirements") return Promise.resolve(snapshot);
      return Promise.reject(new Error(`Unexpected command: ${command} ${JSON.stringify(args)}`));
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "打开 nlp-study.pdf" }));
    await user.click(within(screen.getByRole("navigation", { name: "投稿准备主任务" })).getByRole("button", { name: /目标期刊/ }));
    expect(await screen.findByText("当前记录使用 HTTP；授权后先尝试对应的 HTTPS 地址。")).toBeVisible();
    expect(screen.getByRole("button", { name: "获取官方投稿要求" })).toBeDisabled();
    expect(screen.getByText("手动粘贴时，HTTP 网址只作为本地来源记录保存，不触发联网。")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "EN" }));
    expect(screen.getByText("The recorded URL uses HTTP. After authorization, the corresponding HTTPS URL is tried first.")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "中文" }));

    expect(screen.getByLabelText("官方来源网址")).toHaveValue(target.homepageUrl);
    await user.type(screen.getByLabelText("作者指南原文"), "A separate title page is required for every submitted manuscript.");
    await user.click(screen.getByRole("checkbox", { name: /我确认这段原文来自/ }));
    const saveButton = screen.getByRole("button", { name: "保存并生成本地快照" });
    expect(saveButton).toBeEnabled();
    await user.click(saveButton);
    expect(invokeMock).toHaveBeenCalledWith("save_manual_journal_requirements", {
      workspaceId: workspace.id,
      targetSelectionId: target.selectionId,
      sourceUrl: target.homepageUrl,
      requirementText: "A separate title page is required for every submitted manuscript.",
      authorAttestedOfficial: true,
    });
    expect(await screen.findByText("已建立期刊专属要求快照")).toBeVisible();
  });

  it("configures the shared model API key before any manuscript workflow", async () => {
    isTauriMock.mockReturnValue(true);
    let settings = {
      schemaVersion: 1,
      secureStore: "Operating system credential store",
      slots: [
        { role: "primary", enabled: false, providerLabel: "", baseUrl: "", model: "", hasApiKey: false },
        { role: "fallback_1", enabled: false, providerLabel: "", baseUrl: "", model: "", hasApiKey: false },
        { role: "fallback_2", enabled: false, providerLabel: "", baseUrl: "", model: "", hasApiKey: false },
      ],
    };
    invokeMock.mockImplementation((command, args) => {
      if (command === "list_workspaces") return Promise.resolve({ workspaces: [], archivedWorkspaces: [], warnings: [] });
      if (command === "get_workspace_storage_summary") return Promise.resolve({ defaultLocation: "/synthetic/library", storageMode: "application_managed_local_library", sourcePolicy: "immutable_versioned_copy" });
      if (command === "get_model_settings") return Promise.resolve(settings);
      if (command === "save_model_settings") {
        const primary = (args as { slots: Array<Record<string, unknown>> }).slots.find((slot) => slot.role === "primary")!;
        settings = { ...settings, slots: settings.slots.map((slot) => slot.role === "primary" ? { ...slot, enabled: true, providerLabel: String(primary.providerLabel), baseUrl: String(primary.baseUrl), model: String(primary.model), hasApiKey: true } : slot) };
        return Promise.resolve(settings);
      }
      return Promise.reject(new Error(`unexpected command ${command}`));
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "模型设置" }));
    const dialog = await screen.findByRole("dialog", { name: "模型与 API Key" });
    expect(within(dialog).getByText(/期刊资料发现、学校规则抽取和知识体问答共用/)).toBeVisible();
    const primarySlot = within(dialog).getByRole("group", { name: "主模型" });
    await user.click(within(primarySlot).getByRole("button", { name: "使用 DeepSeek 官方配置" }));
    await user.click(within(dialog).getByRole("button", { name: "关闭模型设置" }));
    expect(within(dialog).getByText("放弃尚未保存的模型设置？")).toBeVisible();
    await user.click(within(dialog).getByRole("button", { name: "继续编辑" }));
    await user.type(within(primarySlot).getByLabelText("主模型 API Key"), "synthetic-global-secret");
    await user.click(within(dialog).getByRole("button", { name: "保存模型设置" }));

    expect(await within(dialog).findByText(/API Key 仅保存在系统凭据库/)).toBeVisible();
    expect(within(dialog).getByText(/每个已保存 Key 只向系统凭据库读取一次/)).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("save_model_settings", { slots: expect.arrayContaining([expect.objectContaining({ role: "primary", enabled: true, providerLabel: "DeepSeek", baseUrl: "https://api.deepseek.com", model: "deepseek-v4-flash", apiKey: "synthetic-global-secret" })]) });
    await user.click(within(dialog).getByRole("button", { name: "关闭模型设置" }));
    const modelSettingsButton = screen.getByRole("button", { name: "模型设置" });
    expect(modelSettingsButton).toBeEnabled();
    expect(within(modelSettingsButton).getByText("1")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "EN" }));
    await user.click(screen.getByRole("button", { name: "Models" }));
    expect(await screen.findByText(/each stored key is read from the credential store only once/)).toBeVisible();
  });

  it("renders only safe manuscript metadata after Rust selection", async () => {
    invokeMock.mockResolvedValue({
      status: "selected",
      selectionId: "synthetic-selection",
      manuscript: {
        name: "synthetic-study.docx",
        extension: "docx",
        kind: "word",
        sizeBytes: 15360,
        modifiedUnixMs: Date.UTC(2026, 7, 23, 2, 30),
      },
    });
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "选择论文" }));

    expect(await screen.findByRole("heading", { name: "synthetic-study.docx" })).toBeVisible();
    expect(screen.getByText("本地校验完成")).toBeVisible();
    expect(screen.getByText("15 KB")).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("select_manuscript");
  });

  it("keeps the empty state when the native picker is cancelled", async () => {
    invokeMock.mockResolvedValue({ status: "cancelled" });
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "选择论文" }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "选择论文" })).toBeEnabled();
    });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("shows a recoverable error returned by the Rust boundary", async () => {
    invokeMock.mockResolvedValue({
      status: "rejected",
      message: "当前仅支持 DOCX、PDF 和 TEX 格式",
    });
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "选择论文" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("操作未完成");
    expect(alert).toHaveTextContent("当前仅支持 DOCX、PDF 和 TEX 格式");

    await user.click(screen.getByRole("button", { name: "EN" }));
    expect(screen.getByRole("alert")).toHaveTextContent("Only DOCX, PDF, and TEX formats are currently supported.");
  });

  it("creates an immutable local workspace from the pending selection", async () => {
    invokeMock
      .mockResolvedValueOnce({
        status: "selected",
        selectionId: "one-time-selection",
        manuscript: {
          name: "synthetic-study.tex",
          extension: "tex",
          kind: "latex",
          sizeBytes: 2048,
          modifiedUnixMs: Date.UTC(2026, 7, 24, 1, 0),
        },
      })
      .mockResolvedValueOnce({
        status: "created",
        workspace: {
          id: "synthetic-workspace",
          manuscript: {
            name: "synthetic-study.tex",
            extension: "tex",
            kind: "latex",
            sizeBytes: 2048,
            modifiedUnixMs: Date.UTC(2026, 7, 24, 1, 0),
          },
          contentHash: "a".repeat(64),
          importedUnixMs: Date.UTC(2026, 7, 24, 1, 5),
          snapshotVersion: 1,
        },
      });
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "选择论文" }));
    await screen.findByRole("heading", { name: "synthetic-study.tex" });
    await user.click(screen.getByRole("button", { name: "创建本地工作区" }));

    expect(await screen.findByText("论文已安全保存在本地工作区")).toBeVisible();
    expect(screen.getByText("不可变；历史不会被覆盖")).toBeVisible();
    expect(screen.getByRole("button", { name: "所有论文" })).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("create_workspace", {
      selectionId: "one-time-selection",
    });

    await user.click(screen.getByRole("button", { name: "所有论文" }));
    expect(screen.getByRole("heading", { name: "我的工作台" })).toBeVisible();
    expect(screen.getByRole("button", { name: "我的工作台" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByText("synthetic-study.tex")).toBeVisible();
  });

  it("recovers recent local workspaces when running inside Tauri", async () => {
    isTauriMock.mockReturnValue(true);
    const user = userEvent.setup();
    invokeMock.mockResolvedValue({
      workspaces: [
        {
          id: "recovered-workspace",
          manuscript: {
            name: "recovered-study.pdf",
            extension: "pdf",
            kind: "pdf",
            sizeBytes: 4096,
            modifiedUnixMs: null,
          },
          contentHash: "b".repeat(64),
          importedUnixMs: Date.UTC(2026, 7, 24, 2, 0),
          snapshotVersion: 1,
        },
      ],
      warnings: [],
    });

    render(<App />);

    expect(await screen.findByRole("heading", { name: "最近工作区" })).toBeVisible();
    expect(screen.getByText("recovered-study.pdf")).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("list_workspaces");

    await user.click(screen.getByRole("button", { name: "打开 recovered-study.pdf" }));
    await user.click(screen.getByRole("button", { name: "EN" }));
    const importAnother = await screen.findByRole("button", { name: "Import another" });
    const actions = importAnother.closest(".bar-actions");
    expect(actions).not.toBeNull();
    expect(within(actions as HTMLElement).getByRole("button", { name: "中文" })).toBeVisible();
    expect(within(actions as HTMLElement).getByRole("button", { name: "EN" })).toBeVisible();
    expect(within(actions as HTMLElement).getByText("Local only")).toBeVisible();
  });

  it("archives, restores, and confirms permanent deletion for each manuscript workspace", async () => {
    isTauriMock.mockReturnValue(true);
    const workspace = {
      id: "managed-workspace",
      manuscript: { name: "managed-study.tex", extension: "tex", kind: "latex", sizeBytes: 2048, modifiedUnixMs: null },
      contentHash: "9".repeat(64),
      importedUnixMs: Date.UTC(2026, 7, 25, 2, 0),
      snapshotVersion: 2,
    };
    let catalog = { workspaces: [workspace], archivedWorkspaces: [] as typeof workspace[], warnings: [] as string[] };
    invokeMock.mockImplementation((command) => {
      if (command === "list_workspaces") return Promise.resolve(catalog);
      if (command === "get_workspace_storage_summary") return Promise.resolve({ defaultLocation: "~/Library/Application Support/com.manuscriptdock.desktop/workspace", storageMode: "application_managed_local_library", sourcePolicy: "immutable_versioned_copy" });
      if (command === "export_workspace_copy") return Promise.resolve({ folderName: "ManuscriptDock-managed-study-v2-managed-", workspaceId: workspace.id, manuscriptVersion: 2, fileCount: 12, exportedUnixMs: Date.UTC(2026,7,25), externalTransmission: "not_performed" });
      if (command === "archive_workspace") {
        catalog = { workspaces: [], archivedWorkspaces: [workspace], warnings: [] };
        return Promise.resolve(catalog);
      }
      if (command === "restore_workspace") {
        catalog = { workspaces: [workspace], archivedWorkspaces: [], warnings: [] };
        return Promise.resolve(catalog);
      }
      if (command === "delete_workspace") {
        catalog = { workspaces: [], archivedWorkspaces: [], warnings: [] };
        return Promise.resolve(catalog);
      }
      return Promise.reject(new Error(`unexpected command ${command}`));
    });

    const user = userEvent.setup();
    render(<App />);
    expect(await screen.findByText("managed-study.tex")).toBeVisible();
    expect(screen.getByRole("heading", { name: "ManuscriptDock 本地资料库" })).toBeVisible();
    expect(screen.getByText(/Library\/Application Support/)).toBeVisible();

    await user.click(screen.getByRole("button", { name: "管理 managed-study.tex" }));
    await user.click(screen.getByRole("menuitem", { name: "另存完整工作区…" }));
    expect(await screen.findByText(/已另存完整工作区/)).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("export_workspace_copy", { workspaceId: workspace.id, archived: false });

    await user.click(screen.getByRole("button", { name: "管理 managed-study.tex" }));
    await user.click(screen.getByRole("menuitem", { name: "归档工作区" }));
    expect(await screen.findByText("已归档《managed-study.tex》")).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("archive_workspace", { workspaceId: workspace.id });

    await user.click(screen.getByRole("tab", { name: /已归档/ }));
    expect(screen.getByRole("button", { name: "managed-study.tex 已归档" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "管理 managed-study.tex" }));
    await user.click(screen.getByRole("menuitem", { name: "恢复到最近工作区" }));
    expect(await screen.findByText("已恢复《managed-study.tex》")).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("restore_workspace", { workspaceId: workspace.id });

    await user.click(screen.getByRole("tab", { name: /最近工作区/ }));
    await user.click(screen.getByRole("button", { name: "管理 managed-study.tex" }));
    await user.click(screen.getByRole("menuitem", { name: "永久删除…" }));
    expect(screen.getByText("永久删除这个论文工作区？")).toBeVisible();
    expect(screen.getByText(/全部论文版本、分析、检查、存证、投稿和知识体问答记录/)).toBeVisible();
    await user.click(screen.getByRole("button", { name: "取消" }));
    expect(screen.queryByText("永久删除这个论文工作区？")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "管理 managed-study.tex" }));
    await user.click(screen.getByRole("menuitem", { name: "永久删除…" }));
    await user.click(screen.getByRole("button", { name: "确认永久删除" }));
    expect(await screen.findByText("已永久删除《managed-study.tex》")).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("delete_workspace", { workspaceId: workspace.id, archived: false, authorConfirmed: true });
    expect(screen.getByText("最近工作区为空。")).toBeVisible();
  });

  it("runs local structure and signed-rule readiness checks for a recovered workspace", async () => {
    isTauriMock.mockReturnValue(true);
    const workspace = {
      id: "recovered-workspace",
      manuscript: {
        name: "structured-study.tex",
        extension: "tex",
        kind: "latex",
        sizeBytes: 4096,
        modifiedUnixMs: null,
      },
      contentHash: "c".repeat(64),
      importedUnixMs: Date.UTC(2026, 7, 24, 2, 0),
      snapshotVersion: 1,
    };
    invokeMock
      .mockResolvedValueOnce({ workspaces: [workspace], warnings: [] })
      .mockResolvedValueOnce({
        status: "completed",
        report: {
          analysisVersion: 2,
          workspaceId: workspace.id,
          sourceContentHash: workspace.contentHash,
          sourceSnapshotVersion: 1,
          quality: "complete",
          title: "Synthetic Evidence Study",
          authors: ["Ada Author", "Ben Researcher"],
          abstractPresent: true,
          abstractText: "A compact synthetic abstract.",
          keywordsPresent: true,
          sections: [
            { level: 1, heading: "Introduction" },
            { level: 1, heading: "Methods" },
          ],
          figureCount: 1,
          tableCount: 2,
          referencesPresent: true,
          declarations: ["data_availability"],
          pageCount: null,
          wordCount: 428,
          warnings: [],
        },
      })
      .mockResolvedValueOnce({
        rulePacks: [
          {
            id: "md.publisher.ieee",
            version: "1.0.0",
            coverage: "B",
            stage: "initial_submission",
            region: "global",
            category: "publisher",
            sourceLabel: "IEEE 期刊通用稿件结构",
            sourceLabelEn: "IEEE journal article structure baseline",
            description: "检查 IEEE 期刊文章的通用结构。",
            descriptionEn: "Checks common IEEE journal-article structure.",
            sourceUrls: ["https://journals.ieeeauthorcenter.ieee.org/"],
            verifiedAt: "2026-08-24",
            signatureVerified: true,
          },
        ],
      })
      .mockResolvedValueOnce({
        elements: [
          {
            id: "abstract",
            group: "manuscript",
            label: "单段摘要",
            labelEn: "Single-paragraph abstract",
            description: "核对摘要单段、自足且不超过所选期刊限制。",
            descriptionEn: "Verify a self-contained single paragraph within the journal limit.",
            requirement: "author_confirmation",
            editableField: "abstract",
            rulePackIds: ["md.publisher.ieee"],
            sourceLabels: ["IEEE 期刊通用稿件结构"],
            sourceLabelsEn: ["IEEE journal article structure baseline"],
            sourceUrls: ["https://journals.ieeeauthorcenter.ieee.org/"],
          },
          {
            id: "orcid",
            group: "identity",
            label: "ORCID",
            labelEn: "ORCID",
            description: "核对投稿作者所需 ORCID。",
            descriptionEn: "Verify required ORCID records for submitting authors.",
            requirement: "author_confirmation",
            editableField: null,
            rulePackIds: ["md.publisher.ieee"],
            sourceLabels: ["IEEE 期刊通用稿件结构"],
            sourceLabelsEn: ["IEEE journal article structure baseline"],
            sourceUrls: ["https://journals.ieeeauthorcenter.ieee.org/"],
          },
        ],
        rulePacks: [
          {
            id: "md.publisher.ieee",
            version: "1.1.0",
            coverage: "B",
            stage: "initial_submission",
            sourceLabel: "IEEE 期刊通用稿件结构",
            sourceLabelEn: "IEEE journal article structure baseline",
            sourceUrls: ["https://journals.ieeeauthorcenter.ieee.org/"],
            verifiedAt: "2026-08-24",
            signatureVerified: true,
          },
        ],
      })
      .mockResolvedValueOnce({
        workspaceId: workspace.id,
        baseVersion: 1,
        format: "tex",
        fields: [
          { field: "title", label: "论文标题", labelEn: "Manuscript title", value: "Synthetic Evidence Study", editable: true, limitation: null, limitationEn: null },
          { field: "abstract", label: "摘要", labelEn: "Abstract", value: "Synthetic abstract", editable: true, limitation: null, limitationEn: null },
          { field: "keywords", label: "关键词", labelEn: "Keywords", value: "local, evidence", editable: true, limitation: null, limitationEn: null },
        ],
        warnings: [],
      })
      .mockResolvedValueOnce({
        status: "completed",
        report: {
          reportVersion: 1,
          reportId: "readiness-report",
          workspaceId: workspace.id,
          sourceContentHash: workspace.contentHash,
          sourceSnapshotVersion: 1,
          outputSnapshotVersion: 1,
          generatedUnixMs: Date.UTC(2026, 7, 24, 3, 0),
          outcome: "needs_attention",
          passedCount: 6,
          warningCount: 1,
          blockedCount: 0,
          confirmationCount: 1,
          findings: [
            {
              ruleId: "initial.keywords.recommended",
              rulePackId: "md.stage.initial-submission",
              classification: "recommendation",
              status: "warning",
              message: "补充关键词有助于投稿系统录入与检索。",
              messageEn: "Adding keywords helps submission-system entry and discovery.",
              sourceLocation: "document.keywords",
            },
          ],
          rulePacks: [
            {
              id: "md.stage.initial-submission",
              version: "1.0.0",
              coverage: "C",
              stage: "initial_submission",
              sourceLabel: "ManuscriptDock 通用初投稿准备规则",
              sourceLabelEn: "ManuscriptDock general initial-submission rules",
              signatureVerified: true,
            },
          ],
          externalTransmission: "not_performed",
        },
      });
    const structureReport = {
      analysisVersion: 2, workspaceId: workspace.id, sourceContentHash: workspace.contentHash, sourceSnapshotVersion: 1, quality: "complete",
      title: "Synthetic Evidence Study", authors: ["Ada Author", "Ben Researcher"], abstractPresent: true, abstractText: "A compact synthetic abstract.", keywordsPresent: true,
      sections: [{ level: 1, heading: "Introduction" }, { level: 1, heading: "Methods" }], figureCount: 1, tableCount: 2, referencesPresent: true,
      declarations: ["data_availability"], pageCount: null, wordCount: 428, warnings: [],
    };
    const readinessReport = {
      reportVersion: 1, reportId: "readiness-report", workspaceId: workspace.id, sourceContentHash: workspace.contentHash, sourceSnapshotVersion: 1,
      outputSnapshotVersion: 1, generatedUnixMs: Date.UTC(2026, 7, 24, 3, 0), outcome: "needs_attention", passedCount: 6, warningCount: 1, blockedCount: 0, confirmationCount: 1,
      findings: [{ ruleId: "initial.keywords.recommended", rulePackId: "md.stage.initial-submission", classification: "recommendation", status: "warning", message: "补充关键词有助于投稿系统录入与检索。", messageEn: "Adding keywords helps submission-system entry and discovery.", sourceLocation: "document.keywords" }],
      rulePacks: [{ id: "md.stage.initial-submission", version: "1.0.0", coverage: "C", stage: "initial_submission", sourceLabel: "ManuscriptDock 通用初投稿准备规则", sourceLabelEn: "ManuscriptDock general initial-submission rules", signatureVerified: true }],
      externalTransmission: "not_performed",
    };
    const ieeeRule = { id: "md.publisher.ieee", version: "1.0.0", coverage: "B", stage: "initial_submission", region: "global", category: "publisher", sourceLabel: "IEEE 期刊通用稿件结构", sourceLabelEn: "IEEE journal article structure baseline", description: "检查 IEEE 期刊文章的通用结构。", descriptionEn: "Checks common IEEE journal-article structure.", sourceUrls: ["https://journals.ieeeauthorcenter.ieee.org/"], verifiedAt: "2026-08-24", signatureVerified: true };
    const submissionTarget = makeCurrentTarget(workspace);
    const journalRequirements = makeCurrentRequirements(workspace, submissionTarget);
    invokeMock.mockReset();
    invokeMock.mockImplementation((command) => {
      if (command === "list_workspaces") return Promise.resolve({ workspaces: [workspace], warnings: [] });
      if (command === "get_workspace_lifecycle") return Promise.resolve({ workspaceId: workspace.id, currentVersion: 1, structureReport: null, readinessReport: null, attestation: null, submission: null, knowledgeBody: null, submissionTarget, journalRequirements });
      if (command === "get_journal_requirement_snapshots") return Promise.resolve([journalRequirements]);
      if (command === "list_rule_packs") return Promise.resolve({ rulePacks: [ieeeRule] });
      if (command === "analyze_workspace") return Promise.resolve({ status: "completed", report: structureReport });
      if (command === "evaluate_readiness") return Promise.resolve({ status: "completed", report: readinessReport });
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "打开 structured-study.tex" }));
    await user.click(screen.getByRole("button", { name: "EN" }));
    expect(screen.getByRole("button", { name: /Check revise/ })).toHaveAttribute("title", "Check/revise");
    await user.click(screen.getByRole("button", { name: "中文" }));
    await user.click(screen.getByRole("button", { name: /检查与修订/ }));
    await user.click(screen.getByRole("button", { name: "提取论文结构" }));

    expect(await screen.findByRole("heading", { name: "Synthetic Evidence Study" })).toBeVisible();
    expect(screen.getAllByText("Ada Author · Ben Researcher").length).toBeGreaterThan(0);
    expect(screen.getByText("A compact synthetic abstract.")).toBeVisible();
    expect(screen.getByRole("list", { name: "检测到的章节" })).toHaveTextContent("Methods");
    expect(invokeMock).toHaveBeenCalledWith("analyze_workspace", {
      workspaceId: workspace.id,
    });

    const ieeeOption = await screen.findByRole("checkbox", { name: /IEEE 期刊通用稿件结构/ });
    await user.click(ieeeOption);
    await user.click(screen.getByRole("button", { name: "运行投稿检查" }));

    expect(await screen.findByRole("heading", { name: "仍有事项需要处理" })).toBeVisible();
    expect(screen.getByRole("list", { name: "投稿检查明细" })).toHaveTextContent("补充关键词");
    expect(screen.getByText(/完整性已校验/)).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("evaluate_readiness", {
      workspaceId: workspace.id,
      rulePackIds: ["md.publisher.ieee"],
    });

    await user.click(screen.getByRole("button", { name: "EN" }));
    expect(screen.getByRole("heading", { name: "Items still need attention" })).toBeVisible();
    expect(screen.getByRole("list", { name: "Submission-check details" })).toHaveTextContent("Adding keywords");
    expect(screen.getByText("ManuscriptDock general initial-submission rules")).toBeVisible();
  });

  it("saves and compares revisions through the local version timeline", async () => {
    isTauriMock.mockReturnValue(true);
    const original = {
      id: "versioned-workspace",
      manuscript: { name: "study.tex", extension: "tex", kind: "latex", sizeBytes: 1024, modifiedUnixMs: null },
      contentHash: "1".repeat(64),
      importedUnixMs: Date.UTC(2026, 7, 24, 4, 0),
      snapshotVersion: 1,
    };
    const updated = {
      ...original,
      manuscript: { ...original.manuscript, name: "study-revised.tex", sizeBytes: 1280 },
      contentHash: "2".repeat(64),
      snapshotVersion: 2,
    };
    const v1 = {
      version: 1,
      parentVersion: null,
      manuscript: original.manuscript,
      contentHash: original.contentHash,
      createdUnixMs: original.importedUnixMs,
      note: "",
      origin: "imported",
      restoredFromVersion: null,
    };
    const v2 = {
      version: 2,
      parentVersion: 1,
      manuscript: updated.manuscript,
      contentHash: updated.contentHash,
      createdUnixMs: Date.UTC(2026, 7, 24, 5, 0),
      note: "补充方法与统计分析",
      origin: "revision",
      restoredFromVersion: null,
    };
    let historyCalls = 0;
    invokeMock.mockImplementation((command) => {
      if (command === "list_workspaces") return Promise.resolve({ workspaces: [original], warnings: [] });
      if (command === "get_workspace_lifecycle") return Promise.resolve({ workspaceId: original.id, currentVersion: original.snapshotVersion, structureReport: null, readinessReport: null, attestation: null, submission: null, knowledgeBody: null });
      if (command === "get_version_history") {
        historyCalls += 1;
        return Promise.resolve(historyCalls === 1
          ? { workspaceId: original.id, currentVersion: 1, versions: [v1] }
          : { workspaceId: original.id, currentVersion: 2, versions: [v1, v2] });
      }
      if (command === "select_manuscript") return Promise.resolve({ status: "selected", selectionId: "revision-selection", manuscript: updated.manuscript });
      if (command === "save_manuscript_version") return Promise.resolve({ status: "created", workspace: updated, version: v2 });
      if (command === "compare_manuscript_versions") return Promise.resolve({
        workspaceId: original.id,
        fromVersion: 1,
        toVersion: 2,
        identical: false,
        fromContentHash: original.contentHash,
        toContentHash: updated.contentHash,
        titleBefore: "Study",
        titleAfter: "Study Revised",
        wordCountDelta: 126,
        figureCountDelta: 1,
        tableCountDelta: 0,
        addedSections: ["Methods"],
        removedSections: [],
        addedDeclarations: ["data_availability"],
        removedDeclarations: [],
        externalTransmission: "not_performed",
      });
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "打开 study.tex" }));
    await user.click(screen.getByText("记录与高级功能"));
    await user.click(screen.getByRole("button", { name: "版本历史" }));
    expect(await screen.findByRole("list", { name: "论文版本时间线" })).toHaveTextContent("v1");

    await user.click(screen.getByRole("button", { name: "选择修改稿" }));
    expect(await screen.findByText("study-revised.tex")).toBeVisible();
    await user.type(screen.getByRole("textbox", { name: /版本说明/ }), "补充方法与统计分析");
    await user.click(screen.getByRole("button", { name: /保存为 v2/ }));

    expect(await screen.findByText("已保存版本 v2")).toBeVisible();
    expect(screen.getByRole("list", { name: "论文版本时间线" })).toHaveTextContent("补充方法与统计分析");
    expect(await screen.findByRole("heading", { name: "v1 → v2" })).toBeVisible();
    expect(screen.getByText("Methods")).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("save_manuscript_version", {
      workspaceId: original.id,
      selectionId: "revision-selection",
      note: "补充方法与统计分析",
    });
  });

  it("previews a structured field change and saves it as a new local version", async () => {
    isTauriMock.mockReturnValue(true);
    const workspace = { id: "revision-desk-workspace", manuscript: { name: "study.tex", extension: "tex", kind: "latex", sizeBytes: 1024, modifiedUnixMs: null }, contentHash: "3".repeat(64), importedUnixMs: Date.UTC(2026, 7, 24, 6, 0), snapshotVersion: 1 };
    const revised = { ...workspace, contentHash: "4".repeat(64), snapshotVersion: 2 };
    const draft = { workspaceId: workspace.id, baseVersion: 1, format: "tex", fields: [{ field: "title", label: "论文标题", labelEn: "Manuscript title", value: "Original title", editable: true, limitation: null, limitationEn: null }], warnings: [] };
    const revisedDraft = { ...draft, baseVersion: 2, fields: [{ ...draft.fields[0], value: "Revised title" }] };
    const structureReport = { analysisVersion: 4, workspaceId: workspace.id, sourceContentHash: workspace.contentHash, sourceSnapshotVersion: 1, quality: "complete", title: "Original title", authors: [], abstractPresent: true, abstractText: "Abstract", keywordsPresent: true, sections: [], figureCount: 0, tableCount: 0, referencesPresent: true, declarations: [], pageCount: null, wordCount: 20, warnings: [] };
    const readinessReport = { reportVersion: 1, reportId: "report-v1", workspaceId: workspace.id, sourceContentHash: workspace.contentHash, sourceSnapshotVersion: 1, outputSnapshotVersion: 1, generatedUnixMs: Date.UTC(2026, 7, 24, 6, 10), outcome: "needs_attention", passedCount: 1, warningCount: 1, blockedCount: 0, confirmationCount: 0, findings: [], rulePacks: [], externalTransmission: "not_performed" };
    const revisedStructure = { ...structureReport, sourceContentHash: revised.contentHash, sourceSnapshotVersion: 2, title: "Revised title" };
    const revisedReadiness = { ...readinessReport, reportId: "report-v2", sourceContentHash: revised.contentHash, sourceSnapshotVersion: 2, outputSnapshotVersion: 2 };
    const v1 = { version: 1, parentVersion: null, manuscript: workspace.manuscript, contentHash: workspace.contentHash, createdUnixMs: workspace.importedUnixMs, note: "", origin: "imported", restoredFromVersion: null };
    const v2 = { version: 2, parentVersion: 1, manuscript: revised.manuscript, contentHash: revised.contentHash, createdUnixMs: Date.UTC(2026, 7, 24, 6, 30), note: "投稿优化修订台：1 项修改", origin: "revision", restoredFromVersion: null };
    let draftCalls = 0;
    invokeMock.mockImplementation((command) => {
      if (command === "list_workspaces") return Promise.resolve({ workspaces: [workspace], warnings: [] });
      if (command === "get_workspace_lifecycle") return Promise.resolve({ workspaceId: workspace.id, currentVersion: 1, structureReport, readinessReport, attestation: null, submission: null, knowledgeBody: null });
      if (command === "list_submission_elements") return Promise.resolve({ elements: [], rulePacks: [] });
      if (command === "get_revision_draft") { draftCalls += 1; return Promise.resolve(draftCalls === 1 ? draft : revisedDraft); }
      if (command === "apply_manuscript_revision") return Promise.resolve({ status: "created", workspace: revised, version: v2, revisionSet: { revisionId: "revision-set", workspaceId: workspace.id, baseVersion: 1, outputVersion: 2, createdUnixMs: Date.UTC(2026, 7, 24, 6, 30), changes: [{ field: "title", before: "Original title", after: "Revised title", basis: "author_edit", status: "accepted" }], externalTransmission: "not_performed" } });
      if (command === "analyze_workspace") return Promise.resolve({ status: "completed", report: revisedStructure });
      if (command === "evaluate_readiness") return Promise.resolve({ status: "completed", report: revisedReadiness });
      if (command === "get_submission_materials") return Promise.resolve({ schemaVersion: 1, workspaceId: revised.id, manuscriptVersion: 2, materials: [], checklist: [], requiredComplete: false, targetCheckReady: false });
      if (command === "get_version_history") return Promise.resolve({ workspaceId: workspace.id, currentVersion: 2, versions: [v1, v2] });
      if (command === "compare_manuscript_versions") return Promise.resolve({ workspaceId: workspace.id, fromVersion: 1, toVersion: 2, identical: false, fromContentHash: workspace.contentHash, toContentHash: revised.contentHash, titleBefore: "Original title", titleAfter: "Revised title", wordCountDelta: 0, figureCountDelta: 0, tableCountDelta: 0, addedSections: [], removedSections: [], addedDeclarations: [], removedDeclarations: [], externalTransmission: "not_performed" });
      return Promise.reject(new Error(`unexpected command ${command}`));
    });
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "打开 study.tex" }));
    await user.click(screen.getByRole("button", { name: /检查与修订/ }));
    await user.click(screen.getByRole("button", { name: "修订" }));
    const title = await screen.findByLabelText("论文标题");
    await user.clear(title); await user.type(title, "Revised title");
    expect(screen.getByText("保存前预览")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "保存新版本并重新确认目标" }));
    expect(await screen.findByRole("heading", { name: "先用当前主稿发现适合的期刊" })).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("apply_manuscript_revision", { workspaceId: workspace.id, baseVersion: 1, changes: [{ field: "title", after: "Revised title" }] });
  });

  it("creates local attestation, exports the handoff, and records manual submission", async () => {
    isTauriMock.mockReturnValue(true);
    const workspace = { id: "lifecycle-workspace", manuscript: { name: "lifecycle.tex", extension: "tex", kind: "latex", sizeBytes: 2048, modifiedUnixMs: null }, contentHash: "9".repeat(64), importedUnixMs: Date.UTC(2026, 7, 24, 7, 0), snapshotVersion: 2 };
    const structureReport = { analysisVersion: 4, workspaceId: workspace.id, sourceContentHash: workspace.contentHash, sourceSnapshotVersion: 2, quality: "complete", title: "Lifecycle Study", authors: ["Author"], abstractPresent: true, abstractText: "Abstract", keywordsPresent: true, sections: [], figureCount: 0, tableCount: 0, referencesPresent: true, declarations: [], pageCount: null, wordCount: 100, warnings: [] };
    const readinessReport = { reportVersion: 1, reportId: "report-current", workspaceId: workspace.id, sourceContentHash: workspace.contentHash, sourceSnapshotVersion: 2, outputSnapshotVersion: 2, generatedUnixMs: Date.UTC(2026, 7, 24, 7, 10), outcome: "ready", passedCount: 5, warningCount: 0, blockedCount: 0, confirmationCount: 0, findings: [], rulePacks: [], externalTransmission: "not_performed" };
    const attestation = { attestationId: "attestation-current", workspaceId: workspace.id, manuscriptVersion: 2, manuscriptHash: workspace.contentHash, readinessReportId: readinessReport.reportId, readinessOutputSnapshotVersion: 2, readinessOutcome: "ready", attestedUnixMs: Date.UTC(2026, 7, 24, 7, 20), statement: "confirmed", recordHash: "7".repeat(64), externalTransmission: "not_performed" };
    const submission = { schemaVersion: 2, submissionId: "submission-current", workspaceId: workspace.id, manuscriptVersion: 2, attestationId: attestation.attestationId, targetSelectionId: "target-current", target: "Synthetic Journal", publisher: "Synthetic Publisher", receipt: "SYN-2026", submittedUnixMs: Date.UTC(2026, 7, 24, 7, 30), statement: "recorded", recordHash: "8".repeat(64), externalTransmission: "not_performed" };
    const submissionMaterials = { schemaVersion: 1, workspaceId: workspace.id, manuscriptVersion: 2, materials: [], checklist: [], requiredComplete: true, targetCheckReady: true };
    const submissionTarget = { schemaVersion: 1, selectionId: "target-current", workspaceId: workspace.id, selectedAgainstManuscriptVersion: 2, recommendationRunId: "run-current", journalId: "synthetic-journal", name: "Synthetic Journal", nameEn: "Synthetic Journal", publisher: "Synthetic Publisher", region: "international", rankSystem: "JCR", rankTier: "Q1", homepageUrl: "https://example.test", selectedUnixMs: Date.UTC(2026, 7, 24, 7, 15), recordHash: "6".repeat(64), externalTransmission: "not_performed" };
    const journalRequirements = makeCurrentRequirements(workspace, submissionTarget as ReturnType<typeof makeCurrentTarget>);
    const packagePlan = { schemaVersion: 1, workspaceId: workspace.id, manuscriptVersion: 2, targetSelectionId: submissionTarget.selectionId, targetName: submissionTarget.name, anonymousReview: false, ready: true, files: [{ materialId: null, displayName: workspace.manuscript.name, relativePath: "submission/manuscript.tex", role: "main_manuscript", materialKind: null, checklistItemId: "main-manuscript", checklistLabel: "当前主稿", required: true, included: true, sizeBytes: workspace.manuscript.sizeBytes, contentHash: workspace.contentHash, validationStatus: "passed", validationIssues: [] }], warnings: [], blockers: [], createdUnixMs: Date.UTC(2026, 7, 24, 7, 24), externalTransmission: "not_performed" };
    invokeMock.mockImplementation((command) => {
      if (command === "list_workspaces") return Promise.resolve({ workspaces: [workspace], warnings: [] });
      if (command === "get_workspace_lifecycle") return Promise.resolve({ workspaceId: workspace.id, currentVersion: 2, structureReport, readinessReport, attestation: null, submission: null, knowledgeBody: null, submissionMaterials, submissionTarget, journalRequirements });
      if (command === "get_journal_requirement_snapshots") return Promise.resolve([journalRequirements]);
      if (command === "create_local_attestation") return Promise.resolve(attestation);
      if (command === "get_target_submission_package_plan") return Promise.resolve(packagePlan);
      if (command === "export_target_submission_package") return Promise.resolve({ packageName: "ManuscriptDock-lifecycl-v2", manuscriptVersion: 2, targetSelectionId: submissionTarget.selectionId, targetName: submissionTarget.name, files: ["submission/manuscript.tex", "records/target-selection.json", "records/package-manifest.json", "README.txt"], warnings: [], exportedUnixMs: Date.UTC(2026, 7, 24, 7, 25), externalTransmission: "not_performed" });
      if (command === "record_manual_submission") return Promise.resolve(submission);
      return Promise.reject(new Error(`unexpected command ${command}`));
    });
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "打开 lifecycle.tex" }));
    await user.click(screen.getByText("记录与高级功能"));
    await user.click(screen.getByRole("button", { name: "本地存证" }));
    await user.click(await screen.findByRole("checkbox", { name: /我已核对当前稿件/ }));
    await user.click(screen.getByRole("button", { name: "创建本地存证" }));
    expect(await screen.findByRole("heading", { name: "v2 已完成存证" })).toBeVisible();
    await user.click(screen.getByRole("button", { name: "返回投稿包" }));
    expect(await screen.findByRole("heading", { name: "可以导出" })).toBeVisible();
    expect(screen.getByText("submission/manuscript.tex")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "选择导出文件夹" }));
    expect(await screen.findByText(/已导出 ManuscriptDock-lifecycl-v2/)).toBeVisible();
    expect(screen.getByLabelText("投稿期刊（来自当前主线）")).toHaveValue("Synthetic Journal");
    await user.type(screen.getByLabelText("稿件号或回执（可选）"), "SYN-2026");
    await user.click(screen.getByRole("checkbox", { name: /我确认已经向上述期刊/ }));
    await user.click(screen.getByRole("button", { name: "登记投稿记录" }));
    expect(await screen.findByRole("heading", { name: "Synthetic Journal" })).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("create_local_attestation", { workspaceId: workspace.id, authorConfirmed: true });
    expect(invokeMock).toHaveBeenCalledWith("export_target_submission_package", { workspaceId: workspace.id });
    expect(invokeMock).toHaveBeenCalledWith("record_manual_submission", { workspaceId: workspace.id, target: "Synthetic Journal", receipt: "SYN-2026", authorConfirmed: true });
  });

  it("requires an author-selected discipline and displays the full knowledge-body hash", async () => {
    isTauriMock.mockReturnValue(true);
    const workspace = { id: "classified-workspace", manuscript: { name: "classified-study.tex", extension: "tex", kind: "latex", sizeBytes: 1024, modifiedUnixMs: null }, contentHash: "6".repeat(64), importedUnixMs: Date.UTC(2026, 7, 24, 8, 0), snapshotVersion: 1 };
    const reference = (objectId: string, objectType: string, version: number) => ({ objectId, objectType, version });
    const claim = reference("claim:classified", "claim", 1);
    const anchor = reference("anchor:classified", "source_anchor", 1);
    const method = reference("method:classified", "method", 0);
    const candidate = { candidateId: "kb:classified:candidate:claim:1", text: "The study proposes a traceable synthetic method.", sourceLabel: "Abstract", sourceFragmentId: "fragment:abstract", modality: "text", confidencePercent: 84, authorConfirmed: false };
    const snapshot = {
      schemaVersion: 4, knowledgeBodyId: "kb:classified", snapshotVersion: 1, manuscript: reference("artifact:classified", "artifact_version", 1),
      claim: { claim, proposition: { ...reference("proposition:classified", "proposition", 1), state: "candidate" }, conditions: { ...reference("scope:classified", "scope", 0), state: "pending" }, evidence: { ...reference("evidence:classified", "evidence", 0), state: "pending" }, sources: { ...anchor, state: "established" }, status: { ...reference("status:classified", "status", 1), state: "candidate" } },
      objects: { artifactVersion: reference("artifact:classified", "artifact_version", 1), claim, scope: reference("scope:classified", "scope", 0), method, result: reference("result:classified", "result", 0), evidenceRelation: reference("evidence-relation:classified", "evidence_relation", 0), sourceAnchor: anchor, aiReviewReport: null, provenance: reference("provenance:classified", "provenance", 1), knowledgeBodySnapshot: reference("snapshot:classified", "knowledge_body_snapshot", 1) },
      aiReviewReport: null, aiReviewHistory: { reportId: "review:classified", currentVersion: null, versions: [] },
      sourceIdentity: { version: 1, title: "Classified Study", authors: ["Synthetic Author"], affiliations: ["Synthetic University"], contacts: [{ kind: "email", value: "author@example.edu", sourceLabel: "首页通讯信息", sourceFragmentId: "fragment:contact" }], sourceArtifact: reference("artifact:classified", "artifact_version", 1), status: "extracted", disclosureBasis: "source_document_declared_metadata", localVisibility: "visible_in_local_workspace", externalModelPolicy: "excluded_from_default_model_projection" },
      extraction: { decompositionId: "decomposition:classified", decompositionHash: "d".repeat(64), analysisVersion: 6, sourceSnapshotVersion: 1, generatedBy: "local_deterministic_semantic_extraction", confirmationPolicy: "machine_candidates_require_author_confirmation", claim: { object: reference("proposition:classified", "proposition", 1), state: "candidate", candidates: [candidate] }, scope: { object: reference("scope:classified", "scope", 0), state: "pending", candidates: [] }, method: { object: method, state: "pending", candidates: [] }, result: { object: reference("result:classified", "result", 0), state: "pending", candidates: [] }, evidence: { object: reference("evidence:classified", "evidence", 0), state: "pending", candidates: [] } },
      network: { bodies: [{ body: reference("kb:classified", "knowledge_body", 1), displayId: "K-A", title: "Classified Study", role: "current_study", claim, sourceAnchor: anchor, method }], assertions: [], supportedRelations: ["citation", "claim_relation", "evidence_relation", "method_transfer", "reproduction", "alignment", "version_relation", "classification"] },
      externalTransmission: "not_performed",
    };
    const attestation = { attestationId: "attestation-classified", workspaceId: workspace.id, manuscriptVersion: 1, manuscriptHash: workspace.contentHash, readinessReportId: "report-classified", readinessOutputSnapshotVersion: 1, readinessOutcome: "ready", attestedUnixMs: Date.UTC(2026, 7, 24, 8, 10), statement: "synthetic", recordHash: "a".repeat(64), externalTransmission: "not_performed" };
    const submission = { submissionId: "submission-classified", workspaceId: workspace.id, manuscriptVersion: 1, attestationId: attestation.attestationId, target: "Synthetic Journal", receipt: null, submittedUnixMs: Date.UTC(2026, 7, 24, 8, 20), statement: "synthetic", recordHash: "b".repeat(64), externalTransmission: "not_performed" };
    const classification = { assignmentId: "classification-classified", version: 1, scheme: "ManuscriptDock Discipline Index", schemeVersion: "1.0", code: "life_sciences", label: "生命科学", labelEn: "Life sciences", status: "author_confirmed", basis: "author_selection" };
    const record = { recordId: "knowledge-classified", workspaceId: workspace.id, manuscriptVersion: 1, attestationId: attestation.attestationId, submissionId: submission.submissionId, finalizedUnixMs: Date.UTC(2026, 7, 24, 8, 30), disciplineClassification: classification, snapshot, recordHash: "f".repeat(64), externalTransmission: "not_performed" };
    invokeMock.mockImplementation((command) => {
      if (command === "list_workspaces") return Promise.resolve({ workspaces: [workspace], warnings: [] });
      if (command === "get_workspace_lifecycle") return Promise.resolve({ workspaceId: workspace.id, currentVersion: 1, structureReport: { analysisVersion: 6, workspaceId: workspace.id, sourceContentHash: workspace.contentHash, sourceSnapshotVersion: 1, quality: "complete", title: "Classified Study", authors: ["Synthetic Author"], abstractPresent: true, abstractText: "We propose a synthetic method and report a traceable result.", keywordsPresent: true, sections: [], figureCount: 0, tableCount: 0, referencesPresent: true, declarations: [], pageCount: null, wordCount: 120, semanticCandidates: [], extractionCoverage: { textFragments: 1, tableFragments: 0, figureFragments: 0 }, warnings: [] }, readinessReport: null, attestation, submission, knowledgeBody: null });
      if (command === "get_knowledge_body_snapshot") return Promise.resolve(snapshot);
      if (command === "list_discipline_index") return Promise.resolve([{ code: "life_sciences", label: "生命科学", labelEn: "Life sciences" }]);
      if (command === "finalize_knowledge_body") return Promise.resolve(record);
      if (command === "get_model_settings") return Promise.resolve({ schemaVersion: 1, secureStore: "macOS Keychain", slots: [
        { role: "primary", enabled: false, providerLabel: "", baseUrl: "", model: "", hasApiKey: false },
        { role: "fallback_1", enabled: false, providerLabel: "", baseUrl: "", model: "", hasApiKey: false },
        { role: "fallback_2", enabled: false, providerLabel: "", baseUrl: "", model: "", hasApiKey: false },
      ] });
      if (command === "get_knowledge_dialogue") return Promise.resolve({ workspaceId: workspace.id, knowledgeBodyRecordId: record.recordId, knowledgeBodyHash: record.recordHash, items: [] });
      return Promise.reject(new Error(`unexpected command ${command}`));
    });

    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "打开 classified-study.tex" }));
    await user.click(screen.getByRole("button", { name: "个人知识体" }));
    expect(screen.getByRole("button", { name: /概览/ })).not.toHaveAttribute("aria-current");
    const finalize = await screen.findByRole("button", { name: "确认审核并固化知识体" });
    expect(finalize).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "纳入知识体" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "学科索引分类" }), "life_sciences");
    await user.click(screen.getByRole("checkbox", { name: /我已逐条核对候选内容及来源/ }));
    expect(finalize).toBeEnabled();
    await user.click(finalize);

    expect(await screen.findByRole("heading", { name: "知识体哈希与学科索引" })).toBeVisible();
    expect(screen.getByText("生命科学")).toBeVisible();
    expect(screen.getByText("f".repeat(64))).toBeVisible();
    expect(screen.getByRole("heading", { name: "作者身份与公开联系方式" })).toBeVisible();
    expect(screen.getByText("Synthetic Author")).toBeVisible();
    expect(screen.getByText("Synthetic University")).toBeVisible();
    expect(screen.getByText("author@example.edu")).toBeVisible();
    expect(screen.getByText(/默认不会随知识体问答发送给外部模型/)).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("finalize_knowledge_body", { workspaceId: workspace.id, disciplineCode: "life_sciences", decisions: [{ candidateId: candidate.candidateId, included: true }], authorConfirmed: true });
  });

  it("exposes the staged workspace and author-controlled knowledge body", async () => {
    isTauriMock.mockReturnValue(true);
    const workspace = {
      id: "navigation-workspace",
      manuscript: { name: "navigation-study.pdf", extension: "pdf", kind: "pdf", sizeBytes: 8192, modifiedUnixMs: null },
      contentHash: "d".repeat(64), importedUnixMs: Date.UTC(2026, 7, 24, 4, 0), snapshotVersion: 1,
    };
    const reference = (objectId: string, objectType: string, version: number) => ({ objectId, objectType, version });
    const bodies = ([
      ["K-A", "原研究", "original_research", 7, 3, 3, 2],
      ["K-B", "复现研究", "reproduction_research", 4, 2, 2, 2],
      ["K-C", "竞争研究", "competing_research", 3, 2, 1, 2],
      ["K-D", "跨域应用", "cross_domain_application", 2, 1, 2, 1],
      ["K-E", "后续综合", "later_synthesis", 5, 3, 2, 1],
    ] as const).map(([displayId, title, role, bodyVersion, claimVersion, anchorVersion, methodVersion]) => ({
      body: reference(`body:${displayId}`, "knowledge_body", bodyVersion), displayId, title, role,
      claim: reference(`claim:${displayId}`, "claim", claimVersion), sourceAnchor: reference(`anchor:${displayId}`, "source_anchor", anchorVersion), method: reference(`method:${displayId}`, "method", methodVersion),
    }));
    const assertion = (id: string, relationKind: string, protocolObject: string, from: number, to: number) => ({ assertionId: id, version: 1, relationKind, protocolObject, source: bodies[from].claim, target: bodies[to].claim, basis: [{ label: "synthetic DOI anchor", source: bodies[from].sourceAnchor }], status: "author_confirmed" });
    const knowledgeSnapshot = {
      schemaVersion: 2, knowledgeBodyId: "body:K-A", snapshotVersion: 7, manuscript: reference("artifact:K-A", "artifact_version", 3),
      claim: { claim: bodies[0].claim, proposition: { ...reference("proposition:K-A", "proposition", 3), state: "established" }, conditions: { ...reference("scope:K-A", "scope", 3), state: "established" }, evidence: { ...reference("evidence:K-A", "evidence", 2), state: "established" }, sources: { ...bodies[0].sourceAnchor, state: "established" }, status: { ...reference("status:K-A", "status", 2), state: "established" } },
      objects: { artifactVersion: reference("artifact:K-A", "artifact_version", 3), claim: bodies[0].claim, scope: reference("scope:K-A", "scope", 3), method: bodies[0].method, result: reference("result:K-A", "result", 2), evidenceRelation: reference("evidence-relation:K-A", "evidence_relation", 2), sourceAnchor: bodies[0].sourceAnchor, aiReviewReport: reference("review:K-A", "ai_review_report", 2), provenance: reference("provenance:K-A", "provenance", 2), knowledgeBodySnapshot: reference("snapshot:K-A", "knowledge_body_snapshot", 7) },
      aiReviewReport: reference("review:K-A", "ai_review_report", 2),
      aiReviewHistory: { reportId: "review:K-A", currentVersion: 2, versions: [{ reportId: "review:K-A", version: 1, previousVersion: null }, { reportId: "review:K-A", version: 2, previousVersion: 1 }] },
      sourceIdentity: { version: 7, title: "Navigation Study", authors: ["Ada Author", "Ben Researcher"], affiliations: ["Synthetic Research Institute"], contacts: [{ kind: "email", value: "ada@example.edu", sourceLabel: "PDF 首页", sourceFragmentId: "fragment:identity:1" }], sourceArtifact: reference("artifact:K-A", "artifact_version", 3), status: "extracted", disclosureBasis: "source_document_declared_metadata", localVisibility: "visible_in_local_workspace", externalModelPolicy: "excluded_from_default_model_projection" },
      extraction: {
        decompositionId: "decomposition:K-A", decompositionHash: "7".repeat(64), analysisVersion: 6, sourceSnapshotVersion: 7, generatedBy: "local_deterministic_semantic_extraction", confirmationPolicy: "machine_candidates_require_author_confirmation",
        claim: { object: reference("proposition:K-A", "proposition", 3), state: "established", candidates: [{ candidateId: "candidate:claim:K-A", text: "The proposed method improves traceable manuscript analysis under the reported conditions.", sourceLabel: "Abstract", sourceFragmentId: "fragment:abstract", modality: "text", confidencePercent: 91, authorConfirmed: true }] },
        scope: { object: reference("scope:K-A", "scope", 3), state: "established", candidates: [{ candidateId: "candidate:scope:K-A", text: "The claim applies to the evaluated academic manuscript corpus.", sourceLabel: "Methods", sourceFragmentId: "fragment:methods:1", modality: "text", confidencePercent: 86, authorConfirmed: true }] },
        method: { object: bodies[0].method, state: "established", candidates: [{ candidateId: "candidate:method:K-A", text: "The study uses a deterministic extraction and comparison pipeline.", sourceLabel: "Methods", sourceFragmentId: "fragment:methods:2", modality: "text", confidencePercent: 88, authorConfirmed: true }] },
        result: { object: reference("result:K-A", "result", 2), state: "established", candidates: [{ candidateId: "candidate:result:K-A", text: "The pipeline reports improved traceability in the synthetic evaluation.", sourceLabel: "Results", sourceFragmentId: "fragment:results:1", modality: "text", confidencePercent: 89, authorConfirmed: true }] },
        evidence: { object: reference("evidence:K-A", "evidence", 2), state: "established", candidates: [{ candidateId: "candidate:evidence:K-A", text: "Reported measurements support the primary claim within the stated scope.", sourceLabel: "Table 1", sourceFragmentId: "fragment:table:1", modality: "table", confidencePercent: 87, authorConfirmed: true }] },
      },
      serviceArchitecture: {
        identityAndVersion: { knowledgeBody: bodies[0].body, currentSnapshot: reference("snapshot:K-A", "knowledge_body_snapshot", 7), sourceArtifact: reference("artifact:K-A", "artifact_version", 3), creatorProvenance: reference("provenance:K-A", "provenance", 2), lifecycleStatus: "active", supersedes: null, immutableHistory: true },
        knowledgeBoundaryAndEvidence: { claims: [bodies[0].claim], scope: reference("scope:K-A", "scope", 3), method: bodies[0].method, result: reference("result:K-A", "result", 2), evidence: reference("evidence:K-A", "evidence", 2), evidenceRelation: reference("evidence-relation:K-A", "evidence_relation", 2), sourceAnchor: bodies[0].sourceAnchor, knownLimitations: [], unverifiedObjects: [] },
        capabilityContracts: [{ contractId: "capability:qa", version: 1, capability: "evidence_bounded_question_answering", inputContract: ["question"], outputContract: ["answer", "source_anchors"], preconditions: ["author_configured_runtime"], refusalConditions: ["insufficient_evidence"], evidenceSources: [bodies[0].sourceAnchor], availability: "requires_runtime" }],
        interactionRuntime: { runtimeProfile: reference("runtime:K-A", "runtime_profile", 1), bindingPolicy: "replaceable", coordinatorRole: "author_configured_model", allowedTools: ["source_anchor_lookup"], perCallAuthorization: true, externalTransmission: "author_confirmed_per_request" },
        validationRightsAndReputation: { validationRecords: [reference("review:K-A", "ai_review_report", 2)], rightsPolicy: reference("rights:K-A", "rights_policy", 1), reputationRecord: reference("reputation:K-A", "reputation_record", 4), contentSnapshot: reference("snapshot:K-A", "knowledge_body_snapshot", 7), attributionRequired: true, reputationUpdatesIndependently: true, reuseControl: "author_controlled" },
      },
      network: { bodies, assertions: [assertion("reproduction:1", "reproduction", "ReproductionAssertion", 0, 1), assertion("conflict:1", "claim_relation", "ClaimRelationAssertion", 1, 2), assertion("transfer:1", "method_transfer", "MethodRelationAssertion", 0, 3), assertion("citation:1", "citation", "CitationAssertion", 1, 4), assertion("classification:1", "classification", "ClassificationAssignment", 3, 4), assertion("evidence:1", "evidence_relation", "EvidenceRelation", 2, 4)], supportedRelations: ["citation", "claim_relation", "evidence_relation", "method_transfer", "reproduction", "alignment", "version_relation", "classification"] },
      externalTransmission: "not_performed",
    };
    const attestation = { attestationId: "attestation-1", workspaceId: workspace.id, manuscriptVersion: 1, manuscriptHash: workspace.contentHash, readinessReportId: "report-1", readinessOutputSnapshotVersion: 1, readinessOutcome: "ready", attestedUnixMs: Date.UTC(2026, 7, 24, 5, 0), statement: "synthetic", recordHash: "a".repeat(64), externalTransmission: "not_performed" };
    const submission = { submissionId: "submission-1", workspaceId: workspace.id, manuscriptVersion: 1, attestationId: attestation.attestationId, target: "Synthetic Journal", receipt: "SYN-1", submittedUnixMs: Date.UTC(2026, 7, 24, 5, 30), statement: "synthetic", recordHash: "b".repeat(64), externalTransmission: "not_performed" };
    const disciplineClassification = { assignmentId: "classification-1", version: 1, scheme: "ManuscriptDock Discipline Index", schemeVersion: "1.0", code: "computer_information_sciences", label: "计算机与信息科学", labelEn: "Computer and information sciences", status: "author_confirmed", basis: "author_selection" };
    const knowledgeBody = { recordId: "knowledge-1", workspaceId: workspace.id, manuscriptVersion: 1, attestationId: attestation.attestationId, submissionId: submission.submissionId, finalizedUnixMs: Date.UTC(2026, 7, 24, 6, 0), disciplineClassification, snapshot: knowledgeSnapshot, recordHash: "c".repeat(64), externalTransmission: "not_performed" };
    let modelConfigured = false;
    const emptyModelSlots = [
      { role: "primary", enabled: false, providerLabel: "", baseUrl: "", model: "", hasApiKey: false },
      { role: "fallback_1", enabled: false, providerLabel: "", baseUrl: "", model: "", hasApiKey: false },
      { role: "fallback_2", enabled: false, providerLabel: "", baseUrl: "", model: "", hasApiKey: false },
    ];
    const configuredModelSlots = [
      { role: "primary", enabled: true, providerLabel: "Synthetic AI", baseUrl: "https://api.synthetic.example/v1", model: "synthetic-reasoner", hasApiKey: true },
      emptyModelSlots[1], emptyModelSlots[2],
    ];
    const inquiry = { schemaVersion: 1, inquiryId: "inquiry-1", workspaceId: workspace.id, knowledgeBodyRecordId: knowledgeBody.recordId, knowledgeBodyHash: knowledgeBody.recordHash, snapshotVersion: 7, origin: "owner", stance: "challenge", target: "claim", question: "这个 Claim 缺少哪些直接来源锚点？", externalActorLabel: null, createdUnixMs: Date.UTC(2026, 7, 24, 7, 0), recordHash: "d".repeat(64), externalTransmission: "author_confirmed_model_projection" };
    const answer = { schemaVersion: 1, answerId: "answer-1", inquiryId: inquiry.inquiryId, workspaceId: workspace.id, knowledgeBodyRecordId: knowledgeBody.recordId, modelSlot: "primary", providerLabel: "Synthetic AI", model: "synthetic-reasoner", answer: "当前投影只给出一个 SourceAnchor；需要逐条核验 Claim 对应的页、段和句。", sourceAnchors: [bodies[0].sourceAnchor], createdUnixMs: Date.UTC(2026, 7, 24, 7, 1), recordHash: "e".repeat(64), externalTransmission: "author_confirmed_model_projection" };
    invokeMock.mockImplementation((command) => {
      if (command === "list_workspaces") return Promise.resolve({ workspaces: [workspace], warnings: [] });
      if (command === "get_workspace_lifecycle") return Promise.resolve({ workspaceId: workspace.id, currentVersion: 1, structureReport: null, readinessReport: null, attestation, submission, knowledgeBody });
      if (command === "list_discipline_index") return Promise.resolve([disciplineClassification]);
      if (command === "get_model_settings") return Promise.resolve({ schemaVersion: 1, secureStore: "macOS Keychain", slots: modelConfigured ? configuredModelSlots : emptyModelSlots });
      if (command === "get_knowledge_dialogue") return Promise.resolve({ workspaceId: workspace.id, knowledgeBodyRecordId: knowledgeBody.recordId, knowledgeBodyHash: knowledgeBody.recordHash, items: [] });
      if (command === "save_model_settings") { modelConfigured = true; return Promise.resolve({ schemaVersion: 1, secureStore: "macOS Keychain", slots: configuredModelSlots }); }
      if (command === "ask_knowledge_body") return Promise.resolve({ workspaceId: workspace.id, knowledgeBodyRecordId: knowledgeBody.recordId, knowledgeBodyHash: knowledgeBody.recordHash, items: [{ inquiry, answers: [answer] }] });
      return Promise.reject(new Error(`unexpected command ${command}`));
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "打开 navigation-study.pdf" }));
    expect(screen.getByRole("navigation", { name: "投稿准备主任务" })).toBeVisible();
    await user.click(screen.getByRole("button", { name: "查看依据" }));
    expect(screen.getByLabelText("概览 证据")).toHaveTextContent("只读");

    await user.click(screen.getByText("记录与高级功能"));
    await user.click(screen.getByRole("button", { name: "个人知识体" }));
    expect(screen.getByRole("button", { name: /概览/ })).not.toHaveAttribute("aria-current");
    expect(await screen.findByRole("heading", { name: "知识体与关联网络" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "知识体哈希与学科索引" })).toBeVisible();
    expect(screen.getByText("计算机与信息科学")).toBeVisible();
    expect(screen.getByText("computer_information_sciences")).toBeVisible();
    expect(screen.getByText("c".repeat(64))).toBeVisible();
    expect(screen.getByText("ClassificationAssignment · v1")).toBeVisible();
    const fiveLayers = screen.getByRole("list", { name: "知识体五部分架构" });
    expect(fiveLayers.children).toHaveLength(5);
    expect(fiveLayers).toHaveTextContent("身份与版本 · Artifact v3 · Snapshot S7");
    expect(fiveLayers).toHaveTextContent("知识、边界与证据 · Claim v3");
    expect(fiveLayers).toHaveTextContent("能力契约 · 1 项");
    expect(fiveLayers).toHaveTextContent("交互与执行运行时 · RuntimeProfile v1");
    expect(fiveLayers).toHaveTextContent("验证、权利与信誉 · Reputation v4");

    const spatialMap = screen.getByRole("region", { name: "动态知识点云与关联网络" });
    expect(spatialMap.querySelector(".knowledge-point-cloud")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "打开模型设置" })).toBeEnabled();

    await user.click(screen.getByRole("button", { name: "查看依据" }));
    expect(screen.getByRole("button", { name: "收起依据" })).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText(/动态图、节点交互和 AI 问答已放回知识体主界面/)).toBeVisible();
    expect(spatialMap.querySelector(".claim-dodecahedron")).toBeInTheDocument();
    expect(spatialMap.querySelectorAll(".dodeca-edge")).toHaveLength(30);
    expect(spatialMap.querySelector(".claim-core")).toHaveTextContent("Claim · v3作者已确认");
    expect(spatialMap.querySelectorAll(".service-layer-node")).toHaveLength(5);
    expect(spatialMap).toHaveTextContent("身份与版本S72 位作者 · 1 项联系方式 · Artifact v3");
    expect(spatialMap).toHaveTextContent("能力契约v1 · 1输入 · 输出 · 前置 · 拒绝");
    expect(spatialMap).toHaveTextContent("验证、权利与信誉Reputation · v4AIReview v2 · 历史 v1");
    await user.click(within(spatialMap).getByRole("button", { name: /能力契约/ }));
    expect(within(spatialMap).getByRole("status")).toHaveTextContent("能力契约 · v1 · 1");
    expect(screen.getByRole("heading", { name: "知识摘要与来源" })).toBeVisible();
    expect(screen.getAllByText("The proposed method improves traceable manuscript analysis under the reported conditions.").length).toBeGreaterThanOrEqual(2);

    await user.click(screen.getByRole("tab", { name: "2. 两体关联" }));
    expect(screen.getByRole("img", { name: /2 个保持边界的知识体/ })).toBeVisible();
    expect(document.querySelectorAll(".network-body")).toHaveLength(2);

    await user.click(screen.getByRole("tab", { name: "3. 关联网络" }));
    expect(screen.getByRole("img", { name: /5 个保持边界的知识体/ })).toBeVisible();
    expect(document.querySelectorAll(".network-body")).toHaveLength(5);
    expect(document.querySelectorAll(".network-assertion")).toHaveLength(6);

    await user.click(screen.getByRole("button", { name: "打开模型设置" }));
    expect(screen.getByText("1 个主模型，2 个备选模型")).toBeVisible();
    expect(screen.getByText("主模型")).toBeVisible();
    expect(screen.getByText("备选模型 1")).toBeVisible();
    expect(screen.getByText("备选模型 2")).toBeVisible();
    const primarySlot = screen.getByRole("group", { name: "主模型" });
    await user.click(within(primarySlot).getByRole("button", { name: "使用 DeepSeek 官方配置" }));
    expect(within(primarySlot).getByLabelText("提供方名称")).toHaveValue("DeepSeek");
    expect(within(primarySlot).getByLabelText("API 地址")).toHaveValue("https://api.deepseek.com");
    expect(within(primarySlot).getByLabelText("模型名称")).toHaveValue("deepseek-v4-flash");
    expect(screen.getByRole("button", { name: "请先补全启用项" })).toBeDisabled();
    await user.type(within(primarySlot).getByLabelText("API Key"), "synthetic-secret");
    await user.click(screen.getByRole("button", { name: "保存模型设置" }));
    expect(await screen.findByText(/API Key 仅保存在系统凭据库/)).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("save_model_settings", { slots: expect.arrayContaining([expect.objectContaining({ role: "primary", enabled: true, providerLabel: "DeepSeek", baseUrl: "https://api.deepseek.com", model: "deepseek-v4-flash", apiKey: "synthetic-secret" })]) });

    await user.selectOptions(screen.getByLabelText("提问类型"), "challenge");
    await user.selectOptions(screen.getByLabelText("针对对象"), "claim");
    await user.type(screen.getByLabelText("问题或需求"), inquiry.question);
    await user.click(screen.getByRole("button", { name: "询问知识体" }));
    expect(await screen.findByText(answer.answer)).toBeVisible();
    expect(screen.getByText("Synthetic AI · synthetic-reasoner")).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("ask_knowledge_body", { workspaceId: workspace.id, stance: "challenge", target: "claim", question: inquiry.question, authorConfirmedExternalTransmission: true });

    await user.click(screen.getByRole("tab", { name: /外部反馈 · 预留/ }));
    expect(screen.getByRole("heading", { name: "为外部读者保留的论文提问窗口" })).toBeVisible();
    expect(screen.getByText("当前版本不接收外部网络请求，也不会虚构外部反馈。")).toBeVisible();
  });

  it("recommends a two-three-three portfolio for each region and recalculates after adjustment", async () => {
    isTauriMock.mockReturnValue(true);
    const workspace = { id: "journal-workspace", manuscript: { name: "vision-study.tex", extension: "tex", kind: "latex", sizeBytes: 4096, modifiedUnixMs: null }, contentHash: "e".repeat(64), importedUnixMs: Date.UTC(2026, 7, 30), snapshotVersion: 2 };
    const makeItem = (id: string, domestic: boolean, index: number) => ({ id, name: `${domestic ? "国内期刊" : "国际期刊"}${index}`, nameEn: `${domestic ? "Domestic" : "International"} Journal ${index}`, region: domestic ? "domestic" : "international", publisher: "合成学术出版社", publisherEn: "Synthetic Society", rankSystem: "Synthetic CCF", rankTier: domestic ? "T1" : "CCF A", deadlineStatus: "planning_window_sufficient", institutionEligibility: "requires_verified_official_rules", rankingSourceUrl: "https://example.test/rank", homepageUrl: "https://example.test/journal", openAccessStatus: "open", directoryEvidence: [] });
    let runCount = 0;
    let targetPlan: { schemaVersion: number; workspaceId: string; primary: Record<string, unknown> | null; backups: Record<string, unknown>[]; updatedUnixMs: number } = { schemaVersion: 1, workspaceId: workspace.id, primary: null, backups: [], updatedUnixMs: 0 };
    let requirementSnapshots: Record<string, unknown>[] = [];
    let recommendationRuns: Record<string, unknown>[] = [];
    let profileDiscoveries: Record<string, unknown>[] = [];
    let materialPresent = true;
    const storedMaterial = { materialId: "11111111-1111-4111-8111-111111111111", kind: "title_page", originalName: "title-page.docx", extension: "docx", sizeBytes: 2048, contentHash: "9".repeat(64), importedUnixMs: Date.UTC(2026,7,30), manuscriptVersion: 2, targetSelectionId: "selection-dr1-primary", requirementSnapshotId: "requirements-1", checklistItemId: "journal-title-page", included: true, validationStatus: "passed", validationIssues: [], detectedMediaType: "application/vnd.openxmlformats-officedocument.wordprocessingml.document" };
    const materialCatalog = () => ({ schemaVersion: 4, workspaceId: workspace.id, manuscriptVersion: 2, detectedFigureCount: 1, detectedTableCount: 1, materials: materialPresent ? [storedMaterial] : [], checklist: [
      { id: "journal-title-page", label: "标题页", labelEn: "Title page", group: "files", requirement: "required", status: materialPresent ? "passed" : "missing", detail: materialPresent ? "已绑定标题页" : "请上传标题页", verification: "file", materialKind: "title_page", blocking: true, confirmable: false, sourceUrl: "https://example.test/journal/guide-for-authors", evidenceExcerpt: "A separate title page is required", capturedUnixMs: Date.UTC(2026,7,30), freshUntilUnixMs: Date.UTC(2026,10,30), requiredCount: 1, matchedMaterialIds: materialPresent ? [storedMaterial.materialId] : [] },
      { id: "figure-originals", label: "原始图件", labelEn: "Original figures", group: "files", requirement: "recommended", status: "missing", detail: "正文包含图片；请提供独立高精度原图", verification: "file", materialKind: "figure", blocking: false, confirmable: false, sourceUrl: null, evidenceExcerpt: null, capturedUnixMs: null, freshUntilUnixMs: null, requiredCount: 1, matchedMaterialIds: [] },
      { id: "table-editables", label: "可编辑表格", labelEn: "Editable tables", group: "files", requirement: "recommended", status: "missing", detail: "正文包含表格；请提供可编辑源文件", verification: "file", materialKind: "table", blocking: false, confirmable: false, sourceUrl: null, evidenceExcerpt: null, capturedUnixMs: null, freshUntilUnixMs: null, requiredCount: 1, matchedMaterialIds: [] },
      { id: "common-cover-letter", label: "投稿信", labelEn: "Cover letter", group: "files", requirement: "recommended", status: "recommended", detail: "可上传致编辑的投稿信", verification: "file", materialKind: "cover_letter", blocking: false, confirmable: false, sourceUrl: null, evidenceExcerpt: null, capturedUnixMs: null, freshUntilUnixMs: null, requiredCount: 1, matchedMaterialIds: [] },
      { id: "common-declaration-files", label: "声明文件", labelEn: "Declaration documents", group: "declarations", requirement: "recommended", status: "recommended", detail: "可上传伦理、知情同意、利益冲突、资金、数据可用性、作者贡献或 AI 使用声明", verification: "file", materialKind: "declaration", blocking: false, confirmable: false, sourceUrl: null, evidenceExcerpt: null, capturedUnixMs: null, freshUntilUnixMs: null, requiredCount: 1, matchedMaterialIds: [] },
      { id: "common-bibliography-files", label: "参考文献文件", labelEn: "Bibliography files", group: "files", requirement: "recommended", status: "recommended", detail: "可上传可编辑参考文献文件", verification: "file", materialKind: "bibliography", blocking: false, confirmable: false, sourceUrl: null, evidenceExcerpt: null, capturedUnixMs: null, freshUntilUnixMs: null, requiredCount: 1, matchedMaterialIds: [] },
      { id: "common-supplementary-files", label: "补充材料与研究数据", labelEn: "Supplementary materials and research data", group: "files", requirement: "recommended", status: "recommended", detail: "可上传附录、方法补充、数据、代码归档、演示或音视频", verification: "file", materialKind: "supplementary", blocking: false, confirmable: false, sourceUrl: null, evidenceExcerpt: null, capturedUnixMs: null, freshUntilUnixMs: null, requiredCount: 1, matchedMaterialIds: [] },
      { id: "common-explanation-files", label: "说明、回复与其他支持文件", labelEn: "Explanations, responses, and other supporting files", group: "files", requirement: "recommended", status: "recommended", detail: "可上传情况说明、回复信、报告清单、版权或许可文件、作者协议及其他支持资料", verification: "file", materialKind: "other", blocking: false, confirmable: false, sourceUrl: null, evidenceExcerpt: null, capturedUnixMs: null, freshUntilUnixMs: null, requiredCount: 1, matchedMaterialIds: [] },
    ], recommendationReady: true, targetVerified: true, requiredComplete: materialPresent, targetCheckReady: false, workflowStatus: materialPresent ? "materials_complete_check_required" : "materials_required", requiredTotal: 1, requiredCompleted: materialPresent ? 1 : 0 });
    const makeTarget = (journalId: string, role: "primary" | "backup", priority: number) => ({ schemaVersion: 3, selectionId: `selection-${journalId}-${role}`, workspaceId: workspace.id, selectedAgainstManuscriptVersion: 2, recommendationRunId: `jmr-${runCount}`, journalId, name: journalId.startsWith("i") ? "国际期刊1" : "国内期刊1", nameEn: journalId.startsWith("i") ? "International Journal 1" : "Domestic Journal 1", publisher: "Synthetic Society", region: journalId.startsWith("i") ? "international" : "domestic", rankSystem: "Synthetic CCF", rankTier: journalId.startsWith("i") ? "CCF A" : "T1", homepageUrl: "https://example.test/journal", articleType: "research", planRole: role, priority, selectedUnixMs: Date.UTC(2026,7,30), recordHash: "f".repeat(64), externalTransmission: "not_performed" });
    invokeMock.mockImplementation((command, args) => {
      if (command === "list_workspaces") return Promise.resolve({ workspaces: [workspace], archivedWorkspaces: [], warnings: [] });
      if (command === "get_workspace_lifecycle") return Promise.resolve({ workspaceId: workspace.id, currentVersion: 2, structureReport: null, readinessReport: null, attestation: null, submission: null, knowledgeBody: null, submissionTargetPlan: targetPlan, journalRequirements: null });
      if (command === "get_submission_target_plan") return Promise.resolve(targetPlan);
      if (command === "get_journal_requirement_snapshots") return Promise.resolve(requirementSnapshots);
      if (command === "get_journal_profile_discoveries") return Promise.resolve(profileDiscoveries);
      if (command === "get_submission_materials") return Promise.resolve(requirementSnapshots.length > 0 ? materialCatalog() : { schemaVersion: 1, workspaceId: workspace.id, manuscriptVersion: 2, materials: [], checklist: [], requiredComplete: false, targetCheckReady: false });
      if (command === "delete_submission_material") { materialPresent = false; return Promise.resolve(materialCatalog()); }
      if (command === "add_submission_materials") { materialPresent = true; return Promise.resolve(materialCatalog()); }
      if (command === "get_target_submission_package_plan") return Promise.reject(new Error("package plan not ready in fixture"));
      if (command === "select_recommended_journal") { const journalId = (args as { journalId: string }).journalId; const target = makeTarget(journalId, "primary", 0); targetPlan = { ...targetPlan, primary: target, updatedUnixMs: Date.UTC(2026,7,30) }; return Promise.resolve(target); }
      if (command === "clear_primary_submission_target") { const selectionId = (args as { primarySelectionId: string }).primarySelectionId; if (targetPlan.primary?.selectionId === selectionId) targetPlan = { ...targetPlan, primary: null, updatedUnixMs: Date.UTC(2026,7,30) }; return Promise.resolve(targetPlan); }
      if (command === "add_backup_recommended_journal") { const journalId = (args as { journalId: string }).journalId; targetPlan = { ...targetPlan, backups: [...targetPlan.backups, makeTarget(journalId, "backup", targetPlan.backups.length + 1)] }; return Promise.resolve(targetPlan); }
      if (command === "remove_backup_target") { const selectionId = (args as { backupSelectionId: string }).backupSelectionId; targetPlan = { ...targetPlan, backups: targetPlan.backups.filter((target) => target.selectionId !== selectionId) }; return Promise.resolve(targetPlan); }
      if (command === "promote_backup_target") { const selectionId = (args as { backupSelectionId: string }).backupSelectionId; const backup = targetPlan.backups.find((target) => target.selectionId === selectionId); if (!backup) return Promise.reject(new Error("backup fixture missing")); const journalId = backup.journalId as string; targetPlan = { ...targetPlan, primary: makeTarget(journalId, "primary", 0), backups: targetPlan.backups.filter((target) => target.selectionId !== selectionId), updatedUnixMs: Date.UTC(2026,7,30) }; return Promise.resolve(targetPlan); }
      if (command === "discover_journal_requirements") { const targetSelectionId = (args as { targetSelectionId: string }).targetSelectionId; const snapshot = { schemaVersion: 1, snapshotId: "requirements-1", workspaceId: workspace.id, targetSelectionId, journalId: "dr1", journalName: "国内期刊1", sourceMode: "official_network_fetch", status: "official_sources_captured", sources: [{ url: "https://example.test/journal/guide-for-authors", title: "Guide for authors", contentHash: "a".repeat(64), capturedUnixMs: Date.UTC(2026,7,30), officialHostMatched: true }], requirements: [{ id: "requirement-title-page", category: "title_page", label: "标题页", labelEn: "Title page", obligation: "required", detail: "官方原文含明确义务词", sourceUrl: "https://example.test/journal/guide-for-authors", evidenceExcerpt: "A separate title page is required" }], limitations: [], capturedUnixMs: Date.UTC(2026,7,30), freshUntilUnixMs: Date.UTC(2026,10,30), recordHash: "b".repeat(64), externalTransmission: "author_confirmed_official_source_fetch" }; requirementSnapshots = [snapshot]; return Promise.resolve({ runId: "fetch-test", snapshot, events: [], pending: [], partial: false, options: { approvedOrigins: [], httpOrigins: [] } }); }
      if (command === "discover_journal_profile") { const targetSelectionId = (args as { targetSelectionId: string }).targetSelectionId; const record = { schemaVersion: 1, discoveryId: `jed-${"1".repeat(20)}`, workspaceId: workspace.id, targetSelectionId, journalId: "dr1", journalName: "国内期刊1", issn: "1234-5678", eissn: null, publisher: "Synthetic Society", scopeSummary: "Publishes computer vision and robotics research.", reportedPrintCirculation: null, averageReviewDays: null, submissionToPublicationDays: 120, publicationFrequency: "monthly", apcStatus: "unknown", openAccessStatus: "hybrid", officialHomepageUrl: "https://example.test/journal", aimsScopeUrl: null, authorInstructionsUrl: null, sourceUrls: ["https://example.test/journal"], missingFields: ["eissn", "reported_print_circulation", "average_review_days"], evidenceStatus: "candidate_requires_official_verification", sourceMode: "configured_model_candidate", providerLabel: "Synthetic AI", model: "synthetic-model", externalTransmission: "author_confirmed_public_journal_identity_only", createdUnixMs: Date.UTC(2026,7,30) }; profileDiscoveries = [record]; return Promise.resolve(record); }
      if (command === "list_journal_recommendations") return Promise.resolve(recommendationRuns);
      if (command === "save_journal_recommendation_profile") { const profile = (args as { profile: Record<string, string> }).profile; return Promise.resolve({ ...profile, schemaVersion: 1, profileId: `jmp-${"a".repeat(20)}`, profileVersion: runCount + 1, workspaceId: workspace.id, savedUnixMs: Date.UTC(2026,7,30), institutionRuleEvidence: { status: "search_required", ruleSetId: null, ruleSetVersion: null, sourceUrls: [], verifiedAt: null, recognizedRankTiers: [], blockedRankTiers: [] }, externalTransmission: "not_performed" }); }
      if (command === "recommend_journals") { runCount += 1; const recommendationProfile = { authorName: "", institution: "", specialty: "", manuscriptPurpose: "academic_communication", submissionDeadline: "2099-12-31", schemaVersion: 1, profileId: `jmp-${"a".repeat(20)}`, profileVersion: runCount, workspaceId: workspace.id, savedUnixMs: Date.UTC(2026,7,30), institutionRuleEvidence: { status: "search_required", ruleSetId: null, ruleSetVersion: null, sourceUrls: [], verifiedAt: null, recognizedRankTiers: [], blockedRankTiers: [] }, externalTransmission: "not_performed" }; const makePortfolio = (prefix: string, domestic: boolean) => ({ sprint: [1,2].map((index)=>makeItem(`${prefix}r${index}`,domestic,index)), matching: [1,2,3].map((index)=>makeItem(`${prefix}m${index}`,domestic,index)), safeguard: [1,2,3].map((index)=>makeItem(`${prefix}s${index}`,domestic,index)) }); const result = { schemaVersion: 6, runId: `jmr-${runCount}`, workspaceId: workspace.id, manuscriptVersion: 2, resolvedArticleType: "research", catalogVersion: "computer-ai-2025.1", catalogVerifiedDate: "2025-04-16", evaluatedUnixMs: Date.UTC(2026,7,30), recommendationProfile, deadlineDaysRemaining: 120, domestic: makePortfolio("d",true), international: makePortfolio("i",false), schoolRuleStatus: "official_source_search_required_excluded_from_score", institutionDirectoryStatus: "local_directory_not_imported", journalDirectoryVersion: null, limitations: ["不是录用概率"], externalTransmission: "not_performed" }; recommendationRuns = [result, ...recommendationRuns]; return Promise.resolve(result); }
      return Promise.reject(new Error(`unexpected command ${command}`));
    });
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "打开 vision-study.tex" }));
    await user.click(within(screen.getByRole("navigation", { name: "投稿准备主任务" })).getByRole("button", { name: /目标期刊/ }));
    expect(screen.getByText(/学校规则需要正式来源/)).toBeVisible();
    const calculateButton = screen.getByRole("button", { name: "根据当前论文生成初步推荐" });
    expect(calculateButton).toBeEnabled();
    expect(screen.getByRole("heading", { name: "提供学校正式要求" })).toBeVisible();
    expect(screen.getByText(/作者姓名、来源网址、联系方式、学号和论文正文均不发送/)).toBeVisible();
    await user.click(screen.getByRole("button", { name: "EN" }));
    expect(screen.getByText(/Generating authorizes this model extraction once/)).toBeVisible();
    await user.click(screen.getByRole("button", { name: "中文" }));
    await user.click(calculateButton);
    expect(await screen.findByRole("heading", { name: "已保存推荐 · 1 条" })).toBeVisible();
    expect(screen.getByText("期刊对应出版社")).toBeVisible();
    expect(screen.getByRole("button", { name: /查看推荐记录 jmr-1/ })).toHaveAttribute("aria-pressed", "true");
    expect(await screen.findByRole("heading", { name: "中国期刊与出版社 · 8 家" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "全球期刊与出版社 · 8 家" })).toBeVisible();
    const chinaTargetMap = screen.getByRole("region", { name: "中国期刊与出版社推荐靶图" });
    expect(within(chinaTargetMap).getAllByRole("button")).toHaveLength(8);
    const firstReachPoint = within(chinaTargetMap).getByRole("button", { name: /R1/ });
    await user.click(firstReachPoint);
    expect(firstReachPoint).toHaveAttribute("aria-pressed", "true");
    expect(screen.getAllByRole("heading", { name: "出版社与期刊资料" })).toHaveLength(2);
    expect(screen.getAllByRole("heading", { name: "冲刺型 · 2 家" })).toHaveLength(2);
    expect(screen.getAllByRole("heading", { name: "匹配型 · 3 家" })).toHaveLength(2);
    expect(screen.getAllByRole("heading", { name: "保底型 · 3 家" })).toHaveLength(2);
    expect(screen.getByText(/学校规则尚未核验/)).toBeVisible();
    expect(screen.queryByText(/主题范围适配 100 分/)).not.toBeInTheDocument();
    expect(screen.queryByText(/校规 24/)).not.toBeInTheDocument();
    expect(screen.queryByText(/LOCAL FIT/)).not.toBeInTheDocument();
    expect(screen.getAllByText(/jmr-1/)).toHaveLength(2);
    await user.selectOptions(screen.getByLabelText("研究方向"), "natural_language_processing");
    expect(screen.getByRole("button", { name: /查看推荐记录 jmr-1/ })).toBeVisible();
    expect(screen.queryByText("本地推荐记录")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "根据当前论文生成初步推荐" }));
    expect(await screen.findByRole("button", { name: /查看推荐记录 jmr-2/ })).toHaveAttribute("aria-pressed", "true");
    expect(invokeMock).toHaveBeenCalledWith("recommend_journals", { workspaceId: workspace.id, profileId: `jmp-${"a".repeat(20)}`, preferences: expect.objectContaining({ topic: "natural_language_processing" }) });
    expect(invokeMock).toHaveBeenCalledWith("save_journal_recommendation_profile", { workspaceId: workspace.id, profile: expect.objectContaining({ authorName: "", institution: "", specialty: "", manuscriptPurpose: "academic_communication" }) });
    await user.click(screen.getAllByRole("button", { name: "设为投稿目标" })[0]);
    const primaryRoute = await screen.findByRole("article", { name: /当前投稿主线/ });
    expect(within(primaryRoute).getByText("唯一激活")).toBeVisible();
    expect(screen.getByRole("heading", { name: "核对目标期刊画像" })).toBeVisible();
    expect(screen.getByText(/点击即授权这一次受限发现/)).toBeVisible();
    expect(screen.queryByRole("checkbox", { name: /允许发送期刊名/ })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "EN" }));
    expect(screen.getByText(/Clicking authorizes this restricted discovery once/)).toBeVisible();
    await user.click(screen.getByRole("button", { name: "中文" }));
    await user.click(screen.getByRole("button", { name: "核对本地；缺失时发现" }));
    expect(await screen.findByText("模型线索 · 待核验")).toBeVisible();
    expect(screen.getByText(/模型输出只是待核验线索/)).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("discover_journal_profile", { workspaceId: workspace.id, targetSelectionId: "selection-dr1-primary", authorConfirmedExternalTransmission: true });
    await user.click(screen.getAllByRole("button", { name: "加入备选支线" })[0]);
    expect(await screen.findByRole("article", { name: /备选投稿支线/ })).toBeVisible();
    await user.click(screen.getByRole("button", { name: "取消备选" }));
    expect(screen.queryByRole("article", { name: /备选投稿支线/ })).not.toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("remove_backup_target", { workspaceId: workspace.id, backupSelectionId: "selection-dr2-backup" });
    await user.click(screen.getAllByRole("button", { name: "加入备选支线" })[0]);
    expect(await screen.findByRole("article", { name: /备选投稿支线/ })).toBeVisible();
    const refreshedPrimary = screen.getByRole("article", { name: /当前投稿主线/ });
    await user.click(within(refreshedPrimary).getByLabelText(/仅本次允许后端读取/));
    await user.click(within(refreshedPrimary).getByRole("button", { name: "获取官方投稿要求" }));
    expect(await within(refreshedPrimary).findByText("已建立期刊专属要求快照")).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("discover_journal_requirements", { workspaceId: workspace.id, targetSelectionId: "selection-dr1-primary", authorConfirmedExternalTransmission: true, options: { approvedOrigins: [], httpOrigins: [] } });
    await user.click(screen.getByRole("button", { name: "按要求准备投稿资料" }));
    expect(await screen.findByRole("heading", { name: "按目标期刊组织投稿资料" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "投稿包准备树" })).toBeVisible();
    expect(screen.getByText("正文扫描 1 幅 · 已上传 0 个")).toBeVisible();
    expect(screen.getByText("正文扫描 1 个 · 已上传 0 个")).toBeVisible();
    expect(screen.getAllByText("少 1 个")).toHaveLength(2);
    expect(screen.getByText("当前准备包 · 2 个拟组包文件")).toBeVisible();
    expect(screen.getByText("必需项已达标")).toBeVisible();
    expect(screen.getByText("AI 语义与语法审计 · 后续迭代")).toBeVisible();
    const materialViewTabs = screen.getByRole("tablist", { name: "投稿资料查看模式" });
    expect(within(materialViewTabs).getAllByRole("tab")).toHaveLength(4);
    const overviewTab = within(materialViewTabs).getByRole("tab", { name: /准备概览/ });
    expect(overviewTab).toHaveAttribute("aria-selected", "true");
    expect(screen.queryByRole("heading", { name: "图表文件" })).not.toBeInTheDocument();
    overviewTab.focus();
    await user.keyboard("{ArrowRight}");
    expect(within(materialViewTabs).getByRole("tab", { name: /要求清单/ })).toHaveAttribute("aria-selected", "true");
    await user.click(within(materialViewTabs).getByRole("tab", { name: /上传资料/ }));
    expect(within(materialViewTabs).getByRole("tab", { name: /上传资料/ })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByText(/发现同名文件后，必须确认才会替换工作区副本/)).toBeVisible();
    expect(screen.getByRole("heading", { name: "图表文件" })).toBeVisible();
    expect(screen.getByRole("button", { name: "为原始图件上传原始图件" })).toHaveTextContent("选择图片文件");
    expect(screen.getByRole("button", { name: "为可编辑表格上传可编辑表格" })).toHaveTextContent("选择表格文件");
    expect(screen.getAllByText(/CSV、TSV、XLS、XLSX、ODS/).length).toBeGreaterThan(0);
    expect(screen.getByRole("heading", { name: "常见投稿附件" })).toBeVisible();
    expect(screen.getByText("按需补充 · 不默认设为必需")).toBeVisible();
    expect(screen.getByRole("button", { name: "上传声明文件" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "上传说明、回复与其他支持文件" })).toBeEnabled();
    expect(screen.getAllByText(/· 可选/).length).toBeGreaterThanOrEqual(5);
    await user.click(screen.getByRole("button", { name: "为原始图件上传原始图件" }));
    expect(invokeMock).toHaveBeenCalledWith("add_submission_materials", { workspaceId: workspace.id, kind: "figure", checklistItemId: "figure-originals", locale: "zh-CN" });
    await waitFor(() => expect(screen.getByRole("button", { name: "为可编辑表格上传可编辑表格" })).toBeEnabled());
    await user.click(screen.getByRole("button", { name: "为可编辑表格上传可编辑表格" }));
    expect(invokeMock).toHaveBeenCalledWith("add_submission_materials", { workspaceId: workspace.id, kind: "table", checklistItemId: "table-editables", locale: "zh-CN" });
    await waitFor(() => expect(screen.getByRole("button", { name: "上传声明文件" })).toBeEnabled());
    await user.click(screen.getByRole("button", { name: "上传声明文件" }));
    expect(invokeMock).toHaveBeenCalledWith("add_submission_materials", { workspaceId: workspace.id, kind: "declaration", checklistItemId: "common-declaration-files", locale: "zh-CN" });
    await waitFor(() => expect(screen.getByRole("button", { name: "上传说明、回复与其他支持文件" })).toBeEnabled());
    await user.click(screen.getByRole("button", { name: "上传说明、回复与其他支持文件" }));
    expect(invokeMock).toHaveBeenCalledWith("add_submission_materials", { workspaceId: workspace.id, kind: "other", checklistItemId: "common-explanation-files", locale: "zh-CN" });
    await user.click(within(materialViewTabs).getByRole("tab", { name: /已存文件/ }));
    expect(screen.getByText("title-page.docx")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "删除附件 title-page.docx" }));
    expect(screen.getByText("删除工作区中的附件副本？")).toBeVisible();
    await user.click(within(screen.getByRole("group", { name: "确认删除 title-page.docx" })).getByRole("button", { name: "取消" }));
    expect(screen.getByText("title-page.docx")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "删除附件 title-page.docx" }));
    await user.click(within(screen.getByRole("group", { name: "确认删除 title-page.docx" })).getByRole("button", { name: "确认删除附件" }));
    await user.click(within(materialViewTabs).getByRole("tab", { name: /要求清单/ }));
    expect(await screen.findByText("请上传标题页")).toBeVisible();
    expect(screen.queryByText("title-page.docx")).not.toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("delete_submission_material", { workspaceId: workspace.id, materialId: storedMaterial.materialId, authorConfirmed: true });
    await user.click(screen.getByRole("button", { name: "为标题页添加文件" }));
    await user.click(within(materialViewTabs).getByRole("tab", { name: /已存文件/ }));
    expect(await screen.findByText("title-page.docx")).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("add_submission_materials", { workspaceId: workspace.id, kind: "title_page", checklistItemId: "journal-title-page", locale: "zh-CN" });
    expect(screen.queryByText("按目标期刊重新检查")).not.toBeInTheDocument();
    expect(screen.queryByText("补齐材料后运行一次与当前目标绑定的投稿检查")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "进入目标检查" })).toBeVisible();
    expect(screen.getByText("目标检查是材料完成后的独立步骤，不属于待上传材料。进入“检查与修订”后，系统会将检查报告绑定到当前稿件版本、目标期刊和官方要求。")).toBeVisible();
    await user.click(within(materialViewTabs).getByRole("tab", { name: /上传资料/ }));
    await user.click(screen.getByRole("button", { name: "EN" }));
    expect(screen.getByRole("heading", { name: "Common submission attachments" })).toBeVisible();
    expect(screen.getByText(/Replacing an existing workspace copy always requires confirmation/)).toBeVisible();
    expect(screen.getByRole("button", { name: "Upload Declaration documents" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Upload Explanations, responses, and other supporting files" })).toBeEnabled();
    await user.click(within(materialViewTabs).getByRole("tab", { name: /Overview/ }));
    expect(screen.getByRole("heading", { name: "Submission package preparation tree" })).toBeVisible();
    expect(screen.getByText("Scanned 1 figure(s) · 0 uploaded")).toBeVisible();
    expect(screen.getByText("Scanned 1 table(s) · 0 uploaded")).toBeVisible();
    expect(screen.getByRole("heading", { name: "Continue to target checks" })).toBeVisible();
    expect(screen.getByRole("button", { name: /Continue to manuscript checks/ })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "中文" }));
    await user.click(screen.getByRole("button", { name: /继续检查当前稿件/ }));
    expect(await screen.findByRole("heading", { name: "先建立论文结构" })).toBeVisible();
    await user.click(within(screen.getByRole("navigation", { name: "投稿准备主任务" })).getByRole("button", { name: /目标期刊/ }));
    await user.click(screen.getByRole("button", { name: "取消主选期刊" }));
    const clearFromRecommendation = screen.getByRole("group", { name: /确认取消主选期刊/ });
    expect(within(clearFromRecommendation).getByText("取消这家主选期刊？")).toBeVisible();
    await user.click(within(clearFromRecommendation).getByRole("button", { name: "返回" }));
    expect(screen.getByRole("button", { name: "取消主选期刊" })).toBeVisible();
    await user.click(screen.getByRole("button", { name: "取消主选期刊" }));
    await user.click(within(screen.getByRole("group", { name: /确认取消主选期刊/ })).getByRole("button", { name: "确认取消主选" }));
    expect(screen.queryByRole("article", { name: /当前投稿主线/ })).not.toBeInTheDocument();
    expect(screen.getByText("当前没有激活的投稿主线")).toBeVisible();
    const suggestedBackup = screen.getByRole("article", { name: /备选投稿支线/ });
    expect(within(suggestedBackup).getByText("建议下一主线")).toBeVisible();
    expect(screen.getAllByRole("button", { name: "设为投稿目标" }).length).toBeGreaterThan(0);
    expect(invokeMock).toHaveBeenCalledWith("clear_primary_submission_target", { workspaceId: workspace.id, primarySelectionId: "selection-dr1-primary", authorConfirmed: true });
    await user.click(within(suggestedBackup).getByRole("button", { name: "设为当前主线" }));
    expect(await screen.findByRole("article", { name: /当前投稿主线/ })).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("promote_backup_target", { workspaceId: workspace.id, backupSelectionId: "selection-dr2-backup", reason: "not_submitted" });
  });
});
