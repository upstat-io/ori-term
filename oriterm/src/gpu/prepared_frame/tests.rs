//! Unit tests for prepared frame.

use oriterm_core::Rgb;

use super::{OverlayDrawRange, PreparedFrame};
use crate::gpu::frame_input::ViewportSize;
use crate::gpu::instance_writer::ScreenRect;

const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
const WHITE: Rgb = Rgb {
    r: 255,
    g: 255,
    b: 255,
};

/// Default test viewport (not meaningful for these tests).
const VP: ViewportSize = ViewportSize {
    width: 640,
    height: 480,
};

// --- Construction ---

#[test]
fn new_frame_is_empty() {
    let frame = PreparedFrame::new(VP, BLACK, 1.0);
    assert!(frame.is_empty());
    assert_eq!(frame.total_instances(), 0);
    assert_eq!(frame.prev_dispatch_fingerprint, None);
}

#[test]
fn with_capacity_starts_empty() {
    let frame = PreparedFrame::with_capacity(VP, 80, 24, BLACK, 1.0);
    assert!(frame.is_empty());
    assert_eq!(frame.total_instances(), 0);
    assert_eq!(frame.prev_dispatch_fingerprint, None);
}

// --- Clear color ---

#[test]
fn clear_color_opaque_black() {
    let frame = PreparedFrame::new(VP, BLACK, 1.0);
    assert_eq!(frame.clear_color, [0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn clear_color_opaque_white() {
    let frame = PreparedFrame::new(VP, WHITE, 1.0);
    assert_eq!(frame.clear_color, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn clear_color_half_transparent() {
    let frame = PreparedFrame::new(VP, WHITE, 0.5);
    // Premultiplied: each channel * 0.5.
    assert_eq!(frame.clear_color, [0.5, 0.5, 0.5, 0.5]);
}

#[test]
fn clear_color_fully_transparent() {
    let frame = PreparedFrame::new(VP, WHITE, 0.0);
    assert_eq!(frame.clear_color, [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn set_clear_color_updates() {
    let mut frame = PreparedFrame::new(VP, BLACK, 1.0);
    frame.set_clear_color(WHITE, 0.5);
    assert_eq!(frame.clear_color, [0.5, 0.5, 0.5, 0.5]);
}

// --- Lifecycle ---

#[test]
fn populate_and_count() {
    let mut frame = PreparedFrame::new(VP, BLACK, 1.0);

    let r1 = ScreenRect {
        x: 0.0,
        y: 0.0,
        w: 8.0,
        h: 16.0,
    };
    let r2 = ScreenRect {
        x: 8.0,
        y: 0.0,
        w: 8.0,
        h: 16.0,
    };
    frame.backgrounds.push_rect(r1, BLACK, 1.0);
    frame.backgrounds.push_rect(r2, BLACK, 1.0);
    frame.glyphs.push_glyph(
        r1,
        [0.0; 4],
        WHITE,
        1.0,
        0,
        crate::gpu::instance_writer::CLIP_UNCLIPPED,
    );
    frame.cursors.push_cursor(r1, WHITE, 1.0);

    assert!(!frame.is_empty());
    assert_eq!(frame.total_instances(), 4);
    assert_eq!(frame.backgrounds.len(), 2);
    assert_eq!(frame.glyphs.len(), 1);
    assert_eq!(frame.cursors.len(), 1);
}

#[test]
fn clear_resets_all_buffers() {
    let mut frame = PreparedFrame::new(VP, BLACK, 1.0);
    let r = ScreenRect {
        x: 0.0,
        y: 0.0,
        w: 8.0,
        h: 16.0,
    };
    frame.backgrounds.push_rect(r, BLACK, 1.0);
    frame.glyphs.push_glyph(
        r,
        [0.0; 4],
        WHITE,
        1.0,
        0,
        crate::gpu::instance_writer::CLIP_UNCLIPPED,
    );
    frame.cursors.push_cursor(r, WHITE, 1.0);

    frame.clear();

    assert!(frame.is_empty());
    assert_eq!(frame.total_instances(), 0);
}

#[test]
fn clear_and_reuse() {
    let mut frame = PreparedFrame::new(VP, BLACK, 1.0);

    // First frame.
    let r = ScreenRect {
        x: 0.0,
        y: 0.0,
        w: 8.0,
        h: 16.0,
    };
    frame.backgrounds.push_rect(r, BLACK, 1.0);
    assert_eq!(frame.total_instances(), 1);

    // Clear for next frame.
    frame.clear();
    assert!(frame.is_empty());

    // Second frame.
    frame.glyphs.push_glyph(
        r,
        [0.0; 4],
        WHITE,
        1.0,
        0,
        crate::gpu::instance_writer::CLIP_UNCLIPPED,
    );
    let r2 = ScreenRect {
        x: 8.0,
        y: 0.0,
        w: 8.0,
        h: 16.0,
    };
    frame.glyphs.push_glyph(
        r2,
        [0.0; 4],
        WHITE,
        1.0,
        0,
        crate::gpu::instance_writer::CLIP_UNCLIPPED,
    );
    assert_eq!(frame.total_instances(), 2);
}

// --- extend_from with overlays ---

/// Helper to push a dummy UI rect to an overlay buffer.
fn push_dummy_overlay_rect(frame: &mut PreparedFrame) {
    let r = ScreenRect {
        x: 0.0,
        y: 0.0,
        w: 100.0,
        h: 30.0,
    };
    frame.overlay_rects.push_ui_rect(
        r,
        [1.0, 0.0, 0.0, 1.0],
        [0.0; 4],
        [0.0; 4],
        [[0.0; 4]; 4],
        [0.0, 0.0, 640.0, 480.0],
    );
}

#[test]
fn extend_from_shifts_overlay_draw_ranges_correctly() {
    // Self frame has 2 overlay rects in one overlay.
    let mut self_frame = PreparedFrame::new(VP, BLACK, 1.0);
    push_dummy_overlay_rect(&mut self_frame);
    push_dummy_overlay_rect(&mut self_frame);
    self_frame.overlay_draw_ranges.push(OverlayDrawRange {
        rects: (0, 2),
        mono: (0, 0),
        subpixel: (0, 0),
        color: (0, 0),
    });

    // Other frame has 1 overlay rect in one overlay.
    let mut other = PreparedFrame::new(VP, BLACK, 1.0);
    push_dummy_overlay_rect(&mut other);
    other.overlay_draw_ranges.push(OverlayDrawRange {
        rects: (0, 1),
        mono: (0, 0),
        subpixel: (0, 0),
        color: (0, 0),
    });

    self_frame.extend_from(&other);

    // Self now has 3 overlay rects total.
    assert_eq!(self_frame.overlay_rects.len(), 3);
    // Two overlay draw ranges: the original and the shifted one.
    assert_eq!(self_frame.overlay_draw_ranges.len(), 2);
    // Original range is unchanged.
    assert_eq!(self_frame.overlay_draw_ranges[0].rects, (0, 2));
    // Shifted range: base was 2 (self had 2 rects before append),
    // so other's (0,1) becomes (2,3).
    assert_eq!(self_frame.overlay_draw_ranges[1].rects, (2, 3));
}

// ── Regression pin against re-introducing enumerated prev_* fields ──
//
// The previous design had 9 prev_* fields on PreparedFrame used solely by the
// dispatch predicate; the fingerprint refactor replaced them with a single
// prev_dispatch_fingerprint field. This test asserts the 9 identifiers no
// longer appear in the source — re-introducing any of them would silently
// re-create the sync-point drift hazard.

/// Regression: BUG-06-030 — 9 enumerated prev_* fields replaced with single
/// content-aware fingerprint; this test prevents accidental re-introduction.
/// See: bug-tracker/plans/BUG-06-030/
#[test]
fn enumerated_prev_fields_removed() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    let path = std::path::PathBuf::from(manifest)
        .join("src")
        .join("gpu")
        .join("prepared_frame")
        .join("mod.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));

    // Field declarations look like `pub(crate) prev_<name>:` or `pub(crate) prev_<name> :`.
    // Use the punctuated form to avoid matching inline doc-comment mentions.
    for forbidden in &[
        "prev_text_blink_opacity:",
        "prev_palette_opacity:",
        "prev_cell_size:",
        "prev_origin:",
        "prev_content_cols:",
        "prev_content_rows:",
        "prev_fg_dim:",
        "prev_subpixel_positioning:",
        "prev_search_fingerprint:",
    ] {
        assert!(
            !source.contains(forbidden),
            "regression: {} re-introduced as a field declaration in {}. \
             The fingerprint refactor removed these in favor of prev_dispatch_fingerprint; \
             re-adding any of them re-creates the parallel-sync-point drift hazard.",
            forbidden,
            path.display()
        );
    }
}

/// Regression: BUG-06-040 — `prev_cursor_line: Option<usize>` field was
/// renamed to `prev_resolved_cursor: Option<RenderableCursor>` (visibility-
/// canonicalized SSOT). This test asserts the old field name no longer
/// appears in field-access patterns under `oriterm/src/gpu/`. Excludes
/// (a) string literals (assertion messages, doc comments) — these don't
/// affect correctness — and (b) the `selection_damage::dirty_set`
/// parameter named `prev_cursor_line: Option<usize>`, which is local to
/// that function's signature and intentionally retained.
/// See: bug-tracker/plans/completed/BUG-06-040/
#[test]
fn prev_cursor_line_field_access_removed() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    let gpu_root = std::path::PathBuf::from(manifest).join("src").join("gpu");

    // Walk every .rs file under src/gpu/ and check field-access patterns.
    let mut offending = Vec::new();
    walk_rs_files(&gpu_root, &mut |path: &std::path::Path| {
        let Ok(source) = std::fs::read_to_string(path) else {
            return;
        };
        // Field-access patterns — must NOT match string literals or
        // doc-comment prose. Each marker is the literal field-access
        // syntax that a code consumer would emit.
        for needle in &[
            "frame.prev_cursor_line",
            "out.prev_cursor_line",
            "self.prepared.prev_cursor_line",
            "pub(crate) prev_cursor_line:",
        ] {
            if source.contains(needle) {
                offending.push(format!("{}: {needle}", path.display()));
            }
        }
    });

    assert!(
        offending.is_empty(),
        "regression: prev_cursor_line field-access patterns survived the rename to \
         prev_resolved_cursor. Offending sites:\n  {}",
        offending.join("\n  ")
    );
}

fn walk_rs_files(dir: &std::path::Path, visitor: &mut dyn FnMut(&std::path::Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_rs_files(&path, visitor);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            // Skip test files — they may contain source-scan needles
            // as string literals (this very test, for example) and the
            // SSOT contract is about production code, not test fixtures.
            if path.file_name().and_then(|s| s.to_str()) == Some("tests.rs") {
                continue;
            }
            visitor(&path);
        }
    }
}
