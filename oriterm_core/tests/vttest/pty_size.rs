//! PTY size propagation test.

use std::io::Read;
use std::thread;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

#[test]
#[cfg(unix)]
fn pty_size_is_propagated() {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 33,
            cols: 97,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("open PTY");

    let mut cmd = CommandBuilder::new("stty");
    cmd.arg("size");

    // Get reader BEFORE spawning so we capture all output.
    // Use a background thread because on macOS/BSD, once the slave closes
    // the master returns EIO immediately — data must be read while the
    // child is still running.
    let mut reader = pair.master.try_clone_reader().expect("reader");
    let mut child = pair.slave.spawn_command(cmd).expect("spawn stty");
    drop(pair.slave);

    let reader_handle = thread::spawn(move || {
        let mut output = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => output.extend_from_slice(&buf[..n]),
            }
        }
        String::from_utf8_lossy(&output).into_owned()
    });

    let _ = child.wait();
    let output = reader_handle.join().expect("reader thread panicked");
    let trimmed = output.trim();
    assert_eq!(
        trimmed, "33 97",
        "PTY size should be 33 rows x 97 cols, got: {trimmed}"
    );
}
