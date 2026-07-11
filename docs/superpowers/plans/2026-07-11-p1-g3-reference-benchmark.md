# P1-G3 Reference Benchmark Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a repeatable, fail-closed reference benchmark that measures warm real-ASR pipeline p50/p95 plus ASR-resident RAM and idle CPU on macOS, Windows, and Linux without claiming that a no-op delivery sink proves physical OS-field injection.

**Architecture:** A feature-gated Rust binary owns the production-path measurement: it loads a checksum-reviewed local Whisper artifact, warms the selected lane once, replays a real 16 kHz PCM clip through WAL, VAD, ASR, Raw commit selection, and injection policy, then idles while resident state remains loaded. A dependency-free Node runner launches that binary, samples the child process with the host's native process tool, merges latency and footprint into schema-versioned JSON, and enforces the relevant constitutional budgets. Physical field injection remains a separately named evidence requirement.

**Tech Stack:** Rust stable, existing `whisper-rs` feature flags, existing Kaydence pipeline traits, Node.js standard library, PowerShell only as the Windows process-query backend.

## Global Constraints

- Do not add a Rust or JavaScript dependency.
- Do not add a network call; model and clip paths are operator-supplied local files.
- React remains untouched and presentation-only.
- The Rust probe must compile to an explicit failure message without `asr-whisper`.
- Warmup is excluded from release-to-policy timing and reported separately.
- Report p50 and p95 over at least five measured samples; default to ten.
- Resident RAM budget is `<=250 MB`; idle CPU budget is `<=1%`.
- GPU p95 budget is `<=700 ms`; CPU p95 budget is `<=1200 ms`.
- The report must call the measured endpoint `delivery_policy`, not physical injection.
- The report must list `physical_os_field_injection` as unmeasured until a focused native target proves it.
- One platform report proves only that platform and lane.
- No model bytes, audio bytes, transcript text, or user paths are written to evidence JSON.

---

### Task 1: Real-ASR Probe Contract

**Files:**
- Create: `apps/desktop/src-tauri/src/bin/reference-bench.rs`
- Test: `apps/desktop/src-tauri/src/bin/reference-bench.rs`

**Interfaces:**
- Consumes: `KAYDENCE_WHISPER_MODEL`, `KAYDENCE_WHISPER_CLIP`, `KAYDENCE_WHISPER_LANE`, `KAYDENCE_REFERENCE_SAMPLES`, `KAYDENCE_REFERENCE_IDLE_MS`, and `KAYDENCE_REFERENCE_READY_FILE`.
- Produces: one JSON document on stdout with `schema`, `platform`, `lane`, `warmup_ms`, `sample_count`, `release_to_delivery_policy_p50_ms`, `release_to_delivery_policy_p95_ms`, `audio_ms`, `transcript_nonempty`, `events`, and `unmeasured`.

- [ ] **Step 1: Write failing unit tests for percentile selection and argument validation**

```rust
#[test]
fn percentile_uses_nearest_rank() {
    let samples = vec![10, 20, 30, 40, 50];
    assert_eq!(nearest_rank(&samples, 50), 30);
    assert_eq!(nearest_rank(&samples, 95), 50);
}

#[test]
fn sample_count_rejects_values_below_five() {
    assert!(parse_sample_count(Some("4")).is_err());
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --bin reference-bench
```

Expected: FAIL because the binary and helper functions do not exist.

- [ ] **Step 3: Implement the feature-gated probe**

The implementation must:

```rust
let warmed_lane = stack.warm_up()?;
for _ in 0..sample_count {
    let started = Instant::now();
    let events = pipeline.process_capture(&summary)?;
    let committed = committed_text(&events).ok_or(Error::MissingCommit)?;
    let delivered = inject_committed_text(
        &mut PolicySink,
        committed.id,
        &committed.text,
        UnknownFieldPolicy::Strict,
        false,
    );
    require_native_delivery(delivered)?;
    samples_ms.push(elapsed_ms(started.elapsed()));
}
```

Write the ready file atomically after measurements and before the idle hold. It contains only `pid`, `phase`, and `idle_ms`. Keep the engine stack and pipeline alive for the entire hold.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run:

```bash
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --bin reference-bench
```

Expected: PASS with no model required for helper tests.

- [ ] **Step 5: Verify the real Metal lane locally**

Run:

```bash
KAYDENCE_WHISPER_MODEL=/Users/IDC2.5/Documents/Kaydence/models/ggml-base.en.bin \
KAYDENCE_WHISPER_CLIP=/Users/IDC2.5/Documents/Kaydence/models/clip16k.wav \
KAYDENCE_WHISPER_LANE=gpu KAYDENCE_REFERENCE_SAMPLES=10 \
cargo run --release --manifest-path apps/desktop/src-tauri/Cargo.toml \
  --features asr-whisper-metal --bin reference-bench
```

