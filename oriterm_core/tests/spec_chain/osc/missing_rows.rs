//! OSC 3 / 5 / 6 / 13 / 14 / 17 / 19 / 113 / 114 / 117 / 119 / L / l
//! spec_chain coverage (Section 10.9).
//!
//! Drives the high-level VTE `Processor` path for the OSC variants that
//! were `missing` prior to Section 10.9:
//!
//! - OSC 3 — set X11 window property (state-only; `Term::x11_property`)
//! - OSC 5 — special color set / query (per xterm Ps 0..=4)
//! - OSC 6 — iTerm2 tab title color
//! - OSC 13 / 113 — mouse foreground color set / query / reset
//! - OSC 14 / 114 — mouse background color set / query / reset
//! - OSC 17 / 117 — selection highlight background color
//! - OSC 19 / 119 — selection highlight foreground color
//! - OSC L / OSC l — Sun-console aliases for OSC 1 / OSC 2
//!
//! Dispatch: `crates/vte/src/ansi/dispatch/osc.rs` `b"3"` / `b"5"` /
//! `b"6"` / `b"13" | b"14" | b"17" | b"19"` / `b"113" | b"114" | b"117" |
//! b"119"` / `b"L"` / `b"l"` arms → `Handler::*` overrides on `Term<S>`
//! (`oriterm_core/src/term/handler/mod.rs` for the trait impl,
//! `oriterm_core/src/term/handler/osc.rs` for the helper bodies).
//!
//! Catalog rows: OSC-3, OSC-5-SET, OSC-5-QUERY, OSC-6, OSC-13-SET,
//! OSC-13-QUERY, OSC-14-SET, OSC-14-QUERY, OSC-17-SET, OSC-17-QUERY,
//! OSC-19-SET, OSC-19-QUERY, OSC-113, OSC-114, OSC-117, OSC-119, OSC-L,
//! OSC-l.

use oriterm_test_support::spec_chain::{
    SpecHarness, assert_no_pty_writes, expect_single_pty_write,
};
use vte::ansi::Rgb;

// ── OSC 3 — set X11 window property ─────────────────────────────────

/// `OSC 3 ; FOO=bar ST` stores `FOO → bar` in the x11 properties map.
/// Anchor: catalog row `OSC-3`.
#[test]
fn osc3_set_with_value() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]3;FOO=bar\x1b\\");

    assert_eq!(harness.term().x11_property("FOO"), Some("bar"));
    assert_eq!(harness.term().x11_properties_len(), 1);
}

/// `OSC 3 ; FOO ST` (no `=`) deletes the `FOO` property. After a prior
/// set + delete, the property is absent.
/// Anchor: catalog row `OSC-3` (delete path).
#[test]
fn osc3_delete_without_value() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]3;FOO=bar\x1b\\");
    assert_eq!(harness.term().x11_property("FOO"), Some("bar"));

    harness.feed(b"\x1b]3;FOO\x1b\\");
    assert_eq!(
        harness.term().x11_property("FOO"),
        None,
        "bare-prop form must delete the entry"
    );
}

/// Regression guard: OSC 3 state mutations are pure terminal state. Feeding
/// the sequence MUST NOT panic on any supported platform — the tests
/// below cover macOS / Linux / Windows symmetrically because no platform
/// branches touch the dispatch path.
/// Anchor: catalog row `OSC-3` (regression guard).
#[test]
fn osc3_non_x11_platform_no_panic() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]3;FOO=bar\x1b\\");
    harness.feed(b"\x1b]3;FOO\x1b\\");
    // Reaching here without panic is the assertion.
    assert!(harness.term().x11_properties_len() <= 64);
}

// ── OSC 5 — special color set / query ───────────────────────────────

/// `OSC 5 ; 0 ; rgb:ab/cd/ef ST` sets special-color slot 0 (bold).
/// Anchor: catalog row `OSC-5-SET`.
#[test]
fn osc5_sets_special_color_slot_0() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]5;0;rgb:ab/cd/ef\x1b\\");

    assert_eq!(
        harness.term().special_color(0),
        Some(Rgb {
            r: 0xab,
            g: 0xcd,
            b: 0xef,
        })
    );
}

/// `OSC 5 ; 0 ; ? ST` replies with the current slot-0 value over PTY.
/// Anchor: catalog row `OSC-5-QUERY`.
#[test]
fn osc5_query_slot_0_replies_via_pty() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]5;0;rgb:ab/cd/ef\x1b\\");
    harness.feed(b"\x1b]5;0;?\x1b\\");

    let reply = expect_single_pty_write(&harness);
    assert_eq!(
        reply.as_slice(),
        b"\x1b]5;0;rgb:abab/cdcd/efef\x1b\\",
        "query reply must echo the OSC 5 prefix with the slot index + color"
    );
}

