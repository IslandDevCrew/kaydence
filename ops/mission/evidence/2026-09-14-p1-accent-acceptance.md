# ADR-0018 / targeted accent acceptance — 2026-09-14

Explicit operator scope is preserved by PR41. Independent Judge polish_review PASS
on frozen productb22d282, unchanged after docs-only rebase34f182b/heartbeat5120316.
Own rerun:90 scopes/930 samples, zero accent failures, blocked pointers or frontend
errors, minimum4.951315354177956:1. Actual hover/focus and OS reference→runtime
switches pass. All65 original manifest entries match; source/test hashes bind that
manifest (SHA d61d5a366b2c293f5cae9240dbc663c5a3a81d285af8233089ae1b69e05ad16a).

Main AUDIT: narrow operator go + ADR0018 Accepted; original raw accents/white,
reserved colors, identity/layouts/type unchanged. One explicit overflow prerequisite
restores pointer access. No Rust/IPC/dependency edits, no non-negotiable weakened.
Local fmt/clippy/253 Rust tests/frontend/privacy/ADR/build and39 browser regressions
PASS. Benchmark measured fields PASS, physical fields PARTIAL. Main inspected four
rendered cross-theme/OS views; this is not native, app-wide AA or P1-G4 closure.
27 nonaccent low-contrast and18 unsupported opacity occurrences remain queued.

Additional archive SHA256 (under2026-09-14-p1-accessible-accents):
- standing-gates.log.gz 9d6e43e67c90bc553ba8be45ea3539a4d961d4fad99f3e8eb4e8d9479d0110da
- browser-regressions.log.gz 99bfbc26c87f2e412c70dd7388bad387972d99bbd955ca798de8186abd9d0606
- adr-status.log.gz 905ba5f6fe40d7693753ce5db54aca5a529460d97c2bf864375ea00841e46592
- pr41-premerge.json.gz 7c3d993bf7490eca7f3c764aba4ea7862d4945fb2bed111e22ac2b0976d5dafa

Pre-merge requirement: all3 exact-head OS CI checks SUCCESS and CLEAN, merge commit,
then state/journal/render heartbeat. No native/phase/shipping claim accompanies this ADR.
