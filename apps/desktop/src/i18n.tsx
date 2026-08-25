import type { ReactNode } from "react";
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";

export type Locale = "zh-CN" | "en";

interface I18nValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  text: (chinese: string, english: string) => string;
}

const STORAGE_KEY = "manuscriptdock.locale";
const I18nContext = createContext<I18nValue | null>(null);

export function localize(locale: Locale, chinese: string, english: string) {
  return locale === "zh-CN" ? chinese : english;
}

const BACKEND_ENGLISH: Record<string, string> = {
  "未检测到 \\title{}": "No \\title{} declaration was detected",
  "未检测到 \\author{}": "No \\author{} declaration was detected",
  "未检测到 Word 标题样式": "No Word title style was detected",
  "未检测到 Word 作者样式或可靠的首页作者行": "No Word author style or reliable first-page author line was detected",
  "已使用增强字体映射读取 PDF 文本层；多栏顺序、公式和复杂版式仍需人工确认": "The PDF text layer was read with enhanced font mapping; columns, formulas, and complex layouts still need manual confirmation",
  "已使用基础 PDF 内容流读取文本；字体映射不完整，结构结果需人工确认": "The PDF text was read from its basic content stream; incomplete font mapping means the structure needs manual confirmation",
  "PDF 未包含可读取的文本层，已标记为 OCR 候选": "The PDF has no readable text layer and has been marked as an OCR candidate",
  "本次分析没有执行 OCR，源文件和现有快照均未改动": "OCR was not run; the source file and existing snapshots remain unchanged",
  "章节层级优先采用 PDF 内置书签目录": "Section hierarchy was taken from the PDF bookmarks when available",
  "未可靠识别作者；请核对首页作者行或 PDF 元数据": "Authors could not be identified reliably; verify the first-page author line or PDF metadata",
  "作者根据首页版式候选行识别，请作者核对姓名与顺序": "Authors were inferred from first-page layout candidates; verify every name and the author order",
  "未定位摘要标题或摘要正文；请核对首页与双语摘要页": "No abstract heading or text was located; verify the first page and any bilingual abstract page",
  "未找到显式摘要标题；已根据首页连续正文识别摘要候选，请作者确认": "No explicit abstract heading was found; a candidate was inferred from continuous first-page text and requires author confirmation",
  "补充关键词有助于投稿系统录入与检索。": "Adding keywords helps submission-system entry and discovery.",
  "正文未检测到足够的章节结构，请确认解析结果或稿件结构。": "Not enough body sections were detected; confirm the extraction result or manuscript structure.",
  "请确认是否需要利益冲突声明。": "Confirm whether a conflict-of-interest statement is required.",
  "请确认目标期刊是否要求数据可用性声明。": "Confirm whether the target journal requires a data-availability statement.",
  "PDF 结构提取受版面限制，请作者核对检查结果。": "PDF structure extraction is layout-limited; the author should verify the results.",
  "论文需要可识别的标题。": "The manuscript needs an identifiable title.",
  "初投稿稿件通常需要摘要。": "An initial submission usually requires an abstract.",
  "研究论文需要可识别的参考文献部分。": "A research manuscript needs an identifiable references section.",
  "ManuscriptDock 通用初投稿准备规则": "ManuscriptDock general initial-submission rules",
  "ManuscriptDock 通用论文结构基线": "ManuscriptDock general manuscript-structure baseline",
  "本地选择状态不可用，请重启应用后再试": "The local selection state is unavailable. Restart the app and try again.",
  "该文件选择已失效，请重新选择论文": "This file selection has expired. Select the manuscript again.",
  "本地工作区标识无效": "The local workspace identifier is invalid.",
  "未找到需要管理的本地工作区": "The local workspace to manage was not found.",
  "目标位置已存在同一工作区，未移动任何文件": "The destination already contains this workspace; no files were moved.",
  "导入期间源稿件发生变化，请重新选择后再试": "The source manuscript changed during import. Select it again and retry.",
  "所选稿件与当前版本内容完全一致，未创建重复版本": "The selected manuscript is identical to the current version, so no duplicate version was created.",
  "新版本必须与当前稿件保持相同文件类型；格式转换应作为投稿输出保存": "A new version must use the same file type as the current manuscript; save format conversions as submission outputs.",
  "版本说明不能超过 200 个字符": "The version note cannot exceed 200 characters.",
  "系统时间无效，无法创建审计记录": "The system time is invalid, so an audit record could not be created.",
  "内置规则信任锚无效": "The built-in rule trust anchor is invalid.",
  "所选文件没有可显示的文件名": "The selected file has no displayable filename.",
  "请选择一个论文文件，而不是文件夹": "Select a manuscript file, not a folder.",
  "当前仅支持 DOCX、PDF 和 TEX 格式": "Only DOCX, PDF, and TEX formats are currently supported.",
  "无法打开该文件。请确认文件可访问后重试。": "The file could not be opened. Check that it is accessible and try again.",
};

