//! Facade-level unit tests for [`WindowRenderer`].
//!
//! Per-leaf unit tests live in sibling `tests.rs` files:
//! - `error/tests.rs` — `SurfaceError` display
//! - `icons/tests.rs` — `ICON_SIZES` registry
//! - `helpers/tests.rs` — raster-key generation
//! - `multi_pane/tests.rs` — chrome rendering (gpu-tests gated)
//! - `font_config/tests.rs` — font config (gpu-tests gated)
//!
//! This file retains CROSS-CUTTING facade tests that exercise multiple leaves
//! together: the `cache_invalidated_this_frame` SSOT propagation through
//! prepare + multi_pane + chrome consumption sites, and the source-scan
//! regression pin for chrome.rs's parallel-predicate consumption.

// --- Chrome render SSOT consolidation pins ---
//
// Pin the `cache_invalidated_this_frame` SSOT contract:
// - Constructor-init: both `WindowRenderer::new` and `new_ui_only` start
//   with the flag false (no frame prepared yet).
// - Multi-pane setter: `begin_multi_pane_frame` unconditionally sets the
//   flag true (multi-pane always rebuilds the aggregate prepared frame).
// - Chrome SSOT consumer: source-scan archaeology asserts `chrome.rs`
//   reads from `renderer.cache_invalidated_this_frame()` and does NOT
//   maintain a parallel `ChromeParams` predicate.

/// Regression: chrome.rs must read from
/// `WindowRenderer::cache_invalidated_this_frame()` and NOT carry a
/// parallel `ChromeParams::{content_dirty, selection_changed, blink_changed}`
/// predicate. Drift would re-introduce the SSOT violation; this test
/// fails at compile time if `chrome.rs` regresses.
///
/// See: bug-tracker/plans/BUG-06-033/section-03-tdd-matrix.md §"Parallel-
/// predicate regression pin (renderer-level surrogate + automated archaeology)"
#[test]
fn chrome_render_queries_renderer_ssot() {
    let chrome_src = include_str!("../../app/redraw/chrome.rs");

    // Positive: chrome reads from the SSOT query.
    assert!(
        chrome_src.contains("renderer.cache_invalidated_this_frame() || ctx.ui_stale"),
        "chrome.rs must read needs_full_render from renderer.cache_invalidated_this_frame() \
         (SSOT consolidation regression)"
    );

    // Negative: the old parallel-predicate fields must be gone from chrome.
    let banned = [
        "params.content_dirty",
        "params.selection_changed",
        "params.blink_changed",
    ];
    for forbidden in &banned {
        assert!(
            !chrome_src.contains(forbidden),
            "chrome.rs must NOT reference `{forbidden}` (parallel predicate removed in SSOT consolidation)"
        );
    }
}

#[cfg(feature = "gpu-tests")]
use super::WindowRenderer;
#[cfg(feature = "gpu-tests")]
use crate::gpu::ViewportSize;
#[cfg(feature = "gpu-tests")]
use crate::gpu::pipelines::GpuPipelines;
#[cfg(feature = "gpu-tests")]
use crate::gpu::state::GpuState;
#[cfg(feature = "gpu-tests")]
use oriterm_core::Rgb;

#[cfg(feature = "gpu-tests")]
use crate::gpu::visual_regression::headless_env;

