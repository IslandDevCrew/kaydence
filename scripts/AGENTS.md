# scripts/ — Tooling

Planned inventory (create as phases need them; keep each runnable on macOS
AND Windows — use cross-platform runners or paired .sh/.ps1):

- `bench.sh` — drives the golden audio corpus through the pipeline, emits the
  latency/WER table; CI compares against `bench-baseline.json` and fails on
  >10% regression of any root-AGENTS §5 budget.
- `audit-network.sh` — builds the app, greps the dependency graph and binary
  for network-capable calls, diffs against the allowlist
  (`network-allowlist.json`: model mirrors + user-configured BYOK endpoints
  only). A new network surface failing this gate is a release blocker
  (non-negotiable #1).
- `gen-types` — Rust `SessionEvent`/Settings → TypeScript types.
- `corpus/` tools — record/annotate golden transcripts (raw audio + expected
  Light output pairs).
- `release.sh` — version bump, changelog from conventional commits, build,
  sign (notarize on macOS), checksum.

Scripts are products too: `--help` text, non-zero exit on failure, no silent
fallthrough.

- `bench-prediction.sh` — measures prediction-model candidates (Qwen 2.5 1.5B /
  Gemma 3 1B / Qwen 3 0.6B) on the current machine: TTFT, tok/s, p50/p95 latency,
  RAM, acceptance@5 over a real-continuation corpus. Run on both reference
  machines to lock per-platform defaults (ADR-0007). Skeleton present; the
  acceptance scorer wires to the production prediction prompt once `prediction/`
  exists.
