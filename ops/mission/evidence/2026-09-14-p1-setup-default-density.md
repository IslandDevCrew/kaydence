# P1-G4 Setup default-window density — local verification

Base: `0ef4e6cea7b86482a472faee852e12a96cc821e4`. Product scope is only `FirstRunView.css`, SHA256 `f9b045c5ff935e86cd91c49b914616a3dd39fc4d41dfb40dedafe8086e21ebf9`; baseline CSS was `b8401fb04d9aff30a8827973643f623610f9cde2f0349cbd252241b6910df886`. Independent Judge, actual-parent integration, CI, native/VM acceptance and overall P1-G4 remain pending; this is not phase closure.

Within the existing max-height650/min-width681 media scope only: configuration-row vertical padding3→2px (`--space-05`); next-step banner5→3px (`--space-075`); license-card6→4px (`--space-1`). Horizontal padding, fonts/line heights, control sizes, gap/padding around the workspace, DOM/navigation, board composition, colors, radii and identity remain unchanged. No TSX, IPC, Rust, dependency or mission-state changes.

## Discriminating evidence

- Actual 900×600 Chromium151.0.7922.34/WebKit26.5, light/dark, macOS/Windows/Linux reference selections: baseline12/12 RED, workspace588/564 and action bottom615.03125. Candidate12/12 PASS, workspace564/564 at scrollTop0; action bottom591.03125, existing2px+1px outline fits. All103–104 text nodes /1,246–1,259 nonspace characters are initially visible and at least11px. Both controls fit; only enabled controls receive keyboard focus. The reference primary remains intentionally disabled.
- Exact baseline/candidate comparison12/12 PASS: text/font inventory, native control sizes and DOM order, and raw accent inputs unchanged;24px reclaimed. Screen-family10 hierarchy is preserved; archived board imagery is not itself a native or current-copy acceptance gate.
- Actual-bundle full reading and natural forward/reverse cues:32/32 at900×600 and450/501/600×300, default plus long blocked fixture, both themes/engines;1,368 whole cues. Additional681/700/899×300 default/blocked cases24/24 with1,032 cues. All text reaches the real scroll bottom; no positive style, scroll-position or forced-focus override. WebKit uses Option+Tab, explicitly browser compatibility rather than native full-keyboard-settings proof.
- Negative controls14/14 reject the intended defect: old spacing, hidden text,10px text and opaque ancestor pseudo-paint in both engines (8); clipped long copy, hidden scroller and unrelated overlay in both engines (6). The unrelated overlay rejects at actual wheel receipt, not glyph proof. The guarded reader rejects unsupported ancestor pseudo painting; it does not claim general arbitrary-CSS visibility proof.
- Shared focus-color regression96 scopes/780 states PASS; qualifying-side minimum3.7451306265:1. Adjacent-button overlap samples remain disclosed by the inherited checker, not an all-pixel/AAA-area claim. Fresh tsc/eslint and direct app-cwd Vite production build PASS; build output is outside the repository. No Cargo or native commands run by this unit.

## Recompute and custody

Run from the worktree root with `PLAYWRIGHT_MODULE` pointing at the installed Playwright package and fresh `CAPTURE_DIR`; browser URL is `APP_URL=http://127.0.0.1:1432`.

```sh
node ops/mission/evidence/2026-09-14-p1-setup-density-check.cjs
node /tmp/kaydence-setup-main-board-oracle-v2/natural-board.cjs
# Repeat the reader with WIDTH=681, WIDTH=700, WIDTH=899.
# Density negatives: THEME=light LANE=macOS NEGATIVE=spacing|hidden|font|paint.
# Reader negatives: WIDTH=450 MODE=blocked THEME=light NEGATIVE=clipped|scroller|occlusion.
node ops/mission/evidence/2026-09-14-p1-focus-check.cjs
bash scripts/check-frontend.sh
# From apps/desktop, use an external output directory:
node_modules/.bin/vite build --outDir /tmp/kaydence-setup-default-density-20260914/build
```

`2026-09-14-p1-setup-default-density/browser-evidence.json.gz` contains236 hashed raw artifacts (208 PNGs,28 JSON/log records), with per-file encoding/data and current source/dependency hashes. Five direct PNGs expose representative before/after/action-cue views. Plain current checker stays checked in; supplemental temporary scripts are path/hash-bound in the bundle, not compressed or claimed portable. Reader8408b566 depends on the inherited checked-in Evidence helper464aa376. Evidence packaging supplies byte custody, not self-enforcing or fresh-clone independent acceptance.

Raw run root: `/tmp/kaydence-setup-default-density-20260914`. `red` used the initial plain checker0d89afd3 and exits1 for the genuine at-rest overflow before keyboard assertions. Its first candidate run (`green`) also exposed an author test error demanding focus on disabled reference actions; that failure remains preserved. The final checkera6e8c821 explicitly asserts the existing disabled boundary and exits0 for all12 cases; current negative spacing still exits1 at the unchanged fold assertion. Initial root-cwd Vite invocation failed to resolve index.html (command setup error, not product RED); corrected app-cwd build exits0. Current reader32/intermediate24/focus/frontend/build exits0; all14 deliberate negatives exit1 with their stated reasons. No historical failures were reclassified or removed.
