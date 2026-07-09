//! Recognize: chunks -> streaming Partial(text) -> RawFinal(text). Parakeet CPU + Whisper GPU behind the AsrEngine trait (ADR-0002).
//!
//! This module owns the dependency-free ASR contract and fallback policy. Model
//! adapters (Parakeet, Whisper, BYOK) plug in later; the contract is testable
//! now without pretending that model inference exists.
#![allow(dead_code)]

use crate::events::{SessionEvent, SessionId};

pub const DEFAULT_NO_SPEECH_REJECT_THRESHOLD: f32 = 0.80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineLane {
    ByokCloud,
    LocalGpu,
    LocalCpu,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AsrRequest {
    pub id: SessionId,
    pub sample_rate: u32,
    pub start_sample: u64,
    pub samples: Vec<f32>,
    pub dictionary_hints: Vec<String>,
}

impl AsrRequest {
    pub fn new(
        id: SessionId,
        sample_rate: u32,
        start_sample: u64,
        samples: Vec<f32>,
        dictionary_hints: Vec<String>,
    ) -> Self {
        Self {
            id,
            sample_rate,
            start_sample,
            samples,
            dictionary_hints,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialTranscript {
    pub text: String,
    pub t_lag_ms: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AsrTranscript {
    pub partials: Vec<PartialTranscript>,
    pub final_text: String,
    pub no_speech_probability: Option<f32>,
}

impl AsrTranscript {
    pub fn raw(final_text: impl Into<String>) -> Self {
        Self {
            partials: Vec::new(),
            final_text: final_text.into(),
            no_speech_probability: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AsrError {
    Unavailable(String),
    Timeout { after_ms: u64 },
    Inference(String),
    EmptyTranscript,
    NoSpeech { probability: f32, threshold: f32 },
    AllEnginesFailed,
}

impl std::fmt::Display for AsrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(reason) => write!(f, "engine unavailable: {reason}"),
            Self::Timeout { after_ms } => write!(f, "engine timed out after {after_ms} ms"),
            Self::Inference(reason) => write!(f, "engine inference failed: {reason}"),
            Self::EmptyTranscript => write!(f, "engine returned an empty transcript"),
            Self::NoSpeech {
                probability,
                threshold,
            } => write!(
                f,
                "engine rejected likely no-speech final: probability {probability:.2}, threshold {threshold:.2}"
            ),
            Self::AllEnginesFailed => write!(f, "all configured ASR engines failed"),
        }
    }
}

impl std::error::Error for AsrError {}

pub trait AsrEngine {
    fn lane(&self) -> EngineLane;
    fn transcribe(&mut self, request: &AsrRequest) -> Result<AsrTranscript, AsrError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineAttempt {
    pub lane: EngineLane,
    pub outcome: EngineAttemptOutcome,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EngineAttemptOutcome {
    Succeeded,
    Failed(AsrError),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineRun {
    pub lane: EngineLane,
    pub attempts: Vec<EngineAttempt>,
    pub transcript: AsrTranscript,
    pub events: Vec<SessionEvent>,
}

pub struct EngineStack {
    engines: Vec<Box<dyn AsrEngine + Send>>,
    no_speech_threshold: f32,
}

impl EngineStack {
    pub fn new(engines: Vec<Box<dyn AsrEngine + Send>>) -> Self {
        Self {
            engines,
            no_speech_threshold: DEFAULT_NO_SPEECH_REJECT_THRESHOLD,
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn with_no_speech_threshold(mut self, threshold: f32) -> Self {
        self.no_speech_threshold = threshold;
        self
    }

    pub fn transcribe(&mut self, request: &AsrRequest) -> Result<EngineRun, AsrError> {
        let mut attempts = Vec::new();

        for engine in &mut self.engines {
            let lane = engine.lane();
            match engine.transcribe(request) {
                Ok(transcript) => {
                    let events =
                        transcript_events(request.id, &transcript, self.no_speech_threshold)?;
                    attempts.push(EngineAttempt {
                        lane,
                        outcome: EngineAttemptOutcome::Succeeded,
                    });
                    return Ok(EngineRun {
                        lane,
                        attempts,
                        transcript,
                        events,
                    });
                }
                Err(err) => {
                    attempts.push(EngineAttempt {
                        lane,
                        outcome: EngineAttemptOutcome::Failed(err),
                    });
                }
            }
        }

        Err(AsrError::AllEnginesFailed)
    }
}

pub fn transcript_events(
    id: SessionId,
    transcript: &AsrTranscript,
    no_speech_threshold: f32,
) -> Result<Vec<SessionEvent>, AsrError> {
    if transcript
        .no_speech_probability
        .is_some_and(|probability| probability >= no_speech_threshold)
    {
        return Err(AsrError::NoSpeech {
            probability: transcript.no_speech_probability.unwrap_or_default(),
            threshold: no_speech_threshold,
        });
    }

    if transcript.final_text.trim().is_empty() {
        return Err(AsrError::EmptyTranscript);
    }

    let mut events = transcript
        .partials
        .iter()
        .map(|partial| SessionEvent::Partial {
            id,
            text: partial.text.clone(),
            t_lag_ms: partial.t_lag_ms,
        })
        .collect::<Vec<_>>();
    events.push(SessionEvent::RawFinal {
        id,
        text: transcript.final_text.clone(),
    });
    Ok(events)
}

/// Placeholder entry point for the `engine` stage. Returns the event(s) it emits
/// once implemented.
pub fn stage() -> SessionEvent {
    todo!("engine: implement per apps/desktop/src-tauri/src/engine/AGENTS.md")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ulid::Ulid;

    fn session_id() -> SessionId {
        SessionId::new(Ulid::new())
    }

    fn request(id: SessionId) -> AsrRequest {
        AsrRequest::new(
            id,
            16_000,
            0,
            vec![0.1, 0.2, 0.1],
            vec!["Kaydence".to_string()],
        )
    }

    #[derive(Clone)]
    struct ScriptedEngine {
        lane: EngineLane,
        result: Result<AsrTranscript, AsrError>,
    }

    impl ScriptedEngine {
        fn boxed(
            lane: EngineLane,
            result: Result<AsrTranscript, AsrError>,
        ) -> Box<dyn AsrEngine + Send> {
            Box::new(Self { lane, result })
        }
    }

    impl AsrEngine for ScriptedEngine {
        fn lane(&self) -> EngineLane {
            self.lane
        }

        fn transcribe(&mut self, _request: &AsrRequest) -> Result<AsrTranscript, AsrError> {
            self.result.clone()
        }
    }

    #[test]
    fn transcript_maps_partials_before_raw_final() {
        let id = session_id();
        let transcript = AsrTranscript {
            partials: vec![
                PartialTranscript {
                    text: "hello".to_string(),
                    t_lag_ms: 120,
                },
                PartialTranscript {
                    text: "hello world".to_string(),
                    t_lag_ms: 180,
                },
            ],
            final_text: "hello world.".to_string(),
            no_speech_probability: Some(0.05),
        };

        let events = transcript_events(id, &transcript, 0.8).unwrap();

        assert_eq!(
            events,
            vec![
                SessionEvent::Partial {
                    id,
                    text: "hello".to_string(),
                    t_lag_ms: 120,
                },
                SessionEvent::Partial {
                    id,
                    text: "hello world".to_string(),
                    t_lag_ms: 180,
                },
                SessionEvent::RawFinal {
                    id,
                    text: "hello world.".to_string(),
                },
            ]
        );
    }

    #[test]
    fn request_constructor_preserves_audio_boundary_metadata() {
        let id = session_id();
        let request = AsrRequest::new(id, 16_000, 4_000, vec![0.1, -0.1], vec!["IDC".into()]);

        assert_eq!(request.id, id);
        assert_eq!(request.sample_rate, 16_000);
        assert_eq!(request.start_sample, 4_000);
        assert_eq!(request.samples, vec![0.1, -0.1]);
        assert_eq!(request.dictionary_hints, vec!["IDC"]);
    }

    #[test]
    fn fallback_chain_uses_local_cpu_after_cloud_timeout_and_gpu_unavailable() {
        let id = session_id();
        let mut stack = EngineStack::new(vec![
            ScriptedEngine::boxed(
                EngineLane::ByokCloud,
                Err(AsrError::Timeout { after_ms: 2_000 }),
            ),
            ScriptedEngine::boxed(
                EngineLane::LocalGpu,
                Err(AsrError::Unavailable("no GPU provider".to_string())),
            ),
            ScriptedEngine::boxed(EngineLane::LocalCpu, Ok(AsrTranscript::raw("local result"))),
        ]);

        let run = stack.transcribe(&request(id)).unwrap();

        assert_eq!(run.lane, EngineLane::LocalCpu);
        assert_eq!(run.attempts.len(), 3);
        assert!(matches!(
            run.attempts[0].outcome,
            EngineAttemptOutcome::Failed(AsrError::Timeout { after_ms: 2_000 })
        ));
        assert_eq!(
            run.events,
            vec![SessionEvent::RawFinal {
                id,
                text: "local result".to_string()
            }]
        );
    }

    #[test]
    fn no_speech_probability_rejects_final() {
        let transcript = AsrTranscript {
            partials: Vec::new(),
            final_text: "random hallucination".to_string(),
            no_speech_probability: Some(0.92),
        };

        let err = transcript_events(session_id(), &transcript, 0.8).unwrap_err();

        assert!(matches!(
            err,
            AsrError::NoSpeech {
                probability,
                threshold: 0.8
            } if (probability - 0.92).abs() < f32::EPSILON
        ));
    }

    #[test]
    fn empty_transcript_is_rejected() {
        let err = transcript_events(session_id(), &AsrTranscript::raw("  "), 0.8).unwrap_err();

        assert_eq!(err, AsrError::EmptyTranscript);
    }

    #[test]
    fn all_engines_failed_reports_terminal_failure() {
        let id = session_id();
        let mut stack = EngineStack::new(vec![ScriptedEngine::boxed(
            EngineLane::LocalCpu,
            Err(AsrError::Inference("model missing".to_string())),
        )]);

        let err = stack.transcribe(&request(id)).unwrap_err();

        assert_eq!(err, AsrError::AllEnginesFailed);
    }
}
