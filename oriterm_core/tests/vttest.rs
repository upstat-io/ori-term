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

    /// Block until data arrives or timeout expires, then drain everything.
    fn drain_blocking(&mut self, timeout_ms: u64) -> usize {
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
    /// drain and sleep. Important for multi-step handshakes (DA1 →
    /// CSI 18t) where vttest sends queries after receiving responses.
    fn wait(&mut self, quiet_ms: u64) {
        loop {
            if self.drain_blocking(quiet_ms) == 0 {
                break;
            }
        }
    }

    /// Wait until the grid contains `needle`, with a hard timeout.
    fn wait_for(&mut self, needle: &str, timeout_ms: u64) {
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
    fn send(&mut self, key: &[u8]) {
        self.writer.write_all(key).expect("write key");
        self.writer.flush().expect("flush");
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
        "PTY size should be 33 rows × 97 cols, got: {trimmed}"
    );
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

    // Wait for main menu to fully render.
    s.wait_for("Enter choice number", 5000);
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

    // Wait for main menu to fully render.
    s.wait_for("Enter choice number", 5000);

    // Select menu item 2.
    s.send(b"2\r");

    // Walk through all sub-screens.
    let mut screen = 1;
    loop {
        let text = s.grid_text();

        if text.contains("Enter choice number") {
            break;
        }

        // Structural assertions for specific screens.
        match screen {
            11 => {
                assert!(
                    text.contains("bottom of the screen"),
                    "{label} screen 11: should contain 'bottom of the screen'"
                );
            }
            12 => {
                let first_line = text.lines().next().unwrap_or("");
                assert!(
                    first_line.contains("top of the screen"),
                    "{label} screen 12: first line should contain 'top of the screen', \
                     got: {first_line:?}"
                );
            }
            15 => {
                // SAVE/RESTORE cursor test: "5 x 4 A's filling the top left."
                let lines: Vec<&str> = text.lines().collect();
                for row in 0..4 {
                    assert!(
                        lines[row].starts_with("AAAAA"),
                        "{label} screen 15: row {row} should start with 'AAAAA', \
                         got: {:?}",
                        &lines[row][..10.min(lines[row].len())]
                    );
                }
            }
            _ => {}
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
fn vttest_menu2_97x33() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu2_screen_features(97, 33);
}

#[test]
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

    // Wait for main menu to fully render.
    s.wait_for("Enter choice number", 5000);

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
fn vttest_border_fills_97x33() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    let grid = capture_border_screen(97, 33);
    assert_border_fills_terminal(&grid, 97, 33);
}

#[test]
fn vttest_border_fills_120x40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    let grid = capture_border_screen(120, 40);
    assert_border_fills_terminal(&grid, 120, 40);
}

// -- DECCOLM: screen 02 is the 132-column version of the border --
//
// vttest menu 1 draws the border twice: pass 0 at min_cols (screen 01)
// and pass 1 at max_cols with DECCOLM set (screen 02). DECCOLM does NOT
// resize the grid (design decision: reflow at current width). Screen 02
// content designed for 132 cols wraps at the current width.

/// Capture screen 02 (132-col pass) from menu 1 and verify side effects.
fn capture_deccolm_screen(cols: u16, rows: u16) -> Vec<Vec<char>> {
    let mut s = VtTestSession::new(cols, rows);
    s.wait_for("Enter choice number", 5000);
    s.send(b"1\r");

    // Screen 01 (min_cols border) — skip it.
    s.send(b"\r");

    // Screen 02 (max_cols=132 border after DECCOLM set).
    grid_chars(&s.term)
}

#[test]
fn vttest_deccolm_resizes_to_132_with_mode_40() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    let grid = capture_deccolm_screen(80, 24);
    // Mode 40 is enabled in the vttest session setup, so DECCOLM
    // resizes the grid to 132 columns.
    assert_eq!(
        grid[0].len(),
        132,
        "DECCOLM should resize grid to 132 columns when Mode 40 is enabled"
    );
}

// -- Menu 3: Character Sets --
//
// Tests DEC Special Graphics (line drawing), UK National, US ASCII,
// and G0/G1 designation/invocation via SO/SI.

/// DEC Special Graphics box-drawing characters used by vttest.
const LINE_DRAWING_CHARS: &[char] = &[
    '┌', // l: top-left corner
    '┐', // k: top-right corner
    '└', // m: bottom-left corner
    '┘', // j: bottom-right corner
    '─', // q: horizontal line
    '│', // x: vertical line
    '├', // t: T-junction right
    '┤', // u: T-junction left
    '┬', // w: T-junction down
    '┴', // v: T-junction up
    '┼', // n: cross
];

/// Check that the grid contains at least `min_count` distinct
/// DEC Special Graphics line-drawing characters.
fn assert_has_line_drawing_chars(grid: &[Vec<char>], min_count: usize, context: &str) {
    let mut found: std::collections::HashSet<char> = std::collections::HashSet::new();
    for row in grid {
        for &ch in row {
            if LINE_DRAWING_CHARS.contains(&ch) {
                found.insert(ch);
            }
        }
    }
    assert!(
        found.len() >= min_count,
        "{context}: expected at least {min_count} distinct line-drawing chars, \
         found {}: {found:?}",
        found.len()
    );
}

