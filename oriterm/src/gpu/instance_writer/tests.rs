//! Unit tests for GPU instance buffer writer.

use oriterm_core::Rgb;

use super::{
    CLIP_UNCLIPPED, GlyphInstance, GlyphInstanceBg, INSTANCE_SIZE, InstanceKind, InstanceWriter,
    ScreenRect,
};
use crate::gpu::srgb_to_linear;

/// Read a little-endian `f32` from the given byte offset.
fn read_f32(buf: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

/// Read a little-endian `u32` from the given byte offset.
fn read_u32(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

const WHITE: Rgb = Rgb {
    r: 255,
    g: 255,
    b: 255,
};
const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
const RED: Rgb = Rgb { r: 255, g: 0, b: 0 };

// --- Record layout ---

#[test]
fn instance_size_is_96_bytes() {
    assert_eq!(INSTANCE_SIZE, 96);
}

#[test]
fn push_rect_produces_96_byte_record() {
    let mut w = InstanceWriter::new();
    w.push_rect(
        ScreenRect {
            x: 10.0,
            y: 20.0,
            w: 8.0,
            h: 16.0,
        },
        BLACK,
        1.0,
    );

    assert_eq!(w.len(), 1);
    assert_eq!(w.byte_len(), 96);
}

#[test]
fn push_rect_field_offsets() {
    let mut w = InstanceWriter::new();
    w.push_rect(
        ScreenRect {
            x: 10.0,
            y: 20.0,
            w: 8.0,
            h: 16.0,
        },
        RED,
        0.5,
    );

    let rec = w.as_bytes();

    // Position.
    assert_eq!(read_f32(rec, 0), 10.0);
    assert_eq!(read_f32(rec, 4), 20.0);

    // Size.
    assert_eq!(read_f32(rec, 8), 8.0);
    assert_eq!(read_f32(rec, 12), 16.0);

    // UV (zeroed for rects).
    assert_eq!(read_f32(rec, 16), 0.0);
    assert_eq!(read_f32(rec, 20), 0.0);
    assert_eq!(read_f32(rec, 24), 0.0);
    assert_eq!(read_f32(rec, 28), 0.0);

    // FG (zeroed for rects).
    assert_eq!(read_f32(rec, 32), 0.0);
    assert_eq!(read_f32(rec, 36), 0.0);
    assert_eq!(read_f32(rec, 40), 0.0);
    assert_eq!(read_f32(rec, 44), 0.0);

    // BG = RED at alpha 0.5, STRAIGHT (color un-premultiplied; the shader
    // premultiplies). RED linear R == 1.0, so straight R = 1.0; alpha = 0.5.
    assert_eq!(read_f32(rec, 48), 1.0); // R straight
    assert_eq!(read_f32(rec, 52), 0.0); // G
    assert_eq!(read_f32(rec, 56), 0.0); // B
    assert_eq!(read_f32(rec, 60), 0.5); // A straight

    // Kind = Rect (0).
    assert_eq!(read_u32(rec, 64), 0);

    // Remaining fields zeroed for plain rects.
    assert_eq!(read_u32(rec, 68), 0);
    assert_eq!(read_f32(rec, 72), 0.0);
    assert_eq!(read_f32(rec, 76), 0.0);
}

#[test]
fn push_glyph_field_offsets() {
    let mut w = InstanceWriter::new();
    let uv = [0.25, 0.5, 0.125, 0.25];
    w.push_glyph(
        ScreenRect {
            x: 100.0,
            y: 200.0,
            w: 8.0,
            h: 16.0,
        },
        GlyphInstance {
            uv,
            fg: WHITE,
            alpha: 1.0,
            atlas_page: 0,
            clip: CLIP_UNCLIPPED,
        },
    );

    let rec = w.as_bytes();

    // Position.
    assert_eq!(read_f32(rec, 0), 100.0);
    assert_eq!(read_f32(rec, 4), 200.0);

    // UV.
    assert_eq!(read_f32(rec, 16), 0.25);
    assert_eq!(read_f32(rec, 20), 0.5);
    assert_eq!(read_f32(rec, 24), 0.125);
    assert_eq!(read_f32(rec, 28), 0.25);

    // FG = WHITE at alpha 1.0.
    assert_eq!(read_f32(rec, 32), 1.0);
    assert_eq!(read_f32(rec, 36), 1.0);
    assert_eq!(read_f32(rec, 40), 1.0);
    assert_eq!(read_f32(rec, 44), 1.0);

    // BG (zeroed for glyphs).
    assert_eq!(read_f32(rec, 48), 0.0);
    assert_eq!(read_f32(rec, 52), 0.0);
    assert_eq!(read_f32(rec, 56), 0.0);
    assert_eq!(read_f32(rec, 60), 0.0);

    // Kind = Glyph (1).
    assert_eq!(read_u32(rec, 64), 1);
}

#[test]
fn push_cursor_field_offsets() {
    let mut w = InstanceWriter::new();
    let green = Rgb { r: 0, g: 128, b: 0 };
    w.push_cursor(
        ScreenRect {
            x: 50.0,
            y: 100.0,
            w: 8.0,
            h: 16.0,
        },
        green,
        0.75,
    );

    let rec = w.as_bytes();

    // FG (zeroed for cursors — color goes in BG for bg_pipeline rendering).
    assert_eq!(read_f32(rec, 32), 0.0);
    assert_eq!(read_f32(rec, 36), 0.0);
    assert_eq!(read_f32(rec, 40), 0.0);
    assert_eq!(read_f32(rec, 44), 0.0);

    // BG = green at alpha 0.75 (sRGB→linear), STRAIGHT color + straight alpha
    // (the shader premultiplies).
    assert_eq!(read_f32(rec, 48), 0.0);
    assert!((read_f32(rec, 52) - srgb_to_linear(128)).abs() < 1e-6);
    assert_eq!(read_f32(rec, 56), 0.0);
    assert_eq!(read_f32(rec, 60), 0.75);

    // Kind = Cursor (2).
    assert_eq!(read_u32(rec, 64), 2);
}

// --- push_glyph_with_bg ---

#[test]
fn push_glyph_with_bg_field_offsets() {
    let mut w = InstanceWriter::new();
    let uv = [0.1, 0.2, 0.3, 0.4];
    w.push_glyph_with_bg(
        ScreenRect {
            x: 10.0,
            y: 20.0,
            w: 8.0,
            h: 16.0,
        },
        GlyphInstanceBg {
            uv,
            fg: WHITE,
            bg: RED,
            alpha: 0.9,    // fg alpha
            bg_alpha: 1.0, // opaque bg
            atlas_page: 3,
            clip: CLIP_UNCLIPPED,
        },
    );

    let rec = w.as_bytes();

    // Position.
    assert_eq!(read_f32(rec, 0), 10.0);
    assert_eq!(read_f32(rec, 4), 20.0);
    assert_eq!(read_f32(rec, 8), 8.0);
    assert_eq!(read_f32(rec, 12), 16.0);

    // UV.
    assert!((read_f32(rec, 16) - 0.1).abs() < 1e-6);
    assert!((read_f32(rec, 20) - 0.2).abs() < 1e-6);
    assert!((read_f32(rec, 24) - 0.3).abs() < 1e-6);
    assert!((read_f32(rec, 28) - 0.4).abs() < 1e-6);

    // FG = WHITE STRAIGHT (1.0 per channel); alpha 0.9 in the alpha slot only
    // (the shader premultiplies color by alpha). srgb_to_linear(255) == 1.0
    // exactly, so the colour channels are pinned with exact equality; alpha 0.9
    // is not f32-exact, so it keeps a tolerance.
    assert_eq!(read_f32(rec, 32), 1.0);
    assert_eq!(read_f32(rec, 36), 1.0);
    assert_eq!(read_f32(rec, 40), 1.0);
    assert!((read_f32(rec, 44) - 0.9).abs() < 1e-6);

    // BG = RED at opaque bg_alpha=1.0 (straight == premul at 1.0).
    assert_eq!(read_f32(rec, 48), 1.0); // R
    assert_eq!(read_f32(rec, 52), 0.0); // G
    assert_eq!(read_f32(rec, 56), 0.0); // B
    assert_eq!(read_f32(rec, 60), 1.0); // A

    // Kind = Glyph (1).
    assert_eq!(read_u32(rec, 64), 1);

    // Atlas page = 3.
    assert_eq!(read_u32(rec, 68), 3);
}

/// A translucent bg riding the subpixel‑with‑bg path threads the resolved
/// bg_alpha into the bg alpha slot (the genuine 15.3 deliverable) while
/// writing STRAIGHT bg color (the shader premultiplies). The bg alpha is
/// independent of the fg alpha field.
#[test]
fn push_glyph_with_bg_threads_translucent_bg_alpha() {
    let mut w = InstanceWriter::new();
    let mid = Rgb {
        r: 200,
        g: 100,
        b: 50,
    };
    w.push_glyph_with_bg(
        ScreenRect {
            x: 0.0,
            y: 0.0,
            w: 8.0,
            h: 16.0,
        },
        GlyphInstanceBg {
            uv: [0.0; 4],
            fg: WHITE,
            bg: mid,
            alpha: 1.0,    // fg fully opaque
            bg_alpha: 0.5, // translucent bg
            atlas_page: 0,
            clip: CLIP_UNCLIPPED,
        },
    );

    let rec = w.as_bytes();
    // BG STRAIGHT: srgb_to_linear(c) on each channel, un‑premultiplied.
    assert!(
        (read_f32(rec, 48) - srgb_to_linear(200)).abs() < 1e-6,
        "bg R straight (not premultiplied by bg_alpha)"
    );
    assert!(
        (read_f32(rec, 52) - srgb_to_linear(100)).abs() < 1e-6,
        "bg G straight (not premultiplied by bg_alpha)"
    );
    assert!(
        (read_f32(rec, 56) - srgb_to_linear(50)).abs() < 1e-6,
        "bg B straight (not premultiplied by bg_alpha)"
    );
    assert!((read_f32(rec, 60) - 0.5).abs() < 1e-6, "bg A carries 0.5");
    // Negative: bg alpha must NOT be hardcoded opaque 1.0 (the §15.3 deliverable
    // threads the real bg_alpha; the pre‑§15.3 path hardcoded 1.0).
    assert!(
        (read_f32(rec, 60) - 1.0).abs() > 1e-3,
        "bg alpha must thread the real bg_alpha, NOT a hardcoded 1.0"
    );
    // Negative: bg color must NOT be premultiplied (double‑premul bug).
    assert!(
        (read_f32(rec, 48) - srgb_to_linear(200) * 0.5).abs() > 1e-3,
        "bg R must NOT be premultiplied (the shader owns the single premultiply)"
    );
}

// --- Color conversion ---

#[test]
fn rgb_conversion_boundary_values() {
    let mut w = InstanceWriter::new();
    w.push_rect(
        ScreenRect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        },
        WHITE,
        1.0,
    );

    let rec = w.as_bytes();
    assert_eq!(read_f32(rec, 48), 1.0);
    assert_eq!(read_f32(rec, 52), 1.0);
    assert_eq!(read_f32(rec, 56), 1.0);

    let mut w2 = InstanceWriter::new();
    w2.push_rect(
        ScreenRect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        },
        BLACK,
        0.0,
    );

    let rec2 = w2.as_bytes();
    assert_eq!(read_f32(rec2, 48), 0.0);
    assert_eq!(read_f32(rec2, 52), 0.0);
    assert_eq!(read_f32(rec2, 56), 0.0);
    assert_eq!(read_f32(rec2, 60), 0.0);
}

