//! Tab bar animation lifecycle — hover, drag, open/close tab transitions.
//!
//! Manages per-tab animation state: hover background fading, close-button
//! opacity, tab width multipliers for open/close animations, drag visual
//! positioning, and the animation offset buffer used by the compositor.

use std::time::Instant;

use crate::animation::AnimProperty;

use super::super::hit::TabBarHit;
use super::super::layout::TabBarLayout;
use super::TabBarWidget;

impl TabBarWidget {
    /// Updates which element the cursor is hovering, driving hover animations.
    ///
    /// Starts animated transitions for hover background and close button
    /// visibility on the affected tabs.
    pub fn set_hover_hit(&mut self, hit: TabBarHit) {
        let old_tab = self.hover_hit.tab_index();
        let new_tab = hit.tab_index();
        self.hover_hit = hit;

        // Animate hover leave on old tab.
        if let Some(i) = old_tab {
            if Some(i) != new_tab {
                if let Some(p) = self.hover_progress.get_mut(i) {
                    p.set(0.0);
                }
                if let Some(o) = self.close_btn_opacity.get_mut(i) {
                    o.set(0.0);
                }
            }
        }
        // Animate hover enter on new tab.
        if let Some(i) = new_tab {
            if Some(i) != old_tab {
                if let Some(p) = self.hover_progress.get_mut(i) {
                    p.set(1.0);
                }
                if let Some(o) = self.close_btn_opacity.get_mut(i) {
                    o.set(1.0);
                }
            }
        }
    }

    /// Sets the dragged tab visual state.
    ///
    /// `Some((index, x))` means tab `index` is being dragged and its visual
    /// position is at `x` logical pixels. `None` means no drag in progress.
    pub fn set_drag_visual(&mut self, drag: Option<(usize, f32)>) {
        self.drag_visual = drag;
    }

    // Tab lifecycle animations

    /// Starts a tab open animation, expanding from zero to full width.
    ///
    /// Call after `set_tabs()` which initializes the entry at 1.0.
    /// This overrides to start from 0.0 and animate to 1.0 over 200ms.
    pub fn animate_tab_open(&mut self, index: usize) {
        if let Some(m) = self.width_multipliers.get_mut(index) {
            m.set_immediate(0.0);
            m.set(1.0);
        }
    }

    /// Starts a tab close animation, shrinking from full to zero width.
    ///
    /// Marks the tab as closing (skipped for hover/click interaction).
    /// When the animation completes, call [`closing_complete`] to find
    /// which tab to remove.
    pub fn animate_tab_close(&mut self, index: usize) {
        use super::TAB_CLOSE_DURATION;
        use crate::animation::{AnimBehavior, AnimProperty};

        if let Some(m) = self.width_multipliers.get_mut(index) {
            *m = AnimProperty::with_behavior(
                1.0,
                AnimBehavior::ease_out(TAB_CLOSE_DURATION.as_millis() as u64),
            );
            m.set(0.0);
        }
        if let Some(c) = self.closing_tabs.get_mut(index) {
            *c = true;
        }
    }

    /// Returns the index of a tab whose close animation has finished.
    ///
    /// The app layer polls this during redraw and removes the finished
    /// tab via `set_tabs()`.
    pub fn closing_complete(&self) -> Option<usize> {
        self.closing_tabs
            .iter()
            .enumerate()
            .find(|&(i, &closing)| {
                closing && self.width_multipliers.get(i).is_none_or(|m| m.get() < 0.01)
            })
            .map(|(i, _)| i)
    }

    /// Whether the tab at `index` is in closing state.
    pub fn is_closing(&self, index: usize) -> bool {
        self.closing_tabs.get(index).copied().unwrap_or(false)
    }

    /// Whether any width animation is currently running.
    pub fn has_width_animation(&self) -> bool {
        self.width_multipliers
            .iter()
            .any(AnimProperty::is_animating)
    }

    /// Updates layout with current animated width multipliers.
    ///
    /// Call once per frame before draw when width animations are active.
    /// No-op when no width animations are running.
    pub fn update_animated_layout(&mut self) {
        if self.has_width_animation() {
            self.recompute_layout_animated();
        }
    }

    // Accessors

    /// Current computed layout.
    pub fn layout(&self) -> &TabBarLayout {
        &self.layout
    }

    /// Number of tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Current hover hit state.
    pub fn hover_hit(&self) -> TabBarHit {
        self.hover_hit
    }

    /// Current tab width lock value, if active.
    pub fn tab_width_lock(&self) -> Option<f32> {
        self.tab_width_lock
    }

    /// Update the title of the tab at `index`.
    ///
    /// No-op if `index` is out of bounds.
    pub fn update_tab_title(&mut self, index: usize, title: String) {
        if let Some(entry) = self.tabs.get_mut(index) {
            entry.title = title;
        }
    }

    /// Start a bell animation on the tab at `index`.
    ///
    /// Records `now` as the bell start time. No-op if `index` is out of
    /// bounds.
    pub fn ring_bell(&mut self, index: usize, now: Instant) {
        if let Some(entry) = self.tabs.get_mut(index) {
            entry.bell_start = Some(now);
        }
    }

    // Private helpers

    /// Recomputes layout from current state.
    ///
    /// When width multipliers are active (during open/close animations),
    /// passes current multiplier values to the layout computation.
    pub(super) fn recompute_layout(&mut self) {
        self.layout = TabBarLayout::compute(
            self.tabs.len(),
            self.window_width,
            self.tab_width_lock,
            self.left_inset,
            &self.metrics,
        );
    }

    /// Recomputes layout with current animated width multipliers.
    ///
    /// Called during draw when width animations are running. Samples
    /// each `AnimProperty` at `now` and passes the snapshot to layout.
    fn recompute_layout_animated(&mut self) {
        let multipliers: Vec<f32> = self
            .width_multipliers
            .iter()
            .map(AnimProperty::get)
            .collect();
        self.layout = TabBarLayout::compute_with_multipliers(
            self.tabs.len(),
            self.window_width,
            self.tab_width_lock,
            self.left_inset,
            Some(&multipliers),
            &self.metrics,
        );
    }

    /// Returns the animation offset for a tab, or 0.0 if none.
    pub(super) fn anim_offset(&self, index: usize) -> f32 {
        self.anim_offsets.get(index).copied().unwrap_or(0.0)
    }

    /// Whether a tab drag overlay should be drawn.
    pub fn has_drag_overlay(&self) -> bool {
        self.drag_visual.is_some_and(|(i, _)| i < self.tabs.len())
    }

    /// Whether the given tab index is the one being dragged.
    pub(super) fn is_dragged(&self, index: usize) -> bool {
        self.drag_visual.is_some_and(|(i, _)| i == index)
    }

    /// Swaps the internal animation offset buffer with an external one.
    ///
    /// Used by [`TabSlideState`](super::super::slide::TabSlideState) to populate
    /// per-tab offsets from compositor transforms without allocating. The
    /// caller fills `buf` with compositor-driven offsets, swaps in, and
    /// gets the old buffer back for reuse next frame.
    pub(crate) fn swap_anim_offsets(&mut self, buf: &mut Vec<f32>) {
        std::mem::swap(&mut self.anim_offsets, buf);
    }
}
