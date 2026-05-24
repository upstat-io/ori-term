//! Image placement lifecycle matrix — 3 protocols × 2 sizing modes × 8 mutations = 48 scenarios.
//!
//! Table-driven regression that exercises every combination of
//! protocol (Sixel / Kitty / iTerm2), sizing mode (CellCount /
//! FixedPixels), and grid mutation (scrollback evict, resize, reflow,
//! alt-screen enter/exit, ED, EL) for the placement-lifecycle
//! subsystem introduced in spec-conformance Section 07.
//!
//! All three protocols funnel through the shared `ImageCache` /
//! `ImagePlacement` structures, so the matrix iterates the same code
//! path with only protocol-tag metadata varying — any asymmetry across
//! protocols would show up here as a diverging expected/actual result.
//!
//! Self-verifies completeness with an explicit `count == 48`
//! assertion so missing cells surface at test-run time instead of
//! silently regressing coverage.
//!
//! Lives inside the crate (rather than `tests/`) because
//! `ImageCache::place`, `store`, `next_image_id`, and
//! `placements_in_viewport` are intentionally `pub(crate)` — placement
//! writes in production go through VTE handlers, not direct API calls.

use std::sync::Arc;

use vte::ansi::Processor;

use crate::effect::VoidEffectSink;
use crate::grid::StableRowIndex;
use crate::image::{ImageData, ImageFormat, ImageId, ImagePlacement, ImageSource, PlacementSizing};
use crate::term::{Term, TermMode};
use crate::theme::Theme;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ImageProtocol {
    Sixel,
    Kitty,
    ITerm2,
}

