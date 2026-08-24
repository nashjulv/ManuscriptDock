import { invoke, isTauri } from "@tauri-apps/api/core";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: vi.fn() }));

const invokeMock = vi.mocked(invoke);
const isTauriMock = vi.mocked(isTauri);

describe("App", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isTauriMock.mockReset();
    isTauriMock.mockReturnValue(false);
    window.localStorage.clear();
  });

  it("explains the local-first import step", () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: "我的工作台" })).toBeVisible();
    expect(screen.getByRole("button", { name: "我的工作台" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: "选择论文" })).toBeEnabled();
    expect(screen.getByText("没有文件会在此阶段上传")).toBeVisible();
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
    expect(document.documentElement).toHaveAttribute("lang", "en");
    expect(window.localStorage.getItem("manuscriptdock.locale")).toBe("en");

    unmount();
    render(<App />);
    expect(screen.getByRole("heading", { name: "My Workspace" })).toBeVisible();
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

    expect(await screen.findByText("本地工作区已创建")).toBeVisible();
    expect(screen.getByText("不可变；历史不会被覆盖")).toBeVisible();
    expect(screen.getByRole("button", { name: "我的工作台" })).toBeVisible();
    expect(invokeMock).toHaveBeenLastCalledWith("create_workspace", {
      selectionId: "one-time-selection",
    });

    await user.click(screen.getByRole("button", { name: "我的工作台" }));
    expect(screen.getByRole("heading", { name: "我的工作台" })).toBeVisible();
    expect(screen.getByRole("button", { name: "我的工作台" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByText("synthetic-study.tex")).toBeVisible();
  });

  it("recovers recent local workspaces when running inside Tauri", async () => {
    isTauriMock.mockReturnValue(true);
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
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /structured-study\.tex/ }));
    await user.click(screen.getByRole("button", { name: "结构" }));
    await user.click(screen.getByRole("button", { name: "开始结构提取" }));

    expect(await screen.findByRole("heading", { name: "Synthetic Evidence Study" })).toBeVisible();
    expect(screen.getByText("完整提取")).toBeVisible();
    expect(screen.getAllByText("Ada Author · Ben Researcher").length).toBeGreaterThan(0);
    expect(screen.getByText("A compact synthetic abstract.")).toBeVisible();
    expect(screen.getByRole("list", { name: "必要结构检测结果" })).toHaveTextContent("作者已检测");
    expect(screen.getByRole("list", { name: "检测到的章节" })).toHaveTextContent("Methods");
    expect(invokeMock).toHaveBeenLastCalledWith("analyze_workspace", {
      workspaceId: workspace.id,
    });

    await user.click(screen.getByRole("button", { name: "选择检查标准" }));
    const ieeeRule = await screen.findByRole("checkbox", { name: /IEEE 期刊通用稿件结构/ });
    await user.click(ieeeRule);
    await user.click(screen.getByRole("button", { name: "核对投稿要素" }));
    expect(await screen.findByRole("heading", { name: "核对出版社投稿要素" })).toBeVisible();
    expect(screen.getByText("单段摘要")).toBeVisible();
    expect(screen.getByText("ORCID")).toBeVisible();
    expect(screen.getByLabelText("论文标题")).toHaveValue("Synthetic Evidence Study");
    expect(invokeMock).toHaveBeenCalledWith("list_submission_elements", { rulePackIds: ["md.publisher.ieee"] });
    expect(invokeMock).toHaveBeenCalledWith("get_revision_draft", { workspaceId: workspace.id });
    await user.click(screen.getByRole("button", { name: "进入投稿检查" }));
    await user.click(screen.getByRole("button", { name: "开始检查" }));

    expect(await screen.findByRole("heading", { name: "仍有事项需要处理" })).toBeVisible();
    expect(screen.getByRole("list", { name: "投稿检查明细" })).toHaveTextContent("补充关键词");
    expect(screen.getByText(/来源可信，内容未被篡改/)).toBeVisible();
    expect(screen.getAllByText(/未发生外部传输/).length).toBeGreaterThan(0);
    expect(invokeMock).toHaveBeenLastCalledWith("evaluate_readiness", {
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

    await user.click(await screen.findByRole("button", { name: /study\.tex/ }));
    await user.click(screen.getByRole("button", { name: "版本" }));
    expect(await screen.findByRole("list", { name: "论文版本时间线" })).toHaveTextContent("v1");

    await user.click(screen.getByRole("button", { name: "选择修改后的稿件" }));
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
    let draftCalls = 0;
    invokeMock.mockImplementation((command) => {
      if (command === "list_workspaces") return Promise.resolve({ workspaces: [workspace], warnings: [] });
      if (command === "list_submission_elements") return Promise.resolve({ elements: [], rulePacks: [] });
      if (command === "get_revision_draft") { draftCalls += 1; return Promise.resolve(draftCalls === 1 ? draft : revisedDraft); }
      if (command === "apply_manuscript_revision") return Promise.resolve({ status: "created", workspace: revised, version: { version: 2, parentVersion: 1, manuscript: revised.manuscript, contentHash: revised.contentHash, createdUnixMs: Date.UTC(2026, 7, 24, 6, 30), note: "投稿优化修订台：1 项修改", origin: "revision", restoredFromVersion: null }, revisionSet: { revisionId: "revision-set", workspaceId: workspace.id, baseVersion: 1, outputVersion: 2, createdUnixMs: Date.UTC(2026, 7, 24, 6, 30), changes: [{ field: "title", before: "Original title", after: "Revised title", basis: "author_edit", status: "accepted" }], externalTransmission: "not_performed" } });
      return Promise.reject(new Error(`unexpected command ${command}`));
    });
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: /study\.tex/ }));
    await user.click(screen.getByRole("button", { name: "修订" }));
    const title = await screen.findByLabelText("论文标题");
    await user.clear(title); await user.type(title, "Revised title");
    expect(screen.getByText("保存前预览")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "保存为新版本" }));
    expect(await screen.findByText(/已保存为 v2/)).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("apply_manuscript_revision", { workspaceId: workspace.id, baseVersion: 1, changes: [{ field: "title", after: "Revised title" }] });
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
      schemaVersion: 1, knowledgeBodyId: "body:K-A", snapshotVersion: 7, manuscript: reference("artifact:K-A", "artifact_version", 3),
      claim: { claim: bodies[0].claim, proposition: { ...reference("proposition:K-A", "proposition", 3), state: "established" }, conditions: { ...reference("scope:K-A", "scope", 3), state: "established" }, evidence: { ...reference("evidence:K-A", "evidence", 2), state: "established" }, sources: { ...bodies[0].sourceAnchor, state: "established" }, status: { ...reference("status:K-A", "status", 2), state: "established" } },
      objects: { artifactVersion: reference("artifact:K-A", "artifact_version", 3), claim: bodies[0].claim, scope: reference("scope:K-A", "scope", 3), method: bodies[0].method, result: reference("result:K-A", "result", 2), evidenceRelation: reference("evidence-relation:K-A", "evidence_relation", 2), sourceAnchor: bodies[0].sourceAnchor, aiReviewReport: reference("review:K-A", "ai_review_report", 2), provenance: reference("provenance:K-A", "provenance", 2), knowledgeBodySnapshot: reference("snapshot:K-A", "knowledge_body_snapshot", 7) },
      aiReviewReport: reference("review:K-A", "ai_review_report", 2),
      aiReviewHistory: { reportId: "review:K-A", currentVersion: 2, versions: [{ reportId: "review:K-A", version: 1, previousVersion: null }, { reportId: "review:K-A", version: 2, previousVersion: 1 }] },
      network: { bodies, assertions: [assertion("reproduction:1", "reproduction", "ReproductionAssertion", 0, 1), assertion("conflict:1", "claim_relation", "ClaimRelationAssertion", 1, 2), assertion("transfer:1", "method_transfer", "MethodRelationAssertion", 0, 3), assertion("citation:1", "citation", "CitationAssertion", 1, 4), assertion("classification:1", "classification", "ClassificationAssignment", 3, 4), assertion("evidence:1", "evidence_relation", "EvidenceRelation", 2, 4)], supportedRelations: ["citation", "claim_relation", "evidence_relation", "method_transfer", "reproduction", "alignment", "version_relation", "classification"] },
      externalTransmission: "not_performed",
    };
    invokeMock.mockImplementation((command) => Promise.resolve(command === "list_rule_packs" ? { rulePacks: [] } : command === "get_knowledge_body_snapshot" ? knowledgeSnapshot : { workspaces: [workspace], warnings: [] }) as never);
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /navigation-study\.pdf/ }));
    expect(screen.getByRole("navigation", { name: "论文工作阶段" })).toBeVisible();
    expect(screen.getByRole("tabpanel", { name: "原稿 证据" })).toHaveTextContent("只读");

    await user.click(screen.getByRole("button", { name: "目标" }));
    expect(screen.getByRole("heading", { name: "选择适用于这篇论文的标准" })).toBeVisible();

    await user.click(screen.getByRole("button", { name: "知识体" }));
    expect(await screen.findByRole("heading", { name: "知识体与关联网络" })).toBeVisible();
    expect(screen.getByRole("list", { name: "知识体核心要素" })).toHaveTextContent("ArtifactVersion · v3");
    expect(screen.getByRole("list", { name: "知识体核心要素" })).toHaveTextContent("Scope · v3");
    expect(screen.getByRole("list", { name: "知识体核心要素" })).toHaveTextContent("KnowledgeBodySnapshot · S7");
    expect(screen.getByRole("heading", { name: "AIReviewReport" }).parentElement?.parentElement).toHaveTextContent("v2");
    expect(screen.getByText(/内部保留 2 个审核版本/)).toBeVisible();

    await user.click(screen.getByRole("tab", { name: "证据" }));
    expect(screen.getByRole("tab", { name: "证据" })).toHaveAttribute("aria-selected", "true");
    const spatialMap = screen.getByRole("img", { name: /中心是 Claim v3 十二面体/ });
    expect(spatialMap.querySelector(".claim-dodecahedron")).toBeInTheDocument();
    expect(spatialMap.querySelectorAll(".dodeca-edge")).toHaveLength(30);
    expect(spatialMap.querySelector(".claim-core")).toHaveTextContent("Claim · v3十二面体核心");
    expect(spatialMap.querySelectorAll(".claim-element")).toHaveLength(8);
    expect(spatialMap).toHaveTextContent("ArtifactVersionv3");
    expect(spatialMap).toHaveTextContent("EvidenceRelationv2");
    expect(spatialMap).toHaveTextContent("AIReviewReportv2历史 v1");

    await user.click(screen.getByRole("tab", { name: "2. 两体关联" }));
    expect(screen.getByRole("img", { name: /2 个保持边界的知识体/ })).toBeVisible();
    expect(document.querySelectorAll(".network-body")).toHaveLength(2);

    await user.click(screen.getByRole("tab", { name: "3. 关联网络" }));
    expect(screen.getByRole("img", { name: /5 个保持边界的知识体/ })).toBeVisible();
    expect(document.querySelectorAll(".network-body")).toHaveLength(5);
    expect(document.querySelectorAll(".network-assertion")).toHaveLength(6);
  });
});
