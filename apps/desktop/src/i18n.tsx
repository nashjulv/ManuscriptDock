import type { ReactNode } from "react";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import { PRODUCT_VERSION } from "./version";

export type Locale = "zh-CN" | "en";
export interface LocalizedText {
  zhCn: string;
  en: string;
}
interface I18nValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  text: (zh: string, en: string) => string;
  localize: (value: LocalizedText) => string;
}
const I18nContext = createContext<I18nValue | null>(null);
const STORAGE_KEY = "manuscriptdock.locale";

export function localize(locale: Locale, chinese: string, english: string) {
  return locale === "zh-CN" ? chinese : english;
}
export function localizeBackendText(locale: Locale, value: string) {
  return (
    ERROR_MESSAGES[value]?.[locale === "zh-CN" ? 0 : 1] ??
    localize(
      locale,
      `操作未完成（${value}）`,
      `The operation could not be completed (${value}).`,
    )
  );
}

const ERROR_MESSAGES: Record<string, [string, string]> = {
  INPUT_UNREADABLE: [
    "无法读取所选文件，请重新选择。",
    "The selected file could not be read. Choose another file.",
  ],
  FORMAT_UNSUPPORTED: [
    "当前仅支持安全的 PDF 或 DOCX 文件。",
    "Only safe PDF or DOCX files are currently supported.",
  ],
  PDF_ENCRYPTED: [
    "所选 PDF 已加密或受密码保护，无法在本机读取。",
    "The selected PDF is encrypted or password-protected and cannot be read locally.",
  ],
  PDF_TEXT_EXTRACTION_FAILED: [
    "无法从所选 PDF 读取文本；可改用 DOCX，或检查 PDF 是否完整。",
    "Text could not be read from the selected PDF. Use a DOCX instead or check that the PDF is intact.",
  ],
  LIMIT_EXCEEDED: [
    "所选内容超过本地处理上限。",
    "The selected content exceeds the local processing limit.",
  ],
  FOLDER_MANUSCRIPT_REQUIRED: [
    "所选文件夹中没有 PDF 或 DOCX 主稿，请重新选择。",
    "The selected folder contains no PDF or DOCX manuscript. Choose another folder.",
  ],
  FOLDER_SELECTION_INVALID: [
    "无法确认所选项目文件夹，请重新选择。",
    "The selected project folder could not be confirmed. Choose it again.",
  ],
  PROJECT_FOLDER_UNAVAILABLE: [
    "原项目文件夹目前不可用，请确认文件夹仍在原位置且可以访问。",
    "The original project folder is unavailable. Check that it is still in place and accessible.",
  ],
  FOLDER_MANUSCRIPT_OUTSIDE: [
    "主稿必须位于所选项目文件夹的第一层，请重新选择。",
    "The manuscript must be directly inside the selected project folder. Choose it again.",
  ],
  FOLDER_BINDING_CONFLICT: [
    "项目文件夹绑定不一致，已停止操作以避免写入错误位置。",
    "The project-folder binding does not match. The operation stopped to avoid writing to the wrong location.",
  ],
  FOLDER_BINDING_INVALID: [
    "项目文件夹绑定记录无效，未读取或写入该文件夹。",
    "The project-folder binding is invalid. The folder was not read or written.",
  ],
  PROJECT_FOLDER_NOT_BOUND: [
    "此任务不是从项目文件夹打开的，请手动选择保存位置。",
    "This task was not opened from a project folder. Choose a save location manually.",
  ],
  DIALOG_UNAVAILABLE: [
    "系统文件选择器暂时不可用，请重试。",
    "The system file chooser is temporarily unavailable. Try again.",
  ],
  SELECTION_EXPIRED: [
    "文件选择已失效，请重新选择。",
    "The file selection expired. Choose the file again.",
  ],
  CONTEXT_CHANGED: [
    "内容已经变化，已为你重新读取最新状态。",
    "The content changed. The latest state has been reloaded.",
  ],
  RULES_UNVERIFIED: [
    "尚无已核验的期刊要求，可选择受支持期刊。",
    "No verified requirements are available for this journal. Choose a supported journal.",
  ],
  RULES_CHANGED: [
    "期刊规则版本已经变化，请重新选择目标后核对要求。",
    "The journal rule version changed. Reselect the target and review the requirements.",
  ],
  RESOURCE_BUNDLE_INVALID: [
    "内置期刊资料未通过完整性校验，已停止使用。",
    "The built-in journal resources failed integrity verification and were not used.",
  ],
  MATERIAL_REQUIRED: [
    "请先补齐必需事实或材料。",
    "Complete the required facts or materials first.",
  ],
  EDITABLE_MANUSCRIPT_REQUIRED: [
    "PDF 项目的正式投稿包需要作者核对的可编辑 DOCX；PDF 未被转换或改名。",
    "A final package for a PDF project needs an author-verified editable DOCX. The PDF was not converted or renamed.",
  ],
  RECENT_INDEX_INVALID: [
    "无法读取已移除任务列表；任何论文或生成文件均未被删除。",
    "The removed-task list could not be read. No manuscript or generated file was deleted.",
  ],
  MATERIAL_KIND_INVALID: [
    "附件用途无效，请重新选择用途。",
    "The attachment purpose is invalid. Choose a purpose again.",
  ],
  ANONYMITY_UNVERIFIED: [
    "匿名稿仍包含与作者姓名、单位或邮箱匹配的内容，请在外部编辑后重新选择。",
    "The anonymized manuscript still contains content matching an author name, affiliation, or email. Edit it externally and choose it again.",
  ],
  DOCX_GENERATION_FAILED: [
    "无法生成 Word 作者工具，请稍后重试。",
    "The Word author tool could not be generated. Try again.",
  ],
  XLSX_GENERATION_FAILED: [
    "无法生成 Excel 填写表，请稍后重试。",
    "The Excel submission sheet could not be generated. Try again.",
  ],
  OUTPUT_PERMISSION_DENIED: [
    "无法写入所选文件夹，请选择其他位置。",
    "The selected folder is not writable. Choose another location.",
  ],
  OUTPUT_ALREADY_EXISTS: [
    "目标位置已有同名输出，未覆盖任何文件。",
    "An output with the same name already exists. No files were overwritten.",
  ],
  PROJECT_NOT_FOUND: [
    "未找到该本地任务。",
    "This local task could not be found.",
  ],
  PROJECT_LOCK_UNAVAILABLE: [
    "本地任务正在被另一项操作更新，请稍后重试。",
    "Another operation is updating this local task. Try again shortly.",
  ],
  REQUEST_ID_INVALID: [
    "请求标识无效，请重新执行此操作。",
    "The request identifier is invalid. Start the operation again.",
  ],
  REQUEST_ID_CONFLICT: [
    "该请求标识已用于另一项操作，未重复执行。",
    "This request identifier belongs to another operation, so nothing was repeated.",
  ],
  REQUEST_RECORD_INVALID: [
    "无法读取本地请求记录，请重新打开任务后检查当前状态。",
    "The local request record could not be read. Reopen the task and check its current state.",
  ],
  JOB_NOT_FOUND: [
    "未找到该本地任务记录，请重新打开项目检查当前状态。",
    "This local job record could not be found. Reopen the project and check its current state.",
  ],
  JOB_RECORD_INVALID: [
    "本地任务记录损坏，未自动重做可能产生文件的操作。",
    "The local job record is invalid. An operation that may create files was not repeated automatically.",
  ],
  JOB_CANCELLED: [
    "该操作已取消，现有文件未被删除。",
    "This operation was cancelled. Existing files were not deleted.",
  ],
  JOB_INTERRUPTED: [
    "本地后台任务意外中断；请重新打开项目检查当前状态。",
    "The local background job was interrupted. Reopen the project and check its current state.",
  ],
  SOURCE_CHANGED_DURING_IMPORT: [
    "原稿在读取期间发生变化，未创建不完整副本；请重新选择。",
    "The manuscript changed while it was being read. No incomplete copy was created; choose it again.",
  ],
  SNAPSHOT_CHANGED: [
    "本地快照与记录的哈希不一致，已停止生成；请从原始文件新建任务。",
    "The local snapshot no longer matches its recorded hash. Generation stopped; create a new task from the original file.",
  ],
  INPUT_SYMLINK_NOT_ALLOWED: [
    "不能通过符号链接读取稿件或附件，请选择原始文件。",
    "Manuscripts and attachments cannot be read through symbolic links. Choose the original file.",
  ],
  PACKAGE_NOT_FOUND: [
    "未找到这次生成结果，请重新生成。",
    "This generated package could not be found. Build it again.",
  ],
};

