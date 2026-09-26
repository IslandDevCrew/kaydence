//! Local control socket + `record` CLI (ADR-0023) — the Wayland hotkey.
//!
//! Wayland gives no client a global key grab; the compositor owns the keyboard.
//! What every Wayland desktop *does* offer is "run a command when this key is
//! pressed" (Hyprland/Sway also on release). So on Linux the hotkey is:
//!
//! ```text
//!   compositor bind ─► `<app> record start|stop|toggle|status` (short-lived CLI)
//!                      └─► $XDG_RUNTIME_DIR/<app>/control.sock ─► running app
//!                          └─► hotkeys::control_signal → the SAME Press/Release
//!                              edges the X11/macOS/Windows grab feeds the coordinator
//! ```
//!
//! Security model (local IPC — not a network surface; nothing persisted):
//! - The socket lives in `$XDG_RUNTIME_DIR` (per-user tmpfs, 0700 by spec) inside
//!   our own 0700 directory; the socket file is 0600.
//! - Every connection's peer uid is checked with `SO_PEERCRED`; other users are
//!   refused even if permissions were loosened.
//! - The protocol carries one verb in and one state word out. It can start/stop a
//!   capture (exactly what the physical hotkey can do) and read idle/recording/
//!   finalizing. It never carries or returns audio or transcript text.
//! - Connections are handled sequentially on one thread, so a press and its
//!   release can never be reordered (Pitfall P1's single serialized owner).
#![allow(dead_code)]

use super::ControlCommand;

/// Max bytes read from one request; the longest valid verb is 6 bytes.
const MAX_REQUEST_BYTES: u64 = 64;

/// CLI usage text. `bin` is the invoked program name.
pub fn usage(bin: &str) -> String {
    let ctl = ctl_command();
    format!(
        "usage: {bin} record <start|stop|toggle|status>\n\
         \n\
         Drive dictation from a compositor keybinding (Wayland has no global key grab).\n\
         Bindings should call `{ctl}`: it links no GUI stack, so a keypress costs ~2 ms.\n\
         Omarchy / Hyprland push-to-talk (hold F10; ~/.config/hypr/bindings.lua):\n\
         \x20 o.bind(\"F10\", \"Kaydence: start dictation\", \"{ctl} record start\")\n\
         \x20 o.bind(\"F10\", \"Kaydence: stop dictation\", \"{ctl} record stop\", {{ release = true }})\n\
         One-key toggle (GNOME/KDE custom shortcut): {ctl} record toggle\n\
         Status bar: {ctl} record status  →  idle | recording | finalizing\n"
    )
}

/// The lightweight binding client (`src/bin/kaydence-ctl.rs`).
pub fn ctl_command() -> String {
    format!("{}-ctl", crate::settings::APP_NAME.to_lowercase())
}

/// What a CLI invocation asks for, parsed from argv. Pure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliRequest {
    /// Not a control invocation — start the app normally.
    NotControl,
    /// `record <verb>` with a valid verb.
    Command(ControlCommand),
    /// `record` with a missing/unknown verb, or `record --help`.
    Usage { error: bool },
}

/// Parse argv (including argv[0]). Only `record …` is ours; everything else
/// (including Tauri's own flags) passes through untouched. Pure.
pub fn parse_cli(args: &[String]) -> CliRequest {
    match args.get(1).map(String::as_str) {
        Some("record") => match args.get(2).map(String::as_str) {
            Some("-h" | "--help" | "help") => CliRequest::Usage { error: false },
            Some(verb) if args.len() == 3 => match ControlCommand::parse(verb) {
                Some(cmd) => CliRequest::Command(cmd),
                None => CliRequest::Usage { error: true },
            },
            _ => CliRequest::Usage { error: true },
        },
        _ => CliRequest::NotControl,
    }
}

/// Parse one wire request line. Pure.
pub fn parse_request(line: &str) -> Option<ControlCommand> {
    ControlCommand::parse(line.lines().next().unwrap_or_default())
}

/// The reply the server sends. `Ok(state)` → `ok <state>`; `Err(reason)` →
/// `err <reason>`. Pure.
pub fn render_reply(reply: &Result<&str, &str>) -> String {
    match reply {
        Ok(state) => format!("ok {state}\n"),
        Err(reason) => format!("err {reason}\n"),
    }
}

