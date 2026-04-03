//! vttest golden image tests.
//!
//! Spawns `vttest` in a real PTY, feeds output through `Term`'s VTE parser,
//! navigates menus with scripted keystrokes, and renders each test screen
//! through the full GPU pipeline. The resulting framebuffer is compared
//! against golden reference PNGs — capturing colors, attributes, borders,
//! and character rendering.

mod menus_3_8;

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use oriterm_core::event::{Event, EventListener};
use oriterm_core::{CellFlags, Rgb, Term, TermMode, Theme};
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
pub(super) struct VtTestSession {
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
    writer: Box<dyn Write + Send>,
    term: Term<PtyResponder>,
    proc: vte::ansi::Processor,
    cols: u16,
    rows: u16,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl VtTestSession {
    pub(super) fn new(cols: u16, rows: u16) -> Self {
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

        // Enable Mode 40 so DECCOLM (mode 3) resizes the grid to 80/132.
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
    pub(super) fn wait(&mut self, quiet_ms: u64) {
        loop {
            if self.drain_blocking(quiet_ms) == 0 {
                break;
            }
        }
    }

    /// Wait until the grid contains `needle`.
    pub(super) fn wait_for(&mut self, needle: &str, timeout_ms: u64) {
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
    pub(super) fn send(&mut self, key: &[u8]) {
        self.writer.write_all(key).expect("write key");
        self.writer.flush().expect("flush");
        self.wait(300);
    }

    /// Serialize the visible grid to text for content-based waiting.
    pub(super) fn grid_text(&self) -> String {
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

        let reverse_video = content.mode.contains(TermMode::REVERSE_VIDEO);

        // When DECSCNM is active, cell colors are already resolved against the
        // swapped palette in `renderable_content_into()`. The FramePalette
        // fg/bg must also be swapped so the clear color (screen background)
        // matches the swapped default background.
        let (frame_fg, frame_bg) = if reverse_video {
            (palette_bg, fg)
        } else {
            (fg, palette_bg)
        };
        let palette = FramePalette {
            background: frame_bg,
            foreground: frame_fg,
            cursor_color: Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            opacity: 1.0,
            selection_fg: None,
            selection_bg: None,
        };

        FrameInput {
            content,
            viewport: ViewportSize::new(w, h),
            cell_size: cell,
            content_cols: cols,
            content_rows: rows,
            palette,
            selection: None,
            search: None,
            hovered_cell: None,
            hovered_url_segments: Vec::new(),
            mark_cursor: None,
            window_focused: true,
            reverse_video,
            fg_dim: 1.0,
            text_blink_opacity: 1.0,
            subpixel_positioning: true,
            prompt_marker_rows: Vec::new(),
        }
    }

    /// Build a `FrameInput` with a custom `text_blink_opacity`.
    fn frame_input_with_blink(&self, cell: CellMetrics, text_blink_opacity: f32) -> FrameInput {
        let mut input = self.frame_input(cell);
        input.text_blink_opacity = text_blink_opacity;
        input
    }

    /// Render the current screen to pixels and compare against a golden ref.
    pub(super) fn assert_golden(
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

pub(super) fn vttest_available() -> bool {
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

/// Navigate vttest to the SGR blink screen (menu 2 screen 13) and verify
/// that rendering at different `text_blink_opacity` values produces
/// visibly different output for BLINK cells while non-BLINK cells stay
/// constant.
#[test]
fn vttest_blink_multi_frame() {
    if !vttest_available() {
        return;
    }

    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let mut s = VtTestSession::new(80, 24);
    s.wait_for("Enter choice number", 5000);
    s.send(b"2\r");

    // Advance to screen 13 (SGR test — dark background with blink text).
    for _ in 1..13 {
        s.send(b"\r");
    }

    // Verify we actually have BLINK cells in this screen.
    let content = s.term.renderable_content();
    let blink_count = content
        .cells
        .iter()
        .filter(|c| c.flags.contains(CellFlags::BLINK) && c.ch != ' ')
        .count();
    assert!(
        blink_count > 0,
        "screen 13 should contain cells with CellFlags::BLINK, found 0"
    );

    // Find a BLINK cell and a non-BLINK non-space cell for comparison.
    let blink_idx = content
        .cells
        .iter()
        .position(|c| c.flags.contains(CellFlags::BLINK) && c.ch != ' ')
        .expect("should have a BLINK cell");
    let normal_idx = content
        .cells
        .iter()
        .position(|c| !c.flags.contains(CellFlags::BLINK) && c.ch != ' ')
        .expect("should have a non-BLINK cell");

    let cell = renderer.cell_metrics();
    let cols = 80_usize;

    let blink_col = blink_idx % cols;
    let blink_row = blink_idx / cols;
    let normal_col = normal_idx % cols;
    let normal_row = normal_idx / cols;

    // Render 3 frames at opacity 1.0, 0.5, and 0.0.
    let opacities = [1.0_f32, 0.5, 0.0];
    let mut blink_brightness = Vec::new();
    let mut normal_brightness = Vec::new();

    for &opacity in &opacities {
        let input = s.frame_input_with_blink(cell, opacity);
        let w = input.viewport.width;
        let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);

        let b_br = cell_brightness(&pixels, w, blink_col, blink_row, cell.width, cell.height);
        let n_br = cell_brightness(&pixels, w, normal_col, normal_row, cell.width, cell.height);

        blink_brightness.push(b_br);
        normal_brightness.push(n_br);
    }

    // BLINK cell brightness must decrease: 1.0 > 0.5 > 0.0.
    assert!(
        blink_brightness[0] > blink_brightness[1],
        "BLINK cell should dim at 0.5: full={} half={}",
        blink_brightness[0],
        blink_brightness[1],
    );
    assert!(
        blink_brightness[1] > blink_brightness[2],
        "BLINK cell should dim at 0.0: half={} hidden={}",
        blink_brightness[1],
        blink_brightness[2],
    );

    // Non-BLINK cell brightness must stay constant (within tolerance).
    for i in 0..2 {
        let diff = (normal_brightness[i] as i32 - normal_brightness[i + 1] as i32).abs();
        assert!(
            diff < 5,
            "non-BLINK cell should be constant: frame{}={} frame{}={} diff={}",
            i,
            normal_brightness[i],
            i + 1,
            normal_brightness[i + 1],
            diff,
        );
    }
}

/// Compute the average RGB brightness across all pixels in a grid cell.
///
/// Sums R+G+B for every pixel within the cell bounds and divides by pixel
/// count. This avoids false readings from sampling glyph counters (holes
/// inside letters like 'b', 'e', 'o').
fn cell_brightness(pixels: &[u8], width: u32, col: usize, row: usize, cw: f32, ch: f32) -> u32 {
    let x0 = (col as f32 * cw) as u32;
    let y0 = (row as f32 * ch) as u32;
    let x1 = ((col + 1) as f32 * cw).ceil() as u32;
    let y1 = ((row + 1) as f32 * ch).ceil() as u32;

    let mut total: u64 = 0;
    let mut count: u64 = 0;
    for py in y0..y1 {
        for px in x0..x1 {
            let idx = ((py * width + px) * 4) as usize;
            total += pixels[idx] as u64 + pixels[idx + 1] as u64 + pixels[idx + 2] as u64;
            count += 1;
        }
    }
    if count == 0 {
        return 0;
    }
    (total / count) as u32
}
