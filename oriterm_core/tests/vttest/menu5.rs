//! vttest menu 5: Keyboard tests.
//!
//! Menu 5 has sub-menus that test LED states, auto-repeat, and key reporting.
//! Some sub-tests are automatable; interactive tests requiring human judgment
//! on physical key mapping are skipped.

use super::session::{PtySession, vttest_available, walk_vttest_screens};

/// Run vttest menu 5 (keyboard) at a given size.
///
/// Menu 5 has a sub-menu. We automate:
/// - Sub-item 1: LED tests (verify CSI responses)
/// - Sub-item 2: Auto-repeat
/// Interactive tests that need human judgment are skipped.
fn run_menu5_keyboard(cols: u16, rows: u16) {
    let mut s = PtySession::spawn_vttest(cols, rows);
    let label = s.size_label();

    s.wait_for("Enter choice number", 5000);

    // Select menu 5.
    s.send(b"5\r");

    // Wait for the keyboard sub-menu.
    s.wait_for("Menu 5", 3000);
    insta::assert_snapshot!(format!("{label}_05_menu"), s.grid_text());

    // Sub-item 1: LED tests.
    s.send(b"1\r");
    walk_vttest_screens(&mut s, 10, &["Menu 5"], |_session, text, screen| {
        insta::assert_snapshot!(format!("{label}_05_led_{screen:02}"), text);
    });

    // Sub-item 2: Auto-repeat. The closure interleaves `a` + sleep + drain
    // BEFORE returning; the walker sends `\r` after.
    s.send(b"2\r");
    walk_vttest_screens(&mut s, 5, &["Menu 5"], |session, text, screen| {
        insta::assert_snapshot!(format!("{label}_05_repeat_{screen:02}"), text);
        session.send(b"a");
        std::thread::sleep(std::time::Duration::from_millis(500));
        session.drain();
    });

    // Exit sub-menu back to main menu.
    s.send(b"0\r");
}

#[test]
fn vttest_menu5_80x24() {
    if !vttest_available() {
        eprintln!("SKIP: vttest not installed");
        return;
    }
    run_menu5_keyboard(80, 24);
}

#[test]
fn vttest_menu5_97x33() {
    if !vttest_available() {
        eprintln!("SKIP: vttest not installed");
        return;
    }
    run_menu5_keyboard(97, 33);
}

#[test]
fn vttest_menu5_120x40() {
    if !vttest_available() {
        eprintln!("SKIP: vttest not installed");
        return;
    }
    run_menu5_keyboard(120, 40);
}
