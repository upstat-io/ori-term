//! Unit tests for IME handling, preedit overlay, and mark-mode dispatch wiring.

use std::cell::RefCell;
use std::collections::HashMap;

use winit::event::{ElementState, Ime};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

use oriterm_core::grid::StableRowIndex;
use oriterm_core::{
    CellFlags, Column, CursorShape, RenderableCell, RenderableContent, RenderableCursor, Rgb,
    Selection, Side, TermMode,
};
use oriterm_mux::{MarkCursor, PaneId};

use super::super::mark_mode::{MarkAction, MarkModeKeyEvent, MarkModeResult, SelectionUpdate};
use super::super::redraw::preedit::overlay_preedit_cells;
use super::ime::{ImeEffect, ImeState};
use super::mark_mode_dispatch::{
    MarkKeyInput, MarkModeDispatch, MarkModeResources, MarkModeSink, dispatch_mark_mode,
    mark_mode_should_exit,
};
use super::{PtyInputRedrawState, should_redraw_after_pty_input};

const FG: Rgb = Rgb {
    r: 211,
    g: 215,
    b: 207,
};
const BG: Rgb = Rgb { r: 0, g: 0, b: 0 };

/// Build a renderable content with a grid of spaces and cursor at `(line, col)`.
fn content_with_cursor(
    cols: usize,
    rows: usize,
    cursor_line: usize,
    cursor_col: usize,
) -> RenderableContent {
    let mut cells = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            cells.push(RenderableCell {
                line: row,
                column: Column(col),
                ch: ' ',
                fg: FG,
                bg: BG,
                flags: CellFlags::empty(),
                underline_color: None,
                has_hyperlink: false,
                hyperlink_uri: None,
                zerowidth: Vec::new(),
            });
        }
    }
    let mut c = RenderableContent::default();
    c.cells = cells;
    c.cursor = RenderableCursor {
        line: cursor_line,
        column: Column(cursor_col),
        shape: CursorShape::Block,
        visible: true,
    };
    c.mode = TermMode::SHOW_CURSOR;
    c.all_dirty = true;
    c
}

// --- overlay_preedit_cells ---

#[test]
fn preedit_replaces_cell_at_cursor() {
    let mut content = content_with_cursor(10, 1, 0, 3);
    overlay_preedit_cells("A", &mut content, 10);

    assert_eq!(content.cells[3].ch, 'A');
    assert!(content.cells[3].flags.contains(CellFlags::UNDERLINE));
}

#[test]
fn preedit_hides_cursor() {
    let mut content = content_with_cursor(10, 1, 0, 0);
    assert!(content.cursor.visible);

    overlay_preedit_cells("x", &mut content, 10);
    assert!(!content.cursor.visible);
}

#[test]
fn preedit_wide_char_sets_flags() {
    let mut content = content_with_cursor(10, 1, 0, 0);
    // U+4E2D '中' is a CJK character (display width 2).
    overlay_preedit_cells("中", &mut content, 10);

    assert_eq!(content.cells[0].ch, '中');
    assert!(content.cells[0].flags.contains(CellFlags::WIDE_CHAR));
    assert!(content.cells[0].flags.contains(CellFlags::UNDERLINE));
    assert!(content.cells[1].flags.contains(CellFlags::WIDE_CHAR_SPACER));
}

#[test]
fn preedit_multiple_chars() {
    let mut content = content_with_cursor(10, 1, 0, 2);
    overlay_preedit_cells("AB", &mut content, 10);

    assert_eq!(content.cells[2].ch, 'A');
    assert_eq!(content.cells[3].ch, 'B');
    assert!(content.cells[2].flags.contains(CellFlags::UNDERLINE));
    assert!(content.cells[3].flags.contains(CellFlags::UNDERLINE));
    // Other cells unchanged.
    assert_eq!(content.cells[0].ch, ' ');
    assert_eq!(content.cells[4].ch, ' ');
}

#[test]
fn preedit_clips_at_grid_edge() {
    let mut content = content_with_cursor(4, 1, 0, 3);
    // Cursor at col 3, grid is 4 cols — only 1 cell available.
    overlay_preedit_cells("XY", &mut content, 4);

    assert_eq!(content.cells[3].ch, 'X');
    // 'Y' is clipped (col 4 doesn't exist).
}

#[test]
fn preedit_wide_char_clips_at_edge() {
    let mut content = content_with_cursor(4, 1, 0, 3);
    // Wide char at col 3 needs 2 cells but only 1 available — still placed
    // (the spacer at col 4 is out of bounds, which is handled gracefully).
    overlay_preedit_cells("中", &mut content, 4);

    assert_eq!(content.cells[3].ch, '中');
    assert!(content.cells[3].flags.contains(CellFlags::WIDE_CHAR));
}

#[test]
fn preedit_empty_string_no_change() {
    let mut content = content_with_cursor(10, 1, 0, 0);
    // Empty preedit shouldn't change anything (but cursor is still hidden
    // because overlay_preedit_cells is only called when preedit is non-empty
    // in the actual app code).
    let original_ch = content.cells[0].ch;
    overlay_preedit_cells("", &mut content, 10);

    assert_eq!(content.cells[0].ch, original_ch);
}

#[test]
fn preedit_on_second_row() {
    let mut content = content_with_cursor(10, 3, 1, 5);
    overlay_preedit_cells("Z", &mut content, 10);

    // Row 1, col 5 = index 1*10 + 5 = 15.
    assert_eq!(content.cells[15].ch, 'Z');
    assert!(content.cells[15].flags.contains(CellFlags::UNDERLINE));
}

#[test]
fn preedit_cjk_composition_sequence() {
    let mut content = content_with_cursor(20, 1, 0, 0);
    // Typical CJK composition: two wide characters.
    overlay_preedit_cells("中文", &mut content, 20);

    // First char at cols 0-1.
    assert_eq!(content.cells[0].ch, '中');
    assert!(content.cells[0].flags.contains(CellFlags::WIDE_CHAR));
    assert!(content.cells[1].flags.contains(CellFlags::WIDE_CHAR_SPACER));
    // Second char at cols 2-3.
    assert_eq!(content.cells[2].ch, '文');
    assert!(content.cells[2].flags.contains(CellFlags::WIDE_CHAR));
    assert!(content.cells[3].flags.contains(CellFlags::WIDE_CHAR_SPACER));
}

// --- ImeState: state machine transitions ---

#[test]
fn ime_enabled_sets_active() {
    let mut ime = ImeState::new();
    assert!(!ime.active);

    let effect = ime.handle_event(Ime::Enabled);
    assert!(ime.active);
    assert_eq!(effect, ImeEffect::Redraw);
}

#[test]
fn ime_preedit_stores_text_and_cursor() {
    let mut ime = ImeState::new();
    ime.active = true;

    let effect = ime.handle_event(Ime::Preedit("abc".into(), Some((1, 2))));
    assert_eq!(ime.preedit, "abc");
    assert_eq!(ime.preedit_cursor, Some(1));
    assert_eq!(effect, ImeEffect::UpdateCursorArea);
}

