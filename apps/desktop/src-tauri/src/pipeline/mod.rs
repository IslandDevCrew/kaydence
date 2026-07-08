//! Pipeline coordination across the typed stages.
//!
//! This module does not implement capture, VAD, or ASR itself. It connects the
//! already-owned pieces in the only order allowed by the product contract:
//! WAL on disk first, VAD gate second, engine third.

use crate::audio::{
    self,
    vad::{SpeechGate, SpeechGateConfig, VadDetector},
};
use crate::cleanup;
use crate::engine::{AsrError, AsrRequest, EngineStack};
use crate::events::{CleanupDial, SessionEvent};

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("wal: {0}")]
    Wal(#[from] audio::wal::WalError),
    #[error("asr: {0}")]
    Asr(#[from] AsrError),
}

pub struct TranscriptionPipeline<D> {
    detector: D,
    vad_config: SpeechGateConfig,
    engines: EngineStack,
    dictionary_hints: Vec<String>,
    cleanup_dial: CleanupDial,
}

impl<D> TranscriptionPipeline<D>
where
    D: VadDetector,
{
    pub fn new(detector: D, vad_config: SpeechGateConfig, engines: EngineStack) -> Self {
        Self {
            detector,
            vad_config,
            engines,
            dictionary_hints: Vec::new(),
            cleanup_dial: CleanupDial::Light,
        }
    }

    pub fn with_dictionary_hints(mut self, dictionary_hints: Vec<String>) -> Self {
        self.dictionary_hints = dictionary_hints;
        self
    }

    pub fn with_cleanup_dial(mut self, cleanup_dial: CleanupDial) -> Self {
        self.cleanup_dial = cleanup_dial;
        self
    }

    pub fn process_capture(
        &mut self,
        summary: &audio::CaptureSessionSummary,
    ) -> Result<Vec<SessionEvent>, PipelineError> {
        let wal_samples = audio::wal::read_samples(&summary.wal_path)?;
        let mut gate = SpeechGate::new(&mut self.detector, self.vad_config);
        let mut segments = gate.push_samples(&wal_samples.samples);
        if let Some(segment) = gate.flush() {
            segments.push(segment);
        }

        let mut events = vec![summary.audio_persisted_event()];
        for segment in segments {
            let request = AsrRequest::new(
                summary.id,
                wal_samples.sample_rate,
                segment.start_sample,
                segment.samples,
                self.dictionary_hints.clone(),
            );
            let run = self.engines.transcribe(&request)?;
            for event in run.events {
                let clean_event = match &event {
                    SessionEvent::RawFinal { id, text } => {
                        cleanup::clean_final_event(*id, text, self.cleanup_dial)
                    }
                    _ => None,
                };
                events.push(event);
                if let Some(clean_event) = clean_event {
                    events.push(clean_event);
                }
            }
        }
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{vad::EnergyVad, wal};
    use crate::engine::{AsrEngine, AsrTranscript, EngineLane};
    use crate::events::{CleanupDial, SessionId};
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use ulid::Ulid;

    fn tmp() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kaydence-pipeline-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn test_vad_config() -> SpeechGateConfig {
        SpeechGateConfig {
            sample_rate: 1_000,
            frame_samples: 2,
            pre_roll_samples: 4,
            end_silence_samples: 4,
        }
    }

    fn summary_for_samples(samples: &[f32]) -> (audio::CaptureSessionSummary, std::path::PathBuf) {
        let app_data = tmp();
        let dir = app_data.join("sessions");
        let id = SessionId::new(Ulid::new());
        let mut writer = wal::WalWriter::create(&dir, &id.0.to_string()).unwrap();
        writer.append(samples).unwrap();
        let wal_path = writer.finalize().unwrap();
        (
            audio::CaptureSessionSummary {
                id,
                wal_path,
                samples_written: samples.len() as u64,
                dropped_input_samples: 0,
                started_ms: 10,
                finalized_ms: 400,
            },
            app_data,
        )
    }

    struct QueueEngine {
        outputs: VecDeque<Result<AsrTranscript, AsrError>>,
        requests: Arc<Mutex<Vec<AsrRequest>>>,
    }

    impl QueueEngine {
        fn boxed(
            outputs: Vec<Result<AsrTranscript, AsrError>>,
            requests: Arc<Mutex<Vec<AsrRequest>>>,
        ) -> Box<dyn AsrEngine + Send> {
            Box::new(Self {
                outputs: outputs.into(),
                requests,
            })
        }
    }

    impl AsrEngine for QueueEngine {
        fn lane(&self) -> EngineLane {
            EngineLane::LocalCpu
        }

        fn transcribe(&mut self, request: &AsrRequest) -> Result<AsrTranscript, AsrError> {
            self.requests.lock().unwrap().push(request.clone());
            self.outputs
                .pop_front()
                .unwrap_or_else(|| Err(AsrError::Inference("no scripted output".to_string())))
        }
    }

    fn pipeline(
        outputs: Vec<Result<AsrTranscript, AsrError>>,
        requests: Arc<Mutex<Vec<AsrRequest>>>,
    ) -> TranscriptionPipeline<EnergyVad> {
        let engines = EngineStack::new(vec![QueueEngine::boxed(outputs, requests)]);
        TranscriptionPipeline::new(EnergyVad::new(0.2), test_vad_config(), engines)
            .with_dictionary_hints(vec!["Kaydence".to_string()])
    }

    #[test]
    fn persisted_audio_event_precedes_engine_and_clean_events_for_speech() {
        let (summary, app_data) = summary_for_samples(&[0.0, 0.0, 0.5, 0.5]);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let mut pipeline = pipeline(
            vec![Ok(AsrTranscript::raw("um hello captain"))],
            Arc::clone(&requests),
        );

        let events = pipeline.process_capture(&summary).unwrap();

        assert_eq!(events[0], summary.audio_persisted_event());
        assert_eq!(
            events[1],
            SessionEvent::RawFinal {
                id: summary.id,
                text: "um hello captain".to_string()
            }
        );
        assert_eq!(
            events[2],
            SessionEvent::CleanFinal {
                id: summary.id,
                text: "Hello captain.".to_string(),
                dial: CleanupDial::Light
            }
        );
        let seen = requests.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].sample_rate, wal::SAMPLE_RATE);
        assert_eq!(seen[0].start_sample, 0);
        assert_eq!(seen[0].dictionary_hints, vec!["Kaydence"]);
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn silence_only_capture_does_not_call_engine() {
        let (summary, app_data) = summary_for_samples(&[0.0; 8]);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let mut pipeline = pipeline(
            vec![Ok(AsrTranscript::raw("should not happen"))],
            Arc::clone(&requests),
        );

        let events = pipeline.process_capture(&summary).unwrap();

        assert_eq!(events, vec![summary.audio_persisted_event()]);
        assert!(requests.lock().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn separated_speech_segments_create_separate_engine_requests() {
        let samples = [0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 0.6, 0.6];
        let (summary, app_data) = summary_for_samples(&samples);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let mut pipeline = pipeline(
            vec![
                Ok(AsrTranscript::raw("first")),
                Ok(AsrTranscript::raw("second")),
            ],
            Arc::clone(&requests),
        );

        let events = pipeline.process_capture(&summary).unwrap();

        assert_eq!(events.len(), 5);
        assert_eq!(
            events[1],
            SessionEvent::RawFinal {
                id: summary.id,
                text: "first".to_string()
            }
        );
        assert_eq!(
            events[2],
            SessionEvent::CleanFinal {
                id: summary.id,
                text: "First.".to_string(),
                dial: CleanupDial::Light
            }
        );
        assert_eq!(
            events[3],
            SessionEvent::RawFinal {
                id: summary.id,
                text: "second".to_string()
            }
        );
        assert_eq!(
            events[4],
            SessionEvent::CleanFinal {
                id: summary.id,
                text: "Second.".to_string(),
                dial: CleanupDial::Light
            }
        );
        let seen = requests.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].samples.len(), 2);
        assert_eq!(seen[1].start_sample, 2);
        assert_eq!(seen[1].samples.len(), 6);
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn raw_cleanup_dial_bypasses_clean_final_events() {
        let (summary, app_data) = summary_for_samples(&[0.0, 0.0, 0.5, 0.5]);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let mut pipeline = pipeline(
            vec![Ok(AsrTranscript::raw("um untouched raw"))],
            Arc::clone(&requests),
        )
        .with_cleanup_dial(CleanupDial::Raw);

        let events = pipeline.process_capture(&summary).unwrap();

        assert_eq!(
            events,
            vec![
                summary.audio_persisted_event(),
                SessionEvent::RawFinal {
                    id: summary.id,
                    text: "um untouched raw".to_string(),
                }
            ]
        );
        let _ = std::fs::remove_dir_all(app_data);
    }
}
