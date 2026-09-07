import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { I18nProvider, localizeBackendText } from "./i18n";
import { RequirementDecision } from "./RequirementDecision";
import { CheckRequirementReviews, confirmationReviewed } from "./CheckRequirementReviews";
import { StructureReview } from "./StructureReview";
import { SourceRecovery } from "./SourceRecovery";
import { restoredRuleSelection, HistoricalMaterialReuse, PendingTasks, SubmissionMaterialsCenter, type SubmissionMaterialCatalog, type SubmissionMaterialChecklistItem } from "./App";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: () => true }));
const mocked = vi.mocked(invoke);
const item: SubmissionMaterialChecklistItem = { id: "journal-template", label: "期刊模板", labelEn: "Journal template", group: "manuscript", requirement: "required", status: "manual_verification", detail: "REQUIREMENT_COMPARE_SOURCE", verification: "manual", materialKind: null, blocking: true, confirmable: true, sourceUrl: "https://example.test/guide", evidenceExcerpt: "Synthetic template must be used", capturedUnixMs: 1, freshUntilUnixMs: Date.UTC(2099, 1), requiredCount: 0, matchedMaterialIds: [], decision: "unknown", notApplicableAllowed: false };
const catalog: SubmissionMaterialCatalog = { schemaVersion: 5, workspaceId: "synthetic", manuscriptVersion: 1, materials: [], checklist: [item], recommendationReady: true, targetVerified: true, requiredComplete: false, authorInputsReady: false, targetCheckReady: false, checkCurrent: false, workflowStatus: "materials_required", requiredTotal: 1, requiredCompleted: 0, pendingTasks: [{ id: item.id, label: item.label, labelEn: item.labelEn, state: item.status, destination: "materials", blockingScope: "export" }] };

