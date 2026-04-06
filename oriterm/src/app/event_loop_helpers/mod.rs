//! Helper functions and methods for the winit event loop.
//!
//! Extracted from `event_loop.rs` to keep the main event dispatch table under
//! the 500-line limit. Contains theme resolution, modifier conversion, and
//! platform-specific modal loop support.

use std::sync::atomic::Ordering;

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

        // Detect DPI/size changes that occurred inside the modal loop.
        //
        // WM_DPICHANGED triggers SetWindowPos (which generates WM_SIZE),
        // but winit does not dispatch WindowEvent::Resized during modal
        // loops — so handle_resize never runs. Query each window's actual
        // inner_size and compare against the stored size to detect changes.
        // This also picks up DPI changes because SetWindowPos resizes the
        // window to maintain logical size at the new DPI.
        {
            self.scratch_dirty_windows.clear();
            self.scratch_dirty_windows
                .extend(self.windows.keys().copied());
            for i in 0..self.scratch_dirty_windows.len() {
                let wid = self.scratch_dirty_windows[i];
                let needs_resize = self.windows.get(&wid).is_some_and(|ctx| {
                    let inner = ctx.window.window().inner_size();
                    let (sw, sh) = ctx.window.size_px();
                    inner.width != sw || inner.height != sh
                });
                if needs_resize {
                    let inner = self.windows[&wid].window.window().inner_size();
                    self.handle_resize(wid, inner);
                }
            }
        }

        // Detect size/DPI changes for dialog windows.
        // Same issue: winit does not dispatch Resized during modal loops.
        {
            self.scratch_dirty_windows.clear();
            self.scratch_dirty_windows
                .extend(self.dialogs.keys().copied());
            for i in 0..self.scratch_dirty_windows.len() {
                let wid = self.scratch_dirty_windows[i];
                let needs_resize = self.dialogs.get(&wid).is_some_and(|ctx| {
                    let inner = ctx.window.inner_size();
                    ctx.surface_config.width != inner.width
                        || ctx.surface_config.height != inner.height
                });
                if needs_resize {
                    // DPI detection — subclass proc consumed WM_DPICHANGED.
                    if let Some(ctx) = self.dialogs.get(&wid) {
                        if let Some(new_scale) =
                            oriterm_ui::platform_windows::get_current_dpi(ctx.window.as_ref())
                        {
                            let cur = ctx.scale_factor.factor();
                            if (new_scale - cur).abs() > 0.001 {
                                self.handle_dialog_dpi_change(wid, new_scale);
                            }
                        }
                    }
                    if let Some(gpu) = self.gpu.as_ref() {
                        let inner = self.dialogs[&wid].window.inner_size();
                        if let Some(ctx) = self.dialogs.get_mut(&wid) {
                            ctx.resize_surface(inner.width, inner.height, gpu);
                        }
                    }
                    self.refresh_dialog_platform_rects(wid);
                }
            }
        }

        // Check both terminal and dialog windows for dirty state.
        if !self.is_any_window_dirty() {
            return;
        }

        self.render_dirty_windows();
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
                    let hidden = self.config.window.tab_bar_position
                        == crate::config::TabBarPosition::Hidden;
                    let caption_h = if hidden {
                        0.0
                    } else {
                        ctx.tab_bar.metrics().height * scale
                    };
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
                ctx.root.mark_dirty();
            }
        }
        if events.did_exit() {
            // Safety-net centering after the animation completes. Usually
            // a no-op since we already centered during the resize above.
            if let Some(ctx) = self.focused_ctx() {
                let scale = ctx.window.scale_factor().factor() as f32;
                let hidden =
                    self.config.window.tab_bar_position == crate::config::TabBarPosition::Hidden;
                let caption_h = if hidden {
                    0.0
                } else {
                    ctx.tab_bar.metrics().height * scale
                };
                crate::window_manager::platform::macos::reapply_traffic_lights(
                    ctx.window.window(),
                    caption_h,
                );
            }
            if let Some(ctx) = self.focused_ctx_mut() {
                ctx.root.mark_dirty();
            }
        }
    }

    /// Reset the cursor blink cycle and invalidate any pending wakeup thread.
    ///
    /// Sets the generation counter to zero so `schedule_blink_wakeup()` can
    /// spawn a new thread with the correct delay. The old sleeper's CAS will
    /// fail because its generation no longer matches, preventing it from
    /// clearing the new thread's guard.
    pub(super) fn reset_cursor_blink(&mut self) {
        self.cursor_blink.reset();
        self.blink_wakeup_gen.store(0, Ordering::Release);
    }

    /// Schedule a delayed wakeup for the next blink state change.
    ///
    /// Uses `next_change()` to sleep until the next visual change — ~16ms
    /// during fade transitions, ~300ms during plateaus. Sends `MuxWakeup`
    /// to force the event loop to iterate, working around platforms where
    /// `ControlFlow::WaitUntil` doesn't reliably wake (Windows/WSL2).
    ///
    /// At most one pending wakeup thread at a time: a generation counter
    /// prevents stale threads from clearing the guard after a reset+respawn.
    pub(super) fn schedule_blink_wakeup(&mut self) {
        // A nonzero generation means a thread is already pending.
        if self.blink_wakeup_gen.load(Ordering::Acquire) != 0 {
            return;
        }

        let delay = if self.text_blink.is_animating()
            || (self.blinking_active && self.cursor_blink.is_animating())
        {
            std::time::Duration::from_millis(16)
        } else {
            // During plateau: wake at the next phase boundary.
            let now = std::time::Instant::now();
            let next = self.text_blink.next_change().min(if self.blinking_active {
                self.cursor_blink.next_change()
            } else {
                self.text_blink.next_change()
            });
            next.saturating_duration_since(now)
                .max(std::time::Duration::from_millis(1))
        };

        let wakeup_gen = self.next_blink_gen;
        self.next_blink_gen = wakeup_gen.wrapping_add(1).max(1);
        self.blink_wakeup_gen.store(wakeup_gen, Ordering::Release);
        let sender = self.event_proxy.clone();
        let gen_ref = self.blink_wakeup_gen.clone();
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            // Only send MuxWakeup if we're still the active generation.
            // If a reset or config reload spawned a newer thread, our CAS
            // fails and this thread exits silently — no stale wakeup.
            if gen_ref
                .compare_exchange(wakeup_gen, 0, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                sender.send(crate::event::TermEvent::MuxWakeup);
            }
        });
    }

    /// Drive cursor blink and text blink timers.
    ///
    /// Marks windows dirty and requests redraw when opacity changes OR
    /// when actively fading (`is_animating`) to ensure continuous redraws
    /// during fade transitions even when the per-frame delta is below
    /// the `update()` threshold.
    ///
    /// Returns `true` if any blink animation is active (fade transition
    /// in progress). The caller uses this to bypass the frame budget gate
    /// which would otherwise block animation redraws.
    pub(super) fn drive_blink_timers(&mut self) -> bool {
        let mut animating = false;

        if self.blinking_active && (self.cursor_blink.update() || self.cursor_blink.is_animating())
        {
            animating = true;
            if let Some(ctx) = self.focused_ctx_mut() {
                ctx.root.mark_dirty();
                ctx.window.window().request_redraw();
            }
        }

        if self.text_blink.update() || self.text_blink.is_animating() {
            animating = true;
            for ctx in self.windows.values_mut() {
                ctx.root.mark_dirty();
                ctx.window.window().request_redraw();
            }
        }

        animating
    }

    /// Tick overlay animations in dialog windows.
    ///
    /// Drives dropdown fade-in/fade-out transitions and cleans up
    /// fully-dismissed overlays. Called from `about_to_wait`.
    pub(super) fn tick_dialog_animations(&mut self) {
        if self.dialogs.is_empty() {
            return;
        }
        let now = std::time::Instant::now();
        for ctx in self.dialogs.values_mut() {
            if ctx.root.tick_overlay_animations(now) {
                ctx.root.mark_dirty();
            }
        }
    }

    /// Whether cursor blinking is enabled: config allows it AND the terminal
    /// has set the `CURSOR_BLINKING` mode via DECSCUSR.
    pub(super) fn cursor_should_blink(&self, terminal_blinking: bool) -> bool {
        self.config.terminal.cursor_blink && terminal_blinking
    }

    /// Apply the current UI theme to all window chrome widgets and invalidate caches.
    ///
    /// This is the canonical theme-application path. All sites that change
    /// `self.ui_theme` must call this afterwards instead of manually applying.
    pub(super) fn apply_theme_to_chrome(&mut self) {
        for ctx in self.windows.values_mut() {
            ctx.tab_bar.apply_theme(&self.ui_theme);
            ctx.status_bar.apply_theme(&self.ui_theme);
            ctx.pane_cache.invalidate_all();
            ctx.text_cache.clear();
            ctx.root.invalidation_mut().invalidate_all();
            ctx.root.damage_mut().reset();
            ctx.root.mark_dirty();
            ctx.ui_stale = true;
        }
    }
}

