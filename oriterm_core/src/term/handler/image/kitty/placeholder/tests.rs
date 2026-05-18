//! Unit tests for the kitty unicode-placeholder encoding module.

use vte::ansi::{Color, Rgb};

use super::{
    DIACRITICS, IncompletePlacement, ResolvedPlaceholder, color_to_id, diacritic_to_index,
};

#[test]
fn diacritic_table_is_sorted() {
    for pair in DIACRITICS.windows(2) {
        assert!(
            pair[0] < pair[1],
            "DIACRITICS not strictly ascending at {:?} > {:?}",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn diacritic_table_has_exactly_297_entries() {
    assert_eq!(DIACRITICS.len(), 297);
}

#[test]
fn diacritic_to_index_returns_some_for_valid_codepoints() {
    assert_eq!(diacritic_to_index('\u{0305}'), Some(0));
    assert_eq!(diacritic_to_index('\u{030D}'), Some(1));
    assert_eq!(diacritic_to_index('\u{030E}'), Some(2));
    assert_eq!(diacritic_to_index('\u{0483}'), Some(30));
    assert_eq!(diacritic_to_index('\u{1D242}'), Some(294));
    assert_eq!(diacritic_to_index('\u{1D243}'), Some(295));
    assert_eq!(diacritic_to_index('\u{1D244}'), Some(296));
}

#[test]
fn diacritic_to_index_returns_none_for_invalid_codepoints() {
    // ASCII, regular letters, punctuation, emoji — anything not in the
    // diacritic table.
    assert_eq!(diacritic_to_index('a'), None);
    assert_eq!(diacritic_to_index(' '), None);
    assert_eq!(diacritic_to_index('1'), None);
    assert_eq!(diacritic_to_index('\u{10EEEE}'), None);
    assert_eq!(diacritic_to_index('\u{0300}'), None); // combining grave — not in kitty's table
    assert_eq!(diacritic_to_index('\u{2603}'), None); // snowman
}

#[test]
fn color_to_id_named_returns_zero() {
    use vte::ansi::NamedColor;
    assert_eq!(color_to_id(Color::Named(NamedColor::Foreground)), 0);
    assert_eq!(color_to_id(Color::Named(NamedColor::Background)), 0);
    assert_eq!(color_to_id(Color::Named(NamedColor::Red)), 0);
}

#[test]
fn color_to_id_indexed_returns_palette_value() {
    assert_eq!(color_to_id(Color::Indexed(0)), 0);
    assert_eq!(color_to_id(Color::Indexed(42)), 42);
    assert_eq!(color_to_id(Color::Indexed(255)), 255);
}

#[test]
fn color_to_id_spec_returns_24bit_rgb() {
    assert_eq!(
        color_to_id(Color::Spec(Rgb {
            r: 0xAB,
            g: 0xCD,
            b: 0xEF
        })),
        0x00AB_CDEF
    );
    assert_eq!(
        color_to_id(Color::Spec(Rgb {
            r: 0xFF,
            g: 0,
            b: 0
        })),
        0x00FF_0000
    );
    assert_eq!(
        color_to_id(Color::Spec(Rgb {
            r: 0,
            g: 0,
            b: 0xFF
        })),
        0x0000_00FF
    );
    assert_eq!(color_to_id(Color::Spec(Rgb { r: 0, g: 0, b: 0 })), 0);
}

#[test]
fn decode_simple_row_col() {
    let zerowidth = ['\u{0305}', '\u{0305}'];
    let p = IncompletePlacement::decode(&zerowidth, Color::Indexed(42), None);
    assert_eq!(p.image_id_low, 42);
    assert_eq!(p.row, Some(0));
    assert_eq!(p.col, Some(0));
    assert_eq!(p.image_id_high, None);
    assert_eq!(p.placement_id, None);
}

#[test]
fn decode_with_high_byte() {
    // 3rd diacritic is the image_id high byte (here, index 5 → high = 5).
    let zerowidth = ['\u{0305}', '\u{030D}', '\u{033D}'];
    let p = IncompletePlacement::decode(&zerowidth, Color::Indexed(1), None);
    assert_eq!(p.row, Some(0));
    assert_eq!(p.col, Some(1));
    assert_eq!(p.image_id_high, Some(5));
}

#[test]
fn decode_no_diacritics_returns_none_fields() {
    let p = IncompletePlacement::decode(&[], Color::Indexed(0), None);
    assert_eq!(p.row, None);
    assert_eq!(p.col, None);
    assert_eq!(p.image_id_high, None);
    assert_eq!(p.placement_id, None);
    assert_eq!(p.image_id_low, 0);
}

#[test]
fn decode_invalid_first_diacritic() {
    let zerowidth = ['a', '\u{0305}'];
    let p = IncompletePlacement::decode(&zerowidth, Color::Indexed(7), None);
    assert_eq!(p.row, None);
    // Invalid row diacritic does NOT cascade — col still decodes if its
    // diacritic is valid (matches ghostty's "skip invalid" behavior).
    assert_eq!(p.col, Some(0));
    assert_eq!(p.image_id_low, 7);
}

#[test]
fn decode_invalid_high_byte() {
    // 3rd diacritic position holds an invalid codepoint — high byte stays None.
    let zerowidth = ['\u{0305}', '\u{0305}', 'x'];
    let p = IncompletePlacement::decode(&zerowidth, Color::Indexed(1), None);
    assert_eq!(p.row, Some(0));
    assert_eq!(p.col, Some(0));
    assert_eq!(p.image_id_high, None);
}

#[test]
fn decode_with_placement_id_from_underline_color() {
    let p = IncompletePlacement::decode(
        &['\u{0305}', '\u{0305}'],
        Color::Indexed(1),
        Some(Color::Indexed(7)),
    );
    assert_eq!(p.placement_id, Some(7));
}

#[test]
fn decode_zero_underline_color_yields_no_placement_id() {
    let p = IncompletePlacement::decode(
        &['\u{0305}', '\u{0305}'],
        Color::Indexed(1),
        Some(Color::Indexed(0)),
    );
    assert_eq!(p.placement_id, None);
}

#[test]
fn resolve_no_prev_uses_defaults() {
    let p = IncompletePlacement::decode(&[], Color::Indexed(0), None);
    let r = p.resolve_with_continuation(None);
    assert_eq!(r.image_id, 0);
    assert_eq!(r.image_row, 0);
    assert_eq!(r.image_col, 0);
    assert_eq!(r.placement_id, 0);
}

#[test]
fn resolve_explicit_row_col_overrides_continuation() {
    let prev = ResolvedPlaceholder {
        image_id: 1,
        placement_id: 0,
        image_row: 0,
        image_col: 5,
        image_id_low: 1,
        image_id_high: None,
    };
    let p = IncompletePlacement::decode(&['\u{030D}', '\u{030E}'], Color::Indexed(1), None);
    let r = p.resolve_with_continuation(Some(&prev));
    assert_eq!(r.image_row, 1);
    assert_eq!(r.image_col, 2);
}

#[test]
fn resolve_inherits_row_from_prev_when_no_diacritics() {
    // kitty docs: no diacritics on a continuation cell inherits row/col+1/high
    // from the left cell when fg+ul match.
    let prev = ResolvedPlaceholder {
        image_id: 42,
        placement_id: 0,
        image_row: 3,
        image_col: 7,
        image_id_low: 42,
        image_id_high: None,
    };
    let p = IncompletePlacement::decode(&[], Color::Indexed(42), None);
    let r = p.resolve_with_continuation(Some(&prev));
    assert_eq!(r.image_row, 3);
    assert_eq!(r.image_col, 8);
    assert_eq!(r.image_id, 42);
}

#[test]
fn resolve_inherits_col_plus_one_when_row_only() {
    // Row diacritic only → col=prev.col+1, high inherited.
    let prev = ResolvedPlaceholder {
        image_id: 1,
        placement_id: 0,
        image_row: 0,
        image_col: 0,
        image_id_low: 1,
        image_id_high: None,
    };
    let p = IncompletePlacement::decode(&['\u{0305}'], Color::Indexed(1), None);
    let r = p.resolve_with_continuation(Some(&prev));
    assert_eq!(r.image_row, 0);
    assert_eq!(r.image_col, 1);
}

#[test]
fn resolve_inherits_high_when_row_col_only() {
    // Row+col only → image_id high inherited from prev.
    let prev = ResolvedPlaceholder {
        image_id: 1 | (5 << 24),
        placement_id: 0,
        image_row: 0,
        image_col: 0,
        image_id_low: 1,
        image_id_high: Some(5),
    };
    let p = IncompletePlacement::decode(&['\u{0305}', '\u{030D}'], Color::Indexed(1), None);
    let r = p.resolve_with_continuation(Some(&prev));
    assert_eq!(r.image_id, 1 | (5 << 24));
    assert_eq!(r.image_id_high, Some(5));
}

#[test]
fn resolve_breaks_continuation_when_fg_differs() {
    // Different fg color → no inheritance, col defaults to 0.
    let prev = ResolvedPlaceholder {
        image_id: 1,
        placement_id: 0,
        image_row: 0,
        image_col: 5,
        image_id_low: 1,
        image_id_high: None,
    };
    let p = IncompletePlacement::decode(&[], Color::Indexed(2), None);
    let r = p.resolve_with_continuation(Some(&prev));
    assert_eq!(r.image_id, 2);
    assert_eq!(r.image_row, 0);
    assert_eq!(r.image_col, 0);
}

#[test]
fn resolve_breaks_continuation_when_ul_differs() {
    let prev = ResolvedPlaceholder {
        image_id: 1,
        placement_id: 0, // no placement_id
        image_row: 0,
        image_col: 5,
        image_id_low: 1,
        image_id_high: None,
    };
    let p = IncompletePlacement::decode(
        &[],
        Color::Indexed(1),
        Some(Color::Indexed(9)), // placement_id = 9
    );
    let r = p.resolve_with_continuation(Some(&prev));
    // placement_id mismatch breaks continuation.
    assert_eq!(r.placement_id, 9);
    assert_eq!(r.image_col, 0);
}

#[test]
fn resolve_assembles_image_id_from_low_and_high() {
    // Sanity check: when the high byte is set on the current cell, it
    // contributes to the assembled image_id regardless of continuation.
    let p = IncompletePlacement {
        image_id_low: 0x00_1234,
        image_id_high: Some(0xAB),
        placement_id: None,
        row: Some(0),
        col: Some(0),
    };
    let r = p.resolve_with_continuation(None);
    assert_eq!(r.image_id, 0x00_1234 | (0xAB << 24));
}
