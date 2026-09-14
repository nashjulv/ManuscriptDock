import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import App from "./App";

const invokeMock = vi.fn();
const jobEvent = vi.hoisted(() => ({
  handler: null as null | ((event: { payload: any }) => void),
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(
    async (_name: string, handler: (event: { payload: any }) => void) => {
      jobEvent.handler = handler;
      return () => {
        jobEvent.handler = null;
      };
    },
  ),
}));

const baseFacts = {
  sourceHash: "hash",
  extractorVersion: "docx-local-v1",
  title: "A Local AI Study",
  abstractText: "Abstract",
  keywords: ["machine learning"],
  language: "en",
  articleType: "research_article",
  authors: [],
  affiliations: [],
  correspondingEmail: null,
  conflictOfInterest: null,
  funding: null,
  dataAvailability: null,
  ethicsStatement: null,
  highlights: [],
  creditContributions: null,
  generativeAiDisclosure: null,
  authorConfirmedFields: [],
};
let project: any;
let folderItems: any[] | null;
let recentItems: any[];
let removedItems: any[];
let manuscriptSelection: any;
let openProjectResult: Promise<any> | null;
const journal = {
  id: "elsevier-artificial-intelligence",
  displayName: "Artificial Intelligence",
  publisher: "Elsevier",
  issn: "0004-3702",
  eissn: "1872-7921",
  aliases: ["AIJ"],
  languages: ["en"],
  articleTypes: ["research_article"],
  topics: ["artificial intelligence"],
  indexing: ["Scopus"],
  publicationRoute: "hybrid",
  apcAmount: null,
  apcCurrency: null,
  firstDecisionDays: null,
  generationCoverage: "supported",
  status: "active",
  verifiedAt: "2026-09-14",
  evidence: [
    {
      sourceUrl: "https://example.test/guide",
      label: { zhCn: "官方作者指南", en: "Official guide for authors" },
      verifiedAt: "2026-09-14",
    },
  ],
};

