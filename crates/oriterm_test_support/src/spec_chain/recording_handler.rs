//! Recording handler wrapper for semantic dispatch observation.
//!
//! Wraps `Term<S: EffectSink>` and implements `vte::ansi::Handler`.
//! Every handler method records a `DispatchCall` (method name + typed
//! arguments) and delegates to the inner `Term`. This is how the
//! `SpecHarness` captures rung 2 (dispatch routing).

use oriterm_core::Term;
use oriterm_core::effect::sink::EffectSink;
use vte::ansi::Handler;

// Re-import cursor_icon via the vendored VTE's re-export.
use vte::ansi::cursor_icon;

/// A single recorded handler dispatch call.
#[derive(Debug, Clone)]
pub struct DispatchCall {
    /// Handler method name (e.g. `"goto"`, `"input"`, `"bell"`).
    pub method: &'static str,
    /// Typed argument capture.
    pub args: DispatchArgs,
}

/// Typed argument capture for each handler method family.
///
/// Exhaustive coverage is built incrementally as pilots exercise methods.
/// Methods not yet covered use `Other`.
#[derive(Debug, Clone)]
pub enum DispatchArgs {
    /// `Handler::input(c)`.
    Input(char),
    /// `Handler::goto(line, col)`.
    Goto { line: i32, col: usize },
    /// `Handler::goto_line(line)`.
    GotoLine { line: i32 },
    /// `Handler::goto_col(col)`.
    GotoCol { col: usize },
    /// `Handler::bell()`.
    Bell,
    /// `Handler::linefeed()`.
    Linefeed,
    /// `Handler::carriage_return()`.
    CarriageReturn,
    /// `Handler::backspace()`.
    Backspace,
    /// `Handler::identify_terminal(intermediate)`.
    IdentifyTerminal { intermediate: Option<char> },
    /// `Handler::device_status(n)`.
    DeviceStatus { n: usize },
    /// Fallback for methods not yet given typed args.
    Other { method: &'static str },
}

/// Recording wrapper around `Term<S>`.
///
/// Implements `Handler` by recording a `DispatchCall` for each method
/// call, then delegating to the inner `Term`.
pub struct RecordingHandler<S: EffectSink> {
    term: Term<S>,
    calls: Vec<DispatchCall>,
}

impl<S: EffectSink> RecordingHandler<S> {
    /// Create a new recording handler wrapping the given `Term`.
    pub fn new(term: Term<S>) -> Self {
        Self {
            term,
            calls: Vec::new(),
        }
    }

    /// Borrow the inner `Term`.
    pub fn term(&self) -> &Term<S> {
        &self.term
    }

    /// Mutably borrow the inner `Term`.
    pub fn term_mut(&mut self) -> &mut Term<S> {
        &mut self.term
    }

    /// Drain all recorded calls into the provided `Vec`.
    pub fn drain_calls_into(&mut self, out: &mut Vec<DispatchCall>) {
        out.append(&mut self.calls);
    }

    /// Push a recorded call.
    fn record(&mut self, method: &'static str, args: DispatchArgs) {
        self.calls.push(DispatchCall { method, args });
    }

    /// Push a recorded call with `Other` args.
    fn record_other(&mut self, method: &'static str) {
        self.record(method, DispatchArgs::Other { method });
    }
}

/// Macro for handler methods that just record `Other` and delegate.
///
/// Uses `Handler::$method` to ensure we call the trait impl, not any
/// inherent method with a different signature (e.g. `set_cursor_shape`
/// has both a Handler trait method taking `vte::ansi::CursorShape` and
/// an inherent Term method taking `oriterm_core::CursorShape`).
macro_rules! delegate_other {
    ($method:ident $(, $arg:ident : $ty:ty)*) => {
        fn $method(&mut self $(, $arg: $ty)*) {
            self.record_other(stringify!($method));
            Handler::$method(&mut self.term $(, $arg)*);
        }
    };
}

impl<S: EffectSink> Handler for RecordingHandler<S> {
    fn input(&mut self, c: char) {
        self.record("input", DispatchArgs::Input(c));
        Handler::input(&mut self.term, c);
    }

