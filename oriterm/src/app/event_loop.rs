//! Winit `ApplicationHandler` impl — the event dispatch table.
//!
//! Separated from `mod.rs` to keep the main module definition file under the
//! 500-line limit. Helper functions (theme resolution, modifier conversion,
//! modal loop support) live in `event_loop_helpers.rs`.

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, DeviceEvents};

use super::App;
use super::event_loop_helpers::{ControlFlowDecision, ControlFlowInput, compute_control_flow};
use crate::event::TermEvent;
use crate::gpu::GpuState;

impl ApplicationHandler<TermEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.windows.is_empty() {
            return;
        }
        // Unregister raw input devices (WM_INPUT). winit defaults to
        // WhenFocused, which floods the message queue with mouse raw input
        // at 125-1000 Hz — stalling the render loop during flood output.
        // Terminals only need cooked WindowEvent variants.
        event_loop.listen_device_events(DeviceEvents::Never);
        if let Err(e) = self.try_init(event_loop) {
            log::error!("startup failed: {e}");
            event_loop.exit();
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "event dispatch table — inherently one arm per event variant"
    )]
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        // Dialog windows: handle separately from terminal windows.
        if self.dialogs.contains_key(&window_id) {
            self.handle_dialog_window_event(window_id, event);
            return;
        }

        // Modal blocking: when a modal dialog (e.g. Confirmation) is open,
        // its parent window ignores input events. Non-input events (resize,
        // redraw, scale change) still pass through.
        if self.window_manager.is_modal_blocked(window_id) {
            match &event {
                WindowEvent::Resized(_)
                | WindowEvent::RedrawRequested
                | WindowEvent::ScaleFactorChanged { .. }
                | WindowEvent::Focused(_)
                | WindowEvent::ThemeChanged(_) => {
                    // Allow these — they don't represent user input.
                }
                WindowEvent::MouseInput { .. } => {
                    // Clicking a blocked window brings the modal dialog to front.
                    if let Some(modal_id) = self.window_manager.find_modal_child(window_id) {
                        if let Some(ctx) = self.dialogs.get(&modal_id) {
                            ctx.window.focus_window();
                        }
                    }
                    return;
                }
                _ => return,
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                self.close_window(window_id, event_loop);
            }

            WindowEvent::Resized(size) => {
                self.handle_resize(window_id, size);
                // Defer render to about_to_wait() batching pipeline.
                // Synchronous redraw here causes per-event jitter.
            }

            WindowEvent::RedrawRequested => {
                // During Win32 modal move/resize loops, about_to_wait never
                // fires. A SetTimer ticks at 60 FPS, generating
                // RedrawRequested via InvalidateRect. Pump mux events and
                // render all windows here instead.
                #[cfg(target_os = "windows")]
                if oriterm_ui::platform_windows::in_modal_loop() {
                    self.modal_loop_render();
                    return;
                }
                self.handle_redraw();
            }

            WindowEvent::ModifiersChanged(mods) => {
                let prev_ctrl = self.modifiers.control_key();
                self.modifiers = mods.state();
                // Clear URL hover when Ctrl is released.
                if prev_ctrl && !mods.state().control_key() {
                    self.clear_url_hover();
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input(&event);
            }

            WindowEvent::Ime(ime) => self.handle_ime_event(ime),

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(ctx) = self.windows.get_mut(&window_id) {
                    if ctx.window.update_scale_factor(scale_factor) {
                        self.handle_dpi_change(window_id, scale_factor);
                        self.update_resize_increments(window_id);
                    }
                }
            }

            WindowEvent::Focused(focused) => {
                if focused {
                    // Flush any pending focus-out before processing focus-in.
                    // If a previous window lost focus to this one (not a child
                    // dialog), finalize the focus-out escape sequence now.
                    self.flush_pending_focus_out();

                    // Track which winit window is focused and update the
                    // mux active_window to match.
                    self.focused_window_id = Some(window_id);
                    self.window_manager.set_focused(Some(window_id));
                    if let Some(mux_id) = self
                        .windows
                        .get(&window_id)
                        .map(|ctx| ctx.window.session_window_id())
                    {
                        self.active_window = Some(mux_id);
                    }
                    // Re-evaluate blink from config + pane's terminal mode.
                    self.blinking_active = self.config.terminal.cursor_blink
                        && self
                            .terminal_mode()
                            .is_some_and(|m| m.contains(oriterm_core::TermMode::CURSOR_BLINKING));
                    self.send_focus_event(true);
                } else {
                    // Freeze cursor visible when window loses focus.
                    self.blinking_active = false;
                    // Commit any active tab title edit.
                    self.commit_tab_edit();
                    // Restore mouse cursor so it isn't stuck hidden in other apps.
                    self.restore_mouse_cursor(window_id);
                    // Transient popups should never survive window deactivation.
                    self.clear_window_popups(window_id);
                    // Defer focus-out: if focus is moving to a child dialog,
                    // the PTY focus-out escape should be suppressed.
                    self.pending_focus_out = Some(super::PendingFocusOut { window_id });
                }
                // Apply focus-dependent window transparency.
                let opacity = if focused {
                    self.config.window.effective_opacity()
                } else {
                    self.config.window.effective_unfocused_opacity()
                };
                if let Some(ctx) = self.windows.get(&window_id) {
                    ctx.window
                        .set_transparency(opacity, self.config.window.blur);
                }
                // Reset blink timer so cursor is visible immediately
                // (on focus-in: fresh start; on focus-out: frozen visible).
                self.reset_cursor_blink();
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.set_active(focused);
                    ctx.root.mark_dirty();
                }
            }

            WindowEvent::CursorLeft { .. } => {
                self.restore_mouse_cursor(window_id);
                self.clear_tab_bar_hover();
                self.clear_url_hover();
                self.clear_divider_hover();
                // On macOS, mouseDragged: continues after CursorLeft. Don't
                // cancel an active drag — it needs to reach tear-off threshold.
                #[cfg(not(target_os = "macos"))]
                self.cancel_tab_drag();
                #[cfg(target_os = "macos")]
                if !self.has_tab_drag() || !self.mouse.left_down() {
                    self.cancel_tab_drag();
                }
                self.cancel_divider_drag();
                self.cancel_floating_drag();
                self.release_tab_width_lock();
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.perf.record_cursor_move();
                self.restore_mouse_cursor(window_id);
                self.mouse.set_cursor_pos(position);

                // Overlays take priority: deliver move events for per-widget
                // hover tracking and consume the event when any overlay is
                // active. Without this, tab bar hover, divider hover, and
                // terminal mouse handlers interfere with menu interaction.
                if self.try_overlay_mouse_move(position) {
                    return;
                }

                self.update_tab_bar_hover(position);

                // Tab drag: consume all cursor moves when active.
                if self.update_tab_drag(position, event_loop) {
                    return;
                }

                // macOS/Linux: track torn-off window under cursor during drag.
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                if self.update_torn_off_drag() {
                    return;
                }

                // Floating pane hover/drag: check before divider and terminal.
                if self.update_floating_hover(position) {
                    // Only consume if a drag is active; hover just sets cursor.
                    if self
                        .focused_ctx()
                        .is_some_and(|ctx| ctx.floating_drag.is_some())
                    {
                        return;
                    }
                }

                // Divider hover/drag: check before terminal mouse handling.
                // Active drag consumes all moves.
                if self.update_divider_hover(position) {
                    return;
                }

                // Skip terminal mouse handling when the cursor is in the
                // chrome caption area. This avoids acquiring the terminal
                // lock on every cursor move over the title bar.
                if !self.cursor_in_tab_bar(position) {
                    if let Some(mode) = self.terminal_mode() {
                        if self.report_mouse_motion(position, mode) {
                            return;
                        }
                    }
                    if self.mouse.left_down() {
                        self.handle_mouse_drag(position);
                    }
                    // URL hover detection (Ctrl+move).
                    self.update_url_hover(position);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                // Track button state unconditionally — must run before any
                // early-return branch. Otherwise a press that reaches
                // handle_mouse_input but whose release is consumed by the tab
                // bar (or overlay/chrome) leaves buttons.left() stuck true,
                // causing phantom auto-scroll on subsequent CursorMoved.
                self.mouse
                    .set_button_down(button, state == winit::event::ElementState::Pressed);

                // Modal overlay: intercept mouse events.
                if self.try_overlay_mouse(button, state) {
                    return;
                }
                // Suppress stale WM_LBUTTONUP after live merge.
                if button == winit::event::MouseButton::Left
                    && state == winit::event::ElementState::Released
                {
                    let suppress = self
                        .focused_ctx_mut()
                        .and_then(|ctx| ctx.tab_drag.as_mut())
                        .is_some_and(|d| std::mem::replace(&mut d.suppress_next_release, false));
                    if suppress {
                        return;
                    }
                }
                // macOS/Linux: finish torn-off drag on mouse-up → check merge.
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                if button == winit::event::MouseButton::Left
                    && state == winit::event::ElementState::Released
                    && self.torn_off_pending.is_some()
                {
                    self.check_torn_off_merge();
                    return;
                }
                // Tab drag: finish on left-button release.
                if button == winit::event::MouseButton::Left
                    && state == winit::event::ElementState::Released
                    && self.try_finish_tab_drag()
                {
                    return;
                }
                // Tab bar clicks: switch tab, close tab, window controls, drag.
                if self.try_tab_bar_mouse(button, state, event_loop) {
                    return;
                }
                // Commit any active tab title edit when clicking the grid.
                if state == winit::event::ElementState::Pressed {
                    self.commit_tab_edit();
                }
                self.handle_mouse_input(button, state);
            }

            // Mouse wheel: overlays first, then report/alternate/viewport scroll.
            WindowEvent::MouseWheel { delta, .. } => {
                if self.try_overlay_scroll(delta) {
                    return;
                }
                if let Some(mode) = self.terminal_mode() {
                    self.handle_mouse_wheel(delta, mode);
                }
            }

            // File drag-and-drop: paste paths into terminal.
            WindowEvent::DroppedFile(path) => {
                self.paste_dropped_files(&[path]);
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.root.mark_dirty();
                }
            }

            WindowEvent::ThemeChanged(winit_theme) => {
                self.handle_theme_changed(winit_theme);
            }

            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: TermEvent) {
        match event {
            TermEvent::ConfigReload => {
                self.apply_config_reload();
            }
            TermEvent::MuxWakeup => {
                self.perf.record_wakeup();
            }
            TermEvent::CreateWindow => {
                self.create_window(event_loop);
            }
            TermEvent::MoveTabToNewWindow(tab_id) => {
                self.move_tab_to_new_window(tab_id, event_loop);
            }
            TermEvent::OpenSettings => {
                self.open_settings_dialog(event_loop);
            }
            TermEvent::OpenConfirmation(request) => {
                self.open_confirmation_dialog(event_loop, request);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.perf.record_tick();

        // Flush deferred focus-out. If focus left our app entirely (no
        // Focused(true) arrived), send the focus-out escape now.
        self.flush_pending_focus_out();

        // macOS: process fullscreen transition notifications.
        #[cfg(target_os = "macos")]
        self.process_fullscreen_events();

        // Windows: check for completed OS-level tab drag (merge on return
        // from blocking drag_window()). macOS checks on mouse-up instead.
        #[cfg(target_os = "windows")]
        self.check_torn_off_merge();

        // Windows: after a modal move/resize loop ends, force a full repaint.
        // During a pure move (no resize), the window is never marked dirty, so
        // the surface would show stale content until the next cursor blink.
        #[cfg(target_os = "windows")]
        if oriterm_ui::platform_windows::modal_loop_just_ended() {
            for ctx in self.windows.values_mut() {
                ctx.root.mark_dirty();
            }
        }

        // macOS/Linux: update torn-off window position every frame as a backup.
        // CursorMoved events may not always reach the source window.
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        self.update_torn_off_drag();

        // Pump mux events: drain PTY reader thread messages and process
        // resulting notifications before rendering.
        let pump_start = std::time::Instant::now();
        self.pump_mux_events();
        self.perf.last_pump_time = pump_start.elapsed();

        let blink_animating = self.drive_blink_timers();

        // Tick compositor animations and clean up fully-faded overlays.
        // Iterate all windows so unfocused windows with active animations
        // (e.g., a fade started just before a focus switch) continue to
        // progress rather than stalling.
        {
            let now = std::time::Instant::now();
            for ctx in self.windows.values_mut() {
                if !ctx.root.layer_animator().is_any_animating() {
                    continue;
                }
                let animating = ctx.root.tick_overlay_animations(now);

                // Clean up finished tab slide layers and sync offsets to widget.
                if ctx.tab_slide.has_active() {
                    let (tree, animator) = ctx.root.layer_tree_mut_and_animator();
                    ctx.tab_slide.cleanup(tree, animator);
                    let count = ctx.tab_bar.tab_count();
                    ctx.tab_slide
                        .sync_to_widget(count, ctx.root.layer_tree(), &mut ctx.tab_bar);
                }

                if animating {
                    ctx.root.mark_dirty();
                    ctx.ui_stale = true;
                }
            }
        }

        // Tick dialog overlay animations (dropdown fade-in/fade-out).
        self.tick_dialog_animations();

        // Lifecycle: show Primed dialogs (first frame committed) and
        // destroy Closing dialogs (deferred from close_dialog()).
        self.show_primed_dialogs();
        self.drain_pending_destroy();

        // Check if any window (terminal or dialog) is dirty and render it.
        let any_dirty = self.is_any_window_dirty();
        let now = std::time::Instant::now();
        let urgent_redraw = self.is_any_urgent_redraw();
        let budget_elapsed = now.duration_since(self.last_render) >= super::FRAME_BUDGET;

        // Render when dirty. PresentMode::Mailbox/Fifo provide hardware
        // pacing — render immediately to minimize input-to-display latency.
        // On Immediate mode (no hardware pacing), apply a client-side budget
        // gate to prevent uncapped redraws during sustained PTY output.
        let needs_budget = self.gpu.as_ref().is_some_and(GpuState::needs_frame_budget);
        if any_dirty && (!needs_budget || budget_elapsed || urgent_redraw || blink_animating) {
            self.render_dirty_windows();
        }

        // Periodic performance stats and idle detection.
        self.perf.check_idle();
        self.perf.maybe_log();

        // Decide ControlFlow via pure function (testable without winit).
        let still_dirty = self.is_any_window_dirty();
        let has_animations = self.has_active_animations();
        let remaining = super::FRAME_BUDGET.saturating_sub(now.duration_since(self.last_render));

        let input = ControlFlowInput {
            still_dirty,
            needs_budget,
            budget_elapsed,
            has_animations,
            blinking_active: self.blinking_active,
            next_blink_change: self.cursor_blink.next_change(),
            next_text_blink_change: self.text_blink.next_change(),
            budget_remaining: remaining,
            now,
            scheduler_wake: None,
        };
        match compute_control_flow(&input) {
            ControlFlowDecision::Wait => event_loop.set_control_flow(ControlFlow::Wait),
            ControlFlowDecision::WaitUntil(t) => {
                event_loop.set_control_flow(ControlFlow::WaitUntil(t));
            }
        }

        // Schedule a wakeup for the next blink state change via the
        // event proxy. WaitUntil doesn't reliably wake the event loop
        // on all platforms (observed on Windows/WSL2). The thread sleeps
        // until the next visual change (~16ms during fades, ~300ms during
        // plateaus) then sends MuxWakeup.
        self.schedule_blink_wakeup();
    }
}
