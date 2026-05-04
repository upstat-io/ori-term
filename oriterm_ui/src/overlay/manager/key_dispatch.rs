//! Key event routing through the overlay stack.
//!
//! Owns BOTH key dispatch entry points on `OverlayManager`:
//!
//! - [`OverlayManager::process_key_event`]: legacy `on_input`-only routing —
//!   special-cases `Escape` to dismiss the topmost overlay, otherwise routes
//!   the event through `deliver_via_pipeline` to `widget.on_input()`. Kept as
//!   the keymap-miss fallback for overlays whose `key_context()` has no
//!   binding in `Keymap::defaults()` (e.g. `SettingsPanel`).
//! - [`OverlayManager::process_key_event_with_keymap`]: keymap-first routing.
//!   Looks up the topmost overlay's context in the
//!   provided keymap and dispatches via
//!   [`crate::pipeline::dispatch_keymap_action`] when a binding matches. On a
//!   `widget::Dismiss` match, the manager translates the result into
//!   [`OverlayEventResult::Dismissed`] directly so dialog-window dispatch
//!   paths (which only handle the `Dismissed` variant, not
//!   `WidgetAction::DismissOverlay`) work correctly. Falls through to
//!   `process_key_event` on a keymap miss.
//!
//! See `bug-tracker/plans/completed//` (post-archival) for the
//! design rationale, including why the dispatch lives on `OverlayManager`
//! (data locality + SRP, mirrors `process_mouse_event`'s pattern) and why
//! the inline `Escape` short-circuit is acceptable as a fallback (mutually
//! exclusive runtime paths after keymap-first).

use std::time::Instant;

use crate::action::{Keymap, Keystroke};
use crate::compositor::layer_animator::LayerAnimator;
use crate::compositor::layer_tree::LayerTree;
use crate::geometry::Rect;
use crate::input::{InputEvent, Key, KeyEvent};
use crate::pipeline::dispatch_keymap_action;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use crate::overlay::overlay_id::OverlayId;

use super::event_routing::deliver_via_pipeline;
use super::{OverlayEventResult, OverlayKind, OverlayManager, OverlayResponse};

impl OverlayManager {
    /// Routes a key event through the overlay stack via the legacy `on_input`
    /// pipeline.
    ///
    /// Escape dismisses the topmost overlay with a fade-out animation.
    /// Modal overlays never pass through. The `focused_widget` parameter
    /// indicates which widget currently has keyboard focus (from the app
    /// layer's `FocusManager`).
    ///
 /// As of this is the keymap-miss fallback for
    /// [`OverlayManager::process_key_event_with_keymap`]; direct callers
    /// (legacy paths that don't supply a keymap) still get the same behavior
    /// they did before the keymap-first dispatch landed.
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

        // Escape always dismisses topmost (fallback for overlays whose
        // `key_context()` has no keymap binding — keymap-bound contexts route
        // Escape through `process_key_event_with_keymap` BEFORE this method
        // is called).
        if event.key == Key::Escape {
            let id = self
                .begin_dismiss_topmost(tree, animator, now)
                .expect("checked non-empty above");
            return OverlayEventResult::Dismissed(id);
        }

        let topmost = self.overlays.last_mut().expect("checked non-empty above");
        let id = topmost.id;
        let is_modal = topmost.kind == OverlayKind::Modal;
        let input_event = InputEvent::from_key_event(event);
        let pipeline_result = deliver_via_pipeline(
            topmost.widget.as_mut(),
            &input_event,
            topmost.computed_rect,
            topmost.layout_node.as_ref(),
            false,
            now,
        );

        let (is_handled, response) = match pipeline_result {
            Some((output, _source)) => {
                let handled = output.handled || !output.actions.is_empty();
                let resp = OverlayResponse {
                    action: output.actions.into_iter().next(),
                    handled: output.handled,
                };
                (handled, resp)
            }
            None => (
                false,
                OverlayResponse {
                    action: None,
                    handled: false,
                },
            ),
        };

