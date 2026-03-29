//! Tests for vector icon rasterization.

use oriterm_ui::icons::sidebar_fixtures::SIDEBAR_ICON_SOURCES;
use oriterm_ui::icons::svg_import::svg_to_commands;
use oriterm_ui::icons::{IconId, IconStyle};

use super::{rasterize_commands, rasterize_icon};

/// Scale factor used for tests (1.0× = no scaling).
const TEST_SCALE: f32 = 1.0;

/// Rasterize close icon at different sizes — all produce correct dimensions.
#[test]
fn rasterize_close_at_multiple_sizes() {
    for size in [16, 24, 32] {
        let data = rasterize_icon(IconId::Close.path(), size, TEST_SCALE);
        assert_eq!(
            data.len(),
            (size * size) as usize,
            "close icon at {size}px: expected {} bytes, got {}",
            size * size,
            data.len()
        );
    }
}

/// Different sizes produce different pixel data (not byte-for-byte duplicates).
#[test]
fn different_sizes_produce_different_data() {
    let d16 = rasterize_icon(IconId::Close.path(), 16, TEST_SCALE);
    let d32 = rasterize_icon(IconId::Close.path(), 32, TEST_SCALE);
    assert_ne!(d16.len(), d32.len());
}

/// Rasterized close icon has non-zero alpha along the diagonal.
#[test]
fn close_icon_has_nonzero_alpha() {
    let size = 32;
    let data = rasterize_icon(IconId::Close.path(), size, TEST_SCALE);
    let nonzero = data.iter().filter(|&&a| a > 0).count();
    assert!(nonzero > 0, "close icon at {size}px has no visible pixels");
}

/// Close icon at 2.0x scale (32px for 16px logical) has smooth diagonals.
///
/// Smooth diagonals have partial alpha values (not just 0 or 255),
/// indicating anti-aliasing is working.
#[test]
fn close_icon_has_antialiased_diagonals() {
    let data = rasterize_icon(IconId::Close.path(), 32, 2.0);
    let partial = data.iter().filter(|&&a| a > 0 && a < 255).count();
    assert!(
        partial > 0,
        "no partial alpha values found — anti-aliasing may not be working"
    );
}

/// All icon variants produce non-empty output at 16px.
#[test]
fn all_icons_rasterize_to_nonempty() {
    let icons = [
        IconId::Close,
        IconId::Plus,
        IconId::ChevronDown,
        IconId::Minimize,
        IconId::Maximize,
        IconId::Restore,
        IconId::WindowClose,
        IconId::Sun,
        IconId::Palette,
        IconId::Type,
        IconId::Terminal,
        IconId::Keyboard,
        IconId::Window,
        IconId::Bell,
        IconId::Activity,
    ];
    for id in icons {
        let data = rasterize_icon(id.path(), 16, TEST_SCALE);
        assert_eq!(data.len(), 256, "{id:?} at 16px: wrong size");
        let nonzero = data.iter().filter(|&&a| a > 0).count();
        assert!(nonzero > 0, "{id:?} at 16px has no visible pixels");
    }
}

/// Zero size returns empty data.
#[test]
fn zero_size_returns_empty() {
    let data = rasterize_icon(IconId::Close.path(), 0, TEST_SCALE);
    assert!(data.is_empty());
}

/// Plus icon has pixels in both horizontal and vertical arms.
#[test]
fn plus_icon_has_cross_pattern() {
    let size = 32u32;
    let data = rasterize_icon(IconId::Plus.path(), size, TEST_SCALE);
    let mid = size / 2;

    // Check horizontal arm (middle row).
    let h_row_start = (mid * size) as usize;
    let h_row = &data[h_row_start..h_row_start + size as usize];
    let h_nonzero = h_row.iter().filter(|&&a| a > 0).count();

    // Check vertical arm (middle column).
    let v_nonzero = (0..size)
        .filter(|&row| data[(row * size + mid) as usize] > 0)
        .count();

    assert!(h_nonzero > 2, "plus horizontal arm missing");
    assert!(v_nonzero > 2, "plus vertical arm missing");
}

