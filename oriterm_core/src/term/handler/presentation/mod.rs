//! DEC private presentation / column / back-forward-index handler
//! implementations.
//!
//! Scaffolding module added by spec-conformance §09A.4. Hosts the
//! inherent helper methods on `Term<S>` that the `Handler` trait impl
//! in `super::mod.rs` delegates to for DECRQPSR / DECRQUPSS / DECRQDE /
//! DECSCL / DECSCA / DECSASD / DECSSDT (presentation queries, CSI
//! path), DECIC / DECDC (column insert/delete), and DECBI / DECFI
//! (back/forward index, ESC path).
//!
//! §09A.4 lands the structural wiring only — each helper is a debug-
//! traced no-op. Real semantics land in §09A.7 (column + index ops),
//! §09A.8 (CSI-path presentation queries), and §09A.9 (DCS-path
//! DECRQSS / DECRSPS).

// §09A.4 stubs don't touch `self` yet; §09A.7/§09A.8 replace each body
// with grid/cursor/mode-mutating logic that satisfies both lints.
#![expect(
    clippy::unused_self,
    reason = "§09A.4 scaffolding stubs — real impls in §09A.7/§09A.8/§09A.9 use self"
)]
#![expect(
    clippy::needless_pass_by_ref_mut,
    reason = "§09A.4 scaffolding stubs — real impls in §09A.7/§09A.8/§09A.9 mutate self"
)]

use log::debug;

use crate::effect::sink::EffectSink;

use super::super::Term;

impl<S: EffectSink> Term<S> {
    /// DECRQPSR (CSI Ps $ w) — Request Presentation State Report.
    pub(super) fn decrqpsr_impl(&mut self, mode: u16) {
        debug!("DECRQPSR: mode={mode} (stub — §09A.8)");
    }

    /// DECRQUPSS (CSI & u) — Request User-Preferred Supplemental Set.
    pub(super) fn decrqupss_impl(&mut self) {
        debug!("DECRQUPSS: (stub — §09A.8)");
    }

    /// DECRQDE (CSI " v) — Request Displayed Extent.
    pub(super) fn decrqde_impl(&mut self) {
        debug!("DECRQDE: (stub — §09A.8)");
    }

    /// DECSCL (CSI Pl;Pc " p) — Set Conformance Level.
    pub(super) fn decscl_impl(&mut self, level: u16, c1_mode: u16) {
        debug!("DECSCL: level={level} c1_mode={c1_mode} (stub — §09A.8)");
    }

    /// DECSCA (CSI Ps " q) — Select Character Protection Attribute.
    pub(super) fn decsca_impl(&mut self, protected: u16) {
        debug!("DECSCA: protected={protected} (stub — §09A.8)");
    }

    /// DECSASD (CSI Ps $ }) — Select Active Status Display.
    pub(super) fn decsasd_impl(&mut self, target: u16) {
        debug!("DECSASD: target={target} (stub — §09A.8)");
    }

    /// DECSSDT (CSI Ps $ ~) — Select Status Display Type.
    pub(super) fn decssdt_impl(&mut self, line_type: u16) {
        debug!("DECSSDT: line_type={line_type} (stub — §09A.8)");
    }

    /// DECIC (CSI Ps ' }) — Insert Column.
    pub(super) fn decic_impl(&mut self, count: u16) {
        debug!("DECIC: count={count} (stub — §09A.7)");
    }

    /// DECDC (CSI Ps ' ~) — Delete Column.
    pub(super) fn decdc_impl(&mut self, count: u16) {
        debug!("DECDC: count={count} (stub — §09A.7)");
    }

    /// DECBI (ESC 6) — Back Index.
    pub(super) fn decbi_impl(&mut self) {
        debug!("DECBI: (stub — §09A.7)");
    }

    /// DECFI (ESC 9) — Forward Index.
    pub(super) fn decfi_impl(&mut self) {
        debug!("DECFI: (stub — §09A.7)");
    }
}

#[cfg(test)]
mod tests;
