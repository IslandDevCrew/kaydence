//! Opt-in, in-memory-only surrounding-text read (Tier 1 AX/UIA, Tier 2 OCR). Never persisted, never transmitted (ADR-0006).
//!
//! Stub (P0-T3). Exposes the typed `SessionEvent` contract with `todo!()`
//! bodies so the event contract compiles before feature work (SCAFFOLD.md §6).
//! Implement per this module's `AGENTS.md`. No stage imports another's internals
//! — communicate only via `SessionEvent`.
#![allow(dead_code)]

use crate::events::SessionEvent;

/// Placeholder entry point for the `context` stage. Returns the event(s) it emits
/// once implemented.
pub fn stage() -> SessionEvent {
    todo!("context: implement per apps/desktop/src-tauri/src/context/AGENTS.md")
}
