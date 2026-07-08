//! Global hotkey registration (push-to-talk + toggle). Serialised via a single coordinator thread + debounce (Pitfall P1, ADR-0004 study).
//!
//! Stub (P0-T3). Exposes the typed `SessionEvent` contract with `todo!()`
//! bodies so the event contract compiles before feature work (SCAFFOLD.md §6).
//! Implement per this module's `AGENTS.md`. No stage imports another's internals
//! — communicate only via `SessionEvent`.
#![allow(dead_code)]

use crate::events::SessionEvent;

/// Placeholder entry point for the `hotkeys` stage. Returns the event(s) it emits
/// once implemented.
pub fn stage() -> SessionEvent {
    todo!("hotkeys: implement per apps/desktop/src-tauri/src/hotkeys/AGENTS.md")
}
