//! Handler trait for terminal emulator actions.

extern crate alloc;

use alloc::string::String;

use cursor_icon::CursorIcon;

use super::colors::{Hyperlink, Rgb};
use super::types::{
    Attr, CharsetIndex, ClearMode, CursorShape, CursorStyle, KeyboardModes,
    KeyboardModesApplyBehavior, LineClearMode, Mode, ModifyOtherKeys, PrivateMode, ScpCharPath,
    ScpUpdateMode, StandardCharset, TabulationClearMode,
};

/// Type that handles actions from the parser.
///
/// XXX Should probably not provide default impls for everything, but it makes
/// writing specific handler impls for tests far easier.
pub trait Handler {
    /// OSC to set window title.
    fn set_title(&mut self, _: Option<String>) {}

    /// OSC 1: set icon name.
    fn set_icon_name(&mut self, _: Option<String>) {}

    /// OSC 7: set working directory (shell integration).
    fn set_working_directory(&mut self, _: Option<String>) {}

    /// Set the cursor style.
    fn set_cursor_style(&mut self, _: Option<CursorStyle>) {}

    /// Set the cursor shape.
    fn set_cursor_shape(&mut self, _shape: CursorShape) {}

    /// A character to be displayed.
    fn input(&mut self, _c: char) {}

    /// Set cursor to position.
    fn goto(&mut self, _line: i32, _col: usize) {}

    /// Set cursor to specific row.
    fn goto_line(&mut self, _line: i32) {}

    /// Set cursor to specific column.
    fn goto_col(&mut self, _col: usize) {}

    /// Insert blank characters in current line starting from cursor.
    fn insert_blank(&mut self, _: usize) {}

    /// Move cursor up `rows`.
    fn move_up(&mut self, _: usize) {}

    /// Move cursor down `rows`.
    fn move_down(&mut self, _: usize) {}

    /// Identify the terminal (should write back to the pty stream).
    fn identify_terminal(&mut self, _intermediate: Option<char>) {}

    /// Report device status.
    fn device_status(&mut self, _: usize) {}

    /// Move cursor forward `cols`.
    fn move_forward(&mut self, _col: usize) {}

    /// Move cursor backward `cols`.
    fn move_backward(&mut self, _col: usize) {}

    /// Move cursor down `rows` and set to column 1.
    fn move_down_and_cr(&mut self, _row: usize) {}

    /// Move cursor up `rows` and set to column 1.
    fn move_up_and_cr(&mut self, _row: usize) {}

    /// Put `count` tabs.
    fn put_tab(&mut self, _count: u16) {}

    /// Backspace `count` characters.
    fn backspace(&mut self) {}

    /// Carriage return.
    fn carriage_return(&mut self) {}

    /// Linefeed.
    fn linefeed(&mut self) {}

    /// Ring the bell.
    ///
    /// Hopefully this is never implemented.
    fn bell(&mut self) {}

    /// Substitute char under cursor.
    fn substitute(&mut self) {}

    /// Newline.
    fn newline(&mut self) {}

    /// Set current position as a tabstop.
    fn set_horizontal_tabstop(&mut self) {}

    /// Scroll up `rows` rows.
    fn scroll_up(&mut self, _: usize) {}

    /// Scroll down `rows` rows.
    fn scroll_down(&mut self, _: usize) {}

    /// Insert `count` blank lines.
    fn insert_blank_lines(&mut self, _: usize) {}

    /// Delete `count` lines.
    fn delete_lines(&mut self, _: usize) {}

    /// Erase `count` chars in current line following cursor.
    ///
    /// Erase means resetting to the default state (default colors, no content,
    /// no mode flags).
    fn erase_chars(&mut self, _: usize) {}

    /// Delete `count` chars.
    ///
    /// Deleting a character is like the delete key on the keyboard - everything
    /// to the right of the deleted things is shifted left.
    fn delete_chars(&mut self, _: usize) {}

    /// Move backward `count` tabs.
    fn move_backward_tabs(&mut self, _count: u16) {}

    /// Move forward `count` tabs.
    fn move_forward_tabs(&mut self, _count: u16) {}

    /// Save current cursor position.
    fn save_cursor_position(&mut self) {}

    /// Restore cursor position.
    fn restore_cursor_position(&mut self) {}

    /// Clear current line.
    fn clear_line(&mut self, _mode: LineClearMode) {}

    /// Clear screen.
    fn clear_screen(&mut self, _mode: ClearMode) {}

    /// Clear tab stops.
    fn clear_tabs(&mut self, _mode: TabulationClearMode) {}

    /// Set tab stops at every `interval`.
    fn set_tabs(&mut self, _interval: u16) {}

    /// Reset terminal state.
    fn reset_state(&mut self) {}

    /// Reverse Index.
    ///
    /// Move the active position to the same horizontal position on the
    /// preceding line. If the active position is at the top margin, a scroll
    /// down is performed.
    fn reverse_index(&mut self) {}

    /// Set a terminal attribute.
    fn terminal_attribute(&mut self, _attr: Attr) {}

    /// Set mode.
    fn set_mode(&mut self, _mode: Mode) {}

    /// Unset mode.
    fn unset_mode(&mut self, _mode: Mode) {}

    /// DECRPM - report mode.
    fn report_mode(&mut self, _mode: Mode) {}

    /// Set private mode.
    fn set_private_mode(&mut self, _mode: PrivateMode) {}

