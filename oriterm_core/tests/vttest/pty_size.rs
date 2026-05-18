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

    // Take the writer up-front (before spawning the child) and, on
    // Windows ConPTY, pre-write a CPR response into the stdin pipe.
    //
    // `crates/portable-pty/src/win/psuedocon.rs` sets
    // `PSUEDOCONSOLE_INHERIT_CURSOR`. ConPTY emits `\x1b[6n` (cursor
    // position request) at startup AND blocks every subsequent byte
    // of master output until a CPR response lands on the writer.
    //
    // The CPR write MUST be staged before the child spawns, otherwise
    // the following race occurs under CI scheduler jitter:
    //
    //   1. spawn_command — cmd.exe starts, runs `mode con`, exits.
    //   2. ConPTY queues mode con's output bytes pending CPR.
    //   3. Slave-side closes (cmd exited). ConPTY begins teardown,
    //      discards queued bytes, emits only the shutdown handshake.
    //   4. The main thread finally writes CPR — too late; ConPTY is
    //      already tearing down, so the reader only ever sees the
    //      init+shutdown handshake (`\x1b[6n\x1b[?9001h\x1b[?1004h\x1b[m
    //      \x1b]0;...cmd.EXE\x07\x1b[?25h\x1b[?9001l\x1b[?1004l`),
    //      not `Lines:` / `Columns:`. Pre-writing CPR into the pipe
    //      means ConPTY can answer its own DSR-wait on first read,
    //      before any teardown race window opens.
    //
    // POSIX intentionally skips the write: slave TTYs echo input by
    // default, so writing `\x1b[1;1R` to the master would echo back
    // through the slave's output stream and prepend literal escape
    // bytes to `stty size`'s `33 97\n`, breaking the parse with
    // `ParseIntError`. POSIX `openpty` does not have ConPTY's DSR gate.
    // Take the writer up-front (before spawning the child) and, on
    // Windows ConPTY, pre-write a CPR response into the stdin pipe.
    //
    // The writer is held alive past child.wait + master drop on BOTH
    // platforms. On POSIX, dropping the writer before child.wait
    // sends EOF data into the kernel that interleaves with the
    // slave's output stream — on macOS this corrupts `stty size`'s
    // `33 97\n` with junk bytes, producing `ParseIntError` (same root
    // cause as the macOS race documented in
    // `crates/portable-pty/examples/whoami.rs`).
    let writer = pair.master.take_writer().expect("take stdin writer");
    #[cfg(windows)]
    let writer = {
        // `crates/portable-pty/src/win/psuedocon.rs` sets
        // `PSUEDOCONSOLE_INHERIT_CURSOR`. ConPTY emits `\x1b[6n`
        // (cursor position request) at startup AND blocks every
        // subsequent byte of master output until a CPR response lands
        // on the writer. Pre-stage the CPR response into the pipe
        // before spawn_command so ConPTY can answer its own DSR-wait
        // on first read, before any teardown race window opens.
        //
        // Race that the prior (post-spawn) CPR write hit under CI
        // scheduler load:
        //
        //   1. spawn_command — cmd.exe starts, runs `mode con`, exits.
        //   2. ConPTY queues mode con's output bytes pending CPR.
        //   3. Slave-side closes (cmd exited). ConPTY begins teardown,
        //      discards queued bytes, emits only the shutdown handshake.
        //   4. The main thread finally writes CPR — too late; ConPTY is
        //      already tearing down.
        //
        // POSIX intentionally skips the write: slave TTYs echo input by
        // default, so writing `\x1b[1;1R` to the master would echo back
        // through the slave's output stream and prepend literal escape
        // bytes to `stty size`'s `33 97\n`, breaking the parse with
        // `ParseIntError`. POSIX `openpty` does not have ConPTY's DSR
        // gate.
        let mut writer = writer;
        writer
            .write_all(b"\x1b[1;1R")
            .expect("pre-write CPR response to stdin");
        writer.flush().expect("flush CPR response");
        writer
    };

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

    child.wait().expect("child wait failed");
    drop(writer);
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

/// Regression: Windows PTY size propagation test removed.
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
    // Windows ConPTY interleaves DECSET/DECRST + cursor-visibility escapes
    // (`\x1b[?25l`, `\x1b[?9001l`, `\x1b[?1004l`, `\x1b[m`, etc.) into the
    // line containing the `Lines:` label. Substring-search the lowercased
    // line for the label so the parser is robust to any ANSI prefix.
    let mut rows: Option<u16> = None;
    let mut cols: Option<u16> = None;
    for line in raw.lines() {
        let lower = line.to_ascii_lowercase();
        let value_after = |label_end: usize| -> Option<u16> {
            line.get(label_end..)?
                .trim()
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        };
        if let Some(idx) = lower.find("lines:") {
            rows = value_after(idx + "lines:".len());
        } else if let Some(idx) = lower.find("columns:") {
            cols = value_after(idx + "columns:".len());
        } else if let Some(idx) = lower.find("cols:") {
            cols = value_after(idx + "cols:".len());
        }
    }
    (
        rows.unwrap_or_else(|| panic!("mode con output missing Lines: in {raw:?}")),
        cols.unwrap_or_else(|| panic!("mode con output missing Columns:/Cols: in {raw:?}")),
    )
}

/// Regression: Windows PTY size propagation test removed.
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

/// Regression: nightly CI run 25588160727 captured `mode con` output where
/// Windows ConPTY emitted `\x1b[?25l\x1b[?9001l\x1b[?1004l` immediately
/// before the `Lines:` label on the same line, with no intervening newline.
/// The previous parser used `trimmed.starts_with("lines:")` which missed
/// the label whenever ANSI escapes prefixed it. Pin substring-search
/// behavior so this exact captured shape parses to (50, 40).
#[test]
#[cfg(windows)]
fn parse_mode_con_output_handles_ansi_prefixed_label_line() {
    let raw = "\u{1b}[6n\u{1b}[?9001h\u{1b}[?1004h\u{1b}[m\u{1b}]0;C:\\Windows\\system32\\cmd.EXE\u{7}\u{1b}[?25h\r\nStatus for device CON:\r\n----------------------\r\n\u{1b}[?25l\u{1b}[?9001l\u{1b}[?1004l    Lines:          50\r\n    Columns:        40\r\n    Keyboard rate:  31\r\n    Keyboard delay: 1\r\n    Code page:      437\u{1b}[10;1H\u{1b}[?25h";
    assert_eq!(parse_mode_con_output(raw), (50, 40));
}
