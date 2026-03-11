//! Winit `ApplicationHandler` impl — the event dispatch table.
//!
//! Separated from `mod.rs` to keep the main module definition file under the
//! 500-line limit. Helper functions (theme resolution, modifier conversion,
//! modal loop support) live in `event_loop_helpers.rs`.

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, DeviceEvents};

use super::App;
use crate::event::TermEvent;

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
                    // Restore mouse cursor so it isn't stuck hidden in other apps.
                    self.restore_mouse_cursor(window_id);
                    // Defer focus-out: if focus is moving to a child dialog,
                    // the PTY focus-out escape should be suppressed.
                    self.pending_focus_out = Some(super::PendingFocusOut { window_id });
                }
                // Reset blink timer so cursor is visible immediately
                // (on focus-in: fresh start; on focus-out: frozen visible).
                self.cursor_blink.reset();
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.set_active(focused);
                    ctx.dirty = true;
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
                    ctx.dirty = true;
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
                // The real work happens in `pump_mux_events()` during
                // `about_to_wait`. This wakeup ensures the event loop
                // doesn't sleep past pending mux events. Mark ALL windows
                // dirty — PTY output may come from any pane in any window.
                self.mark_all_windows_dirty();
            }
            TermEvent::CreateWindow => {
                self.create_window(event_loop);
            }
            TermEvent::MoveTabToNewWindow(tab_index) => {
                let tab_id = self.active_window.and_then(|wid| {
                    let win = self.session.get_window(wid)?;
                    win.tabs().get(tab_index).copied()
                });
                if let Some(tid) = tab_id {
                    self.move_tab_to_new_window(tid, event_loop);
                }
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

        // macOS/Linux: update torn-off window position every frame as a backup.
        // CursorMoved events may not always reach the source window.
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        self.update_torn_off_drag();

        // Pump mux events: drain PTY reader thread messages and process
        // resulting notifications before rendering.
        self.pump_mux_events();

        // Drive cursor blink timer only when blinking is active.
        if self.blinking_active && self.cursor_blink.update() {
            if let Some(ctx) = self.focused_ctx_mut() {
                ctx.dirty = true;
            }
        }

        // Tick compositor animations and clean up fully-faded overlays.
        let any_animating = {
            let now = std::time::Instant::now();
            if let Some(ctx) = self.focused_ctx_mut() {
                let animating = ctx.layer_animator.tick(&mut ctx.layer_tree, now);
                ctx.overlays
                    .cleanup_dismissed(&mut ctx.layer_tree, &ctx.layer_animator);

                // Clean up finished tab slide layers and sync offsets to widget.
                if ctx.tab_slide.has_active() {
                    ctx.tab_slide
                        .cleanup(&mut ctx.layer_tree, &ctx.layer_animator);
                    let count = ctx.tab_bar.tab_count();
                    ctx.tab_slide
                        .sync_to_widget(count, &ctx.layer_tree, &mut ctx.tab_bar);
                }

                animating
            } else {
                false
            }
        };
        if any_animating {
            if let Some(ctx) = self.focused_ctx_mut() {
                ctx.dirty = true;
            }
        }

        // Tick dialog overlay animations (dropdown fade-in/fade-out).
        self.tick_dialog_animations();

        // Check if any window (terminal or dialog) is dirty and render it.
        let any_dirty = self.windows.values().any(|ctx| ctx.dirty)
            || self.dialogs.values().any(|ctx| ctx.dirty);
        let now = std::time::Instant::now();
        let budget_elapsed = now.duration_since(self.last_render) >= super::FRAME_BUDGET;

        if any_dirty && budget_elapsed {
            self.render_dirty_windows();
        }

        // Periodic performance stats.
        self.perf.maybe_log();

        // Schedule wakeup for continuous rendering when animations are
        // active or for the next blink toggle. The default ControlFlow::Wait
        // lets the event loop sleep indefinitely when nothing is animating.
        //
        // Re-check: widget animations may set dirty during draw.
        let still_dirty =
            self.windows.values().any(|c| c.dirty) || self.dialogs.values().any(|c| c.dirty);
        let has_animations = self
            .windows
            .values()
            .any(|c| c.layer_animator.is_any_animating())
            || self
                .dialogs
                .values()
                .any(|c| c.layer_animator.is_any_animating());
        if (any_dirty && !budget_elapsed) || still_dirty {
            // Dirty but budget not yet elapsed, or re-dirtied by widget
            // animations during render — wake up when budget allows.
            let remaining =
                super::FRAME_BUDGET.saturating_sub(now.duration_since(self.last_render));
            event_loop.set_control_flow(ControlFlow::WaitUntil(now + remaining));
        } else if has_animations {
            // Compositor animations: wake up promptly to drive the next frame.
            // 16ms ≈ 60 FPS — smooth enough for fade transitions.
            let next_frame = now + std::time::Duration::from_millis(16);
            event_loop.set_control_flow(ControlFlow::WaitUntil(next_frame));
        } else if self.blinking_active {
            let next_toggle = self.cursor_blink.next_toggle();
            event_loop.set_control_flow(ControlFlow::WaitUntil(next_toggle));
        } else {
            // Nothing animating — sleep until the next external event.
        }
    }
}

impl App {
    /// Render all dirty terminal and dialog windows.
    ///
    /// Temporarily swaps `focused_window_id`/`active_window` to target each
    /// dirty window, then restores the original focus.
    fn render_dirty_windows(&mut self) {
        let dirty_winit_ids: Vec<winit::window::WindowId> = self
            .windows
            .iter()
            .filter(|(_, ctx)| ctx.dirty)
            .map(|(&id, _)| id)
            .collect();

        let saved_focused = self.focused_window_id;
        let saved_active = self.active_window;

        for wid in dirty_winit_ids {
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

        // Render dirty dialog windows.
        let dirty_dialog_ids: Vec<winit::window::WindowId> = self
            .dialogs
            .iter()
            .filter(|(_, ctx)| ctx.dirty)
            .map(|(&id, _)| id)
            .collect();
        for wid in dirty_dialog_ids {
            if let Some(ctx) = self.dialogs.get_mut(&wid) {
                ctx.dirty = false;
            }
            self.render_dialog(wid);
        }

        self.last_render = std::time::Instant::now();
        self.perf.record_render();
    }
}
