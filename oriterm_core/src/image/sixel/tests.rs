use super::*;

/// Feed a byte slice to the parser.
fn feed_all(parser: &mut SixelParser, data: &[u8]) {
    for &b in data {
        parser.feed(b);
    }
}

/// Create a parser with default params and a black terminal bg.
fn default_parser() -> SixelParser {
    SixelParser::new(&[0, 0, 0], [0, 0, 0])
}

/// Create a transparent-background parser (P2=1).
fn transparent_parser() -> SixelParser {
    SixelParser::new(&[0, 1, 0], [0, 0, 0])
}

/// Create a `SetToBg` parser (P2=2) with an explicit terminal bg.
fn set_to_bg_parser(terminal_bg: [u8; 3]) -> SixelParser {
    SixelParser::new(&[0, 2, 0], terminal_bg)
}

#[test]
fn simple_single_column_sixel() {
    // A single sixel character `?` (0x3F) = value 0 = no pixels set.
    // `@` (0x40) = value 1 = bottom pixel of the 6 set.
    let mut p = default_parser();
    feed_all(&mut p, b"#0;2;100;0;0@");
    let (pixels, w, h) = p.finish().unwrap();
    assert_eq!(w, 1);
    assert_eq!(h, 6);
    // Pixel at (0,0) should be red (bit 0 set in value 1).
    assert_eq!(&pixels[0..4], &[255, 0, 0, 255]);
    // Pixel at (0,1) should be background (bit 1 not set).
    assert_eq!(pixels[7], 255); // Alpha is 255 for bg (non-transparent mode).
}

#[test]
fn repeat_operator_produces_correct_count() {
    let mut p = default_parser();
    // Define color 0 as green, then repeat `~` (value 63 = all 6 pixels) 5 times.
    feed_all(&mut p, b"#0;2;0;100;0!5~");
    let (pixels, w, h) = p.finish().unwrap();
    assert_eq!(w, 5);
    assert_eq!(h, 6);
    // All 5 columns should have all 6 rows set to green.
    for col in 0..5 {
        for row in 0..6 {
            let off = (row * 5 + col) * 4;
            assert_eq!(pixels[off], 0, "col={col} row={row} r");
            assert_eq!(pixels[off + 1], 255, "col={col} row={row} g");
            assert_eq!(pixels[off + 2], 0, "col={col} row={row} b");
            assert_eq!(pixels[off + 3], 255, "col={col} row={row} a");
        }
    }
}

#[test]
fn repeat_clamped_at_max_width() {
    let mut p = default_parser();
    // Repeat 20000 times — should be clamped to MAX_DIMENSION (10000).
    feed_all(&mut p, b"!20000~");
    let (_, w, _) = p.finish().unwrap();
    assert!(w <= 10000);
}

#[test]
fn color_palette_rgb_definition() {
    let mut p = default_parser();
    // Define color 5 as RGB(50, 75, 100) → scaled to 0-255.
    feed_all(&mut p, b"#5;2;50;75;100");
    // Select color 5 and draw one pixel.
    feed_all(&mut p, b"#5@");
    let (pixels, _, _) = p.finish().unwrap();
    // Expected: 50*255/100=127, 75*255/100=191, 100*255/100=255.
    assert_eq!(pixels[0], 127);
    assert_eq!(pixels[1], 191);
    assert_eq!(pixels[2], 255);
    assert_eq!(pixels[3], 255);
}

#[test]
fn color_palette_hls_definition() {
    let mut p = default_parser();
    // Define color 1 via HLS: H=120 (red in sixel), L=50, S=100.
    // Sixel hue 120 → standard hue 0 (red).
    feed_all(&mut p, b"#1;1;120;50;100");
    feed_all(&mut p, b"#1@");
    let (pixels, _, _) = p.finish().unwrap();
    // Should be approximately pure red.
    assert!(pixels[0] > 200, "r={}", pixels[0]);
    assert!(pixels[1] < 10, "g={}", pixels[1]);
    assert!(pixels[2] < 10, "b={}", pixels[2]);
}

#[test]
fn multi_row_sixel_newline() {
    let mut p = default_parser();
    // Two sixel rows: first row (`~` = all 6 pixels), newline, second row.
    feed_all(&mut p, b"#0;2;100;0;0~-~");
    let (_, w, h) = p.finish().unwrap();
    assert_eq!(w, 1);
    assert_eq!(h, 12); // Two sixel bands × 6 pixels each.
}

