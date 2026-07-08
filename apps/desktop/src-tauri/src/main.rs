// Prevent an extra console window on Windows in release (root §5/§7 polish).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    kaydence_lib::run();
}
