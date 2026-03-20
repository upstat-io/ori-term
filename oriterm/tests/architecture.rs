//! Architectural boundary tests.
//!
//! These tests verify that the crate responsibility boundaries are maintained.
//! If a test fails, it means code has drifted into the wrong crate.
//!
//! See `.claude/rules/crate-boundaries.md` for the full ownership rules.

use oriterm_ui::action::WidgetAction;
use oriterm_ui::geometry::Point;
use oriterm_ui::input::MouseButton;
use oriterm_ui::layout::Direction;
use oriterm_ui::testing::WidgetTestHarness;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::button::ButtonWidget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::window_root::WindowRoot;

// ---------------------------------------------------------------------------
// WindowRoot headless construction
// ---------------------------------------------------------------------------

/// `WindowRoot` must be constructable without GPU or platform dependencies.
#[test]
fn window_root_is_headless() {
    let _root = WindowRoot::new(ButtonWidget::new("test"));
}

/// `WidgetTestHarness` wraps `WindowRoot` and exposes it.
#[test]
fn harness_wraps_window_root() {
    let harness = WidgetTestHarness::new(ButtonWidget::new("test"));
    let root = harness.root();
    assert!(
        root.viewport().width() > 0.0,
        "WindowRoot must have a valid viewport"
    );
}

// ---------------------------------------------------------------------------
// Event propagation through WindowRoot
// ---------------------------------------------------------------------------

/// Events propagate through WindowRoot -> container -> button.
#[test]
fn event_propagation_through_window_root() {
    let button = ButtonWidget::new("nested");
    let button_id = button.id();
    let container = ContainerWidget::new(Direction::Column).with_child(Box::new(button));

    let mut harness = WidgetTestHarness::new(container);
    let actions = harness.click(button_id);

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, WidgetAction::Clicked(id) if *id == button_id)),
        "Click on nested button must produce Clicked action"
    );
}

/// Overlay events take priority over widget tree events.
#[test]
fn overlay_event_priority_through_window_root() {
    let button = ButtonWidget::new("under");
    let button_id = button.id();
    let mut harness = WidgetTestHarness::new(button);

    // Push an overlay that covers the entire viewport.
    let overlay_button = ButtonWidget::new("overlay");
    let viewport = harness.viewport();
    harness.push_popup(overlay_button, viewport);

    // Click at the center of the button — overlay should intercept.
    let bounds = harness.widget_bounds(button_id);
    let center = Point::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    );
    harness.mouse_move(center);
    harness.mouse_down(MouseButton::Left);
    harness.mouse_up(MouseButton::Left);
    let actions = harness.take_actions();

    // The underlying button must NOT receive Clicked — the overlay consumed it.
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, WidgetAction::Clicked(id) if *id == button_id)),
        "Overlay must intercept clicks, button should not receive Clicked"
    );
}

/// `InteractionManager` state updates through `WindowRoot`.
#[test]
fn interaction_state_through_window_root() {
    let button = ButtonWidget::new("focusable");
    let button_id = button.id();
    let mut harness = WidgetTestHarness::new(button);

    // Move mouse over button center.
    let bounds = harness.widget_bounds(button_id);
    let center = Point::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    );
    harness.mouse_move(center);
    assert!(harness.is_hot(button_id), "Button must be hot after hover");

    // Press mouse — button becomes active.
    harness.mouse_down(MouseButton::Left);
    assert!(
        harness.is_active(button_id),
        "Button must be active after mouse down"
    );
}

// ---------------------------------------------------------------------------
// Crate dependency direction validation
// ---------------------------------------------------------------------------

/// Extracts dependency crate names from a `Cargo.toml` string.
///
/// Only scans lines inside `[dependencies]`, `[dev-dependencies]`,
/// `[build-dependencies]`, and their `[target.*.dependencies]` variants.
/// Ignores `[package]`, `[features]`, `[lints]`, comments, etc.
fn dep_names(cargo_toml: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_deps = false;
    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed.contains("dependencies");
            continue;
        }
        if in_deps && !trimmed.is_empty() && !trimmed.starts_with('#') {
            if let Some(name) = trimmed.split(&['=', '.'][..]).next() {
                let name = name.trim();
                if !name.is_empty() {
                    names.push(name.to_string());
                }
            }
        }
    }
    names
}

/// `oriterm_ui` must NOT depend on GPU or font rasterization crates.
#[test]
fn oriterm_ui_has_no_gpu_or_font_deps() {
    let cargo_toml = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../oriterm_ui/Cargo.toml"
    ))
    .unwrap();
    let deps = dep_names(&cargo_toml);
    for forbidden in &["wgpu", "tiny-skia", "swash", "skrifa", "rustybuzz"] {
        assert!(
            !deps.iter().any(|d| d == *forbidden),
            "oriterm_ui must not depend on {forbidden} (GPU/font pipeline belongs in oriterm)"
        );
    }
}

/// `oriterm_ui` must NOT depend on `oriterm`, `oriterm_mux`, or `oriterm_ipc`.
#[test]
fn oriterm_ui_has_no_upstream_deps() {
    let cargo_toml = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../oriterm_ui/Cargo.toml"
    ))
    .unwrap();
    let deps = dep_names(&cargo_toml);
    for forbidden in &["oriterm", "oriterm_mux", "oriterm_ipc"] {
        assert!(
            !deps.iter().any(|d| d == *forbidden),
            "oriterm_ui must not depend on {forbidden}"
        );
    }
}

/// `oriterm_core` must NOT depend on any other workspace crate.
#[test]
fn oriterm_core_has_no_upstream_deps() {
    let cargo_toml = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../oriterm_core/Cargo.toml"
    ))
    .unwrap();
    let deps = dep_names(&cargo_toml);
    for forbidden in &["oriterm", "oriterm_ui", "oriterm_mux", "oriterm_ipc"] {
        assert!(
            !deps.iter().any(|d| d == *forbidden),
            "oriterm_core must not depend on {forbidden}"
        );
    }
}

/// `oriterm_mux` must NOT depend on `oriterm_ui` or `oriterm`.
#[test]
fn oriterm_mux_has_no_ui_or_app_deps() {
    let cargo_toml = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../oriterm_mux/Cargo.toml"
    ))
    .unwrap();
    let deps = dep_names(&cargo_toml);
    for forbidden in &["oriterm", "oriterm_ui"] {
        assert!(
            !deps.iter().any(|d| d == *forbidden),
            "oriterm_mux must not depend on {forbidden}"
        );
    }
}

/// `oriterm_ipc` must NOT depend on any other `oriterm_*` crate.
#[test]
fn oriterm_ipc_is_standalone() {
    let cargo_toml = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../oriterm_ipc/Cargo.toml"
    ))
    .unwrap();
    let deps = dep_names(&cargo_toml);
    for dep in &deps {
        assert!(
            !dep.starts_with("oriterm"),
            "oriterm_ipc must not depend on any oriterm crate (found: {dep})"
        );
    }
}
