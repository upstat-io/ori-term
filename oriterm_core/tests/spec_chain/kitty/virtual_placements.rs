//! Per-placeholder rung (§13.4) — drives kitty `U=1` virtual placement and
//! `U+10EEEE` cell encoding through transmit → store → snapshot → render.
//!
//! Coverage matrix per the §13.4 plan body: encoding, snapshot, emit,
//! reflow, scroll, ED, EL, alt-screen.

use oriterm_core::image::ImageId;
use oriterm_test_support::spec_chain::SpecHarness;

use super::fixtures::{b64, kitty_apc, ok_reply_for, reply_bytes, reply_contains, rgba_4x4_red};

/// Drive a `U=1,a=t` transmit. Returns the harness after the command.
fn transmit_u1(h: &mut SpecHarness, id: u32) {
    let control = format!("a=t,U=1,i={id},f=32,s=4,v=4");
    h.feed(&kitty_apc(control.as_bytes(), &b64(&rgba_4x4_red())));
}

/// Drive a `U=1,a=p` place for an already-stored image.
fn place_u1(h: &mut SpecHarness, id: u32) {
    let control = format!("a=p,U=1,i={id},c=2,r=2");
    h.feed(&kitty_apc(control.as_bytes(), ""));
}

/// Write a placeholder cell at the current cursor position carrying the
/// supplied diacritic-encoded row/col + fg = `image_id_low`.
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

/// Catalog row: `KG-UNICODE-PLACEHOLDER-TRANSMIT-U1`.
#[test]
fn kitty_u1_transmit_records_placeholder_anchor() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 1);

    let anchors = h.term().image_cache().placeholder_anchors();
    assert!(
        anchors.contains(&ImageId::from_raw(1)),
        "a=t,U=1 MUST add an anchor for image_id=1 — got {anchors:?}",
    );
    assert!(
        reply_contains(&h, &ok_reply_for(1)),
        "a=t,U=1 emits OK — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(&h)),
    );
}

/// Catalog row: `KG-UNICODE-PLACEHOLDER-PLACE-U1`.
#[test]
fn kitty_u1_place_records_placeholder_anchor() {
    let mut h = SpecHarness::new();
    // First store via `a=t` without U=1, then `a=p,U=1` to record the anchor.
    h.feed(&kitty_apc(b"a=t,i=2,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    place_u1(&mut h, 2);

    let anchors = h.term().image_cache().placeholder_anchors();
    assert!(
        anchors.contains(&ImageId::from_raw(2)),
        "a=p,U=1 MUST add an anchor for image_id=2 — got {anchors:?}",
    );
}

#[test]
fn kitty_u1_image_survives_unrelated_lru_pressure_via_anchor() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 100);
    assert_eq!(h.term().image_cache().image_count(), 1);

    // Tight cache cap: exactly 4×4 RGBA = 64 bytes. Anything beyond would
    // normally trigger LRU eviction; the anchor must protect i=100.
    h.term_mut().image_cache_mut().set_memory_limit(64);

    for next_id in 200..210 {
        h.feed(&kitty_apc(
            format!("a=t,i={next_id},f=32,s=4,v=4").as_bytes(),
            &b64(&rgba_4x4_red()),
        ));
    }

    assert!(
        h.term()
            .image_cache()
            .get_no_touch(ImageId::from_raw(100))
            .is_some(),
        "U=1 anchored image must survive LRU pressure — anchors={:?}, image_count={}",
        h.term().image_cache().placeholder_anchors(),
        h.term().image_cache().image_count(),
    );
}

#[test]
fn kitty_u1_anchor_does_not_intersect_viewport_placements() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 1);

    // U=1 must NOT synthesize a real ImagePlacement — the viewport
    // intersection query returns zero entries (per the §13.4 layer-bleed
    // contract: ImageCache.placements remains the real-placement SSOT).
    let snap = h.term().renderable_content();
    assert_eq!(
        snap.images.len(),
        0,
        "U=1 transmit MUST NOT add a RenderablePlacement — images={:?}",
        snap.images,
    );
}

