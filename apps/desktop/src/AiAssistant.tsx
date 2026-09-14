import { useEffect, useRef, useState, type ReactNode } from "react";
import { api } from "./shared/ipc";
import type { AiEvidence, AiPreview, AiRun, AiSettings, AiTask, AppError } from "./shared/contracts";
import { localizeBackendText, useI18n } from "./i18n";

function Dialog({ title, busy, onClose, children }: { title: string; busy: boolean; onClose: () => void; children: ReactNode }) {
  const { text } = useI18n();
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => { const node = ref.current; node?.showModal(); return () => node?.close(); }, []);
  return <dialog ref={ref} className="ai-dialog" aria-label={title} onCancel={event => { event.preventDefault(); if (!busy) onClose(); }}>
    <header><h2>{title}</h2><button disabled={busy} onClick={onClose}>{text("关闭", "Close")}</button></header>{children}
  </dialog>;
}
export function AiSettingsDialog({ onClose }: { onClose: () => void }) {
  const { locale, text } = useI18n();
  const [settings, setSettings] = useState<AiSettings | null>(null);
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const lock = useRef(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => { let active = true; void api.aiSettings().then(value => { if (active) setSettings(value); }).catch((e: AppError) => { if (active) setError(e.code); }); return () => { active = false; }; }, []);
  const save = async () => {
    if (!settings || lock.current) return;
    lock.current = true; setBusy(true); setError(null);
    try { const { hasKey: _, ...input } = settings; await api.saveAiSettings({ ...input, apiKey: key }); setKey(""); onClose(); }
    catch (e) { setError((e as AppError).code); }
    finally { lock.current = false; setBusy(false); }
  };
  return <Dialog title={text("AI 辅助设置", "AI assistance settings")} busy={busy} onClose={onClose}>
    <p>{text("基础功能无需配置模型。AI 辅助按需启用，每次调用前预览资料并单独确认。", "Core features need no model setup. AI assistance is optional, with a source preview and confirmation for each request.")}</p>
    {error && <p role="alert">{localizeBackendText(locale, error)}</p>}
    {settings ? <form className="ai-settings" onSubmit={event => { event.preventDefault(); void save(); }}>
      <fieldset disabled={busy}>
        <label><input type="checkbox" checked={settings.enabled} onChange={e => setSettings({ ...settings, enabled: e.target.checked })}/>{text("启用 AI 辅助", "Enable AI assistance")}</label>
        <label><input type="checkbox" checked={settings.local} onChange={e => setSettings({ ...settings, local: e.target.checked })}/>{text("使用本机模型服务（仅回环地址）", "Use an on-device model service (loopback only)")}</label>
        <label>{text("兼容 Chat Completions 的服务地址", "Chat Completions-compatible service URL")}<input type="url" value={settings.endpoint} required={settings.enabled} placeholder={settings.local ? "http://127.0.0.1:11434/v1" : "https://…/v1"} onChange={e => setSettings({ ...settings, endpoint: e.target.value })}/></label>
        <label>{text("模型名称", "Model name")}<input value={settings.model} required={settings.enabled} maxLength={200} onChange={e => setSettings({ ...settings, model: e.target.value })}/></label>
        <label>{text("API 密钥", "API key")}<input type="password" autoComplete="new-password" value={key} onChange={e => setKey(e.target.value)}/></label>
        <small>{settings.hasKey ? text("密钥已保存。仅同一服务地址留空时保留；更换地址需重新输入。", "A key is saved. Leave blank to keep it only for the same service URL; enter a new key when changing services.") : text("密钥保存在系统凭据库；本机服务可留空。", "Keys are stored in the system credential store; on-device services may omit a key.")}</small>
        <label>{text("最大输出 token 数", "Maximum output tokens")}<input type="number" min={512} max={8000} value={settings.maxOutputTokens} onChange={e => setSettings({ ...settings, maxOutputTokens: Number(e.target.value) })}/></label>
      </fieldset>
      <p>{text("保存设置不会调用模型。外部服务按其定价计费；不会自动重试或切换服务。", "Saving settings makes no model request. External providers charge at their own rates. Requests are never automatically retried or switched to another provider.")}</p>
      <button className="primary" disabled={busy}>{busy ? text("保存中…", "Saving…") : text("保存设置", "Save settings")}</button>
    </form> : !error && <p role="status">{text("正在读取设置…", "Loading settings…")}</p>}
  </Dialog>;
}
export function AiWorkbench({ projectId, task, materialId, existingRun, onClose, onChanged }: { projectId: string; task: AiTask; materialId?: string; existingRun?: AiRun; onClose: () => void; onChanged: () => void }) {
  const { locale, text, localize } = useI18n();
  const [preview, setPreview] = useState<AiPreview | null>(null);
  const [result, setResult] = useState<AiRun | null>(existingRun ?? null);
  const [error, setError] = useState<string | null>(null);
  const [consent, setConsent] = useState(false);
  const [busy, setBusy] = useState(false);
  const [sent, setSent] = useState(false);
  const lock = useRef(false);
  useEffect(() => { if (existingRun) return; let active = true; void api.prepareAi(projectId, task, materialId).then(value => { if (active) setPreview(value); }).catch((e: AppError) => { if (active) setError(e.code); }); return () => { active = false; }; }, [projectId, task, materialId, existingRun]);
  const execute = async (accept = false) => {
    if (lock.current || (!accept && (!preview || !consent || sent))) return;
    lock.current = true; setBusy(true); setError(null);
    try {
      if (accept && result) { const acceptedPath = await api.acceptAiDraft(projectId, result.id); setResult({ ...result, acceptedPath }); }
      else if (preview) { setSent(true); setResult(await api.runAi(preview.id)); }
      onChanged();
    } catch (e) { setError((e as AppError).code); onChanged(); }
    finally { lock.current = false; setBusy(false); }
  };
  const evidence = (items: AiEvidence[]) => <details><summary>{text("查看依据", "View evidence")}</summary>{items.map((item, index) => <blockquote key={index}><small>{item.sourceId}</small><p>{item.quote}</p></blockquote>)}</details>;
  return <Dialog title={text("AI 材料助手", "AI material assistant")} busy={busy} onClose={onClose}>
    <p>{text("AI 仅提供建议，不能确认研究事实或替代作者核对。稿件输入仅含摘要与填写的信息，不是主稿全文检查。", "AI provides suggestions and cannot verify research facts or replace author review. Manuscript input includes only the abstract and entered facts, not a full-text review.")}</p>
    {error && <p role="alert">{localizeBackendText(locale, error)}</p>}
    {!result && preview && <>
      <div className="ai-service-summary"><strong>{preview.local ? text("本机模型服务", "On-device model service") : text("外部模型服务 · 将发送以下资料", "External model service · the following sources will be sent")}</strong><code>{preview.provider}</code><span>{preview.model}</span></div>
      <p>{text(`预计输入约 ${preview.estimatedInputTokens.toLocaleString(locale)} token，最多输出 ${preview.maxOutputTokens.toLocaleString(locale)} token。费用取决于服务商，估算不代表账单。`, `Estimated input: ${preview.estimatedInputTokens.toLocaleString(locale)} tokens; output limit: ${preview.maxOutputTokens.toLocaleString(locale)} tokens. Pricing depends on your provider; this estimate is not a bill.`)}</p>
      <div className="ai-sources">{preview.input.sources.map(source => <details key={source.id}><summary>{localize(source.label)}</summary><pre>{source.text}</pre></details>)}</div>
      <label className="ai-consent"><input type="checkbox" disabled={busy || sent} checked={consent} onChange={e => setConsent(e.target.checked)}/>{text("我已检查以上资料，有权将其交由此服务处理，并同意本次调用。", "I have reviewed the sources, have permission to process them with this service, and approve this request.")}</label>
      <button className="primary" disabled={busy || !consent || sent} onClick={() => void execute()}>{busy ? text("正在处理，请勿重复发起…", "Processing; do not submit again…") : sent ? text("本次请求已发起", "Request already submitted") : preview.local ? text("运行一次", "Run once") : text("发送并执行一次", "Send and run once")}</button>
    </>}
    {!result && !preview && !error && <p role="status">{text("正在准备资料预览…", "Preparing source preview…")}</p>}
    {result && <div className="ai-result">
      <p><strong>{result.model}</strong> · {new Date(result.createdAt * 1000).toLocaleString(locale)}</p><code>{result.provider}</code>
      {result.status === "started" && <p role="status">{text("请求已记录，结果尚未确认。请稍后查看记录，不要重复发起。", "The request is recorded but its outcome is unknown. Check the history later before submitting again.")}</p>}
      {result.errorCode && <p role="alert">{localizeBackendText(locale, result.errorCode)}</p>}
      {result.acceptedPath ? <p role="status">{text("已保存为待核对草稿，请在材料清单中检查并确认。", "Saved as a draft for review. Check and confirm it in the material checklist.")} <code>{result.acceptedPath}</code></p> : !result.current && <p role="status">{text("资料或文件已变化，此结果已过期。请关闭后重新检查。", "Sources or files have changed. This result is outdated. Close and check again.")}</p>}
      {result.output && <>
        <strong>{text("AI 建议 · 需作者核对", "AI suggestions · author review required")}</strong>
        {result.output.paragraphs.map((paragraph, index) => <article key={index}><p>{paragraph.text}</p>{evidence(paragraph.evidence)}</article>)}
        {result.output.findings.map((finding, index) => <article key={index}><p>{localize(finding.message)}</p>{evidence(finding.evidence)}</article>)}
        {!result.output.findings.length && <p>{text("未发现额外问题，不代表已通过投稿检查。", "No additional issues were reported. This does not mean submission checks passed.")}</p>}
        {result.output.paragraphs.length > 0 && <button className="primary" disabled={busy || !result.current || !!result.acceptedPath} onClick={() => void execute(true)}>{text("保存为待核对草稿", "Save as draft for review")}</button>}
      </>}
      {(result.inputTokens != null || result.outputTokens != null) && <p>{text("服务商报告的 token 用量", "Provider-reported token usage")}: {result.inputTokens?.toLocaleString(locale) ?? "—"} / {result.outputTokens?.toLocaleString(locale) ?? "—"} ({text("输入 / 输出", "input / output")})</p>}
    </div>}
  </Dialog>;
}
export function AiHistory({ projectId, refreshKey, onSelect }: { projectId: string; refreshKey: number; onSelect: (run: AiRun) => void }) {
  const { locale, text } = useI18n();
  const [runs, setRuns] = useState<AiRun[]>([]);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => { let active = true; const refresh = () => { void api.aiHistory(projectId).then(value => { if (active) { setRuns(value); setError(null); } }).catch((e: AppError) => { if (active) setError(e.code); }); }; refresh(); window.addEventListener("focus", refresh); return () => { active = false; window.removeEventListener("focus", refresh); }; }, [projectId, refreshKey]);
  return <details className="ai-history"><summary>{text("AI 使用记录与依据", "AI usage history and evidence")}</summary><p>{text("记录服务、模型、用途、输入版本和结果，供作者核对与撰写 AI 使用声明。声明仍需根据期刊政策由作者确认。", "Records retain the service, model, purpose, source versions and results for author review and AI disclosure. Authors must confirm disclosures against journal policy.")}</p>{error && <p role="alert">{localizeBackendText(locale, error)}</p>}{!runs.length && !error && <p>{text("暂无 AI 调用记录", "No AI requests yet")}</p>}{runs.map(run => <button key={run.id} onClick={() => onSelect(run)}>{run.model} · {run.task === "draft_cover_letter" ? text("投稿信草稿", "Cover letter draft") : run.task === "draft_highlights" ? text("亮点草稿", "Highlights draft") : run.task === "review_material" ? text("材料检查", "Material review") : text("一致性检查", "Consistency review")} · {run.status === "failed" ? text("调用失败", "Request failed") : run.status === "started" ? text("结果待确认", "Outcome unknown") : run.acceptedPath ? text("草稿已保存", "Draft saved") : run.current ? text("待核对", "Review needed") : text("已过期", "Outdated")} · {new Date(run.createdAt * 1000).toLocaleString(locale)}</button>)}</details>;
}
