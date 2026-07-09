//! Kaydence backend library — the real product (root `AGENTS.md`).
//!
//! Wires the pipeline modules (each a P0-T3 stub for now) and the Tauri app
//! shell. The pipeline is: hotkeys -> audio(+VAD) -> engine -> cleanup ->
//! dictionary -> profiles -> inject -> history, with prediction/context as a
//! parallel local-only layer. Stages couple only through `events::SessionEvent`.

#[cfg(desktop)]
use std::path::PathBuf;
#[cfg(desktop)]
use std::sync::{Arc, Mutex};
#[cfg(desktop)]
use std::time::{Duration, Instant};

pub mod events;

pub mod audio;
pub mod cleanup;
pub mod context;
pub mod dictionary;
pub mod engine;
pub mod history;
pub mod hotkeys;
pub mod inject;
pub mod models;
pub mod pipeline;
pub mod prediction;
pub mod profiles;
pub mod settings;

/// Initial app/config snapshot for the presentation layer.
#[tauri::command]
fn app_snapshot() -> settings::AppSnapshot {
    settings::AppSnapshot::default()
}

#[cfg(desktop)]
struct HotkeyRuntime {
    coordinator: hotkeys::CaptureCoordinator,
    recorder: audio::WalCaptureRuntime,
    history: history::HistoryStore,
    target_resolver: Box<dyn profiles::ResolveSessionTarget + Send>,
}

