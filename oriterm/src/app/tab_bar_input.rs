//! Tab bar mouse click dispatch.
//!
//! Routes mouse clicks in the tab bar zone to the appropriate action
//! based on the [`TabBarHit`](oriterm_ui::widgets::tab_bar::TabBarHit) at
//! the cursor position.

use std::time::{Duration, Instant};

use winit::event::ElementState;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};

#[cfg(not(target_os = "macos"))]
use oriterm_ui::geometry::Point;
use oriterm_ui::geometry::Rect;
#[cfg(not(target_os = "macos"))]
use oriterm_ui::input::MouseButton;
use oriterm_ui::overlay::Placement;
#[cfg(not(target_os = "macos"))]
use oriterm_ui::widgets::WidgetAction;
use oriterm_ui::widgets::menu::{MenuStyle, MenuWidget};
use oriterm_ui::widgets::tab_bar::TabBarHit;
use oriterm_ui::widgets::tab_bar::constants::DROPDOWN_BUTTON_WIDTH;

use super::{App, context_menu};

/// Time window for two clicks to count as a double-click.
const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(500);

impl App {
    /// Dispatch a mouse click in the tab bar zone.
    ///
    /// Returns `true` if the event was consumed (click landed on a tab bar
    /// element). Returns `false` if the click is outside the tab bar.
    pub(super) fn try_tab_bar_mouse(
        &mut self,
        button: winit::event::MouseButton,
        state: ElementState,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        // macOS native traffic lights handle window controls — event_loop
        // is only used for chrome actions on other platforms.
        let _ = event_loop;
        let pos = self.mouse.cursor_pos();
        if !self.cursor_in_tab_bar(pos) {
            return false;
        }

        // Right-click on a tab opens the tab context menu.
        if button == winit::event::MouseButton::Right && state == ElementState::Pressed {
            let hit = self
                .focused_ctx()
                .map_or(TabBarHit::None, |ctx| ctx.tab_bar.hover_hit());
            if let TabBarHit::Tab(idx) = hit {
                self.open_tab_context_menu(idx);
                return true;
            }
            // Right-click elsewhere in the tab bar is consumed without action.
            return true;
        }

        // Only handle left-button events.
        if button != winit::event::MouseButton::Left {
            return false;
        }

        // On release: route to control buttons for press/release cycle.
        // Window control actions fire on mouse-up (matching Windows caption
        // button behavior: press highlights, release fires, drag-off cancels).
        // Not on macOS — native traffic lights handle their own events.
        if state != ElementState::Pressed {
            #[cfg(not(target_os = "macos"))]
            if let Some(action) = self.route_control_mouse(MouseButton::Left, false) {
                self.handle_chrome_action(&action, event_loop);
            }
            return true;
        }

        // Use the hover hit already computed by update_tab_bar_hover.
        let hit = self
            .focused_ctx()
            .map_or(TabBarHit::None, |ctx| ctx.tab_bar.hover_hit());

        // If editing a tab and clicking elsewhere, commit the edit first.
        let editing_idx = self
            .focused_ctx()
            .and_then(|ctx| ctx.tab_bar.editing_tab_index());
        if let Some(eidx) = editing_idx {
            let click_on_editing_tab = matches!(hit, TabBarHit::Tab(i) if i == eidx);
            if !click_on_editing_tab {
                self.commit_tab_edit();
            }
        }

        match hit {
            TabBarHit::None => false,

            TabBarHit::Tab(idx) => {
                self.handle_tab_click(idx);
                true
            }

            TabBarHit::CloseTab(idx) => {
                // Acquire width lock for stable close-button targeting
                // during rapid close clicks.
                if let Some(ctx) = self.focused_ctx() {
                    let w = ctx.tab_bar.layout().base_tab_width();
                    self.acquire_tab_width_lock(w);
                }
                self.close_tab_at_index(idx);
                true
            }

            TabBarHit::NewTab => {
                if let Some(win_id) = self.active_window {
                    self.new_tab_in_window(win_id);
                }
                true
            }

            TabBarHit::Dropdown => {
                self.open_dropdown_menu();
                true
            }

            TabBarHit::Minimize | TabBarHit::Maximize | TabBarHit::CloseWindow => {
                // Route press to control button widget — sets pressed state
                // but does not fire the action until mouse-up.
                // On macOS: native traffic lights handle their own events,
                // but these hit types won't occur since controls are removed
                // from interactive_rects.
                #[cfg(not(target_os = "macos"))]
                self.route_control_mouse(MouseButton::Left, true);
                true
            }

            TabBarHit::DragArea => {
                self.handle_tab_bar_drag_area();
                true
            }
        }
    }

