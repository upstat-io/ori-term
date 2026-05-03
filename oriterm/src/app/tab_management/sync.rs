//! Tab-bar sync helpers — rebuild tab bar entries from session + mux state.
//!
//! Four variants by routing scope, all delegating to one canonical
//! [`App::sync_tab_bar_for_session_window`]:
//! - [`App::sync_tab_bar_from_mux`] — active-window-scoped (default sync).
//! - [`App::sync_tab_bar_for_pane`] — owning-window-of-pane-scoped (used for
//!   per-pane mux notifications: bell, command-complete, OSC notifications,
//!   pane output, pane metadata changes, pane close).
//! - [`App::sync_tab_bar_for_window`] — winit-id-scoped (used by tear-off /
//!   merge where source and destination winit windows are both known).
//! - [`App::sync_tab_bar_for_session_window`] — canonical session-window-id
//!   sync (used directly by `handle_pane_closed` where the pane is gone but
//!   we know its former owning window).
//!
//! All four ultimately call into [`build_tab_entries`], which derives
//! `TabEntry`s from a session window's tab list + mux snapshots (titles,
//! icons, modified flag, bell flag).

use oriterm_mux::PaneId;
use winit::window::WindowId;

use crate::app::App;
use crate::session::WindowId as SessionWindowId;

impl App {
    /// Rebuild the tab bar entries on the given session window.
    ///
    /// Canonical implementation — every other `sync_tab_bar_*` helper
    /// resolves its scope to a `SessionWindowId` and delegates here.
    /// Routes through `owning_window_ctx_mut` for the `WindowContext` lookup,
    /// ensuring the SAME `windows.values_mut() + session_window_id() ==
    /// session_wid` walk pattern is used everywhere — no inline duplicates.
    pub(in crate::app) fn sync_tab_bar_for_session_window(
        &mut self,
        session_wid: SessionWindowId,
    ) {
        // Scoped immutable borrow of self.session ends before mutable
        // borrow of self.windows via owning_window_ctx_mut.
        let computed = {
            let Some(mux) = self.mux.as_ref() else { return };
            let Some(win) = self.session.get_window(session_wid) else {
                return;
            };
            build_tab_entries(mux.as_ref(), &self.session, win)
        };
        let (entries, active_idx) = computed;

        if let Some(ctx) = self.owning_window_ctx_mut(session_wid) {
            ctx.tab_bar.set_tabs(entries);
            ctx.tab_bar.set_active_index(active_idx);
        }
    }

    /// Rebuild the tab bar entries from the mux's window state.
    ///
    /// Resolves to the active session window and delegates to
    /// [`App::sync_tab_bar_for_session_window`].
    pub(in crate::app) fn sync_tab_bar_from_mux(&mut self) {
        let Some(session_wid) = self.active_window else {
            return;
        };
        self.sync_tab_bar_for_session_window(session_wid);
    }

    /// Rebuild the tab bar entries on the OWNING window of `pane_id`.
    ///
    /// Used when a mux notification targets a specific pane and we need
    /// its OWNING window's tab bar updated — not the focused window's.
    /// Resolves the pane's owning session window via `pane_position` and
    /// delegates to [`App::sync_tab_bar_for_session_window`].
    pub(in crate::app) fn sync_tab_bar_for_pane(&mut self, pane_id: PaneId) {
        let Some(pos) = self.session.pane_position(pane_id) else {
            return;
        };
        self.sync_tab_bar_for_session_window(pos.window_id);
    }

    /// Rebuild the tab bar for a specific winit window.
    ///
    /// Used by tear-off/merge when both source and destination windows
    /// need their tab bars updated. Resolves the winit ID to a session
    /// window id and delegates to [`App::sync_tab_bar_for_session_window`].
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub(in crate::app) fn sync_tab_bar_for_window(&mut self, winit_id: WindowId) {
        let Some(session_wid) = self
            .windows
            .get(&winit_id)
            .map(|c| c.window.session_window_id())
        else {
            return;
        };
        self.sync_tab_bar_for_session_window(session_wid);
    }
}

/// Build tab bar entries from a session window's tab list.
///
/// Returns `(entries, active_tab_index)`. Shared by all
/// `sync_tab_bar_*` variants above.
fn build_tab_entries(
    mux: &dyn oriterm_mux::backend::MuxBackend,
    session: &crate::session::SessionRegistry,
    win: &crate::session::Window,
) -> (Vec<oriterm_ui::widgets::tab_bar::TabEntry>, usize) {
    let active_idx = win.active_tab_idx();
    let entries = win
        .tabs()
        .iter()
        .map(|&tab_id| {
            let tab = session.get_tab(tab_id);
            let pane_id = tab.map(crate::session::Tab::active_pane);
            let snapshot = pane_id.and_then(|pid| mux.pane_snapshot(pid));
            // User-set title override takes priority over OSC-derived title.
            // OSC icons still show dynamically alongside the overridden title.
            let has_override = tab.is_some_and(|t| t.title_override().is_some());
            let mut title = if has_override {
                tab.and_then(|t| t.title_override().map(str::to_owned))
                    .unwrap_or_default()
            } else {
                snapshot.map(|s| s.title.clone()).unwrap_or_default()
            };
            let icon = snapshot
                .and_then(|s| s.icon_name.as_deref())
                .and_then(oriterm_ui::widgets::tab_bar::extract_emoji_icon);
            // Strip leading emoji from title when it matches the icon
            // (OSC 0 sets both title and icon_name to the same string).
            // Only strip from OSC-derived titles, not user overrides.
            if !has_override {
                if let Some(oriterm_ui::widgets::tab_bar::TabIcon::Emoji(ref e)) = icon {
                    let stripped = title
                        .strip_prefix(e.as_str())
                        .map(|r| r.trim_start().to_owned());
                    if let Some(s) = stripped {
                        title = s;
                    }
                }
            }
            let is_zoomed = tab.is_some_and(|t| t.zoomed_pane().is_some());
            let display = if is_zoomed {
                format!("{title} [Z]")
            } else {
                title
            };
            let modified =
                tab.is_some_and(|t| t.all_panes().iter().any(|&pid| mux.has_unseen_output(pid)));
            let has_bell = tab.is_some_and(|t| t.all_panes().iter().any(|&pid| mux.has_bell(pid)));
            oriterm_ui::widgets::tab_bar::TabEntry::new(display)
                .with_icon(icon)
                .with_modified(modified)
                .with_bell(has_bell)
        })
        .collect();
    (entries, active_idx)
}
