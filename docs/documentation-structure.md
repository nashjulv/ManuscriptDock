# 文档目录结构、权威关系与迁移计划

- 日期：2026-09-14。
- 本轮实际落地：新增 `docs/implementation/` 详细实现文档、本文与 ADR；更新总索引。
- 本轮未批量移动旧文件。下面 product/reference/archive/releases 是后续文档整理的目标目录，不能当作当前已存在路径。

## 1. 目标结构

```text
docs/
  README.md                       只做总导航、现有版本与实现状态
  documentation-structure.md      文档组织与迁移规则（本文件）
  product/
    overview.md                   两个核心功能与首版范围
    interaction.md                两路径、页面和中英文文案
    journal-matching.md            作者视角的推荐合同
    submission-package.md          作者视角的整理与输出合同
    journal-rules.md               规则来源、维护和支持边界
    scope-plan.md                  产品范围与 P0—P5 总览
  implementation/                 本轮已创建
    README.md
    architecture.md
    contracts-and-state.md
    journal-data-and-matching.md
    submission-compiler.md
    rewrite-and-removal.md
    delivery-plan.md
    validation.md
  reference/
    repository-structure.md        实际源码目录与构建关系
    versioning.md                 显示版本、包版本和安装身份
    desktop-release.md            安装与签名操作事实
    window-and-reading.md         有效窗口／字体规范索引
  releases/
    <实际版本>/
      changes.md                  本版真实实现，不重写未来规划
      validation.md               自动／原生／文件／隐私验证
      known-limitations.md
      user-manual.md              与该实际版本一致的操作说明
  archive/
    lifecycle/                    旧五／七阶段、返修与存证背景
    knowledge-body/               旧知识体、问答、服务网络设计
    research/                     仍有参考价值的旧研究／竞争分析
    validation/                   老版本验证与修复记录
  adr/
    README.md
    0001-*.md ...                 保留决策时间与替代关系
```

不为每个组件创建一份设计文档，不复制同一状态表到多个目录。设计系统继续在仓库现有 `design-system/`，不再复制一套颜色与字体规范进 docs。

## 2. 当前文件到目标目录的映射

| 当前路径（docs 下） | 目标路径 | 处理 |
| --- | --- | --- |
| product-design-overview.md | product/overview.md | 完整移动，保持唯一产品定义 |
| ui-design-direction.md | product/interaction.md | 完整移动，保留交互图与术语表 |
| local-journal-package-plan.md | product/scope-plan.md | 保留范围，实施细节以 implementation 为准 |
| journal-matching-and-submission-targeting.md | product/journal-matching.md + archive/lifecycle/journal-matching-v052.md | 拆出新版合同，旧算法与靶图只归档一次 |
| submission-materials-and-target-package.md | product/submission-package.md + archive/lifecycle/submission-package-v052.md | 新旧合同拆开，避免长文前后相互覆盖 |
| submission-rule-system.md | product/journal-rules.md | 新规则原则为正文，2026-08-24 基线移历史记录 |
| repository-structure.md、versioning-policy.md | reference/repository-structure.md、reference/versioning.md | 与实际代码和版本保持一致，不提前描述目录已存在 |
| desktop-installers.md | reference/desktop-release.md | 保留有效命令，历史平台验收另归 releases/archive |
| multi-monitor-window-state-design.md、typography-and-reading-settings.md | reference/ 下对应原名 | 保留有效规范，由 window-and-reading.md 统一索引 |
| user-manual.md、mvp-release-status.md | releases/v0.52/ 下对应说明 | 只记录实际 V0.52；新版本另建，不能伪造交付状态 |
| first-use-flow-*.md、package-export-status.md、action-prerequisite-guidance-audit.md、platform-compatibility-audit-*.md、journal-url-audit-*.md | archive/validation/ 下原名 | 保留实测日期、版本、原证据链接 |
| mvp-development-plan.md、end-to-end-manuscript-lifecycle.md、submission-revision-desk.md、local-version-library.md、local-workspace-management.md | archive/lifecycle/ 或 reference/ 的有效摘录 | 有用数据不变量提取到当前规范；旧阶段流程不再是入口 |
| academic-knowledge-body-roadmap.md、knowledge-body-*.md、single-knowledge-body-service-architecture.md | archive/knowledge-body/ 下原名 | 保留研究价值，不作为当前开发前置 |
| competitive-positioning-paperpal.md、pdf-extraction-and-normalization.md | archive/research/ 下原名 | 标明资料日期，不因移动认为竞品事实已重新核验 |
| official-source-access.md、ui-visual-system.md、publication-standards-catalog.md | reference/ 的有效部分或 archive/ | 按实际是否保留抓取代码／样式／规则逐项处理 |
| development-log.md | archive/validation/development-log-pre-refocus.md | 旧记录原样保留，新执行事实进入 releases |

