//! Windows Default Terminal Registration (Section 03.9).
//!
//! Implements the COM server side of the Windows 11+ default-terminal
//! mechanism, where third-party terminal applications register as the
//! default handler for new console sessions launched by other programs.
//!
//! ## Module layout
//!
//! - [`registry`] (Phase 2): Helpers that read and write the registry
//!   keys controlling default-terminal selection
//!   (`HKCU\Console\%%Startup` selectors plus
//!   `HKCU\Software\Classes\CLSID\{ORITERM_TERMINAL_CLSID}\LocalServer32`
//!   for the COM server registration).
//! - [`handoff`] (Phase 3): The `ITerminalHandoff3` COM server that
//!   `conhost.exe` activates when a console-launching program runs.
//!   Defines `HandoffServer`, `HandoffData`, and the `TERMINAL_STARTUP_INFO`
//!   parser.
//!
//! - [`run_com_server`] (Phase 3): The 9-step COM lifecycle —
//!   `CoInitializeEx`(MTA) → `IClassFactory` registration →
//!   `CoRegisterClassObject` → block on the handoff channel → return
//!   `HandoffData` to the main thread.
//!
//! ## Future phases (planned by Section 03.9 in `plans/roadmap`)
//!
//! - **Phase 4**: CLI subcommands (`--register-default` /
//!   `--unregister-default`) and Settings UI toggle.
//!
//! The entire module is gated `#[cfg(target_os = "windows")]` because the
//! feature has no analogue on Linux or macOS — the Windows console
//! delegation registry has no cross-platform counterpart. Non-Windows
//! callers receive "not supported" errors at the CLI/UI boundary rather
//! than from this module.

#![allow(
    unsafe_code,
    reason = "COM server FFI: CoInitializeEx, CoRegisterClassObject, IClassFactory vtable"
)]

mod com_server;
pub(crate) mod handoff;
pub(crate) mod registry;

#[allow(
    unused_imports,
    reason = "wired into main.rs by Phase 4 -Embedding detection"
)]
pub(crate) use com_server::run_com_server;

#[cfg(test)]
mod tests;
