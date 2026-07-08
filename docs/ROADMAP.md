# Kaydence Roadmap (v3 — 2026-06-21, committed 2026-07-07)

Phases gate on exit criteria, not dates. Week counts assume one primary builder
plus agent workflows, and a Fri-sunset → Sat-sunset weekly off-block — do not
schedule releases or crunch against it. Ordered by risk reduction. The mission
loop halts at every phase boundary for an operator `go`
(`ops/mission/state.json` is the live tracker). Platform mandate is 3-OS
(macOS + Windows + Linux; X11 + Wayland) per ADR-0011.

> **Supersedes the v2 roadmap.** v2 made this 2-OS and put Linux + the flagship
> features in a vague "Expansion" phase. v3 makes Linux first-class from P0 and
> commits the flagship trio as P4 (Voiceprint → Relay → Conductor).

## Phase 0 — Toolchain & law (~1 week) · SCAFFOLD.md
Bootstrap Tauri on all three OSes; stand up the 3-OS CI matrix (fmt/clippy/test +
frontend check on macOS + Windows + Linux); Handy study pass (ADR-0004);
reconcile the constitution/PRD/architecture to v3; ADRs 0001–0011 accepted
(ADR-0009 stays Proposed until the Relay design review — see below).
**Exit:** `cargo tauri dev` opens a window on all three OSes; CI green.

## Phase 1 — MVP dictation (~3 weeks) → PRD P0-1..P0-4, P0-6..P0-8
Hotkey → capture → WAL → VAD → local ASR (Parakeet CPU first, Whisper GPU second)
→ native injection on Mac (AX), Windows (UIA — spike week 2, highest legacy risk)
and Linux (**Wayland spike early — highest platform risk overall**) → history.
Raw mode only. First-run flow + model download.
**Exit:** daily-drivable raw dictation on all three OSes; crash-recovery and
short-utterance suites pass; latency budgets met in Raw mode.

## Phase 2 — The cleanup differentiator (~3 weeks) → P0-5, P1-1, P1-2, P1-5
Raw/Light/Full dial (rule engine → Light w/ local LLM → Full), streaming partials
in the HUD, dictionary + Wispr/Glaido import, BYOK lanes with the local fallback
chain.
**Exit:** ≥95% zero-edit rate at Light on the golden corpus; Wispr Flow
uninstalled from the operator's machines.

## Phase 3 — Daily-driver polish + Whisper-Ahead (~4 weeks) → P1-3, P1-4, P1-6 + P2-7..P2-13
Per-app profiles, snippets, correction-learning, signed installers for all three
OSes (notarized .dmg, signed .msi, AppImage/.deb), latency hardening, diagnostics
panel. Then the prediction layer (P3.5): local prediction engine + per-platform
model via `bench-prediction`; Surface A HUD dual-line (Streaming → Paused-Merge);
merge matcher + Tab/Shift-Tab + gold pulse; Surface B inline fallback + surface
router; opt-in `context/` (Tier 1 → Tier 2); local analytics + dashboard; the
timing ladder.
**Exit:** a non-builder installs and succeeds in 60 s; prediction is a daily-used
assist on all three OSes within budget; merge acceptance rate measured; privacy
tests (no persistence/transmission of context) pass; prediction defaults locked
from `bench-prediction.sh` on the two reference machines (ADR-0007); Gemma license
review closed before any paid build ships it; naming/trademark pass complete;
public beta candidate.

## Phase 4 — The Flagship Trio (post-beta) → PRD P4-1..P4-9 · ADR-0009
Sequenced by dependency: **Voiceprint first** (extends the learning already
flowing from P1-3/P2-11), **Relay second** (the pairing/crypto design review
gates the build — ADR-0009, operator gate), **Conductor third** (MCP substrate →
agent targets → confirm gates → fleet mode via Relay).
**Exit:** all three daily-usable across the fleet; ladder targets hit (Relay
≤500 ms LAN E2E; Voiceprint measured acceptance lift; Conductor confirm-gate
100%); Harbor optional and unbuilt without blocking anything.

## Backlog (post-trio, re-prioritized from beta feedback) → residual P2-x
Voice edit commands (P2-1), file transcription (P2-3), agentic text mode (P2-4),
mobile exploration (P2-6). (MCP server P2-2 is subsumed by Conductor P4-8; Linux
P2-5 promoted to first-class in P0.)
