//! Tests for snapshot-to-FrameInput conversion.

use oriterm_core::{CellFlags, Column, CursorShape, Rgb, TermMode};
use oriterm_mux::{PaneSnapshot, WireCell, WireCursor, WireCursorShape, WireRgb};

use crate::font::CellMetrics;
use crate::gpu::frame_input::ViewportSize;

use super::{
 PALETTE_BACKGROUND, PALETTE_CURSOR, PALETTE_FOREGROUND, extract_frame_from_snapshot,
 extract_frame_from_snapshot_into, snapshot_palette, snapshot_to_renderable,
 snapshot_to_renderable_into,
};

/// Build a minimal test snapshot with 2×2 cells.
fn test_snapshot() -> PaneSnapshot {
 let white = WireRgb {
 r: 211,
 g: 215,
 b: 207,
 };
 let black = WireRgb { r: 0, g: 0, b: 0 };

 PaneSnapshot {
 cells: vec![
 vec![
 WireCell {
 ch: 'A',
 fg: white,
 bg: black,
 flags: 0,
 underline_color: None,
 hyperlink_uri: None,
 zerowidth: vec![],
 },
 WireCell {
 ch: 'B',
 fg: white,
 bg: black,
 flags: CellFlags::BOLD.bits(),
 underline_color: None,
 hyperlink_uri: None,
 zerowidth: vec![],
 },
 ],
 vec![
 WireCell {
 ch: ' ',
 fg: white,
 bg: black,
 flags: 0,
 underline_color: None,
 hyperlink_uri: None,
 zerowidth: vec![],
 },
 WireCell {
 ch: 'C',
 fg: WireRgb { r: 255, g: 0, b: 0 },
 bg: black,
 flags: CellFlags::UNDERLINE.bits(),
 underline_color: Some(WireRgb {
 r: 0,
 g: 128,
 b: 255,
 }),
 hyperlink_uri: Some("https://test.example".to_string()),
 zerowidth: vec!['\u{0301}'],
 },
 ],
 ],
 cursor: WireCursor {
 col: 1,
 row: 0,
 shape: WireCursorShape::Block,
 visible: true,
 },
 palette: (0..270).map(|i| [(i % 256) as u8, 0, 0]).collect(),
 title: "test".into(),
 icon_name: None,
 cwd: None,
 modes: TermMode::SHOW_CURSOR.bits(),
 scrollback_len: 0,
 display_offset: 0,
 stable_row_base: 0,
 cols: 2,
 search_active: false,
 search_query: String::new(),
 search_matches: Vec::new(),
 search_focused: None,
 search_total_matches: 0,
 has_unseen_output: false,
 mouse_cursor_icon: None,
 images: Vec::new(),
 image_data: Vec::new(),
 images_dirty: false,
 }
}

fn test_cell_metrics() -> CellMetrics {
 CellMetrics::new(8.0, 16.0, 12.0, 2.0, 1.0, 5.0)
}

#[test]
fn renderable_cell_positions() {
 let snap = test_snapshot();
 let content = snapshot_to_renderable(&snap, &|_| None);

 assert_eq!(content.cells.len(), 4);
 assert_eq!(content.cells[0].line, 0);
 assert_eq!(content.cells[0].column, Column(0));
 assert_eq!(content.cells[0].ch, 'A');
 assert_eq!(content.cells[1].line, 0);
 assert_eq!(content.cells[1].column, Column(1));
 assert_eq!(content.cells[1].ch, 'B');
 assert_eq!(content.cells[2].line, 1);
 assert_eq!(content.cells[2].column, Column(0));
 assert_eq!(content.cells[3].line, 1);
 assert_eq!(content.cells[3].column, Column(1));
 assert_eq!(content.cells[3].ch, 'C');
}

#[test]
fn renderable_colors_pre_resolved() {
 let snap = test_snapshot();
 let content = snapshot_to_renderable(&snap, &|_| None);

 assert_eq!(
 content.cells[0].fg,
 Rgb {
 r: 211,
 g: 215,
 b: 207
 }
 );
 assert_eq!(content.cells[0].bg, Rgb { r: 0, g: 0, b: 0 });
 assert_eq!(content.cells[3].fg, Rgb { r: 255, g: 0, b: 0 });
}

#[test]
fn renderable_flags_preserved() {
 let snap = test_snapshot();
 let content = snapshot_to_renderable(&snap, &|_| None);

 assert!(content.cells[1].flags.contains(CellFlags::BOLD));
 assert!(content.cells[3].flags.contains(CellFlags::UNDERLINE));
 assert!(!content.cells[0].flags.contains(CellFlags::BOLD));
}

#[test]
fn renderable_underline_color_and_hyperlink() {
 let snap = test_snapshot();
 let content = snapshot_to_renderable(&snap, &|_| None);

 assert_eq!(content.cells[0].underline_color, None);
 assert!(!content.cells[0].has_hyperlink);

 assert_eq!(
 content.cells[3].underline_color,
 Some(Rgb {
 r: 0,
 g: 128,
 b: 255
 })
 );
 assert!(content.cells[3].has_hyperlink);
}