#[test]
fn ime_commit_clears_state_and_returns_text() {
    let mut ime = ImeState::new();
    ime.active = true;
    ime.preedit = "composing".into();
    ime.preedit_cursor = Some(3);

    let effect = ime.handle_event(Ime::Commit("final".into()));
    assert!(!ime.active);
    assert!(ime.preedit.is_empty());
    assert!(ime.preedit_cursor.is_none());
    assert_eq!(effect, ImeEffect::Commit("final".into()));
}

#[test]
fn ime_disabled_clears_all_state() {
    let mut ime = ImeState::new();
    ime.active = true;
    ime.preedit = "partial".into();
    ime.preedit_cursor = Some(2);

    let effect = ime.handle_event(Ime::Disabled);
    assert!(!ime.active);
    assert!(ime.preedit.is_empty());
    assert!(ime.preedit_cursor.is_none());
    assert_eq!(effect, ImeEffect::Redraw);
}

#[test]
fn ime_full_lifecycle() {
    let mut ime = ImeState::new();

    // Enabled.
    ime.handle_event(Ime::Enabled);
    assert!(ime.active);

    // Preedit starts.
    ime.handle_event(Ime::Preedit("k".into(), Some((1, 1))));
    assert_eq!(ime.preedit, "k");
    assert!(ime.should_suppress_key());

    // Preedit updates.
    ime.handle_event(Ime::Preedit("ka".into(), Some((2, 2))));
    assert_eq!(ime.preedit, "ka");

    // Commit.
    let effect = ime.handle_event(Ime::Commit("か".into()));
    assert_eq!(effect, ImeEffect::Commit("か".into()));
    assert!(!ime.active);
    assert!(!ime.should_suppress_key());
}

#[test]
fn ime_enabled_then_disabled_without_preedit() {
    let mut ime = ImeState::new();

    ime.handle_event(Ime::Enabled);
    assert!(ime.active);
    // No preedit — should NOT suppress keys (active but preedit empty).
    assert!(!ime.should_suppress_key());

    ime.handle_event(Ime::Disabled);
    assert!(!ime.active);
}

// --- ImeState: key suppression ---

#[test]
fn suppress_key_when_active_with_preedit() {
    let mut ime = ImeState::new();
    ime.active = true;
    ime.preedit = "中".into();
    assert!(ime.should_suppress_key());
}

#[test]
fn no_suppress_when_inactive() {
    let ime = ImeState::new();
    assert!(!ime.should_suppress_key());
}

#[test]
fn no_suppress_when_active_but_preedit_empty() {
    let mut ime = ImeState::new();
    ime.active = true;
    // Active with empty preedit (between commit and disable).
    assert!(!ime.should_suppress_key());
}

#[test]
fn no_suppress_after_commit() {
    let mut ime = ImeState::new();
    ime.active = true;
    ime.preedit = "test".into();
    assert!(ime.should_suppress_key());

    ime.handle_event(Ime::Commit("test".into()));
    // After commit: active=false, preedit=empty.
    assert!(!ime.should_suppress_key());
}

// --- overlay_preedit_cells: combining marks ---

#[test]
fn preedit_combining_mark_does_not_occupy_cell() {
    let mut content = content_with_cursor(10, 1, 0, 0);
    // 'a' followed by U+0301 (combining acute accent) — only 'a' has width.
    overlay_preedit_cells("a\u{0301}", &mut content, 10);

    // The 'a' occupies col 0.
    assert_eq!(content.cells[0].ch, 'a');
    assert!(content.cells[0].flags.contains(CellFlags::UNDERLINE));
    // Col 1 is unchanged (combining mark has width 0, skipped).
    assert_eq!(content.cells[1].ch, ' ');
}

// --- overlay_preedit_cells: cursor offset with wide chars ---

#[test]
fn preedit_wide_chars_advance_column_by_two() {
    let mut content = content_with_cursor(20, 1, 0, 0);
    // Mix of narrow and wide: 'A' (w=1) + '中' (w=2) + 'B' (w=1).
    overlay_preedit_cells("A中B", &mut content, 20);

    // 'A' at col 0.
    assert_eq!(content.cells[0].ch, 'A');
    // '中' at col 1 (wide: occupies cols 1-2).
    assert_eq!(content.cells[1].ch, '中');
    assert!(content.cells[1].flags.contains(CellFlags::WIDE_CHAR));
    assert!(content.cells[2].flags.contains(CellFlags::WIDE_CHAR_SPACER));
    // 'B' at col 3 (after wide char's 2-cell span).
    assert_eq!(content.cells[3].ch, 'B');
}

// --- overlay_preedit_cells: very long preedit ---

#[test]
fn preedit_long_string_truncates_at_grid_width() {
    let mut content = content_with_cursor(5, 1, 0, 0);
    let long_preedit = "ABCDEFGHIJ"; // 10 chars, grid is 5 wide.
    overlay_preedit_cells(long_preedit, &mut content, 5);

    assert_eq!(content.cells[0].ch, 'A');
    assert_eq!(content.cells[1].ch, 'B');
    assert_eq!(content.cells[2].ch, 'C');
    assert_eq!(content.cells[3].ch, 'D');
    assert_eq!(content.cells[4].ch, 'E');
    // Only 5 chars fit, rest clipped. No panic.
}

#[test]
fn preedit_long_wide_string_truncates_gracefully() {
    let mut content = content_with_cursor(6, 1, 0, 0);
    // 4 CJK chars = 8 display width, grid is 6 wide.
    overlay_preedit_cells("中文日本", &mut content, 6);

    // '中' at cols 0-1.
    assert_eq!(content.cells[0].ch, '中');
    // '文' at cols 2-3.
    assert_eq!(content.cells[2].ch, '文');
    // '日' at cols 4-5.
    assert_eq!(content.cells[4].ch, '日');
    // '本' would need cols 6-7 but col 6 >= grid width → clipped.
}

// --- overlay_preedit_cells: emoji ---

#[test]
fn preedit_emoji_basic() {
    let mut content = content_with_cursor(10, 1, 0, 0);
    // U+1F600 GRINNING FACE — typically width 2.
    overlay_preedit_cells("😀", &mut content, 10);

    assert_eq!(content.cells[0].ch, '😀');
    // unicode-width reports 2 for most emoji.
    assert!(content.cells[0].flags.contains(CellFlags::WIDE_CHAR));
    assert!(content.cells[0].flags.contains(CellFlags::UNDERLINE));
    assert!(content.cells[1].flags.contains(CellFlags::WIDE_CHAR_SPACER));
}

// --- overlay_preedit_cells: rapid updates ---

#[test]
fn preedit_successive_overlays_replace_cleanly() {
    let mut content = content_with_cursor(10, 1, 0, 0);

    // First preedit: "AB" at cols 0-1.
    overlay_preedit_cells("AB", &mut content, 10);
    assert_eq!(content.cells[0].ch, 'A');
    assert_eq!(content.cells[1].ch, 'B');

    // Reset cursor visibility for next overlay (in real code, content is
    // re-extracted each frame, so cursor.visible starts true again).
    content.cursor.visible = true;

    // Second preedit: "X" at col 0. Col 1 retains 'B' from first overlay
    // (in real code, content is fresh each frame — this tests the function
    // itself doesn't leave stale state beyond its written range).
    overlay_preedit_cells("X", &mut content, 10);
    assert_eq!(content.cells[0].ch, 'X');
    assert!(content.cells[0].flags.contains(CellFlags::UNDERLINE));
}