const BACKEND_PATTERNS: Array<[RegExp, (match: RegExpMatchArray) => string]> = [
  [/^无法读取所选文件路径：(.+)$/, (match) => `The selected file path could not be read: ${match[1]}`],
  [/^无法读取所选文件：(.+)$/, (match) => `The selected file could not be read: ${match[1]}`],
  [/^无法读取本地源快照：(.+)$/, (match) => `The local source snapshot could not be read: ${match[1]}`],
  [/^DOCX 结构无法解析：(.+)$/, (match) => `The DOCX structure could not be parsed: ${match[1]}`],
  [/^PDF 结构无法解析：(.+)$/, (match) => `The PDF structure could not be parsed: ${match[1]}`],
  [/^本地工作区写入失败：(.+)$/, (match) => `The local workspace could not be written: ${match[1]}`],
  [/^本地工作区记录无效：(.+)$/, (match) => `The local workspace record is invalid: ${match[1]}`],
  [/^无法定位本地应用数据目录：(.+)$/, (match) => `The local app-data directory could not be located: ${match[1]}`],
  [/^规则包 (.+) 的签名验证失败$/, (match) => `Signature verification failed for rule pack ${match[1]}`],
  [/^投稿规则包无效：(.+)$/, (match) => `The submission rule pack is invalid: ${match[1]}`],
  [/^工作区 (.+) 的标识不一致，已跳过$/, (match) => `Workspace ${match[1]} has a mismatched identifier and was skipped`],
  [/^工作区 (.+) 无法读取，已跳过$/, (match) => `Workspace ${match[1]} could not be read and was skipped`],
  [/^归档工作区 (.+) 的标识不一致，已跳过$/, (match) => `Archived workspace ${match[1]} has a mismatched identifier and was skipped`],
  [/^归档工作区 (.+) 无法读取，已跳过$/, (match) => `Archived workspace ${match[1]} could not be read and was skipped`],
  [/^未找到论文版本 v(.+)$/, (match) => `Manuscript version v${match[1]} was not found`],
  [/^该文件选择已失效，请重新选择修改稿$/, () => "This file selection has expired. Select the revised manuscript again."],
  [/^文件大小为 (.+) 字节，超过 (.+) 字节的本地处理上限$/, (match) => `The file is ${match[1]} bytes, above the local processing limit of ${match[2]} bytes`],
];

export function localizeBackendText(locale: Locale, value: string) {
  if (locale === "zh-CN") return value;
  const exact = BACKEND_ENGLISH[value];
  if (exact) return exact;
  for (const [pattern, translate] of BACKEND_PATTERNS) {
    const match = value.match(pattern);
    if (match) return translate(match);
  }
  return value;
}

function initialLocale(): Locale {
  if (typeof window === "undefined") return "zh-CN";
  return window.localStorage.getItem(STORAGE_KEY) === "en" ? "en" : "zh-CN";
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(initialLocale);
  const setLocale = useCallback((nextLocale: Locale) => {
    setLocaleState(nextLocale);
    window.localStorage.setItem(STORAGE_KEY, nextLocale);
  }, []);
  const text = useCallback((chinese: string, english: string) => localize(locale, chinese, english), [locale]);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  const value = useMemo(() => ({ locale, setLocale, text }), [locale, setLocale, text]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const value = useContext(I18nContext);
  if (!value) throw new Error("useI18n must be used inside I18nProvider");
  return value;
}
