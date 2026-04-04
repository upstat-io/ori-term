//! vttest menu 3: Character sets — DEC Special Graphics, UK National,
//! US ASCII, G0/G1 designation, SO/SI.

use std::collections::HashSet;

use super::session::{VtTestSession, grid_chars, vttest_available};

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

/// Walk sub-screens of a vttest menu-3 sub-item, returning the number
/// of screens captured. Applies line-drawing structural assertions.
fn walk_menu3_subscreens(
    s: &mut VtTestSession,
    label: &str,
    sub_item: &str,
    tag: &str,
    check_line_drawing: bool,
) -> (usize, bool) {
    let mut screen = 1;
    let mut saw_line_drawing = false;
    loop {
        let text = s.grid_text();

        // Return to sub-menu means all screens captured.
        if text.contains("Enter choice number") {
            break;
        }

        if check_line_drawing {
            let grid = grid_chars(&s.term);
            let has_drawing = grid
                .iter()
                .any(|row| row.iter().any(|ch| LINE_DRAWING_CHARS.contains(ch)));
            if has_drawing {
                saw_line_drawing = true;
                assert_has_line_drawing_chars(
                    &grid,
                    3,
                    &format!("{label} {sub_item} screen {screen}"),
                );
            }
        }

        insta::assert_snapshot!(format!("{label}_03_{tag}_{screen:02}"), text);

        s.send(b"\r");
        screen += 1;

        if screen > 20 {
            break;
        }
    }
    (screen - 1, saw_line_drawing)
}

/// Run vttest menu 3 (character sets) at a given size, capturing all screens.
///
/// Menu 3 has a sub-menu. We test sub-items 8 (VT100 character sets)
/// and 9 (Shift In/Shift Out).
fn run_menu3_character_sets(cols: u16, rows: u16) {
    let mut s = VtTestSession::new(cols, rows);
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
    let (count_8, saw_drawing) = walk_menu3_subscreens(&mut s, &label, "sub8", "vt100cs", true);
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
    let (count_9, _) = walk_menu3_subscreens(&mut s, &label, "sub9", "siso", false);
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
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu3_character_sets(80, 24);
}

#[test]
fn vttest_menu3_97x33() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu3_character_sets(97, 33);
}

#[test]
fn vttest_menu3_120x40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu3_character_sets(120, 40);
}
