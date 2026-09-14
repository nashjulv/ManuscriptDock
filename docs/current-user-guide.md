# 投稿舱 ManuscriptDock V0.60 使用指南

基础功能无需配置模型 · AI 辅助按需启用 · 原稿始终保留。

## 选择任务与目标

首页选择“推荐期刊”或“整理投稿包”。打开本地 PDF、DOCX 或材料文件夹；已有目标可以直接选择期刊。中文／English 界面切换不改变投稿语言限制，推荐结果以同心圆展示候选。目标页面提供要求说明和官方来源链接。

顶部“首页”按钮可随时返回；存在未保存输入时，可选择继续编辑或放弃输入。最近任务的移除只隐藏记录，支持恢复，不删除原稿或已生成文件。

## 整理文件和材料

从文件夹打开时，在所选文件夹内创建投稿包；从主稿文件打开时，在主稿同级目录创建。目录支持图标和列表、进入子文件夹、应用内拖动及外部文件导入。也可以在系统文件管理器中编辑或整理，返回应用触发刷新，或手动刷新。

按目标期刊逐项补齐材料。“一键生成模板”生成可填写结构，“一键生成”根据已具备的信息生成本地草稿。批量按钮遵循同样的跳过规则：已有模板不再生成模板，已有文件不再生成模板或草稿。缺少必要事实、无法生成的材料需要人工补充。

| 右上角状态 | 下一步 |
| --- | --- |
| 需手动补充 | 提供实际材料，或生成可用模板后填写 |
| 模板已存在 · 需补充 | 点击打开模板，填写并保存，再返回刷新 |
| 已修改 · 待检查 | 点击“检查并确认完成” |
| 草稿已存在 · 待核对 | 打开核对，处理待填内容，再执行检查 |
| 已确认 | 当前版本已保存为确认快照；后续编辑需要重新检查 |

本地检查通过后，作者仍须确认事实与期刊要求，再保存快照。文件内容、作者事实或目标变化会使旧结果失效。AI 建议也不能代替此步骤。

## 可选 AI 辅助

1. 在顶部“AI 设置”选择本机回环模型服务或公网 HTTPS 服务，填写兼容 Chat Completions 的地址和模型。外部服务需要密钥，密钥保存在系统凭据库。保存设置不调用模型。
2. 对缺失的投稿信／Highlights 选择“AI 起草”；已有材料可选择“AI 深度检查”，清单顶部可运行“AI 一致性检查”。已有文件不会再次起草。
3. 查看本次服务、模型、发送资料及估计 token，确认有权交由此服务处理后，勾选并执行一次。服务商自行计费；应用不自动重试或切换服务。
4. 核对建议及引用。起草结果需要点击“保存为待核对草稿”才写入新 DOCX；过期结果不可采用。检查没有发现问题也不代表投稿就绪。
5. “AI 使用记录与依据”保留调用状态与来源版本；作者可据此核对使用声明。完整记录随导出包保存在 `records/ai-assistance.json`，不进入出版社文件 ZIP。

当前 AI 稿件输入为摘要及填写的信息，不是主稿全文；材料检查仅针对预览中列出的 DOCX 正文。真实模型内容质量、系统凭据库与 Windows 原生验收边界见[实现记录](releases/V0.60/implementation-status.md)。

## 导出

确认目标要求和必要材料后生成草稿包或正式投稿包。`submission/` 放投稿文件，`author-tools/` 放作者工具，`records/` 放审计记录；`publisher-files.zip` 仅包含投稿文件。由作者将所需文件提交至期刊系统，应用不代为投稿。

## English quick reference

Choose **Find journals** or **Prepare submission package**, then open a local PDF, DOCX, or folder. The interface language does not implicitly filter out English journals. Use the visible **Home** button to return; unsaved input requires a choice.

Browse the package as icons or a list, move/import files, and refresh after external edits. Existing files and templates are skipped by generation actions. Edit templates, run **Check and confirm completion**, verify the author statements, and save the confirmed snapshot. Later changes require another check.

AI assistance is optional. Configure a compatible service in **AI settings**, preview the exact sources, and approve each request. Review evidence before **Save as draft for review**. AI suggestions do not certify facts or confirm submission readiness. Input covers the abstract, entered facts, and the material text shown in the preview, not a full manuscript review. Provider charges and actual model compatibility depend on the selected service.
