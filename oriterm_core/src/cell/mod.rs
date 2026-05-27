//! Terminal cell types.
//!
//! A `Cell` represents one character position in the terminal grid. Most cells
//! are 24 bytes on the stack. Only cells with combining marks, colored
//! underlines, or hyperlinks allocate heap storage via `CellExtra`. Extra data
//! is stored behind `Arc` so cloning a cell (e.g. from cursor template) is O(1).

use std::fmt;
use std::sync::Arc;

use bitflags::bitflags;
use unicode_width::UnicodeWidthChar;
use vte::ansi::Color;

/// Fully-opaque per-channel alpha. Every cell defaults to this on all
/// channels; only SGR mode-6 (`38:6`/`48:6`/`58:6`) sets a lower value.
pub const OPAQUE_ALPHA: u8 = 255;

bitflags! {
 /// Per-cell attribute flags (SGR and internal).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CellFlags: u32 {
        const BOLD              = 1 << 0;
        const DIM               = 1 << 1;
        const ITALIC            = 1 << 2;
        const UNDERLINE         = 1 << 3;
        const BLINK             = 1 << 4;
        const INVERSE           = 1 << 5;
        const HIDDEN            = 1 << 6;
        const STRIKETHROUGH     = 1 << 7;
        const WIDE_CHAR                 = 1 << 8;
        const WIDE_CHAR_SPACER          = 1 << 9;
        const WRAP                      = 1 << 10;
        const CURLY_UNDERLINE   = 1 << 11;
        const DOTTED_UNDERLINE  = 1 << 12;
        const DASHED_UNDERLINE  = 1 << 13;
        const DOUBLE_UNDERLINE  = 1 << 14;
 /// Padding cell before a wide char that wrapped to the next line.
 ///
 /// Inserted at `cols - 1` when a wide char can't fit and wraps.
 /// Skipped during text extraction, selection, search, and reflow
 /// to avoid spurious spaces.
        const LEADING_WIDE_CHAR_SPACER  = 1 << 15;
        const OVERLINE          = 1 << 16;
        const SUPERSCRIPT       = 1 << 17;
        const SUBSCRIPT         = 1 << 18;

 /// Cell has been written by the application (xterm CHARDRAWN
 /// equivalent). Set at every put_char / wide-spacer / leading-
 /// wide-spacer / DECALN / reflow-synthesized-spacer /
 /// push_zerowidth write site; cleared at every reset path via
 /// template copy (templates must never carry DRAWN — enforced
 /// by `debug_assert!` in the Grid write paths). Consumed by
 /// DECRQCRA (`CSI * y`) to distinguish application-written
 /// blanks from pristine cells per xterm `screen.c:3178-3180`.
        const DRAWN             = 1 << 19;

 /// DECSCA per-character protection attribute. Set on cells
 /// written while `Term::char_protection == true` (DECSCA Ps=1).
 /// Consumed by DECSERA (`CSI $ {`) — protected cells survive
 /// selective erase. DECERA (`CSI $ z`) erases unconditionally.
 /// This is a semantic per-character attribute (spec-defined),
 /// NOT a structural internal-state bit; it does NOT join
 /// `INTERNAL_CELL_STATE` and cells may legitimately carry it
 /// without violating the cursor-template invariant.
        const PROTECTED         = 1 << 20;

 /// Cell carries non-opaque per-channel alpha (SGR mode-6 RGBA).
 ///
 /// Fast-skip bit: when clear, all channels are opaque and the GPU emit
 /// path skips the `CellExtra` alpha deref. The concrete 0-255 per-channel
 /// bytes live in `CellExtra` (`fg_alpha`/`bg_alpha`/`underline_alpha`) per
 /// Decision 08 (Option C); this bit mirrors "any channel < 255". A
 /// template-legal SGR attribute (rides the cursor template like fg/bg) —
 /// does NOT join `INTERNAL_CELL_STATE`.
        const HAS_ALPHA         = 1 << 21;

 /// Union of all underline variants for mutual exclusion.
        const ALL_UNDERLINES = Self::UNDERLINE.bits()
            | Self::DOUBLE_UNDERLINE.bits()
            | Self::CURLY_UNDERLINE.bits()
            | Self::DOTTED_UNDERLINE.bits()
            | Self::DASHED_UNDERLINE.bits();

 /// Internal cell-state bits that must NEVER appear on a
 /// `cursor.template.flags` value. SGR attributes (BOLD /
 /// UNDERLINE / COLORED / …) are template-legal; these
 /// structural bits are set only by concrete cell-write paths
 /// and would corrupt written cells if they leaked via the
 /// template. `Grid::put_char_ascii` and `Grid::put_char_slow`
 /// debug_assert that `cursor.template.flags & INTERNAL_CELL_STATE`
 /// is empty on every write.
        const INTERNAL_CELL_STATE = Self::DRAWN.bits()
            | Self::WRAP.bits()
            | Self::WIDE_CHAR.bits()
            | Self::WIDE_CHAR_SPACER.bits()
            | Self::LEADING_WIDE_CHAR_SPACER.bits();
    }
}

