# Gate-Merge Fusion — take the human out of the merge loop

**Status:** Activated with explicit operator approval, 2026-09-07. Strict required
three-OS CI and auto-merge are enabled. Use merge commits, not squash, per the
operator's activation instruction. Evidence: `ops/mission/evidence/2026-09-07-gate-merge-activation.txt`.
The historical rationale and activation commands follow; critical human gates remain.

## The problem this solves

The Buzz P1-G3 pass produced ~20 chat messages of one agent (Ruew) re-verifying
another (Echo): parent-chain checks, changed-file checks, provenance wording
corrections — for evidence that CI could have judged mechanically. That manual
relay **is** the merge bottleneck archipelago exists to remove. The fleet's
audit instinct is right; the *plane* was wrong. Move the verdict to a check.

## What Kaydence already has (verified 2026-08-08)

`.github/workflows/ci.yml` runs, on the 3-OS matrix, on every push and PR:

- `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` (3 OS)
- shipped-ASR feature build + clippy + test
- `scripts/check-frontend.sh` (tsc + eslint)
- `scripts/audit-network.sh` (non-negotiable #1, zero-egress)
- `scripts/check-privacy-posture.sh --check`
- `scripts/check-adr-status.sh` (ADRs accepted or declared-Proposed)

That is already most of an archipelago gate wall. Latest run at `e6e646a`:
**green on setup + ubuntu + macOS + windows**.

## The three operator settings that finish it

These are **operator actions** (they change merge authority) — an agent must not
do them silently. Each is one command; run from the repo:

1. **Make CI required on `main`** (branch protection):
   ```bash
   gh api -X PUT repos/IslandDevCrew/kaydence/branches/main/protection \
     -H "Accept: application/vnd.github+json" \
     -f 'required_status_checks[strict]=true' \
     -f 'required_status_checks[contexts][]=build-test (ubuntu-latest)' \
     -f 'required_status_checks[contexts][]=build-test (macos-latest)' \
     -f 'required_status_checks[contexts][]=build-test (windows-latest)' \
     -F 'enforce_admins=false' \
     -F 'required_pull_request_reviews=null' \
     -F 'restrictions=null'
   ```
2. **Enable auto-merge** on the repo (Settings → General → Pull Requests →
   "Allow auto-merge"), or via API:
   ```bash
   gh api -X PATCH repos/IslandDevCrew/kaydence -F allow_auto_merge=true
   ```
3. **Every phase PR opts into auto-merge** — the agent that opens it runs:
   ```bash
   gh pr merge <N> --auto --merge --match-head-commit <reviewed-head-sha>
   ```
   GitHub lands it the instant the required 3-OS gate passes. No human click,
   and no agent merged on a claim — the merge is a *consequence of evidence*.

After this: an agent's job ends at "PR open + auto-merge armed + green." The
human reviews only when they *choose* to, never as the gate.

## The archipelago evidence layer (add on top, incrementally)

The CI gates above prove the code is sound. Archipelago adds a **phase-evidence**
gate that proves the *definition of done* — the thing the fleet argues about:

- Vendor the protocol (`archipelago/scripts`, `archipelago/schemas`) into the
  repo, or reference the skill's `protocol/` by absolute path.
- Per phase, build an `evidence-bundle` (schema v2) whose claims carry
  `status: verified|falsified|unverified` and hashed lane artifacts.
- Add a required check that runs `dogfood_lanes.py bundle.json` — it **passes
  only at band 5**. A `falsified` claim → band 1; an `unverified` or a UI claim
  with no `runtime-journey` → capped ≤4; any open finding → capped ≤3.

A ready workflow skeleton is at `docs/protocol/archipelago-gates.yml.template`
(kept out of `.github/workflows/` so it doesn't run until the bundles exist).
When P1's evidence bundle is real, move it in and add its check to the required
list in step 1.

## Honest boundaries (state, never imply)

- Required-CI + auto-merge removes the *manual* merge click — it does **not**
  close **human gates** (P1-G4 visual signoff, ADR-0016 ship auth, ADR-0017
  default-download). Those stay operator-only by design (interop §4).
- A green gate proves the check passed on that run, on those runners — no more.
- If a gate is flaky, fix the evidence or the check; never make it non-required
  to get a merge. That is the one move archipelago forbids.
