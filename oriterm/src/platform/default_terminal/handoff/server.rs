//! `HandoffServer` — the [`ITerminalHandoff3`] + [`IDefaultTerminalMarker`]
//! COM object that receives the actual handoff from `conhost.exe`.
//!
//! `EstablishPtyHandoff` runs on an arbitrary RPC thread (not the main
//! event-loop thread). It MUST NOT touch winit, GPU, or any other
//! resource bound to a specific thread. Instead, it builds a
//! [`HandoffData`] payload and sends it through an `mpsc::Sender` to the
//! main thread, which is parked on the corresponding `Receiver` in
//! [`run_com_server`](super::super::run_com_server).

use std::fs::File;
use std::os::windows::io::FromRawHandle;
use std::sync::Mutex;

use oriterm_mux::AdoptedSignal;
use windows::Win32::Foundation;
use windows::Win32::Foundation::{DUPLICATE_SAME_ACCESS, GetLastError, HANDLE};
use windows::Win32::System::Pipes::CreatePipe;
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessId};
use windows_core::{HRESULT, implement};

use super::startup_info::HandoffData;
use super::{
    IDefaultTerminalMarker, IDefaultTerminalMarker_Impl, ITerminalHandoff3, ITerminalHandoff3_Impl,
    TERMINAL_STARTUP_INFO, from_startup_info,
};

/// `S_OK` — operation succeeded.
const S_OK: HRESULT = HRESULT(0);

/// `E_FAIL` — generic failure (returned to conhost when our handoff
/// pipeline rejects the call).
const E_FAIL: HRESULT = HRESULT(0x8000_4005_u32 as i32);

/// `E_UNEXPECTED` — unexpected internal state (e.g. handoff already
/// consumed; would only happen if conhost called `EstablishPtyHandoff`
/// twice on a single object instance, which violates the contract).
const E_UNEXPECTED: HRESULT = HRESULT(0x8000_FFFF_u32 as i32);

/// Pipe buffer size (64 KB) — matches Windows Terminal's choice. Larger
/// than the default 4 KB so a single `WriteFile` from the console host
/// doesn't block waiting for the terminal to drain.
const PIPE_BUFFER_SIZE: u32 = 64 * 1024;

/// COM object that fulfils `EstablishPtyHandoff` by sending its payload
/// to the parked main thread.
///
/// The `Mutex<Option<...>>` is `Some` until the first (and only) call to
/// `EstablishPtyHandoff` consumes it via `take()`. Subsequent calls
/// return `E_UNEXPECTED` — the conhost protocol guarantees a 1:1 mapping
/// of COM activations to handoffs (`REGCLS_SINGLEUSE`), so this should
/// never happen in practice but we defend against it anyway.
#[implement(ITerminalHandoff3, IDefaultTerminalMarker)]
pub(crate) struct HandoffServer {
    /// Channel sender to the main thread. Wrapped in `Mutex<Option<_>>`
    /// because the COM trait methods take `&self` (interior mutability)
    /// and we need to consume the sender on the first call.
    handoff_tx: Mutex<Option<std::sync::mpsc::Sender<HandoffData>>>,
}

impl HandoffServer {
    /// Construct a new `HandoffServer` bound to a one-shot channel.
    pub(crate) fn new(handoff_tx: std::sync::mpsc::Sender<HandoffData>) -> Self {
        Self {
            handoff_tx: Mutex::new(Some(handoff_tx)),
        }
    }
}

impl ITerminalHandoff3_Impl for HandoffServer_Impl {
    unsafe fn EstablishPtyHandoff(
        &self,
        in_handle: *mut HANDLE,
        out_handle: *mut HANDLE,
        signal: HANDLE,
        reference: HANDLE,
        server: HANDLE,
        client: HANDLE,
        startup_info: *const TERMINAL_STARTUP_INFO,
    ) -> HRESULT {
        // SAFETY: COM marshalling guarantees `in_handle` and `out_handle`
        // point to valid `HANDLE` slots in the caller's address space.
        // The startup_info pointer (if non-null) is also valid for the
        // duration of this call. We delegate to a safe Rust helper that
        // returns Result so we don't sprinkle error handling through the
        // unsafe block.
        let result = unsafe {
            establish_handoff(
                self,
                in_handle,
                out_handle,
                signal,
                reference,
                server,
                client,
                startup_info,
            )
        };
        match result {
            Ok(()) => S_OK,
            Err(hr) => hr,
        }
    }
}

