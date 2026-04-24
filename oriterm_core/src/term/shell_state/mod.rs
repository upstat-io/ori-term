//! Shell integration state accessors and navigation.
//!
//! Extracted from `term/mod.rs` to keep the main file under the 500-line
//! limit. These methods manage prompt state (OSC 133), CWD (OSC 7),
//! title resolution, notifications, and prompt-based navigation. The
//! shell-integration state types (`PromptState`, `PromptMarker`,
//! `Notification`, `PendingMarks`) also live here; `term/mod.rs`
//! re-exports them for a stable public API.

use std::collections::VecDeque;

use vte::ansi::{KeyboardModes, KeyboardModesApplyBehavior};

use super::Term;
use crate::effect::sink::EffectSink;

/// Shell integration prompt lifecycle state.
///
/// Tracks transitions from OSC 133 sub-parameters:
/// `None` → `PromptStart` (A) → `CommandStart` (B) → `OutputStart` (C) → `None` (D).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptState {
    /// No prompt activity or command completed (after D marker).
    #[default]
    None,
    /// Prompt is being displayed (after A marker).
    PromptStart,
    /// User is typing a command (after B marker).
    CommandStart,
    /// Command output is being produced (after C marker).
    OutputStart,
}

/// A single prompt lifecycle's boundary rows (absolute row indices).
///
/// Associates the OSC 133 sub-marker rows for one prompt: where the prompt
/// started (A), where the command line started (B), and where command output
/// started (C). Used for semantic zone navigation and selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptMarker {
    /// Absolute row where OSC 133;A (prompt start) was received.
    pub prompt: usize,
    /// Absolute row where OSC 133;B (command start) was received.
    pub command: Option<usize>,
    /// Absolute row where OSC 133;C (output start) was received.
    pub output: Option<usize>,
}

/// Desktop notification from the shell (OSC 9/99/777).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    /// Notification title (may be empty for OSC 9/99).
    pub title: String,
    /// Notification body text.
    pub body: String,
}

bitflags::bitflags! {
    /// Deferred OSC 133 marking actions.
    ///
    /// These flags are set when the corresponding OSC 133 sequence arrives
    /// and cleared after both VTE parsers finish processing, when the actual
    /// grid row marking occurs.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct PendingMarks: u8 {
        /// OSC 133;A received — prompt row marking deferred.
        const PROMPT = 1;
        /// OSC 133;B received — command start row marking deferred.
        const COMMAND_START = 2;
        /// OSC 133;C received — output start row marking deferred.
        const OUTPUT_START = 4;
    }
}

/// Extract the last path component from a CWD path for tab display.
///
/// - `/home/user/projects` → `projects`
/// - `/` → `/`
/// - `~` passthrough (shouldn't occur from OSC 7, but handle gracefully).
pub fn cwd_short_path(cwd: &str) -> &str {
    if cwd == "/" {
        return cwd;
    }
    // Strip trailing slash then take last component.
    let trimmed = cwd.strip_suffix('/').unwrap_or(cwd);
    let component = trimmed.rsplit('/').next().unwrap_or(cwd);
    // Paths like `///` reduce to an empty component after stripping — return `/`.
    if component.is_empty() { "/" } else { component }
}

impl<S: EffectSink> Term<S> {
    // -- Kitty keyboard mode stack accessors (BUG-08-12) --

    /// Current keyboard mode stack (Kitty keyboard protocol).
    pub fn keyboard_mode_stack(&self) -> &VecDeque<KeyboardModes> {
        &self.keyboard_mode_stack
    }

    /// Inactive-screen keyboard mode stack (swapped on alt-screen toggle).
    pub fn inactive_keyboard_mode_stack(&self) -> &VecDeque<KeyboardModes> {
        &self.inactive_keyboard_mode_stack
    }

    /// Pre-command snapshot of the active-screen keyboard mode stack
    /// (taken at OSC 133 ; C, consumed at OSC 133 ; A / ; D).
    ///
    /// `None` means no snapshot is active for this screen. `Some(deque)`
    /// holds the verbatim contents of [`Self::keyboard_mode_stack`] at
    /// the moment the pre-command snapshot was taken. See BUG-08-12.
    pub fn pre_command_kb_stack_snapshot(&self) -> Option<&VecDeque<KeyboardModes>> {
        self.pre_command_kb_stack_snapshot.as_ref()
    }

