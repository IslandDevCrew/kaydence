//! Windows text-injection backend (P1-P0-3, ADR-0013). UI Automation provides
//! BOTH the secure-field signal (`IsPassword` — non-negotiable #8) and the
//! native insertion path (`ValuePattern.SetValue`, which works on Windows,
//! unlike AT-SPI `InsertText` on GNOME — the inverse of the Linux ladder).
//! `SendInput` with `KEYEVENTF_UNICODE` is the keystroke fallback (full
//! Unicode, layout-independent), and the shared clipboard fallback rides on a
//! Ctrl+V synthesized the same way. Compiled only on Windows.
//!
//! The injector is stateless: the runtime boxes it as `dyn TextInjector +
//! Send`, and COM interface pointers are not `Send`, so every call creates its
//! UIA client on the calling thread (COM is initialized once per thread and
//! deliberately never uninitialized — it lives as long as the thread).
#![cfg(target_os = "windows")]
#![allow(dead_code)]

use std::cell::Cell;
use std::thread;
use std::time::Duration;

use windows::core::{Interface, BSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HGLOBAL, HWND, RPC_E_CHANGED_MODE};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::System::DataExchange::{
    CloseClipboard, CountClipboardFormats, EmptyClipboard, GetClipboardData,
    IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationTextPattern,
    IUIAutomationTextRange, IUIAutomationValuePattern, TextPatternRangeEndpoint_End,
    TextPatternRangeEndpoint_Start, UIA_ComboBoxControlTypeId, UIA_DocumentControlTypeId,
    UIA_EditControlTypeId, UIA_TextPatternId, UIA_ValuePatternId,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_CONTROL, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

use super::{
    clipboard_paste, Clipboard, ClipboardSnapshot, FieldKind, InjectError, InjectorCaps,
    KeystrokeChannel, TextInjector,
};
use crate::events::AppRef;

/// `CF_UNICODETEXT` (winuser.h). The value is ABI-stable; declared locally so
/// the `Win32_System_Ole` feature isn't pulled in for one constant.
const CF_UNICODETEXT: u32 = 13;

/// Delay between the synthesized Ctrl+V and the clipboard restore, so the
/// target app's message loop reads the injected text before it disappears.
/// Well inside the ≤200 ms restore budget (invariant #2 / Pitfall P2).
const PASTE_SETTLE_MS: u64 = 120;

/// `SendInput` batch size in `INPUT` events (one UTF-16 unit = down + up).
const SENDINPUT_CHUNK: usize = 512;

// ─────────────────────────────── field classification ──────────────────────

/// What the UIA focus probe learned, as plain data so the mapping to
/// [`FieldKind`] is pure and unit-testable (mirrors `linux::role_to_field_kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiaFocusFacts {
    /// `UIA_CONTROLTYPE_ID.0` of the focused element.
    pub control_type: i32,
    /// UIA `IsPassword` — the load-bearing secure-field signal.
    pub is_password: bool,
    pub is_enabled: bool,
    /// A `ValuePattern` is exposed (native `SetValue` is possible).
    pub has_value_pattern: bool,
    /// The `ValuePattern` reports read-only (meaningless without one).
    pub value_read_only: bool,
    /// A `TextPattern` is exposed (caret/selection-aware insertion possible).
    pub has_text_pattern: bool,
}

/// Map UIA focus facts to the platform-agnostic [`FieldKind`]. A password
/// control is Secure regardless of anything else; a disabled control is not a
/// target; text-entry control types and writable value providers are editable.
/// Pure + total.
pub fn classify_uia_focus(facts: &UiaFocusFacts) -> FieldKind {
    if facts.is_password {
        return FieldKind::Secure;
    }
    if !facts.is_enabled {
        return FieldKind::NoTarget;
    }
    let text_entry_type = facts.control_type == UIA_EditControlTypeId.0
        || facts.control_type == UIA_DocumentControlTypeId.0
        || facts.control_type == UIA_ComboBoxControlTypeId.0;
    let writable_value = facts.has_value_pattern && !facts.value_read_only;
    if text_entry_type || writable_value {
        FieldKind::Editable
    } else {
        FieldKind::NoTarget
    }
}

/// Whether `insert_native` can succeed for these facts: a writable
/// `ValuePattern` is required (`TextPattern` alone is read-only in UIA).
pub fn supports_native_insert(facts: &UiaFocusFacts) -> bool {
    classify_uia_focus(facts) == FieldKind::Editable
        && facts.has_value_pattern
        && !facts.value_read_only
}

