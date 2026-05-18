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

    // Take the writer up-front (before spawning the child) and hold
    // it alive past child.wait + master drop on BOTH platforms.
    //
    // POSIX: dropping the writer before child.wait sends EOF data
    // into the kernel that interleaves with the slave's output
    // stream — on macOS this corrupts `stty size`'s `33 97\n` with
    // junk bytes and produces `ParseIntError` (same root cause as
    // the macOS race documented in
    // `crates/portable-pty/examples/whoami.rs`).
    //
    // Windows ConPTY: `crates/portable-pty/src/win/psuedocon.rs`
    // sets `PSUEDOCONSOLE_INHERIT_CURSOR`, so ConPTY emits `\x1b[6n`
    // (cursor position request) at startup AND blocks subsequent
    // master output until a CPR response lands on the writer. We
    // pre-stage the CPR response into the pipe BEFORE spawn_command
    // so ConPTY can satisfy its DSR-wait on first read, before any
    // teardown race window opens. (POSIX must not write to the
    // master — slave TTYs echo input, so the CPR bytes would echo
    // back into stty's output and break the parse.)
    let writer = pair.master.take_writer().expect("take stdin writer");
    #[cfg(windows)]
    let writer = {
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
fn windows_pty_size_probe_command() -> CommandBuilder {
    // PowerShell's `$Host.UI.RawUI.WindowSize` calls
    // `GetConsoleScreenBufferInfo` on the underlying ConPTY-attached
    // console, which reflects the size the master opened the PTY with
    // (per `portable_pty::win::psuedocon::PsuedoCon::new`'s COORD
    // argument). Output is plain stdout via `Write-Output` — no
    // dependency on conhost's console-buffer→VT translation path,
    // which `mode con` relied on and which the GitHub-Actions
    // `windows-latest` runner image stopped routing reliably to
    // ConPTY master since ~2026-05-13 (mode con's "Status for device
    // CON: ... Lines: N Columns: N" rows stopped reaching the master
    // pipe; only the cmd.exe init+shutdown handshake leaked through).
    //
    // The exact output shape is two `KEY: VALUE` lines:
    //
    //     LINES=33
    //     COLUMNS=97
    //
    // Which `parse_pwsh_size_output` matches on case-insensitively,
    // ignoring any ANSI prefix conhost may interleave on the same
    // line (mirrors the original parser's robustness contract).
    let mut cmd = CommandBuilder::new("powershell");
    cmd.arg("-NoProfile");
    cmd.arg("-NonInteractive");
    cmd.arg("-Command");
    cmd.arg(
        "$s = $Host.UI.RawUI.WindowSize; \
         Write-Output (\"LINES=\" + $s.Height); \
         Write-Output (\"COLUMNS=\" + $s.Width)",
    );
    cmd
}

#[cfg(windows)]
fn parse_pwsh_size_output(raw: &str) -> (u16, u16) {
    // ConPTY can interleave DECSET/DECRST + cursor-visibility escapes
    // (`\x1b[?25l`, `\x1b[?9001l`, etc.) into the line containing the
    // `LINES=` / `COLUMNS=` markers. Substring-search the lowercased
    // line so the parser is robust to any ANSI prefix.
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
        if let Some(idx) = lower.find("lines=") {
            rows = value_after(idx + "lines=".len());
        } else if let Some(idx) = lower.find("columns=") {
            cols = value_after(idx + "columns=".len());
        }
    }
    (
        rows.unwrap_or_else(|| panic!("pwsh probe missing LINES= in {raw:?}")),
        cols.unwrap_or_else(|| panic!("pwsh probe missing COLUMNS= in {raw:?}")),
    )
}

/// Regression: Windows PTY size propagation test removed.
/// Pins `portable_pty::native_pty_system()` ConPTY path: `openpty` with
/// `PtySize { rows, cols }` delivers the requested size via
/// `CreatePseudoConsole`. Uses a PowerShell one-liner that queries
/// `$Host.UI.RawUI.WindowSize` (a `GetConsoleScreenBufferInfo` wrapper)
/// and prints `LINES=N` / `COLUMNS=N` to stdout. Two cases (33×97 and
/// 50×40) clamp the matrix per the Unix counterpart.
#[test]
#[cfg(windows)]
fn pty_size_propagation_windows_pwsh_reports_correct_dimensions() {
    assert_pty_reports_size(
        33,
        97,
        windows_pty_size_probe_command(),
        parse_pwsh_size_output,
    );
    assert_pty_reports_size(
        50,
        40,
        windows_pty_size_probe_command(),
        parse_pwsh_size_output,
    );
}

/// Regression: nightly CI captured ConPTY-interleaved DECSET/DECRST
/// escapes immediately before the `LINES=` / `COLUMNS=` markers on the
/// same line. Pin substring-search behavior so this exact captured
/// shape parses to (50, 40).
#[test]
#[cfg(windows)]
fn parse_pwsh_size_output_handles_ansi_prefixed_label_line() {
    let raw = "\u{1b}[6n\u{1b}[?9001h\u{1b}[?1004h\u{1b}[m\u{1b}[?25h\r\n\u{1b}[?25lLINES=50\r\nCOLUMNS=40\r\n\u{1b}[?25h";
    assert_eq!(parse_pwsh_size_output(raw), (50, 40));
}
