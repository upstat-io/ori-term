//! Spec_chain conversion of the tack `character_sets` scenario family.
//!
//! Per `crates/oriterm_test_support/src/tack_framework/scenarios/character_sets/mod.rs`,
//! tack's `t -> c) ANSI character sets` tool draws a composite screen
//! showing GL+GR banks with the currently-designated charset. Tack
//! v1.08 pre-designates G1 to DEC special graphics so the preview
//! pane renders `◆▒␉... ┘┐┌└┼⎺⎻─⎼⎽├┤┴┬│` before the user types any
//! bank-switch bytes. Under the hood, these characters ride on
//! `ESC ( 0` / `ESC ) 0` (SCS designation) + SO/SI (bank switching).
//!
//! # Catalog rows verified
//!
//! - `ECMA48-ESC-0` — Designate DEC Special Graphics to a G-bank
//!   (`ESC ( 0`, `ESC ) 0`, `ESC * 0`, `ESC + 0`)
//! - `ECMA48-ESC-B` — Designate ASCII to a G-bank (`ESC ( B`, etc.)
//! - `ECMA48-C0-SO` — Shift Out (`0x0E`), activates G1
//! - `ECMA48-C0-SI` — Shift In (`0x0F`), activates G0
//!
//! Section 18 (Charsets + UAX Policy) owns exhaustive per-G-bank
//! coverage of the full ISO 2022 designation matrix (G0–G3 across all
//! supported charsets). The four rows above are the subset tack's
//! preview pane actually exercises at tack v1.08; spec_chain coverage
//! here is the mandatory minimum.

use oriterm_test_support::spec_chain::{
    ApexLayer, DispatchExpectation, ParserExpectation, ScenarioExpectations, SpecHarness,
    SpecScenario,
};
use vte::ansi::CharsetIndex;

// --- ESC ( 0 (designate G0 to DEC special graphics) -----------------------

/// `ESC ( 0` designates DEC Special Graphics to G0.
///
/// Drives the parser (EscDispatch byte `0x30`, intermediate `(`),
/// the dispatch rung (`configure_charset`), and the state snapshot
/// (the G0 slot now holds `StandardCharset::SpecialCharacterAndLineDrawing`).
#[test]
fn esc_g0_dec_special_graphics_drives_to_state_apex() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-ESC-0",
        bytes: b"\x1b(0",
        apex_layer: ApexLayer::State,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation {
                action: '0',
                params: &[],
                intermediates: b"(",
                osc_command: None,
            }),
            dispatch: Some(DispatchExpectation::method("configure_charset")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    // A DEC special graphics designation translates ASCII `q` to the
    // horizontal box-drawing char `─` once G0 is active. Activating G0
    // is the default after SCS (no SI/SO needed on a fresh harness).
    // The charset state exposes `is_ascii()` — after designating DEC
    // special graphics to G0, the active charset is no longer ASCII.
    assert!(
        !harness.term().charset().is_ascii(),
        "after ESC ( 0, active charset must not be ASCII"
    );
}

// --- ESC ) 0 (designate G1 to DEC special graphics) -----------------------

/// `ESC ) 0` designates DEC Special Graphics to G1.
///
/// Tack's preview pane relies on this designation + SO to show the
/// line-drawing glyphs. Separate scenario from the G0 path so the
/// per-bank configure_charset dispatches are individually pinned.
#[test]
fn esc_g1_dec_special_graphics_drives_to_state_apex() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-ESC-0",
        bytes: b"\x1b)0",
        apex_layer: ApexLayer::State,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation {
                action: '0',
                params: &[],
                intermediates: b")",
                osc_command: None,
            }),
            dispatch: Some(DispatchExpectation::method("configure_charset")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    // G1 was configured but G0 is still active (default). Switching
    // to G1 via SO activates DEC special graphics.
    harness.feed(b"\x0E");
    assert!(
        !harness.term().charset().is_ascii(),
        "after ESC ) 0 then SO, active charset must be DEC special graphics"
    );
}

// --- ESC ( B (designate G0 to ASCII) ---------------------------------------

