# Kaydence Codex Super Goal Prompt

Use this prompt when opening a fresh Codex session to continue Kaydence without
losing the thread.

```md
/goal Continue the Kaydence end-to-end build from the real local repo.

Repo: /Users/IDC2.5/Kaydence/kaydence
Visual/report staging: /Users/IDC2.5/Documents/Kaydence
Remote state as of 2026-07-09: origin points to
https://github.com/IslandDevCrew/kaydence.git. Private repo access is restored
after switching the active GitHub CLI account to IslandDevCrew and running
gh auth setup-git; gh repo view resolves IslandDevCrew/kaydence as PRIVATE with
ADMIN permission and git fetch succeeds.

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
- GitHub remote access is restored; `IslandDevCrew/kaydence` resolves as a
  PRIVATE repo with ADMIN permission and `git fetch origin` succeeds.
- Current verified remote baseline before the first-run proof-export slice:
  main at `5bd35e0` had green 3-OS Actions CI in run 29029907649, completed
  2026-07-09T15:44:45Z on macOS, Ubuntu, and Windows.
- P0 is complete: 7/7 tasks and 7/7 gates, including windowed `cargo tauri dev`
  observed on macOS, Linux, and Windows.
- P1 is active; P1-P0-7 privacy posture is done with evidence, P1-G1 is locally
  passed with a repaired real `crash_recovery` filter, and P1-G2 has a runnable
  capture/WAL short-utterance suite. P1-G2 remains pending for ASR golden clips.
- Fresh local gates passed on 2026-07-09 for the first-run proof-export
  slice: cargo fmt, cargo clippy, cargo test (200 lib tests + 2
  crash-recovery tests + 5 event-sequence tests + 5 short-utterance tests),
  pnpm --dir apps/desktop run check, scripts/check-privacy-posture.sh,
  scripts/check-adr-status.sh, scripts/bench.sh --check, production frontend
  build, desktop verify/build, and Browser UI smoke at desktop plus narrow
  window widths.
- The `event_sequences` integration gate now exists and passed locally plus 3-OS
  CI through `d531a4c` / run 29016531062. It covers
  the typed cross-module order for happy path, focus-change hold, secure-field
  hold, BYOK/GPU/local-CPU fallback, and the current Full cleanup rule-floor
  fallback before injection.
- The latency bench gate has a working partial floor: `kaydence --bench` emits
  JSON before Tauri startup, and `scripts/bench.sh --check` enforces measured
  Raw-path budgets in CI instead of skipping. 3-OS CI run 29022084029 enforced
  the bench step successfully; local follow-up PASS measured
  hotkey_to_capture=7ms, partial_lag=120ms, release_to_inject_cpu=6ms, and
  light_cleanup_added=1ms. P1-G3 remains pending for real
  ASR/GPU/idle-footprint and reference-machine p95 evidence.
- First-run completion truth is now backend-owned: first_dictation_completed is
  persisted in app-data settings.json, hydrated into AppSnapshot on startup, and
  set only after the hotkey runtime records a real SessionEvent::Injected
  outcome. Held delivery paths, including focus-change before delivery, do not
  mark setup complete. FirstRunStatus also carries backend-owned
  permission_requirements for macOS, Windows, and Linux, and Screen Family 10
  renders those runtime OS requirements with state/detail/action copy.
  Permission rows also carry backend-owned action_label metadata, and the
  first_run_permission_action Tauri command returns manual_step plus
  proof_requirement for the setup UI. FirstRunSetupTiming now records
  first_run_started_at_ms, first_dictation_completed_at_ms, elapsed_ms,
  target_ms=60000, and within_target in Rust-owned AppSnapshot/settings truth;
  the setup UI renders a 60-second setup proof strip. FirstRunStatus also
  carries a backend-owned next_step sequence, and export_first_run_proof_plan
  writes app-data exports/first-run-proof-plan.json from Rust-owned AppSnapshot
  truth with next_step, setup_timing, model/ASR state, permission requirements,
  hotkey state, first-dictation state, and proof_items for build-agent handoff.
  Screen Family 10 exposes the Export Proof action and reports the returned
  path/item count. first_run_model_download_preflight now gives Screen Family
  10 a read-only Review path for required models: destination, expected sha256,
  size, sources, license, block reason, operator action, and proof requirement
  come from Rust without fetching bytes or creating files. This is not fake
  detection, fake timing proof, or a network downloader: default permission
  states remain needs_hardware or needs_review until live platform proof exists,
  and the real fetch/progress UI still requires ADR/operator review plus an
  allowlisted, toggled network surface. P1-P0-8 remains in progress for live
  permission prompts/proof, approved network download/progress UI, live
  first-dictation journey, and <=60s reference proof.
- Frontend cockpit/setup shell is present and must remain presentation-only.
- The Codex app Run action is wired to ./script/build_and_run.sh.
- The hotkey runtime now honors persisted push-to-talk vs toggle mode. The
  cockpit mode control calls set_hotkey_mode, Rust persists hotkey_mode to
  app-data settings.json, and the active coordinator switches only while idle.
  The hotkey binding rebind path is wired locally: set_hotkey_binding accepts
  backend-allowlisted bindings (RightAlt, F13, F14, Control+Space, Shift+F13),
  persists hotkey_primary_binding, and re-registers the active global shortcut
  while idle. Live OS permission/conflict proof across macOS, Windows, and
  Linux remains follow-up work. The short_utterance filter now covers 0.3s PTT,
  1.5s toggle, <250ms discard, and short WAL recovery without truncation.
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
- Privacy posture is now checkable: README states no telemetry, no account, no
  cloud screen capture, no default network egress, local-only history, and
  memory-only context; scripts/check-privacy-posture.sh runs audit-network,
  asserts those README promises, and blocks banned screen-capture primitives;
  CI runs it after audit-network. The cockpit includes Screen Family 07 Privacy
  & Context, with desktop/narrow screenshots in ops/mission/evidence/.
- Windows injection is merged in PR #2: UIA ValuePattern native insert,
  SendInput Unicode fallback, clipboard snapshot/restore, and secure-field
  refusal were live-validated on the Fable box. Remaining P1-P0-3 work is Linux
  human-focus validation, Linux unicode-beyond-ASCII, and optional portal/libei.

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
1. Continue P1-P0-8 first run: the final checklist step now has durable Rust
   truth after a real injected dictation, the setup card renders the
   backend-owned OS permission requirements contract, permission rows can ask
   Rust for a manual step/proof boundary, FirstRunSetupTiming drives the
   60-second setup proof strip, FirstRunStatus.next_step sequences the next
   setup action, export_first_run_proof_plan writes the local proof-plan JSON
   under app-data exports/, and first_run_model_download_preflight renders a
   read-only model Review card without fetching bytes. Next are live permission
   prompts/proof, approved network download/progress UI, a live first-dictation
   journey, and <=60s reference proof.
2. Close the remaining P1-P0-1 hotkey proof: live OS permission/conflict
   validation on macOS, Windows, and Linux plus ASR golden-clip proof. The
   global-shortcut plugin, WAL runtime path, persisted push-to-talk/toggle
   mode, allowlisted rebind UI/registration path, repaired crash_recovery
   filter, and capture/WAL short_utterance suite are already wired locally.
3. Advance P1-P0-2 ASR readiness: replace TODO model hashes/sources with
   reviewed artifacts, implement the approved network fetch/progress UI, and
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
4. Continue injection backends behind TextInjector: Windows UIA+SendInput is
   merged and live-validated; Linux AT-SPI/uinput still needs human-focus VM
   validation plus unicode-beyond-ASCII work; macOS AX/CGEvent/NSPasteboard has
   local evidence.
5. Build first-run permission/setup and dictation cockpit against screen
   families 01, 03, and 10; Screen Family 07 has a first source-backed privacy
   panel but still needs the eventual human design fidelity review with the rest
   of P1-G4.
6. Watch GitHub after each push: confirm Actions starts on the pushed HEAD and
   do not claim current-head 3-OS parity until the fresh CI run is green.

Never mark complete until each PRD item/gate has current evidence proving it.
```