/// Constructor pin: fresh `WindowRenderer::new` reports cache-NOT-invalidated.
///
/// No frame has been prepared yet, so the flag must default to false.
#[cfg(feature = "gpu-tests")]
#[test]
fn new_constructor_initializes_cache_invalidated_to_false() {
    let Some((_gpu, _pip, renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    assert!(
        !renderer.cache_invalidated_this_frame(),
        "WindowRenderer::new must initialize cache_invalidated_this_frame to false"
    );
}

/// Constructor pin: fresh `WindowRenderer::new_ui_only` reports cache-NOT-invalidated.
#[cfg(feature = "gpu-tests")]
#[test]
fn new_ui_only_constructor_initializes_cache_invalidated_to_false() {
    let Some(gpu) = GpuState::new_headless().ok() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let pipelines = GpuPipelines::new(&gpu);
    let font_set = crate::font::collection::FontSet::ui_embedded();
    let Some(ui_sizes) = crate::font::ui_font_sizes::UiFontSizes::new(
        font_set,
        96.0,
        crate::font::GlyphFormat::Alpha,
        crate::font::HintingMode::None,
        400,
        550,
        crate::font::ui_font_sizes::PRELOAD_SIZES,
    )
    .ok() else {
        eprintln!("SKIP: UI font setup failed");
        return;
    };
    let renderer = WindowRenderer::new_ui_only(&gpu, &pipelines, ui_sizes);
    assert!(
        !renderer.cache_invalidated_this_frame(),
        "WindowRenderer::new_ui_only must initialize cache_invalidated_this_frame to false"
    );
}

/// Multi-pane setter pin: `begin_multi_pane_frame` unconditionally sets
/// the flag true. Multi-pane always rebuilds the aggregate prepared
/// frame (no incremental fast path), so the SSOT must report invalidation.
///
/// Note: this is a setter-correctness pin, not a regression pin —
/// `multi_pane/mod.rs:122-127` already triggers `any_content_changed`
/// via `is_focused`, so the chrome stale-blit case the parallel
/// predicate missed was unreachable in production. The unconditional
/// `true` here matches the cleared-cache state, not closes a real bug.
#[cfg(feature = "gpu-tests")]
#[test]
fn begin_multi_pane_frame_sets_cache_invalidated_true() {
    let Some((_gpu, _pip, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let bg = Rgb { r: 0, g: 0, b: 0 };
    renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);
    assert!(
        renderer.cache_invalidated_this_frame(),
        "begin_multi_pane_frame must set cache_invalidated_this_frame=true \
         (multi-pane always rebuilds, no incremental fast path)"
    );
}

#[cfg(feature = "gpu-tests")]
#[test]
fn opacity_change_invalidates_cache() {
    let Some((_gpu, _pip, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };

    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);

    // Align input so only palette.opacity is the variable.
    input.text_blink_opacity = 1.0;
    input.fg_dim = 1.0;
    input.subpixel_positioning = false;
    input.selection = None;
    input.hovered_cell = None;

    // Seed prev_dispatch_fingerprint by computing for the baseline input.
    // Default PreparedFrame has prev_dispatch_fingerprint=None, so we
    // must publish a fingerprint first (mimicking what prepare does).
    input.palette.opacity = 1.0;
    let baseline = crate::gpu::prepare::compute_dispatch_fingerprint(&input, origin);
    renderer.prepared.prev_dispatch_fingerprint = Some(baseline);

    // Same opacity: no change.
    assert!(
        !renderer.has_dispatch_change(&input, origin),
        "opacity 1.0 to 1.0 must not invalidate fingerprint"
    );

    // Delta of 0.5 changes the fingerprint.
    input.palette.opacity = 0.5;
    assert!(
        renderer.has_dispatch_change(&input, origin),
        "opacity change from 1.0 to 0.5 must invalidate dispatch fingerprint"
    );

    // Bitwise-exact comparison replaces the prior `> 0.001` threshold.
    // Sub-EPSILON deltas now invalidate (extra rebuilds, never stale reuse).
    input.palette.opacity = 0.9995;
    assert!(
        renderer.has_dispatch_change(&input, origin),
        "bitwise-exact comparison must invalidate on any non-zero delta"
    );
}

/// Regression: BUG-06-030 — selection change MUST cause the full prepare
/// pass to run, NOT the cursor-only fast path. Calls `WindowRenderer::prepare`
/// directly and asserts on `cache_invalidated_this_frame()` so the integrated
/// dispatch semantic is exercised end-to-end.
/// See: bug-tracker/plans/completed/BUG-06-030/
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_skipped_when_selection_changes() {
    use oriterm_core::{Selection, Side, StableRowIndex};

    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };

    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;

    // Frame 1: baseline prepare with no selection. Establishes prev_*
    // state on the renderer's PreparedFrame so the fast-path predicate
    // has a meaningful prior to compare against.
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);

    // Frame 2: same content, no content change — would normally take
    // the cursor-only fast path. Add a selection: the row-state change
    // MUST force the full prepare pass.
    let mut sel = Selection::new_char(StableRowIndex(0), 0, Side::Left);
    sel.end = oriterm_core::SelectionPoint {
        row: StableRowIndex(0),
        col: 5,
        side: Side::Right,
    };
    input.selection = Some(crate::gpu::frame_input::FrameSelection::new(&sel, 0));
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        renderer.cache_invalidated_this_frame(),
        "selection change MUST force full prepare; fast path with stale selection \
         would replay stale decorations from saved_tier"
    );
}

