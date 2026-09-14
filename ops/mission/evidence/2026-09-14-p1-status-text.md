# P1-G4 — status text contrast follow-through

Scope: the operator-approved narrow contrast correction; no palette, identity,
layout, status meaning, backend, dependency or shipping-policy changes.
Base is PR42 tested head02a1ff5 (tree-identical to merged main9d0d6a7).

## Finding and correction

The accent-only acceptance explicitly left nonaccent status text unresolved.
Extend its paint-aware checker with CHECK_ALL_TEXT=1: every sampled enabled
text node must meet4.5:1. Unsupported paints still fail rather than receive a waiver.
Controlled ready/blocked/held/failed IPC fixtures exercise presentation only;
they are not native permissions, injection, capture or runtime evidence.

Shared success/warn/danger text shades mix60% existing status color with40%
current theme ink. Raw palette values, dots, backgrounds and four-state meanings
stay unchanged. Replace informational future-step group opacity with muted ink;
the P3 label remains and no future control/feature is enabled.
The controlled failed-injection fixture additionally exposed Cleanup's Issue
badge at4.331607:1 in light mode across the three accent selectors. Its text now
uses danger-ink; the failure remains red and its behavior is unchanged.

## Verification boundary

Browser Chromium at900x600, light/dark and all three OS accent selectors;
four screens, three cockpit layouts, disclosures and dialogs, rest/hover/focus.
The original baseline contains unsupported group opacity: its approximate
numeric ratios are not valid contrast proof. Supported failures are retained.
Programmatic focus/scroll here measures paint states, not keyboard usability.
Separate body/modal keyboard packets own natural-navigation acceptance.
Final own runs pass450 scopes/24,036 samples: preview5,346; ready4,752;
blocked4,434; held4,752; failed4,752. Every scenario minimum4.656229:1,
zero failed/unsupported sampled paints, blocked pointer states or frontend errors.
Twenty-four final preview PNGs cover all four screens, both themes and accents.
Main inspected Setup light; body typography remains a separate queued correction.
Fresh fmt/clippy/253Rust tests/frontend/privacy/ADR/build PASS; synthetic bench
measured fields PASS and physical fields PARTIAL. Final results use *-final names;
other JSON archives preserve preliminary/baseline findings, not acceptance.
Independent Judge PASS at130e3a7: own180 scopes/10,098 samples, same minimum;
50 hashes/counts verified and original Issue-color/unsupported-gradient negative
probes rejected. Archived raw Judge record/logs bind scope and measured results.
Main Audit PASS: scoped ADR follow-through, no invariant or gate weakened.
Additional39 reference/rail/dialog/hydration/privacy browser regressions PASS.
Final integrated source binding, docs Audit and exact-head CI/CLEAN are pending.
Final docs Audit rejected the initial0018 append: accepted0018 is restored,
and additive0019 now records this treatment. The integrated record owns acceptance.
P1-G4 and full WCAG/native/reference-hardware gates remain open.
