//! Adopted PTY handle for the Windows Default Terminal handoff path.
//!
//! When the Windows console host hands a session off to `ori_term` via
//! the `ITerminalHandoff3` COM interface, `ori_term` receives
//! pre-existing OS pipe handles instead of spawning its own PTY. There
//! is no `portable_pty::MasterPty` and no spawned child process — the
//! console host owns the child lifecycle. This module provides
//! [`AdoptedPtyHandle`], a [`PtyLifecycle`] implementor that wraps the
//! adopted reader, writer, and signal handles, plus [`AdoptedSignal`],
//! the platform-specific wrapper for the conhost
//! signal/reference/server/client `HANDLE` quartet.
//!
//! On non-Windows targets [`AdoptedSignal`] compiles to a stub with no
//! handles and no `unsafe` code — the `oriterm_mux` adopt path remains
//! cross-platform so it can be exercised by unit tests on Linux and
//! macOS, even though the production handoff is Windows-only.

use std::io::{self, Read, Write};
use std::sync::{Arc, Condvar, Mutex};

use super::{ExitStatus, PtyLifecycle};

/// Shared state used by [`AdoptedPtyHandle::wait`] to block until the
/// reader thread observes EOF.
///
/// The `Mutex<Option<ExitStatus>>` is `None` while the I/O stream is
/// open and becomes `Some(_)` once `deliver_exit` is called (typically
/// from the PTY reader thread on EOF). The `Condvar` notifies any
/// thread parked in [`PtyLifecycle::wait`].
pub(crate) type ExitSignal = Arc<(Mutex<Option<ExitStatus>>, Condvar)>;

/// Adopted PTY handle — wraps pre-existing reader/writer/signal handles
/// from a Windows console handoff.
///
/// Constructed from raw OS handles by the COM server (Phase 3, Section
/// 03.9). Implements [`PtyLifecycle`] so it can live behind the same
/// `Box<dyn PtyLifecycle + Send>` boundary as a spawned [`PtyHandle`].
///
/// `kill()` is a no-op (no spawned child to kill). `wait()` blocks on
/// the [`ExitSignal`] until the reader thread reports EOF.
///
/// [`PtyHandle`]: super::PtyHandle
pub struct AdoptedPtyHandle {
    /// PTY output reader (handed to the reader thread via `take_reader`).
    reader: Option<Box<dyn Read + Send>>,
    /// PTY input writer (handed to the writer thread via `take_writer`).
    writer: Option<Box<dyn Write + Send>>,
    /// Platform-specific signal/reference/server/client handles.
    signal: Option<AdoptedSignal>,
    /// Client process ID reported by the console host (informational).
    client_pid: Option<u32>,
    /// Shared exit signal — set by the reader thread on EOF, polled by
    /// `wait`/`try_wait`. Cloned via `clone_exit_signal` for the reader
    /// thread.
    exit_signal: ExitSignal,
}

impl AdoptedPtyHandle {
    /// Construct a new adopted PTY handle.
    pub fn new(
        reader: Box<dyn Read + Send>,
        writer: Box<dyn Write + Send>,
        signal: AdoptedSignal,
        client_pid: Option<u32>,
    ) -> Self {
        Self {
            reader: Some(reader),
            writer: Some(writer),
            signal: Some(signal),
            client_pid,
            exit_signal: Arc::new((Mutex::new(None), Condvar::new())),
        }
    }

    /// Take the PTY output reader. Returns `None` if already taken.
    pub fn take_reader(&mut self) -> Option<Box<dyn Read + Send>> {
        self.reader.take()
    }

    /// Take the PTY input writer. Returns `None` if already taken.
    pub fn take_writer(&mut self) -> Option<Box<dyn Write + Send>> {
        self.writer.take()
    }

    /// Take the platform-specific signal/reference/server/client handles.
    /// Returns `None` if already taken.
    pub fn take_signal(&mut self) -> Option<AdoptedSignal> {
        self.signal.take()
    }

    /// Get the client process ID reported by the console host.
    pub fn process_id(&self) -> Option<u32> {
        self.client_pid
    }

    /// Clone the exit signal for use by the reader thread.
    ///
    /// The reader thread calls [`AdoptedPtyHandle::deliver_exit`] on the
    /// returned `ExitSignal` when its read loop exits (EOF or error),
    /// which wakes any threads parked in [`PtyLifecycle::wait`].
    pub(crate) fn clone_exit_signal(&self) -> ExitSignal {
        Arc::clone(&self.exit_signal)
    }

