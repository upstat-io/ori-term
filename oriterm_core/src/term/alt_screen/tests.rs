//! Alt-screen toggle tests.
//!
//! Covers `swap_alt`, `swap_alt_no_cursor`, `swap_alt_clear`, and the
//! keyboard-mode-stack swap + reapply behavior in `toggle_alt_common`.
//!
//! Existing tests migrated from `term/tests.rs` after the alt_screen
//! directory-module conversion (BUG-08-012). New toggle-reapply tests
//! pin the pre-existing DRIFT fix landed with BUG-08-012.

use std::collections::VecDeque;

use vte::ansi::{KeyboardModes, Processor};

use crate::effect::VoidEffectSink;
use crate::index::{Column, Line};
use crate::term::{Term, TermMode};
use crate::theme::Theme;

fn make_term() -> Term<VoidEffectSink> {
    Term::new(24, 80, 1000, Theme::default(), VoidEffectSink)
}

fn damage_term() -> Term<VoidEffectSink> {
    let mut t = Term::new(6, 10, 100, Theme::default(), VoidEffectSink);
    t.reset_damage();
    t
}

fn feed(term: &mut impl vte::ansi::Handler, bytes: &[u8]) {
    let mut processor: Processor = Processor::new();
    processor.advance(term, bytes);
}

// --- Existing alt-screen tests (migrated from term/tests.rs) ---

#[test]
fn swap_alt_switches_to_alt_grid_and_back() {
    let mut term = make_term();
    term.grid_mut().put_char('A');

    term.swap_alt();
    assert!(term.mode().contains(TermMode::ALT_SCREEN));
    assert_eq!(term.grid()[Line(0)][Column(0)].ch, ' ');

    term.grid_mut().put_char('B');

    term.swap_alt();
    assert!(!term.mode().contains(TermMode::ALT_SCREEN));
    assert_eq!(term.grid()[Line(0)][Column(0)].ch, 'A');
}

#[test]
fn swap_alt_preserves_keyboard_mode_stacks() {
    let mut term = make_term();
    let mode1 = KeyboardModes::DISAMBIGUATE_ESC_CODES;
    let mode3 = KeyboardModes::DISAMBIGUATE_ESC_CODES | KeyboardModes::REPORT_EVENT_TYPES;
    term.keyboard_mode_stack.push_back(mode1);
    term.keyboard_mode_stack.push_back(mode3);

    term.swap_alt();
    assert!(term.keyboard_mode_stack.is_empty());
    assert_eq!(
        term.inactive_keyboard_mode_stack,
        VecDeque::from(vec![mode1, mode3])
    );

    term.swap_alt();
    assert_eq!(term.keyboard_mode_stack, VecDeque::from(vec![mode1, mode3]));
    assert!(term.inactive_keyboard_mode_stack.is_empty());
}

#[test]
fn damage_swap_alt_marks_all_dirty() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[?1049h");

    let dmg = t.damage();
    assert!(dmg.is_all_dirty(), "swap_alt should mark all dirty");
    drop(dmg);
}

#[test]
fn damage_swap_alt_back_marks_all_dirty() {
    let mut t = damage_term();
    feed(&mut t, b"\x1b[?1049h");
    t.reset_damage();

    feed(&mut t, b"\x1b[?1049l");

    let dmg = t.damage();
    assert!(dmg.is_all_dirty(), "swap_alt back should mark all dirty");
    drop(dmg);
}