impl IDefaultTerminalMarker_Impl for HandoffServer_Impl {}

/// Safe-ish helper for `EstablishPtyHandoff` — moves all the unsafe FFI
/// behind a single function so the COM impl stays compact and the
/// SAFETY notes are co-located with the operations they justify.
///
/// # Safety
///
/// Same contract as [`HandoffServer_Impl::EstablishPtyHandoff`]:
/// `in_handle` and `out_handle` must be valid out-pointers; `signal`,
/// `reference`, `server`, `client` must be valid Windows handles owned
/// by the COM caller; `startup_info` must point to a valid struct or
/// be null.
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors the ITerminalHandoff::EstablishPtyHandoff COM ABI method signature — param count fixed by the interface, not reducible"
)]
unsafe fn establish_handoff(
    server_impl: &HandoffServer_Impl,
    in_handle: *mut HANDLE,
    out_handle: *mut HANDLE,
    signal: HANDLE,
    reference: HANDLE,
    server: HANDLE,
    client: HANDLE,
    startup_info: *const TERMINAL_STARTUP_INFO,
) -> Result<(), HRESULT> {
    if in_handle.is_null() || out_handle.is_null() {
        return Err(E_FAIL);
    }

    // TPR-3 ordering rule: never publish the [out] pipe handles to the
    // caller until ALL fallible work has succeeded. Steps 1-9 build up
    // the full HandoffData payload (pipes, duplicated handles, parsed
    // startup info) without writing to *in_handle / *out_handle. Only
    // after the channel send succeeds do we hand the console-side ends
    // to the caller via the out-params (step 10). On any earlier
    // failure, the cleanup ladder closes pipes/handles and the caller
    // never observes a partially populated state — exactly the WT
    // pattern from `ConptyConnection.cpp`.

    // 1. Create the input pipe (console reads its stdin from `in_read`,
    // we write keystrokes into `in_write`).
    let (in_read, in_write) = create_pipe()?;
    // 2. Create the output pipe (console writes its stdout into
    // `out_write`, we read terminal output from `out_read`).
    let (out_read, out_write) = create_pipe().inspect_err(|_| {
        // SAFETY: in_read and in_write are valid handles produced by
        // CreatePipe in step 1. CloseHandle releases each one exactly
        // once and we abandon them by returning early.
        unsafe {
            close_handle(in_read);
            close_handle(in_write);
        }
    })?;

    // 3. Wrap our ends as std::fs::File trait objects. File takes
    // ownership of the raw handle and closes it on Drop, which
    // matches the lifetime of the resulting Pane. NOTE: in_read and
    // out_write are still raw HANDLEs at this point — they remain
    // closeable via close_handle on the cleanup paths until we
    // publish them to the caller in step 10.
    // SAFETY: in_write and out_read are valid handles created by
    // CreatePipe and not yet wrapped or closed. From this point on,
    // the File instances exclusively own them.
    let writer = unsafe { File::from_raw_handle(in_write.0.cast()) };
    let reader = unsafe { File::from_raw_handle(out_read.0.cast()) };

    // 4. Duplicate the [in] handles so they outlive this COM call. On
    // any failure, drop the wrapped File ends (which closes them
    // via File::Drop) AND close the still-raw console-side ends so
    // the caller never sees them.
    let dup_signal = match unsafe { duplicate_handle(signal) } {
        Ok(handle) => handle,
        Err(hr) => {
            drop(writer);
            drop(reader);
            // SAFETY: in_read and out_write are valid handles created
            // by CreatePipe and never wrapped or published.
            unsafe {
                close_handle(in_read);
                close_handle(out_write);
            }
            return Err(hr);
        }
    };
    let dup_reference = match unsafe { duplicate_handle(reference) } {
        Ok(handle) => handle,
        Err(hr) => {
            drop(writer);
            drop(reader);
            // SAFETY: dup_signal is a duplicated copy we own; in_read /
            // out_write are still owned by us — neither has been
            // published to the caller.
            unsafe {
                close_handle(dup_signal);
                close_handle(in_read);
                close_handle(out_write);
            }
            return Err(hr);
        }
    };
    let dup_server = match unsafe { duplicate_handle(server) } {
        Ok(handle) => handle,
        Err(hr) => {
            drop(writer);
            drop(reader);
            // SAFETY: dup_signal/dup_reference are duplicated copies we
            // own; in_read / out_write are still raw and unpublished.
            unsafe {
                close_handle(dup_signal);
                close_handle(dup_reference);
                close_handle(in_read);
                close_handle(out_write);
            }
            return Err(hr);
        }
    };
    let dup_client = match unsafe { duplicate_handle(client) } {
        Ok(handle) => handle,
        Err(hr) => {
            drop(writer);
            drop(reader);
            // SAFETY: as above — close every duplicated copy and the
            // still-raw console-side ends before propagating.
            unsafe {
                close_handle(dup_signal);
                close_handle(dup_reference);
                close_handle(dup_server);
                close_handle(in_read);
                close_handle(out_write);
            }
            return Err(hr);
        }
    };

    // 5. Wrap the duplicated handles in AdoptedSignal (RAII close on
    // Drop). After this point, the AdoptedSignal owns all four
    // handles and we MUST NOT manually close them.
    //
    // AdoptedSignal lives in oriterm_mux which uses windows-sys
    // HANDLE (= *mut c_void), so unwrap each windows HANDLE wrapper
    // via `.0` before passing.
    // SAFETY: AdoptedSignal::from_duplicated_handles requires four
    // duplicated copies. We just produced them via DuplicateHandle and
    // are passing them by-move (the local variables are not used after
    // this call).
    let adopted_signal = unsafe {
        AdoptedSignal::from_duplicated_handles(
            dup_signal.0,
            dup_reference.0,
            dup_server.0,
            dup_client.0,
        )
    };

    // 6. Read the client PID from the duplicated client handle. The
    // handle is now owned by AdoptedSignal, so we query before the
    // next step (which moves it).
    // SAFETY: GetProcessId is a standard Win32 query that takes any
    // process HANDLE. AdoptedSignal still holds the handle alive.
    let client_pid_raw = unsafe { GetProcessId(dup_client) };
    let client_pid = if client_pid_raw == 0 {
        None
    } else {
        Some(client_pid_raw)
    };

    // 7. Parse the startup info into owned strings + dimensions.
    // SAFETY: caller contract — startup_info is valid for the duration
    // of this call (or null, which from_startup_info handles).
    let parsed = unsafe { from_startup_info(startup_info) };

    // 8. Build the payload (no allocations beyond String fields).
    let payload = HandoffData {
        reader: Box::new(reader),
        writer: Box::new(writer),
        signal: adopted_signal,
        client_pid,
        title: parsed.title,
        icon_path: parsed.icon_path,
        initial_rows: parsed.initial_rows,
        initial_cols: parsed.initial_cols,
    };

    // 9. Send the payload through the channel. Failures here drop the
    // payload (which closes the wrapped File ends) and we still need
    // to close in_read / out_write because they were never published
    // to the caller.
    // SAFETY: in_read / out_write are still raw handles that have not
    // been published to the caller, so deliver_handoff is allowed to
    // close them on every failure path.
    unsafe { deliver_handoff(server_impl, payload, in_read, out_write) }?;

    // 10. Only NOW that the entire pipeline succeeded do we publish the
    // console-side pipe ends to the caller. From this point on, COM
    // marshalling owns these handles. If we had hit any failure
    // above, the caller would never have observed a partially
    // populated state — matching the WT pattern in
    // `ConptyConnection.cpp::_initiateConnection`.
    // SAFETY: in_handle and out_handle are valid out-pointers per the
    // function contract; we are writing initialized HANDLE values that
    // we just successfully created via CreatePipe.
    unsafe {
        *in_handle = in_read;
        *out_handle = out_write;
    }
    Ok(())
}

