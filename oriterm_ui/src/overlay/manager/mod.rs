//! Overlay manager — lifecycle, event routing, and drawing for floating layers.
//!
//! Sits alongside the widget tree (not inside it). The application layer calls
//! into the manager at specific frame-loop points: events before the main tree,
//! layout after the main tree, drawing after the main tree.

pub(in crate::overlay) mod event_routing;
mod lifecycle;

use std::time::Duration;

use crate::color::Color;
use crate::compositor::layer_tree::LayerTree;
use crate::draw::RectStyle;
use crate::geometry::LayerId;
use crate::geometry::{Point, Rect, Size};
use crate::layout::{LayoutNode, compute_layout};
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;
use crate::widgets::{DrawCtx, LayoutCtx, Widget, WidgetResponse};

use super::overlay_id::OverlayId;
use super::placement::{Placement, compute_overlay_rect};

/// Semi-transparent black for modal dimming.
const MODAL_DIM_COLOR: Color = Color::rgba(0.0, 0.0, 0.0, 0.5);

/// Duration for overlay fade-in and fade-out animations.
const FADE_DURATION: Duration = Duration::from_millis(150);

/// Discriminates overlay behavior: popup vs. modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::overlay) enum OverlayKind {
    /// Non-modal popup — dismissed on click outside.
    Popup,
    /// Modal dialog — blocks interaction below, not dismissable by click outside.
    Modal,
}

/// A floating overlay containing a widget.
pub(in crate::overlay) struct Overlay {
    /// Unique identifier for this overlay.
    pub(in crate::overlay) id: OverlayId,
    /// The widget displayed in this overlay.
    pub(in crate::overlay) widget: Box<dyn Widget>,
    /// Anchor rectangle for placement computation.
    pub(in crate::overlay) anchor: Rect,
    /// Placement strategy relative to anchor.
    pub(in crate::overlay) placement: Placement,
    /// Popup vs. modal behavior.
    pub(in crate::overlay) kind: OverlayKind,
    /// Computed screen-space rectangle (set by `layout_overlays`).
    pub(in crate::overlay) computed_rect: Rect,
    /// Layout tree root for this overlay's widget (set by `layout_overlays`).
    ///
    /// Used by the propagation pipeline to hit-test into the widget tree.
    /// `None` before the first layout pass.
    pub(in crate::overlay) layout_node: Option<LayoutNode>,

    // Compositor integration.
    /// Compositor layer for this overlay's content.
    pub(in crate::overlay) layer_id: LayerId,
    /// Compositor layer for modal dimming (modals only).
    pub(in crate::overlay) dim_layer_id: Option<LayerId>,
}

/// Result of routing an event through the overlay stack.
#[derive(Debug)]
pub enum OverlayEventResult {
    /// Event was delivered to an overlay widget.
    Delivered {
        /// Which overlay received the event.
        overlay_id: OverlayId,
        /// The widget's response.
        response: WidgetResponse,
    },
    /// A click outside dismissed the topmost overlay.
    Dismissed(OverlayId),
    /// A modal overlay blocked the event (consumed without delivery).
    Blocked,
    /// No overlay intercepted the event — deliver to the main widget tree.
    PassThrough,
}

/// Manages a stack of floating overlays above the main widget tree.
///
/// Overlays are ordered back-to-front: the last overlay in the stack is
/// topmost (drawn last, receives events first).
pub struct OverlayManager {
    pub(in crate::overlay) overlays: Vec<Overlay>,
    /// Overlays being faded out — still drawn, but excluded from event routing.
    pub(in crate::overlay) dismissing: Vec<Overlay>,
    pub(in crate::overlay) viewport: Rect,
    /// Index of the overlay currently under the cursor.
    ///
    /// Tracked across `process_hover_event` calls so we can send
    /// `HoverEvent::Leave` to the old overlay when hover transitions.
    pub(in crate::overlay) hovered_overlay: Option<usize>,
    /// Index of the overlay with active mouse capture (drag in progress).
    ///
    /// When set, all mouse events route to this overlay regardless of cursor
    /// position, and click-outside dismiss is suppressed. Cleared on `MouseUp`
    /// or explicit `CaptureRequest::Release`. Benign if the cursor leaves the
    /// window entirely — the next mouse event re-enters and routes correctly.
    pub(in crate::overlay) captured_overlay: Option<usize>,
    /// Whether overlay placement needs recomputation.
    ///
    /// Set on push, remove, or viewport change. Cleared after
    /// `layout_overlays` runs. Avoids expensive `widget.layout()` calls
    /// every frame when overlay positions haven't changed.
    pub(in crate::overlay) layout_dirty: bool,
}

