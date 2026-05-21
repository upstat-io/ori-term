//! Wire-friendly types for pane state transfer.
//!
//! These are separate from internal types (`Cell`, `Palette`, `TermMode`) to
//! decouple the wire format from internal representation. Internal types may
//! use `Arc`, external crate types (`vte::ansi::Color`), or bitflags that
//! aren't directly serializable — wire types are flat and self-contained.

use serde::{Deserialize, Serialize};
use vte::ansi::cursor_icon::CursorIcon;

use oriterm_core::Side;
use oriterm_core::grid::StableRowIndex;
use oriterm_core::selection::{Selection, SelectionMode, SelectionPoint};

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

/// Terminal color on the wire (unresolved palette reference).
/// Reserved for future incremental wire format where cells send only
/// changed fields and colors may reference palette indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code, reason = "reserved for future incremental wire format")]
pub enum WireColor {
    /// Named color (0–15).
    Named(u8),
    /// Indexed color (0–255).
    Indexed(u8),
    /// 24-bit true color.
    Rgb(WireRgb),
}

/// Cell SGR flags as raw bits.
/// Maps 1:1 to `oriterm_core::CellFlags` bits. Using raw `u32` avoids
/// coupling the wire format to the bitflags type.
pub type WireCellFlags = u32;

/// A single terminal cell on the wire.
/// Colors are pre-resolved RGB values — bold-as-bright, dim, and inverse
/// have already been applied server-side via `renderable_content()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireCell {
    /// Displayed character.
    pub ch: char,
    /// Pre-resolved foreground color.
    pub fg: WireRgb,
    /// Pre-resolved background color.
    pub bg: WireRgb,
    /// SGR attribute flags (bold, italic, etc.).
    pub flags: WireCellFlags,
    /// Resolved underline color (`None` = use foreground).
    pub underline_color: Option<WireRgb>,
    /// OSC 8 hyperlink URI (`None` if no hyperlink).
    pub hyperlink_uri: Option<String>,
    /// Combining marks / zero-width characters.
    pub zerowidth: Vec<char>,
}

/// Cursor shape on the wire.
/// Stable `#[repr(u8)]` encoding decoupled from `oriterm_core::CursorShape`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum WireCursorShape {
    /// Filled block cursor.
    #[default]
    Block = 0,
    /// Underline cursor.
    Underline = 1,
    /// Vertical bar cursor.
    Bar = 2,
    /// Hollow (outline) block cursor.
    HollowBlock = 3,
    /// Cursor hidden.
    Hidden = 4,
}

impl WireCursorShape {
    /// Convert from wire `u8` discriminant, defaulting to `Block` for unknown values.
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Underline,
            2 => Self::Bar,
            3 => Self::HollowBlock,
            4 => Self::Hidden,
            _ => Self::Block,
        }
    }
}

impl From<oriterm_core::CursorShape> for WireCursorShape {
    fn from(shape: oriterm_core::CursorShape) -> Self {
        match shape {
            oriterm_core::CursorShape::Block => Self::Block,
            oriterm_core::CursorShape::Underline => Self::Underline,
            oriterm_core::CursorShape::Bar => Self::Bar,
            oriterm_core::CursorShape::HollowBlock => Self::HollowBlock,
            oriterm_core::CursorShape::Hidden => Self::Hidden,
        }
    }
}

impl From<WireCursorShape> for oriterm_core::CursorShape {
    fn from(shape: WireCursorShape) -> Self {
        match shape {
            WireCursorShape::Block => Self::Block,
            WireCursorShape::Underline => Self::Underline,
            WireCursorShape::Bar => Self::Bar,
            WireCursorShape::HollowBlock => Self::HollowBlock,
            WireCursorShape::Hidden => Self::Hidden,
        }
    }
}

/// Cursor state on the wire.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireCursor {
    /// Column (0-indexed).
    pub col: u16,
    /// Row (0-indexed, within viewport).
    pub row: u16,
    /// Cursor shape.
    pub shape: WireCursorShape,
    /// Whether the cursor is visible.
    pub visible: bool,
}

/// A search match position on the wire.
/// Uses raw `u64` for stable row indices and `u16` for columns,
/// decoupled from `oriterm_core::SearchMatch` (which uses `StableRowIndex`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireSearchMatch {
    /// Stable row of match start.
    pub start_row: u64,
    /// Start column.
    pub start_col: u16,
    /// Stable row of match end.
    pub end_row: u64,
    /// End column (inclusive).
    pub end_col: u16,
}

