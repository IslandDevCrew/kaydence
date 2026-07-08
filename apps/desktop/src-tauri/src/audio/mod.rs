//! Capture: mic -> lock-free ring buffer -> WAL file -> Silero VAD. Emits AudioPersisted (non-negotiable #2).
//!
//! Stub (P0-T3). Exposes the typed `SessionEvent` contract with `todo!()`
//! bodies so the event contract compiles before feature work (SCAFFOLD.md §6).
//! Implement per this module's `AGENTS.md`. No stage imports another's internals
//! — communicate only via `SessionEvent`.
#![allow(dead_code)]

use crate::events::SessionEvent;

/// Placeholder entry point for the `audio` stage. Returns the event(s) it emits
/// once implemented.
pub fn stage() -> SessionEvent {
    todo!("audio: implement per apps/desktop/src-tauri/src/audio/AGENTS.md")
}
