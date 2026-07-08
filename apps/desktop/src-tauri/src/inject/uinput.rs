//! uinput keystroke synthesis (P1-P0-3, ADR-0013 amendment) — the validated
//! primary insertion path on Wayland (AT-SPI `InsertText` no-ops on modern
//! GNOME). A virtual keyboard is created under `/dev/uinput`; text is typed as
//! key events below the compositor, so it works regardless of compositor
//! (wlroots, GNOME/Mutter, KDE). Only the injected-into field's *keyboard focus*
//! decides where it lands — that, and the secure-field gate, are the caller's
//! responsibility (AT-SPI detection must clear before we type).
//!
//! The character → keycode map is a pure US-QWERTY table (Linux
//! `input-event-codes`), unit-tested on every OS. The device I/O is Linux-only.
#![cfg(target_os = "linux")]
#![allow(dead_code)]

// Linux input-event-codes (subset). Kept as raw u16 so the keymap is pure and
// host-testable without evdev.
mod code {
    pub const A: u16 = 30;
    pub const LEFTSHIFT: u16 = 42;
    pub const SPACE: u16 = 57;
    pub const ENTER: u16 = 28;
    pub const TAB: u16 = 15;
    // Row/letter bases are handled by explicit tables below.
}

/// One physical keystroke: a base key, optionally with Shift held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyStroke {
    pub code: u16,
    pub shift: bool,
}

const fn k(code: u16, shift: bool) -> KeyStroke {
    KeyStroke { code, shift }
}

/// Map a character to a US-QWERTY keystroke, or `None` if it is not directly
/// typable on that layout (callers should skip / log — full-Unicode entry is a
/// follow-up, e.g. compositor compose or a keymap swap).
pub fn char_to_key(ch: char) -> Option<KeyStroke> {
    // Letters: KEY_* codes for a..z, uppercase => same code + shift.
    if ch.is_ascii_alphabetic() {
        let lower = ch.to_ascii_lowercase();
        let code = LETTER_CODES[(lower as u8 - b'a') as usize];
        return Some(k(code, ch.is_ascii_uppercase()));
    }
    // Digits and their shifted symbols share a code.
    if let Some(stroke) = digit_or_symbol(ch) {
        return Some(stroke);
    }
    match ch {
        ' ' => Some(k(code::SPACE, false)),
        '\n' => Some(k(code::ENTER, false)),
        '\t' => Some(k(code::TAB, false)),
        '-' => Some(k(12, false)),
        '_' => Some(k(12, true)),
        '=' => Some(k(13, false)),
        '+' => Some(k(13, true)),
        '[' => Some(k(26, false)),
        '{' => Some(k(26, true)),
        ']' => Some(k(27, false)),
        '}' => Some(k(27, true)),
        ';' => Some(k(39, false)),
        ':' => Some(k(39, true)),
        '\'' => Some(k(40, false)),
        '"' => Some(k(40, true)),
        '`' => Some(k(41, false)),
        '~' => Some(k(41, true)),
        '\\' => Some(k(43, false)),
        '|' => Some(k(43, true)),
        ',' => Some(k(51, false)),
        '<' => Some(k(51, true)),
        '.' => Some(k(52, false)),
        '>' => Some(k(52, true)),
        '/' => Some(k(53, false)),
        '?' => Some(k(53, true)),
        _ => None,
    }
}

// a..z → KEY_A(30) etc. Index 0 = 'a'.
const LETTER_CODES: [u16; 26] = [
    30, 48, 46, 32, 18, 33, 34, 35, 23, 36, 37, 38, 50, 49, 24, 25, 16, 19, 31, 20, 22, 47, 17, 45,
    21, 44,
];

fn digit_or_symbol(ch: char) -> Option<KeyStroke> {
    // KEY_1..KEY_9 = 2..10, KEY_0 = 11.
    let table = [
        ('1', 2u16, '!'),
        ('2', 3, '@'),
        ('3', 4, '#'),
        ('4', 5, '$'),
        ('5', 6, '%'),
        ('6', 7, '^'),
        ('7', 8, '&'),
        ('8', 9, '*'),
        ('9', 10, '('),
        ('0', 11, ')'),
    ];
    for (digit, code, shifted) in table {
        if ch == digit {
            return Some(k(code, false));
        }
        if ch == shifted {
            return Some(k(code, true));
        }
    }
    None
}

