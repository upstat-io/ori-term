//! Unit tests for raster-key generation helpers.

use oriterm_ui::text::ShapedGlyph;

use super::super::super::prepare::ShapedFrame;
use super::{grid_raster_keys, scene_raster_keys};
use crate::font::build_col_glyph_map;

// --- Raster key: subpixel positioning ---

#[test]
fn grid_raster_keys_disabled_subpx_all_zero() {
    let size_q6 = 768;
    let mut sf = ShapedFrame::new(3, size_q6);
    let glyphs = vec![
        ShapedGlyph {
            glyph_id: 42,
            face_index: 0,
            synthetic: 0,
            x_advance: 8.0,
            x_offset: 0.3, // fractional — would produce non-zero subpx_x
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 43,
            face_index: 0,
            synthetic: 0,
            x_advance: 8.0,
            x_offset: 0.7,
            y_offset: 0.0,
        },
    ];
    let col_starts = vec![0, 1];
    let mut col_map = Vec::new();
    build_col_glyph_map(&col_starts, 3, &mut col_map);
    sf.push_row(&glyphs, &col_starts, &col_map);

    // subpixel_positioning = false → all keys have subpx_x == 0.
    let keys: Vec<_> = grid_raster_keys(&sf, true, false).collect();
    assert!(keys.len() >= 2);
    for key in &keys {
        assert_eq!(
            key.subpx_x, 0,
            "expected subpx_x=0 when positioning disabled"
        );
    }
}

#[test]
fn grid_raster_keys_enabled_subpx_nonzero() {
    let size_q6 = 768;
    let mut sf = ShapedFrame::new(2, size_q6);
    let glyphs = vec![ShapedGlyph {
        glyph_id: 42,
        face_index: 0,
        synthetic: 0,
        x_advance: 8.0,
        x_offset: 0.3, // fractional → non-zero subpx_x with positioning enabled
        y_offset: 0.0,
    }];
    let col_starts = vec![0];
    let mut col_map = Vec::new();
    build_col_glyph_map(&col_starts, 2, &mut col_map);
    sf.push_row(&glyphs, &col_starts, &col_map);

    let keys: Vec<_> = grid_raster_keys(&sf, true, true).collect();
    assert!(!keys.is_empty());
    // 0.3 maps to subpx bin 1 (quarter-pixel: 0.25-0.5 → bin 1).
    assert_ne!(
        keys[0].subpx_x, 0,
        "expected non-zero subpx_x with 0.3 offset"
    );
}

#[test]
fn scene_raster_keys_disabled_subpx_all_zero() {
    use oriterm_ui::draw::Scene;
    use oriterm_ui::geometry::Point;
    use oriterm_ui::text::ShapedText;

    let mut scene = Scene::new();
    let glyphs = vec![
        ShapedGlyph {
            glyph_id: 65,
            face_index: 0,
            synthetic: 0,
            x_advance: 7.0,
            x_offset: 0.3,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 66,
            face_index: 0,
            synthetic: 0,
            x_advance: 7.0,
            x_offset: 0.7,
            y_offset: 0.0,
        },
    ];
    let width: f32 = glyphs.iter().map(|g| g.x_advance).sum();
    let st = ShapedText::new(glyphs, width, 14.0, 12.0, 768, 400);
    scene.push_text(Point::new(10.0, 20.0), st, oriterm_ui::color::Color::WHITE);

    let mut keys = Vec::new();
    scene_raster_keys(&scene, true, 1.0, &mut keys, false);

    assert!(
        keys.len() >= 2,
        "expected at least 2 keys, got {}",
        keys.len()
    );
    for key in &keys {
        assert_eq!(
            key.subpx_x, 0,
            "expected subpx_x=0 when positioning disabled"
        );
    }
}
