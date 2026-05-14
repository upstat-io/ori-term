//! Helper functions for VTE handler dispatch.
//!
//! Mode-to-flag mappings, cursor positioning helpers, and version encoding
//! used by the Handler impl and mode dispatch.

use std::cmp;

use unicode_width::UnicodeWidthChar;
use vte::ansi::{ClearMode, LineClearMode, NamedPrivateMode};

use crate::cell::CellFlags;
use crate::effect::sink::EffectSink;
use crate::grid::StableRowIndex;
use crate::index::{Column, Line};
use crate::term::{Term, TermMode};

/// DECRPM value: 1 = set, 2 = reset.
pub(super) fn mode_report_value(is_set: bool) -> u8 {
    if is_set { 1 } else { 2 }
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
        NamedPrivateMode::EnableMode3 => Some(TermMode::ENABLE_MODE_3),
        NamedPrivateMode::Win32Input => Some(TermMode::WIN32_INPUT),
        NamedPrivateMode::LeftRightMargin => Some(TermMode::LEFT_RIGHT_MARGIN),
        NamedPrivateMode::ColorSchemeUpdate => Some(TermMode::COLOR_SCHEME_UPDATE),
        NamedPrivateMode::DecNumericKeypad => Some(TermMode::APP_KEYPAD),
        NamedPrivateMode::DecBackarrowKey => Some(TermMode::DECBKM),
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

impl<S: EffectSink> Term<S> {
    /// Try reverse wraparound: if cursor is at the left edge (column 0 or
    /// `left_margin` under DECLRMM) and the previous line was soft-wrapped,
    /// move cursor to the right edge of that line.
    ///
    /// Returns `true` if the wrap happened, `false` if no-op.
    pub(super) fn try_reverse_wrap(&mut self) -> bool {
        let grid = self.grid_mut();
        let col = grid.cursor().col().0;
        let (left, right) = grid.left_right_margins();
        let in_band = grid.cursor_in_margin_band();
        let edge = if in_band { left } else { 0 };
        if col != edge {
            return false;
        }
        let line = grid.cursor().line();
        if line == 0 {
            return false;
        }
        let wrap_col = if in_band {
            right
        } else {
            grid.cols().saturating_sub(1)
        };
        let prev = line - 1;
        let wrapped = grid[Line(prev as i32)][Column(wrap_col)]
            .flags
            .contains(CellFlags::WRAP);
        if wrapped {
            grid.move_to(prev, Column(wrap_col));
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
        let resolved_col = self.origin_aware_col(col);
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

        grid.move_to(line, resolved_col);
    }

    /// Resolve an incoming column parameter to an absolute screen column,
    /// applying DECOM + DECLRMM offset/clamp when appropriate.
    ///
    /// This is the canonical column-resolution point for horizontal
    /// positioning sequences (CUP/HVP column axis, CHA, HPA). Callers
    /// pass the raw 0-based column parameter from the escape sequence;
    /// this helper returns the absolute column where the cursor should
    /// land.
    ///
    /// - `ORIGIN && LEFT_RIGHT_MARGIN`: treat `col` as relative to the
    ///   left margin, offset and clamp to the margin band.
    /// - Otherwise: treat `col` as absolute, clamp to the last column.
    ///
    /// Centralizing this logic ensures CUP/HVP (via `goto_origin_aware`)
    /// and CHA/HPA (via `Handler::goto_col`) stay in sync — future
    /// changes to DECOM semantics land in one place.
    pub(super) fn origin_aware_col(&self, col: usize) -> Column {
        let origin = self.mode.contains(TermMode::ORIGIN);
        let lrm = self.mode.contains(TermMode::LEFT_RIGHT_MARGIN);
        let grid = self.grid();
        let (left, right) = grid.left_right_margins();
        if origin && lrm {
            Column((col + left).min(right))
        } else {
            Column(col.min(grid.cols().saturating_sub(1)))
        }
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
        // Erase paths may evict the U+10EEEE cells anchoring images;
        // recompute the surviving anchor set from the post-erase grid.
        self.reconcile_placeholder_anchors_from_grid();
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
        self.reconcile_placeholder_anchors_from_grid();
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
            self.reconcile_placeholder_anchors_from_grid();
        }
    }

    /// Check if scrollback evicted rows since `prev_evicted` and prune
    /// image placements that fell off.
    pub(super) fn prune_images_if_evicted(&mut self, prev_evicted: usize) {
        let new_evicted = self.grid().total_evicted();
        if new_evicted > prev_evicted {
            self.image_cache_mut()
                .prune_scrollback(StableRowIndex(new_evicted as u64));
            self.reconcile_placeholder_anchors_from_grid();
        }
    }

    /// Recompute the set of placeholder-anchored image IDs by walking the
    /// active grid + scrollback for `U+10EEEE` cells, then forward the
    /// survivor set to `ImageCache::reconcile_placeholder_anchors`.
    ///
    /// Term owns the grid; this keeps `ImageCache` ignorant of cell/Grid
    /// types per the §13.4 layer-boundary contract.
    pub(in crate::term) fn reconcile_placeholder_anchors_from_grid(&mut self) {
        if self.image_cache().placeholder_anchors().is_empty() {
            return;
        }
        let survivors = self.collect_placeholder_image_ids();
        self.image_cache_mut().reconcile_placeholder_anchors(&survivors);
    }

    /// Collect every `image_id` referenced by a `U+10EEEE` placeholder
    /// cell currently in the grid (visible rows + scrollback).
    fn collect_placeholder_image_ids(&self) -> std::collections::HashSet<crate::image::ImageId> {
        use crate::image::KITTY_PLACEHOLDER;
        use crate::term::handler::image::kitty::placeholder::IncompletePlacement;

        let mut survivors: std::collections::HashSet<crate::image::ImageId> =
            std::collections::HashSet::new();
        let grid = self.grid();
        let lines = grid.lines();
        let cols = grid.cols();

        let visit_row = |row: &crate::grid::Row,
                         survivors: &mut std::collections::HashSet<crate::image::ImageId>| {
            let mut prev = None;
            for col_idx in 0..cols {
                let cell = &row[Column(col_idx)];
                if cell.ch == KITTY_PLACEHOLDER {
                    let (ul, zw) = match cell.extra.as_ref() {
                        Some(e) => (e.underline_color, e.zerowidth.clone()),
                        None => (None, Vec::new()),
                    };
                    let inc = IncompletePlacement::decode(&zw, cell.fg, ul);
                    let resolved = inc.resolve_with_continuation(prev.as_ref());
                    if resolved.image_id != 0 {
                        survivors.insert(crate::image::ImageId::from_raw(resolved.image_id));
                    }
                    prev = Some(resolved);
                } else {
                    prev = None;
                }
            }
        };

        // Visible grid rows.
        for line in 0..lines {
            let row = &grid[Line(line as i32)];
            visit_row(row, &mut survivors);
        }
        // Scrollback rows.
        for sb_idx in 0..grid.scrollback().len() {
            if let Some(row) = grid.scrollback().get(sb_idx) {
                visit_row(row, &mut survivors);
            }
        }
        survivors
    }

    /// Printable character input: charset translation, width handling, wrapping.
    #[inline]
    pub(super) fn input_char(&mut self, c: char) {
        self.selection_dirty = true;

        if !self.mode.contains(TermMode::LINE_WRAP) {
            let cols = self.grid().cols();
            if self.grid().cursor().col().0 >= cols {
                self.grid_mut().cursor_mut().set_col(Column(cols - 1));
            }
        }

        if c as u32 <= 0x7E
            && c as u32 >= 0x20
            && !self.mode.contains(TermMode::INSERT)
            && self.charset.is_ascii()
            && self.grid_mut().put_char_ascii(c)
        {
            return;
        }

        let c = self.charset.translate(c);
        let width = match UnicodeWidthChar::width(c) {
            Some(width) => width,
            None => return,
        };
        if width == 0 {
            self.grid_mut().push_zerowidth(c);
            return;
        }

        if width == 2 && !self.mode.contains(TermMode::LINE_WRAP) {
            let col = self.grid().cursor().col().0;
            let cols = self.grid().cols();
            if col + 1 >= cols {
                self.grid_mut().cursor_mut().set_col(Column(cols));
                return;
            }
        }

        let prev = self.grid().total_evicted();
        let insert = self.mode.contains(TermMode::INSERT);
        let grid = self.grid_mut();
        if insert {
            grid.insert_blank(width);
        }
        grid.put_char(c);
        self.prune_images_if_evicted(prev);
    }
}
