---
description: Force an evidence audit of the current working state
---

Act as the Auditor (`prompts/JUDGE-AUDITOR.md`) over the current working tree —
do not change code. For every claim of completeness in the last unit, demand the
artifact and report PASS/FAIL:
- `cargo test` green on macOS AND Windows (CI matrix)
- latency budgets (`bench` / `bench-prediction`) within root §5 if the hot path changed
- `audit-network.sh` clean (no un-allowlisted egress) — non-negotiable #1
- relevant gate suites: crash_recovery, short_utterance, event_sequences
- context/ changes: no disk write, no transmission, no verbatim log
Output an evidence ledger and a single verdict: MERGE-READY or BLOCKED (with the
exact missing artifacts). Accept nothing on assertion.
