# Kaydence Design Language v2 — Gauntlet Lock (2026-08-09)

## Status: LOCKED (operator 8.5/10, explicit threshold waiver) — supersedes the ad-hoc P1-G4 board-selection path for the design *language*. The P1-G4 *gate* itself remains PENDING until D4 rebuild + D5 fresh capture land.

## Why this exists

A design-consistency audit of the built P1 screens (Cockpit, Cleanup, Privacy, First Run) against
the Visual System v1 boards found the drift was architectural, not taste: every view forked its
own CSS tokens with different hex values, no theme system existed (dark mode was unimplemented on
every screen), navigation was built three different ways within one screen family, and the Windows
per-OS accent was byte-identical to the reserved Whisper-Ahead prediction color. Full findings:
`ops/mission/design-relock/idea.lock.json`.

## Process

Run as a **gauntlet loop** (fan-out builders + blind critics against a falsifiable bar) under
**archipelago** discipline (locked idea + plan, evidence-gated, nothing crosses on claims alone):

1. **Brief** — the original 12-tool competitor research (Wispr Flow, Aqua Voice, Superwhisper,
   Willow, VoiceInk, MacWhisper, Spokenly, Handy, Glaido, OpenWhispr, native macOS/Windows/Google)
   plus the design-consistency audit's falsifiable rubric (R1-R8).
2. **Gauntlet-lite** — three independent directions (Calm Utility, Editorial Precision, Warm Focus)
   each built families 01 (Cockpit) + 07 (Privacy) from one token system; each blind-critiqued
   against R1-R8 and competitor cohesion.
3. **Judge** — **Editorial Precision (Swiss/International)** won: the only candidate clearing all
   8 rubric dimensions (both other directions shipped mouse-only primary navigation; Warm Focus
   additionally forked its token system across four scopes).
4. **Fix + graft** — the judge's 7 fixes applied (raw-palette dedup, lexicon cleanup, `--on-accent`
   token, full spacing tokenization, contrast tightening, the Whisper-Ahead hero animation, a wired
   record button) plus 5 grafts from the runners-up (ready-pulse, warm-paper alternate palette,
   token-derived atmosphere, tactile key styling, the Network Audit trust moment).
5. **Blind re-verify** — a fresh agent re-checked the fixed file against R1-R8 with zero trust in
   the apply step's self-report: all 8 dimensions pass, all fixes and grafts confirmed applied.
6. **Operator acceptance** — Jon reviewed the rendered artifact and locked it at **8.5/10**,
   explicitly waiving the idea lock's literal ≥9/10 threshold. Known deferred issues (not
   blockers): the Cool/Warm palette toggle has no effect in dark mode (only light-mode neutrals
   were overridden); other unspecified minor nuances to surface during the D4 review.

Full artifact chain: `ops/mission/design-relock/idea.lock.json`,
`ops/mission/design-relock/plan.lock.json`,
`ops/mission/evidence/2026-08-09-design-gauntlet-winner-locked-v1.html` (the locked reference).

## What's locked

- **One token system**: raw palette hexes declared once (`--l-*`/`--d-*`), semantic tokens
  (`--bg`, `--surface`, `--ink`, `--muted`, `--line`, `--success`, `--warn`, `--danger`) re-point
  via `var()` only in theme scopes — zero raw-value duplication.
- **Reserved semantics fenced off**: `--blue` (`#2f72f2`) is Whisper-Ahead prediction text only;
  `--gold` (`#d99b18`) is the accepted-prediction merge pulse only. Per-OS `--accent` must never
  equal either — this reworks the current Windows accent, which today collides with `--blue`.
- **One `NAV` constant**: the 10-item order `Dictate, Whisper-Ahead, Cleanup, Relay, Voiceprint,
  Conductor, Privacy, Dictionary, Analytics, Setup`, resolving the board-vs-invariant conflict the
  audit found (the boards depicted a different 12-item order; the build had followed the boards).
- **Theme parity**: dark works via both `prefers-color-scheme` and an explicit `data-theme`
  override, in both directions.
- **Full token coverage**: type scale (`--text-*`, floor 11px), space scale down to 1px steps,
  one `--radius-card` (8px), and the 4-state status model (good/standby/idle/danger).
- **Editorial Precision visual identity**: hairline dividers, uppercase tracked section labels,
  tabular-nums for all data, one elevated "active surface" treatment, a Cool/Warm palette toggle
  as a secondary lever (not the primary per-OS variation, which stays the single `--accent`).

## What's next (D4/D5 — this worktree)

Rebuild `apps/desktop/src/views/{DictateView,CleanupView,PrivacyView,FirstRunView}` plus a shared
`NavRail` component to consume `tokens.css` exclusively and render this system, preserving all
existing React behavior. Then a fresh capture at the closeout SHA closes P1-G4 for real. See
`ops/mission/design-relock/plan.lock.json` phases P3/P4.
