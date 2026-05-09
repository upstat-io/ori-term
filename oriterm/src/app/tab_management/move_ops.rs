//! Tab move and reorder operations.
//!
//! Extracted from `tab_management/mod.rs` for file size compliance.

use crate::session::TabId;
use crate::window_manager::types::{ManagedWindow, WindowKind};
use base64::{Engine as _, engine::general_purpose};

use crate::app::App;

impl App {
    /// Sends a deferred move-tab-to-new-window event through the event loop.
    ///
    /// The actual tab move happens in `user_event()` where `ActiveEventLoop`
    /// is available.
    pub(in crate::app) fn move_tab_to_new_window_deferred(
        &self,
        tab_id: TabId,
        position: Option<(i32, i32)>,
    ) {
        self.event_proxy
            .send(crate::event::TermEvent::MoveTabToNewWindow(
                tab_id, position,
            ));
    }

    /// Move a tab to a new window.
    ///
    /// In embedded mode: creates a new OS window in this process, moves
    /// the tab there. In daemon mode: creates a new mux window via the
    /// daemon, moves the tab, then spawns a new `oriterm` process with
    /// `--connect` + `--window` to render it.
    ///
    /// Refuses if the tab is the last tab in the last window.
    pub(in crate::app) fn move_tab_to_new_window(
        &mut self,
        tab_id: TabId,
        event_loop: &winit::event_loop::ActiveEventLoop,
        position: Option<(i32, i32)>,
    ) {
        // Refuse if this is the last tab in the entire session.
        let is_last = self.session.tab_count() <= 1;
        if is_last {
            log::warn!("move_tab_to_new_window: refused — last tab in session");
            return;
        }

        let is_daemon = self.mux.as_ref().is_some_and(|m| m.is_daemon_mode());

        if is_daemon {
            self.move_tab_to_new_window_daemon(tab_id, position);
        } else {
            self.move_tab_to_new_window_embedded(tab_id, event_loop);
            // position is handled by position_torn_off_window in embedded mode
        }
    }

    /// Daemon-mode: move tab to a new window process.
    ///
    /// Spawns a new oriterm process connected to the same daemon, and moves
    /// the tab's panes to render in the new process. The local session is
    /// updated directly — no mux session sync needed (mux is a flat pane
    /// server, it doesn't know about tabs or windows).
    pub(in crate::app) fn move_tab_to_new_window_daemon(
        &mut self,
        tab_id: TabId,
        position: Option<(i32, i32)>,
    ) {
        // 1. Get the tab state for serialization.
        let Some(tab) = self.session.get_tab(tab_id).cloned() else {
            log::error!("move_tab_to_new_window_daemon: tab {tab_id} not found");
            return;
        };

        // 2. Serialize and base64 encode for CLI transfer.
        let tab_json = match serde_json::to_vec(&tab) {
            Ok(v) => v,
            Err(e) => {
                log::error!("move_tab_to_new_window_daemon: failed to serialize tab: {e}");
                return;
            }
        };
        let tabs_base64 = general_purpose::STANDARD.encode(tab_json);

        // Allocate a new local window and move the tab there.
        let new_session_wid = self.session.alloc_window_id();
        self.session
            .add_window(crate::session::Window::new(new_session_wid));

        // Move tab from source to destination window locally.
        if let Some(src_wid) = self.session.window_for_tab(tab_id) {
            if let Some(win) = self.session.get_window_mut(src_wid) {
                win.remove_tab(tab_id);
            }
        }
        if let Some(win) = self.session.get_window_mut(new_session_wid) {
            win.add_tab(tab_id);
        }

        // Unsubscribe from the moved panes in the source process.
        // The new process will re-subscribe during init.
        let pane_ids = tab.all_panes();
        if let Some(mux) = self.mux.as_mut() {
            for pid in pane_ids {
                let _ = mux.unsubscribe(pid);
            }
        }

        // Spawn a new oriterm process to render the new window.
        let exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => {
                log::error!("move_tab_to_new_window_daemon: cannot determine exe path: {e}");
                return;
            }
        };
        let socket_path = oriterm_mux::server::socket_path();
        let mut cmd = std::process::Command::new(exe);
        cmd.arg("--connect")
            .arg(&socket_path)
            .arg("--window")
            .arg(new_session_wid.raw().to_string())
            .arg("--tabs-json")
            .arg(tabs_base64);

        if let Some((x, y)) = position {
            cmd.arg("--position").arg(format!("{x},{y}"));
        }

        match cmd.spawn() {
            Ok(child) => {
                log::info!(
                    "spawned new window process (pid={}) for {new_session_wid}",
                    child.id()
                );
            }
            Err(e) => {
                log::error!("failed to spawn new window process: {e}");
            }
        }

