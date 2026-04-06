//! Benchmarks for the prepare phase of the GPU rendering pipeline.
//!
//! Measures how long `prepare_frame_shaped_into()` takes to convert a
//! terminal snapshot into GPU-ready instance buffers. This is pure CPU
//! work — no wgpu types, no device, no queue.
//!
//! Baseline targets (from Section 23.5):
//! - 120x50 plain ASCII: <2ms
//! - 120x50 unique colors: <2ms
//! - 240x80 mixed content: <8ms
//!
//! Baseline results (2026-04-04, WSL2/llvmpipe, Ryzen 7 7700X):
//! - 120x50 plain:   ~942 µs (well under 2ms target)
//! - 120x50 colored:  ~775 µs (well under 2ms target)
//! - 240x80 mixed:   ~2.21 ms (well under 8ms target)

use std::collections::HashMap;

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

use oriterm_core::{
    CellFlags, Column, CursorShape, RenderableCell, RenderableContent, Rgb, TermMode,
};

use oriterm::gpu::{
    AtlasEntry, AtlasKind, AtlasLookup, CellMetrics, FaceIdx, FontRealm, FrameInput, FramePalette,
    GlyphStyle, PreparedFrame, RasterKey, ShapedFrame, SyntheticFlags, ViewportSize,
    prepare_frame_shaped_into,
};
use oriterm_ui::text::ShapedGlyph;

// ---------------------------------------------------------------------------
// Test atlas
// ---------------------------------------------------------------------------

/// Test atlas backed by a `HashMap` keyed on `RasterKey`.
struct BenchAtlas(HashMap<RasterKey, AtlasEntry>);

impl AtlasLookup for BenchAtlas {
    fn lookup(&self, _ch: char, _style: GlyphStyle) -> Option<&AtlasEntry> {
        None
    }

    fn lookup_key(&self, key: RasterKey) -> Option<&AtlasEntry> {
        self.0.get(&key)
    }
}

/// Create a deterministic atlas entry for a glyph ID.
fn make_entry(glyph_id: u16) -> AtlasEntry {
    let id = u32::from(glyph_id);
    AtlasEntry {
        page: 0,
        uv_x: (id % 16) as f32 / 16.0,
        uv_y: (id / 16) as f32 / 16.0,
        uv_w: 7.0 / 1024.0,
        uv_h: 14.0 / 1024.0,
        width: 7,
        height: 14,
        bearing_x: 1,
        bearing_y: 12,
        kind: AtlasKind::Mono,
    }
}

/// Build an atlas with entries for glyph IDs 1..=count.
fn build_atlas(count: u16) -> BenchAtlas {
    let mut map = HashMap::new();
    for id in 1..=count {
        let key = RasterKey {
            glyph_id: u32::from(id),
            face_idx: FaceIdx(0),
            weight: 0,
            size_q6: 960,
            synthetic: SyntheticFlags::empty(),
            hinted: true,
            subpx_x: 0,
            font_realm: FontRealm::Terminal,
        };
        map.insert(key, make_entry(id));
    }
    BenchAtlas(map)
}

// ---------------------------------------------------------------------------
// Frame construction helpers
// ---------------------------------------------------------------------------

/// Default cell colors (dark theme).
const FG: Rgb = Rgb {
    r: 211,
    g: 215,
    b: 207,
};
const BG: Rgb = Rgb {
    r: 30,
    g: 30,
    b: 46,
};
const PALETTE_BG: Rgb = Rgb { r: 0, g: 0, b: 0 };

/// Build a `FrameInput` with the given grid of cells.
fn build_frame_input(cols: usize, rows: usize, cells: Vec<RenderableCell>) -> FrameInput {
    let mut content = RenderableContent::default();
    content.cells = cells;
    content.cursor.visible = true;
    content.cursor.shape = CursorShape::Block;
    content.mode = TermMode::SHOW_CURSOR;
    content.all_dirty = true;

    FrameInput {
        content,
        viewport: ViewportSize::new(cols as u32 * 8, rows as u32 * 16),
        cell_size: CellMetrics::new(8.0, 16.0, 12.0, 2.0, 1.0, 4.0),
        content_cols: cols,
        content_rows: rows,
        palette: FramePalette {
            background: PALETTE_BG,
            foreground: FG,
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
        reverse_video: false,
        fg_dim: 1.0,
        text_blink_opacity: 1.0,
        subpixel_positioning: true,
        prompt_marker_rows: Vec::new(),
    }
}

/// Build a plain ASCII grid: every cell contains 'a'..'z' cycling.
fn plain_ascii_grid(cols: usize, rows: usize) -> Vec<RenderableCell> {
    let mut cells = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            let ch = (b'a' + ((row * cols + col) % 26) as u8) as char;
            cells.push(RenderableCell {
                line: row,
                column: Column(col),
                ch,
                fg: FG,
                bg: BG,
                flags: CellFlags::empty(),
                underline_color: None,
                has_hyperlink: false,
                hyperlink_uri: None,
                zerowidth: Vec::new(),
            });
        }
    }
    cells
}

/// Build a colored grid: every cell has a unique fg/bg derived from position.
fn colored_grid(cols: usize, rows: usize) -> Vec<RenderableCell> {
    let mut cells = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            let idx = row * cols + col;
            let ch = (b'A' + (idx % 26) as u8) as char;
            cells.push(RenderableCell {
                line: row,
                column: Column(col),
                ch,
                fg: Rgb {
                    r: (idx % 256) as u8,
                    g: ((idx * 7) % 256) as u8,
                    b: ((idx * 13) % 256) as u8,
                },
                bg: Rgb {
                    r: ((idx * 3) % 256) as u8,
                    g: ((idx * 5) % 256) as u8,
                    b: ((idx * 11) % 256) as u8,
                },
                flags: CellFlags::empty(),
                underline_color: None,
                has_hyperlink: false,
                hyperlink_uri: None,
                zerowidth: Vec::new(),
            });
        }
    }
    cells
}