#[test]
fn rgb_mid_value_conversion() {
    let mid = Rgb {
        r: 128,
        g: 64,
        b: 192,
    };
    let mut w = InstanceWriter::new();
    w.push_rect(
        ScreenRect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        },
        mid,
        1.0,
    );

    let rec = w.as_bytes();
    assert!((read_f32(rec, 48) - srgb_to_linear(128)).abs() < 1e-6);
    assert!((read_f32(rec, 52) - srgb_to_linear(64)).abs() < 1e-6);
    assert!((read_f32(rec, 56) - srgb_to_linear(192)).abs() < 1e-6);
}

/// `rgb_to_floats` must emit STRAIGHT linear color + straight alpha. The
/// fragment shaders own the single premultiply step (`bg.wgsl` does
/// `vec4(c.rgb*c.a, c.a)`), so at a<1.0 the color channels are the
/// un-multiplied `srgb_to_linear(c)`, NOT `srgb_to_linear(c) * a`.
/// Premultiplying here too would double-multiply and composite too dark.
#[test]
fn rgb_to_floats_emits_straight_color_with_straight_alpha() {
    let mid = Rgb {
        r: 128,
        g: 64,
        b: 192,
    };
    let mut w = InstanceWriter::new();
    w.push_rect(
        ScreenRect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        },
        mid,
        0.5,
    );

    let rec = w.as_bytes();
    assert!(
        (read_f32(rec, 48) - srgb_to_linear(128)).abs() < 1e-6,
        "R must be straight: srgb_to_linear(128), NOT premultiplied by alpha"
    );
    assert!(
        (read_f32(rec, 52) - srgb_to_linear(64)).abs() < 1e-6,
        "G must be straight: srgb_to_linear(64), NOT premultiplied by alpha"
    );
    assert!(
        (read_f32(rec, 56) - srgb_to_linear(192)).abs() < 1e-6,
        "B must be straight: srgb_to_linear(192), NOT premultiplied by alpha"
    );
    assert!(
        (read_f32(rec, 60) - 0.5).abs() < 1e-6,
        "A carries straight 0.5"
    );
}

