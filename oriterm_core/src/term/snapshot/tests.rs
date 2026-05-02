//! Tests for `Term::renderable_content`, `Term::damage`, and
//! `Term::reset_damage` — the snapshot-extraction surface of `Term`.

use std::sync::Arc;

use vte::ansi::Handler;
use vte::ansi::Processor;
use vte::ansi::cursor_icon::CursorIcon;

use crate::effect::VoidEffectSink;
use crate::grid::StableRowIndex;
use crate::image::{ImageData, ImageFormat, ImageId, ImagePlacement, ImageSource, PlacementSizing};
use crate::index::Column;
use crate::term::Term;
use crate::term::renderable::RenderableContent;
use crate::theme::Theme;

fn make_term() -> Term<VoidEffectSink> {
    Term::new(24, 80, 1000, Theme::default(), VoidEffectSink)
}

/// Create a small terminal and clear initial damage.
fn damage_term() -> Term<VoidEffectSink> {
    let mut t = Term::new(6, 10, 100, Theme::default(), VoidEffectSink);
    t.reset_damage();
    t
}

/// Collect damaged line indices from a term.
fn damaged_lines(term: &mut Term<VoidEffectSink>) -> Vec<usize> {
    term.damage().map(|d| d.line).collect()
}

/// Feed raw bytes through the VTE processor.
fn feed(term: &mut impl Handler, bytes: &[u8]) {
    let mut processor: Processor = Processor::new();
    processor.advance(term, bytes);
}

/// Store a 2×2 RGBA test image and place it at the given cell row/col.
fn place_test_image(
    term: &mut Term<VoidEffectSink>,
    stable_row: u64,
    col: usize,
    rows: usize,
    cols: usize,
) -> ImageId {
    let id = term.image_cache_mut().next_image_id();
    let data = ImageData {
        id,
        width: 2,
        height: 2,
        data: Arc::new(vec![255; 16]),
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    };
    term.image_cache_mut().store(data).unwrap();
    term.image_cache_mut().place(ImagePlacement {
        image_id: id,
        placement_id: None,
        source_x: 0,
        source_y: 0,
        source_w: 0,
        source_h: 0,
        cell_col: col,
        cell_row: StableRowIndex(stable_row),
        cols,
        rows,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    });
    id
}

// --- Damage tracking integration (Term::damage / Term::reset_damage) ---

// Basic damage semantics

#[test]
fn damage_write_char_marks_line() {
    let mut t = damage_term();
    feed(&mut t, b"X");

    let dmg: Vec<_> = t.damage().collect();
    assert!(dmg.iter().any(|d| d.line == 0));
    assert!(dmg.iter().all(|d| d.line == 0));
    assert_eq!(dmg[0].left, Column(0));
    assert_eq!(dmg[0].right, Column(0));
}

#[test]
fn damage_drain_clears_marks() {
    let mut t = damage_term();
    feed(&mut t, b"A");

    let first: Vec<_> = t.damage().collect();
    assert!(!first.is_empty(), "first drain should report damage");

    let second: Vec<_> = t.damage().collect();
    assert!(second.is_empty(), "second drain should be empty");
}

#[test]
fn damage_no_changes_empty() {
    let mut t = damage_term();
    let dmg: Vec<_> = t.damage().collect();
    assert!(dmg.is_empty());
}

#[test]
fn damage_scroll_marks_all_dirty() {
    let mut t = damage_term();
    feed(&mut t, b"\r\n\r\n\r\n\r\n\r\n\r\n\r\n");

    let dmg = t.damage();
    assert!(dmg.is_all_dirty());
    let lines: Vec<_> = dmg.collect();
    assert_eq!(lines.len(), 6);
}

// Cursor movement damage

#[test]
fn damage_goto_marks_old_and_new_lines() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[4;6H");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&0), "old cursor line 0 should be damaged");
    assert!(lines.contains(&3), "new cursor line 3 should be damaged");
}

#[test]
fn damage_move_forward() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[2;3H");
    t.reset_damage();

    feed(&mut t, b"\x1b[3C");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&1), "cursor line should be damaged");
    assert!(lines.iter().all(|&l| l == 1));
}

