import type { ReactNode } from "react";
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import { PRODUCT_VERSION } from "./version";

export type Locale = "zh-CN" | "en";

interface I18nValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  text: (chinese: string, english: string) => string;
}

const STORAGE_KEY = "manuscriptdock.locale";
const PRODUCT_TITLE = `投稿舱 ManuscriptDock ${PRODUCT_VERSION}`;
const PAGE_DESCRIPTIONS: Record<Locale, string> = {
  "zh-CN": "ManuscriptDock 投稿舱：本地优先的论文投稿准备工作台",
  en: "ManuscriptDock: a local-first manuscript submission workspace",
};
const I18nContext = createContext<I18nValue | null>(null);

export function localize(locale: Locale, chinese: string, english: string) {
  return locale === "zh-CN" ? chinese : english;
}

const BACKEND_ENGLISH: Record<string, string> = {
  "自动抽取只建立带来源的准备清单，不替代作者对官网原文的最终核对": "Automatic extraction creates a source-backed preparation checklist; the author must still verify the official text.",
  "已保存官方页面指纹，但未识别到明确投稿条目；请粘贴作者指南原文": "The page fingerprint was saved, but no explicit submission requirements were identified. Paste the author-guide text.",
  "部分来源由作者确认，域名未与期刊主页自动匹配": "Some sources were confirmed by the author; their domains did not automatically match the journal homepage.",
  "来源为作者确认的 HTTP 官方页面；系统未联网读取，传输安全性和原文真实性需作者复核": "The author supplied an HTTP official-page source. The app did not fetch it; the author must verify transport security and the original text.",
  "未检测到 \\title{}": "No \\title{} declaration was detected",
  "未检测到 \\author{}": "No \\author{} declaration was detected",
  "未检测到 Word 标题样式": "No Word title style was detected",
  "未检测到 Word 作者样式或可靠的首页作者行": "No Word author style or reliable first-page author line was detected",
  "已使用增强字体映射读取 PDF 文本层；多栏顺序、公式和复杂版式仍需人工确认": "The PDF text layer was read with enhanced font mapping; columns, formulas, and complex layouts still need manual confirmation",
  "已使用布局感知 PDF 解析：按字体、坐标和分栏顺序规整文本；公式与复杂跨页表格仍需人工确认": "Layout-aware PDF parsing normalized text using fonts, coordinates, and column order; formulas and complex cross-page tables still need manual confirmation",
  "检测到 PDF 字体编码异常；已保留可靠页面，异常页面需 OCR 或作者确认": "PDF font-encoding issues were detected; reliable pages were retained, while affected pages require OCR or author confirmation",
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
  "该文件选择已失效，请重新选择修改稿": "This file selection has expired. Select the revised manuscript again.",
  "本地工作区标识无效": "The local workspace identifier is invalid.",
  "未找到需要管理的本地工作区": "The local workspace to manage was not found.",
  "目标位置已存在同一工作区，未移动任何文件": "The destination already contains this workspace; no files were moved.",
  "当前填写的是 DeepSeek 文档地址；API 地址应为 https://api.deepseek.com": "This is the DeepSeek documentation URL; use https://api.deepseek.com as the API base URL.",
  "导入期间源稿件发生变化，请重新选择后再试": "The source manuscript changed during import. Select it again and retry.",
  "所选稿件与当前版本内容完全一致，未创建重复版本": "The selected manuscript is identical to the current version, so no duplicate version was created.",
  "新版本必须与当前稿件保持相同文件类型；格式转换应作为投稿输出保存": "A new version must use the same file type as the current manuscript; save format conversions as submission outputs.",
  "版本说明不能超过 200 个字符": "The version note cannot exceed 200 characters.",
  "系统时间无效，无法创建审计记录": "The system time is invalid, so an audit record could not be created.",
  "系统时间早于 Unix 纪元": "The system clock is earlier than the Unix epoch.",
  "内置规则信任锚无效": "The built-in rule trust anchor is invalid.",
  "所选文件没有可显示的文件名": "The selected file has no displayable filename.",
  "请选择一个论文文件，而不是文件夹": "Select a manuscript file, not a folder.",
  "当前仅支持 DOCX、PDF 和 TEX 格式": "Only DOCX, PDF, and TEX formats are currently supported.",
  "无法打开该文件。请确认文件可访问后重试。": "The file could not be opened. Check that it is accessible and try again.",
  "最近的本地工作区暂时无法读取": "Recent local workspaces could not be loaded.",
  "TEX 文件不是有效的 UTF-8 文本": "The TEX file is not valid UTF-8 text.",
  "PDF 仅提供只读证据；请使用 DOCX 或 TEX 源稿进行结构化修订": "PDF is read-only evidence; use a DOCX or TEX source for structured revision.",
  "PDF 保持只读；请提供 DOCX 或 TEX 源稿进行结构化修订": "PDF remains read-only; provide a DOCX or TEX source for structured revision.",
  "DOCX 首轮仅安全回写使用 Title 样式的标题；摘要和关键词继续保留为只读证据": "The first DOCX revision pass can safely write only a title using the Title style; abstract and keywords remain read-only evidence.",
  "没有检测到字段变化，未创建重复版本": "No field changes were detected, so no duplicate version was created.",
  "修改后内容与当前版本相同，未创建重复版本": "The revised content matches the current version, so no duplicate version was created.",
  "请完整填写姓名、学校、专业、论文用途和有效的未来投稿截止日期": "Complete the name, institution, specialty, manuscript purpose, and a valid future submission deadline.",
  "投稿背景档案不完整或字段格式无效": "The submission context profile is incomplete or contains invalid fields.",
  "学校要求抽取结果缺少可追溯来源、有效规则版本或合法的分区条件": "The extracted institution policy lacks traceable sources, a valid rule version, or valid partition conditions.",
  "未找到已保存的投稿背景档案，请先保存后再计算推荐": "No saved submission context profile was found. Save it before calculating recommendations.",
  "当前论文版本尚未完成投稿检查，请先重新检查": "The current manuscript version has not completed submission checks. Run the checks again.",
  "需要作者明确确认后才能创建记录": "Explicit author confirmation is required before creating this record.",
  "投稿目标不能为空，且不能超过 200 个字符": "The submission target is required and cannot exceed 200 characters.",
  "当前论文版本尚未完成本地存证": "The current manuscript version has not completed local attestation.",
  "当前论文版本尚未登记投稿记录": "The current manuscript version has no recorded submission.",
  "请选择有效的学科索引分类后再固化知识体": "Choose a valid discipline classification before finalizing the knowledge body.",
  "当前论文版本尚未固化知识体，不能建立问答记录": "The current manuscript version has not finalized a knowledge body, so a dialogue record cannot be created.",
  "知识体问题不能为空，且不能超过 4000 个字符": "The knowledge-body question is required and cannot exceed 4,000 characters.",
  "未找到当前知识体对应的问题记录": "No question record was found for the current knowledge body.",
  "模型回答、模型名称和提供方不能为空，且长度必须在限制内": "The model answer, model name, and provider are required and must remain within their length limits.",
  "请选择可写入的导出文件夹": "Choose a writable export folder.",
  "目标文件夹中已存在同名投稿包，未覆盖任何文件": "A submission package with the same name already exists in the destination; no files were overwritten.",
  "知识体快照版本无效": "The knowledge-body snapshot version is invalid.",
  "AI 审核报告版本链无效": "The AI review report version chain is invalid.",
  "知识体快照引用的 AI 审核报告版本不存在": "The knowledge-body snapshot references an AI review report version that does not exist.",
  "知识体五部分服务架构或能力契约无效": "The five-part knowledge-body service architecture or capability contract is invalid.",
  "关联知识体声明缺少成立依据或协议类型无效": "A related knowledge-body assertion lacks a valid basis or protocol type.",
  "知识候选审核必须逐条选择纳入或排除，且必须对应当前论文分解": "Every knowledge candidate must be included or excluded and must belong to the current manuscript decomposition.",
  "尚未配置可用模型，请点击应用顶部的“模型设置”保存主模型或备选模型": "No model is configured. Use Models in the product bar to save a primary or fallback model.",
  "达到输出上限，但没有形成最终回答": "The output limit was reached before a final answer was produced.",
  "已完成内部推理，但没有形成最终回答": "Internal reasoning completed without producing a final answer.",
  "返回了空回答": "The model returned an empty answer.",
  "请求格式未被模型服务接受，请检查模型兼容性": "The model service rejected the request format. Check model compatibility.",
  "API Key 无效或无权调用该模型，请检查提供方权限": "The API key is invalid or lacks access to this model. Check provider permissions.",
  "账户余额不足或计费未开通，请检查模型提供方账户余额": "The account has insufficient balance or billing is not enabled. Check the model-provider account.",
  "未找到对话接口，请检查 API 地址": "The chat endpoint was not found. Check the API base URL.",
  "请求参数未被模型服务接受，请检查模型名称和接口兼容性": "The model service rejected the request parameters. Check the model name and API compatibility.",
  "请求频率或账户限额已达到上限，请稍后重试或使用备选模型": "The request or account limit has been reached. Retry later or use a fallback model.",
  "模型服务暂时故障或繁忙，请稍后重试或使用备选模型": "The model service is temporarily unavailable or busy. Retry later or use a fallback model.",
  "模型设置必须包含 1 个主模型和 2 个备选模型": "Model settings must contain one primary and two fallback slots.",
  "模型设置槽位重复或缺失": "Model-setting slots are duplicated or missing.",
  "模型设置字段超过长度限制": "A model-setting field exceeds its length limit.",
  "模型 API 地址无效": "The model API base URL is invalid.",
  "模型 API 地址不能包含账号、密码、查询参数或片段": "The model API base URL cannot contain credentials, query parameters, or a fragment.",
  "远程模型 API 必须使用 HTTPS；只有本机 localhost 可以使用 HTTP": "Remote model APIs must use HTTPS; only localhost may use HTTP.",
  "无法生成模型问答地址": "The model chat endpoint could not be constructed.",
  "模型设置文件格式无效": "The model settings file is invalid.",
  "模型设置版本暂不受支持": "This model-settings version is not supported.",
  "需要作者确认本次模型外发后才能提问": "Author confirmation is required before sending this model request.",
  "当前论文版本尚未固化知识体": "The current manuscript version has not finalized a knowledge body.",
  "需要作者确认学校名称、学科、论文用途和脱敏规则原文的本次模型外发": "Author confirmation is required before sending the institution, discipline, manuscript purpose, and redacted policy text.",
  "请粘贴 40–30000 字符的学校正式要求原文": "Paste 40–30,000 characters from the official institution policy.",
  "学校要求来源必须是有效的 HTTPS 官方页面": "The institution-policy source must be a valid official HTTPS page.",
  "官方来源网址无效": "The official-source URL is invalid.",
  "官方来源网址只支持 HTTP 或 HTTPS": "The official-source URL must use HTTP or HTTPS.",
  "官方来源网址不能包含用户名或密码": "The official-source URL cannot contain a username or password.",
  "该期刊官方来源仅提供 HTTP，无法安全自动读取；请粘贴官方作者指南原文": "This journal source only provides HTTP and cannot be fetched securely. Paste the official author-guide text instead.",
  "期刊要求必须来自有效的官方网页地址，并包含可核对的原文": "Journal requirements must include a valid official web source and verifiable source text.",
  "所提供原文未包含适用于当前学校、专业和论文用途的明确投稿要求": "The supplied text contains no explicit submission requirement applicable to the current institution, specialty, and manuscript purpose.",
  "模型未返回学校要求 JSON 对象": "The model did not return an institution-policy JSON object.",
  "模型返回的学校要求 JSON 不完整": "The institution-policy JSON returned by the model is incomplete.",
  "模型返回的学校要求结构无法校验，请重试或更换模型": "The institution-policy structure returned by the model could not be validated. Retry or use another model.",
};

