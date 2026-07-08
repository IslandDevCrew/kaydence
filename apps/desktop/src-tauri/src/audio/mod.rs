//! Capture: mic -> lock-free ring buffer -> WAL file -> Silero VAD. Emits
//! AudioPersisted (non-negotiable #2).
//!
//! `wal` is real (P1, PRD P0-4): write-ahead persistence + crash recovery.
//! Capture (cpal) and VAD land next; the cpal callback only feeds the ring
//! buffer (invariant 1) — the capture task drains it into the WAL.
#![allow(dead_code)]

pub mod wal;

use crate::events::SessionEvent;

/// Placeholder entry point for the capture stage. Returns the event(s) it
/// emits once implemented.
pub fn stage() -> SessionEvent {
    todo!("audio: implement capture per apps/desktop/src-tauri/src/audio/AGENTS.md")
}
