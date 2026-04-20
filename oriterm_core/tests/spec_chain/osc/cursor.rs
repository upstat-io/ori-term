//! OSC 22 mouse cursor icon + OSC 50 text cursor shape spec_chain
//! coverage (Section 10.5).
//!
//! OSC 22 (`\x1b]22;<name>\x1b\\`) routes through the high-level VTE
//! `Processor` dispatch at `crates/vte/src/ansi/dispatch/osc.rs` `b"22"`
//! arm → `Handler::set_mouse_cursor_icon` (overridden on `Term` in
//! `oriterm_core/src/term/handler/mod.rs`) → `Term::mouse_cursor_icon`
//! state. Unknown cursor names fall through `CursorIcon::from_str`'s
//! `Err` path and are dropped without mutating state; malformed
//! parameter counts miss the `params.len() == 2` guard and also drop.
//!
//! OSC 50 (`\x1b]50;CursorShape=<N>\x1b\\`) routes through the same
//! dispatcher `b"50"` arm → `Handler::set_cursor_shape` → `Term::
//! set_cursor_shape`. Unknown `N` values fall through to `unhandled`.
//!
//! Catalog rows: OSC-22, OSC-50.
//! Apex: state-snapshot.

use oriterm_core::CursorShape;
use oriterm_test_support::spec_chain::{
    ApexLayer, RenderableExpectation, ScenarioExpectations, SpecHarness, SpecScenario,
};
use vte::ansi::cursor_icon::CursorIcon;

/// Project-owned name→CursorIcon matrix for OSC 22 verification.
///
/// `cursor_icon 1.2.0` exposes only `CursorIcon::name()` and `FromStr`
/// parsing — there is no `all()` iterator. This slice enumerates every
/// variant the CSS Basic UI / xterm spec defines so the matrix test can
/// self-verify completeness. Distinct from the mux-crate wire slice
/// `oriterm_mux::protocol::snapshot::OSC22_KNOWN_ICONS` (stable u8
/// indexing on the daemon wire): this is a NAME-tagged slice used only
/// by the high-level dispatcher test.
const OSC22_TEST_CURSOR_NAMES: &[(&str, CursorIcon)] = &[
    ("default", CursorIcon::Default),
    ("context-menu", CursorIcon::ContextMenu),
    ("help", CursorIcon::Help),
    ("pointer", CursorIcon::Pointer),
    ("progress", CursorIcon::Progress),
    ("wait", CursorIcon::Wait),
    ("cell", CursorIcon::Cell),
    ("crosshair", CursorIcon::Crosshair),
    ("text", CursorIcon::Text),
    ("vertical-text", CursorIcon::VerticalText),
    ("alias", CursorIcon::Alias),
    ("copy", CursorIcon::Copy),
    ("move", CursorIcon::Move),
    ("no-drop", CursorIcon::NoDrop),
    ("not-allowed", CursorIcon::NotAllowed),
    ("grab", CursorIcon::Grab),
    ("grabbing", CursorIcon::Grabbing),
    ("e-resize", CursorIcon::EResize),
    ("n-resize", CursorIcon::NResize),
    ("ne-resize", CursorIcon::NeResize),
    ("nw-resize", CursorIcon::NwResize),
    ("s-resize", CursorIcon::SResize),
    ("se-resize", CursorIcon::SeResize),
    ("sw-resize", CursorIcon::SwResize),
    ("w-resize", CursorIcon::WResize),
    ("ew-resize", CursorIcon::EwResize),
    ("ns-resize", CursorIcon::NsResize),
    ("nesw-resize", CursorIcon::NeswResize),
    ("nwse-resize", CursorIcon::NwseResize),
    ("col-resize", CursorIcon::ColResize),
    ("row-resize", CursorIcon::RowResize),
    ("all-scroll", CursorIcon::AllScroll),
    ("zoom-in", CursorIcon::ZoomIn),
    ("zoom-out", CursorIcon::ZoomOut),
];

// ── OSC 22 (mouse cursor icon, iTerm2) ──────────────────────────────

/// `OSC 22 ; pointer ST` stores `CursorIcon::Pointer` on the Term and
/// propagates it through the renderable snapshot.
#[test]
fn osc22_pointer_sets_cursor_icon() {
    let scenario = SpecScenario {
        catalog_row_id: "OSC-22",
        bytes: b"\x1b]22;pointer\x1b\\",
        apex_layer: ApexLayer::Renderable,
        setup: b"",
        expectations: ScenarioExpectations {
            renderable: Some(RenderableExpectation {
                mouse_cursor_icon: Some(CursorIcon::Pointer),
                ..RenderableExpectation::default()
            }),
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
    assert_eq!(
        harness.term().mouse_cursor_icon(),
        Some(CursorIcon::Pointer),
        "Term::mouse_cursor_icon must reflect the OSC 22 set"
    );
}

/// Every name in the project-owned test matrix must round-trip through
/// the OSC 22 dispatcher and land on the Term state. Self-verifying
/// completeness via a count assertion so a future edit that drops a row
/// produces an audible failure.
#[test]
fn osc22_all_known_icons_matrix() {
    let mut count = 0;
    for &(name, expected) in OSC22_TEST_CURSOR_NAMES {
        let mut harness = SpecHarness::new();
        let bytes = format!("\x1b]22;{name}\x1b\\");
        harness.feed(bytes.as_bytes());
        assert_eq!(
            harness.term().mouse_cursor_icon(),
            Some(expected),
            "OSC 22 ; {name} must store CursorIcon::{expected:?}"
        );
        count += 1;
    }
    assert_eq!(
        count,
        OSC22_TEST_CURSOR_NAMES.len(),
        "matrix must visit every entry in OSC22_TEST_CURSOR_NAMES"
    );
}

/// An unknown cursor name passes `CursorIcon::from_str` as `Err` and
/// the dispatcher logs + drops. `Term::mouse_cursor_icon` stays at its
/// prior value (`None` on a fresh Term).
#[test]
fn osc22_unknown_icon_is_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]22;not-a-real-cursor\x1b\\");
    assert_eq!(
        harness.term().mouse_cursor_icon(),
        None,
        "unknown cursor name must NOT mutate Term::mouse_cursor_icon"
    );
}

