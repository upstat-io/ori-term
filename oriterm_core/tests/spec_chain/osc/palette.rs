//! OSC 4 palette set / query spec_chain coverage (Section 10.8).
//!
//! Drives the high-level VTE `Processor` path for the OSC 4 palette-index
//! set + query arms:
//!
//! - `OSC 4 ; Ps ; spec BEL|ST` — set palette index `Ps` to `spec`.
//! - `OSC 4 ; Ps ; ? BEL|ST` — query index `Ps`; emits
//! `Effect::HostRequest(HostRequest::ColorQuery { prefix: "4;Ps", index: Ps,.. })`.
//! - Multi-param chunks walk `(index, spec)` pairs in pairs from
//! `params[1..]`.
//!
//! Dispatch: `crates/vte/src/ansi/dispatch/osc.rs` `b"4"` arm → `xparse_color`
//! → `Handler::set_color` OR `Handler::dynamic_color_sequence` (for `?`).
//! State apex is `Term::palette().color(index)` / `Palette::color`
//! at `oriterm_core/src/color/palette/mod.rs:282`.
//!
//! Reply formatting for the query path (`OSC 4 ; Ps ; rgb:… ST`) is
//! produced by the mux consumer — spec_chain stops at `HostRequest`
//! emission per the §10 scope boundary.
//!
//! Catalog rows: OSC-4-SET, OSC-4-QUERY. Apex: state-snapshot / effect-pty-write.

use oriterm_core::Theme;
use oriterm_core::color::palette::Palette;
use oriterm_core::effect::{Effect, PtyEffect};
use oriterm_test_support::spec_chain::SpecHarness;
use vte::ansi::{Color, Rgb};

/// Factory-default palette the harness starts from. Used for spot-checks
/// that an invalid OSC 4 doesn't mutate unrelated indices.
fn theme_default_palette() -> Palette {
    Palette::for_theme(Theme::default())
}

/// Assert exactly one OSC color reply (`Effect::Pty(PtyEffect::Write { .. })`
/// with bytes starting `\x1b]N;` for the given OSC number) is on the
/// transcript and return its `(osc_number, index_or_none)`. Panics
/// otherwise. Palette index for OSC 4 is parsed from the reply bytes
/// (`\x1b]4;<idx>;rgb:...`); OSC 10/11/12 return `None` for index.
fn expect_single_color_reply(harness: &SpecHarness) -> (String, usize) {
    let mut found = None;
    for eff in &harness.outcome().effects_emitted {
        if let Effect::Pty(PtyEffect::Write { bytes, .. }) = eff {
            // OSC reply bytes start with `\x1b]<N>;...` — parse the N
            // and (for OSC 4) the index. We treat the prefix (e.g.
            // `4;5`) the same way the legacy `ColorQuery` payload did.
            if let Some(stripped) = bytes.strip_prefix(b"\x1b]") {
                let after_st = stripped
                    .iter()
                    .position(|&b| b == b';')
                    .map(|p| (&stripped[..p], &stripped[p + 1..]))
                    .unwrap_or((stripped, &[][..]));
                let osc_num_bytes = after_st.0;
                let tail = after_st.1;
                let osc_num = std::str::from_utf8(osc_num_bytes).unwrap_or("");
                // Skip non-color-reply OSC writes (titles, working-dir, etc.).
                if !matches!(osc_num, "4" | "10" | "11" | "12") {
                    continue;
                }
                assert!(
                    found.is_none(),
                    "more than one OSC color reply emitted; got {:?}",
                    harness.outcome().effects_emitted
                );
                let (prefix, index) = if osc_num == "4" {
                    // OSC 4 reply: `\x1b]4;<idx>;rgb:...`
                    let idx_end = tail.iter().position(|&b| b == b';').unwrap_or(tail.len());
                    let idx_str = std::str::from_utf8(&tail[..idx_end]).unwrap_or("0");
                    let idx: usize = idx_str.parse().unwrap_or(0);
                    (format!("{osc_num};{idx}"), idx)
                } else {
                    let idx = match osc_num {
                        "10" => 256,
                        "11" => 257,
                        "12" => 258,
                        _ => 0,
                    };
                    (osc_num.to_string(), idx)
                };
                found = Some((prefix, index));
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

/// `OSC 4 ; 5 ; rgb:ff/00/00 ST` writes pure red to palette index 5.
#[test]
fn osc4_set_palette_index() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]4;5;rgb:ff/00/00\x1b\\");

    assert_eq!(
        harness.term().palette().color(5),
        Rgb {
            r: 0xff,
            g: 0x00,
            b: 0x00
        },
        "OSC 4 must mutate `Palette::color(index)` via `Term::set_color`"
    );
}

/// `OSC 4 ; 5 ; ? ST` emits `Effect::HostRequest(HostRequest::ColorQuery
/// { prefix: "4;5", index: 5,.. })`. The dispatcher builds the prefix
/// via `format!("4;{index}")` at `crates/vte/src/ansi/dispatch/osc.rs:108`
/// so BOTH the OSC number and palette index survive into the reply
/// formatter. Reply bytes are produced by the consumer (out of spec_chain
/// scope).
#[test]
fn osc4_query_palette_index() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]4;5;?\x1b\\");

    let (prefix, index) = expect_single_color_reply(&harness);
    assert_eq!(prefix, "4;5");
    assert_eq!(index, 5);
}

