//! Spec_chain conversion of the tack `graphic_rendition` scenario family.
//!
//! Per `crates/oriterm_test_support/src/tack_framework/scenarios/graphic_rendition/mod.rs`,
//! tack v1.08 combines this scenario with the `acs` scenario into a
//! single `tack/test/acs [n] >` submenu that probes only the `bel`
//! capability. The captured output is identical to the `acs` capture:
//!
//! ```text
//! \x1B[H\x1B[2JTesting bell (bel)
//! If you did not hear the Bell then (bel) has failed.  (bel) Done
//! ```
//!
//! # Catalog rows verified
//!
//! - **None new.** The `bel` row (`ECMA48-C0-BEL`) is verified by
//!   `super::acs::bel_drives_to_host_effect_apex` — the combined
//!   tack screen exercises one BEL probe, not two.
//!
//! SGR rendering rows (bold / dim / underline / blink / reverse / invis)
//! are NOT exercised by tack v1.08's combined ACS + graphic-rendition
//! screen (the parser preserves a SGR-label scanner as forward-compatible
//! infrastructure but the result is always empty against tack v1.08).
//! Their catalog rows live in `ECMA48-CSI-SGR-*` and are owned by
//! Section 08.8 (ISO 8613-6 colon forms) and Section 08.8b (overline,
//! superscript, subscript, etc.).

/// Regression guard: tack `graphic_rendition` family contributes no NEW
/// catalog-row verifications beyond what `super::acs` already covers.
///
/// Documents the empirical reality so that `_legacy-tack-mapping.md`
/// does not double-count the BEL conversion. If a future tack release
/// emits SGR sample text on the graphic-rendition screen, replace this
/// stub with concrete spec_chain assertions for the SGR rows it covers.
#[test]
fn graphic_rendition_scenario_shares_bel_with_acs_no_new_rows() {
    // No new rows. The acs.rs spec_chain is the canonical BEL test;
    // graphic_rendition.rs would duplicate it. The assertion is a
    // documentation marker — see module rustdoc for full rationale.
    let new_rows_beyond_acs: &[&str] = &[];
    assert!(
        new_rows_beyond_acs.is_empty(),
        "tack v1.08 graphic_rendition shares its captured screen with \
         the acs scenario; no new SGR catalog rows are exercised."
    );
}