const BACKEND_PATTERNS: Array<[RegExp, (match: RegExpMatchArray) => string]> = [
  [/^无法读取所选文件路径：(.+)$/, (match) => `The selected file path could not be read: ${systemDetail(match[1])}`],
  [/^无法读取所选文件：(.+)$/, (match) => `The selected file could not be read: ${systemDetail(match[1])}`],
  [/^无法读取本地源快照：(.+)$/, (match) => `The local source snapshot could not be read: ${systemDetail(match[1])}`],
  [/^无法读取导出文件夹：(.+)$/, (match) => `The export folder could not be read: ${systemDetail(match[1])}`],
  [/^无法定位本地应用数据目录：(.+)$/, (match) => `The local app-data directory could not be located: ${systemDetail(match[1])}`],
  [/^无法定位模型设置目录：(.+)$/, (match) => `The model-settings directory could not be located: ${systemDetail(match[1])}`],
  [/^无法创建模型设置目录：(.+)$/, (match) => `The model-settings directory could not be created: ${systemDetail(match[1])}`],
  [/^无法读取模型设置：(.+)$/, (match) => `Model settings could not be read: ${systemDetail(match[1])}`],
  [/^无法编码模型设置：(.+)$/, (match) => `Model settings could not be encoded: ${systemDetail(match[1])}`],
  [/^无法写入模型设置：(.+)$/, (match) => `Model settings could not be written: ${systemDetail(match[1])}`],
  [/^无法提交模型设置：(.+)$/, (match) => `Model settings could not be committed: ${systemDetail(match[1])}`],
  [/^无法初始化模型连接：(.+)$/, (match) => `The model connection could not be initialized: ${systemDetail(match[1])}`],
  [/^无法从系统凭据库删除 API Key：(.+)$/, (match) => `The API key could not be deleted from the system credential store: ${systemDetail(match[1])}`],
  [/^无法将 API Key 保存到系统凭据库：(.+)$/, (match) => `The API key could not be saved to the system credential store: ${systemDetail(match[1])}`],
  [/^无法从系统凭据库读取 (.+) API Key：(.+)$/, (match) => `The ${match[1]} API key could not be read from the system credential store: ${systemDetail(match[2])}`],
  [/^系统凭据库不可用：(.+)$/, (match) => `The system credential store is unavailable: ${systemDetail(match[1])}`],
  [/^无法读取系统凭据库状态：(.+)$/, (match) => `The system credential-store status could not be read: ${systemDetail(match[1])}`],
  [/^无法生成最小知识体投影：(.+)$/, (match) => `The minimal knowledge-body projection could not be generated: ${systemDetail(match[1])}`],
  [/^无法生成学校要求最小投影：(.+)$/, (match) => `The minimal institution-policy projection could not be generated: ${systemDetail(match[1])}`],
  [/^DOCX 结构无法解析：(.+)$/, (match) => `The DOCX structure could not be parsed: ${localizeBackendText("en", match[1])}`],
  [/^PDF 结构无法解析：(.+)$/, (match) => `The PDF structure could not be parsed: ${localizeBackendText("en", match[1])}`],
  [/^无法生成本地修订稿：(.+)$/, (match) => `The local revision could not be generated: ${systemDetail(match[1])}`],
  [/^当前稿件中无法安全定位修订字段 (.+)$/, (match) => `The revision field ${match[1]} could not be located safely in the current manuscript.`],
  [/^修订字段 (.+) 不能为空或超过 20000 个字符$/, (match) => `The revision field ${match[1]} is empty or exceeds 20,000 characters.`],
  [/^DOCX 无法安全修订：(.+)$/, (match) => `The DOCX file cannot be revised safely: ${localizeBackendText("en", match[1])}`],
  [/^检测到 (\d+) 个页面需要 OCR 或字体解码复核：(.+)；当前版本未执行 OCR$/, (match) => `${match[1]} pages require OCR or font-decoding review: ${match[2]}. OCR was not run in this version.`],
  [/^PDF 文档分类：(.+)（置信度 (.+)%）；已优先执行原生结构提取$/, (match) => `PDF classification: ${match[1]} (${match[2]}% confidence). Native structure extraction was run first.`],
  [/^PDF 文档分类：(.+)；已优先执行原生结构提取$/, (match) => `PDF classification: ${match[1]}. Native structure extraction was run first.`],
  [/^原生表格候选页：(.+)；已优先保留版面结构，不使用文本 OCR 覆盖$/, (match) => `Native table candidates were detected on pages ${match[1]}; layout structure was retained and will not be overwritten by text OCR.`],
  [/^多栏版面候选页：(.+)；已按坐标重排阅读顺序$/, (match) => `Multi-column layout was detected on pages ${match[1]}; reading order was rearranged using coordinates.`],
  [/^本地工作区写入失败：(.+)$/, (match) => `The local workspace could not be written: ${systemDetail(match[1])}`],
  [/^本地工作区记录无效：(.+)$/, (match) => `The local workspace record is invalid: ${localizeBackendText("en", match[1])}`],
  [/^规则包 (.+) 的签名验证失败$/, (match) => `Signature verification failed for rule pack ${match[1]}`],
  [/^投稿规则包无效：(.+)$/, (match) => `The submission rule pack is invalid: ${localizeBackendText("en", match[1])}`],
  [/^工作区 (.+) 的标识不一致，已跳过$/, (match) => `Workspace ${match[1]} has a mismatched identifier and was skipped`],
  [/^工作区 (.+) 无法读取，已跳过$/, (match) => `Workspace ${match[1]} could not be read and was skipped`],
  [/^归档工作区 (.+) 的标识不一致，已跳过$/, (match) => `Archived workspace ${match[1]} has a mismatched identifier and was skipped`],
  [/^归档工作区 (.+) 无法读取，已跳过$/, (match) => `Archived workspace ${match[1]} could not be read and was skipped`],
  [/^(.+) 已启用，但提供方、地址或模型名称不完整$/, (match) => `${match[1]} is enabled but its provider, URL, or model name is incomplete.`],
  [/^(.+) 已启用，但尚未提供 API Key；请输入 Key 后再保存$/, (match) => `${match[1]} is enabled but has no API key. Enter a key before saving.`],
  [/^主模型和备选模型均未完成回答：(.+)$/, (match) => `No configured model completed the answer: ${localizeBackendText("en", match[1])}`],
  [/^(primary|fallback_1|fallback_2) 返回了无法识别的响应$/, (match) => `${match[1]} returned an unrecognized response.`],
  [/^(primary|fallback_1|fallback_2) 连接超时$/, (match) => `${match[1]} timed out.`],
  [/^(primary|fallback_1|fallback_2) 连接失败$/, (match) => `${match[1]} could not connect.`],
  [/^(primary|fallback_1|fallback_2) 返回 HTTP (\d+)$/, (match) => `${match[1]} returned HTTP ${match[2]}.`],
  [/^未找到论文版本 v(.+)$/, (match) => `Manuscript version v${match[1]} was not found`],
  [/^文件大小为 (.+) 字节，超过 (.+) 字节的本地处理上限$/, (match) => `The file is ${match[1]} bytes, above the local processing limit of ${match[2]} bytes`],
  [/^主题范围适配 (\d+) 分$/, (match) => `Topic-scope fit: ${match[1]}`],
  [/^作者专业背景适配 (\d+) 分$/, (match) => `Author-specialty fit: ${match[1]}`],
  [/^论文用途适配 (\d+) 分$/, (match) => `Manuscript-purpose fit: ${match[1]}`],
  [/^投稿准备时间适配 (\d+) 分（内部规划 (\d+) 天）$/, (match) => `Submission-preparation timing fit: ${match[1]} (${match[2]}-day internal plan)`],
  [/^当前稿件完备度适配 (\d+) 分；当前版本结构完备度 (\d+)，达到该层级的投稿准备门槛 (\d+)$/, (match) => `Current-manuscript readiness fit: ${match[1]}; structural readiness ${match[2]} meets the tier threshold of ${match[3]}`],
  [/^当前稿件完备度适配 (\d+) 分；当前版本结构完备度 (\d+)，距离该层级的投稿准备门槛还差 (\d+)$/, (match) => `Current-manuscript readiness fit: ${match[1]}; structural readiness ${match[2]} is ${match[3]} points below the tier threshold`],
  [/^目标策略适配 (\d+) 分$/, (match) => `Target-strategy fit: ${match[1]}`],
];

