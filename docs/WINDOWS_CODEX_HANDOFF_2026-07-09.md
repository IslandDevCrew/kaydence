# Kaydence Windows Codex Handoff

Date: 2026-07-09T17:42Z

This handoff is for a Windows Codex session continuing Kaydence from the current
P1 build. It supersedes `docs/WINDOWS_CODEX_HANDOFF_2026-07-08.md`.

## Current Truth

- Main repo: `https://github.com/IslandDevCrew/kaydence.git`
- Private repo access is restored; active GitHub auth sees
  `IslandDevCrew/kaydence` with ADMIN permission.
- `origin/main` is `f61ab49 docs(ops): record actions billing block`.
- Local Mac repo `/Users/IDC2.5/Kaydence/kaydence` is ahead of origin/main by
  8 commits and must not be discarded:
  - `c64a2b7 feat(p1): open first-run permission settings`
  - `36f538d feat(p1): align first-run action labels`
  - `f4e0853 feat(p1): align first-run permission checklist`
  - `bb2542f feat(p1): require permission rows for first-run readiness`
  - `e12b4fa feat(p1): refresh runtime permission proofs`
  - `0930d09 feat(p1): expose asr runtime status`
  - `8bef075 feat(p1): wire artifact-aware asr runtime state`
  - `6d64400 feat(p1): surface permission proof boundaries`
- Latest remote green baseline: Actions run `29031546518` at `ca60244`, green
  on macOS, Ubuntu, and Windows.
- Current hosted CI blocker: runs `29033116849` and `29033270673` failed before
  checkout because GitHub reported failed payments or a spending-limit increase
  requirement. Treat that as remote infra blocked, not test failure.

## Windows Status

Already done and proven:
- P0-G5 Windows window launch closed on the Fable box.
- P1-P0-3 Windows injection backend is merged in PR #2 with live evidence:
  UI Automation native insertion, SendInput Unicode fallback, clipboard
  snapshot/restore, frontmost app detection, and secure-field refusal.
- Evidence: `ops/mission/evidence/2026-07-09-windows-injection.txt` and
  `ops/mission/evidence/2026-07-09-windows-tauri-dev-window.png`.

Still needed on Windows:
- Re-run Windows proof after the 8 local Mac follow-up commits land on a branch
  or are transferred to the Windows machine.
- Validate P1-P0-8 first-run permission flow on real Windows:
  microphone privacy panel target `ms-settings:privacy-microphone`, UIA proof,
  SendInput proof, hotkey proof, and first-dictation proof.
- Confirm Screen Family 10 renders with the Windows cobalt lane and the new
  permission proof/settings opener surfaces.
- Eventually provide reference-machine timing for P1-P0-8 and P1-P0-6 on a
  mid-range Windows laptop with no dGPU.

## Getting The Current Work Onto Windows

Preferred path after GitHub billing is fixed:

```powershell
git clone https://github.com/IslandDevCrew/kaydence.git kaydence
cd kaydence
git checkout main
git pull
```

If the 8 local commits are still unpushed, create a bundle on the Mac:

```bash
cd /Users/IDC2.5/Kaydence/kaydence
git bundle create /Users/IDC2.5/Desktop/kaydence-local-main-2026-07-09.bundle --all
```

Then on Windows:

```powershell
git clone kaydence-local-main-2026-07-09.bundle kaydence
cd kaydence
git status -sb
git log --oneline -10
```

## Windows Setup

Install:
- Git for Windows
- Rust stable
- Node 22 or newer plus Corepack
- pnpm 11.10.0 through Corepack
- Microsoft Visual Studio Build Tools with Desktop development with C++
- WebView2 Runtime

Then:

```powershell
corepack enable
pnpm install --frozen-lockfile
pnpm --dir apps/desktop run check
pnpm --dir apps/desktop run build
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --lib
scripts\check-privacy-posture.sh
scripts\check-adr-status.sh
scripts\bench.sh --check
```

If shell-script execution is awkward under PowerShell, run the shell scripts
from Git Bash.

## Windows Proof Tasks

1. Read:
   - `AGENTS.md`
   - `apps/desktop/AGENTS.md`
   - `apps/desktop/src-tauri/AGENTS.md`
   - `apps/desktop/src-tauri/src/inject/AGENTS.md`
   - `docs/PRD.md`
   - `docs/ARCHITECTURE.md`
   - `docs/decisions/0013-linux-injection-strategy.md`
   - `prompts/P1-CAPTURE-INJECTION-PROMPT.md`
2. Launch the app:

```powershell
.\script\build_and_run.sh --verify
```

3. Validate Screen Family 10 on Windows:
   - Windows cobalt lane selected or selectable.
   - Permission rows include microphone, UI Automation focus access, and
     SendInput fallback.
   - Microphone permission action exposes/open attempts the Windows Settings
     privacy target.
   - UIA and SendInput remain proof-command/manual proof surfaces until the
     selftests pass.
4. Re-run Windows injection proof:
   - normal field native insertion via UIA ValuePattern;
   - secure/password field refusal;
   - SendInput fallback into a classic Win32 Edit control;
   - clipboard fallback preserves/restores user clipboard.
5. Validate hotkey and first-run proof:
   - selected binding registers;
   - push-to-talk/toggle behavior follows the runtime contract;
   - short utterance is not truncated;
   - a real Injected event is the only path that marks first dictation complete.

## Evidence To Save

Save date-stamped evidence under `ops/mission/evidence/`, for example:

- `2026-07-09-windows-first-run-permission-proof.txt`
- `2026-07-09-windows-first-run-screen-family-10.png`
- `2026-07-09-windows-hotkey-first-dictation-proof.txt`
- `2026-07-09-windows-injection-regression.txt`

Each evidence file should include:
- machine and OS build;
- exact commit hash;
- commands run;
- observed result;
- whether the result proves readiness or only preserves a manual/review
  boundary;
- any screenshots referenced.

## Do Not Claim

- Do not claim current-head 3-OS parity until a fresh GitHub Actions run
  executes on the relevant pushed commit and passes macOS, Ubuntu, and Windows.
- Do not mark any first-run permission row ready merely because Settings opened.
- Do not fake ASR output. Until Parakeet/Whisper adapters land, keep
  Pending/Blocked/VerifiedArtifact runtime status honest.
- Do not add model-download network behavior without ADR/operator approval,
  allowlist, and explicit user toggle.
