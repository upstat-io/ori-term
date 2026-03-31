//! Sidebar input handling — pointer tracking, click routing, keyboard nav, search.

use winit::window::CursorIcon;

use crate::action::WidgetAction;
use crate::geometry::{Point, Rect};
use crate::input::{InputEvent, Key};
use crate::interaction::LifecycleEvent;
use crate::widgets::{LifecycleCtx, OnInputResult};

use super::geometry::{self, SIDEBAR_PADDING_Y};
use super::{FooterTarget, HoveredFooterTarget, SidebarNavWidget};

/// Left padding inside the search field.
const SEARCH_TEXT_INSET: f32 = 8.0;

impl SidebarNavWidget {
    /// Handles all input events for the sidebar.
    pub(super) fn handle_input(&mut self, event: &InputEvent, bounds: Rect) -> OnInputResult {
        match event {
            // Track which item / footer target is hovered for instant highlight.
            InputEvent::MouseMove { pos, .. } => {
                let local_y = pos.y - bounds.y() - SIDEBAR_PADDING_Y;
                self.hovered_item = self.hit_test_item(local_y);
                self.hovered_footer = self.hit_test_footer(*pos);

                // Pointer cursor on interactive items, default elsewhere.
                let cursor = if self.hovered_item.is_some() || self.hovered_footer.is_some() {
                    CursorIcon::Pointer
                } else {
                    CursorIcon::Default
                };
                self.cursor_icon.set(cursor);

                OnInputResult::handled()
            }
            InputEvent::MouseDown { pos, .. } => self.handle_mouse_down(*pos, bounds),
            InputEvent::KeyDown { key, modifiers } => {
                if self.search_focused {
                    self.handle_search_key(*key, *modifiers)
                } else {
                    self.handle_nav_key(*key)
                }
            }
            _ => OnInputResult::ignored(),
        }
    }

    /// Routes mouse clicks to search field or nav items.
    fn handle_mouse_down(&mut self, pos: Point, bounds: Rect) -> OnInputResult {
        let search_rect = geometry::search_field_rect(bounds);

        if search_rect.contains(pos) {
            self.search_focused = true;
            // Position cursor at click X offset within the text.
            self.position_cursor_at_x(pos.x - search_rect.x() - SEARCH_TEXT_INSET);
            return OnInputResult::handled().with_focus_request();
        }

        // Click outside search → unfocus search.
        if self.search_focused {
            self.search_focused = false;
        }

        // Hit-test footer targets.
        if let Some(target) = self.hit_test_footer(pos) {
            let action = match target {
                HoveredFooterTarget::UpdateLink => {
                    FooterTarget::UpdateLink(self.update_url.clone())
                }
                HoveredFooterTarget::ConfigPath => FooterTarget::ConfigPath,
            };
            return OnInputResult::handled().with_action(WidgetAction::FooterAction(action));
        }

        // Hit-test nav items — request focus so arrow keys route here.
        let local_y = pos.y - bounds.y() - SIDEBAR_PADDING_Y;
        if let Some(flat_idx) = self.hit_test_item(local_y) {
            if let Some(page_idx) = self.page_for_flat_index(flat_idx) {
                return OnInputResult::handled().with_focus_request().with_action(
                    WidgetAction::Selected {
                        id: self.id,
                        index: page_idx,
                    },
                );
            }
        }
        OnInputResult::handled().with_focus_request()
    }

    /// Handles keyboard input when the search field is focused.
    fn handle_search_key(&mut self, key: Key, modifiers: crate::input::Modifiers) -> OnInputResult {
        let shift = modifiers.shift();
        match key {
            Key::Escape => {
                self.search_focused = false;
                OnInputResult::handled()
            }
            Key::Character(ch) if modifiers.ctrl() && (ch == 'a' || ch == 'A') => {
                self.search_state.select_all();
                OnInputResult::handled()
            }
            Key::Character(ch) if !modifiers.ctrl() && !modifiers.alt() => {
                self.search_state.insert_char(ch);
                OnInputResult::handled()
            }
            Key::Backspace => {
                self.search_state.backspace();
                OnInputResult::handled()
            }
            Key::Delete => {
                self.search_state.delete();
                OnInputResult::handled()
            }
            Key::ArrowLeft => {
                self.search_state.move_left(shift);
                OnInputResult::handled()
            }
            Key::ArrowRight => {
                self.search_state.move_right(shift);
                OnInputResult::handled()
            }
            Key::Home => {
                self.search_state.home(shift);
                OnInputResult::handled()
            }
            Key::End => {
                self.search_state.end(shift);
                OnInputResult::handled()
            }
            _ => OnInputResult::ignored(),
        }
    }

