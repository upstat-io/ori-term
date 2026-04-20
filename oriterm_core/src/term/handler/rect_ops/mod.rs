//! DEC private rectangular-area handler implementations.
//!
//! Scaffolding module added by spec-conformance §09A.4. Hosts the
//! inherent helper methods on `Term<S>` that the `Handler` trait impl
//! in `super::mod.rs` delegates to for DECSACE / DECCARA / DECRARA /
//! DECCRA / DECFRA / XTCHECKSUM / DECRQCRA / DECERA / DECSERA /
//! XTREPORTSGR.
//!
//! §09A.4 lands the structural wiring only — each helper is a debug-
//! traced no-op. Real semantics land in §09A.5 (DECRQCRA checksum)
//! and §09A.6 (the six rectangular-area mutation ops + DECSACE extent
//! selection + XTCHECKSUM flag register + XTREPORTSGR reply).

// §09A.4 stubs don't touch `self` yet; §09A.5/§09A.6 replace each body
// with grid-mutating logic that satisfies both lints.
#![expect(
    clippy::unused_self,
    reason = "§09A.4 scaffolding stubs — real impls in §09A.5/§09A.6 use self"
)]
#![expect(
    clippy::needless_pass_by_ref_mut,
    reason = "§09A.4 scaffolding stubs — real impls in §09A.5/§09A.6 mutate self"
)]

use log::debug;

use crate::effect::sink::EffectSink;

use super::super::Term;

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
    pub(super) fn xtchecksum_impl(&mut self, flags: u16) {
        debug!("XTCHECKSUM: flags=0x{flags:04x} (stub — §09A.5)");
    }

    /// DECRQCRA (CSI Pi;Pg;Pt;Pl;Pb;Pr * y) — Request Checksum of
    /// Rectangular Area.
    #[expect(
        clippy::too_many_arguments,
        reason = "DECRQCRA spec: id + page + top/left/bot/right — 6 distinct param slots (id, page, 4 rect coords)"
    )]
    pub(super) fn decrqcra_impl(
        &mut self,
        id: u16,
        page: u16,
        top: u16,
        left: u16,
        bot: u16,
        right: u16,
    ) {
        debug!(
            "DECRQCRA: id={id} page={page} rect=({top},{left})-({bot},{right}) \
             (stub — §09A.5 emits DCS Pi !~ <hex> ST)"
        );
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
