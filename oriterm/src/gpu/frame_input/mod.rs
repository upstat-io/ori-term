//! Frame input types for the Extract phase of the render pipeline.
//!
//! [`FrameInput`] composes `oriterm_core::RenderableContent` (the terminal
//! snapshot) with rendering context: viewport pixel dimensions, cell metrics,
//! and semantic palette colors. The Prepare phase consumes a `FrameInput` and
//! produces a [`PreparedFrame`](super::prepared_frame::PreparedFrame).

mod search_match;

use oriterm_core::grid::StableRowIndex;
use oriterm_core::index::Side;
use oriterm_core::search::MatchType;
use oriterm_core::selection::{Selection, SelectionBounds, SelectionMode};
use oriterm_core::{Column, CursorShape, RenderableContent, Rgb, SearchMatch};
use oriterm_mux::PaneSnapshot;

use crate::font::CellMetrics;
use crate::url_detect::UrlSegment;

/// Selection state snapshotted for one frame.
///
/// Encapsulates [`SelectionBounds`] with the viewport→stable row mapping
/// so the Prepare phase can test containment without terminal access.
#[derive(Debug)]
pub struct FrameSelection {
    bounds: SelectionBounds,
    /// Stable row index of viewport line 0.
    base_stable: u64,
}

impl FrameSelection {
    /// Build from an active selection and the viewport's stable row base.
    ///
    /// `stable_row_base` is `RenderableContent::stable_row_base` — the
    /// `StableRowIndex` value of viewport line 0.
    pub fn new(selection: &Selection, stable_row_base: u64) -> Self {
        Self {
            bounds: selection.bounds(),
            base_stable: stable_row_base,
        }
    }

    /// Test whether a visible cell at (`viewport_line`, `col`) is selected.
    pub fn contains(&self, viewport_line: usize, col: usize) -> bool {
        let stable = StableRowIndex(self.base_stable + viewport_line as u64);
        self.bounds.contains(stable, col)
    }

    /// Compute the viewport line range covered by this selection.
    ///
    /// Returns `Some((start, end))` where both are inclusive viewport line
    /// indices clamped to `[0, num_rows)`. Returns `None` if the selection
    /// is entirely outside the viewport.
    pub fn viewport_line_range(&self, num_rows: usize) -> Option<(usize, usize)> {
        if num_rows == 0 {
            return None;
        }

        let sel_start = self.bounds.start.row.0;
        let sel_end = self.bounds.end.row.0;

        // Selection entirely above viewport.
        if sel_end < self.base_stable {
            return None;
        }

        // Convert stable rows to viewport-relative, clamping to [0, num_rows).
        let start = if sel_start >= self.base_stable {
            (sel_start - self.base_stable) as usize
        } else {
            0 // Selection starts above viewport.
        };

        let end = (sel_end - self.base_stable) as usize;

        // Selection entirely below viewport.
        if start >= num_rows {
            return None;
        }

        Some((start, end.min(num_rows - 1)))
    }

    /// Snapshot the selection state for incremental damage tracking.
    ///
    /// Captures line range, column extents, and mode so the incremental
    /// path detects intra-line selection changes (e.g. same-row drag).
    pub fn damage_snapshot(&self, num_rows: usize) -> Option<SelectionDamageSnapshot> {
        let (start_line, end_line) = self.viewport_line_range(num_rows)?;
        Some(SelectionDamageSnapshot {
            start_line,
            end_line,
            start_col: self.bounds.start.col,
            start_side: self.bounds.start.side,
            end_col: self.bounds.end.col,
            end_side: self.bounds.end.side,
            mode: self.bounds.mode,
        })
    }
}

/// Compact snapshot of selection state for incremental damage tracking.
///
/// Compared between frames to determine which rows need regeneration
/// when the selection changes (including intra-line column changes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionDamageSnapshot {
    /// First viewport line (inclusive).
    pub start_line: usize,
    /// Last viewport line (inclusive).
    pub end_line: usize,
    /// Column of the selection start point.
    pub start_col: usize,
    /// Side of the selection start point.
    pub start_side: Side,
    /// Column of the selection end point.
    pub end_col: usize,
    /// Side of the selection end point.
    pub end_side: Side,
    /// Selection mode (char/word/line/block affects damage scope).
    pub mode: SelectionMode,
}

