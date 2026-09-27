//! `kaydence-ctl` — the compositor-binding client (ADR-0023), in the spirit of
//! `hyprctl` / `swaymsg`.
//!
//!   kaydence-ctl record start|stop|toggle|status
//!
//! Same grammar and code path as `kaydence record …`, but this binary links no
//! GUI stack: the full app loads ~145 shared libraries (GTK/WebKit) before
//! `main`, costing ~74 ms per exec on the Omarchy reference box — more than the
//! whole 50 ms hotkey→capture budget (root §5). This client execs in ~2 ms, so
//! compositor keybindings should always call it.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = kaydence_lib::hotkeys::control::cli_main(&args).unwrap_or_else(|| {
        let bin = args.first().map(String::as_str).unwrap_or("kaydence-ctl");
        eprint!("{}", kaydence_lib::hotkeys::control::usage(bin));
        kaydence_lib::hotkeys::control::exit::USAGE
    });
    std::process::exit(code);
}
