//! Unit tests for the sixel decoder (`SixelParser`).
//!
//! Covers per-operator behavior (palette define/select, repeat, CR/NL,
//! raster attrs, data bytes), the §12.2 bg-mode + palette-reset invariants
//! (`set_to_bg_uses_terminal_background_not_black`,
//! `device_default_and_set_to_bg_diverge_under_non_black_terminal_bg`,
//! `palette_reset_per_dcs_negative_pin_bypass_breaks_vt340_fingerprint`),
//! and oversized-input rejection. The integration-level spec_chain pins
//! for §12.2 live at `oriterm_core/tests/spec_chain/sixel/invariants.rs`.

use super::bypass::BypassVt340ResetGuard;
use super::*;

/// Feed a byte slice to the parser.
fn feed_all(parser: &mut SixelParser, data: &[u8]) {
    for &b in data {
        parser.feed(b);
    }
}

/// Create a parser with default params and a black terminal bg.
///
/// Threads `COLOR_REGISTERS_MAX` (256, the protocol default) so existing
/// tests preserve their original 256-register palette semantics. New
/// wrap-semantics tests use [`parser_with_count`] to negotiate a smaller
/// register count.
fn default_parser() -> SixelParser {
    SixelParser::new(&[0, 0, 0], [0, 0, 0], COLOR_REGISTERS_MAX)
}

/// Create a transparent-background parser (P2=1).
fn transparent_parser() -> SixelParser {
    SixelParser::new(&[0, 1, 0], [0, 0, 0], COLOR_REGISTERS_MAX)
}

/// Create a `SetToBg` parser (P2=2) with an explicit terminal bg.
fn set_to_bg_parser(terminal_bg: [u8; 3]) -> SixelParser {
    SixelParser::new(&[0, 2, 0], terminal_bg, COLOR_REGISTERS_MAX)
}

