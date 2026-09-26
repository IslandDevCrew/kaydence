//! Hyprland compositor IPC (ADR-0023) — the Linux answer to "which window has
//! keyboard focus?", which generic Wayland deliberately refuses to answer.
//!
//! Hyprland (Omarchy's compositor) exposes a per-instance request socket at
//! `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock`. We send
//! `j/activewindow` and read one JSON reply. That gives:
//!   - the frontmost-app identity for `profiles/` (class = stable app id), and
//!   - the focus-binding proof for Pitfall P9 (bind at capture start, re-check
//!     before delivery), plus the focused client's pid so the AT-SPI secure-field
//!     signal can be matched to the window that will actually receive the keys.
//!
//! This is the shared frontmost-window helper `inject/AGENTS.md` asks for (the
//! Windows twin is `inject::windows::foreground_app`). Local Unix socket owned by
//! the user's compositor — NOT a network surface; nothing is persisted.
//!
//! Parsing is pure and unit-tested on every OS; only the socket I/O is Linux-only.
#![allow(dead_code)]

use crate::events::AppRef;

/// The focused Hyprland client, as far as focus binding and profiles need it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveWindow {
    /// Hyprland window address (`0x…`) — unique per live window.
    pub address: String,
    /// Wayland `app_id` / X11 WM_CLASS — the stable app identity.
    pub class: String,
    /// Window title. Used only as a human label; never persisted by this module.
    pub title: String,
    /// Owning process id, when Hyprland reports one (> 0).
    pub pid: Option<u32>,
    /// True for XWayland clients (they see our keystrokes through XWayland).
    pub xwayland: bool,
}

impl ActiveWindow {
    /// The `profiles/` identity for this window. The class is the id (stable
    /// across launches); the human name prefers the class, falling back to the
    /// title only when a client never set an app id.
    pub fn app_ref(&self) -> Option<AppRef> {
        let id = if self.class.is_empty() {
            return None;
        } else {
            self.class.clone()
        };
        Some(AppRef {
            name: display_name(&id),
            id,
        })
    }
}

/// `org.mozilla.firefox` → `firefox`, `com.mitchellh.ghostty` → `ghostty`,
/// `foot` → `foot`. Pure.
fn display_name(class: &str) -> String {
    class.rsplit('.').next().unwrap_or(class).to_string()
}

/// Parse Hyprland's `j/activewindow` reply. Hyprland answers `{}` when no
/// window is focused (e.g. an empty workspace or a layer-shell surface has
/// keyboard focus) — that is "no target", not an error. Pure and total.
pub fn parse_active_window(json: &str) -> Option<ActiveWindow> {
    let value: serde_json::Value = serde_json::from_str(json.trim()).ok()?;
    let obj = value.as_object()?;
    let address = obj.get("address")?.as_str()?.to_string();
    if address.is_empty() {
        return None;
    }
    let class = obj
        .get("class")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| obj.get("initialClass").and_then(|v| v.as_str()))
        .unwrap_or_default()
        .to_string();
    let title = obj
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let pid = obj
        .get("pid")
        .and_then(|v| v.as_i64())
        .filter(|p| *p > 0)
        .and_then(|p| u32::try_from(p).ok());
    let xwayland = obj
        .get("xwayland")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Some(ActiveWindow {
        address,
        class,
        title,
        pid,
        xwayland,
    })
}

/// True when any compositor bind mentions `needle` (e.g. `kaydence`), matched
/// case-insensitively. Parses Hyprland's `j/binds` reply; used to show an honest
/// "hotkey bound in your compositor" state instead of assuming an X11 grab works
/// on Wayland. Two shapes exist: hyprlang `exec` binds carry the command in
/// `arg`; Lua-configured binds (Hyprland ≥ 0.55, Omarchy 4) report
/// `dispatcher: "__lua"` with an opaque function id in `arg`, so only their
/// `description` carries intent.
pub fn binds_mention(json: &str, needle: &str) -> bool {
    let Ok(serde_json::Value::Array(binds)) = serde_json::from_str::<serde_json::Value>(json)
    else {
        return false;
    };
    let needle = needle.to_lowercase();
    binds.iter().any(|bind| {
        ["arg", "description"].iter().any(|field| {
            bind.get(*field)
                .and_then(|v| v.as_str())
                .is_some_and(|text| text.to_lowercase().contains(&needle))
        })
    })
}

