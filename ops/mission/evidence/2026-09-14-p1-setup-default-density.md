# P1-G4 Setup default-window density — local verification

Original verification base: `0ef4e6cea7b86482a472faee852e12a96cc821e4`. Product scope is only `FirstRunView.css`, SHA256 `f9b045c5ff935e86cd91c49b914616a3dd39fc4d41dfb40dedafe8086e21ebf9`; baseline CSS was `b8401fb04d9aff30a8827973643f623610f9cde2f0349cbd252241b6910df886`. Independent bounded Judge passed at `3f35a9d`; actual-parent integration, final publication Audit, own CI, native/VM acceptance and overall P1-G4 remain pending. This is not phase closure.

Within the existing max-height650/min-width681 media scope only: configuration-row vertical padding3→2px (`--space-05`); next-step banner5→3px (`--space-075`); license-card6→4px (`--space-1`). Horizontal padding, fonts/line heights, control sizes, gap/padding around the workspace, DOM/navigation, board composition, colors, radii and identity remain unchanged. No TSX, IPC, Rust, dependency or mission-state changes.

## Discriminating evidence

- Actual 900×600 Chromium151.0.7922.34/WebKit26.5, light/dark, macOS/Windows/Linux reference selections: baseline12/12 RED, workspace588/564 and action bottom615.03125. Candidate12/12 PASS, workspace564/564 at scrollTop0; action bottom591.03125, existing2px+1px outline fits. All103–104 text nodes /1,246–1,259 nonspace characters are initially visible and at least11px. Both controls fit; only enabled controls receive keyboard focus. The reference primary remains intentionally disabled.
- Exact baseline/candidate comparison12/12 PASS: text/font inventory, native control sizes and DOM order, and raw accent inputs unchanged;24px reclaimed. Screen-family10 hierarchy is preserved; archived board imagery is not itself a native or current-copy acceptance gate.
- Actual-bundle full reading and natural forward/reverse cues:32/32 at900×600 and450/501/600×300, default plus long blocked fixture, both themes/engines;1,368 whole cues. Additional681/700/899×300 default/blocked cases24/24 with1,032 cues. All text reaches the real scroll bottom; no positive style, scroll-position or forced-focus override. WebKit uses Option+Tab, explicitly browser compatibility rather than native full-keyboard-settings proof.
- Negative controls14/14 reject the intended defect: old spacing, hidden text,10px text and opaque ancestor pseudo-paint in both engines (8); clipped long copy, hidden scroller and unrelated overlay in both engines (6). The unrelated overlay rejects at actual wheel receipt, not glyph proof. The guarded reader rejects unsupported ancestor pseudo painting; it does not claim general arbitrary-CSS visibility proof.
- Shared focus-color regression96 scopes/780 states PASS; qualifying-side minimum3.7451306265:1. Adjacent-button overlap samples remain disclosed by the inherited checker, not an all-pixel/AAA-area claim. Fresh tsc/eslint and direct app-cwd Vite production build PASS; build output is outside the repository. Author implementation checks ran no Cargo or native commands; root's separate standing-gate run is archived below.

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

## Independent review and publication packet

Independent Judge `/tmp/kaydence-setup-density-judge-20260914/JUDGE.md` SHA256 `68db27b5780a32053f139c7ab0ceba817425b1100f6fb09362b16991a8d68ec1` is PASS at exact `3f35a9d5f577ddc8a16ed8174f595ee8cf2fc9e6`: separate12 actual cases/192 cues,12 single-row-padding counterfactuals, owned12 reruns, full reader32/1,368 cues and intermediate24/1,032 cues. Zero app errors; no independent color-matrix rerun or native/full-fidelity claim. Its source-identical precommit run and missing-PLAYWRIGHT_MODULE launch error remain historical, not product RED or a substitute for the committed-head repeat.

New `publication-evidence.json.gz` preserves the independent manifest's231 files/220 PNGs byte-for-byte plus its manifest, original author manifest, root standing log and root visual-review record:235 hashed artifacts total. Manifest SHA256 `6d12fb9ad86ff65b5e79e653005c483600eb1e63e0b263811cab389f06d1aba6`; all seven source and five external plain-runner bindings verify. No source scripts are compressed. Three direct Judge PNGs show complete default, natural primary focus and the discriminating one-rule clipping. Original browser bundle `1c190fbb…` and all earlier REDs are unchanged; its old nine-entry manifest is preserved inside the new packet.

Root standing `/tmp/kaydence-density-3f35-standing-20260914.log` SHA256 `5a109eec4ea253425c20bf7608bbc56dcb29903a70df95c2d6d485930b662f5c` names the same reviewed head and exits0:253 Rust tests, fmt/clippy, frontend, privacy/network, ADR checks, four measured synthetic budgets and direct build. Synthetic physical/unmeasured metrics explicitly remain PARTIAL. Root visual review `/tmp/kaydence-density-main-review-20260914.md` SHA256 `9775244228b88f7d71f3b8c62b2bede66d526badf82cf532963cd8a0264e5ed5` accepts bounded default/action visibility, not Privacy recomposition or phase closure. This addition is author packaging of independent/root evidence, not a new author-issued Judge; later copy-parent integration and publication gates remain with root.

## Local copy-candidate integration — not the actual merged parent

Root rebased unchanged density onto capability-copy candidate `a75b184b5e294dd78ab36286d0120acfd6b76e62`; tested combined head `ad9532a6f49989d9a9178c022f9fad5c065e2d69` is still a locally prepared candidate. Independent integration report `/tmp/kaydence-density-copy-parent-20260914/JUDGE.md` SHA256 `b75e59110905392163d07229d9f81afc2cba5334f11f4a8774cb568d171970af` passes12 default cases and32 full natural-reader cases/1,368 cues, both engines/themes and default/long-blocked states, with zero app errors. All12 default text/font/visibility/geometry inventories exactly match standalone; all ten source bindings and external reader8408b566 verify. This does not rejudge all capability-copy claims or refresh the omitted intermediate24/negative controls; those remain standalone evidence.

Third immutable `copy-candidate-evidence.json.gz` contains103 hashed artifacts: all100 independent-manifest files/92 PNGs, its manifest, root standing log and the prior thirteen-entry packet manifest. Independent manifest SHA256 `82459ad8b513d9bbd2ab0ac2134e4e5ac39d34ca16fc72671a17d57f867f9793`. Two direct PNGs show combined default and narrow blocked reading. Both earlier bundles and their RED/history bytes remain unchanged; source runners remain external plain path/hash references, not compressed source or a portability claim.

Root `/tmp/kaydence-density-copy-parent-standing-20260914.log` SHA256 `238dbdca803ed5574b2486be83f9bd73f499d59fd7489773c3cf49520ad5fab5` names `ad9532a` and exits0:253 tests plus fmt/clippy/frontend/privacy/ADR/synthetic4/direct build; unmeasured physical metrics remain PARTIAL. This is evidence packaging only, with no new browser/product/state action. Actual merged-copy-parent reconciliation, final publication Audit, fresh exact-head three-OS CI/CLEAN and expected-head merge/heartbeat remain required. Browser fixtures and WebKit Option+Tab do not prove native readiness, full AA/fidelity or P1 completion.
