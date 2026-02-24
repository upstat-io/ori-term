//! Keyboard input dispatch for the application.
//!
//! Routes key events through mark mode, keybinding table lookup, and
//! finally key encoding to the PTY. Also handles IME commit events.

use winit::event::ElementState;
use winit::keyboard::SmolStr;

use super::{App, mark_mode};
use crate::key_encoding::{self, KeyEventType, KeyInput};
use crate::keybindings::{self, Action};

impl App {
    /// Dispatch a keyboard event through mark mode, keybindings, or PTY encoding.
    ///
    /// Priority order:
    /// 1. Mark mode (if active, consumes all events).
    /// 2. Keybinding table lookup.
    /// 3. Normal key encoding to PTY.
    pub(super) fn handle_keyboard_input(&mut self, event: &winit::event::KeyEvent) {
        // Suppress raw key events during active IME composition.
        // The IME subsystem sends Ime::Commit when done; raw KeyboardInput
        // events during composition are intermediate and must not reach the PTY.
        if self.ime_active && !self.ime_preedit.is_empty() {
            return;
        }

        // Mark mode: consume ALL key events (including releases) to prevent
        // leaking input to the PTY while navigating.
        if let Some(tab) = &mut self.tab {
            if tab.is_mark_mode() {
                if event.state == ElementState::Pressed {
                    let action = mark_mode::handle_mark_mode_key(
                        tab,
                        event,
                        self.modifiers,
                        &self.config.behavior.word_delimiters,
                    );
                    match action {
                        mark_mode::MarkAction::Handled => {
                            self.dirty = true;
                        }
                        mark_mode::MarkAction::Exit { copy } => {
                            if copy {
                                self.copy_selection();
                            }
                            self.dirty = true;
                        }
                        mark_mode::MarkAction::Ignored => {}
                    }
                }
                return;
            }
        }

        // Keybinding dispatch: look up the key+modifiers in the binding table.
        if event.state == ElementState::Pressed {
            let mods = self.modifiers.into();
            if let Some(binding_key) = keybindings::key_to_binding_key(&event.logical_key) {
                if let Some(action) = keybindings::find_binding(&self.bindings, &binding_key, mods)
                {
                    // Clone to release the immutable borrow on self.bindings
                    // before calling execute_action which needs &mut self.
                    let action = action.clone();
                    if self.execute_action(&action) {
                        return;
                    }
                }
            }
        }

        // Normal key encoding to PTY.
        self.encode_key_to_pty(event);
    }

    /// Encode a key event and send the result to the PTY.
    fn encode_key_to_pty(&mut self, event: &winit::event::KeyEvent) {
        let Some(tab) = &self.tab else { return };

        let mode = tab.terminal().lock().mode();

        let event_type = match (event.state, event.repeat) {
            (ElementState::Released, _) => KeyEventType::Release,
            (ElementState::Pressed, true) => KeyEventType::Repeat,
            (ElementState::Pressed, false) => KeyEventType::Press,
        };

        let bytes = key_encoding::encode_key(&KeyInput {
            key: &event.logical_key,
            mods: self.modifiers.into(),
            mode,
            text: event.text.as_ref().map(SmolStr::as_str),
            location: event.location,
            event_type,
        });

        if !bytes.is_empty() {
            tab.scroll_to_bottom();
            tab.write_input(&bytes);
            self.cursor_blink.reset();
            self.dirty = true;
        }
    }

