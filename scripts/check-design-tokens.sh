#!/usr/bin/env bash
# check-design-tokens.sh — design-relock gate: single-token-source invariants.
#
# Enforces two rules from docs/design/DESIGN_LANGUAGE_V2_LOCK.md:
#   1. View CSS never redeclares a core semantic token or a local :root scope
#      — styles.css is the single token source (--no-view-token-redeclare).
#   2. View CSS/TSX never hardcodes a hex color literal as a property value
#      or inline style — colors must flow through tokens (--no-hardcoded-hex).
#
# Portable to bash 3.2: no mapfile, no associative arrays. Uses a scratch
# file per check (cleaned up on exit) instead of subshell-scoped counters,
# matching the dependency-free spirit of scripts/audit-network.sh.
#
#   usage: bash scripts/check-design-tokens.sh --no-view-token-redeclare
#          bash scripts/check-design-tokens.sh --no-hardcoded-hex
#          bash scripts/check-design-tokens.sh --no-view-token-redeclare --no-hardcoded-hex
#          bash scripts/check-design-tokens.sh --no-view-token-redeclare --dir apps/desktop/src/views
#          bash scripts/check-design-tokens.sh --no-hardcoded-hex --file apps/desktop/src/views/DictateView.css
#
# Exit 0 only if every requested check passes. Exit 1 on any violation.
# Exit 2 on usage error (no check flag given, or a bad --dir/--file target).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEFAULT_DIR="$ROOT/apps/desktop/src/views"

RUN_TOKEN_REDECLARE=0
RUN_HARDCODED_HEX=0
SCOPE_DIR=""
SCOPE_FILE=""

usage() {
  cat >&2 <<'EOF'
usage: check-design-tokens.sh [--no-view-token-redeclare] [--no-hardcoded-hex]
                               [--dir <path>] [--file <path>]
At least one of --no-view-token-redeclare / --no-hardcoded-hex is required.
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --no-view-token-redeclare) RUN_TOKEN_REDECLARE=1 ;;
    --no-hardcoded-hex) RUN_HARDCODED_HEX=1 ;;
    --dir)
      shift
      [ $# -gt 0 ] || { echo "ERROR: --dir requires a path" >&2; exit 2; }
      SCOPE_DIR="$1"
      ;;
    --file)
      shift
      [ $# -gt 0 ] || { echo "ERROR: --file requires a path" >&2; exit 2; }
      SCOPE_FILE="$1"
      ;;
    -h|--help) usage; exit 0 ;;
    *) echo "ERROR: unknown flag: $1" >&2; usage; exit 2 ;;
  esac
  shift
done

if [ "$RUN_TOKEN_REDECLARE" -eq 0 ] && [ "$RUN_HARDCODED_HEX" -eq 0 ]; then
  usage
  exit 2
fi

if [ -n "$SCOPE_FILE" ] && [ -n "$SCOPE_DIR" ]; then
  echo "ERROR: pass only one of --dir or --file" >&2
  exit 2
fi

# Resolve the file list to scan. Paths are stored newline-delimited in a
# scratch file (bash-3.2-portable stand-in for an array).
WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/kaydence-design-tokens.XXXXXX")"
trap 'rm -rf "$WORKDIR"' EXIT

FILELIST="$WORKDIR/files.txt"
VIOLATIONS="$WORKDIR/violations.txt"
: > "$FILELIST"
: > "$VIOLATIONS"

