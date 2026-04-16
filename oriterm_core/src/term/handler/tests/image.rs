use std::sync::Arc;

use crate::grid::StableRowIndex;
use crate::image::{ImageData, ImageFormat, ImageId, ImagePlacement, ImageSource, PlacementSizing};
use crate::term::Term;
use crate::theme::Theme;

use super::super::test_helpers::feed;

/// Create a Term with VoidEffectSink (when effects don't matter).
fn term() -> Term<crate::effect::VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), crate::effect::VoidEffectSink)
}

// --- Image clearing on erase operations ---

/// Create a test image and placement at the given grid position.
fn place_test_image(
    t: &mut Term<crate::effect::VoidEffectSink>,
    col: usize,
    row: usize,
) -> ImageId {
    let id = t.image_cache_mut().next_image_id();
    let data = ImageData {
        id,
        width: 8,
        height: 16,
        data: Arc::new(vec![0u8; 8 * 16 * 4]),
        format: ImageFormat::Rgba,
        source: ImageSource::Direct,
        last_accessed: 0,
    };
    t.image_cache_mut().store(data).expect("store failed");
    let grid = t.grid();
    let stable = StableRowIndex::from_absolute(grid, grid.scrollback().len() + row);
    let placement = ImagePlacement {
        image_id: id,
        placement_id: None,
        source_x: 0,
        source_y: 0,
        source_w: 8,
        source_h: 16,
        cell_col: col,
        cell_row: stable,
        cols: 2,
        rows: 1,
        z_index: 0,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing: PlacementSizing::CellCount,
    };
    t.image_cache_mut().place(placement);
    id
}

#[test]
fn ed_below_clears_images_below_cursor() {
    let mut t = term();
    place_test_image(&mut t, 0, 0); // Row 0 — above cursor.
    place_test_image(&mut t, 5, 10); // Row 10 — below cursor.
    // Move cursor to row 5, col 0.
    feed(&mut t, b"\x1b[6;1H");
    assert_eq!(t.image_cache().placement_count(), 2);

    // ED 0 (erase below).
    feed(&mut t, b"\x1b[0J");
    assert_eq!(t.image_cache().placement_count(), 1);
    // Row 0 image should remain.
    let grid = t.grid();
    let stable_0 = StableRowIndex::from_absolute(grid, grid.scrollback().len());
    let visible = t.image_cache().placements_in_viewport(stable_0, stable_0);
    assert_eq!(visible.len(), 1);
}

#[test]
fn ed_above_clears_images_above_cursor() {
    let mut t = term();
    place_test_image(&mut t, 0, 0); // Row 0 — above cursor.
    place_test_image(&mut t, 5, 20); // Row 20 — below cursor.
    // Move cursor to row 10.
    feed(&mut t, b"\x1b[11;1H");
    assert_eq!(t.image_cache().placement_count(), 2);

    // ED 1 (erase above).
    feed(&mut t, b"\x1b[1J");
    assert_eq!(t.image_cache().placement_count(), 1);
}

#[test]
fn ed_all_clears_all_images() {
    let mut t = term();
    place_test_image(&mut t, 0, 0);
    place_test_image(&mut t, 5, 10);
    place_test_image(&mut t, 10, 20);
    assert_eq!(t.image_cache().placement_count(), 3);

    // ED 2 (erase all).
    feed(&mut t, b"\x1b[2J");
    assert_eq!(t.image_cache().placement_count(), 0);
}

#[test]
fn el_right_clears_images_right_of_cursor() {
    let mut t = term();
    place_test_image(&mut t, 0, 0); // Col 0 — left of cursor.
    place_test_image(&mut t, 50, 0); // Col 50 — right of cursor.
    // Move cursor to col 10 on row 0.
    feed(&mut t, b"\x1b[1;11H");
    assert_eq!(t.image_cache().placement_count(), 2);

    // EL 0 (erase right).
    feed(&mut t, b"\x1b[0K");
    // Image at col 50 (right of cursor) should be removed.
    assert_eq!(t.image_cache().placement_count(), 1);
}

#[test]
fn el_all_clears_images_on_line() {
    let mut t = term();
    place_test_image(&mut t, 0, 0); // Row 0 — same as cursor.
    place_test_image(&mut t, 5, 5); // Row 5 — different line.
    assert_eq!(t.image_cache().placement_count(), 2);

    // EL 2 (erase entire line, cursor on row 0).
    feed(&mut t, b"\x1b[2K");
    assert_eq!(t.image_cache().placement_count(), 1);
}

#[test]
fn ech_clears_images_in_char_range() {
    let mut t = term();
    place_test_image(&mut t, 5, 0); // Col 5-6 on row 0.
    place_test_image(&mut t, 20, 0); // Col 20-21 on row 0.
    // Move cursor to col 4 on row 0.
    feed(&mut t, b"\x1b[1;5H");

    // ECH 5 (erase 5 chars from col 4 to col 8).
    feed(&mut t, b"\x1b[5X");
    // Image at col 5-6 overlaps erased range → removed.
    // Image at col 20-21 is outside → kept.
    assert_eq!(t.image_cache().placement_count(), 1);
}

