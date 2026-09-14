# P1-G4 modal keyboard custody — bounded local verification

Historical author packet at3d5f604: its original source manifest remains unchanged. Current focus-color composition and independent results are in `2026-09-14-p1-modal-keyboard-integrated.md`; original color/source bindings are not current after rebase.

- Base: PR46 `7ff203bd954e3154a709a9a70ee914b47f19dbee`; candidate served at `http://127.0.0.1:1444`.
- Scope: non-control-origin Tab confinement, explicit keyboard cue, and native focus visibility. No IPC, backend, command, copy, palette, ring geometry, navigation or identity changes.
- Status: local checks PASS; independent Judge, full standing gates, remote Audit/CI and merge remain pending. P1-G4 remains open.

## Reproduction and correction

- Click Privacy/Setup, open Constitution/Evidence, click modal text, then reverse Tab: Chromium escaped to BODY. WebKit Option+Tab could focus a dialog rather than its control. The same final checker against the untouched base at port 1440 reproduces **34 failures / 56 behavior cases** (22 pass).
- Managed controls now exclude disabled, negative-tabindex and non-rendered elements. Explicit unmanaged-origin and first/last boundary handling keeps actual Tab navigation inside the native dialog.
- Keyboard movement adds a temporary marker to the actual focused control. Existing screen-specific outline declarations are reused unchanged; the selector keeps native specificity. Marker/style removal independently removes the visible indicator in WebKit even when `:focus-visible` is false.
- Pointer input or blur removes the marker. Keyboard Escape/Close restores the actual enabled opener, otherwise the existing enabled Evidence fallback, with the same temporary cue. No delayed callback changes focus or command state.
- Before the native scroll-margin dependency, 44/52 cases passed; eight narrow Proof cases still clipped Export local proof's existing ring. Stable button bottom 281.140625 met a clip at 281, plus its unchanged 2px outline / 1px offset. Parent authorized allocating the existing shared 8px (`--space-2`) focus scroll-margin from held Unit B. It restores native focus visibility without explicit focus scrolling, wheel containment or ring/layout changes.

## Final checks

- Plain checker: `ops/mission/evidence/2026-09-14-p1-modal-keyboard-check.cjs`; source/checker SHA256 bindings are in the companion manifest and raw bundle.
- Chromium 151.0.7922.34 and WebKit 26.5: **56/56 behavior cases PASS**, covering both themes, 900x600 / 450x300 / 501x300, Constitution plus all three Evidence sections, forward/reverse wrapping, actual indicator and full clipping bounds, pointer clearing, keyboard Escape/Close return, and 900/450 held-IPC fallback with no late focus theft.
- **3/3 negative guards PASS**: removing the actual marker, actual authored selector, or native scroll-margin causes the original indicator/visibility oracle to fail. These are guard cases, not additional product-flow cases. The base cannot pass these correction-dependent guards.
- Existing actual-opener checker **164/164 PASS**, existing pending-fallback checker **24/24 PASS**. The fixture derives from the current source literal; no native IPC executes.
- Fresh `bash scripts/check-frontend.sh` PASS (direct tsc and eslint, zero warnings); `git diff --check` PASS. Parent owns fresh full standing gates and build.
- Six WebKit pointer-text/reverse-key screenshots show the authored cue while native `:focus-visible` is false: Privacy/Models at 900 and Proof at 450, light/dark. The narrow Proof ring is wholly visible after native scrolling.
- Re-run from repository root with `PLAYWRIGHT_MODULE=/Users/IDC2.5/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright APP_URL=http://127.0.0.1:1444 CAPTURE_DIR=<fresh-directory> node ops/mission/evidence/2026-09-14-p1-modal-keyboard-check.cjs`.

## Evidence and limits

- Companion `2026-09-14-p1-modal-keyboard/raw-and-bindings.json.gz` stores original raw JSON/log text and SHA256s, not compressed source code. It retains base RED, pre-margin eight-case RED, final GREEN, opener/pending regressions, and the earlier negative-harness failure. That margin negative originally reused an already-scrolled dialog; the final check reopens it and reproduces the real clipping without test-driven focus/scroll assignments.
- The temporary six-image capture runner stays plain at `/tmp/kaydence-keyboard-visual.cjs`; its hash and exact observed focus values are archived. Checked-in primary regression remains plain and repeatable.
- WebKit Option+Tab is labeled explicitly; no browser/system settings changed. Ordinary WebKit Tab skipping buttons is previously reproduced in unstyled control HTML; this is not native macOS full-keyboard acceptance.
- Held Unit B's complete wheel/read/scroll restoration remains separate and blocked until integrated and rechecked. This packet does not claim full-wheel reading, body fidelity, native WebView/VM/hardware acceptance, whole-WCAG conformance or phase closure.
- Ring colors remain the predecessor's exact recipes here. The independently reviewed focus-color unit is a required later integration parent; its derived colors must survive rebase and receive fresh verification.