#[test]
fn selection_dirty_set_by_swap_alt() {
    let mut term = make_term();
    term.clear_selection_dirty();
    term.swap_alt();
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_alt_screen_via_decset() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[?1049h");
    assert!(term.is_selection_dirty());
}

#[test]
fn resize_before_alt_screen_no_crash() {
    let mut term = make_term();
    assert!(term.alt_grid.is_none());
    term.resize(10, 40, true);
    assert!(
        term.alt_grid.is_none(),
        "resize should not allocate alt grid"
    );
}

#[test]
fn alt_screen_reentry_correct() {
    let mut term = make_term();
    feed(&mut term, b"\x1b[?1049h");
    feed(&mut term, b"hello");
    feed(&mut term, b"\x1b[?1049l");
    feed(&mut term, b"\x1b[?1049h");

    assert!(term.mode().contains(TermMode::ALT_SCREEN));
    assert_eq!(term.grid().lines(), 24);
    assert_eq!(term.grid().cols(), 80);
}

#[test]
fn resize_on_alt_screen_then_snapshot() {
    let mut term = make_term();
    feed(&mut term, b"primary content");
    term.swap_alt();
    feed(&mut term, b"alt content");

    term.resize(10, 40, true);
    let snap = term.renderable_content();
    assert_eq!(snap.lines, 10);
    assert_eq!(snap.cols, 40);
    assert_eq!(snap.cells.len(), 10 * 40);
}

// --- New BUG-08-012 toggle_alt_common drift/paired-snapshot tests ---

/// Regression: BUG-08-012 — kitty keyboard mode bits leaked across
/// `?1049h/l` toggles before the reapply-top fix in `toggle_alt_common`.
/// See: bug-tracker/plans/completed/BUG-08-012/00-overview.md
#[test]
fn toggle_alt_common_swaps_nonempty_stacks_reapplies_new_active_top() {
    let mut term = make_term();
    // Shell pushes DISAMBIGUATE on primary; this path updates both the
    // stack AND the live bits via the VTE handler — which is what real
    // shell integrations do.
    feed(&mut term, b"\x1b[>1u");
    assert!(term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES));
    // Simulate alt screen having a prior push of REPORT_ALL.
    term.inactive_keyboard_mode_stack
        .push_back(KeyboardModes::REPORT_ALL_KEYS_AS_ESC);
    term.inactive_keyboard_mode_bits = KeyboardModes::REPORT_ALL_KEYS_AS_ESC;

    // DECSET 1049 → alt screen: swap stacks + swap live bits.
    feed(&mut term, b"\x1b[?1049h");

    assert!(term.mode().contains(TermMode::ALT_SCREEN));
    assert_eq!(
        term.keyboard_mode_stack,
        VecDeque::from(vec![KeyboardModes::REPORT_ALL_KEYS_AS_ESC])
    );
    assert!(
        term.mode().contains(TermMode::REPORT_ALL_KEYS_AS_ESC),
        "toggle_alt_common must reapply new-active-screen bits"
    );
    assert!(
        !term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES),
        "old active-screen bits must be cleared after swap"
    );
    assert_eq!(
        term.inactive_keyboard_mode_bits(),
        KeyboardModes::DISAMBIGUATE_ESC_CODES,
        "primary's bits are now held in the inactive slot"
    );
}

/// Regression: BUG-08-012 — empty alt stack left primary's KITTY bits
/// active after `?1049h`.
#[test]
fn toggle_alt_common_swaps_to_empty_alt_clears_mode_bits() {
    let mut term = make_term();
    let mode_a = KeyboardModes::DISAMBIGUATE_ESC_CODES;
    term.keyboard_mode_stack.push_back(mode_a);
    term.mode.insert(TermMode::DISAMBIGUATE_ESC_CODES);

    feed(&mut term, b"\x1b[?1049h");

    assert!(term.mode().contains(TermMode::ALT_SCREEN));
    assert!(term.keyboard_mode_stack.is_empty());
    assert!(
        !term.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL),
        "empty new-active-stack → Replace with NO_MODE → KITTY bits cleared"
    );
}

/// Regression: BUG-08-012 — paired snapshot fields must swap alongside
/// stacks so a command-boundary snapshot taken on one screen only fires
/// restore on that screen.
#[test]
fn toggle_alt_common_also_swaps_paired_snapshots() {
    let mut term = make_term();
    let mode_a = KeyboardModes::DISAMBIGUATE_ESC_CODES;
    // Populate primary's pre-command snapshot with Some([mode_a]);
    // leave alt's snapshot at None.
    term.pre_command_kb_stack_snapshot = Some(VecDeque::from(vec![mode_a]));
    term.inactive_pre_command_kb_stack_snapshot = None;

    term.swap_alt();

    assert!(
        term.pre_command_kb_stack_snapshot().is_none(),
        "after swap, active (alt) snapshot inherits the inactive-side None"
    );
    assert_eq!(
        term.inactive_pre_command_kb_stack_snapshot(),
        Some(&VecDeque::from(vec![mode_a])),
        "after swap, inactive (primary) snapshot carries the pre-swap active snapshot"
    );
}