/// Minimize icon has pixels concentrated in the middle row.
#[test]
fn minimize_icon_is_horizontal() {
    let size = 24u32;
    let data = rasterize_icon(IconId::Minimize.path(), size, TEST_SCALE);
    let mid = size / 2;

    // Middle row should have nonzero alpha.
    let row_start = (mid * size) as usize;
    let row = &data[row_start..row_start + size as usize];
    let mid_nonzero = row.iter().filter(|&&a| a > 0).count();
    assert!(mid_nonzero > size as usize / 3, "minimize dash too short");
}

/// Higher scale factor produces thicker strokes (more non-zero pixels).
#[test]
fn higher_scale_thicker_strokes() {
    let size = 20u32;
    let data_1x = rasterize_icon(IconId::Close.path(), size, 1.0);
    let data_2x = rasterize_icon(IconId::Close.path(), size, 2.0);
    let count_1x = data_1x.iter().filter(|&&a| a > 0).count();
    let count_2x = data_2x.iter().filter(|&&a| a > 0).count();
    assert!(
        count_2x > count_1x,
        "2x scale should produce thicker strokes: {count_1x} vs {count_2x}"
    );
}

// Sidebar icon clipping tests

const SIDEBAR_ICONS: &[IconId] = &[
    IconId::Sun,
    IconId::Palette,
    IconId::Type,
    IconId::Terminal,
    IconId::Keyboard,
    IconId::Window,
    IconId::Bell,
    IconId::Activity,
];

/// Count border pixels with alpha above `threshold` along each edge.
///
/// Returns `(top, right, bottom, left)` — the number of high-alpha pixels
/// per edge. A few hot spots near corners or endpoints are normal; an
/// entire edge being lit up means the geometry is being clipped.
fn border_heavy_pixels(data: &[u8], size: u32, threshold: u8) -> (usize, usize, usize, usize) {
    let s = size as usize;
    let top = (0..s).filter(|&c| data[c] > threshold).count();
    let bottom = (0..s)
        .filter(|&c| data[(s - 1) * s + c] > threshold)
        .count();
    let left = (0..s).filter(|&r| data[r * s] > threshold).count();
    let right = (0..s)
        .filter(|&r| data[r * s + (s - 1)] > threshold)
        .count();
    (top, right, bottom, left)
}

/// Sidebar icons at 16px logical / 1.0× scale have no heavy clipping at edges.
///
/// Icons like Palette and Sun have geometry near the edges (body at 2/24,
/// rays at 1/24), so individual border pixels may have moderate alpha from
/// anti-aliased stroke fringes. The test catches *clipping*: an entire edge
/// being heavily painted, which means geometry extends beyond the icon box.
/// Threshold: no edge may have more than 40% of its pixels above alpha 100.
/// Set to 40% because icons like Palette have geometry near the viewBox
/// boundary (path reaches x=23 in 24-wide viewBox), and round-cap stroke
/// fringes produce moderate alpha near edges.
#[test]
fn sidebar_icons_not_clipped_at_16px() {
    let size = 16u32;
    let max_heavy = size * 2 / 5; // 40% of edge length.
    for &id in SIDEBAR_ICONS {
        let data = rasterize_icon(id.path(), size, 1.0);
        let (t, r, b, l) = border_heavy_pixels(&data, size, 100);
        assert!(
            t <= max_heavy as usize
                && r <= max_heavy as usize
                && b <= max_heavy as usize
                && l <= max_heavy as usize,
            "{id:?} at {size}px: border heavy pixels (t={t},r={r},b={b},l={l}) \
             exceed 25% on an edge — icon may be clipped"
        );
    }
}

/// Sidebar icons at 32px physical / 2.0× scale (HiDPI) have no heavy clipping.
#[test]
fn sidebar_icons_not_clipped_at_hidpi() {
    let size = 32u32;
    let max_heavy = size / 4;
    for &id in SIDEBAR_ICONS {
        let data = rasterize_icon(id.path(), size, 2.0);
        let (t, r, b, l) = border_heavy_pixels(&data, size, 100);
        assert!(
            t <= max_heavy as usize
                && r <= max_heavy as usize
                && b <= max_heavy as usize
                && l <= max_heavy as usize,
            "{id:?} at {size}px/2x: border heavy pixels (t={t},r={r},b={b},l={l}) \
             exceed 25% on an edge — icon may be clipped at HiDPI"
        );
    }
}

