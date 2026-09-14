# P1-G4 — Setup Evidence dialog readability

## Scoped plan and custody

Routine frontend presentation correction for PRD P0-8 / P1-G4, under the approved
11px/readability continuation. Only `FirstRunEvidence.css` and owned checks/evidence
change; model selection, permission/download behavior, IPC, identity, colors, 8px cards,
main-board CSS, Rust and mission state are untouched. The preserved full-Setup branch
`mission/p1-g4-setup-type` remains at9e47df3 for the later main-board unit.
This separate unit started on shared-modal B c4b9a79, then clean-rebased to B2835f544.
Actual source/checker anchor3dcc81d remains local, not merged or phase-accepted.

## Implementation and falsification

Restore dialog text to existing type tokens (minimum11px), including the eyebrow inherited
from main-board CSS. Wrap long evidence words and size evidence grid rows to their content.
Permission buttons use an inset cue for both native focus-visible and A's explicit keyboard
marker; retain Modal's existing8px scroll margin instead of adding the obsolete4px override.
The first actual-source baseline was36 RED:32 font failures and4 Chromium proof cases that
stopped at an opposite-wheel gesture. Its checker SHA was8e71e90af839887358798f58512195cea84bf777a27c619b1a52bbf12c958cc7.
Adding80ms input-start allowance before300ms observed scroll quiescence cured that harness
timing issue; the corrected proof run still failed its actual8–9px text. No scroll assignment.
The first typography candidate exposed a real450/501 model-option spill: a27px card held
a28px text child plus padding. Those interrupted candidate logs/captures are retained, not
called complete runs. Content-sized rows cure it; removing that declaration reproduces it.

## Validation scope

Flow: Setup → actual Evidence opener → Models / Permissions / Proof → read every nonspace
character through real vertical/horizontal wheel → natural keyboard → Escape and Close →
same visible opener. Chromium and WebKit Option+Tab, both themes,900x600/450x300/501x300.
Browser plugin unavailable; bundled Playwright is the explicitly permitted fallback.
Positive runs never inject CSS, force focus or assign scrolling. Extended IPC fixtures show
blocked model/preflight and permission guidance; downloads stay disabled, OS settings are
never opened, and unknown commands fail. They are browser fixtures, not native proof.
Historical c4-based candidate:36 default +36 extended PASS and12 negative probes; retained.
Fresh actual B2835 parent at3dcc81d:36 default +36 extended PASS and12 negatives reject
their faults. Typecheck/lint, reference16, rail10, modal4 and external production build PASS;
main standing gates also pass253 tests/fmt/clippy/privacy/ADR/synthetic4, physical PARTIAL.

The plain negative checker rejects8px text, hidden/clipped text, missing outlines, removal
of content-sized rows and removal of the eyebrow floor. Independent Judge at3dcc81d PASS:
fresh72/12 plus24 separate narrow character/client-box/cue cases; four matched row-rule
removals reproduce card spill. Permission inset changes offset only; it is not unchanged geometry.
Manifest-bound historical/integrated/Judge bundles retain16/13/13 raw records,4/80/96 PNGs,
nine current source hashes and verbatim Judge. Two plain independent supplemental runners
remain under /tmp/kaydence-setup-evidence-judge-20260914 with paths/hashes in Judge data;
no source scripts are compressed. Reproduce checked-in primary with PLAYWRIGHT_MODULE,
APP_URL=http://127.0.0.1:1435 and fresh CAPTURE_DIR, then DETAILS=1; run sibling negative.
Publication Audit, actual merged-parent reconciliation and own3OS CI/CLEAN remain required.
Main-board9px text,
other dynamic error/contrast states, native/OS scaling and full-AA/P1 acceptance remain open.
