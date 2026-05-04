//! Rectangular-area grid editing primitives (DEC §09A.6).
//!
//! Hosts the row-mutation logic invoked by the DEC rectangular-area
//! handlers (DECCARA, DECRARA, DECCRA, DECFRA, DECERA, DECSERA) per
//! the §09A.6 architectural pin: the handler layer is an adapter,
//! every cell write lives here.
//!
//! Coordinates are 0-based and inclusive on every edge. Callers are
//! responsible for clamping / DECLRMM-awareness BEFORE invoking these
//! primitives — the grid layer trusts its input and asserts bounds
//! via `debug_assert!`. `selection_dirty = true` is the handler's
//! responsibility too; this file never touches Term state.

use vte::ansi::Color;

use crate::cell::{Cell, CellFlags};
use crate::index::Column;
use crate::term::AceMode;

use super::super::Grid;

/// DECCARA (Ps=0) resets these SGR flags to off; every other DECCARA
/// param individually sets or clears a matching bit.
///
/// The bit set matches the DEC STD 070 reversible-attribute set plus
/// xterm's SGR 8 (HIDDEN) extension. Color SGRs are intentionally
/// excluded per spec — DECCARA does not touch cell colors.
const DECCARA_RESET_MASK: CellFlags = CellFlags::BOLD
    .union(CellFlags::UNDERLINE)
    .union(CellFlags::BLINK)
    .union(CellFlags::INVERSE)
    .union(CellFlags::HIDDEN);

/// Translate a DECCARA / DECRARA SGR parameter into a `(set, clear)`
/// pair of `CellFlags`. Unknown parameters are a silent no-op
/// `(empty, empty)` pair — consistent with xterm's ignore-unknowns
/// behavior (`screen.c:3006-3040`).
fn deccara_set_clear(param: u16) -> (CellFlags, CellFlags) {
    match param {
        0 => (CellFlags::empty(), DECCARA_RESET_MASK),
        1 => (CellFlags::BOLD, CellFlags::empty()),
        4 => (CellFlags::UNDERLINE, CellFlags::empty()),
        5 => (CellFlags::BLINK, CellFlags::empty()),
        7 => (CellFlags::INVERSE, CellFlags::empty()),
        8 => (CellFlags::HIDDEN, CellFlags::empty()),
        22 => (CellFlags::empty(), CellFlags::BOLD),
        24 => (CellFlags::empty(), CellFlags::UNDERLINE),
        25 => (CellFlags::empty(), CellFlags::BLINK),
        27 => (CellFlags::empty(), CellFlags::INVERSE),
        28 => (CellFlags::empty(), CellFlags::HIDDEN),
        _ => (CellFlags::empty(), CellFlags::empty()),
    }
}

/// Translate a DECRARA SGR parameter into the mask of `CellFlags`
/// bits to toggle. Matches xterm `screen.c:2984-3004`: Ps=0 toggles
/// every reversible bit; Ps=N toggles the single matching bit.
fn decrara_toggle_mask(param: u16) -> CellFlags {
    match param {
        0 => DECCARA_RESET_MASK,
        1 => CellFlags::BOLD,
        4 => CellFlags::UNDERLINE,
        5 => CellFlags::BLINK,
        7 => CellFlags::INVERSE,
        8 => CellFlags::HIDDEN,
        _ => CellFlags::empty(),
    }
}

/// Collapse an SGR parameter list into cumulative set/clear masks.
fn deccara_combined(params: &[u16]) -> (CellFlags, CellFlags) {
    let mut to_set = CellFlags::empty();
    let mut to_clear = CellFlags::empty();
    for &p in params {
        let (s, c) = deccara_set_clear(p);
        // `Reset` clears everything — honor that by dropping any
        // previously-accumulated "set" bits it would reset.
        to_set -= c;
        to_clear -= s;
        to_set |= s;
        to_clear |= c;
    }
    (to_set, to_clear)
}

/// Collapse a DECRARA parameter list into a single toggle mask.
fn decrara_combined(params: &[u16]) -> CellFlags {
    let mut toggle = CellFlags::empty();
    for &p in params {
        toggle |= decrara_toggle_mask(p);
    }
    toggle
}