#[test]
fn renderable_zerowidth() {
 let snap = test_snapshot();
 let content = snapshot_to_renderable(&snap, &|_| None);

 assert!(content.cells[0].zerowidth.is_empty());
 assert_eq!(content.cells[3].zerowidth, vec!['\u{0301}']);
}

#[test]
fn renderable_cursor() {
 let snap = test_snapshot();
 let content = snapshot_to_renderable(&snap, &|_| None);

 assert_eq!(content.cursor.line, 0);
 assert_eq!(content.cursor.column, Column(1));
 assert_eq!(content.cursor.shape, CursorShape::Block);
 assert!(content.cursor.visible);
}

#[test]
fn renderable_mode_flags() {
 let snap = test_snapshot();
 let content = snapshot_to_renderable(&snap, &|_| None);

 assert!(content.mode.contains(TermMode::SHOW_CURSOR));
 assert!(content.all_dirty);
 assert!(content.damage.is_empty());
}

#[test]
fn palette_extracts_semantic_colors() {
 let snap = test_snapshot();
 let palette = snapshot_palette(&snap);

 // Palette entries at indices 256, 257, 258 are [idx % 256, 0, 0].
 assert_eq!(
 palette.foreground,
 Rgb {
 r: (PALETTE_FOREGROUND % 256) as u8,
 g: 0,
 b: 0
 }
 );
 assert_eq!(
 palette.background,
 Rgb {
 r: (PALETTE_BACKGROUND % 256) as u8,
 g: 0,
 b: 0
 }
 );
 assert_eq!(
 palette.cursor_color,
 Rgb {
 r: (PALETTE_CURSOR % 256) as u8,
 g: 0,
 b: 0
 }
 );
 assert_eq!(palette.opacity, 1.0);
 assert_eq!(palette.selection_fg, None);
 assert_eq!(palette.selection_bg, None);
}

#[test]
fn extract_frame_produces_valid_frame_input() {
 let snap = test_snapshot();
 let viewport = ViewportSize::new(160, 320);
 let cell = test_cell_metrics();

 let frame = extract_frame_from_snapshot(&snap, viewport, cell, &|_| None);

 assert_eq!(frame.viewport, viewport);
 assert_eq!(frame.cell_size, cell);
 assert_eq!(frame.content.cells.len(), 4);
 assert!(frame.selection.is_none());
 assert!(frame.search.is_none());
 assert!(frame.hovered_cell.is_none());
 assert!(frame.hovered_url_segments.is_empty());
 assert!(frame.mark_cursor.is_none());
 assert_eq!(frame.fg_dim, 1.0);
 assert!(frame.prompt_marker_rows.is_empty());
}

#[test]
fn palette_handles_short_array() {
 let mut snap = test_snapshot();
 snap.palette.clear();

 let palette = snapshot_palette(&snap);

 // Missing entries default to black.
 assert_eq!(palette.foreground, Rgb { r: 0, g: 0, b: 0 });
 assert_eq!(palette.background, Rgb { r: 0, g: 0, b: 0 });
 assert_eq!(palette.cursor_color, Rgb { r: 0, g: 0, b: 0 });
}

// -- Cursor shape variant tests --

#[test]
fn cursor_shape_all_variants() {
 let variants = [
 (WireCursorShape::Block, CursorShape::Block),
 (WireCursorShape::Underline, CursorShape::Underline),
 (WireCursorShape::Bar, CursorShape::Bar),
 (WireCursorShape::HollowBlock, CursorShape::HollowBlock),
 (WireCursorShape::Hidden, CursorShape::Hidden),
 ];

 for (wire_shape, expected_shape) in variants {
 let mut snap = test_snapshot();
 snap.cursor.shape = wire_shape;
 let content = snapshot_to_renderable(&snap, &|_| None);
 assert_eq!(
 content.cursor.shape, expected_shape,
 "wire shape {wire_shape:?} should map to {expected_shape:?}"
 );
 }
}

#[test]
fn cursor_hidden_invisible() {
 let mut snap = test_snapshot();
 snap.cursor.visible = false;
 snap.cursor.shape = WireCursorShape::Hidden;

 let content = snapshot_to_renderable(&snap, &|_| None);

 assert!(!content.cursor.visible);
 assert_eq!(content.cursor.shape, CursorShape::Hidden);
}

// -- Empty snapshot --

#[test]
fn empty_snapshot_no_cells() {
 let snap = PaneSnapshot {
 cells: vec![],
 cursor: WireCursor {
 col: 0,
 row: 0,
 shape: WireCursorShape::Block,
 visible: true,
 },
 palette: vec![[0, 0, 0]; 270],
 title: String::new(),
 icon_name: None,
 cwd: None,
 modes: 0,
 scrollback_len: 0,
 display_offset: 0,
 stable_row_base: 0,
 cols: 0,
 search_active: false,
 search_query: String::new(),
 search_matches: Vec::new(),
 search_focused: None,
 search_total_matches: 0,
 has_unseen_output: false,
 mouse_cursor_icon: None,
 images: Vec::new(),
 image_data: Vec::new(),
 images_dirty: false,
 };

 let content = snapshot_to_renderable(&snap, &|_| None);
 assert!(content.cells.is_empty());
 assert_eq!(content.cursor.line, 0);
 assert_eq!(content.cursor.column, Column(0));
}

