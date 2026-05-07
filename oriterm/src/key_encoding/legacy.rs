//! Legacy xterm key encoding and `APP_KEYPAD` numpad sequences.
//!
//! Produces VT/xterm escape sequences for named keys (arrows, function keys),
//! C0 control codes for Ctrl+letter, and ESC-prefixed bytes for Alt+key. This
//! is the baseline encoding used when the Kitty keyboard protocol is not active.

use winit::keyboard::{Key, NamedKey};

use super::Modifiers;
use super::cursor_keys::{CursorKey, cursor_key_bytes};
use oriterm_core::TermMode;

/// Look up a DECCKM-controlled cursor-style key.
///
/// Returns `Some(CursorKey)` for Up/Down/Right/Left/Home/End — the keys whose
/// SS3-vs-CSI prefix flips with the `APP_CURSOR` mode flag. F1-F4 are NOT in
/// this set; see [`function_key_terminator`].
#[must_use]
pub(super) fn cursor_key_for_named(key: NamedKey) -> Option<CursorKey> {
    Some(match key {
        NamedKey::ArrowUp => CursorKey::Up,
        NamedKey::ArrowDown => CursorKey::Down,
        NamedKey::ArrowRight => CursorKey::Right,
        NamedKey::ArrowLeft => CursorKey::Left,
        NamedKey::Home => CursorKey::Home,
        NamedKey::End => CursorKey::End,
        _ => return None,
    })
}

/// Look up a function-key terminator byte (F1-F4).
///
/// F1-F4 always use SS3 when unmodified (`ESC O P/Q/R/S`) and CSI with
/// modifiers — they do NOT depend on DECCKM, so they are kept separate from
/// [`CursorKey`] / [`cursor_key_bytes`].
#[must_use]
pub(super) fn function_key_terminator(key: NamedKey) -> Option<u8> {
    Some(match key {
        NamedKey::F1 => b'P',
        NamedKey::F2 => b'Q',
        NamedKey::F3 => b'R',
        NamedKey::F4 => b'S',
        _ => return None,
    })
}

/// Named key with a tilde terminator (`ESC [ {num} ~`).
///
/// With modifiers: `ESC [ {num} ; {mod} ~`.
pub(super) struct TildeKey {
    /// Numeric parameter before the tilde.
    pub(super) num: u8,
}

/// Look up a tilde-terminated named key.
///
/// Returns the numeric parameter for Insert, Delete, PageUp/Down, and F5-F12.
/// `pub(super)` because `kitty.rs` consumes the same SSOT for its CSI-u
/// `legacy_csi_info` lookup.
#[must_use]
pub(super) fn tilde_key(key: NamedKey) -> Option<TildeKey> {
    Some(match key {
        NamedKey::Insert => TildeKey { num: 2 },
        NamedKey::Delete => TildeKey { num: 3 },
        NamedKey::PageUp => TildeKey { num: 5 },
        NamedKey::PageDown => TildeKey { num: 6 },
        NamedKey::F5 => TildeKey { num: 15 },
        NamedKey::F6 => TildeKey { num: 17 },
        NamedKey::F7 => TildeKey { num: 18 },
        NamedKey::F8 => TildeKey { num: 19 },
        NamedKey::F9 => TildeKey { num: 20 },
        NamedKey::F10 => TildeKey { num: 21 },
        NamedKey::F11 => TildeKey { num: 23 },
        NamedKey::F12 => TildeKey { num: 24 },
        _ => return None,
    })
}

/// Encode a key event using legacy xterm/VT sequences.
///
/// Handles named keys (arrows, function keys, Home/End, etc.), Ctrl+letter
/// C0 control codes, Alt+key ESC prefix, and plain text fallback.
#[must_use]
pub(super) fn encode_legacy(
    key: &Key,
    mods: Modifiers,
    mode: TermMode,
    text: Option<&str>,
) -> Vec<u8> {
    let app_cursor = mode.contains(TermMode::APP_CURSOR);
    let mod_param = mods.xterm_param();

    // Named keys.
    if let Key::Named(named) = key {
        // DECCKM-controlled cursor keys (arrows, Home, End).
        if let Some(cursor) = cursor_key_for_named(*named) {
            return if mod_param > 0 {
                // Modifiers always use CSI format: ESC [ 1 ; {mod} {term}.
                format!("\x1b[1;{}{}", mod_param, cursor.terminator() as char).into_bytes()
            } else {
                // SSOT byte selection (SS3 in DECCKM, CSI in normal mode).
                cursor_key_bytes(cursor, app_cursor).to_vec()
            };
        }

        // F1-F4: always SS3 when unmodified, CSI with modifiers.
        if let Some(term) = function_key_terminator(*named) {
            return if mod_param > 0 {
                format!("\x1b[1;{}{}", mod_param, term as char).into_bytes()
            } else {
                vec![0x1b, b'O', term]
            };
        }

        // Tilde-terminated keys (Insert, Delete, PgUp, PgDn, F5-F12).
        if let Some(tk) = tilde_key(*named) {
            return if mod_param > 0 {
                format!("\x1b[{};{}~", tk.num, mod_param).into_bytes()
            } else {
                format!("\x1b[{}~", tk.num).into_bytes()
            };
        }

        // Simple named keys with fixed byte output.
        return encode_simple_named(*named, mods, mode);
    }

    // Character keys.
    if let Key::Character(ch) = key {
        return encode_character(ch.as_str(), mods, text);
    }

    // Fallback: send the text as-is (handles Unidentified keys from RDP/IME).
    text.map_or_else(Vec::new, |t| t.as_bytes().to_vec())
}

