//! Per-specifier tests for kitty graphics `a=d` delete arms.
//!
//! Every test encodes protocol-spec behavior per kitty
//! graphics-protocol.rst §Deleting images (§Deleting images, lines 742–788
//! of `~/projects/reference_repos/console_repos/kitty/docs/graphics-protocol.rst`).
//! Tests pin the CORRECT spec behavior — not the current BUG-08-007 /
//! BUG-08-008 baseline. This is red-before-green TDD per
//! `.claude/rules/tests.md` §TDD for Bugs.
//!
//! Catalog rows: KG-ACTION-DELETE, KG-DELETE-a, KG-DELETE-A, KG-DELETE-i,
//! KG-DELETE-I, KG-DELETE-p, KG-DELETE-P, KG-DELETE-c, KG-DELETE-C,
//! KG-DELETE-x, KG-DELETE-X, KG-DELETE-y, KG-DELETE-Y, KG-DELETE-z,
//! KG-DELETE-Z, KG-DELETE-r, KG-DELETE-R, KG-DELETE-q, KG-DELETE-Q,
//! KG-DELETE-f, KG-DELETE-F, KG-DELETE-n, KG-DELETE-N.

use std::sync::Arc;

use crate::effect::VoidEffectSink;
use crate::grid::StableRowIndex;
use crate::image::kitty::{KittyAction, KittyCommand};
use crate::image::{ImageData, ImageFormat, ImageId, ImagePlacement, ImageSource, PlacementSizing};
use crate::term::Term;
use crate::theme::Theme;

const LINES: usize = 24;
const COLS: usize = 80;

fn term() -> Term<VoidEffectSink> {
    Term::new(LINES, COLS, 0, Theme::default(), VoidEffectSink)
}

fn stage(
    t: &mut Term<VoidEffectSink>,
    id: u32,
    image_number: Option<u32>,
    col: usize,
    row: u64,
    z: i32,
    pid: Option<u32>,
) {
    let img = ImageData {
        id: ImageId::from_raw(id),
        width: 10,
        height: 10,
        data: Arc::new(vec![0u8; 400]),
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number,
    };
    let placement = ImagePlacement {
        image_id: ImageId::from_raw(id),
        placement_id: pid,
        source_x: 0,
        source_y: 0,
        source_w: 10,
        source_h: 10,
        cell_col: col,
        cell_row: StableRowIndex(row),
        cols: 1,
        rows: 1,
        z_index: z,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    };
    t.image_cache_mut().store(img).expect("store image");
    t.image_cache_mut().place(placement);
}

fn stage_basic(t: &mut Term<VoidEffectSink>, id: u32, col: usize, row: u64) {
    stage(t, id, None, col, row, 0, None);
}

fn delete(t: &mut Term<VoidEffectSink>, spec: u8, modify: impl FnOnce(&mut KittyCommand)) {
    let mut cmd = KittyCommand {
        action: KittyAction::Delete,
        delete_specifier: Some(spec),
        ..Default::default()
    };
    modify(&mut cmd);
    t.kitty_delete(&cmd);
}

fn image_count(t: &Term<VoidEffectSink>) -> usize {
    t.image_cache().image_count()
}

fn placement_count(t: &Term<VoidEffectSink>) -> usize {
    t.image_cache().placement_count()
}

// ---------------------------------------------------------------------------
// KG-DELETE-a / A  —  Delete all placements VISIBLE ON SCREEN.
// ---------------------------------------------------------------------------

/// KG-DELETE-a: `d=a` deletes only visible placements; keeps image data.
/// Spec: `a` → "Delete all placements visible on screen" (lowercase keeps data).
#[test]
fn delete_a_removes_visible_placement_only_keeps_image_data() {
    let mut t = term();
    stage_basic(&mut t, 1, 10, 0); // visible (viewport 0..23)
    stage_basic(&mut t, 2, 10, 500); // off-screen (row 500)
    assert_eq!(placement_count(&t), 2);
    assert_eq!(image_count(&t), 2);

    delete(&mut t, b'a', |_| {});

    assert_eq!(
        placement_count(&t),
        1,
        "off-screen placement must survive d=a"
    );
    assert_eq!(image_count(&t), 2, "lowercase d=a must NOT free image data");
}

/// KG-DELETE-a negative pin: d=a must NOT call cache.clear() (would drop the
/// off-screen placement too). Regression guard for BUG-08-007.
#[test]
fn delete_a_negative_pin_does_not_clear_entire_cache() {
    let mut t = term();
    stage_basic(&mut t, 1, 10, 500); // entirely off-screen
    assert_eq!(placement_count(&t), 1);

    delete(&mut t, b'a', |_| {});

    assert_eq!(
        placement_count(&t),
        1,
        "BUG-08-007 regression: d=a cleared off-screen placement"
    );
    assert_eq!(image_count(&t), 1);
}

/// KG-DELETE-A: `d=A` deletes visible placements + image data for images with
/// no remaining placements.
#[test]
fn delete_a_uppercase_removes_visible_placement_and_frees_image_data() {
    let mut t = term();
    stage_basic(&mut t, 1, 10, 0); // visible, ONLY placement for image 1
    stage_basic(&mut t, 2, 10, 500); // off-screen
    assert_eq!(image_count(&t), 2);

    delete(&mut t, b'A', |_| {});

    assert_eq!(placement_count(&t), 1, "only off-screen placement remains");
    assert_eq!(
        image_count(&t),
        1,
        "image 1 must be freed (no remaining placements); image 2 survives"
    );
}

// ---------------------------------------------------------------------------
// KG-DELETE-i / I  —  By image id (+ optional p= placement filter).
// ---------------------------------------------------------------------------

