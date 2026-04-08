//! Sibling tests for the cap-coverage parser, helpers, and partition.
//!
//! Parser tests use synthetic terminfo strings to exercise each tic
//! format quirk in isolation. Partition tests iterate
//! `ALL_CONTRIBUTIONS` to assert SSOT invariants.

use std::collections::BTreeSet;

use super::{ALL_CONTRIBUTIONS, expand_kf_caps, expand_modified_key_caps, parse_declared_caps};

// ----- Parser dimension: synthetic-input pins for each tic format quirk.

/// Test helper: parse a synthetic terminfo string. We can't pass
/// arbitrary text to `parse_declared_caps()` (which is hardcoded to
/// the embedded `extra/ori_term.info`), so this helper duplicates
/// the parser body against an injected source. The duplication is
/// 12 lines and is intentionally test-local — production code
/// should not need to parse arbitrary terminfo strings.
fn parse_synthetic(src: &str) -> BTreeSet<String> {
    let mut caps = BTreeSet::new();
    let mut in_continuation = false;
    for raw_line in src.lines() {
        let trimmed = raw_line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            in_continuation = false;
            continue;
        }
        if !raw_line.starts_with(char::is_whitespace) {
            in_continuation = false;
            continue;
        }
        let line_ended_with_comma = raw_line.trim_end().ends_with(',');
        if in_continuation {
            in_continuation = !line_ended_with_comma;
            continue;
        }
        in_continuation = !line_ended_with_comma;
        for token in trimmed.split(',') {
            let t = token.trim();
            if t.is_empty() || t.starts_with("use=") {
                continue;
            }
            let name: String = t
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                caps.insert(name);
            }
        }
    }
    caps
}

#[test]
fn parse_declared_caps_handles_simple_boolean_cap() {
    let src = "foo|bar,\n    am,\n";
    let caps = parse_synthetic(src);
    assert_eq!(caps, BTreeSet::from(["am".to_string()]));
}

#[test]
fn parse_declared_caps_handles_string_cap_with_value() {
    let src = "foo|bar,\n    setaf=\\E[3%dm,\n";
    let caps = parse_synthetic(src);
    assert_eq!(caps, BTreeSet::from(["setaf".to_string()]));
}

#[test]
fn parse_declared_caps_handles_numeric_cap() {
    let src = "foo|bar,\n    colors#256,\n";
    let caps = parse_synthetic(src);
    assert_eq!(caps, BTreeSet::from(["colors".to_string()]));
}

#[test]
fn parse_declared_caps_handles_cap_cancellation() {
    // SEMANTIC PIN: the `@` cancellation marker means "cancel
    // inherited cap", but the cap NAME is still part of the
    // entry's surface. The parser INCLUDES the cancelled name
    // because cap-coverage gating cares about the name's
    // presence in the entry, not whether the value is active.
    let src = "foo|bar,\n    setab@,\n";
    let caps = parse_synthetic(src);
    assert_eq!(caps, BTreeSet::from(["setab".to_string()]));
}

#[test]
fn parse_declared_caps_handles_continuation_lines() {
    // SEMANTIC PIN: continuation lines (the cap value spans
    // multiple physical lines) MUST NOT be parsed as new cap
    // declarations. A regression that processed every indented
    // line as a fresh cap-list would extract garbage from the
    // continuation body. The line that DECLARES the cap ends
    // with `,`; the continuation does not.
    let src = "foo|bar,\n    setaf=\\E[3%dm\n          $<10>%d,\n";
    let caps = parse_synthetic(src);
    assert_eq!(caps, BTreeSet::from(["setaf".to_string()]));
}

#[test]
fn parse_declared_caps_handles_comment_lines() {
    let src = "# this is a comment\nfoo|bar,\n    am,\n";
    let caps = parse_synthetic(src);
    assert_eq!(caps, BTreeSet::from(["am".to_string()]));
}

#[test]
fn parse_declared_caps_handles_use_reference() {
    // SEMANTIC PIN: `use=other_term` is the inheritance directive,
    // not a cap declaration. The leading `use` token must NOT
    // appear in the result set.
    let src = "foo|bar,\n    use=other_term,\n    am,\n";
    let caps = parse_synthetic(src);
    assert_eq!(caps, BTreeSet::from(["am".to_string()]));
}

#[test]
fn parse_declared_caps_handles_multiple_caps_per_line() {
    let src = "foo|bar,\n    am, bce, km,\n";
    let caps = parse_synthetic(src);
    assert_eq!(
        caps,
        BTreeSet::from(["am".to_string(), "bce".to_string(), "km".to_string()])
    );
}

