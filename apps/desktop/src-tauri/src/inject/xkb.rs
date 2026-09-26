//! Per-injection XKB keymaps for layout-independent, full-Unicode keystroke
//! synthesis (ADR-0023; closes the "Unicode beyond ASCII" gap ADR-0013 left on
//! the Linux lane).
//!
//! A Wayland virtual keyboard uploads its OWN keymap. Instead of pretending to be
//! a US-QWERTY board (the uinput limitation), we build a tiny keymap in which
//! every distinct character of the text gets its own keycode bound to that
//! character's keysym at level 1. Typing is then "press keycode N" per character
//! — no Shift, no dead keys, no dependence on the user's physical layout — so
//! emoji, CJK, RTL, and combining marks all type (inject invariant #5).
//!
//! **Keycode safety (live-found on Hyprland 0.56, 2026-09-25).** A compositor may
//! match its keybindings by translating a virtual keyboard's keycodes through
//! the *physical* keyboard's keymap, not the one we upload. A key sent on evdev
//! code 113 therefore fires the user's `XF86AudioMute` binding, 99 fires
//! Print → screenshot, 190 mic-mute, 116 the power menu, and 67 F9. So slots use
//! ONLY [`SAFE_EVDEV_CODES`]: keys whose stock meaning is a plain printable
//! character (digits, letters, punctuation, space). Unmodified binds on those
//! would already break normal typing, so no sane configuration has them.
//!
//! Pure and host-testable on every OS; `inject/wayland_vk.rs` does the I/O.
#![allow(dead_code)]

/// XKB keycodes are evdev codes + 8.
const XKB_KEYCODE_OFFSET: u32 = 8;

/// evdev codes whose stock XKB (evdev/pc105) meaning is an ordinary printable
/// key: `1`…`=`, `q`…`]`, `a`…`` ` ``, `\`…`/`, and space. Never media/system/
/// function/modifier/keypad keys, which compositors bind. All are ≤ 255 - 8, so
/// XWayland clients can address them.
pub const SAFE_EVDEV_CODES: [u32; 48] = [
    2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, // 1 2 3 4 5 6 7 8 9 0 - =
    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, // q w e r t y u i o p [ ]
    30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, // a s d f g h j k l ; ' `
    43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, // \ z x c v b n m , . /
    57, // space
];

/// Distinct characters one keymap can carry. Typical English dictation fits
/// one or two keymaps; each extra keymap costs a fresh virtual keyboard
/// (see `wayland_vk.rs`).
pub const MAX_KEYS_PER_KEYMAP: usize = SAFE_EVDEV_CODES.len();

const XK_TAB: u32 = 0xff09;
const XK_RETURN: u32 = 0xff0d;
/// Unicode keysyms are `0x0100_0000 + codepoint` (xkbcommon/X11 convention).
const UNICODE_KEYSYM_BASE: u32 = 0x0100_0000;

/// Map a character to the keysym that produces it. Latin-1 printables use their
/// legacy keysym (identical to the codepoint); everything else uses the Unicode
/// keysym range. Control characters other than newline/tab are not typable and
/// return `None` — callers report them instead of guessing. Pure.
pub fn char_to_keysym(ch: char) -> Option<u32> {
    let cp = ch as u32;
    match ch {
        '\n' => Some(XK_RETURN),
        '\t' => Some(XK_TAB),
        _ if ch.is_control() => None,
        _ if (0x20..=0x7e).contains(&cp) || (0xa0..=0xff).contains(&cp) => Some(cp),
        _ => Some(UNICODE_KEYSYM_BASE + cp),
    }
}

/// One keymap upload plus the evdev key codes to press, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapChunk {
    /// XKB v1 text keymap (without the trailing NUL the protocol wants).
    pub keymap: String,
    /// evdev key codes (xkb keycode − 8), one per typed character, in order.
    pub keys: Vec<u32>,
}

/// A plan to type `text`: one or more keymap chunks. Characters that cannot be
/// typed (bare control characters) are counted in `skipped`, never silently
/// dropped from the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypingPlan {
    pub chunks: Vec<KeymapChunk>,
    pub skipped: usize,
}

impl TypingPlan {
    /// Number of key presses the plan will send.
    pub fn key_count(&self) -> usize {
        self.chunks.iter().map(|c| c.keys.len()).sum()
    }
}

