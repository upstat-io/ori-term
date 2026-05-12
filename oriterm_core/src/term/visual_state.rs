//! Visual/presentation accessors for `Term<S>`.
//!
//! Groups accessors for state that affects rendering/presentation but is
//! orthogonal to the core grid + escape-sequence-handling surface:
//!
//! - Theme + palette + effective background (DECSCNM-aware).
//! - Bold-as-bright behavior.
//! - Cursor shape (config-driven override).
//! - Mouse cursor icon (OSC 22).
//! - Answerback string (ENQ response).
//! - Last command line (VS Code OSC 633 ; E).
//!
//! Backing fields live on `Term<S>` in `mod.rs`; this module groups the
//! accessor surface so `mod.rs` stays under the `code-hygiene.md §File Size`
//! cap.

use vte::ansi::cursor_icon::CursorIcon;

use crate::color::Palette;
use crate::effect::sink::EffectSink;
use crate::effect::{Effect, PtyEffect, PtyWriteKind};
use crate::grid::CursorShape;
use crate::theme::Theme;

use super::{Term, TermMode};

impl<S: EffectSink> Term<S> {
    /// Set the answerback string emitted on `ENQ` (`0x05`).
    ///
    /// Empty (default) suppresses emission entirely per ECMA-48 §8.3.40
    /// and the `WezTerm` reference behavior. Non-empty bytes are written
    /// to the PTY verbatim on each `ENQ` byte received.
    pub fn set_answerback(&mut self, bytes: Vec<u8>) {
        self.answerback = bytes;
    }

    /// Mouse cursor icon requested by the shell (OSC 22).
    ///
    /// `None` if no OSC 22 sequence has been received. The renderer reads
    /// this to update the OS pointer shape while hovering the terminal.
    pub fn mouse_cursor_icon(&self) -> Option<CursorIcon> {
        self.mouse_cursor_icon
    }

    /// Override the mouse cursor icon (raw interceptor / OSC 22 path).
    pub fn set_mouse_cursor_icon(&mut self, icon: Option<CursorIcon>) {
        self.mouse_cursor_icon = icon;
    }

    /// Last command line reported by VS Code OSC 633;E.
    ///
    /// `None` until the shell integration sends OSC 633;E with the raw typed
    /// command. The `E` sub-command is interceptor-only — it is NOT routed
    /// through the `Handler` trait because the high-level `vte::ansi::Processor`
    /// does not dispatch OSC 633.
    pub fn last_command_line(&self) -> Option<&str> {
        self.last_command_line.as_deref()
    }

    /// Record the raw command line text (OSC 633;E path, interceptor-only).
    pub fn set_last_command_line(&mut self, line: Option<String>) {
        self.last_command_line = line;
    }

    /// Whether bold text promotes ANSI colors 0–7 to bright 8–15.
    pub fn bold_is_bright(&self) -> bool {
        self.bold_is_bright
    }

    /// Set the bold-as-bright behavior.
    pub fn set_bold_is_bright(&mut self, enabled: bool) {
        self.bold_is_bright = enabled;
    }

    /// Reference to the color palette.
    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Mutable reference to the color palette (for config overrides).
    pub fn palette_mut(&mut self) -> &mut Palette {
        &mut self.palette
    }

    /// Terminal's effective background color, accounting for DECSCNM
    /// (reverse video, mode 5). When `REVERSE_VIDEO` is active, the
    /// effective bg is the foreground slot — matching the snapshot-time
    /// `Palette::swap_fg_bg` at `Term::renderable_content` so consumers
    /// that need to snapshot the "background in effect at this moment"
    /// (e.g. sixel P2=2 per DEC STD 070 §6.2.2) do not diverge from the
    /// render path when DECSCNM is active.
    pub fn effective_background(&self) -> vte::ansi::Rgb {
        if self.mode.contains(TermMode::REVERSE_VIDEO) {
            self.palette.foreground()
        } else {
            self.palette.background()
        }
    }

    /// Current color theme.
    pub fn theme(&self) -> Theme {
        self.theme
    }

    /// Switch the active color theme.
    ///
    /// Rebuilds the palette for the new theme and marks all lines dirty so
    /// the renderer repaints with the new colors. No-op if the theme is
    /// unchanged.
    pub fn set_theme(&mut self, theme: Theme) {
        if self.theme == theme {
            return;
        }
        self.theme = theme;
        self.palette = Palette::for_theme(theme);
        self.grid_mut().dirty_mut().mark_all();

        // Mode 2031: notify child process of color scheme change.
        if self.mode.contains(TermMode::COLOR_SCHEME_UPDATE) {
            let notification = match theme {
                Theme::Dark => Some(b"\x1b[?997;1n".as_slice()),
                Theme::Light => Some(b"\x1b[?997;2n".as_slice()),
                Theme::Unknown => None,
            };
            if let Some(bytes) = notification {
                self.effect_sink.push(Effect::Pty(PtyEffect::Write {
                    bytes: bytes.to_vec(),
                    kind: PtyWriteKind::Other,
                }));
            }
        }
    }

    /// Current cursor shape.
    pub fn cursor_shape(&self) -> CursorShape {
        self.cursor_shape
    }

    /// Override the cursor shape (config-driven, not VTE-driven).
    pub fn set_cursor_shape(&mut self, shape: CursorShape) {
        self.cursor_shape = shape;
    }
}
