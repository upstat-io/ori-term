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
    ALL_CONTRIBUTIONS, covered_caps, exempt_caps, expand_kf_caps, expand_modified_key_caps,
    parse_declared_caps,
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

    // Negative pin: as Sections 06 / 08 land, they MUST move
    // entries OUT of their `exempt` and INTO their `covered`.
    // A cap appearing in BOTH any section's `covered` AND any
    // section's `exempt` is a stale exemption — the matrix
    // fails loudly so the cleanup happens. The cleanup is part
    // of the 06.N / 08.N completion checklists.
    //
    // TPR-05-025 fix: the stale-exemption gate must scan BOTH
    // explicit per-section `contrib.exempt` slices AND the
    // iterator-built keyboard exemptions from `expand_kf_caps()`
    // and `expand_modified_key_caps()`. The iterator-built
    // exemptions are the dominant source of Section 08's keyboard
    // coverage gap (kf1..=kf63 + the modified-key family ~125
    // caps total). If Section 08 later moves any of those caps
    // into its `CONTRIBUTION.covered`, the cleanup contract
    // requires the iterator helper to ALSO be updated (or the
    // moved cap removed from the helper). Without scanning the
    // helpers here, Section 08 could move `kf1` into `covered`
    // while `expand_kf_caps()` still produces it, leaving the
    // exemption silently stale and the SSOT decay protection
    // broken.
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
    // ALSO check the iterator-built exemptions (TPR-05-025).
    for cap in expand_kf_caps() {
        if covered.contains(&cap) {
            stale_exemptions.push(format!(
                "{cap} (in expand_kf_caps() iterator AND in some section's covered — \
                 update expand_kf_caps() to remove this cap, OR remove the cap from \
                 the section's `covered` if the helper is the canonical owner)"
            ));
        }
    }
    for cap in expand_modified_key_caps() {
        if covered.contains(&cap) {
            stale_exemptions.push(format!(
                "{cap} (in expand_modified_key_caps() iterator AND in some section's covered — \
                 update expand_modified_key_caps() to remove this cap, OR remove the cap from \
                 the section's `covered` if the helper is the canonical owner)"
            ));
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
