//! UIA injection self-test (P1-P0-3, ADR-0013) — run on a real Windows desktop
//! to validate the injection path live. Windows-only; a no-op elsewhere.
//!
//! Modes (each starts with a countdown so the operator can focus the target):
//!   uia-selftest --probe [secs]           report focused element + FieldKind + caps
//!   uia-selftest --inject [secs] [text]   run the REAL production path
//!                                         (inject_committed_text: policy gate →
//!                                         plan selection → backend) and print
//!                                         the resulting SessionEvent
//!   uia-selftest --synth [secs] [text]    force the SendInput keystroke rung
//!   uia-selftest --paste [secs] [text]    force the clipboard snapshot→paste→
//!                                         restore rung (both keep the backend's
//!                                         own secure-field defense in depth)
//!
//! Focus Notepad → expect Injected{native|keystroke}. Focus a password field →
//! expect Held{SecureField} (the load-bearing refusal, non-negotiable #8).

fn main() {
    #[cfg(target_os = "windows")]
    {
        let args: Vec<String> = std::env::args().collect();
        let secs = |i: usize, default| {
            args.get(i)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(default)
        };
        match args.get(1).map(String::as_str) {
            Some(mode @ ("--inject" | "--synth" | "--paste")) => {
                let text = args
                    .get(3)
                    .cloned()
                    .unwrap_or_else(|| "[Kaydence UIA \u{2713}] ".into());
                match mode {
                    "--synth" => win::run_direct(secs(2, 5), &text, false),
                    "--paste" => win::run_direct(secs(2, 5), &text, true),
                    _ => win::run_inject(secs(2, 5), &text),
                }
            }
            _ => win::run_probe(secs(2, 5)),
        }
    }
    #[cfg(not(target_os = "windows"))]
    eprintln!("uia-selftest is Windows-only.");
}

#[cfg(target_os = "windows")]
mod win {
    use kaydence_lib::events::SessionId;
    use kaydence_lib::inject::windows::{foreground_app, WindowsTextInjector};
    use kaydence_lib::inject::{
        inject_committed_text, FieldKind, TextInjector, UnknownFieldPolicy,
    };

    fn countdown(secs: u64) {
        println!("[uia-selftest] focus the target field — acting in {secs}s...");
        for remaining in (1..=secs).rev() {
            println!("  {remaining}...");
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    fn report_focus(injector: &WindowsTextInjector) -> FieldKind {
        match foreground_app() {
            Some(app) => println!(
                "[focus] foreground app: id={:?} name={:?}",
                app.id, app.name
            ),
            None => println!("[focus] foreground app: <none>"),
        }
        let kind = injector.focused_field();
        let caps = injector.caps();
        println!("[focus] field kind: {kind:?}");
        println!(
            "[focus] caps: native={} keystroke={:?} clipboard={}",
            caps.native_text_insert, caps.keystroke, caps.clipboard
        );
        kind
    }

    pub fn run_probe(secs: u64) {
        countdown(secs);
        let injector = WindowsTextInjector;
        report_focus(&injector);
    }

    /// Force one fallback rung directly (`synth_text` or `paste_clipboard`) to
    /// validate it even where the ladder would pick native first. The backend's
    /// own secure-field defense in depth still applies.
    pub fn run_direct(secs: u64, text: &str, paste: bool) {
        countdown(secs);
        let mut injector = WindowsTextInjector;
        report_focus(&injector);
        let (rung, result) = if paste {
            ("paste_clipboard", injector.paste_clipboard(text))
        } else {
            ("synth_text", injector.synth_text(text))
        };
        match result {
            Ok(()) => println!("[{rung}] OK — check the target for {text:?}."),
            Err(e) => println!("[{rung}] REFUSED/FAILED: {}", e.0),
        }
    }

    /// The real integrated path: decide_secure → select_plan → backend. The
    /// printed SessionEvent is the wire-level truth (Injected/Held/Failed).
    pub fn run_inject(secs: u64, text: &str) {
        countdown(secs);
        let mut injector = WindowsTextInjector;
        let kind = report_focus(&injector);
        let id = SessionId(ulid::Ulid::new());
        let event =
            inject_committed_text(&mut injector, id, text, UnknownFieldPolicy::Lenient, false);
        println!("[inject] SessionEvent: {event:?}");
        match kind {
            FieldKind::Secure => println!(
                "[inject] expected: Held(SecureField) — refusal is the PASS condition here."
            ),
            FieldKind::Editable => {
                println!("[inject] expected: Injected — check the target for {text:?}.")
            }
            _ => println!("[inject] expected: Held/Failed (no editable target)."),
        }
    }
}
