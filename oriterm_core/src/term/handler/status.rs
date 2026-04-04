//! CSI device status and mode reporting handlers.
//!
//! DA (device attributes), DSR (device status report), DECRQM (mode report),
//! and CSI t (text area size). Methods are called by the `vte::ansi::Handler`
//! trait impl on `Term<T>`.
//!
//! All methods take `&mut self` because the `Handler` trait requires it,
//! even though these only read state and send events.

use log::debug;
use vte::ansi::{Color, Mode, NamedColor, NamedMode, PrivateMode};

use crate::cell::CellFlags;
use crate::event::{Event, EventListener};
use crate::term::{Term, TermMode};

use super::helpers::{
    crate_version_number, mode_report_value, named_private_mode_flag, named_private_mode_number,
};

/// Build the SGR parameter string for the current cursor attributes.
///
/// Returns `"0"` when all attributes are default, otherwise a semicolon-
/// separated list of SGR codes (e.g. `"0;1;4;31"` for bold+underline+red fg).
fn build_sgr_string(flags: CellFlags, fg: Color, bg: Color) -> String {
    let mut params = vec!["0".to_string()];

    if flags.contains(CellFlags::BOLD) {
        params.push("1".to_string());
    }
    if flags.contains(CellFlags::DIM) {
        params.push("2".to_string());
    }
    if flags.contains(CellFlags::ITALIC) {
        params.push("3".to_string());
    }
    if flags.contains(CellFlags::UNDERLINE) {
        params.push("4".to_string());
    }
    if flags.contains(CellFlags::BLINK) {
        params.push("5".to_string());
    }
    if flags.contains(CellFlags::INVERSE) {
        params.push("7".to_string());
    }
    if flags.contains(CellFlags::HIDDEN) {
        params.push("8".to_string());
    }
    if flags.contains(CellFlags::STRIKETHROUGH) {
        params.push("9".to_string());
    }

    push_color_params(&mut params, fg, true);
    push_color_params(&mut params, bg, false);

    params.join(";")
}

/// Append SGR color parameters for foreground or background.
fn push_color_params(params: &mut Vec<String>, color: Color, is_fg: bool) {
    let base = if is_fg { 30 } else { 40 };
    match color {
        Color::Named(NamedColor::Foreground | NamedColor::Background) => {}
        Color::Named(named) => {
            let idx = named as u8;
            // SGR 30-37 for colors 0-7, SGR 90-97 for bright colors 8-15.
            let code = match idx {
                0..8 => base + idx,
                8..16 => base + 60 + idx - 8,
                _ => return,
            };
            params.push(format!("{code}"));
        }
        Color::Indexed(idx) => {
            params.push(format!("{};5;{idx}", base + 8));
        }
        Color::Spec(rgb) => {
            params.push(format!("{};2;{};{};{}", base + 8, rgb.r, rgb.g, rgb.b));
        }
    }
}

#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "Handler trait requires &mut self"
)]
impl<T: EventListener> Term<T> {
    /// DECRQM: report ANSI mode status.
    pub(super) fn status_report_mode(&mut self, mode: Mode) {
        let (num, value) = match mode {
            Mode::Named(NamedMode::Insert) => (
                4u16,
                mode_report_value(self.mode.contains(TermMode::INSERT)),
            ),
            Mode::Named(NamedMode::LineFeedNewLine) => (
                20,
                mode_report_value(self.mode.contains(TermMode::LINE_FEED_NEW_LINE)),
            ),
            Mode::Unknown(n) => (n, 0),
        };
        let response = format!("\x1b[{num};{value}$y");
        self.event_listener.send_event(Event::PtyWrite(response));
    }

    /// DECRQM: report DEC private mode status.
    pub(super) fn status_report_private_mode(&mut self, mode: PrivateMode) {
        let (num, value) = match mode {
            PrivateMode::Named(named) => {
                let num = named_private_mode_number(named);
                let flag = named_private_mode_flag(named);
                let value = flag.map_or(0, |f| mode_report_value(self.mode.contains(f)));
                (num, value)
            }
            PrivateMode::Unknown(n) => (n, 0),
        };
        let response = format!("\x1b[?{num};{value}$y");
        self.event_listener.send_event(Event::PtyWrite(response));
    }