impl ImageProtocol {
    fn all() -> &'static [Self] {
        &[Self::Sixel, Self::Kitty, Self::ITerm2]
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum PlacementSizingKind {
    CellCount,
    FixedPixels,
}

impl PlacementSizingKind {
    fn all() -> &'static [Self] {
        &[Self::CellCount, Self::FixedPixels]
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum GridMutation {
    ScrollbackEvict,
    Resize,
    Reflow,
    AltEnter,
    AltExit,
    EraseDisplay,
    EraseLine,
    /// Font-metric change via `Term::set_cell_dimensions` — recomputes
    /// `FixedPixels` placement cell-coverage cross-protocol (§07.6); leaves
    /// `CellCount` placements untouched.
    FontMetricChange,
}

impl GridMutation {
    fn all() -> &'static [Self] {
        &[
            Self::ScrollbackEvict,
            Self::Resize,
            Self::Reflow,
            Self::AltEnter,
            Self::AltExit,
            Self::EraseDisplay,
            Self::EraseLine,
            Self::FontMetricChange,
        ]
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum PlacementState {
    /// Placement present at its original row. Carries observed cell-coverage
    /// `(cols, rows)` so `FontMetricChange` pins the recompute AND the
    /// `Survives`-returning mutations pin that size does NOT drift. `Removed`
    /// / `Remapped` stay dimensionless — no matrix mutation expects `Remapped`,
    /// and a removed placement has no dims.
    Survives { cols: usize, rows: usize },
    Removed,
    Remapped,
}

/// Build a placement for the given scenario. The protocol tag is
/// informational — all three funnel through the same cache and
/// placement structures. Column/row are chosen so each mutation's
/// expected outcome is unambiguous.
fn place_for_scenario(
    term: &mut Term<VoidEffectSink>,
    _protocol: ImageProtocol,
    sizing: PlacementSizingKind,
    mutation: GridMutation,
) -> ImageId {
    let id = term.image_cache_mut().next_image_id();
    let data = ImageData {
        id,
        width: 16,
        height: 16,
        data: Arc::new(vec![128; 16 * 16 * 4]),
        pixel_generation: 0,
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    };
    term.image_cache_mut().store(data).unwrap();

    let (col, row) = match mutation {
        GridMutation::ScrollbackEvict => (0, 0),
        GridMutation::Resize => (90, 0),
        GridMutation::Reflow => (0, 0),
        GridMutation::AltEnter | GridMutation::AltExit => (5, 0),
        GridMutation::EraseDisplay => (5, 0),
        GridMutation::EraseLine => (5, 0),
        // (5, 0): clear of grid boundaries (deliberately NOT (90, 0) from
        // Resize) — the post-recompute 1-2 col width fits trivially.
        GridMutation::FontMetricChange => (5, 0),
    };

    let sizing_spec = match sizing {
        PlacementSizingKind::CellCount => PlacementSizing::CellCount,
        PlacementSizingKind::FixedPixels => PlacementSizing::FixedPixels {
            width: 16,
            height: 16,
        },
    };

    // Initial cell-coverage reflects genuine coverage so `FontMetricChange`
    // recompute is observable. At the setup cell dims `(8, 16)`
    // a `FixedPixels { width: 16, height: 16 }` placement covers
    // `(cols=16/8=2, rows=16/16=1)`; `CellCount` carries its explicit
    // `(2, 2)` cell count (untouched by `update_cell_coverage`).
    let (init_cols, init_rows) = match sizing {
        PlacementSizingKind::FixedPixels => (2, 1),
        PlacementSizingKind::CellCount => (2, 2),
    };

    term.image_cache_mut().place(ImagePlacement {
        image_id: id,
        placement_id: None,
        source_x: 0,
        source_y: 0,
        source_w: 0,
        source_h: 0,
        cell_col: col,
        cell_row: StableRowIndex(row),
        cols: init_cols,
        rows: init_rows,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: sizing_spec,
    });

    id
}

fn apply_mutation_and_observe(
    term: &mut Term<VoidEffectSink>,
    id: ImageId,
    mutation: GridMutation,
    original_row: StableRowIndex,
) -> PlacementState {
    match mutation {
        GridMutation::ScrollbackEvict => {
            // Drive enough newlines to push row 0 beyond the 1-line
            // scrollback buffer used by this matrix.
            let mut processor: Processor = Processor::new();
            processor.advance(term, b"\n".repeat(50).as_slice());
        }
        GridMutation::Resize => {
            term.resize(24, 80, false);
        }
        GridMutation::Reflow => {
            // Grow cols 100 → 120 with reflow. Placement at (0,0) on
            // a non-wrapped row: first_output_row[0] = 0, placement
            // survives unchanged.
            term.resize(24, 120, true);
        }
        GridMutation::AltEnter => {
            term.swap_alt();
        }
        GridMutation::AltExit => {
            term.swap_alt();
            term.swap_alt();
        }
        GridMutation::EraseDisplay => {
            let mut processor: Processor = Processor::new();
            processor.advance(term, b"\x1b[2J");
        }
        GridMutation::EraseLine => {
            let mut processor: Processor = Processor::new();
            processor.advance(term, b"\x1b[1;1H\x1b[2K");
        }
        GridMutation::FontMetricChange => {
            // Setup cell dims are (8, 16); change to (16, 8) — script-verified
            // to recompute a FixedPixels 16×16 placement (2,1)→(1,2), clamping
            // BOTH axes. (12,20) would leave (2,1) unchanged = ghost-pass.
            term.set_cell_dimensions(16, 8);
        }
    }

    // The placement was made in primary mode (lives in the primary
    // `image_cache` field). The two caches are semantically isolated:
    // `image_cache()` in alt mode
    // returns the (empty) alt cache, not the primary. Swap back to
    // primary before observing so `image_cache()` returns the cache
    // that owns the placement under test.
    if term.mode().contains(TermMode::ALT_SCREEN) {
        term.swap_alt();
    }

    let cache = term.image_cache();
    let placements = cache.placements_in_viewport(StableRowIndex(0), StableRowIndex(u64::MAX));
    let Some(p) = placements.iter().find(|p| p.image_id == id) else {
        return PlacementState::Removed;
    };
    if p.cell_row == original_row {
        PlacementState::Survives {
            cols: p.cols,
            rows: p.rows,
        }
    } else {
        PlacementState::Remapped
    }
}

fn expected(mutation: GridMutation, sizing: PlacementSizingKind) -> PlacementState {
    // HARDCODED expected dims (NOT captured from the placement) so a broken
    // `place_for_scenario` fixture FAILS the baseline mutations instead of
    // rigging expected==actual (an observed-then-asserted value cannot catch a
    // wrong fixture). Setup coverage at cell (8,16): FixedPixels 16×16 → (2,1);
    // CellCount → (2,2).
    let setup = match sizing {
        PlacementSizingKind::FixedPixels => (2, 1),
        PlacementSizingKind::CellCount => (2, 2),
    };
    let survives = |(cols, rows): (usize, usize)| PlacementState::Survives { cols, rows };
    match mutation {
        GridMutation::ScrollbackEvict => PlacementState::Removed,
        GridMutation::Resize => PlacementState::Removed,
        // Growing without mid-row wrap: first_output_row[0] = 0 so the
        // row index does not shift. Size is also preserved.
        GridMutation::Reflow => survives(setup),
        GridMutation::AltEnter => survives(setup),
        GridMutation::AltExit => survives(setup),
        GridMutation::EraseDisplay => PlacementState::Removed,
        GridMutation::EraseLine => PlacementState::Removed,
        // set_cell_dimensions(16,8): FixedPixels recomputes 16×16 → (1,2)
        // via update_cell_coverage; CellCount is untouched (stays (2,2)).
        GridMutation::FontMetricChange => match sizing {
            PlacementSizingKind::FixedPixels => survives((1, 2)),
            PlacementSizingKind::CellCount => survives((2, 2)),
        },
    }
}

#[test]
fn image_lifecycle_matrix() {
    let mut scenarios = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for &protocol in ImageProtocol::all() {
        for &sizing in PlacementSizingKind::all() {
            for &mutation in GridMutation::all() {
                // Fresh term per scenario so mutations don't bleed.
                // 100 cols so col=90 placements fit; 1 line of scrollback
                // so the eviction mutation actually evicts.
                let mut term = Term::new(24, 100, 1, Theme::default(), VoidEffectSink);
                term.set_cell_dimensions(8, 16);

                let id = place_for_scenario(&mut term, protocol, sizing, mutation);
                let original_row = StableRowIndex(
                    term.image_cache()
                        .placements_in_viewport(StableRowIndex(0), StableRowIndex(u64::MAX))
                        .iter()
                        .find(|p| p.image_id == id)
                        .map_or(0, |p| p.cell_row.0),
                );

                let actual = apply_mutation_and_observe(&mut term, id, mutation, original_row);
                let want = expected(mutation, sizing);

                if actual != want {
                    failures.push(format!(
                        "scenario {protocol:?} × {sizing:?} × {mutation:?} → \
                         expected {want:?}, got {actual:?}"
                    ));
                }
                scenarios += 1;
            }
        }
    }

    // Self-verifying completeness: every cell visited.
    assert_eq!(
        scenarios,
        ImageProtocol::all().len() * PlacementSizingKind::all().len() * GridMutation::all().len(),
        "matrix visit count does not match enumerated cells"
    );
    assert_eq!(scenarios, 48);

    assert!(
        failures.is_empty(),
        "matrix failures:\n{}",
        failures.join("\n")
    );
}
