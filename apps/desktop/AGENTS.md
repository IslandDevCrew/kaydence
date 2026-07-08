# apps/desktop/ — Agent Guide

The Tauri application. Two halves with a hard boundary:

- `src-tauri/` — Rust backend. **All product logic lives here.**
- `src/` — React frontend. **Presentation only.**

## The boundary rule
If a change makes the frontend compute, decide, transform text, touch the
network, or hold canonical state, it is in the wrong place. The frontend
renders state and forwards intents; Rust owns truth. IPC surface = Tauri
commands (intents in) + the `SessionEvent` stream (state out), mirrored to TS
via generated types — never hand-write the TS event types.

## Permissions choreography (first-run, all three OSes)
- macOS: Microphone, Accessibility, Input Monitoring — request in that order,
  each with a plain-language why-screen; the app must remain useful enough to
  finish onboarding if a permission is deferred.
- Windows: microphone privacy setting; no elevation required ever.

## Definition of done here
Feature works on all three OSes, first-run flow unbroken (P0-8: 60s to first
dictation), no new permissions without PRD trace, HUD overlay verified against
fullscreen apps and multi-monitor setups.