// ─────────────────────────────── COM / UIA plumbing ────────────────────────

thread_local! {
    static COM_READY: Cell<bool> = const { Cell::new(false) };
}

/// Initialize COM for this thread once (leaked for the thread's lifetime —
/// the injector may be called repeatedly from the runtime thread). A thread
/// already in an STA (`RPC_E_CHANGED_MODE`) is fine: COM is usable either way.
fn ensure_com() -> Result<(), InjectError> {
    COM_READY.with(|ready| {
        if ready.get() {
            return Ok(());
        }
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if hr.is_ok() || hr == RPC_E_CHANGED_MODE {
            ready.set(true);
            Ok(())
        } else {
            Err(InjectError(format!(
                "Windows COM initialization failed: {hr}"
            )))
        }
    })
}

fn uia_client() -> Result<IUIAutomation, InjectError> {
    ensure_com()?;
    unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }
        .map_err(|e| InjectError(format!("UI Automation client unavailable: {e}")))
}

/// The focused element plus its facts. `Unavailable` = UIA itself is down
/// (degraded mode — maps to [`FieldKind::Unknown`], the Unknown-focus policy
/// governs); `NoFocus` = UIA works but nothing injectable has focus.
enum FocusProbe {
    Unavailable(String),
    NoFocus,
    Focused {
        element: IUIAutomationElement,
        facts: UiaFocusFacts,
    },
}

fn probe_focus() -> FocusProbe {
    let uia = match uia_client() {
        Ok(uia) => uia,
        Err(e) => return FocusProbe::Unavailable(e.0),
    };
    let element = match unsafe { uia.GetFocusedElement() } {
        Ok(element) => element,
        Err(_) => return FocusProbe::NoFocus,
    };
    let facts = gather_facts(&element);
    FocusProbe::Focused { element, facts }
}

fn gather_facts(element: &IUIAutomationElement) -> UiaFocusFacts {
    let control_type = unsafe { element.CurrentControlType() }
        .map(|ct| ct.0)
        .unwrap_or(0);
    let is_password = unsafe { element.CurrentIsPassword() }
        .map(|b| b.as_bool())
        // If the password bit is unreadable, fail CLOSED: treat as secure.
        .unwrap_or(true);
    let is_enabled = unsafe { element.CurrentIsEnabled() }
        .map(|b| b.as_bool())
        .unwrap_or(false);
    let value_pattern = element_pattern::<IUIAutomationValuePattern>(element, UIA_ValuePatternId.0);
    let value_read_only = value_pattern
        .as_ref()
        .map(|vp| unsafe { vp.CurrentIsReadOnly() }.map_or(true, |b| b.as_bool()))
        .unwrap_or(true);
    let has_text_pattern =
        element_pattern::<IUIAutomationTextPattern>(element, UIA_TextPatternId.0).is_some();
    UiaFocusFacts {
        control_type,
        is_password,
        is_enabled,
        has_value_pattern: value_pattern.is_some(),
        value_read_only,
        has_text_pattern,
    }
}

fn element_pattern<T: Interface>(element: &IUIAutomationElement, pattern_id: i32) -> Option<T> {
    let unknown = unsafe {
        element.GetCurrentPattern(windows::Win32::UI::Accessibility::UIA_PATTERN_ID(
            pattern_id,
        ))
    }
    .ok()?;
    unknown.cast::<T>().ok()
}

// ─────────────────────────────── the injector ──────────────────────────────

#[derive(Debug, Default)]
pub struct WindowsTextInjector;

impl TextInjector for WindowsTextInjector {
    fn caps(&self) -> InjectorCaps {
        match probe_focus() {
            FocusProbe::Focused { facts, .. } => {
                let kind = classify_uia_focus(&facts);
                let injectable = kind == FieldKind::Editable;
                InjectorCaps {
                    native_text_insert: injectable && supports_native_insert(&facts),
                    keystroke: if injectable {
                        KeystrokeChannel::WindowsSendInput
                    } else {
                        KeystrokeChannel::None
                    },
                    clipboard: injectable,
                }
            }
            // UIA down: degrade to keystrokes; the Unknown-focus policy has
            // already decided whether injection may proceed at all.
            FocusProbe::Unavailable(_) => InjectorCaps {
                native_text_insert: false,
                keystroke: KeystrokeChannel::WindowsSendInput,
                clipboard: true,
            },
            FocusProbe::NoFocus => InjectorCaps {
                native_text_insert: false,
                keystroke: KeystrokeChannel::None,
                clipboard: false,
            },
        }
    }

