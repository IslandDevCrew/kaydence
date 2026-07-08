//! Speech gating at the audio/engine boundary.
//!
//! The production detector will be Silero-backed, but this module keeps the
//! contract dependency-free: WAL receives every sample first, then this gate
//! decides which 16 kHz mono samples are eligible for ASR.

use super::wal;
use std::collections::VecDeque;

pub const DEFAULT_FRAME_SAMPLES: usize = 512;
pub const DEFAULT_PRE_ROLL_MS: u32 = 250;
pub const DEFAULT_END_SILENCE_MS: u32 = 300;

#[derive(Debug, Clone, PartialEq)]
pub struct SpeechSegment {
    pub start_sample: u64,
    pub samples: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeechGateConfig {
    pub sample_rate: u32,
    pub frame_samples: usize,
    pub pre_roll_samples: usize,
    pub end_silence_samples: usize,
}

impl Default for SpeechGateConfig {
    fn default() -> Self {
        Self::from_millis(
            wal::SAMPLE_RATE,
            DEFAULT_FRAME_SAMPLES,
            DEFAULT_PRE_ROLL_MS,
            DEFAULT_END_SILENCE_MS,
        )
    }
}

impl SpeechGateConfig {
    pub fn from_millis(
        sample_rate: u32,
        frame_samples: usize,
        pre_roll_ms: u32,
        end_silence_ms: u32,
    ) -> Self {
        Self {
            sample_rate,
            frame_samples,
            pre_roll_samples: samples_for_ms(sample_rate, pre_roll_ms),
            end_silence_samples: samples_for_ms(sample_rate, end_silence_ms),
        }
    }
}

pub trait VadDetector {
    fn is_speech(&mut self, frame: &[f32]) -> bool;
}

#[derive(Debug, Clone, Copy)]
pub struct EnergyVad {
    threshold_rms: f32,
}

impl EnergyVad {
    pub fn new(threshold_rms: f32) -> Self {
        Self { threshold_rms }
    }
}

impl VadDetector for EnergyVad {
    fn is_speech(&mut self, frame: &[f32]) -> bool {
        if frame.is_empty() {
            return false;
        }

        let energy = frame.iter().map(|sample| sample * sample).sum::<f32>();
        let rms = (energy / frame.len() as f32).sqrt();
        rms >= self.threshold_rms
    }
}

pub struct SpeechGate<D> {
    detector: D,
    config: SpeechGateConfig,
    frame_buffer: Vec<f32>,
    pre_roll: VecDeque<f32>,
    active: Option<ActiveSegment>,
    pending_silence: Vec<f32>,
    next_frame_start_sample: u64,
}

impl<D> SpeechGate<D>
where
    D: VadDetector,
{
    pub fn new(detector: D, config: SpeechGateConfig) -> Self {
        assert!(config.frame_samples > 0, "frame_samples must be non-zero");
        Self {
            detector,
            config,
            frame_buffer: Vec::with_capacity(config.frame_samples),
            pre_roll: VecDeque::with_capacity(config.pre_roll_samples),
            active: None,
            pending_silence: Vec::new(),
            next_frame_start_sample: 0,
        }
    }

    pub fn push_samples(&mut self, samples: &[f32]) -> Vec<SpeechSegment> {
        let mut segments = Vec::new();
        self.frame_buffer.extend_from_slice(samples);

        while self.frame_buffer.len() >= self.config.frame_samples {
            let frame = self
                .frame_buffer
                .drain(..self.config.frame_samples)
                .collect::<Vec<_>>();
            let is_speech = self.detector.is_speech(&frame);
            self.process_frame(&frame, is_speech, &mut segments);
            self.next_frame_start_sample += frame.len() as u64;
        }

        segments
    }

    pub fn flush(&mut self) -> Option<SpeechSegment> {
        if !self.frame_buffer.is_empty() {
            let frame = std::mem::take(&mut self.frame_buffer);
            let is_speech = self.detector.is_speech(&frame);
            let mut segments = Vec::new();
            self.process_frame(&frame, is_speech, &mut segments);
            self.next_frame_start_sample += frame.len() as u64;
            debug_assert!(segments.is_empty());
        }

        self.pending_silence.clear();
        self.active.take().map(|active| SpeechSegment {
            start_sample: active.start_sample,
            samples: active.samples,
        })
    }

    fn process_frame(&mut self, frame: &[f32], is_speech: bool, segments: &mut Vec<SpeechSegment>) {
        match (&mut self.active, is_speech) {
            (None, false) => self.push_pre_roll(frame),
            (None, true) => self.start_segment_with_pre_roll(frame),
            (Some(active), true) => {
                active.samples.append(&mut self.pending_silence);
                active.samples.extend_from_slice(frame);
            }
            (Some(_), false) => {
                self.pending_silence.extend_from_slice(frame);
                if self.pending_silence.len() >= self.config.end_silence_samples {
                    if let Some(active) = self.active.take() {
                        segments.push(SpeechSegment {
                            start_sample: active.start_sample,
                            samples: active.samples,
                        });
                    }
                    let silence = std::mem::take(&mut self.pending_silence);
                    self.push_pre_roll(&silence);
                }
            }
        }
    }

    fn start_segment_with_pre_roll(&mut self, frame: &[f32]) {
        let pre_roll_len = self.pre_roll.len();
        let start_sample = self
            .next_frame_start_sample
            .saturating_sub(pre_roll_len as u64);
        let mut samples = Vec::with_capacity(pre_roll_len + frame.len());
        samples.extend(self.pre_roll.drain(..));
        samples.extend_from_slice(frame);
        self.active = Some(ActiveSegment {
            start_sample,
            samples,
        });
    }

    fn push_pre_roll(&mut self, samples: &[f32]) {
        if self.config.pre_roll_samples == 0 {
            self.pre_roll.clear();
            return;
        }

        for sample in samples {
            if self.pre_roll.len() == self.config.pre_roll_samples {
                self.pre_roll.pop_front();
            }
            self.pre_roll.push_back(*sample);
        }
    }
}

struct ActiveSegment {
    start_sample: u64,
    samples: Vec<f32>,
}

fn samples_for_ms(sample_rate: u32, ms: u32) -> usize {
    (((sample_rate as u64) * (ms as u64)) / 1_000) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> SpeechGateConfig {
        SpeechGateConfig {
            sample_rate: 1_000,
            frame_samples: 2,
            pre_roll_samples: 4,
            end_silence_samples: 4,
        }
    }

    fn gate() -> SpeechGate<EnergyVad> {
        SpeechGate::new(EnergyVad::new(0.2), test_config())
    }

    #[test]
    fn default_config_matches_audio_boundary_and_preroll_contract() {
        let config = SpeechGateConfig::default();

        assert_eq!(config.sample_rate, wal::SAMPLE_RATE);
        assert_eq!(config.frame_samples, DEFAULT_FRAME_SAMPLES);
        assert_eq!(config.pre_roll_samples, 4_000);
        assert_eq!(config.end_silence_samples, 4_800);
    }

    #[test]
    fn energy_vad_detects_frames_above_threshold() {
        let mut vad = EnergyVad::new(0.2);

        assert!(!vad.is_speech(&[0.01, -0.01, 0.0, 0.02]));
        assert!(vad.is_speech(&[0.25, -0.25, 0.3, -0.3]));
    }

    #[test]
    fn speech_segment_includes_configured_pre_roll() {
        let mut gate = gate();

        assert!(gate.push_samples(&[0.0, 0.0, 0.0, 0.0]).is_empty());
        assert!(gate.push_samples(&[0.5, 0.5]).is_empty());
        let segment = gate.flush().unwrap();

        assert_eq!(segment.start_sample, 0);
        assert_eq!(segment.samples, vec![0.0, 0.0, 0.0, 0.0, 0.5, 0.5]);
    }

    #[test]
    fn long_leading_silence_is_trimmed_to_pre_roll() {
        let mut gate = gate();

        assert!(gate
            .push_samples(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
            .is_empty());
        assert!(gate.push_samples(&[0.5, 0.5]).is_empty());
        let segment = gate.flush().unwrap();

        assert_eq!(segment.start_sample, 2);
        assert_eq!(segment.samples, vec![0.0, 0.0, 0.0, 0.0, 0.5, 0.5]);
    }

    #[test]
    fn short_pause_stays_inside_active_segment() {
        let mut gate = gate();

        assert!(gate.push_samples(&[0.5, 0.5]).is_empty());
        assert!(gate.push_samples(&[0.0, 0.0]).is_empty());
        assert!(gate.push_samples(&[0.6, 0.6]).is_empty());
        let segment = gate.flush().unwrap();

        assert_eq!(segment.start_sample, 0);
        assert_eq!(segment.samples, vec![0.5, 0.5, 0.0, 0.0, 0.6, 0.6]);
    }

    #[test]
    fn configured_silence_ends_segment_without_trailing_silence() {
        let mut gate = gate();

        assert!(gate.push_samples(&[0.5, 0.5]).is_empty());
        let ended = gate.push_samples(&[0.0, 0.0, 0.0, 0.0]);
        assert_eq!(ended.len(), 1);
        assert_eq!(ended[0].samples, vec![0.5, 0.5]);

        assert!(gate.push_samples(&[0.4, 0.4]).is_empty());
        let next = gate.flush().unwrap();
        assert_eq!(next.start_sample, 2);
        assert_eq!(next.samples, vec![0.0, 0.0, 0.0, 0.0, 0.4, 0.4]);
    }

    #[test]
    fn incomplete_frames_are_buffered_until_flush() {
        let mut gate = gate();

        assert!(gate.push_samples(&[0.5]).is_empty());
        let segment = gate.flush().unwrap();

        assert_eq!(segment.start_sample, 0);
        assert_eq!(segment.samples, vec![0.5]);
    }
}
