use oriterm_core::{CellFlags, Column, RenderableCell, RenderableCursor};

use crate::font::{GlyphStyle, RasterKey};
use crate::gpu::atlas::{AtlasEntry, AtlasKind};
use crate::gpu::frame_input::FrameInput;
use crate::gpu::instance_writer::INSTANCE_SIZE;
use crate::gpu::prepare::shaped_frame::ShapedFrame;
use crate::gpu::prepared_frame::PreparedFrame;

use super::super::AtlasLookup;
use super::{EmitCtx, emit_cell};

// ---------------------------------------------------------------------------
// Minimal atlas that returns a test entry for every lookup.
// ---------------------------------------------------------------------------

struct TestAtlas;

fn test_entry() -> AtlasEntry {
    AtlasEntry {
        uv_x: 0.0,
        uv_y: 0.0,
        uv_w: 0.5,
        uv_h: 0.5,
        bearing_x: 1,
        bearing_y: 12,
        width: 6,
        height: 12,
        page: 0,
        kind: AtlasKind::Mono,
    }
}

impl AtlasLookup for TestAtlas {
    fn lookup(&self, _ch: char, _style: GlyphStyle) -> Option<&AtlasEntry> {
        // Safe: entry is stored in a static so the reference lives long enough.
        static ENTRY: std::sync::OnceLock<AtlasEntry> = std::sync::OnceLock::new();
        Some(ENTRY.get_or_init(test_entry))
    }

    fn lookup_key(&self, _key: RasterKey) -> Option<&AtlasEntry> {
        static ENTRY: std::sync::OnceLock<AtlasEntry> = std::sync::OnceLock::new();
        Some(ENTRY.get_or_init(test_entry))
    }
}

/// Atlas that responds ONLY to `lookup_key` — proves the shaped builtin branch
/// uses `lookup_key`, not `lookup`. If the wrong branch fires (unshaped `lookup`),
/// no glyph entry is returned and no instance is pushed.
struct LookupKeyOnlyAtlas;

impl AtlasLookup for LookupKeyOnlyAtlas {
    fn lookup_key(&self, _key: RasterKey) -> Option<&AtlasEntry> {
        static ENTRY: std::sync::OnceLock<AtlasEntry> = std::sync::OnceLock::new();
        Some(ENTRY.get_or_init(test_entry))
    }
    // `lookup` returns None (trait default) — proves the builtin check uses lookup_key.
}

// ---------------------------------------------------------------------------
// Instance byte helpers.
// ---------------------------------------------------------------------------

fn read_f32(bytes: &[u8], off: usize) -> f32 {
    f32::from_le_bytes(bytes[off..off + 4].try_into().unwrap())
}

fn instance_x(bytes: &[u8], n: usize) -> f32 {
    read_f32(bytes, n * INSTANCE_SIZE)
}

fn instance_y(bytes: &[u8], n: usize) -> f32 {
    read_f32(bytes, n * INSTANCE_SIZE + 4)
}

fn instance_w(bytes: &[u8], n: usize) -> f32 {
    read_f32(bytes, n * INSTANCE_SIZE + 8)
}

fn instance_h(bytes: &[u8], n: usize) -> f32 {
    read_f32(bytes, n * INSTANCE_SIZE + 12)
}

/// Read the fg alpha (offset 44) — glyph dim / tint alpha.
fn instance_glyph_alpha(bytes: &[u8], n: usize) -> f32 {
    read_f32(bytes, n * INSTANCE_SIZE + 44)
}

/// Read the bg alpha (offset 60) — rect fill alpha (push_rect writes here, fg slot is zeroed).
fn instance_rect_alpha(bytes: &[u8], n: usize) -> f32 {
    read_f32(bytes, n * INSTANCE_SIZE + 60)
}

fn bg_count(frame: &PreparedFrame) -> usize {
    frame.backgrounds.as_bytes().len() / INSTANCE_SIZE
}

fn glyph_count(frame: &PreparedFrame) -> usize {
    frame.glyphs.as_bytes().len() / INSTANCE_SIZE
}

// ---------------------------------------------------------------------------
// Test helpers.
// ---------------------------------------------------------------------------