#[test]
fn damage_move_backward() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[2;8H");
    t.reset_damage();

    feed(&mut t, b"\x1b[5D");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&1));
    assert!(lines.iter().all(|&l| l == 1));
}

#[test]
fn damage_move_up() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[4;1H");
    t.reset_damage();

    feed(&mut t, b"\x1b[2A");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&3), "old line 3 should be damaged");
    assert!(lines.contains(&1), "new line 1 should be damaged");
}

#[test]
fn damage_move_down() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[2;1H");
    t.reset_damage();

    feed(&mut t, b"\x1b[3B");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&1), "old line 1 should be damaged");
    assert!(lines.contains(&4), "new line 4 should be damaged");
}

#[test]
fn damage_carriage_return() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[1;6H");
    t.reset_damage();

    feed(&mut t, b"\r");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&0), "CR damages cursor line");
    assert!(lines.iter().all(|&l| l == 0));
}

#[test]
fn damage_linefeed_two_lines() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[3;1H");
    t.reset_damage();

    feed(&mut t, b"\n");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&2), "old line should be damaged");
    assert!(lines.contains(&3), "new line should be damaged");
}

#[test]
fn damage_backspace() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[1;5H");
    t.reset_damage();

    feed(&mut t, b"\x08");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&0));
    assert!(lines.iter().all(|&l| l == 0));
}

#[test]
fn damage_wrapline() {
    let mut t = damage_term();
    feed(&mut t, b"0123456789");
    t.reset_damage();

    feed(&mut t, b"X");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&0), "wrapped-from line should be damaged");
    assert!(lines.contains(&1), "wrapped-to line should be damaged");
}

#[test]
fn damage_reverse_index_scrolls() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[1;4r");
    feed(&mut t, b"\x1b[1;1H");
    t.reset_damage();

    feed(&mut t, b"\x1bM");

    let lines = damaged_lines(&mut t);
    for l in 0..4 {
        assert!(
            lines.contains(&l),
            "line {l} in scroll region should be damaged"
        );
    }
}

#[test]
fn damage_reverse_index_no_scroll() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[3;1H");
    t.reset_damage();

    feed(&mut t, b"\x1bM");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&2), "old line 2 should be damaged");
    assert!(lines.contains(&1), "new line 1 should be damaged");
}

#[test]
fn damage_tab_forward() {
    let mut t = damage_term();
    t.reset_damage();

    feed(&mut t, b"\t");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&0), "tab forward should damage cursor line");
}

#[test]
fn damage_tab_backward() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[1;10H");
    t.reset_damage();

    feed(&mut t, b"\x1b[Z");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&0), "tab backward should damage cursor line");
}

#[test]
fn damage_save_restore_cursor() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[3;5H");
    feed(&mut t, b"\x1b7");

    feed(&mut t, b"\x1b[6;1H");
    t.reset_damage();

    feed(&mut t, b"\x1b8");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&5), "old cursor line 5 should be damaged");
    assert!(
        lines.contains(&2),
        "restored cursor line 2 should be damaged"
    );
}

// Erase operation damage

#[test]
fn damage_erase_chars() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[3;1H");
    t.reset_damage();

    feed(&mut t, b"\x1b[5X");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&2));
    assert!(lines.iter().all(|&l| l == 2));
}

#[test]
fn damage_delete_chars() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[2;3H");
    t.reset_damage();

    feed(&mut t, b"\x1b[3P");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&1));
    assert!(lines.iter().all(|&l| l == 1));
}

#[test]
fn damage_clear_line_all() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[4;5H");
    t.reset_damage();

    feed(&mut t, b"\x1b[2K");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&3));
}

#[test]
fn damage_clear_line_right() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[2;5H");
    t.reset_damage();

    feed(&mut t, b"\x1b[0K");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&1));
}

#[test]
fn damage_clear_line_left() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[3;5H");
    t.reset_damage();

    feed(&mut t, b"\x1b[1K");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&2));
}

#[test]
fn damage_clear_screen_below() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[3;1H");
    t.reset_damage();

    feed(&mut t, b"\x1b[0J");

    let lines = damaged_lines(&mut t);
    for l in 2..6 {
        assert!(lines.contains(&l), "line {l} should be damaged");
    }
}

