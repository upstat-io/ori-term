//! OSC 104/110/111/112 color reset spec_chain coverage (Section 10.6).
//!
//! Drives OSC 104 (reset palette entries), OSC 110 (reset default
//! foreground), OSC 111 (reset default background), and OSC 112 (reset
//! text cursor color) through the high-level VTE `Processor` path and
//! asserts `Term::palette` reverts to the factory defaults captured at
//! `Palette::for_theme(Theme::default())` construction time.
//!
//! Dispatch: `crates/vte/src/ansi/dispatch/osc.rs` `b"104"` / `b"110"` /
//! `b"111"` / `b"112"` arms → `Handler::reset_color(index)` →
//! `Term::osc_reset_color` (`oriterm_core/src/term/handler/osc.rs`) →
//! `Palette::reset_indexed` + full-line damage when the index is not the
//! cursor slot.
//!
//! OSC 104 (zero args) resets indices 0..256; OSC 104 with explicit
//! indices resets only those; OSC 110/111/112 reset the named slots
//! `NamedColor::Foreground` (256), `Background` (257), `Cursor` (258).
//! The query round-trip pin (`osc_reset_round_trip_with_query`) asserts
//! OSC 10/11/12 `;?` emit `Effect::HostRequest(HostRequest::ColorQuery
//! {..})` — the spec_chain boundary. Reply formatting is covered by
//! `oriterm_mux` IO-thread response-fulfillment tests.
//!
//! Catalog rows: OSC-104, OSC-110, OSC-111, OSC-112.
//! Apex: state-snapshot.

use oriterm_core::Theme;
use oriterm_core::color::palette::Palette;
use oriterm_core::effect::{Effect, PtyEffect};
use oriterm_test_support::spec_chain::SpecHarness;
use vte::ansi::{Color, NamedColor, Rgb};

/// Factory-default palette the harness starts from. OSC 104/110/111/112
/// reset targets come from this snapshot — never from whatever the
/// current live palette happens to be after prior mutation.
fn theme_default_palette() -> Palette {
    Palette::for_theme(Theme::default())
}

/// Assert exactly one OSC 10/11/12 color reply (sync `PtyEffect::Write`)
/// is on the transcript and return its `(prefix, slot)`. Panics
/// otherwise.
fn expect_single_color_reply(harness: &SpecHarness) -> (String, usize) {
    let mut found = None;
    for eff in &harness.outcome().effects_emitted {
        if let Effect::Pty(PtyEffect::Write { bytes, .. }) = eff {
            if let Some(stripped) = bytes.strip_prefix(b"\x1b]") {
                let osc_end = stripped.iter().position(|&b| b == b';').unwrap_or(stripped.len());
                let osc_num = std::str::from_utf8(&stripped[..osc_end]).unwrap_or("");
                let slot = match osc_num {
                    "10" => 256,
                    "11" => 257,
                    "12" => 258,
                    _ => continue,
                };
                assert!(
                    found.is_none(),
                    "more than one OSC color reply emitted; got {:?}",
                    harness.outcome().effects_emitted
                );
                found = Some((osc_num.to_string(), slot));
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "expected one OSC color reply; got {:?}",
            harness.outcome().effects_emitted
        )
    })
}

// ── OSC 104 (reset palette entries) ─────────────────────────────────

/// Zero-arg `OSC 104 ST` resets every index in 0..256 back to the
/// factory-default palette built from `Theme::default()`. Set indices
/// 0..256 to pure white first so a failure to reset is audible — a
/// regression where only some indices are restored fails with an
/// index-specific assertion message.
#[test]
fn osc104_zero_args_resets_all_256_palette() {
    let mut harness = SpecHarness::new();

    // Pre-populate indices 0..=255 to a non-default color via OSC 4.
    for index in 0_u16..256 {
        let osc4 = format!("\x1b]4;{index};rgb:ff/ff/ff\x1b\\");
        harness.feed(osc4.as_bytes());
    }

    // Reset-all with zero params.
    harness.feed(b"\x1b]104\x1b\\");

    let defaults = theme_default_palette();
    for index in 0_u8..=255 {
        let live = harness.term().palette().resolve(Color::Indexed(index));
        let expected = defaults.resolve(Color::Indexed(index));
        assert_eq!(
            live, expected,
            "palette[{index}] must match theme default after OSC 104 reset-all"
        );
    }
}

