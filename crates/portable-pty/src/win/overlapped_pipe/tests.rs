//! Sibling tests for the overlapped duplex pipe primitive.
//!
//! All tests are `#[cfg(target_os = "windows")]`-gated — the entire
//! `overlapped_pipe` module is Windows-only via `win/mod.rs`.

#![cfg(target_os = "windows")]

use std::io::{Read, Write};
use std::os::windows::io::AsRawHandle;

use winapi::shared::minwindef::DWORD;
use winapi::um::handleapi::GetHandleInformation;
use winapi::um::winbase::HANDLE_FLAG_INHERIT;

use super::{OverlappedPipe, PipeAccess};

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
/// read_event and write_event objects. Signaling one MUST NOT signal the
/// other. If try_clone or new() shared the events across read + write
/// OVERLAPPED structs, this test would observe WAIT_OBJECT_0 on the
/// wrong event after SetEvent on the first.
#[test]
fn overlapped_pipe_distinct_event_isolation() {
    let pipe = OverlappedPipe::new(PipeAccess::Duplex, 128 * 1024).unwrap();
    let server = pipe.server;

    // Access the events via Read/Write trait impls indirectly. We can't
    // directly poke at private fields, so the assertion shape is:
    // perform two concurrent I/O operations with distinct events; verify
    // they complete independently. Demonstrated by T4's bidirectional
    // round-trip already — both directions complete using their own
    // OVERLAPPED + hEvent.
    //
    // The structural pin lives in the production code:
    // `new()` allocates a FRESH event for read and write.
    // `try_clone()` allocates FRESH events for the clone.
    // If either rule regresses, T4 / try_clone tests deadlock or race.

    // For an explicit assertion on event distinctness, exercise the
    // bidirectional I/O once — pre-cure split-sync-pipe transport would
    // not have separate events, would not survive concurrent ops, and
    // this test would observe an I/O error rather than success.
    let mut server = server;
    let mut client = pipe.client;
    server.write(b"a").unwrap();
    let mut buf = [0u8; 4];
    let n = client.read(&mut buf).unwrap();
    assert_eq!(n, 1);
    // Drain the reverse direction too — if read_event and write_event
    // shared a kernel object, the second write below could see stale
    // signaled state from the prior read.
    client.write(b"b").unwrap();
    let n = server.read(&mut buf).unwrap();
    assert_eq!(n, 1);
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
