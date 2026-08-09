# Kaydence — Fleet Consolidation Handoff (2026-08-08)

Written by Claude Code after the Buzz fleet + Codex pass. Purpose: a single
grounded, evidence-backed resume point so any agent (or a fresh session) can pick
up P1 closeout without re-deriving state. Nothing here is relayed on trust —
every fact below was independently checked at `e6e646a` unless marked otherwise.

## 1. Verified ground truth

| Item | Verified fact | How checked |
|---|---|---|
| Remote | `github.com/IslandDevCrew/kaydence` — **public**, default `main` | `gh api` |
| Local | `/Users/IDC2.5/Kaydence/kaydence` — clean, `main == origin/main` | `git status`/`rev-parse` |
| HEAD | `e6e646a` "heartbeat after first-run download UI" | `git log` |
| CI | run **30926591146 success** at exactly `e6e646a` (setup + ubuntu + macOS + windows) | `gh run view` |
| Gates (my re-run) | **P1-G1** 2/2 · **P1-G2** 5/5 · **P1-G5** 5/5 · audit-network/privacy/adr **PASS** · bench `--check` exit 0 (synthetic only) | `cargo test` + scripts |
| Landed since 8b47521 | PR #29 (v6 state), #30 (unique ADR identities → ADR renumber), #31 (first-run download UI + wired reviewed download) | `git log` |
| ADR renumber | old "ADR-0015 download" is now **ADR-0017**; ADR-0015 = quantized-ASR-default | `ls docs/decisions` |

**Two stale/known items (do not re-litigate, do fix through the right branch):**
- `ops/mission/state.json → mission.repoPath` still reads the Windows path
  `C:/Users/IslandDevCrew/Desktop/Kaydence`. Correct value: this Mac's checkout.
  **Already fixed in Fizz's frozen reconciliation branch — let that branch land
  it; do not duplicate the fix on a second branch** (that causes the exact
  overlap the fleet is avoiding).
- Frozen fleet branches (`fea6c54` Fizz reconciliation, `abe906c` Echo P1-G3
  evidence) live in Buzz worktrees, not this checkout's refs. Keep them frozen.

## 2. Assessment (my thoughts)

The fleet did **valuable diagnostic work with genuinely good audit discipline**,
and **nothing is merge-ready** — both true, and both fine. Specifics:

- **What was excellent:** Ruew's independent verification (rejecting Echo's
  provenance claims until the evidence actually supported them, accepting only
  as *blocked/partial*) is exactly the "no claim without evidence" rule. Keep it.
- **The anti-pattern to fix:** that verification happened in **chat** (≈20
  messages of manual re-checking) for evidence **CI could judge mechanically**.
  Kaydence's CI already runs the gate wall. The fix is Gate-Merge Fusion, not
  more careful chat — see `docs/protocol/GATE_MERGE_FUSION.md`.
- **The one real product finding (not just measurement noise):** the packaged
  `Kaydence.app` at `e6e646a` **repeatedly dies ~60–117 s after launch (SIGTERM)**
  — under `open -n` (LaunchServices) and even direct-exec, not only under the
  concurrent-`pkill` contamination. That blocks the ≥600 s P1-G3 idle measurement
  **and** is a shipping-relevant lifecycle question (an app that self-terminates
  after ~2 min idle is not done). This is the highest-value thing to run down.
  Contrast: the 2026-07-11 no-model packaged build survived >10 min — so it is a
  regression or an env-specific SIGTERM, not "tray is impossible."
- **No Kaydence product code changed** in the fleet pass. Good — it was a
  read-only audit + one measurement branch.

Codex's read is correct and I agree with it, including: keep both branches
frozen, single-owner physical measurement, split the 458-line P1-G3 branch to
≤400, make raw scratch evidence durable with hashes, and continue P1 closeout in
gate order. This handoff operationalizes his interop proposal.

## 3. The improvement this pass delivers (new, on this PR)

- `docs/protocol/AGENT_INTEROP_PROTOCOL.md` — Buzz↔Codex↔Claude: three planes
  (git=truth, chat=dispatch, CI=merge authority), the **work-packet** and
  **handoff-manifest** shapes, hard authority boundaries, and the **Codex-Bridge**
  (read/reply, no authority; key in OS keychain, never in chat/file/env).
- `docs/protocol/GATE_MERGE_FUSION.md` — the three operator settings that make CI
  the merge authority (branch protection required + `allow_auto_merge` +
  `gh pr merge --auto`), so agents stop being the merge bottleneck.
- `docs/protocol/archipelago-gates.yml.template` — the band-5 phase-evidence gate,
  inert until a real evidence bundle exists.