/// Split `text` into keymap chunks of at most `max_keys` distinct characters
/// each (clamped to `1..=MAX_KEYS_PER_KEYMAP`), preserving order. Most dictation
/// fits one chunk; long CJK passages roll over to a fresh keymap. Pure.
pub fn plan_typing(text: &str, max_keys: usize) -> TypingPlan {
    let max_keys = max_keys.clamp(1, MAX_KEYS_PER_KEYMAP);
    let mut chunks = Vec::new();
    let mut skipped = 0;
    // (keysym per slot, keys for this chunk)
    let mut slots: Vec<u32> = Vec::new();
    let mut keys: Vec<u32> = Vec::new();

    for ch in text.chars() {
        let Some(keysym) = char_to_keysym(ch) else {
            skipped += 1;
            continue;
        };
        let slot = match slots.iter().position(|k| *k == keysym) {
            Some(i) => i,
            None => {
                if slots.len() == max_keys {
                    chunks.push(KeymapChunk {
                        keymap: render_keymap(&slots),
                        keys: std::mem::take(&mut keys),
                    });
                    slots.clear();
                }
                slots.push(keysym);
                slots.len() - 1
            }
        };
        keys.push(SAFE_EVDEV_CODES[slot]);
    }
    if !keys.is_empty() {
        chunks.push(KeymapChunk {
            keymap: render_keymap(&slots),
            keys,
        });
    }
    TypingPlan { chunks, skipped }
}

