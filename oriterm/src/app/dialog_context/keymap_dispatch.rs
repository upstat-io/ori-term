//! Keymap-aware keyboard dispatch for dialog windows.
//!
//! Intercepts `KeyDown` events and tries keymap lookup before falling
//! through to the normal controller pipeline via `deliver_event_to_tree`.

use std::time::Instant;

use oriterm_ui::action::{Keystroke, build_context_stack};
use oriterm_ui::controllers::ControllerRequests;
use oriterm_ui::geometry::Rect;
use oriterm_ui::input::InputEvent;
use oriterm_ui::input::dispatch::tree::{TreeDispatchResult, deliver_event_to_tree};
use oriterm_ui::pipeline::dispatch_keymap_action;

/// Dispatches a keyboard input event through the keymap or controller pipeline.
///
/// Tries keymap lookup first for `KeyDown` events. If a binding matches,
/// dispatches the action directly. Otherwise falls through to the normal
/// `deliver_event_to_tree` controller pipeline.
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors deliver_event_to_tree params"
)]
pub(super) fn dispatch_dialog_key_event(
    input_event: &InputEvent,
    ctx: &mut super::DialogWindowContext,
    focus_path: &[oriterm_ui::widget_id::WidgetId],
    active: Option<oriterm_ui::widget_id::WidgetId>,
    content_bounds: Rect,
    layout_node: &oriterm_ui::layout::LayoutNode,
    now: Instant,
    #[cfg(debug_assertions)] layout_ids: &std::collections::HashSet<
        oriterm_ui::widget_id::WidgetId,
    >,
) -> TreeDispatchResult {
    if let InputEvent::KeyDown { key, modifiers } = *input_event {
        let keystroke = Keystroke::new(key, modifiers);
        let context_stack = build_context_stack(ctx.root.key_contexts(), focus_path);
        if let Some(action) = ctx.root.keymap().lookup(keystroke, &context_stack) {
            let mut r = TreeDispatchResult::new();
            r.handled = true;
            match action.name() {
                "widget::FocusNext" => {
                    r.requests = ControllerRequests::FOCUS_NEXT;
                    r.source = focus_path.last().copied();
                }
                "widget::FocusPrev" => {
                    r.requests = ControllerRequests::FOCUS_PREV;
                    r.source = focus_path.last().copied();
                }
                _ => {
                    if let Some(focused_id) = focus_path.last().copied() {
                        if let Some(widget_action) = dispatch_keymap_action(
                            ctx.content.content_widget_mut(),
                            focused_id,
                            &*action,
                            content_bounds,
                        ) {
                            r.actions.push(widget_action);
                        }
                        r.source = Some(focused_id);
                    }
                }
            }
            ctx.root.set_last_keymap_handled(Some(key));
            return r;
        }
    }

    deliver_event_to_tree(
        ctx.content.content_widget_mut(),
        input_event,
        content_bounds,
        Some(layout_node),
        active,
        focus_path,
        now,
        #[cfg(debug_assertions)]
        Some(layout_ids),
        #[cfg(not(debug_assertions))]
        None,
    )
}
