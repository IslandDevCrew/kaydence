#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-run}"
APP_NAME="kaydence"
APP_DISPLAY_NAME="Kaydence"
BUNDLE_ID="io.kaydence.app"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_FILE="${TMPDIR:-/tmp}/kaydence-tauri.log"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"
export CARGO_TARGET_DIR
BUILD_PROFILE="${KAYDENCE_BUILD_PROFILE:-release}"
case "$MODE" in
  --debug|debug) BUILD_PROFILE="debug" ;;
esac
case "$BUILD_PROFILE" in
  debug|release) ;;
  *) echo "unsupported build profile: $BUILD_PROFILE" >&2; exit 2 ;;
esac
APP_BINARY="$CARGO_TARGET_DIR/$BUILD_PROFILE/$APP_NAME"
PLATFORM="$(uname -s)"
BACKEND_FEATURES="custom-protocol,asr-whisper"
if [ "$PLATFORM" = "Darwin" ]; then
  BACKEND_FEATURES="custom-protocol,asr-whisper-metal"
fi
# whisper.cpp defaults to host-native instructions. Shipped binaries must remain
# portable; an architecture-specific release can still opt back in explicitly.
export GGML_NATIVE="${GGML_NATIVE:-OFF}"
WHISPER_BUILD_STAMP="$CARGO_TARGET_DIR/.kaydence-whisper-build-config-$BUILD_PROFILE"
WHISPER_BUILD_CONFIG=""
MACOS_APP_BUNDLE="${KAYDENCE_APP_BUNDLE_DIR:-$CARGO_TARGET_DIR/$BUILD_PROFILE/$APP_DISPLAY_NAME.app}"
MACOS_BUNDLE_BINARY="$MACOS_APP_BUNDLE/Contents/MacOS/$APP_NAME"
OPEN_BIN="${KAYDENCE_OPEN_BIN:-/usr/bin/open}"
CODESIGN_BIN="${KAYDENCE_CODESIGN_BIN:-/usr/bin/codesign}"
VERIFY_DELAY_SECONDS="${KAYDENCE_VERIFY_DELAY_SECONDS:-2}"

cd "$ROOT_DIR"

stop_existing() {
  pkill -x "$APP_NAME" >/dev/null 2>&1 || true
}

build_frontend() {
  pnpm --filter kaydence-desktop build
}

whisper_build_config() {
  printf 'BACKEND_FEATURES=%s\n' "$BACKEND_FEATURES"
  env | LC_ALL=C sort | grep -E '^(CC|CXX|GGML_[^=]*|WHISPER_[^=]*|CMAKE_[^=]*)='
}

prepare_whisper_build() {
  local previous_config=""
  WHISPER_BUILD_CONFIG="$(whisper_build_config)"
  if [ -f "$WHISPER_BUILD_STAMP" ]; then
    previous_config="$(cat "$WHISPER_BUILD_STAMP")"
  fi
  if [ "$previous_config" != "$WHISPER_BUILD_CONFIG" ]; then
    if [ "$BUILD_PROFILE" = "release" ]; then
      cargo clean --release --manifest-path apps/desktop/src-tauri/Cargo.toml -p whisper-rs-sys
    else
      cargo clean --manifest-path apps/desktop/src-tauri/Cargo.toml -p whisper-rs-sys
    fi
  fi
}

record_whisper_build_config() {
  mkdir -p "$CARGO_TARGET_DIR"
  printf '%s\n' "$WHISPER_BUILD_CONFIG" >"$WHISPER_BUILD_STAMP"
}

build_backend() {
  prepare_whisper_build
  if [ "$BUILD_PROFILE" = "release" ]; then
    cargo build --release --features "$BACKEND_FEATURES" --manifest-path apps/desktop/src-tauri/Cargo.toml
  else
    cargo build --features "$BACKEND_FEATURES" --manifest-path apps/desktop/src-tauri/Cargo.toml
  fi
  record_whisper_build_config
}

