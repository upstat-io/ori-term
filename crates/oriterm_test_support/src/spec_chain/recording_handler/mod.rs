//! Recording handler wrapper for semantic dispatch observation.
//!
//! Wraps `Term<S: EffectSink>` and implements `vte::ansi::Handler`.
//! Every handler method records a `DispatchCall` (method name + typed
//! arguments) and delegates to the inner `Term`. This is how the
//! `SpecHarness` captures rung 2 (dispatch routing).

use oriterm_core::Term;
use oriterm_core::effect::sink::EffectSink;
use oriterm_core::image::kitty::{KittyCommand, parse_kitty_command};
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
 /// `Handler::sixel_start(params)` — captures P1/P2/P3 (and any
 /// trailing DCS parameters the parser collected).
 SixelStart { params: Vec<u16> },
 /// `Handler::sixel_end(aborted)` — captures the abort bit so
 /// abort-plumbing tests can verify the dispatch-rung argument, not
 /// just the end-to-end `ImageCache` state.
 SixelEnd { aborted: bool },
 /// `Handler::xtgettcap(payload, aborted)` — captures the
 /// hex-encoded capability name list and the abort bit so
 /// parser-dispatch tests can verify both the payload bytes (case,
 /// content) and the abort plumbing.
 Xtgettcap { payload: Vec<u8>, aborted: bool },
 /// `Handler::apc_dispatch` routed to the Kitty graphics protocol
 /// (payload's first byte is `G`).
 /// The captured `KittyCommand` lets matrix tests assert per-cell
 /// dispatch identity (action × format × transmission) rather than
 /// collapsing every APC `_G` into the generic `Other` arm. The
 /// parser output is boxed to keep the enum discriminant small —
 /// `KittyCommand` is a wide struct (~200 bytes with its payload
 /// buffer) and the `Box` avoids bloating every other `DispatchArgs`
 /// variant.
 KittyApc { command: Box<KittyCommand> },
 /// Fallback for methods not yet given typed args.
 Other { method: &'static str },
}

