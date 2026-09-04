# Developer Scripts

Repository-wide developer automation belongs here. Scripts must be deterministic, non-interactive by default, and safe to run from the repository root.

Document every script's inputs, outputs, prerequisites, and whether it accesses the network. Generated outputs must not overwrite source manuscripts or historical research material.

## `start-dev.sh`

由仓库根目录的 `make start-dev` 调用。脚本检查 `node`、`npm`、`cargo`、`lsof`、`ps`、npm
依赖、Tauri 配置和开发端口，然后以前台任务启动 `npm run dev`。退出或按 Ctrl+C 时，脚本会
清理仍占用相关端口的本项目子进程。默认端口是 `1420`；设置 `TAURI_DEV_HOST` 时也检查 HMR
端口 `1421`。

- `MANUSCRIPTDOCK_DEV_PORT`：仅在检查模式中覆盖 Vite 测试端口；
- `MANUSCRIPTDOCK_HMR_PORT`：仅在检查模式中覆盖 HMR 测试端口；
- `MANUSCRIPTDOCK_AUTO_STOP_DEV=0`：发现当前仓库的旧监听进程时退出，不自动发送 `TERM`；
- `MANUSCRIPTDOCK_CHECK_PORTS_ONLY=1`：只运行环境与端口检查，用于诊断和测试。

脚本不访问外部网络。它只会停止工作目录位于当前仓库内、且占用所需端口的进程；其他进程
只报告、不终止。
