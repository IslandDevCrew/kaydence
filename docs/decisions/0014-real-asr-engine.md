# ADR-0014: Real ASR engine — whisper.cpp adapter first, feature-gated; download stays a separate gated surface

- **Status:** Accepted (engine adapter + trait wiring) — 2026-07-10, operator go given
- **Date:** 2026-07-10
- **PRD items affected:** P1-P0-2 (local ASR), P1-G2 (golden-clip finals); refines ADR-0002 (AsrEngine trait), builds on ADR-0004 (Handy study)

## Context
Through P1 the `engine/` module shipped the full ASR *contract* — the
`AsrEngine` trait, the ordered fallback `EngineStack` (BYOK → local GPU → local
CPU), no-speech rejection, and a `LocalAsrAdapterState::VerifiedArtifact` state —
but **no adapter transcribes**: a verified model artifact still returns
`Unavailable("the runtime adapter is not implemented yet")`. Nothing the user
says becomes text. That makes "Raw mode daily-drivable" (P1 exit) and P1-G2
(golden-clip finals) unreachable. Turning ASR real means adding a native
inference dependency and, eventually, a way to fetch model weights — the latter
is a **new network surface**, which the constitution treats as a critical
decision path.

## Decision
Implement a **real whisper.cpp adapter** (`WhisperCppEngine`) via the MIT-licensed
`whisper-rs` crate, behind an **off-by-default cargo feature `asr-whisper`**, and
wire it into the existing `LocalAsrAdapterState::VerifiedArtifact` seam so a
checksum-verified local ggml model actually transcribes the 16 kHz mono f32 the
WAL already produces. Specifically:

1. **Engine choice — whisper.cpp first.** `whisper-rs` compiles whisper.cpp from
   source through cargo (no build-time weight download); MIT; Metal on macOS via
   the crate's feature, CPU everywhere. Parakeet/ort (CPU lane) is the documented
   *second* adapter (ADR-0002's two-engine hedge) and lands next — deferred here
   only to keep this unit ≤400 lines and the dep surface minimal.
2. **Feature-gated, default-off.** The default build, CI, and every other module
   stay dependency-free and fast; `asr-whisper` opts the native compile in. This
   keeps the standing gates green while the heavy C++ dep is proven incrementally.
3. **Model input is local-only in this ADR.** The adapter loads weights from the
   already-verified artifact path (`LocalAsrAdapterSpec.artifact_path`, sha256
   checked upstream). **No network fetch is added here.**
4. **The download surface remains a SEPARATE, still-gated sub-decision.** On-demand
   HTTPS model download (registry `sources`, checksum-verify, progress UI,
   audit-network allowlist entry) is a new network surface and **stays Proposed —
   it does not ship until its own operator go.** Until then, models are supplied
   locally (operator-provided path / manual install, as `install_model_artifact`
   already supports).
5. **No Gemma dependency.** ASR uses whisper/parakeet; Gemma 3 1B is the *cleanup/
   prediction* LLM (ADR-0007/0010) and its license review is unaffected by this ADR.
6. **Honesty on P1-G2.** The golden-clip transcription test is real but requires a
   locally-provided model (env `KAYDENCE_WHISPER_MODEL`); it is `#[ignore]`d when
   absent. P1-G2 is **not** claimed passed until that test runs green on a real
   model with a measured WER/latency artifact.

## Alternatives considered
- **Parakeet/ort first:** ort downloads ONNX Runtime binaries at build (build-time
  network) and the Parakeet streaming path is more involved — heavier first step
  than whisper.cpp's cargo-source compile. Deferred, not rejected.
- **Bundle a tiny model to make the test unconditional:** violates "no weights in
  git" (ARCHITECTURE §7). Rejected.
- **Ship model download now to make G2 runnable tonight:** crosses the new-network-
  surface HALT without its own review. Rejected — kept as the separate gated sub-decision.
- **Keep faking it (pending adapter):** leaves the product unable to transcribe;
  fails the P1 exit. Rejected.

## Consequences
- Easier: a verified local model now yields real `RawFinal` text through the
  existing event contract; P1-G2 becomes runnable the moment a model is present.
- Harder / to maintain: `whisper-rs` pulls a C++ (whisper.cpp) build — cmake/clang
  required when `asr-whisper` is on; per-OS acceleration features (Metal/CUDA) need
  matrix coverage once CI billing is restored. The feature stays off by default to
  contain that cost.
- Now owed (tracked, not done here): the Parakeet/ort CPU adapter; the gated
  model-download network surface (its own ADR-accept); real WER/latency evidence on
  the reference machines for P1-G2/P1-G3.