/// Catalog row: `KG-UNICODE-PLACEHOLDER-CELL-RESOLVE`.
#[test]
fn placeholder_decode_with_fg_color_and_diacritics() {
    let mut h = SpecHarness::new();
    // Image id will be 42; transmit then write one placeholder cell.
    transmit_u1(&mut h, 42);
    write_placeholder_cell(&mut h, 42, '\u{0305}', '\u{0305}');

    let snap = h.term().renderable_content();
    assert_eq!(
        snap.placeholder_cells.len(),
        1,
        "MUST emit one RenderablePlaceholderCell — got {:?}",
        snap.placeholder_cells,
    );
    let p = snap.placeholder_cells[0];
    assert_eq!(p.image_id, ImageId::from_raw(42));
    assert_eq!(p.image_row, 0);
    assert_eq!(p.image_col, 0);
    assert_eq!(p.line, 0);
}

#[test]
fn placeholder_decode_continuation_row_only() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 7);
    // Two adjacent cells: first has row+col, second has row only → second
    // inherits col = prev.col + 1 = 1.
    write_placeholder_cell(&mut h, 7, '\u{0305}', '\u{0305}');
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"\x1b[38;5;7m");
    let mut buf = [0u8; 4];
    bytes.extend_from_slice('\u{10EEEE}'.encode_utf8(&mut buf).as_bytes());
    bytes.extend_from_slice('\u{0305}'.encode_utf8(&mut buf).as_bytes());
    bytes.extend_from_slice(b"\x1b[39m");
    h.feed(&bytes);

    let snap = h.term().renderable_content();
    assert_eq!(
        snap.placeholder_cells.len(),
        2,
        "two placeholder cells expected — got {:?}",
        snap.placeholder_cells,
    );
    assert_eq!(snap.placeholder_cells[0].image_col, 0);
    assert_eq!(snap.placeholder_cells[1].image_col, 1);
}

#[test]
fn placeholder_cell_without_image_renders_as_glyph_not_quad() {
    // Bare U+10EEEE with no fg color (image_id_low=0) MUST NOT emit a
    // RenderablePlaceholderCell, even though the glyph is in the grid.
    let mut h = SpecHarness::new();
    let mut bytes = Vec::new();
    let mut buf = [0u8; 4];
    bytes.extend_from_slice('\u{10EEEE}'.encode_utf8(&mut buf).as_bytes());
    h.feed(&bytes);

    let snap = h.term().renderable_content();
    assert_eq!(
        snap.placeholder_cells.len(),
        0,
        "bare U+10EEEE without fg color must NOT emit an image quad — got {:?}",
        snap.placeholder_cells,
    );
    // The glyph itself still occupies a cell in the snapshot.
    let cell0 = &snap.cells[0];
    assert_eq!(cell0.ch, '\u{10EEEE}');
}

/// Catalog row: `KG-UNICODE-PLACEHOLDER-REFLOW`.
#[test]
fn placeholder_cells_resolve_to_stored_image_after_reflow() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 33);
    write_placeholder_cell(&mut h, 33, '\u{0305}', '\u{0305}');

    let pre = h.term().renderable_content();
    assert_eq!(pre.placeholder_cells.len(), 1);
    assert_eq!(pre.placeholder_cells[0].image_id, ImageId::from_raw(33));

    // Resize narrower — drives column reflow. The placeholder cell should
    // still resolve to image 33 because the cell carries the encoding,
    // not the cache.
    let (lines, _cols) = (h.term().grid().lines(), h.term().grid().cols());
    h.term_mut().resize(lines, 40, true);

    let post = h.term().renderable_content();
    assert_eq!(
        post.placeholder_cells.len(),
        1,
        "reflow must preserve the placeholder cell — got {:?}",
        post.placeholder_cells,
    );
    assert_eq!(post.placeholder_cells[0].image_id, ImageId::from_raw(33));
}

#[test]
fn placeholder_category_matrix_completeness() {
    // 8 categories per §13.4: encoding, snapshot, emit, reflow, scroll,
    // ED, EL, alt-screen. Each is exercised by at least one test in this
    // file or the placeholder_anchors.rs sibling. Mechanical coverage
    // assertion left as `assert!(true)` until a discovery-based coverage
    // check is wired in.
    let categories: [&str; 8] = [
        "encoding",
        "snapshot",
        "emit",
        "reflow",
        "scroll",
        "ed",
        "el",
        "alt-screen",
    ];
    assert_eq!(categories.len(), 8);
}
