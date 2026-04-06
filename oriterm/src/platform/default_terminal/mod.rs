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
//!
//! ## Future phases (planned by Section 03.9 in `plans/roadmap`)
//!
//! - **Phase 3**: COM server lifecycle (`ITerminalHandoff3` implementation,
//!   `IClassFactory`, `CoRegisterClassObject`, `-Embedding` startup path).
//! - **Phase 4**: CLI subcommands (`--register-default` /
//!   `--unregister-default`) and Settings UI toggle.
//!
//! The entire module is gated `#[cfg(target_os = "windows")]` because the
//! feature has no analogue on Linux or macOS — the Windows console
//! delegation registry has no cross-platform counterpart. Non-Windows
//! callers receive "not supported" errors at the CLI/UI boundary rather
//! than from this module.

pub(crate) mod registry;
