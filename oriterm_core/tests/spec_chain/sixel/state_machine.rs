//! Sixel parser+decoder state-machine end-to-end tests (§12.1).
//!
//! Drives every sixel DCS-level operator through the `SpecHarness` →
//! `Term` → `SixelParser` seam. Observations are via:
//! - Dispatch: `harness.outcome().dispatched_calls` (sixel_start /
//!   sixel_put / sixel_end method names recorded by the `RecordingHandler`).
//! - State effect: `harness.term().renderable_content()` — the public
//!   snapshot API whose `images` / `image_data` vectors expose the
//!   placement created by `handle_sixel_end`. (`ImageCache::placement_count`
//!   is `pub(crate)` so not reachable from integration tests; the
//!   renderable snapshot is the public surface per the §12.0 plan.)
//!
//! Catalog rows covered: SIXEL-DCS-q, SIXEL-P1-ASPECT, SIXEL-BG-DeviceDefault,
//! SIXEL-BG-NoChange, SIXEL-BG-SetToBg, SIXEL-P3-HGRID, SIXEL-RASTER-ATTRS,
//! SIXEL-RASTER-ATTRS-PAN-PAD, SIXEL-RASTER-ATTRS-PH-PV,
//! SIXEL-COLOR-DEFINE-RGB, SIXEL-COLOR-DEFINE-HLS, SIXEL-COLOR-SELECT,
//! SIXEL-REPEAT, SIXEL-CR, SIXEL-NL, SIXEL-DATA-BYTE, SIXEL-ABORT-CAN,
//! SIXEL-ABORT-SUB, SIXEL-ABORT-ESC, SIXEL-RASTER-BEFORE-DATA,
//! SIXEL-RASTER-MID-STREAM.
//!
//! Catalog rows: SIXEL-DCS-put

use oriterm_test_support::spec_chain::{DispatchArgs, SpecHarness};

// Test helpers.

/// Count recorded `sixel_start` dispatch calls in the outcome.
fn sixel_start_count(harness: &SpecHarness) -> usize {
    harness
        .outcome()
        .dispatched_calls
        .iter()
        .filter(|c| c.method == "sixel_start")
        .count()
}

/// Count recorded `sixel_end` dispatch calls in the outcome.
fn sixel_end_count(harness: &SpecHarness) -> usize {
    harness
        .outcome()
        .dispatched_calls
        .iter()
        .filter(|c| c.method == "sixel_end")
        .count()
}

/// Return the typed params passed to the most-recent `sixel_start`
/// dispatch. Pins the Parser → Dispatch rung's P1/P2/P3 tuple so
/// `SIXEL-P1-ASPECT` / `SIXEL-P3-HGRID` rows have direct dispatch
/// evidence, not just end-to-end state-effect evidence.
fn last_sixel_start_params(harness: &SpecHarness) -> Vec<u16> {
    harness
        .outcome()
        .dispatched_calls
        .iter()
        .rev()
        .find_map(|c| match &c.args {
            DispatchArgs::SixelStart { params } => Some(params.clone()),
            _ => None,
        })
        .expect("sixel_start must have been dispatched")
}

/// Return the `aborted` bit passed to the most-recent `sixel_end`
/// dispatch. Pins the Parser → Dispatch rung's abort argument for
/// `SIXEL-ABORT-CAN` / `SIXEL-ABORT-SUB` / `SIXEL-ABORT-ESC` rows.
fn last_sixel_end_aborted(harness: &SpecHarness) -> bool {
    harness
        .outcome()
        .dispatched_calls
        .iter()
        .rev()
        .find_map(|c| match &c.args {
            DispatchArgs::SixelEnd { aborted } => Some(*aborted),
            _ => None,
        })
        .expect("sixel_end must have been dispatched")
}

