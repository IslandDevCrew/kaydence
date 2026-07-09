#!/usr/bin/env bash
# check-privacy-posture.sh — release gate for Kaydence's public privacy claims.
#
# This combines the source-level network audit with a few hard assertions that
# the README keeps the plain-language promises from PRD P0-7. It also scans app
# source for screen-capture implementation primitives, which are permanently
# banned by the root charter and ADR-0006.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
README="$ROOT/README.md"

usage() {
  cat <<'USAGE'
usage: bash scripts/check-privacy-posture.sh [--check|--help]

Runs the P1 privacy posture gate:
  - scripts/audit-network.sh --check
  - README plain-language privacy promise assertions
  - source scan for banned screen-capture implementation primitives
USAGE
}

case "${1:---check}" in
  --check) ;;
  --help|-h) usage; exit 0 ;;
  *) usage >&2; exit 2 ;;
esac

echo "== Kaydence privacy posture =="
echo "root: $ROOT"

bash "$ROOT/scripts/audit-network.sh" --check

if [ ! -f "$README" ]; then
  echo "ERROR: missing README.md" >&2
  exit 2
fi

required_readme_terms=(
  "No telemetry."
  "No account."
  "No cloud screen capture, ever."
  "No default network egress."
  "Your history is on your machine."
  "Context is memory-only."
)

missing=0
for term in "${required_readme_terms[@]}"; do
  if ! grep -Fq "$term" "$README"; then
    echo "MISSING README PRIVACY CLAIM: $term" >&2
    missing=$((missing + 1))
  fi
done

if [ "$missing" -gt 0 ]; then
  echo "RESULT: FAIL ($missing README privacy promise(s) missing)" >&2
  exit 1
fi

# Implementation primitives only. Plain text policy copy may mention screen
# capture; these tokens indicate code paths that could actually capture pixels.
banned_source_patterns=(
  "getDisplayMedia"
  "desktopCapturer"
  "screencapture"
  "CGWindowListCreateImage"
  "CGDisplayStream"
  "Windows.Graphics.Capture"
  "IDXGIOutputDuplication"
  "xcb_shm_get_image"
  "zwlr_screencopy"
)

grep_args=""
for pattern in "${banned_source_patterns[@]}"; do
  grep_args="$grep_args -e $pattern"
done

# shellcheck disable=SC2086
hits="$(cd "$ROOT" && grep -rnF $grep_args apps crates \
  --include='*.rs' --include='*.ts' --include='*.tsx' \
  2>/dev/null | grep -vE '/(node_modules|target|dist)/' || true)"

if [ -n "$hits" ]; then
  echo "$hits" | sed 's/^/  BANNED SCREEN-CAPTURE PRIMITIVE: /' >&2
  echo "RESULT: FAIL (screen capture is permanently banned)" >&2
  exit 1
fi

echo "RESULT: PASS (README promises present; network audit clean; no screen-capture primitives)"
