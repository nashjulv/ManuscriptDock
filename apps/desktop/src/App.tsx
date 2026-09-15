import { useCallback, useEffect, useRef, useState } from "react";
import { I18nProvider, localizeBackendText, useI18n } from "./i18n";
import type { Locale } from "./i18n";
import { TextSizeProvider, TextSizeSettings } from "./TextSizeSettings";
import { api } from "./shared/ipc";
import type {
  AppError,
  AuthorConstraints,
  CompiledPackage,
  DocumentFacts,
  ExportReceipt,
  JournalRecord,
  JobRecord,
  PreparationView,
  Project,
  RecommendationResult,
  SelectedInput,
  TaskKind,
} from "./shared/contracts";
import { PRODUCT_VERSION } from "./version";
import { PackageWorkspace } from "./PackageWorkspace";
import { MaterialChecklist } from "./MaterialChecklist";
import productLogo from "./assets/manuscriptdock-logo.svg";
import { factLabels, reconcileFactDraft, type FactConflict } from "./factDraft";
import { JournalTargetMap } from "./JournalTargetMap";
import { AiSettingsDialog } from "./AiAssistant";

type Screen = "home" | "matching" | "preparation";
const DEFAULT_CONSTRAINTS: AuthorConstraints = {
  requiredLanguage: null,
  articleType: "research_article",
  topicKeywords: [],
  requiredIndexing: [],
  maximumApc: null,
  currency: "USD",
  requireOpenAccess: false,
  preferFastFirstDecision: true,
};

function constraintsForProject(
  project: Project | null,
  _locale: Locale,
): AuthorConstraints {
  return {
    ...DEFAULT_CONSTRAINTS,
    requiredLanguage: null,
    articleType:
      project?.facts.articleType || DEFAULT_CONSTRAINTS.articleType,
  };
}

function keywordsForProject(project: Project | null): string {
  return project?.facts.keywords.join(", ") ?? "";
}

