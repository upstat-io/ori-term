use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Lock-free mode cache: store and load round-trip.
#[test]
fn mode_cache_round_trip() {
    let cache = Arc::new(AtomicU32::new(0));

    // Simulate IO thread updating mode bits.
    cache.store(0x1234, Ordering::Release);
    assert_eq!(cache.load(Ordering::Acquire), 0x1234);

    // Update again.
    cache.store(0x5678, Ordering::Release);
    assert_eq!(cache.load(Ordering::Acquire), 0x5678);
}

/// Cross-thread atomic visibility (simulated with sequential ops).
#[test]
fn dirty_flag_cross_thread_pattern() {
    let dirty = Arc::new(AtomicBool::new(false));
    let dirty2 = Arc::clone(&dirty);

    // "IO thread" sets dirty.
    std::thread::spawn(move || {
        dirty2.store(true, Ordering::Release);
    })
    .join()
    .unwrap();

    // "Main thread" reads dirty.
    assert!(dirty.load(Ordering::Acquire));
}

/// Unseen output flag: set and clear round-trip (simulated with bool).
///
/// Mirrors the `has_bell` pattern: set on background output, clear on focus.
#[test]
fn unseen_output_set_and_clear() {
    // Starts false (no unseen output).
    let flag = AtomicBool::new(false);
    assert!(!flag.load(Ordering::Acquire));

    // Background output arrives → set.
    flag.store(true, Ordering::Release);
    assert!(flag.load(Ordering::Acquire));

    // Idempotent: setting again is harmless.
    flag.store(true, Ordering::Release);
    assert!(flag.load(Ordering::Acquire));

    // Pane gains focus → clear.
    flag.store(false, Ordering::Release);
    assert!(!flag.load(Ordering::Acquire));
}

/// Selection-dirty flag: swap-based clear returns previous value.
#[test]
fn selection_dirty_swap_clear() {
    let flag = Arc::new(AtomicBool::new(false));

    // IO thread sets dirty.
    flag.store(true, Ordering::Release);
    assert!(flag.load(Ordering::Acquire));

    // Main thread clears via swap — gets true back.
    let was_dirty = flag.swap(false, Ordering::AcqRel);
    assert!(was_dirty);
    assert!(!flag.load(Ordering::Acquire));
}
