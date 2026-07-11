# P1-G3 Quantized ASR Default Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the already-supported quantized Whisper artifact the measured macOS/Windows/Linux P1 first-run default, promoting each platform only after its own live evidence, so Kaydence can meet the constitutional `<=250 MB` ASR-resident budget without relaxing the gate.

**Architecture:** Keep the existing `whisper.cpp` adapter, lane fallback, checksum verification, and operator-supplied artifact boundary. Add one verified registry entry for `whisper-base-en-q5_1` and select it through the existing `default_for` mechanism on all three desktop platforms after direct proof. Automatic fallback excludes placeholder-only candidates when reviewed metadata exists; an all-placeholder registry retains one explicit blocked selection. Point the runner's local default at the q5 artifact filename and normalize every first-run lane label to the compiled backend. The download surface remains preflight-only: this change records pinned HTTPS metadata but adds no fetch command, dependency, or runtime network call.

**Tech Stack:** Existing Rust model registry and first-run logic, existing React preview fixture, Node.js standard-library benchmark runner, existing `whisper-rs` Metal/CPU adapter.

## Global Constraints

- Do not relax `idle_ram_mb <= 250`, `idle_cpu_pct <= 1`, GPU p95 `<=700 ms`, or CPU p95 `<=1200 ms`.
- Do not add a dependency, runtime network call, model byte, audio byte, transcript, or user path to git.
- Use registry ID `whisper-base-en-q5_1`, artifact `ggml-base.en-q5_1.bin`, and SHA-256 `4baf70dd0d7c4247ba2b81fafd9c01005ac77c2f9ef064e00dcf195d0e2fdd2f` consistently.
- Keep Parakeet V3 and Whisper large-v3-turbo visible as separate candidate lanes; neither may be called runtime-ready while its checksum/source/runtime proof remains open.
- Promote q5 through `default_for` one platform at a time only after that platform's live quality, latency, RAM, and idle-CPU evidence passes.
- A GPU request may fall back truthfully to CPU on builds without acceleration; the actual lane owns the latency budget.
- Preserve the ADR-0014 operator gate: registry download metadata and read-only preflight are allowed, but downloading requires a separately accepted network decision.
- Keep P1-G3 pending until physical focused-field injection timing, the no-model process-group boundary, and fresh three-OS CI are complete.

---

### Task 1: Registry And Recommendation Contract

