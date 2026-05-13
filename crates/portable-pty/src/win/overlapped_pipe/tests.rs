//! Sibling tests for the overlapped duplex pipe primitive.
//!
//! All tests are `#[cfg(target_os = "windows")]`-gated — the entire
//! `overlapped_pipe` module is Windows-only via `win/mod.rs`.

#![cfg(target_os = "windows")]

use std::io::{Read, Write};
use std::os::windows::io::AsRawHandle;

use winapi::shared::minwindef::DWORD;
use winapi::um::handleapi::GetHandleInformation;
use winapi::um::synchapi::{ResetEvent, SetEvent, WaitForSingleObject};
use winapi::um::winbase::{HANDLE_FLAG_INHERIT, WAIT_OBJECT_0};

use super::{OverlappedPipe, PipeAccess};

const WAIT_TIMEOUT: DWORD = 0x102;

/// T3 — OverlappedPipe::new(Duplex, 128 KiB) constructs cleanly with
/// both server and client handles valid (non-null).
#[test]
fn overlapped_pipe_constructs_with_duplex_128k() {
    let pipe = OverlappedPipe::new(PipeAccess::Duplex, 128 * 1024)
        .expect("OverlappedPipe::new(Duplex, 128 KiB) succeeds");

    let server_raw = pipe.server.as_raw_handle();
    let client_raw = pipe.client.as_raw_handle();
    assert!(!server_raw.is_null(), "server handle is non-null");
    assert!(!client_raw.is_null(), "client handle is non-null");
    assert_ne!(server_raw, client_raw, "server and client handles distinct");
}

/// T4 — Read/Write round-trip across the duplex pipe endpoints.
/// Writes from server flow to client; writes from client flow to server.
#[test]
fn overlapped_pipe_roundtrips_bytes_server_to_client() {
    let pipe = OverlappedPipe::new(PipeAccess::Duplex, 128 * 1024)
        .expect("OverlappedPipe::new succeeds");
    let mut server = pipe.server;
    let mut client = pipe.client;

    // Server → Client direction.
    let payload = b"hello from server";
    let written = server.write(payload).expect("server.write succeeds");
    assert_eq!(written, payload.len());

    let mut buf = [0u8; 64];
    let read = client.read(&mut buf).expect("client.read succeeds");
    assert_eq!(read, payload.len());
    assert_eq!(&buf[..read], payload);

    // Client → Server direction (duplex semantics).
    let reply = b"reply from client";
    let written = client.write(reply).expect("client.write succeeds");
    assert_eq!(written, reply.len());

    let mut buf = [0u8; 64];
    let read = server.read(&mut buf).expect("server.read succeeds");
    assert_eq!(read, reply.len());
    assert_eq!(&buf[..read], reply);
}

/// T4 edge case — single-byte transfer.
#[test]
fn overlapped_pipe_roundtrips_single_byte() {
    let pipe = OverlappedPipe::new(PipeAccess::Duplex, 128 * 1024).unwrap();
    let mut server = pipe.server;
    let mut client = pipe.client;

    server.write(b"X").unwrap();
    let mut buf = [0u8; 4];
    let n = client.read(&mut buf).unwrap();
    assert_eq!(n, 1);
    assert_eq!(buf[0], b'X');
}

/// T5 — try_clone produces an independent usable handle.
/// The clone reads from the same kernel pipe object as the original.
#[test]
fn overlapped_pipe_try_clone_handle_works_independently() {
    let pipe = OverlappedPipe::new(PipeAccess::Duplex, 128 * 1024).unwrap();
    let mut server_clone = pipe.server.try_clone().expect("try_clone succeeds");
    let mut client = pipe.client;

    // The clone's events MUST be distinct from the original's events.
    // (Use raw handle comparison via as_raw_handle.)
    // We can't directly compare events here (they're private fields), but
    // we can verify functional independence via I/O.

    // Write from client, read from cloned server.
    client.write(b"clone-read").unwrap();
    let mut buf = [0u8; 32];
    let n = server_clone.read(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"clone-read");
}

