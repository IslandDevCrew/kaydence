# ADR-0006: Local context reading — refine the privacy line

- **Status:** Accepted
- **Date:** 2026-06-19
- **PRD items affected:** P2-12; refines non-negotiable #1

## Context
Prediction quality improves markedly with surrounding context (Cotypist's
longer-text predictions feel "uncanny" for this reason). The original charter
said "no screen capture of any kind," aimed at Wispr Flow's cloud screenshots.
But Cotypist's actual mechanism is **on-device text recognition / accessibility
reads, held in memory only, never transmitted** — categorically different from
cloud capture. Banning local awareness outright would forfeit real quality for no
privacy gain, since the user's data still never leaves the machine.

## Decision
Refine non-negotiable #1: **the prohibition is on transmission and persistence,
not on local awareness.** Kaydence MAY read context locally to improve
predictions, **opt-in, default off**, in two tiers:
- Tier 1: accessibility text read of the surrounding field (AX / UI Automation).
- Tier 2: optional on-device OCR of the region near the cursor (local Vision /
  Windows OCR) for apps that don't expose text.
All context is **in memory only — never written to disk, never transmitted,
never logged verbatim**; password/secure fields are always blocked; cloud screen
capture remains permanently banned with no toggle. New `context/` module.

## Alternatives considered
- Keep the absolute "no awareness" ban: forfeits quality with zero privacy
  benefit (data never left the machine anyway). Rejected.
- Always-on context: violates opt-in/consent posture. Rejected.
- Cloud context for richer models: violates the bright line. Permanently rejected.

## Consequences
`context/` module with two tiers; first-run consent flow; per-app disable in
`profiles/`; tests assert non-persistence and non-transmission. The headline
privacy promise ("nothing leaves your machine") is preserved and made explicit.
