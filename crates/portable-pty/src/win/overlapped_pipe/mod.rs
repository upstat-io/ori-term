//! Overlapped duplex named pipe for ConPTY transport.
//!
//! Ports Microsoft Terminal's `Utils::CreateOverlappedPipe`
//! (`~/projects/reference_repos/console_repos/terminal/src/types/utils.cpp:770-885`).
//! Returns a `(server, client)` pair where `client` is passed to BOTH
//! `hInput` and `hOutput` of `CreatePseudoConsole` matching
//! `ConptyConnection.cpp:406-407`.
//!
//! Required for correct ConPTY transport on Windows: a split synchronous
//! pipe pair loses APC byte sequences from the child's WSL bridge while
//! CSI bytes flow through. A single overlapped duplex named pipe with
//! the client handle shared between `hInput` and `hOutput` preserves the
//! full byte stream.
//!
//! Implementation notes:
//! - `bInheritHandle = FALSE` on BOTH server and client handles. ConPTY
//!   child inherits the client handle through `ProcThreadAttribute_PseudoConsole`
//!   (set in `psuedocon.rs::spawn_command`), NOT Win32 handle inheritance.
//! - Distinct `hEvent` per OVERLAPPED struct (one for read, one for write
//!   per handle) — required by `GetOverlappedResultSameThread` contract
//!   (utils.cpp:887-916).
//! - `try_clone` allocates FRESH events for the clone (never duplicates
//!   the existing events — that would cause cross-signaling).

use std::io::{Error as IoError, ErrorKind, Read, Result as IoResult, Write};
use std::os::windows::io::{AsRawHandle, FromRawHandle, RawHandle};
use std::ptr;

use filedescriptor::OwnedHandle;
use winapi::shared::minwindef::{BOOL, DWORD, FALSE, TRUE, ULONG};
use winapi::shared::ntdef::{HANDLE, LARGE_INTEGER, NTSTATUS, NT_SUCCESS, OBJ_CASE_INSENSITIVE, PVOID, USHORT};
use winapi::um::ioapiset::{CancelIoEx, GetOverlappedResult};
use winapi::um::minwinbase::OVERLAPPED;
use winapi::um::synchapi::{CreateEventW, WaitForSingleObject};
use winapi::um::winbase::INFINITE;
use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE, SYNCHRONIZE};

#[cfg(all(test, target_os = "windows"))]
mod tests;

/// Access mode for the overlapped duplex pipe.
#[derive(Clone, Copy)]
#[allow(dead_code)] // InboundOnly + OutboundOnly are part of the WT-parity API
                    // surface but unused by ConPTY's Duplex-only consumption.
                    // Keeping them preserves utils.cpp's API shape.
pub enum PipeAccess {
    /// Server reads, client writes.
    InboundOnly,
    /// Server writes, client reads.
    OutboundOnly,
    /// Both endpoints can read and write — required for ConPTY's
    /// single-handle-for-both-input-and-output usage.
    Duplex,
}

/// A `(server, client)` pair returned by [`OverlappedPipe::new`].
///
/// The `server` side stays with the parent (wrapped as
/// [`OverlappedHandle`] with Read+Write traits for blocking I/O over
/// overlapped operations). The `client` side is passed to
/// `CreatePseudoConsole` as both `hInput` and `hOutput`, then dropped —
/// ConPTY duplicates the handle internally into the host process so the
/// parent-side `client` is consumable.
pub struct OverlappedPipe {
    pub server: OverlappedHandle,
    pub client: ClientHandle,
}

impl OverlappedPipe {
    /// Constructs a new overlapped duplex named pipe matching WT's
    /// `Utils::CreateOverlappedPipe(access, buffer_size)`.
    ///
    /// - `access`: `PipeAccess::Duplex` for ConPTY (matches utils.cpp:814).
    /// - `buffer_size`: 128 KiB for ConPTY consumers (matches
    ///   `ConptyConnection.cpp:406`'s `128 * 1024`).
    pub fn new(access: PipeAccess, buffer_size: u32) -> IoResult<Self> {
        let (server_access, client_access) = desired_access_pair(access);

        // Open `\Device\NamedPipe\` once as the relative-name root for
        // the anonymous-named pipe creation (utils.cpp:826-841).
        let pipe_directory = open_named_pipe_directory()?;

        // Create the server end (NtCreateNamedPipeFile) — anonymous within
        // the pipe directory. utils.cpp:843-867.
        let server_handle = create_named_pipe_server(
            &pipe_directory,
            server_access,
            buffer_size,
        )?;

        // Open the client end (NtCreateFile) — relative to the server
        // handle's anonymous name. utils.cpp:869-882.
        let client_handle = open_named_pipe_client(
            &server_handle,
            client_access,
        )?;

        let server = OverlappedHandle::new(server_handle)?;
        let client = ClientHandle::new(client_handle)?;
        Ok(Self { server, client })
    }
}

