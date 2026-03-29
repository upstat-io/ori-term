//! Tests for window decoration mode.

use super::DecorationMode;

#[test]
fn native_to_frameless_requires_restart() {
    assert!(DecorationMode::Native.macos_requires_restart(DecorationMode::Frameless));
}

#[test]
fn frameless_to_native_requires_restart() {
    assert!(DecorationMode::Frameless.macos_requires_restart(DecorationMode::Native));
}

#[test]
fn native_to_buttonless_requires_restart() {
    assert!(DecorationMode::Native.macos_requires_restart(DecorationMode::Buttonless));
}

#[test]
fn frameless_to_buttonless_requires_restart() {
    assert!(DecorationMode::Frameless.macos_requires_restart(DecorationMode::Buttonless));
}

#[test]
fn frameless_to_transparent_no_restart() {
    assert!(!DecorationMode::Frameless.macos_requires_restart(DecorationMode::TransparentTitlebar));
}

#[test]
fn transparent_to_frameless_no_restart() {
    assert!(!DecorationMode::TransparentTitlebar.macos_requires_restart(DecorationMode::Frameless));
}

#[test]
fn same_mode_no_restart() {
    for mode in [
        DecorationMode::Native,
        DecorationMode::Frameless,
        DecorationMode::TransparentTitlebar,
        DecorationMode::Buttonless,
    ] {
        assert!(!mode.macos_requires_restart(mode), "{mode:?} → {mode:?}");
    }
}