    /// Signal that the I/O stream has closed.
    ///
    /// Stores [`ExitStatus::synthesized_eof`] in the shared `Mutex` and
    /// notifies all threads parked in [`PtyLifecycle::wait`]. Idempotent —
    /// repeated calls overwrite the stored status with the same value.
    pub(crate) fn deliver_exit(signal: &ExitSignal) {
        let (lock, cvar) = &**signal;
        if let Ok(mut guard) = lock.lock() {
            *guard = Some(ExitStatus::synthesized_eof());
            cvar.notify_all();
        }
    }
}

impl PtyLifecycle for AdoptedPtyHandle {
    fn kill(&mut self) -> io::Result<()> {
        // ori_term did not spawn the child process — the console host
        // that handed the session off owns child lifecycle. `kill` is a
        // successful no-op so `Pane::Drop` can call it uniformly.
        Ok(())
    }

    fn wait(&mut self) -> io::Result<ExitStatus> {
        let (lock, cvar) = &*self.exit_signal;
        let guard = lock.lock().map_err(poisoned_exit_signal)?;
        let final_guard = cvar
            .wait_while(guard, |state| state.is_none())
            .map_err(poisoned_exit_signal)?;
        // The wait_while predicate guarantees `Some(_)` here.
        final_guard
            .clone()
            .ok_or_else(|| io::Error::other("AdoptedPtyHandle exit_signal vacated"))
    }

    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        let (lock, _cvar) = &*self.exit_signal;
        let guard = lock.lock().map_err(poisoned_exit_signal)?;
        Ok(guard.clone())
    }

    fn process_id(&self) -> Option<u32> {
        Self::process_id(self)
    }
}

/// Map a `PoisonError` to an `io::Error` describing the poisoned mutex.
///
/// Extracted as a free function so the `wait`/`try_wait` callers don't
/// need wildcard `|_|` closures (`clippy::map-err-ignore`).
#[cold]
fn poisoned_exit_signal<T>(_err: std::sync::PoisonError<T>) -> io::Error {
    io::Error::other("AdoptedPtyHandle exit_signal poisoned")
}

// AdoptedSignal — platform-specific wrapper for conhost handoff handles.

#[cfg(windows)]
mod windows_signal {
    //! Windows implementation: owns the duplicated `signal`, `reference`,
    //! `server`, and `client` `HANDLE`s from `ITerminalHandoff3`.
    //!
    //! The handles are duplicated by the COM `EstablishPtyHandoff` callback
    //! (Phase 3) before the method returns, because the caller-owned
    //! handles are freed by COM after the call. `AdoptedSignal::Drop`
    //! closes the duplicated handles.

