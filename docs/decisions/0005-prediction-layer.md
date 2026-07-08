# ADR-0005: Whisper-Ahead prediction layer (two surfaces, gold merge)

- **Status:** Accepted
- **Date:** 2026-06-19
- **PRD items affected:** P2-7..P2-11 (Whisper-Ahead epic)

## Context
Users want Cotypist-style live word prediction during dictation, but Cotypist's
ghost text lands on the *same line* being composed, which is distracting (P11).
A floating dual-line cannot render reliably inside arbitrary apps (editors,
Gmail). We need prediction that stays off the active line where possible, and
degrades sensibly where the overlay isn't possible.

## Decision
Add a local-only prediction layer with **two delivery surfaces**, chosen by app
capability at session start:
- **Surface A (HUD dual-line):** prediction in a blue lane on the line below,
  tracking the mic's column. Two modes — Streaming (continuous) and Paused-Merge
  (after a 5 s pause). On merge (say or Tab), the word lifts with a ~0.5 s
  **amber-gold glow**, then settles to text color; each merge is counted locally.
- **Surface B (inline fallback):** in non-overlay apps, streaming is disabled
  (P12); after a longer 10 s pause an inline ghost appears Cotypist-style, Tab to
  accept.
Accept gestures: Tab = next word, Shift-Tab = whole phrase. Timing ladder
(5 s / 10 s / 20 s auto-stop) is configurable. Prediction is **opt-in and
local-only**, with its own resource budget, and **yields to dictation** under
latency contention.

## Alternatives considered
- Same-line ghost like Cotypist: the distraction complaint (P11). Rejected as default.
- Streaming everywhere: breaks focus in coding apps (P12). Rejected for Surface B.
- Red merge highlight (the user's initial sketch): red reads as error on every
  success, planting a negative signal. Replaced with amber-gold ("counted");
  fully themeable.
- Cloud prediction model: violates non-negotiable #6 and the privacy posture.

## Consequences
New `prediction/` module (scheduler, surface router, merge matcher) and new
`Prediction*` events; the frontend HUD becomes dual-line; a local
`prediction_events` analytics store; a second opt-in model in `models/`. Lands as
a milestone *after* core dictation + streaming partials are solid (ROADMAP).
