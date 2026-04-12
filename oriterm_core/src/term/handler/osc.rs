//! OSC (Operating System Command) handler implementations.
//!
//! Handles title management (OSC 0/1/2), color operations (OSC 4/10-12/104/110-112),
//! clipboard (OSC 52), and hyperlinks (OSC 8). Methods are called by the
//! `vte::ansi::Handler` trait impl on `Term<S>`.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as Base64;
use log::debug;
use vte::ansi::{Hyperlink as VteHyperlink, NamedColor};

use crate::cell::Hyperlink;
use crate::color::Rgb;
use crate::effect::sink::EffectSink;
use crate::effect::{ClipboardSelection, Effect, HostEffect, HostRequest, ResponseToken};

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
    /// Emits a `HostRequest::ColorQuery` with the prefix, index, and
    /// terminator as plain data. The consumer fulfills the token with the
    /// color value; the reply formatter reconstructs the escape sequence.
    pub(super) fn osc_dynamic_color_sequence(&self, prefix: &str, index: usize, terminator: &str) {
        debug!("Color query for index {index} (prefix={prefix})");
        self.effect_sink
            .push(Effect::HostRequest(HostRequest::ColorQuery {
                prefix: prefix.to_owned(),
                index,
                terminator: terminator.to_owned(),
                reply: ResponseToken::new(),
            }));
    }

    /// OSC 52: store clipboard content (base64 encoded).
    pub(super) fn osc_clipboard_store(&self, clipboard: u8, base64: &[u8]) {
        let selection = match clipboard {
            b'c' => ClipboardSelection::Clipboard,
            b'p' | b's' => ClipboardSelection::Primary,
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
            b'p' | b's' => ClipboardSelection::Primary,
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
}