function AppContent() {
  const [showAiSettings, setShowAiSettings] = useState(false);
  const { locale, setLocale, text } = useI18n();
  const [screen, setScreen] = useState<Screen>("home");
  const [project, setProject] = useState<Project | null>(null);
  const [recent, setRecent] = useState<Project[]>([]);
  const [removed, setRemoved] = useState<Project[]>([]);
  const [pendingFolder, setPendingFolder] = useState<{
    task: TaskKind;
    items: SelectedInput[];
    folder: { token: string; name: string };
  } | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const [refreshComplete, setRefreshComplete] = useState(false);
  const [activeJob, setActiveJob] = useState<JobRecord | null>(null);
  const busyRef = useRef(false);
  const pageDirty = useRef(false);
  const [showHomePrompt, setShowHomePrompt] = useState(false);
  const reportDirty = useCallback((dirty: boolean) => { pageDirty.current = dirty; }, []);
  const returnHome = () => {
    setShowHomePrompt(false);
    pageDirty.current = false;
    setScreen("home");
    refreshRecent();
  };
  const goHome = () => {
    if (busyRef.current) return;
    if (pageDirty.current) { setShowHomePrompt(true); return; }
    returnHome();
  };
  const refreshRecent = useCallback(() => {
    void api
      .recent()
      .then(setRecent)
      .catch(() => setRecent([]));
    void api
      .removed()
      .then(setRemoved)
      .catch(() => setRemoved([]));
  }, []);
  useEffect(refreshRecent, [refreshRecent]);
  useEffect(() => {
    let disposed = false;
    let unlisten: () => void = () => undefined;
    void api
      .onJob((job) => {
        if (disposed) return;
        setActiveJob(
          job.status === "queued" || job.status === "running" ? job : null,
        );
      })
      .then((dispose) => {
        if (disposed) dispose();
        else unlisten = dispose;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten();
    };
  }, []);
  const run = useCallback(
    async <T,>(action: () => Promise<T>): Promise<T | null> => {
      if (busyRef.current) return null;
      busyRef.current = true;
      setBusy(true);
      setError(null);
      try {
        return await action();
      } catch (value) {
        setError(value as AppError);
        return null;
      } finally {
        busyRef.current = false;
        setBusy(false);
      }
    },
    [],
  );
  const openManuscript = useCallback(
    async (task: TaskKind) => {
      const selected = await run(() => api.choose("manuscript"));
      if (!selected || selected.status === "cancelled") return;
      const opened = await run(() =>
        api.openProject(selected.items[0].token, task),
      );
      if (opened) {
        setProject(opened);
        setScreen(task === "find_journals" ? "matching" : "preparation");
        refreshRecent();
      }
    },
    [refreshRecent, run],
  );
  const finishFolder = useCallback(
    async (
      task: TaskKind,
      selected: SelectedInput,
      items: SelectedInput[],
      folder: { token: string; name: string },
    ) => {
      const opened = await run(() =>
        api.openProject(selected.token, task, folder.token),
      );
      if (!opened) return;
      let current = opened;
      for (const item of items.filter((value) => value.kind === "material")) {
        const updated = await run(() =>
          api.addMaterial(current, item.token, "unclassified"),
        );
        if (!updated) break;
        current = updated;
      }
      setPendingFolder(null);
      setProject(current);
      setScreen(task === "find_journals" ? "matching" : "preparation");
      refreshRecent();
    },
    [refreshRecent, run],
  );
  const openFolder = useCallback(
    async (task: TaskKind) => {
      const selection = await run(() => api.choose("folder"));
      if (!selection || selection.status === "cancelled") return;
      if (!selection.folder) {
        setError(uiError("FOLDER_SELECTION_INVALID"));
        return;
      }
      const manuscripts = selection.items.filter(
        (item) => item.kind === "manuscript",
      );
      if (!manuscripts.length) {
        setError(uiError("FOLDER_MANUSCRIPT_REQUIRED"));
        return;
      }
      if (manuscripts.length === 1) {
        await finishFolder(
          task,
          manuscripts[0],
          selection.items,
          selection.folder,
        );
        return;
      }
      setPendingFolder({
        task,
        items: selection.items,
        folder: selection.folder,
      });
    },
    [finishFolder, run],
  );
  const refreshAll = async () => {
    setRefreshComplete(false);
    await run(async () => {
      const [current, recentProjects, removedProjects] = await Promise.all([
        project && screen !== "home" ? api.project(project.id) : Promise.resolve(null),
        api.recent(), api.removed(),
      ]);
      if (current) setProject(current);
      setRecent(recentProjects); setRemoved(removedProjects);
      setRefreshKey(value => value + 1);
      setRefreshComplete(true);
    });
  };
  const resume = useCallback(async (item: Project) => {
    const current = await run(() => api.project(item.id));
    if (!current) return;
    setProject(current);
    setScreen(current.lastTask === "find_journals" ? "matching" : "preparation");
  }, [run]);
  const removeRecent = useCallback(
    async (item: Project) => {
      const hidden = await run(() => api.hideRecent(item.id));
      if (hidden) refreshRecent();
    },
    [refreshRecent, run],
  );
  const restoreRecent = useCallback(
    async (item: Project) => {
      const restored = await run(() => api.restoreRecent(item.id));
      if (restored) refreshRecent();
    },
    [refreshRecent, run],
  );
  return (
    <div className="app-shell">
      <header className="product-bar">
        <div className="brand-navigation">
        <button
          className="brand"
          onClick={goHome}
          disabled={busy}
          aria-label={text("返回首页", "Back to home")}
        >
          <img src={productLogo} alt="" />
          <span>
            投稿舱 ManuscriptDock <strong>{PRODUCT_VERSION}</strong>
          </span>
        </button>
        <button className="home-navigation" onClick={goHome} disabled={busy} aria-current={screen === "home" ? "page" : undefined}>
          <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true"><path d="m3 10 9-7 9 7M5 9v11h5v-6h4v6h5V9" strokeLinecap="round" strokeLinejoin="round"/></svg>
          {text("首页", "Home")}
        </button>
        </div>
        <div className="bar-actions">
          <span className="local-badge">
            {text("本地优先", "Local first")}
          </span>
          <TextSizeSettings />
          <button disabled={busy} aria-label={text("全局刷新", "Refresh all")} title={text("重新读取当前项目、材料和目录，保留未保存的输入", "Reload the project, materials, and folder while preserving unsaved input")} onClick={() => void refreshAll()}>{text("刷新", "Refresh")}</button>
          {refreshComplete && <span className="sr-only" role="status">{text("已重新读取最新状态", "Latest state reloaded")}</span>}
          <button disabled={busy} onClick={() => setShowAiSettings(true)}>{text("AI 设置", "AI settings")}</button>
          <button
            className="language"
            onClick={() => setLocale(locale === "zh-CN" ? "en" : "zh-CN")}
          >
            {locale === "zh-CN" ? "EN" : "中文"}
          </button>
        </div>
      </header>
      {showAiSettings && <AiSettingsDialog onClose={() => setShowAiSettings(false)}/>}
      {showHomePrompt ? <div className="modal-backdrop"><section className="panel home-prompt" role="alertdialog" aria-modal="true" aria-labelledby="home-prompt-title" aria-describedby="home-prompt-description" onKeyDown={event => {
        if (event.key === "Escape") { setShowHomePrompt(false); event.preventDefault(); }
        if (event.key === "Tab") {
          const buttons = event.currentTarget.querySelectorAll("button");
          if (event.shiftKey && document.activeElement === buttons[0]) { buttons[1].focus(); event.preventDefault(); }
          else if (!event.shiftKey && document.activeElement === buttons[1]) { buttons[0].focus(); event.preventDefault(); }
        }
      }}>
        <h2 id="home-prompt-title">{text("有未保存的输入", "Unsaved input")}</h2>
        <p id="home-prompt-description">{text("返回首页将放弃当前页面未保存的输入。已保存的任务和本地文件会保留。", "Returning home discards unsaved input on this page. Saved tasks and local files are preserved.")}</p>
        <div className="footer-actions"><button autoFocus className="secondary" onClick={() => setShowHomePrompt(false)}>{text("继续编辑", "Continue editing")}</button><button className="primary" onClick={returnHome}>{text("放弃输入并返回首页", "Discard input and go home")}</button></div>
      </section></div> : null}
      {error ? (
        <div className="error-banner" role="alert">
          <span>{localizeBackendText(locale, error.code)}</span>
          <small>
            {text("诊断编号", "Diagnostic ID")}: {error.diagnosticId}
          </small>
          <button onClick={() => setError(null)}>
            {text("关闭", "Dismiss")}
          </button>
        </div>
      ) : null}
      {busy ? (
        <div className="progress" role="status">
          <span>{text("正在本机处理…", "Processing locally…")}</span>
          {activeJob ? (
            <button
              type="button"
              onClick={() => {
                void api
                  .cancelJob(activeJob.projectId, activeJob.id)
                  .catch((value) => setError(value as AppError));
              }}
            >
              {text("取消任务", "Cancel task")}
            </button>
          ) : null}
        </div>
      ) : null}
      <main>
        {screen === "home" ? (
          <Home
            recent={recent}
            removed={removed}
            onTask={(task) => {
              setProject(null);
              setScreen(task === "find_journals" ? "matching" : "preparation");
            }}
            onResume={resume}
            onRemove={removeRecent}
            onRestore={restoreRecent}
          />
        ) : null}
        {screen === "matching" ? (
          <Matching
            onDirtyChange={reportDirty}
            key={project?.id ?? "new-matching-task"}
            project={project}
            setProject={setProject}
            open={() => openManuscript("find_journals")}
            openFolder={() => openFolder("find_journals")}
            run={run}
            onPrepare={() => setScreen("preparation")}
          />
        ) : null}
        {screen === "preparation" ? (
          <Preparation
            refreshKey={refreshKey}
            onDirtyChange={reportDirty}
            project={project}
            setProject={setProject}
            open={() => openManuscript("prepare_package")}
            openFolder={() => openFolder("prepare_package")}
            run={run}
          />
        ) : null}
        {pendingFolder ? (
          <FolderManuscriptPicker
            items={pendingFolder.items.filter(
              (item) => item.kind === "manuscript",
            )}
            onChoose={(item) =>
              finishFolder(
                pendingFolder.task,
                item,
                pendingFolder.items,
                pendingFolder.folder,
              )
            }
            onCancel={() => setPendingFolder(null)}
          />
        ) : null}
      </main>
    </div>
  );
}

function Home({
  recent,
  removed,
  onTask,
  onResume,
  onRemove,
  onRestore,
}: {
  recent: Project[];
  removed: Project[];
  onTask: (task: TaskKind) => void;
  onResume: (project: Project) => void;
  onRemove: (project: Project) => void;
  onRestore: (project: Project) => void;
}) {
  const { text, locale } = useI18n();
  return (
    <section className="home hero">
      <div className="eyebrow">
        {text("两个任务，一份本地稿件", "Two tasks, one local manuscript")}
      </div>
      <h1>
        {text(
          "选好期刊，准备投稿材料",
          "Choose a journal. Prepare the right files.",
        )}
      </h1>
      <p>
        {text(
          "打开本地 PDF 或 DOCX，在设备上匹配期刊、检查规则和整理文件；AI 起草与检查可按需启用。",
          "Open a local PDF or DOCX to match journals, check rules, and organize files on your device. AI drafting and review are optional.",
        )}
      </p>
      <div className="task-grid">
        <button className="task-card" onClick={() => onTask("find_journals")}>
          <span className="task-index">01</span>
          <strong>{text("推荐期刊", "Find journals")}</strong>
          <span>{text("还没选定投哪里", "I have not chosen a journal")}</span>
          <i aria-hidden="true">→</i>
        </button>
        <button className="task-card" onClick={() => onTask("prepare_package")}>
          <span className="task-index">02</span>
          <strong>{text("整理投稿包", "Prepare submission package")}</strong>
          <span>
            {text("已经有目标期刊", "I already have a target journal")}
          </span>
          <i aria-hidden="true">→</i>
        </button>
      </div>
      <div className="privacy-line">
        <span aria-hidden="true">⌁</span>
        {text(
          "基础功能无需配置模型 · AI 辅助按需启用 · 原稿始终保留",
          "Core features need no model setup · AI assistance is optional · Original files are preserved",
        )}
      </div>
      <section className="recent">
        <div className="section-heading">
          <h2>{text("最近打开", "Recently opened")}</h2>
          <span>
            {recent.length
              ? text(
                  `${recent.length} 个本地任务`,
                  `${recent.length} local task${recent.length === 1 ? "" : "s"}`,
                )
              : text("暂无记录", "No recent tasks")}
          </span>
        </div>
        {recent.map((item) => (
          <article className="recent-row" key={item.id}>
            <button className="recent-main" onClick={() => onResume(item)}>
              <span>
                <strong>{item.displayName}</strong>
                <small>
                  {new Intl.DateTimeFormat(locale, {
                    dateStyle: "medium",
                    timeStyle: "short",
                  }).format(item.updatedAtUnixMs)}
                </small>
              </span>
              <span>
                {item.target
                  ? text("材料补充", "Package preparation")
                  : text("期刊匹配", "Journal matching")}
              </span>
              <b>{text("继续", "Continue")}</b>
            </button>
            <button
              className="recent-remove"
              onClick={() => onRemove(item)}
              aria-label={text(
                `从最近任务移除 ${item.displayName}`,
                `Remove ${item.displayName} from recent tasks`,
              )}
            >
              {text("移除", "Remove")}
            </button>
          </article>
        ))}
      </section>
      {removed.length ? (
        <details className="recent removed-projects">
          <summary>
            {text(
              `已移除任务（${removed.length}）`,
              `Removed tasks (${removed.length})`,
            )}
          </summary>
          <p>
            {text(
              "这里只隐藏最近任务入口；论文副本、生成文件和外部导出均未删除。",
              "Only the recent-task entry is hidden. Manuscript snapshots, generated files, and external exports were not deleted.",
            )}
          </p>
          {removed.map((item) => (
            <div className="removed-row" key={item.id}>
              <span>{item.displayName}</span>
              <button className="secondary" onClick={() => onRestore(item)}>
                {text("恢复到最近任务", "Restore to recent tasks")}
              </button>
            </div>
          ))}
        </details>
      ) : null}
    </section>
  );
}

function EmptyTask({
  kind,
  open,
  openFolder,
}: {
  kind: TaskKind;
  open: () => void;
  openFolder: () => void;
}) {
  const { text } = useI18n();
  const matching = kind === "find_journals";
  return (
    <section className="empty-task">
      <div className="empty-card">
        <span className="file-orbit">PDF / DOCX</span>
        <h1>
          {matching
            ? text("为这篇论文推荐期刊", "Find journals for this manuscript")
            : text(
                "按目标期刊整理投稿包",
                "Prepare a package for a target journal",
              )}
        </h1>
        <p>
          {text(
            "选择本地 PDF 或 DOCX 后会自动建立只读快照。取消选择不会丢失任何内容。",
            "Choosing a local PDF or DOCX creates a read-only snapshot automatically. Cancelling does not lose anything.",
          )}
        </p>
        <div className="empty-actions">
          <button className="primary" onClick={open}>
            {text("打开本地论文", "Open local manuscript")}
          </button>
          <button className="secondary" onClick={openFolder}>
            {text("打开材料文件夹", "Open materials folder")}
          </button>
        </div>
        <small>
          {text(
            "当前支持 PDF 与 DOCX；文件夹仅读取直接子文件，同一文件夹再次打开会回到同一个项目记录，并请你在多份主稿中明确选择。PDF 不会被伪装成 Word 文件。",
            "PDF and DOCX are supported. Folder import reads direct child files only, reopens the same project record for the same folder, and asks you to choose when several manuscripts are found. A PDF is never disguised as a Word file.",
          )}
        </small>
      </div>
    </section>
  );
}

function FolderManuscriptPicker({
  items,
  onChoose,
  onCancel,
}: {
  items: SelectedInput[];
  onChoose: (item: SelectedInput) => void;
  onCancel: () => void;
}) {
  const { text } = useI18n();
  return (
    <div className="modal-backdrop" role="presentation">
      <section
        className="folder-picker"
        role="dialog"
        aria-modal="true"
        aria-labelledby="folder-picker-title"
      >
        <span className="eyebrow">
          {text(
            "文件夹中有多份 PDF / DOCX 主稿",
            "Several PDF / DOCX manuscripts found",
          )}
        </span>
        <h2 id="folder-picker-title">
          {text("哪一份是本次主稿？", "Which file is the manuscript?")}
        </h2>
        <p>
          {text(
            "其他 PDF / DOCX 不会被自动当作附件。",
            "Other PDF / DOCX files will not be treated as attachments automatically.",
          )}
        </p>
        <div className="folder-candidates">
          {items.map((item) => (
            <button
              className="secondary"
              key={item.token}
              onClick={() => onChoose(item)}
            >
              <strong>{item.name}</strong>
              <small>
                {new Intl.NumberFormat(localeNumberLocale()).format(
                  item.sizeBytes,
                )}{" "}
                B
              </small>
            </button>
          ))}
        </div>
        <div className="footer-actions">
          <button className="secondary" onClick={onCancel}>
            {text("取消", "Cancel")}
          </button>
        </div>
      </section>
    </div>
  );
}

function Matching({
  project,
  setProject,
  open,
  openFolder,
  run,
  onPrepare,
  onDirtyChange,
}: {
  project: Project | null;
  setProject: (value: Project) => void;
  open: () => void;
  openFolder: () => void;
  run: <T>(action: () => Promise<T>) => Promise<T | null>;
  onPrepare: () => void;
  onDirtyChange: (dirty: boolean) => void;
}) {
  const { locale, text, localize } = useI18n();
  const [constraints, setConstraints] = useState(() =>
    constraintsForProject(project, locale),
  );
  const [keywords, setKeywords] = useState(() => keywordsForProject(project));
  const [result, setResult] = useState<RecommendationResult | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  useEffect(() => { setResult(null); setSelected(null); }, [project?.id, project?.revision]);
  useEffect(() => {
    onDirtyChange(!!project && (keywords !== keywordsForProject(project) || JSON.stringify(constraints) !== JSON.stringify(constraintsForProject(project, locale))));
    return () => onDirtyChange(false);
  }, [project, keywords, constraints, locale, onDirtyChange]);
  if (!project)
    return (
      <EmptyTask kind="find_journals" open={open} openFolder={openFolder} />
    );
  const calculate = async (
    overrides: Partial<AuthorConstraints> = {},
  ) => {
    const nextConstraints = { ...constraints, ...overrides };
    if (Object.keys(overrides).length) setConstraints(nextConstraints);
    const next = await run(() =>
      api.recommend(project.id, {
        ...nextConstraints,
        topicKeywords: keywords
          .split(/[,，;]/)
          .map((v) => v.trim())
          .filter(Boolean),
      }),
    );
    if (next) {
      setResult(next);
      setSelected(next.recommendations[0]?.journal.id ?? null);
    }
  };
  const continueToPackage = async () => {
    const journal = result?.recommendations.find(
      (item) => item.journal.id === selected,
    );
    if (!journal || !result) return;
    const next = await run(() =>
      api.selectTarget(
        project,
        journal.journal.id,
        "recommendation",
        result.runId,
        constraints.articleType ?? undefined,
      ),
    );
    if (next) {
      setProject(next);
      onPrepare();
    }
  };
  return (
    <section className="task-page">
      <TaskHeader
        eyebrow={text("任务 01 · 推荐期刊", "Task 01 · Find journals")}
        title={project.displayName}
        meta={`${project.activeSource.fileName} · ${project.activeSource.featureProfile.toUpperCase()}`}
        onChange={open}
      />
      {!result ? (
        <div className="content-grid">
          <section className="panel">
            <h2>
              {text(
                "这次投稿最看重什么？",
                "What matters most for this submission?",
              )}
            </h2>
            <p>
              {text(
                "只填真正的硬条件。留空会显示未知，不会被当作通过。",
                "Only enter real hard constraints. Empty values remain unknown, not passed.",
              )}
            </p>
            <div className="form-grid">
              <label>
                {text("投稿语言", "Submission language")}
                <select
                  value={constraints.requiredLanguage ?? ""}
                  onChange={(event) => {
                    setConstraints({
                      ...constraints,
                      requiredLanguage: event.target.value || null,
                    });
                  }}
                >
                  <option value="">{text("不限", "Any")}</option>
                  <option value="en">English</option>
                  <option value="zh-CN">简体中文</option>
                </select>
              </label>
              <label>
                {text("文章类型", "Article type")}
                <select
                  value={constraints.articleType ?? ""}
                  onChange={(event) =>
                    setConstraints({
                      ...constraints,
                      articleType: event.target.value || null,
                    })
                  }
                >
                  <option value="research_article">
                    {text("研究论文", "Research article")}
                  </option>
                  <option value="review_article">
                    {text("综述", "Review article")}
                  </option>
                </select>
              </label>
              <label className="wide">
                {text("研究关键词", "Research keywords")}
                <input
                  value={keywords}
                  onChange={(event) => setKeywords(event.target.value)}
                  placeholder={text(
                    "例如：机器学习，计算机视觉",
                    "e.g. machine learning, computer vision",
                  )}
                />
              </label>
              <details className="advanced-settings wide">
                <summary>
                  <span>{text("更多设置", "More settings")}</span>
                  <small>
                    {text(
                      "APC、开放获取、收录与审稿速度",
                      "APC, open access, indexing, and review speed",
                    )}
                  </small>
                </summary>
                <p>
                  {text(
                    "只有确实不能妥协的条件才需要勾选；其他项目已采用稳妥默认值。",
                    "Select only conditions that are truly non-negotiable; the remaining options use safe defaults.",
                  )}
                </p>
                <div className="advanced-settings-grid">
                  <label>
                    {text("最高 APC（可选）", "Maximum APC (optional)")}
                    <input
                      inputMode="numeric"
                      value={constraints.maximumApc ?? ""}
                      onChange={(event) =>
                        setConstraints({
                          ...constraints,
                          maximumApc: event.target.value
                            ? Number(event.target.value)
                            : null,
                        })
                      }
                    />
                  </label>
                  <label>
                    {text("币种", "Currency")}
                    <select
                      value={constraints.currency ?? "USD"}
                      onChange={(event) =>
                        setConstraints({
                          ...constraints,
                          currency: event.target.value,
                        })
                      }
                    >
                      <option>USD</option>
                      <option>CNY</option>
                      <option>EUR</option>
                    </select>
                  </label>
                  <label className="check">
                    <input
                      type="checkbox"
                      checked={constraints.requireOpenAccess}
                      onChange={(event) =>
                        setConstraints({
                          ...constraints,
                          requireOpenAccess: event.target.checked,
                        })
                      }
                    />
                    {text(
                      "必须提供开放获取路径",
                      "An open-access route is required",
                    )}
                  </label>
                  <fieldset className="constraint-options wide">
                    <legend>
                      {text("学校／单位要求收录", "Required indexing")}
                    </legend>
                    {[
                      [
                        "SCIE",
                        text("必须被 SCIE 收录", "Must be indexed in SCIE"),
                      ],
                      [
                        "Scopus",
                        text(
                          "必须被 Scopus 收录",
                          "Must be indexed in Scopus",
                        ),
                      ],
                    ].map(([value, label]) => (
                      <label className="check compact" key={value}>
                        <input
                          type="checkbox"
                          checked={constraints.requiredIndexing.includes(value)}
                          onChange={(event) =>
                            setConstraints({
                              ...constraints,
                              requiredIndexing: event.target.checked
                                ? [...constraints.requiredIndexing, value]
                                : constraints.requiredIndexing.filter(
                                    (candidate) => candidate !== value,
                                  ),
                            })
                          }
                        />
                        {label}
                      </label>
                    ))}
                  </fieldset>
                  <label className="check">
                    <input
                      type="checkbox"
                      checked={constraints.preferFastFirstDecision}
                      onChange={(event) =>
                        setConstraints({
                          ...constraints,
                          preferFastFirstDecision: event.target.checked,
                        })
                      }
                    />
                    {text(
                      "优先首轮决定较快的期刊（偏好，不是硬条件）",
                      "Prefer a faster first decision (preference, not a hard constraint)",
                    )}
                  </label>
                </div>
              </details>
            </div>
            <button
              className="primary align-right"
              onClick={() => void calculate()}
            >
              {text("推荐期刊", "Find journals")}
            </button>
          </section>
          <EvidenceAside />
        </div>
      ) : (
        <section>
          <div className="results-head">
            <div>
              <span className="eyebrow">
                {text("本地确定性结果", "Local deterministic results")}
              </span>
              <h2>
                {text(
                  "最多三个解释充分的候选",
                  "Up to three evidence-backed candidates",
                )}
              </h2>
            </div>
            <button className="secondary" onClick={() => setResult(null)}>
              {text("修改偏好", "Edit preferences")}
            </button>
          </div>
          {result.recommendations.length ? <JournalTargetMap items={result.recommendations} selected={selected} onSelect={setSelected} /> : null}
          {result.recommendations.length ? (
            <div className="recommendation-grid">
              {result.recommendations.map((item) => (
                <article
                  key={item.journal.id}
                  className={`recommendation-card ${selected === item.journal.id ? "selected" : ""}`}
                >
                  <button
                    className="recommendation-choice"
                    onClick={() => setSelected(item.journal.id)}
                    aria-pressed={selected === item.journal.id}
                  >
                    <span className="role">{roleLabel(item.role, text)}</span>
                    <h3>{item.journal.displayName}</h3>
                    <small>
                      {item.journal.publisher} ·{" "}
                      {item.journal.issn ?? item.journal.eissn}
                    </small>
                    <div className="card-section">
                      <b>{text("为什么推荐", "Why it fits")}</b>
                      {item.reasons.map((reason, index) => (
                        <p key={index}>{localize(reason)}</p>
                      ))}
                    </div>
                    <div className="card-section">
                      <b>{text("主要风险", "Main risk")}</b>
                      <p>
                        {item.risks.length
                          ? localize(item.risks[0])
                          : text("暂无已知硬冲突", "No known hard conflict")}
                      </p>
                    </div>
                  </button>
                  <details>
                    <summary>{text("查看依据", "View evidence")}</summary>
                    {item.constraints.length ? (
                      <div className="constraint-results">
                        <b>{text("硬条件核对", "Hard-constraint checks")}</b>
                        {item.constraints.map((constraint) => (
                          <span
                            className={`constraint-${constraint.status}`}
                            key={constraint.constraintId}
                          >
                            {localize(constraint.explanation)}
                          </span>
                        ))}
                      </div>
                    ) : null}
                    {item.preparation.length ? (
                      <div className="constraint-results">
                        <b>{text("选定后需要完成", "After selection")}</b>
                        {item.preparation.map((step, index) => (
                          <span key={index}>{localize(step)}</span>
                        ))}
                      </div>
                    ) : null}
                    {item.journal.evidence.map((evidence) => (
                      <span
                        className="evidence-reference"
                        key={evidence.sourceUrl}
                      >
                        <b>
                          {localize(evidence.label)} · {evidence.verifiedAt}
                        </b>
                        <code>{evidence.sourceUrl}</code>
                      </span>
                    ))}
                  </details>
                </article>
              ))}
            </div>
          ) : (
            <div className="empty-result">
              <h3>
                {text(
                  "没有满足全部硬条件的候选",
                  "No candidate meets every hard constraint",
                )}
              </h3>
              <p>
                {constraints.requiredLanguage === "zh-CN"
                  ? text(
                      "当前内置期刊目录还没有已核验、可生成投稿包的简体中文期刊。系统没有自动修改投稿语言。",
                      "The built-in catalog does not yet contain a verified, package-ready Simplified Chinese journal. The submission language was not changed automatically.",
                    )
                  : text(
                      "请修改条件；系统不会为了凑数自动放宽。",
                      "Edit the constraints. They will not be relaxed automatically to fill the list.",
                    )}
              </p>
              {constraints.requiredLanguage === "zh-CN" ? (
                <button
                  className="secondary"
                  onClick={() => {
                    void calculate({ requiredLanguage: "en" });
                  }}
                >
                  {text(
                    "改为 English 并重新推荐",
                    "Switch to English and retry",
                  )}
                </button>
              ) : null}
            </div>
          )}
          {result.needsVerification.length ? (
            <section className="verification-pool">
              <h3>
                {text("另有候选需要补证", "Other candidates need verification")}
              </h3>
              <p>
                {text(
                  "这些期刊可能满足部分条件，但规则或费用存在未知，因此不会进入可选前三名。",
                  "These journals may meet some constraints, but unknown rules or fees keep them out of the selectable top three.",
                )}
              </p>
              <div>
                {result.needsVerification.slice(0, 6).map((journal) => (
                  <span key={journal.id}>{journal.displayName}</span>
                ))}
              </div>
            </section>
          ) : null}
          <div className="sticky-action">
            <span>
              {selected
                ? result.recommendations.find((v) => v.journal.id === selected)
                    ?.journal.displayName
                : text("选择一个候选", "Choose a candidate")}
            </span>
            <button
              className="primary"
              disabled={!selected}
              onClick={continueToPackage}
            >
              {text("按这本期刊整理投稿包", "Prepare package for this journal")}
            </button>
          </div>
        </section>
      )}
    </section>
  );
}

function Preparation({
  refreshKey,
  project,
  setProject,
  open,
  openFolder,
  run,
  onDirtyChange,
}: {
  refreshKey: number;
  project: Project | null;
  setProject: (value: Project) => void;
  open: () => void;
  openFolder: () => void;
  run: <T>(action: () => Promise<T>) => Promise<T | null>;
  onDirtyChange: (dirty: boolean) => void;
}) {
  const { text, localize } = useI18n();
  const [journals, setJournals] = useState<JournalRecord[]>([]);
  const [query, setQuery] = useState("");
  const [preparation, setPreparation] = useState<PreparationView | null>(null);
  const [facts, setFacts] = useState<DocumentFacts | null>(
    project?.facts ?? null,
  );
  const factsBase = useRef(project);
  const factsRef = useRef(facts);
  factsRef.current = facts;
  const [factConflicts, setFactConflicts] = useState<FactConflict[]>([]);
  useEffect(() => {
    onDirtyChange(factConflicts.length > 0 || (!!project && !!facts && JSON.stringify(facts) !== JSON.stringify(project.facts)));
    return () => onDirtyChange(false);
  }, [project, facts, factConflicts, onDirtyChange]);
  const [materialKind, setMaterialKind] = useState("supplementary");
  const [authorConfirmed, setAuthorConfirmed] = useState(false);
  const [changingTarget, setChangingTarget] = useState(false);
  const [targetName, setTargetName] = useState<string | null>(null);
  const [targetJournal, setTargetJournal] = useState<JournalRecord | null>(null);
  const [pkg, setPackage] = useState<CompiledPackage | null>(null);
  const [receipt, setReceipt] = useState<ExportReceipt | null>(null);
  const [workspaceRefresh, setWorkspaceRefresh] = useState(0);
  const preparationTarget = useRef<string | null>(null);
  const [preparationLoading, setPreparationLoading] = useState(false);
  useEffect(() => {
    let active = true;
    let request = 0;
    const refresh = () => {
      const current = ++request;
      setPreparationLoading(!!project?.target);
      if (project?.target) void api.preparation(project.id).then(value => {
        if (active && current === request) { setPreparation(value); setPackage(current => current && current.contextHash !== value.contextHash ? null : current); }
      }).catch(() => { if (active && current === request) setPreparation(null); })
        .finally(() => { if (active && current === request) setPreparationLoading(false); });
    };
    const targetKey = project?.target ? `${project.id}/${project.target.journalId}` : null;
    if (preparationTarget.current !== targetKey) setPreparation(null);
    preparationTarget.current = targetKey;
    refresh();
    window.addEventListener("focus", refresh);
    return () => { active = false; window.removeEventListener("focus", refresh); };
  }, [refreshKey, workspaceRefresh, project?.id, project?.revision, project?.target?.journalId]);
  useEffect(() => {
    let active = true;
    void api.journals("").then(items => { if (active) setJournals(items); }).catch(() => undefined);
    return () => { active = false; };
  }, [refreshKey, project?.id, changingTarget]);
  useEffect(() => {
    const base = factsBase.current;
    if (base && project && base.id === project.id && factsRef.current) {
      const recovered = reconcileFactDraft(base, factsRef.current, project);
      setFacts(recovered.facts);
      setFactConflicts(previous => {
        // Keep unresolved inputs even if another update arrives during review.
        const pending = previous.map(conflict => ({ ...conflict, saved: project.facts[conflict.field] }));
        return [...pending.filter(old => !recovered.conflicts.some(next => next.field === old.field)), ...recovered.conflicts];
      });
    } else { setFacts(project?.facts ?? null); setFactConflicts([]); }
    factsBase.current = project;
    setAuthorConfirmed(false);
    setPackage(null);
    setReceipt(null);
  }, [project?.id, project?.revision, project?.target]);
  useEffect(() => {
    const journalId = project?.target?.journalId;
    let active = true;
    setTargetJournal(null);
    if (!journalId) {
      setTargetName(null);
      return;
    }
    void api
      .journals(journalId)
      .then((items) => {
        if (!active) return;
        const journal = items.find((item) => item.id === journalId) ?? null;
        setTargetName(journal?.displayName ?? journalId);
        setTargetJournal(journal);
      })
      .catch(() => { if (active) setTargetName(journalId); });
    return () => { active = false; };
  }, [refreshKey, project?.target?.journalId]);
  const search = async () => {
    const result = await run(() => api.journals(query));
    if (result) setJournals(result);
  };
  const select = async (journal: JournalRecord) => {
    if (!project) return;
    const result = await run(() =>
      api.selectTarget(project, journal.id, "catalog"),
    );
    if (result) {
      setProject(result);
      setJournals([]);
      setChangingTarget(false);
      setPackage(null);
    }
  };
  const saveFacts = async () => {
    if (!project || !facts || !authorConfirmed || factConflicts.length) return;
    const confirmed = {
      ...facts,
      authorConfirmedFields: [
        "title",
        "authors",
        "affiliations",
        "corresponding_email",
        "conflict_of_interest",
        "funding",
        "data_availability",
        "ethics_statement",
        "highlights",
        "abstract_text",
        "keywords",
        "credit_contributions",
        "generative_ai_disclosure",
      ],
    };
    const result = await run(async () => {
      try { return await api.updateFacts(project, confirmed); }
      catch (cause) {
        const error = cause as AppError;
        if (error.code !== "CONTEXT_CHANGED") throw cause;
        let latest: Project;
        try { latest = await api.project(project.id); }
        catch { throw { ...error, code: "FACTS_RELOAD_FAILED" }; }
        setProject(latest);
        setAuthorConfirmed(false);
        throw { ...error, code: "FACTS_RELOADED" };
      }
    });
    if (result) {
      factsBase.current = result;
      factsRef.current = result.facts;
      setFacts(result.facts);
      setFactConflicts([]);
      setProject(result);
      setAuthorConfirmed(false);
    }
  };
  const addMaterial = async () => {
    if (!project) return;
    const selection = await run(() =>
      api.choose(
        materialKind === "anonymized_manuscript" ||
          materialKind === "editable_manuscript"
          ? "editable_manuscript"
          : "material",
      ),
    );
    if (!selection || selection.status === "cancelled") return;
    const result = await run(() =>
      api.addMaterial(project, selection.items[0].token, materialKind),
    );
    if (result) setProject(result);
  };
  const build = async (mode: "draft" | "final") => {
    if (!project || !preparation || preparationLoading) return;
    const result = await run(() =>
      api.build(project.id, preparation.contextHash, mode),
    );
    if (result) {
      setPackage(result);
      setWorkspaceRefresh(value => value + 1);
      setReceipt(null);
    }
  };
  const exportFiles = async () => {
    if (!pkg || !project) return;
    const selection = await run(() =>
      api.chooseProjectExport(project.id),
    );
    if (!selection || selection.status === "cancelled") return;
    const result = await run(() => api.export(pkg, selection.items[0].token));
    if (result) setReceipt(result);
  };
  if (!project)
    return (
      <EmptyTask kind="prepare_package" open={open} openFolder={openFolder} />
    );
  // Match compiler.rs: an included anonymized manuscript takes precedence,
  // then an included editable manuscript, then the immutable import source.
  const packageManuscript = project.materials.find(material => material.included && material.kind === "anonymized_manuscript")
    ?? project.materials.find(material => material.included && material.kind === "editable_manuscript");
  const needsDocx = !packageManuscript && project.activeSource.format === "pdf";
  return (
    <section className="task-page">
      <TaskHeader
        eyebrow={text("任务 02 · 整理投稿包", "Task 02 · Prepare package")}
        title={project.displayName}
        meta={`${needsDocx ? text("导入来源", "Imported source") : text("投稿主稿", "Submission manuscript")}: ${packageManuscript?.fileName ?? project.activeSource.fileName} · ${project.materials.length} ${text("份附件", "attachment(s)")}`}
        onChange={open}
      />
      {needsDocx ? (
        <aside className="pdf-source-note">
          <strong>{text("尚未关联投稿主稿 DOCX", "No submission manuscript DOCX linked")}</strong>
          <span>
            {text(
              "最初导入的 PDF 保留为项目来源，可用于期刊匹配和草稿检查。正式导出前请在材料清单中选择并关联作者核对的 DOCX；仅删除目录中的 PDF 不会关联新主稿。系统不会把 PDF 转换或改名成 Word 文件。",
              "The imported PDF is retained as the project source for journal matching and draft review. Before final export, choose and link an author-verified DOCX in the material checklist. Deleting the PDF from the folder does not link a new manuscript. The app will not convert or rename the PDF as a Word file.",
            )}
          </span>
        </aside>
      ) : null}
      {!project.target || changingTarget ? (
        <section className="target-picker panel">
          <span className="eyebrow">
            {project.target
              ? text("更换目标期刊", "Change target journal")
              : text("先选择目标", "Choose a target")}
          </span>
          <h2>
            {text("搜索已内置期刊", "Search the built-in journal catalog")}
          </h2>
          <p>
            {text(
              "三本试点期刊具备本地投稿包规则；其他目录记录不会冒充完整支持。导出前仍应在官网复核最新要求。",
              "Three pilot journals have local package rules. Other catalog records are not presented as fully supported. Recheck the latest official requirements before submission.",
            )}
          </p>
          <div className="search-row">
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") void search();
              }}
              aria-label={text("搜索目标期刊", "Search target journals")}
              placeholder={text("中文名、英文名、缩写或 ISSN", "Chinese or English name, abbreviation, or ISSN")}
            />
            <button className="primary" onClick={search}>
              {text("搜索", "Search")}
            </button>
          </div>
          <div className="journal-list">
            {journals.map((journal) => (
              <button
                key={journal.id}
                onClick={() => select(journal)}
                disabled={journal.generationCoverage !== "supported"}
              >
                <span>
                  <strong>{journal.displayName}</strong>
                  <small>
                    {journal.publisher} · {journal.issn ?? journal.eissn}
                  </small>
                </span>
                <em
                  className={
                    journal.generationCoverage === "supported"
                      ? "verified"
                      : "catalog-only"
                  }
                >
                  {journal.generationCoverage === "supported"
                    ? text("试点规则", "Pilot rules")
                    : text("仅目录", "Catalog only")}
                </em>
              </button>
            ))}
          </div>
          {project.target ? (
            <div className="footer-actions">
              <button
                className="secondary"
                onClick={() => setChangingTarget(false)}
              >
                {text("取消更换", "Cancel change")}
              </button>
            </div>
          ) : null}
        </section>
      ) : !pkg ? (
        <div className="content-grid package-content-grid">
          <section className="panel preparation">
            <div className="target-line">
              <span>{text("目标", "Target")}</span>
              <strong>{targetName ?? project.target.journalId}</strong>
              <button
                className="link"
                onClick={() => {
                  setChangingTarget(true);
                  setJournals([]);
                }}
              >
                {text("更换", "Change")}
              </button>
            </div>
            {preparation ? (
              <>
                <div className={`status-block ${preparation.status}`}>
                  <span>{statusLabel(preparation.status, text)}</span>
                  <strong>
                    {preparation.blockers.length
                      ? text(
                          `还需要你完成 ${preparation.blockers.length} 项`,
                          `${preparation.blockers.length} item(s) still needed`,
                        )
                      : text(
                          "当前要求已齐备",
                          "Current requirements are complete",
                        )}
                  </strong>
                </div>
                <PackageWorkspace key={`${project.id}-${project.target.journalId}`} project={project} setProject={setProject} run={run} refreshKey={`${refreshKey}-${workspaceRefresh}`} onLocationChanged={() => setWorkspaceRefresh(value => value + 1)} />
                <MaterialChecklist key={`materials-${project.id}-${project.target.journalId}`} project={project} setProject={setProject} refreshKey={refreshKey + workspaceRefresh} run={run} onGenerated={() => setWorkspaceRefresh(value => value + 1)} />
                <section className="target-requirements" aria-label={text("目标期刊要求", "Target journal requirements")}>
                  <h3>{text("目标期刊要求与待补材料", "Target requirements and missing materials")}</h3>
                  {targetJournal?.evidence.map(evidence => <button key={evidence.sourceUrl} className="link" onClick={() => void run(() => api.openSource(project.target!.journalId, evidence.sourceUrl))}>{localize(evidence.label)} ↗ </button>)}
                  <p>{text("先按目标期刊确认以下清单，再补充作者信息和文件。", "Review this target-specific checklist, then complete the author details and files.")}</p>
                  {[...preparation.blockers, ...preparation.warnings, ...preparation.readyItems].map(item => <details key={item.requirementId}>
                    <summary><strong>{localize(item.label)}</strong><span>{item.status === "ready" ? text("已齐备", "Ready") : text("待补充", "To complete")} · {item.required ? text("必需", "Required") : text("建议", "Recommended")}</span></summary>
                    <p>{localize(item.description)}</p>
                    <button className="link" onClick={() => void run(() => api.openSource(project.target!.journalId, item.evidence.sourceUrl))}>{localize(item.evidence.label)} ↗</button>
                    <small>{text("核验日期", "Verified on")} · {item.evidence.verifiedAt}</small>
                    <code>{item.evidence.sourceUrl}</code>
                  </details>)}
                </section>
                {factConflicts.length > 0 && <section className="fact-conflicts" aria-label={text("核对同时修改的内容", "Review conflicting edits")}>
                  <h3>{text("请核对同时修改的内容", "Review conflicting edits")}</h3>
                  <p>{text("最新状态已读取，本次输入仍保留。原稿或相同字段发生了变化，请逐项选择，再重新确认保存。", "The latest state is loaded and your input is preserved. The source or the same fields changed. Choose each value, then confirm and save again.")}</p>
                  {factConflicts.map(conflict => <div key={conflict.field}>
                    <strong>{text(factLabels[conflict.field][0], factLabels[conflict.field][1])}</strong>
                    <p>{text("本次输入", "Your input")}</p><pre>{Array.isArray(conflict.local) ? conflict.local.join("\n") : conflict.local || text("（空）", "(empty)")}</pre>
                    <p>{text("已保存内容", "Saved value")}</p><pre>{Array.isArray(conflict.saved) ? conflict.saved.join("\n") : conflict.saved || text("（空）", "(empty)")}</pre>
                    <div className="inline-actions">{(["local", "saved"] as const).map(choice => <button className="secondary" key={choice} onClick={() => {
                      setFacts(current => current && ({ ...current, [conflict.field]: conflict[choice] }));
                      setFactConflicts(current => current.filter(item => item.field !== conflict.field));
                      setAuthorConfirmed(false);
                    }}>{choice === "local" ? text("采用本次输入", "Use my input") : text("采用已保存内容", "Use saved value")}</button>)}</div>
                  </div>)}
                </section>}
                <div className="fact-form">
                  <label>
                    {text("论文标题", "Manuscript title")}
                    <input
                      value={facts?.title ?? ""}
                      onChange={(event) =>
                        facts &&
                        setFacts({ ...facts, title: event.target.value })
                      }
                    />
                  </label>
                  <label className="wide">
                    {text("摘要", "Abstract")}
                    <textarea
                      value={facts?.abstractText ?? ""}
                      onChange={(event) =>
                        facts &&
                        setFacts({ ...facts, abstractText: event.target.value })
                      }
                    />
                  </label>
                  <label className="wide">
                    {text("关键词（每行一个）", "Keywords (one per line)")}
                    <textarea
                      value={facts?.keywords.join("\n") ?? ""}
                      onChange={(event) =>
                        facts &&
                        setFacts({
                          ...facts,
                          keywords: event.target.value
                            .split("\n")
                            .map((value) => value.trim())
                            .filter(Boolean),
                        })
                      }
                    />
                  </label>
                  <label>
                    {text("作者（每行一位）", "Authors (one per line)")}
                    <textarea
                      value={facts?.authors.join("\n") ?? ""}
                      onChange={(event) =>
                        facts &&
                        setFacts({
                          ...facts,
                          authors: event.target.value
                            .split("\n")
                            .map((v) => v.trim())
                            .filter(Boolean),
                        })
                      }
                    />
                  </label>
                  <label>
                    {text(
                      "作者单位（每行一个）",
                      "Affiliations (one per line)",
                    )}
                    <textarea
                      value={facts?.affiliations.join("\n") ?? ""}
                      onChange={(event) =>
                        facts &&
                        setFacts({
                          ...facts,
                          affiliations: event.target.value
                            .split("\n")
                            .map((v) => v.trim())
                            .filter(Boolean),
                        })
                      }
                    />
                  </label>
                  <label>
                    {text("通讯作者邮箱", "Corresponding-author email")}
                    <input
                      type="email"
                      value={facts?.correspondingEmail ?? ""}
                      onChange={(event) =>
                        facts &&
                        setFacts({
                          ...facts,
                          correspondingEmail: event.target.value,
                        })
                      }
                    />
                  </label>
                  <label className="wide">
                    {text("利益冲突声明", "Competing-interest statement")}
                    <textarea
                      value={facts?.conflictOfInterest ?? ""}
                      onChange={(event) =>
                        facts &&
                        setFacts({
                          ...facts,
                          conflictOfInterest: event.target.value,
                        })
                      }
                      placeholder={text(
                        "必须由作者填写或确认",
                        "Must be entered or confirmed by the author",
                      )}
                    />
                  </label>
                  <label className="wide">
                    {text("经费声明", "Funding statement")}
                    <textarea
                      value={facts?.funding ?? ""}
                      onChange={(event) =>
                        facts && setFacts({ ...facts, funding: event.target.value })
                      }
                      placeholder={text(
                        "如不适用，也请由作者明确说明",
                        "If not applicable, state that explicitly",
                      )}
                    />
                  </label>
                  <label className="wide">
                    {text("数据可用性声明", "Data-availability statement")}
                    <textarea
                      value={facts?.dataAvailability ?? ""}
                      onChange={(event) =>
                        facts &&
                        setFacts({ ...facts, dataAvailability: event.target.value })
                      }
                    />
                  </label>
                  <label className="wide">
                    {text("伦理声明", "Ethics statement")}
                    <textarea
                      value={facts?.ethicsStatement ?? ""}
                      onChange={(event) =>
                        facts &&
                        setFacts({ ...facts, ethicsStatement: event.target.value })
                      }
                    />
                  </label>
                  <label className="wide">
                    {text("研究亮点（每行一条）", "Highlights (one per line)")}
                    <textarea
                      value={facts?.highlights.join("\n") ?? ""}
                      onChange={(event) =>
                        facts &&
                        setFacts({
                          ...facts,
                          highlights: event.target.value
                            .split("\n")
                            .map((value) => value.trim())
                            .filter(Boolean),
                        })
                      }
                    />
                  </label>
                  <label className="wide">
                    {text("CRediT 作者贡献", "CRediT author contributions")}
                    <textarea
                      value={facts?.creditContributions ?? ""}
                      onChange={(event) =>
                        facts &&
                        setFacts({
                          ...facts,
                          creditContributions: event.target.value,
                        })
                      }
                      placeholder={text(
                        "逐位作者填写已核对的贡献角色",
                        "Enter the verified contributor roles for each author",
                      )}
                    />
                  </label>
                  <label className="wide">
                    {text(
                      "生成式 AI 使用披露（如适用）",
                      "Generative-AI disclosure (if applicable)",
                    )}
                    <textarea
                      value={facts?.generativeAiDisclosure ?? ""}
                      onChange={(event) =>
                        facts &&
                        setFacts({
                          ...facts,
                          generativeAiDisclosure: event.target.value,
                        })
                      }
                      placeholder={text(
                        "仅在实际使用时填写工具、用途及人工复核说明",
                        "If used, state the tool, purpose, and human review",
                      )}
                    />
                  </label>
                  <label className="check confirm-fields">
                    <input
                      type="checkbox"
                      checked={authorConfirmed}
                      onChange={(event) =>
                        setAuthorConfirmed(event.target.checked)
                      }
                    />
                    {text(
                      "我已核对以上摘要、关键词、作者信息、声明和研究亮点；这些内容不会由系统推断",
                      "I checked the abstract, keywords, author details, declarations, and highlights; the app will not infer them",
                    )}
                  </label>
                </div>
                {project.materials.some(
                  (material) => material.kind === "unclassified",
                ) ? (
                  <div className="unclassified-materials">
                    <strong>
                      {text(
                        "文件夹中的其他文件尚未加入投稿包",
                        "Other folder files are not yet included",
                      )}
                    </strong>
                    <p>
                      {text(
                        "请用“选择附件”明确加入需要的文件；系统不会猜测附件用途。",
                        "Use Choose attachment to add each required file explicitly. The app will not guess its purpose.",
                      )}
                    </p>
                    {project.materials
                      .filter((material) => material.kind === "unclassified")
                      .map((material) => (
                        <small key={material.id}>{material.fileName}</small>
                      ))}
                  </div>
                ) : null}
                <div className="inline-actions">
                  <label className="material-kind">
                    <span>{text("附件用途", "Attachment purpose")}</span>
                    <select
                      value={materialKind}
                      onChange={(event) => setMaterialKind(event.target.value)}
                    >
                      <option value="supplementary">
                        {text("补充材料", "Supplementary material")}
                      </option>
                      <option value="figure">
                        {text("图或图表", "Figure or artwork")}
                      </option>
                      <option value="reporting_checklist">
                        {text("报告清单", "Reporting checklist")}
                      </option>
                      <option value="anonymized_manuscript">
                        {text(
                          "作者核对的匿名稿 DOCX",
                          "Author-verified anonymized DOCX",
                        )}
                      </option>
                      <option value="editable_manuscript">
                        {text(
                          "作者核对的可编辑主稿 DOCX",
                          "Author-verified editable manuscript DOCX",
                        )}
                      </option>
                    </select>
                  </label>
                  <button className="secondary" onClick={addMaterial}>
                    {text("选择附件", "Choose attachment")}
                  </button>
                  <button
                    className="secondary"
                    disabled={!authorConfirmed || factConflicts.length > 0}
                    onClick={saveFacts}
                  >
                    {text("保存、确认并检查", "Save, confirm, and check")}
                  </button>
                </div>
                <details className="package-plan">
                  <summary>
                    {text(
                      `查看计划生成的 ${preparation.packagePlan.filePlan.length} 个文件`,
                      `View ${preparation.packagePlan.filePlan.length} planned files`,
                    )}
                  </summary>
                  <p>
                    {text(
                      "主稿与附件只复制，不执行排版或匿名化变换。",
                      "The manuscript and attachments are copied only; no formatting or anonymization transform is planned.",
                    )}
                  </p>
                  <ul>
                    {preparation.packagePlan.filePlan.map((file) => (
                      <li key={file.relativePath}>
                        <code>{file.relativePath}</code>
                        <span>{operationLabel(file.operation, text)}</span>
                      </li>
                    ))}
                  </ul>
                </details>
                <div className="footer-actions">
                  <button
                    className="secondary"
                    disabled={
                      preparationLoading || !preparation.allowedActions.includes("build_draft")
                    }
                    onClick={() => build("draft")}
                  >
                    {text("预览草稿", "Preview draft")}
                  </button>
                  <button
                    className="primary"
                    disabled={
                      preparationLoading || !preparation.allowedActions.includes("build_final")
                    }
                    onClick={() => build("final")}
                  >
                    {text("预览投稿包", "Preview package")}
                  </button>
                </div>
              </>
            ) : (
              <p>
                {text("正在读取内置要求…", "Loading built-in requirements…")}
              </p>
            )}
          </section>
          <EvidenceAside />
        </div>
      ) : (
        <section className="package-preview panel">
          <div className="results-head">
            <div>
              <span className="eyebrow">
                {pkg.mode === "draft"
                  ? text("草稿预览", "Draft preview")
                  : text("真实生成文件", "Generated files")}
              </span>
              <h2>
                {text(
                  "投稿文件与作者记录已分开",
                  "Submission files and author records are separated",
                )}
              </h2>
            </div>
            <button className="secondary" onClick={() => setPackage(null)}>
              {text("返回补充", "Back to preparation")}
            </button>
          </div>
          {pkg.warnings.map((warning, index) => (
            <div className="draft-warning" key={index}>
              {localize(warning)}
            </div>
          ))}
          <div className="file-manifest">
            {pkg.files.map((file) => (
              <div key={file.id}>
                <span
                  className={`file-kind ${file.publisherFile ? "publisher" : "internal"}`}
                >
                  {file.publisherFile
                    ? text("投稿", "Publisher")
                    : text("留档", "Internal")}
                </span>
                <span>
                  <strong>{file.relativePath}</strong>
                  <small>
                    {localize(file.purpose)} ·{" "}
                    {new Intl.NumberFormat(localeNumberLocale()).format(
                      file.sizeBytes,
                    )}{" "}
                    B
                  </small>
                </span>
              </div>
            ))}
          </div>
          {receipt ? (
            <div className="success" role="status">
              <strong>{text("投稿包已保存", "Package saved")}</strong>
              <span>{receipt.outputDirectory}</span>
              <small>
                {text(
                  `共 ${receipt.fileCount} 个文件；应用未执行真实投稿。`,
                  `${receipt.fileCount} files; the app did not submit them to a journal.`,
                )}
              </small>
              {!receipt.recordPersisted ? (
                <small className="inline-warning">
                  {text(
                    "文件已经保存，但本地回执记录失败；请保留此页面并核对导出目录。",
                    "The files were saved, but the local receipt record failed. Keep this page open and verify the export folder.",
                  )}
                </small>
              ) : null}
            </div>
          ) : (
            <>
              {project.workspace?.kind === "folder" ? (
                <p className="folder-export-note">
                  {text(
                    `将在“${project.workspace.name}”内新建带版本标识的 ManuscriptDock 输出文件夹，不会覆盖原稿或附件。`,
                    `A versioned ManuscriptDock output folder will be created inside “${project.workspace.name}”. The manuscript and attachments will not be overwritten.`,
                  )}
                </p>
              ) : null}
              <div className="footer-actions">
                <button className="primary" onClick={exportFiles}>
                  {project.workspace?.kind === "folder"
                    ? text("保存到项目文件夹", "Save to project folder")
                    : pkg.mode === "draft"
                      ? text("保存草稿", "Save draft")
                      : text("保存投稿包", "Save package")}
                </button>
              </div>
            </>
          )}
        </section>
      )}
    </section>
  );
}