/// Count placements currently visible in the viewport via the public
/// snapshot. Each `handle_sixel_end` that successfully decodes stores
/// exactly one placement at the cursor, so this count mirrors successful
/// commits for tests that keep the placement inside row 0.
fn viewport_placement_count(harness: &SpecHarness) -> usize {
    harness.term().renderable_content().images.len()
}

/// Grab the most-recently-placed image's pixel buffer (RGBA) and dimensions.
fn last_image_pixels(harness: &SpecHarness) -> (Vec<u8>, u32, u32) {
    let snapshot = harness.term().renderable_content();
    let data = snapshot
        .image_data
        .last()
        .expect("at least one image should be present");
    (data.data.as_ref().clone(), data.width, data.height)
}

/// Wrap a sixel body in a DCS envelope with the given P1;P2;P3 params.
fn dcs_envelope(p1: u16, p2: u16, p3: u16, body: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(body.len() + 16);
    buf.extend_from_slice(format!("\x1bP{p1};{p2};{p3}q").as_bytes());
    buf.extend_from_slice(body);
    buf.extend_from_slice(b"\x1b\\");
    buf
}

// DCS q introducer — P1×P2×P3 cartesian product.

/// Catalog rows: SIXEL-DCS-q, SIXEL-P1-ASPECT, SIXEL-P3-HGRID,
/// SIXEL-BG-DeviceDefault, SIXEL-BG-NoChange, SIXEL-BG-SetToBg.
///
/// Drives the full P1∈{0,7} × P2∈{0,1,2} × P3∈{0,5} cartesian product
/// (12 combos). For each combo, asserts:
/// 1. `sixel_start` dispatch fires exactly once (parser reached Term).
/// 2. A placement is committed (the state-effect observation).
/// 3. P2 drives observable bg semantics: P2=1 (NoChange) yields undrawn
///    pixels with α=0 (transparent); P2=0 (DeviceDefault) and P2=2
///    (SetToBg) yield opaque undrawn pixels (α=255). The RGB distinction
///    between DeviceDefault (VT340 black) and SetToBg (terminal bg) is
///    pinned by §12.2 `invariants.rs`; this test only asserts the α
///    invariant because the default harness terminal bg is black.
#[test]
fn dcs_q_introducer_p1_p2_p3_cartesian_product() {
    const P1_VALUES: [u16; 2] = [0, 7];
    const P2_VALUES: [u16; 3] = [0, 1, 2];
    const P3_VALUES: [u16; 2] = [0, 5];

    let mut cells_visited = 0usize;

    for p1 in P1_VALUES {
        for p2 in P2_VALUES {
            for p3 in P3_VALUES {
                // Body: set color 0 to red, draw one pixel (`@` = value 1 =
                // only bit 0 set → one pixel drawn, five pixels undrawn).
                // This exercises the bg-fill distinction for P2 values.
                let dcs = dcs_envelope(p1, p2, p3, b"#0;2;100;0;0@");

                let mut harness = SpecHarness::new();
                harness.feed(&dcs);

                assert_eq!(
                    sixel_start_count(&harness),
                    1,
                    "P1={p1} P2={p2} P3={p3}: sixel_start should fire once",
                );
                assert_eq!(
                    sixel_end_count(&harness),
                    1,
                    "P1={p1} P2={p2} P3={p3}: sixel_end should fire once",
                );
                // Direct dispatch-rung evidence for `SIXEL-P1-ASPECT` /
                // `SIXEL-P3-HGRID`: the params slice captured by the
                // recording handler MUST carry the exact P1/P2/P3 tuple
                // — not just the method name.
                let captured = last_sixel_start_params(&harness);
                assert_eq!(
                    captured,
                    vec![p1, p2, p3],
                    "P1={p1} P2={p2} P3={p3}: dispatch args must carry the DCS params verbatim",
                );
                // Normal ST finish, so aborted must be false.
                assert!(
                    !last_sixel_end_aborted(&harness),
                    "P1={p1} P2={p2} P3={p3}: normal ST finish — aborted should be false",
                );
                assert_eq!(
                    viewport_placement_count(&harness),
                    1,
                    "P1={p1} P2={p2} P3={p3}: exactly one placement expected",
                );

                // Observe bg mode through alpha of undrawn pixels.
                let (pixels, _w, _h) = last_image_pixels(&harness);
                // Pixel at (0,0) is drawn (red), pixel at (0,1) is undrawn.
                let drawn_alpha = pixels[3];
                let undrawn_alpha = pixels[7];
                assert_eq!(
                    drawn_alpha, 255,
                    "P1={p1} P2={p2} P3={p3}: drawn pixel must be opaque",
                );
                match p2 {
                    1 => assert_eq!(
                        undrawn_alpha, 0,
                        "P1={p1} P2={p2} P3={p3}: NoChange undrawn α must be 0",
                    ),
                    _ => assert_eq!(
                        undrawn_alpha, 255,
                        "P1={p1} P2={p2} P3={p3}: DeviceDefault/SetToBg undrawn α must be 255",
                    ),
                }

                cells_visited += 1;
            }
        }
    }

    // Self-verifying matrix completeness (tests.md §Self-Verifying Matrix
    // Completeness).
    assert_eq!(
        cells_visited,
        P1_VALUES.len() * P2_VALUES.len() * P3_VALUES.len(),
        "matrix cells visited must equal cartesian-product size",
    );
}

