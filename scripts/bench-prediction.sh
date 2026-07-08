#!/usr/bin/env bash
# bench-prediction.sh — measure prediction-model candidates on THIS machine.
# Prints a table the operator uses to lock per-platform defaults (ADR-0007).
#
# Cross-platform: run on macOS (Apple Silicon) AND a mid-range Windows laptop
# (via Git Bash / WSL). Requires: llama.cpp (llama-cli + llama-bench) on PATH,
# the candidate GGUF files present (pulled via models/registry.json), and a
# corpus of real dictation continuations at scripts/corpus/prediction/*.jsonl
# (each line: {"prefix": "...", "expected": "..."}).
#
# NOTE: This is the harness skeleton. The acceptance-quality scorer hooks into
# the same prediction prompt the app uses once prediction/ is built — marked TODO.
set -euo pipefail

CANDIDATES=("qwen2.5-1.5b" "gemma3-1b" "qwen3-0.6b")
MODELS_DIR="${KAYDENCE_MODELS_DIR:-$HOME/.kaydence/models}"
CORPUS="${1:-scripts/corpus/prediction}"
BUDGET_MS=400   # root AGENTS §5: pause -> guess rendered

usage() { echo "usage: $0 [corpus_dir]"; echo "  measures TTFT, tok/s, p50/p95 latency, RAM, acceptance@5 per candidate"; exit 0; }
[[ "${1:-}" == "-h" || "${1:-}" == "--help" ]] && usage

command -v llama-cli >/dev/null || { echo "ERROR: llama.cpp (llama-cli) not on PATH"; exit 1; }

detect_platform() {
  case "$(uname -s)" in
    Darwin) [[ "$(uname -m)" == "arm64" ]] && echo "macos_apple_silicon" || echo "macos_intel" ;;
    Linux)  echo "linux" ;;
    MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
    *) echo "unknown" ;;
  esac
}

PLATFORM="$(detect_platform)"
echo "platform: $PLATFORM   budget: ${BUDGET_MS}ms   corpus: $CORPUS"
printf '%-16s | %8s | %8s | %9s | %9s | %7s | %10s\n' "model" "TTFT(ms)" "tok/s" "p50(ms)" "p95(ms)" "RAM(MB)" "accept@5"
printf -- '-----------------+----------+----------+-----------+-----------+---------+-----------\n'

for m in "${CANDIDATES[@]}"; do
  gguf="$MODELS_DIR/$m.gguf"
  if [[ ! -f "$gguf" ]]; then printf '%-16s | %s\n' "$m" "MISSING ($gguf) — pull via registry"; continue; fi

  # llama-bench gives prompt/gen throughput; parse tok/s (TODO: tighten parsing per llama.cpp version)
  toks=$(llama-bench -m "$gguf" -p 64 -n 16 2>/dev/null | awk -F'|' '/tg/ {gsub(/ /,"",$NF); print $NF}' | tail -1)
  toks="${toks:-NA}"

  # Per-sample latency over the corpus: time a short, capped generation per prefix.
  # TTFT/p50/p95 computed from the per-sample wall times. (Skeleton: wire the app's
  # exact prediction prompt + stop tokens here so numbers match production.)
  ttft="TODO"; p50="TODO"; p95="TODO"; ram="TODO"; acc="TODO"
  # acceptance@5: fraction of corpus where the model's <=5-token continuation
  # prefix-matches `expected`. TODO: implement with the production prompt.

  printf '%-16s | %8s | %8s | %9s | %9s | %7s | %10s\n' "$m" "$ttft" "$toks" "$p50" "$p95" "$ram" "$acc"
done

echo ""
echo "Lock the default for THIS platform as the candidate that meets <=${BUDGET_MS}ms p95"
echo "within the RAM budget at the highest acceptance@5. Record it in models/registry.json"
echo "(default_for) and ADR-0007. Run on BOTH reference machines before locking."