/// Inputs for the control flow decision (no winit types).
#[allow(
    clippy::struct_excessive_bools,
    reason = "mirrors event loop state flags"
)]
pub(super) struct ControlFlowInput {
    /// Whether any window still has dirty flag after rendering.
    pub still_dirty: bool,
    /// Whether the frame budget has elapsed since last render.
    pub budget_elapsed: bool,
    /// Whether compositor animations are running.
    pub has_animations: bool,
    /// Whether cursor blink is active.
    pub blinking_active: bool,
    /// Next cursor blink change time (only meaningful if `blinking_active`).
    ///
    /// During fade transitions this is ~16ms (animation frame rate); during
    /// plateaus it is ~530ms (phase boundary).
    pub next_blink_change: std::time::Instant,
    /// Next text blink phase boundary (always active — unconditional timer).
    ///
    /// Any cell could have the BLINK flag; scanning cells each frame is too
    /// expensive, so the timer runs unconditionally (~2 wakeups/sec).
    pub next_text_blink_change: std::time::Instant,
    /// Current time.
    pub now: std::time::Instant,
    /// Earliest deferred repaint from `RenderScheduler`.
    ///
    /// Feeds into `WaitUntil` when no animations or dirty state is active.
    /// `None` when the scheduler has no deferred repaints.
    pub scheduler_wake: Option<std::time::Instant>,
}