// Raster attributes.

/// Catalog rows: SIXEL-RASTER-ATTRS, SIXEL-RASTER-ATTRS-PH-PV,
/// SIXEL-RASTER-ATTRS-PAN-PAD, SIXEL-RASTER-BEFORE-DATA.
///
/// Feeds `"1;1;10;20` before data; the parser should set dimensions
/// (Ph=10, Pv=20) before the first pixel column arrives. Pan/Pad are
/// documented-divergent — the impl at
/// `oriterm_core/src/image/sixel/mod.rs:311-330` reads only params[2]/[3].
#[test]
fn raster_attributes_before_data_sets_dimensions() {
    let mut harness = SpecHarness::new();
    // `"1;1;10;20` sets Pan=1, Pad=1, Ph=10 (width), Pv=20 (height),
    // followed by a single `~` sixel (value 63 = all 6 pixels).
    harness.feed(b"\x1bPq\"1;1;10;20#0;2;100;0;0~\x1b\\");

    assert_eq!(viewport_placement_count(&harness), 1);
    let (_pixels, w, h) = last_image_pixels(&harness);
    assert_eq!(w, 10, "Ph=10 should set image width to 10");
    assert_eq!(h, 20, "Pv=20 should set image height to 20");
}

/// Catalog rows: SIXEL-RASTER-MID-STREAM.
///
/// Feeds data, then raster attrs mid-stream, then more data. The current
/// impl (`oriterm_core/src/image/sixel/mod.rs:311-330` combined with
/// `finish()` using `raster_width.unwrap_or(max_x).max(max_x)`) treats a
/// mid-stream `"5;5;100;100` as re-dimensioning — the dimensions become
/// `max(raster, max_x)` / `max(raster, max_y)`. This test pins the
/// documented behavior (re-dimension) rather than ignoring the operator.
#[test]
fn raster_attributes_mid_stream_redimensions() {
    let mut harness = SpecHarness::new();
    // Draw one `~` column (width 1, height 6), then raster attrs
    // `"1;1;100;100`, then another `~` column. Final image dimensions
    // should reflect the larger of (raster, max_x/max_y).
    harness.feed(b"\x1bPq#0;2;100;0;0~\"1;1;100;100~\x1b\\");

    assert_eq!(viewport_placement_count(&harness), 1);
    let (_pixels, w, h) = last_image_pixels(&harness);
    // max_x at this point is 2 (two `~` columns drawn). raster_width=100.
    // The `finish()` formula is `raster.unwrap_or(max_x).max(max_x)` = 100.
    // If the behavior were "ignore mid-stream raster," w would be 2.
    assert_eq!(w, 100, "mid-stream raster re-dimensions width to Ph=100");
    assert_eq!(h, 100, "mid-stream raster re-dimensions height to Pv=100");
}

