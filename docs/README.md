# 投稿舱 ManuscriptDock V0.54 文档

## 当前方向与实施入口

2026-09-14 确认收敛为“推荐期刊”和“按目标期刊整理投稿包”。V0.54 已实现 PDF／DOCX 本地入口、可恢复的最近任务移除，并延续新的本地核心、双任务界面与受控 IPC；未完成项和验证边界以 [V0.54 实现状态](releases/V0.54/implementation-status.md)为准。规则由团队人工核对后直接维护仓库配置并内置客户端，首版不开发维护功能或独立更新服务。

进一步确认：实际实现可重写全部旧代码与交互，删除无用功能，不以旧体系兼容为目标；由于早期版本尚未上线，旧任务入口、扫描、导入和迁移运行时均已移除，应用不会主动删除磁盘上已有文件。当前详细方案集中在新建的 `implementation/`，product/reference/archive 目录仍按迁移计划后续整理；本轮真实实现与验证口径记录在 [V0.54 实现状态](releases/V0.54/implementation-status.md)。

### 详细实现（先读）

- [实现总览](implementation/README.md)：阅读顺序、已决定／需试验的技术边界。
- [架构与源码目录](implementation/architecture.md)：新模块、存储树、权限和依赖。
- [数据合同与状态](implementation/contracts-and-state.md)：对象、IPC、任务、失效和恢复。
- [期刊数据与推荐](implementation/journal-data-and-matching.md)：内置配置、人工核验、过滤排序和语义试验。
- [投稿包整理器](implementation/submission-compiler.md)：DOCX、模板、匿名、实页预览、XLSX/PDF/ZIP。
- [重写与删除清单](implementation/rewrite-and-removal.md)：逐模块去留、依赖及旧任务兼容移除边界。
- [实施工作包](implementation/delivery-plan.md)：W01—W14 依赖、产物、完成条件与当前状态。
- [三本试点期刊规则核对](releases/V0.53/journal-rule-audit.md)：2026-09-14 官方指南逐项核对、运行时映射与独立人工复核边界。
- [测试与验收](implementation/validation.md)：合成样例、两路径、双语、平台、真实文件和隐私证据。
- [文档目录规划](documentation-structure.md)：目标树、逐文件迁移映射、唯一权威位置与整理批次。
- [ADR 0011：重建与淘汰旧流程](adr/0011-rebuild-core-and-retire-legacy-flows.md)。

### 产品范围与交互

- [产品设计总纲](product-design-overview.md)：定位、两核心功能、用户、范围和隐私边界。
- [本地选刊与投稿包整理实施方案](local-journal-package-plan.md)：两条路径、文件夹行为、规则配置维护、P0—P5 切片及验收。
- [简洁交互设计](ui-design-direction.md)：两个入口、页面示意、本地打开语义、失败恢复和中英文文案。
- [ADR 0010：范围调整](adr/0010-local-journal-package-focus.md)：与旧生命周期、远程模型和知识体方向的关系。
- [期刊匹配与投稿目标推荐](journal-matching-and-submission-targeting.md)：顶部为新版推荐合同，后文为既有实现参考。
- [投稿资料与目标投稿包](submission-materials-and-target-package.md)：顶部为新版整理合同，后文保留资料、确认和导出事实。
- [投稿规则系统](submission-rule-system.md)：分层规则、AI 初稿、人工核验、仓库配置与客户端内置。

产品范围与交互决定作者体验，详细实现目录决定工程合同与工作包；同类信息按[目录治理](documentation-structure.md)只维护一处。旧使用手册、里程碑和验证记录说明当前或历史实现，不能作为新方案已交付的证据。旧设计中的多阶段导航与常驻证据栏让位于简洁交互；仅复用有用的视觉、可访问性和数据完整性规范。

## 当前版本使用与验证

- [产品使用手册](user-manual.md)：V0.52 旧界面历史手册，尚未改写为 V0.54 新界面。
- [版本规则](versioning-policy.md)：用户可见更新递增显示版本，并同步 SemVer 包版本。
- [MVP 完成状态与边界](mvp-release-status.md)：已有能力与历史测试记录。
- [开发日志](development-log.md)：既有实现记录。
- [V0.51 投稿包导出状态修复](package-export-status.md)：导出与登记分离、恢复及兼容。
- [V0.50 修复与验证记录](first-use-flow-v050-validation.md)：已有主流程验收及边界。
- [首次使用与投稿主流程修复方案](first-use-flow-remediation-plan.md)：旧流程问题与修复背景。
- [旧 MVP 开发计划](mvp-development-plan.md)：M0—M3 历史里程碑，新工作使用本轮实施方案。
- [操作前置条件与点击引导盘点](action-prerequisite-guidance-audit.md)：旧状态引导与测试参考。
- [期刊网址核验记录](journal-url-audit-2026-09-06.md)：17 条内置入口的历史核验。

## 可复用技术与设计基础

- [内置出版标准目录](publication-standards-catalog.md)：既有基础包及覆盖边界。
- [本地论文版本库](local-version-library.md)：不可变原稿、比较与恢复；不作为新版独立主步骤。
- [最近工作区管理](local-workspace-management.md)：归档、恢复和删除边界。
- [投稿优化修订台](submission-revision-desk.md)：现有修订能力参考，后续范围按两核心任务取舍。
- [PDF 提取与规整方案](pdf-extraction-and-normalization.md)：既有研究；新版不承诺 PDF 到高质量 Word。
- [端到端论文生命周期](end-to-end-manuscript-lifecycle.md)：保留内部证据状态，旧五任务导航不再定义新界面。
- [官方来源读取与 HTTP 兼容](official-source-access.md)：既有实现；新作者流程优先内置配置，不要求实时抓取。
- [多显示器窗口恢复](multi-monitor-window-state-design.md)：原生窗口兼容与待验范围。
- [字体与阅读设置](typography-and-reading-settings.md)：字号与双语可读性。
- [全局设计系统](../design-system/manuscriptdock/MASTER.md)：配色、字体、图标与可访问性；新导航以简洁交互为准。
- [旧 UI 视觉方案](ui-visual-system.md)：视觉资产与令牌参考，旧信息架构已被本轮方案替代。
- [仓库结构](repository-structure.md)
- [桌面安装包](desktop-installers.md)
- [macOS / Windows 兼容性复验](platform-compatibility-audit-2026-08-24.md)
- [架构决策记录](adr/README.md)

## 暂缓的背景方案

以下资料保留研究与历史决策，不作为当前产品根目标、作者必经步骤或首批实施依赖。

- [学术知识体演进路线](academic-knowledge-body-roadmap.md)
- [学术知识体服务模型](knowledge-body-service-model.md)
- [单篇论文知识体五部分服务架构](single-knowledge-body-service-architecture.md)
- [学术知识体关联网络模型](knowledge-body-network-model.md)
- [知识体问答与作者自带模型](knowledge-body-dialogue-and-model-routing.md)
- [旧 Paperpal 竞争定位](competitive-positioning-paperpal.md)：知识体为长期差异的旧定位已被替代；竞品资料需另行更新核验。
