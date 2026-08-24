# 学术知识体关联网络模型

## 1. 决策

ManuscriptDock 的知识层不再停留在单一论文的孤立对象，也不把所有研究内容合并为一个
无边界大图。正式模型由多个保持自身身份、对象版本和授权边界的
`AcademicKnowledgeBody` 组成，跨体连接由可引用、可审计、可争议和可撤回的一等声明
对象承担。

```text
单一知识体 → 两体关联 → 关联知识体网络
```

界面采用三级渐进视图。只有当前单体时，后两级保持不可用；形成足够的有效声明后才开放，
不能用演示关系或文本相似度伪造网络完整度。

## 2. 单体边界

单一学术知识体不是“一篇论文的摘要”，而是围绕一个或一组 `Claim` 构成的、可追溯且
可版本化的研究记忆单元。每个知识体保持圆形语义边界，并固定以下核心对象：

| 核心对象 | 作用 |
| --- | --- |
| `ArtifactVersion` | 确定知识体所依据的论文、预印本或报告版本，是不可变来源边界 |
| `Claim` | 核心可引用主张；表达“研究者在特定条件下提出了什么” |
| `Scope` | 限定 Claim 成立的人群、时间、空间、参数、假设和适用范围 |
| `Method` | 记录研究设计、算法、实验流程、数据处理方式和关键参数 |
| `Result` | 保存论文实际报告的观察、测量、统计结果或实验输出 |
| `EvidenceRelation` | 表达某个 Result 如何支持、削弱或无法支持某个 Claim；它不是自动推理结论 |
| `SourceAnchor` | 将 Claim、Method、Result 等对象精确定位到页、段、句、表、图、公式或数据位置 |
| `AIReviewReport` | 审核抽取忠实性、来源锚点、结构完整性和越界推理；不裁定科学真理 |
| `Provenance` | 记录对象由谁、何时、使用什么模型或流程产生、审核和修订 |
| `KnowledgeBodySnapshot` | 固定组合上述对象的具体版本，形成一个不可变、可引用的知识体快照 |

`Claim =〈命题、条件、证据、来源、状态〉` 仍可作为 Claim 内部的语义约束，但不能替代
整个学术知识体。`Method`、`Result`、`AIReviewReport` 等是知识体内部各自版本化的一等
对象，不是 Claim 五元组的新增字段。

## 3. AIReviewReport 独立版本

`AIReviewReport` 是独立版本对象，不是 Claim 的可变属性，也不是确定性投稿检查的别名。

- 首次专业 AI 审核建立 `AIReviewReport v1`；
- 后续审核策略、审核器或结论升级产生 `v2`、`v3`，旧版本继续保留；
- 审核报告版本变化不自动推进 Claim 或五元组要素版本；
- 每个报告版本固定引用被审核的 Claim 具体版本和审核器具体版本；
- 每个知识体快照固定引用一个具体审核报告版本，例如当前引用 `v2`，历史仍保留 `v1`；
- 如果尚未执行专业审核，引用必须为空，不能用本地规则报告补位。

因此，一个快照的关键引用关系是：

```text
KnowledgeBodySnapshot v7
├── Claim v3
├── SourceAnchor v3
├── Method v2
└── AIReviewReport v2
    └── previousVersion: v1
```

## 4. 跨体关系协议

| 关联方法 | 连接对象 | 成立依据 | 协议对象 |
| --- | --- | --- | --- |
| 显式引用 | Artifact ↔ Artifact | 参考文献、DOI、原文引用锚点 | `CitationAssertion` |
| Claim 语义关系 | Claim ↔ Claim | 等价、蕴含、限定、支持、质疑、冲突 | `ClaimRelationAssertion` |
| 证据关联 | Result/Evidence ↔ Claim | 结果是否承担支持或挑战角色 | `EvidenceRelation` |
| 方法迁移 | Method ↔ Method/Claim | 复用、改造、跨学科迁移 | `MethodRelationAssertion` |
| 复现关联 | Study ↔ Study | 目标、实验条件、数据与结果可比较 | `ReproductionAssertion` |
| 概念与身份映射 | Entity/Concept ↔ Entity/Concept | 同一、近似、上下位、部分重叠或争议 | `AlignmentAssertion` |
| 版本与更正 | Version ↔ Version | 修订、替代、更正、撤稿 | `VersionRelation` |
| 学科索引关联 | KnowledgeBody ↔ Discipline | 标准、新兴、弱关联或争议分类 | `ClassificationAssignment` |

每个声明至少包含稳定标识、声明版本、关系类型、协议对象、源对象具体版本、目标对象具体
版本、一个或多个成立依据以及候选、作者确认、已验证、争议或撤回状态。缺少依据、协议类型
不匹配或只由模型相似度产生的边必须被拒绝。

## 5. 参考网络

产品和测试可使用以下完全合成的网络验证布局与协议，不得把它展示为用户论文的真实关系：

| 知识体 | 角色 | 快照 | Claim | SourceAnchor | Method |
| --- | --- | ---: | ---: | ---: | ---: |
| K-A | 原研究 | S7 | v3 | v3 | v2 |
| K-B | 复现研究 | S4 | v2 | v2 | v2 |
| K-C | 竞争研究 | S3 | v2 | v1 | v2 |
| K-D | 跨域应用 | S2 | v1 | v2 | v1 |
| K-E | 后续综合 | S5 | v3 | v2 | v1 |

其中可分别建立 Reproduction、Conflict、MethodTransfer、Citation、EvidenceRelation 和
Classification 等声明。绿色菱形表示声明对象，不表示“关系强度”；圆形边界表示每个知识体
仍拥有独立身份和版本历史。

## 6. 当前实现边界

MVP 的 Rust 可信核心已经定义 schema-versioned 本地快照、十类核心对象引用、Claim
内部语义约束、独立 `AIReviewReport` 版本链、八类关系协议和网络声明校验。当前真实工作区只注册本地单体、
不可变稿件来源和建构状态，不会虚构外部研究、AI 审核或跨体关系。

桌面端支持“单一知识体 / 两体关联 / 关联网络”三级只读投影。后两级只有在核心返回足够的
知识体和有效声明时可用。下一阶段需要增加外部 Artifact 登记、候选声明审阅和作者确认，
再持久化新的知识体快照。