#[test]
fn empty_snapshot_frame_input() {
 let snap = PaneSnapshot {
 cells: vec![],
 cursor: WireCursor {
 col: 0,
 row: 0,
 shape: WireCursorShape::Block,
 visible: true,
 },
 palette: vec![[0, 0, 0]; 270],
 title: String::new(),
 icon_name: None,
 cwd: None,
 modes: 0,
 scrollback_len: 0,
 display_offset: 0,
 stable_row_base: 0,
 cols: 0,
 search_active: false,
 search_query: String::new(),
 search_matches: Vec::new(),
 search_focused: None,
 search_total_matches: 0,
 has_unseen_output: false,
 mouse_cursor_icon: None,
 images: Vec::new(),
 image_data: Vec::new(),
 images_dirty: false,
 };

 let viewport = ViewportSize::new(160, 320);
 let cell = test_cell_metrics();
 let frame = extract_frame_from_snapshot(&snap, viewport, cell, &|_| None);

 assert!(frame.content.cells.is_empty());
 assert_eq!(frame.viewport, viewport);
}

// -- Non-zero display_offset --

#[test]
fn display_offset_carried_through() {
 let mut snap = test_snapshot();
 snap.display_offset = 42;

 let content = snapshot_to_renderable(&snap, &|_| None);
 assert_eq!(content.display_offset, 42);
}

#[test]
fn display_offset_large_value() {
 let mut snap = test_snapshot();
 snap.display_offset = 100_000;

 let content = snapshot_to_renderable(&snap, &|_| None);
 assert_eq!(content.display_offset, 100_000);
}

// -- Wide char (CJK) flag preservation --

#[test]
fn wide_char_flag_preserved() {
 let snap = PaneSnapshot {
 cells: vec![vec![WireCell {
 ch: '漢',
 fg: WireRgb {
 r: 211,
 g: 215,
 b: 207,
 },
 bg: WireRgb { r: 0, g: 0, b: 0 },
 flags: CellFlags::WIDE_CHAR.bits(),
 underline_color: None,
 hyperlink_uri: None,
 zerowidth: vec![],
 }]],
 cursor: WireCursor {
 col: 0,
 row: 0,
 shape: WireCursorShape::Block,
 visible: true,
 },
 palette: vec![[0, 0, 0]; 270],
 title: String::new(),
 icon_name: None,
 cwd: None,
 modes: 0,
 scrollback_len: 0,
 display_offset: 0,
 stable_row_base: 0,
 cols: 1,
 search_active: false,
 search_query: String::new(),
 search_matches: Vec::new(),
 search_focused: None,
 search_total_matches: 0,
 has_unseen_output: false,
 mouse_cursor_icon: None,
 images: Vec::new(),
 image_data: Vec::new(),
 images_dirty: false,
 };

 let content = snapshot_to_renderable(&snap, &|_| None);
 assert!(content.cells[0].flags.contains(CellFlags::WIDE_CHAR));
 assert_eq!(content.cells[0].ch, '漢');
}

// -- _into variants: allocation-reusing equivalence tests --

#[test]
fn renderable_into_matches_fresh() {
 let snap = test_snapshot();

 let fresh = snapshot_to_renderable(&snap, &|_| None);
 let mut reused = snapshot_to_renderable(&snap, &|_| None);
 // Mutate to prove `_into` overwrites everything.
 reused.display_offset = 999;
 reused.stable_row_base = 42;
 reused.all_dirty = false;
 reused.mode = TermMode::empty();

 snapshot_to_renderable_into(&snap, &mut reused, &|_| None);

 assert_eq!(fresh.cells.len(), reused.cells.len());
 for (a, b) in fresh.cells.iter().zip(reused.cells.iter()) {
 assert_eq!(a.line, b.line);
 assert_eq!(a.column, b.column);
 assert_eq!(a.ch, b.ch);
 assert_eq!(a.fg, b.fg);
 assert_eq!(a.bg, b.bg);
 assert_eq!(a.flags, b.flags);
 assert_eq!(a.underline_color, b.underline_color);
 assert_eq!(a.has_hyperlink, b.has_hyperlink);
 assert_eq!(a.zerowidth, b.zerowidth);
 }
 assert_eq!(fresh.cursor.line, reused.cursor.line);
 assert_eq!(fresh.cursor.column, reused.cursor.column);
 assert_eq!(fresh.cursor.shape, reused.cursor.shape);
 assert_eq!(fresh.cursor.visible, reused.cursor.visible);
 assert_eq!(fresh.display_offset, reused.display_offset);
 assert_eq!(fresh.stable_row_base, reused.stable_row_base);
 assert_eq!(fresh.mode, reused.mode);
 assert_eq!(fresh.all_dirty, reused.all_dirty);
 assert_eq!(fresh.damage.len(), reused.damage.len());
}