/// Image placement on the wire — viewport position + UV + ordering for a referenced `ImageId`.
/// Mirrors [`oriterm_core::RenderablePlacement`] but with `image_id: u32` (the raw
/// `ImageId` inner value) so the wire schema decouples from the Rust newtype.
/// f32 fields preclude `Eq`; this drops `PaneSnapshot`'s `Eq` derive transitively (and `MuxPdu`'s),
/// so wire-protocol types now derive `PartialEq` only. `assert_eq!` callers continue to work
/// (`assert_eq!` requires `PartialEq + Debug`, not `Eq`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WirePlacement {
    /// Raw `ImageId` inner value (per `oriterm_core::ImageId(pub(crate) u32)`).
    pub image_id: u32,
    /// Viewport X position in pixels (top-left corner).
    pub viewport_x: f32,
    /// Viewport Y position in pixels (top-left corner).
    pub viewport_y: f32,
    /// Display width in pixels.
    pub display_width: f32,
    /// Display height in pixels.
    pub display_height: f32,
    /// UV source rect origin X (0.0–1.0).
    pub source_x: f32,
    /// UV source rect origin Y (0.0–1.0).
    pub source_y: f32,
    /// UV source rect width (0.0–1.0).
    pub source_w: f32,
    /// UV source rect height (0.0–1.0).
    pub source_h: f32,
    /// Layer ordering: negative = below text, positive = above text.
    pub z_index: i32,
    /// Opacity for fade transitions (default 1.0).
    pub opacity: f32,
}

/// Decoded image pixel data on the wire — RGBA bytes + dimensions for one `ImageId`.
/// Owned `Vec<u8>` for the pixel buffer (necessary for bincode `Deserialize`). Server-side
/// fill path clones `Arc<Vec<u8>>::to_vec()` on the slow path (when image data must flow
/// over the wire — first-observation OR `images_dirty=true`); fast path (steady-state
/// shared-PDU) carries empty `image_data` and skips the clone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireImageData {
    /// Raw `ImageId` inner value.
    pub id: u32,
    /// Decoded RGBA pixel bytes.
    pub data: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Monotonic counter from `ImageData::pixel_generation` so client-side
    /// GPU upload can detect mutated frames after IPC transfer.
    /// Defaults to 0 for backward compatibility with pre-cure snapshots
    /// (a single un-mutated image generation).
    #[serde(default)]
    pub pixel_generation: u64,
}