/// Send the constructed [`HandoffData`] payload through the one-shot
/// channel, closing the still-raw console-side pipe ends on every
/// failure path. Extracted from [`establish_handoff`] to keep that
/// function under the clippy line limit and to colocate the four
/// failure-cleanup branches in one place.
///
/// # Safety
///
/// `in_read` and `out_write` must be valid handles never yet published
/// to the caller. On error, they are closed via `close_handle`.
unsafe fn deliver_handoff(
    server_impl: &HandoffServer_Impl,
    payload: HandoffData,
    in_read: HANDLE,
    out_write: HANDLE,
) -> Result<(), HRESULT> {
    let Ok(mut guard) = server_impl.handoff_tx.lock() else {
        drop(payload);
        // SAFETY: in_read and out_write are still raw handles never
        // published to the caller per the function contract.
        unsafe {
            close_handle(in_read);
            close_handle(out_write);
        }
        return Err(E_UNEXPECTED);
    };
    let Some(tx) = guard.take() else {
        drop(payload);
        // SAFETY: see above.
        unsafe {
            close_handle(in_read);
            close_handle(out_write);
        }
        return Err(E_UNEXPECTED);
    };
    if tx.send(payload).is_err() {
        // SAFETY: see above. Receiver dropped before consuming payload.
        unsafe {
            close_handle(in_read);
            close_handle(out_write);
        }
        return Err(E_FAIL);
    }
    Ok(())
}

