use oriterm_core::{CellFlags, Column, RenderableCell, RenderableCursor};

use crate::font::{GlyphStyle, RasterKey};
use crate::gpu::atlas::{AtlasEntry, AtlasKind};
use crate::gpu::frame_input::FrameInput;
use crate::gpu::instance_writer::INSTANCE_SIZE;
use crate::gpu::prepare::shaped_frame::ShapedFrame;
use crate::gpu::prepared_frame::PreparedFrame;

use super::super::AtlasLookup;
use super::super::resolve::CellColorContext;
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
        fg_alpha: 255,
        bg_alpha: 255,
        underline_alpha: 255,
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
        color_ctx: CellColorContext {
            palette: &input.palette,
            sel: input.selection.as_ref(),
            search: input.search.as_ref(),
            cursor: idle_cursor(),
            cursor_opacity: 1.0,
        },
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
        color_ctx: CellColorContext {
            palette: &input.palette,
            sel: input.selection.as_ref(),
            search: input.search.as_ref(),
            cursor: idle_cursor(),
            cursor_opacity: 1.0,
        },
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
/// BLINK reduces glyph alpha (cell_dim) but does NOT affect bg alpha — an
/// opaque cell (bg_alpha=255) keeps bg alpha at 1.0 regardless of blink.
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

    // Regression guard: an OPAQUE cell's bg stays at alpha=1.0 regardless of
    // blink state. push_rect writes the color into the bg slot (offset 60).
    assert_eq!(bg_count(&frame), 1, "bg pushed even when blink=0.5");
    let b = frame.backgrounds.as_bytes();
    let bg_alpha = instance_rect_alpha(b, 0);
    assert!(
        (bg_alpha - 1.0).abs() < 0.01,
        "opaque cell bg alpha must remain 1.0 even for BLINK cells, got {bg_alpha}"
    );
}

#[test]
/// A translucent cell (SGR mode-6 bg_alpha=128) emits its bg quad at the
/// resolved alpha (128/255 ≈ 0.502), NOT a hardcoded 1.0. Blink composes by
/// multiply and must not clobber the cell alpha.
fn emit_cell_threads_translucent_bg_alpha_to_rect() {
    let input = single_cell_input();
    let atlas = TestAtlas;
    let mut frame = new_frame(&input);

    let mut c = cell('A', CellFlags::empty());
    c.bg_alpha = 128;
    let mut ctx = unshaped_ctx(&input, &atlas, &mut frame);
    emit_cell(&c, 0.0, 0.0, &mut ctx);

    assert_eq!(bg_count(&frame), 1, "translucent cell still emits bg quad");
    let b = frame.backgrounds.as_bytes();
    let bg_alpha = instance_rect_alpha(b, 0);
    assert!(
        (bg_alpha - 128.0 / 255.0).abs() < 0.01,
        "bg alpha must be 128/255 ≈ 0.502, got {bg_alpha}"
    );
    // Negative: must NOT be the pre-fix hardcoded opaque 1.0.
    assert!(
        (bg_alpha - 1.0).abs() > 0.05,
        "translucent bg alpha must NOT be hardcoded 1.0 (pre-fix bug)"
    );
}

#[test]
/// F3 regression: a cell with the DEFAULT bg color (== palette.background) but
/// an explicit SGR mode-6 translucent `bg_alpha < 1.0` MUST still emit its bg
/// quad at the resolved alpha. The pre-fix gate only checked
/// `bg != palette.background`, so a default-colored translucent cell got NO bg
/// quad and its alpha was silently dropped.
fn emit_cell_emits_bg_quad_for_default_color_translucent_cell() {
    use oriterm_core::Rgb;

    let input = single_cell_input();
    let atlas = TestAtlas;
    let mut frame = new_frame(&input);

    // test_grid palette background is (0,0,0). Build a cell whose bg matches
    // it exactly but carries a translucent SGR mode-6 alpha.
    let palette_bg = input.palette.background;
    assert_eq!(
        palette_bg,
        Rgb { r: 0, g: 0, b: 0 },
        "test fixture invariant"
    );
    let mut c = cell('A', CellFlags::empty());
    c.bg = palette_bg;
    c.bg_alpha = 128;

    let mut ctx = unshaped_ctx(&input, &atlas, &mut frame);
    emit_cell(&c, 0.0, 0.0, &mut ctx);

    assert_eq!(
        bg_count(&frame),
        1,
        "default-colored translucent cell must still emit its bg quad"
    );
    let b = frame.backgrounds.as_bytes();
    let bg_alpha = instance_rect_alpha(b, 0);
    assert!(
        (bg_alpha - 128.0 / 255.0).abs() < 0.01,
        "default-colored translucent cell bg alpha must be 128/255 ≈ 0.502, got {bg_alpha}"
    );
}

