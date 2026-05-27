use std::sync::Arc;

use vte::ansi::{Color, NamedColor};

use super::{Cell, CellExtra, CellFlags, Hyperlink, OPAQUE_ALPHA};

#[test]
fn size_assertion() {
    assert!(
        size_of::<Cell>() <= 24,
        "Cell is {} bytes, expected <= 24",
        size_of::<Cell>()
    );
}

#[test]
fn default_cell_is_space_with_default_colors() {
    let cell = Cell::default();
    assert_eq!(cell.ch, ' ');
    assert_eq!(cell.fg, Color::Named(NamedColor::Foreground));
    assert_eq!(cell.bg, Color::Named(NamedColor::Background));
    assert!(cell.flags.is_empty());
    assert!(cell.extra.is_none());
}

#[test]
fn reset_clears_to_template() {
    let mut cell = Cell::default();
    cell.ch = 'X';
    cell.flags = CellFlags::BOLD;

    let template = Cell::default();
    cell.reset(&template);

    assert_eq!(cell.ch, ' ');
    assert!(cell.flags.is_empty());
}

#[test]
fn is_empty_for_default() {
    assert!(Cell::default().is_empty());
}

#[test]
fn is_empty_false_after_setting_char() {
    let mut cell = Cell::default();
    cell.ch = 'A';
    assert!(!cell.is_empty());
}

/// Regression:. `CellFlags::DRAWN` must default clear so
/// pristine cells are correctly distinguished from application-written
/// ones by `compute_rect_checksum`.
#[test]
fn default_cell_has_drawn_clear() {
    assert!(!Cell::default().flags.contains(CellFlags::DRAWN));
}

/// Regression: TPR F1. `is_empty()` stays orthogonal
/// to DRAWN — it answers "visually empty", NOT "never written". A
/// cell that was written by the application as a plain space under
/// default SGR still LOOKS empty (same pixels as a pristine cell),
/// so `is_empty()` MUST return true. Consumers that need the "never
/// written" semantic query `flags.contains(CellFlags::DRAWN)` directly.
///
/// This orthogonality preserves the pre-fix behavior of `Row::is_blank`
/// and `Row::content_len` — used by reflow at
/// `oriterm_core/src/grid/resize/mod.rs:222` and `:407` — so existing
/// reflow semantics are unchanged by the CHARDRAWN infrastructure.
#[test]
fn is_empty_stays_true_when_only_drawn_is_set() {
    let mut cell = Cell::default();
    cell.flags.insert(CellFlags::DRAWN);
    assert!(
        cell.is_empty(),
        "a drawn-but-visually-empty cell MUST still be is_empty \
         (DRAWN is orthogonal to visual emptiness)"
    );
}

/// Regression: TPR F1. A cell with DRAWN + any other
/// flag (SGR or structural) is NOT is_empty — the non-DRAWN flag bit
/// is what makes it visually non-empty, not DRAWN itself.
#[test]
fn is_empty_false_when_drawn_plus_sgr_flag() {
    let mut cell = Cell::default();
    cell.flags.insert(CellFlags::DRAWN | CellFlags::BOLD);
    assert!(
        !cell.is_empty(),
        "DRAWN + BOLD: BOLD is a visible attribute so is_empty is false"
    );
}

/// Regression:. `Cell::reset(&template)` copies flags from
/// the template wholesale. A DRAWN-clear template must produce a
/// DRAWN-clear cell, so every reset path automatically clears DRAWN
/// without explicit instrumentation.
#[test]
fn cell_reset_from_default_template_clears_drawn() {
    let mut cell = Cell::default();
    cell.flags = CellFlags::DRAWN | CellFlags::BOLD;
    cell.reset(&Cell::default());
    assert!(!cell.flags.contains(CellFlags::DRAWN));
}

/// Regression:. BCE template (`Cell::from(bg)`) must not
/// carry DRAWN either — BCE-erased cells are NOT drawn per xterm.
#[test]
fn bce_template_has_drawn_clear() {
    let template = Cell::from(Color::Named(NamedColor::Red));
    assert!(!template.flags.contains(CellFlags::DRAWN));
}

