//! Per-cell color resolution and cursor override helpers.
//!
//! Pure functions that resolve the effective `(fg, bg)` for a cell,
//! accounting for selection inversion, search highlighting, block cursor
//! exclusion, INVERSE flag, and fg==bg reveal. Also resolves the
//! effective cursor when mark mode overrides the terminal cursor.

use oriterm_core::search::MatchType;
use oriterm_core::{CellFlags, Column, CursorShape, RenderableCell, RenderableCursor, Rgb};

use super::super::frame_input::{FramePalette, FrameSearch, FrameSelection, MarkCursorOverride};

/// Match highlight background: yellow-tinted for visibility.
pub(super) const SEARCH_MATCH_BG: Rgb = Rgb {
    r: 100,
    g: 100,
    b: 30,
};

/// Focused match highlight: brighter yellow.
pub(super) const SEARCH_FOCUSED_BG: Rgb = Rgb {
    r: 200,
    g: 170,
    b: 40,
};

/// Focused match foreground: dark for contrast.
pub(super) const SEARCH_FOCUSED_FG: Rgb = Rgb { r: 0, g: 0, b: 0 };

/// Resolve the effective cursor for rendering.
///
/// When mark mode is active (`mark_cursor` is `Some`), the override replaces
/// the terminal cursor's position and shape. Otherwise the extracted terminal
/// cursor is used as-is.
pub(super) fn resolve_cursor(
    content_cursor: &RenderableCursor,
    mark: Option<&MarkCursorOverride>,
) -> RenderableCursor {
    match mark {
        Some(mc) => RenderableCursor {
            line: mc.line,
            column: mc.column,
            shape: mc.shape,
            visible: true,
        },
        None => *content_cursor,
    }
}

/// Resolve per-cell colors with selection highlighting applied.
///
/// Returns `(fg, bg)` for the cell, accounting for:
/// - **Selection inversion**: selected cells swap fg/bg.
/// - **Block cursor exclusion**: the cell under a visible block cursor is not
///   inverted — the cursor overlay handles its own visual.
/// - **INVERSE flag**: cells already inverted by SGR 7 would look identical
///   to unselected normal cells after a naive swap. Falls back to palette
///   defaults to ensure the selection is visible.
/// - **fg==bg reveal**: if inversion produces matching fg/bg (invisible text),
///   falls back to palette defaults — unless the cell has HIDDEN set (SGR 8
///   intentionally hides text, and selection should not reveal it).
#[expect(
    clippy::too_many_arguments,
    reason = "cell, selection, search, cursor, blink, palette are all distinct concerns"
)]
pub(super) fn resolve_cell_colors(
    cell: &RenderableCell,
    sel: Option<&FrameSelection>,
    search: Option<&FrameSearch>,
    cursor: &RenderableCursor,
    cursor_opacity: f32,
    palette: &FramePalette,
) -> (Rgb, Rgb) {
    let col = cell.column.0;
    let row = cell.line;
    let is_wide = cell.flags.contains(CellFlags::WIDE_CHAR);

    // Block cursor cell: at opacity > 0.5 the cursor dominates visually, so
    // skip selection/search inversion. At lower opacity the cursor is fading
    // out and text should revert to normal colors for readability.
    let is_block_cursor_cell = cursor_opacity > 0.5
        && cursor.visible
        && cursor.shape == CursorShape::Block
        && cursor.line == row
        && cursor.column == Column(col);

    // Selection takes priority over search highlighting.
    let selected = !is_block_cursor_cell
        && sel.is_some_and(|s| s.contains(row, col) || (is_wide && s.contains(row, col + 1)));

    if selected {
        // When explicit selection colors are configured, use them directly.
        if let (Some(sfg), Some(sbg)) = (palette.selection_fg, palette.selection_bg) {
            return (sfg, sbg);
        }
        // Fallback: swap fg/bg with INVERSE and visibility guards.
        if cell.flags.contains(CellFlags::INVERSE) {
            return (palette.background, palette.foreground);
        }
        let (sel_fg, sel_bg) = (cell.bg, cell.fg);
        if sel_fg == sel_bg && !cell.flags.contains(CellFlags::HIDDEN) {
            return (palette.background, palette.foreground);
        }
        return (sel_fg, sel_bg);
    }

    // Search match highlighting (below selection in priority).
    if !is_block_cursor_cell {
        if let Some(search) = search {
            match search.cell_match_type(row, col) {
                MatchType::FocusedMatch => return (SEARCH_FOCUSED_FG, SEARCH_FOCUSED_BG),
                MatchType::Match => return (cell.fg, SEARCH_MATCH_BG),
                MatchType::None => {}
            }
        }
    }

    (cell.fg, cell.bg)
}
