//! DEC private presentation / column / back-forward-index handler
//! implementations.
//!
//! Hosts the inherent helper methods on `Term<S>` that the `Handler`
//! trait impl in `super::mod.rs` delegates to for the CSI-path
//! presentation queries (DECRQPSR / DECRQUPSS / DECRQDE / DECSCL /
//! DECSCA / DECSASD / DECSSDT) and the DECIC / DECDC column ops and
//! DECBI / DECFI back/forward index ops.
//!
//! §09A.4 landed the structural wiring. §09A.8 replaced the CSI-path
//! presentation stubs with real semantics. §09A.7 replaces the
//! DECIC / DECDC / DECBI / DECFI stubs with real semantics that
//! delegate to the column primitives in `grid/editing/column.rs` and
//! to `grid.move_forward` / `grid.move_backward` for the non-margin
//! cursor-move branches of DECBI / DECFI.

use log::debug;

use crate::cell::CellFlags;
use crate::effect::sink::EffectSink;
use crate::effect::{Effect, PtyEffect, PtyWriteKind};

use super::super::Term;

impl<S: EffectSink> Term<S> {
    /// DECRQPSR (CSI Ps $ w) — Request Presentation State Report.
    ///
    /// Mode 1 replies with a cursor information report; mode 2 replies
    /// with the current tab-stop positions. Both responses are DCS
    /// streams terminated by ST. Mode 1 is a `verified-with-deviation`
    /// stub (cursor-state serialization is complex — see §09A.8); mode
    /// 2 serializes the real tab-stop vector.
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "receiver stays `&mut self` so the method matches the `delegate_osc!` shape; `effect_sink.push(&self)` is interior-mutability"
    )]
    pub(super) fn decrqpsr_impl(&mut self, mode: u16) {
        let bytes = match mode {
            1 => {
                // Cursor information report — stub. Serialize just the
                // cursor line/col + page (no charset designations,
                // SGR, or protection state yet). DCS 1 $ u ... ST.
                let line = self.grid().cursor().line() + 1;
                let col = self.grid().cursor().col().0 + 1;
                format!("\x1bP1$u{line};{col};1;@;@;@;0;@;BBBB\x1b\\").into_bytes()
            }
            2 => {
                // Tab-stop report — real. DCS 2 $ u col1/col2/... ST.
                // Columns are 1-based; `/` separates them per xterm.
                let stops = self.grid().tab_stops();
                let mut reply = String::from("\x1bP2$u");
                let mut first = true;
                for (idx, &set) in stops.iter().enumerate() {
                    if set {
                        if !first {
                            reply.push('/');
                        }
                        reply.push_str(&(idx + 1).to_string());
                        first = false;
                    }
                }
                reply.push_str("\x1b\\");
                reply.into_bytes()
            }
            other => {
                debug!("DECRQPSR: ignoring unknown mode {other}");
                return;
            }
        };
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes,
            kind: PtyWriteKind::StatusString,
        }));
    }

    /// DECRQUPSS (CSI & u) — Request User-Preferred Supplemental Set.
    ///
    /// Constant reply identifying ISO Latin-1 as the user-preferred
    /// supplemental set. NRCS charset selection is not implemented;
    /// marked `verified-with-deviation` in the catalog.
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "receiver stays `&mut self` so the method matches the `delegate_osc!` shape; `effect_sink.push(&self)` is interior-mutability"
    )]
    pub(super) fn decrqupss_impl(&mut self) {
        // DCS 1 ! u %5 ST — `%5` is the DEC designator for ISO Latin-1.
        let bytes = b"\x1bP1!u%5\x1b\\".to_vec();
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes,
            kind: PtyWriteKind::StatusString,
        }));
    }

    /// DECRQDE (CSI " v) — Request Displayed Extent.
    ///
    /// Replies with current visible-grid dimensions. xterm reply:
    /// `CSI Ph;Pw;Pml;Pmt;Pmp " w` where Ph = visible height, Pw =
    /// visible width, Pml = left margin, Pmt = top margin (both 1),
    /// Pmp = page number (1). `ori_term` has no multi-page support, so
    /// margin + page values are constant.
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "receiver stays `&mut self` so the method matches the `delegate_osc!` shape; `effect_sink.push(&self)` is interior-mutability"
    )]
    pub(super) fn decrqde_impl(&mut self) {
        let rows = self.grid().lines();
        let cols = self.grid().cols();
        let reply = format!("\x1b[{rows};{cols};1;1;1\"w");
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: reply.into_bytes(),
            kind: PtyWriteKind::StatusString,
        }));
    }

    /// DECSCL (CSI Pl;Pc " p) — Set Conformance Level.
    ///
    /// Stores the conformance level + C1 encoding mode as observable
    /// state, then triggers a soft reset (DECSTR). The parser's 8-bit
    /// C1 dispatch is NOT gated on `c1_7bit` — that is a vendored-
    /// parser concern documented as a deviation in the catalog.
    pub(super) fn decscl_impl(&mut self, level: u16, c1_mode: u16) {
        // xterm accepts Pl = 61 (VT100), 62 (VT200), 63 (VT300), 64
        // (VT400). We accept them verbatim as observable state.
        self.conformance_level = level;
        // Pc semantics: 0 or 2 = 8-bit C1 (default), 1 = 7-bit C1.
        self.c1_7bit = c1_mode == 1;
        debug!("DECSCL: level={level} c1_mode={c1_mode}");
        self.soft_reset();
    }

    /// DECSCA (CSI Ps " q) — Select Character Protection Attribute.
    ///
    /// Ps=0 or 2 → unprotected, Ps=1 → protected. The flag is stored
    /// on Term for observability AND applied to the cursor template
    /// so every subsequent `Grid::put_char`/`put_char_ascii` carries
    /// `CellFlags::PROTECTED` through the existing `cell.flags =
    /// template.flags | DRAWN` write path (same mechanism as SGR).
    /// Protected cells survive DECSERA (`CSI $ {`) but not DECERA.
    pub(super) fn decsca_impl(&mut self, protected: u16) {
        let enable = protected == 1;
        self.char_protection = enable;
        let template = &mut self.grid_mut().cursor_mut().template;
        if enable {
            template.flags.insert(CellFlags::PROTECTED);
        } else {
            template.flags.remove(CellFlags::PROTECTED);
        }
        debug!("DECSCA: char_protection={enable}");
    }

    /// DECSASD (CSI Ps $ }) — Select Active Status Display.
    ///
    /// 0 = main display, 1 = status line. `ori_term` does not render a
    /// DEC status line; the value is stored as observable state only.
    pub(super) fn decsasd_impl(&mut self, target: u16) {
        self.active_status_display = target;
        debug!("DECSASD: active_status_display={target}");
    }

    /// DECSSDT (CSI Ps $ ~) — Select Status Display Type.
    ///
    /// 0 = off, 1 = indicator, 2 = host-writable. `ori_term` does not
    /// render a status line; the value is stored as observable state.
    pub(super) fn decssdt_impl(&mut self, line_type: u16) {
        self.status_line_type = line_type;
        debug!("DECSSDT: status_line_type={line_type}");
    }

    /// DECIC (CSI Ps ' }) — Insert Column.
    ///
    /// Inserts `count` blank columns at the cursor column on every row
    /// in the active scroll region (xterm `util.c:819 xtermColScroll`
    /// loop). Content from the cursor column to the right edge of the
    /// DECLRMM band shifts right; cells past the right edge are lost.
    /// No-op when the cursor is outside the vertical scroll region or
    /// outside the DECLRMM horizontal band.
    pub(super) fn decic_impl(&mut self, count: u16) {
        let count = usize::from(count.max(1));
        let grid = self.grid();
        let at_col = grid.cursor().col().0;
        // Cursor outside the DECLRMM band → no-op (matches xterm guard
        // at `util.c:836`).
        if grid.has_horizontal_margins() {
            let (left, right) = grid.left_right_margins();
            if at_col < left || at_col > right {
                debug!("DECIC: cursor outside DECLRMM band — no-op");
                return;
            }
        }
        self.selection_dirty = true;
        self.grid_mut().insert_columns(count, at_col);
    }

    /// DECDC (CSI Ps ' ~) — Delete Column.
    ///
    /// Deletes `count` columns at the cursor column on every row in
    /// the active scroll region; remaining content shifts left with
    /// blanks filling the right edge of the DECLRMM band. No-op when
    /// cursor is outside the scroll region or outside the horizontal
    /// margin band.
    pub(super) fn decdc_impl(&mut self, count: u16) {
        let count = usize::from(count.max(1));
        let grid = self.grid();
        let at_col = grid.cursor().col().0;
        if grid.has_horizontal_margins() {
            let (left, right) = grid.left_right_margins();
            if at_col < left || at_col > right {
                debug!("DECDC: cursor outside DECLRMM band — no-op");
                return;
            }
        }
        self.selection_dirty = true;
        self.grid_mut().delete_columns(count, at_col);
    }

    /// DECBI (ESC 6) — Back Index.
    ///
    /// Cursor at the left margin (or column 0 when DECLRMM is inactive)
    /// AND inside the vertical scroll region: insert a blank column at
    /// the left margin on every row of the scroll region (scrolling
    /// the margin band right by one). Otherwise: move cursor one
    /// column left (bounded by the left margin / column 0). Mirrors
    /// `xterm util.c:788 xtermColIndex(toLeft=True)`.
    pub(super) fn decbi_impl(&mut self) {
        let grid = self.grid();
        let col = grid.cursor().col().0;
        let left_bound = if grid.has_horizontal_margins() {
            grid.left_right_margins().0
        } else {
            0
        };
        if col == left_bound {
            // Scroll-right path: cursor must also be inside the
            // vertical scroll region (xterm checks this via
            // `ScrnIsColInMargins` + the `xtermColScroll` row-span
            // guards).
            self.selection_dirty = true;
            self.grid_mut().insert_columns(1, left_bound);
        } else {
            self.grid_mut().move_backward(1);
        }
    }

    /// DECFI (ESC 9) — Forward Index.
    ///
    /// Cursor at the right margin (or last column when DECLRMM is
    /// inactive) AND inside the vertical scroll region: delete a
    /// column at the left margin on every row of the scroll region
    /// (scrolling the margin band left by one). Otherwise: move
    /// cursor one column right (bounded by the right margin / last
    /// column). Mirrors `xterm util.c:803 xtermColIndex(toLeft=False)`.
    pub(super) fn decfi_impl(&mut self) {
        let grid = self.grid();
        let col = grid.cursor().col().0;
        let (left_bound, right_bound) = if grid.has_horizontal_margins() {
            grid.left_right_margins()
        } else {
            (0, grid.cols().saturating_sub(1))
        };
        if col == right_bound {
            // xterm `util.c:805` issues `xtermColScroll(1, True,
            // ScrnLeftMargin)` — toLeft=True is a *delete* at the left
            // margin, which scrolls the band left by one and fills the
            // right margin with blanks.
            self.selection_dirty = true;
            self.grid_mut().delete_columns(1, left_bound);
        } else {
            self.grid_mut().move_forward(1);
        }
    }
}

#[cfg(test)]
mod tests;