// --- overlay_preedit_cells: empty/zero grid ---

#[test]
fn preedit_zero_cols_no_panic() {
    let mut content = content_with_cursor(10, 1, 0, 0);
    // cols=0 should early-return without modifying anything.
    overlay_preedit_cells("A", &mut content, 0);
    assert_eq!(content.cells[0].ch, ' ');
    assert!(content.cursor.visible);
}

#[test]
fn preedit_empty_cells_no_panic() {
    let mut content = RenderableContent::default();
    content.cursor.visible = true;
    content.mode = TermMode::SHOW_CURSOR;
    content.all_dirty = true;
    // Empty cells vec should early-return without panic.
    overlay_preedit_cells("A", &mut content, 10);
    assert!(content.cells.is_empty());
}

// --- PTY input redraw policy ---

#[test]
fn pty_input_skips_redraw_at_live_prompt_with_visible_cursor() {
    assert!(!should_redraw_after_pty_input(PtyInputRedrawState {
        cursor_hidden_by_blink: false,
        snapshot_dirty: false,
        snapshot_display_offset: Some(0),
    }));
}

#[test]
fn pty_input_redraws_when_cursor_was_blink_hidden() {
    assert!(should_redraw_after_pty_input(PtyInputRedrawState {
        cursor_hidden_by_blink: true,
        snapshot_dirty: false,
        snapshot_display_offset: Some(0),
    }));
}

#[test]
fn pty_input_redraws_when_typing_exits_scrollback() {
    assert!(should_redraw_after_pty_input(PtyInputRedrawState {
        cursor_hidden_by_blink: false,
        snapshot_dirty: false,
        snapshot_display_offset: Some(4),
    }));
}

#[test]
fn pty_input_redraws_without_a_cached_snapshot() {
    assert!(should_redraw_after_pty_input(PtyInputRedrawState {
        cursor_hidden_by_blink: false,
        snapshot_dirty: false,
        snapshot_display_offset: None,
    }));
}

// --- Dispatch chain: keybinding priority over PTY ---

/// Verify that bound key combos are intercepted by the keybinding table
/// before reaching PTY encoding.
///
/// In `handle_keyboard_input`, bindings are checked (via `find_binding`)
/// before `encode_key_to_pty`. When a binding is found and `execute_action`
/// returns `true`, the method returns early — PTY encoding is never reached.
#[test]
fn binding_takes_priority_over_pty_send() {
    use winit::keyboard::Key;

    use oriterm_core::TermMode;

    use crate::key_encoding::{KeyEventType, KeyInput, Modifiers, encode_key};
    use crate::keybindings::{self, Action, BindingKey};

    let bindings = keybindings::default_bindings();

    // Ctrl+Shift+C is bound to Copy — execute_action(Copy) always returns
    // true, so the event is consumed and PTY encoding is skipped.
    let key_c = BindingKey::Character("c".to_owned());
    let action =
        keybindings::find_binding(&bindings, &key_c, Modifiers::CONTROL | Modifiers::SHIFT);
    assert_eq!(
        action,
        Some(&Action::Copy),
        "Ctrl+Shift+C should bind to Copy (consumes event, PTY skipped)"
    );

    // Contrast: unbound key 'a' has no binding, so handle_keyboard_input
    // falls through to encode_key_to_pty, producing PTY output.
    let key_a = BindingKey::Character("a".to_owned());
    assert_eq!(
        keybindings::find_binding(&bindings, &key_a, Modifiers::empty()),
        None,
        "bare 'a' should have no binding (falls through to PTY)"
    );

    // Confirm the unbound key produces PTY bytes via encode_key.
    let pty_bytes = encode_key(&KeyInput {
        key: &Key::Character("a".into()),
        mods: Modifiers::empty(),
        mode: TermMode::default(),
        text: Some("a"),
        location: winit::keyboard::KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: None,
    });
    assert_eq!(pty_bytes, b"a", "unbound 'a' should encode as PTY output");
}

// --- Dispatch chain: SmartCopy conditional fallthrough ---

/// Verify that Ctrl+C uses SmartCopy, which conditionally falls through
/// to PTY encoding when no selection exists.
///
/// Dispatch path with no selection:
///   find_binding(Ctrl+C) → SmartCopy
///   → execute_action(SmartCopy) returns false (no selection)
///   → encode_key_to_pty(Ctrl+C) → sends \x03 (ETX/SIGINT)
///
/// Dispatch path with selection:
///   find_binding(Ctrl+C) → SmartCopy
///   → execute_action(SmartCopy) copies + returns true (event consumed)
///   → PTY never reached (no SIGINT)
#[test]
fn ctrl_c_smart_copy_falls_through_to_pty_without_selection() {
    use winit::keyboard::Key;

    use oriterm_core::TermMode;

    use crate::key_encoding::{KeyEventType, KeyInput, Modifiers, encode_key};
    use crate::keybindings::{self, Action, BindingKey};

    let bindings = keybindings::default_bindings();

    // Ctrl+C is bound to SmartCopy — the only action whose execute_action
    // returns false (when no selection exists), allowing PTY fallthrough.
    let key_c = BindingKey::Character("c".to_owned());
    let action = keybindings::find_binding(&bindings, &key_c, Modifiers::CONTROL);
    assert_eq!(
        action,
        Some(&Action::SmartCopy),
        "Ctrl+C should bind to SmartCopy (conditional consumption)"
    );

    // When SmartCopy returns false (no selection), handle_keyboard_input
    // falls through to encode_key_to_pty. Ctrl+C encodes as \x03
    // (ETX — the byte that triggers SIGINT in the PTY).
    let pty_bytes = encode_key(&KeyInput {
        key: &Key::Character("c".into()),
        mods: Modifiers::CONTROL,
        mode: TermMode::default(),
        text: None,
        location: winit::keyboard::KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: None,
    });
    assert_eq!(
        pty_bytes,
        vec![0x03],
        "Ctrl+C should encode as \\x03 for PTY (SIGINT)"
    );

    // Ctrl+Shift+C uses Copy (not SmartCopy) — always consumes, never
    // falls through. This ensures users have an unconditional copy binding.
    let copy_action =
        keybindings::find_binding(&bindings, &key_c, Modifiers::CONTROL | Modifiers::SHIFT);
    assert_eq!(
        copy_action,
        Some(&Action::Copy),
        "Ctrl+Shift+C should bind to Copy (always consumes)"
    );
}

// --- Mark mode exit decision (BUG-08-031) ---

/// Regression: BUG-08-031 — try_dispatch_mark_mode swallowed keypresses
/// when mux/cursor/snapshot was None during mark mode. The pure decision
/// predicate `mark_mode_should_exit` returns false ONLY when all three
/// resources are present, so the caller proceeds into mark-mode dispatch.
/// See: bug-tracker/plans/BUG-08-031/00-overview.md
#[test]
fn mark_mode_exit_decision_with_all_resources_present_returns_false() {
    let resources = MarkModeResources {
        mux_present: true,
        cursor_present: true,
        snapshot_present: true,
    };
    assert!(
        !mark_mode_should_exit(resources),
        "all resources present → proceed into mark-mode dispatch (do NOT exit)"
    );
}

