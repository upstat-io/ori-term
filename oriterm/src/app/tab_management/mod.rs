//! Tab lifecycle — create, close, duplicate, cycle, reorder.
//!
//! All operations go through the mux layer (flat pane server). The GUI
//! session owns tab/window/layout state; the App owns rendering state
//! (tab bar layout, animation offsets).

mod move_ops;
mod sync;
mod width_lock;

use std::path::PathBuf;

use oriterm_mux::PaneId;
use oriterm_mux::backend::MuxBackend;
use oriterm_mux::domain::SpawnConfig;

use crate::session::{SessionRegistry, Tab, TabId, WindowId as SessionWindowId};

use super::App;

/// Pure helper underlying [`App::clear_tab_bells`].
///
/// For each pane in `tab_id`'s `all_panes()` set, calls
/// `mux.clear_bell(pane_id)` and `mux.mark_output_seen(pane_id)`.
/// Iteration order matches `Tab::all_panes()` (depth-first split-tree
/// traversal); the order does not affect correctness but is stable for
/// test pins. The function reads `session` immutably and mutates `mux`
/// — the App method is responsible for the borrow-split.
///
/// Exposed at module scope (not `impl App`) so the cross-pane sweep can
/// be exercised directly against a real `EmbeddedMux` in tests, without
/// constructing a full `App`. The behavioral pin lives in
/// `tab_management/tests.rs`: a tab with N split panes registers N
/// `clear_bell` + `mark_output_seen` calls, regardless of which pane
/// the tab marks as active.
pub(super) fn clear_tab_bells_impl(
    session: &SessionRegistry,
    mux: &mut dyn MuxBackend,
    tab_id: TabId,
) {
    let pane_ids = session
        .get_tab(tab_id)
        .map(Tab::all_panes)
        .unwrap_or_default();
    for pid in pane_ids {
        mux.clear_bell(pid);
        mux.mark_output_seen(pid);
    }
}

