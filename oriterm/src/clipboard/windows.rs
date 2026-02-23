//! Windows clipboard provider using `clipboard-win` for text, `arboard` for HTML.
//!
//! Plain text uses `clipboard-win` (stateless free functions). HTML format
//! uses `arboard` which handles the Windows `CF_HTML` format automatically.

use log::warn;

use super::{Clipboard, ClipboardProvider};

/// Windows clipboard provider.
///
/// Text operations use `clipboard-win` (lightweight, stateless). HTML
/// operations lazily create an `arboard::Clipboard` context because
/// `clipboard-win` doesn't support custom clipboard formats.
struct WindowsProvider {
    arboard: Option<arboard::Clipboard>,
}

impl WindowsProvider {
    fn new() -> Self {
        Self { arboard: None }
    }

    /// Get or create the arboard context for HTML operations.
    fn arboard(&mut self) -> Option<&mut arboard::Clipboard> {
        if self.arboard.is_none() {
            match arboard::Clipboard::new() {
                Ok(ctx) => self.arboard = Some(ctx),
                Err(e) => {
                    warn!("failed to create arboard context for HTML clipboard: {e}");
                    return None;
                }
            }
        }
        self.arboard.as_mut()
    }
}

impl ClipboardProvider for WindowsProvider {
    fn get_text(&mut self) -> Option<String> {
        clipboard_win::get_clipboard_string().ok()
    }

    fn set_text(&mut self, text: &str) -> bool {
        clipboard_win::set_clipboard_string(text).is_ok()
    }

    fn set_html(&mut self, html: &str, alt_text: &str) -> bool {
        if let Some(ctx) = self.arboard() {
            ctx.set_html(html, Some(alt_text)).is_ok()
        } else {
            self.set_text(alt_text)
        }
    }
}

/// Create a Windows clipboard (system clipboard only, no primary selection).
pub(super) fn create() -> Clipboard {
    Clipboard {
        clipboard: Box::new(WindowsProvider::new()),
        selection: None,
    }
}