beforeEach(() => {
  window.localStorage.clear();
  window.localStorage.setItem("manuscriptdock.locale", "zh-CN");
  project = {
    id: "project-1",
    schemaVersion: 1,
    revision: 1,
    displayName: "A Local AI Study",
    activeSource: {
      id: "source-1",
      fileName: "study.docx",
      sha256: "hash",
      format: "docx",
      sizeBytes: 1000,
      createdAtUnixMs: Date.UTC(2026, 8, 14),
      featureProfile: "d1",
    },
    facts: structuredClone(baseFacts),
    materials: [],
    target: null,
    authorDecisions: [],
    lastTask: "find_journals",
    updatedAtUnixMs: Date.UTC(2026, 8, 14),
  };
  folderItems = null;
  recentItems = [];
  removedItems = [];
  manuscriptSelection = {
    token: "safe-token",
    name: "study.docx",
    extension: "docx",
    sizeBytes: 1000,
    kind: "manuscript",
  };
  openProjectResult = null;
  jobEvent.handler = null;
  invokeMock.mockReset().mockImplementation((command: string, args?: any) => {
    if (command === "list_recent_projects") return Promise.resolve(recentItems);
    if (command === "list_removed_projects") return Promise.resolve(removedItems);
    if (command === "hide_recent_project") {
      const hidden = recentItems.find((item) => item.id === args.projectId);
      recentItems = recentItems.filter((item) => item.id !== args.projectId);
      if (hidden) removedItems = [hidden, ...removedItems];
      return Promise.resolve(hidden);
    }
    if (command === "restore_recent_project") {
      const restored = removedItems.find((item) => item.id === args.projectId);
      removedItems = removedItems.filter((item) => item.id !== args.projectId);
      if (restored) recentItems = [restored, ...recentItems];
      return Promise.resolve(restored);
    }
    if (command === "choose_local_inputs")
      return Promise.resolve({
        status: "selected",
        folder:
          args.kind === "folder"
            ? { token: "folder-token", name: "Study Folder" }
            : undefined,
        items:
          args.kind === "folder" && folderItems
            ? folderItems
            : [{ ...manuscriptSelection, kind: args.kind }],
      });
    if (command === "open_project") {
      if (openProjectResult) return openProjectResult;
      return Promise.resolve(
        args.folderToken
          ? {
              ...project,
              displayName: "Study Folder",
              workspace: {
                kind: "folder",
                name: "Study Folder",
                bindingId: "binding-id",
              },
            }
          : project,
      );
    }
    if (command === "recommend_journals")
      if (args.constraints.requiredLanguage === "zh-CN")
        return Promise.resolve({
          runId: "run-zh",
          catalogVersion: "2026.1",
          recommendations: [],
          needsVerification: [],
          excludedCount: 12,
        });
      else
      return Promise.resolve({
        runId: "run-1",
        catalogVersion: "2026.1",
        recommendations: [
          {
            journal,
            role: "best_overall_fit",
            reasons: [{ zhCn: "关键词相符", en: "Keywords match" }],
            risks: [],
            preparation: [
              { zhCn: "补齐作者事实", en: "Complete author facts" },
            ],
            constraints: [
              {
                constraintId: "indexing",
                status: "pass",
                explanation: {
                  zhCn: "满足名单要求",
                  en: "Meets indexing requirements",
                },
              },
            ],
            evidenceCoverage: 100,
          },
        ],
        needsVerification: [],
        excludedCount: 0,
      });
    if (command === "select_target") {
      project = {
        ...project,
        revision: project.revision + 1,
        lastTask: "prepare_package",
        target: {
          id: "target-1",
          journalId: journal.id,
          articleType: "research_article",
          stage: "initial_submission",
          origin: args.input.origin,
          recommendationRef: args.input.recommendationRef,
          rulesHash: "rules",
        },
      };
      return Promise.resolve(project);
    }
    if (command === "add_material") {
      project = {
        ...project,
        revision: project.revision + 1,
        materials: [
          ...project.materials,
          {
            id: "material-1",
            fileName: "figure.png",
            sha256: "figure",
            sizeBytes: 300,
            kind: args.kind,
            included: false,
          },
        ],
      };
      return Promise.resolve(project);
    }
    if (command === "list_journals")
      return Promise.resolve([
        journal,
        {
          ...journal,
          id: "catalog-only",
          displayName: "Catalog Only Journal",
          generationCoverage: "catalog_only",
        },
      ]);
    if (command === "list_material_tasks") return Promise.resolve([]);
    if (command === "list_ai_runs") return Promise.resolve([]);
    if (command === "list_package_workspace") return Promise.resolve({ rootPath: "/local/manuscript/Submission Package", entries: [] });
    if (command === "get_preparation")
      return Promise.resolve({
        projectId: project.id,
        revision: project.revision,
        contextHash: "context",
        status: "draft_ready",
        blockers: [
          {
            requirementId: "authors",
            label: { zhCn: "作者与顺序", en: "Authors and order" },
            description: { zhCn: "请确认作者", en: "Confirm authors" },
            action: "fill",
            status: "missing",
            required: true,
            evidence: journal.evidence[0],
          },
        ],
        warnings: [],
        readyItems: [],
        allowedActions: ["build_draft"],
        packagePlan: {
          id: "plan-1",
          contextHash: "context",
          filePlan: [
            {
              relativePath: "submission/manuscript.docx",
              operation: "copy_source_unchanged",
              publisherFile: true,
            },
          ],
          transformPlan: [],
          capabilities: ["copy_source_unchanged"],
          status: "blocked",
        },
      });
    if (command === "build_package")
      return Promise.resolve({
        id: "package-1",
        projectId: project.id,
        contextHash: "context",
        mode: args.mode,
        files: [],
        validationPassed: true,
        warnings: [],
      });
    if (
      command === "choose_export_folder" ||
      command === "choose_project_folder_for_export"
    )
      return Promise.resolve({
        status: "selected",
        items: [
          {
            token: "destination-token",
            name: "export",
            extension: "",
            sizeBytes: 0,
            kind: "destination",
          },
        ],
      });
    if (command === "export_package")
      return Promise.resolve({
        id: "receipt-1",
        packageId: "package-1",
        outputDirectory: "/safe/export",
        fileCount: 12,
        packageHash: "package-hash",
        finishedAtUnixMs: Date.UTC(2026, 8, 14),
        recordPersisted: false,
      });
    if (command === "cancel_job")
      return Promise.resolve({
        id: args.jobId,
        projectId: args.projectId,
        status: "cancelled",
      });
    return Promise.reject({
      code: `UNEXPECTED_${command}`,
      retryable: false,
      diagnosticId: "test",
    });
  });
});

