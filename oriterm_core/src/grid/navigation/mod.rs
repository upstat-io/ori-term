//! Cursor movement and navigation operations.
//!
//! Implements CUU/CUD/CUF/CUB/CUP/CHA/VPA/CR/LF/RI/NEL/HT/CBT and
//! tab stop management. All movement is clamped to grid bounds and
//! respects the scroll region where applicable.

use crate::index::Column;

use super::Grid;

/// Tab clear mode for TBC (Tabulation Clear).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabClearMode {
    /// Clear tab stop at the current column.
    Current,
    /// Clear all tab stops.
    All,
}

impl Grid {
    /// CUU: move cursor up by `count` lines, clamped to the top of the
    /// scroll region (if inside it) or line 0.
    pub fn move_up(&mut self, count: usize) {
        let line = self.cursor.line();
        let top = if line >= self.scroll_region.start && line < self.scroll_region.end {
            self.scroll_region.start
        } else {
            0
        };
        self.move_cursor_line(line.saturating_sub(count).max(top));
    }

    /// CUD: move cursor down by `count` lines, clamped to the bottom of
    /// the scroll region (if inside it) or the last line.
    pub fn move_down(&mut self, count: usize) {
        let line = self.cursor.line();
        let bottom = if line >= self.scroll_region.start && line < self.scroll_region.end {
            self.scroll_region.end - 1
        } else {
            self.lines - 1
        };
        self.move_cursor_line((line + count).min(bottom));
    }

    /// CUF: move cursor right by `count` columns, clamped to the right
    /// margin when cursor is within the DECLRMM band, or to the last
    /// column otherwise.
    pub fn move_forward(&mut self, count: usize) {
        let col = self.cursor.col().0;
        let bound = if self.cursor_in_margin_band() {
            self.right_margin
        } else {
            self.cols - 1
        };
        self.move_cursor_col(Column((col + count).min(bound)));
    }

    /// CUB: move cursor left by `count` columns, clamped to the left
    /// margin when cursor is within the DECLRMM band, or to column 0
    /// otherwise.
    pub fn move_backward(&mut self, count: usize) {
        let col = self.cursor.col().0;
        let bound = if self.cursor_in_margin_band() {
            self.left_margin
        } else {
            0
        };
        self.move_cursor_col(Column(col.saturating_sub(count).max(bound)));
    }

    /// CUP: set cursor to absolute `(line, col)`, clamped to grid bounds.
    ///
    /// CUP/HVP are absolute addressing — they bypass horizontal margins
    /// entirely. DECOM (origin mode) shifts the addressable origin to
    /// `(scroll_top, left_margin)` at the `Term` layer via
    /// `Term::goto_origin_aware`, which translates incoming coordinates
    /// before reaching this function. By the time we are here, `line`
    /// and `col` are already absolute screen positions, so only
    /// grid-bound clamping applies.
    pub fn move_to(&mut self, line: usize, col: Column) {
        self.move_cursor_line(line.min(self.lines - 1));
        self.move_cursor_col(Column(col.0.min(self.cols - 1)));
    }

    /// CHA: set cursor column to absolute `col`, clamped to grid bounds.
    ///
    /// CHA/HPA are absolute addressing — margins are NOT enforced here.
    /// The DECOM+DECLRMM offset (`col += left_margin`) is applied at the
    /// `Term` layer in `Term::goto_col` before reaching this function.
    pub fn move_to_column(&mut self, col: Column) {
        self.move_cursor_col(Column(col.0.min(self.cols - 1)));
    }

    /// VPA: set cursor line to `line`, clamped to the last line.
    pub fn move_to_line(&mut self, line: usize) {
        self.move_cursor_line(line.min(self.lines - 1));
    }

    /// CR: move cursor to `left_margin` when within the DECLRMM margin
    /// band, or to column 0 otherwise.
    pub fn carriage_return(&mut self) {
        let target = if self.cursor_in_margin_band() {
            self.left_margin
        } else {
            0
        };
        self.move_cursor_col(Column(target));
    }

    /// BS: move cursor left by one column.
    ///
    /// If the cursor is in wrap-pending state (col >= cols), snaps to the
    /// last column. Otherwise moves left by one, clamped at `left_margin`
    /// when within the DECLRMM band or column 0 otherwise.
    pub fn backspace(&mut self) {
        let col = self.cursor.col().0;
        let cols = self.cols;

        if col >= cols {
            let snap = if self.cursor_in_margin_band() {
                self.right_margin
            } else {
                cols - 1
            };
            self.move_cursor_col(Column(snap));
        } else {
            let bound = if self.cursor_in_margin_band() {
                self.left_margin
            } else {
                0
            };
            if col > bound {
                self.move_cursor_col(Column(col - 1));
            }
        }
    }

