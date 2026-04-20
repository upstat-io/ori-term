//! OSC 0/1/2 title + icon-name spec_chain coverage (Section 10.8).
//!
//! Drives the high-level VTE `Processor` path for the three title/icon
//! shared-arm OSCs:
//!
//! - `OSC 0 ; Pt BEL|ST` → sets BOTH window title and icon name to `Pt`.
//! - `OSC 1 ; Pt BEL|ST` → sets ONLY icon name.
//! - `OSC 2 ; Pt BEL|ST` → sets ONLY window title.
//!
//! Dispatch: `crates/vte/src/ansi/dispatch/osc.rs` `b"0" | b"1" | b"2"` arm
//! → `Handler::set_title` / `Handler::set_icon_name` → `Term::title` +
//! `Term::icon_name` fields (`oriterm_core/src/term/mod.rs:399-406`).
//!
//! Dispatch accuracy note: the shared-arm routes `Some(text.clone())` to
//! the handler regardless of payload length — including the empty-string
//! case (`OSC 0 ; ST`). There is NO `ResetTitle` path triggered by an
//! empty-payload OSC 0; the handler receives `Some("")`, not `None`.
//! `HostEffect::TitleSet { value: None }` is reserved for the explicit
//! reset paths (ESC c, title-stack eviction, etc.), NOT OSC 0 with an
//! empty `Pt`.
//!
//! Catalog rows: OSC-0, OSC-1, OSC-2.
//! Apex: effect-host-title (via the state surface on `Term`).

use oriterm_test_support::spec_chain::SpecHarness;

/// `OSC 0 ; myapp ST` sets BOTH `Term::title` and `Term::icon_name` to
/// the same string — the shared-arm routes `text.clone()` into both
/// handler slots.
#[test]
fn osc0_sets_title_and_icon() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]0;myapp\x1b\\");

    assert_eq!(harness.term().title(), "myapp");
    assert_eq!(harness.term().icon_name(), "myapp");
}

/// `OSC 1 ; myicon ST` sets ONLY `Term::icon_name`. `Term::title` stays
/// at its constructor default (empty string) — a regression that writes
/// both fields would widen the dispatch arm incorrectly.
#[test]
fn osc1_sets_only_icon_name() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]1;myicon\x1b\\");

    assert_eq!(harness.term().icon_name(), "myicon");
    assert_eq!(
        harness.term().title(),
        "",
        "OSC 1 must NOT touch the title field — regression would collapse OSC 1 into OSC 0"
    );
}

/// `OSC 2 ; mytitle ST` sets ONLY `Term::title`. `Term::icon_name` stays
/// at its constructor default.
#[test]
fn osc2_sets_only_title() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]2;mytitle\x1b\\");

    assert_eq!(harness.term().title(), "mytitle");
    assert_eq!(
        harness.term().icon_name(),
        "",
        "OSC 2 must NOT touch the icon name — regression would collapse OSC 2 into OSC 0"
    );
}

/// Negative pin: `OSC 0 ; ST` (empty `Pt`) sets BOTH slots to the empty
/// string. The dispatcher always passes `Some(text)` to the handler —
/// `Some("")` for an empty payload — so the fields observably become
/// `""`, NOT `None` or the prior value. A regression that special-cased
/// empty payloads to route through `ResetTitle` / `TitleSet { value:
/// None }` would change observable behavior.
#[test]
fn osc0_empty_sets_empty_string() {
    let mut harness = SpecHarness::new();
    // Seed a non-empty title first so the assertion distinguishes "empty
    // string was stored" from "initial state was already empty".
    harness.feed(b"\x1b]0;seeded\x1b\\");
    assert_eq!(harness.term().title(), "seeded");

    harness.feed(b"\x1b]0;\x1b\\");

    assert_eq!(
        harness.term().title(),
        "",
        "OSC 0 with empty payload must store the empty string, not preserve the prior title"
    );
    assert_eq!(
        harness.term().icon_name(),
        "",
        "OSC 0 with empty payload must store the empty string, not preserve the prior icon name"
    );
}

/// Matrix pin: OSC 0 accepts both terminator forms. BEL (0x07) and ST
/// (ESC backslash, 0x1b 0x5c) must both drive the title through the
/// shared dispatch arm. The `bell_terminated` parameter only affects
/// the terminator echoed back in query-response paths; for
/// fire-and-forget setters both forms must reach the handler.
#[test]
fn osc0_bel_and_st_terminators_both_accepted() {
    // BEL terminator.
    {
        let mut harness = SpecHarness::new();
        harness.feed(b"\x1b]0;t1\x07");
        assert_eq!(harness.term().title(), "t1");
        assert_eq!(harness.term().icon_name(), "t1");
    }

    // ST terminator.
    {
        let mut harness = SpecHarness::new();
        harness.feed(b"\x1b]0;t2\x1b\\");
        assert_eq!(harness.term().title(), "t2");
        assert_eq!(harness.term().icon_name(), "t2");
    }
}
