# SCAFFOLD — Phase 0 bootstrap instructions (for the first agent)

This repo intentionally ships docs-first. Initialize tooling as follows, and
delete this file once Phase 0's exit criteria pass (its content moves to
CONTRIBUTING.md).

1. **Prereqs:** Rust stable + `cargo install tauri-cli`, Node LTS + pnpm,
   platform deps per current Tauri 2.x docs (Xcode CLT / VS Build Tools +
   WebView2). Verify current versions — do not trust trained knowledge.
2. **App init:** `pnpm create tauri-app` → React + TypeScript template →
   merge into `apps/desktop/` preserving the `src-tauri/src/<module>/`
   directories and every `AGENTS.md`. Wire a root `pnpm-workspace.yaml` and a
   Cargo workspace (`apps/desktop/src-tauri` + `crates/*`).
3. **Dependencies (verify latest):** cpal, ringbuf, whisper-rs or direct
   whisper.cpp FFI, ort (ONNX Runtime) for Parakeet V3, silero-vad port,
   rusqlite, tracing, thiserror, ulid, global-hotkey (or per ADR-0004, the
   pattern Handy uses post their macOS shortcut rewrite).
4. **Study pass (ADR-0004):** read Handy's `src-tauri` — capture, model
   loading, VAD wiring, tray, shortcuts. Record adopt/redesign notes in the PR.
5. **CI:** `.github/workflows/ci.yml` matrix `[macos-latest, windows-latest]`:
   fmt, clippy -D warnings, test, frontend typecheck + lint. Add the bench
   gate as soon as `scripts/bench.sh` exists (Phase 1).
6. **Module stubs:** create `mod.rs` per backend module exposing the typed
   events from `docs/ARCHITECTURE.md` §4 with `todo!()` bodies, so the event
   contract compiles before any feature work begins.

Definition of done for this file's job: `cargo tauri dev` opens a window on
macOS, Windows, and Linux; CI is green on all three; ADRs 0001–0004 marked Accepted. (Win/Linux legs waived-pending-infra on a macOS-only host — see ops/mission/state.json.)
