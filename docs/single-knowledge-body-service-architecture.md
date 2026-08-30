# 单篇论文知识体五部分服务架构

- 状态：产品与领域模型基线
- Schema：`AcademicKnowledgeBodySnapshot v3`
- 适用范围：单机知识体、作者问答、未来 PWC 服务与公开知识网络投影

## 1. 定义

单一知识体不是论文摘要、文件容器或某个大模型的聊天记忆，而是：

> 具有稳定身份、明确知识边界、可验证证据、能力契约和可替换运行时的知识服务单元。

单篇论文是首个来源边界，一个知识体可以围绕一个或一组 Claim 形成。Claim 仍采用
`〈命题、条件、证据、来源、状态〉`语义约束，但 Claim 只是知识核心，不等于整个知识体。

```text
┌──────────────────────────────┐
│          身份与版本           │
├──────────────────────────────┤
│       知识、边界与证据         │
├──────────────────────────────┤
│          能力契约             │
├──────────────────────────────┤
│        交互与执行运行时         │
├──────────────────────────────┤
│      验证、权利与信誉记录       │
└──────────────────────────────┘
```

## 2. 五个部分

### 2.1 身份与版本

`IdentityVersionLayer` 固定长期稳定的 `KnowledgeBody` ID，并引用当前
`KnowledgeBodySnapshot`、`ArtifactVersion` 和创建者 `Provenance`。更新形成新快照，
旧快照不得被覆盖。生命周期显式区分 `active`、`deprecated`、`withdrawn` 和
`superseded`；替代关系必须指向具体版本。

知识体身份与知识体版本不能合并：身份用于长期引用，版本用于复现实验和历史解释。

### 2.2 知识、边界与证据

`KnowledgeBoundaryEvidenceLayer` 组合 Claim、Scope、Method、Result、Evidence、
EvidenceRelation 和 SourceAnchor，并显式保存已知限制与未验证对象。

它必须同时回答：

- 知道什么；
- 为什么这样认为；
- 在什么条件下成立；
- 在什么条件下不能使用；
- 哪些部分尚未由作者或专业审核确认。

边界是正式数据，不是界面脚注。对象状态分为：

- `pending v0`：当前可解析内容中没有提取到该对象；
- `candidate vN`：已从统一分解资产提取出带来源的候选，但尚未经作者逐条确认；
- `established vN`：作者已经确认的正式对象。

上传后的本地分解不能只生成标题、摘要和章节统计。`decomposition manifest` 是每个
`ArtifactVersion` 的统一、不可变派生资产，保存文本、表格、图片片段和语义候选，并计算
独立 SHA-256。知识体和投稿包必须引用同一分解 ID 与哈希，实现“一次分解、两个出口”；
规则检查不得另起一套解析结果。

### 2.3 能力契约

每个 `CapabilityContract` 都有稳定 ID 和独立版本，并声明能力、输入、输出、前置条件、
拒绝条件、证据来源与可用状态。状态分为：

- `available`：本机确定性能力已经可执行；
- `requires_runtime`：契约已建立，但需要作者配置可替换运行时；
- `planned`：边界已经声明，当前版本必须拒绝执行。

当前本地知识体声明三个契约：

| 能力 | 状态 | 核心拒绝条件 |
| --- | --- | --- |
| 来源追溯 `source_traceability` | 可用 | 对象没有确认的 SourceAnchor |
| 证据约束问答 `evidence_bounded_question_answering` | 需要运行时 | 超出知识边界、证据不足或运行时不可用 |
| 方法适用性检查 `method_applicability_check` | 规划中 | Scope/Method 尚未确认或输入超出契约 |

能力不能由大模型临时发明。后续执行评分也必须先建立公式、参数、输入约束、不确定性输出和
拒绝规则，再开放执行。

### 2.4 交互与执行运行时

`InteractionRuntimeLayer` 引用独立 `RuntimeProfile`。模型绑定策略固定为
`replaceable`，不保存某个供应商为知识本体。作者配置的主模型和备选模型只负责：

1. 理解用户或 Agent 的任务；
2. 检查能力契约和知识边界；
3. 调用获准的本地投影与 SourceAnchor 查询；
4. 根据证据组织回答；
5. 在超出能力、证据不足或未授权时拒绝。

每次模型外发仍需作者单独确认。更换模型不改变知识体身份、内容快照、证据或历史。

### 2.5 验证、权利与信誉记录

`ValidationRightsReputationLayer` 分别保存：

- 验证记录，如具体版本的 `AIReviewReport`；
- `RightsPolicy`，包括作者控制、归因和再利用约束；
- 独立 `ReputationRecord`，记录复现、挑战、勘误、撤回和任务表现。

必须区分两条时间线：

```text
固定内容：ArtifactVersion → Claim/Method/Result → KnowledgeBodySnapshot Sx
动态信誉：验证 / 复现 / 挑战 / 勘误 / 撤回 → ReputationRecord vN
```

信誉更新不能改写论文内容；内容形成新版本也不能清空既有挑战和验证历史。知识体快照只固定
信誉记录的某个引用，实时信誉是知识体身份下独立演进的状态。

## 3. 单篇论文空间关系

空间图采用“双边界、一个核心、五个星区”：

```text
                         身份与版本
                    KnowledgeBody / Sx
                             │
       知识、边界与证据 ─── Claim ─── 能力契约
       Scope/Method/...     十二面体       Contract vN
                          ╱       ╲
             验证、权利与信誉       交互与执行运行时
             Reputation vN          RuntimeProfile vN
```

- 最外层圆形边界表示长期稳定的 `KnowledgeBody` 身份；
- 内层虚线边界表示不可变内容快照 `KnowledgeBodySnapshot Sx`；
- Claim 仍是缓慢转动的十二面体知识核心；
- 五条直线连接五个正式分区，而不是把所有字段平铺为同级球体；
- Runtime 使用虚线，表示可替换且不属于知识本体；
- Reputation 使用点线和双边界，表示它属于知识体外部状态但独立于固定内容演进；
- 每个星区在物件最上层显示名称，其下显示版本和状态；颜色不是唯一编码；
- Claim 核心上方显示当前分解得到的 Claim 摘要；空间图下方提供 Claim、Scope、Method、
  Result、Evidence 五项可读图例，逐项给出摘要、候选/已确认状态、来源片段和置信度；
- 图例只能读取同一 `DecompositionManifest` 的真实语义候选，不得用字段名或版本号冒充
  论文知识；
- 减少动态模式固定十二面体视角，屏幕阅读器获得完整的五部分等价描述。

两体与关联网络仍使用一等声明对象连接独立知识体，不因为单体内部重组而改变
CitationAssertion、ClaimRelationAssertion、EvidenceRelation、MethodRelationAssertion、
ReproductionAssertion、AlignmentAssertion、VersionRelation 和 ClassificationAssignment。

## 4. 兼容与真实性边界

- 新建快照采用 schema v3，并必须包含五部分架构和可选的统一分解层；
- 既有 schema v1 快照仍可读取和校验，不会因升级失效；
- 机器候选必须由作者逐项决定“纳入”或“排除”；后端校验决定集合与当前分解候选完全
  一致，只有纳入项进入 `established`，审核决定随知识体哈希固化；
- 当前 `AIReviewReport` 默认缺省，不用确定性检查伪装专业评审；
- 当前 `ReputationRecord v0` 表示尚未建立外部信誉记录；
- 方法适用性检查处于 `planned`，在 Scope 和 Method 未确认前必须拒绝；
- 作者问答的最小模型投影包含能力契约和五部分边界，模型回答不能自动改写任何对象。
