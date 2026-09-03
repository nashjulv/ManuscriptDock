# 桌面安装包

## 产物

ManuscriptDock 采用平台原生安装包：

| 平台 | 架构 | 安装包 | 默认安装范围 |
| --- | --- | --- | --- |
| macOS 11+ | Apple Silicon + Intel 通用二进制 | `.dmg`，同时保留 `.app` | 拖入 Applications |
| Windows 10/11 | x64 | NSIS `-setup.exe` | 当前用户，无需管理员权限 |

Windows 安装器包含简体中文和英文，并根据系统语言自动选择。缺少 WebView2 时，安装器使用
微软 bootstrapper 静默补齐；Windows 10 和 11 通常已经随系统提供 WebView2。

## 本机构建

```text
npm run bundle:mac
npm run bundle:windows
```

第二条命令应在 Windows x64 环境执行。在 macOS 上准备好 Tauri 官方交叉工具链后，可运行：

```text
npm run bundle:windows:cross
```

macOS 交叉链要求 `cargo-xwin`、NSIS、LLVM 和 LLD；脚本通过 Homebrew 自动定位 keg-only
的 `llvm-rc` 与 `lld-link`，缺少工具时会在编译前明确失败。

macOS 产物位于 `target/universal-apple-darwin/release/bundle/`，Windows 产物位于
`target/x86_64-pc-windows-msvc/release/bundle/nsis/`。这些二进制产物属于本机构建结果，
不提交到 Git。

## V0.35 本地构建记录

2026-09-03 已从提交 `0273e2e` 重新生成以下开发安装包，并复制到本机 `installers/` 交付目录：

| 平台 | 文件 | 大小 | SHA-256 |
| --- | --- | ---: | --- |
| macOS universal | `ManuscriptDock-0.35.0-macOS-universal.dmg` | 20 MB | `5c52a761c2ca861f3cd135cf4d322d3e33e4fb4ed59e49a22f05c4770e86d5ac` |
| Windows x64 | `ManuscriptDock-0.35.0-Windows-x64-setup.exe` | 6.6 MB | `cbf9c99679f0536b3b934e2fa99be3b1b19e440a5d43c5d6ed06985a0dfc5d3e` |

macOS DMG 已通过 `hdiutil verify`，内含应用版本 `0.35.0`、标识
`com.manuscriptdock.desktop`，主程序同时包含 `arm64` 与 `x86_64`。Windows 安装器由 NSIS
生成，内含 GUI 子系统的 x86-64 PE 主程序；由于它是在 macOS 上交叉构建，仍需在干净的
Windows 10/11 真机完成安装、启动、卸载和 WebView2 回归。两套包均未完成公开发行所需的
Developer ID/Apple 公证或 Windows Authenticode 签名。

## 自动构建

`.github/workflows/build-installers.yml` 在手动触发或推送 `desktop-v*` 标签时，分别使用真实
macOS 和 Windows runner 构建并上传两个安装包 artifact。Windows 构建不使用 macOS 产物
冒充；Mac 构建同时编译两种 CPU 架构。

## 签名边界

当前开发包未配置 Apple Developer ID、公证凭据或 Windows Authenticode 证书。macOS CI
使用 ad-hoc 签名保证包体完整性，但公开分发前仍需 Developer ID 签名与 Apple 公证；
Windows 公开分发前需要 Authenticode 签名以减少 SmartScreen 警告。签名证书和密码只能
放入发布环境的秘密存储，不能写入仓库。
