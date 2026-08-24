#!/bin/sh
set -eu

if command -v brew >/dev/null 2>&1; then
  build_llvm_bin="$(brew --prefix llvm)/bin"
  build_lld_bin="$(brew --prefix lld)/bin"
  PATH="$build_llvm_bin:$build_lld_bin:$PATH"
  export PATH
fi

for build_tool in cargo-xwin llvm-rc lld-link makensis; do
  if ! command -v "$build_tool" >/dev/null 2>&1; then
    echo "Missing Windows cross-build tool: $build_tool" >&2
    exit 1
  fi
done

npm run tauri -- build \
  --runner cargo-xwin \
  --target x86_64-pc-windows-msvc \
  --bundles nsis