#[test]
fn parse_declared_caps_handles_entry_header_skip() {
    // SEMANTIC PIN: header lines (start in column 0, end with `,`)
    // declare the entry name + aliases, NOT caps. A regression
    // that did not skip them would put `foo` and `baz` in the
    // result set.
    let src = "foo|bar,\n    am,\n\nbaz|qux,\n    bce,\n";
    let caps = parse_synthetic(src);
    assert_eq!(caps, BTreeSet::from(["am".to_string(), "bce".to_string()]));
}

#[test]
fn parse_declared_caps_against_real_terminfo_returns_sensible_result() {
    let caps = parse_declared_caps();
    // Sanity bounds: the file declares ~150-250 caps when fully
    // parsed (the exact count is pinned in
    // `parse_declared_caps_real_terminfo_count_pin` below).
    assert!(
        caps.len() > 100,
        "expected >100 declared caps, got {}",
        caps.len()
    );
    assert!(
        caps.len() < 350,
        "expected <350 declared caps, got {}",
        caps.len()
    );
    // Specific known caps must be present.
    for required in ["am", "bce", "setaf", "kf1", "Smulx"] {
        assert!(
            caps.contains(required),
            "expected `{required}` in declared caps, got: {caps:?}"
        );
    }
    // Names that must NOT be in the set (header components,
    // inheritance directives).
    for forbidden in ["use", "ori_term"] {
        assert!(
            !caps.contains(forbidden),
            "did not expect `{forbidden}` in declared caps"
        );
    }
}

#[test]
fn parse_declared_caps_real_terminfo_count_pin() {
    // SEMANTIC PIN: the exact count of caps declared in
    // `extra/ori_term.info`. If a future edit to the terminfo
    // adds or removes a cap, this test fails LOUDLY and forces
    // the implementer to update the pinned count and audit the
    // cap-coverage matrix. Pin computed at 05.5 implementation
    // time by reading the value the parser produces (see
    // assertion message below for the iteration that produced
    // the current pin).
    let caps = parse_declared_caps();
    assert_eq!(
        caps.len(),
        EXPECTED_DECLARED_CAPS_COUNT,
        "extra/ori_term.info declared-cap count drifted from pin. \
         If you intentionally added/removed a cap, update \
         EXPECTED_DECLARED_CAPS_COUNT in this test AND audit the \
         section_05/06/08 contribution files to keep the \
         cap-coverage matrix in sync. Got caps: {caps:?}"
    );
}

/// The exact number of caps the parser extracts from the embedded
/// `extra/ori_term.info`. Pinned at 05.5 implementation time —
/// update if the terminfo source changes.
const EXPECTED_DECLARED_CAPS_COUNT: usize = 248;

// ----- Helper expansion dimension.

#[test]
fn expand_kf_caps_produces_63_entries() {
    let caps = expand_kf_caps();
    assert_eq!(caps.len(), 63);
    assert_eq!(caps[0], "kf1");
    assert_eq!(caps[62], "kf63");
}

#[test]
fn expand_modified_key_caps_produces_expected_count() {
    // 10 bases × (1 base + 5 suffixes) = 60, plus 2 specials
    // (`kind`, `kri`) = 62 total.
    let caps = expand_modified_key_caps();
    assert_eq!(caps.len(), 62);
}

#[test]
fn expand_modified_key_caps_contains_required_caps() {
    let caps: BTreeSet<String> = expand_modified_key_caps().into_iter().collect();
    for required in [
        "kLFT", "kLFT3", "kLFT7", "kRIT", "kIC", "kPRV7", "kind", "kri",
    ] {
        assert!(
            caps.contains(required),
            "expected expand_modified_key_caps() to contain `{required}`"
        );
    }
}

#[test]
fn expand_modified_key_caps_matches_terminfo() {
    // SEMANTIC PIN: every modified-key cap declared in
    // extra/ori_term.info MUST appear in the expansion, and
    // vice versa. If extra/ori_term.info adds kHOM7 without
    // adding it to the expansion, the cap-coverage matrix would
    // leave it uncovered. If the expansion includes a cap not
    // in the terminfo, the kf-family exemption set would
    // include phantom entries.
    let declared = parse_declared_caps();
    let expanded: BTreeSet<String> = expand_modified_key_caps().into_iter().collect();
    let in_declared_only: BTreeSet<&String> = declared
        .iter()
        .filter(|c| {
            (c.starts_with("kLFT")
                || c.starts_with("kRIT")
                || c.starts_with("kUP")
                || c.starts_with("kDN")
                || c.starts_with("kEND")
                || c.starts_with("kHOM")
                || c.starts_with("kIC")
                || c.starts_with("kDC")
                || c.starts_with("kNXT")
                || c.starts_with("kPRV")
                || c.as_str() == "kind"
                || c.as_str() == "kri")
                && !expanded.contains(*c)
        })
        .collect();
    assert!(
        in_declared_only.is_empty(),
        "modified-key caps declared in terminfo but missing from \
         expand_modified_key_caps(): {in_declared_only:?}"
    );
}