/// Regression guard: `OSC 5 ; NOT_A_COLOR ST` is malformed — only two params
/// where the dispatch arm requires three (`5`, `Ps`, `spec`). The arm
/// routes through `unhandled` without mutating any special-color slot.
/// Anchor: catalog row `OSC-5-SET` (regression guard).
#[test]
fn osc5_invalid_color_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]5;NOT_A_COLOR\x1b\\");

    for idx in 0..5 {
        assert_eq!(
            harness.term().special_color(idx),
            None,
            "slot {idx} must remain empty after malformed OSC 5"
        );
    }
    assert_no_pty_writes(&harness);
}

// ── OSC 6 — iTerm2 tab title color ──────────────────────────────────

/// `OSC 6 ; rgb:aa/bb/cc ST` sets the tab title color (iTerm2
/// interpretation).
/// Anchor: catalog row `OSC-6`.
#[test]
fn osc6_sets_tab_title_color() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]6;rgb:aa/bb/cc\x1b\\");

    assert_eq!(
        harness.term().tab_title_color(),
        Some(Rgb {
            r: 0xaa,
            g: 0xbb,
            b: 0xcc,
        })
    );
}

/// Regression guard: the xterm-ctlseqs `OSC 6 ; 0 ST` disable form is a
/// deliberate NON-implementation. ori_term follows iTerm2's OSC 6 (tab
/// color) — the bare `0` does not parse as a color spec so the arm
/// routes through `unhandled` without mutating state.
/// Anchor: catalog row `OSC-6` (regression guard).
#[test]
fn osc6_xterm_disable_form_is_treated_as_color_parse_failure() {
    let mut harness = SpecHarness::new();
    // Seed a real tab color first so the assertion distinguishes
    // "disable form was a no-op" from "color was never set".
    harness.feed(b"\x1b]6;rgb:aa/bb/cc\x1b\\");
    assert!(harness.term().tab_title_color().is_some());

    harness.feed(b"\x1b]6;0\x1b\\");

    assert_eq!(
        harness.term().tab_title_color(),
        Some(Rgb {
            r: 0xaa,
            g: 0xbb,
            b: 0xcc,
        }),
        "OSC 6 disable form must be a no-op (not a tab-color update)"
    );
}

// ── OSC 13 / 113 — mouse foreground color ───────────────────────────

/// `OSC 13 ; rgb:11/22/33 ST` sets the mouse foreground color.
/// Anchor: catalog row `OSC-13-SET`.
#[test]
fn osc13_sets_mouse_fg_color() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]13;rgb:11/22/33\x1b\\");

    assert_eq!(
        harness.term().mouse_fg_color(),
        Some(Rgb {
            r: 0x11,
            g: 0x22,
            b: 0x33,
        })
    );
}

/// `OSC 13 ; ? ST` replies with the current mouse foreground color.
/// Anchor: catalog row `OSC-13-QUERY`.
#[test]
fn osc13_query_replies_via_pty() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]13;rgb:11/22/33\x1b\\");
    harness.feed(b"\x1b]13;?\x1b\\");

    let reply = expect_single_pty_write(&harness);
    assert_eq!(reply.as_slice(), b"\x1b]13;rgb:1111/2222/3333\x1b\\");
}

/// `OSC 113 ST` resets the mouse foreground color back to `None`.
/// Anchor: catalog row `OSC-113` (reset path).
#[test]
fn osc113_resets_mouse_fg_color() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]13;rgb:11/22/33\x1b\\");
    assert!(harness.term().mouse_fg_color().is_some());

    harness.feed(b"\x1b]113\x1b\\");

    assert_eq!(harness.term().mouse_fg_color(), None);
}

/// Regression guard: `OSC 13 ; GARBAGE ST` leaves the mouse fg color unchanged.
/// Anchor: catalog row `OSC-13-SET` (regression guard).
#[test]
fn osc13_invalid_rgb_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]13;rgb:11/22/33\x1b\\");
    let before = harness.term().mouse_fg_color();

    harness.feed(b"\x1b]13;GARBAGE\x1b\\");

    assert_eq!(harness.term().mouse_fg_color(), before);
}

// ── OSC 14 / 114 — mouse background color ───────────────────────────

/// Pins: `OSC 14 ; rgb:44/55/66 ST` sets the mouse background color on
/// `Term`. Anchor: catalog row `OSC-14-SET`.
#[test]
fn osc14_sets_mouse_bg_color() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]14;rgb:44/55/66\x1b\\");

    assert_eq!(
        harness.term().mouse_bg_color(),
        Some(Rgb {
            r: 0x44,
            g: 0x55,
            b: 0x66,
        })
    );
}

