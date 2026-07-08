//! Target: frontmost-app detection -> AppProfile (tone, dial override, prompt). Only platform module besides inject/hotkeys with cfg blocks.
//!
//! Stub (P0-T3). Exposes the typed `SessionEvent` contract with `todo!()`
//! bodies so the event contract compiles before feature work (SCAFFOLD.md §6).
//! Implement per this module's `AGENTS.md`. No stage imports another's internals
//! — communicate only via `SessionEvent`.
#![allow(dead_code)]

use crate::events::SessionEvent;

/// Placeholder entry point for the `profiles` stage. Returns the event(s) it emits
/// once implemented.
pub fn stage() -> SessionEvent {
    todo!("profiles: implement per apps/desktop/src-tauri/src/profiles/AGENTS.md")
}
