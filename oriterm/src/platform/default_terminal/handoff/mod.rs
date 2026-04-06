//! `ITerminalHandoff3` COM server for Windows Default Terminal handoff
//! (Section 03.9 Phase 3).
//!
//! When a console-launching program runs while `ori_term` is registered
//! as the default terminal, `conhost.exe` activates this COM server via
//! `CoCreateInstance({ORITERM_TERMINAL_CLSID}, CLSCTX_LOCAL_SERVER)` and
//! calls [`HandoffServer::EstablishPtyHandoff`]. The implementation:
//!
//! 1. Creates the input/output pipe pair (the `[out]` parameters in
//!    `ITerminalHandoff3` give the terminal control over buffer size and
//!    overlapped I/O — that's the whole point of v3).
//! 2. Duplicates the caller-owned signal/reference/server/client handles
//!    via `DuplicateHandle` so they outlive the COM call.
//! 3. Copies the `TERMINAL_STARTUP_INFO` `BSTR` fields into owned
//!    `String`s before returning (the caller frees the original BSTRs).
//! 4. Builds a [`HandoffData`] payload and sends it to the main thread
//!    via the `mpsc::Sender` provided by [`run_com_server`](super::run_com_server).
//!
//! The actual conhost-side handoff and `OpenConsole.exe` proxying live in
//! Windows Terminal source — see `terminal/src/host/proxy/`.

#![allow(
    unsafe_code,
    reason = "COM interface FFI: HANDLE duplication, BSTR decoding, IClassFactory vtable"
)]
#![allow(
    clippy::inline_always,
    clippy::ref_as_ptr,
    clippy::transmute_ptr_to_ptr,
    clippy::transmute_undefined_repr,
    clippy::borrow_as_ptr,
    clippy::too_many_arguments,
    clippy::missing_safety_doc,
    reason = "lints fire inside #[interface] / #[implement] proc-macro expansion that we cannot edit"
)]

mod com_interfaces;
mod server;
mod startup_info;

pub(crate) use com_interfaces::{
    IDefaultTerminalMarker, IDefaultTerminalMarker_Impl, ITerminalHandoff3, ITerminalHandoff3_Impl,
    TERMINAL_STARTUP_INFO,
};
pub(crate) use server::HandoffServer;
pub(crate) use startup_info::{HandoffData, from_startup_info};

#[cfg(test)]
mod tests;