/// Walk sub-screens of a vttest menu-3 sub-item, returning the number
/// of screens captured. Applies line-drawing structural assertions.
fn walk_menu3_subscreens(
    s: &mut VtTestSession,
    label: &str,
    sub_item: &str,
    tag: &str,
    check_line_drawing: bool,
) -> (usize, bool) {
    let mut screen = 1;
    let mut saw_line_drawing = false;
    loop {
        let text = s.grid_text();

        // Return to sub-menu means all screens captured.
        if text.contains("Enter choice number") {
            break;
        }

        if check_line_drawing {
            let grid = grid_chars(&s.term);
            let has_drawing = grid
                .iter()
                .any(|row| row.iter().any(|ch| LINE_DRAWING_CHARS.contains(ch)));
            if has_drawing {
                saw_line_drawing = true;
                assert_has_line_drawing_chars(
                    &grid,
                    3,
                    &format!("{label} {sub_item} screen {screen}"),
                );
            }
        }

        insta::assert_snapshot!(format!("{label}_03_{tag}_{screen:02}"), text);

        s.send(b"\r");
        screen += 1;

        if screen > 20 {
            break;
        }
    }
    (screen - 1, saw_line_drawing)
}

/// Run vttest menu 3 (character sets) at a given size, capturing all screens.
///
/// Menu 3 has a sub-menu. We test sub-items 8 (VT100 character sets)
/// and 9 (Shift In/Shift Out).
fn run_menu3_character_sets(cols: u16, rows: u16) {
    let mut s = VtTestSession::new(cols, rows);
    let label = s.size_label();

    // Wait for main menu.
    s.wait_for("Enter choice number", 5000);

    // Select menu 3: Character Sets.
    s.send(b"3\r");

    // Wait for sub-menu.
    s.wait_for("Menu 3", 3000);
    insta::assert_snapshot!(format!("{label}_03_menu"), s.grid_text());

    // Sub-item 8: Test VT100 Character Sets (DEC Special Graphics).
    s.send(b"8\r");
    let (count_8, saw_drawing) = walk_menu3_subscreens(&mut s, &label, "sub8", "vt100cs", true);
    assert!(
        count_8 > 0,
        "{label}: sub-item 8 should have at least one screen"
    );
    assert!(
        saw_drawing,
        "{label}: VT100 Character Sets should contain DEC Special Graphics line-drawing characters"
    );

    // Sub-item 9: Test Shift In/Shift Out (SI/SO).
    s.send(b"9\r");
    let (count_9, _) = walk_menu3_subscreens(&mut s, &label, "sub9", "siso", false);
    assert!(
        count_9 > 0,
        "{label}: sub-item 9 should have at least one screen"
    );

    // Exit sub-menu back to main menu.
    s.send(b"0\r");
}

#[test]
fn vttest_menu3_80x24() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu3_character_sets(80, 24);
}

// -- Menu 8: VT102 Features (Insert/Delete Char/Line) --

/// Run vttest menu 8 (VT102 features) at a given size, capturing all screens.
///
/// Menu 8 tests ICH (insert character), DCH (delete character),
/// IL (insert line), and DL (delete line).
fn run_menu8_vt102(cols: u16, rows: u16) {
    let mut s = VtTestSession::new(cols, rows);
    let label = s.size_label();

    // Wait for main menu.
    s.wait_for("Enter choice number", 5000);

    // Select menu 8: VT102 features.
    s.send(b"8\r");

    // Walk through all sub-screens.
    let mut screen = 1;
    loop {
        let text = s.grid_text();

        if text.contains("Enter choice number") {
            break;
        }

        // Structural assertions for ICH/DCH/IL/DL screens.
        let grid = grid_chars(&s.term);
        assert_vt102_screen_structure(&grid, &text, screen, &label);

        insta::assert_snapshot!(format!("{label}_08_vt102_{screen:02}"), text);

        s.send(b"\r");
        screen += 1;

        if screen > 20 {
            break;
        }
    }

    assert!(
        screen > 1,
        "{label}: menu 8 should have at least one screen"
    );
}

/// Structural assertions for VT102 menu 8 screens.
fn assert_vt102_screen_structure(grid: &[Vec<char>], _text: &str, screen: usize, label: &str) {
    match screen {
        // Screen 2: IL/DL accordion result — A's on top, X's on bottom.
        2 => {
            assert!(
                grid[0].iter().all(|&c| c == 'A'),
                "{label} screen 2: top row should be all A's"
            );
            let last = grid.len() - 1;
            assert!(
                grid[last].iter().all(|&c| c == 'X'),
                "{label} screen 2: bottom row should be all X's"
            );
        }
        // Screen 3: Insert Mode — first char 'A', last non-space 'B'.
        3 => {
            assert_eq!(
                grid[0][0], 'A',
                "{label} screen 3: first char should be 'A'"
            );
            let last_nonspace = grid[0].iter().rposition(|&c| c != ' ');
            if let Some(pos) = last_nonspace {
                assert_eq!(
                    grid[0][pos], 'B',
                    "{label} screen 3: last non-space char should be 'B'"
                );
            }
        }
        // Screen 4: Delete Character — row 0 starts with "AB".
        4 => {
            assert_eq!(
                grid[0][0], 'A',
                "{label} screen 4: row 0 col 0 should be 'A'"
            );
            assert_eq!(
                grid[0][1], 'B',
                "{label} screen 4: row 0 col 1 should be 'B'"
            );
        }
        // Screen 5: DCH stagger — each row shorter by 1 on the right.
        5 => {
            let len0 = grid[0].iter().rposition(|&c| c != ' ').unwrap_or(0);
            let len1 = grid[1].iter().rposition(|&c| c != ' ').unwrap_or(0);
            assert!(
                len0 > len1,
                "{label} screen 5: row 0 should be longer than row 1 \
                 (stagger), got len0={len0}, len1={len1}"
            );
        }
        _ => {}
    }
}

#[test]
fn vttest_menu8_80x24() {
    if !vttest_available() {
        eprintln!("vttest not installed, skipping");
        return;
    }
    run_menu8_vt102(80, 24);
}
