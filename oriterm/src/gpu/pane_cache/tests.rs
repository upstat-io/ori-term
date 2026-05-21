//! Tests for `PaneRenderCache`.

use oriterm_core::Rgb;
use oriterm_mux::id::PaneId;

use crate::session::{PaneLayout, Rect};

use super::PaneRenderCache;
use crate::gpu::frame_input::ViewportSize;
use crate::gpu::instance_writer::ScreenRect;
use crate::gpu::prepared_frame::PreparedFrame;

fn make_layout(pane_id: PaneId, x: f32, y: f32, w: f32, h: f32) -> PaneLayout {
    PaneLayout {
        pane_id,
        pixel_rect: Rect {
            x,
            y,
            width: w,
            height: h,
        },
        cols: (w / 8.0) as u16,
        rows: (h / 16.0) as u16,
        is_focused: true,
        is_floating: false,
    }
}

/// Push a single background rect so we can detect whether prepare_fn was called.
fn push_marker(frame: &mut PreparedFrame, x: f32) {
    frame.backgrounds.push_rect(
        ScreenRect {
            x,
            y: 0.0,
            w: 8.0,
            h: 16.0,
        },
        Rgb { r: 255, g: 0, b: 0 },
        1.0,
    );
}

#[test]
fn clean_pane_returns_cached_frame() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout = make_layout(id, 0.0, 0.0, 640.0, 480.0);

    // First call: dirty=true → prepare_fn is called.
    let mut called = false;
    let frame = cache.get_or_prepare(id, &layout, true, 0, |f| {
        called = true;
        push_marker(f, 42.0);
    });
    assert!(called, "prepare_fn should be called on first access");
    assert_eq!(frame.backgrounds.len(), 1);

    // Second call: dirty=false, same layout → cached, prepare_fn NOT called.
    let mut called = false;
    let frame = cache.get_or_prepare(id, &layout, false, 0, |_f| {
        called = true;
    });
    assert!(!called, "prepare_fn should NOT be called for clean pane");
    assert_eq!(frame.backgrounds.len(), 1, "cached frame preserved");
}

#[test]
fn dirty_pane_calls_prepare_fn() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout = make_layout(id, 0.0, 0.0, 640.0, 480.0);

    // Seed cache.
    cache.get_or_prepare(id, &layout, true, 0, |f| push_marker(f, 1.0));

    // Dirty=true → re-prepare.
    let mut called = false;
    let frame = cache.get_or_prepare(id, &layout, true, 0, |f| {
        called = true;
        push_marker(f, 2.0);
        push_marker(f, 3.0);
    });
    assert!(called, "prepare_fn should be called for dirty pane");
    assert_eq!(frame.backgrounds.len(), 2, "old instances replaced");
}

#[test]
fn layout_change_triggers_reprepare() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout_a = make_layout(id, 0.0, 0.0, 640.0, 480.0);
    let layout_b = make_layout(id, 0.0, 0.0, 800.0, 600.0);

    // Seed cache with layout_a.
    cache.get_or_prepare(id, &layout_a, true, 0, |f| push_marker(f, 1.0));

    // Clean but layout changed → re-prepare.
    let mut called = false;
    let frame = cache.get_or_prepare(id, &layout_b, false, 0, |f| {
        called = true;
        push_marker(f, 2.0);
    });
    assert!(called, "layout change should trigger re-prepare");
    assert_eq!(frame.backgrounds.len(), 1);
}

#[test]
fn invalidate_all_forces_reprepare() {
    let mut cache = PaneRenderCache::new();
    let id1 = PaneId::from_raw(1);
    let id2 = PaneId::from_raw(2);
    let layout1 = make_layout(id1, 0.0, 0.0, 640.0, 480.0);
    let layout2 = make_layout(id2, 640.0, 0.0, 640.0, 480.0);

    // Seed cache for both panes.
    cache.get_or_prepare(id1, &layout1, true, 0, |f| push_marker(f, 1.0));
    cache.get_or_prepare(id2, &layout2, true, 0, |f| push_marker(f, 2.0));

    cache.invalidate_all();

    // Both panes should re-prepare despite dirty=false, same layout.
    let mut called1 = false;
    cache.get_or_prepare(id1, &layout1, false, 0, |f| {
        called1 = true;
        push_marker(f, 10.0);
    });
    let mut called2 = false;
    cache.get_or_prepare(id2, &layout2, false, 0, |f| {
        called2 = true;
        push_marker(f, 20.0);
    });
    assert!(called1, "pane 1 should re-prepare after invalidate_all");
    assert!(called2, "pane 2 should re-prepare after invalidate_all");
}

