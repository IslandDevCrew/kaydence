# ADR-0015: Quantized P1 ASR default with evidence-gated platform promotion

- **Status:** Proposed — operator approval required before merge
- **Date:** 2026-07-11
- **PRD items affected:** P0-2 local ASR, P0-6 latency budgets, P1-G3 footprint gate
- **Refines:** ADR-0014 without modifying its accepted record

## Context
ADR-0014 made the real `whisper.cpp` adapter and verified local-artifact seam
available but did not choose a production first-run artifact. The initial
reference run used the unquantized 147,964,211-byte base.en model. Its true Metal
lane passed latency and idle CPU but failed the constitutional `<=250 MB`
resident-RAM gate at 268.188 MB.

Allocator-pressure and flash-attention experiments did not remove the overage.
The pinned 59,721,011-byte Q5_1 base.en artifact did: on the Apple M5 Pro it
passed the four-phrase synthetic corpus and both canonical reference lanes,
Metal at 152.656 MB RSS and 78 ms p95 and CPU at 138.516 MB and 214 ms p95.
The Debian 12 ARM64 CPU lane also passed at 71.859 MB RSS and 600 ms p95, with
an exact `Hello there.` synthetic golden transcript. Windows 11 ARM64 passed the
same exact golden transcript at 828 ms and four consecutive quiescent ten-sample
reference runs at 1121, 965, 903, and 902 ms p95, about 73-75 MB RSS, and 0%
idle CPU. The first recorded run, executed after the native release build,
missed the CPU gate at 1433 ms p95. That temporal ordering is retained, but the
proof did not measure host load or establish why the run missed.

## Decision
Register `whisper-base-en-q5_1` with its exact SHA-256 and source metadata. Make
it the macOS, Windows, and Linux first-run default because all three platforms
now have direct quality, lane, latency, RAM, and idle-CPU evidence. The Windows
promotion is based on four consecutive quiescent reference-benchmark passes.
This is not packaged/installed-app evidence and does not assign a cause to the
first failed run; that run remains evidence, not a passing run.

When at least one reviewed candidate exists, automatic recommendation fallback
ignores candidates whose checksum or download sources are still placeholders.
This prevents the unfinished Parakeet entry from displacing reviewed q5. If
every candidate is still unreviewed, Kaydence retains an explicit blocked
selection so Setup names the metadata failure instead of pretending no model
exists.

Normalize pre-warmup candidate and runtime lane labels to the backend compiled
into the current build. A GPU-preferred `whisper.cpp` artifact reports CPU before
warmup whenever the Metal feature is absent, matching the adapter plan instead
of the registry request.

This ADR does not authorize downloading. Registry metadata and read-only
preflight remain allowed, while any fetch command, progress UI, retry policy,
or runtime network access still requires the separate ADR-0014 network decision
and operator gate.

## Alternatives considered
- **Relax or reinterpret the 250 MB gate:** rejected; the footprint ceiling is a
  constitutional product differentiator and the q5 artifact passes it directly.
- **Keep f16 and tune allocator/attention flags:** rejected by measured negative
  controls; neither reduced resident RAM below the gate.
- **Promote q5 on all three OSes from Mac evidence:** rejected; model changes
  require per-platform WER/latency evidence, and status must not outrun proof.
- **Make large-v3-turbo the default now:** rejected until its runtime, quality,
  and resident-memory evidence passes the same budgets.
- **Make Parakeet the universal default now:** rejected until the independent
  ONNX/RNNT runtime is implemented and benchmarked.

## Consequences
- macOS, Windows, and Linux get a measured, checksum-pinned first-run model that
  meets current P1 quality, latency, RAM, and idle-CPU evidence criteria.
- Windows retains the first p95 miss alongside four consecutive quiescent
  passes. Packaged-app measurement and host-load attribution remain separate
  future evidence requirements.
- Default and Metal-feature tests must cover effective candidate, pending, and
  verified-artifact lane labels.
- The current fallback URL is a second revision path in the same Hugging Face
  repository, not an independent host mirror. SHA-256 protects integrity; an
  independently operated mirror remains an availability follow-up.
- P1-G3 remains pending for physical focused-field timing, the no-model
  process-group boundary, and fresh three-OS CI.