/// Rejection guard: reject the double-premultiply behavior. At a<1.0 the
/// color channels must NOT equal `srgb_to_linear(c) * a` — premultiplying in
/// `rgb_to_floats` on top of the shader's own premultiply double-multiplies
/// and composites the color too dark.
#[test]
fn rgb_to_floats_rejects_premultiplied_color_at_partial_alpha() {
    let mid = Rgb {
        r: 200,
        g: 100,
        b: 50,
    };
    let mut w = InstanceWriter::new();
    w.push_rect(
        ScreenRect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        },
        mid,
        0.5,
    );

    let rec = w.as_bytes();
    assert!(
        (read_f32(rec, 48) - srgb_to_linear(200) * 0.5).abs() > 1e-3,
        "R must NOT be premultiplied srgb_to_linear(200) * 0.5 (double-premul bug)"
    );
}

/// Opaque (a=1.0) is byte-identical to the straight color: at a=1.0 straight
/// and premultiplied coincide, so opaque cells never regress.
#[test]
fn rgb_to_floats_opaque_color_unchanged() {
    let mid = Rgb {
        r: 128,
        g: 64,
        b: 192,
    };
    let mut w = InstanceWriter::new();
    w.push_rect(
        ScreenRect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        },
        mid,
        1.0,
    );
    let rec = w.as_bytes();
    assert_eq!(read_f32(rec, 48), srgb_to_linear(128), "opaque R unchanged");
    assert_eq!(read_f32(rec, 52), srgb_to_linear(64), "opaque G unchanged");
    assert_eq!(read_f32(rec, 56), srgb_to_linear(192), "opaque B unchanged");
    assert_eq!(read_f32(rec, 60), 1.0, "opaque A unchanged");
}

