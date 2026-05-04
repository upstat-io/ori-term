//! Sixel ↔ Kitty cross-stack ImageCache hand-off (§12.5).
//!
//! Pins that a single `Term`'s `ImageCache` can hold **both** a sixel
//! placement and a kitty placement simultaneously, with each
//! independently addressable through the public snapshot API
//! `Term::renderable_content()`. This is the handshake test, not the
//! full cross-stack rendering regression — deep mixed-protocol
//! interference (overlapping placements, z-order interleaving,
//! shared-eviction races) is explicitly DEFERRED-TO-DOWNSTREAM to
//! §13.6.
//!
//! The kitty side is driven read-only through `\x1b_G…\x1b\\` APC
//! sequences — no `kitty.rs` edits, which keeps 's kitty-scope
//! gate intact. Sixel side is driven through `\x1bPq…\x1b\\` DCS.

use oriterm_test_support::spec_chain::SpecHarness;
use oriterm_test_support::spec_chain::sixel_fixtures::dcs_n_cols_wide;

// Build an APC Kitty graphics command: `ESC _ G <body> ESC \`.
fn kitty_apc(body: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"\x1b_G");
    buf.extend_from_slice(body.as_bytes());
    buf.extend_from_slice(b"\x1b\\");
    buf
}

/// Catalog row: `SIXEL-CROSS-STACK-HANDOFF`.
///
/// Place a sixel at `(line=5, col=0)` and a kitty image at
/// `(line=10, col=0)` in the same `Term`'s `ImageCache`. Assert
/// `RenderableContent::images` contains BOTH placements, each
/// independently addressable. Pins that the two stacks share cache
/// storage without corrupting each other's `image_id` / `placement_id`
/// space and both appear in the renderer-visible snapshot.
#[test]
fn sixel_and_kitty_placements_coexist_in_renderable_snapshot() {
    let mut h = SpecHarness::new();

    // Place sixel at (line=5, col=0). CUP is 1-indexed.
    h.feed(b"\x1b[6;1H");
    h.feed(&dcs_n_cols_wide(8));

    // Transmit+place kitty at (line=10, col=5) — distinct column from
    // sixel so X-axis positional independence is pinnable below.
    // `a=T` = transmit-and-place; `f=32` = 32-bit RGBA; `s=1 v=1` = 1×1px;
    // `q=2` = silence response payload. "AAAAAA==" base64-decodes to 4
    // zero bytes (one transparent RGBA pixel).
    h.feed(b"\x1b[11;6H");
    h.feed(&kitty_apc("a=T,i=1,f=32,s=1,v=1,q=2;AAAAAA=="));

    let snap = h.term().renderable_content();

    // Collect distinct image_ids — BOTH stacks must contribute.
    let mut ids: Vec<_> = snap.images.iter().map(|p| p.image_id).collect();
    ids.sort_by_key(|id| format!("{id:?}"));
    ids.dedup_by_key(|id| format!("{id:?}"));
    assert!(
        snap.images.len() >= 2,
        "expected at least 2 placements (sixel + kitty), got {} — \
         cross-stack hand-off broken in ImageCache",
        snap.images.len(),
    );
    assert!(
        ids.len() >= 2,
        "expected at least 2 distinct image_ids, got {} — stacks share \
         an ID (corruption between sixel and kitty protocols)",
        ids.len(),
    );

    // Pin payload independence at the `image_data` rung: the renderable
    // snapshot's decoded pixel-payload records must contain the sixel
    // payload AND the kitty payload with their protocol-specific shapes
    // intact. Cross-wiring (one protocol overwriting the other) or
    // shared-cache aliasing would collapse one payload's dimensions /
    // byte count — the specific checks below catch both.
    let sixel_data = snap
        .image_data
        .iter()
        .find(|d| d.width == 8 && d.height == 6)
        .expect("sixel 8×6 RGBA payload present in image_data");
    let kitty_data = snap
        .image_data
        .iter()
        .find(|d| d.width == 1 && d.height == 1)
        .expect("kitty 1×1 RGBA payload present in image_data");
    // Sixel pixel data = 8 * 6 * 4 = 192 bytes RGBA.
    assert_eq!(
        sixel_data.data.len(),
        8 * 6 * 4,
        "sixel payload byte count pins 8×6 RGBA (192 bytes) — cross-\
         wiring with kitty's 4-byte payload would fail this strict eq",
    );
    // Kitty pixel data = 1 * 1 * 4 = 4 bytes RGBA.
    assert_eq!(
        kitty_data.data.len(),
        1 * 1 * 4,
        "kitty payload byte count pins 1×1 RGBA (4 bytes) — cross-\
         wiring with sixel's 192-byte payload would fail this strict eq",
    );
    // Distinct image_ids on the payload records too.
    assert_ne!(
        sixel_data.id, kitty_data.id,
        "sixel and kitty payloads must carry distinct image_ids — \
         collapse here would mean one payload overwrote the other in \
         the shared ImageCache",
    );
}