#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_skipped_when_hovered_cell_changes() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };

    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);

    input.hovered_cell = Some((1, 2));
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        renderer.cache_invalidated_this_frame(),
        "hovered_cell change MUST force full prepare; fast path with stale hover \
         would replay stale hyperlink-underline decorations from saved_tier"
    );
}

/// Counter-pin: identical state across two prepare calls keeps the
/// fast path alive (cache_invalidated_this_frame == false after frame 2).
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_taken_when_selection_unchanged() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };

    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    // Frame 2: same input, content_changed=false — fast path eligible.
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        !renderer.cache_invalidated_this_frame(),
        "identical state across two frames must allow the cursor-only fast path"
    );
}

/// Counter-pin: identical hovered_cell state preserves fast-path eligibility.
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_taken_when_hovered_cell_unchanged() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };

    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = Some((2, 3));

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    // Frame 2: same hovered_cell.
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        !renderer.cache_invalidated_this_frame(),
        "identical hovered_cell across two frames must allow the cursor-only fast path"
    );
}

// --- Cursor position/shape/visibility row-state pins ---
/// Regression: BUG-06-040 — `WindowRenderer::has_row_state_change` did not
/// gate the cursor-only fast path on cursor position, shape, or visibility,
/// so cursor moves under a stationary content frame replayed stale per-cell
/// colors at the old + new cursor positions. The fix extends the predicate
/// with `prev_resolved_cursor: Option<RenderableCursor>` (visibility-
/// canonicalized: invisible cursors store as None).
/// See: bug-tracker/plans/completed/BUG-06-040/
#[cfg(feature = "gpu-tests")]
use oriterm_core::{Column, CursorShape, RenderableCursor};

/// Helper: seed `input.content.cursor` with full position/shape/visible state.
#[cfg(feature = "gpu-tests")]
fn set_cursor(
    input: &mut crate::gpu::frame_input::FrameInput,
    line: usize,
    col: usize,
    shape: CursorShape,
    visible: bool,
) {
    input.content.cursor = RenderableCursor {
        line,
        column: Column(col),
        shape,
        visible,
    };
}

/// Column-only move: cursor changes column on the same line without
/// content changing → MUST force full prepare to re-emit cell colors.
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_skipped_when_cursor_column_changes() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;
    set_cursor(&mut input, 0, 2, CursorShape::Block, true);

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    set_cursor(&mut input, 0, 5, CursorShape::Block, true);
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        renderer.cache_invalidated_this_frame(),
        "cursor column change MUST force full prepare; fast path with stale cursor \
         would replay stale per-cell colors at the OLD cursor position"
    );
}

/// Edge case: cursor changes line only.
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_skipped_when_cursor_line_changes() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;
    set_cursor(&mut input, 0, 3, CursorShape::Block, true);

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    set_cursor(&mut input, 2, 3, CursorShape::Block, true);
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        renderer.cache_invalidated_this_frame(),
        "cursor line change MUST force full prepare"
    );
}

