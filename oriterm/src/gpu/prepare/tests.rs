//! Unit tests for the prepare phase.

use std::collections::HashMap;

use oriterm_core::{CellFlags, Column, CursorShape, Rgb};

use super::{prepare_frame, prepare_frame_into, AtlasLookup};
use crate::font::GlyphStyle;
use crate::gpu::atlas::AtlasEntry;
use crate::gpu::frame_input::FrameInput;
use crate::gpu::instance_writer::INSTANCE_SIZE;
use crate::gpu::prepared_frame::PreparedFrame;

// ── Test atlas ──

/// Test atlas backed by a `HashMap`.
struct TestAtlas(HashMap<(char, GlyphStyle), AtlasEntry>);

impl AtlasLookup for TestAtlas {
    fn lookup(&self, ch: char, style: GlyphStyle) -> Option<&AtlasEntry> {
        self.0.get(&(ch, style))
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

/// Convert Rgb to the [f32; 4] that push_rect writes to bg_color.
fn rgb_f32(c: Rgb) -> [f32; 4] {
    [
        f32::from(c.r) / 255.0,
        f32::from(c.g) / 255.0,
        f32::from(c.b) / 255.0,
        1.0,
    ]
}

// ── Instance buffer correctness ──

#[test]
fn single_char_produces_one_bg_and_one_fg() {
    let input = FrameInput::test_grid(1, 1, "A");
    let atlas = atlas_with(&['A']);

    let frame = prepare_frame(&input, &atlas);

    // 1 bg for the cell, 1 fg for the glyph, 1 cursor (block at 0,0).
    assert_counts(&frame, 1, 1, 1);
}

#[test]
fn single_char_bg_position_and_size() {
    let input = FrameInput::test_grid(2, 2, "A");
    let atlas = atlas_with(&['A']);

    let frame = prepare_frame(&input, &atlas);

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

    let frame = prepare_frame(&input, &atlas);

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

    let frame = prepare_frame(&input, &atlas);

    let fg = nth_instance(frame.glyphs.as_bytes(), 0);
    assert_eq!(fg.fg_color, rgb_f32(fg_rgb));
}

#[test]
fn single_char_bg_color_matches_cell() {
    let input = FrameInput::test_grid(1, 1, "A");
    let atlas = atlas_with(&['A']);
    let bg_rgb = input.content.cells[0].bg;

    let frame = prepare_frame(&input, &atlas);

    let bg = nth_instance(frame.backgrounds.as_bytes(), 0);
    assert_eq!(bg.bg_color, rgb_f32(bg_rgb));
}

// ── Empty cells ──

#[test]
fn empty_cell_produces_bg_only() {
    let input = FrameInput::test_grid(1, 1, " ");
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas);

    assert_eq!(frame.backgrounds.len(), 1);
    assert_eq!(frame.glyphs.len(), 0);
}

#[test]
fn all_spaces_grid_no_fg_instances() {
    let input = FrameInput::test_grid(10, 5, "");
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas);

    assert_eq!(frame.backgrounds.len(), 50);
    assert_eq!(frame.glyphs.len(), 0);
}

#[test]
fn all_chars_grid_equal_bg_and_fg() {
    let text: String = std::iter::repeat_n('A', 10).collect();
    let input = FrameInput::test_grid(10, 1, &text);
    let atlas = atlas_with(&['A']);

    let frame = prepare_frame(&input, &atlas);

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

    let frame = prepare_frame(&input, &atlas);

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

    let frame = prepare_frame(&input, &atlas);

    // Only 1 bg (the wide char covers both columns), not 2.
    assert_eq!(frame.backgrounds.len(), 1);
}

// ── Cell positions are pixel-perfect ──

#[test]
fn cell_positions_are_pixel_perfect() {
    let input = FrameInput::test_grid(3, 3, "ABCDEFGHI");
    let atlas = atlas_with(&['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I']);

    let frame = prepare_frame(&input, &atlas);

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

    let frame = prepare_frame(&input, &atlas);

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

    let frame = prepare_frame(&input, &atlas);

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

    let frame = prepare_frame(&input, &atlas);

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

    let frame1 = prepare_frame(&input, &atlas);
    let frame2 = prepare_frame(&input, &atlas);

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

    let frame = prepare_frame(&input, &atlas);

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

    let frame = prepare_frame(&input, &atlas);

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

    let frame = prepare_frame(&input, &atlas);

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

    let frame = prepare_frame(&input, &atlas);

    assert_eq!(frame.cursors.len(), 4);
}

#[test]
fn hollow_block_edges() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.cursor.shape = CursorShape::HollowBlock;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas);

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

    let frame = prepare_frame(&input, &atlas);

    assert_eq!(frame.cursors.len(), 0);
}

