#!/usr/bin/env bash
# check-frontend.sh — P0-G4 gate: frontend typecheck + lint.
#
# Runs the locally-installed tsc + eslint binaries directly, bypassing
# `pnpm run`/`exec` (whose pnpm-11 pre-run deps-status check errors on
# not-yet-approved dependency build scripts like esbuild's, unrelated to code
# quality). Run `pnpm install` first. Portable across pnpm versions.
#   usage: bash scripts/check-frontend.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/apps/desktop"
BIN="$APP/node_modules/.bin"
if [ ! -x "$BIN/tsc" ]; then
  echo "ERROR: frontend deps not installed. Run: pnpm install" >&2
  exit 2
fi
echo "== tsc --noEmit =="
( cd "$APP" && "$BIN/tsc" --noEmit )
echo "== eslint (max-warnings 0) =="
( cd "$APP" && "$BIN/eslint" . --max-warnings 0 )
echo "RESULT: PASS (frontend typechecks + lints clean)"
