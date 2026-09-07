import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SubmissionMaterialsCenter, TargetPackageAssembly, type SubmissionMaterialCatalog, type JournalRequirementItem } from "./App";
import { DECLARATION_MESSAGES, I18nProvider, localizeBackendText, type Locale } from "./i18n";
import { type DeclarationRequirement } from "./DeclarationRequirements";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: () => false }));
const invokeMock = vi.mocked(invoke);
const declaration: DeclarationRequirement = { delivery: ["manuscript", "attachment"], stage: "initial", condition: null, applicability: "applicable", fileCount: 1, allowedExtensions: ["pdf"], signatureRequired: true, stampRequired: false, sharedFileAllowed: false, templateUrl: "https://journal.example/form.pdf", authorNote: null };
const requirement: JournalRequirementItem = { id: "requirement-conflict", category: "conflict_of_interest", label: "利益冲突声明", labelEn: "Conflict-of-interest statement", obligation: "required", detail: "", sourceUrl: "https://journal.example/authors", evidenceExcerpt: "At submission authors must include a conflict of interest statement in the manuscript and upload a signed PDF attachment.", declaration };
const catalog: SubmissionMaterialCatalog = {
  schemaVersion: 4, workspaceId: "synthetic-workspace", manuscriptVersion: 1, recommendationReady: true, targetVerified: true, requiredComplete: false, targetCheckReady: false, workflowStatus: "materials_required", requiredTotal: 3, requiredCompleted: 0,
  declarationPlan: { manuscriptVersion: 1, targetSelectionId: "target", requirementSnapshotId: "snapshot", stage: "initial", requirements: [requirement] },
  materials: [{ materialId: "stored", kind: "declaration", originalName: "signed-declaration.pdf", extension: "pdf", contentHash: "a".repeat(64), sizeBytes: 123, importedUnixMs: 1, manuscriptVersion: 1, targetSelectionId: "target", requirementSnapshotId: "snapshot", checklistItemId: "common-declaration-files", included: true, validationStatus: "passed", validationIssues: [], detectedMediaType: "application/pdf" }],
  checklist: ["attachment", "manuscript", "attestation"].map(action => ({ id: `declaration-${action}`, label: requirement.label, labelEn: requirement.labelEn, group: "declarations", requirement: "required", status: action === "attachment" ? "missing" : "author_confirmation", detail: `DECLARATION_${action.toUpperCase()}`, verification: action === "attachment" ? "file" : "author", materialKind: action === "attachment" ? "declaration" : null, blocking: true, confirmable: action !== "attachment", sourceUrl: requirement.sourceUrl, evidenceExcerpt: requirement.evidenceExcerpt, capturedUnixMs: 1, freshUntilUnixMs: 9999999999999, requiredCount: action === "attachment" ? 1 : 0, matchedMaterialIds: [], declaration, declarationRequirementId: requirement.id, declarationAction: action })),
};
const workspace = { id: "synthetic-workspace", snapshotVersion: 1, manuscript: { name: "Synthetic.pdf", extension: "pdf", kind: "pdf" as const, sizeBytes: 100, modifiedUnixMs: 1 }, contentHash: "a".repeat(64), importedUnixMs: 1 };
const add = vi.fn(); const confirm = vi.fn();
function Harness({ initial = catalog }: { initial?: SubmissionMaterialCatalog }) {
  const [current, setCurrent] = useState(initial);
  return <I18nProvider><SubmissionMaterialsCenter workspace={workspace} catalog={current} busy={false} confirmingRequirementId={null} targetReady onAdd={add} onConfirm={confirm} onCatalogUpdated={setCurrent} onDelete={vi.fn()} onSetIncluded={vi.fn()} onContinue={vi.fn()} /></I18nProvider>;
}