    /// Execute a keybinding action. Returns `true` if the event was consumed.
    ///
    /// `SmartCopy` returns `false` when no selection exists (fall through to PTY
    /// so Ctrl+C sends SIGINT). Other actions always consume the event.
    fn execute_action(&mut self, action: &Action) -> bool {
        match action {
            Action::Copy => {
                self.copy_selection();
                self.dirty = true;
                true
            }
            Action::Paste | Action::SmartPaste => {
                self.paste_from_clipboard();
                self.dirty = true;
                true
            }
            Action::SmartCopy => {
                let has_sel = self.tab.as_ref().is_some_and(|t| t.selection().is_some());
                if has_sel {
                    self.copy_selection();
                    self.dirty = true;
                    true
                } else {
                    false
                }
            }
            Action::ScrollPageUp => self.execute_scroll(true),
            Action::ScrollPageDown => self.execute_scroll(false),
            Action::ScrollToTop => {
                if let Some(tab) = &self.tab {
                    tab.scroll_display(isize::MAX);
                }
                self.dirty = true;
                true
            }
            Action::ScrollToBottom => {
                if let Some(tab) = &self.tab {
                    tab.scroll_to_bottom();
                }
                self.dirty = true;
                true
            }
            Action::ReloadConfig => {
                self.apply_config_reload();
                true
            }
            Action::ToggleFullscreen => {
                if let Some(window) = &self.window {
                    let is_fs = window.is_fullscreen();
                    window.set_fullscreen(!is_fs);
                }
                true
            }
            Action::EnterMarkMode => {
                if let Some(tab) = &mut self.tab {
                    tab.enter_mark_mode();
                    self.dirty = true;
                }
                true
            }
            Action::SendText(text) => {
                if let Some(tab) = &self.tab {
                    tab.scroll_to_bottom();
                    tab.write_input(text.as_bytes());
                    self.cursor_blink.reset();
                }
                self.dirty = true;
                true
            }
            // Actions for future sections — consume the event but log a stub.
            Action::NewTab
            | Action::CloseTab
            | Action::NextTab
            | Action::PrevTab
            | Action::ZoomIn
            | Action::ZoomOut
            | Action::ZoomReset
            | Action::OpenSearch
            | Action::PreviousPrompt
            | Action::NextPrompt
            | Action::DuplicateTab
            | Action::MoveTabToNewWindow => {
                log::debug!("keybinding action not yet implemented: {action:?}");
                true
            }
            Action::None => true,
        }
    }

    /// Scroll by one page in the given direction.
    fn execute_scroll(&mut self, up: bool) -> bool {
        if let Some(tab) = &self.tab {
            let term = tab.terminal().lock();
            let lines = term.grid().lines() as isize;
            drop(term);
            tab.scroll_display(if up { lines } else { -lines });
        }
        self.dirty = true;
        true
    }

    /// Dispatch an IME event: preedit, commit, enabled, or disabled.
    pub(super) fn handle_ime_event(&mut self, ime: winit::event::Ime) {
        match ime {
            winit::event::Ime::Enabled => {
                self.ime_active = true;
                self.dirty = true;
            }
            winit::event::Ime::Preedit(text, cursor) => {
                self.ime_preedit = text;
                self.ime_preedit_cursor = cursor.map(|(start, _)| start);
                self.update_ime_cursor_area();
                self.dirty = true;
            }
            winit::event::Ime::Commit(text) => {
                self.ime_preedit.clear();
                self.ime_preedit_cursor = None;
                self.ime_active = false;
                self.handle_ime_commit(&text);
            }
            winit::event::Ime::Disabled => {
                self.ime_active = false;
                self.ime_preedit.clear();
                self.ime_preedit_cursor = None;
                self.dirty = true;
            }
        }
    }

    /// Update the IME candidate window position to match the terminal cursor.
    ///
    /// Tells the OS where to place the IME candidate/composition popup so it
    /// appears near the cursor. Uses 2× cell width for the exclusion zone
    /// (Alacritty convention — avoids tight exclusion on right edge).
    pub(super) fn update_ime_cursor_area(&self) {
        let Some(window) = &self.window else { return };
        let Some(tab) = &self.tab else { return };
        let Some(renderer) = &self.renderer else {
            return;
        };
        let Some(grid_widget) = &self.terminal_grid else {
            return;
        };
        let Some(bounds) = grid_widget.bounds() else {
            return;
        };

        let metrics = renderer.cell_metrics();
        let term = tab.terminal().lock();
        let cursor = term.grid().cursor();
        let cursor_line = cursor.line();
        let cursor_col = cursor.col().0;
        drop(term);

        // Pixel position of the cursor cell, relative to the window.
        let x = f64::from(bounds.x()) + cursor_col as f64 * f64::from(metrics.width);
        let y = f64::from(bounds.y()) + cursor_line as f64 * f64::from(metrics.height);

        // Exclusion zone: 2× cell width, 1× cell height.
        let w = f64::from(metrics.width) * 2.0;
        let h = f64::from(metrics.height);

        window.window().set_ime_cursor_area(
            winit::dpi::PhysicalPosition::new(x, y),
            winit::dpi::PhysicalSize::new(w, h),
        );
    }

    /// Handle IME commit: send committed text directly to the PTY.
    pub(super) fn handle_ime_commit(&mut self, text: &str) {
        let Some(tab) = &self.tab else { return };
        if !text.is_empty() {
            tab.scroll_to_bottom();
            tab.write_input(text.as_bytes());
            self.cursor_blink.reset();
            self.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests;
