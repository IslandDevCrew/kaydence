# Kaydence Windows Codex Handoff

> Superseded by `docs/WINDOWS_CODEX_HANDOFF_2026-07-09.md`. This file is kept
> as historical evidence from the July 8 recovery pass.

## Purpose

Continue the P1 build on a Windows machine without re-litigating the whole plan.
Windows is a first-class lane, and the macOS host cannot prove UI Automation,
SendInput, Windows microphone privacy, signed MSI behavior, or real window launch.

## Starting Point

- Authoritative local repo on this machine: `/Users/IDC2.5/Kaydence/kaydence`
- Remote currently unavailable: `https://github.com/IslandDevCrew/kaydence.git`
  returns `Repository not found` on authenticated fetch.
- Local `main` has committed work beyond `origin/main`. Do not discard it.
- If transfer is needed, create a bundle from the Mac:

```bash
cd /Users/IDC2.5/Kaydence/kaydence
git bundle create /Users/IDC2.5/Desktop/kaydence-local-main-2026-07-08.bundle --all
```

On Windows:

```powershell
git clone kaydence-local-main-2026-07-08.bundle kaydence
cd kaydence
git status
```

## Windows Setup

Install:
- Git for Windows
- Rust stable
- Node 22 or newer plus Corepack
- pnpm 11.10.0 through Corepack
- Microsoft Visual Studio Build Tools with Desktop development with C++
- WebView2 runtime

Then:

```powershell
corepack enable
pnpm install --frozen-lockfile
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all
bash scripts/check-frontend.sh
bash scripts/audit-network.sh --check
bash scripts/check-adr-status.sh
```

## Windows P1 Work

1. Read `AGENTS.md`, `apps/desktop/AGENTS.md`,
   `apps/desktop/src-tauri/AGENTS.md`, `docs/PRD.md`, `docs/ARCHITECTURE.md`,
   `docs/decisions/0013-linux-injection-strategy.md`, and
   `prompts/P1-CAPTURE-INJECTION-PROMPT.md`.
2. Implement Windows injection behind the existing `TextInjector` trait:
   UI Automation text insertion first, `SendInput` fallback second, clipboard
   restore only when needed.
3. Add secure-field refusal evidence for password/protected UIA fields.
4. Validate the global hotkey path on real Windows, including push-to-talk,
   toggle, min-capture, and tail-buffer behavior.
5. Save evidence under `ops/mission/evidence/` with date-stamped filenames.

## Gates To Close On Windows

- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all`
- Windows-specific injection tests
- Manual focused-editor injection proof
- Secure-field refusal proof
- `cargo tauri dev` opens a real window
- Frontend shell renders with the Windows cobalt lane selected

Do not claim Windows parity from compile-only CI. The lane needs one real machine
proof for window launch and injection behavior.
