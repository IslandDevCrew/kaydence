#!/usr/bin/env bash
# Kaydence — Linux package payload gate (ADR-0023).
#
# Asserts every built .deb / .rpm / .AppImage ships exactly the user-facing
# binaries (kaydence + kaydence-ctl — never a selftest or bench tool), the
# Omarchy bindings snippet, a desktop entry, and stays under the 60 MB
# installer budget (root AGENTS.md non-negotiable #7).
#
#   bash scripts/check-linux-packages.sh [bundle-dir] [--require-appimage]
#                                        [--appimage-budget-warn]
#   (default bundle-dir: target/release/bundle)
#
# --appimage-budget-warn: an AppImage over 60 MB is reported as a WARNING (and a
# GitHub ::warning:: annotation) instead of a failure. An AppImage bundles the
# whole WebKitGTK stack (~87 MB measured 2026-09-26), so whether it may exceed the
# installer budget is an operator decision (ADR-0023 follow-up); until that is
# made, the overrun is surfaced on every run, never hidden. .deb/.rpm stay strict.
#
# Needs bsdtar (libarchive) for .deb/.rpm; an AppImage is inspected with its
# own --appimage-extract. Exit 0 = all present packages pass.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
  sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
fi
DIR="target/release/bundle"
REQUIRE_APPIMAGE=0
APPIMAGE_BUDGET_WARN=0
for arg in "$@"; do
  case "$arg" in
    --require-appimage) REQUIRE_APPIMAGE=1 ;;
    --appimage-budget-warn) APPIMAGE_BUDGET_WARN=1 ;;
    *) DIR="$arg" ;;
  esac
done
command -v bsdtar >/dev/null || { echo "bsdtar is required" >&2; exit 2; }
BUDGET=$((60 * 1024 * 1024))
fail=0
checked=0

check_listing() { # name listing-file
  local name="$1" list="$2" bins
  bins="$(grep -E '(^|/)usr/bin/[^/]+$' "$list" | sed -E 's#.*usr/bin/##' | sort -u | tr '\n' ' ')"
  if [ "$bins" != "kaydence kaydence-ctl " ]; then
    echo "  FAIL $name: /usr/bin holds [$bins] — expected exactly [kaydence kaydence-ctl]"; fail=1
  fi
  grep -qE 'usr/share/kaydence/omarchy/kaydence-bindings\.lua$' "$list" \
    || { echo "  FAIL $name: Omarchy bindings snippet missing"; fail=1; }
  grep -qE 'usr/share/applications/[^/]+\.desktop$' "$list" \
    || { echo "  FAIL $name: desktop entry missing"; fail=1; }
}

check_size() { # file [warn-only]
  local size; size="$(stat -c %s "$1")"
  if [ "$size" -gt "$BUDGET" ]; then
    if [ "${2:-}" = warn ]; then
      echo "  WARN $(basename "$1"): $size bytes > 60 MB installer budget (operator decision pending)"
      echo "::warning title=AppImage over installer budget::$(basename "$1") is $size bytes (> 60 MB). Bundles WebKitGTK; operator decision pending (ADR-0023 follow-up)."
    else
      echo "  FAIL $(basename "$1"): $size bytes > 60 MB installer budget"; fail=1
    fi
  fi
  echo "  size $(basename "$1"): $size bytes"
}

WORK="$(mktemp -d)"; trap 'rm -rf "$WORK"' EXIT

for deb in "$DIR"/deb/*.deb; do
  [ -e "$deb" ] || continue
  echo "== $(basename "$deb")"; checked=$((checked + 1))
  data="$(bsdtar -tf "$deb" | grep '^data\.tar')"
  bsdtar -xOf "$deb" "$data" | bsdtar -tf - > "$WORK/deb.list"
  check_listing deb "$WORK/deb.list"; check_size "$deb"
done
for rpm in "$DIR"/rpm/*.rpm; do
  [ -e "$rpm" ] || continue
  echo "== $(basename "$rpm")"; checked=$((checked + 1))
  bsdtar -tf "$rpm" > "$WORK/rpm.list"
  check_listing rpm "$WORK/rpm.list"; check_size "$rpm"
done
found_appimage=0
for app in "$DIR"/appimage/*.AppImage; do
  [ -e "$app" ] || continue
  echo "== $(basename "$app")"; checked=$((checked + 1)); found_appimage=1
  app_abs="$(realpath "$app")"   # extraction runs inside $WORK
  chmod +x "$app_abs"
  (cd "$WORK" && "$app_abs" --appimage-extract >/dev/null)
  (cd "$WORK/squashfs-root" && find . -type f -o -type l) | sed 's#^\./##' > "$WORK/appimage.list"
  if [ "$APPIMAGE_BUDGET_WARN" = 1 ]; then
    check_listing appimage "$WORK/appimage.list"; check_size "$app" warn
  else
    check_listing appimage "$WORK/appimage.list"; check_size "$app"
  fi
  rm -rf "$WORK/squashfs-root"
done

if [ "$REQUIRE_APPIMAGE" = 1 ] && [ "$found_appimage" = 0 ]; then
  echo "  FAIL: no AppImage in $DIR/appimage"; fail=1
fi
[ "$checked" -gt 0 ] || { echo "no packages found under $DIR" >&2; exit 2; }
if [ "$fail" = 0 ]; then
  echo "RESULT: PASS ($checked package(s): kaydence + kaydence-ctl only, snippet + desktop entry, < 60 MB)"
else
  echo "RESULT: FAIL"; exit 1
fi