/// Regression:. `CellFlags::INTERNAL_CELL_STATE` is the
/// union of bits that must never appear on `cursor.template.flags`.
/// Every structural / write-history bit belongs here; SGR attrs stay
/// out. Pin the exact membership so drift at the definition site trips
/// this test.
#[test]
fn internal_cell_state_union_pins_exact_membership() {
    let expected = CellFlags::DRAWN
        | CellFlags::WRAP
        | CellFlags::WIDE_CHAR
        | CellFlags::WIDE_CHAR_SPACER
        | CellFlags::LEADING_WIDE_CHAR_SPACER;
    assert_eq!(CellFlags::INTERNAL_CELL_STATE, expected);

    // Regression guard: SGR attributes MUST NOT be in the internal set.
    assert!(!CellFlags::INTERNAL_CELL_STATE.contains(CellFlags::BOLD));
    assert!(!CellFlags::INTERNAL_CELL_STATE.contains(CellFlags::UNDERLINE));
    assert!(!CellFlags::INTERNAL_CELL_STATE.contains(CellFlags::INVERSE));
    assert!(!CellFlags::INTERNAL_CELL_STATE.contains(CellFlags::HIDDEN));
    assert!(!CellFlags::INTERNAL_CELL_STATE.contains(CellFlags::BLINK));
}

/// Regression:. Size invariant MUST hold after adding DRAWN.
/// The existing `size_assertion` test covers the same property, but
/// this test explicitly cross-references the bug so future additions
/// to CellFlags (e.g. future §09A.8 PROTECTED bit) re-check it with
/// the right framing.
#[test]
fn cell_size_survives_drawn_addition() {
    assert!(size_of::<Cell>() <= 24);
}

#[test]
fn wide_char_width() {
    let mut cell = Cell::default();
    cell.ch = '\u{597d}';
    cell.flags = CellFlags::WIDE_CHAR;
    assert_eq!(cell.width(), 2);
}

#[test]
fn spacer_width() {
    let mut cell = Cell::default();
    cell.flags = CellFlags::WIDE_CHAR_SPACER;
    assert_eq!(cell.width(), 0);
}

#[test]
fn normal_char_width() {
    let mut cell = Cell::default();
    cell.ch = 'A';
    assert_eq!(cell.width(), 1);
}

#[test]
fn extra_is_none_for_normal_cells() {
    let cell = Cell::default();
    assert!(cell.extra.is_none());
}

#[test]
fn extra_created_for_underline_color() {
    let mut cell = Cell::default();
    cell.extra = Some(Arc::new(CellExtra {
        underline_color: Some(Color::Spec(vte::ansi::Rgb { r: 255, g: 0, b: 0 })),
        hyperlink: None,
        zerowidth: Vec::new(),
        ..Default::default()
    }));
    assert!(cell.extra.is_some());
    assert_eq!(
        cell.extra.as_ref().unwrap().underline_color,
        Some(Color::Spec(vte::ansi::Rgb { r: 255, g: 0, b: 0 }))
    );
}

#[test]
fn extra_created_for_hyperlink() {
    let mut cell = Cell::default();
    cell.extra = Some(Arc::new(CellExtra {
        underline_color: None,
        hyperlink: Some(Hyperlink {
            id: None,
            uri: "https://example.com".to_string(),
        }),
        zerowidth: Vec::new(),
        ..Default::default()
    }));
    assert!(cell.extra.is_some());
}

#[test]
fn push_zerowidth_creates_extra() {
    let mut cell = Cell::default();
    assert!(cell.extra.is_none());

    // U+0301 COMBINING ACUTE ACCENT.
    cell.push_zerowidth('\u{0301}');

    assert!(cell.extra.is_some());
    assert_eq!(cell.extra.as_ref().unwrap().zerowidth, vec!['\u{0301}']);
}

#[test]
fn cellflags_set_clear_query() {
    let mut flags = CellFlags::empty();
    assert!(!flags.contains(CellFlags::BOLD));

    flags |= CellFlags::BOLD;
    assert!(flags.contains(CellFlags::BOLD));

    flags &= !CellFlags::BOLD;
    assert!(!flags.contains(CellFlags::BOLD));
}

