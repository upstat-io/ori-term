//! OSC 8 hyperlink spec_chain coverage (Section 10.1).
//!
//! Drives `OSC 8 ; params ; URI ST` through the high-level VTE
//! `Processor` and asserts cell metadata at the state and renderable
//! rungs. The OSC 8 dispatch arm at
//! `crates/vte/src/ansi/dispatch/osc.rs::dispatch` (`b"8"` arm) routes
//! to `Term::set_hyperlink` → `Term::osc_set_hyperlink`
//! (`oriterm_core/src/term/handler/osc.rs:162`); cell attachment uses
//! `Cell::set_hyperlink` (`oriterm_core/src/cell/mod.rs:220`).
//!
//! Catalog row: OSC-8 (`plans/spec-conformance/catalog/osc.md`).
//! Apex: state-snapshot.

use oriterm_core::{Column, Line};
use oriterm_test_support::spec_chain::{
    ApexLayer, RenderableExpectation, ScenarioExpectations, SpecHarness, SpecScenario,
};

const URI: &str = "https://example.com";

/// OSC 8 attaches the URI to every cell printed between the open and
/// close sequences.
///
/// Renderable rung pins position 0 against the URI. State rung sweeps
/// columns 0..5 to confirm every glyph cell carries the attachment, and
/// column 5 (never written) confirms the cursor template's hyperlink is
/// cleared by the empty-URI close form.
#[test]
fn osc8_basic_attach() {
    let scenario = SpecScenario {
        catalog_row_id: "OSC-8",
        bytes: b"\x1b]8;;https://example.com\x1b\\Hello\x1b]8;;\x1b\\",
        apex_layer: ApexLayer::Renderable,
        setup: b"",
        expectations: ScenarioExpectations {
            renderable: Some(RenderableExpectation {
                hyperlink_at: Some((0, 0, URI)),
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

    let grid = harness.term().grid();
    for col in 0..5 {
        let cell = &grid[Line(0)][Column(col)];
        let link = cell.hyperlink().unwrap_or_else(|| {
            panic!("expected hyperlink on cell ({col}), got None");
        });
        assert_eq!(link.uri, URI, "uri mismatch at column {col}");
    }

    // Column 5 was never written and the empty-URI close cleared the
    // cursor template, so the default cell must carry no hyperlink.
    assert!(
        grid[Line(0)][Column(5)].hyperlink().is_none(),
        "column 5 must have no hyperlink after OSC 8 ;; close"
    );
}

/// `OSC 8 ; id=foo ; URI` preserves the optional `id` field on cell
/// metadata, and a second open-with-same-id cycle still carries the id.
#[test]
fn osc8_with_id() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]8;id=foo;https://example.com\x1b\\X\x1b]8;;\x1b\\");
    harness.feed(b" \x1b]8;id=foo;https://example.com\x1b\\Y\x1b]8;;\x1b\\");

    let grid = harness.term().grid();

    let first = grid[Line(0)][Column(0)]
        .hyperlink()
        .expect("cycle 1 cell must carry hyperlink");
    assert_eq!(first.uri, URI);
    assert_eq!(
        first.id.as_deref(),
        Some("foo"),
        "cycle 1 id must be preserved"
    );

    let second = grid[Line(0)][Column(2)]
        .hyperlink()
        .expect("cycle 2 cell must carry hyperlink");
    assert_eq!(second.uri, URI);
    assert_eq!(
        second.id.as_deref(),
        Some("foo"),
        "cycle 2 id must be preserved"
    );
}

/// Hyperlink metadata survives reflow: shrinking the column count must
/// re-flow the wrapped row without dropping the URI on the wrapped
/// continuation.
#[test]
fn osc8_survives_reflow() {
    let mut harness = SpecHarness::new();

    // 60 ASCII characters with the hyperlink attached, so the row is
    // longer than the post-reflow width (40 cols) and must wrap.
    let mut payload = Vec::new();
    payload.extend_from_slice(b"\x1b]8;;https://example.com\x1b\\");
    payload.extend(std::iter::repeat(b'A').take(60));
    payload.extend_from_slice(b"\x1b]8;;\x1b\\");
    harness.feed(&payload);

    // Confirm the entire pre-reflow run carries the URI.
    {
        let grid = harness.term().grid();
        for col in 0..60 {
            let cell = &grid[Line(0)][Column(col)];
            assert_eq!(
                cell.hyperlink().map(|h| h.uri.as_str()),
                Some(URI),
                "pre-reflow column {col} must carry URI"
            );
        }
    }

    // Reflow from 80 → 40 columns. The 60-char run wraps onto rows 0 + 1.
    harness.term_mut().resize(24, 40, true);

    let grid = harness.term().grid();
    for col in 0..40 {
        let cell = &grid[Line(0)][Column(col)];
        assert_eq!(
            cell.hyperlink().map(|h| h.uri.as_str()),
            Some(URI),
            "post-reflow row 0 column {col} must carry URI"
        );
    }
    for col in 0..20 {
        let cell = &grid[Line(1)][Column(col)];
        assert_eq!(
            cell.hyperlink().map(|h| h.uri.as_str()),
            Some(URI),
            "post-reflow row 1 column {col} must carry URI"
        );
    }
}

/// Hyperlink metadata follows rows scrolled into scrollback. Asserts
/// against `ScrollbackBuffer::get` (state rung) — `RenderableContent`
/// only exposes the visible viewport.
#[test]
fn osc8_survives_scrollback() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]8;;https://example.com\x1b\\Hello\x1b]8;;\x1b\\");

    // Push the row off the viewport. 24-line default + 5 spare gives a
    // comfortable margin to land in scrollback at index 0 (newest).
    let mut feeder = Vec::new();
    for _ in 0..30 {
        feeder.extend_from_slice(b"\r\n");
    }
    harness.feed(&feeder);

    let grid = harness.term().grid();
    let scrollback = grid.scrollback();
    assert!(
        scrollback.len() >= 1,
        "scrollback must contain the scrolled-off row"
    );

    // Walk newest → oldest until we find the row whose first cell
    // carries the URI. This avoids hard-coding an exact scrollback
    // index, which depends on Grid::scrollback push timing details.
    let mut found = false;
    for idx in 0..scrollback.len() {
        let row = scrollback.get(idx).expect("index in range");
        if row[Column(0)].hyperlink().is_some_and(|h| h.uri == URI) {
            for col in 0..5 {
                let cell = &row[Column(col)];
                assert_eq!(
                    cell.hyperlink().map(|h| h.uri.as_str()),
                    Some(URI),
                    "scrollback column {col} must carry URI"
                );
            }
            found = true;
            break;
        }
    }
    assert!(
        found,
        "no scrollback row carries the OSC 8 URI on its first cell"
    );
}

