//! Tab title inline editing.
//!
//! Handles committing, cancelling, and routing key events during
//! tab title inline editing mode.

use winit::event::ElementState;

use super::super::App;
use super::{TabEditAction, tab_edit_key_action};

impl App {
    /// Commit an active tab title edit.
    ///
    /// Sets the title override on the session `Tab` so the user-set title
    /// persists across OSC title changes. Also marks dirty for repaint.
    pub(in crate::app) fn commit_tab_edit(&mut self) {
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
    pub(in crate::app) fn cancel_tab_edit(&mut self) {
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
    pub(in crate::app) fn handle_tab_editing_key(
        &mut self,
        event: &winit::event::KeyEvent,
    ) -> bool {
        let is_editing = self
            .focused_ctx()
            .is_some_and(|ctx| ctx.tab_bar.is_editing());
        if !is_editing || event.state != ElementState::Pressed {
            return false;
        }

        let shift = self.modifiers.shift_key();
        let ctrl = self.modifiers.control_key();

        match tab_edit_key_action(&event.logical_key, shift, ctrl) {
            TabEditAction::Commit => self.commit_tab_edit(),
            TabEditAction::Cancel => self.cancel_tab_edit(),
            TabEditAction::Backspace => {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.editing_backspace();
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
            }
            TabEditAction::Delete => {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.editing_delete();
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
            }
            TabEditAction::MoveLeft { extend_selection } => {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.editing_move_left(extend_selection);
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
            }
            TabEditAction::MoveRight { extend_selection } => {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.editing_move_right(extend_selection);
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
            }
            TabEditAction::Home { extend_selection } => {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.editing_home(extend_selection);
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
            }
            TabEditAction::End { extend_selection } => {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.editing_end(extend_selection);
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
            }
            TabEditAction::SelectAll => {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.editing_select_all();
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
            }
            TabEditAction::InsertChars(chars) => {
                for c in chars.chars() {
                    if let Some(ctx) = self.focused_ctx_mut() {
                        ctx.tab_bar.editing_insert_char(c);
                        ctx.root.mark_dirty();
                        ctx.ui_stale = true;
                    }
                }
            }
            TabEditAction::Consumed => {}
        }
        true
    }
}