impl OverlayManager {
    // Constructors

    /// Creates a new overlay manager with the given viewport bounds.
    pub fn new(viewport: Rect) -> Self {
        Self {
            overlays: Vec::new(),
            dismissing: Vec::new(),
            viewport,
            hovered_overlay: None,
            captured_overlay: None,
            layout_dirty: false,
        }
    }

    // Accessors

    /// Updates the viewport bounds (e.g. on window resize).
    pub fn set_viewport(&mut self, viewport: Rect) {
        if self.viewport != viewport {
            self.viewport = viewport;
            self.layout_dirty = true;
        }
    }

    /// Returns the current viewport.
    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    // Predicates

    /// Returns `true` if no overlays are active or dismissing.
    pub fn is_empty(&self) -> bool {
        self.overlays.is_empty() && self.dismissing.is_empty()
    }

    /// Returns `true` if no overlays are active (excludes dismissing).
    pub fn is_active_empty(&self) -> bool {
        self.overlays.is_empty()
    }

    /// Returns the number of active overlays.
    pub fn count(&self) -> usize {
        self.overlays.len()
    }

    /// Returns `true` if the topmost overlay is modal.
    pub fn has_modal(&self) -> bool {
        self.overlays
            .last()
            .is_some_and(|o| o.kind == OverlayKind::Modal)
    }

    /// Returns the computed screen-space rectangle for an overlay.
    ///
    /// Returns `None` if the ID is not found. The rect is only valid
    /// after calling [`layout_overlays`](Self::layout_overlays).
    pub fn overlay_rect(&self, id: OverlayId) -> Option<Rect> {
        self.overlays
            .iter()
            .find(|o| o.id == id)
            .map(|o| o.computed_rect)
    }

    /// Offsets the topmost overlay's position by a screen-space delta.
    ///
    /// Used for header-drag repositioning. Switches placement to `AtPoint`
    /// so that subsequent `layout_overlays` calls preserve the dragged
    /// position instead of snapping back to the original placement.
    /// Clamps to keep the overlay within the viewport.
    pub fn offset_topmost(&mut self, dx: f32, dy: f32) -> bool {
        let Some(overlay) = self.overlays.last_mut() else {
            return false;
        };
        let vp = self.viewport;
        let r = overlay.computed_rect;
        let new_x = (r.x() + dx).clamp(0.0, (vp.width() - r.width()).max(0.0));
        let new_y = (r.y() + dy).clamp(0.0, (vp.height() - r.height()).max(0.0));
        overlay.computed_rect = Rect::new(new_x, new_y, r.width(), r.height());
        overlay.placement = Placement::AtPoint(Point::new(new_x, new_y));
        true
    }

    /// Visits each active overlay's root widget mutably.
    ///
    /// Used by the framework pipeline to walk overlay widget trees for
    /// lifecycle delivery, animation ticks, and visual state updates.
    pub fn for_each_widget_mut(&mut self, mut visitor: impl FnMut(&mut dyn Widget)) {
        for overlay in &mut self.overlays {
            visitor(overlay.widget.as_mut());
        }
    }

    // Frame-loop API

