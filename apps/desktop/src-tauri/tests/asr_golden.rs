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
            let read = wal::read_samples(&PathBuf::from(&clip))
                .unwrap_or_else(|e| panic!("failed to read {CLIP_ENV}={clip}: {e}"));
            assert_eq!(read.sample_rate, wal::SAMPLE_RATE, "clip must be 16 kHz mono");
            read.samples
        }
        Err(_) => {
            eprintln!("SKIP: {MODEL_ENV} set but {CLIP_ENV} missing — provide a 16 kHz mono wav.");
            return;
        }
    };
    assert!(!samples.is_empty(), "golden clip decoded to zero samples");

    let spec = LocalAsrAdapterSpec {
        model_id: "whisper-local-golden".to_string(),
        lane: EngineLane::LocalGpu,
        runtime: "whisper.cpp".to_string(),
        artifact_path: model_path,
        artifact_size_bytes: std::fs::metadata(&model).map(|m| m.len()).unwrap_or(0),
    };
    let mut stack = local_asr_stack(LocalAsrAdapterState::VerifiedArtifact { spec });

    let request = AsrRequest::new(
        SessionId::new(Ulid::new()),
        wal::SAMPLE_RATE,
        0,
        samples,
        vec!["Kaydence".to_string()],
    );
    let run = stack
        .transcribe(&request)
        .expect("real whisper.cpp transcription should succeed on a valid model+clip");

    let text = run.transcript.final_text.trim().to_string();
    eprintln!("whisper final_text = {text:?}");
    assert!(!text.is_empty(), "transcription produced empty final text");

    if let Ok(expect) = std::env::var(EXPECT_ENV) {
        let got = text.to_lowercase();
        let want = expect.trim().to_lowercase();
        assert!(
            got.contains(&want),
            "expected transcript to contain {want:?}, got {text:?}"
        );
    }
}
