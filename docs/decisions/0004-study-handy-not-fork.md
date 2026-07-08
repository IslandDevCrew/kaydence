# ADR-0004: Study Handy's patterns; do not hard-fork

- **Status:** Accepted
- **Date:** 2026-06-12
- **PRD items affected:** Phase 0/1 velocity

## Context
Handy (MIT, ~20k stars) already solves Tauri + cpal capture + whisper.cpp +
Parakeet ONNX + Silero VAD + global shortcuts cross-platform, and its author
explicitly optimizes for forkability. But its event model, raw-only pipeline,
and paste-based injection differ from our architecture (typed SessionEvent
contract, cleanup stage, native-injection-first).

## Decision
Treat Handy as a reference implementation: adopt its dependency choices and
platform workarounds, vendor specific snippets only with MIT attribution
preserved, but build our pipeline on our own event contract from day one.

## Alternatives considered
- Hard fork: fastest week 1, but we'd spend Phase 2 fighting an architecture
  that has no cleanup stage or injection abstraction.
- Clean-room ignore: wastes their hard-won platform fixes (e.g., macOS global
  shortcut rewrite).

## Consequences
Phase 0 includes a structured read of their `src-tauri`; an
`ATTRIBUTIONS.md` is mandatory the moment any code is vendored.