/// Interpret a server reply for the CLI: `(exit_code, text_to_print)`. Pure.
pub fn interpret_reply(reply: &str) -> (i32, String) {
    let line = reply.lines().next().unwrap_or_default().trim();
    if let Some(state) = line.strip_prefix("ok ") {
        (0, state.to_string())
    } else if let Some(reason) = line.strip_prefix("err ") {
        (1, format!("error: {reason}"))
    } else {
        (1, "error: malformed reply from the running app".to_string())
    }
}

/// Exit codes for the CLI (sysexits-style where one fits).
pub mod exit {
    pub const OK: i32 = 0;
    pub const REFUSED: i32 = 1;
    /// The app is not running (no live socket).
    pub const NOT_RUNNING: i32 = 2;
    pub const USAGE: i32 = 64;
    pub const UNSUPPORTED: i32 = 69;
}

/// Entry point for `main.rs`: `Some(exit_code)` when argv was a control
/// invocation (the caller exits without starting the GUI), `None` otherwise.
pub fn cli_main(args: &[String]) -> Option<i32> {
    let bin = args
        .first()
        .and_then(|a| std::path::Path::new(a).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("kaydence")
        .to_string();
    match parse_cli(args) {
        CliRequest::NotControl => None,
        CliRequest::Usage { error } => {
            if error {
                eprint!("{}", usage(&bin));
                Some(exit::USAGE)
            } else {
                print!("{}", usage(&bin));
                Some(exit::OK)
            }
        }
        CliRequest::Command(cmd) => Some(platform::send(cmd)),
    }
}

#[cfg(target_os = "linux")]
pub use platform::{socket_path, ControlError, ControlServer};

/// Hard ceiling on a capture started through the control socket. A compositor
/// can lose a key-release (focus/VT switch, a modifier lifted first on a
/// modified release bind), so the socket path carries the same 5-minute safety
/// stop hotkeys invariant 2 requires for toggle mode.
pub const CONTROL_CAPTURE_LIMIT_MS: u64 = 5 * 60 * 1000;

/// Whether the compositor-bound hotkey applies here, and if so whether a
/// binding for this app exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositorHotkey {
    /// Not a Wayland session (macOS, Windows, X11): the native grab is the hotkey.
    NotApplicable,
    /// Wayland + a compositor binding that runs our `record` command was found.
    Bound,
    /// Wayland + the compositor can be queried (Hyprland) but has no binding.
    NotBound,
    /// Wayland on a compositor we cannot query (GNOME/KDE/Sway).
    Unverifiable,
}

/// Classify the compositor-hotkey situation from session facts. Pure.
pub fn classify_compositor_hotkey(
    wayland_session: bool,
    hyprland_binds: Option<&str>,
    needle: &str,
) -> CompositorHotkey {
    if !wayland_session {
        return CompositorHotkey::NotApplicable;
    }
    match hyprland_binds {
        Some(binds) if crate::inject::hyprland::binds_mention(binds, needle) => {
            CompositorHotkey::Bound
        }
        Some(_) => CompositorHotkey::NotBound,
        None => CompositorHotkey::Unverifiable,
    }
}