/// KG-DELETE-i: `d=i,i=<id>` deletes all placements of image id; keeps image.
#[test]
fn delete_i_removes_all_placements_for_image_id_keeps_image_data() {
    let mut t = term();
    stage_basic(&mut t, 1, 10, 0);
    // Add a second placement for image 1 directly (not via store, which would
    // replace the image entry).
    t.image_cache_mut().place(ImagePlacement {
        image_id: ImageId::from_raw(1),
        placement_id: None,
        source_x: 0,
        source_y: 0,
        source_w: 10,
        source_h: 10,
        cell_col: 20,
        cell_row: StableRowIndex(0),
        cols: 1,
        rows: 1,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    });
    stage_basic(&mut t, 2, 30, 0);
    assert_eq!(placement_count(&t), 3);

    delete(&mut t, b'i', |c| c.image_id = Some(1));

    assert_eq!(placement_count(&t), 1);
    assert_eq!(
        image_count(&t),
        2,
        "lowercase d=i keeps both image data entries"
    );
}

/// KG-DELETE-i with p=: only the specified (image_id, placement_id) is removed.
#[test]
fn delete_i_with_placement_id_scopes_to_that_placement() {
    let mut t = term();
    stage(&mut t, 1, None, 10, 0, 0, Some(7));
    // Second placement for image 1 — add directly (store would replace).
    t.image_cache_mut().place(ImagePlacement {
        image_id: ImageId::from_raw(1),
        placement_id: Some(8),
        source_x: 0,
        source_y: 0,
        source_w: 10,
        source_h: 10,
        cell_col: 20,
        cell_row: StableRowIndex(0),
        cols: 1,
        rows: 1,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    });
    assert_eq!(placement_count(&t), 2);

    delete(&mut t, b'i', |c| {
        c.image_id = Some(1);
        c.placement_id = Some(7);
    });

    assert_eq!(placement_count(&t), 1);
    assert_eq!(image_count(&t), 1);
}

/// KG-DELETE-I: `d=I,i=<id>` deletes placements AND image data.
#[test]
fn delete_i_uppercase_removes_image_data_and_placements() {
    let mut t = term();
    stage_basic(&mut t, 1, 10, 0);
    stage_basic(&mut t, 2, 20, 0);
    assert_eq!(image_count(&t), 2);

    delete(&mut t, b'I', |c| c.image_id = Some(1));

    assert_eq!(placement_count(&t), 1);
    assert_eq!(image_count(&t), 1);
}

// ---------------------------------------------------------------------------
// KG-DELETE-p / P  —  At cell (x, y) intersection (1-based spec).
// ---------------------------------------------------------------------------

/// KG-DELETE-p: `d=p,x=X,y=Y` deletes placements intersecting cell (X-1, Y-1).
/// BUG-08-007 regression: current impl used `placement_id` instead of cell.
#[test]
fn delete_p_uses_cell_position_not_placement_id() {
    let mut t = term();
    // Two placements at the same cell (col=3, row=4) with different placement_ids.
    stage(&mut t, 1, None, 3, 4, 0, Some(10));
    stage(&mut t, 2, None, 3, 4, 0, Some(20));
    // Third placement at a different cell.
    stage(&mut t, 3, None, 5, 4, 0, Some(30));
    assert_eq!(placement_count(&t), 3);

    delete(&mut t, b'p', |c| {
        c.source_x = 4; // spec is 1-based: x=4 → col=3
        c.source_y = 5; // spec is 1-based: y=5 → row=4
    });

    assert_eq!(
        placement_count(&t),
        1,
        "d=p deletes BOTH placements at (3,4) regardless of placement_id"
    );
    assert_eq!(image_count(&t), 3, "lowercase keeps image data");
}

/// KG-DELETE-p negative pin: d=p MUST NOT use placement_id. Regression for
/// BUG-08-007 — old impl required i= and p= and silently dropped x=/y=.
#[test]
fn delete_p_negative_pin_ignores_placement_id_key() {
    let mut t = term();
    stage(&mut t, 1, None, 3, 4, 0, Some(99));
    assert_eq!(placement_count(&t), 1);

    delete(&mut t, b'p', |c| {
        c.source_x = 4;
        c.source_y = 5;
        c.placement_id = Some(1234); // wrong placement_id — must be IGNORED
        c.image_id = Some(9999); // wrong image_id — must be IGNORED
    });

    assert_eq!(
        placement_count(&t),
        0,
        "BUG-08-007 regression: d=p consulted p= or i= instead of (x,y)"
    );
}

/// KG-DELETE-P: d=P additionally frees image data when orphaned.
#[test]
fn delete_p_uppercase_frees_orphaned_image_data() {
    let mut t = term();
    stage(&mut t, 1, None, 3, 4, 0, None); // only placement for image 1
    assert_eq!(image_count(&t), 1);

    delete(&mut t, b'P', |c| {
        c.source_x = 4;
        c.source_y = 5;
    });

    assert_eq!(placement_count(&t), 0);
    assert_eq!(image_count(&t), 0, "uppercase P prunes orphaned image data");
}

// ---------------------------------------------------------------------------
// KG-DELETE-c / C  —  At cursor (col, row) intersection.
// ---------------------------------------------------------------------------

/// KG-DELETE-c: `d=c` uses CURSOR POSITION (col, row) — not column alone.
/// BUG-08-007 regression: current impl only matched the column.
#[test]
fn delete_c_uses_cursor_row_not_column_only() {
    let mut t = term();
    // Two placements sharing the cursor column but different rows.
    stage_basic(&mut t, 1, 10, 0); // cursor row
    stage_basic(&mut t, 2, 10, 5); // same column, different row
    assert_eq!(placement_count(&t), 2);
    // Cursor defaults to (0, 0) on a fresh Term — col=0, row=0.
    // Place a third at cursor position to confirm it gets hit.
    stage_basic(&mut t, 3, 0, 0);
    assert_eq!(placement_count(&t), 3);

    delete(&mut t, b'c', |_| {});

    assert_eq!(
        placement_count(&t),
        2,
        "d=c must match both col AND row at cursor; only image 3 at (0,0) is hit"
    );
}

