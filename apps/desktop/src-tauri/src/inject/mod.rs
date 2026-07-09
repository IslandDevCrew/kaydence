//! Universal text injection (P1-P0-3). Delivers committed text to the focused
//! field — native-first, with a clipboard fallback and a hard secure-field
//! refusal (non-negotiable #8). Platform key/text synthesis lives behind the
//! [`TextInjector`] trait; this module owns the platform-agnostic policy, method
//! selection, and fallback orchestration so they are unit-tested without a
//! display server (verified on the macOS host + 3-OS CI).
//!
//! The per-compositor feasibility matrix and the chosen Linux strategy are in
//! `docs/spikes/P1-P0-3-wayland-injection.md` and accepted ADR-0013. Platform
//! syscall bodies land incrementally after reference-machine validation; the
//! tested policy/selection/fallback logic below is the shared core.
//!
//! No stage imports another's internals — communicate only via `SessionEvent`.
#![allow(dead_code)]

use crate::events::{AppRef, HoldReason, InjectMethod, SessionEvent, SessionId, Stage};

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod uinput;

#[cfg(target_os = "windows")]
pub mod windows;

// ─────────────────────────────── secure-field policy ───────────────────────

/// What the accessibility layer can tell us about the focused field. On some
/// Wayland clients nothing is exposed → [`FieldKind::Unknown`]: an honest
/// limitation, not a safe default (ADR-0013 §"secure-field detection").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// A normal editable text field — safe to inject.
    Editable,
    /// A password / secure-entry field — injection is refused (non-negotiable #8).
    Secure,
    /// No injectable field is focused.
    NoTarget,
    /// The platform exposes no accessibility info for the focus (opaque client).
    Unknown,
}

/// How to treat [`FieldKind::Unknown`] focus. `Strict` assumes it could be
/// secure and refuses (maximum privacy); `Lenient` injects but marks the result
/// unverified so the UI can warn once per app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnknownFieldPolicy {
    #[default]
    Lenient,
    Strict,
}

/// Outcome of the secure-field gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    /// Proceed. `verified` is false when secure-field status could not be
    /// confirmed (Unknown focus under `Lenient`) — the UI should surface that.
    Inject { verified: bool },
    /// Do not inject; hold with this reason (text is still preserved in history).
    Refuse(HoldReason),
}

/// Decide whether injection may proceed for a focused field. Pure and total —
/// this is the load-bearing safety gate for non-negotiable #8.
pub fn decide_secure(kind: FieldKind, policy: UnknownFieldPolicy) -> PolicyDecision {
    match kind {
        FieldKind::Editable => PolicyDecision::Inject { verified: true },
        FieldKind::Secure => PolicyDecision::Refuse(HoldReason::SecureField),
        FieldKind::NoTarget => PolicyDecision::Refuse(HoldReason::NoTarget),
        FieldKind::Unknown => match policy {
            UnknownFieldPolicy::Strict => PolicyDecision::Refuse(HoldReason::SecureField),
            UnknownFieldPolicy::Lenient => PolicyDecision::Inject { verified: false },
        },
    }
}

// ─────────────────────────────── focus binding ────────────────────────────

/// The app identity used to prove delivery-time focus still matches the capture
/// target. Unknown targets are represented but not trusted for delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusTarget {
    pub app: AppRef,
    pub verified: bool,
}

impl FocusTarget {
    pub fn detected(app: AppRef) -> Self {
        Self {
            app,
            verified: true,
        }
    }

    pub fn unknown(app: AppRef) -> Self {
        Self {
            app,
            verified: false,
        }
    }
}

pub fn verify_focus_binding(bound: &FocusTarget, current: &FocusTarget) -> Result<(), HoldReason> {
    if bound.verified && current.verified && bound.app.id == current.app.id {
        Ok(())
    } else {
        Err(HoldReason::FocusChanged)
    }
}

// ─────────────────────────────── method selection ──────────────────────────