#[test]
fn damage_clear_screen_above() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[4;1H");
    t.reset_damage();

    feed(&mut t, b"\x1b[1J");

    let lines = damaged_lines(&mut t);
    for l in 0..4 {
        assert!(lines.contains(&l), "line {l} should be damaged");
    }
}

#[test]
fn damage_clear_screen_all() {
    let mut t = damage_term();
    t.reset_damage();

    feed(&mut t, b"\x1b[2J");

    let dmg = t.damage();
    assert!(dmg.is_all_dirty(), "clear screen should mark all dirty");
    drop(dmg);
}

// Scroll operations

#[test]
fn damage_scroll_up_csi() {
    let mut t = damage_term();
    t.reset_damage();

    feed(&mut t, b"\x1b[2S");

    let dmg = t.damage();
    assert!(dmg.is_all_dirty());
    drop(dmg);
}

#[test]
fn damage_scroll_down_csi() {
    let mut t = damage_term();
    t.reset_damage();

    feed(&mut t, b"\x1b[1T");

    let dmg = t.damage();
    assert!(dmg.is_all_dirty());
    drop(dmg);
}

#[test]
fn damage_scroll_up_in_region() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[2;5r");
    t.reset_damage();

    feed(&mut t, b"\x1b[1S");

    let lines = damaged_lines(&mut t);
    for l in 1..5 {
        assert!(
            lines.contains(&l),
            "line {l} in scroll region should be damaged"
        );
    }
    assert!(
        !lines.contains(&0),
        "line 0 above region should not be damaged"
    );
    assert!(
        !lines.contains(&5),
        "line 5 below region should not be damaged"
    );
}

#[test]
fn damage_insert_lines() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[3;1H");
    t.reset_damage();

    feed(&mut t, b"\x1b[2L");

    let lines = damaged_lines(&mut t);
    for l in 2..6 {
        assert!(
            lines.contains(&l),
            "line {l} should be damaged by insert_lines"
        );
    }
}

#[test]
fn damage_delete_lines() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[2;1H");
    t.reset_damage();

    feed(&mut t, b"\x1b[1M");

    let lines = damaged_lines(&mut t);
    for l in 1..6 {
        assert!(
            lines.contains(&l),
            "line {l} should be damaged by delete_lines"
        );
    }
}

// Full damage triggers

#[test]
fn damage_palette_set_color_marks_all_dirty() {
    let mut t = damage_term();

    feed(&mut t, b"\x1b]4;1;rgb:ff/00/00\x1b\\");

    let dmg = t.damage();
    assert!(dmg.is_all_dirty(), "palette change should mark all dirty");
    drop(dmg);
}

#[test]
fn damage_palette_reset_color_marks_all_dirty() {
    let mut t = damage_term();

    feed(&mut t, b"\x1b]104;1\x1b\\");

    let dmg = t.damage();
    assert!(dmg.is_all_dirty(), "palette reset should mark all dirty");
    drop(dmg);
}

#[test]
fn damage_resize_marks_all_dirty() {
    let mut t = damage_term();

    t.grid_mut().dirty_mut().resize(8, 80);

    let dmg = t.damage();
    assert!(dmg.is_all_dirty(), "resize should mark all dirty");
    drop(dmg);
}

#[test]
fn damage_scroll_display_marks_all_dirty() {
    let mut t = damage_term();
    feed(&mut t, b"\r\n\r\n\r\n\r\n\r\n\r\n\r\n\r\n");
    t.reset_damage();

    t.grid_mut().scroll_display(2);

    let dmg = t.damage();
    assert!(dmg.is_all_dirty(), "scroll_display should mark all dirty");
    drop(dmg);
}

// Edge cases

#[test]
fn damage_multiple_writes_same_line_single_entry() {
    let mut t = damage_term();

    feed(&mut t, b"ABCDE");

    let dmg: Vec<_> = t.damage().collect();
    let line0_count = dmg.iter().filter(|d| d.line == 0).count();
    assert_eq!(line0_count, 1, "same line should appear once in damage");
}