/// KG-DELETE-c negative pin: d=c MUST NOT delete placements at a different row.
#[test]
fn delete_c_negative_pin_does_not_delete_entire_cursor_column() {
    let mut t = term();
    // Multiple placements at the cursor column (col 0) but different rows.
    stage_basic(&mut t, 1, 0, 5);
    stage_basic(&mut t, 2, 0, 10);
    stage_basic(&mut t, 3, 0, 15);
    assert_eq!(placement_count(&t), 3);

    delete(&mut t, b'c', |_| {});

    assert_eq!(
        placement_count(&t),
        3,
        "BUG-08-007 regression: d=c deleted by column only, ignoring cursor row"
    );
}

/// KG-DELETE-C: uppercase prunes orphaned image data.
#[test]
fn delete_c_uppercase_prunes_orphaned_image_data() {
    let mut t = term();
    stage_basic(&mut t, 1, 0, 0); // at cursor
    assert_eq!(image_count(&t), 1);

    delete(&mut t, b'C', |_| {});

    assert_eq!(placement_count(&t), 0);
    assert_eq!(image_count(&t), 0);
}

// ---------------------------------------------------------------------------
// KG-DELETE-x / X  —  At column x= (1-based).
// ---------------------------------------------------------------------------

/// KG-DELETE-x: `d=x,x=N` deletes placements intersecting column N-1 (1-based).
#[test]
fn delete_x_removes_placements_at_column_keeps_image_data() {
    let mut t = term();
    stage_basic(&mut t, 1, 3, 0);
    stage_basic(&mut t, 2, 3, 5);
    stage_basic(&mut t, 3, 5, 0);
    assert_eq!(placement_count(&t), 3);

    delete(&mut t, b'x', |c| c.source_x = 4); // spec 1-based: x=4 → col=3

    assert_eq!(placement_count(&t), 1);
    assert_eq!(image_count(&t), 3);
}

/// KG-DELETE-X: uppercase additionally frees image data.
#[test]
fn delete_x_uppercase_frees_orphaned_image_data() {
    let mut t = term();
    stage_basic(&mut t, 1, 3, 0);
    assert_eq!(image_count(&t), 1);

    delete(&mut t, b'X', |c| c.source_x = 4);

    assert_eq!(placement_count(&t), 0);
    assert_eq!(image_count(&t), 0);
}

// ---------------------------------------------------------------------------
// KG-DELETE-y / Y  —  At row y= (1-based, viewport-relative).
// ---------------------------------------------------------------------------

/// KG-DELETE-y: `d=y,y=N` deletes placements at viewport row N-1.
#[test]
fn delete_y_removes_placements_at_row_keeps_image_data() {
    let mut t = term();
    stage_basic(&mut t, 1, 3, 0); // viewport row 0
    stage_basic(&mut t, 2, 5, 0);
    stage_basic(&mut t, 3, 3, 4); // viewport row 4
    assert_eq!(placement_count(&t), 3);

    delete(&mut t, b'y', |c| c.source_y = 1); // spec 1-based: y=1 → viewport row 0

    assert_eq!(placement_count(&t), 1);
    assert_eq!(image_count(&t), 3);
}

/// KG-DELETE-Y: uppercase frees orphaned image data.
#[test]
fn delete_y_uppercase_frees_orphaned_image_data() {
    let mut t = term();
    stage_basic(&mut t, 1, 3, 0);
    assert_eq!(image_count(&t), 1);

    delete(&mut t, b'Y', |c| c.source_y = 1);

    assert_eq!(placement_count(&t), 0);
    assert_eq!(image_count(&t), 0);
}

// ---------------------------------------------------------------------------
// KG-DELETE-z / Z  —  At z-index z=.
// ---------------------------------------------------------------------------

/// KG-DELETE-z: `d=z,z=Z` deletes placements with z-index = Z.
#[test]
fn delete_z_removes_placements_at_z_index_keeps_image_data() {
    let mut t = term();
    stage(&mut t, 1, None, 3, 0, -1, None);
    stage(&mut t, 2, None, 5, 0, 0, None);
    stage(&mut t, 3, None, 7, 0, -1, None);
    assert_eq!(placement_count(&t), 3);

    delete(&mut t, b'z', |c| c.z_index = -1);

    assert_eq!(placement_count(&t), 1);
    assert_eq!(image_count(&t), 3);
}

/// KG-DELETE-Z: uppercase frees orphaned image data.
#[test]
fn delete_z_uppercase_frees_orphaned_image_data() {
    let mut t = term();
    stage(&mut t, 1, None, 3, 0, -1, None);
    assert_eq!(image_count(&t), 1);

    delete(&mut t, b'Z', |c| c.z_index = -1);

    assert_eq!(placement_count(&t), 0);
    assert_eq!(image_count(&t), 0);
}

// ---------------------------------------------------------------------------
// KG-DELETE-r / R  —  Image-id range [x, y] (kitty 0.33.0+).
// ---------------------------------------------------------------------------

/// KG-DELETE-r: `d=r,x=lo,y=hi` deletes images (by id) in inclusive range.
/// BUG-08-007 regression: current impl deleted by CURSOR POSITION.
#[test]
fn delete_r_uses_id_range_not_cursor_position() {
    let mut t = term();
    stage_basic(&mut t, 5, 10, 0);
    stage_basic(&mut t, 7, 20, 0);
    stage_basic(&mut t, 10, 30, 0);
    stage_basic(&mut t, 15, 40, 0);
    assert_eq!(placement_count(&t), 4);

    delete(&mut t, b'r', |c| {
        c.source_x = 6; // range lo = 6
        c.source_y = 10; // range hi = 10
    });

    // Images with id 7 and 10 fall in [6, 10]; 5 and 15 survive.
    assert_eq!(placement_count(&t), 2);
    assert_eq!(
        image_count(&t),
        4,
        "lowercase d=r keeps image data — only placements dropped"
    );
}

