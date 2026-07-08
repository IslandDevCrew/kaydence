# Kaydence Roadmap

Phases gate on exit criteria, not dates. Week counts assume one primary builder
plus agent workflows, and a Fri-sunset → Sat-sunset weekly off-block — do not
schedule releases or crunch against it.

## Phase 0 — Foundation (Week 1)
Study Handy's source (Tauri config, cpal capture, whisper.cpp + Parakeet ONNX
wiring, VAD integration, tray/hotkey plumbing). Write ADR-0004 notes on what we
adopt vs. redesign. Bootstrap toolchain per `SCAFFOLD.md`. CI skeleton running
fmt/clippy/test on macOS + Windows runners.
**Exit:** `cargo tauri dev` opens a window on both OSes; CI green; ADRs 1–4 accepted.

## Phase 1 — MVP parity (Weeks 2–4) → PRD P0-1..P0-4, P0-6..P0-8
Hotkey → capture → WAL → VAD → local ASR (Parakeet CPU first, Whisper GPU second)
→ injection (Mac AX, then Windows UIA — spike Windows injection in week 2, it is
the highest-risk item) → history. Raw mode only. First-run flow + model download.
**Exit:** daily-drivable raw dictation on both OSes; crash-recovery and
short-utterance test suites pass; latency budgets met in Raw mode.

## Phase 2 — The differentiator (Weeks 5–7) → P0-5, P1-1, P1-2, P1-5
Cleanup dial (rule engine → Light w/ local LLM → Full), streaming partials in
HUD, dictionary + Wispr/Glaido import, BYOK lanes with fallback chain.
**Exit:** ≥95% zero-edit rate at Light on the golden corpus; Wispr Flow
uninstalled from the primary user's machines.

## Phase 3 — Daily-driver polish (Weeks 8–10) → P1-3, P1-4, P1-6
Per-app profiles, snippets, correction-learning, signed installers
(notarized .dmg, signed .msi), latency hardening, diagnostics panel.
**Exit:** a non-builder can install and succeed in 60 seconds; naming/trademark
pass complete; public beta candidate.

## Phase 4 — Expansion (post-beta) → P2-x
Voice edit commands, MCP server for coding agents, file transcription,
agentic text mode, Linux build, mobile exploration. Re-prioritize from beta feedback.

## Phase 3.5 — Whisper-Ahead milestone (predictive completion) → P2-7..P2-13
Gated on Phase 2 (streaming partials + local-LLM lane solid) and Phase 3 (daily
driver). Build order: (1) local prediction engine + per-platform model via
`bench-prediction`; (2) Surface A HUD dual-line (Streaming, then Paused-Merge);
(3) merge matcher + Tab/Shift-Tab + gold pulse; (4) Surface B inline fallback +
surface router; (5) opt-in `context/` (Tier 1, then Tier 2); (6) local analytics
+ dashboard; (7) the timing ladder.
**Exit:** prediction is a daily-used assist on both OSes within budget; merge
acceptance rate measured; privacy tests (no persistence/transmission of context)
pass; Gemma license review closed before any paid build ships it.