    fn focused_field(&self) -> FieldKind {
        match probe_focus() {
            FocusProbe::Focused { facts, .. } => classify_uia_focus(&facts),
            FocusProbe::Unavailable(_) => FieldKind::Unknown,
            FocusProbe::NoFocus => FieldKind::NoTarget,
        }
    }

    fn insert_native(&mut self, text: &str) -> Result<(), InjectError> {
        let FocusProbe::Focused { element, facts } = probe_focus() else {
            return Err(InjectError(
                "Windows native insert refused: no UIA focused element".to_string(),
            ));
        };
        require_editable(&facts, "Windows native insert")?;
        let value_pattern =
            element_pattern::<IUIAutomationValuePattern>(&element, UIA_ValuePatternId.0)
                .ok_or_else(|| {
                    InjectError("Windows focused field exposes no ValuePattern".to_string())
                })?;
        if unsafe { value_pattern.CurrentIsReadOnly() }.map_or(true, |b| b.as_bool()) {
            return Err(InjectError(
                "Windows focused field's ValuePattern is read-only".to_string(),
            ));
        }
        let composed = compose_insertion(&element, &value_pattern, text);
        unsafe { value_pattern.SetValue(&BSTR::from(composed.as_str())) }
            .map_err(|e| InjectError(format!("Windows UIA SetValue failed: {e}")))
    }

    fn synth_text(&mut self, text: &str) -> Result<(), InjectError> {
        match probe_focus() {
            FocusProbe::Focused { facts, .. } => {
                require_editable(&facts, "Windows keystroke insert")?;
            }
            // UIA down: the shared policy (Lenient/Strict on Unknown) already
            // ran — typing is the degraded-mode contract, not a bypass.
            FocusProbe::Unavailable(_) => {}
            FocusProbe::NoFocus => {
                return Err(InjectError(
                    "Windows keystroke insert refused: no focused element".to_string(),
                ));
            }
        }
        send_unicode_text(&normalize_newlines_for_keystrokes(text))
    }

    fn paste_clipboard(&mut self, text: &str) -> Result<(), InjectError> {
        match probe_focus() {
            FocusProbe::Focused { facts, .. } => {
                require_editable(&facts, "Windows clipboard paste")?;
            }
            FocusProbe::Unavailable(_) => {}
            FocusProbe::NoFocus => {
                return Err(InjectError(
                    "Windows clipboard paste refused: no focused element".to_string(),
                ));
            }
        }
        let mut clipboard = WindowsClipboard::default();
        clipboard_paste(&mut clipboard, text, || {
            send_ctrl_v()?;
            // Let the target's message loop consume the paste before restore.
            thread::sleep(Duration::from_millis(PASTE_SETTLE_MS));
            Ok(())
        })
    }
}

/// Defense in depth: the shared `decide_secure` gate has already run, but no
/// backend method will act on a secure or non-editable focus either.
fn require_editable(facts: &UiaFocusFacts, action: &str) -> Result<(), InjectError> {
    match classify_uia_focus(facts) {
        FieldKind::Editable => Ok(()),
        FieldKind::Secure => Err(InjectError(format!(
            "{action} refused: secure field focused (non-negotiable #8)"
        ))),
        FieldKind::NoTarget | FieldKind::Unknown => Err(InjectError(format!(
            "{action} refused: no editable field focused"
        ))),
    }
}

// ─────────────────────────────── native insertion ──────────────────────────

/// Compose the full post-insertion value. With a `TextPattern`, the text goes
/// at the caret (replacing any selection); without caret info, it appends —
/// `SetValue` replaces the whole value, so append is the safe default.
fn compose_insertion(
    element: &IUIAutomationElement,
    value_pattern: &IUIAutomationValuePattern,
    text: &str,
) -> String {
    if let Some(text_pattern) =
        element_pattern::<IUIAutomationTextPattern>(element, UIA_TextPatternId.0)
    {
        if let Some((prefix, suffix)) = split_at_selection(&text_pattern) {
            return format!("{prefix}{text}{suffix}");
        }
    }
    let current = unsafe { value_pattern.CurrentValue() }
        .map(|b| b.to_string())
        .unwrap_or_default();
    format!("{current}{text}")
}