#[test]
fn cellflags_combine() {
    let flags = CellFlags::BOLD | CellFlags::ITALIC | CellFlags::UNDERLINE;
    assert!(flags.contains(CellFlags::BOLD));
    assert!(flags.contains(CellFlags::ITALIC));
    assert!(flags.contains(CellFlags::UNDERLINE));
    assert!(!flags.contains(CellFlags::DIM));
}

// --- Additional tests from reference repo gap analysis ---

#[test]
fn from_color_creates_bce_cell() {
    let color = Color::Indexed(1);
    let cell = Cell::from(color);
    assert_eq!(cell.ch, ' ');
    assert_eq!(cell.bg, color);
    assert_eq!(cell.fg, Color::Named(NamedColor::Foreground));
    assert!(cell.flags.is_empty());
    assert!(cell.extra.is_none());
}

#[test]
fn is_empty_false_for_non_default_bg() {
    let mut cell = Cell::default();
    cell.bg = Color::Indexed(1);
    assert!(!cell.is_empty());
}

#[test]
fn is_empty_false_for_flags() {
    let mut cell = Cell::default();
    cell.flags = CellFlags::BOLD;
    assert!(!cell.is_empty());
}

#[test]
fn is_empty_false_for_extra() {
    let mut cell = Cell::default();
    cell.push_zerowidth('\u{0301}');
    assert!(!cell.is_empty());
}

#[test]
fn width_cjk_ideographic_space() {
    // U+3000 IDEOGRAPHIC SPACE — width 2 (wezterm issue_1161).
    let mut cell = Cell::default();
    cell.ch = '\u{3000}';
    cell.flags = CellFlags::WIDE_CHAR;
    assert_eq!(cell.width(), 2);
}

#[test]
fn width_emoji() {
    // Emoji crab — width 2 via unicode-width when WIDE_CHAR flag set.
    let mut cell = Cell::default();
    cell.ch = '\u{1f980}';
    cell.flags = CellFlags::WIDE_CHAR;
    assert_eq!(cell.width(), 2);
}

#[test]
fn push_zerowidth_multiple_marks() {
    let mut cell = Cell::default();
    cell.ch = 'e';
    cell.push_zerowidth('\u{0301}'); // COMBINING ACUTE ACCENT
    cell.push_zerowidth('\u{0327}'); // COMBINING CEDILLA
    let zw = &cell.extra.as_ref().unwrap().zerowidth;
    assert_eq!(zw.len(), 2);
    assert_eq!(zw[0], '\u{0301}');
    assert_eq!(zw[1], '\u{0327}');
}

/// `CellExtra.zerowidth` is `Vec<char>` — UNBOUNDED storage. Pin the
/// invariant by feeding 64 combining marks and asserting all 64 round-trip
/// without panic or silent truncation. The kitty unicode-placeholder
/// (`U=1`) protocol writes up to 3 diacritics per cell (row, column, MSB);
/// a future cap below the kitty minimum would corrupt the decoded lookup
/// tuple. Companion spec-coverage pin:
/// `kitty_placeholder_max_diacritic_count_round_trips_through_grid` at
/// `oriterm_core/tests/spec_chain/kitty/placeholder_anchors.rs`.
#[test]
fn cell_extra_zerowidth_storage_remains_unbounded() {
    let mut cell = Cell::default();
    cell.ch = 'a';
    // 64 distinct combining marks from the General Combining range
    // (U+0300..U+036F — 112 codepoints, all single-`char`).
    let marks: Vec<char> = (0..64u32)
        .map(|i| char::from_u32(0x0300 + i).expect("combining range fits in BMP"))
        .collect();
    for &m in &marks {
        cell.push_zerowidth(m);
    }
    let stored = &cell
        .extra
        .as_ref()
        .expect("extra allocated on first push")
        .zerowidth;
    assert_eq!(
        stored.len(),
        64,
        "Vec<char> MUST store all 64 combining marks without truncation"
    );
    assert_eq!(
        stored.as_slice(),
        marks.as_slice(),
        "stored marks MUST preserve push order — silent truncation or \
         reordering breaks kitty U=1 placeholder decode"
    );
}

