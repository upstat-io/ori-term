//! VTE handler implementation for `Term<S>`.
//!
//! Implements `vte::ansi::Handler` to process escape sequences, control
//! characters, and printable input. Each method delegates to the
//! appropriate grid/cursor/mode operation.

use vte::ansi::{
    Attr, CharsetIndex, ClearMode, CursorStyle, Handler, Hyperlink as VteHyperlink, KeyboardModes,
    KeyboardModesApplyBehavior, LineClearMode, Mode, ModifyOtherKeys, PrivateMode, Rgb,
    StandardCharset, TabulationClearMode,
};

use crate::effect::sink::EffectSink;
use crate::effect::{Effect, PtyEffect, PtyWriteKind};
use crate::encode::mouse::{
    MouseEvent, button_mask_for_event, encode_mouse_event, should_handle_mouse_input,
};
use crate::term::dec_locator::LocatorPosition;

use self::rect_ops::DecRect;
use super::{Term, TermMode};

mod control;
mod dcs;
mod esc;
mod helpers;
pub(in crate::term) mod image;
mod modes;
mod osc;
mod presentation;
mod rect_ops;
mod sgr;
mod status;
mod xtgettcap;

/// Generate a one-line `Handler` trait method that delegates to an
/// inherent helper on `Term<S>`. Used by the OSC 3/5/6/… block, the
/// iTerm2 / sixel / APC / DECRQSS delegates, and the §09A.4 DEC
/// private rect-ops + presentation delegates. Keeps the trait impl
/// under the 500-line file budget.
macro_rules! delegate_osc {
    ($method:ident($($arg:ident : $ty:ty),*) => $helper:ident) => {
        fn $method(&mut self, $($arg: $ty),*) { self.$helper($($arg),*); }
    };
}

impl<S: EffectSink> Term<S> {
    /// Compose the DECLRP reply per xterm spec (`CSI Pe ; Pm ; Pr ; Pc ; Pp & w`).
    ///
    /// Reads the last-observed locator position from
    /// `self.dec_locator.position()` (written by `handle_mouse_input`
    /// Step A while reporting is active). Emits the locator-unavailable
    /// reply `CSI 0 & w` (ONE parameter, `a_nparam = 1` per xterm
    /// `button.c:857-861`) when reporting is disabled OR no position has
    /// been observed. When a position is known, emits Pe=1 ("request
    /// response"), the Pm button mask, the 1-indexed Pr/Pc coords, and
    /// Pp=1 (`ori_term` has no page memory). The coordinate unit follows
    /// DECELR Pu — character cells when Pu=0, DEVICE physical pixels when
    /// Pu=1.
    fn compose_declrp_reply(&self) -> Vec<u8> {
        match (self.dec_locator.reporting(), self.dec_locator.position()) {
            (None, _) | (_, LocatorPosition::Unavailable) => {
                // Pe=0 locator-unavailable — one parameter per button.c:857-861.
                b"\x1b[0&w".to_vec()
            }
            (
                Some(_),
                LocatorPosition::Known {
                    cell,
                    pixel,
                    buttons,
                },
            ) => {
                let (pc, pr): (u32, u32) = if self.dec_locator.pixel_unit() {
                    (pixel.0 + 1, pixel.1 + 1) // Pu=1 — device pixels, 1-indexed
                } else {
                    (cell.0 + 1, cell.1 + 1) // Pu=0 — cells, 1-indexed
                };
                // Wire order: Pe ; Pm ; Pr(row) ; Pc(col) ; Pp.
                format!("\x1b[1;{buttons};{pr};{pc};1&w").into_bytes()
            }
        }
    }