/// Render an XKB v1 text keymap binding slot `i` to keycode
/// `SAFE_EVDEV_CODES[i] + 8`. The types/compat sections include the stock
/// "complete" sets so every client's xkbcommon compiles it; symbols are hex
/// keysyms (always ≥ 0x20, so xkbcommon never mistakes them for the digit-key
/// shorthand 0–9). At most [`MAX_KEYS_PER_KEYMAP`] keysyms are rendered. Pure.
pub fn render_keymap(keysyms: &[u32]) -> String {
    let keysyms = &keysyms[..keysyms.len().min(MAX_KEYS_PER_KEYMAP)];
    let max_keycode = SAFE_EVDEV_CODES[..keysyms.len().max(1)]
        .iter()
        .max()
        .copied()
        .unwrap_or(SAFE_EVDEV_CODES[0])
        + XKB_KEYCODE_OFFSET;
    let mut out = String::with_capacity(256 + keysyms.len() * 48);
    out.push_str("xkb_keymap {\n");
    out.push_str("xkb_keycodes \"(unnamed)\" {\n");
    out.push_str(&format!("minimum = {XKB_KEYCODE_OFFSET};\n"));
    out.push_str(&format!("maximum = {max_keycode};\n"));
    for (i, code) in SAFE_EVDEV_CODES.iter().take(keysyms.len()).enumerate() {
        out.push_str(&format!("<K{}> = {};\n", i + 1, code + XKB_KEYCODE_OFFSET));
    }
    out.push_str("};\n");
    out.push_str("xkb_types \"(unnamed)\" { include \"complete\" };\n");
    out.push_str("xkb_compatibility \"(unnamed)\" { include \"complete\" };\n");
    out.push_str("xkb_symbols \"(unnamed)\" {\n");
    for (i, keysym) in keysyms.iter().enumerate() {
        out.push_str(&format!("key <K{}> {{[ {keysym:#x} ]}};\n", i + 1));
    }
    out.push_str("};\n");
    out.push_str("};\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_and_latin1_use_legacy_keysyms() {
        assert_eq!(char_to_keysym('a'), Some(0x61));
        assert_eq!(char_to_keysym('A'), Some(0x41));
        assert_eq!(char_to_keysym(' '), Some(0x20));
        assert_eq!(char_to_keysym('~'), Some(0x7e));
        assert_eq!(char_to_keysym('é'), Some(0xe9));
        assert_eq!(char_to_keysym('\u{a0}'), Some(0xa0)); // no-break space
    }

    #[test]
    fn everything_else_uses_unicode_keysyms() {
        assert_eq!(char_to_keysym('€'), Some(0x0100_20ac));
        assert_eq!(char_to_keysym('中'), Some(0x0100_4e2d));
        assert_eq!(char_to_keysym('ש'), Some(0x0100_05e9)); // RTL
        assert_eq!(char_to_keysym('\u{301}'), Some(0x0100_0301)); // combining acute
        assert_eq!(char_to_keysym('😀'), Some(0x0101_f600)); // astral plane
    }

    #[test]
    fn newline_and_tab_are_real_keys_other_controls_are_not() {
        assert_eq!(char_to_keysym('\n'), Some(0xff0d));
        assert_eq!(char_to_keysym('\t'), Some(0xff09));
        assert_eq!(char_to_keysym('\r'), None);
        assert_eq!(char_to_keysym('\u{7f}'), None);
        assert_eq!(char_to_keysym('\u{1b}'), None);
    }

    #[test]
    fn repeated_characters_share_one_key() {
        let plan = plan_typing("hello", MAX_KEYS_PER_KEYMAP);
        assert_eq!(plan.chunks.len(), 1);
        // h e l l o → slots 0 1 2 2 3 → the first four safe codes
        assert_eq!(plan.chunks[0].keys, vec![2, 3, 4, 4, 5]);
        assert_eq!(plan.skipped, 0);
        assert_eq!(plan.key_count(), 5);
    }

    #[test]
    fn keymap_binds_each_slot_to_its_keysym() {
        let plan = plan_typing("Hé😀", MAX_KEYS_PER_KEYMAP);
        let km = &plan.chunks[0].keymap;
        assert!(km.contains("minimum = 8;"));
        assert!(km.contains("maximum = 12;")); // evdev 4 + 8
        assert!(km.contains("<K1> = 10;")); // evdev 2 + 8
        assert!(km.contains("<K3> = 12;"));
        assert!(km.contains("key <K1> {[ 0x48 ]};"));
        assert!(km.contains("key <K2> {[ 0xe9 ]};"));
        assert!(km.contains("key <K3> {[ 0x101f600 ]};"));
        assert!(km.contains("include \"complete\""));
    }

    #[test]
    fn keycodes_never_exceed_the_xwayland_ceiling() {
        // 400 distinct CJK characters must roll over into multiple keymaps.
        let text: String = (0x4e00u32..0x4e00 + 400)
            .filter_map(char::from_u32)
            .collect();
        let plan = plan_typing(&text, usize::MAX);
        assert_eq!(plan.chunks.len(), 400usize.div_ceil(MAX_KEYS_PER_KEYMAP));
        assert_eq!(plan.key_count(), 400);
        for chunk in &plan.chunks {
            let max_evdev = chunk.keys.iter().copied().max().unwrap();
            assert!(max_evdev + XKB_KEYCODE_OFFSET <= 255, "keycode over 255");
        }
    }

    #[test]
    fn chunking_preserves_order_across_keymaps() {
        let plan = plan_typing("abcab", 2);
        // chunk 1: a b ; chunk 2 starts fresh at 'c', so its first slot is 'c'
        assert_eq!(plan.chunks[0].keys, vec![2, 3]);
        assert!(plan.chunks[0].keymap.contains("key <K1> {[ 0x61 ]};"));
        assert!(plan.chunks[1].keymap.contains("key <K1> {[ 0x63 ]};"));
        assert_eq!(plan.key_count(), 5);
    }

    #[test]
    fn untypable_controls_are_counted_not_hidden() {
        let plan = plan_typing("a\rb\u{7}", MAX_KEYS_PER_KEYMAP);
        assert_eq!(plan.key_count(), 2);
        assert_eq!(plan.skipped, 2);
    }

    #[test]
    fn empty_text_is_an_empty_plan() {
        let plan = plan_typing("", MAX_KEYS_PER_KEYMAP);
        assert!(plan.chunks.is_empty());
        assert_eq!(plan.skipped, 0);
    }

    /// Keycodes Omarchy 4 / Hyprland binds by default that a 247-key keymap
    /// actually fired on 2026-09-25 (voxtype F9, screenshot, mute, mic-mute,
    /// power menu, calculator, media, touchpad, brightness, kbd backlight).
    const BOUND_BY_DEFAULT: [u32; 24] = [
        67, 99, 113, 114, 115, 116, 140, 161, 162, 163, 164, 165, 190, 191, 192, 200, 201, 207,
        210, 224, 225, 228, 229, 230,
    ];

    #[test]
    fn typing_never_touches_a_bindable_keycode() {
        // Every distinct BMP character we could be asked to type, 2000 at a time.
        let text: String = (0x4e00u32..0x4e00 + 2000)
            .chain(0x20..0x7f)
            .filter_map(char::from_u32)
            .collect();
        let plan = plan_typing(&text, usize::MAX);
        for chunk in &plan.chunks {
            for key in &chunk.keys {
                assert!(SAFE_EVDEV_CODES.contains(key), "unsafe keycode {key}");
                assert!(!BOUND_BY_DEFAULT.contains(key), "bindable keycode {key}");
                assert!(key + XKB_KEYCODE_OFFSET <= 255, "XWayland ceiling");
            }
        }
    }

    #[test]
    fn safe_codes_are_distinct_printable_keys() {
        let mut sorted = SAFE_EVDEV_CODES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), SAFE_EVDEV_CODES.len(), "duplicate safe code");
        // No modifiers (29 ctrl, 42/54 shift, 56/100 alt, 125/126 meta),
        // no F-keys (59..=68, 87, 88), no keypad (71..=83).
        for code in SAFE_EVDEV_CODES {
            assert!(![29, 42, 54, 56, 100, 125, 126].contains(&code));
            assert!(!(59..=68).contains(&code) && code != 87 && code != 88);
            assert!(!(71..=83).contains(&code));
        }
    }
}
