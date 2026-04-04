//! vttest menu 4: Double-size characters (DECDHL/DECDWL).
//!
//! Known limitation: DECDHL (double-height) and DECDWL (double-width) escape
//! sequences are NOT implemented in oriterm_core. These screens render at
//! normal size. Snapshots are captured as a baseline — when DECDHL/DECDWL
//! support is added, the snapshots will change to reflect correct rendering.

use super::session::{VtTestSession, vttest_available};

/// Run vttest menu 4 (double-size characters) at a given size.
///
/// Menu 4 tests DECDHL and DECDWL. Since these are unimplemented,
/// text renders at normal size. Snapshot-only — no structural assertions.
fn run_menu4_double_size(cols: u16, rows: u16) {
    let mut s = VtTestSession::new(cols, rows);
    let label = s.size_label();

    s.wait_for("Enter choice number", 5000);

    // Select menu 4.
    s.send(b"4\r");

    let mut screen = 1;
    loop {
        let text = s.grid_text();

        if text.contains("Enter choice number") {
            break;
        }

        insta::assert_snapshot!(format!("{label}_04_dblsize_{screen:02}"), text);

        s.send(b"\r");
        screen += 1;

        if screen > 20 {
            break;
        }
    }

    assert!(
        screen > 1,
        "{label}: menu 4 should have at least one screen"
    );
}

#[test]
fn vttest_menu4_80x24() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu4_double_size(80, 24);
}

#[test]
fn vttest_menu4_97x33() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu4_double_size(97, 33);
}

#[test]
fn vttest_menu4_120x40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu4_double_size(120, 40);
}
