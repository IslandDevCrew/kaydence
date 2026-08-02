//! Silero VAD live proof (P1-P0-2 / ADR-0016) — feature `vad-silero`.
//!
//! Runs the real Silero v5 ONNX detector over a 16 kHz mono WAV through the
//! production `SpeechGate<SileroVad>` and reports detected speech segments.
//! Needs the native ONNX Runtime (load-dynamic) and a local model:
//!
//!   ORT_DYLIB_PATH=/abs/libonnxruntime.dylib \
//!   KAYDENCE_SILERO_MODEL=/abs/silero_vad.onnx \
//!   KAYDENCE_VAD_CLIP=/abs/clip16k.wav \
//!     cargo run --features vad-silero --bin vad-selftest

fn main() {
    #[cfg(feature = "vad-silero")]
    run();
    #[cfg(not(feature = "vad-silero"))]
    eprintln!("vad-selftest needs --features vad-silero");
}

#[cfg(feature = "vad-silero")]
fn run() {
    use kaydence_lib::audio::vad::{SileroVad, SpeechGate, SpeechGateConfig, VadDetector};

    let model = std::env::var("KAYDENCE_SILERO_MODEL").expect("KAYDENCE_SILERO_MODEL");
    let clip = std::env::var("KAYDENCE_VAD_CLIP").expect("KAYDENCE_VAD_CLIP");

    let (rate, samples) = read_pcm16_mono_wav(std::path::Path::new(&clip)).expect("read clip");
    assert_eq!(rate, 16_000, "clip must be 16 kHz mono");
    println!(
        "[vad] clip: {} samples ({:.2}s @16k)",
        samples.len(),
        samples.len() as f32 / 16_000.0
    );

    // 1) Raw per-frame probabilities: count how many 512-frames read as speech.
    let mut det =
        SileroVad::from_model_path(std::path::Path::new(&model), 0.5).expect("load model");
    let mut speech_frames = 0usize;
    let mut total = 0usize;
    for chunk in samples.chunks(512) {
        total += 1;
        if det.is_speech(chunk) {
            speech_frames += 1;
        }
    }
    println!("[vad] speech frames: {speech_frames}/{total}");

    // 2) Full SpeechGate segmentation (pre-roll 250ms + end-silence 300ms).
    det.reset();
    let cfg = SpeechGateConfig::default();
    let mut gate = SpeechGate::new(det, cfg);
    let mut segs = gate.push_samples(&samples);
    segs.extend(gate.flush());
    println!("[vad] segments detected: {}", segs.len());
    for (i, s) in segs.iter().enumerate() {
        println!(
            "  segment {}: start_sample={} len={} ({:.2}s)",
            i + 1,
            s.start_sample,
            s.samples.len(),
            s.samples.len() as f32 / 16_000.0
        );
    }

    // 3) Silence must produce NO speech frames (fail-closed correctness).
    let mut det2 =
        SileroVad::from_model_path(std::path::Path::new(&model), 0.5).expect("load model");
    let silence = vec![0f32; 16_000];
    let sil_speech = silence.chunks(512).filter(|c| det2.is_speech(c)).count();
    println!("[vad] silence speech frames: {sil_speech} (expect 0)");

    if speech_frames > 0 && !segs.is_empty() && sil_speech == 0 {
        println!(
            "[vad] PASS — Silero detected speech in the clip, segmented it, and rejected silence."
        );
    } else {
        eprintln!(
            "[vad] FAIL — speech_frames={speech_frames} segments={} silence={sil_speech}",
            segs.len()
        );
        std::process::exit(1);
    }
}

/// Minimal PCM16-mono WAV reader (walks the RIFF chunk list). Returns (rate, f32).
#[cfg(feature = "vad-silero")]
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
