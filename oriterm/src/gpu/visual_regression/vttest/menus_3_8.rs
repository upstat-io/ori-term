//! vttest golden image tests for menus 3-8.

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

// Menu 4: Double-size characters (DECDHL/DECDWL — unimplemented, baseline only).

/// Run vttest menu 4 and capture golden images.
///
/// DECDHL/DECDWL are not implemented — text renders at normal size.
/// Golden images serve as a baseline for when support is added.
fn run_menu4_golden(cols: u16, rows: u16) {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let mut s = VtTestSession::new(cols, rows);
    let label = format!("{}x{}", cols, rows);

    s.wait_for("Enter choice number", 5000);
    s.send(b"4\r");

    let mut screen = 1;
    loop {
        let text = s.grid_text();
        if text.contains("Enter choice number") {
            break;
        }

        s.assert_golden(
            &format!("vttest_{label}_04_dblsize_{screen:02}"),
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
fn vttest_golden_menu4_80x24() {
    if !vttest_available() {
        return;
    }
    run_menu4_golden(80, 24);
}

// Menu 6: Terminal reports (DA, DSR, DECRQM).

/// Run vttest menu 6 and capture golden images.
///
/// Menu 6 has a sub-menu. We enter each sub-item and capture screens.
fn run_menu6_golden(cols: u16, rows: u16) {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let mut s = VtTestSession::new(cols, rows);
    let label = format!("{}x{}", cols, rows);

    s.wait_for("Enter choice number", 5000);
    s.send(b"6\r");

    // Check for sub-menu.
    let text = s.grid_text();
    if text.contains("Menu 6") || text.contains("menu 6") {
        s.assert_golden(
            &format!("vttest_{label}_06_menu"),
            &gpu,
            &pipelines,
            &mut renderer,
        );

        // Walk sub-items 1-6.
        for item in 1..=6 {
            let item_text = s.grid_text();
            if !item_text.contains("Enter choice number") {
                continue;
            }
            s.send(format!("{item}\r").as_bytes());

            let mut screen = 1;
            loop {
                let t = s.grid_text();
                if t.contains("Enter choice number") {
                    break;
                }
                s.assert_golden(
                    &format!("vttest_{label}_06_sub{item}_{screen:02}"),
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
        s.send(b"0\r");
    }
}

#[test]
fn vttest_golden_menu6_80x24() {
    if !vttest_available() {
        return;
    }
    run_menu6_golden(80, 24);
}

// Menu 7: VT52 mode (unimplemented — navigation-only, no golden images).
//
// VT52 escape sequences are not processed, so the rendered output is
// non-deterministic (timing-dependent garbage). Golden image comparison
// would be inherently flaky. The text-based test in
// `oriterm_core/tests/vttest/menu7.rs` already verifies navigation
// doesn't crash. Menu 7 is excluded from the conformance pass rate.

/// Run vttest menu 7 and verify navigation completes without crash.
///
/// No golden image assertions — VT52 output is non-deterministic.
fn run_menu7_golden(cols: u16, rows: u16) {
    let mut s = VtTestSession::new(cols, rows);

    s.wait_for("Enter choice number", 5000);
    s.send(b"7\r");

    let mut screen = 1;
    loop {
        let text = s.grid_text();
        if text.contains("Enter choice number") {
            break;
        }

        s.send(b"\r");
        screen += 1;
        if screen > 20 {
            break;
        }
    }

    assert!(screen > 1, "menu 7 should have at least one screen");
}

#[test]
fn vttest_golden_menu7_80x24() {
    if !vttest_available() {
        return;
    }
    run_menu7_golden(80, 24);
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
