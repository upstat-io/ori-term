//! Helper functions and methods for the winit event loop.
//!
//! Extracted from `event_loop.rs` to keep the main event dispatch table under
//! the 500-line limit. Contains theme resolution, modifier conversion, and
//! platform-specific modal loop support.

use winit::keyboard::ModifiersState;

use oriterm_core::Theme;
use oriterm_ui::theme::UiTheme;

use super::App;
use crate::config::Config;

/// Resolve the [`UiTheme`] from config override + system theme.
///
/// Maps [`ThemeOverride`] → [`UiTheme`]: `Dark` → `dark()`, `Light` → `light()`,
/// `Auto` → delegates to the provided system theme (falls back to dark on `Unknown`).
pub(super) fn resolve_ui_theme_with(config: &Config, system: Theme) -> UiTheme {
    use crate::config::ThemeOverride;

    match config.colors.theme {
        ThemeOverride::Dark => UiTheme::dark(),
        ThemeOverride::Light => UiTheme::light(),
        ThemeOverride::Auto => match system {
            Theme::Light => UiTheme::light(),
            _ => UiTheme::dark(),
        },
    }
}

/// Resolve the [`UiTheme`] at startup by detecting the system theme.
pub(super) fn resolve_ui_theme(config: &Config) -> UiTheme {
    resolve_ui_theme_with(config, crate::platform::theme::system_theme())
}

/// Convert winit modifier state to `oriterm_ui` modifier bitmask.
pub(super) fn winit_mods_to_ui(state: ModifiersState) -> oriterm_ui::input::Modifiers {
    let mut m = oriterm_ui::input::Modifiers::NONE;
    if state.shift_key() {
        m = m.union(oriterm_ui::input::Modifiers::SHIFT_ONLY);
    }
    if state.control_key() {
        m = m.union(oriterm_ui::input::Modifiers::CTRL_ONLY);
    }
    if state.alt_key() {
        m = m.union(oriterm_ui::input::Modifiers::ALT_ONLY);
    }
    if state.super_key() {
        m = m.union(oriterm_ui::input::Modifiers::LOGO_ONLY);
    }
    m
}

impl App {
    /// Pump mux events and render all dirty windows during a Win32 modal loop.
    ///
    /// During modal move/resize, `about_to_wait` never fires. A `SetTimer`
    /// in the `WndProc` ticks at 60 FPS, generating `RedrawRequested` via
    /// `InvalidateRect`. This method substitutes for `about_to_wait`'s
    /// render loop: pump mux events, then render every dirty window using
    /// the same focus-swapping pattern.
    #[cfg(target_os = "windows")]
    pub(super) fn modal_loop_render(&mut self) {
        self.pump_mux_events();

        let dirty_ids: Vec<winit::window::WindowId> = self
            .windows
            .iter()
            .filter(|(_, ctx)| ctx.dirty)
            .map(|(&id, _)| id)
            .collect();
        if dirty_ids.is_empty() {
            return;
        }

        let saved_focused = self.focused_window_id;
        let saved_active = self.active_window;

        for wid in dirty_ids {
            if let Some(ctx) = self.windows.get_mut(&wid) {
                ctx.dirty = false;
            }
            let mux_wid = self
                .windows
                .get(&wid)
                .map(|ctx| ctx.window.session_window_id());
            self.focused_window_id = Some(wid);
            self.active_window = mux_wid;
            self.handle_redraw();
        }

        self.focused_window_id = saved_focused;
        self.active_window = saved_active;
        self.last_render = std::time::Instant::now();
    }

    /// Send a focus-in or focus-out escape sequence to the active pane.
    ///
    /// Only sends when the terminal has `FOCUS_IN_OUT` mode enabled (mode 1004).
    /// Focus-in: `CSI I` (`\x1b[I`), focus-out: `CSI O` (`\x1b[O`).
    pub(super) fn send_focus_event(&mut self, focused: bool) {
        let Some(pane_id) = self.active_pane_id() else {
            return;
        };
        let Some(mode) = self.pane_mode(pane_id) else {
            return;
        };
        if !mode.contains(oriterm_core::TermMode::FOCUS_IN_OUT) {
            return;
        }
        let seq: &[u8] = if focused { b"\x1b[I" } else { b"\x1b[O" };
        self.write_pane_input(pane_id, seq);
    }