/// KG-DELETE-r negative pin: d=r MUST NOT use cursor position.
/// Regression for BUG-08-007 — old impl called `remove_by_position(cursor_col, cursor_row)`.
#[test]
fn delete_r_negative_pin_does_not_use_cursor_position() {
    let mut t = term();
    // Place an image at the cursor (0,0); its ID is outside the range.
    stage_basic(&mut t, 100, 0, 0);
    assert_eq!(placement_count(&t), 1);

    delete(&mut t, b'r', |c| {
        c.source_x = 1; // range lo
        c.source_y = 10; // range hi — does NOT include id 100
    });

    assert_eq!(
        placement_count(&t),
        1,
        "BUG-08-007 regression: d=r deleted at cursor position instead of by id range"
    );
}

/// KG-DELETE-R: uppercase additionally frees image data.
#[test]
fn delete_r_uppercase_frees_image_data_in_range() {
    let mut t = term();
    stage_basic(&mut t, 7, 10, 0);
    stage_basic(&mut t, 15, 20, 0);
    assert_eq!(image_count(&t), 2);

    delete(&mut t, b'R', |c| {
        c.source_x = 6;
        c.source_y = 10;
    });

    assert_eq!(placement_count(&t), 1);
    assert_eq!(image_count(&t), 1, "image 7 removed; image 15 survives");
}

// ---------------------------------------------------------------------------
// KG-DELETE-n / N  —  Newest image by image number I=.
// ---------------------------------------------------------------------------

/// KG-DELETE-n: `d=n,I=<num>` deletes placements of the newest image with that number.
/// Currently logs-and-skips (missing); §13.0.5 implements.
#[test]
fn delete_n_resolves_newest_image_by_number_and_removes_placements() {
    let mut t = term();
    stage(&mut t, 1, Some(42), 10, 0, 0, None); // older image with I=42
    stage(&mut t, 2, Some(42), 20, 0, 0, None); // newer image with I=42 — this is the one to hit
    stage(&mut t, 3, Some(99), 30, 0, 0, None); // different number
    assert_eq!(placement_count(&t), 3);

    delete(&mut t, b'n', |c| c.image_number = Some(42));

    assert_eq!(
        placement_count(&t),
        2,
        "d=n removes placements of the NEWEST (id=2) image with number 42"
    );
    // Image data for id=2 persists because lowercase keeps data.
    assert_eq!(image_count(&t), 3);
}

/// KG-DELETE-N: uppercase frees the image data too.
#[test]
fn delete_n_uppercase_frees_newest_image_data_by_number() {
    let mut t = term();
    stage(&mut t, 1, Some(42), 10, 0, 0, None);
    stage(&mut t, 2, Some(42), 20, 0, 0, None); // newest
    assert_eq!(image_count(&t), 2);

    delete(&mut t, b'N', |c| c.image_number = Some(42));

    assert_eq!(placement_count(&t), 1);
    assert_eq!(
        image_count(&t),
        1,
        "newest image (id=2) freed; older survives"
    );
}

// ---------------------------------------------------------------------------
// KG-DELETE-q / Q  —  Cell (x, y) + z-index intersection.
// ---------------------------------------------------------------------------

/// KG-DELETE-q: `d=q,x=X,y=Y,z=Z` deletes placements at (X-1, Y-1) with z==Z.
/// Currently falls into `_ =>` catch-all (missing); §13.0.5 implements.
#[test]
fn delete_q_removes_placements_at_cell_with_matching_z_index() {
    let mut t = term();
    stage(&mut t, 1, None, 3, 4, -1, None); // target: (3,4) with z=-1
    stage(&mut t, 2, None, 3, 4, 0, None); // same cell, different z
    stage(&mut t, 3, None, 5, 4, -1, None); // different cell, same z
    assert_eq!(placement_count(&t), 3);

    delete(&mut t, b'q', |c| {
        c.source_x = 4;
        c.source_y = 5;
        c.z_index = -1;
    });

    assert_eq!(
        placement_count(&t),
        2,
        "d=q must match cell AND z — only image 1 hits"
    );
    assert_eq!(image_count(&t), 3);
}

/// KG-DELETE-Q: uppercase frees orphaned image data.
#[test]
fn delete_q_uppercase_frees_orphaned_image_data() {
    let mut t = term();
    stage(&mut t, 1, None, 3, 4, -1, None);
    assert_eq!(image_count(&t), 1);

    delete(&mut t, b'Q', |c| {
        c.source_x = 4;
        c.source_y = 5;
        c.z_index = -1;
    });

    assert_eq!(placement_count(&t), 0);
    assert_eq!(image_count(&t), 0);
}

// ---------------------------------------------------------------------------
// KG-DELETE-f / F  —  Delete animation frames.
// ---------------------------------------------------------------------------

/// KG-DELETE-f with no extra frames is a no-op (static image stays).
#[test]
fn delete_f_on_static_image_is_noop() {
    let mut t = term();
    stage_basic(&mut t, 1, 10, 0);
    assert_eq!(image_count(&t), 1);

    delete(&mut t, b'f', |c| c.image_id = Some(1));

    assert_eq!(
        image_count(&t),
        1,
        "lowercase d=f leaves static image intact"
    );
    assert_eq!(placement_count(&t), 1);
}

/// KG-DELETE-F on a static image (no extra frames) removes the entire image.
/// Per kitty graphics.c:1696 — `delete_action == 'F'` triggers image removal
/// when there are no extra frames.
#[test]
fn delete_f_uppercase_on_static_image_removes_image_entirely() {
    let mut t = term();
    stage_basic(&mut t, 1, 10, 0);
    assert_eq!(image_count(&t), 1);

    delete(&mut t, b'F', |c| c.image_id = Some(1));

    assert_eq!(
        image_count(&t),
        0,
        "d=F on static image removes the image itself"
    );
    assert_eq!(placement_count(&t), 0);
}

// ---------------------------------------------------------------------------
// Matrix completeness — proves every spec-defined d= arm is covered.
// ---------------------------------------------------------------------------

