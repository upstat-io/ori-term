//! vttest menu 6: Terminal reports — Device Attributes (DA), Device Status
//! Report (DSR), cursor position reporting, and mode queries (DECRQM).
//!
//! Menu 6 has a sub-menu structure. We automate individual sub-items
//! and capture their output with structural assertions verifying DA/DSR
//! responses appear in the terminal output.

use super::session::{VtTestSession, vttest_available};

/// Walk screens for a menu 6 sub-item until returning to the sub-menu.
/// Returns (screen_count, all_screen_text) for structural assertions.
fn walk_menu6_subscreens(s: &mut VtTestSession, label: &str, tag: &str) -> (usize, String) {
    let mut screen = 1;
    let mut all_text = String::new();
    loop {
        let text = s.grid_text();

        // Return to sub-menu or main menu means all screens captured.
        if text.contains("Enter choice number") {
            break;
        }

        // Verify screens are non-blank — report tests should produce output.
        let has_content = text.lines().any(|line| line.trim().len() > 1);
        assert!(
            has_content,
            "{label} menu 6 {tag} screen {screen}: report screen should not be blank"
        );

        all_text.push_str(&text);
        insta::assert_snapshot!(format!("{label}_06_{tag}_{screen:02}"), text);

        s.send(b"\r");
        screen += 1;

        if screen > 15 {
            break;
        }
    }
    (screen - 1, all_text)
}

/// Run vttest menu 6 (terminal reports) at a given size.
///
/// Menu 6 has a sub-menu. We automate each sub-item that tests DA/DSR/DECRQM
/// responses. vttest sends queries and displays the terminal's responses.
/// Structural assertions verify that DA and DSR responses appear in the output.
fn run_menu6_reports(cols: u16, rows: u16) {
    let mut s = VtTestSession::new(cols, rows);
    let label = s.size_label();

    s.wait_for("Enter choice number", 5000);

    // Select menu 6.
    s.send(b"6\r");

    // Menu 6 has a sub-menu structure.
    let text = s.grid_text();
    if text.contains("Menu 6") || text.contains("menu 6") {
        insta::assert_snapshot!(format!("{label}_06_menu"), text);

        let mut saw_da_response = false;
        let mut saw_dsr_response = false;
        let mut total_screens = 0;

        // Try sub-items 1 through 6 (common report tests).
        for item in 1..=6 {
            let item_text = s.grid_text();
            if item_text.contains("Enter choice number") {
                s.send(format!("{item}\r").as_bytes());

                let after = s.grid_text();
                if after.contains("Enter choice number") {
                    continue;
                }

                let (count, sub_text) =
                    walk_menu6_subscreens(&mut s, &label, &format!("sub{item}"));
                total_screens += count;

                // DA response: vttest echoes "Report is: ... VT" or "what are you".
                if sub_text.contains("what are you") || sub_text.contains("VT") {
                    saw_da_response = true;
                }

                // DSR response: vttest echoes "TERMINAL OK" or "cursor position".
                if sub_text.contains("TERMINAL OK") || sub_text.contains("cursor position") {
                    saw_dsr_response = true;
                }
            }
        }

        // Exit sub-menu.
        s.send(b"0\r");

        // Structural assertions: DA and DSR responses must appear.
        assert!(
            saw_da_response,
            "{label}: menu 6 should display Device Attributes (DA) response"
        );
        assert!(
            saw_dsr_response,
            "{label}: menu 6 should display Device Status Report (DSR) response"
        );
        assert!(
            total_screens >= 3,
            "{label}: menu 6 should exercise at least 3 report screens, got {total_screens}"
        );
    } else {
        // No sub-menu — screens run directly.
        let mut screen = 1;
        loop {
            let t = s.grid_text();
            if t.contains("Enter choice number") {
                break;
            }

            insta::assert_snapshot!(format!("{label}_06_report_{screen:02}"), t);

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
