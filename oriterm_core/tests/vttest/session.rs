//! Shared vttest session infrastructure: PTY management, VTE processing,
//! grid inspection, and the `VtTestSession` type used by all menu tests.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use oriterm_core::event::{Event, EventListener};
use oriterm_core::{Term, Theme};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

/// Event listener that captures `PtyWrite` responses so they can be
/// written back to the PTY, completing DA/DSR query-response handshakes.
pub struct PtyResponder {
    responses: Arc<Mutex<Vec<String>>>,
}

impl PtyResponder {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn take_responses(&self) -> Vec<String> {
        std::mem::take(&mut *self.responses.lock().unwrap())
    }
}

impl EventListener for PtyResponder {
    fn send_event(&self, event: Event) {
        if let Event::PtyWrite(data) = event {
            self.responses.lock().unwrap().push(data);
        }
    }
}

/// Holds a vttest session: PTY channel, writer, Term, and VTE processor.
pub struct VtTestSession {
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
    writer: Box<dyn Write + Send>,
    pub term: Term<PtyResponder>,
    proc: vte::ansi::Processor,
    pub cols: u16,
    pub rows: u16,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl VtTestSession {
    /// Spawn vttest at the given terminal size.
    pub fn new(cols: u16, rows: u16) -> Self {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("failed to open PTY");

        let mut cmd = CommandBuilder::new("vttest");
        // vttest hardcodes 80x24 — pass actual size as LINESxMIN_COLS.MAX_COLS.
        // max_cols=132 so vttest's pass-1 (DECCOLM set) draws at 132 columns.
        cmd.arg(format!("{rows}x{cols}.132"));
        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .expect("failed to spawn vttest");
        drop(pair.slave);

        let mut pty_reader = pair.master.try_clone_reader().expect("clone reader");
        let writer = pair.master.take_writer().expect("take writer");

        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match pty_reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let listener = PtyResponder::new();
        let mut term = Term::new(rows as usize, cols as usize, 0, Theme::default(), listener);
        let mut proc = vte::ansi::Processor::new();

        // Enable Mode 40 (ENABLE_MODE_3) so that DECCOLM (mode 3) actually
        // resizes the grid to 80/132 columns. vttest's 132-column iteration
        // relies on this for correct rendering.
        proc.advance(&mut term, b"\x1b[?40h");

        Self {
            rx,
            writer,
            term,
            proc,
            cols,
            rows,
            _child: child,
        }
    }

    /// Drain all buffered PTY output into Term, writing DA/DSR responses back.
    pub fn drain(&mut self) -> usize {
        let mut total = 0;
        while let Ok(data) = self.rx.try_recv() {
            self.proc.advance(&mut self.term, &data);
            total += data.len();

            for resp in self.term.event_listener().take_responses() {
                let _ = self.writer.write_all(resp.as_bytes());
            }
            let _ = self.writer.flush();
        }
        total
    }

    /// Block until data arrives or timeout expires, then drain everything.
    pub fn drain_blocking(&mut self, timeout_ms: u64) -> usize {
        let mut total = 0;
        // Block until the first chunk arrives (or timeout).
        if let Ok(data) = self.rx.recv_timeout(Duration::from_millis(timeout_ms)) {
            self.proc.advance(&mut self.term, &data);
            total += data.len();
            for resp in self.term.event_listener().take_responses() {
                let _ = self.writer.write_all(resp.as_bytes());
            }
            let _ = self.writer.flush();
        }
        // Drain any remaining buffered data.
        total += self.drain();
        total
    }

    /// Wait until no new PTY output arrives for `quiet_ms`.
    ///
    /// Uses blocking recv to avoid missing data that arrives between
    /// drain and sleep. Important for multi-step handshakes (DA1 ->
    /// CSI 18t) where vttest sends queries after receiving responses.
    pub fn wait(&mut self, quiet_ms: u64) {
        loop {
            if self.drain_blocking(quiet_ms) == 0 {
                break;
            }
        }
    }

    /// Wait until the grid contains `needle`, with a hard timeout.
    pub fn wait_for(&mut self, needle: &str, timeout_ms: u64) {
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            self.drain_blocking(100);
            let text = self.grid_text();
            if text.contains(needle) {
                // Drain any trailing output.
                self.wait(200);
                return;
            }
            if std::time::Instant::now() >= deadline {
                panic!(
                    "timed out waiting for {:?} after {timeout_ms}ms.\nGrid:\n{}",
                    needle,
                    self.grid_text()
                );
            }
        }
    }

    /// Send bytes to vttest and wait for the screen to settle.
    pub fn send(&mut self, key: &[u8]) {
        self.writer.write_all(key).expect("write key");
        self.writer.flush().expect("flush");
        self.wait(300);
    }

    /// Serialize the visible grid to text, preserving full width.
    pub fn grid_text(&self) -> String {
        let content = self.term.renderable_content();
        let lines = content.lines;
        let cols = content.cols;

        let mut grid = vec![vec![' '; cols]; lines];
        for cell in &content.cells {
            if cell.line < lines && cell.column.0 < cols {
                let ch = if cell.ch == '\0' { ' ' } else { cell.ch };
                grid[cell.line][cell.column.0] = ch;
            }
        }

        let mut out = String::new();
        for row in &grid {
            let line: String = row.iter().collect();
            out.push_str(&line);
            out.push('\n');
        }
        out
    }

    /// Size label for snapshot naming (e.g., "80x24").
    pub fn size_label(&self) -> String {
        format!("{}x{}", self.cols, self.rows)
    }
}

/// Extract the grid as a 2D `Vec<Vec<char>>` from a `Term`.
pub fn grid_chars(term: &Term<PtyResponder>) -> Vec<Vec<char>> {
    let content = term.renderable_content();
    let lines = content.lines;
    let cols = content.cols;

    let mut grid = vec![vec![' '; cols]; lines];
    for cell in &content.cells {
        if cell.line < lines && cell.column.0 < cols {
            let ch = if cell.ch == '\0' { ' ' } else { cell.ch };
            grid[cell.line][cell.column.0] = ch;
        }
    }
    grid
}

/// Check if vttest is installed.
pub fn vttest_available() -> bool {
    std::process::Command::new("vttest")
        .arg("--help")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}
