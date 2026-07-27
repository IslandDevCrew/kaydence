# ADR-0016: Second ASR lane behind ONNX Runtime (CTC-first) + real Silero VAD

- **Status:** Proposed — awaits operator go. New dependency (`ort`) + new native binary
  (libonnxruntime) + frontier `engine/` work + model-registry change = a root AGENTS
  §7.5 critical decision path: needs a human gate before merge, not just a green gate
  (the gate is blind to the build-time binary — see Decision 6).
- **Date:** 2026-07-10
- **PRD items affected:** P1-P0-2 (≥2 local ASR engines), P1-G2 (golden finals); refines
  ADR-0002 (AsrEngine trait / ≥2-engine hedge); builds on ADR-0014 (whisper adapter) and
  ADR-0015 (gated model download).

## Context
ADR-0002 mandates ≥2 viable local ASR engines for supply-chain resilience. Today only
the whisper.cpp lane transcribes (ADR-0014, proven live 2026-07-10); the registry's
second "recommended" ASR (`parakeet-v3`) was a fail-closed placeholder with three
factual errors (now corrected in `models/registry.json`): it is **CC-BY-4.0**, not
Apache-2.0; a **four-file** model (encoder + decoder_joint + `nemo128` mel preprocessor +
vocab, ~671 MB), not the single file named; and it needs **~2 GB inference RAM** —
10-15× the whisper q5 lane and far over the ≤250 MB ASR-resident budget (invariant 9,
the same budget that rejected base.en f16 at 268 MB in ADR-0015). Meanwhile `audio/vad.rs`
ships a dependency-free energy/timing `SpeechGate` stub, not a real detector — leaving
whisper exposed to silence-hallucination.

A parallel research sweep (6 agents, 2026-07-10) established the honest path: a real
Parakeet-V3 **TDT** adapter is achievable but is the wrong first unit (2 GB RAM; TDT
token+duration decoding is the highest correctness risk to hand-roll; the ready-made
`parakeet-rs` crate bypasses the VerifiedArtifact/sha256 seam and is single-maintainer
pre-1.0). The lowest-effort genuinely-correct ONNX ASR unit is a **CTC** lane.

## Decision
1. **Second ASR lane = a CTC-decode ONNX adapter, not TDT-Parakeet.** Add `engine/onnx.rs`
   (`OnnxCtcEngine`) behind the existing `spec.runtime == "onnxruntime"` seam
   (`engine::local_asr_stack`), structurally identical to `whisper.rs`. It runs the model's
   bundled `nemo128` mel-preprocessor ONNX graph (feature extraction is *free* — no
   hand-rolled DSP, the top risk finding) then the encoder graph, and decodes **greedy
   CTC**: per-frame argmax → collapse consecutive dups → strip the blank id → vocab/
   SentencePiece detok. A few dozen verifiable lines, no transducer loop. Appended LAST
   as `EngineLane::LocalCpu` — the ADR-0002 hedge. **whisper.cpp q5 stays every platform's
   default.** Real Parakeet-V3 TDT is deferred to a follow-up once a real TDT adapter and
   per-platform ≤250 MB evidence exist. Candidate model: Parakeet-CTC-110M ONNX
   (CC-BY-4.0) or a Conformer/Zipformer-CTC export (registry id `onnx-ctc-asr`).
2. **Real Silero VAD.** Add `SileroVad` as a new `impl VadDetector` in `audio/vad.rs`,
   reusing `SpeechGate`/`SpeechGateConfig`/`SpeechSegment` **unchanged**; `EnergyVad` stays
   the always-compiled, zero-dependency fallback. Silero v5 (512-sample @16 kHz frames,
   an LSTM state `(2,1,128)` threaded frame-to-frame, sr scalar), thresholded to a
   speech/silence bool — the *only* new logic; pre-roll/end-silence/segment assembly stay
   in the unchanged `SpeechGate`. Silero gates ASR eligibility only, never mutates or drops
   samples, and the **WAL still receives every sample first** (non-negotiable #2). Loads
   from the registry-verified local file, not a crate's `include_bytes!` (which would
   bypass the sha256 gate). `reset()` re-zeros state per stream.
3. **One shared native runtime.** ASR and VAD share a single `ort` dependency (two
   inference runtimes, not three). Both behind **off-by-default** features (`asr-onnx`,
   `vad-silero`); the default build compiles no `ort` and does no build-time fetch —
   mirrors the `asr-whisper`/`model-download` discipline.