    /// Flush a pending focus-out event.
    ///
    /// Called from `about_to_wait` and from `Focused(true)` handlers. Checks
    /// if focus moved to a child dialog — if so, the focus-out is suppressed
    /// because the terminal is still "active" from the user's perspective.
    pub(super) fn flush_pending_focus_out(&mut self) {
        let Some(pending) = self.pending_focus_out.take() else {
            return;
        };
        // If focus moved to a child dialog of the window that lost focus,
        // suppress the PTY focus-out escape sequence.
        if self.window_manager.focused_is_child_of(pending.window_id) {
            return;
        }
        self.send_focus_event(false);
    }

    /// Process pending macOS fullscreen transition events.
    ///
    /// Consumes atomic flags set by `NSNotificationCenter` observers and
    /// applies tab bar inset changes + traffic light re-centering.
    ///
    /// Called from `about_to_wait` AND `handle_resize`. During fullscreen
    /// transitions, macOS fires resize events before capturing the "to"
    /// state for the animation. Processing flags in `handle_resize` ensures
    /// the tab bar layout is correct in the animation snapshot, avoiding
    /// a visible pop after the animation completes.
    #[cfg(target_os = "macos")]
    pub(super) fn process_fullscreen_events(&mut self) {
        let Some(events) = crate::window_manager::platform::macos::take_fullscreen_events() else {
            return;
        };
        if events.will_exit() || events.will_enter() {
            // For exit: center traffic lights NOW, before macOS captures
            // the "to" snapshot for the animation. This ensures the buttons
            // are at their correct centered positions in the exit animation
            // target frame, preventing the visible "bump down" artifact.
            if events.will_exit() {
                if let Some(ctx) = self.focused_ctx() {
                    let scale = ctx.window.scale_factor().factor() as f32;
                    let caption_h = oriterm_ui::widgets::tab_bar::constants::TAB_BAR_HEIGHT * scale;
                    crate::window_manager::platform::macos::reapply_traffic_lights(
                        ctx.window.window(),
                        caption_h,
                    );
                }
            }
            if let Some(ctx) = self.focused_ctx_mut() {
                let inset = if events.will_enter() {
                    0.0
                } else {
                    oriterm_ui::widgets::tab_bar::constants::MACOS_TRAFFIC_LIGHT_WIDTH
                };
                ctx.tab_bar.set_left_inset(inset);
                ctx.dirty = true;
            }
        }
        if events.did_exit() {
            // Safety-net centering after the animation completes. Usually
            // a no-op since we already centered during the resize above.
            if let Some(ctx) = self.focused_ctx() {
                let scale = ctx.window.scale_factor().factor() as f32;
                let caption_h = oriterm_ui::widgets::tab_bar::constants::TAB_BAR_HEIGHT * scale;
                crate::window_manager::platform::macos::reapply_traffic_lights(
                    ctx.window.window(),
                    caption_h,
                );
            }
            if let Some(ctx) = self.focused_ctx_mut() {
                ctx.dirty = true;
            }
        }
    }

    /// Tick overlay animations in dialog windows.
    ///
    /// Drives dropdown fade-in/fade-out transitions and cleans up
    /// fully-dismissed overlays. Called from `about_to_wait`.
    pub(super) fn tick_dialog_animations(&mut self) {
        let now = std::time::Instant::now();
        for ctx in self.dialogs.values_mut() {
            if ctx.layer_animator.is_any_animating() {
                let animating = ctx.layer_animator.tick(&mut ctx.layer_tree, now);
                ctx.overlays
                    .cleanup_dismissed(&mut ctx.layer_tree, &ctx.layer_animator);
                if animating {
                    ctx.dirty = true;
                }
            }
        }
    }
}
