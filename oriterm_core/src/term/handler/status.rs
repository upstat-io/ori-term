//! CSI device status and mode reporting handlers.
//!
//! DA (device attributes), DSR (device status report), DECRQM (mode report),
//! and CSI t (text area size). Methods are called by the `vte::ansi::Handler`
//! trait impl on `Term<S>`.
//!
//! All methods take `&mut self` because the `Handler` trait requires it,
//! even though these only read state and send events.

use log::{debug, info};
use vte::ansi::{Color, Mode, NamedColor, NamedMode, PrivateMode};

use crate::cell::CellFlags;
use crate::effect::sink::EffectSink;
use crate::effect::{Effect, PtyEffect, PtyWriteKind};
use crate::grid::CursorShape;
use crate::term::{Term, TermMode};

use super::helpers::{crate_version_number, mode_report_value, named_private_mode_flag};

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
    if flags.contains(CellFlags::OVERLINE) {
        params.push("53".to_string());
    }
    if flags.contains(CellFlags::SUPERSCRIPT) {
        params.push("73".to_string());
    }
    if flags.contains(CellFlags::SUBSCRIPT) {
        params.push("74".to_string());
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
impl<S: EffectSink> Term<S> {
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
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: response.into_bytes(),
            kind: PtyWriteKind::ModeReport,
        }));
    }

    /// DECRQM: report DEC private mode status.
    pub(super) fn status_report_private_mode(&mut self, mode: PrivateMode) {
        let (num, value) = match mode {
            PrivateMode::Named(named) => {
                let num = named as u16;
                let flag = named_private_mode_flag(named);
                let value = flag.map_or(0, |f| mode_report_value(self.mode.contains(f)));
                (num, value)
            }
            PrivateMode::Unknown(n) => (n, 0),
        };
        let response = format!("\x1b[?{num};{value}$y");
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: response.into_bytes(),
            kind: PtyWriteKind::ModeReport,
        }));
    }

    /// XTVERSION (CSI > q): report terminal name and version per xterm.
    ///
    /// Reply format: `DCS > | oriterm(<version>) ST`.
    ///
    /// The Ps=0 gate is enforced at the parser level
    /// (`crates/vte/src/ansi/dispatch/csi/mod.rs`) per xterm
    /// `charproc.c::CASE_REPORT_VERSION` semantics; this handler runs
    /// only for default/zero Ps and emits the constant reply.
    pub(super) fn status_xtversion(&mut self) {
        let version = env!("CARGO_PKG_VERSION");
        let response = format!("\x1bP>|oriterm({version})\x1b\\");
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: response.into_bytes(),
            kind: PtyWriteKind::DeviceAttribute,
        }));
    }

    /// DA: device attributes response.
    pub(super) fn status_identify_terminal(&mut self, intermediate: Option<char>) {
        match intermediate {
            None => {
                // DA1: report VT420-class terminal with ANSI color + sixel.
                // Format: CSI ? Pc ; Pp1 ; Pp2 c
                // 64 = VT420 conformance level (same as xterm)
                // 6 = selective erase / ANSI color
                // 4 = sixel graphics
                // vttest checks for 62+ (VT220+) to enable CSI 18t
                // size queries and other VT200+ features.
                let response = "\x1b[?64;6;4c";
                self.effect_sink.push(Effect::Pty(PtyEffect::Write {
                    bytes: response.as_bytes().to_vec(),
                    kind: PtyWriteKind::DeviceAttribute,
                }));
            }
            Some('>') => {
                // DA2: terminal type 0, version, conformance level 1.
                let version = crate_version_number();
                let response = format!("\x1b[>0;{version};1c");
                self.effect_sink.push(Effect::Pty(PtyEffect::Write {
                    bytes: response.into_bytes(),
                    kind: PtyWriteKind::DeviceAttribute,
                }));
            }
            Some('=') => {
                // DA3: unit ID. DCS response: DCS ! | XXXXXXXX ST.
                // Eight zero digits as unit ID (same as xterm default).
                let response = "\x1bP!|00000000\x1b\\";
                self.effect_sink.push(Effect::Pty(PtyEffect::Write {
                    bytes: response.as_bytes().to_vec(),
                    kind: PtyWriteKind::DeviceAttribute,
                }));
            }
            Some(c) => debug!("Unsupported DA intermediate '{c}'"),
        }
    }

    /// DSR: device status report.
    pub(super) fn status_device_status(&mut self, arg: usize) {
        match arg {
            5 => {
                self.effect_sink.push(Effect::Pty(PtyEffect::Write {
                    bytes: b"\x1b[0n".to_vec(),
                    kind: PtyWriteKind::DeviceStatus,
                }));
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
                self.effect_sink.push(Effect::Pty(PtyEffect::Write {
                    bytes: response.into_bytes(),
                    kind: PtyWriteKind::CursorReport,
                }));
            }
            _ => debug!("Unknown device status query: {arg}"),
        }
    }

    /// CSI 18 t: report text area size in characters.
    pub(super) fn status_text_area_size_chars(&mut self) {
        let lines = self.grid().lines();
        let cols = self.grid().cols();
        let response = format!("\x1b[8;{lines};{cols}t");
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: response.into_bytes(),
            kind: PtyWriteKind::Other,
        }));
    }

    /// XTSMGRAPHICS reply per xterm ctlseqs / `charproc.c:5153-5279`.
    ///
    /// Reply format: `CSI ? Pi ; Ps [; Pv [; Pv2]] S`. `Pv` fields are
    /// emitted only on `Ps=0` (success).
    ///
    /// - `Pi=1`: number of color registers (single `Pv`).
    /// - `Pi=2`: sixel graphics geometry (`Pv=width`, `Pv2=height`).
    /// - `Pi=3`: `ReGIS` graphics geometry — unsupported, always `Ps=3`.
    ///
    /// `Pa` semantics:
    /// - `Pa=1` read, `Pa=2` reset, `Pa=3` set, `Pa=4` read max.
    /// - `Pi=1 Pa=3` succeeds only when `1 < Pv <= COLOR_REGISTERS_MAX`.
    /// - `Pi=2 Pa=2/3` always `Ps=3` (xterm refuses geometry mutation).
    /// - Unknown `Pi` → `Ps=1`; unknown `Pa` → `Ps=2`.
    /// - When `image_protocol_enabled == false` for `Pi=1` or `Pi=2`,
    ///   success replies downgrade to `Ps=3` and drop the `Pv` field
    ///   (xterm `charproc.c:5198-5200` Pi=1 + `:5226-5227` Pi=2 sixel
    ///   gate).
    pub(super) fn status_graphics_attribute(&mut self, pi: u16, pa: u16, pv: u16) {
        let max = crate::image::sixel::COLOR_REGISTERS_MAX;
        let mut status: u16 = 3;
        let mut result: Option<u32> = None;
        let mut result2: Option<u32> = None;

        // Capability gate per xterm `charproc.c:5198-5200` (Pi=1) +
        // `:5226-5227` (Pi=2): the gate sits at the TOP of each item's
        // block, BEFORE any Pa dispatch. When sixel is disabled the Pa
        // branches NEVER execute, so neither state mutation nor Pv-field
        // emission can leak — status stays 3 (failure default). oriterm
        // gates all image-protocol handling on `image_protocol_enabled`
        // (peer pattern at `term/handler/image/sixel.rs:28`,
        // `iterm2.rs:35`, `kitty/mod.rs:61`). Pi=3 is unaffected
        // (already always status=3 since ReGIS isn't implemented).
        let sixel_enabled = self.image_protocol_enabled;
        match pi {
            1 if sixel_enabled => match pa {
                // Pa=1 read: reflect the CURRENT count (may have been
                // mutated by a prior Pa=3 set).
                1 => {
                    status = 0;
                    result = Some(u32::from(self.color_register_count));
                }
                // Pa=2 reset: restore default and reply with new value.
                2 => {
                    self.color_register_count = max;
                    status = 0;
                    result = Some(u32::from(self.color_register_count));
                }
                // Pa=3 set: mutate iff 1 < pv <= MAX, then reply with
                // the SET state per xterm `charproc.c:5181-5185`.
                3 => {
                    if pv > 1 && pv <= max {
                        self.color_register_count = pv;
                        status = 0;
                        result = Some(u32::from(self.color_register_count));
                    }
                    // else: status stays 3 (failure)
                }
                // Pa=4 read max: always MAX, regardless of any prior set.
                4 => {
                    status = 0;
                    result = Some(u32::from(max));
                }
                _ => status = 2,
            },
            2 if sixel_enabled => match pa {
                // For oriterm, current geometry == max geometry — no
                // separate max distinct from the current grid. xterm
                // separates them via `screen->graphics_max_wide`;
                // oriterm dynamically tracks the current grid pixel size.
                1 | 4 => {
                    let grid = self.grid();
                    status = 0;
                    result = Some(grid.cols() as u32 * u32::from(self.cell_pixel_width));
                    result2 = Some(grid.lines() as u32 * u32::from(self.cell_pixel_height));
                }
                // Pa=2 reset / Pa=3 set: xterm rejects geometry mutation
                // (empty block in xterm, status stays 3).
                2 | 3 => {}
                _ => status = 2,
            },
            // Pi=1 / Pi=2 with sixel disabled: gate falls through to here
            // — no state mutation, no Pa dispatch, status stays 3.
            1 | 2 => {}
            3 => match pa {
                1..=4 => {} // ReGIS unsupported regardless of Pa.
                _ => status = 2,
            },
            _ => status = 1,
        }

        debug_assert!(
            status == 0 || (result.is_none() && result2.is_none()),
            "non-zero status MUST NOT carry Pv fields per xterm `charproc.c:5267-5271`"
        );

        let response = if status == 0 {
            match (result, result2) {
                (Some(r1), Some(r2)) => format!("\x1b[?{pi};{status};{r1};{r2}S"),
                (Some(r1), None) => format!("\x1b[?{pi};{status};{r1}S"),
                _ => format!("\x1b[?{pi};{status}S"),
            }
        } else {
            format!("\x1b[?{pi};{status}S")
        };

        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: response.into_bytes(),
            kind: PtyWriteKind::GraphicsAttributeReport,
        }));
    }

    /// DECRQSS: Request Status String.
    ///
    /// Responds to DCS $ q... ST queries. Each query type requests the
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
            // DECSLRM: left/right margins.
            b"s" => {
                let (left, right) = self.grid().left_right_margins();
                format!("\x1bP1$r{};{}s\x1b\\", left + 1, right + 1)
            }
            // DECSCUSR: cursor style. Report shape + blink per xterm mapping:
            // 1 = blink block, 2 = steady block,
            // 3 = blink underline, 4 = steady underline,
            // 5 = blink bar, 6 = steady bar.
            b" q" => {
                let blinking = self.mode.contains(TermMode::CURSOR_BLINKING);
                let ps = decscusr_param(self.cursor_shape, blinking);
                format!("\x1bP1$r{ps} q\x1b\\")
            }
            // DECSCA: character protection attribute.
            // 1 = protected, 2 = unprotected (default per xterm).
            b"\"q" => {
                let ps = if self.char_protection { 1 } else { 2 };
                format!("\x1bP1$r{ps}\"q\x1b\\")
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
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: response.into_bytes(),
            kind: PtyWriteKind::StatusString,
        }));
    }

    /// DECRSPS: Restore Presentation Status (DCS Ps $ t Pt ST).
    ///
    /// Parse-and-acknowledge stub. Full state restoration (DECCIR cursor
    /// info and DECTABSR tab stops) is not yet implemented. The handler
    /// logs the request so PTY traffic remains observable via the log
    /// sink without mutating terminal state or emitting any reply —
    /// DECRSPS has no acknowledgement per the xterm spec.
    #[expect(
        clippy::unused_self,
        reason = "stub — will mutate presentation state when Ps=1/Ps=2 restoration lands"
    )]
    pub(super) fn status_decrsps(&mut self, ps: u16, pt: &[u8]) {
        info!(
            "DECRSPS stub: Ps={ps}, Pt bytes={} (state restoration not implemented)",
            pt.len()
        );
    }
}

/// Map `(CursorShape, blinking)` to the DECSCUSR Ps parameter per xterm.
///
/// Returned values mirror the DECSCUSR *setter* accepted in
/// `crates/vte/src/ansi/types.rs`:
/// 1 = blink block, 2 = steady block,
/// 3 = blink underline, 4 = steady underline,
/// 5 = blink bar (Beam), 6 = steady bar.
///
/// `HollowBlock` and `Hidden` are ori-extensions with no DECSCUSR encoding;
/// they report as the nearest steady block (Ps=2) so consumers receive a
/// well-formed reply.
fn decscusr_param(shape: CursorShape, blinking: bool) -> u16 {
    match shape {
        CursorShape::Block | CursorShape::HollowBlock | CursorShape::Hidden => {
            if blinking {
                1
            } else {
                2
            }
        }
        CursorShape::Underline => {
            if blinking {
                3
            } else {
                4
            }
        }
        CursorShape::Bar => {
            if blinking {
                5
            } else {
                6
            }
        }
    }
}
