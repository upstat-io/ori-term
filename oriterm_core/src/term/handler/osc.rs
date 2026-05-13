//! OSC (Operating System Command) handler implementations.
//!
//! Handles title management (OSC 0/1/2), color operations (OSC 4/10-12/104/110-112),
//! clipboard (OSC 52), hyperlinks (OSC 8), and the iTerm2 OSC 1337 non-image
//! sub-ops (`SetMark`, `RemoteHost`, `CurrentDir`, `Copy`, `ReportCellSize`,
//! `SetUserVar`, `ShellIntegrationVersion`). Methods are called by the
//! `vte::ansi::Handler` trait impl on `Term<S>`.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as Base64;
use log::debug;
use vte::ansi::{Hyperlink as VteHyperlink, NamedColor};

use crate::cell::Hyperlink;
use crate::color::Rgb;
use crate::effect::sink::EffectSink;
use crate::effect::{
    ClipboardSelection, Effect, HostEffect, HostRequest, PtyEffect, PtyWriteKind, ResponseToken,
};
use crate::term::PromptMarker;

use super::super::{TITLE_STACK_MAX_DEPTH, Term};

impl<S: EffectSink> Term<S> {
    /// OSC 0/2: set window title.
    pub(super) fn osc_set_title(&mut self, title: Option<String>) {
        if let Some(t) = &title {
            debug!("Setting title to '{t}'");
            self.title.clone_from(t);
            self.has_explicit_title = true;
        } else {
            debug!("Resetting title");
            self.title.clear();
            self.has_explicit_title = false;
        }
        self.title_dirty = true;
        self.effect_sink
            .push(Effect::Host(HostEffect::TitleSet { value: title }));
    }

    /// OSC 0/1: set icon name.
    pub(super) fn osc_set_icon_name(&mut self, name: Option<String>) {
        if let Some(n) = &name {
            debug!("Setting icon name to '{n}'");
            self.icon_name.clone_from(n);
        } else {
            debug!("Resetting icon name");
            self.icon_name.clear();
        }
        self.effect_sink
            .push(Effect::Host(HostEffect::IconNameSet { value: name }));
    }

    /// Push current title onto the title stack (xterm extension).
    pub(super) fn osc_push_title(&mut self) {
        debug!("Pushing title '{}'", self.title);
        if self.title_stack.len() >= TITLE_STACK_MAX_DEPTH {
            self.title_stack.pop_front();
        }
        self.title_stack.push_back(self.title.clone());
    }

    /// Pop title from the stack and set it (xterm extension).
    pub(super) fn osc_pop_title(&mut self) {
        if let Some(title) = self.title_stack.pop_back() {
            debug!("Popped title '{title}'");
            self.osc_set_title(Some(title));
        }
    }

    /// OSC 4/10/11/12: set a palette color by index.
    ///
    /// Marks all lines dirty when a non-cursor color changes, since any
    /// cell could reference the modified palette entry.
    pub(super) fn osc_set_color(&mut self, index: usize, color: Rgb) {
        debug!("Setting color[{index}] = {color:?}");
        self.palette.set_indexed(index, color);

        // Cursor color change doesn't require full redraw.
        if index != NamedColor::Cursor as usize {
            self.grid_mut().dirty_mut().mark_all();
        }
    }

    /// OSC 104/110/111/112: reset a palette color to its default.
    pub(super) fn osc_reset_color(&mut self, index: usize) {
        debug!("Resetting color[{index}]");
        self.palette.reset_indexed(index);

        if index != NamedColor::Cursor as usize {
            self.grid_mut().dirty_mut().mark_all();
        }
    }

