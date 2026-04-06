//! COM interface and struct definitions for the Windows Default
//! Terminal handoff (Section 03.9 Phase 3).
//!
//! These types are NOT shipped in the standard Windows SDK headers —
//! they live in the Windows Terminal source under
//! `terminal/src/host/proxy/ITerminalHandoff.idl`. We mirror that IDL
//! here using the [`windows_core::interface`] proc macro so the
//! [`HandoffServer`](super::HandoffServer) can implement them via
//! `windows_core::implement`.
//!
//! Names preserve the original Win32 / IDL conventions (`PascalCase`
//! methods, `pszTitle`, `dwXCountChars`, etc.) so the mapping back to
//! the spec is unambiguous. Suppressed at module level rather than per
//! item because the `#[interface]` proc macro rejects sibling
//! attributes on the trait declaration.

#![allow(
    non_snake_case,
    reason = "preserves IDL field/method names from terminal/src/host/proxy/ITerminalHandoff.idl"
)]
#![allow(
    clippy::inline_always,
    clippy::ref_as_ptr,
    clippy::transmute_ptr_to_ptr,
    clippy::transmute_undefined_repr,
    clippy::borrow_as_ptr,
    clippy::too_many_arguments,
    reason = "lints fire inside #[interface] proc-macro expansion that we cannot edit"
)]

use windows::Win32::Foundation::HANDLE;
use windows_core::{BSTR, HRESULT, IUnknown, IUnknown_Vtbl, interface};

/// Mirror of the `_TERMINAL_STARTUP_INFO` struct from the Windows
/// Terminal IDL.
///
/// Field order and types must match `terminal/src/host/proxy/ITerminalHandoff.idl`
/// byte-for-byte — COM marshals this struct by C ABI layout. The
/// `BSTR` fields are caller-owned: they remain valid only for the
/// duration of the COM callback that received the pointer, after which
/// the caller frees them.
///
/// Field naming preserves the original Win32 / IDL conventions
/// (`pszTitle`, `dwXCountChars`, etc.) so the mapping back to the spec
/// is unambiguous. Lint suppression lives at the module level.
#[repr(C)]
pub(crate) struct TERMINAL_STARTUP_INFO {
    /// Window title BSTR (from `STARTUPINFO.lpTitle` or `.lnk` metadata).
    pub pszTitle: BSTR,
    /// Icon path BSTR (from `STARTUPINFO.lpReserved` icon metadata).
    pub pszIconPath: BSTR,
    /// Index into the icon resource file at `pszIconPath`.
    pub iconIndex: i32,
    /// `STARTUPINFO.dwX`.
    pub dwX: u32,
    /// `STARTUPINFO.dwY`.
    pub dwY: u32,
    /// `STARTUPINFO.dwXSize`.
    pub dwXSize: u32,
    /// `STARTUPINFO.dwYSize`.
    pub dwYSize: u32,
    /// `STARTUPINFO.dwXCountChars` (initial pane column count).
    pub dwXCountChars: u32,
    /// `STARTUPINFO.dwYCountChars` (initial pane row count).
    pub dwYCountChars: u32,
    /// `STARTUPINFO.dwFillAttribute`.
    pub dwFillAttribute: u32,
    /// `STARTUPINFO.dwFlags` bitmask.
    pub dwFlags: u32,
    /// `STARTUPINFO.wShowWindow`.
    pub wShowWindow: u16,
}

/// `ITerminalHandoff3` — current (v3) interface used by `conhost.exe` to
/// hand a console session off to a registered terminal application.
///
/// IID `{6F23DA90-15C5-4203-9DB0-64E73F1B1B00}` matches the canonical
/// definition in `ITerminalHandoff.idl`. The interface differs from v1/v2
/// in two ways:
///
/// 1. The `in`/`out` PTY pipe handles are `[out]` (terminal-created) so
///    we control buffer size and overlapped I/O mode.
/// 2. `TERMINAL_STARTUP_INFO` is passed by-pointer instead of by-value
///    (consistent with COM convention for non-trivial structs).
///
/// `EstablishPtyHandoff` parameter direction:
/// - `[out] in`/`[out] out`: terminal-created pipe ends. The terminal
///   keeps one end and returns the other to the console host via these
///   out-pointers.
/// - `[in] signal`/`reference`/`server`/`client`: caller-owned handles
///   that must be `DuplicateHandle`d before the call returns.
/// - `[in] startup_info`: caller-owned struct; copy out the fields you
///   need before returning.
#[interface("6F23DA90-15C5-4203-9DB0-64E73F1B1B00")]
pub unsafe trait ITerminalHandoff3: IUnknown {
    unsafe fn EstablishPtyHandoff(
        &self,
        in_handle: *mut HANDLE,
        out_handle: *mut HANDLE,
        signal: HANDLE,
        reference: HANDLE,
        server: HANDLE,
        client: HANDLE,
        startup_info: *const TERMINAL_STARTUP_INFO,
    ) -> HRESULT;
}

/// `IDefaultTerminalMarker` — empty marker interface that signals to
/// `conhost.exe` that this COM server supports the default terminal
/// handoff feature.
///
/// IID `{746E6BC0-AB05-4E38-AB14-71E86763141F}`. Conhost queries this
/// interface during startup; objects that implement it are considered
/// valid default-terminal targets. The interface has no methods of its
/// own — its presence on a CLSID is the entire signal.
#[interface("746E6BC0-AB05-4E38-AB14-71E86763141F")]
pub unsafe trait IDefaultTerminalMarker: IUnknown {}
