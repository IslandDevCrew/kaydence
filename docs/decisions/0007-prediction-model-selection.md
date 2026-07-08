# ADR-0007: Prediction model — runtime and per-platform defaults

- **Status:** Accepted (defaults to be locked from on-device bench)
- **Date:** 2026-06-19
- **PRD items affected:** P2-7 (Whisper-Ahead engine)

## Context
The prediction model must produce 2–5 word continuations fast enough to feel live
(≤400 ms) on both Apple Silicon and mid-range Windows (often no dGPU). Cotypist
proved Qwen 2.5 1.5B for exactly this on Apple Silicon, but uses MLX — Mac-only,
which would break Windows-first (non-negotiable #5).

## Decision
Runtime: **llama.cpp / GGUF**, cross-platform (Mac Metal + Windows CPU/CUDA/
Vulkan). Per-platform defaults, hardware-detected at first run:
- **macOS / Apple Silicon:** Qwen 2.5 1.5B (Q4_K_M) — Apache 2.0. Primary.
- **Windows mid-range:** Gemma 3 1B (Q4_K_M) — strong quality on mid Windows
  systems (operator's call). **License caveat: Gemma Terms, not Apache —
  legal review required before commercial bundling; ship via on-demand download,
  not bundled, to keep redistribution clean.**
- **Low-end / CPU floor (any OS):** Qwen 3 0.6B (Q4_K_M) — Apache 2.0.
Final defaults are locked from the table printed by `scripts/bench-prediction.sh`
run on the two reference machines (M-series Mac + mid Windows laptop). MLX may be
added later as a Mac-only acceleration option, never as the only path.

## Alternatives considered
- MLX runtime (Cotypist's): Mac-only, rejected as the base; allowed as a later
  Mac optimization.
- Single model everywhere: no one model is optimal across the hardware range.
- Bundling Gemma weights: redistribution friction under Gemma Terms — download
  on demand instead.

## Consequences
`models/registry.json` carries all three with checksums + license flags; the
bench harness gates the default; the Gemma license review is a tracked task
before any paid build ships it.