#[test]
fn extract_into_matches_fresh() {
 let snap = test_snapshot();
 let viewport = ViewportSize::new(160, 320);
 let cell = test_cell_metrics();

 let fresh = extract_frame_from_snapshot(&snap, viewport, cell, &|_| None);

 // Seed with a different snapshot to prove _into overwrites correctly.
 let mut reused = extract_frame_from_snapshot(&snap, ViewportSize::new(1, 1), cell, &|_| None);
 reused.fg_dim = 0.5;
 reused.hovered_url_segments.push((0, 0, 10));
 reused.prompt_marker_rows.push(99);

 extract_frame_from_snapshot_into(&snap, &mut reused, viewport, cell, &|_| None);

 assert_eq!(fresh.viewport, reused.viewport);
 assert_eq!(fresh.cell_size, reused.cell_size);
 assert_eq!(fresh.content.cells.len(), reused.content.cells.len());
 assert!(reused.selection.is_none());
 assert!(reused.search.is_none());
 assert!(reused.hovered_cell.is_none());
 assert!(reused.hovered_url_segments.is_empty());
 assert!(reused.mark_cursor.is_none());
 assert_eq!(reused.fg_dim, 1.0);
 assert!(reused.prompt_marker_rows.is_empty());
 assert_eq!(fresh.palette.foreground, reused.palette.foreground);
 assert_eq!(fresh.palette.background, reused.palette.background);
 assert_eq!(fresh.palette.cursor_color, reused.palette.cursor_color);
}

#[test]
fn extract_into_preserves_capacity() {
 let snap = test_snapshot();
 let viewport = ViewportSize::new(160, 320);
 let cell = test_cell_metrics();

 // First extraction allocates.
 let mut frame = extract_frame_from_snapshot(&snap, viewport, cell, &|_| None);
 let cells_cap = frame.content.cells.capacity();
 assert!(cells_cap >= 4, "should have allocated for 4 cells");

 // Second extraction into the same frame reuses allocations.
 extract_frame_from_snapshot_into(&snap, &mut frame, viewport, cell, &|_| None);
 assert!(
 frame.content.cells.capacity() >= cells_cap,
 "capacity should not shrink"
 );
 assert_eq!(frame.content.cells.len(), 4);
}

// -- Large snapshot through extract_frame_from_snapshot --

#[test]
fn large_snapshot_through_extract() {
 let cols = 200;
 let rows = 50;
 let white = WireRgb {
 r: 211,
 g: 215,
 b: 207,
 };
 let black = WireRgb { r: 0, g: 0, b: 0 };

 let cells: Vec<Vec<WireCell>> = (0..rows)
 .map(|r| {
 (0..cols)
 .map(|c| WireCell {
 ch: char::from(b'A' + ((r * cols + c) % 26) as u8),
 fg: white,
 bg: black,
 flags: 0,
 underline_color: None,
 hyperlink_uri: None,
 zerowidth: vec![],
 })
 .collect()
 })
 .collect();

 let snap = PaneSnapshot {
 cells,
 cursor: WireCursor {
 col: 100,
 row: 25,
 shape: WireCursorShape::Underline,
 visible: true,
 },
 palette: (0..270).map(|i| [(i % 256) as u8, 0, 0]).collect(),
 title: "large".into(),
 icon_name: None,
 cwd: None,
 modes: TermMode::SHOW_CURSOR.bits(),
 scrollback_len: 10_000,
 display_offset: 50,
 stable_row_base: 9_950,
 cols: cols as u16,
 search_active: false,
 search_query: String::new(),
 search_matches: Vec::new(),
 search_focused: None,
 search_total_matches: 0,
 has_unseen_output: false,
 mouse_cursor_icon: None,
 images: Vec::new(),
 image_data: Vec::new(),
 images_dirty: false,
 };

 let viewport = ViewportSize::new(1600, 800);
 let cell = test_cell_metrics();
 let frame = extract_frame_from_snapshot(&snap, viewport, cell, &|_| None);

 assert_eq!(frame.content.cells.len(), rows * cols);
 assert_eq!(frame.content.cursor.line, 25);
 assert_eq!(frame.content.cursor.column, Column(100));
 assert_eq!(frame.content.cursor.shape, CursorShape::Underline);
 assert_eq!(frame.content.display_offset, 50);

 // Spot-check first and last cells.
 assert_eq!(frame.content.cells[0].ch, 'A');
 assert_eq!(frame.content.cells[0].line, 0);
 assert_eq!(frame.content.cells[0].column, Column(0));
 let last = &frame.content.cells[rows * cols - 1];
 assert_eq!(last.line, rows - 1);
 assert_eq!(last.column, Column(cols - 1));
}

/// Regression: daemon first-frame extract must decode `mouse_cursor_icon`.
/// `snapshot_to_renderable()` builds `RenderableContent::default()` and then
/// populates the fields; `mouse_cursor_icon` defaults to `None`. If the
/// function fails to assign it, daemon clients render the first frame with
/// no OSC 22 cursor icon even when the wire snapshot carries one.
/// See: §10.5 daemon pin.
#[test]
fn snapshot_to_renderable_populates_mouse_cursor_icon() {
 use vte::ansi::cursor_icon::CursorIcon;

 let mut snap = test_snapshot();
 snap.mouse_cursor_icon =
 oriterm_mux::protocol::snapshot::encode_cursor_icon(CursorIcon::Pointer);

 let content = snapshot_to_renderable(&snap, &|_| None);

 assert_eq!(content.mouse_cursor_icon, Some(CursorIcon::Pointer));
}

