//! Win32 keyboard-input encoding for `ConPTY` (`DECSET ? 9001`).
//!
//! `ConPTY` asks the hosting terminal to switch into "win32 input mode" so
//! key presses can be sent as `KEY_EVENT_RECORD` data instead of plain VT text.
//! This lets conhost handle special keys like Ctrl+C before the input stream is
//! serialized behind flood output.

use winit::keyboard::{Key, KeyLocation, NamedKey};

use super::{KeyEventType, KeyInput, Modifiers};

const CSI: &str = "\x1b[";

const LEFT_ALT_PRESSED: u16 = 0x0002;
const RIGHT_ALT_PRESSED: u16 = 0x0001;
const LEFT_CTRL_PRESSED: u16 = 0x0008;
const RIGHT_CTRL_PRESSED: u16 = 0x0004;
const SHIFT_PRESSED: u16 = 0x0010;

pub(super) fn encode_win32(input: &KeyInput<'_>) -> Vec<u8> {
    let Some((vk, sc, uc)) = key_record_fields(input) else {
        return Vec::new();
    };

    let key_down = match input.event_type {
        KeyEventType::Release => 0,
        KeyEventType::Press | KeyEventType::Repeat => 1,
    };
    let control_state = control_key_state(input.mods, input.location);

    format!("{CSI}{vk};{sc};{uc};{key_down};{control_state};1_",).into_bytes()
}

fn key_record_fields(input: &KeyInput<'_>) -> Option<(u16, u16, u16)> {
    let unicode_char = unicode_char(input);

    match input.key {
        Key::Named(named) => named_key_fields(*named, input.mods).map(|(vk, sc, uc)| {
            let uc = if unicode_char != 0 { unicode_char } else { uc };
            (vk, sc, uc)
        }),
        Key::Character(s) => character_key_fields(s.as_str(), unicode_char),
        _ => {
            if unicode_char != 0 {
                Some((0, 0, unicode_char))
            } else {
                None
            }
        }
    }
}

fn unicode_char(input: &KeyInput<'_>) -> u16 {
    if input.event_type == KeyEventType::Release {
        return 0;
    }

    if matches!(input.key, Key::Named(NamedKey::Space)) && input.mods.contains(Modifiers::CONTROL) {
        return 0;
    }

    if let Key::Character(s) = input.key {
        if input.mods.contains(Modifiers::CONTROL) {
            if let Some(c0) = ctrl_key_byte(s.as_str()) {
                return c0 as u16;
            }
        }
    }

    input
        .text
        .and_then(first_char)
        .or_else(|| match input.key {
            Key::Character(s) => first_char(s.as_str()),
            Key::Named(NamedKey::Enter) => Some('\r'),
            Key::Named(NamedKey::Tab) => Some('\t'),
            Key::Named(NamedKey::Backspace) => Some('\x08'),
            Key::Named(NamedKey::Escape) => Some('\x1b'),
            Key::Named(NamedKey::Space) => Some(' '),
            _ => None,
        })
        .map_or(0, |c| c as u16)
}

fn named_key_fields(named: NamedKey, mods: Modifiers) -> Option<(u16, u16, u16)> {
    Some(match named {
        NamedKey::Enter => (0x0D, 28, 0x0D),
        NamedKey::Backspace => (0x08, 14, 0x08),
        NamedKey::Tab => (0x09, 15, 0x09),
        NamedKey::Escape => (0x1B, 1, 0x1B),
        NamedKey::Space => (
            0x20,
            57,
            if mods.contains(Modifiers::CONTROL) {
                0
            } else {
                0x20
            },
        ),
        NamedKey::ArrowLeft => (0x25, 75, 0),
        NamedKey::ArrowUp => (0x26, 72, 0),
        NamedKey::ArrowRight => (0x27, 77, 0),
        NamedKey::ArrowDown => (0x28, 80, 0),
        NamedKey::Home => (0x24, 71, 0),
        NamedKey::End => (0x23, 79, 0),
        NamedKey::PageUp => (0x21, 73, 0),
        NamedKey::PageDown => (0x22, 81, 0),
        NamedKey::Insert => (0x2D, 82, 0),
        NamedKey::Delete => (0x2E, 83, 0),
        NamedKey::F1 => (0x70, 59, 0),
        NamedKey::F2 => (0x71, 60, 0),
        NamedKey::F3 => (0x72, 61, 0),
        NamedKey::F4 => (0x73, 62, 0),
        NamedKey::F5 => (0x74, 63, 0),
        NamedKey::F6 => (0x75, 64, 0),
        NamedKey::F7 => (0x76, 65, 0),
        NamedKey::F8 => (0x77, 66, 0),
        NamedKey::F9 => (0x78, 67, 0),
        NamedKey::F10 => (0x79, 68, 0),
        NamedKey::F11 => (0x7A, 87, 0),
        NamedKey::F12 => (0x7B, 88, 0),
        _ => return None,
    })
}