/// How many characters of `s` this keymap can type on US-QWERTY.
pub fn typable_count(s: &str) -> usize {
    s.chars().filter(|&c| char_to_key(c).is_some()).count()
}

// The device writer (Linux-only I/O). The pure keymap above is what CI tests
// on every OS; this module talks to /dev/uinput.
mod device {
    use super::{char_to_key, code};
    use evdev::{uinput::VirtualDevice, AttributeSet, InputEvent, KeyCode};
    use std::io;

    /// EV_KEY event type (Linux `input-event-codes`).
    const EV_KEY: u16 = 1;

    /// A virtual keyboard under /dev/uinput. Requires write access to
    /// `/dev/uinput` (udev rule / `input` group) — an explicit opt-in per
    /// ADR-0013; never enabled silently.
    pub struct UinputKeyboard {
        dev: VirtualDevice,
    }

    impl UinputKeyboard {
        /// Create the virtual keyboard, advertising every key the map can emit.
        pub fn open() -> io::Result<Self> {
            let mut keys = AttributeSet::<KeyCode>::new();
            keys.insert(KeyCode::new(code::LEFTSHIFT));
            // Advertise the full set of codes the keymap can produce.
            for cp in 0x20u8..0x7f {
                if let Some(stroke) = char_to_key(cp as char) {
                    keys.insert(KeyCode::new(stroke.code));
                }
            }
            keys.insert(KeyCode::new(code::SPACE));
            keys.insert(KeyCode::new(code::ENTER));
            keys.insert(KeyCode::new(code::TAB));

            let dev = VirtualDevice::builder()?
                .name("Kaydence Virtual Keyboard")
                .with_keys(&keys)?
                .build()?;
            Ok(Self { dev })
        }

        /// Type `text`; returns the number of characters actually emitted
        /// (untypable characters on this layout are skipped).
        pub fn type_text(&mut self, text: &str) -> io::Result<usize> {
            let mut typed = 0usize;
            for ch in text.chars() {
                let Some(stroke) = char_to_key(ch) else {
                    continue;
                };
                self.tap(stroke.code, stroke.shift)?;
                typed += 1;
            }
            Ok(typed)
        }

        fn tap(&mut self, key: u16, shift: bool) -> io::Result<()> {
            let mut ev = Vec::with_capacity(4);
            if shift {
                ev.push(InputEvent::new(EV_KEY, code::LEFTSHIFT, 1));
            }
            ev.push(InputEvent::new(EV_KEY, key, 1));
            ev.push(InputEvent::new(EV_KEY, key, 0));
            if shift {
                ev.push(InputEvent::new(EV_KEY, code::LEFTSHIFT, 0));
            }
            self.dev.emit(&ev)
        }

        /// The /dev/input/eventN node(s) backing this virtual device — used to
        /// verify emitted events in tests.
        pub fn dev_nodes(&mut self) -> io::Result<Vec<std::path::PathBuf>> {
            Ok(self
                .dev
                .enumerate_dev_nodes_blocking()?
                .filter_map(Result::ok)
                .collect())
        }
    }
}

pub use device::UinputKeyboard;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercase_letters_map_without_shift() {
        assert_eq!(char_to_key('a'), Some(k(30, false)));
        assert_eq!(char_to_key('z'), Some(k(44, false)));
    }

    #[test]
    fn uppercase_letters_use_shift_same_code() {
        assert_eq!(char_to_key('A'), Some(k(30, true)));
        assert_eq!(char_to_key('K'), Some(k(37, true)));
    }

    #[test]
    fn digits_and_shifted_symbols_share_a_code() {
        assert_eq!(char_to_key('1'), Some(k(2, false)));
        assert_eq!(char_to_key('!'), Some(k(2, true)));
        assert_eq!(char_to_key('9'), Some(k(10, false)));
        assert_eq!(char_to_key('('), Some(k(10, true)));
    }

    #[test]
    fn whitespace_and_punctuation() {
        assert_eq!(char_to_key(' '), Some(k(57, false)));
        assert_eq!(char_to_key('\n'), Some(k(28, false)));
        assert_eq!(char_to_key('.'), Some(k(52, false)));
        assert_eq!(char_to_key(':'), Some(k(39, true)));
    }

    #[test]
    fn untypable_unicode_returns_none() {
        assert_eq!(char_to_key('\u{2713}'), None); // ✓ — needs Unicode entry
        assert_eq!(typable_count("Kaydence \u{2713} OK"), "Kaydence  OK".len());
    }
}
