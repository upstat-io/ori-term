//! vttest golden snapshot tests.
//!
//! Spawns `vttest` in a real PTY, feeds its output through `Term`'s VTE
//! parser, sends scripted keystrokes to navigate menus, and captures grid
//! snapshots at each test screen. Snapshots are compared against insta
//! golden references.
//!
//! Tests run at multiple terminal sizes (80×24, 97×33, 120×40) to catch
//! size-dependent bugs in cursor positioning, origin mode, and border drawing.
//!
//! Requires `vttest` installed (`sudo apt install vttest`).
//!
//! Run: `cargo test -p oriterm_core --test vttest`

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use oriterm_core::event::{Event, EventListener};
use oriterm_core::{Term, Theme};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

/// Event listener that captures `PtyWrite` responses so they can be
/// written back to the PTY, completing DA/DSR query-response handshakes.
struct PtyResponder {
    responses: Arc<Mutex<Vec<String>>>,
}

impl PtyResponder {
    fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn take_responses(&self) -> Vec<String> {
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
struct VtTestSession {
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
    writer: Box<dyn Write + Send>,
    term: Term<PtyResponder>,
    proc: vte::ansi::Processor,
    cols: u16,
    rows: u16,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl VtTestSession {
    /// Spawn vttest at the given terminal size.
    fn new(cols: u16, rows: u16) -> Self {
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
        let term = Term::new(rows as usize, cols as usize, 0, Theme::default(), listener);
        let proc = vte::ansi::Processor::new();

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
    fn drain(&mut self) -> usize {
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

    /// Wait until no new PTY output arrives for `quiet_ms`.
    fn wait(&mut self, quiet_ms: u64) {
        loop {
            thread::sleep(Duration::from_millis(quiet_ms));
            if self.drain() == 0 {
                break;
            }
        }
    }

    /// Send bytes to vttest and wait for the screen to settle.
    fn send(&mut self, key: &[u8]) {
        self.writer.write_all(key).expect("write key");
        self.writer.flush().expect("flush");
        thread::sleep(Duration::from_millis(100));
        self.wait(300);
    }

    /// Serialize the visible grid to text, preserving full width.
    fn grid_text(&self) -> String {
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
    fn size_label(&self) -> String {
        format!("{}x{}", self.cols, self.rows)
    }
}

/// Check if vttest is installed.
fn vttest_available() -> bool {
    std::process::Command::new("vttest")
        .arg("--help")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

/// Run vttest menu 1 (cursor movement) at a given size, capturing all screens.
fn run_menu1_cursor_movement(cols: u16, rows: u16) {
    let mut s = VtTestSession::new(cols, rows);
    let label = s.size_label();

    // Wait for main menu.
    s.wait(300);
    insta::assert_snapshot!(format!("{label}_00_main_menu"), s.grid_text());

    // Select menu item 1.
    s.send(b"1\r");

    // Walk through all sub-screens.
    let mut screen = 1;
    loop {
        let text = s.grid_text();

        if text.contains("Enter choice number") {
            break;
        }

        insta::assert_snapshot!(format!("{label}_01_cursor_{screen:02}"), text);

        s.send(b"\r");
        screen += 1;

        if screen > 20 {
            break;
        }
    }

    assert!(
        screen > 1,
        "{label}: should have captured at least one screen"
    );
}

#[test]
fn vttest_menu1_80x24() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu1_cursor_movement(80, 24);
}

#[test]
fn vttest_menu1_97x33() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu1_cursor_movement(97, 33);
}

#[test]
fn vttest_menu1_120x40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu1_cursor_movement(120, 40);
}

/// Run vttest menu 2 (screen features) at a given size, capturing all screens.
fn run_menu2_screen_features(cols: u16, rows: u16) {
    let mut s = VtTestSession::new(cols, rows);
    let label = s.size_label();

    // Wait for main menu.
    s.wait(300);

    // Select menu item 2.
    s.send(b"2\r");

    // Walk through all sub-screens.
    let mut screen = 1;
    loop {
        let text = s.grid_text();

        if text.contains("Enter choice number") {
            break;
        }

        insta::assert_snapshot!(format!("{label}_02_screen_{screen:02}"), text);

        s.send(b"\r");
        screen += 1;

        if screen > 20 {
            break;
        }
    }

    assert!(
        screen > 1,
        "{label}: should have captured at least one screen"
    );
}

#[test]
fn vttest_menu2_80x24() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu2_screen_features(80, 24);
}

#[test]
#[ignore = "DA1 response lacks VT220+ class — vttest falls back to 80 cols (Section 01)"]
fn vttest_menu2_97x33() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu2_screen_features(97, 33);
}

#[test]
#[ignore = "DA1 response lacks VT220+ class — vttest falls back to 80 cols (Section 01)"]
fn vttest_menu2_120x40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu2_screen_features(120, 40);
}

// -- Structural verification tests --
//
// These don't rely on snapshot comparison — they programmatically verify
// that the vttest border screen fills the entire terminal at any size.

/// Extract the grid as a 2D `Vec<Vec<char>>` from a `Term`.
fn grid_chars(term: &Term<PtyResponder>) -> Vec<Vec<char>> {
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

/// Verify the vttest screen-01 border fills the entire terminal.
///
/// Expected pattern for an `cols × rows` terminal:
/// ```text
/// Row 0:        * * * * ... * * * *     (all `*`, width = cols)
/// Row 1:        * + + + ... + + + *     (`*` edges, `+` interior)
/// Rows 2..R-3:  * +             + *     (`*` col 0, `+` col 1, `+` col C-2, `*` col C-1)
/// Row R-2:      * + + + ... + + + *     (same as row 1)
/// Row R-1:      * * * * ... * * * *     (same as row 0)
/// ```
fn assert_border_fills_terminal(grid: &[Vec<char>], cols: usize, rows: usize) {
    assert_eq!(grid.len(), rows, "grid should have {rows} rows");
    for row in grid {
        assert_eq!(row.len(), cols, "each row should have {cols} columns");
    }

    // Row 0: all `*`.
    for (c, &ch) in grid[0].iter().enumerate() {
        assert_eq!(ch, '*', "row 0, col {c}: expected '*', got '{ch}'");
    }

    // Row rows-1: all `*`.
    let last = rows - 1;
    for (c, &ch) in grid[last].iter().enumerate() {
        assert_eq!(ch, '*', "row {last}, col {c}: expected '*', got '{ch}'");
    }

    // Row 1: `*` at edges, `+` in between.
    assert_eq!(grid[1][0], '*', "row 1, col 0: expected '*'");
    assert_eq!(
        grid[1][cols - 1],
        '*',
        "row 1, col {}: expected '*'",
        cols - 1
    );
    for c in 1..cols - 1 {
        assert_eq!(
            grid[1][c], '+',
            "row 1, col {c}: expected '+', got '{}'",
            grid[1][c]
        );
    }

    // Row rows-2: `*` at edges, `+` in between.
    let pen = rows - 2;
    assert_eq!(grid[pen][0], '*', "row {pen}, col 0: expected '*'");
    assert_eq!(
        grid[pen][cols - 1],
        '*',
        "row {pen}, col {}: expected '*'",
        cols - 1
    );
    for c in 1..cols - 1 {
        assert_eq!(
            grid[pen][c], '+',
            "row {pen}, col {c}: expected '+', got '{}'",
            grid[pen][c]
        );
    }

    // Interior rows 2..rows-3: border characters at edges.
    for r in 2..rows - 2 {
        assert_eq!(
            grid[r][0], '*',
            "row {r}, col 0: expected '*', got '{}'",
            grid[r][0]
        );
        assert_eq!(
            grid[r][1], '+',
            "row {r}, col 1: expected '+', got '{}'",
            grid[r][1]
        );
        assert_eq!(
            grid[r][cols - 2],
            '+',
            "row {r}, col {}: expected '+', got '{}'",
            cols - 2,
            grid[r][cols - 2]
        );
        assert_eq!(
            grid[r][cols - 1],
            '*',
            "row {r}, col {}: expected '*', got '{}'",
            cols - 1,
            grid[r][cols - 1]
        );
    }
}

/// Navigate vttest to screen 01 (the border test) and return the grid.
fn capture_border_screen(cols: u16, rows: u16) -> Vec<Vec<char>> {
    let mut s = VtTestSession::new(cols, rows);

    // Wait for main menu.
    s.wait(300);

    // Select menu 1, wait for first sub-screen.
    s.send(b"1\r");

    grid_chars(&s.term)
}

#[test]
fn vttest_border_fills_80x24() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    let grid = capture_border_screen(80, 24);
    assert_border_fills_terminal(&grid, 80, 24);
}

#[test]
#[ignore = "DA1 response lacks VT220+ class — vttest falls back to 80 cols (Section 01)"]
fn vttest_border_fills_97x33() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    let grid = capture_border_screen(97, 33);
    assert_border_fills_terminal(&grid, 97, 33);
}

