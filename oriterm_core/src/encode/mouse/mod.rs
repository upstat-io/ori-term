//! Mouse event encoding: SGR, UTF-8, URXVT, and Normal (X10) formats.
//!
//! Pure functions that encode mouse events as escape sequences. Zero-allocation:
//! all output is written into a stack-allocated [`MouseReportBuf`].
//!
//! Per Decision 10 Option A (`plans/spec-conformance/decisions/10-mouse-verification-apex-effect-vs-app-capture.md`)
//! and §16.2.0, the encoder body lives in `oriterm_core` so that emission
//! flows through `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::MouseEvent, bytes })`
//! via [`Term::handle_mouse_input`](crate::term::Term::handle_mouse_input).
//! The App layer constructs semantic [`MouseEvent`] values from winit input
//! and dispatches them; the encoder reads `TermMode` and emits through the
//! existing Effect sink — the same apex pattern §16.1.C established for DECLRP.

use std::io::{Cursor, Write};

use crate::TermMode;
use crate::input::Modifiers;

/// Mouse button for reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
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
pub enum MouseEventKind {
    /// Button pressed.
    Press,
    /// Button released.
    Release,
    /// Cursor moved while button held (or any motion in mode 1003).
    Motion,
}

/// Boundary type for mouse-event callers (bool-field shape).
///
/// Converts to the canonical [`Modifiers`] via
/// `From<MouseModifiers> for Modifiers`. App-side construction sites
/// (and test fixtures) write the bool-field form; the encoder runs
/// on `Modifiers` internally per §16.3 algorithmic SSOT.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MouseModifiers {
    /// Shift key held.
    pub shift: bool,
    /// Alt/Meta key held.
    pub alt: bool,
    /// Ctrl key held.
    pub ctrl: bool,
}

impl From<MouseModifiers> for Modifiers {
    fn from(m: MouseModifiers) -> Self {
        Self::from_shift_alt_ctrl(m.shift, m.alt, m.ctrl)
    }
}

impl From<Modifiers> for MouseModifiers {
    fn from(m: Modifiers) -> Self {
        Self {
            shift: m.contains(Modifiers::SHIFT),
            alt: m.contains(Modifiers::ALT),
            ctrl: m.contains(Modifiers::CONTROL),
        }
    }
}

/// Stack-allocated buffer for encoded mouse report (max 32 bytes).
///
/// Avoids heap allocation in the hot path. All encoding functions
/// write into this buffer via `std::io::Cursor`.
pub struct MouseReportBuf {
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
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len]
    }
}