#[test]
fn cursor_position_mode_80_default_scrolling() {
    // Mode 80 (SIXEL_SCROLLING) is on by default — cursor moves below image.
    // We test the parser itself here; cursor movement is tested in handler tests.
    let mut p = default_parser();
    feed_all(&mut p, b"~");
    let (_, _, h) = p.finish().unwrap();
    assert_eq!(h, 6);
}

#[test]
fn transparent_bg_mode() {
    let mut p = transparent_parser();
    // Draw one pixel only (value 1 = bit 0).
    feed_all(&mut p, b"#0;2;100;0;0@");
    let (pixels, w, h) = p.finish().unwrap();
    assert_eq!(w, 1);
    assert_eq!(h, 6);
    // Pixel (0,0) drawn: opaque red.
    assert_eq!(pixels[3], 255);
    // Pixel (0,1) NOT drawn: transparent.
    assert_eq!(pixels[7], 0);
}

#[test]
fn oversized_sixel_rejected() {
    let mut p = default_parser();
    // Raster attributes declare 100001 × 100001 pixels — exceeds limit.
    feed_all(&mut p, b"\"1;1;100001;100001");
    feed_all(&mut p, b"~");
    let result = p.finish();
    assert!(result.is_err());
}

#[test]
fn palette_index_over_256_ignored() {
    let mut p = default_parser();
    // Define color 300 — should be silently ignored (palette is 256).
    feed_all(&mut p, b"#300;2;100;0;0");
    // Select color 300 and draw — should use fallback white.
    feed_all(&mut p, b"#300@");
    let (pixels, _, _) = p.finish().unwrap();
    // Should still produce a valid image (no crash).
    assert_eq!(pixels[3], 255);
}

#[test]
fn carriage_return_resets_x() {
    let mut p = default_parser();
    // Draw two pixels, carriage return, draw again — should overwrite column 0.
    feed_all(&mut p, b"#0;2;100;0;0~~");
    feed_all(&mut p, b"$");
    feed_all(&mut p, b"#1;2;0;100;0~");
    let (pixels, w, _) = p.finish().unwrap();
    assert_eq!(w, 2);
    // Column 0 should now be green (overwritten by second pass).
    assert_eq!(pixels[0], 0, "r");
    assert_eq!(pixels[1], 255, "g");
    assert_eq!(pixels[2], 0, "b");
    // Column 1 should still be red (from first pass).
    assert_eq!(pixels[4], 255, "r");
    assert_eq!(pixels[5], 0, "g");
    assert_eq!(pixels[6], 0, "b");
}

#[test]
fn wikipedia_hi_example() {
    // The classic "HI" sixel example from Wikipedia.
    let mut p = default_parser();
    let data = b"\
        #0;2;0;0;0\
        #1;2;100;100;0\
        #2;2;0;100;0\
        #1~~@@vv@@~~@@~~\
        $\
        #2??}}GG}}??}}??\
        -\
        #1!14@";
    feed_all(&mut p, data);
    let (pixels, w, h) = p.finish().unwrap();
    assert_eq!(w, 14);
    assert_eq!(h, 12); // 2 sixel rows × 6 pixels each.
    // Verify some pixels are non-transparent.
    let has_yellow = pixels
        .chunks(4)
        .any(|p| p[0] == 255 && p[1] == 255 && p[2] == 0);
    let has_green = pixels
        .chunks(4)
        .any(|p| p[0] == 0 && p[1] == 255 && p[2] == 0);
    assert!(has_yellow, "should contain yellow pixels");
    assert!(has_green, "should contain green pixels");
}

#[test]
fn empty_sixel_returns_error() {
    let p = default_parser();
    let result = p.finish();
    assert!(result.is_err());
}

#[test]
fn raster_attributes_set_dimensions() {
    let mut p = default_parser();
    // Declare 20×12 via raster attributes, but only draw 1 column.
    feed_all(&mut p, b"\"1;1;20;12~");
    let (_, w, h) = p.finish().unwrap();
    // Dimensions should be at least the declared size.
    assert_eq!(w, 20);
    assert_eq!(h, 12);
}

// §12.2 — SetToBg plumbing + palette-leak negative pin.