function TaskHeader({
  eyebrow,
  title,
  meta,
  onChange,
}: {
  eyebrow: string;
  title: string;
  meta: string;
  onChange: () => void;
}) {
  const { text } = useI18n();
  return (
    <header className="task-header">
      <div>
        <span className="eyebrow">{eyebrow}</span>
        <h1>{title}</h1>
        <p>{meta}</p>
      </div>
      <button className="secondary" onClick={onChange}>
        {text("打开另一篇", "Open another")}
      </button>
    </header>
  );
}
function EvidenceAside() {
  const { text } = useI18n();
  return (
    <aside className="evidence-aside">
      <span className="eyebrow">{text("执行边界", "Processing boundary")}</span>
      <h3>
        {text("本地优先，外发前确认", "Local first, consent before sending")}
      </h3>
      <ul>
        <li>{text("基础流程无需模型；AI 调用前预览并确认", "Core workflows need no model; preview and approve each AI request")}</li>
        <li>{text("不实时抓取期刊官网", "No live journal-site scraping")}</li>
        <li>
          {text(
            "源稿只读，输出另存",
            "Read-only source; outputs are saved separately",
          )}
        </li>
      </ul>
      <p>
        {text(
          "内置规则随应用版本维护，核验日期和来源可展开查看。",
          "Built-in rules are maintained with the app version. Verification dates and sources remain inspectable.",
        )}
      </p>
    </aside>
  );
}
function roleLabel(role: string, text: (zh: string, en: string) => string) {
  if (role === "best_overall_fit") return text("综合推荐", "Best overall fit");
  if (role === "ambitious_option") return text("进阶选择", "Ambitious option");
  return text("准备更省力", "Less preparation needed");
}
function statusLabel(
  status: PreparationView["status"],
  text: (zh: string, en: string) => string,
) {
  if (status === "ready_to_export") return text("可以导出", "Ready to export");
  if (status === "draft_ready") return text("草稿可预览", "Draft ready");
  return text("待补充", "Needs input");
}

