//! Keyboard input dispatch for the application.
//!
//! Routes key events through mark mode, keybinding table lookup, and
//! finally key encoding to the PTY. Also handles IME commit events.

mod action_dispatch;
pub(super) mod ime;
mod mark_mode_dispatch;
mod overlay_dispatch;

use winit::event::ElementState;
use winit::keyboard::SmolStr;

use oriterm_ui::input::Key;

use super::{App, mark_mode};
use crate::key_encoding::{self, KeyEventType, KeyInput};
use crate::keybindings;

pub(super) use ime::ImeState;
use mark_mode_dispatch::{MarkModeDispatch, dispatch_mark_mode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PtyInputRedrawState {
    cursor_hidden_by_blink: bool,
    snapshot_dirty: bool,
    snapshot_display_offset: Option<u32>,
}

fn should_redraw_after_pty_input(state: PtyInputRedrawState) -> bool {
    state.cursor_hidden_by_blink
        || state.snapshot_dirty
        || match state.snapshot_display_offset {
            Some(offset) => offset > 0,
            None => true,
        }
}

impl App {
    /// Dispatch a keyboard event through overlays, mark mode, keybindings,
    /// or PTY encoding.
    ///
    /// Priority order:
    /// 0. Modal overlay (if active, consumes ALL key events).
    /// 1. Mark mode (if active, consumes all events).
    /// 2. Keybinding table lookup.
    /// 3. Normal key encoding to PTY.
    pub(super) fn handle_keyboard_input(&mut self, event: &winit::event::KeyEvent) {
        // Record timestamp for key-to-render latency tracking (profiling mode).
        if event.state == ElementState::Pressed {
            self.perf.last_key_time = Some(std::time::Instant::now());
            self.perf.tick_at_last_key = self.perf.ticks;
        }

        // Cancel active tab drag on Escape press.
        if event.state == ElementState::Pressed
            && event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
            && self.has_tab_drag()
        {
            self.cancel_tab_drag();
            return;
        }

        // Escape dismisses active selection. Only consumed when a selection
        // exists — otherwise falls through to PTY encoding so the shell
        // receives the escape sequence.
        if event.state == ElementState::Pressed
            && event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
        {
            if let Some(pane_id) = self.active_pane_id() {
                if self.pane_selection(pane_id).is_some() {
                    self.clear_pane_selection(pane_id);
                    if let Some(ctx) = self.focused_ctx_mut() {
                        ctx.root.mark_dirty();
                    }
                    return;
                }
            }
        }

        // Suppress raw key events during active IME composition.
        // The IME subsystem sends Ime::Commit when done; raw KeyboardInput
        // events during composition are intermediate and must not reach the PTY.
        if self.ime.should_suppress_key() {
            return;
        }

        // Tab title inline editing: intercept keys before overlays/PTY.
        if self.handle_tab_editing_key(event) {
            return;
        }

        // Modal overlay: intercept keyboard events before anything else.
        if self.try_dispatch_overlay_key(event) {
            return;
        }

        // Search mode: consume ALL key events while search is active.
        if self.is_search_active() {
            self.handle_search_key(event);
            return;
        }

        // Mark mode: consume ALL key events (including releases) to prevent
        // leaking input to the PTY while navigating.
        if self.try_dispatch_mark_mode(event) {
            return;
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

    /// Dispatch a key event to an active modal overlay if any.
    ///
    /// Returns `true` if the event was consumed by an overlay (caller
    /// should return). Only active overlays consume input — dismissing
    /// (fading-out) overlays are visual-only.
    fn try_dispatch_overlay_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        let has_overlays = self
            .focused_ctx()
            .is_some_and(|ctx| !ctx.root.overlays().is_active_empty());
        if !has_overlays || event.state != ElementState::Pressed {
            return false;
        }
        let Some(key) = winit_key_to_ui_key(&event.logical_key) else {
            // Overlays consume even unmapped keys to prevent leaking to PTY.
            return true;
        };
        let ui_event = oriterm_ui::input::KeyEvent {
            key,
            modifiers: super::winit_mods_to_ui(self.modifiers),
        };
        let now = std::time::Instant::now();
        let result = {
            let Some(ctx) = self
                .focused_window_id
                .and_then(|id| self.windows.get_mut(&id))
            else {
                return true;
            };
            let scale = ctx.window.scale_factor().factor() as f32;
            let Some(renderer) = ctx.renderer.as_ref() else {
                return true;
            };
            let measurer = crate::font::CachedTextMeasurer::new(
                renderer.ui_measurer(scale),
                &ctx.text_cache,
                scale,
            );
            ctx.root
                .process_overlay_key_event(ui_event, &measurer, &self.ui_theme, None, now)
        };
        self.handle_overlay_result(result);
        true
    }

    /// Dispatch a key event to mark mode if active.
    ///
    /// Returns `true` if mark mode consumed the event (caller should return).
    fn try_dispatch_mark_mode(&mut self, event: &winit::event::KeyEvent) -> bool {
        // Boundary fast-path: skip MarkModeKeyEvent construction (which clones
        // event.logical_key) on the common case where mark mode is inactive.
        // The internal early returns in dispatch_mark_mode are preserved for
        // test reachability and defense-in-depth.
        let Some(pane_id) = self.active_pane_id() else {
            return false;
        };
        if !self.is_mark_mode(pane_id) {
            return false;
        }
        let modifiers = self.modifiers;
        dispatch_mark_mode(
            MarkModeDispatch {
                event: mark_mode::MarkModeKeyEvent::from_winit(event),
                event_state: event.state,
                event_repeat: event.repeat,
                modifiers,
                active_pane_id: Some(pane_id),
                mark_mode_active: true,
            },
            self,
        )
    }

    /// Encode a key event and send the result to the PTY.
    ///
    /// Works in both embedded mode (local pane) and daemon mode (snapshot
    /// for mode flags, IPC transport for input).
    fn encode_key_to_pty(&mut self, event: &winit::event::KeyEvent) {
        let Some(pane_id) = self.active_pane_id() else {
            return;
        };
        let Some(mode) = self.pane_mode(pane_id) else {
            return;
        };

        let event_type = match (event.state, event.repeat) {
            (ElementState::Released, _) => KeyEventType::Release,
            (ElementState::Pressed, true) => KeyEventType::Repeat,
            (ElementState::Pressed, false) => KeyEventType::Press,
        };

        let alternate_key =
            key_encoding::physical_key_to_us_codepoint(event.physical_key, &event.logical_key);
        let bytes = key_encoding::encode_key(&KeyInput {
            key: &event.logical_key,
            mods: self.modifiers.into(),
            mode,
            text: event.text.as_ref().map(SmolStr::as_str),
            location: event.location,
            event_type,
            alternate_key,
        });

        if !bytes.is_empty() {
            let redraw_after_input = self.redraw_after_pty_input(pane_id);
            // Only send scroll-to-bottom when actually scrolled up.
            // During key repeat at the live prompt, display_offset is 0
            // and this is a no-op — skip the IPC round-trip.
            let scrolled_up = self
                .mux
                .as_ref()
                .and_then(|mux| mux.pane_snapshot(pane_id))
                .is_some_and(|s| s.display_offset > 0);
            if scrolled_up {
                if let Some(mux) = self.mux.as_mut() {
                    mux.scroll_to_bottom(pane_id);
                }
            }
            self.write_pane_input(pane_id, &bytes);

            self.reset_cursor_blink();

            // Hide the mouse cursor while the user types.
            let hide_ctx = oriterm_ui::interaction::cursor_hide::HideContext {
                config_enabled: self.config.behavior.hide_mouse_when_typing,
                already_hidden: self.mouse_cursor_hidden,
                key: &event.logical_key,
                mouse_reporting: mode.intersects(oriterm_core::TermMode::ANY_MOUSE),
                ime_active: self.ime.should_suppress_key(),
            };
            if oriterm_ui::interaction::cursor_hide::should_hide_cursor(&hide_ctx) {
                self.mouse_cursor_hidden = true;
                if let Some(ctx) = self.focused_ctx() {
                    ctx.window.window().set_cursor_visible(false);
                }
            }

            if redraw_after_input {
                // Avoid a redundant pre-echo frame when typing at the live
                // prompt with a visible cursor. That frame can consume the
                // render budget and push the echoed glyph to the next tick.
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.root.mark_dirty();
                }
            }
        }
    }

    fn redraw_after_pty_input(&self, pane_id: oriterm_mux::PaneId) -> bool {
        let snapshot_display_offset = self
            .mux
            .as_ref()
            .and_then(|mux| mux.pane_snapshot(pane_id))
            .map(|snapshot| snapshot.display_offset);
        let snapshot_dirty = self
            .mux
            .as_ref()
            .is_some_and(|mux| mux.is_pane_snapshot_dirty(pane_id));
        should_redraw_after_pty_input(PtyInputRedrawState {
            cursor_hidden_by_blink: self.blinking_active && self.cursor_blink.intensity() < 0.01,
            snapshot_dirty,
            snapshot_display_offset,
        })
    }

    /// Scroll by one page in the given direction.
    fn execute_scroll(&mut self, up: bool) -> bool {
        if let Some(pane_id) = self.active_pane_id() {
            let lines = self
                .mux
                .as_ref()
                .and_then(|m| m.pane_snapshot(pane_id))
                .map_or(24, |s| s.cells.len() as isize);
            let delta = if up { lines } else { -lines };
            if let Some(mux) = self.mux.as_mut() {
                mux.scroll_display(pane_id, delta);
            }
        }
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.root.mark_dirty();
        }
        true
    }
}

