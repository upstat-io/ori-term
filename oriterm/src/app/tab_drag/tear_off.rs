//! Tab tear-off: detach a tab into a new window and start an OS-level drag.
//!
//! When the cursor exceeds the tear-off threshold during an in-bar drag, this
//! module creates a new window for the tab and initiates a `drag_window()` OS
//! drag session. Platform-specific code handles positioning and drag config.

use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::session::TabId;
use crate::window_manager::types::{ManagedWindow, WindowKind};
use oriterm_ui::widgets::tab_bar::constants::TAB_LEFT_MARGIN;

#[cfg(target_os = "windows")]
use oriterm_ui::platform_windows::{self, OsDragConfig};
#[cfg(target_os = "windows")]
use oriterm_ui::widgets::tab_bar::constants::{CONTROLS_ZONE_WIDTH, TAB_BAR_HEIGHT};

#[cfg(target_os = "windows")]
use super::TornOffPending;
use crate::app::App;

impl App {
    /// Tear off the currently dragged tab into a new window.
    ///
    /// Chrome-style in-process tear-off: creates a bare window, moves the tab
    /// via mux, pre-renders both windows, positions under the cursor, then
    /// enters the OS modal drag loop. Works for both embedded and daemon mode
    /// since `create_window_bare` handles daemon RPC transparently.
    pub(super) fn tear_off_tab(&mut self, event_loop: &ActiveEventLoop) {
        // Extract drag state from the source window.
        let (tab_id, mouse_offset, origin_y, source_winit_id) = {
            let Some(ctx) = self.focused_ctx_mut() else {
                return;
            };
            let Some(drag) = ctx.tab_drag.take() else {
                return;
            };
            // Clear drag visual on source.
            ctx.tab_bar.set_drag_visual(None);
            ctx.dirty = true;
            let wid = ctx.window.window_id();
            (drag.tab_id, drag.mouse_offset_in_tab, drag.origin_y, wid)
        };

        // Release width lock on source window.
        self.release_tab_width_lock();

        // Refuse to tear off the last tab in the session.
        let is_last = self.session.tab_count() <= 1;
        if is_last {
            log::warn!("tear_off_tab: refused — last tab in session");
            return;
        }

        // Create bare window (hidden, no tabs).
        let Some((new_winit_id, new_session_wid)) = self.create_window_bare(event_loop) else {
            return;
        };

        // Register tear-off with window manager (parent is the source window).
        self.window_manager.register(ManagedWindow::with_parent(
            new_winit_id,
            WindowKind::TearOff,
            source_winit_id,
        ));

        // Move tab from source window to new window (local session).
        {
            let src_wid = self.session.window_for_tab(tab_id);
            if let Some(wid) = src_wid {
                if let Some(win) = self.session.get_window_mut(wid) {
                    win.remove_tab(tab_id);
                }
            }
            if let Some(win) = self.session.get_window_mut(new_session_wid) {
                win.insert_tab_at(0, tab_id);
            }
        }

        // Drain mux notifications from the move.
        self.pump_mux_events();

        // Sync tab bars on both windows and refresh platform hit test rects.
        self.sync_tab_bar_for_window(source_winit_id);
        self.sync_tab_bar_for_window(new_winit_id);
        self.refresh_platform_rects(source_winit_id);
        self.refresh_platform_rects(new_winit_id);

        // Position the new window under the cursor.
        self.position_torn_off_window(new_winit_id, mouse_offset, origin_y);

        // Pre-render the new window with full content (tab bar + terminal).
        {
            let saved_focused = self.focused_window_id;
            let saved_active = self.active_window;
            self.focused_window_id = Some(new_winit_id);
            self.active_window = Some(new_session_wid);
            self.handle_redraw();
            self.focused_window_id = saved_focused;
            self.active_window = saved_active;
        }

        // Render the source window (tab bar now shows the torn tab removed).
        self.handle_redraw();

        // Show the new window.
        self.show_torn_off_window(new_winit_id);

        // If source window is now empty, remove it.
        let source_empty = self
            .windows
            .get(&source_winit_id)
            .and_then(|ctx| {
                let win = self.session.get_window(ctx.window.session_window_id())?;
                Some(win.tabs().is_empty())
            })
            .unwrap_or(false);
        if source_empty {
            self.remove_empty_window(source_winit_id);
        }

        // Start OS drag on the new window.
        self.begin_os_tab_drag(new_winit_id, tab_id, mouse_offset, origin_y);
    }

