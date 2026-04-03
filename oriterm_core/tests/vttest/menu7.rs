//! vttest menu 7: VT52 compatibility mode.
//!
//! Known limitation: VT52 mode is NOT implemented in oriterm_core.
//! The VT52 escape sequences are ignored, so these screens will not
//! render correctly. Tests verify navigation doesn't crash but do NOT
//! assert on screen content — the output is non-deterministic because
//! VT52 escape sequences are not processed, leading to timing-dependent
//! rendering artifacts.

use super::session::{VtTestSession, vttest_available};

/// Run vttest menu 7 (VT52 mode) at a given size.
///
/// VT52 mode is unimplemented — no content assertions. Tests verify
/// that vttest navigation works without crashing.
fn run_menu7_vt52(cols: u16, rows: u16) {
    let mut s = VtTestSession::new(cols, rows);
    let label = s.size_label();

    s.wait_for("Enter choice number", 5000);

    // Select menu 7.
    s.send(b"7\r");

    let mut screen = 1;
    loop {
        let text = s.grid_text();

        if text.contains("Enter choice number") {
            break;
        }

        // No snapshot assertions — VT52 output is non-deterministic
        // because the escape sequences are unimplemented.

        s.send(b"\r");
        screen += 1;

        if screen > 20 {
            break;
        }
    }

    assert!(
        screen > 1,
        "{label}: menu 7 should have at least one screen"
    );
}

#[test]
fn vttest_menu7_80x24() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu7_vt52(80, 24);
}

#[test]
fn vttest_menu7_97x33() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu7_vt52(97, 33);
}

#[test]
fn vttest_menu7_120x40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu7_vt52(120, 40);
}