/// Split the document text around the current selection/caret: everything
/// before the selection start, and everything after the selection end.
fn split_at_selection(text_pattern: &IUIAutomationTextPattern) -> Option<(String, String)> {
    let document = unsafe { text_pattern.DocumentRange() }.ok()?;
    let selections = unsafe { text_pattern.GetSelection() }.ok()?;
    if unsafe { selections.Length() }.ok()? < 1 {
        return None;
    }
    let selection = unsafe { selections.GetElement(0) }.ok()?;
    let prefix = range_text_until(&document, &selection, true)?;
    let suffix = range_text_until(&document, &selection, false)?;
    Some((prefix, suffix))
}

/// Text of the document range clipped against one end of the selection:
/// `before_selection` keeps document-start→selection-start, else
/// selection-end→document-end.
fn range_text_until(
    document: &IUIAutomationTextRange,
    selection: &IUIAutomationTextRange,
    before_selection: bool,
) -> Option<String> {
    let clipped = unsafe { document.Clone() }.ok()?;
    if before_selection {
        unsafe {
            clipped.MoveEndpointByRange(
                TextPatternRangeEndpoint_End,
                selection,
                TextPatternRangeEndpoint_Start,
            )
        }
        .ok()?;
    } else {
        unsafe {
            clipped.MoveEndpointByRange(
                TextPatternRangeEndpoint_Start,
                selection,
                TextPatternRangeEndpoint_End,
            )
        }
        .ok()?;
    }
    Some(unsafe { clipped.GetText(-1) }.ok()?.to_string())
}

// ─────────────────────────────── keystroke synthesis ───────────────────────

/// Edit controls expect Enter as CR (`WM_CHAR` 0x0D); bare LF is ignored by
/// many of them. Normalized before UTF-16 encoding. Pure.
pub fn normalize_newlines_for_keystrokes(text: &str) -> String {
    text.replace("\r\n", "\r").replace('\n', "\r")
}

/// Type `text` into the focused control via `SendInput` `KEYEVENTF_UNICODE` —
/// full Unicode (surrogate pairs are sent unit-by-unit and reassembled by the
/// target), independent of keyboard layout.
fn send_unicode_text(text: &str) -> Result<(), InjectError> {
    let units: Vec<u16> = text.encode_utf16().collect();
    for chunk in units.chunks(SENDINPUT_CHUNK / 2) {
        let inputs: Vec<INPUT> = chunk
            .iter()
            .flat_map(|&unit| {
                [
                    unicode_key_input(unit, false),
                    unicode_key_input(unit, true),
                ]
            })
            .collect();
        send_inputs(&inputs)?;
    }
    Ok(())
}

fn unicode_key_input(unit: u16, key_up: bool) -> INPUT {
    let mut flags = KEYEVENTF_UNICODE;
    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }
    keyboard_input(VIRTUAL_KEY(0), unit, flags)
}

fn keyboard_input(vk: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_inputs(inputs: &[INPUT]) -> Result<(), InjectError> {
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err(InjectError(format!(
            "Windows SendInput delivered {sent} of {} events (input blocked by the foreground app or UIPI)",
            inputs.len()
        )))
    }
}

fn send_ctrl_v() -> Result<(), InjectError> {
    let inputs = [
        keyboard_input(VK_CONTROL, 0, KEYBD_EVENT_FLAGS(0)),
        keyboard_input(VK_V, 0, KEYBD_EVENT_FLAGS(0)),
        keyboard_input(VK_V, 0, KEYEVENTF_KEYUP),
        keyboard_input(VK_CONTROL, 0, KEYEVENTF_KEYUP),
    ];
    send_inputs(&inputs)
}

// ─────────────────────────────── clipboard fallback ────────────────────────

/// Win32 clipboard behind the shared [`Clipboard`] trait. Text-only snapshot
/// (the trait's MVP contract); to honor Pitfall P2 without a full multi-format
/// snapshot, `set_text` REFUSES when the clipboard holds non-text content it
/// could not restore, and `restore(None)` only clears what we ourselves set.
#[derive(Debug, Default)]
struct WindowsClipboard {
    /// True once we replaced the clipboard contents (so an empty snapshot is
    /// restored by clearing; before that, restore must not touch anything).
    mutated: bool,
}

impl Clipboard for WindowsClipboard {
    fn snapshot(&mut self) -> ClipboardSnapshot {
        ClipboardSnapshot(read_clipboard_text())
    }

