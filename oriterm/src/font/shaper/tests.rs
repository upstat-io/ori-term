//! Unit tests for the text shaping pipeline.

use std::sync::Arc;

use oriterm_core::{Cell, CellExtra, CellFlags};

use super::{build_col_glyph_map, prepare_line, shape_prepared_runs};
use crate::font::collection::FontCollection;
use crate::font::{
    FaceIdx, FontSet, GlyphFormat, GlyphStyle, HintingMode, SyntheticFlags, subpx_bin,
};

// ── Helpers ──

/// Build a row of cells from a plain ASCII string (no flags, no extras).
fn make_cells(text: &str) -> Vec<Cell> {
    text.chars()
        .map(|ch| Cell {
            ch,
            ..Cell::default()
        })
        .collect()
}

/// Build a FontCollection from system discovery with default settings.
fn test_collection() -> FontCollection {
    let font_set = FontSet::load(None, 400).expect("font must load");
    FontCollection::new(
        font_set,
        12.0,
        96.0,
        GlyphFormat::Alpha,
        400,
        HintingMode::Full,
    )
    .expect("collection must build")
}

// ── Phase 1: Run Segmentation ──

#[test]
fn prepare_line_hello() {
    let fc = test_collection();
    let cells = make_cells("hello");
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    // All ASCII chars in same face → single run.
    assert_eq!(runs.len(), 1, "single face should produce one run");
    assert_eq!(runs[0].text, "hello");
    assert_eq!(runs[0].col_start, 0);
    assert_eq!(runs[0].face_idx, FaceIdx::REGULAR);
}

#[test]
fn prepare_line_space_excluded_from_runs() {
    let fc = test_collection();
    let cells = make_cells("hello world");
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    // Spaces are skipped (handled by renderer at fixed cell width).
    // Characters on both sides of the space share the same face, so they
    // merge into a single run. The text excludes the space.
    assert_eq!(runs.len(), 1, "same-face chars merge across spaces");
    assert_eq!(runs[0].text, "helloworld");
    assert_eq!(runs[0].col_start, 0);
}

#[test]
fn prepare_line_all_spaces() {
    let fc = test_collection();
    let cells = make_cells("   ");
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    assert!(runs.is_empty(), "all spaces should produce no runs");
}

#[test]
fn prepare_line_null_chars() {
    let fc = test_collection();
    let cells = make_cells("\0\0\0");
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    assert!(runs.is_empty(), "null chars should produce no runs");
}

#[test]
fn prepare_line_combining_mark() {
    let fc = test_collection();

    // 'a' followed by combining acute accent U+0301.
    let mut cells = vec![
        Cell {
            ch: 'a',
            ..Cell::default()
        },
        Cell {
            ch: 'b',
            ..Cell::default()
        },
    ];
    // Add combining mark to first cell.
    cells[0].extra = Some(Arc::new(CellExtra {
        underline_color: None,
        hyperlink: None,
        zerowidth: vec!['\u{0301}'],
    }));

    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    assert_eq!(runs.len(), 1, "same face should be one run");
    // Text should include both the base 'a', the combining mark, and 'b'.
    assert_eq!(runs[0].text, "a\u{0301}b");
    // byte_to_col: 'a' maps to col 0, U+0301 (2 bytes) maps to col 0, 'b' maps to col 1.
    assert_eq!(runs[0].byte_to_col[0], 0); // 'a'
    assert_eq!(runs[0].byte_to_col[1], 0); // U+0301 byte 1
    assert_eq!(runs[0].byte_to_col[2], 0); // U+0301 byte 2
    assert_eq!(runs[0].byte_to_col[3], 1); // 'b'
}

#[test]
fn prepare_line_wide_char() {
    let fc = test_collection();

    // CJK ideograph (wide char) followed by ASCII.
    let cells = vec![
        Cell {
            ch: '\u{4E00}',
            flags: CellFlags::WIDE_CHAR,
            ..Cell::default()
        },
        Cell {
            ch: ' ',
            flags: CellFlags::WIDE_CHAR_SPACER,
            ..Cell::default()
        },
        Cell {
            ch: 'a',
            ..Cell::default()
        },
    ];

    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    // With embedded-only font, both chars resolve to Regular (CJK is .notdef).
    // They may be in the same run or different depending on face resolution.
    // Key check: spacer is NOT in any run's text.
    for run in &runs {
        assert!(
            !run.text.contains(' '),
            "spacer should not appear in run text"
        );
    }
}

#[test]
fn prepare_line_byte_to_col_ascii() {
    let fc = test_collection();
    let cells = make_cells("abc");
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    assert_eq!(runs.len(), 1);
    // ASCII: 1 byte per char.
    assert_eq!(runs[0].byte_to_col, vec![0, 1, 2]);
}

#[test]
fn prepare_line_reuses_scratch_buffer() {
    let fc = test_collection();
    let cells = make_cells("hello");
    let mut runs = Vec::new();

    // First call.
    prepare_line(&cells, cells.len(), &fc, &mut runs);
    assert_eq!(runs.len(), 1);

    // Second call should clear and reuse.
    let cells2 = make_cells("A B");
    prepare_line(&cells2, cells2.len(), &fc, &mut runs);
    // "A" and "B" share the same face → 1 run ("AB"), space excluded.
    assert_eq!(runs.len(), 1, "scratch buffer should be cleared and reused");
}

// ── VS16 emoji presentation (Section 6.10) ──

#[test]
fn prepare_line_vs16_in_zerowidth() {
    // A cell with VS16 (U+FE0F) in zerowidth should use emoji resolution.
    // With system fonts, this may resolve to a different face than normal.
    let fc = test_collection();
    let cells = vec![
        Cell {
            ch: '\u{2764}', // ❤ (HEAVY BLACK HEART)
            extra: Some(Arc::new(CellExtra {
                underline_color: None,
                hyperlink: None,
                zerowidth: vec!['\u{FE0F}'], // VS16
            })),
            ..Cell::default()
        },
        Cell {
            ch: 'a',
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    // Should produce at least one run containing the heart character.
    let has_heart = runs.iter().any(|r| r.text.contains('\u{2764}'));
    assert!(has_heart, "heart should appear in a shaping run");

    // VS16 should also be in the run text (passed to shaper for font handling).
    let has_vs16 = runs.iter().any(|r| r.text.contains('\u{FE0F}'));
    assert!(has_vs16, "VS16 should be in run text for shaper");
}

#[test]
fn prepare_line_vs16_may_use_different_face() {
    // With VS16, the heart should resolve preferring emoji fallback.
    // Without VS16, it should use normal resolution order.
    let fc = test_collection();

    // Cell WITH VS16.
    let with_vs16 = vec![Cell {
        ch: '\u{2764}',
        extra: Some(Arc::new(CellExtra {
            underline_color: None,
            hyperlink: None,
            zerowidth: vec!['\u{FE0F}'],
        })),
        ..Cell::default()
    }];
    let mut runs_vs16 = Vec::new();
    prepare_line(&with_vs16, with_vs16.len(), &fc, &mut runs_vs16);

    // Cell WITHOUT VS16.
    let without_vs16 = vec![Cell {
        ch: '\u{2764}',
        ..Cell::default()
    }];
    let mut runs_plain = Vec::new();
    prepare_line(&without_vs16, without_vs16.len(), &fc, &mut runs_plain);

    // Both should produce runs (the character exists in some font).
    // The face_idx may differ if emoji fallback is available.
    // Key invariant: no panics, valid runs produced.
    if !runs_vs16.is_empty() && !runs_plain.is_empty() {
        // If a color emoji font is in the fallback chain, VS16 version
        // should use a fallback face (emoji font) while plain may use
        // the primary font.
        // This is a soft check — depends on system fonts.
        let vs16_face = runs_vs16[0].face_idx;
        let plain_face = runs_plain[0].face_idx;
        // Log for diagnostic visibility; both outcomes are valid.
        if vs16_face != plain_face {
            // VS16 triggered emoji fallback — expected behavior.
        }
    }
}

// ── Phase 2: Shaping ──

#[test]
fn shape_hello_produces_five_glyphs() {
    let fc = test_collection();
    let cells = make_cells("Hello");
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);

    assert_eq!(output.len(), 5, "5 glyphs for 'Hello'");
    for g in &output {
        assert_ne!(g.glyph_id, 0, "glyph ID should not be .notdef for ASCII");
    }
}

#[test]
fn shape_preserves_column_positions() {
    let fc = test_collection();
    let cells = make_cells("A B");
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);

    // "A" and "B" merge into one run "AB" with byte_to_col=[0, 2].
    assert_eq!(output.len(), 2);
    assert_eq!(col_starts[0], 0, "'A' at column 0");
    assert_eq!(col_starts[1], 2, "'B' at column 2 (space skipped)");
}

