import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { localizeBackendText, useI18n } from "./i18n";

export interface OfficialFetchOptions { approvedOrigins: string[]; httpOrigins?: string[]; }
export interface OfficialAccessEvent { requestedUrl: string; url: string; code: string; detail: string | null; }
export interface PendingAccess { origin: string; kind: "origin" | "http"; }
export interface OfficialFetchResult<T = unknown> {
  runId: string; snapshot: T | null; events: OfficialAccessEvent[]; pending: PendingAccess[]; partial: boolean; options: OfficialFetchOptions;
}
export type DiscoverOfficialSource<T = unknown> = (selectionId: string, options: OfficialFetchOptions) => Promise<OfficialFetchResult<T> | undefined>;

const EMPTY_OPTIONS: OfficialFetchOptions = { approvedOrigins: [] };
interface HomepageCorrection { url: string; authorityUrl: string; verifiedOn: string; }

const INTERNAL_ACCESS_EVENTS = new Set(["OFFICIAL_REQUESTED", "OFFICIAL_VIRTUAL_DNS", "OFFICIAL_DNS_RECOVERED", "OFFICIAL_HTTP_USED", "OFFICIAL_REDIRECT", "OFFICIAL_SOURCE_CORRECTED", "OFFICIAL_GZIP_DECODED"]);
const CONNECTION_FAILURES = new Set(["OFFICIAL_CONNECTION_FAILED", "OFFICIAL_TLS_FAILED", "OFFICIAL_TIMEOUT", "OFFICIAL_HTTP_STATUS", "OFFICIAL_DNS_FAILED", "OFFICIAL_ENCRYPTED_DNS_FAILED"]);

export function visibleAccessEvents(events: OfficialAccessEvent[]): OfficialAccessEvent[] {
  const recovered = new Set<string>();
  const failed = new Set<string>();
  const visible: OfficialAccessEvent[] = [];
  for (let index = events.length - 1; index >= 0; index--) {
    const event = events[index];
    if (INTERNAL_ACCESS_EVENTS.has(event.code)) continue;
    if (event.code === "OFFICIAL_RECEIVED" || event.code === "OFFICIAL_CAPTURED") recovered.add(event.requestedUrl);
    if (CONNECTION_FAILURES.has(event.code)) {
      if (recovered.has(event.requestedUrl) || failed.has(event.requestedUrl)) continue;
      failed.add(event.requestedUrl);
    }
    visible.push(event);
  }
  return visible.reverse();
}

export function OfficialSourceAccess({ workspaceId, selectionId, homepageUrl, busy, onDiscover }: {
  workspaceId: string; selectionId: string; homepageUrl: string; busy: boolean;
  onDiscover: (options: OfficialFetchOptions) => Promise<OfficialFetchResult | undefined>;
}) {
  const { locale, text } = useI18n();
  const [result, setResult] = useState<OfficialFetchResult | null>(null);
  const [correction, setCorrection] = useState<HomepageCorrection | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [approved, setApproved] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [cancelled, setCancelled] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const visibleEvents = visibleAccessEvents(result?.events ?? []);
  const generation = useRef(0);
  useEffect(() => {
    const current = ++generation.current;
    setResult(null); setConfirmed(false); setApproved([]); setError(null); setCancelled(false);
    setCorrection(null);
    void invoke<HomepageCorrection | null>("get_journal_homepage_correction", { homepageUrl })
      .then((value) => { if (current === generation.current) setCorrection(value); })
      .catch(() => { /* The capture command also resolves verified corrections. */ });
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
      ...result.pending.filter((choice) => choice.kind === "origin"),
    ]) {
      if (!choices.some((item) => item.kind === choice.kind && item.origin === choice.origin)) choices.push(choice);
    }
  }
  const key = (choice: PendingAccess) => `${choice.kind}:${choice.origin}`;
  const fetch = async () => {
    const current = ++generation.current;
    const options = {
      approvedOrigins: choices.filter((item) => item.kind === "origin" && approved.includes(key(item))).map((item) => item.origin),
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
    <label><input type="checkbox" checked={confirmed} disabled={busy || cancelling} onChange={(event) => setConfirmed(event.target.checked)} />{text("仅本次允许后端读取该期刊公开页面", "Allow the backend to read this journal's public pages for this request only")}</label>
    {choices.map((choice) => <label key={key(choice)}><input type="checkbox" disabled={busy || cancelling} checked={approved.includes(key(choice))} onChange={(event) => setApproved((current) => event.target.checked ? [...current, key(choice)] : current.filter((value) => value !== key(choice)))} /><span>{text(`我确认 ${choice.origin} 的域名属于期刊或出版社官方来源，并仅授权本次读取公开页面。`, `I confirm the domain at ${choice.origin} is an official journal or publisher source and authorize reading its public pages for this request only.`)}</span></label>)}
    <button className="secondary-button" type="button" disabled={busy || cancelling || !confirmed || choices.some((choice) => !approved.includes(key(choice)))} onClick={() => void fetch()}>{busy ? text("正在读取官方页面…", "Reading official pages…") : text("获取官方投稿要求", "Capture official requirements")}</button>
    {choices.length > 0 ? <button className="text-button" type="button" disabled={busy || cancelling} onClick={() => void cancel()}>{text("取消额外访问，改用粘贴原文", "Cancel additional access and paste official text")}</button> : null}
    <small>{text("仅读取期刊公开资料，结果保存在本机。", "Only public journal information is read. Results are stored locally.")}</small>
    <details className="official-access-scope"><summary>{text("读取说明", "About this access")}</summary>
      {correction && correction.url !== homepageUrl ? <p>{text("本次使用已核验的官方地址：", "This request uses the verified official URL:")}<code>{correction.url}</code><small>{text("核验来源：", "Verification source:")}<code>{correction.authorityUrl}</code></small></p> : null}
      <p>{text("最多读取 4 个指南候选页面，另含受限的重定向和动态正文请求；每次响应不超过 2 MB。", "Reads at most four candidate pages, plus bounded redirects and dynamic-text requests; each response is limited to 2 MB.")}</p>
      <p>{text("遇到虚拟 DNS 地址时，自动通过 Cloudflare 加密 DNS 查询公开域名；不发送论文、作者身份或网址路径。", "If DNS returns a virtual address, Cloudflare encrypted DNS resolves the public domain automatically; no manuscript, author identity, or URL path is sent.")}</p>
    </details>
    {cancelled ? <p role="status">{text("已取消额外访问，请在下方粘贴官方原文。", "Additional access cancelled. Paste official text below.")}</p> : null}
    {error ? <p role="alert">{localizeBackendText(locale, error)}</p> : null}
    {result ? <div className="official-access-result" role="status">
      <strong>{result.partial ? text("获取未完成 · 仍需确认或补充官方原文", "Capture incomplete · confirmation or official text is still needed") : text("本次页面读取完成，请核对要求快照", "Page reading complete; review the requirement snapshot")}</strong>
      <details><summary>{text("查看访问地址与结果", "View accessed URLs and results")}</summary><ul>{visibleEvents.map((event, index) => <li key={`${index}:${event.url}`}><span>{localizeBackendText(locale, ["OFFICIAL_DNS_FAILED", "OFFICIAL_ENCRYPTED_DNS_FAILED"].includes(event.code) ? "OFFICIAL_SOURCE_UNAVAILABLE" : event.code)}{event.detail ? ` (${event.detail})` : ""}</span><code>{event.url}</code></li>)}</ul></details>
    </div> : null}
  </div>;
}
