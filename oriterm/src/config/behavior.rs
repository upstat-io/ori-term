//! Behavior configuration for user interactions and command notifications.

use serde::{Deserialize, Serialize};

use super::paste_warning::PasteWarning;

/// When to send desktop notifications on long-running command completion.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum NotifyOnCommandFinish {
    /// Never send command completion notifications.
    Never,
    /// Notify only when the pane is not focused (default).
    #[default]
    Unfocused,
    /// Always notify, even when the pane is focused.
    Always,
}

/// User interaction behavior configuration.
#[allow(
    clippy::struct_excessive_bools,
    reason = "config toggles are naturally boolean"
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct BehaviorConfig {
    /// Auto-copy on selection release (default: true).
    pub copy_on_select: bool,
    /// Bold text uses bright colors (default: true).
    pub bold_is_bright: bool,
    /// Enable shell integration injection (default: true).
    pub shell_integration: bool,
    /// Include HTML/RTF formatting when copying selection to clipboard.
    ///
    /// When `true`, clipboard copy places both plain text and HTML with inline
    /// styles (colors, bold, italic, underline) so pasting into rich text
    /// editors preserves terminal formatting. Default: `false`.
    pub copy_formatting: bool,
    /// Characters that act as word boundaries for double-click selection.
    ///
    /// When double-clicking, the selection expands to include all contiguous
    /// characters that are NOT in this set. For example, if `-` is not in the
    /// delimiter set, `hello-world` selects as one word.
    pub word_delimiters: String,
    /// Filter special characters from pasted text (default: true).
    pub filter_on_paste: bool,
    /// Warn before pasting multi-line text.
    ///
    /// `"always"` (any newline), `"never"`, or a number N (warn if >= N lines).
    pub warn_on_paste: PasteWarning,
    /// When to send a desktop notification on long-running command completion.
    ///
    /// `"never"` — disabled. `"unfocused"` — only when the pane is not focused
    /// (default). `"always"` — notify even when focused.
    pub notify_on_command_finish: NotifyOnCommandFinish,
    /// Minimum command duration (seconds) to trigger completion notification.
    pub notify_command_threshold_secs: u64,
    /// Flash the tab bar when a long-running command completes (reuses bell pulse).
    pub notify_command_bell: bool,
    /// Show visual prompt markers in the left margin at prompt lines (OSC 133;A).
    ///
    /// When `true`, a thin colored bar is drawn at the left edge of each prompt
    /// line. Requires shell integration to be active. Default: `false`.
    pub prompt_markers: bool,
    /// Hide the mouse cursor while typing in the terminal grid.
    ///
    /// The cursor reappears on any mouse movement. Does not hide when the
    /// terminal has mouse reporting enabled (e.g., vim, tmux mouse mode).
    /// Default: `true`.
    pub hide_mouse_when_typing: bool,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            copy_on_select: true,
            bold_is_bright: true,
            shell_integration: true,
            copy_formatting: false,
            word_delimiters: oriterm_core::DEFAULT_WORD_DELIMITERS.to_owned(),
            filter_on_paste: true,
            warn_on_paste: PasteWarning::default(),
            notify_on_command_finish: NotifyOnCommandFinish::default(),
            notify_command_threshold_secs: 10,
            notify_command_bell: true,
            prompt_markers: false,
            hide_mouse_when_typing: true,
        }
    }
}
