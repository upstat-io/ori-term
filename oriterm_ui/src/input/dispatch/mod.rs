//! Two-phase event propagation pipeline.
//!
//! Replaces `InputState` (single-pass routing) with Capture + Bubble
//! propagation inspired by WPF's Preview/Bubble and GTK4's three-phase
//! controller model.
//!
//! `plan_propagation()` is a pure routing function: it reads a hit path
//! and active-widget state, then writes a sequence of `DeliveryAction`s
//! into a caller-owned buffer. The caller iterates actions and delivers
//! each to the appropriate widget, stopping on first handled response.
//!
//! `tree` submodule provides `dispatch_to_widget_tree` and
//! `deliver_event_to_tree` for walking a widget tree and dispatching
//! to controllers at each matching node.

pub mod tree;

use crate::geometry::Rect;
use crate::widget_id::WidgetId;

use super::event::EventPhase;
use super::hit_test::WidgetHitTestResult;

/// A single delivery action in the propagation sequence.
///
/// Produced by `plan_propagation()`. The caller delivers each action to
/// the target widget in order, stopping when a widget handles the event.
#[derive(Debug, Clone, PartialEq)]
pub struct DeliveryAction {
    /// Target widget to receive the event.
    pub widget_id: WidgetId,
    /// Propagation phase for this delivery.
    pub phase: EventPhase,
    /// Layout bounds of the target widget (from `HitEntry`).
    pub bounds: Rect,
}

/// Plans the propagation sequence for an input event.
///
/// This is a **pure routing function** — it does not touch the widget tree,
/// does not allocate, and has no side effects. It writes into a caller-owned
/// `&mut Vec<DeliveryAction>` buffer (cleared and refilled; caller retains
/// capacity across frames).
///
/// # Mouse events
///
/// When `active_widget` is `Some`:
/// - `MouseMove` and `MouseUp`: single `Target`-phase delivery to the active
///   widget (capture bypass — no hit testing, no capture/bubble phases).
/// - `Scroll`: always uses normal hit-test routing even during capture
///   (scroll containers must work during a drag).
/// - `MouseDown`: normal two-phase propagation (shouldn't happen during
///   capture, but handled gracefully).
///
/// When no active widget: full two-phase propagation using `hit_path`.
///
/// # Keyboard events
///
/// Routed through the `focus_path` (root-to-leaf ancestor chain of the
/// focused widget). Capture phase walks root to focused, Target is the
/// focused widget, Bubble walks focused back to root.
///
/// # Arguments
///
/// - `event` — the input event to route.
/// - `hit_path` — result of `layout_hit_test_path()` for mouse events.
///   Ignored for keyboard events.
/// - `active_widget` — currently captured widget, if any.
/// - `focus_path` — root-to-leaf ancestor path for keyboard routing.
///   Ignored for mouse events.
/// - `out` — output buffer, cleared before writing.
pub fn plan_propagation(
    event: &super::event::InputEvent,
    hit_path: &WidgetHitTestResult,
    active_widget: Option<WidgetId>,
    focus_path: &[WidgetId],
    out: &mut Vec<DeliveryAction>,
) {
    out.clear();

    if event.is_keyboard() {
        plan_keyboard_propagation(focus_path, out);
        return;
    }

    // Mouse event routing.
    if let Some(active_id) = active_widget {
        plan_captured_mouse(event, active_id, hit_path, out);
    } else {
        plan_mouse_propagation(hit_path, out);
    }
}

/// Two-phase propagation for mouse events through the hit path.
fn plan_mouse_propagation(hit_path: &WidgetHitTestResult, out: &mut Vec<DeliveryAction>) {
    if hit_path.path.is_empty() {
        return;
    }

    let path = &hit_path.path;
    let last = path.len() - 1;

    // Capture phase: root → target.
    for entry in path {
        out.push(DeliveryAction {
            widget_id: entry.widget_id,
            phase: EventPhase::Capture,
            bounds: entry.bounds,
        });
    }

    // Target phase: deepest hit widget.
    out.push(DeliveryAction {
        widget_id: path[last].widget_id,
        phase: EventPhase::Target,
        bounds: path[last].bounds,
    });

    // Bubble phase: target → root (reverse order, skip target itself).
    for entry in path[..last].iter().rev() {
        out.push(DeliveryAction {
            widget_id: entry.widget_id,
            phase: EventPhase::Bubble,
            bounds: entry.bounds,
        });
    }
}

/// Routes mouse events during active capture.
///
/// `MouseMove` and `MouseUp` go directly to the active widget (single
/// Target-phase delivery). `Scroll` uses normal hit-test routing.
/// `MouseDown` uses normal routing (shouldn't happen during capture).
fn plan_captured_mouse(
    event: &super::event::InputEvent,
    active_id: WidgetId,
    hit_path: &WidgetHitTestResult,
    out: &mut Vec<DeliveryAction>,
) {
    use super::event::InputEvent;

    match event {
        InputEvent::MouseMove { .. } | InputEvent::MouseUp { .. } => {
            // Direct delivery to active widget — capture bypass.
            // Use Rect::default() since the widget may be outside the hit path.
            // The active widget's actual bounds are not available from the hit
            // test (pointer may be outside its bounds). The delivery loop can
            // look up bounds from the layout tree if needed.
            out.push(DeliveryAction {
                widget_id: active_id,
                phase: EventPhase::Target,
                bounds: Rect::default(),
            });
        }
        InputEvent::Scroll { .. } => {
            // Scroll always uses normal hit-test routing even during capture.
            plan_mouse_propagation(hit_path, out);
        }
        InputEvent::MouseDown { .. } => {
            // MouseDown during capture: use normal routing (edge case).
            plan_mouse_propagation(hit_path, out);
        }
        InputEvent::KeyDown { .. } | InputEvent::KeyUp { .. } => {
            // Keyboard events don't reach this path (handled above).
        }
    }
}

/// Two-phase propagation for keyboard events through the focus path.
fn plan_keyboard_propagation(focus_path: &[WidgetId], out: &mut Vec<DeliveryAction>) {
    if focus_path.is_empty() {
        return;
    }

    let last = focus_path.len() - 1;
    let bounds = Rect::default();

    // Capture phase: root → focused widget.
    for &id in focus_path {
        out.push(DeliveryAction {
            widget_id: id,
            phase: EventPhase::Capture,
            bounds,
        });
    }

    // Target phase: focused widget.
    out.push(DeliveryAction {
        widget_id: focus_path[last],
        phase: EventPhase::Target,
        bounds,
    });

    // Bubble phase: focused → root (reverse, skip target).
    for &id in focus_path[..last].iter().rev() {
        out.push(DeliveryAction {
            widget_id: id,
            phase: EventPhase::Bubble,
            bounds,
        });
    }
}

#[cfg(test)]
mod tests;