    /// Unset private mode.
    fn unset_private_mode(&mut self, _mode: PrivateMode) {}

    /// DECRPM - report private mode.
    fn report_private_mode(&mut self, _mode: PrivateMode) {}

    /// XTSAVE -- save private mode values.
    fn save_private_mode_values(&mut self, _modes: &[u16]) {}

    /// XTRESTORE -- restore private mode values.
    fn restore_private_mode_values(&mut self, _modes: &[u16]) {}

    /// DECSTBM - Set the terminal scrolling region.
    fn set_scrolling_region(&mut self, _top: usize, _bottom: Option<usize>) {}

    /// DECKPAM - Set keypad to applications mode (ESCape instead of digits).
    fn set_keypad_application_mode(&mut self) {}

    /// DECKPNM - Set keypad to numeric mode (digits instead of ESCape seq).
    fn unset_keypad_application_mode(&mut self) {}

    /// Set one of the graphic character sets, G0 to G3, as the active charset.
    ///
    /// 'Invoke' one of G0 to G3 in the GL area. Also referred to as shift in,
    /// shift out and locking shift depending on the set being activated.
    fn set_active_charset(&mut self, _: CharsetIndex) {}

    /// Single Shift (SS2/SS3): temporarily invoke G2 or G3 for the next character.
    fn set_single_shift(&mut self, _: CharsetIndex) {}

    /// Assign a graphic character set to G0, G1, G2 or G3.
    ///
    /// 'Designate' a graphic character set as one of G0 to G3, so that it can
    /// later be 'invoked' by `set_active_charset`.
    fn configure_charset(&mut self, _: CharsetIndex, _: StandardCharset) {}

    /// Set an indexed color value.
    fn set_color(&mut self, _: usize, _: Rgb) {}

    /// Respond to a color query escape sequence.
    fn dynamic_color_sequence(&mut self, _: String, _: usize, _: &str) {}

    /// Reset an indexed color to original value.
    fn reset_color(&mut self, _: usize) {}

    /// Store data into clipboard.
    fn clipboard_store(&mut self, _: u8, _: &[u8]) {}

    /// Load data from clipboard.
    fn clipboard_load(&mut self, _: u8, _: &str) {}

    /// Run the decaln routine.
    fn decaln(&mut self) {}

    /// Push a title onto the stack.
    fn push_title(&mut self) {}

    /// Pop the last title from the stack.
    fn pop_title(&mut self) {}

    /// Report text area size in pixels.
    fn text_area_size_pixels(&mut self) {}

    /// Report text area size in characters.
    fn text_area_size_chars(&mut self) {}

    /// Set hyperlink.
    fn set_hyperlink(&mut self, _: Option<Hyperlink>) {}

    /// Set mouse cursor icon.
    fn set_mouse_cursor_icon(&mut self, _: CursorIcon) {}

    /// Report current keyboard mode.
    fn report_keyboard_mode(&mut self) {}

    /// Push keyboard mode into the keyboard mode stack.
    fn push_keyboard_mode(&mut self, _mode: KeyboardModes) {}

    /// Pop the given amount of keyboard modes from the
    /// keyboard mode stack.
    fn pop_keyboard_modes(&mut self, _to_pop: u16) {}

    /// Set the [`keyboard mode`] using the given [`behavior`].
    ///
    /// [`keyboard mode`]: crate::ansi::KeyboardModes
    /// [`behavior`]: crate::ansi::KeyboardModesApplyBehavior
    fn set_keyboard_mode(&mut self, _mode: KeyboardModes, _behavior: KeyboardModesApplyBehavior) {}

    /// Set XTerm's [`ModifyOtherKeys`] option.
    fn set_modify_other_keys(&mut self, _mode: ModifyOtherKeys) {}

    /// Report XTerm's [`ModifyOtherKeys`] state.
    ///
    /// The output is of form `CSI > 4 ; mode m`.
    fn report_modify_other_keys(&mut self) {}

    // Set SCP control.
    fn set_scp(&mut self, _char_path: ScpCharPath, _update_mode: ScpUpdateMode) {}

    /// Called when a DCS sixel sequence begins (DCS with action `q`).
    ///
    /// `params` contains P1/P2/P3 from the DCS introducer.
    fn sixel_start(&mut self, _params: &[u16]) {}

    /// Called for each byte of sixel data within an active DCS sixel sequence.
    fn sixel_put(&mut self, _byte: u8) {}

    /// Called when the DCS sixel sequence ends (ST terminator).
    fn sixel_end(&mut self) {}

    /// DECRQSS: Request Status String (DCS $ q ... ST).
    ///
    /// `query` contains the status type bytes, e.g. `"p` for DECSCL
    /// (conformance level), `r` for DECSTBM, `m` for SGR.
    fn decrqss(&mut self, _query: &[u8]) {}

    /// Dispatch an APC (Application Program Command) sequence.
    ///
    /// The `payload` contains the raw bytes between `ESC _` and `ST`.
    /// The first byte typically identifies the command type (e.g., `G` for
    /// Kitty graphics protocol).
    fn apc_dispatch(&mut self, _payload: &[u8]) {}

    /// Handle an iTerm2 image protocol sequence (OSC 1337 File=...).
    ///
    /// `params` are the raw OSC params after `1337`, with the VTE parser
    /// having split on `;`. The first param starts with `File=` and the
    /// last param contains `:<base64-data>` after the final key=value pair.
    fn iterm2_file(&mut self, _params: &[&[u8]]) {}
}
