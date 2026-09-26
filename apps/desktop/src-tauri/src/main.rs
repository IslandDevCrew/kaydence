// Prevent an extra console window on Windows in release (root §5/§7 polish).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `record start|stop|toggle|status`: a compositor keybinding talking to the
    // running app over its local control socket (Wayland hotkey, ADR-0023).
    // Handled before any GUI/runtime setup so a keypress costs one short exec.
    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = kaydence_lib::hotkeys::control::cli_main(&args) {
        std::process::exit(code);
    }

    if std::env::args().any(|arg| arg == "--bench") {
        match kaydence_lib::run_bench_json() {
            Ok(json) => println!("{json}"),
            Err(err) => {
                eprintln!("Kaydence bench failed: {err}");
                std::process::exit(2);
            }
        }
        return;
    }

    kaydence_lib::run();
}