it("asks which DOCX is the manuscript when a folder contains multiple candidates", async () => {
  folderItems = [
    {
      token: "draft-token",
      name: "draft.docx",
      extension: "docx",
      sizeBytes: 800,
      kind: "manuscript",
    },
    {
      token: "final-token",
      name: "final.docx",
      extension: "docx",
      sizeBytes: 900,
      kind: "manuscript",
    },
    {
      token: "figure-token",
      name: "figure.png",
      extension: "png",
      sizeBytes: 300,
      kind: "material",
    },
  ];
  const user = userEvent.setup();
  render(<App />);
  await user.click(await screen.findByRole("button", { name: /整理投稿包/ }));
  await user.click(screen.getByRole("button", { name: "打开材料文件夹" }));
  expect(
    await screen.findByRole("dialog", { name: "哪一份是本次主稿？" }),
  ).toBeVisible();
  await user.click(screen.getByRole("button", { name: /final\.docx/ }));
  expect(invokeMock).toHaveBeenCalledWith(
    "open_project",
    expect.objectContaining({
      manuscriptToken: "final-token",
      folderToken: "folder-token",
    }),
  );
  expect(invokeMock).toHaveBeenCalledWith(
    "add_material",
    expect.objectContaining({ token: "figure-token", kind: "unclassified" }),
  );
});

it("shows two clear bilingual tasks and local processing boundary", async () => {
  const user = userEvent.setup();
  render(<App />);
  expect(await screen.findByRole("button", { name: /推荐期刊/ })).toBeVisible();
  expect(screen.getByRole("button", { name: /整理投稿包/ })).toBeVisible();
  expect(screen.getByText(/基础功能无需配置模型/)).toBeVisible();
  await user.click(screen.getByRole("button", { name: "EN" }));
  expect(screen.getByRole("button", { name: /Find journals/ })).toBeVisible();
  expect(
    screen.getByRole("button", { name: /Prepare submission package/ }),
  ).toBeVisible();
  expect(document.documentElement.lang).toBe("en");
});

for (const locale of ["zh-CN", "en"] as const) {
  for (const code of ["PDF_TEXT_EXTRACTION_FAILED", "JOB_WORKER_PANICKED"]) {
    it(`releases loading and allows another file after ${code} in ${locale}`, async () => {
      localStorage.setItem("manuscriptdock.locale", locale);
      const originalInvoke = invokeMock.getMockImplementation()!;
      let firstOpen = true;
      const requestId = crypto.randomUUID();
      const queued = {
        id: requestId, requestId, operation: "open_project", status: "queued", events: [],
      };
      invokeMock.mockImplementation((command: string, args: any) => {
        if (command === "open_project" && firstOpen) {
          firstOpen = false;
          return Promise.resolve(queued);
        }
        if (command === "get_job") return Promise.resolve({
          ...queued, status: code === "JOB_WORKER_PANICKED" ? "failed" : "needs_input",
          events: [{ error: { code, retryable: false, diagnosticId: "synthetic-failure" } }],
        });
        return originalInvoke(command, args);
      });
      const user = userEvent.setup();
      render(<App />);
      await user.click(screen.getByRole("button", { name: locale === "en" ? /Find journals/ : /推荐期刊/ }));
      const openLabel = locale === "en" ? "Open local manuscript" : "打开本地论文";
      await user.click(screen.getByRole("button", { name: openLabel }));
      const message = code === "PDF_TEXT_EXTRACTION_FAILED"
        ? locale === "en" ? /font encoding/ : /字体编码/
        : locale === "en" ? /task has stopped/ : /任务已停止/;
      expect(await screen.findByText(message)).toBeVisible();
      expect(screen.getByRole("button", { name: openLabel })).toBeEnabled();
      await user.click(screen.getByRole("button", { name: openLabel }));
      expect(await screen.findByLabelText(locale === "en" ? "Research keywords" : "研究关键词")).toBeVisible();
      expect(screen.queryByText(message)).not.toBeInTheDocument();
    });
  }
}

