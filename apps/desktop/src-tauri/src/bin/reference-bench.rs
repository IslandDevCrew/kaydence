#[cfg(feature = "asr-whisper-metal")]
use kaydence_lib::audio::{self, vad::EnergyVad, vad::SpeechGateConfig, wal};
#[cfg(feature = "asr-whisper-metal")]
use kaydence_lib::engine::{
    local_asr_stack, AsrError, EngineLane, LocalAsrAdapterSpec, LocalAsrAdapterState,
};
#[cfg(feature = "asr-whisper-metal")]
use kaydence_lib::events::{CleanupDial, InjectMethod, SessionEvent, SessionId};
#[cfg(feature = "asr-whisper-metal")]
use kaydence_lib::inject::{
    inject_committed_text, FieldKind, InjectError, InjectorCaps, KeystrokeChannel, TextInjector,
    UnknownFieldPolicy,
};
#[cfg(feature = "asr-whisper-metal")]
use kaydence_lib::pipeline::{self, TranscriptionPipeline};
#[cfg(feature = "asr-whisper-metal")]
use serde::Serialize;
#[cfg(feature = "asr-whisper-metal")]
use std::fs::{self, OpenOptions};
#[cfg(feature = "asr-whisper-metal")]
use std::io::Write;
#[cfg(feature = "asr-whisper-metal")]
use std::path::{Path, PathBuf};
#[cfg(feature = "asr-whisper-metal")]
use std::time::{Duration, Instant};
#[cfg(feature = "asr-whisper-metal")]
use ulid::Ulid;

const DEFAULT_SAMPLE_COUNT: u64 = 10;
const MINIMUM_SAMPLE_COUNT: u64 = 5;

fn nearest_rank(samples: &[u64], percentile: u8) -> u64 {
    assert!(
        !samples.is_empty(),
        "percentile requires at least one sample"
    );
    assert!(
        (1..=100).contains(&percentile),
        "percentile must be in 1..=100"
    );

    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let rank = (usize::from(percentile) * sorted.len()).div_ceil(100);
    sorted[rank - 1]
}

fn parse_sample_count(value: Option<&str>) -> Result<u64, String> {
    let sample_count = match value {
        Some(value) => value
            .parse::<u64>()
            .map_err(|_| format!("KAYDENCE_REFERENCE_SAMPLES must be an integer, got {value:?}"))?,
        None => DEFAULT_SAMPLE_COUNT,
    };

    if sample_count < MINIMUM_SAMPLE_COUNT {
        return Err(format!(
            "KAYDENCE_REFERENCE_SAMPLES must be at least {MINIMUM_SAMPLE_COUNT}, got {sample_count}"
        ));
    }

    Ok(sample_count)
}

#[cfg(feature = "asr-whisper-metal")]
const MODEL_ENV: &str = "KAYDENCE_WHISPER_MODEL";
#[cfg(feature = "asr-whisper-metal")]
const CLIP_ENV: &str = "KAYDENCE_WHISPER_CLIP";
#[cfg(feature = "asr-whisper-metal")]
const LANE_ENV: &str = "KAYDENCE_WHISPER_LANE";
#[cfg(feature = "asr-whisper-metal")]
const IDLE_ENV: &str = "KAYDENCE_REFERENCE_IDLE_MS";
#[cfg(feature = "asr-whisper-metal")]
const READY_FILE_ENV: &str = "KAYDENCE_REFERENCE_READY_FILE";

