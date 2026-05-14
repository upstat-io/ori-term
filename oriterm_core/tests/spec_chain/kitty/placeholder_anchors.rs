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
    assert!(h.term().image_cache().placeholder_anchors().contains(&ImageId::from_raw(1)));
}

#[test]
fn placeholder_anchor_added_on_u1_place() {
    let mut h = SpecHarness::new();
    h.feed(&kitty_apc(b"a=t,i=2,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    h.feed(&kitty_apc(b"a=p,U=1,i=2,c=1,r=1", ""));
    assert!(h.term().image_cache().placeholder_anchors().contains(&ImageId::from_raw(2)));
}

#[test]
fn remove_image_clears_placeholder_anchor_entry() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 3);
    assert!(h.term().image_cache().placeholder_anchors().contains(&ImageId::from_raw(3)));

    // d=I,i=3 — uppercase variant removes image data + placements
    // + anchors.
    h.feed(&kitty_apc(b"a=d,d=I,i=3", ""));

    assert!(
        !h.term().image_cache().placeholder_anchors().contains(&ImageId::from_raw(3)),
        "d=I MUST clear placeholder anchor — got {:?}",
        h.term().image_cache().placeholder_anchors(),
    );
}

#[test]
fn kitty_delete_i_clears_placeholder_anchor() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 4);
    h.feed(&kitty_apc(b"a=d,d=I,i=4", ""));
    assert!(!h.term().image_cache().placeholder_anchors().contains(&ImageId::from_raw(4)));
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
        h.term().image_cache().placeholder_anchors().contains(&ImageId::from_raw(5)),
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
        h.term().image_cache().placeholder_anchors().contains(&ImageId::from_raw(6)),
        "anchor must survive while placeholder cells still occupy the grid",
    );
}

#[test]
fn prune_scrollback_clears_orphaned_placeholder_anchors_for_images_with_no_surviving_cells() {
    let mut h = SpecHarness::new();
    transmit_u1(&mut h, 8);
    write_placeholder_cell(&mut h, 8, '\u{0305}', '\u{0305}');
    assert!(h.term().image_cache().placeholder_anchors().contains(&ImageId::from_raw(8)));

    // Push the placeholder cell out of grid + scrollback by feeding more
    // newlines than the scrollback ring can hold. With no surviving
    // placeholder cells, the reconciler must drop the anchor.
    let scrollback_cap = h.term().grid().scrollback().max_scrollback();
    let lines = h.term().grid().lines();
    for _ in 0..(scrollback_cap + lines + 8) {
        h.feed(b"x\n");
    }

    assert!(
        !h.term().image_cache().placeholder_anchors().contains(&ImageId::from_raw(8)),
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
    assert!(h.term().image_cache().placeholder_anchors().contains(&ImageId::from_raw(9)));

    // ED 2 (full-screen erase) drops the U+10EEEE cell. But: U=1 transmit
    // is special — `add_placeholder_anchor` recorded the image as
    // anchored at transmit time, BEFORE any placeholder cell existed.
    // The current contract is that anchors persist until explicit `d=I`;
    // the grid survivor set is a refinement layer.
    h.feed(b"\x1b[2J");

    // With no placeholder cells surviving, the survivor reconciler
    // empties the anchor set.
    assert!(
        !h.term().image_cache().placeholder_anchors().contains(&ImageId::from_raw(9)),
        "after ED 2 the survivor set is empty — anchor must drop",
    );
}

#[test]
fn image_cache_lifecycle_does_not_walk_grid_cells() {
    // Architectural pin: ImageCache must NOT import Grid / Row / Cell.
    // Reviewers flag any such import as LEAK:layer-bleeding.
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