/// Edge case: cursor changes both line and column.
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_skipped_when_cursor_line_and_column_change() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;
    set_cursor(&mut input, 1, 4, CursorShape::Block, true);

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    set_cursor(&mut input, 3, 7, CursorShape::Block, true);
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        renderer.cache_invalidated_this_frame(),
        "cursor line+column change MUST force full prepare"
    );
}

/// Edge case — same-position shape change (Block↔Underline):
/// `is_block_cursor_cell` gates on shape, so the cell at cursor position
/// resolves with different colors when shape changes.
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_skipped_when_cursor_shape_changes() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;
    set_cursor(&mut input, 2, 4, CursorShape::Block, true);

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    set_cursor(&mut input, 2, 4, CursorShape::Underline, true);
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        renderer.cache_invalidated_this_frame(),
        "cursor shape change at same position MUST force full prepare; \
         is_block_cursor_cell gates on shape, so stale Block-shape colors leak \
         when fast path replays without re-emission"
    );
}

/// Visibility-edge: visible → invisible flip.
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_skipped_when_cursor_visibility_flips_to_hidden() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;
    set_cursor(&mut input, 1, 2, CursorShape::Block, true);

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    set_cursor(&mut input, 1, 2, CursorShape::Block, false);
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        renderer.cache_invalidated_this_frame(),
        "cursor visibility flip (visible→invisible) MUST force full prepare"
    );
}

/// Visibility-edge: invisible → visible flip.
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_skipped_when_cursor_visibility_flips_to_visible() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;
    set_cursor(&mut input, 1, 2, CursorShape::Block, false);

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    set_cursor(&mut input, 1, 2, CursorShape::Block, true);
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        renderer.cache_invalidated_this_frame(),
        "cursor visibility flip (invisible→visible) MUST force full prepare"
    );
}

/// Mark-cursor edge: None → Some (mark mode appears, terminal cursor
/// stationary). Resolved cursor position jumps from terminal-cursor (0,0)
/// to mark-cursor (2,5) → fast path must reject.
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_skipped_when_mark_cursor_appears() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;
    set_cursor(&mut input, 0, 0, CursorShape::Block, true);
    input.mark_cursor = None;

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    input.mark_cursor = Some(crate::gpu::frame_input::MarkCursorOverride {
        line: 2,
        column: Column(5),
        shape: CursorShape::Block,
    });
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        renderer.cache_invalidated_this_frame(),
        "mark_cursor None→Some MUST force full prepare (resolved cursor jumps positions)"
    );
}

/// Mark-cursor edge: Some → None (mark mode disappears).
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_skipped_when_mark_cursor_disappears() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;
    set_cursor(&mut input, 0, 0, CursorShape::Block, true);
    input.mark_cursor = Some(crate::gpu::frame_input::MarkCursorOverride {
        line: 2,
        column: Column(5),
        shape: CursorShape::Block,
    });

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    input.mark_cursor = None;
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        renderer.cache_invalidated_this_frame(),
        "mark_cursor Some→None MUST force full prepare"
    );
}

/// Mark-cursor moves under the mark override (terminal cursor stationary).
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_skipped_when_mark_cursor_override_moves() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;
    set_cursor(&mut input, 0, 0, CursorShape::Block, true);
    input.mark_cursor = Some(crate::gpu::frame_input::MarkCursorOverride {
        line: 2,
        column: Column(5),
        shape: CursorShape::Block,
    });

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    input.mark_cursor = Some(crate::gpu::frame_input::MarkCursorOverride {
        line: 2,
        column: Column(8),
        shape: CursorShape::Block,
    });
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        renderer.cache_invalidated_this_frame(),
        "mark_cursor override move MUST force full prepare"
    );
}