/// Search rendering snapshot for one frame.
///
/// Contains the match data and viewport mapping needed to classify cells
/// for search highlighting. Built from `PaneSnapshot` search fields —
/// copies the match list for frame-local access.
#[derive(Debug)]
pub struct FrameSearch {
    /// Matches from the search state (copied per frame).
    matches: Vec<SearchMatch>,
    /// Index of the focused match.
    focused: usize,
    /// Stable row index of viewport line 0.
    base_stable: u64,
    /// Total match count (for search bar "N of M" display).
    match_count: usize,
    /// Query string (for search bar display).
    query: String,
}

impl FrameSearch {
    /// Build from snapshot search data.
    ///
    /// Converts wire-format search matches into `SearchMatch` values
    /// for client-side highlight rendering. Used in daemon mode where
    /// search state lives on the server.
    pub fn from_snapshot(snapshot: &PaneSnapshot) -> Option<Self> {
        if !snapshot.search_active {
            return None;
        }
        let matches: Vec<SearchMatch> = snapshot
            .search_matches
            .iter()
            .map(|m| SearchMatch {
                start_row: StableRowIndex(m.start_row),
                start_col: m.start_col as usize,
                end_row: StableRowIndex(m.end_row),
                end_col: m.end_col as usize,
            })
            .collect();
        let match_count = matches.len();
        let focused = snapshot.search_focused.map_or(0, |f| f as usize);
        Some(Self {
            matches,
            focused,
            base_stable: snapshot.stable_row_base,
            match_count,
            query: snapshot.search_query.clone(),
        })
    }

    /// Refill this `FrameSearch` from a snapshot, reusing allocations.
    ///
    /// Returns `false` if search is not active (caller should set field to `None`).
    #[allow(
        dead_code,
        reason = "infrastructure for allocation-reusing extract path"
    )]
    pub fn update_from_snapshot(&mut self, snapshot: &PaneSnapshot) -> bool {
        if !snapshot.search_active {
            return false;
        }
        self.matches.clear();
        self.matches
            .extend(snapshot.search_matches.iter().map(|m| SearchMatch {
                start_row: StableRowIndex(m.start_row),
                start_col: m.start_col as usize,
                end_row: StableRowIndex(m.end_row),
                end_col: m.end_col as usize,
            }));
        self.match_count = self.matches.len();
        self.focused = snapshot.search_focused.map_or(0, |f| f as usize);
        self.base_stable = snapshot.stable_row_base;
        self.query.clear();
        self.query.push_str(&snapshot.search_query);
        true
    }

    /// Classify a visible cell for search match highlighting.
    pub fn cell_match_type(&self, viewport_line: usize, col: usize) -> MatchType {
        if self.matches.is_empty() {
            return MatchType::None;
        }
        let stable = StableRowIndex(self.base_stable + viewport_line as u64);

        // Binary search: find first match whose start is beyond (row, col).
        let idx = self
            .matches
            .partition_point(|m| (m.start_row, m.start_col) <= (stable, col));

        let start = idx.saturating_sub(1);
        let end = (idx + 1).min(self.matches.len());

        for i in start..end {
            if cell_in_search_match(&self.matches[i], stable, col) {
                return if i == self.focused {
                    MatchType::FocusedMatch
                } else {
                    MatchType::Match
                };
            }
        }
        MatchType::None
    }

    /// Total number of matches.
    pub fn match_count(&self) -> usize {
        self.match_count
    }

    /// 1-based focused match index (for "N of M" display).
    pub fn focused_display(&self) -> usize {
        if self.match_count == 0 {
            0
        } else {
            self.focused + 1
        }
    }

    /// The current query string.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Build a test search snapshot from manually constructed matches.
    ///
    /// `focused` is the index into `matches` of the focused match.
    /// `stable_row_base` maps viewport line 0 to stable row coordinates.
    #[cfg(test)]
    pub fn for_test(matches: Vec<SearchMatch>, focused: usize, stable_row_base: u64) -> Self {
        Self {
            match_count: matches.len(),
            matches,
            focused,
            base_stable: stable_row_base,
            query: String::from("test"),
        }
    }
}