/// The keystroke-synthesis channel available in this environment, populated by
/// the platform backend at startup. The Linux variants encode the ADR-0013
/// compositor matrix (wlroots vs GNOME/KDE vs X11 vs uinput).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeystrokeChannel {
    /// No keystroke path available.
    None,
    /// wlroots compositors: `zwp_virtual_keyboard_v1`.
    WaylandVirtualKeyboard,
    /// GNOME / KDE: `xdg-desktop-portal` RemoteDesktop + libei (permission-gated).
    WaylandPortalRemoteDesktop,
    /// Compositor-agnostic `/dev/uinput` (needs device permission; ADR-0013).
    LinuxUinput,
    /// X11 `XTEST`.
    X11XTest,
    /// macOS AX press / `CGEvent`.
    MacOsEvent,
    /// Windows `SendInput`.
    WindowsSendInput,
}

impl KeystrokeChannel {
    fn is_available(self) -> bool {
        self != KeystrokeChannel::None
    }
}

/// Injection channels the running environment offers (detected once at startup).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InjectorCaps {
    /// Native text insertion — macOS AX selected-text, Windows UIA `ValuePattern`,
    /// Linux AT-SPI `EditableText`. No synthetic keys; cleanest path.
    pub native_text_insert: bool,
    /// Best available keystroke channel (drives `Keystroke` and clipboard paste).
    pub keystroke: KeystrokeChannel,
    /// A readable + writable clipboard exists.
    pub clipboard: bool,
}

/// The chosen delivery path for one injection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectPlan {
    /// Native accessibility insertion.
    Native,
    /// Synthesized keystrokes via this channel.
    Keystroke(KeystrokeChannel),
    /// Clipboard set + paste (via this keystroke channel) + restore.
    Clipboard(KeystrokeChannel),
    /// No usable channel — hold and tell the user (text preserved in history).
    Unavailable,
}

/// Choose the injection method. Native-first; keystroke next; clipboard only when
/// a keystroke channel exists to send the paste — a Wayland nuance: there is no
/// "paste" without a key path. Pure and total.
pub fn select_plan(caps: InjectorCaps, prefer_clipboard: bool) -> InjectPlan {
    let has_ks = caps.keystroke.is_available();
    if prefer_clipboard && caps.clipboard && has_ks {
        InjectPlan::Clipboard(caps.keystroke)
    } else if caps.native_text_insert {
        InjectPlan::Native
    } else if has_ks {
        InjectPlan::Keystroke(caps.keystroke)
    } else {
        InjectPlan::Unavailable
    }
}

/// Map an [`InjectPlan`] to the [`InjectMethod`] recorded on the wire.
pub fn plan_method(plan: InjectPlan) -> Option<InjectMethod> {
    match plan {
        InjectPlan::Native => Some(InjectMethod::Native),
        InjectPlan::Keystroke(_) => Some(InjectMethod::Keystroke),
        InjectPlan::Clipboard(_) => Some(InjectMethod::ClipboardRestore),
        InjectPlan::Unavailable => None,
    }
}

// ─────────────────────────────── clipboard fallback ────────────────────────

/// A saved clipboard state (only text is modelled for the MVP).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClipboardSnapshot(pub Option<String>);

/// An injection-layer error carrying a user-surfaceable message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InjectError(pub String);

/// The minimal clipboard the fallback needs. Backend-provided; mocked in tests.
pub trait Clipboard {
    fn snapshot(&mut self) -> ClipboardSnapshot;
    fn set_text(&mut self, text: &str) -> Result<(), InjectError>;
    fn restore(&mut self, snap: ClipboardSnapshot) -> Result<(), InjectError>;
}

