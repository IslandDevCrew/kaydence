# Privacy History focus — local correction, not acceptance

Independent shared-modal review found a separate body defect: View History's
button was visible at450px, but its4px focus outline ended beyond the viewport.
The new natural Tab guard checks the complete ring against viewport and overflow
ancestors; it reproduces12 failures at450/500 in both themes/all OS selectors.
An8px scroll margin on that single audit-header button fixes all24 cases without
changing the outline, shrinking it, moving focus artificially or changing actions.
Raw red/green logs and source/checker hashes are in the adjacent manifest.
The guard returns by Shift+Tab to Setup and retains the existing navigation check.

This is layered on6f22aec, not merged main. Shared modal integration, current
contrast/body integration, fresh gates and independent Judge remain mandatory.
No Rust, dependency, semantic, identity or phase/native acceptance change.
