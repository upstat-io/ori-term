//! VTE handler implementation for `Term<S>`.
//!
//! Implements `vte::ansi::Handler` to process escape sequences, control
//! characters, and printable input. Each method delegates to the
//! appropriate grid/cursor/mode operation.

use log::debug;
use vte::ansi::{
    Attr, CharsetIndex, ClearMode, CursorStyle, Handler, Hyperlink as VteHyperlink, KeyboardModes,
    KeyboardModesApplyBehavior, LineClearMode, Mode, ModifyOtherKeys, NamedMode, PrivateMode, Rgb,
    StandardCharset, TabulationClearMode,
};

use crate::effect::sink::EffectSink;
use crate::effect::{Effect, HostEffect};
use crate::grid::editing::{DisplayEraseMode, LineEraseMode};
use crate::grid::navigation::TabClearMode;

use super::{Term, TermMode};

mod dcs;
mod esc;
mod helpers;
mod image;
mod modes;
mod osc;
mod presentation;
mod rect_ops;
mod sgr;
mod status;

/// Generate a one-line `Handler` trait method that delegates to an
/// inherent helper on `Term<S>`. Used by the OSC 3/5/6/… block, the
/// iTerm2 / sixel / APC / DECRQSS delegates, and the §09A.4 DEC
/// private rect-ops + presentation delegates. Keeps the trait impl
/// under the 500-line file budget per `.claude/rules/code-hygiene.md`.
macro_rules! delegate_osc {
    ($method:ident($($arg:ident : $ty:ty),*) => $helper:ident) => {
        fn $method(&mut self, $($arg: $ty),*) { self.$helper($($arg),*); }
    };
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
        self.selection_dirty = true;
        let lnm = self.mode.contains(TermMode::LINE_FEED_NEW_LINE);
        let prev = self.grid().total_evicted();
        let grid = self.grid_mut();
        if lnm {
            grid.next_line();
        } else {
            grid.linefeed();
        }
        self.prune_images_if_evicted(prev);
    }

    #[inline]
    fn carriage_return(&mut self) {
        self.grid_mut().carriage_return();
    }

    #[inline]
    fn bell(&mut self) {
        self.effect_sink.push(Effect::Host(HostEffect::Bell));
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
        self.selection_dirty = true;
        let erase = match mode {
            ClearMode::Below => DisplayEraseMode::Below,
            ClearMode::Above => DisplayEraseMode::Above,
            ClearMode::All => DisplayEraseMode::All,
            ClearMode::Saved => DisplayEraseMode::Scrollback,
        };
        self.grid_mut().erase_display(erase);
        self.clear_images_after_ed(&mode);
    }

    fn clear_line(&mut self, mode: LineClearMode) {
        self.selection_dirty = true;
        let erase = match mode {
            LineClearMode::Right => LineEraseMode::Right,
            LineClearMode::Left => LineEraseMode::Left,
            LineClearMode::All => LineEraseMode::All,
        };
        self.grid_mut().erase_line(erase);
        self.clear_images_after_el(&mode);
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
        self.selection_dirty = true;
        let prev = self.grid().total_evicted();
        self.grid_mut().scroll_up(count);
        self.prune_images_if_evicted(prev);
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
        self.selection_dirty = true;
        let prev = self.grid().total_evicted();
        self.grid_mut().next_line();
        self.prune_images_if_evicted(prev);
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
        let clear = match mode {
            TabulationClearMode::Current => TabClearMode::Current,
            TabulationClearMode::All => TabClearMode::All,
        };
        self.grid_mut().clear_tab_stop(clear);
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
        match mode {
            Mode::Named(NamedMode::Insert) => self.mode.insert(TermMode::INSERT),
            Mode::Named(NamedMode::LineFeedNewLine) => {
                self.mode.insert(TermMode::LINE_FEED_NEW_LINE);
            }
            Mode::Unknown(n) => debug!("Ignoring unknown mode {n} in SM"),
        }
    }

    fn unset_mode(&mut self, mode: Mode) {
        match mode {
            Mode::Named(NamedMode::Insert) => self.mode.remove(TermMode::INSERT),
            Mode::Named(NamedMode::LineFeedNewLine) => {
                self.mode.remove(TermMode::LINE_FEED_NEW_LINE);
            }
            Mode::Unknown(n) => debug!("Ignoring unknown mode {n} in RM"),
        }
    }

    fn set_private_mode(&mut self, mode: PrivateMode) {
        match mode {
            PrivateMode::Named(m) => self.apply_decset(m),
            PrivateMode::Unknown(n) => debug!("Ignoring unknown private mode {n} in DECSET"),
        }
    }

    fn unset_private_mode(&mut self, mode: PrivateMode) {
        match mode {
            PrivateMode::Named(m) => self.apply_decrst(m),
            PrivateMode::Unknown(n) => debug!("Ignoring unknown private mode {n} in DECRST"),
        }
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
    fn device_status(&mut self, arg: usize) {
        self.status_device_status(arg);
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
        let template = &mut self.grid_mut().cursor_mut().template;
        sgr::apply(template, &attr);
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
    delegate_osc!(apc_dispatch(payload: &[u8]) => handle_apc_dispatch);
    delegate_osc!(sixel_start(params: &[u16]) => handle_sixel_start);
    delegate_osc!(sixel_put(byte: u8) => handle_sixel_put);
    delegate_osc!(sixel_end() => handle_sixel_end);
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

    // §10.9 OSC 3 / 5 / 6 / 13 / 14 / 17 / 19 / 113 / 114 / 117 / 119
    // delegates — each forwards to its `osc_*` helper in `handler/osc.rs`.
    // Compressed via `delegate_osc!` to keep the trait impl under the
    // 500-line file budget (see `.claude/rules/code-hygiene.md`).
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
    delegate_osc!(deccara(top: u16, left: u16, bot: u16, right: u16, attrs: &[u16]) => deccara_impl);
    delegate_osc!(decrara(top: u16, left: u16, bot: u16, right: u16, attrs: &[u16]) => decrara_impl);
    fn deccra(&mut self, st: u16, sl: u16, sb: u16, sr: u16, sp: u16, dt: u16, dl: u16, dp: u16) {
        self.deccra_impl(st, sl, sb, sr, sp, dt, dl, dp);
    }
    delegate_osc!(decfra(ch: u16, top: u16, left: u16, bot: u16, right: u16) => decfra_impl);
    delegate_osc!(xtchecksum(flags: u16) => xtchecksum_impl);
    delegate_osc!(decrqcra(id: u16, page: u16, top: u16, left: u16, bot: u16, right: u16) => decrqcra_impl);
    delegate_osc!(decera(top: u16, left: u16, bot: u16, right: u16) => decera_impl);
    delegate_osc!(decsera(top: u16, left: u16, bot: u16, right: u16) => decsera_impl);
    delegate_osc!(xtreportsgr(top: u16, left: u16, bot: u16, right: u16) => xtreportsgr_impl);
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
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod tack_cap_xcheck;
