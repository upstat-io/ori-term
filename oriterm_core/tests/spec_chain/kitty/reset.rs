//! Terminal-reset interactions with kitty image cache.
//!
//! Both DECSTR (`CSI ! p` — soft terminal reset) and RIS (`ESC c` — hard
//! reset) clear the kitty `ImageCache` image store AND every active
//! placement (cache-coordinate placements + U=1 placeholder anchors +
//! placeholder anchor grid) so a post-reset client cannot observe ghost
//! images or memory-leaked image data. Spec: kitty graphics-protocol
//! "When resetting the terminal, all images that are visible on the
//! screen must be cleared". WezTerm parity:
//! `kitty_remove_all_placements` on DECSTR.

use oriterm_test_support::spec_chain::SpecHarness;

use super::fixtures::{b64, kitty_apc, placement_count, rgba_4x4_red};

const RIS: &[u8] = b"\x1bc";
const DECSTR: &[u8] = b"\x1b[!p";
const CSI_CNL: &[u8] = b"\x1b[2E";

/// Seed two kitty placements on the active screen so the post-reset
/// assertions have non-trivial state to inspect:
///   - `i=1` is a non-U=1 cache-coordinate placement (`a=T`).
///   - `i=2` is a U=1 placeholder anchor (`a=T,U=1`).
///
/// Returns the `SpecHarness` ready for the reset sequence under test.
fn harness_with_two_kitty_placements() -> SpecHarness {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,i=1,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    h.feed(&kitty_apc(b"a=T,i=2,f=32,s=4,v=4,U=1", &b64(&rgba_4x4_red())));
    assert_eq!(
        h.term().image_cache().image_count(),
        2,
        "precondition: both kitty images stored",
    );
    assert_eq!(
        placement_count(&h),
        1,
        "precondition: non-U=1 image created a placement (U=1 image uses anchor)",
    );
    assert_eq!(
        h.term().image_cache().placeholder_anchors().len(),
        1,
        "precondition: U=1 image created a placeholder anchor",
    );
    h
}

/// RIS (`ESC c`) clears kitty image store + active placements + U=1
/// placeholder anchors.
#[test]
fn kitty_image_store_and_placements_cleared_on_ris_hard_reset() {
    let mut h = harness_with_two_kitty_placements();
    h.feed(RIS);

    let cache = h.term().image_cache();
    assert_eq!(
        cache.image_count(),
        0,
        "RIS MUST clear the kitty image store",
    );
    assert_eq!(
        cache.placement_count(),
        0,
        "RIS MUST clear active kitty placements",
    );
    assert!(
        cache.placeholder_anchors().is_empty(),
        "RIS MUST clear U=1 placeholder anchors",
    );
}

/// DECSTR (`CSI ! p`) clears kitty image store + active placements +
/// U=1 placeholder anchors.
#[test]
fn kitty_image_store_and_placements_cleared_on_decstr_soft_reset() {
    let mut h = harness_with_two_kitty_placements();
    h.feed(DECSTR);

    let cache = h.term().image_cache();
    assert_eq!(
        cache.image_count(),
        0,
        "DECSTR MUST clear the kitty image store",
    );
    assert_eq!(
        cache.placement_count(),
        0,
        "DECSTR MUST clear active kitty placements",
    );
    assert!(
        cache.placeholder_anchors().is_empty(),
        "DECSTR MUST clear U=1 placeholder anchors",
    );
}

/// Non-reset CSI sequences (e.g., `CSI 2 E` — CNL "cursor next line")
/// MUST NOT touch the kitty image store. Proves the DECSTR/RIS clear
/// is gated on the specific reset sequences, not any arbitrary CSI.
#[test]
fn kitty_image_store_persists_across_csi_cnl_or_other_non_reset_sequences() {
    let mut h = harness_with_two_kitty_placements();
    h.feed(CSI_CNL);

    let cache = h.term().image_cache();
    assert_eq!(
        cache.image_count(),
        2,
        "non-reset CSI MUST NOT clear the kitty image store",
    );
    assert_eq!(
        cache.placement_count(),
        1,
        "non-reset CSI MUST NOT clear active kitty placements",
    );
    assert_eq!(
        cache.placeholder_anchors().len(),
        1,
        "non-reset CSI MUST NOT clear U=1 placeholder anchors",
    );
}