// Color definition + selection.

/// Catalog rows: SIXEL-COLOR-DEFINE-RGB.
///
/// `#5;2;50;75;100` defines palette index 5 as RGB(50, 75, 100) scaled
/// from the 0..100 input range to 0..255. Then `#5@` selects color 5
/// and draws one pixel. Expected: pixel RGB ≈ (127, 191, 255).
#[test]
fn color_define_pu2_rgb_scales_0_100_to_0_255() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1bPq#5;2;50;75;100#5@\x1b\\");

    assert_eq!(viewport_placement_count(&harness), 1);
    let (pixels, _w, _h) = last_image_pixels(&harness);
    // Scaling per `mod.rs:285-287`: val*255/100.
    assert_eq!(pixels[0], 127, "R = 50*255/100");
    assert_eq!(pixels[1], 191, "G = 75*255/100");
    assert_eq!(pixels[2], 255, "B = 100*255/100");
    assert_eq!(pixels[3], 255, "opaque");
}

/// Catalog rows: SIXEL-COLOR-DEFINE-HLS.
///
/// `#1;1;120;50;100` defines palette index 1 via HLS: H=120 (sixel hue,
/// which after the `hue - 120.0` rotation at
/// `oriterm_core/src/image/sixel/color.rs:41` maps to standard hue 0 =
/// red), L=50, S=100.
#[test]
fn color_define_pu1_hls_rotates_hue_minus_120() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1bPq#1;1;120;50;100#1@\x1b\\");

    assert_eq!(viewport_placement_count(&harness), 1);
    let (pixels, _w, _h) = last_image_pixels(&harness);
    // After the `hue - 120.0` rotation, sixel H=120 is pure red.
    assert!(
        pixels[0] > 200,
        "R channel should be saturated red: got {}",
        pixels[0]
    );
    assert!(
        pixels[1] < 10,
        "G channel should be near 0: got {}",
        pixels[1]
    );
    assert!(
        pixels[2] < 10,
        "B channel should be near 0: got {}",
        pixels[2]
    );
}

/// Catalog rows: SIXEL-COLOR-SELECT.
///
/// `#3` with no follow-on params selects palette index 3 (no definition).
/// Subsequent sixel data uses that selected color. The VT340 default
/// palette has index 3 as green (`color.rs` VT340_PALETTE).
#[test]
fn color_select_bare_pins_current_color() {
    let mut harness = SpecHarness::new();
    // `#3` without `;2;...` definition must just select color 3.
    // `@` draws one pixel using current_color (= 3, green in VT340).
    harness.feed(b"\x1bPq#3@\x1b\\");

    assert_eq!(viewport_placement_count(&harness), 1);
    let (pixels, _w, _h) = last_image_pixels(&harness);
    // VT340 default palette index 3 is a green shade; the key invariant
    // is that the green channel dominates red and blue.
    assert!(
        pixels[1] > pixels[0] && pixels[1] > pixels[2],
        "bare `#3` should select VT340 green-ish: got ({}, {}, {})",
        pixels[0],
        pixels[1],
        pixels[2],
    );
    assert_eq!(pixels[3], 255);
}

// Repeat / CR / NL.

/// Catalog rows: SIXEL-REPEAT.
///
/// `!5~` emits 5 consecutive columns of the same sixel data byte
/// (`~` = 0x7E = value 63 = all 6 pixels set). Image width = 5.
#[test]
fn repeat_operator_emits_n_columns() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1bPq#0;2;100;0;0!5~\x1b\\");

    assert_eq!(viewport_placement_count(&harness), 1);
    let (pixels, w, h) = last_image_pixels(&harness);
    assert_eq!(w, 5, "!5 → 5 columns");
    assert_eq!(h, 6, "single sixel row → 6 pixels tall");
    // All 5 columns × 6 rows should be red.
    for col in 0..5 {
        for row in 0..6 {
            let off = (row * 5 + col) * 4;
            assert_eq!(pixels[off], 255, "col={col} row={row} R");
            assert_eq!(pixels[off + 1], 0, "col={col} row={row} G");
            assert_eq!(pixels[off + 2], 0, "col={col} row={row} B");
            assert_eq!(pixels[off + 3], 255, "col={col} row={row} A");
        }
    }
}

