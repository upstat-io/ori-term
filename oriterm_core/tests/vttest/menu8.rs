//! vttest menu 8: VT102 features — ICH (insert character), DCH (delete
//! character), IL (insert line), DL (delete line).

use super::session::{VtTestSession, grid_chars, vttest_available};

/// Structural assertions for VT102 menu 8 screens.
///
/// Some assertions are only valid at 80x24 where the VT102 accordion
/// test produces a predictable layout. At larger sizes, vttest may
/// fill more or fewer rows, changing the bottom-row content.
fn assert_vt102_screen_structure(grid: &[Vec<char>], _text: &str, screen: usize, label: &str) {
    let cols = grid.first().map_or(0, |r| r.len());
    let is_80x24 = cols == 80 && grid.len() == 24;
    match screen {
        // Screen 1: First-round accordion result — row 0 = all A's.
        1 => {
            assert!(
                grid[0].iter().all(|&c| c == 'A'),
                "{label} screen 1: top row should be all A's"
            );
        }
        // Screen 2: IL/DL accordion result — A's on top, X's on bottom.
        // Bottom row X's assertion is only reliable at 80x24 where the
        // accordion exactly fills the screen.
        2 => {
            assert!(
                grid[0].iter().all(|&c| c == 'A'),
                "{label} screen 2: top row should be all A's"
            );
            if is_80x24 {
                let last = grid.len() - 1;
                assert!(
                    grid[last].iter().all(|&c| c == 'X'),
                    "{label} screen 2: bottom row should be all X's"
                );
            }
        }
        // Screen 3: Insert Mode — first char 'A', last non-space 'B'.
        3 => {
            assert_eq!(
                grid[0][0], 'A',
                "{label} screen 3: first char should be 'A'"
            );
            let last_nonspace = grid[0].iter().rposition(|&c| c != ' ');
            if let Some(pos) = last_nonspace {
                assert_eq!(
                    grid[0][pos], 'B',
                    "{label} screen 3: last non-space char should be 'B'"
                );
            }
        }
        // Screen 4: Delete Character — row 0 starts with "AB".
        4 => {
            assert_eq!(
                grid[0][0], 'A',
                "{label} screen 4: row 0 col 0 should be 'A'"
            );
            assert_eq!(
                grid[0][1], 'B',
                "{label} screen 4: row 0 col 1 should be 'B'"
            );
        }
        // Screen 5: DCH stagger — each row shorter by 1 on the right.
        5 => {
            let len0 = grid[0].iter().rposition(|&c| c != ' ').unwrap_or(0);
            let len1 = grid[1].iter().rposition(|&c| c != ' ').unwrap_or(0);
            assert!(
                len0 > len1,
                "{label} screen 5: row 0 should be longer than row 1 \
                 (stagger), got len0={len0}, len1={len1}"
            );
        }
        // Screen 6: ICH stagger — row 0 starts with 'A'.
        6 => {
            assert_eq!(
                grid[0][0], 'A',
                "{label} screen 6: row 0 col 0 should be 'A'"
            );
        }
        // Screen 7: ICH ANSI test — informational text.
        7 => {
            assert_eq!(
                grid[0][0], 'I',
                "{label} screen 7: row 0 should start with 'I' (informational text)"
            );
        }
        // Screen 8: Second-round accordion (with scroll region) — row 0 = all A's.
        8 => {
            assert!(
                grid[0].iter().all(|&c| c == 'A'),
                "{label} screen 8: top row should be all A's"
            );
        }
        // Screen 9: Second-round IL/DL result (with scroll region) — row 0 = all A's.
        9 => {
            assert!(
                grid[0].iter().all(|&c| c == 'A'),
                "{label} screen 9: top row should be all A's"
            );
        }
        // Screen 10: Second-round Insert Mode — first char 'A', last non-space 'B'.
        10 => {
            assert_eq!(
                grid[0][0], 'A',
                "{label} screen 10: first char should be 'A'"
            );
            let last_nonspace = grid[0].iter().rposition(|&c| c != ' ');
            if let Some(pos) = last_nonspace {
                assert_eq!(
                    grid[0][pos], 'B',
                    "{label} screen 10: last non-space char should be 'B'"
                );
            }
        }
        // Screen 11: Second-round Delete Character — row 0 starts with "AB".
        11 => {
            assert_eq!(
                grid[0][0], 'A',
                "{label} screen 11: row 0 col 0 should be 'A'"
            );
            assert_eq!(
                grid[0][1], 'B',
                "{label} screen 11: row 0 col 1 should be 'B'"
            );
        }
        // Screen 12: Second-round DCH stagger — row 0 longer than row 1.
        12 => {
            let len0 = grid[0].iter().rposition(|&c| c != ' ').unwrap_or(0);
            let len1 = grid[1].iter().rposition(|&c| c != ' ').unwrap_or(0);
            assert!(
                len0 > len1,
                "{label} screen 12: row 0 should be longer than row 1 \
                 (stagger), got len0={len0}, len1={len1}"
            );
        }
        // Screen 13: Second-round ICH stagger (with scroll region) — row 0 starts with 'A'.
        13 => {
            assert_eq!(
                grid[0][0], 'A',
                "{label} screen 13: row 0 col 0 should be 'A'"
            );
        }
        // Screen 14: Second-round ICH ANSI test — informational text.
        14 => {
            assert_eq!(
                grid[0][0], 'I',
                "{label} screen 14: row 0 should start with 'I' (informational text)"
            );
        }
        // All 14 screens are covered above. This catch-all exists only for
        // safety if vttest ever adds new screens.
        _ => {}
    }
}

/// Run vttest menu 8 (VT102 features) at a given size, capturing all screens.
///
/// Menu 8 tests ICH (insert character), DCH (delete character),
/// IL (insert line), and DL (delete line).
fn run_menu8_vt102(cols: u16, rows: u16) {
    let mut s = VtTestSession::new(cols, rows);
    let label = s.size_label();

    // Wait for main menu.
    s.wait_for("Enter choice number", 5000);

    // Select menu 8: VT102 features.
    s.send(b"8\r");

    // Walk through all sub-screens.
    let mut screen = 1;
    loop {
        let text = s.grid_text();

        if text.contains("Enter choice number") {
            break;
        }

        // Structural assertions for ICH/DCH/IL/DL screens.
        let grid = grid_chars(&s.term);
        assert_vt102_screen_structure(&grid, &text, screen, &label);

        insta::assert_snapshot!(format!("{label}_08_vt102_{screen:02}"), text);

        s.send(b"\r");
        screen += 1;

        if screen > 20 {
            break;
        }
    }

    assert!(
        screen > 1,
        "{label}: menu 8 should have at least one screen"
    );
}

#[test]
fn vttest_menu8_80x24() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu8_vt102(80, 24);
}

#[test]
fn vttest_menu8_97x33() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu8_vt102(97, 33);
}

#[test]
fn vttest_menu8_120x40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu8_vt102(120, 40);
}
