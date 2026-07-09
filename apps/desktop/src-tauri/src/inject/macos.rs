//! macOS Accessibility text-injection backend.
//!
//! The backend intentionally uses `AXSelectedText` for native insertion instead
//! of `AXValue` so it inserts at the caret or replaces the current selection
//! without overwriting an entire field. Secure or unsupported focus resolves to
//! `NoTarget`/`Secure`, keeping the shared policy in fail-closed control.
#![cfg(target_os = "macos")]
#![allow(dead_code)]

use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::thread;
use std::time::Duration;

use objc2::rc::{autoreleasepool, Retained};
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardTypeString, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSData, NSString};

use super::{
    FieldKind, InjectError, InjectorCaps, KeystrokeChannel, PlatformPermissionProof, TextInjector,
};

const CFSTRING_ENCODING_UTF8: CFStringEncoding = 0x0800_0100;
const AX_ERROR_SUCCESS: AXError = 0;
const CG_HID_EVENT_TAP: CGEventTapLocation = 0;
const CG_COMMAND_FLAG: CGEventFlags = 0x0010_0000;
const CG_UNICODE_KEY_CODE: CGKeyCode = 0;
const CG_V_KEY_CODE: CGKeyCode = 0x09;
const CLIPBOARD_RESTORE_DELAY_MS: u64 = 20;
const MAX_UNICHARS_PER_EVENT: usize = 512;

const ATTR_FOCUSED_UI_ELEMENT: &str = "AXFocusedUIElement";
const ATTR_ROLE: &str = "AXRole";
const ATTR_SUBROLE: &str = "AXSubrole";
const ATTR_SELECTED_TEXT: &str = "AXSelectedText";

