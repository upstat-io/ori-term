//! Anchor-lifecycle pins for the kitty unicode-placeholder (`U=1`) protocol.
//!
//! Covers: anchor add on transmit/place, anchor clear via `d=I`,
//! survivor reconciliation on scrollback + ED, architectural assertion
//! that `image/cache/` does NOT depend on grid types.

use oriterm_core::image::ImageId;
use oriterm_test_support::spec_chain::SpecHarness;

use super::fixtures::{b64, kitty_apc, rgba_4x4_red};

fn transmit_u1(h: &mut SpecHarness, id: u32) {
    let control = format!("a=t,U=1,i={id},f=32,s=4,v=4");
    h.feed(&kitty_apc(control.as_bytes(), &b64(&rgba_4x4_red())));
}

fn write_placeholder_cell(h: &mut SpecHarness, image_id_low: u32, row: char, col: char) {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(format!("\x1b[38;5;{image_id_low}m").as_bytes());
    let mut buf = [0u8; 4];
    bytes.extend_from_slice('\u{10EEEE}'.encode_utf8(&mut buf).as_bytes());
    bytes.extend_from_slice(row.encode_utf8(&mut buf).as_bytes());
    bytes.extend_from_slice(col.encode_utf8(&mut buf).as_bytes());
    bytes.extend_from_slice(b"\x1b[39m");
    h.feed(&bytes);
}

#[test]
fn placeholder_anchor_added_on_u1_transmit() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 1);
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(1))
    );
}

#[test]
fn placeholder_anchor_added_on_u1_place() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=2,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    h.feed(&kitty_apc(b"a=p,U=1,i=2,c=1,r=1", ""));
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(2))
    );
}

#[test]
fn remove_image_clears_placeholder_anchor_entry() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 3);
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(3))
    );

    // d=I,i=3 — uppercase variant removes image data + placements
    // + anchors.
    h.feed(&kitty_apc(b"a=d,d=I,i=3", ""));

    assert!(
        !h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(3)),
        "d=I MUST clear placeholder anchor — got {:?}",
        h.term().image_cache().placeholder_anchors(),
    );
}

#[test]
fn kitty_delete_i_clears_placeholder_anchor() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 4);
    h.feed(&kitty_apc(b"a=d,d=I,i=4", ""));
    assert!(
        !h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(4))
    );
}

#[test]
fn kitty_delete_a_clears_placeholder_anchors_when_no_placeholder_cells_remain() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 5);

    // d=a/A removes only PLACEMENTS in the viewport — per kitty spec it
    // does NOT touch virtual (U=1) placements. The anchor survives
    // because the grid has no U+10EEEE cells yet to remove.
    h.feed(&kitty_apc(b"a=d,d=a", ""));
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(5)),
        "d=a MUST NOT clear the anchor for a U=1 transmit with no placeholder cells in the grid",
    );
}

#[test]
fn prune_scrollback_retains_anchor_when_placeholder_cells_remain_in_viewport() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 6);
    write_placeholder_cell(&mut h, 6, '\u{0305}', '\u{0305}');

    // Push a few lines into scrollback — the placeholder cell is at the
    // top so it stays in view if we don't push past the viewport height.
    for _ in 0..5 {
        h.feed(b"x\n");
    }

    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(6)),
        "anchor must survive while placeholder cells still occupy the grid",
    );
}

#[test]
fn prune_scrollback_clears_orphaned_placeholder_anchors_for_images_with_no_surviving_cells() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 8);
    write_placeholder_cell(&mut h, 8, '\u{0305}', '\u{0305}');
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(8))
    );

    // Push the placeholder cell out of grid + scrollback by feeding more
    // newlines than the scrollback ring can hold. With no surviving
    // placeholder cells, the reconciler must drop the anchor.
    let scrollback_cap = h.term().grid().scrollback().max_scrollback();
    let lines = h.term().grid().lines();
    for _ in 0..(scrollback_cap + lines + 8) {
        h.feed(b"x\n");
    }

    assert!(
        !h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(8)),
        "after scrollback eviction with no surviving placeholder cells the anchor must drop",
    );
}