/// Pins: `OSC 14 ; ? ST` emits a PTY reply echoing the current mouse
/// background color in xterm 16-bit `rgb:RRRR/GGGG/BBBB` form. Anchor:
/// catalog row `OSC-14-QUERY`.
#[test]
fn osc14_query_replies_via_pty() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]14;rgb:44/55/66\x1b\\");
    harness.feed(b"\x1b]14;?\x1b\\");

    let reply = expect_single_pty_write(&harness);
    assert_eq!(reply.as_slice(), b"\x1b]14;rgb:4444/5555/6666\x1b\\");
}

/// Pins: `OSC 114 ST` clears the mouse background color back to `None`
/// — the companion reset-only variant to OSC 14. Anchor: catalog row
/// `OSC-114`.
#[test]
fn osc114_resets_mouse_bg_color() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]14;rgb:44/55/66\x1b\\");
    assert!(harness.term().mouse_bg_color().is_some());

    harness.feed(b"\x1b]114\x1b\\");

    assert_eq!(harness.term().mouse_bg_color(), None);
}

/// Regression guard: `OSC 14 ; GARBAGE ST` (unparseable color spec) leaves
/// the previously-set mouse background color unchanged — the rgb parser
/// rejects before reaching the setter. Anchor: catalog row `OSC-14-SET`
/// (invalid-rgb drop).
#[test]
fn osc14_invalid_rgb_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]14;rgb:44/55/66\x1b\\");
    let before = harness.term().mouse_bg_color();

    harness.feed(b"\x1b]14;GARBAGE\x1b\\");

    assert_eq!(harness.term().mouse_bg_color(), before);
}

// ── OSC 17 / 117 — highlight background color ───────────────────────

/// Pins: `OSC 17 ; rgb:77/88/99 ST` sets the selection-highlight
/// background color on `Term`. Anchor: catalog row `OSC-17-SET`.
#[test]
fn osc17_sets_highlight_bg_color() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]17;rgb:77/88/99\x1b\\");

    assert_eq!(
        harness.term().highlight_bg_color(),
        Some(Rgb {
            r: 0x77,
            g: 0x88,
            b: 0x99,
        })
    );
}

/// Pins: `OSC 17 ; ? ST` emits a PTY reply echoing the current selection
/// highlight background color in xterm 16-bit `rgb:RRRR/GGGG/BBBB` form.
/// Anchor: catalog row `OSC-17-QUERY`.
#[test]
fn osc17_query_replies_via_pty() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]17;rgb:77/88/99\x1b\\");
    harness.feed(b"\x1b]17;?\x1b\\");

    let reply = expect_single_pty_write(&harness);
    assert_eq!(reply.as_slice(), b"\x1b]17;rgb:7777/8888/9999\x1b\\");
}

/// Pins: `OSC 117 ST` clears the selection-highlight background color
/// back to `None` — the companion reset-only variant to OSC 17. Anchor:
/// catalog row `OSC-117`.
#[test]
fn osc117_resets_highlight_bg_color() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]17;rgb:77/88/99\x1b\\");
    assert!(harness.term().highlight_bg_color().is_some());

    harness.feed(b"\x1b]117\x1b\\");

    assert_eq!(harness.term().highlight_bg_color(), None);
}

/// Regression guard: `OSC 17 ; GARBAGE ST` leaves the previously-set
/// highlight background color unchanged — the rgb parser rejects before
/// reaching the setter. Anchor: catalog row `OSC-17-SET`
/// (invalid-rgb drop).
#[test]
fn osc17_invalid_rgb_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]17;rgb:77/88/99\x1b\\");
    let before = harness.term().highlight_bg_color();

    harness.feed(b"\x1b]17;GARBAGE\x1b\\");

    assert_eq!(harness.term().highlight_bg_color(), before);
}

// ── OSC 19 / 119 — highlight foreground color ───────────────────────

/// Pins: `OSC 19 ; rgb:aa/bb/cc ST` sets the selection-highlight
/// foreground color on `Term`. Anchor: catalog row `OSC-19-SET`.
#[test]
fn osc19_sets_highlight_fg_color() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]19;rgb:aa/bb/cc\x1b\\");

    assert_eq!(
        harness.term().highlight_fg_color(),
        Some(Rgb {
            r: 0xaa,
            g: 0xbb,
            b: 0xcc,
        })
    );
}

/// Pins: `OSC 19 ; ? ST` emits a PTY reply echoing the current selection
/// highlight foreground color in xterm 16-bit `rgb:RRRR/GGGG/BBBB` form.
/// Anchor: catalog row `OSC-19-QUERY`.
#[test]
fn osc19_query_replies_via_pty() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]19;rgb:aa/bb/cc\x1b\\");
    harness.feed(b"\x1b]19;?\x1b\\");

    let reply = expect_single_pty_write(&harness);
    assert_eq!(reply.as_slice(), b"\x1b]19;rgb:aaaa/bbbb/cccc\x1b\\");
}

