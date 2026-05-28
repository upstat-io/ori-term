//! Event routing through the overlay stack.
//!
//! Mouse events, key events, and hover events are routed through the overlay
//! stack before reaching the main widget tree. Dismissals trigger compositor
//! fade-out animations.
//!
//! Dispatch uses a two-phase propagation pipeline: hit-test the overlay's
//! layout tree, plan Capture → Target → Bubble delivery, then walk the widget
//! tree to dispatch to controllers at each matching widget.

use std::time::Instant;

use winit::window::CursorIcon;

use crate::controllers::{ControllerRequests, DispatchOutput};
use crate::geometry::{Point, Rect};
use crate::input::dispatch::tree::{DispatchInputs, TreeDispatchResult, dispatch_to_widget_tree};
use crate::input::{
    HitEntry, InputEvent, MouseEvent, MouseEventKind, WidgetHitTestResult, layout_hit_test_path,
    plan_propagation,
};
use crate::layout::LayoutNode;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;
use crate::widgets::{LayoutCtx, Widget};

use super::{CompositorHandles, OverlayEventResult, OverlayKind, OverlayManager, OverlayResponse};

/// Read-only context for one overlay propagation-pipeline dispatch.
#[derive(Clone, Copy)]
pub(in crate::overlay::manager) struct PipelineCtx<'a> {
    /// The input event being delivered.
    pub event: &'a InputEvent,
    /// Screen-space rectangle of the overlay.
    pub overlay_rect: Rect,
    /// The overlay's laid-out widget tree, if a layout pass has run.
    pub layout_node: Option<&'a LayoutNode>,
    /// Whether the overlay currently holds mouse capture.
    pub captured: bool,
    /// Current frame timestamp.
    pub now: Instant,
}

/// Runs the propagation pipeline for an overlay widget tree.
///
/// Hit-tests the overlay's layout tree, plans Capture → Target → Bubble
/// delivery, then walks the widget tree to dispatch to controllers at
/// each matching widget.
///
/// Returns `Some((output, source))` if any controller handled the event.
/// Returns `None` if no widget in the hit path has controllers or none handled.
pub(in crate::overlay::manager) fn deliver_via_pipeline(
    widget: &mut dyn Widget,
    ctx: PipelineCtx<'_>,
) -> Option<(DispatchOutput, WidgetId)> {
    let PipelineCtx {
        event,
        overlay_rect,
        layout_node,
        captured,
        now,
    } = ctx;
    let root_id = widget.id();
    let root_sense = widget.sense();

    // Build the hit path for plan_propagation.
    let hit_result = if event.is_keyboard() {
        WidgetHitTestResult { path: Vec::new() }
    } else if captured {
        WidgetHitTestResult {
            path: vec![HitEntry {
                widget_id: root_id,
                bounds: overlay_rect,
                sense: root_sense,
                cursor_icon: CursorIcon::Default,
            }],
        }
    } else if let Some(node) = layout_node {
        if let Some(pos) = event.pos() {
            let local = Point::new(pos.x - overlay_rect.x(), pos.y - overlay_rect.y());
            let mut result = layout_hit_test_path(node, local);
            // Hit test returns local-space bounds. Offset to overlay-space
            // so controller bounds match the screen-space event coordinates.
            for entry in &mut result.path {
                entry.bounds = Rect::new(
                    entry.bounds.x() + overlay_rect.x(),
                    entry.bounds.y() + overlay_rect.y(),
                    entry.bounds.width(),
                    entry.bounds.height(),
                );
            }
            result
        } else {
            WidgetHitTestResult { path: Vec::new() }
        }
    } else {
        WidgetHitTestResult {
            path: vec![HitEntry {
                widget_id: root_id,
                bounds: overlay_rect,
                sense: root_sense,
                cursor_icon: CursorIcon::Default,
            }],
        }
    };

    // Plan propagation.
    let focus_path = if event.is_keyboard() {
        vec![root_id]
    } else {
        Vec::new()
    };
    let active_widget = if captured { Some(root_id) } else { None };
    let mut delivery_actions = Vec::new();
    plan_propagation(
        event,
        &hit_result,
        active_widget,
        &focus_path,
        &mut delivery_actions,
    );

    if delivery_actions.is_empty() {
        return None;
    }

    // Walk the widget tree and dispatch to controllers of matching widgets.
    let mut result = TreeDispatchResult::new();
    dispatch_to_widget_tree(
        widget,
        DispatchInputs {
            event,
            actions: &delivery_actions,
            now,
        },
        &mut result,
        None,
    );

    if result.handled || !result.actions.is_empty() {
        let output = DispatchOutput {
            requests: result.requests,
            actions: result.actions,
            handled: result.handled,
        };
        Some((output, result.source.unwrap_or(root_id)))
    } else {
        None
    }
}