/// Full snapshot of a pane's visible state.
/// Transferred when a client subscribes to a pane or explicitly requests
/// a snapshot. Contains everything needed to render the pane from scratch.
/// `Default` produces an empty snapshot suitable as an initial cache entry.
/// `Eq` is no longer derived because `WirePlacement` contains f32 fields. `PartialEq`
/// continues to work for `==` and `assert_eq!`. Callers do not require `Eq` semantics
/// on `PaneSnapshot` (verified — no `HashSet<PaneSnapshot>` or `HashMap<PaneSnapshot, _>`
/// use exists in the workspace).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PaneSnapshot {
    /// Visible grid contents (rows × cols).
    pub cells: Vec<Vec<WireCell>>,
    /// Cursor position and shape.
    pub cursor: WireCursor,
    /// Color palette as 270 RGB triplets.
    pub palette: Vec<[u8; 3]>,
    /// Image placements visible in this snapshot.
    /// Small structs (~52 bytes each); always populated when source has placements.
    /// The pixel data for each placement lives in `image_data` (when transmitted inline)
    /// OR in the client's per-pane image cache (resolved at extract time).
    pub images: Vec<WirePlacement>,
    /// Decoded pixel data for newly-needed `ImageId`s.
    /// Populated by the server-side dispatch layer ONLY when (a) `images_dirty == true`
    /// for this snapshot, OR (b) one or more `images[*].image_id` is new-to-this-client
    /// (first-observation handshake per `ClientConnection.sent_images` tracking).
    /// Steady-state frames (no new images, no dirty flag) carry an empty `image_data`.
    pub image_data: Vec<WireImageData>,
    /// Whether the image cache changed since the last snapshot for this pane.
    /// When `true`, the client clears its cached image data and treats the wire snapshot's
    /// `image_data` as authoritative. Mirrors `oriterm_core::RenderableContent.images_dirty`.
    /// When `true`, the client also sets `out.all_dirty = true` to force a full repaint
    /// (image cache changes don't tag per-line grid damage).
    pub images_dirty: bool,
    /// Pane title (resolved via `effective_title()`).
    pub title: String,
    /// Icon name (from OSC 0/1), if set.
    pub icon_name: Option<String>,
    /// Current working directory (from OSC 7), if known.
    pub cwd: Option<String>,
    /// Terminal mode flags as raw bits.
    /// # Wire format
    /// | Bit | Mode |
    /// |-----|------|
    /// | 0 | `SHOW_CURSOR` (DECTCEM) |
    /// | 1 | `APP_CURSOR` (DECCKM) |
    /// | 2 | `APP_KEYPAD` (DECKPAM/DECKPNM) |
    /// | 3 | `MOUSE_REPORT_CLICK` (mode 1000) |
    /// | 4 | `MOUSE_DRAG` (mode 1002) |
    /// | 5 | `MOUSE_MOTION` (mode 1003) |
    /// | 6 | `MOUSE_SGR` (mode 1006) |
    /// | 7 | `MOUSE_UTF8` (mode 1005) |
    /// | 8 | `ALT_SCREEN` (mode 1049) |
    /// | 9 | `LINE_WRAP` (DECAWM) |
    /// | 10 | `ORIGIN` (DECOM) |
    /// | 11 | `INSERT` (IRM) |
    /// | 12 | `FOCUS_IN_OUT` (mode 1004) |
    /// | 13 | `BRACKETED_PASTE` (mode 2004) |
    /// | 14 | `SYNC_UPDATE` (mode 2026) |
    /// | 15 | `URGENCY_HINTS` (mode 1042) |
    /// | 16 | `CURSOR_BLINKING` (ATT610) |
    /// | 17 | `LINE_FEED_NEW_LINE` (LNM) |
    /// | 18 | `DISAMBIGUATE_ESC_CODES` (Kitty) |
    /// | 19 | `REPORT_EVENT_TYPES` (Kitty) |
    /// | 20 | `REPORT_ALTERNATE_KEYS` (Kitty) |
    /// | 21 | `REPORT_ALL_KEYS_AS_ESC` (Kitty) |
    /// | 22 | `REPORT_ASSOCIATED_TEXT` (Kitty) |
    /// | 23 | `ALTERNATE_SCROLL` (mode 1007) |
    /// | 24 | `REVERSE_WRAP` (mode 45) |
    /// | 25 | `MOUSE_URXVT` (mode 1015) |
    /// | 26 | `MOUSE_X10` (mode 9) |
    pub modes: u64,
    /// Number of scrollback rows above the viewport.
    pub scrollback_len: u32,
    /// Current scroll position (0 = bottom, `scrollback_len` = top).
    pub display_offset: u32,
    /// Absolute row index of the first viewport row.
    /// Matches `StableRowIndex` semantics: accounts for scrollback eviction,
    /// not just current buffer length.
    pub stable_row_base: u64,
    /// Whether the pane has output the user hasn't seen (background tab).
    pub has_unseen_output: bool,
    /// Grid column count.
    /// Explicit to avoid fragile `cells[0].len()` inference.
    pub cols: u16,

    // -- Search state --
    /// Whether search is currently active for this pane.
    pub search_active: bool,
    /// Current search query (may be empty while search UI is still open).
    pub search_query: String,
    /// All search matches (sorted by position).
    pub search_matches: Vec<WireSearchMatch>,
    /// Index of the focused match (`None` if no matches).
    pub search_focused: Option<u32>,
    /// Total match count across the full scrollback.
    pub search_total_matches: u32,
    /// Mouse cursor icon requested by the shell (OSC 22), encoded as a
    /// stable `u8` index per [`encode_cursor_icon`] / [`decode_cursor_icon`].
    /// `None` means no OSC 22 has been received (or the icon is unknown).
    /// The wire index space is project-owned (not `CursorIcon::default()`
    /// ordinal) so reordering of upstream `cursor_icon` variants cannot
    /// invalidate serialized daemon snapshots.
    pub mouse_cursor_icon: Option<u8>,
}

/// A selection on the wire.
/// Encodes the three-point selection model (`anchor`, `pivot`, `end`) with
/// mode, stable row indices, and side information. Decoupled from
/// `oriterm_core::Selection` (which uses `StableRowIndex` and `Side` enums).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireSelection {
    /// Selection mode: 0=Char, 1=Word, 2=Line, 3=Block.
    pub mode: u8,
    /// Anchor point.
    pub anchor_row: u64,
    /// Anchor column.
    pub anchor_col: u32,
    /// Anchor side: 0=Left, 1=Right.
    pub anchor_side: u8,
    /// Pivot point.
    pub pivot_row: u64,
    /// Pivot column.
    pub pivot_col: u32,
    /// Pivot side.
    pub pivot_side: u8,
    /// End point.
    pub end_row: u64,
    /// End column.
    pub end_col: u32,
    /// End side.
    pub end_side: u8,
}

