//! VTE handler implementation for `Term<T>`.
//!
//! Implements `vte::ansi::Handler` to process escape sequences, control
//! characters, and printable input. Each method delegates to the
//! appropriate grid/cursor/mode operation.

use log::debug;
use unicode_width::UnicodeWidthChar;
use vte::ansi::{
    Attr, CharsetIndex, ClearMode, CursorStyle, Handler, Hyperlink as VteHyperlink, KeyboardModes,
    KeyboardModesApplyBehavior, LineClearMode, Mode, ModifyOtherKeys, NamedMode, PrivateMode, Rgb,
    StandardCharset, TabulationClearMode,
};

use crate::event::{Event, EventListener};
use crate::grid::editing::{DisplayEraseMode, LineEraseMode};
use crate::grid::navigation::TabClearMode;
use crate::index::Column;

use super::{Term, TermMode};

mod dcs;
mod esc;
mod helpers;
mod image;
mod modes;
mod osc;
mod sgr;
mod status;

// VTE Handler trait impl — a dispatch table mapping escape sequences to
// helper methods in submodules (helpers, dcs, esc, modes, osc, sgr, status,
// image). Each method is a thin delegation; logic lives in the submodule.
//
// Rust requires a single `impl Trait` block, so this file is exempt from
// the 500-line split rule (it's a pure dispatch table with no logic).

impl<T: EventListener> Handler for Term<T> {
    #[inline]
    fn input(&mut self, c: char) {
        self.selection_dirty = true;

        // DECAWM off: when cursor is at wrap-pending position (past last
        // column), snap it back to the last column. The next character
        // overwrites the last cell instead of wrapping to the next line.
        if !self.mode.contains(TermMode::LINE_WRAP) {
            let cols = self.grid().cols();
            if self.grid().cursor().col().0 >= cols {
                self.grid_mut().cursor_mut().set_col(Column(cols - 1));
            }
        }

        // Fast path: ASCII printable (0x20–0x7E), no charset mapping, no
        // INSERT mode. Skips charset.translate(), UnicodeWidthChar::width(),
        // insert_blank(), and image pruning. The grid's put_char_ascii
        // skips the range check (we already verified it) and writes the cell
        // directly. If put_char_ascii declines (wrap pending, wide cell at
        // cursor), we fall through to the full slow path which handles
        // wrapping + linefeed + image pruning correctly.
        if c as u32 <= 0x7E
            && c as u32 >= 0x20
            && !self.mode.contains(TermMode::INSERT)
            && self.charset.is_ascii()
            && self.grid_mut().put_char_ascii(c)
        {
            return;
        }

        let c = self.charset.translate(c);
        let width = match UnicodeWidthChar::width(c) {
            Some(width) => width,
            None => return,
        };
        if width == 0 {
            self.grid_mut().push_zerowidth(c);
            return;
        }

        // Wide char at last column with DECAWM off: doesn't fit, skip it.
        // Cursor stays at wrap-pending so subsequent narrow chars overwrite.
        if width == 2 && !self.mode.contains(TermMode::LINE_WRAP) {
            let col = self.grid().cursor().col().0;
            let cols = self.grid().cols();
            if col + 1 >= cols {
                self.grid_mut().cursor_mut().set_col(Column(cols));
                return;
            }
        }

        let prev = self.grid().total_evicted();
        let insert = self.mode.contains(TermMode::INSERT);
        let grid = self.grid_mut();
        if insert {
            grid.insert_blank(width);
        }
        grid.put_char(c);
        self.prune_images_if_evicted(prev);
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
        self.event_listener.send_event(Event::Bell);
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
        self.grid_mut().move_to_column(Column(col));
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
        self.grid_mut().save_cursor();
    }
    fn restore_cursor_position(&mut self) {
        self.grid_mut().restore_cursor();
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
    fn apc_dispatch(&mut self, payload: &[u8]) {
        self.handle_apc_dispatch(payload);
    }
    fn sixel_start(&mut self, params: &[u16]) {
        self.handle_sixel_start(params);
    }
    fn sixel_put(&mut self, byte: u8) {
        self.handle_sixel_put(byte);
    }
    fn sixel_end(&mut self) {
        self.handle_sixel_end();
    }
    fn iterm2_file(&mut self, params: &[&[u8]]) {
        self.handle_iterm2_file(params);
    }
    fn decrqss(&mut self, query: &[u8]) {
        self.status_decrqss(query);
    }
}

#[cfg(test)]
mod tests;