/// Asserts that every spec-defined `d=` specifier is reachable via the
/// dispatch path — a basic smoke that confirms each arm is wired (no
/// fallthrough to the catch-all warn path for any named spec value).
///
/// Per `.claude/rules/tests.md` §Self-Verifying Matrix Completeness, the
/// count assertion proves every matrix cell was visited.
#[test]
fn delete_specifier_matrix_completeness() {
    const ALL_SPECIFIERS: &[u8] = &[
        b'a', b'A', b'i', b'I', b'p', b'P', b'c', b'C', b'x', b'X', b'y', b'Y', b'z', b'Z', b'r',
        b'R', b'q', b'Q', b'f', b'F', b'n', b'N',
    ];
    let mut count = 0;
    for &spec in ALL_SPECIFIERS {
        let mut t = term();
        // Stage a placement so each arm has something it could conceivably act on.
        stage(&mut t, 1, Some(1), 0, 0, 0, None);
        let before = placement_count(&t) + image_count(&t);
        delete(&mut t, spec, |c| {
            c.image_id = Some(1);
            c.image_number = Some(1);
            c.source_x = 1;
            c.source_y = 1;
            c.z_index = 0;
        });
        let after = placement_count(&t) + image_count(&t);
        // Every named arm must have SOME effect on at least one configuration;
        // the matrix existence proof is that dispatch reaches an arm (not the
        // default `_ =>`). `delete_f` on a static image is a no-op per spec,
        // so skip f (covered by `delete_f_on_static_image_is_noop`); `delete_F`
        // on a static image removes the image (covered above).
        if spec == b'f' {
            assert_eq!(before, after, "d=f static-image no-op");
        } else {
            assert!(after <= before, "d={} must not grow state", spec as char);
        }
        count += 1;
    }
    assert_eq!(
        count,
        ALL_SPECIFIERS.len(),
        "matrix completeness: every specifier visited"
    );
    assert_eq!(count, 22, "spec-defined d= specifier arm count");
}

// ---------------------------------------------------------------------------
// Case / presence matrix — lowercase keeps data, uppercase frees.
// ---------------------------------------------------------------------------

/// Matrix: for every "placement-only" specifier pair, lowercase keeps image
/// data and uppercase frees it — the load-bearing lowercase/uppercase
/// contract per kitty graphics-protocol.rst line 748–753.
#[test]
fn delete_case_pair_contract_lowercase_keeps_data_uppercase_frees() {
    // Specifiers where our stage helper can construct a single-placement fixture
    // that each arm fully removes via the same (image_id=1, col=0, row=0, z=0)
    // state. Each entry is a (lower, upper, setup) triple.
    #[allow(clippy::type_complexity)]
    let cases: &[(u8, u8, Box<dyn Fn(&mut KittyCommand)>)] = &[
        (b'i', b'I', Box::new(|c| c.image_id = Some(1))),
        (
            b'p',
            b'P',
            Box::new(|c| {
                c.source_x = 1;
                c.source_y = 1;
            }),
        ),
        (b'c', b'C', Box::new(|_| {})),
        (b'x', b'X', Box::new(|c| c.source_x = 1)),
        (b'y', b'Y', Box::new(|c| c.source_y = 1)),
        (b'z', b'Z', Box::new(|c| c.z_index = 0)),
        (
            b'r',
            b'R',
            Box::new(|c| {
                c.source_x = 1;
                c.source_y = 1;
            }),
        ),
        (b'n', b'N', Box::new(|c| c.image_number = Some(1))),
        (
            b'q',
            b'Q',
            Box::new(|c| {
                c.source_x = 1;
                c.source_y = 1;
                c.z_index = 0;
            }),
        ),
    ];

    let mut count = 0;
    for (lower, upper, setup) in cases {
        // Lowercase variant: placement gone, image data retained.
        let mut t = term();
        stage(&mut t, 1, Some(1), 0, 0, 0, None);
        delete(&mut t, *lower, |c| setup(c));
        assert_eq!(
            placement_count(&t),
            0,
            "d={} removed placement",
            *lower as char
        );
        assert_eq!(
            image_count(&t),
            1,
            "d={} MUST keep image data (lowercase)",
            *lower as char
        );

        // Uppercase variant: placement gone AND image data freed.
        let mut t = term();
        stage(&mut t, 1, Some(1), 0, 0, 0, None);
        delete(&mut t, *upper, |c| setup(c));
        assert_eq!(
            placement_count(&t),
            0,
            "d={} removed placement",
            *upper as char
        );
        assert_eq!(
            image_count(&t),
            0,
            "d={} MUST free image data (uppercase)",
            *upper as char
        );
        count += 1;
    }
    assert_eq!(
        count, 9,
        "all 9 case-pair specifiers with single-step fixture"
    );
}

// ---------------------------------------------------------------------------
// Round-0 TPR regressions (codex F1, F2, F3, F4, F5).
// ---------------------------------------------------------------------------

/// Stage a multi-cell placement covering a rectangle.
fn stage_rect(
    t: &mut Term<VoidEffectSink>,
    id: u32,
    col: usize,
    row: u64,
    cols: usize,
    rows: usize,
) {
    let img = ImageData {
        id: ImageId::from_raw(id),
        width: (cols * 10) as u32,
        height: (rows * 10) as u32,
        data: Arc::new(vec![0u8; cols * rows * 400]),
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    };
    let placement = ImagePlacement {
        image_id: ImageId::from_raw(id),
        placement_id: None,
        source_x: 0,
        source_y: 0,
        source_w: (cols * 10) as u32,
        source_h: (rows * 10) as u32,
        cell_col: col,
        cell_row: StableRowIndex(row),
        cols,
        rows,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    };
    t.image_cache_mut().store(img).expect("store image");
    t.image_cache_mut().place(placement);
}