/// Build a default-filled RenderableCell at (row=0, col=0).
fn cell(ch: char, flags: CellFlags) -> RenderableCell {
    use oriterm_core::Rgb;
    RenderableCell {
        line: 0,
        column: Column(0),
        ch,
        fg: Rgb {
            r: 200,
            g: 200,
            b: 200,
        },
        bg: Rgb {
            r: 30,
            g: 30,
            b: 46,
        },
        flags,
        underline_color: None,
        has_hyperlink: false,
        hyperlink_uri: None,
        zerowidth: Vec::new(),
    }
}

fn idle_cursor() -> RenderableCursor {
    RenderableCursor {
        line: 99,
        column: Column(99),
        shape: oriterm_core::CursorShape::Block,
        visible: false,
    }
}

/// Build a minimal EmitCtx from a FrameInput for the unshaped test path.
fn unshaped_ctx<'a>(
    input: &'a FrameInput,
    atlas: &'a dyn AtlasLookup,
    frame: &'a mut PreparedFrame,
) -> EmitCtx<'a> {
    EmitCtx {
        fg_dim: input.fg_dim,
        text_blink_opacity: input.text_blink_opacity,
        subpixel_positioning: false,
        palette: &input.palette,
        sel: input.selection.as_ref(),
        search: input.search.as_ref(),
        cursor: idle_cursor(),
        cursor_opacity: 1.0,
        hovered_cell: input.hovered_cell,
        cell_size: &input.cell_size,
        atlas,
        size_q6: 0,
        frame,
        shaped: None,
    }
}

/// Build a minimal EmitCtx for the shaped test path with a given ShapedFrame.
fn shaped_ctx<'a>(
    input: &'a FrameInput,
    atlas: &'a dyn AtlasLookup,
    frame: &'a mut PreparedFrame,
    shaped: &'a ShapedFrame,
) -> EmitCtx<'a> {
    EmitCtx {
        fg_dim: input.fg_dim,
        text_blink_opacity: input.text_blink_opacity,
        subpixel_positioning: false,
        palette: &input.palette,
        sel: input.selection.as_ref(),
        search: input.search.as_ref(),
        cursor: idle_cursor(),
        cursor_opacity: 1.0,
        hovered_cell: input.hovered_cell,
        cell_size: &input.cell_size,
        atlas,
        size_q6: shaped.size_q6(),
        frame,
        shaped: Some((shaped, false)),
    }
}

fn new_frame(input: &FrameInput) -> PreparedFrame {
    PreparedFrame::with_capacity(
        input.viewport,
        input.columns(),
        input.rows(),
        input.palette.background,
        f64::from(input.palette.opacity),
    )
}