for (const locale of ["zh-CN", "en"] as const) {
  it(`returns from both task pages with the visible Home button in ${locale}`, async () => {
    localStorage.setItem("manuscriptdock.locale", locale);
    const user = userEvent.setup();
    render(<App />);
    const home = screen.getByRole("button", { name: locale === "en" ? "Home" : "首页" });
    expect(home).toHaveAttribute("aria-current", "page");
    for (const task of locale === "en" ? [/Find journals/, /Prepare submission package/] : [/推荐期刊/, /整理投稿包/]) {
      await user.click(await screen.findByRole("button", { name: task }));
      expect(home).not.toHaveAttribute("aria-current");
      await user.click(home);
      expect(home).toHaveAttribute("aria-current", "page");
      expect(screen.getByRole("button", { name: locale === "en" ? /Find journals/ : /推荐期刊/ })).toBeVisible();
    }
    expect(invokeMock).not.toHaveBeenCalledWith("hide_recent_project", expect.anything());
  });
  it(`keeps edited inputs when returning home is cancelled in ${locale}`, async () => {
    localStorage.setItem("manuscriptdock.locale", locale);
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: locale === "en" ? /Find journals/ : /推荐期刊/ }));
    await user.click(screen.getByRole("button", { name: locale === "en" ? "Open local manuscript" : "打开本地论文" }));
    const keywords = await screen.findByLabelText(locale === "en" ? "Research keywords" : "研究关键词");
    await user.type(keywords, ", new topic");
    await user.click(screen.getByRole("button", { name: locale === "en" ? "Home" : "首页" }));
    expect(screen.getByRole("alertdialog")).toHaveTextContent(locale === "en" ? "Unsaved" : "未保存");
    await user.click(screen.getByRole("button", { name: locale === "en" ? "Continue editing" : "继续编辑" }));
    expect((keywords as HTMLInputElement).value).toContain("new topic");
    await user.click(screen.getByRole("button", { name: locale === "en" ? "Home" : "首页" }));
    await user.click(screen.getByRole("button", { name: locale === "en" ? "Discard input and go home" : "放弃输入并返回首页" }));
    expect(screen.getByRole("button", { name: locale === "en" ? "Home" : "首页" })).toHaveAttribute("aria-current", "page");
  });
}

it("explains in both locales that the same folder reopens one project record", async () => {
  const user = userEvent.setup();
  render(<App />);
  await user.click(await screen.findByRole("button", { name: /推荐期刊/ }));
  expect(screen.getByText(/同一文件夹再次打开会回到同一个项目记录/)).toBeVisible();
  await user.click(screen.getByRole("button", { name: "EN" }));
  expect(screen.getByText(/reopens the same project record/)).toBeVisible();
});

it("shows a bilingual cancel action while a dispatched local job is active", async () => {
  let resolveOpen: (value: any) => void = () => undefined;
  openProjectResult = new Promise((resolve) => {
    resolveOpen = resolve;
  });
  const user = userEvent.setup();
  render(<App />);
  await user.click(await screen.findByRole("button", { name: /推荐期刊/ }));
  await user.click(screen.getByRole("button", { name: "打开本地论文" }));
  await vi.waitFor(() => expect(jobEvent.handler).not.toBeNull());
  act(() => {
    jobEvent.handler?.({
      payload: {
        schemaVersion: 1,
        id: "job-1",
        requestId: "request-1",
        operation: "open_project",
        status: "running",
        cancelRequested: false,
        events: [],
        updatedAtUnixMs: 1,
      },
    });
  });
  await user.click(screen.getByRole("button", { name: "取消任务" }));
  expect(invokeMock).toHaveBeenCalledWith("cancel_job", {
    projectId: undefined,
    jobId: "job-1",
  });
  await user.click(screen.getByRole("button", { name: "EN" }));
  expect(screen.getByRole("button", { name: "Cancel task" })).toBeVisible();
  await act(async () => resolveOpen(project));
});

it("does not expose or query the never-shipped earlier-version task flow", async () => {
  const user = userEvent.setup();
  render(<App />);
  expect(await screen.findByText("最近打开")).toBeVisible();
  expect(screen.queryByText("早期版本任务")).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "EN" }));
  expect(screen.queryByText("Earlier-version tasks")).not.toBeInTheDocument();
  expect(
    invokeMock.mock.calls.some(([command]) =>
      ["list_legacy_projects", "import_legacy_project"].includes(command),
    ),
  ).toBe(false);
});