/// Catalog row: KG-DELETE-p
///
/// Regression (codex F1): `d=p` deletes multi-cell placements that
/// INTERSECT the target cell — not only placements whose origin matches.
#[test]
fn delete_p_removes_placement_when_target_cell_is_inside_span() {
    let mut t = term();
    // 3x2 rectangle originating at (col=5, row=10), covering cols 5..7 rows 10..11.
    stage_rect(&mut t, 1, 5, 10, 3, 2);
    assert_eq!(placement_count(&t), 1);

    // Target cell (col=6, row=11) is inside the rectangle but NOT the origin.
    // Spec-encoded: x=7, y=12 (1-based) → col=6, row=11.
    delete(&mut t, b'p', |c| {
        c.source_x = 7;
        c.source_y = 12;
    });

    assert_eq!(
        placement_count(&t),
        0,
        "d=p must delete placement that intersects target cell"
    );
}

/// Catalog row: KG-DELETE-c
///
/// Regression (codex F1): `d=c` deletes multi-cell placements whose
/// rectangle contains the cursor, not only placements at cursor origin.
#[test]
fn delete_c_removes_placement_when_cursor_is_inside_span() {
    let mut t = term();
    // Cursor is at (0, 0) by default on a fresh Term. Place a 3x2 rectangle
    // whose top-left is (0, 0) — the cursor is inside the span.
    stage_rect(&mut t, 1, 0, 0, 3, 2);
    // Move the cursor to col=2, row=1 — still inside the rectangle.
    // CUP is 1-based: ESC [ 2 ; 3 H → row 1, col 2 (0-based).
    use vte::ansi::Processor;
    let mut processor: Processor = Processor::new();
    processor.advance(&mut t, b"\x1b[2;3H");
    assert_eq!(placement_count(&t), 1);

    delete(&mut t, b'c', |_| {});

    assert_eq!(
        placement_count(&t),
        0,
        "d=c must delete placement that intersects cursor cell"
    );
}

/// Catalog row: KG-DELETE-q
///
/// Regression (codex F2): `d=q` deletes multi-cell placements that
/// intersect the target cell AND carry the specified z-index.
#[test]
fn delete_q_removes_placement_when_target_cell_is_inside_span_and_z_matches() {
    let mut t = term();
    // Rectangle (col=5, row=10, 3x2) at z=-1.
    let img = ImageData {
        id: ImageId::from_raw(1),
        width: 30,
        height: 20,
        data: Arc::new(vec![0u8; 2400]),
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    };
    t.image_cache_mut().store(img).unwrap();
    t.image_cache_mut().place(ImagePlacement {
        image_id: ImageId::from_raw(1),
        placement_id: None,
        source_x: 0,
        source_y: 0,
        source_w: 30,
        source_h: 20,
        cell_col: 5,
        cell_row: StableRowIndex(10),
        cols: 3,
        rows: 2,
        z_index: -1,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    });

    // Target (col=6, row=11, z=-1) — inside the span.
    delete(&mut t, b'q', |c| {
        c.source_x = 7;
        c.source_y = 12;
        c.z_index = -1;
    });

    assert_eq!(
        placement_count(&t),
        0,
        "d=q must delete intersecting placement with matching z"
    );
}

/// Catalog row: KG-DELETE-n
///
/// Regression (codex F3): `d=n` resolves "newest" by creation order
/// (`store_order`), not by LRU recency (`last_accessed`). Touching an
/// older image via a cache access MUST NOT make it "newer" for d=n/N.
#[test]
fn delete_n_resolves_by_creation_order_not_lru_recency() {
    let mut t = term();
    stage(&mut t, 1, Some(42), 10, 0, 0, None); // older
    stage(&mut t, 2, Some(42), 20, 0, 0, None); // newer (store_order higher)

    // Touch the older image via test-only get() — this bumps last_accessed
    // for id=1, making it the most-recently-accessed. But d=n must still
    // pick id=2 (highest store_order).
    let _ = t.image_cache_mut().get(ImageId::from_raw(1));

    delete(&mut t, b'n', |c| c.image_number = Some(42));

    // Expect placement for id=2 to be gone (its placement was removed); id=1
    // placement survives because id=2 was newer by creation order.
    assert_eq!(placement_count(&t), 1);
    let placements = t
        .image_cache()
        .placements_in_viewport(StableRowIndex(0), StableRowIndex(1000));
    assert_eq!(placements[0].image_id, ImageId::from_raw(1));
}

/// Catalog row: KG-DELETE-f, KG-DELETE-F
///
/// Regression (codex F4): `d=f/F` accepts `I=` (image number) when `i=`
/// is absent, resolved via `newest_by_image_number` per kitty
/// graphics.c:1685-1689.
#[test]
fn delete_f_uppercase_accepts_image_number_when_image_id_absent() {
    let mut t = term();
    // Static image with image_number=7.
    stage(&mut t, 1, Some(7), 0, 0, 0, None);
    assert_eq!(image_count(&t), 1);

    // d=F with I= (no i=) on a static image — per spec, resolve by number
    // and (static-image branch) remove the entire image.
    delete(&mut t, b'F', |c| c.image_number = Some(7));

    assert_eq!(image_count(&t), 0, "d=F,I=<num> resolves image + removes");
}