    #![allow(
        unsafe_code,
        reason = "wraps Win32 HANDLE FFI for handoff signal/reference/server/client handles"
    )]

    use std::io;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Storage::FileSystem::WriteFile;

    /// Conhost signal pipe message type for window resize, mirroring
    /// `PTY_SIGNAL_RESIZE_WINDOW` from
    /// `terminal/src/winconpty/winconpty.h:49`. The packet layout is
    /// three little-endian `u16`s: `[message_type, cols, rows]` (X then
    /// Y), per `_ResizePseudoConsole` in
    /// `terminal/src/winconpty/winconpty.cpp:266`.
    const PTY_SIGNAL_RESIZE_WINDOW: u16 = 8;

    /// Owned conhost handoff handles. All four are duplicated copies — the
    /// originals are owned by the COM caller and freed when the handoff
    /// method returns.
    pub struct AdoptedSignal {
        signal: HANDLE,
        reference: HANDLE,
        server: HANDLE,
        client: HANDLE,
    }

    impl AdoptedSignal {
        /// Construct from already-duplicated handles.
        ///
        /// # Safety
        /// All four handles must be duplicated copies that this type now
        /// owns exclusively. Drop calls `CloseHandle` on each. Passing
        /// the original (non-duplicated) caller-owned handles will cause
        /// double-free.
        pub unsafe fn from_duplicated_handles(
            signal: HANDLE,
            reference: HANDLE,
            server: HANDLE,
            client: HANDLE,
        ) -> Self {
            Self {
                signal,
                reference,
                server,
                client,
            }
        }

        /// Test-only constructor producing a stub with null handles.
        ///
        /// `Drop` skips `CloseHandle` for null handles, so this is safe to
        /// drop without leaking or double-freeing. `resize` returns an
        /// error for stubs because the null signal pipe cannot be
        /// written to.
        #[cfg(test)]
        pub fn stub_for_tests() -> Self {
            Self {
                signal: std::ptr::null_mut(),
                reference: std::ptr::null_mut(),
                server: std::ptr::null_mut(),
                client: std::ptr::null_mut(),
            }
        }

        /// Send a resize message through the conhost signal pipe so the
        /// adopted console session updates its window dimensions.
        ///
        /// Mirrors `_ResizePseudoConsole` from Windows Terminal's
        /// `winconpty.cpp`: writes a 6-byte packet of three `u16`s
        /// (`[PTY_SIGNAL_RESIZE_WINDOW, cols, rows]`) to the signal
        /// pipe handle. Returns an error if the signal handle is null
        /// (e.g. a test stub) or `WriteFile` fails.
        pub fn resize(&self, rows: u16, cols: u16) -> io::Result<()> {
            if self.signal.is_null() {
                return Err(io::Error::other(
                    "AdoptedSignal::resize called on a null signal handle",
                ));
            }
            let packet: [u16; 3] = [PTY_SIGNAL_RESIZE_WINDOW, cols, rows];
            // SAFETY: as_ptr() yields a valid pointer to a stack array;
            // size_of_val gives the exact byte length. The transmute to
            // *const u8 is safe because [u16; 3] has well-defined layout
            // and is repr(Rust) which has consistent size.
            let bytes_ptr: *const u8 = packet.as_ptr().cast();
            let mut written: u32 = 0;
            // SAFETY: signal is a valid duplicated handle owned by this
            // struct. WriteFile takes a HANDLE, a pointer to bytes, the
            // length in bytes, an out-pointer for bytes-written, and an
            // optional OVERLAPPED (we pass null for synchronous write).
            let success = unsafe {
                WriteFile(
                    self.signal,
                    bytes_ptr,
                    size_of_val(&packet) as u32,
                    &raw mut written,
                    std::ptr::null_mut(),
                )
            };
            if success != 0 && written as usize == size_of_val(&packet) {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        }
    }

    impl Drop for AdoptedSignal {
        fn drop(&mut self) {
            for h in [self.signal, self.reference, self.server, self.client] {
                if !h.is_null() {
                    // SAFETY: each handle is a duplicated copy owned by
                    // this type per the `from_duplicated_handles` contract,
                    // and is closed exactly once on Drop.
                    unsafe {
                        CloseHandle(h);
                    }
                }
            }
        }
    }

    // SAFETY: Windows `HANDLE` is `*mut c_void` (not auto-Send). The four
    // handles owned by `AdoptedSignal` are duplicated copies obtained via
    // `DuplicateHandle` and are exclusively owned by this struct — no
    // other thread holds aliasing references. Sending the struct between
    // threads transfers ownership of the handles atomically. Sync is also
    // safe because the struct exposes no `&self` operations on the raw
    // handles — only `Drop` (`&mut self`) ever touches them.
    unsafe impl Send for AdoptedSignal {}
    unsafe impl Sync for AdoptedSignal {}
}

#[cfg(not(windows))]
mod stub_signal {
    //! Non-Windows stub — `AdoptedSignal` is a unit struct with no fields
    //! and no `unsafe` code. The COM handoff path only exists on Windows;
    //! on other platforms `AdoptedPtyHandle` still compiles so the adopt
    //! path is unit-testable cross-platform.

    use std::io;

    /// Empty stub. Constructing one is a logic error on non-Windows
    /// production paths, but `stub_for_tests` exists so cross-platform
    /// unit tests can exercise `AdoptedPtyHandle` without Windows
    /// `HANDLE` types.
    pub struct AdoptedSignal {
        _private: (),
    }

    impl AdoptedSignal {
        /// Test-only constructor.
        #[cfg(test)]
        pub fn stub_for_tests() -> Self {
            Self { _private: () }
        }

        /// Resize stub — there is no signal pipe on non-Windows targets,
        /// so the IO thread treats this as "no resize sink available"
        /// and skips the call. Returns an error so the caller can log.
        ///
        /// Takes `&self` to mirror the Windows signature even though the
        /// stub does not read any state — this keeps the IO thread's
        /// fallback chain (`pty_control` → `adopted_signal`) shape
        /// consistent across platforms.
        #[allow(
            clippy::unused_self,
            reason = "mirrors Windows signature for cross-platform call sites"
        )]
        pub fn resize(&self, _rows: u16, _cols: u16) -> io::Result<()> {
            Err(io::Error::other(
                "AdoptedSignal::resize is not supported on this platform",
            ))
        }
    }
}

#[cfg(windows)]
pub use windows_signal::AdoptedSignal;

#[cfg(not(windows))]
pub use stub_signal::AdoptedSignal;

#[cfg(test)]
mod tests;