it("prefills essential matching fields and keeps optional constraints under bilingual more settings", async () => {
  project.facts.language = "zh-CN";
  const user = userEvent.setup();
  render(<App />);
  await user.click(await screen.findByRole("button", { name: /推荐期刊/ }));
  await user.click(screen.getByRole("button", { name: "打开本地论文" }));
  expect(await screen.findByLabelText("投稿语言")).toHaveValue("");
  expect(screen.getByLabelText("文章类型")).toHaveValue("research_article");
  expect(screen.getByLabelText("研究关键词")).toHaveValue("machine learning");
  expect(screen.getByLabelText("最高 APC（可选）")).not.toBeVisible();
  expect(
    screen.getByLabelText("优先首轮决定较快的期刊（偏好，不是硬条件）"),
  ).not.toBeVisible();

  await user.click(screen.getByText("更多设置"));
  expect(screen.getByLabelText("最高 APC（可选）")).toBeVisible();
  expect(screen.getByLabelText("币种")).toHaveValue("USD");
  expect(screen.getByLabelText("必须提供开放获取路径")).not.toBeChecked();
  expect(
    screen.getByLabelText("优先首轮决定较快的期刊（偏好，不是硬条件）"),
  ).toBeChecked();

  await user.click(screen.getByRole("button", { name: "EN" }));
  expect(screen.getByLabelText("Submission language")).toHaveValue("");
  expect(screen.getByLabelText("Article type")).toHaveValue("research_article");
  expect(screen.getByLabelText("Research keywords")).toHaveValue(
    "machine learning",
  );
  expect(screen.getByText("More settings")).toBeVisible();
});

it("keeps a manually changed submission language when the client locale changes", async () => {
  const user = userEvent.setup();
  render(<App />);
  await user.click(await screen.findByRole("button", { name: /推荐期刊/ }));
  await user.click(screen.getByRole("button", { name: "打开本地论文" }));
  await user.selectOptions(await screen.findByLabelText("投稿语言"), "");
  await user.click(screen.getByRole("button", { name: "EN" }));
  expect(screen.getByLabelText("Submission language")).toHaveValue("");
});

it("explains the Chinese catalog gap and retries in English only after confirmation", async () => {
  const user = userEvent.setup();
  render(<App />);
  await user.click(await screen.findByRole("button", { name: /推荐期刊/ }));
  await user.click(screen.getByRole("button", { name: "打开本地论文" }));
  expect(await screen.findByLabelText("投稿语言")).toHaveValue("");
  await user.selectOptions(screen.getByLabelText("投稿语言"), "zh-CN");
  await user.click(screen.getByRole("button", { name: "推荐期刊" }));
  expect(
    await screen.findByText(/还没有已核验、可生成投稿包的简体中文期刊/),
  ).toBeVisible();
  expect(invokeMock).toHaveBeenLastCalledWith(
    "recommend_journals",
    expect.objectContaining({
      constraints: expect.objectContaining({ requiredLanguage: "zh-CN" }),
    }),
  );
  await user.click(screen.getByRole("button", { name: "EN" }));
  expect(screen.getByText(/does not yet contain a verified/)).toBeVisible();
  await user.click(screen.getByRole("button", { name: "Switch to English and retry" }));
  expect(
    await screen.findByRole("heading", { name: "Artificial Intelligence" }),
  ).toBeVisible();
  expect(invokeMock).toHaveBeenLastCalledWith(
    "recommend_journals",
    expect.objectContaining({
      constraints: expect.objectContaining({ requiredLanguage: "en" }),
    }),
  );
});

