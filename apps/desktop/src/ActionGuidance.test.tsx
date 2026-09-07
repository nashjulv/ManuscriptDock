import { useState, type ComponentProps } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { GuidedButton, GuidanceProvider, useGuidanceDraft } from "./ActionGuidance";
import App, { BackupRouteCard, JournalMatchStage, JournalRequirementCapture, SubmissionMaterialsCenter, VersionManager } from "./App";
import { I18nProvider } from "./i18n";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
const mocked = vi.mocked(invoke);
const workspace = { id: "guide-synthetic", snapshotVersion: 1, contentHash: "a".repeat(64), importedUnixMs: 1, manuscript: { name: "synthetic.pdf", kind: "pdf" as const, extension: "pdf", sizeBytes: 100, modifiedUnixMs: null } };
const target: ComponentProps<typeof JournalRequirementCapture>["target"] = { schemaVersion: 3, workspaceId: workspace.id, selectionId: "target", selectedAgainstManuscriptVersion: 1, recommendationRunId: "run", journalId: "synthetic", name: "合成期刊", nameEn: "Synthetic Journal", publisher: "Synthetic", region: "international", rankSystem: "JCR", rankTier: "Q1", homepageUrl: "https://example.test/guide", articleType: "research", planRole: "primary", priority: 0, selectedUnixMs: 1, recordHash: "b".repeat(64), externalTransmission: "not_performed" };

