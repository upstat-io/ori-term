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
use crate::effect::{Effect, PtyEffect, PtyWriteKind};
use crate::index::{Column, Line};
use crate::term::AceMode;

use super::super::Term;

/// 0-based inclusive rectangle produced by [`Term::clamp_rect`].
#[derive(Debug, Clone, Copy)]
struct ClampedRect {
    top: usize,
    left: usize,
    bot: usize,
    right: usize,
}

/// XTCHECKSUM bitmask constants (xterm patch-336; `screen.c:3149`).
///
/// Every bit opts *out* of the default behavior. Default (`0`) matches
/// xterm: negate-on, attribs-included, trim trailing blanks, skip
/// never-drawn cells, DEC-translate the character.
const CS_POSITIVE: u16 = 1; // do NOT negate final sum
const CS_ATTRIBS: u16 = 2; // EXCLUDE SGR attrs from sum
const CS_NOTRIM: u16 = 4; // do NOT trim trailing blanks (also forces undrawn cells to ' ')
const CS_DRAWN: u16 = 8; // include undrawn cells as space (distinct from csNOTRIM)
const CS_BYTE: u16 = 16; // use raw char byte (structurally inapplicable to ori_term)

/// xterm attribute tag constants (`screen.c:3221-3234`).
///
/// `CellFlags::PROTECTED` (DECSCA) folds with xterm tag `0x04` once
/// the char-protection bit is set on the cell (§09A.8).
const ATTR_PROTECTED: i32 = 0x04; // xterm PROTECTED (DECSCA)
const ATTR_HIDDEN: i32 = 0x08; // xterm INVISIBLE
const ATTR_UNDERLINE: i32 = 0x10; // any underline variant
const ATTR_INVERSE: i32 = 0x20;
const ATTR_BLINK: i32 = 0x40;
const ATTR_BOLD: i32 = 0x80;

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
    }

    /// XTCHECKSUM (CSI Ps # y) — Set DECRQCRA checksum-extension flags.
    ///
    /// Stores the bitmask on `Term` for the next DECRQCRA request to
    /// consume. Default `0` matches xterm (negate + include attrs).
    pub(super) fn xtchecksum_impl(&mut self, flags: u16) {
        self.checksum_flags = flags;
    }

    /// DECRQCRA (CSI Pi;Pg;Pt;Pl;Pb;Pr * y) — Request Checksum of
    /// Rectangular Area.
    ///
    /// Synchronous: clamps the rectangle, folds cells into a 16-bit
    /// checksum (xterm sum-then-negate), and emits the DCS reply
    /// `DCS Pi ! ~ XXXX ST` via `PtyEffect::Write`. `page` is ignored
    /// because `ori_term` only supports a single page.
    #[expect(
        clippy::too_many_arguments,
        reason = "DECRQCRA spec: id + page + top/left/bot/right — 6 distinct param slots (id, page, 4 rect coords)"
    )]
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "receiver stays `&mut self` so the method matches the other rect-ops delegates; `effect_sink.push(&self)` is interior-mutability"
    )]
    pub(super) fn decrqcra_impl(
        &mut self,
        id: u16,
        _page: u16,
        top: u16,
        left: u16,
        bot: u16,
        right: u16,
    ) {
        let checksum = self.compute_rect_checksum(top, left, bot, right);
        let response = format!("\x1bP{id}!~{checksum:04X}\x1b\\");
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: response.into_bytes(),
            kind: PtyWriteKind::ChecksumReport,
        }));
    }

    /// Compute the DECRQCRA checksum over a 1-based inclusive rectangle.
    ///
    /// Mirrors xterm `xtermCheckRect()` at `screen.c:3136`. Maintains
    /// `total` (every counted cell) and `trimmed` (same, minus trailing
    /// blanks per row); the final checksum is `trimmed` in default mode
    /// and `total` when `csNOTRIM` is set. Undrawn cells (detected via
    /// `CellFlags::DRAWN` — the `ori_term` CHARDRAWN analog set on every
    /// cell-write path per ) are skipped in default mode and
    /// treated as `' '` when `csNOTRIM` or `csDRAWN` is set. Combining
    /// marks fold into `total` only when `csBYTE` is unset.
    ///
    /// **Structural deviations from xterm** — documented so downstream
    /// consumers expecting byte-exact xterm parity know where we diverge:
    ///
    /// * **`csBYTE`** is accepted as a no-op. `ori_term` cells already
    ///   store the Unicode codepoint after charset translation; there
    ///   is no second "raw byte" storage to switch to. The bit still
    ///   governs combining-mark inclusion (unset = folded, set = not
    ///   folded), matching xterm's `if_OPT_WIDE_CHARS` guard.
    /// * **PROTECTED (xterm tag `0x04`)** is folded into the attribute
    ///   sum when `CellFlags::PROTECTED` is set. The bit is written by
    ///   DECSCA via the cursor template (§09A.8).
    /// * **Color folding** is intentionally dropped. Mainline xterm
    ///   only folds colors into the checksum under VT525 emulation
    ///   (`screen.c:3198`); `ori_term` is not a VT525 emulator, so the
    ///   mainline branch that we match does not contribute colors.
    ///
    /// Zero allocations in the inner loop — every accumulator stays on
    /// the stack; the single `format!()` for the reply is the caller's
    /// responsibility (see `decrqcra_impl`).
    pub fn compute_rect_checksum(&self, top: u16, left: u16, bot: u16, right: u16) -> u16 {
        let grid = self.grid();
        let lines = grid.lines();
        let cols = grid.cols();
        if lines == 0 || cols == 0 {
            return 0;
        }

        // 1-based → 0-based inclusive; clamp each edge independently
        // so partially-out-of-range rectangles still produce the same
        // checksum as a valid-bounds equivalent (xterm "clamped to
        // physical buffer" semantics, `screen.c:3162 validRect`).
        let top0 = top.max(1).min(lines as u16) - 1;
        let bot0 = bot.max(1).min(lines as u16) - 1;
        let left0 = left.max(1).min(cols as u16) - 1;
        let right0 = right.max(1).min(cols as u16) - 1;
        if top0 > bot0 || left0 > right0 {
            return 0;
        }

        let flags = self.checksum_flags;
        let negate = (flags & CS_POSITIVE) == 0;
        let include_attribs = (flags & CS_ATTRIBS) == 0;
        let trim = (flags & CS_NOTRIM) == 0;
        let space_for_undrawn = (flags & (CS_NOTRIM | CS_DRAWN)) != 0;
        let fold_combining = (flags & CS_BYTE) == 0;

        let mut total: i32 = 0;
        let mut trimmed: i32 = 0;
        // xterm declares `first` and `embedded` ONCE, outside the row
        // loop (`screen.c:3166-3167`). The end-of-row block resets them
        // in default (trim) mode but leaves them untouched when
        // csNOTRIM is set — trim state is per-rectangle, not per-row.
        let mut first = true;
        let mut embedded: i32 = 0;

        for line in top0..=bot0 {
            let row = &grid[Line(line as i32)];

            for col in left0..=right0 {
                let cell = &row[Column(col as usize)];
                // xterm CHARDRAWN analog (). The DRAWN bit is
                // set on every cell-write path and cleared on every
                // reset path, giving us a persistent "was written"
                // marker distinct from the visual `is_empty()` query.
                let drawn = cell.flags.contains(CellFlags::DRAWN);
                let ch: i32 = if drawn {
                    cell.ch as i32
                } else if space_for_undrawn {
                    ' ' as i32
                } else {
                    continue;
                };

                let mut ch_val = ch;
                if include_attribs {
                    let cf = cell.flags;
                    if cf.contains(CellFlags::PROTECTED) {
                        ch_val = ch_val.wrapping_add(ATTR_PROTECTED);
                    }
                    if cf.contains(CellFlags::HIDDEN) {
                        ch_val = ch_val.wrapping_add(ATTR_HIDDEN);
                    }
                    if cf.intersects(CellFlags::ALL_UNDERLINES) {
                        ch_val = ch_val.wrapping_add(ATTR_UNDERLINE);
                    }
                    if cf.contains(CellFlags::INVERSE) {
                        ch_val = ch_val.wrapping_add(ATTR_INVERSE);
                    }
                    if cf.contains(CellFlags::BLINK) {
                        ch_val = ch_val.wrapping_add(ATTR_BLINK);
                    }
                    if cf.contains(CellFlags::BOLD) {
                        ch_val = ch_val.wrapping_add(ATTR_BOLD);
                    }
                }

                // Trim accounting mirrors xterm `screen.c:3236-3241`.
                // The third disjunct (`drawn`) is ori_term's
                // `DRAWX_MASK` analog: xterm uses
                // `(ld->attribs[col] & DRAWX_MASK)` where
                // `DRAWX_MASK = ATTRIBUTES | CHARDRAWN`, which is true
                // for every cell that was actually drawn OR that
                // carries any SGR attribute. Our `drawn` bit already
                // discriminates "actually written" cells from the
                // csNOTRIM|csDRAWN-synthesized space path above, so
                // wide-char spacers and structurally-drawn blanks push
                // into `trimmed` even when `ch_val == ' '`.
                if first || ch_val != ' ' as i32 || drawn {
                    trimmed = trimmed.wrapping_add(ch_val).wrapping_add(embedded);
                    embedded = 0;
                } else if !trim {
                    embedded = embedded.wrapping_add(ch_val);
                } else {
                    // Default mode: synthetic trailing blanks (undrawn
                    // cells in csDRAWN mode only, which cannot reach
                    // this branch because csDRAWN implies `!trim`)
                    // would drop; structurally there is nothing to do.
                }

                total = total.wrapping_add(ch_val);

                // xterm folds combining marks into `total` only (not
                // `trimmed`) — the FIXME at `screen.c:3244` notes the
                // combining contribution is lost in default (trim) mode.
                // We reproduce that behavior faithfully.
                if fold_combining && let Some(extra) = &cell.extra {
                    for combining_ch in &extra.zerowidth {
                        total = total.wrapping_add(*combining_ch as i32);
                    }
                }

                // xterm keeps `first` pinned to `True` for the whole
                // row when `csNOTRIM` is set; otherwise it flips off
                // after the first column (`screen.c:3252`).
                first = !trim;
            }

            // End-of-row reset per xterm `screen.c:3254-3257` — only
            // applies in default (trim) mode. When csNOTRIM is set, the
            // trim accounting is cumulative across the whole rectangle.
            if trim {
                embedded = 0;
                first = false;
            }
        }

        let mut checksum = if trim { trimmed } else { total };
        if negate {
            checksum = checksum.wrapping_neg();
        }
        (checksum & 0xFFFF) as u16
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