it("runs manuscript to recommendation to preparation without a second file selection", async () => {
  const user = userEvent.setup();
  render(<App />);
  await user.click(await screen.findByRole("button", { name: /推荐期刊/ }));
  await user.click(screen.getByRole("button", { name: "打开本地论文" }));
  expect(
    await screen.findByRole("heading", { name: "这次投稿最看重什么？" }),
  ).toBeVisible();
  await user.clear(screen.getByLabelText("研究关键词"));
  await user.type(screen.getByLabelText("研究关键词"), "machine learning");
  await user.click(screen.getByText("更多设置"));
  await user.click(screen.getByLabelText("必须被 SCIE 收录"));
  await user.selectOptions(screen.getByLabelText("投稿语言"), "en");
  expect(
    screen.getByLabelText("优先首轮决定较快的期刊（偏好，不是硬条件）"),
  ).toBeChecked();
  await user.click(screen.getByRole("button", { name: "推荐期刊" }));
  expect(
    await screen.findByRole("heading", { name: "Artificial Intelligence" }),
  ).toBeVisible();
  expect(invokeMock).toHaveBeenCalledWith(
    "recommend_journals",
    expect.objectContaining({
      constraints: expect.objectContaining({
        requiredIndexing: ["SCIE"],
        preferFastFirstDecision: true,
      }),
    }),
  );
  await user.click(screen.getByText("查看依据"));
  expect(screen.getByText("满足名单要求")).toBeVisible();
  expect(screen.getByText("补齐作者事实")).toBeVisible();
  await user.click(
    screen.getByRole("button", { name: "按这本期刊整理投稿包" }),
  );
  expect(await screen.findByText("作者与顺序")).toBeVisible();
  expect(screen.getByLabelText("经费声明")).toBeVisible();
  expect(screen.getByLabelText("摘要")).toBeVisible();
  expect(screen.getByLabelText("关键词（每行一个）")).toBeVisible();
  expect(screen.getByLabelText("CRediT 作者贡献")).toBeVisible();
  expect(screen.getByLabelText("生成式 AI 使用披露（如适用）")).toBeVisible();
  expect(screen.getByLabelText("附件用途")).toBeVisible();
  await user.click(screen.getByText("作者与顺序"));
  expect(screen.getByText("https://example.test/guide")).toBeVisible();
  expect(screen.getByText("查看计划生成的 1 个文件")).toBeVisible();
  expect(
    invokeMock.mock.calls.filter(
      ([command]) => command === "choose_local_inputs",
    ),
  ).toHaveLength(1);
  expect(invokeMock).toHaveBeenCalledWith(
    "select_target",
    expect.objectContaining({
      input: expect.objectContaining({
        origin: "recommendation",
        recommendationRef: "run-1",
      }),
    }),
  );
  await user.click(screen.getByRole("button", { name: "EN" }));
  expect(screen.getByText("View 1 planned files")).toBeVisible();
  expect(screen.getByLabelText("Funding statement")).toBeVisible();
  expect(screen.getByLabelText("Abstract")).toBeVisible();
  expect(screen.getByLabelText("Keywords (one per line)")).toBeVisible();
  expect(screen.getByLabelText("CRediT author contributions")).toBeVisible();
  expect(
    screen.getByLabelText("Generative-AI disclosure (if applicable)"),
  ).toBeVisible();
  expect(screen.getByLabelText("Attachment purpose")).toBeVisible();
});

it("lets a known-target user select only journals with verified package coverage", async () => {
  const user = userEvent.setup();
  render(<App />);
  await user.click(await screen.findByRole("button", { name: /整理投稿包/ }));
  await user.click(screen.getByRole("button", { name: "打开本地论文" }));
  await user.click(await screen.findByRole("button", { name: "搜索" }));
  const list = screen.getByText("Artificial Intelligence").closest("button")!;
  expect(list).toBeEnabled();
  expect(
    screen.getByText("Catalog Only Journal").closest("button"),
  ).toBeDisabled();
  await user.click(list);
  expect(await screen.findByText("作者与顺序")).toBeVisible();
  expect(invokeMock).toHaveBeenCalledWith(
    "select_target",
    expect.objectContaining({
      input: expect.objectContaining({
        origin: "catalog",
        recommendationRef: undefined,
      }),
    }),
  );
});

it("requires an explicit attachment purpose for an author-verified anonymized DOCX", async () => {
  const user = userEvent.setup();
  render(<App />);
  await user.click(await screen.findByRole("button", { name: /整理投稿包/ }));
  await user.click(screen.getByRole("button", { name: "打开本地论文" }));
  await user.click(await screen.findByRole("button", { name: "搜索" }));
  await user.click(
    screen.getByText("Artificial Intelligence").closest("button")!,
  );
  await user.selectOptions(
    await screen.findByLabelText("附件用途"),
    "anonymized_manuscript",
  );
  await user.click(screen.getByRole("button", { name: "选择附件" }));
  expect(invokeMock).toHaveBeenCalledWith("choose_local_inputs", {
    kind: "editable_manuscript",
  });
  expect(invokeMock).toHaveBeenCalledWith(
    "add_material",
    expect.objectContaining({ kind: "anonymized_manuscript" }),
  );
});