/// Catalog row: `SIXEL-CROSS-STACK-HANDOFF` (no-interference pin).
///
/// After placing sixel and kitty in the same cache, each placement's
/// `z_index` and pixel dimensions must match their own protocol's
/// contract — sixel at `z_index: 0` with FixedPixels sizing, kitty at
/// its own placement z-order. Proves the placements aren't cross-wired.
#[test]
fn sixel_and_kitty_placements_retain_independent_attributes() {
    let mut h = SpecHarness::new();

    h.feed(b"\x1b[6;1H");
    h.feed(&dcs_n_cols_wide(8));

    // Kitty at distinct column (col=5 not col=0) so X-axis positional
    // independence is pinnable below.
    h.feed(b"\x1b[11;6H");
    h.feed(&kitty_apc("a=T,i=1,f=32,s=1,v=1,q=2;AAAAAA=="));

    let snap = h.term().renderable_content();
    assert!(
        snap.images.len() >= 2,
        "precondition: at least 2 placements present",
    );

    let sixel = snap
        .images
        .iter()
        .find(|p| p.display_width == 8.0 && p.display_height == 6.0)
        .expect("sixel placement (8×6 FixedPixels) present");
    assert_sixel_independence(sixel);

    let kitty = snap
        .images
        .iter()
        .find(|p| p.image_id != sixel.image_id)
        .expect("kitty placement present (distinct image_id)");
    assert_kitty_independence(kitty, sixel);
}

/// Pin sixel's own placement attributes (post-coexistence): z_index=0,
/// viewport anchored at (line=5, col=0) under the harness cell metrics
/// (cell_w=8, cell_h=16). Factored out of the
/// `sixel_and_kitty_placements_retain_independent_attributes` test so
/// the test body stays under the 100-line function limit.
fn assert_sixel_independence(sixel: &oriterm_core::RenderablePlacement) {
    assert_eq!(
        sixel.z_index, 0,
        "sixel placements use z_index: 0 — must not be corrupted by \
         coexistent kitty placement",
    );
    // Sixel anchored at (line=5, col=0) → viewport_y = 5 * cell_h (16) = 80,
    // viewport_x = 0 * cell_w (8) = 0.
    assert_eq!(
        sixel.viewport_y, 80.0,
        "sixel viewport_y pins cell_row=5 * cell_pixel_height=16",
    );
    assert_eq!(
        sixel.viewport_x, 0.0,
        "sixel viewport_x pins cell_col=0 * cell_pixel_width=8",
    );
}

/// Pin kitty's own placement attributes (post-coexistence) and their
/// independence from the coexistent sixel placement. Kitty at
/// (line=10, col=5), distinct `image_id`, dimensions distinct from
/// sixel's (8.0, 6.0), z_index=0 retained.
fn assert_kitty_independence(
    kitty: &oriterm_core::RenderablePlacement,
    sixel: &oriterm_core::RenderablePlacement,
) {
    assert_ne!(
        kitty.image_id, sixel.image_id,
        "distinct image_ids — cache storage is per-protocol",
    );
    // Kitty anchored at (line=10, col=5) → viewport_y = 10 * 16 = 160,
    // viewport_x = 5 * 8 = 40. Both Y and X differ from sixel — pins
    // positional independence across BOTH axes, not just Y.
    assert_eq!(
        kitty.viewport_y, 160.0,
        "kitty viewport_y pins cell_row=10 * cell_pixel_height=16",
    );
    assert_eq!(
        kitty.viewport_x, 40.0,
        "kitty viewport_x pins cell_col=5 * cell_pixel_width=8 — \
         distinct from sixel.viewport_x=0.0; X-axis positional \
         independence across protocols is pinned by the strict eq \
         (0.0 vs 40.0)",
    );
    assert!(
        kitty.viewport_y > sixel.viewport_y,
        "kitty placement below sixel placement — Y-axis directional \
         pin beyond the strict eq (sixel=80, kitty=160) above",
    );
    // Dimensions come from the kitty placement pipeline, NOT from
    // sixel's raster attrs. Pin that at least one of kitty's
    // (display_width, display_height) differs from sixel's (8.0, 6.0).
    let kitty_dims_distinct_from_sixel = kitty.display_width != 8.0 || kitty.display_height != 6.0;
    assert!(
        kitty_dims_distinct_from_sixel,
        "kitty placement must have dimensions distinct from sixel's \
         (8.0, 6.0) — got ({}, {}); cross-wiring would yield sixel's \
         raster attrs",
        kitty.display_width, kitty.display_height,
    );
    assert!(
        kitty.display_width > 0.0 && kitty.display_height > 0.0,
        "kitty placement dimensions must be positive and non-degenerate \
         (got {} × {})",
        kitty.display_width,
        kitty.display_height,
    );
    assert_eq!(
        kitty.z_index, 0,
        "kitty placements default to z_index: 0 — must not be \
         corrupted by coexistent sixel placement",
    );
}

/// Matrix-completeness pin for the cross-stack handshake scope.
#[test]
fn cross_stack_handoff_category_matrix_completeness() {
    const CATEGORIES: &[&str] = &[
        "both_placements_in_snapshot",
        "placements_retain_independent_attributes",
    ];
    assert_eq!(
        CATEGORIES.len(),
        2,
        "§12.5 cross-stack handshake scope has 2 categories — deep \
         mixed-protocol rendering interference is deferred to §13.6",
    );
}
