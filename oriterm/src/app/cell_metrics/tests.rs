//! Tests for cell-metric broadcast decision logic.
//!
//! Covers the pure `cell_metric_broadcast_needed` helper. Full
//! integration of `broadcast_cell_metrics_to_window` (App → session →
//! mux) requires a full `App` fixture, which is exercised by GPU /
//! end-to-end test suites. The helper extraction keeps the
//! short-circuit rule unit-testable independent of that scaffolding
//! (TPR-07-002-codex / TPR-07-001-gemini round 7).

use super::cell_metric_broadcast_needed;

/// First broadcast (no prior state) must fire.
#[test]
fn first_broadcast_always_fires() {
    assert!(cell_metric_broadcast_needed(None, (8, 16)));
    assert!(cell_metric_broadcast_needed(None, (16, 32)));
}

/// Identical dims short-circuit.
#[test]
fn identical_dims_short_circuit() {
    assert!(!cell_metric_broadcast_needed(Some((8, 16)), (8, 16)));
    assert!(!cell_metric_broadcast_needed(Some((16, 32)), (16, 32)));
}

/// Width change fires a broadcast.
#[test]
fn width_change_fires() {
    assert!(cell_metric_broadcast_needed(Some((8, 16)), (10, 16)));
}

/// Height change fires a broadcast.
#[test]
fn height_change_fires() {
    assert!(cell_metric_broadcast_needed(Some((8, 16)), (8, 20)));
}

/// Both-axis change fires a broadcast.
#[test]
fn both_axis_change_fires() {
    assert!(cell_metric_broadcast_needed(Some((8, 16)), (16, 32)));
}

/// Negative pin: any non-None equal value must NOT fire, even for
/// degenerate `(1, 1)` dims that happen after a font-size clamp.
#[test]
fn negative_pin_degenerate_dims_short_circuit() {
    assert!(!cell_metric_broadcast_needed(Some((1, 1)), (1, 1)));
}
