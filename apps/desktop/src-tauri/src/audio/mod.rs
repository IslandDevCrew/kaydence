//! Capture: mic -> lock-free ring buffer -> WAL file -> Silero VAD. Emits
//! AudioPersisted (non-negotiable #2).
//!
//! `wal` is real (P1, PRD P0-4): write-ahead persistence + crash recovery.
//! Capture (cpal) and VAD land next; the cpal callback only feeds the ring
//! buffer (invariant 1) — the capture task drains it into the WAL.
#![allow(dead_code)]

pub mod wal;

use crate::events::{SessionEvent, SessionId};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use ulid::Ulid;

/// Metadata for a capture whose WAL was finalized and is ready for downstream
/// recognition/history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureSessionSummary {
    pub id: SessionId,
    pub wal_path: PathBuf,
    pub samples_written: u64,
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
}

struct ActiveWalSession {
    id: SessionId,
    writer: wal::WalWriter,
    started_ms: u64,
}

/// Minimal runtime owner for P1 hotkey capture: one active dictation maps to one
/// app-data `sessions/{ulid}.wav` WAL. Mic draining lands next; this already
/// enforces the session file lifecycle the OS shortcut feeds.
pub struct WalCaptureRuntime {
    sessions_dir: PathBuf,
    active: Option<ActiveWalSession>,
}

impl WalCaptureRuntime {
    pub fn new(app_data_dir: impl Into<PathBuf>) -> Self {
        Self {
            sessions_dir: app_data_dir.into().join("sessions"),
            active: None,
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

        let id = SessionId::new(Ulid::new());
        let writer = wal::WalWriter::create(&self.sessions_dir, &id.0.to_string())?;
        self.active = Some(ActiveWalSession {
            id,
            writer,
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

        let samples_written = active.writer.samples_written();
        let wal_path = active.writer.finalize()?;
        Ok(CaptureSessionSummary {
            id: active.id,
            wal_path,
            samples_written,
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

        let wal_path = active.writer.path().to_path_buf();
        drop(active.writer);
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
            removed,
        })
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

    #[test]
    fn start_capture_creates_a_wal_session_under_app_data() {
        let app_data = tmp();
        let mut runtime = WalCaptureRuntime::new(&app_data);

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
        let mut runtime = WalCaptureRuntime::new(&app_data);
        let id = runtime.start_capture(10).unwrap();

        let summary = runtime.finalize_capture(400).unwrap();

        assert_eq!(summary.id, id);
        assert_eq!(summary.started_ms, 10);
        assert_eq!(summary.finalized_ms, 400);
        assert_eq!(summary.samples_written, 0);
        assert!(runtime.active_session_id().is_none());
        let event = summary.audio_persisted_event();
        assert_eq!(event.session_id(), id);
        assert!(summary.wal_path.exists());
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn discard_capture_removes_the_short_tap_wal() {
        let app_data = tmp();
        let mut runtime = WalCaptureRuntime::new(&app_data);
        let id = runtime.start_capture(0).unwrap();
        let path = app_data.join("sessions").join(format!("{}.wav", id.0));
        assert!(path.exists());

        let discarded = runtime.discard_capture(100).unwrap();

        assert_eq!(discarded.id, id);
        assert!(discarded.removed);
        assert!(!path.exists());
        assert!(runtime.active_session_id().is_none());
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn double_start_is_refused_while_wal_is_active() {
        let app_data = tmp();
        let mut runtime = WalCaptureRuntime::new(&app_data);
        let id = runtime.start_capture(0).unwrap();

        let err = runtime.start_capture(10).unwrap_err();

        assert!(matches!(err, CaptureRuntimeError::AlreadyActive(active) if active == id));
        let _ = std::fs::remove_dir_all(app_data);
    }
}
