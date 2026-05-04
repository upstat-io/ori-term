//! Sibling tests for the `tack_cap_xcheck` module: meta-test +
//! 06.5.a compile gate.
//!
//! The meta-test asserts that [`super::NON_TACK_CAP_XCHECK_CAPS`]
//! and the union of every per-submodule [`REGISTERED`] slice
//! contain the SAME set of cap names. A regression that adds a
//! cap to one list without the other fires here with a set-diff
//! diagnostic.

use std::collections::BTreeSet;

use super::{NON_TACK_CAP_XCHECK_CAPS, XCHECK_REGISTERED_CAPS};

#[test]
fn tack_cap_xcheck_can_consume_test_helpers_from_sibling_module() {
    // 06.5.a compile gate — kept here as the load-bearing pin
    // that `super::super::test_helpers::*` is reachable from
    // `tack_cap_xcheck::tests`. If this stops compiling, the
    // visibility on `test_helpers` changed in a way that breaks
    // every per-submodule cap test.
    use super::super::test_helpers::{feed, term_with_recorder};

    let (mut term, listener) = term_with_recorder();
    feed(&mut term, b"x");
    assert_eq!(
        term.grid()[crate::index::Line(0)][crate::index::Column(0)].ch,
        'x'
    );
    assert!(listener.events().is_empty());
}

#[test]
fn tack_cap_xcheck_covers_every_non_tack_cap() {
    // META-TEST — assert that every cap declared in
    // NON_TACK_CAP_XCHECK_CAPS has a backing test (tracked via
    // its submodule's REGISTERED slice) and vice versa. The
    // dual-list pattern is the SSOT-bridge between "what caps
    // Section 06 owns" (NON_TACK_CAP_XCHECK_CAPS) and "what
    // tests actually exist" (the union of submodule REGISTERED
    // slices). A drift between the two is a regression.
    let owned: BTreeSet<&str> = NON_TACK_CAP_XCHECK_CAPS.iter().copied().collect();
    let registered: BTreeSet<&str> = XCHECK_REGISTERED_CAPS
        .iter()
        .flat_map(|slice| slice.iter().copied())
        .collect();

    let only_in_owned: Vec<&&str> = owned.difference(&registered).collect();
    let only_in_registered: Vec<&&str> = registered.difference(&owned).collect();

    assert!(
        only_in_owned.is_empty() && only_in_registered.is_empty(),
        "tack_cap_xcheck registry drift:\n\
         caps in NON_TACK_CAP_XCHECK_CAPS without a backing \
         submodule REGISTERED entry: {only_in_owned:?}\n\
         caps in submodule REGISTERED slices without a \
         NON_TACK_CAP_XCHECK_CAPS entry: {only_in_registered:?}\n\
         Add the missing entry to BOTH lists in the same commit \
         so the SSOT-bridge stays intact.",
    );
}

#[test]
fn tack_cap_xcheck_owned_list_has_no_duplicates() {
    // SSOT pin: NON_TACK_CAP_XCHECK_CAPS must contain each cap
    // exactly once. A duplicate would let two submodules claim
    // the same cap and pass the meta-test by accident.
    let owned: BTreeSet<&str> = NON_TACK_CAP_XCHECK_CAPS.iter().copied().collect();
    assert_eq!(
        owned.len(),
        NON_TACK_CAP_XCHECK_CAPS.len(),
        "NON_TACK_CAP_XCHECK_CAPS has duplicate entries; the \
         BTreeSet collapses {} unique caps from {} list entries.",
        owned.len(),
        NON_TACK_CAP_XCHECK_CAPS.len(),
    );
}

#[test]
fn tack_cap_xcheck_registered_caps_have_no_duplicates_across_submodules() {
    // SSOT pin: each cap belongs to EXACTLY one submodule. A
    // duplicate registration across submodules would mean two
    // different test fns claim ownership of the same cap, which
    // is an algorithmic-DRY violation.
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut duplicates: Vec<&str> = Vec::new();
    for slice in XCHECK_REGISTERED_CAPS {
        for &cap in *slice {
            if !seen.insert(cap) {
                duplicates.push(cap);
            }
        }
    }
    assert!(
        duplicates.is_empty(),
        "tack_cap_xcheck duplicate cap registration across \
         submodules: {duplicates:?}. Each cap must belong to \
         exactly one per-cap-family submodule.",
    );
}

#[test]
fn tack_cap_xcheck_owned_count_matches_section_06_plan() {
    // Verifies — Section 06.5's plan declares 23 direct-VTE
    // caps (19 escape-sequence-emitting + 4 pure-bool markers).
    // Pin the count so a refactor that removed an entry without
    // updating the plan flips this red.
    assert_eq!(
        NON_TACK_CAP_XCHECK_CAPS.len(),
        23,
        "Section 06.5 plan declares 23 direct-VTE caps; \
         NON_TACK_CAP_XCHECK_CAPS has {}. Update the plan \
         (cap_coverage matrix + Section 06.5 mission criterion) \
         in the same commit as any cap-list change.",
        NON_TACK_CAP_XCHECK_CAPS.len(),
    );
}
