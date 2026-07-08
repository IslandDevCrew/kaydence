//! Capture: mic -> lock-free ring buffer -> WAL file -> Silero VAD. Emits
//! AudioPersisted (non-negotiable #2).
//!
//! `wal` is real (P1, PRD P0-4): write-ahead persistence + crash recovery.
//! cpal capture now feeds a lock-free ring buffer; the drain thread owns WAL
//! writes. VAD and the rubato fallback for devices that cannot open 16 kHz PCM
//! land next.
#![allow(dead_code)]

pub mod wal;

use crate::events::{SessionEvent, SessionId};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapCons, HeapProd, HeapRb,
};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use ulid::Ulid;

const INPUT_RING_SECONDS: usize = 2;
const DRAIN_CHUNK_SAMPLES: usize = 1024;
const DRAIN_IDLE_SLEEP: Duration = Duration::from_millis(5);

/// Metadata for a capture whose WAL was finalized and is ready for downstream
/// recognition/history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureSessionSummary {
    pub id: SessionId,
    pub wal_path: PathBuf,
    pub samples_written: u64,
    pub dropped_input_samples: u64,
    pub started_ms: u64,
    pub finalized_ms: u64,
}

impl CaptureSessionSummary {
    pub fn audio_persisted_event(&self) -> SessionEvent {
        SessionEvent::AudioPersisted {
            id: self.id,
            wal_path: self.wal_path.display().to_string(),
        }
    }
}

