# Kaydence (working title — domain: kaydence.io)

Local-first, cross-platform AI dictation for macOS, Windows, and Linux.
One hotkey → speak → clean text in any app. Private by architecture.
Plus **Whisper-Ahead**: local predictive completion that stays off the line
you're speaking on.

**Speak anywhere. Ship clean text. Never leak, never lose, never lag.**

## Start here
- New (human or agent)? Read `AGENTS.md` (the constitution), then `docs/PRD.md`.
- Building? The whole project is driven by one command: **`/kickoff`** (Claude
  Code) — see `prompts/KICKOFF.md` and `prompts/BUILD-LOOP.md`.
- Bootstrapping the toolchain? `SCAFFOLD.md`.
- Why decisions were made? `docs/decisions/` (ADR-0001 … 0008).

## Privacy, plainly
No telemetry. No account. No cloud screen capture, ever. Nothing leaves your
machine, and nothing is persisted, without your consent. Optional, on-device,
in-memory context reading can sharpen predictions — it never touches disk or the
network (ADR-0006). Your audio and transcripts live on your disk, period.

## Status
Docs-first scaffold complete: constitution, PRD (incl. the Whisper-Ahead epic),
architecture, ADRs, model registry, bench harnesses, CI matrix, and the
autonomous build loop are in place. Phase 0 (toolchain bootstrap) is next — run
`/kickoff`.
