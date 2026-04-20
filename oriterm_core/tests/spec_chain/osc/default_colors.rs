//! OSC 10 / 11 / 12 default foreground / background / cursor color
//! spec_chain coverage (Section 10.8).
//!
//! Drives the high-level VTE `Processor` path for the OSC 10/11/12
//! set + query arms:
//!
//! - `OSC 10 ; spec ST` → sets `NamedColor::Foreground` (256) via
//!   `Term::set_color`. Multi-param form walks `NamedColor::Foreground..=Cursor`.
//! - `OSC 11 ; spec ST` → sets `NamedColor::Background` (257).
//! - `OSC 12 ; spec ST` → sets `NamedColor::Cursor` (258).
//! - `OSC N ; ? ST` (N ∈ {10,11,12}) → emits `Effect::HostRequest(
//!   HostRequest::ColorQuery { prefix, index, .. })` where `prefix` is
//!   the bare OSC number ("10", "11", "12") — reply bytes are produced
//!   by the mux consumer (out of spec_chain scope).
//!
//! Dispatch: `crates/vte/src/ansi/dispatch/osc.rs` `b"10" | b"11" | b"12"`
//! arm at line 146. The per-param offset math at lines 151–152
//! (`offset = dynamic_code - 10; index = NamedColor::Foreground + offset`)
//! lets a single `OSC 10 ; a ; b ; c ST` payload walk the three named
//! slots in one dispatch.
//!
//! Catalog rows: OSC-10-SET, OSC-10-QUERY, OSC-11-SET, OSC-11-QUERY,
//! OSC-12-SET, OSC-12-QUERY. Apex: state-snapshot / effect-pty-write.

use oriterm_core::effect::{Effect, HostRequest};
use oriterm_test_support::spec_chain::SpecHarness;
use vte::ansi::{NamedColor, Rgb};

/// Assert exactly one `HostRequest::ColorQuery` is on the transcript and
/// return its `(prefix, index)`. Panics otherwise.
fn expect_single_color_query(harness: &SpecHarness) -> (String, usize) {
    let mut found = None;
    for eff in &harness.outcome().effects_emitted {
        if let Effect::HostRequest(HostRequest::ColorQuery { prefix, index, .. }) = eff {
            assert!(
                found.is_none(),
                "more than one ColorQuery emitted; got {:?}",
                harness.outcome().effects_emitted
            );
            found = Some((prefix.clone(), *index));
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "expected one ColorQuery; got {:?}",
            harness.outcome().effects_emitted
        )
    })
}

/// `OSC 10 ; rgb:de/ad/be ST` writes the default foreground via
/// `Palette::foreground()` (the SSOT accessor on the live palette).
#[test]
fn osc10_sets_default_foreground() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]10;rgb:de/ad/be\x1b\\");

    assert_eq!(
        harness.term().palette().foreground(),
        Rgb {
            r: 0xde,
            g: 0xad,
            b: 0xbe
        }
    );
}

/// `OSC 11 ; rgb:ca/fe/ba ST` writes the default background via
/// `Palette::background()`.
#[test]
fn osc11_sets_default_background() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]11;rgb:ca/fe/ba\x1b\\");

    assert_eq!(
        harness.term().palette().background(),
        Rgb {
            r: 0xca,
            g: 0xfe,
            b: 0xba
        }
    );
}

/// `OSC 12 ; rgb:12/34/56 ST` writes the cursor color via
/// `Palette::cursor_color()`.
#[test]
fn osc12_sets_cursor_color() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]12;rgb:12/34/56\x1b\\");

    assert_eq!(
        harness.term().palette().cursor_color(),
        Rgb {
            r: 0x12,
            g: 0x34,
            b: 0x56
        }
    );
}

/// `OSC 10 ; ? ST` emits `Effect::HostRequest(HostRequest::ColorQuery {
/// prefix: "10", index: 256, .. })`. The dispatcher stringifies the bare
/// OSC number as the prefix (`dynamic_code.to_string()`) and maps the
/// OSC number to `NamedColor::Foreground as usize` (256) via
/// `offset = dynamic_code - 10` + `NamedColor::Foreground + offset`.
#[test]
fn osc10_query_replies_rgb() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]10;?\x1b\\");

    let (prefix, index) = expect_single_color_query(&harness);
    assert_eq!(prefix, "10");
    assert_eq!(index, NamedColor::Foreground as usize);
}

/// Multi-param form: `OSC 10 ; spec1 ; spec2 ; spec3 ST` walks
/// `NamedColor::Foreground..=Cursor` in order. Feed three distinct
/// color specs and assert all three named slots land.
///
/// This pins the `dynamic_code += 1` increment at
/// `crates/vte/src/ansi/dispatch/osc.rs:171` — without it, the multi-param
/// form would re-apply the first color to every named slot.
#[test]
fn osc10_multi_param_walks_named_colors() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]10;rgb:10/10/10;rgb:20/20/20;rgb:30/30/30\x1b\\");

    assert_eq!(
        harness.term().palette().foreground(),
        Rgb {
            r: 0x10,
            g: 0x10,
            b: 0x10
        },
        "multi-param chunk 0 must set Foreground (offset 0)"
    );
    assert_eq!(
        harness.term().palette().background(),
        Rgb {
            r: 0x20,
            g: 0x20,
            b: 0x20
        },
        "multi-param chunk 1 must set Background (offset 1)"
    );
    assert_eq!(
        harness.term().palette().cursor_color(),
        Rgb {
            r: 0x30,
            g: 0x30,
            b: 0x30
        },
        "multi-param chunk 2 must set Cursor (offset 2)"
    );
}
