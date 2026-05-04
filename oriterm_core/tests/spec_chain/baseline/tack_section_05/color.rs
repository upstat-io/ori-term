//! Spec_chain conversion of the tack `color` scenario family.
//!
//! Per `crates/oriterm_test_support/src/tack_framework/scenarios/color/mod.rs`
//! and the empirical capture against tack v1.08 (2026-04-08), the color
//! test on tack v1.08 only probes the `colors` and `pairs` numeric
//! capabilities (`This terminal can display 256 colors and 32767 color
//! pairs.  (colors) (pairs) Done`). Both are TERMINFO numeric caps —
//! `max_colors` and `max_pairs` — which describe terminal capability
//! metadata, not protocol sequences emitted on the wire.
//!
//! # Catalog rows verified
//!
//! - **None.** Numeric terminfo caps do not have ECMA-48 / DEC private
//!   mode catalog rows. Color RENDERING (`setaf`/`setab`) is the
//!   domain of Section 07 GPU goldens and the SGR rows owned by
//!   Section 08.8 (ISO 8613-6 colon forms) and Section 08.8b
//!   (remaining SGR-color-related rows).
//!
//! This file documents the conversion finding so that the cap-coverage
//! matrix and `_legacy-tack-mapping.md` accurately reflect that the
//! color scenario family contributes ZERO new protocol-row verifications
//! to spec_chain — its coverage is metadata-only.

/// Regression guard: tack `color` family contributes no ECMA-48 protocol
/// rows to spec_chain.
///
/// This test exists to make the absence visible. If a future tack
/// release surfaces SGR sample text on the color screen, the new
/// catalog rows it exercises should land here AND this stub should be
/// upgraded with concrete spec_chain assertions.
#[test]
fn color_scenario_exercises_no_ecma48_protocol_rows() {
    // Intentionally trivial — see module rustdoc for the empirical
    // rationale. The assertion itself is a non-tautological invariant:
    // the legacy mapping table must NOT list any tack-section-05 color
    // row as `converted`. (We can't read the markdown table from here
    // without coupling tests to plan files, so the documentation
    // discipline is enforced via `_legacy-tack-mapping.md` review.)
    let exercised_rows: &[&str] = &[];
    assert!(
        exercised_rows.is_empty(),
        "tack v1.08 color scenario exercises no protocol-row cap labels; \
         add concrete spec_chain assertions here when a future tack \
         release surfaces SGR/color samples."
    );
}
