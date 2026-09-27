#!/usr/bin/env bash
# Kaydence — Linux end-to-end dictation proof on Hyprland (ADR-0023).
#
#   speech → PipeWire → WAL → VAD → whisper.cpp ASR → cleanup → focus binding
#          → zwp_virtual_keyboard_v1 → the focused app        (byte readback)
#
# The REAL app does all of it, driven exactly like an Omarchy keybinding would
# (`kaydence-ctl record start` / `stop`). Nothing of the operator's is touched:
#   - speech comes from a TEMPORARY PipeWire null sink + remapped source,
#     routed to the app process only (PIPEWIRE_NODE); both removed on exit;
#     the operator's microphone and default devices are never used or changed;
#   - the app runs against a throwaway XDG_DATA_HOME (own settings, models,
#     history), so the operator's Kaydence data is untouched;
#   - text lands in a throwaway foot window focused through Hyprland IPC.
#
#   KAYDENCE_WHISPER_MODEL=/path/ggml-base.en-q5_1.bin \
#   KAYDENCE_E2E_CLIP=/path/speech-16k-mono.wav \
#   KAYDENCE_E2E_EXPECT="words that must appear" \
#   bash scripts/linux-e2e-dictation-proof.sh
#
# Needs a release build with the shipped Linux ASR feature:
#   cargo build --release --features custom-protocol,asr-whisper \
#     --manifest-path apps/desktop/src-tauri/Cargo.toml --bin kaydence --bin kaydence-ctl
# (build the frontend first: pnpm --filter kaydence-desktop build).
set -euo pipefail
cd "$(dirname "$0")/.."

if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
  sed -n '2,24p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
fi
fail() { echo "[e2e] FAIL: $*" >&2; exit 1; }
now_ms() { date +%s%3N; }

MODEL="${KAYDENCE_WHISPER_MODEL:-}"
CLIP="${KAYDENCE_E2E_CLIP:-}"
EXPECT="${KAYDENCE_E2E_EXPECT:-}"
MODEL_ID=whisper-base-en-q5_1
APP=target/release/kaydence
CTL=target/release/kaydence-ctl
[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ] || fail "not a Hyprland session"
for t in foot jq pactl pw-play sha256sum; do command -v "$t" >/dev/null || fail "$t is required"; done
# A locked session (Omarchy/Quickshell ext-session-lock) keeps keyboard focus on
# the lock screen: refuse up front with a clear reason instead of a focus miss.
if hyprctl -j monitors | jq -e 'any(.[]; (.solitaryBlockedBy // []) | index("LOCK"))' >/dev/null 2>&1; then
  fail "the session is locked — unlock it and rerun (the lock screen holds keyboard focus)"
fi
[ -x "$APP" ] && [ -x "$CTL" ] || fail "build the release app + ctl first (see header)"
[ -f "$MODEL" ] && [ -f "$CLIP" ] && [ -n "$EXPECT" ] || fail "set KAYDENCE_WHISPER_MODEL, KAYDENCE_E2E_CLIP, KAYDENCE_E2E_EXPECT"
WANT_SHA="$(jq -r --arg id "$MODEL_ID" '.models[] | select(.id == $id) | .sha256' models/registry.json)"
[ "$(sha256sum "$MODEL" | cut -d' ' -f1)" = "$WANT_SHA" ] || fail "model does not match the registry SHA-256"
"$CTL" record status >/dev/null 2>&1 && fail "a Kaydence instance is already running; quit it first"

WORK="$(mktemp -d)"
DATA="$WORK/data"
mkdir -p "$DATA/io.kaydence.app/models"
ln -s "$(realpath "$MODEL")" "$DATA/io.kaydence.app/models/ggml-base.en-q5_1.bin"
printf '{"schema_version":1,"selected_asr_model_id":"%s"}\n' "$MODEL_ID" \
  > "$DATA/io.kaydence.app/settings.json"

