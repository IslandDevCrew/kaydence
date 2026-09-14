# P1-G4 modal opener custody — bounded frontend unit

## ORIENT / PLAN

- Base: `0f0c5668dfaf5d6ae2b9047609d5d504b32ea912`, branch `mission/p1-modal-opener-custody`; P1-G4 remains pending.
- Scope: preserve the actual Privacy Constitution and all Setup Evidence triggers across pointer activation; preserve primary commands, order, disabled rules and non-modal routes. No CSS, palette, identity, Rust, IPC implementation, dependencies or browser settings change.
- Real pointer clicks in WebKit leave BODY active; document.activeElement is therefore not reliable trigger evidence. Async primary actions also disable their button before native modality captures focus in Chromium. Explicit trigger custody is required, not a test-induced focus workaround.
- The later shared modal scroll/containment packet (`a16a9da`) is NOT on this base. Its initial-focus, wheel, keyboard-wrap and restored-opener scrolling obligations remain separate integration prerequisites.

## BUILD / RED → GREEN

- Two views retain the actual `event.currentTarget`; Modal restores it after native close only when connected/enabled. Setup supplies its existing Evidence button as logical fallback when the actual opener is unavailable. The fallback is synchronously revealed, with no delayed focus action and no command-state changes.
- Fresh exact-base RED: 164 cases, 72 PASS / 92 FAIL, zero frontend errors. Chromium 66/82 PASS; WebKit 6/82 PASS. Every failed case lost the actual opener to BODY; the 12 non-modal/disabled route guards passed.
- Final pointer suite: **164/164 PASS**, Chromium `151.0.7922.34` and WebKit `26.5`. Privacy plus all three direct Setup entries and hotkey primary cover light/dark at 900×600, 450×300 and 501×300, both Escape/Close. All four async primary kinds and setup/dictation/complete route guards additionally cover both themes at 900×600.
- Assertions retain the original clicked DOM identity, whole visible focus bounds against viewport/clipping ancestors, selected section, exact primary IPC intent and no unexpected commands/errors. Controlled source-derived snapshot fixtures never invoke a native backend; settled-action checks explicitly await existing disabled-state clearance.
- Held-IPC discovery: original 4/4 cases returned BODY while primary remained disabled; BODY persisted after release. This RED is retained, not labeled acceptance. Original 37-line probe SHA256: `acdea93a6fe7024181ea7bed98a3f51e882df9d233ebc15496e58d6ddfbc55fd` (superseded by checked-in stronger probe).
- Stronger fallback RED **0/24 PASS** → GREEN **24/24 PASS** across both engines, both themes, three sizes and both dismissals. Checks prove exact enabled Evidence fallback identity, whole focus visibility, original command/args exactly once, disabled primary respected, and no focus theft after delayed IPC resolves.
- Existing Chromium modal regression: **4/4 PASS** (Tab/Shift-Tab, Close, Escape, opener return at 900×600/450×300). Frontend typecheck and eslint gates PASS before and after. Full standing gates/build and independent Judge remain main-owned prepublication gates, not inferred from these results.

## Reproduce / bindings

From the worktree root with real Vite at `http://127.0.0.1:1440`, set `PLAYWRIGHT_MODULE=/Users/IDC2.5/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright`, `APP_URL=http://127.0.0.1:1440` and a fresh `CAPTURE_DIR` outside archived evidence; run:

1. `node ops/mission/evidence/2026-09-14-p1-modal-opener-check.cjs`
2. `node ops/mission/evidence/2026-09-14-p1-modal-pending-check.cjs`
3. `node ops/mission/evidence/2026-09-07-p1-dialog-keyboard-check.cjs`
4. `bash scripts/check-frontend.sh`

`2026-09-14-p1-modal-opener/SHA256SUMS.txt` binds final source/checkers, this record, 14 PNGs and 17 gzip-compressed raw JSON/logs. `red.*` and `fallback-red.*` are real failing runs using the checked-in corresponding checkers. No compressed source scripts are checked in. Representative green Privacy/Setup and fallback PNGs were visually inspected; this is not complete screen-family acceptance.

## Preserved limitations / Audit handoff

- Original independent-executor WebKit pointer RED and ordinary-Tab RED are retained as `webkit-original-pointer.*` / `webkit-default-tab.*`. Source report remains `/tmp/kaydence-webkit-compat-UgTvSi/COMPATIBILITY.md`, SHA256 `e95b0900bcc41bc76dee616ae26357191f6d4bb54b017079586a16136cd75776`; original runners `pointer-return.cjs` SHA256 `1c1b81622e167ba54d170e715687be61c1e84d4efed7988f61dc91333c65779a`, `check.cjs` SHA256 `a2f59534c21c275cb0c748f7c89d01c6dd4422b1e9cf336f30c70c4ff6225236`.
- Unmodified WebKit ordinary Tab did not traverse the app buttons; Option/Alt+Tab did. This remains a RED compatibility/configuration question, not waived by pointer success. No settings changes or QA focus/scroll assignments were used to pass either owned suite.
- Browser evidence does not establish native WebView/VM/OS/permission behavior. Broader Privacy/Setup readability, native default keyboard behavior, shared modal scrolling, screen-family fidelity and P1-G4 closure remain separate. Reconcile actual/fallback custody with the later shared revealFocus implementation, then rerun both suites and its wheel gates on the integrated tree.
- Independent Judge and remote Audit/3-OS CI/merge are pending at author freeze. No PR, push, main/state/journal/renderer change or phase closure is performed by this work unit.