        if is_handled || is_modal {
            OverlayEventResult::Delivered {
                overlay_id: id,
                response,
            }
        } else {
            OverlayEventResult::PassThrough
        }
    }

    /// Routes a key event through the overlay stack with keymap-first
 /// dispatch ().
    ///
    /// Resolves the topmost overlay's `key_context()` against the provided
    /// keymap. On a hit:
    /// - `widget::Dismiss` → manager-level dismissal returning
    ///   [`OverlayEventResult::Dismissed`] (so dialog-window paths that
    ///   only handle `Dismissed` work correctly without widget cooperation).
    /// - Anything else → [`crate::pipeline::dispatch_keymap_action`] on the
    ///   overlay's root widget; result wrapped in
    ///   [`OverlayEventResult::Delivered`].
    ///
    /// On a miss, falls through to [`OverlayManager::process_key_event`]
    /// (the legacy `on_input` pipeline plus inline-Escape backstop) so
    /// overlays with no keymap-bound context still dismiss on Escape and
    /// reach `widget.on_input` for non-keymap-routed keys.
    ///
    /// `focus_path` is hardcoded to `[overlay_root_id]` matching the
    /// existing `deliver_via_pipeline` keyboard path. Overlays do not
    /// currently have a focused-child model; future work to plumb a real
    /// overlay focus path would update this method along with
    /// `event_routing::deliver_via_pipeline`.
    #[expect(
        clippy::too_many_arguments,
        reason = "event routing: event, keymap, measurer, theme, focus, tree, animator, now"
    )]
    pub fn process_key_event_with_keymap(
        &mut self,
        event: KeyEvent,
        keymap: &Keymap,
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

        // Phase 1: extract immutable data from topmost. The borrow on
        // `topmost` must die before Phase 3a/3b re-borrows `self`.
        // Overlay focus path is `[root_id]` (no focused-child plumbing per
        // §02 design constraint), so the context stack is at most one entry —
        // skip the HashMap + `build_context_stack` walk that the main-tree
        // path uses (those exist to resolve a multi-level focus path against
        // a many-entry map).
        let (id, root_id, bounds, root_ctx): (OverlayId, WidgetId, Rect, Option<&'static str>) = {
            let topmost = self.overlays.last().expect("checked non-empty above");
            (
                topmost.id,
                topmost.widget.id(),
                topmost.computed_rect,
                topmost.widget.key_context(),
            )
        };

        // Phase 2: keymap lookup (borrows only `keymap`, not `self`).
        // `Option::as_slice` returns `&[T]` of length 0 or 1 from `&Option<T>`
        // — no allocation, exactly the shape `keymap.lookup` consumes.
        let ctx_stack: &[&str] = root_ctx.as_slice();
        let keystroke = Keystroke::new(event.key, event.modifiers);
        let action = match keymap.lookup(keystroke, ctx_stack) {
            Some(a) => a,
            None => {
                // No keymap match — fall through to legacy on_input pipeline
                // (which also handles the inline Escape backstop for
                // unbound contexts).
                return self.process_key_event(
                    event,
                    measurer,
                    theme,
                    focused_widget,
                    tree,
                    animator,
                    now,
                );
            }
        };

        // Phase 3a: Focus traversal (Tab / Shift+Tab) is bound globally in
        // `Keymap::defaults()` (no `key_context()` scope), so `keymap.lookup`
        // matches it on EVERY overlay regardless of context. Overlays do
        // not currently have a per-overlay focus model registered with
 // `WindowRoot::InteractionManager` (per §02 design
        // constraint), so dispatching `widget::FocusNext`/`widget::FocusPrev`
        // via `dispatch_keymap_action` on the overlay's root would emit
        // no widget action AND no focus would move — silently swallowing
        // Tab. Fall through to the legacy `on_input` pipeline so widgets
        // that implement custom Tab handling (`DialogWidget::on_input` at
        // `widgets/dialog/mod.rs:303` for OkCancel buttons) continue to
        // receive Tab events. Future work: plumb a per-overlay focus
        // model through and dispatch FocusNext/FocusPrev natively here.
        if matches!(action.name(), "widget::FocusNext" | "widget::FocusPrev") {
            return self.process_key_event(
                event,
                measurer,
                theme,
                focused_widget,
                tree,
                animator,
                now,
            );
        }

        // Phase 3b: Dismiss is a manager-level concern (touches the overlay
        // stack). Translate to `Dismissed` directly so dialog-window
        // handlers (which match on `Dismissed`, not `DismissOverlay`)
        // dismiss correctly without widget cooperation. `DialogWidget`
        // does not implement `handle_keymap_action`, so without this
        // translation a `Dismiss` match would emit no widget action and
        // the popup would stay mounted on dialog windows.
        if action.name() == "widget::Dismiss" {
            self.begin_dismiss_topmost(tree, animator, now)
                .expect("checked non-empty above");
            return OverlayEventResult::Dismissed(id);
        }

        // Phase 3c: dispatch to the overlay root widget. Re-borrow
        // `self.overlays.last_mut()` now that the Phase 1 borrow is dead.
        let topmost = self.overlays.last_mut().expect("checked non-empty above");
        let widget_action =
            dispatch_keymap_action(topmost.widget.as_mut(), root_id, &*action, bounds);
        OverlayEventResult::Delivered {
            overlay_id: id,
            response: OverlayResponse {
                action: widget_action,
                handled: true,
            },
        }
    }
}