/// `OSC 104 ; 5 ; 10 ST` resets ONLY indices 5 and 10. Index 0's OSC 4
/// customization must survive; indices 1–4, 6–9, 11–255 stay at their
/// (unchanged) defaults.
#[test]
fn osc104_specific_indices_resets_only_those() {
    let mut harness = SpecHarness::new();

    // Customize three indices.
    harness.feed(b"\x1b]4;0;rgb:11/11/11\x1b\\");
    harness.feed(b"\x1b]4;5;rgb:22/22/22\x1b\\");
    harness.feed(b"\x1b]4;10;rgb:33/33/33\x1b\\");

    let custom_0 = Rgb {
        r: 0x11,
        g: 0x11,
        b: 0x11,
    };
    assert_eq!(
        harness.term().palette().resolve(Color::Indexed(0)),
        custom_0,
        "precondition: index 0 must hold the OSC 4 override"
    );

    // Reset indices 5 and 10 only.
    harness.feed(b"\x1b]104;5;10\x1b\\");

    let defaults = theme_default_palette();

    assert_eq!(
        harness.term().palette().resolve(Color::Indexed(0)),
        custom_0,
        "index 0 must keep its OSC 4 override — not named in the reset list"
    );
    assert_eq!(
        harness.term().palette().resolve(Color::Indexed(5)),
        defaults.resolve(Color::Indexed(5)),
        "index 5 must be reset to the theme default"
    );
    assert_eq!(
        harness.term().palette().resolve(Color::Indexed(10)),
        defaults.resolve(Color::Indexed(10)),
        "index 10 must be reset to the theme default"
    );

    // Spot-check collateral: indices 1..5, 6..10, 11..=255 were never
    // mutated, so they must still equal the theme default.
    for index in (1_u8..5).chain(6_u8..10).chain(11_u8..=255) {
        assert_eq!(
            harness.term().palette().resolve(Color::Indexed(index)),
            defaults.resolve(Color::Indexed(index)),
            "index {index} must stay at the theme default (no collateral reset)"
        );
    }
}

/// Regression guard: `OSC 104 ; 999 ; abc ST` contains one out-of-range
/// numeric index and one non-numeric token. `parse_number` returns
/// `None` for `abc`, routing to `unhandled` without mutating any
/// palette entry. Index 999 is beyond `NUM_COLORS` (270) so
/// `Palette::reset_indexed` silently ignores it via the bounds check.
#[test]
fn osc104_invalid_index_dropped() {
    let mut harness = SpecHarness::new();

    // Seed a customization we can watch for accidental mutation.
    harness.feed(b"\x1b]4;7;rgb:ab/cd/ef\x1b\\");
    let custom_7 = Rgb {
        r: 0xab,
        g: 0xcd,
        b: 0xef,
    };
    assert_eq!(
        harness.term().palette().resolve(Color::Indexed(7)),
        custom_7
    );

    // Snapshot every index so we can verify nothing changed.
    let before: Vec<Rgb> = (0_u8..=255)
        .map(|i| harness.term().palette().resolve(Color::Indexed(i)))
        .collect();

    harness.feed(b"\x1b]104;999;abc\x1b\\");

    assert_eq!(
        harness.term().palette().resolve(Color::Indexed(7)),
        custom_7,
        "index 7's OSC 4 override must survive an invalid OSC 104"
    );

    for (index, expected) in before.iter().enumerate() {
        let idx = u8::try_from(index).expect("before loop covers 0..=255 only");
        assert_eq!(
            harness.term().palette().resolve(Color::Indexed(idx)),
            *expected,
            "index {idx} must not mutate when OSC 104 receives invalid args"
        );
    }
}

// ── OSC 110 / 111 / 112 (reset named color slots) ───────────────────

/// `OSC 110 ST` resets `NamedColor::Foreground` (index 256) back to the
/// theme default. Set the foreground to red first via OSC 10 so the
/// reset is observable.
#[test]
fn osc110_resets_default_foreground() {
    let mut harness = SpecHarness::new();

    harness.feed(b"\x1b]10;rgb:ff/00/00\x1b\\");
    assert_eq!(
        harness.term().palette().foreground(),
        Rgb {
            r: 0xff,
            g: 0x00,
            b: 0x00
        },
        "precondition: OSC 10 must have set the foreground to red"
    );

    harness.feed(b"\x1b]110\x1b\\");

    assert_eq!(
        harness.term().palette().foreground(),
        theme_default_palette().foreground(),
        "OSC 110 must restore the theme-default foreground"
    );
}

/// `OSC 111 ST` resets `NamedColor::Background` (index 257) back to the
/// theme default.
#[test]
fn osc111_resets_default_background() {
    let mut harness = SpecHarness::new();

    harness.feed(b"\x1b]11;rgb:00/ff/00\x1b\\");
    assert_eq!(
        harness.term().palette().background(),
        Rgb {
            r: 0x00,
            g: 0xff,
            b: 0x00
        },
        "precondition: OSC 11 must have set the background to green"
    );

    harness.feed(b"\x1b]111\x1b\\");

    assert_eq!(
        harness.term().palette().background(),
        theme_default_palette().background(),
        "OSC 111 must restore the theme-default background"
    );
}

