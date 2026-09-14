# 领域合同、IPC 与状态

- 状态：部分实现。表格保留完整目标合同；V0.54 已注册命令与尚未实现的差异在本页明确列出。
- 关联：[架构](architecture.md)、[整理器](submission-compiler.md)、[验收](validation.md)。

## 1. 核心对象

| 对象 | 核心字段 | 不变量 |
| --- | --- | --- |
| Project | id、schemaVersion、revision、activeSourceVersion、activeTargetRevision、lastTask | UUID；显示名与路径分离；来源不可变 |
| SourceSnapshot | id、sha256、format、size、createdAt | 文件存在且哈希匹配才进入处理 |
| DocumentFacts | sourceHash、extractorVersion、fieldValues、coverage、authorCorrections | 提取值／作者值分开；每字段带证据与确认状态 |
| AuthorConstraints | revision、goal、institutionRules、budget、time、publicationMode、extraFilters | 每项含 required/preferred；未知不是 false |
| JournalRecord | id、issn、eissn、name、publisher、sourceFacts、ruleRefs、coverage | 名称不是主键；可被目录直接选择 |
| TargetSelection | id、journalId、articleType、stage、origin、recommendationRef?、rulesHash | origin=catalog 或 recommendation；推荐引用可空 |
| Material | id、hash、kind、included、anonymityCheck? | 绑定具体用途；匿名检查绑定当前身份事实哈希，不用一个附件自动满足多个独立要求 |
| AuthorDecision | requirementId、value、evidenceRef、contextHash、confirmedAt | 已阅读不等于事实符合，未知不伪装成确认 |
| PreparationView | contextHash、blockers、warnings、missingItems、readyItems、allowedActions | 由 Rust 单次投影，不由 UI 拼多个布尔值 |
| PackagePlan | id、contextHash、filePlan、transformPlan、capabilities、status | 每文件明确用途、生成方式、输入、输出位置 |
| CompiledPackage | id、planHash、files、validation、previewManifest | 保存成功的真实产物，有内容哈希 |
| ExportReceipt | id、packageHash、kind、destinationRef、fileManifest、finishedAt | 写入校验完成后记录，不是网站投稿凭证 |

值缺失统一用显式状态 `known / unknown / not_applicable`，不使用 0、空字符串或 false 混淆三者。适用性未知的必需规则保持待补充；不适用需有条件判断或作者给出的有效依据。

DTO 仅包含安全显示字段。不得发送原始机构工作簿、系统凭据、绝对路径或内部全部目录到前端。期刊依据可以安全投影，不因隐藏内部算法而隐藏推荐原因。

## 2. 目标命令表

所有修改命令携带 `requestId` 与 `expectedRevision`；Rust 负责鉴权、上下文检查和幂等。表中选择 token 均由原生对话框创建，不接受前端任意构造路径。

| 命令 | 输入要点 | 返回与行为 |
| --- | --- | --- |
| choose_local_inputs | kind=manuscript/editable_manuscript/material/folder | PDF／DOCX 主稿、仅 DOCX 可编辑主稿、普通附件或文件夹的 selected(tokens, metadata)，也可 cancelled |
| open_project | manuscriptToken、optional folderToken | projectId、jobId；提交不可变快照并启动解析；folderToken 相同则复用同一项目记录 |
| get_project_view | projectId | 当前任务、revision、输入摘要及 PreparationView |
| list_recent_projects | cursor? | 可恢复记录，无无关历史正文 |
| list_removed_projects / hide_recent_project / restore_recent_project | projectId、修改命令含 requestId | 仅维护可恢复隐藏索引；不删除项目或导出文件 |
| add_materials | projectId、tokens、expectedRevision | 原子加载清单；分类未知等待作者选择 |
| update_document_facts | field patches、sourceHash、revision | 保存作者纠正，不覆盖源稿，自动重查 |
| save_constraints | projectId、constraints、revision | 新约束版本，旧推荐保留但不再是当前 |
| list_journals | query、filters、cursor? | 本地目录搜索，已知目标可直接选择 |
| recommend_journals | projectId、constraintsRevision | jobId；结果绑定论文、条件和目录版本 |
| select_target | projectId、journalId、articleType、origin、recommendationRef? | 新目标；origin=catalog 不要求 recommendationRef |
| get_preparation | projectId | 当前缺项、依据、作者输入和可用操作 |
| save_preparation_inputs | facts、decisions、materialBindings、revision | 局部更新后自动检查，返回 jobId 或就绪视图 |
| build_package | projectId、contextHash、mode=draft/final | jobId；重新验证而非信任 UI 的 ready 标识 |
| get_generated_preview | projectId、packageId、fileId | 受控本地预览引用和真实格式，不返回任意路径 |
| choose_export_folder | 无路径参数 | 短期 destinationToken 或 cancelled |
| choose_project_folder_for_export | projectId | Rust 校验持久化文件夹绑定后返回短期 destinationToken；不向前端暴露路径 |
| export_package | packageId、contextHash、destinationToken、kind | jobId；正式 publisher ZIP 与 draft/archive 类型显式分离 |
| show_generated_file | receiptId 或 packageId/fileId | 只打开已授权生成物的系统位置 |
| cancel_job / get_job | optional projectId、jobId | 创建任务时尚无 projectId 的操作可按不可猜测的 jobId 查询／取消；已有项目时同时校验 projectId；取消不删除既有输出 |
| remove_project | projectId、明确删除动作 | 仅删应用管理副本，外部原稿及导出物不动 |

未上线的早期任务读取和迁移服务已移除，不再注册其扫描或导入命令；知识体问答、存证或备选转投命令同样不在新运行时中。目录未知 journal ID 不能绕过规则校验，通用整理需明确 generic target 状态。

