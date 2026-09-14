# 内置期刊数据、规则与推荐实现

- 状态：部分实现。V0.53 已内置 12 本目录、3 本试点组合规则、类型化数量／长度约束和签名资源 manifest；官方指南核对范围与未完成的独立人工复核见[核对记录](../releases/V0.53/journal-rule-audit.md)。公共要求的维护仍是编辑仓库数据的团队工作，不是产品功能。
- 关联：[合同](contracts-and-state.md)、[规则原则](../submission-rule-system.md)、[工作包](delivery-plan.md)。

## 1. 数据分开存，使用时组合

| 数据 | 建议位置（规划） | 用途 |
| --- | --- | --- |
| 期刊基本身份与候选检索字段 | `crates/manuscript-core/journal-data/catalog.json` | 目录搜索和候选召回，不承诺规则完整 |
| 期刊资料 | `journal-data/journals/<journal-id>.json` | scope、文章类型、出版路径、费用、时间及来源 |
| 可执行投稿要求 | `rule-packs/journals/<journal-id>/<version>.json` | 期刊差异与父规则引用 |
| 配套文档模板 | `templates/<template-id>/<version>/` | 标题页、投稿信、声明与字段表 |
| 核验材料与变更说明 | `journal-data/evidence/<journal-id>/` | 短证据、指纹、核验日期与审核记录 |
| 许可与资源清单 | `journal-data/licenses/` 与资源 manifest | 哪些内容可以随客户端分发 |
| AI 草稿 | 开发者本地临时目录 | 不被运行时加载或默认打包；不得含用户论文 |

发布资源使用显式 manifest 白名单，不递归把 evidence、原始网页或草稿全部打包。客户端只需要允许分发的短证据、来源链接和核验元数据。首次可以直接 include/embed 配置；增量热更新、维护后台和在线目录都不建设。

## 2. JournalRecord 的结构约定

必需：`id, displayName, publisher, identity, schemaVersion, dataVersion, sourceFacts, ruleRefs, generationCoverage`。身份优先以有效 ISSN/eISSN 对齐，保留刊名别名与更名关系；不能只靠模糊名称合并两个期刊。

`sourceFacts` 至少包含：主题标签与原文 scope、支持文章类型、语言、出版路径、索引／排名证据、投稿状态、费用、时间信息。每一事实带 `value, status, sourceRef, verifiedAt, validUntil?`，禁止用一条全局 updatedAt 掩盖各字段更新差异。

费用按 publication route 建模：订阅／全 OA／混合 OA；包括 currency、amount 或 range、mandatory/optional、taxIncluded、额外费用、减免适用条件。预算比较只使用可比币种；首版不请求在线汇率，币种不一致且无明确折算依据时为未知。不能因为 APC=0 就断言总费用为零。

时间数据记录 metric（首轮决定／录用／线上发表）、统计窗口、样本范围与来源。禁止用首轮决定替代录用周期；描述性统计不能保证某篇论文在截止日前录用。作者完成投稿期限可结合实际准备任务评估，不能按期刊档次硬编码天数。

`generationCoverage` 按 articleType＋stage＋documentFeatureProfile 标注：supported、partial、unsupported；匹配覆盖和生成覆盖分别展示。只有目录记录不能被标成“已支持生成”。

## 3. 单条规则合同

新增字段与旧 RulePack schema 的兼容在数据工作包明确；不能修改旧签名 JSON 而继续使用旧签名。

| 字段组 | 内容 |
| --- | --- |
| 身份 | ruleId、schemaVersion、ruleVersion、journalId、parentRefs |
| 适用 | articleTypes、stages、conditions、exceptions |
| 义务 | required/recommended/author_confirmation、severity、deliveryKind |
| 判断 | factKey、operator、typedValue、unit、unknownPolicy |
| 输出 | requiredFileKind、templateRef、allowedTransformIds、reviewSlot |
| 证据 | officialSource、shortQuote、capturedAt、verifiedAt、sourceHash、reviewer、licenseRef |
| 展示 | label/labelEn、description/descriptionEn、messageCode 与参数 |
| 生命周期 | active/superseded/disabled、validUntil、replacementRef |

首批运算符限定为类型受控的存在、集合成员、数值比较、布尔条件和作者事实确认。规则是数据，不执行任意 JS、Python、shell 或网络请求；不为规则表达式引入通用脚本引擎。

来源义务与执行结果分开：required 是期刊要求，pass/fail/unknown/not_applicable 是本次判断结果。无法检测的必需事实进入作者确认，不改成建议；无法核验的规则不能凭作者随手勾选变成事实准确。

继承先检查范围：通用→出版商→具体期刊→文章类型／阶段差异。明确例外覆盖指定规则 ID，保留覆盖原因；相同优先级矛盾即配置错误。不通过简单“最后一条胜出”消除冲突。

## 4. 人工核验并内置的实际步骤

1. 从种子作者需求确定一个子领域、1—2 类文章和 3 本试点，记录选择依据。
2. 获取官网／官方指南；AI 可整理文本和提议规则字段，也可借允许使用的其他投稿站发现遗漏。
3. 人工比对原始条目，核对投稿阶段、稿件类型、例外、数值与单位。第三方与官方矛盾时不自行平均，记录待核验。
4. 将已核验条目写入仓库；未知费用／周期留空并标状态，未核验条目不进入有效必需规则集合。
5. 为规则编写正例、反例与不适用／未知样例，检查父规则和模板引用。
6. 核验身份、英文说明和来源许可；配置及审核说明进入同一个变更。
7. 更新资源 manifest、独立规则版本和对应签名材料；正式发布密钥来自仓库外的发行配置，不放入源码。复用已有验证，不先建签名服务。
8. 随应用构建，在离线干净环境读取相同资源并完成目标投稿包验收。

