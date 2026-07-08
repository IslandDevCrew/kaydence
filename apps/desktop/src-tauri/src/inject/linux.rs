//! Linux text-injection backend (P1-P0-3, ADR-0013). AT-SPI2 provides the
//! native text-insertion path AND the secure-field signal (non-negotiable #8).
//! Built + validated on the Debian-12 GNOME reference VM. This module is
//! compiled only on Linux (`atspi` is a cfg-gated dependency).
//!
//! Strategy + compositor matrix: docs/spikes/P1-P0-3-wayland-injection.md.
#![cfg(target_os = "linux")]
#![allow(dead_code)]

use super::FieldKind;

/// Map an AT-SPI role + editable state to our platform-agnostic [`FieldKind`].
/// A password role is secure regardless of the editable bit; a non-editable
/// object is not an injection target. Pure + total.
pub fn role_to_field_kind(role: atspi::Role, editable: bool) -> FieldKind {
    match role {
        atspi::Role::PasswordText => FieldKind::Secure,
        _ if editable => FieldKind::Editable,
        _ => FieldKind::NoTarget,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_role_is_secure_even_if_editable() {
        assert_eq!(
            role_to_field_kind(atspi::Role::PasswordText, true),
            FieldKind::Secure
        );
    }

    #[test]
    fn editable_entry_is_a_target() {
        assert_eq!(
            role_to_field_kind(atspi::Role::Entry, true),
            FieldKind::Editable
        );
    }

    #[test]
    fn non_editable_is_no_target() {
        assert_eq!(
            role_to_field_kind(atspi::Role::Label, false),
            FieldKind::NoTarget
        );
    }
}