/// Create an anonymous pipe and return `(read, write)` handles.
fn create_pipe() -> Result<(HANDLE, HANDLE), HRESULT> {
    let mut read = HANDLE::default();
    let mut write = HANDLE::default();
    // SAFETY: read/write are valid out-pointers to local variables.
    // CreatePipe initializes both on success.
    let result = unsafe { CreatePipe(&raw mut read, &raw mut write, None, PIPE_BUFFER_SIZE) };
    if result.is_ok() {
        Ok((read, write))
    } else {
        Err(last_os_error_hresult())
    }
}

/// Duplicate a HANDLE into the current process with the same access.
///
/// # Safety
///
/// `source` must be a valid handle that this function is allowed to
/// duplicate (typically a process or pipe handle owned by the COM
/// caller).
unsafe fn duplicate_handle(source: HANDLE) -> Result<HANDLE, HRESULT> {
    if source.is_invalid() || source.0.is_null() {
        return Err(E_FAIL);
    }
    let mut target = HANDLE::default();
    // SAFETY: GetCurrentProcess returns a pseudo-handle valid for the
    // current process. source is non-null per the check above. target
    // is a valid out-pointer. DUPLICATE_SAME_ACCESS asks for an exact
    // copy of the source handle's access rights.
    let result = unsafe {
        Foundation::DuplicateHandle(
            GetCurrentProcess(),
            source,
            GetCurrentProcess(),
            &raw mut target,
            0,
            false,
            DUPLICATE_SAME_ACCESS,
        )
    };
    if result.is_ok() {
        Ok(target)
    } else {
        Err(last_os_error_hresult())
    }
}

/// Close a HANDLE, ignoring errors (used in cleanup paths).
///
/// # Safety
///
/// `handle` must be a valid HANDLE this function is allowed to close
/// exactly once. Caller must not use the handle after this call.
unsafe fn close_handle(handle: HANDLE) {
    if !handle.is_invalid() && !handle.0.is_null() {
        // SAFETY: caller contract — `handle` is a valid HANDLE this
        // function is allowed to close exactly once.
        unsafe {
            let _ = Foundation::CloseHandle(handle);
        }
    }
}

/// Convert the most recent Win32 error into an `HRESULT`.
fn last_os_error_hresult() -> HRESULT {
    // SAFETY: GetLastError is a thread-local query with no preconditions.
    let code = unsafe { GetLastError() }.0;
    // Standard Win32 → HRESULT conversion: HRESULT_FROM_WIN32(x).
    if code == 0 {
        E_FAIL
    } else {
        HRESULT(((code & 0x0000_FFFF) | 0x8007_0000) as i32)
    }
}
