//! vttest menu 1: Cursor movement tests, border fill assertions, and
//! DECCOLM column mode verification.

use super::session::{VtTestSession, grid_chars, vttest_available};

/// Run vttest menu 1 (cursor movement) at a given size, capturing all screens.
fn run_menu1_cursor_movement(cols: u16, rows: u16) {
    let mut s = VtTestSession::new(cols, rows);
    let label = s.size_label();

    // Wait for main menu to fully render.
    s.wait_for("Enter choice number", 5000);
    insta::assert_snapshot!(format!("{label}_00_main_menu"), s.grid_text());

    // Select menu item 1.
    s.send(b"1\r");

    // Walk through all sub-screens.
    let mut screen = 1;
    loop {
        let text = s.grid_text();

        if text.contains("Enter choice number") {
            break;
        }

        insta::assert_snapshot!(format!("{label}_01_cursor_{screen:02}"), text);

        s.send(b"\r");
        screen += 1;

        if screen > 20 {
            break;
        }
    }

    assert!(
        screen > 1,
        "{label}: should have captured at least one screen"
    );
}

#[test]
fn vttest_menu1_80x24() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu1_cursor_movement(80, 24);
}

#[test]
fn vttest_menu1_97x33() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu1_cursor_movement(97, 33);
}

#[test]
fn vttest_menu1_120x40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu1_cursor_movement(120, 40);
}

// Border fill structural assertions.

/// Verify the vttest screen-01 border fills the entire terminal.
///
/// Expected pattern for a `cols x rows` terminal:
/// ```text
/// Row 0:        * * * * ... * * * *     (all `*`, width = cols)
/// Row 1:        * + + + ... + + + *     (`*` edges, `+` interior)
/// Rows 2..R-3:  * +             + *     (`*` col 0, `+` col 1, `+` col C-2, `*` col C-1)
/// Row R-2:      * + + + ... + + + *     (same as row 1)
/// Row R-1:      * * * * ... * * * *     (same as row 0)
/// ```
fn assert_border_fills_terminal(grid: &[Vec<char>], cols: usize, rows: usize) {
    assert_eq!(grid.len(), rows, "grid should have {rows} rows");
    for row in grid {
        assert_eq!(row.len(), cols, "each row should have {cols} columns");
    }

    // Row 0: all `*`.
    for (c, &ch) in grid[0].iter().enumerate() {
        assert_eq!(ch, '*', "row 0, col {c}: expected '*', got '{ch}'");
    }

    // Row rows-1: all `*`.
    let last = rows - 1;
    for (c, &ch) in grid[last].iter().enumerate() {
        assert_eq!(ch, '*', "row {last}, col {c}: expected '*', got '{ch}'");
    }

    // Row 1: `*` at edges, `+` in between.
    assert_eq!(grid[1][0], '*', "row 1, col 0: expected '*'");
    assert_eq!(
        grid[1][cols - 1],
        '*',
        "row 1, col {}: expected '*'",
        cols - 1
    );
    for c in 1..cols - 1 {
        assert_eq!(
            grid[1][c], '+',
            "row 1, col {c}: expected '+', got '{}'",
            grid[1][c]
        );
    }

    // Row rows-2: `*` at edges, `+` in between.
    let pen = rows - 2;
    assert_eq!(grid[pen][0], '*', "row {pen}, col 0: expected '*'");
    assert_eq!(
        grid[pen][cols - 1],
        '*',
        "row {pen}, col {}: expected '*'",
        cols - 1
    );
    for c in 1..cols - 1 {
        assert_eq!(
            grid[pen][c], '+',
            "row {pen}, col {c}: expected '+', got '{}'",
            grid[pen][c]
        );
    }

    // Interior rows 2..rows-3: border characters at edges.
    for r in 2..rows - 2 {
        assert_eq!(
            grid[r][0], '*',
            "row {r}, col 0: expected '*', got '{}'",
            grid[r][0]
        );
        assert_eq!(
            grid[r][1], '+',
            "row {r}, col 1: expected '+', got '{}'",
            grid[r][1]
        );
        assert_eq!(
            grid[r][cols - 2],
            '+',
            "row {r}, col {}: expected '+', got '{}'",
            cols - 2,
            grid[r][cols - 2]
        );
        assert_eq!(
            grid[r][cols - 1],
            '*',
            "row {r}, col {}: expected '*', got '{}'",
            cols - 1,
            grid[r][cols - 1]
        );
    }
}

/// Navigate vttest to screen 01 (the border test) and return the grid.
fn capture_border_screen(cols: u16, rows: u16) -> Vec<Vec<char>> {
    let mut s = VtTestSession::new(cols, rows);

    // Wait for main menu to fully render.
    s.wait_for("Enter choice number", 5000);

    // Select menu 1, wait for first sub-screen.
    s.send(b"1\r");

    grid_chars(&s.term)
}

#[test]
fn vttest_border_fills_80x24() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    let grid = capture_border_screen(80, 24);
    assert_border_fills_terminal(&grid, 80, 24);
}

#[test]
fn vttest_border_fills_97x33() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    let grid = capture_border_screen(97, 33);
    assert_border_fills_terminal(&grid, 97, 33);
}

#[test]
fn vttest_border_fills_120x40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    let grid = capture_border_screen(120, 40);
    assert_border_fills_terminal(&grid, 120, 40);
}

// DECCOLM: screen 02 is the 132-column version of the border.
//
// vttest menu 1 draws the border twice: pass 0 at min_cols (screen 01)
// and pass 1 at max_cols with DECCOLM set (screen 02). DECCOLM does NOT
// resize the grid (design decision: reflow at current width). Screen 02
// content designed for 132 cols wraps at the current width.

/// Capture screen 02 (132-col pass) from menu 1 and verify side effects.
fn capture_deccolm_screen(cols: u16, rows: u16) -> Vec<Vec<char>> {
    let mut s = VtTestSession::new(cols, rows);
    s.wait_for("Enter choice number", 5000);
    s.send(b"1\r");

    // Screen 01 (min_cols border) — skip it.
    s.send(b"\r");

    // Screen 02 (max_cols=132 border after DECCOLM set).
    grid_chars(&s.term)
}

#[test]
fn vttest_deccolm_resizes_to_132_with_mode_40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    let grid = capture_deccolm_screen(80, 24);
    // Mode 40 is enabled in the vttest session setup, so DECCOLM
    // resizes the grid to 132 columns.
    assert_eq!(
        grid[0].len(),
        132,
        "DECCOLM should resize grid to 132 columns when Mode 40 is enabled"
    );
}
