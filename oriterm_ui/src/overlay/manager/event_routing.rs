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

use crate::compositor::layer_animator::LayerAnimator;
use crate::compositor::layer_tree::LayerTree;
use crate::controllers::{ControllerRequests, DispatchOutput};
use crate::geometry::{Point, Rect};
use crate::input::dispatch::tree::{TreeDispatchResult, dispatch_to_widget_tree};
use crate::input::{
    EventResponse, HitEntry, InputEvent, Key, KeyEvent, MouseEvent, MouseEventKind,
    WidgetHitTestResult, layout_hit_test_path, plan_propagation,
};
use crate::layout::LayoutNode;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;
use crate::widgets::{CaptureRequest, Widget, WidgetResponse};

use super::{OverlayEventResult, OverlayKind, OverlayManager};

/// Converts a [`DispatchOutput`] into a [`WidgetResponse`].
///
/// Used by the overlay event routing layer to translate controller dispatch
/// results into the `WidgetResponse` type that `OverlayEventResult` expects.
///
/// Mapping:
/// - `handled` → `EventResponse::Handled` / `Ignored`
/// - `PAINT` request → `RequestPaint`
/// - `REQUEST_FOCUS` → `RequestFocus`
/// - `SET_ACTIVE` → `CaptureRequest::Acquire`
/// - `CLEAR_ACTIVE` → `CaptureRequest::Release`
/// - First emitted action taken (singular slot in `WidgetResponse`)
pub(in crate::overlay) fn bridge_dispatch_to_response(
    output: DispatchOutput,
    source: WidgetId,
) -> WidgetResponse {
    let response = if output.requests.contains(ControllerRequests::PAINT) {
        EventResponse::RequestPaint
    } else if output.requests.contains(ControllerRequests::REQUEST_FOCUS) {
        EventResponse::RequestFocus
    } else if output.handled {
        EventResponse::Handled
    } else {
        EventResponse::Ignored
    };

    let capture = if output.requests.contains(ControllerRequests::SET_ACTIVE) {
        CaptureRequest::Acquire
    } else if output.requests.contains(ControllerRequests::CLEAR_ACTIVE) {
        CaptureRequest::Release
    } else {
        CaptureRequest::None
    };

    WidgetResponse {
        response,
        action: output.actions.into_iter().next(),
        capture,
        source: Some(source),
    }
}

/// Runs the propagation pipeline for an overlay widget tree.
///
/// Hit-tests the overlay's layout tree, plans Capture → Target → Bubble
/// delivery, then walks the widget tree to dispatch to controllers at
/// each matching widget.
///
/// Returns `Some(response)` if any controller handled the event.
/// Returns `None` if no widget in the hit path has controllers or none handled.
#[expect(
    clippy::too_many_arguments,
    reason = "pipeline dispatch: widget, event, rect, layout, captured, now"
)]
fn deliver_via_pipeline(
    widget: &mut dyn Widget,
    event: &InputEvent,
    overlay_rect: Rect,
    layout_node: Option<&LayoutNode>,
    captured: bool,
    now: Instant,
) -> Option<WidgetResponse> {
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
    dispatch_to_widget_tree(widget, event, &delivery_actions, now, &mut result);

    if result.handled || !result.actions.is_empty() {
        let output = DispatchOutput {
            requests: result.requests,
            actions: result.actions,
            handled: result.handled,
        };
        Some(bridge_dispatch_to_response(
            output,
            result.source.unwrap_or(root_id),
        ))
    } else {
        None
    }
}