    fn goto(&mut self, line: i32, col: usize) {
        self.record("goto", DispatchArgs::Goto { line, col });
        Handler::goto(&mut self.term, line, col);
    }

    fn goto_line(&mut self, line: i32) {
        self.record("goto_line", DispatchArgs::GotoLine { line });
        Handler::goto_line(&mut self.term, line);
    }

    fn goto_col(&mut self, col: usize) {
        self.record("goto_col", DispatchArgs::GotoCol { col });
        Handler::goto_col(&mut self.term, col);
    }

    fn bell(&mut self) {
        self.record("bell", DispatchArgs::Bell);
        Handler::bell(&mut self.term);
    }

    fn linefeed(&mut self) {
        self.record("linefeed", DispatchArgs::Linefeed);
        Handler::linefeed(&mut self.term);
    }

    fn carriage_return(&mut self) {
        self.record("carriage_return", DispatchArgs::CarriageReturn);
        Handler::carriage_return(&mut self.term);
    }

    fn backspace(&mut self) {
        self.record("backspace", DispatchArgs::Backspace);
        Handler::backspace(&mut self.term);
    }

    fn identify_terminal(&mut self, intermediate: Option<char>) {
        self.record(
            "identify_terminal",
            DispatchArgs::IdentifyTerminal { intermediate },
        );
        Handler::identify_terminal(&mut self.term, intermediate);
    }

    fn device_status(&mut self, n: usize) {
        self.record("device_status", DispatchArgs::DeviceStatus { n });
        Handler::device_status(&mut self.term, n);
    }

    // All other handler methods — record as Other and delegate.
    delegate_other!(set_title, title: Option<String>);
    delegate_other!(set_icon_name, name: Option<String>);
    delegate_other!(set_working_directory, dir: Option<String>);
    delegate_other!(set_cursor_style, style: Option<vte::ansi::CursorStyle>);
    delegate_other!(set_cursor_shape, shape: vte::ansi::CursorShape);
    delegate_other!(insert_blank, count: usize);
    delegate_other!(move_up, rows: usize);
    delegate_other!(move_down, rows: usize);
    delegate_other!(move_forward, cols: usize);
    delegate_other!(move_backward, cols: usize);
    delegate_other!(move_down_and_cr, rows: usize);
    delegate_other!(move_up_and_cr, rows: usize);
    delegate_other!(put_tab, count: u16);
    delegate_other!(substitute);
    delegate_other!(newline);
    delegate_other!(set_horizontal_tabstop);
    delegate_other!(scroll_up, rows: usize);
    delegate_other!(scroll_down, rows: usize);
    delegate_other!(scroll_left, cols: usize);
    delegate_other!(scroll_right, cols: usize);
    delegate_other!(insert_blank_lines, count: usize);
    delegate_other!(delete_lines, count: usize);
    delegate_other!(erase_chars, count: usize);
    delegate_other!(delete_chars, count: usize);
    delegate_other!(move_backward_tabs, count: u16);
    delegate_other!(move_forward_tabs, count: u16);
    delegate_other!(save_cursor_position);
    delegate_other!(decslrm_or_save_cursor, has_params: bool, left: u16, right: u16);
    delegate_other!(restore_cursor_position);
    delegate_other!(clear_line, mode: vte::ansi::LineClearMode);
    delegate_other!(clear_screen, mode: vte::ansi::ClearMode);
    delegate_other!(clear_tabs, mode: vte::ansi::TabulationClearMode);
    delegate_other!(set_tabs, interval: u16);
    delegate_other!(reset_state);
    delegate_other!(reverse_index);
    delegate_other!(terminal_attribute, attr: vte::ansi::Attr);
    delegate_other!(set_mode, mode: vte::ansi::Mode);
    delegate_other!(unset_mode, mode: vte::ansi::Mode);
    delegate_other!(report_mode, mode: vte::ansi::Mode);
    delegate_other!(set_private_mode, mode: vte::ansi::PrivateMode);
    delegate_other!(unset_private_mode, mode: vte::ansi::PrivateMode);
    delegate_other!(report_private_mode, mode: vte::ansi::PrivateMode);

