//! Wayland/Hyprland injection self-test (ADR-0023) — the Omarchy twin of
//! `atspi-selftest`. Linux-only; a no-op elsewhere.
//!
//!   wayland-selftest --probe
//!       Print the Hyprland focused window + whether a virtual keyboard can be
//!       created in this session.
//!
//!   wayland-selftest --type-into <hyprland-window-address> <text>
//!       Focus that window through Hyprland IPC, RE-VERIFY it is the focused
//!       window (the Pitfall-P9 focus gate — if anything else has focus we
//!       refuse to type), type <text> through `zwp_virtual_keyboard_v1`, then
//!       hand focus back to the window that had it before. Harness: see
//!       `scripts/linux-wayland-inject-proof.sh`.

fn main() {
    #[cfg(target_os = "linux")]
    std::process::exit(linux::run(std::env::args().collect()));
    #[cfg(not(target_os = "linux"))]
    eprintln!("wayland-selftest is Linux-only.");
}

#[cfg(target_os = "linux")]
mod linux {
    use kaydence_lib::inject::hyprland;
    use kaydence_lib::inject::wayland_vk::VirtualKeyboard;
    use std::time::{Duration, Instant};

    pub fn run(args: Vec<String>) -> i32 {
        match args.get(1).map(String::as_str) {
            Some("--probe") => probe(),
            Some("--type-into") if args.len() == 4 => type_into(&args[2], &args[3]),
            _ => {
                eprintln!("usage: wayland-selftest --probe | --type-into <window-address> <text>");
                64
            }
        }
    }

    fn probe() -> i32 {
        println!("[probe] hyprland session: {}", hyprland::detected());
        match hyprland::active_window() {
            Some(w) => println!(
                "[probe] focused window: class={:?} pid={:?} xwayland={} app_ref={:?}",
                w.class,
                w.pid,
                w.xwayland,
                w.app_ref()
            ),
            None => println!("[probe] focused window: none reported"),
        }
        let t = Instant::now();
        match VirtualKeyboard::connect() {
            Ok(_) => {
                println!(
                    "[probe] zwp_virtual_keyboard_v1: AVAILABLE (connect+create {} µs)",
                    t.elapsed().as_micros()
                );
                0
            }
            Err(e) => {
                println!("[probe] zwp_virtual_keyboard_v1: unavailable — {e}");
                1
            }
        }
    }

    fn focused_address() -> Option<String> {
        hyprland::active_window().map(|w| w.address)
    }

    /// Ask Hyprland to focus a window. Hyprland ≥ 0.55 in Lua-config mode (Omarchy
    /// 4) evaluates `dispatch` arguments as Lua; older/hyprlang sessions take the
    /// legacy dispatcher string. Try Lua first, as `omarchy-launch-or-focus` does.
    fn focus_window(address: &str) {
        let lua = format!("dispatch hl.dsp.focus({{ window = \"address:{address}\" }})");
        let ok = hyprland::request(&lua).is_some_and(|reply| reply.trim() == "ok");
        if !ok {
            let _ = hyprland::request(&format!("dispatch focuswindow address:{address}"));
        }
    }

    fn type_into(address: &str, text: &str) -> i32 {
        if !address.starts_with("0x") || !address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
            eprintln!("[type] refusing: {address:?} is not a Hyprland window address");
            return 64;
        }
        let previous = focused_address();
        focus_window(address);

        // Wait (bounded) for the compositor to report the target as focused.
        let deadline = Instant::now() + Duration::from_millis(1500);
        while focused_address().as_deref() != Some(address) {
            if Instant::now() > deadline {
                eprintln!("[type] REFUSED: target never took focus — nothing typed (P9 gate)");
                return 3;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        let mut kb = match VirtualKeyboard::connect() {
            Ok(kb) => kb,
            Err(e) => {
                eprintln!("[type] virtual keyboard unavailable: {e}");
                return 1;
            }
        };
        // Final gate immediately before the first key.
        if focused_address().as_deref() != Some(address) {
            eprintln!("[type] REFUSED: focus moved before typing — nothing typed (P9 gate)");
            return 3;
        }
        let started = Instant::now();
        let result = kb.type_text(text);
        let elapsed = started.elapsed();
        drop(kb);

        if let Some(prev) = previous.filter(|p| p != address) {
            focus_window(&prev);
        }
        match result {
            Ok(n) => {
                println!(
                    "[type] typed {n} chars ({} bytes) into {address} in {} ms via zwp_virtual_keyboard_v1",
                    text.len(),
                    elapsed.as_millis()
                );
                0
            }
            Err(e) => {
                eprintln!("[type] FAILED: {e}");
                1
            }
        }
    }
}
