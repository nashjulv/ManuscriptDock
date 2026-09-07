import { GuidedButton } from "./ActionGuidance";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { localizeBackendText, useI18n } from "./i18n";

export interface RecognitionObject { id: string; kind: string; label: string; text: string; page: number; status: string; parserVersion: number; sourceVersion: number }
export interface ReviewableStructure { workspaceId: string; sourceContentHash: string; sourceSnapshotVersion: number; authors: string[]; recognitions?: RecognitionObject[]; reviewId?: string | null }

export function StructureReview({ report, onSaved }: { report: ReviewableStructure; onSaved: () => void }) {
  const { locale, text } = useI18n();
  const [authors, setAuthors] = useState(report.authors.join("\n"));
  const [objects, setObjects] = useState((report.recognitions ?? []).filter(object => object.kind !== "author"));
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const candidateCount = objects.filter(object => object.status === "candidate").length;
  async function perform(action: () => Promise<unknown>) {
    setBusy(true); setError("");
    try { await action(); } catch (error) { setError(localizeBackendText(locale, String(error))); } finally { setBusy(false); }
  }
  return <details className="structure-review"><summary>{text("核对作者与图表（页码及原文）", "Review authors and captions (pages and source text)")}</summary>
    <p>{text("图表数量来自编号题注；版面中的表格区域另属候选。PDF 类型置信度不代表识别正确率。", "Figure and table counts come from numbered captions; layout regions are separate candidates. PDF classification confidence is not extraction accuracy.")}</p>
    {candidateCount ? <p>{text(`${candidateCount} 项题注待核对，未计入确定数量。`, `${candidateCount} candidate captions need review and are excluded from detected counts.`)}</p> : null}
    <button type="button" className="secondary-button" disabled={busy} onClick={() => void perform(() => invoke("open_manuscript_source", { workspaceId: report.workspaceId }))}>{text("打开原稿对照页码", "Open source and compare pages")}</button>
    <label>{text("作者与顺序（每行一位）", "Authors in order (one per line)")}<textarea value={authors} onChange={event => setAuthors(event.target.value)} rows={5} /></label>
    <ol>{objects.map((object, index) => <li key={object.id}><strong>{text(object.kind === "figure" ? "图" : "表", object.kind === "figure" ? "Figure" : "Table")} {object.label} · {text(`第 ${object.page} 页`, `Page ${object.page}`)}</strong><blockquote>{object.text}</blockquote><label>{text("题注状态", "Caption status")}<select value={object.status} onChange={event => setObjects(current => current.map((entry, i) => i === index ? { ...entry, status: event.target.value } : entry))}><option value="detected">{text("自动识别", "Detected")}</option><option value="candidate">{text("待核对", "Candidate")}</option><option value="confirmed">{text("已核实", "Verified")}</option><option value="excluded">{text("误识别／排除", "False match / exclude")}</option></select></label></li>)}</ol>
    <button type="button" onClick={() => setObjects(current => [...current, { id: `new-${current.length}`, kind: "figure", label: "?", text: "", page: 1, status: "candidate", parserVersion: 9, sourceVersion: report.sourceSnapshotVersion }])}>{text("补充漏识别的题注", "Add a missing caption")}</button>
    {objects.map((object, index) => object.id.startsWith("new-") ? <fieldset key={object.id}><legend>{text("补充题注", "Missing caption")}</legend><label>{text("类型", "Type")}<select value={object.kind} onChange={event => setObjects(current => current.map((entry,i) => i === index ? { ...entry, kind: event.target.value } : entry))}><option value="figure">{text("图", "Figure")}</option><option value="table">{text("表", "Table")}</option></select></label><label>{text("编号", "Number")}<input value={object.label} onChange={event => setObjects(current => current.map((entry,i) => i === index ? { ...entry, label: event.target.value } : entry))} /></label><label>{text("页码", "Page")}<input type="number" min={1} value={object.page} onChange={event => setObjects(current => current.map((entry,i) => i === index ? { ...entry, page: Number(event.target.value) } : entry))} /></label><label>{text("原文题注", "Original caption")}<input value={object.text} onChange={event => setObjects(current => current.map((entry,i) => i === index ? { ...entry, text: event.target.value } : entry))} /></label></fieldset> : null)}
    <label>{text("核对说明（必填）", "Review note (required)")}<textarea data-review-reason value={reason} maxLength={2000} onChange={event => setReason(event.target.value)} /></label>
    <GuidedButton className="secondary-button" type="button" disabled={busy} prerequisite={!reason.trim() && { message: text("请填写核对说明，记录调整依据。", "Enter a review note explaining the basis for your changes."), scope: ".structure-review", target: "[data-review-reason]" }} onClick={() => void perform(async () => { await invoke("review_manuscript_structure", { workspaceId: report.workspaceId, input: { sourceContentHash: report.sourceContentHash, baseReviewId: report.reviewId ?? null, authors: authors.split("\n").map(name => name.trim()).filter(Boolean), objects, reason } }); onSaved(); })}>{text("保存核对记录并更新清单", "Save review and update checklist")}</GuidedButton>
    <p>{text("内容未变时保留完成状态；实际修改只影响相关核验，原稿保持不变。", "Unchanged content keeps its completion state. Actual changes affect only related reviews; the source remains unchanged.")}</p>
    {error ? <p role="alert">{error}</p> : null}
  </details>;
}