    /// DA: device attributes response.
    pub(super) fn status_identify_terminal(&mut self, intermediate: Option<char>) {
        match intermediate {
            None => {
                // DA1: report VT420-class terminal with ANSI color + sixel.
                // Format: CSI ? Pc ; Pp1 ; Pp2 c
                //   64 = VT420 conformance level (same as xterm)
                //   6  = selective erase / ANSI color
                //   4  = sixel graphics
                // vttest checks for 62+ (VT220+) to enable CSI 18t
                // size queries and other VT200+ features.
                let response = "\x1b[?64;6;4c".to_string();
                self.event_listener.send_event(Event::PtyWrite(response));
            }
            Some('>') => {
                // DA2: terminal type 0, version, conformance level 1.
                let version = crate_version_number();
                let response = format!("\x1b[>0;{version};1c");
                self.event_listener.send_event(Event::PtyWrite(response));
            }
            Some('=') => {
                // DA3: unit ID. DCS response: DCS ! | XXXXXXXX ST.
                // Eight zero digits as unit ID (same as xterm default).
                let response = "\x1bP!|00000000\x1b\\".to_string();
                self.event_listener.send_event(Event::PtyWrite(response));
            }
            Some(c) => debug!("Unsupported DA intermediate '{c}'"),
        }
    }

    /// DSR: device status report.
    pub(super) fn status_device_status(&mut self, arg: usize) {
        match arg {
            5 => {
                self.event_listener
                    .send_event(Event::PtyWrite("\x1b[0n".to_string()));
            }
            6 => {
                // Per DEC spec, when DECOM is active, DSR 6 reports the
                // cursor position relative to the scroll region origin.
                let abs_line = self.grid().cursor().line();
                let line = if self.mode.contains(TermMode::ORIGIN) {
                    abs_line.saturating_sub(self.grid().scroll_region().start) + 1
                } else {
                    abs_line + 1
                };
                let col = self.grid().cursor().col().0 + 1;
                let response = format!("\x1b[{line};{col}R");
                self.event_listener.send_event(Event::PtyWrite(response));
            }
            _ => debug!("Unknown device status query: {arg}"),
        }
    }

    /// CSI 18 t: report text area size in characters.
    pub(super) fn status_text_area_size_chars(&mut self) {
        let lines = self.grid().lines();
        let cols = self.grid().cols();
        let response = format!("\x1b[8;{lines};{cols}t");
        self.event_listener.send_event(Event::PtyWrite(response));
    }

    /// DECRQSS: Request Status String.
    ///
    /// Responds to DCS $ q ... ST queries. Each query type requests the
    /// current setting of a specific terminal feature. Valid responses
    /// use `DCS 1 $ r <value> ST`; invalid queries get `DCS 0 $ r ST`.
    pub(super) fn status_decrqss(&mut self, query: &[u8]) {
        let response = match query {
            // DECSCL: conformance level. Report VT400 level, 7-bit controls.
            b"\"p" => "\x1bP1$r64;1\"p\x1b\\".to_string(),
            // DECSTBM: scrolling region (top;bottom margins).
            b"r" => {
                let region = self.grid().scroll_region();
                let top = region.start + 1;
                let bottom = region.end;
                format!("\x1bP1$r{top};{bottom}r\x1b\\")
            }
            // SGR: current graphic rendition from cursor template.
            b"m" => {
                let t = self.grid().cursor().template();
                let sgr = build_sgr_string(t.flags, t.fg, t.bg);
                format!("\x1bP1$r{sgr}m\x1b\\")
            }
            // Unrecognized query: report invalid.
            _ => {
                debug!(
                    "Unrecognized DECRQSS query: {:?}",
                    String::from_utf8_lossy(query)
                );
                "\x1bP0$r\x1b\\".to_string()
            }
        };
        self.event_listener.send_event(Event::PtyWrite(response));
    }
}
