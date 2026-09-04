#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
auto_stop=${MANUSCRIPTDOCK_AUTO_STOP_DEV:-1}
check_only=${MANUSCRIPTDOCK_CHECK_PORTS_ONLY:-0}
dev_port=1420
hmr_port=1421

if [ "$check_only" = '1' ]; then
  dev_port=${MANUSCRIPTDOCK_DEV_PORT:-1420}
  hmr_port=${MANUSCRIPTDOCK_HMR_PORT:-1421}
fi

fail() {
  printf '%s\n' "错误 / Error: $*" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "缺少命令 '$1'；请先安装本地开发依赖。 / Missing '$1'; install the local development prerequisite first."
}

validate_port() {
  case "$1" in
    ''|*[!0-9]*) fail "端口必须是 1–65535 的整数：$1 / Port must be an integer from 1 to 65535: $1" ;;
  esac
  [ "$1" -ge 1 ] && [ "$1" -le 65535 ] || fail "端口超出范围：$1 / Port is out of range: $1"
}

listener_pids() {
  lsof -nP -t -iTCP:"$1" -sTCP:LISTEN 2>/dev/null | sort -u || true
}

process_command() {
  ps -p "$1" -o command= 2>/dev/null || printf '%s' "命令不可用 / command unavailable"
}

process_cwd() {
  lsof -a -p "$1" -d cwd -Fn 2>/dev/null | sed -n 's/^n//p' | head -n 1
}

is_project_process() {
  process_dir=$(process_cwd "$1")
  case "$process_dir" in
    "$project_root"|"$project_root"/*) return 0 ;;
    *) return 1 ;;
  esac
}

wait_for_port_release() {
  wait_port=$1
  wait_count=0
  while [ "$wait_count" -lt 50 ]; do
    [ -z "$(listener_pids "$wait_port")" ] && return 0
    sleep 0.1
    wait_count=$((wait_count + 1))
  done
  return 1
}

check_port() {
  check_port_number=$1
  check_port_role=$2
  occupied_pids=$(listener_pids "$check_port_number")
  if [ -z "$occupied_pids" ]; then
    printf '%s\n' "✓ $check_port_role 端口 $check_port_number 可用 / port $check_port_number is available"
    return 0
  fi

  foreign_processes=''
  project_processes=''
  for occupied_pid in $occupied_pids; do
    occupied_command=$(process_command "$occupied_pid")
    if is_project_process "$occupied_pid"; then
      project_processes="$project_processes $occupied_pid"
      printf '%s\n' "发现本项目旧进程占用 ${check_port_number}：PID $occupied_pid · $occupied_command"
    else
      foreign_processes="$foreign_processes $occupied_pid"
      printf '%s\n' "端口 $check_port_number 被其他程序占用：PID $occupied_pid · $occupied_command" >&2
    fi
  done

  if [ -n "$foreign_processes" ]; then
    fail "不会终止其他程序。请释放端口 $check_port_number 后重试。 / Another program owns port $check_port_number; it was not stopped."
  fi
  if [ "$auto_stop" != '1' ]; then
    fail "检测到本项目旧进程。可先运行 'kill$project_processes'，或使用默认自动清理。 / A previous project process is still running."
  fi

  printf '%s\n' "正在停止本项目旧开发进程… / Stopping the previous project dev process…"
  for project_pid in $project_processes; do
    kill -TERM "$project_pid" 2>/dev/null || true
  done
  wait_for_port_release "$check_port_number" || fail "端口 $check_port_number 未在 5 秒内释放；请检查残留进程。 / Port $check_port_number was not released within 5 seconds."
  printf '%s\n' "✓ 已释放 $check_port_role 端口 $check_port_number / released port $check_port_number"
}

require_command node
require_command npm
require_command cargo
require_command lsof
require_command ps

[ -f "$project_root/package.json" ] || fail "找不到仓库根目录 package.json。 / Repository package.json was not found."
[ -f "$project_root/apps/desktop/src-tauri/tauri.conf.json" ] || fail "找不到 Tauri 配置。 / Tauri configuration was not found."
[ -d "$project_root/node_modules" ] || fail "尚未安装 npm 依赖；请先运行 'npm install'。 / npm dependencies are missing; run 'npm install' first."

validate_port "$dev_port"
check_port "$dev_port" "Vite"

if [ -n "${TAURI_DEV_HOST:-}" ]; then
  validate_port "$hmr_port"
  check_port "$hmr_port" "HMR"
fi

if [ "$check_only" = '1' ]; then
  printf '%s\n' "✓ 开发环境检查完成 / development preflight completed"
  exit 0
fi

cleanup_project_port() {
  cleanup_port_number=$1
  cleanup_pids=$(listener_pids "$cleanup_port_number")
  for cleanup_pid in $cleanup_pids; do
    if is_project_process "$cleanup_pid"; then
      kill -TERM "$cleanup_pid" 2>/dev/null || true
    fi
  done
  wait_for_port_release "$cleanup_port_number" || true
}

cleanup_started_dev() {
  trap - EXIT HUP INT TERM
  if [ -n "${dev_pid:-}" ]; then
    kill -TERM "$dev_pid" 2>/dev/null || true
    wait "$dev_pid" 2>/dev/null || true
  fi
  cleanup_project_port "$dev_port"
  if [ -n "${TAURI_DEV_HOST:-}" ]; then
    cleanup_project_port "$hmr_port"
  fi
}

trap cleanup_started_dev EXIT HUP INT TERM
printf '%s\n' "正在启动投稿舱 ManuscriptDock 开发环境… / Starting the ManuscriptDock development environment…"
cd "$project_root"
npm run dev &
dev_pid=$!
wait "$dev_pid"
