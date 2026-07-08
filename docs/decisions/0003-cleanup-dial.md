# ADR-0003: Cleanup is a three-position dial, Light by default

- **Status:** Accepted
- **Date:** 2026-06-12
- **PRD items affected:** P0-5, P1-4

## Context
The two loudest *opposing* complaints in the category: raw output imposes an
edit tax (built-ins, Handy), and full AI rewrite erases the writer's voice
(Wispr Flow backlash — users downgrading to "Light Cleanup" themselves).
The vendor picking one intensity for everyone is the mistake.

## Decision
Raw / Light / Full dial. Light default: deterministic rule engine (fillers,
self-correction collapse, punctuation) plus a *constrained* LLM pass that must
preserve token overlap above a threshold or be discarded. Full: profile-driven
rewrite, opt-in only. Per-app and per-invocation overrides.

## Alternatives considered
- Single "smart" mode (Wispr default): the documented backlash.
- Raw only (Handy): the documented edit tax.
- Free-form prompt per user only: power-user-only UX; kept as advanced option
  inside profiles, not the primary control.

## Consequences
The rule engine becomes permanent core IP with its own golden-transcript test
corpus; LLM prompts are versioned artifacts with regression tests; UI must
make the dial legible in 1 second (HUD indicator).
