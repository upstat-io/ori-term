use std::sync::Arc;

use vte::ansi::{Color, NamedColor};

use super::{Cell, CellExtra, CellFlags, Hyperlink};

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

/// Regression: BUG-08-17. `CellFlags::DRAWN` must default clear so
/// pristine cells are correctly distinguished from application-written
/// ones by `compute_rect_checksum`.
#[test]
fn default_cell_has_drawn_clear() {
    assert!(!Cell::default().flags.contains(CellFlags::DRAWN));
}

/// Regression: BUG-08-17. `is_empty()` stays orthogonal to DRAWN — it
/// answers "visually empty", not "never written". A drawn-but-visually-
/// empty cell (application wrote a plain space) MUST NOT be `is_empty()`
/// because it carries the DRAWN bit in its flags.
#[test]
fn is_empty_sees_drawn_as_non_empty() {
    let mut cell = Cell::default();
    cell.flags.insert(CellFlags::DRAWN);
    assert!(
        !cell.is_empty(),
        "a drawn cell with default ch/fg/bg must NOT be is_empty"
    );
}

/// Regression: BUG-08-17. `Cell::reset(&template)` copies flags from
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

/// Regression: BUG-08-17. BCE template (`Cell::from(bg)`) must not
/// carry DRAWN either — BCE-erased cells are NOT drawn per xterm.
#[test]
fn bce_template_has_drawn_clear() {
    let template = Cell::from(Color::Named(NamedColor::Red));
    assert!(!template.flags.contains(CellFlags::DRAWN));
}

/// Regression: BUG-08-17. `CellFlags::INTERNAL_CELL_STATE` is the
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

    // Negative pin: SGR attributes MUST NOT be in the internal set.
    assert!(!CellFlags::INTERNAL_CELL_STATE.contains(CellFlags::BOLD));
    assert!(!CellFlags::INTERNAL_CELL_STATE.contains(CellFlags::UNDERLINE));
    assert!(!CellFlags::INTERNAL_CELL_STATE.contains(CellFlags::INVERSE));
    assert!(!CellFlags::INTERNAL_CELL_STATE.contains(CellFlags::HIDDEN));
    assert!(!CellFlags::INTERNAL_CELL_STATE.contains(CellFlags::BLINK));
}

/// Regression: BUG-08-17. Size invariant MUST hold after adding DRAWN.
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