impl Default for CellFlags {
    fn default() -> Self {
        Self::empty()
    }
}

/// Heap-allocated optional data for cells that need it.
///
/// Only allocated when a cell has combining marks, a colored underline,
/// a hyperlink, or non-opaque per-channel alpha. Normal cells keep
/// `extra: None` (zero overhead).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellExtra {
    /// Colored underline (SGR 58).
    pub underline_color: Option<Color>,
    /// OSC 8 hyperlink.
    pub hyperlink: Option<Hyperlink>,
    /// Combining marks and zero-width characters appended to this cell.
    pub zerowidth: Vec<char>,
    /// Concrete foreground alpha (SGR mode-6 `38:6`), 0-255. 255 = opaque.
    pub fg_alpha: u8,
    /// Concrete background alpha (SGR mode-6 `48:6`), 0-255. 255 = opaque.
    pub bg_alpha: u8,
    /// Concrete underline alpha (SGR mode-6 `58:6`), 0-255. 255 = opaque.
    pub underline_alpha: u8,
}

impl CellExtra {
    /// Create an empty extra with all fields at their defaults (all alpha
    /// channels fully opaque).
    pub fn new() -> Self {
        Self {
            underline_color: None,
            hyperlink: None,
            zerowidth: Vec::new(),
            fg_alpha: OPAQUE_ALPHA,
            bg_alpha: OPAQUE_ALPHA,
            underline_alpha: OPAQUE_ALPHA,
        }
    }

    /// Returns `true` when this extra carries nothing — no underline color,
    /// no hyperlink, no combining marks, and all alpha channels opaque.
    ///
    /// SSOT for "the sidecar is droppable": every mutator that can empty a
    /// `CellExtra` checks this to decide whether to set `Cell::extra = None`.
    pub fn is_default(&self) -> bool {
        self.underline_color.is_none()
            && self.hyperlink.is_none()
            && self.zerowidth.is_empty()
            && self.fg_alpha == OPAQUE_ALPHA
            && self.bg_alpha == OPAQUE_ALPHA
            && self.underline_alpha == OPAQUE_ALPHA
    }
}

impl Default for CellExtra {
    fn default() -> Self {
        Self::new()
    }
}

/// OSC 8 hyperlink data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hyperlink {
    /// Optional link id for grouping.
    pub id: Option<String>,
    /// The URI target.
    pub uri: String,
}

impl fmt::Display for Hyperlink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.uri)
    }
}

impl From<vte::ansi::Hyperlink> for Hyperlink {
    fn from(h: vte::ansi::Hyperlink) -> Self {
        Self {
            id: h.id,
            uri: h.uri,
        }
    }
}

/// One character position in the terminal grid.
///
/// Target size: 24 bytes, fully packed with no padding:
/// `char(4) + Color(4) + Color(4) + CellFlags(4) + Option<Arc>(8)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The character stored in this cell.
    pub ch: char,
    /// Foreground color (deferred palette resolution).
    pub fg: Color,
    /// Background color (deferred palette resolution).
    pub bg: Color,
    /// SGR attribute flags.
    pub flags: CellFlags,
    /// Optional heap data for combining marks, underline color, or hyperlinks.
    ///
    /// Uses `Arc` so that cloning a cell with extra data (e.g. propagating
    /// cursor template attributes) is O(1) — a refcount bump instead of a
    /// heap allocation.
    pub extra: Option<Arc<CellExtra>>,
}

