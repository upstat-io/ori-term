//! vttest menu 2: Screen features (ED, EL, scroll regions, SGR attributes).

use super::session::{VtTestSession, vttest_available};

/// Run vttest menu 2 (screen features) at a given size, capturing all screens.
fn run_menu2_screen_features(cols: u16, rows: u16) {
    let mut s = VtTestSession::new(cols, rows);
    let label = s.size_label();

    // Wait for main menu to fully render.
    s.wait_for("Enter choice number", 5000);

    // Select menu item 2.
    s.send(b"2\r");

    // Walk through all sub-screens.
    let mut screen = 1;
    loop {
        let text = s.grid_text();

        if text.contains("Enter choice number") {
            break;
        }

        // Structural assertions for specific screens.
        match screen {
            11 => {
                assert!(
                    text.contains("bottom of the screen"),
                    "{label} screen 11: should contain 'bottom of the screen'"
                );
            }
            12 => {
                let first_line = text.lines().next().unwrap_or("");
                assert!(
                    first_line.contains("top of the screen"),
                    "{label} screen 12: first line should contain 'top of the screen', \
                     got: {first_line:?}"
                );
            }
            15 => {
                // SAVE/RESTORE cursor test: "5 x 4 A's filling the top left."
                let lines: Vec<&str> = text.lines().collect();
                for row in 0..4 {
                    assert!(
                        lines[row].starts_with("AAAAA"),
                        "{label} screen 15: row {row} should start with 'AAAAA', \
                         got: {:?}",
                        &lines[row][..10.min(lines[row].len())]
                    );
                }
            }
            _ => {}
        }

        insta::assert_snapshot!(format!("{label}_02_screen_{screen:02}"), text);

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
fn vttest_menu2_80x24() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu2_screen_features(80, 24);
}

#[test]
fn vttest_menu2_97x33() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu2_screen_features(97, 33);
}

#[test]
fn vttest_menu2_120x40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu2_screen_features(120, 40);
}
