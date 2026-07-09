# Kaydence Codex Super Goal Prompt

Use this prompt when opening a fresh Codex session to continue Kaydence from the
real repository state. It is intentionally explicit so a new session can build
instead of re-discovering the same blockers.

```md
/goal Continue the Kaydence end-to-end build from the real local repo until the
scoped v3 product is complete and verified. Work from current files, current git
state, and current external truth. Do not redefine completion around a smaller
subset.

Repo: /Users/IDC2.5/Kaydence/kaydence
Remote: https://github.com/IslandDevCrew/kaydence.git
Primary SOTU: ops/mission/state-of-the-union.html
Mission state: ops/mission/state.json

Read first, in order:
1. AGENTS.md
2. apps/desktop/AGENTS.md
3. apps/desktop/src-tauri/AGENTS.md
4. apps/desktop/src/AGENTS.md
5. docs/PRD.md
6. docs/ARCHITECTURE.md
7. docs/ROADMAP.md
8. docs/design/README.md
9. prompts/BUILD-LOOP.md
10. prompts/JUDGE-AUDITOR.md

Repository truth as of 2026-07-09T17:52Z:
- The private GitHub repo resolves as IslandDevCrew/kaydence with ADMIN
  permission for the active GitHub auth.
- origin/main is f61ab49 docs(ops): record actions billing block.
- Local main is intentionally ahead of origin/main with unpublished follow-up
  commits including:
  - c64a2b7 feat(p1): open first-run permission settings
  - b7bab46 docs(ops): refresh current continuation handoff
  - the current P1-P0-1 secondary cleanup override hotkey slice
  - 36f538d feat(p1): align first-run action labels
  - f4e0853 feat(p1): align first-run permission checklist
  - bb2542f feat(p1): require permission rows for first-run readiness
  - e12b4fa feat(p1): refresh runtime permission proofs
  - 0930d09 feat(p1): expose asr runtime status
  - 8bef075 feat(p1): wire artifact-aware asr runtime state
  - 6d64400 feat(p1): surface permission proof boundaries
- Do not push those commits until the operator accepts the remote CI boundary.
- Latest verified green remote baseline is Actions run 29031546518 at ca60244,
  completed 2026-07-09T16:09:29Z on macOS, Ubuntu, and Windows.
- Latest checked runs 29033116849 and 29033270673 failed before checkout because
  GitHub reported account payment or spending-limit requirements. No repo tests
  ran in those failures. Fix billing/spending limit and rerun before claiming
  remote parity for f61ab49 or any local follow-up commits.

Mission:
Build Kaydence into the v3 local-first cross-platform AI dictation app:
one hotkey; local capture; write-ahead audio; local ASR; Raw/Light/Full cleanup;
native injection on macOS, Windows, and Linux; truthful 60-second first run;
local history; Whisper-Ahead; Voiceprint; Relay; and Conductor in roadmap order.

Operating rules:
- Work from evidence, not intent.
- Keep React presentation-only. Product logic, canonical state, timing, platform
  behavior, and text transformation live in Rust.
- Every meaningful slice updates evidence under ops/mission/evidence/, the
  mission journal, ops/mission/state.json, and the generated SOTU.
- Critical decision paths require ADR plus operator go before merge/push:
  new dependencies, network surfaces, model registry changes, platform OS-input
  code, event-contract changes, Relay crypto, or non-negotiable reinterpretation.
- Keep Visual System v1 locked: left nav order, 8px cards, compact utility UI,
  macOS aqua / Windows cobalt / Linux emerald accent lanes, prediction blue,
  merge gold, no cloud-sync copy for core features, and explicit destructive
  confirm gates.
- Do not claim 3-OS parity from local macOS tests. Current Windows/Linux parity
  requires a fresh green CI run once hosted runners are unblocked, plus real
  reference-machine proof for the hands-on paths.

Current product state:
- P0 is done. Tauri scaffold, ADR/doc reconciliation, standing gates, Linux CI
  deps, 3-OS CI, and windowed cargo tauri dev on macOS/Linux/Windows are closed
  with evidence.
- P1 is active.
- P1-P0-3 Windows injection is merged and live-validated on the Fable box:
  UIA ValuePattern native insert, SendInput Unicode fallback, clipboard
  snapshot/restore, secure-field refusal, and frontmost app detection.
- P1-P0-3 remaining work is Linux human-focus validation, Linux
  unicode-beyond-ASCII, optional portal/libei, and continued macOS/Windows
  regression proof as changes land.
- P1-P0-4 WAL/history core is done: audio persists before ASR, crash recovery
  and short-utterance gates exist, SQLite history lists sessions, safe delete,
  export, purge, orphan recovery, and audio playback are wired.
- P1-P0-7 privacy posture is done: no telemetry/accounts/cloud screen capture/
  default egress, audit-network plus privacy posture checks, and Screen Family
  07 cockpit panel.
- P1-P0-6 has a partial enforced latency floor: kaydence --bench emits JSON and
  scripts/bench.sh --check enforces implemented Raw-path budgets. Real ASR/GPU,
  idle footprint, and reference-machine p95 evidence remain.
- P1-P0-1 hotkey core is wired: push-to-talk/toggle, selected binding
  persistence, global shortcut registration, idle-only switching/rebinding,
  runtime proof refresh for hotkey and microphone evidence, and short-utterance
  tests. The secondary cleanup override chord can now register Shift+RightAlt/
  Option as a Raw-only per-invocation capture path while the next primary
  capture returns to the profile cleanup dial. Live OS permission/conflict proof
  and ASR golden clips remain.
- P1-P0-2 first-run model readiness is honest but not finished: registry
  verification, ASR picker, selected/recommended model ids, local artifact
  install, mismatch quarantine, read-only download preflight, and
  artifact-aware selected ASR runtime status are wired. TODO hashes/sources,
  approved download/progress UI, real Parakeet/Whisper adapters, WER, and
  latency evidence remain.
- P1-P0-8 first run is actively moving: backend-owned next_step, durable
  first-dictation completion only after real Injected, setup timing, permission
  rows for macOS/Windows/Linux, stricter ready_to_dictate contract, proof export
  JSON, runtime proof refresh, permission settings opener, UI action-label
  alignment, and Screen Family 10 proof surfaces are wired. Remaining work:
  live OS permission detectors/proof, approved model download/progress UI, live
  first-dictation journey, and <=60s reference-machine proof.

Best next build sequence:
1. Keep closing P1-P0-8 as the main spine because it exercises model readiness,
   permissions, hotkey, first dictation, timing, and build-agent proof.
2. For permissions, add real platform proof carefully:
   - macOS: microphone/accessibility/input monitoring preflight and prompt path
     if it can be done without false readiness.
   - Windows: microphone privacy proof plus UIA/SendInput proof reruns on the
     Windows handoff machine.
   - Linux: AT-SPI/uinput proof on the Debian GNOME VM, operator-in-the-loop
     where focus cannot be moved safely by automation.
3. For models, replace registry TODO hashes/sources only with reviewed artifacts
   and ADR/operator approval for any network downloader/progress UI.
4. Build real ASR adapters after model provenance is solved. Until then, keep
   runtime states Pending/Blocked/VerifiedArtifact honest and never fake text.
5. Continue injection proof, especially Linux human-focus and unicode
   beyond ASCII.
6. Keep SOTU current and pause before any push while hosted CI is billing-blocked.

Local gate set for most P1 slices:
- cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --all -- --check
- cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
- cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --lib
- pnpm --dir apps/desktop run check
- pnpm --dir apps/desktop run build
- ./script/build_and_run.sh --verify
- scripts/check-privacy-posture.sh
- scripts/check-adr-status.sh
- scripts/bench.sh --check
- Browser smoke for affected UI surfaces
- Fresh 3-OS CI after push, once GitHub hosted runners execute again

Windows continuation:
Use docs/WINDOWS_CODEX_HANDOFF_2026-07-09.md. It supersedes the July 8 handoff.

Never mark complete until every explicit PRD item, gate, invariant, and
deliverable has current evidence proving completion.
```
