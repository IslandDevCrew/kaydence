//! The `SessionEvent` contract — the ONLY coupling allowed between pipeline
//! stages (root `AGENTS.md` §4, `docs/ARCHITECTURE.md` §4/§6b).
//!
//! Every stage consumes upstream events and emits its own; no stage imports
//! another stage's internals. This module is plain data + serde so the frontend
//! can mirror it as generated TS types (never hand-written — root §9). Keep it
//! dependency-light: the contract must compile without the rest of the app.
//!
//! Divergence from ARCHITECTURE §4 (documented on purpose): `Instant` is not
//! serializable, and the frontend needs a number, so timestamps are represented
//! as `at_ms: u64` (milliseconds on a monotonic session clock) rather than
//! `Instant`. Semantics are unchanged.

use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// One dictation = one `SessionId` (ULID), threaded through every stage and log
/// line (ARCHITECTURE §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Ulid);

impl SessionId {
    /// Construct from a ULID. Generation happens in the app (needs a clock/RNG);
    /// the contract stays pure so it compiles and tests without a runtime.
    pub fn new(id: Ulid) -> Self {
        Self(id)
    }
}

/// A reference to the frontmost application a session targets (bound at capture
/// start — Pitfall P9). Kept minimal; platform detail lives in `profiles/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRef {
    /// Stable process/bundle identifier (e.g. `com.apple.Safari`, `Code.exe`,
    /// `org.gnome.Console`).
    pub id: String,
    /// Human-readable name for the HUD/history.
    pub name: String,
}

/// Cleanup intensity — a dial, not a default (non-negotiable #3, ADR-0003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CleanupDial {
    /// Stage skipped — verbatim ASR output.
    Raw,
    /// Default: rules + constrained local LLM (filler removal, self-correction
    /// collapse, punctuation). No authoring.
    #[default]
    Light,
    /// Opt-in profile-prompt rewrite.
    Full,
}

/// How committed text reached the focused field (ARCHITECTURE §5). Native first;
/// clipboard only as a snapshot-restoring fallback (Pitfall P2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjectMethod {
    /// macOS AX `AXValue`, Windows UIA `ValuePattern`/`TextPattern`, Linux
    /// AT-SPI insertion.
    Native,
    /// Synthesized keystrokes (e.g. `enigo`) where native insertion is
    /// unavailable.
    Keystroke,
    /// Clipboard set + paste + restore (bounded ≤200 ms; snapshot kept until
    /// restore confirms).
    ClipboardRestore,
}

/// Why a session's text was held instead of injected (never lost — non-negotiable
/// #2; text is preserved in history regardless).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldReason {
    /// Focus moved to a different app mid-dictation (Pitfall P9).
    FocusChanged,
    /// Target is a password/secure field — injection refused (non-negotiable #8).
    SecureField,
    /// No injectable field had focus.
    NoTarget,
}

/// The pipeline stage a failure occurred in (for `Failed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Capture,
    Vad,
    Recognize,
    Clean,
    Inject,
    History,
}

/// Prediction surface (Whisper-Ahead; ADR-0005, ARCHITECTURE §6b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PredictionSurface {
    /// HUD dual-line (blue lane below the mic).
    HudDualLine,
    /// Inline ghost in non-overlay apps (paused-only).
    InlineGhost,
}

/// Prediction timing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PredictionMode {
    /// Continuous, ≤400 ms (HUD only).
    Streaming,
    /// After a pause (5 s HUD / 10 s inline).
    PausedMerge,
}

/// How a prediction was accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PredictionSource {
    /// The user spoke the predicted continuation (merge matcher).
    Spoken,
    /// Explicit accept (Tab = word, Shift-Tab = phrase).
    Accepted,
}