#[cfg(feature = "asr-whisper-metal")]
#[derive(Debug, thiserror::Error)]
enum ProbeError {
    #[error("{0}")]
    Configuration(String),
    #[error("audio WAL: {0}")]
    Wal(#[from] wal::WalError),
    #[error("ASR warmup: {0}")]
    Asr(#[from] AsrError),
    #[error("pipeline: {0}")]
    Pipeline(#[from] pipeline::PipelineError),
    #[error("file IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the ASR pipeline produced no committed text")]
    MissingCommit,
    #[error("policy delivery did not use the native path: {0:?}")]
    UnexpectedDelivery(SessionEvent),
}

#[cfg(feature = "asr-whisper-metal")]
#[derive(Debug)]
struct ProbeConfig {
    model_path: PathBuf,
    clip_path: PathBuf,
    lane: EngineLane,
    sample_count: u64,
    idle_ms: u64,
    ready_file: Option<PathBuf>,
}

#[cfg(feature = "asr-whisper-metal")]
impl ProbeConfig {
    fn from_environment() -> Result<Self, ProbeError> {
        let model_path = required_path(MODEL_ENV)?;
        let clip_path = required_path(CLIP_ENV)?;
        let lane = match std::env::var(LANE_ENV).as_deref() {
            Ok("gpu") | Err(_) => EngineLane::LocalGpu,
            Ok("cpu") => EngineLane::LocalCpu,
            Ok(value) => {
                return Err(ProbeError::Configuration(format!(
                    "{LANE_ENV} must be gpu or cpu, got {value:?}"
                )));
            }
        };
        let sample_count =
            parse_sample_count(std::env::var("KAYDENCE_REFERENCE_SAMPLES").ok().as_deref())
                .map_err(ProbeError::Configuration)?;
        let idle_ms = optional_u64(IDLE_ENV)?.unwrap_or(0);
        let ready_file = std::env::var_os(READY_FILE_ENV).map(PathBuf::from);

        Ok(Self {
            model_path,
            clip_path,
            lane,
            sample_count,
            idle_ms,
            ready_file,
        })
    }
}

#[cfg(feature = "asr-whisper-metal")]
#[derive(Debug, Serialize)]
struct ReferenceBenchReport {
    schema: u8,
    platform: &'static str,
    lane: &'static str,
    warmup_ms: u64,
    sample_count: u64,
    release_to_delivery_policy_p50_ms: u64,
    release_to_delivery_policy_p95_ms: u64,
    audio_ms: u64,
    transcript_nonempty: bool,
    events: Vec<&'static str>,
    unmeasured: Vec<&'static str>,
}

#[cfg(feature = "asr-whisper-metal")]
#[derive(Debug, Serialize)]
struct ReadySignal {
    pid: u32,
    phase: &'static str,
    idle_ms: u64,
}

#[cfg(feature = "asr-whisper-metal")]
struct CaptureFixture {
    summary: audio::CaptureSessionSummary,
    directory: PathBuf,
}

#[cfg(feature = "asr-whisper-metal")]
impl Drop for CaptureFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[cfg(feature = "asr-whisper-metal")]
struct PolicySink;

#[cfg(feature = "asr-whisper-metal")]
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
        Err(InjectError(
            "policy sink does not synthesize text".to_string(),
        ))
    }
}

#[cfg(feature = "asr-whisper-metal")]
fn main() {
    match run_reference_bench() {
        Ok(report) => match serde_json::to_string(&report) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("reference-bench: {error}");
                std::process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("reference-bench: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(feature = "asr-whisper-metal"))]
fn main() {
    eprintln!("reference-bench requires --features asr-whisper-metal");
    std::process::exit(2);
}

#[cfg(feature = "asr-whisper-metal")]
fn run_reference_bench() -> Result<ReferenceBenchReport, ProbeError> {
    let config = ProbeConfig::from_environment()?;
    let samples = read_pcm16_mono_wav(&config.clip_path)?;
    let audio_ms = (samples.len() as u64 * 1_000) / u64::from(wal::SAMPLE_RATE);
    let fixture = create_capture_fixture(&samples)?;
    let spec = LocalAsrAdapterSpec {
        model_id: "reference-bench".to_string(),
        lane: config.lane,
        runtime: "whisper.cpp".to_string(),
        artifact_size_bytes: fs::metadata(&config.model_path)?.len(),
        artifact_path: config.model_path,
    };
    let mut stack = local_asr_stack(LocalAsrAdapterState::VerifiedArtifact { spec });

    let warmup_started = Instant::now();
    let warmed_lane = stack.warm_up()?;
    let warmup_ms = elapsed_ms(warmup_started.elapsed());
    let warmed_lane = warmed_lane.ok_or_else(|| {
        ProbeError::Configuration("the selected ASR stack did not warm a lane".to_string())
    })?;

    let mut pipeline =
        TranscriptionPipeline::new(EnergyVad::new(0.01), SpeechGateConfig::default(), stack)
            .with_cleanup_dial(CleanupDial::Raw);
    let mut samples_ms = Vec::with_capacity(config.sample_count as usize);
    let mut events = Vec::new();

    for _ in 0..config.sample_count {
        let started = Instant::now();
        let measured_events = pipeline.process_capture(&fixture.summary)?;
        let committed =
            pipeline::committed_text(&measured_events).ok_or(ProbeError::MissingCommit)?;
        let delivered = inject_committed_text(
            &mut PolicySink,
            committed.id,
            &committed.text,
            UnknownFieldPolicy::Strict,
            false,
        );
        require_native_delivery(delivered)?;
        samples_ms.push(elapsed_ms(started.elapsed()));
        events = measured_events;
    }

    let report = ReferenceBenchReport {
        schema: 1,
        platform: std::env::consts::OS,
        lane: lane_label(warmed_lane),
        warmup_ms,
        sample_count: config.sample_count,
        release_to_delivery_policy_p50_ms: nearest_rank(&samples_ms, 50),
        release_to_delivery_policy_p95_ms: nearest_rank(&samples_ms, 95),
        audio_ms,
        transcript_nonempty: true,
        events: event_names(&events),
        unmeasured: vec!["physical_os_field_injection"],
    };

    if let Some(ready_file) = config.ready_file.as_deref() {
        write_ready_file(ready_file, config.idle_ms)?;
    }
    std::thread::sleep(Duration::from_millis(config.idle_ms));
    std::hint::black_box(&pipeline);

    Ok(report)
}

#[cfg(feature = "asr-whisper-metal")]
fn required_path(name: &str) -> Result<PathBuf, ProbeError> {
    let value = std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ProbeError::Configuration(format!("{name} must be set")))?;
    let path = PathBuf::from(value);
    if !path.is_file() {
        return Err(ProbeError::Configuration(format!(
            "{name} is not a file: {}",
            path.display()
        )));
    }
    Ok(path)
}

#[cfg(feature = "asr-whisper-metal")]
fn optional_u64(name: &str) -> Result<Option<u64>, ProbeError> {
    match std::env::var(name) {
        Ok(value) => value.parse::<u64>().map(Some).map_err(|_| {
            ProbeError::Configuration(format!("{name} must be an integer, got {value:?}"))
        }),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(ProbeError::Configuration(format!(
            "{name} must be valid Unicode"
        ))),
    }
}

#[cfg(feature = "asr-whisper-metal")]
fn read_pcm16_mono_wav(path: &Path) -> Result<Vec<f32>, ProbeError> {
    let bytes = fs::read(path)?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(ProbeError::Configuration(format!(
            "{} is not a RIFF/WAVE file",
            path.display()
        )));
    }

    let mut rate = None;
    let mut pcm = None;
    let mut channels = None;
    let mut bits = None;
    let mut data = None;
    let mut position = 12;
    while position + 8 <= bytes.len() {
        let chunk_id = &bytes[position..position + 4];
        let chunk_size =
            u32::from_le_bytes(bytes[position + 4..position + 8].try_into().map_err(|_| {
                ProbeError::Configuration(format!("{} has an invalid chunk length", path.display()))
            })?) as usize;
        let body = position + 8;
        let end = body.checked_add(chunk_size).ok_or_else(|| {
            ProbeError::Configuration(format!("{} has an oversized WAV chunk", path.display()))
        })?;
        if end > bytes.len() {
            return Err(ProbeError::Configuration(format!(
                "{} has a truncated WAV chunk",
                path.display()
            )));
        }

        match chunk_id {
            b"fmt " if chunk_size >= 16 => {
                pcm = Some(u16::from_le_bytes([bytes[body], bytes[body + 1]]));
                channels = Some(u16::from_le_bytes([bytes[body + 2], bytes[body + 3]]));
                rate = Some(u32::from_le_bytes([
                    bytes[body + 4],
                    bytes[body + 5],
                    bytes[body + 6],
                    bytes[body + 7],
                ]));
                bits = Some(u16::from_le_bytes([bytes[body + 14], bytes[body + 15]]));
            }
            b"fmt " => {
                return Err(ProbeError::Configuration(format!(
                    "{} has a truncated fmt chunk",
                    path.display()
                )));
            }
            b"data" => data = Some(&bytes[body..end]),
            _ => {}
        }

        position = end + (chunk_size & 1);
    }

    if pcm != Some(1) || channels != Some(1) || rate != Some(wal::SAMPLE_RATE) || bits != Some(16) {
        return Err(ProbeError::Configuration(format!(
            "{} must be PCM16 mono at {} Hz",
            path.display(),
            wal::SAMPLE_RATE
        )));
    }
    let data = data.ok_or_else(|| {
        ProbeError::Configuration(format!("{} has no data chunk", path.display()))
    })?;
    if data.len() % 2 != 0 {
        return Err(ProbeError::Configuration(format!(
            "{} has an odd PCM16 data length",
            path.display()
        )));
    }
    let samples: Vec<f32> = data
        .chunks_exact(2)
        .map(|sample| i16::from_le_bytes([sample[0], sample[1]]) as f32 / i16::MAX as f32)
        .collect();
    if samples.is_empty() {
        return Err(ProbeError::Configuration(format!(
            "{} contains no audio samples",
            path.display()
        )));
    }
    Ok(samples)
}

#[cfg(feature = "asr-whisper-metal")]
fn create_capture_fixture(samples: &[f32]) -> Result<CaptureFixture, ProbeError> {
    let directory = std::env::temp_dir().join(format!(
        "kaydence-reference-bench-{}-{}",
        std::process::id(),
        Ulid::new()
    ));
    let sessions_directory = directory.join("sessions");
    let id = SessionId::new(Ulid::new());
    let mut writer = wal::WalWriter::create(&sessions_directory, &id.0.to_string())?;
    writer.append(samples)?;
    let wal_path = writer.finalize()?;
    let finalized_ms = (samples.len() as u64 * 1_000) / u64::from(wal::SAMPLE_RATE);

    Ok(CaptureFixture {
        summary: audio::CaptureSessionSummary {
            id,
            wal_path,
            samples_written: samples.len() as u64,
            dropped_input_samples: 0,
            started_ms: 0,
            finalized_ms,
        },
        directory,
    })
}

#[cfg(feature = "asr-whisper-metal")]
fn require_native_delivery(delivered: SessionEvent) -> Result<(), ProbeError> {
    match delivered {
        SessionEvent::Injected {
            method: InjectMethod::Native,
            ..
        } => Ok(()),
        other => Err(ProbeError::UnexpectedDelivery(other)),
    }
}

#[cfg(feature = "asr-whisper-metal")]
fn elapsed_ms(duration: Duration) -> u64 {
    let micros = duration.as_micros();
    if micros == 0 {
        0
    } else {
        micros.div_ceil(1_000) as u64
    }
}

#[cfg(feature = "asr-whisper-metal")]
fn lane_label(lane: EngineLane) -> &'static str {
    match lane {
        EngineLane::ByokCloud => "byok_cloud",
        EngineLane::LocalGpu => "local_gpu",
        EngineLane::LocalCpu => "local_cpu",
    }
}

#[cfg(feature = "asr-whisper-metal")]
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

#[cfg(feature = "asr-whisper-metal")]
fn write_ready_file(path: &Path, idle_ms: u64) -> Result<(), ProbeError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| ProbeError::Configuration(format!("{READY_FILE_ENV} must name a file")))?;
    let temporary_path = parent.join(format!(
        ".{}-{}-{}.tmp",
        file_name.to_string_lossy(),
        std::process::id(),
        Ulid::new()
    ));
    let signal = ReadySignal {
        pid: std::process::id(),
        phase: "idle",
        idle_ms,
    };
    let encoded = serde_json::to_vec(&signal)?;

    let mut temporary = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)?;
    temporary.write_all(&encoded)?;
    temporary.sync_all()?;
    drop(temporary);
    fs::rename(&temporary_path, path)?;

    Ok(())
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