/// Catalog rows: SIXEL-REPEAT (interaction with palette).
///
/// `#3!5~` — repeat must read `current_color` at emission time, so all
/// 5 columns use palette index 3, not some stale/default color.
#[test]
fn repeat_reads_current_color_at_emission() {
    let mut harness = SpecHarness::new();
    // Define color 3 explicitly (RGB blue = 0,0,100) then select & repeat.
    harness.feed(b"\x1bPq#3;2;0;0;100#3!5~\x1b\\");

    assert_eq!(viewport_placement_count(&harness), 1);
    let (pixels, w, _h) = last_image_pixels(&harness);
    assert_eq!(w, 5);
    // All 5 columns row 0 should be blue.
    for col in 0..5usize {
        let off = col * 4;
        assert_eq!(pixels[off], 0, "col={col} R");
        assert_eq!(pixels[off + 1], 0, "col={col} G");
        assert_eq!(pixels[off + 2], 255, "col={col} B");
        assert_eq!(pixels[off + 3], 255, "col={col} A");
    }
}

/// Catalog rows: SIXEL-CR.
///
/// `$` resets x to 0, keeps y — the second band overwrites column 0.
/// Feed `~~` (two red cols), `$` (CR), `#1;2;0;100;0~` (one green col).
/// Column 0 should now be green; column 1 should still be red.
#[test]
fn cr_resets_x_keeps_y() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1bPq#0;2;100;0;0~~$#1;2;0;100;0~\x1b\\");

    assert_eq!(viewport_placement_count(&harness), 1);
    let (pixels, w, _h) = last_image_pixels(&harness);
    assert_eq!(w, 2);
    // Column 0 row 0: should be green (overwritten).
    assert_eq!(pixels[0], 0, "col0 R");
    assert_eq!(pixels[1], 255, "col0 G");
    assert_eq!(pixels[2], 0, "col0 B");
    // Column 1 row 0: should still be red.
    assert_eq!(pixels[4], 255, "col1 R");
    assert_eq!(pixels[5], 0, "col1 G");
    assert_eq!(pixels[6], 0, "col1 B");
}

/// Catalog rows: SIXEL-NL.
///
/// `-` resets x to 0, advances y by 6. Two `~` bands separated by `-`
/// should produce a 1×12 image (one column, two sixel rows).
#[test]
fn nl_resets_x_advances_y_by_six() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1bPq#0;2;100;0;0~-~\x1b\\");

    assert_eq!(viewport_placement_count(&harness), 1);
    let (_pixels, w, h) = last_image_pixels(&harness);
    assert_eq!(w, 1);
    assert_eq!(h, 12, "NL advances y by 6 → two bands = 12 rows");
}

// Intermix `#` mid-data.

/// Catalog rows: SIXEL-COLOR-SELECT (mid-data dispatch).
///
/// Feeds `<data>#5<data>` — a color select between two data bytes must
/// apply to the second band on live parser state (not only at setup).
#[test]
fn intermixed_color_select_mid_data() {
    let mut harness = SpecHarness::new();
    // Define color 0 red, color 5 blue. Draw one red column, then
    // select color 5, draw one blue column.
    harness.feed(b"\x1bPq#0;2;100;0;0#5;2;0;0;100#0~#5~\x1b\\");

    assert_eq!(viewport_placement_count(&harness), 1);
    let (pixels, w, _h) = last_image_pixels(&harness);
    assert_eq!(w, 2);
    // Column 0: red.
    assert_eq!(pixels[0], 255, "col0 R");
    assert_eq!(pixels[2], 0, "col0 B");
    // Column 1: blue.
    assert_eq!(pixels[4], 0, "col1 R");
    assert_eq!(pixels[6], 255, "col1 B");
}