/// Regression: BUG-08-031 — pins the mux-None recovery path. The original
/// bug returned `true` (silent swallow) at `mod.rs:177`; the fix routes
/// through `mark_mode_should_exit → true → caller exits mark mode +
/// returns false → keystroke flows to keybinding then PTY`.
/// See: bug-tracker/plans/BUG-08-031/00-overview.md
#[test]
fn mark_mode_exit_decision_with_mux_missing_returns_true() {
    let resources = MarkModeResources {
        mux_present: false,
        cursor_present: true,
        snapshot_present: true,
    };
    assert!(
        mark_mode_should_exit(resources),
        "mux missing → exit mark mode (recover from missing-resource state)"
    );
}

/// Regression: BUG-08-031 — pins the cursor-None recovery path. The
/// original bug returned `true` (silent swallow) at `mod.rs:183`; the fix
/// routes the keystroke through normal dispatch instead.
/// See: bug-tracker/plans/BUG-08-031/00-overview.md
#[test]
fn mark_mode_exit_decision_with_cursor_missing_returns_true() {
    let resources = MarkModeResources {
        mux_present: true,
        cursor_present: false,
        snapshot_present: true,
    };
    assert!(
        mark_mode_should_exit(resources),
        "cursor missing → exit mark mode (recover from missing-resource state)"
    );
}

/// Regression: BUG-08-031 — pins the snapshot-None recovery path. The
/// original bug returned `true` (silent swallow) at `mod.rs:189`; the fix
/// routes the keystroke through normal dispatch instead.
/// See: bug-tracker/plans/BUG-08-031/00-overview.md
#[test]
fn mark_mode_exit_decision_with_snapshot_missing_returns_true() {
    let resources = MarkModeResources {
        mux_present: true,
        cursor_present: true,
        snapshot_present: false,
    };
    assert!(
        mark_mode_should_exit(resources),
        "snapshot missing → exit mark mode (recover from missing-resource state)"
    );
}

/// Regression: BUG-08-031 — matrix completeness pin. Enumerates all 8
/// truth-table cells over `(mux_present, cursor_present, snapshot_present)`
/// and asserts that exactly one cell (`true, true, true`) returns false
/// while the other 7 return true. The `count == 8` assertion proves no
/// cells were skipped per `tests.md §Matrix Testing Rule` self-verifying
/// completeness pattern.
/// See: bug-tracker/plans/BUG-08-031/00-overview.md
#[test]
fn mark_mode_exit_decision_truth_table_complete() {
    let mut count = 0;
    let mut proceed_cells = 0;
    for mux in [false, true] {
        for cursor in [false, true] {
            for snapshot in [false, true] {
                let resources = MarkModeResources {
                    mux_present: mux,
                    cursor_present: cursor,
                    snapshot_present: snapshot,
                };
                let exit = mark_mode_should_exit(resources);
                let all_present = mux && cursor && snapshot;
                assert_eq!(
                    exit, !all_present,
                    "({mux}, {cursor}, {snapshot}) — exit should be {} (proceed iff all present)",
                    !all_present
                );
                if !exit {
                    proceed_cells += 1;
                }
                count += 1;
            }
        }
    }
    assert_eq!(count, 8, "truth table must enumerate all 2³ = 8 cells");
    assert_eq!(
        proceed_cells, 1,
        "exactly one cell (all-present) must proceed; all 7 missing-resource cells must exit"
    );
}

// --- Mark-mode dispatch chain (BUG-08-034) ---
//
// Pins the wiring between gate decision, pre-handler refresh, pure handler
// dispatch, and post-handler state mutations. Replaces the BUG-08-031
// helper-test-only coverage of `mark_mode_should_exit`.
//
// See: bug-tracker/plans/BUG-08-034/00-overview.md.

/// Distinguishable record of every `MarkModeSink` method invocation. Stored
/// in `RecordingSink::call_log` (RefCell — &self query methods record too).
#[derive(Debug, Clone, PartialEq, Eq)]
enum MethodCall {
    MarkModeResources {
        pane_id: PaneId,
    },
    PaneMarkCursor {
        pane_id: PaneId,
    },
    PaneSelection {
        pane_id: PaneId,
    },
    RefreshPaneSnapshot {
        pane_id: PaneId,
    },
    ExitMarkMode {
        pane_id: PaneId,
    },
    SetMarkCursor {
        pane_id: PaneId,
        cursor: MarkCursor,
    },
    SetPaneSelection {
        pane_id: PaneId,
        sel: Selection,
    },
    ClearPaneSelection {
        pane_id: PaneId,
    },
    ScrollDisplay {
        pane_id: PaneId,
        lines: isize,
    },
    CopySelection,
    MarkDirty,
    DispatchMarkModeKey {
        pane_id: PaneId,
        cursor: MarkCursor,
        selection: Option<Selection>,
        event: MarkModeKeyEvent,
        modifiers: ModifiersState,
    },
}

/// Test fixture for `dispatch_mark_mode`. Records every sink-method call
/// (including `&self` query methods via RefCell) and lets each test
/// configure resource presence + the dispatch result.
#[derive(Default)]
struct RecordingSink {
    cursor_for_pane: HashMap<PaneId, MarkCursor>,
    selection_for_pane: HashMap<PaneId, Selection>,
    snapshot_present: HashMap<PaneId, bool>,
    mux_present: bool,
    dispatch_result: Option<MarkModeResult>,
    refresh_makes_snapshot_present: bool,
    call_log: RefCell<Vec<MethodCall>>,
    exit_mark_mode_calls: Vec<PaneId>,
    mark_dirty_calls: usize,
    refresh_snapshot_calls: usize,
    dispatch_key_calls: usize,
    set_mark_cursor_calls: Vec<(PaneId, MarkCursor)>,
    set_pane_selection_calls: Vec<(PaneId, Selection)>,
    clear_pane_selection_calls: Vec<PaneId>,
    scroll_display_calls: Vec<(PaneId, isize)>,
    copy_selection_calls: usize,
    last_dispatch_args: Option<(
        PaneId,
        MarkCursor,
        Option<Selection>,
        MarkModeKeyEvent,
        ModifiersState,
    )>,
}

impl RecordingSink {
    /// Count of `mark_mode_resources()` queries for a specific pane. Single
    /// SSOT (the call log) for query/mutation accounting.
    fn mark_mode_resources_calls(&self, pane_id: PaneId) -> usize {
        self.call_log
            .borrow()
            .iter()
            .filter(|c| matches!(c, MethodCall::MarkModeResources { pane_id: p } if *p == pane_id))
            .count()
    }

    /// Count of `pane_mark_cursor()` queries for a specific pane.
    fn pane_mark_cursor_calls(&self, pane_id: PaneId) -> usize {
        self.call_log
            .borrow()
            .iter()
            .filter(|c| matches!(c, MethodCall::PaneMarkCursor { pane_id: p } if *p == pane_id))
            .count()
    }

    /// Count of `pane_selection()` queries for a specific pane.
    fn pane_selection_calls(&self, pane_id: PaneId) -> usize {
        self.call_log
            .borrow()
            .iter()
            .filter(|c| matches!(c, MethodCall::PaneSelection { pane_id: p } if *p == pane_id))
            .count()
    }