const SOURCE_LABEL_ENGLISH: Record<string, string> = {
  "摘要 / Abstract": "Abstract",
  "LaTeX 正文": "LaTeX body",
  "LaTeX 表格环境": "LaTeX table environment",
  "LaTeX 图片环境": "LaTeX figure environment",
  "提取文本": "Extracted text",
  "首页作者单位": "First-page author affiliation",
  "首页通讯信息": "First-page contact information",
  "PDF 首页": "PDF first page",
};

const SOURCE_LABEL_PATTERNS: Array<[RegExp, (match: RegExpMatchArray) => string]> = [
  [/^PDF Markdown · 行 (\d+)$/, (match) => `PDF Markdown · Line ${match[1]}`],
  [/^Word 段落 (\d+)$/, (match) => `Word paragraph ${match[1]}`],
  [/^Word (.+) · 段落 (\d+)$/, (match) => `Word ${match[1]} · Paragraph ${match[2]}`],
  [/^(.+) · 片段 (\d+)$/, (match) => `${localizeSourceLabel("en", match[1])} · Fragment ${match[2]}`],
];

function containsChinese(value: string) {
  return /[\u3400-\u9fff]/u.test(value);
}

function systemDetail(value: string) {
  return containsChinese(value) ? "See the local audit record for system details." : value;
}

