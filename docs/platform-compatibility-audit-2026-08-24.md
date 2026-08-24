# ManuscriptDock macOS / Windows 兼容性复验

- 日期：2026-08-24
- 版本：0.1.0 开发 MVP
- 结论：macOS 开发版通过；Windows 核心代码通过交叉目标检查，但桌面应用尚未完成 Windows 原生验收，不能作为 Windows 可用版本发布。

## 验证环境

- 宿主：Apple Silicon macOS 26.5.1，ARM64
- Node.js：25.6.0（仓库发布基线仍为 Node 24）
- npm：11.8.0
- Rust：1.93.0
- 桌面框架：Tauri 2，系统 WebView

本次没有 Windows 真机、Windows 虚拟机或可用的远端 Windows 执行器。所有 Windows
结论均明确区分交叉编译检查与原生运行验证。

## 结果摘要

| 检查层级 | macOS Apple Silicon | macOS Intel | Windows x86_64 |
| --- | --- | --- | --- |
| TypeScript、前端测试、Vite 构建 | 通过 | 共用产物 | 通过，平台无关 |
| Rust 核心测试 | 原生通过，21/21 | 目标编译通过 | 目标编译通过 |
| Tauri 桌面目标编译 | 通过 | 通过 | 在 Windows 资源编译步骤停止 |
| 原生可执行文件 | ARM64 已生成并启动 | x86_64 已生成，未在 Intel 真机启动 | 未生成 |
| WebView 界面回归 | 通过 | 共用前端 | 1366 × 768 布局通过；非 WebView2 真机 |
| 安装包、签名、自动更新 | 未实现 | 未实现 | 未实现 |

## macOS 验证

执行并通过：

```bash
RUSTUP_TOOLCHAIN=stable npm run check
RUSTUP_TOOLCHAIN=stable npm run tauri -- build --debug --no-bundle
rustup run stable cargo check --workspace --all-targets --target x86_64-apple-darwin
RUSTUP_TOOLCHAIN=stable npm run tauri -- build --debug --no-bundle --target x86_64-apple-darwin
```

结果：

- 7 个 React 工作流测试通过。
- 21 个 Rust 测试通过；rustfmt、Clippy `-D warnings`、类型检查和生产前端构建通过。
- 生成并检查了 64 位 ARM64 Mach-O 可执行文件；程序真实启动后保持运行，未立即崩溃，随后由测试主动结束。
- 生成了 64 位 x86_64 Mach-O 可执行文件，证明 Intel Mac 目标可完整构建；当前 ARM64
  设备不能替代 Intel 真机运行验收。
- 当前可执行文件只有链接器临时签名，不是 Developer ID 签名发行物。

因此，macOS 当前达到“本机开发与演示可用”，尚未达到“可交付安装包”。

## Windows 验证

已执行：

```bash
rustup run stable cargo check -p manuscript-core --all-targets --target x86_64-pc-windows-msvc
rustup run stable cargo check --workspace --all-targets --target x86_64-pc-windows-msvc
```

结果：

- `manuscript-core` 及其测试目标通过 Windows MSVC 目标编译，包含 Windows 使用的
  `cfg(not(unix))` 只读权限恢复路径。
- 整个工作区已经编译到 Tauri Windows 资源生成步骤；macOS 宿主缺少 `llvm-rc`，
  `tauri-winres` 因而停止。此结果没有暴露 ManuscriptDock 源代码错误，但也不构成桌面壳通过。
- 当前 CI 只运行 Ubuntu，没有 Windows 或 macOS 原生任务；仓库也没有可供本次调用的
  远端执行器。

Windows 发布前必须在 Windows 原生环境完成：

1. `npm ci` 与 `npm run check`；
2. Tauri 原生构建和 `.exe` 启动；
3. WebView2 首屏、系统文件选择器、应用数据目录、只读快照和重启恢复；
4. DOCX、PDF、TEX 合成稿件的导入、结构解析和投稿准备全流程；
5. 高 DPI、125%/150% 缩放、中文路径、长文件名及 Windows Defender 场景；
6. MSI 或 NSIS 安装、卸载、代码签名与更新验证。

## 界面与可访问性回归

浏览器共用前端在 1180 × 780、配置的最小窗口 760 × 620，以及 Windows 常见的
1366 × 768 下均无横向溢出。主操作尺寸为 150 × 46 px，达到 44 px 最小目标；页面
语言、标题、区域名称和交互控件名称完整，控制台没有警告或错误。

浅色和深色主题、减少动态效果及可见焦点样式均已定义。自动化环境未能可靠地产生
真实 Tab 焦点遍历，因此键盘顺序只完成 DOM 与样式静态检查，仍需在原生 WebView 中手测。

对比度抽查：

- 浅色正文 14.82:1，次要正文 5.37:1，主按钮 6.44:1；
- 深色正文 15.73:1，次要正文 8.01:1，主按钮 7.67:1；
- 浅色 `--text-muted` 小字约 3.43:1，低于小字号文本 4.5:1 的发布目标，需在发行前加深。

## 发布阻断项

当前 `tauri.conf.json` 的 `bundle.active` 为 `false`，所以不会生成 macOS 或 Windows
安装包。本轮结论不等于可公开分发。发布阻断项按优先级为：

1. 增加 macOS 和 Windows 原生 CI，并保留合成稿件端到端测试结果；
2. 在 Windows 真机验证 WebView2、文件系统与安装流程；
3. 启用平台安装包，建立 Apple Developer ID / Windows Authenticode 签名与密钥流程；
4. 修正浅色弱化小字对比度并完成真实键盘遍历；
5. 为 Apple Silicon 与 Intel Mac 决定分别发布或生成 Universal 2 安装包。

## 最终判定

- macOS：**开发版可用，发行版未就绪**。
- Windows：**核心代码具备兼容性基础，桌面版未完成原生验证，不应宣称可用或发布**。