/// Mark-mode cursor override for the Prepare phase.
///
/// When mark mode is active, the app sets this on [`FrameInput`] so the
/// Prepare phase renders a hollow block at the mark position instead of
/// the terminal's real cursor. The extract snapshot (`content.cursor`)
/// is never mutated — this override is a separate rendering concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkCursorOverride {
    /// Viewport line (0 = top of visible area).
    pub line: usize,
    /// Column (0-based).
    pub column: Column,
    /// Cursor shape to render (always `HollowBlock` for mark mode).
    pub shape: CursorShape,
}

/// Pixel dimensions of the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportSize {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
}

impl ViewportSize {
    /// Create a viewport size from pixel dimensions.
    ///
    /// Dimensions are clamped to a minimum of 1 to avoid zero-size surfaces.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: width.max(1),
            height: height.max(1),
        }
    }
}

/// Semantic colors needed beyond per-cell resolved colors.
///
/// Per-cell fg/bg are already resolved in `RenderableCell`. This captures
/// only the three global colors the renderer needs: clear color, cursor
/// fill, and text-under-cursor inversion color — plus the window opacity
/// for transparent rendering and optional selection color overrides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FramePalette {
    /// Window clear color (terminal background).
    pub background: Rgb,
    /// Default foreground (used for cursor text inversion).
    pub foreground: Rgb,
    /// Cursor rectangle fill color.
    pub cursor_color: Rgb,
    /// Window opacity (0.0 = fully transparent, 1.0 = fully opaque).
    pub opacity: f32,
    /// Explicit selection foreground (from scheme or config override).
    pub selection_fg: Option<Rgb>,
    /// Explicit selection background (from scheme or config override).
    pub selection_bg: Option<Rgb>,
}

/// Complete input for one render frame.
///
/// Composes the terminal snapshot ([`RenderableContent`]) with the rendering
/// context needed to convert logical cells into pixel geometry. Built during
/// the Extract phase, consumed by the Prepare phase.
#[derive(Debug)]
pub struct FrameInput {
    /// Terminal content snapshot (cells, cursor, damage, mode).
    pub content: RenderableContent,
    /// Viewport pixel dimensions.
    pub viewport: ViewportSize,
    /// Cell pixel dimensions from font metrics.
    pub cell_size: CellMetrics,
    /// Content grid columns (from snapshot, not viewport).
    ///
    /// During async resize in daemon mode, the viewport may have different
    /// dimensions than the snapshot content. This field records the actual
    /// column count of `content.cells` so shaping and rendering index the
    /// flat cell array correctly.
    pub content_cols: usize,
    /// Content grid rows (from snapshot, not viewport).
    pub content_rows: usize,
    /// Semantic colors for background clear and cursor.
    pub palette: FramePalette,
    /// Active selection for highlight rendering.
    pub selection: Option<FrameSelection>,
    /// Active search state for match highlighting.
    pub search: Option<FrameSearch>,
    /// Viewport cell under the mouse cursor for hyperlink hover detection.
    ///
    /// `(viewport_line, column)`. Set from mouse state after extraction;
    /// `None` when the cursor is outside the grid.
    pub hovered_cell: Option<(usize, usize)>,
    /// Viewport-relative segments of an implicitly detected URL being hovered.
    ///
    /// Each entry is `(viewport_line, start_col, end_col)` inclusive. Set when
    /// Ctrl is held and the cursor is over a detected URL. Empty when no
    /// implicit URL is hovered.
    pub hovered_url_segments: Vec<UrlSegment>,
    /// Mark-mode cursor override.
    ///
    /// When set, the Prepare phase renders this cursor instead of
    /// `content.cursor`. Set by the app layer after extraction when mark
    /// mode is active; the extracted content is never mutated.
    pub mark_cursor: Option<MarkCursorOverride>,
    /// Whether the containing window has OS-level focus.
    ///
    /// When `false`, the cursor renders as a hollow block regardless of the
    /// terminal's configured cursor shape. Set from `App::focused_window_id`.
    pub window_focused: bool,
    /// Foreground alpha multiplier for inactive pane dimming.
    ///
    /// 1.0 = fully opaque (default, focused pane). Values < 1.0 dim glyph
    /// alpha proportionally for unfocused panes. Set by the multi-pane
    /// render path; single-pane rendering always uses 1.0.
    pub fg_dim: f32,
    /// Whether subpixel glyph positioning is enabled.
    ///
    /// When `false`, all glyph X offsets snap to integer pixels (no fractional
    /// subpixel phase). Propagated from `WindowRenderer::subpixel_positioning`.
    pub subpixel_positioning: bool,
    /// Viewport-relative line indices that have a prompt marker (OSC 133;A).
    ///
    /// Populated during extraction when shell integration is active. The
    /// Prepare phase draws a thin colored bar at the left margin for each
    /// listed row. Empty when prompt markers are disabled or no markers are
    /// visible in the current viewport.
    pub prompt_marker_rows: Vec<usize>,
}

