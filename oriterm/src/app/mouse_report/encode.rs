//! Mouse event encoding: SGR, UTF-8, URXVT, and Normal (X10) formats.
//!
//! Pure functions that encode mouse events as escape sequences. Zero-allocation:
//! all output is written into a stack-allocated [`MouseReportBuf`].

use std::io::{Cursor, Write};

use oriterm_core::TermMode;

/// Mouse button for reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MouseButton {
    /// Left button (code 0).
    Left,
    /// Middle button (code 1).
    Middle,
    /// Right button (code 2).
    Right,
    /// No button held (code 3, used for mode 1003 buttonless motion).
    None,
    /// Scroll wheel up (code 64).
    ScrollUp,
    /// Scroll wheel down (code 65).
    ScrollDown,
}

/// Mouse event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MouseEventKind {
    /// Button pressed.
    Press,
    /// Button released.
    Release,
    /// Cursor moved while button held (or any motion in mode 1003).
    Motion,
}

/// Modifier state for mouse reports.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MouseModifiers {
    /// Shift key held.
    pub shift: bool,
    /// Alt/Meta key held.
    pub alt: bool,
    /// Ctrl key held.
    pub ctrl: bool,
}

/// Stack-allocated buffer for encoded mouse report (max 32 bytes).
///
/// Avoids heap allocation in the hot path. All encoding functions
/// write into this buffer via `std::io::Cursor`.
pub(crate) struct MouseReportBuf {
    data: [u8; 32],
    len: usize,
}

impl MouseReportBuf {
    /// Create an empty report buffer.
    fn new() -> Self {
        Self {
            data: [0u8; 32],
            len: 0,
        }
    }

    /// The encoded bytes, or empty if encoding failed.
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len]
    }
}

/// Compute the base button code for a mouse report.
///
/// Left=0, Middle=1, Right=2, ScrollUp=64, ScrollDown=65.
/// Motion adds 32 to the base code.
pub(super) fn button_code(button: MouseButton, kind: MouseEventKind) -> u8 {
    let base = match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::None => 3,
        MouseButton::ScrollUp => 64,
        MouseButton::ScrollDown => 65,
    };
    if kind == MouseEventKind::Motion {
        base + 32
    } else {
        base
    }
}

/// Apply modifier bits to a button code.
///
/// Shift=+4, Alt=+8, Ctrl=+16.
pub(super) fn apply_modifiers(code: u8, mods: MouseModifiers) -> u8 {
    let mut result = code;
    if mods.shift {
        result += 4;
    }
    if mods.alt {
        result += 8;
    }
    if mods.ctrl {
        result += 16;
    }
    result
}

/// Encode a mouse event in SGR format.
///
/// Format: `\x1b[<code;col+1;line+1{M|m}`
/// Uses `M` for press/motion, `m` for release. Coordinates are 1-indexed.
/// Returns the number of bytes written.
pub(super) fn encode_sgr(
    buf: &mut [u8],
    code: u8,
    col: usize,
    line: usize,
    pressed: bool,
) -> usize {
    let suffix = if pressed { 'M' } else { 'm' };
    let mut cursor = Cursor::new(buf);
    // write! on Cursor<&mut [u8]> returns io::Error on overflow — treat as 0.
    let Ok(()) = write!(cursor, "\x1b[<{code};{};{}{suffix}", col + 1, line + 1) else {
        return 0;
    };
    cursor.position() as usize
}

/// Write a single coordinate in the UTF-8 mouse encoding.
///
/// Values < 128 use a single byte. Values 128–2047 use a custom 2-byte
/// encoding. Values > 2047 are out of range and return `false`.
fn write_utf8_coord(cursor: &mut Cursor<&mut [u8]>, pos: usize) -> bool {
    let val = 32 + 1 + pos as u32;
    if val < 128 {
        cursor.write_all(&[val as u8]).is_ok()
    } else if val <= 0x7FF {
        let first = (0xC0 + val / 64) as u8;
        let second = (0x80 + (val & 63)) as u8;
        cursor.write_all(&[first, second]).is_ok()
    } else {
        false
    }
}

