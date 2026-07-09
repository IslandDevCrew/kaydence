//! Local-only latency bench contract consumed by `scripts/bench.sh`.
//!
//! This is the dependency-free floor for P1-G3. It measures the implemented Raw
//! orchestration path in process: hotkey timing, WAL-backed pipeline ordering,
//! deterministic ASR trait output, Raw commit selection, native injection policy,
//! and Light cleanup overhead. Real Parakeet/Whisper inference, GPU timings,
//! physical idle RAM/CPU, and reference-machine runs stay explicit unmeasured
//! fields until those adapters and machines are available.

use crate::audio::{
    self,
    vad::{EnergyVad, SpeechGateConfig},
    wal,
};
use crate::cleanup;
use crate::engine::{AsrEngine, AsrError, AsrRequest, AsrTranscript, EngineLane, EngineStack};
use crate::events::{CleanupDial, InjectMethod, SessionEvent, SessionId};
use crate::hotkeys::{Action, CaptureConfig, CaptureCoordinator, HotkeyMode, Signal};
use crate::inject::{
    FieldKind, InjectError, InjectorCaps, KeystrokeChannel, TextInjector, UnknownFieldPolicy,
};
use crate::pipeline::{self, TranscriptionPipeline};
use serde::Serialize;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use ulid::Ulid;

const BENCH_SCHEMA: u8 = 1;
const BENCH_FIXTURE: &str = "synthetic_raw_pipeline_v1";
const BENCH_TEXT: &str = "um ship the Kaydence bench now period";

#[derive(Debug, Clone, Serialize)]
pub struct BenchReport {
    pub schema: u8,
    pub platform: &'static str,
    pub fixture: &'static str,
    pub contract: &'static str,
    pub hotkey_to_capture_ms: u64,
    pub partial_lag_ms: u64,
    pub release_to_inject_cpu_ms: u64,
    pub release_to_inject_gpu_ms: Option<u64>,
    pub light_cleanup_added_ms: u64,
    pub prediction_guess_ms: Option<u64>,
    pub idle_ram_mb: Option<u64>,
    pub idle_cpu_pct: Option<f64>,
    pub samples_written: u64,
    pub events: Vec<&'static str>,
    pub unmeasured: Vec<&'static str>,
}