impl FrameInput {
    /// Number of grid columns in the content.
    ///
    /// Returns the content-derived column count, not viewport-derived.
    /// During async resize in daemon mode, the viewport may race ahead of
    /// the snapshot — using viewport dimensions to index the flat cell
    /// array would read cells at wrong offsets, placing text fragments on
    /// wrong lines.
    pub fn columns(&self) -> usize {
        self.content_cols
    }

    /// Number of grid rows in the content.
    ///
    /// Returns the content-derived row count. See [`columns()`](Self::columns).
    pub fn rows(&self) -> usize {
        self.content_rows
    }

    /// Whether the entire viewport needs a full repaint.
    #[allow(dead_code, reason = "damage tracking optimization for later sections")]
    pub fn needs_full_repaint(&self) -> bool {
        self.content.all_dirty
    }

    /// Build a test frame from a text string.
    ///
    /// Creates a grid of `cols × rows` cells. `text` is laid out left-to-right,
    /// top-to-bottom; cells beyond the text length are filled with spaces. All
    /// cells use default dark-theme colors. Cell size is 8×16 px.
    #[cfg(test)]
    pub fn test_grid(cols: usize, rows: usize, text: &str) -> Self {
        use oriterm_core::{CellFlags, Column, RenderableCell, RenderableContent, TermMode};

        let fg = Rgb {
            r: 211,
            g: 215,
            b: 207,
        };
        // Cell bg differs from palette background so that bg quads are
        // emitted in tests (the prepare phase skips cells whose bg matches
        // the palette background to support window transparency/glass).
        let bg = Rgb {
            r: 30,
            g: 30,
            b: 46,
        };
        let palette_bg = Rgb { r: 0, g: 0, b: 0 };

        let mut cells = Vec::with_capacity(cols * rows);
        let mut chars = text.chars();

        for row in 0..rows {
            for col in 0..cols {
                let ch = chars.next().unwrap_or(' ');
                cells.push(RenderableCell {
                    line: row,
                    column: Column(col),
                    ch,
                    fg,
                    bg,
                    flags: CellFlags::empty(),
                    underline_color: None,
                    has_hyperlink: false,
                    hyperlink_uri: None,
                    zerowidth: Vec::new(),
                });
            }
        }

        let mut content = RenderableContent::default();
        content.cells = cells;
        content.cursor.visible = true;
        content.mode = TermMode::SHOW_CURSOR;
        content.all_dirty = true;

        Self {
            content,
            viewport: ViewportSize::new(cols as u32 * 8, rows as u32 * 16),
            cell_size: CellMetrics::new(8.0, 16.0, 12.0, 2.0, 1.0, 4.0),
            content_cols: cols,
            content_rows: rows,
            palette: FramePalette {
                background: palette_bg,
                foreground: fg,
                cursor_color: Rgb {
                    r: 255,
                    g: 255,
                    b: 255,
                },
                opacity: 1.0,
                selection_fg: None,
                selection_bg: None,
            },
            selection: None,
            search: None,
            hovered_cell: None,
            hovered_url_segments: Vec::new(),
            mark_cursor: None,
            window_focused: true,
            fg_dim: 1.0,
            subpixel_positioning: true,
            prompt_marker_rows: Vec::new(),
        }
    }
}

use search_match::cell_in_search_match;

#[cfg(test)]
mod tests;
