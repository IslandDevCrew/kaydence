#!/usr/bin/env bash
# bench.sh — the latency & footprint gate (root AGENTS §5).
#
# Drives the golden audio corpus through the built pipeline and emits the timing
# table (release->inject, streaming lag, Light cleanup cost) plus idle RAM/CPU.
# In --check mode it compares the results against the §5 budgets and fails if any
# budget regresses (>10% band applied to the baseline once one is recorded).
#
# Runs on each OS (macOS/Windows/Linux) — budgets are per-platform. Requires the
# app built with `--bench` (emits the machine-readable timing table, ARCHITECTURE
# §8) and a golden corpus at scripts/corpus/audio/*.wav with expected outputs.
#
# Portable to bash 3.2 (macOS default): no associative arrays.
#   usage: bash scripts/bench.sh [--check]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="${1:-run}"

# Root AGENTS §5 budgets as a JSON literal (single source; also fed to the checker).
BUDGET_JSON='{
  "hotkey_to_capture_ms": 50,
  "partial_lag_ms": 300,
  "release_to_inject_gpu_ms": 700,
  "release_to_inject_cpu_ms": 1200,
  "light_cleanup_added_ms": 800,
  "prediction_guess_ms": 400,
  "idle_ram_mb": 250,
  "idle_cpu_pct": 1
}'

BENCH_BIN="${KAYDENCE_BENCH_BIN:-$ROOT/apps/desktop/src-tauri/target/release/kaydence}"
if [ ! -x "$BENCH_BIN" ]; then
  echo "== bench.sh =="
  echo "no built --bench binary at: $BENCH_BIN"
  echo "Build the app first (P0-T2+), then this gate measures vs root AGENTS §5:"
  printf '%s\n' "$BUDGET_JSON"
  if [ "$MODE" = "--check" ]; then
    echo "RESULT: SKIP (no build to bench) — gate not measurable until the pipeline exists." >&2
    exit 3   # 3 = not-measurable-yet (distinct from 1 = regression)
  fi
  exit 0
fi

RESULTS="$("$BENCH_BIN" --bench)"
printf '%s\n' "$RESULTS"
[ "$MODE" != "--check" ] && exit 0

command -v node >/dev/null || { echo "WARN: node not found; cannot compare budgets." >&2; exit 2; }
printf '%s' "$RESULTS" | BUDGET_JSON="$BUDGET_JSON" node -e '
  let data=""; process.stdin.on("data",c=>data+=c).on("end",()=>{
    const m=JSON.parse(data), b=JSON.parse(process.env.BUDGET_JSON);
    let fail=0;
    for (const k of Object.keys(b)) {
      if (m[k]==null) continue;
      if (m[k] > b[k]) { console.error(`REGRESSION ${k}: ${m[k]} > ${b[k]}`); fail=1; }
    }
    if (fail) process.exit(1);
    console.log("RESULT: PASS (all measured metrics within §5 budgets)");
  });
' || { echo "RESULT: FAIL (budget regression)" >&2; exit 1; }
