use crate::font::FaceIdx;

use super::{CodepointMap, parse_hex_range};

// ── Lookup ──

#[test]
fn empty_map_returns_none() {
    let map = CodepointMap::new();
    assert!(map.is_empty());
    assert_eq!(map.lookup(0x41), None);
}

#[test]
fn single_entry_hit() {
    let mut map = CodepointMap::new();
    map.add(0xE000, 0xF8FF, FaceIdx(4));
    assert_eq!(map.lookup(0xE000), Some(FaceIdx(4)));
    assert_eq!(map.lookup(0xE500), Some(FaceIdx(4)));
    assert_eq!(map.lookup(0xF8FF), Some(FaceIdx(4)));
}

#[test]
fn single_entry_miss() {
    let mut map = CodepointMap::new();
    map.add(0xE000, 0xF8FF, FaceIdx(4));
    assert_eq!(map.lookup(0xDFFF), None);
    assert_eq!(map.lookup(0xF900), None);
    assert_eq!(map.lookup(0x41), None);
}

#[test]
fn single_codepoint_range() {
    let mut map = CodepointMap::new();
    map.add(0xE0B0, 0xE0B0, FaceIdx(5));
    assert_eq!(map.lookup(0xE0B0), Some(FaceIdx(5)));
    assert_eq!(map.lookup(0xE0AF), None);
    assert_eq!(map.lookup(0xE0B1), None);
}

#[test]
fn multiple_disjoint_ranges() {
    let mut map = CodepointMap::new();
    map.add(0xE000, 0xF8FF, FaceIdx(4)); // PUA
    map.add(0x4E00, 0x9FFF, FaceIdx(5)); // CJK
    map.add(0x1F600, 0x1F64F, FaceIdx(6)); // Emoticons

    // PUA range.
    assert_eq!(map.lookup(0xE100), Some(FaceIdx(4)));
    // CJK range.
    assert_eq!(map.lookup(0x5000), Some(FaceIdx(5)));
    // Emoticons range.
    assert_eq!(map.lookup(0x1F600), Some(FaceIdx(6)));
    // Gaps between ranges.
    assert_eq!(map.lookup(0x41), None);
    assert_eq!(map.lookup(0xA000), None);
    assert_eq!(map.lookup(0x10000), None);
}

#[test]
fn overlapping_ranges_largest_start_wins() {
    let mut map = CodepointMap::new();
    map.add(0xE000, 0xF8FF, FaceIdx(4)); // Broad PUA
    map.add(0xE0B0, 0xE0B6, FaceIdx(5)); // Narrow powerline subset

    // Inside narrow range: narrow wins (larger start).
    assert_eq!(map.lookup(0xE0B0), Some(FaceIdx(5)));
    assert_eq!(map.lookup(0xE0B3), Some(FaceIdx(5)));
    // Outside narrow but inside broad.
    assert_eq!(map.lookup(0xE000), Some(FaceIdx(4)));
    assert_eq!(map.lookup(0xE0AF), Some(FaceIdx(4)));
    assert_eq!(map.lookup(0xE0B7), Some(FaceIdx(4)));
}

#[test]
fn same_codepoint_override_last_writer_wins() {
    let mut map = CodepointMap::new();
    map.add(0xE0B0, 0xE0B0, FaceIdx(4));
    assert_eq!(map.lookup(0xE0B0), Some(FaceIdx(4)));
    // Second add for the same codepoint overrides.
    map.add(0xE0B0, 0xE0B0, FaceIdx(5));
    assert_eq!(map.lookup(0xE0B0), Some(FaceIdx(5)));
}

#[test]
fn adjacent_ranges_no_gap() {
    let mut map = CodepointMap::new();
    map.add(0xE000, 0xE0FF, FaceIdx(4));
    map.add(0xE100, 0xE1FF, FaceIdx(5));
    // Last codepoint of first range.
    assert_eq!(map.lookup(0xE0FF), Some(FaceIdx(4)));
    // First codepoint of second range.
    assert_eq!(map.lookup(0xE100), Some(FaceIdx(5)));
    // Gap between ranges (there is none, but the boundary is clean).
    assert_eq!(map.lookup(0xE200), None);
}

#[test]
fn boundary_codepoints() {
    let mut map = CodepointMap::new();
    map.add(0, 0x7F, FaceIdx(4)); // ASCII control + printable
    map.add(0x10_FF00, 0x10_FFFF, FaceIdx(5)); // Last valid Unicode block

    assert_eq!(map.lookup(0), Some(FaceIdx(4)));
    assert_eq!(map.lookup(0x7F), Some(FaceIdx(4)));
    assert_eq!(map.lookup(0x80), None);
    assert_eq!(map.lookup(0x10_FFFF), Some(FaceIdx(5)));
    assert_eq!(map.lookup(0x10_FF00), Some(FaceIdx(5)));
}

#[test]
fn not_empty_after_add() {
    let mut map = CodepointMap::new();
    assert!(map.is_empty());
    map.add(0x41, 0x5A, FaceIdx(4));
    assert!(!map.is_empty());
}

// ── Hex range parsing ──

#[test]
fn parse_range() {
    assert_eq!(parse_hex_range("E000-F8FF"), Some((0xE000, 0xF8FF)));
}

#[test]
fn parse_range_with_spaces() {
    assert_eq!(parse_hex_range(" E000 - F8FF "), Some((0xE000, 0xF8FF)));
}

#[test]
fn parse_single_codepoint() {
    assert_eq!(parse_hex_range("E0B0"), Some((0xE0B0, 0xE0B0)));
}

#[test]
fn parse_single_with_whitespace() {
    assert_eq!(parse_hex_range("  E0B0  "), Some((0xE0B0, 0xE0B0)));
}

#[test]
fn parse_lowercase_hex() {
    assert_eq!(parse_hex_range("e000-f8ff"), Some((0xE000, 0xF8FF)));
}

#[test]
fn parse_mixed_case() {
    assert_eq!(parse_hex_range("4e00-9FFF"), Some((0x4E00, 0x9FFF)));
}

#[test]
fn parse_reversed_range_returns_none() {
    assert_eq!(parse_hex_range("F8FF-E000"), None);
}

#[test]
fn parse_invalid_hex_returns_none() {
    assert_eq!(parse_hex_range("ZZZZ"), None);
    assert_eq!(parse_hex_range("E000-ZZZZ"), None);
    assert_eq!(parse_hex_range(""), None);
}

#[test]
fn parse_supplementary_plane() {
    assert_eq!(parse_hex_range("1F600-1F64F"), Some((0x1F600, 0x1F64F)));
}

#[test]
fn parse_u_plus_prefix_rejected() {
    // Our parser expects bare hex. The U+ prefix (Ghostty format) is
    // handled by config-layer stripping, not the range parser.
    assert_eq!(parse_hex_range("U+E000"), None);
    assert_eq!(parse_hex_range("U+E000-U+F8FF"), None);
}

#[test]
fn parse_max_unicode() {
    assert_eq!(parse_hex_range("10FFFF"), Some((0x10_FFFF, 0x10_FFFF)));
    assert_eq!(
        parse_hex_range("100000-10FFFF"),
        Some((0x10_0000, 0x10_FFFF))
    );
}