export const OFFICIAL_SOURCE_MESSAGES: Record<string, [string, string]> = {
  OFFICIAL_INVALID_URL: ["请输入有效的 HTTP 或 HTTPS 来源网址。", "Enter a valid HTTP or HTTPS source URL."],
  OFFICIAL_CREDENTIALS: ["来源网址不能包含用户名或密码。", "Source URLs cannot contain a username or password."],
  OFFICIAL_PORT_BLOCKED: ["公开页面读取仅支持标准 HTTP／HTTPS 端口，请粘贴官方原文。", "Public-page reading supports standard HTTP/HTTPS ports only. Paste official text instead."],
  OFFICIAL_PRIVATE_ADDRESS: ["目标地址指向本机、私网或非公网范围，已停止访问。", "The destination resolves to a local, private, or non-public address. Access was stopped."],
  OFFICIAL_DNS_FAILED: ["无法解析官网域名，请稍后重试或粘贴官方原文。", "The journal domain could not be resolved. Retry later or paste official text."],
  OFFICIAL_TIMEOUT: ["官方页面读取超时，请重试或粘贴原文。", "The official-page request timed out. Retry or paste the source text."],
  OFFICIAL_TLS_FAILED: ["HTTPS 证书验证失败，未绕过证书校验。", "HTTPS certificate validation failed. Certificate checks were not bypassed."],
  OFFICIAL_CONNECTION_FAILED: ["无法连接官方页面；这不代表网站仅支持 HTTP。", "The official page could not be reached; this does not establish that the site supports only HTTP."],
  OFFICIAL_CLIENT_FAILED: ["无法建立受控页面读取客户端。", "The controlled page reader could not be initialized."],
  OFFICIAL_ORIGIN_CONFIRMATION: ["发现另一域名，访问前需确认其属于官方来源。", "Another domain was found. Confirm it is an official source before accessing it."],
  OFFICIAL_HTTP_CONFIRMATION: ["HTTP 访问尚未授权，需要本次明确确认。", "HTTP access is not authorized. Explicit confirmation is required for this request."],
  OFFICIAL_REDIRECT_LIMIT: ["页面循环跳转或超过跳转上限，已停止访问。", "The page redirected in a loop or exceeded the redirect limit. Access was stopped."],
  OFFICIAL_REQUEST_LIMIT: ["本次访问已达到请求数量上限，请粘贴官方原文。", "The request limit was reached. Paste official text instead."],
  OFFICIAL_BAD_REDIRECT: ["官方页面返回了无效跳转地址。", "The official page returned an invalid redirect URL."],
  OFFICIAL_HTTP_STATUS: ["官方页面返回 HTTP 状态码", "The official page returned HTTP status"],
  OFFICIAL_TOO_LARGE: ["页面超过 2 MB 上限，请粘贴相关官方原文。", "The page exceeds the 2 MB limit. Paste the relevant official text."],
  OFFICIAL_UNSUPPORTED_FORMAT: ["该来源不是支持的网页或纯文本，PDF 等来源请粘贴原文。", "This source is not a supported web or plain-text page. Paste text from PDF or other sources."],
  OFFICIAL_NO_TEXT: ["页面没有可读取正文，可能依赖脚本，请粘贴原文。", "No readable page text was found. The page may require scripts; paste the source text."],
  OFFICIAL_GUIDE_NOT_FOUND: ["已读取主页，但未取得可用的作者指南，请粘贴官方指南原文。", "The homepage was read, but no usable author guide was captured. Paste the official guide text."],
  OFFICIAL_ENCODING_FAILED: ["无法可靠解码页面文字，请粘贴官方原文。", "The page text could not be decoded reliably. Paste official text."],
  OFFICIAL_DYNAMIC_UNAVAILABLE: ["动态正文未能读取，当前结果仍需人工补充。", "Dynamic text could not be read. Manual input is still required."],
  OFFICIAL_REQUESTED: ["已发起受控读取", "Controlled request started"],
  OFFICIAL_RECEIVED: ["已收到页面响应，正文仍需解析", "Page response received; text still needs parsing"],
  OFFICIAL_REDIRECT: ["页面返回重定向", "The page returned a redirect"],
  OFFICIAL_CAPTURED: ["页面正文已读取", "Page text captured"],
  OFFICIAL_PARTIAL_CAPTURE: ["部分官方来源未能读取；当前快照需要补充或重新获取。", "Some official sources could not be read. Supplement or refresh this snapshot."],
  OFFICIAL_HTTP_EVIDENCE: ["部分证据经作者授权通过未加密 HTTP 获取，原文真实性需复核。", "Some evidence was fetched over unencrypted HTTP with author authorization. Verify the source's authenticity."],
  OFFICIAL_CONSENT_REQUIRED: ["访问官方页面前需要作者本次明确授权。", "Explicit author authorization is required before accessing official pages."],
  OFFICIAL_AUDIT_FAILED: ["无法保存或读取访问记录，请检查本地存储后重试。", "The access record could not be saved or read. Check local storage and retry."],
};

