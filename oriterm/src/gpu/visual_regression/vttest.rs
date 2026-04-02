//! vttest golden image tests.
//!
//! Spawns `vttest` in a real PTY, feeds output through `Term`'s VTE parser,
//! navigates menus with scripted keystrokes, and renders each test screen
//! through the full GPU pipeline. The resulting framebuffer is compared
//! against golden reference PNGs — capturing colors, attributes, borders,
//! and character rendering.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use oriterm_core::event::{Event, EventListener};
use oriterm_core::{Rgb, Term, Theme};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use super::{compare_with_reference, headless_env, render_to_pixels};
use crate::font::CellMetrics;
use crate::gpu::frame_input::{FrameInput, FramePalette, ViewportSize};
use crate::gpu::pipelines::GpuPipelines;
use crate::gpu::state::GpuState;
use crate::gpu::window_renderer::WindowRenderer;

/// Event listener that captures `PtyWrite` responses for DA/DSR handshakes.
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

/// Holds a vttest session with PTY, Term, and GPU renderer.
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
        cmd.arg(format!("{rows}x{cols}.{cols}"));
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

    /// Drain all buffered PTY output, writing DA/DSR responses back.
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
        if let Ok(data) = self.rx.recv_timeout(Duration::from_millis(timeout_ms)) {
            self.proc.advance(&mut self.term, &data);
            total += data.len();
            for resp in self.term.event_listener().take_responses() {
                let _ = self.writer.write_all(resp.as_bytes());
            }
            let _ = self.writer.flush();
        }
        total += self.drain();
        total
    }

    /// Wait until no new PTY output for `quiet_ms`.
    fn wait(&mut self, quiet_ms: u64) {
        loop {
            if self.drain_blocking(quiet_ms) == 0 {
                break;
            }
        }
    }

    /// Wait until the grid contains `needle`.
    fn wait_for(&mut self, needle: &str, timeout_ms: u64) {
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            self.drain_blocking(100);
            if self.grid_text().contains(needle) {
                self.wait(200);
                return;
            }
            if std::time::Instant::now() >= deadline {
                panic!("timed out waiting for {:?} after {timeout_ms}ms", needle);
            }
        }
    }

    /// Send bytes and wait for screen to settle.
    fn send(&mut self, key: &[u8]) {
        self.writer.write_all(key).expect("write key");
        self.writer.flush().expect("flush");
        self.wait(300);
    }

    /// Serialize the visible grid to text for content-based waiting.
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

    /// Build a `FrameInput` from the current `Term` state.
    fn frame_input(&self, cell: CellMetrics) -> FrameInput {
        let cols = self.cols as usize;
        let rows = self.rows as usize;
        let w = (cell.width * cols as f32).ceil() as u32;
        let h = (cell.height * rows as f32).ceil() as u32;

        let content = self.term.renderable_content();

        let fg = Rgb {
            r: 211,
            g: 215,
            b: 207,
        };
        // Palette bg must differ from the cell bg so the prepare phase emits
        // bg quads. Cells have bg=(0,0,0) from the terminal, so use a
        // slightly different palette bg. The renderer clears to palette bg,
        // then draws cell bg quads on top, then glyphs.
        let palette_bg = Rgb { r: 1, g: 1, b: 1 };

        FrameInput {
            content,
            viewport: ViewportSize::new(w, h),
            cell_size: cell,
            content_cols: cols,
            content_rows: rows,
            palette: FramePalette {
                background: palette_bg,
                foreground: fg,
                cursor_color: Rgb {
                    r: 255,
                    g: 255,
                    b: 255,
                },
                opacity: 1.0,
                selection_fg: None,
                selection_bg: None,
            },
            selection: None,
            search: None,
            hovered_cell: None,
            hovered_url_segments: Vec::new(),
            mark_cursor: None,
            window_focused: true,
            fg_dim: 1.0,
            subpixel_positioning: true,
            prompt_marker_rows: Vec::new(),
        }
    }

    /// Render the current screen to pixels and compare against a golden ref.
    fn assert_golden(
        &self,
        name: &str,
        gpu: &GpuState,
        pipelines: &GpuPipelines,
        renderer: &mut WindowRenderer,
    ) {
        let cell = renderer.cell_metrics();
        let input = self.frame_input(cell);
        let w = input.viewport.width;
        let h = input.viewport.height;

        let pixels = render_to_pixels(gpu, pipelines, renderer, &input);
        if let Err(msg) = compare_with_reference(name, &pixels, w, h) {
            panic!("vttest visual regression ({name}): {msg}");
        }
    }
}

fn vttest_available() -> bool {
    std::process::Command::new("vttest")
        .arg("--help")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

/// Run vttest menu 1 (cursor movement) and capture golden images.
fn run_menu1_golden(cols: u16, rows: u16) {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let mut s = VtTestSession::new(cols, rows);
    let label = format!("{}x{}", cols, rows);

    // Wait for main menu to fully render.
    s.wait_for("Enter choice number", 5000);
    s.assert_golden(
        &format!("vttest_{label}_menu"),
        &gpu,
        &pipelines,
        &mut renderer,
    );

    // Select menu item 1.
    s.send(b"1\r");

    let mut screen = 1;
    loop {
        let text = {
            let c = s.term.renderable_content();
            let mut t = String::new();
            for cell in &c.cells {
                t.push(cell.ch);
            }
            t
        };

        if text.contains("Enter choice number") {
            break;
        }

        s.assert_golden(
            &format!("vttest_{label}_01_{screen:02}"),
            &gpu,
            &pipelines,
            &mut renderer,
        );

        s.send(b"\r");
        screen += 1;

        if screen > 20 {
            break;
        }
    }
}

/// Run vttest menu 2 (screen features) and capture golden images.
fn run_menu2_golden(cols: u16, rows: u16) {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let mut s = VtTestSession::new(cols, rows);
    let label = format!("{}x{}", cols, rows);

    s.wait_for("Enter choice number", 5000);
    s.send(b"2\r");

    let mut screen = 1;
    loop {
        let text = {
            let c = s.term.renderable_content();
            let mut t = String::new();
            for cell in &c.cells {
                t.push(cell.ch);
            }
            t
        };

        if text.contains("Enter choice number") {
            break;
        }

        s.assert_golden(
            &format!("vttest_{label}_02_{screen:02}"),
            &gpu,
            &pipelines,
            &mut renderer,
        );

        s.send(b"\r");
        screen += 1;

        if screen > 20 {
            break;
        }
    }
}

// -- Menu 1: Cursor movements --

#[test]
fn vttest_golden_menu1_80x24() {
    if !vttest_available() {
        return;
    }
    run_menu1_golden(80, 24);
}

#[test]
fn vttest_golden_menu1_97x33() {
    if !vttest_available() {
        return;
    }
    run_menu1_golden(97, 33);
}

#[test]
fn vttest_golden_menu1_120x40() {
    if !vttest_available() {
        return;
    }
    run_menu1_golden(120, 40);
}

// -- Menu 2: Screen features --

#[test]
fn vttest_golden_menu2_80x24() {
    if !vttest_available() {
        return;
    }
    run_menu2_golden(80, 24);
}

#[test]
fn vttest_golden_menu2_97x33() {
    if !vttest_available() {
        return;
    }
    run_menu2_golden(97, 33);
}

#[test]
fn vttest_golden_menu2_120x40() {
    if !vttest_available() {
        return;
    }
    run_menu2_golden(120, 40);
}