/// T6 — Both server and client handles are NON-inheritable.
/// ConPTY inherits the client handle via ProcThreadAttribute_PseudoConsole,
/// NOT via Win32 handle inheritance. An inheritable handle on either side
/// would risk leak to grandchildren.
#[test]
fn overlapped_pipe_neither_handle_inheritable() {
    let pipe = OverlappedPipe::new(PipeAccess::Duplex, 128 * 1024).unwrap();

    let server_raw = pipe.server.as_raw_handle();
    let client_raw = pipe.client.as_raw_handle();

    let mut server_flags: DWORD = 0;
    let mut client_flags: DWORD = 0;
    let ok_server = unsafe { GetHandleInformation(server_raw as _, &mut server_flags) };
    let ok_client = unsafe { GetHandleInformation(client_raw as _, &mut client_flags) };

    assert_ne!(ok_server, 0, "GetHandleInformation(server) succeeds");
    assert_ne!(ok_client, 0, "GetHandleInformation(client) succeeds");

    assert_eq!(
        server_flags & HANDLE_FLAG_INHERIT,
        0,
        "server handle MUST NOT be inheritable"
    );
    assert_eq!(
        client_flags & HANDLE_FLAG_INHERIT,
        0,
        "client handle MUST NOT be inheritable"
    );
}

/// T4b — Cross-signal isolation: each OverlappedHandle owns distinct
/// read_event and write_event kernel objects. SetEvent on one MUST
/// leave the other unsignaled (and vice versa); if try_clone or new()
/// shared the events across read + write OVERLAPPED structs, the
/// WaitForSingleObject(other, 0) calls below would return WAIT_OBJECT_0
/// instead of WAIT_TIMEOUT.
#[test]
fn overlapped_pipe_distinct_event_isolation() {
    let pipe = OverlappedPipe::new(PipeAccess::Duplex, 128 * 1024).unwrap();
    let server = pipe.server;
    let read_h = server.read_event_raw();
    let write_h = server.write_event_raw();

    // The two event handles MUST be distinct kernel objects.
    assert_ne!(
        read_h, write_h,
        "read_event and write_event must be distinct kernel handles"
    );

    // Start with both events reset.
    unsafe { ResetEvent(read_h as _) };
    unsafe { ResetEvent(write_h as _) };

    // Signaling read_event MUST NOT signal write_event.
    unsafe { SetEvent(read_h as _) };
    assert_eq!(
        unsafe { WaitForSingleObject(read_h as _, 0) },
        WAIT_OBJECT_0,
        "read_event must be signaled after SetEvent(read_event)"
    );
    assert_eq!(
        unsafe { WaitForSingleObject(write_h as _, 0) },
        WAIT_TIMEOUT,
        "write_event must NOT be signaled when only read_event was set"
    );

    // Reset and reverse: signaling write_event MUST NOT signal read_event.
    unsafe { ResetEvent(read_h as _) };
    unsafe { SetEvent(write_h as _) };
    assert_eq!(
        unsafe { WaitForSingleObject(write_h as _, 0) },
        WAIT_OBJECT_0,
        "write_event must be signaled after SetEvent(write_event)"
    );
    assert_eq!(
        unsafe { WaitForSingleObject(read_h as _, 0) },
        WAIT_TIMEOUT,
        "read_event must NOT be signaled when only write_event was set"
    );

    // Also pin the try_clone case: cloned handle gets FRESH events
    // (NOT DuplicateHandle'd from the original — that would alias).
    let clone = server.try_clone().expect("try_clone succeeds");
    let clone_read_h = clone.read_event_raw();
    let clone_write_h = clone.write_event_raw();
    assert_ne!(
        clone_read_h, read_h,
        "cloned read_event must be a fresh kernel handle, not aliased"
    );
    assert_ne!(
        clone_write_h, write_h,
        "cloned write_event must be a fresh kernel handle, not aliased"
    );
}

/// T4b companion — concurrent read + write across threads using cloned
/// server (read) and original client (write); verifies the pipe is
/// genuinely concurrent across endpoints.
#[test]
fn overlapped_pipe_concurrent_read_write_across_endpoints() {
    use std::thread;

    let pipe = OverlappedPipe::new(PipeAccess::Duplex, 128 * 1024).unwrap();
    let mut server_clone = pipe.server.try_clone().unwrap();
    let mut client = pipe.client;
    let payload: Vec<u8> = (0..64u8).collect();
    let expected = payload.clone();

    let reader = thread::spawn(move || {
        let mut buf = vec![0u8; expected.len()];
        let mut read_total = 0;
        while read_total < expected.len() {
            let n = server_clone.read(&mut buf[read_total..]).unwrap();
            if n == 0 {
                break;
            }
            read_total += n;
        }
        assert_eq!(read_total, expected.len());
        assert_eq!(buf, expected);
    });

    // Give the reader a beat to issue ReadFile, then write from this
    // thread via the client endpoint. Both operations use distinct
    // OVERLAPPED + hEvent objects (per the distinct-event contract);
    // concurrent operation MUST succeed.
    client.write(&payload).unwrap();
    reader.join().expect("reader thread completes successfully");
}
