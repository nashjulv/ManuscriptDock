# 投稿舱 ManuscriptDock V0.60

**中文名：投稿舱**

**新定位：本地期刊匹配与投稿包整理客户端**

ManuscriptDock 的产品方向收敛为两个核心功能：**推荐期刊**、**根据目标期刊要求整理投稿包**。
打开本地论文或材料文件夹，完成必要条件与事实确认，在本机预览并保存结果。已有目标的作者可直接整理，不必先推荐。

期刊要求由团队 AI 辅助获取公开资料、人工核验后，直接维护仓库中的结构化配置并随客户端内置。
首版不开发人工维护功能、管理后台或独立规则更新服务。输入使用“打开／选择／加载本地文件”，
不以云端上传概念组织交互；版本、证据与原稿保护继续保留，知识体和远程评审不进入新版核心流程。

**状态：V0.60 增加可选 AI 材料助手。**支持投稿信/Highlights 起草、单项材料语义检查及材料间一致性检查。每次调用前预览资料并确认，结果须作者核对，现有文件不会被覆盖。基础功能无需配置模型，原稿始终保留。见 [AI 辅助范围](docs/optional-ai-assistance.md)；材料检查与快照流程见 [V0.58 实现记录](docs/releases/V0.58/implementation-status.md)。
执行入口：[详细实现方案](docs/implementation/README.md)；产品范围见[总体计划](docs/local-journal-package-plan.md)。
实际实现允许重写旧代码和交互，删除不服务于两个核心功能的模块，不为保留旧体系增加兼容层。
用户原稿和已有导出文件仍须保护；因早期版本尚未上线，V0.54 不再扫描、展示或迁移早期任务，磁盘上已有文件也不会被应用主动删除。

当前本地 MVP 已形成两条直接任务路径：

- 原生选择 PDF、DOCX 或材料文件，WebView 只获得短期 token，不获得任意文件路径；PDF 在本机提取有限文本用于推荐，不推断作者；
- 在 `workspace-next` 中创建带内容指纹的不可变源快照并恢复最近任务；同一规范化文件夹复用同一个项目记录，主稿变化保留旧版本，导出默认写入新的版本化子目录；
- 最近任务可以从首页隐藏并恢复；此操作不删除项目快照、生成文件或外部导出；
- 从 12 本内置 AI 期刊目录执行确定性硬过滤、稳定排序和最多三个候选；
- 界面语言与投稿语言分开，中文界面不再自动过滤掉英文期刊；作者可明确选择投稿语言；
- 3 本 Elsevier 试点期刊提供核验来源、作者事实缺项和正式导出边界；
- 已知目标可跳过推荐，目录目标不需要伪造推荐记录；
- 草稿和正式投稿包严格分隔 `submission/`、`author-tools/` 与 `records/`，并生成只含投稿文件的 ZIP；
- 投稿目录支持图标／列表、目录内拖动、导入与刷新；默认在所选文件夹内或所选主稿的同级目录创建投稿包；
- 按目标规则列出缺失材料，支持逐项／批量生成模板与本地草稿，已有文件跳过；编辑后检查、作者确认并保存版本快照；
- 可选 AI 设置只服务于材料起草与检查；知识体、官网实时抓取、存证、投稿登记和旧多阶段路由不恢复。

首批主稿采取包级原样保留。PDF 可直接用于推荐与草稿检查；三本试点期刊的正式投稿包仍要求作者明确提供核对过的可编辑 DOCX，应用不会把 PDF 反向转换或伪装成 Word 文件。XLSX 填写表已生成并通过兼容阅读器试验，但复杂 OOXML 变换、Windows 真机与 Microsoft Word／Excel 实页验收仍待后续工作包完成。当前证据和验证边界见 [V0.60 实现状态](docs/releases/V0.60/implementation-status.md)，基础核心的历史证据见 [V0.54 记录](docs/releases/V0.54/implementation-status.md)。

## 技术方向

- 桌面框架：Tauri 2.x；
- 前端：React 18 + TypeScript 5；
- 构建：Vite 5 + Cargo；
- 新版原则：两项核心任务本地运行、非破坏性整理、规则内置、简单交互；旧外发命令已从运行时移除。

## 文档入口

- [V0.60 使用指南](docs/current-user-guide.md)
- [可选 AI 材料助手](docs/optional-ai-assistance.md)
- [产品设计总纲](docs/product-design-overview.md)
- [详细实现方案与工作包](docs/implementation/README.md)
- [旧代码重写与删除清单](docs/implementation/rewrite-and-removal.md)
- [文档目录结构与迁移计划](docs/documentation-structure.md)
- [本地选刊与投稿包整理实施方案](docs/local-journal-package-plan.md)
- [两核心任务的简洁交互](docs/ui-design-direction.md)
- [ADR 0010：定位与架构范围调整](docs/adr/0010-local-journal-package-focus.md)
- [开发日志](docs/development-log.md)
- [MVP 完成状态与边界](docs/mvp-release-status.md)
- [投稿规则系统](docs/submission-rule-system.md)
- [设计系统](design-system/manuscriptdock/MASTER.md)
- [文档索引](docs/README.md)

旧里程碑与暂缓的知识体／服务网络方案保留在文档索引中，不再作为当前产品定位。

## 仓库结构

- `apps/desktop/`：Tauri + React 桌面应用；
- `crates/manuscript-core/`：Rust 本地可信核心；
- `packages/`：前端 UI 与应用契约；
- `schemas/`：论文、规则包、快照和交换模式；
- `fixtures/`：仅限合成测试资料；
- `tests/`：跨模块和端到端验证；
- `scripts/`：仓库级开发工具；
- `docs/adr/`：架构决策记录。

完整说明见 [仓库结构](docs/repository-structure.md)。

## 本地开发

正式开发环境使用 Node.js 24 LTS、npm 11 和 Rust 1.93。

```bash
npm install
make start-dev
```

`make start-dev` 会先检查 Node、npm、Rust、依赖目录及 Vite 端口 `1420`。如果端口由当前
仓库遗留的开发进程占用，它会正常停止该进程并等待端口释放；如果占用者属于其他项目，命令
会安全退出并显示 PID/命令，不会终止无关程序。需要绕过自动清理时，可设置
`MANUSCRIPTDOCK_AUTO_STOP_DEV=0`。底层仍可直接运行 `npm run dev`。

仓库级验证：

```bash
npm run check
npm run tauri -- build --debug --no-bundle
```

`npm run frontend:dev` 仅用于浏览器安全的界面状态开发；真实文件选择必须在 Tauri
桌面运行时内验证。
