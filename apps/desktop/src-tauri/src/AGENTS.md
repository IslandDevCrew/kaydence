
---

## Whisper-Ahead surfaces (added — see prediction/ + ADR-0005)

The HUD is now a **dual-line surface** plus a fallback path:

- **Surface A — dual-line overlay.** Line 1: committed/streaming text with the
  mic indicator riding the insertion point. Line 2: the **prediction lane in
  blue**, positioned to track the mic's column (the backend supplies tokens + an
  anchor hint; the frontend does the positioning). On merge, the word lifts up
  with a **~0.5 s amber-gold glow, then settles to normal text color** (gold =
  "counted"; themeable via settings, gold is default).
- **Surface B — inline fallback.** In apps that can't host the overlay, the
  backend's surface router emits an inline ghost prediction (paused-only); the
  frontend's role here is minimal (the host app renders the ghost). Show a small
  mode indicator so the user knows which surface is active.
- **Per-invocation toggle.** A control to flip prediction off / streaming /
  paused without leaving the flow.
- **Analytics view.** A local dashboard reading `prediction_events`: words merged
  (gold), acceptance rate (offered vs merged), by app, model used. Local-only,
  exportable, deletable.

Mock all of this against fixture event streams (streaming, paused-merge, merge,
surface-B, dismiss). The mic indicator and gold pulse must hit 60 fps on
integrated graphics and respect reduced-motion.
