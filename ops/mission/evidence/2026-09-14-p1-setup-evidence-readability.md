# P1-G4 — Setup Evidence dialog readability

## Scoped plan and custody

Routine frontend presentation correction for PRD P0-8 / P1-G4, under the approved
11px/readability continuation. Only `FirstRunEvidence.css` and owned checks/evidence
change; model selection, permission/download behavior, IPC, identity, colors, 8px cards,
main-board CSS, Rust and mission state are untouched. The preserved full-Setup branch
`mission/p1-g4-setup-type` remains at9e47df3 for the later main-board unit.
This separate unit started on shared-modal B c4b9a79; it is a dependent local candidate,
not a merged or phase-accepted result. Parent integration and independent Judge follow.

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
Local c4b9a79-based candidate:36 default +36 extended cases PASS;12 negative probes reject
their faults. The shared scroll-parent rebase and fresh source-bound matrix remain pending.

The plain negative checker rejects8px text, hidden/clipped text, missing outlines, removal
of content-sized rows and removal of the eyebrow floor. Source-bound raw logs/results,
PNG hashes and gate output will be archived with the final candidate. Main-board9px text,
other dynamic error/contrast states, native/OS scaling and full-AA/P1 acceptance remain open.
