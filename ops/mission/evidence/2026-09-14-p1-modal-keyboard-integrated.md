# P1-G4 keyboard custody: composed focus-color verification

Anchor `0be60925dd3e33387b4dba11588974a30517ae91`, based on focus PR47 head3212c05. This is a dependent local branch until PR47 merges; no phase closure.
Rebase intent: retain Unit A's explicit keyboard-cue selectors and the parent's ADR0020 `--focus-ring` colors. Widths, offsets and radii remain unchanged. Modal behavior/margin and the plain135-line checker are byte-identical to independently reviewed3d5f604.

## Evidence ledger

- Independent Judge: original3d5f604 report SHA2f874c648afd4e5a50b983b9044c559acba501063ee7a22dae9e26a6a6358497; integrated0be6092 report SHA6ca39baeddf1d6e7eaa6b42c1ee8866479ddda78c836ad3d62c3611feb105f14. Reviewer `linux_oss_direction` did not implement Unit A.
- Fresh integrated checked-in regression59/59 (56 behavior plus3 negative guards), independent60/60 (48 repeated lifecycle plus12 delayed-rejection cases), independent4/4 negative guards, and24/24 OS-reference color/cue checks PASS.
- Full local fmt/clippy,253 Rust tests,frontend type/lint,privacy/network,ADR and production build PASS. Synthetic4 measured budgets PASS; physical benchmark fields remain PARTIAL.
- Parent focus checker rerun96 scopes/768 states PASS; qualifying-side minimum3.745131, all-sample minimum1.632205 retained. No old color-only position-equality claim: Unit A deliberately adds native8px scroll-margin to reveal existing rings. No full-WCAG conformance claim.
- Companion raw-and-bindings.json.gz preserves26 verbatim records,7 external plain-runner hashes,7 current source bindings and88 PNG hashes:38 original/50 integrated. Sources are not compressed. Original RED/margin-negative history remains in the separate author packet.
- Reproduce: run the checked-in modal-keyboard-check.cjs at1444 with PLAYWRIGHT_MODULE and fresh CAPTURE_DIR as documented in the original record; the parent focus-check.cjs uses RESULT_PATH, without color-only BASELINE_PATH geometry assertion.

## Limits and publication

Old author manifest/source colors are historical3d5f604, not current after composition. Current sources bind in this integrated packet. Browser fixtures and explicit WebKit Option+Tab are not native OS/full-keyboard/scaling evidence; no settings or native IPC changed.
Unit B full-wheel reading/scroll restoration, downstream body fidelity, native/VM/physical/runtime/shipping gates remain separate. No Rust, dependency, IPC contract, download default, brand or policy change.
Actual merged-parent rebase, final publication Audit, own exact-head three-OS CI/CLEAN and expected-head merge remain required. P1-G4 remains pending.