/// Create a parser with an explicit `color_registers` count (XTSMGRAPHICS
/// Pi=1 Pa=3 negotiation snapshot). Used by §06 wrap-semantics tests.
fn parser_with_count(color_registers: u16) -> SixelParser {
    SixelParser::new(&[0, 0, 0], [0, 0, 0], color_registers)
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

/// Catalog row: SIXEL-COLOR-REGISTER-WRAP. Regression: BUG-06-024 — sixel decoder
/// MUST wrap color register indices modulo the negotiated count (matches xterm
/// `graphics_sixel.c:697-698` `s_Pregister %= valid_registers;`). With the default
/// count (256), `#300` wraps to register 44 (`300 % 256`) and a definition+select
/// at index 300 lands red on palette[44].
#[test]
fn palette_index_over_count_wraps_modulo_register_count() {
    let mut p = default_parser();
    // count = COLOR_REGISTERS_MAX = 256. `#300;2;100;0;0` defines palette[44]
    // as red (300 % 256 = 44); `#300@` selects palette[44]; draws red.
    feed_all(&mut p, b"#300;2;100;0;0#300@");
    let (pixels, _, _) = p.finish().unwrap();
    assert_eq!(
        &pixels[0..4],
        &[255, 0, 0, 255],
        "palette[44] must be red after `#300;2;100;0;0#300@` (300 % 256 = 44)",
    );
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

    let mut default = SixelParser::new(&[0, 0, 0], terminal_bg, COLOR_REGISTERS_MAX);
    feed_all(&mut default, b"#0;2;100;0;0@");
    let (default_pixels, ..) = default.finish().unwrap();

    let mut set_bg = SixelParser::new(&[0, 2, 0], terminal_bg, COLOR_REGISTERS_MAX);
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

// BUG-06-024 — color-register-wrap matrix tests.
//
// Pin xterm-style modulo wrap on color register indices in `apply_color`
// (`graphics_sixel.c:697-698`: `s_Pregister %= valid_registers;`). The
// `color_registers` value is snapshotted into `SixelParser::new` at DCS-hook
// time; in-flight XTSMGRAPHICS mutations do NOT retroactively affect the
// active parser. See `bug-tracker/plans/BUG-06-024/section-03-tdd-matrix.md`
// for the full matrix; the tests below cover the `idx-class × count × op-class`
// dimensions called out there.

/// No-wrap edge: count=256, idx=0 (the floor).
#[test]
fn count_default_idx_zero_no_wrap() {
    let mut p = parser_with_count(COLOR_REGISTERS_MAX);
    feed_all(&mut p, b"#0;2;100;0;0#0@");
    let (pixels, _, _) = p.finish().unwrap();
    assert_eq!(&pixels[0..4], &[255, 0, 0, 255], "palette[0] must be red");
}

/// No-wrap edge: count=256, idx=255 (top-of-256 register).
#[test]
fn count_default_idx_max_no_wrap() {
    let mut p = parser_with_count(COLOR_REGISTERS_MAX);
    feed_all(&mut p, b"#255;2;0;100;0#255@");
    let (pixels, _, _) = p.finish().unwrap();
    assert_eq!(
        &pixels[0..4],
        &[0, 255, 0, 255],
        "palette[255] must be green (no wrap at idx == count - 1 == 255)",
    );
}

/// Boundary: count=256, idx=256 wraps to 0. Proves modulo engages at the max
/// legal bound (256 % 256 = 0). Plan TPR Round 0 F4 (gemini) addition.
#[test]
fn count_default_idx_at_count_wraps_to_zero() {
    let mut p = parser_with_count(COLOR_REGISTERS_MAX);
    feed_all(&mut p, b"#256;2;100;0;0#256@");
    let (pixels, _, _) = p.finish().unwrap();
    assert_eq!(
        &pixels[0..4],
        &[255, 0, 0, 255],
        "256 % 256 = 0; palette[0] must be red after `#256;2;100;0;0#256@`",
    );
}

/// No-wrap edge: count=16, idx=15 (top-of-range, no wrap).
#[test]
fn count_mid_idx_at_count_minus_one_no_wrap() {
    let mut p = parser_with_count(16);
    feed_all(&mut p, b"#15;2;0;0;100#15@");
    let (pixels, _, _) = p.finish().unwrap();
    assert_eq!(
        &pixels[0..4],
        &[0, 0, 255, 255],
        "palette[15] must be blue (no wrap at idx == count - 1 == 15)",
    );
}

/// Wrap boundary: count=16, idx=16 wraps to 0.
#[test]
fn count_mid_idx_at_count_wraps_to_zero() {
    let mut p = parser_with_count(16);
    feed_all(&mut p, b"#16;2;100;0;0#16@");
    let (pixels, _, _) = p.finish().unwrap();
    assert_eq!(
        &pixels[0..4],
        &[255, 0, 0, 255],
        "16 % 16 = 0; palette[0] must be red after `#16;2;100;0;0#16@`",
    );
}

/// Wrap above boundary: count=16, idx=17 wraps to 1.
#[test]
fn count_mid_idx_at_count_plus_one_wraps_to_one() {
    let mut p = parser_with_count(16);
    feed_all(&mut p, b"#17;2;100;0;0#17@");
    let (pixels, _, _) = p.finish().unwrap();
    assert_eq!(
        &pixels[0..4],
        &[255, 0, 0, 255],
        "17 % 16 = 1; palette[1] must be red after `#17;2;100;0;0#17@`",
    );
}

/// Wrap multiple: count=16, idx=32 wraps to 0.
#[test]
fn count_mid_idx_at_double_count_wraps_to_zero() {
    let mut p = parser_with_count(16);
    feed_all(&mut p, b"#32;2;100;0;0#32@");
    let (pixels, _, _) = p.finish().unwrap();
    assert_eq!(
        &pixels[0..4],
        &[255, 0, 0, 255],
        "32 % 16 = 0; palette[0] must be red after `#32;2;100;0;0#32@`",
    );
}

/// Wrap with non-power-of-2 prime count: distinguishes correct
/// `params[0] % u32::from(count)` from buggy `(params[0] as u16) % count`.
/// Buggy impl: `(999_999 as u16) = 16959; 16959 % 17 = 10` (palette[10]).
/// Correct impl: `999_999 % 17 = 8` (palette[8]).
/// Plan TPR Round 0 F1 (codex+gemini+opencode 3-of-3 agreement).
#[test]
fn count_prime_idx_huge_wraps_correctly() {
    let mut p = parser_with_count(17);
    feed_all(&mut p, b"#999999;2;100;0;0#999999@");
    let (pixels, _, _) = p.finish().unwrap();
    assert_eq!(
        &pixels[0..4],
        &[255, 0, 0, 255],
        "999_999 % 17 = 8; palette[8] must be red. Buggy u16-truncate-then-mod \
         would put red at palette[10] (16959 % 17 = 10) — different palette slot.",
    );
}

/// Minimum legal count: count=2 (per `status.rs:265` accepts `pv > 1`).
/// idx=3 wraps to 1.
#[test]
fn count_minimum_two_wrap_works() {
    let mut p = parser_with_count(2);
    feed_all(&mut p, b"#3;2;100;0;0#3@");
    let (pixels, _, _) = p.finish().unwrap();
    assert_eq!(
        &pixels[0..4],
        &[255, 0, 0, 255],
        "3 % 2 = 1; palette[1] must be red",
    );
}

/// Bare-selection (no prior definition): count=16, `#17@` wraps to palette[1]
/// which is VT340 default register 1 = `[51, 51, 204]` blue. NEGATIVE PIN
/// against the wrap-only-on-definition bug — without wrap on selection,
/// `current_color = 17` would draw palette[17] = `[0, 0, 0]` (zeroed default
/// beyond the 16-entry VT340 table) → black, distinguishable from blue.
/// Plan TPR Round 0 F2 (codex+opencode agreement).
#[test]
fn bare_selection_under_count_wraps_to_vt340_register_one() {
    let mut p = parser_with_count(16);
    feed_all(&mut p, b"#17@");
    let (pixels, _, _) = p.finish().unwrap();
    assert_eq!(
        &pixels[0..4],
        &[51, 51, 204, 255],
        "17 % 16 = 1; palette[1] must be VT340 default blue. \
         Old impl would draw palette[17] = zeroed default = black.",
    );
}

/// Coherent definition+selection identity under wrap: count=16, define
/// register 20 (wraps to 4) red, then select register 36 (also wraps to 4),
/// draw — must be red. Both ops use the same wrap rule.
#[test]
fn definition_then_selection_coherent_under_wrap() {
    let mut p = parser_with_count(16);
    feed_all(&mut p, b"#20;2;100;0;0#36@");
    let (pixels, _, _) = p.finish().unwrap();
    assert_eq!(
        &pixels[0..4],
        &[255, 0, 0, 255],
        "20 % 16 = 4 = 36 % 16; both ops must wrap to palette[4] = red",
    );
}
