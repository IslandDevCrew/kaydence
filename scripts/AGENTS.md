# scripts/ — Tooling

Inventory (create as phases need them; keep each runnable on macOS, Windows AND
Linux — portable bash 3.2 [macOS default: no `declare -A`, no `mapfile`] or paired
.sh/.ps1):

- `bench.sh` — **present (P1-P0-6, partial working gate)**. Drives the Rust
  `--bench` contract, emits the latency table, compares every measured value
  against root-AGENTS §5, and fails CI on regression. When no packaged binary
  exists it runs `cargo run -- --bench`, so CI no longer skips the gate. The
  current contract measures the implemented Raw orchestration path and marks
  real ASR/GPU/reference-machine/idle-footprint values as explicit unmeasured
  fields until those adapters and machines land.
- `audit-network.sh` — **present (P0-T6), a working gate**. Fixed-string scan of
  the Rust/TS source for network-capable call sites, diffed against
  `network-allowlist.json` (model mirrors + user-configured BYOK endpoints + Relay
  peers only). Zero-source tree = clean pass. Any unreviewed surface = exit 1, a
  release blocker (non-negotiable #1). Deeper binary/deps scan is a TODO for
  post-build. Verified: FAILs on a planted `std::net` call outside the allowlist.
- `check-privacy-posture.sh` — **present (P1-P0-7)**. Runs the network audit,
  asserts the plain-language README privacy promises are still present, and scans
  source for banned screen-capture implementation primitives. This is the
  release privacy posture gate until deeper binary/dependency audits land.
- `check-adr-status.sh` — **present (P0-T6)**. P0-G7 gate: all ADRs Accepted
  except declared by-design exceptions (ADR-0009 Relay stays Proposed until the
  operator crypto review gates P4-1). Comment-safe status parse.
- `gen-types` — Rust `SessionEvent`/Settings → TypeScript types.
- `corpus/` tools — record/annotate golden transcripts (raw audio + expected
  Light output pairs).
- `release.sh` — version bump, changelog from conventional commits, build,
  sign (notarize on macOS), checksum.
- `windows-webview-proof.ps1` — Windows-only native visual evidence harness for
  manual GitHub Actions dispatch. It launches the release Tauri executable,
  navigates the real WebView through Windows UI Automation, captures exact
  900x600 client images plus native-window provenance frames, exports the UIA
  tree, and fails closed on non-interactive, blank, uniform, or wrong-sized
  output. Hosted WebView startup gets three bounded attempts with forced renderer
  accessibility; every miss saves unchecked client/window frames and a UIA tree
  before restarting. This proves rendering/navigation only; it never marks
  microphone, hotkey, injection, secure-field, ASR, or first-dictation readiness.

Scripts are products too: `--help` text, non-zero exit on failure, no silent
fallthrough.

- `bench-prediction.sh` — measures prediction-model candidates (Qwen 2.5 1.5B /
  Gemma 3 1B / Qwen 3 0.6B) on the current machine: TTFT, tok/s, p50/p95 latency,
  RAM, acceptance@5 over a real-continuation corpus. Run on both reference
  machines to lock per-platform defaults (ADR-0007). Skeleton present; the
  acceptance scorer wires to the production prediction prompt once `prediction/`
  exists.