/// `ESC ( B` designates ASCII to G0.
///
/// The default charset state is already ASCII, so the post-assertion
/// checks round-trip: designate G0 DEC special graphics first, then
/// designate G0 back to ASCII, and verify we are ASCII again.
#[test]
fn esc_g0_ascii_round_trip() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-ESC-B",
        bytes: b"\x1b(B",
        apex_layer: ApexLayer::State,
        setup: b"\x1b(0", // First designate G0 DEC special graphics
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation {
                action: 'B',
                params: &[],
                intermediates: b"(",
                osc_command: None,
            }),
            dispatch: Some(DispatchExpectation::method("configure_charset")),
            ..ScenarioExpectations::default()
        },
    };

    // Pre-flight the setup on a separate harness so we can pin that
    // the DEC-graphics designation happened before the ASCII reset.
    let mut pre_harness = SpecHarness::new();
    pre_harness.feed(scenario.setup);
    assert!(
        !pre_harness.term().charset().is_ascii(),
        "setup bytes must designate G0 as DEC special graphics on a fresh harness"
    );

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    assert!(
        harness.term().charset().is_ascii(),
        "after ESC ( B, active charset must be ASCII again"
    );
}

// --- SO / SI (bank switching) ---------------------------------------------

/// `SO` (`0x0E`) activates G1 as the working GL charset.
///
/// Checks the dispatch routes through `set_active_charset(G1)` (per
/// `crates/vte/src/ansi/dispatch/mod.rs:46`) and the charset state's
/// `active()` returns `G1` afterward.
#[test]
fn so_activates_g1() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-C0-SO",
        bytes: b"\x0E",
        apex_layer: ApexLayer::State,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation {
                action: '\x0E',
                params: &[],
                intermediates: &[],
                osc_command: None,
            }),
            dispatch: Some(DispatchExpectation::method("set_active_charset")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    assert_eq!(
        *harness.term().charset().active(),
        CharsetIndex::G0,
        "default active charset must be G0"
    );

    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    assert_eq!(
        *harness.term().charset().active(),
        CharsetIndex::G1,
        "after SO (0x0E), active charset must be G1"
    );
}

/// `SI` (`0x0F`) activates G0 as the working GL charset.
///
/// Matrix pair with `so_activates_g1`: feed SO first to move to G1,
/// then feed SI and assert we return to G0.
#[test]
fn si_activates_g0() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-C0-SI",
        bytes: b"\x0F",
        apex_layer: ApexLayer::State,
        setup: b"\x0E", // SO first, so we start in G1
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation {
                action: '\x0F',
                params: &[],
                intermediates: &[],
                osc_command: None,
            }),
            dispatch: Some(DispatchExpectation::method("set_active_charset")),
            ..ScenarioExpectations::default()
        },
    };

    // Pre-flight the setup on a separate harness so we can pin that
    // SO actually moved the active charset to G1.
    let mut pre_harness = SpecHarness::new();
    pre_harness.feed(scenario.setup);
    assert_eq!(
        *pre_harness.term().charset().active(),
        CharsetIndex::G1,
        "setup (SO) must activate G1 on a fresh harness"
    );

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    assert_eq!(
        *harness.term().charset().active(),
        CharsetIndex::G0,
        "after SI (0x0F), active charset must be G0"
    );
}

// --- Integration: tack's preview pane end-to-end --------------------------

/// End-to-end: `ESC ) 0` + SO + `q` renders as `─` (box-drawing).
///
/// This is the byte sequence tack's preview pane relies on. After
/// designating G1 to DEC special graphics and shifting out, a
/// subsequent ASCII `q` in the input stream must translate to the
/// horizontal box-drawing char `─` when it reaches the grid.
#[test]
fn preview_pane_renders_box_drawing_after_g1_dec_graphics() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b)0\x0Eq");

    let cell = &harness.term().grid()[oriterm_core::Line(0)][oriterm_core::Column(0)];
    assert_eq!(
        cell.ch, '─',
        "after ESC ) 0, SO, `q` must render as `─` (U+2500); got {:?}",
        cell.ch
    );
}

// --- Negative pin ---------------------------------------------------------

/// Negative pin: `ESC ( B` (ASCII) in a fresh harness must NOT render `q` as `─`.
///
/// Proves the translation layer is conditional on the active
/// charset, not blanketly applied. A regression that always
/// translated `q` to box-drawing (ignoring the active charset)
/// would pass the positive preview test above.
#[test]
fn ascii_g0_renders_q_literally_not_as_box_drawing() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b(Bq"); // ASCII G0 (no SO), then `q`

    let cell = &harness.term().grid()[oriterm_core::Line(0)][oriterm_core::Column(0)];
    assert_eq!(
        cell.ch, 'q',
        "with ASCII G0 active, `q` must render as `q`; got {:?}",
        cell.ch
    );
}
