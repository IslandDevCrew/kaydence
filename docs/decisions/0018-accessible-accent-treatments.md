# ADR-0018: Accessible shared accent treatments

- **Status:** Accepted
- **Date:** 2026-09-14
- **PRD items affected:** P0-1..P0-8; P1-G4 screen-family fidelity

## Context

The locked Editorial Precision reference pairs white text with raw OS accents.
Enabled normal text measures 2.969510:1 on aqua and 3.464649:1 on emerald,
below the same lock's AA requirement; cobalt measures 5.673754:1.
Restoring the 11px floor does not make this large text or cure the contrast failure.
The operator explicitly approved a narrowly scoped contrast ADR/correction on
2026-09-14, preserving identity, layouts and OS accent colors. Evidence:
`ops/mission/evidence/2026-09-14-p1-contrast-approval.md`.

## Decision

Retain the raw brand accents and use shared derived semantic shades where text
needs contrast: `--accent-control` mixes 75% accent with 25% black for white text;
`--accent-ink` mixes 60% accent with 40% current theme ink. The same formulas apply
to every OS; only the existing raw accent input differs. No per-OS selector,
foreground switch, new dependency or JavaScript color logic is introduced.

Raw aqua `#14a7a1`, cobalt `#0067c0`, emerald `#1f9d63`, white `--on-accent`,
reserved prediction-blue/merge-gold, identity assets, 8px cards, typography,
navigation and all three cockpit compositions remain unchanged.
This supersedes only literal raw-accent use for text-bearing controls/text in
the old reference; archived evidence is not rewritten or treated as compliant.

Accepted under the operator's scoped approval: both themes/all three accents,
90 scopes/930 samples at rest/hover/focus pass, minimum4.951315:1. Independent
Judge rerun and keyboard/lane-switch checks pass; source-bound evidence is in
`ops/mission/evidence/2026-09-14-p1-accessible-accents.txt` and the acceptance ledger.
Nonaccent status/opacity findings remain open. This is not full WCAG/native proof.

## Alternatives considered

- Keep white on raw aqua/emerald: violates the locked accessibility requirement.
- Change the raw brand palette everywhere: broader than the approved correction.
- Fork foreground rules per OS: violates the single shared treatment principle.
- Waive contrast or shrink text: rejected; neither is a valid gate cure.

## Consequences

Text-bearing accent surfaces use accessible derived shades rather than the
literal reference fill. New controls must consume the semantic tokens and tests.
P1-G4, recording integration, native proof, ADR-0016 shipping, ADR-0017 downloads
and later-phase gates remain separate; this ADR changes no backend authority.

## Status-text follow-through — 2026-09-14

The same scoped operator approval covers the remaining measured text-contrast
defects recorded above. Apply shared `--success-ink`, `--warn-ink` and
`--danger-ink` (60% existing status color, 40% theme ink) to affected text only.
Keep raw status palette values, dots, backgrounds and four-state meanings.
Informational future-step prose uses muted ink instead of group opacity; its
P3 label and disabled feature remain unchanged. No identity/layout change.
Evidence and acceptance are tracked in
`ops/mission/evidence/2026-09-14-p1-status-text.md`; browser paint checks do not
close keyboard, native or P1-G4 gates. The original accent acceptance is historical.