/// CJK wide-char interaction: cursor moves off a CJK base cell. The
/// `is_block_cursor_cell` predicate handles `is_wide` for the cursor cell +
/// spacer cell pair; cursor-move off a CJK base must dirty both columns.
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_skipped_when_cursor_moves_off_cjk_wide_cell() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;
    // Mark cell (1, 2) as a wide CJK base; (1, 3) as the spacer.
    if let Some(cell) = input
        .content
        .cells
        .iter_mut()
        .find(|c| c.line == 1 && c.column == Column(2))
    {
        cell.ch = '中';
        cell.flags.insert(oriterm_core::CellFlags::WIDE_CHAR);
    }
    if let Some(cell) = input
        .content
        .cells
        .iter_mut()
        .find(|c| c.line == 1 && c.column == Column(3))
    {
        cell.flags.insert(oriterm_core::CellFlags::WIDE_CHAR_SPACER);
    }
    set_cursor(&mut input, 1, 2, CursorShape::Block, true);

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    set_cursor(&mut input, 1, 4, CursorShape::Block, true);
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        renderer.cache_invalidated_this_frame(),
        "cursor move off CJK base cell MUST force full prepare; \
         is_block_cursor_cell wide-char branch needs re-emission for the wide pair"
    );
}

/// Counter-pin: identical cursor state across two frames keeps the fast path.
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_taken_when_cursor_unchanged() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;
    set_cursor(&mut input, 2, 4, CursorShape::Block, true);

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        !renderer.cache_invalidated_this_frame(),
        "identical cursor state across two frames MUST keep the cursor-only fast path"
    );
}

/// Counter-pin: identical selection + cursor state keeps the fast path.
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_taken_when_selection_and_cursor_both_unchanged() {
    use oriterm_core::{Selection, Side, StableRowIndex};

    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.hovered_cell = None;
    set_cursor(&mut input, 2, 4, CursorShape::Block, true);

    let mut sel = Selection::new_char(StableRowIndex(2), 0, Side::Left);
    sel.end = oriterm_core::SelectionPoint {
        row: StableRowIndex(2),
        col: 10,
        side: Side::Right,
    };
    input.selection = Some(crate::gpu::frame_input::FrameSelection::new(&sel, 0));

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        !renderer.cache_invalidated_this_frame(),
        "identical selection+cursor across two frames MUST keep the cursor-only fast path"
    );
}

/// Counter-pin: an invisible cursor "moving" must NOT invalidate the fast
/// path. Storage rule canonicalizes invisible cursors to None — so two
/// consecutive invisible-cursor frames at different positions are
/// None == None at the predicate.
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_taken_when_invisible_cursor_moves() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;
    set_cursor(&mut input, 1, 2, CursorShape::Block, false);

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    set_cursor(&mut input, 3, 8, CursorShape::Block, false);
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        !renderer.cache_invalidated_this_frame(),
        "invisible cursor 'move' MUST NOT invalidate the fast path; \
         visibility-canonicalized storage rule (None for invisible) makes \
         hidden-to-hidden comparisons no-op"
    );
}

/// Counter-pin: mark_cursor active masks terminal cursor moves — resolved
/// cursor stays at the mark override, so terminal cursor changes under an
/// active mark don't invalidate.
#[cfg(feature = "gpu-tests")]
#[test]
fn fast_path_taken_when_mark_cursor_active_and_terminal_cursor_moves() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;
    set_cursor(&mut input, 0, 0, CursorShape::Block, true);
    input.mark_cursor = Some(crate::gpu::frame_input::MarkCursorOverride {
        line: 2,
        column: Column(5),
        shape: CursorShape::Block,
    });

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    // Move terminal cursor; mark override unchanged; resolved cursor stays at (2,5).
    set_cursor(&mut input, 0, 4, CursorShape::Block, true);
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);
    assert!(
        !renderer.cache_invalidated_this_frame(),
        "mark_cursor masks terminal cursor; terminal cursor moves under an active \
         mark override MUST NOT invalidate the fast path"
    );
}

