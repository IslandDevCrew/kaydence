# P1-G4 focus color: bounded candidate, not phase acceptance

Implementation/execution by `linux_oss_direction`; independent Judge and ADR-0020 acceptance pending. Base `0b16de0908b20e9dac64ebb9c2200a030ddaefad`; no push, PR, merge, heartbeat, Rust, dependency or backend change.
The four CSS outline colors now use 20% existing OS accent + 80% theme ink. Widths, offsets/insets, geometry, raw/reserved colors, identity and navigation are unchanged; accepted ADR-0018/0019 are byte-identical.

## Evidence and reproduction

Directory `2026-09-14-p1-focus-contrast/`: 26 manifest-verified artifacts, including 18 PNGs, final RED/candidate metrics and raw logs, historical first-candidate metrics/log, local gates and source/ranking JSON (gzip is data, not compressed test source).
`source-and-ranking.json.gz` binds all frontend source hashes, both served CSS payloads and the ranked shortest continuous cues with every retained low segment. `SHA256SUMS` verifies the archived files.
The plain checker is `2026-09-14-p1-focus-check.cjs` (SHA256 `69794ad901d134ba7835d1cf84872d1a2e3c9a1fbd3826a20d897bfacae6bece`). Run from the worktree with the installed Playwright module:
```sh
PLAYWRIGHT_MODULE=/Users/IDC2.5/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright APP_URL=http://127.0.0.1:1442 BASELINE_PATH=ops/mission/evidence/2026-09-14-p1-focus-contrast/red-metrics.json.gz RESULT_PATH=/tmp/focus-recheck.json.gz node ops/mission/evidence/2026-09-14-p1-focus-check.cjs
PLAYWRIGHT_MODULE=/Users/IDC2.5/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright node ops/mission/evidence/2026-09-14-p1-focus-negative.cjs
```
Baseline was served at 1439; final candidate at 1442. Chromium 151.0.7922.34, natural Tab/Enter/Escape, three OS selector references, both themes, 900x600 and 450x300 (CSS 200%-equivalent, not native scaling): 96 view/dialog scopes, 768 authored focus states per run.
Final checker: RED 560 states without a qualifying side → candidate 0; 0 unresolved paint and 0 frontend errors. Exact focused geometry compares equal. Missing indicators, weak paint and unsupported gradients fail the separate negative probes; the strong control passes.
First 60% accent candidate and its stricter predecessor checker remain historical: 582 RED, then 0 measured failures, but lowest retained segment 1.055539:1. Final measurement no longer rejects valid differing behind/adjacent colors; neither pixels nor low segments were dropped to improve results.

## Interpretation and remaining limits

[WCAG 2.1 AA 1.4.11](https://www.w3.org/TR/WCAG21/#non-text-contrast) requires 3:1 for visual state information against adjacent colors. [W3C guidance](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html#adjacent-colors) distinguishes inside/outside adjacency and permits nonessential component colors where identification remains clear.
Our inference, requiring independent visual judgment: substantial continuous qualifying sides can identify focus despite a low-contrast segment overlapping a neighboring selected control. One sampled side is measurement evidence, not an AA conformance rule. No [AAA Focus Appearance](https://www.w3.org/WAI/WCAG22/Understanding/focus-appearance.html) area claim is made.
Final qualifying-side sample minimum: 3.745131:1. All 507 measured low samples across 63 states remain archived, minimum 1.632205:1. A uniform opaque color cannot contrast 3:1 with both white and selected cobalt fill: its required luminance would be both ≥0.320693 and ≤0.300000.
Four light Cleanup 450 OS-selector cases conservatively record only a 7px flat side; whole PNGs show longer visible top/bottom cues across border transitions, which this uniform-band method undercounts. Those cues require the independent visual decision, not a relaxed threshold.
The original active inset navigation and tinted Setup Evidence findings improve in the final actual-pixel records. Inspect selected filled controls and tinted surfaces, not only page backgrounds.
The 12 previously unmatched measurement states are now included with actual adjacent paint; Privacy Constitution and Setup models/permissions/proof dialogs are included. This is browser fixture proof, not native capture, OS/VM, full-screen readability or P1-G4 closure.
Inherited preintegration short-window body clipping (including Setup Full) remains outside this color unit. The separately held shared-scroll WebKit pointer-to-keyboard missing-outline defect is not cured here; an absent ring is never a color PASS.
Local tsc/eslint, external-directory Vite build, strict token checks, privacy/network posture and diff checks pass. ADR status intentionally fails only because additive ADR-0020 remains Proposed; no gate exception or accepted ADR was edited. No cargo/native tests were run in this lane.
Next: independent cue/scope Judge, operator/ADR acceptance, then rebase onto approved integrated body/modal parents and rerun exact-head gates/three-OS CI before publication. Do not merge this pending candidate or mark P1-G4 done.