/// The single event contract. Stages emit/consume only these.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEvent {
    /// Capture started; target app bound (Pitfall P9).
    Started {
        id: SessionId,
        target_app: AppRef,
        at_ms: u64,
    },
    /// Audio is on disk *before* transcription (WAL — non-negotiable #2).
    AudioPersisted { id: SessionId, wal_path: String },
    /// Streaming partial (≤300 ms behind speech — §5).
    Partial {
        id: SessionId,
        text: String,
        t_lag_ms: u32,
    },
    /// Final verbatim ASR output.
    RawFinal { id: SessionId, text: String },
    /// Cleaned output at the selected dial.
    CleanFinal {
        id: SessionId,
        text: String,
        dial: CleanupDial,
    },
    /// Only COMMITTED text injects (ARCHITECTURE §4).
    Injected { id: SessionId, method: InjectMethod },
    /// Held instead of injected; text preserved in history.
    Held { id: SessionId, reason: HoldReason },
    /// A stage failed; text is preserved in history regardless.
    Failed {
        id: SessionId,
        stage: Stage,
        error: String,
    },
    /// Prediction offered (Whisper-Ahead).
    PredictionOffered {
        id: SessionId,
        tokens: Vec<String>,
        mode: PredictionMode,
        surface: PredictionSurface,
    },
    /// Prediction merged into the committed text.
    PredictionMerged {
        id: SessionId,
        tokens: Vec<String>,
        source: PredictionSource,
    },
    /// Prediction dismissed (mismatch, edit, timeout).
    PredictionDismissed { id: SessionId, reason: String },
    /// A live prediction went stale and should be re-guessed.
    PredictionStale { id: SessionId },
}

impl SessionEvent {
    /// The session this event belongs to. Handy on every branch, so the log/
    /// history layers never pattern-match the whole enum just to route by id.
    pub fn session_id(&self) -> SessionId {
        use SessionEvent::*;
        match *self {
            Started { id, .. }
            | AudioPersisted { id, .. }
            | Partial { id, .. }
            | RawFinal { id, .. }
            | CleanFinal { id, .. }
            | Injected { id, .. }
            | Held { id, .. }
            | Failed { id, .. }
            | PredictionOffered { id, .. }
            | PredictionMerged { id, .. }
            | PredictionDismissed { id, .. }
            | PredictionStale { id } => id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid() -> SessionId {
        SessionId::new(Ulid::from_parts(1, 2))
    }

    fn app() -> AppRef {
        AppRef {
            id: "com.apple.Safari".into(),
            name: "Safari".into(),
        }
    }

    #[test]
    fn light_is_the_default_dial() {
        assert_eq!(CleanupDial::default(), CleanupDial::Light);
    }

    #[test]
    fn session_id_routes_across_every_variant() {
        let id = sid();
        let events = vec![
            SessionEvent::Started {
                id,
                target_app: app(),
                at_ms: 0,
            },
            SessionEvent::RawFinal {
                id,
                text: "hi".into(),
            },
            SessionEvent::PredictionStale { id },
        ];
        for e in &events {
            assert_eq!(e.session_id(), id);
        }
    }

    #[test]
    fn normal_flow_sequence_serializes_round_trip() {
        let id = sid();
        let seq = vec![
            SessionEvent::Started {
                id,
                target_app: app(),
                at_ms: 10,
            },
            SessionEvent::AudioPersisted {
                id,
                wal_path: "/tmp/s.wav".into(),
            },
            SessionEvent::Partial {
                id,
                text: "hel".into(),
                t_lag_ms: 120,
            },
            SessionEvent::RawFinal {
                id,
                text: "hello".into(),
            },
            SessionEvent::CleanFinal {
                id,
                text: "Hello.".into(),
                dial: CleanupDial::Light,
            },
            SessionEvent::Injected {
                id,
                method: InjectMethod::Native,
            },
        ];
        let json = serde_json::to_string(&seq).expect("serialize");
        let back: Vec<SessionEvent> = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(seq, back);
        // Tagged representation the frontend keys on.
        assert!(json.contains("\"type\":\"started\""));
        assert!(json.contains("\"type\":\"clean_final\""));
        assert!(json.contains("\"dial\":\"light\""));
    }

    #[test]
    fn secure_field_holds_rather_than_injects() {
        let id = sid();
        let held = SessionEvent::Held {
            id,
            reason: HoldReason::SecureField,
        };
        let json = serde_json::to_string(&held).unwrap();
        assert!(json.contains("\"reason\":\"secure_field\""));
    }
}