    /// OSC 4/10/11/12 query: respond with the current color value.
    ///
    /// Resolves the color synchronously from `self.palette` and emits the
    /// reply as `Effect::Pty(PtyEffect::Write)` directly. This bypasses
    /// the `HostRequest::ColorQuery` coordinator roundtrip — Term has
    /// the full palette via `Palette::for_theme(self.theme)` plus any
    /// runtime OSC 4/104 mutations, so no GUI-thread lookup is needed.
    ///
    /// The synchronous emission is load-bearing for Windows ConPTY: the
    /// async coordinator path delays OSC replies past the child's
    /// initial-handshake polling window, causing notcurses-info to miss
    /// our color replies and fall back to ASCII rendering. Emitting
    /// inline in the same VTE-processing pass as the query keeps replies
    /// in lockstep with CSI/DCS replies (which already emit synchronously).
    pub(super) fn osc_dynamic_color_sequence(&self, prefix: &str, index: usize, terminator: &str) {
        debug!("Color query for index {index} (prefix={prefix})");
        let color = self.palette.color(index);
        let bytes = crate::effect::format_color_reply(color, prefix, terminator);
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes,
            kind: PtyWriteKind::Other,
        }));
    }

    /// OSC 52: store clipboard content (base64 encoded).
    pub(super) fn osc_clipboard_store(&self, clipboard: u8, base64: &[u8]) {
        let selection = match clipboard {
            b'c' => ClipboardSelection::Clipboard,
            b'p' => ClipboardSelection::Primary,
            b's' => ClipboardSelection::Select,
            _ => return,
        };

        let bytes = match Base64.decode(base64) {
            Ok(b) => b,
            Err(e) => {
                debug!("OSC 52: invalid base64: {e}");
                return;
            }
        };

        let text = match String::from_utf8(bytes) {
            Ok(t) => t,
            Err(e) => {
                debug!("OSC 52: invalid UTF-8: {e}");
                return;
            }
        };

        self.effect_sink
            .push(Effect::Host(HostEffect::ClipboardStore {
                selection,
                data: text,
            }));
    }

    /// OSC 52: request clipboard content.
    ///
    /// Emits a `HostRequest::ClipboardLoad` with the selection target,
    /// clipboard character, and terminator as plain data. The consumer
    /// fulfills the token; the reply formatter reconstructs the escape
    /// sequence with base64-encoded content.
    pub(super) fn osc_clipboard_load(&self, clipboard: u8, terminator: &str) {
        let selection = match clipboard {
            b'c' => ClipboardSelection::Clipboard,
            b'p' => ClipboardSelection::Primary,
            b's' => ClipboardSelection::Select,
            _ => return,
        };

        self.effect_sink
            .push(Effect::HostRequest(HostRequest::ClipboardLoad {
                selection,
                clipboard_char: clipboard,
                terminator: terminator.to_owned(),
                reply: ResponseToken::new(),
            }));
    }

    /// OSC 8: set or clear hyperlink on cursor template.
    pub(super) fn osc_set_hyperlink(&mut self, hyperlink: Option<VteHyperlink>) {
        debug!("Setting hyperlink: {hyperlink:?}");
        self.grid_mut()
            .cursor_mut()
            .template
            .set_hyperlink(hyperlink.map(Hyperlink::from));
    }

    /// OSC 1337 ; `SetMark` — iTerm2 navigation mark.
    ///
    /// Records the current cursor row as a `PromptMarker` (SSOT with
    /// OSC 133;A via the shared `prompt_markers` vec). Avoids duplicate
    /// markers at the same row (shell may re-emit on prompt redraw).
    pub(super) fn osc_iterm2_set_mark(&mut self) {
        let abs_row = self.grid.scrollback().len() + self.grid.cursor().line();
        if self
            .prompt_markers
            .last()
            .is_some_and(|m| m.prompt == abs_row)
        {
            return;
        }
        self.prompt_markers.push(PromptMarker {
            prompt: abs_row,
            command: None,
            output: None,
        });
    }

    /// OSC 1337 ; RemoteHost=user@host — record the remote-host identifier.
    pub(super) fn osc_iterm2_remote_host(&mut self, host: &[u8]) {
        let Ok(host) = std::str::from_utf8(host) else {
            debug!("OSC 1337 RemoteHost: invalid UTF-8");
            return;
        };
        self.iterm2_state.remote_host = Some(host.to_owned());
    }

    /// OSC 1337 ; CurrentDir=/path — update CWD (SSOT with OSC 7 / 133).
    pub(super) fn osc_iterm2_current_dir(&mut self, path: &[u8]) {
        let Ok(path) = std::str::from_utf8(path) else {
            debug!("OSC 1337 CurrentDir: invalid UTF-8");
            return;
        };
        self.set_cwd(Some(path.to_owned()));
    }

    /// OSC 1337 ; Copy=<selection>:<base64> — store the decoded text in the
    /// clipboard. Empty `<selection>` defaults to the system clipboard.
    pub(super) fn osc_iterm2_copy(&self, data: &[u8]) {
        let Some(sep) = data.iter().position(|&b| b == b':') else {
            debug!("OSC 1337 Copy: missing ':' separator");
            return;
        };
        let selection_bytes = &data[..sep];
        let base64 = &data[sep + 1..];
        let selection = if selection_bytes.is_empty() {
            ClipboardSelection::Clipboard
        } else {
            // Use the first recognized selection character; iTerm2 accepts a
            // multi-char mask but we route to a single selection slot.
            match selection_bytes[0] {
                b'c' => ClipboardSelection::Clipboard,
                b'p' => ClipboardSelection::Primary,
                b's' => ClipboardSelection::Select,
                _ => return,
            }
        };
        let bytes = match Base64.decode(base64) {
            Ok(b) => b,
            Err(e) => {
                debug!("OSC 1337 Copy: invalid base64: {e}");
                return;
            }
        };
        let text = match String::from_utf8(bytes) {
            Ok(t) => t,
            Err(e) => {
                debug!("OSC 1337 Copy: invalid UTF-8: {e}");
                return;
            }
        };
        self.effect_sink
            .push(Effect::Host(HostEffect::ClipboardStore {
                selection,
                data: text,
            }));
    }

    /// OSC 1337 ; `ReportCellSize` — reply with the current cell dimensions
    /// via PTY.
    ///
    /// iTerm2 wire format (per iTerm2 proprietary escape codes documentation,
    /// cross-checked against `wezterm-escape-parser/src/osc.rs:1351`):
    ///
    /// ```text
    /// OSC 1337 ; ReportCellSize=<H>;<W>[;<scale>] ST
    /// ```
    ///
    /// `H` and `W` are floating-point values (one decimal place) giving
    /// logical cell dimensions. `scale` is an optional third parameter
    /// (iTerm2 3.3+) reporting the backing-store scale factor; we omit it
    /// because `oriterm_core` has no DPI knowledge — only the GUI layer
    /// owns scale. Floats with a `.0` fractional part match `WezTerm`'s
    /// unscaled-emit shape and parse cleanly in consumers that expect
    /// numeric tokens (e.g., `imgcat`, `viu`).
    pub(super) fn osc_iterm2_report_cell_size(&self) {
        let reply = format!(
            "\x1b]1337;ReportCellSize={:.1};{:.1}\x1b\\",
            f32::from(self.cell_pixel_height),
            f32::from(self.cell_pixel_width),
        );
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: reply.into_bytes(),
            kind: PtyWriteKind::Other,
        }));
    }

    /// OSC 1337 ; SetUserVar=NAME=<base64> — record a user variable.
    ///
    /// Invalid base64 or non-UTF-8 values drop without mutating state.
    /// Bounded at [`crate::term::iterm2_state::USER_VARS_MAX_ENTRIES`] with
    /// FIFO (insertion-order) eviction.
    pub(super) fn osc_iterm2_set_user_var(&mut self, name: &[u8], value: &[u8]) {
        let Ok(name) = std::str::from_utf8(name) else {
            debug!("OSC 1337 SetUserVar: invalid UTF-8 in name");
            return;
        };
        let bytes = match Base64.decode(value) {
            Ok(b) => b,
            Err(e) => {
                debug!("OSC 1337 SetUserVar: invalid base64: {e}");
                return;
            }
        };
        let text = match String::from_utf8(bytes) {
            Ok(t) => t,
            Err(e) => {
                debug!("OSC 1337 SetUserVar: invalid UTF-8 in value: {e}");
                return;
            }
        };
        self.iterm2_state.record_user_var(name.to_owned(), text);
    }

    /// OSC 1337 ; ShellIntegrationVersion=N — record the shell-integration
    /// version string advertised by the shell.
    pub(super) fn osc_iterm2_shell_integration_version(&mut self, version: &[u8]) {
        let Ok(v) = std::str::from_utf8(version) else {
            debug!("OSC 1337 ShellIntegrationVersion: invalid UTF-8");
            return;
        };
        self.iterm2_state.shell_integration_version = Some(v.to_owned());
    }

    /// OSC 3 ; Pt — set or delete an X11 window property. `Pt` is
    /// `prop[=value]`; bare `prop` deletes the property.
    pub(super) fn osc_set_x11_property(&mut self, payload: &[u8]) {
        let Ok(payload) = std::str::from_utf8(payload) else {
            debug!("OSC 3: invalid UTF-8 payload");
            return;
        };
        if payload.is_empty() {
            return;
        }
        match payload.split_once('=') {
            Some((name, value)) => {
                if name.is_empty() {
                    return;
                }
                self.colors_state
                    .set_x11_property(name.to_owned(), value.to_owned());
            }
            None => {
                self.colors_state.delete_x11_property(payload);
            }
        }
    }

    /// OSC 5 ; Ps ; spec — set one of the special-color slots.
    pub(super) fn osc_set_special_color(&mut self, index: usize, color: Rgb) {
        if let Some(slot) = self.colors_state.special_colors.get_mut(index) {
            *slot = Some(color);
        }
    }

    /// OSC 5 ; Ps ; ? — reply with the current special-color slot value.
    pub(super) fn osc_query_special_color(&self, index: usize, terminator: &str) {
        let color = self
            .colors_state
            .special_colors
            .get(index)
            .copied()
            .flatten()
            .unwrap_or_else(|| self.palette.foreground());
        self.push_color_reply(format_args!("5;{index}"), color, terminator);
    }

    /// OSC 6 ; spec — set the iTerm2 tab title color.
    pub(super) fn osc_set_tab_title_color(&mut self, color: Rgb) {
        self.colors_state.tab_title_color = Some(color);
    }

    /// OSC 13 ; spec — set the mouse-cursor foreground color.
    pub(super) fn osc_set_mouse_fg_color(&mut self, color: Rgb) {
        self.colors_state.mouse_fg_color = Some(color);
    }

    /// OSC 14 ; spec — set the mouse-cursor background color.
    pub(super) fn osc_set_mouse_bg_color(&mut self, color: Rgb) {
        self.colors_state.mouse_bg_color = Some(color);
    }

    /// OSC 17 ; spec — set the selection highlight background color.
    pub(super) fn osc_set_highlight_bg_color(&mut self, color: Rgb) {
        self.colors_state.highlight_bg_color = Some(color);
    }

    /// OSC 19 ; spec — set the selection highlight foreground color.
    pub(super) fn osc_set_highlight_fg_color(&mut self, color: Rgb) {
        self.colors_state.highlight_fg_color = Some(color);
    }

    /// OSC 13 ; ? — reply with the current mouse-cursor foreground color.
    pub(super) fn osc_query_mouse_fg_color(&self, terminator: &str) {
        let color = self
            .colors_state
            .mouse_fg_color
            .unwrap_or_else(|| self.palette.foreground());
        self.push_color_reply(format_args!("13"), color, terminator);
    }

    /// OSC 14 ; ? — reply with the current mouse-cursor background color.
    pub(super) fn osc_query_mouse_bg_color(&self, terminator: &str) {
        let color = self
            .colors_state
            .mouse_bg_color
            .unwrap_or_else(|| self.palette.background());
        self.push_color_reply(format_args!("14"), color, terminator);
    }

    /// OSC 17 ; ? — reply with the current highlight background color.
    pub(super) fn osc_query_highlight_bg_color(&self, terminator: &str) {
        let color = self
            .colors_state
            .highlight_bg_color
            .unwrap_or_else(|| self.palette.background());
        self.push_color_reply(format_args!("17"), color, terminator);
    }

    /// OSC 19 ; ? — reply with the current highlight foreground color.
    pub(super) fn osc_query_highlight_fg_color(&self, terminator: &str) {
        let color = self
            .colors_state
            .highlight_fg_color
            .unwrap_or_else(|| self.palette.foreground());
        self.push_color_reply(format_args!("19"), color, terminator);
    }

    /// OSC 113 — reset mouse-cursor foreground color.
    pub(super) fn osc_reset_mouse_fg_color(&mut self) {
        self.colors_state.mouse_fg_color = None;
    }

    /// OSC 114 — reset mouse-cursor background color.
    pub(super) fn osc_reset_mouse_bg_color(&mut self) {
        self.colors_state.mouse_bg_color = None;
    }

    /// OSC 117 — reset selection highlight background color.
    pub(super) fn osc_reset_highlight_bg_color(&mut self) {
        self.colors_state.highlight_bg_color = None;
    }

    /// OSC 119 — reset selection highlight foreground color.
    pub(super) fn osc_reset_highlight_fg_color(&mut self) {
        self.colors_state.highlight_fg_color = None;
    }

    /// Push an `OSC <prefix> ; rgb:RRRR/GGGG/BBBB <terminator>` reply as a
    /// PTY write effect. Shared helper for OSC 5 / 13 / 14 / 17 / 19
    /// query paths. Uses four-hex-digit channel encoding to match
    /// xterm's reply format (`rgb:RRRR/GGGG/BBBB`).
    fn push_color_reply(&self, prefix: std::fmt::Arguments<'_>, color: Rgb, terminator: &str) {
        let r = u16::from(color.r) * 0x101;
        let g = u16::from(color.g) * 0x101;
        let b = u16::from(color.b) * 0x101;
        let reply = format!("\x1b]{prefix};rgb:{r:04x}/{g:04x}/{b:04x}{terminator}");
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: reply.into_bytes(),
            kind: PtyWriteKind::Other,
        }));
    }
}