describe("action prerequisite guidance", () => {
  afterEach(() => vi.useRealTimers());
  beforeEach(() => { window.localStorage.clear(); mocked.mockReset(); mocked.mockResolvedValue(null); vi.mocked(isTauri).mockReturnValue(false); });
  for (const locale of ["zh-CN", "en"] as const) {
    const en = locale === "en";
    const setup = () => { window.localStorage.setItem("manuscriptdock.locale", locale); return userEvent.setup(); };

    it(`${locale}: expires the highlight after four seconds, keeps the explanation and restarts on repeat clicks`, () => {
      vi.useFakeTimers(); window.localStorage.setItem("manuscriptdock.locale", locale);
      const action = vi.fn(); const message = en ? "Complete this field" : "请先填写此项";
      render(<I18nProvider><label>Field<input id="timed-field" /></label><GuidedButton prerequisite={{ target: "#timed-field", message }} onClick={action}>Continue</GuidedButton></I18nProvider>);
      const field = screen.getByRole("textbox"); const button = screen.getByRole("button", { name: "Continue" });
      fireEvent.click(button);
      expect(field).toHaveClass("prerequisite-highlight");
      act(() => vi.advanceTimersByTime(3500));
      fireEvent.click(button);
      act(() => vi.advanceTimersByTime(600));
      expect(field).toHaveClass("prerequisite-highlight");
      act(() => vi.advanceTimersByTime(3400));
      expect(field).not.toHaveClass("prerequisite-highlight");
      expect(field).toHaveFocus(); expect(screen.getByText(message)).toBeVisible();
      expect(action).not.toHaveBeenCalled();
    });

    it(`${locale}: destination highlights expire and typing clears a restarted highlight immediately`, () => {
      vi.useFakeTimers(); window.localStorage.setItem("manuscriptdock.locale", locale);
      const navigate = vi.fn();
      render(<I18nProvider><GuidanceProvider onNavigate={navigate}><input id="timed-destination" /><GuidedButton prerequisite={{ destination: "journals", target: "#timed-destination", message: "Missing", actionLabel: "Go" }}>Upload</GuidedButton></GuidanceProvider></I18nProvider>);
      const go = () => { fireEvent.click(screen.getByText("Upload")); fireEvent.click(screen.getByText("Go")); act(() => vi.advanceTimersByTime(32)); };
      go(); const field = screen.getByRole("textbox");
      expect(field).toHaveClass("prerequisite-highlight");
      act(() => vi.advanceTimersByTime(4000));
      expect(field).not.toHaveClass("prerequisite-highlight");
      go(); expect(field).toHaveClass("prerequisite-highlight");
      fireEvent.input(field, { target: { value: "Filled" } });
      expect(field).not.toHaveClass("prerequisite-highlight");
      expect(navigate).toHaveBeenCalledTimes(2);
    });

    it(`${locale}: opens nested details, focuses the missing checkbox without consenting, and retries explicitly`, async () => {
      const user = setup(); const action = vi.fn();
      function Form() {
        const [checked, setChecked] = useState(false);
        return <><details><summary>Outer</summary><details><summary>Inner</summary><label>Consent<input id="consent" type="checkbox" checked={checked} onChange={e => setChecked(e.target.checked)} /></label></details></details><GuidedButton prerequisite={!checked && { message: en ? "Confirm first" : "请先确认", target: "#consent" }} onClick={action}>Submit</GuidedButton></>;
      }
      render(<I18nProvider><Form /></I18nProvider>);
      await user.click(screen.getByRole("button", { name: "Submit" }));
      const checkbox = screen.getByRole("checkbox", { name: "Consent" });
      expect(checkbox).toHaveFocus(); expect(checkbox).not.toBeChecked(); expect(checkbox).toHaveClass("prerequisite-highlight");
      expect(checkbox.closest("details")).toHaveAttribute("open"); expect(action).not.toHaveBeenCalled();
      await user.click(checkbox); expect(action).not.toHaveBeenCalled(); expect(checkbox).not.toHaveClass("prerequisite-highlight");
      await user.click(screen.getByRole("button", { name: "Submit" })); expect(action).toHaveBeenCalledTimes(1);
    });

    it(`${locale}: scopes repeated controls to the clicked card and keeps busy actions locked`, async () => {
      const user = setup(); const action = vi.fn();
      render(<I18nProvider>{["one", "two"].map(name => <section className="card" key={name}><label>{name}<input /></label><GuidedButton prerequisite={{ message: "Missing", scope: ".card", target: "input" }} onClick={action}>{name}</GuidedButton></section>)}<GuidedButton disabled prerequisite={{ message: "Missing", target: "input" }} onClick={action}>Busy</GuidedButton></I18nProvider>);
      await user.click(screen.getByRole("button", { name: "two" })); expect(screen.getByLabelText("two")).toHaveFocus();
      await user.click(screen.getByRole("button", { name: "Busy" })); expect(action).not.toHaveBeenCalled();
    });

    it(`${locale}: cross-page dialog supports cancel, Escape and explicit navigation without running the original action`, async () => {
      const user = setup(); const action = vi.fn(); const navigate = vi.fn();
      render(<I18nProvider><GuidanceProvider onNavigate={navigate}><GuidedButton prerequisite={{ message: en ? "Review in Materials" : "请到投稿资料核验", destination: "materials", itemId: "required-item", actionLabel: "Go", target: "#destination" }} onClick={action}>Export</GuidedButton><input id="destination" /></GuidanceProvider></I18nProvider>);
      const button = screen.getByRole("button", { name: "Export" });
      await user.click(button); expect(screen.getByRole("dialog")).toHaveTextContent(en ? "Review in Materials" : "请到投稿资料核验");
      await user.keyboard("{Escape}"); expect(screen.queryByRole("dialog")).toBeNull(); expect(button).toHaveFocus(); expect(navigate).not.toHaveBeenCalled();
      await user.click(button); await user.click(screen.getByRole("button", { name: en ? "Not now" : "暂不处理" })); expect(navigate).not.toHaveBeenCalled();
      await user.click(button); await user.click(screen.getByRole("button", { name: "Go" }));
      expect(navigate).toHaveBeenCalledWith("materials", "required-item"); expect(action).not.toHaveBeenCalled();
      await waitFor(() => expect(document.getElementById("destination")).toHaveFocus());
    });

    it(`${locale}: a stale dialog cannot execute a prerequisite that has already been resolved`, async () => {
      const user = setup(); const navigate = vi.fn(); const action = vi.fn();
      const body = (missing: boolean) => <I18nProvider><GuidanceProvider onNavigate={navigate}><GuidedButton prerequisite={missing && { message: "Missing", destination: "check", actionLabel: "Go" }} onClick={action}>Export</GuidedButton></GuidanceProvider></I18nProvider>;
      const view = render(body(true)); await user.click(screen.getByText("Export")); view.rerender(body(false));
      await user.click(screen.getByText("Go")); expect(navigate).not.toHaveBeenCalled(); expect(action).not.toHaveBeenCalled();
      await user.click(screen.getByText("Export")); expect(action).toHaveBeenCalledTimes(1);
    });

    it(`${locale}: preserves a question draft when navigating to a prerequisite and back`, async () => {
      const user = setup();
      function Draft() { const [value, set] = useGuidanceDraft("question"); return <label>Question<input value={value} onChange={e => set(e.target.value)} /></label>; }
      function Pages() { const [page, set] = useState(true); return <GuidanceProvider onNavigate={vi.fn()}>{page ? <Draft /> : <p>Other page</p>}<button onClick={() => set(!page)}>Switch</button></GuidanceProvider>; }
      render(<I18nProvider><Pages /></I18nProvider>); await user.type(screen.getByLabelText("Question"), "Synthetic question");
      await user.click(screen.getByText("Switch")); await user.click(screen.getByText("Switch")); expect(screen.getByLabelText("Question")).toHaveValue("Synthetic question");
    });

    it(`${locale}: rechecks expiring prerequisites at click time without needing a render`, async () => {
      const user = setup(); const action = vi.fn(); let expired = false;
      render(<I18nProvider><input id="refresh" /><GuidedButton prerequisite={() => expired && { message: "Refresh required", target: "#refresh" }} onClick={action}>Export</GuidedButton></I18nProvider>);
      expired = true;
      await user.click(screen.getByText("Export")); expect(action).not.toHaveBeenCalled(); expect(document.getElementById("refresh")).toHaveFocus();
      expired = false;
      await user.click(screen.getByText("Export")); expect(action).toHaveBeenCalledTimes(1);
    });

    it(`${locale}: switching a local tab still focuses the target after the clicked button unmounts`, async () => {
      const user = setup();
      function Tabs() { const [upload, setUpload] = useState(false); return <GuidanceProvider onNavigate={vi.fn()}>{upload ? <section id="upload-panel">Upload</section> : <GuidedButton prerequisite={{ message: "Choose a file in Upload", prepare: () => setUpload(true), target: "#upload-panel" }}>Link</GuidedButton>}</GuidanceProvider>; }
      render(<I18nProvider><Tabs /></I18nProvider>); await user.click(screen.getByText("Link"));
      expect(screen.queryByRole("dialog")).toBeNull(); await waitFor(() => expect(document.getElementById("upload-panel")).toHaveFocus());
    });

    it(`${locale}: waits for the destination to finish loading before using a fallback target`, async () => {
      const user = setup();
      function Pages() {
        const [page, set] = useState(false); const [loaded, finish] = useState(false);
        return <GuidanceProvider onNavigate={() => set(true)}>{page ? <><span hidden data-guidance-loading={loaded ? "false" : "true"} /><button id="fallback" onClick={() => finish(true)}>Finish loading</button>{loaded ? <input id="loaded-target" /> : null}</> : <GuidedButton prerequisite={{ message: "Missing", destination: "journals", target: "#loaded-target, #fallback", actionLabel: "Go" }}>Upload</GuidedButton>}</GuidanceProvider>;
      }
      render(<I18nProvider><Pages /></I18nProvider>);
      await user.click(screen.getByText("Upload")); await user.click(screen.getByText("Go"));
      expect(document.getElementById("fallback")).not.toHaveClass("prerequisite-highlight");
      await user.click(screen.getByText("Finish loading")); await waitFor(() => expect(document.getElementById("loaded-target")).toHaveFocus());
    });

    it(`${locale}: empty Upload retains an entry and routes missing target setup without picking files`, async () => {
      const user = setup(); const add = vi.fn(); const navigate = vi.fn();
      render(<I18nProvider><GuidanceProvider onNavigate={navigate}><SubmissionMaterialsCenter workspace={workspace} catalog={null} busy={false} confirmingRequirementId={null} targetReady={false} onAdd={add} onSetIncluded={vi.fn()} onDelete={vi.fn()} onConfirm={vi.fn()} onCatalogUpdated={vi.fn()} onContinue={vi.fn()} /></GuidanceProvider></I18nProvider>);
      await user.click(screen.getByRole("tab", { name: en ? /^Upload/ : /^上传资料/ }));
      await user.click(screen.getByRole("button", { name: en ? "Choose files" : "选择文件" }));
      expect(screen.getByRole("dialog")).toHaveTextContent(en ? "Target Journal" : "目标期刊"); expect(add).not.toHaveBeenCalled();
      await user.click(screen.getByRole("button", { name: en ? "Go to Target Journal" : "前往目标期刊" })); expect(navigate).toHaveBeenCalledWith("journals", undefined);
    });

    it(`${locale}: export blockers explain the required page without opening a file picker`, async () => {
      const user = setup(); vi.mocked(isTauri).mockReturnValue(true);
      const requirements = { schemaVersion: 3, snapshotId: "snapshot", workspaceId: workspace.id, targetSelectionId: target.selectionId, status: "author_attested_official", freshUntilUnixMs: Date.UTC(2099, 0, 1), requirements: [{ id: "main", label: "主稿", labelEn: "Manuscript" }], sources: [], limitations: [] };
      mocked.mockImplementation(async command => {
        if (command === "list_workspaces") return { workspaces: [workspace], archivedWorkspaces: [], warnings: [] };
        if (command === "get_workspace_lifecycle") return { workspaceId: workspace.id, currentVersion: 1, structureReport: null, readinessReport: null, attestation: null, submission: null, knowledgeBody: null, submissionTarget: target, submissionTargetPlan: { primary: target, backups: [] }, journalRequirements: requirements };
        if (command === "get_journal_requirement_snapshots") return [requirements];
        if (command === "get_submission_materials") return { schemaVersion: 5, workspaceId: workspace.id, manuscriptVersion: 1, materials: [], checklist: [], targetVerified: true, requiredComplete: true, authorInputsReady: true, targetCheckReady: false, checkCurrent: false, workflowStatus: "materials_complete_check_required", requiredTotal: 0, requiredCompleted: 0, pendingTasks: [] };
        if (command === "get_target_submission_package_plan") return { schemaVersion: 5, workspaceId: workspace.id, manuscriptVersion: 1, targetSelectionId: "target", targetName: target.name, ready: false, files: [], warnings: [], blockers: ["CHECK_REQUIRED"], createdUnixMs: 1 };
        if (["get_journal_profile_discoveries", "list_linked_sources", "find_workspace_matches", "list_journal_recommendations"].includes(command)) return [];
        return null;
      });
      render(<App />);
      await user.click(await screen.findByRole("button", { name: en ? "Open synthetic.pdf" : "打开 synthetic.pdf" }));
      await user.click(within(screen.getByRole("navigation", { name: en ? "Primary submission tasks" : "投稿准备主任务" })).getByRole("button", { name: en ? /^Package/ : /^投稿包/ }));
      await user.click(await screen.findByRole("button", { name: en ? "Choose export folder" : "选择导出文件夹" }));
      expect(screen.getByRole("dialog")).toHaveTextContent(en ? "unresolved tasks" : "尚有待办");
      expect(mocked).not.toHaveBeenCalledWith("export_target_submission_package", expect.anything());
      await user.click(screen.getByRole("button", { name: en ? "Go to Check and revise" : "前往检查与修订" }));
      await waitFor(() => expect(document.getElementById("operation-pane")).toHaveFocus());
    });

    it(`${locale}: manual official text guides each missing field and submits only after explicit confirmation`, async () => {
      const user = setup(); const save = vi.fn();
      render(<I18nProvider><JournalRequirementCapture target={target} snapshot={null} busy={false} onDiscover={vi.fn()} onSaveManual={save} /></I18nProvider>);
      await user.click(screen.getByText(en ? "Page unavailable? Paste official text" : "网页无法读取？粘贴官方原文"));
      const button = screen.getByRole("button", { name: en ? "Save local snapshot" : "保存并生成本地快照" });
      const url = screen.getByLabelText(en ? "Official source URL" : "官方来源网址");
      await user.clear(url); await user.click(button); expect(url).toHaveFocus(); expect(save).not.toHaveBeenCalled();
      await user.type(url, "https://example.test/guide"); await user.click(button);
      const original = screen.getByLabelText(en ? "Author-guide text" : "作者指南原文"); expect(original).toHaveFocus();
      await user.type(original, "Synthetic official requirements for every submitted manuscript."); await user.click(button);
      const confirm = screen.getByRole("checkbox", { name: /I confirm this text|我确认这段原文/ }); expect(confirm).toHaveFocus(); expect(confirm).not.toBeChecked(); expect(save).not.toHaveBeenCalled();
      await user.click(confirm); await user.click(button); expect(save).toHaveBeenCalledTimes(1);
    });

    it(`${locale}: promoting a backup focuses its route reason, not the journal consent`, async () => {
      const user = setup(); const promote = vi.fn();
      render(<I18nProvider><BackupRouteCard target={target} order={1} snapshot={null} busy={false} requirementBusy={false} primaryActive suggestedNext onPromote={promote} onDiscover={vi.fn()} onSaveManual={vi.fn()} /></I18nProvider>);
      const button = screen.getByRole("button", { name: en ? "Promote to primary" : "提升为当前主线" });
      await user.click(button); expect(screen.getByRole("combobox")).toHaveFocus(); expect(promote).not.toHaveBeenCalled();
      await user.selectOptions(screen.getByRole("combobox"), "not_submitted"); await user.click(button); expect(promote).toHaveBeenCalledWith("target", "not_submitted");
    });

    it(`${locale}: a mismatched revision focuses reselection and never saves`, async () => {
      const user = setup(); const save = vi.fn();
      render(<I18nProvider><VersionManager workspace={workspace} history={null} selectedVersion={null} candidate={{ ...workspace.manuscript, kind: "word", extension: "docx" }} note="" notice={null} selecting={false} saving={false} restoring={false} onSelectCandidate={vi.fn()} onNoteChange={vi.fn()} onSave={save} onSelectVersion={vi.fn()} onRestore={vi.fn()} onContinue={vi.fn()} continueReady={false} /></I18nProvider>);
      await user.click(screen.getByRole("button", { name: en ? "Save as v2 and recheck" : "保存为 v2 并复查" })); expect(screen.getByRole("button", { name: en ? "Choose another" : "重新选择" })).toHaveFocus(); expect(save).not.toHaveBeenCalled();
    });

    it(`${locale}: recommendation validation opens the optional section and focuses date or policy text`, async () => {
      const user = setup();
      render(<I18nProvider><JournalMatchStage workspace={workspace} selectedTarget={null} targetPlan={null} requirementSnapshots={[]} selectingTarget={false} targetPlanBusyId={null} requirementBusyId={null} onRecommendationGenerated={vi.fn()} onSelectTarget={vi.fn()} onClearPrimary={vi.fn()} onAddBackup={vi.fn()} onPromoteBackup={vi.fn()} onDiscoverRequirements={vi.fn()} onSaveManualRequirements={vi.fn()} onContinue={vi.fn()} /></I18nProvider>);
      const deadline = document.getElementById("journal-deadline") as HTMLInputElement;
      deadline.closest("details")!.open = true;
      await user.clear(deadline); deadline.closest("details")!.open = false;
      await user.click(screen.getByRole("button", { name: en ? "Generate preliminary recommendations from this manuscript" : "根据当前论文生成初步推荐" }));
      expect(deadline).toHaveFocus(); expect(deadline.closest("details")).toHaveAttribute("open"); expect(mocked).not.toHaveBeenCalled();
      await user.type(deadline, "2099-01-01");
      const policy = screen.getByLabelText(en ? "Institution requirement text" : "学校要求说明文字"); await user.type(policy, "Short");
      await user.click(screen.getByRole("button", { name: en ? "Generate preliminary recommendations with institution rules" : "结合校规生成初步推荐" })); expect(policy).toHaveFocus(); expect(mocked).not.toHaveBeenCalled();
    });
  }
});
