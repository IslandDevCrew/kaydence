# ADR-0020: Accessible shared focus-indicator color

- **Status:** Accepted
- **Date:** 2026-09-14
- **PRD items affected:** P0-1..P0-8; PRD §5 accessibility; P1-G4

## Context

Author-defined focus outlines failed the locked AA contrast requirement after
the text corrections in immutable ADR-0018/0019. Painted active navigation was
1.374149:1 against its own selected fill (mac/light); Setup Evidence was
2.931778:1 on its tinted banner (Linux/dark). This is not inferred from alpha.
The operator's narrow contrast approval preserves identity, layouts and OS
colors: `ops/mission/evidence/2026-09-14-p1-contrast-approval.md`.

## Decision

Use shared `--focus-ring`, derived from the existing palette, for authored
focus outlines: 20% unchanged OS accent plus 80% current theme ink. Preserve
outline widths, offsets/insets, target geometry, navigation, all raw/reserved
colors, identity and all existing states. No runtime JavaScript or per-OS rule is added.
This supplements, rather than edits, accepted ADR-0018/0019. Recorded operator
approval and independent integrated review bind this narrowly scoped decision.

## Alternatives considered

- Keep translucent/72%-accent outlines: actual adjacent-color measurements fail.
- Compare every ring only with the page: invalid for selected inset controls.
- Remove outlines, lower the threshold, or change geometry/palette: out of scope.

## Consequences

New authored focus treatments must use the semantic token and actual-surface
checks. [WCAG 2.1 AA 1.4.11](https://www.w3.org/TR/WCAG21/#non-text-contrast)
requires 3:1 against adjacent colors; this is not a claim of
[WCAG 2.2 AAA Focus Appearance](https://www.w3.org/WAI/WCAG22/Understanding/focus-appearance.html)
area compliance. Native/VM proofs and P1-G4 closure remain separate. No backend,
dependency, policy, download-default or publication authority changes.