/// Decode an instance's bg_color from the InstanceWriter byte buffer.
///
/// Uses `instance_writer::OFF_*` constants as the layout SSOT — the 96-byte
/// record shape is owned by `instance_writer/mod.rs`.
#[cfg(feature = "gpu-tests")]
fn decode_bg_at_pos(bytes: &[u8], target_x: f32, target_y: f32) -> Option<[f32; 4]> {
    use crate::gpu::instance_writer::{
        INSTANCE_SIZE, OFF_BG_A, OFF_BG_B, OFF_BG_G, OFF_BG_R, OFF_POS_X, OFF_POS_Y,
    };
    let read = |off: usize| f32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
    let count = bytes.len() / INSTANCE_SIZE;
    for i in 0..count {
        let base = i * INSTANCE_SIZE;
        if (read(base + OFF_POS_X) - target_x).abs() < 0.01
            && (read(base + OFF_POS_Y) - target_y).abs() < 0.01
        {
            return Some([
                read(base + OFF_BG_R),
                read(base + OFF_BG_G),
                read(base + OFF_BG_B),
                read(base + OFF_BG_A),
            ]);
        }
    }
    None
}

/// sRGB-to-linear conversion matches the GPU pipeline's color packing
/// (see `crate::gpu::srgb_to_linear`). Decoded instance bg_color values
/// are in linear space; expected values must convert to match.
#[cfg(feature = "gpu-tests")]
fn rgb_to_f32(r: u8, g: u8, b: u8) -> [f32; 4] {
    [
        crate::gpu::srgb_to_linear(r),
        crate::gpu::srgb_to_linear(g),
        crate::gpu::srgb_to_linear(b),
        1.0,
    ]
}

/// Behavioral pin: cursor moves within an active selection. Asserts:
/// (a) gate fires (cache invalidated), (b) full prepare re-emits the
/// same cell count, AND (c) actual bg_color at the OLD cursor cell is
/// selection-inverted while the NEW cursor cell is block-cursor-
/// suppressed (cell.bg). Decodes the instance buffer to read colors
/// directly from emitted bytes.
#[cfg(feature = "gpu-tests")]
#[test]
fn cell_colors_correct_after_cursor_moves_within_selection() {
    use oriterm_core::{Selection, Side, StableRowIndex};

    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.hovered_cell = None;
    set_cursor(&mut input, 1, 3, CursorShape::Block, true);

    // Capture the cell.fg/cell.bg pair (per FrameInput::test_grid setup).
    let cell_fg = input.content.cells[0].fg;
    let cell_bg = input.content.cells[0].bg;

    let mut sel = Selection::new_char(StableRowIndex(1), 0, Side::Left);
    sel.end = oriterm_core::SelectionPoint {
        row: StableRowIndex(1),
        col: 10,
        side: Side::Right,
    };
    input.selection = Some(crate::gpu::frame_input::FrameSelection::new(&sel, 0));

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    let frame1_bg_bytes = renderer.prepared.backgrounds.byte_len();
    assert!(frame1_bg_bytes > 0, "frame 1 must emit instances");

    // Frame 2: cursor moves within the selection.
    set_cursor(&mut input, 1, 7, CursorShape::Block, true);
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);

    assert!(
        renderer.cache_invalidated_this_frame(),
        "cursor move within selection MUST force full prepare"
    );
    assert_eq!(
        renderer.prepared.backgrounds.byte_len(),
        frame1_bg_bytes,
        "frame 2 must re-emit the same per-cell instance count"
    );

    // Cell positions: test_grid uses 8.0 × 16.0 cells.
    // OLD cursor (1, 3) at pixel (24.0, 16.0) — must now be selection-inverted.
    // NEW cursor (1, 7) at pixel (56.0, 16.0) — must be block-cursor-suppressed.
    let bytes = renderer.prepared.backgrounds.as_bytes();

    // OLD cursor cell — selection inversion swaps fg↔bg, so bg_color = cell.fg.
    let old_bg = decode_bg_at_pos(bytes, 24.0, 16.0)
        .expect("OLD cursor cell (1,3) bg instance MUST be in the buffer");
    assert_eq!(
        old_bg,
        rgb_to_f32(cell_fg.r, cell_fg.g, cell_fg.b),
        "OLD cursor cell at (1,3) MUST emit selection-inverted bg = cell.fg \
         after frame 2; stale Block-cursor-suppressed colors would leak \
         without the prev_resolved_cursor fast-path gate"
    );

    // NEW cursor cell — block-cursor-suppressed, so bg_color = cell.bg.
    let new_bg = decode_bg_at_pos(bytes, 56.0, 16.0)
        .expect("NEW cursor cell (1,7) bg instance MUST be in the buffer");
    assert_eq!(
        new_bg,
        rgb_to_f32(cell_bg.r, cell_bg.g, cell_bg.b),
        "NEW cursor cell at (1,7) MUST emit block-cursor-suppressed bg = cell.bg"
    );
}