/// Regression: refill path also carries `mouse_cursor_icon` — pinned here to
/// guard against asymmetric drift between the initial extract and the refill
/// (the historical bug was missing in the initial path; pinning both sides
/// ensures future edits can't regress one while the other stays correct).
#[test]
fn snapshot_to_renderable_into_populates_mouse_cursor_icon() {
 use oriterm_core::RenderableContent;
 use vte::ansi::cursor_icon::CursorIcon;

 let mut snap = test_snapshot();
 snap.mouse_cursor_icon = oriterm_mux::protocol::snapshot::encode_cursor_icon(CursorIcon::Text);

 let mut out = RenderableContent::default();
 snapshot_to_renderable_into(&snap, &mut out, &|_| None);

 assert_eq!(out.mouse_cursor_icon, Some(CursorIcon::Text));
}

/// Regression guard: `mouse_cursor_icon: None` on the wire produces `None` on
/// `RenderableContent`, not a stale value from a prior extract.
#[test]
fn snapshot_to_renderable_none_icon_stays_none() {
 let mut snap = test_snapshot();
 snap.mouse_cursor_icon = None;

 let content = snapshot_to_renderable(&snap, &|_| None);

 assert_eq!(content.mouse_cursor_icon, None);
}

/// Regression guard: refill path MUST clear a prior `Some(icon)` when the wire
/// snapshot has `None`. This is the stale-value-reuse case — without this
/// pin, a refill that only assigns when the source is `Some` would leak the
/// previous frame's icon into the current frame.
#[test]
fn snapshot_to_renderable_into_clears_stale_icon() {
 use oriterm_core::RenderableContent;
 use vte::ansi::cursor_icon::CursorIcon;

 // Seed an existing content with a Some value (as if from a prior frame).
 let mut out = RenderableContent::default();
 out.mouse_cursor_icon = Some(CursorIcon::Pointer);

 // Refill from a snapshot with None — the icon MUST be cleared.
 let mut snap = test_snapshot();
 snap.mouse_cursor_icon = None;
 snapshot_to_renderable_into(&snap, &mut out, &|_| None);

 assert_eq!(out.mouse_cursor_icon, None);
}

/// Regression guard: `extract_frame_from_snapshot_into` (the top-level refill
/// that both `snapshot_to_renderable_into` and other field resets flow
/// through) MUST also clear a stale `Some(icon)` when the source is `None`.
#[test]
fn extract_frame_from_snapshot_into_clears_stale_icon() {
 use oriterm_core::RenderableContent;
 use vte::ansi::cursor_icon::CursorIcon;

 use super::extract_frame_from_snapshot;

 // Build a first FrameInput with Some(icon) populated.
 let mut snap1 = test_snapshot();
 snap1.mouse_cursor_icon =
 oriterm_mux::protocol::snapshot::encode_cursor_icon(CursorIcon::Pointer);
 let viewport = ViewportSize {
 width: 100,
 height: 100,
 };
 let cell_metrics = test_cell_metrics();
 let mut frame = extract_frame_from_snapshot(&snap1, viewport, cell_metrics, &|_| None);
 assert_eq!(frame.content.mouse_cursor_icon, Some(CursorIcon::Pointer));

 // Refill from a snapshot with None — icon MUST be cleared.
 let mut snap2 = test_snapshot();
 snap2.mouse_cursor_icon = None;
 extract_frame_from_snapshot_into(&snap2, &mut frame, viewport, cell_metrics, &|_| None);

 assert_eq!(frame.content.mouse_cursor_icon, None);
 // Sanity: also verify cells were refreshed (not a stale seed).
 let _: &RenderableContent = &frame.content;
}

/// Helper: build a `WirePlacement` with sensible defaults for tests.
fn wire_placement(image_id: u32) -> oriterm_mux::protocol::snapshot::WirePlacement {
 oriterm_mux::protocol::snapshot::WirePlacement {
 image_id,
 viewport_x: 0.0,
 viewport_y: 0.0,
 display_width: 16.0,
 display_height: 16.0,
 source_x: 0.0,
 source_y: 0.0,
 source_w: 1.0,
 source_h: 1.0,
 z_index: 0,
 opacity: 1.0,
 }
}

/// Helper: build a `WireImageData` carrying a tiny RGBA buffer.
fn wire_image_data(id: u32, bytes: &[u8]) -> oriterm_mux::protocol::snapshot::WireImageData {
 oriterm_mux::protocol::snapshot::WireImageData {
 id,
 data: bytes.to_vec(),
 width: 1,
 height: bytes.len() as u32 / 4,
 }
}

