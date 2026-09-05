import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { localizeBackendText, useI18n } from "./i18n";

export interface OfficialFetchOptions { approvedOrigins: string[]; httpOrigins: string[]; }
export interface OfficialAccessEvent { requestedUrl: string; url: string; code: string; detail: string | null; }
export interface PendingAccess { origin: string; kind: "origin" | "http"; }
export interface OfficialFetchResult<T = unknown> {
  runId: string; snapshot: T | null; events: OfficialAccessEvent[]; pending: PendingAccess[]; partial: boolean; options: OfficialFetchOptions;
}
export type DiscoverOfficialSource<T = unknown> = (selectionId: string, options: OfficialFetchOptions) => Promise<OfficialFetchResult<T> | undefined>;

const EMPTY_OPTIONS: OfficialFetchOptions = { approvedOrigins: [], httpOrigins: [] };

export function OfficialSourceAccess({ workspaceId, selectionId, homepageUrl, busy, onDiscover }: {
  workspaceId: string; selectionId: string; homepageUrl: string; busy: boolean;
  onDiscover: (options: OfficialFetchOptions) => Promise<OfficialFetchResult | undefined>;
}) {
  const { locale, text } = useI18n();
  const [result, setResult] = useState<OfficialFetchResult | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [approved, setApproved] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [cancelled, setCancelled] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const generation = useRef(0);
  useEffect(() => {
    const current = ++generation.current;
    setResult(null); setConfirmed(false); setApproved([]); setError(null); setCancelled(false);
    void invoke<OfficialFetchResult | null>("get_journal_source_access", { workspaceId, targetSelectionId: selectionId })
      .then((record) => { if (current === generation.current) setResult(record); })
      .catch(() => { /* Old workspaces may not have an access record yet. */ });
    return () => { generation.current++; };
  }, [workspaceId, selectionId, homepageUrl]);

  const choices: PendingAccess[] = [];
  if (result && !cancelled) {
    const options = result.options ?? EMPTY_OPTIONS;
    for (const choice of [
      ...options.approvedOrigins.map((origin): PendingAccess => ({ origin, kind: "origin" })),
      ...options.httpOrigins.map((origin): PendingAccess => ({ origin, kind: "http" })),
      ...result.pending,
    ]) {
      if (!choices.some((item) => item.kind === choice.kind && item.origin === choice.origin)) choices.push(choice);
    }
  }
  const key = (choice: PendingAccess) => `${choice.kind}:${choice.origin}`;
  const fetch = async () => {
    const current = ++generation.current;
    const options = {
      approvedOrigins: choices.filter((item) => item.kind === "origin" && approved.includes(key(item))).map((item) => item.origin),
      httpOrigins: choices.filter((item) => item.kind === "http" && approved.includes(key(item))).map((item) => item.origin),
    };
    setConfirmed(false); setApproved([]); setError(null); setCancelled(false);
    const next = await onDiscover(options);
    if (current === generation.current && next) setResult(next);
  };
  const cancel = async () => {
    setCancelling(true); setError(null); setConfirmed(false); setApproved([]);
    try {
      await invoke("cancel_journal_source_access", { workspaceId, targetSelectionId: selectionId });
      setCancelled(true);
    } catch { setError("OFFICIAL_AUDIT_FAILED"); }
    finally { setCancelling(false); }
  };
  let protocol: string | null = null;
  try { const url = new URL(homepageUrl); if (["http:", "https:"].includes(url.protocol) && !url.username && !url.password) protocol = url.protocol; } catch { /* invalid URL */ }
  if (!protocol) return <p role="alert">{localizeBackendText(locale, "OFFICIAL_INVALID_URL")}</p>;
  return <div className="network-consent official-source-access">
    {protocol === "http:" ? <p>{text("当前记录使用 HTTP；授权后先尝试对应的 HTTPS 地址。", "The recorded URL uses HTTP. After authorization, the corresponding HTTPS URL is tried first.")}</p> : null}
    <label><input type="checkbox" checked={confirmed} disabled={busy || cancelling} onChange={(event) => setConfirmed(event.target.checked)} />{text("仅本次允许后端读取该期刊公开页面", "Allow the backend to read this journal's public pages for this request only")}</label>
    {choices.map((choice) => <label key={key(choice)}><input type="checkbox" disabled={busy || cancelling} checked={approved.includes(key(choice))} onChange={(event) => setApproved((current) => event.target.checked ? [...current, key(choice)] : current.filter((value) => value !== key(choice)))} /><span>{choice.kind === "http"
      ? text(`仅本次允许读取 ${choice.origin} 的公开 HTTP 页面；传输不加密，不发送论文或登录凭据。`, `For this request only, allow public HTTP pages at ${choice.origin}. Traffic is unencrypted; no manuscript or login credentials are sent.`)
      : text(`我确认 ${choice.origin} 属于期刊或出版社官方来源，并仅授权本次访问。`, `I confirm ${choice.origin} is an official journal or publisher source and authorize access for this request only.`)}</span></label>)}
    <button className="secondary-button" type="button" disabled={busy || cancelling || !confirmed || choices.some((choice) => !approved.includes(key(choice)))} onClick={() => void fetch()}>{busy ? text("正在读取官方页面…", "Reading official pages…") : text("获取官方投稿要求", "Capture official requirements")}</button>
    {choices.length > 0 ? <button className="text-button" type="button" disabled={busy || cancelling} onClick={() => void cancel()}>{text("取消额外访问，改用粘贴原文", "Cancel additional access and paste official text")}</button> : null}
    <small>{text("最多读取 4 个指南候选页面，另含受限的重定向和动态正文请求；每次响应不超过 2 MB。访问结果保存在本机。", "Reads at most four candidate pages, plus bounded redirects and dynamic-text requests; each response is limited to 2 MB. Access results are stored locally.")}</small>
    {cancelled ? <p role="status">{text("已取消额外访问，请在下方粘贴官方原文。", "Additional access cancelled. Paste official text below.")}</p> : null}
    {error ? <p role="alert">{localizeBackendText(locale, error)}</p> : null}
    {result ? <div className="official-access-result" role="status">
      <strong>{result.partial ? text("获取未完成 · 仍需确认或补充官方原文", "Capture incomplete · confirmation or official text is still needed") : text("本次页面读取完成，请核对要求快照", "Page reading complete; review the requirement snapshot")}</strong>
      <details><summary>{text("查看访问地址与结果", "View accessed URLs and results")}</summary><ul>{result.events.map((event, index) => <li key={`${index}:${event.url}`}><span>{localizeBackendText(locale, event.code)}{event.detail ? ` (${event.detail})` : ""}</span><code>{event.requestedUrl}</code>{event.url !== event.requestedUrl ? <code>→ {event.url}</code> : null}</li>)}</ul></details>
    </div> : null}
  </div>;
}
