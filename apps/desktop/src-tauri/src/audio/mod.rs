//! Capture: mic -> lock-free ring buffer -> WAL file -> VAD gate. Emits
//! AudioPersisted (non-negotiable #2).
//!
//! `wal` is real (P1, PRD P0-4): write-ahead persistence + crash recovery.
//! cpal capture now feeds a lock-free ring buffer; the drain thread owns WAL
//! writes and resamples native device rates back to the 16 kHz WAL contract.
//! The VAD gate contract is in `vad`; the production Silero adapter lands with
//! ASR.
#![allow(dead_code)]

pub mod vad;
pub mod wal;

use crate::events::{SessionEvent, SessionId};
use audioadapter_buffers::direct::InterleavedSlice;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapCons, HeapProd, HeapRb,
};
use rubato::{Fft, FixedSync, Indexing, Resampler};
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
const FALLBACK_INPUT_SAMPLE_RATES: [cpal::SampleRate; 3] = [48_000, 44_100, 32_000];

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
    #[error("no supported input config can feed {0} Hz WAL PCM")]
    NoSupportedInputConfig(u32),
    #[error("unsupported input sample format: {0}")]
    UnsupportedSampleFormat(cpal::SampleFormat),
    #[error("input config: {0}")]
    InputConfig(#[from] cpal::SupportedStreamConfigsError),
    #[error("build input stream: {0}")]
    BuildInputStream(#[from] cpal::BuildStreamError),
    #[error("play input stream: {0}")]
    PlayInputStream(#[from] cpal::PlayStreamError),
    #[error("resampler construction: {0}")]
    ResamplerConstruction(#[from] rubato::ResamplerConstructionError),
    #[error("input drain: {0}")]
    Drain(#[from] DrainError),
    #[error("input drain thread panicked")]
    DrainThreadPanicked,
}

#[derive(Debug, thiserror::Error)]
pub enum DrainError {
    #[error("wal: {0}")]
    Wal(#[from] wal::WalError),
    #[error("resample: {0}")]
    Resample(#[from] rubato::ResampleError),
    #[error("audio adapter: {0}")]
    Adapter(#[from] audioadapter_buffers::SizeError),
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
    drain_thread: Option<JoinHandle<Result<wal::WalWriter, DrainError>>>,
}

impl MicCaptureSession {
    fn start(
        prepared: PreparedMicInput,
        writer: wal::WalWriter,
    ) -> Result<Self, CaptureRuntimeError> {
        let stop = Arc::new(AtomicBool::new(false));
        let dropped_input_samples = Arc::clone(&prepared.dropped_input_samples);
        let drain = WalDrain::new(prepared.source_sample_rate)?;
        let drain_thread = spawn_drain_thread(prepared.consumer, writer, Arc::clone(&stop), drain);
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
    source_sample_rate: cpal::SampleRate,
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
        let source_sample_rate = config.sample_rate;
        let rb = HeapRb::<f32>::new((source_sample_rate as usize) * INPUT_RING_SECONDS);
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
            source_sample_rate,
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
    choose_input_config_from_ranges(device.supported_input_configs()?)
}

fn choose_input_config_from_ranges(
    ranges: impl IntoIterator<Item = cpal::SupportedStreamConfigRange>,
) -> Result<cpal::SupportedStreamConfig, CaptureRuntimeError> {
    let target_rate = wal::SAMPLE_RATE;
    let ranges = ranges
        .into_iter()
        .filter(|range| supported_pcm_sample_format(range.sample_format()))
        .collect::<Vec<_>>();

    let mut exact_candidates = ranges
        .iter()
        .filter_map(|range| range.try_with_sample_rate(target_rate))
        .collect::<Vec<_>>();
    exact_candidates.sort_by_key(|config| {
        (
            config.channels() != 1,
            sample_format_priority(config.sample_format()),
            config.channels(),
        )
    });
    if let Some(config) = exact_candidates.into_iter().next() {
        return Ok(config);
    }

    let mut candidates = ranges
        .into_iter()
        .map(recommended_fallback_input_config)
        .collect::<Vec<_>>();
    candidates.sort_by_key(|config| {
        (
            fallback_sample_rate_priority(config.sample_rate()),
            sample_format_priority(config.sample_format()),
            config.channels() != 1,
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

fn recommended_fallback_input_config(
    range: cpal::SupportedStreamConfigRange,
) -> cpal::SupportedStreamConfig {
    FALLBACK_INPUT_SAMPLE_RATES
        .into_iter()
        .find_map(|sample_rate| range.try_with_sample_rate(sample_rate))
        .unwrap_or_else(|| range.with_max_sample_rate())
}

fn fallback_sample_rate_priority(sample_rate: cpal::SampleRate) -> (u8, u32) {
    let common_rate_rank = FALLBACK_INPUT_SAMPLE_RATES
        .iter()
        .position(|rate| *rate == sample_rate)
        .map(|index| index as u8)
        .unwrap_or(u8::MAX);
    (common_rate_rank, sample_rate.abs_diff(wal::SAMPLE_RATE))
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
    mut drain: WalDrain,
) -> JoinHandle<Result<wal::WalWriter, DrainError>> {
    thread::spawn(move || {
        let mut scratch = vec![0.0; DRAIN_CHUNK_SAMPLES];
        loop {
            let n = consumer.pop_slice(&mut scratch);
            if n > 0 {
                drain.append(&mut writer, &scratch[..n])?;
                continue;
            }

            if stop.load(Ordering::Acquire) {
                break;
            }

            thread::sleep(DRAIN_IDLE_SLEEP);
        }
        drain.finish(&mut writer)?;
        Ok(writer)
    })
}

enum WalDrain {
    Passthrough,
    Resampling(Box<ResamplingWalDrain>),
}

impl WalDrain {
    fn new(source_sample_rate: cpal::SampleRate) -> Result<Self, CaptureRuntimeError> {
        if source_sample_rate == wal::SAMPLE_RATE {
            return Ok(Self::Passthrough);
        }

        Ok(Self::Resampling(Box::new(ResamplingWalDrain::new(
            source_sample_rate,
        )?)))
    }

    fn append(&mut self, writer: &mut wal::WalWriter, samples: &[f32]) -> Result<(), DrainError> {
        match self {
            Self::Passthrough => writer.append(samples).map_err(DrainError::Wal),
            Self::Resampling(drain) => drain.append(writer, samples),
        }
    }

    fn finish(&mut self, writer: &mut wal::WalWriter) -> Result<(), DrainError> {
        match self {
            Self::Passthrough => Ok(()),
            Self::Resampling(drain) => drain.finish(writer),
        }
    }
}

struct ResamplingWalDrain {
    resampler: Fft<f32>,
    input_buffer: Vec<f32>,
    output_buffer: Vec<f32>,
    processed_any: bool,
}

impl ResamplingWalDrain {
    fn new(
        source_sample_rate: cpal::SampleRate,
    ) -> Result<Self, rubato::ResamplerConstructionError> {
        let resampler = Fft::<f32>::new(
            source_sample_rate as usize,
            wal::SAMPLE_RATE as usize,
            DRAIN_CHUNK_SAMPLES,
            2,
            1,
            FixedSync::Input,
        )?;
        Ok(Self {
            resampler,
            input_buffer: Vec::with_capacity(DRAIN_CHUNK_SAMPLES * 2),
            output_buffer: Vec::new(),
            processed_any: false,
        })
    }

    fn append(&mut self, writer: &mut wal::WalWriter, samples: &[f32]) -> Result<(), DrainError> {
        self.input_buffer.extend_from_slice(samples);
        while self.input_buffer.len() >= self.resampler.input_frames_next() {
            self.process_next(writer, None)?;
        }
        Ok(())
    }

    fn finish(&mut self, writer: &mut wal::WalWriter) -> Result<(), DrainError> {
        if self.processed_any || !self.input_buffer.is_empty() {
            self.process_next(writer, Some(self.input_buffer.len()))?;
        }
        Ok(())
    }

    fn process_next(
        &mut self,
        writer: &mut wal::WalWriter,
        partial_len: Option<usize>,
    ) -> Result<(), DrainError> {
        let input_frames = partial_len.unwrap_or_else(|| self.resampler.input_frames_next());
        let output_frames = self.resampler.output_frames_next();
        self.output_buffer.resize(output_frames, 0.0);

        let input = InterleavedSlice::new(&self.input_buffer, 1, input_frames)?;
        let mut output = InterleavedSlice::new_mut(&mut self.output_buffer, 1, output_frames)?;
        let indexing = partial_len.map(|partial_len| Indexing {
            input_offset: 0,
            output_offset: 0,
            partial_len: Some(partial_len),
            active_channels_mask: None,
        });
        let (input_consumed, output_written) =
            self.resampler
                .process_into_buffer(&input, &mut output, indexing.as_ref())?;

        writer.append(&self.output_buffer[..output_written])?;
        let consumed = if partial_len.is_some() {
            self.input_buffer.len()
        } else {
            input_consumed.min(self.input_buffer.len())
        };
        self.input_buffer.drain(..consumed);
        self.processed_any = true;
        Ok(())
    }
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

    fn input_range(
        channels: cpal::ChannelCount,
        min_sample_rate: cpal::SampleRate,
        max_sample_rate: cpal::SampleRate,
        sample_format: cpal::SampleFormat,
    ) -> cpal::SupportedStreamConfigRange {
        cpal::SupportedStreamConfigRange::new(
            channels,
            min_sample_rate,
            max_sample_rate,
            cpal::SupportedBufferSize::Unknown,
            sample_format,
        )
    }

    #[test]
    fn input_config_prefers_native_wal_rate_when_available() {
        let config = choose_input_config_from_ranges([
            input_range(2, 48_000, 48_000, cpal::SampleFormat::F32),
            input_range(
                1,
                wal::SAMPLE_RATE,
                wal::SAMPLE_RATE,
                cpal::SampleFormat::I16,
            ),
        ])
        .unwrap();

        assert_eq!(config.sample_rate(), wal::SAMPLE_RATE);
        assert_eq!(config.channels(), 1);
        assert_eq!(config.sample_format(), cpal::SampleFormat::I16);
    }

    #[test]
    fn input_config_falls_back_to_common_device_rate() {
        let config = choose_input_config_from_ranges([
            input_range(1, 96_000, 96_000, cpal::SampleFormat::F32),
            input_range(2, 44_100, 48_000, cpal::SampleFormat::I16),
        ])
        .unwrap();

        assert_eq!(config.sample_rate(), 48_000);
        assert_eq!(config.channels(), 2);
        assert_eq!(config.sample_format(), cpal::SampleFormat::I16);
    }

    #[test]
    fn input_config_rejects_non_pcm_ranges() {
        let err = choose_input_config_from_ranges([input_range(
            1,
            48_000,
            48_000,
            cpal::SampleFormat::I24,
        )])
        .unwrap_err();

        assert!(matches!(
            err,
            CaptureRuntimeError::NoSupportedInputConfig(rate) if rate == wal::SAMPLE_RATE
        ));
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

        let drain = WalDrain::new(wal::SAMPLE_RATE).unwrap();
        let handle = spawn_drain_thread(consumer, writer, stop, drain);
        let writer = handle.join().unwrap().unwrap();

        assert_eq!(writer.samples_written(), 3);
        let path = writer.finalize().unwrap();
        let recovered = wal::recover(&path).unwrap();
        assert_eq!(recovered.samples, 3);
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn drain_resamples_native_input_rate_to_wal_rate() {
        let app_data = tmp();
        let dir = app_data.join("sessions");
        let mut writer = wal::WalWriter::create(&dir, "01RESAMPLE").unwrap();
        let mut drain = WalDrain::new(48_000).unwrap();
        let samples = vec![0.25f32; 4_800];

        drain.append(&mut writer, &samples).unwrap();
        drain.finish(&mut writer).unwrap();

        let samples_written = writer.samples_written();
        assert!(samples_written > 1_200);
        assert!(samples_written < 2_200);
        let path = writer.finalize().unwrap();
        let recovered = wal::recover(&path).unwrap();
        assert_eq!(recovered.samples, samples_written);
        let _ = std::fs::remove_dir_all(app_data);
    }
}