#[test]
fn clone_shares_arc_refcount() {
    let mut cell = Cell::default();
    cell.push_zerowidth('\u{0301}');
    let cloned = cell.clone();
    // Both cells should share the same Arc allocation.
    assert!(Arc::ptr_eq(
        cell.extra.as_ref().unwrap(),
        cloned.extra.as_ref().unwrap()
    ));
}

#[test]
fn push_zerowidth_cow_on_shared_arc() {
    let mut cell = Cell::default();
    cell.push_zerowidth('\u{0301}');
    let original = cell.clone();
    // Mutating cell's extra triggers COW — original stays unchanged.
    cell.push_zerowidth('\u{0327}');
    assert_eq!(original.extra.as_ref().unwrap().zerowidth.len(), 1);
    assert_eq!(cell.extra.as_ref().unwrap().zerowidth.len(), 2);
}

#[test]
fn reset_copies_template_extra() {
    let mut cell = Cell::default();
    cell.ch = 'X';
    let mut template = Cell::default();
    template.extra = Some(Arc::new(CellExtra {
        underline_color: Some(Color::Spec(vte::ansi::Rgb { r: 0, g: 255, b: 0 })),
        hyperlink: None,
        zerowidth: Vec::new(),
        ..Default::default()
    }));
    cell.reset(&template);
    assert!(cell.extra.is_some());
    assert_eq!(
        cell.extra.as_ref().unwrap().underline_color,
        Some(Color::Spec(vte::ansi::Rgb { r: 0, g: 255, b: 0 }))
    );
}

#[test]
fn reset_clears_extra_when_template_has_none() {
    let mut cell = Cell::default();
    cell.push_zerowidth('\u{0301}');
    assert!(cell.extra.is_some());
    cell.reset(&Cell::default());
    assert!(cell.extra.is_none());
}

#[test]
fn hyperlink_display() {
    let link = Hyperlink {
        id: Some("id1".to_string()),
        uri: "https://example.com".to_string(),
    };
    assert_eq!(format!("{link}"), "https://example.com");
}

// --- SGR mode-6 concrete per-channel alpha (Decision 08 Option C) ---

/// A pristine cell is fully opaque on every channel and carries NO sidecar
/// and NO `HAS_ALPHA` bit — opaque cells must never allocate.
#[test]
fn default_cell_alpha_channels_opaque() {
    let cell = Cell::default();
    assert_eq!(cell.fg_alpha(), OPAQUE_ALPHA);
    assert_eq!(cell.bg_alpha(), OPAQUE_ALPHA);
    assert_eq!(cell.underline_alpha(), OPAQUE_ALPHA);
    assert!(
        cell.extra.is_none(),
        "opaque cell must not allocate a sidecar"
    );
    assert!(!cell.flags.contains(CellFlags::HAS_ALPHA));
}

/// Setting a channel below 255 stores the concrete byte, allocates the
/// sidecar, and raises the `HAS_ALPHA` fast-skip bit; the stored 128
/// round-trips back out through the sidecar accessor.
#[test]
fn set_bg_alpha_translucent_stores_and_flags() {
    let mut cell = Cell::default();
    cell.set_bg_alpha(128);
    assert_eq!(cell.bg_alpha(), 128);
    assert!(cell.extra.is_some(), "translucent cell allocates a sidecar");
    assert!(cell.flags.contains(CellFlags::HAS_ALPHA));
    // Other channels stay opaque.
    assert_eq!(cell.fg_alpha(), OPAQUE_ALPHA);
    assert_eq!(cell.underline_alpha(), OPAQUE_ALPHA);
}

/// Setting an already-opaque channel to 255 on a clean cell is a no-op:
/// never allocates, never raises `HAS_ALPHA`.
#[test]
fn set_opaque_alpha_on_clean_cell_is_noop() {
    let mut cell = Cell::default();
    cell.set_fg_alpha(OPAQUE_ALPHA);
    cell.set_bg_alpha(OPAQUE_ALPHA);
    cell.set_underline_alpha(OPAQUE_ALPHA);
    assert!(cell.extra.is_none(), "opaque set must not allocate");
    assert!(!cell.flags.contains(CellFlags::HAS_ALPHA));
}

