#!/usr/bin/env bash
# audit-network.sh — the privacy egress gate (non-negotiable #1).
#
# Scans the Rust + TS source for network-capable calls and fails if any live
# outside the reviewed allowlist (scripts/network-allowlist.json). A new network
# surface that isn't in the allowlist is a release blocker — it must first land as
# an ADR + user toggle, then be added here with its justification.
#
# This is a SOURCE-LEVEL audit (greps the tree). A deeper binary/deps audit hooks
# in once the app builds (TODO: cargo-geiger / cargo tree scan + symbol grep).
#
# Runs on all three OSes (pure grep). CI wires it as a gate once source exists.
#   usage: bash scripts/audit-network.sh [--check]
#     --check : exit non-zero on any unreviewed hit (CI mode). Default same.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ALLOWLIST="$ROOT/scripts/network-allowlist.json"

# Network-capable primitives we care about. Presence isn't automatically a
# violation — it must be justified in the allowlist with a file glob + reason.
PATTERNS=(
  'reqwest' 'hyper::' 'ureq' 'isahc' 'surf' 'curl' 'libcurl'
  'TcpStream' 'TcpListener' 'UdpSocket' 'std::net' 'tokio::net'
  'reqwest::Client' 'ws://' 'wss://' 'http://' 'https://'
  'fetch(' 'XMLHttpRequest' 'WebSocket' 'navigator.sendBeacon' 'EventSource'
  'axios' 'got(' 'node-fetch'
)

SCAN_DIRS=("apps" "crates")
EXCLUDES=(':!*/node_modules/*' ':!*/target/*' ':!*/dist/*')

echo "== Kaydence network audit =="
echo "root: $ROOT"

# No source yet? That is a clean pass — nothing can leak.
have_src=0
for d in "${SCAN_DIRS[@]}"; do
  if [ -d "$ROOT/$d" ] && find "$ROOT/$d" -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' \) \
       -not -path '*/node_modules/*' -not -path '*/target/*' | grep -q .; then
    have_src=1
  fi
done
if [ "$have_src" -eq 0 ]; then
  echo "no Rust/TS source under ${SCAN_DIRS[*]} yet — nothing to audit (clean pass)."
  echo "RESULT: PASS (0 network call sites; 0 unreviewed)"
  exit 0
fi

if [ ! -f "$ALLOWLIST" ]; then
  echo "ERROR: missing $ALLOWLIST — the audit needs a reviewed allowlist to diff against." >&2
  exit 2
fi

# Scan with FIXED-STRING matching (-F -e): the patterns contain regex-meta chars
# (e.g. 'fetch(', 'std::net', 'ws://') that would break an ERE alternation and
# make grep error out — a privacy gate must never silently pass on a grep failure.
grep_args=""
for p in "${PATTERNS[@]}"; do grep_args="$grep_args -e $p"; done
# shellcheck disable=SC2086
hits="$(cd "$ROOT" && grep -rnF $grep_args "${SCAN_DIRS[@]}" \
        --include='*.rs' --include='*.ts' --include='*.tsx' \
        2>/dev/null | grep -vE '/(node_modules|target|dist)/' || true)"

count="$(printf '%s' "$hits" | grep -c . || true)"
echo "network-capable call sites found: $count"

if [ "$count" -eq 0 ]; then
  echo "RESULT: PASS (0 network call sites; 0 unreviewed)"
  exit 0
fi

# Each hit must match an allowlist glob. Allowlist entries: {"file":"glob","reason":"..."}.
# Minimal dependency-free matcher: a hit is allowed if its path contains any
# allowlist file-substring (globs simplified to substrings for portability).
# Portable to bash 3.2: no mapfile — read into a newline-delimited string.
globs="$(grep -oE '"file"[[:space:]]*:[[:space:]]*"[^"]*"' "$ALLOWLIST" \
  | sed -E 's/.*"file"[[:space:]]*:[[:space:]]*"([^"]*)".*/\1/' | sed 's/\*//g')"

# A hit is allowed if its path contains any allowlist substring. `allowed()` is
# reused for the printed report and the exit-code count.
allowed() {
  _path="$1"
  printf '%s\n' "$globs" | while IFS= read -r g; do
    [ -n "$g" ] && case "$_path" in *"$g"*) echo yes; return;; esac
  done
}

echo "$hits" | while IFS= read -r line; do
  [ -z "$line" ] && continue
  [ -z "$(allowed "${line%%:*}")" ] && echo "  UNREVIEWED: $line"
done

# Count unreviewed for the exit code (subshell above can't set a parent var).
unreviewed="$(echo "$hits" | while IFS= read -r line; do
  [ -z "$line" ] && continue
  [ -z "$(allowed "${line%%:*}")" ] && echo x
done | grep -c . || true)"

if [ "$unreviewed" -gt 0 ]; then
  echo "RESULT: FAIL ($unreviewed unreviewed network call site(s)) — add an ADR + toggle, then allowlist." >&2
  exit 1
fi
echo "RESULT: PASS ($count call site(s), all in allowlist)"
exit 0
