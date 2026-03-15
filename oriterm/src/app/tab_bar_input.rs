//! Tab bar mouse click dispatch.
//!
//! Routes mouse clicks in the tab bar zone to the appropriate action
//! based on the [`TabBarHit`](oriterm_ui::widgets::tab_bar::TabBarHit) at
//! the cursor position.

use std::time::{Duration, Instant};

use winit::event::ElementState;
use winit::event_loop::ActiveEventLoop;

#[cfg(not(target_os = "macos"))]
use oriterm_ui::geometry::Point;
use oriterm_ui::geometry::Rect;
#[cfg(not(target_os = "macos"))]
use oriterm_ui::input::{MouseButton, MouseEvent, MouseEventKind};
use oriterm_ui::overlay::Placement;
use oriterm_ui::widgets::menu::{MenuStyle, MenuWidget};
use oriterm_ui::widgets::tab_bar::TabBarHit;
use oriterm_ui::widgets::tab_bar::constants::{
    DROPDOWN_BUTTON_WIDTH, TAB_BAR_HEIGHT, TAB_TOP_MARGIN,
};
#[cfg(not(target_os = "macos"))]
use oriterm_ui::widgets::{EventCtx, WidgetAction};

#[cfg(not(target_os = "macos"))]
use crate::font::{CachedTextMeasurer, UiFontMeasurer};

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

        match hit {
            TabBarHit::None => false,

            TabBarHit::Tab(idx) => {
                self.switch_to_tab_index(idx);
                self.try_start_tab_drag(idx);
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
                let layout = ctx.tab_bar.layout();
                let tx = layout.tab_x(tab_index);
                Rect::new(
                    tx,
                    TAB_BAR_HEIGHT - TAB_TOP_MARGIN,
                    layout.base_tab_width(),
                    TAB_TOP_MARGIN,
                )
            })
            .unwrap_or_default();
        let now = Instant::now();

        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.context_menu = Some(state);
            ctx.overlays.replace_popup(
                Box::new(widget),
                anchor,
                Placement::Below,
                &mut ctx.layer_tree,
                &mut ctx.layer_animator,
                now,
            );
            ctx.dirty = true;
            ctx.urgent_redraw = true;
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
                let dx = ctx.tab_bar.layout().dropdown_x();
                Rect::new(
                    dx,
                    TAB_BAR_HEIGHT - TAB_TOP_MARGIN,
                    DROPDOWN_BUTTON_WIDTH,
                    TAB_TOP_MARGIN,
                )
            })
            .unwrap_or_default();
        let now = Instant::now();

        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.context_menu = Some(state);
            ctx.overlays.replace_popup(
                Box::new(widget),
                anchor,
                Placement::Below,
                &mut ctx.layer_tree,
                &mut ctx.layer_animator,
                now,
            );
            ctx.dirty = true;
            ctx.urgent_redraw = true;
        }
    }

    /// Route a mouse event to the tab bar's window control buttons.
    ///
    /// Delegates to [`TabBarWidget::handle_control_mouse`] which manages
    /// the press/release cycle on [`WindowControlButton`]s. Returns the
    /// emitted [`WidgetAction`] (if any) — the caller dispatches it.
    #[cfg(not(target_os = "macos"))]
    fn route_control_mouse(&mut self, button: MouseButton, is_down: bool) -> Option<WidgetAction> {
        let pos = self.mouse.cursor_pos();
        let ui_theme = self.ui_theme;
        let ctx = self.focused_ctx_mut()?;
        let scale = ctx.window.scale_factor().factor() as f32;
        let logical_pos = Point::new(pos.x as f32 / scale, pos.y as f32 / scale);
        let kind = if is_down {
            MouseEventKind::Down(button)
        } else {
            MouseEventKind::Up(button)
        };
        let mouse_event = MouseEvent {
            kind,
            pos: logical_pos,
            modifiers: oriterm_ui::input::Modifiers::NONE,
        };
        let renderer = ctx.renderer.as_ref()?;
        let measurer = CachedTextMeasurer::new(
            UiFontMeasurer::new(renderer.active_ui_collection(), scale),
            &ctx.text_cache,
            scale,
        );
        let event_ctx = EventCtx {
            measurer: &measurer,
            bounds: Rect::default(),
            is_focused: false,
            focused_widget: None,
            theme: &ui_theme,
            interaction: None,
            widget_id: None,
        };
        let resp = ctx.tab_bar.handle_control_mouse(&mouse_event, &event_ctx);
        if matches!(
            resp.response,
            oriterm_ui::input::EventResponse::RequestPaint
                | oriterm_ui::input::EventResponse::RequestLayout
        ) {
            ctx.dirty = true;
            ctx.ui_stale = true;
        }
        resp.action
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