fn desired_access_pair(access: PipeAccess) -> (DWORD, DWORD) {
    // Mirrors the switch at utils.cpp:799-820. For PIPE_ACCESS_DUPLEX
    // both endpoints get full duplex access (utils.cpp:815-816).
    match access {
        PipeAccess::InboundOnly => (
            SYNCHRONIZE | GENERIC_READ | FILE_WRITE_ATTRIBUTES,
            SYNCHRONIZE | GENERIC_WRITE | FILE_READ_ATTRIBUTES,
        ),
        PipeAccess::OutboundOnly => (
            SYNCHRONIZE | GENERIC_WRITE | FILE_READ_ATTRIBUTES,
            SYNCHRONIZE | GENERIC_READ | FILE_WRITE_ATTRIBUTES,
        ),
        PipeAccess::Duplex => (
            SYNCHRONIZE | GENERIC_READ | GENERIC_WRITE,
            SYNCHRONIZE | GENERIC_READ | GENERIC_WRITE,
        ),
    }
}

fn open_named_pipe_directory() -> IoResult<OwnedHandle> {
    let mut name_buf: Vec<u16> = "\\Device\\NamedPipe\\".encode_utf16().collect();
    let mut unicode_name = UNICODE_STRING {
        Length: (name_buf.len() * 2) as USHORT,
        MaximumLength: (name_buf.len() * 2) as USHORT,
        Buffer: name_buf.as_mut_ptr(),
    };
    let mut object_attributes = OBJECT_ATTRIBUTES {
        Length: std::mem::size_of::<OBJECT_ATTRIBUTES>() as ULONG,
        RootDirectory: ptr::null_mut(),
        ObjectName: &mut unicode_name,
        Attributes: OBJ_CASE_INSENSITIVE,
        SecurityDescriptor: ptr::null_mut(),
        SecurityQualityOfService: ptr::null_mut(),
    };
    let mut handle: HANDLE = ptr::null_mut();
    let mut io_status_block = IO_STATUS_BLOCK::zeroed();

    let status = unsafe {
        NtOpenFile(
            &mut handle,
            SYNCHRONIZE | GENERIC_READ,
            &mut object_attributes,
            &mut io_status_block,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            0,
        )
    };

    if !NT_SUCCESS(status) {
        return Err(IoError::new(
            ErrorKind::Other,
            format!("NtOpenFile(\\Device\\NamedPipe\\) failed: status=0x{status:08x}"),
        ));
    }

    // SAFETY: NT API returned a valid HANDLE.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle as RawHandle) })
}

fn create_named_pipe_server(
    pipe_directory: &OwnedHandle,
    desired_access: DWORD,
    buffer_size: u32,
) -> IoResult<OwnedHandle> {
    // Anonymous-named pipe (utils.cpp:843-867): empty ObjectName rooted
    // under \Device\NamedPipe\ via RootDirectory.
    let mut empty_buf: [u16; 1] = [0];
    let mut empty_name = UNICODE_STRING {
        Length: 0,
        MaximumLength: 0,
        Buffer: empty_buf.as_mut_ptr(),
    };
    let mut object_attributes = OBJECT_ATTRIBUTES {
        Length: std::mem::size_of::<OBJECT_ATTRIBUTES>() as ULONG,
        RootDirectory: pipe_directory.as_raw_handle() as HANDLE,
        ObjectName: &mut empty_name,
        Attributes: OBJ_CASE_INSENSITIVE, // NEVER OBJ_INHERIT.
        SecurityDescriptor: ptr::null_mut(),
        SecurityQualityOfService: ptr::null_mut(),
    };
    let mut handle: HANDLE = ptr::null_mut();
    let mut io_status_block = IO_STATUS_BLOCK::zeroed();
    // 50-second default timeout in 100-ns intervals, negative = relative.
    // Matches utils.cpp's default DEFAULT_PIPE_TIMEOUT.
    let mut default_timeout: LARGE_INTEGER = unsafe { std::mem::zeroed() };
    unsafe { *default_timeout.QuadPart_mut() = -500_000_000i64 };

    let status = unsafe {
        NtCreateNamedPipeFile(
            &mut handle,
            desired_access,
            &mut object_attributes,
            &mut io_status_block,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            FILE_CREATE,
            0, // CreateOptions = 0 → overlapped, not synchronous (utils.cpp:860).
            FILE_PIPE_BYTE_STREAM_TYPE,
            FILE_PIPE_BYTE_STREAM_MODE,
            FILE_PIPE_QUEUE_OPERATION,
            1, // MaximumInstances.
            buffer_size,
            buffer_size,
            &mut default_timeout,
        )
    };

    if !NT_SUCCESS(status) {
        return Err(IoError::new(
            ErrorKind::Other,
            format!("NtCreateNamedPipeFile failed: status=0x{status:08x}"),
        ));
    }

    Ok(unsafe { OwnedHandle::from_raw_handle(handle as RawHandle) })
}

