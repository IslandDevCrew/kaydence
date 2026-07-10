# Kaydence — Codex Handoff (2026-07-10)

Companion to `docs/generated/Kaydence-SOTU-Plan-v4-Codex-Handoff-2026-07-10.html`
(the visual SOTU + Plan of Attack). This file is the paste-ready text for a fresh
Codex (GPT-5.6 Sol Ultra) session. Current truth is always
`ops/mission/state.json` + `ops/mission/state-of-the-union.html`.

## Snapshot (verified against git + CI)
- Repo `/Users/IDC2.5/Kaydence/kaydence` · remote `IslandDevCrew/kaydence` (private) · HEAD `4327651` · 138 commits.
- **P0 closed.** P1 in progress: **passed** P1-G1 (crash recovery), P1-G2 (real whisper.cpp ASR, 0-WER golden corpus, GPU+CPU, ADR-0014), P1-G5 (event sequences). **Pending** P1-G3 (latency/footprint real numbers), P1-G4 (screen fidelity, boards 01/03/07/10).
- Gates 10/23. Design locked (Visual System v1, ADR-0012).
- **CI billing fixed**: $5 Actions budget on IslandDevCrew, card on file; CI green on Linux. Diet: Linux-only on push, full 3-OS on PRs into main + manual dispatch.

## Read first, in order
1. `AGENTS.md` 2. `apps/desktop/AGENTS.md` 3. `apps/desktop/src-tauri/AGENTS.md`
4. `apps/desktop/src/AGENTS.md` 5. `docs/PRD.md` 6. `docs/ARCHITECTURE.md`
7. `docs/ROADMAP.md` 8. `docs/design/README.md` 9. `prompts/BUILD-LOOP.md`
10. `prompts/JUDGE-AUDITOR.md` 11. `docs/decisions/0014-real-asr-engine.md`

## Standing rules
- No authority without evidence: every "done" saves an artifact under `ops/mission/evidence/`.
- One work unit = one branch `mission/<phaseId>-<slug>` = one merge; ≤400 changed lines.
- Never launder unverified into verified. Green Judge + red Audit does not merge.
- Critical decision paths (new network surface, crypto, model registry, prompt contracts, changing a shared UI invariant) HALT for operator go + an ADR.
- Heartbeat after every merge: update `state.json`, run `node ops/mission/render-sotu.mjs`, append `ops/mission/journal.md`, commit.

## Account gotcha (read before pushing)
- `IslandDevCrew` owns the repo + holds billing. `Navigata1` is Jon's other personal account; `git`/`gh` default to it, which causes "Repository not found" on push.
- **Before any push / gh API call:** `gh auth switch --user IslandDevCrew`.
- IslandDevCrew's browser billing lives only in the Chrome profile "Stripe MBPro"; it's a **user** account so use `github.com/settings/billing` (not `/organizations/...`). Operator enters payment; the agent only sets non-credential config.

## Next unit — P1-G4 board-01 Dictation Cockpit
Board: `assets/brand/screens/kaydence-screen-family-01.png` (do not edit).
Built: `apps/desktop/src/App.tsx` (`activeView === "Dictate"`).
Bring it to board composition: waveform record header + circular record/stop + elapsed;
left status cards (Engine, Target App, Global Hotkey, Privacy "Local Only", WAL "Healthy",
all from real `AppSnapshot`); right cleanup dial + output checklist + Output destination +
Auto-paste; 5-icon bottom nav; status footer. OS accent is the only per-OS variation;
8px cards; prediction-blue; Option 1 mark top-left. Split the 1,914-line `App.tsx` into
per-view components + shared primitives (`StatusCard`, `SegmentedDial`, `NavRail`,
`StatusFooter`), files ≤800 lines.

Verify before merge: `npm run check` green; **run the app and screenshot the Dictate view
beside board-01** (a UI claim with no runtime screenshot is not done); `cargo fmt --check`,
`clippy -D`, `cargo test --all` green; save evidence; one branch, one merge, heartbeat.

## Standing gate command
```bash
gh auth switch --user IslandDevCrew
M=apps/desktop/src-tauri/Cargo.toml
cargo fmt --manifest-path $M --all -- --check
cargo clippy --manifest-path $M --all-targets -- -D warnings
cargo test --manifest-path $M --all
bash scripts/check-frontend.sh
bash scripts/audit-network.sh
bash scripts/check-adr-status.sh
bash scripts/check-privacy-posture.sh --check
bash scripts/bench.sh --check
```
Real ASR proof (needs local model, no run-time network):
```bash
KAYDENCE_WHISPER_MODEL=/Users/IDC2.5/Documents/Kaydence/models/ggml-base.en.bin \
KAYDENCE_WHISPER_CLIP=<16kHz mono wav> KAYDENCE_WHISPER_EXPECT=<substr> \
cargo test --manifest-path $M --features asr-whisper --test asr_golden -- --nocapture
# add KAYDENCE_WHISPER_LANE=cpu to force the CPU fallback lane
```

## Arch-build pointer (optional)
Jon's ARCHIPELAGO methodology (evidence-gated build; "nothing crosses a gate on claims
alone") lives at `/Users/IDC2.5/.claude/skills/arch-build/SKILL.md` (source of truth:
`Navigata1/archipelago`). It's the same discipline this repo's loop already enforces. Hand
it to GPT only if it needs a workflow framework or starts crossing gates on claims. Minimal
version for GPT: before marking anything done, produce evidence a skeptic could check (a
test run, a screenshot, command output) and save it — else mark it `unverified`, never
`verified`.