    fn set_text(&mut self, text: &str) -> Result<(), InjectError> {
        let guard = ClipboardGuard::open()?;
        let format_count = unsafe { CountClipboardFormats() };
        let has_text = unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) }.is_ok();
        if format_count > 0 && !has_text {
            return Err(InjectError(
                "clipboard holds non-text content Kaydence cannot restore — refusing the \
                 clipboard fallback (Pitfall P2)"
                    .to_string(),
            ));
        }
        write_clipboard_text(&guard, text)?;
        self.mutated = true;
        Ok(())
    }

    fn restore(&mut self, snap: ClipboardSnapshot) -> Result<(), InjectError> {
        match snap.0 {
            Some(text) => {
                let guard = ClipboardGuard::open()?;
                write_clipboard_text(&guard, &text)
            }
            None if self.mutated => {
                let _guard = ClipboardGuard::open()?;
                unsafe { EmptyClipboard() }
                    .map_err(|e| InjectError(format!("Windows clipboard clear failed: {e}")))
            }
            None => Ok(()),
        }
    }
}

/// Open/close RAII for the Win32 clipboard, with a short retry because another
/// process may hold it momentarily.
struct ClipboardGuard;

impl ClipboardGuard {
    fn open() -> Result<Self, InjectError> {
        for attempt in 0..10 {
            if unsafe { OpenClipboard(None) }.is_ok() {
                return Ok(Self);
            }
            thread::sleep(Duration::from_millis(5 * (attempt + 1)));
        }
        Err(InjectError(
            "Windows clipboard is held by another process".to_string(),
        ))
    }
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        let _ = unsafe { CloseClipboard() };
    }
}

fn read_clipboard_text() -> Option<String> {
    let _guard = ClipboardGuard::open().ok()?;
    unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) }.ok()?;
    let handle = unsafe { GetClipboardData(CF_UNICODETEXT) }.ok()?;
    let global = HGLOBAL(handle.0);
    let locked = unsafe { GlobalLock(global) };
    if locked.is_null() {
        return None;
    }
    let mut units = Vec::new();
    let mut cursor = locked as *const u16;
    // The clipboard contract guarantees NUL termination for CF_UNICODETEXT.
    unsafe {
        while *cursor != 0 {
            units.push(*cursor);
            cursor = cursor.add(1);
        }
        let _ = GlobalUnlock(global);
    }
    Some(String::from_utf16_lossy(&units))
}

/// Write text as `CF_UNICODETEXT`. The `_guard` proves the clipboard is open.
fn write_clipboard_text(_guard: &ClipboardGuard, text: &str) -> Result<(), InjectError> {
    unsafe { EmptyClipboard() }
        .map_err(|e| InjectError(format!("Windows clipboard clear failed: {e}")))?;
    let units: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = units.len() * std::mem::size_of::<u16>();
    let global = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) }
        .map_err(|e| InjectError(format!("Windows clipboard allocation failed: {e}")))?;
    let locked = unsafe { GlobalLock(global) };
    if locked.is_null() {
        return Err(InjectError(
            "Windows clipboard allocation could not be locked".to_string(),
        ));
    }
    unsafe {
        std::ptr::copy_nonoverlapping(units.as_ptr(), locked as *mut u16, units.len());
        let _ = GlobalUnlock(global);
    }
    // On success the system owns the allocation; it must not be freed here.
    unsafe { SetClipboardData(CF_UNICODETEXT, Some(HANDLE(global.0))) }
        .map_err(|e| InjectError(format!("Windows clipboard write failed: {e}")))?;
    Ok(())
}

// ─────────────────────────────── foreground app ────────────────────────────

/// Identify the foreground app for per-app profiles + focus binding:
/// `GetForegroundWindow` → owning process → executable name. `id` is the
/// executable file name (the stable per-app key on Windows, e.g. `Code.exe`);
/// `name` is the file stem for the HUD. Shared with `profiles/` (its
/// `WindowsFrontmostAppDetector` calls this — the blessed common helper).
pub fn foreground_app() -> Option<AppRef> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return None;
    }
    let path = window_process_image(hwnd)?;
    let file_name = path.rsplit(['\\', '/']).next()?.to_string();
    let name = file_name
        .strip_suffix(".exe")
        .unwrap_or(&file_name)
        .to_string();
    if file_name.is_empty() || name.is_empty() {
        return None;
    }
    Some(AppRef {
        id: file_name,
        name,
    })
}

