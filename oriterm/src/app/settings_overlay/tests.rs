//! Tests for settings overlay utilities.

use crate::config::Config;

use super::per_page_dirty;

#[test]
fn per_page_dirty_all_clean_when_identical() {
    let a = Config::default();
    let b = Config::default();
    let dirty = per_page_dirty(&a, &b);
    assert!(
        dirty.iter().all(|&d| !d),
        "identical configs should have no dirty pages"
    );
}

#[test]
fn per_page_dirty_appearance_detects_opacity() {
    let original = Config::default();
    let mut pending = original.clone();
    pending.window.opacity = 0.5;
    let dirty = per_page_dirty(&pending, &original);
    assert!(
        dirty[0],
        "appearance page should be dirty when opacity changes"
    );
    assert!(
        !dirty[5],
        "window page should NOT be dirty for opacity change"
    );
}

#[test]
fn per_page_dirty_font_page() {
    let original = Config::default();
    let mut pending = original.clone();
    pending.font.size = 18.0;
    let dirty = per_page_dirty(&pending, &original);
    assert!(dirty[2], "font page should be dirty");
    assert!(!dirty[0], "appearance page should be clean");
}

#[test]
fn per_page_dirty_terminal_page() {
    let original = Config::default();
    let mut pending = original.clone();
    pending.terminal.scrollback = 5000;
    let dirty = per_page_dirty(&pending, &original);
    assert!(dirty[3], "terminal page should be dirty");
}

#[test]
fn per_page_dirty_keybindings_always_clean() {
    let original = Config::default();
    let mut pending = original.clone();
    // Change everything else.
    pending.window.opacity = 0.5;
    pending.font.size = 18.0;
    let dirty = per_page_dirty(&pending, &original);
    assert!(!dirty[4], "keybindings page should always be clean");
}

#[test]
fn per_page_dirty_window_page_detects_columns() {
    let original = Config::default();
    let mut pending = original.clone();
    pending.window.columns = 80;
    let dirty = per_page_dirty(&pending, &original);
    assert!(dirty[5], "window page should be dirty when columns changes");
    assert!(!dirty[0], "appearance page should be clean");
}

#[test]
fn per_page_dirty_bell_page() {
    let original = Config::default();
    let mut pending = original.clone();
    pending.bell.duration_ms = 100;
    let dirty = per_page_dirty(&pending, &original);
    assert!(dirty[6], "bell page should be dirty");
}

#[test]
fn per_page_dirty_tab_bar_position_hidden_dirties_both_appearance_and_window() {
    let original = Config::default();
    let mut pending = original.clone();
    pending.window.tab_bar_position = crate::config::TabBarPosition::Hidden;
    let dirty = per_page_dirty(&pending, &original);
    assert!(
        dirty[0],
        "appearance page should be dirty when tab_bar_position changes to Hidden"
    );
    assert!(
        dirty[5],
        "window page should be dirty when tab_bar_position changes"
    );
}

#[test]
fn per_page_dirty_scheme_changes_two_pages() {
    let original = Config::default();
    let mut pending = original.clone();
    pending.colors.scheme = "solarized".to_owned();
    let dirty = per_page_dirty(&pending, &original);
    // Scheme affects both Appearance (0) and Colors (1).
    assert!(
        dirty[0],
        "appearance page should be dirty for scheme change"
    );
    assert!(dirty[1], "colors page should be dirty for scheme change");
}