#[test]
fn scrollback_eviction_prunes_image_placements() {
    // Create a term with 1 scrollback line.
    let mut t = Term::new(5, 80, 1, Theme::default(), crate::effect::VoidEffectSink);
    // Place image on row 0.
    place_test_image(&mut t, 0, 0);
    assert_eq!(t.image_cache().placement_count(), 1);

    // Scroll enough lines to push row 0 into scrollback and then evict it.
    // With 5 lines and 1 scrollback: after 6 linefeeds at the bottom,
    // 6 rows scroll up, 1 goes to scrollback, 5 more push the scrollback
    // row out (evicted).
    // Move to last line, then linefeed multiple times.
    feed(&mut t, b"\x1b[5;1H"); // Move to line 5.
    for _ in 0..10 {
        feed(&mut t, b"\n");
    }

    // Placement should have been pruned.
    assert_eq!(t.image_cache().placement_count(), 0);
}

#[test]
fn resize_prunes_evicted_image_placements() {
    // Create a term with 2 scrollback lines.
    let mut t = Term::new(10, 80, 2, Theme::default(), crate::effect::VoidEffectSink);
    // Place image on row 0 (first visible row).
    place_test_image(&mut t, 0, 0);
    assert_eq!(t.image_cache().placement_count(), 1);

    // Fill visible area so shrinking pushes rows to scrollback/eviction.
    for line in 0..10 {
        let seq = format!("\x1b[{};1HLine{line}", line + 1);
        feed(&mut t, seq.as_bytes());
    }

    // Shrink to 3 lines — this pushes many rows to scrollback, evicting some.
    t.resize(3, 80, true);

    // If the image's row was evicted, placement should be pruned.
    let evicted = t.grid().total_evicted();
    if evicted > 0 {
        // The row-0 image was at StableRowIndex(0) which is now evicted.
        assert_eq!(t.image_cache().placement_count(), 0);
    }
}

// DECSET/DECRST mode sync: verify `named_private_mode_flag` agrees with
// `apply_decset`/`apply_decrst` for every `NamedPrivateMode` variant.

#[test]
fn decset_decrst_flag_sync() {
    use vte::ansi::NamedPrivateMode;

    use super::super::helpers::named_private_mode_flag;
    use crate::term::TermMode;

    // All variants that map to a simple flag (no side-effects beyond
    // `mode.insert`/`mode.remove`). Alt screen and SaveCursor variants
    // have side effects (grid swaps, cursor save/restore) and are tested
    // separately via VTE sequence tests.
    let flag_variants = [
        NamedPrivateMode::CursorKeys,
        NamedPrivateMode::Origin,
        NamedPrivateMode::LineWrap,
        NamedPrivateMode::BlinkingCursor,
        NamedPrivateMode::ShowCursor,
        NamedPrivateMode::ReverseWraparound,
        NamedPrivateMode::X10Mouse,
        NamedPrivateMode::ReportMouseClicks,
        NamedPrivateMode::ReportCellMouseMotion,
        NamedPrivateMode::ReportAllMouseMotion,
        NamedPrivateMode::ReportFocusInOut,
        NamedPrivateMode::Utf8Mouse,
        NamedPrivateMode::SgrMouse,
        NamedPrivateMode::UrxvtMouse,
        NamedPrivateMode::UrgencyHints,
        NamedPrivateMode::BracketedPaste,
        NamedPrivateMode::SyncUpdate,
        NamedPrivateMode::AlternateScroll,
        NamedPrivateMode::SixelScrolling,
        NamedPrivateMode::SixelCursorRight,
        NamedPrivateMode::Win32Input,
        NamedPrivateMode::LeftRightMargin,
    ];

    for variant in flag_variants {
        let flag = named_private_mode_flag(variant)
            .unwrap_or_else(|| panic!("{variant:?}: named_private_mode_flag returned None"));

        // Start with the flag cleared so we can verify DECSET sets it.
        let mut t = term();
        t.mode.remove(flag);

        t.apply_decset(variant);
        assert!(
            t.mode().contains(flag),
            "{variant:?}: flag not set after apply_decset"
        );

        // apply_decrst should clear the flag.
        t.apply_decrst(variant);
        assert!(
            !t.mode().contains(flag),
            "{variant:?}: flag not cleared after apply_decrst"
        );
    }

    // Variants that return None must be handled without panic.
    let none_variants = [NamedPrivateMode::SaveCursor, NamedPrivateMode::ColumnMode];
    for variant in none_variants {
        assert!(
            named_private_mode_flag(variant).is_none(),
            "{variant:?}: expected None from named_private_mode_flag"
        );
    }

    // Alt screen variants return Some(ALT_SCREEN) but have side effects.
    // Just verify the flag mapping is correct.
    let alt_variants = [
        NamedPrivateMode::AltScreen,
        NamedPrivateMode::AltScreenOpt,
        NamedPrivateMode::SwapScreenAndSetRestoreCursor,
    ];
    for variant in alt_variants {
        let flag = named_private_mode_flag(variant);
        assert_eq!(
            flag,
            Some(TermMode::ALT_SCREEN),
            "{variant:?}: expected ALT_SCREEN flag"
        );
    }
}
