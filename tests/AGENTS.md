# tests/ — Integration & Pipeline Tests

Unit tests live beside code; this directory holds what crosses module
boundaries.

## Required suites (merge gates marked ⛔)
- ⛔ `crash_recovery` — kill -9 mid-session at randomized stages; audio file
  playable, history row recoverable on next launch.
- ⛔ `short_utterance` — 0.3–1.5 s golden clips; zero empty/truncated finals
  (pitfall P1).
- ⛔ `event_sequences` — assert the SessionEvent order for: happy path,
  focus-change hold, secure-field hold, LLM timeout fallback, engine fallback.
- `inject_matrix.md` — the per-app manual/automated checklist (see inject/).
- `corpus/` — golden audio + expected outputs powering the ≥95% zero-edit
  metric; grow it with every real-world miss (a user-reported transcription
  bug is not fixed until its clip is in the corpus).

## Rules
Tests run headless on CI for all three OSes wherever feasible; injection tests that
need a real session run in the nightly job against a VM matrix. Flaky tests
are quarantined within 24 h and fixed within a week — a flaky gate is no gate.