#[test]
fn remove_frees_entry() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout = make_layout(id, 0.0, 0.0, 640.0, 480.0);

    cache.get_or_prepare(id, &layout, true, 0, |f| push_marker(f, 1.0));
    cache.remove(id);

    // Next access should call prepare_fn (entry gone).
    let mut called = false;
    cache.get_or_prepare(id, &layout, false, 0, |f| {
        called = true;
        push_marker(f, 2.0);
    });
    assert!(called, "removed pane should re-prepare");
}

#[test]
fn extend_from_merges_cached_frames() {
    let mut cache = PaneRenderCache::new();
    let id1 = PaneId::from_raw(1);
    let id2 = PaneId::from_raw(2);
    let layout1 = make_layout(id1, 0.0, 0.0, 320.0, 240.0);
    let layout2 = make_layout(id2, 320.0, 0.0, 320.0, 240.0);

    cache.get_or_prepare(id1, &layout1, true, 0, |f| {
        push_marker(f, 0.0);
        push_marker(f, 8.0);
    });
    cache.get_or_prepare(id2, &layout2, true, 0, |f| {
        push_marker(f, 320.0);
    });

    // Merge both cached frames into a main frame.
    let viewport = ViewportSize::new(640, 240);
    let mut main = PreparedFrame::new(viewport, Rgb { r: 0, g: 0, b: 0 }, 1.0);

    let f1 = cache.get_or_prepare(id1, &layout1, false, 0, |_| {});
    main.extend_from(f1);
    let f2 = cache.get_or_prepare(id2, &layout2, false, 0, |_| {});
    main.extend_from(f2);

    assert_eq!(main.backgrounds.len(), 3, "2 from pane1 + 1 from pane2");
}

#[test]
fn position_change_same_size_triggers_reprepare() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout_a = make_layout(id, 0.0, 0.0, 640.0, 480.0);
    // Same dimensions but different position (pane shifted right after sibling closed).
    let layout_b = make_layout(id, 320.0, 0.0, 640.0, 480.0);

    cache.get_or_prepare(id, &layout_a, true, 0, |f| push_marker(f, 1.0));

    let mut called = false;
    cache.get_or_prepare(id, &layout_b, false, 0, |f| {
        called = true;
        push_marker(f, 2.0);
    });
    assert!(called, "position change should trigger re-prepare");
}

#[test]
fn selective_dirty_only_reprepares_dirty_pane() {
    let mut cache = PaneRenderCache::new();
    let id1 = PaneId::from_raw(1);
    let id2 = PaneId::from_raw(2);
    let layout1 = make_layout(id1, 0.0, 0.0, 640.0, 480.0);
    let layout2 = make_layout(id2, 640.0, 0.0, 640.0, 480.0);

    // Seed both.
    cache.get_or_prepare(id1, &layout1, true, 0, |f| push_marker(f, 1.0));
    cache.get_or_prepare(id2, &layout2, true, 0, |f| push_marker(f, 2.0));

    // Only pane 1 is dirty.
    let mut called1 = false;
    let frame1 = cache.get_or_prepare(id1, &layout1, true, 0, |f| {
        called1 = true;
        push_marker(f, 10.0);
        push_marker(f, 11.0);
    });
    assert!(called1, "dirty pane 1 should re-prepare");
    assert_eq!(frame1.backgrounds.len(), 2);

    // Pane 2 is clean — should NOT re-prepare.
    let mut called2 = false;
    let frame2 = cache.get_or_prepare(id2, &layout2, false, 0, |_f| {
        called2 = true;
    });
    assert!(!called2, "clean pane 2 should use cache");
    assert_eq!(frame2.backgrounds.len(), 1, "pane 2 cached frame untouched");
}

// ── is_cached ────────────────────────────────────────────────────