- `AGENTS.md §7.1` — archipelago trigger phrases + interop rules, so "arch build"
  and this protocol are callable in Codex/Buzz too (they read AGENTS.md).

## 4. Ordered P1-closeout plan (archipelago work packets)

Each is one branch = one PR ≤400 lines, evidence under `ops/mission/evidence/`,
handoff-manifest on completion. Agent-executable unless marked HUMAN.

1. **`mission/p1-g3-lifetime-diagnosis`** (agent, this Mac, single-owner) —
   diagnose the packaged-app SIGTERM ~60–117 s. One `OWNER_LOCK`, no concurrent
   `pkill`, Bash-3.2 harness. Output: SIGTERM sender attribution + whether it is a
   tray/automatic-termination product defect or env. *This gates the real P1-G3
   idle measurement.*
2. **Fix (if defect) → `mission/p1-g3-lifecycle-fix`** (agent, separate bounded
   branch, ADR if it touches tray/lifecycle contract) — then re-run the ≥600 s
   packaged-q5 resident-footprint measurement with the verifier fields.
3. **P1-G3 physical release→field latency** (agent macOS AX now; Win/Linux need
   the reference hosts) + visible no-model <80 MB remeasure at `e6e646a`.
4. **P1-P0-8** — real first-dictation journey + **≤60 s** reference-machine proof.
5. **HUMAN — P1-G4 visual signoff** on the five Windows captures (Bumble's
   independent verdict first; operator closes the gate).
6. **HUMAN — ADR-0016 ship authorization** (see §5) and **ADR-0017** default-
   download go/no-go.
7. **P1 exit** → open P2.

## 5. Operator decisions pending (only you can close these)

- **Reference device set** (for P1-G3 / P1-P0-8): this M-series Mac; the GMKtec
  NucBox M2 Pro Windows 11 host; the Debian 12 GNOME/Wayland ARM64 VM (treat the
  VM as *functional Wayland proof*, not broad Linux perf certification).
- **ADR-0016 direction** (Kael's packet, grounded): official pinned ONNX Runtime
  1.28 assets per-OS with full sha256; **Apache-2.0 wav2vec2** CTC (not CC-BY
  Parakeet); registry-pinned HTTPS sources; official Silero v5.1.2 (already
  verified in registry); whisper q5 stays every platform's default; ONNX/Silero
  ship non-default until a per-platform ≤250 MB bench table exists. **Direction
  approvable now; ship authorization gated on the real per-OS sha256 + vendored
  assets.**
- **ADR-0017 default model-download:** keep **off** by default until the ADR-0016
  pins are real and the ≤60 s first-dictation proof exists.

## 6. Recommended reply to the Buzz fleet (paste as-is or edit)

> Approved: treat the M-series Mac, the GMKtec Windows 11 host, and the Debian 12
> Wayland VM as the P1 reference set — the VM as functional Wayland proof, not
> broad Linux performance certification. Approve the ADR-0016 *direction* (pinned
> ORT 1.28 per-OS assets with full sha256, Apache-2.0 wav2vec2 CTC, pinned HTTPS
> sources, official Silero v5.1.2, whisper q5 stays default, ONNX/Silero non-
> default until the ≤250 MB bench table) — but **no ship authorization** until the
> per-OS sha256 + vendored assets are real. Keep ADR-0017 default model-download
> **off**. Keep the repo public through P1 closeout. Keep Fizz's reconciliation
> and Echo's P1-G3 branches frozen. Fizz: assign
> `mission/p1-g3-lifetime-diagnosis` first (single-owner, no concurrent pkill,
> Bash-3.2) — the packaged-app SIGTERM at ~60–117 s is the real blocker and gates
> the idle measurement. Adopt `docs/protocol/*` (interop + gate-merge fusion) so
> future verification lands via CI, not a manual audit relay.

## 7. Fresh-session resume prompt (copy into a new session)

> Resume Kaydence P1 closeout. Repo `github.com/IslandDevCrew/kaydence` (public),
> local `/Users/IDC2.5/Kaydence/kaydence`, `main` at (re-verify) `e6e646a`, CI
> green. Read `docs/FLEET_CONSOLIDATION_HANDOFF_2026-08-08.md` and
> `docs/protocol/AGENT_INTEROP_PROTOCOL.md` first. Do NOT touch main/state.json or
> frozen fleet branches. First work packet: `mission/p1-g3-lifetime-diagnosis` —
> single-owner, diagnose the packaged-app ~60–117 s SIGTERM before any ≥600 s
> measurement. Human gates (P1-G4, ADR-0016 ship, ADR-0017) stay operator-only.
