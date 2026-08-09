# Agent Interop Protocol — Buzz ↔ Codex ↔ Claude on one repo

**Status:** Proposed (this PR). **Owner decision required** to activate the
operator-side pieces (branch protection, auto-merge, the Codex-Bridge identity).

This is how three different agent systems — the **Buzz** fleet (Fizz, Honey,
Bumble, Kael, Echo, Ruew…), **Codex**, and **Claude Code** — collaborate on
Kaydence without stepping on each other, without a human becoming the merge
bottleneck, and without any agent merging work on claims alone. It operationalizes
the archipelago rule (*nothing crosses a gate on claims alone*) across tools.

---

## 1. Three planes (the whole model)

| Plane | What it is | Who writes | Authority |
|---|---|---|---|
| **Durable** | the git repo — code, evidence under `ops/mission/evidence/`, ADRs, this doc | any agent, on a **bounded branch** | the only source of truth |
| **Dispatch** | Buzz `#Kaydence`, Codex chat, this handoff — assignments + discussion | humans + agents | **no authority**; talk is inert until it lands in git behind a gate |
| **Gate** | CI on every PR (fmt, clippy, 3-OS test, network/privacy/ADR audits) + branch protection | machine | **the merge authority** — a PR lands *because a check that could have failed, didn't* |

The fleet's recent pain — Ruew re-verifying Echo's provenance six times over
wording — happened because verification lived in the **dispatch** plane (chat)
instead of the **gate** plane (CI). The fix is not more careful chat; it is to
move the verdict to a check. See `GATE_MERGE_FUSION.md`.

**Rule:** an agent's chat report is a *pointer*, never a *proof*. The proof is
the committed evidence file + the gate result at a named SHA.

---

## 2. The work packet (dispatch → an agent)

Every assignment carries a structured packet so any agent (Buzz, Codex, Claude)
can execute it identically and independently. Post it in the dispatch plane; it
names everything needed to work purely from the grounded tree.

```json
{
  "packet_id": "P1-G3-lifetime-diagnosis-01",
  "base_sha": "e6e646a44a236f996dd994c7cbf465fb26bff15e",
  "branch": "mission/p1-g3-lifetime-diagnosis",
  "worktree": "optional: /path if isolation needed",
  "objective": "one sentence — the falsifiable thing to achieve",
  "allowed_paths": ["apps/desktop/src-tauri/src/**", "ops/mission/evidence/**"],
  "forbidden": ["main edits", "ops/mission/state.json", "human-gate closure"],
  "gates": ["cargo test ...", "scripts/audit-network.sh"],
  "evidence_required": ["ops/mission/evidence/<id>.txt with HEAD/PIDs/commands"],
  "authority": "read-write on branch; NO push to main; NO PR merge; NO ADR status change",
  "line_budget": 400,
  "expected_handoff": "a handoff manifest (§3) on completion"
}
```

Rules that make packets safe:

- **One packet = one branch = one PR, ≤400 changed lines** (AGENTS §8). Echo's
  P1-G3 branch overran at 458 — split it.
- **Single-owner for physical measurement.** Device measurements (RAM, latency,
  process lifetime) must run under one `OWNER_LOCK`. Concurrent harnesses that
  each `pkill -x kaydence` SIGTERM each other and contaminate results — this
  is exactly what corrupted the first P1-G3 runs.
- **Bash 3.2 only on macOS harnesses** (no `mapfile`) — another real P1-G3 abort.
- **`allowed_paths` is a fence, not a suggestion.** Touch nothing outside it.

## 3. The handoff manifest (an agent → back to dispatch)

Every agent returns this on completion. It is the pointer from chat into the
durable plane; a reviewer (human or the gate) accepts or rejects *from it alone*,
without re-running.

```json
{
  "packet_id": "P1-G3-lifetime-diagnosis-01",
  "head_sha": "abe906c…",
  "parent_chain": ["abe906c → e6e646a"],
  "changed_files": ["ops/mission/evidence/2026-08-08-….txt"],
  "evidence_paths": ["ops/mission/evidence/2026-08-08-….txt"],
  "gate_results": { "cargo test": "pass", "audit-network": "pass" },
  "judge": "self-assessment",
  "audit": "independent verifier verdict + reviewer id",
  "blockers": ["≥600s idle unreachable: packaged process dies ~60-117s (SIGTERM)"],
  "claim_status": "blocked",           // verified | falsified | unverified | blocked
  "pr_url": "null until opened",
  "authority_respected": true          // no main/state/human-gate touched
}
```

`claim_status` maps to the archipelago band caps: `falsified` → the work is
wrong (band 1); `unverified`/`blocked` → not done, do not describe as passed
(capped at 3). **Never launder `blocked` into `verified`** — Ruew was right to
refuse that every time.

## 4. Authority boundaries (hard, for every agent)

No agent — Buzz, Codex, Claude, or a bridge — may automatically:

- push or merge to `main`;
- edit `ops/mission/state.json` or shared evidence outside an assigned packet;
- close a **human gate** (P1-G4 visual signoff; ADR-0016 ship authorization;
  ADR-0017 default-download enablement);
- change an ADR's status;
- flip repo visibility.

These are operator decisions. Agents *prepare* them (evidence, recommended
language) and stop. A PR **may** land automatically — but only as a consequence
of the required gates passing (§ Gate-Merge Fusion), never on an agent's say-so.

## 5. The Codex-Bridge (read/reply access, no authority)

To let Codex and Claude see the Buzz fleet's live dispatch (and vice-versa)
without exported thread copies:

- A dedicated **non-admin** `Codex-Bridge` identity gets **read + reply** on
  `#Kaydence` only.
- Its signing key lives in the **OS keychain or a remote signer** — never in
  chat, a file, a command line, or an environment variable. (Codex correctly
  refused to expose `BUZZ_PRIVATE_KEY`; that instinct is the policy.)
- The bridge **may**: read threads, post work packets, post handoff manifests,
  link SHAs/PRs/evidence.
- The bridge **may not**: approve ADRs, merge PRs, close human gates, or edit
  mission state. Same boundaries as §4.

Until the bridge exists, the durable plane already works cross-tool: every agent
reads the same repo, the same `ops/mission/evidence/`, and the same CI. Exported
threads + branches are a sufficient fallback (Codex confirmed it reads all of it).

## 6. Where each system is strongest (use them for that)

- **Buzz fleet** — parallel, adversarial, independent verification. Its lane
  discipline (Ruew ≠ Echo) is genuinely good; keep it for *review*, not for
  serial re-verification a CI check should do.
- **Codex** — deep single-context grounding + build; excellent at reading the
  whole tree and producing one correct unit.
- **Claude Code** — long multi-step build + orchestration + this protocol layer.
- **The gate (CI)** — the tiebreaker. When two agents disagree, the answer is a
  check at a SHA, not a longer argument.

---

## 7. Adopt in one commit per repo

For any repo Jon builds, drop the AGENTS.md "Archipelago + interop" section (see
root `AGENTS.md`), this file, and `GATE_MERGE_FUSION.md`. Codex and Buzz read
`AGENTS.md`, so "arch build" and this protocol become callable there with no
separate skills system. Kaydence's CI already runs the gate set — the remaining
step is operator-side (branch protection + auto-merge).