// --- Lifecycle ---

#[test]
fn empty_writer() {
    let w = InstanceWriter::new();
    assert!(w.is_empty());
    assert_eq!(w.len(), 0);
    assert_eq!(w.byte_len(), 0);
    assert!(w.as_bytes().is_empty());
}

#[test]
fn with_capacity_starts_empty() {
    let w = InstanceWriter::with_capacity(100);
    assert!(w.is_empty());
    assert_eq!(w.len(), 0);
}

#[test]
fn multiple_pushes_accumulate() {
    let mut w = InstanceWriter::new();
    w.push_rect(
        ScreenRect {
            x: 0.0,
            y: 0.0,
            w: 8.0,
            h: 16.0,
        },
        BLACK,
        1.0,
    );
    w.push_glyph(
        ScreenRect {
            x: 8.0,
            y: 0.0,
            w: 8.0,
            h: 16.0,
        },
        GlyphInstance {
            uv: [0.0; 4],
            fg: WHITE,
            alpha: 1.0,
            atlas_page: 0,
            clip: CLIP_UNCLIPPED,
        },
    );
    w.push_cursor(
        ScreenRect {
            x: 16.0,
            y: 0.0,
            w: 2.0,
            h: 16.0,
        },
        RED,
        1.0,
    );

    assert_eq!(w.len(), 3);
    assert_eq!(w.byte_len(), 288);

    // Each record starts at the right offset.
    let bytes = w.as_bytes();
    assert_eq!(read_u32(bytes, 64), InstanceKind::Rect as u32);
    assert_eq!(read_u32(bytes, 96 + 64), InstanceKind::Glyph as u32);
    assert_eq!(read_u32(bytes, 192 + 64), InstanceKind::Cursor as u32);
}