#[cfg(desktop)]
#[derive(Debug, thiserror::Error)]
enum HotkeyRuntimeError {
    #[error("audio: {0}")]
    Audio(#[from] audio::CaptureRuntimeError),
    #[error("history: {0}")]
    History(#[from] history::HistoryError),
}

#[cfg(desktop)]
impl HotkeyRuntime {
    fn new(app_data_dir: impl Into<PathBuf>) -> Result<Self, HotkeyRuntimeError> {
        let app_data_dir = app_data_dir.into();
        let recorder = audio::WalCaptureRuntime::new(app_data_dir.clone());
        let history = history::HistoryStore::open(&app_data_dir)?;
        Ok(Self::with_recorder(
            recorder,
            history,
            Box::new(profiles::platform_target_resolver()),
        ))
    }

    #[cfg(test)]
    fn new_wal_only(app_data_dir: impl Into<PathBuf>) -> Result<Self, HotkeyRuntimeError> {
        let app_data_dir = app_data_dir.into();
        let recorder = audio::WalCaptureRuntime::new_wal_only(app_data_dir.clone());
        let history = history::HistoryStore::open(&app_data_dir)?;
        Ok(Self::with_recorder(
            recorder,
            history,
            Box::new(profiles::platform_target_resolver()),
        ))
    }

    fn with_recorder(
        recorder: audio::WalCaptureRuntime,
        history: history::HistoryStore,
        target_resolver: Box<dyn profiles::ResolveSessionTarget + Send>,
    ) -> Self {
        Self {
            coordinator: hotkeys::CaptureCoordinator::new(
                hotkeys::HotkeyMode::PushToTalk,
                hotkeys::CaptureConfig::default(),
            ),
            recorder,
            history,
            target_resolver,
        }
    }

    #[cfg(test)]
    fn with_target_resolver(
        mut self,
        target_resolver: Box<dyn profiles::ResolveSessionTarget + Send>,
    ) -> Self {
        self.target_resolver = target_resolver;
        self
    }

    fn handle_signal(
        &mut self,
        signal: hotkeys::Signal,
    ) -> Result<Option<u64>, HotkeyRuntimeError> {
        let was_finalizing = matches!(
            self.coordinator.state(),
            hotkeys::CaptureState::Finalizing { .. }
        );
        let at_ms = signal_at_ms(signal);
        let action = self.coordinator.step(signal);

        match action {
            hotkeys::Action::None => {}
            hotkeys::Action::StartCapture => {
                let id = match self.recorder.start_capture(at_ms) {
                    Ok(id) => id,
                    Err(err) => {
                        self.coordinator.reset();
                        return Err(err.into());
                    }
                };
                let target = self.target_resolver.resolve_session_target();
                let started = events::SessionEvent::Started {
                    id,
                    target_app: target.app.clone(),
                    at_ms,
                };
                if let Err(err) = self.history.record_event(&started) {
                    let _ = self.recorder.discard_capture(at_ms);
                    self.coordinator.reset();
                    return Err(err.into());
                }
                println!(
                    "Kaydence capture started: id={:?} target_app={} profile={} sessions_dir={}",
                    id,
                    target.app.name,
                    target.profile.id,
                    self.recorder.sessions_dir().display()
                );
            }
            hotkeys::Action::FinalizeCapture => {
                let summary = self.recorder.finalize_capture(at_ms)?;
                let event = summary.audio_persisted_event();
                self.history.record_event(&event)?;
                println!(
                    "Kaydence audio persisted: event={:?} samples={} started_ms={} finalized_ms={}",
                    event, summary.samples_written, summary.started_ms, summary.finalized_ms
                );
            }
            hotkeys::Action::DiscardCapture => {
                let discarded = self.recorder.discard_capture(at_ms)?;
                self.history.delete_session(discarded.id)?;
                println!(
                    "Kaydence capture discarded: id={:?} path={} started_ms={} discarded_ms={} removed={}",
                    discarded.id,
                    discarded.wal_path.display(),
                    discarded.started_ms,
                    discarded.discarded_ms,
                    discarded.removed
                );
            }
        }

        if !was_finalizing {
            if let hotkeys::CaptureState::Finalizing { ends_ms, .. } = self.coordinator.state() {
                return Ok(Some(ends_ms));
            }
        }
        Ok(None)
    }
}

#[cfg(desktop)]
fn signal_at_ms(signal: hotkeys::Signal) -> u64 {
    match signal {
        hotkeys::Signal::Press { at_ms }
        | hotkeys::Signal::Release { at_ms }
        | hotkeys::Signal::Tick { at_ms } => at_ms,
    }
}

#[cfg(desktop)]
fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(desktop)]
fn schedule_tail_tick(runtime: &Arc<Mutex<HotkeyRuntime>>, started: Instant, ends_ms: u64) {
    let runtime = Arc::clone(runtime);
    std::thread::spawn(move || {
        let sleep_ms = ends_ms.saturating_sub(elapsed_ms(started));
        if sleep_ms > 0 {
            std::thread::sleep(Duration::from_millis(sleep_ms));
        }
        let at_ms = elapsed_ms(started);
        match runtime.lock() {
            Ok(mut runtime) => {
                if let Err(err) = runtime.handle_signal(hotkeys::Signal::Tick { at_ms }) {
                    eprintln!("Kaydence hotkey tail tick failed: {err}");
                }
            }
            Err(_) => {
                eprintln!("Kaydence hotkey runtime lock poisoned during tail tick");
            }
        }
    });
}

#[cfg(desktop)]
fn install_global_hotkey(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use hotkeys::Signal;
    use tauri::Manager;
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

    let shortcut = Shortcut::new(None, Code::AltRight);
    let app_data_dir = app.path().app_data_dir()?;
    let runtime = Arc::new(Mutex::new(HotkeyRuntime::new(app_data_dir)?));
    let started = Instant::now();
    let handler_runtime = Arc::clone(&runtime);

    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, observed, event| {
                if observed != &shortcut {
                    return;
                }

                let at_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                let signal = match event.state() {
                    ShortcutState::Pressed => Signal::Press { at_ms },
                    ShortcutState::Released => Signal::Release { at_ms },
                };

                let tail_wake_ms = match handler_runtime.lock() {
                    Ok(mut runtime) => match runtime.handle_signal(signal) {
                        Ok(tail_wake_ms) => tail_wake_ms,
                        Err(err) => {
                            eprintln!("Kaydence hotkey runtime failed: {err}");
                            None
                        }
                    },
                    Err(_) => {
                        eprintln!("Kaydence hotkey runtime lock poisoned");
                        None
                    }
                };

                if let Some(ends_ms) = tail_wake_ms {
                    schedule_tail_tick(&handler_runtime, started, ends_ms);
                }
            })
            .build(),
    )?;

    app.global_shortcut().register(shortcut)?;
    Ok(())
}

