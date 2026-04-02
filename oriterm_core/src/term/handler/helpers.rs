//! Helper functions for VTE handler dispatch.
//!
//! Mode number lookups, mode-to-flag mappings, cursor positioning helpers,
//! and version encoding used by the Handler impl and mode dispatch.

use std::cmp;

use vte::ansi::{ClearMode, LineClearMode, NamedPrivateMode};

use crate::cell::CellFlags;
use crate::event::EventListener;
use crate::grid::StableRowIndex;
use crate::index::{Column, Line};
use crate::term::{Term, TermMode};

/// DECRPM value: 1 = set, 2 = reset.
pub(super) fn mode_report_value(is_set: bool) -> u8 {
    if is_set { 1 } else { 2 }
}

/// Map `NamedPrivateMode` to its CSI mode number.
pub(super) fn named_private_mode_number(mode: NamedPrivateMode) -> u16 {
    match mode {
        NamedPrivateMode::CursorKeys => 1,
        NamedPrivateMode::ColumnMode => 3,
        NamedPrivateMode::ReverseVideo => 5,
        NamedPrivateMode::Origin => 6,
        NamedPrivateMode::X10Mouse => 9,
        NamedPrivateMode::LineWrap => 7,
        NamedPrivateMode::BlinkingCursor => 12,
        NamedPrivateMode::ShowCursor => 25,
        NamedPrivateMode::ReverseWraparound => 45,
        NamedPrivateMode::AltScreen => 47,
        NamedPrivateMode::ReportMouseClicks => 1000,
        NamedPrivateMode::ReportCellMouseMotion => 1002,
        NamedPrivateMode::ReportAllMouseMotion => 1003,
        NamedPrivateMode::ReportFocusInOut => 1004,
        NamedPrivateMode::Utf8Mouse => 1005,
        NamedPrivateMode::SgrMouse => 1006,
        NamedPrivateMode::AlternateScroll => 1007,
        NamedPrivateMode::UrxvtMouse => 1015,
        NamedPrivateMode::UrgencyHints => 1042,
        NamedPrivateMode::AltScreenOpt => 1047,
        NamedPrivateMode::SaveCursor => 1048,
        NamedPrivateMode::SwapScreenAndSetRestoreCursor => 1049,
        NamedPrivateMode::BracketedPaste => 2004,
        NamedPrivateMode::SyncUpdate => 2026,
        NamedPrivateMode::SixelScrolling => 80,
        NamedPrivateMode::SixelCursorRight => 8452,
    }
}

/// Map `NamedPrivateMode` to the corresponding `TermMode` flag, if supported.
pub(super) fn named_private_mode_flag(mode: NamedPrivateMode) -> Option<TermMode> {
    match mode {
        NamedPrivateMode::CursorKeys => Some(TermMode::APP_CURSOR),
        NamedPrivateMode::Origin => Some(TermMode::ORIGIN),
        NamedPrivateMode::LineWrap => Some(TermMode::LINE_WRAP),
        NamedPrivateMode::BlinkingCursor => Some(TermMode::CURSOR_BLINKING),
        NamedPrivateMode::ShowCursor => Some(TermMode::SHOW_CURSOR),
        NamedPrivateMode::ReverseWraparound => Some(TermMode::REVERSE_WRAP),
        NamedPrivateMode::AltScreen
        | NamedPrivateMode::AltScreenOpt
        | NamedPrivateMode::SwapScreenAndSetRestoreCursor => Some(TermMode::ALT_SCREEN),
        NamedPrivateMode::X10Mouse => Some(TermMode::MOUSE_X10),
        NamedPrivateMode::ReportMouseClicks => Some(TermMode::MOUSE_REPORT_CLICK),
        NamedPrivateMode::ReportCellMouseMotion => Some(TermMode::MOUSE_DRAG),
        NamedPrivateMode::ReportAllMouseMotion => Some(TermMode::MOUSE_MOTION),
        NamedPrivateMode::ReportFocusInOut => Some(TermMode::FOCUS_IN_OUT),
        NamedPrivateMode::Utf8Mouse => Some(TermMode::MOUSE_UTF8),
        NamedPrivateMode::SgrMouse => Some(TermMode::MOUSE_SGR),
        NamedPrivateMode::UrxvtMouse => Some(TermMode::MOUSE_URXVT),
        NamedPrivateMode::UrgencyHints => Some(TermMode::URGENCY_HINTS),
        NamedPrivateMode::BracketedPaste => Some(TermMode::BRACKETED_PASTE),
        NamedPrivateMode::SyncUpdate => Some(TermMode::SYNC_UPDATE),
        NamedPrivateMode::AlternateScroll => Some(TermMode::ALTERNATE_SCROLL),
        NamedPrivateMode::SixelScrolling => Some(TermMode::SIXEL_SCROLLING),
        NamedPrivateMode::SixelCursorRight => Some(TermMode::SIXEL_CURSOR_RIGHT),
        NamedPrivateMode::ReverseVideo => Some(TermMode::REVERSE_VIDEO),
        NamedPrivateMode::SaveCursor | NamedPrivateMode::ColumnMode => None,
    }
}

/// Convert the crate version (semver) to a single integer for DA2 response.
///
/// `"0.1.3"` → `103`.
pub(super) fn crate_version_number() -> usize {
    let mut result = 0usize;
    let version = env!("CARGO_PKG_VERSION");
    // Strip any pre-release suffix (e.g. "-alpha.3").
    let version = version.split('-').next().unwrap_or(version);
    for (i, part) in version.split('.').rev().enumerate() {
        let n = part.parse::<usize>().unwrap_or(0);
        result += n * 100usize.pow(i as u32);
    }
    result
}