/// Result of the control flow decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ControlFlowDecision {
    /// Sleep until an external event arrives.
    #[allow(
        dead_code,
        reason = "reserved for future use when all timers can be disabled"
    )]
    Wait,
    /// Sleep until the given instant.
    WaitUntil(std::time::Instant),
}

/// Pure function: decide the next `ControlFlow` from event loop state.
///
/// No winit types — testable without a display server. Mirrors the
/// decision tree in `about_to_wait`.
pub(super) fn compute_control_flow(input: &ControlFlowInput) -> ControlFlowDecision {
    // Still dirty after render attempt — sleep until the next event
    // if the frame budget hasn't elapsed yet. Using `Wait` instead of
    // `WaitUntil` because WaitUntil doesn't reliably sleep on
    // Windows/WSL2 (observed: returns immediately, creating a tight
    // loop that starves keyboard dispatch — BUG-11-1). With `Wait`,
    // the coalesced MuxWakeup from the next PTY snapshot wakes us,
    // and winit's PeekMessage drains ALL pending messages (including
    // keyboard events) before calling about_to_wait().
    if input.still_dirty {
        if !input.budget_elapsed {
            return ControlFlowDecision::Wait;
        }
        return ControlFlowDecision::WaitUntil(input.now);
    }
    if input.has_animations {
        ControlFlowDecision::WaitUntil(input.now + std::time::Duration::from_millis(16))
    } else {
        // Text blink timer always contributes (any cell could have BLINK flag;
        // scanning cells each frame is too expensive, so the timer runs
        // unconditionally at ~2 wakeups/sec — negligible cost).
        let mut wake_at = input.next_text_blink_change;
        if input.blinking_active {
            wake_at = wake_at.min(input.next_blink_change);
        }
        match input.scheduler_wake {
            Some(wake) => ControlFlowDecision::WaitUntil(wake.min(wake_at)),
            None => ControlFlowDecision::WaitUntil(wake_at),
        }
    }
}

#[cfg(test)]
mod tests;
