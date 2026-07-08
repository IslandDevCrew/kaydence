# Kaydence Build Loop — autonomous, evidence-gated execution

This is the protocol the `/kickoff` command runs. It turns the docs in this repo
into a self-driving build that advances **only on evidence**, halting at defined
human gates. It encodes Jon's North Star Command Protocol and the
"no authority without evidence" rule.

## The unit of work
One **work unit** = one PRD item (e.g. `P0-3`) or one explicit ROADMAP sub-task.
Work units are selected in ROADMAP phase order, respecting dependencies. Never
start a unit you can't trace to a PRD ID or an approved issue.

## The loop (run per work unit)

```
        ┌──────────────────────────────────────────────────────────┐
        ▼                                                          │
 1 ORIENT ─► 2 PLAN ─► 3 BUILD ─► 4 JUDGE ─► 5 AUDIT ─► 6 GATE ────┘ (next unit)
                          ▲           │           │         │
                          └──── fail ─┘──── fail ─┘    human gate? ─► HALT + recap
```

### 1 · ORIENT
- Read root `AGENTS.md`, the `AGENTS.md` of every directory you'll touch, and any
  ADR they reference. Every time — familiarity is not a reason to skip.
- State the work unit, its PRD ID, and the non-negotiables it touches.
- If the requirement can't be traced or is ambiguous → **HALT, `clarify`**.

### 2 · PLAN
- Write the plan: files to change, the event(s) involved, tests to add, budgets
  affected, and which non-negotiables/invariants are in play.
- Classify the path: **routine** or **critical** (see `JUDGE-AUDITOR.md`).
  Critical paths require an ADR and a human gate *before* merge.
- For routine paths, proceed without asking. For critical paths, write/confirm
  the ADR and request the gate.

### 3 · BUILD
- Implement the smallest correct change: code + tests together.
- Obey style (root §9), module invariants, and the event contract. No `unwrap()`
  outside tests. Frontend stays presentation-only.

### 4 · JUDGE  (quality lens — "is this right and on-charter?")
- Self-critique against: the eight non-negotiables, the module's invariants, the
  Definition of Done, and code quality. Produce explicit findings.
- **Fail → return to BUILD** (max 3 attempts; then HALT + `recap` with the
  blocker). Do not advance on a soft pass.

### 5 · AUDIT  (evidence lens — "prove it")
- Accept nothing on assertion. Attach the artifact for each claim:
  - tests pass **on macOS and Windows** (CI matrix output)
  - latency budgets measured if the hot path changed (`bench` / `bench-prediction`)
  - no new network calls (`audit-network.sh`) — or ADR + toggle + allowlist diff
  - relevant gate suites green (crash-recovery, short-utterance, event-sequences)
- Produce an **evidence ledger** (claim → artifact). Missing evidence = fail →
  return to BUILD or gather the evidence. A green Judge with a red Audit does not
  merge.

### 6 · GATE
- **Both pass, routine path** → conventional commit (`feat(prediction): …`) →
  select the next work unit → loop.
- **Critical path, or phase boundary, or repeated failure** → **HALT** and
  summarize for the operator using the command vocabulary below.

## Human gates (where the loop always stops)
- Any **critical decision path** merge (`JUDGE-AUDITOR.md`).
- A **phase boundary** — present the phase's exit criteria with evidence, await `go`.
- **Ambiguity** — requirement can't be traced (`clarify`).
- **3× Judge/Audit failure** on one unit — escalate, don't thrash.

## Command vocabulary (operator ⇄ loop)
- `proceed` / `go` — execute the agreed plan / advance past this gate.
- `recap` — print current phase, last commit, evidence ledger, and next 3 units.
- `clarify` — list the open questions blocking progress.
- `rewind` — revert to the last committed checkpoint; discard work since.

## Invariants of the loop itself
1. Never claim "done" without an artifact (no authority without evidence).
2. Never cross a critical path without an ADR + human gate.
3. Never let prediction/context work weaken a non-negotiable to pass a gate.
4. Keep `AGENTS.md` and ADRs current in the same commit that changes behavior.
5. One work unit at a time; small commits; green CI before the next unit.