#[test]
fn damage_writes_different_lines_separate_entries() {
    let mut t = damage_term();

    feed(&mut t, b"A\x1b[4;1HB");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&0), "line 0 should be damaged");
    assert!(lines.contains(&3), "line 3 should be damaged");
}

#[test]
fn damage_wide_char_marks_line() {
    let mut t = damage_term();

    feed(&mut t, "世".as_bytes());

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&0));
}

#[test]
fn damage_combining_mark_marks_line() {
    let mut t = damage_term();
    feed(&mut t, b"e");
    t.reset_damage();

    feed(&mut t, "\u{0301}".as_bytes());

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&0), "combining mark should damage its line");
}

#[test]
fn damage_insert_blank_chars() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[2;3H");
    t.reset_damage();

    feed(&mut t, b"\x1b[2@");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&1));
}

#[test]
fn damage_newline_cr_plus_lf() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[3;5H");
    t.reset_damage();

    feed(&mut t, b"\r\n");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&2), "CR should damage line 2");
    assert!(lines.contains(&3), "LF should damage line 3");
}

#[test]
fn damage_set_scroll_region_damages_via_goto() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[3;1H");
    t.reset_damage();

    feed(&mut t, b"\x1b[2;5r");

    let lines = damaged_lines(&mut t);
    assert!(lines.contains(&0), "cursor-to-origin damages line 0");
    assert!(lines.contains(&2), "old cursor line 2 should be damaged");
}

// ── Image scrolling (renderable_content image extraction) ──

#[test]
fn image_scrolls_with_display_offset() {
    let mut term = Term::new(4, 10, 10, Theme::default(), VoidEffectSink);
    term.set_cell_dimensions(8, 16);

    place_test_image(&mut term, 0, 0, 1, 2);

    let mut out = RenderableContent::default();
    term.renderable_content_into(&mut out);
    assert_eq!(out.images.len(), 1, "image should be visible");
    assert_eq!(out.images[0].viewport_y, 0.0);

    feed(&mut term, b"\n\n\n\n");
    term.renderable_content_into(&mut out);
    assert!(out.images.is_empty(), "image should scroll out of viewport");

    term.grid_mut().scroll_display(4);
    term.renderable_content_into(&mut out);
    assert_eq!(out.images.len(), 1, "image visible after scroll back");
    assert_eq!(out.images[0].viewport_y, 0.0, "image at top of viewport");
}

#[test]
fn image_partially_above_viewport_has_negative_y() {
    let mut term = Term::new(4, 10, 20, Theme::default(), VoidEffectSink);
    term.set_cell_dimensions(8, 16);

    place_test_image(&mut term, 0, 0, 3, 2);

    feed(&mut term, b"\n\n\n\n\n\n");

    term.grid_mut().scroll_display(2);
    let mut out = RenderableContent::default();
    term.renderable_content_into(&mut out);

    assert_eq!(out.images.len(), 1, "multi-row image partially visible");
    assert!(
        out.images[0].viewport_y < 0.0,
        "image starting above viewport should have negative Y, got {}",
        out.images[0].viewport_y,
    );
    assert_eq!(out.images[0].viewport_y, -16.0);
}

#[test]
fn image_at_viewport_bottom_visible() {
    let mut term = Term::new(4, 10, 10, Theme::default(), VoidEffectSink);
    term.set_cell_dimensions(8, 16);

    place_test_image(&mut term, 3, 0, 1, 2);

    let mut out = RenderableContent::default();
    term.renderable_content_into(&mut out);
    assert_eq!(out.images.len(), 1);
    assert_eq!(out.images[0].viewport_y, 48.0);
}

// ── Resize + snapshot integration ──

#[test]
fn resize_then_snapshot_empty_term() {
    let mut term = make_term();
    term.resize(10, 40, true);
    let snap = term.renderable_content();
    assert_eq!(snap.lines, 10);
    assert_eq!(snap.cols, 40);
    assert_eq!(snap.cells.len(), 10 * 40);
}

#[test]
fn resize_then_snapshot_with_content() {
    let mut term = make_term();
    feed(&mut term, b"hello world\r\nline two\r\nline three");
    term.resize(10, 40, true);
    let snap = term.renderable_content();
    assert_eq!(snap.lines, 10);
    assert_eq!(snap.cols, 40);
    assert_eq!(snap.cells.len(), 10 * 40);
    assert_eq!(snap.cells[0].ch, 'h');
}