it("opens a PDF for matching and requests an explicit editable DOCX for final packaging", async () => {
  const user = userEvent.setup();
  manuscriptSelection = {
    token: "pdf-token",
    name: "study.pdf",
    extension: "pdf",
    sizeBytes: 2048,
    kind: "manuscript",
  };
  project = {
    ...project,
    activeSource: {
      ...project.activeSource,
      fileName: "study.pdf",
      format: "pdf",
      sizeBytes: 2048,
      featureProfile: "pdf_text",
    },
    facts: { ...project.facts, extractorVersion: "pdf-text-local-v1" },
    lastTask: "prepare_package",
  };
  render(<App />);
  await user.click(await screen.findByRole("button", { name: /整理投稿包/ }));
  await user.click(screen.getByRole("button", { name: "打开本地论文" }));
  expect(await screen.findByText("当前主稿为 PDF")).toBeVisible();
  expect(screen.getByText(/不会把 PDF 转换或改名成 Word 文件/)).toBeVisible();
  await user.click(screen.getByRole("button", { name: "搜索" }));
  await user.click(screen.getByText("Artificial Intelligence").closest("button")!);
  await user.selectOptions(
    await screen.findByLabelText("附件用途"),
    "editable_manuscript",
  );
  await user.click(screen.getByRole("button", { name: "选择附件" }));
  expect(invokeMock).toHaveBeenCalledWith("choose_local_inputs", {
    kind: "editable_manuscript",
  });
  expect(invokeMock).toHaveBeenCalledWith(
    "add_material",
    expect.objectContaining({ kind: "editable_manuscript" }),
  );
});

it("removes and restores a recent task without presenting it as file deletion", async () => {
  const user = userEvent.setup();
  recentItems = [project];
  render(<App />);
  expect(await screen.findByText("A Local AI Study")).toBeVisible();
  await user.click(
    screen.getByRole("button", { name: /从最近任务移除 A Local AI Study/ }),
  );
  const removed = await screen.findByText("已移除任务（1）");
  await user.click(removed);
  expect(screen.getByText(/论文副本、生成文件和外部导出均未删除/)).toBeVisible();
  await user.click(screen.getByRole("button", { name: "恢复到最近任务" }));
  expect(
    await screen.findByRole("button", { name: /从最近任务移除 A Local AI Study/ }),
  ).toBeVisible();
  expect(invokeMock).toHaveBeenCalledWith(
    "hide_recent_project",
    expect.objectContaining({ projectId: "project-1" }),
  );
  expect(invokeMock).toHaveBeenCalledWith(
    "restore_recent_project",
    expect.objectContaining({ projectId: "project-1" }),
  );
});

it("distinguishes saved files from a failed receipt record in both locales", async () => {
  project.workspace = {
    kind: "folder",
    name: "Study Folder",
    bindingId: "binding-id",
  };
  const user = userEvent.setup();
  render(<App />);
  await user.click(await screen.findByRole("button", { name: /整理投稿包/ }));
  await user.click(screen.getByRole("button", { name: "打开本地论文" }));
  await user.click(await screen.findByRole("button", { name: "搜索" }));
  await user.click(
    screen.getByText("Artificial Intelligence").closest("button")!,
  );
  await user.click(await screen.findByRole("button", { name: "预览草稿" }));
  expect(screen.getByText(/不会覆盖原稿或附件/)).toBeVisible();
  await user.click(
    await screen.findByRole("button", { name: "保存到项目文件夹" }),
  );
  expect(invokeMock).toHaveBeenCalledWith(
    "choose_project_folder_for_export",
    { projectId: "project-1" },
  );
  expect(
    await screen.findByText(/文件已经保存，但本地回执记录失败/),
  ).toBeVisible();
  await user.click(screen.getByRole("button", { name: "EN" }));
  expect(
    screen.getByText(/files were saved, but the local receipt record failed/i),
  ).toBeVisible();
});
