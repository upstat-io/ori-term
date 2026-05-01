//! Widget trait implementation for `MenuWidget`.
//!
//! Handles layout, drawing (with scroll clipping and scrollbar), and event
//! dispatch (mouse, keyboard, hover). Scroll support activates when
//! `MenuStyle::max_height` is set and content exceeds the limit.
//!
//! Press/drag input flows through `ScrubController` → `on_action()` with
//! zone discrimination (scrollbar thumb, scrollbar track, or menu item).
//! Idle hover and scroll wheel remain in `on_input()`.

use crate::geometry::{Point, Rect};
use crate::input::{InputEvent, Key, ScrollDelta};
use crate::interaction::LifecycleEvent;
use crate::layout::LayoutBox;
use crate::sense::Sense;

use super::super::scrollbar::{
    compute_rects, drag_delta_to_offset, pointer_to_offset, should_show,
};
use super::super::{DrawCtx, LayoutCtx, LifecycleCtx, OnInputResult, Widget, WidgetAction};
use super::MenuWidget;
use super::style::{DragMode, SCROLL_LINE_HEIGHT};

impl Widget for MenuWidget {
    fn id(&self) -> crate::widget_id::WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let style = self.text_style();
        let left_margin = self.label_left_margin();

        // Width is computed against the canonical entry list — searchable
        // mode hides separators at runtime but the popup must still fit any
        // entry the user could type-filter to.
        let max_label_w: f32 = self
            .entries
            .iter()
            .filter_map(|e| e.label())
            .map(|label| ctx.measurer.measure(label, &style, f32::INFINITY).width)
            .fold(0.0_f32, f32::max);

        let width = (left_margin + max_label_w + self.style.extra_width).max(self.style.min_width);
        let height = self.visible_height();

        LayoutBox::leaf(width, height)
            .with_widget_id(self.id)
            .with_cursor_icon(winit::window::CursorIcon::Pointer)
    }

    fn sense(&self) -> Sense {
        Sense::click_and_drag()
    }

    fn controllers(&self) -> &[Box<dyn crate::controllers::EventController>] {
        &self.controllers
    }

    fn controllers_mut(&mut self) -> &mut [Box<dyn crate::controllers::EventController>] {
        &mut self.controllers
    }

    fn lifecycle(&mut self, event: &LifecycleEvent, _ctx: &mut LifecycleCtx<'_>) {
        if let LifecycleEvent::HotChanged { is_hot: false, .. } = event {
            self.scrollbar_state.track_hovered = false;
            self.scrollbar_state.thumb_hovered = false;
        }
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let bounds = ctx.bounds;
        let s = &self.style;
        let scrollable = self.is_scrollable();

        self.draw_chrome(ctx, bounds);

        // The query row sits above the scrolling region so it stays pinned
        // while entries scroll. Draw it before pushing the clip rect.
        if self.searchable {
            self.draw_query_row(ctx, bounds);
        }

        // Clip the entries region when scrolling. The query row is excluded
        // from the clip so its rendering remains crisp.
        if scrollable {
            let inset = s.border_width;
            let qh = self.query_row_height();
            let clip = Rect::new(
                bounds.x() + inset,
                bounds.y() + inset + qh,
                bounds.width() - inset * 2.0,
                bounds.height() - inset * 2.0 - qh,
            );
            ctx.scene.push_clip(clip);
        }

        self.draw_entries(ctx, bounds);

        if scrollable {
            ctx.scene.pop_clip();
            self.draw_scrollbar(ctx, bounds);
        }

        ctx.scene.pop_layer_bg();
    }

    fn on_action(&mut self, action: WidgetAction, bounds: Rect) -> Option<WidgetAction> {
        match action {
            WidgetAction::DragStart { pos, .. } => {
                self.drag_origin = Some(pos);
                self.handle_drag_start(pos, bounds);
                None
            }
            WidgetAction::DragUpdate { total_delta, .. } => {
                self.handle_drag_update(total_delta, bounds);
                None
            }
            WidgetAction::DragEnd { .. } => {
                let result = self.handle_drag_end();
                self.drag_origin = None;
                self.drag_mode = None;
                result
            }
            other => Some(other),
        }
    }

    fn on_input(&mut self, event: &InputEvent, bounds: Rect) -> OnInputResult {
        match event {
            // Idle hover — ScrubController does not consume MouseMove when idle.
            InputEvent::MouseMove { pos, .. } => {
                if self.is_scrollable() {
                    self.update_scrollbar_hover(*pos, bounds);
                }
                // Clear entry hover when cursor is on the scrollbar.
                if self.scrollbar_state.track_hovered {
                    self.hovered = None;
                } else {
                    let rel_y = pos.y - bounds.y();
                    let new_hover = self.entry_at_y(rel_y);
                    if new_hover != self.hovered {
                        self.hovered = new_hover;
                    }
                }
                OnInputResult::handled()
            }
            InputEvent::Scroll { delta, pos, .. } => {
                let delta_y = match *delta {
                    ScrollDelta::Pixels { y, .. } => -y,
                    ScrollDelta::Lines { y, .. } => -y * SCROLL_LINE_HEIGHT,
                };
                if self.scroll_by(delta_y) {
                    // Only update hover if the cursor is on a menu item, not the
                    // scrollbar — same guard as the MouseMove path above.
                    if !self.scrollbar_state.track_hovered {
                        let rel_y = pos.y - bounds.y();
                        self.hovered = self.entry_at_y(rel_y);
                    }
                }
                OnInputResult::handled()
            }
            // Filter input — only consumed in searchable mode.
            //
            // Character / Space append to the filter query; Backspace pops
            // the last character. Navigation and confirmation (ArrowDown,
            // ArrowUp, Enter, Escape) route through the keymap path —
            // see `key_context()` returning `"MenuSearchable"`, which the
            // default keymap binds without `Space` so printable filter
            // input reaches this branch. Per BUG-03-003 (resolved): the
            // overlay keymap dispatch path now invokes
            // `MenuWidget::handle_keymap_action` for nav/confirm keys,
            // so this `on_input` arm only handles the printable filter
            // characters that the keymap intentionally omits.
            InputEvent::KeyDown { key, .. } if self.searchable => match *key {
                Key::Character(c) => {
                    self.handle_filter_character(c);
                    OnInputResult::handled()
                }
                Key::Space => {
                    self.handle_filter_character(' ');
                    OnInputResult::handled()
                }
                Key::Backspace => {
                    self.handle_filter_backspace();
                    OnInputResult::handled()
                }
                _ => OnInputResult::ignored(),
            },
            _ => OnInputResult::ignored(),
        }
    }

    fn key_context(&self) -> Option<&'static str> {
        // BUG-03-003: searchable Menu uses a distinct context so the
        // keymap omits Space (which would otherwise steal printable
        // filter input via Space->Confirm). Non-searchable Menu keeps
        // Space->Confirm via the "Menu" context.
        Some(if self.searchable {
            "MenuSearchable"
        } else {
            "Menu"
        })
    }

    fn handle_keymap_action(
        &mut self,
        action: &dyn crate::action::KeymapAction,
        _bounds: Rect,
    ) -> Option<WidgetAction> {
        match action.name() {
            "widget::NavigateDown" => {
                self.navigate_keyboard(true);
                None
            }
            "widget::NavigateUp" => {
                self.navigate_keyboard(false);
                None
            }
            "widget::Confirm" => self.try_select_hovered(),
            "widget::Dismiss" => Some(WidgetAction::DismissOverlay(self.id)),
            _ => None,
        }
    }
}

