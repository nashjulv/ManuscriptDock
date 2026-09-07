import { GuidedButton } from "./ActionGuidance";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { localizeBackendText, useI18n } from "./i18n";
import type { SubmissionMaterialCatalog, SubmissionMaterialChecklistItem } from "./App";

export function RequirementDecision({ workspaceId, item, onUpdated, disabled = false }: { workspaceId: string; item: SubmissionMaterialChecklistItem; onUpdated: (catalog: SubmissionMaterialCatalog) => void; disabled?: boolean }) {
  const { locale, text } = useI18n();
  const [decision, setDecision] = useState(item.decision === "legacy_reviewed" ? "unknown" : item.decision ?? "unknown");
  const [reason, setReason] = useState(item.decisionReason ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function save() {
    setBusy(true); setError("");
    try { onUpdated(await invoke<SubmissionMaterialCatalog>("decide_submission_requirement", { workspaceId, itemId: item.id, decision, reason })); }
    catch (error) { setError(localizeBackendText(locale, String(error))); }
    finally { setBusy(false); }
  }
  return <fieldset className="requirement-decision" disabled={disabled || busy}>
    <legend>{text("对照原文记录核验结果", "Record your review against the source")}</legend>
    {item.decision === "legacy_reviewed" ? <p>{text("旧记录只有已核验标记，请明确核验结果。", "The legacy record only says reviewed. Record an explicit outcome.")}</p> : null}
    <label>{text("核验结果", "Review outcome")}<select value={decision} onChange={event => setDecision(event.target.value)}>
      <option value="unknown">{text("不确定／待补依据", "Uncertain / evidence needed")}</option>
      <option value="compliant">{text("符合（已按原文核实）", "Compliant (verified against source)")}</option>
      <option value="noncompliant">{text("不符合（需要修改）", "Noncompliant (changes needed)")}</option>
      <option value="not_applicable" disabled={!item.notApplicableAllowed}>{text("不适用（需说明依据）", "Not applicable (basis required)")}</option>
    </select></label>
    <label>{text("核验依据或待解决问题", "Evidence or unresolved question")}<textarea value={reason} maxLength={2000} onChange={event => setReason(event.target.value)} required={decision === "not_applicable"} /></label>
    <small>{item.notApplicableAllowed ? text("仅在要求不适用于本文时选择“不适用”，并说明适用范围和依据；尚未准备好不等于不适用。", "Use Not applicable only when the requirement does not apply to this manuscript, explaining its scope and your basis. Not yet prepared does not mean not applicable.") : text("此项为明确的无条件要求，不能在此免除。", "This is an explicit unconditional requirement and cannot be waived here.")}</small>
    <GuidedButton className="secondary-button" type="button" disabled={disabled || busy} prerequisite={decision === "not_applicable" && !reason.trim() && { message: text("请说明该要求为何不适用于本次投稿。", "Explain why this requirement does not apply to this submission."), scope: ".requirement-decision", target: "textarea" }} onClick={() => void save()}>{busy ? text("保存中…", "Saving…") : text("保存核验结果", "Save review")}</GuidedButton>
    {error ? <p role="alert">{error}</p> : null}
  </fieldset>;
}
