//! vttest golden image tests for menus 3 (character sets) and 8 (VT102).

use super::{VtTestSession, vttest_available};
use crate::gpu::visual_regression::headless_env;

/// Run vttest menu 3 (character sets) and capture golden images.
///
/// Menu 3 has a sub-menu; we test sub-item 8 (VT100 Character Sets)
/// which exercises DEC Special Graphics line drawing.
fn run_menu3_golden(cols: u16, rows: u16) {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let mut s = VtTestSession::new(cols, rows);
    let label = format!("{}x{}", cols, rows);

    s.wait_for("Enter choice number", 5000);

    // Enter menu 3: Character Sets.
    s.send(b"3\r");
    s.wait_for("Menu 3", 3000);

    // Sub-item 8: VT100 Character Sets (DEC Special Graphics).
    s.send(b"8\r");

    let mut screen = 1;
    loop {
        let text = s.grid_text();
        if text.contains("Enter choice number") {
            break;
        }

        s.assert_golden(
            &format!("vttest_{label}_03_cs_{screen:02}"),
            &gpu,
            &pipelines,
            &mut renderer,
        );

        s.send(b"\r");
        screen += 1;
        if screen > 10 {
            break;
        }
    }
}

#[test]
fn vttest_golden_menu3_80x24() {
    if !vttest_available() {
        return;
    }
    run_menu3_golden(80, 24);
}

/// Run vttest menu 8 (VT102 insert/delete) and capture golden images.
fn run_menu8_golden(cols: u16, rows: u16) {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let mut s = VtTestSession::new(cols, rows);
    let label = format!("{}x{}", cols, rows);

    s.wait_for("Enter choice number", 5000);

    // Enter menu 8: VT102 features.
    s.send(b"8\r");

    let mut screen = 1;
    loop {
        let text = s.grid_text();
        if text.contains("Enter choice number") {
            break;
        }

        s.assert_golden(
            &format!("vttest_{label}_08_vt102_{screen:02}"),
            &gpu,
            &pipelines,
            &mut renderer,
        );

        s.send(b"\r");
        screen += 1;
        if screen > 20 {
            break;
        }
    }
}

#[test]
fn vttest_golden_menu8_80x24() {
    if !vttest_available() {
        return;
    }
    run_menu8_golden(80, 24);
}