#[test]
fn clear_resets_length_but_retains_capacity() {
    let mut w = InstanceWriter::new();
    for _ in 0..50 {
        w.push_rect(
            ScreenRect {
                x: 0.0,
                y: 0.0,
                w: 8.0,
                h: 16.0,
            },
            BLACK,
            1.0,
        );
    }
    assert_eq!(w.len(), 50);

    w.clear();
    assert!(w.is_empty());
    assert_eq!(w.len(), 0);
    assert_eq!(w.byte_len(), 0);

    // Capacity should still be at least 50 * 80.
    // (Vec::capacity is in bytes for Vec<u8>.)
}

#[test]
fn clear_and_reuse() {
    let mut w = InstanceWriter::new();
    w.push_rect(
        ScreenRect {
            x: 0.0,
            y: 0.0,
            w: 8.0,
            h: 16.0,
        },
        RED,
        1.0,
    );
    w.clear();
    w.push_glyph(
        ScreenRect {
            x: 10.0,
            y: 20.0,
            w: 8.0,
            h: 16.0,
        },
        GlyphInstance {
            uv: [0.1, 0.2, 0.3, 0.4],
            fg: WHITE,
            alpha: 1.0,
            atlas_page: 0,
            clip: CLIP_UNCLIPPED,
        },
    );

    assert_eq!(w.len(), 1);
    let rec = w.as_bytes();
    assert_eq!(read_f32(rec, 0), 10.0);
    assert_eq!(read_u32(rec, 64), InstanceKind::Glyph as u32);
}

// --- Raw push ---

#[test]
fn push_raw_valid() {
    let mut w = InstanceWriter::new();
    let mut raw = [0u8; INSTANCE_SIZE];
    raw[64..68].copy_from_slice(&42u32.to_le_bytes());
    w.push_raw(&raw);

    assert_eq!(w.len(), 1);
    assert_eq!(read_u32(w.as_bytes(), 64), 42);
}

#[test]
#[should_panic(expected = "raw instance must be exactly 96 bytes")]
fn push_raw_wrong_size_panics() {
    let mut w = InstanceWriter::new();
    w.push_raw(&[0u8; 40]);
}

// --- Default ---

#[test]
fn default_is_empty() {
    let w = InstanceWriter::default();
    assert!(w.is_empty());
}

// --- InstanceKind values ---

#[test]
fn instance_kind_discriminants() {
    assert_eq!(InstanceKind::Rect as u32, 0);
    assert_eq!(InstanceKind::Glyph as u32, 1);
    assert_eq!(InstanceKind::Cursor as u32, 2);
}

// UI rect tests moved to ui_rect_writer/tests.rs.
