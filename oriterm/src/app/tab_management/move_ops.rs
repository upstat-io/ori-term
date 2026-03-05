//! Tab move and reorder operations.
//!
//! Extracted from `tab_management/mod.rs` for file size compliance.

use oriterm_mux::{TabId, WindowId as MuxWindowId};

use super::super::App;

impl App {
    /// Move a tab to a different window.
    ///
    /// Preserves the tab's panes and split layout. If the source window
    /// becomes empty, it is closed. Panes in the moved tab are resized to
    /// fit the destination window dimensions.
    pub(in crate::app) fn move_tab_to_window(&mut self, tab_id: TabId, dest_window: MuxWindowId) {
        let Some(mux) = &mut self.mux else { return };
        if !mux.move_tab_to_window(tab_id, dest_window) {
            return;
        }

        // Mux notifications (WindowTabsChanged, WindowClosed, TabLayoutChanged)
        // are processed in the normal pump_mux_events cycle. Sync both windows'
        // tab bars immediately so the UI doesn't lag.
        self.release_tab_width_lock();
        self.sync_tab_bar_from_mux();

        // Resize panes in the moved tab to fit the destination window.
        self.resize_all_panes();

        // Mark the destination window dirty.
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.pane_cache.invalidate_all();
            ctx.cached_dividers = None;
            ctx.dirty = true;
        }
    }

    /// Sends a deferred move-tab-to-new-window event through the event loop.
    ///
    /// The actual tab move happens in `user_event()` where `ActiveEventLoop`
    /// is available.
    pub(in crate::app) fn move_tab_to_new_window_deferred(&self, tab_index: usize) {
        let _ = self
            .event_proxy
            .send_event(crate::event::TermEvent::MoveTabToNewWindow(tab_index));
    }

    /// Move a tab to a new window.
    ///
    /// In embedded mode: creates a new OS window in this process, moves
    /// the tab there. In daemon mode: creates a new mux window via the
    /// daemon, moves the tab, then spawns a new `oriterm` process with
    /// `--connect` + `--window` to render it.
    ///
    /// Refuses if the tab is the last tab in the last window.
    #[allow(dead_code, reason = "superseded by tear_off_tab in Section 17.2")]
    pub(in crate::app) fn move_tab_to_new_window(
        &mut self,
        tab_id: TabId,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        // Refuse if this is the last tab in the entire session.
        let is_last = self
            .mux
            .as_ref()
            .is_some_and(|m| m.session().tab_count() <= 1);
        if is_last {
            log::warn!("move_tab_to_new_window: refused — last tab in session");
            return;
        }

        let is_daemon = self.mux.as_ref().is_some_and(|m| m.is_daemon_mode());

        if is_daemon {
            self.move_tab_to_new_window_daemon(tab_id);
        } else {
            self.move_tab_to_new_window_embedded(tab_id, event_loop);
        }
    }

    /// Daemon-mode: create window via daemon, move tab, spawn new process.
    pub(in crate::app) fn move_tab_to_new_window_daemon(&mut self, tab_id: TabId) {
        let Some(mux) = &mut self.mux else { return };

        // Create a new empty window in the daemon.
        let new_window_id = match mux.create_window() {
            Ok(id) => id,
            Err(e) => {
                log::error!("move_tab_to_new_window_daemon: failed to create window: {e}");
                return;
            }
        };

        // Move the tab to the new window.
        if !mux.move_tab_to_window(tab_id, new_window_id) {
            log::error!("move_tab_to_new_window_daemon: failed to move tab");
            return;
        }

        // Spawn a new oriterm process to render the new window.
        // It connects to the same daemon socket and claims the window ID.
        {
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
                .arg(new_window_id.raw().to_string());
            match cmd.spawn() {
                Ok(child) => {
                    log::info!(
                        "spawned new window process (pid={}) for {new_window_id}",
                        child.id()
                    );
                }
                Err(e) => {
                    log::error!("failed to spawn new window process: {e}");
                }
            }
        }

        // Sync tab bars for the source window.
        self.release_tab_width_lock();
        self.sync_tab_bar_from_mux();
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.dirty = true;
        }
    }

    /// Embedded-mode: create in-process window, move tab there.
    fn move_tab_to_new_window_embedded(
        &mut self,
        tab_id: TabId,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        // Create new window (GPU, surface, chrome, initial tab).
        let Some(new_winit_id) = self.create_window(event_loop) else {
            return;
        };

        // The new window got a fresh initial tab. Find the mux window ID,
        // then move the requested tab there BEFORE closing the initial tab.
        let Some(ctx) = self.windows.get(&new_winit_id) else {
            return;
        };
        let new_mux_id = ctx.window.mux_window_id();

        // Capture the initial tab ID before moving (the move changes active tab).
        let initial_tab = self.mux.as_ref().and_then(|m| m.active_tab_id(new_mux_id));

        // Move the requested tab to the new window (now has 2 tabs).
        self.move_tab_to_window(tab_id, new_mux_id);

        // Close the initial (empty) tab that `create_window` spawned
        // (window now has 1 tab — the moved one).
        if let Some(initial) = initial_tab {
            if let Some(mux) = &mut self.mux {
                let pane_ids = mux.close_tab(initial);
                for pid in pane_ids {
                    mux.cleanup_closed_pane(pid);
                }
            }
        }

        // Sync tab bars: old window lost a tab, new window gained one.
        self.sync_tab_bar_from_mux();
        self.sync_tab_bar_for_window(new_winit_id);
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.dirty = true;
        }
    }

    /// Reorder a tab within the active window (with animation).
    #[allow(
        dead_code,
        reason = "used by keybinding-driven reorder; drag uses reorder_tab_silent"
    )]
    pub(in crate::app) fn move_tab(&mut self, from: usize, to: usize) {
        // Capture tab width before the mutable mux borrow.
        let tab_width = self
            .focused_ctx()
            .map_or(0.0, |ctx| ctx.tab_bar.layout().tab_width);

        let Some(mux) = &mut self.mux else { return };
        let Some(win_id) = self.active_window else {
            return;
        };

        if !mux.reorder_tab(win_id, from, to) {
            return;
        }
        self.sync_tab_bar_from_mux();

        // Start slide animation for displaced tabs.
        self.start_tab_reorder_slide(from, to, tab_width);

        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.dirty = true;
        }
    }
}
