//! Unit tests for the prepare phase.

use std::collections::HashMap;

use oriterm_core::{CellFlags, Column, CursorShape, Rgb, Selection, Side, StableRowIndex};

use super::{
    AtlasLookup, ShapedFrame, prepare_frame, prepare_frame_into, prepare_frame_shaped,
    prepare_frame_shaped_into,
};
use crate::font::{CellMetrics, FaceIdx, FontRealm, GlyphStyle, RasterKey, SyntheticFlags};
use crate::gpu::atlas::{AtlasEntry, AtlasKind};
use crate::gpu::frame_input::{FrameInput, FrameSelection, ViewportSize};
use crate::gpu::instance_writer::INSTANCE_SIZE;
use crate::gpu::prepared_frame::PreparedFrame;
use crate::gpu::srgb_to_linear;
use oriterm_ui::text::ShapedGlyph;

// ── Test atlas ──

/// Test atlas backed by a `HashMap`.
struct TestAtlas(HashMap<(char, GlyphStyle), AtlasEntry>);

impl AtlasLookup for TestAtlas {
    fn lookup(&self, ch: char, style: GlyphStyle) -> Option<&AtlasEntry> {
        self.0.get(&(ch, style))
    }

    fn lookup_key(&self, _key: RasterKey) -> Option<&AtlasEntry> {
        None
    }
}

/// Create a deterministic atlas entry for a character.
///
/// UV coordinates are derived from the char code for predictable assertions.
fn test_entry(ch: char) -> AtlasEntry {
    let code = ch as u32;
    AtlasEntry {
        page: 0,
        uv_x: (code % 16) as f32 / 16.0,
        uv_y: (code / 16) as f32 / 16.0,
        uv_w: 7.0 / 1024.0,
        uv_h: 14.0 / 1024.0,
        width: 7,
        height: 14,
        bearing_x: 1,
        bearing_y: 12,
        kind: AtlasKind::Mono,
    }
}

/// Build a test atlas with entries for the given characters (Regular style).
fn atlas_with(chars: &[char]) -> TestAtlas {
    let mut map = HashMap::new();
    for &c in chars {
        map.insert((c, GlyphStyle::Regular), test_entry(c));
    }
    TestAtlas(map)
}

/// Empty atlas that returns `None` for every lookup.
fn empty_atlas() -> TestAtlas {
    TestAtlas(HashMap::new())
}

// ── Decoded instance for assertions ──

/// Parsed 80-byte instance record for test assertions.
#[derive(Debug)]
struct DecodedInstance {
    pos: (f32, f32),
    size: (f32, f32),
    uv: [f32; 4],
    fg_color: [f32; 4],
    bg_color: [f32; 4],
    kind: u32,
}

fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn decode_instance(bytes: &[u8]) -> DecodedInstance {
    assert_eq!(bytes.len(), INSTANCE_SIZE);
    DecodedInstance {
        pos: (read_f32(bytes, 0), read_f32(bytes, 4)),
        size: (read_f32(bytes, 8), read_f32(bytes, 12)),
        uv: [
            read_f32(bytes, 16),
            read_f32(bytes, 20),
            read_f32(bytes, 24),
            read_f32(bytes, 28),
        ],
        fg_color: [
            read_f32(bytes, 32),
            read_f32(bytes, 36),
            read_f32(bytes, 40),
            read_f32(bytes, 44),
        ],
        bg_color: [
            read_f32(bytes, 48),
            read_f32(bytes, 52),
            read_f32(bytes, 56),
            read_f32(bytes, 60),
        ],
        kind: read_u32(bytes, 64),
    }
}

/// Decode the nth instance from a writer's byte buffer.
fn nth_instance(bytes: &[u8], n: usize) -> DecodedInstance {
    let start = n * INSTANCE_SIZE;
    decode_instance(&bytes[start..start + INSTANCE_SIZE])
}

/// Assert instance counts across all three buffers.
fn assert_counts(frame: &PreparedFrame, bg: usize, fg: usize, cursor: usize) {
    assert_eq!(
        frame.backgrounds.len(),
        bg,
        "expected {bg} bg instances, got {}",
        frame.backgrounds.len(),
    );
    assert_eq!(
        frame.glyphs.len(),
        fg,
        "expected {fg} fg instances, got {}",
        frame.glyphs.len(),
    );
    assert_eq!(
        frame.cursors.len(),
        cursor,
        "expected {cursor} cursor instances, got {}",
        frame.cursors.len(),
    );
}

/// Convert Rgb to the linear-light `[f32; 4]` that push_rect writes to bg_color.
fn rgb_f32(c: Rgb) -> [f32; 4] {
    [
        srgb_to_linear(c.r),
        srgb_to_linear(c.g),
        srgb_to_linear(c.b),
        1.0,
    ]
}

// ── Instance buffer correctness ──

#[test]
fn single_char_produces_one_bg_and_one_fg() {
    let input = FrameInput::test_grid(1, 1, "A");
    let atlas = atlas_with(&['A']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // 1 bg for the cell, 1 fg for the glyph, 1 cursor (block at 0,0).
    assert_counts(&frame, 1, 1, 1);
}

#[test]
fn single_char_bg_position_and_size() {
    let input = FrameInput::test_grid(2, 2, "A");
    let atlas = atlas_with(&['A']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let bg = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg.pos, (0.0, 0.0));
    assert_eq!(bg.size, (8.0, 16.0));
    assert_eq!(bg.kind, 0); // InstanceKind::Rect
}

#[test]
fn single_char_fg_position_with_bearing() {
    let input = FrameInput::test_grid(2, 2, "A");
    let atlas = atlas_with(&['A']);
    let entry = test_entry('A');

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    // glyph_x = 0.0 + bearing_x(1) = 1.0
    // glyph_y = 0.0 + baseline(12.0) - bearing_y(12) = 0.0
    assert_eq!(fg.pos, (1.0, 0.0));
    assert_eq!(fg.size, (entry.width as f32, entry.height as f32));
    assert_eq!(fg.uv, [entry.uv_x, entry.uv_y, entry.uv_w, entry.uv_h]);
    assert_eq!(fg.kind, 1); // InstanceKind::Glyph
}

#[test]
fn single_char_fg_color_matches_cell() {
    let input = FrameInput::test_grid(1, 1, "A");
    let atlas = atlas_with(&['A']);
    let fg_rgb = input.content.cells[0].fg;

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(fg.fg_color, rgb_f32(fg_rgb));
}

#[test]
fn single_char_bg_color_matches_cell() {
    let input = FrameInput::test_grid(1, 1, "A");
    let atlas = atlas_with(&['A']);
    let bg_rgb = input.content.cells[0].bg;

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let bg = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg.bg_color, rgb_f32(bg_rgb));
}

// ── Empty cells ──

#[test]
fn empty_cell_produces_bg_only() {
    let input = FrameInput::test_grid(1, 1, " ");
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.backgrounds.len(), 1);
    assert_eq!(frame.glyphs.len(), 0);
}

#[test]
fn all_spaces_grid_no_fg_instances() {
    let input = FrameInput::test_grid(10, 5, "");
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.backgrounds.len(), 50);
    assert_eq!(frame.glyphs.len(), 0);
}

#[test]
fn all_chars_grid_equal_bg_and_fg() {
    let text: String = std::iter::repeat_n('A', 10).collect();
    let input = FrameInput::test_grid(10, 1, &text);
    let atlas = atlas_with(&['A']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.backgrounds.len(), 10);
    assert_eq!(frame.glyphs.len(), 10);
}

// ── Wide characters ──

#[test]
fn wide_char_produces_double_width_bg() {
    let mut input = FrameInput::test_grid(4, 1, "");
    // Manually set up a wide char at column 0.
    input.content.cells[0].ch = '\u{4E16}'; // 世
    input.content.cells[0].flags = CellFlags::WIDE_CHAR;
    input.content.cells[1].ch = ' ';
    input.content.cells[1].flags = CellFlags::WIDE_CHAR_SPACER;

    let atlas = atlas_with(&['\u{4E16}']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // 1 bg for wide char (double width) + 2 bg for remaining cells = 3 bg.
    // 1 fg for the wide char glyph.
    assert_eq!(frame.backgrounds.len(), 3);
    assert_eq!(frame.glyphs.len(), 1);

    let bg = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg.size, (16.0, 16.0)); // 2 * cell_width
}

#[test]
fn wide_char_spacer_skipped() {
    let mut input = FrameInput::test_grid(2, 1, "");
    input.content.cells[0].ch = '\u{4E16}';
    input.content.cells[0].flags = CellFlags::WIDE_CHAR;
    input.content.cells[1].ch = ' ';
    input.content.cells[1].flags = CellFlags::WIDE_CHAR_SPACER;

    let atlas = atlas_with(&['\u{4E16}']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Only 1 bg (the wide char covers both columns), not 2.
    assert_eq!(frame.backgrounds.len(), 1);
}

// ── Cell positions are pixel-perfect ──

#[test]
fn cell_positions_are_pixel_perfect() {
    let input = FrameInput::test_grid(3, 3, "ABCDEFGHI");
    let atlas = atlas_with(&['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Cell (0,0) → (0, 0), (1,0) → (8, 0), (2,0) → (16, 0)
    // Cell (0,1) → (0, 16), (1,1) → (8, 16), etc.
    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg0.pos, (0.0, 0.0));

    let bg1 = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(bg1.pos, (8.0, 0.0));

    let bg2 = nth_instance(frame.backgrounds.as_bytes(), 2);
    assert_eq!(bg2.pos, (16.0, 0.0));

    let bg3 = nth_instance(frame.backgrounds.as_bytes(), 3);
    assert_eq!(bg3.pos, (0.0, 16.0));

    let bg4 = nth_instance(frame.backgrounds.as_bytes(), 4);
    assert_eq!(bg4.pos, (8.0, 16.0));
}

#[test]
fn glyph_bearing_offsets_applied() {
    let input = FrameInput::test_grid(2, 2, "A");
    let atlas = atlas_with(&['A']);
    let entry = test_entry('A');

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    let expected_x = 0.0 + entry.bearing_x as f32;
    let expected_y = 0.0 + 12.0 - entry.bearing_y as f32; // baseline=12
    assert_eq!(fg.pos, (expected_x, expected_y));
}

// ── Color resolution (passthrough from extract phase) ──

#[test]
fn default_colors_in_instances() {
    let input = FrameInput::test_grid(1, 1, "A");
    let atlas = atlas_with(&['A']);
    let cell = &input.content.cells[0];

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let bg = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg.bg_color, rgb_f32(cell.bg));

    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(fg.fg_color, rgb_f32(cell.fg));
}

#[test]
fn inverse_colors_passed_through() {
    // Extract phase already swaps fg/bg for INVERSE cells. Prepare just
    // copies them through. Verify the passthrough works.
    let mut input = FrameInput::test_grid(1, 1, "X");
    let original_fg = input.content.cells[0].fg;
    let original_bg = input.content.cells[0].bg;
    // Simulate what extract would have done: swap fg/bg.
    input.content.cells[0].fg = original_bg;
    input.content.cells[0].bg = original_fg;

    let atlas = atlas_with(&['X']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let bg = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg.bg_color, rgb_f32(original_fg));

    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(fg.fg_color, rgb_f32(original_bg));
}

// ── Determinism ──

#[test]
fn same_input_produces_identical_output() {
    let input = FrameInput::test_grid(10, 5, "Hello World! Testing determinism.");
    let atlas = atlas_with(&[
        'H', 'e', 'l', 'o', 'W', 'r', 'd', '!', 'T', 's', 't', 'i', 'n', 'g', 'm', '.',
    ]);

    let frame1 = prepare_frame(&input, &atlas, (0.0, 0.0));
    let frame2 = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame1.backgrounds.as_bytes(), frame2.backgrounds.as_bytes());
    assert_eq!(frame1.glyphs.as_bytes(), frame2.glyphs.as_bytes());
    assert_eq!(frame1.cursors.as_bytes(), frame2.cursors.as_bytes());
    assert_eq!(frame1.clear_color, frame2.clear_color);
}

// ── Cursor shapes ──

#[test]
fn block_cursor_one_instance() {
    let input = FrameInput::test_grid(10, 5, "");
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Default cursor is Block at (0,0), visible.
    assert_eq!(frame.cursors.len(), 1);

    let c = nth_instance(frame.cursors.as_bytes(), 0);
    assert_eq!(c.pos, (0.0, 0.0));
    assert_eq!(c.size, (8.0, 16.0));
    assert_eq!(c.kind, 2); // InstanceKind::Cursor
}

#[test]
fn bar_cursor_one_instance_2px_wide() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.cursor.shape = CursorShape::Bar;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.cursors.len(), 1);

    let c = nth_instance(frame.cursors.as_bytes(), 0);
    assert_eq!(c.pos, (0.0, 0.0));
    assert_eq!(c.size, (2.0, 16.0));
}

#[test]
fn underline_cursor_one_instance_2px_tall() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.cursor.shape = CursorShape::Underline;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.cursors.len(), 1);

    let c = nth_instance(frame.cursors.as_bytes(), 0);
    assert_eq!(c.pos, (0.0, 14.0)); // y + ch - 2.0 = 0 + 16 - 2 = 14
    assert_eq!(c.size, (8.0, 2.0));
}

#[test]
fn hollow_block_cursor_four_instances() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.cursor.shape = CursorShape::HollowBlock;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.cursors.len(), 4);
}

#[test]
fn hollow_block_edges() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.cursor.shape = CursorShape::HollowBlock;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let top = nth_instance(frame.cursors.as_bytes(), 0);
    assert_eq!(top.pos, (0.0, 0.0));
    assert_eq!(top.size, (8.0, 2.0));

    let bottom = nth_instance(frame.cursors.as_bytes(), 1);
    assert_eq!(bottom.pos, (0.0, 14.0));
    assert_eq!(bottom.size, (8.0, 2.0));

    let left = nth_instance(frame.cursors.as_bytes(), 2);
    assert_eq!(left.pos, (0.0, 0.0));
    assert_eq!(left.size, (2.0, 16.0));

    let right = nth_instance(frame.cursors.as_bytes(), 3);
    assert_eq!(right.pos, (6.0, 0.0)); // cw - 2.0 = 8 - 2 = 6
    assert_eq!(right.size, (2.0, 16.0));
}

#[test]
fn hidden_cursor_zero_instances() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.cursor.shape = CursorShape::Hidden;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.cursors.len(), 0);
}

#[test]
fn cursor_invisible_zero_instances() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.cursor.visible = false;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.cursors.len(), 0);
}

#[test]
fn cursor_at_position() {
    let mut input = FrameInput::test_grid(10, 10, "");
    input.content.cursor.column = Column(5);
    input.content.cursor.line = 3;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let c = nth_instance(frame.cursors.as_bytes(), 0);
    assert_eq!(c.pos, (40.0, 48.0)); // 5*8=40, 3*16=48
}

#[test]
fn cursor_color_from_palette() {
    let input = FrameInput::test_grid(10, 5, "");
    let atlas = empty_atlas();
    let cursor_color = input.palette.cursor_color;

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let c = nth_instance(frame.cursors.as_bytes(), 0);
    // Cursor color is in bg_color (rendered via bg_pipeline as solid-fill rect).
    assert_eq!(c.bg_color, rgb_f32(cursor_color));
}

#[test]
fn unfocused_window_renders_hollow_cursor() {
    let mut input = FrameInput::test_grid(10, 5, "");
    // Default cursor is Block; unfocused window overrides to HollowBlock.
    input.window_focused = false;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // HollowBlock emits 4 edge rectangles (top, bottom, left, right).
    assert_eq!(frame.cursors.len(), 4);
}

#[test]
fn focused_window_renders_block_cursor() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.window_focused = true;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Focused window renders default Block cursor (1 filled rect).
    assert_eq!(frame.cursors.len(), 1);
}

#[test]
fn unfocused_window_bar_cursor_becomes_hollow() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.cursor.shape = CursorShape::Bar;
    input.window_focused = false;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Bar cursor overridden to HollowBlock when unfocused.
    assert_eq!(frame.cursors.len(), 4);
}

// ── Missing atlas entries ──

#[test]
fn missing_glyph_skips_fg_instance() {
    let input = FrameInput::test_grid(1, 1, "Z");
    let atlas = empty_atlas(); // No entry for 'Z'.

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.backgrounds.len(), 1);
    assert_eq!(frame.glyphs.len(), 0);
}

// ── Glyph style from flags ──

#[test]
fn bold_cell_uses_bold_style() {
    let mut input = FrameInput::test_grid(1, 1, "B");
    input.content.cells[0].flags = CellFlags::BOLD;

    let mut map = HashMap::new();
    map.insert((('B'), GlyphStyle::Bold), test_entry('B'));
    let atlas = TestAtlas(map);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Should find the Bold entry and produce a glyph.
    assert_eq!(frame.glyphs.len(), 1);
}

#[test]
fn italic_cell_uses_italic_style() {
    let mut input = FrameInput::test_grid(1, 1, "I");
    input.content.cells[0].flags = CellFlags::ITALIC;

    let mut map = HashMap::new();
    map.insert(('I', GlyphStyle::Italic), test_entry('I'));
    let atlas = TestAtlas(map);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.glyphs.len(), 1);
}

#[test]
fn bold_italic_cell_uses_bold_italic_style() {
    let mut input = FrameInput::test_grid(1, 1, "X");
    input.content.cells[0].flags = CellFlags::BOLD | CellFlags::ITALIC;

    let mut map = HashMap::new();
    map.insert(('X', GlyphStyle::BoldItalic), test_entry('X'));
    let atlas = TestAtlas(map);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.glyphs.len(), 1);
}

// ── Instance count for larger grids ──

#[test]
fn ten_by_five_all_spaces() {
    let input = FrameInput::test_grid(10, 5, "");
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_counts(&frame, 50, 0, 1); // 1 cursor (block, visible)
}

#[test]
fn clear_color_matches_palette_background() {
    let input = FrameInput::test_grid(10, 5, "");
    let atlas = empty_atlas();
    let bg = input.palette.background;

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let expected = [
        f64::from(srgb_to_linear(bg.r)),
        f64::from(srgb_to_linear(bg.g)),
        f64::from(srgb_to_linear(bg.b)),
        1.0,
    ];
    assert_eq!(frame.clear_color, expected);
}

#[test]
fn clear_color_respects_palette_opacity() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.palette.opacity = 0.5;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let bg = input.palette.background;
    let expected = [
        f64::from(srgb_to_linear(bg.r)) * 0.5,
        f64::from(srgb_to_linear(bg.g)) * 0.5,
        f64::from(srgb_to_linear(bg.b)) * 0.5,
        0.5,
    ];
    assert_eq!(frame.clear_color, expected);
}

// ── DECSCNM (reverse video) ──

#[test]
fn reverse_video_clear_color_uses_swapped_bg() {
    let mut input = FrameInput::test_grid(10, 5, "A");
    let original_fg = input.palette.foreground;

    // Simulate DECSCNM: swap palette fg/bg and set reverse_video flag.
    std::mem::swap(&mut input.palette.foreground, &mut input.palette.background);
    input.reverse_video = true;

    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Clear color should be the original foreground (now stored as palette.background
    // after the swap in extract).
    let expected = [
        f64::from(srgb_to_linear(original_fg.r)),
        f64::from(srgb_to_linear(original_fg.g)),
        f64::from(srgb_to_linear(original_fg.b)),
        1.0,
    ];
    assert_eq!(frame.clear_color, expected);
}

// ── prepare_frame_into ──

#[test]
fn prepare_into_matches_prepare() {
    let input = FrameInput::test_grid(10, 5, "Hello World!");
    let atlas = atlas_with(&['H', 'e', 'l', 'o', 'W', 'r', 'd', '!']);

    let fresh = prepare_frame(&input, &atlas, (0.0, 0.0));

    let mut reused = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    prepare_frame_into(&input, &atlas, &mut reused, (0.0, 0.0));

    assert_eq!(fresh.backgrounds.as_bytes(), reused.backgrounds.as_bytes());
    assert_eq!(fresh.glyphs.as_bytes(), reused.glyphs.as_bytes());
    assert_eq!(fresh.cursors.as_bytes(), reused.cursors.as_bytes());
    assert_eq!(fresh.clear_color, reused.clear_color);
}

#[test]
fn prepare_into_reuses_allocation() {
    let large_text: String = std::iter::repeat_n('A', 50).collect();
    let input = FrameInput::test_grid(10, 5, &large_text);
    let atlas = atlas_with(&['A']);

    // First prepare allocates large buffers.
    let mut frame = prepare_frame(&input, &atlas, (0.0, 0.0));
    let first_bg_count = frame.backgrounds.len();
    let first_fg_count = frame.glyphs.len();

    // Second prepare with smaller input reuses (clear + refill).
    let small = FrameInput::test_grid(2, 1, "A");
    prepare_frame_into(&small, &atlas, &mut frame, (0.0, 0.0));

    // Counts reflect new input, not old.
    assert_eq!(frame.backgrounds.len(), 2);
    assert_eq!(frame.glyphs.len(), 1);
    assert!(first_bg_count > frame.backgrounds.len());
    assert!(first_fg_count > frame.glyphs.len());
}

#[test]
fn prepare_into_clears_previous_content() {
    let input1 = FrameInput::test_grid(10, 5, "AAAAAAAAAA");
    let atlas = atlas_with(&['A', 'B']);

    let mut frame = prepare_frame(&input1, &atlas, (0.0, 0.0));
    let first_bg = frame.backgrounds.len();
    let first_fg = frame.glyphs.len();

    // Second frame with different content.
    let input2 = FrameInput::test_grid(2, 1, "B");
    prepare_frame_into(&input2, &atlas, &mut frame, (0.0, 0.0));

    // Counts should reflect the new input, not accumulate.
    assert_eq!(frame.backgrounds.len(), 2); // 2 cells
    assert_eq!(frame.glyphs.len(), 1); // 1 glyph ('B')
    assert_ne!(frame.backgrounds.len(), first_bg + 2);
    assert_ne!(frame.glyphs.len(), first_fg + 1);
}

#[test]
fn prepare_into_updates_clear_color() {
    let input1 = FrameInput::test_grid(2, 1, "");
    let atlas = empty_atlas();

    let mut frame = prepare_frame(&input1, &atlas, (0.0, 0.0));
    let first_clear = frame.clear_color;

    // Change palette background.
    let mut input2 = FrameInput::test_grid(2, 1, "");
    input2.palette.background = Rgb { r: 255, g: 0, b: 0 };
    prepare_frame_into(&input2, &atlas, &mut frame, (0.0, 0.0));

    assert_ne!(frame.clear_color, first_clear);
    assert_eq!(frame.clear_color, [1.0, 0.0, 0.0, 1.0]);
}

// ── Full-size grid instance counts (80×24) ──

#[test]
fn full_grid_all_spaces_1920_bg_zero_fg() {
    let input = FrameInput::test_grid(80, 24, "");
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.backgrounds.len(), 80 * 24);
    assert_eq!(frame.glyphs.len(), 0);
}

