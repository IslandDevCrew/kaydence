#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-run}"
APP_NAME="kaydence"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_FILE="${TMPDIR:-/tmp}/kaydence-tauri.log"

cd "$ROOT_DIR"

stop_existing() {
  pkill -x "$APP_NAME" >/dev/null 2>&1 || true
  pkill -f "cargo tauri dev" >/dev/null 2>&1 || true
  pkill -f "vite --host" >/dev/null 2>&1 || true
}

build_frontend() {
  pnpm --filter kaydence-desktop build
}

build_backend() {
  cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml
}

run_dev() {
  cargo tauri dev
}

stop_existing

case "$MODE" in
  run)
    run_dev
    ;;
  --build|build)
    build_frontend
    build_backend
    ;;
  --debug|debug)
    build_frontend
    build_backend
    lldb -- target/debug/kaydence
    ;;
  --logs|logs)
    run_dev >"$LOG_FILE" 2>&1 &
    tail -f "$LOG_FILE"
    ;;
  --telemetry|telemetry)
    run_dev >"$LOG_FILE" 2>&1 &
    /usr/bin/log stream --info --style compact --predicate 'process CONTAINS "kaydence"'
    ;;
  --verify|verify)
    run_dev >"$LOG_FILE" 2>&1 &
    sleep 8
    pgrep -if "kaydence|cargo tauri dev" >/dev/null
    ;;
  *)
    echo "usage: $0 [run|build|--debug|--logs|--telemetry|--verify]" >&2
    exit 2
    ;;
esac
