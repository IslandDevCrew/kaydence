//! ONNX CTC ASR lane live proof (P1-P0-2 / ADR-0016) — feature `asr-onnx`.
//!
//! Drives the PRODUCTION dispatch (`local_asr_stack` → OnnxCtcEngine) over a real
//! 16 kHz mono clip and checks the transcript contains the expected words.
//!
//!   ORT_DYLIB_PATH=/abs/libonnxruntime.dylib \
//!   KAYDENCE_ONNX_MODEL=/abs/model.onnx  (with a sibling vocab.json) \
//!   KAYDENCE_ONNX_CLIP=/abs/clip16k.wav \
//!   KAYDENCE_ONNX_EXPECT="the quick brown fox" \
//!     cargo run --features asr-onnx --bin asr-onnx-selftest

fn main() {
    #[cfg(feature = "asr-onnx")]
    run();
    #[cfg(not(feature = "asr-onnx"))]
    eprintln!("asr-onnx-selftest needs --features asr-onnx");
}

#[cfg(feature = "asr-onnx")]
fn run() {
    use kaydence_lib::engine::{
        local_asr_stack, AsrRequest, EngineLane, LocalAsrAdapterSpec, LocalAsrAdapterState,
    };
    use kaydence_lib::events::SessionId;
    use ulid::Ulid;

    let model = std::env::var("KAYDENCE_ONNX_MODEL").expect("KAYDENCE_ONNX_MODEL");
    let clip = std::env::var("KAYDENCE_ONNX_CLIP").expect("KAYDENCE_ONNX_CLIP");
    let expect = std::env::var("KAYDENCE_ONNX_EXPECT").unwrap_or_default();

    let (rate, samples) = read_pcm16_mono_wav(std::path::Path::new(&clip)).expect("read clip");
    assert_eq!(rate, 16_000, "clip must be 16 kHz mono");
    let size = std::fs::metadata(&model).map(|m| m.len()).unwrap_or(0);

    let spec = LocalAsrAdapterSpec {
        model_id: "onnx-ctc-selftest".into(),
        lane: EngineLane::LocalCpu,
        runtime: "onnxruntime".into(),
        artifact_path: std::path::PathBuf::from(&model),
        artifact_size_bytes: size,
    };
    let mut stack = local_asr_stack(LocalAsrAdapterState::VerifiedArtifact { spec });

    let request = AsrRequest::new(SessionId::new(Ulid::new()), 16_000, 0, samples, Vec::new());
    let run = stack
        .transcribe(&request)
        .expect("transcription must succeed");
    let text = run.transcript.final_text;
    println!("[onnx-ctc] transcript: {text:?}");

    // Content check: every expected word appears (case-insensitive).
    let got = text.to_lowercase();
    let missing: Vec<&str> = expect
        .split_whitespace()
        .filter(|w| !got.contains(&w.to_lowercase()))
        .collect();
    if !text.trim().is_empty() && missing.is_empty() {
        println!(
            "[onnx-ctc] PASS — production local_asr_stack → OnnxCtcEngine transcribed real audio."
        );
    } else {
        eprintln!("[onnx-ctc] FAIL — missing words {missing:?} in {text:?}");
        std::process::exit(1);
    }
}

/// Minimal PCM16-mono WAV reader (walks the RIFF chunk list). Returns (rate, f32).
#[cfg(feature = "asr-onnx")]
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
        let sz = u32le(pos + 4) as usize;
        let body = pos + 8;
        if id == b"fmt " && body + 16 <= b.len() {
            channels = u16le(body + 2);
            rate = u32le(body + 4);
            bits = u16le(body + 14);
        } else if id == b"data" {
            let end = (body + sz).min(b.len());
            data = Some(&b[body..end]);
        }
        pos = body + sz + (sz & 1);
    }
    if channels != 1 || bits != 16 {
        return Err(format!("need PCM16 mono, got {channels}ch/{bits}bit"));
    }
    let data = data.ok_or("no data chunk")?;
    let samples = data
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
        .collect();
    Ok((rate, samples))
}
