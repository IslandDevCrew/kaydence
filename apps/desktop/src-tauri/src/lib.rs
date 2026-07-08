//! Kaydence backend library — the real product (root `AGENTS.md`).
//!
//! Wires the pipeline modules (each a P0-T3 stub for now) and the Tauri app
//! shell. The pipeline is: hotkeys -> audio(+VAD) -> engine -> cleanup ->
//! dictionary -> profiles -> inject -> history, with prediction/context as a
//! parallel local-only layer. Stages couple only through `events::SessionEvent`.

#[cfg(desktop)]
use std::sync::{Arc, Mutex};
#[cfg(desktop)]
use std::time::Instant;

pub mod events;

pub mod audio;
pub mod cleanup;
pub mod context;
pub mod dictionary;
pub mod engine;
pub mod history;
pub mod hotkeys;
pub mod inject;
pub mod prediction;
pub mod profiles;
pub mod settings;

/// Initial app/config snapshot for the presentation layer.
#[tauri::command]
fn app_snapshot() -> settings::AppSnapshot {
    settings::AppSnapshot::default()
}

#[cfg(desktop)]
fn install_global_hotkey(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use hotkeys::{CaptureConfig, CaptureCoordinator, HotkeyMode, Signal};
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

    let shortcut = Shortcut::new(None, Code::AltRight);
    let coordinator = Arc::new(Mutex::new(CaptureCoordinator::new(
        HotkeyMode::PushToTalk,
        CaptureConfig::default(),
    )));
    let started = Instant::now();
    let handler_coordinator = Arc::clone(&coordinator);

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

                match handler_coordinator.lock() {
                    Ok(mut coordinator) => {
                        let action = coordinator.step(signal);
                        println!("Kaydence hotkey action: {action:?}");
                    }
                    Err(_) => {
                        eprintln!("Kaydence hotkey coordinator lock poisoned");
                    }
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