/// Encode a mouse event in UTF-8 extended format.
///
/// Format: `\x1b[M` + button byte + col byte(s) + line byte(s).
/// Coordinates use a custom 2-byte encoding for values >= 95.
/// Returns 0 if coordinates are out of range (> 2015).
pub(super) fn encode_utf8(buf: &mut [u8], code: u8, col: usize, line: usize) -> usize {
    let mut cursor = Cursor::new(buf);
    let Ok(()) = cursor.write_all(b"\x1b[M") else {
        return 0;
    };

    // Button byte: always 32 + code (single byte).
    let btn = 32u32 + u32::from(code);
    if btn > 127 {
        return 0;
    }
    let Ok(()) = cursor.write_all(&[btn as u8]) else {
        return 0;
    };

    // Encode each coordinate.
    for pos in [col, line] {
        if !write_utf8_coord(&mut cursor, pos) {
            return 0;
        }
    }

    cursor.position() as usize
}

/// Encode a mouse event in URXVT format.
///
/// Format: `\x1b[Cb;Cx;CyM` where Cb = 32 + button code,
/// Cx/Cy are 1-indexed decimal. No press/release distinction
/// (all events use `M` suffix).
fn encode_urxvt(buf: &mut [u8], code: u8, col: usize, line: usize) -> usize {
    let cb = 32 + u32::from(code);
    let mut cursor = Cursor::new(buf);
    let Ok(()) = write!(cursor, "\x1b[{cb};{};{}M", col + 1, line + 1) else {
        return 0;
    };
    cursor.position() as usize
}

/// Encode a mouse event in Normal (X10) format.
///
/// Format: `\x1b[M` + 3 bytes (button, col, line).
/// Returns 0 (drops the event) if either coordinate exceeds 222,
/// since 32 + 1 + 222 = 255 is the max encodable `u8` value.
/// Sending a clamped coordinate would report a wrong position.
pub(super) fn encode_normal(buf: &mut [u8], code: u8, col: usize, line: usize) -> usize {
    if col > 222 || line > 222 {
        return 0;
    }

    let btn = 32 + code;
    let cx = (32 + 1 + col) as u8;
    let cy = (32 + 1 + line) as u8;

    let mut cursor = Cursor::new(buf);
    let Ok(()) = cursor.write_all(&[0x1b, b'[', b'M', btn, cx, cy]) else {
        return 0;
    };
    cursor.position() as usize
}

/// Input parameters for [`encode_mouse_event`].
pub(crate) struct MouseEvent {
    /// Which button (or scroll direction).
    pub button: MouseButton,
    /// Press, release, or motion.
    pub kind: MouseEventKind,
    /// Grid column (0-indexed).
    pub col: usize,
    /// Grid line (0-indexed).
    pub line: usize,
    /// Modifier keys held during the event.
    pub mods: MouseModifiers,
}

/// Encode a mouse event, selecting the format based on terminal mode.
///
/// Priority: SGR > URXVT > UTF-8 > Normal. Returns the encoded bytes in
/// the buffer. For X10 mode (mode 9), modifiers are stripped and only
/// presses are encoded (releases return an empty buffer).
pub(crate) fn encode_mouse_event(event: &MouseEvent, mode: TermMode) -> MouseReportBuf {
    let mut buf = MouseReportBuf::new();
    let x10 = mode.contains(TermMode::MOUSE_X10);

    // X10 mode: no modifiers in the button code.
    let code = if x10 {
        button_code(event.button, event.kind)
    } else {
        apply_modifiers(button_code(event.button, event.kind), event.mods)
    };
    let pressed = event.kind != MouseEventKind::Release;

    // X10 mode: only report button presses, not releases or motion.
    if x10 && !pressed {
        return buf;
    }

    buf.len = if mode.contains(TermMode::MOUSE_SGR) {
        encode_sgr(&mut buf.data, code, event.col, event.line, pressed)
    } else if mode.contains(TermMode::MOUSE_URXVT) {
        encode_urxvt(&mut buf.data, code, event.col, event.line)
    } else if mode.contains(TermMode::MOUSE_UTF8) {
        encode_utf8(&mut buf.data, code, event.col, event.line)
    } else {
        // Normal (X10) format: release uses code 3 (+ modifiers).
        let code = if event.kind == MouseEventKind::Release {
            apply_modifiers(3, event.mods)
        } else {
            code
        };
        encode_normal(&mut buf.data, code, event.col, event.line)
    };

    buf
}