fn window_process_image(hwnd: HWND) -> Option<String> {
    let mut pid = 0u32;
    if unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) } == 0 || pid == 0 {
        return None;
    }
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut buffer = [0u16; 1024];
    let mut length = buffer.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    };
    let _ = unsafe { CloseHandle(process) };
    result.ok()?;
    Some(String::from_utf16_lossy(&buffer[..length as usize]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(control_type: i32) -> UiaFocusFacts {
        UiaFocusFacts {
            control_type,
            is_password: false,
            is_enabled: true,
            has_value_pattern: true,
            value_read_only: false,
            has_text_pattern: true,
        }
    }

    // ── the UIA-property → FieldKind mapping (non-negotiable #8) ──

    #[test]
    fn password_control_is_secure_regardless_of_everything_else() {
        let mut probe = facts(UIA_EditControlTypeId.0);
        probe.is_password = true;
        assert_eq!(classify_uia_focus(&probe), FieldKind::Secure);
        // Even a "read-only, disabled, pattern-free" password stays Secure.
        probe.is_enabled = false;
        probe.has_value_pattern = false;
        probe.value_read_only = true;
        probe.has_text_pattern = false;
        assert_eq!(classify_uia_focus(&probe), FieldKind::Secure);
        assert!(!supports_native_insert(&probe));
    }

    #[test]
    fn unreadable_password_bit_fails_closed() {
        // gather_facts maps an unreadable IsPassword to is_password=true; the
        // classifier must then refuse. This test pins the classifier half.
        let mut probe = facts(UIA_EditControlTypeId.0);
        probe.is_password = true;
        assert_eq!(classify_uia_focus(&probe), FieldKind::Secure);
    }

    #[test]
    fn text_entry_control_types_are_editable() {
        for ct in [
            UIA_EditControlTypeId.0,
            UIA_DocumentControlTypeId.0,
            UIA_ComboBoxControlTypeId.0,
        ] {
            assert_eq!(classify_uia_focus(&facts(ct)), FieldKind::Editable);
        }
    }

    #[test]
    fn writable_value_provider_is_editable_even_for_custom_control_types() {
        let custom = facts(50000); // UIA_ButtonControlTypeId — but with a writable value
        assert_eq!(classify_uia_focus(&custom), FieldKind::Editable);
    }

    #[test]
    fn disabled_or_patternless_non_text_controls_are_no_target() {
        let mut disabled = facts(UIA_EditControlTypeId.0);
        disabled.is_enabled = false;
        assert_eq!(classify_uia_focus(&disabled), FieldKind::NoTarget);

        let mut button = facts(50000);
        button.has_value_pattern = false;
        button.has_text_pattern = false;
        assert_eq!(classify_uia_focus(&button), FieldKind::NoTarget);

        let mut read_only_value = facts(50000);
        read_only_value.value_read_only = true;
        assert_eq!(classify_uia_focus(&read_only_value), FieldKind::NoTarget);
    }

    #[test]
    fn native_insert_requires_a_writable_value_pattern() {
        assert!(supports_native_insert(&facts(UIA_EditControlTypeId.0)));

        let mut no_value = facts(UIA_DocumentControlTypeId.0);
        no_value.has_value_pattern = false;
        assert_eq!(classify_uia_focus(&no_value), FieldKind::Editable);
        assert!(!supports_native_insert(&no_value)); // keystroke path instead

        let mut read_only = facts(UIA_EditControlTypeId.0);
        read_only.value_read_only = true;
        assert!(!supports_native_insert(&read_only));
    }

    // ── keystroke text preparation ──

    #[test]
    fn newlines_normalize_to_carriage_returns() {
        assert_eq!(normalize_newlines_for_keystrokes("a\r\nb\nc"), "a\rb\rc");
        assert_eq!(normalize_newlines_for_keystrokes("plain"), "plain");
    }

    #[test]
    fn unicode_text_round_trips_through_utf16_units() {
        let text = "Kaydence ✓ 🪄 العربية かな";
        let units: Vec<u16> = text.encode_utf16().collect();
        assert_eq!(String::from_utf16(&units).unwrap(), text);
        // Each unit becomes a down+up pair; surrogate halves are separate
        // KEYEVENTF_UNICODE events that the target reassembles.
        assert_eq!(units.len() * 2, text.encode_utf16().count() * 2);
    }

    // ── live smoke (safe headless: probes only, never injects) ──

    #[test]
    fn focused_field_probe_does_not_panic_headless() {
        // On CI this may be Unknown (no UIA session) or whatever has focus;
        // the contract is only that probing never panics or hangs.
        let injector = WindowsTextInjector;
        let _ = injector.focused_field();
        let _ = injector.caps();
    }

    #[test]
    fn foreground_app_probe_does_not_panic_headless() {
        if let Some(app) = foreground_app() {
            assert!(!app.id.is_empty());
            assert!(!app.name.is_empty());
        }
    }
}