/// Recording wrapper around `Term<S>`.
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

 fn xtgettcap(&mut self, payload: &[u8], aborted: bool) {
 self.record(
 "xtgettcap",
 DispatchArgs::Xtgettcap {
 payload: payload.to_vec(),
 aborted,
 },
 );
 Handler::xtgettcap(&mut self.term, payload, aborted);
 }

 // All other handler methods — record as Other and delegate.
 delegate_other!(xtversion);
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
 delegate_other!(enquiry);
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
 delegate_other!(graphics_attribute, pi: u16, pa: u16, pv: u16);

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
 self.record(
 "sixel_start",
 DispatchArgs::SixelStart {
 params: params.to_vec(),
 },
 );
 Handler::sixel_start(&mut self.term, params);
 }

 delegate_other!(sixel_put, byte: u8);

 fn sixel_end(&mut self, aborted: bool) {
 self.record("sixel_end", DispatchArgs::SixelEnd { aborted });
 Handler::sixel_end(&mut self.term, aborted);
 }

 fn decrqss(&mut self, query: &[u8]) {
 self.record_other("decrqss");
 Handler::decrqss(&mut self.term, query);
 }

 fn apc_dispatch(&mut self, payload: &[u8]) {
 // Payloads starting with `G` are kitty graphics commands; parse
 // into a typed `KittyCommand` so matrix tests can assert per-cell
 // dispatch identity (action × format × transmission) rather than
 // collapsing every APC `_G` into the generic `Other` arm. Parse
 // failures fall back to `Other` — a malformed APC body is a
 // distinct test concern from dispatch-arg capture.
 match payload.split_first() {
 Some((&b'G', body)) => match parse_kitty_command(body) {
 Ok(command) => self.record(
 "apc_dispatch",
 DispatchArgs::KittyApc {
 command: Box::new(command),
 },
 ),
 Err(_) => self.record_other("apc_dispatch"),
 },
 _ => self.record_other("apc_dispatch"),
 }
 Handler::apc_dispatch(&mut self.term, payload);
 }

 fn iterm2_file(&mut self, params: &[&[u8]]) {
 self.record_other("iterm2_file");
 Handler::iterm2_file(&mut self.term, params);
 }

 // Registration sync for the OSC 1337 non-image sub-ops added in §10.0
 // ( §10.0 REGISTRATION
 // SYNC). Without these arms, SpecHarness silently drops the new
 // Handler::iterm2_* dispatches and §10.7's spec_chain tests cannot
 // observe them.
 delegate_other!(iterm2_set_mark);
 delegate_other!(iterm2_remote_host, host: &[u8]);
 delegate_other!(iterm2_current_dir, path: &[u8]);
 delegate_other!(iterm2_copy, data: &[u8]);
 delegate_other!(iterm2_report_cell_size);
 delegate_other!(iterm2_set_user_var, name: &[u8], value: &[u8]);
 delegate_other!(iterm2_shell_integration_version, version: &[u8]);

 // Registration sync for the OSC 3 / 5 / 6 / 13 / 14 / 17 / 19 / 113 /
 // 114 / 117 / 119 Handler methods added in §10.9.
 delegate_other!(set_x11_property, payload: &[u8]);
 delegate_other!(set_special_color, index: usize, color: vte::ansi::Rgb);
 delegate_other!(query_special_color, index: usize, terminator: &str);
 delegate_other!(set_tab_title_color, color: vte::ansi::Rgb);
 delegate_other!(set_mouse_fg_color, color: vte::ansi::Rgb);
 delegate_other!(set_mouse_bg_color, color: vte::ansi::Rgb);
 delegate_other!(set_highlight_bg_color, color: vte::ansi::Rgb);
 delegate_other!(set_highlight_fg_color, color: vte::ansi::Rgb);
 delegate_other!(query_mouse_fg_color, terminator: &str);
 delegate_other!(query_mouse_bg_color, terminator: &str);
 delegate_other!(query_highlight_bg_color, terminator: &str);
 delegate_other!(query_highlight_fg_color, terminator: &str);
 delegate_other!(reset_mouse_fg_color);
 delegate_other!(reset_mouse_bg_color);
 delegate_other!(reset_highlight_bg_color);
 delegate_other!(reset_highlight_fg_color);

 // Registration sync for the DEC private rect-ops + presentation
 // Handler methods added in §09A.3/§09A.4. Without these arms,
 // SpecHarness silently drops dispatches and §09A.5+ spec_chain
 // tests cannot observe the emitted effects.
 delegate_other!(decsace, mode: u16);
 delegate_other!(deccara, top: u16, left: u16, bot: u16, right: u16, attrs: &[u16]);
 delegate_other!(decrara, top: u16, left: u16, bot: u16, right: u16, attrs: &[u16]);

 fn deccra(&mut self, st: u16, sl: u16, sb: u16, sr: u16, sp: u16, dt: u16, dl: u16, dp: u16) {
 self.record_other("deccra");
 Handler::deccra(&mut self.term, st, sl, sb, sr, sp, dt, dl, dp);
 }

 delegate_other!(decfra, ch: u16, top: u16, left: u16, bot: u16, right: u16);
 delegate_other!(xtchecksum, flags: u16);

 fn decrqcra(&mut self, id: u16, page: u16, top: u16, left: u16, bot: u16, right: u16) {
 self.record_other("decrqcra");
 Handler::decrqcra(&mut self.term, id, page, top, left, bot, right);
 }

 delegate_other!(decera, top: u16, left: u16, bot: u16, right: u16);
 delegate_other!(decsera, top: u16, left: u16, bot: u16, right: u16);
 delegate_other!(xtreportsgr, top: u16, left: u16, bot: u16, right: u16);
 delegate_other!(decrqpsr, mode: u16);
 delegate_other!(decrqupss);
 delegate_other!(decrqde);
 delegate_other!(decscl, level: u16, c1_mode: u16);
 delegate_other!(decsca, protected: u16);
 delegate_other!(decsasd, target: u16);
 delegate_other!(decssdt, line_type: u16);
 delegate_other!(decic, count: u16);
 delegate_other!(decdc, count: u16);
 delegate_other!(decbi);
 delegate_other!(decfi);
}

#[cfg(test)]
mod tests;