/// Clipboard fallback: snapshot → set → paste → restore. The snapshot is ALWAYS
/// restored, even if the paste keystroke fails — never leave the user's
/// clipboard changed (Pitfall P2). `paste` is the backend's synth-paste closure.
/// Returns the paste error first if both paste and restore fail.
pub fn clipboard_paste<C: Clipboard>(
    cb: &mut C,
    text: &str,
    paste: impl FnOnce() -> Result<(), InjectError>,
) -> Result<(), InjectError> {
    let snap = cb.snapshot();
    if let Err(e) = cb.set_text(text) {
        let _ = cb.restore(snap); // set failed; restore for safety, keep the set error
        return Err(e);
    }
    let pasted = paste();
    let restored = cb.restore(snap);
    pasted.and(restored)
}

// ─────────────────────────────── outcome → event ───────────────────────────

/// Why an injection did not deliver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectFailure {
    /// Deliberately withheld (secure field, no target, focus changed).
    Held(HoldReason),
    /// The chosen path errored. Text is preserved in history regardless.
    Error(String),
}

/// Map an injection attempt to the typed `SessionEvent` it emits. Finalized text
/// is never lost: `Held`/`Failed` still leave it in history (non-negotiable #2).
pub fn outcome_event(id: SessionId, result: Result<InjectMethod, InjectFailure>) -> SessionEvent {
    match result {
        Ok(method) => SessionEvent::Injected { id, method },
        Err(InjectFailure::Held(reason)) => SessionEvent::Held { id, reason },
        Err(InjectFailure::Error(error)) => SessionEvent::Failed {
            id,
            stage: Stage::Inject,
            error,
        },
    }
}

// ─────────────────────────────── platform backend ──────────────────────────

/// A platform text injector. Backends detect [`InjectorCaps`] + the focused
/// [`FieldKind`] and perform native insert / keystroke synth. Orchestration
/// (policy → selection → fallback) is the free functions above so it is tested
/// without any backend.
pub trait TextInjector {
    fn caps(&self) -> InjectorCaps;
    fn focused_field(&self) -> FieldKind;
    fn insert_native(&mut self, text: &str) -> Result<(), InjectError>;
    fn synth_text(&mut self, text: &str) -> Result<(), InjectError>;
    fn paste_clipboard(&mut self, _text: &str) -> Result<(), InjectError> {
        Err(InjectError(
            "clipboard paste not implemented for this backend".to_string(),
        ))
    }
}

pub fn inject_committed_text<I>(
    injector: &mut I,
    id: SessionId,
    text: &str,
    unknown_policy: UnknownFieldPolicy,
    prefer_clipboard: bool,
) -> SessionEvent
where
    I: TextInjector + ?Sized,
{
    match decide_secure(injector.focused_field(), unknown_policy) {
        PolicyDecision::Refuse(reason) => outcome_event(id, Err(InjectFailure::Held(reason))),
        PolicyDecision::Inject { .. } => {
            let plan = select_plan(injector.caps(), prefer_clipboard);
            let result = match plan {
                InjectPlan::Native => injector
                    .insert_native(text)
                    .map(|_| InjectMethod::Native)
                    .map_err(|err| InjectFailure::Error(err.0)),
                InjectPlan::Keystroke(_) => injector
                    .synth_text(text)
                    .map(|_| InjectMethod::Keystroke)
                    .map_err(|err| InjectFailure::Error(err.0)),
                InjectPlan::Clipboard(_) => injector
                    .paste_clipboard(text)
                    .map(|_| InjectMethod::ClipboardRestore)
                    .map_err(|err| InjectFailure::Error(err.0)),
                InjectPlan::Unavailable => Err(InjectFailure::Error(
                    "no injection channel available".to_string(),
                )),
            };
            outcome_event(id, result)
        }
    }
}

/// Placeholder backend used until ADR-0013's per-OS implementations land. It
/// reports no capabilities and refuses to act, so nothing silently "succeeds"
/// before the real, validated backends exist.
pub struct UnimplementedInjector {
    pub platform: &'static str,
}