    // -- macOS implementation --

    /// Position the torn-off window under the cursor (macOS).
    #[cfg(target_os = "macos")]
    fn position_torn_off_window(
        &self,
        new_winit_id: WindowId,
        mouse_offset: f32,
        origin_y: f32,
    ) {
        let Some(ctx) = self.windows.get(&new_winit_id) else {
            return;
        };
        let scale = ctx.window.scale_factor().factor() as f32;
        let grab_x = (TAB_LEFT_MARGIN + mouse_offset) * scale;
        let grab_y = origin_y * scale;

        // Use winit's cursor_position for cross-platform cursor location.
        // On macOS, outer_position + cursor offset gives us the right spot.
        let cursor = self.mouse.cursor_pos();
        if let Some(src_ctx) = self.focused_ctx() {
            if let Ok(outer) = src_ctx.window.window().outer_position() {
                let pos_x = outer.x as f32 + cursor.x as f32 - grab_x;
                let pos_y = outer.y as f32 + cursor.y as f32 - grab_y;
                ctx.window
                    .window()
                    .set_outer_position(winit::dpi::PhysicalPosition::new(
                        pos_x as i32,
                        pos_y as i32,
                    ));
            }
        }
    }

    /// Show the torn-off window (macOS).
    #[cfg(target_os = "macos")]
    fn show_torn_off_window(&self, new_winit_id: WindowId) {
        if let Some(ctx) = self.windows.get(&new_winit_id) {
            ctx.window.set_visible(true);
        }
    }

    /// Start tracking a torn-off window under the cursor (macOS).
    ///
    /// Unlike Windows (which uses a blocking `drag_window()` modal loop),
    /// macOS can't use `performWindowDrag:` here — the newly-created window
    /// has no current event. Instead, we store `torn_off_pending` and let
    /// the event loop handle tracking: cursor move events on the source
    /// window (macOS delivers `leftMouseDragged:` to the mouseDown window)
    /// update the torn-off window's position, and mouse-up triggers merge
    /// detection.
    #[cfg(target_os = "macos")]
    fn begin_os_tab_drag(
        &mut self,
        winit_id: WindowId,
        tab_id: TabId,
        mouse_offset: f32,
        _origin_y: f32,
    ) {
        use crate::window_manager::platform::macos;
        self.torn_off_pending = Some(super::TornOffPending {
            winit_id,
            tab_id,
            mouse_offset,
            merge_enabled: false,
            tear_off_origin: macos::cursor_screen_pos(),
        });
    }

    /// Drag a single-tab window directly (macOS).
    ///
    /// Instead of native `drag_window()` (which bypasses merge detection),
    /// enter the torn-off tracking state so that continuous merge detection
    /// runs during the drag. This lets the user drag a single-tab window
    /// back onto another window's tab bar to merge.
    #[cfg(target_os = "macos")]
    pub(super) fn begin_single_tab_window_drag(&mut self) {
        use crate::window_manager::platform::macos;

        // Extract drag info before cleaning up.
        let (tab_id, mouse_offset, winit_id) = {
            let Some(ctx) = self.focused_ctx_mut() else {
                return;
            };
            let Some(drag) = ctx.tab_drag.take() else {
                return;
            };
            ctx.tab_bar.set_drag_visual(None);
            (drag.tab_id, drag.mouse_offset_in_tab, ctx.window.window_id())
        };
        self.release_tab_width_lock();

        // Enter torn-off tracking — the event loop will move the window
        // under the cursor and check for merge on every frame.
        self.torn_off_pending = Some(super::TornOffPending {
            winit_id,
            tab_id,
            mouse_offset,
            merge_enabled: false,
            tear_off_origin: macos::cursor_screen_pos(),
        });
    }

    // -- Windows implementation --