const _: () = assert!(size_of::<Cell>() <= 24);

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Color::Named(vte::ansi::NamedColor::Foreground),
            bg: Color::Named(vte::ansi::NamedColor::Background),
            flags: CellFlags::empty(),
            extra: None,
        }
    }
}

impl From<Color> for Cell {
    /// BCE (Background Color Erase) cell: default cell with a custom background.
    fn from(bg: Color) -> Self {
        Self {
            bg,
            ..Self::default()
        }
    }
}

impl Cell {
    /// Test helper: construct a drawn cell carrying the given character
    /// with default SGR + `CellFlags::DRAWN` set.
    ///
    /// Cuts the `Cell { ch: 'X', flags: CellFlags::DRAWN,..Cell::default() }`
    /// boilerplate that appears repeatedly in grid/rect-op/row tests.
    /// Use this (not bare `Cell::default` + flag insert) whenever a
    /// test needs to construct a cell that behaves like it was written
    /// by the application — so `compute_rect_checksum` and
    /// `Row::is_blank`/`content_len` see the correct write-history
    /// state per.
    #[cfg(test)]
    #[must_use]
    pub fn drawn(ch: char) -> Self {
        Self {
            ch,
            flags: CellFlags::DRAWN,
            ..Self::default()
        }
    }

    /// Reset this cell to match the given template.
    pub fn reset(&mut self, template: &Self) {
        self.ch = template.ch;
        self.fg = template.fg;
        self.bg = template.bg;
        self.flags = template.flags;
        self.extra.clone_from(&template.extra);
    }

    /// Returns `true` if this cell is visually empty (space, default
    /// colors, no visible SGR flags).
    ///
    /// `CellFlags::DRAWN` is MASKED OUT of the flags check: it's the
    /// xterm CHARDRAWN analog (write history, consumed by DECRQCRA)
    /// and is orthogonal to visual emptiness. A cell that was written
    /// by the application as a plain space under default SGR still
    /// LOOKS empty — so `is_empty()` returns true. Consumers that
    /// need the "never written" semantic (`compute_rect_checksum`)
    /// must query `flags.contains(CellFlags::DRAWN)` directly.
    ///
    /// `CellFlags::PROTECTED` is similarly masked out: DECSCA
    /// protection is a semantic attribute consumed by DECSERA, with
    /// no visual effect. A blank cell marked PROTECTED still LOOKS
    /// empty.
    pub fn is_empty(&self) -> bool {
        self.ch == ' '
            && self.fg == Color::Named(vte::ansi::NamedColor::Foreground)
            && self.bg == Color::Named(vte::ansi::NamedColor::Background)
            && (self.flags - (CellFlags::DRAWN | CellFlags::PROTECTED)).is_empty()
            && self.extra.is_none()
    }

    /// Display width of this cell's character.
    ///
    /// Respects the `WIDE_CHAR` flag and falls back to `unicode-width`.
    pub fn width(&self) -> usize {
        if self.flags.contains(CellFlags::WIDE_CHAR) {
            return 2;
        }
        if self
            .flags
            .intersects(CellFlags::WIDE_CHAR_SPACER | CellFlags::LEADING_WIDE_CHAR_SPACER)
        {
            return 0;
        }
        UnicodeWidthChar::width(self.ch).unwrap_or(1)
    }

    /// Set or clear the underline color (SGR 58/59).
    ///
    /// `Some(color)` allocates `CellExtra` if needed. `None` clears the
    /// underline color and drops `CellExtra` if it becomes empty.
    pub fn set_underline_color(&mut self, color: Option<Color>) {
        match color {
            Some(c) => {
                let extra = self.extra.get_or_insert_with(Default::default);
                Arc::make_mut(extra).underline_color = Some(c);
            }
            None => {
                if let Some(extra) = &mut self.extra {
                    Arc::make_mut(extra).underline_color = None;
                    if extra.is_default() {
                        self.extra = None;
                    }
                }
            }
        }
    }

    /// Set or clear the hyperlink (OSC 8).
    ///
    /// `Some(link)` allocates `CellExtra` if needed. `None` clears the
    /// hyperlink and drops `CellExtra` if it becomes empty.
    pub fn set_hyperlink(&mut self, link: Option<Hyperlink>) {
        match link {
            Some(l) => {
                let extra = self.extra.get_or_insert_with(Default::default);
                Arc::make_mut(extra).hyperlink = Some(l);
            }
            None => {
                if let Some(extra) = &mut self.extra {
                    Arc::make_mut(extra).hyperlink = None;
                    if extra.is_default() {
                        self.extra = None;
                    }
                }
            }
        }
    }