impl OverlayManager {
    /// Routes a mouse event through the overlay stack.
    ///
    /// Hit-tests overlays back-to-front (topmost first). See
    /// [`OverlayEventResult`] for routing rules.
    ///
    /// Click-outside dismissals start a fade-out animation via the compositor.
    pub fn process_mouse_event(
        &mut self,
        event: &MouseEvent,
        layout: &LayoutCtx<'_>,
        compositor: &mut CompositorHandles<'_>,
        now: Instant,
    ) -> OverlayEventResult {
        if self.overlays.is_empty() {
            return OverlayEventResult::PassThrough;
        }

        // Newly pushed overlays may receive input before the next redraw.
        // Ensure placement is current so hit-testing works immediately.
        self.layout_overlays(layout.measurer, layout.theme);

        // During capture, route all events to the captured overlay.
        if let Some(cap_idx) = self.captured_overlay {
            if let Some(overlay) = self.overlays.get_mut(cap_idx) {
                let id = overlay.id;
                let input_event = InputEvent::from_mouse_event(event);
                let pipeline_result = deliver_via_pipeline(
                    overlay.widget.as_mut(),
                    PipelineCtx {
                        event: &input_event,
                        overlay_rect: overlay.computed_rect,
                        layout_node: overlay.layout_node.as_ref(),
                        captured: true,
                        now,
                    },
                );
                let response = if let Some((output, _source)) = pipeline_result {
                    // Release capture on explicit CLEAR_ACTIVE or implicit mouse-up.
                    if output.requests.contains(ControllerRequests::CLEAR_ACTIVE)
                        || matches!(event.kind, MouseEventKind::Up(_))
                    {
                        self.captured_overlay = None;
                    }
                    OverlayResponse {
                        action: output.actions.into_iter().next(),
                        handled: output.handled,
                    }
                } else {
                    // No controller handled — implicit release on mouse up.
                    if matches!(event.kind, MouseEventKind::Up(_)) {
                        self.captured_overlay = None;
                    }
                    OverlayResponse {
                        action: None,
                        handled: false,
                    }
                };
                return OverlayEventResult::Delivered {
                    overlay_id: id,
                    response,
                };
            }
            // Captured overlay no longer exists — clear stale capture.
            self.captured_overlay = None;
        }

        // Auto-dismiss topmost popup on click outside it (even if click lands
        // inside a lower overlay like a modal). Standard dropdown behavior:
        // the click is consumed, the user clicks again to interact below.
        if matches!(event.kind, MouseEventKind::Down(_)) {
            if let Some(topmost) = self.overlays.last() {
                if topmost.kind == OverlayKind::Popup && !topmost.computed_rect.contains(event.pos)
                {
                    let topmost_id = topmost.id;
                    self.begin_dismiss_topmost(compositor.tree, compositor.animator, now);
                    return OverlayEventResult::Dismissed(topmost_id);
                }
            }
        }

        // Scroll events: route to the topmost popup if one exists, even
        // when the cursor is over a modal below. This prevents the modal's
        // scroll widget from stealing wheel events intended for the popup
        // (e.g. a scrollable dropdown list over the settings panel).
        if matches!(event.kind, MouseEventKind::Scroll(_)) {
            if let Some(result) = self.route_scroll_to_popup(event, now) {
                return result;
            }
            // No popup — fall through to normal hit-test for modals.
        }

        // Hit test from topmost to bottom.
        for i in (0..self.overlays.len()).rev() {
            if self.overlays[i].computed_rect.contains(event.pos) {
                let result = self.deliver_to_overlay(i, event, now);
                return result;
            }
        }

        // Click is outside all overlays — check topmost overlay's policy.
        let topmost = self.overlays.last().expect("checked non-empty above");
        let topmost_id = topmost.id;

        match topmost.kind {
            OverlayKind::Modal => OverlayEventResult::Blocked,
            OverlayKind::Popup => {
                // Only dismiss on actual clicks (Down), not moves/scrolls.
                if matches!(event.kind, MouseEventKind::Down(_)) {
                    self.begin_dismiss_topmost(compositor.tree, compositor.animator, now);
                    OverlayEventResult::Dismissed(topmost_id)
                } else {
                    OverlayEventResult::PassThrough
                }
            }
            // Flash overlays live exclusively on `self.dismissing`; they
            // never appear on `self.overlays`. `debug_assert!` documents the
            // invariant + catches violations in debug builds; release falls
            // through to `PassThrough` so user-input handlers never panic.
            OverlayKind::Flash => {
                debug_assert!(
                    false,
                    "OverlayKind::Flash must live on dismissing list, not active overlays"
                );
                OverlayEventResult::PassThrough
            }
        }
    }

