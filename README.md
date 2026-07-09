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
Kaydence is local-first by design, not by slogan.

- **No telemetry.** There is no analytics beacon, product-usage feed, or hidden
  crash uploader in the app.
- **No account.** Core dictation does not require signup, login, sync, or a
  vendor identity.
- **No cloud screen capture, ever.** Kaydence may only read local context when
  you explicitly enable it, and cloud screenshot/screen-stream features are
  permanently out of scope.
- **No default network egress.** Model downloads, BYOK ASR/cleanup, update
  checks, and Relay/Harbor-style services must be explicit, user-enabled paths
  with reviewed endpoints. The source gate is `bash scripts/audit-network.sh --check`.
- **Your history is on your machine.** Audio, raw transcript, cleaned transcript,
  and exports live under the OS app-data directory until retention, delete, or
  purge removes them.
- **Context is memory-only.** Optional local context reading and local OCR are
  off by default, blocked for secure fields, never written to disk, and never
  transmitted (ADR-0006).

The release privacy gate is `bash scripts/check-privacy-posture.sh --check`.

## Status
P0 scaffolding is in place and P1 is active. The desktop app now has the Tauri
shell, typed Rust event/settings contracts, write-ahead audio persistence,
local history, hotkey runtime wiring, model readiness surfaces, and the locked
visual system. It is not a complete daily-driver dictation app yet: real ASR
adapters, full OS injection proof, first-run completion, and latency evidence
remain phase gates.