/// Resolve the request-socket path from the session environment. Pure over its
/// inputs so the path rules are testable: modern Hyprland lives under
/// `$XDG_RUNTIME_DIR/hypr/<sig>/`, versions before 0.40 used `/tmp/hypr/<sig>/`.
pub fn socket_candidates(runtime_dir: Option<&str>, signature: Option<&str>) -> Vec<String> {
    let Some(sig) = signature.filter(|s| !s.is_empty() && !s.contains('/')) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(2);
    if let Some(dir) = runtime_dir.filter(|d| !d.is_empty()) {
        out.push(format!("{dir}/hypr/{sig}/.socket.sock"));
    }
    out.push(format!("/tmp/hypr/{sig}/.socket.sock"));
    out
}

#[cfg(target_os = "linux")]
mod io {
    use super::{parse_active_window, socket_candidates, ActiveWindow};
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    /// Hard ceiling for one IPC round trip. Capture start has a 50 ms budget
    /// (root §5); a local socket answer is ~1 ms, so a slow compositor degrades
    /// to "unknown focus" instead of stalling the hotkey path.
    const IPC_TIMEOUT: Duration = Duration::from_millis(40);
    /// Replies we care about are small; cap reads so a misbehaving peer cannot
    /// balloon memory.
    const MAX_REPLY_BYTES: u64 = 4 * 1024 * 1024;

    /// True when this process runs inside a Hyprland session.
    pub fn detected() -> bool {
        std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some_and(|s| !s.is_empty())
    }

    /// Send one Hyprland request (e.g. `j/activewindow`) and return the reply.
    pub fn request(command: &str) -> Option<String> {
        let runtime = std::env::var("XDG_RUNTIME_DIR").ok();
        let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();
        for path in socket_candidates(runtime.as_deref(), sig.as_deref()) {
            let Ok(mut stream) = UnixStream::connect(&path) else {
                continue;
            };
            stream.set_read_timeout(Some(IPC_TIMEOUT)).ok()?;
            stream.set_write_timeout(Some(IPC_TIMEOUT)).ok()?;
            stream.write_all(command.as_bytes()).ok()?;
            let mut reply = String::new();
            stream
                .take(MAX_REPLY_BYTES)
                .read_to_string(&mut reply)
                .ok()?;
            return Some(reply);
        }
        None
    }

    /// The currently focused Hyprland window, if any.
    pub fn active_window() -> Option<ActiveWindow> {
        if !detected() {
            return None;
        }
        parse_active_window(&request("j/activewindow")?)
    }

    /// The raw `j/binds` reply (for the compositor-hotkey setup check).
    pub fn binds_json() -> Option<String> {
        if !detected() {
            return None;
        }
        request("j/binds")
    }
}

#[cfg(target_os = "linux")]
pub use io::{active_window, binds_json, detected, request};