4. **Binary provenance = load-dynamic against a pinned, checksummed asset.**
   `ort = { version = "=2.0.0-rc.12", default-features = false, features = ["ndarray"] }`
   with `ort/load-dynamic`. `default-features = false` is load-bearing: it drops `ort`'s
   default `download-binaries` feature (which fetches libonnxruntime from `cdn.pyke.io`
   **at build time**) plus its `ureq`/TLS chain, so the CDN path isn't even compiled.
   libonnxruntime **1.28.0** is shipped as a version-pinned, sha256-recorded LOCAL runtime
   asset resolved through the same VerifiedArtifact seam as the ggml model, pointed to via
   `ORT_DYLIB_PATH` / `ort::init_from(path)?.commit()` at warmup — so even feature-on
   builds do no network. The `ort` analogue of ADR-0014's `GGML_NATIVE=OFF` and "no weights
   in git / sha256 verify". (`ort` rc.12 requires rustc ≥ 1.88 — host 1.95, VM 1.96: OK.)
5. **Lane honesty (invariant 7).** The actually-selected `ort` ExecutionProvider is chosen
   and logged from one function and *is* the label; a build without an accel EP routes a
   requested GPU lane to `LocalCpu` before warmup/transcription/candidate/preflight/status/
   evidence — mirroring `effective_model_engine_lane`. A generic "GPU" is never backend proof.
6. **No new runtime egress.** `ort`/Silero inference is fully local — no source-level call
   site trips `audit-network` (its scan covers `apps`/`crates` `.rs`/`.ts`, not `ort::`
   nor dep `build.rs`). **No new `network-allowlist.json` entry** — the build-time binary
   is a pinned/vendored supply-chain artifact, not runtime egress; adding it there would
   mischaracterize it. A later weight fetch reuses the existing `models/download` entry
   (ADR-0015) once registry sha256+sources are real. **Owed hardening (tracked, not
   claimed done):** extend `audit-network`'s TODO deep pass (cargo-tree/geiger +
   cargo-deny + a `.cargo/config` offline pin) so an `ort`-style build-time binary fetch
   can't silently reopen egress and CI can build `--offline`.

## Alternatives considered
- **Keep `ort` `download-binaries` on:** build-time network the source audit can't see. Rejected.
- **`ort` in the default build:** every build pulls the binary + grows RAM; violates
  non-negotiable #1 / invariant 7. Rejected — off-by-default feature.
- **A `network-allowlist` entry for the ORT binary:** mischaracterizes a build-time
  supply-chain fetch as runtime egress. Rejected — pin/vendor instead.
- **Hand-roll Parakeet-V3 TDT now:** highest correctness risk (duration-driven frame
  skips) + ~2 GB RAM breaks invariant 9. Rejected — CTC first.
- **A separate runtime for Silero (e.g. its own crate binary):** `ort` is already needed;
  one dep serves both. Rejected.
- **Promote ONNX/Parakeet to any platform default now:** no per-platform ≤250 MB evidence.
  Rejected — ships as a non-default hedge (no `default_for`) until the bench table exists.
- **A second whisper.cpp model size as the "2nd lane" (honest fallback):** genuinely
  correct with near-zero new risk, but does NOT satisfy "behind ONNX Runtime" nor give
  runtime diversity, and Silero needs `ort` regardless — so doing the `ort` work once for
  both ASR and VAD is the better engineering call. Held as the fallback if the ORT native
  cost is judged too high.

## Open operator decisions (gate the LIVE proof / merge)
1. **ORT binary provenance** — approve the exact libonnxruntime 1.28.0 build + source +
   sha256 to vendor as the `load-dynamic` asset (rc.12 has no Intel-mac/musl prebuilt).
2. **ONNX ASR model + license** — pick the CTC model now (English CTC-110M, light) vs
   deferring; **accept the CC-BY-4.0 attribution obligation** in the app's notices.
3. **New model-download sources** — approve the pinned model + mirror hosts (GitHub/HF raw
   for the CTC model and Silero) before real sources land in `registry.json`.
4. **Silero source pin** — approve the official Silero v5.1.2 GitHub file (2,327,524 bytes,
   MIT) — NOT the HF quantized q4f16 variant — and the real sha256 to commit.
5. **Default-lane posture** — confirm whisper q5 stays every platform's default and the
   ONNX lane ships as a non-default hedge until direct per-platform ≤250 MB resident + WER
   + latency + idle-CPU evidence exists (invariant 9).

## Consequences
- **Easier:** ADR-0002's ≥2-lane hedge becomes real via an independent ONNX runtime lane;
  the real Silero VAD replaces the stub as the first defense against whisper
  silence-hallucination; both fit the existing `AsrEngine`/`VadDetector` contracts with no
  upstream plumbing change.
- **Harder / now owed (tracked, not done):** a native libonnxruntime to pin+vendor+checksum
  per OS/arch; `ort` is pre-1.0 rc (pin `=2.0.0-rc.12`); real sha256 + mirror sources for
  the `onnx-ctc-asr` and `silero-vad` entries (today TODO → fail-closed); a per-platform
  WER/latency/resident-RAM/idle-CPU bench table before any `default_for` promotion; and
  closing the `audit-network` build-time-fetch gap.