    /// Position the torn-off window under the cursor (Windows).
    #[cfg(target_os = "windows")]
    fn position_torn_off_window(
        &self,
        new_winit_id: WindowId,
        mouse_offset: f32,
        origin_y: f32,
    ) {
        let Some(ctx) = self.windows.get(&new_winit_id) else {
            return;
        };
        let scale = ctx.window.scale_factor().factor() as f32;
        let grab_x = ((TAB_LEFT_MARGIN + mouse_offset) * scale).round() as i32;
        let grab_y = (origin_y * scale).round() as i32;
        let cursor = platform_windows::cursor_screen_pos();
        let pos_x = cursor.0 - grab_x;
        let pos_y = cursor.1 - grab_y;
        ctx.window
            .window()
            .set_outer_position(winit::dpi::PhysicalPosition::new(pos_x, pos_y));
    }

    /// Show the torn-off window (Windows).
    ///
    /// Disables DWM transition animations for instant appearance.
    #[cfg(target_os = "windows")]
    fn show_torn_off_window(&self, new_winit_id: WindowId) {
        if let Some(ctx) = self.windows.get(&new_winit_id) {
            platform_windows::set_transitions_enabled(ctx.window.window(), false);
            ctx.window.set_visible(true);
            platform_windows::set_transitions_enabled(ctx.window.window(), true);
        }
    }

    /// Start an OS-level drag on the torn-off window (Windows).
    ///
    /// Collects merge rects, configures `WM_MOVING`, sets `torn_off_pending`,
    /// and calls `drag_window()` which blocks in the OS modal move loop.
    #[cfg(target_os = "windows")]
    fn begin_os_tab_drag(
        &mut self,
        winit_id: WindowId,
        tab_id: TabId,
        mouse_offset: f32,
        origin_y: f32,
    ) {
        let grab_offset = {
            let Some(ctx) = self.windows.get(&winit_id) else {
                return;
            };
            let scale = ctx.window.scale_factor().factor() as f32;
            let grab_x = ((TAB_LEFT_MARGIN + mouse_offset) * scale).round() as i32;
            let grab_y = (origin_y * scale).round() as i32;
            (grab_x, grab_y)
        };

        let merge_rects = self.collect_merge_rects(winit_id);

        if let Some(ctx) = self.windows.get(&winit_id) {
            platform_windows::begin_os_drag(
                ctx.window.window(),
                OsDragConfig {
                    grab_offset,
                    merge_rects,
                    skip_count: 3,
                },
            );
        }

        self.torn_off_pending = Some(TornOffPending {
            winit_id,
            tab_id,
            mouse_offset,
        });

        if let Some(ctx) = self.windows.get(&winit_id) {
            if let Err(e) = ctx.window.window().drag_window() {
                log::warn!("drag_window failed: {e}");
            }
        }
    }

    /// Start an OS-level drag on a single-tab window (Windows).
    #[cfg(target_os = "windows")]
    pub(super) fn begin_single_tab_os_drag(&mut self, _event_loop: &ActiveEventLoop) {
        let (tab_id, mouse_offset, origin_y, winit_id) = {
            let Some(ctx) = self.focused_ctx_mut() else {
                return;
            };
            let Some(drag) = ctx.tab_drag.take() else {
                return;
            };
            ctx.tab_bar.set_drag_visual(None);
            let wid = ctx.window.window_id();
            (drag.tab_id, drag.mouse_offset_in_tab, drag.origin_y, wid)
        };

        self.release_tab_width_lock();
        self.begin_os_tab_drag(winit_id, tab_id, mouse_offset, origin_y);
    }

    /// Collect tab bar merge rects from all primary windows except `exclude`.
    #[cfg(target_os = "windows")]
    fn collect_merge_rects(&self, exclude: WindowId) -> Vec<[i32; 4]> {
        let mut rects = Vec::new();
        for managed in self.window_manager.windows_of_kind(WindowKind::is_primary) {
            let wid = managed.winit_id;
            if wid == exclude {
                continue;
            }
            let Some(ctx) = self.windows.get(&wid) else {
                continue;
            };
            let scale = ctx.window.scale_factor().factor() as f32;
            let tab_bar_h = (TAB_BAR_HEIGHT * scale).round() as i32;
            let controls_w = (CONTROLS_ZONE_WIDTH * scale).round() as i32;
            if let Some((l, t, r, _)) =
                platform_windows::visible_frame_bounds(ctx.window.window())
            {
                rects.push([l, t, r - controls_w, t + tab_bar_h]);
            }
        }
        rects
    }
}