function initialLocale(): Locale {
  if (typeof window === "undefined") return "zh-CN";
  const stored = window.localStorage.getItem(STORAGE_KEY);
  if (stored === "zh-CN" || stored === "en") return stored;
  return window.navigator.language.toLowerCase().startsWith("zh")
    ? "zh-CN"
    : "en";
}
export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(initialLocale);
  const setLocale = useCallback((next: Locale) => {
    setLocaleState(next);
    window.localStorage.setItem(STORAGE_KEY, next);
  }, []);
  const text = useCallback(
    (zh: string, en: string) => localize(locale, zh, en),
    [locale],
  );
  const pick = useCallback(
    (value: LocalizedText) => value[locale === "zh-CN" ? "zhCn" : "en"],
    [locale],
  );
  useEffect(() => {
    document.documentElement.lang = locale;
    document.title = `投稿舱 ManuscriptDock ${PRODUCT_VERSION}`;
    document
      .querySelector('meta[name="description"]')
      ?.setAttribute(
        "content",
        text(
          "本地期刊匹配与投稿包整理",
          "Local journal matching and submission-package preparation",
        ),
      );
  }, [locale, text]);
  const value = useMemo(
    () => ({ locale, setLocale, text, localize: pick }),
    [locale, setLocale, text, pick],
  );
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}
export function useI18n() {
  const value = useContext(I18nContext);
  if (!value) throw new Error("useI18n must be used inside I18nProvider");
  return value;
}