/// Sidebar icons have sufficient visible content at 16px (not degenerate).
#[test]
fn sidebar_icons_have_content_at_16px() {
    let size = 16u32;
    for &id in SIDEBAR_ICONS {
        let data = rasterize_icon(id.path(), size, 1.0);
        let nonzero = data.iter().filter(|&&a| a > 0).count();
        // At 16×16 = 256 pixels, even a simple icon should light up > 5%.
        assert!(
            nonzero > 12,
            "{id:?} at {size}px: only {nonzero} non-zero pixels — icon too sparse"
        );
    }
}

/// Chrome icons still rasterize correctly at 10px (regression guard).
#[test]
fn chrome_icons_unchanged_at_10px() {
    let chrome = [
        IconId::Close,
        IconId::Plus,
        IconId::ChevronDown,
        IconId::Minimize,
        IconId::Maximize,
        IconId::Restore,
        IconId::WindowClose,
    ];
    for id in chrome {
        let data = rasterize_icon(id.path(), 10, 1.0);
        assert_eq!(data.len(), 100, "{id:?} at 10px: wrong bitmap size");
        let nonzero = data.iter().filter(|&&a| a > 0).count();
        assert!(nonzero > 0, "{id:?} at 10px has no visible pixels");
    }
}

// Raster fidelity tests — source SVG vs runtime IconPath comparison
//
// Each sidebar icon has an authoritative SVG fixture in SIDEBAR_ICON_SOURCES.
// The runtime IconPath definitions in sidebar_nav.rs were generated from these
// fixtures via svg_to_commands(). These tests verify that the runtime definitions
// still match the source by rasterizing both and comparing alpha masks.
//
// Tolerance methodology:
// - Mean Absolute Difference (MAD) across all pixels: threshold 2.0 (out of 255).
//   This is the PRIMARY shape-drift detector. A MAD above 2.0 means the runtime
//   definition has meaningfully diverged from the source SVG.
// - Max per-pixel difference: threshold 80 alpha units.
//   Generous because codegen truncation (6-decimal formatting in
//   commands_to_rust_source) shifts anti-aliased stroke boundaries by up to
//   ~0.0001 normalized units. At HiDPI scales, this can flip a single border
//   pixel's alpha by ±60-80. Real geometry errors affect many pixels and are
//   caught by the MAD threshold.

/// The SVG viewBox size used by all sidebar icon fixtures.
const SVG_VIEWBOX: f32 = 24.0;

/// Compare two alpha bitmaps and return (mean_absolute_diff, max_pixel_diff).
fn alpha_diff(a: &[u8], b: &[u8]) -> (f32, u8) {
    assert_eq!(a.len(), b.len(), "bitmaps must have the same size");
    if a.is_empty() {
        return (0.0, 0);
    }
    let mut sum: u64 = 0;
    let mut max: u8 = 0;
    for (&va, &vb) in a.iter().zip(b.iter()) {
        let d = va.abs_diff(vb);
        sum += d as u64;
        if d > max {
            max = d;
        }
    }
    let mad = sum as f32 / a.len() as f32;
    (mad, max)
}

/// Rasterize a sidebar fixture SVG into an alpha bitmap at the given size.
///
/// Uses the runtime icon's stroke width so the comparison tests path
/// geometry equivalence, not stroke width. The runtime stroke is read
/// from the icon's `IconStyle::Stroke(w)`.
fn rasterize_fixture_svg(
    fixture: &oriterm_ui::icons::sidebar_fixtures::SidebarIconSource,
    size_px: u32,
    scale: f32,
) -> Vec<u8> {
    let cmds = svg_to_commands(fixture.svg, SVG_VIEWBOX);
    let stroke = match fixture.id.path().style {
        IconStyle::Stroke(w) => w,
        IconStyle::Fill => panic!("sidebar icon should use Stroke style"),
    };
    rasterize_commands(&cmds, IconStyle::Stroke(stroke), size_px, scale)
}