    /// Handle a semantic mouse input from the UI layer.
    ///
    /// Reads `TermMode`, encodes per protocol selection (SGR / URXVT /
    /// UTF-8 / Normal / X10), and emits `Effect::Pty(PtyEffect::Write
    /// { kind: PtyWriteKind::MouseEvent, bytes })`. App's effect-drain
    /// loop carries the emission to the PTY.
    ///
    /// No-op when (a) the encoder returns an empty buffer (X10 release,
    /// coordinate overflow) OR (b) no mouse-encoding mode is active —
    /// avoids pushing an empty `PtyEffect::Write` that downstream
    /// observers would have to filter.
    ///
    /// Decision 10 Option A apex per `plans/spec-conformance/decisions/10-mouse-verification-apex-effect-vs-app-capture.md`
    /// and §16.2.0.
    /// Handle a semantic focus change from the UI layer (winit
    /// `WindowEvent::Focused(focused)`).
    ///
    /// When `TermMode::FOCUS_IN_OUT` (DECSET 1004) is active, emits
    /// `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::FocusEvent,
    /// bytes: b"\x1b[I" | b"\x1b[O" })` via `effect_sink`. App's
    /// effect-drain loop carries the bytes to the PTY exactly as
    /// §16.1.C's DECLRP + §16.2.0.B's mouse-event paths do.
    ///
    /// Defense-in-depth: silent no-op when 1004 not enabled. App-side
    /// `should_emit_focus` predicate already gates; double-gating
    /// here keeps the apex safe.
    ///
    /// Decision 10 Option A apex per `plans/spec-conformance/decisions/10-mouse-verification-apex-effect-vs-app-capture.md`
    /// + §16.7.
    pub fn handle_focus_event(&self, focused: bool) {
        if !self.mode.contains(TermMode::FOCUS_IN_OUT) {
            return;
        }
        let bytes: &[u8] = if focused { b"\x1b[I" } else { b"\x1b[O" };
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: bytes.to_vec(),
            kind: PtyWriteKind::FocusEvent,
        }));
    }

    /// Observe a mouse event for DEC Locator position tracking — Step A
    /// ONLY, never encodes a mouse report.
    ///
    /// Records the position when locator reporting is active (DECELR
    /// Ps=1/Ps=2), INDEPENDENT of the mouse-tracking encoder gate: DEC
    /// Locator (DECELR-activated) and mouse tracking (DECSET 1000/1002/1003)
    /// are orthogonal subsystems per xterm. A subsequent DECRQLP reads this
    /// state. Out-of-grid cursor (`!event.in_grid`) → `Unavailable` so
    /// DECRQLP emits Pe=0 per xterm button.c:857-861.
    ///
    /// The App dispatches HERE (not `handle_mouse_input`) for locator-only
    /// observation — on the shift-bypass / no-tracking path — so the
    /// encoder (Step B) cannot fire. `handle_mouse_input` calls this for
    /// the combined observe+encode path. Continuous/OneShot DECLRP
    /// push-emission on DECSLE-filtered events is a separate deliverable.
    /// See: bug-tracker/section-08-core-terminal.md (BUG-08-063) — DECSLE
    /// push-emission path.
    pub fn observe_locator_input(&mut self, event: &MouseEvent) {
        if self.dec_locator.reporting().is_some() {
            // `in_grid` is the unclamped grid-membership flag (App sets it
            // from `mouse_cell()`, distinct from the clamped col/line the
            // encoder consumes). Out-of-grid → `cell` None → `Unavailable`.
            // `physical_px` stays `Option`: `observe` stores `Unavailable`
            // for Pu=1-without-pixels rather than fabricating `(0, 0)`.
            let cell = event
                .in_grid
                .then_some((event.col as u32, event.line as u32));
            let buttons = button_mask_for_event(self.dec_locator.buttons(), event);
            self.dec_locator.observe(cell, event.physical_px, buttons);
        }
    }

    pub fn handle_mouse_input(&mut self, event: &MouseEvent) {
        // STEP A — DEC Locator observation (shared with the App's
        // observe-only `observe_locator_input` entry point).
        self.observe_locator_input(event);

        // STEP B — mouse-tracking encoder emission. Gated on the
        // `should_handle_mouse_input` SSOT predicate (NOT raw
        // `intersects` — keeps the gate single-sourced with the
        // daemon-default backend). The App layer's `should_report_mouse`
        // predicate also gates before dispatching; double-gating here
        // keeps the apex contract safe against callers that skip it.
        if !should_handle_mouse_input(self.mode) {
            return;
        }
        let report = encode_mouse_event(event, self.mode);
        let bytes = report.as_bytes();
        if bytes.is_empty() {
            return;
        }
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: bytes.to_vec(),
            kind: PtyWriteKind::MouseEvent,
        }));
    }
}

impl<S: EffectSink> Handler for Term<S> {
    #[inline]
    fn input(&mut self, c: char) {
        self.input_char(c);
    }

    fn backspace(&mut self) {
        if self.mode.contains(TermMode::REVERSE_WRAP) && self.try_reverse_wrap() {
            return;
        }
        self.grid_mut().backspace();
    }

    fn put_tab(&mut self, count: u16) {
        let grid = self.grid_mut();
        for _ in 0..count {
            grid.tab();
        }
    }

