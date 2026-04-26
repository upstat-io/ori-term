//! vttest menu 3: Character sets — DEC Special Graphics, UK National,
//! US ASCII, G0/G1 designation, SO/SI.

use std::collections::HashSet;

use super::session::{PtySession, vttest_available, walk_vttest_screens};

/// DEC Special Graphics box-drawing characters used by vttest.
const LINE_DRAWING_CHARS: &[char] = &[
    '\u{250C}', // top-left corner
    '\u{2510}', // top-right corner
    '\u{2514}', // bottom-left corner
    '\u{2518}', // bottom-right corner
    '\u{2500}', // horizontal line
    '\u{2502}', // vertical line
    '\u{251C}', // T-junction right
    '\u{2524}', // T-junction left
    '\u{252C}', // T-junction down
    '\u{2534}', // T-junction up
    '\u{253C}', // cross
];

/// Check that the grid contains at least `min_count` distinct
/// DEC Special Graphics line-drawing characters.
pub fn assert_has_line_drawing_chars(grid: &[Vec<char>], min_count: usize, context: &str) {
    let mut found: HashSet<char> = HashSet::new();
    for row in grid {
        for &ch in row {
            if LINE_DRAWING_CHARS.contains(&ch) {
                found.insert(ch);
            }
        }
    }
    assert!(
        found.len() >= min_count,
        "{context}: expected at least {min_count} distinct line-drawing chars, \
         found {}: {found:?}",
        found.len()
    );
}

/// Run vttest menu 3 (character sets) at a given size, capturing all screens.
///
/// Menu 3 has a sub-menu. We test sub-items 8 (VT100 character sets)
/// and 9 (Shift In/Shift Out).
fn run_menu3_character_sets(cols: u16, rows: u16) {
    let mut s = PtySession::spawn_vttest(cols, rows);
    let label = s.size_label();

    // Wait for main menu.
    s.wait_for("Enter choice number", 5000);

    // Select menu 3: Character Sets.
    s.send(b"3\r");

    // Wait for sub-menu.
    s.wait_for("Menu 3", 3000);
    insta::assert_snapshot!(format!("{label}_03_menu"), s.grid_text());

    // Sub-item 8: Test VT100 Character Sets (DEC Special Graphics).
    s.send(b"8\r");
    let mut saw_drawing = false;
    let count_8 = walk_vttest_screens(&mut s, 20, &[], |session, text, screen| {
        let grid = session.grid_chars();
        let has_drawing = grid
            .iter()
            .any(|row| row.iter().any(|ch| LINE_DRAWING_CHARS.contains(ch)));
        if has_drawing {
            saw_drawing = true;
            assert_has_line_drawing_chars(&grid, 3, &format!("{label} sub8 screen {screen}"));
        }
        insta::assert_snapshot!(format!("{label}_03_vt100cs_{screen:02}"), text);
    });
    assert!(
        count_8 > 0,
        "{label}: sub-item 8 should have at least one screen"
    );
    assert!(
        saw_drawing,
        "{label}: VT100 Character Sets should contain DEC Special Graphics line-drawing characters"
    );

    // Sub-item 9: Test Shift In/Shift Out (SI/SO).
    s.send(b"9\r");
    let count_9 = walk_vttest_screens(&mut s, 20, &[], |_session, text, screen| {
        insta::assert_snapshot!(format!("{label}_03_siso_{screen:02}"), text);
    });
    assert!(
        count_9 > 0,
        "{label}: sub-item 9 should have at least one screen"
    );

    // Exit sub-menu back to main menu.
    s.send(b"0\r");
}

#[test]
fn vttest_menu3_80x24() {
    if !vttest_available() {
        eprintln!("SKIP: vttest not installed");
        return;
    }
    run_menu3_character_sets(80, 24);
}

#[test]
fn vttest_menu3_97x33() {
    if !vttest_available() {
        eprintln!("SKIP: vttest not installed");
        return;
    }
    run_menu3_character_sets(97, 33);
}

#[test]
fn vttest_menu3_120x40() {
    if !vttest_available() {
        eprintln!("SKIP: vttest not installed");
        return;
    }
    run_menu3_character_sets(120, 40);
}
