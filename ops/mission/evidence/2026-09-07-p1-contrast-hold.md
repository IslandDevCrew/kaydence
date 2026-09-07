# P1-G4 Astra repass: design ruling and resumable checkpoint
Date: 2026-09-07. Status: HOLD for the contrast decision; P1-G4 remains pending.
Scope: frontend presentation and evidence only. No Rust, model, dependencies, native actions or default-download changes.

## Four-step audit
1. Grounded the pass in live mission state, locked Editorial Precision reference and screen-family requirements.
2. Captured real browser-rendered frontend screens, then tested text ranges, scrolling, focus, state provenance and footer geometry.
3. Independent Judge reproduced hidden interactions and rejected false greens; corrections were rerun with negative regressions.
4. Kept inherited reference contradictions separate from routine drift. Neither historical taste approval nor green CI closes the unresolved gates.

## Blocking reference contradiction: enabled text contrast
The locked reference `2026-08-09-design-gauntlet-winner-locked-v1.html:43-45` specifies aqua and white `--on-accent`.
The same design contract `ops/mission/design-relock/idea.lock.json:32-33` requires AA contrast.
The punchlist requires the same root tokens/component grammar. This is not merely implementation drift.
The enabled Save Changes action is normal-size text: 8.96px in PR40's frontend, 11px in the prepared Cleanup packet.
Both are below the large-text threshold. Restoring the 11px floor does not cure the contrast failure.

| Shared white foreground over selected accent | Measured ratio | 4.5:1 minimum |
|---|---:|---|
| macOS aqua #14a7a1 | 2.969510:1 | FAIL |
| Linux emerald #1f9d63 | 3.464649:1 | FAIL |
| Windows cobalt #0067c0 | 5.673754:1 | PASS |

Both light and dark themes reproduce these values. Foreground/background are opaque; button is enabled.
Main reproduced against exact PR40 candidate d21ace91ea9f50ff4a0c1c492a1eaea7e6a17721 on Vite1431.
Independent Judge reproduced the same ratios at the restored 11px size on the Cleanup packet (Vite1432).
The repeatable check intentionally exits1 (four failing cases); six native browser-capture crops and computed data are hashed.
These are selected frontend OS references, NOT actual macOS/Windows/Linux WebView or hardware proof.
[W3C SC1.4.3](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html) supplies the normal-text minimum.

## Ruling requested, not accepted
Authorize a narrowly scoped design ADR to select a contrast-compliant shared control/foreground treatment while retaining
the aqua/cobalt/emerald brand accents, shared navigation, 8px cards, identity and reserved blue/gold semantics.
The exact treatment must be compared in both themes/all three accents and independently measured before acceptance.
No palette, foreground, design lock or accessibility threshold has been changed. No ADR status was advanced.
Do not interpret the earlier general activation/merge go as this missing specific ruling.

## Screen findings and prepared work
| Screen | Routine correction prepared | Remaining truth / acceptance issue |
|---|---|---|
| Dictate, all3 layouts | >=11px body, real narrow scrolling, exclusive popovers/Escape, docked meter below900 | Raw still shows active checks; Full copy promises an unimplemented profile rewrite; native capture projection absent |
| Cleanup | >=11px body, intrinsic sections, unclipped full copy, legend separation and real scrolling | Dictionary capability not production-wired; profile wording/examples overclaim current rules; accent contrast HOLD |
| Privacy | >=11px body, intrinsic short-window grid, reachable footer/policy details | Integrated independent Judge, CI and native fidelity still required |
| Setup plus evidence dialog | >=11px body/dialogs, restored guidance/hotkey controls, intrinsic status column, complete focus ring | Integrated independent Judge, CI, real permissions and first-dictation proof still required |

Prepared branches are isolated local work, not merged production features:
- `mission/p1-g4-dictate-readability`: 5652998c4ea0360e8b3116ef72723db17a25ea1c, base697de42; bounded independent Judge PASS after54 cases, final source hashes in its packet. Rebase onto currentmain before publication.
- `mission/p1-g4-cleanup-type`: 6b0a396dedf9637a86d1b72c7e0993d3be4e7aa8, based on PR40d21ace9; integrated local gates/browser24 PASS, independent Judge still pending.
- `mission/p1-g4-privacy-type`: 6f22aec839ecbf546e3e5178eaa2f00d02e46363, base697de42; 240 changed text lines,24 local cases PASS; integration/Judge/CI pending.
- `mission/p1-g4-setup-type`: 74272b00e9716dd53ecdc38def3a6bf24e0d08dd, base697de42; 268 changed text lines,24 local cases PASS; integration/Judge/CI pending.
All worktrees are under `/Users/IDC2.5/Documents/Kaydence/.worktrees/`. Preserve canonical checkout/cache.
Do not combine their diffs into one oversized PR. Every rebased packet needs fresh source bindings and required CI.

## Additional cleanup truth audit (read-only, not implemented)
Runtime `cleanup/mod.rs:20-29` routes Light and Full through RuleCleaner; its existing test386-397 binds the same floor.
Raw returns unchanged text and no CleanFinal event. Dictate's unconditional five checkmarks must not imply applied rules.
DictionaryPass exists, but production pipeline uses an empty pass; custom terms are only supplied in tests.
AppSnapshot exposes the default dial, not active profile rule results. Replace unsupported "Profile rule" claims.
Illustrative examples exceed current semantics: source-derived results are respectively:
`This is a test lets see how it works.`; `So basically like the point is this.`;
`I want to go to the store no the market.`. These are source-derived, not newly executed Rust proof.
Use explicit examples already bound by existing tests; do not change cleanup/prompt behavior in a copy correction.

## Remaining mission boundaries
Live capture UI needs a separately owned backend projection/bootstrap/reopen contract; see the capture-bridge boundary note.
Buzz's Rust/injection lane remains untouched. Never poll recent_history for a clock or treat Started.at_ms as Unix time.
ADR0016 shipping go, ADR0017 default-download go, physical P1G3/P0-8 evidence, native fidelity and later-phase authority remain open.
Pinned Voxtype/Handy research is already onmain; it informs capability truth and failure-path tests, not copied dependencies.
No production-ready, native-proof, P1-G4 completion or P2 activation claim is made.
