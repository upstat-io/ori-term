//! DEC private rectangular-area handler implementations.
//!
//! Hosts the inherent helper methods on `Term<S>` that the `Handler`
//! trait impl in `super::mod.rs` delegates to for DECSACE / DECCARA /
//! DECRARA / DECCRA / DECFRA / XTCHECKSUM / DECRQCRA / DECERA /
//! DECSERA / XTREPORTSGR.
//!
//! §09A.5 implements XTCHECKSUM flag storage + DECRQCRA checksum
//! emission. The remaining methods are still §09A.4 debug-traced
//! stubs — their real semantics land in §09A.6 (mutation ops) and
//! §09A.8 (DECSACE alongside other presentation modes).

// §09A.6+ stubs still take `&mut self` and don't touch it; keep the
// file-level expects so those stubs don't fail the nursery lints.
// Every file-level expect is satisfied by at least one remaining stub.
#![expect(
    clippy::unused_self,
    reason = "§09A.6+ scaffolding stubs — real impls in §09A.6/§09A.8 use self"
)]
#![expect(
    clippy::needless_pass_by_ref_mut,
    reason = "§09A.6+ scaffolding stubs — real impls in §09A.6/§09A.8 mutate self"
)]

use log::debug;

use crate::cell::CellFlags;
use crate::effect::sink::EffectSink;
use crate::effect::{Effect, PtyEffect, PtyWriteKind};
use crate::index::{Column, Line};

use super::super::Term;

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
/// `ori_term` has no PROTECTED flag yet (that arrives in §09A.8 with
/// DECSCA); every other xterm video attribute maps to a `CellFlags`
/// bit that we already carry.
const ATTR_HIDDEN: i32 = 0x08; // xterm INVISIBLE
const ATTR_UNDERLINE: i32 = 0x10; // any underline variant
const ATTR_INVERSE: i32 = 0x20;
const ATTR_BLINK: i32 = 0x40;
const ATTR_BOLD: i32 = 0x80;

impl<S: EffectSink> Term<S> {
    /// DECSACE (CSI Ps * x) — Select Attribute Change Extent.
    pub(super) fn decsace_impl(&mut self, mode: u16) {
        debug!("DECSACE: mode={mode} (stub — §09A.6)");
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
        debug!("DECCARA: rect=({top},{left})-({bot},{right}) attrs={attrs:?} (stub — §09A.6)");
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
        debug!("DECRARA: rect=({top},{left})-({bot},{right}) attrs={attrs:?} (stub — §09A.6)");
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
        src_page: u16,
        dst_top: u16,
        dst_left: u16,
        dst_page: u16,
    ) {
        debug!(
            "DECCRA: src=({src_top},{src_left})-({src_bot},{src_right}) \
             src_page={src_page} dst=({dst_top},{dst_left}) dst_page={dst_page} \
             (stub — §09A.6)"
        );
    }

    /// DECFRA (CSI Pc;Pt;Pl;Pb;Pr $ x) — Fill Rectangular Area.
    #[expect(
        clippy::too_many_arguments,
        reason = "DECFRA spec: char + top/left/bot/right — 5 coords + char = 6 distinct param slots"
    )]
    pub(super) fn decfra_impl(&mut self, ch: u16, top: u16, left: u16, bot: u16, right: u16) {
        debug!("DECFRA: ch={ch} rect=({top},{left})-({bot},{right}) (stub — §09A.6)");
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
    /// and `total` when `csNOTRIM` is set. Undrawn cells (proxied via
    /// `Cell::is_empty()`) are skipped in default mode and treated as
    /// `' '` when `csNOTRIM` or `csDRAWN` is set. Combining marks fold
    /// into `total` only when `csBYTE` is unset.
    ///
    /// **Structural deviations from xterm** — documented so downstream
    /// consumers expecting byte-exact xterm parity know where we diverge:
    ///
    /// * **`csBYTE`** is accepted as a no-op. `ori_term` cells already
    ///   store the Unicode codepoint after charset translation; there
    ///   is no second "raw byte" storage to switch to. The bit still
    ///   governs combining-mark inclusion (unset = folded, set = not
    ///   folded), matching xterm's `if_OPT_WIDE_CHARS` guard.
    /// * **PROTECTED (xterm tag `0x04`)** is not folded yet — the
    ///   `CellFlags::PROTECTED` bit lands with DECSCA in §09A.8. Every
    ///   other xterm video attribute (BOLD, BLINK, INVERSE, UNDERLINE,
    ///   INVISIBLE/HIDDEN) uses its exact xterm constant.
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
                // xterm CHARDRAWN analog (BUG-08-17). The DRAWN bit is
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
    pub(super) fn decera_impl(&mut self, top: u16, left: u16, bot: u16, right: u16) {
        debug!("DECERA: rect=({top},{left})-({bot},{right}) (stub — §09A.6)");
    }

    /// DECSERA (CSI Pt;Pl;Pb;Pr $ {) — Selective Erase Rectangular Area.
    pub(super) fn decsera_impl(&mut self, top: u16, left: u16, bot: u16, right: u16) {
        debug!("DECSERA: rect=({top},{left})-({bot},{right}) (stub — §09A.6)");
    }

    /// XTREPORTSGR (CSI Pt;Pl;Pb;Pr # |) — Report SGR attributes of
    /// Rectangular Area.
    pub(super) fn xtreportsgr_impl(&mut self, top: u16, left: u16, bot: u16, right: u16) {
        debug!("XTREPORTSGR: rect=({top},{left})-({bot},{right}) (stub — §09A.6)");
    }
}

#[cfg(test)]
mod tests;