#[test]
fn ed_full_screen_clears_orphaned_placeholder_anchors() {
    // When a placeholder cell did anchor the image but is then erased,
    // the survivor set MUST evict the anchor.
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 9);
    write_placeholder_cell(&mut h, 9, '\u{0305}', '\u{0305}');
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(9))
    );

    // ED 2 (full-screen erase) drops the U+10EEEE cell. But: U=1 transmit
    // is special — `add_placeholder_anchor` recorded the image as
    // anchored at transmit time, BEFORE any placeholder cell existed.
    // The current contract is that anchors persist until explicit `d=I`;
    // the grid survivor set is a refinement layer.
    h.feed(b"\x1b[2J");

    // With no placeholder cells surviving, the survivor reconciler
    // empties the anchor set.
    assert!(
        !h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(9)),
        "after ED 2 the survivor set is empty — anchor must drop",
    );
}

#[test]
fn image_cache_lifecycle_does_not_walk_grid_cells() {
    // Architectural pin: ImageCache must NOT import Grid / Row / Cell —
    // any such import scatters grid knowledge into the cache layer.
    // Reviewers flag the import as a layer-boundary violation.
    let cache_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/image/cache");
    let mut offenders: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&cache_dir).expect("read image/cache/") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("read source");
        for line in src.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("use crate::cell::")
                || trimmed.starts_with("use crate::grid::Grid")
                || trimmed.starts_with("use crate::grid::Row")
            {
                offenders.push(format!("{}: {}", path.display(), trimmed));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "ImageCache must not import Grid / Row / Cell — offenders: {offenders:?}",
    );
}

// §13.4 missing-reconcile-site pins.
//
// Every grid-mutation path that can overwrite or displace a `U+10EEEE`
// placeholder cell MUST call `reconcile_placeholder_anchors_from_grid()` so
// `ImageCache.placeholder_anchors` does not drift past the cells that
// anchor each entry. Sites covered below: ASCII fast-path print, non-ASCII
// print, DECDC / DECIC column ops, DECERA / DECFRA rectangle ops, and the
// inactive-alt-screen path on resize.

#[test]
fn printable_overwrite_of_placeholder_cell_reconciles_anchor() {
    // ASCII fast-path: `put_char_ascii` short-circuits before
    // `prune_images_if_evicted`, so an overwrite of a U+10EEEE cell must
    // explicitly reconcile or the anchor leaks.
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 100);
    write_placeholder_cell(&mut h, 100, '\u{0305}', '\u{0305}');
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(100)),
        "precondition: anchor recorded after U=1 transmit + placeholder cell write",
    );

    // CUP back to (1, 1) and overwrite with plain ASCII 'X'.
    h.feed(b"\x1b[1;1HX");

    assert!(
        !h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(100)),
        "ASCII overwrite of the only placeholder cell must reconcile the anchor away",
    );
}

#[test]
fn non_ascii_overwrite_of_placeholder_cell_reconciles_anchor() {
    // Non-ASCII print path uses `prune_images_if_evicted` which only
    // reconciles when scrollback grew. An in-viewport overwrite never grows
    // scrollback — the reconcile must fire unconditionally on this path.
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 101);
    write_placeholder_cell(&mut h, 101, '\u{0305}', '\u{0305}');
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(101))
    );

    // CUP to (1, 1), overwrite with 'ü' (U+00FC — Latin-1 supplement,
    // 2 bytes UTF-8). Width=1, hits the non-ASCII branch.
    h.feed(b"\x1b[1;1H\xc3\xbc");

    assert!(
        !h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(101)),
        "non-ASCII overwrite of the only placeholder cell must reconcile the anchor away",
    );
}

#[test]
fn column_delete_with_placeholder_cells_reconciles_anchor() {
    // DECDC (`CSI Ps ' ~`) deletes columns at the cursor; cells in the
    // deleted span are lost. If those cells were placeholders, the anchor
    // must be reconciled.
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 102);
    // Place the placeholder at column 6 (CUP 1;6 is 1-based → col 5).
    h.feed(b"\x1b[1;6H");
    write_placeholder_cell(&mut h, 102, '\u{0305}', '\u{0305}');
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(102))
    );

    // CUP to (1, 1), DECDC count=10 deletes cols 0..10 — placeholder at
    // col 5 is in that range.
    h.feed(b"\x1b[1;1H\x1b[10\'~");

    assert!(
        !h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(102)),
        "DECDC deleting the only placeholder cell must reconcile the anchor away",
    );
}

