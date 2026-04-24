//! Centralized tracking of animation frame and repaint requests.
//!
//! [`RenderScheduler`] is owned by the application layer (one per window
//! context). Widgets signal requests via context flags; the framework reads
//! those flags after each widget call and forwards them here.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
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
    /// Per-key animation deadline map — feeds the event loop's `WaitUntil`
    /// for pane-global work like kitty graphics animation ticks (see
    /// `set_animation_deadline`). Key is opaque to the scheduler — callers
    /// (typically the oriterm app layer) pass `PaneId.as_u64()` so each
    /// pane has exactly one outstanding deadline, with REPLACEMENT
    /// semantics: a new call overwrites the prior deadline; `None` removes
    /// the entry. This prevents heap growth under animation start/stop
    /// churn and lets a stop notification actively cancel a queued wake
    /// that would otherwise fire spuriously.
    animation_deadlines: HashMap<u64, Instant>,
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
            animation_deadlines: HashMap::new(),
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

    /// Set (or clear) a per-key animation deadline.
    ///
    /// Pane-global work (kitty graphics animation frame deadlines, sixel
    /// animation, any non-widget-bound timer) calls this so
    /// `next_wake_time()` surfaces the deadline to the event loop's
    /// `ControlFlow::WaitUntil`. Distinct from `request_repaint_after`
    /// which binds a repaint to a specific widget — animation deadlines
    /// are terminal-state events that repaint the whole pane via the
    /// normal dirty-snapshot path once the event loop wakes.
    ///
    /// REPLACEMENT semantics per `key`: a new `Some(deadline)` overwrites
    /// the prior deadline for that key. `None` removes the entry — used
    /// when an animation stops so the queued wake does not fire
    /// spuriously. Key is opaque to the scheduler; the oriterm app layer
    /// passes `PaneId.as_u64()` so each pane has exactly one outstanding
    /// deadline.
    pub fn set_animation_deadline(&mut self, key: u64, deadline: Option<Instant>) {
        match deadline {
            Some(instant) => {
                self.animation_deadlines.insert(key, instant);
            }
            None => {
                self.animation_deadlines.remove(&key);
            }
        }
    }

    /// Whether any work is pending that requires a frame or wakeup.
    pub fn has_pending_work(&self, now: Instant) -> bool {
        !self.anim_frame_requests.is_empty()
            || !self.paint_requests.is_empty()
            || self.has_ready_deferred(now)
            || self.has_ready_animation_deadline(now)
    }

    /// Earliest deferred repaint time, if any.
    ///
    /// Feeds into the event loop's `ControlFlow::WaitUntil` computation.
    /// Returns the minimum of the widget-bound deferred-repaint heap and
    /// the widget-less animation-deadline heap so both wake sources are
    /// honored with a single query.
    pub fn next_wake_time(&self) -> Option<Instant> {
        let widget_wake = self
            .deferred_repaints
            .peek()
            .map(|Reverse(entry)| entry.wake_at);
        let anim_wake = self.animation_deadlines.values().min().copied();
        match (widget_wake, anim_wake) {
            (Some(w), Some(a)) => Some(w.min(a)),
            (Some(w), None) => Some(w),
            (None, Some(a)) => Some(a),
            (None, None) => None,
        }
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
    /// entries for removed widgets. Also drains matured animation
    /// deadlines — they have no widget to repaint; their role was to wake
    /// the event loop at the right instant, and the event loop's normal
    /// dirty-snapshot path handles the repaint once the PTY-driven snapshot
    /// flips. Under REPLACEMENT semantics a pane's entry is rewritten each
    /// tick by the IO thread while the animation is active; the drain here
    /// removes entries that matured without a replacement (the animation
    /// stopped just as its deadline fired).
    pub fn promote_deferred(&mut self, now: Instant) {
        while let Some(Reverse(entry)) = self.deferred_repaints.peek() {
            if entry.wake_at > now {
                break;
            }
            // Safe: `peek()` returned `Some`, so `pop()` will too.
            let Reverse(entry) = self.deferred_repaints.pop().expect("peek succeeded");
            // Lazy removal: skip entries for removed widgets.
            if !self.removed_widgets.contains(&entry.widget_id) {
                self.paint_requests.insert(entry.widget_id);
            }
        }
        self.animation_deadlines.retain(|_, wake_at| *wake_at > now);
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

    /// Check if any per-key animation deadline has matured.
    fn has_ready_animation_deadline(&self, now: Instant) -> bool {
        self.animation_deadlines
            .values()
            .any(|wake_at| *wake_at <= now)
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
            .field("animation_deadline_count", &self.animation_deadlines.len())
            .field("removed_count", &self.removed_widgets.len())
            .finish()
    }
}

#[cfg(test)]
mod tests;