/// Metadata for a short/accidental capture that was discarded before ASR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscardedCapture {
    pub id: SessionId,
    pub wal_path: PathBuf,
    pub started_ms: u64,
    pub discarded_ms: u64,
    pub dropped_input_samples: u64,
    pub removed: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum CaptureRuntimeError {
    #[error("capture session already active: {0:?}")]
    AlreadyActive(SessionId),
    #[error("no active capture session")]
    NoActiveSession,
    #[error("wal: {0}")]
    Wal(#[from] wal::WalError),
    #[error("capture file io: {0}")]
    Io(#[from] std::io::Error),
    #[error("no default input device")]
    NoInputDevice,
    #[error("no supported input config can produce {0} Hz PCM")]
    NoSupportedInputConfig(u32),
    #[error("unsupported input sample format: {0}")]
    UnsupportedSampleFormat(cpal::SampleFormat),
    #[error("input config: {0}")]
    InputConfig(#[from] cpal::SupportedStreamConfigsError),
    #[error("build input stream: {0}")]
    BuildInputStream(#[from] cpal::BuildStreamError),
    #[error("play input stream: {0}")]
    PlayInputStream(#[from] cpal::PlayStreamError),
    #[error("input drain thread panicked")]
    DrainThreadPanicked,
}

struct ActiveWalSession {
    id: SessionId,
    input: ActiveCaptureInput,
    started_ms: u64,
}

enum ActiveCaptureInput {
    WalOnly(wal::WalWriter),
    Mic(MicCaptureSession),
}

impl ActiveCaptureInput {
    fn stop(self) -> Result<StoppedCaptureInput, CaptureRuntimeError> {
        match self {
            Self::WalOnly(writer) => Ok(StoppedCaptureInput {
                writer,
                dropped_input_samples: 0,
            }),
            Self::Mic(session) => session.stop_and_reclaim(),
        }
    }
}

struct StoppedCaptureInput {
    writer: wal::WalWriter,
    dropped_input_samples: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureInputMode {
    DefaultMic,
    WalOnly,
}

struct MicCaptureSession {
    stream: Option<cpal::Stream>,
    stop: Arc<AtomicBool>,
    dropped_input_samples: Arc<AtomicU64>,
    drain_thread: Option<JoinHandle<Result<wal::WalWriter, wal::WalError>>>,
}

impl MicCaptureSession {
    fn start(
        prepared: PreparedMicInput,
        writer: wal::WalWriter,
    ) -> Result<Self, CaptureRuntimeError> {
        let stop = Arc::new(AtomicBool::new(false));
        let dropped_input_samples = Arc::clone(&prepared.dropped_input_samples);
        let drain_thread = spawn_drain_thread(prepared.consumer, writer, Arc::clone(&stop));
        let session = Self {
            stream: Some(prepared.stream),
            stop,
            dropped_input_samples,
            drain_thread: Some(drain_thread),
        };

        let Some(stream) = session.stream.as_ref() else {
            return Err(CaptureRuntimeError::DrainThreadPanicked);
        };
        if let Err(err) = stream.play() {
            if let Ok(stopped) = session.stop_and_reclaim() {
                let path = stopped.writer.path().to_path_buf();
                drop(stopped.writer);
                let _ = std::fs::remove_file(path);
            }
            return Err(CaptureRuntimeError::PlayInputStream(err));
        }

        Ok(session)
    }

    fn stop_and_reclaim(mut self) -> Result<StoppedCaptureInput, CaptureRuntimeError> {
        drop(self.stream.take());
        self.stop.store(true, Ordering::Release);
        let writer = self
            .drain_thread
            .take()
            .ok_or(CaptureRuntimeError::DrainThreadPanicked)?
            .join()
            .map_err(|_| CaptureRuntimeError::DrainThreadPanicked)??;
        Ok(StoppedCaptureInput {
            writer,
            dropped_input_samples: self.dropped_input_samples.load(Ordering::Relaxed),
        })
    }
}

struct PreparedMicInput {
    stream: cpal::Stream,
    consumer: HeapCons<f32>,
    dropped_input_samples: Arc<AtomicU64>,
}

impl PreparedMicInput {
    fn new() -> Result<Self, CaptureRuntimeError> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(CaptureRuntimeError::NoInputDevice)?;
        let supported = choose_input_config(&device)?;
        let sample_format = supported.sample_format();
        let config = supported.config();
        let channels = usize::from(config.channels);
        let rb = HeapRb::<f32>::new((wal::SAMPLE_RATE as usize) * INPUT_RING_SECONDS);
        let (producer, consumer) = rb.split();
        let dropped_input_samples = Arc::new(AtomicU64::new(0));
        let stream = build_input_stream(
            &device,
            &config,
            sample_format,
            producer,
            Arc::clone(&dropped_input_samples),
            channels,
        )?;

        Ok(Self {
            stream,
            consumer,
            dropped_input_samples,
        })
    }
}

/// Minimal runtime owner for P1 hotkey capture: one active dictation maps to one
/// app-data `sessions/{ulid}.wav` WAL. Production mode starts a cpal input stream
/// whose realtime callback only downmixes into a lock-free ring; the drain thread
/// owns WAL writes.
pub struct WalCaptureRuntime {
    sessions_dir: PathBuf,
    active: Option<ActiveWalSession>,
    input_mode: CaptureInputMode,
}

impl WalCaptureRuntime {
    pub fn new(app_data_dir: impl Into<PathBuf>) -> Self {
        Self::with_input_mode(app_data_dir, CaptureInputMode::DefaultMic)
    }

    pub fn new_wal_only(app_data_dir: impl Into<PathBuf>) -> Self {
        Self::with_input_mode(app_data_dir, CaptureInputMode::WalOnly)
    }

    fn with_input_mode(app_data_dir: impl Into<PathBuf>, input_mode: CaptureInputMode) -> Self {
        Self {
            sessions_dir: app_data_dir.into().join("sessions"),
            active: None,
            input_mode,
        }
    }

    pub fn sessions_dir(&self) -> &Path {
        &self.sessions_dir
    }

    pub fn active_session_id(&self) -> Option<SessionId> {
        self.active.as_ref().map(|session| session.id)
    }

    pub fn start_capture(&mut self, at_ms: u64) -> Result<SessionId, CaptureRuntimeError> {
        if let Some(active) = &self.active {
            return Err(CaptureRuntimeError::AlreadyActive(active.id));
        }

        let prepared_mic = match self.input_mode {
            CaptureInputMode::DefaultMic => Some(PreparedMicInput::new()?),
            CaptureInputMode::WalOnly => None,
        };
        let id = SessionId::new(Ulid::new());
        let writer = wal::WalWriter::create(&self.sessions_dir, &id.0.to_string())?;
        let input = match prepared_mic {
            Some(prepared) => ActiveCaptureInput::Mic(MicCaptureSession::start(prepared, writer)?),
            None => ActiveCaptureInput::WalOnly(writer),
        };
        self.active = Some(ActiveWalSession {
            id,
            input,
            started_ms: at_ms,
        });
        Ok(id)
    }

    pub fn finalize_capture(
        &mut self,
        finalized_ms: u64,
    ) -> Result<CaptureSessionSummary, CaptureRuntimeError> {
        let Some(active) = self.active.take() else {
            return Err(CaptureRuntimeError::NoActiveSession);
        };

        let stopped = active.input.stop()?;
        let samples_written = stopped.writer.samples_written();
        let dropped_input_samples = stopped.dropped_input_samples;
        let wal_path = stopped.writer.finalize()?;
        Ok(CaptureSessionSummary {
            id: active.id,
            wal_path,
            samples_written,
            dropped_input_samples,
            started_ms: active.started_ms,
            finalized_ms,
        })
    }

    pub fn discard_capture(
        &mut self,
        discarded_ms: u64,
    ) -> Result<DiscardedCapture, CaptureRuntimeError> {
        let Some(active) = self.active.take() else {
            return Err(CaptureRuntimeError::NoActiveSession);
        };

        let stopped = active.input.stop()?;
        let wal_path = stopped.writer.path().to_path_buf();
        let dropped_input_samples = stopped.dropped_input_samples;
        drop(stopped.writer);
        let removed = match std::fs::remove_file(&wal_path) {
            Ok(()) => true,
            Err(err) if err.kind() == ErrorKind::NotFound => false,
            Err(err) => return Err(CaptureRuntimeError::Io(err)),
        };

        Ok(DiscardedCapture {
            id: active.id,
            wal_path,
            started_ms: active.started_ms,
            discarded_ms,
            dropped_input_samples,
            removed,
        })
    }
}

trait CaptureSample {
    fn to_capture_f32(self) -> f32;
}

impl CaptureSample for f32 {
    fn to_capture_f32(self) -> f32 {
        self.clamp(-1.0, 1.0)
    }
}

impl CaptureSample for f64 {
    fn to_capture_f32(self) -> f32 {
        (self as f32).clamp(-1.0, 1.0)
    }
}

impl CaptureSample for i8 {
    fn to_capture_f32(self) -> f32 {
        self as f32 / i8::MAX as f32
    }
}

impl CaptureSample for i16 {
    fn to_capture_f32(self) -> f32 {
        self as f32 / i16::MAX as f32
    }
}

impl CaptureSample for i32 {
    fn to_capture_f32(self) -> f32 {
        self as f32 / i32::MAX as f32
    }
}

impl CaptureSample for i64 {
    fn to_capture_f32(self) -> f32 {
        self as f32 / i64::MAX as f32
    }
}

impl CaptureSample for u8 {
    fn to_capture_f32(self) -> f32 {
        (self as f32 - 128.0) / 128.0
    }
}

impl CaptureSample for u16 {
    fn to_capture_f32(self) -> f32 {
        (self as f32 - 32_768.0) / 32_768.0
    }
}

impl CaptureSample for u32 {
    fn to_capture_f32(self) -> f32 {
        (self as f32 - 2_147_483_648.0) / 2_147_483_648.0
    }
}

impl CaptureSample for u64 {
    fn to_capture_f32(self) -> f32 {
        (self as f64 - 9_223_372_036_854_775_808.0) as f32 / 9_223_372_036_854_775_808.0_f32
    }
}

fn choose_input_config(
    device: &cpal::Device,
) -> Result<cpal::SupportedStreamConfig, CaptureRuntimeError> {
    let target_rate = wal::SAMPLE_RATE;
    let mut candidates = device
        .supported_input_configs()?
        .filter(|range| supported_pcm_sample_format(range.sample_format()))
        .filter_map(|range| range.try_with_sample_rate(target_rate))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|config| {
        (
            config.channels() != 1,
            sample_format_priority(config.sample_format()),
            config.channels(),
        )
    });
    candidates
        .into_iter()
        .next()
        .ok_or(CaptureRuntimeError::NoSupportedInputConfig(
            wal::SAMPLE_RATE,
        ))
}

fn supported_pcm_sample_format(sample_format: cpal::SampleFormat) -> bool {
    matches!(
        sample_format,
        cpal::SampleFormat::F32
            | cpal::SampleFormat::F64
            | cpal::SampleFormat::I8
            | cpal::SampleFormat::I16
            | cpal::SampleFormat::I32
            | cpal::SampleFormat::I64
            | cpal::SampleFormat::U8
            | cpal::SampleFormat::U16
            | cpal::SampleFormat::U32
            | cpal::SampleFormat::U64
    )
}

fn sample_format_priority(sample_format: cpal::SampleFormat) -> u8 {
    match sample_format {
        cpal::SampleFormat::F32 => 0,
        cpal::SampleFormat::I16 => 1,
        cpal::SampleFormat::F64 => 2,
        cpal::SampleFormat::I32 => 3,
        cpal::SampleFormat::U16 => 4,
        cpal::SampleFormat::U32 => 5,
        cpal::SampleFormat::I8 => 6,
        cpal::SampleFormat::U8 => 7,
        cpal::SampleFormat::I64 => 8,
        cpal::SampleFormat::U64 => 9,
        _ => 10,
    }
}

fn build_input_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    producer: HeapProd<f32>,
    dropped_input_samples: Arc<AtomicU64>,
    channels: usize,
) -> Result<cpal::Stream, CaptureRuntimeError> {
    match sample_format {
        cpal::SampleFormat::F32 => {
            build_input_stream_for::<f32>(device, config, producer, dropped_input_samples, channels)
        }
        cpal::SampleFormat::F64 => {
            build_input_stream_for::<f64>(device, config, producer, dropped_input_samples, channels)
        }
        cpal::SampleFormat::I8 => {
            build_input_stream_for::<i8>(device, config, producer, dropped_input_samples, channels)
        }
        cpal::SampleFormat::I16 => {
            build_input_stream_for::<i16>(device, config, producer, dropped_input_samples, channels)
        }
        cpal::SampleFormat::I32 => {
            build_input_stream_for::<i32>(device, config, producer, dropped_input_samples, channels)
        }
        cpal::SampleFormat::I64 => {
            build_input_stream_for::<i64>(device, config, producer, dropped_input_samples, channels)
        }
        cpal::SampleFormat::U8 => {
            build_input_stream_for::<u8>(device, config, producer, dropped_input_samples, channels)
        }
        cpal::SampleFormat::U16 => {
            build_input_stream_for::<u16>(device, config, producer, dropped_input_samples, channels)
        }
        cpal::SampleFormat::U32 => {
            build_input_stream_for::<u32>(device, config, producer, dropped_input_samples, channels)
        }
        cpal::SampleFormat::U64 => {
            build_input_stream_for::<u64>(device, config, producer, dropped_input_samples, channels)
        }
        sample_format => Err(CaptureRuntimeError::UnsupportedSampleFormat(sample_format)),
    }
}

fn build_input_stream_for<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut producer: HeapProd<f32>,
    dropped_input_samples: Arc<AtomicU64>,
    channels: usize,
) -> Result<cpal::Stream, CaptureRuntimeError>
where
    T: cpal::SizedSample + CaptureSample + Copy + Send + 'static,
{
    let err_fn = |err| eprintln!("Kaydence audio input stream error: {err}");
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| {
            push_interleaved_mono(data, channels, &mut producer, &dropped_input_samples);
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}

fn push_interleaved_mono<T>(
    data: &[T],
    channels: usize,
    producer: &mut HeapProd<f32>,
    dropped_input_samples: &AtomicU64,
) where
    T: CaptureSample + Copy,
{
    if channels == 0 {
        return;
    }

    for frame in data.chunks(channels) {
        let mut sum = 0.0;
        for sample in frame {
            sum += sample.to_capture_f32();
        }
        let mono = sum / frame.len() as f32;
        if producer.try_push(mono).is_err() {
            dropped_input_samples.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn spawn_drain_thread(
    mut consumer: HeapCons<f32>,
    mut writer: wal::WalWriter,
    stop: Arc<AtomicBool>,
) -> JoinHandle<Result<wal::WalWriter, wal::WalError>> {
    thread::spawn(move || {
        let mut scratch = vec![0.0; DRAIN_CHUNK_SAMPLES];
        loop {
            let n = consumer.pop_slice(&mut scratch);
            if n > 0 {
                writer.append(&scratch[..n])?;
                continue;
            }

            if stop.load(Ordering::Acquire) {
                break;
            }

            thread::sleep(DRAIN_IDLE_SLEEP);
        }
        Ok(writer)
    })
}

/// Placeholder entry point for the capture stage. Returns the event(s) it
/// emits once implemented.
pub fn stage() -> SessionEvent {
    todo!("audio: implement capture per apps/desktop/src-tauri/src/audio/AGENTS.md")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kaydence-capture-runtime-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn start_capture_creates_a_wal_session_under_app_data() {
        let app_data = tmp();
        let mut runtime = WalCaptureRuntime::new_wal_only(&app_data);

        let id = runtime.start_capture(10).unwrap();
        let path = app_data.join("sessions").join(format!("{}.wav", id.0));

        assert_eq!(runtime.active_session_id(), Some(id));
        assert_eq!(runtime.sessions_dir(), app_data.join("sessions"));
        assert!(path.exists());
        assert_eq!(&std::fs::read(path).unwrap()[0..4], b"RIFF");
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn finalize_capture_patches_and_reports_the_wal() {
        let app_data = tmp();
        let mut runtime = WalCaptureRuntime::new_wal_only(&app_data);
        let id = runtime.start_capture(10).unwrap();

        let summary = runtime.finalize_capture(400).unwrap();

        assert_eq!(summary.id, id);
        assert_eq!(summary.started_ms, 10);
        assert_eq!(summary.finalized_ms, 400);
        assert_eq!(summary.samples_written, 0);
        assert_eq!(summary.dropped_input_samples, 0);
        assert!(runtime.active_session_id().is_none());
        let event = summary.audio_persisted_event();
        assert_eq!(event.session_id(), id);
        assert!(summary.wal_path.exists());
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn discard_capture_removes_the_short_tap_wal() {
        let app_data = tmp();
        let mut runtime = WalCaptureRuntime::new_wal_only(&app_data);
        let id = runtime.start_capture(0).unwrap();
        let path = app_data.join("sessions").join(format!("{}.wav", id.0));
        assert!(path.exists());

        let discarded = runtime.discard_capture(100).unwrap();

        assert_eq!(discarded.id, id);
        assert!(discarded.removed);
        assert_eq!(discarded.dropped_input_samples, 0);
        assert!(!path.exists());
        assert!(runtime.active_session_id().is_none());
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn double_start_is_refused_while_wal_is_active() {
        let app_data = tmp();
        let mut runtime = WalCaptureRuntime::new_wal_only(&app_data);
        let id = runtime.start_capture(0).unwrap();

        let err = runtime.start_capture(10).unwrap_err();

        assert!(matches!(err, CaptureRuntimeError::AlreadyActive(active) if active == id));
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn input_callback_downmixes_without_blocking_and_counts_overflow() {
        let rb = HeapRb::<f32>::new(2);
        let (mut producer, mut consumer) = rb.split();
        let dropped = AtomicU64::new(0);

        push_interleaved_mono(
            &[1.0f32, 0.0, 0.5, -0.5, 0.25, 0.75],
            2,
            &mut producer,
            &dropped,
        );

        assert_eq!(consumer.try_pop(), Some(0.5));
        assert_eq!(consumer.try_pop(), Some(0.0));
        assert_eq!(consumer.try_pop(), None);
        assert_eq!(dropped.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn drain_thread_appends_ring_samples_to_wal() {
        let app_data = tmp();
        let dir = app_data.join("sessions");
        let writer = wal::WalWriter::create(&dir, "01DRAIN").unwrap();
        let rb = HeapRb::<f32>::new(8);
        let (mut producer, consumer) = rb.split();
        for sample in [0.25f32, 0.5, -0.25] {
            producer.try_push(sample).unwrap();
        }
        drop(producer);
        let stop = Arc::new(AtomicBool::new(true));

        let handle = spawn_drain_thread(consumer, writer, stop);
        let writer = handle.join().unwrap().unwrap();

        assert_eq!(writer.samples_written(), 3);
        let path = writer.finalize().unwrap();
        let recovered = wal::recover(&path).unwrap();
        assert_eq!(recovered.samples, 3);
        let _ = std::fs::remove_dir_all(app_data);
    }
}