/// Run the Kaydence desktop app. Called by the thin `main.rs` binary.
///
/// P0 scaffold: brings up the Tauri window only. Pipeline wiring lands in P1.
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_snapshot])
        .setup(|app| {
            #[cfg(desktop)]
            install_global_hotkey(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Kaydence");
}

#[cfg(all(test, desktop))]
mod tests {
    use super::*;

    fn tmp() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kaydence-hotkey-runtime-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn detected_app() -> events::AppRef {
        events::AppRef {
            id: "com.example.editor".to_string(),
            name: "Example Editor".to_string(),
        }
    }

    #[test]
    fn hotkey_runtime_finalizes_wal_after_tail_tick() {
        let app_data = tmp();
        let mut runtime = HotkeyRuntime::new_wal_only(&app_data)
            .unwrap()
            .with_target_resolver(Box::new(profiles::SessionTargetResolver::new(
                profiles::StaticFrontmostAppDetector::new(detected_app()),
                profiles::ProfileStore::default(),
            )));

        assert_eq!(
            runtime
                .handle_signal(hotkeys::Signal::Press { at_ms: 0 })
                .unwrap(),
            None
        );
        let id = runtime.recorder.active_session_id().unwrap();
        assert_eq!(
            runtime
                .handle_signal(hotkeys::Signal::Release { at_ms: 400 })
                .unwrap(),
            Some(700)
        );
        assert_eq!(
            runtime
                .handle_signal(hotkeys::Signal::Tick { at_ms: 700 })
                .unwrap(),
            None
        );

        let path = app_data.join("sessions").join(format!("{}.wav", id.0));
        assert!(runtime.recorder.active_session_id().is_none());
        assert!(path.exists());
        assert_eq!(&std::fs::read(&path).unwrap()[0..4], b"RIFF");
        assert!(app_data.join(history::HISTORY_DB_FILE).exists());

        let session = runtime.history.get_session(id).unwrap().unwrap();
        assert_eq!(session.target_app, Some(detected_app()));
        assert_eq!(session.audio_path.as_deref(), Some(path.to_str().unwrap()));
        assert_eq!(session.event_count, 2);
        assert_eq!(
            runtime.history.events_for_session(id).unwrap(),
            vec![
                events::SessionEvent::Started {
                    id,
                    target_app: detected_app(),
                    at_ms: 0,
                },
                audio::CaptureSessionSummary {
                    id,
                    wal_path: path,
                    samples_written: 0,
                    dropped_input_samples: 0,
                    started_ms: 0,
                    finalized_ms: 700
                }
                .audio_persisted_event(),
            ]
        );
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn hotkey_runtime_discards_short_tap_wal() {
        let app_data = tmp();
        let mut runtime = HotkeyRuntime::new_wal_only(&app_data).unwrap();

        runtime
            .handle_signal(hotkeys::Signal::Press { at_ms: 0 })
            .unwrap();
        let id = runtime.recorder.active_session_id().unwrap();
        let path = app_data.join("sessions").join(format!("{}.wav", id.0));
        assert!(path.exists());

        assert_eq!(
            runtime
                .handle_signal(hotkeys::Signal::Release { at_ms: 100 })
                .unwrap(),
            None
        );

        assert!(runtime.recorder.active_session_id().is_none());
        assert!(!path.exists());
        assert!(runtime.history.get_session(id).unwrap().is_none());
        assert!(runtime.history.events_for_session(id).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(app_data);
    }
}