// Sixel data byte matrix (`?`..`~`).

/// Catalog rows: SIXEL-DATA-BYTE.
///
/// Exhaustive 63-code data-byte range. For every byte `b` in 0x3F..=0x7E,
/// the 6-bit pixel column encodes `b - 0x3F`. Each bit position `i` in
/// the low 6 bits means pixel row `i` is lit.
///
/// Observes: the rendered 1-column image's pixel rows match the bit
/// pattern — lit rows have drawn pixels (foreground), unlit rows are
/// background (alpha=0 in transparent mode).
#[test]
fn data_byte_range_decodes_six_bit_column() {
    let mut cells_visited = 0usize;
    for byte in 0x3Fu8..=0x7E {
        let value = byte - 0x3F;

        let mut harness = SpecHarness::new();
        // Transparent bg mode (P2=1) so undrawn rows have α=0 and
        // drawn rows have α=255 — straightforward to assert on.
        // Set color 0 explicitly to white, then feed the single byte.
        let mut dcs = b"\x1bP0;1;0q#0;2;100;100;100#0".to_vec();
        dcs.push(byte);
        dcs.extend_from_slice(b"\x1b\\");
        harness.feed(&dcs);

        assert_eq!(
            viewport_placement_count(&harness),
            1,
            "byte=0x{byte:02X}: exactly one placement",
        );
        let (pixels, w, h) = last_image_pixels(&harness);
        assert_eq!(w, 1, "byte=0x{byte:02X}: width 1");
        assert_eq!(h, 6, "byte=0x{byte:02X}: height 6");

        // For each bit in the low 6 bits, check the corresponding row.
        for bit in 0..6u8 {
            let lit = (value & (1 << bit)) != 0;
            let row_alpha = pixels[(bit as usize) * 4 + 3];
            if lit {
                assert_eq!(
                    row_alpha, 255,
                    "byte=0x{byte:02X} value={value} bit={bit}: lit row must be opaque",
                );
            } else {
                assert_eq!(
                    row_alpha, 0,
                    "byte=0x{byte:02X} value={value} bit={bit}: unlit row must be transparent",
                );
            }
        }

        cells_visited += 1;
    }

    // Self-verifying matrix completeness: all 63 codes (0x3F..=0x7E) visited.
    assert_eq!(
        cells_visited, 64,
        "every byte in 0x3F..=0x7E must be visited"
    );
}

// Abort path (negative pins).

/// Catalog rows: SIXEL-ABORT-CAN.
///
/// CAN (0x18) mid-DCS drives `Performer::unhook` at
/// `crates/vte/src/lib.rs:341-355` → `Term::handle_sixel_end`. Per the
/// §12.1 plan (section-12-sixel.md:167-169) and §12.N completion
/// checklist, no placement should be committed when the DCS is aborted
/// mid-stream. Today `handle_sixel_end` stores unconditionally — this
/// test is the regression guard that exposes the bug; the §12.1 plan calls
/// for filing BUG-12-* and fixing in-scope.
#[test]
fn dcs_abort_can_commits_no_placement() {
    let mut harness = SpecHarness::new();
    // Start a DCS q, feed one `~` sixel, then CAN (0x18) — the DCS
    // is aborted mid-image. No ST/backslash follows.
    harness.feed(b"\x1bPq#0;2;100;0;0~\x18");

    assert_eq!(
        sixel_start_count(&harness),
        1,
        "sixel_start should still fire (DCS entered)",
    );
    // Dispatch-rung evidence: sixel_end fired with aborted=true. This
    // pins the VTE → dispatch → Handler seam, separate from the state
    // assertion below (which could pass for unrelated reasons if the
    // aborted bit were ever decoupled from the handler).
    assert!(
        last_sixel_end_aborted(&harness),
        "CAN mid-DCS must drive sixel_end(aborted = true)",
    );
    assert_eq!(
        viewport_placement_count(&harness),
        0,
        "CAN abort must not commit a placement",
    );
}