/// Build a mixed content grid (ASCII + bold + italic + underline).
fn mixed_content_grid(cols: usize, rows: usize) -> Vec<RenderableCell> {
    let mut cells = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            let idx = row * cols + col;
            let ch = (b'a' + (idx % 26) as u8) as char;
            let flags = match idx % 4 {
                0 => CellFlags::empty(),
                1 => CellFlags::BOLD,
                2 => CellFlags::ITALIC,
                3 => CellFlags::UNDERLINE,
                _ => unreachable!(),
            };
            cells.push(RenderableCell {
                line: row,
                column: Column(col),
                ch,
                fg: FG,
                bg: BG,
                flags,
                underline_color: None,
                has_hyperlink: false,
                hyperlink_uri: None,
                zerowidth: Vec::new(),
            });
        }
    }
    cells
}

/// Build a `ShapedFrame` that maps each cell to a glyph ID.
///
/// Glyph IDs are assigned 1..=26 based on the character ('a'=1, 'b'=2, etc.).
/// This simulates a real shaped frame where each character has been resolved
/// to a glyph ID by the shaper.
fn build_shaped_frame(cols: usize, rows: usize, cells: &[RenderableCell]) -> ShapedFrame {
    let size_q6 = 960; // 15pt * 64
    let mut shaped = ShapedFrame::new(cols, size_q6);

    for row in 0..rows {
        let mut glyphs = Vec::with_capacity(cols);
        let mut col_starts = Vec::with_capacity(cols);
        let mut col_map = vec![None; cols];

        for col in 0..cols {
            let cell = &cells[row * cols + col];
            if cell.ch == ' ' || cell.ch == '\0' {
                continue;
            }
            let glyph_idx = glyphs.len();
            col_starts.push(col);
            col_map[col] = Some(glyph_idx);

            // Map character to glyph ID (1-based).
            let glyph_id = match cell.ch {
                'a'..='z' => (cell.ch as u16 - b'a' as u16) + 1,
                'A'..='Z' => (cell.ch as u16 - b'A' as u16) + 1,
                _ => 1,
            };

            glyphs.push(ShapedGlyph {
                glyph_id,
                face_index: 0,
                x_advance: 8.0,
                x_offset: 0.0,
                y_offset: 0.0,
                synthetic: 0,
            });
        }
        shaped.push_row(&glyphs, &col_starts, &col_map);
    }
    shaped
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_prepare_plain(c: &mut Criterion) {
    let cols = 120;
    let rows = 50;
    let cells = plain_ascii_grid(cols, rows);
    let input = build_frame_input(cols, rows, cells.clone());
    let shaped = build_shaped_frame(cols, rows, &cells);
    let atlas = build_atlas(26);
    let opacity = f64::from(input.palette.opacity);
    let mut frame = PreparedFrame::new(input.viewport, input.palette.background, opacity);

    c.bench_with_input(
        BenchmarkId::new("prepare_shaped", "120x50_plain"),
        &(&input, &shaped, &atlas),
        |b, (input, shaped, atlas)| {
            b.iter(|| {
                prepare_frame_shaped_into(
                    black_box(input),
                    black_box(*atlas as &dyn AtlasLookup),
                    black_box(shaped),
                    black_box(&mut frame),
                    (0.0, 0.0),
                    1.0,
                );
            });
        },
    );
}

fn bench_prepare_colored(c: &mut Criterion) {
    let cols = 120;
    let rows = 50;
    let cells = colored_grid(cols, rows);
    let input = build_frame_input(cols, rows, cells.clone());
    let shaped = build_shaped_frame(cols, rows, &cells);
    let atlas = build_atlas(26);
    let opacity = f64::from(input.palette.opacity);
    let mut frame = PreparedFrame::new(input.viewport, input.palette.background, opacity);

    c.bench_with_input(
        BenchmarkId::new("prepare_shaped", "120x50_colored"),
        &(&input, &shaped, &atlas),
        |b, (input, shaped, atlas)| {
            b.iter(|| {
                prepare_frame_shaped_into(
                    black_box(input),
                    black_box(*atlas as &dyn AtlasLookup),
                    black_box(shaped),
                    black_box(&mut frame),
                    (0.0, 0.0),
                    1.0,
                );
            });
        },
    );
}

fn bench_prepare_large(c: &mut Criterion) {
    let cols = 240;
    let rows = 80;
    let cells = mixed_content_grid(cols, rows);
    let input = build_frame_input(cols, rows, cells.clone());
    let shaped = build_shaped_frame(cols, rows, &cells);
    let atlas = build_atlas(26);
    let opacity = f64::from(input.palette.opacity);
    let mut frame = PreparedFrame::new(input.viewport, input.palette.background, opacity);

    c.bench_with_input(
        BenchmarkId::new("prepare_shaped", "240x80_mixed"),
        &(&input, &shaped, &atlas),
        |b, (input, shaped, atlas)| {
            b.iter(|| {
                prepare_frame_shaped_into(
                    black_box(input),
                    black_box(*atlas as &dyn AtlasLookup),
                    black_box(shaped),
                    black_box(&mut frame),
                    (0.0, 0.0),
                    1.0,
                );
            });
        },
    );
}

criterion_group!(
    benches,
    bench_prepare_plain,
    bench_prepare_colored,
    bench_prepare_large
);
criterion_main!(benches);