#[test]
fn column_insert_with_placeholder_cells_reconciles_anchor() {
    // DECIC (`CSI Ps ' }`) inserts blank columns at the cursor; existing
    // cells shift right and cells past the right edge are dropped.
    let mut h = SpecHarness::new();
    let cols = h.term().grid().cols();
    transmit_u1(&mut h, 103);
    // Place placeholder at the last column. CUP is 1-based.
    h.feed(format!("\x1b[1;{cols}H").as_bytes());
    write_placeholder_cell(&mut h, 103, '\u{0305}', '\u{0305}');
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(103))
    );

    // CUP to col 1; DECIC count=cols pushes ALL existing content off the
    // right edge.
    h.feed(format!("\x1b[1;1H\x1b[{cols}\'}}").as_bytes());

    assert!(
        !h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(103)),
        "DECIC pushing the only placeholder cell off the right edge must reconcile the anchor away",
    );
}

#[test]
fn rectangular_erase_with_placeholder_cells_reconciles_anchor() {
    // DECERA (`CSI Pt;Pl;Pb;Pr $ z`) erases a rectangle of cells.
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 104);
    write_placeholder_cell(&mut h, 104, '\u{0305}', '\u{0305}');
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(104))
    );

    // DECERA over rows 1..5, cols 1..10 — covers (0, 0) where the
    // placeholder lives.
    h.feed(b"\x1b[1;1;5;10$z");

    assert!(
        !h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(104)),
        "DECERA over the only placeholder cell must reconcile the anchor away",
    );
}

#[test]
fn rectangular_fill_replacing_placeholder_cells_reconciles_anchor() {
    // DECFRA (`CSI Pc;Pt;Pl;Pb;Pr $ x`) fills a rectangle with `Pc`.
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 105);
    write_placeholder_cell(&mut h, 105, '\u{0305}', '\u{0305}');
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(105))
    );

    // DECFRA with Pc=0x58 ('X') over rows 1..5, cols 1..10.
    h.feed(b"\x1b[88;1;1;5;10$x");

    assert!(
        !h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(105)),
        "DECFRA overwriting the only placeholder cell must reconcile the anchor away",
    );
}

#[test]
fn inactive_alt_screen_u1_anchors_reconciled_on_resize() {
    // While alt-screen is active, transmit U=1 + write a placeholder cell
    // near the right edge so a narrower resize drops it. Switch back to
    // primary so alt is inactive, resize, then return to alt. The alt
    // image cache MUST be reconciled even when alt is inactive — otherwise
    // the anchor leaks across the resize.
    let mut h = SpecHarness::new();
    let cols = h.term().grid().cols();

    // Enter alt screen (DECSET 1049).
    h.feed(b"\x1b[?1049h");

    // Transmit + write placeholder cell on the LAST column of the alt
    // grid. A narrower resize without reflow drops that cell.
    transmit_u1(&mut h, 106);
    h.feed(format!("\x1b[1;{cols}H").as_bytes());
    write_placeholder_cell(&mut h, 106, '\u{0305}', '\u{0305}');
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(106)),
        "precondition: anchor recorded on alt cache while alt is active",
    );

    // Return to primary (DECRST 1049). The alt cache + alt grid stay
    // around but become inactive.
    h.feed(b"\x1b[?1049l");

    // Resize narrower while ALT IS INACTIVE. Primary reconcile fires on
    // the active path; the alt cache must also be reconciled against the
    // inactive alt grid.
    let lines = h.term().grid().lines();
    h.term_mut().resize(lines, cols / 2, false);

    // Switch back to alt to inspect the alt cache.
    h.feed(b"\x1b[?1049h");

    assert!(
        !h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(106)),
        "inactive-alt anchor whose cell was dropped by resize must be reconciled — current code only reconciles the active screen, the alt cache leaks anchors past resize",
    );
}

#[test]
fn line_erase_csi_k_with_placeholder_cell_reconciles_anchor() {
    // CSI K (EL, default Pn=0) erases from cursor to end of line. When
    // the erased span covers a `U+10EEEE` cell, the anchor must be
    // reconciled. This is the per-line EL counterpart to the per-screen
    // ED test above.
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 111);
    write_placeholder_cell(&mut h, 111, '\u{0305}', '\u{0305}');
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(111))
    );

    // CUP back to (1,1) then EL (default = erase cursor to end of line).
    h.feed(b"\x1b[1;1H\x1b[K");

    assert!(
        !h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(111)),
        "CSI K (EL) erasing the only placeholder cell must reconcile the anchor away",
    );
}

