#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/kaydence-run-test.XXXXXX")"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

mkdir -p "$TMP_DIR/bin"

cat >"$TMP_DIR/bin/cargo" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

if [ "${1:-}" = "build" ]; then
  profile="debug"
  for argument in "$@"; do
    if [ "$argument" = "--release" ]; then
      profile="release"
    fi
  done
  printf '%s\n' "$*" >"$FAKE_CARGO_MARKER"
  mkdir -p "$CARGO_TARGET_DIR/$profile"
  if [ "${FAKE_APP_BEHAVIOR:-crash}" = "live" ]; then
    cat >"$CARGO_TARGET_DIR/$profile/kaydence" <<'APP'
#!/usr/bin/env bash
/bin/sleep 5
APP
  else
    cat >"$CARGO_TARGET_DIR/$profile/kaydence" <<'APP'
#!/usr/bin/env bash
exit 17
APP
  fi
  chmod +x "$CARGO_TARGET_DIR/$profile/kaydence"
  exit 0
fi

exit 17
SCRIPT

cat >"$TMP_DIR/bin/pnpm" <<'SCRIPT'
#!/usr/bin/env bash
exit 0
SCRIPT

cat >"$TMP_DIR/bin/pkill" <<'SCRIPT'
#!/usr/bin/env bash
exit 0
SCRIPT

cat >"$TMP_DIR/bin/codesign" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >"$FAKE_CODESIGN_MARKER"
SCRIPT

cat >"$TMP_DIR/bin/open" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

app_bundle=""
for argument in "$@"; do
  case "$argument" in
    *.app) app_bundle="$argument" ;;
  esac
done

if [ -z "$app_bundle" ]; then
  exit 18
fi

nohup "$app_bundle/Contents/MacOS/kaydence" </dev/null >/dev/null 2>&1 &
echo "$!" >"$FAKE_APP_PID_FILE"
SCRIPT

cat >"$TMP_DIR/bin/pgrep" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

if [ "${1:-}" != "-x" ] || [ "${2:-}" != "kaydence" ]; then
  exit 19
fi

pid="$(cat "$FAKE_APP_PID_FILE")"
kill -0 "$pid" >/dev/null 2>&1
echo "$pid"
SCRIPT

cat >"$TMP_DIR/bin/sleep" <<'SCRIPT'
#!/usr/bin/env bash
if [ "${1:-}" = "8" ]; then
  exit 0
fi
/bin/sleep "$@"
SCRIPT

chmod +x "$TMP_DIR/bin/"*

run_verify() {
  local behavior="$1"
  local output_file="$TMP_DIR/$behavior.log"
  local status

  set +e
  PATH="$TMP_DIR/bin:/usr/bin:/bin" \
    CARGO_TARGET_DIR="$TMP_DIR/target-$behavior" \
    FAKE_APP_BEHAVIOR="$behavior" \
    FAKE_APP_PID_FILE="$TMP_DIR/pid-$behavior" \
    FAKE_CARGO_MARKER="$TMP_DIR/cargo-$behavior" \
    FAKE_CODESIGN_MARKER="$TMP_DIR/codesign-$behavior" \
    KAYDENCE_CODESIGN_BIN="$TMP_DIR/bin/codesign" \
    KAYDENCE_OPEN_BIN="$TMP_DIR/bin/open" \
    KAYDENCE_VERIFY_DELAY_SECONDS="0.1" \
    "$ROOT_DIR/script/build_and_run.sh" --verify >"$output_file" 2>&1
  status=$?
  set -e

  printf '%s' "$status"
}

crash_status="$(run_verify crash)"
if [ "$crash_status" -eq 0 ]; then
  echo "FAIL: --verify accepted a launcher that exited immediately" >&2
  exit 1
fi

live_status="$(run_verify live)"
if [ "$live_status" -ne 0 ]; then
  echo "FAIL: --verify rejected a running Kaydence child process" >&2
  cat "$TMP_DIR/live.log" >&2
  exit 1
fi

expected_asr_feature="asr-whisper"
if [ "$(uname -s)" = "Darwin" ]; then
  expected_asr_feature="asr-whisper-metal"
fi
if ! grep -q -- "--features custom-protocol,$expected_asr_feature" "$TMP_DIR/cargo-live"; then
  echo "FAIL: release run path did not compile the platform ASR adapter" >&2
  exit 1
fi

if [ "$(uname -s)" = "Darwin" ]; then
  app_bundle="$TMP_DIR/target-live/release/Kaydence.app"
  if ! grep -q -- '--release' "$TMP_DIR/cargo-live"; then
    echo "FAIL: macOS run path did not build the self-contained release profile" >&2
    exit 1
  fi
  if ! grep -q -- '--features custom-protocol,asr-whisper-metal' "$TMP_DIR/cargo-live"; then
    echo "FAIL: macOS run path did not enable Tauri's embedded custom protocol" >&2
    exit 1
  fi
  if [ ! -x "$app_bundle/Contents/MacOS/kaydence" ]; then
    echo "FAIL: macOS build did not stage an executable Kaydence.app bundle" >&2
    exit 1
  fi
  if ! grep -q '<string>io.kaydence.app</string>' "$app_bundle/Contents/Info.plist"; then
    echo "FAIL: staged Kaydence.app is missing its bundle identifier" >&2
    exit 1
  fi
  if ! grep -q -- '--identifier io.kaydence.app' "$TMP_DIR/codesign-live"; then
    echo "FAIL: staged Kaydence.app was not signed with its bundle identifier" >&2
    exit 1
  fi
fi

live_pid="$(sed -n 's/.*pid \([0-9][0-9]*\).*/\1/p' "$TMP_DIR/live.log")"
if [ -z "$live_pid" ]; then
  echo "FAIL: --verify did not report the launched child pid" >&2
  exit 1
fi

kill -HUP "$live_pid"
/bin/sleep 0.1
if ! kill -0 "$live_pid" >/dev/null 2>&1; then
  echo "FAIL: verified Kaydence process did not survive launcher hangup" >&2
  exit 1
fi
kill "$live_pid" >/dev/null 2>&1 || true

echo "PASS: build-and-run verification tracks the launched child process"