impl<T: EventListener> Term<T> {
    /// Try reverse wraparound: if cursor is at column 0 and the previous
    /// line was soft-wrapped, move cursor to the last column of that line.
    ///
    /// Returns `true` if the wrap happened, `false` if no-op.
    pub(super) fn try_reverse_wrap(&mut self) -> bool {
        let grid = self.grid_mut();
        if grid.cursor().col().0 != 0 {
            return false;
        }
        let line = grid.cursor().line();
        if line == 0 {
            return false;
        }
        let last_col = grid.cols().saturating_sub(1);
        let prev = line - 1;
        let wrapped = grid[Line(prev as i32)][Column(last_col)]
            .flags
            .contains(CellFlags::WRAP);
        if wrapped {
            grid.move_to(prev, Column(last_col));
            true
        } else {
            false
        }
    }

    /// Origin-aware absolute cursor positioning.
    ///
    /// When ORIGIN mode is active, `line` is relative to the scroll region
    /// and clamped to it. Otherwise, `line` is relative to the screen top
    /// and clamped to the full viewport. Used by `Handler::goto`,
    /// `set_scrolling_region`, and DECSET/DECRST origin-mode toggling.
    pub(super) fn goto_origin_aware(&mut self, line: i32, col: usize) {
        let origin = self.mode.contains(TermMode::ORIGIN);
        let grid = self.grid_mut();
        let region_start = grid.scroll_region().start;
        let region_end = grid.scroll_region().end;

        let (offset, max_line) = if origin {
            (region_start, region_end.saturating_sub(1))
        } else {
            (0, grid.lines().saturating_sub(1))
        };

        let line = cmp::max(0, line) as usize;
        let line = cmp::min(line + offset, max_line);
        let col = Column(col.min(grid.cols().saturating_sub(1)));
        grid.move_to(line, col);
    }

    /// Convert a grid line (0-based visible row) to a `StableRowIndex`.
    fn grid_line_stable(&self, line: usize) -> StableRowIndex {
        let grid = self.grid();
        let base = grid.total_evicted() as u64 + grid.scrollback().len() as u64;
        StableRowIndex(base + line as u64)
    }

    /// After ED (erase in display): remove image placements in the erased
    /// region. Grid is image-unaware; `Term` coordinates between Grid and
    /// `ImageCache`.
    pub(super) fn clear_images_after_ed(&mut self, mode: &ClearMode) {
        let grid = self.grid();
        let cl = grid.cursor().line();
        let cc = grid.cursor().col().0;
        let lines = grid.lines();

        match mode {
            ClearMode::Below => {
                let top = self.grid_line_stable(cl);
                self.image_cache_mut()
                    .remove_placements_in_region(top, top, Some(cc), None);
                if cl + 1 < lines {
                    let next = self.grid_line_stable(cl + 1);
                    let bot = self.grid_line_stable(lines - 1);
                    self.image_cache_mut()
                        .remove_placements_in_region(next, bot, None, None);
                }
            }
            ClearMode::Above => {
                let top = self.grid_line_stable(cl);
                self.image_cache_mut()
                    .remove_placements_in_region(top, top, None, Some(cc));
                if cl > 0 {
                    let first = self.grid_line_stable(0);
                    let prev = self.grid_line_stable(cl - 1);
                    self.image_cache_mut()
                        .remove_placements_in_region(first, prev, None, None);
                }
            }
            ClearMode::All => {
                let first = self.grid_line_stable(0);
                let last = self.grid_line_stable(lines.saturating_sub(1));
                self.image_cache_mut()
                    .remove_placements_in_region(first, last, None, None);
            }
            ClearMode::Saved => {
                // Scrollback was just cleared — prune image placements that
                // referenced evicted rows.
                let threshold = StableRowIndex(self.grid().total_evicted() as u64);
                self.image_cache_mut().prune_scrollback(threshold);
            }
        }
    }

    /// After EL (erase in line): remove image placements on the erased
    /// portion of the cursor line.
    pub(super) fn clear_images_after_el(&mut self, mode: &LineClearMode) {
        let grid = self.grid();
        let cl = grid.cursor().line();
        let cc = grid.cursor().col().0;

        let (left, right) = match mode {
            LineClearMode::Right => (Some(cc), None),
            LineClearMode::Left => (None, Some(cc)),
            LineClearMode::All => (None, None),
        };
        let row = self.grid_line_stable(cl);
        self.image_cache_mut()
            .remove_placements_in_region(row, row, left, right);
    }

    /// After ECH (erase characters): remove image placements in the erased
    /// cell range on the cursor line.
    pub(super) fn clear_images_after_ech(&mut self, count: usize) {
        let grid = self.grid();
        let cl = grid.cursor().line();
        let cc = grid.cursor().col().0;
        let cols = grid.cols();

        let end = (cc + count).min(cols);
        if end > cc {
            let row = self.grid_line_stable(cl);
            self.image_cache_mut()
                .remove_placements_in_region(row, row, Some(cc), Some(end - 1));
        }
    }

    /// Check if scrollback evicted rows since `prev_evicted` and prune
    /// image placements that fell off.
    pub(super) fn prune_images_if_evicted(&mut self, prev_evicted: usize) {
        let new_evicted = self.grid().total_evicted();
        if new_evicted > prev_evicted {
            self.image_cache_mut()
                .prune_scrollback(StableRowIndex(new_evicted as u64));
        }
    }
}
