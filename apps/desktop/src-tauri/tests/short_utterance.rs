//! Short-utterance suite scaffold (PRD P0-1 / Pitfall P1).
//!
//! This suite proves the part of the short-utterance gate that exists today:
//! the hotkey timing contract accepts 0.3-1.5 s utterances, holds the 300 ms
//! tail, discards only sub-floor taps, and the WAL keeps short audio recoverable.
//! Real ASR golden clips stay pending until the Parakeet/Whisper adapters land.

use kaydence_lib::audio::{wal, WalCaptureRuntime};
use kaydence_lib::hotkeys::{
    Action, CaptureConfig, CaptureCoordinator, CaptureState, HotkeyMode, Signal,
};
use std::path::{Path, PathBuf};

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kaydence-short-utterance-{label}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn write_orphaned_short_wal(dir: &Path, session: &str, duration_ms: u64) -> (PathBuf, usize) {
    let sample_count = (wal::SAMPLE_RATE as u64 * duration_ms / 1_000) as usize;
    let samples = vec![0.2f32; sample_count];
    let mut writer = wal::WalWriter::create(dir, session).expect("create short wal");
    writer.append(&samples).expect("append short wal samples");
    let path = writer.path().to_path_buf();
    drop(writer); // crash/no-finalize shape: recovery must preserve the clip.
    (path, sample_count)
}

#[test]
fn short_utterance_0_3s_ptt_is_accepted_and_finalizes_after_tail() {
    let mut coordinator = CaptureCoordinator::new(HotkeyMode::PushToTalk, CaptureConfig::default());

    assert_eq!(
        coordinator.step(Signal::Press { at_ms: 0 }),
        Action::StartCapture
    );
    assert_eq!(
        coordinator.step(Signal::Release { at_ms: 300 }),
        Action::None
    );
    assert_eq!(
        coordinator.state(),
        CaptureState::Finalizing {
            started_ms: 0,
            ends_ms: 600,
        }
    );
    assert_eq!(coordinator.step(Signal::Tick { at_ms: 599 }), Action::None);
    assert_eq!(
        coordinator.step(Signal::Tick { at_ms: 600 }),
        Action::FinalizeCapture
    );
}

#[test]
fn short_utterance_1_5s_toggle_is_accepted_and_finalizes_after_tail() {
    let mut coordinator = CaptureCoordinator::new(HotkeyMode::Toggle, CaptureConfig::default());

    assert_eq!(
        coordinator.step(Signal::Press { at_ms: 0 }),
        Action::StartCapture
    );
    assert_eq!(
        coordinator.step(Signal::Release { at_ms: 120 }),
        Action::None
    );
    assert_eq!(
        coordinator.step(Signal::Press { at_ms: 1_500 }),
        Action::None
    );
    assert_eq!(
        coordinator.state(),
        CaptureState::Finalizing {
            started_ms: 0,
            ends_ms: 1_800,
        }
    );
    assert_eq!(
        coordinator.step(Signal::Tick { at_ms: 1_800 }),
        Action::FinalizeCapture
    );
}

#[test]
fn short_utterance_under_250ms_is_discarded_and_removes_wal() {
    let app_data = temp_dir("discard");
    let mut coordinator = CaptureCoordinator::new(HotkeyMode::PushToTalk, CaptureConfig::default());
    let mut runtime = WalCaptureRuntime::new_wal_only(&app_data);

    assert_eq!(
        coordinator.step(Signal::Press { at_ms: 0 }),
        Action::StartCapture
    );
    let id = runtime.start_capture(0).expect("start wal-only capture");
    let wal_path = app_data.join("sessions").join(format!("{}.wav", id.0));
    assert!(wal_path.exists());

    assert_eq!(
        coordinator.step(Signal::Release { at_ms: 249 }),
        Action::DiscardCapture
    );
    let discarded = runtime.discard_capture(249).expect("discard short tap");

    assert_eq!(discarded.id, id);
    assert!(discarded.removed);
    assert!(!wal_path.exists());
    assert!(runtime.active_session_id().is_none());
    let _ = std::fs::remove_dir_all(app_data);
}

#[test]
fn short_utterance_0_3s_wal_is_recoverable_and_not_truncated() {
    let dir = temp_dir("wal-0300");
    let (path, sample_count) = write_orphaned_short_wal(&dir, "01SHORT0300", 300);

    let recovered = wal::recover(&path).expect("recover 0.3s wal");
    let decoded = wal::read_samples(&path).expect("read 0.3s wal");

    assert_eq!(recovered.samples as usize, sample_count);
    assert_eq!(decoded.samples.len(), sample_count);
    assert!(decoded.samples.iter().any(|sample| sample.abs() > 0.01));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn short_utterance_1_5s_wal_is_recoverable_and_not_truncated() {
    let dir = temp_dir("wal-1500");
    let (path, sample_count) = write_orphaned_short_wal(&dir, "01SHORT1500", 1_500);

    let recovered = wal::recover(&path).expect("recover 1.5s wal");
    let decoded = wal::read_samples(&path).expect("read 1.5s wal");

    assert_eq!(recovered.samples as usize, sample_count);
    assert_eq!(decoded.samples.len(), sample_count);
    assert!(decoded.samples.iter().any(|sample| sample.abs() > 0.01));
    let _ = std::fs::remove_dir_all(dir);
}