    /// Handles arrow/Home/End nav when search is NOT focused.
    ///
    /// When a search query is active, navigates among visible (filtered)
    /// items only. Without a query, uses the full unfiltered page range.
    fn handle_nav_key(&self, key: Key) -> OnInputResult {
        if self.search_query().is_some() {
            return self.handle_filtered_nav_key(key);
        }
        // Fast path: no search query — navigate full page range.
        let total = self.total_item_count();
        if total == 0 {
            return OnInputResult::ignored();
        }
        let new_page = match key {
            Key::ArrowUp if self.active_page > 0 => Some(self.active_page - 1),
            Key::ArrowDown if self.active_page + 1 < total => Some(self.active_page + 1),
            Key::Home => Some(0),
            Key::End => Some(total - 1),
            _ => None,
        };
        if let Some(page_idx) = new_page {
            OnInputResult::handled().with_action(WidgetAction::Selected {
                id: self.id,
                index: page_idx,
            })
        } else {
            OnInputResult::ignored()
        }
    }

    /// Navigates among visible items when a search query is active.
    fn handle_filtered_nav_key(&self, key: Key) -> OnInputResult {
        let visible: Vec<usize> = self.visible_items().map(|item| item.page_index).collect();
        if visible.is_empty() {
            return OnInputResult::ignored();
        }
        // Find the current position in the visible list.
        let cur_idx = visible
            .iter()
            .position(|&p| p == self.active_page)
            .unwrap_or(0);
        let new_page = match key {
            Key::ArrowUp if cur_idx > 0 => Some(visible[cur_idx - 1]),
            Key::ArrowDown if cur_idx + 1 < visible.len() => Some(visible[cur_idx + 1]),
            Key::Home => Some(visible[0]),
            Key::End => Some(*visible.last().expect("checked non-empty")),
            _ => None,
        };
        if let Some(page_idx) = new_page {
            OnInputResult::handled().with_action(WidgetAction::Selected {
                id: self.id,
                index: page_idx,
            })
        } else {
            OnInputResult::ignored()
        }
    }

    /// Hit-tests a point against cached footer rects.
    ///
    /// Returns the hovered footer target, or `None` if the point is outside
    /// all footer interactive regions.
    fn hit_test_footer(&self, pos: Point) -> Option<HoveredFooterTarget> {
        let rects = self.footer_rects.get();
        if let Some(r) = rects.update_link {
            if r.contains(pos) {
                return Some(HoveredFooterTarget::UpdateLink);
            }
        }
        if let Some(r) = rects.config_path {
            if r.contains(pos) {
                return Some(HoveredFooterTarget::ConfigPath);
            }
        }
        None
    }

    /// Positions the cursor at the nearest character boundary to `click_x`.
    ///
    /// Uses character X-offsets cached during paint for accurate positioning.
    /// Falls back to cursor position 0 if offsets are empty (e.g. first
    /// click before any paint).
    fn position_cursor_at_x(&mut self, click_x: f32) {
        let offsets = self.search_char_offsets.borrow();
        if offsets.is_empty() {
            self.search_state.set_cursor(0);
            return;
        }
        let mut best_pos = 0;
        let mut best_dist = f32::MAX;
        for &(byte_pos, x_offset) in offsets.iter() {
            let dist = (x_offset - click_x).abs();
            if dist < best_dist {
                best_dist = dist;
                best_pos = byte_pos;
            }
        }
        self.search_state.set_cursor(best_pos);
    }

    /// Handles lifecycle events (hot/focus state changes).
    pub(super) fn handle_lifecycle(&mut self, event: &LifecycleEvent, _ctx: &mut LifecycleCtx<'_>) {
        match event {
            // When the cursor leaves the sidebar entirely, clear all hover.
            LifecycleEvent::HotChanged { is_hot: false, .. } => {
                self.hovered_item = None;
                self.hovered_footer = None;
            }
            // When framework focus is lost, clear search focus to stay in sync.
            LifecycleEvent::FocusChanged {
                is_focused: false, ..
            } => {
                self.search_focused = false;
            }
            _ => {}
        }
    }
}