        // Sync tab bars for the source window.
        self.release_tab_width_lock();
        self.sync_tab_bar_from_mux();
        if let Some(wid) = self.focused_window_id {
            self.refresh_platform_rects(wid);
        }
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.root.mark_dirty();
        }
    }

    /// Embedded-mode: create in-process window, move tab there.
    ///
    /// Mirrors the working `tear_off_tab` sequence (see
    /// `oriterm/src/app/tab_drag/tear_off.rs`): create a bare (hidden, no
    /// tabs) window, insert the moved tab directly, pump mux notifications,
    /// seed pane cell metrics, sync tab bars + refresh platform rects for
    /// both windows, pre-render the new window with focused-id swap so its
    /// content paints before show, pre-render the source so its tab bar
    /// reflects the removal, then show the new window. Closes the source
    /// window if it ended up empty.
    ///
    /// The previous implementation used `create_window()` (which spawned an
    /// unwanted initial tab) and never explicitly pre-rendered the new
    /// window — the result was a blank window flash ().
    fn move_tab_to_new_window_embedded(
        &mut self,
        tab_id: TabId,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        let source_winit_id = self.focused_window_id;

        // Release the source-window tab-width lock before mutating its tab
        // list — mirrors `tear_off_tab` and prevents stale layout cached
        // against the pre-move tab count.
        self.release_tab_width_lock();

        // Bare window: hidden, no tabs. Caller (this function) inserts the
        // moved tab directly, pre-renders, then shows.
        let Some((new_winit_id, new_session_wid)) = self.create_window_bare(event_loop) else {
            return;
        };

        // Register as a primary Main window — no OS drag follows (unlike
        // tear-off). The Main kind makes it eligible for context-menu
        // operations on its own tabs.
        self.window_manager
            .register(ManagedWindow::new(new_winit_id, WindowKind::Main));

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

        // Seed moved panes with the new window's cell metrics — the
        // `broadcast_cell_metrics_to_window` short-circuit would skip these
        // if the new window's cached dims happen to match the source's.
        let moved_pane_ids: Vec<oriterm_mux::PaneId> = self
            .session
            .get_tab(tab_id)
            .map(crate::session::Tab::all_panes)
            .unwrap_or_default();
        for pid in moved_pane_ids {
            self.seed_pane_with_window_cell_metrics(new_winit_id, pid);
        }

        // Sync tab bars + refresh platform rects on both windows.
        if let Some(src_id) = source_winit_id {
            self.sync_tab_bar_for_window(src_id);
            self.refresh_platform_rects(src_id);
        }
        self.sync_tab_bar_for_window(new_winit_id);
        self.refresh_platform_rects(new_winit_id);

        // Pre-render the new window with content before showing — the
        // focused-id swap forces `handle_redraw` to paint the new window
        // rather than the currently-focused source.
        {
            let saved_focused = self.focused_window_id;
            let saved_active = self.active_window;
            self.focused_window_id = Some(new_winit_id);
            self.active_window = Some(new_session_wid);
            self.handle_redraw();
            self.focused_window_id = saved_focused;
            self.active_window = saved_active;
        }
        // Pre-render the source so its tab bar shows the removed tab.
        self.handle_redraw();

        // Show the new window (it now has rendered content — no blank flash).
        if let Some(ctx) = self.windows.get(&new_winit_id) {
            ctx.window.set_visible(true);
        }

        // Close the source window if it's now empty.
        if let Some(src_id) = source_winit_id {
            let source_empty = self
                .windows
                .get(&src_id)
                .and_then(|ctx| {
                    let win = self.session.get_window(ctx.window.session_window_id())?;
                    Some(win.tabs().is_empty())
                })
                .unwrap_or(false);
            if source_empty {
                self.remove_empty_window(src_id);
            }
        }
    }

    /// Reorder a tab within the active window (with animation).
    #[allow(
        dead_code,
        reason = "used by keybinding-driven reorder; drag uses reorder_tab_silent"
    )]
    pub(in crate::app) fn move_tab(&mut self, from: usize, to: usize) {
        let tab_width = self
            .focused_ctx()
            .map_or(0.0, |ctx| ctx.tab_bar.layout().base_tab_width());

        let Some(win_id) = self.active_window else {
            return;
        };
        let reordered = self
            .session
            .get_window_mut(win_id)
            .is_some_and(|win| win.reorder_tab(from, to));
        if !reordered {
            return;
        }

        self.sync_tab_bar_from_mux();
        if let Some(wid) = self.focused_window_id {
            self.refresh_platform_rects(wid);
        }

        // Start slide animation for displaced tabs.
        self.start_tab_reorder_slide(from, to, tab_width);

        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.root.mark_dirty();
        }
    }
}