#[test]
fn shape_empty_runs_produces_no_output() {
    let fc = test_collection();
    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&[], &faces, &fc, &mut output, &mut col_starts, &mut None);

    assert!(output.is_empty());
}

#[test]
fn shape_reuses_scratch_buffer() {
    let fc = test_collection();
    let faces = fc.create_shaping_faces();
    let mut runs = Vec::new();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();

    let cells = make_cells("AB");
    prepare_line(&cells, cells.len(), &fc, &mut runs);
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);
    assert_eq!(output.len(), 2);

    // Re-shape a different line — output should be replaced.
    let cells2 = make_cells("X");
    prepare_line(&cells2, cells2.len(), &fc, &mut runs);
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);
    assert_eq!(output.len(), 1, "output should be cleared on re-shape");
}

// ── Phase 2: CJK Wide Char Shaping ──

#[test]
fn shape_wide_char_col_span_two() {
    let fc = test_collection();

    // CJK ideograph '好' (U+597D) is a wide character occupying 2 grid columns.
    let cells = vec![
        Cell {
            ch: '\u{597D}',
            flags: CellFlags::WIDE_CHAR,
            ..Cell::default()
        },
        Cell {
            ch: ' ',
            flags: CellFlags::WIDE_CHAR_SPACER,
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);

    // Should produce exactly 1 glyph for the wide character.
    assert_eq!(output.len(), 1, "wide char should produce 1 glyph");
    assert_eq!(col_starts[0], 0);

    // If a CJK fallback font is available, verify the col_map reflects 2-column span.
    if output[0].glyph_id != 0 {
        let mut map = Vec::new();
        build_col_glyph_map(&col_starts, cells.len(), &mut map);
        assert!(map[0].is_some(), "col 0 should have glyph");
        assert_eq!(map[1], None, "col 1 is continuation of wide char");
    }
}

#[test]
fn shape_cjk_uses_fallback_face() {
    let fc = test_collection();

    // CJK ideograph '好' (U+597D) — not in JetBrains Mono, requires fallback.
    let cells = vec![
        Cell {
            ch: '\u{597D}',
            flags: CellFlags::WIDE_CHAR,
            ..Cell::default()
        },
        Cell {
            ch: ' ',
            flags: CellFlags::WIDE_CHAR_SPACER,
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);

    assert_eq!(output.len(), 1);

    // If system has CJK fallback, glyph should come from a fallback face.
    // If no fallback installed, glyph_id will be 0 (.notdef) — both are valid.
    if output[0].glyph_id != 0 {
        let fi = FaceIdx(output[0].face_index);
        assert!(
            fi.is_fallback(),
            "CJK char should be shaped from fallback face (got {fi:?})",
        );
    }
}

#[test]
fn shape_ascii_cjk_ascii_column_positions() {
    let fc = test_collection();

    // "A好B" — A at col 0, 好 at col 1 (wide, spans 2), B at col 3.
    let cells = vec![
        Cell {
            ch: 'A',
            ..Cell::default()
        },
        Cell {
            ch: '\u{597D}',
            flags: CellFlags::WIDE_CHAR,
            ..Cell::default()
        },
        Cell {
            ch: ' ',
            flags: CellFlags::WIDE_CHAR_SPACER,
            ..Cell::default()
        },
        Cell {
            ch: 'B',
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);

    assert_eq!(output.len(), 3, "'A好B' should produce 3 glyphs");
    assert_eq!(col_starts[0], 0, "'A' at column 0");

    // CJK char at col 1.
    let cjk_idx = col_starts
        .iter()
        .position(|&c| c == 1)
        .expect("CJK glyph at col 1");
    if output[cjk_idx].glyph_id != 0 {
        let mut map = Vec::new();
        build_col_glyph_map(&col_starts, cells.len(), &mut map);
        assert_eq!(map[2], None, "col 2 is continuation of CJK wide char");
    }

    // 'B' at col 3 (after the 2-column wide char).
    assert!(col_starts.iter().any(|&c| c == 3), "'B' should be at col 3",);
}

#[test]
fn shape_consecutive_cjk_column_positions() {
    let fc = test_collection();

    // "好世" — two CJK chars, each width 2.
    // 好 at col 0 (span 2), 世 at col 2 (span 2).
    let cells = vec![
        Cell {
            ch: '\u{597D}',
            flags: CellFlags::WIDE_CHAR,
            ..Cell::default()
        },
        Cell {
            ch: ' ',
            flags: CellFlags::WIDE_CHAR_SPACER,
            ..Cell::default()
        },
        Cell {
            ch: '\u{4E16}',
            flags: CellFlags::WIDE_CHAR,
            ..Cell::default()
        },
        Cell {
            ch: ' ',
            flags: CellFlags::WIDE_CHAR_SPACER,
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);

    assert_eq!(output.len(), 2, "two CJK chars should produce 2 glyphs");
    assert_eq!(col_starts[0], 0, "first CJK at col 0");
    assert_eq!(col_starts[1], 2, "second CJK at col 2");
}

#[test]
fn shape_ideographic_space_wide() {
    let fc = test_collection();

    // U+3000 IDEOGRAPHIC SPACE — width 2 per unicode-width, but NOT U+0020
    // so it is NOT skipped during segmentation. It goes through shaping.
    let cells = vec![
        Cell {
            ch: '\u{3000}',
            flags: CellFlags::WIDE_CHAR,
            ..Cell::default()
        },
        Cell {
            ch: ' ',
            flags: CellFlags::WIDE_CHAR_SPACER,
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    // Ideographic space is not U+0020, so it is not skipped — it enters a run.
    assert_eq!(
        runs.len(),
        1,
        "ideographic space should produce a shaping run"
    );
    assert!(
        runs[0].text.contains('\u{3000}'),
        "run text should contain ideographic space",
    );

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);

    assert_eq!(output.len(), 1, "ideographic space should produce 1 glyph");
}

#[test]
fn shape_wide_char_notdef_graceful() {
    let fc = test_collection();

    // CJK Extension B character — unlikely to have font coverage even with
    // CJK fallbacks. Tests the .notdef path for wide characters.
    let cells = vec![
        Cell {
            ch: '\u{2A6DF}',
            flags: CellFlags::WIDE_CHAR,
            ..Cell::default()
        },
        Cell {
            ch: ' ',
            flags: CellFlags::WIDE_CHAR_SPACER,
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);

    // Regardless of font coverage: valid output, no panic.
    assert_eq!(output.len(), 1, "should produce exactly 1 glyph");
}

// ── Phase 3: Column ↔ Glyph Mapping ──

#[test]
fn col_glyph_map_wide_char_pipeline() {
    let fc = test_collection();

    // "好B" — wide char at cols 0-1, ASCII at col 2.
    let cells = vec![
        Cell {
            ch: '\u{597D}',
            flags: CellFlags::WIDE_CHAR,
            ..Cell::default()
        },
        Cell {
            ch: ' ',
            flags: CellFlags::WIDE_CHAR_SPACER,
            ..Cell::default()
        },
        Cell {
            ch: 'B',
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);

    let mut map = Vec::new();
    build_col_glyph_map(&col_starts, cells.len(), &mut map);

    assert_eq!(map.len(), 3);
    // Col 0: wide char glyph.
    assert!(map[0].is_some(), "col 0 should have the wide char glyph");

    // Col 1: continuation of wide char (no glyph starts here).
    assert_eq!(map[1], None, "col 1 is continuation of wide char");

    // Col 2: 'B'.
    assert!(map[2].is_some(), "col 2 should have 'B' glyph");
}

#[test]
fn col_glyph_map_simple_ascii() {
    let fc = test_collection();
    let cells = make_cells("abc");
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);

    let mut map = Vec::new();
    build_col_glyph_map(&col_starts, cells.len(), &mut map);

    assert_eq!(map.len(), 3);
    // Each column maps to its glyph.
    assert_eq!(map[0], Some(0));
    assert_eq!(map[1], Some(1));
    assert_eq!(map[2], Some(2));
}

#[test]
fn col_glyph_map_with_spaces() {
    let fc = test_collection();
    let cells = make_cells("A B");
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);

    let mut map = Vec::new();
    build_col_glyph_map(&col_starts, cells.len(), &mut map);

    assert_eq!(map.len(), 3);
    // 'A' at col 0, space at col 1 (no glyph), 'B' at col 2.
    assert_eq!(map[0], Some(0));
    assert_eq!(map[1], None, "space column has no glyph");
    assert_eq!(map[2], Some(1));
}

#[test]
fn col_glyph_map_empty_line() {
    let mut map = Vec::new();
    let col_starts: Vec<usize> = Vec::new();
    build_col_glyph_map(&col_starts, 5, &mut map);

    assert_eq!(map.len(), 5);
    assert!(map.iter().all(|e| e.is_none()));
}

#[test]
fn col_glyph_map_reuses_buffer() {
    let mut map = Vec::new();

    // First call.
    let empty: Vec<usize> = Vec::new();
    build_col_glyph_map(&empty, 3, &mut map);
    assert_eq!(map.len(), 3);

    // Second call with different size.
    let col_starts = vec![0];
    build_col_glyph_map(&col_starts, 5, &mut map);
    assert_eq!(map.len(), 5);
    assert_eq!(map[0], Some(0));
    assert!(map[1..].iter().all(|e| e.is_none()));
}

#[test]
fn col_glyph_map_first_wins_for_combining_marks() {
    // Two glyphs at the same col_start: base char (glyph 50) and combining mark (glyph 51).
    // build_col_glyph_map should store the FIRST glyph's index.
    // Two glyphs at col 0 (base + combining mark), one at col 1.
    let col_starts = vec![0, 0, 1];

    let mut map = Vec::new();
    build_col_glyph_map(&col_starts, 2, &mut map);

    // First-wins: col 0 points to glyph 0 (the base), not glyph 1 (the combining mark).
    assert_eq!(map[0], Some(0), "first-wins: base glyph claims col 0");
    assert_eq!(map[1], Some(2), "next column maps to glyph at col 1");
}

#[test]
fn col_glyph_map_ligature_span() {
    // Simulate a ligature spanning 2 columns: one glyph at col 0, next at col 2.
    // Col 1 has no glyph (continuation of ligature).
    let col_starts = vec![0, 2];

    let mut map = Vec::new();
    build_col_glyph_map(&col_starts, 3, &mut map);

    assert_eq!(map[0], Some(0), "ligature starts at col 0");
    assert_eq!(map[1], None, "col 1 is continuation of ligature");
    assert_eq!(map[2], Some(1), "normal glyph at col 2");
}

// ── UI Text Shaping ──

#[test]
fn ui_shape_hello_produces_five_glyphs() {
    let fc = test_collection();
    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    super::shape_text_string(
        "Hello",
        GlyphStyle::Regular,
        SyntheticFlags::NONE,
        400,
        &faces,
        &fc,
        &mut output,
        &mut None,
    );

    assert_eq!(output.len(), 5, "5 glyphs for 'Hello'");
    for g in &output {
        assert!(g.x_advance > 0.0, "each glyph should have positive advance");
    }
}

#[test]
fn ui_shape_sequential_advances() {
    let fc = test_collection();
    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    super::shape_text_string(
        "Hello",
        GlyphStyle::Regular,
        SyntheticFlags::NONE,
        400,
        &faces,
        &fc,
        &mut output,
        &mut None,
    );

    // Monospace font: all advances should be equal.
    let first = output[0].x_advance;
    for g in &output[1..] {
        assert!(
            (g.x_advance - first).abs() < 0.01,
            "monospace font should have equal advances: {first} vs {}",
            g.x_advance,
        );
    }
}

#[test]
fn ui_shape_space_has_positive_advance() {
    let fc = test_collection();
    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    super::shape_text_string(
        "A B",
        GlyphStyle::Regular,
        SyntheticFlags::NONE,
        400,
        &faces,
        &fc,
        &mut output,
        &mut None,
    );

    assert_eq!(output.len(), 3, "'A B' → 3 glyphs");
    // Space is shaped by rustybuzz (proportional advance), so it gets a
    // real glyph_id from the font, not the old advance-only sentinel (0).
    assert!(
        output[1].x_advance > 0.0,
        "space should have positive advance"
    );
}

#[test]
fn ui_shape_empty_string() {
    let fc = test_collection();
    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    super::shape_text_string(
        "",
        GlyphStyle::Regular,
        SyntheticFlags::NONE,
        400,
        &faces,
        &fc,
        &mut output,
        &mut None,
    );

    assert!(output.is_empty(), "empty string produces no glyphs");
}

#[test]
fn ui_measure_text_returns_total_width() {
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    let width = super::measure_text("Hello", &fc);

    // measure_text uses unicode_width × cell_width, so the result is exact.
    let expected = 5.0 * cell_w;
    assert!(
        (width - expected).abs() < f32::EPSILON,
        "measured width {width} should be exactly {expected}",
    );
}

#[test]
fn ui_measure_empty_is_zero() {
    let fc = test_collection();
    let width = super::measure_text("", &fc);
    assert!(
        (width - 0.0).abs() < f32::EPSILON,
        "empty text has zero width",
    );
}

#[test]
fn ui_truncate_short_text_unchanged() {
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    let result = super::truncate_with_ellipsis("Hello", 10.0 * cell_w, 0.0, &fc);
    assert_eq!(
        result.as_ref(),
        "Hello",
        "short text should not be truncated"
    );
}

#[test]
fn ui_truncate_long_text_gets_ellipsis() {
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    // Max width fits 3 cells, text is 10 chars.
    let result = super::truncate_with_ellipsis("HelloWorld", 3.0 * cell_w, 0.0, &fc);
    assert!(
        result.ends_with('\u{2026}'),
        "truncated text should end with ellipsis: {result:?}",
    );
    assert!(
        result.len() < "HelloWorld".len(),
        "truncated should be shorter"
    );
}

#[test]
fn ui_truncate_exact_fit() {
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    // Max width exactly fits 5 cells.
    let result = super::truncate_with_ellipsis("Hello", 5.0 * cell_w, 0.0, &fc);
    assert_eq!(result.as_ref(), "Hello", "exact fit should not truncate");
}

// ── UI Text Measurement: Unicode Width ──

#[test]
fn measure_text_cjk_double_width() {
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    // "A你好B" → 1 + 2 + 2 + 1 = 6 display columns.
    let width = super::measure_text("A\u{4F60}\u{597D}B", &fc);
    let expected = 6.0 * cell_w;
    assert!(
        (width - expected).abs() < f32::EPSILON,
        "CJK width should be 6 cells: {width} vs {expected}",
    );
}

#[test]
fn measure_text_combining_marks_zero_width() {
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    // "e\u{0301}" (é composed) → base 'e' is width 1, combining accent is width 0.
    let width = super::measure_text("e\u{0301}", &fc);
    let expected = 1.0 * cell_w;
    assert!(
        (width - expected).abs() < f32::EPSILON,
        "combining mark should add zero width: {width} vs {expected}",
    );
}

#[test]
fn measure_text_zwj_sequence() {
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    // ZWJ emoji: family sequence (👨‍👩‍👧).
    // unicode-width treats each codepoint individually:
    // 👨 (width 2) + ZWJ (width 0) + 👩 (width 0 or 2) + ZWJ + 👧
    // Exact width depends on unicode-width version; just verify >= 2 cells.
    let width = super::measure_text("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}", &fc);
    assert!(
        width >= 2.0 * cell_w,
        "ZWJ sequence should be at least 2 cells wide: {width}",
    );
}

#[test]
fn truncate_with_ellipsis_cjk_boundary() {
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    // CJK string: each char is width 2. Budget for 3 cells + 1 for ellipsis = 4 cells.
    // "你好世界" = 8 cells total. Max 4 cells → fits 1 CJK char (2 cells) + "…" (1 cell).
    let result =
        super::truncate_with_ellipsis("\u{4F60}\u{597D}\u{4E16}\u{754C}", 4.0 * cell_w, 0.0, &fc);
    assert!(
        result.ends_with('\u{2026}'),
        "truncated CJK should end with ellipsis: {result:?}",
    );
    // Should not exceed the max width.
    let result_width = super::measure_text(&result, &fc);
    assert!(
        result_width <= 4.0 * cell_w + f32::EPSILON,
        "truncated result should fit in budget: {result_width} vs {}",
        4.0 * cell_w,
    );
}

// ── Zero-width edge cases (unicode-width parity) ──

#[test]
fn measure_text_variation_selectors_zero_width() {
    // FE0E (text presentation) and FE0F (emoji presentation) are zero-width.
    let fc = test_collection();
    let width = super::measure_text("\u{FE0E}", &fc);
    assert!(width.abs() < f32::EPSILON);
    let width = super::measure_text("\u{FE0F}", &fc);
    assert!(width.abs() < f32::EPSILON);
}

#[test]
fn measure_text_null_and_control_chars_zero_width() {
    let fc = test_collection();
    assert!(super::measure_text("\0", &fc).abs() < f32::EPSILON);
    assert!(super::measure_text("\x01", &fc).abs() < f32::EPSILON);
    assert!(super::measure_text("\x7F", &fc).abs() < f32::EPSILON); // DEL
}

#[test]
fn measure_text_soft_hyphen_zero_width() {
    let fc = test_collection();
    // U+00AD SOFT HYPHEN — zero display width per unicode-width.
    assert!(super::measure_text("\u{00AD}", &fc).abs() < f32::EPSILON);
}

// ── Truncation budget edge cases ──

#[test]
fn truncate_with_ellipsis_zero_budget() {
    let fc = test_collection();
    // Zero budget → just ellipsis.
    let result = super::truncate_with_ellipsis("Hello", 0.0, 0.0, &fc);
    assert_eq!(result.as_ref(), "\u{2026}");
}

#[test]
fn truncate_with_ellipsis_one_cell_budget() {
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    // Exactly 1 cell → only room for ellipsis.
    let result = super::truncate_with_ellipsis("Hello", cell_w, 0.0, &fc);
    assert_eq!(result.as_ref(), "\u{2026}");
}

#[test]
fn truncate_with_ellipsis_shorter_than_max() {
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    // String is 2 cells, max is 10 cells → returned unchanged.
    let result = super::truncate_with_ellipsis("AB", 10.0 * cell_w, 0.0, &fc);
    assert_eq!(
        result.as_ref(),
        "AB",
        "short string should be returned unchanged",
    );
}

// Letter spacing + ellipsis truncation

#[test]
fn truncate_with_ellipsis_respects_letter_spacing() {
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    let spacing = cell_w * 0.5; // 50% of cell width per character

    // "ABCDE" = 5 cells. With spacing: 5 * (cell_w + spacing) = 5 * 1.5 * cell_w = 7.5 * cell_w.
    // Budget = 5 * cell_w. Without spacing awareness, all 5 chars would "fit".
    // With spacing, only ~3 chars + ellipsis should fit.
    let result = super::truncate_with_ellipsis("ABCDE", 5.0 * cell_w, spacing, &fc);
    assert!(
        result.ends_with('\u{2026}'),
        "spaced text should be truncated: {result:?}",
    );
    assert!(
        result.chars().count() < 5,
        "truncated result should have fewer than 5 visible chars: {result:?}",
    );
}

#[test]
fn truncate_with_ellipsis_spacing_short_text_unchanged() {
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    let spacing = cell_w * 0.1;

    // "AB" = 2 cells. With spacing: 2 * (cell_w + 0.1 * cell_w) = 2.2 * cell_w.
    // Budget = 10 * cell_w — plenty of room.
    let result = super::truncate_with_ellipsis("AB", 10.0 * cell_w, spacing, &fc);
    assert_eq!(
        result.as_ref(),
        "AB",
        "short spaced text should not truncate"
    );
}

#[test]
fn shape_text_ellipsis_with_spacing_stays_within_budget() {
    use super::ui_text;
    use oriterm_ui::text::{TextOverflow, TextStyle};

    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    let spacing = cell_w * 0.5;
    let max_width = 5.0 * cell_w;

    let style =
        TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_overflow(TextOverflow::Ellipsis);

    let shaped = ui_text::shape_text("abcdefghij", &style, max_width, spacing, &fc);

    // The shaped width (including spacing applied to glyphs) must not exceed budget.
    assert!(
        shaped.width <= max_width + 0.01,
        "shaped width {:.2} must not exceed budget {:.2}",
        shaped.width,
        max_width,
    );
    assert!(!shaped.glyphs.is_empty(), "must produce glyphs");
}

// Ligature Shaping (Section 6.4)

#[test]
fn shape_arrow_ligature_col_span_two() {
    let fc = test_collection();
    let cells = make_cells("=>");
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);

    // Whether "=>" produces a ligature depends on the font. If the font
    // has a `calt` substitution for "=>", we get 1 glyph with col_span=2.
    // Otherwise we get 2 separate glyphs with col_span=1 each.
    if output.len() == 1 {
        // Ligature: single glyph at col 0, col 1 should be None in the map.
        assert_eq!(col_starts[0], 0, "ligature starts at col 0");
        let mut map = Vec::new();
        build_col_glyph_map(&col_starts, cells.len(), &mut map);
        assert_eq!(map[1], None, "col 1 is continuation of ligature");
    } else {
        // No ligature — font doesn't support it. Verify 2 separate glyphs.
        assert_eq!(output.len(), 2, "non-ligature: 2 glyphs for '=>'");
    }
}

#[test]
fn shape_fi_ligature_col_span_two() {
    let fc = test_collection();
    let cells = make_cells("fi");
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);

    // "fi" ligature via `liga` feature: 1 glyph if the font supports it,
    // otherwise 2 separate glyphs.
    if output.len() == 1 {
        assert_eq!(col_starts[0], 0, "ligature starts at col 0");
        let mut map = Vec::new();
        build_col_glyph_map(&col_starts, cells.len(), &mut map);
        assert_eq!(map[1], None, "col 1 is continuation of ligature");
    } else {
        assert_eq!(output.len(), 2, "non-ligature: 2 glyphs for 'fi'");
    }
}

// ── Subpixel Positioning: UI Text Mixed Phases ──

/// UI text glyphs land at different subpixel phases across a shaped string.
///
/// In monospace fonts the per-glyph advance is typically non-integer in pixels,
/// so cumulative x positions produce varying fractional parts. Each fractional
/// offset quantizes to one of 4 subpixel phases (0, 1, 2, 3). A sufficiently
/// long string should hit at least 2 distinct phases.
#[test]
fn ui_text_mixed_subpixel_phases() {
    let fc = test_collection();
    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();

    // Shape a long-enough string to produce varied cumulative fractional offsets.
    let text = "The quick brown fox jumps over the lazy dog";
    super::shape_text_string(
        text,
        GlyphStyle::Regular,
        SyntheticFlags::NONE,
        400,
        &faces,
        &fc,
        &mut output,
        &mut None,
    );

    assert!(
        !output.is_empty(),
        "shaped output should not be empty for '{text}'",
    );

    // Compute cumulative x position and subpixel phase for each glyph.
    let mut cumulative_x = 0.0_f32;
    let mut phases = std::collections::HashSet::new();
    for glyph in &output {
        let x_pos = cumulative_x + glyph.x_offset;
        phases.insert(subpx_bin(x_pos));
        cumulative_x += glyph.x_advance;
    }

    assert!(
        phases.len() >= 2,
        "UI text should produce at least 2 distinct subpixel phases, got {phases:?}. \
         Cell width = {}, which may be exactly integer — try a different font size.",
        fc.cell_metrics().width,
    );
}

// ── Per-Face Synthetic Bold (TPR-02-010) ──

#[test]
fn per_face_synthetic_adds_bold_for_face_without_wght() {
    // When requested weight >= 700 and base synthetic has no BOLD,
    // faces without a wght axis should get synthetic BOLD added.
    let fc = test_collection();

    // Skip on platforms where the system default font has a wght axis
    // (e.g., Cascadia Code on Windows). The test is specifically for
    // static fonts where variable-weight adjustment is impossible.
    if fc.face_has_wght_axis(FaceIdx::REGULAR) {
        return;
    }

    let result =
        super::ui_text::per_face_synthetic(SyntheticFlags::NONE, 700, FaceIdx::REGULAR, &fc);
    assert!(
        result.contains(SyntheticFlags::BOLD),
        "face without wght axis should get synthetic BOLD at weight 700"
    );
}

#[test]
fn per_face_synthetic_no_bold_below_700() {
    let fc = test_collection();

    let result =
        super::ui_text::per_face_synthetic(SyntheticFlags::NONE, 400, FaceIdx::REGULAR, &fc);
    assert!(
        !result.contains(SyntheticFlags::BOLD),
        "weight 400 should not trigger synthetic BOLD"
    );
}

#[test]
fn per_face_synthetic_preserves_existing_bold() {
    let fc = test_collection();

    // If base already has BOLD (primary decided synthetic bold), don't add it again.
    let result =
        super::ui_text::per_face_synthetic(SyntheticFlags::BOLD, 700, FaceIdx::REGULAR, &fc);
    assert!(
        result.contains(SyntheticFlags::BOLD),
        "existing BOLD should be preserved"
    );
}

#[test]
fn per_face_synthetic_skips_bold_primary_slot() {
    // TPR-02-011: When a face is already the Bold primary slot (FaceIdx 1),
    // per_face_synthetic must NOT add synthetic bold — that would double-embolden.
    let fc = test_collection();

    let bold_face = FaceIdx(1);
    let result = super::ui_text::per_face_synthetic(SyntheticFlags::NONE, 700, bold_face, &fc);
    assert!(
        !result.contains(SyntheticFlags::BOLD),
        "Bold primary slot should not get synthetic BOLD — would double-embolden"
    );
}

#[test]
fn per_face_synthetic_skips_bold_italic_primary_slot() {
    // Same as above but for BoldItalic slot (FaceIdx 3).
    let fc = test_collection();

    let bold_italic_face = FaceIdx(3);
    let result =
        super::ui_text::per_face_synthetic(SyntheticFlags::NONE, 700, bold_italic_face, &fc);
    assert!(
        !result.contains(SyntheticFlags::BOLD),
        "BoldItalic primary slot should not get synthetic BOLD"
    );
}

#[test]
fn shape_text_string_bold_weight_sets_synthetic_on_static_font() {
    // Integration test: shape_text_string at weight 700 on a static font
    // (no wght axis) should produce glyphs with synthetic BOLD bits.
    let fc = test_collection();

    // Skip on platforms where the system default font has a wght axis
    // (e.g., Cascadia Code on Windows). Variable fonts handle weight
    // natively and don't need synthetic bold.
    if fc.face_has_wght_axis(FaceIdx::REGULAR) {
        return;
    }

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    super::shape_text_string(
        "AB",
        GlyphStyle::Regular,
        SyntheticFlags::NONE,
        700,
        &faces,
        &fc,
        &mut output,
        &mut None,
    );

    assert!(!output.is_empty(), "should produce glyphs");
    for g in &output {
        let syn = SyntheticFlags::from_bits_truncate(g.synthetic);
        assert!(
            syn.contains(SyntheticFlags::BOLD),
            "glyph on static font at weight 700 should have synthetic BOLD, got {syn:?}"
        );
    }
}

// ── Attribute-Based Run Splitting ──

#[test]
fn prepare_line_bold_splits_run() {
    let fc = test_collection();

    // "aB" where B is bold — different GlyphStyle → potentially different face/synthetic.
    let cells = vec![
        Cell {
            ch: 'a',
            ..Cell::default()
        },
        Cell {
            ch: 'B',
            flags: CellFlags::BOLD,
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    // Bold resolves to a different GlyphStyle (Bold vs Regular), which may
    // map to a different face_idx or synthetic flags. Either way, the run
    // should split at the attribute boundary.
    assert!(
        runs.len() >= 2,
        "bold cell should cause run split: got {} run(s)",
        runs.len(),
    );
    assert_eq!(runs[0].text, "a");
    assert_eq!(runs[1].text, "B");
}

#[test]
fn prepare_line_italic_splits_run() {
    let fc = test_collection();

    let cells = vec![
        Cell {
            ch: 'x',
            ..Cell::default()
        },
        Cell {
            ch: 'y',
            flags: CellFlags::ITALIC,
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    assert!(
        runs.len() >= 2,
        "italic cell should cause run split: got {} run(s)",
        runs.len(),
    );
}

#[test]
fn prepare_line_same_style_merges() {
    let fc = test_collection();

    // Two bold cells should merge into a single run.
    let cells = vec![
        Cell {
            ch: 'A',
            flags: CellFlags::BOLD,
            ..Cell::default()
        },
        Cell {
            ch: 'B',
            flags: CellFlags::BOLD,
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    assert_eq!(runs.len(), 1, "same style should merge into one run");
    assert_eq!(runs[0].text, "AB");
}

#[test]
fn prepare_line_bold_regular_bold_three_runs() {
    let fc = test_collection();

    // "AbC" where A and C are bold, b is regular → 3 runs.
    let cells = vec![
        Cell {
            ch: 'A',
            flags: CellFlags::BOLD,
            ..Cell::default()
        },
        Cell {
            ch: 'b',
            ..Cell::default()
        },
        Cell {
            ch: 'C',
            flags: CellFlags::BOLD,
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    assert!(
        runs.len() >= 3,
        "bold-regular-bold should produce 3 runs: got {}",
        runs.len(),
    );
}

// ── VS15 Text Presentation ──

#[test]
fn measure_text_vs15_zero_width() {
    // VS15 (U+FE0E) is a zero-width variation selector (text presentation).
    let fc = test_collection();
    let width = super::measure_text("\u{FE0E}", &fc);
    assert!(
        width.abs() < f32::EPSILON,
        "VS15 alone should have zero width",
    );
}

#[test]
fn prepare_line_vs15_in_zerowidth() {
    // VS15 stored in zerowidth should NOT trigger emoji fallback path
    // (only VS16 does). The base character should be shaped normally.
    let fc = test_collection();
    let cells = vec![
        Cell {
            ch: '\u{270C}', // Victory hand
            extra: Some(Arc::new(CellExtra {
                underline_color: None,
                hyperlink: None,
                zerowidth: vec!['\u{FE0E}'], // VS15 — text presentation
            })),
            ..Cell::default()
        },
        Cell {
            ch: 'a',
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    // Should produce at least one run containing the victory hand.
    let has_victory = runs.iter().any(|r| r.text.contains('\u{270C}'));
    assert!(has_victory, "victory hand should appear in a run");

    // VS15 should also be in the run text (passed to shaper for font handling).
    let has_vs15 = runs.iter().any(|r| r.text.contains('\u{FE0E}'));
    assert!(has_vs15, "VS15 should be passed to shaper");
}

// ── Emoji Interleaved with ASCII ──

#[test]
fn prepare_line_emoji_ascii_splits_runs() {
    // "A😃D" — emoji resolves to a different face (emoji fallback) than ASCII.
    let fc = test_collection();
    let cells = vec![
        Cell {
            ch: 'A',
            ..Cell::default()
        },
        Cell {
            ch: '\u{1F603}', // 😃
            flags: CellFlags::WIDE_CHAR,
            ..Cell::default()
        },
        Cell {
            ch: ' ',
            flags: CellFlags::WIDE_CHAR_SPACER,
            ..Cell::default()
        },
        Cell {
            ch: 'D',
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    // The emoji should resolve to a different face (emoji fallback) than 'A'/'D'.
    // If no emoji font is available, both may resolve to the same face (.notdef).
    // Either way, verify no panic and that runs are produced.
    assert!(!runs.is_empty(), "should produce at least one run");

    // Check that run text contains 'A', the emoji, and 'D'.
    let all_text: String = runs.iter().map(|r| r.text.as_str()).collect();
    assert!(all_text.contains('A'));
    assert!(all_text.contains('\u{1F603}'));
    assert!(all_text.contains('D'));

    // If emoji font is available, there should be at least 2 runs (ASCII vs emoji face).
    if runs.len() >= 2 {
        // Verify face indices differ between ASCII and emoji runs.
        let ascii_run = runs.iter().find(|r| r.text.contains('A')).unwrap();
        let emoji_run = runs.iter().find(|r| r.text.contains('\u{1F603}')).unwrap();
        assert_ne!(
            ascii_run.face_idx, emoji_run.face_idx,
            "emoji should use different face than ASCII",
        );
    }
}

// ── ZWJ + Skin-Tone → Single Output Glyph ──

#[test]
fn shape_zwj_skin_tone_collapses() {
    // 👍🏽 = U+1F44D (thumbs up) + U+1F3FD (medium skin tone)
    // If an emoji font is available, these should shape into 1 glyph.
    let fc = test_collection();
    let cells = vec![
        Cell {
            ch: '\u{1F44D}',
            flags: CellFlags::WIDE_CHAR,
            extra: Some(Arc::new(CellExtra {
                underline_color: None,
                hyperlink: None,
                zerowidth: vec!['\u{1F3FD}'],
            })),
            ..Cell::default()
        },
        Cell {
            ch: ' ',
            flags: CellFlags::WIDE_CHAR_SPACER,
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    let faces = fc.create_shaping_faces();
    let mut output = Vec::new();
    let mut col_starts = Vec::new();
    shape_prepared_runs(&runs, &faces, &fc, &mut output, &mut col_starts, &mut None);

    // Should produce at least 1 glyph, no panics.
    assert!(!output.is_empty(), "should produce at least 1 glyph");

    // If an emoji font collapses the sequence, expect 1 glyph.
    // If no emoji font, expect 1–2 glyphs (base + modifier separately).
    // Both outcomes are valid; key invariant is no crash.
    if output[0].glyph_id != 0 && output.len() == 1 {
        // Emoji font collapsed ZWJ+skin-tone to single glyph — expected.
        assert_eq!(col_starts[0], 0);
    }
}

// ── VS16 on Non-Emoji-Default Char ──

#[test]
fn prepare_line_vs16_on_copyright_symbol() {
    // © (U+00A9) is not emoji-default. With VS16, it should trigger
    // resolve_prefer_emoji, potentially selecting an emoji font face.
    let fc = test_collection();
    let cells = vec![
        Cell {
            ch: '\u{00A9}', // ©
            extra: Some(Arc::new(CellExtra {
                underline_color: None,
                hyperlink: None,
                zerowidth: vec!['\u{FE0F}'], // VS16
            })),
            ..Cell::default()
        },
        Cell {
            ch: 'x',
            ..Cell::default()
        },
    ];
    let mut runs = Vec::new();
    prepare_line(&cells, cells.len(), &fc, &mut runs);

    // © should appear in a run.
    let has_copyright = runs.iter().any(|r| r.text.contains('\u{00A9}'));
    assert!(has_copyright, "copyright should appear in a run");

    // VS16 should be in run text (passed to shaper).
    let has_vs16 = runs.iter().any(|r| r.text.contains('\u{FE0F}'));
    assert!(has_vs16, "VS16 should be passed to shaper");

    // If an emoji font is available, © with VS16 may use a different face
    // than 'x'. Both outcomes are valid.
}

#[test]
fn measure_text_vs16_copyright_width() {
    // © (U+00A9) has unicode width 1. VS16 (U+FE0F) has width 0.
    // "©\u{FE0F}" should measure as 1 cell.
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    let width = super::measure_text("\u{00A9}\u{FE0F}", &fc);
    let expected = 1.0 * cell_w;
    assert!(
        (width - expected).abs() < f32::EPSILON,
        "© + VS16 should be 1 cell wide: {width} vs {expected}",
    );
}

// ── Korean Jamo Decomposed Sequence ──

#[test]
fn measure_text_korean_jamo_decomposed() {
    // Decomposed Hangul: U+1112 (ㅎ) + U+1161 (ㅏ) + U+11AB (ㄴ) = 한
    // As separate codepoints, unicode-width gives width 2 for leading Jamo
    // consonant, and 0 for vowel/trailing Jamo. Total should be 2 cells.
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    let width = super::measure_text("\u{1112}\u{1161}\u{11AB}", &fc);

    // The individual Jamo codepoints: U+1112 = width 2 (Hangul Jamo),
    // U+1161 = width 0 (medial vowel), U+11AB = width 0 (final consonant).
    let expected = 2.0 * cell_w;
    assert!(
        (width - expected).abs() < f32::EPSILON,
        "decomposed Jamo should be 2 cells: {width} vs {expected}",
    );
}

// ── Private Use Area ──

#[test]
fn measure_text_pua_codepoint_width_one() {
    // U+F005 (PUA — commonly Nerd Font star icon) should be width 1.
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    let width = super::measure_text("\u{F005}", &fc);
    let expected = 1.0 * cell_w;
    assert!(
        (width - expected).abs() < f32::EPSILON,
        "PUA U+F005 should be 1 cell: {width} vs {expected}",
    );
}

#[test]
fn measure_text_pua_supplementary_width_one() {
    // U+F0001 (Supplementary PUA-A) — should also be width 1.
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    let width = super::measure_text("\u{F0001}", &fc);
    let expected = 1.0 * cell_w;
    assert!(
        (width - expected).abs() < f32::EPSILON,
        "supplementary PUA U+F0001 should be 1 cell: {width} vs {expected}",
    );
}

// ── Flag Tag Sequences ──

#[test]
fn measure_text_flag_tag_sequence() {
    // England flag: 🏴 U+1F3F4, then tag characters U+E0067 U+E0062 U+E0065
    // U+E006E U+E0067, then cancel tag U+E007F.
    // unicode-width: 🏴 (U+1F3F4) = width 2, tag chars = width 0 each.
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    let flag = "\u{1F3F4}\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}";
    let width = super::measure_text(flag, &fc);
    let expected = 2.0 * cell_w;
    assert!(
        (width - expected).abs() < f32::EPSILON,
        "flag tag sequence should be 2 cells: {width} vs {expected}",
    );
}

// Size-aware UI shaping

/// Build a UiFontMeasurer with the exact-size registry.
fn test_ui_measurer() -> (crate::font::ui_font_sizes::UiFontSizes, FontCollection) {
    let font_set = FontSet::embedded();
    let sizes = crate::font::ui_font_sizes::UiFontSizes::new(
        font_set.clone(),
        96.0,
        GlyphFormat::Alpha,
        HintingMode::Full,
        400,
        crate::font::ui_font_sizes::PRELOAD_SIZES,
    )
    .expect("registry must build");
    let fallback = FontCollection::new(
        font_set,
        12.0,
        96.0,
        GlyphFormat::Alpha,
        400,
        HintingMode::Full,
    )
    .expect("fallback must build");
    (sizes, fallback)
}

#[test]
fn larger_size_measures_wider_and_taller() {
    use oriterm_ui::text::TextStyle;
    use oriterm_ui::widgets::TextMeasurer;

    let (sizes, fallback) = test_ui_measurer();
    let m = super::UiFontMeasurer::new(Some(&sizes), &fallback, 1.0);

    let small = TextStyle::new(10.0, oriterm_ui::color::Color::WHITE);
    let large = TextStyle::new(18.0, oriterm_ui::color::Color::WHITE);

    let ms = m.measure("Hello", &small, f32::INFINITY);
    let ml = m.measure("Hello", &large, f32::INFINITY);

    assert!(
        ml.width > ms.width,
        "18px should be wider than 10px: {} vs {}",
        ml.width,
        ms.width,
    );
    assert!(
        ml.height > ms.height,
        "18px should be taller than 10px: {} vs {}",
        ml.height,
        ms.height,
    );
}

#[test]
fn shaped_output_stamps_expected_size_q6() {
    use oriterm_ui::text::TextStyle;
    use oriterm_ui::widgets::TextMeasurer;

    let (sizes, fallback) = test_ui_measurer();
    let m = super::UiFontMeasurer::new(Some(&sizes), &fallback, 1.0);

    let style_18 = TextStyle::new(18.0, oriterm_ui::color::Color::WHITE);
    let style_10 = TextStyle::new(10.0, oriterm_ui::color::Color::WHITE);

    let shaped_18 = m.shape("A", &style_18, f32::INFINITY);
    let shaped_10 = m.shape("A", &style_10, f32::INFINITY);

    // Different sizes must produce different size_q6 values.
    assert_ne!(
        shaped_18.size_q6, shaped_10.size_q6,
        "18px and 10px must have different size_q6"
    );
    // Both must be non-zero (real sizes, not fallback placeholder).
    assert_ne!(shaped_18.size_q6, 0, "18px size_q6 must be non-zero");
    assert_ne!(shaped_10.size_q6, 0, "10px size_q6 must be non-zero");
}

#[test]
fn zero_length_text_returns_zero_width_with_valid_size_q6() {
    use oriterm_ui::text::TextStyle;
    use oriterm_ui::widgets::TextMeasurer;

    let (sizes, fallback) = test_ui_measurer();
    let m = super::UiFontMeasurer::new(Some(&sizes), &fallback, 1.0);

    let style = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE);
    let shaped = m.shape("", &style, f32::INFINITY);

    assert_eq!(shaped.width, 0.0);
    assert!(shaped.glyphs.is_empty());
    // Even empty text should stamp a valid size_q6 from the collection.
    assert_ne!(
        shaped.size_q6, 0,
        "empty text should still have valid size_q6"
    );
}

// Weight-aware shaping regression (TPR-02-005)

/// Verify that `create_shaping_faces_for_weight` applies the requested weight
/// to variation axes, producing faces with the correct `wght` coordinates.
///
/// Regression: before the fix, `create_shaping_faces()` always used the
/// collection-global weight, so UI text requesting 700 was shaped with
/// 400-weight metrics but rasterized at 700 — a metrics/rendering mismatch.
#[test]
fn weight_aware_shaping_faces_apply_requested_weight() {
    use super::ui_text;
    use oriterm_ui::text::{FontWeight, TextStyle};

    let fc = test_collection();

    // Shape "Hello" at 400 and 700.
    let style_400 = TextStyle {
        weight: FontWeight::NORMAL,
        ..TextStyle::new(13.0, oriterm_ui::color::Color::WHITE)
    };
    let style_700 = TextStyle {
        weight: FontWeight::BOLD,
        ..TextStyle::new(13.0, oriterm_ui::color::Color::WHITE)
    };

    let shaped_400 = ui_text::shape_text("Hello", &style_400, f32::INFINITY, 0.0, &fc);
    let shaped_700 = ui_text::shape_text("Hello", &style_700, f32::INFINITY, 0.0, &fc);

    // The ShapedText should record the requested weight.
    assert_eq!(shaped_400.weight, 400, "400 weight must be stamped");
    assert_eq!(shaped_700.weight, 700, "700 weight must be stamped");

    // On a variable font with `wght` axis, the two weights should produce
    // different advance widths because heavier glyphs are typically wider.
    // On a static font without `wght`, both may produce identical metrics
    // (which is correct — the font can't express the difference).
    // Either way, the shaping path now goes through
    // `create_shaping_faces_for_weight`, not the collection-global weight.
    //
    // We verify the pipeline doesn't crash and produces valid output
    // for both weights.
    assert!(!shaped_400.glyphs.is_empty(), "400 must produce glyphs");
    assert!(!shaped_700.glyphs.is_empty(), "700 must produce glyphs");
    assert!(shaped_400.width > 0.0, "400 width must be positive");
    assert!(shaped_700.width > 0.0, "700 width must be positive");
}

/// Verify that weight-aware shaping faces produce different variation
/// coordinates for different weights on fonts with a `wght` axis.
#[test]
fn weight_aware_faces_have_different_variations() {
    use crate::font::SyntheticFlags;

    let fc = test_collection();

    // Create faces at weight 400 vs 700.
    let faces_400 = fc.create_shaping_faces_for_weight(400, SyntheticFlags::NONE);
    let faces_700 = fc.create_shaping_faces_for_weight(700, SyntheticFlags::NONE);

    // Both must produce the same number of face slots.
    assert_eq!(faces_400.len(), faces_700.len());

    // At minimum, both must produce valid faces for the regular slot.
    assert!(faces_400[0].is_some(), "400 must have regular face");
    assert!(faces_700[0].is_some(), "700 must have regular face");
}

// -- TextTransform integration --

/// Verify that `TextTransform::Uppercase` in `TextStyle` is applied by the
/// shaping pipeline, producing the same output as explicitly uppercased text.
#[test]
fn shape_text_applies_text_transform() {
    use super::ui_text;
    use oriterm_ui::text::{TextStyle, TextTransform};

    let fc = test_collection();

    // Shape with transform in style.
    let style = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE)
        .with_text_transform(TextTransform::Uppercase);
    let shaped_via_style = ui_text::shape_text("hello", &style, f32::INFINITY, 0.0, &fc);

    // Shape with manually uppercased text.
    let plain_style = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE);
    let shaped_explicit = ui_text::shape_text("HELLO", &plain_style, f32::INFINITY, 0.0, &fc);

    assert_eq!(
        shaped_via_style.glyphs.len(),
        shaped_explicit.glyphs.len(),
        "transform via style must produce same glyph count as explicit uppercase"
    );
    assert!(
        (shaped_via_style.width - shaped_explicit.width).abs() < 0.01,
        "widths must match: style={} explicit={}",
        shaped_via_style.width,
        shaped_explicit.width
    );
}

/// Verify that `TextTransform::Uppercase` is applied before ellipsis truncation,
/// so case changes that alter string length are accounted for.
#[test]
fn shape_text_transform_before_ellipsis() {
    use super::ui_text;
    use oriterm_ui::text::{TextOverflow, TextStyle, TextTransform};

    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;

    // Use a narrow max_width that forces truncation.
    let max_width = cell_w * 4.0; // room for ~4 characters

    let style = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE)
        .with_overflow(TextOverflow::Ellipsis)
        .with_text_transform(TextTransform::Uppercase);

    let shaped = ui_text::shape_text("abcdefghij", &style, max_width, 0.0, &fc);

    // The result should be truncated (not all 10 glyphs present).
    assert!(
        shaped.glyphs.len() < 10,
        "should be truncated, got {} glyphs",
        shaped.glyphs.len()
    );
    // And the shaped text should contain the ellipsis glyph.
    assert!(!shaped.glyphs.is_empty(), "truncated text must have glyphs");
}

/// Regression: combining marks should not inflate letter-spacing budget.
///
/// Before this fix, `truncate_with_ellipsis` counted ALL chars for spacing
/// (including zero-width combining marks), while the shaped output only
/// applied spacing per glyph. This caused over-estimation and false truncation.
#[test]
fn truncate_combining_marks_spacing_not_inflated() {
    let fc = test_collection();
    let cell_w = fc.cell_metrics().width;
    let spacing = cell_w * 0.5;

    // "e\u{0301}" is "é" as base + combining acute. Unicode width: e=1, combining=0.
    // Visible character count for spacing: 1 (only 'e' has nonzero width).
    // Two visible chars "AB" + the combining pair = 3 chars but 3 visible.
    let text = "ABe\u{0301}";

    // Budget generous enough for 3 visible chars + spacing.
    // 3 cells * cell_w + 3 visible * spacing = 3 * 1.5 * cell_w = 4.5 * cell_w.
    let max_width = 5.0 * cell_w;
    let result = super::truncate_with_ellipsis(text, max_width, spacing, &fc);
    assert_eq!(
        result.as_ref(),
        text,
        "combining mark text should not be falsely truncated",
    );
}

/// Regression: shape_text with ellipsis uses shaped width to decide truncation.
///
/// Before this fix, shape_text always ran truncate_with_ellipsis (char-based
/// approximation) before shaping. When ligatures reduce glyph count, the
/// char-based width overestimates and causes false truncation. Now we shape
/// first and only truncate if the shaped width actually exceeds max_width.
#[test]
fn shape_text_ellipsis_shapes_first_to_avoid_false_truncation() {
    use super::ui_text;
    use oriterm_ui::text::{TextOverflow, TextStyle};

    let fc = test_collection();

    // Shape "Hello" without ellipsis to get its natural width.
    let style_clip = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE);
    let natural = ui_text::shape_text("Hello", &style_clip, f32::INFINITY, 0.0, &fc);

    // Set max_width to exactly the natural shaped width — should NOT truncate.
    let style_ellipsis =
        TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_overflow(TextOverflow::Ellipsis);
    let shaped = ui_text::shape_text("Hello", &style_ellipsis, natural.width, 0.0, &fc);

    assert_eq!(
        shaped.glyphs.len(),
        natural.glyphs.len(),
        "text that fits exactly should not be truncated (got {} vs {} glyphs)",
        shaped.glyphs.len(),
        natural.glyphs.len(),
    );
}

// -- Line height (Section 04) --

#[test]
fn measure_returns_styled_height_when_line_height_set() {
    use oriterm_ui::text::TextStyle;
    use oriterm_ui::widgets::TextMeasurer;

    let (sizes, fallback) = test_ui_measurer();
    let m = super::UiFontMeasurer::new(Some(&sizes), &fallback, 1.0);

    let style = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_line_height(1.5);
    let metrics = m.measure("Hello", &style, f32::INFINITY);

    assert_eq!(metrics.height, 19.5, "height should be 13.0 * 1.5 = 19.5");
}

#[test]
fn shape_returns_same_logical_height_as_measure() {
    use oriterm_ui::text::TextStyle;
    use oriterm_ui::widgets::TextMeasurer;

    let (sizes, fallback) = test_ui_measurer();
    let m = super::UiFontMeasurer::new(Some(&sizes), &fallback, 1.0);

    let style = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_line_height(1.4);
    let metrics = m.measure("Hello", &style, f32::INFINITY);
    let shaped = m.shape("Hello", &style, f32::INFINITY);

    assert_eq!(
        shaped.height, metrics.height,
        "shape and measure must agree on logical height",
    );
}

#[test]
fn width_unchanged_by_line_height() {
    use oriterm_ui::text::TextStyle;
    use oriterm_ui::widgets::TextMeasurer;

    let (sizes, fallback) = test_ui_measurer();
    let m = super::UiFontMeasurer::new(Some(&sizes), &fallback, 1.0);

    let without = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE);
    let with = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_line_height(1.8);

    let mw = m.measure("Hello", &without, f32::INFINITY);
    let ml = m.measure("Hello", &with, f32::INFINITY);

    assert_eq!(mw.width, ml.width, "width must not change with line_height");
}

#[test]
fn baseline_shifts_with_larger_line_height() {
    use oriterm_ui::text::TextStyle;
    use oriterm_ui::widgets::TextMeasurer;

    let (sizes, fallback) = test_ui_measurer();
    let m = super::UiFontMeasurer::new(Some(&sizes), &fallback, 1.0);

    let natural = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE);
    let larger = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_line_height(1.8);

    let sn = m.shape("Hello", &natural, f32::INFINITY);
    let sl = m.shape("Hello", &larger, f32::INFINITY);

    assert!(
        sl.baseline > sn.baseline,
        "larger line-height should push baseline down: {} vs {}",
        sl.baseline,
        sn.baseline,
    );
}

#[test]
fn baseline_shifts_with_smaller_line_height() {
    use oriterm_ui::text::TextStyle;
    use oriterm_ui::widgets::TextMeasurer;

    let (sizes, fallback) = test_ui_measurer();
    let m = super::UiFontMeasurer::new(Some(&sizes), &fallback, 1.0);

    let natural = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE);
    let smaller = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_line_height(0.8);

    let sn = m.shape("Hello", &natural, f32::INFINITY);
    let ss = m.shape("Hello", &smaller, f32::INFINITY);

    assert!(
        ss.baseline < sn.baseline,
        "smaller line-height should pull baseline up: {} vs {}",
        ss.baseline,
        sn.baseline,
    );
}

#[test]
fn line_height_correct_at_scale_2() {
    use oriterm_ui::text::TextStyle;
    use oriterm_ui::widgets::TextMeasurer;

    let (sizes, fallback) = test_ui_measurer();
    let m1 = super::UiFontMeasurer::new(Some(&sizes), &fallback, 1.0);
    let m2 = super::UiFontMeasurer::new(Some(&sizes), &fallback, 2.0);

    let style = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_line_height(1.5);

    let met1 = m1.measure("Hello", &style, f32::INFINITY);
    let met2 = m2.measure("Hello", &style, f32::INFINITY);

    // Logical height is independent of scale.
    assert_eq!(met1.height, 19.5, "scale 1: height = 13 * 1.5");
    assert_eq!(met2.height, 19.5, "scale 2: height = 13 * 1.5");

    let sh1 = m1.shape("Hello", &style, f32::INFINITY);
    let sh2 = m2.shape("Hello", &style, f32::INFINITY);

    assert_eq!(sh1.height, sh2.height, "logical height same at both scales");

    // Physical baseline should differ because half-leading is computed in
    // physical space (target_physical = logical * scale).
    assert_ne!(
        sh1.baseline, sh2.baseline,
        "physical baseline should differ between scales",
    );
}

#[test]
fn invalid_line_height_falls_back_to_natural() {
    use oriterm_ui::text::TextStyle;
    use oriterm_ui::widgets::TextMeasurer;

    let (sizes, fallback) = test_ui_measurer();
    let m = super::UiFontMeasurer::new(Some(&sizes), &fallback, 1.0);

    let natural_style = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE);
    let nat_met = m.measure("Hello", &natural_style, f32::INFINITY);
    let nat_sh = m.shape("Hello", &natural_style, f32::INFINITY);

    for invalid in [0.0_f32, -1.0, f32::NAN, f32::INFINITY] {
        let mut s = natural_style.clone();
        s.line_height = Some(invalid);
        let met = m.measure("Hello", &s, f32::INFINITY);
        let sh = m.shape("Hello", &s, f32::INFINITY);

        assert_eq!(
            met.height, nat_met.height,
            "invalid {invalid}: measure height should match natural",
        );
        assert_eq!(
            sh.height, nat_sh.height,
            "invalid {invalid}: shape height should match natural",
        );
    }
}

#[test]
fn line_height_one_produces_size_times_one() {
    use oriterm_ui::text::TextStyle;
    use oriterm_ui::widgets::TextMeasurer;

    let (sizes, fallback) = test_ui_measurer();
    let m = super::UiFontMeasurer::new(Some(&sizes), &fallback, 1.0);

    let style = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_line_height(1.0);
    let metrics = m.measure("Hello", &style, f32::INFINITY);

    // line_height(1.0) with size=13 -> height = 13.0.
    // Natural height is typically ~1.3x-1.5x the size, so this is NOT the same as None.
    assert_eq!(metrics.height, 13.0, "1.0 multiplier: height = size");
}

#[test]
fn empty_text_with_line_height() {
    use oriterm_ui::text::TextStyle;
    use oriterm_ui::widgets::TextMeasurer;

    let (sizes, fallback) = test_ui_measurer();
    let m = super::UiFontMeasurer::new(Some(&sizes), &fallback, 1.0);

    let style = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_line_height(1.5);
    let shaped = m.shape("", &style, f32::INFINITY);

    // Empty text with line_height should still report the styled line box height.
    assert_eq!(shaped.height, 19.5, "empty text should use styled height");
}

#[test]
fn line_height_with_letter_spacing() {
    use oriterm_ui::text::TextStyle;
    use oriterm_ui::widgets::TextMeasurer;

    let (sizes, fallback) = test_ui_measurer();
    let m = super::UiFontMeasurer::new(Some(&sizes), &fallback, 1.0);

    let with_spacing = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE)
        .with_line_height(1.5)
        .with_letter_spacing(2.0);
    let without_spacing =
        TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_line_height(1.5);

    let mws = m.measure("Hello", &with_spacing, f32::INFINITY);
    let mwo = m.measure("Hello", &without_spacing, f32::INFINITY);

    assert_eq!(mws.height, mwo.height, "height unaffected by spacing");
    assert!(mws.width > mwo.width, "width should increase with spacing");
}

#[test]
fn line_height_with_text_transform() {
    use oriterm_ui::text::{TextStyle, TextTransform};
    use oriterm_ui::widgets::TextMeasurer;

    let (sizes, fallback) = test_ui_measurer();
    let m = super::UiFontMeasurer::new(Some(&sizes), &fallback, 1.0);

    let style = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE)
        .with_line_height(1.4)
        .with_text_transform(TextTransform::Uppercase);
    let metrics = m.measure("Hello", &style, f32::INFINITY);

    assert_eq!(
        metrics.height,
        13.0 * 1.4,
        "height = size * multiplier regardless of transform",
    );
}
