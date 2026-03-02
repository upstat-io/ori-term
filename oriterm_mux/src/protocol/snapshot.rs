//! Wire-friendly types for pane state transfer.
//!
//! These are separate from internal types (`Cell`, `Palette`, `TermMode`) to
//! decouple the wire format from internal representation. Internal types may
//! use `Arc`, external crate types (`vte::ansi::Color`), or bitflags that
//! aren't directly serializable — wire types are flat and self-contained.

use serde::{Deserialize, Serialize};

use crate::id::{PaneId, TabId, WindowId};

/// RGB color on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireRgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

/// Terminal color on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireColor {
    /// Named color (0–15).
    Named(u8),
    /// Indexed color (0–255).
    Indexed(u8),
    /// 24-bit true color.
    Rgb(WireRgb),
}

/// Cell SGR flags as raw bits.
///
/// Maps 1:1 to `oriterm_core::CellFlags` bits. Using raw `u16` avoids
/// coupling the wire format to the bitflags type.
pub type WireCellFlags = u16;

/// A single terminal cell on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireCell {
    /// Displayed character.
    pub ch: char,
    /// Foreground color.
    pub fg: WireColor,
    /// Background color.
    pub bg: WireColor,
    /// SGR attribute flags (bold, italic, etc.).
    pub flags: WireCellFlags,
    /// Combining marks / zero-width characters.
    pub zerowidth: Vec<char>,
}

/// Cursor state on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireCursor {
    /// Column (0-indexed).
    pub col: u16,
    /// Row (0-indexed, within viewport).
    pub row: u16,
    /// Cursor shape: 0 = Block, 1 = Underline, 2 = Bar, 3 = `HollowBlock`, 4 = Hidden.
    pub shape: u8,
    /// Whether the cursor is visible.
    pub visible: bool,
}

/// Full snapshot of a pane's visible state.
///
/// Transferred when a client subscribes to a pane or explicitly requests
/// a snapshot. Contains everything needed to render the pane from scratch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneSnapshot {
    /// Visible grid contents (rows × cols).
    pub cells: Vec<Vec<WireCell>>,
    /// Cursor position and shape.
    pub cursor: WireCursor,
    /// Color palette as 270 RGB triplets.
    pub palette: Vec<[u8; 3]>,
    /// Pane title (from OSC 0/2).
    pub title: String,
    /// Terminal mode flags as raw bits (maps to `TermMode`).
    pub modes: u32,
    /// Number of scrollback rows above the viewport.
    pub scrollback_len: u32,
    /// Current scroll position (0 = bottom, `scrollback_len` = top).
    pub display_offset: u32,
}

/// Summary info for a mux window (used in `ListWindows` response).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MuxWindowInfo {
    /// Window identity.
    pub window_id: WindowId,
    /// Number of tabs in the window.
    pub tab_count: u32,
    /// Currently active tab.
    pub active_tab_id: TabId,
}

/// Summary info for a mux tab (used in `ListTabs` response).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MuxTabInfo {
    /// Tab identity.
    pub tab_id: TabId,
    /// Currently focused pane.
    pub active_pane_id: PaneId,
    /// Number of panes in the tab.
    pub pane_count: u32,
    /// Tab title (derived from the active pane's title).
    pub title: String,
}
