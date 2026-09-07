import { GuidedButton } from "./ActionGuidance";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { localizeBackendText, useI18n } from "./i18n";

interface Link { id: string; pdfWorkspaceId: string; sourceWorkspaceId: string; confirmedUnixMs: number }
interface Selection { status: string; selectionId?: string; manuscript?: { name: string; kind: string }; message?: string }
export function SourceRecovery({ workspaceId, isPdf, onOpenWorkspace }: { workspaceId: string; isPdf: boolean; onOpenWorkspace: (id: string) => void }) {
  const { locale, text } = useI18n();
  const [links, setLinks] = useState<Link[]>([]);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => { let current = true; void invoke<Link[]>("list_linked_sources", { workspaceId }).then(value => { if (current && Array.isArray(value)) setLinks(value); }).catch(reason => { if (current) setError(localizeBackendText(locale, String(reason))); }); return () => { current = false; }; }, [workspaceId, locale]);
  async function choose() {
    setBusy(true); setError(""); setConfirmed(false); setSelection(null);
    try {
      const value = await invoke<Selection>("select_manuscript");
      if (value.status === "selected" && value.manuscript?.kind !== "pdf") setSelection(value);
      else if (value.status !== "cancelled") setError(text("请选择 DOCX 或 TEX 源稿。", "Choose a DOCX or TEX source."));
    } catch (reason) { setError(localizeBackendText(locale, String(reason))); } finally { setBusy(false); }
  }
  async function create() {
    setBusy(true); setError("");
    try { const workspace = await invoke<{ id: string }>("create_linked_source", { workspaceId, selectionId: selection?.selectionId, authorConfirmed: confirmed }); onOpenWorkspace(workspace.id); }
    catch (reason) { setError(localizeBackendText(locale, String(reason))); } finally { setBusy(false); }
  }
  if (!isPdf && links.length === 0) return error ? <p role="alert">{error}</p> : null;
  return <section className="source-recovery"><h3>{text("源稿与关联档案", "Source and linked workspaces")}</h3>
    {isPdf ? <><p>{text("PDF 可检查，不能直接改写。选择 DOCX/TEX 源稿后会建立关联的新工作区，从 v1 开始；原 PDF 和记录保留。", "PDFs can be checked but cannot be rewritten here. A DOCX/TEX source creates a linked workspace starting at v1; the PDF and its records remain intact.")}</p><button type="button" className="secondary-button" disabled={busy} onClick={() => void choose()}>{text("选择源稿继续准备", "Choose source to continue")}</button></> : null}
    {selection ? <div role="group" aria-label={text("确认源稿关联", "Confirm source association")}><strong>{selection.manuscript?.name}</strong><label><input type="checkbox" checked={confirmed} onChange={event => setConfirmed(event.target.checked)} />{text("我确认这份源稿与当前 PDF 属于同一论文。检查、声明与附件需要重新核验。", "I confirm this source and the PDF represent the same manuscript. Checks, declarations, and attachments need review.")}</label><GuidedButton type="button" disabled={busy} prerequisite={!confirmed && { message: text("请核对源稿，并确认它与当前 PDF 属于同一论文。", "Review the source and confirm it represents the same manuscript as this PDF."), scope: ".source-recovery", target: 'input[type="checkbox"]' }} onClick={() => void create()}>{text("建立关联并继续", "Link and continue")}</GuidedButton><button type="button" onClick={() => setSelection(null)}>{text("取消", "Cancel")}</button></div> : null}
    {links.map(link => <button key={link.id} type="button" onClick={() => onOpenWorkspace(link.pdfWorkspaceId === workspaceId ? link.sourceWorkspaceId : link.pdfWorkspaceId)}>{link.pdfWorkspaceId === workspaceId ? text("继续关联源稿准备", "Continue with linked source") : text("返回原 PDF 档案", "Return to original PDF")}</button>)}
    {error ? <p role="alert">{error}</p> : null}
  </section>;
}