V0.54 当前实际注册：`choose_local_inputs`、`open_project`、`get_project_view`、`list_recent_projects`、`list_removed_projects`、`hide_recent_project`、`restore_recent_project`、`list_journals`、`recommend_journals`、`select_target`、`get_preparation`、`update_document_facts`、`add_material`、`build_package`、`choose_export_folder`、`choose_project_folder_for_export`、`export_package`、`get_job`、`cancel_job` 及两个界面设置命令。修改命令均携带 UUID requestId；涉及项目修改的命令另带 expectedRevision。上述修改命令会立即返回持久化 job，业务结果保存在其 `result` 字段；前端监听 `manuscriptdock://job`，丢事件时查询 job。当前没有单独的 `save_constraints`、`save_preparation_inputs`、`get_generated_preview`、`show_generated_file` 或永久删除用 `remove_project`；现有事实／决定保存、生成结果、可恢复列表隐藏和系统导出路径已覆盖核心界面，但不能把缺少的命令宣称为已注册。

## 3. 工作任务状态

```text
queued → running → succeeded
                 → needs_input
                 → failed
                 → cancelled
```

事件字段：job 上包含 `id, projectId?, requestId, contextHash?, status, result?`，每条事件包含 `seq, phase, completedUnits?, totalUnits?, status, error?`。进度未知时用阶段文案，不伪造百分比。重启把未完成任务标为 interrupted，重新读取快照并由作者继续；不静默重复导出。事件丢失时 `get_job`／`get_project_view` 可恢复。取消会阻止该 job 写入 succeeded 终态；当前内部文件循环尚未布置细粒度检查点，因此不能承诺每个解析／生成步骤都即时停下。

每个项目只有一个当前可发布编译结果；旧任务即使晚到也只能保存为历史，不能替换新目标结果。UI 根据 jobId、seq 和 revision 丢弃过期视图。重复 requestId 返回同一结果；同一次导出的不确定结果必须先查询回执，不能盲重试制造多个输出。

## 4. 三种作者可见状态

| 状态 | 条件 | 主要操作 |
| --- | --- | --- |
| 待补充 | 缺必需材料、事实或关键要求证据 | 补齐当前项；可保存进度 |
| 草稿可预览 | 部分生成可完成，仍有未确认或不支持项 | 预览／保存明确标识的草稿 |
| 可以导出 | 完整目标覆盖内的必需项已确认、变换及输出检查通过 | 保存投稿包 |

“保存进度”自动发生，不设置独立必点按钮。草稿可以保留占位提示，正式文件与正式 ZIP 禁止遗留占位符。完成生成不等于可以导出；只有当前 CompiledPackage 验证通过才启用正式保存。

## 5. 上下文与失效矩阵

`contextHash` 由规范序列化的 sourceHash、有效事实修订、target、ruleRefs+hash、材料哈希与绑定、作者决定、编译器版本、模板版本、输出语言组成。不得依赖 JSON 不稳定键序。匹配另绑定约束、目录和模型／索引版本。

| 变化 | 保留 | 自动失效／重算 |
| --- | --- | --- |
| 仅字号或界面语言 | 稿件、目标、事实、输出文件 | 重绘视图；若作者明确更改输出说明语言才重编译 |
| 修改投稿偏好 | 源稿、材料、事实、已锁定目标 | 推荐结果；不偷偷更换已选目标 |
| 修改作者信息或声明 | 原稿、非相关附件 | 引用该字段的确认、匿名／标题页／声明等输出 |
| 新版主稿 | 原始版本、已选目标身份、可复用材料 | 重新解析、匹配、适用性、相关确认与所有当前生成物 |
| 替换附件 | 其他材料与事实 | 依赖该文件的检查及投稿包 |
| 更换目标或文章类型 | 通用事实与材料 | 规则、文件用途、目标特定确认与编译结果 |
| 应用新内置规则 | 历史输出与旧规则快照 | 受影响准备项及当前计划 |

初次实现可保守地重算准备视图和编译计划，不必立刻建立复杂增量图。作者输入尽量保留，失效的事实确认只能复核后恢复，不能自动再勾选。作者无需额外点击“重新检查”。

## 6. 错误与国际化

返回 `AppError { code, params, retryable, recoveryAction, diagnosticId }`。首批错误码包括 INPUT_UNREADABLE、FORMAT_UNSUPPORTED、LIMIT_EXCEEDED、MULTIPLE_MANUSCRIPTS、CONTEXT_CHANGED、RULES_UNVERIFIED、MATERIAL_REQUIRED、TRANSFORM_UNSUPPORTED、ANONYMITY_UNVERIFIED、RENDER_UNAVAILABLE、OUTPUT_PERMISSION_DENIED、OUTPUT_DISK_FULL、EXPORT_INTERRUPTED。

前端通过现有 i18n 入口映射 zh-CN/en；Rust 不构造中英混杂句子。params 只放安全字段和数字，不带原始 OS 路径或论文正文。未知错误提供可恢复通用双语说明及诊断 ID，详细本地记录也避免保存敏感全文。

科研内容与官方原文保留原语言，说明和错误按界面 locale。模板文档语言由期刊投稿语言决定，不能因为切换中文界面把英语投稿信翻成中文；作者工具的说明语言另行记录。

## 7. 合同核对方式

先以少量真实序列化样例验证 Rust 与 TypeScript 字段、枚举、缺失值和错误码。没有第二个应用前不引入独立发布的契约包。若选择类型生成器，在已有工具链内验证后再加入命令；本方案不预设当前存在生成脚本。