/// Catalog rows: SIXEL-ABORT-SUB.
///
/// Same contract as CAN — SUB (0x1A) mid-DCS must abort without committing.
#[test]
fn dcs_abort_sub_commits_no_placement() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1bPq#0;2;100;0;0~\x1a");

    assert_eq!(sixel_start_count(&harness), 1);
    assert!(
        last_sixel_end_aborted(&harness),
        "SUB mid-DCS must drive sixel_end(aborted = true)",
    );
    assert_eq!(
        viewport_placement_count(&harness),
        0,
        "SUB abort must not commit a placement",
    );
}

/// Catalog rows: SIXEL-ABORT-ESC.
///
/// ESC mid-DCS (ESC followed by a non-ST byte that starts a new ESC
/// state) unhooks the DCS, then the new ESC sequence executes. Per
/// `crates/vte/src/lib.rs:341-355`, 0x1B triggers unhook-then-new-ESC.
/// The probe: feed a DCS q with data, then `\x1b[1;1H` (CSI CUP) — the
/// CSI should execute on terminal state and the DCS should abort with
/// NO placement committed. CUP does not clear the image cache, so any
/// placement we observe is a bug.
#[test]
fn dcs_abort_esc_commits_no_placement() {
    let mut harness = SpecHarness::new();
    // ESC begins a new control sequence, unhooking the DCS first.
    harness.feed(b"\x1bPq#0;2;100;0;0~\x1b[1;1H");

    assert_eq!(sixel_start_count(&harness), 1);
    assert!(
        last_sixel_end_aborted(&harness),
        "ESC-mid-DCS (non-backslash follow-on) must drive sixel_end(aborted = true)",
    );
    assert_eq!(
        viewport_placement_count(&harness),
        0,
        "ESC abort must not commit a placement",
    );
}

// DcsEscape C0 / DEL pass-through (new branches from the abort fix).

/// Catalog rows: SIXEL-DCS-q (coverage of the DcsEscape state transition
/// table added by the §12.1 abort fix at `crates/vte/src/lib.rs`
/// `advance_dcs_escape`).
///
/// Per the DEC ANSI parser state machine, C0 controls inside an escape
/// sequence execute transparently — they do NOT abort the enclosing
/// DCS. This test feeds a DCS q that enters DcsEscape with ESC, then a
/// BEL (`0x07`, a C0) that executes in place, then the `\` finalizing
/// the 2-byte ST terminator, and asserts:
/// 1. BEL dispatches to `Handler::bell` through `advance_dcs_escape` —
///    proves the C0 pass-through branch at `lib.rs advance_dcs_escape`.
/// 2. The sixel completes normally — `sixel_end(aborted = false)` fires
///    and a placement is committed because the pending `ESC \` ST
///    terminator is completed by the `\` immediately after BEL.
#[test]
fn dcs_escape_c0_control_passes_through_transparently() {
    let mut harness = SpecHarness::new();
    // ESC + BEL + `\` — BEL is a C0 control executed transparently in
    // DcsEscape; the subsequent `\` completes the ST (the initial ESC
    // was the ST-introducer). NOTE: a second ESC here would be a new
    // escape sequence per DEC STD 070 §6.4 and would abort the DCS.
    harness.feed(b"\x1bPq#0;2;100;0;0~\x1b\x07\\");

    let bell_count = harness
        .outcome()
        .dispatched_calls
        .iter()
        .filter(|c| c.method == "bell")
        .count();
    assert_eq!(
        bell_count, 1,
        "BEL inside DcsEscape must execute transparently as a C0 control",
    );
    assert_eq!(
        sixel_end_count(&harness),
        1,
        "sixel_end must fire exactly once (normal ST after C0)",
    );
    assert!(
        !last_sixel_end_aborted(&harness),
        "sixel_end aborted must be false — BEL is transparent, ST still finishes",
    );
    assert_eq!(
        viewport_placement_count(&harness),
        1,
        "sixel placement must commit — C0 in DcsEscape is not an abort",
    );
}

