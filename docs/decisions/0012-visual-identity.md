# ADR-0012: Visual identity — logo selection + OS accent lanes

- **Status:** Proposed <!-- OPERATOR DECISION PENDING: becomes Accepted when the operator selects the winning logo (task P1-D2). Declared by-design exception in scripts/check-adr-status.sh until then, same mechanism as ADR-0009. -->
- **Date:** 2026-07-08
- **PRD items affected:** every UI-bearing item from P1 on (P0-1..P0-8, P1-*, P2-*, P4-*);
  public-site header; installer/dock/taskbar/tray iconography on all three OSes

## Context

Visual System v1 is locked (see `designDirection` in `ops/mission/state.json` and
`docs/design/README.md`): ten screen-family boards × 3 OS = 30 views over one interface
system, with the free-tier per-OS variation limited to the selection accent (macOS aqua /
Windows cobalt / Linux emerald). The pack also generated **five logo directions**
(`assets/brand/logos/`). One must be selected as the single mark used for the app icon,
tray icon, installers, and public-site header across all three OS lanes. Screen layout
is logo-independent — every board reserves the same slot, so the winner drops in without
layout change. Selecting the identity is a critical decision path (operator-facing,
hard to reverse after public exposure), so it HALTS for operator selection per
JUDGE-AUDITOR policy.

## Decision

**PENDING OPERATOR SELECTION.** The operator picks one of the five options below; on
selection this ADR flips to Accepted, the option number is recorded here, and the mark
replaces the placeholder in the reserved slot across all screens + the public site.

**Recommendation (prepared 2026-07-08, task P1-D2): Option 1 — K Waveform**, with
Option 4 — Wordmark Forward as runner-up.

### Review of all five options (against the boards and the three-OS reality)

| # | Option | App icon at 16–32 px | Three-desktop / product story | Palette & system fit | Risks |
|---|--------|----------------------|-------------------------------|----------------------|-------|
| 1 | **K Waveform** | **Strong.** Dark rounded tile with a bold chevron-K silhouette survives small sizes; the tile ships as-is in dock/taskbar/tray on light and dark. | **Strong.** Waveform (voice) flows through an amber-gold text caret into the K — literally speech→text, the core product. | **Best.** Teal/blue match the brand ramp; the gold caret echoes the amber-gold merge pulse; neutral against all three accent lanes. | Waveform bars soften below 24 px, but the tile + chevron silhouette carries recognition. |
| 2 | Protected Path | Weak. Thin line-art circle degrades at small sizes; reads as a badge/stamp, not an everyday icon. | Good privacy story (voice enters, sealed loop, text exits) without security-tool styling. | Palette-consistent. | Better as a secondary privacy seal (site/privacy page) than the primary mark. |
| 3 | Relay Monogram | Weak. Interlocked double-K + embedded window/traffic-light details turn to noise below 48 px. | Most literal three-OS story — but it embeds the **Windows flag and Tux** directly. | Gold waveform between the K halves is a nice merge-pulse echo. | **Trademark/brand risk: third-party OS marks inside our logo.** Not shippable as-is; disqualifying for the primary mark. |
| 4 | **Wordmark Forward** | **Good.** High-contrast serif K on a light tile is legible small; the pack already shows the icon variant. | Quieter story — the small waveform in the K counter is subtle; premium/editorial rather than utility-app energy. | Distinctive against the sans-serif tech field; fits the pay-once "considered tool" positioning; strongest **public-site header** of the five. | Serif identity is a stronger stylistic commitment; light tile is less distinctive in a dock of dark tiles. |
| 5 | Voice Constellation | Weak. Three floating device nodes + connectors is a diagram, not an icon; illegible small. | Great **Relay/fleet illustration** — three devices, gold spark at the meet point. | Palette-consistent. | Keep as a Relay feature illustration (board 04, marketing), not the identity. |

### Why Option 1 over Option 4 (the trade-off)

Option 1 is purpose-built for the surface the mark lives on 99% of the time — the OS
dock/taskbar/tray at small sizes — and it encodes the product (voice → caret → text) in
one glance without borrowing anyone's trademarks. Option 4 is the more sophisticated
*brand*, and the better site header; its cost is icon distinctiveness and a heavier
stylistic commitment. A workable hybrid (icon from 1, site typography discipline from 4)
is possible later without reopening this ADR, since both share the palette.

### Codified regardless of selection

1. The OS accent lanes stay as locked in `designDirection`: macOS aqua / Windows cobalt /
   Linux emerald, selection-accent only; the logo itself is identical across lanes.
2. Option 3's embedded third-party OS marks are rejected for any shipping asset.
3. Options 2 and 5 remain available as secondary illustrations (privacy seal; Relay
   fleet art) — usage there does not require reopening this ADR.
4. Until Accepted, all screens use a neutral placeholder mark in the reserved slot.

## Alternatives considered

- **Ship with a placeholder indefinitely** — rejected: the public site and installers
  (P3-G2 signed installers) need a real identity; deferring past P3 blocks release art.
- **Commission a sixth direction** — rejected for now: the five cover the plausible
  space (icon-led, seal, monogram, wordmark, constellation); iterate on the winner
  instead if refinement is needed.
- **Different logo per OS** — rejected: violates the locked one-system rule (accent is
  the only per-OS variation) and fragments the brand across the fleet Relay unifies.

## Consequences

- **Easier:** every screen family, installer, tray, and site header inherits one mark;
  ADR-0012 + `docs/design/README.md` give any build agent the identity rules without
  re-deriving them.
- **Harder:** the winner must be produced as proper multi-resolution assets
  (macOS .icns incl. 16/32/128/256/512@2x, Windows .ico, Linux hicolor PNG set +
  symbolic tray variant) — that asset-production task lands with the P3 installer work.
- **Maintained:** `scripts/check-adr-status.sh` carries 0012 as a by-design Proposed
  exception until selection; flipping to Accepted removes it from the exception list
  (the reverse edit of what added it).