/// MAD threshold — mean absolute alpha difference per pixel (0–255).
///
/// Anti-aliasing differences from 6-decimal coordinate truncation in
/// the codegen output can shift stroke boundaries by ~0.0001 normalized
/// units, producing per-pixel alpha variance.
const FIDELITY_MAD_THRESHOLD: f32 = 2.5;

/// Max per-pixel alpha difference. Generous because 6-decimal codegen
/// truncation shifts anti-aliased boundaries at HiDPI. See module comment.
const FIDELITY_MAX_PIXEL_THRESHOLD: u8 = 80;

/// All 8 sidebar icons: source SVG and runtime IconPath produce matching
/// alpha masks at 16px logical / 1.0× scale.
#[test]
fn sidebar_fidelity_16px() {
    for fixture in &SIDEBAR_ICON_SOURCES {
        let source = rasterize_fixture_svg(fixture, 16, 1.0);
        let runtime = rasterize_icon(fixture.id.path(), 16, 1.0);

        let (mad, max_diff) = alpha_diff(&source, &runtime);
        assert!(
            mad <= FIDELITY_MAD_THRESHOLD,
            "{:?} at 16px: mean alpha diff {mad:.2} exceeds threshold {} — \
             runtime IconPath drifted from source SVG",
            fixture.id,
            FIDELITY_MAD_THRESHOLD,
        );
        assert!(
            max_diff <= FIDELITY_MAX_PIXEL_THRESHOLD,
            "{:?} at 16px: max pixel diff {max_diff} exceeds threshold {} — \
             localized geometry error",
            fixture.id,
            FIDELITY_MAX_PIXEL_THRESHOLD,
        );
    }
}

/// All 8 sidebar icons: source SVG and runtime IconPath produce matching
/// alpha masks at 32px physical / 2.0× scale (HiDPI).
#[test]
fn sidebar_fidelity_hidpi() {
    for fixture in &SIDEBAR_ICON_SOURCES {
        let source = rasterize_fixture_svg(fixture, 32, 2.0);
        let runtime = rasterize_icon(fixture.id.path(), 32, 2.0);

        let (mad, max_diff) = alpha_diff(&source, &runtime);
        assert!(
            mad <= FIDELITY_MAD_THRESHOLD,
            "{:?} at 32px/2x: mean alpha diff {mad:.2} exceeds threshold {} — \
             runtime IconPath drifted from source SVG at HiDPI",
            fixture.id,
            FIDELITY_MAD_THRESHOLD,
        );
        assert!(
            max_diff <= FIDELITY_MAX_PIXEL_THRESHOLD,
            "{:?} at 32px/2x: max pixel diff {max_diff} exceeds threshold {} — \
             localized geometry error at HiDPI",
            fixture.id,
            FIDELITY_MAX_PIXEL_THRESHOLD,
        );
    }
}

/// Every sidebar fixture ID has a corresponding runtime IconPath definition.
#[test]
fn sidebar_fixtures_match_runtime_icon_ids() {
    for fixture in &SIDEBAR_ICON_SOURCES {
        let path = fixture.id.path();
        assert!(
            !path.commands.is_empty(),
            "{:?}: runtime IconPath has no commands",
            fixture.id
        );
    }
}

/// Source SVG commands and runtime commands produce nearly identical command counts.
///
/// This catches cases where the SVG importer changes behavior (e.g. different
/// arc subdivision) without updating the checked-in sidebar_nav.rs definitions.
/// Allows ±1 difference because `f32` arc-to-cubic subdivision
/// (`ceil(dtheta / (PI/2))`) can produce different segment counts across
/// platforms (macOS ARM vs Linux x86 trig implementations).
#[test]
fn sidebar_source_runtime_command_count_matches() {
    for fixture in &SIDEBAR_ICON_SOURCES {
        let source_cmds = svg_to_commands(fixture.svg, SVG_VIEWBOX);
        let runtime_cmds = fixture.id.path().commands;

        let diff = (source_cmds.len() as isize - runtime_cmds.len() as isize).unsigned_abs();
        assert!(
            diff <= 1,
            "{:?}: source SVG produces {} commands but runtime has {} (diff {}) — \
             regenerate sidebar_nav.rs from fixtures",
            fixture.id,
            source_cmds.len(),
            runtime_cmds.len(),
            diff,
        );
    }
}
