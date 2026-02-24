//! Unit tests for IME handling and preedit overlay.

use oriterm_core::{
    CellFlags, Column, CursorShape, RenderableCell, RenderableContent, RenderableCursor, Rgb,
    TermMode,
};

use super::super::redraw::overlay_preedit_cells;

const FG: Rgb = Rgb {
    r: 211,
    g: 215,
    b: 207,
};
const BG: Rgb = Rgb { r: 0, g: 0, b: 0 };

/// Build a renderable content with a grid of spaces and cursor at `(line, col)`.
fn content_with_cursor(
    cols: usize,
    rows: usize,
    cursor_line: usize,
    cursor_col: usize,
) -> RenderableContent {
    let mut cells = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            cells.push(RenderableCell {
                line: row,
                column: Column(col),
                ch: ' ',
                fg: FG,
                bg: BG,
                flags: CellFlags::empty(),
                underline_color: None,
                has_hyperlink: false,
                zerowidth: Vec::new(),
            });
        }
    }
    RenderableContent {
        cells,
        cursor: RenderableCursor {
            line: cursor_line,
            column: Column(cursor_col),
            shape: CursorShape::Block,
            visible: true,
        },
        display_offset: 0,
        stable_row_base: 0,
        mode: TermMode::SHOW_CURSOR,
        all_dirty: true,
        damage: Vec::new(),
    }
}

// --- overlay_preedit_cells ---

#[test]
fn preedit_replaces_cell_at_cursor() {
    let mut content = content_with_cursor(10, 1, 0, 3);
    overlay_preedit_cells("A", &mut content, 10);

    assert_eq!(content.cells[3].ch, 'A');
    assert!(content.cells[3].flags.contains(CellFlags::UNDERLINE));
}

#[test]
fn preedit_hides_cursor() {
    let mut content = content_with_cursor(10, 1, 0, 0);
    assert!(content.cursor.visible);

    overlay_preedit_cells("x", &mut content, 10);
    assert!(!content.cursor.visible);
}

#[test]
fn preedit_wide_char_sets_flags() {
    let mut content = content_with_cursor(10, 1, 0, 0);
    // U+4E2D '中' is a CJK character (display width 2).
    overlay_preedit_cells("中", &mut content, 10);

    assert_eq!(content.cells[0].ch, '中');
    assert!(content.cells[0].flags.contains(CellFlags::WIDE_CHAR));
    assert!(content.cells[0].flags.contains(CellFlags::UNDERLINE));
    assert!(content.cells[1].flags.contains(CellFlags::WIDE_CHAR_SPACER));
}

#[test]
fn preedit_multiple_chars() {
    let mut content = content_with_cursor(10, 1, 0, 2);
    overlay_preedit_cells("AB", &mut content, 10);

    assert_eq!(content.cells[2].ch, 'A');
    assert_eq!(content.cells[3].ch, 'B');
    assert!(content.cells[2].flags.contains(CellFlags::UNDERLINE));
    assert!(content.cells[3].flags.contains(CellFlags::UNDERLINE));
    // Other cells unchanged.
    assert_eq!(content.cells[0].ch, ' ');
    assert_eq!(content.cells[4].ch, ' ');
}

#[test]
fn preedit_clips_at_grid_edge() {
    let mut content = content_with_cursor(4, 1, 0, 3);
    // Cursor at col 3, grid is 4 cols — only 1 cell available.
    overlay_preedit_cells("XY", &mut content, 4);

    assert_eq!(content.cells[3].ch, 'X');
    // 'Y' is clipped (col 4 doesn't exist).
}

#[test]
fn preedit_wide_char_clips_at_edge() {
    let mut content = content_with_cursor(4, 1, 0, 3);
    // Wide char at col 3 needs 2 cells but only 1 available — still placed
    // (the spacer at col 4 is out of bounds, which is handled gracefully).
    overlay_preedit_cells("中", &mut content, 4);

    assert_eq!(content.cells[3].ch, '中');
    assert!(content.cells[3].flags.contains(CellFlags::WIDE_CHAR));
}

#[test]
fn preedit_empty_string_no_change() {
    let mut content = content_with_cursor(10, 1, 0, 0);
    // Empty preedit shouldn't change anything (but cursor is still hidden
    // because overlay_preedit_cells is only called when preedit is non-empty
    // in the actual app code).
    let original_ch = content.cells[0].ch;
    overlay_preedit_cells("", &mut content, 10);

    assert_eq!(content.cells[0].ch, original_ch);
}

#[test]
fn preedit_on_second_row() {
    let mut content = content_with_cursor(10, 3, 1, 5);
    overlay_preedit_cells("Z", &mut content, 10);

    // Row 1, col 5 = index 1*10 + 5 = 15.
    assert_eq!(content.cells[15].ch, 'Z');
    assert!(content.cells[15].flags.contains(CellFlags::UNDERLINE));
}

#[test]
fn preedit_cjk_composition_sequence() {
    let mut content = content_with_cursor(20, 1, 0, 0);
    // Typical CJK composition: two wide characters.
    overlay_preedit_cells("中文", &mut content, 20);

    // First char at cols 0-1.
    assert_eq!(content.cells[0].ch, '中');
    assert!(content.cells[0].flags.contains(CellFlags::WIDE_CHAR));
    assert!(content.cells[1].flags.contains(CellFlags::WIDE_CHAR_SPACER));
    // Second char at cols 2-3.
    assert_eq!(content.cells[2].ch, '文');
    assert!(content.cells[2].flags.contains(CellFlags::WIDE_CHAR));
    assert!(content.cells[3].flags.contains(CellFlags::WIDE_CHAR_SPACER));
}