fn single_cell_input() -> FrameInput {
    FrameInput::test_grid(1, 1, "A")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
/// Bg quad has the correct x, y, width, height for a normal cell at origin.
fn emit_cell_pushes_bg_instance_with_correct_dims() {
    let input = single_cell_input();
    let atlas = TestAtlas;
    let mut frame = new_frame(&input);

    let mut ctx = unshaped_ctx(&input, &atlas, &mut frame);
    emit_cell(&input.content.cells[0], 0.0, 0.0, &mut ctx);

    assert_eq!(bg_count(&frame), 1, "expected 1 bg instance");
    let b = frame.backgrounds.as_bytes();
    assert_eq!(instance_x(b, 0), 0.0, "bg x");
    assert_eq!(instance_y(b, 0), 0.0, "bg y");
    // cell_size is 8×16 from test_grid
    assert_eq!(instance_w(b, 0), 8.0, "bg width = cell width");
    assert_eq!(instance_h(b, 0), 16.0, "bg height = cell height");
}

#[test]
/// Wide char (WIDE_CHAR flag) gets a background quad twice as wide.
fn emit_cell_uses_bg_w_for_wide_char() {
    let input = single_cell_input();
    let atlas = TestAtlas;
    let mut frame = new_frame(&input);

    let wide_cell = cell('W', CellFlags::WIDE_CHAR);
    let mut ctx = unshaped_ctx(&input, &atlas, &mut frame);
    emit_cell(&wide_cell, 0.0, 0.0, &mut ctx);

    assert_eq!(bg_count(&frame), 1, "expected 1 bg instance");
    let b = frame.backgrounds.as_bytes();
    assert_eq!(
        instance_w(b, 0),
        16.0,
        "wide-char bg width = 2 × cell_width"
    );
}

#[test]
/// BLINK reduces glyph alpha (cell_dim) but DOES NOT affect bg alpha (always 1.0).
fn emit_cell_applies_blink_alpha_to_bg() {
    let mut input = single_cell_input();
    input.text_blink_opacity = 0.5;
    let atlas = TestAtlas;

    // BLINK cell: glyph alpha = fg_dim * text_blink_opacity = 1.0 * 0.5 = 0.5
    let mut frame = new_frame(&input);
    let blink_cell = cell('A', CellFlags::BLINK);
    let mut ctx = unshaped_ctx(&input, &atlas, &mut frame);
    emit_cell(&blink_cell, 0.0, 0.0, &mut ctx);

    assert_eq!(
        glyph_count(&frame),
        1,
        "glyph must be emitted for BLINK cell"
    );
    let g = frame.glyphs.as_bytes();
    let glyph_alpha = instance_glyph_alpha(g, 0);
    assert!(
        (glyph_alpha - 0.5).abs() < 0.01,
        "BLINK glyph alpha should be 0.5 (fg_dim * text_blink_opacity), got {glyph_alpha}"
    );

    // Negative pin: bg must stay at alpha=1.0 regardless of blink state.
    // push_rect writes the color into the bg slot (offset 60), fg slot is zeroed.
    assert_eq!(bg_count(&frame), 1, "bg pushed even when blink=0.5");
    let b = frame.backgrounds.as_bytes();
    let bg_alpha = instance_rect_alpha(b, 0);
    assert!(
        (bg_alpha - 1.0).abs() < 0.01,
        "bg alpha must remain 1.0 even for BLINK cells, got {bg_alpha}"
    );
}

#[test]
/// Superscript shifts the glyph y upward, leaving bg y unchanged at cell-top.
fn emit_cell_applies_super_sub_glyph_offset_to_glyph_only() {
    let input = single_cell_input();
    let atlas = TestAtlas;
    let mut frame = new_frame(&input);

    let super_cell = cell('A', CellFlags::SUPERSCRIPT);
    let mut ctx = unshaped_ctx(&input, &atlas, &mut frame);
    emit_cell(&super_cell, 0.0, 0.0, &mut ctx);

    // bg is at cell-top y=0
    let b = frame.backgrounds.as_bytes();
    assert_eq!(instance_y(b, 0), 0.0, "bg y must stay at cell-top");

    // glyph y = cell-top + baseline - bearing_y + super_sub_offset
    // super_sub_offset = -(16.0 * 0.25).round() = -4.0
    // glyph_y before bearing = 0.0 + (-4.0) = -4.0
    // glyph position = glyph_y + baseline - bearing_y = -4.0 + 12.0 - 12 = -4.0
    let g = frame.glyphs.as_bytes();
    assert_eq!(glyph_count(&frame), 1, "one glyph emitted");
    let gy = instance_y(g, 0);
    assert!(
        gy < 0.0,
        "superscript glyph y must be above cell-top (got {})",
        gy
    );
}

#[test]
/// Negative pin: decoration y is anchored to cell-top, NOT to the glyph_y offset.
/// If SGR 74 (subscript) were accidentally applied to the decoration draw call,
/// the underline would appear below its correct position.
fn emit_cell_anchors_decoration_to_cell_top_y() {
    let input = single_cell_input();
    let atlas = TestAtlas;
    let mut frame = new_frame(&input);

    // Subscript + underline: glyph shifts DOWN by (16*0.25)=4px; underline must NOT.
    let cell_flags = CellFlags::SUBSCRIPT | CellFlags::UNDERLINE;
    let c = cell('A', cell_flags);
    let mut ctx = unshaped_ctx(&input, &atlas, &mut frame);
    emit_cell(&c, 0.0, 0.0, &mut ctx);

    // Find the underline rect in backgrounds.
    // Underline y = cell_top_y + baseline + underline_offset = 0 + 12.0 + 2.0 = 14.0
    // If subscript leaked: underline_y would be 4.0 + 12.0 + 2.0 = 18.0
    let expected_underline_y = 14.0_f32; // from test_grid metrics: baseline=12, underline_offset=2
    let wrong_underline_y = 18.0_f32; // what it would be if glyph_y leaked

    let b = frame.backgrounds.as_bytes();
    let n = bg_count(&frame);
    let ys: Vec<f32> = (0..n).map(|i| instance_y(b, i)).collect();

    assert!(
        ys.iter().any(|&y| (y - expected_underline_y).abs() < 0.1),
        "underline should be at cell-top y={expected_underline_y}; got y-positions: {ys:?}"
    );
    assert!(
        !ys.iter().any(|&y| (y - wrong_underline_y).abs() < 0.1),
        "underline must NOT be at leaked glyph_y={wrong_underline_y} (subscript must not affect decoration y)"
    );
}

#[test]
/// In the shaped path, a builtin char uses `lookup_key` (not GlyphEmitter or `lookup`).
///
/// Uses `LookupKeyOnlyAtlas` — `lookup` returns None, so if the wrong path fires
/// (unshaped branch or GlyphEmitter), no glyph instance is pushed and the test fails.
fn emit_cell_routes_builtin_glyph_via_builtin_branch() {
    // U+2500 (BOX DRAWINGS LIGHT HORIZONTAL) is a builtin geometric glyph.
    let builtin_ch = '\u{2500}';
    let input = single_cell_input();
    let atlas = LookupKeyOnlyAtlas;
    let shaped = ShapedFrame::new(1, 256);
    let mut frame = new_frame(&input);

    let c = cell(builtin_ch, CellFlags::empty());
    let mut ctx = shaped_ctx(&input, &atlas, &mut frame, &shaped);
    emit_cell(&c, 0.0, 0.0, &mut ctx);

    // Shaped builtin path: lookup_key returns Some → glyph pushed.
    // If unshaped else-branch fired, lookup returns None → 0 glyphs (test fails).
    assert_eq!(
        glyph_count(&frame),
        1,
        "shaped builtin must use lookup_key path"
    );
}

#[test]
/// BLINK cells apply `text_blink_opacity` to decoration draw (deco_alpha), so
/// underlines/strikethroughs/overlines fade alongside glyphs during blink.
fn emit_cell_applies_blink_alpha_to_decoration() {
    let mut input = single_cell_input();
    input.text_blink_opacity = 0.5;
    let atlas = TestAtlas;
    let mut frame = new_frame(&input);

    // BLINK + UNDERLINE: deco_alpha = text_blink_opacity = 0.5
    let c = cell('A', CellFlags::BLINK | CellFlags::UNDERLINE);
    let mut ctx = unshaped_ctx(&input, &atlas, &mut frame);
    emit_cell(&c, 0.0, 0.0, &mut ctx);

    // frame.backgrounds: [bg rect at y=0.0][underline rect at y=14.0]
    // underline_y = cell_top(0) + baseline(12) + underline_offset(2) = 14.0
    let b = frame.backgrounds.as_bytes();
    let n = bg_count(&frame);
    let underline_idx = (0..n)
        .find(|&i| (instance_y(b, i) - 14.0_f32).abs() < 0.1)
        .expect("BLINK+UNDERLINE must emit underline rect at y=14.0");

    let deco_alpha = instance_rect_alpha(b, underline_idx);
    assert!(
        (deco_alpha - 0.5).abs() < 0.01,
        "BLINK deco_alpha must equal text_blink_opacity=0.5, got {deco_alpha}"
    );

    // Negative pin: bg quad must NOT be dimmed by BLINK (bg alpha always 1.0).
    let bg_idx = (0..n)
        .find(|&i| (instance_y(b, i) - 0.0_f32).abs() < 0.1)
        .expect("bg rect at y=0 must exist");
    let bg_alpha = instance_rect_alpha(b, bg_idx);
    assert!(
        (bg_alpha - 1.0).abs() < 0.01,
        "bg alpha must be 1.0 even when BLINK, got {bg_alpha}"
    );
}
