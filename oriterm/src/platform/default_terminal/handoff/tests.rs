//! Tests for `HandoffData` Send/Sync invariants and the
//! `from_startup_info` parser.
//!
//! These tests run on Windows targets only because the parent module
//! itself is gated `#[cfg(target_os = "windows")]`. Cross-compiling to
//! `x86_64-pc-windows-gnu` from Linux still type-checks them, so the
//! Send/Sync `assert_*` calls catch regressions during normal CI.

use windows::core::BSTR;

use super::startup_info::{ParsedStartupInfo, from_startup_info};
use super::{HandoffData, TERMINAL_STARTUP_INFO};

// Compile-time assertions: HandoffData must cross thread boundaries.

const _: fn() = || {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<HandoffData>();
    assert_sync::<HandoffData>();
};

/// Build a synthetic `TERMINAL_STARTUP_INFO` for parser tests.
///
/// `BSTR` allocates via `SysAllocStringLen` so the pointer behaves
/// exactly like one constructed by `conhost.exe`. The struct lives on
/// the test stack and is dropped after the parser returns — Drop on
/// `BSTR` calls `SysFreeString`, mirroring the COM caller's cleanup
/// after `EstablishPtyHandoff` returns.
fn make_startup_info(
    title: &str,
    icon_path: Option<&str>,
    rows: u32,
    cols: u32,
) -> TERMINAL_STARTUP_INFO {
    TERMINAL_STARTUP_INFO {
        pszTitle: BSTR::from(title),
        pszIconPath: icon_path.map(BSTR::from).unwrap_or_default(),
        iconIndex: 0,
        dwX: 0,
        dwY: 0,
        dwXSize: 0,
        dwYSize: 0,
        dwXCountChars: cols,
        dwYCountChars: rows,
        dwFillAttribute: 0,
        dwFlags: 0,
        wShowWindow: 0,
    }
}

#[test]
fn from_startup_info_full_payload() {
    let info = make_startup_info("My Console", Some(r"C:\icons\app.ico"), 30, 100);
    // SAFETY: `info` is a stack-allocated, fully initialized struct.
    let parsed = unsafe { from_startup_info(&raw const info) };
    assert_eq!(
        parsed,
        ParsedStartupInfo {
            title: "My Console".to_string(),
            icon_path: Some(r"C:\icons\app.ico".to_string()),
            initial_rows: 30,
            initial_cols: 100,
        }
    );
}

#[test]
fn from_startup_info_empty_title_yields_empty_string() {
    let info = make_startup_info("", None, 24, 80);
    // SAFETY: see above.
    let parsed = unsafe { from_startup_info(&raw const info) };
    assert!(parsed.title.is_empty());
    assert!(parsed.icon_path.is_none());
}

#[test]
fn from_startup_info_null_icon_returns_none() {
    let info = make_startup_info("Title", None, 24, 80);
    // SAFETY: see above.
    let parsed = unsafe { from_startup_info(&raw const info) };
    assert_eq!(parsed.title, "Title");
    assert_eq!(parsed.icon_path, None);
}

#[test]
fn from_startup_info_zero_dimensions_use_defaults() {
    let info = make_startup_info("Title", None, 0, 0);
    // SAFETY: see above.
    let parsed = unsafe { from_startup_info(&raw const info) };
    assert_eq!(
        parsed.initial_rows, 24,
        "0 rows should fall back to DEFAULT_ROWS",
    );
    assert_eq!(
        parsed.initial_cols, 80,
        "0 cols should fall back to DEFAULT_COLS",
    );
}

#[test]
fn from_startup_info_oversized_dimensions_clamp_to_u16_max() {
    let info = make_startup_info("Title", None, u32::MAX, u32::MAX);
    // SAFETY: see above.
    let parsed = unsafe { from_startup_info(&raw const info) };
    assert_eq!(parsed.initial_rows, u16::MAX);
    assert_eq!(parsed.initial_cols, u16::MAX);
}

#[test]
fn from_startup_info_null_pointer_returns_defaults() {
    // SAFETY: from_startup_info explicitly handles a null pointer.
    let parsed = unsafe { from_startup_info(std::ptr::null()) };
    assert!(parsed.title.is_empty());
    assert!(parsed.icon_path.is_none());
    assert_eq!(parsed.initial_rows, 24);
    assert_eq!(parsed.initial_cols, 80);
}