/// §12.2: `SixelBgMode::SetToBg` fills undrawn pixels with the terminal
/// bg captured at DCS-hook time — NOT opaque black. DeviceDefault
/// continues to render as opaque black per VT340 spec.
#[test]
fn set_to_bg_uses_terminal_background_not_black() {
    let terminal_bg: [u8; 3] = [0, 128, 255]; // distinguishable blue-ish
    let mut p = set_to_bg_parser(terminal_bg);
    feed_all(&mut p, b"#0;2;100;0;0@"); // red @ (0,0); rows 1..5 undrawn
    let (pixels, w, h) = p.finish().unwrap();
    assert_eq!(w, 1);
    assert_eq!(h, 6);
    assert_eq!(&pixels[0..4], &[255, 0, 0, 255], "drawn pixel must be red");
    // Undrawn row 1: α=255, RGB = terminal bg (NOT [0,0,0]).
    assert_eq!(
        &pixels[4..8],
        &[terminal_bg[0], terminal_bg[1], terminal_bg[2], 255],
        "SetToBg undrawn pixel must carry terminal bg, not black",
    );
}

/// §12.2 semantic pin: `DeviceDefault` and `SetToBg` must diverge on
/// identical pixel data when the terminal bg is not black. If both
/// render identically, the bg-mode invariant is broken.
#[test]
fn device_default_and_set_to_bg_diverge_under_non_black_terminal_bg() {
    let terminal_bg: [u8; 3] = [12, 34, 56];

    let mut default = SixelParser::new(&[0, 0, 0], terminal_bg);
    feed_all(&mut default, b"#0;2;100;0;0@");
    let (default_pixels, ..) = default.finish().unwrap();

    let mut set_bg = SixelParser::new(&[0, 2, 0], terminal_bg);
    feed_all(&mut set_bg, b"#0;2;100;0;0@");
    let (set_bg_pixels, ..) = set_bg.finish().unwrap();

    // Drawn pixel identical; undrawn differs.
    assert_eq!(
        &default_pixels[0..4],
        &set_bg_pixels[0..4],
        "drawn pixel must be identical across bg modes",
    );
    assert_ne!(
        &default_pixels[4..8],
        &set_bg_pixels[4..8],
        "DeviceDefault and SetToBg must diverge on undrawn pixels",
    );
    assert_eq!(&default_pixels[4..8], &[0, 0, 0, 255]);
    assert_eq!(
        &set_bg_pixels[4..8],
        &[terminal_bg[0], terminal_bg[1], terminal_bg[2], 255]
    );
}

/// §12.2 negative pin — palette-leak guard is live.
///
/// Under normal operation, `SixelParser::new` rebuilds VT340 defaults so
/// a selector `#5` (no definition) maps to cyan `[51, 204, 204]`. Under
/// the test-only `BypassVt340ResetGuard`, the rebuild is skipped and
/// `palette[5]` stays zeroed — the selector maps to black.
///
/// If the production rebuild loop were ever removed, this test would
/// fail — proving the regression guard is load-bearing rather than a
/// coincidence of zero-init. The guard's `Drop` impl restores the
/// thread-local flag even if an assertion panics between bypass-enable
/// and end-of-test, per `.claude/rules/impl-hygiene.md §Temporal
/// Coupling & RAII Guards`.
#[test]
fn palette_reset_per_dcs_negative_pin_bypass_breaks_vt340_fingerprint() {
    // Baseline: no bypass — `#5@` yields VT340 cyan.
    let mut baseline = default_parser();
    feed_all(&mut baseline, b"#5@");
    let (baseline_pixels, ..) = baseline.finish().unwrap();
    assert_eq!(
        &baseline_pixels[0..4],
        &[51, 204, 204, 255],
        "VT340 default palette[5] must be cyan",
    );

    // Bypass: palette is all-zero — `#5@` yields opaque black.
    // The RAII guard restores the flag even if an assert below panics.
    let bypassed_pixels = {
        let _guard = BypassVt340ResetGuard::enable();
        let mut bypassed = default_parser();
        feed_all(&mut bypassed, b"#5@");
        bypassed.finish().unwrap().0
    };

    assert_eq!(
        &bypassed_pixels[0..4],
        &[0, 0, 0, 255],
        "bypassed palette rebuild must expose all-zero palette[5]",
    );
    assert_ne!(
        &baseline_pixels[0..4],
        &bypassed_pixels[0..4],
        "baseline and bypassed pixels must diverge — the VT340 rebuild is load-bearing",
    );
}