/// Convert a winit logical key to an `oriterm_ui` [`Key`].
///
/// Returns `None` for keys that the UI framework doesn't handle.
fn winit_key_to_ui_key(key: &winit::keyboard::Key) -> Option<Key> {
    use winit::keyboard::{Key as WKey, NamedKey};
    match key {
        WKey::Named(NamedKey::Enter) => Some(Key::Enter),
        WKey::Named(NamedKey::Space) => Some(Key::Space),
        WKey::Named(NamedKey::Escape) => Some(Key::Escape),
        WKey::Named(NamedKey::Tab) => Some(Key::Tab),
        WKey::Named(NamedKey::Backspace) => Some(Key::Backspace),
        WKey::Named(NamedKey::Delete) => Some(Key::Delete),
        WKey::Named(NamedKey::Home) => Some(Key::Home),
        WKey::Named(NamedKey::End) => Some(Key::End),
        WKey::Named(NamedKey::ArrowUp) => Some(Key::ArrowUp),
        WKey::Named(NamedKey::ArrowDown) => Some(Key::ArrowDown),
        WKey::Named(NamedKey::ArrowLeft) => Some(Key::ArrowLeft),
        WKey::Named(NamedKey::ArrowRight) => Some(Key::ArrowRight),
        WKey::Named(NamedKey::PageUp) => Some(Key::PageUp),
        WKey::Named(NamedKey::PageDown) => Some(Key::PageDown),
        WKey::Character(s) => s.chars().next().map(Key::Character),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
