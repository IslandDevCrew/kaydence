# Judge & Auditor — the two lenses + critical decision paths

Every work unit passes through two distinct reviews before it can merge. They are
deliberately different jobs; do not collapse them.

## The Judge — quality & charter ("is this right?")
Asks whether the change is correct, well-designed, and on-charter. Checklist:
- Does it honor all eight **non-negotiables**? (privacy line, never-lose-a-word,
  cleanup dial, latency budgets, Windows-first, local-always-works, lightweight,
  no secure-field injection)
- Does it honor the **module's invariants** (its `AGENTS.md`) and the event
  contract (no cross-module internals)?
- Is it the **smallest correct** change? Any dead code, speculative generality,
  or scope creep beyond the PRD item?
- Style: `clippy -D warnings` clean, no `unwrap()` outside tests, frontend
  presentation-only, conventional commit.
- **Reader test:** could a fresh agent, given only this diff + the relevant
  `AGENTS.md`, explain what changed and why?

Output: PASS/FAIL + specific findings. A soft "looks fine" is a FAIL.

## The Auditor — evidence ("prove it")
Accepts nothing on assertion. For every claim the Judge passed, demands the
artifact. Required evidence by change type:

| Change touches… | Required artifact |
|---|---|
| Any code | `cargo test` green on **macOS AND Windows** (CI matrix) |
| Hot path (audio→inject, prediction) | `bench` / `bench-prediction` table within §5 budgets (≤10% regression) |
| Network surface | `audit-network.sh` pass, or ADR + user toggle + allowlist diff |
| Capture / ASR | `crash_recovery` + `short_utterance` suites green |
| Pipeline / events | `event_sequences` suite green |
| Prediction merge | matcher tests + a `prediction_events` row asserted |
| context/ | tests asserting **no disk write, no transmission, no verbatim log** |
| A model add/upgrade | before/after bench table (WER or acceptance + latency) |

Output: an **evidence ledger** (claim → artifact → result). Any missing artifact
= FAIL, regardless of how good the code looks.

## Critical decision paths (ADR + human gate required)
A path is **critical** — and must not merge without an ADR *and* an operator gate
— if it does any of:
1. Touches or reinterprets a **non-negotiable** (esp. privacy/network).
2. Adds a **new dependency** (crate or JS package).
3. Opens a **new network surface** (any egress, including a new BYOK endpoint).
4. Adds or changes **platform-specific** code (`inject/`, `hotkeys/`, device
   layer, app detection).
5. Changes the **model registry** (new/updated model, new default).
6. Changes a **cleanup or prediction prompt contract** (must update its golden
   test set in the same change).
7. Alters the **event contract** (`SessionEvent`).

Everything else is **routine**: the loop proceeds through Judge+Audit without a
human gate, committing on green.

## How the loop uses these
Step 4 = Judge. Step 5 = Auditor. Step 6 gates: routine → commit + continue;
critical → HALT for the operator with the ADR linked and the evidence ledger
attached. No exceptions — passing a gate by weakening a non-negotiable is itself a
blocking defect.