/// Encode simple named keys (Enter, Backspace, Tab, Escape, Space).
fn encode_simple_named(named: NamedKey, mods: Modifiers, mode: TermMode) -> Vec<u8> {
    match named {
        NamedKey::Enter => {
            let base = oriterm_core::encode_enter_base(mode);
            if mods.contains(Modifiers::ALT) {
                let mut v = Vec::with_capacity(1 + base.len());
                v.push(0x1b);
                v.extend_from_slice(base);
                v
            } else {
                base.to_vec()
            }
        }
        NamedKey::Backspace => {
            // DECBKM (mode 67) inverts backspace polarity. Ctrl XORs it.
            // Default (DECBKM reset): plain→DEL(0x7f), Ctrl→BS(0x08).
            // DECBKM set: plain→BS(0x08), Ctrl→DEL(0x7f).
            let ctrl = mods.contains(Modifiers::CONTROL);
            let decbkm = mode.contains(TermMode::DECBKM);
            let byte = if ctrl ^ decbkm { 0x08 } else { 0x7f };
            if mods.contains(Modifiers::ALT) {
                vec![0x1b, byte]
            } else {
                vec![byte]
            }
        }
        NamedKey::Tab => {
            if mods.contains(Modifiers::SHIFT) {
                b"\x1b[Z".to_vec()
            } else {
                vec![b'\t']
            }
        }
        NamedKey::Escape => vec![0x1b],
        NamedKey::Space => encode_space(mods),
        _ => Vec::new(),
    }
}

/// Encode Space with modifier combinations.
fn encode_space(mods: Modifiers) -> Vec<u8> {
    if mods.contains(Modifiers::CONTROL) {
        // Ctrl+Space = NUL, optionally with Alt prefix.
        let mut v = Vec::with_capacity(2);
        if mods.contains(Modifiers::ALT) {
            v.push(0x1b);
        }
        v.push(0x00);
        v
    } else if mods.contains(Modifiers::ALT) {
        vec![0x1b, b' ']
    } else {
        vec![b' ']
    }
}

/// Encode a character key (Ctrl+letter → C0, Alt → ESC prefix, or plain text).
fn encode_character(s: &str, mods: Modifiers, text: Option<&str>) -> Vec<u8> {
    // Ctrl+letter → C0 control code.
    if mods.contains(Modifiers::CONTROL) {
        if let Some(c0) = ctrl_key_byte(s) {
            let mut v = Vec::with_capacity(2);
            if mods.contains(Modifiers::ALT) {
                v.push(0x1b);
            }
            v.push(c0);
            return v;
        }
    }

    // Textual byte source: prefer winit's `text` (locale-aware; may reflect IME
    // composition), fall back to the logical `Key::Character` content. Covers
    // backends that don't populate `text` for numpad keys ().
    let bytes: &[u8] = text.map_or_else(|| s.as_bytes(), str::as_bytes);

    // Alt prefix for character keys (without Ctrl).
    if mods.contains(Modifiers::ALT) && !mods.contains(Modifiers::CONTROL) {
        let mut v = Vec::with_capacity(1 + bytes.len());
        v.push(0x1b);
        v.extend_from_slice(bytes);
        return v;
    }

    bytes.to_vec()
}

/// Map a Ctrl+key combination to its C0 control byte.
///
/// Handles a-z (case-insensitive), bracket/punctuation keys (`[`, `\\`, `]`,
/// `^`, `_`, `` ` ``), and the digit shortcuts 2-8 (xterm-compatible).
fn ctrl_key_byte(s: &str) -> Option<u8> {
    let bytes = s.as_bytes();
    if bytes.len() != 1 {
        return None;
    }
    match bytes[0] {
        // a-z → 0x01-0x1A.
        b'a'..=b'z' => Some(bytes[0] - b'a' + 1),
        b'A'..=b'Z' => Some(bytes[0] - b'A' + 1),
        // Punctuation → control codes.
        b'[' | b'3' => Some(0x1b),  // Ctrl+[ = ESC
        b'\\' | b'4' => Some(0x1c), // Ctrl+\ = FS
        b']' | b'5' => Some(0x1d),  // Ctrl+] = GS
        b'^' | b'6' => Some(0x1e),  // Ctrl+^ = RS
        b'_' | b'7' => Some(0x1f),  // Ctrl+_ = US
        b'`' | b'2' => Some(0x00),  // Ctrl+` = NUL
        b'8' => Some(0x7f),         // Ctrl+8 = DEL
        _ => None,
    }
}

/// Encode numpad keys in `APP_KEYPAD` mode (SS3 sequences).
///
/// Produces `ESC O {code}` for numpad digits 0-9, operators, decimal,
/// and Enter. Returns `None` for non-numpad keys.
pub(super) fn encode_numpad_app(key: &Key) -> Option<Vec<u8>> {
    let code = match key {
        Key::Character(c) => match c.as_str() {
            "0" => b'p',
            "1" => b'q',
            "2" => b'r',
            "3" => b's',
            "4" => b't',
            "5" => b'u',
            "6" => b'v',
            "7" => b'w',
            "8" => b'x',
            "9" => b'y',
            "+" => b'k',
            "-" => b'm',
            "*" => b'j',
            "/" => b'o',
            "." => b'n',
            _ => return None,
        },
        Key::Named(NamedKey::Enter) => b'M',
        _ => return None,
    };
    Some(vec![0x1b, b'O', code])
}