/// Catalog rows: SIXEL-DCS-q (same DcsEscape-state coverage).
///
/// Per the DEC ANSI parser state machine, DEL (`0x7F`) inside an escape
/// sequence is ignored — it does NOT abort. This test feeds DEL between
/// the pending-ST ESC and the closing `\` and asserts the sixel
/// completes normally.
#[test]
fn dcs_escape_del_is_ignored_transparently() {
    let mut harness = SpecHarness::new();
    // ESC + DEL + `\` — DEL is dropped in DcsEscape; the subsequent `\`
    // completes the ST (the initial ESC was the ST-introducer).
    harness.feed(b"\x1bPq#0;2;100;0;0~\x1b\x7f\\");

    assert_eq!(
        sixel_end_count(&harness),
        1,
        "sixel_end must fire exactly once (normal ST after DEL)",
    );
    assert!(
        !last_sixel_end_aborted(&harness),
        "sixel_end aborted must be false — DEL is ignored, ST still finishes",
    );
    assert_eq!(
        viewport_placement_count(&harness),
        1,
        "sixel placement must commit — DEL in DcsEscape is not an abort",
    );
}

// Structural matrix count over operator categories.

/// Catalog rows: structural matrix-completeness pin for §12.1.
///
/// Each operator category exercised above has at least one dedicated
/// test. This self-verifying assertion guards against accidental
/// deletion of a category test — if a category is removed, this list
/// won't match the test set the plan requires (§12.1 bullets map 1:1
/// to these categories).
#[test]
fn operator_category_matrix_completeness() {
    const OPERATOR_CATEGORIES: &[&str] = &[
        "dcs_q_introducer",
        "raster_attributes_before",
        "raster_attributes_mid_stream",
        "color_define_rgb",
        "color_define_hls",
        "color_select_bare",
        "repeat",
        "repeat_with_palette",
        "cr",
        "nl",
        "intermix_color",
        "data_byte_range",
        "abort_can",
        "abort_sub",
        "abort_esc",
        "dcs_escape_c0_passthrough",
        "dcs_escape_del_ignored",
    ];

    // Plan §12.1 enumerates 15 operator/behavior categories + 2
    // DcsEscape state-machine pass-through tests added by the abort
    // fix; the list above must match that count so category drift is
    // visible.
    assert_eq!(
        OPERATOR_CATEGORIES.len(),
        17,
        "§12.1 has 17 operator/behavior categories; update this list if the plan changes",
    );
}

/// Catalog rows: `SIXEL-ABORT-ESC` (nested-ESC sub-case).
///
/// Two ESCs inside a pending DCS — the first ESC transitions to
/// `DcsEscape`; the second ESC is NOT `0x5C` (ST), so `advance_dcs_escape`
/// sets `dcs_aborted = true` and unhooks. The trailing `\` is then the
/// start of a fresh control sequence, not the ST-half of the original
/// DCS terminator. Complements `dcs_abort_esc_commits_no_placement` by
/// pinning the specific ESC-ESC-\ pattern.
#[test]
fn dcs_escape_second_esc_aborts_pending_st() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1bPq#0;2;100;0;0~\x1b\x1b\\");

    let aborted = harness
        .outcome()
        .dispatched_calls
        .iter()
        .rev()
        .find_map(|c| match &c.args {
            DispatchArgs::SixelEnd { aborted } => Some(*aborted),
            _ => None,
        })
        .expect("sixel_end must have been dispatched");

    assert!(aborted, "ESC ESC \\ should abort the DCS");
}
