import { useEffect, useRef, useState } from "react";
import { localizeBackendText, useI18n } from "./i18n";
import { api } from "./shared/ipc";
import type { AppError, MaterialTask, Project } from "./shared/contracts";

type Candidate = { name: string; token: string } | { name: string; relativePath: string };

export function ManuscriptInput({ project, item, disabled, onProvided }: {
  project: Project; item: MaterialTask; disabled: boolean;
  onProvided: (project: Project) => void;
}) {
  const { locale, text } = useI18n();
  const [paths, setPaths] = useState<string[] | null>(null);
  const [candidate, setCandidate] = useState<Candidate | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const lock = useRef(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => { setPaths(null); setCandidate(null); setConfirmed(false); setError(null); }, [project.id, project.revision, item.manuscriptKind]);
  const perform = async (action: () => Promise<void>) => {
    if (lock.current || disabled) return;
    lock.current = true; setBusy(true); setError(null);
    try { await action(); }
    catch (cause) { setError((cause as AppError).code ?? "UNKNOWN"); }
    finally { lock.current = false; setBusy(false); }
  };
  const choose = () => perform(async () => {
    const selection = await api.choose("editable_manuscript");
    if (selection.status !== "selected" || !selection.items.length) return;
    const file = selection.items[0];
    setCandidate({ name: file.name, token: file.token }); setConfirmed(false);
  });
  const search = () => perform(async () => {
    const workspace = await api.workspace(project.id);
    setPaths(workspace.entries.filter(entry => !entry.directory && /\.docx$/i.test(entry.relativePath)).map(entry => entry.relativePath));
    setCandidate(null); setConfirmed(false);
  });
  const provide = () => perform(async () => {
    if (!candidate || !confirmed || !item.manuscriptKind) return;
    const updated = "token" in candidate
      ? await api.addMaterial(project, candidate.token, item.manuscriptKind)
      : await api.useWorkspaceMaterial(project, candidate.relativePath, item.manuscriptKind);
    setCandidate(null); setConfirmed(false); setPaths(null);
    onProvided(updated);
  });
  return <div className="manuscript-input">
    {item.providedFileName && <p>{text("已关联主稿：", "Linked manuscript: ")}<strong>{item.providedFileName}</strong></p>}
    <div className="manuscript-input-actions">
      <button disabled={disabled || busy} onClick={() => void choose()}>{text("选择 DOCX", "Choose DOCX")}</button>
      <button disabled={disabled || busy} onClick={() => void search()}>{text("检索目录内 DOCX", "Find DOCX in folder")}</button>
    </div>
    {paths !== null && (paths.length ? <label>{text("投稿目录中的 DOCX", "DOCX files in the submission folder")}
      <select disabled={disabled || busy} value={candidate && "relativePath" in candidate ? candidate.relativePath : ""} onChange={event => { setCandidate(event.target.value ? { name: event.target.value, relativePath: event.target.value } : null); setConfirmed(false); }}>
        <option value="">{text("请选择文件", "Select a file")}</option>
        {paths.map(path => <option key={path} value={path}>{path}</option>)}
      </select>
    </label> : <p role="status">{text("目录内未找到 DOCX，可用“选择 DOCX”从其他位置添加。", "No DOCX found in this folder. Use Choose DOCX to add one from another location.")}</p>)}
    {candidate && <div className="manuscript-candidate">
      <p>{text("待关联：", "Selected: ")}{candidate.name}</p>
      <label><input type="checkbox" disabled={disabled || busy} checked={confirmed} onChange={event => setConfirmed(event.target.checked)}/>{item.manuscriptKind === "anonymized_manuscript"
        ? text("我已核对这是作者提供的匿名主稿，并已移除作者身份信息。", "I verified this author-supplied anonymized manuscript and removed author identity information.")
        : text("我已核对这是本次投稿的可编辑主稿。", "I verified this is the editable manuscript for this submission.")}</label>
      <button className="primary" disabled={disabled || busy || !confirmed || !item.manuscriptKind} onClick={() => void provide()}>{busy ? text("正在关联…", "Linking…") : text("用作主稿", "Use as manuscript")}</button>
    </div>}
    {error && <p role="alert">{localizeBackendText(locale, error)}</p>}
  </div>;
}