#[test]
#[ignore = "DA1 response lacks VT220+ class — vttest falls back to 80 cols (Section 01)"]
fn vttest_border_fills_120x40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    let grid = capture_border_screen(120, 40);
    assert_border_fills_terminal(&grid, 120, 40);
}

// -- Origin mode: screen 02 must be identical to screen 01 --
//
// vttest draws the same border with and without origin mode (DECOM).
// If origin mode is implemented correctly, the output is byte-identical.

/// Capture screens 01 and 02 from menu 1 and return both grids.
fn capture_border_screens_01_and_02(cols: u16, rows: u16) -> (Vec<Vec<char>>, Vec<Vec<char>>) {
    let mut s = VtTestSession::new(cols, rows);
    s.wait(300);
    s.send(b"1\r");

    let screen_01 = grid_chars(&s.term);

    // Advance to screen 02.
    s.send(b"\r");

    let screen_02 = grid_chars(&s.term);

    (screen_01, screen_02)
}

#[test]
#[ignore = "Origin mode (DECOM) cursor offset incorrect with scroll regions (Section 02)"]
fn vttest_origin_mode_matches_normal_80x24() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    let (s01, s02) = capture_border_screens_01_and_02(80, 24);

    for r in 0..s01.len() {
        for c in 0..s01[r].len() {
            assert_eq!(
                s01[r][c], s02[r][c],
                "origin mode mismatch at row {r}, col {c}: \
                 normal='{}', origin='{}'",
                s01[r][c], s02[r][c],
            );
        }
    }
}

#[test]
#[ignore = "Origin mode (DECOM) cursor offset incorrect with scroll regions (Section 02)"]
fn vttest_origin_mode_matches_normal_97x33() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    let (s01, s02) = capture_border_screens_01_and_02(97, 33);

    for r in 0..s01.len() {
        for c in 0..s01[r].len() {
            assert_eq!(
                s01[r][c], s02[r][c],
                "origin mode mismatch at row {r}, col {c}: \
                 normal='{}', origin='{}'",
                s01[r][c], s02[r][c],
            );
        }
    }
}

#[test]
#[ignore = "Origin mode (DECOM) cursor offset incorrect with scroll regions (Section 02)"]
fn vttest_origin_mode_matches_normal_120x40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    let (s01, s02) = capture_border_screens_01_and_02(120, 40);

    for r in 0..s01.len() {
        for c in 0..s01[r].len() {
            assert_eq!(
                s01[r][c], s02[r][c],
                "origin mode mismatch at row {r}, col {c}: \
                 normal='{}', origin='{}'",
                s01[r][c], s02[r][c],
            );
        }
    }
}