    #[inline]
    fn linefeed(&mut self) {
        self.linefeed_impl();
    }

    #[inline]
    fn carriage_return(&mut self) {
        self.grid_mut().carriage_return();
    }

    #[inline]
    fn bell(&mut self) {
        self.bell_impl();
    }

    fn substitute(&mut self) {
        self.input(' ');
    }

    #[inline]
    fn set_active_charset(&mut self, index: CharsetIndex) {
        self.charset.set_active(index);
    }

    #[inline]
    fn configure_charset(&mut self, index: CharsetIndex, charset: StandardCharset) {
        self.charset.set_charset(index, charset);
    }

    #[inline]
    fn set_single_shift(&mut self, index: CharsetIndex) {
        self.charset.set_single_shift(index);
    }

    fn goto(&mut self, line: i32, col: usize) {
        self.goto_origin_aware(line, col);
    }

    fn goto_line(&mut self, line: i32) {
        let col = self.grid().cursor().col().0;
        self.goto_origin_aware(line, col);
    }

    fn goto_col(&mut self, col: usize) {
        let target = self.origin_aware_col(col);
        self.grid_mut().move_to_column(target);
    }
    fn move_up(&mut self, count: usize) {
        self.grid_mut().move_up(count);
    }
    fn move_down(&mut self, count: usize) {
        self.grid_mut().move_down(count);
    }
    fn move_forward(&mut self, col: usize) {
        self.grid_mut().move_forward(col);
    }
    fn move_backward(&mut self, col: usize) {
        self.grid_mut().move_backward(col);
    }

    fn move_down_and_cr(&mut self, count: usize) {
        let grid = self.grid_mut();
        grid.move_down(count);
        grid.carriage_return();
    }

    fn move_up_and_cr(&mut self, count: usize) {
        let grid = self.grid_mut();
        grid.move_up(count);
        grid.carriage_return();
    }

    fn clear_screen(&mut self, mode: ClearMode) {
        self.clear_screen_impl(&mode);
    }

    fn clear_line(&mut self, mode: LineClearMode) {
        self.clear_line_impl(&mode);
    }

    fn erase_chars(&mut self, count: usize) {
        self.selection_dirty = true;
        self.clear_images_after_ech(count);
        self.grid_mut().erase_chars(count);
    }

    fn insert_blank(&mut self, count: usize) {
        self.selection_dirty = true;
        self.grid_mut().insert_blank(count);
    }

    fn delete_chars(&mut self, count: usize) {
        self.selection_dirty = true;
        self.grid_mut().delete_chars(count);
    }

    fn insert_blank_lines(&mut self, count: usize) {
        self.selection_dirty = true;
        self.grid_mut().insert_lines(count);
    }

    fn delete_lines(&mut self, count: usize) {
        self.selection_dirty = true;
        self.grid_mut().delete_lines(count);
    }

    fn scroll_up(&mut self, count: usize) {
        self.scroll_up_impl(count);
    }

    fn scroll_down(&mut self, count: usize) {
        self.selection_dirty = true;
        self.grid_mut().scroll_down(count);
    }

    fn scroll_left(&mut self, count: usize) {
        self.selection_dirty = true;
        self.grid_mut().scroll_left(count);
    }

    fn scroll_right(&mut self, count: usize) {
        self.selection_dirty = true;
        self.grid_mut().scroll_right(count);
    }

    fn reverse_index(&mut self) {
        self.selection_dirty = true;
        self.grid_mut().reverse_index();
    }

    fn newline(&mut self) {
        self.newline_impl();
    }

    fn move_forward_tabs(&mut self, count: u16) {
        self.put_tab(count);
    }

    fn move_backward_tabs(&mut self, count: u16) {
        let grid = self.grid_mut();
        for _ in 0..count {
            grid.tab_backward();
        }
    }

    fn set_horizontal_tabstop(&mut self) {
        self.grid_mut().set_tab_stop();
    }

    fn clear_tabs(&mut self, mode: TabulationClearMode) {
        self.clear_tabs_impl(&mode);
    }

    fn set_scrolling_region(&mut self, top: usize, bottom: Option<usize>) {
        self.grid_mut().set_scroll_region(top, bottom);
        self.goto_origin_aware(0, 0);
    }

    fn decaln(&mut self) {
        self.selection_dirty = true;
        self.decaln_impl();
    }

    fn save_cursor_position(&mut self) {
        self.save_cursor_impl();
    }

