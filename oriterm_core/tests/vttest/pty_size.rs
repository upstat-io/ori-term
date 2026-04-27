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

    // ConPTY teardown is platform-divergent enough that the Unix and
    // Windows branches use different writer-drop ordering:
    //
    //   * POSIX (Unix path, used on Linux + macOS): mirror the
    //     `crates/portable-pty/examples/whoami.rs` shape — drop the
    //     stdin writer BEFORE child.wait so the child sees stdin EOF
    //     and exits naturally. macOS needs a 20 ms grace period before
    //     the writer drops (whoami.rs's documented race; "data we send
    //     to the kernel to trigger EOF is interleaved with the data
    //     read by the reader").
    //
    //   * Windows ConPTY: keep the stdin writer alive until AFTER
    //     child.wait. Closing stdin mid-startup causes `cmd /d /c
    //     mode con` to exit before executing the inner command — only
    //     the cmd.exe init handshake (DSR query, focus mode toggle,
    //     title set) reaches the reader; mode con's own output never
    //     fires. After child.wait the child has already produced its
    //     output and exited; drop-writer-then-drop-master then drains
    //     ConPTY cleanly via ClosePseudoConsole.
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

    let writer = pair.master.take_writer().expect("take stdin writer");

    #[cfg(unix)]
    {
        // macOS race grace period (whoami.rs).
        if cfg!(target_os = "macos") {
            thread::sleep(std::time::Duration::from_millis(20));
        }
        drop(writer);
        child.wait().expect("child wait failed");
    }

    #[cfg(windows)]
    {
        // Don't drop the writer mid-startup — cmd /c quits early on
        // stdin EOF and never runs mode con. Hold the writer until
        // the child exits naturally, then drop in cleanup ordering.
        child.wait().expect("child wait failed");
        drop(writer);
    }

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