#[test]
/// F3 boundary: a default-colored, fully-OPAQUE cell (bg ==
/// palette.background, bg_alpha=255) emits NO bg quad — the window clear color
/// (theme opacity for glass/acrylic) shows through. The F3 gate widening must
/// not start emitting quads for ordinary default-bg opaque cells.
fn emit_cell_skips_bg_quad_for_default_color_opaque_cell() {
    let input = single_cell_input();
    let atlas = TestAtlas;
    let mut frame = new_frame(&input);

    let mut c = cell('A', CellFlags::empty());
    c.bg = input.palette.background;
    c.bg_alpha = 255;

    let mut ctx = unshaped_ctx(&input, &atlas, &mut frame);
    emit_cell(&c, 0.0, 0.0, &mut ctx);

    assert_eq!(
        bg_count(&frame),
        0,
        "default-colored OPAQUE cell must emit no bg quad (clear color shows through)"
    );
}

#[test]
/// A fully-transparent cell bg (bg_alpha=0) emits a bg quad at alpha 0.0 —
/// the underlying surface shows through.
fn emit_cell_threads_zero_bg_alpha_to_rect() {
    let input = single_cell_input();
    let atlas = TestAtlas;
    let mut frame = new_frame(&input);

    let mut c = cell('A', CellFlags::empty());
    c.bg_alpha = 0;
    let mut ctx = unshaped_ctx(&input, &atlas, &mut frame);
    emit_cell(&c, 0.0, 0.0, &mut ctx);

    let b = frame.backgrounds.as_bytes();
    let bg_alpha = instance_rect_alpha(b, 0);
    assert!(
        bg_alpha.abs() < 0.01,
        "fully-transparent bg alpha must be 0.0, got {bg_alpha}"
    );
}

#[test]
/// SGR mode-6 fg alpha multiplies the glyph alpha. At fg_alpha=128 (≈0.502)
/// with no blink/dim, the glyph alpha is ≈0.502.
fn emit_cell_threads_fg_alpha_to_glyph() {
    let input = single_cell_input();
    let atlas = TestAtlas;
    let mut frame = new_frame(&input);

    let mut c = cell('A', CellFlags::empty());
    c.fg_alpha = 128;
    let mut ctx = unshaped_ctx(&input, &atlas, &mut frame);
    emit_cell(&c, 0.0, 0.0, &mut ctx);

    assert_eq!(glyph_count(&frame), 1, "glyph emitted");
    let g = frame.glyphs.as_bytes();
    let glyph_alpha = instance_glyph_alpha(g, 0);
    assert!(
        (glyph_alpha - 128.0 / 255.0).abs() < 0.01,
        "glyph alpha must be cell_dim(1.0) * fg_alpha(128/255) ≈ 0.502, got {glyph_alpha}"
    );
}

