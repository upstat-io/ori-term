//! DEC private rectangular-area handler implementations.
//!
//! Hosts the inherent helper methods on `Term<S>` that the `Handler`
//! trait impl in `super::mod.rs` delegates to for DECSACE / DECCARA /
//! DECRARA / DECCRA / DECFRA / XTCHECKSUM / DECRQCRA / DECERA /
//! DECSERA / XTREPORTSGR.
//!
//! §09A.5 implemented XTCHECKSUM + DECRQCRA. §09A.6 implements the
//! six mutation ops (DECCARA / DECRARA / DECCRA / DECFRA / DECERA /
//! DECSERA) as thin adapters: each handler clamps the rectangle
//! (DECLRMM-aware via `clamp_rect`), flips `selection_dirty` per the
//! pattern at `term/handler/mod.rs:140-177`, then delegates to a
//! `Grid::*_rect` primitive in `oriterm_core/src/grid/editing/rect.rs`
//! (LEAK guard per §09A.R finding #8). DECSACE stores its extent mode
//! on Term (never Grid — LEAK guard per §09A.R finding #9).

use log::debug;

use crate::cell::CellFlags;
use crate::effect::sink::EffectSink;
use crate::term::AceMode;

use super::super::Term;

mod checksum;

/// 0-based inclusive rectangle produced by [`Term::clamp_rect`].
#[derive(Debug, Clone, Copy)]
struct ClampedRect {
    top: usize,
    left: usize,
    bot: usize,
    right: usize,
}

impl<S: EffectSink> Term<S> {
    /// DECSACE (CSI Ps * x) — Select Attribute Change Extent.
    ///
    /// Per xterm `screen.c:2941`, `Ps == 2` selects rectangle mode;
    /// every other value (including the default `0`) is stream mode.
    /// The resolved mode is stored on Term (NEVER on Grid — LEAK
    /// guard per §09A.R finding #9) for DECCARA / DECRARA to consume.
    pub(super) fn decsace_impl(&mut self, mode: u16) {
        self.ace_mode = if mode == 2 {
            AceMode::Rectangle
        } else {
            AceMode::Stream
        };
        debug!("DECSACE: mode={mode} -> {:?}", self.ace_mode);
    }

