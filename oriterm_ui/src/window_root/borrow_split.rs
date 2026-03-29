//! Multi-field borrow-splitting accessor methods for `WindowRoot`.
//!
//! When callers need simultaneous references to multiple `WindowRoot` fields
//! (e.g. `&mut InteractionManager` + `&mut FocusManager` for
//! `apply_dispatch_requests`), individual accessor methods like
//! `interaction_mut()` and `focus_mut()` conflict because each borrows
//! `&mut self`. These methods destructure `self` to yield multiple
//! disjoint references in a single call.

use crate::animation::FrameRequestFlags;
use crate::compositor::{LayerAnimator, LayerTree};
use crate::draw::DamageTracker;
use crate::focus::FocusManager;
use crate::interaction::InteractionManager;
use crate::invalidation::InvalidationTracker;
use crate::overlay::OverlayManager;

use super::WindowRoot;

impl WindowRoot {
    /// Returns `(&mut InteractionManager, &mut FocusManager)`.
    ///
    /// For `apply_dispatch_requests` and `request_focus`.
    pub fn interaction_and_focus_mut(&mut self) -> (&mut InteractionManager, &mut FocusManager) {
        (&mut self.interaction, &mut self.focus)
    }

    /// Returns `(&mut InteractionManager, &mut InvalidationTracker, &FrameRequestFlags)`.
    ///
    /// For `prepare_widget_tree` callers that also need the tracker for
    /// selective tree walks.
    pub fn interaction_invalidation_and_frame_requests_mut(
        &mut self,
    ) -> (
        &mut InteractionManager,
        &mut InvalidationTracker,
        &FrameRequestFlags,
    ) {
        (
            &mut self.interaction,
            &mut self.invalidation,
            &self.frame_requests,
        )
    }

    /// Returns `(&InteractionManager, &FrameRequestFlags)`.
    ///
    /// For `prepaint_widget_tree`.
    pub fn interaction_and_frame_requests(&self) -> (&InteractionManager, &FrameRequestFlags) {
        (&self.interaction, &self.frame_requests)
    }

    /// Returns `(&InteractionManager, &FrameRequestFlags, &mut DamageTracker)`.
    ///
    /// For `draw_tab_bar` which reads interaction + frame requests while
    /// mutating the damage tracker.
    pub fn interaction_frame_requests_and_damage_mut(
        &mut self,
    ) -> (&InteractionManager, &FrameRequestFlags, &mut DamageTracker) {
        (&self.interaction, &self.frame_requests, &mut self.damage)
    }

    /// Returns `(&mut OverlayManager, &LayerTree, &InteractionManager, &FrameRequestFlags)`.
    ///
    /// For `draw_overlays` which mutates overlays while reading layer tree,
    /// interaction, and frame requests.
    pub fn overlays_layer_tree_interaction_and_frame_requests(
        &mut self,
    ) -> (
        &mut OverlayManager,
        &LayerTree,
        &InteractionManager,
        &FrameRequestFlags,
    ) {
        (
            &mut self.overlays,
            &self.layer_tree,
            &self.interaction,
            &self.frame_requests,
        )
    }

    /// Returns `(&mut LayerTree, &LayerAnimator)`.
    ///
    /// For `TabSlideState::cleanup` which mutates layer tree while reading
    /// the animator.
    pub fn layer_tree_mut_and_animator(&mut self) -> (&mut LayerTree, &LayerAnimator) {
        (&mut self.layer_tree, &self.layer_animator)
    }

    /// Returns `(&mut LayerTree, &mut LayerAnimator)`.
    ///
    /// For operations requiring mutable access to both compositor components.
    pub fn layer_tree_and_animator_mut(&mut self) -> (&mut LayerTree, &mut LayerAnimator) {
        (&mut self.layer_tree, &mut self.layer_animator)
    }
}