#[derive(Debug, thiserror::Error)]
pub enum BenchError {
    #[error("capture: {0}")]
    Capture(#[from] audio::CaptureRuntimeError),
    #[error("wal: {0}")]
    Wal(#[from] wal::WalError),
    #[error("pipeline: {0}")]
    Pipeline(#[from] pipeline::PipelineError),
    #[error("no committed text was produced by the bench pipeline")]
    MissingCommit,
    #[error("bench injection did not use native method: {0:?}")]
    UnexpectedInjection(SessionEvent),
    #[error("bench json: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn run_bench_json() -> Result<String, BenchError> {
    Ok(serde_json::to_string_pretty(&run_bench()?)?)
}

pub fn run_bench() -> Result<BenchReport, BenchError> {
    let (hotkey_to_capture_ms, _hotkey_session) = measure_hotkey_to_capture()?;
    let (summary, fixture_dir) = synthetic_capture_summary()?;

    let release_started = Instant::now();
    let mut pipeline = TranscriptionPipeline::new(
        EnergyVad::new(0.01),
        SpeechGateConfig::default(),
        EngineStack::new(vec![Box::new(BenchAsrEngine)]),
    )
    .with_cleanup_dial(CleanupDial::Raw);
    let events = pipeline.process_capture(&summary)?;
    let committed = pipeline::committed_text(&events).ok_or(BenchError::MissingCommit)?;
    let mut injector = BenchInjector;
    let injection = crate::inject::inject_committed_text(
        &mut injector,
        committed.id,
        &committed.text,
        UnknownFieldPolicy::Strict,
        false,
    );
    match injection {
        SessionEvent::Injected {
            method: InjectMethod::Native,
            ..
        } => {}
        other => return Err(BenchError::UnexpectedInjection(other)),
    }
    let release_to_inject_cpu_ms = elapsed_ms(release_started.elapsed());

    let partial_lag_ms = events
        .iter()
        .filter_map(|event| match event {
            SessionEvent::Partial { t_lag_ms, .. } => Some(u64::from(*t_lag_ms)),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    let light_cleanup_added_ms = measure_light_cleanup_added();
    let event_names = event_names(&events);

    let _ = std::fs::remove_dir_all(fixture_dir);
    Ok(BenchReport {
        schema: BENCH_SCHEMA,
        platform: std::env::consts::OS,
        fixture: BENCH_FIXTURE,
        contract: "implemented_raw_path; real_asr_gpu_idle_footprint_pending",
        hotkey_to_capture_ms,
        partial_lag_ms,
        release_to_inject_cpu_ms,
        release_to_inject_gpu_ms: None,
        light_cleanup_added_ms,
        prediction_guess_ms: None,
        idle_ram_mb: None,
        idle_cpu_pct: None,
        samples_written: summary.samples_written,
        events: event_names,
        unmeasured: vec![
            "release_to_inject_gpu_ms",
            "prediction_guess_ms",
            "idle_ram_mb",
            "idle_cpu_pct",
            "real_parakeet_whisper_asr",
            "reference_machine_p95",
        ],
    })
}

fn measure_hotkey_to_capture() -> Result<(u64, SessionId), BenchError> {
    let dir = temp_dir("hotkey");
    let mut coordinator = CaptureCoordinator::new(HotkeyMode::PushToTalk, CaptureConfig::default());
    let mut recorder = audio::WalCaptureRuntime::new_wal_only(&dir);
    let started = Instant::now();
    let action = coordinator.step(Signal::Press { at_ms: 0 });
    let id = match action {
        Action::StartCapture => recorder.start_capture(0)?,
        _ => return Err(BenchError::MissingCommit),
    };
    let elapsed = elapsed_ms(started.elapsed());
    let _ = recorder.discard_capture(1);
    let _ = std::fs::remove_dir_all(dir);
    Ok((elapsed, id))
}

fn synthetic_capture_summary() -> Result<(audio::CaptureSessionSummary, PathBuf), BenchError> {
    let dir = temp_dir("pipeline");
    let sessions_dir = dir.join("sessions");
    let id = SessionId::new(Ulid::new());
    let mut writer = wal::WalWriter::create(&sessions_dir, &id.0.to_string())?;
    let samples = speech_samples(600);
    writer.append(&samples)?;
    let wal_path = writer.finalize()?;
    Ok((
        audio::CaptureSessionSummary {
            id,
            wal_path,
            samples_written: samples.len() as u64,
            dropped_input_samples: 0,
            started_ms: 0,
            finalized_ms: 600,
        },
        dir,
    ))
}

fn speech_samples(duration_ms: u64) -> Vec<f32> {
    let sample_count = (u64::from(wal::SAMPLE_RATE) * duration_ms / 1_000) as usize;
    (0..sample_count)
        .map(|sample| {
            let phase = (sample % 64) as f32 / 64.0;
            (phase * std::f32::consts::TAU).sin() * 0.18
        })
        .collect()
}

fn measure_light_cleanup_added() -> u64 {
    let input = std::hint::black_box(BENCH_TEXT);
    let raw_started = Instant::now();
    let raw = cleanup::clean_text(input, CleanupDial::Raw);
    let raw_ms = elapsed_ms(raw_started.elapsed());

    let light_started = Instant::now();
    let light = cleanup::clean_text(input, CleanupDial::Light);
    let light_ms = elapsed_ms(light_started.elapsed());

    // Use the results so optimized builds still preserve the two transformations.
    std::hint::black_box(&raw);
    std::hint::black_box(&light);
    debug_assert!(raw.text.starts_with("um "));
    debug_assert!(light.text.starts_with("Ship"));
    light_ms.saturating_sub(raw_ms)
}

fn elapsed_ms(duration: Duration) -> u64 {
    let micros = duration.as_micros();
    if micros == 0 {
        0
    } else {
        micros.div_ceil(1_000) as u64
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

fn temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "kaydence-bench-{label}-{}-{}",
        std::process::id(),
        Ulid::new()
    ))
}

struct BenchAsrEngine;

impl AsrEngine for BenchAsrEngine {
    fn lane(&self) -> EngineLane {
        EngineLane::LocalCpu
    }

    fn transcribe(&mut self, request: &AsrRequest) -> Result<AsrTranscript, AsrError> {
        if request.samples.is_empty() {
            return Err(AsrError::EmptyTranscript);
        }
        Ok(AsrTranscript {
            partials: vec![crate::engine::PartialTranscript {
                text: "um ship".to_string(),
                t_lag_ms: 120,
            }],
            final_text: BENCH_TEXT.to_string(),
            no_speech_probability: Some(0.01),
        })
    }
}

struct BenchInjector;

impl TextInjector for BenchInjector {
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
            "bench injector only exposes native insert".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn bench_report_measures_implemented_raw_budget_fields() {
        let report = run_bench().expect("bench report");

        assert_eq!(report.schema, BENCH_SCHEMA);
        assert_eq!(report.partial_lag_ms, 120);
        assert!(report.hotkey_to_capture_ms > 0);
        assert!(report.release_to_inject_cpu_ms > 0);
        assert!(report.light_cleanup_added_ms <= 800);
        assert_eq!(
            report.events,
            vec!["audio_persisted", "partial", "raw_final"]
        );
        assert!(report.release_to_inject_gpu_ms.is_none());
        assert!(report.idle_ram_mb.is_none());
    }

    #[test]
    fn bench_json_is_machine_readable_for_script_gate() {
        let json = run_bench_json().expect("bench json");
        let decoded: Value = serde_json::from_str(&json).expect("valid json");

        assert_eq!(decoded["schema"], BENCH_SCHEMA);
        assert!(decoded["hotkey_to_capture_ms"].as_u64().is_some());
        assert!(decoded["release_to_inject_cpu_ms"].as_u64().is_some());
        assert!(decoded["release_to_inject_gpu_ms"].is_null());
        assert!(decoded["unmeasured"]
            .as_array()
            .expect("unmeasured array")
            .iter()
            .any(|value| value == "real_parakeet_whisper_asr"));
    }
}
