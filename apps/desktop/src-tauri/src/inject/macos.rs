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

use super::{FieldKind, InjectError, InjectorCaps, KeystrokeChannel, TextInjector};

const CFSTRING_ENCODING_UTF8: CFStringEncoding = 0x0800_0100;
const AX_ERROR_SUCCESS: AXError = 0;

const ATTR_FOCUSED_UI_ELEMENT: &str = "AXFocusedUIElement";
const ATTR_ROLE: &str = "AXRole";
const ATTR_SUBROLE: &str = "AXSubrole";
const ATTR_SELECTED_TEXT: &str = "AXSelectedText";

type AXError = i32;
type AXUIElementRef = *const c_void;
type Boolean = u8;
type CFIndex = isize;
type CFStringEncoding = u32;
type CFStringRef = *const c_void;
type CFTypeID = usize;
type CFTypeRef = *const c_void;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
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

impl TextInjector for MacOsTextInjector {
    fn caps(&self) -> InjectorCaps {
        let native_text_insert = focused_ax_element()
            .filter(|element| element.field_kind() == FieldKind::Editable)
            .is_some_and(|element| element.is_attribute_settable(ATTR_SELECTED_TEXT));

        InjectorCaps {
            native_text_insert,
            keystroke: KeystrokeChannel::None,
            clipboard: false,
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

    fn synth_text(&mut self, _text: &str) -> Result<(), InjectError> {
        Err(InjectError(
            "macOS CGEvent text synthesis not implemented".to_string(),
        ))
    }
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
}
