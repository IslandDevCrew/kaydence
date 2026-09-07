# P1-G4 readability audit — pre-correction findings

Scope: real frontend at PR37 head 31da593, Codex in-app browser, dark theme,
900x600; browser fixture state, not the native app. This audit supplements
automated checks; it does not certify screen-reader support or native scaling.
Goal: clear daily dictation, discoverable controls, readable status and honest proof.

1. **Dictate — needs work.** `01-dictate-900.png` accepted after capture/readback.
   Strength: capture and full-width transcript have clear hierarchy. Risks:
   compact Cleanup is internally scrolled at default size, nav/secondary text
   is tiny, and sample recording/history/operational state lacks a visible
   global preview label. DOM measurement: nav/brand 9.76px, phase badge 8.16px,
   below locked11px. Preserve Candidate2 hierarchy while restoring the floor.
2. **Cleanup — needs work.** `02-cleanup-900.png` accepted after capture/readback.
   Strength: rules/examples and injection are separated. Risks: tiny secondary
   text and a compressed/colliding status legend; fixture green gates imply
   secure-field and round-trip proof not present in its contract. Next routine
   truth packet removes that overclaim; typography/legend reflow remains separate.
3. **Privacy — improved claims, readability pending.** `03-privacy-900.png`
   accepted after capture/readback. Retained-audio and source-only network copy
   is now honest. Dense small-print policy summaries and compact permission
   lists need readable reflow; visible presence of some rows is not proof every
   hidden/scrolling permission detail is discoverable. Keep the Constitution
   dialog and locked panel hierarchy; do not trade readability for fit.
4. **Setup — usable structure, small-text risk.** `04-setup-900.png` accepted
   after capture/readback. Next-step, four layout/engine/permission controls,
   and the final CTA are clear. Secondary labels, pricing detail and fixture
   disclaimer are very small; completed-looking fixture rows need prominent
   provenance. No purchase, license/default download change or live setup was run.

Recommendation order: finish the runtime/reference truth packet; isolate native
loading/error/retry and clearly label browser preview; restore the locked 11px
floor with bounded per-screen reflow; then repeat light/dark, three-layout,
keyboard/zoom and native WebView evidence. The existing design is retained.
Screenshots alone cannot establish contrast conformance, speech recording,
secure-field refusal, delivery timing, persistence correctness or phase closure.
This is a four-screen current-state audit, not an additional ship gate approval.