#[test]
fn is_cached_true_after_prepare() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout = make_layout(id, 0.0, 0.0, 640.0, 480.0);

    assert!(!cache.is_cached(id, &layout, 0), "empty cache");

    cache.get_or_prepare(id, &layout, true, 0, |f| push_marker(f, 1.0));
    assert!(cache.is_cached(id, &layout, 0));
}

#[test]
fn is_cached_false_after_remove() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout = make_layout(id, 0.0, 0.0, 640.0, 480.0);

    cache.get_or_prepare(id, &layout, true, 0, |f| push_marker(f, 1.0));
    cache.remove(id);
    assert!(!cache.is_cached(id, &layout, 0));
}

#[test]
fn is_cached_false_after_invalidate_all() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout = make_layout(id, 0.0, 0.0, 640.0, 480.0);

    cache.get_or_prepare(id, &layout, true, 0, |f| push_marker(f, 1.0));
    cache.invalidate_all();
    assert!(!cache.is_cached(id, &layout, 0));
}

#[test]
fn is_cached_false_when_layout_mismatches() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout_a = make_layout(id, 0.0, 0.0, 640.0, 480.0);
    let layout_b = make_layout(id, 0.0, 0.0, 800.0, 600.0);

    cache.get_or_prepare(id, &layout_a, true, 0, |f| push_marker(f, 1.0));
    assert!(cache.is_cached(id, &layout_a, 0));
    assert!(
        !cache.is_cached(id, &layout_b, 0),
        "different layout should miss"
    );
}

// ── get_cached ───────────────────────────────────────────────────

#[test]
fn get_cached_returns_some_after_prepare() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout = make_layout(id, 0.0, 0.0, 640.0, 480.0);

    cache.get_or_prepare(id, &layout, true, 0, |f| push_marker(f, 1.0));
    let cached = cache.get_cached(id);
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().backgrounds.len(), 1);
}

#[test]
fn get_cached_returns_none_for_unknown_pane() {
    let cache = PaneRenderCache::new();
    assert!(cache.get_cached(PaneId::from_raw(99)).is_none());
}

#[test]
fn get_cached_returns_none_after_remove() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout = make_layout(id, 0.0, 0.0, 640.0, 480.0);

    cache.get_or_prepare(id, &layout, true, 0, |f| push_marker(f, 1.0));
    cache.remove(id);
    assert!(cache.get_cached(id).is_none());
}

// ── invalidate (single pane) ────────────────────────────────────

#[test]
fn invalidate_single_pane_triggers_reprepare() {
    let mut cache = PaneRenderCache::new();
    let id1 = PaneId::from_raw(1);
    let id2 = PaneId::from_raw(2);
    let layout1 = make_layout(id1, 0.0, 0.0, 640.0, 480.0);
    let layout2 = make_layout(id2, 640.0, 0.0, 640.0, 480.0);

    // Seed both panes.
    cache.get_or_prepare(id1, &layout1, true, 0, |f| push_marker(f, 1.0));
    cache.get_or_prepare(id2, &layout2, true, 0, |f| push_marker(f, 2.0));

    // Invalidate only pane 1.
    cache.invalidate(id1);

    // Pane 1 should re-prepare.
    let mut called1 = false;
    cache.get_or_prepare(id1, &layout1, false, 0, |f| {
        called1 = true;
        push_marker(f, 10.0);
    });
    assert!(called1, "invalidated pane should re-prepare");

    // Pane 2 should still be cached.
    let mut called2 = false;
    cache.get_or_prepare(id2, &layout2, false, 0, |_f| {
        called2 = true;
    });
    assert!(!called2, "non-invalidated pane should use cache");
}

// ── damage_key ───────────────────────────────────────────────────

/// damage_key SSOT routing for multi-pane cache.
/// Three cache-hit conditions must hold:
/// 1. `!dirty`
/// 2. `cached.layout == *layout`
/// 3. `cached.damage_key == damage_key`
/// Each of these positive/negative pairs clamps one input.

#[test]
fn cache_hit_when_damage_key_matches_and_layout_matches_and_not_dirty() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout = make_layout(id, 0.0, 0.0, 640.0, 480.0);
    let key = 0xABCD_1234_5678_DEADu64;

    cache.get_or_prepare(id, &layout, true, key, |f| push_marker(f, 1.0));

    let mut called = false;
    cache.get_or_prepare(id, &layout, false, key, |_| {
        called = true;
    });
    assert!(!called, "all 3 conditions matched → cache hit");
}

