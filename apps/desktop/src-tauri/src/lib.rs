//! Kaydence backend library — the real product (root `AGENTS.md`).
//!
//! Wires the pipeline modules (each a P0-T3 stub for now) and the Tauri app
//! shell. The pipeline is: hotkeys -> audio(+VAD) -> engine -> cleanup ->
//! dictionary -> profiles -> inject -> history, with prediction/context as a
//! parallel local-only layer. Stages couple only through `events::SessionEvent`.

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

/// Run the Kaydence desktop app. Called by the thin `main.rs` binary.
///
/// P0 scaffold: brings up the Tauri window only. Pipeline wiring lands in P1.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running Kaydence");
}