    fn decslrm_or_save_cursor(&mut self, has_params: bool, left: u16, right: u16) {
        self.decslrm_or_save_cursor_impl(has_params, left, right);
    }

    fn restore_cursor_position(&mut self) {
        self.restore_cursor_impl();
    }

    fn set_mode(&mut self, mode: Mode) {
        self.set_named_mode_dispatch(mode);
    }

    fn unset_mode(&mut self, mode: Mode) {
        self.unset_named_mode_dispatch(mode);
    }

    fn set_private_mode(&mut self, mode: PrivateMode) {
        self.set_private_mode_dispatch(mode);
    }

    fn unset_private_mode(&mut self, mode: PrivateMode) {
        self.unset_private_mode_dispatch(mode);
    }

    fn report_mode(&mut self, mode: Mode) {
        self.status_report_mode(mode);
    }
    fn report_private_mode(&mut self, mode: PrivateMode) {
        self.status_report_private_mode(mode);
    }

    fn save_private_mode_values(&mut self, modes: &[u16]) {
        self.apply_xtsave(modes);
    }
    fn restore_private_mode_values(&mut self, modes: &[u16]) {
        self.apply_xtrestore(modes);
    }
    fn identify_terminal(&mut self, intermediate: Option<char>) {
        self.status_identify_terminal(intermediate);
    }
    fn xtversion(&mut self) {
        self.status_xtversion();
    }
    fn xtgettcap(&mut self, payload: &[u8], aborted: bool) {
        self.status_xtgettcap(payload, aborted);
    }
    fn device_status(&mut self, arg: usize) {
        self.status_device_status(arg);
    }
    fn enquiry(&mut self) {
        self.status_enquiry();
    }
    fn text_area_size_chars(&mut self) {
        self.status_text_area_size_chars();
    }

    fn set_keypad_application_mode(&mut self) {
        self.mode.insert(TermMode::APP_KEYPAD);
    }
    fn unset_keypad_application_mode(&mut self) {
        self.mode.remove(TermMode::APP_KEYPAD);
    }
    fn reset_state(&mut self) {
        self.esc_reset_state();
    }

    fn decstr(&mut self) {
        self.soft_reset();
    }

    fn push_sgr(&mut self) {
        self.grid_mut().push_sgr();
    }

    fn pop_sgr(&mut self) {
        self.grid_mut().pop_sgr();
    }

    #[inline]
    fn terminal_attribute(&mut self, attr: Attr) {
        self.terminal_attribute_impl(&attr);
    }

    fn set_title(&mut self, title: Option<String>) {
        self.osc_set_title(title);
    }
    fn set_icon_name(&mut self, name: Option<String>) {
        self.osc_set_icon_name(name);
    }
    fn push_title(&mut self) {
        self.osc_push_title();
    }
    fn pop_title(&mut self) {
        self.osc_pop_title();
    }
    fn set_color(&mut self, index: usize, color: Rgb) {
        self.osc_set_color(index, color);
    }
    fn reset_color(&mut self, index: usize) {
        self.osc_reset_color(index);
    }

    fn dynamic_color_sequence(&mut self, prefix: String, index: usize, terminator: &str) {
        self.osc_dynamic_color_sequence(&prefix, index, terminator);
    }

    fn clipboard_store(&mut self, clipboard: u8, base64: &[u8]) {
        self.osc_clipboard_store(clipboard, base64);
    }

    fn clipboard_load(&mut self, clipboard: u8, terminator: &str) {
        self.osc_clipboard_load(clipboard, terminator);
    }

    fn set_hyperlink(&mut self, hyperlink: Option<VteHyperlink>) {
        self.osc_set_hyperlink(hyperlink);
    }

    fn set_cursor_style(&mut self, style: Option<CursorStyle>) {
        self.dcs_set_cursor_style(style);
    }

    fn set_cursor_shape(&mut self, shape: vte::ansi::CursorShape) {
        self.dcs_set_cursor_shape(shape);
    }

    fn set_mouse_cursor_icon(&mut self, icon: vte::ansi::cursor_icon::CursorIcon) {
        Self::set_mouse_cursor_icon(self, Some(icon));
    }

    fn push_keyboard_mode(&mut self, mode: KeyboardModes) {
        self.dcs_push_keyboard_mode(mode);
    }

    fn pop_keyboard_modes(&mut self, to_pop: u16) {
        self.dcs_pop_keyboard_modes(to_pop);
    }

    fn set_keyboard_mode(&mut self, mode: KeyboardModes, apply: KeyboardModesApplyBehavior) {
        self.dcs_set_keyboard_mode(mode, apply);
    }

