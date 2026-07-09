#!/usr/bin/env bash
# bench.sh — the latency & footprint gate (root AGENTS §5).
#
# Drives the implemented bench contract and emits the timing table
# (release->inject, streaming lag, Light cleanup cost, plus explicit unmeasured
# fields for model/footprint work that has not landed yet). In --check mode it
# compares every measured result against the §5 budgets and fails if any budget
# regresses (>10% band applied to the baseline once one is recorded).
#
# Runs on each OS (macOS/Windows/Linux) — budgets are per-platform. Uses a built
# `--bench` binary when present, otherwise runs the Rust bench entry point through
# Cargo so CI can enforce the gate before release packaging exists.
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
BENCH_BIN_EXE="${BENCH_BIN}.exe"
if [ -x "$BENCH_BIN" ]; then
  RESULTS="$("$BENCH_BIN" --bench)"
elif [ -x "$BENCH_BIN_EXE" ]; then
  RESULTS="$("$BENCH_BIN_EXE" --bench)"
elif command -v cargo >/dev/null; then
  RESULTS="$(cargo run --quiet --manifest-path "$ROOT/apps/desktop/src-tauri/Cargo.toml" --bin kaydence -- --bench)"
else
  echo "RESULT: FAIL (no bench binary and cargo is unavailable)" >&2
  exit 2
fi
printf '%s\n' "$RESULTS"
[ "$MODE" != "--check" ] && exit 0

command -v node >/dev/null || { echo "WARN: node not found; cannot compare budgets." >&2; exit 2; }
printf '%s' "$RESULTS" | BUDGET_JSON="$BUDGET_JSON" node -e '
  let data=""; process.stdin.on("data",c=>data+=c).on("end",()=>{
    const m=JSON.parse(data), b=JSON.parse(process.env.BUDGET_JSON);
    let fail=0;
    let measured=0;
    const skipped=[];
    for (const k of Object.keys(b)) {
      if (m[k]==null) { skipped.push(k); continue; }
      measured++;
      if (m[k] > b[k]) { console.error(`REGRESSION ${k}: ${m[k]} > ${b[k]}`); fail=1; }
    }
    if (measured===0) { console.error("RESULT: FAIL (bench emitted no measured budget fields)"); process.exit(2); }
    if (fail) process.exit(1);
    console.log(`RESULT: PASS (${measured} measured metrics within §5 budgets)`);
    if (skipped.length) console.log(`RESULT: PARTIAL (unmeasured: ${skipped.join(", ")})`);
  });
' || { echo "RESULT: FAIL (budget regression)" >&2; exit 1; }
