# P1-G4 focus color: bounded candidate, not phase acceptance

Original implementation/execution by `linux_oss_direction` on0b16de0; rebased unchangedee86500 onto merged PR46. Independent integrated Judge PASS at05fc71a enables scoped ADR-0020 acceptance under the recorded operator approval; no Rust/dependency/backend change.
The four CSS outline colors now use 20% existing OS accent + 80% theme ink. Widths, offsets/insets, geometry, raw/reserved colors, identity and navigation are unchanged; accepted ADR-0018/0019 are byte-identical.

## Evidence and reproduction

Directory `2026-09-14-p1-focus-contrast/`: 30 manifest-bound artifacts include the original18 PNGs and historical measurements; `integrated-data.json.gz` additionally binds42 natural/18 negative PNGs, verbatim independent raw data/logs, current sources at8f49272 and external plain-runner hashes (gzip is data, not compressed test source).
`source-and-ranking.json.gz` binds the historical pre-PR46 sources, served CSS and ranked cues with every retained low segment. Current integrated bindings are separate; inherited Modal/Privacy/Setup TSX changed on rebase, not the focus CSS/checkers. `SHA256SUMS` verifies archived files.
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
Independent review resolves all30 conservatively short controls:60 continuous top/bottom runs, every actual matched foreground/adjacent ratio>=7.330716. The four one-side cases have46/35/26/46px bands on BOTH sides, min9.0344;42 natural captures support the visible-cue judgment without dropping low overlap samples.
The original active inset navigation and tinted Setup Evidence findings improve in the final actual-pixel records. Inspect selected filled controls and tinted surfaces, not only page backgrounds.
The 12 previously unmatched measurement states are now included with actual adjacent paint; Privacy Constitution and Setup models/permissions/proof dialogs are included. This is browser fixture proof, not native capture, OS/VM, full-screen readability or P1-G4 closure.
Inherited preintegration short-window body clipping (including Setup Full) remains outside this color unit. The separately held shared-scroll WebKit pointer-to-keyboard missing-outline defect is not cured here; an absent ring is never a color PASS.
Integrated Judge reruns96/768,18 negatives and42 natural captures PASS. Original local ADR gate RED was due to then-Proposed0020; acceptance is now recorded after review, without changing accepted0018/0019 or granting an exception. Main standing gates at8f49272 PASS: fmt/clippy,253 Rust tests,frontend type/lint,privacy,ADR,synthetic4 metrics,production build; physical benchmarks remain PARTIAL. Final independent Audit PASS atde33989 (report SHA114f95985a5882b047832a8ed48f346e65b5eac7329def94da2b3acec6beb62d); publication-audit.json.gz preserves verdict/bindings and extra37 inherited browser checks PASS. No product changed after review.
Own exact-head three-OS CI/CLEAN and expected-head merge remain required. Later body/keyboard/scroll integration must revalidate this color treatment; full-AA/native/P1-G4 acceptance is not claimed.
