#[cfg(any(feature = "asr-whisper", test))]
const MINIMUM_SAMPLE_COUNT: u64 = 5;
#[cfg(any(feature = "asr-whisper", test))]
const MAXIMUM_IDLE_MS: u64 = 30_000;
#[cfg(any(feature = "asr-whisper", test))]
fn nearest_rank(samples: &[u64], percentile: u8) -> u64 {
    assert!(!samples.is_empty());
    assert!((1..=100).contains(&percentile), "invalid percentile");
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let rank = (usize::from(percentile) * sorted.len()).div_ceil(100);
    sorted[rank - 1]
}
#[cfg(any(feature = "asr-whisper", test))]
fn parse_sample_count(value: Option<&str>) -> Result<u64, String> {
    let count = value
        .unwrap_or("10")
        .parse::<u64>()
        .map_err(|_| format!("KAYDENCE_REFERENCE_SAMPLES must be an integer, got {value:?}"))?;
    if count < MINIMUM_SAMPLE_COUNT {
        return Err(format!(
            "KAYDENCE_REFERENCE_SAMPLES must be at least {MINIMUM_SAMPLE_COUNT}, got {count}"
        ));
    }
    Ok(count)
}
#[cfg(any(feature = "asr-whisper", test))]
fn parse_sha256(value: Option<&str>) -> Result<String, String> {
    let value = value.ok_or_else(|| "KAYDENCE_WHISPER_SHA256 must be set".to_string())?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("KAYDENCE_WHISPER_SHA256 must be exactly 64 hex characters".into());
    }
    Ok(value.to_ascii_lowercase())
}
#[cfg(any(feature = "asr-whisper", test))]
fn parse_model_id(value: Option<&str>) -> Result<String, String> {
    let value = value.ok_or_else(|| "KAYDENCE_WHISPER_MODEL_ID must be set".to_string())?;
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(
            "KAYDENCE_WHISPER_MODEL_ID must be a 1-64 character [A-Za-z0-9._-] identifier".into(),
        );
    }
    Ok(value.to_string())
}
#[cfg(any(feature = "asr-whisper", test))]
fn parse_idle_ms(value: Option<&str>) -> Result<u64, String> {
    let idle_ms = value
        .unwrap_or("0")
        .parse::<u64>()
        .map_err(|_| "KAYDENCE_REFERENCE_IDLE_MS must be an integer".to_string())?;
    if idle_ms > MAXIMUM_IDLE_MS {
        return Err(format!(
            "KAYDENCE_REFERENCE_IDLE_MS must be at most {MAXIMUM_IDLE_MS}"
        ));
    }
    Ok(idle_ms)
}
#[cfg(feature = "asr-whisper")]
fn main() {
    probe::main();
}
#[cfg(not(feature = "asr-whisper"))]
fn main() {
    eprintln!("reference-bench requires --features asr-whisper");
    std::process::exit(2);
}
#[cfg(feature = "asr-whisper")]
mod probe {
    use super::{nearest_rank, parse_idle_ms, parse_model_id, parse_sample_count, parse_sha256};
    use kaydence_lib::audio::{self, vad::EnergyVad, vad::SpeechGateConfig, wal};
    use kaydence_lib::engine::{
        local_asr_stack, AsrEngine, AsrError, AsrRequest, AsrTranscript, EngineLane, EngineStack,
        LocalAsrAdapterSpec, LocalAsrAdapterState,
    };
    use kaydence_lib::events::{CleanupDial, InjectMethod, SessionEvent, SessionId};
    use kaydence_lib::inject::{
        inject_committed_text, FieldKind, InjectError, InjectorCaps, KeystrokeChannel,
        TextInjector, UnknownFieldPolicy,
    };
    use kaydence_lib::pipeline::{self, TranscriptionPipeline};
    use sha2::{Digest, Sha256};
    use std::fs::{self, OpenOptions};
    use std::io::{Read, Write};
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};
    use ulid::Ulid;
    const LANE: &str = "KAYDENCE_WHISPER_LANE";
    const READY: &str = "KAYDENCE_REFERENCE_READY_FILE";
    #[derive(Debug, thiserror::Error)]
    enum Error {
        #[error("{0}")]
        Config(String),
        #[error("pipeline: {0}")]
        Pipeline(#[from] pipeline::PipelineError),
        #[error("file IO: {0}")]
        Io(#[from] std::io::Error),
    }
    struct Config {
        model: PathBuf,
        model_id: String,
        model_sha256: String,
        clip: PathBuf,
        lane: EngineLane,
        count: u64,
        idle_ms: u64,
        ready: Option<PathBuf>,
    }
    impl Config {
        fn from_env() -> Result<Self, Error> {
            let lane = match std::env::var(LANE).as_deref() {
                Ok("gpu") | Err(_) => EngineLane::LocalGpu,
                Ok("cpu") => EngineLane::LocalCpu,
                Ok(value) => {
                    return Err(config(format!("{LANE} must be gpu or cpu, got {value:?}")))
                }
            };
            Ok(Self {
                model: required_file("KAYDENCE_WHISPER_MODEL")?,
                model_id: parse_model_id(
                    std::env::var("KAYDENCE_WHISPER_MODEL_ID").ok().as_deref(),
                )
                .map_err(Error::Config)?,
                model_sha256: parse_sha256(
                    std::env::var("KAYDENCE_WHISPER_SHA256").ok().as_deref(),
                )
                .map_err(Error::Config)?,
                clip: required_file("KAYDENCE_WHISPER_CLIP")?,
                lane,
                count: parse_sample_count(
                    std::env::var("KAYDENCE_REFERENCE_SAMPLES").ok().as_deref(),
                )
                .map_err(Error::Config)?,
                idle_ms: parse_idle_ms(std::env::var("KAYDENCE_REFERENCE_IDLE_MS").ok().as_deref())
                    .map_err(Error::Config)?,
                ready: std::env::var_os(READY).map(PathBuf::from),
            })
        }
    }
    struct Fixture(audio::CaptureSessionSummary, PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.1);
        }
    }
    struct PinnedLaneEngine(EngineStack, EngineLane);
    impl AsrEngine for PinnedLaneEngine {
        fn lane(&self) -> EngineLane {
            self.1
        }
        fn transcribe(&mut self, request: &AsrRequest) -> Result<AsrTranscript, AsrError> {
            let run = self.0.transcribe(request)?;
            if run.lane != self.1 {
                return Err(AsrError::Inference(format!(
                    "reference benchmark lane mismatch: expected {}, got {}",
                    lane_label(self.1),
                    lane_label(run.lane)
                )));
            }
            Ok(run.transcript)
        }
    }
    struct PolicySink;
    impl TextInjector for PolicySink {
        fn caps(&self) -> InjectorCaps {
            InjectorCaps {
                native_text_insert: true,
                keystroke: KeystrokeChannel::None,
                clipboard: false,
            }
        }
        fn focused_field(&self) -> FieldKind {
            FieldKind::Editable
        }
        fn insert_native(&mut self, _text: &str) -> Result<(), InjectError> {
            Ok(())
        }
        fn synth_text(&mut self, _text: &str) -> Result<(), InjectError> {
            Err(InjectError("policy sink cannot synthesize text".into()))
        }
    }
    pub fn main() {
        match run() {
            Ok(report) => println!("{report}"),
            Err(error) => {
                eprintln!("reference-bench: {error}");
                std::process::exit(1);
            }
        }
    }
    fn run() -> Result<serde_json::Value, Error> {
        let settings = Config::from_env()?;
        let model_sha256 = hash_file_sha256(&settings.model)?;
        if model_sha256 != settings.model_sha256 {
            return Err(config(
                "KAYDENCE_WHISPER_MODEL sha256 did not match KAYDENCE_WHISPER_SHA256",
            ));
        }
        let fixture_sha256 = hash_file_sha256(&settings.clip)?;
        let samples = read_wav(&settings.clip)?;
        let audio_ms = samples.len() as u64 * 1_000 / u64::from(wal::SAMPLE_RATE);
        let fixture = create_fixture(&samples)?;
        let spec = LocalAsrAdapterSpec {
            model_id: settings.model_id.clone(),
            lane: settings.lane,
            runtime: "whisper.cpp".into(),
            artifact_size_bytes: fs::metadata(&settings.model)?.len(),
            artifact_path: settings.model,
        };
        let mut inner = local_asr_stack(LocalAsrAdapterState::VerifiedArtifact { spec });
        let started = Instant::now();
        let warmed_lane = inner
            .warm_up()
            .map_err(pipeline::PipelineError::from)?
            .ok_or_else(|| config("the selected ASR stack did not warm a lane"))?;
        let warmup_ms = elapsed_ms(started.elapsed());
        let outer = EngineStack::new(vec![Box::new(PinnedLaneEngine(inner, warmed_lane))]);
        let mut pipeline =
            TranscriptionPipeline::new(EnergyVad::new(0.01), SpeechGateConfig::default(), outer)
                .with_cleanup_dial(CleanupDial::Raw);
        let mut timings = Vec::new();
        let mut events = Vec::new();
        let mut last_committed_text = String::new();
        for _ in 0..settings.count {
            let started = Instant::now();
            let measured = pipeline.process_capture(&fixture.0)?;
            validate_measurement(&measured).map_err(config)?;
            let committed = pipeline::committed_text(&measured)
                .ok_or_else(|| config("the ASR pipeline produced no committed text"))?;
            let delivered = inject_committed_text(
                &mut PolicySink,
                committed.id,
                &committed.text,
                UnknownFieldPolicy::Strict,
                false,
            );
            if !matches!(
                delivered,
                SessionEvent::Injected {
                    method: InjectMethod::Native,
                    ..
                }
            ) {
                return Err(config(format!(
                    "policy delivery did not use the native path: {delivered:?}"
                )));
            }
            timings.push(elapsed_ms(started.elapsed()));
            last_committed_text = committed.text;
            events = measured;
        }
        let report = serde_json::json!({
            "schema": 1,
            "platform": std::env::consts::OS,
            "lane": lane_label(warmed_lane),
            "model_id": settings.model_id,
            "model_sha256": model_sha256,
            "fixture_sha256": fixture_sha256,
            "warmup_ms": warmup_ms,
            "sample_count": settings.count,
            "release_to_delivery_policy_p50_ms": nearest_rank(&timings, 50),
            "release_to_delivery_policy_p95_ms": nearest_rank(&timings, 95),
            "audio_ms": audio_ms,
            "transcript_nonempty": !last_committed_text.trim().is_empty(),
            "events": event_names(&events),
            "unmeasured": ["physical_os_field_injection"],
        });
        if let Some(path) = settings.ready.as_deref() {
            write_ready_file(path, settings.idle_ms)?;
        }
        std::thread::sleep(Duration::from_millis(settings.idle_ms));
        std::hint::black_box(&pipeline);
        Ok(report)
    }
    fn config(message: impl Into<String>) -> Error {
        Error::Config(message.into())
    }
    fn validate_measurement(events: &[SessionEvent]) -> Result<(), &'static str> {
        (!events
            .iter()
            .any(|event| matches!(event, SessionEvent::Failed { .. })))
        .then_some(())
        .ok_or("reference benchmark sample contains a failed event")
    }
    fn required_file(name: &str) -> Result<PathBuf, Error> {
        let path = std::env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| config(format!("{name} must be set")))?;
        if path.is_file() {
            Ok(path)
        } else {
            Err(config(format!("{name} is not a file: {}", path.display())))
        }
    }
    fn read_wav(path: &Path) -> Result<Vec<f32>, Error> {
        let bytes = fs::read(path)?;
        if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(config(format!(
                "{} is not a RIFF/WAVE file",
                path.display()
            )));
        }
        let (mut format, mut data, mut position) = (None, None, 12usize);
        while position + 8 <= bytes.len() {
            let id = &bytes[position..position + 4];
            let size = le32(&bytes, position + 4) as usize;
            let body = position + 8;
            let end = body
                .checked_add(size)
                .ok_or_else(|| config("oversized WAV chunk"))?;
            if end > bytes.len() {
                return Err(config(format!(
                    "{} has a truncated WAV chunk",
                    path.display()
                )));
            }
            match id {
                b"fmt " if size >= 16 => {
                    format = Some((
                        le16(&bytes, body),
                        le16(&bytes, body + 2),
                        le32(&bytes, body + 4),
                        le16(&bytes, body + 14),
                    ));
                }
                b"fmt " => return Err(config("truncated WAV fmt chunk")),
                b"data" => data = Some(&bytes[body..end]),
                _ => {}
            }
            position = end + (size & 1);
        }
        if format != Some((1, 1, wal::SAMPLE_RATE, 16)) {
            return Err(config(format!(
                "{} must be PCM16 mono at {} Hz",
                path.display(),
                wal::SAMPLE_RATE
            )));
        }
        let data = data.ok_or_else(|| config(format!("{} has no data chunk", path.display())))?;
        if data.is_empty() || data.len() % 2 != 0 {
            return Err(config(format!("{} has invalid PCM16 data", path.display())));
        }
        Ok(data
            .chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]) as f32 / i16::MAX as f32)
            .collect())
    }
    fn hash_file_sha256(path: &Path) -> Result<String, Error> {
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }
    fn le16(bytes: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
    }
    fn le32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    }
    fn create_fixture(samples: &[f32]) -> Result<Fixture, pipeline::PipelineError> {
        let directory = std::env::temp_dir().join(format!("kaydence-bench-{}", Ulid::new()));
        let id = SessionId::new(Ulid::new());
        let mut writer = wal::WalWriter::create(&directory.join("sessions"), &id.0.to_string())?;
        writer.append(samples)?;
        let wal_path = writer.finalize()?;
        Ok(Fixture(
            audio::CaptureSessionSummary {
                id,
                wal_path,
                samples_written: samples.len() as u64,
                dropped_input_samples: 0,
                started_ms: 0,
                finalized_ms: samples.len() as u64 * 1_000 / u64::from(wal::SAMPLE_RATE),
            },
            directory,
        ))
    }
    fn elapsed_ms(duration: Duration) -> u64 {
        duration.as_micros().div_ceil(1_000) as u64
    }
    fn lane_label(lane: EngineLane) -> &'static str {
        match lane {
            EngineLane::ByokCloud => "byok_cloud",
            EngineLane::LocalGpu => "local_gpu",
            EngineLane::LocalCpu => "local_cpu",
        }
    }
    fn event_names(events: &[SessionEvent]) -> Vec<&'static str> {
        events
            .iter()
            .map(|event| match event {
                SessionEvent::Started { .. } => "started",
                SessionEvent::AudioPersisted { .. } => "audio_persisted",
                SessionEvent::Partial { .. } => "partial",
                SessionEvent::RawFinal { .. } => "raw_final",
                SessionEvent::CleanFinal { .. } => "clean_final",
                SessionEvent::Injected { .. } => "injected",
                SessionEvent::Held { .. } => "held",
                SessionEvent::Failed { .. } => "failed",
                SessionEvent::PredictionOffered { .. } => "prediction_offered",
                SessionEvent::PredictionMerged { .. } => "prediction_merged",
                SessionEvent::PredictionDismissed { .. } => "prediction_dismissed",
                SessionEvent::PredictionStale { .. } => "prediction_stale",
            })
            .collect()
    }
    fn write_ready_file(path: &Path, idle_ms: u64) -> Result<(), Error> {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let name = path
            .file_name()
            .ok_or_else(|| config(format!("{READY} must name a file")))?;
        let temporary = parent.join(format!(".{}-{}.tmp", name.to_string_lossy(), Ulid::new()));
        let encoded = serde_json::json!({
            "pid": std::process::id(), "phase": "idle", "idle_ms": idle_ms
        })
        .to_string();
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(encoded.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(temporary, path)?;
        Ok(())
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        struct CpuFallback;
        impl AsrEngine for CpuFallback {
            fn lane(&self) -> EngineLane {
                EngineLane::LocalCpu
            }
            fn transcribe(&mut self, _: &AsrRequest) -> Result<AsrTranscript, AsrError> {
                Ok(AsrTranscript::raw("fallback transcript"))
            }
        }
        #[test]
        fn pinned_lane_rejects_gpu_expected_cpu_actual() {
            let inner = EngineStack::new(vec![Box::new(CpuFallback)]);
            let mut pinned = PinnedLaneEngine(inner, EngineLane::LocalGpu);
            let id = SessionId::new(Ulid::new());
            let request = AsrRequest::new(id, wal::SAMPLE_RATE, 0, vec![0.1], Vec::new());
            let error = pinned.transcribe(&request).unwrap_err().to_string();
            assert!(error.contains("expected local_gpu, got local_cpu"));
        }
        #[rustfmt::skip]
        #[test]
        fn mixed_failed_and_raw_final_measurement_is_rejected_without_content() {
            let id = SessionId::new(Ulid::new());
            let events = vec![
                SessionEvent::Failed { id, stage: kaydence_lib::events::Stage::Recognize, error: "sensitive/model/path".into() },
                SessionEvent::RawFinal { id, text: "sensitive transcript".into() },
            ];
            let error = validate_measurement(&events).unwrap_err();
            assert_eq!(error, "reference benchmark sample contains a failed event");
        }
        #[test]
        fn ready_file_contains_only_controller_fields() {
            let directory = std::env::temp_dir().join(format!("kaydence-ready-{}", Ulid::new()));
            fs::create_dir_all(&directory).unwrap();
            let path = directory.join("ready.json");
            write_ready_file(&path, 250).unwrap();
            let value: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            let object = value.as_object().unwrap();
            assert_eq!(object.len(), 3);
            assert_eq!(object["pid"], std::process::id());
            assert_eq!(object["phase"], "idle");
            assert_eq!(object["idle_ms"], 250);
            assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
            fs::remove_dir_all(directory).unwrap();
        }
    }
}
#[test]
fn percentile_uses_nearest_rank() {
    let samples = vec![10, 20, 30, 40, 50];
    assert_eq!(nearest_rank(&samples, 50), 30);
    assert_eq!(nearest_rank(&samples, 95), 50);
}
#[test]
fn sample_count_rejects_values_below_five() {
    assert!(parse_sample_count(Some("4")).is_err());
}

#[test]
fn sha256_requires_exactly_64_hex_characters() {
    let valid = "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002";
    assert_eq!(parse_sha256(Some(valid)).unwrap(), valid);
    assert!(parse_sha256(None).is_err());
    assert!(parse_sha256(Some(&valid[..63])).is_err());
    assert!(parse_sha256(Some(&format!("{}g", &valid[..63]))).is_err());
}

#[test]
fn model_id_accepts_only_conservative_identifiers() {
    assert_eq!(
        parse_model_id(Some("ggml-base.en")).unwrap(),
        "ggml-base.en"
    );
    assert!(parse_model_id(None).is_err());
    assert!(parse_model_id(Some("")).is_err());
    assert!(parse_model_id(Some("model/path")).is_err());
    assert!(parse_model_id(Some(&"a".repeat(65))).is_err());
}

#[test]
fn idle_interval_has_a_reviewed_upper_bound() {
    assert_eq!(parse_idle_ms(Some("30000")).unwrap(), 30_000);
    assert!(parse_idle_ms(Some("30001")).is_err());
}