每次规则变更在 Git 记录改动项、来源变化、受影响类型、测试与维护责任人；没有后台工单状态管理的要求。升级后保留旧任务引用的规则快照，新任务使用新有效规则；旧客户端不能假装知道服务器从未推送的停刊消息。

## 5. 本地特征与召回

从 DOCX 读取标题、摘要、关键词、语言、类型线索、章节、图表、参考文献和声明准备情况。主题与方法识别都要保留来自哪些段落的依据及不确定性。首版不要求提炼所有科学结论，也不建立完整知识体。

将当前 17 本静态常量迁为目录数据；候选来源统一由 CatalogRepository 提供，补充新的记录即可成为真实候选，不再仅为常量添加分区证据。小目录先顺序扫描，不建复杂向量数据库。

本地 XLSX 导入复用已有读取能力，但显式选择它是认可名单还是分区证据；确认表头和年份。优先 ISSN 匹配，模糊名称匹配显示待核对；空表、不完整名单、重复行和同名刊需回报。机构规则不能从学校名称或模型记忆推测。

## 6. 硬过滤

对每个硬条件生成 `ConstraintResult { constraintId, status, evidenceRefs, explanationCode }`。

- 任一 fail → 排除；无 fail 但有 unknown → 还需核对；全部 pass → 合格池。
- 未知候选不能用高主题分补偿；不自动调整用户预算或目标索引。
- 方法明确不接收、文章类型明确不接受、明确停用、语言不符等可排除。
- 字数与图表差距通常计入准备成本；只有规则绝对限制且作者不接受必要修改时才排除。
- 不接收该研究主题须有证据，语义相似度低本身是排序信号，不等于官方拒收。

UI 展示各池数量及最主要冲突原因；作者改变条件后重新计算，不以新增手工“同意不符合”按钮绕过硬条件。

V0.53 的首个可操作切片在推荐页提供 SCIE／Scopus 收录复选框，并在每张候选卡片中展开对应的 `ConstraintResult`。这只覆盖标准收录条件；机构名单／分区 XLSX 仍须完成上文所述的用途、年份、表头和匹配确认后才能作为硬条件。候选卡片同时列出选刊后需要完成的准备任务，避免把主题接近误写成“可立即投稿”。

## 7. 评分与缺失信号

采用固定版本的可解释特征评分，先设确定性基线，再与语义增强比较。八维权重是实验起点，不在正式 UI 展示精确总分。每维带 observed、score、evidenceRefs，不以中性常数填未知。

用于实验的缺失处理：固定全部权重的分母，已知信号计算 lowerBound，缺失部分只扩大 upperBound；另算 coverage。不要仅按已有维度归一化，让资料贫乏期刊轻易升到最高。候选比较优先满足证据覆盖的标准，同级再按已知证据与角色目标排序；重叠区间明确视为不确定，使用稳定 journal ID 作最终并列规则。

这些边界不是录用概率或统计置信区间。各学科覆盖阈值、关键词基线、信号归一化与权重在训练／开发样本上确定，独立测试集锁定后验收。若缺少方法或周期数据，解释只能承诺已观测维度。

准备成本来自目标任务：需作者输入的字段、缺少的附件、可自动执行的变换、必须外部修改的内容。首轮按任务类别和数量排序，未有真实工时样本前不显示“还需 X 分钟”。

“优先首轮决定较快”是软偏好：只有目录中存在来源明确的同口径 `firstDecisionDays` 时才给排序增益；缺失时增加待核对风险，不作为硬过滤，也不以零值或默认天数参与排序。

## 8. 三个角色的确定

先从合格池选综合推荐，再在剩余候选中选有目标证据支持的进阶选择，最后选准备任务更少的候选。每期刊只占一位；角色依据不足可以使用其他明确描述或少给一位，不从未知池凑数。

角色差异以用户条件和证据定义，不把“进阶”写成录用难度量化。每张卡片的解释由实际命中的字段／规则生成模板，附来源引用与缺失信号；不调用云模型生成看似合理的推荐理由。

## 9. 本地语义增强的独立试验

仅在确定性基线建立后加入 EmbeddingProvider。模型版本、tokenizer、截断方式、归一化、向量维度、索引版本与授权一同固化；查询和内置向量必须来自相同模型配置。标题与摘要分块，不能静默只取开头造成方法信息丢失。

候选规模小先用内存余弦比较，近期论文数量扩大后再根据内存实测选择索引，不先上 Qdrant。训练／参考语料需有可分发依据，发布资源只包含获准内容；测试论文、近重复版和未来数据必须排除。

ONNX Runtime 提供原生 C 集成文档；Rust 接入、二进制打包和特定模型支持仍需单独验证，不能把通用 runtime 文档当某模型的部署证明。[官方文档](https://onnxruntime.ai/docs/get-started/with-c.html)

模型未通过效果、内存、许可或离线分发验收时保留透明的确定性基线，不声称“近期论文语义相似度已参与”。是否在首个公开版本启用由工作包的独立评测决定；不阻塞已知目标的材料整理。
