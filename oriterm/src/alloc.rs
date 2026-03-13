//! Counting global allocator for profiling allocation pressure.
//!
//! Active only when the `profile` feature is enabled. Wraps the system
//! allocator and counts allocations/deallocations via relaxed atomics
//! (< 1 ns overhead per call). [`PerfStats`](super::app::PerfStats) reads
//! and resets these counters every logging interval.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

/// Total allocations since last [`snapshot_and_reset`] call.
static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);

/// Total deallocations since last [`snapshot_and_reset`] call.
static DEALLOC_COUNT: AtomicU64 = AtomicU64::new(0);

/// Total bytes requested via `alloc` since last [`snapshot_and_reset`] call.
static BYTES_ALLOCATED: AtomicU64 = AtomicU64::new(0);

/// Total bytes released via `dealloc` since last [`snapshot_and_reset`] call.
static BYTES_DEALLOCATED: AtomicU64 = AtomicU64::new(0);

/// Snapshot of counters returned by [`snapshot_and_reset`].
pub(crate) struct AllocSnapshot {
    /// Number of `alloc` calls in the interval.
    pub(crate) allocs: u64,
    /// Number of `dealloc` calls in the interval.
    pub(crate) deallocs: u64,
    /// Bytes requested in the interval.
    pub(crate) bytes_allocated: u64,
    /// Bytes freed in the interval.
    pub(crate) bytes_deallocated: u64,
}

/// Read current counters and reset them to zero.
///
/// Uses `Relaxed` ordering — counters may be slightly stale but the cost
/// is effectively zero and the values are only used for logging.
pub(crate) fn snapshot_and_reset() -> AllocSnapshot {
    AllocSnapshot {
        allocs: ALLOC_COUNT.swap(0, Ordering::Relaxed),
        deallocs: DEALLOC_COUNT.swap(0, Ordering::Relaxed),
        bytes_allocated: BYTES_ALLOCATED.swap(0, Ordering::Relaxed),
        bytes_deallocated: BYTES_DEALLOCATED.swap(0, Ordering::Relaxed),
    }
}

/// Counting wrapper around the system allocator.
pub(crate) struct CountingAlloc;

// SAFETY: Delegates directly to `System` which is a sound `GlobalAlloc`.
// The only additions are relaxed atomic increments (no synchronization
// requirements, no memory access beyond the atomics).
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        BYTES_ALLOCATED.fetch_add(layout.size() as u64, Ordering::Relaxed);
        // SAFETY: Forwarding to `System` with the same layout.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        BYTES_DEALLOCATED.fetch_add(layout.size() as u64, Ordering::Relaxed);
        // SAFETY: Forwarding to `System` with the same pointer and layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}
