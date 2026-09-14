# P1-G4 Cleanup capability copy — integrated local verification

ORIENT/PLAN: copy-only original8d06b19 rebased clean to3574189 onto exact integrated
Modal/status/Cleanup/Dictate base5a6e53e1859ab90a2cbeed8ca290fd18a651e5fa.
P1 remains active and P1-G4 pending. This is the authorized frontend wording and
presentation unit, not a cleanup-engine/prompt/dictionary-policy change.
Earlier2026-09-14-p1-cleanup-copy evidence remains byte-for-byte historical; its
old-body visual limitations are not represented as current integrated acceptance.

Frontend-testing/debugging and test-first investigation used existing Playwright
Chromium because the Browser plugin/skill is not available. Real Vite1437 serves
this worktree. Flow: natural Tab/Space selects a Dictate dial, real wheel exposes
every copy fragment, Configure opens Cleanup, full rule/example copy is read,
Save becomes keyboard-reachable, and navigation preserves the selected dial.
No test assigns focus/scroll offsets, forces clicks or injects substitute CSS.

RED: at450x300 after reading Raw, Configure bounds ended299.625px plus its1px
outline, clipped by the300px viewport. focus-red.log.gz and focus-failure.png retain
the failing case (pre-fix DictateView.css SHA000e109a, checker SHAca70fc0a).
BUILD: one existing control-panel button rule gains scroll-margin-block using the
existing8px token. No dimensions, type, palette, identity or navigation changes.
The identical narrow path then passed all three dials without relaxing the gate.

Copy remains identical to the previously source-reviewed2TSX files. Rust cleanup
and pipeline sources are unchanged from86a389a: Light/Full use RuleCleaner; Raw
bypasses it; DictionaryPass helper exists but production construction is empty.
Examples remain literal matches to existing Rust test input/output pairs, not
browser-executed cleanup results. No Rust/backend/dependency file was changed.

Validation commands use PLAYWRIGHT_MODULE pointing at the bundled runtime and
APP_URL=http://127.0.0.1:1437; CAPTURE_DIR names this packet's integrated directory.
Run plain cleanup-copy-check.cjs, cleanup-copy-visible-check.cjs, inherited
cleanup-natural-check.cjs and dictate-readability-check.cjs under ops/mission/evidence.
Frontend gate is bash scripts/check-frontend.sh. Direct Vite build runs from
apps/desktop, with output under /tmp; an initial wrong-root invocation is retained
as a resolved command error, not a product/build pass. No tracked dist changed.

GREEN: original18 fixture-state checks plus72 natural Dictate dial/layout cases
and24 complete Cleanup copy reads pass at900x600,450/500/501x300 in both themes.
The inherited24-case three-OS-reference Cleanup full-text/action walk and54-case
Dictate regression pass. All six900x600 meter readings remain17.140625/17.125px;
no active copy test reports frontend errors. Typecheck/lint and corrected direct
build pass. Full-copy gates retain font>=11px, complete clipped-ancestor ranges,
center hit-testing, actual wheel reachability and whole native focus outlines.

Final source, logs, screenshots and checks are bound by the adjacent manifest.
Independent integration Judge PASS atbcd744e: fresh72+18 and independent18 flows,
bad-copy/dictionary/hidden/clipped negatives reject. Exact-wheel old margin RED
299.625+1px versus final291.625+1px GREEN. Initial private ellipsis false alarm
and inconclusive alternate-wheel negative remain disclosed in archived Judge.
Main full fmt/clippy/253Rust/frontend/privacy/ADR gates PASS; physical bench PARTIAL.
Final parent integration, Audit and exact-head CI/PR/merge remain pending.
Browser CSS viewports are not native OS/WebView, hardware or whole-P1-G4 proof.
