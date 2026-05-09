//! Unit tests for the SSOT helpers in `resolve/mod.rs`.
//!
//! The `block_cursor_color_exclusion_active` helper is the single decision
//! point for "should per-cell selection / search / inverse re-coloring be
//! suppressed on the cursor cell?", consumed by both `resolve_cell_colors`
//! and `prepare::evaluate_row_state_change`.

use oriterm_core::{Column, CursorShape, RenderableCursor};

use super::{BLOCK_CURSOR_OPACITY_THRESHOLD, block_cursor_color_exclusion_active};

fn cursor(shape: CursorShape, visible: bool) -> RenderableCursor {
    RenderableCursor {
        line: 0,
        column: Column(0),
        shape,
        visible,
    }
}

/// Visible Block cursor with opacity above threshold suppresses per-cell
/// inversion — the cursor sprite dominates visually.
#[test]
fn helper_returns_true_for_visible_block_above_threshold() {
    assert!(block_cursor_color_exclusion_active(
        &cursor(CursorShape::Block, true),
        0.6
    ));
}

/// Visible Block cursor with opacity below threshold does NOT suppress
/// inversion — the cursor is fading out and per-cell colors should
/// revert to normal for readability.
#[test]
fn helper_returns_false_for_visible_block_below_threshold() {
    assert!(!block_cursor_color_exclusion_active(
        &cursor(CursorShape::Block, true),
        0.4
    ));
}

/// Invisible cursor cannot dominate the cell visually regardless of
/// opacity, so the suppression is inactive.
#[test]
fn helper_returns_false_for_invisible_cursor_regardless_of_opacity() {
    for opacity in [0.0_f32, 0.4, 0.6, 1.0] {
        assert!(
            !block_cursor_color_exclusion_active(&cursor(CursorShape::Block, false), opacity),
            "opacity {opacity} should not suppress when cursor invisible"
        );
    }
}

/// Non-Block cursor shapes never trigger the suppression — only Block
/// cursors visually dominate the underlying cell. Beam / Underline /
/// HollowBlock / Hidden all return false even at high opacity.
#[test]
fn helper_returns_false_for_non_block_shape_above_threshold() {
    for shape in [
        CursorShape::Bar,
        CursorShape::Underline,
        CursorShape::HollowBlock,
        CursorShape::Hidden,
    ] {
        assert!(
            !block_cursor_color_exclusion_active(&cursor(shape, true), 0.7),
            "shape {shape:?} at opacity 0.7 should not suppress"
        );
    }
}

/// Threshold constant value pin — protects against accidental change.
/// The 0.5 boundary is shared between the helper and downstream gating
/// logic; widening or narrowing it would silently change rendering
/// behavior across the entire fast-path / full-prepare decision.
#[test]
fn helper_threshold_constant_value_pin() {
    assert_eq!(BLOCK_CURSOR_OPACITY_THRESHOLD, 0.5_f32);
}

/// Boundary case: opacity exactly equal to threshold returns false (uses
/// strictly greater-than `>`), and crossing in either direction is
/// detectable for the row-state gate.
#[test]
fn helper_returns_false_at_exact_threshold() {
    assert!(!block_cursor_color_exclusion_active(
        &cursor(CursorShape::Block, true),
        0.5
    ));
    // Just above: true.
    assert!(block_cursor_color_exclusion_active(
        &cursor(CursorShape::Block, true),
        0.500_001
    ));
    // Just below: false.
    assert!(!block_cursor_color_exclusion_active(
        &cursor(CursorShape::Block, true),
        0.499_999
    ));
}