/// Frontmost-app identity via Hyprland, for `profiles/`. `None` outside
/// Hyprland or when no window holds focus.
#[cfg(target_os = "linux")]
pub fn foreground_app() -> Option<AppRef> {
    active_window()?.app_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed from a real Hyprland 0.56 reply on Omarchy 4.0.4.
    const FOOT: &str = r#"{
        "address": "0x60745e8d99b0", "mapped": true, "hidden": false,
        "at": [0, 0], "size": [1500, 1000],
        "workspace": {"id": 1, "name": "1"},
        "floating": false, "monitor": 0,
        "class": "foot", "title": "claude", "initialClass": "foot",
        "initialTitle": "foot", "pid": 4242, "xwayland": false
    }"#;

    #[test]
    fn parses_a_native_wayland_window() {
        let w = parse_active_window(FOOT).unwrap();
        assert_eq!(w.address, "0x60745e8d99b0");
        assert_eq!(w.class, "foot");
        assert_eq!(w.pid, Some(4242));
        assert!(!w.xwayland);
        assert_eq!(
            w.app_ref().unwrap(),
            AppRef {
                id: "foot".into(),
                name: "foot".into()
            }
        );
    }

    #[test]
    fn empty_object_means_no_focused_window() {
        assert_eq!(parse_active_window("{}"), None);
        assert_eq!(parse_active_window("  {}\n"), None);
    }

    #[test]
    fn garbage_is_no_window_not_a_panic() {
        assert_eq!(parse_active_window(""), None);
        assert_eq!(parse_active_window("unknown request"), None);
        assert_eq!(parse_active_window("[]"), None);
    }

    #[test]
    fn falls_back_to_initial_class_and_drops_bogus_pid() {
        let w = parse_active_window(
            r#"{"address":"0x1","class":"","initialClass":"org.mozilla.firefox","title":"t","pid":-1,"xwayland":true}"#,
        )
        .unwrap();
        assert_eq!(w.class, "org.mozilla.firefox");
        assert_eq!(w.pid, None);
        assert!(w.xwayland);
        assert_eq!(w.app_ref().unwrap().name, "firefox");
    }

    #[test]
    fn classless_window_has_no_app_identity() {
        let w = parse_active_window(r#"{"address":"0x2","class":"","title":"x","pid":7}"#).unwrap();
        assert_eq!(w.app_ref(), None);
    }

    #[test]
    fn socket_paths_follow_runtime_dir_then_legacy_tmp() {
        assert_eq!(
            socket_candidates(Some("/run/user/1000"), Some("abc_1_2")),
            vec![
                "/run/user/1000/hypr/abc_1_2/.socket.sock".to_string(),
                "/tmp/hypr/abc_1_2/.socket.sock".to_string()
            ]
        );
        assert_eq!(
            socket_candidates(None, Some("abc")),
            vec!["/tmp/hypr/abc/.socket.sock".to_string()]
        );
    }

    #[test]
    fn no_signature_or_path_traversal_means_no_socket() {
        assert!(socket_candidates(Some("/run/user/1000"), None).is_empty());
        assert!(socket_candidates(Some("/run/user/1000"), Some("")).is_empty());
        assert!(socket_candidates(Some("/run/user/1000"), Some("../../etc")).is_empty());
    }

    #[test]
    fn binds_check_finds_a_hyprlang_exec_bind() {
        let binds = r#"[
            {"modmask":0,"key":"F9","dispatcher":"exec","arg":"voxtype record start"},
            {"modmask":64,"key":"D","dispatcher":"exec","arg":"kaydence record start"}
        ]"#;
        assert!(binds_mention(binds, "kaydence record"));
        assert!(!binds_mention(binds, "whisper-ahead"));
        assert!(!binds_mention("not json", "kaydence record"));
    }

    #[test]
    fn binds_check_finds_a_lua_bind_by_description() {
        // Hyprland 0.56 Lua binds: the command is hidden behind a function id.
        let binds = r#"[
            {"key":"F9","dispatcher":"__lua","arg":"270","description":"Start dictation (push-to-talk)"},
            {"key":"F10","dispatcher":"__lua","arg":"301","description":"Kaydence: start dictation"}
        ]"#;
        assert!(binds_mention(binds, "kaydence"));
        let voxtype_only = r#"[{"key":"F9","dispatcher":"__lua","arg":"270","description":"Start dictation (push-to-talk)"}]"#;
        assert!(!binds_mention(voxtype_only, "kaydence"));
    }
}
