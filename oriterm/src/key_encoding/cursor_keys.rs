//! SSOT for xterm DECCKM-aware cursor-key byte encoding.
//!
//! Per xterm `ctlseqs.txt:2465-2473`: cursor keys, Home, and End transmit
//! `ESC O <c>` when DECCKM is set or `ESC [ <c>` when clear, where `<c>`
//! is the terminator byte (`A`/`B`/`C`/`D` for Up/Down/Right/Left,
//! `H` for Home, `F` for End).
//!
//! Both `key_encoding/legacy.rs` keyboard arrow encoding and
//! `app/mouse_report::tier2_alt_scroll_payload` alt-scroll synthesis
//! route through this table to prevent semantic drift. `key_encoding/kitty.rs`
//! also queries `cursor_key_for_named` + `function_key_terminator` (defined in
//! `legacy.rs`) for its CSI-u terminator selection — the terminator data has
//! one canonical home (this module + the two lookup helpers) regardless of
//! protocol layer.

/// DECCKM-controlled cursor-style keys.
///
/// These six keys flip between SS3 (`ESC O`) and CSI (`ESC [`) prefix
/// based on the `APP_CURSOR` mode flag. F1-F4 are NOT in this set —
/// they always use SS3 when unmodified, regardless of DECCKM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CursorKey {
    Up,
    Down,
    Right,
    Left,
    Home,
    End,
}

impl CursorKey {
    /// Terminator byte (`A`/`B`/`C`/`D`/`H`/`F`) for the modifier-CSI form
    /// `ESC [ 1 ; <mod> <term>` and the Kitty CSI-u form `ESC [ 1 <term>`.
    /// For unmodified non-Kitty cases, callers use [`cursor_key_bytes`].
    #[must_use]
    #[inline]
    pub(crate) const fn terminator(self) -> u8 {
        match self {
            Self::Up => b'A',
            Self::Down => b'B',
            Self::Right => b'C',
            Self::Left => b'D',
            Self::Home => b'H',
            Self::End => b'F',
        }
    }
}

/// SSOT: encode a DECCKM-controlled cursor key as SS3 or CSI bytes.
///
/// Returns one of 12 static byte slices (6 keys × 2 modes). Zero-alloc.
#[must_use]
#[inline]
pub(crate) const fn cursor_key_bytes(key: CursorKey, app_cursor: bool) -> &'static [u8] {
    use CursorKey::{Down, End, Home, Left, Right, Up};
    match (key, app_cursor) {
        (Up, true) => b"\x1bOA",
        (Up, false) => b"\x1b[A",
        (Down, true) => b"\x1bOB",
        (Down, false) => b"\x1b[B",
        (Right, true) => b"\x1bOC",
        (Right, false) => b"\x1b[C",
        (Left, true) => b"\x1bOD",
        (Left, false) => b"\x1b[D",
        (Home, true) => b"\x1bOH",
        (Home, false) => b"\x1b[H",
        (End, true) => b"\x1bOF",
        (End, false) => b"\x1b[F",
    }
}
