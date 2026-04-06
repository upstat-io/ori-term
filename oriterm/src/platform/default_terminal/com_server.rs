//! COM server lifecycle for the Windows Default Terminal handoff
//! (Section 03.9 Phase 3).
//!
//! `run_com_server` runs through the canonical 9-step `-Embedding`
//! startup sequence:
//!
//! 1. (Caller) Detect `-Embedding` in `argv` and call this function
//!    instead of the normal app startup path.
//! 2. `CoInitializeEx(COINIT_MULTITHREADED)`.
//! 3. Construct an [`OriTermClassFactory`] bound to a one-shot
//!    [`mpsc::Sender`] for the eventual [`HandoffData`] payload.
//! 4. `CoRegisterClassObject(ORITERM_TERMINAL_CLSID, factory,
//!    CLSCTX_LOCAL_SERVER, REGCLS_SINGLEUSE)` — `REGCLS_SINGLEUSE`
//!    means a single COM activation revokes the registration
//!    automatically (1:1 process-to-handoff mapping).
//! 5. Block on the handoff `Receiver`. The COM RPC thread fills
//!    `EstablishPtyHandoff`, builds a [`HandoffData`], and sends it
//!    through the channel.
//! 6. Take the `HandoffData` and return it to the caller (the main
//!    thread).
//! 7. (Caller) Build the winit event loop, GPU, window, and a `Pane`
//!    backed by the adopted handles.
//! 8. (Caller) Run the event loop normally.
//! 9. (Caller) On session end, drop the `Pane` and exit. Drop closes
//!    every duplicated handle via `AdoptedSignal::Drop`.

#![allow(
    dead_code,
    reason = "consumed by Phase 4 -Embedding detection in main.rs"
)]
#![allow(
    clippy::inline_always,
    clippy::ref_as_ptr,
    clippy::transmute_ptr_to_ptr,
    clippy::transmute_undefined_repr,
    clippy::borrow_as_ptr,
    clippy::too_many_arguments,
    reason = "lints fire inside #[implement] proc-macro expansion that we cannot edit"
)]

use std::io;
use std::sync::mpsc;
use std::time::Duration;

use windows::Win32::Foundation::E_POINTER;
use windows::Win32::System::Com::{
    CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED, CoInitializeEx, CoRegisterClassObject,
    CoRevokeClassObject, IClassFactory, IClassFactory_Impl, REGCLS_SINGLEUSE,
};
use windows::core::{BOOL, GUID, IUnknown, Interface, Ref, implement};

use super::handoff::{HandoffData, HandoffServer};

/// Stable IID for `ori_term`'s `ITerminalHandoff3` COM server.
///
/// Must match the string in [`super::registry::ORITERM_TERMINAL_CLSID`]
/// byte-for-byte. The string form is what conhost reads from the
/// registry; the `GUID` form is what `CoRegisterClassObject` expects.
/// `static` so we can take its address with `&raw const` (a `const`
/// would materialize a temporary at every reference).
static ORITERM_CLSID_GUID: GUID = GUID::from_u128(0x86A2D6B1_7A4C_4F37_9C5E_9E0F0B7DBAE2);

/// Maximum time to wait for the COM RPC thread to deliver the handoff
/// payload. After this, `run_com_server` returns an error and the
/// process exits — conhost falls back to the built-in console.
///
/// 30 seconds is generous: the COM activation + `EstablishPtyHandoff`
/// round-trip normally completes in milliseconds. The timeout exists
/// to defend against a hung COM call (e.g. conhost crashed mid-handoff).
const HANDOFF_RECV_TIMEOUT: Duration = Duration::from_secs(30);

/// `IClassFactory` implementation that constructs [`HandoffServer`]
/// instances bound to the channel sender.
///
/// Each `CreateInstance` call clones the sender — but `REGCLS_SINGLEUSE`
/// guarantees only one activation will occur, so cloning is conceptually
/// free. The factory does not implement `LockServer` (returns `Ok(())`
/// without effect) because we don't manage explicit lock counts.
#[implement(IClassFactory)]
struct OriTermClassFactory {
    handoff_tx: mpsc::Sender<HandoffData>,
}

impl OriTermClassFactory {
    fn new(handoff_tx: mpsc::Sender<HandoffData>) -> Self {
        Self { handoff_tx }
    }
}

