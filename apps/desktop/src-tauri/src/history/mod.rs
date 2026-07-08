//! Remember: persist session row (audio path, raw, clean, app, timings) to local SQLite. Survives crash (non-negotiable #2).
//!
//! Stub (P0-T3). Exposes the typed `SessionEvent` contract with `todo!()`
//! bodies so the event contract compiles before feature work (SCAFFOLD.md §6).
//! Implement per this module's `AGENTS.md`. No stage imports another's internals
//! — communicate only via `SessionEvent`.
#![allow(dead_code)]

use crate::events::SessionEvent;

/// Placeholder entry point for the `history` stage. Returns the event(s) it emits
/// once implemented.
pub fn stage() -> SessionEvent {
    todo!("history: implement per apps/desktop/src-tauri/src/history/AGENTS.md")
}
