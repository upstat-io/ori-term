//! Shared sixel DCS fixtures + protocol-neutral placement helper for
//! `spec_chain` tests.
//!
//! Two concerns live here:
//!
//! 1. **Sixel-specific builders** (`dcs_n_bands_tall`, `dcs_n_cols_wide`):
//!    DCS prefix + payload helpers for §12 sixel tests (state machine,
//!    grid integration, lifecycle, cross-stack hand-off).
//!
//! 2. **Protocol-neutral `placement_count`**: snapshot-length probe
//!    reused across §12 sixel AND §13 kitty tests (via `pub(super) use`
//!    from `oriterm_core/tests/spec_chain/kitty/fixtures.rs`). The
//!    helper operates on `SpecHarness::term().renderable_content().images`
//!    which is protocol-agnostic — it counts any `RenderablePlacement`
//!    regardless of origin.
//!
//! The module is NOT renamed (yet) because all non-`placement_count`
//! helpers are sixel-specific; a split to `image_fixtures.rs` is tracked
//! under §13.1's Hygiene Findings block for when §14 iterm2 becomes the
//! third protocol consumer.

use super::SpecHarness;

/// DCS prefix `\x1bPq#0;2;100;0;0` (16 bytes) that opens every test
/// sixel payload in red. Named so the capacity calculations below are
/// self-auditing.
pub(super) const DCS_RED_PREFIX: &[u8] = b"\x1bPq#0;2;100;0;0";
/// ST terminator `\x1b\\` (2 bytes) that closes the DCS.
pub(super) const DCS_TERMINATOR: &[u8] = b"\x1b\\";

/// Sixel band height in pixels per the sixel protocol — one `~` glyph
/// = 6 vertical pixels.
pub(super) const PIXELS_PER_BAND: usize = 6;

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

/// Build a DCS-wrapped sixel body painting a solid red rectangle of
/// `width_px` pixels wide × `height_px` pixels tall (rounded up to the
/// nearest sixel band of `PIXELS_PER_BAND` pixels).
///
/// Use this for visual-regression pilots that need a cell-scaled sixel
/// — compute `width_px = cell_w * N_cells` and `height_px = cell_h`
/// at the call site from `harness.renderer().cell_metrics()`. PIXEL
/// parameter names (with `_px` suffix) block cell-vs-pixel confusion.
///
/// Uses sixel run-length encoding `!<N>~` so byte count stays bounded
/// even for 1000+ px widths (≤ 8 bytes per band regardless of width).
///
/// Zero-dimension inputs emit a valid empty DCS frame
/// (`DCS_RED_PREFIX` + `DCS_TERMINATOR`). The sixel parser would
/// otherwise silently clamp `!0~` to 1 sixel column
/// (`oriterm_core/src/image/sixel/mod.rs:176`), which would surprise
/// callers expecting an empty image.
pub fn dcs_red_pixel_block(width_px: usize, height_px: usize) -> Vec<u8> {
    use std::io::Write;
    let mut buf = Vec::new();
    buf.extend_from_slice(DCS_RED_PREFIX);
    if width_px == 0 || height_px == 0 {
        buf.extend_from_slice(DCS_TERMINATOR);
        return buf;
    }
    let bands = height_px.div_ceil(PIXELS_PER_BAND);
    for i in 0..bands {
        if i > 0 {
            buf.push(b'-');
        }
        write!(buf, "!{width_px}~").unwrap();
    }
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

#[cfg(test)]
mod tests;
