# Kaydence Codex Super Goal Prompt

Use this prompt when opening a fresh Codex session to continue Kaydence without
losing the thread.

```md
/goal Continue the Kaydence end-to-end build from the real local repo.

Repo: /Users/IDC2.5/Kaydence/kaydence
Visual/report staging: /Users/IDC2.5/Documents/Kaydence
Remote state as of 2026-07-08: origin points to
https://github.com/IslandDevCrew/kaydence.git, but authenticated fetch returns
"Repository not found." Treat the local repo as authoritative until the remote
is restored, recreated, or repointed by the operator.

Mission: build Kaydence into a real local-first cross-platform AI dictation app:
one hotkey, local capture, write-ahead audio, local ASR, Raw/Light/Full cleanup,
native injection on macOS/Windows/Linux, first-run setup, history, Whisper-Ahead,
Voiceprint, Relay, and Conductor in the v3 order.

Read first, in order:
1. AGENTS.md
2. ops/mission/state.json
3. docs/PRD.md
4. docs/ARCHITECTURE.md
5. docs/ROADMAP.md
6. docs/design/README.md
7. prompts/BUILD-LOOP.md and prompts/JUDGE-AUDITOR.md

Current verified state:
- local main is ahead of origin/main because the remote is unavailable.
- P0 is effectively complete except hardware-only window checks.
- P1 is active.
- Fresh local gates passed on 2026-07-08: cargo fmt, cargo clippy, cargo test
  (28 tests including crash recovery), scripts/check-frontend.sh,
  scripts/audit-network.sh, scripts/check-adr-status.sh.
- Frontend cockpit/setup shell is present and must remain presentation-only.
- The Codex app Run action is wired to ./script/build_and_run.sh.
- The hotkey runtime now honors persisted push-to-talk vs toggle mode. The
  cockpit mode control calls set_hotkey_mode, Rust persists hotkey_mode to
  app-data settings.json, and the active coordinator switches only while idle.
  The hotkey binding rebind path is wired locally: set_hotkey_binding accepts
  backend-allowlisted bindings (RightAlt, F13, F14, Control+Space, Shift+F13),
  persists hotkey_primary_binding, and re-registers the active global shortcut
  while idle. Live OS permission/conflict proof across macOS, Windows, and
  Linux remains follow-up work.
- Recent local history is visible in the cockpit via recent_history; per-session
  delete now removes the DB row plus safe app-data session audio after confirm.
  recent_history also surfaces untracked recoverable app-data WAVs as Capture
  failures so crash-orphan audio is visible. export_history_session now writes
  JSON + text exports under app-data exports/. purge_history clears rows,
  app-data session audio, and app-data exports; recent_history applies the
  Rust settings retention window before listing. play_history_audio validates a
  safe app-data WAV and exposes it through Tauri's scoped asset protocol for the
  cockpit audio control. Recovered-audio re-transcription remains follow-up
  history/recovery work until real ASR adapters land.

Operating rules:
- Work from evidence, not assertions.
- One task = one branch/commit-sized unit where possible.
- Critical decision paths require ADR + operator go: new dependencies, platform
  OS-input code, network surfaces, model registry changes, event contract changes,
  Relay crypto, and non-negotiable reinterpretations.
- Keep the locked Visual System v1: nav order, 8px cards, macOS aqua / Windows
  cobalt / Linux emerald accent lanes, prediction blue, merge gold, no cloud-sync
  copy for core features, and explicit confirm gates for destructive Conductor
  actions.
- React is presentation-only. Product logic, timing, text transformation,
  canonical state, and platform behavior live in Rust.
- Save proof under ops/mission/evidence/ and update state/journal/SOTU after
  significant merges.

Next best work:
1. Close the remaining P1-P0-1 hotkey proof: live OS permission/conflict
   validation on macOS, Windows, and Linux plus P1-G2/P1-G3 gates. The
   global-shortcut plugin, WAL runtime path, persisted push-to-talk/toggle
   mode, and allowlisted rebind UI/registration path are already wired locally.
2. Advance P1-P0-2 ASR readiness: replace TODO model hashes/sources with
   reviewed artifacts, implement the network fetch/progress UI, and
   Parakeet/Whisper engine adapters. AppSnapshot already
   exposes ready/missing/blocked model states plus backend-owned ASR candidates,
   selected/recommended model ids, and validated download metadata; the
   select_asr_model command persists the selected ASR model id to app-data
   settings.json and reapplies it on startup; ModelDownloadPlan validates safe
   destination paths, expected sha256 values, and HTTPS sources before first-run
   can show Download required; checksum mismatches quarantine the suspect
   artifact before readiness blocks load; refresh_model_readiness lets first-run
   recheck app-data model artifacts without restarting after a user/build agent
   installs reviewed files; install_model_artifact now opens a native file picker
   and lets Rust copy a selected reviewed local artifact into app-data models/
   only after the file matches the registry sha256; the default hotkey runtime
   carries a PendingLocalAsrEngine keyed to the selected model/lane, so recordings
   persist audio then record a specific Recognize failure until real
   Parakeet/Whisper adapters are implemented.
3. Continue injection backends behind TextInjector: Windows UIA+SendInput still
   needs the Windows Codex session; Linux AT-SPI/uinput needs human-focus VM
   validation; macOS AX/CGEvent/NSPasteboard has local evidence.
4. Build first-run permission/setup and dictation cockpit against screen
   families 01, 03, 07, and 10.
5. Prepare remote recovery only after preserving local commits: either restore
   IslandDevCrew/kaydence, create Navigata1/kaydence, or fork one from the other
   once one exists.

Never mark complete until each PRD item/gate has current evidence proving it.
```