/// The live compositor-hotkey status for this session.
pub fn compositor_hotkey_status() -> CompositorHotkey {
    #[cfg(target_os = "linux")]
    {
        let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
        let binds = crate::inject::hyprland::binds_json();
        classify_compositor_hotkey(
            wayland,
            binds.as_deref(),
            &crate::settings::APP_NAME.to_lowercase(),
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        CompositorHotkey::NotApplicable
    }
}

/// Plain-language guidance when a Wayland session has no working hotkey yet.
pub fn wayland_hotkey_guidance(status: CompositorHotkey) -> Option<String> {
    let bin = ctl_command();
    match status {
        CompositorHotkey::NotBound => Some(format!(
            "Wayland: apps cannot grab global keys. Bind `{bin} record start` (press) and \
             `{bin} record stop` (release) in ~/.config/hypr/bindings.lua — see \
             docs/platforms/OMARCHY.md."
        )),
        CompositorHotkey::Unverifiable => Some(format!(
            "Wayland: apps cannot grab global keys. Add a custom shortcut in your desktop \
             settings that runs `{bin} record toggle`."
        )),
        CompositorHotkey::NotApplicable | CompositorHotkey::Bound => None,
    }
}

/// Start the control socket for the life of the process where it applies
/// (Linux). `Ok(Some(path))` when serving, `Ok(None)` on platforms that use a
/// native hotkey grab instead.
pub fn serve<F>(handler: F) -> Result<Option<String>, String>
where
    F: Fn(ControlCommand) -> Result<&'static str, &'static str> + Send + Sync + 'static,
{
    #[cfg(target_os = "linux")]
    {
        static SERVER: std::sync::OnceLock<ControlServer> = std::sync::OnceLock::new();
        if let Some(server) = SERVER.get() {
            return Ok(Some(server.path().display().to_string()));
        }
        let server = ControlServer::spawn(handler).map_err(|e| e.to_string())?;
        let path = server.path().display().to_string();
        let _ = SERVER.set(server);
        Ok(Some(path))
    }
    #[cfg(not(target_os = "linux"))]
    {
        drop(handler);
        Ok(None)
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::{exit, interpret_reply, parse_request, render_reply, MAX_REQUEST_BYTES};
    use crate::hotkeys::ControlCommand;
    use crate::settings::APP_NAME;
    use std::fs::{self, DirBuilder};
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::thread::JoinHandle;
    use std::time::Duration;

    const CLIENT_TIMEOUT: Duration = Duration::from_secs(2);
    const SERVER_IO_TIMEOUT: Duration = Duration::from_millis(500);

    #[derive(Debug, thiserror::Error)]
    pub enum ControlError {
        #[error("XDG_RUNTIME_DIR is not set; the control socket needs a per-user runtime dir")]
        NoRuntimeDir,
        #[error("control directory {0} is not a private directory owned by this user")]
        UnsafeDir(PathBuf),
        #[error("another instance already owns the control socket at {0}")]
        AlreadyRunning(PathBuf),
        #[error("control socket I/O: {0}")]
        Io(#[from] std::io::Error),
    }

    /// `$XDG_RUNTIME_DIR/<app>/control.sock`. `None` without a runtime dir.
    pub fn socket_path() -> Option<PathBuf> {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR").filter(|d| !d.is_empty())?;
        Some(
            PathBuf::from(runtime)
                .join(APP_NAME.to_lowercase())
                .join("control.sock"),
        )
    }

    /// Create (or validate) our private directory: a real directory, owned by
    /// us, with no group/other access. Never chmods a directory it did not make.
    fn ensure_private_dir(dir: &Path) -> Result<(), ControlError> {
        match fs::symlink_metadata(dir) {
            Ok(meta) => {
                let uid = rustix::process::getuid().as_raw();
                if !meta.is_dir() || meta.uid() != uid || meta.mode() & 0o077 != 0 {
                    return Err(ControlError::UnsafeDir(dir.to_path_buf()));
                }
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                DirBuilder::new().mode(0o700).create(dir)?;
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// True when the peer on `stream` runs as this same user.
    fn same_user(stream: &UnixStream) -> bool {
        match rustix::net::sockopt::socket_peercred(stream) {
            Ok(cred) => cred.uid == rustix::process::getuid(),
            Err(_) => false,
        }
    }

    type Handler = Arc<dyn Fn(ControlCommand) -> Result<&'static str, &'static str> + Send + Sync>;

    /// The running app's end: owns the socket file for its lifetime.
    pub struct ControlServer {
        path: PathBuf,
        _thread: JoinHandle<()>,
    }

    impl ControlServer {
        /// Bind the socket and serve `handler` on a background thread. Refuses
        /// to steal a live socket (a second app instance), but reclaims a stale
        /// one left by a crash.
        pub fn spawn<F>(handler: F) -> Result<Self, ControlError>
        where
            F: Fn(ControlCommand) -> Result<&'static str, &'static str> + Send + Sync + 'static,
        {
            let path = socket_path().ok_or(ControlError::NoRuntimeDir)?;
            Self::spawn_at(path, Arc::new(handler))
        }

        fn spawn_at(path: PathBuf, handler: Handler) -> Result<Self, ControlError> {
            if let Some(dir) = path.parent() {
                ensure_private_dir(dir)?;
            }
            if fs::symlink_metadata(&path).is_ok() {
                if UnixStream::connect(&path).is_ok() {
                    return Err(ControlError::AlreadyRunning(path));
                }
                fs::remove_file(&path)?; // stale socket from a crashed run
            }
            let listener = UnixListener::bind(&path)?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
            let thread = std::thread::Builder::new()
                .name("control-socket".into())
                .spawn(move || serve(listener, handler))?;
            Ok(Self {
                path,
                _thread: thread,
            })
        }

        pub fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for ControlServer {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    fn serve(listener: UnixListener, handler: Handler) {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            // One request per connection, handled in arrival order.
            if let Err(err) = handle(stream, &handler) {
                eprintln!("{APP_NAME} control socket request failed: {err}");
            }
        }
    }

    fn handle(mut stream: UnixStream, handler: &Handler) -> std::io::Result<()> {
        stream.set_read_timeout(Some(SERVER_IO_TIMEOUT))?;
        stream.set_write_timeout(Some(SERVER_IO_TIMEOUT))?;
        if !same_user(&stream) {
            stream.write_all(render_reply(&Err("forbidden")).as_bytes())?;
            return Ok(());
        }
        let mut line = String::new();
        BufReader::new((&stream).take(MAX_REQUEST_BYTES)).read_line(&mut line)?;
        let reply = match parse_request(&line) {
            Some(cmd) => handler(cmd),
            None => Err("unknown-command"),
        };
        stream.write_all(render_reply(&reply).as_bytes())
    }

    /// CLI side: send one command to the running app and print its answer.
    pub fn send(cmd: ControlCommand) -> i32 {
        let Some(path) = socket_path() else {
            eprintln!("error: XDG_RUNTIME_DIR is not set");
            return exit::NOT_RUNNING;
        };
        match exchange(&path, cmd) {
            Ok(reply) => {
                let (code, text) = interpret_reply(&reply);
                if code == exit::OK {
                    println!("{text}");
                } else {
                    eprintln!("{text}");
                }
                code
            }
            Err(_) => {
                eprintln!(
                    "{APP_NAME} is not running (no control socket at {})",
                    path.display()
                );
                exit::NOT_RUNNING
            }
        }
    }

    fn exchange(path: &Path, cmd: ControlCommand) -> std::io::Result<String> {
        let mut stream = UnixStream::connect(path)?;
        stream.set_read_timeout(Some(CLIENT_TIMEOUT))?;
        stream.set_write_timeout(Some(CLIENT_TIMEOUT))?;
        stream.write_all(format!("{}\n", cmd.verb()).as_bytes())?;
        let mut reply = String::new();
        stream.take(MAX_REQUEST_BYTES).read_to_string(&mut reply)?;
        Ok(reply)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};

        fn temp_socket(tag: &str) -> PathBuf {
            let dir = std::env::temp_dir().join(format!(
                "kaydence-control-test-{tag}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&dir);
            dir.join("control.sock")
        }

        #[test]
        fn round_trips_a_command_and_refuses_garbage() {
            let path = temp_socket("rt");
            let calls = Arc::new(AtomicUsize::new(0));
            let seen = Arc::clone(&calls);
            let server = ControlServer::spawn_at(
                path.clone(),
                Arc::new(move |cmd| {
                    seen.fetch_add(1, Ordering::SeqCst);
                    match cmd {
                        ControlCommand::Status => Ok("idle"),
                        _ => Ok("recording"),
                    }
                }),
            )
            .unwrap();

            assert_eq!(
                exchange(&path, ControlCommand::Start).unwrap(),
                "ok recording\n"
            );
            assert_eq!(
                exchange(&path, ControlCommand::Status).unwrap(),
                "ok idle\n"
            );

            let mut raw = UnixStream::connect(&path).unwrap();
            raw.write_all(b"rm -rf /\n").unwrap();
            let mut reply = String::new();
            raw.read_to_string(&mut reply).unwrap();
            assert_eq!(reply, "err unknown-command\n");
            assert_eq!(
                calls.load(Ordering::SeqCst),
                2,
                "garbage never reaches the handler"
            );

            // Private dir + 0600 socket.
            let dir_mode = fs::metadata(path.parent().unwrap()).unwrap().mode() & 0o777;
            let sock_mode = fs::symlink_metadata(&path).unwrap().mode() & 0o777;
            assert_eq!(dir_mode, 0o700);
            assert_eq!(sock_mode, 0o600);
            drop(server);
            assert!(!path.exists(), "socket removed on shutdown");
        }

        #[test]
        fn refuses_to_steal_a_live_socket_but_reclaims_a_stale_one() {
            let path = temp_socket("live");
            let first = ControlServer::spawn_at(path.clone(), Arc::new(|_| Ok("idle"))).unwrap();
            assert!(matches!(
                ControlServer::spawn_at(path.clone(), Arc::new(|_| Ok("idle"))),
                Err(ControlError::AlreadyRunning(_))
            ));
            drop(first);

            // Simulate a crash: a dead socket file with no listener behind it.
            let stale = UnixListener::bind(&path).unwrap();
            drop(stale);
            assert!(path.exists());
            let reclaimed = ControlServer::spawn_at(path.clone(), Arc::new(|_| Ok("idle")));
            assert!(reclaimed.is_ok(), "stale socket reclaimed");
        }

        #[test]
        fn refuses_a_group_readable_directory() {
            let path = temp_socket("perm");
            let dir = path.parent().unwrap();
            DirBuilder::new().mode(0o755).create(dir).unwrap();
            fs::set_permissions(dir, fs::Permissions::from_mode(0o755)).unwrap();
            assert!(matches!(
                ControlServer::spawn_at(path.clone(), Arc::new(|_| Ok("idle"))),
                Err(ControlError::UnsafeDir(_))
            ));
            let _ = fs::remove_dir_all(dir);
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use super::exit;
    use crate::hotkeys::ControlCommand;

    /// Compositor-bound control is the Linux (Wayland) hotkey path; macOS and
    /// Windows use their native global-hotkey grabs.
    pub fn send(_cmd: ControlCommand) -> i32 {
        eprintln!("the record control command is only available on Linux");
        exit::UNSUPPORTED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn only_record_invocations_are_intercepted() {
        assert_eq!(parse_cli(&argv(&["kaydence"])), CliRequest::NotControl);
        assert_eq!(
            parse_cli(&argv(&["kaydence", "--bench"])),
            CliRequest::NotControl
        );
        assert_eq!(
            parse_cli(&argv(&["kaydence", "record", "start"])),
            CliRequest::Command(ControlCommand::Start)
        );
        assert_eq!(
            parse_cli(&argv(&["kaydence", "record", "status"])),
            CliRequest::Command(ControlCommand::Status)
        );
    }

    #[test]
    fn malformed_record_invocations_print_usage() {
        assert_eq!(
            parse_cli(&argv(&["kaydence", "record"])),
            CliRequest::Usage { error: true }
        );
        assert_eq!(
            parse_cli(&argv(&["kaydence", "record", "explode"])),
            CliRequest::Usage { error: true }
        );
        assert_eq!(
            parse_cli(&argv(&["kaydence", "record", "start", "extra"])),
            CliRequest::Usage { error: true }
        );
        assert_eq!(
            parse_cli(&argv(&["kaydence", "record", "--help"])),
            CliRequest::Usage { error: false }
        );
    }

    #[test]
    fn wire_format_round_trips() {
        assert_eq!(parse_request("start\n"), Some(ControlCommand::Start));
        assert_eq!(
            parse_request("toggle\nstop\n"),
            Some(ControlCommand::Toggle)
        );
        assert_eq!(parse_request(""), None);
        assert_eq!(render_reply(&Ok("recording")), "ok recording\n");
        assert_eq!(render_reply(&Err("forbidden")), "err forbidden\n");
        assert_eq!(interpret_reply("ok idle\n"), (0, "idle".to_string()));
        assert_eq!(
            interpret_reply("err forbidden\n"),
            (1, "error: forbidden".to_string())
        );
        assert_eq!(interpret_reply("").0, 1);
    }

    #[test]
    fn compositor_hotkey_classification() {
        use CompositorHotkey::*;
        assert_eq!(
            classify_compositor_hotkey(false, None, "kaydence"),
            NotApplicable
        );
        assert_eq!(
            classify_compositor_hotkey(false, Some("[]"), "kaydence"),
            NotApplicable
        );
        assert_eq!(
            classify_compositor_hotkey(true, None, "kaydence"),
            Unverifiable
        );
        assert_eq!(
            classify_compositor_hotkey(true, Some("[]"), "kaydence"),
            NotBound
        );
        let bound =
            r#"[{"key":"F10","dispatcher":"__lua","arg":"9","description":"Kaydence: dictate"}]"#;
        assert_eq!(
            classify_compositor_hotkey(true, Some(bound), "kaydence"),
            Bound
        );
        assert!(wayland_hotkey_guidance(NotBound)
            .unwrap()
            .contains("record start"));
        assert!(wayland_hotkey_guidance(Unverifiable)
            .unwrap()
            .contains("record toggle"));
        assert_eq!(wayland_hotkey_guidance(Bound), None);
        assert_eq!(wayland_hotkey_guidance(NotApplicable), None);
    }

    #[test]
    fn usage_shows_an_omarchy_push_to_talk_binding() {
        let text = usage("kaydence");
        assert!(text.starts_with("usage: kaydence record"));
        assert!(text.contains("kaydence-ctl record start"));
        assert!(text.contains("release = true"));
    }
}