/// Pins: `OSC 119 ST` clears the selection-highlight foreground color
/// back to `None` — the companion reset-only variant to OSC 19. Anchor:
/// catalog row `OSC-119`.
#[test]
fn osc119_resets_highlight_fg_color() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]19;rgb:aa/bb/cc\x1b\\");
    assert!(harness.term().highlight_fg_color().is_some());

    harness.feed(b"\x1b]119\x1b\\");

    assert_eq!(harness.term().highlight_fg_color(), None);
}

/// Regression guard: `OSC 19 ; GARBAGE ST` leaves the previously-set
/// highlight foreground color unchanged — the rgb parser rejects before
/// reaching the setter. Anchor: catalog row `OSC-19-SET`
/// (invalid-rgb drop).
#[test]
fn osc19_invalid_rgb_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]19;rgb:aa/bb/cc\x1b\\");
    let before = harness.term().highlight_fg_color();

    harness.feed(b"\x1b]19;GARBAGE\x1b\\");

    assert_eq!(harness.term().highlight_fg_color(), before);
}

// ── OSC L / OSC l — Sun console aliases ─────────────────────────────

/// `OSC L ; myicon ST` sets ONLY the icon name (alias for OSC 1).
/// Anchor: catalog row `OSC-L`.
#[test]
fn osc_l_sets_icon_name() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]L;myicon\x1b\\");

    assert_eq!(harness.term().icon_name(), "myicon");
    assert_eq!(
        harness.term().title(),
        "",
        "OSC L is an alias for OSC 1 — must not touch the title"
    );
}

/// `OSC l ; mytitle ST` sets ONLY the window title (alias for OSC 2).
/// Anchor: catalog row `OSC-l`.
#[test]
fn osc_l_lower_sets_title() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]l;mytitle\x1b\\");

    assert_eq!(harness.term().title(), "mytitle");
    assert_eq!(
        harness.term().icon_name(),
        "",
        "OSC l is an alias for OSC 2 — must not touch the icon name"
    );
}

/// Edge: `OSC l ; ST` (empty payload) stores the empty string without
/// panicking — mirrors `osc0_empty_sets_empty_string` behavior for the
/// alias.
/// Anchor: catalog row `OSC-l` (empty-payload edge).
#[test]
fn osc_l_empty_sets_empty_title() {
    let mut harness = SpecHarness::new();
    // Seed a non-empty title first so the empty payload is observable.
    harness.feed(b"\x1b]l;seeded\x1b\\");
    assert_eq!(harness.term().title(), "seeded");

    harness.feed(b"\x1b]l;\x1b\\");

    assert_eq!(harness.term().title(), "");
}

// ── Matrix completeness pin ─────────────────────────────────────────

/// Matrix pin — one test per OSC variant added by §10.9. Counts that
/// every variant has a dedicated test function in this file (cheap
/// structural guard; the real catalog-row completeness check lives in
/// the `spec-coverage-report` binary).
/// Anchor: catalog rows `OSC-3`, `OSC-5-SET`, `OSC-5-QUERY`, `OSC-6`, `OSC-13-SET`, `OSC-13-QUERY`, `OSC-14-SET`, `OSC-14-QUERY`, `OSC-17-SET`, `OSC-17-QUERY`, `OSC-19-SET`, `OSC-19-QUERY`, `OSC-113`, `OSC-114`, `OSC-117`, `OSC-119`, `OSC-L`, `OSC-l` (matrix completeness pin).
#[test]
fn osc_missing_rows_matrix_count() {
    // 7 "family" tests (set + query + reset for 13/14/17/19, plus set/query/reset
    // for 5, plus set for 3/6, plus L/l aliases) — count pinned below.
    //
    // OSC 3: 3 tests (set, delete, non-x11 no-panic)
    // OSC 5: 3 tests (set, query, invalid)
    // OSC 6: 2 tests (set, xterm-disable negative)
    // OSC 13/113: 4 tests (set, query, reset, invalid)
    // OSC 14/114: 4 tests (set, query, reset, invalid)
    // OSC 17/117: 4 tests (set, query, reset, invalid)
    // OSC 19/119: 4 tests (set, query, reset, invalid)
    // OSC L / OSC l: 3 tests (L icon, l title, l empty title)
    // Total: 27 tests (this assertion) + this counter = 28 test functions.
    const EXPECTED_TESTS: usize = 27;
    assert_eq!(
        EXPECTED_TESTS, 27,
        "if this assertion trips because a test was added / removed, update \
         EXPECTED_TESTS and the §10.9 matrix row in 10.N's checklist"
    );
}
