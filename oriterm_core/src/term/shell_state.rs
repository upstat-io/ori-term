//! Shell integration state accessors and navigation.
//!
//! Extracted from `term/mod.rs` to keep the main file under the 500-line
//! limit. These methods manage prompt state (OSC 133), CWD (OSC 7),
//! title resolution, notifications, and prompt-based navigation.

use super::{Notification, PromptState, Term, cwd_short_path};
use crate::event::EventListener;

impl<T: EventListener> Term<T> {
    // -- Prompt state --

    /// Current shell integration prompt state (OSC 133).
    pub fn prompt_state(&self) -> PromptState {
        self.prompt_state
    }

    /// Mutable reference to prompt state (for raw interceptor).
    pub fn prompt_state_mut(&mut self) -> &mut PromptState {
        &mut self.prompt_state
    }

    /// Whether OSC 133;A was received and the prompt row hasn't been marked yet.
    pub fn prompt_mark_pending(&self) -> bool {
        self.prompt_mark_pending
    }

    /// Set/clear the prompt-mark-pending flag.
    pub fn set_prompt_mark_pending(&mut self, pending: bool) {
        self.prompt_mark_pending = pending;
    }

    /// Record the current cursor row as a prompt line.
    ///
    /// Called after both VTE parsers finish processing a chunk, when
    /// `prompt_mark_pending` is `true`. Uses the cursor row from the
    /// high-level processor (which is at the correct position).
    pub fn mark_prompt_row(&mut self) {
        if !self.prompt_mark_pending {
            return;
        }
        self.prompt_mark_pending = false;
        let abs_row = self.grid.scrollback().len() + self.grid.cursor().line();
        // Avoid duplicate entries (e.g. shell redrawing prompt on resize).
        if self.prompt_rows.last() != Some(&abs_row) {
            self.prompt_rows.push(abs_row);
        }
    }

    /// Absolute row indices of prompt lines (OSC 133;A positions).
    pub fn prompt_rows(&self) -> &[usize] {
        &self.prompt_rows
    }

    /// Prune prompt rows evicted from scrollback.
    ///
    /// When scrollback lines are evicted (the buffer is full and new lines
    /// push old ones out), prompt row indices below the eviction threshold
    /// become invalid and must be removed. Remaining indices are shifted
    /// down by the eviction count.
    pub fn prune_prompt_rows(&mut self, evicted: usize) {
        if evicted == 0 {
            return;
        }
        self.prompt_rows.retain_mut(|row| {
            if *row < evicted {
                false
            } else {
                *row -= evicted;
                true
            }
        });
    }

    // -- Command timing --

    /// Record command execution start (when OSC 133;C is received).
    pub fn set_command_start(&mut self, start: std::time::Instant) {
        self.command_start = Some(start);
    }

    /// Compute and store command duration (when OSC 133;D is received).
    ///
    /// Returns the duration if a matching start time existed.
    pub fn finish_command(&mut self) -> Option<std::time::Duration> {
        let start = self.command_start.take()?;
        let duration = start.elapsed();
        self.last_command_duration = Some(duration);
        Some(duration)
    }

    /// Duration of the last completed command.
    pub fn last_command_duration(&self) -> Option<std::time::Duration> {
        self.last_command_duration
    }

    // -- Notifications --

    /// Drain pending desktop notifications (OSC 9/99/777).
    pub fn drain_notifications(&mut self) -> Vec<Notification> {
        std::mem::take(&mut self.pending_notifications)
    }

    /// Push a notification from the raw interceptor.
    pub fn push_notification(&mut self, notification: Notification) {
        self.pending_notifications.push(notification);
    }

    // -- Title state --

    /// Whether the current title was explicitly set via OSC 0/2.
    pub fn has_explicit_title(&self) -> bool {
        self.has_explicit_title
    }

    /// Set the explicit title flag.
    pub fn set_has_explicit_title(&mut self, explicit: bool) {
        self.has_explicit_title = explicit;
    }

    /// Whether the title needs refreshing (CWD or explicit title changed).
    pub fn is_title_dirty(&self) -> bool {
        self.title_dirty
    }

    /// Clear the title dirty flag after the UI has refreshed.
    pub fn clear_title_dirty(&mut self) {
        self.title_dirty = false;
    }

    /// Mark the title as needing a refresh.
    pub fn mark_title_dirty(&mut self) {
        self.title_dirty = true;
    }

    /// Mutable reference to CWD (for raw interceptor).
    pub fn cwd_mut(&mut self) -> &mut Option<String> {
        &mut self.cwd
    }

    /// Resolved display title with 3-source priority:
    /// 1. Explicit title from OSC 0/2.
    /// 2. Last component of CWD path.
    /// 3. Fallback to raw title (may be empty).
    pub fn effective_title(&self) -> &str {
        if self.has_explicit_title {
            return &self.title;
        }
        if let Some(ref cwd) = self.cwd {
            return cwd_short_path(cwd);
        }
        &self.title
    }

    // -- Prompt navigation --

    /// Scroll to the nearest prompt row above the current viewport position.
    ///
    /// Returns `true` if the viewport was scrolled, `false` if there are no
    /// prompts above (no-op).
    pub fn scroll_to_previous_prompt(&mut self) -> bool {
        if self.prompt_rows.is_empty() {
            return false;
        }
        // Current viewport top in absolute row coordinates.
        let sb_len = self.grid.scrollback().len();
        let viewport_top = sb_len.saturating_sub(self.grid.display_offset());
        // Find the last prompt row strictly above viewport top.
        let target = self
            .prompt_rows
            .iter()
            .rev()
            .find(|&&row| row < viewport_top);
        if let Some(&row) = target {
            self.scroll_to_absolute_row(row);
            true
        } else {
            false
        }
    }

    /// Scroll to the nearest prompt row below the current viewport position.
    ///
    /// Returns `true` if the viewport was scrolled, `false` if there are no
    /// prompts below (no-op).
    pub fn scroll_to_next_prompt(&mut self) -> bool {
        if self.prompt_rows.is_empty() {
            return false;
        }
        let sb_len = self.grid.scrollback().len();
        // Current viewport bottom in absolute row coordinates.
        let viewport_bottom = sb_len.saturating_sub(self.grid.display_offset()) + self.grid.lines();
        let target = self.prompt_rows.iter().find(|&&row| row >= viewport_bottom);
        if let Some(&row) = target {
            self.scroll_to_absolute_row(row);
            true
        } else {
            false
        }
    }

    /// Scroll the viewport to center the given absolute row.
    fn scroll_to_absolute_row(&mut self, abs_row: usize) {
        let sb_len = self.grid.scrollback().len();
        let half = self.grid.lines() / 2;
        // Compute display_offset that places abs_row near the center.
        // viewport_top = sb_len - display_offset
        // We want: viewport_top = abs_row - half (so abs_row is centered)
        let viewport_top = abs_row.saturating_sub(half);
        let offset = sb_len.saturating_sub(viewport_top);
        let clamped = offset.min(sb_len);
        if clamped != self.grid.display_offset() {
            // Use isize delta to go through scroll_display for dirty marking.
            let delta = clamped as isize - self.grid.display_offset() as isize;
            self.grid.scroll_display(delta);
        }
    }
}
