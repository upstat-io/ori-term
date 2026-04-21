//! Shared sixel DCS fixtures for spec-conformance tests.
//!
//! SSOT for the common sixel DCS byte builders + renderable-snapshot
//! placement-count helper consumed by the §12 sixel `spec_chain` tests
//! (state machine, grid integration, lifecycle, cross-stack hand-off).
//! Extracting here removes the duplication flagged by the §12.4-§12.5
//! bundled TPR round 0 + bundled hygiene round (`placement_count`
//! 3-site duplicate) and matches the sibling-file helper contract in
//! `.claude/rules/test-organization.md` §6 and
//! `.claude/rules/impl-hygiene.md` §Algorithmic DRY.

use super::SpecHarness;

/// DCS prefix `\x1bPq#0;2;100;0;0` (16 bytes) that opens every test
/// sixel payload in red. Named so the capacity calculations below are
/// self-auditing.
const DCS_RED_PREFIX: &[u8] = b"\x1bPq#0;2;100;0;0";
/// ST terminator `\x1b\\` (2 bytes) that closes the DCS.
const DCS_TERMINATOR: &[u8] = b"\x1b\\";

/// Build a DCS-wrapped sixel body N pixel-bands tall, each band 6 pixels
/// = one `~` followed by a `-` NL between bands. Color is red
/// `#0;2;100;0;0`. Resulting image: 1 pixel wide × 6*N pixels tall.
pub fn dcs_n_bands_tall(bands: usize) -> Vec<u8> {
    // bands data = N `~` bytes + (N-1) `-` separators = 2*N-1 bytes
    // (clamped to 0 when bands=0).
    let data_bytes = bands * 2 - bands.min(1);
    let mut buf = Vec::with_capacity(DCS_RED_PREFIX.len() + data_bytes + DCS_TERMINATOR.len());
    buf.extend_from_slice(DCS_RED_PREFIX);
    for i in 0..bands {
        if i > 0 {
            buf.push(b'-');
        }
        buf.push(b'~');
    }
    buf.extend_from_slice(DCS_TERMINATOR);
    buf
}

/// Build a DCS-wrapped sixel body N pixel-columns wide (N `~` in one
/// band). Color is red `#0;2;100;0;0`. Resulting image: N pixels wide ×
/// 6 pixels tall.
pub fn dcs_n_cols_wide(cols: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(DCS_RED_PREFIX.len() + cols + DCS_TERMINATOR.len());
    buf.extend_from_slice(DCS_RED_PREFIX);
    buf.extend(std::iter::repeat_n(b'~', cols));
    buf.extend_from_slice(DCS_TERMINATOR);
    buf
}

/// Count image placements in the harness's renderable snapshot.
///
/// Shared across §12's sibling test files (`state_machine.rs`,
/// `invariants.rs`, `grid_integration.rs`, `lifecycle.rs`) instead of
/// being defined identically in each.
pub fn placement_count(harness: &SpecHarness) -> usize {
    harness.term().renderable_content().images.len()
}
