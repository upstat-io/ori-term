//! Type definitions consumed by the [`MuxBackend`](super::MuxBackend) trait.
//!
//! Extracted from `backend/mod.rs` to keep that file under the 500-line
//! source-file budget. Trait definitions themselves stay in `mod.rs`; this
//! sibling owns the supporting struct/enum types.

use oriterm_core::Theme;
use oriterm_core::color::Rgb;
use oriterm_core::effect::ResponseToken;

/// Payload for [`MuxBackend::fulfill_host_request`](super::MuxBackend::fulfill_host_request).
///
/// Carries the `ResponseToken` the main thread extracted from a
/// `MuxNotification::HostClipboardLoad` / `HostColorQuery`, paired with
/// the value the main thread resolved (clipboard text, palette color).
/// The embedded backend forwards the fulfillment to the owning pane's
/// `PaneIoHandle`; the daemon backend rejects today and will gain a
/// reply-PDU wire in a follow-up plan.
#[derive(Debug)]
pub enum HostReply {
    /// Reply to an OSC 52 clipboard read request.
    ClipboardLoad {
        /// Token carried by the originating notification.
        token: ResponseToken<String>,
        /// Clipboard text read by the main thread.
        text: String,
    },
    /// Reply to an OSC color query.
    ColorQuery {
        /// Token carried by the originating notification.
        token: ResponseToken<Rgb>,
        /// Resolved `Rgb` value.
        color: Rgb,
    },
}

/// Image protocol configuration for a pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageConfig {
    /// Whether image protocols are enabled.
    pub enabled: bool,
    /// CPU-side image cache memory limit in bytes.
    pub memory_limit: usize,
    /// Maximum single image size in bytes.
    pub max_single: usize,
    /// Whether animated images play their frames.
    pub animation_enabled: bool,
}

/// Per-pane parameters for [`MuxBackend::adopt_pane`](super::MuxBackend::adopt_pane).
///
/// Bundles the terminal dimensions, scrollback size, theme, and any
/// startup metadata so the trait method stays under the hygiene rule's
/// argument limit. The `AdoptedPtyHandle` is passed separately because
/// callers typically own it via move semantics already.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptPaneRequest {
    /// Initial terminal rows (typically from `TERMINAL_STARTUP_INFO.dwYCountChars`).
    pub rows: u16,
    /// Initial terminal columns (typically from `TERMINAL_STARTUP_INFO.dwXCountChars`).
    pub cols: u16,
    /// Scrollback buffer size in lines (from the user's config).
    pub scrollback: usize,
    /// Color theme for the new pane.
    pub theme: Theme,
    /// Initial pane title (typically from `TERMINAL_STARTUP_INFO.pszTitle`,
    /// e.g. the title embedded in a `.lnk` shortcut). Empty string means
    /// "no explicit title" — the pane falls back to its CWD-derived or
    /// shell-set title via the standard `Pane::effective_title` chain.
    pub initial_title: String,
    /// Initial pane icon name/path (typically from
    /// `TERMINAL_STARTUP_INFO.pszIconPath`, e.g. the icon embedded in a
    /// `.lnk` shortcut). `None` if the COM caller did not supply one.
    /// Stored on the pane via `Pane::set_icon_name` and consumed by the
    /// tab bar to render a per-pane icon.
    pub initial_icon: Option<String>,
}