function operationLabel(
  operation: string,
  text: (zh: string, en: string) => string,
) {
  if (operation === "copy_source_unchanged")
    return text("原样复制主稿", "Copy source unchanged");
  if (operation === "use_author_verified_anonymized_manuscript")
    return text(
      "使用作者核对的匿名稿",
      "Use author-verified anonymized manuscript",
    );
  if (operation === "use_author_verified_editable_manuscript")
    return text(
      "使用作者核对的可编辑主稿",
      "Use author-verified editable manuscript",
    );
  if (operation === "provide_editable_manuscript")
    return text(
      "需补充作者核对的可编辑 DOCX",
      "Provide an author-verified editable DOCX",
    );
  if (operation === "provide_anonymized_manuscript")
    return text("需补充作者核对的匿名 DOCX", "Provide an author-verified anonymized DOCX");
  if (operation === "copy_attachment_unchanged")
    return text("原样复制附件", "Copy attachment unchanged");
  if (operation === "generate_from_confirmed_fact")
    return text("从已确认事实生成", "Generate from confirmed facts");
  if (operation === "archive_submission_files")
    return text("仅归档投稿文件", "Archive submission files only");
  if (operation === "write_audit_record")
    return text("写入本地记录", "Write local audit record");
  return text("生成作者核对文件", "Generate author-review file");
}
function localeNumberLocale() {
  return document.documentElement.lang === "en" ? "en" : "zh-CN";
}
function uiError(code: string): AppError {
  return { code, retryable: true, diagnosticId: `ui-${crypto.randomUUID()}` };
}

export default function App() {
  return (
    <I18nProvider>
      <TextSizeProvider>
        <AppContent />
      </TextSizeProvider>
    </I18nProvider>
  );
}