stage_macos_bundle() {
  local contents_dir="$MACOS_APP_BUNDLE/Contents"
  local resources_dir="$contents_dir/Resources"

  rm -rf "$MACOS_APP_BUNDLE"
  mkdir -p "$contents_dir/MacOS" "$resources_dir"
  cp "$APP_BINARY" "$MACOS_BUNDLE_BINARY"
  chmod +x "$MACOS_BUNDLE_BINARY"
  cp apps/desktop/src-tauri/icons/icon.icns "$resources_dir/icon.icns"

  cat >"$contents_dir/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDisplayName</key>
  <string>$APP_DISPLAY_NAME</string>
  <key>CFBundleExecutable</key>
  <string>$APP_NAME</string>
  <key>CFBundleIconFile</key>
  <string>icon.icns</string>
  <key>CFBundleIdentifier</key>
  <string>$BUNDLE_ID</string>
  <key>CFBundleName</key>
  <string>$APP_DISPLAY_NAME</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1.0</string>
  <key>CFBundleVersion</key>
  <string>0.1.0</string>
  <key>LSMinimumSystemVersion</key>
  <string>10.13</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSMicrophoneUsageDescription</key>
  <string>Kaydence needs microphone access to capture dictation on this Mac.</string>
  <key>NSPrincipalClass</key>
  <string>NSApplication</string>
</dict>
</plist>
PLIST

  "$CODESIGN_BIN" --force --deep --sign - --identifier "$BUNDLE_ID" "$MACOS_APP_BUNDLE" >/dev/null
}

build_app() {
  build_frontend
  build_backend
  if [ "$PLATFORM" = "Darwin" ]; then
    stage_macos_bundle
  fi
}

launch_app() {
  if [ "$PLATFORM" = "Darwin" ]; then
    "$OPEN_BIN" -n --stdout "$LOG_FILE" --stderr "$LOG_FILE" "$MACOS_APP_BUNDLE"
    APP_PID=""
    return
  fi

  nohup "$APP_BINARY" </dev/null >"$LOG_FILE" 2>&1 &
  APP_PID=$!
}

verify_app() {
  launch_app
  sleep "$VERIFY_DELAY_SECONDS"

  if [ "$PLATFORM" = "Darwin" ]; then
    APP_PID="$(pgrep -x "$APP_NAME" | tail -n 1 || true)"
    if [ -n "$APP_PID" ] && kill -0 "$APP_PID" >/dev/null 2>&1; then
      echo "Kaydence is running (pid $APP_PID)"
      return 0
    fi

    echo "Kaydence did not remain running after LaunchServices opened $MACOS_APP_BUNDLE." >&2
    tail -n 40 "$LOG_FILE" >&2 || true
    return 1
  fi

  if kill -0 "$APP_PID" >/dev/null 2>&1; then
    echo "Kaydence is running (pid $APP_PID)"
    return 0
  fi

  local exit_status
  if wait "$APP_PID"; then
    exit_status=0
  else
    exit_status=$?
  fi

  echo "Kaydence exited before verification (status $exit_status)." >&2
  tail -n 40 "$LOG_FILE" >&2 || true
  return 1
}

stop_existing

case "$MODE" in
  run)
    build_app
    if [ "$PLATFORM" = "Darwin" ]; then
      exec "$OPEN_BIN" -W -n --stdout "$LOG_FILE" --stderr "$LOG_FILE" "$MACOS_APP_BUNDLE"
    fi
    exec "$APP_BINARY"
    ;;
  --build|build)
    build_app
    ;;
  --debug|debug)
    build_app
    if [ "$PLATFORM" = "Darwin" ]; then
      lldb -- "$MACOS_BUNDLE_BINARY"
    else
      lldb -- "$APP_BINARY"
    fi
    ;;
  --logs|logs)
    build_app
    if [ "$PLATFORM" = "Darwin" ]; then
      launch_app
      tail -f "$LOG_FILE"
    else
      "$APP_BINARY" 2>&1 | tee "$LOG_FILE"
    fi
    ;;
  --telemetry|telemetry)
    build_app
    launch_app
    /usr/bin/log stream --info --style compact --predicate 'process CONTAINS "kaydence"'
    ;;
  --verify|verify)
    build_app
    verify_app
    ;;
  *)
    echo "usage: $0 [run|build|--debug|--logs|--telemetry|--verify]" >&2
    exit 2
    ;;
esac
