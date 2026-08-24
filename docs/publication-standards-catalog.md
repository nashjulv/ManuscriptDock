# ManuscriptDock 内置出版标准目录

- 状态：首批可执行目录
- 核验日期：2026-08-24
- 适用阶段：初投稿准备
- 执行方式：本机确定性规则，不调用 AI，不传输论文

## 1. 产品口径

“内置主流出版标准”不等于“已经完整覆盖全球每一本期刊”。ManuscriptDock 使用分层规则：

1. 通用论文结构和初投稿规则始终启用；
2. 用户按论文实际情况选择国家标准、出版商和研究报告指南；
3. 具体期刊作者指南以后作为差异包叠加，并拥有最高优先级；
4. 能够可靠从稿件判断的项目给出确定性结论；
5. 无法可靠判断的项目明确列为“作者确认”，不伪装成自动通过。

覆盖等级保持原定义：B 表示出版商通用要求已经覆盖但期刊差异仍需确认，C 表示使用国家、学科或通用标准。当前目录没有冒充 A 级具体期刊规则。

## 2. 首批内置目录

### 中国国家标准与文章类型

| 规则包 | 覆盖 | 当前实现 |
| --- | --- | --- |
| GB/T 7713.2—2022 学术论文编写规则 | C | 通用结构、资助、作者与单位、中英文元数据确认 |
| GB/T 7714—2025 参考文献著录规则 | C | 参考文献部分存在性与现行样式确认；逐条格式化等待 CSL 接入 |
| GB/T 7713.4—2025 数据论文编写规则 | C | 数据可用性、仓储、持久标识符、版本、许可和访问条件 |

GB/T 7714—2025 已于 2026-07-01 实施并替代 2015 版，应用不再把 2015 版显示为现行标准。

### 国际基础与出版商

| 规则包 | 覆盖 | 当前实现 |
| --- | --- | --- |
| COPE / CRediT 出版伦理与透明度基础 | C | 利益冲突、资助、作者贡献、数据与 AI 使用声明 |
| Elsevier 出版商通用准备 | B | 标题、作者单位、摘要、关键词、数据、贡献、披露、图表、附信与补充材料 |
| Springer Nature 出版商通用准备 | B | 标题、作者单位、摘要、关键词、声明、图表、附信、补充材料与匿名差异确认 |
| Wiley 出版商通用准备 | B | 标题、作者单位、摘要、关键词、利益冲突、数据、伦理、图表与投稿材料 |
| IEEE 期刊通用稿件结构 | B | 标题、作者与 ORCID、摘要、关键词、首页脚注、正文结构、资助、伦理与投稿文件 |

四套出版商规则现已内置结构化 `submissionElements`。选择多个出版商时，相同要素按稳定
标识合并，同时保留全部规则包、官方来源和完整性状态。要素只表达出版商级常见准备事项；
所有数量、匿名方式、文件格式和文章类型差异继续要求作者核对具体期刊指南。

### 研究报告指南

| 规则包 | 适用研究 | 当前实现 |
| --- | --- | --- |
| ICMJE Recommendations 2026 | 医学研究 | IMRaD、伦理、披露、资助、试验注册与数据共享 |
| CONSORT 2025 | 随机试验结果 | 研究设计标识、注册、官方清单与参与者流程图 |
| SPIRIT 2025 | 随机试验方案 | 注册、伦理、知情同意、方案清单 |
| STROBE | 队列、病例对照、横断面研究 | 设计标识、主要章节和适用清单 |
| PRISMA 2020 | 系统综述与 Meta 分析 | 方法、结果、注册与方案、清单和流程图 |
| CARE | 临床病例报告 | 病例呈现、发表同意、匿名化、时间线和清单 |
| ARRIVE 2.0 | 活体动物研究 | 方法、结果、动物伦理、Essential 10 与 Recommended Set |
| COREQ / SRQR | 定性研究 | 方法、伦理、反身性、抽样、分析路径和适用清单 |

官方清单仍由作者在其官方来源完成。应用内规则只提供可追溯的准备检查，不复制完整清单，也不把报告完整性误称为研究质量评分。

## 3. 已实现的可信边界

- 16 套可选增强规则包和 2 套内部基础规则包均作为只读数据编译进 Rust；
- 每套增强规则记录中英文名称、说明、版本、覆盖等级、地区、类别、官方来源和核验日期；
- Rust 在展示目录和执行检查前验证 Ed25519 数字签名；篡改、未知选择、依赖缺失、循环继承或规则 ID 冲突会拒绝执行；
- WebView 只能提交规则包 ID，不能注入规则内容、文件路径或任意网络地址；
- 检查报告 v2 保存实际使用的全部规则包、版本、来源和完整性状态；
- 规则选择变化后旧检查结果立即失效，重新执行会产生新的版本化输出快照；
- 出版商规则可以输出作者身份、稿件正文、声明伦理和投稿文件四组投稿要素，相同要素不会重复；
- 本批签名私钥只在生成签名时临时存在，随后销毁，未进入仓库。

## 4. 仍未覆盖

- 具体期刊 A 级差异包和重点期刊清单；
- 按期刊自动下载、撤销、轮换和回滚规则；
- GB/T 7714—2025 逐条参考文献解析、文内外对应和 CSL 格式化；
- 摘要字数、关键词数量、图像分辨率、匿名稿和附件包的完整确定性检查；
- 完整官方报告清单的交互填写、附件导出和页码映射；
- 自动排版、返修规则、接收后生产规范和正式出版版式。

下一阶段应根据种子用户真实投稿目标建立首批 A 级期刊差异包，而不是主观维护一个庞大且迅速过期的期刊模板库。

## 5. 官方来源

- [GB/T 7713.2—2022](https://std.samr.gov.cn/gb/search/gbDetailed?id=F159DFC2A91247EFE05397BE0A0AF334)
- [GB/T 7714—2025](https://std.samr.gov.cn/gb/search/gbDetailedCNF?id=4507EFE13D37CB6AE06397BE0A0A601F)
- [ICMJE Recommendations](https://www.icmje.org/recommendations/)
- [EQUATOR reporting-guideline library](https://www.equator-network.org/reporting-guidelines/)
- [CONSORT / SPIRIT 2025](https://www.consort-spirit.org/)
- [PRISMA 2020](https://www.prisma-statement.org/prisma-2020)
- [ARRIVE 2.0](https://arriveguidelines.org/arrive-guidelines)
- [COPE Core Practices](https://publicationethics.org/core-practices)
- [CRediT](https://credit.niso.org/)
- [IEEE article structure](https://journals.ieeeauthorcenter.ieee.org/create-your-ieee-journal-article/create-the-text-of-your-article/structure-your-article/)
- [Elsevier author policies](https://www.elsevier.com/en-gb/researcher/author/policies-and-guidelines)
- [Springer Nature manuscript guidance](https://www.springernature.com/gp/authors/campaigns/writing-a-manuscript/titles-abstracts-keywords)
- [Wiley manuscript preparation](https://authorservices.wiley.com/author-resources/Journal-Authors/Prepare/index.html)