fn open_named_pipe_client(
    server: &OwnedHandle,
    desired_access: DWORD,
) -> IoResult<OwnedHandle> {
    let mut empty_buf: [u16; 1] = [0];
    let mut empty_name = UNICODE_STRING {
        Length: 0,
        MaximumLength: 0,
        Buffer: empty_buf.as_mut_ptr(),
    };
    let mut object_attributes = OBJECT_ATTRIBUTES {
        Length: std::mem::size_of::<OBJECT_ATTRIBUTES>() as ULONG,
        RootDirectory: server.as_raw_handle() as HANDLE,
        ObjectName: &mut empty_name,
        Attributes: OBJ_CASE_INSENSITIVE,
        SecurityDescriptor: ptr::null_mut(),
        SecurityQualityOfService: ptr::null_mut(),
    };
    let mut handle: HANDLE = ptr::null_mut();
    let mut io_status_block = IO_STATUS_BLOCK::zeroed();

    let status = unsafe {
        NtCreateFile(
            &mut handle,
            desired_access,
            &mut object_attributes,
            &mut io_status_block,
            ptr::null_mut(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            FILE_OPEN,
            FILE_NON_DIRECTORY_FILE,
            ptr::null_mut(),
            0,
        )
    };

    if !NT_SUCCESS(status) {
        return Err(IoError::new(
            ErrorKind::Other,
            format!("NtCreateFile(client) failed: status=0x{status:08x}"),
        ));
    }

    Ok(unsafe { OwnedHandle::from_raw_handle(handle as RawHandle) })
}

fn create_manual_reset_event() -> IoResult<OwnedHandle> {
    let handle =
        unsafe { CreateEventW(ptr::null_mut(), TRUE /* manual reset */, FALSE, ptr::null()) };
    if handle.is_null() {
        return Err(IoError::last_os_error());
    }
    Ok(unsafe { OwnedHandle::from_raw_handle(handle as RawHandle) })
}

/// Server-side overlapped pipe handle with synchronous Read+Write traits.
///
/// Each `OverlappedHandle` owns TWO independent `OVERLAPPED` event
/// objects (one for read, one for write) so concurrent read+write from
/// different threads each have their own completion event — matches
/// WT's distinct-hEvent contract (utils.cpp:887-916).
pub struct OverlappedHandle {
    handle: OwnedHandle,
    read_event: OwnedHandle,
    write_event: OwnedHandle,
}

// HANDLE is a Win32 kernel object reference, safe to move across threads.
// Concurrent reads/writes use distinct OVERLAPPED + hEvent per call, so
// no aliased mutable state crosses threads.
unsafe impl Send for OverlappedHandle {}
unsafe impl Sync for OverlappedHandle {}

impl OverlappedHandle {
    fn new(handle: OwnedHandle) -> IoResult<Self> {
        Ok(Self {
            handle,
            read_event: create_manual_reset_event()?,
            write_event: create_manual_reset_event()?,
        })
    }

    #[cfg(test)]
    pub(crate) fn read_event_raw(&self) -> RawHandle {
        self.read_event.as_raw_handle()
    }

    #[cfg(test)]
    pub(crate) fn write_event_raw(&self) -> RawHandle {
        self.write_event.as_raw_handle()
    }

    /// Clones the FILE handle via DuplicateHandle(DUPLICATE_SAME_ACCESS)
    /// and allocates FRESH manual-reset events for the clone.
    ///
    /// Do NOT `DuplicateHandle` the existing events — duplicated event
    /// handles share the underlying kernel object, causing cross-signaling
    /// across clones. Each clone owns its own independent events.
    pub fn try_clone(&self) -> IoResult<Self> {
        let handle = self
            .handle
            .try_clone()
            .map_err(|e| IoError::new(ErrorKind::Other, format!("DuplicateHandle failed: {e}")))?;
        Ok(Self {
            handle,
            read_event: create_manual_reset_event()?,
            write_event: create_manual_reset_event()?,
        })
    }
}

impl AsRawHandle for OverlappedHandle {
    fn as_raw_handle(&self) -> RawHandle {
        self.handle.as_raw_handle()
    }
}

impl Read for OverlappedHandle {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        overlapped_read(
            self.handle.as_raw_handle(),
            self.read_event.as_raw_handle(),
            buf,
        )
    }
}