/// Round-trip: translucent → opaque drops the sidecar AND clears the
/// `HAS_ALPHA` bit when nothing else lives in the sidecar — guards
/// against a stale flag/sidecar after the channel returns to opaque.
#[test]
fn set_alpha_back_to_opaque_drops_sidecar_and_flag() {
    let mut cell = Cell::default();
    cell.set_bg_alpha(64);
    assert!(cell.flags.contains(CellFlags::HAS_ALPHA));
    cell.set_bg_alpha(OPAQUE_ALPHA);
    assert_eq!(cell.bg_alpha(), OPAQUE_ALPHA);
    assert!(
        cell.extra.is_none(),
        "sidecar must drop when all channels return to opaque and nothing else lives in it"
    );
    assert!(!cell.flags.contains(CellFlags::HAS_ALPHA));
}

/// When the sidecar carries other data (underline color), returning alpha
/// to opaque keeps the sidecar but still clears `HAS_ALPHA`.
#[test]
fn opaque_alpha_keeps_sidecar_when_other_data_present() {
    let mut cell = Cell::default();
    cell.set_underline_color(Some(Color::Spec(vte::ansi::Rgb { r: 1, g: 2, b: 3 })));
    cell.set_fg_alpha(200);
    assert!(cell.flags.contains(CellFlags::HAS_ALPHA));
    cell.set_fg_alpha(OPAQUE_ALPHA);
    assert!(
        cell.extra.is_some(),
        "sidecar must survive while underline color is present"
    );
    assert!(!cell.flags.contains(CellFlags::HAS_ALPHA));
    assert_eq!(cell.fg_alpha(), OPAQUE_ALPHA);
}

/// `HAS_ALPHA` reflects "any channel non-opaque" across all three channels.
#[test]
fn has_alpha_reflects_any_channel() {
    let channels: [fn(&mut Cell, u8); 3] = [
        Cell::set_fg_alpha,
        Cell::set_bg_alpha,
        Cell::set_underline_alpha,
    ];
    let mut count = 0;
    for set in channels {
        let mut cell = Cell::default();
        set(&mut cell, 100);
        assert!(
            cell.flags.contains(CellFlags::HAS_ALPHA),
            "any single non-opaque channel must raise HAS_ALPHA"
        );
        set(&mut cell, OPAQUE_ALPHA);
        assert!(
            !cell.flags.contains(CellFlags::HAS_ALPHA),
            "returning the channel to opaque must clear HAS_ALPHA"
        );
        count += 1;
    }
    assert_eq!(count, 3, "every alpha channel must be exercised");
}

/// `CellExtra::is_default()` pins the exact droppable condition: the SSOT
/// every sidecar mutator consults.
#[test]
fn cell_extra_is_default_membership() {
    assert!(CellExtra::new().is_default());

    let mut e = CellExtra::new();
    e.fg_alpha = 200;
    assert!(!e.is_default(), "non-opaque fg alpha is not default");

    let mut e = CellExtra::new();
    e.underline_color = Some(Color::Spec(vte::ansi::Rgb { r: 1, g: 2, b: 3 }));
    assert!(!e.is_default(), "underline color is not default");

    let mut e = CellExtra::new();
    e.zerowidth.push('\u{0301}');
    assert!(!e.is_default(), "combining mark is not default");
}

/// Alpha mutation on a shared `Arc` triggers copy-on-write — the clone
/// keeps the prior alpha.
#[test]
fn set_alpha_cow_on_shared_arc() {
    let mut cell = Cell::default();
    cell.set_bg_alpha(128);
    let original = cell.clone();
    cell.set_bg_alpha(64);
    assert_eq!(original.bg_alpha(), 128);
    assert_eq!(cell.bg_alpha(), 64);
}

/// The `size_of::<Cell>() <= 24` invariant survives alpha storage:
/// Decision 08 Option C puts concrete alpha in the heap `CellExtra`,
/// adding ZERO bytes to `Cell`. Reverting to a `Cell`-resident alpha
/// field (Option A) would fail this and the top-level `size_assertion`.
#[test]
fn cell_size_survives_alpha_addition() {
    assert!(
        size_of::<Cell>() <= 24,
        "Cell is {} bytes — alpha must live in CellExtra, not Cell",
        size_of::<Cell>()
    );
}