// Selection helper shared across keymap dispatch (`handle_keymap_action`)
// and the searchable-mode `on_input` Enter branch.
impl MenuWidget {
    /// Returns a `Selected` action for the currently hovered entry, or `None`
    /// when no entry is hovered or the hovered entry is non-clickable.
    pub(super) fn try_select_hovered(&self) -> Option<WidgetAction> {
        let idx = self.hovered?;
        if self.entries[idx].is_clickable() {
            Some(WidgetAction::Selected { id: self.id, index: idx })
        } else {
            None
        }
    }
}

// Drag action handlers.
impl MenuWidget {
    /// Determines the press zone and starts the appropriate interaction.
    fn handle_drag_start(&mut self, pos: Point, bounds: Rect) {
        if self.is_scrollable() {
            let (m, inner) = self.scrollbar_context(bounds);
            if should_show(&m) {
                let rects = compute_rects(inner, &m, &self.style.scrollbar, 0.0);
                if rects.thumb_hit.contains(pos) {
                    self.scrollbar_state.dragging = true;
                    self.scrollbar_state.drag_start_offset = self.scroll_offset;
                    self.drag_mode = Some(DragMode::ScrollbarThumb);
                    return;
                }
                if rects.track_hit.contains(pos) {
                    self.scroll_offset = pointer_to_offset(pos.y, &rects, &m);
                    self.drag_mode = Some(DragMode::ScrollbarTrack);
                    return;
                }
            }
        }
        // Update hover from the press position so a click without prior
        // MouseMove (e.g. menu opens under a stationary cursor) selects
        // the correct item instead of silently no-oping.
        let rel_y = pos.y - bounds.y();
        self.hovered = self.entry_at_y(rel_y);
        self.drag_mode = Some(DragMode::ItemPress);
    }

    /// Updates state during an active drag.
    fn handle_drag_update(&mut self, total_delta: Point, bounds: Rect) {
        match self.drag_mode {
            Some(DragMode::ScrollbarThumb) => {
                let (m, inner) = self.scrollbar_context(bounds);
                let rects = compute_rects(inner, &m, &self.style.scrollbar, 0.0);
                let offset_delta = drag_delta_to_offset(total_delta.y, &rects, &m);
                let max = self.max_scroll();
                self.scroll_offset =
                    (self.scrollbar_state.drag_start_offset + offset_delta).clamp(0.0, max);
                self.hovered = None;
            }
            Some(DragMode::ItemPress) => {
                // Update item hover based on current absolute position.
                if let Some(origin) = self.drag_origin {
                    let cur_y = origin.y + total_delta.y;
                    self.hovered = self.entry_at_y(cur_y - bounds.y());
                }
            }
            Some(DragMode::ScrollbarTrack) | None => {}
        }
    }

    /// Finalizes the drag and optionally emits a Selected action.
    fn handle_drag_end(&mut self) -> Option<WidgetAction> {
        match self.drag_mode {
            Some(DragMode::ScrollbarThumb) => {
                self.scrollbar_state.dragging = false;
                None
            }
            Some(DragMode::ItemPress) => {
                if let Some(idx) = self.hovered {
                    if self.entries[idx].is_clickable() {
                        return Some(WidgetAction::Selected {
                            id: self.id,
                            index: idx,
                        });
                    }
                }
                None
            }
            Some(DragMode::ScrollbarTrack) | None => None,
        }
    }
}

// Drawing helpers (`draw_chrome` / `draw_entries` / `draw_query_row` /
// `draw_no_matches_row` / `draw_item` / `draw_separator` / `draw_scrollbar`)
// + scrollbar geometry helpers (`scrollbar_context`, `update_scrollbar_hover`)
// live in the sibling `paint` module so this file stays under the 500-line
// budget per `code-hygiene.md §File Size`.
