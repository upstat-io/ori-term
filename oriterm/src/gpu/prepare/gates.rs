//! Pure predicates for dispatch and cursor fast-path gates.
//!
//! `compute_dispatch_fingerprint` and `evaluate_row_state_change` are pure
//! free functions read by `WindowRenderer::has_dispatch_change` and
//! `WindowRenderer::has_row_state_change` respectively.
//! `PreparedFrame::can_incremental` is the dispatch-eligibility predicate
//! consumed by `prepare_frame_shaped_into`. Co-locating them here keeps
//! `prepare/mod.rs` under the 500-line file-size cap and makes all three
//! predicates non-GPU testable from `prepare/tests.rs`.
//!
//! `compute_pane_damage_key` is the SSOT for multi-pane per-pane cache
//! invalidation. It layers `compute_dispatch_fingerprint_from_inputs`
//! (frame-level dispatch state) with `PaneRowState` (row-level inputs that
//! single-pane handles via per-row dirty but multi-pane must hash because
//! the cache reuses the entire `PreparedFrame` on hit).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use oriterm_core::RenderableCursor;

use super::super::frame_input::{
    FrameInput, MarkCursorOverride, PaletteDamageKey, SearchDamageKey,
    SelectionDamageSnapshot, ViewportSize,
};
use super::super::prepared_frame::PreparedFrame;
use super::resolve_cursor_state;
use crate::font::CellMetrics;

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
    compute_dispatch_fingerprint_from_inputs(&input.dispatch_fingerprint_inputs(origin))
}

/// Inputs hashed by the dispatch fingerprint — frame-level values that
/// affect per-cell instance emission. SSOT for the dispatch-eligibility
/// decision; consumers either project from `FrameInput` (single-pane via
/// `FrameInput::dispatch_fingerprint_inputs`) or populate directly from
/// snapshot + layout + local vars (multi-pane, BEFORE the extract block).
#[derive(Debug, Clone, Copy)]
pub(crate) struct DispatchFingerprintInputs {
    pub viewport: ViewportSize,
    pub cell_size: CellMetrics,
    pub content_cols: usize,
    pub content_rows: usize,
    pub origin: (f32, f32),
    pub text_blink_opacity: f32,
    pub palette: PaletteDamageKey,
    pub fg_dim: f32,
    pub subpixel_positioning: bool,
    pub search: Option<SearchDamageKey>,
}

/// SSOT hasher — consumed by both single-pane (via `compute_dispatch_fingerprint`)
/// and multi-pane (via `compute_pane_damage_key`).
pub(crate) fn compute_dispatch_fingerprint_from_inputs(inputs: &DispatchFingerprintInputs) -> u64 {
    let mut hasher = DefaultHasher::new();

    inputs.viewport.width.hash(&mut hasher);
    inputs.viewport.height.hash(&mut hasher);

    inputs.cell_size.width.to_bits().hash(&mut hasher);
    inputs.cell_size.height.to_bits().hash(&mut hasher);
    inputs.cell_size.baseline.to_bits().hash(&mut hasher);
    inputs.cell_size.underline_offset.to_bits().hash(&mut hasher);
    inputs.cell_size.stroke_size.to_bits().hash(&mut hasher);
    inputs.cell_size.strikeout_offset.to_bits().hash(&mut hasher);

    inputs.content_cols.hash(&mut hasher);
    inputs.content_rows.hash(&mut hasher);
    inputs.origin.0.to_bits().hash(&mut hasher);
    inputs.origin.1.to_bits().hash(&mut hasher);

    inputs.text_blink_opacity.to_bits().hash(&mut hasher);
    inputs.palette.hash(&mut hasher);
    inputs.fg_dim.to_bits().hash(&mut hasher);

    u8::from(inputs.subpixel_positioning).hash(&mut hasher);
    inputs.search.hash(&mut hasher);

    hasher.finish()
}

/// Per-pane row-state inputs that the multi-pane cache must hash into its
/// `damage_key`. Single-pane handles these via per-row dirty tracking inside
/// `build_dirty_set`; multi-pane has no per-row path so the cache key MUST
/// hash everything that prepare reads beyond the dispatch fingerprint.
///
/// Mirrors the inputs consumed by `evaluate_row_state_change` plus per-pane
/// focus + IME preedit state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct PaneRowState {
    /// Visibility-canonicalized resolved cursor — `None` when hidden.
    /// Hidden cursors compare equal regardless of underlying position
    /// (matches `evaluate_row_state_change` semantics via
    /// `RenderableCursor::into_visible`).
    pub resolved_cursor_visible: Option<RenderableCursor>,
    pub selection_snapshot: Option<SelectionDamageSnapshot>,
    pub hovered_cell: Option<(usize, usize)>,
    pub mark_cursor: Option<MarkCursorOverride>,
    /// `f32::to_bits()` of `cursor_opacity` — bitwise-exact like the
    /// fingerprint's f32 fields.
    pub cursor_opacity_bits: u32,
    pub block_cursor_color_exclusion_active: bool,
    /// Monotonic counter from `app::ime::ImeState::preedit_revision`. 0
    /// before first preedit; increments on every preedit mutation. Always 0
    /// for non-focused panes.
    pub preedit_revision: u64,
    pub window_focused: bool,
    /// Hash of the focused pane's `hovered_url_segments` (URL underline
    /// state). 0 for non-focused panes AND when no URL is hovered. Without
    /// this signal, releasing Ctrl while hovering a URL leaves a stale
    /// underline in the cached prepared frame — `hovered_url_segments` is a
    /// prepare input (consumed by `draw_url_hover_underline`) that the rest
    /// of the dispatch fingerprint does not cover.
    pub hovered_url_segments_hash: u64,
}

/// Compose dispatch fingerprint with per-pane row-state into one cache key.
///
/// Layers `compute_dispatch_fingerprint_from_inputs` (dispatch-eligibility
/// SSOT) with `PaneRowState`. Used by the multi-pane render path's per-pane
/// cache; single-pane uses the dispatch fingerprint alone + per-row dirty.
pub(crate) fn compute_pane_damage_key(
    dispatch: &DispatchFingerprintInputs,
    row_state: &PaneRowState,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    compute_dispatch_fingerprint_from_inputs(dispatch).hash(&mut hasher);
    row_state.hash(&mut hasher);
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

impl PreparedFrame {
    /// Predicate for the incremental-dispatch eligibility decision.
    ///
    /// Returns `true` when the previous frame's saved terminal-tier rows can be
    /// reused for the current frame. All three conditions must hold: the frame
    /// input does not force a full rebuild, the saved tier has cached rows, and
    /// the dispatch fingerprint matches the previous frame's.
    ///
    /// Co-located with [`compute_dispatch_fingerprint`] because the predicate
    /// and the fingerprint are a coupled pair — fingerprint is computed once,
    /// then consumed by both this predicate AND the post-prepare snapshot write.
    pub(super) fn can_incremental(&self, all_dirty: bool, fingerprint: u64) -> bool {
        !all_dirty
            && self.saved_tier.has_cached_rows()
            && self.prev_dispatch_fingerprint == Some(fingerprint)
    }
}