    fn report_keyboard_mode(&mut self) {
        self.dcs_report_keyboard_mode();
    }
    fn set_modify_other_keys(&mut self, mode: ModifyOtherKeys) {
        self.dcs_set_modify_other_keys(mode);
    }
    fn report_modify_other_keys(&mut self) {
        self.dcs_report_modify_other_keys();
    }
    fn text_area_size_pixels(&mut self) {
        self.dcs_text_area_size_pixels();
    }
    delegate_osc!(graphics_attribute(pi: u16, pa: u16, pv: u16) => status_graphics_attribute);
    delegate_osc!(apc_dispatch(payload: &[u8]) => handle_apc_dispatch);
    delegate_osc!(sixel_start(params: &[u16]) => handle_sixel_start);
    delegate_osc!(sixel_put(byte: u8) => handle_sixel_put);
    delegate_osc!(sixel_end(aborted: bool) => handle_sixel_end);
    delegate_osc!(iterm2_file(params: &[&[u8]]) => handle_iterm2_file);
    delegate_osc!(iterm2_set_mark() => osc_iterm2_set_mark);
    delegate_osc!(iterm2_remote_host(host: &[u8]) => osc_iterm2_remote_host);
    delegate_osc!(iterm2_current_dir(path: &[u8]) => osc_iterm2_current_dir);
    delegate_osc!(iterm2_copy(data: &[u8]) => osc_iterm2_copy);
    delegate_osc!(iterm2_report_cell_size() => osc_iterm2_report_cell_size);
    delegate_osc!(iterm2_set_user_var(name: &[u8], value: &[u8]) => osc_iterm2_set_user_var);
    delegate_osc!(iterm2_shell_integration_version(version: &[u8]) => osc_iterm2_shell_integration_version);
    delegate_osc!(decrqss(query: &[u8]) => status_decrqss);
    delegate_osc!(decrsps(ps: u16, pt: &[u8]) => status_decrsps);

    // §10.9 OSC 3 / 5 / 6 / 13 / 14 / 17 / 19 / 113 / 114 / 117 / 119 delegates — each forwards to its `osc_*` helper in `handler/osc.rs`. Compressed via `delegate_osc!` to keep the trait impl under the 500-line file budget.
    delegate_osc!(set_x11_property(payload: &[u8]) => osc_set_x11_property);
    delegate_osc!(set_special_color(index: usize, color: Rgb) => osc_set_special_color);
    delegate_osc!(query_special_color(index: usize, terminator: &str) => osc_query_special_color);
    delegate_osc!(set_tab_title_color(color: Rgb) => osc_set_tab_title_color);
    delegate_osc!(set_mouse_fg_color(color: Rgb) => osc_set_mouse_fg_color);
    delegate_osc!(set_mouse_bg_color(color: Rgb) => osc_set_mouse_bg_color);
    delegate_osc!(set_highlight_bg_color(color: Rgb) => osc_set_highlight_bg_color);
    delegate_osc!(set_highlight_fg_color(color: Rgb) => osc_set_highlight_fg_color);
    delegate_osc!(query_mouse_fg_color(terminator: &str) => osc_query_mouse_fg_color);
    delegate_osc!(query_mouse_bg_color(terminator: &str) => osc_query_mouse_bg_color);
    delegate_osc!(query_highlight_bg_color(terminator: &str) => osc_query_highlight_bg_color);
    delegate_osc!(query_highlight_fg_color(terminator: &str) => osc_query_highlight_fg_color);
    delegate_osc!(reset_mouse_fg_color() => osc_reset_mouse_fg_color);
    delegate_osc!(reset_mouse_bg_color() => osc_reset_mouse_bg_color);
    delegate_osc!(reset_highlight_bg_color() => osc_reset_highlight_bg_color);
    delegate_osc!(reset_highlight_fg_color() => osc_reset_highlight_fg_color);