describe("structured declaration requirements", () => {
  beforeEach(() => { invokeMock.mockReset(); add.mockReset(); confirm.mockReset(); window.localStorage.clear(); });
  for (const locale of ["zh-CN", "en"] as Locale[]) {
    const en = locale === "en";
    it(`${locale}: keeps file upload separate from confirmations and reuses stored files explicitly`, async () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const user = userEvent.setup(); render(<Harness />);
      await user.click(screen.getByRole("tab", { name: en ? /Upload/ : /上传资料/ }));
      const button = screen.getByRole("button", { name: en ? "Choose files for Conflict-of-interest statement" : "为利益冲突声明选择文件" });
      expect(button).toBeEnabled();
      expect(screen.getByText(en ? "Official formats: PDF" : "官方限定格式：PDF")).toBeVisible();
      expect(screen.getByText(en ? "Accepted: PDF" : "支持：PDF")).toBeVisible();
      expect(screen.getByRole("link", { name: en ? "View official template" : "查看官方模板" })).toHaveAttribute("href", "https://journal.example/form.pdf");
      await user.click(button); expect(add).toHaveBeenCalledWith("declaration", "declaration-attachment");
      await user.click(screen.getByRole("tab", { name: en ? /Requirements/ : /要求清单/ }));
      expect(screen.getAllByText(en ? "Official formats: PDF" : "官方限定格式：PDF")).toHaveLength(1);
      expect(screen.getByRole("button", { name: en ? /Run local basic checks/ : /运行本地基础检查/ })).toBeEnabled();
      const confirmations = screen.getAllByLabelText(en ? "Review outcome" : "核验结果");
      expect(confirmations).toHaveLength(2);
      await user.selectOptions(confirmations[0], "compliant");
      invokeMock.mockResolvedValueOnce(catalog);
      await user.click(screen.getAllByRole("button", { name: en ? "Save review" : "保存核验结果" })[0]);
      expect(invokeMock).toHaveBeenCalledWith("decide_submission_requirement", expect.objectContaining({ itemId: "declaration-manuscript", decision: "compliant" }));
      invokeMock.mockResolvedValueOnce(catalog);
      await user.selectOptions(screen.getByLabelText(en ? "Link a stored file" : "关联已有文件"), "stored");
      await user.click(screen.getByRole("button", { name: en ? "Link to this requirement" : "关联到此要求" }));
      expect(invokeMock).toHaveBeenCalledWith("update_declaration_plan", { workspaceId: "synthetic-workspace", update: { materialId: "stored", checklistItemId: "declaration-attachment", manuscriptVersion: 1, targetSelectionId: "target", requirementSnapshotId: "snapshot" } });
    });
    it(`${locale}: edits conditions and stage with a note, preserves errors, and refreshes saved state`, async () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const user = userEvent.setup(); render(<Harness />);
      await user.click(screen.getByRole("tab", { name: en ? /Requirements/ : /要求清单/ }));
      await user.click(screen.getByRole("button", { name: en ? "Verify or adjust requirement" : "核验或调整要求" }));
      const form = screen.getByRole("form", { name: en ? "Edit declaration requirement" : "编辑声明要求" });
      await user.selectOptions(within(form).getByLabelText(en ? "Required submission stage" : "要求的提交阶段"), "accepted");
      await user.selectOptions(within(form).getByLabelText(en ? "Applies to this manuscript" : "对当前论文是否适用"), "not_applicable");
      await user.type(within(form).getByLabelText(en ? "Verification basis and adjustment note" : "核验依据与调整说明"), "Synthetic official exception checked by author");
      invokeMock.mockRejectedValueOnce("投稿材料无效：DECLARATION_INVALID_UPDATE");
      await user.click(within(form).getByRole("button", { name: en ? "Save verification" : "保存核验结果" }));
      expect(await screen.findByRole("alert")).toHaveTextContent(DECLARATION_MESSAGES.DECLARATION_INVALID_UPDATE[en ? 1 : 0]);
      expect(form).toBeVisible();
      const updated = structuredClone(catalog); updated.declarationPlan!.requirements[0].declaration!.authorNote = "Synthetic official exception checked by author";
      invokeMock.mockResolvedValueOnce(updated);
      await user.click(within(form).getByRole("button", { name: en ? "Save verification" : "保存核验结果" }));
      expect(await screen.findByText(new RegExp("Synthetic official exception checked by author"))).toBeVisible();
      expect(screen.queryByRole("form")).not.toBeInTheDocument();
      const sent = invokeMock.mock.calls[0][1] as { update: { requirement: JournalRequirementItem } };
      expect(sent.update.requirement.declaration).toMatchObject({ stage: "accepted", applicability: "not_applicable", delivery: ["manuscript", "attachment"] });
      invokeMock.mockResolvedValueOnce({ ...updated, declarationPlan: { ...updated.declarationPlan!, stage: "revision" } });
      await user.selectOptions(screen.getByLabelText(en ? "Current submission stage" : "当前投稿阶段"), "revision");
      expect(invokeMock).toHaveBeenLastCalledWith("update_declaration_plan", expect.objectContaining({ update: expect.objectContaining({ stage: "revision" }) }));
    });
    it(`${locale}: adds an omitted requirement with its source, delivery and verification note`, async () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const user = userEvent.setup(); render(<Harness />);
      await user.click(screen.getByRole("tab", { name: en ? /Requirements/ : /要求清单/ }));
      await user.click(screen.getByRole("button", { name: en ? "Add a missing declaration requirement" : "补充遗漏的声明要求" }));
      const form = within(screen.getByRole("form", { name: en ? "Edit declaration requirement" : "编辑声明要求" }));
      await user.type(form.getByLabelText(en ? "Chinese label" : "中文名称"), "资金声明");
      await user.type(form.getByLabelText(en ? "English label" : "英文名称"), "Funding declaration");
      await user.type(form.getByLabelText(en ? "Official source URL" : "官方来源地址"), "https://publisher.example/funding");
      await user.type(form.getByLabelText(en ? "Official source excerpt" : "官方原文"), "At revision authors must complete the funding declaration in an online form.");
      await user.click(form.getByRole("checkbox", { name: en ? "Submission system entry" : "投稿系统填写" }));
      await user.selectOptions(form.getByLabelText(en ? "Required submission stage" : "要求的提交阶段"), "revision");
      await user.selectOptions(form.getByLabelText(en ? "Applies to this manuscript" : "对当前论文是否适用"), "applicable");
      await user.type(form.getByLabelText(en ? "Verification basis and adjustment note" : "核验依据与调整说明"), "Checked the official funding instructions");
      invokeMock.mockResolvedValueOnce(catalog);
      await user.click(form.getByRole("button", { name: en ? "Save verification" : "保存核验结果" }));
      expect(invokeMock).toHaveBeenCalledWith("update_declaration_plan", expect.objectContaining({ update: expect.objectContaining({ requirement: expect.objectContaining({ id: "", label: "资金声明", labelEn: "Funding declaration", sourceUrl: "https://publisher.example/funding", declaration: expect.objectContaining({ delivery: ["submission_system"], stage: "revision", applicability: "applicable" }) }) }) }));
    });
    it(`${locale}: exposes separate file slots and lets authors select the reuse destination`, async () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const initial = structuredClone(catalog);
      initial.declarationPlan!.requirements[0].declaration!.fileCount = 2;
      const original = initial.checklist[0];
      initial.checklist.splice(0, 1, ...[1, 2].map(index => ({ ...original, id: `file-${index}`, label: `利益冲突声明 · 文件 ${index}`, labelEn: `Conflict-of-interest statement · File ${index}`, declaration: { ...declaration, fileCount: 2 } })));
      const user = userEvent.setup(); render(<Harness initial={initial} />);
      await user.click(screen.getByRole("tab", { name: en ? /Upload/ : /上传资料/ }));
      await user.click(screen.getByRole("button", { name: en ? "Choose files for Conflict-of-interest statement · File 2" : "为利益冲突声明 · 文件 2选择文件" }));
      expect(add).toHaveBeenCalledWith("declaration", "file-2");
      await user.click(screen.getByRole("tab", { name: en ? /Requirements/ : /要求清单/ }));
      await user.selectOptions(screen.getByLabelText(en ? "Destination file slot" : "关联目标文件项"), "file-2");
      await user.selectOptions(screen.getByLabelText(en ? "Link a stored file" : "关联已有文件"), "stored");
      invokeMock.mockResolvedValueOnce(initial);
      await user.click(screen.getByRole("button", { name: en ? "Link to this requirement" : "关联到此要求" }));
      expect(invokeMock).toHaveBeenCalledWith("update_declaration_plan", expect.objectContaining({ update: expect.objectContaining({ materialId: "stored", checklistItemId: "file-2" }) }));
    });
    it(`${locale}: excludes later-stage files from the overview and stored-file selection`, async () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const initial = structuredClone(catalog);
      initial.materials[0].checklistItemId = "declaration-attachment";
      initial.checklist[0].status = "later"; initial.checklist[0].blocking = false;
      initial.checklist[0].matchedMaterialIds = ["stored"];
      initial.checklist[0].declaration!.stage = "accepted";
      const user = userEvent.setup(); render(<Harness initial={initial} />);
      expect(screen.getByText(en ? "Current package · 1 file(s) planned" : "当前准备包 · 1 个拟组包文件")).toBeVisible();
      await user.click(screen.getByRole("tab", { name: en ? /Stored files/ : /已存文件/ }));
      expect(screen.getByText(en ? "Later-stage material; excluded from the current package" : "后续阶段材料，当前投稿包不纳入")).toBeVisible();
      expect(screen.getByRole("checkbox", { name: en ? "Include" : "纳入组包" })).not.toBeChecked();
      expect(screen.getByRole("checkbox", { name: en ? "Include" : "纳入组包" })).toBeDisabled();
    });
    it(`${locale}: previews one combined file, localizes slot names, and disables deferred inclusion`, () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const plan = { schemaVersion: 1, workspaceId: "synthetic-workspace", manuscriptVersion: 1, targetSelectionId: "target", targetName: "Synthetic Journal", anonymousReview: false, ready: false, createdUnixMs: 1, externalTransmission: "not_performed" as const,
        files: [1, 2, 3].map(index => ({ materialId: `file-${index}`, displayName: "combined.pdf", relativePath: index < 3 ? "submission/declarations/combined.pdf" : "submission/declarations/later.pdf", role: "declaration", materialKind: "declaration" as const, checklistItemId: `slot-${index}`, checklistLabel: `声明 ${index}`, checklistLabelEn: `Declaration ${index}`, inclusionAvailable: index < 3, required: index < 3, included: index < 3, sizeBytes: 100, contentHash: "a".repeat(64), validationStatus: "passed", validationIssues: [] })),
        warnings: [], blockers: ['DECLARATION_CHECK:["声明：待核验","Declaration: needs verification"]'] };
      render(<I18nProvider><TargetPackageAssembly plan={plan} busy={false} onSetIncluded={vi.fn()} /></I18nProvider>);
      expect(screen.getByText(en ? "1 outgoing file(s)" : "1 个外发文件")).toBeVisible();
      expect(screen.getByText(en ? /Upload slot: Declaration 1/ : /对应上传项：声明 1/)).toBeVisible();
      expect(screen.getByText(en ? "Declaration: needs verification" : "声明：待核验")).toBeVisible();
      expect(screen.getAllByRole("checkbox")[2]).toBeDisabled();
    });
  }
  it("localizes every declaration status and wrapped backend failure in both locales", () => {
    for (const [code, [zh, en]] of Object.entries(DECLARATION_MESSAGES)) {
      expect(localizeBackendText("zh-CN", code)).toBe(zh); expect(localizeBackendText("en", code)).toBe(en);
      expect(localizeBackendText("zh-CN", `投稿材料无效：${code}`)).toBe(zh); expect(localizeBackendText("en", `投稿材料无效：${code}`)).toBe(en);
    }
    const message = 'DECLARATION_CHECK:["利益冲突声明：缺少附件","Conflict-of-interest statement: attachment missing"]';
    expect(localizeBackendText("zh-CN", message)).toBe("利益冲突声明：缺少附件");
    expect(localizeBackendText("en", message)).toBe("Conflict-of-interest statement: attachment missing");
    expect(localizeBackendText("en", "投稿材料无效：合成.zip：DECLARATION_SUPPORTED_FORMATS")).toBe(`合成.zip: ${DECLARATION_MESSAGES.DECLARATION_SUPPORTED_FORMATS[1]}`);
  });
});