if [ -n "$SCOPE_FILE" ]; then
  TARGET="$SCOPE_FILE"
  case "$TARGET" in /*) : ;; *) TARGET="$ROOT/$TARGET" ;; esac
  if [ ! -f "$TARGET" ]; then
    echo "ERROR: --file target not found: $SCOPE_FILE" >&2
    exit 2
  fi
  printf '%s\n' "$TARGET" > "$FILELIST"
elif [ -n "$SCOPE_DIR" ]; then
  TARGET="$SCOPE_DIR"
  case "$TARGET" in /*) : ;; *) TARGET="$ROOT/$TARGET" ;; esac
  if [ ! -d "$TARGET" ]; then
    echo "ERROR: --dir target not found: $SCOPE_DIR" >&2
    exit 2
  fi
  find "$TARGET" -type f \( -name '*.css' -o -name '*.tsx' \) | sort > "$FILELIST"
else
  find "$DEFAULT_DIR" -type f \( -name '*.css' -o -name '*.tsx' \) | sort > "$FILELIST"
fi

echo "== Kaydence design-token gate =="
echo "root: $ROOT"
echo "scanning: $(wc -l < "$FILELIST" | tr -d ' ') file(s)"

if [ "$RUN_TOKEN_REDECLARE" -eq 1 ]; then
  echo "-- --no-view-token-redeclare --"
  CORE_TOKENS='--(bg|surface|surface-soft|ink|muted|line|success|warn|danger)[[:space:]]*:'

  while IFS= read -r f; do
    [ -z "$f" ] && continue
    case "$f" in */styles.css) continue ;; esac   # the one allowed token home
    case "$f" in *.tsx) continue ;; esac           # this rule is CSS-only

    grep -nE '^[[:space:]]*:root([^-]|$)' "$f" 2>/dev/null | while IFS=: read -r num rest; do
      echo "${f#"$ROOT"/}:$num: local :root selector (styles.css is the single token source)" >> "$VIOLATIONS"
    done || true

    grep -nE "$CORE_TOKENS" "$f" 2>/dev/null | while IFS=: read -r num rest; do
      echo "${f#"$ROOT"/}:$num: redeclares a core token outside styles.css ->${rest}" >> "$VIOLATIONS"
    done || true
  done < "$FILELIST"

  redeclare_count="$(wc -l < "$VIOLATIONS" | tr -d ' ')"
  if [ "$redeclare_count" -gt 0 ]; then
    sed -n '1,200p' "$VIOLATIONS" | sed 's/^/  VIOLATION: /'
    echo "RESULT (--no-view-token-redeclare): FAIL ($redeclare_count violation(s))"
    TOKEN_REDECLARE_FAIL=1
  else
    echo "RESULT (--no-view-token-redeclare): PASS"
    TOKEN_REDECLARE_FAIL=0
  fi
  : > "$VIOLATIONS"
else
  TOKEN_REDECLARE_FAIL=0
fi

if [ "$RUN_HARDCODED_HEX" -eq 1 ]; then
  echo "-- --no-hardcoded-hex --"
  # Fixed hex-literal shapes: #abc #abcd #aabbcc #aabbccdd, as a value token
  # (preceded by boundary punctuation/whitespace, not part of a longer word).
  HEX_RE='#[0-9a-fA-F]{3,8}([0-9a-fA-F]|[^0-9a-zA-Z])'

  while IFS= read -r f; do
    [ -z "$f" ] && continue
    case "$f" in */styles.css) continue ;; esac   # token declarations live here by design

    grep -nE "$HEX_RE" "$f" 2>/dev/null | while IFS=: read -r num rest; do
      # Skip whole-line comments (best-effort, line-based — matches the
      # audit-network.sh precedent of pragmatic grep over a real parser).
      trimmed="${rest#"${rest%%[![:space:]]*}"}"
      case "$trimmed" in
        '/*'*|'//'*) continue ;;
      esac
      # Skip a hex literal sitting inside a viewBox attribute — those are
      # numeric coordinate lists, never colors, and can coincidentally
      # contain a `#` when adjacent markup wraps onto the same line.
      case "$rest" in *viewBox=*)
        case "$rest" in *'"'*"#"*'"'*) : ;; esac
        ;;
      esac
      echo "${f#"$ROOT"/}:$num: hardcoded hex literal ->${rest}" >> "$VIOLATIONS"
    done || true
  done < "$FILELIST"

  hex_count="$(wc -l < "$VIOLATIONS" | tr -d ' ')"
  if [ "$hex_count" -gt 0 ]; then
    sed -n '1,200p' "$VIOLATIONS" | sed 's/^/  VIOLATION: /'
    echo "RESULT (--no-hardcoded-hex): FAIL ($hex_count violation(s))"
    HARDCODED_HEX_FAIL=1
  else
    echo "RESULT (--no-hardcoded-hex): PASS"
    HARDCODED_HEX_FAIL=0
  fi
else
  HARDCODED_HEX_FAIL=0
fi

if [ "$TOKEN_REDECLARE_FAIL" -eq 1 ] || [ "$HARDCODED_HEX_FAIL" -eq 1 ]; then
  exit 1
fi
echo "RESULT: PASS"
exit 0