    /// Pre-command snapshot of the inactive-screen keyboard mode stack.
    ///
    /// See [`Self::pre_command_kb_stack_snapshot`] for semantics. Paired
    /// with that field and swapped alongside the stacks on alt-screen
    /// toggle so a snapshot taken on one screen only fires restore on
    /// that screen.
    pub fn inactive_pre_command_kb_stack_snapshot(&self) -> Option<&VecDeque<KeyboardModes>> {
        self.inactive_pre_command_kb_stack_snapshot.as_ref()
    }

    /// Pre-command snapshot of the active-screen Kitty-keyboard-protocol
    /// `TermMode` bits. Paired with [`Self::pre_command_kb_stack_snapshot`]
    /// — preserves shell-held kitty bits set via `CSI = Ps u` that never
    /// enter the stack. See BUG-08-12 TPR round-1 F1.
    pub fn pre_command_kb_mode_bits_snapshot(&self) -> Option<KeyboardModes> {
        self.pre_command_kb_mode_bits_snapshot
    }

    /// Paired inactive-screen bits snapshot — see
    /// [`Self::pre_command_kb_mode_bits_snapshot`].
    pub fn inactive_pre_command_kb_mode_bits_snapshot(&self) -> Option<KeyboardModes> {
        self.inactive_pre_command_kb_mode_bits_snapshot
    }

    // -- Kitty keyboard mode stack snapshot / restore (OSC 133 C/A/D) --

    /// Clone BOTH active and inactive keyboard-mode stack contents AND
    /// capture the current Kitty-keyboard-protocol `TermMode` bits so a
    /// subsequent OSC 133 `;A` / `;D` (or OSC 633 `;A` / `;D`) can
    /// restore them verbatim.
    ///
    /// Contents-based (not just depth-based) so we recover shell-held
    /// modes even when the child over-pops via `CSI < N u` or pushes past
    /// [`crate::term::KEYBOARD_MODE_STACK_MAX_DEPTH`] and `pop_front`
    /// evicts shell-held entries. Paired bits snapshot (not derived from
    /// stack top) so shells that set kitty modes via `CSI = Ps u` without
    /// pushing are also restored — see BUG-08-12 TPR round-1 F1. Allocates
    /// up to `2 × KEYBOARD_MODE_STACK_MAX_DEPTH × size_of::<KeyboardModes>()`
    /// plus 2 bytes of bits snapshot per command boundary — infrequent
    /// (once per command), not a hot path. See BUG-08-12.
    pub fn snapshot_keyboard_mode_stack(&mut self) {
        self.pre_command_kb_stack_snapshot = Some(self.keyboard_mode_stack.clone());
        self.inactive_pre_command_kb_stack_snapshot =
            Some(self.inactive_keyboard_mode_stack.clone());
        let active_bits = KeyboardModes::from(self.mode);
        self.pre_command_kb_mode_bits_snapshot = Some(active_bits);
        // Inactive bits snapshot is always captured alongside so the
        // paired invariant (both-or-neither) holds across `toggle_alt_common`
        // swaps. The inactive screen's effective bits are `NO_MODE` while
        // it is not the active screen; paired field lets the snapshot
        // follow the primary stack through alt-screen swaps without
        // dropping/leaking state.
        self.inactive_pre_command_kb_mode_bits_snapshot = Some(KeyboardModes::NO_MODE);
    }

    /// If a snapshot is active, replace BOTH stacks with the snapshotted
    /// contents and apply the snapshotted `TermMode` bits directly.
    ///
    /// Applying the paired BITS snapshot (rather than deriving from
    /// stack top) preserves shell-held kitty state set via `CSI = Ps u`
    /// WITHOUT pushing — the top of an empty stack is `NO_MODE`, which
    /// would silently clear a shell's set-only kitty mode at prompt
    /// boundary. Also covers `CSI = Ps u` same-depth mutations during a
    /// command (child changes bits without touching the stack; restore
    /// reverts both stack AND bits). The inactive stack is restored but
    /// not applied — `toggle_alt_common` applies its bits when the user
    /// switches to that screen. See BUG-08-12.
    pub fn restore_keyboard_mode_stack(&mut self) {
        if let Some(saved) = self.pre_command_kb_stack_snapshot.take() {
            self.keyboard_mode_stack = saved;
        }
        if let Some(saved_bits) = self.pre_command_kb_mode_bits_snapshot.take() {
            self.dcs_set_keyboard_mode(saved_bits, KeyboardModesApplyBehavior::Replace);
        }
        if let Some(saved) = self.inactive_pre_command_kb_stack_snapshot.take() {
            self.inactive_keyboard_mode_stack = saved;
        }
        // Clear the inactive bits snapshot; `toggle_alt_common` applies
        // top-of-new-active on screen swap rather than consuming this
        // field. Clearing keeps the paired invariant clean between
        // commands. See BUG-08-12 TPR round-1 F1.
        self.inactive_pre_command_kb_mode_bits_snapshot = None;
    }

