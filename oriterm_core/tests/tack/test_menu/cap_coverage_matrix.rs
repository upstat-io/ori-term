//! Cap-coverage matrix: every cap declared in `extra/ori_term.info`
//! must be exercised by at least one Section 05 / 06 / 08 scenario,
//! OR be on a section's per-section `CapCoverageContribution::exempt`
//! list with a justification.
//!
//! This test does NOT spawn tack — it runs unconditionally on every
//! platform. Tack drift detection happens via the discovery test
//! (`begin_testing_inventory`) and the per-scenario tests; this is
//! the static SSOT gate that catches "added a cap to terminfo,
//! forgot to add a scenario."
//!
//! Owner-partitioned design (Pivot 5 of /review-plan): each
//! consuming section owns its own contribution; this test sums
//! them. There is no central `EXEMPT_CAPS` constant.

use oriterm_test_support::tack_framework::cap_coverage::{
    ALL_CONTRIBUTIONS, covered_caps, exempt_caps, parse_declared_caps,
};

#[test]
fn tack_cap_coverage_matrix() {
    let declared = parse_declared_caps();
    let covered = covered_caps();
    let exempt = exempt_caps();

    let uncovered: Vec<String> = declared
        .iter()
        .filter(|cap| !covered.contains(*cap) && !exempt.contains(*cap))
        .cloned()
        .collect();

    assert!(
        uncovered.is_empty(),
        "{} caps in extra/ori_term.info are not exercised by any \
         Section 05/06/08 scenario and not on any section's \
         `exempt` list:\n  {}\n\n\
         Either add a scenario that exercises them, or add an \
         entry to the owning section's \
         `CapCoverageContribution::exempt` with a justification \
         (and a `deferred to Section NN` note).",
        uncovered.len(),
        uncovered.join("\n  "),
    );

 // Regression guard: a cap appearing in BOTH any section's `covered`
    // AND any section's `exempt` is a stale exemption — the matrix
    // fails loudly so the cleanup happens.
    //
    // Note: expand_kf_caps() and expand_modified_key_caps() now feed
    // into covered_caps() (via section_08::covered_caps_08), not
    // exempt_caps(). The stale-exemption check only scans the
    // per-section `contrib.exempt` slices.
    let mut stale_exemptions: Vec<String> = Vec::new();
    for contrib in ALL_CONTRIBUTIONS {
        for (cap, _reason) in contrib.exempt {
            if covered.contains(*cap) {
                stale_exemptions.push(format!(
                    "{cap} (in section_{section}.exempt AND in some section's covered)",
                    section = contrib.section,
                ));
            }
        }
    }
    assert!(
        stale_exemptions.is_empty(),
        "Stale exemption entries — these caps are now in some \
         section's `covered` and should be REMOVED from the \
         exempting section's `exempt` (or from the iterator helper \
         that produces them):\n  {}",
        stale_exemptions.join("\n  "),
    );
}