impl TextInjector for UnimplementedInjector {
    fn caps(&self) -> InjectorCaps {
        InjectorCaps {
            native_text_insert: false,
            keystroke: KeystrokeChannel::None,
            clipboard: false,
        }
    }
    fn focused_field(&self) -> FieldKind {
        FieldKind::NoTarget
    }
    fn insert_native(&mut self, _text: &str) -> Result<(), InjectError> {
        Err(InjectError(format!(
            "native insert not yet implemented on {}",
            self.platform
        )))
    }
    fn synth_text(&mut self, _text: &str) -> Result<(), InjectError> {
        Err(InjectError(format!(
            "keystroke synth not yet implemented on {}",
            self.platform
        )))
    }
}

#[cfg(target_os = "macos")]
pub type PlatformTextInjector = macos::MacOsTextInjector;

#[cfg(target_os = "windows")]
pub type PlatformTextInjector = windows::WindowsTextInjector;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub type PlatformTextInjector = UnimplementedInjector;

/// The current platform's injector. Platforms without an approved backend use
/// the placeholder so nothing silently "succeeds" before validation.
pub fn platform_injector() -> PlatformTextInjector {
    #[cfg(target_os = "macos")]
    {
        macos::MacOsTextInjector
    }

    #[cfg(target_os = "windows")]
    {
        windows::WindowsTextInjector
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let platform = if cfg!(target_os = "linux") {
            "Linux"
        } else {
            "unknown"
        };
        UnimplementedInjector { platform }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── secure-field gate (non-negotiable #8) ──
    #[test]
    fn secure_field_is_always_refused() {
        assert_eq!(
            decide_secure(FieldKind::Secure, UnknownFieldPolicy::Lenient),
            PolicyDecision::Refuse(HoldReason::SecureField)
        );
        assert_eq!(
            decide_secure(FieldKind::Secure, UnknownFieldPolicy::Strict),
            PolicyDecision::Refuse(HoldReason::SecureField)
        );
    }

    #[test]
    fn editable_field_injects_verified() {
        assert_eq!(
            decide_secure(FieldKind::Editable, UnknownFieldPolicy::Lenient),
            PolicyDecision::Inject { verified: true }
        );
    }

    #[test]
    fn no_target_holds() {
        assert_eq!(
            decide_secure(FieldKind::NoTarget, UnknownFieldPolicy::Lenient),
            PolicyDecision::Refuse(HoldReason::NoTarget)
        );
    }

    #[test]
    fn unknown_focus_respects_policy() {
        // Lenient injects but marks it unverified (UI warns once per app).
        assert_eq!(
            decide_secure(FieldKind::Unknown, UnknownFieldPolicy::Lenient),
            PolicyDecision::Inject { verified: false }
        );
        // Strict treats an opaque client as possibly-secure and refuses.
        assert_eq!(
            decide_secure(FieldKind::Unknown, UnknownFieldPolicy::Strict),
            PolicyDecision::Refuse(HoldReason::SecureField)
        );
    }

    #[test]
    fn focus_binding_requires_same_verified_app() {
        let bound = FocusTarget::detected(AppRef {
            id: "com.example.editor".to_string(),
            name: "Editor".to_string(),
        });
        let same = FocusTarget::detected(AppRef {
            id: "com.example.editor".to_string(),
            name: "Editor".to_string(),
        });
        let other = FocusTarget::detected(AppRef {
            id: "com.example.mail".to_string(),
            name: "Mail".to_string(),
        });
        let unknown = FocusTarget::unknown(AppRef {
            id: "unknown".to_string(),
            name: "Unknown app".to_string(),
        });

        assert_eq!(verify_focus_binding(&bound, &same), Ok(()));
        assert_eq!(
            verify_focus_binding(&bound, &other),
            Err(HoldReason::FocusChanged)
        );
        assert_eq!(
            verify_focus_binding(&unknown, &unknown),
            Err(HoldReason::FocusChanged)
        );
    }

    // ── method selection across the compositor matrix ──
    fn caps(native: bool, ks: KeystrokeChannel, clip: bool) -> InjectorCaps {
        InjectorCaps {
            native_text_insert: native,
            keystroke: ks,
            clipboard: clip,
        }
    }

    #[test]
    fn wlroots_prefers_native_then_virtual_keyboard() {
        // AT-SPI-exposing app on wlroots → native.
        assert_eq!(
            select_plan(
                caps(true, KeystrokeChannel::WaylandVirtualKeyboard, true),
                false
            ),
            InjectPlan::Native
        );
        // Opaque app on wlroots → virtual-keyboard keystrokes.
        assert_eq!(
            select_plan(
                caps(false, KeystrokeChannel::WaylandVirtualKeyboard, true),
                false
            ),
            InjectPlan::Keystroke(KeystrokeChannel::WaylandVirtualKeyboard)
        );
    }

    #[test]
    fn gnome_uses_portal_when_no_native() {
        assert_eq!(
            select_plan(
                caps(false, KeystrokeChannel::WaylandPortalRemoteDesktop, true),
                false
            ),
            InjectPlan::Keystroke(KeystrokeChannel::WaylandPortalRemoteDesktop)
        );
    }

    #[test]
    fn clipboard_needs_a_keystroke_channel_for_paste() {
        // prefer_clipboard but no keystroke path → cannot paste; fall through to native.
        assert_eq!(
            select_plan(caps(true, KeystrokeChannel::None, true), true),
            InjectPlan::Native
        );
        // prefer_clipboard with a keystroke path → clipboard plan.
        assert_eq!(
            select_plan(caps(true, KeystrokeChannel::X11XTest, true), true),
            InjectPlan::Clipboard(KeystrokeChannel::X11XTest)
        );
    }

    #[test]
    fn no_channels_is_unavailable() {
        assert_eq!(
            select_plan(caps(false, KeystrokeChannel::None, false), false),
            InjectPlan::Unavailable
        );
        assert_eq!(plan_method(InjectPlan::Unavailable), None);
    }

    // ── clipboard fallback: restore is unconditional (Pitfall P2) ──
    struct MockClipboard {
        current: Option<String>,
        log: Vec<String>,
    }
    impl Clipboard for MockClipboard {
        fn snapshot(&mut self) -> ClipboardSnapshot {
            self.log.push("snapshot".into());
            ClipboardSnapshot(self.current.clone())
        }
        fn set_text(&mut self, text: &str) -> Result<(), InjectError> {
            self.log.push(format!("set:{text}"));
            self.current = Some(text.to_string());
            Ok(())
        }
        fn restore(&mut self, snap: ClipboardSnapshot) -> Result<(), InjectError> {
            self.log.push("restore".into());
            self.current = snap.0;
            Ok(())
        }
    }

    #[test]
    fn clipboard_paste_restores_original_on_success() {
        let mut cb = MockClipboard {
            current: Some("user copied this".into()),
            log: vec![],
        };
        let r = clipboard_paste(&mut cb, "dictated text", || Ok(()));
        assert!(r.is_ok());
        assert_eq!(cb.current.as_deref(), Some("user copied this"));
        assert_eq!(cb.log, vec!["snapshot", "set:dictated text", "restore"]);
    }

    #[test]
    fn clipboard_paste_restores_even_when_paste_fails() {
        let mut cb = MockClipboard {
            current: Some("original".into()),
            log: vec![],
        };
        let r = clipboard_paste(&mut cb, "dictated", || {
            Err(InjectError("paste key failed".into()))
        });
        assert!(r.is_err()); // the paste error surfaces
        assert_eq!(cb.current.as_deref(), Some("original")); // but the clipboard is intact
        assert!(cb.log.contains(&"restore".to_string()));
    }

    // ── outcome → typed event ──
    #[test]
    fn outcomes_map_to_events() {
        let id = SessionId(ulid::Ulid::new());
        assert!(matches!(
            outcome_event(id, Ok(InjectMethod::Native)),
            SessionEvent::Injected {
                method: InjectMethod::Native,
                ..
            }
        ));
        assert!(matches!(
            outcome_event(id, Err(InjectFailure::Held(HoldReason::SecureField))),
            SessionEvent::Held {
                reason: HoldReason::SecureField,
                ..
            }
        ));
        assert!(matches!(
            outcome_event(id, Err(InjectFailure::Error("x".into()))),
            SessionEvent::Failed {
                stage: Stage::Inject,
                ..
            }
        ));
    }

    struct ScriptedInjector {
        caps: InjectorCaps,
        field: FieldKind,
        native_error: Option<String>,
        synth_error: Option<String>,
        paste_error: Option<String>,
        delivered: Vec<String>,
    }

    impl ScriptedInjector {
        fn native(field: FieldKind) -> Self {
            Self {
                caps: caps(true, KeystrokeChannel::None, false),
                field,
                native_error: None,
                synth_error: None,
                paste_error: None,
                delivered: Vec::new(),
            }
        }

        fn unavailable(field: FieldKind) -> Self {
            Self {
                caps: caps(false, KeystrokeChannel::None, false),
                field,
                native_error: None,
                synth_error: None,
                paste_error: None,
                delivered: Vec::new(),
            }
        }
    }

    impl TextInjector for ScriptedInjector {
        fn caps(&self) -> InjectorCaps {
            self.caps
        }

        fn focused_field(&self) -> FieldKind {
            self.field
        }

        fn insert_native(&mut self, text: &str) -> Result<(), InjectError> {
            if let Some(error) = &self.native_error {
                Err(InjectError(error.clone()))
            } else {
                self.delivered.push(text.to_string());
                Ok(())
            }
        }

        fn synth_text(&mut self, text: &str) -> Result<(), InjectError> {
            if let Some(error) = &self.synth_error {
                Err(InjectError(error.clone()))
            } else {
                self.delivered.push(text.to_string());
                Ok(())
            }
        }

        fn paste_clipboard(&mut self, text: &str) -> Result<(), InjectError> {
            if let Some(error) = &self.paste_error {
                Err(InjectError(error.clone()))
            } else {
                self.delivered.push(text.to_string());
                Ok(())
            }
        }
    }

    #[test]
    fn inject_committed_text_holds_when_no_target_is_focused() {
        let id = SessionId(ulid::Ulid::new());
        let mut injector = ScriptedInjector::unavailable(FieldKind::NoTarget);

        assert_eq!(
            inject_committed_text(
                &mut injector,
                id,
                "hello",
                UnknownFieldPolicy::Lenient,
                false
            ),
            SessionEvent::Held {
                id,
                reason: HoldReason::NoTarget
            }
        );
        assert!(injector.delivered.is_empty());
    }

    #[test]
    fn inject_committed_text_uses_native_path_when_available() {
        let id = SessionId(ulid::Ulid::new());
        let mut injector = ScriptedInjector::native(FieldKind::Editable);

        assert_eq!(
            inject_committed_text(
                &mut injector,
                id,
                "clean text",
                UnknownFieldPolicy::Lenient,
                false
            ),
            SessionEvent::Injected {
                id,
                method: InjectMethod::Native
            }
        );
        assert_eq!(injector.delivered, vec!["clean text"]);
    }

    #[test]
    fn inject_committed_text_records_backend_errors() {
        let id = SessionId(ulid::Ulid::new());
        let mut injector = ScriptedInjector::native(FieldKind::Editable);
        injector.native_error = Some("AX insert failed".to_string());

        assert_eq!(
            inject_committed_text(
                &mut injector,
                id,
                "clean text",
                UnknownFieldPolicy::Lenient,
                false
            ),
            SessionEvent::Failed {
                id,
                stage: Stage::Inject,
                error: "AX insert failed".to_string()
            }
        );
        assert!(injector.delivered.is_empty());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    #[test]
    fn placeholder_backend_refuses_and_reports_no_caps() {
        let mut inj = platform_injector();
        assert_eq!(inj.caps().keystroke, KeystrokeChannel::None);
        assert_eq!(inj.focused_field(), FieldKind::NoTarget);
        assert!(inj.insert_native("x").is_err());
        assert!(inj.synth_text("x").is_err());
    }
}