    // -- Prompt state --

    /// Current shell integration prompt state (OSC 133).
    pub fn prompt_state(&self) -> PromptState {
        self.prompt_state
    }

    /// Set the prompt state (for raw interceptor).
    pub fn set_prompt_state(&mut self, state: PromptState) {
        self.prompt_state = state;
    }

    /// Whether OSC 133;A was received and the prompt row hasn't been marked yet.
    pub fn prompt_mark_pending(&self) -> bool {
        self.pending_marks.contains(PendingMarks::PROMPT)
    }

    /// Set/clear the prompt-mark-pending flag.
    pub fn set_prompt_mark_pending(&mut self, pending: bool) {
        self.pending_marks.set(PendingMarks::PROMPT, pending);
    }

    /// Record the current cursor row as a prompt line (OSC 133;A).
    ///
    /// Called after both VTE parsers finish processing a chunk, when
    /// `prompt_mark_pending` is `true`. Uses the cursor row from the
    /// high-level processor (which is at the correct position).
    pub fn mark_prompt_row(&mut self) {
        if !self.pending_marks.contains(PendingMarks::PROMPT) {
            return;
        }
        self.pending_marks.remove(PendingMarks::PROMPT);
        let abs_row = self.grid.scrollback().len() + self.grid.cursor().line();
        // Avoid duplicate entries (e.g. shell redrawing prompt on resize).
        if self
            .prompt_markers
            .last()
            .is_some_and(|m| m.prompt == abs_row)
        {
            return;
        }
        self.prompt_markers.push(PromptMarker {
            prompt: abs_row,
            command: None,
            output: None,
        });
    }

    /// Record the current cursor row as a command start (OSC 133;B).
    ///
    /// Fills `command_start` on the most recent prompt marker.
    pub fn mark_command_start_row(&mut self) {
        if !self.pending_marks.contains(PendingMarks::COMMAND_START) {
            return;
        }
        self.pending_marks.remove(PendingMarks::COMMAND_START);
        let abs_row = self.grid.scrollback().len() + self.grid.cursor().line();
        if let Some(marker) = self.prompt_markers.last_mut() {
            marker.command = Some(abs_row);
        }
    }

    /// Record the current cursor row as output start (OSC 133;C).
    ///
    /// Fills `output` on the most recent prompt marker.
    pub fn mark_output_start_row(&mut self) {
        if !self.pending_marks.contains(PendingMarks::OUTPUT_START) {
            return;
        }
        self.pending_marks.remove(PendingMarks::OUTPUT_START);
        let abs_row = self.grid.scrollback().len() + self.grid.cursor().line();
        if let Some(marker) = self.prompt_markers.last_mut() {
            marker.output = Some(abs_row);
        }
    }

    /// Whether OSC 133;B was received and hasn't been marked yet.
    pub fn command_start_mark_pending(&self) -> bool {
        self.pending_marks.contains(PendingMarks::COMMAND_START)
    }

    /// Set/clear the command-start-mark-pending flag.
    pub fn set_command_start_mark_pending(&mut self, pending: bool) {
        self.pending_marks.set(PendingMarks::COMMAND_START, pending);
    }

    /// Whether OSC 133;C was received and hasn't been marked yet.
    pub fn output_start_mark_pending(&self) -> bool {
        self.pending_marks.contains(PendingMarks::OUTPUT_START)
    }

    /// Set/clear the output-start-mark-pending flag.
    pub fn set_output_start_mark_pending(&mut self, pending: bool) {
        self.pending_marks.set(PendingMarks::OUTPUT_START, pending);
    }

    /// All prompt lifecycle markers.
    pub fn prompt_markers(&self) -> &[PromptMarker] {
        &self.prompt_markers
    }

