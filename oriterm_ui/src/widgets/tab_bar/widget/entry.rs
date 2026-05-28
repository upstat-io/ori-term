//! Per-tab visual state types for the tab bar widget.

use std::time::Instant;

/// Icon type for tab entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabIcon {
    /// Single emoji grapheme cluster.
    Emoji(String),
}

/// Per-tab visual state provided by the application layer.
#[derive(Debug, Clone)]
pub struct TabEntry {
    /// Tab title (empty string shows "Terminal" as fallback).
    pub title: String,
    /// Optional icon to show before the title.
    pub icon: Option<TabIcon>,
    /// When the bell last fired (for pulse animation). `None` if no bell.
    pub bell_start: Option<Instant>,
    /// Whether the tab content has been modified (shows accent dot).
    pub modified: bool,
    /// Whether the bell is currently active for this tab (shows persistent
    /// bell icon until the user focuses the tab). Sourced from the mux's
    /// `has_bell` query in `build_tab_entries`. Cleared via
    /// `mux.clear_bell` on tab focus change. Independent of `bell_start`'s
    /// 3-second pulse animation.
    pub has_bell: bool,
}

impl TabEntry {
    /// Creates a tab entry with the given title, no icon, and no bell.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            icon: None,
            bell_start: None,
            modified: false,
            has_bell: false,
        }
    }

    /// Sets the tab icon.
    #[must_use]
    pub fn with_icon(mut self, icon: Option<TabIcon>) -> Self {
        self.icon = icon;
        self
    }

    /// Sets the modified state (shows accent dot indicator).
    #[must_use]
    pub fn with_modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }

    /// Sets the persistent bell-icon state.
    #[must_use]
    pub fn with_bell(mut self, has_bell: bool) -> Self {
        self.has_bell = has_bell;
        self
    }
}
