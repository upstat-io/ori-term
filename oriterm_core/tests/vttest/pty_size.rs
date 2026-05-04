//! PTY size propagation tests — verify that `portable_pty` delivers the
//! requested rows/cols to the child process across both POSIX (`openpty` +
//! `TIOCSWINSZ`) and Windows ConPTY (`CreatePseudoConsole`) backends.
//!
//! Each platform branch runs two cases: a primary `33×97` and a
//! `50×40` regression guard proving the helper assertion is parameterized
//! and not coincidentally hardcoded against `33×97`.

#[cfg(any(unix, windows))]
use std::io::Read;
#[cfg(windows)]
use std::io::Write;
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

    // Unified teardown across POSIX + Windows. Both probes (`stty size`
    // and `cmd /d /c mode con`) print and exit without ever reading
    // stdin, so we don't need stdin-EOF to wake the child — we can wait
    // for it to exit naturally, THEN drop the writer + master. This:
    //
    //   * Removes the documented macOS race in `crates/portable-pty/
    //     examples/whoami.rs` ("data we send to the kernel to trigger
    //     EOF is interleaved with the data read by the reader") — no
    //     EOF data is sent to the kernel until after the child is gone.
    //   * Fixes the Windows ConPTY path where dropping the writer
    //     mid-startup caused `cmd /c` to quit before executing the
    //     inner command, leaking only the cmd.exe init handshake
    //     (DSR query, focus toggle, title set) to the reader.
    //
    // The whoami.rs pattern (drop writer before child.wait) is correct
    // for the general case where the child may block on stdin EOF;
    // here the workload is a deterministic short-lived probe, so the
    // simpler "wait then drop" wins on every supported platform.
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

    // Take the stdin writer so its handle lives in this scope. Windows
    // ConPTY needs a CPR response written here BEFORE child.wait;
    // POSIX must NOT write to the writer because the slave TTY would
    // echo the bytes back into stty's output stream and corrupt the
    // parse.
    let writer = pair.master.take_writer().expect("take stdin writer");

    // Windows ConPTY: answer ConPTY's DSR cursor-position query.
    //
    // `crates/portable-pty/src/win/psuedocon.rs:87` sets
    // `PSUEDOCONSOLE_INHERIT_CURSOR` on `CreatePseudoConsole`. ConPTY
    // emits `\x1b[6n` (cursor-position request) at startup AND blocks
    // every subsequent byte of master output until the response lands
    // on the writer. Without a responder the child's actual output
    // never reaches the reader — only the unconditional init handshake
    // (focus reporting toggle, screen clear, title set) does. The
    // failure mode depends entirely on writer-drop timing:
    //
    //   * writer dropped early → ConPTY abandons the DSR wait via stdin
    //     EOF, but kills `cmd /c` mid-startup so `mode con` never runs;
    //     parser sees only the init handshake and panics on missing
    //     `Lines:`.
    //   * writer held until after `child.wait` → ConPTY blocks forever
    //     waiting for DSR; child never produces output; child.wait
    //     blocks; the CI job hits its 10-minute kill.
    //
    // Writing any well-formed CPR response (`\x1b[r;cR`, 1-indexed) is
    // sufficient — ConPTY only needs the protocol question answered.
    //
    // POSIX intentionally skips this write: TTYs echo input characters
    // by default, so writing `\x1b[1;1R` to the master would echo back
    // through the slave's output stream and prepend literal escape
    // bytes to `stty size`'s `33 97\n`, breaking the parse with
    // `ParseIntError`. POSIX `openpty` does not have ConPTY's DSR
    // gate; nothing waits on a CPR response.
    //
 // Source: third-party-review consensus 2026-04-27 ( + )
    // verified the DSR origin against `psuedocon.rs:87`.
    #[cfg(windows)]
    {
        let mut writer = writer;
        writer
            .write_all(b"\x1b[1;1R")
            .expect("write CPR response to stdin");
        writer.flush().expect("flush CPR response");
        child.wait().expect("child wait failed");
        drop(writer);
    }
    #[cfg(unix)]
    {
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

/// Regression: — Windows PTY size propagation test removed.
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

/// Regression: — Windows PTY size propagation test removed.
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
