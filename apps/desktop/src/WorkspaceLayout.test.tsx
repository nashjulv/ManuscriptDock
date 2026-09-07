import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: () => true }));

const workspace = {
  id: "layout-synthetic", snapshotVersion: 1, importedUnixMs: 0, contentHash: "a".repeat(64),
  manuscript: { name: "Synthetic layout study.pdf", extension: "pdf", kind: "pdf", sizeBytes: 2048, modifiedUnixMs: null },
};

beforeEach(() => {
  window.localStorage.clear();
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation(async (command) => {
    if (command === "list_workspaces") return { workspaces: [workspace], archivedWorkspaces: [], warnings: [] };
    if (command === "get_workspace_storage_summary") return null;
    if (command === "get_ui_preferences") return { textSize: "default" };
    if (command === "get_journal_requirement_snapshots") return [];
    if (command === "list_rule_packs") return { rulePacks: [] };
    if (command === "get_workspace_lifecycle") return {
      structureReport: null, readinessReport: null, attestation: null, submission: null, knowledgeBody: null,
      submissionMaterials: { schemaVersion: 1, workspaceId: workspace.id, manuscriptVersion: 1, materials: [], checklist: [], recommendationReady: true, targetVerified: false, requiredComplete: false, targetCheckReady: false, workflowStatus: "preliminary_recommendation", requiredTotal: 0, requiredCompleted: 0 },
    };
    throw new Error(`Unexpected command: ${command}`);
  });
});

it.each(["zh-CN", "en"])("preserves the shared workflow cards across overview and check navigation in %s", async (locale) => {
  window.localStorage.setItem("manuscriptdock.locale", locale);
  const en = locale === "en";
  const user = userEvent.setup();
  const { container } = render(<App />);
  await user.click(await screen.findByRole("button", { name: `${en ? "Open" : "打开"} Synthetic layout study.pdf` }));
  const rail = screen.getByRole("list", { name: en ? "Submission preparation progress" : "投稿准备进度" });
  const cards = within(rail).getAllByRole("listitem");
  expect(cards).toHaveLength(4);
  await waitFor(() => expect(cards[0]).toHaveAttribute("data-complete", "true"));
  expect(cards[1]).toHaveAttribute("data-active", "true");
  expect(cards[1]).toHaveTextContent(en ? "Journal, article type, and official rules" : "期刊、文章类型与官方要求");
  expect(cards[3]).toHaveTextContent(en ? "Package preflight" : "投稿包预检");
  expect(cards[3]).toHaveAttribute("data-complete", "false");
  await user.click(screen.getByTitle(en ? "Check/revise" : "检查与修订"));
  expect(screen.getByRole("navigation", { name: en ? "Check and revise" : "检查与修订" })).toBeVisible();
  expect(rail).toBeVisible();
  expect(container.querySelector(".workspace-panes")?.previousElementSibling).toHaveClass("prepare-subnav");
  await user.click(screen.getByRole("button", { name: en ? "View evidence" : "查看依据" }));
  expect(container.querySelector(".workspace-panes")).toHaveAttribute("data-evidence-open", "true");
});