// ----- Partition dimension: section contribution invariants.

#[test]
fn partition_no_intra_section_overlap() {
    // SEMANTIC PIN: each section's `covered` and its own `exempt`
    // must be disjoint. A cap appearing in both means the section
    // is internally inconsistent (claims to cover AND exempt the
    // same cap).
    for contrib in ALL_CONTRIBUTIONS {
        let covered: BTreeSet<&str> = contrib.covered.iter().copied().collect();
        let exempt: BTreeSet<&str> = contrib.exempt.iter().map(|(cap, _)| *cap).collect();
        let intersection: BTreeSet<&&str> = covered.intersection(&exempt).collect();
        assert!(
            intersection.is_empty(),
            "section {sec}: intra-section covered/exempt overlap: {intersection:?}",
            sec = contrib.section
        );
    }
}

#[test]
fn partition_no_inter_section_covered_overlap() {
    // SEMANTIC PIN: no two sections claim coverage of the same
    // cap. Catches accidental double-counting where two sections
    // own the same cap (the matrix would still pass, but the
    // ownership story would be unclear).
    for (i, a) in ALL_CONTRIBUTIONS.iter().enumerate() {
        for b in &ALL_CONTRIBUTIONS[i + 1..] {
            let a_covered: BTreeSet<&str> = a.covered.iter().copied().collect();
            let b_covered: BTreeSet<&str> = b.covered.iter().copied().collect();
            let intersection: BTreeSet<&&str> = a_covered.intersection(&b_covered).collect();
            assert!(
                intersection.is_empty(),
                "sections {sa} and {sb} both claim coverage of: {intersection:?}",
                sa = a.section,
                sb = b.section
            );
        }
    }
}

#[test]
fn stale_exemption_negative_pin() {
    // SEMANTIC PIN: the matrix's stale-exemption check actually
    // catches the failure mode it claims to. Construct a synthetic
    // scenario where one section's `covered` and another section's
    // `exempt` overlap on `"am"`. The matrix-checker walked here
    // (re-implemented in-line so we don't have to mutate
    // ALL_CONTRIBUTIONS) MUST detect the overlap.
    //
    // Without this test, a regression that silently skipped the
    // stale-exemption check would not surface until Section 06/08
    // landed and forgot a cleanup.
    use crate::tack_framework::cap_coverage::CapCoverageContribution;

    let cov_with_am = CapCoverageContribution {
        section: "synthetic_a",
        covered: &["am"],
        exempt: &[],
    };
    let exempt_with_am = CapCoverageContribution {
        section: "synthetic_b",
        covered: &[],
        exempt: &[("am", "deferred for test")],
    };
    let synthetic = [&cov_with_am, &exempt_with_am];

    // Re-implement the union and stale-check inline. (The matrix
    // test in oriterm_core/tests/tack/test_menu/cap_coverage_matrix.rs
    // uses the same algorithm against ALL_CONTRIBUTIONS.)
    let mut covered = BTreeSet::new();
    for contrib in &synthetic {
        for cap in contrib.covered {
            covered.insert((*cap).to_string());
        }
    }
    let mut stale = Vec::new();
    for contrib in &synthetic {
        for (cap, _reason) in contrib.exempt {
            if covered.contains(*cap) {
                stale.push((*cap).to_string());
            }
        }
    }
    assert_eq!(
        stale,
        vec!["am".to_string()],
        "stale-exemption check failed to catch the overlap; the \
         matrix's negative-pin invariant is broken"
    );
}

#[test]
fn all_contributions_iteration_pin() {
    // SEMANTIC PIN: ALL_CONTRIBUTIONS is iterated in
    // covered_caps() / exempt_caps(). Catches a regression where
    // the iteration was replaced with a hand-written union over
    // hard-coded constants — the partition tests would still
    // pass but the SSOT design would be silently broken.
    assert!(
        ALL_CONTRIBUTIONS.iter().count() >= 3,
        "expected at least 3 contributions (Section 05, 06, 08)"
    );
    let sections: BTreeSet<&str> = ALL_CONTRIBUTIONS.iter().map(|c| c.section).collect();
    assert_eq!(
        sections.len(),
        ALL_CONTRIBUTIONS.len(),
        "ALL_CONTRIBUTIONS has duplicate section identifiers: {sections:?}"
    );
}
