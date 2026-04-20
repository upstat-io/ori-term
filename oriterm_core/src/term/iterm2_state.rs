//! iTerm2 OSC 1337 non-image sub-op state.
//!
//! Holds the terminal-level state mutated by the non-image variants of
//! OSC 1337 (`RemoteHost=`, `SetUserVar=`, `ShellIntegrationVersion=`).
//! `SetMark=` and `CurrentDir=` reuse existing SSOT fields
//! (`prompt_markers` and `cwd` respectively) and therefore live in
//! `shell_state.rs` / `mod.rs`, not here.
//!
//! `user_vars` is bounded by `USER_VARS_MAX_ENTRIES` (default 256) with
//! FIFO (insertion-order) eviction via `IndexMap::shift_remove_index(0)`
//! so adversarial PTY output cannot grow RSS unboundedly — required by
//! `oriterm_core/tests/rss_regression.rs`. Re-inserting an existing key
//! replaces the value in place without refreshing the eviction position
//! (non-LRU semantic).

use indexmap::IndexMap;

use crate::effect::sink::EffectSink;

use super::Term;

/// Maximum number of `SetUserVar` entries retained by default.
pub(crate) const USER_VARS_MAX_ENTRIES: usize = 256;

/// iTerm2 OSC 1337 non-image sub-op state.
#[derive(Debug)]
pub(crate) struct Iterm2State {
    /// Remote-host identifier reported by OSC 1337 ; RemoteHost=user@host.
    pub(crate) remote_host: Option<String>,
    /// User variables reported by OSC 1337 ; SetUserVar=NAME=<base64>.
    pub(crate) user_vars: IndexMap<String, String>,
    /// Maximum entries retained in `user_vars`. Defaults to
    /// [`USER_VARS_MAX_ENTRIES`]; tests may override via
    /// [`Term::set_user_vars_max`].
    pub(crate) user_vars_max: usize,
    /// Shell-integration version reported by OSC 1337 ;
    /// ShellIntegrationVersion=N.
    pub(crate) shell_integration_version: Option<String>,
}

impl Iterm2State {
    pub(crate) fn new() -> Self {
        Self {
            remote_host: None,
            user_vars: IndexMap::new(),
            user_vars_max: USER_VARS_MAX_ENTRIES,
            shell_integration_version: None,
        }
    }

    /// Record a user-variable, enforcing FIFO eviction when at capacity.
    ///
    /// Re-inserting an existing key replaces the value without refreshing
    /// the eviction position (matches `IndexMap::insert` semantics) —
    /// `osc1337_user_vars_reinsert_does_not_refresh_position` pins this.
    ///
    /// A cap of zero disables storage entirely — insertions are dropped
    /// so the invariant `user_vars.len() <= user_vars_max` always holds.
    pub(crate) fn record_user_var(&mut self, name: String, value: String) {
        if self.user_vars_max == 0 {
            return;
        }
        if !self.user_vars.contains_key(&name) && self.user_vars.len() >= self.user_vars_max {
            self.user_vars.shift_remove_index(0);
        }
        self.user_vars.insert(name, value);
    }
}

impl<S: EffectSink> Term<S> {
    /// Remote host reported by OSC 1337 ; RemoteHost=user@host.
    pub fn remote_host(&self) -> Option<&str> {
        self.iterm2_state.remote_host.as_deref()
    }

    /// Value of an iTerm2 user variable reported by OSC 1337 ; `SetUserVar`.
    pub fn user_var(&self, name: &str) -> Option<&str> {
        self.iterm2_state.user_vars.get(name).map(String::as_str)
    }

    /// Number of user variables currently retained.
    pub fn user_vars_len(&self) -> usize {
        self.iterm2_state.user_vars.len()
    }

    /// Shell integration version reported by OSC 1337 ;
    /// ShellIntegrationVersion=N.
    pub fn shell_integration_version(&self) -> Option<&str> {
        self.iterm2_state.shell_integration_version.as_deref()
    }

    /// Cell width in pixels (set by the GUI once font metrics are known).
    ///
    /// Used by OSC 1337 ; `ReportCellSize` to build the PTY reply.
    pub fn cell_pixel_width(&self) -> u16 {
        self.cell_pixel_width
    }

    /// Cell height in pixels (set by the GUI once font metrics are known).
    pub fn cell_pixel_height(&self) -> u16 {
        self.cell_pixel_height
    }

    /// Override the `user_vars` FIFO cap.
    ///
    /// Exposed so the `osc1337_user_vars_reinsert_does_not_refresh_position`
    /// integration test can exercise eviction at cap = 3 without allocating
    /// 256 distinct base64 strings. Reducing the cap below the current map
    /// size trims the oldest-by-insertion-order entries immediately so the
    /// invariant `user_vars.len() <= user_vars_max` always holds.
    pub fn set_user_vars_max(&mut self, cap: usize) {
        while self.iterm2_state.user_vars.len() > cap {
            self.iterm2_state.user_vars.shift_remove_index(0);
        }
        self.iterm2_state.user_vars_max = cap;
    }
}

#[cfg(test)]
mod tests;