impl Grid {
    /// DECFRA primitive: fill a rectangle with `template` cells.
    ///
    /// Every cell in `[top..=bot] × [left..=right]` is reset to
    /// `template` (carrying `DRAWN` so DECRQCRA treats them as written
 /// per ) and marked dirty. Preserves grid invariants via
    /// `Grid::clear_wide_char_at` for wide-char spacer cleanup along
    /// the left/right edges.
    #[expect(
        clippy::too_many_arguments,
        reason = "rectangle bounds (top/left/bot/right) map 1:1 to DEC spec params — collapsing loses direct param-to-spec mapping"
    )]
    pub fn fill_rect(
        &mut self,
        top: usize,
        left: usize,
        bot: usize,
        right: usize,
        template: &Cell,
    ) {
        debug_assert!(top <= bot && bot < self.lines);
        debug_assert!(left <= right && right < self.cols);

        let cols = self.cols;
        for line in top..=bot {
            self.clear_wide_char_at(line, left);
            if right + 1 < cols {
                self.clear_wide_char_at(line, right);
            }

            let row = &mut self.rows[line];
            for col in left..=right {
                let cell = &mut row[Column(col)];
                cell.reset(template);
                cell.flags.insert(CellFlags::DRAWN);
            }
            let new_occ = right + 1;
            if new_occ > row.occ() {
                row.set_occ(new_occ);
            }
            self.dirty.mark_cols(line, left, right);
        }
    }

    /// DECERA primitive: erase `[top..=bot] × [left..=right]`.
    ///
    /// Writes a default-background cell (space + BCE bg + DRAWN) to
    /// every cell in the rectangle, IGNORING `CellFlags::PROTECTED`.
    /// DECERA is the unconditional-erase counterpart to DECSERA.
    #[expect(
        clippy::too_many_arguments,
        reason = "rectangle bounds (top/left/bot/right) + BCE bg map 1:1 to DEC spec params"
    )]
    pub fn erase_rect_all(&mut self, top: usize, left: usize, bot: usize, right: usize, bg: Color) {
        let template = Cell::from(bg);
        self.fill_rect(top, left, bot, right, &template);
    }

    /// DECSERA primitive: erase unprotected cells in the rectangle.
    ///
    /// Cells with `CellFlags::PROTECTED` (set by DECSCA Ps=1) survive
    /// the erase. Everything else matches [`Self::erase_rect_all`].
    #[expect(
        clippy::too_many_arguments,
        reason = "rectangle bounds (top/left/bot/right) + BCE bg map 1:1 to DEC spec params"
    )]
    pub fn erase_rect_unprotected(
        &mut self,
        top: usize,
        left: usize,
        bot: usize,
        right: usize,
        bg: Color,
    ) {
        debug_assert!(top <= bot && bot < self.lines);
        debug_assert!(left <= right && right < self.cols);

        let cols = self.cols;
        let template = Cell::from(bg);
        for line in top..=bot {
            let left_unprotected = !self.rows[line][Column(left)]
                .flags
                .contains(CellFlags::PROTECTED);
            let right_unprotected = !self.rows[line][Column(right)]
                .flags
                .contains(CellFlags::PROTECTED);
            if left_unprotected {
                self.clear_wide_char_at(line, left);
            }
            if right_unprotected && right + 1 < cols {
                self.clear_wide_char_at(line, right);
            }

            let row = &mut self.rows[line];
            let mut last_erased: Option<usize> = None;
            for col in left..=right {
                let cell = &mut row[Column(col)];
                if cell.flags.contains(CellFlags::PROTECTED) {
                    continue;
                }
                cell.reset(&template);
                cell.flags.insert(CellFlags::DRAWN);
                last_erased = Some(col);
            }
            if let Some(end_col) = last_erased {
                let new_occ = end_col + 1;
                if new_occ > row.occ() {
                    row.set_occ(new_occ);
                }
                self.dirty.mark_cols(line, left, end_col);
            }
        }
    }

    /// DECCARA primitive: apply SGR `params` to every cell in the rect.
    ///
    /// `ace_mode` selects rectangular vs stream extent. Only DEC
    /// STD 070 SGR params (0, 1, 4, 5, 7, 22, 24, 25, 27) plus xterm's
    /// SGR 8 (HIDDEN) extension are honored; unknown params are
    /// silently ignored per xterm (`screen.c:3040`). Colors are NOT
    /// applied — DECCARA spec omits them.
    #[expect(
        clippy::too_many_arguments,
        reason = "rectangle bounds + SGR params + DECSACE mode map 1:1 to DEC spec inputs"
    )]
    pub fn apply_sgr_rect(
        &mut self,
        top: usize,
        left: usize,
        bot: usize,
        right: usize,
        params: &[u16],
        ace_mode: AceMode,
    ) {
        debug_assert!(top <= bot && bot < self.lines);
        debug_assert!(left <= right && right < self.cols);

        let (to_set, to_clear) = deccara_combined(params);
        if to_set.is_empty() && to_clear.is_empty() {
            return;
        }

        let cols = self.cols;
        for line in top..=bot {
            let (row_left, row_right) =
                Self::line_extent(line, top, bot, left, right, cols, ace_mode);
            let row = &mut self.rows[line];
            for col in row_left..=row_right {
                let cell = &mut row[Column(col)];
                cell.flags.remove(to_clear);
                cell.flags.insert(to_set);
            }
            self.dirty.mark_cols(line, row_left, row_right);
        }
    }

    /// DECRARA primitive: toggle SGR `params` on every cell in the rect.
    ///
    /// Ps=0 toggles every reversible bit (BOLD | UNDERLINE | BLINK |
    /// INVERSE | HIDDEN); Ps=N toggles only the matching bit. Matches
    /// xterm `screen.c:2984-3004`.
    #[expect(
        clippy::too_many_arguments,
        reason = "rectangle bounds + SGR params + DECSACE mode map 1:1 to DEC spec inputs"
    )]
    pub fn reverse_sgr_rect(
        &mut self,
        top: usize,
        left: usize,
        bot: usize,
        right: usize,
        params: &[u16],
        ace_mode: AceMode,
    ) {
        debug_assert!(top <= bot && bot < self.lines);
        debug_assert!(left <= right && right < self.cols);

        let toggle = decrara_combined(params);
        if toggle.is_empty() {
            return;
        }

        let cols = self.cols;
        for line in top..=bot {
            let (row_left, row_right) =
                Self::line_extent(line, top, bot, left, right, cols, ace_mode);
            let row = &mut self.rows[line];
            for col in row_left..=row_right {
                let cell = &mut row[Column(col)];
                cell.flags ^= toggle;
            }
            self.dirty.mark_cols(line, row_left, row_right);
        }
    }

    /// DECCRA primitive: copy a source rectangle to a destination.
    ///
    /// Source and destination may overlap. Source cells are cloned
    /// into a scratch buffer first, THEN written to the destination —
    /// guaranteeing no cell is read after it has been overwritten.
    /// Destination cells are clipped to the grid; cells whose source
    /// coordinate would fall outside the source rectangle are left
    /// untouched (matches xterm `ScrnCopyRectangle`).
    #[expect(
        clippy::too_many_arguments,
        reason = "DECCRA encodes src rect (4 coords) + dst top-left (2 coords) as 6 distinct spec params"
    )]
    pub fn copy_rect(
        &mut self,
        src_top: usize,
        src_left: usize,
        src_bot: usize,
        src_right: usize,
        dst_top: usize,
        dst_left: usize,
    ) {
        debug_assert!(src_top <= src_bot && src_bot < self.lines);
        debug_assert!(src_left <= src_right && src_right < self.cols);
        debug_assert!(dst_top < self.lines);
        debug_assert!(dst_left < self.cols);

        let lines = self.lines;
        let cols = self.cols;
        let height = src_bot - src_top + 1;
        let width = src_right - src_left + 1;

        // Clip the destination extent to the physical grid.
        let dst_bot_clamped = (dst_top + height - 1).min(lines - 1);
        let dst_right_clamped = (dst_left + width - 1).min(cols - 1);
        let dh = dst_bot_clamped - dst_top + 1;
        let dw = dst_right_clamped - dst_left + 1;
        if dh == 0 || dw == 0 {
            return;
        }

        // Snapshot the source region into a scratch buffer. ONE
        // allocation per DECCRA invocation — permitted by the plan:
        // "No per-cell allocations outside DECCRA's scratch buffer".
        let mut scratch: Vec<Cell> = Vec::with_capacity(dh * dw);
        for r in 0..dh {
            for c in 0..dw {
                let src_line = src_top + r;
                let src_col = src_left + c;
                scratch.push(self.rows[src_line][Column(src_col)].clone());
            }
        }

        // Write scratch → destination.
        for r in 0..dh {
            let dst_line = dst_top + r;
            self.clear_wide_char_at(dst_line, dst_left);
            if dst_right_clamped + 1 < cols {
                self.clear_wide_char_at(dst_line, dst_right_clamped);
            }
            let row = &mut self.rows[dst_line];
            for c in 0..dw {
                let dst_col = dst_left + c;
                let cell = &scratch[r * dw + c];
                row[Column(dst_col)] = cell.clone();
            }
            let new_occ = dst_right_clamped + 1;
            if new_occ > row.occ() {
                row.set_occ(new_occ);
            }
            self.dirty.mark_cols(dst_line, dst_left, dst_right_clamped);
        }
    }

    /// Compute the `(left, right)` column span for one line of a
    /// DECSACE-extent rectangle.
    ///
    /// In `Rectangle` mode every row is clipped to `[left, right]`.
    /// In `Stream` mode, only the first row's left and the last row's
    /// right clamp the span; intermediate rows span the full width.
    #[expect(
        clippy::too_many_arguments,
        reason = "per-line extent computation needs line + rect bounds + cols + mode — all independent inputs"
    )]
    fn line_extent(
        line: usize,
        top: usize,
        bot: usize,
        left: usize,
        right: usize,
        cols: usize,
        mode: AceMode,
    ) -> (usize, usize) {
        match mode {
            AceMode::Rectangle => (left, right),
            AceMode::Stream => {
                let l = if line == top { left } else { 0 };
                let r = if line == bot {
                    right
                } else {
                    cols.saturating_sub(1)
                };
                (l, r)
            }
        }
    }
}