SINK_MOD="" SRC_MOD="" APP_PID="" FOOT_PID=""
PREV_FOCUS="$(hyprctl activewindow -j | jq -r '.address // empty')"
cleanup() {
  [ -n "$FOOT_PID" ] && kill "$FOOT_PID" 2>/dev/null || true
  [ -n "$APP_PID" ] && kill "$APP_PID" 2>/dev/null || true
  [ -n "$SRC_MOD" ] && pactl unload-module "$SRC_MOD" 2>/dev/null || true
  [ -n "$SINK_MOD" ] && pactl unload-module "$SINK_MOD" 2>/dev/null || true
  [ -n "$PREV_FOCUS" ] && hyprctl dispatch "hl.dsp.focus({ window = \"address:$PREV_FOCUS\" })" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

SINK_MOD="$(pactl load-module module-null-sink sink_name=kaydence_e2e rate=16000 channels=1 \
  sink_properties=device.description=Kaydence-E2E-sink)"
SRC_MOD="$(pactl load-module module-remap-source master=kaydence_e2e.monitor \
  source_name=kaydence_e2e_mic channels=1 source_properties=device.description=Kaydence-E2E-mic)"

XDG_DATA_HOME="$DATA" PIPEWIRE_NODE=kaydence_e2e_mic "$APP" > "$WORK/app.log" 2>&1 &
APP_PID=$!
for _ in $(seq 1 300); do
  grep -q "ASR startup warmup complete" "$WORK/app.log" && "$CTL" record status >/dev/null 2>&1 && break
  kill -0 "$APP_PID" 2>/dev/null || { cat "$WORK/app.log" >&2; fail "app exited during startup"; }
  sleep 0.1
done
grep -q "ASR startup warmup complete" "$WORK/app.log" || { grep -i asr "$WORK/app.log" >&2; fail "ASR never became ready"; }
echo "[e2e] app ready: $(grep -m1 'ASR startup warmup complete' "$WORK/app.log")"

OUT="$WORK/typed.txt"; : > "$OUT"
foot --app-id kaydence-e2e-proof --title "Kaydence E2E proof" \
  sh -c "stty -icanon -echo; exec cat > '$OUT'" &
FOOT_PID=$!
ADDR=""
for _ in $(seq 1 100); do
  ADDR="$(hyprctl clients -j | jq -r '.[] | select(.class == "kaydence-e2e-proof") | .address' | head -n1)"
  [ -n "$ADDR" ] && break; sleep 0.05
done
[ -n "$ADDR" ] || fail "proof window never mapped"
hyprctl dispatch "hl.dsp.focus({ window = \"address:$ADDR\" })" >/dev/null
for _ in $(seq 1 60); do
  [ "$(hyprctl activewindow -j | jq -r '.address')" = "$ADDR" ] && break; sleep 0.05
done
[ "$(hyprctl activewindow -j | jq -r '.address')" = "$ADDR" ] || fail "proof window never took focus — nothing dictated"

"$CTL" record start >/dev/null
pw-play --target kaydence_e2e "$CLIP"
sleep 0.2
RELEASE_MS="$(now_ms)"
"$CTL" record stop >/dev/null

FIRST_MS=""; LAST=""; STABLE=0
for _ in $(seq 1 3000); do
  CUR="$(cat "$OUT")"
  if [ -n "$CUR" ] && [ -z "$FIRST_MS" ]; then FIRST_MS="$(now_ms)"; fi
  if [ -n "$CUR" ] && [ "$CUR" = "$LAST" ]; then STABLE=$((STABLE + 1)); else STABLE=0; fi
  [ "$STABLE" -ge 40 ] && break
  LAST="$CUR"; sleep 0.01
done
TYPED="$(cat "$OUT")"
DONE_MS="$(now_ms)"
echo "[e2e] app outcome: $(grep -m1 -o 'injection outcome: event=[A-Za-z]*' "$WORK/app.log" || echo 'none logged')"
echo "[e2e] typed: $TYPED"
[ -n "$TYPED" ] || { tail -20 "$WORK/app.log" >&2; fail "nothing was typed"; }
norm() { tr '[:upper:]' '[:lower:]' | tr -cd 'a-z0-9 \n' | tr -s ' '; }
printf '%s' "$TYPED" | norm | grep -qF "$(printf '%s' "$EXPECT" | norm)" \
  || fail "typed text does not contain the expected words"
echo "[e2e] release → first character typed: $((FIRST_MS - RELEASE_MS)) ms (includes the 300 ms capture tail)"
echo "[e2e] release → text settled: $((DONE_MS - RELEASE_MS - 400)) ms (±10 ms poll)"
echo "[e2e] RESULT: PASS (spoken audio → real app → text typed into the focused window)"