/// `OSC 4 ; 1 ; rgb:00/ff/00 ; 2 ; rgb:00/00/ff ST` walks the
/// `params[1..]` slice in `(index, spec)` pairs and sets indices 1 and
/// 2 to green and blue respectively.
#[test]
fn osc4_multi_param_sets_multiple_indices() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]4;1;rgb:00/ff/00;2;rgb:00/00/ff\x1b\\");

    assert_eq!(
        harness.term().palette().color(1),
        Rgb {
            r: 0x00,
            g: 0xff,
            b: 0x00
        },
        "multi-param chunk 0 must set index 1 to green"
    );
    assert_eq!(
        harness.term().palette().color(2),
        Rgb {
            r: 0x00,
            g: 0x00,
            b: 0xff
        },
        "multi-param chunk 1 must set index 2 to blue"
    );
}

/// Regression guard: `OSC 4 ; 999 ; rgb:ff/ff/ff ST` names an out-of-range
/// index. `Palette::set` has an internal `index < NUM_COLORS` bounds
/// check (`oriterm_core/src/color/palette/mod.rs:239-243`) so the
/// mutation is silently dropped. No assertion panics; the call is a
/// no-op. Verify by scanning every in-range index against the factory
/// default palette — no collateral mutation is permitted.
#[test]
fn osc4_out_of_range_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]4;999;rgb:ff/ff/ff\x1b\\");

    let defaults = theme_default_palette();
    for index in 0_u8..=255 {
        let live = harness.term().palette().resolve(Color::Indexed(index));
        let expected = defaults.resolve(Color::Indexed(index));
        assert_eq!(
            live, expected,
            "index {index} must not mutate when OSC 4 names an out-of-range index"
        );
    }
}

/// Regression guard: `OSC 4 ; 5 ; NOT_A_COLOR ST` fails the `xparse_color`
/// branch in the dispatch arm — the chunk routes to `unhandled` without
/// mutating index 5. Verify index 5 still holds its theme-default value.
#[test]
fn osc4_invalid_color_dropped() {
    let mut harness = SpecHarness::new();

    let before = harness.term().palette().resolve(Color::Indexed(5));
    harness.feed(b"\x1b]4;5;NOT_A_COLOR\x1b\\");

    assert_eq!(
        harness.term().palette().resolve(Color::Indexed(5)),
        before,
        "OSC 4 with unparseable color spec must leave the palette entry untouched"
    );
}