/// The empty-URI close form (`OSC 8 ; ; ST`) cancels attachment so the
/// next character carries no hyperlink.
#[test]
fn osc8_terminator_cancels_attachment() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]8;;https://example.com\x1b\\A\x1b]8;;\x1b\\B");

    let grid = harness.term().grid();
    assert_eq!(
        grid[Line(0)][Column(0)].hyperlink().map(|h| h.uri.as_str()),
        Some(URI),
        "first character (open form) must carry URI"
    );
    assert!(
        grid[Line(0)][Column(1)].hyperlink().is_none(),
        "second character (post-close) must carry no URI"
    );
}

/// OSC 8 records the URI verbatim — the dispatcher does not validate
/// or strip whitespace. Truncated sequences (no terminator) attach
/// nothing because the parser never emits the OSC dispatch.
#[test]
fn osc8_malformed_uri_dropped() {
    // Whitespace-laden URI: recorded as-is.
    {
        let mut harness = SpecHarness::new();
        harness.feed(b"\x1b]8;; BROKEN URI WITH SPACES \x1b\\X\x1b]8;;\x1b\\");
        let grid = harness.term().grid();
        let link = grid[Line(0)][Column(0)]
            .hyperlink()
            .expect("whitespace URI should still attach");
        assert_eq!(link.uri, " BROKEN URI WITH SPACES ");
    }

    // Truncated open form (no ST): parser holds the OSC bytes pending
    // a terminator that never arrives, so no dispatch fires and no
    // hyperlink attaches to subsequent input.
    {
        let mut harness = SpecHarness::new();
        harness.feed(b"\x1b]8;;");
        // Feed a follow-up character outside an OSC frame to confirm
        // attachment did not bleed past the truncated open.
        harness.feed(b"\x1b\\Z");
        let grid = harness.term().grid();
        assert!(
            grid[Line(0)][Column(0)].hyperlink().is_none(),
            "truncated OSC 8 open must not attach a hyperlink"
        );
    }
}

/// Alt-screen hyperlinks must not bleed back to the primary screen.
#[test]
fn osc8_alt_screen_toggle_clears() {
    let mut harness = SpecHarness::new();
    // Seed primary with plain text — no hyperlink.
    harness.feed(b"primary");

    // Enter alt screen, attach a hyperlink, write text, leave alt.
    harness.term_mut().swap_alt();
    harness.feed(b"\x1b]8;;https://alt.example\x1b\\ALT\x1b]8;;\x1b\\");
    harness.term_mut().swap_alt();

    // Primary cells must not carry the alt-screen URI on any column.
    let grid = harness.term().grid();
    for col in 0..7 {
        let cell = &grid[Line(0)][Column(col)];
        assert!(
            cell.hyperlink().is_none(),
            "primary column {col} must not carry alt-screen URI"
        );
    }
}

/// Property: the renderable observer is no longer the §10.0 stub.
///
/// Asserts that an intentionally-wrong expected URI causes the
/// renderable rung to fail. If the observer regresses to
/// `RungResult::pass` regardless of input, this test silently passes
/// where it should fail — fail it explicitly so the regression is
/// loud.
#[test]
fn osc8_renderable_observer_not_stub() {
    let scenario = SpecScenario {
        catalog_row_id: "OSC-8",
        bytes: b"\x1b]8;;http://example.com\x1b\\X\x1b]8;;\x1b\\",
        apex_layer: ApexLayer::Renderable,
        setup: b"",
        expectations: ScenarioExpectations {
            renderable: Some(RenderableExpectation {
                hyperlink_at: Some((0, 0, "WRONG_URI")),
                ..RenderableExpectation::default()
            }),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);
    let renderable = results
        .iter()
        .find(|r| {
            matches!(
                r.rung_name,
                oriterm_test_support::spec_chain::RungName::Renderable
            )
        })
        .expect("renderable rung must execute for ApexLayer::Renderable");
    assert!(
        !renderable.passed,
        "renderable rung must reject WRONG_URI vs http://example.com — \
         observer is regressing to a stub"
    );
}
