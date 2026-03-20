//! Allocation regression tests for Scene pipeline zero-alloc invariants.
//!
//! Uses a counting global allocator to verify that `Scene::clear()` + push
//! operations and `DamageTracker::compute_damage()` perform zero heap
//! allocations after warmup. Placed in an integration test file (separate
//! binary) to isolate the `#[global_allocator]`.
//!
//! **Thread safety**: The counting allocator is process-wide. Thresholds
//! account for parallel test thread noise (~5-20 allocs per window).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use oriterm_ui::color::Color;
use oriterm_ui::draw::{DamageTracker, RectStyle, Scene};
use oriterm_ui::geometry::Rect;
use oriterm_ui::widget_id::WidgetId;

// --- Counting allocator with enable/disable gate ---

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);

struct CountingAlloc;

#[allow(unsafe_code)]
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[allow(unsafe_code)]
#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

fn measure_allocs(f: impl FnOnce()) -> u64 {
    ALLOC_COUNT.store(0, Ordering::SeqCst);
    COUNTING.store(true, Ordering::SeqCst);
    f();
    COUNTING.store(false, Ordering::SeqCst);
    ALLOC_COUNT.load(Ordering::SeqCst)
}

/// Threshold for "zero-alloc" assertion. Accounts for noise from parallel
/// test threads. A real regression (per-primitive alloc) would produce
/// thousands of allocs on a 80x24 scene.
const ZERO_ALLOC_THRESHOLD: u64 = 50;

// --- Helpers ---

/// Populates a scene with quads simulating an 80x24 terminal (~1920 cells).
fn populate_scene(scene: &mut Scene, ids: &[WidgetId]) {
    scene.clear();
    for (i, &id) in ids.iter().enumerate() {
        scene.set_widget_id(Some(id));
        let x = (i % 80) as f32 * 10.0;
        let y = (i / 80) as f32 * 20.0;
        scene.push_quad(Rect::new(x, y, 10.0, 20.0), RectStyle::filled(Color::WHITE));
    }
}

// --- Tests ---

/// After warmup, `Scene::clear()` + repopulate must perform zero allocations
/// because typed Vec arrays retain capacity.
#[test]
fn scene_clear_and_repopulate_zero_alloc() {
    let mut scene = Scene::new();
    let ids: Vec<WidgetId> = (0..1920).map(|_| WidgetId::next()).collect();

    // Warmup: establish Vec capacities.
    populate_scene(&mut scene, &ids);

    // Measure: second populate should allocate nothing.
    let allocs = measure_allocs(|| {
        populate_scene(&mut scene, &ids);
    });

    assert!(
        allocs <= ZERO_ALLOC_THRESHOLD,
        "Scene clear+repopulate allocated {allocs} times (threshold {ZERO_ALLOC_THRESHOLD})"
    );
}

/// After warmup, `DamageTracker::compute_damage()` must perform zero
/// allocations because HashMap fields swap via `std::mem::swap` and the
/// `merge_used` bool buffer is reused.
#[test]
fn damage_tracker_compute_damage_zero_alloc() {
    let mut tracker = DamageTracker::new();
    let mut scene = Scene::new();
    let ids: Vec<WidgetId> = (0..100).map(|_| WidgetId::next()).collect();

    // Frame 1 — warmup (first frame always dirty, establishes HashMap caps).
    populate_scene(&mut scene, &ids);
    tracker.compute_damage(&scene);

    // Frame 2 — warmup (swap establishes both HashMap capacities).
    populate_scene(&mut scene, &ids);
    tracker.compute_damage(&scene);

    // Frame 3 — identical scene, measure should be zero-alloc.
    let allocs = measure_allocs(|| {
        populate_scene(&mut scene, &ids);
        tracker.compute_damage(&scene);
    });

    assert!(
        allocs <= ZERO_ALLOC_THRESHOLD,
        "DamageTracker compute_damage allocated {allocs} times (threshold {ZERO_ALLOC_THRESHOLD})"
    );
}

/// Damage tracking with changed widgets (dirty regions + merge) should still
/// be zero-alloc after warmup because `merge_used` and `merge_scratch` reuse.
#[test]
fn damage_tracker_with_changes_zero_alloc() {
    let mut tracker = DamageTracker::new();
    let ids: Vec<WidgetId> = (0..50).map(|_| WidgetId::next()).collect();

    // Frame 1 — warmup.
    let mut scene = Scene::new();
    populate_scene(&mut scene, &ids);
    tracker.compute_damage(&scene);

    // Frame 2 — warmup with changes to establish merge buffer sizes.
    let mut scene2 = Scene::new();
    for (i, &id) in ids.iter().enumerate() {
        scene2.set_widget_id(Some(id));
        let x = (i % 80) as f32 * 10.0;
        let y = (i / 80) as f32 * 20.0;
        scene2.push_quad(Rect::new(x, y, 10.0, 20.0), RectStyle::filled(Color::BLACK));
    }
    tracker.compute_damage(&scene2);

    // Frame 3 — measure: change colors again.
    let allocs = measure_allocs(|| {
        populate_scene(&mut scene, &ids);
        tracker.compute_damage(&scene);
    });

    assert!(
        allocs <= ZERO_ALLOC_THRESHOLD,
        "DamageTracker with changes allocated {allocs} times (threshold {ZERO_ALLOC_THRESHOLD})"
    );
}
