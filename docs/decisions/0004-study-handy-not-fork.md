# ADR-0004: Study Handy's patterns; do not hard-fork

- **Status:** Accepted
- **Date:** 2026-06-12
- **PRD items affected:** Phase 0/1 velocity

## Context
Handy (MIT, ~20k stars) already solves Tauri + cpal capture + whisper.cpp +
Parakeet ONNX + Silero VAD + global shortcuts cross-platform, and its author
explicitly optimizes for forkability. But its event model, raw-only pipeline,
and paste-based injection differ from our architecture (typed SessionEvent
contract, cleanup stage, native-injection-first).

## Decision
Treat Handy as a reference implementation: adopt its dependency choices and
platform workarounds, vendor specific snippets only with MIT attribution
preserved, but build our pipeline on our own event contract from day one.

## Alternatives considered
- Hard fork: fastest week 1, but we'd spend Phase 2 fighting an architecture
  that has no cleanup stage or injection abstraction.
- Clean-room ignore: wastes their hard-won platform fixes (e.g., macOS global
  shortcut rewrite).

## Consequences
Phase 0 includes a structured read of their `src-tauri`; an
`ATTRIBUTIONS.md` is mandatory the moment any code is vendored.

## Study pass findings (2026-07-07) — Handy @ v0.9.0, `com.pais.handy`

Structured read of `src-tauri/` (shallow clone of `cjpais/Handy`). Concrete
adopt/redesign notes for P0/P1. **No Handy code is vendored yet; if any is,
`ATTRIBUTIONS.md` lands in the same commit (MIT preserved).**

### ADOPT (patterns that map cleanly to our charter)
| Handy pattern | Where | Why we adopt |
|---|---|---|
| **`TranscriptionCoordinator`** — one dedicated thread serialises every lifecycle event (shortcut press/release, cancel, signals, async transcribe→paste) via `mpsc`, with a 30 ms **debounce** and an explicit `Idle/Recording/Processing` `Stage` enum | `transcription_coordinator.rs` | This is the production answer to **Pitfall P1** (hotkey races truncating short utterances — the VoiceInk scars). Our `hotkeys/`→`audio/` path adopts the single-serialising-thread + debounce shape; we thread our typed `SessionEvent` through it. |
| **`specta` + `tauri-specta`** generate the TS types from Rust | `Cargo.toml` | Exactly our ARCHITECTURE §4 mandate (SessionEvent → generated TS, never hand-written). Adopt as our `gen-types` path. |
| **Per-platform ASR backend via Cargo `[target.*]` features** — macOS `metal`; Win-x64 + Linux `vulkan` + `dynamic-backends`; Win-ARM static CPU-only (ships zero DLLs, sidesteps per-DLL signtool verify) | `Cargo.toml` targets | Mirror this exact target-feature matrix for our whisper.cpp lane; it is hard-won and current. |
| `cpal` + `rubato` (resample) + `hound` (WAV) capture stack; `vad-rs` (Silero); `rusqlite` bundled + `rusqlite_migration`; checksum'd model download | `managers/audio.rs`, deps | Same choices our ARCHITECTURE already names; confirms feasibility. |
| **Linux is real in Handy** — `gtk-layer-shell` + `gtk` for the overlay, vulkan whisper | `[target.linux]` | Validates ADR-0011 (Linux first-class). Adopt `gtk-layer-shell` as a candidate for our HUD Surface A on Linux. |

### REDESIGN (where we diverge on purpose)
| Handy | Our redesign | Reason |
|---|---|---|
| Injection = `enigo` keystroke synth + clipboard paste | **Native-first**: AX `AXValue` / UIA `ValuePattern` / AT-SPI insertion, `enigo` as *fallback*, clipboard fallback that **snapshots + restores ≤200 ms** | Non-negotiable #8 + **Pitfall P2** (paste destroys the clipboard). Handy's paste path is the scar we avoid. |
| Raw output only, no cleanup stage | Cleanup dial (Raw/Light/Full) as a first-class pipeline stage (ADR-0003) | Our core differentiator. |
| Several **git-fork deps** (`rdev` rustdesk fork, `vad-rs`/`rodio`/`hf-hub` cjpais forks) | Prefer crates.io releases; if a fork is unavoidable, pin a commit + record it as a critical-path dependency (ADR + `audit-network` allowlist) | Supply-chain hygiene; git deps are a maintenance/security risk. |
| `reqwest` present for LLM/model in the base dep set | Keep network deps **behind BYOK/relay features + the audit-network allowlist**; no egress by default | Non-negotiable #1. |

### CRITICAL LESSON captured (bug we must not re-hit)
Handy's Windows notes document that **pyke's prebuilt ONNX Runtime is compiled
with a global `/arch:AVX2` baseline that executes BMI2/AVX in a static
initializer and crashes at process startup on any pre-Haswell CPU** (Sandy/Ivy
Bridge, older AMD). Their fix: drop DirectML, link a **baseline ONNX Runtime with
runtime CPU dispatch** (via `ORT_LIB_LOCATION`). Our Windows **CPU-first Parakeet
lane targets exactly those mid-range/older no-dGPU machines**, so we inherit this
posture from day one and add a pre-Haswell smoke check to the Windows CI leg.