    /// Routes a scroll event to the topmost popup overlay.
    ///
    /// Returns `Some(result)` if a popup was found and the event was
    /// delivered. Returns `None` if no popup exists (caller should fall
    /// through to normal hit-test routing).
    fn route_scroll_to_popup(
        &mut self,
        event: &MouseEvent,
        now: Instant,
    ) -> Option<OverlayEventResult> {
        let idx = self
            .overlays
            .iter()
            .rposition(|o| o.kind == OverlayKind::Popup)?;
        Some(self.deliver_to_overlay(idx, event, now))
    }

    /// Delivers a mouse event to a specific overlay by index.
    ///
    /// Runs the propagation pipeline through the overlay's widget tree.
    /// Handles capture acquisition from controller requests internally.
    fn deliver_to_overlay(
        &mut self,
        idx: usize,
        event: &MouseEvent,
        now: Instant,
    ) -> OverlayEventResult {
        let overlay = &mut self.overlays[idx];
        let id = overlay.id;
        let input_event = InputEvent::from_mouse_event(event);
        let pipeline_result = deliver_via_pipeline(
            overlay.widget.as_mut(),
            PipelineCtx {
                event: &input_event,
                overlay_rect: overlay.computed_rect,
                layout_node: overlay.layout_node.as_ref(),
                captured: false,
                now,
            },
        );
        let response = match pipeline_result {
            Some((output, _source)) => {
                // Acquire capture if controller requested SET_ACTIVE.
                if output.requests.contains(ControllerRequests::SET_ACTIVE) {
                    self.captured_overlay = Some(idx);
                }
                OverlayResponse {
                    action: output.actions.into_iter().next(),
                    handled: output.handled,
                }
            }
            None => OverlayResponse {
                action: None,
                handled: false,
            },
        };
        OverlayEventResult::Delivered {
            overlay_id: id,
            response,
        }
    }

    /// Routes a hover event through the overlay stack.
    ///
    /// Tracks which overlay was previously hovered. When the cursor moves
    /// between overlays, delivers `LifecycleEvent::HotChanged` to the old
    /// overlay and the new one.
    ///
    /// Hover enter/leave are lifecycle events in the new controller model
    /// (`LifecycleEvent::HotChanged`), not input events. Migration to
    /// `InteractionManager`-driven hover tracking will happen when overlays
    /// integrate with the full widget tree (§08.6+).
    pub fn process_hover_event(
        &mut self,
        point: Point,
        measurer: &dyn crate::widgets::TextMeasurer,
        theme: &UiTheme,
        _focused_widget: Option<WidgetId>,
    ) -> OverlayEventResult {
        if self.overlays.is_empty() {
            self.hovered_overlay = None;
            return OverlayEventResult::PassThrough;
        }

        // Hover hit-testing must see the latest placement even before a redraw.
        self.layout_overlays(measurer, theme);

        // Find topmost overlay containing the point.
        let new_hover = (0..self.overlays.len())
            .rev()
            .find(|&i| self.overlays[i].computed_rect.contains(point));

        let hover_changed = self.hovered_overlay != new_hover;

        if hover_changed {
            self.hovered_overlay = new_hover;
        }

        // Hover enter/leave visual state changes are driven by the
        // InteractionManager + LifecycleEvent::HotChanged pipeline,
        // not by explicit handle_hover calls. We just track which
        // overlay is hovered for event routing purposes.
        match new_hover {
            Some(idx) if hover_changed => {
                let overlay = &self.overlays[idx];
                OverlayEventResult::Delivered {
                    overlay_id: overlay.id,
                    response: OverlayResponse {
                        action: None,
                        handled: true,
                    },
                }
            }
            Some(idx) => {
                // Hover unchanged, still over this overlay — no re-enter.
                OverlayEventResult::Delivered {
                    overlay_id: self.overlays[idx].id,
                    response: OverlayResponse {
                        action: None,
                        handled: true,
                    },
                }
            }
            None => {
                // Point is outside all overlays.
                if self.has_modal() {
                    OverlayEventResult::Blocked
                } else {
                    OverlayEventResult::PassThrough
                }
            }
        }
    }
}