    /// Prune prompt markers evicted from scrollback.
    ///
    /// When scrollback lines are evicted (the buffer is full and new lines
    /// push old ones out), markers with `prompt_start` below the eviction
    /// threshold are removed. Remaining row indices are shifted down.
    pub fn prune_prompt_markers(&mut self, evicted: usize) {
        if evicted == 0 {
            return;
        }
        self.prompt_markers.retain_mut(|marker| {
            if marker.prompt < evicted {
                false
            } else {
                marker.prompt -= evicted;
                if let Some(ref mut cr) = marker.command {
                    *cr = cr.saturating_sub(evicted);
                }
                if let Some(ref mut or) = marker.output {
                    *or = or.saturating_sub(evicted);
                }
                true
            }
        });
    }

    /// Find the output range for the prompt nearest to `near_row`.
    ///
    /// Returns `(output_start_row, end_row)` where `end_row` is one before
    /// the next prompt's `prompt_start`, or the current cursor row if this
    /// is the last marker.
    pub fn command_output_range(&self, near_row: usize) -> Option<(usize, usize)> {
        let (idx, marker) = self.find_nearest_marker(near_row)?;
        let output_row = marker.output?;
        let end = if idx + 1 < self.prompt_markers.len() {
            self.prompt_markers[idx + 1].prompt.saturating_sub(1)
        } else {
            // Last marker: end at the current cursor row.
            self.grid.scrollback().len() + self.grid.cursor().line()
        };
        if end < output_row {
            return None;
        }
        Some((output_row, end))
    }

    /// Find the command input range for the prompt nearest to `near_row`.
    ///
    /// Returns `(command_start_row, end_row)` where `end_row` is one before
    /// `output_start`, or one before the next prompt if no output marker.
    pub fn command_input_range(&self, near_row: usize) -> Option<(usize, usize)> {
        let (idx, marker) = self.find_nearest_marker(near_row)?;
        let cmd_row = marker.command?;
        let end = if let Some(or) = marker.output {
            or.saturating_sub(1)
        } else if idx + 1 < self.prompt_markers.len() {
            self.prompt_markers[idx + 1].prompt.saturating_sub(1)
        } else {
            self.grid.scrollback().len() + self.grid.cursor().line()
        };
        if end < cmd_row {
            return None;
        }
        Some((cmd_row, end))
    }

    // -- Command timing --

    /// Record command execution start (when OSC 133;C is received).
    pub fn set_command_start(&mut self, start: std::time::Instant) {
        self.command_start = Some(start);
    }

    /// Compute and store command duration (when OSC 133;D is received).
    ///
    /// Returns the duration if a matching start time existed. `now` lets
    /// tests inject a deterministic end timestamp; production callers pass
    /// `None` so the current wall-clock is used.
    pub fn finish_command(
        &mut self,
        now: Option<std::time::Instant>,
    ) -> Option<std::time::Duration> {
        let start = self.command_start.take()?;
        let end = now.unwrap_or_else(std::time::Instant::now);
        let duration = end.saturating_duration_since(start);
        self.last_command_duration = Some(duration);
        Some(duration)
    }

    /// Duration of the last completed command.
    pub fn last_command_duration(&self) -> Option<std::time::Duration> {
        self.last_command_duration
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

    /// Set the current working directory (for raw interceptor).
    pub fn set_cwd(&mut self, cwd: Option<String>) {
        self.cwd = cwd;
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
        if self.prompt_markers.is_empty() {
            return false;
        }
        // Current viewport top in absolute row coordinates.
        let sb_len = self.grid.scrollback().len();
        let viewport_top = sb_len.saturating_sub(self.grid.display_offset());
        // Find the last prompt row strictly above viewport top.
        let target = self
            .prompt_markers
            .iter()
            .rev()
            .find(|m| m.prompt < viewport_top);
        if let Some(marker) = target {
            let row = marker.prompt;
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
        if self.prompt_markers.is_empty() {
            return false;
        }
        let sb_len = self.grid.scrollback().len();
        // Current viewport bottom in absolute row coordinates.
        let viewport_bottom = sb_len.saturating_sub(self.grid.display_offset()) + self.grid.lines();
        let target = self
            .prompt_markers
            .iter()
            .find(|m| m.prompt >= viewport_bottom);
        if let Some(marker) = target {
            let row = marker.prompt;
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

    /// Find the prompt marker whose zone contains `near_row`.
    ///
    /// Returns the index and a reference to the last marker whose
    /// `prompt_start <= near_row`.
    fn find_nearest_marker(&self, near_row: usize) -> Option<(usize, &PromptMarker)> {
        self.prompt_markers
            .iter()
            .enumerate()
            .rev()
            .find(|(_, m)| m.prompt <= near_row)
    }
}

#[cfg(test)]
mod tests;