fn character_key_fields(s: &str, unicode_char: u16) -> Option<(u16, u16, u16)> {
    let ch = first_char(s)?;
    let (vk, sc) = match ch {
        'a' | 'A' => (b'A' as u16, 30),
        'b' | 'B' => (b'B' as u16, 48),
        'c' | 'C' => (b'C' as u16, 46),
        'd' | 'D' => (b'D' as u16, 32),
        'e' | 'E' => (b'E' as u16, 18),
        'f' | 'F' => (b'F' as u16, 33),
        'g' | 'G' => (b'G' as u16, 34),
        'h' | 'H' => (b'H' as u16, 35),
        'i' | 'I' => (b'I' as u16, 23),
        'j' | 'J' => (b'J' as u16, 36),
        'k' | 'K' => (b'K' as u16, 37),
        'l' | 'L' => (b'L' as u16, 38),
        'm' | 'M' => (b'M' as u16, 50),
        'n' | 'N' => (b'N' as u16, 49),
        'o' | 'O' => (b'O' as u16, 24),
        'p' | 'P' => (b'P' as u16, 25),
        'q' | 'Q' => (b'Q' as u16, 16),
        'r' | 'R' => (b'R' as u16, 19),
        's' | 'S' => (b'S' as u16, 31),
        't' | 'T' => (b'T' as u16, 20),
        'u' | 'U' => (b'U' as u16, 22),
        'v' | 'V' => (b'V' as u16, 47),
        'w' | 'W' => (b'W' as u16, 17),
        'x' | 'X' => (b'X' as u16, 45),
        'y' | 'Y' => (b'Y' as u16, 21),
        'z' | 'Z' => (b'Z' as u16, 44),
        '0' => (b'0' as u16, 11),
        '1' => (b'1' as u16, 2),
        '2' => (b'2' as u16, 3),
        '3' => (b'3' as u16, 4),
        '4' => (b'4' as u16, 5),
        '5' => (b'5' as u16, 6),
        '6' => (b'6' as u16, 7),
        '7' => (b'7' as u16, 8),
        '8' => (b'8' as u16, 9),
        '9' => (b'9' as u16, 10),
        '-' => (0xBD, 12),
        '=' => (0xBB, 13),
        '[' => (0xDB, 26),
        ']' => (0xDD, 27),
        '\\' => (0xDC, 43),
        ';' => (0xBA, 39),
        '\'' => (0xDE, 40),
        '`' => (0xC0, 41),
        ',' => (0xBC, 51),
        '.' => (0xBE, 52),
        '/' => (0xBF, 53),
        _ => return Some((0, 0, unicode_char)),
    };
    Some((vk, sc, unicode_char))
}

fn control_key_state(mods: Modifiers, location: KeyLocation) -> u16 {
    let mut state = 0;
    if mods.contains(Modifiers::SHIFT) {
        state |= SHIFT_PRESSED;
    }
    if mods.contains(Modifiers::ALT) {
        state |= match location {
            KeyLocation::Right => RIGHT_ALT_PRESSED,
            _ => LEFT_ALT_PRESSED,
        };
    }
    if mods.contains(Modifiers::CONTROL) {
        state |= match location {
            KeyLocation::Right => RIGHT_CTRL_PRESSED,
            _ => LEFT_CTRL_PRESSED,
        };
    }
    state
}

fn first_char(s: &str) -> Option<char> {
    let mut chars = s.chars();
    let ch = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    Some(ch)
}

fn ctrl_key_byte(s: &str) -> Option<u8> {
    let bytes = s.as_bytes();
    if bytes.len() != 1 {
        return None;
    }
    match bytes[0] {
        b'a'..=b'z' => Some(bytes[0] - b'a' + 1),
        b'A'..=b'Z' => Some(bytes[0] - b'A' + 1),
        b'[' | b'3' => Some(0x1b),
        b'\\' | b'4' => Some(0x1c),
        b']' | b'5' => Some(0x1d),
        b'^' | b'6' => Some(0x1e),
        b'_' | b'7' => Some(0x1f),
        b'`' | b'2' => Some(0x00),
        b'8' => Some(0x7f),
        _ => None,
    }
}
