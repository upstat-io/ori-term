//! Pure predicates for the cursor-only fast-path gates.
//!
//! `compute_dispatch_fingerprint` and `evaluate_row_state_change` are pure
//! free functions read by `WindowRenderer::has_dispatch_change` and
//! `WindowRenderer::has_row_state_change` respectively. Co-locating them
//! here keeps `prepare/mod.rs` under the 500-line file-size cap and makes
//! both predicates non-GPU testable from `prepare/tests.rs` (no
//! `WindowRenderer` instance needed).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::super::frame_input::FrameInput;
use super::super::prepared_frame::PreparedFrame;
use super::resolve_cursor_state;

/// Content-aware fingerprint of every frame-level input that affects per-cell
/// instance emission. SSOT for the incremental dispatch decision — one hash +
/// one comparison + one tail write replace the prior parallel sync points.
///
/// Hashed inputs:
/// - viewport (`width`, `height`) — full viewport-pixel dimensions.
/// - full `CellMetrics` (`width`, `height`, `baseline`, `underline_offset`,
///   `stroke_size`, `strikeout_offset`) — every metric affects cell or
///   decoration emission. Hashing only the first 3 would silently replay
///   stale decoration geometry on font/scale changes.
/// - content grid dimensions (`content_cols`, `content_rows`).
/// - origin (x, y) — saved-tier rows carry pixel positions baked at emit time.
/// - per-cell alpha multipliers + palette overlay state via
///   `FramePalette::damage_fingerprint`.
/// - `subpixel_positioning` — flips between subpixel/glyphs writers.
/// - `search_fingerprint()` — already content-aware via
///   `FrameSearch::damage_fingerprint`.
///
/// NOT hashed (handled by per-row dirty tracking in `build_dirty_set` /
/// [`evaluate_row_state_change`]):
/// - selection snapshot, cursor row, hovered cell, cursor opacity.
///
/// `f32` fields are hashed via `.to_bits()` (bitwise-exact, not epsilon-
/// tolerant). Tiny float deltas now force full rebuild instead of stale
/// replay; the effect is extra rebuilds, not stale reuse.
pub(crate) fn compute_dispatch_fingerprint(input: &FrameInput, origin: (f32, f32)) -> u64 {
    let mut hasher = DefaultHasher::new();

    // Geometry — affects every cell's pixel position.
    input.viewport.width.hash(&mut hasher);
    input.viewport.height.hash(&mut hasher);

    // ALL 6 CellMetrics fields — underline/stroke/strikeout offsets affect
    // decoration emission and must invalidate on font/scale change.
    input.cell_size.width.to_bits().hash(&mut hasher);
    input.cell_size.height.to_bits().hash(&mut hasher);
    input.cell_size.baseline.to_bits().hash(&mut hasher);
    input.cell_size.underline_offset.to_bits().hash(&mut hasher);
    input.cell_size.stroke_size.to_bits().hash(&mut hasher);
    input.cell_size.strikeout_offset.to_bits().hash(&mut hasher);

    input.content_cols.hash(&mut hasher);
    input.content_rows.hash(&mut hasher);
    origin.0.to_bits().hash(&mut hasher);
    origin.1.to_bits().hash(&mut hasher);

    // Per-cell alpha multipliers — baked into per-cell instances at emit time.
    input.text_blink_opacity.to_bits().hash(&mut hasher);
    // Full palette overlay state via PaletteDamageKey — covers background,
    // foreground, cursor_color, selection_fg, selection_bg, opacity.
    // Hashing only opacity would let theme/reverse_video changes replay
    // stale per-cell colors from saved_tier.
    input.palette.damage_fingerprint().hash(&mut hasher);
    input.fg_dim.to_bits().hash(&mut hasher);

    // Atlas routing — flips between subpixel/glyphs writers in emit.rs.
    u8::from(input.subpixel_positioning).hash(&mut hasher);

    // INVARIANT: hash the Option, not the unwrapped tuple — the discriminant
    // distinguishes None from Some(all-zeros).
    input.search_fingerprint().hash(&mut hasher);

    hasher.finish()
}

/// Pure predicate for the cursor-only fast-path gate. SSOT for "did per-row-
/// deciding inputs change since the last rendered frame?"
///
/// Returns `true` when the cursor-only fast path MUST be bypassed (one of:
/// resolved cursor changed, selection snapshot changed, hovered cell changed,
/// Block-cursor color-exclusion threshold crossed).
///
/// Free function (not method) so non-GPU tests can drive it directly with
/// constructed `PreparedFrame` + `FrameInput`. Mirrors the
/// `compute_dispatch_fingerprint` shape — orchestrator state is read via
/// `prepared`, not via `&self` on `WindowRenderer`.
///
/// The cursor-opacity threshold-cross detection lives in the final clause
/// of this predicate. `cursor_opacity` is intentionally NOT
/// in `compute_dispatch_fingerprint` (perf invariant — would force full
/// rebuild every blink frame). The cursor-only fast path BYPASSES
/// `build_dirty_set` entirely, so the threshold cross MUST gate the fast
/// path here.
pub(crate) fn evaluate_row_state_change(
    prepared: &PreparedFrame,
    input: &FrameInput,
    cursor_opacity: f32,
) -> bool {
    // gates.rs is a sibling of mod.rs under prepare/; reach
    // resolve_cursor_state via super::.
    let cur_resolved = resolve_cursor_state(input);
    let cur_visible = cur_resolved.into_visible();
    if cur_visible != prepared.prev_resolved_cursor {
        return true;
    }
    let new_sel = input
        .selection
        .as_ref()
        .and_then(|s| s.damage_snapshot(input.rows()));
    if new_sel != prepared.prev_selection_snapshot {
        return true;
    }
    if input.hovered_cell != prepared.prev_hovered_cell {
        return true;
    }
    let cur_exclusion = Some(super::resolve::block_cursor_color_exclusion_active(
        &cur_resolved,
        cursor_opacity,
    ));
    if cur_exclusion != prepared.prev_block_cursor_color_exclusion_active {
        return true;
    }
    false
}