#[test]
fn cursor_invisible_zero_instances() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.cursor.visible = false;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas);

    assert_eq!(frame.cursors.len(), 0);
}

#[test]
fn cursor_at_position() {
    let mut input = FrameInput::test_grid(10, 10, "");
    input.content.cursor.column = Column(5);
    input.content.cursor.line = 3;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas);

    let c = nth_instance(frame.cursors.as_bytes(), 0);
    assert_eq!(c.pos, (40.0, 48.0)); // 5*8=40, 3*16=48
}

#[test]
fn cursor_color_from_palette() {
    let input = FrameInput::test_grid(10, 5, "");
    let atlas = empty_atlas();
    let cursor_color = input.palette.cursor_color;

    let frame = prepare_frame(&input, &atlas);

    let c = nth_instance(frame.cursors.as_bytes(), 0);
    assert_eq!(c.fg_color, rgb_f32(cursor_color));
}

// ── Missing atlas entries ──

#[test]
fn missing_glyph_skips_fg_instance() {
    let input = FrameInput::test_grid(1, 1, "Z");
    let atlas = empty_atlas(); // No entry for 'Z'.

    let frame = prepare_frame(&input, &atlas);

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

    let frame = prepare_frame(&input, &atlas);

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

    let frame = prepare_frame(&input, &atlas);

    assert_eq!(frame.glyphs.len(), 1);
}

#[test]
fn bold_italic_cell_uses_bold_italic_style() {
    let mut input = FrameInput::test_grid(1, 1, "X");
    input.content.cells[0].flags = CellFlags::BOLD | CellFlags::ITALIC;

    let mut map = HashMap::new();
    map.insert(('X', GlyphStyle::BoldItalic), test_entry('X'));
    let atlas = TestAtlas(map);

    let frame = prepare_frame(&input, &atlas);

    assert_eq!(frame.glyphs.len(), 1);
}

// ── Instance count for larger grids ──

#[test]
fn ten_by_five_all_spaces() {
    let input = FrameInput::test_grid(10, 5, "");
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas);

    assert_counts(&frame, 50, 0, 1); // 1 cursor (block, visible)
}

#[test]
fn clear_color_matches_palette_background() {
    let input = FrameInput::test_grid(10, 5, "");
    let atlas = empty_atlas();
    let bg = input.palette.background;

    let frame = prepare_frame(&input, &atlas);

    let expected = [
        f64::from(bg.r) / 255.0,
        f64::from(bg.g) / 255.0,
        f64::from(bg.b) / 255.0,
        1.0,
    ];
    assert_eq!(frame.clear_color, expected);
}

#[test]
fn clear_color_respects_palette_opacity() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.palette.opacity = 0.5;
    let atlas = empty_atlas();

    let frame = prepare_frame(&input, &atlas);

    let bg = input.palette.background;
    let expected = [
        f64::from(bg.r) / 255.0 * 0.5,
        f64::from(bg.g) / 255.0 * 0.5,
        f64::from(bg.b) / 255.0 * 0.5,
        0.5,
    ];
    assert_eq!(frame.clear_color, expected);
}

// ── prepare_frame_into ──

#[test]
fn prepare_into_matches_prepare() {
    let input = FrameInput::test_grid(10, 5, "Hello World!");
    let atlas = atlas_with(&['H', 'e', 'l', 'o', 'W', 'r', 'd', '!']);

    let fresh = prepare_frame(&input, &atlas);

    let mut reused = PreparedFrame::new(Rgb { r: 0, g: 0, b: 0 }, 1.0);
    prepare_frame_into(&input, &atlas, &mut reused);

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
    let mut frame = prepare_frame(&input, &atlas);
    let first_bg_count = frame.backgrounds.len();
    let first_fg_count = frame.glyphs.len();

    // Second prepare with smaller input reuses (clear + refill).
    let small = FrameInput::test_grid(2, 1, "A");
    prepare_frame_into(&small, &atlas, &mut frame);

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

    let mut frame = prepare_frame(&input1, &atlas);
    let first_bg = frame.backgrounds.len();
    let first_fg = frame.glyphs.len();

    // Second frame with different content.
    let input2 = FrameInput::test_grid(2, 1, "B");
    prepare_frame_into(&input2, &atlas, &mut frame);

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

    let mut frame = prepare_frame(&input1, &atlas);
    let first_clear = frame.clear_color;

    // Change palette background.
    let mut input2 = FrameInput::test_grid(2, 1, "");
    input2.palette.background = Rgb {
        r: 255,
        g: 0,
        b: 0,
    };
    prepare_frame_into(&input2, &atlas, &mut frame);

    assert_ne!(frame.clear_color, first_clear);
    assert_eq!(frame.clear_color, [1.0, 0.0, 0.0, 1.0]);
}