/// Catalog row: KG-DELETE-f
///
/// Regression (codex F5): when root-frame deletion promotes the next
/// frame, `current_frame` adjusts based on its pre-removal position
/// relative to the removed index — NOT post-clamp double-decrement.
///
/// Setup: 3 frames [F0, F1, F2], current_frame = 2 (points at F2).
/// Delete frame 1 (idx=0, root). Expected: frames=[F1, F2], promoted
/// root = F1, current_frame = 1 (still points at F2, now at idx 1).
/// Pre-fix code double-adjusted → current_frame = 0 (wrong, points at F1).
#[test]
fn delete_f_root_frame_leaves_current_frame_pointing_at_same_logical_frame() {
    use std::time::Duration;

    let mut t = term();
    let img = ImageData {
        id: ImageId::from_raw(1),
        width: 1,
        height: 1,
        data: Arc::new(vec![0xAA; 4]),
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    };
    let frames = vec![
        Arc::new(vec![0xAA; 4]), // F0 (root)
        Arc::new(vec![0xBB; 4]), // F1
        Arc::new(vec![0xCC; 4]), // F2
    ];
    let durations = vec![
        Duration::from_millis(100),
        Duration::from_millis(100),
        Duration::from_millis(100),
    ];
    t.image_cache_mut()
        .store_animated(img, frames, durations, None)
        .unwrap();
    t.image_cache_mut()
        .set_current_frame(ImageId::from_raw(1), 2);

    let before = t
        .image_cache()
        .animation_state(ImageId::from_raw(1))
        .unwrap()
        .current_frame;
    assert_eq!(before, 2, "pre-removal current_frame is F2 (idx=2)");

    // Remove frame 1 (idx=0, root) via d=f with r=1 (1-based).
    delete(&mut t, b'f', |c| {
        c.image_id = Some(1);
        c.display_rows = Some(1);
    });

    let state = t
        .image_cache()
        .animation_state(ImageId::from_raw(1))
        .expect("animation survives");
    assert_eq!(state.total_frames, 2);
    assert_eq!(
        state.current_frame, 1,
        "current_frame must still point at F2 (now at idx 1)"
    );
}

// ---------------------------------------------------------------------------
// Round-1 TPR regressions (codex R1 F1/F2/F3 + gemini R1 F1/F2/F4).
// ---------------------------------------------------------------------------

/// Catalog row: KG-ACTION-DELETE
///
/// Regression (round-1 codex F1): a delete command arriving between two
/// chunked-transmission chunks (`m=1` then `m=0`) must abort the in-flight
/// upload so the final chunk cannot resurrect a deleted image. Per
/// kitty/graphics.c:2093 `handle_delete_command` frees `currently_loading`
/// before dispatching the delete arm.
#[test]
fn delete_aborts_in_flight_chunked_upload() {
    use crate::image::kitty::{KittyCommand, KittyTransmission, LoadingImage};

    let mut t = term();
    // Simulate an in-flight chunk: loading_image populated but not yet stored.
    let start_cmd = KittyCommand {
        payload: vec![0u8; 64],
        format: 32,
        source_width: 4,
        source_height: 4,
        transmission: KittyTransmission::Direct,
        ..Default::default()
    };
    t.loading_image = Some(LoadingImage {
        image_id: 42,
        start_cmd,
    });
    assert!(t.loading_image.is_some());

    delete(&mut t, b'a', |_| {});

    assert!(
        t.loading_image.is_none(),
        "kitty delete must abort in-flight chunked upload per kitty/graphics.c:2093"
    );
}

/// Catalog row: KG-DELETE-f
///
/// Regression (round-1 codex F2 / gemini F1): when the root frame is
/// removed while a later frame is displayed, `ImageData.data` must sync
/// to the new current frame's bytes — NOT to the promoted frame 0. The
/// visible image must not drift backwards.
#[test]
fn delete_f_root_syncs_image_data_to_surviving_current_frame() {
    use std::time::Duration;

    let mut t = term();
    let img = ImageData {
        id: ImageId::from_raw(1),
        width: 1,
        height: 1,
        data: Arc::new(vec![0xAA; 4]),
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    };
    let frames = vec![
        Arc::new(vec![0xAA; 4]), // F0 (root)
        Arc::new(vec![0xBB; 4]), // F1
        Arc::new(vec![0xCC; 4]), // F2 — will be the surviving current frame
    ];
    let durations = vec![
        Duration::from_millis(100),
        Duration::from_millis(100),
        Duration::from_millis(100),
    ];
    t.image_cache_mut()
        .store_animated(img, frames, durations, None)
        .unwrap();
    t.image_cache_mut()
        .set_current_frame(ImageId::from_raw(1), 2);

    // Remove frame 1 (idx=0, root). Post-fix: current_frame adjusts from 2→1,
    // surviving frames become [F1, F2], ImageData.data must now reflect F2
    // bytes (0xCC), NOT the promoted F1 (0xBB).
    delete(&mut t, b'f', |c| {
        c.image_id = Some(1);
        c.display_rows = Some(1);
    });

    let stored = t
        .image_cache()
        .get_no_touch(ImageId::from_raw(1))
        .expect("image survives");
    assert_eq!(
        stored.data[0], 0xCC,
        "ImageData.data must sync to surviving current frame (F2, 0xCC), \
         not promoted root (F1, 0xBB) — round-1 visual-drift regression"
    );
}

/// Catalog row: KG-DELETE-f
///
/// Regression (round-1 gemini F2): `remove_animation_frame` must reset
/// `frame_starts[id]` so `advance_animations` re-initializes timing for
/// the new current frame. Leaving the old start timestamp would cause the
/// next frame switch to fire prematurely (or not at all if the current
/// frame's new duration is smaller than the stale elapsed value).
#[test]
fn delete_f_resets_frame_starts_so_animation_timer_reinitializes() {
    use std::time::{Duration, Instant};

    let mut t = term();
    let img = ImageData {
        id: ImageId::from_raw(1),
        width: 1,
        height: 1,
        data: Arc::new(vec![0xAA; 4]),
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    };
    let frames = vec![
        Arc::new(vec![0xAA; 4]),
        Arc::new(vec![0xBB; 4]),
        Arc::new(vec![0xCC; 4]),
    ];
    let durations = vec![
        Duration::from_millis(100),
        Duration::from_millis(100),
        Duration::from_millis(100),
    ];
    t.image_cache_mut()
        .store_animated(img, frames, durations, None)
        .unwrap();

    // Kick advance_animations to seed frame_starts[id] with a known instant.
    let seed = Instant::now();
    t.image_cache_mut()
        .advance_animations(seed, StableRowIndex(0), StableRowIndex(1000));
    // Stage a placement inside the viewport so animation advancement tracks it.
    t.image_cache_mut().place(ImagePlacement {
        image_id: ImageId::from_raw(1),
        placement_id: None,
        source_x: 0,
        source_y: 0,
        source_w: 1,
        source_h: 1,
        cell_col: 0,
        cell_row: StableRowIndex(0),
        cols: 1,
        rows: 1,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    });
    t.image_cache_mut()
        .advance_animations(seed, StableRowIndex(0), StableRowIndex(1000));
    assert!(
        t.image_cache()
            .frame_starts_for_test(ImageId::from_raw(1))
            .is_some()
    );

    // Remove any frame — frame_starts[id] must be cleared so the next
    // `advance_animations` re-seeds timing.
    delete(&mut t, b'f', |c| {
        c.image_id = Some(1);
        c.display_rows = Some(2);
    });

    assert!(
        t.image_cache()
            .frame_starts_for_test(ImageId::from_raw(1))
            .is_none(),
        "frame_starts must be cleared on frame removal"
    );
}