    /// Computes layout for all overlays (active + dismissing).
    ///
    /// For each overlay: measures the widget's intrinsic size via the layout
    /// solver, then computes the screen-space placement rectangle.
    pub fn layout_overlays(
        &mut self,
        measurer: &dyn crate::widgets::TextMeasurer,
        theme: &UiTheme,
    ) {
        if !self.layout_dirty {
            return;
        }

        let viewport = self.viewport;
        let layout_ctx = LayoutCtx { measurer, theme };

        for overlay in self.overlays.iter_mut().chain(self.dismissing.iter_mut()) {
            let layout_box = overlay.widget.layout(&layout_ctx);
            // Use viewport as constraint so `Fill`-sized widgets resolve to
            // the viewport dimensions instead of zero (infinite available space
            // gives fill children zero remaining space). `Hug`-sized widgets
            // are unaffected — they use intrinsic size regardless.
            let node = compute_layout(&layout_box, viewport);
            let content_size = Size::new(node.rect.width(), node.rect.height());

            overlay.computed_rect =
                compute_overlay_rect(overlay.anchor, content_size, viewport, overlay.placement);
            overlay.layout_node = Some(node);
        }

        self.layout_dirty = false;
    }

    /// Returns the total number of overlays to draw (active + dismissing).
    pub fn draw_count(&self) -> usize {
        self.overlays.len() + self.dismissing.len()
    }

    /// Draws a single overlay at `draw_idx` and returns its compositor opacity.
    ///
    /// Indices `0..active_count` draw active overlays; the rest draw dismissing
    /// overlays. Modal overlays emit a dimming rectangle before the content.
    /// Panics if `draw_idx >= draw_count()`.
    pub fn draw_overlay_at(&self, draw_idx: usize, ctx: &mut DrawCtx<'_>, tree: &LayerTree) -> f32 {
        let overlay = if draw_idx < self.overlays.len() {
            &self.overlays[draw_idx]
        } else {
            &self.dismissing[draw_idx - self.overlays.len()]
        };

        let opacity = tree
            .get(overlay.layer_id)
            .map_or(1.0, |l| l.properties().opacity);

        // Modal dim — apply dim layer's own opacity to the color alpha.
        if overlay.kind == OverlayKind::Modal {
            let dim_opacity = overlay
                .dim_layer_id
                .and_then(|id| tree.get(id))
                .map_or(1.0, |l| l.properties().opacity);
            let dim_color = Color::rgba(
                MODAL_DIM_COLOR.r,
                MODAL_DIM_COLOR.g,
                MODAL_DIM_COLOR.b,
                MODAL_DIM_COLOR.a * dim_opacity,
            );
            ctx.draw_list
                .push_rect(self.viewport, RectStyle::filled(dim_color));
        }

        // Content widget draws at full alpha — the returned opacity is
        // applied by the GPU converter to all emitted instances.
        let mut overlay_ctx = DrawCtx {
            measurer: ctx.measurer,
            draw_list: ctx.draw_list,
            bounds: overlay.computed_rect,
            focused_widget: ctx.focused_widget,
            now: ctx.now,
            theme: ctx.theme,
            icons: ctx.icons,
            scene_cache: None,
            interaction: None,
            widget_id: None,
            frame_requests: None,
        };
        overlay.widget.paint(&mut overlay_ctx);

        opacity
    }

    /// Returns focusable widget IDs from the topmost modal overlay.
    ///
    /// The application layer can use this with `FocusManager::set_focus_order()`
    /// to trap focus within the modal. Returns `None` if there is no modal.
    pub fn modal_focus_order(&self) -> Option<Vec<WidgetId>> {
        let topmost = self.overlays.last()?;
        if topmost.kind != OverlayKind::Modal {
            return None;
        }
        Some(topmost.widget.focusable_children())
    }

    /// Propagates an action to the topmost overlay's widget tree.
    ///
    /// Used to update child widget state after an external action (e.g.,
    /// updating a dropdown's selected index after its popup menu was dismissed).
    pub fn accept_action_topmost(&mut self, action: &crate::widgets::WidgetAction) -> bool {
        self.overlays
            .last_mut()
            .is_some_and(|o| o.widget.accept_action(action))
    }
}