    /// Returns the hyperlink attached to this cell, if any.
    pub fn hyperlink(&self) -> Option<&Hyperlink> {
        self.extra.as_ref().and_then(|e| e.hyperlink.as_ref())
    }

    /// Append a combining mark (zero-width character) to this cell.
    ///
    /// Lazily allocates `CellExtra` on first combining mark. Uses
    /// `Arc::make_mut` for copy-on-write when the extra is shared.
    pub fn push_zerowidth(&mut self, ch: char) {
        let extra = self.extra.get_or_insert_with(Default::default);
        Arc::make_mut(extra).zerowidth.push(ch);
    }

    /// Foreground alpha (SGR mode-6 `38:6`). 255 (opaque) when the cell
    /// has no sidecar.
    pub fn fg_alpha(&self) -> u8 {
        self.extra.as_ref().map_or(OPAQUE_ALPHA, |e| e.fg_alpha)
    }

    /// Background alpha (SGR mode-6 `48:6`). 255 (opaque) when the cell
    /// has no sidecar.
    pub fn bg_alpha(&self) -> u8 {
        self.extra.as_ref().map_or(OPAQUE_ALPHA, |e| e.bg_alpha)
    }

    /// Underline alpha (SGR mode-6 `58:6`). 255 (opaque) when the cell
    /// has no sidecar.
    pub fn underline_alpha(&self) -> u8 {
        self.extra
            .as_ref()
            .map_or(OPAQUE_ALPHA, |e| e.underline_alpha)
    }

    /// Set the foreground alpha (SGR mode-6 `38:6`). 255 = opaque.
    ///
    /// Lazily allocates `CellExtra` only when `alpha < 255`; setting an
    /// already-opaque channel to 255 is a no-op that never allocates.
    pub fn set_fg_alpha(&mut self, alpha: u8) {
        self.set_channel_alpha(alpha, |extra| extra.fg_alpha = alpha);
    }

    /// Set the background alpha (SGR mode-6 `48:6`). 255 = opaque.
    pub fn set_bg_alpha(&mut self, alpha: u8) {
        self.set_channel_alpha(alpha, |extra| extra.bg_alpha = alpha);
    }

    /// Set the underline alpha (SGR mode-6 `58:6`). 255 = opaque.
    pub fn set_underline_alpha(&mut self, alpha: u8) {
        self.set_channel_alpha(alpha, |extra| extra.underline_alpha = alpha);
    }

    /// Shared per-channel alpha mutator: opaque-on-clean fast-skip (no
    /// sidecar alloc), `Arc::make_mut` COW allocation, `assign` the channel
    /// byte, then `refresh_alpha_state`. The three public setters delegate
    /// here with a per-channel field assignment (SSOT for the alpha-set path).
    fn set_channel_alpha(&mut self, alpha: u8, assign: impl FnOnce(&mut CellExtra)) {
        if alpha == OPAQUE_ALPHA && self.extra.is_none() {
            return;
        }
        assign(Arc::make_mut(
            self.extra.get_or_insert_with(Default::default),
        ));
        self.refresh_alpha_state();
    }

    /// Drop the sidecar if it became default, then re-sync the
    /// `CellFlags::HAS_ALPHA` fast-skip bit to "any channel non-opaque".
    ///
    /// SSOT for the flag↔sidecar invariant: `HAS_ALPHA` is set iff the
    /// sidecar exists with at least one channel below `OPAQUE_ALPHA`.
    fn refresh_alpha_state(&mut self) {
        if self.extra.as_ref().is_some_and(|e| e.is_default()) {
            self.extra = None;
        }
        let has_alpha = self.extra.as_ref().is_some_and(|e| {
            e.fg_alpha != OPAQUE_ALPHA
                || e.bg_alpha != OPAQUE_ALPHA
                || e.underline_alpha != OPAQUE_ALPHA
        });
        self.flags.set(CellFlags::HAS_ALPHA, has_alpha);
    }
}

#[cfg(test)]
mod tests;
