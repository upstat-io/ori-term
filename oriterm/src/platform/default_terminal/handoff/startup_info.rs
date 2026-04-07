//! `HandoffData` payload + `TERMINAL_STARTUP_INFO` parser.
//!
//! Pure-data half of the handoff module — defined separately from the
//! COM interface vtables so the parser can be unit-tested with synthetic
//! `TERMINAL_STARTUP_INFO` structs constructed in the test body.

use std::io;

use oriterm_mux::pty::adopt::AdoptedSignal;

use super::TERMINAL_STARTUP_INFO;

/// Payload sent from the COM RPC thread to the main thread once a
/// handoff session has been received.
///
/// The COM `EstablishPtyHandoff` callback constructs one of these from
/// the parameters passed by `conhost.exe` and forwards it through an
/// `mpsc::channel` so the main thread can construct the winit event
/// loop and a [`Pane`](oriterm_mux::pane::Pane) backed by the adopted
/// handles.
#[allow(dead_code, reason = "fields read by App::run_with_handoff in Phase 4")]
pub(crate) struct HandoffData {
    /// PTY output reader (the read end of the input pipe `oriterm`
    /// created via `CreatePipe` and returned to the console host through
    /// the `[out] in` parameter — naming inverted because "in" / "out"
    /// in the IDL refer to the *console process*'s direction).
    pub reader: Box<dyn io::Read + Send>,
    /// PTY input writer (the write end of the output pipe).
    pub writer: Box<dyn io::Write + Send>,
    /// Duplicated `signal`/`reference`/`server`/`client` handles. Owned
    /// for the lifetime of the resulting [`Pane`].
    pub signal: AdoptedSignal,
    /// Client process ID reported by the console host (informational —
    /// `Pane::process_id` exposes it via `PtyLifecycle`).
    pub client_pid: Option<u32>,
    /// Title parsed from `TERMINAL_STARTUP_INFO.pszTitle`. Empty if the
    /// caller passed a null/empty `BSTR`.
    pub title: String,
    /// Icon path parsed from `TERMINAL_STARTUP_INFO.pszIconPath`. `None`
    /// for null/empty `BSTR`.
    pub icon_path: Option<String>,
    /// Initial pane row count (from `dwYCountChars`, or 24 if zero).
    pub initial_rows: u16,
    /// Initial pane column count (from `dwXCountChars`, or 80 if zero).
    pub initial_cols: u16,
}

// SAFETY: Every field of HandoffData is itself `Send`. The trait object
// `Box<dyn io::Read + Send>` and `Box<dyn io::Write + Send>` carry the
// `Send` bound explicitly. `AdoptedSignal` (oriterm_mux) implements
// `Send` via an `unsafe impl` for the same reason: the wrapped Win32
// `HANDLE`s are exclusively owned by this struct.
//
// Stating it explicitly here lets the compile-time tests assert
// `Send + Sync` without depending on auto-trait inference, which can be
// disturbed by future field additions.
unsafe impl Send for HandoffData {}
unsafe impl Sync for HandoffData {}

/// Parsed scalar fields extracted from `TERMINAL_STARTUP_INFO`.
///
/// Returned by [`from_startup_info`] so the COM callback can copy out
/// everything it needs from the caller-owned struct before the call
/// returns. Once the callback returns, the original `TERMINAL_STARTUP_INFO`
/// (and its `BSTR` fields) are freed by COM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedStartupInfo {
    /// Title decoded from `pszTitle` (empty for null/empty `BSTR`).
    pub title: String,
    /// Icon path decoded from `pszIconPath` (`None` for null/empty).
    pub icon_path: Option<String>,
    /// Row count from `dwYCountChars` (0 → fallback `DEFAULT_ROWS`).
    pub initial_rows: u16,
    /// Column count from `dwXCountChars` (0 → fallback `DEFAULT_COLS`).
    pub initial_cols: u16,
}

/// Default initial pane size when the console host doesn't specify one.
const DEFAULT_ROWS: u16 = 24;
const DEFAULT_COLS: u16 = 80;

/// Parse a caller-owned `TERMINAL_STARTUP_INFO` into owned data.
///
/// The pointer is dereferenced read-only — the function does not take
/// ownership of the `BSTR` fields, so the COM `EstablishPtyHandoff`
/// caller is still responsible for freeing them after the callback
/// returns. Strings are *copied* into owned `String`s.
///
/// # Safety
///
/// `info` must point to a valid, fully initialized `TERMINAL_STARTUP_INFO`
/// for the duration of this call. The COM `EstablishPtyHandoff`
/// signature guarantees this for `[in] const TERMINAL_STARTUP_INFO*`
/// parameters.
pub(crate) unsafe fn from_startup_info(info: *const TERMINAL_STARTUP_INFO) -> ParsedStartupInfo {
    if info.is_null() {
        return ParsedStartupInfo {
            title: String::new(),
            icon_path: None,
            initial_rows: DEFAULT_ROWS,
            initial_cols: DEFAULT_COLS,
        };
    }

    // SAFETY: caller contract — `info` is a valid pointer to a fully
    // initialized `TERMINAL_STARTUP_INFO`. We only read fields, never
    // write. The reference is short-lived and does not outlive the
    // pointer's validity (it lives only on this stack frame).
    let info = unsafe { &*info };

    let title = decode_bstr_or_empty(&info.pszTitle);
    let icon_path = decode_bstr_optional(&info.pszIconPath);
    let initial_rows = if info.dwYCountChars == 0 {
        DEFAULT_ROWS
    } else {
        u16::try_from(info.dwYCountChars).unwrap_or(u16::MAX)
    };
    let initial_cols = if info.dwXCountChars == 0 {
        DEFAULT_COLS
    } else {
        u16::try_from(info.dwXCountChars).unwrap_or(u16::MAX)
    };

    ParsedStartupInfo {
        title,
        icon_path,
        initial_rows,
        initial_cols,
    }
}

/// Decode a `BSTR` slice into an owned `String`. Returns `String::new()`
/// for empty/null inputs.
fn decode_bstr_or_empty(bstr: &windows::core::BSTR) -> String {
    let slice: &[u16] = bstr;
    if slice.is_empty() {
        return String::new();
    }
    String::from_utf16_lossy(slice)
}

/// Decode a `BSTR` slice into an owned `String`. Returns `None` for
/// empty/null inputs (used for optional fields like `pszIconPath`).
fn decode_bstr_optional(bstr: &windows::core::BSTR) -> Option<String> {
    let slice: &[u16] = bstr;
    if slice.is_empty() {
        None
    } else {
        Some(String::from_utf16_lossy(slice))
    }
}
