# ADR-0001: Tauri over Electron (and over pure native)

- **Status:** Accepted
- **Date:** 2026-06-12
- **PRD items affected:** P0-6, P0-7, non-functional footprint

## Context
The category's loudest resource complaint is Wispr Flow's Electron builds
(~800 MB idle RAM reported; Windows app freezing target apps). Pure-native
(Swift + C#) doubles every feature's cost and slows Windows to second-class —
the exact failure we exist to avoid. Handy proves Tauri ships a 12.6 MB
cross-platform dictation app.

## Decision
Tauri 2.x: Rust core, system webview UI, single codebase for macOS + Windows
(+ Linux nearly free later).

## Alternatives considered
- **Electron:** footprint and the P3 pitfall; rejected.
- **Native ×2:** velocity and Windows-parity risk; rejected.
- **Rust + egui/iced (no webview):** viable, but slower UI iteration and worse
  accessibility tooling for settings/history UI; revisit only if webview
  becomes a real constraint.

## Consequences
All product logic must live in Rust (frontend is presentation-only — enforce in
review). We accept webview quirks for the HUD overlay window and must test
overlay behavior per-OS.
