import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { localizeBackendText, useI18n } from "./i18n";
import { api } from "./shared/ipc";
import { BrowserIcon, FileIcon } from "./FileIcons";
import type { AppError, Project, SelectedInput } from "./shared/contracts";

type Listing = Awaited<ReturnType<typeof api.workspace>>;
const parentOf = (path: string) => path.split("/").slice(0, -1).join("/");
const nameOf = (path: string) => path.split("/").pop() ?? path;

export function PackageWorkspace({ project, setProject, run, refreshKey, onLocationChanged }: {
  project: Project; setProject: (project: Project) => void;
  run: <T>(action: () => Promise<T>) => Promise<T | null>;
  refreshKey: string; onLocationChanged?: () => void;
}) {
  const { locale, text } = useI18n();
  const [listing, setListing] = useState<Listing | null>(null);
  const [directory, setDirectory] = useState("");
  const [view, setView] = useState<"icons" | "list">("list");
  const [selected, setSelected] = useState<string | null>(null);
  const [materialKind, setMaterialKind] = useState("supplementary");
  const [error, setError] = useState<AppError | null>(null);
  const [loading, setLoading] = useState(false);
  const [expanded, setExpanded] = useState(new Set(["", "submission"]));
  const [sort, setSort] = useState<"name" | "modified" | "size">("name");
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  const generation = useRef(0);
  const pointerDrag = useRef<{ path: string; x: number; y: number } | null>(null);
  const refresh = useCallback(async () => {
    const current = ++generation.current;
    setLoading(true);
    try {
      const next = await api.workspace(project.id);
      if (generation.current === current) { setListing(next); setError(null); }
    } catch (cause) {
      if (generation.current === current) setError((cause && typeof cause === "object" && "code" in cause ? cause : { code: "WORKSPACE_IO_FAILED" }) as AppError);
    } finally { if (generation.current === current) setLoading(false); }
  }, [project.id]);
  useEffect(() => {
    void refresh();
    const focus = () => void refresh();
    window.addEventListener("focus", focus);
    return () => { generation.current++; window.removeEventListener("focus", focus); };
  }, [refresh, refreshKey]);
  useEffect(() => {
    if (directory && listing && !listing.entries.some(e => e.directory && e.relativePath === directory)) setDirectory("");
    if (selected && listing && !listing.entries.some(e => e.relativePath === selected)) setSelected(null);
  }, [listing, directory, selected]);
  useEffect(() => {
    let disposed = false;
    const disposers: Array<() => void> = [];
    const subscribe = async () => {
      const off = await listen<{ items: SelectedInput[]; x: number; y: number }>("manuscriptdock://files-dropped", event => {
        if (disposed) return;
        const point = document.elementFromPoint(event.payload.x / window.devicePixelRatio, event.payload.y / window.devicePixelRatio);
        if (!point?.closest(".package-workspace")) return;
        const target = point.closest<HTMLElement>("[data-directory]")?.dataset.directory ?? directory;
        void run(async () => {
          try { for (const item of event.payload.items) await api.importWorkspace(project.id, item.token, target); }
          finally { await refresh(); }
          return true;
        });
      });
      if (disposed) off(); else disposers.push(off);
      const offError = await listen<AppError>("manuscriptdock://drop-error", event => { if (!disposed) void run(async () => { throw event.payload; }); });
      if (disposed) offError(); else disposers.push(offError);
    };
    void subscribe().catch(() => undefined);
    return () => { disposed = true; disposers.forEach(off => off()); };
  }, [project.id, directory, refresh, run]);
  const enter = (path: string) => {
    setDirectory(path); setSelected(null);
    setExpanded(current => {
      const next = new Set(current); next.add(path);
      let parent = parentOf(path);
      while (parent) { next.add(parent); parent = parentOf(parent); }
      return next;
    });
  };
  const move = (source: string, target: string) => void run(async () => { await api.moveWorkspace(project.id, source, target); await refresh(); return true; });
  const drop = (event: React.DragEvent, target: string) => {
    event.preventDefault(); event.stopPropagation();
    const source = event.dataTransfer.getData("application/x-manuscriptdock-file");
    if (source) move(source, target);
  };
  const importFile = () => void run(async () => {
    const selection = await api.choose("material");
    if (selection.status === "cancelled") return false;
    try { for (const item of selection.items) await api.importWorkspace(project.id, item.token, directory); }
    finally { await refresh(); }
    return true;
  });
  const allEntries = listing?.entries ?? [];
  const entries = allEntries.filter(e => parentOf(e.relativePath) === directory).sort((a, b) => Number(b.directory) - Number(a.directory) || (sort === "modified" ? (b.modifiedAtUnixMs ?? 0) - (a.modifiedAtUnixMs ?? 0) : sort === "size" ? b.sizeBytes - a.sizeBytes : nameOf(a.relativePath).localeCompare(nameOf(b.relativePath), locale, { numeric: true })));
  const selectedEntry = allEntries.find(e => e.relativePath === selected);
  const size = (bytes: number) => new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(bytes / (bytes >= 1048576 ? 1048576 : 1024)) + (bytes >= 1048576 ? " MB" : " KB");
  const modified = (time?: number) => time ? new Intl.DateTimeFormat(locale, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" }).format(time) : "—";
  const folderName = (path: string) => ({ submission: text("投稿文件", "Submission files"), "author-tools": text("作者草稿", "Author drafts"), figures: text("图表", "Figures"), supplementary: text("补充材料", "Supplementary"), drafts: text("历史生成版本", "Generated versions"), records: text("核验记录", "Records") })[nameOf(path)] ?? nameOf(path);
  const tree = (parent: string, depth = 0): React.ReactNode => allEntries.filter(e => e.directory && parentOf(e.relativePath) === parent).map(entry => {
    const children = allEntries.some(e => e.directory && parentOf(e.relativePath) === entry.relativePath);
    const isExpanded = expanded.has(entry.relativePath);
    return <div key={entry.relativePath}>
      <div className="folder-tree-row" data-current={directory === entry.relativePath} data-directory={entry.relativePath} data-drop-target={dropTarget === entry.relativePath} style={{ paddingInlineStart: 8 + depth * 14 }} onDragOver={e => e.preventDefault()} onDrop={e => drop(e, entry.relativePath)}>
        <button className="folder-disclosure" disabled={!children} aria-label={text(`展开或折叠 ${folderName(entry.relativePath)}`, `Expand or collapse ${folderName(entry.relativePath)}`)} aria-expanded={children ? isExpanded : undefined} onClick={() => setExpanded(current => { const next = new Set(current); if (isExpanded) next.delete(entry.relativePath); else next.add(entry.relativePath); return next; })}><BrowserIcon name="chevron" /></button>
        <button className="folder-tree-item" aria-current={directory === entry.relativePath ? "location" : undefined} title={entry.relativePath} onClick={() => enter(entry.relativePath)}><FileIcon name="" folder /><span>{folderName(entry.relativePath)}</span></button>
      </div>
      {isExpanded ? tree(entry.relativePath, depth + 1) : null}
    </div>;
  });
  const openSelected = () => { if (!selectedEntry) return; if (selectedEntry.directory) enter(selectedEntry.relativePath); else void run(() => api.openWorkspaceEntry(project.id, selectedEntry.relativePath)); };
  return <section className="package-workspace native-browser" aria-label={text("投稿包本地目录", "Local package folder")}>
    <header className="workspace-toolbar">
      <button className="icon-button" aria-label={text("上一级", "Up")} title={text("上一级", "Up")} disabled={!directory} onClick={() => enter(parentOf(directory))}><BrowserIcon name="back" /></button>
      <div className="browser-title"><strong>{directory ? folderName(directory) : text("投稿包本地目录", "Local package folder")}</strong><span>{text("保存在稿件旁", "Saved beside your manuscript")}</span></div>
      <div className="view-switch" role="group" aria-label={text("显示方式", "View mode")}>
        <button className="icon-button" aria-label={text("图标", "Icons")} title={text("图标", "Icons")} aria-pressed={view === "icons"} onClick={() => setView("icons")}><BrowserIcon name="grid" /></button>
        <button className="icon-button" aria-label={text("列表", "List")} title={text("列表", "List")} aria-pressed={view === "list"} onClick={() => setView("list")}><BrowserIcon name="list" /></button>
      </div>
      <button className="icon-button" aria-label={text("刷新", "Refresh")} title={text("刷新", "Refresh")} onClick={() => { void refresh(); onLocationChanged?.(); }}><BrowserIcon name="refresh" /></button>
      <button className="icon-button" aria-label={text("在文件管理器中打开", "Open in file manager")} title={text("在文件管理器中打开", "Open in file manager")} onClick={() => void run(() => api.openWorkspace(project.id))}><BrowserIcon name="external" /></button>
      <button className="browser-add" onClick={importFile}><BrowserIcon name="add" />{text("添加文件", "Add files")}</button>
    </header>
    {error ? <div className="workspace-error" role="alert"><span>{localizeBackendText(locale, error.code)}</span><button onClick={() => void run(async () => { const chosen = await api.choosePackageLocation(project.id); if (chosen) { await refresh(); onLocationChanged?.(); } return chosen; })}>{text("选择稿件所在文件夹", "Choose manuscript folder")}</button></div> : null}
    <div className="workspace-browser">
      <nav aria-label={text("目录树", "Folder tree")}>
        <div className="tree-heading">{text("本地文件", "LOCAL FILES")}</div>
        <button className="tree-root" data-directory="" aria-current={!directory ? "location" : undefined} onClick={() => enter("")} onDragOver={e => e.preventDefault()} onDrop={e => drop(e, "")}><FileIcon name="" folder /><span>{text("投稿包", "Package")}</span></button>
        {tree("")}
      </nav>
      <div className="workspace-content" data-directory={directory} onDragOver={e => e.preventDefault()} onDrop={e => drop(e, directory)}>
        {view === "list" ? <div className="file-list-heading"><button onClick={() => setSort("name")}>{text("名称", "Name")}</button><button onClick={() => setSort("modified")}>{text("修改日期", "Date modified")}</button><button onClick={() => setSort("size")}>{text("大小", "Size")}</button></div> : null}
        <div className={`workspace-files ${view}`}>
          {entries.map(entry => <button key={entry.relativePath} className="workspace-entry" draggable={false} data-directory={entry.directory ? entry.relativePath : undefined} data-drop-target={dropTarget === entry.relativePath} aria-pressed={selected === entry.relativePath}
            onDragStart={event => event.preventDefault()}
            onPointerDown={event => { if (entry.directory || event.button !== 0) return; pointerDrag.current = { path: entry.relativePath, x: event.clientX, y: event.clientY }; event.currentTarget.setPointerCapture(event.pointerId); }}
            onPointerMove={event => { const drag = pointerDrag.current; if (!drag || Math.hypot(event.clientX - drag.x, event.clientY - drag.y) < 8) return; const point = document.elementFromPoint(event.clientX, event.clientY); setDropTarget(point?.closest(".package-workspace") ? point.closest<HTMLElement>("[data-directory]")?.dataset.directory ?? null : null); }}
            onPointerCancel={() => { pointerDrag.current = null; setDropTarget(null); }}
            onPointerUp={event => { const drag = pointerDrag.current; pointerDrag.current = null; setDropTarget(null); if (!drag || Math.hypot(event.clientX - drag.x, event.clientY - drag.y) < 8) return; const point = document.elementFromPoint(event.clientX, event.clientY); const target = point?.closest(".package-workspace") ? point.closest<HTMLElement>("[data-directory]")?.dataset.directory : undefined; if (target !== undefined) move(drag.path, target); }}
            onDragOver={event => { if (entry.directory) event.preventDefault(); }} onDrop={event => { if (entry.directory) drop(event, entry.relativePath); }}
            onClick={() => setSelected(entry.relativePath)} onDoubleClick={() => entry.directory ? enter(entry.relativePath) : void run(() => api.openWorkspaceEntry(project.id, entry.relativePath))} onKeyDown={event => { if (event.key === "Enter") { event.preventDefault(); if (entry.directory) enter(entry.relativePath); else void run(() => api.openWorkspaceEntry(project.id, entry.relativePath)); } }}>
            <span className="file-name"><FileIcon name={entry.relativePath} folder={entry.directory}/><span>{entry.directory ? folderName(entry.relativePath) : nameOf(entry.relativePath)}</span></span>
            <span className="file-modified">{modified(entry.modifiedAtUnixMs)}</span><small className="file-size">{entry.directory ? "—" : size(entry.sizeBytes)}</small>
          </button>)}
        </div>
        {!entries.length ? <div className="workspace-empty"><FileIcon name="" folder/><strong>{loading ? text("正在读取文件…", "Reading files…") : text("此文件夹为空", "This folder is empty")}</strong><span>{text("将本地文件拖到这里，或点击“添加文件”", "Drop local files here, or choose Add files")}</span></div> : null}
      </div>
    </div>
    <footer className="workspace-status"><span>{text(`${entries.length.toLocaleString(locale)} 项`, `${entries.length.toLocaleString(locale)} ${entries.length === 1 ? "item" : "items"}`)}{selected ? text(" · 已选择 1 项", " · 1 selected") : ""}</span><span>{loading ? text("正在刷新…", "Refreshing…") : text("返回应用时自动刷新", "Refreshes when you return")}</span></footer>
    <div className="workspace-location" title={listing?.rootPath}><BrowserIcon name="chevron"/><code>{listing?.rootPath ?? text("等待选择保存位置", "Waiting for a save location")}</code>{directory ? <span>/ {directory}</span> : null}</div>
    {selectedEntry ? <div className="workspace-selection"><span className="selected-file-name">{nameOf(selectedEntry.relativePath)}</span><button onClick={openSelected}>{text("打开", "Open")}</button>{!selectedEntry.directory ? <>
      <label><span className="sr-only">{text("目录文件用途", "Folder file purpose")}</span><select value={materialKind} onChange={event => setMaterialKind(event.target.value)}>
        <option value="supplementary">{text("补充材料", "Supplementary material")}</option><option value="figure">{text("图或图表", "Figure or artwork")}</option><option value="reporting_checklist">{text("报告清单", "Reporting checklist")}</option><option value="editable_manuscript">{text("可编辑主稿 DOCX", "Editable manuscript DOCX")}</option><option value="anonymized_manuscript">{text("匿名主稿 DOCX", "Anonymized manuscript DOCX")}</option>
      </select></label><button className="include-file" aria-label={text("按所选附件用途加入投稿包", "Include using selected attachment purpose")} onClick={() => void run(async () => { const next = await api.useWorkspaceMaterial(project, selectedEntry.relativePath, materialKind); setProject(next); return true; })}>{text("加入投稿包", "Include in package")}</button>
    </> : null}</div> : null}
  </section>;
}
