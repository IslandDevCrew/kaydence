//! Second local ASR lane — a CTC-decode ONNX adapter (P1-P0-2, ADR-0016),
//! the ADR-0002 supply-chain hedge alongside the whisper.cpp lane. Feature
//! `asr-onnx`; compiles only when opted in, so the default build carries no ort.
//!
//! Plugs into the existing `AsrEngine` trait + the `VerifiedArtifact` seam exactly
//! like `whisper.rs`. Runs a raw-audio char-CTC model (wav2vec2-class): 16 kHz
//! mono f32 in → logits [1, T, V] → **greedy CTC** (argmax per frame, collapse
//! consecutive dups, drop the blank id, map ids through the model's vocab). No
//! transducer loop, no external mel DSP (the raw-audio graph subsumes it), no
//! SentencePiece — a few dozen provable lines. The native libonnxruntime is a
//! load-dynamic runtime asset (ORT_DYLIB_PATH), pinned+checksummed; no build-time
//! network. Model + sidecar `vocab.json` load from registry-verified local paths.
#![cfg(feature = "asr-onnx")]

use super::{
    AsrEngine, AsrError, AsrRequest, AsrTranscript, AsrWarmup, EngineLane, LocalAsrAdapterSpec,
};
use crate::audio::wal::SAMPLE_RATE;
use ndarray::Array2;
use ort::session::Session;
use ort::value::Tensor;
use std::collections::HashMap;
use std::path::Path;

/// The CTC blank id (index 0) — the universal convention for these exports
/// (`<pad>` in the wav2vec2 vocab).
const BLANK_ID: usize = 0;

pub struct OnnxCtcEngine {
    spec: LocalAsrAdapterSpec,
    session: Option<Session>,
    /// id → token (loaded from the model's sidecar `vocab.json`).
    vocab: Vec<String>,
}

impl OnnxCtcEngine {
    pub fn boxed(spec: LocalAsrAdapterSpec) -> Box<dyn AsrEngine + Send> {
        Box::new(Self {
            spec,
            session: None,
            vocab: Vec::new(),
        })
    }

    fn ensure_loaded(&mut self) -> Result<(), AsrError> {
        if self.session.is_none() {
            let session = Session::builder()
                .and_then(|mut b| b.commit_from_file(&self.spec.artifact_path))
                .map_err(|e| {
                    AsrError::Unavailable(format!(
                        "failed to load onnx model '{}' at {}: {e}",
                        self.spec.model_id,
                        self.spec.artifact_path.display()
                    ))
                })?;
            self.session = Some(session);
        }
        if self.vocab.is_empty() {
            self.vocab = load_vocab(&self.spec.artifact_path)?;
        }
        Ok(())
    }

    /// Greedy CTC over logits `[1, T, V]`: argmax per frame, collapse runs, drop
    /// the blank, map to tokens (`|` → space, `<...>` specials skipped). Takes the
    /// vocab slice (not `&self`) so it borrows a field disjoint from the session.
    fn decode(vocab: &[String], logits: &[f32], frames: usize, classes: usize) -> String {
        let mut text = String::new();
        let mut prev = usize::MAX;
        for f in 0..frames {
            let base = f * classes;
            let mut best = 0usize;
            let mut best_val = f32::MIN;
            for c in 0..classes {
                let v = logits[base + c];
                if v > best_val {
                    best_val = v;
                    best = c;
                }
            }
            if best != prev {
                if best != BLANK_ID {
                    match vocab.get(best).map(String::as_str) {
                        Some("|") => text.push(' '),
                        Some(tok) if tok.starts_with('<') && tok.ends_with('>') => {}
                        Some(tok) => text.push_str(tok),
                        None => {}
                    }
                }
                prev = best;
            }
        }
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

impl AsrEngine for OnnxCtcEngine {
    fn lane(&self) -> EngineLane {
        self.spec.lane
    }

    fn warm_up(&mut self) -> Result<AsrWarmup, AsrError> {
        self.ensure_loaded().map(|_| AsrWarmup::Ready)
    }

    fn transcribe(&mut self, request: &AsrRequest) -> Result<AsrTranscript, AsrError> {
        if request.sample_rate != SAMPLE_RATE {
            return Err(AsrError::Inference(format!(
                "onnx ctc adapter requires {SAMPLE_RATE} Hz mono f32, got {} Hz",
                request.sample_rate
            )));
        }
        if request.samples.is_empty() {
            return Err(AsrError::EmptyTranscript);
        }
        self.ensure_loaded()?;

        // Per-utterance zero-mean unit-variance normalization (wav2vec2
        // do_normalize=true). Raw-audio CTC models fold feature extraction into
        // the graph, so this is the only front-end step.
        let n = request.samples.len();
        let mean = request.samples.iter().sum::<f32>() / n as f32;
        let var = request
            .samples
            .iter()
            .map(|x| (x - mean) * (x - mean))
            .sum::<f32>()
            / n as f32;
        let std = (var + 1e-7).sqrt();
        let norm: Vec<f32> = request.samples.iter().map(|x| (x - mean) / std).collect();

        let input =
            Array2::from_shape_vec((1, n), norm).map_err(|e| AsrError::Inference(e.to_string()))?;
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| AsrError::Unavailable("onnx session not loaded".into()))?;

        let outputs = session
            .run(ort::inputs![
                "input_values" => Tensor::from_array(input).map_err(|e| AsrError::Inference(e.to_string()))?,
            ])
            .map_err(|e| AsrError::Inference(format!("onnx inference failed: {e}")))?;

        let (shape, logits) = outputs["logits"]
            .try_extract_tensor::<f32>()
            .map_err(|e| AsrError::Inference(format!("onnx logits extract: {e}")))?;
        // logits shape: [1, T, V]
        if shape.len() != 3 {
            return Err(AsrError::Inference(format!(
                "onnx ctc expected rank-3 logits, got shape {shape:?}"
            )));
        }
        let frames = shape[1] as usize;
        let classes = shape[2] as usize;

        let final_text = Self::decode(&self.vocab, logits, frames, classes);
        if final_text.is_empty() {
            return Err(AsrError::EmptyTranscript);
        }
        Ok(AsrTranscript {
            partials: Vec::new(),
            final_text,
            no_speech_probability: None,
        })
    }
}

/// Load the model's sidecar `vocab.json` (`{token: id}`) as an id→token table.
fn load_vocab(model_path: &Path) -> Result<Vec<String>, AsrError> {
    let vpath = model_path
        .parent()
        .map(|p| p.join("vocab.json"))
        .ok_or_else(|| AsrError::Unavailable("model path has no parent for vocab.json".into()))?;
    let bytes = std::fs::read(&vpath)
        .map_err(|e| AsrError::Unavailable(format!("read vocab {}: {e}", vpath.display())))?;
    let map: HashMap<String, usize> = serde_json::from_slice(&bytes)
        .map_err(|e| AsrError::Unavailable(format!("parse vocab {}: {e}", vpath.display())))?;
    let size = map.values().copied().max().map(|m| m + 1).unwrap_or(0);
    let mut table = vec![String::new(); size];
    for (tok, id) in map {
        if id < size {
            table[id] = tok;
        }
    }
    Ok(table)
}
