//! Double-buffered snapshot transfer between IO thread and main thread.
//!
//! The IO thread produces a [`RenderableContent`] into a work buffer,
//! then flips. The main thread reads from the front buffer. Both sides
//! hold the lock only for the flip (two pointer swaps) — nanoseconds,
//! not microseconds.
//!
//! **Why not `Option::take()`?** A latest-only slot loses damage state
//! from skipped snapshots and breaks buffer reuse (the producer gets
//! `None` back and loses `Vec` allocations). The double buffer ensures:
//! 1. Every consumed snapshot has valid, cumulative damage.
//! 2. Both sides always have a buffer with retained allocations.
//! 3. Skipped snapshots merge damage into the next one.

use std::sync::Arc;

use parking_lot::Mutex;

use oriterm_core::RenderableContent;

/// Double-buffered snapshot transfer.
///
/// The IO thread writes to its work buffer, then calls [`flip_swap()`]
/// to make it the new front. The main thread calls [`swap_front()`] to
/// exchange its old buffer for the latest snapshot. Lock is held only
/// for the swap — nanoseconds.
///
/// [`flip_swap()`]: Self::flip_swap
/// [`swap_front()`]: Self::swap_front
#[derive(Clone)]
pub struct SnapshotDoubleBuffer {
    inner: Arc<Mutex<DoubleBufferSlot>>,
}

struct DoubleBufferSlot {
    /// Front buffer — latest completed snapshot for the main thread.
    front: RenderableContent,
    /// Sequence number incremented on each flip.
    seqno: u64,
    /// Sequence number the main thread last consumed.
    consumed_seqno: u64,
}

impl SnapshotDoubleBuffer {
    /// Create a new double buffer with empty snapshots.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DoubleBufferSlot {
                front: RenderableContent::default(),
                seqno: 0,
                consumed_seqno: 0,
            })),
        }
    }

    /// Flip: the IO thread's completed buffer becomes front.
    ///
    /// Swaps the caller's buffer with the front in-place. After this
    /// call, `buf` contains the old front (with retained `Vec`
    /// allocations) for the IO thread to reuse. If the main thread
    /// hasn't consumed the previous front, `all_dirty` is set on the
    /// new front to avoid losing damage from the skipped frame.
    pub fn flip_swap(&self, buf: &mut RenderableContent) {
        let mut slot = self.inner.lock();
        let skipped = slot.seqno > slot.consumed_seqno;
        slot.seqno += 1;
        std::mem::swap(&mut slot.front, buf);
        if skipped {
            slot.front.all_dirty = true;
        }
        // `buf` now holds the old front — IO thread reuses its allocations.
    }

    /// Swap the front buffer with the caller's buffer.
    ///
    /// The caller (main thread) gives its old buffer and receives the
    /// latest snapshot. Returns `true` if a new snapshot was available.
    /// Both sides retain `Vec` allocations across swaps.
    pub fn swap_front(&self, caller_buf: &mut RenderableContent) -> bool {
        let mut slot = self.inner.lock();
        if slot.seqno == slot.consumed_seqno {
            return false;
        }
        std::mem::swap(&mut slot.front, caller_buf);
        slot.consumed_seqno = slot.seqno;
        true
    }

    /// Whether a new snapshot is available (not yet consumed).
    pub fn has_new(&self) -> bool {
        let slot = self.inner.lock();
        slot.seqno > slot.consumed_seqno
    }
}

#[cfg(test)]
mod tests;