impl Write for OverlappedHandle {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        overlapped_write(
            self.handle.as_raw_handle(),
            self.write_event.as_raw_handle(),
            buf,
        )
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}

/// Client-side overlapped pipe handle.
///
/// Production code drops `client` after `PsuedoCon::new` (ConPTY
/// duplicates the handle internally); the event fields are test-path
/// only — used for cross-endpoint I/O in `OverlappedPipe`'s sibling
/// tests where the test owns both ends.
pub struct ClientHandle {
    handle: OwnedHandle,
    read_event: OwnedHandle,
    write_event: OwnedHandle,
}

unsafe impl Send for ClientHandle {}
unsafe impl Sync for ClientHandle {}

impl ClientHandle {
    fn new(handle: OwnedHandle) -> IoResult<Self> {
        Ok(Self {
            handle,
            read_event: create_manual_reset_event()?,
            write_event: create_manual_reset_event()?,
        })
    }
}

impl AsRawHandle for ClientHandle {
    fn as_raw_handle(&self) -> RawHandle {
        self.handle.as_raw_handle()
    }
}

impl Read for ClientHandle {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        overlapped_read(
            self.handle.as_raw_handle(),
            self.read_event.as_raw_handle(),
            buf,
        )
    }
}

impl Write for ClientHandle {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        overlapped_write(
            self.handle.as_raw_handle(),
            self.write_event.as_raw_handle(),
            buf,
        )
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}

fn overlapped_read(handle: RawHandle, event: RawHandle, buf: &mut [u8]) -> IoResult<usize> {
    use winapi::um::fileapi::ReadFile;
    let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
    overlapped.hEvent = event as HANDLE;
    let mut bytes_read: DWORD = 0;
    let ok = unsafe {
        ReadFile(
            handle as HANDLE,
            buf.as_mut_ptr() as *mut _,
            buf.len() as DWORD,
            &mut bytes_read,
            &mut overlapped,
        )
    };
    finish_overlapped(handle, &mut overlapped, ok, bytes_read)
}

fn overlapped_write(handle: RawHandle, event: RawHandle, buf: &[u8]) -> IoResult<usize> {
    use winapi::um::fileapi::WriteFile;
    let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
    overlapped.hEvent = event as HANDLE;
    let mut bytes_written: DWORD = 0;
    let ok = unsafe {
        WriteFile(
            handle as HANDLE,
            buf.as_ptr() as *const _,
            buf.len() as DWORD,
            &mut bytes_written,
            &mut overlapped,
        )
    };
    finish_overlapped(handle, &mut overlapped, ok, bytes_written)
}

fn finish_overlapped(
    handle: RawHandle,
    overlapped: &mut OVERLAPPED,
    initial_result: BOOL,
    initial_bytes: DWORD,
) -> IoResult<usize> {
    if initial_result != 0 {
        // Synchronous completion (rare for overlapped handles but possible).
        return Ok(initial_bytes as usize);
    }

    let err = unsafe { winapi::um::errhandlingapi::GetLastError() };
    const ERROR_IO_PENDING: DWORD = 997;
    if err != ERROR_IO_PENDING {
        return Err(IoError::from_raw_os_error(err as i32));
    }

    // Wait for the operation to complete via the dedicated event, then
    // collect the result. GetOverlappedResult(bWait=TRUE) blocks until
    // the OVERLAPPED structure is fully written by the kernel.
    let wait_status = unsafe { WaitForSingleObject(overlapped.hEvent, INFINITE) };
    if wait_status != 0 {
        // The wait failed (invalid event handle, etc.) but the kernel may
        // still hold a pointer to this stack-allocated OVERLAPPED. Cancel
        // the pending I/O AND drain the cancellation via GetOverlappedResult
        // before unwinding, otherwise the kernel can dereference the stack
        // OVERLAPPED + buffer after the frame goes away (use-after-free).
        unsafe { CancelIoEx(handle as HANDLE, overlapped) };
        let mut drained: DWORD = 0;
        unsafe {
            GetOverlappedResult(handle as HANDLE, overlapped, &mut drained, TRUE);
        }
        return Err(IoError::new(
            ErrorKind::Other,
            format!("WaitForSingleObject on overlapped event failed: status={wait_status}"),
        ));
    }

    let mut bytes_transferred: DWORD = 0;
    let ok =
        unsafe { GetOverlappedResult(handle as HANDLE, overlapped, &mut bytes_transferred, TRUE) };
    if ok == 0 {
        return Err(IoError::last_os_error());
    }
    Ok(bytes_transferred as usize)
}

