//! Tests for compositor-driven tab slide animations.

use std::time::{Duration, Instant};

use crate::compositor::layer_animator::LayerAnimator;
use crate::compositor::layer_tree::LayerTree;
use crate::geometry::Rect;

use super::{SlideContext, TabBarWidget, TabSlideState};

/// Creates a tree + animator for testing.
fn make_test_env() -> (LayerTree, LayerAnimator) {
    let tree = LayerTree::new(Rect::new(0.0, 0.0, 1200.0, 46.0));
    let animator = LayerAnimator::new();
    (tree, animator)
}

#[test]
fn new_state_has_no_active() {
    let state = TabSlideState::new();
    assert!(!state.has_active());
}

#[test]
fn close_creates_layers_for_displaced_tabs() {
    let mut state = TabSlideState::new();
    let (mut tree, mut animator) = make_test_env();
    let now = Instant::now();
    let mut cx = SlideContext {
        tree: &mut tree,
        animator: &mut animator,
        now,
    };

    // Close tab 1 out of 4 remaining tabs → tabs 1,2,3 slide.
    state.start_close_slide(1, 200.0, 4, &mut cx);

    assert!(state.has_active());
    // Should have 3 active animations (indices 1, 2, 3).
    assert_eq!(state.active.len(), 3);
}

#[test]
fn close_last_index_creates_no_layers() {
    let mut state = TabSlideState::new();
    let (mut tree, mut animator) = make_test_env();
    let now = Instant::now();
    let mut cx = SlideContext {
        tree: &mut tree,
        animator: &mut animator,
        now,
    };

    // Close at index == tab_count → range is empty.
    state.start_close_slide(4, 200.0, 4, &mut cx);

    assert!(!state.has_active());
}

#[test]
fn reorder_creates_layers() {
    let mut state = TabSlideState::new();
    let (mut tree, mut animator) = make_test_env();
    let now = Instant::now();
    let mut cx = SlideContext {
        tree: &mut tree,
        animator: &mut animator,
        now,
    };

    // Move tab 0 → tab 2: indices 0, 1 get +tab_width.
    state.start_reorder_slide(0, 2, 200.0, &mut cx);

    assert!(state.has_active());
    assert_eq!(state.active.len(), 2);
}

#[test]
fn reorder_same_index_is_noop() {
    let mut state = TabSlideState::new();
    let (mut tree, mut animator) = make_test_env();
    let now = Instant::now();
    let mut cx = SlideContext {
        tree: &mut tree,
        animator: &mut animator,
        now,
    };

    state.start_reorder_slide(2, 2, 200.0, &mut cx);

    assert!(!state.has_active());
}

#[test]
fn reorder_direction_from_greater_than_to() {
    let mut state = TabSlideState::new();
    let (mut tree, mut animator) = make_test_env();
    let now = Instant::now();
    let mut cx = SlideContext {
        tree: &mut tree,
        animator: &mut animator,
        now,
    };

    // Move tab 3 → tab 1: indices 2, 3 get -tab_width.
    state.start_reorder_slide(3, 1, 200.0, &mut cx);

    assert!(state.has_active());
    assert_eq!(state.active.len(), 2);

    // Verify layers have negative initial translation.
    for &layer_id in state.active.values() {
        let layer = tree.get(layer_id).unwrap();
        assert!(
            layer.properties().transform.translation_x() < 0.0,
            "from > to should create negative offset"
        );
    }
}

#[test]
fn cleanup_removes_finished_layers() {
    let mut state = TabSlideState::new();
    let (mut tree, mut animator) = make_test_env();
    let now = Instant::now();
    let mut cx = SlideContext {
        tree: &mut tree,
        animator: &mut animator,
        now,
    };

    state.start_close_slide(0, 200.0, 2, &mut cx);
    assert!(state.has_active());

    // Tick past the animation duration.
    let after = now + Duration::from_millis(200);
    animator.tick(&mut tree, after);

    state.cleanup(&mut tree, &animator);
    assert!(!state.has_active());
}