    /// Snapshot of the call log for ordering / structural assertions.
    fn call_log_snapshot(&self) -> Vec<MethodCall> {
        self.call_log.borrow().clone()
    }
}

impl MarkModeSink for RecordingSink {
    fn mark_mode_resources(&mut self, pane_id: PaneId) -> MarkModeResources {
        self.call_log
            .borrow_mut()
            .push(MethodCall::MarkModeResources { pane_id });
        MarkModeResources {
            mux_present: self.mux_present,
            cursor_present: self.cursor_for_pane.contains_key(&pane_id),
            snapshot_present: *self.snapshot_present.get(&pane_id).unwrap_or(&false),
        }
    }
    fn pane_mark_cursor(&self, pane_id: PaneId) -> Option<MarkCursor> {
        self.call_log
            .borrow_mut()
            .push(MethodCall::PaneMarkCursor { pane_id });
        self.cursor_for_pane.get(&pane_id).copied()
    }
    fn pane_selection(&self, pane_id: PaneId) -> Option<Selection> {
        self.call_log
            .borrow_mut()
            .push(MethodCall::PaneSelection { pane_id });
        self.selection_for_pane.get(&pane_id).copied()
    }
    fn refresh_pane_snapshot(&mut self, pane_id: PaneId) {
        self.call_log
            .borrow_mut()
            .push(MethodCall::RefreshPaneSnapshot { pane_id });
        self.refresh_snapshot_calls += 1;
        if self.refresh_makes_snapshot_present {
            self.snapshot_present.insert(pane_id, true);
        }
    }
    fn exit_mark_mode(&mut self, pane_id: PaneId) {
        self.call_log
            .borrow_mut()
            .push(MethodCall::ExitMarkMode { pane_id });
        self.exit_mark_mode_calls.push(pane_id);
    }
    fn set_mark_cursor(&mut self, pane_id: PaneId, cursor: MarkCursor) {
        self.call_log
            .borrow_mut()
            .push(MethodCall::SetMarkCursor { pane_id, cursor });
        self.set_mark_cursor_calls.push((pane_id, cursor));
    }
    fn set_pane_selection(&mut self, pane_id: PaneId, sel: Selection) {
        self.call_log
            .borrow_mut()
            .push(MethodCall::SetPaneSelection { pane_id, sel });
        self.set_pane_selection_calls.push((pane_id, sel));
    }
    fn clear_pane_selection(&mut self, pane_id: PaneId) {
        self.call_log
            .borrow_mut()
            .push(MethodCall::ClearPaneSelection { pane_id });
        self.clear_pane_selection_calls.push(pane_id);
    }
    fn scroll_display(&mut self, pane_id: PaneId, lines: isize) {
        self.call_log
            .borrow_mut()
            .push(MethodCall::ScrollDisplay { pane_id, lines });
        self.scroll_display_calls.push((pane_id, lines));
    }
    fn copy_selection(&mut self) {
        self.call_log.borrow_mut().push(MethodCall::CopySelection);
        self.copy_selection_calls += 1;
    }
    fn mark_dirty(&mut self) {
        self.call_log.borrow_mut().push(MethodCall::MarkDirty);
        self.mark_dirty_calls += 1;
    }
    fn dispatch_mark_mode_key(&mut self, input: MarkKeyInput) -> MarkModeResult {
        self.call_log
            .borrow_mut()
            .push(MethodCall::DispatchMarkModeKey {
                pane_id: input.pane_id,
                cursor: input.cursor,
                selection: input.selection,
                event: input.event.clone(),
                modifiers: input.modifiers,
            });
        self.dispatch_key_calls += 1;
        self.last_dispatch_args = Some((
            input.pane_id,
            input.cursor,
            input.selection,
            input.event,
            input.modifiers,
        ));
        self.dispatch_result
            .take()
            .expect("dispatch_result must be configured by test")
    }
}

fn make_pane_id() -> PaneId {
    PaneId::from_raw(1)
}

fn make_mark_key_event() -> MarkModeKeyEvent {
    MarkModeKeyEvent {
        physical_key: PhysicalKey::Code(KeyCode::ArrowDown),
        logical_key: Key::Named(NamedKey::ArrowDown),
    }
}

fn make_mark_cursor() -> MarkCursor {
    MarkCursor {
        row: StableRowIndex(0),
        col: 0,
    }
}

fn make_selection() -> Selection {
    Selection::new_char(StableRowIndex(0), 5, Side::Left)
}

fn make_handled_no_motion() -> MarkModeResult {
    MarkModeResult {
        action: MarkAction::Handled { scroll_delta: None },
        new_cursor: None,
        new_selection: None,
    }
}

/// Bundled cell axes for the matrix loops, per `impl-hygiene.md
/// §Parameter Hygiene` (>4 params → struct).
struct DispatchCell {
    state: ElementState,
    repeat: bool,
    active_pane_id: Option<PaneId>,
    mark_mode_active: bool,
}

/// Wrapper around `dispatch_mark_mode(MarkModeDispatch { ... }, sink)` that
/// hides struct construction noise from each test body.
fn dispatch_with(sink: &mut RecordingSink, cell: DispatchCell) -> bool {
    dispatch_mark_mode(
        MarkModeDispatch {
            event: make_mark_key_event(),
            event_state: cell.state,
            event_repeat: cell.repeat,
            modifiers: ModifiersState::empty(),
            active_pane_id: cell.active_pane_id,
            mark_mode_active: cell.mark_mode_active,
        },
        sink,
    )
}

/// PIN 1 — Pressed event 16-cell truth table over
/// `(mux_present, cursor_present, snapshot_present) × event_repeat`.
///
/// Pins that the missing-resource recovery path returns false (forwards to
/// PTY) for every cell where any required resource is absent, and that
/// dispatch proceeds (returns true + key handler runs + mark_dirty fires)
/// for both `event_repeat = false` AND `event_repeat = true` when all
/// resources are present — proving repeat orthogonality.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 1).
#[test]
fn dispatch_mark_mode_pressed_truth_table_pins_exit_on_missing_resource() {
    let pane_id = make_pane_id();
    let mut cell_count = 0;
    for mux in [false, true] {
        for cursor in [false, true] {
            for snapshot in [false, true] {
                for repeat in [false, true] {
                    let mut sink = RecordingSink {
                        mux_present: mux,
                        ..Default::default()
                    };
                    if cursor {
                        sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
                    }
                    sink.snapshot_present.insert(pane_id, snapshot);
                    let all_present = mux && cursor && snapshot;
                    if all_present {
                        sink.dispatch_result = Some(make_handled_no_motion());
                    }
                    let result = dispatch_with(
                        &mut sink,
                        DispatchCell {
                            state: ElementState::Pressed,
                            repeat,
                            active_pane_id: Some(pane_id),
                            mark_mode_active: true,
                        },
                    );
                    if all_present {
                        assert!(
                            result,
                            "({mux}, {cursor}, {snapshot}, repeat={repeat}) → all present must consume"
                        );
                        assert!(
                            sink.exit_mark_mode_calls.is_empty(),
                            "({mux}, {cursor}, {snapshot}, repeat={repeat}) → all present must NOT exit"
                        );
                        assert_eq!(
                            sink.dispatch_key_calls, 1,
                            "({mux}, {cursor}, {snapshot}, repeat={repeat}) → all present must dispatch"
                        );
                        assert_eq!(
                            sink.mark_dirty_calls, 1,
                            "({mux}, {cursor}, {snapshot}, repeat={repeat}) → all present must mark_dirty"
                        );
                    } else {
                        assert!(
                            !result,
                            "({mux}, {cursor}, {snapshot}, repeat={repeat}) → missing-resource must forward (false)"
                        );
                        assert_eq!(
                            sink.exit_mark_mode_calls,
                            vec![pane_id],
                            "({mux}, {cursor}, {snapshot}, repeat={repeat}) → missing-resource must exit"
                        );
                        assert_eq!(
                            sink.dispatch_key_calls, 0,
                            "({mux}, {cursor}, {snapshot}, repeat={repeat}) → missing-resource must NOT dispatch"
                        );
                        assert_eq!(
                            sink.mark_dirty_calls, 0,
                            "({mux}, {cursor}, {snapshot}, repeat={repeat}) → missing-resource must NOT mark_dirty"
                        );
                    }
                    cell_count += 1;
                }
            }
        }
    }
    assert_eq!(
        cell_count, 16,
        "must enumerate (mux, cursor, snapshot) × repeat — 2³ × 2 = 16 cells"
    );
}

