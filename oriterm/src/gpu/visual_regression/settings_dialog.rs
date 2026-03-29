//! Settings-dialog golden tests.
//!
//! Renders the settings dialog through the real UI-only pipeline and
//! compares against committed reference PNGs. Covers sidebar, content
//! typography, footer, dirty state, and the expanded widget controls.

use super::compare_with_reference;
use super::dialog_helpers::{
    headless_dialog_env, headless_dialog_env_with_dpi, render_dialog_to_pixels,
};
use crate::app::test_support::build_dialog_scene;

/// Appearance page at 96 DPI — sidebar, sliders, toggles, dropdowns, clean footer.
#[test]
fn settings_appearance_clean_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_dialog_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let scene = build_dialog_scene(&renderer, 0, false, 800.0, 600.0, 1.0);
    let pixels = render_dialog_to_pixels(&gpu, &pipelines, &mut renderer, &scene, 800, 600, 1.0);

    if let Err(msg) = compare_with_reference("settings_appearance_clean_96dpi", &pixels, 800, 600) {
        panic!("visual regression: {msg}");
    }
}

/// Colors page at 96 DPI — scheme cards grid, badge chip, swatches.
#[test]
fn settings_colors_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_dialog_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let scene = build_dialog_scene(&renderer, 1, false, 900.0, 600.0, 1.0);
    let pixels = render_dialog_to_pixels(&gpu, &pipelines, &mut renderer, &scene, 900, 600, 1.0);

    if let Err(msg) = compare_with_reference("settings_colors_96dpi", &pixels, 900, 600) {
        panic!("visual regression: {msg}");
    }
}

/// Terminal page at 96 DPI — cursor picker, number inputs, text inputs.
#[test]
fn settings_terminal_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_dialog_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let scene = build_dialog_scene(&renderer, 3, false, 800.0, 600.0, 1.0);
    let pixels = render_dialog_to_pixels(&gpu, &pipelines, &mut renderer, &scene, 800, 600, 1.0);

    if let Err(msg) = compare_with_reference("settings_terminal_96dpi", &pixels, 800, 600) {
        panic!("visual regression: {msg}");
    }
}

/// Window page with dirty footer at 96 DPI — number/text inputs, dirty state.
#[test]
fn settings_window_dirty_96dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_dialog_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let scene = build_dialog_scene(&renderer, 5, true, 800.0, 600.0, 1.0);
    let pixels = render_dialog_to_pixels(&gpu, &pipelines, &mut renderer, &scene, 800, 600, 1.0);

    if let Err(msg) = compare_with_reference("settings_window_dirty_96dpi", &pixels, 800, 600) {
        panic!("visual regression: {msg}");
    }
}

/// Appearance page at 192 DPI — catches rounding and scaling regressions.
#[test]
fn settings_appearance_clean_192dpi() {
    let Some((gpu, pipelines, mut renderer)) = headless_dialog_env_with_dpi(192.0) else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let scene = build_dialog_scene(&renderer, 0, false, 800.0, 600.0, 2.0);
    let pixels = render_dialog_to_pixels(&gpu, &pipelines, &mut renderer, &scene, 1600, 1200, 2.0);

    if let Err(msg) =
        compare_with_reference("settings_appearance_clean_192dpi", &pixels, 1600, 1200)
    {
        panic!("visual regression: {msg}");
    }
}
