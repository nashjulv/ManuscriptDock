import { useEffect, useRef, useState } from "react";
import { api } from "./shared/ipc";
import { localizeBackendText, useI18n } from "./i18n";
import { FileIcon } from "./FileIcons";
import { ManuscriptInput } from "./ManuscriptInput";
import { AiHistory, AiWorkbench } from "./AiAssistant";
import type { AiTask, AiRun } from "./shared/contracts";
import type { AppError, MaterialCheck, MaterialTask, Project } from "./shared/contracts";

export function MaterialChecklist({ project, setProject, refreshKey, onGenerated, run }: {
  project: Project; refreshKey: number; onGenerated: () => void;
  setProject: (project: Project) => void;
  run: <T>(action: () => Promise<T>) => Promise<T | null>;
}) {
  const { locale, text, localize } = useI18n();
  const [items, setItems] = useState<MaterialTask[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [generating, setGenerating] = useState<string | null>(null);
  const generationLock = useRef(false);
  const [ai, setAi] = useState<{ task: AiTask; materialId?: string; existingRun?: AiRun } | null>(null);
  const [aiRefresh, setAiRefresh] = useState(0);
  const openAi = (task: AiTask, materialId?: string, existingRun?: AiRun) => {
    if (generationLock.current) return;
    generationLock.current = true; setGenerating("ai"); setAi({ task, materialId, existingRun });
  };
  const closeAi = () => { setAi(null); generationLock.current = false; setGenerating(null); setAiRefresh(value => value + 1); };
  const [checks, setChecks] = useState<Record<string, MaterialCheck>>({});
  const [approvals, setApprovals] = useState<Record<string, boolean[]>>({});
  useEffect(() => {
    let active = true;
    let request = 0;
    const refresh = () => {
      setChecks({}); setApprovals({});
      const current = ++request;
      void api.materialTasks(project.id).then(value => { if (active && current === request) { setItems(value); setError(null); } }).catch((cause: AppError) => { if (active && current === request) { setItems([]); setError(cause.code ?? "UNKNOWN"); } });
    };
    refresh();
    window.addEventListener("focus", refresh);
    return () => { active = false; window.removeEventListener("focus", refresh); };
  }, [project.id, project.revision, project.target?.journalId, refreshKey]);
  const generate = async (id?: string, template = false) => {
    if (generationLock.current) return;
    generationLock.current = true;
    setGenerating(template ? `template:${id ?? "all"}` : id ?? "all");
    try {
      await run(async () => {
        try { return template ? id ? await api.generateMaterialTemplate(project, id) : await api.generateMaterialTemplates(project) : await api.generateMaterials(project, id ? [id] : []); }
        finally {
          try { setItems(await api.materialTasks(project.id)); }
          catch { setItems([]); setError("WORKSPACE_IO_FAILED"); }
          onGenerated();
        }
      });
    } finally { generationLock.current = false; setGenerating(null); }
  };
  const review = async (item: MaterialTask, confirm = false) => {
    if (generationLock.current) return;
    generationLock.current = true; setGenerating(`review:${item.id}`); setError(null);
    try {
      await run(async () => {
        try {
          if (confirm) {
            const checked = checks[item.id];
            if (!checked || checked.issues.length || !checked.authorChecks.every((_, index) => approvals[item.id]?.[index])) return;
            await api.confirmMaterial(project.id, checked);
            setChecks(value => { const next = { ...value }; delete next[item.id]; return next; });
            setItems(await api.materialTasks(project.id));
            onGenerated();
          } else {
            const checked = await api.checkMaterial(project.id, item.id);
            setChecks(value => ({ ...value, [item.id]: checked }));
            setApprovals(value => ({ ...value, [item.id]: [] }));
          }
        } catch (cause) {
          setChecks(value => { const next = { ...value }; delete next[item.id]; return next; });
          setError((cause as AppError).code ?? "UNKNOWN");
          setItems(await api.materialTasks(project.id));
        }
      });
    } finally { generationLock.current = false; setGenerating(null); }
  };
  const statusLabel = (item: MaterialTask) => item.status === "confirmed" ? text("已确认", "Confirmed") : item.status === "modified" ? text("已修改 · 待检查", "Modified · check needed") : item.status === "draft_present" ? (item.existingFilePath && item.existingFilePath !== item.relativePath ? text("文件已存在 · 待核对", "File present · review needed") : text("草稿已存在 · 待核对", "Draft present · review needed")) : item.templatePresent ? text("模板已存在 · 需补充", "Template present · input needed") : item.status === "manual_required" ? text("需手动补充", "Manual input required") : item.status === "ready_to_generate" ? text("可一键生成", "Ready to generate") : text("已提供", "Provided");
  return <section className="material-checklist" aria-label={text("投稿材料清单", "Submission material checklist")}>
    <header><div><h3>{text("投稿材料清单", "Submission material checklist")}</h3><p>{text("逐项补齐目标期刊所需材料", "Complete the materials for your target journal")}</p></div>
      <div className="material-bulk-actions"><button disabled={!!generating || !items.some(item => item.reviewPath)} onClick={() => openAi("review_consistency")}>{text("AI 一致性检查", "AI consistency review")}</button><button disabled={!!generating || !items.some(item => item.canGenerateTemplate)} onClick={() => void generate(undefined, true)}>{generating === "template:all" ? text("正在生成…", "Generating…") : text("一键生成模板", "Generate all templates")}</button>
      <button className="primary" disabled={!!generating || !items.some(item => item.canGenerate)} onClick={() => void generate()}>{generating === "all" ? text("正在生成…", "Generating…") : text("全部一键生成", "Generate all available")}</button></div></header>
    {error ? <p role="alert">{localizeBackendText(locale, error)}</p> : null}
    <ul>{items.map(item => <li key={item.id} data-status={item.status}>
      <FileIcon name={item.relativePath}/><div className="material-task-copy"><div><strong>{localize(item.label)}</strong><span className="material-required">{item.required ? text("必需", "Required") : text("可选", "Optional")}</span></div>
      <details><summary>{text("要求与文件位置", "Requirements and file location")}</summary><p>{localize(item.description)}</p>{item.id === "manuscript" && <p>{text("打包后的文件名（不是待检索的源文件）：", "File name in the built package (not the source file to locate): ")}</p>}<code>{item.reviewPath ?? item.relativePath}</code></details>
      {item.id === "manuscript" && <ManuscriptInput project={project} item={item} disabled={!!generating} onProvided={updated => { setProject(updated); onGenerated(); }}/ >}
      {checks[item.id] ? <div className="material-review" aria-label={text("材料检查结果", "Material check result")}>
        {checks[item.id].issues.length ? <><strong>{text("请处理后重新检查", "Resolve these issues and check again")}</strong><ul>{checks[item.id].issues.map((issue, index) => <li key={index}>{localize(issue)}</li>)}</ul></> : <>
          <strong>{text("本地检查通过 · 请作者确认", "Local checks passed · author confirmation required")}</strong>
          <p>{text("自动检查不验证研究事实真实性。确认将保存此版本快照并用于投稿包。", "Automated checks do not verify research facts. Confirmation saves this version as a snapshot for the submission package.")}</p>
          {checks[item.id].authorChecks.map((requirement, index) => <label key={index}><input type="checkbox" checked={!!approvals[item.id]?.[index]} onChange={event => { const checked = event.target.checked; setApprovals(value => { const next = [...(value[item.id] ?? [])]; next[index] = checked; return { ...value, [item.id]: next }; }); }}/>{localize(requirement)}</label>)}
          <button disabled={!!generating || !checks[item.id].authorChecks.every((_, index) => approvals[item.id]?.[index])} onClick={() => void review(item, true)}>{text("确认完成并保存快照", "Confirm completion and save snapshot")}</button>
        </>}
      </div> : null}</div>
      <div className="material-task-actions">
      {(item.id === "cover_letter" || item.id === "highlights") && !item.existingFilePath && item.status !== "confirmed" ? <button disabled={!!generating || !project.facts.title?.trim() || !project.facts.abstractText?.trim()} onClick={() => openAi(item.id === "cover_letter" ? "draft_cover_letter" : "draft_highlights", item.id)}>{text("AI 起草", "Draft with AI")}</button> : null}
      {item.reviewPath ? <button disabled={!!generating} onClick={() => openAi("review_material", item.id)}>{text("AI 深度检查", "AI semantic review")}</button> : null}
      {item.id !== "manuscript" && (item.reviewPath || item.existingFilePath || item.templatePresent || item.status === "draft_present") ? <button className={`material-status ${item.status}`} title={text("打开文件核对", "Open file for review")} onClick={() => void run(() => api.openWorkspaceEntry(project.id, item.reviewPath ?? item.existingFilePath ?? (item.templatePresent && item.status !== "draft_present" ? item.relativePath.replace("-DRAFT.docx", "-TEMPLATE.docx") : item.relativePath)))}>{statusLabel(item)}<span aria-hidden="true"> ↗</span></button> : <span className={`material-status ${item.status}`}>{statusLabel(item)}</span>}
      {item.reviewPath && item.status !== "confirmed" ? <button className="generate-item" disabled={!!generating} onClick={() => void review(item)}>{generating === `review:${item.id}` ? text("正在检查…", "Checking…") : text("检查并确认完成", "Check and confirm completion")}</button> : null}
      {item.canGenerate ? <button className="generate-item" disabled={!!generating} aria-label={text(`一键生成：${localize(item.label)}`, `Generate: ${localize(item.label)}`)} onClick={() => void generate(item.id)}>{generating === item.id ? text("生成中…", "Generating…") : text("一键生成", "Generate")}</button> : null}
      {item.canGenerateTemplate ? <button className="generate-item" disabled={!!generating} aria-label={text(`一键生成模板：${localize(item.label)}`, `Generate template: ${localize(item.label)}`)} onClick={() => void generate(item.id, true)}>{generating === `template:${item.id}` ? text("生成中…", "Generating…") : text("一键生成模板", "Generate template")}</button> : null}
      </div>
    </li>)}</ul>
    <p className="generation-note">{text("只生成已有能力支持、且信息齐备的缺失材料。编辑后返回应用刷新，再检查并确认。声明与研究事实须由作者确认；现有文件不会被覆盖。", "Generate only missing materials with supported generation and sufficient information. After editing, return to refresh, check, and confirm. Authors must confirm declarations and research facts. Existing files are preserved.")}</p>
    <AiHistory projectId={project.id} refreshKey={refreshKey + aiRefresh} onSelect={entry => openAi(entry.task, entry.materialId ?? undefined, entry)}/>
    {ai && <AiWorkbench projectId={project.id} {...ai} onClose={closeAi} onChanged={() => { onGenerated(); setAiRefresh(value => value + 1); void api.materialTasks(project.id).then(setItems).catch(() => setError("WORKSPACE_IO_FAILED")); }}/ >}
  </section>;
}