/// PIN 2 — Released event consumes regardless of resources. The Pressed-only
/// branch in `dispatch_mark_mode` never queries resources for Released events.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 2).
#[test]
fn dispatch_mark_mode_released_event_consumes_regardless_of_resources() {
    let pane_id = make_pane_id();
    let mut cell_count = 0;
    for mux in [false, true] {
        for cursor in [false, true] {
            for snapshot in [false, true] {
                let mut sink = RecordingSink {
                    mux_present: mux,
                    ..Default::default()
                };
                if cursor {
                    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
                }
                sink.snapshot_present.insert(pane_id, snapshot);
                let result = dispatch_with(
                    &mut sink,
                    DispatchCell {
                        state: ElementState::Released,
                        repeat: false,
                        active_pane_id: Some(pane_id),
                        mark_mode_active: true,
                    },
                );
                assert!(
                    result,
                    "({mux}, {cursor}, {snapshot}) Released → must consume"
                );
                assert_eq!(
                    sink.mark_mode_resources_calls(pane_id),
                    0,
                    "({mux}, {cursor}, {snapshot}) Released → must NOT query resources"
                );
                assert_eq!(
                    sink.pane_mark_cursor_calls(pane_id),
                    0,
                    "({mux}, {cursor}, {snapshot}) Released → must NOT query mark cursor"
                );
                assert_eq!(
                    sink.pane_selection_calls(pane_id),
                    0,
                    "({mux}, {cursor}, {snapshot}) Released → must NOT query selection"
                );
                assert_eq!(
                    sink.refresh_snapshot_calls, 0,
                    "({mux}, {cursor}, {snapshot}) Released → must NOT refresh"
                );
                assert!(
                    sink.exit_mark_mode_calls.is_empty(),
                    "({mux}, {cursor}, {snapshot}) Released → must NOT exit"
                );
                assert_eq!(
                    sink.dispatch_key_calls, 0,
                    "({mux}, {cursor}, {snapshot}) Released → must NOT dispatch"
                );
                assert_eq!(
                    sink.mark_dirty_calls, 0,
                    "({mux}, {cursor}, {snapshot}) Released → must NOT mark_dirty"
                );
                cell_count += 1;
            }
        }
    }
    assert_eq!(
        cell_count, 8,
        "must enumerate (mux, cursor, snapshot) — 2³ = 8 cells for Released"
    );
}

/// PIN 3 — Mark mode inactive: passthrough returns false without ANY sink
/// query or mutation. Pins the early `if !mark_mode_active { return false; }`.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 3).
#[test]
fn dispatch_mark_mode_inactive_returns_false_without_side_effects() {
    let pane_id = make_pane_id();
    let mut cell_count = 0;
    for state in [ElementState::Pressed, ElementState::Released] {
        for mux in [false, true] {
            for cursor in [false, true] {
                for snapshot in [false, true] {
                    let mut sink = RecordingSink {
                        mux_present: mux,
                        ..Default::default()
                    };
                    if cursor {
                        sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
                    }
                    sink.snapshot_present.insert(pane_id, snapshot);
                    let result = dispatch_with(
                        &mut sink,
                        DispatchCell {
                            state,
                            repeat: false,
                            active_pane_id: Some(pane_id),
                            mark_mode_active: false,
                        },
                    );
                    assert!(
                        !result,
                        "({state:?}, {mux}, {cursor}, {snapshot}) inactive → must passthrough false"
                    );
                    assert_eq!(
                        sink.mark_mode_resources_calls(pane_id),
                        0,
                        "inactive must NOT query resources"
                    );
                    assert_eq!(
                        sink.pane_mark_cursor_calls(pane_id),
                        0,
                        "inactive must NOT query mark cursor"
                    );
                    assert_eq!(
                        sink.pane_selection_calls(pane_id),
                        0,
                        "inactive must NOT query selection"
                    );
                    assert_eq!(sink.refresh_snapshot_calls, 0);
                    assert!(sink.exit_mark_mode_calls.is_empty());
                    assert_eq!(sink.dispatch_key_calls, 0);
                    assert_eq!(sink.mark_dirty_calls, 0);
                    assert!(
                        !sink
                            .call_log
                            .borrow()
                            .iter()
                            .any(|c| matches!(c, MethodCall::MarkModeResources { .. })),
                        "call_log must contain NO MarkModeResources entries"
                    );
                    cell_count += 1;
                }
            }
        }
    }
    assert_eq!(
        cell_count, 16,
        "must enumerate event_state × resource_presence — 2 × 2³ = 16 cells"
    );
}

/// PIN 4 — No active pane: returns false short-circuiting before any sink
/// method is called. The `let Some(pane_id) = input.active_pane_id else
/// return false` exit fires before reaching `mark_mode_active` check.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 4).
#[test]
fn dispatch_mark_mode_no_active_pane_returns_false_without_side_effects() {
    let mut cell_count = 0;
    for state in [ElementState::Pressed, ElementState::Released] {
        for mark_active in [false, true] {
            let mut sink = RecordingSink::default();
            let result = dispatch_with(
                &mut sink,
                DispatchCell {
                    state,
                    repeat: false,
                    active_pane_id: None,
                    mark_mode_active: mark_active,
                },
            );
            assert!(
                !result,
                "({state:?}, mark_active={mark_active}) None pane → must passthrough false"
            );
            assert!(
                sink.call_log.borrow().is_empty(),
                "({state:?}, mark_active={mark_active}) None pane → no sink method must fire"
            );
            assert!(
                sink.call_log
                    .borrow()
                    .iter()
                    .all(|c| !matches!(c, MethodCall::MarkModeResources { .. })),
                "no resource query"
            );
            cell_count += 1;
        }
    }
    assert_eq!(cell_count, 4, "must enumerate event_state × mark_active");
}

