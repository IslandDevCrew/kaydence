//! Whisper-Ahead: parallel local-only prediction layer. Emits PredictionOffered/Merged/Dismissed/Stale (ADR-0005). Dictation always wins latency.
//!
//! Stub (P0-T3). Exposes the typed `SessionEvent` contract with `todo!()`
//! bodies so the event contract compiles before feature work (SCAFFOLD.md §6).
//! Implement per this module's `AGENTS.md`. No stage imports another's internals
//! — communicate only via `SessionEvent`.
#![allow(dead_code)]

use crate::events::SessionEvent;

/// Placeholder entry point for the `prediction` stage. Returns the event(s) it emits
/// once implemented.
pub fn stage() -> SessionEvent {
    todo!("prediction: implement per apps/desktop/src-tauri/src/prediction/AGENTS.md")
}
