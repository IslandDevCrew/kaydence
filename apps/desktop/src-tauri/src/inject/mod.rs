//! Deliver: CleanFinal -> focused field. Native first (AX/UIA/AT-SPI), clipboard fallback w/ restore. Secure fields refused (non-negotiable #8).
//!
//! Stub (P0-T3). Exposes the typed `SessionEvent` contract with `todo!()`
//! bodies so the event contract compiles before feature work (SCAFFOLD.md §6).
//! Implement per this module's `AGENTS.md`. No stage imports another's internals
//! — communicate only via `SessionEvent`.
#![allow(dead_code)]

use crate::events::SessionEvent;

/// Placeholder entry point for the `inject` stage. Returns the event(s) it emits
/// once implemented.
pub fn stage() -> SessionEvent {
    todo!("inject: implement per apps/desktop/src-tauri/src/inject/AGENTS.md")
}
