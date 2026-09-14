# ADR-0019: Accessible shared status text

- **Status:** Accepted
- **Date:** 2026-09-14
- **PRD items affected:** P0-1..P0-8; P1-G4 screen-family fidelity

## Context

ADR-0018 resolved accent text but explicitly left status text and informational
group opacity open. It remains immutable. The same operator-approved narrow
contrast correction preserves identity, layouts and raw OS colors; authority is
recorded in `ops/mission/evidence/2026-09-14-p1-contrast-approval.md`.
The current Issue badge measured4.331607:1; other supported status failures and
unsupported group-opacity cases are retained in the status evidence packet.

## Decision

Apply shared semantic ink shades to affected status text without changing its meaning.
Use `--success-ink`, `--warn-ink` and `--danger-ink`:60% existing status color,
40% current theme ink. Keep raw palette values, dots, backgrounds, four-state
semantics and all identity/layout/reserved-color invariants. Informational future
prose uses muted ink rather than group opacity; P3 labels/disabled features remain.
This supplements0018 only for its unresolved status-text scope, not its accent rules.
Independent integrated180-scope Judge and own450-scope/24,036-sample runs pass
at minimum4.656229:1. See `ops/mission/evidence/2026-09-14-p1-status-integrated.md`.

## Alternatives considered

- Keep failing status colors or waive contrast: violates the accessibility gate.
- Change the raw palette or remove status hues: exceeds the scoped correction.
- Edit accepted0018: violates immutable-ADR custody; use this additive decision.

## Consequences

Affected status text uses derived shades; raw visual semantics remain stable.
Maintain light/dark/all-accent paint checks and reject unsupported measurements.
Browser paint results do not close keyboard, native, full WCAG or P1-G4 gates.
No backend, dependency, shipping, default-download or later-phase authority changes.
