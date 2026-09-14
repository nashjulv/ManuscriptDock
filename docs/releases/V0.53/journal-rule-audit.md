# V0.53 三本试点期刊官方指南核对记录

- 核对日期：2026-09-14
- 范围：初次投稿、research article 的公共准备要求；不覆盖特刊、受邀文章、修回、录用后制作或 Editorial Manager 的动态表单
- 方法：在浏览器中直接读取三本期刊当日官方 Guide for Authors 页面，对标题页、摘要、关键词、匿名评审、Highlights、利益冲突、经费、研究数据、CRediT 和生成式 AI 披露章节逐项比对；仓库只保存改写后的短规则、来源链接和日期，不转载网页正文
- 审核边界：这是实现会话中的官方来源核对，不替代发行前由产品／编辑责任人完成的独立人工复核和签字

## 共同条款

三本试点均要求可编辑的 DOCX/TEX 源文件，PDF 不能作为源文件；标题页需包含题名、作者、单位和通讯作者邮箱。摘要上限均为 250 个英文单词。所有作者需披露利益冲突，官方流程会生成独立 Word 声明；经费来源及资助方角色也需披露，无经费时应明确说明。研究数据均采用 Option C：存入适当仓库并在稿件中引用／链接，无法共享时说明原因。

三本均鼓励提交独立可编辑的 Highlights 文件，规则为 3—5 条、每条最多 85 个字符。由于官方用语是 encouraged，运行时将其建模为建议项；只有作者确认且满足条数和长度时，才生成进入出版社 ZIP 的 `submission/highlights.docx`，不满足时只保留作者工具草稿和提示。

生成式 AI 使用披露是条件义务：实际用于稿件准备时需说明工具、用途和人工复核责任；基础拼写、语法和参考文献检查不适用。运行时将其显示为建议／条件项，不把空值误判为作者使用了 AI，也不自动编造声明。

## 期刊差异

| 期刊 | 官方来源 | 关键词 | 评审匿名 | CRediT | 运行时落实 |
| --- | --- | --- | --- | --- | --- |
| Artificial Intelligence | [Guide for Authors](https://www.sciencedirect.com/journal/artificial-intelligence/publish/guide-for-authors) | 1—10 个英文关键词 | 单匿名 | 必需 | 关键词数量和 250 词摘要为必需验证；CRediT 为必需作者事实 |
| Expert Systems with Applications | [Guide for Authors](https://www.sciencedirect.com/journal/expert-systems-with-applications/publish/guide-for-authors) | 1—7 个英文关键词 | 双匿名 | 建议 | 正式包必须提供作者核对的匿名 DOCX；匿名稿替代实名源稿进入 `submission/`，实名原稿不进入出版社 ZIP；标题页另行生成 |
| Knowledge-Based Systems | [Guide for Authors](https://www.sciencedirect.com/journal/knowledge-based-systems/publish/guide-for-authors) | 1—7 个英文关键词 | 单匿名 | 必需 | 关键词数量和 250 词摘要为必需验证；CRediT 为必需作者事实 |

## 配置与测试映射

- `rules.json` 使用公共 Elsevier 父规则与三本子规则，数值约束为类型化字段，不执行脚本。
- `resource-manifest.json` 升级到 `2026-09-14.2`，绑定新的规则哈希；Ed25519 公钥与签名同步轮换，私钥未写入仓库。
- `PreparationView` 只有在事实经作者确认且满足数量／长度约束时才标为 ready；必需项阻止 final，建议项不会伪装成硬要求。
- 合成集成测试覆盖 ESWA 8 个关键词被阻止、修正为 7 个后通过、缺少匿名稿被阻止、作者选择匿名稿后实名源稿不进入 `submission/manuscript.docx`，以及本地快照哈希异常时停止生成。
- macOS 隔离 QA 应用已实走 ESWA 路径：加入作者核对匿名稿后必需阻断项归零，生成并导出 19 个文件；出版社 ZIP 只有 4 个 `submission/` 文件。导出主稿哈希与匿名稿相同且与实名源稿不同，zh-CN/en 的文件分类和成功回执均已核对。
- 后续合成测试与 macOS 原生负向验收补充匿名稿泄漏防线：XML／关系部件中与作者姓名、单位或邮箱完全匹配的内容会阻止准备和生成，zh-CN/en 均只显示命中类别；更改作者事实会重新扫描，安全替代稿可恢复就绪。该检查不覆盖别名、自引语义、图片文字和不透明嵌入内容，仍需作者人工核对。

## 仍需独立复核

- 发行前由具名责任人复核页面是否有当日变更、文章类型例外、特刊差异和投稿系统新增动态字段。
- 目前没有把生成式 AI 的“适用／不适用”做成独立三态选择；空值只表示未提供，不代表作者确认未使用。
- 资金、数据和 CRediT 文本会进入作者工具与字段表，但当前不会自动改写源稿正文；作者仍需在实际稿件中的正确位置核对这些段落。