`AGENTS.md` 和根 README 保持仓库根入口，移动产品规范时同步更新其中路径。ADR 不编号重排；被替代的 ADR 增加后继链接，不擦掉历史。

## 3. 迁移批次

| 批次 | 何时 | 动作 | 验证 |
| --- | --- | --- | --- |
| D0 详细方案（本轮） | 代码实现前 | 建 implementation、更新导航与删改授权 | 相对链接、锚点、术语与状态检查 |
| D1 当前产品文档归位 | W01 或首次代码工作包 | 新建 product，移动六份当前规范；新旧混合文档拆分 | 全仓库 Markdown 引用与脚本输入路径检查 |
| D2 旧能力退出 | W13 及各删除工作包 | 将仍有价值旧文档归 archive；无价值重复说明直接删除 | 代码、测试、设计页不再引用旧规范；保护研究材料不误删 |
| D3 实际版本记录 | W14 | 建 releases/<实际版本>，编写真实手册和验收 | 手册能按原生界面执行，所有完成声明有证据 |

迁移可以随着实现统一完成，不要求为每个链接保留永久兼容空文件。若外部链接确需兼容，只保留一句迁移链接，明确移除时机，不在旧路径维护第二份正文。Git 已保留历史，不需要把每次编辑另存 v2/final/final2。

在 D1/D2 前检查 `scripts/build_user_manual_pdf.py` 等生成脚本及其他工具引用，必要时同步真实路径。无法更新的固定引用先保留跳转文件并记录原因，不能通过删除检查来掩盖断链。

## 4. 权威与更新责任

| 内容 | 唯一位置 | 更新触发 |
| --- | --- | --- |
| 做什么／不做什么 | 产品总纲 | 用户确认范围变化 |
| 用户怎么操作 | 交互设计 | 任务或页面变化 |
| 内部类型／命令／失效 | contracts-and-state | 合同改变 |
| 推荐、规则数据 | journal-data-and-matching | 字段／算法／维护策略改变 |
| 生成文件与安全边界 | submission-compiler | 格式或变换能力改变 |
| 工作包进度 | delivery-plan | 有证据的开始、完成或阻塞 |
| 测试要求 | validation | 支持矩阵或风险变化 |
| 测试事实／已发布功能 | releases | 实际运行与发行后 |
| 源代码去留 | rewrite-and-removal | 替换或删除工作包完成 |

若新用户指令与文档冲突，先更新决定和受影响合同，再实施；不把旧文档当作阻止重写的理由。保真、用户资料边界和有用规范可以保留，旧功能存在本身不是保留依据。

## 5. 文档写作与状态规则

每份当前规范至少有日期、状态、适用范围及相关链接。计划写“待实现”，验收写真实运行环境和结果；历史文档写版本与是否被替代。不要在“已完成”段落混写未选型的库或未存在的目录。

源码树与命令示例分清规划和可执行；只有工具链／脚本已存在才给运行命令。仓库文件使用相对 Markdown 链接，规划位置用代码路径并标识尚未创建；用户沟通使用绝对文件链接。

对外操作说明统一“打开／选择／加载本地文件（夹）”；真实期刊网站才使用上传语义。产品规范可中文写作，但涉及用户界面和导出说明需给出 zh-CN/en 合同，保留科研原文。

## 6. 删除与保护边界

允许删除无用旧代码和重复文档，不必为旧交互兼容保留内容。仍有审计、研究或用户参考价值的材料可归档，归档不会进入新版必读顺序。

AGENTS 明列的早期研究路径（demo、output、tmp 及指定研究文件）本轮不改；未来若真正清理这些材料应依据当时明确范围，不将“重写产品源码”扩大成删除真实稿件、系统凭据或其他项目。

本轮只是目录规划与新方案落地，未移动应用运行数据，也未删除旧实现文件。
