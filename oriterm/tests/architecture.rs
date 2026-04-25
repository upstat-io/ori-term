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

// BUG-09-1: move_tab_to_new_window_embedded must mirror tear_off_tab's
// working sequence (create_window_bare → release width lock → insert →
// pump events → seed → sync → refresh → pre-render new (focused-id swap)
// → pre-render source → set_visible). Without all of these, the new
// window appears blank or with stale source-window dimensions. These
// grep-based pins catch accidental removal.
// See plans/bug-tracker/fix-BUG-09-001.md.

/// `move_tab_to_new_window_embedded` must include the canonical call
/// sequence from `tear_off_tab` (no visible-then-render flash). Bounded
/// to the target function body and ordered so reordering or relocating
/// the canonical steps fails the test.
#[test]
fn move_to_new_window_embedded_mirrors_tear_off_sequence() {
    let body = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/tab_management/move_ops.rs"
    ))
    .unwrap();
    let fn_body = extract_fn_body(&body, "fn move_tab_to_new_window_embedded(");

    // Each entry: a required step, asserted in order. The sequence below
    // matches `tear_off_tab` (sans the OS-drag start at the end of
    // tear-off) — release the source width lock, create bare hidden
    // window, insert the moved tab directly, pump events from the move,
    // seed pane cell metrics for the new window, sync + refresh both
    // windows, then the critical pre-render-before-show ordering:
    // focused-id swap → new-window redraw → restore focused → source
    // redraw → set_visible(true).
    let ordered: &[&str] = &[
        "release_tab_width_lock",
        "create_window_bare",
        "insert_tab_at",
        "pump_mux_events",
        "seed_pane_with_window_cell_metrics",
        "sync_tab_bar_for_window",
        "refresh_platform_rects",
        // The focused-id + active-window swap that forces handle_redraw
        // to paint the new window. handle_redraw resolves the pane
        // through `active_window`, so BOTH must be swapped — `focused_id`
        // alone leaves the redraw painting the source's pane on the new
        // window's surface.
        "self.focused_window_id = Some(new_winit_id);",
        "self.active_window = Some(new_session_wid);",
        // The first handle_redraw — paints the new window with content.
        "self.handle_redraw();",
        // Restore both halves of the swap.
        "self.focused_window_id = saved_focused;",
        "self.active_window = saved_active;",
        // Second handle_redraw — paints the source so its tab bar updates.
        "self.handle_redraw();",
        // Show the new window AFTER it has been pre-rendered.
        "set_visible(true)",
        // Close the source if it's now empty.
        "remove_empty_window",
    ];

    let mut cursor = 0usize;
    for required in ordered {
        match fn_body[cursor..].find(required) {
            Some(rel) => cursor += rel + required.len(),
            None => panic!(
                "move_tab_to_new_window_embedded must call `{required}` AFTER the prior step \
                 (BUG-09-1 mirror invariant). Either the call is missing or the canonical \
                 sequence has been reordered.",
            ),
        }
    }
}

/// Slice the body of a Rust function, bounded by its opening `{` and the
/// matching `}` at the same brace-depth. Test-only helper.
fn extract_fn_body<'a>(file_body: &'a str, fn_signature_prefix: &str) -> &'a str {
    let fn_start = file_body
        .find(fn_signature_prefix)
        .unwrap_or_else(|| panic!("could not find `{fn_signature_prefix}` in file"));
    let open_brace_rel = file_body[fn_start..]
        .find('{')
        .expect("function signature must be followed by an opening brace");
    let body_start = fn_start + open_brace_rel;
    let bytes = file_body.as_bytes();
    let mut depth = 0i32;
    let mut idx = body_start;
    while idx < bytes.len() {
        match bytes[idx] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &file_body[body_start..=idx];
                }
            }
            _ => {}
        }
        idx += 1;
    }
    panic!("unbalanced braces while bounding `{fn_signature_prefix}`");
}

/// `move_tab_to_window` must NOT come back — it had a known correctness
/// bug (BUG-09-2: used `resize_all_panes()` against the focused window
/// instead of the destination) and was removed during the BUG-09-1 fix.
/// Resurrecting it without first fixing `resize_all_panes` would
/// re-introduce the bug.
#[test]
fn move_tab_to_window_helper_remains_removed() {
    let body = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/tab_management/move_ops.rs"
    ))
    .unwrap();
    assert!(
        !body.contains("fn move_tab_to_window("),
        "fn move_tab_to_window must remain removed; resurrecting it would re-introduce BUG-09-2 \
         (resize_all_panes targets focused window, not destination). If a cross-window move \
         helper is needed, design it with destination-targeted layout from day 1.",
    );
}