describe("first-use remediation", () => {
  it("restores selectable rules without resubmitting internal dependencies after restart", () => {
    expect(restoredRuleSelection(["md.core.structure", "md.stage.initial-submission", "core-structure-v1", "initial-submission-v1", "md.publisher.ieee", "md.publisher.ieee"].map(id => ({ id })))).toEqual(["md.publisher.ieee"]);
  });
  beforeEach(() => { mocked.mockReset(); window.localStorage.clear(); });
  for (const locale of ["zh-CN", "en"] as const) {
    const en = locale === "en";
    it(`${locale}: keeps a legacy reviewed-only record unresolved until an explicit decision`, () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      render(<I18nProvider><RequirementDecision workspaceId="synthetic" item={{ ...item, decision: "legacy_reviewed" }} onUpdated={vi.fn()} /></I18nProvider>);
      expect(screen.getByRole("combobox")).toHaveValue("unknown");
      expect(screen.getByText(en ? /The legacy record only says reviewed/ : /旧记录只有已核验标记/)).toBeVisible();
      expect(screen.getByRole("option", { name: en ? "Not applicable (basis required)" : "不适用（需说明依据）" })).toBeDisabled();
      expect(mocked).not.toHaveBeenCalled();
    });
    it(`${locale}: reviews a check in place, keeps failures visible, then shows the completed state`, async () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const user = userEvent.setup(); const updated = vi.fn();
      const checkItem = { ...item, id: "check-initial.pdf.confirm", notApplicableAllowed: true };
      const pending = { ...catalog, checklist: [checkItem] };
      const view = render(<I18nProvider><CheckRequirementReviews workspaceId="synthetic" catalog={pending} focusItemId={checkItem.id} onUpdated={updated} /></I18nProvider>);
      expect(screen.getByRole("region", { name: en ? "Complete reviews here" : "在此完成核验" })).toHaveTextContent(en ? "Saving refreshes local checks automatically" : "保存后自动更新本地检查");
      expect(document.activeElement).toHaveAttribute("data-review-id", checkItem.id);
      await user.selectOptions(screen.getByRole("combobox"), "compliant");
      mocked.mockRejectedValueOnce("投稿材料无效：REQUIREMENT_DECISION_INVALID");
      await user.click(screen.getByRole("button", { name: en ? "Save review" : "保存核验结果" }));
      expect(await screen.findByRole("alert")).toHaveTextContent(localizeBackendText(locale, "REQUIREMENT_DECISION_INVALID"));
      expect(updated).not.toHaveBeenCalled();
      const complete: SubmissionMaterialCatalog = { ...pending, checkCurrent: true, checklist: [{ ...checkItem, decision: "compliant", status: "passed" }] };
      mocked.mockResolvedValueOnce(complete);
      await user.click(screen.getByRole("button", { name: en ? "Save review" : "保存核验结果" }));
      await waitFor(() => expect(updated).toHaveBeenCalledWith(complete));
      view.rerender(<I18nProvider><CheckRequirementReviews workspaceId="synthetic" catalog={complete} onUpdated={updated} /></I18nProvider>);
      expect(screen.getByText(en ? /Journal template · Complete/ : /期刊模板 · 已完成/)).toBeVisible();
      expect(confirmationReviewed(complete, "initial.pdf.confirm")).toBe(true);
      expect(confirmationReviewed(pending, "initial.pdf.confirm")).toBe(false);
    });
    it(`${locale}: explains local migration and automatic presence checks without untranslated codes`, () => {
      for (const code of ["REQUIREMENTS_UPGRADED_FROM_LOCAL_EVIDENCE", "REQUIREMENT_PRESENCE_CHECK"]) {
        const translated = localizeBackendText(locale, code);
        expect(translated).not.toContain(code);
        expect(translated).toMatch(en ? /local|presence/ : /本地|存在/);
      }
    });
    it(`${locale}: rebinds historical files explicitly and exposes integrity failures`, async () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const user = userEvent.setup(); const updated = vi.fn();
      const file = { materialId: "old-file", kind: "title_page" as const, originalName: "Synthetic-title.docx", extension: "docx", sizeBytes: 100, contentHash: "a".repeat(64), importedUnixMs: 1, manuscriptVersion: 1, targetSelectionId: null, requirementSnapshotId: null, checklistItemId: null, included: false, validationStatus: "passed" as const, validationIssues: [], detectedMediaType: null };
      render(<I18nProvider><HistoricalMaterialReuse workspaceId="synthetic" material={file} items={[{ ...item, id: "current-title", verification: "file", materialKind: "title_page", label: "标题页", labelEn: "Title page" }]} onUpdated={updated} /></I18nProvider>);
      await user.click(screen.getByText(en ? "Review and reuse this file" : "核对后沿用此文件"));
      const save = screen.getByRole("button", { name: en ? "Confirm applicability and reuse" : "确认适用并沿用" });
      expect(save).toBeEnabled();
      await user.click(save);
      expect(screen.getByRole("combobox")).toHaveFocus();
      expect(mocked).not.toHaveBeenCalled();
      await user.selectOptions(screen.getByRole("combobox"), "current-title");
      mocked.mockRejectedValueOnce("投稿材料无效：MATERIAL_REUSE_INVALID");
      await user.click(save); expect(await screen.findByRole("alert")).toHaveTextContent(en ? "Choose the file again" : "重新选择文件");
      mocked.mockResolvedValueOnce(catalog); await user.click(save);
      await waitFor(() => expect(updated).toHaveBeenCalledWith(catalog));
      expect(mocked).toHaveBeenLastCalledWith("reuse_previous_material", { workspaceId: "synthetic", materialId: "old-file", checklistItemId: "current-title" });
    });
    it(`${locale}: keeps four decisions distinct, requires applicability evidence, and reports save failures`, async () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const user = userEvent.setup();
      const updated = vi.fn();
      mocked.mockRejectedValueOnce("投稿材料无效：REQUIREMENT_DECISION_INVALID");
      render(<I18nProvider><RequirementDecision workspaceId="synthetic" item={{ ...item, notApplicableAllowed: true }} onUpdated={updated} /></I18nProvider>);
      const select = screen.getByLabelText(en ? "Review outcome" : "核验结果");
      expect(within(select).getAllByRole("option")).toHaveLength(4);
      await user.selectOptions(select, "not_applicable");
      const save = screen.getByRole("button", { name: en ? "Save review" : "保存核验结果" });
      expect(save).toBeEnabled();
      await user.click(save);
      expect(screen.getByLabelText(en ? "Evidence or unresolved question" : "核验依据或待解决问题")).toHaveFocus();
      expect(mocked).not.toHaveBeenCalled();
      await user.type(screen.getByLabelText(en ? "Evidence or unresolved question" : "核验依据或待解决问题"), "Synthetic condition is not met");
      await user.click(save);
      expect(await screen.findByRole("alert")).toHaveTextContent(localizeBackendText(locale, "REQUIREMENT_DECISION_INVALID"));
      expect(updated).not.toHaveBeenCalled();
      mocked.mockResolvedValueOnce(catalog);
      await user.selectOptions(select, "noncompliant");
      await user.click(save);
      expect(mocked).toHaveBeenLastCalledWith("decide_submission_requirement", expect.objectContaining({ decision: "noncompliant", reason: "Synthetic condition is not met" }));
      await waitFor(() => expect(updated).toHaveBeenCalledWith(catalog));
    });
    it(`${locale}: routes a pending item to its stable ID and focuses the material row`, async () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const user = userEvent.setup(); const navigate = vi.fn();
      const view = render(<I18nProvider><PendingTasks catalog={catalog} onOpenStage={navigate} /></I18nProvider>);
      await user.click(screen.getByRole("button", { name: /Journal template|期刊模板/ }));
      expect(navigate).toHaveBeenCalledWith("materials", item.id);
      view.unmount();
      render(<I18nProvider><SubmissionMaterialsCenter focusItemId={item.id} workspace={{ id: "synthetic", contentHash: "a".repeat(64), importedUnixMs: 1, snapshotVersion: 1, manuscript: { name: "synthetic.pdf", kind: "pdf", extension: "pdf", sizeBytes: 100, modifiedUnixMs: 1 } }} catalog={catalog} busy={false} confirmingRequirementId={null} targetReady onAdd={vi.fn()} onSetIncluded={vi.fn()} onDelete={vi.fn()} onConfirm={vi.fn()} onCatalogUpdated={vi.fn()} onContinue={vi.fn()} /></I18nProvider>);
      await waitFor(() => expect(document.getElementById(`requirement-${item.id}`)).toHaveFocus());
      expect(screen.getByRole("button", { name: en ? "Run local basic checks" : "运行本地基础检查" })).toBeEnabled();
    });
    it(`${locale}: saves an evidence-bound structure overlay and opens the original through Rust`, async () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const user = userEvent.setup(); const saved = vi.fn(); mocked.mockResolvedValue({});
      render(<I18nProvider><StructureReview report={{ workspaceId: "synthetic", sourceContentHash: "a".repeat(64), sourceSnapshotVersion: 1, authors: ["Synthetic Author"], recognitions: [{ id: "figure-1", kind: "figure", label: "1", text: "Synthetic caption", page: 2, status: "candidate", parserVersion: 9, sourceVersion: 1 }] }} onSaved={saved} /></I18nProvider>);
      await user.click(screen.getByText(en ? "Review authors and captions (pages and source text)" : "核对作者与图表（页码及原文）"));
      await user.click(screen.getByRole("button", { name: en ? "Open source and compare pages" : "打开原稿对照页码" }));
      expect(mocked).toHaveBeenCalledWith("open_manuscript_source", { workspaceId: "synthetic" });
      await user.selectOptions(screen.getByLabelText(en ? "Caption status" : "题注状态"), "excluded");
      await user.type(screen.getByLabelText(en ? "Review note (required)" : "核对说明（必填）"), "Synthetic body reference excluded");
      await user.click(screen.getByRole("button", { name: en ? "Save review and update checklist" : "保存核对记录并更新清单" }));
      await waitFor(() => expect(saved).toHaveBeenCalled());
      expect(mocked).toHaveBeenLastCalledWith("review_manuscript_structure", expect.objectContaining({ input: expect.objectContaining({ sourceContentHash: "a".repeat(64), objects: [expect.objectContaining({ status: "excluded", page: 2 })] }) }));
    });
    it(`${locale}: creates a linked source only after explicit association confirmation`, async () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      mocked.mockImplementation(async command => command === "list_linked_sources" ? [] : command === "select_manuscript" ? { status: "selected", selectionId: "picked", manuscript: { name: "Synthetic.tex", kind: "latex" } } : { id: "linked" });
      const user = userEvent.setup(); const open = vi.fn();
      render(<I18nProvider><SourceRecovery workspaceId="pdf" isPdf onOpenWorkspace={open} /></I18nProvider>);
      await user.click(screen.getByRole("button", { name: en ? "Choose source to continue" : "选择源稿继续准备" }));
      const create = await screen.findByRole("button", { name: en ? "Link and continue" : "建立关联并继续" });
      expect(create).toBeEnabled();
      await user.click(create);
      expect(screen.getByRole("checkbox")).toHaveFocus();
      expect(mocked).not.toHaveBeenCalledWith("create_linked_source", expect.anything());
      await user.click(screen.getByRole("checkbox")); await user.click(create);
      await waitFor(() => expect(open).toHaveBeenCalledWith("linked"));
      expect(mocked).toHaveBeenCalledWith("create_linked_source", { workspaceId: "pdf", selectionId: "picked", authorConfirmed: true });
    });
  }
});