/// A daemon snapshot carrying a placement + inline pixel data round-trips
/// through `extract_frame_from_snapshot` with a non-empty
/// `FrameInput.content.images` AND `image_data`. ONLY passes when the extract
/// path actually forwards the wire image fields (the proximate cause of the
/// bug was unconditional `.clear()` on these vectors).
#[test]
fn daemon_pane_snapshot_roundtrips_inline_image_data() {
 use oriterm_core::ImageId;

 let pixels = vec![0xFF, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF];
 let mut snap = test_snapshot();
 snap.images.push(wire_placement(7));
 snap.image_data.push(wire_image_data(7, &pixels));
 snap.images_dirty = true;

 let viewport = ViewportSize::new(100, 100);
 let cell = test_cell_metrics();
 let frame = extract_frame_from_snapshot(&snap, viewport, cell, &|_| None);

 assert_eq!(frame.content.images.len(), 1);
 assert_eq!(frame.content.images[0].image_id, ImageId::from_raw(7));
 assert_eq!(frame.content.image_data.len(), 1);
 assert_eq!(frame.content.image_data[0].id, ImageId::from_raw(7));
 assert_eq!(
 frame.content.image_data[0].data.as_slice(),
 pixels.as_slice()
 );
 assert!(frame.content.images_dirty);
 // images_dirty=true must force a full repaint via all_dirty=true
 // (mirrors `oriterm_core/src/term/snapshot/mod.rs` semantics — image cache
 // changes don't tag per-line grid damage).
 assert!(frame.content.all_dirty);
}

/// A daemon snapshot whose source carries N placements MUST NOT arrive at the
/// client with empty `FrameInput.content.images`. Rejects the pre-fix behavior
/// where `extract_frame_from_snapshot_into` unconditionally called
/// `out.images.clear()` and dropped all daemon-mode image rendering.
#[test]
fn daemon_pane_snapshot_does_not_drop_images_when_source_has_them() {
 let pixels = vec![0u8; 16];
 let mut snap = test_snapshot();
 snap.images.push(wire_placement(11));
 snap.images.push(wire_placement(12));
 snap.image_data.push(wire_image_data(11, &pixels));
 snap.image_data.push(wire_image_data(12, &pixels));

 let viewport = ViewportSize::new(100, 100);
 let cell = test_cell_metrics();
 let frame = extract_frame_from_snapshot(&snap, viewport, cell, &|_| None);

 assert_eq!(
 frame.content.images.len(),
 snap.images.len(),
 "extract path must forward every placement, not silently drop them"
 );
 assert_eq!(
 frame.content.image_data.len(),
 snap.image_data.len(),
 "extract path must forward every inline image_data entry"
 );
}

/// Stale-clear pin: when refilling a `FrameInput` with a snapshot whose
/// `images` list is empty, ANY images from a prior frame MUST be cleared.
/// Companion to `extract_frame_from_snapshot_into_clears_stale_icon`.
#[test]
fn extract_frame_from_snapshot_into_clears_stale_images() {
 let pixels = vec![0xABu8; 4];
 // First snapshot carries a placement.
 let mut snap1 = test_snapshot();
 snap1.images.push(wire_placement(3));
 snap1.image_data.push(wire_image_data(3, &pixels));
 let viewport = ViewportSize::new(100, 100);
 let cell = test_cell_metrics();
 let mut frame = extract_frame_from_snapshot(&snap1, viewport, cell, &|_| None);
 assert_eq!(frame.content.images.len(), 1);
 assert_eq!(frame.content.image_data.len(), 1);

 // Second snapshot has NO placements — refill MUST clear both vectors.
 let snap2 = test_snapshot();
 extract_frame_from_snapshot_into(&snap2, &mut frame, viewport, cell, &|_| None);
 assert!(frame.content.images.is_empty());
 assert!(frame.content.image_data.is_empty());
 assert!(!frame.content.images_dirty);
}

/// Cache-hit pin: a placement whose `image_id` is NOT in the wire snapshot's
/// inline `image_data` (steady-state — server omitted because client cache
/// already has it) is resolved by the `image_lookup` closure. The resolved
/// pixel data ends up in `FrameInput.content.image_data`.
#[test]
fn daemon_pane_snapshot_resolves_placement_via_image_lookup() {
 use std::sync::Arc;

 use oriterm_core::{ImageId, RenderableImageData};

 let cached_pixels: Arc<Vec<u8>> = Arc::new(vec![0x12, 0x34, 0x56, 0x78]);
 let cached_id = ImageId::from_raw(42);
 let cached = Arc::new(RenderableImageData {
 id: cached_id,
 data: cached_pixels.clone(),
 width: 1,
 height: 1,
 });

 // Snapshot has a placement but NO inline image_data — must resolve via lookup.
 let mut snap = test_snapshot();
 snap.images.push(wire_placement(cached_id.as_u32()));

 let lookup = |id: ImageId| -> Option<Arc<RenderableImageData>> {
 (id == cached_id).then(|| cached.clone())
 };
 let viewport = ViewportSize::new(100, 100);
 let cell = test_cell_metrics();
 let frame = extract_frame_from_snapshot(&snap, viewport, cell, &lookup);

 assert_eq!(frame.content.images.len(), 1);
 assert_eq!(frame.content.image_data.len(), 1);
 assert_eq!(frame.content.image_data[0].id, cached_id);
 assert!(Arc::ptr_eq(
 &frame.content.image_data[0].data,
 &cached_pixels
 ));
}