impl OverlayManager {
    /// Routes a mouse event through the overlay stack.
    ///
    /// Hit-tests overlays back-to-front (topmost first). The `focused_widget`
    /// parameter indicates which widget currently has keyboard focus (from the
    /// app layer's `FocusManager`). See [`OverlayEventResult`] for routing rules.
    ///
    /// Click-outside dismissals start a fade-out animation via the compositor.
    #[expect(
        clippy::too_many_arguments,
        reason = "event routing: event, measurer, theme, focus, tree, animator, now"
    )]
    pub fn process_mouse_event(
        &mut self,
        event: &MouseEvent,
        measurer: &dyn crate::widgets::TextMeasurer,
        theme: &UiTheme,
        focused_widget: Option<WidgetId>,
        tree: &mut LayerTree,
        animator: &mut LayerAnimator,
        now: Instant,
    ) -> OverlayEventResult {
        if self.overlays.is_empty() {
            return OverlayEventResult::PassThrough;
        }

        // Newly pushed overlays may receive input before the next redraw.
        // Ensure placement is current so hit-testing works immediately.
        self.layout_overlays(measurer, theme);

        // During capture, route all events to the captured overlay.
        if let Some(cap_idx) = self.captured_overlay {
            if let Some(overlay) = self.overlays.get_mut(cap_idx) {
                let id = overlay.id;
                let root_id = overlay.widget.id();
                let input_event = InputEvent::from_mouse_event(event);
                let mut response = deliver_via_pipeline(
                    overlay.widget.as_mut(),
                    &input_event,
                    overlay.computed_rect,
                    overlay.layout_node.as_ref(),
                    true,
                    now,
                )
                .unwrap_or_else(|| WidgetResponse::ignored().with_source(root_id));
                response.inject_source(root_id);
                match response.capture {
                    CaptureRequest::Release => self.captured_overlay = None,
                    CaptureRequest::None if matches!(event.kind, MouseEventKind::Up(_)) => {
                        self.captured_overlay = None;
                    }
                    _ => {}
                }
                if response.response.needs_layout() {
                    self.layout_dirty = true;
                }
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
                    self.begin_dismiss_topmost(tree, animator, now);
                    return OverlayEventResult::Dismissed(topmost_id);
                }
            }
        }

        // Scroll events: route to the topmost popup if one exists, even
        // when the cursor is over a modal below. This prevents the modal's
        // scroll widget from stealing wheel events intended for the popup
        // (e.g. a scrollable dropdown list over the settings panel).
        if matches!(event.kind, MouseEventKind::Scroll(_)) {
            if let Some(result) =
                self.route_scroll_to_popup(event, measurer, focused_widget, theme, now)
            {
                return result;
            }
            // No popup — fall through to normal hit-test for modals.
        }

        // Hit test from topmost to bottom.
        for i in (0..self.overlays.len()).rev() {
            if self.overlays[i].computed_rect.contains(event.pos) {
                let result =
                    self.deliver_to_overlay(i, event, measurer, focused_widget, theme, now);
                if let OverlayEventResult::Delivered { ref response, .. } = result {
                    if response.capture == CaptureRequest::Acquire {
                        self.captured_overlay = Some(i);
                    }
                }
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
                    self.begin_dismiss_topmost(tree, animator, now);
                    OverlayEventResult::Dismissed(topmost_id)
                } else {
                    OverlayEventResult::PassThrough
                }
            }
        }
    }

    /// Routes a scroll event to the topmost popup overlay.
    ///
    /// Returns `Some(result)` if a popup was found and the event was
    /// delivered. Returns `None` if no popup exists (caller should fall
    /// through to normal hit-test routing).
    #[expect(
        clippy::too_many_arguments,
        reason = "internal helper, params mirror caller + now for controller dispatch"
    )]
    fn route_scroll_to_popup(
        &mut self,
        event: &MouseEvent,
        measurer: &dyn crate::widgets::TextMeasurer,
        focused_widget: Option<WidgetId>,
        theme: &UiTheme,
        now: Instant,
    ) -> Option<OverlayEventResult> {
        let idx = self
            .overlays
            .iter()
            .rposition(|o| o.kind == OverlayKind::Popup)?;
        Some(self.deliver_to_overlay(idx, event, measurer, focused_widget, theme, now))
    }

    /// Delivers a mouse event to a specific overlay by index.
    ///
    /// Delivers a mouse event to a specific overlay by index.
    ///
    /// Runs the propagation pipeline through the overlay's widget tree.
    #[expect(
        clippy::too_many_arguments,
        reason = "internal helper, params mirror caller"
    )]
    fn deliver_to_overlay(
        &mut self,
        idx: usize,
        event: &MouseEvent,
        _measurer: &dyn crate::widgets::TextMeasurer,
        _focused_widget: Option<WidgetId>,
        _theme: &UiTheme,
        now: Instant,
    ) -> OverlayEventResult {
        let overlay = &mut self.overlays[idx];
        let id = overlay.id;
        let root_id = overlay.widget.id();
        let input_event = InputEvent::from_mouse_event(event);
        let mut response = deliver_via_pipeline(
            overlay.widget.as_mut(),
            &input_event,
            overlay.computed_rect,
            overlay.layout_node.as_ref(),
            false,
            now,
        )
        .unwrap_or_else(|| WidgetResponse::ignored().with_source(root_id));
        response.inject_source(root_id);
        if response.response.needs_layout() {
            self.layout_dirty = true;
        }
        OverlayEventResult::Delivered {
            overlay_id: id,
            response,
        }
    }

    /// Routes a key event through the overlay stack.
    ///
    /// Escape dismisses the topmost overlay with a fade-out animation.
    /// Modal overlays never pass through. The `focused_widget` parameter
    /// indicates which widget currently has keyboard focus (from the app
    /// layer's `FocusManager`).
    ///
    /// Routes a key event through the overlay stack.
    ///
    /// Escape dismisses the topmost overlay with a fade-out animation.
    /// Modal overlays never pass through.
    #[expect(
        clippy::too_many_arguments,
        reason = "event routing: event, measurer, theme, focus, tree, animator, now"
    )]
    pub fn process_key_event(
        &mut self,
        event: KeyEvent,
        _measurer: &dyn crate::widgets::TextMeasurer,
        _theme: &UiTheme,
        _focused_widget: Option<WidgetId>,
        tree: &mut LayerTree,
        animator: &mut LayerAnimator,
        now: Instant,
    ) -> OverlayEventResult {
        if self.overlays.is_empty() {
            return OverlayEventResult::PassThrough;
        }

        // Escape always dismisses topmost.
        if event.key == Key::Escape {
            let id = self
                .begin_dismiss_topmost(tree, animator, now)
                .expect("checked non-empty above");
            return OverlayEventResult::Dismissed(id);
        }

        let topmost = self.overlays.last_mut().expect("checked non-empty above");
        let id = topmost.id;
        let is_modal = topmost.kind == OverlayKind::Modal;
        let root_id = topmost.widget.id();
        let input_event = InputEvent::from_key_event(event);
        let mut response = deliver_via_pipeline(
            topmost.widget.as_mut(),
            &input_event,
            topmost.computed_rect,
            topmost.layout_node.as_ref(),
            false,
            now,
        )
        .unwrap_or_else(|| WidgetResponse::ignored().with_source(root_id));
        response.inject_source(root_id);
        if response.response.needs_layout() {
            self.layout_dirty = true;
        }

        if response.response.is_handled() || is_modal {
            OverlayEventResult::Delivered {
                overlay_id: id,
                response,
            }
        } else {
            OverlayEventResult::PassThrough
        }
    }

    /// Routes a hover event through the overlay stack.
    ///
    /// Tracks which overlay was previously hovered. When the cursor moves
    /// between overlays, sends `HoverEvent::Leave` to the old overlay and
    /// `HoverEvent::Enter` to the new one.
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
                let root_id = overlay.widget.id();
                let mut response = WidgetResponse::paint();
                response.inject_source(root_id);
                OverlayEventResult::Delivered {
                    overlay_id: overlay.id,
                    response,
                }
            }
            Some(idx) => {
                // Hover unchanged, still over this overlay — no re-enter.
                OverlayEventResult::Delivered {
                    overlay_id: self.overlays[idx].id,
                    response: WidgetResponse::handled(),
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