type AXError = i32;
type AXUIElementRef = *const c_void;
type Boolean = u8;
type CGEventFlags = u64;
type CGEventRef = *const c_void;
type CGEventSourceRef = *const c_void;
type CGEventTapLocation = u32;
type CGKeyCode = u16;
type CFIndex = isize;
type CFStringEncoding = u32;
type CFStringRef = *const c_void;
type CFTypeID = usize;
type CFTypeRef = *const c_void;
type UniChar = u16;
type UniCharCount = usize;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    #[link_name = "AXIsProcessTrusted"]
    fn ax_is_process_trusted() -> Boolean;

    #[link_name = "AXUIElementCreateSystemWide"]
    fn axui_element_create_system_wide() -> AXUIElementRef;

    #[link_name = "AXUIElementCopyAttributeValue"]
    fn axui_element_copy_attribute_value(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> AXError;

    #[link_name = "AXUIElementSetAttributeValue"]
    fn axui_element_set_attribute_value(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> AXError;

    #[link_name = "AXUIElementIsAttributeSettable"]
    fn axui_element_is_attribute_settable(
        element: AXUIElementRef,
        attribute: CFStringRef,
        settable: *mut Boolean,
    ) -> AXError;

    #[link_name = "CGEventCreateKeyboardEvent"]
    fn cg_event_create_keyboard_event(
        source: CGEventSourceRef,
        virtual_key: CGKeyCode,
        key_down: bool,
    ) -> CGEventRef;

    #[link_name = "CGEventKeyboardSetUnicodeString"]
    fn cg_event_keyboard_set_unicode_string(
        event: CGEventRef,
        string_length: UniCharCount,
        unicode_string: *const UniChar,
    );

    #[link_name = "CGEventPost"]
    fn cg_event_post(tap: CGEventTapLocation, event: CGEventRef);

    #[link_name = "CGEventSetFlags"]
    fn cg_event_set_flags(event: CGEventRef, flags: CGEventFlags);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    #[link_name = "CFGetTypeID"]
    fn cf_get_type_id(cf: CFTypeRef) -> CFTypeID;

    #[link_name = "CFRelease"]
    fn cf_release(cf: CFTypeRef);

    #[link_name = "CFStringCreateWithBytes"]
    fn cf_string_create_with_bytes(
        alloc: CFTypeRef,
        bytes: *const u8,
        num_bytes: CFIndex,
        encoding: CFStringEncoding,
        is_external_representation: Boolean,
    ) -> CFStringRef;

    #[link_name = "CFStringGetCString"]
    fn cf_string_get_c_string(
        string: CFStringRef,
        buffer: *mut c_char,
        buffer_size: CFIndex,
        encoding: CFStringEncoding,
    ) -> Boolean;

    #[link_name = "CFStringGetLength"]
    fn cf_string_get_length(string: CFStringRef) -> CFIndex;

    #[link_name = "CFStringGetMaximumSizeForEncoding"]
    fn cf_string_get_maximum_size_for_encoding(
        length: CFIndex,
        encoding: CFStringEncoding,
    ) -> CFIndex;

    #[link_name = "CFStringGetTypeID"]
    fn cf_string_get_type_id() -> CFTypeID;
}

#[derive(Debug, Default)]
pub struct MacOsTextInjector;

pub fn accessibility_permission_ready() -> bool {
    // Preflight only; prompting remains owned by the first-run UI/settings path.
    unsafe { ax_is_process_trusted() != 0 }
}

pub fn platform_permission_proofs() -> Vec<PlatformPermissionProof> {
    let mut proofs = Vec::new();
    if accessibility_permission_ready() {
        proofs.push(PlatformPermissionProof {
            requirement_id: "accessibility",
            detail:
                "Runtime proof observed: macOS Accessibility preflight trusts this Kaydence process.",
            action: "No action needed; Accessibility proof is recorded for this runtime.",
        });
    }
    proofs
}

impl TextInjector for MacOsTextInjector {
    fn caps(&self) -> InjectorCaps {
        let element = focused_ax_element();
        let focused_kind = element
            .as_ref()
            .map(|element| element.field_kind())
            .unwrap_or(FieldKind::NoTarget);
        let native_text_insert = focused_kind == FieldKind::Editable
            && element
                .as_ref()
                .is_some_and(|element| element.is_attribute_settable(ATTR_SELECTED_TEXT));
        let keystroke = if focused_kind == FieldKind::Editable {
            KeystrokeChannel::MacOsEvent
        } else {
            KeystrokeChannel::None
        };

        InjectorCaps {
            native_text_insert,
            keystroke,
            clipboard: focused_kind == FieldKind::Editable,
        }
    }

    fn focused_field(&self) -> FieldKind {
        focused_ax_element()
            .map(|element| element.field_kind())
            .unwrap_or(FieldKind::NoTarget)
    }

    fn insert_native(&mut self, text: &str) -> Result<(), InjectError> {
        let Some(element) = focused_ax_element() else {
            return Err(InjectError(
                "macOS Accessibility focused element unavailable".to_string(),
            ));
        };

        match element.field_kind() {
            FieldKind::Editable => {}
            FieldKind::Secure => {
                return Err(InjectError(
                    "macOS native insert refused: secure field focused".to_string(),
                ));
            }
            FieldKind::NoTarget | FieldKind::Unknown => {
                return Err(InjectError(
                    "macOS native insert refused: no editable field focused".to_string(),
                ));
            }
        }

        if !element.is_attribute_settable(ATTR_SELECTED_TEXT) {
            return Err(InjectError(
                "macOS focused field does not support AXSelectedText insertion".to_string(),
            ));
        }

        let attribute = OwnedCfString::new(ATTR_SELECTED_TEXT)?;
        let value = OwnedCfString::new(text)?;
        let error = unsafe {
            axui_element_set_attribute_value(element.as_ax(), attribute.as_ref(), value.as_type())
        };
        if error == AX_ERROR_SUCCESS {
            Ok(())
        } else {
            Err(InjectError(format!(
                "macOS AXSelectedText insert failed: AXError {error}"
            )))
        }
    }

    fn synth_text(&mut self, text: &str) -> Result<(), InjectError> {
        let Some(element) = focused_ax_element() else {
            return Err(InjectError(
                "macOS Accessibility focused element unavailable".to_string(),
            ));
        };

        match element.field_kind() {
            FieldKind::Editable => post_unicode_text(text),
            FieldKind::Secure => Err(InjectError(
                "macOS CGEvent insert refused: secure field focused".to_string(),
            )),
            FieldKind::NoTarget | FieldKind::Unknown => Err(InjectError(
                "macOS CGEvent insert refused: no editable field focused".to_string(),
            )),
        }
    }

    fn paste_clipboard(&mut self, text: &str) -> Result<(), InjectError> {
        require_editable_focus("macOS clipboard paste")?;

        let snap = snapshot_clipboard()?;
        if let Err(err) = set_clipboard_text_for_paste(text) {
            let _ = restore_clipboard(snap);
            return Err(err);
        }

        let pasted = post_command_v();
        thread::sleep(Duration::from_millis(CLIPBOARD_RESTORE_DELAY_MS));
        let restored = restore_clipboard(snap);
        pasted.and(restored)
    }
}

fn require_editable_focus(action: &str) -> Result<(), InjectError> {
    let Some(element) = focused_ax_element() else {
        return Err(InjectError(format!(
            "{action} refused: Accessibility focused element unavailable"
        )));
    };

    match element.field_kind() {
        FieldKind::Editable => Ok(()),
        FieldKind::Secure => Err(InjectError(format!(
            "{action} refused: secure field focused"
        ))),
        FieldKind::NoTarget | FieldKind::Unknown => Err(InjectError(format!(
            "{action} refused: no editable field focused"
        ))),
    }
}

fn post_unicode_text(text: &str) -> Result<(), InjectError> {
    for chunk in utf16_event_chunks(text, MAX_UNICHARS_PER_EVENT) {
        post_unicode_chunk(&chunk)?;
    }
    Ok(())
}

fn post_unicode_chunk(chunk: &[u16]) -> Result<(), InjectError> {
    if chunk.is_empty() {
        return Ok(());
    }

    let key_down = create_keyboard_event(true)?;
    unsafe {
        cg_event_keyboard_set_unicode_string(
            key_down.as_event(),
            chunk.len() as UniCharCount,
            chunk.as_ptr(),
        );
        cg_event_post(CG_HID_EVENT_TAP, key_down.as_event());
    }

    let key_up = create_keyboard_event(false)?;
    unsafe {
        cg_event_post(CG_HID_EVENT_TAP, key_up.as_event());
    }

    Ok(())
}

fn create_keyboard_event(key_down: bool) -> Result<OwnedCfType, InjectError> {
    let event = unsafe {
        cg_event_create_keyboard_event(ptr::null(), CG_UNICODE_KEY_CODE, key_down) as CFTypeRef
    };
    OwnedCfType::new(event)
        .ok_or_else(|| InjectError("macOS CGEvent keyboard event could not be created".to_string()))
}

fn post_command_v() -> Result<(), InjectError> {
    let key_down = create_key_event_with_flags(CG_V_KEY_CODE, true, CG_COMMAND_FLAG)?;
    unsafe {
        cg_event_post(CG_HID_EVENT_TAP, key_down.as_event());
    }

    let key_up = create_key_event_with_flags(CG_V_KEY_CODE, false, CG_COMMAND_FLAG)?;
    unsafe {
        cg_event_post(CG_HID_EVENT_TAP, key_up.as_event());
    }

    Ok(())
}

fn create_key_event_with_flags(
    key_code: CGKeyCode,
    key_down: bool,
    flags: CGEventFlags,
) -> Result<OwnedCfType, InjectError> {
    let event =
        unsafe { cg_event_create_keyboard_event(ptr::null(), key_code, key_down) as CFTypeRef };
    let event = OwnedCfType::new(event).ok_or_else(|| {
        InjectError("macOS CGEvent keyboard event could not be created".to_string())
    })?;
    unsafe {
        cg_event_set_flags(event.as_event(), flags);
    }
    Ok(event)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MacOsClipboardSnapshot {
    Empty,
    Items(Vec<MacOsPasteboardItemSnapshot>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MacOsPasteboardItemSnapshot {
    flavors: Vec<MacOsPasteboardFlavor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MacOsPasteboardFlavor {
    type_id: String,
    data: Vec<u8>,
}

fn snapshot_clipboard() -> Result<MacOsClipboardSnapshot, InjectError> {
    autoreleasepool(|_| {
        let pasteboard = NSPasteboard::generalPasteboard();
        let Some(items) = pasteboard.pasteboardItems() else {
            return Ok(MacOsClipboardSnapshot::Empty);
        };

        let mut snapshots = Vec::new();
        for item in &items {
            let mut flavors = Vec::new();
            for pasteboard_type in item.types().iter() {
                let type_id = pasteboard_type.to_string();
                let data = item.dataForType(&pasteboard_type).ok_or_else(|| {
                    InjectError(format!(
                        "macOS clipboard type {type_id} could not be read for restore"
                    ))
                })?;
                flavors.push(MacOsPasteboardFlavor {
                    type_id,
                    data: data.to_vec(),
                });
            }
            snapshots.push(MacOsPasteboardItemSnapshot { flavors });
        }

        if snapshots.is_empty() {
            Ok(MacOsClipboardSnapshot::Empty)
        } else {
            Ok(MacOsClipboardSnapshot::Items(snapshots))
        }
    })
}

fn restore_clipboard(snapshot: MacOsClipboardSnapshot) -> Result<(), InjectError> {
    autoreleasepool(|_| {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();

        let MacOsClipboardSnapshot::Items(items) = snapshot else {
            return Ok(());
        };

        let pasteboard_items = build_pasteboard_items(&items)?;
        if pasteboard_items.is_empty() {
            return Ok(());
        }

        let writing_items = pasteboard_items
            .into_iter()
            .map(ProtocolObject::<dyn NSPasteboardWriting>::from_retained)
            .collect::<Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>>>();
        let writing_array = NSArray::from_retained_slice(&writing_items);
        if pasteboard.writeObjects(&writing_array) {
            Ok(())
        } else {
            Err(InjectError(
                "macOS clipboard restore writeObjects failed".to_string(),
            ))
        }
    })
}

fn set_clipboard_text_for_paste(text: &str) -> Result<(), InjectError> {
    autoreleasepool(|_| {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        let ns_text = NSString::from_str(text);
        let string_type = unsafe { NSPasteboardTypeString };
        if pasteboard.setString_forType(&ns_text, string_type) {
            Ok(())
        } else {
            Err(InjectError(
                "macOS clipboard text write failed before paste".to_string(),
            ))
        }
    })
}

fn build_pasteboard_items(
    items: &[MacOsPasteboardItemSnapshot],
) -> Result<Vec<Retained<NSPasteboardItem>>, InjectError> {
    let mut restored = Vec::new();
    for item in items {
        let pasteboard_item = NSPasteboardItem::new();
        for flavor in &item.flavors {
            let pasteboard_type = NSString::from_str(&flavor.type_id);
            let data = NSData::with_bytes(&flavor.data);
            if !pasteboard_item.setData_forType(&data, &pasteboard_type) {
                return Err(InjectError(format!(
                    "macOS clipboard restore failed for pasteboard type {}",
                    flavor.type_id
                )));
            }
        }
        restored.push(pasteboard_item);
    }
    Ok(restored)
}

fn utf16_event_chunks(text: &str, limit: usize) -> Vec<Vec<u16>> {
    assert!(limit > 0, "unicode event chunk limit must be positive");

    let mut chunks = Vec::new();
    let mut current = Vec::new();
    for ch in text.chars() {
        let mut encoded = [0; 2];
        let units = ch.encode_utf16(&mut encoded);
        if !current.is_empty() && current.len() + units.len() > limit {
            chunks.push(current);
            current = Vec::new();
        }
        current.extend_from_slice(units);
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

fn classify_ax_focus(role: Option<&str>, subrole: Option<&str>) -> FieldKind {
    if matches!(role, Some("AXSecureTextField")) || matches!(subrole, Some("AXSecureTextField")) {
        return FieldKind::Secure;
    }

    if matches!(
        role,
        Some("AXTextField" | "AXTextArea" | "AXComboBox" | "AXSearchField")
    ) || matches!(subrole, Some("AXSearchField"))
    {
        return FieldKind::Editable;
    }

    FieldKind::NoTarget
}

fn focused_ax_element() -> Option<FocusedAxElement> {
    let system = unsafe { axui_element_create_system_wide() };
    let system = OwnedCfType::new(system as CFTypeRef)?;
    copy_attribute(system.as_ax(), ATTR_FOCUSED_UI_ELEMENT)
        .ok()
        .map(FocusedAxElement::new)
}

fn copy_attribute(element: AXUIElementRef, attribute: &str) -> Result<OwnedCfType, AXError> {
    let attribute = OwnedCfString::new(attribute).map_err(|_| -1)?;
    let mut value = ptr::null();
    let error =
        unsafe { axui_element_copy_attribute_value(element, attribute.as_ref(), &mut value) };
    if error == AX_ERROR_SUCCESS {
        OwnedCfType::new(value).ok_or(-1)
    } else {
        Err(error)
    }
}

struct FocusedAxElement {
    element: OwnedCfType,
}

impl FocusedAxElement {
    fn new(element: OwnedCfType) -> Self {
        Self { element }
    }

    fn as_ax(&self) -> AXUIElementRef {
        self.element.as_ax()
    }

    fn field_kind(&self) -> FieldKind {
        classify_ax_focus(
            self.attribute_string(ATTR_ROLE).as_deref(),
            self.attribute_string(ATTR_SUBROLE).as_deref(),
        )
    }

    fn attribute_string(&self, attribute: &str) -> Option<String> {
        let value = copy_attribute(self.as_ax(), attribute).ok()?;
        cf_string_to_string(value.as_type())
    }

    fn is_attribute_settable(&self, attribute: &str) -> bool {
        let Ok(attribute) = OwnedCfString::new(attribute) else {
            return false;
        };
        let mut settable = 0;
        let error = unsafe {
            axui_element_is_attribute_settable(self.as_ax(), attribute.as_ref(), &mut settable)
        };
        error == AX_ERROR_SUCCESS && settable != 0
    }
}

struct OwnedCfString(CFStringRef);

impl OwnedCfString {
    fn new(value: &str) -> Result<Self, InjectError> {
        let value = unsafe {
            cf_string_create_with_bytes(
                ptr::null(),
                value.as_ptr(),
                value.len() as CFIndex,
                CFSTRING_ENCODING_UTF8,
                0,
            )
        };

        if value.is_null() {
            Err(InjectError(
                "failed to create CoreFoundation string".to_string(),
            ))
        } else {
            Ok(Self(value))
        }
    }

    fn as_ref(&self) -> CFStringRef {
        self.0
    }

    fn as_type(&self) -> CFTypeRef {
        self.0 as CFTypeRef
    }
}

impl Drop for OwnedCfString {
    fn drop(&mut self) {
        unsafe { cf_release(self.as_type()) };
    }
}

struct OwnedCfType(CFTypeRef);

impl OwnedCfType {
    fn new(value: CFTypeRef) -> Option<Self> {
        (!value.is_null()).then_some(Self(value))
    }

    fn as_ax(&self) -> AXUIElementRef {
        self.0 as AXUIElementRef
    }

    fn as_type(&self) -> CFTypeRef {
        self.0
    }

    fn as_event(&self) -> CGEventRef {
        self.0 as CGEventRef
    }
}

impl Drop for OwnedCfType {
    fn drop(&mut self) {
        unsafe { cf_release(self.0) };
    }
}

fn cf_string_to_string(value: CFTypeRef) -> Option<String> {
    if value.is_null() {
        return None;
    }

    let is_string = unsafe { cf_get_type_id(value) == cf_string_get_type_id() };
    if !is_string {
        return None;
    }

    let string = value as CFStringRef;
    let length = unsafe { cf_string_get_length(string) };
    let max_size =
        unsafe { cf_string_get_maximum_size_for_encoding(length, CFSTRING_ENCODING_UTF8) };
    let buffer_size = max_size.checked_add(1)?;
    let mut buffer = vec![0; buffer_size as usize];
    let ok = unsafe {
        cf_string_get_c_string(
            string,
            buffer.as_mut_ptr(),
            buffer_size,
            CFSTRING_ENCODING_UTF8,
        )
    };

    if ok == 0 {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(buffer.as_ptr()) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_text_subrole_is_secure() {
        assert_eq!(
            classify_ax_focus(Some("AXTextField"), Some("AXSecureTextField")),
            FieldKind::Secure
        );
    }

    #[test]
    fn text_roles_are_editable_targets() {
        assert_eq!(
            classify_ax_focus(Some("AXTextField"), None),
            FieldKind::Editable
        );
        assert_eq!(
            classify_ax_focus(Some("AXTextArea"), None),
            FieldKind::Editable
        );
        assert_eq!(
            classify_ax_focus(Some("AXComboBox"), None),
            FieldKind::Editable
        );
    }

    #[test]
    fn search_subrole_is_editable() {
        assert_eq!(
            classify_ax_focus(Some("AXTextField"), Some("AXSearchField")),
            FieldKind::Editable
        );
    }

    #[test]
    fn unsupported_or_missing_role_is_no_target() {
        assert_eq!(
            classify_ax_focus(Some("AXButton"), None),
            FieldKind::NoTarget
        );
        assert_eq!(classify_ax_focus(None, None), FieldKind::NoTarget);
    }

    #[test]
    fn unicode_chunks_preserve_text_roundtrip() {
        let text = "Kaydence 🪄 العربية かな";
        let chunks = utf16_event_chunks(text, 5);
        let stitched = chunks.into_iter().flatten().collect::<Vec<_>>();

        assert_eq!(String::from_utf16(&stitched).unwrap(), text);
    }

    #[test]
    fn unicode_chunks_do_not_split_surrogate_pairs() {
        let chunks = utf16_event_chunks("ab🪄cd", 3);

        assert!(chunks.iter().all(|chunk| !chunk
            .last()
            .is_some_and(|unit| (0xD800..=0xDBFF).contains(unit))));
        assert!(chunks.iter().all(|chunk| !chunk
            .first()
            .is_some_and(|unit| (0xDC00..=0xDFFF).contains(unit))));
        let stitched = chunks.into_iter().flatten().collect::<Vec<_>>();
        assert_eq!(String::from_utf16(&stitched).unwrap(), "ab🪄cd");
    }

    #[test]
    fn empty_text_creates_no_unicode_chunks() {
        assert!(utf16_event_chunks("", 8).is_empty());
    }

    #[test]
    fn pasteboard_snapshot_can_represent_empty_clipboard() {
        assert_eq!(MacOsClipboardSnapshot::Empty, MacOsClipboardSnapshot::Empty);
    }

    #[test]
    fn pasteboard_snapshot_preserves_multiple_items_and_types() {
        let snapshot = MacOsClipboardSnapshot::Items(vec![
            MacOsPasteboardItemSnapshot {
                flavors: vec![
                    MacOsPasteboardFlavor {
                        type_id: "public.utf8-plain-text".to_string(),
                        data: b"hello".to_vec(),
                    },
                    MacOsPasteboardFlavor {
                        type_id: "public.html".to_string(),
                        data: b"<b>hello</b>".to_vec(),
                    },
                ],
            },
            MacOsPasteboardItemSnapshot {
                flavors: vec![MacOsPasteboardFlavor {
                    type_id: "public.png".to_string(),
                    data: vec![137, 80, 78, 71],
                }],
            },
        ]);

        let MacOsClipboardSnapshot::Items(items) = snapshot else {
            panic!("expected item snapshot");
        };

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].flavors.len(), 2);
        assert_eq!(items[0].flavors[1].type_id, "public.html");
        assert_eq!(items[1].flavors[0].data, vec![137, 80, 78, 71]);
    }

    #[test]
    fn restore_builder_accepts_plain_data_flavors() {
        let items = vec![MacOsPasteboardItemSnapshot {
            flavors: vec![MacOsPasteboardFlavor {
                type_id: "public.utf8-plain-text".to_string(),
                data: b"Kaydence".to_vec(),
            }],
        }];

        autoreleasepool(|_| {
            let rebuilt = build_pasteboard_items(&items).unwrap();
            assert_eq!(rebuilt.len(), 1);
            let ty = NSString::from_str("public.utf8-plain-text");
            assert_eq!(
                rebuilt[0].dataForType(&ty).unwrap().to_vec(),
                b"Kaydence".to_vec()
            );
        });
    }

    #[test]
    fn restore_builder_keeps_each_item_separate() {
        let items = vec![
            MacOsPasteboardItemSnapshot {
                flavors: vec![MacOsPasteboardFlavor {
                    type_id: "public.utf8-plain-text".to_string(),
                    data: b"first".to_vec(),
                }],
            },
            MacOsPasteboardItemSnapshot {
                flavors: vec![MacOsPasteboardFlavor {
                    type_id: "public.utf8-plain-text".to_string(),
                    data: b"second".to_vec(),
                }],
            },
        ];

        autoreleasepool(|_| {
            let rebuilt = build_pasteboard_items(&items).unwrap();
            let ty = NSString::from_str("public.utf8-plain-text");
            assert_eq!(rebuilt.len(), 2);
            assert_eq!(
                rebuilt[0].dataForType(&ty).unwrap().to_vec(),
                b"first".to_vec()
            );
            assert_eq!(
                rebuilt[1].dataForType(&ty).unwrap().to_vec(),
                b"second".to_vec()
            );
        });
    }

    #[test]
    fn restore_builder_accepts_multiple_flavors_on_one_item() {
        let items = vec![MacOsPasteboardItemSnapshot {
            flavors: vec![
                MacOsPasteboardFlavor {
                    type_id: "public.utf8-plain-text".to_string(),
                    data: b"text".to_vec(),
                },
                MacOsPasteboardFlavor {
                    type_id: "public.rtf".to_string(),
                    data: b"{\\rtf1 text}".to_vec(),
                },
            ],
        }];

        autoreleasepool(|_| {
            let rebuilt = build_pasteboard_items(&items).unwrap();
            let types = rebuilt[0]
                .types()
                .iter()
                .map(|ty| ty.to_string())
                .collect::<Vec<_>>();

            assert_eq!(
                types,
                vec![
                    "public.utf8-plain-text".to_string(),
                    "public.rtf".to_string()
                ]
            );
        });
    }

    #[test]
    fn pasteboard_snapshot_equality_includes_type_and_data() {
        let left = MacOsPasteboardFlavor {
            type_id: "public.data".to_string(),
            data: vec![1],
        };
        let same_type_different_data = MacOsPasteboardFlavor {
            type_id: "public.data".to_string(),
            data: vec![2],
        };
        let same_data_different_type = MacOsPasteboardFlavor {
            type_id: "public.png".to_string(),
            data: vec![1],
        };

        assert_ne!(left, same_type_different_data);
        assert_ne!(left, same_data_different_type);
    }

    #[test]
    fn pasteboard_snapshot_no_longer_rejects_mixed_content() {
        let snapshot = MacOsClipboardSnapshot::Items(vec![MacOsPasteboardItemSnapshot {
            flavors: vec![
                MacOsPasteboardFlavor {
                    type_id: "public.utf8-plain-text".to_string(),
                    data: b"caption".to_vec(),
                },
                MacOsPasteboardFlavor {
                    type_id: "public.png".to_string(),
                    data: vec![137, 80, 78, 71],
                },
            ],
        }]);

        assert!(matches!(snapshot, MacOsClipboardSnapshot::Items(_)));
    }

    #[test]
    fn pasteboard_snapshot_debug_includes_type_ids() {
        let snapshot = MacOsClipboardSnapshot::Items(vec![MacOsPasteboardItemSnapshot {
            flavors: vec![MacOsPasteboardFlavor {
                type_id: "public.file-url".to_string(),
                data: b"file:///tmp/example.txt".to_vec(),
            }],
        }]);

        assert!(format!("{snapshot:?}").contains("public.file-url"));
    }

    #[test]
    fn restore_builder_handles_empty_item_snapshots() {
        autoreleasepool(|_| {
            let rebuilt =
                build_pasteboard_items(&[MacOsPasteboardItemSnapshot { flavors: vec![] }]).unwrap();

            assert_eq!(rebuilt.len(), 1);
            assert_eq!(rebuilt[0].types().len(), 0);
        });
    }

    #[test]
    fn pasteboard_snapshot_text_and_binary_can_coexist() {
        let item = MacOsPasteboardItemSnapshot {
            flavors: vec![
                MacOsPasteboardFlavor {
                    type_id: "public.utf8-plain-text".to_string(),
                    data: b"plain".to_vec(),
                },
                MacOsPasteboardFlavor {
                    type_id: "public.tiff".to_string(),
                    data: vec![73, 73, 42, 0],
                },
            ],
        };

        assert_eq!(item.flavors.len(), 2);
        assert_eq!(item.flavors[0].data, b"plain".to_vec());
        assert_eq!(item.flavors[1].type_id, "public.tiff");
    }

    #[test]
    fn restore_builder_preserves_binary_data() {
        let items = vec![MacOsPasteboardItemSnapshot {
            flavors: vec![MacOsPasteboardFlavor {
                type_id: "public.data".to_string(),
                data: vec![0, 1, 2, 255],
            }],
        }];

        autoreleasepool(|_| {
            let rebuilt = build_pasteboard_items(&items).unwrap();
            let ty = NSString::from_str("public.data");
            assert_eq!(
                rebuilt[0].dataForType(&ty).unwrap().to_vec(),
                vec![0, 1, 2, 255]
            );
        });
    }

    #[test]
    fn restore_builder_preserves_declared_type_order() {
        let items = vec![MacOsPasteboardItemSnapshot {
            flavors: vec![
                MacOsPasteboardFlavor {
                    type_id: "public.html".to_string(),
                    data: b"<p>hi</p>".to_vec(),
                },
                MacOsPasteboardFlavor {
                    type_id: "public.utf8-plain-text".to_string(),
                    data: b"hi".to_vec(),
                },
            ],
        }];

        autoreleasepool(|_| {
            let rebuilt = build_pasteboard_items(&items).unwrap();
            let types = rebuilt[0]
                .types()
                .iter()
                .map(|ty| ty.to_string())
                .collect::<Vec<_>>();

            assert_eq!(
                types,
                vec![
                    "public.html".to_string(),
                    "public.utf8-plain-text".to_string()
                ]
            );
        });
    }

    #[test]
    fn pasteboard_snapshot_items_can_be_empty_vec() {
        assert_eq!(
            MacOsClipboardSnapshot::Items(vec![]),
            MacOsClipboardSnapshot::Items(vec![])
        );
    }
}