    /// LF: move cursor down one line. If at the bottom of the scroll
    /// region, scroll the region up instead of moving.
    pub fn linefeed(&mut self) {
        let line = self.cursor.line();
        if line + 1 == self.scroll_region.end {
            // At bottom of scroll region: scroll region content up.
            self.scroll_up(1);
        } else if line + 1 < self.lines {
            self.move_cursor_line(line + 1);
        } else {
            // Already at last line, outside scroll region: no-op.
        }
    }

    /// RI: move cursor up one line. If at the top of the scroll region,
    /// scroll the region down instead of moving.
    pub fn reverse_index(&mut self) {
        let line = self.cursor.line();
        if line == self.scroll_region.start {
            // At top of scroll region: scroll region content down.
            self.scroll_down(1);
        } else if line > 0 {
            self.move_cursor_line(line - 1);
        } else {
            // Already at line 0, outside scroll region: no-op.
        }
    }

    /// NEL: carriage return followed by linefeed.
    pub fn next_line(&mut self) {
        self.carriage_return();
        self.linefeed();
    }

    /// HT: advance cursor to the next tab stop, or the right bound.
    ///
    /// When cursor is within the DECLRMM margin band, tab stops beyond
    /// `right_margin` are unreachable; HT stops at `right_margin`.
    pub fn tab(&mut self) {
        let col = self.cursor.col().0;
        let in_band = self.cursor_in_margin_band();
        let right_bound = if in_band {
            self.right_margin
        } else {
            self.cols - 1
        };

        for c in (col + 1)..=right_bound {
            if c < self.cols && self.tab_stops[c] {
                self.move_cursor_col(Column(c));
                return;
            }
        }
        self.move_cursor_col(Column(right_bound));
    }

    /// CBT: move cursor to the previous tab stop, or the left bound.
    ///
    /// When cursor is within the DECLRMM margin band, tab stops before
    /// `left_margin` are unreachable; CBT stops at `left_margin`.
    pub fn tab_backward(&mut self) {
        let col = self.cursor.col().0.min(self.cols);
        let in_band = self.cursor_in_margin_band();
        let left_bound = if in_band { self.left_margin } else { 0 };

        for c in (left_bound..col).rev() {
            if self.tab_stops[c] {
                self.move_cursor_col(Column(c));
                return;
            }
        }
        self.move_cursor_col(Column(left_bound));
    }

    /// HTS: set a tab stop at the current cursor column.
    pub fn set_tab_stop(&mut self) {
        let col = self.cursor.col().0;
        if col < self.cols {
            self.tab_stops[col] = true;
        }
    }

    /// TBC: clear tab stop(s) according to mode.
    pub fn clear_tab_stop(&mut self, mode: TabClearMode) {
        match mode {
            TabClearMode::Current => {
                let col = self.cursor.col().0;
                if col < self.cols {
                    self.tab_stops[col] = false;
                }
            }
            TabClearMode::All => {
                self.tab_stops.fill(false);
            }
        }
    }

    /// DECSC: save cursor position and template.
    ///
    /// Per DEC STD 070 §5.6.1 and cross-verified against wezterm, alacritty,
    /// and ghostty, the DECSC save set is the cursor position, character
    /// attributes, charset state, wrap flag, and DECOM flag. DECLRMM margins
    /// are NOT saved — margin state is scoped to the screen (alt vs primary),
    /// not to the cursor save/restore pair, and is toggled via RIS, DECSTR,
    /// DECCOLM, DECALN, resize, or explicit mode reset.
    pub fn save_cursor(&mut self) {
        if crate::xray_trace::next_sgr() {
            log::info!(
                target: "oriterm_core::xray",
                "DECSC saving line={} col={} fg={:?} bg={:?}",
                self.cursor.line(),
                self.cursor.col().0,
                self.cursor.template.fg,
                self.cursor.template.bg,
            );
        }
        self.saved_cursor = Some(self.cursor.clone());
    }

    /// DECRC: restore cursor position and template from saved state, or
    /// reset to origin if nothing was saved. Does not touch margins or
    /// DECLRMM mode (see `save_cursor` for the save-set rationale).
    pub fn restore_cursor(&mut self) {
        if crate::xray_trace::next_sgr() {
            match &self.saved_cursor {
                Some(saved) => log::info!(
                    target: "oriterm_core::xray",
                    "DECRC restoring to line={} col={} fg={:?} bg={:?} (had saved)",
                    saved.line(),
                    saved.col().0,
                    saved.template.fg,
                    saved.template.bg,
                ),
                None => log::info!(
                    target: "oriterm_core::xray",
                    "DECRC no-saved → reset to origin with DEFAULT template (was fg={:?} bg={:?})",
                    self.cursor.template.fg,
                    self.cursor.template.bg,
                ),
            }
        }
        let old_line = self.cursor.line();
        self.dirty.mark(old_line);
        if let Some(saved) = &self.saved_cursor {
            self.cursor = saved.clone();
        } else {
            self.cursor = super::cursor::Cursor::new();
        }
        self.dirty.mark(self.cursor.line());
    }
}

#[cfg(test)]
mod tests;