#[allow(
    non_snake_case,
    reason = "preserves COM IClassFactory method names from windows-rs trait definition"
)]
impl IClassFactory_Impl for OriTermClassFactory_Impl {
    fn CreateInstance(
        &self,
        _outer: Ref<'_, IUnknown>,
        riid: *const GUID,
        ppv_object: *mut *mut std::ffi::c_void,
    ) -> windows::core::Result<()> {
        if ppv_object.is_null() {
            return Err(windows::core::Error::from(E_POINTER));
        }
        // SAFETY: COM marshalling guarantees `riid` and `ppv_object`
        // point to valid memory the caller owns. We dereference `riid`
        // read-only and write the resulting interface pointer through
        // `ppv_object`.
        unsafe {
            *ppv_object = std::ptr::null_mut();

            let server = HandoffServer::new(self.handoff_tx.clone());
            // Convert the concrete server into IUnknown, then call
            // QueryInterface for the IID conhost requested. The server
            // implements both ITerminalHandoff3 and IDefaultTerminalMarker.
            let unknown: IUnknown = server.into();
            unknown.query(&*riid, ppv_object).ok()
        }
    }

    fn LockServer(&self, _flock: BOOL) -> windows::core::Result<()> {
        // Single-use server — lock counts have no semantic effect.
        Ok(())
    }
}

/// Entry point for the `-Embedding` startup path.
///
/// Initializes COM in MTA mode, registers the class factory, and
/// blocks until either the COM RPC thread delivers a [`HandoffData`]
/// payload or the [`HANDOFF_RECV_TIMEOUT`] expires. On success, returns
/// the payload to the caller (which constructs the event loop and
/// adopted pane). On any failure, revokes the class registration and
/// returns an `io::Error`.
pub(crate) fn run_com_server() -> io::Result<HandoffData> {
    init_com_mta()?;

    let (tx, rx) = mpsc::channel::<HandoffData>();
    let factory: IClassFactory = OriTermClassFactory::new(tx).into();

    let cookie = register_class_object(&factory)?;

    // The COM RPC thread will eventually call EstablishPtyHandoff and
    // push a HandoffData through `tx`. Block here until it arrives or
    // the timeout fires. Drop `factory` BEFORE the wait so its IUnknown
    // refcount only stays alive via the COM internal table — but that
    // would invalidate the registration. Keep it alive via `&factory`
    // implicitly (factory is owned by this stack frame).
    let result = rx.recv_timeout(HANDOFF_RECV_TIMEOUT);

    // Always revoke the class object, even on success — once we have
    // the handoff, no further activations should occur. Errors during
    // revoke are logged but don't override the primary result.
    revoke_class_object(cookie);

    match result {
        Ok(payload) => Ok(payload),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(io::Error::other(
            "default-terminal handoff timed out before EstablishPtyHandoff was called",
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(io::Error::other(
            "default-terminal handoff sender dropped before delivering a payload",
        )),
    }
}

/// Step 2: `CoInitializeEx(COINIT_MULTITHREADED)`.
fn init_com_mta() -> io::Result<()> {
    // SAFETY: CoInitializeEx is callable from any thread before any
    // other COM call. We pass NULL for the reserved parameter
    // (matching the documented contract) and request MTA so the COM
    // RPC dispatcher can deliver `EstablishPtyHandoff` on a worker
    // thread without forcing apartment marshalling.
    let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if hr.is_ok() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "CoInitializeEx(COINIT_MULTITHREADED) failed: {hr:?}"
        )))
    }
}

/// Step 4: `CoRegisterClassObject` with `REGCLS_SINGLEUSE`.
///
/// Returns the registration cookie that must be passed to
/// `CoRevokeClassObject` on shutdown.
fn register_class_object(factory: &IClassFactory) -> io::Result<u32> {
    // SAFETY: ORITERM_CLSID_GUID is a valid GUID (compile-time
    // constant). `factory` is a valid IClassFactory instance owned by
    // the caller for the duration of this call. CLSCTX_LOCAL_SERVER
    // matches our LocalServer32 registry registration. REGCLS_SINGLEUSE
    // tells COM to revoke the registration automatically after the
    // first activation, which keeps the 1:1 process-to-handoff
    // relationship.
    let result = unsafe {
        CoRegisterClassObject(
            &raw const ORITERM_CLSID_GUID,
            factory,
            CLSCTX_LOCAL_SERVER,
            REGCLS_SINGLEUSE,
        )
    };
    result.map_err(|err| {
        io::Error::other(format!(
            "CoRegisterClassObject(ORITERM_TERMINAL_CLSID) failed: {err:?}"
        ))
    })
}

/// Revoke a previously registered class object. Errors are logged but
/// not propagated — the caller is past the point where revoke failures
/// can be meaningfully handled (the process is about to either run the
/// event loop or exit with an error).
fn revoke_class_object(cookie: u32) {
    // SAFETY: cookie was returned by a successful CoRegisterClassObject
    // call. CoRevokeClassObject is the documented inverse.
    let result = unsafe { CoRevokeClassObject(cookie) };
    if let Err(err) = result {
        log::warn!("CoRevokeClassObject({cookie}) failed: {err:?}");
    }
}