/// Behavioral pin: cursor moves within an active search match. Asserts
/// (a) gate fires, (b) full prepare re-emits, AND (c) actual bg_color
/// at OLD cursor cell is SEARCH_MATCH_BG (yellow) while NEW cursor cell
/// is block-cursor-suppressed.
#[cfg(feature = "gpu-tests")]
#[test]
fn cell_colors_correct_after_cursor_moves_within_search_match() {
    use oriterm_core::{SearchMatch, StableRowIndex};

    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("SKIP: GPU adapter unavailable");
        return;
    };
    let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
    let origin = (0.0_f32, 0.0_f32);
    input.selection = None;
    input.hovered_cell = None;
    set_cursor(&mut input, 2, 6, CursorShape::Block, true);

    let cell_bg = input.content.cells[0].bg;
    // SEARCH_MATCH_BG from prepare/resolve.rs:14-18 — yellow-tinted.
    let search_match_bg = rgb_to_f32(100, 100, 30);

    // Search match covers row 2 cols 4-10 (focused index 999 = no focus).
    let m = SearchMatch {
        start_row: StableRowIndex(2),
        start_col: 4,
        end_row: StableRowIndex(2),
        end_col: 10,
    };
    input.search = Some(crate::gpu::frame_input::FrameSearch::for_test(
        vec![m],
        999,
        0,
    ));

    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    let frame1_bg_bytes = renderer.prepared.backgrounds.byte_len();
    assert!(frame1_bg_bytes > 0, "frame 1 must emit instances");

    // Frame 2: cursor moves to (2, 4); content + search unchanged.
    set_cursor(&mut input, 2, 4, CursorShape::Block, true);
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, false);

    assert!(
        renderer.cache_invalidated_this_frame(),
        "cursor move under stationary content MUST force full prepare"
    );
    assert_eq!(
        renderer.prepared.backgrounds.byte_len(),
        frame1_bg_bytes,
        "frame 2 must re-emit the same per-cell instance count"
    );

    // OLD cursor (2, 6) at (48.0, 32.0) — must now show SEARCH_MATCH_BG.
    // NEW cursor (2, 4) at (32.0, 32.0) — must show block-cursor-suppressed cell.bg.
    let bytes = renderer.prepared.backgrounds.as_bytes();

    let old_bg =
        decode_bg_at_pos(bytes, 48.0, 32.0).expect("OLD cursor cell (2,6) MUST be in the buffer");
    assert_eq!(
        old_bg, search_match_bg,
        "OLD cursor cell at (2,6) MUST emit SEARCH_MATCH_BG after cursor moves \
         off — stale block-cursor suppression would leak without the gate"
    );

    let new_bg =
        decode_bg_at_pos(bytes, 32.0, 32.0).expect("NEW cursor cell (2,4) MUST be in the buffer");
    assert_eq!(
        new_bg,
        rgb_to_f32(cell_bg.r, cell_bg.g, cell_bg.b),
        "NEW cursor cell at (2,4) MUST emit block-cursor-suppressed bg = cell.bg \
         (search highlight suppressed by is_block_cursor_cell)"
    );
}