/// `OSC 112 ST` resets `NamedColor::Cursor` (index 258) back to the
/// theme default.
#[test]
fn osc112_resets_cursor_color() {
    let mut harness = SpecHarness::new();

    harness.feed(b"\x1b]12;rgb:00/00/ff\x1b\\");
    assert_eq!(
        harness.term().palette().cursor_color(),
        Rgb {
            r: 0x00,
            g: 0x00,
            b: 0xff
        },
        "precondition: OSC 12 must have set the cursor color to blue"
    );

    harness.feed(b"\x1b]112\x1b\\");

    assert_eq!(
        harness.term().palette().cursor_color(),
        theme_default_palette().cursor_color(),
        "OSC 112 must restore the theme-default cursor color"
    );
}

// ── Round-trip with query ───────────────────────────────────────────

/// After each of OSC 110/111/112, feeding the matching `OSC 10/11/12 ;
/// ? ST` query must emit `Effect::HostRequest(HostRequest::ColorQuery
/// {..})`. Apex is the HostRequest itself — reply formatting
/// (`format_color_reply`) is verified in `oriterm_mux` IO-thread
/// response-fulfillment tests, NOT here.
#[test]
fn osc_reset_round_trip_with_query() {
    let cases: &[(&[u8], &[u8], &str, usize)] = &[
        (
            b"\x1b]110\x1b\\",
            b"\x1b]10;?\x1b\\",
            "10",
            NamedColor::Foreground as usize,
        ),
        (
            b"\x1b]111\x1b\\",
            b"\x1b]11;?\x1b\\",
            "11",
            NamedColor::Background as usize,
        ),
        (
            b"\x1b]112\x1b\\",
            b"\x1b]12;?\x1b\\",
            "12",
            NamedColor::Cursor as usize,
        ),
    ];

    let mut count = 0;
    for &(reset_bytes, query_bytes, expected_prefix, expected_index) in cases {
        let mut harness = SpecHarness::new();

        // Reset first — we want to verify the query fires AFTER reset,
        // not that the reset itself emits a ColorQuery.
        harness.feed(reset_bytes);
        assert!(
            harness
                .outcome()
                .effects_emitted
                .iter()
                .all(|e| !matches!(e, Effect::Pty(PtyEffect::Write { bytes, .. }) if bytes.starts_with(b"\x1b]10") || bytes.starts_with(b"\x1b]11") || bytes.starts_with(b"\x1b]12"))),
            "reset alone must not emit a ColorQuery: {:?}",
            harness.outcome().effects_emitted
        );

        harness.feed(query_bytes);

        let (prefix, index) = expect_single_color_reply(&harness);
        assert_eq!(prefix, expected_prefix);
        assert_eq!(index, expected_index);
        count += 1;
    }

    assert_eq!(
        count,
        cases.len(),
        "matrix must visit every reset/query pairing"
    );
}

// ── Property: OSC 104 marks the grid dirty ──────────────────────

/// OSC 104 (non-cursor index) must mark every visible row dirty so a
/// damage-aware renderer repaints the reset palette. `Term::
/// osc_reset_color` calls `grid.dirty_mut().mark_all()` when the
/// index is not `NamedColor::Cursor` — the pin asserts the
/// `all_dirty` state specifically, not merely "some line dirty". A
/// regression that downgrades `mark_all()` to `mark(cursor_line)`
/// would still leave `is_any_dirty() == true`; only `is_all_dirty()`
/// catches that narrowing and forces the renderer to repaint every
/// cell that references the reset palette entry.
#[test]
fn osc104_reset_marks_grid_dirty() {
    let mut harness = SpecHarness::new();

    // Seed a palette override to give OSC 104 something to reset.
    harness.feed(b"\x1b]4;3;rgb:ff/00/00\x1b\\");

    // Drain any prior damage so the next assertion only sees damage
    // introduced by the OSC 104 reset itself.
    harness
        .term_mut()
        .grid_mut()
        .dirty_mut()
        .drain()
        .for_each(drop);
    assert!(
        !harness.term().grid().dirty().is_any_dirty(),
        "precondition: dirty tracker must be drained before firing OSC 104"
    );

    harness.feed(b"\x1b]104;3\x1b\\");

    assert!(
        harness.term().grid().dirty().is_all_dirty(),
        "OSC 104 must call mark_all() on the dirty tracker (not a narrower \
         mark(line)) so every cell referencing the reset palette repaints"
    );
}
