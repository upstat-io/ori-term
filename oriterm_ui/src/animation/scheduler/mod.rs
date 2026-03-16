//! Centralized tracking of animation frame and repaint requests.
//!
//! [`RenderScheduler`] is owned by the application layer (one per window
//! context). Widgets signal requests via context flags; the framework reads
//! those flags after each widget call and forwards them here.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};
use std::time::{Duration, Instant};

use crate::widget_id::WidgetId;

/// A deferred repaint request, ordered by wake time.
///
/// `WidgetId` is NOT included in the ordering — only `wake_at` matters
/// for the min-heap. Ties are broken arbitrarily.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct DeferredRepaint {
    widget_id: WidgetId,
    wake_at: Instant,
}

impl Ord for DeferredRepaint {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.wake_at.cmp(&other.wake_at)
    }
}

impl PartialOrd for DeferredRepaint {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Centralized tracking of animation frame and repaint requests.
///
/// Owned by the application layer (one per window context). Widgets
/// signal requests via context flags; the framework reads those flags
/// after each widget call and forwards them to the scheduler.
pub struct RenderScheduler {
    /// Widgets that have requested animation frames.
    anim_frame_requests: HashSet<WidgetId>,
    /// Widgets that have requested repaint.
    paint_requests: HashSet<WidgetId>,
    /// Deferred repaints (e.g., cursor blink after 530ms).
    /// Uses `Reverse<>` because `BinaryHeap` is a max-heap; wrapping in
    /// `Reverse` gives min-heap behavior (earliest `wake_at` first).
    deferred_repaints: BinaryHeap<Reverse<DeferredRepaint>>,
    /// Widget IDs that have been removed. Used for lazy cleanup of
    /// deferred repaints during `promote_deferred()`.
    removed_widgets: HashSet<WidgetId>,
}

impl RenderScheduler {
    /// Creates a new scheduler with no pending requests.
    pub fn new() -> Self {
        Self {
            anim_frame_requests: HashSet::new(),
            paint_requests: HashSet::new(),
            deferred_repaints: BinaryHeap::new(),
            removed_widgets: HashSet::new(),
        }
    }

    /// Request an animation frame for the given widget.
    pub fn request_anim_frame(&mut self, widget_id: WidgetId) {
        self.anim_frame_requests.insert(widget_id);
    }

    /// Request a repaint for the given widget.
    pub fn request_paint(&mut self, widget_id: WidgetId) {
        self.paint_requests.insert(widget_id);
    }

    /// Request a repaint after a delay (e.g., cursor blink timer).
    pub fn request_repaint_after(&mut self, widget_id: WidgetId, duration: Duration, now: Instant) {
        self.deferred_repaints.push(Reverse(DeferredRepaint {
            widget_id,
            wake_at: now + duration,
        }));
    }

    /// Whether any work is pending that requires a frame or wakeup.
    pub fn has_pending_work(&self, now: Instant) -> bool {
        !self.anim_frame_requests.is_empty()
            || !self.paint_requests.is_empty()
            || self.has_ready_deferred(now)
    }

    /// Earliest deferred repaint time, if any.
    ///
    /// Feeds into the event loop's `ControlFlow::WaitUntil` computation.
    pub fn next_wake_time(&self) -> Option<Instant> {
        self.deferred_repaints
            .peek()
            .map(|Reverse(entry)| entry.wake_at)
    }

    /// Move the animation frame request set out via `std::mem::take()`.
    ///
    /// Zero-alloc if the set was empty. The scheduler's field becomes an
    /// empty `HashSet` with zero capacity.
    pub fn take_anim_frames(&mut self) -> HashSet<WidgetId> {
        std::mem::take(&mut self.anim_frame_requests)
    }

    /// Move the paint request set out via `std::mem::take()`.
    pub fn take_paint_requests(&mut self) -> HashSet<WidgetId> {
        std::mem::take(&mut self.paint_requests)
    }

    /// Promote deferred repaints whose `wake_at <= now` into `paint_requests`.
    ///
    /// Called at the start of each frame before draining. Lazily skips
    /// entries for removed widgets.
    pub fn promote_deferred(&mut self, now: Instant) {
        while let Some(Reverse(entry)) = self.deferred_repaints.peek() {
            if entry.wake_at > now {
                break;
            }
            let entry = self.deferred_repaints.pop().unwrap().0;
            // Lazy removal: skip entries for removed widgets.
            if !self.removed_widgets.contains(&entry.widget_id) {
                self.paint_requests.insert(entry.widget_id);
            }
        }
    }

    /// Remove all pending requests for a widget.
    ///
    /// Called on widget removal / deregistration. Uses lazy removal for
    /// deferred repaints (they're skipped during `promote_deferred()`).
    pub fn remove_widget(&mut self, widget_id: WidgetId) {
        self.anim_frame_requests.remove(&widget_id);
        self.paint_requests.remove(&widget_id);
        // Lazy removal for deferred heap entries.
        self.removed_widgets.insert(widget_id);
    }

    /// Check if any deferred repaints are ready.
    fn has_ready_deferred(&self, now: Instant) -> bool {
        self.deferred_repaints
            .peek()
            .is_some_and(|Reverse(entry)| entry.wake_at <= now)
    }
}

impl Default for RenderScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for RenderScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderScheduler")
            .field("anim_frame_count", &self.anim_frame_requests.len())
            .field("paint_count", &self.paint_requests.len())
            .field("deferred_count", &self.deferred_repaints.len())
            .field("removed_count", &self.removed_widgets.len())
            .finish()
    }
}

#[cfg(test)]
mod tests;
