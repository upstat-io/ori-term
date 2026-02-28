use oriterm_ui::widgets::tab_bar::constants::{
    TAB_BAR_HEIGHT, TAB_LEFT_MARGIN, TEAR_OFF_THRESHOLD, TEAR_OFF_THRESHOLD_UP,
};

use super::{
    DragPhase, TabDragState, compute_drag_visual_x, compute_insertion_index, exceeds_tear_off,
};

// -- compute_drag_visual_x --

#[test]
fn drag_visual_x_preserves_offset() {
    // Cursor at 200, offset 50, max 500 → visual at 150.
    assert!((compute_drag_visual_x(200.0, 50.0, 500.0) - 150.0).abs() < f32::EPSILON);
}

#[test]
fn drag_visual_x_clamps_to_zero() {
    // Cursor at 10, offset 50 → would be -40, clamps to 0.
    assert!((compute_drag_visual_x(10.0, 50.0, 500.0)).abs() < f32::EPSILON);
}

#[test]
fn drag_visual_x_clamps_to_max() {
    // Cursor at 600, offset 10, max 500 → would be 590, clamps to 500.
    assert!((compute_drag_visual_x(600.0, 10.0, 500.0) - 500.0).abs() < f32::EPSILON);
}

// -- compute_insertion_index --

#[test]
fn insertion_index_first_slot() {
    // Tab center falls in first slot.
    let visual_x = TAB_LEFT_MARGIN;
    assert_eq!(compute_insertion_index(visual_x, 120.0, 5), 0);
}

#[test]
fn insertion_index_middle_slot() {
    // Tab center should map to the 2nd tab (index 1).
    let visual_x = TAB_LEFT_MARGIN + 120.0;
    assert_eq!(compute_insertion_index(visual_x, 120.0, 5), 1);
}

#[test]
fn insertion_index_last_slot() {
    // Visual X far to the right — clamps to last valid index.
    let visual_x = TAB_LEFT_MARGIN + 4.0 * 120.0 + 50.0;
    assert_eq!(compute_insertion_index(visual_x, 120.0, 5), 4);
}

#[test]
fn insertion_index_clamps_to_last() {
    // Way beyond the last tab.
    assert_eq!(compute_insertion_index(2000.0, 120.0, 3), 2);
}

#[test]
fn insertion_index_single_tab() {
    // Single tab → always index 0.
    assert_eq!(compute_insertion_index(0.0, 120.0, 1), 0);
    assert_eq!(compute_insertion_index(500.0, 120.0, 1), 0);
}

#[test]
fn insertion_index_zero_tabs() {
    // Edge case: zero tabs → 0.
    assert_eq!(compute_insertion_index(100.0, 120.0, 0), 0);
}

#[test]
fn insertion_index_zero_width() {
    // Edge case: zero width → 0.
    assert_eq!(compute_insertion_index(100.0, 0.0, 5), 0);
}

// -- exceeds_tear_off --

#[test]
fn tear_off_above_bar_within_threshold() {
    let bar_y = 10.0;
    let bar_bottom = bar_y + TAB_BAR_HEIGHT;
    // Just at the threshold edge (not exceeded).
    let cursor_y = bar_y - TEAR_OFF_THRESHOLD_UP;
    assert!(!exceeds_tear_off(cursor_y, bar_y, bar_bottom));
}

#[test]
fn tear_off_above_bar_exceeds_threshold() {
    let bar_y = 10.0;
    let bar_bottom = bar_y + TAB_BAR_HEIGHT;
    // One pixel beyond.
    let cursor_y = bar_y - TEAR_OFF_THRESHOLD_UP - 1.0;
    assert!(exceeds_tear_off(cursor_y, bar_y, bar_bottom));
}

#[test]
fn tear_off_below_bar_within_threshold() {
    let bar_y = 10.0;
    let bar_bottom = bar_y + TAB_BAR_HEIGHT;
    let cursor_y = bar_bottom + TEAR_OFF_THRESHOLD;
    assert!(!exceeds_tear_off(cursor_y, bar_y, bar_bottom));
}

#[test]
fn tear_off_below_bar_exceeds_threshold() {
    let bar_y = 10.0;
    let bar_bottom = bar_y + TAB_BAR_HEIGHT;
    let cursor_y = bar_bottom + TEAR_OFF_THRESHOLD + 1.0;
    assert!(exceeds_tear_off(cursor_y, bar_y, bar_bottom));
}

#[test]
fn tear_off_within_bar_never_exceeds() {
    let bar_y = 10.0;
    let bar_bottom = bar_y + TAB_BAR_HEIGHT;
    // Dead center of bar.
    assert!(!exceeds_tear_off(
        bar_y + TAB_BAR_HEIGHT / 2.0,
        bar_y,
        bar_bottom
    ));
    // At top edge.
    assert!(!exceeds_tear_off(bar_y, bar_y, bar_bottom));
    // At bottom edge.
    assert!(!exceeds_tear_off(bar_bottom, bar_y, bar_bottom));
}

// -- TabDragState construction --

#[test]
fn drag_state_construction() {
    let state = TabDragState {
        tab_id: oriterm_mux::TabId::from_raw(42),
        original_index: 2,
        current_index: 2,
        origin_x: 100.0,
        origin_y: 30.0,
        phase: DragPhase::Pending,
        mouse_offset_in_tab: 25.0,
        tab_bar_y: 10.0,
        tab_bar_bottom: 10.0 + TAB_BAR_HEIGHT,
    };
    assert_eq!(state.phase, DragPhase::Pending);
    assert_eq!(state.original_index, 2);
    assert_eq!(state.current_index, 2);
    assert!((state.mouse_offset_in_tab - 25.0).abs() < f32::EPSILON);
}

// -- Directional threshold asymmetry --

#[test]
fn tear_off_upward_more_sensitive_than_downward() {
    // Verify the design: upward threshold is smaller.
    assert!(TEAR_OFF_THRESHOLD_UP < TEAR_OFF_THRESHOLD);
}