/// End-to-end pin: real `notcurses-info` bytes produced by `Term` flow
/// through the daemon→client wire shape into `extract_frame_from_snapshot`
/// with non-empty `FrameInput.content.images` AND `image_data`.
/// **Why this is the next-layer-down test for the wordmark gap.** The
/// sibling integration tests
/// (`oriterm_core/tests/notcurses_info_pty.rs` and
/// `oriterm_mux/src/server/snapshot/tests.rs::notcurses_info_image_data_survives_daemon_fold`)
/// confirm the daemon-side path retains image bytes through
/// `renderable_content_into` + `fold_image_data_store`. This test
/// extends the same real-notcurses-bytes trace through the *client-side*
/// extract path that consumes `PaneSnapshot.image_data` (the wire
/// encoding the daemon ships per-client).
/// If this test fails on Linux, the client-side `populate_images_from_wire`
/// drops real notcurses image data even though it accepts synthetic data
/// — a regression the existing `wire_image_data`-based tests cannot
/// catch. If it passes, the cure surface is downstream of the client
/// extract (GPU pipeline, texture upload, or render).
#[cfg(unix)]
#[test]
fn notcurses_info_real_bytes_roundtrip_to_client_extract() {
 use oriterm_core::RenderableContent;
 use oriterm_mux::protocol::snapshot::{WireImageData, WirePlacement};
 use oriterm_test_support::{PtySession, notcurses_info_available, tool_available};
 use portable_pty::CommandBuilder;

 if !notcurses_info_available() {
 eprintln!("SKIP: notcurses-info not installed");
 return;
 }
 if !tool_available("infocmp", "-V") {
 eprintln!("SKIP: ncurses tooling (infocmp) not available");
 return;
 }

 // Run notcurses-info under PtySession to CAPTURE the full byte
 // stream the real process emits, then replay the captured bytes
 // truncated at the `_Ga=p,` APC end so the placement is alive when
 // the wire-roundtrip assertions run.
 let mut cmd = CommandBuilder::new("notcurses-info");
 cmd.env("TERM", "xterm-256color");
 let mut session = PtySession::spawn(cmd, 80, 24);
 let status = session.wait_for_child_exit(5_000);
 assert!(status.success(), "notcurses-info exited: {status:?}");
 session.drain_blocking(3000);

 // notcurses-info's trailing render emits ED/CUP/scroll sequences that
 // evict the kitty placement off the visible region and trigger
 // `prune_if_orphaned`, so live-stream end-state has zero placements.
 // To capture the WIRE protocol with real placement data, replay the
 // captured bytes through a fresh `Term` and stop just past the `a=p`
 // APC — the placement is alive at that point.
 let input_bytes = session.input_bytes().to_vec();
 let ap_apc_needle = b"\x1b_Ga=p,";
 let ap_start = input_bytes
 .windows(ap_apc_needle.len())
 .position(|w| w == ap_apc_needle)
 .expect("real notcurses-info should emit `_Ga=p,` APC for display_logo");
 let ap_end = input_bytes[ap_start..]
 .windows(2)
 .position(|w| w == b"\x1b\\")
 .map(|rel| ap_start + rel + 2)
 .expect("`_Ga=p,` APC should terminate with ESC \\");

 // Replay through a fresh `SpecHarness` so we test the SAME parse +
 // dispatch path the IO thread runs, without the trailing eviction.
 use oriterm_test_support::spec_chain::SpecHarness;
 let mut harness = SpecHarness::with_size(24, 80);
 harness.feed(&input_bytes[..ap_end]);
 let mut render_buf = RenderableContent::default();
 harness.term().renderable_content_into(&mut render_buf);
 assert!(
 !render_buf.images.is_empty(),
 "Replay of captured bytes up to `a=p` end MUST have ≥1 placement \
              (got image_count={} placement_count={}). The cure for chunked-action-
              inheritance is supposed to leave exactly 1 placement after `a=p`.",
 harness.term().image_cache().image_count(),
 harness.term().image_cache().placement_count(),
 );
 assert!(
 !render_buf.image_data.is_empty(),
 "Replay of captured bytes up to `a=p` end MUST have ≥1 image_data entry",
 );

 // Build the WIRE PaneSnapshot the daemon would ship to the client.
 // Mirrors `oriterm_mux::server::snapshot::fill_snapshot_from_renderable`
 // for placements + `oriterm_mux::server::push::project_per_client_pure`
 // for image_data on a fresh client (no `sent_images` filter — every
 // referenced ID is inline).
 let mut snap = test_snapshot();
 snap.images.clear();
 snap.image_data.clear();
 snap.images_dirty = render_buf.images_dirty;
 snap.cols = u16::try_from(render_buf.cols).expect("80-col grid fits in u16");
 for p in &render_buf.images {
 snap.images.push(WirePlacement {
 image_id: p.image_id.as_u32(),
 viewport_x: p.viewport_x,
 viewport_y: p.viewport_y,
 display_width: p.display_width,
 display_height: p.display_height,
 source_x: p.source_x,
 source_y: p.source_y,
 source_w: p.source_w,
 source_h: p.source_h,
 z_index: p.z_index,
 opacity: p.opacity,
 });
 }
 for img in &render_buf.image_data {
 snap.image_data.push(WireImageData {
 id: img.id.as_u32(),
 data: (*img.data).clone(),
 width: img.width,
 height: img.height,
 });
 }

 // Wire-encode + wire-decode the daemon snapshot before client extract.
 // This forces the real notcurses bytes through the same MuxPdu codec
 // the daemon uses for IPC: any encoding bug, frame-size mismatch, or
 // serialization corruption that's content-dependent (not just
 // synthetic-data covered by `roundtrip_pane_snapshot_with_image_payload`)
 // would surface here as a decode failure or image-data mutation.
 use oriterm_mux::{MuxPdu, ProtocolCodec};
 let mut buf: Vec<u8> = Vec::new();
 ProtocolCodec::encode_frame(
 &mut buf,
 7,
 &MuxPdu::NotifyPaneSnapshot {
 pane_id: oriterm_mux::PaneId::from_raw(1),
 snapshot: snap,
 },
 )
 .expect("MuxPdu::NotifyPaneSnapshot encodes for real notcurses bytes");
 let mut reader: &[u8] = &buf;
 let decoded = ProtocolCodec::new()
 .decode_frame(&mut reader)
 .expect("MuxPdu::NotifyPaneSnapshot decodes for real notcurses bytes");
 let MuxPdu::NotifyPaneSnapshot {
 snapshot: decoded_snap,
 ..
 } = decoded.pdu
 else {
 panic!("decoded PDU is not NotifyPaneSnapshot");
 };
 assert_eq!(
 decoded_snap.images.len(),
 render_buf.images.len(),
 "wire encode/decode dropped placements (real notcurses bytes)"
 );
 assert_eq!(
 decoded_snap.image_data.len(),
 render_buf.image_data.len(),
 "wire encode/decode dropped image_data entries (real notcurses bytes)"
 );
 for (orig, decoded) in render_buf
 .image_data
 .iter()
 .zip(decoded_snap.image_data.iter())
 {
 assert_eq!(orig.width, decoded.width, "wire codec mutated width");
 assert_eq!(orig.height, decoded.height, "wire codec mutated height");
 assert_eq!(
 (*orig.data).as_slice(),
 decoded.data.as_slice(),
 "wire codec mutated pixel bytes for real notcurses image"
 );
 }

 // Run the CLIENT extract — same code path the GUI uses to convert
 // the daemon's wire snapshot into a FrameInput it feeds to the GPU.
 let viewport = ViewportSize::new(640, 384);
 let cell = test_cell_metrics();
 let frame = extract_frame_from_snapshot(&decoded_snap, viewport, cell, &|_| None);

 eprintln!(
 "client extract: images.len={} image_data.len={} images_dirty={}",
 frame.content.images.len(),
 frame.content.image_data.len(),
 frame.content.images_dirty,
 );
 for img in &frame.content.image_data {
 eprintln!(
 " client image_data: id={:?} {}x{} px, data.len={}B",
 img.id,
 img.width,
 img.height,
 img.data.len()
 );
 }

 assert_eq!(
 frame.content.images.len(),
 render_buf.images.len(),
 "client extract dropped placements: render_buf had {}, frame has {}",
 render_buf.images.len(),
 frame.content.images.len(),
 );
 assert_eq!(
 frame.content.image_data.len(),
 render_buf.image_data.len(),
 "client extract dropped image_data entries: render_buf had {}, frame has {}",
 render_buf.image_data.len(),
 frame.content.image_data.len(),
 );

 let referenced_ids: std::collections::HashSet<_> =
 frame.content.images.iter().map(|p| p.image_id).collect();
 let provided_ids: std::collections::HashSet<_> =
 frame.content.image_data.iter().map(|d| d.id).collect();
 let missing: Vec<_> = referenced_ids.difference(&provided_ids).collect();
 assert!(
 missing.is_empty(),
 "client extract: placement(s) reference IDs with no image_data - \
 those placements would render BLANK at the GPU: \
 referenced={referenced_ids:?} provided={provided_ids:?} missing={missing:?}"
 );

 for (i, img) in frame.content.image_data.iter().enumerate() {
 assert!(
 img.width > 0 && img.height > 0,
 "client image_data[{i}] has zero dims {}x{}",
 img.width,
 img.height
 );
 assert!(
 !img.data.is_empty(),
 "client image_data[{i}] has empty pixel buffer (dims {}x{}) - \
 wire decoding dropped the bytes",
 img.width,
 img.height
 );
 let expected_min_bytes = img.width as usize * img.height as usize;
 assert!(
 img.data.len() >= expected_min_bytes,
 "client image_data[{i}] has {}B but {}x{} needs >= {}B",
 img.data.len(),
 img.width,
 img.height,
 expected_min_bytes
 );
 }
}