#[test]
fn full_grid_all_chars_1920_bg_and_fg() {
    let text: String = std::iter::repeat_n('A', 80 * 24).collect();
    let input = FrameInput::test_grid(80, 24, &text);
    let atlas = atlas_with(&['A']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.backgrounds.len(), 80 * 24);
    assert_eq!(frame.glyphs.len(), 80 * 24);
}

// ── Color resolution: bold, 256-color, truecolor ──

#[test]
fn bold_color_variant_in_instance_bytes() {
    // Bold cells: the extract phase resolves the bold color. The prepare phase
    // passes it through. Verify the bold flag affects glyph style selection and
    // that the fg_color in the instance matches what was set on the cell.
    let bright_red = Rgb {
        r: 255,
        g: 100,
        b: 100,
    };
    let mut input = FrameInput::test_grid(1, 1, "B");
    input.content.cells[0].flags = CellFlags::BOLD;
    input.content.cells[0].fg = bright_red;

    let mut map = HashMap::new();
    map.insert(('B', GlyphStyle::Bold), test_entry('B'));
    let atlas = TestAtlas(map);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.glyphs.len(), 1);
    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(fg.fg_color, rgb_f32(bright_red));
}

#[test]
fn ansi_256_color_in_instance_bytes() {
    let color_208 = Rgb {
        r: 255,
        g: 135,
        b: 0,
    };
    let mut input = FrameInput::test_grid(1, 1, "X");
    input.content.cells[0].fg = color_208;

    let atlas = atlas_with(&['X']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(fg.fg_color, rgb_f32(color_208));
}

#[test]
fn truecolor_in_instance_bytes() {
    let tc = Rgb {
        r: 100,
        g: 200,
        b: 50,
    };
    let mut input = FrameInput::test_grid(1, 1, "T");
    input.content.cells[0].fg = tc;
    input.content.cells[0].bg = Rgb {
        r: 30,
        g: 30,
        b: 30,
    };

    let atlas = atlas_with(&['T']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(fg.fg_color, rgb_f32(tc));

    let bg = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(
        bg.bg_color,
        rgb_f32(Rgb {
            r: 30,
            g: 30,
            b: 30,
        }),
    );
}

// ── Viewport bounds ──

#[test]
fn no_instances_outside_grid_bounds() {
    // 3×2 grid at 8×16 cell size = 24×32 viewport.
    let input = FrameInput::test_grid(3, 2, "ABCDEF");
    let atlas = atlas_with(&['A', 'B', 'C', 'D', 'E', 'F']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let vp_w = 3.0 * 8.0; // 24.0
    let vp_h = 2.0 * 16.0; // 32.0

    // Verify all bg instances are within viewport.
    for i in 0..frame.backgrounds.len() {
        let inst = nth_instance(frame.backgrounds.as_bytes(), i);
        assert!(
            inst.pos.0 >= 0.0 && inst.pos.0 + inst.size.0 <= vp_w,
            "bg instance {i} x out of bounds: pos={}, size={}",
            inst.pos.0,
            inst.size.0,
        );
        assert!(
            inst.pos.1 >= 0.0 && inst.pos.1 + inst.size.1 <= vp_h,
            "bg instance {i} y out of bounds: pos={}, size={}",
            inst.pos.1,
            inst.size.1,
        );
    }
}

// ── Shaped rendering tests ──

/// Test atlas that looks up glyphs by [`RasterKey`] (shaped path).
struct KeyTestAtlas(HashMap<RasterKey, AtlasEntry>);

impl AtlasLookup for KeyTestAtlas {
    fn lookup(&self, _ch: char, _style: GlyphStyle) -> Option<&AtlasEntry> {
        None
    }

    fn lookup_key(&self, key: RasterKey) -> Option<&AtlasEntry> {
        self.0.get(&key)
    }
}

/// Create a deterministic atlas entry for a glyph ID.
fn test_entry_for_glyph(glyph_id: u16) -> AtlasEntry {
    AtlasEntry {
        page: 0,
        uv_x: (glyph_id % 16) as f32 / 16.0,
        uv_y: (glyph_id / 16) as f32 / 16.0,
        uv_w: 7.0 / 1024.0,
        uv_h: 14.0 / 1024.0,
        width: 7,
        height: 14,
        bearing_x: 1,
        bearing_y: 12,
        kind: AtlasKind::Mono,
    }
}

/// Build a `KeyTestAtlas` with entries for the given glyph IDs.
fn key_atlas_with(glyph_ids: &[u16], size_q6: u32) -> KeyTestAtlas {
    let mut map = HashMap::new();
    for &gid in glyph_ids {
        let key = RasterKey {
            glyph_id: gid.into(),
            face_idx: FaceIdx::REGULAR,
            weight: 0,
            size_q6,
            synthetic: SyntheticFlags::NONE,
            hinted: true,
            subpx_x: 0,
            font_realm: FontRealm::Terminal,
        };
        map.insert(key, test_entry_for_glyph(gid));
    }
    KeyTestAtlas(map)
}

/// Build a `KeyTestAtlas` whose glyph IDs route to all three terminal-tier
/// atlas kinds (mono, subpixel, color) so cross-buffer replay assertions
/// have non-empty buffers to compare. Glyph IDs are partitioned by index
/// modulo 3: 0 → Mono, 1 → Subpixel, 2 → Color. Caller-supplied IDs must
/// be at least three glyphs long for full kind coverage.
fn key_atlas_mixed_kinds(glyph_ids: &[u16], size_q6: u32) -> KeyTestAtlas {
    let mut map = HashMap::new();
    let kinds = [AtlasKind::Mono, AtlasKind::Subpixel, AtlasKind::Color];
    for (i, &gid) in glyph_ids.iter().enumerate() {
        let key = RasterKey {
            glyph_id: gid.into(),
            face_idx: FaceIdx::REGULAR,
            weight: 0,
            size_q6,
            synthetic: SyntheticFlags::NONE,
            hinted: true,
            subpx_x: 0,
            font_realm: FontRealm::Terminal,
        };
        let kind = kinds[i % kinds.len()];
        map.insert(
            key,
            AtlasEntry {
                kind,
                ..test_entry_for_glyph(gid)
            },
        );
    }
    KeyTestAtlas(map)
}

/// Build a ShapedFrame for a 1-row grid from a slice of ShapedGlyphs.
fn shaped_one_row(
    cols: usize,
    glyphs: &[ShapedGlyph],
    col_starts: &[usize],
    size_q6: u32,
) -> ShapedFrame {
    let mut sf = ShapedFrame::new(cols, size_q6);
    let mut col_map = Vec::new();
    crate::font::build_col_glyph_map(col_starts, cols, &mut col_map);
    sf.push_row(glyphs, col_starts, &col_map);
    sf
}

#[test]
fn shaped_single_glyph_one_bg_one_fg() {
    let size_q6 = 768; // 12px * 64
    let input = FrameInput::test_grid(3, 1, "A  ");
    let atlas = key_atlas_with(&[42], size_q6);

    let glyphs = vec![ShapedGlyph {
        glyph_id: 42,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let col_starts = vec![0];
    let shaped = shaped_one_row(3, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    // 3 bg instances (one per cell), 1 fg instance (shaped glyph at col 0), 1 cursor.
    assert_counts(&frame, 3, 1, 1);
}

#[test]
fn shaped_ligature_one_fg_two_bg() {
    // Simulate a ligature spanning cols 0-1 (e.g. "fi" → single glyph).
    let size_q6 = 768;
    let mut input = FrameInput::test_grid(3, 1, "fi ");
    // Mark col 0 as the ligature origin, col 1 as regular (the shaper
    // handles the merge — bg instances come from the cell data).
    input.content.cells[0].ch = 'f';
    input.content.cells[1].ch = 'i';

    let atlas = key_atlas_with(&[100], size_q6);
    let glyphs = vec![ShapedGlyph {
        glyph_id: 100,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let col_starts = vec![0]; // ligature starts at col 0, spans 2 columns via col_map
    let shaped = shaped_one_row(3, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    // 3 bg (per-cell), 1 fg (single ligature glyph at col 0), 1 cursor.
    assert_counts(&frame, 3, 1, 1);

    // The fg glyph should be at col 0 position.
    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    let entry = test_entry_for_glyph(100);
    assert_eq!(fg.pos.0, 0.0 + entry.bearing_x as f32);
}

#[test]
fn shaped_combining_marks_two_fg_instances() {
    // Base glyph at col 0 + combining mark at col 0 → 2 fg instances.
    let size_q6 = 768;
    let input = FrameInput::test_grid(2, 1, "a ");
    let atlas = key_atlas_with(&[50, 51], size_q6);

    let glyphs = vec![
        ShapedGlyph {
            glyph_id: 50,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 51,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 2.0,
            y_offset: 3.0,
        },
    ];
    let col_starts = vec![0, 0]; // both at col 0 — combining mark
    let shaped = shaped_one_row(2, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    // 2 bg (per-cell), 2 fg (base + combining mark), 1 cursor.
    assert_counts(&frame, 2, 2, 1);
}

#[test]
fn shaped_offset_applied_to_glyph_position() {
    use crate::font::{subpx_bin, subpx_offset};

    let size_q6 = 768;
    let input = FrameInput::test_grid(1, 1, "X");

    // x_offset 1.5 → fract 0.5 → subpx phase 2.
    let subpx = subpx_bin(1.5);
    let mut map = HashMap::new();
    map.insert(
        RasterKey {
            glyph_id: 60,
            face_idx: FaceIdx::REGULAR,
            weight: 0,
            size_q6,
            synthetic: SyntheticFlags::NONE,
            hinted: true,
            subpx_x: subpx,
            font_realm: FontRealm::Terminal,
        },
        test_entry_for_glyph(60),
    );
    let atlas = KeyTestAtlas(map);

    let glyphs = vec![ShapedGlyph {
        glyph_id: 60,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 1.5,
        y_offset: 2.0,
    }];
    let col_starts = vec![0];
    let shaped = shaped_one_row(1, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    assert_eq!(frame.glyphs.len(), 1);
    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    let entry = test_entry_for_glyph(60);

    // glyph_x = 0.0 + bearing_x(1) + x_offset(1.5) - absorbed(0.5) = 2.0
    let absorbed = subpx_offset(subpx);
    let expected_x = 0.0 + entry.bearing_x as f32 + 1.5 - absorbed;
    // glyph_y = 0.0 + baseline(12.0) - bearing_y(12) - y_offset(2.0) = -2.0
    let expected_y = 0.0 + 12.0 - entry.bearing_y as f32 - 2.0;
    assert_eq!(fg.pos, (expected_x, expected_y));
}

#[test]
fn shaped_backgrounds_independent_of_glyphs() {
    // Backgrounds should be per-cell regardless of shaped glyph layout.
    let size_q6 = 768;
    let input = FrameInput::test_grid(4, 1, "ABCD");
    // Ligature spans cols 0-1, normal glyphs at 2 and 3.
    let atlas = key_atlas_with(&[100, 101, 102], size_q6);
    let glyphs = vec![
        ShapedGlyph {
            glyph_id: 100,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 101,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 102,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
    ];
    let col_starts = vec![0, 2, 3];
    let shaped = shaped_one_row(4, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    // 4 bg instances (one per cell), 3 fg instances (ligature + 2 normal).
    assert_counts(&frame, 4, 3, 1);

    // Each bg is cell_width × cell_height at the correct position.
    for i in 0..4 {
        let bg = nth_instance(frame.backgrounds.as_bytes(), i);
        assert_eq!(bg.size, (8.0, 16.0), "bg {i} should be cell-sized");
        assert_eq!(bg.pos.0, i as f32 * 8.0, "bg {i} x position");
    }
}

#[test]
fn shaped_missing_glyph_in_atlas_skips_fg() {
    // Shaped glyph exists but atlas doesn't have it → no fg instance.
    let size_q6 = 768;
    let input = FrameInput::test_grid(1, 1, "X");
    let atlas = KeyTestAtlas(HashMap::new()); // empty atlas

    let glyphs = vec![ShapedGlyph {
        glyph_id: 99,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let col_starts = vec![0];
    let shaped = shaped_one_row(1, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    // 1 bg, 0 fg (atlas miss), 1 cursor.
    assert_counts(&frame, 1, 0, 1);
}

#[test]
fn shaped_empty_glyphs_produces_bg_only() {
    // All cells are spaces → no shaped glyphs → bg only.
    let size_q6 = 768;
    let input = FrameInput::test_grid(3, 1, "   ");
    let atlas = KeyTestAtlas(HashMap::new());

    let shaped = ShapedFrame::new(3, size_q6);
    // Push an empty row (no glyphs).
    let empty_glyphs: Vec<ShapedGlyph> = Vec::new();
    let empty_col_starts: Vec<usize> = Vec::new();
    let mut col_map = Vec::new();
    crate::font::build_col_glyph_map(&empty_col_starts, 3, &mut col_map);

    let mut sf = shaped;
    sf.push_row(&empty_glyphs, &empty_col_starts, &col_map);
    let frame = prepare_frame_shaped(&input, &atlas, &sf, (0.0, 0.0));

    assert_counts(&frame, 3, 0, 1);
}

// ── Color glyph routing (Section 6.10) ──

#[test]
fn color_glyph_routes_to_color_glyphs_buffer() {
    // A shaped glyph with AtlasKind::Color should go to frame.color_glyphs,
    // not frame.glyphs.
    let size_q6 = 768;
    let input = FrameInput::test_grid(1, 1, "E"); // emoji placeholder

    let mut map = HashMap::new();
    let key = RasterKey {
        glyph_id: 200,
        face_idx: FaceIdx::REGULAR,
        weight: 0,
        size_q6,
        synthetic: SyntheticFlags::NONE,
        hinted: true,
        subpx_x: 0,
        font_realm: FontRealm::Terminal,
    };
    map.insert(
        key,
        AtlasEntry {
            page: 0,
            uv_x: 0.1,
            uv_y: 0.2,
            uv_w: 0.05,
            uv_h: 0.05,
            width: 14,
            height: 14,
            bearing_x: 0,
            bearing_y: 12,
            kind: AtlasKind::Color, // Color emoji!
        },
    );
    let atlas = KeyTestAtlas(map);

    let glyphs = vec![ShapedGlyph {
        glyph_id: 200,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let col_starts = vec![0];
    let shaped = shaped_one_row(1, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    // Monochrome glyphs should be empty — the color glyph went to color_glyphs.
    assert_eq!(
        frame.glyphs.len(),
        0,
        "color glyph should NOT be in monochrome buffer"
    );
    assert_eq!(
        frame.color_glyphs.len(),
        1,
        "color glyph should be in color buffer"
    );
}

#[test]
fn mixed_color_and_mono_glyphs_route_correctly() {
    // Mix of monochrome and color glyphs in the same row.
    let size_q6 = 768;
    let input = FrameInput::test_grid(3, 1, "AEB");

    let mut map = HashMap::new();
    // Mono glyph 'A' at col 0.
    map.insert(
        RasterKey {
            glyph_id: 10,
            face_idx: FaceIdx::REGULAR,
            weight: 0,
            size_q6,
            synthetic: SyntheticFlags::NONE,
            hinted: true,
            subpx_x: 0,
            font_realm: FontRealm::Terminal,
        },
        test_entry_for_glyph(10),
    );
    // Color emoji 'E' at col 1.
    map.insert(
        RasterKey {
            glyph_id: 200,
            face_idx: FaceIdx::REGULAR,
            weight: 0,
            size_q6,
            synthetic: SyntheticFlags::NONE,
            hinted: true,
            subpx_x: 0,
            font_realm: FontRealm::Terminal,
        },
        AtlasEntry {
            kind: AtlasKind::Color,
            ..test_entry_for_glyph(200)
        },
    );
    // Mono glyph 'B' at col 2.
    map.insert(
        RasterKey {
            glyph_id: 11,
            face_idx: FaceIdx::REGULAR,
            weight: 0,
            size_q6,
            synthetic: SyntheticFlags::NONE,
            hinted: true,
            subpx_x: 0,
            font_realm: FontRealm::Terminal,
        },
        test_entry_for_glyph(11),
    );
    let atlas = KeyTestAtlas(map);

    let glyphs = vec![
        ShapedGlyph {
            glyph_id: 10,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 200,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 11,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
    ];
    let col_starts = vec![0, 1, 2];
    let shaped = shaped_one_row(3, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    assert_eq!(frame.glyphs.len(), 2, "2 mono glyphs in monochrome buffer");
    assert_eq!(frame.color_glyphs.len(), 1, "1 color glyph in color buffer");
    assert_eq!(frame.backgrounds.len(), 3, "3 backgrounds (one per cell)");
}

// ── prepare_frame_shaped_into ──

#[test]
fn shaped_into_matches_shaped() {
    let size_q6 = 768;
    let input = FrameInput::test_grid(4, 1, "ABCD");
    let atlas = key_atlas_with(&[100, 101, 102], size_q6);
    let glyphs = vec![
        ShapedGlyph {
            glyph_id: 100,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 101,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 102,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
    ];
    let col_starts = vec![0, 2, 3];
    let shaped = shaped_one_row(4, &glyphs, &col_starts, size_q6);

    let fresh = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    let mut reused = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut reused, (0.0, 0.0), 1.0);

    assert_eq!(fresh.backgrounds.as_bytes(), reused.backgrounds.as_bytes());
    assert_eq!(fresh.glyphs.as_bytes(), reused.glyphs.as_bytes());
    assert_eq!(fresh.cursors.as_bytes(), reused.cursors.as_bytes());
    assert_eq!(fresh.clear_color, reused.clear_color);
    assert_eq!(fresh.viewport, reused.viewport);
}

#[test]
fn shaped_into_reuses_allocation() {
    let size_q6 = 768;
    let large_text: String = std::iter::repeat_n('A', 50).collect();
    let input = FrameInput::test_grid(10, 5, &large_text);

    // Build shaped data for 50 glyphs.
    let glyphs: Vec<ShapedGlyph> = (0..50)
        .map(|_| ShapedGlyph {
            glyph_id: 42,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        })
        .collect();
    let col_starts: Vec<usize> = (0..50).map(|i| i % 10).collect();
    let atlas = key_atlas_with(&[42], size_q6);

    // Build shaped frame with all 5 rows.
    let mut sf = ShapedFrame::new(10, size_q6);
    for row_start in (0..50).step_by(10) {
        let row_glyphs = &glyphs[row_start..row_start + 10];
        let row_col_starts = &col_starts[row_start..row_start + 10];
        let mut col_map = Vec::new();
        crate::font::build_col_glyph_map(row_col_starts, 10, &mut col_map);
        sf.push_row(row_glyphs, row_col_starts, &col_map);
    }

    // First prepare.
    let mut frame = prepare_frame_shaped(&input, &atlas, &sf, (0.0, 0.0));
    let first_bg = frame.backgrounds.len();
    let first_fg = frame.glyphs.len();

    // Second prepare with smaller input reuses allocations.
    let small = FrameInput::test_grid(2, 1, "A ");
    let small_glyphs = vec![ShapedGlyph {
        glyph_id: 42,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let small_col_starts = vec![0];
    let small_shaped = shaped_one_row(2, &small_glyphs, &small_col_starts, size_q6);
    prepare_frame_shaped_into(&small, &atlas, &small_shaped, &mut frame, (0.0, 0.0), 1.0);

    assert_eq!(frame.backgrounds.len(), 2);
    assert!(first_bg > frame.backgrounds.len());
    assert!(first_fg > frame.glyphs.len());
}

// ── Text decoration tests (Section 6.12) ──

/// Build a 1×1 test grid with the given flags on cell 0.
fn frame_with_flags(flags: CellFlags) -> FrameInput {
    let mut input = FrameInput::test_grid(1, 1, "A");
    input.content.cells[0].flags = flags;
    input
}

/// Build a 1×1 test grid with flags and an explicit underline color.
fn frame_with_underline_color(flags: CellFlags, color: Rgb) -> FrameInput {
    let mut input = FrameInput::test_grid(1, 1, "A");
    input.content.cells[0].flags = flags;
    input.content.cells[0].underline_color = Some(color);
    input
}

/// Count background instances beyond the 1 base background rect per cell.
///
/// In a 1×1 grid, the first bg instance is always the cell background.
/// Any additional instances come from decorations.
fn decoration_bg_count(frame: &PreparedFrame) -> usize {
    frame.backgrounds.len() - 1
}

#[test]
fn single_underline_one_extra_bg() {
    let input = frame_with_flags(CellFlags::UNDERLINE);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // 1 base bg + 1 underline rect.
    assert_eq!(decoration_bg_count(&frame), 1);

    // Underline Y = y + cell_height - 2.0 = 0 + 16 - 2 = 14.
    let ul = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(ul.pos.1, 14.0);
    assert_eq!(ul.size, (8.0, 1.0));
}

#[test]
fn single_underline_uses_fg_color() {
    let input = frame_with_flags(CellFlags::UNDERLINE);
    let fg = input.content.cells[0].fg;
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let ul = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(ul.bg_color, rgb_f32(fg));
}

#[test]
fn single_underline_uses_sgr58_color() {
    let sgr58 = Rgb {
        r: 255,
        g: 0,
        b: 128,
    };
    let input = frame_with_underline_color(CellFlags::UNDERLINE, sgr58);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let ul = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(ul.bg_color, rgb_f32(sgr58));
}

#[test]
fn double_underline_two_extra_bgs() {
    let input = frame_with_flags(CellFlags::DOUBLE_UNDERLINE);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // 1 base bg + 2 underline rects.
    assert_eq!(decoration_bg_count(&frame), 2);

    // First line at underline_y = 14, second at underline_y - 2 = 12.
    let ul1 = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(ul1.pos.1, 14.0);
    assert_eq!(ul1.size, (8.0, 1.0));

    let ul2 = nth_instance(frame.backgrounds.as_bytes(), 2);
    assert_eq!(ul2.pos.1, 12.0);
    assert_eq!(ul2.size, (8.0, 1.0));
}

#[test]
fn curly_underline_per_pixel_rects() {
    let input = frame_with_flags(CellFlags::CURLY_UNDERLINE);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // cell_width=8 → 8 per-pixel rects.
    assert_eq!(decoration_bg_count(&frame), 8);
}

#[test]
fn dotted_underline_alternating() {
    let input = frame_with_flags(CellFlags::DOTTED_UNDERLINE);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // cell_width=8, step_by(2) → 4 dots (at 0, 2, 4, 6).
    assert_eq!(decoration_bg_count(&frame), 4);
}

#[test]
fn dashed_underline_pattern() {
    let input = frame_with_flags(CellFlags::DASHED_UNDERLINE);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // cell_width=8, pattern 3-on-2-off: dx 0,1,2 on, 3,4 off, 5,6,7 on → 6.
    assert_eq!(decoration_bg_count(&frame), 6);
}

#[test]
fn strikethrough_at_center() {
    let input = frame_with_flags(CellFlags::STRIKETHROUGH);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // 1 base bg + 1 strikethrough rect.
    assert_eq!(decoration_bg_count(&frame), 1);

    // Strikethrough Y = y + cell_height / 2.0 = 0 + 8.0.
    let st = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(st.pos.1, 8.0);
    assert_eq!(st.size, (8.0, 1.0));
}

#[test]
fn strikethrough_uses_fg_color() {
    let input = frame_with_flags(CellFlags::STRIKETHROUGH);
    let fg = input.content.cells[0].fg;
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let st = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(st.bg_color, rgb_f32(fg));
}

#[test]
fn underline_and_strikethrough_coexist() {
    let input = frame_with_flags(CellFlags::UNDERLINE | CellFlags::STRIKETHROUGH);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // 1 base bg + 1 underline + 1 strikethrough = 2 decoration rects.
    assert_eq!(decoration_bg_count(&frame), 2);
}

#[test]
fn no_flags_no_decorations() {
    let input = frame_with_flags(CellFlags::empty());
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // 1 base bg only, no decorations.
    assert_eq!(decoration_bg_count(&frame), 0);
}

#[test]
fn wide_char_underline_spans_double_width() {
    let mut input = FrameInput::test_grid(4, 1, "");
    // Wide char at col 0.
    input.content.cells[0].ch = '\u{4E16}';
    input.content.cells[0].flags = CellFlags::WIDE_CHAR | CellFlags::UNDERLINE;
    input.content.cells[1].ch = ' ';
    input.content.cells[1].flags = CellFlags::WIDE_CHAR_SPACER;

    let atlas = atlas_with(&['\u{4E16}']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Find the underline rect (second bg instance for the wide char cell).
    let ul = nth_instance(frame.backgrounds.as_bytes(), 1);
    // Wide char bg_w = 2 * cell_width = 16.0, underline should match.
    assert_eq!(ul.size.0, 16.0);
    assert_eq!(ul.size.1, 1.0);
}

// ── Overline / superscript / subscript tests () ──
// Cell metrics in test_grid: 8x16 cell, baseline=12, stroke=1, strikeout_offset=4.
// Overline y = cell_top y = 0; thickness = stroke_size = 1.
// Super offset = -16 * 0.25 = -4; Sub offset = +4. Both already integer.

/// Regression: property for SGR 53 (overline) GPU emission.
#[test]
fn overline_emits_rect_at_cell_top_with_stroke_size_thickness() {
    let input = frame_with_flags(CellFlags::OVERLINE);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // 1 base bg + 1 overline rect.
    assert_eq!(decoration_bg_count(&frame), 1);

    let ol = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(ol.pos.1, 0.0, "overline y must be cell-top y (0.0)");
    assert_eq!(
        ol.size,
        (8.0, 1.0),
        "overline must span cell_width x stroke_size"
    );
}

/// Regression: overline uses fg color (no SGR for "colored overline").
#[test]
fn overline_uses_fg_color() {
    let input = frame_with_flags(CellFlags::OVERLINE);
    let fg = input.content.cells[0].fg;
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let ol = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(ol.bg_color, rgb_f32(fg));
}

/// Regression: regression guard: cell without OVERLINE produces no top rect.
#[test]
fn overline_absent_emits_no_top_rect() {
    let input = frame_with_flags(CellFlags::empty());
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));
    assert_eq!(decoration_bg_count(&frame), 0);
}

/// Regression: OVERLINE-only cell triggers decoration emission
/// (pins the early-return predicate update).
#[test]
fn overline_only_cell_passes_decoration_fast_path_gate() {
    // Without the early-return predicate update, an OVERLINE-only cell would
    // silently skip the entire DecorationContext::draw function.
    let input = frame_with_flags(CellFlags::OVERLINE);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));
    // Decoration count > 0 proves the gate was reached.
    assert!(decoration_bg_count(&frame) > 0);
}

/// Regression: OVERLINE composes with UNDERLINE (top + bottom rects).
#[test]
fn overline_with_underline_emits_two_separate_rects() {
    let input = frame_with_flags(CellFlags::OVERLINE | CellFlags::UNDERLINE);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // 1 base bg + 1 underline + 1 overline = 2 decoration rects.
    assert_eq!(decoration_bg_count(&frame), 2);
}

/// Regression: OVERLINE composes with STRIKETHROUGH.
#[test]
fn overline_with_strikethrough_emits_two_separate_rects() {
    let input = frame_with_flags(CellFlags::OVERLINE | CellFlags::STRIKETHROUGH);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(decoration_bg_count(&frame), 2);
}

/// Regression: OVERLINE composes with DOUBLE_UNDERLINE (1 + 2 = 3 rects).
#[test]
fn overline_with_double_underline_emits_three_rects() {
    let input = frame_with_flags(CellFlags::OVERLINE | CellFlags::DOUBLE_UNDERLINE);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // 1 base bg + 2 double-underline rects + 1 overline = 3 decoration rects.
    assert_eq!(decoration_bg_count(&frame), 3);
}

/// Regression: OVERLINE + DOUBLE_UNDERLINE + STRIKETHROUGH
/// composition (matrix gap from close-out).
#[test]
fn overline_with_double_underline_and_strikethrough_emits_four_rects() {
    let flags = CellFlags::OVERLINE | CellFlags::DOUBLE_UNDERLINE | CellFlags::STRIKETHROUGH;
    let input = frame_with_flags(flags);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // 1 base bg + 2 double-underline + 1 strikethrough + 1 overline = 4 decoration rects.
    assert_eq!(decoration_bg_count(&frame), 4);
}

/// Regression: OVERLINE on a wide char spans 2 cell-widths.
#[test]
fn overline_on_wide_char_spans_double_width() {
    let mut input = FrameInput::test_grid(4, 1, "");
    input.content.cells[0].ch = '\u{4E16}';
    input.content.cells[0].flags = CellFlags::WIDE_CHAR | CellFlags::OVERLINE;
    input.content.cells[1].ch = ' ';
    input.content.cells[1].flags = CellFlags::WIDE_CHAR_SPACER;

    let atlas = atlas_with(&['\u{4E16}']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let ol = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(ol.pos.1, 0.0);
    assert_eq!(
        ol.size.0, 16.0,
        "overline must span 2 * cell_width on wide char"
    );
    assert_eq!(ol.size.1, 1.0);
}

/// Regression: property for SGR 73 (superscript) glyph y shift.
/// In test_grid: cell_height=16, FACTOR=0.25 → offset=-4. With baseline=12 and
/// test atlas bearing_y=12, normal glyph_y = 0 + 12 - 12 = 0; super glyph_y = -4.
#[test]
fn superscript_shifts_glyph_y_up_by_quarter_cell_height() {
    let input = frame_with_flags(CellFlags::SUPERSCRIPT);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // The test atlas has bearing_y=12 == baseline, so a non-shifted glyph
    // emits at y=0. SUPERSCRIPT shifts by `-cell_height * 0.25 = -4.0`.
    let glyph = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(
        glyph.pos.1, -4.0,
        "SUPERSCRIPT must shift glyph y up by 4px (= 16 * 0.25); pinned y={}",
        glyph.pos.1
    );
}

/// Regression: property for SGR 74 (subscript) glyph y shift.
#[test]
fn subscript_shifts_glyph_y_down_by_quarter_cell_height() {
    let input = frame_with_flags(CellFlags::SUBSCRIPT);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let glyph = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(
        glyph.pos.1, 4.0,
        "SUBSCRIPT must shift glyph y down by 4px; pinned y={}",
        glyph.pos.1
    );
}

/// Regression: without super/sub flags, glyph y is unshifted.
#[test]
fn no_super_sub_flag_emits_unshifted_glyph_y() {
    let input = frame_with_flags(CellFlags::empty());
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let glyph = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(glyph.pos.1, 0.0, "no super/sub flag => unshifted glyph y");
}

/// Regression: regression guard: SUPERSCRIPT/SUBSCRIPT MUST NOT shift
/// decoration y (underline, strikethrough, overline stay anchored to cell).
#[test]
fn decorations_y_unaffected_by_super_sub() {
    // SUPERSCRIPT + UNDERLINE: glyph shifts up, underline stays at baseline+offset.
    let input = frame_with_flags(CellFlags::SUPERSCRIPT | CellFlags::UNDERLINE);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Underline is the second bg (after the cell bg).
    let ul = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(
        ul.pos.1, 14.0,
        "underline y must remain at cell-anchored 14.0"
    );

    // Glyph y is shifted by SUPERSCRIPT.
    let glyph = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(glyph.pos.1, -4.0, "glyph y must shift up");
}

/// Regression: regression guard: SUBSCRIPT MUST NOT shift strikethrough y.
#[test]
fn subscript_does_not_shift_strikethrough_y() {
    let input = frame_with_flags(CellFlags::SUBSCRIPT | CellFlags::STRIKETHROUGH);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Strikethrough at y = baseline - strikeout_offset = 12 - 4 = 8.
    let st = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(st.pos.1, 8.0, "strikethrough y must stay cell-anchored");

    let glyph = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(glyph.pos.1, 4.0);
}

/// Regression: regression guard: SUBSCRIPT MUST NOT shift overline y.
#[test]
fn subscript_does_not_shift_overline_y() {
    let input = frame_with_flags(CellFlags::SUBSCRIPT | CellFlags::OVERLINE);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Overline at cell top y = 0 (NOT shifted with glyph).
    let ol = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(
        ol.pos.1, 0.0,
        "overline y stays at cell top regardless of SUBSCRIPT"
    );

    let glyph = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(glyph.pos.1, 4.0);
}

/// Regression: SUPERSCRIPT + INVERSE: bg quad fills full cell rect
/// (NOT shifted with glyph).
#[test]
fn superscript_with_inverse_keeps_full_cell_background() {
    let input = frame_with_flags(CellFlags::SUPERSCRIPT | CellFlags::INVERSE);
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Background is the FIRST bg instance (the cell bg quad). It must be at
    // y=0 with the full cell height (16), regardless of SUPERSCRIPT.
    let bg = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg.pos.1, 0.0, "SUPERSCRIPT must NOT shift cell bg quad");
    assert_eq!(bg.size, (8.0, 16.0), "bg fills full cell");

    // Glyph still shifts up.
    let glyph = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(glyph.pos.1, -4.0);
}

/// Regression: fractional cell heights (e.g. 13px) must round
/// to integer pixel offset, preserving Y-snap from `mod.rs:257`.
#[test]
fn super_sub_offset_rounds_to_integer_for_fractional_cell_height() {
    use super::super_sub_glyph_offset;

    // 13.0 * 0.25 = 3.25 → must round to 3.0.
    assert_eq!(
        super_sub_glyph_offset(CellFlags::SUBSCRIPT, 13.0),
        3.0,
        "SUBSCRIPT offset on 13px cell must round to integer 3.0, not 3.25"
    );
    assert_eq!(
        super_sub_glyph_offset(CellFlags::SUPERSCRIPT, 13.0),
        -3.0,
        "SUPERSCRIPT offset on 13px cell must round to integer -3.0"
    );
    // 16.0 * 0.25 = 4.0 → already integer, no round artifact.
    assert_eq!(super_sub_glyph_offset(CellFlags::SUBSCRIPT, 16.0), 4.0,);
    // empty flags → 0.0 always.
    assert_eq!(super_sub_glyph_offset(CellFlags::empty(), 16.0), 0.0,);
    // Mutually exclusive: only one of SUPERSCRIPT/SUBSCRIPT is checked.
    // SGR handler enforces mutual exclusion (sgr.rs:64-69), so the helper
    // never sees both at once. Still test the happy-path edge values.
}

/// Regression: shaped path applies super/sub offset to glyph y.
/// Pinned via `prepare_frame_shaped` rather than the unshaped path so the
/// production code path (`fill_frame_shaped`) is exercised directly.
#[test]
fn shaped_superscript_shifts_glyph_y_up_by_quarter_cell_height() {
    let size_q6 = 768;
    let mut input = FrameInput::test_grid(1, 1, "X");
    input.content.cells[0].flags = CellFlags::SUPERSCRIPT;

    let atlas = key_atlas_with(&[60], size_q6);
    let glyphs = vec![ShapedGlyph {
        glyph_id: 60,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let col_starts = vec![0];
    let shaped = shaped_one_row(1, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    assert_eq!(frame.glyphs.len(), 1);
    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    let entry = test_entry_for_glyph(60);
    // glyph_y = (y + super_offset) + baseline - bearing_y
    // = (0.0 + -4.0) + 12.0 - 12.0 = -4.0
    let expected_y = -4.0 + 12.0 - entry.bearing_y as f32;
    assert_eq!(
        fg.pos.1, expected_y,
        "shaped SUPERSCRIPT must shift glyph y up by 4.0; got {}",
        fg.pos.1
    );
}

/// Regression: shaped path applies SUBSCRIPT offset (downward shift).
#[test]
fn shaped_subscript_shifts_glyph_y_down_by_quarter_cell_height() {
    let size_q6 = 768;
    let mut input = FrameInput::test_grid(1, 1, "X");
    input.content.cells[0].flags = CellFlags::SUBSCRIPT;

    let atlas = key_atlas_with(&[61], size_q6);
    let glyphs = vec![ShapedGlyph {
        glyph_id: 61,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let col_starts = vec![0];
    let shaped = shaped_one_row(1, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    let entry = test_entry_for_glyph(61);
    let expected_y = 4.0 + 12.0 - entry.bearing_y as f32;
    assert_eq!(fg.pos.1, expected_y);
}

/// Regression: shaped path WITHOUT super/sub keeps glyph y unshifted.
/// Pins that the offset only applies when the flag is set (no spurious shift).
#[test]
fn shaped_no_super_sub_keeps_glyph_y_unshifted() {
    let size_q6 = 768;
    let input = FrameInput::test_grid(1, 1, "X");
    let atlas = key_atlas_with(&[62], size_q6);
    let glyphs = vec![ShapedGlyph {
        glyph_id: 62,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let col_starts = vec![0];
    let shaped = shaped_one_row(1, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    let entry = test_entry_for_glyph(62);
    // No offset → glyph_y = 0.0 + 12.0 - 12.0 = 0.0.
    let expected_y = 12.0 - entry.bearing_y as f32;
    assert_eq!(fg.pos.1, expected_y);
}

/// Regression: built-in glyph path (e.g. box-drawing chars in
/// U+2500..=U+257F) shifts y when SUPERSCRIPT/SUBSCRIPT is set. The built-in
/// branch is a separate emission site from `GlyphEmitter::emit`; both must
/// honor the offset.
#[test]
fn shaped_builtin_glyph_with_superscript_shifts_y() {
    let size_q6 = 768;
    let mut input = FrameInput::test_grid(1, 1, "");
    // U+2500 BOX DRAWINGS LIGHT HORIZONTAL — built-in geometric glyph.
    input.content.cells[0].ch = '\u{2500}';
    input.content.cells[0].flags = CellFlags::SUPERSCRIPT;

    // Built-in glyphs use FaceIdx::BUILTIN with the codepoint as glyph_id.
    let key = crate::gpu::builtin_glyphs::raster_key('\u{2500}', size_q6);
    let mut map = HashMap::new();
    map.insert(key, test_entry_for_glyph('\u{2500}' as u16));
    let atlas = KeyTestAtlas(map);

    // ShapedFrame is empty for built-ins (the prepare path takes the built-in
    // branch BEFORE consulting the shaped frame).
    let shaped = shaped_one_row(1, &[], &[], size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    assert_eq!(
        frame.glyphs.len(),
        1,
        "built-in glyph should emit 1 fg instance"
    );
    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    // Built-in glyph rect uses (x, glyph_y) directly — no baseline/bearing math.
    // glyph_y = y + super_sub_offset = 0.0 + (-4.0) = -4.0.
    assert_eq!(
        fg.pos.1, -4.0,
        "built-in glyph y must shift by SUPERSCRIPT offset"
    );
}

/// Regression: dirty-skip incremental path applies super/sub
/// offset to dirty rows. Pin: a row with SUPERSCRIPT, after a dirty rebuild,
/// emits a shifted glyph y.
#[test]
fn incremental_dirty_row_with_superscript_shifts_glyph_y() {
    use crate::gpu::frame_input::ViewportSize;
    use oriterm_core::Rgb;

    let size_q6 = 768;
    let cols = 1;
    let rows = 1;
    let mut input = FrameInput::test_grid(cols, rows, "X");
    input.content.cells[0].flags = CellFlags::SUPERSCRIPT;

    let atlas = key_atlas_with(&[70], size_q6);
    let glyphs = vec![ShapedGlyph {
        glyph_id: 70,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let col_starts = vec![0];
    let shaped = shaped_one_row(cols, &glyphs, &col_starts, size_q6);

    // First pass: populate row_ranges via full rebuild (all_dirty true by default).
    let mut frame = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);

    // Second pass: mark row 0 as dirty so the incremental path regenerates it.
    input.content.all_dirty = false;
    input.content.damage.push(oriterm_core::DamageLine {
        line: 0,
        left: Column(0),
        right: Column(0),
    });
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);

    // Even after the incremental rebuild, the SUPERSCRIPT cell's glyph y
    // must be shifted. Find the glyph instance in the (post-incremental) frame.
    assert!(
        !frame.glyphs.as_bytes().is_empty(),
        "dirty row must regenerate the glyph"
    );
    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    let entry = test_entry_for_glyph(70);
    let expected_y = -4.0 + 12.0 - entry.bearing_y as f32;
    assert_eq!(
        fg.pos.1, expected_y,
        "incremental SUPERSCRIPT must produce shifted glyph y"
    );
}

/// Regression: shaped path emits OVERLINE rect at cell top
/// (matrix gap closed during impl-hygiene Phase 5).
#[test]
fn shaped_overline_emits_top_rect() {
    let size_q6 = 768;
    let mut input = FrameInput::test_grid(1, 1, "X");
    input.content.cells[0].flags = CellFlags::OVERLINE;

    let atlas = key_atlas_with(&[80], size_q6);
    let glyphs = vec![ShapedGlyph {
        glyph_id: 80,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let col_starts = vec![0];
    let shaped = shaped_one_row(1, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    // 1 base bg + 1 overline rect = 2 backgrounds; 1 fg glyph.
    assert_eq!(
        frame.backgrounds.len(),
        2,
        "shaped OVERLINE emits decoration rect"
    );
    let ol = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(ol.pos.1, 0.0, "shaped overline y must be cell-top y");
    assert_eq!(
        ol.size,
        (8.0, 1.0),
        "shaped overline must span cell_width x stroke"
    );
}

/// Regression: dirty-skip incremental path emits OVERLINE rect
/// for a dirty row with the OVERLINE flag (matrix gap closed during
/// impl-hygiene Phase 5).
#[test]
fn incremental_dirty_row_with_overline_emits_top_rect() {
    use crate::gpu::frame_input::ViewportSize;
    use oriterm_core::Rgb;

    let size_q6 = 768;
    let cols = 1;
    let rows = 1;
    let mut input = FrameInput::test_grid(cols, rows, "X");
    input.content.cells[0].flags = CellFlags::OVERLINE;

    let atlas = key_atlas_with(&[81], size_q6);
    let glyphs = vec![ShapedGlyph {
        glyph_id: 81,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let col_starts = vec![0];
    let shaped = shaped_one_row(cols, &glyphs, &col_starts, size_q6);

    // First pass: full rebuild to populate row_ranges.
    let mut frame = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);

    // Second pass: dirty-mark row 0 so the incremental path regenerates it.
    input.content.all_dirty = false;
    input.content.damage.push(oriterm_core::DamageLine {
        line: 0,
        left: Column(0),
        right: Column(0),
    });
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);

    // Even after the incremental rebuild, the OVERLINE rect must be present.
    // 1 base bg + 1 overline rect (cursor is in cursors buffer, not backgrounds).
    assert_eq!(
        frame.backgrounds.len(),
        2,
        "incremental dirty row with OVERLINE must emit decoration rect"
    );
    let ol = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(ol.pos.1, 0.0);
    assert_eq!(ol.size, (8.0, 1.0));
}

// ── Subpixel glyph routing (Section 6.16) ──

#[test]
fn subpixel_glyph_routes_to_subpixel_buffer() {
    // A shaped glyph with AtlasKind::Subpixel should go to frame.subpixel_glyphs,
    // not frame.glyphs (mono) or frame.color_glyphs.
    let size_q6 = 768;
    let input = FrameInput::test_grid(1, 1, "A");

    let mut map = HashMap::new();
    let key = RasterKey {
        glyph_id: 42,
        face_idx: FaceIdx::REGULAR,
        weight: 0,
        size_q6,
        synthetic: SyntheticFlags::NONE,
        hinted: true,
        subpx_x: 0,
        font_realm: FontRealm::Terminal,
    };
    map.insert(
        key,
        AtlasEntry {
            kind: AtlasKind::Subpixel,
            ..test_entry_for_glyph(42)
        },
    );
    let atlas = KeyTestAtlas(map);

    let glyphs = vec![ShapedGlyph {
        glyph_id: 42,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let col_starts = vec![0];
    let shaped = shaped_one_row(1, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    assert_eq!(
        frame.glyphs.len(),
        0,
        "subpixel glyph should NOT be in monochrome buffer",
    );
    assert_eq!(
        frame.subpixel_glyphs.len(),
        1,
        "subpixel glyph should be in subpixel buffer",
    );
    assert_eq!(
        frame.color_glyphs.len(),
        0,
        "subpixel glyph should NOT be in color buffer",
    );
}

#[test]
fn mixed_mono_subpixel_color_route_to_separate_buffers() {
    // Three glyphs, one per atlas kind, all route to their correct buffers.
    let size_q6 = 768;
    let input = FrameInput::test_grid(3, 1, "ABC");

    let mut map = HashMap::new();
    // Mono glyph.
    map.insert(
        RasterKey {
            glyph_id: 10,
            face_idx: FaceIdx::REGULAR,
            weight: 0,
            size_q6,
            synthetic: SyntheticFlags::NONE,
            hinted: true,
            subpx_x: 0,
            font_realm: FontRealm::Terminal,
        },
        test_entry_for_glyph(10), // default: AtlasKind::Mono
    );
    // Subpixel glyph.
    map.insert(
        RasterKey {
            glyph_id: 20,
            face_idx: FaceIdx::REGULAR,
            weight: 0,
            size_q6,
            synthetic: SyntheticFlags::NONE,
            hinted: true,
            subpx_x: 0,
            font_realm: FontRealm::Terminal,
        },
        AtlasEntry {
            kind: AtlasKind::Subpixel,
            ..test_entry_for_glyph(20)
        },
    );
    // Color glyph.
    map.insert(
        RasterKey {
            glyph_id: 30,
            face_idx: FaceIdx::REGULAR,
            weight: 0,
            size_q6,
            synthetic: SyntheticFlags::NONE,
            hinted: true,
            subpx_x: 0,
            font_realm: FontRealm::Terminal,
        },
        AtlasEntry {
            kind: AtlasKind::Color,
            ..test_entry_for_glyph(30)
        },
    );
    let atlas = KeyTestAtlas(map);

    let glyphs = vec![
        ShapedGlyph {
            glyph_id: 10,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 20,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 30,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
    ];
    let col_starts = vec![0, 1, 2];
    let shaped = shaped_one_row(3, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    assert_eq!(frame.glyphs.len(), 1, "1 mono glyph");
    assert_eq!(frame.subpixel_glyphs.len(), 1, "1 subpixel glyph");
    assert_eq!(frame.color_glyphs.len(), 1, "1 color glyph");
}

// ── Async resize guard tests ──

#[test]
fn shaped_frame_smaller_than_viewport_skips_excess_cells() {
    // Shaped frame has 2 cols, but viewport grid has 4 cols.
    // Cells beyond shaped.cols() should produce bg but no fg panic.
    let size_q6 = 768;
    let input = FrameInput::test_grid(4, 1, "ABCD");

    // Atlas has entries for glyph IDs used in the shaped frame.
    let atlas = key_atlas_with(&[10, 11], size_q6);

    // Shaped frame only covers 2 columns (not 4).
    let glyphs = vec![
        ShapedGlyph {
            glyph_id: 10,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 11,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
    ];
    let col_starts = vec![0, 1];
    let shaped = shaped_one_row(2, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    // All 4 cells produce backgrounds.
    assert_eq!(frame.backgrounds.len(), 4);
    // Only 2 shaped glyphs (cols 2-3 skipped by the resize guard).
    assert_eq!(frame.glyphs.len(), 2);
}

#[test]
fn shaped_frame_fewer_rows_than_viewport_skips_excess_rows() {
    // Viewport has 3 rows, shaped frame has 1 row.
    let size_q6 = 768;
    let input = FrameInput::test_grid(2, 3, "AB    ");

    let atlas = key_atlas_with(&[10], size_q6);

    // Only 1 row in the shaped frame.
    let glyphs = vec![ShapedGlyph {
        glyph_id: 10,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let col_starts = vec![0];
    let shaped = shaped_one_row(2, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    // 6 backgrounds (2 cols * 3 rows).
    assert_eq!(frame.backgrounds.len(), 6);
    // Only 1 glyph (from row 0); rows 1-2 skipped by guard.
    assert_eq!(frame.glyphs.len(), 1);
}

#[test]
fn shaped_frame_larger_than_viewport_no_panic() {
    // Shaped frame has more data than the viewport — should not panic,
    // only viewport cells get iterated.
    let size_q6 = 768;
    let input = FrameInput::test_grid(2, 1, "AB");

    // Atlas has both glyph IDs.
    let atlas = key_atlas_with(&[10, 11, 12, 13], size_q6);

    // Shaped frame has 4 columns (more than viewport's 2).
    let glyphs = vec![
        ShapedGlyph {
            glyph_id: 10,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 11,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 12,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 13,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
    ];
    let col_starts = vec![0, 1, 2, 3];
    let shaped = shaped_one_row(4, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    // Only 2 backgrounds (viewport is 2×1).
    assert_eq!(frame.backgrounds.len(), 2);
    // Only 2 glyphs (viewport cols 0 and 1).
    assert_eq!(frame.glyphs.len(), 2);
}

// ── Origin offset tests (Section 07.11) ──

#[test]
fn origin_offset_shifts_bg_positions() {
    let input = FrameInput::test_grid(2, 1, "AB");
    let atlas = atlas_with(&['A', 'B']);

    let frame = prepare_frame(&input, &atlas, (10.0, 20.0));

    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg0.pos, (10.0, 20.0));

    let bg1 = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(bg1.pos, (18.0, 20.0)); // 10.0 + 1*8.0
}

#[test]
fn origin_offset_shifts_glyph_positions() {
    let input = FrameInput::test_grid(1, 1, "A");
    let atlas = atlas_with(&['A']);
    let entry = test_entry('A');

    let frame = prepare_frame(&input, &atlas, (5.0, 15.0));

    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    // glyph_x = 5.0 + 0*8 + bearing_x(1) = 6.0
    // glyph_y = 15.0 + 0*16 + baseline(12.0) - bearing_y(12) = 15.0
    assert_eq!(fg.pos, (5.0 + entry.bearing_x as f32, 15.0));
}

#[test]
fn origin_offset_shifts_cursor_position() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.cursor.column = Column(2);
    input.content.cursor.line = 3;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (30.0, 50.0));

    let c = nth_instance(frame.cursors.as_bytes(), 0);
    // x = 30.0 + 2*8 = 46.0, y = 50.0 + 3*16 = 98.0
    assert_eq!(c.pos, (46.0, 98.0));
}

#[test]
fn zero_origin_matches_no_origin() {
    let input = FrameInput::test_grid(3, 2, "ABCDEF");
    let atlas = atlas_with(&['A', 'B', 'C', 'D', 'E', 'F']);

    // Default origin is (0.0, 0.0).
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg0.pos, (0.0, 0.0));

    let bg1 = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(bg1.pos, (8.0, 0.0));
}

#[test]
fn origin_offset_shaped_shifts_all_instances() {
    let size_q6 = 768;
    let mut input = FrameInput::test_grid(2, 1, "AB");
    // Viewport must be large enough to contain origin + cell area.
    input.viewport = ViewportSize::new(200, 300);

    let atlas = key_atlas_with(&[10, 11], size_q6);
    let glyphs = vec![
        ShapedGlyph {
            glyph_id: 10,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 11,
            face_index: 0,
            synthetic: 0,
            x_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
    ];
    let col_starts = vec![0, 1];
    let shaped = shaped_one_row(2, &glyphs, &col_starts, size_q6);
    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (100.0, 200.0));

    // Backgrounds shifted by origin.
    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg0.pos, (100.0, 200.0));

    let bg1 = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(bg1.pos, (108.0, 200.0)); // 100 + 1*8

    // Cursor shifted by origin.
    let c = nth_instance(frame.cursors.as_bytes(), 0);
    assert_eq!(c.pos, (100.0, 200.0));
}

// ── Selection rendering ──

/// Helper: create a FrameSelection covering columns `start_col..=end_col` on
/// viewport line `line`. Uses `stable_row_base = 0` so stable row == viewport line.
fn selection_range(line: usize, start_col: usize, end_col: usize) -> FrameSelection {
    let anchor = oriterm_core::SelectionPoint {
        row: StableRowIndex(line as u64),
        col: start_col,
        side: Side::Left,
    };
    let end = oriterm_core::SelectionPoint {
        row: StableRowIndex(line as u64),
        col: end_col,
        side: Side::Right,
    };
    let sel = Selection::new_char(anchor.row, anchor.col, Side::Left);
    // Build a selection spanning the range by constructing bounds directly.
    let mut sel = sel;
    sel.end = end;
    FrameSelection::new(&sel, 0)
}

#[test]
fn selection_inverts_bg_color() {
    let mut input = FrameInput::test_grid(3, 1, "ABC");
    let atlas = atlas_with(&['A', 'B', 'C']);

    // Select column 1 ("B").
    input.selection = Some(selection_range(0, 1, 1));

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // 3 bg instances: col 0 (normal), col 1 (selected), col 2 (normal).
    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    let bg1 = nth_instance(frame.backgrounds.as_bytes(), 1);
    let bg2 = nth_instance(frame.backgrounds.as_bytes(), 2);

    let normal_bg = rgb_f32(input.content.cells[0].bg);
    let selected_bg = rgb_f32(Rgb {
        r: 211,
        g: 215,
        b: 207,
    });

    assert_eq!(bg0.bg_color, normal_bg, "col 0 should be normal bg");
    assert_eq!(bg1.bg_color, selected_bg, "col 1 should have inverted bg");
    assert_eq!(bg2.bg_color, normal_bg, "col 2 should be normal bg");
}

#[test]
fn selection_inverts_fg_color() {
    let mut input = FrameInput::test_grid(2, 1, "AB");
    let atlas = atlas_with(&['A', 'B']);

    // Hide cursor so block cursor exclusion doesn't interfere.
    input.content.cursor.visible = false;

    // Select column 0 ("A").
    input.selection = Some(selection_range(0, 0, 0));

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Glyph "A" (col 0) should have inverted fg (cell bg instead of light gray).
    let fg0 = nth_instance(frame.glyphs.as_bytes(), 0);
    let selected_fg = rgb_f32(input.content.cells[0].bg);
    assert_eq!(
        fg0.fg_color, selected_fg,
        "selected glyph should have inverted fg"
    );

    // Glyph "B" (col 1) should have normal fg.
    let fg1 = nth_instance(frame.glyphs.as_bytes(), 1);
    let normal_fg = rgb_f32(Rgb {
        r: 211,
        g: 215,
        b: 207,
    });
    assert_eq!(
        fg1.fg_color, normal_fg,
        "unselected glyph should have normal fg"
    );
}

#[test]
fn selection_no_effect_when_none() {
    let input = FrameInput::test_grid(2, 1, "AB");
    let atlas = atlas_with(&['A', 'B']);

    // No selection.
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    let bg1 = nth_instance(frame.backgrounds.as_bytes(), 1);
    let normal_bg = rgb_f32(input.content.cells[0].bg);

    assert_eq!(bg0.bg_color, normal_bg);
    assert_eq!(bg1.bg_color, normal_bg);
}

#[test]
fn selection_wide_char_highlights_both_cells() {
    use oriterm_core::RenderableCell;

    // Build a grid with a wide char at col 0: 'Ａ' (fullwidth A, 2 cells wide).
    // Use non-palette bg so bg quads are emitted for assertion.
    let fg = Rgb {
        r: 211,
        g: 215,
        b: 207,
    };
    let bg = Rgb {
        r: 30,
        g: 30,
        b: 46,
    };

    let cells = vec![
        RenderableCell {
            line: 0,
            column: Column(0),
            ch: 'Ａ',
            fg,
            bg,
            flags: CellFlags::WIDE_CHAR,
            underline_color: None,
            has_hyperlink: false,
            hyperlink_uri: None,
            zerowidth: Vec::new(),
        },
        RenderableCell {
            line: 0,
            column: Column(1),
            ch: ' ',
            fg,
            bg,
            flags: CellFlags::WIDE_CHAR_SPACER,
            underline_color: None,
            has_hyperlink: false,
            hyperlink_uri: None,
            zerowidth: Vec::new(),
        },
        RenderableCell {
            line: 0,
            column: Column(2),
            ch: 'B',
            fg,
            bg,
            flags: CellFlags::empty(),
            underline_color: None,
            has_hyperlink: false,
            hyperlink_uri: None,
            zerowidth: Vec::new(),
        },
    ];

    let mut input = FrameInput::test_grid(3, 1, "");
    input.content.cells = cells;
    input.content.cursor.visible = false;

    // Select just col 0 (the wide char base cell).
    input.selection = Some(selection_range(0, 0, 0));

    let atlas = atlas_with(&['Ａ', 'B']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Wide char spacers are skipped, so we get 2 bg instances:
    // bg[0] = wide char (2 cells wide, selected), bg[1] = 'B' (normal).
    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    let bg1 = nth_instance(frame.backgrounds.as_bytes(), 1);

    let selected_bg = rgb_f32(Rgb {
        r: 211,
        g: 215,
        b: 207,
    });
    let normal_bg = rgb_f32(input.content.cells[0].bg);

    assert_eq!(bg0.bg_color, selected_bg, "wide char should be selected");
    assert_eq!(bg0.size, (16.0, 16.0), "wide char bg should span 2 cells");
    assert_eq!(bg1.bg_color, normal_bg, "'B' should be normal");
}

#[test]
fn selection_block_mode_rectangular() {
    use oriterm_core::SelectionPoint;

    // 4x2 grid: "ABCD" / "EFGH". Block select cols 1..2, rows 0..1.
    let mut input = FrameInput::test_grid(4, 2, "ABCDEFGH");
    let atlas = atlas_with(&['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H']);

    let anchor = SelectionPoint {
        row: StableRowIndex(0),
        col: 1,
        side: Side::Left,
    };
    let pivot = SelectionPoint {
        row: StableRowIndex(0),
        col: 1,
        side: Side::Left,
    };
    let mut sel = Selection::new_word(anchor, pivot);
    sel.mode = oriterm_core::SelectionMode::Block;
    sel.end = SelectionPoint {
        row: StableRowIndex(1),
        col: 2,
        side: Side::Right,
    };
    input.selection = Some(FrameSelection::new(&sel, 0));

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let selected_bg = rgb_f32(Rgb {
        r: 211,
        g: 215,
        b: 207,
    });
    let normal_bg = rgb_f32(input.content.cells[0].bg);

    // Row 0: A(normal) B(selected) C(selected) D(normal).
    let a = nth_instance(frame.backgrounds.as_bytes(), 0);
    let b = nth_instance(frame.backgrounds.as_bytes(), 1);
    let c = nth_instance(frame.backgrounds.as_bytes(), 2);
    let d = nth_instance(frame.backgrounds.as_bytes(), 3);

    assert_eq!(a.bg_color, normal_bg, "A should be normal");
    assert_eq!(b.bg_color, selected_bg, "B should be selected");
    assert_eq!(c.bg_color, selected_bg, "C should be selected");
    assert_eq!(d.bg_color, normal_bg, "D should be normal");

    // Row 1: E(normal) F(selected) G(selected) H(normal).
    let e = nth_instance(frame.backgrounds.as_bytes(), 4);
    let f = nth_instance(frame.backgrounds.as_bytes(), 5);
    let g = nth_instance(frame.backgrounds.as_bytes(), 6);
    let h = nth_instance(frame.backgrounds.as_bytes(), 7);

    assert_eq!(e.bg_color, normal_bg, "E should be normal");
    assert_eq!(f.bg_color, selected_bg, "F should be selected");
    assert_eq!(g.bg_color, selected_bg, "G should be selected");
    assert_eq!(h.bg_color, normal_bg, "H should be normal");
}

#[test]
fn selection_wide_char_spacer_only_highlights_both() {
    use oriterm_core::RenderableCell;

    // Wide char at col 0, spacer at col 1, narrow 'B' at col 2.
    // Selection covers only col 1 (the spacer). The wide char should
    // still be highlighted because you can't render half a wide char.
    // Use non-palette bg so bg quads are emitted for assertion.
    let fg = Rgb {
        r: 211,
        g: 215,
        b: 207,
    };
    let bg = Rgb {
        r: 30,
        g: 30,
        b: 46,
    };

    let cells = vec![
        RenderableCell {
            line: 0,
            column: Column(0),
            ch: 'Ａ',
            fg,
            bg,
            flags: CellFlags::WIDE_CHAR,
            underline_color: None,
            has_hyperlink: false,
            hyperlink_uri: None,
            zerowidth: Vec::new(),
        },
        RenderableCell {
            line: 0,
            column: Column(1),
            ch: ' ',
            fg,
            bg,
            flags: CellFlags::WIDE_CHAR_SPACER,
            underline_color: None,
            has_hyperlink: false,
            hyperlink_uri: None,
            zerowidth: Vec::new(),
        },
        RenderableCell {
            line: 0,
            column: Column(2),
            ch: 'B',
            fg,
            bg,
            flags: CellFlags::empty(),
            underline_color: None,
            has_hyperlink: false,
            hyperlink_uri: None,
            zerowidth: Vec::new(),
        },
    ];

    let mut input = FrameInput::test_grid(3, 1, "");
    input.content.cells = cells;
    input.content.cursor.visible = false;

    // Select only col 1 (the spacer column).
    input.selection = Some(selection_range(0, 1, 1));

    let atlas = atlas_with(&['Ａ', 'B']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let selected_bg = rgb_f32(Rgb {
        r: 211,
        g: 215,
        b: 207,
    });
    let normal_bg = rgb_f32(input.content.cells[0].bg);

    // bg[0] = wide char (should be selected because spacer col is in range).
    // bg[1] = 'B' (normal).
    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    let bg1 = nth_instance(frame.backgrounds.as_bytes(), 1);

    assert_eq!(
        bg0.bg_color, selected_bg,
        "wide char should be selected via spacer"
    );
    assert_eq!(bg1.bg_color, normal_bg, "'B' should be normal");
}

#[test]
fn selection_across_wrapped_lines_no_gap() {
    // Two rows, selection spans from row 0 col 2 to row 1 col 1.
    // All cells from col 2 on row 0 and cols 0..1 on row 1 should be selected.
    let mut input = FrameInput::test_grid(4, 2, "ABCDEFGH");
    let atlas = atlas_with(&['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H']);

    let anchor = oriterm_core::SelectionPoint {
        row: StableRowIndex(0),
        col: 2,
        side: Side::Left,
    };
    let sel = Selection::new_char(anchor.row, anchor.col, Side::Left);
    let mut sel = sel;
    sel.end = oriterm_core::SelectionPoint {
        row: StableRowIndex(1),
        col: 1,
        side: Side::Right,
    };
    input.selection = Some(FrameSelection::new(&sel, 0));

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let selected_bg = rgb_f32(Rgb {
        r: 211,
        g: 215,
        b: 207,
    });
    let normal_bg = rgb_f32(input.content.cells[0].bg);

    // Row 0: A(norm) B(norm) C(sel) D(sel).
    let a = nth_instance(frame.backgrounds.as_bytes(), 0);
    let b = nth_instance(frame.backgrounds.as_bytes(), 1);
    let c = nth_instance(frame.backgrounds.as_bytes(), 2);
    let d = nth_instance(frame.backgrounds.as_bytes(), 3);
    assert_eq!(a.bg_color, normal_bg, "A should be normal");
    assert_eq!(b.bg_color, normal_bg, "B should be normal");
    assert_eq!(c.bg_color, selected_bg, "C should be selected");
    assert_eq!(d.bg_color, selected_bg, "D should be selected");

    // Row 1: E(sel) F(sel) G(norm) H(norm).
    let e = nth_instance(frame.backgrounds.as_bytes(), 4);
    let f = nth_instance(frame.backgrounds.as_bytes(), 5);
    let g = nth_instance(frame.backgrounds.as_bytes(), 6);
    let h = nth_instance(frame.backgrounds.as_bytes(), 7);
    assert_eq!(
        e.bg_color, selected_bg,
        "E should be selected (wrap continues)"
    );
    assert_eq!(f.bg_color, selected_bg, "F should be selected");
    assert_eq!(g.bg_color, normal_bg, "G should be normal");
    assert_eq!(h.bg_color, normal_bg, "H should be normal");
}

#[test]
fn selection_block_cursor_skips_inversion() {
    use oriterm_core::RenderableCell;

    // 3x1 grid: "ABC". Select all three columns. Visible block cursor at col 1.
    // Col 1 should NOT be inverted (cursor overlay dominates).
    // Use non-palette bg so bg quads are emitted for assertion.
    let fg = Rgb {
        r: 211,
        g: 215,
        b: 207,
    };
    let bg = Rgb {
        r: 30,
        g: 30,
        b: 46,
    };

    let cells = vec![
        RenderableCell {
            line: 0,
            column: Column(0),
            ch: 'A',
            fg,
            bg,
            flags: CellFlags::empty(),
            underline_color: None,
            has_hyperlink: false,
            hyperlink_uri: None,
            zerowidth: Vec::new(),
        },
        RenderableCell {
            line: 0,
            column: Column(1),
            ch: 'B',
            fg,
            bg,
            flags: CellFlags::empty(),
            underline_color: None,
            has_hyperlink: false,
            hyperlink_uri: None,
            zerowidth: Vec::new(),
        },
        RenderableCell {
            line: 0,
            column: Column(2),
            ch: 'C',
            fg,
            bg,
            flags: CellFlags::empty(),
            underline_color: None,
            has_hyperlink: false,
            hyperlink_uri: None,
            zerowidth: Vec::new(),
        },
    ];

    let mut input = FrameInput::test_grid(3, 1, "");
    input.content.cells = cells;
    // Visible block cursor at col 1.
    input.content.cursor.visible = true;
    input.content.cursor.shape = CursorShape::Block;
    input.content.cursor.line = 0;
    input.content.cursor.column = Column(1);

    input.selection = Some(selection_range(0, 0, 2));

    let atlas = atlas_with(&['A', 'B', 'C']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let selected_bg = rgb_f32(fg);
    let normal_bg = rgb_f32(bg);

    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    let bg1 = nth_instance(frame.backgrounds.as_bytes(), 1);
    let bg2 = nth_instance(frame.backgrounds.as_bytes(), 2);

    assert_eq!(bg0.bg_color, selected_bg, "A should be selected");
    assert_eq!(
        bg1.bg_color, normal_bg,
        "B at cursor should NOT be inverted"
    );
    assert_eq!(bg2.bg_color, selected_bg, "C should be selected");
}

#[test]
fn selection_inverse_cell_uses_palette_defaults() {
    use oriterm_core::RenderableCell;

    // A cell with INVERSE flag already has fg/bg swapped by the renderable layer.
    // Selection on this cell should use palette defaults, not double-swap.
    let fg = Rgb {
        r: 211,
        g: 215,
        b: 207,
    };
    let bg = Rgb { r: 0, g: 0, b: 0 };

    // INVERSE cell: renderable layer already swapped fg↔bg.
    let cells = vec![RenderableCell {
        line: 0,
        column: Column(0),
        ch: 'A',
        fg: bg, // Swapped by renderable layer.
        bg: fg, // Swapped by renderable layer.
        flags: CellFlags::INVERSE,
        underline_color: None,
        has_hyperlink: false,
        hyperlink_uri: None,
        zerowidth: Vec::new(),
    }];

    let mut input = FrameInput::test_grid(1, 1, "");
    input.content.cells = cells;
    input.content.cursor.visible = false;
    input.selection = Some(selection_range(0, 0, 0));

    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // INVERSE + selected: should use palette defaults (bg=foreground, fg=background).
    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    let fg0 = nth_instance(frame.glyphs.as_bytes(), 0);

    let palette_fg = rgb_f32(fg);
    let palette_bg = rgb_f32(bg);

    assert_eq!(
        bg0.bg_color, palette_fg,
        "INVERSE selected bg should be palette foreground"
    );
    assert_eq!(
        fg0.fg_color, palette_bg,
        "INVERSE selected fg should be palette background"
    );
}

#[test]
fn selection_fg_eq_bg_falls_back_to_palette() {
    use oriterm_core::RenderableCell;

    // A cell where fg == bg (e.g., both red). Naive inversion would keep them
    // equal, making text invisible. Should fall back to palette defaults.
    let red = Rgb {
        r: 200,
        g: 50,
        b: 50,
    };

    let cells = vec![RenderableCell {
        line: 0,
        column: Column(0),
        ch: 'X',
        fg: red,
        bg: red,
        flags: CellFlags::empty(),
        underline_color: None,
        has_hyperlink: false,
        hyperlink_uri: None,
        zerowidth: Vec::new(),
    }];

    let mut input = FrameInput::test_grid(1, 1, "");
    input.content.cells = cells;
    input.content.cursor.visible = false;
    input.selection = Some(selection_range(0, 0, 0));

    let atlas = atlas_with(&['X']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // fg==bg swap still produces fg==bg. Should fall back to palette defaults.
    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    let fg0 = nth_instance(frame.glyphs.as_bytes(), 0);

    let palette_fg = rgb_f32(Rgb {
        r: 211,
        g: 215,
        b: 207,
    });
    let palette_bg = rgb_f32(Rgb { r: 0, g: 0, b: 0 });

    assert_eq!(
        bg0.bg_color, palette_fg,
        "fg==bg selected should fall back to palette fg as bg"
    );
    assert_eq!(
        fg0.fg_color, palette_bg,
        "fg==bg selected should fall back to palette bg as fg"
    );
}

#[test]
fn selection_hidden_cell_stays_invisible() {
    use oriterm_core::RenderableCell;

    // A HIDDEN (SGR 8) cell where fg == bg intentionally hides text.
    // Selection should NOT reveal it — the fg==bg fallback should be skipped.
    // Use a non-palette bg so the bg quad is emitted for assertion.
    let bg = Rgb {
        r: 30,
        g: 30,
        b: 46,
    };

    let cells = vec![RenderableCell {
        line: 0,
        column: Column(0),
        ch: 'S',
        fg: bg, // Hidden: fg set to bg.
        bg,
        flags: CellFlags::HIDDEN,
        underline_color: None,
        has_hyperlink: false,
        hyperlink_uri: None,
        zerowidth: Vec::new(),
    }];

    let mut input = FrameInput::test_grid(1, 1, "");
    input.content.cells = cells;
    input.content.cursor.visible = false;
    input.selection = Some(selection_range(0, 0, 0));

    let atlas = atlas_with(&['S']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // HIDDEN + selected: swap produces fg==bg, but HIDDEN guard skips fallback.
    // Result: sel_fg = cell.bg = black, sel_bg = cell.fg = black → both black.
    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    let fg0 = nth_instance(frame.glyphs.as_bytes(), 0);

    assert_eq!(
        bg0.bg_color, fg0.fg_color,
        "HIDDEN cell should remain invisible when selected"
    );
}

#[test]
fn selection_preserves_instance_counts() {
    // Selection is implemented as color inversion on existing instances, not
    // as a separate overlay layer. Instance counts must be identical
    // regardless of whether a selection is active.
    let text: String = std::iter::repeat_n('A', 10).collect();
    let atlas = atlas_with(&['A']);

    // Baseline: no selection.
    let input_no_sel = FrameInput::test_grid(10, 3, &text);
    let frame_no_sel = prepare_frame(&input_no_sel, &atlas, (0.0, 0.0));

    // With selection covering a partial range on row 0.
    let mut input_sel = FrameInput::test_grid(10, 3, &text);
    input_sel.selection = Some(selection_range(0, 2, 7));
    let frame_sel = prepare_frame(&input_sel, &atlas, (0.0, 0.0));

    assert_eq!(
        frame_no_sel.backgrounds.len(),
        frame_sel.backgrounds.len(),
        "selection should not change bg instance count"
    );
    assert_eq!(
        frame_no_sel.glyphs.len(),
        frame_sel.glyphs.len(),
        "selection should not change fg instance count"
    );
    assert_eq!(
        frame_no_sel.cursors.len(),
        frame_sel.cursors.len(),
        "selection should not change cursor instance count"
    );

    // Verify selected cells have inverted colors while unselected cells are unchanged.
    let normal_bg = rgb_f32(input_sel.content.cells[0].bg);
    let selected_bg = rgb_f32(Rgb {
        r: 211,
        g: 215,
        b: 207,
    });

    let bg_col1 = nth_instance(frame_sel.backgrounds.as_bytes(), 1);
    assert_eq!(bg_col1.bg_color, normal_bg, "col 1 should be normal bg");

    let bg_col3 = nth_instance(frame_sel.backgrounds.as_bytes(), 3);
    assert_eq!(
        bg_col3.bg_color, selected_bg,
        "col 3 (in selection) should have inverted bg"
    );
}

#[test]
fn selection_underline_cursor_does_not_skip_inversion() {
    // Non-block cursors (underline, beam) should NOT prevent selection inversion.
    let mut input = FrameInput::test_grid(2, 1, "AB");
    let atlas = atlas_with(&['A', 'B']);

    // Visible underline cursor at col 0.
    input.content.cursor.visible = true;
    input.content.cursor.shape = CursorShape::Underline;
    input.content.cursor.line = 0;
    input.content.cursor.column = Column(0);

    input.selection = Some(selection_range(0, 0, 0));

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let selected_bg = rgb_f32(Rgb {
        r: 211,
        g: 215,
        b: 207,
    });

    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(
        bg0.bg_color, selected_bg,
        "underline cursor should not block selection inversion"
    );
}

// ── Hyperlink underline tests ──

/// Build a 1×1 hyperlink cell (no explicit underline flags).
fn frame_with_hyperlink() -> FrameInput {
    let mut input = FrameInput::test_grid(1, 1, "A");
    input.content.cells[0].has_hyperlink = true;
    input.content.cursor.visible = false;
    input
}

#[test]
fn hyperlink_not_hovered_emits_dotted_underline() {
    let input = frame_with_hyperlink();
    let atlas = empty_atlas();
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Dotted underline fallback: cell_width=8, step_by(2) → 4 dots.
    assert_eq!(
        decoration_bg_count(&frame),
        4,
        "hyperlink (not hovered) should emit dotted underline rects",
    );
}

#[test]
fn hyperlink_hovered_emits_solid_underline() {
    let mut input = frame_with_hyperlink();
    input.hovered_cell = Some((0, 0));
    let atlas = empty_atlas();
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Solid underline: 1 rect.
    assert_eq!(
        decoration_bg_count(&frame),
        1,
        "hyperlink (hovered) should emit single solid underline rect",
    );

    // Verify geometry matches a single underline.
    let ul = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(ul.size, (8.0, 1.0));
}

#[test]
fn hyperlink_hovered_uses_fg_color() {
    let mut input = frame_with_hyperlink();
    input.hovered_cell = Some((0, 0));
    let fg = input.content.cells[0].fg;
    let atlas = empty_atlas();
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let ul = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(ul.bg_color, rgb_f32(fg));
}

#[test]
fn hyperlink_with_explicit_underline_uses_explicit_style() {
    // When a cell has both a hyperlink and an explicit SGR underline,
    // the explicit underline takes priority — no dotted link decoration.
    let mut input = FrameInput::test_grid(1, 1, "A");
    input.content.cells[0].has_hyperlink = true;
    input.content.cells[0].flags = CellFlags::UNDERLINE;
    input.content.cursor.visible = false;

    let atlas = empty_atlas();
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Only the explicit single underline (1 rect), not the dotted link decoration.
    assert_eq!(
        decoration_bg_count(&frame),
        1,
        "explicit underline should override hyperlink decoration",
    );
}

#[test]
fn non_hyperlink_cell_no_extra_decorations() {
    // Verify that a plain cell without hyperlink or underline flags produces
    // no decoration instances — baseline sanity check.
    let input = FrameInput::test_grid(1, 1, "A");
    let atlas = empty_atlas();
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(decoration_bg_count(&frame), 0);
}

// ── Viewport / coordinate system alignment ──
//
// The shader maps pixel positions to NDC: ndc = pos / screen_size * 2 - 1.
// screen_size comes from FrameInput.viewport. Cell positions come from
// origin + col * cell_width. For cells to fill the viewport correctly,
// viewport and cell positions must be in the same coordinate system.

#[test]
fn cells_fill_viewport_when_viewport_matches_cell_units() {
    // 10 cols × 2 rows, cell = 8×16. Default viewport = 80×32 = 10*8 × 2*16.
    let input = FrameInput::test_grid(10, 2, "ABCDEFGHIJKLMNOPQRST");
    let atlas = atlas_with(&[
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T',
    ]);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Last cell in row 0 at col 9: x = 9*8 = 72, right edge = 72+8 = 80.
    let last_bg = nth_instance(frame.backgrounds.as_bytes(), 9);
    let right_edge = last_bg.pos.0 + last_bg.size.0;
    assert_eq!(
        right_edge, frame.viewport.width as f32,
        "cells should fill viewport width"
    );

    // NDC fraction for right edge: 80/80 = 1.0.
    let ndc_frac = right_edge / frame.viewport.width as f32;
    assert!(
        (ndc_frac - 1.0).abs() < 0.001,
        "right edge NDC should be 1.0, got {ndc_frac}",
    );
}

#[test]
fn oversized_viewport_causes_cells_to_underfill() {
    // Demonstrate the bug: physical viewport > logical cell grid.
    // At 1.25x DPI, physical viewport is 100×40 but cells are 10*8 × 2*16 = 80×32.
    let mut input = FrameInput::test_grid(10, 2, "ABCDEFGHIJKLMNOPQRST");
    let atlas = atlas_with(&[
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T',
    ]);

    // Override viewport to physical (larger than cell grid).
    input.viewport = ViewportSize::new(100, 40);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Cell positions are unchanged: right edge still at 80.
    let last_bg = nth_instance(frame.backgrounds.as_bytes(), 9);
    let right_edge = last_bg.pos.0 + last_bg.size.0;
    assert_eq!(
        right_edge, 80.0,
        "cell positions use cell_size, not viewport"
    );

    // NDC fraction: 80/100 = 0.8 — cells only fill 80% of the screen!
    let ndc_frac = right_edge / frame.viewport.width as f32;
    assert!(
        (ndc_frac - 0.8).abs() < 0.001,
        "oversized viewport: cells fill {ndc_frac}, not 1.0",
    );
}

#[test]
fn chrome_origin_aligns_when_viewport_is_logical() {
    // Simulates chrome (caption_height = 46 logical, unified tab bar) with
    // grid below. Viewport must be logical so that chrome and grid NDC match.
    let caption_height = 46.0_f32;
    let scale = 1.25_f32;

    // Logical viewport: 1016×640.
    let logical_h = 640_u32;

    // Chrome bar bottom in NDC (logical coords): 46 / 640 = 0.071875.
    let chrome_bottom_ndc = caption_height / logical_h as f32;

    // Grid origin = caption_height in logical coords.
    let grid_top_ndc = caption_height / logical_h as f32;

    // They match: chrome bottom == grid top.
    assert!(
        (chrome_bottom_ndc - grid_top_ndc).abs() < 0.001,
        "logical viewport: chrome={chrome_bottom_ndc}, grid={grid_top_ndc}",
    );

    // Now demonstrate the mismatch with physical viewport.
    let physical_h = (logical_h as f32 * scale).round() as u32; // 800

    // Chrome draws at physical pixels: 46 * 1.25 = 57.5.
    let chrome_bottom_physical_ndc = (caption_height * scale) / physical_h as f32;
    // Grid origin in logical: 46 / 800 = 0.0575.
    let grid_top_physical_ndc = caption_height / physical_h as f32;

    // Mismatch: chrome (0.071875) > grid (0.0575) — grid starts ABOVE chrome!
    assert!(
        chrome_bottom_physical_ndc > grid_top_physical_ndc,
        "physical viewport mismatch: chrome={chrome_bottom_physical_ndc}, grid={grid_top_physical_ndc}",
    );
}

#[test]
fn origin_with_logical_viewport_fills_grid_area() {
    // After chrome: grid starts at y=caption_height (unified tab bar),
    // viewport is logical. Cells should fill from caption to bottom.
    let caption_height = 46.0_f32;
    let cell_h = 16.0_f32;
    let logical_h = 640_u32;
    let grid_h = logical_h as f32 - caption_height; // 594
    let rows = (grid_h / cell_h).floor() as usize; // 37

    let mut input = FrameInput::test_grid(10, rows, "");
    input.viewport = ViewportSize::new(80, logical_h);

    let atlas = empty_atlas();
    let frame = prepare_frame(&input, &atlas, (0.0, caption_height));

    // First row starts at origin y.
    let first_bg = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(
        first_bg.pos.1, caption_height,
        "first row at caption height"
    );

    // Last row: y = 46 + 36*16 = 46 + 576 = 622.
    // Bottom edge: 622 + 16 = 638.
    let last_row_idx = (rows - 1) * 10; // First cell of last row
    let last_bg = nth_instance(frame.backgrounds.as_bytes(), last_row_idx);
    let bottom_edge = last_bg.pos.1 + last_bg.size.1;

    // Bottom edge (638) < viewport (640): grid doesn't quite reach bottom
    // (because 594/16 = 37.125, we only get 37 rows). This is normal —
    // there's a small gap at the bottom. But it's close.
    assert!(bottom_edge <= logical_h as f32, "grid fits within viewport");
    assert!(
        bottom_edge > logical_h as f32 - cell_h,
        "grid fills most of viewport: bottom={bottom_edge}, viewport={logical_h}",
    );
}

// ── Ligature + selection interaction (Section 6.5) ──

#[test]
fn shaped_ligature_selection_col1_does_not_duplicate_glyph() {
    // A 2-column ligature (glyph 100 at cols 0-1) with selection covering
    // only col 1. The glyph must be emitted exactly once at col 0.
    // Selection highlighting applies per-cell to backgrounds independently.
    let size_q6 = 768;
    let mut input = FrameInput::test_grid(3, 1, "fi ");
    input.content.cells[0].ch = 'f';
    input.content.cells[1].ch = 'i';
    input.content.cursor.visible = false;

    let atlas = key_atlas_with(&[100], size_q6);
    let glyphs = vec![ShapedGlyph {
        glyph_id: 100,
        face_index: 0,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let col_starts = vec![0]; // ligature starts at col 0
    let shaped = shaped_one_row(3, &glyphs, &col_starts, size_q6);

    // Select only col 1 (the continuation column of the ligature).
    input.selection = Some(selection_range(0, 1, 1));

    let frame = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    // 3 bg instances (one per cell), still only 1 fg instance (ligature glyph).
    assert_counts(&frame, 3, 1, 0);

    // Col 0 (unselected) should have normal bg.
    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    let normal_bg = rgb_f32(input.content.cells[0].bg);
    assert_eq!(bg0.bg_color, normal_bg, "col 0 should have normal bg");

    // Col 1 (selected continuation) should have inverted bg.
    let bg1 = nth_instance(frame.backgrounds.as_bytes(), 1);
    let selected_bg = rgb_f32(Rgb {
        r: 211,
        g: 215,
        b: 207,
    });
    assert_eq!(
        bg1.bg_color, selected_bg,
        "col 1 (ligature continuation) should have selected bg"
    );

    // Col 2 (space, unselected) should have normal bg.
    let bg2 = nth_instance(frame.backgrounds.as_bytes(), 2);
    assert_eq!(bg2.bg_color, normal_bg, "col 2 should have normal bg");
}

// ── fg_dim dimming ──

#[test]
fn fg_dim_default_alpha_is_one() {
    let input = FrameInput::test_grid(1, 1, "A");
    let atlas = atlas_with(&['A']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    // fg_color[3] is the alpha component — default fg_dim=1.0.
    assert_eq!(
        fg.fg_color[3], 1.0,
        "default fg_dim should produce alpha 1.0"
    );
}

#[test]
fn fg_dim_reduces_glyph_alpha() {
    let mut input = FrameInput::test_grid(1, 1, "A");
    input.fg_dim = 0.7;
    input.content.cursor.visible = false;
    let atlas = atlas_with(&['A']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    assert!(
        (fg.fg_color[3] - 0.7).abs() < 0.001,
        "fg_dim=0.7 should produce alpha ~0.7, got {}",
        fg.fg_color[3],
    );
}

// ── Multi-pane instance accumulation ──

#[test]
fn fill_frame_shaped_accumulates_without_clearing() {
    use super::fill_frame_shaped;

    let input_a = FrameInput::test_grid(2, 1, "AB");
    let input_b = FrameInput::test_grid(2, 1, "CD");
    let atlas = empty_atlas();

    // Shape empty frames (no glyph hits, but backgrounds still accumulate).
    let shaped_a = ShapedFrame::new(2, 0);
    let shaped_b = ShapedFrame::new(2, 0);

    let mut frame = PreparedFrame::new(ViewportSize::new(32, 16), Rgb { r: 0, g: 0, b: 0 }, 1.0);

    // First fill: pane A at origin (0,0).
    fill_frame_shaped(&input_a, &atlas, &shaped_a, &mut frame, (0.0, 0.0), 1.0);
    let count_after_a = frame.backgrounds.len();

    // Second fill: pane B at origin (16,0) — appends, does NOT clear.
    fill_frame_shaped(&input_b, &atlas, &shaped_b, &mut frame, (16.0, 0.0), 0.0);
    let count_after_b = frame.backgrounds.len();

    assert_eq!(count_after_a, 2, "pane A should produce 2 bg instances");
    assert_eq!(
        count_after_b, 4,
        "pane B should append 2 more, total 4 bg instances"
    );
}

#[test]
fn two_panes_at_correct_offsets() {
    use super::fill_frame_shaped;

    let input_a = FrameInput::test_grid(1, 1, "A");
    let input_b = FrameInput::test_grid(1, 1, "B");
    let atlas = empty_atlas();
    let shaped = ShapedFrame::new(1, 0);

    let mut frame = PreparedFrame::new(ViewportSize::new(16, 16), Rgb { r: 0, g: 0, b: 0 }, 1.0);

    // Pane A at (0, 0).
    fill_frame_shaped(&input_a, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    // Pane B at (400, 0).
    fill_frame_shaped(&input_b, &atlas, &shaped, &mut frame, (400.0, 0.0), 0.0);

    // Pane A background at x=0.
    let bg_a = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg_a.pos.0, 0.0, "pane A bg should be at x=0");

    // Pane B background at x=400.
    let bg_b = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(bg_b.pos.0, 400.0, "pane B bg should be at x=400");
}

#[test]
fn lower_pane_origin_is_not_culled_by_local_pane_height() {
    use super::fill_frame_shaped;

    let size_q6 = 768;
    let glyphs = vec![ShapedGlyph {
        glyph_id: 42,
        face_index: 0,
        synthetic: 0,
        x_advance: 8.0,
        x_offset: 0.0,
        y_offset: 0.0,
    }];
    let col_starts = vec![0];
    let shaped = shaped_one_row(1, &glyphs, &col_starts, size_q6);
    let atlas = key_atlas_with(&[42], size_q6);

    let mut input = FrameInput::test_grid(1, 1, "A");
    input.viewport = ViewportSize::new(8, 16);
    input.content.cursor.visible = false;

    let mut frame = PreparedFrame::new(ViewportSize::new(800, 600), Rgb { r: 0, g: 0, b: 0 }, 1.0);

    // Simulate a lower split pane: pane-local viewport is one row tall,
    // but the pane origin is well below that in window coordinates.
    fill_frame_shaped(&input, &atlas, &shaped, &mut frame, (0.0, 200.0), 0.0);

    assert_eq!(
        frame.glyphs.len(),
        1,
        "lower pane glyph should still render"
    );
    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(fg.pos.1, 200.0, "pane origin y should be preserved");
}

#[test]
fn cursor_only_in_focused_pane() {
    use super::fill_frame_shaped;

    let input_focused = FrameInput::test_grid(1, 1, "A");
    let mut input_unfocused = FrameInput::test_grid(1, 1, "B");
    input_unfocused.content.cursor.visible = true;

    let atlas = empty_atlas();
    let shaped = ShapedFrame::new(1, 0);

    let mut frame = PreparedFrame::new(ViewportSize::new(16, 16), Rgb { r: 0, g: 0, b: 0 }, 1.0);

    // Focused pane: cursor_opacity = 1.0 (fully visible).
    fill_frame_shaped(&input_focused, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    let cursor_after_focused = frame.cursors.len();

    // Unfocused pane: cursor_opacity = 0.0 (hidden).
    fill_frame_shaped(
        &input_unfocused,
        &atlas,
        &shaped,
        &mut frame,
        (100.0, 0.0),
        0.0,
    );
    let cursor_after_unfocused = frame.cursors.len();

    assert_eq!(
        cursor_after_focused, 1,
        "focused pane should emit 1 cursor instance"
    );
    assert_eq!(
        cursor_after_unfocused, 1,
        "unfocused pane should not add more cursor instances"
    );
}

// ── Search match highlighting ──

/// Helper: build a `FrameSearch` with a single match at the given viewport
/// position (`line`, `start_col..=end_col`) with `focused` as the match index.
fn search_with_match(
    line: usize,
    start_col: usize,
    end_col: usize,
    focused: usize,
) -> crate::gpu::frame_input::FrameSearch {
    use oriterm_core::SearchMatch;

    let m = SearchMatch {
        start_row: StableRowIndex(line as u64),
        start_col,
        end_row: StableRowIndex(line as u64),
        end_col,
    };
    crate::gpu::frame_input::FrameSearch::for_test(vec![m], focused, 0)
}

#[test]
fn search_match_highlights_bg() {
    // A non-focused search match should use SEARCH_MATCH_BG for the bg
    // and keep the original fg.
    let match_bg = Rgb {
        r: 100,
        g: 100,
        b: 30,
    };

    let mut input = FrameInput::test_grid(3, 1, "ABC");
    // Match on col 1 only, focused index out of range → no focused match.
    input.search = Some(search_with_match(0, 1, 1, 99));
    input.content.cursor.visible = false;
    let atlas = atlas_with(&['A', 'B', 'C']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Col 0: normal bg.
    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg0.bg_color, rgb_f32(input.content.cells[0].bg));

    // Col 1: search match bg.
    let bg1 = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(
        bg1.bg_color,
        rgb_f32(match_bg),
        "match bg should be yellow-tinted"
    );

    // Col 2: normal bg.
    let bg2 = nth_instance(frame.backgrounds.as_bytes(), 2);
    assert_eq!(bg2.bg_color, rgb_f32(input.content.cells[2].bg));
}

#[test]
fn search_match_preserves_fg() {
    // Non-focused match keeps the cell's original fg color.
    let mut input = FrameInput::test_grid(1, 1, "A");
    input.search = Some(search_with_match(0, 0, 0, 99));
    input.content.cursor.visible = false;
    let fg = input.content.cells[0].fg;
    let atlas = atlas_with(&['A']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let glyph = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(
        glyph.fg_color,
        rgb_f32(fg),
        "non-focused match keeps original fg"
    );
}

#[test]
fn search_focused_match_overrides_fg_and_bg() {
    // The focused match uses SEARCH_FOCUSED_FG and SEARCH_FOCUSED_BG.
    let focused_fg = Rgb { r: 0, g: 0, b: 0 };
    let focused_bg = Rgb {
        r: 200,
        g: 170,
        b: 40,
    };

    let mut input = FrameInput::test_grid(1, 1, "A");
    input.search = Some(search_with_match(0, 0, 0, 0)); // focused index = 0
    input.content.cursor.visible = false;
    let atlas = atlas_with(&['A']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let bg = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg.bg_color, rgb_f32(focused_bg), "focused match bg");

    let glyph = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(
        glyph.fg_color,
        rgb_f32(focused_fg),
        "focused match fg should be dark"
    );
}

#[test]
fn search_match_skips_block_cursor_cell() {
    // The cell under a visible block cursor should NOT get search
    // highlighting — the cursor overlay handles its own visual.
    let mut input = FrameInput::test_grid(3, 1, "ABC");
    input.search = Some(search_with_match(0, 0, 2, 99));
    // Block cursor at col 0.
    input.content.cursor.column = Column(0);
    input.content.cursor.line = 0;
    input.content.cursor.shape = CursorShape::Block;
    input.content.cursor.visible = true;
    let atlas = atlas_with(&['A', 'B', 'C']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let match_bg = Rgb {
        r: 100,
        g: 100,
        b: 30,
    };

    // Col 0 (under block cursor): normal bg, NOT match bg.
    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_ne!(
        bg0.bg_color,
        rgb_f32(match_bg),
        "block cursor cell should skip search highlighting"
    );

    // Col 1 (not under cursor): match bg.
    let bg1 = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(
        bg1.bg_color,
        rgb_f32(match_bg),
        "non-cursor cell should be highlighted"
    );
}

#[test]
fn search_no_match_uses_default_colors() {
    // When search is active but no cells match, colors are unchanged.
    let mut input = FrameInput::test_grid(2, 1, "AB");
    // Match on row 5 (not in our 1-row grid).
    input.search = Some(search_with_match(5, 0, 0, 0));
    input.content.cursor.visible = false;
    let atlas = atlas_with(&['A', 'B']);
    let cell_bg = input.content.cells[0].bg;

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg0.bg_color, rgb_f32(cell_bg));
    let bg1 = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(bg1.bg_color, rgb_f32(cell_bg));
}

// ── URL hover underline ──

#[test]
fn url_hover_produces_cursor_layer_underline() {
    // Hovering a URL should produce cursor-layer underline rects.
    let mut input = FrameInput::test_grid(10, 1, "");
    // URL spans cols 2..5 on line 0.
    input.hovered_url_segments = vec![(0, 2, 5)];
    input.content.cursor.visible = false;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // 1 cursor instance for the URL underline (no terminal cursor).
    assert_eq!(frame.cursors.len(), 1, "should have 1 URL underline rect");

    let ul = nth_instance(frame.cursors.as_bytes(), 0);
    // x = 2 * 8.0 = 16.0
    assert_eq!(ul.pos.0, 16.0);
    // w = (5 - 2 + 1) * 8.0 = 32.0
    assert_eq!(ul.size.0, 32.0);
    // h = stroke_size = 1.0
    assert_eq!(ul.size.1, 1.0);
}

#[test]
fn url_hover_multiple_segments() {
    // A URL wrapping across lines produces multiple segments.
    let mut input = FrameInput::test_grid(10, 3, "");
    input.hovered_url_segments = vec![(0, 5, 9), (1, 0, 9), (2, 0, 3)];
    input.content.cursor.visible = false;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.cursors.len(), 3, "3 URL underline segments");
}

#[test]
fn url_hover_empty_segments_no_extra_instances() {
    // No hovered URL → no extra cursor instances.
    let mut input = FrameInput::test_grid(10, 1, "");
    input.hovered_url_segments = Vec::new();
    input.content.cursor.visible = false;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.cursors.len(), 0, "no URL hover → no cursor instances");
}

#[test]
fn url_hover_with_origin_offset() {
    // URL underline positions should respect the origin offset.
    let mut input = FrameInput::test_grid(10, 1, "");
    input.hovered_url_segments = vec![(0, 0, 2)];
    input.content.cursor.visible = false;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (50.0, 100.0));

    let ul = nth_instance(frame.cursors.as_bytes(), 0);
    // x = 50.0 + 0 * 8.0 = 50.0
    assert_eq!(ul.pos.0, 50.0);
    // y includes origin offset + underline position.
    assert!(ul.pos.1 > 100.0, "y should be offset from origin");
}

// ── Mark cursor override ──

#[test]
fn mark_cursor_overrides_terminal_cursor() {
    // When mark_cursor is set, it should override the terminal cursor position
    // and shape (HollowBlock).
    let mut input = FrameInput::test_grid(10, 5, "");
    // Terminal cursor at (0, 0) as Block.
    input.content.cursor.column = Column(0);
    input.content.cursor.line = 0;
    input.content.cursor.shape = CursorShape::Block;
    input.content.cursor.visible = true;
    // Mark cursor at (3, 5) as HollowBlock.
    input.mark_cursor = Some(crate::gpu::frame_input::MarkCursorOverride {
        line: 3,
        column: Column(5),
        shape: CursorShape::HollowBlock,
    });
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // HollowBlock = 4 cursor instances (top, bottom, left, right).
    assert_eq!(frame.cursors.len(), 4);

    // All 4 edges should be around col 5, row 3.
    let top = nth_instance(frame.cursors.as_bytes(), 0);
    assert_eq!(top.pos, (40.0, 48.0)); // col 5 * 8 = 40, row 3 * 16 = 48
}

#[test]
fn mark_cursor_none_uses_terminal_cursor() {
    // When mark_cursor is None, the terminal cursor is used.
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.cursor.column = Column(7);
    input.content.cursor.line = 2;
    input.content.cursor.shape = CursorShape::Block;
    input.content.cursor.visible = true;
    input.mark_cursor = None;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.cursors.len(), 1);

    let c = nth_instance(frame.cursors.as_bytes(), 0);
    assert_eq!(c.pos, (56.0, 32.0)); // col 7 * 8 = 56, row 2 * 16 = 32
}

#[test]
fn mark_cursor_is_always_visible() {
    // Mark cursor overrides visibility — it's always rendered even if the
    // terminal cursor is hidden.
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.cursor.visible = false; // terminal cursor hidden
    input.mark_cursor = Some(crate::gpu::frame_input::MarkCursorOverride {
        line: 1,
        column: Column(3),
        shape: CursorShape::HollowBlock,
    });
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // HollowBlock = 4 cursor instances.
    assert_eq!(
        frame.cursors.len(),
        4,
        "mark cursor should render even when terminal cursor is hidden"
    );
}

// ── Explicit selection colors ──

#[test]
fn selection_explicit_colors_override_inversion() {
    // When palette.selection_fg and palette.selection_bg are set,
    // selected cells use those colors instead of fg/bg inversion.
    let sel_fg = Rgb {
        r: 255,
        g: 255,
        b: 255,
    };
    let sel_bg = Rgb {
        r: 58,
        g: 61,
        b: 92,
    };

    let mut input = FrameInput::test_grid(3, 1, "ABC");
    input.palette.selection_fg = Some(sel_fg);
    input.palette.selection_bg = Some(sel_bg);
    input.selection = Some(selection_range(0, 1, 1));
    input.content.cursor.visible = false;
    let atlas = atlas_with(&['A', 'B', 'C']);

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Col 1 (selected): should use explicit selection colors.
    let bg1 = nth_instance(frame.backgrounds.as_bytes(), 1);
    assert_eq!(bg1.bg_color, rgb_f32(sel_bg), "explicit selection bg");

    let fg1 = nth_instance(frame.glyphs.as_bytes(), 1);
    assert_eq!(fg1.fg_color, rgb_f32(sel_fg), "explicit selection fg");

    // Col 0 (not selected): normal colors.
    let bg0 = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg0.bg_color, rgb_f32(input.content.cells[0].bg));
}

// ── Empty cells still produce bg instances ──

#[test]
fn null_char_cell_produces_bg_only() {
    // A cell with '\0' should produce a BG instance but no FG instance,
    // same as a space cell.
    let mut input = FrameInput::test_grid(2, 1, "");
    input.content.cells[0].ch = '\0';
    input.content.cells[1].ch = '\0';
    input.content.cursor.visible = false;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(
        frame.backgrounds.len(),
        2,
        "2 bg instances for 2 null cells"
    );
    assert_eq!(frame.glyphs.len(), 0, "no fg instances for null cells");
}

#[test]
fn cells_with_custom_bg_produce_bg_instances() {
    // Cells that are spaces but have non-default background should still
    // produce BG instances with the correct color.
    let mut input = FrameInput::test_grid(3, 1, "");
    let custom_bg = Rgb {
        r: 100,
        g: 50,
        b: 200,
    };
    for cell in &mut input.content.cells {
        cell.bg = custom_bg;
    }
    input.content.cursor.visible = false;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.backgrounds.len(), 3);
    for i in 0..3 {
        let bg = nth_instance(frame.backgrounds.as_bytes(), i);
        assert_eq!(
            bg.bg_color,
            rgb_f32(custom_bg),
            "cell {i} should have custom bg color",
        );
    }
}

// ── Zero-size viewport ──

#[test]
fn zero_cols_zero_rows_produces_empty_frame() {
    let mut input = FrameInput::test_grid(0, 0, "");
    input.content.cursor.visible = false;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.backgrounds.len(), 0);
    assert_eq!(frame.glyphs.len(), 0);
    assert_eq!(frame.cursors.len(), 0);
}

#[test]
fn zero_cols_nonzero_rows_produces_empty_frame() {
    let mut input = FrameInput::test_grid(0, 5, "");
    input.content.cursor.visible = false;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.backgrounds.len(), 0);
    assert_eq!(frame.glyphs.len(), 0);
}

#[test]
fn nonzero_cols_zero_rows_produces_empty_frame() {
    let mut input = FrameInput::test_grid(80, 0, "");
    input.content.cursor.visible = false;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.backgrounds.len(), 0);
    assert_eq!(frame.glyphs.len(), 0);
}

// ── Prompt marker tests ──

#[test]
fn prompt_markers_emit_cursor_rects() {
    let mut input = FrameInput::test_grid(4, 3, "");
    input.content.cursor.visible = false;
    input.prompt_marker_rows = vec![0, 2];
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // Two prompt marker bars should appear in the cursor layer.
    assert_eq!(frame.cursors.len(), 2, "expected 2 prompt marker rects");
}

#[test]
fn prompt_markers_empty_emits_no_rects() {
    let mut input = FrameInput::test_grid(4, 3, "");
    input.content.cursor.visible = false;
    input.prompt_marker_rows = Vec::new();
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(
        frame.cursors.len(),
        0,
        "no prompt markers = no cursor rects"
    );
}

#[test]
fn prompt_markers_with_origin_offset() {
    let mut input = FrameInput::test_grid(4, 3, "");
    input.content.cursor.visible = false;
    input.prompt_marker_rows = vec![1];
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (10.0, 20.0));

    // One marker rect at row 1 with origin offset applied.
    assert_eq!(frame.cursors.len(), 1, "expected 1 prompt marker rect");
}

// ── Image z-index splitting ──

fn placement(z: i32, x: f32, y: f32) -> oriterm_core::RenderablePlacement {
    oriterm_core::RenderablePlacement {
        image_id: oriterm_core::image::ImageId::from_raw(1),
        viewport_x: x,
        viewport_y: y,
        display_width: 32.0,
        display_height: 32.0,
        source_x: 0.0,
        source_y: 0.0,
        source_w: 1.0,
        source_h: 1.0,
        z_index: z,
        opacity: 1.0,
    }
}

#[test]
fn image_z_negative_goes_to_below_list() {
    let mut input = FrameInput::test_grid(4, 2, "");
    input.content.cursor.visible = false;
    input.content.images = vec![placement(-1, 0.0, 0.0)];
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.image_quads_below.len(), 1);
    assert_eq!(frame.image_quads_above.len(), 0);
    assert_eq!(frame.image_quads_below[0].x, 0.0);
}

#[test]
fn image_z_zero_goes_to_above_list() {
    let mut input = FrameInput::test_grid(4, 2, "");
    input.content.cursor.visible = false;
    input.content.images = vec![placement(0, 10.0, 20.0)];
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.image_quads_below.len(), 0);
    assert_eq!(frame.image_quads_above.len(), 1);
    assert_eq!(frame.image_quads_above[0].x, 10.0);
}

#[test]
fn image_z_positive_goes_to_above_list() {
    let mut input = FrameInput::test_grid(4, 2, "");
    input.content.cursor.visible = false;
    input.content.images = vec![placement(5, 0.0, 0.0)];
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.image_quads_below.len(), 0);
    assert_eq!(frame.image_quads_above.len(), 1);
}

#[test]
fn mixed_z_images_split_correctly() {
    let mut input = FrameInput::test_grid(4, 2, "");
    input.content.cursor.visible = false;
    input.content.images = vec![
        placement(-2, 0.0, 0.0),
        placement(1, 10.0, 0.0),
        placement(-1, 20.0, 0.0),
        placement(0, 30.0, 0.0),
    ];
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    assert_eq!(frame.image_quads_below.len(), 2, "z<0 images");
    assert_eq!(frame.image_quads_above.len(), 2, "z>=0 images");
}

#[test]
fn image_origin_offset_applied() {
    let mut input = FrameInput::test_grid(4, 2, "");
    input.content.cursor.visible = false;
    input.content.images = vec![placement(-1, 5.0, 10.0)];
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (100.0, 200.0));

    let q = &frame.image_quads_below[0];
    assert_eq!(q.x, 105.0, "origin x added to viewport_x");
    assert_eq!(q.y, 210.0, "origin y added to viewport_y");
}

#[test]
fn image_uv_and_opacity_propagated() {
    let mut input = FrameInput::test_grid(4, 2, "");
    input.content.cursor.visible = false;
    let mut img = placement(0, 0.0, 0.0);
    img.source_x = 0.25;
    img.source_y = 0.5;
    img.source_w = 0.5;
    img.source_h = 0.25;
    img.opacity = 0.8;
    input.content.images = vec![img];
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let q = &frame.image_quads_above[0];
    assert_eq!(q.uv_x, 0.25);
    assert_eq!(q.uv_y, 0.5);
    assert_eq!(q.uv_w, 0.5);
    assert_eq!(q.uv_h, 0.25);
    assert_eq!(q.opacity, 0.8);
}

// ── Incremental vs full rebuild equivalence ──

/// Build a multi-row ShapedFrame where each cell gets its own glyph.
fn shaped_multi_row(
    cols: usize,
    rows: usize,
    glyph_id_base: u16,
    size_q6: u32,
) -> (ShapedFrame, Vec<u16>) {
    let mut sf = ShapedFrame::new(cols, size_q6);
    let mut all_ids = Vec::new();
    for row in 0..rows {
        let glyphs: Vec<ShapedGlyph> = (0..cols)
            .map(|c| {
                let id = glyph_id_base + (row * cols + c) as u16;
                all_ids.push(id);
                ShapedGlyph {
                    glyph_id: id,
                    face_index: 0,
                    synthetic: 0,
                    x_advance: 0.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                }
            })
            .collect();
        let col_starts: Vec<usize> = (0..cols).collect();
        let mut col_map = Vec::new();
        crate::font::build_col_glyph_map(&col_starts, cols, &mut col_map);
        sf.push_row(&glyphs, &col_starts, &col_map);
    }
    (sf, all_ids)
}

/// Regression: BUG-06-027 — three-frame sequence proves both that the
/// production fix populates `saved_tier` (Frame 1 dispatches incremental)
/// AND that the `!all_dirty` guard remains load-bearing (Frame 2 with
/// `all_dirty=true` dispatches full-rebuild even when `saved_tier` is
/// populated).
///
/// See: bug-tracker/plans/BUG-06-027/
#[test]
fn incremental_all_dirty_matches_full_rebuild() {
    let size_q6 = 768;
    let cols = 4;
    let rows = 3;
    let text: String = std::iter::repeat_n('A', cols * rows).collect();
    let mut input = FrameInput::test_grid(cols, rows, &text);

    let (shaped, ids) = shaped_multi_row(cols, rows, 10, size_q6);
    let atlas = key_atlas_with(&ids, size_q6);

    // Full rebuild (fresh frame, used as the negative-pin baseline).
    let fresh = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));

    // Frame 0: full rebuild — production save_terminal_tier publishes
    // empty tier into saved_tier (no-op the first time).
    let mut frame = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(
        !frame.was_incremental,
        "Frame 0 always full-rebuild (saved_tier empty before first prepare)"
    );

    // Frame 1: with all_dirty=false, must take the incremental path.
    // This proves the production fix's pre-dispatch save_terminal_tier
    // populated saved_tier from Frame 0's terminal-tier.
    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(
        frame.was_incremental,
        "Frame 1 must dispatch to incremental (proves production fix populated saved_tier)"
    );
    assert!(
        frame.saved_tier.has_cached_rows(),
        "saved_tier must hold prev-frame data for the all_dirty fallback assertion to be meaningful"
    );

    // Frame 2: all_dirty=true must dispatch back to full-rebuild EVEN
    // THOUGH saved_tier is populated.
    input.content.all_dirty = true;
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(
        !frame.was_incremental,
        "all_dirty=true must dispatch to full-rebuild even when saved_tier is populated \
         (the !all_dirty guard remains load-bearing)"
    );

    // Output equivalence: the full-rebuild branch produces output identical
    // to a fresh prepare on the same input (rebuild is idempotent).
    assert_eq!(
        fresh.backgrounds.as_bytes(),
        frame.backgrounds.as_bytes(),
        "backgrounds should match"
    );
    assert_eq!(
        fresh.glyphs.as_bytes(),
        frame.glyphs.as_bytes(),
        "glyphs should match"
    );
}

#[test]
fn incremental_no_dirty_rows_matches_cached() {
    // When no rows are dirty (all clean), the incremental path copies
    // all instances from the cached tier — result should match the
    // original full rebuild.
    let size_q6 = 768;
    let cols = 4;
    let rows = 3;
    let text: String = std::iter::repeat_n('A', cols * rows).collect();
    let mut input = FrameInput::test_grid(cols, rows, &text);

    let (shaped, ids) = shaped_multi_row(cols, rows, 10, size_q6);
    let atlas = key_atlas_with(&ids, size_q6);

    // First pass: full rebuild populates row_ranges.
    let mut frame = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);

    // Capture the full rebuild output.
    let full_bg = frame.backgrounds.as_bytes().to_vec();
    let full_fg = frame.glyphs.as_bytes().to_vec();

    // Second pass: no damage, no cursor visible → all rows clean.
    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);

    // Regression anchor: second prepare must hit the incremental path.
    // Without the unconditional pre-dispatch save_terminal_tier,
    // saved_tier would only populate inside the incremental branch itself
    // (chicken-and-egg) and this dispatch would always fall to
    // full-rebuild — the output-equivalence assertions below would pass
    // vacuously because both passes were full-rebuild.
    assert!(
        frame.was_incremental,
        "second pass must hit the incremental path (production dispatch \
         should resolve can_incremental=true with saved_tier populated)"
    );

    assert_eq!(
        full_bg,
        frame.backgrounds.as_bytes(),
        "clean rows should copy cached backgrounds (replay matches full rebuild)"
    );
    assert_eq!(
        full_fg,
        frame.glyphs.as_bytes(),
        "clean rows should copy cached glyphs (replay matches full rebuild)"
    );
}

/// Regression: BUG-06-025 — chrome and overlay tier buffers
/// (`ui_rects`, `ui_glyphs`, `ui_subpixel_glyphs`, `ui_color_glyphs`,
/// `overlay_*`) accumulated frame-after-frame on the incremental prepare
/// path. Production wires `chrome::render_chrome` AFTER `prepare()` and
/// appends fresh chrome instances each frame; without this clear the
/// buffer grew unbounded and stale glyphs from prior frames remained
/// visible whenever chrome content shrank (e.g., shorter tab title after
/// OSC 0/2 updates during high-throughput PTY output like /commit-push).
/// The fix routes the incremental path through the SSOT helper
/// `clear_ephemeral_tiers()` so the chrome+overlay clear contract matches
/// the cursor-blink fast path.
#[test]
fn incremental_clears_chrome_and_overlay_buffers() {
    use crate::gpu::instance_writer::ScreenRect;

    let size_q6 = 768;
    let cols = 4;
    let rows = 3;
    let text: String = std::iter::repeat_n('A', cols * rows).collect();
    let mut input = FrameInput::test_grid(cols, rows, &text);

    let (shaped, ids) = shaped_multi_row(cols, rows, 10, size_q6);
    let atlas = key_atlas_with(&ids, size_q6);

    let mut frame = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);

    // Frame N: full rebuild populates row_ranges. The production
    // pre-dispatch `save_terminal_tier()` swaps Frame N's terminal-tier
    // into saved_tier on the NEXT prepare, so Frame N+1 sees populated
    // saved_tier and dispatches to the incremental path. No explicit
    // test-side swap needed.
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);

    // Simulate post-prepare chrome + overlay rendering — production calls
    // `chrome::render_chrome` after `prepare()` returns, appending into
    // these tiers.
    let rect = ScreenRect {
        x: 0.0,
        y: 0.0,
        w: 10.0,
        h: 10.0,
    };
    let bg = Rgb { r: 0, g: 0, b: 0 };
    frame.ui_rects.push_ui_rect(
        rect,
        [1.0; 4],
        [0.0; 4],
        [0.0; 4],
        [[0.0; 4]; 4],
        [0.0, 0.0, 100.0, 100.0],
    );
    frame.ui_glyphs.push_rect(rect, bg, 1.0);
    frame.ui_subpixel_glyphs.push_rect(rect, bg, 1.0);
    frame.ui_color_glyphs.push_rect(rect, bg, 1.0);
    frame.overlay_rects.push_ui_rect(
        rect,
        [1.0; 4],
        [0.0; 4],
        [0.0; 4],
        [[0.0; 4]; 4],
        [0.0, 0.0, 100.0, 100.0],
    );
    frame.overlay_glyphs.push_rect(rect, bg, 1.0);
    frame.overlay_subpixel_glyphs.push_rect(rect, bg, 1.0);
    frame.overlay_color_glyphs.push_rect(rect, bg, 1.0);

    // Confirm the simulated chrome appended (otherwise the assertions
    // below would pass vacuously).
    assert!(!frame.ui_rects.is_empty(), "ui_rects pre-populated");
    assert!(!frame.ui_glyphs.is_empty(), "ui_glyphs pre-populated");
    assert!(
        !frame.overlay_glyphs.is_empty(),
        "overlay_glyphs pre-populated"
    );

    // Frame N+1: only one row dirty → triggers the incremental path
    // (the path that was missing the chrome/overlay clear).
    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    input.content.damage.push(oriterm_core::DamageLine {
        line: 1,
        left: Column(0),
        right: Column(cols - 1),
    });
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);

    assert!(
        frame.was_incremental,
        "test must hit the incremental path to exercise the fix"
    );

    // Chrome + overlay tiers MUST be cleared by the incremental path so
    // the post-prepare chrome render writes into a fresh buffer.
    assert!(
        frame.ui_rects.is_empty(),
        "ui_rects must be cleared on the incremental path"
    );
    assert!(
        frame.ui_glyphs.is_empty(),
        "ui_glyphs must be cleared on the incremental path"
    );
    assert!(
        frame.ui_subpixel_glyphs.is_empty(),
        "ui_subpixel_glyphs must be cleared on the incremental path"
    );
    assert!(
        frame.ui_color_glyphs.is_empty(),
        "ui_color_glyphs must be cleared on the incremental path"
    );
    assert!(
        frame.overlay_rects.is_empty(),
        "overlay_rects must be cleared on the incremental path"
    );
    assert!(
        frame.overlay_glyphs.is_empty(),
        "overlay_glyphs must be cleared on the incremental path"
    );
    assert!(
        frame.overlay_subpixel_glyphs.is_empty(),
        "overlay_subpixel_glyphs must be cleared on the incremental path"
    );
    assert!(
        frame.overlay_color_glyphs.is_empty(),
        "overlay_color_glyphs must be cleared on the incremental path"
    );
}

/// Regression: BUG-06-025 negative pin — confirms that the chrome and
/// overlay clear contract holds across BOTH non-cursor-blink prepare
/// paths (full rebuild via `out.clear()` AND incremental via
/// `out.clear_ephemeral_tiers()`). If the incremental path ever drops
/// the helper call again, the parity assertion fails before the
/// production symptom (stale chrome glyphs) reaches the renderer.
#[test]
fn full_and_incremental_paths_both_clear_chrome_buffers() {
    use crate::gpu::instance_writer::ScreenRect;

    let size_q6 = 768;
    let cols = 4;
    let rows = 3;
    let text: String = std::iter::repeat_n('A', cols * rows).collect();
    let mut input = FrameInput::test_grid(cols, rows, &text);

    let (shaped, ids) = shaped_multi_row(cols, rows, 10, size_q6);
    let atlas = key_atlas_with(&ids, size_q6);

    let rect = ScreenRect {
        x: 0.0,
        y: 0.0,
        w: 10.0,
        h: 10.0,
    };
    let bg = Rgb { r: 0, g: 0, b: 0 };

    // Path A: full rebuild (saved_tier empty → can_incremental=false →
    // takes `out.clear()` path).
    let mut frame_full = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    frame_full.ui_glyphs.push_rect(rect, bg, 1.0);
    frame_full.overlay_glyphs.push_rect(rect, bg, 1.0);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame_full, (0.0, 0.0), 1.0);
    assert!(
        !frame_full.was_incremental,
        "first prepare without saved_tier MUST take the full-rebuild path"
    );
    assert!(
        frame_full.ui_glyphs.is_empty() && frame_full.overlay_glyphs.is_empty(),
        "full-rebuild path must clear chrome and overlay tiers"
    );

    // Path B: incremental — Frame N seeds, the production fix's
    // pre-dispatch save_terminal_tier publishes saved_tier on Frame N+1.
    let mut frame_inc = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame_inc, (0.0, 0.0), 1.0);
    frame_inc.ui_glyphs.push_rect(rect, bg, 1.0);
    frame_inc.overlay_glyphs.push_rect(rect, bg, 1.0);
    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame_inc, (0.0, 0.0), 1.0);
    assert!(
        frame_inc.was_incremental,
        "second prepare with saved_tier MUST take the incremental path"
    );
    assert!(
        frame_inc.ui_glyphs.is_empty() && frame_inc.overlay_glyphs.is_empty(),
        "incremental path must clear chrome and overlay tiers (parity with full-rebuild path)"
    );
}

// ── Incremental dispatch reachability tests ──

/// Helper: drive a steady-state Frame 0 (full rebuild) populating the
/// production fix's saved_tier swap, returning the frame ready for Frame 1.
fn prepare_frame0_steady(
    cols: usize,
    rows: usize,
) -> (PreparedFrame, FrameInput, ShapedFrame, KeyTestAtlas) {
    let size_q6 = 768;
    let text: String = std::iter::repeat_n('A', cols * rows).collect();
    let input = FrameInput::test_grid(cols, rows, &text);
    let (shaped, ids) = shaped_multi_row(cols, rows, 10, size_q6);
    let atlas = key_atlas_with(&ids, size_q6);
    let mut frame = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    (frame, input, shaped, atlas)
}

/// Regression: BUG-06-027 — Frame 1 dispatches incremental.
///
/// Before the fix, `saved_tier` was populated only INSIDE the incremental
/// branch itself, so the dispatch predicate at `prepare/mod.rs` saw an
/// empty `saved_tier` on every frame and full-rebuild ran every time.
/// Post-fix, the unconditional `out.save_terminal_tier()` at the top of
/// `prepare_frame_shaped_into` publishes Frame N's terminal-tier into
/// `saved_tier`, so Frame N+1's dispatch observes populated `saved_tier`.
#[test]
fn incremental_second_frame_after_full_rebuild_dispatches_incremental() {
    let cols = 4;
    let rows = 3;
    let (mut frame, mut input, shaped, atlas) = prepare_frame0_steady(cols, rows);
    assert!(
        !frame.was_incremental,
        "Frame 0 always full-rebuild (saved_tier empty before first prepare)"
    );

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);

    assert!(
        frame.was_incremental,
        "Frame 1 must dispatch incremental: production fix's pre-dispatch \
         save_terminal_tier populated saved_tier from Frame 0"
    );
    assert!(
        frame.saved_tier.has_cached_rows(),
        "saved_tier holds Frame 0's terminal-tier rows"
    );
}

/// Regression: BUG-06-027 — chained steady-state frames stay incremental.
///
/// Frame 0 full-rebuild → Frame 1 incremental → Frame 2 incremental.
/// Pre-fix the incremental optimization was dormant; post-fix it remains
/// active across consecutive steady-state frames.
#[test]
fn incremental_chained_frames_remain_incremental() {
    let cols = 4;
    let rows = 3;
    let (mut frame, mut input, shaped, atlas) = prepare_frame0_steady(cols, rows);

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();

    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(frame.was_incremental, "Frame 1 incremental");

    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(frame.was_incremental, "Frame 2 incremental");
}

/// Regression: BUG-06-027 — `all_dirty=true` interruption does not
/// permanently break the incremental chain. Frame 0 full-rebuild → Frame 1
/// incremental → Frame 2 `all_dirty=true` full-rebuild → Frame 3 incremental.
/// Pins that the dispatch predicate's `!all_dirty` clause is the only
/// gate, not a sticky disablement.
#[test]
fn incremental_all_dirty_recovery_resumes_incremental() {
    let cols = 4;
    let rows = 3;
    let (mut frame, mut input, shaped, atlas) = prepare_frame0_steady(cols, rows);

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(frame.was_incremental, "Frame 1 incremental");

    input.content.all_dirty = true;
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(!frame.was_incremental, "Frame 2 forced full-rebuild");

    input.content.all_dirty = false;
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(
        frame.was_incremental,
        "Frame 3 must re-enter incremental — all_dirty interruption is not sticky"
    );
}

/// Regression: BUG-06-027 — replay of clean rows produces output identical
/// to a fresh full rebuild across all four terminal-tier buffers
/// (backgrounds + mono glyphs + subpixel glyphs + color glyphs). The
/// equivalence assertion in the existing `incremental_no_dirty_rows_matches_cached`
/// test would pass vacuously if both passes were full-rebuild. This
/// stand-alone test uses a mixed-kind atlas (mono + subpixel + color
/// entries) and `subpixel_positioning = true` to populate every buffer,
/// then asserts `was_incremental` AND non-empty buffers AND output
/// equivalence.
///
/// See: bug-tracker/plans/BUG-06-027/
#[test]
fn incremental_replay_clean_rows_matches_fresh_rebuild_output_across_all_buffers() {
    let size_q6 = 768;
    let cols = 3;
    let rows = 3;
    let text: String = std::iter::repeat_n('A', cols * rows).collect();
    let mut input = FrameInput::test_grid(cols, rows, &text);
    input.subpixel_positioning = true; // route AtlasKind::Subpixel to subpixel_glyphs
    let (shaped, ids) = shaped_multi_row(cols, rows, 10, size_q6);
    let atlas = key_atlas_mixed_kinds(&ids, size_q6);

    let fresh = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));
    assert!(
        !fresh.subpixel_glyphs.is_empty(),
        "fixture must populate subpixel_glyphs for cross-buffer assertion to be non-vacuous"
    );
    assert!(
        !fresh.color_glyphs.is_empty(),
        "fixture must populate color_glyphs for cross-buffer assertion to be non-vacuous"
    );

    let mut frame = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);

    assert!(
        frame.was_incremental,
        "Frame 1 must dispatch incremental for the equivalence assertion below \
         to pin actual replay output (not vacuously equal full-rebuild bytes)"
    );
    assert_eq!(
        fresh.backgrounds.as_bytes(),
        frame.backgrounds.as_bytes(),
        "incremental replay backgrounds match fresh full-rebuild"
    );
    assert_eq!(
        fresh.glyphs.as_bytes(),
        frame.glyphs.as_bytes(),
        "incremental replay mono glyphs match fresh full-rebuild"
    );
    assert_eq!(
        fresh.subpixel_glyphs.as_bytes(),
        frame.subpixel_glyphs.as_bytes(),
        "incremental replay subpixel glyphs match fresh full-rebuild"
    );
    assert_eq!(
        fresh.color_glyphs.as_bytes(),
        frame.color_glyphs.as_bytes(),
        "incremental replay color glyphs match fresh full-rebuild"
    );
}

/// Regression: BUG-06-027 — viewport change forces full-rebuild. Three-frame
/// sequence: Frame 0 full rebuild populates saved_tier; Frame 1 same
/// viewport proves incremental reachable; Frame 2 with changed viewport
/// asserts dispatch falls back to full-rebuild because saved_tier rows
/// were laid out for the old viewport.
#[test]
fn incremental_dispatch_falls_back_on_viewport_change() {
    let size_q6 = 768;
    let cols = 4;
    let rows = 3;
    let text: String = std::iter::repeat_n('A', cols * rows).collect();
    let mut input = FrameInput::test_grid(cols, rows, &text);
    let (shaped, ids) = shaped_multi_row(cols, rows, 10, size_q6);
    let atlas = key_atlas_with(&ids, size_q6);

    let mut frame = PreparedFrame::new(ViewportSize::new(32, 48), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    input.viewport = ViewportSize::new(32, 48);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(!frame.was_incremental, "Frame 0 full rebuild");

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(frame.was_incremental, "Frame 1 reaches incremental path");

    input.viewport = ViewportSize::new(64, 48);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(
        !frame.was_incremental,
        "Frame 2 viewport change must dispatch full-rebuild"
    );
    // Per-buffer equivalence with a fresh rebuild across all four
    // terminal-tier buffers — proves the full rebuild produces correct
    // output despite stale saved_tier.
    let fresh = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));
    assert_eq!(
        fresh.backgrounds.as_bytes(),
        frame.backgrounds.as_bytes(),
        "post-fallback backgrounds match fresh rebuild"
    );
    assert_eq!(
        fresh.glyphs.as_bytes(),
        frame.glyphs.as_bytes(),
        "post-fallback mono glyphs match fresh rebuild"
    );
    assert_eq!(
        fresh.subpixel_glyphs.as_bytes(),
        frame.subpixel_glyphs.as_bytes(),
        "post-fallback subpixel glyphs match fresh rebuild"
    );
    assert_eq!(
        fresh.color_glyphs.as_bytes(),
        frame.color_glyphs.as_bytes(),
        "post-fallback color glyphs match fresh rebuild"
    );
}

/// Regression: BUG-06-027 — content grid topology change (cols × rows)
/// dispatches full-rebuild even when the pixel viewport stays the same.
/// Pixel viewport tracks (width_px, height_px); content grid tracks
/// (cols, rows). During async resize in daemon mode the snapshot grid
/// can race ahead of the pixel viewport — the dispatch fingerprint's
/// content_cols / content_rows hash inputs must catch this without
/// relying on the viewport hash inputs. Three-frame sequence proves
/// incremental is reachable in steady state and the fallback fires on
/// grid topology change in isolation from viewport.
#[test]
fn incremental_dispatch_falls_back_on_content_grid_change() {
    let size_q6 = 768;
    let cols = 4;
    let rows = 3;
    let text: String = std::iter::repeat_n('A', cols * rows).collect();
    let mut input = FrameInput::test_grid(cols, rows, &text);
    let (shaped, ids) = shaped_multi_row(cols, rows, 10, size_q6);
    let atlas = key_atlas_with(&ids, size_q6);

    let mut frame = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(!frame.was_incremental, "Frame 0 full rebuild");

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(frame.was_incremental, "Frame 1 incremental reachable");

    // Bump content_cols WITHOUT changing the pixel viewport. The
    // viewport hash inputs do NOT catch this; only the content_cols
    // hash input does.
    let prev_viewport = input.viewport;
    input.content_cols = cols + 1;
    assert_eq!(input.viewport, prev_viewport, "viewport unchanged");
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(
        !frame.was_incremental,
        "content_cols change must dispatch full-rebuild even when pixel viewport is unchanged"
    );
}

/// Regression: BUG-06-027 — cell-size change (scale or font swap) forces
/// full-rebuild. The dispatch fingerprint hashes all 6 CellMetrics fields
/// so per-cell-layout changes that leave the pixel viewport unchanged
/// still invalidate the saved tier.
#[test]
fn incremental_dispatch_falls_back_on_cell_size_change() {
    let size_q6 = 768;
    let cols = 4;
    let rows = 3;
    let text: String = std::iter::repeat_n('A', cols * rows).collect();
    let mut input = FrameInput::test_grid(cols, rows, &text);
    let (shaped, ids) = shaped_multi_row(cols, rows, 10, size_q6);
    let atlas = key_atlas_with(&ids, size_q6);

    let mut frame = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    input.cell_size = CellMetrics::new(10.0, 20.0, 14.0, 2.0, 1.0, 4.0);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(!frame.was_incremental, "Frame 0 full rebuild");

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(frame.was_incremental, "Frame 1 incremental reachable");

    input.cell_size = CellMetrics::new(20.0, 40.0, 28.0, 4.0, 1.0, 8.0);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(
        !frame.was_incremental,
        "cell_size change must dispatch full-rebuild (saved_tier rows are pixel-positioned for old metrics)"
    );
}

/// Regression: BUG-06-027 — incremental dispatch with partial damage
/// replays clean rows from saved_tier across all four terminal-tier
/// buffers (backgrounds + mono + subpixel + color glyphs). Frame 0 full
/// rebuild populates row ranges; Frame 1 with damage on row 1 only must
/// replay rows 0 and 2 from saved_tier and regenerate row 1 fresh. Mixed-
/// kind atlas + `subpixel_positioning = true` ensure the subpixel and
/// color buffers are non-empty so the cross-buffer assertions catch a
/// real replay bug.
#[test]
fn incremental_dispatch_with_partial_damage_replays_clean_rows_across_all_buffers() {
    let size_q6 = 768;
    let cols = 3;
    let rows = 3;
    let text: String = std::iter::repeat_n('A', cols * rows).collect();
    let mut input = FrameInput::test_grid(cols, rows, &text);
    input.subpixel_positioning = true;
    let (shaped, ids) = shaped_multi_row(cols, rows, 10, size_q6);
    let atlas = key_atlas_mixed_kinds(&ids, size_q6);

    let fresh = prepare_frame_shaped(&input, &atlas, &shaped, (0.0, 0.0));
    assert!(
        !fresh.subpixel_glyphs.is_empty() && !fresh.color_glyphs.is_empty(),
        "fixture must populate subpixel + color buffers"
    );

    let mut frame = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    input.content.damage.push(oriterm_core::DamageLine {
        line: 1,
        left: Column(0),
        right: Column(cols - 1),
    });
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);

    assert!(
        frame.was_incremental,
        "Frame 1 with partial damage must dispatch incremental"
    );
    // With identical input cells, the replay-and-regenerate path produces
    // output identical to a fresh full rebuild on each populated buffer.
    assert_eq!(
        fresh.backgrounds.as_bytes(),
        frame.backgrounds.as_bytes(),
        "row 0 + row 2 backgrounds replayed from saved_tier; row 1 regenerated"
    );
    assert_eq!(
        fresh.glyphs.as_bytes(),
        frame.glyphs.as_bytes(),
        "row 0 + row 2 mono glyphs replayed from saved_tier; row 1 regenerated"
    );
    assert_eq!(
        fresh.subpixel_glyphs.as_bytes(),
        frame.subpixel_glyphs.as_bytes(),
        "row 0 + row 2 subpixel glyphs replayed from saved_tier; row 1 regenerated"
    );
    assert_eq!(
        fresh.color_glyphs.as_bytes(),
        frame.color_glyphs.as_bytes(),
        "row 0 + row 2 color glyphs replayed from saved_tier; row 1 regenerated"
    );
}

/// Regression: BUG-06-027 — scrollback shift via caller's `all_dirty=true`
/// signal dispatches full-rebuild. Three-frame sequence proves incremental
/// is reachable in steady state and the fallback is structural, not
/// vacuous.
#[test]
fn incremental_dispatch_invalidates_on_scrollback_shift() {
    let cols = 4;
    let rows = 3;
    let (mut frame, mut input, shaped, atlas) = prepare_frame0_steady(cols, rows);

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(frame.was_incremental, "Frame 1 incremental");

    // Caller signals scrollback shift via all_dirty=true.
    input.content.all_dirty = true;
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(
        !frame.was_incremental,
        "scrollback shift (all_dirty=true) dispatches full-rebuild"
    );
}

/// Regression: BUG-06-027 — cursor move dirties BOTH the previous cursor
/// row AND the current cursor row when the cursor cell can carry inverted
/// per-cell colors. Pre-fix, `build_dirty_set` only marked the current
/// cursor row dirty; the previous cursor row replayed stale "with cursor"
/// colors from saved_tier. Post-fix, `build_dirty_set` accepts
/// the resolved cursor's line and dirties the previous row too.
#[test]
fn incremental_dispatch_with_cursor_move_dirties_current_and_previous_cursor_rows() {
    let size_q6 = 768;
    let cols = 4;
    let rows = 3;
    let text: String = std::iter::repeat_n('A', cols * rows).collect();
    let mut input = FrameInput::test_grid(cols, rows, &text);
    let (shaped, ids) = shaped_multi_row(cols, rows, 10, size_q6);
    let atlas = key_atlas_with(&ids, size_q6);

    // Frame 0: full rebuild with cursor at row 0.
    input.content.cursor.visible = true;
    input.content.cursor.line = 0;
    input.content.cursor.column = Column(0);
    let mut frame = PreparedFrame::new(ViewportSize::new(1, 1), Rgb { r: 0, g: 0, b: 0 }, 1.0);
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(!frame.was_incremental, "Frame 0 full rebuild");

    // Frame 1: cursor moved to row 1 — both row 0 (prev) and row 1
    // (current) must be dirty so neither row replays stale cursor colors
    // from saved_tier.
    input.content.all_dirty = false;
    input.content.cursor.line = 1;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(
        frame.was_incremental,
        "Frame 1 cursor move dispatches incremental"
    );
    // The dirty-set dirtied row 0 (previous cursor) AND row 1 (current
    // cursor). We assert on the inner scratch_dirty buffer captured during
    // the prepare call.
    assert!(
        frame.scratch_dirty[0],
        "previous cursor row (0) MUST be marked dirty after cursor move"
    );
    assert!(
        frame.scratch_dirty[1],
        "current cursor row (1) MUST be marked dirty"
    );
}

/// Regression: BUG-06-027 — `fg_dim` (pane focus dimming) change forces
/// full-rebuild. Per-cell glyph alpha bakes `fg_dim` at emit time
/// (`emit_cell.rs::fg_alpha`), so a focus change without `all_dirty`
/// would replay stale dimmed alpha from saved_tier.
#[test]
fn incremental_dispatch_falls_back_on_fg_dim_change() {
    let cols = 4;
    let rows = 3;
    let (mut frame, mut input, shaped, atlas) = prepare_frame0_steady(cols, rows);

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(frame.was_incremental, "Frame 1 incremental reachable");

    input.fg_dim = 0.5;
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(
        !frame.was_incremental,
        "fg_dim change must dispatch full-rebuild"
    );
}

/// Regression: BUG-06-027 — hover changes stay on the incremental path
/// (no full-rebuild fallback) and dirty just the affected rows. Hover is
/// row-granular, not frame-granular: full-rebuild on every mouse move
/// would be O(N) waste vs the O(1) row-dirty pattern.
#[test]
fn incremental_dispatch_with_hover_change_stays_incremental_and_dirties_affected_rows() {
    let cols = 4;
    let rows = 3;
    let (mut frame, mut input, shaped, atlas) = prepare_frame0_steady(cols, rows);

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    input.hovered_cell = Some((0, 1));
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(frame.was_incremental, "Frame 1 incremental");

    // Move hover from row 0 to row 2. Must stay on incremental path,
    // must dirty rows 0 and 2 only.
    input.hovered_cell = Some((2, 1));
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(
        frame.was_incremental,
        "hover change must NOT trigger full-rebuild"
    );
    assert!(frame.scratch_dirty[0], "previous hover row (0) dirty");
    assert!(frame.scratch_dirty[2], "current hover row (2) dirty");
    assert!(!frame.scratch_dirty[1], "non-hover row stays clean");
}

/// Regression: BUG-06-027 — subpixel-positioning toggle forces full-rebuild.
/// The flag routes `AtlasKind::Subpixel` glyphs to either `subpixel_glyphs`
/// (when true) or `glyphs` (when false); a toggle would leave saved cells
/// in the wrong buffer.
#[test]
fn incremental_dispatch_falls_back_on_subpixel_positioning_toggle() {
    let cols = 4;
    let rows = 3;
    let (mut frame, mut input, shaped, atlas) = prepare_frame0_steady(cols, rows);

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(frame.was_incremental, "Frame 1 incremental");

    input.subpixel_positioning = !input.subpixel_positioning;
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(
        !frame.was_incremental,
        "subpixel_positioning toggle must dispatch full-rebuild"
    );
}

/// Regression: BUG-06-027 — search-state change forces full-rebuild.
/// Search match highlighting bakes per-cell colors at emit time; without
/// a guard, the cache would replay stale highlights when the user types
/// a new query, navigates between matches, or scrolls.
#[test]
fn incremental_dispatch_falls_back_on_search_state_change() {
    use crate::gpu::frame_input::FrameSearch;
    use oriterm_core::{SearchMatch, StableRowIndex};

    let cols = 4;
    let rows = 3;
    let (mut frame, mut input, shaped, atlas) = prepare_frame0_steady(cols, rows);

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(frame.was_incremental, "Frame 1 incremental");

    // Activate search with one match — different fingerprint from None.
    input.search = Some(FrameSearch::for_test(
        vec![SearchMatch {
            start_row: StableRowIndex(0),
            start_col: 0,
            end_row: StableRowIndex(0),
            end_col: 0,
        }],
        0,
        0,
    ));
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(
        !frame.was_incremental,
        "search activation must dispatch full-rebuild"
    );
}

/// Regression: BUG-06-027 — text blink opacity change forces full-rebuild.
/// Per-cell instances bake `text_blink_opacity` at emit time; replaying
/// clean rows would carry stale opacity. The dispatch fingerprint's
/// `text_blink_opacity` hash input catches this.
#[test]
fn incremental_dispatch_falls_back_on_text_blink_opacity_change() {
    let cols = 4;
    let rows = 3;
    let (mut frame, mut input, shaped, atlas) = prepare_frame0_steady(cols, rows);

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(frame.was_incremental, "Frame 1 incremental reachable");

    input.text_blink_opacity = 0.5;
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(
        !frame.was_incremental,
        "text_blink_opacity change must dispatch full-rebuild"
    );
}

/// Regression: BUG-06-027 — origin change (e.g., scroll without all_dirty)
/// dispatches full-rebuild. Without an origin guard the saved_tier's
/// pixel-positioned cell instances would render at the wrong Y.
#[test]
fn incremental_dispatch_falls_back_on_origin_change() {
    let cols = 4;
    let rows = 3;
    let (mut frame, mut input, shaped, atlas) = prepare_frame0_steady(cols, rows);

    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
    assert!(frame.was_incremental, "Frame 1 incremental reachable");

    prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, -10.0), 1.0);
    assert!(
        !frame.was_incremental,
        "origin change must dispatch full-rebuild (saved_tier cells are pixel-positioned for prev origin)"
    );
}

// ── Grid text Y rounding tests ──

/// Verify all glyph instance Y positions are integer-aligned at fractional
/// DPI (simulating 1.25x scale factor with cell height 18.75).
#[test]
fn grid_y_positions_integer_at_fractional_scale() {
    let mut input = FrameInput::test_grid(5, 4, "ABCDEFGHIJKLMNOPQRST");
    // Simulate 1.25x: fractional cell height and origin.
    input.cell_size = CellMetrics::new(10.0, 18.75, 14.0, 2.0, 1.0, 4.0);
    input.viewport = ViewportSize::new(200, 200);
    let atlas = atlas_with(&[
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T',
    ]);

    let frame = prepare_frame(&input, &atlas, (0.0, 56.3));

    for i in 0..frame.glyphs.len() {
        let inst = nth_instance(frame.glyphs.as_bytes(), i);
        let y = inst.pos.1;
        assert!(
            (y - y.round()).abs() < 0.001,
            "glyph {i} Y = {y} is not integer-aligned"
        );
    }
}

/// Verify Y positions are already integer at 1x scale (regression guard).
#[test]
fn grid_y_positions_integer_at_integer_scale() {
    let input = FrameInput::test_grid(3, 3, "ABCDEFGHI");
    let atlas = atlas_with(&['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I']);

    let frame = prepare_frame(&input, &atlas, (0.0, 56.0));

    for i in 0..frame.glyphs.len() {
        let inst = nth_instance(frame.glyphs.as_bytes(), i);
        let y = inst.pos.1;
        assert!(
            (y - y.round()).abs() < 0.001,
            "glyph {i} Y = {y} is not integer-aligned at 1x scale"
        );
    }
}

/// Verify cursor Y position is integer-aligned at fractional DPI.
#[test]
fn cursor_y_position_integer_at_fractional_scale() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.cell_size = CellMetrics::new(10.0, 18.75, 14.0, 2.0, 1.0, 4.0);
    input.viewport = ViewportSize::new(200, 200);
    input.content.cursor.column = Column(2);
    input.content.cursor.line = 3;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 56.3));

    let c = nth_instance(frame.cursors.as_bytes(), 0);
    let y = c.pos.1;
    assert!(
        (y - y.round()).abs() < 0.001,
        "cursor Y = {y} is not integer-aligned"
    );
}

/// Verify prompt marker Y positions are integer-aligned at fractional DPI.
#[test]
fn prompt_marker_y_integer_at_fractional_scale() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.cell_size = CellMetrics::new(10.0, 18.75, 14.0, 2.0, 1.0, 4.0);
    input.viewport = ViewportSize::new(200, 200);
    input.prompt_marker_rows = vec![0, 2, 4];
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 56.3));

    // Prompt markers are on the cursor layer.
    for i in 0..frame.cursors.len() {
        let inst = nth_instance(frame.cursors.as_bytes(), i);
        let y = inst.pos.1;
        assert!(
            (y - y.round()).abs() < 0.001,
            "prompt marker {i} Y = {y} is not integer-aligned"
        );
    }
}

/// Verify URL underline Y base is integer-aligned (underline offset
/// fractional component is allowed on top of the integer row base).
#[test]
fn url_underline_y_base_integer_at_fractional_scale() {
    let mut input = FrameInput::test_grid(10, 3, "");
    input.cell_size = CellMetrics::new(10.0, 18.75, 14.0, 2.0, 1.0, 4.0);
    input.viewport = ViewportSize::new(200, 200);
    input.hovered_url_segments = vec![(1, 2, 5)]; // row 1, cols 2..5
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas, (0.0, 56.3));

    // The URL underline uses the cursor instance buffer.
    // Find the underline by looking for an instance at row 1's Y.
    let row1_base = (56.3_f32 + 1.0 * 18.75).round();
    let underline_offset = input.cell_size.baseline + input.cell_size.underline_offset;
    let expected_y = row1_base + underline_offset;
    // There should be a cursor instance + the underline. The underline
    // is the one not at row 0 cursor position.
    let mut found_underline = false;
    for i in 0..frame.cursors.len() {
        let inst = nth_instance(frame.cursors.as_bytes(), i);
        if (inst.pos.1 - expected_y).abs() < 0.01 {
            found_underline = true;
        }
    }
    assert!(
        found_underline,
        "expected URL underline at Y={expected_y} (base={row1_base} + offset={underline_offset})"
    );
}

// Text blink opacity tests

#[test]
fn blink_cell_gets_dimmed_fg() {
    let mut input = FrameInput::test_grid(1, 1, "A");
    input.content.cells[0].flags = CellFlags::BLINK;
    input.text_blink_opacity = 0.5;
    // fg_dim defaults to 1.0 in test_grid, so effective alpha = 1.0 * 0.5 = 0.5.
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    // The glyph instance's fg alpha should be fg_dim * text_blink_opacity = 0.5.
    let glyph = nth_instance(frame.glyphs.as_bytes(), 0);
    assert!(
        (glyph.fg_color[3] - 0.5).abs() < 0.01,
        "blink cell alpha should be ~0.5, got {}",
        glyph.fg_color[3],
    );
}

#[test]
fn non_blink_cell_ignores_text_blink_opacity() {
    let mut input = FrameInput::test_grid(1, 1, "A");
    input.text_blink_opacity = 0.5;
    // No BLINK flag — fg_dim = 1.0 unmodified.
    let atlas = atlas_with(&['A']);
    let frame = prepare_frame(&input, &atlas, (0.0, 0.0));

    let glyph = nth_instance(frame.glyphs.as_bytes(), 0);
    assert!(
        (glyph.fg_color[3] - 1.0).abs() < 0.01,
        "non-blink cell alpha should be 1.0, got {}",
        glyph.fg_color[3],
    );
}

// Dirty-skip trace-emission tests.
//
// These tests drive the production `prepare_frame_shaped_into()` path
// (NOT a stub harness) and assert the trace contract via the thread-local
// log capture in `crate::gpu::prepare::trace_capture`. Emission tests must
// observe the real prepare path so the `process_incremental_cells` trace
// is exercised through `frame.was_incremental`.

mod dirty_skip_traces {
    use log::{Level, LevelFilter};
    use oriterm_core::Column;

    use super::{
        FrameInput, PreparedFrame, ViewportSize, key_atlas_with, prepare_frame_shaped_into,
        shaped_multi_row,
    };
    use oriterm_test_support::log_capture::{CapturedRecord, with_capture};

    const DIRTY_SKIP_TARGET: &str = "oriterm::gpu::prepare::dirty_skip::selection_damage";
    const DIRTY_SKIP_MOD_TARGET: &str = "oriterm::gpu::prepare::dirty_skip";

    fn matching_substr(records: &[CapturedRecord], substr: &str) -> Vec<CapturedRecord> {
        records
            .iter()
            .filter(|r| {
                (r.target == DIRTY_SKIP_TARGET || r.target == DIRTY_SKIP_MOD_TARGET)
                    && r.level == Level::Trace
                    && r.message.contains(substr)
            })
            .cloned()
            .collect()
    }

    fn drive_two_frames_for_incremental(cols: usize, rows: usize) -> (PreparedFrame, FrameInput) {
        let size_q6 = 768;
        let text: String = std::iter::repeat_n('A', cols * rows).collect();
        let input = FrameInput::test_grid(cols, rows, &text);
        let (shaped, ids) = shaped_multi_row(cols, rows, 10, size_q6);
        let atlas = key_atlas_with(&ids, size_q6);

        let mut frame = PreparedFrame::new(
            ViewportSize::new(1, 1),
            oriterm_core::Rgb { r: 0, g: 0, b: 0 },
            1.0,
        );
        // Frame 1: full rebuild populates row_ranges. Production's
        // pre-dispatch save_terminal_tier swaps the terminal tier into
        // saved_tier on the next prepare call, so callers using this
        // helper produce a frame whose Frame N+1 prepare will take the
        // incremental path automatically.
        prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
        (frame, input)
    }

    #[test]
    fn incremental_emits_build_dirty_set_per_source_summary() {
        with_capture(LevelFilter::Trace, |sink| {
            let cols = 4;
            let rows = 3;
            let (mut frame, mut input) = drive_two_frames_for_incremental(cols, rows);
            // Frame 2: damage on row 1 only — incremental path.
            input.content.all_dirty = false;
            input.content.cursor.visible = true;
            input.content.cursor.line = 0;
            input.content.damage.clear();
            input.content.damage.push(oriterm_core::DamageLine {
                line: 1,
                left: Column(0),
                right: Column(cols - 1),
            });
            let (shaped, ids) = shaped_multi_row(cols, rows, 10, 768);
            let atlas = key_atlas_with(&ids, 768);
            prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
            assert!(frame.was_incremental, "second prepare must be incremental");

            let recs = matching_substr(&sink.records(), "build_dirty_set all_dirty=false");
            assert_eq!(
                recs.len(),
                1,
                "expected ONE per-source summary; got {:?}",
                recs
            );
            let msg = &recs[0].message;
            assert!(msg.contains("damage="), "msg={msg}");
            assert!(msg.contains("cursor=true"), "msg={msg}");
            assert!(msg.contains("selection_added="), "msg={msg}");
            assert!(msg.contains("total="), "msg={msg}");
        });
    }

    #[test]
    fn full_rebuild_via_all_dirty_does_not_emit_build_dirty_set_trace() {
        // `prepare/mod.rs` gates `can_incremental = !all_dirty && saved_tier.has_cached_rows()`.
        // When `all_dirty == true`, the full-rebuild path runs and `build_dirty_set`
        // is not invoked. This pins that production behavior — operators
        // observing ZERO build_dirty_set traces in a frame correctly conclude
        // the prepare path took the full-rebuild branch.
        with_capture(LevelFilter::Trace, |sink| {
            let cols = 4;
            let rows = 3;
            let (mut frame, mut input) = drive_two_frames_for_incremental(cols, rows);
            input.content.all_dirty = true;
            let (shaped, ids) = shaped_multi_row(cols, rows, 10, 768);
            let atlas = key_atlas_with(&ids, 768);
            prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
            assert!(
                !frame.was_incremental,
                "all_dirty=true must take the full-rebuild path"
            );

            let recs = matching_substr(&sink.records(), "build_dirty_set");
            assert!(
                recs.is_empty(),
                "build_dirty_set must not fire on full-rebuild path; got {:?}",
                recs
            );
        });
    }

    #[test]
    fn incremental_emits_process_incremental_cells_summary() {
        with_capture(LevelFilter::Trace, |sink| {
            let cols = 4;
            let rows = 3;
            let (mut frame, mut input) = drive_two_frames_for_incremental(cols, rows);
            input.content.all_dirty = false;
            input.content.cursor.visible = false;
            input.content.damage.clear();
            input.content.damage.push(oriterm_core::DamageLine {
                line: 1,
                left: Column(0),
                right: Column(cols - 1),
            });
            let (shaped, ids) = shaped_multi_row(cols, rows, 10, 768);
            let atlas = key_atlas_with(&ids, 768);
            prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
            assert!(frame.was_incremental);

            let recs = matching_substr(&sink.records(), "process_incremental_cells");
            assert_eq!(recs.len(), 1, "got: {:?}", recs);
            let msg = &recs[0].message;
            assert!(msg.contains("clean_rows="), "msg={msg}");
            assert!(msg.contains("dirty_rows="), "msg={msg}");
            assert!(msg.contains("emitted_cells="), "msg={msg}");
        });
    }

    #[test]
    fn full_rebuild_path_does_not_emit_process_incremental_cells_trace() {
        with_capture(LevelFilter::Trace, |sink| {
            let cols = 4;
            let rows = 3;
            let size_q6 = 768;
            let text: String = std::iter::repeat_n('A', cols * rows).collect();
            let input = FrameInput::test_grid(cols, rows, &text);
            let (shaped, ids) = shaped_multi_row(cols, rows, 10, size_q6);
            let atlas = key_atlas_with(&ids, size_q6);

            let mut frame = PreparedFrame::new(
                ViewportSize::new(1, 1),
                oriterm_core::Rgb { r: 0, g: 0, b: 0 },
                1.0,
            );
            // First prepare with no saved_tier → can_incremental=false → full rebuild.
            prepare_frame_shaped_into(&input, &atlas, &shaped, &mut frame, (0.0, 0.0), 1.0);
            assert!(
                !frame.was_incremental,
                "first prepare must take the full-rebuild path"
            );

            let recs = matching_substr(&sink.records(), "process_incremental_cells");
            assert!(
                recs.is_empty(),
                "process_incremental_cells trace must NOT fire on full-rebuild path; got {:?}",
                recs
            );
        });
    }
}

// ── Dispatch-fingerprint tests ──
//
// `compute_dispatch_fingerprint` is the SSOT for "did frame-level state
// change?" — replaces the prior 11-clause enumerated dispatch predicate.
// Each test below varies ONE input field relative to a baseline and asserts
// fingerprint equality/inequality. Counter-pin tests verify that fields
// intentionally excluded (selection, cursor, hovered_cell) do NOT change
// the fingerprint — those flow through per-row `build_dirty_set` instead.

mod dispatch_fingerprint {
    use super::super::compute_dispatch_fingerprint;
    use crate::gpu::frame_input::FrameInput;

    fn baseline() -> (FrameInput, (f32, f32)) {
        let mut input = FrameInput::test_grid(10, 5, "Hello");
        input.text_blink_opacity = 1.0;
        input.fg_dim = 1.0;
        input.palette.opacity = 1.0;
        input.subpixel_positioning = false;
        input.selection = None;
        input.hovered_cell = None;
        let origin = (0.0_f32, 0.0_f32);
        (input, origin)
    }

    /// Stable fingerprint: same inputs → same output.
    #[test]
    fn fingerprint_stable_across_unchanged_frames() {
        let (input, origin) = baseline();
        let a = compute_dispatch_fingerprint(&input, origin);
        let b = compute_dispatch_fingerprint(&input, origin);
        assert_eq!(a, b, "identical inputs must produce identical fingerprints");
    }

    /// Geometry change → fingerprint changes.
    #[test]
    fn fingerprint_changes_with_viewport_width() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.viewport.width += 10;
        assert_ne!(compute_dispatch_fingerprint(&input, origin), baseline_fp);
    }

    #[test]
    fn fingerprint_changes_with_viewport_height() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.viewport.height += 10;
        assert_ne!(compute_dispatch_fingerprint(&input, origin), baseline_fp);
    }

    #[test]
    fn fingerprint_changes_with_cell_size_width() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.cell_size.width += 1.0;
        assert_ne!(compute_dispatch_fingerprint(&input, origin), baseline_fp);
    }

    #[test]
    fn fingerprint_changes_with_cell_size_height() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.cell_size.height += 1.0;
        assert_ne!(compute_dispatch_fingerprint(&input, origin), baseline_fp);
    }

    #[test]
    fn fingerprint_changes_with_cell_size_baseline() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.cell_size.baseline += 1.0;
        assert_ne!(compute_dispatch_fingerprint(&input, origin), baseline_fp);
    }

    /// Regression: BUG-06-030 — all 6 CellMetrics fields hashed (not just 3).
    /// See: bug-tracker/plans/BUG-06-030/section-03-tdd-matrix.md
    #[test]
    fn fingerprint_changes_with_cell_size_underline_offset() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.cell_size.underline_offset += 1.0;
        assert_ne!(
            compute_dispatch_fingerprint(&input, origin),
            baseline_fp,
            "underline_offset MUST be in fingerprint — affects decoration emission"
        );
    }

    #[test]
    fn fingerprint_changes_with_cell_size_stroke_size() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.cell_size.stroke_size += 1.0;
        assert_ne!(
            compute_dispatch_fingerprint(&input, origin),
            baseline_fp,
            "stroke_size MUST be in fingerprint — affects underline/strikeout geometry"
        );
    }

    #[test]
    fn fingerprint_changes_with_cell_size_strikeout_offset() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.cell_size.strikeout_offset += 1.0;
        assert_ne!(
            compute_dispatch_fingerprint(&input, origin),
            baseline_fp,
            "strikeout_offset MUST be in fingerprint — affects strikethrough position"
        );
    }

    #[test]
    fn fingerprint_changes_with_content_cols() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.content_cols += 1;
        assert_ne!(compute_dispatch_fingerprint(&input, origin), baseline_fp);
    }

    #[test]
    fn fingerprint_changes_with_content_rows() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.content_rows += 1;
        assert_ne!(compute_dispatch_fingerprint(&input, origin), baseline_fp);
    }

    #[test]
    fn fingerprint_changes_with_origin_x() {
        let (input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        let new_origin = (origin.0 + 5.0, origin.1);
        assert_ne!(
            compute_dispatch_fingerprint(&input, new_origin),
            baseline_fp
        );
    }

    #[test]
    fn fingerprint_changes_with_origin_y() {
        let (input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        let new_origin = (origin.0, origin.1 + 5.0);
        assert_ne!(
            compute_dispatch_fingerprint(&input, new_origin),
            baseline_fp
        );
    }

    #[test]
    fn fingerprint_changes_with_text_blink_opacity() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.text_blink_opacity = 0.5;
        assert_ne!(compute_dispatch_fingerprint(&input, origin), baseline_fp);
    }

    #[test]
    fn fingerprint_changes_with_palette_opacity() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.palette.opacity = 0.8;
        assert_ne!(compute_dispatch_fingerprint(&input, origin), baseline_fp);
    }

    #[test]
    fn fingerprint_changes_with_fg_dim() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.fg_dim = 0.6;
        assert_ne!(compute_dispatch_fingerprint(&input, origin), baseline_fp);
    }

    #[test]
    fn fingerprint_changes_with_subpixel_positioning() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.subpixel_positioning = true;
        assert_ne!(compute_dispatch_fingerprint(&input, origin), baseline_fp);
    }

    /// Search highlight state is hashed via `damage_fingerprint`. Add a search
    /// match to the input and assert the dispatch fingerprint differs from
    /// baseline (no search). Closes the §03 search-fingerprint coverage gap.
    #[test]
    fn fingerprint_changes_with_search_fingerprint() {
        use crate::gpu::frame_input::FrameSearch;
        use oriterm_core::{Column, SearchMatch, StableRowIndex};

        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);

        let _ = Column(0); // unused — SearchMatch fields use bare usize
        let matches = vec![SearchMatch {
            start_row: StableRowIndex(0),
            start_col: 0,
            end_row: StableRowIndex(0),
            end_col: 3,
        }];
        input.search = Some(FrameSearch::for_test(matches, 0, 0));
        let with_search = compute_dispatch_fingerprint(&input, origin);
        assert_ne!(
            with_search, baseline_fp,
            "search match presence must alter the dispatch fingerprint"
        );
    }

    /// Counter-pins: row-state fields are intentionally NOT in the fingerprint.
    /// These changes must flow through `build_dirty_set` (incremental path) or
    /// `WindowRenderer::has_row_state_change` (fast-path gate), never via full
    /// rebuild dispatch.

    #[test]
    fn fingerprint_unchanged_when_hovered_cell_changes() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.hovered_cell = Some((0, 0));
        assert_eq!(
            compute_dispatch_fingerprint(&input, origin),
            baseline_fp,
            "hovered_cell MUST NOT be in fingerprint — handled per-row by build_dirty_set"
        );
    }

    #[test]
    fn fingerprint_unchanged_when_cursor_position_changes() {
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        // RenderableCursor uses flat fields (line, column, shape, visible).
        input.content.cursor.line = 1;
        input.content.cursor.column = oriterm_core::Column(2);
        assert_eq!(
            compute_dispatch_fingerprint(&input, origin),
            baseline_fp,
            "cursor position MUST NOT be in fingerprint — handled per-row via prev_resolved_cursor dirtying"
        );
    }

    #[test]
    fn fingerprint_unchanged_when_selection_changes() {
        use oriterm_core::{Selection, Side, StableRowIndex};
        let (mut input, origin) = baseline();
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        let mut sel = Selection::new_char(StableRowIndex(0), 0, Side::Left);
        sel.end = oriterm_core::SelectionPoint {
            row: StableRowIndex(0),
            col: 3,
            side: Side::Right,
        };
        input.selection = Some(crate::gpu::frame_input::FrameSelection::new(&sel, 0));
        assert_eq!(
            compute_dispatch_fingerprint(&input, origin),
            baseline_fp,
            "selection MUST NOT be in fingerprint — handled per-row via prev_selection_snapshot dirtying"
        );
    }

    /// Regression: BUG-06-030 — bitwise-exact via `.to_bits()`; `+0.0` and
    /// `-0.0` produce different fingerprints (one spurious rebuild on flip).
    /// See: bug-tracker/plans/BUG-06-030/
    #[test]
    fn fingerprint_distinguishes_positive_zero_from_negative_zero() {
        let (mut input, origin) = baseline();
        input.text_blink_opacity = 0.0_f32;
        let pos_zero = compute_dispatch_fingerprint(&input, origin);
        input.text_blink_opacity = -0.0_f32;
        let neg_zero = compute_dispatch_fingerprint(&input, origin);
        assert_ne!(
            pos_zero, neg_zero,
            "bitwise-exact via .to_bits() distinguishes +0.0 from -0.0 \
             (one spurious rebuild on flip, accepted per consensus)"
        );
    }

    /// Bitwise pin: smallest representable f32 delta invalidates (replaces prior
    /// `< f32::EPSILON` epsilon-tolerant comparison). Using `from_bits()` to
    /// construct the next-representable value avoids the float-rounding gotcha
    /// where `1.0 + EPSILON/2.0` rounds back to exactly `1.0` in f32.
    #[test]
    fn fingerprint_distinguishes_smallest_float_delta() {
        let (mut input, origin) = baseline();
        input.text_blink_opacity = 1.0_f32;
        let baseline_fp = compute_dispatch_fingerprint(&input, origin);
        input.text_blink_opacity = f32::from_bits(1.0_f32.to_bits() + 1);
        let perturbed_fp = compute_dispatch_fingerprint(&input, origin);
        assert_ne!(
            baseline_fp, perturbed_fp,
            "bitwise-exact comparison must invalidate on the smallest \
             representable f32 delta (extra rebuilds, never stale reuse)"
        );
    }

    /// Distinct origins with otherwise-identical inputs must produce distinct
    /// fingerprints. Documents that the fingerprint is content-aware, not
    /// just shape-aware.
    #[test]
    fn fingerprint_distinguishes_distinct_origins_same_shape() {
        let (input, _origin) = baseline();
        let a = compute_dispatch_fingerprint(&input, (10.0, 20.0));
        let b = compute_dispatch_fingerprint(&input, (15.0, 20.0));
        assert_ne!(
            a, b,
            "different origins with otherwise-identical inputs must produce different fingerprints"
        );
    }
}

// Regression pin against re-introducing enumerated prev_* fields lives in
// `gpu/prepared_frame/tests.rs::enumerated_prev_fields_removed` (the test
// scans `prepared_frame/mod.rs`, so it is colocated with the file it pins).