#[test]
/// Blink × fg alpha compose by MULTIPLY. A BLINK cell
/// (text_blink_opacity=0.5) with fg_alpha=128 → 0.5 * 0.502 ≈ 0.251.
fn emit_cell_composes_blink_and_fg_alpha_by_multiply() {
    let mut input = single_cell_input();
    input.text_blink_opacity = 0.5;
    let atlas = TestAtlas;
    let mut frame = new_frame(&input);

    let mut c = cell('A', CellFlags::BLINK);
    c.fg_alpha = 128;
    let mut ctx = unshaped_ctx(&input, &atlas, &mut frame);
    emit_cell(&c, 0.0, 0.0, &mut ctx);

    let g = frame.glyphs.as_bytes();
    let glyph_alpha = instance_glyph_alpha(g, 0);
    let expected = 0.5 * (128.0 / 255.0);
    assert!(
        (glyph_alpha - expected).abs() < 0.01,
        "blink×fg_alpha must multiply: 0.5 * 128/255 ≈ {expected}, got {glyph_alpha}"
    );
    // Negative: must NOT be min(0.5, 0.502)=0.5 (multiply, not min).
    assert!(
        (glyph_alpha - 0.5).abs() > 0.05,
        "must compose by multiply, not min(blink, fg_alpha)"
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
/// Regression guard: decoration y is anchored to cell-top, NOT to the glyph_y offset.
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

    // Regression guard: an OPAQUE cell's bg quad must NOT be dimmed by BLINK
    // (opaque bg_alpha=255 → 1.0).
    let bg_idx = (0..n)
        .find(|&i| (instance_y(b, i) - 0.0_f32).abs() < 0.1)
        .expect("bg rect at y=0 must exist");
    let bg_alpha = instance_rect_alpha(b, bg_idx);
    assert!(
        (bg_alpha - 1.0).abs() < 0.01,
        "opaque cell bg alpha must be 1.0 even when BLINK, got {bg_alpha}"
    );
}

#[test]
/// SGR mode-6 underline alpha multiplies the decoration alpha.
/// underline_alpha=128 (≈0.502) with no blink → deco alpha ≈ 0.502.
fn emit_cell_threads_underline_alpha_to_decoration() {
    let input = single_cell_input();
    let atlas = TestAtlas;
    let mut frame = new_frame(&input);

    let mut c = cell('A', CellFlags::UNDERLINE);
    c.underline_alpha = 128;
    let mut ctx = unshaped_ctx(&input, &atlas, &mut frame);
    emit_cell(&c, 0.0, 0.0, &mut ctx);

    let b = frame.backgrounds.as_bytes();
    let n = bg_count(&frame);
    let underline_idx = (0..n)
        .find(|&i| (instance_y(b, i) - 14.0_f32).abs() < 0.1)
        .expect("UNDERLINE must emit underline rect at y=14.0");
    let deco_alpha = instance_rect_alpha(b, underline_idx);
    assert!(
        (deco_alpha - 128.0 / 255.0).abs() < 0.01,
        "deco alpha must be 1.0 * underline_alpha(128/255) ≈ 0.502, got {deco_alpha}"
    );
    assert!(
        (deco_alpha - 1.0).abs() > 0.05,
        "translucent underline alpha must NOT be hardcoded 1.0"
    );
}

#[test]
/// Regression: BUG-06-106 — `emit_cell::deco_alpha` must include the `ctx.fg_dim`
/// factor so decorations (underline/strikethrough/overline) dim with pane focus
/// alongside glyphs. HEAD's `deco_alpha = blink_mul * underline_alpha` omits
/// `fg_dim`, leaving decorations full-bright on an unfocused pane while glyphs
/// (`cell_dim = ctx.fg_dim * blink_mul * fg_alpha`) correctly dim — a visual
/// consistency defect this test pins. Cite this comment if the rule fires.
fn unfocused_pane_dims_underline_decoration_alongside_glyph() {
    let mut input = single_cell_input();
    input.fg_dim = 0.6; // unfocused pane

    let atlas = TestAtlas;
    let mut frame = new_frame(&input);
    let underlined = cell('A', CellFlags::UNDERLINE);
    let mut ctx = unshaped_ctx(&input, &atlas, &mut frame);
    emit_cell(&underlined, 0.0, 0.0, &mut ctx);

    // Locate the underline rect in backgrounds. From the existing
    // `emit_cell_anchors_decoration_to_cell_top_y` precedent: underline_y =
    // cell_top_y + baseline + underline_offset = 0 + 12.0 + 2.0 = 14.0
    // (test_grid metrics). bg_count = 2: cell-bg rect (y=0) + underline rect (y=14).
    let expected_underline_y = 14.0_f32;
    let b = frame.backgrounds.as_bytes();
    let n = bg_count(&frame);
    let underline_idx = (0..n)
        .find(|&i| (instance_y(b, i) - expected_underline_y).abs() < 0.1)
        .expect("underline rect must be emitted at y=14.0");

    let underline_alpha = instance_rect_alpha(b, underline_idx);
    // Expected: deco_alpha = fg_dim * blink_mul * underline_alpha
    //                     = 0.6 * 1.0  * 1.0             = 0.6.
    // Pre-fix HEAD: deco_alpha = blink_mul * underline_alpha = 1.0 (the bug).
    assert!(
        (underline_alpha - 0.6).abs() < 0.01,
        "unfocused-pane underline alpha must be fg_dim*blink_mul*underline_alpha = 0.6; \
         got {underline_alpha}. A value of 1.0 means deco_alpha is missing the \
         ctx.fg_dim factor (see test doc comment)"
    );
}