Expected: valid JSON, `lane=local_gpu`, ten samples, non-empty transcript, p95 `<=700` ms, and `physical_os_field_injection` listed as unmeasured.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src-tauri/src/bin/reference-bench.rs
git commit -m "feat(p1): add real ASR reference benchmark probe"
```

### Task 2: Cross-Platform Process Sampler

**Files:**
- Create: `scripts/bench-reference.mjs`
- Create: `tests/reference-bench-contract.mjs`
- Modify: `scripts/AGENTS.md`

**Interfaces:**
- Consumes: `KAYDENCE_REFERENCE_BIN` or a locally built `reference-bench` binary plus the probe environment from Task 1.
- Produces: `bench-results/reference-<platform>-<lane>.json` and a non-zero exit when any measured budget fails.

- [ ] **Step 1: Write a failing source-contract test**

The test must assert that the runner contains all three host samplers, checks the ready-file PID against the spawned process, enforces p95/RAM/CPU, and preserves the physical-injection unmeasured marker.

```javascript
assert.match(source, /process\.platform === "win32"/);
assert.match(source, /process\.platform === "darwin"/);
assert.match(source, /\/proc\/\$\{pid\}\/stat/);
assert.match(source, /physical_os_field_injection/);
assert.match(source, /idle_cpu_pct/);
```

- [ ] **Step 2: Run the contract test and verify RED**

Run:

```bash
node tests/reference-bench-contract.mjs
```

Expected: FAIL because `scripts/bench-reference.mjs` does not exist.

- [ ] **Step 3: Implement the runner**

Use only `node:child_process`, `node:fs`, `node:os`, and `node:path`. Sample:

- macOS: `ps -o rss=,time= -p <pid>` twice across the idle interval.
- Linux: `/proc/<pid>/stat` plus `/proc/<pid>/status`, using `getconf CLK_TCK`.
- Windows: `Get-Process -Id <pid> | Select-Object Id,CPU,WorkingSet64 | ConvertTo-Json -Compress` twice.

Compute:

```javascript
idle_cpu_pct = ((cpuSecondsAfter - cpuSecondsBefore) / idleSeconds) * 100;
idle_ram_mb = Math.max(rssBeforeBytes, rssAfterBytes) / (1024 * 1024);
```

Reject a ready-file PID mismatch, early child exit, malformed probe JSON, fewer than five samples, a non-reviewed lane, or a missing unmeasured marker. Redact source paths from output.

- [ ] **Step 4: Run tests and the live Mac runner**

Run:

```bash
node tests/reference-bench-contract.mjs
node scripts/bench-reference.mjs --check
```

Expected: contract PASS; live report PASS for latency, RAM, and idle CPU or an honest non-zero budget failure with captured measurements.

- [ ] **Step 5: Commit**

```bash
git add scripts/bench-reference.mjs tests/reference-bench-contract.mjs scripts/AGENTS.md
git commit -m "build(p1): measure reference ASR footprint cross-platform"
```

### Task 3: Evidence, Review, and Mission Heartbeat

**Files:**
- Create: `ops/mission/evidence/2026-07-11-p1-g3-macos-reference-bench.txt`
- Modify: `ops/mission/state.json`
- Modify: `ops/mission/journal.md`
- Modify: `ops/mission/state-of-the-union.html` (generated)

**Interfaces:**
- Consumes: the live JSON report and local standing-gate output.
- Produces: a skeptic-checkable Mac evidence artifact and truthful remaining-gate list.

- [ ] **Step 1: Run the standing local gates**

```bash
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --lib
bash scripts/check-frontend.sh
bash scripts/audit-network.sh
bash scripts/check-privacy-posture.sh --check
bash scripts/check-adr-status.sh
git diff --check
```

Expected: every command exits zero.

- [ ] **Step 2: Save evidence with hashes and explicit limits**

The evidence must include commit SHA, host manifest, feature/lane, sample count, warmup, p50/p95, RAM, idle CPU, report SHA-256, commands, and raw exit codes. It must say that Windows and Linux reference reports and physical OS-field injection remain pending.

- [ ] **Step 3: Run Judge and Auditor review**

Judge checks code quality, dependency/network invariants, and platform sampling math. Auditor checks that every performance claim is directly backed by the generated JSON and that the evidence does not close P1-G3 broadly.

- [ ] **Step 4: Update mission truth and render SOTU**

```bash
jq empty ops/mission/state.json
node ops/mission/render-sotu.mjs
git diff --check
```

Expected: valid JSON, regenerated HTML, no whitespace errors, P1-G3 still pending until Windows plus physical injection evidence exists.

- [ ] **Step 5: Commit and publish as a stacked draft PR**

```bash
git add ops/mission/evidence/2026-07-11-p1-g3-macos-reference-bench.txt \
  ops/mission/state.json ops/mission/journal.md ops/mission/state-of-the-union.html
git commit -m "docs(ops): record Mac P1-G3 reference evidence"
git push -u origin HEAD
```

Expected: draft PR targets the preceding Windows ARM64 heartbeat branch; hosted CI remains mandatory before any merge.

## Self-Review

- Spec coverage: real local ASR, repeatable p50/p95, ASR-resident RAM, idle CPU, three host samplers, budget enforcement, evidence, and non-overclaiming are all assigned.
- Deliberate non-closure: physical OS-field injection and Windows/Linux reference runs remain named requirements; this unit cannot close P1-G3 by itself.
- Dependency/privacy check: no dependency and no network surface are introduced.
- Type consistency: the Rust probe fields consumed by the Node runner use the same names throughout this plan.