    /// DECCARA (CSI Pt;Pl;Pb;Pr;Pm $ r) — Change Attributes in
    /// Rectangular Area.
    #[expect(
        clippy::too_many_arguments,
        reason = "DECCARA spec: top/left/bot/right + SGR attrs slice — collapsing loses direct param-to-spec mapping"
    )]
    pub(super) fn deccara_impl(
        &mut self,
        top: u16,
        left: u16,
        bot: u16,
        right: u16,
        attrs: &[u16],
    ) {
        let Some(rect) = self.clamp_rect(top, left, bot, right) else {
            return;
        };
        self.selection_dirty = true;
        let mode = self.ace_mode;
        self.grid_mut()
            .apply_sgr_rect(rect.top, rect.left, rect.bot, rect.right, attrs, mode);
    }

    /// DECRARA (CSI Pt;Pl;Pb;Pr;Pm $ t) — Reverse Attributes in
    /// Rectangular Area.
    #[expect(
        clippy::too_many_arguments,
        reason = "DECRARA spec: top/left/bot/right + SGR attrs slice — collapsing loses direct param-to-spec mapping"
    )]
    pub(super) fn decrara_impl(
        &mut self,
        top: u16,
        left: u16,
        bot: u16,
        right: u16,
        attrs: &[u16],
    ) {
        let Some(rect) = self.clamp_rect(top, left, bot, right) else {
            return;
        };
        self.selection_dirty = true;
        let mode = self.ace_mode;
        self.grid_mut()
            .reverse_sgr_rect(rect.top, rect.left, rect.bot, rect.right, attrs, mode);
    }

    /// DECCRA (CSI Pts;Pls;Pbs;Prs;Pps;Ptd;Pld;Ppd $ v) — Copy
    /// Rectangular Area.
    #[expect(
        clippy::too_many_arguments,
        reason = "DECCRA spec encodes 8 distinct coordinates — collapsing loses direct param-to-spec mapping"
    )]
    pub(super) fn deccra_impl(
        &mut self,
        src_top: u16,
        src_left: u16,
        src_bot: u16,
        src_right: u16,
        _src_page: u16,
        dst_top: u16,
        dst_left: u16,
        _dst_page: u16,
    ) {
        // `ori_term` is single-page (DA1 declares page=1); src/dst
        // page parameters other than 0/1 are accepted but ignored,
        // matching xterm `screen.c:2785` (`ParamPair` clamps page
        // into `[0, 1]`).
        let Some(src) = self.clamp_rect(src_top, src_left, src_bot, src_right) else {
            return;
        };
        let dst_top_u = dst_top.max(1) as usize - 1;
        let dst_left_u = dst_left.max(1) as usize - 1;
        let lines = self.grid().lines();
        let cols = self.grid().cols();
        if dst_top_u >= lines || dst_left_u >= cols {
            return;
        }
        self.selection_dirty = true;
        self.grid_mut()
            .copy_rect(src.top, src.left, src.bot, src.right, dst_top_u, dst_left_u);
    }

    /// DECFRA (CSI Pc;Pt;Pl;Pb;Pr $ x) — Fill Rectangular Area.
    ///
    /// Per xterm `charproc.c:5661-5674`, `Pc=0` falls back to a space
    /// (ASCII 0x20); other valid values are printable ASCII (0x20–
    /// 0x7E) or Latin-1 supplemental (0xA0–0xFF). Out-of-range values
    /// (e.g. 0x01–0x1F or 0x7F–0x9F) are silently ignored. The fill
    /// cell inherits the cursor's current SGR template plus `DRAWN`
    /// ().
    #[expect(
        clippy::too_many_arguments,
        reason = "DECFRA spec: char + top/left/bot/right — 5 coords + char = 6 distinct param slots"
    )]
    pub(super) fn decfra_impl(&mut self, ch: u16, top: u16, left: u16, bot: u16, right: u16) {
        // Pc=0 falls back to space per xterm's `use_default_value`.
        let fill_code = if ch == 0 { 0x20 } else { ch };
        let valid = (0x20..=0x7E).contains(&fill_code) || (0xA0..=0xFF).contains(&fill_code);
        if !valid {
            return;
        }
        let Some(rect) = self.clamp_rect(top, left, bot, right) else {
            return;
        };
        self.selection_dirty = true;

        // Build the fill template from the cursor template + Pc + DRAWN.
        let grid = self.grid_mut();
        let mut template = grid.cursor().template.clone();
        // The cursor template must never carry INTERNAL_CELL_STATE
        // bits (); sanity-assert so a future regression is
        // surfaced at fill-rect time rather than silently writing
        // structurally-invalid cells.
        debug_assert!(
            !template.flags.intersects(CellFlags::INTERNAL_CELL_STATE),
            "cursor template must not carry INTERNAL_CELL_STATE bits at DECFRA dispatch"
        );
        template.ch = char::from_u32(u32::from(fill_code)).unwrap_or(' ');
        grid.fill_rect(rect.top, rect.left, rect.bot, rect.right, &template);
        // Fill replaces every cell in the rectangle; placeholder cells
        // there are gone — reconcile the anchor set.
        self.reconcile_placeholder_anchors_from_grid();
    }

    /// XTCHECKSUM (CSI Ps # y) — Set DECRQCRA checksum-extension flags.
    ///
    /// Stores the bitmask on `Term` for the next DECRQCRA request to
    /// consume. Default `0` matches xterm (negate + include attrs).
    pub(super) fn xtchecksum_impl(&mut self, flags: u16) {
        self.checksum_flags = flags;
    }

    /// DECERA (CSI Pt;Pl;Pb;Pr $ z) — Erase Rectangular Area.
    ///
    /// Erases every cell in the clamped rectangle with a BCE-colored
    /// space, ignoring `CellFlags::PROTECTED` (DECSCA). The BCE bg
    /// source is the cursor template's current background — matching
    /// how `erase_line` / `erase_chars` already behave.
    pub(super) fn decera_impl(&mut self, top: u16, left: u16, bot: u16, right: u16) {
        let Some(rect) = self.clamp_rect(top, left, bot, right) else {
            return;
        };
        self.selection_dirty = true;
        let bg = self.grid().cursor().template.bg;
        self.grid_mut()
            .erase_rect_all(rect.top, rect.left, rect.bot, rect.right, bg);
        // Erase drops placeholder cells in the rectangle — reconcile.
        self.reconcile_placeholder_anchors_from_grid();
    }

    /// DECSERA (CSI Pt;Pl;Pb;Pr $ {) — Selective Erase Rectangular Area.
    ///
    /// Same as DECERA except cells carrying `CellFlags::PROTECTED`
    /// (set by DECSCA Ps=1) are preserved.
    pub(super) fn decsera_impl(&mut self, top: u16, left: u16, bot: u16, right: u16) {
        let Some(rect) = self.clamp_rect(top, left, bot, right) else {
            return;
        };
        self.selection_dirty = true;
        let bg = self.grid().cursor().template.bg;
        self.grid_mut()
            .erase_rect_unprotected(rect.top, rect.left, rect.bot, rect.right, bg);
        // Selective erase drops any non-protected placeholder cells —
        // reconcile. Protected placeholder cells survive; their anchors
        // are kept by the reconcile walk.
        self.reconcile_placeholder_anchors_from_grid();
    }

    /// XTREPORTSGR (CSI Pt;Pl;Pb;Pr # |) — Report SGR attributes of
    /// Rectangular Area.
    ///
    /// Stays a debug-traced stub per the §09A plan scope ("XTREPORTSGR
    /// stays `missing` per plan — it's not in 09A.6 scope"). The DCS
    /// reply format is pinned against xterm patch-336 once implemented
    /// in a follow-up section. `&mut self` is preserved so the handler
    /// signature stays stable when the real impl lands.
    #[expect(
        clippy::unused_self,
        clippy::needless_pass_by_ref_mut,
        reason = "stub receiver held stable for the real impl in a follow-up section"
    )]
    pub(super) fn xtreportsgr_impl(&mut self, top: u16, left: u16, bot: u16, right: u16) {
        debug!("XTREPORTSGR: rect=({top},{left})-({bot},{right}) (stub — out of §09A.6 scope)");
    }

    /// Clamp a 1-based DEC rectangle to the active grid extent.
    ///
    /// Returns `None` for zero-area rectangles (top > bot or left >
    /// right after clamping). When DECLRMM is active, the left/right
    /// columns are additionally clamped to `[margin_left, margin_right]`
    /// per xterm `screen.c:3162 validRect`. The clamped result uses
    /// 0-based inclusive coordinates suitable for the grid primitives.
    fn clamp_rect(&self, top: u16, left: u16, bot: u16, right: u16) -> Option<ClampedRect> {
        let grid = self.grid();
        let lines = grid.lines();
        let cols = grid.cols();
        if lines == 0 || cols == 0 {
            return None;
        }

        // DEC params are 1-based; `0` means "default" which per xterm
        // falls back to 1 for top/left and last row/col for bot/right.
        let top_1 = if top == 0 { 1 } else { top };
        let left_1 = if left == 0 { 1 } else { left };
        let bot_1 = if bot == 0 { lines as u16 } else { bot };
        let right_1 = if right == 0 { cols as u16 } else { right };

        let top0 = (top_1.min(lines as u16) as usize) - 1;
        let bot0 = (bot_1.min(lines as u16) as usize) - 1;
        let mut left0 = (left_1.min(cols as u16) as usize) - 1;
        let mut right0 = (right_1.min(cols as u16) as usize) - 1;

        // DECLRMM: clamp horizontal extent into the active margin band.
        let (margin_left, margin_right) = grid.left_right_margins();
        if grid.has_horizontal_margins() {
            left0 = left0.max(margin_left);
            right0 = right0.min(margin_right);
        }

        if top0 > bot0 || left0 > right0 {
            return None;
        }
        Some(ClampedRect {
            top: top0,
            left: left0,
            bot: bot0,
            right: right0,
        })
    }
}

#[cfg(test)]
mod tests;