// ---------------------------------------------------------------------------
// NT API FFI declarations.
//
// These symbols are NOT in winapi 0.3. Manual `extern "system"` block keeps
// the FFI surface minimal and avoids adding the `ntapi` Cargo dependency.
// ---------------------------------------------------------------------------

#[repr(C)]
#[allow(non_snake_case)]
struct UNICODE_STRING {
    Length: USHORT,
    MaximumLength: USHORT,
    Buffer: *mut u16,
}

#[repr(C)]
#[allow(non_snake_case)]
struct OBJECT_ATTRIBUTES {
    Length: ULONG,
    RootDirectory: HANDLE,
    ObjectName: *mut UNICODE_STRING,
    Attributes: ULONG,
    SecurityDescriptor: PVOID,
    SecurityQualityOfService: PVOID,
}

#[repr(C)]
#[allow(non_snake_case)]
struct IO_STATUS_BLOCK {
    StatusOrPointer: PVOID,
    Information: usize,
}

impl IO_STATUS_BLOCK {
    fn zeroed() -> Self {
        Self {
            StatusOrPointer: ptr::null_mut(),
            Information: 0,
        }
    }
}

// FILE_READ_ATTRIBUTES + FILE_WRITE_ATTRIBUTES are NOT in
// winapi::um::winnt; they live in ntifs.h. Declare locally.
const FILE_READ_ATTRIBUTES: DWORD = 0x0080;
const FILE_WRITE_ATTRIBUTES: DWORD = 0x0100;

// FILE_CREATE / FILE_OPEN for NtCreateFile (per ntifs.h).
const FILE_CREATE: ULONG = 0x00000002;
const FILE_OPEN: ULONG = 0x00000001;
const FILE_NON_DIRECTORY_FILE: ULONG = 0x00000040;

// NT named pipe type/mode/completion constants (per ntifs.h).
const FILE_PIPE_BYTE_STREAM_TYPE: ULONG = 0x0;
const FILE_PIPE_BYTE_STREAM_MODE: ULONG = 0x0;
const FILE_PIPE_QUEUE_OPERATION: ULONG = 0x0;

#[link(name = "ntdll")]
extern "system" {
    fn NtCreateNamedPipeFile(
        FileHandle: *mut HANDLE,
        DesiredAccess: ULONG,
        ObjectAttributes: *mut OBJECT_ATTRIBUTES,
        IoStatusBlock: *mut IO_STATUS_BLOCK,
        ShareAccess: ULONG,
        CreateDisposition: ULONG,
        CreateOptions: ULONG,
        NamedPipeType: ULONG,
        ReadMode: ULONG,
        CompletionMode: ULONG,
        MaximumInstances: ULONG,
        InboundQuota: ULONG,
        OutboundQuota: ULONG,
        DefaultTimeout: *mut LARGE_INTEGER,
    ) -> NTSTATUS;

    fn NtCreateFile(
        FileHandle: *mut HANDLE,
        DesiredAccess: ULONG,
        ObjectAttributes: *mut OBJECT_ATTRIBUTES,
        IoStatusBlock: *mut IO_STATUS_BLOCK,
        AllocationSize: *mut LARGE_INTEGER,
        FileAttributes: ULONG,
        ShareAccess: ULONG,
        CreateDisposition: ULONG,
        CreateOptions: ULONG,
        EaBuffer: PVOID,
        EaLength: ULONG,
    ) -> NTSTATUS;

    fn NtOpenFile(
        FileHandle: *mut HANDLE,
        DesiredAccess: ULONG,
        ObjectAttributes: *mut OBJECT_ATTRIBUTES,
        IoStatusBlock: *mut IO_STATUS_BLOCK,
        ShareAccess: ULONG,
        OpenOptions: ULONG,
    ) -> NTSTATUS;
}