    /// Open the tab right-click context menu below the clicked tab.
    fn open_tab_context_menu(&mut self, tab_index: usize) {
        let tab_id = self.active_window.and_then(|wid| {
            let win = self.session.get_window(wid)?;
            win.tabs().get(tab_index).copied()
        });
        let Some(tab_id) = tab_id else { return };
        let (entries, state) = context_menu::build_tab_context_menu(tab_index, tab_id);
        let style = MenuStyle::from_theme(&self.ui_theme);
        let widget = MenuWidget::new(entries).with_style(style);

        // Anchor to the right-clicked tab rect.
        let anchor = self
            .focused_ctx()
            .map(|ctx| {
                let m = ctx.tab_bar.metrics();
                let layout = ctx.tab_bar.layout();
                let tx = layout.tab_x(tab_index);
                Rect::new(
                    tx,
                    m.height - m.top_margin,
                    layout.base_tab_width(),
                    m.top_margin,
                )
            })
            .unwrap_or_default();
        let now = Instant::now();

        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.context_menu = Some(state);
            ctx.root
                .replace_popup(Box::new(widget), anchor, Placement::Below, now);
            ctx.root.mark_dirty();
            ctx.root.set_urgent_redraw(true);
        }
    }

    /// Open the dropdown menu below the dropdown button.
    fn open_dropdown_menu(&mut self) {
        let (entries, state) = context_menu::build_dropdown_menu();
        let style = MenuStyle::from_theme(&self.ui_theme);
        let widget = MenuWidget::new(entries).with_style(style);

        // Anchor to the dropdown button rect.
        let anchor = self
            .focused_ctx()
            .map(|ctx| {
                let m = ctx.tab_bar.metrics();
                let dx = ctx.tab_bar.layout().dropdown_x();
                Rect::new(
                    dx,
                    m.height - m.top_margin,
                    DROPDOWN_BUTTON_WIDTH,
                    m.top_margin,
                )
            })
            .unwrap_or_default();
        let now = Instant::now();

        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.context_menu = Some(state);
            ctx.root
                .replace_popup(Box::new(widget), anchor, Placement::Below, now);
            ctx.root.mark_dirty();
            ctx.root.set_urgent_redraw(true);
        }
    }

    /// Route a mouse event to the tab bar's window control buttons.
    ///
    /// Delegates to [`TabBarWidget::dispatch_control_input`] which dispatches
    /// through the controller pipeline on [`WindowControlButton`]s. Returns
    /// the emitted [`WidgetAction`] (if any) — the caller dispatches it.
    #[cfg(not(target_os = "macos"))]
    fn route_control_mouse(&mut self, button: MouseButton, is_down: bool) -> Option<WidgetAction> {
        let pos = self.mouse.cursor_pos();
        let ctx = self.focused_ctx_mut()?;
        let scale = ctx.window.scale_factor().factor() as f32;
        let logical_pos = Point::new(pos.x as f32 / scale, pos.y as f32 / scale);
        let now = Instant::now();
        let event = if is_down {
            oriterm_ui::input::InputEvent::MouseDown {
                pos: logical_pos,
                button,
                modifiers: oriterm_ui::input::Modifiers::NONE,
            }
        } else {
            oriterm_ui::input::InputEvent::MouseUp {
                pos: logical_pos,
                button,
                modifiers: oriterm_ui::input::Modifiers::NONE,
            }
        };
        let result = ctx.tab_bar.dispatch_control_input(&event, now);
        if result.handled
            || result
                .requests
                .contains(oriterm_ui::controllers::ControllerRequests::PAINT)
        {
            ctx.root.mark_dirty();
            ctx.ui_stale = true;
        }
        // Map Clicked(btn_id) → WindowMinimize/Maximize/Close.
        result.actions.into_iter().find_map(|a| {
            if let WidgetAction::Clicked(id) = a {
                ctx.tab_bar.action_for_control(id)
            } else {
                Some(a)
            }
        })
    }

    /// Handle a left-click on a tab body.
    ///
    /// Double-click starts inline title editing; single click switches tab
    /// and initiates drag. If editing a different tab, commits that edit first.
    fn handle_tab_click(&mut self, idx: usize) {
        let now = Instant::now();

        // Check for double-click on the same tab.
        let is_double = self
            .focused_ctx()
            .and_then(|ctx| ctx.last_tab_press)
            .is_some_and(|(prev_idx, t)| {
                prev_idx == idx && now.duration_since(t) < DOUBLE_CLICK_THRESHOLD
            });

        // Update timestamp.
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.last_tab_press = Some((idx, now));
        }

        if is_double {
            // Double-click: start inline editing. Reset timestamp to prevent
            // a third click from re-triggering.
            if let Some(ctx) = self.focused_ctx_mut() {
                ctx.last_tab_press = None;
                ctx.tab_bar.start_editing(idx);
                ctx.root.mark_dirty();
                ctx.ui_stale = true;
            }
        } else {
            // Single click: switch tab and start drag.
            self.switch_to_tab_index(idx);
            self.try_start_tab_drag(idx);
        }
    }

    /// Commit an active tab title edit.
    ///
    /// Sets the title override on the session `Tab` so the user-set title
    /// persists across OSC title changes. Also marks dirty for repaint.
    pub(super) fn commit_tab_edit(&mut self) {
        let committed = self
            .focused_ctx_mut()
            .and_then(|ctx| ctx.tab_bar.commit_editing());
        if let Some((index, title)) = committed {
            // Persist the user-set title on the session Tab.
            if let Some(wid) = self.active_window {
                if let Some(win) = self.session.get_window(wid) {
                    if let Some(&tab_id) = win.tabs().get(index) {
                        if let Some(tab) = self.session.get_tab_mut(tab_id) {
                            tab.set_title_override(Some(title));
                        }
                    }
                }
            }
            if let Some(ctx) = self.focused_ctx_mut() {
                ctx.root.mark_dirty();
                ctx.ui_stale = true;
            }
        }
    }

    /// Cancel an active tab title edit.
    pub(super) fn cancel_tab_edit(&mut self) {
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.tab_bar.cancel_editing();
            ctx.root.mark_dirty();
            ctx.ui_stale = true;
        }
    }

    /// Handle keyboard input during tab title inline editing.
    ///
    /// Returns `true` if the event was consumed (editing is active and
    /// the key was handled). Called before overlay/search/PTY dispatch.
    pub(super) fn handle_tab_editing_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        let is_editing = self
            .focused_ctx()
            .is_some_and(|ctx| ctx.tab_bar.is_editing());
        if !is_editing || event.state != ElementState::Pressed {
            return false;
        }

        let shift = self.modifiers.shift_key();
        let ctrl = self.modifiers.control_key();

        match &event.logical_key {
            Key::Named(NamedKey::Enter | NamedKey::Tab) => {
                self.commit_tab_edit();
                true
            }
            Key::Named(NamedKey::Escape) => {
                self.cancel_tab_edit();
                true
            }
            Key::Named(NamedKey::Backspace) => {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.editing_backspace();
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
                true
            }
            Key::Named(NamedKey::Delete) => {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.editing_delete();
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
                true
            }
            Key::Named(NamedKey::ArrowLeft) => {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.editing_move_left(shift);
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
                true
            }
            Key::Named(NamedKey::ArrowRight) => {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.editing_move_right(shift);
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
                true
            }
            Key::Named(NamedKey::Home) => {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.editing_home(shift);
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
                true
            }
            Key::Named(NamedKey::End) => {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.editing_end(shift);
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
                true
            }
            Key::Character(ch) if ctrl && ch.as_str() == "a" => {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.editing_select_all();
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
                true
            }
            Key::Character(ch) => {
                for c in ch.chars() {
                    if !c.is_control() {
                        if let Some(ctx) = self.focused_ctx_mut() {
                            ctx.tab_bar.editing_insert_char(c);
                            ctx.root.mark_dirty();
                            ctx.ui_stale = true;
                        }
                    }
                }
                true
            }
            _ => true, // Consume all other keys during editing.
        }
    }

    /// Handle a click in the tab bar drag area.
    ///
    /// Double-click toggles maximize; single click initiates window drag.
    fn handle_tab_bar_drag_area(&mut self) {
        let now = Instant::now();
        let is_double = self
            .focused_ctx()
            .and_then(|ctx| ctx.last_drag_area_press)
            .is_some_and(|t| now.duration_since(t) < DOUBLE_CLICK_THRESHOLD);
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.last_drag_area_press = Some(now);
        }

        if is_double {
            // Double-click: toggle maximize. Reset timestamp to prevent
            // a third click from triggering another toggle.
            if let Some(ctx) = self.focused_ctx_mut() {
                ctx.last_drag_area_press = None;
            }
            self.toggle_maximize();
        } else {
            // Single click: initiate native window drag.
            if let Some(ctx) = self.focused_ctx() {
                let _ = ctx.window.window().drag_window();
            }
        }
    }
}