impl App {
    /// Create a new tab in the given window.
    ///
    /// Inherits CWD from the active pane in the current tab. Spawns a
    /// pane via the mux, then creates a local tab and registers it in
    /// the session.
    pub(super) fn new_tab_in_window(&mut self, window_id: SessionWindowId) {
        let cwd = self
            .active_pane_id()
            .and_then(|id| self.mux.as_ref()?.pane_cwd(id))
            .map(PathBuf::from);

        let (rows, cols) = self.current_grid_dims();
        let (cell_w, cell_h) = self.current_cell_dims();
        let theme = self
            .config
            .colors
            .resolve_theme(crate::platform::theme::system_theme);

        let config = SpawnConfig {
            cols,
            rows,
            scrollback: self.config.terminal.scrollback,
            shell_integration: self.config.behavior.shell_integration,
            shell: self.config.terminal.shell.clone(),
            cwd,
            ..SpawnConfig::default()
        };

        let palette =
            crate::app::config_reload::build_palette_from_config(&self.config.colors, theme);

        let Some(mux) = &mut self.mux else { return };
        let pane_id = match mux.spawn_pane(&config, theme) {
            Ok(pid) => {
                mux.set_pane_theme(pid, theme, palette);
                mux.set_image_config(pid, self.config.terminal.image_config());
                mux.set_bold_is_bright(pid, self.config.behavior.bold_is_bright);
                mux.set_cell_dimensions(pid, cell_w, cell_h);
                pid
            }
            Err(e) => {
                log::error!("new tab failed: {e}");
                return;
            }
        };

        // Local tab creation.
        let tab_id = self.session.alloc_tab_id();
        let tab = Tab::new(tab_id, pane_id);
        self.session.add_tab(tab);
        if let Some(win) = self.session.get_window_mut(window_id) {
            win.add_tab(tab_id);
        }
        log::info!("new tab {tab_id:?} with pane {pane_id:?} in window {window_id:?}");

        self.release_tab_width_lock();
        self.sync_tab_bar_from_mux();
        self.resize_all_panes();
        if let Some(wid) = self.focused_window_id {
            self.refresh_platform_rects(wid);
        }
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.pane_cache.invalidate_all();
            ctx.cached_dividers = None;
            ctx.root.mark_dirty();
        }
    }

    /// Close a tab and all its panes.
    ///
    /// If this was the last tab in the last window, shuts down immediately
    /// (ConPTY-safe: `process::exit` before dropping panes). If this was the
    /// last tab in a non-last window, the empty window is closed too.
    /// Otherwise pane cleanup happens via `PaneClosed` notifications in
    /// `pump_mux_events`.
    pub(super) fn close_tab(&mut self, tab_id: TabId) {
        // Capture slide animation data before mutations.
        let slide_info = self.capture_close_slide_info(tab_id);

        let is_last = self.session.tab_count() <= 1;
        let owner_window = self.session.window_for_tab(tab_id);

        // Collect pane IDs from local session before removing the tab.
        let pane_ids: Vec<PaneId> = self
            .session
            .get_tab(tab_id)
            .map(Tab::all_panes)
            .unwrap_or_default();

        // Close each pane through the mux (unregisters from pane registry,
        // emits PaneClosed for cleanup in pump_mux_events).
        if let Some(mux) = &mut self.mux {
            for &pid in &pane_ids {
                mux.close_pane(pid);
            }
        }

        // Remove tab from local session.
        self.session.remove_tab(tab_id);
        if let Some(wid) = owner_window {
            if let Some(win) = self.session.get_window_mut(wid) {
                win.remove_tab(tab_id);
            }
        }

        if is_last {
            log::info!("last tab closed, shutting down");
            self.exit_app();
        }

        // If the owning window is now empty (last tab in a non-last window),
        // close it. This handles torn-off windows and multi-window setups.
        if let Some(win_id) = owner_window {
            let window_empty = self
                .session
                .get_window(win_id)
                .is_some_and(|w| w.tabs().is_empty());
            if window_empty {
                self.close_empty_session_window(win_id);
                return;
            }
        }

        self.sync_tab_bar_from_mux();
        if let Some(wid) = self.focused_window_id {
            self.refresh_platform_rects(wid);
        }

        // Start slide animation for displaced tabs (skip if last tab).
        if let Some((closed_idx, tab_width)) = slide_info {
            self.start_tab_close_slide(closed_idx, tab_width);
        }

        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.root.mark_dirty();
        }
    }

    /// Close the currently active tab.
    pub(super) fn close_active_tab(&mut self) {
        let Some(tab_id) = self.active_tab_id() else {
            return;
        };
        self.close_tab(tab_id);
    }

    /// Close the tab at a specific index in the active window.
    ///
    /// Used by tab bar close-button clicks. Resolves the tab ID from the
    /// index and delegates to `close_tab`.
    pub(super) fn close_tab_at_index(&mut self, index: usize) {
        let tab_id = {
            let Some(win_id) = self.active_window else {
                return;
            };
            let Some(win) = self.session.get_window(win_id) else {
                return;
            };
            match win.tabs().get(index).copied() {
                Some(id) => id,
                None => return,
            }
        };
        self.close_tab(tab_id);
    }

    /// Duplicate the active tab (new shell in the same CWD).
    pub(super) fn duplicate_active_tab(&mut self) {
        let Some(window_id) = self.active_window else {
            return;
        };
        self.new_tab_in_window(window_id);
    }

    /// Cycle to the next or previous tab in the active window.
    pub(super) fn cycle_tab(&mut self, delta: isize) {
        let Some(win_id) = self.active_window else {
            return;
        };
        let cycled = {
            let Some(win) = self.session.get_window_mut(win_id) else {
                return;
            };
            let count = win.tabs().len();
            if count == 0 {
                return;
            }
            let current = win.active_tab_idx();
            let new_idx = (current as isize + delta).rem_euclid(count as isize) as usize;
            if new_idx == current {
                return;
            }
            win.set_active_tab_idx(new_idx);
            true
        };
        if !cycled {
            return;
        }

        // Clear bell/unseen-output badges across ALL panes in the newly
        // active tab, not just the active pane. A tab with N split panes
        // whose non-active pane retained `has_bell` from a prior
        // background-bell would never clear its icon otherwise.
        if let Some(active_tab_id) = self
            .session
            .get_window(win_id)
            .and_then(crate::session::Window::active_tab)
        {
            self.clear_tab_bells(active_tab_id);
        }

        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.pane_cache.invalidate_all();
            ctx.cached_dividers = None;
            ctx.root.mark_dirty();
        }
        self.resize_all_panes();
        self.sync_tab_bar_from_mux();
    }

    /// Switch to a specific tab by its ID.
    pub(super) fn switch_to_tab(&mut self, tab_id: TabId) {
        let Some(win_id) = self.active_window else {
            return;
        };
        {
            let Some(win) = self.session.get_window_mut(win_id) else {
                return;
            };
            let Some(idx) = win.tabs().iter().position(|&id| id == tab_id) else {
                return;
            };
            win.set_active_tab_idx(idx);
        }

        // Clear bells across ALL panes in the newly focused tab, not
        // just `active_pane_id`.
        self.clear_tab_bells(tab_id);

        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.pane_cache.invalidate_all();
            ctx.cached_dividers = None;
            ctx.root.mark_dirty();
        }
        self.resize_all_panes();
        self.sync_tab_bar_from_mux();
    }

    /// Clear bell + unseen-output indicators on every pane in `tab_id`.
    ///
    /// Focus-change clear paths drain bell state across ALL panes in the
    /// newly-active tab, not just the keyboard-focused split.
    /// Delegates to the free function [`clear_tab_bells_impl`] (the
    /// testable form; the App method is the production caller). The
    /// free function is exercised directly against `EmbeddedMux` in
    /// `tab_management/tests.rs` so the per-pane sweep — `clear_bell` +
    /// `mark_output_seen` for every pane in `Tab::all_panes()`, not
    /// just the active pane — has a behavioral regression pin.
    pub(super) fn clear_tab_bells(&mut self, tab_id: TabId) {
        if let Some(mux) = self.mux.as_mut() {
            clear_tab_bells_impl(&self.session, mux.as_mut(), tab_id);
        }
    }

    /// Switch to a tab by its index in the active window.
    pub(super) fn switch_to_tab_index(&mut self, index: usize) {
        let Some(win_id) = self.active_window else {
            return;
        };
        let tab_id = {
            let Some(win) = self.session.get_window(win_id) else {
                return;
            };
            match win.tabs().get(index).copied() {
                Some(id) => id,
                None => return,
            }
        };
        self.switch_to_tab(tab_id);
    }

    // -- Private helpers --

    /// Captures the tab index and width for a close slide animation.
    ///
    /// Returns `None` if the tab or window context cannot be resolved.
    fn capture_close_slide_info(&self, tab_id: TabId) -> Option<(usize, f32)> {
        let win_id = self.active_window?;
        let win = self.session.get_window(win_id)?;
        let idx = win.tabs().iter().position(|&id| id == tab_id)?;
        let tab_width = self.focused_ctx()?.tab_bar.layout().base_tab_width();
        Some((idx, tab_width))
    }

    /// Starts a close-slide animation and syncs offsets to the widget.
    fn start_tab_close_slide(&mut self, closed_idx: usize, tab_width: f32) {
        use oriterm_ui::widgets::tab_bar::slide::SlideContext;

        let now = std::time::Instant::now();
        let Some(ctx) = self.focused_ctx_mut() else {
            return;
        };
        let tab_count = ctx.tab_bar.tab_count();
        let (tree, animator) = ctx.root.layer_tree_and_animator_mut();
        let mut cx = SlideContext {
            tree,
            animator,
            now,
        };
        ctx.tab_slide
            .start_close_slide(closed_idx, tab_width, tab_count, &mut cx);
        ctx.tab_slide
            .sync_to_widget(tab_count, ctx.root.layer_tree(), &mut ctx.tab_bar);
    }

    /// Starts a reorder-slide animation and syncs offsets to the widget.
    pub(super) fn start_tab_reorder_slide(&mut self, from: usize, to: usize, tab_width: f32) {
        use oriterm_ui::widgets::tab_bar::slide::SlideContext;

        let now = std::time::Instant::now();
        let Some(ctx) = self.focused_ctx_mut() else {
            return;
        };
        let tab_count = ctx.tab_bar.tab_count();
        let (tree, animator) = ctx.root.layer_tree_and_animator_mut();
        let mut cx = SlideContext {
            tree,
            animator,
            now,
        };
        ctx.tab_slide
            .start_reorder_slide(from, to, tab_width, &mut cx);
        ctx.tab_slide
            .sync_to_widget(tab_count, ctx.root.layer_tree(), &mut ctx.tab_bar);
    }

    /// The active tab ID for the active window.
    fn active_tab_id(&self) -> Option<TabId> {
        let win_id = self.active_window?;
        self.session.get_window(win_id)?.active_tab()
    }

    /// Current grid dimensions (rows, cols) from the grid widget.
    fn current_grid_dims(&self) -> (u16, u16) {
        self.focused_ctx().map_or((24, 80), |ctx| {
            (
                ctx.terminal_grid.rows() as u16,
                ctx.terminal_grid.cols() as u16,
            )
        })
    }
}

/// Wrapping index arithmetic for tab cycling.
#[cfg(test)]
fn wrap_index(current: usize, delta: isize, count: usize) -> usize {
    let c = count as isize;
    let next = (current as isize + delta).rem_euclid(c);
    next as usize
}

#[cfg(test)]
mod tests;
