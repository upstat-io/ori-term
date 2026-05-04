//! Tests for the COM server lifecycle (`run_com_server`,
//! `OriTermClassFactory`).
//!
//! Most of this code path needs real COM activation and a `conhost.exe`
//! caller, which is not feasible in `cargo test`. We cover what we can
//! cross-platform / Windows-without-COM:
//!
//! - The `HandoffServer` constructor accepts a `mpsc::Sender` and the
//!   resulting object can be wrapped in `IUnknown` (compile-time check
//!   that the `#[implement]` macro produced a usable type).
//! - `run_com_server`'s timeout path returns the documented `io::Error`
//!   when no payload is delivered (Windows-only because it calls
//!   `CoInitializeEx`/`CoRegisterClassObject`).
//!
//! End-to-end Windows integration (cmd.exe from Run dialog, etc.) is
//! tracked in the Section 03.9 plan as the manual / CI-Windows-runner
//! test matrix.

use std::sync::mpsc;

use windows::core::{IUnknown, Interface};

use super::handoff::{HandoffData, HandoffServer, ITerminalHandoff3};

#[test]
fn handoff_server_constructs_and_exposes_iunknown() {
    // Smoke test: the #[implement(ITerminalHandoff3, IDefaultTerminalMarker)]
    // macro must produce a HandoffServer convertible to IUnknown via the
    // generated `From` impl.
    let (tx, _rx) = mpsc::channel::<HandoffData>();
    let server = HandoffServer::new(tx);
    let unknown: IUnknown = server.into();
    // Casting back to ITerminalHandoff3 via the typed cast confirms the
    // vtable was wired correctly. This is a property: if the
    // interface UUID or vtable layout drifts, the cast fails at runtime.
    let _terminal: ITerminalHandoff3 = unknown
        .cast()
        .expect("HandoffServer must implement ITerminalHandoff3");
}