/// Compute the base button code for a mouse report.
///
/// Left=0, Middle=1, Right=2, ScrollUp=64, ScrollDown=65.
/// Motion adds 32 to the base code.
pub fn button_code(button: MouseButton, kind: MouseEventKind) -> u8 {
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

/// Compute the mouse-Cb additive modifier bits per xterm spec.
///
/// Shift=+4, Alt=+8, Ctrl=+16. Structurally distinct from xterm
/// keyboard CSI Pm `1 + bits` returned by `Modifiers::xterm_param()`.
/// §17 keyboard encoder uses `xterm_param`; mouse uses this helper.
/// Super is intentionally ignored — xterm mouse Cb has no Super bit.
pub fn mouse_cb_modifier_bits(mods: Modifiers) -> u8 {
    let mut result = 0u8;
    if mods.contains(Modifiers::SHIFT) {
        result += 4;
    }
    if mods.contains(Modifiers::ALT) {
        result += 8;
    }
    if mods.contains(Modifiers::CONTROL) {
        result += 16;
    }
    result
}

/// Apply modifier bits to a button code (boundary helper).
///
/// Takes `MouseModifiers` (bool-field boundary type) + converts to
/// `Modifiers` + dispatches to `mouse_cb_modifier_bits` per §16.3
/// algorithmic SSOT.
pub fn apply_modifiers(code: u8, mods: MouseModifiers) -> u8 {
    code + mouse_cb_modifier_bits(mods.into())
}

/// Encode a mouse event in SGR format.
///
/// Format: `\x1b[<code;col+1;line+1{M|m}`
/// Uses `M` for press/motion, `m` for release. Coordinates are 1-indexed.
/// Returns the number of bytes written.
pub fn encode_sgr(buf: &mut [u8], code: u8, col: usize, line: usize, pressed: bool) -> usize {
    let suffix = if pressed { 'M' } else { 'm' };
    let mut cursor = Cursor::new(buf);
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
/// Returns 0 if coordinates are out of range (> 2014; UTF-8 byte limit val > 0x7FF).
pub fn encode_utf8(buf: &mut [u8], code: u8, col: usize, line: usize) -> usize {
    let mut cursor = Cursor::new(buf);
    let Ok(()) = cursor.write_all(b"\x1b[M") else {
        return 0;
    };

    let btn = 32u32 + u32::from(code);
    if btn > 127 {
        return 0;
    }
    let Ok(()) = cursor.write_all(&[btn as u8]) else {
        return 0;
    };

    for pos in [col, line] {
        if !write_utf8_coord(&mut cursor, pos) {
            return 0;
        }
    }

    cursor.position() as usize
}

/// Encode a mouse event in SGR-Pixel format (DEC private mode 1016).
///
/// Format: `\x1b[<{code};{px};{py}{M|m}` — same wire shape as SGR
/// but coordinates are logical pixels (1-indexed per xterm spec)
/// rather than cell coords. Returns the number of bytes written.
///
/// Per xterm `charproc.c` `kitty mouse.c` reference: SGR-Pixel adds
/// `1` to the pixel coordinate (Px+1, Py+1) just like SGR adds 1 to
/// cell coords. Pixel coordinates flow from the App layer with the
/// `Window::scale_factor()` already applied (logical, not physical).
pub fn encode_sgr_pixel(buf: &mut [u8], code: u8, px: u32, py: u32, pressed: bool) -> usize {
    let suffix = if pressed { 'M' } else { 'm' };
    let mut cursor = Cursor::new(buf);
    let Ok(()) = write!(cursor, "\x1b[<{code};{};{}{suffix}", px + 1, py + 1) else {
        return 0;
    };
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
pub fn encode_normal(buf: &mut [u8], code: u8, col: usize, line: usize) -> usize {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
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
    /// Logical-pixel x coordinate for SGR-Pixel (mode 1016) encoding.
    /// `None` for non-pixel encoders (the cell encoders ignore the
    /// field). Pre-computed by the App caller from pinned cell metrics
    /// (`FontCollection::cell_metrics()` per §05 Golden Lane SSOT) +
    /// sub-cell offset. Per xterm `charproc.c` SGR-Pixel reports use
    /// logical (CSS) pixels, NOT physical pixels — App applies
    /// `Window::scale_factor()` division before populating.
    pub px: Option<u32>,
    /// Logical-pixel y coordinate for SGR-Pixel encoding. See `px`.
    pub py: Option<u32>,
}

/// Mouse-dispatch gate: true iff `mode` has any ANY_MOUSE-family flag.
///
/// SSOT for the ANY_MOUSE-family predicate so `Term::handle_mouse_input`
/// and `MuxBackend::send_mouse_input`'s default impl share one definition
/// rather than each re-implementing the intersection check. Keeping the
/// predicate here prevents the gate from drifting between callers.
#[must_use]
#[inline]
pub fn should_handle_mouse_input(mode: TermMode) -> bool {
    mode.intersects(TermMode::ANY_MOUSE)
}

/// Encode a mouse event, selecting the format based on terminal mode.
///
/// Priority: SGR > URXVT > UTF-8 > Normal. Returns the encoded bytes in
/// the buffer. For X10 mode (mode 9), modifiers are stripped and only
/// presses are encoded (releases return an empty buffer).
pub fn encode_mouse_event(event: &MouseEvent, mode: TermMode) -> MouseReportBuf {
    let mut buf = MouseReportBuf::new();
    let x10 = mode.contains(TermMode::MOUSE_X10);

    let code = if x10 {
        button_code(event.button, event.kind)
    } else {
        apply_modifiers(button_code(event.button, event.kind), event.mods)
    };
    let pressed = event.kind != MouseEventKind::Release;

    if x10 && !pressed {
        return buf;
    }

    // SGR-Pixel (DEC mode 1016) shares the SGR (1006) wire envelope
    // `\x1b[<{code};{c1};{c2}{M|m}` but the numeric fields ARE pixel
    // coordinates per xterm spec — 1016-aware clients (notcurses
    // `pixelmouse_click`, kitty `SGR_PIXEL_PROTOCOL`) UNCONDITIONALLY
    // divide the parsed values by cell_pixel dimensions when 1016 is
    // active. Substituting cell coordinates inside the 1016 branch
    // would silently corrupt the client-side cell mapping (cite:
    // notcurses/src/lib/in.c:556-586 pixelmouse_click). When 1016 is
    // active and the App caller did not supply pixel coords, the
    // encoder emits NO bytes — the App-side clamping at
    // App::mouse_pixel_coords guarantees Some pixel coords for every
    // in-grid event so this branch is unreachable in production for
    // legitimate clicks. The encoder is a total function: if-let-else
    // produces 0 for the None case, no panic path. The X10 fallthrough
    // in the final `else` arm is reachable ONLY when no extended-
    // encoding flag is set (legitimate legacy clients).
    buf.len = if mode.contains(TermMode::MOUSE_SGR_PIXEL) {
        if let (Some(px), Some(py)) = (event.px, event.py) {
            encode_sgr_pixel(&mut buf.data, code, px, py, pressed)
        } else {
            0
        }
    } else if mode.contains(TermMode::MOUSE_SGR) {
        encode_sgr(&mut buf.data, code, event.col, event.line, pressed)
    } else if mode.contains(TermMode::MOUSE_URXVT) {
        encode_urxvt(&mut buf.data, code, event.col, event.line)
    } else if mode.contains(TermMode::MOUSE_UTF8) {
        encode_utf8(&mut buf.data, code, event.col, event.line)
    } else {
        let code = if event.kind == MouseEventKind::Release {
            apply_modifiers(3, event.mods)
        } else {
            code
        };
        encode_normal(&mut buf.data, code, event.col, event.line)
    };

    buf
}

#[cfg(test)]
mod tests;