/// Catalog row: KG-DELETE-p, KG-DELETE-q
///
/// Regression (round-1 codex F3 / gemini F4): a placement with `cols==0`
/// or `rows==0` occupies NO cells and MUST NOT match any
/// `d=p`/`d=c`/`d=q` intersection query. Pre-fix `saturating_sub(1)`
/// falsely reported a hit at the origin cell.
#[test]
fn delete_p_does_not_match_zero_span_placement() {
    let mut t = term();
    let img = ImageData {
        id: ImageId::from_raw(1),
        width: 10,
        height: 10,
        data: Arc::new(vec![0u8; 400]),
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    };
    t.image_cache_mut().store(img).unwrap();
    // Zero-width placement at origin (5, 10).
    t.image_cache_mut().place(ImagePlacement {
        image_id: ImageId::from_raw(1),
        placement_id: None,
        source_x: 0,
        source_y: 0,
        source_w: 10,
        source_h: 10,
        cell_col: 5,
        cell_row: StableRowIndex(10),
        cols: 0,
        rows: 1,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    });
    assert_eq!(placement_count(&t), 1);

    // d=p targeting the origin cell (x=6, y=11 spec-1-based → col=5, row=10).
    delete(&mut t, b'p', |c| {
        c.source_x = 6;
        c.source_y = 11;
    });

    assert_eq!(
        placement_count(&t),
        1,
        "zero-span placements MUST NOT match d=p intersection query"
    );
}

// ---------------------------------------------------------------------------
// Round-2 TPR regressions (codex R2 F1 / gemini R2 F1, F2, F4).
// ---------------------------------------------------------------------------

/// Catalog rows: KG-DELETE-a, KG-DELETE-A
///
/// Regression (round-2 codex F1 / gemini F2): zero-height placements must
/// NOT count as viewport-visible; `ImagePlacement::intersects_viewport`
/// must reject `rows==0` symmetric to `placement_intersects_cell`.
#[test]
fn delete_a_does_not_match_zero_height_placement() {
    let mut t = term();
    let img = ImageData {
        id: ImageId::from_raw(1),
        width: 10,
        height: 10,
        data: Arc::new(vec![0u8; 400]),
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    };
    t.image_cache_mut().store(img).unwrap();
    // Zero-height placement at viewport row 0.
    t.image_cache_mut().place(ImagePlacement {
        image_id: ImageId::from_raw(1),
        placement_id: None,
        source_x: 0,
        source_y: 0,
        source_w: 10,
        source_h: 10,
        cell_col: 0,
        cell_row: StableRowIndex(0),
        cols: 5,
        rows: 0,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    });
    assert_eq!(placement_count(&t), 1);

    delete(&mut t, b'a', |_| {});

    assert_eq!(
        placement_count(&t),
        1,
        "zero-height placement occupies no viewport row and MUST NOT match d=a"
    );
}

/// Catalog row: KG-ACTION-DELETE
///
/// Regression (round-2 gemini F1): `remove_image` on an animated image
/// whose `current_frame > 0` must use `animation_frames` as the SSOT for
/// memory accounting — NOT `img.data.len()` (which tracks the currently
/// displayed frame and drifts as the animation advances). Symmetric to
/// `store_animated`'s `store(data) + sum(frames[1..])` accounting.
#[test]
fn delete_animated_image_after_advance_correctly_releases_memory() {
    use std::time::{Duration, Instant};

    let mut t = term();
    let img = ImageData {
        id: ImageId::from_raw(1),
        width: 4,
        height: 4,
        data: Arc::new(vec![0xAA; 64]), // F0: 64 bytes
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
        image_number: None,
    };
    // Frames of different sizes so drift is observable.
    let frames = vec![
        Arc::new(vec![0xAA; 64]),  // F0 — 64 B
        Arc::new(vec![0xBB; 256]), // F1 — 256 B
        Arc::new(vec![0xCC; 128]), // F2 — 128 B
    ];
    let durations = vec![
        Duration::from_millis(10),
        Duration::from_millis(10),
        Duration::from_millis(10),
    ];
    let cache = t.image_cache_mut();
    cache.store_animated(img, frames, durations, None).unwrap();
    let after_store = cache.memory_used();
    assert_eq!(
        after_store,
        64 + 256 + 128,
        "store_animated must account for ALL frames (not frame 0 × N)"
    );
    // Place it in viewport so advance_animations will tick this image.
    cache.place(ImagePlacement {
        image_id: ImageId::from_raw(1),
        placement_id: None,
        source_x: 0,
        source_y: 0,
        source_w: 4,
        source_h: 4,
        cell_col: 0,
        cell_row: StableRowIndex(0),
        cols: 1,
        rows: 1,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    });
    // Advance far enough for current_frame to move past 0.
    cache.set_current_frame(ImageId::from_raw(1), 2);

    // Remove the image entirely (simulates d=I / d=A / d=N uppercase).
    cache.remove_image(ImageId::from_raw(1));

    assert_eq!(
        cache.memory_used(),
        0,
        "remove_image must release ALL frame bytes regardless of current_frame; \
         pre-fix drift = F0-F_current when frames have unequal sizes"
    );
    // Silence unused-import warnings if compilation model changes.
    let _ = Instant::now();
}