#[test]
fn cache_miss_when_damage_key_differs_even_if_layout_matches() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout = make_layout(id, 0.0, 0.0, 640.0, 480.0);

    cache.get_or_prepare(id, &layout, true, 1, |f| push_marker(f, 1.0));

    let mut called = false;
    cache.get_or_prepare(id, &layout, false, 2, |f| {
        called = true;
        push_marker(f, 2.0);
    });
    assert!(called, "damage_key change forces re-prepare");
}

#[test]
fn cache_miss_when_dirty_true_even_if_damage_key_matches() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout = make_layout(id, 0.0, 0.0, 640.0, 480.0);

    cache.get_or_prepare(id, &layout, true, 42, |f| push_marker(f, 1.0));

    let mut called = false;
    cache.get_or_prepare(id, &layout, true, 42, |f| {
        called = true;
        push_marker(f, 2.0);
    });
    assert!(
        called,
        "dirty=true forces re-prepare even with matching damage_key"
    );
}

#[test]
fn cache_miss_when_layout_differs_even_if_damage_key_matches() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout_a = make_layout(id, 0.0, 0.0, 640.0, 480.0);
    let layout_b = make_layout(id, 0.0, 0.0, 800.0, 600.0);

    cache.get_or_prepare(id, &layout_a, true, 99, |f| push_marker(f, 1.0));

    let mut called = false;
    cache.get_or_prepare(id, &layout_b, false, 99, |f| {
        called = true;
        push_marker(f, 2.0);
    });
    assert!(
        called,
        "layout change forces re-prepare even with matching damage_key"
    );
}

#[test]
fn stored_damage_key_updates_on_miss() {
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout = make_layout(id, 0.0, 0.0, 640.0, 480.0);

    cache.get_or_prepare(id, &layout, true, 1, |f| push_marker(f, 1.0));
    // Cache miss → re-prepare → stored damage_key now 2.
    cache.get_or_prepare(id, &layout, false, 2, |f| push_marker(f, 2.0));

    // Subsequent call with the new key hits.
    let mut called = false;
    cache.get_or_prepare(id, &layout, false, 2, |_| {
        called = true;
    });
    assert!(!called, "new damage_key was stored on miss");
}

#[test]
fn multiple_panes_independent_damage_keys() {
    let mut cache = PaneRenderCache::new();
    let id1 = PaneId::from_raw(1);
    let id2 = PaneId::from_raw(2);
    let layout1 = make_layout(id1, 0.0, 0.0, 640.0, 480.0);
    let layout2 = make_layout(id2, 640.0, 0.0, 640.0, 480.0);

    cache.get_or_prepare(id1, &layout1, true, 10, |f| push_marker(f, 1.0));
    cache.get_or_prepare(id2, &layout2, true, 20, |f| push_marker(f, 2.0));

    // Pane 1's damage_key changes — only pane 1 should re-prepare.
    let mut called1 = false;
    cache.get_or_prepare(id1, &layout1, false, 11, |f| {
        called1 = true;
        push_marker(f, 3.0);
    });
    assert!(called1, "pane 1's new damage_key forces re-prepare");

    let mut called2 = false;
    cache.get_or_prepare(id2, &layout2, false, 20, |_| {
        called2 = true;
    });
    assert!(!called2, "pane 2's damage_key unchanged → cache hit");
}

#[test]
fn invalidate_all_forces_reprepare_even_with_matching_damage_key() {
    // invalidate_all (font/atlas reload) MUST still trigger re-prepare.
    // The damage_key alone does not cover GPU atlas state.
    let mut cache = PaneRenderCache::new();
    let id = PaneId::from_raw(1);
    let layout = make_layout(id, 0.0, 0.0, 640.0, 480.0);
    let key = 7;

    cache.get_or_prepare(id, &layout, true, key, |f| push_marker(f, 1.0));
    cache.invalidate_all();

    let mut called = false;
    cache.get_or_prepare(id, &layout, false, key, |f| {
        called = true;
        push_marker(f, 2.0);
    });
    assert!(
        called,
        "invalidate_all must force re-prepare regardless of damage_key"
    );
}