**Files:**
- Modify: `apps/desktop/src-tauri/src/models/mod.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `models/registry.json`

**Interfaces:**
- Consumes: `ModelRegistry::load`, `first_run_asr_recommendation`, and existing `default_for` selection.
- Produces: a checksum-valid `whisper-base-en-q5_1` entry selected on proven `macos`, `windows`, and `linux`.

- [x] **Step 1: Add failing registry assertions**

Assert the ID, `gpu` lane, `whisper.cpp` runtime, exact filename/hash, 57 MiB advertised size, usable sources, and at least three recommended ASR candidates.

- [x] **Step 2: Verify the registry test fails**

Run:

```bash
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml parses_current_registry_and_exposes_first_run_candidates -- --nocapture
```

Expected before implementation: `ModelNotFound("whisper-base-en-q5_1")`.

- [x] **Step 3: Add a failing current-registry recommendation test**

Assert macOS, Windows, and Linux return `whisper-base-en-q5_1` with `Recommended for this OS lane`. Placeholder-only Parakeet must not displace a reviewed model.

- [x] **Step 4: Verify the recommendation test fails**

Run:

```bash
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml current_registry_defaults_every_desktop_to_the_footprint_safe_whisper_model -- --nocapture
```

Expected before the first implementation: macOS selected ID is `parakeet-v3`. The first review-fix red cycle proved all-platform promotion was too broad. The second review-fix red cycle then expected Windows to receive reviewed q5 fallback and observed placeholder-only Parakeet; it also expected the visible required-model lane to be CPU in a default build and observed GPU. After four consecutive quiescent Windows passes, the final promotion red cycle expected `Recommended for this OS lane` and observed the fallback label until `default_for` was widened.

- [x] **Step 5: Add the minimal registry entry**

Record the exact artifact identity, MIT license, three-OS defaults, two HTTPS source URLs, and measured P1 note. Do not add download execution.

- [x] **Step 6: Verify both focused tests pass**

Run the two commands from Steps 2 and 4. Expected: one focused test passes in each run.

### Task 2: Benchmark And Preview Alignment

**Files:**
- Modify: `tests/reference-bench-contract.mjs`
- Modify: `scripts/bench-reference.mjs`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `models/AGENTS.md`
- Modify: `apps/desktop/src-tauri/src/engine/AGENTS.md`
- Modify: `docs/decisions/0014-real-asr-engine.md`

**Interfaces:**
- Consumes: the registry artifact filename and ID from Task 1.
- Produces: a q5 reference-runner default plus matching Browser/native fixture copy and maintained module contracts.

- [x] **Step 1: Add a failing runner source contract**

Require `ggml-base.en-q5_1.bin` and reject the old literal default `"ggml-base.en.bin"`.

- [x] **Step 2: Verify the runner contract fails**

Run `node tests/reference-bench-contract.mjs`.

Expected before implementation: assertion failure for missing `ggml-base.en-q5_1.bin`.

- [x] **Step 3: Change only the runner's default artifact filename**

Keep `KAYDENCE_WHISPER_MODEL` override behavior unchanged.

- [x] **Step 4: Verify the runner contract passes**

Run `node tests/reference-bench-contract.mjs`.

Expected: `reference-bench behavioral contract: PASS`.

- [x] **Step 5: Align the macOS preview fixture and ownership docs**

Use registry ID `whisper-base-en-q5_1`, filename `ggml-base.en-q5_1.bin`, and byte size `59721011` in the preview fixture. Document q5 as the measured P1 default, large-v3-turbo as an unproven quality option, and the unchanged network gate.

### Task 3: Canonical Evidence

**Files:**
- Modify: `ops/mission/evidence/2026-07-11-p1-g3-macos-local-gpu.json`
- Modify: `ops/mission/evidence/2026-07-11-p1-g3-macos-local-cpu.json`
- Create: `ops/mission/evidence/2026-07-11-p1-g3-quantized-asr-default.txt`

**Interfaces:**
- Consumes: optimized `target/release/reference-bench`, reviewed local q5 artifact, and the existing 16 kHz fixture/corpus.
- Produces: sanitized canonical JSON plus a before/after and 4/4 corpus evidence record.

- [x] **Step 1: Run four Metal golden phrases**

For `corpus_0.wav` through `corpus_3.wav`, run `asr_golden` with `--features asr-whisper-metal`, q5 artifact, GPU lane, and the expected phrase. Expected: 4/4 pass with zero content-word error.

- [x] **Step 2: Run canonical GPU and CPU reports**

Run `node scripts/bench-reference.mjs --check` twice with model ID `whisper-base-en-q5_1`, the exact q5 SHA, ten samples, and the reviewed fixture. Expected: both reports pass all three budgets and retain `physical_os_field_injection` as unmeasured.

- [x] **Step 3: Record the before/after table and non-claims**

Compare f16 GPU `268.188 MB / 77 ms p95 / fail` with q5 GPU, and f16 CPU `220.859 MB / 229 ms p95 / pass` with q5 CPU. Record flash-attention and allocator-pressure experiments as reverted negative controls. Do not close P1-G3.

### Task 4: Verification, Review, And Publication

**Files:**
- Modify: `ops/mission/state.json`
- Modify: `ops/mission/journal.md`
- Regenerate: `ops/mission/state-of-the-union.html`

**Interfaces:**
- Consumes: Tasks 1-3 and an independent whole-diff review.
- Produces: one stacked draft PR on PR #23 plus exact mission truth.

- [x] **Step 1: Run focused and broad local gates**

Run Rust default and shipped-feature tests/Clippy, Node benchmark contract, frontend type/lint, network audit, privacy, ADR, synthetic benchmark, JSON validation, renderer, and `git diff --check`.

- [x] **Step 2: Obtain independent review**

Require no unresolved Critical or Important findings. Fix findings test-first and rerun affected gates.

- [x] **Step 2a: Move the model-default decision into ADR-0015**

Restore immutable ADR-0014, create a superseding Proposed decision with an explicit operator gate, limit `default_for` promotion to platforms with direct evidence, and test effective CPU/GPU lane labels across candidate, required-model, preflight, and runtime surfaces in default and Metal-feature builds.

- [x] **Step 3: Update mission truth without closing P1-G3**

Record the Mac q5 GPU/CPU and Linux CPU passes plus all five Windows outcomes, and remove only the proven q5 resident-RAM/p95 blockers. Preserve the first Windows miss without inferring its cause, plus packaged-app measurement, physical injection, full-process-group, hosted CI, and P1-G4 blockers.

- [x] **Step 4: Commit, push, and open a draft stacked PR**

Base the new draft on `codex/p1-g3-reference-bench-final-review-fixes`; do not merge while hosted three-OS CI is billing-blocked.
