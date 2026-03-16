//! Overlay lifecycle — push, dismiss, and cleanup operations.
//!
//! Extracted from the main `OverlayManager` to keep `mod.rs` under 500 lines.
//! These methods are called from the application layer at specific points
//! (push on user action, dismiss on click-outside/Escape).

use std::time::Instant;

use crate::animation::Easing;
use crate::compositor::layer::{LayerProperties, LayerType};
use crate::compositor::layer_animator::{AnimationParams, LayerAnimator};
use crate::compositor::layer_tree::LayerTree;
use crate::geometry::Rect;

use super::{FADE_DURATION, MODAL_DIM_COLOR, Overlay, OverlayId, OverlayKind, OverlayManager};
use crate::overlay::placement::Placement;
use crate::widgets::Widget;

impl OverlayManager {
    /// Pushes a non-modal overlay that dismisses on click-outside.
    ///
    /// Creates a `Textured` compositor layer at full opacity (no fade-in).
    /// Popups like dropdown menus and context menus should appear instantly.
    #[expect(
        clippy::too_many_arguments,
        reason = "lifecycle: widget, anchor, placement, tree, animator, now"
    )]
    pub fn push_overlay(
        &mut self,
        widget: Box<dyn Widget>,
        anchor: Rect,
        placement: Placement,
        tree: &mut LayerTree,
        _animator: &mut LayerAnimator,
        _now: Instant,
    ) -> OverlayId {
        let id = OverlayId::next();
        let root = tree.root();

        let layer_id = tree.add(
            root,
            LayerType::Textured,
            LayerProperties {
                opacity: 1.0,
                ..LayerProperties::default()
            },
        );

        self.overlays.push(Overlay {
            id,
            widget,
            anchor,
            placement,
            kind: OverlayKind::Popup,
            computed_rect: Rect::default(),
            layout_node: None,
            layer_id,
            dim_layer_id: None,
        });
        self.layout_dirty = true;
        id
    }

    /// Replaces any active popup overlays with a new popup.
    ///
    /// Preserves modal overlays beneath the popup stack. This is the correct
    /// entry point for transient UI like context menus and dropdown popups,
    /// where duplicate stacked popups would leave stale interaction state.
    #[expect(
        clippy::too_many_arguments,
        reason = "lifecycle: widget, anchor, placement, tree, animator, now"
    )]
    pub fn replace_popup(
        &mut self,
        widget: Box<dyn Widget>,
        anchor: Rect,
        placement: Placement,
        tree: &mut LayerTree,
        animator: &mut LayerAnimator,
        now: Instant,
    ) -> OverlayId {
        self.clear_popups(tree, animator);
        self.push_overlay(widget, anchor, placement, tree, animator, now)
    }

    /// Pushes a modal overlay (blocks interaction below, no click-outside dismiss).
    ///
    /// Creates a `SolidColor` dim layer and a `Textured` content layer,
    /// both with fade-in animations (opacity `0→1`, 150ms `EaseOut`).
    #[expect(
        clippy::too_many_arguments,
        reason = "lifecycle: widget, anchor, placement, tree, animator, now"
    )]
    pub fn push_modal(
        &mut self,
        widget: Box<dyn Widget>,
        anchor: Rect,
        placement: Placement,
        tree: &mut LayerTree,
        _animator: &mut LayerAnimator,
        _now: Instant,
    ) -> OverlayId {
        let id = OverlayId::next();
        let root = tree.root();

        // Dim layer (SolidColor) — drawn behind content.
        // Both layers start at full opacity (no fade-in animation) so the
        // modal appears instantly. The fade-out on dismiss is still animated.
        let dim_layer_id = tree.add(
            root,
            LayerType::SolidColor(MODAL_DIM_COLOR),
            LayerProperties {
                bounds: self.viewport,
                opacity: 1.0,
                ..LayerProperties::default()
            },
        );

        // Content layer (Textured).
        let layer_id = tree.add(
            root,
            LayerType::Textured,
            LayerProperties {
                opacity: 1.0,
                ..LayerProperties::default()
            },
        );

        self.overlays.push(Overlay {
            id,
            widget,
            anchor,
            placement,
            kind: OverlayKind::Modal,
            computed_rect: Rect::default(),
            layout_node: None,
            layer_id,
            dim_layer_id: Some(dim_layer_id),
        });
        self.layout_dirty = true;
        id
    }

    /// Begins dismissing a specific overlay by ID.
    ///
    /// Popup overlays are removed instantly. Modal overlays fade out via
    /// the compositor and are moved to the dismissing list. Returns `true`
    /// if found.
    pub fn begin_dismiss(
        &mut self,
        id: OverlayId,
        tree: &mut LayerTree,
        animator: &mut LayerAnimator,
        now: Instant,
    ) -> bool {
        let Some(idx) = self.overlays.iter().position(|o| o.id == id) else {
            return false;
        };
        let overlay = self.overlays.remove(idx);
        self.dismiss_overlay(overlay, tree, animator, now);
        self.hovered_overlay = None;
        self.captured_overlay = None;
        self.layout_dirty = true;
        true
    }

    /// Begins dismissing the topmost overlay.
    ///
    /// Popup overlays are removed instantly. Modal overlays fade out.
    /// Returns the dismissed overlay's ID, or `None` if the stack is empty.
    pub fn begin_dismiss_topmost(
        &mut self,
        tree: &mut LayerTree,
        animator: &mut LayerAnimator,
        now: Instant,
    ) -> Option<OverlayId> {
        let overlay = self.overlays.pop()?;
        let id = overlay.id;
        self.dismiss_overlay(overlay, tree, animator, now);
        self.hovered_overlay = None;
        self.captured_overlay = None;
        self.layout_dirty = true;
        Some(id)
    }

    /// Removes all overlays instantly, canceling any running animations.
    pub fn clear_all(&mut self, tree: &mut LayerTree, animator: &mut LayerAnimator) {
        for overlay in self.overlays.drain(..).chain(self.dismissing.drain(..)) {
            animator.cancel_all(overlay.layer_id);
            tree.remove_subtree(overlay.layer_id);
            if let Some(dim_id) = overlay.dim_layer_id {
                animator.cancel_all(dim_id);
                tree.remove_subtree(dim_id);
            }
        }
        self.hovered_overlay = None;
        self.captured_overlay = None;
        self.layout_dirty = false;
    }

    /// Removes all active popup overlays, preserving modals.
    ///
    /// Popups are transient UI and are removed immediately rather than faded
    /// out. Returns the number of removed overlays.
    pub fn clear_popups(&mut self, tree: &mut LayerTree, animator: &mut LayerAnimator) -> usize {
        let before = self.overlays.len();
        let mut retained = Vec::with_capacity(before);

        for overlay in self.overlays.drain(..) {
            if overlay.kind == OverlayKind::Popup {
                animator.cancel_all(overlay.layer_id);
                tree.remove_subtree(overlay.layer_id);
            } else {
                retained.push(overlay);
            }
        }

        let removed = before - retained.len();
        self.overlays = retained;

        if removed > 0 {
            self.hovered_overlay = None;
            self.captured_overlay = None;
            self.layout_dirty = true;
        }

        removed
    }

    /// Removes dismissing overlays whose fade-out animations have completed.
    ///
    /// Call after [`LayerAnimator::tick`] each frame. Removes compositor layers
    /// for fully faded overlays.
    pub fn cleanup_dismissed(&mut self, tree: &mut LayerTree, animator: &LayerAnimator) {
        self.dismissing.retain(|overlay| {
            let still_fading = animator.is_animating(
                overlay.layer_id,
                crate::animation::AnimatableProperty::Opacity,
            );
            if !still_fading {
                tree.remove_subtree(overlay.layer_id);
                if let Some(dim_id) = overlay.dim_layer_id {
                    tree.remove_subtree(dim_id);
                }
            }
            still_fading
        });
    }

    /// Dismisses an overlay — instant removal for popups, fade-out for modals.
    fn dismiss_overlay(
        &mut self,
        overlay: Overlay,
        tree: &mut LayerTree,
        animator: &mut LayerAnimator,
        now: Instant,
    ) {
        if overlay.kind == OverlayKind::Popup {
            // Popups disappear instantly — remove compositor layers now.
            animator.cancel_all(overlay.layer_id);
            tree.remove_subtree(overlay.layer_id);
        } else {
            // Modals fade out via the compositor.
            Self::start_fade_out(&overlay, tree, animator, now);
            self.dismissing.push(overlay);
        }
    }

    /// Starts fade-out animations on an overlay's compositor layers.
    fn start_fade_out(
        overlay: &Overlay,
        tree: &LayerTree,
        animator: &mut LayerAnimator,
        now: Instant,
    ) {
        let params = AnimationParams {
            duration: FADE_DURATION,
            easing: Easing::EaseOut,
            tree,
            now,
        };
        animator.animate_opacity(overlay.layer_id, 0.0, &params);
        if let Some(dim_id) = overlay.dim_layer_id {
            animator.animate_opacity(dim_id, 0.0, &params);
        }
    }
}