#[test]
fn resize_then_snapshot_reuses_buffer() {
    let mut term = make_term();
    feed(&mut term, b"content");
    let mut buf = RenderableContent::default();

    term.renderable_content_into(&mut buf);
    assert_eq!(buf.lines, 24);
    assert_eq!(buf.cols, 80);

    term.resize(10, 40, true);
    term.renderable_content_into(&mut buf);
    assert_eq!(buf.lines, 10);
    assert_eq!(buf.cols, 40);
    assert_eq!(buf.cells.len(), 10 * 40);
}

#[test]
fn resize_shrink_then_snapshot_cursor_in_bounds() {
    let mut term = make_term();
    feed(&mut term, b"\x1b[20;70H");
    assert_eq!(term.grid().cursor().line(), 19);

    term.resize(5, 20, true);
    let snap = term.renderable_content();

    assert!(
        snap.cursor.line < 5,
        "cursor line {} out of bounds",
        snap.cursor.line
    );
    assert!(
        snap.cursor.column.0 < 20,
        "cursor col {} out of bounds",
        snap.cursor.column.0
    );
    assert_eq!(snap.cells.len(), 5 * 20);
}

#[test]
fn resize_grow_then_snapshot_with_scrollback() {
    let mut term = Term::new(5, 10, 100, Theme::default(), VoidEffectSink);
    for i in 0..10 {
        let line = format!("line{i:05}\r\n");
        feed(&mut term, line.as_bytes());
    }
    assert!(term.grid().scrollback().len() > 0);

    term.resize(15, 10, true);
    let snap = term.renderable_content();
    assert_eq!(snap.lines, 15);
    assert_eq!(snap.cols, 10);
    assert_eq!(snap.cells.len(), 15 * 10);
}

#[test]
fn resize_reflow_wrap_then_snapshot() {
    let mut term = Term::new(5, 20, 100, Theme::default(), VoidEffectSink);
    feed(&mut term, b"abcdefghijklmnopqrst");

    term.resize(5, 10, true);
    let snap = term.renderable_content();
    assert_eq!(snap.cols, 10);
    assert_eq!(snap.cells.len(), 5 * 10);
    assert_eq!(snap.cells[0].ch, 'a');
    assert_eq!(snap.cells[10].ch, 'k');
}

#[test]
fn resize_reflow_unwrap_then_snapshot() {
    let mut term = Term::new(5, 10, 100, Theme::default(), VoidEffectSink);
    feed(&mut term, b"abcdefghijklmnopqrst");

    term.resize(5, 20, true);
    let snap = term.renderable_content();
    assert_eq!(snap.cols, 20);
    assert_eq!(snap.cells.len(), 5 * 20);
    assert_eq!(snap.cells[0].ch, 'a');
    assert_eq!(snap.cells[19].ch, 't');
}

#[test]
fn resize_snapshot_damage_is_all_dirty() {
    let mut term = make_term();
    feed(&mut term, b"content");
    term.reset_damage();

    term.resize(10, 40, true);
    let snap = term.renderable_content();
    assert!(snap.all_dirty, "resize should mark all dirty");
}

#[test]
fn resize_snapshot_display_offset_reset() {
    let mut term = Term::new(5, 10, 100, Theme::default(), VoidEffectSink);
    for i in 0..20 {
        let line = format!("line{i:03}\r\n");
        feed(&mut term, line.as_bytes());
    }
    term.grid_mut().scroll_display(5);
    assert!(term.grid().display_offset() > 0);

    term.resize(10, 10, true);
    let snap = term.renderable_content();
    assert_eq!(snap.display_offset, 0, "resize should reset display_offset");
}

// ── OSC 22: Mouse cursor icon flows into renderable snapshot ──

#[test]
fn term_mouse_cursor_icon_flows_into_renderable_snapshot() {
    let mut term = make_term();
    Handler::set_mouse_cursor_icon(&mut term, CursorIcon::Crosshair);

    let rc = term.renderable_content();
    assert_eq!(rc.mouse_cursor_icon, Some(CursorIcon::Crosshair));
}
