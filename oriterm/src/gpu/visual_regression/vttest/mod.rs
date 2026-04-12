//! vttest golden image tests.
//!
//! Spawns `vttest` in a real PTY (via [`oriterm_test_support::PtySession`]),
//! feeds output through `Term`'s VTE parser, navigates menus with scripted
//! keystrokes, and renders each test screen through the full GPU pipeline.
//! The resulting framebuffer is compared against golden reference PNGs —
//! capturing colors, attributes, borders, and character rendering.
//!
//! Pre-deduplication this file owned its own copy of `PtyResponder` /
//! `VtTestSession`. The shared session lives in `crates/oriterm_test_support`
//! now.

mod menus_3_8;
mod render;

use oriterm_core::CellFlags;
use oriterm_test_support::{PtySession, vttest_available};

use self::render::{assert_golden, cell_brightness, frame_input_with_blink};
use super::{headless_env, render_to_pixels};

/// Run vttest menu 1 (cursor movement) and capture golden images.
fn run_menu1_golden(cols: u16, rows: u16) {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let mut s = PtySession::spawn_vttest(cols, rows);
    let label = format!("{cols}x{rows}");

    // Wait for main menu to fully render.
    s.wait_for("Enter choice number", 5000);
    assert_golden(
        &s,
        &format!("vttest_{label}_menu"),
        &gpu,
        &pipelines,
        &mut renderer,
    );

    // Select menu item 1.
    s.send(b"1\r");

    let mut screen = 1;
    loop {
        let text = s.grid_text();

        if text.contains("Enter choice number") {
            break;
        }

        assert_golden(
            &s,
            &format!("vttest_{label}_01_{screen:02}"),
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

/// Run vttest menu 2 (screen features) and capture golden images.
fn run_menu2_golden(cols: u16, rows: u16) {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let mut s = PtySession::spawn_vttest(cols, rows);
    let label = format!("{cols}x{rows}");

    s.wait_for("Enter choice number", 5000);
    s.send(b"2\r");

    let mut screen = 1;
    loop {
        let text = s.grid_text();

        if text.contains("Enter choice number") {
            break;
        }

        assert_golden(
            &s,
            &format!("vttest_{label}_02_{screen:02}"),
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

// -- Menu 1: Cursor movements --

#[test]
fn vttest_golden_menu1_80x24() {
    if !vttest_available() {
        return;
    }
    run_menu1_golden(80, 24);
}

#[test]
fn vttest_golden_menu1_97x33() {
    if !vttest_available() {
        return;
    }
    run_menu1_golden(97, 33);
}

#[test]
fn vttest_golden_menu1_120x40() {
    if !vttest_available() {
        return;
    }
    run_menu1_golden(120, 40);
}

// -- Menu 2: Screen features --

#[test]
fn vttest_golden_menu2_80x24() {
    if !vttest_available() {
        return;
    }
    run_menu2_golden(80, 24);
}

#[test]
fn vttest_golden_menu2_97x33() {
    if !vttest_available() {
        return;
    }
    run_menu2_golden(97, 33);
}

#[test]
fn vttest_golden_menu2_120x40() {
    if !vttest_available() {
        return;
    }
    run_menu2_golden(120, 40);
}

/// Navigate vttest to the SGR blink screen (menu 2 screen 13) and verify
/// that rendering at different `text_blink_opacity` values produces
/// visibly different output for BLINK cells while non-BLINK cells stay
/// constant.
#[test]
fn vttest_blink_multi_frame() {
    if !vttest_available() {
        return;
    }

    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let mut s = PtySession::spawn_vttest(80, 24);
    s.wait_for("Enter choice number", 5000);
    s.send(b"2\r");

    // Advance to screen 13 (SGR test — dark background with blink text).
    for _ in 1..13 {
        s.send(b"\r");
    }

    // Verify we actually have BLINK cells in this screen.
    let content = s.term().renderable_content();
    let blink_count = content
        .cells
        .iter()
        .filter(|c| c.flags.contains(CellFlags::BLINK) && c.ch != ' ')
        .count();
    assert!(
        blink_count > 0,
        "screen 13 should contain cells with CellFlags::BLINK, found 0"
    );

    // Find a BLINK cell and a non-BLINK non-space cell for comparison.
    let blink_idx = content
        .cells
        .iter()
        .position(|c| c.flags.contains(CellFlags::BLINK) && c.ch != ' ')
        .expect("should have a BLINK cell");
    let normal_idx = content
        .cells
        .iter()
        .position(|c| !c.flags.contains(CellFlags::BLINK) && c.ch != ' ')
        .expect("should have a non-BLINK cell");

    let cell = renderer.cell_metrics();
    let cols = 80_usize;

    let blink_col = blink_idx % cols;
    let blink_row = blink_idx / cols;
    let normal_col = normal_idx % cols;
    let normal_row = normal_idx / cols;

    // Render 3 frames at opacity 1.0, 0.5, and 0.0.
    let opacities = [1.0_f32, 0.5, 0.0];
    let mut blink_brightness = Vec::new();
    let mut normal_brightness = Vec::new();

    for &opacity in &opacities {
        let input = frame_input_with_blink(&s, cell, opacity);
        let w = input.viewport.width;
        let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);

        let b_br = cell_brightness(&pixels, w, blink_col, blink_row, cell.width, cell.height);
        let n_br = cell_brightness(&pixels, w, normal_col, normal_row, cell.width, cell.height);

        blink_brightness.push(b_br);
        normal_brightness.push(n_br);
    }

    // BLINK cell brightness must decrease: 1.0 > 0.5 > 0.0.
    assert!(
        blink_brightness[0] > blink_brightness[1],
        "BLINK cell should dim at 0.5: full={} half={}",
        blink_brightness[0],
        blink_brightness[1],
    );
    assert!(
        blink_brightness[1] > blink_brightness[2],
        "BLINK cell should dim at 0.0: half={} hidden={}",
        blink_brightness[1],
        blink_brightness[2],
    );

    // Non-BLINK cell brightness must stay constant (within tolerance).
    for i in 0..2 {
        let diff = (normal_brightness[i] as i32 - normal_brightness[i + 1] as i32).abs();
        assert!(
            diff < 5,
            "non-BLINK cell should be constant: frame{}={} frame{}={} diff={}",
            i,
            normal_brightness[i],
            i + 1,
            normal_brightness[i + 1],
            diff,
        );
    }
}