    // §09A.4 — DEC private rect-ops + presentation delegates (stubs;
    // real semantics land in §09A.5 (DECRQCRA), §09A.6 (rect mutation),
    // §09A.7 (column + index), §09A.8 (CSI-path presentation queries)).
    delegate_osc!(decsace(mode: u16) => decsace_impl);
    fn deccara(&mut self, top: u16, left: u16, bot: u16, right: u16, attrs: &[u16]) {
        self.deccara_impl(
            DecRect {
                top,
                left,
                bot,
                right,
            },
            attrs,
        );
    }
    fn decrara(&mut self, top: u16, left: u16, bot: u16, right: u16, attrs: &[u16]) {
        self.decrara_impl(
            DecRect {
                top,
                left,
                bot,
                right,
            },
            attrs,
        );
    }
    fn deccra(&mut self, st: u16, sl: u16, sb: u16, sr: u16, _sp: u16, dt: u16, dl: u16, _dp: u16) {
        self.deccra_impl(
            DecRect {
                top: st,
                left: sl,
                bot: sb,
                right: sr,
            },
            dt,
            dl,
        );
    }
    fn decfra(&mut self, ch: u16, top: u16, left: u16, bot: u16, right: u16) {
        self.decfra_impl(
            ch,
            DecRect {
                top,
                left,
                bot,
                right,
            },
        );
    }
    delegate_osc!(xtchecksum(flags: u16) => xtchecksum_impl);
    fn decrqcra(&mut self, id: u16, page: u16, top: u16, left: u16, bot: u16, right: u16) {
        self.decrqcra_impl(
            id,
            page,
            DecRect {
                top,
                left,
                bot,
                right,
            },
        );
    }
    fn decera(&mut self, top: u16, left: u16, bot: u16, right: u16) {
        self.decera_impl(DecRect {
            top,
            left,
            bot,
            right,
        });
    }
    fn decsera(&mut self, top: u16, left: u16, bot: u16, right: u16) {
        self.decsera_impl(DecRect {
            top,
            left,
            bot,
            right,
        });
    }
    fn xtreportsgr(&mut self, top: u16, left: u16, bot: u16, right: u16) {
        self.xtreportsgr_impl(DecRect {
            top,
            left,
            bot,
            right,
        });
    }
    delegate_osc!(decrqpsr(mode: u16) => decrqpsr_impl);
    delegate_osc!(decrqupss() => decrqupss_impl);
    delegate_osc!(decrqde() => decrqde_impl);
    delegate_osc!(decscl(level: u16, c1_mode: u16) => decscl_impl);
    delegate_osc!(decsca(protected: u16) => decsca_impl);
    delegate_osc!(decsasd(target: u16) => decsasd_impl);
    delegate_osc!(decssdt(line_type: u16) => decssdt_impl);
    delegate_osc!(decic(count: u16) => decic_impl);
    delegate_osc!(decdc(count: u16) => decdc_impl);
    delegate_osc!(decbi() => decbi_impl);
    delegate_osc!(decfi() => decfi_impl);

    // DEC Locator subsystem (CSI ' w/z/{/|). Independent of DECSET 1001
    // (highlight tracking — separate protocol per F1 cure). DECRQLP emits
    // the DECLRP reply (real observed coords via `compose_declrp_reply`)
    // through `effect_sink`; `handle_mouse_input` Step A feeds the observed
    // position. Continuous/OneShot push-emission (DECSLE-filtered events)
    // and daemon-mode wire propagation are separate deliverables.
    fn decefr(&mut self, pt: u16, pl: u16, pb: u16, pr: u16) {
        self.dec_locator.apply_decefr(pt, pl, pb, pr);
    }
    fn decelr(&mut self, ps: u16, pu: u16) {
        self.dec_locator.apply_decelr(ps, pu);
    }
    fn decsle(&mut self, events: &[u16]) {
        self.dec_locator.apply_decsle(events);
    }
    fn decrqlp(&mut self, ps: u16) {
        // DECRQLP — Request Locator Position. Ps=0|1|omitted = transmit
        // single DECLRP reply. Format per xterm spec:
        //   CSI Pe ; Pm ; Pr ; Pc ; Pp & w
        // - Pe = event code (0=unavail, 1=request response, 2-9=button,
        //   10=outside-filter-rect)
        // - Pm = button bitmask
        // - Pr = row (1-based)
        // - Pc = col (1-based)
        // - Pp = page (always 1 for ori_term — no page memory)
        //
        // Ps values other than 0/1 are silently dropped per xterm spec.
        if ps != 0 && ps != 1 {
            return;
        }
        let reply = self.compose_declrp_reply();
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: reply,
            kind: PtyWriteKind::MouseEvent,
        }));
        // Per xterm spec, OneShot reporting auto-clears after the reply
        // fires. on_decrqlp_acknowledged() flips reporting → None iff
        // it was Some(OneShot).
        self.dec_locator.on_decrqlp_acknowledged();
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod tack_cap_xcheck;