/// PIN 5 — `MarkAction::Handled { scroll_delta: Some(d) }` calls
/// `scroll_display(pane_id, d)`.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 5).
#[test]
fn dispatch_mark_mode_handled_with_scroll_delta_calls_scroll_display() {
    let pane_id = make_pane_id();
    let mut sink = RecordingSink {
        mux_present: true,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
    sink.snapshot_present.insert(pane_id, true);
    sink.dispatch_result = Some(MarkModeResult {
        action: MarkAction::Handled {
            scroll_delta: Some(5),
        },
        new_cursor: None,
        new_selection: None,
    });
    let result = dispatch_with(
        &mut sink,
        DispatchCell {
            state: ElementState::Pressed,
            repeat: false,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
    );
    assert!(result);
    assert_eq!(sink.scroll_display_calls, vec![(pane_id, 5)]);
    assert_eq!(sink.mark_dirty_calls, 1);
}

/// PIN 6 — `MarkAction::Handled { scroll_delta: None }` skips
/// `scroll_display`. Pins the `if let Some(delta)` guard.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 6).
#[test]
fn dispatch_mark_mode_handled_without_scroll_delta_skips_scroll_display() {
    let pane_id = make_pane_id();
    let mut sink = RecordingSink {
        mux_present: true,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
    sink.snapshot_present.insert(pane_id, true);
    sink.dispatch_result = Some(make_handled_no_motion());
    let result = dispatch_with(
        &mut sink,
        DispatchCell {
            state: ElementState::Pressed,
            repeat: false,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
    );
    assert!(result);
    assert!(sink.scroll_display_calls.is_empty());
    assert_eq!(sink.mark_dirty_calls, 1);
}

/// PIN 7 — `MarkAction::Exit { copy: true }` calls both `exit_mark_mode`
/// and `copy_selection`.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 7).
#[test]
fn dispatch_mark_mode_exit_with_copy_calls_exit_mark_mode_and_copy_selection() {
    let pane_id = make_pane_id();
    let mut sink = RecordingSink {
        mux_present: true,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
    sink.snapshot_present.insert(pane_id, true);
    sink.dispatch_result = Some(MarkModeResult {
        action: MarkAction::Exit { copy: true },
        new_cursor: None,
        new_selection: None,
    });
    let result = dispatch_with(
        &mut sink,
        DispatchCell {
            state: ElementState::Pressed,
            repeat: false,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
    );
    assert!(result);
    assert_eq!(sink.exit_mark_mode_calls, vec![pane_id]);
    assert_eq!(sink.copy_selection_calls, 1);
    assert_eq!(sink.mark_dirty_calls, 1);
}

/// PIN 8 — `MarkAction::Exit { copy: false }` calls `exit_mark_mode` only.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 8).
#[test]
fn dispatch_mark_mode_exit_without_copy_calls_exit_mark_mode_only() {
    let pane_id = make_pane_id();
    let mut sink = RecordingSink {
        mux_present: true,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
    sink.snapshot_present.insert(pane_id, true);
    sink.dispatch_result = Some(MarkModeResult {
        action: MarkAction::Exit { copy: false },
        new_cursor: None,
        new_selection: None,
    });
    let result = dispatch_with(
        &mut sink,
        DispatchCell {
            state: ElementState::Pressed,
            repeat: false,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
    );
    assert!(result);
    assert_eq!(sink.exit_mark_mode_calls, vec![pane_id]);
    assert_eq!(sink.copy_selection_calls, 0);
    assert_eq!(sink.mark_dirty_calls, 1);
}

/// PIN 9 — `MarkAction::Ignored` only fires `mark_dirty`.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 9).
#[test]
fn dispatch_mark_mode_ignored_action_only_marks_dirty() {
    let pane_id = make_pane_id();
    let mut sink = RecordingSink {
        mux_present: true,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
    sink.snapshot_present.insert(pane_id, true);
    sink.dispatch_result = Some(MarkModeResult {
        action: MarkAction::Ignored,
        new_cursor: None,
        new_selection: None,
    });
    let result = dispatch_with(
        &mut sink,
        DispatchCell {
            state: ElementState::Pressed,
            repeat: false,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
    );
    assert!(result);
    assert!(sink.exit_mark_mode_calls.is_empty());
    assert!(sink.scroll_display_calls.is_empty());
    assert_eq!(sink.copy_selection_calls, 0);
    assert_eq!(sink.mark_dirty_calls, 1);
}

/// PIN 10 — `result.new_cursor = Some(c)` calls `set_mark_cursor(pane, c)`.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 10).
#[test]
fn dispatch_mark_mode_with_new_cursor_calls_set_mark_cursor() {
    let pane_id = make_pane_id();
    let mut sink = RecordingSink {
        mux_present: true,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
    sink.snapshot_present.insert(pane_id, true);
    let new_cursor = MarkCursor {
        row: StableRowIndex(2),
        col: 7,
    };
    sink.dispatch_result = Some(MarkModeResult {
        action: MarkAction::Handled { scroll_delta: None },
        new_cursor: Some(new_cursor),
        new_selection: None,
    });
    let _ = dispatch_with(
        &mut sink,
        DispatchCell {
            state: ElementState::Pressed,
            repeat: false,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
    );
    assert_eq!(sink.set_mark_cursor_calls, vec![(pane_id, new_cursor)]);
}

/// PIN 11 — `result.new_cursor = None` skips `set_mark_cursor`.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 11).
#[test]
fn dispatch_mark_mode_without_new_cursor_skips_set_mark_cursor() {
    let pane_id = make_pane_id();
    let mut sink = RecordingSink {
        mux_present: true,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
    sink.snapshot_present.insert(pane_id, true);
    sink.dispatch_result = Some(make_handled_no_motion());
    let _ = dispatch_with(
        &mut sink,
        DispatchCell {
            state: ElementState::Pressed,
            repeat: false,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
    );
    assert!(sink.set_mark_cursor_calls.is_empty());
}

/// PIN 12 — `SelectionUpdate::Set(s)` calls `set_pane_selection(pane, s)`.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 12).
#[test]
fn dispatch_mark_mode_with_selection_set_calls_set_pane_selection() {
    let pane_id = make_pane_id();
    let mut sink = RecordingSink {
        mux_present: true,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
    sink.snapshot_present.insert(pane_id, true);
    let sel = make_selection();
    sink.dispatch_result = Some(MarkModeResult {
        action: MarkAction::Handled { scroll_delta: None },
        new_cursor: None,
        new_selection: Some(SelectionUpdate::Set(sel)),
    });
    let _ = dispatch_with(
        &mut sink,
        DispatchCell {
            state: ElementState::Pressed,
            repeat: false,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
    );
    assert_eq!(sink.set_pane_selection_calls, vec![(pane_id, sel)]);
    assert!(sink.clear_pane_selection_calls.is_empty());
}

/// PIN 13 — `SelectionUpdate::Clear` calls `clear_pane_selection(pane)`.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 13).
#[test]
fn dispatch_mark_mode_with_selection_clear_calls_clear_pane_selection() {
    let pane_id = make_pane_id();
    let mut sink = RecordingSink {
        mux_present: true,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
    sink.snapshot_present.insert(pane_id, true);
    sink.dispatch_result = Some(MarkModeResult {
        action: MarkAction::Handled { scroll_delta: None },
        new_cursor: None,
        new_selection: Some(SelectionUpdate::Clear),
    });
    let _ = dispatch_with(
        &mut sink,
        DispatchCell {
            state: ElementState::Pressed,
            repeat: false,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
    );
    assert_eq!(sink.clear_pane_selection_calls, vec![pane_id]);
    assert!(sink.set_pane_selection_calls.is_empty());
}

/// PIN 13B — `result.new_selection = None` skips both selection mutations.
/// Pins the outer-Some guard around the SelectionUpdate match.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 13B).
#[test]
fn dispatch_mark_mode_with_selection_none_skips_set_and_clear() {
    let pane_id = make_pane_id();
    let mut sink = RecordingSink {
        mux_present: true,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
    sink.snapshot_present.insert(pane_id, true);
    sink.dispatch_result = Some(make_handled_no_motion());
    let _ = dispatch_with(
        &mut sink,
        DispatchCell {
            state: ElementState::Pressed,
            repeat: false,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
    );
    assert!(sink.set_pane_selection_calls.is_empty());
    assert!(sink.clear_pane_selection_calls.is_empty());
}

/// PIN 14a — Snapshot refresh strictly precedes the resource query. Pins
/// that a stale-snapshot cycle gets a chance to refill before being
/// declared missing.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 14).
#[test]
fn dispatch_mark_mode_pressed_refreshes_snapshot_before_mark_mode_resources_query() {
    let pane_id = make_pane_id();
    let mut sink = RecordingSink {
        mux_present: true,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
    sink.snapshot_present.insert(pane_id, true);
    sink.dispatch_result = Some(make_handled_no_motion());
    let _ = dispatch_with(
        &mut sink,
        DispatchCell {
            state: ElementState::Pressed,
            repeat: false,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
    );
    let log = sink.call_log_snapshot();
    let refresh_idx = log
        .iter()
        .position(|c| matches!(c, MethodCall::RefreshPaneSnapshot { .. }));
    let resources_idx = log
        .iter()
        .position(|c| matches!(c, MethodCall::MarkModeResources { .. }));
    assert_eq!(
        log.iter()
            .filter(|c| matches!(c, MethodCall::RefreshPaneSnapshot { .. }))
            .count(),
        1,
        "must refresh exactly once"
    );
    assert_eq!(
        log.iter()
            .filter(|c| matches!(c, MethodCall::MarkModeResources { .. }))
            .count(),
        1,
        "must query resources exactly once"
    );
    assert!(
        refresh_idx.unwrap() < resources_idx.unwrap(),
        "RefreshPaneSnapshot must precede MarkModeResources query"
    );
}

/// PIN 14b — Refresh promoting an absent snapshot to present allows
/// dispatch to proceed. Pins that refresh ordering matters semantically.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 14).
#[test]
fn dispatch_mark_mode_pressed_refresh_can_promote_snapshot_present() {
    let pane_id = make_pane_id();
    let mut sink = RecordingSink {
        mux_present: true,
        refresh_makes_snapshot_present: true,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
    sink.snapshot_present.insert(pane_id, false);
    sink.dispatch_result = Some(make_handled_no_motion());
    let result = dispatch_with(
        &mut sink,
        DispatchCell {
            state: ElementState::Pressed,
            repeat: false,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
    );
    assert!(result, "refresh promoted snapshot → must proceed");
    assert!(sink.exit_mark_mode_calls.is_empty());
    assert_eq!(sink.dispatch_key_calls, 1);
    assert_eq!(sink.mark_dirty_calls, 1);
}

/// PIN 15 — Negative pin: missing-mux must NOT silently consume the
/// keystroke. The combination `return false AND exit_mark_mode called` IS
/// the BUG-08-031 contract; a regression that returns true would cause the
/// caller's `if x { return; }` to swallow the keystroke.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 15).
/// Anti-regression for: bug-tracker/plans/completed/BUG-08-031/.
#[test]
fn dispatch_mark_mode_missing_mux_does_not_silently_consume_keystroke() {
    let pane_id = make_pane_id();
    let mut sink = RecordingSink {
        mux_present: false,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
    sink.snapshot_present.insert(pane_id, true);
    let result = dispatch_with(
        &mut sink,
        DispatchCell {
            state: ElementState::Pressed,
            repeat: false,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
    );
    assert!(
        !result,
        "missing mux must return false (NOT silently consume)"
    );
    assert_eq!(
        sink.exit_mark_mode_calls,
        vec![pane_id],
        "missing mux must exit mark mode"
    );
    assert_eq!(
        sink.dispatch_key_calls, 0,
        "missing mux must NOT dispatch key"
    );
    assert_eq!(sink.mark_dirty_calls, 0, "missing mux must NOT mark dirty");
}

/// PIN 16a — Dispatch-key argument propagation. Pins that all 5 inputs
/// (pane, cursor, selection, event, modifiers) flow unchanged into
/// `dispatch_mark_mode_key`. Drives non-trivial modifiers
/// (CONTROL | SHIFT) so a regression that drops modifiers would fail —
/// passing `ModifiersState::empty()` would produce a tautological in==out
/// match.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 16).
#[test]
fn dispatch_mark_mode_propagates_args_to_dispatch_mark_mode_key() {
    let pane_id = make_pane_id();
    let known_cursor = MarkCursor {
        row: StableRowIndex(3),
        col: 9,
    };
    let known_selection = make_selection();
    let known_modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
    let mut sink = RecordingSink {
        mux_present: true,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, known_cursor);
    sink.selection_for_pane.insert(pane_id, known_selection);
    sink.snapshot_present.insert(pane_id, true);
    sink.dispatch_result = Some(make_handled_no_motion());
    let _ = dispatch_mark_mode(
        MarkModeDispatch {
            event: make_mark_key_event(),
            event_state: ElementState::Pressed,
            event_repeat: false,
            modifiers: known_modifiers,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
        &mut sink,
    );
    assert_eq!(
        sink.last_dispatch_args,
        Some((
            pane_id,
            known_cursor,
            Some(known_selection),
            make_mark_key_event(),
            known_modifiers,
        )),
        "all 5 inputs must propagate unchanged"
    );
}

/// PIN 16b — `Option<Selection> = None` propagates correctly. Companion
/// to PIN 16a; pins that absence flows through the dispatch chain.
///
/// See: bug-tracker/plans/BUG-08-034/section-03-tdd-matrix.md (Pin 16).
#[test]
fn dispatch_mark_mode_propagates_none_selection() {
    let pane_id = make_pane_id();
    let mut sink = RecordingSink {
        mux_present: true,
        ..Default::default()
    };
    sink.cursor_for_pane.insert(pane_id, make_mark_cursor());
    sink.snapshot_present.insert(pane_id, true);
    sink.dispatch_result = Some(make_handled_no_motion());
    let _ = dispatch_with(
        &mut sink,
        DispatchCell {
            state: ElementState::Pressed,
            repeat: false,
            active_pane_id: Some(pane_id),
            mark_mode_active: true,
        },
    );
    assert_eq!(
        sink.last_dispatch_args.as_ref().unwrap().2,
        None,
        "absent selection must propagate as None"
    );
}