#[test]
fn sync_populates_offsets() {
    let mut state = TabSlideState::new();
    let (mut tree, mut animator) = make_test_env();
    let now = Instant::now();
    let mut widget = TabBarWidget::new(1200.0);
    let mut cx = SlideContext {
        tree: &mut tree,
        animator: &mut animator,
        now,
    };

    state.start_close_slide(1, 200.0, 3, &mut cx);

    // Before any tick, transforms are at their initial values.
    state.sync_to_widget(3, &tree, &mut widget);

    // Tab 0 should have no offset; tabs 1 and 2 should have 200px.
    let mut readback = Vec::new();
    widget.swap_anim_offsets(&mut readback);
    assert_eq!(readback.len(), 3);
    assert!((readback[0] - 0.0).abs() < f32::EPSILON, "tab 0 untouched");
    assert!(
        (readback[1] - 200.0).abs() < f32::EPSILON,
        "tab 1 at initial offset: got {}",
        readback[1]
    );
    assert!(
        (readback[2] - 200.0).abs() < f32::EPSILON,
        "tab 2 at initial offset: got {}",
        readback[2]
    );
}

#[test]
fn sync_idle_is_noop() {
    let mut state = TabSlideState::new();
    let tree = LayerTree::new(Rect::new(0.0, 0.0, 1200.0, 46.0));
    let mut widget = TabBarWidget::new(1200.0);

    state.sync_to_widget(3, &tree, &mut widget);

    // All offsets should be zero.
    let mut readback = Vec::new();
    widget.swap_anim_offsets(&mut readback);
    assert_eq!(readback.len(), 3);
    assert!(readback.iter().all(|&v| v == 0.0));
}

#[test]
fn cancel_removes_all() {
    let mut state = TabSlideState::new();
    let (mut tree, mut animator) = make_test_env();
    let now = Instant::now();
    let mut cx = SlideContext {
        tree: &mut tree,
        animator: &mut animator,
        now,
    };

    state.start_close_slide(0, 200.0, 3, &mut cx);
    assert!(state.has_active());

    state.cancel_all(&mut tree, &mut animator);
    assert!(!state.has_active());
    assert!(!animator.is_any_animating());
}

#[test]
fn rapid_close_cancels_previous() {
    let mut state = TabSlideState::new();
    let (mut tree, mut animator) = make_test_env();
    let now = Instant::now();
    let mut cx = SlideContext {
        tree: &mut tree,
        animator: &mut animator,
        now,
    };

    // First close.
    state.start_close_slide(0, 200.0, 4, &mut cx);
    let first_count = state.active.len();
    assert_eq!(first_count, 4);

    // Rapid second close (before first finishes) — should cancel previous.
    let slightly_later = now + Duration::from_millis(10);
    let mut cx2 = SlideContext {
        tree: &mut tree,
        animator: &mut animator,
        now: slightly_later,
    };
    state.start_close_slide(1, 200.0, 3, &mut cx2);

    // Previous animations were cancelled; new set started.
    assert_eq!(state.active.len(), 2);
}

#[test]
fn close_slide_mid_animation_offset_decreasing() {
    let mut state = TabSlideState::new();
    let (mut tree, mut animator) = make_test_env();
    let now = Instant::now();
    let mut cx = SlideContext {
        tree: &mut tree,
        animator: &mut animator,
        now,
    };

    state.start_close_slide(0, 200.0, 2, &mut cx);

    // Tick to ~50% through the animation.
    let mid = now + Duration::from_millis(75);
    animator.tick(&mut tree, mid);

    // Both layers should have translation_x between 0 and 200.
    for &layer_id in state.active.values() {
        let tx = tree
            .get(layer_id)
            .unwrap()
            .properties()
            .transform
            .translation_x();
        assert!(
            tx > 0.0 && tx < 200.0,
            "mid-animation offset should be between 0 and 200, got {tx}"
        );
    }
}