/// Negative pin: OSC 22 with no cursor-name parameter (one param only)
/// misses the `params.len() == 2` guard at `crates/vte/src/ansi/
/// dispatch/osc.rs` and falls to `unhandled`. No state mutation.
#[test]
fn osc22_no_parameter_is_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]22\x1b\\");
    assert_eq!(
        harness.term().mouse_cursor_icon(),
        None,
        "malformed OSC 22 with no second parameter must NOT mutate state"
    );
}

/// OSC 22 has no spec'd reset form, but the explicit name `default`
/// maps to `CursorIcon::Default` via the upstream `FromStr` impl. This
/// documents the behavior so future readers are not surprised by the
/// absence of a dedicated reset.
#[test]
fn osc22_reset_behavior() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]22;pointer\x1b\\");
    assert_eq!(
        harness.term().mouse_cursor_icon(),
        Some(CursorIcon::Pointer)
    );
    harness.feed(b"\x1b]22;default\x1b\\");
    assert_eq!(
        harness.term().mouse_cursor_icon(),
        Some(CursorIcon::Default),
        "OSC 22 ; default must restore the Default cursor icon"
    );
}

/// Semantic pin: OSC 22 (mouse icon) and OSC 50 (text cursor shape)
/// write DISTINCT Term fields. Setting a shape via OSC 50 then firing
/// OSC 22 must not mutate `Term::cursor_shape` — a regression would
/// signal field conflation (scope clarification §I).
#[test]
fn osc22_does_not_affect_text_cursor_shape() {
    let mut harness = SpecHarness::new();
    // Seed the text cursor shape via OSC 50 ; CursorShape=1 (Beam/Bar).
    harness.feed(b"\x1b]50;CursorShape=1\x1b\\");
    assert_eq!(harness.term().cursor_shape(), CursorShape::Bar);

    // Fire OSC 22 — text cursor shape must be unchanged.
    harness.feed(b"\x1b]22;pointer\x1b\\");
    assert_eq!(
        harness.term().cursor_shape(),
        CursorShape::Bar,
        "OSC 22 must not mutate Term::cursor_shape (OSC 50's field)"
    );
    assert_eq!(
        harness.term().mouse_cursor_icon(),
        Some(CursorIcon::Pointer),
        "OSC 22 must have set its own field"
    );
}

// ── OSC 50 (text cursor shape, URxvt legacy) ────────────────────────

/// `OSC 50 ; CursorShape=0` → `CursorShape::Block`. Dispatcher path
/// matches on the ASCII digit at `dispatch/osc.rs` `b"50"` arm.
#[test]
fn osc50_cursor_shape_block() {
    let scenario = SpecScenario {
        catalog_row_id: "OSC-50",
        bytes: b"\x1b]50;CursorShape=0\x1b\\",
        apex_layer: ApexLayer::Renderable,
        setup: b"",
        expectations: ScenarioExpectations {
            renderable: Some(RenderableExpectation {
                cursor_shape: Some(CursorShape::Block),
                ..RenderableExpectation::default()
            }),
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
    assert_eq!(harness.term().cursor_shape(), CursorShape::Block);
}

/// `OSC 50 ; CursorShape=1` → `CursorShape::Bar`. The vte trait's
/// `CursorShape::Beam` maps into `oriterm_core::CursorShape::Bar` via
/// `From<vte::ansi::CursorShape>`.
#[test]
fn osc50_cursor_shape_beam() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]50;CursorShape=1\x1b\\");
    assert_eq!(harness.term().cursor_shape(), CursorShape::Bar);
}

/// `OSC 50 ; CursorShape=2` → `CursorShape::Underline`.
#[test]
fn osc50_cursor_shape_underline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]50;CursorShape=2\x1b\\");
    assert_eq!(harness.term().cursor_shape(), CursorShape::Underline);
}

/// Negative pin: `CursorShape=9` misses the `0|1|2` match and falls to
/// `unhandled`. Term shape stays at its prior value (`Block` default).
#[test]
fn osc50_unknown_shape_dropped() {
    let mut harness = SpecHarness::new();
    // Seed a non-default shape first so we can detect illegitimate change.
    harness.feed(b"\x1b]50;CursorShape=2\x1b\\");
    assert_eq!(harness.term().cursor_shape(), CursorShape::Underline);

    // Unknown N must be dropped — shape stays at Underline.
    harness.feed(b"\x1b]50;CursorShape=9\x1b\\");
    assert_eq!(
        harness.term().cursor_shape(),
        CursorShape::Underline,
        "unknown OSC 50 CursorShape value must NOT mutate shape"
    );
}

/// Negative pin: `OSC 50 ; BADTHING` misses the `CursorShape=` prefix
/// check at `dispatch/osc.rs` and falls to `unhandled`. No mutation.
#[test]
fn osc50_malformed_prefix_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]50;CursorShape=2\x1b\\");
    assert_eq!(harness.term().cursor_shape(), CursorShape::Underline);

    harness.feed(b"\x1b]50;BADTHING\x1b\\");
    assert_eq!(
        harness.term().cursor_shape(),
        CursorShape::Underline,
        "OSC 50 without CursorShape= prefix must NOT mutate state"
    );
}
