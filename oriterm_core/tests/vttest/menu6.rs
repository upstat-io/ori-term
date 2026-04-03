//! vttest menu 6: Terminal reports — Device Attributes (DA), Device Status
//! Report (DSR), cursor position reporting, and mode queries (DECRQM).
//!
//! Menu 6 has a sub-menu structure. We automate individual sub-items
//! and capture their output.

use super::session::{VtTestSession, vttest_available};

/// Walk screens for a menu 6 sub-item until returning to the sub-menu.
fn walk_menu6_subscreens(s: &mut VtTestSession, label: &str, tag: &str) -> usize {
    let mut screen = 1;
    loop {
        let text = s.grid_text();

        // Return to sub-menu or main menu means all screens captured.
        if text.contains("Enter choice number") {
            break;
        }

        insta::assert_snapshot!(format!("{label}_06_{tag}_{screen:02}"), text);

        s.send(b"\r");
        screen += 1;

        if screen > 15 {
            break;
        }
    }
    screen - 1
}

/// Run vttest menu 6 (terminal reports) at a given size.
///
/// Menu 6 has a sub-menu. We automate each sub-item that tests DA/DSR/DECRQM
/// responses. vttest sends queries and displays the terminal's responses.
fn run_menu6_reports(cols: u16, rows: u16) {
    let mut s = VtTestSession::new(cols, rows);
    let label = s.size_label();

    s.wait_for("Enter choice number", 5000);

    // Select menu 6.
    s.send(b"6\r");

    // Menu 6 may have a sub-menu or run screens directly.
    // Check for sub-menu prompt.
    let text = s.grid_text();
    if text.contains("Menu 6") || text.contains("menu 6") {
        // Sub-menu structure — capture the menu screen.
        insta::assert_snapshot!(format!("{label}_06_menu"), text);

        // Try sub-items 1 through 6 (common report tests).
        for item in 1..=6 {
            let item_text = s.grid_text();
            if item_text.contains("Enter choice number") {
                // We're at the sub-menu. Try entering this sub-item.
                s.send(format!("{item}\r").as_bytes());

                let after = s.grid_text();
                if after.contains("Enter choice number") {
                    // Sub-item didn't produce any screens. Might be invalid.
                    continue;
                }

                let count = walk_menu6_subscreens(&mut s, &label, &format!("sub{item}"));
                if count == 0 {
                    continue;
                }
            }
        }

        // Exit sub-menu.
        s.send(b"0\r");
    } else {
        // No sub-menu — screens run directly. Walk them.
        let mut screen = 1;
        loop {
            let text = s.grid_text();
            if text.contains("Enter choice number") {
                break;
            }

            insta::assert_snapshot!(format!("{label}_06_report_{screen:02}"), text);

            s.send(b"\r");
            screen += 1;

            if screen > 20 {
                break;
            }
        }
    }
}

#[test]
fn vttest_menu6_80x24() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu6_reports(80, 24);
}

#[test]
fn vttest_menu6_97x33() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu6_reports(97, 33);
}

#[test]
fn vttest_menu6_120x40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu6_reports(120, 40);
}