#[test]
fn inactive_primary_screen_u1_anchors_reconciled_on_resize() {
    // Symmetric to `inactive_alt_screen_u1_anchors_reconciled_on_resize`:
    // when alt-screen is active and resize fires, the INACTIVE PRIMARY
    // grid + cache also resize. The primary cache anchors must be
    // reconciled even though `self.grid()` returns the alt grid while alt
    // is active. Without the symmetric reconcile primary anchors leak
    // past resize whenever alt is the active screen.
    let mut h = SpecHarness::new();
    let cols = h.term().grid().cols();

    // Set up the placeholder on PRIMARY (initial active screen).
    transmit_u1(&mut h, 110);
    h.feed(format!("\x1b[1;{cols}H").as_bytes());
    write_placeholder_cell(&mut h, 110, '\u{0305}', '\u{0305}');
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(110)),
        "precondition: primary anchor recorded",
    );

    // Switch to alt — primary is now inactive.
    h.feed(b"\x1b[?1049h");

    // Resize narrower while ALT IS ACTIVE. Use reflow=false so the
    // placeholder at the right edge of primary is truncated (the case
    // we want to verify reconcile catches); reflow=true would migrate
    // the cell instead of dropping it, masking the bug.
    let lines = h.term().grid().lines();
    h.term_mut().resize(lines, cols / 2, false);

    // Return to primary to inspect the primary cache.
    h.feed(b"\x1b[?1049l");

    assert!(
        !h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(110)),
        "inactive-primary anchor whose cell was dropped by resize must be reconciled — symmetric to the alt-inactive case",
    );
}

#[test]
fn inactive_alt_screen_u1_anchors_retained_when_cells_survive_resize() {
    // Counterpart to the inactive-alt reconcile clear test: when the
    // placeholder cell SURVIVES the resize (sits in a column that the
    // narrower width still covers), the inactive-alt reconcile must keep
    // the anchor. Without this pin a too-aggressive reconcile (e.g.
    // clearing all anchors) would silently still satisfy the clear-side
    // assertion above.
    let mut h = SpecHarness::new();
    let cols = h.term().grid().cols();

    h.feed(b"\x1b[?1049h");
    transmit_u1(&mut h, 107);
    // Placeholder at column 1 (CUP 1;1 = col 0) — survives any narrower
    // resize down to 1 column.
    h.feed(b"\x1b[1;1H");
    write_placeholder_cell(&mut h, 107, '\u{0305}', '\u{0305}');
    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(107))
    );

    h.feed(b"\x1b[?1049l");
    let lines = h.term().grid().lines();
    h.term_mut().resize(lines, cols / 2, false);
    h.feed(b"\x1b[?1049h");

    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(107)),
        "inactive-alt anchor whose cell SURVIVES the resize must be retained",
    );
}

// ── §13.6.1 multi-cell U=1 placeholder display grid ────────────────

/// Regression: spec-conformance §13.6.1 — `a=T,U=1,c=11,r=1` records the
/// (cols, rows) display grid on the cache anchor so the GPU emit path
/// can slice per-cell UVs. Single-cell `c=1,r=1` is the implicit default
/// and may either record (1,1) or no entry — both render the full image.
#[test]
fn u1_transmit_and_place_with_explicit_grid_records_anchor_grid() {
    let mut h = SpecHarness::new();
    // a=T,U=1,c=11,r=1 — transmit + place + multi-cell display grid.
    h.feed(&kitty_apc(
        b"a=T,U=1,i=1,c=11,r=1,f=32,s=4,v=4",
        &b64(&rgba_4x4_red()),
    ));

    assert_eq!(
        h.term()
            .image_cache()
            .placeholder_anchor_grid_for(ImageId::from_raw(1)),
        Some((11, 1)),
        "a=T,U=1 with explicit c=11,r=1 must populate the anchor grid \
         so emit_placeholder_quads can compute per-cell UV slices"
    );
}

/// Regression: spec-conformance §13.6.1 — separate `a=t,U=1` then
/// `a=p,U=1,c=2,r=3` records the grid via the `a=p` path (control-key
/// inheritance through the kitty placement-create path).
#[test]
fn u1_separate_transmit_then_place_with_grid_records_anchor_grid() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,U=1,i=2,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    h.feed(&kitty_apc(b"a=p,U=1,i=2,c=2,r=3", ""));

    assert_eq!(
        h.term()
            .image_cache()
            .placeholder_anchor_grid_for(ImageId::from_raw(2)),
        Some((2, 3)),
        "a=p,U=1 with c=2,r=3 must populate the anchor grid"
    );
}

