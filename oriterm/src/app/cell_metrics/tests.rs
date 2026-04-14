//! Tests for cell-metric broadcast decision logic.
//!
//! Covers the pure `cell_metric_broadcast_needed` helper. Full
//! integration of `broadcast_cell_metrics_to_window` (App → session →
//! mux) requires a full `App` fixture, which is exercised by GPU /
//! end-to-end test suites. The helper extraction keeps the
//! short-circuit rule unit-testable independent of that scaffolding
//! (TPR-07-002-codex / TPR-07-001-gemini round 7).

use super::{cell_metric_broadcast_needed, try_claim_broadcast};

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

// ── try_claim_broadcast: decision + state update fused ────────────

/// First broadcast (cache is None) claims the slot AND updates the cache.
/// Pins that `try_claim_broadcast` does the state assignment — a
/// refactor that removes `*cached = Some(new)` fails this test.
#[test]
fn try_claim_broadcast_updates_cache_on_first_call() {
    let mut cached = None;
    assert!(try_claim_broadcast(&mut cached, (8, 16)));
    assert_eq!(
        cached,
        Some((8, 16)),
        "cache must be updated to the claimed dims"
    );
}

/// Second call with identical dims short-circuits AND leaves cache intact.
#[test]
fn try_claim_broadcast_short_circuits_and_leaves_cache() {
    let mut cached = Some((8, 16));
    assert!(!try_claim_broadcast(&mut cached, (8, 16)));
    assert_eq!(
        cached,
        Some((8, 16)),
        "cache must be unchanged on short-circuit"
    );
}

/// Changed dims claim the slot and UPDATE the cache to the new value.
#[test]
fn try_claim_broadcast_updates_cache_on_change() {
    let mut cached = Some((8, 16));
    assert!(try_claim_broadcast(&mut cached, (16, 32)));
    assert_eq!(cached, Some((16, 32)), "cache must update to the new dims");
}

/// Sequence: None → (8,16) → (8,16) → (16,32) — verifies the state
/// machine across multiple calls. A refactor that skips the
/// assignment on "change" path would fail here because the third
/// `try_claim_broadcast` would erroneously claim again.
#[test]
fn try_claim_broadcast_full_sequence() {
    let mut cached = None;
    assert!(try_claim_broadcast(&mut cached, (8, 16)));
    assert_eq!(cached, Some((8, 16)));
    assert!(!try_claim_broadcast(&mut cached, (8, 16)));
    assert_eq!(cached, Some((8, 16)));
    assert!(try_claim_broadcast(&mut cached, (16, 32)));
    assert_eq!(cached, Some((16, 32)));
    assert!(!try_claim_broadcast(&mut cached, (16, 32)));
    assert_eq!(cached, Some((16, 32)));
}