    fn save_private_mode_values(&mut self, modes: &[u16]) {
        self.record_other("save_private_mode_values");
        Handler::save_private_mode_values(&mut self.term, modes);
    }

    fn restore_private_mode_values(&mut self, modes: &[u16]) {
        self.record_other("restore_private_mode_values");
        Handler::restore_private_mode_values(&mut self.term, modes);
    }

    fn set_scrolling_region(&mut self, top: usize, bottom: Option<usize>) {
        self.record_other("set_scrolling_region");
        Handler::set_scrolling_region(&mut self.term, top, bottom);
    }

    delegate_other!(set_keypad_application_mode);
    delegate_other!(unset_keypad_application_mode);
    delegate_other!(set_active_charset, index: vte::ansi::CharsetIndex);
    delegate_other!(set_single_shift, index: vte::ansi::CharsetIndex);

    fn configure_charset(
        &mut self,
        index: vte::ansi::CharsetIndex,
        charset: vte::ansi::StandardCharset,
    ) {
        self.record_other("configure_charset");
        Handler::configure_charset(&mut self.term, index, charset);
    }

    fn set_color(&mut self, index: usize, color: vte::ansi::Rgb) {
        self.record_other("set_color");
        Handler::set_color(&mut self.term, index, color);
    }

    fn dynamic_color_sequence(&mut self, prefix: String, index: usize, terminator: &str) {
        self.record_other("dynamic_color_sequence");
        Handler::dynamic_color_sequence(&mut self.term, prefix, index, terminator);
    }

    delegate_other!(reset_color, index: usize);

    fn clipboard_store(&mut self, clipboard: u8, data: &[u8]) {
        self.record_other("clipboard_store");
        Handler::clipboard_store(&mut self.term, clipboard, data);
    }

    fn clipboard_load(&mut self, clipboard: u8, terminator: &str) {
        self.record_other("clipboard_load");
        Handler::clipboard_load(&mut self.term, clipboard, terminator);
    }

    delegate_other!(decaln);
    delegate_other!(push_title);
    delegate_other!(pop_title);
    delegate_other!(text_area_size_pixels);
    delegate_other!(text_area_size_chars);

    fn set_hyperlink(&mut self, hyperlink: Option<vte::ansi::Hyperlink>) {
        self.record_other("set_hyperlink");
        Handler::set_hyperlink(&mut self.term, hyperlink);
    }

    delegate_other!(set_mouse_cursor_icon, icon: cursor_icon::CursorIcon);
    delegate_other!(report_keyboard_mode);
    delegate_other!(push_keyboard_mode, mode: vte::ansi::KeyboardModes);
    delegate_other!(pop_keyboard_modes, to_pop: u16);

    fn set_keyboard_mode(
        &mut self,
        mode: vte::ansi::KeyboardModes,
        behavior: vte::ansi::KeyboardModesApplyBehavior,
    ) {
        self.record_other("set_keyboard_mode");
        Handler::set_keyboard_mode(&mut self.term, mode, behavior);
    }

    delegate_other!(set_modify_other_keys, mode: vte::ansi::ModifyOtherKeys);
    delegate_other!(report_modify_other_keys);

    fn set_scp(
        &mut self,
        char_path: vte::ansi::ScpCharPath,
        update_mode: vte::ansi::ScpUpdateMode,
    ) {
        self.record_other("set_scp");
        Handler::set_scp(&mut self.term, char_path, update_mode);
    }

    fn sixel_start(&mut self, params: &[u16]) {
        self.record_other("sixel_start");
        Handler::sixel_start(&mut self.term, params);
    }

    delegate_other!(sixel_put, byte: u8);
    delegate_other!(sixel_end);

    fn decrqss(&mut self, query: &[u8]) {
        self.record_other("decrqss");
        Handler::decrqss(&mut self.term, query);
    }

    fn apc_dispatch(&mut self, payload: &[u8]) {
        self.record_other("apc_dispatch");
        Handler::apc_dispatch(&mut self.term, payload);
    }

    fn iterm2_file(&mut self, params: &[&[u8]]) {
        self.record_other("iterm2_file");
        Handler::iterm2_file(&mut self.term, params);
    }
}