impl WireSelection {
    /// Convert from an `oriterm_core::Selection`.
    pub fn from_selection(sel: &Selection) -> Self {
        Self {
            mode: match sel.mode {
                SelectionMode::Char => 0,
                SelectionMode::Word => 1,
                SelectionMode::Line => 2,
                SelectionMode::Block => 3,
            },
            anchor_row: sel.anchor.row.0,
            anchor_col: sel.anchor.col as u32,
            anchor_side: match sel.anchor.side {
                Side::Left => 0,
                Side::Right => 1,
            },
            pivot_row: sel.pivot.row.0,
            pivot_col: sel.pivot.col as u32,
            pivot_side: match sel.pivot.side {
                Side::Left => 0,
                Side::Right => 1,
            },
            end_row: sel.end.row.0,
            end_col: sel.end.col as u32,
            end_side: match sel.end.side {
                Side::Left => 0,
                Side::Right => 1,
            },
        }
    }

    /// Convert to an `oriterm_core::Selection`.
    pub fn to_selection(&self) -> Selection {
        let mode = match self.mode {
            1 => SelectionMode::Word,
            2 => SelectionMode::Line,
            3 => SelectionMode::Block,
            _ => SelectionMode::Char,
        };
        let side = |s: u8| if s == 1 { Side::Right } else { Side::Left };
        Selection {
            mode,
            anchor: SelectionPoint {
                row: StableRowIndex(self.anchor_row),
                col: self.anchor_col as usize,
                side: side(self.anchor_side),
            },
            pivot: SelectionPoint {
                row: StableRowIndex(self.pivot_row),
                col: self.pivot_col as usize,
                side: side(self.pivot_side),
            },
            end: SelectionPoint {
                row: StableRowIndex(self.end_row),
                col: self.end_col as usize,
                side: side(self.end_side),
            },
        }
    }
}

/// Known OSC 22 mouse cursor icons and their stable wire indices.
/// The wire index space is project-owned so reordering of variants in the
/// upstream `cursor_icon` crate cannot invalidate serialized daemon
/// snapshots. Only icons listed here round-trip through the wire; unknown
/// icons encode as `None` (decoder yields `None`, renderer falls back to
/// the default pointer).
/// New icons MUST be appended; existing entries MUST NOT change index.
/// Section 10.5 references this slice to build its OSC 22 matrix.
pub const OSC22_KNOWN_ICONS: &[CursorIcon] = &[
    CursorIcon::Default,
    CursorIcon::ContextMenu,
    CursorIcon::Help,
    CursorIcon::Pointer,
    CursorIcon::Progress,
    CursorIcon::Wait,
    CursorIcon::Cell,
    CursorIcon::Crosshair,
    CursorIcon::Text,
    CursorIcon::VerticalText,
    CursorIcon::Alias,
    CursorIcon::Copy,
    CursorIcon::Move,
    CursorIcon::NoDrop,
    CursorIcon::NotAllowed,
    CursorIcon::Grab,
    CursorIcon::Grabbing,
    CursorIcon::EResize,
    CursorIcon::NResize,
    CursorIcon::NeResize,
    CursorIcon::NwResize,
    CursorIcon::SResize,
    CursorIcon::SeResize,
    CursorIcon::SwResize,
    CursorIcon::WResize,
    CursorIcon::EwResize,
    CursorIcon::NsResize,
    CursorIcon::NeswResize,
    CursorIcon::NwseResize,
    CursorIcon::ColResize,
    CursorIcon::RowResize,
    CursorIcon::AllScroll,
    CursorIcon::ZoomIn,
    CursorIcon::ZoomOut,
];

/// Encode a `CursorIcon` into a stable wire index, or `None` if the icon
/// is not in [`OSC22_KNOWN_ICONS`].
/// Uses linear scan; the slice is ~34 entries so O(n) is cheap compared to
/// a per-variant match table (which would not be kept in sync on upstream
/// variant reorderings).
#[must_use]
pub fn encode_cursor_icon(icon: CursorIcon) -> Option<u8> {
    OSC22_KNOWN_ICONS
        .iter()
        .position(|&known| known == icon)
        .and_then(|idx| u8::try_from(idx).ok())
}

/// Decode a wire index back into a `CursorIcon`. Out-of-range indices
/// return `None` (renderer falls back to the default pointer).
#[must_use]
pub fn decode_cursor_icon(index: u8) -> Option<CursorIcon> {
    OSC22_KNOWN_ICONS.get(index as usize).copied()
}
