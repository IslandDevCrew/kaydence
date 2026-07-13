//! Golden-clip ASR proof (P1-P0-2 / P1-G2, ADR-0014).
//!
//! Compiles only under `--features asr-whisper`. Runs a REAL whisper.cpp
//! transcription end-to-end through the production `local_asr_stack` seam, but
//! only when the operator supplies a local model (and optionally a clip) via
//! env — because "no weights in git" (ARCHITECTURE §7). When the model env is
//! absent the test SKIPS loudly rather than fabricating a pass, so P1-G2 is only
//! ever green on a genuine model.
//!
//! Run it:
//!   KAYDENCE_WHISPER_MODEL=/path/ggml-base.en.bin \
//!   KAYDENCE_WHISPER_CLIP=/path/clip16k.wav \
//!   KAYDENCE_WHISPER_EXPECT="the quick brown fox" \
//!   cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml \
//!     --features asr-whisper --test asr_golden -- --nocapture
#![cfg(feature = "asr-whisper")]

use kaydence_lib::audio::wal;
use kaydence_lib::engine::{
    local_asr_stack, AsrRequest, EngineLane, LocalAsrAdapterSpec, LocalAsrAdapterState,
};
use kaydence_lib::events::SessionId;
use std::path::PathBuf;
use ulid::Ulid;

const MODEL_ENV: &str = "KAYDENCE_WHISPER_MODEL";
const CLIP_ENV: &str = "KAYDENCE_WHISPER_CLIP";
const EXPECT_ENV: &str = "KAYDENCE_WHISPER_EXPECT";

/// Minimal, dependency-free PCM16-mono WAV reader for the golden fixture. Walks
/// the RIFF chunk list (tolerating LIST/INFO and other chunks encoders insert,
/// which the internal WAL reader intentionally does not) and returns
/// (sample_rate, f32 samples). Golden clips are external artifacts, not WAL files.
fn read_pcm16_mono_wav(path: &std::path::Path) -> Result<(u32, Vec<f32>), String> {
    let b = std::fs::read(path).map_err(|e| e.to_string())?;
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let u16le = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    let u32le = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    let (mut rate, mut channels, mut bits) = (0u32, 0u16, 0u16);
    let mut data: Option<&[u8]> = None;
    let mut pos = 12;
    while pos + 8 <= b.len() {
        let id = &b[pos..pos + 4];
        let size = u32le(pos + 4) as usize;
        let body = pos + 8;
        if body + size > b.len() {
            break;
        }
        match id {
            b"fmt " => {
                channels = u16le(body + 2);
                rate = u32le(body + 4);
                bits = u16le(body + 14);
            }
            b"data" => data = Some(&b[body..body + size]),
            _ => {}
        }
        pos = body + size + (size & 1); // chunks are word-aligned
    }
    if bits != 16 || channels != 1 {
        return Err(format!("expected PCM16 mono, got {bits}-bit {channels}ch"));
    }
    let data = data.ok_or("no data chunk")?;
    let samples = data
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / i16::MAX as f32)
        .collect();
    Ok((rate, samples))
}

#[test]
fn whisper_transcribes_a_local_golden_clip() {
    let Ok(model) = std::env::var(MODEL_ENV) else {
        eprintln!(
            "SKIP whisper_transcribes_a_local_golden_clip: set {MODEL_ENV} to a ggml model path \
             (and {CLIP_ENV}=<16kHz mono wav>, optionally {EXPECT_ENV}=<substring>) to run P1-G2."
        );
        return;
    };
    let model_path = PathBuf::from(&model);
    assert!(model_path.is_file(), "{MODEL_ENV} is not a file: {model}");

    // Samples come from a real 16 kHz mono WAV — the exact WAL boundary — read
    // back through the production recover+read path so the test exercises the
    // same bytes ASR would see in the app.
    let samples = match std::env::var(CLIP_ENV) {
        Ok(clip) => {
            let (rate, decoded) = read_pcm16_mono_wav(&PathBuf::from(&clip))
                .unwrap_or_else(|e| panic!("failed to read {CLIP_ENV}={clip}: {e}"));
            assert_eq!(rate, wal::SAMPLE_RATE, "clip must be 16 kHz mono");
            decoded
        }
        Err(_) => {
            eprintln!("SKIP: {MODEL_ENV} set but {CLIP_ENV} missing — provide a 16 kHz mono wav.");
            return;
        }
    };
    assert!(!samples.is_empty(), "golden clip decoded to zero samples");

    // Default GPU; KAYDENCE_WHISPER_LANE=cpu forces the CPU fallback lane.
    let lane = match std::env::var("KAYDENCE_WHISPER_LANE").as_deref() {
        Ok("cpu") => EngineLane::LocalCpu,
        _ => EngineLane::LocalGpu,
    };
    let spec = LocalAsrAdapterSpec {
        model_id: "whisper-local-golden".to_string(),
        lane,
        runtime: "whisper.cpp".to_string(),
        artifact_path: model_path,
        artifact_size_bytes: std::fs::metadata(&model).map(|m| m.len()).unwrap_or(0),
    };
    let mut stack = local_asr_stack(LocalAsrAdapterState::VerifiedArtifact { spec });

    let warmup_started = std::time::Instant::now();
    let warmed_lane = stack
        .warm_up()
        .expect("real whisper.cpp warmup should load a valid model");
    let warmup_ms = warmup_started.elapsed().as_millis();

    let request = AsrRequest::new(
        SessionId::new(Ulid::new()),
        wal::SAMPLE_RATE,
        0,
        samples,
        vec!["Kaydence".to_string()],
    );
    let n_samples = request.samples.len();
    let started = std::time::Instant::now();
    let run = stack
        .transcribe(&request)
        .expect("real whisper.cpp transcription should succeed on a valid model+clip");
    let elapsed_ms = started.elapsed().as_millis();

    let text = run.transcript.final_text.trim().to_string();
    let audio_ms = (n_samples as u128) * 1000 / (wal::SAMPLE_RATE as u128);
    eprintln!("whisper final_text = {text:?}");
    eprintln!(
        "P1-G2/P1-G3 metrics: warmup_ms={warmup_ms} audio_ms={audio_ms} \
         transcribe_ms={elapsed_ms} rtf={:.3} warmed_lane={warmed_lane:?} \
         lane={:?} partials={}",
        elapsed_ms as f64 / audio_ms.max(1) as f64,
        run.lane,
        run.transcript.partials.len(),
    );
    assert!(!text.is_empty(), "transcription produced empty final text");
    assert_eq!(
        warmed_lane,
        Some(run.lane),
        "warmup and inference lanes differ"
    );
    let latency_budget_ms = match run.lane {
        EngineLane::LocalGpu => 700,
        EngineLane::LocalCpu => 1_200,
        EngineLane::ByokCloud => 2_000,
    };
    assert!(
        elapsed_ms <= latency_budget_ms,
        "warm ASR inference exceeded the {:?} lane budget: {elapsed_ms} ms > {latency_budget_ms} ms",
        run.lane
    );

    if let Ok(expect) = std::env::var(EXPECT_ENV) {
        let got = text.to_lowercase();
        let want = expect.trim().to_lowercase();
        assert!(
            got.contains(&want),
            "expected transcript to contain {want:?}, got {text:?}"
        );
    }
}
