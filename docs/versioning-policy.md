# 投稿舱 ManuscriptDock 版本规则

- 当前版本：`V0.44`
- 状态：正式发布前的产品迭代版本
- 生效日期：2026-09-04

## 版本递增

每完成一次用户可感知、经过测试并准备提交的产品更新，显示版本默认增加 `0.01`：

- `V0.44` 的下一版本是 `V0.45`；
- 始终保留两位小数，按十进位进位；
- 进入正式发布时再单独确认 `V1.00` 及正式发布规则。

## 显示与安装

- 顶栏、首页品牌区、窗口标题和用户手册等主要身份位置显示完整名称：`投稿舱 ManuscriptDock V0.44`。
- 正文叙述可以继续简称 `ManuscriptDock`，避免重复干扰阅读。
- npm、Cargo 和 Tauri 的机器版本采用 SemVer，`V0.44` 对应 `0.44.0`。
- 应用产品名、Bundle ID 和安装路径保持稳定，确保新版覆盖旧版，不产生多个独立应用。

## 更新检查

每次递增版本时同步检查：

1. `apps/desktop/src/version.ts` 的界面版本；
2. 根目录及桌面端 `package.json`、`package-lock.json`；
3. Rust crate 的 `Cargo.toml`、`Cargo.lock`；
4. `apps/desktop/src-tauri/tauri.conf.json` 的安装包版本与窗口标题；
5. 产品文档的当前版本和开发日志。