export function localizeBackendText(locale: Locale, value: string) {
  const official = OFFICIAL_SOURCE_MESSAGES[value];
  if (official) return official[locale === "zh-CN" ? 0 : 1];
  if (locale === "zh-CN" || value.trim() === "") return value;
  const exact = BACKEND_ENGLISH[value];
  if (exact) return exact;
  for (const [pattern, translate] of BACKEND_PATTERNS) {
    const match = value.match(pattern);
    if (match) return translate(match);
  }
  return containsChinese(value)
    ? "The operation could not be completed. Switch to Chinese for the original system detail, then retry or review the local audit record."
    : value;
}

export function localizeSourceLabel(locale: Locale, value: string) {
  if (locale === "zh-CN" || value.trim() === "") return value;
  const exact = SOURCE_LABEL_ENGLISH[value];
  if (exact) return exact;
  for (const [pattern, translate] of SOURCE_LABEL_PATTERNS) {
    const match = value.match(pattern);
    if (match) return translate(match);
  }
  return value;
}

function initialLocale(): Locale {
  if (typeof window === "undefined") return "zh-CN";
  const stored = window.localStorage.getItem(STORAGE_KEY);
  if (stored === "zh-CN" || stored === "en") return stored;
  const preferred = window.navigator.languages?.[0] ?? window.navigator.language;
  return preferred?.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
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
    document.documentElement.dir = "ltr";
    document.title = PRODUCT_TITLE;
    document.querySelector('meta[name="description"]')?.setAttribute("content", PAGE_DESCRIPTIONS[locale]);
  }, [locale]);

  const value = useMemo(() => ({ locale, setLocale, text }), [locale, setLocale, text]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const value = useContext(I18nContext);
  if (!value) throw new Error("useI18n must be used inside I18nProvider");
  return value;
}
