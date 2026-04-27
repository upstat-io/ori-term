//! PTY size propagation tests — verify that `portable_pty` delivers the
//! requested rows/cols to the child process across both POSIX (`openpty` +
//! `TIOCSWINSZ`) and Windows ConPTY (`CreatePseudoConsole`) backends.
//!
//! Each platform branch runs two cases: a primary `33×97` and a
//! `50×40` negative pin proving the helper assertion is parameterized
//! and not coincidentally hardcoded against `33×97`.

#[cfg(any(unix, windows))]
use std::io::Read;
#[cfg(any(unix, windows))]
use std::thread;

#[cfg(any(unix, windows))]
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

#[cfg(any(unix, windows))]
fn assert_pty_reports_size(
    rows: u16,
    cols: u16,
    cmd: CommandBuilder,
    parse: impl FnOnce(&str) -> (u16, u16),
) {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("open PTY");

    // Mirror `crates/portable-pty/examples/whoami.rs` end-to-end —
    // the canonical reference for this exact dance. The ordering matters:
    //
    //   1. spawn child
    //   2. drop slave (we no longer need it)
    //   3. start reader thread (so the pipe drains as output arrives)
    //   4. take + drop the stdin writer in an inner block (macOS needs
    //      a 20 ms grace period BEFORE the writer drops — quoting the
    //      example: "the data we send to the kernel to trigger EOF is
    //      interleaved with the data read by the reader! WTF!?")
    //   5. child.wait()
    //   6. drop pair.master (fires ClosePseudoConsole on Windows)
    //   7. recv reader output
    //
    // Earlier versions diverged from this ordering and hung on Windows
    // (no take_writer at all → child stdin stayed open) and panicked on
    // macOS (writer dropped too soon → stty output got truncated).
    let mut reader = pair.master.try_clone_reader().expect("reader");
    let mut child = pair.slave.spawn_command(cmd).expect("spawn command");
    drop(pair.slave);

    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let mut output = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => output.extend_from_slice(&buf[..n]),
            }
        }
        let _ = tx.send(String::from_utf8_lossy(&output).into_owned());
    });

    {
        let _writer = pair.master.take_writer().expect("take stdin writer");

        // macOS quirk per whoami.rs: short-lived children race with
        // the kernel's EOF synthesis, interleaving stdin-EOF data with
        // the reader's stdout drain. The 20 ms grace lets the reader
        // make progress before the writer drops. Replace with a
        // deterministic seam if portable_pty ever ships one.
        if cfg!(target_os = "macos") {
            thread::sleep(std::time::Duration::from_millis(20));
        }
        // _writer drops here, sending stdin EOF to the child.
    }

    child.wait().expect("child wait failed");
    drop(pair.master);

    let raw = rx.recv().expect("reader channel closed without sending");
    let (got_rows, got_cols) = parse(&raw);
    assert_eq!(
        (got_rows, got_cols),
        (rows, cols),
        "PTY child should observe {rows}x{cols}; got {got_rows}x{got_cols}; raw output = {raw:?}",
    );
}

#[cfg(unix)]
fn unix_stty_size_command() -> CommandBuilder {
    let mut cmd = CommandBuilder::new("stty");
    cmd.arg("size");
    cmd
}

#[cfg(unix)]
fn parse_stty_size_output(raw: &str) -> (u16, u16) {
    let trimmed = raw.trim();
    let mut parts = trimmed.split_whitespace();
    let rows: u16 = parts
        .next()
        .expect("stty size missing rows")
        .parse()
        .expect("stty size rows not an integer");
    let cols: u16 = parts
        .next()
        .expect("stty size missing cols")
        .parse()
        .expect("stty size cols not an integer");
    (rows, cols)
}

/// Regression: BUG-07-004 — Windows PTY size propagation test removed.
/// Pins `portable_pty::native_pty_system()` POSIX path: `openpty` with
/// `PtySize { rows, cols }` delivers the requested size to the child.
/// Two cases (33×97 and 50×40) clamp the matrix from both sides — proves
/// the helper assertion is parameterized, not coincidentally hardcoded.
#[test]
#[cfg(unix)]
fn pty_size_propagation_unix_stty_reports_correct_dimensions() {
    assert_pty_reports_size(33, 97, unix_stty_size_command(), parse_stty_size_output);
    assert_pty_reports_size(50, 40, unix_stty_size_command(), parse_stty_size_output);
}

#[cfg(windows)]
fn windows_mode_con_command() -> CommandBuilder {
    let mut cmd = CommandBuilder::new("cmd");
    cmd.arg("/d");
    cmd.arg("/c");
    cmd.arg("mode con");
    cmd
}

#[cfg(windows)]
fn parse_mode_con_output(raw: &str) -> (u16, u16) {
    let mut rows: Option<u16> = None;
    let mut cols: Option<u16> = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        let value_after = |label_len: usize| -> Option<u16> {
            let rest = trimmed.get(label_len..)?.trim();
            rest.split_whitespace().next()?.parse().ok()
        };
        if lower.starts_with("lines:") {
            rows = value_after("lines:".len());
        } else if lower.starts_with("columns:") {
            cols = value_after("columns:".len());
        } else if lower.starts_with("cols:") {
            cols = value_after("cols:".len());
        }
    }
    (
        rows.unwrap_or_else(|| panic!("mode con output missing Lines: in {raw:?}")),
        cols.unwrap_or_else(|| panic!("mode con output missing Columns:/Cols: in {raw:?}")),
    )
}

/// Regression: BUG-07-004 — Windows PTY size propagation test removed.
/// Pins `portable_pty::native_pty_system()` ConPTY path: `openpty` with
/// `PtySize { rows, cols }` delivers the requested size via
/// `CreatePseudoConsole`. Uses `cmd /d /c mode con` to bypass AutoRun.
/// Two cases (33×97 and 50×40) clamp the matrix per the Unix counterpart.
///
/// Locale assumption: en-US Windows. The parser matches the literal English
/// labels `Lines:` / `Columns:` / `Cols:` emitted by `mode con`. On a
/// non-en-US host the test surfaces as `mode con output missing Lines:` —
/// a clear diagnostic, not silent skipping.
#[test]
#[cfg(windows)]
fn pty_size_propagation_windows_mode_con_reports_correct_dimensions() {
    assert_pty_reports_size(33, 97, windows_mode_con_command(), parse_mode_con_output);
    assert_pty_reports_size(50, 40, windows_mode_con_command(), parse_mode_con_output);
}
