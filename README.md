# ManuscriptDock

**中文名：投稿舱**

**产品类别：本地论文投稿工作台**

ManuscriptDock 将作者已有的论文整理为可检查、可投稿、可返修、可追溯和可发布的结构化成果。

投稿准备是建立作者信任的入口，形成由作者控制、可追溯、可计算并可按需开放的
“学术知识体”才是长期目标。排版、检查、评审、返修和发布都应持续丰富同一个论文
知识体，而不是产生彼此割裂的文件与聊天记录。

当前本地 MVP 的 M0–M3 可执行闭环已完成：

- 本地选择 DOCX、PDF 或 TEX，WebView 不获得文件路径；
- 创建带内容指纹和审计事件的不可变源快照；
- 在 Rust 中确定性提取结构，并诚实标注 PDF 的解析限制；
- 验证和组合签名规则包，生成可解释的投稿准备结论；
- 保存版本化 JSON 报告与 HTML 预览，全程不发生外部传输。

目标期刊选择、精确期刊规则、自动修复与排版转换、用户目录导出、返修管理、
PWC 专业评审和预印本发布仍是后续产品切片，不属于当前 `0.1.0` 开发 MVP。

## 技术方向

- 桌面框架：Tauri 2.x；
- 前端：React 18 + TypeScript 5；
- 构建：Vite 5 + Cargo；
- 原则：本地优先、非破坏性转换、明确外发授权。

## 文档入口

- [产品设计总纲](docs/product-design-overview.md)
- [MVP 开发计划](docs/mvp-development-plan.md)
- [开发日志](docs/development-log.md)
- [MVP 完成状态与边界](docs/mvp-release-status.md)
- [学术知识体演进路线](docs/academic-knowledge-body-roadmap.md)
- [学术知识体服务模型](docs/knowledge-body-service-model.md)
- [Paperpal 竞争应对与市场定位](docs/competitive-positioning-paperpal.md)
- [UI 方向：简洁学术工作台](docs/ui-design-direction.md)
- [投稿规则系统](docs/submission-rule-system.md)
- [设计系统](design-system/manuscriptdock/MASTER.md)
- [文档索引](docs/README.md)

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
npm run dev
```

仓库级验证：

```bash
npm run check
npm run tauri -- build --debug --no-bundle
```

`npm run frontend:dev` 仅用于浏览器安全的界面状态开发；真实文件选择必须在 Tauri
桌面运行时内验证。