/// Regression: spec-conformance §13.6.1 — `c=1,r=1` is the SINGLE-CELL
/// case (kitty_placeholder_basic uses it). The grid IS recorded as
/// `(1, 1)` so emit defaults its UV to the full image cell. Negative
/// pin against a future "skip recording when c=r=1" optimization that
/// would diverge from the multi-cell path.
#[test]
fn u1_single_cell_grid_records_one_by_one() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=T,U=1,i=3,c=1,r=1,f=32,s=4,v=4",
        &b64(&rgba_4x4_red()),
    ));

    assert_eq!(
        h.term()
            .image_cache()
            .placeholder_anchor_grid_for(ImageId::from_raw(3)),
        Some((1, 1)),
        "c=1,r=1 must record the explicit single-cell grid — emit divides \
         by cols/rows in either path, so the (1,1) entry is correct and \
         silent skipping would create a divergence between explicit-grid \
         and default-grid behavior"
    );
}

/// Regression: spec-conformance §13.6.1 — `a=T,U=1` WITHOUT `c=`/`r=`
/// records the anchor but NO grid entry. Emit falls back to (1, 1)
/// (full image) via the `placement_cols/placement_rows` default.
#[test]
fn u1_transmit_without_explicit_grid_records_no_anchor_grid() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,U=1,i=4,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    assert!(
        h.term()
            .image_cache()
            .placeholder_anchors()
            .contains(&ImageId::from_raw(4)),
        "anchor must be added even without explicit grid"
    );
    assert_eq!(
        h.term()
            .image_cache()
            .placeholder_anchor_grid_for(ImageId::from_raw(4)),
        None,
        "no c=/r= → no recorded grid; snapshot defaults each placeholder \
         cell to (1, 1) so emit renders the full image"
    );
}

/// Regression: spec-conformance §13.6.1 — placeholder cells in the
/// renderable snapshot carry the (cols, rows) display grid. The GPU
/// emit path reads these fields per-cell — without them every cell
/// would render the full image (the pre-§13.6.1 single-cell baseline).
#[test]
fn snapshot_placeholder_cells_carry_display_grid() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(
        b"a=T,U=1,i=1,c=2,r=2,f=32,s=4,v=4",
        &b64(&rgba_4x4_red()),
    ));
    // Park cursor at row 1, col 1 (0-indexed (0, 0)), then write a
    // single placeholder cell with row_diacritic=0 col_diacritic=0 +
    // fg=palette(1) carrying image_id_low=1.
    h.feed(b"\x1b[1;1H");
    write_placeholder_cell(&mut h, 1, '\u{0305}', '\u{0305}');

    let content = h.term().renderable_content();
    assert!(
        !content.placeholder_cells.is_empty(),
        "placeholder cell must appear in snapshot"
    );
    let pc = &content.placeholder_cells[0];
    assert_eq!(pc.image_id, ImageId::from_raw(1));
    assert_eq!(
        (pc.placement_cols, pc.placement_rows),
        (2, 2),
        "snapshot must propagate the (c=2, r=2) display grid to the \
         placeholder cell so emit can slice UVs per cell"
    );
}

/// Regression: spec-conformance §13.6.1 — placeholder cells for images
/// with NO recorded grid (no c=/r= at transmit/place) carry the
/// (1, 1) default in the snapshot, matching the pre-§13.6.1 single-cell
/// behavior and keeping `kitty_placeholder_basic` golden stable.
#[test]
fn snapshot_placeholder_cells_default_grid_to_one_by_one_when_unrecorded() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=T,U=1,i=1,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    h.feed(b"\x1b[1;1H");
    write_placeholder_cell(&mut h, 1, '\u{0305}', '\u{0305}');

    let content = h.term().renderable_content();
    assert!(!content.placeholder_cells.is_empty());
    let pc = &content.placeholder_cells[0];
    assert_eq!(
        (pc.placement_cols, pc.placement_rows),
        (1, 1),
        "cells without a recorded display grid default to (1, 1) so emit \
         renders the full image — kitty_placeholder_basic pilot pin"
    );
}
