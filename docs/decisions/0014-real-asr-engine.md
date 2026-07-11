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
   stay dependency-free and fast; `asr-whisper` opts the native CPU compile in.
   macOS Metal is a separate `asr-whisper-metal` feature that composes
   `asr-whisper` with `whisper-rs/metal`. A requested GPU lane is relabeled and
   routed to CPU when no acceleration feature is compiled. This keeps the
   standing gates green while the heavy C++ dep is proven incrementally and
   prevents a requested lane from masquerading as backend evidence.
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
7. **Warm before first dictation.** `AsrEngine` exposes an explicit lifecycle
   warmup. Startup and idle-only adapter replacement warm the first viable lane;
   a failed accelerated initialization falls through to CPU, while a successful
   primary warm leaves the fallback unloaded. Runtime readiness becomes true only
   after this warmup succeeds. The golden proof records one-time warmup separately
   and enforces the warm inference budget for the lane that actually ran.
8. **Production launcher includes the adapter.** Default developer builds remain
   feature-free, but `script/build_and_run.sh` compiles the canonical macOS app
   with `custom-protocol,asr-whisper-metal` and Windows/Linux with
   `custom-protocol,asr-whisper`. The Windows hosted WebView proof uses the same
   CPU feature. A shell contract test prevents the launcher from regressing to a
   UI-only release binary. Canonical release and CI builds also default
   `GGML_NATIVE=OFF`: the binary must not inherit whichever instruction set backs
   the build host. A reviewed architecture-specific release may override that
   default and must record the exact target plus runtime proof. The launcher
   stamps the relevant native build settings and invalidates only the matching
   Cargo profile's Whisper artifacts when they change, because upstream does not
   declare generic GGML/CMake environment flags as Cargo rerun inputs.
9. **Use a measured quantized P1 default.** `whisper-base-en-q5_1` is the
   cross-platform first-run default through the existing hardware-recommendation
   mechanism. Its pinned 59,721,011-byte artifact and SHA-256 are recorded in the
   registry. On the Apple M5 Pro reference host it passed the four-phrase
   synthetic corpus and both real reference lanes: Metal at 152.656 MB RSS and
   78 ms p95, CPU at 138.516 MB and 214 ms p95. Parakeet V3 remains the required
   independent runtime, and large-v3-turbo remains a selectable quality target;
   neither replaces this footprint-safe default until it has equivalent runtime,
   quality, and resident-memory evidence.

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
  required when `asr-whisper` is on; each acceleration feature needs native
  backend-log proof and matrix coverage once CI billing is restored. Metal is now
  explicit and proven on the Apple Silicon reference host; Linux ARM64 CPU is
  runtime-proven, while Windows runtime and Windows/Linux acceleration remain
  owed. The features stay off by default for development, while the canonical
  production launcher selects the supported adapter per OS.
- Now owed (tracked, not done here): the Parakeet/ort CPU adapter; the gated
  model-download network surface (its own ADR-accept); real WER/latency evidence on
  the reference machines for P1-G2/P1-G3.
- The verified q5 registry metadata makes manual installation and read-only
  download preflight concrete, but does not add a fetch command or authorize
  runtime network access. Windows/Linux live q5 reports and human-voice WER
  remain required.
