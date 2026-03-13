//! Windows Jump List integration.
//!
//! Provides the cross-platform `JumpListTask` data model and a
//! Windows-specific `submit_jump_list` function that registers tasks
//! in the taskbar right-click menu via COM.

/// A single entry in the taskbar Jump List.
///
/// Pure data — no platform dependency. Built and tested on all platforms.
/// Only consumed at runtime on Windows; on other platforms the struct
/// exists for tests.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) struct JumpListTask {
    /// Display name shown in the jump list (e.g., "New Window").
    pub label: String,
    /// Command-line arguments passed to the binary (e.g., "--new-window").
    pub arguments: String,
    /// Tooltip text shown on hover.
    pub description: String,
}

/// Build the default set of jump list tasks.
///
/// Returns the built-in tasks (New Window, New Tab). Profile entries
/// will be added once the profile system is implemented.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn build_jump_list_tasks() -> Vec<JumpListTask> {
    vec![JumpListTask {
        label: "New Window".to_owned(),
        arguments: "--new-window".to_owned(),
        description: "Open a new terminal window".to_owned(),
    }]
}

#[cfg(target_os = "windows")]
mod windows_impl;

#[cfg(target_os = "windows")]
pub(crate) use windows_impl::submit_jump_list;

#[cfg(test)]
mod tests;
