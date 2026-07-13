//! Real whisper.cpp ASR adapter (P1-P0-2, ADR-0014) — feature `asr-whisper`.
//!
//! Plugs into the existing `AsrEngine` trait and the `VerifiedArtifact` seam:
//! a checksum-verified LOCAL ggml model (path from `LocalAsrAdapterSpec`) is
//! loaded during runtime warmup (with lazy transcription fallback) and run over
//! the 16 kHz mono f32 the WAL already produces (`audio/wal.rs`). No network
//! surface — the model file is supplied locally; on-demand download stays a
//! separate gated decision (ADR-0014 §4).
//!
//! This whole module compiles only under `--features asr-whisper`, so the
//! default build stays free of the native whisper.cpp compile.

use super::{
    AsrEngine, AsrError, AsrRequest, AsrTranscript, AsrWarmup, EngineLane, LocalAsrAdapterSpec,
    PartialTranscript,
};
use crate::audio::wal::SAMPLE_RATE;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// A whisper.cpp-backed engine. Construction is infallible; explicit runtime
/// warmup loads the model before dictation, while `transcribe` retains a lazy
/// fallback for direct callers that do not run the lifecycle hook.
pub struct WhisperCppEngine {
    spec: LocalAsrAdapterSpec,
    ctx: Option<WhisperContext>,
}

impl WhisperCppEngine {
    pub fn boxed(spec: LocalAsrAdapterSpec) -> Box<dyn AsrEngine + Send> {
        Box::new(Self { spec, ctx: None })
    }

    fn ensure_loaded(&mut self) -> Result<&WhisperContext, AsrError> {
        if self.ctx.is_none() {
            let path = self.spec.artifact_path.to_string_lossy().to_string();
            // Lane-aware acceleration: the LocalCpu lane forces GPU off so the
            // same model serves as the real CPU fallback when the GPU lane is
            // unavailable (engine/AGENTS.md fallback chain). LocalGpu/BYOK use
            // GPU where the build supports it.
            let mut params = WhisperContextParameters::default();
            params.use_gpu(!matches!(self.spec.lane, EngineLane::LocalCpu));
            let ctx = WhisperContext::new_with_params(&path, params).map_err(|e| {
                AsrError::Unavailable(format!(
                    "failed to load whisper model '{}' at {}: {e}",
                    self.spec.model_id, path
                ))
            })?;
            self.ctx = Some(ctx);
        }
        self.ctx.as_ref().ok_or_else(|| {
            AsrError::Unavailable("whisper context was not retained after loading".to_string())
        })
    }
}

impl AsrEngine for WhisperCppEngine {
    fn lane(&self) -> EngineLane {
        self.spec.lane
    }

    fn warm_up(&mut self) -> Result<AsrWarmup, AsrError> {
        self.ensure_loaded().map(|_| AsrWarmup::Ready)
    }

    fn transcribe(&mut self, request: &AsrRequest) -> Result<AsrTranscript, AsrError> {
        // whisper.cpp expects 16 kHz mono f32 — exactly the WAL/engine boundary
        // (audio/AGENTS.md). Reject anything else rather than silently mis-decode.
        if request.sample_rate != SAMPLE_RATE {
            return Err(AsrError::Inference(format!(
                "whisper adapter requires {SAMPLE_RATE} Hz mono f32, got {} Hz",
                request.sample_rate
            )));
        }

        let model_id = self.spec.model_id.clone();
        let dictionary_hints = request.dictionary_hints.clone();
        let ctx = self.ensure_loaded()?;
        let mut state = ctx
            .create_state()
            .map_err(|e| AsrError::Inference(format!("whisper create_state failed: {e}")))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        // Keep whisper.cpp silent on stdout/stderr — this is a library path.
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        // Dictionary hints bias recognition via the initial prompt (engine/AGENTS.md
        // invariant 3: accept hints where the engine supports them).
        let prompt;
        if !dictionary_hints.is_empty() {
            prompt = dictionary_hints.join(", ");
            params.set_initial_prompt(&prompt);
        }

        state.full(params, &request.samples).map_err(|e| {
            AsrError::Inference(format!("whisper inference failed on {model_id}: {e}"))
        })?;

        let num_segments = state.full_n_segments();

        let mut partials = Vec::new();
        let mut final_text = String::new();
        for i in 0..num_segments {
            let segment = state
                .get_segment(i)
                .ok_or_else(|| AsrError::Inference(format!("whisper segment {i} missing")))?;
            let seg = segment.to_str().map_err(|e| {
                AsrError::Inference(format!("whisper segment {i} text failed: {e}"))
            })?;
            let trimmed = seg.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !final_text.is_empty() {
                final_text.push(' ');
            }
            final_text.push_str(trimmed);
            // Emit the accumulated text as a coarse partial per segment; real
            // streaming partials land with the streaming path (deferred, ADR-0014).
            partials.push(PartialTranscript {
                text: final_text.clone(),
                t_lag_ms: 0,
            });
        }

        let final_text = final_text.trim().to_string();
        if final_text.is_empty() {
            return Err(AsrError::EmptyTranscript);
        }

        Ok(AsrTranscript {
            partials,
            final_text,
            no_speech_probability: None,
        })
    }
}
