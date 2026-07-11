# src-tauri/ — Rust Backend Agent Guide

This is the product. Read `docs/ARCHITECTURE.md` before editing; the pipeline,
thread model, and event contract there are binding.

## Module map (pipeline order)
| Module | Stage | One-liner |
|---|---|---|
| `audio/` | Capture + VAD | Mic → ring buffer → WAL file → speech-gated chunks |
| `engine/` | Recognize | Chunks → streaming partials → raw final (Parakeet/Whisper/BYOK) |
| `context/` | Awareness | Opt-in, in-memory surrounding-text read (AX/UIA) or local OCR → prediction |
| `prediction/` | Whisper-Ahead | Local LLM completion; surface router (HUD A / inline B); merge matcher |
| `cleanup/` | Clean | Raw → Light/Full cleaned text (rules + constrained LLM) |
| `dictionary/` | Personalize | Term boosting, replacements, snippets |
| `profiles/` | Target | Frontmost app → profile (tone, dial override, prompt) |
| `inject/` | Deliver | Text into the focused field, natively; clipboard fallback |
| `history/` | Remember | SQLite session records + audio retention |
| `hotkeys/` | Trigger | Global shortcuts, push-to-talk + toggle |
| `settings/` | Configure | Typed config, single source of truth, APP_NAME |

## Cross-cutting invariants
1. Stages communicate only via `SessionEvent` (ARCHITECTURE §4). No cross-module
   internal imports. If you need new coupling, extend the event enum via ADR.
2. Every log line and span carries the `SessionId` (ULID).
3. `#[cfg(target_os)]` is legal only in `inject/`, `hotkeys/`, `audio/`
   (device layer), `profiles/` (app detection), plus minimal Tauri shell events
   in `lib.rs` such as macOS reopen handling — and only behind a trait where a
   platform capability is being abstracted.
4. No `unwrap()`/`expect()` outside tests; module error enums via `thiserror`;
   a stage failure emits `Failed{..}` and must still leave audio + raw text
   recoverable in history (non-negotiable #2).
5. Hot path allocates nothing avoidable; model loads never on the hot path.
6. Any new crate dependency: justify in PR (size, maintenance, license) —
   licenses must be MIT/Apache-2/BSD-compatible.
7. User-close destroys the cockpit WebView but keeps the tray-owned hotkey/audio
   runtime alive. Tray or OS reopen recreates the configured main window;
   explicit Quit still terminates. Do not replace this with hide-only behavior.

## Testing
Unit tests beside code; pipeline behavior in `tests/integration/` as event-
sequence assertions. The crash-recovery test (`kill -9` mid-session, audio
survives) and the short-utterance suite are merge gates.
