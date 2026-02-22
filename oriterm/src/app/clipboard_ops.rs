//! Copy, paste, and clipboard operations for the application.
//!
//! Implements clipboard writes from selection content, paste filtering,
//! bracketed paste, and OSC 52 clipboard integration. Keybinding dispatch
//! is handled by `keyboard_input.rs` via the binding table.

use std::path::PathBuf;

use oriterm_core::TermMode;
use oriterm_core::event::ClipboardType;
use oriterm_core::paste;
use oriterm_core::selection::extract_text;

use super::App;

impl App {
    /// Extract text from the active tab's selection.
    ///
    /// Returns `None` if there is no tab, no selection, or the selection is
    /// empty. Borrow of `self.tab` is confined to this method so callers can
    /// mutate `self.clipboard` after.
    fn extract_selection_text(&self) -> Option<String> {
        let tab = self.tab.as_ref()?;
        let sel = tab.selection()?;
        let term = tab.terminal().lock();
        let text = extract_text(term.grid(), sel);
        (!text.is_empty()).then_some(text)
    }

    /// Copy the active selection to the system clipboard.
    ///
    /// Returns `true` if text was copied.
    pub(crate) fn copy_selection(&mut self) -> bool {
        let Some(text) = self.extract_selection_text() else {
            return false;
        };
        self.clipboard.store(ClipboardType::Clipboard, &text);
        log::debug!("copied {} bytes to clipboard", text.len());
        true
    }

    /// Copy the active selection to the X11/Wayland primary selection.
    ///
    /// Called on mouse release after a drag selection. On Windows/macOS this
    /// is a no-op (the clipboard module silently ignores `Selection` stores
    /// when no primary selection provider is available).
    pub(crate) fn copy_selection_to_primary(&mut self) {
        if let Some(text) = self.extract_selection_text() {
            self.clipboard.store(ClipboardType::Selection, &text);
        }
    }

    /// Paste text from the system clipboard into the active terminal.
    ///
    /// Applies character filtering (if enabled), line ending normalization,
    /// ESC stripping (for bracketed paste), and bracketed paste wrapping.
    pub(crate) fn paste_from_clipboard(&mut self) {
        let text = self.clipboard.load(ClipboardType::Clipboard);
        if text.is_empty() {
            return;
        }

        let newlines = paste::count_newlines(&text);
        if newlines > 0 {
            log::debug!("pasting multi-line text ({} lines)", newlines + 1);
            // TODO(section-13): wire multi-line paste warning config.
        }

        self.write_paste_to_pty(&text);
    }

    /// Paste text from the primary selection (X11/Wayland middle-click paste).
    ///
    /// On Windows/macOS, the primary selection is typically empty (no-op).
    pub(crate) fn paste_from_primary(&mut self) {
        let text = self.clipboard.load(ClipboardType::Selection);
        if text.is_empty() {
            return;
        }
        self.write_paste_to_pty(&text);
        self.dirty = true;
    }

    /// Paste dropped file paths into the active terminal.
    ///
    /// Paths with spaces are auto-quoted. Multiple paths are space-separated.
    pub(crate) fn paste_dropped_files(&self, paths: &[PathBuf]) {
        if paths.is_empty() {
            return;
        }

        let text = paste::format_dropped_paths(paths);
        if text.is_empty() {
            return;
        }

        log::debug!("pasting {} dropped file path(s)", paths.len());
        self.write_paste_to_pty(&text);
    }

    /// Process and write paste text to the PTY.
    ///
    /// Reads the terminal mode to determine bracketed paste, applies the
    /// full paste processing pipeline, and writes the result to the PTY.
    fn write_paste_to_pty(&self, text: &str) {
        let Some(tab) = &self.tab else { return };

        let bracketed = tab
            .terminal()
            .lock()
            .mode()
            .contains(TermMode::BRACKETED_PASTE);
        // TODO(section-13): wire FilterOnPaste config setting. Default: enabled.
        let filter = true;

        let bytes = paste::prepare_paste(text, bracketed, filter);
        if bytes.is_empty() {
            return;
        }

        tab.scroll_to_bottom();
        tab.write_input(&bytes);
        log::debug!(
            "pasted {} bytes to PTY (bracketed={})",
            bytes.len(),
            bracketed
        );
    }
}
