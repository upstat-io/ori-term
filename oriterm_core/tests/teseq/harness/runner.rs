//! TeseqHarness runner.
//!
//! Constructs `Term<RecordedListener>`, applies setup sequences, feeds
//! compiled scenario bytes, and produces a `ScenarioOutcome` for assertions.

use std::path::Path;

use oriterm_core::term::renderable::RenderableCell;
use oriterm_core::{Term, Theme};

use super::events::RecordedListener;
use super::loader::ScenarioSpec;
use super::reseq::compile_teseq;

/// Results from running a single teseq scenario.
pub struct ScenarioOutcome {
    /// Grid text (one line per viewport row, space-padded).
    pub grid_text: String,
    /// Grid characters as 2D array.
    pub grid_chars: Vec<Vec<char>>,
    /// Full cell data for attribute inspection.
    pub cells: Vec<RenderableCell>,
    /// Cursor column (0-based).
    pub cursor_col: usize,
    /// Cursor line (0-based, viewport-relative).
    pub cursor_line: usize,
    /// Captured events during the scenario feed.
    pub events: Vec<super::events::RecordedEvent>,
    /// Terminal column count at snapshot time.
    pub cols: usize,
    /// Terminal row count at snapshot time.
    pub rows: usize,
    /// Number of scrollback rows above the viewport.
    pub scrollback_len: usize,
}

/// Integration test harness for teseq-based escape sequence scenarios.
pub struct TeseqHarness {
    term: Term<RecordedListener>,
    proc: vte::ansi::Processor,
    listener: RecordedListener,
    spec: ScenarioSpec,
}

impl TeseqHarness {
    /// Create harness from a `.teseq` file path.
    ///
    /// Loads the optional TOML sidecar, constructs a `Term` with the
    /// specified config, and applies any `pre_feed` setup sequences.
    pub fn from_scenario(teseq_path: &Path) -> Self {
        let spec = ScenarioSpec::load(teseq_path);
        let listener = RecordedListener::new();
        let theme = match spec.terminal.theme.as_str() {
            "light" => Theme::Light,
            _ => Theme::default(),
        };
        let mut term = Term::new(
            spec.terminal.rows,
            spec.terminal.cols,
            spec.terminal.scrollback,
            theme,
            listener.clone(),
        );
        let mut proc = vte::ansi::Processor::new();

        // Apply pre_feed setup sequences.
        for seq in &spec.setup.pre_feed {
            let bytes = parse_escape_string(seq);
            proc.advance(&mut term, &bytes);
        }

        // Clear setup events — only scenario events matter.
        listener.clear();

        Self {
            term,
            proc,
            listener,
            spec,
        }
    }

    /// Feed the `.teseq` scenario through the terminal.
    pub fn run(&mut self, teseq_path: &Path) -> ScenarioOutcome {
        let bytes = compile_teseq(teseq_path).expect("reseq compilation failed");
        self.proc.advance(&mut self.term, &bytes);
        self.outcome()
    }

    /// Access the loaded scenario spec.
    pub fn spec(&self) -> &ScenarioSpec {
        &self.spec
    }

    /// Extract current terminal state as `ScenarioOutcome`.
    fn outcome(&self) -> ScenarioOutcome {
        let content = self.term.renderable_content();
        let grid_text = grid_text_from_content(&content);
        let grid_chars = grid_chars_from_content(&content);

        ScenarioOutcome {
            grid_text,
            grid_chars,
            cells: content.cells.clone(),
            cursor_col: content.cursor.column.0,
            cursor_line: content.cursor.line,
            events: self.listener.events(),
            cols: content.cols,
            rows: content.lines,
            scrollback_len: content.scrollback_len,
        }
    }
}

/// Extract grid text from renderable content.
///
/// Produces one line per viewport row with trailing spaces preserved,
/// separated by newlines. Same logic as `VtTestSession::grid_text()`.
pub fn grid_text_from_content(
    content: &oriterm_core::term::renderable::RenderableContent,
) -> String {
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

/// Extract grid characters as a 2D array from renderable content.
pub fn grid_chars_from_content(
    content: &oriterm_core::term::renderable::RenderableContent,
) -> Vec<Vec<char>> {
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

/// Parse an escape-notation string into raw bytes.
///
/// Handles: `\x##` hex escapes, `\e` for ESC (0x1B), `\n`, `\r`, `\t`,
/// `\\` for literal backslash, and plain ASCII characters.
///
/// Used for `pre_feed` strings in sidecar TOML files. TOML strings use
/// `\\` for literal backslash, so `\\x1b[?40h` in TOML yields the Rust
/// string `\x1b[?40h`, which this function converts to raw bytes.
pub fn parse_escape_string(s: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('x') | Some('X') => {
                    // Hex escape: \xNN
                    let hi = chars.next().expect("incomplete \\x escape");
                    let lo = chars.next().expect("incomplete \\x escape");
                    let hex_str: String = [hi, lo].iter().collect();
                    let byte = u8::from_str_radix(&hex_str, 16).expect("invalid hex in \\x escape");
                    bytes.push(byte);
                }
                Some('e') | Some('E') => bytes.push(0x1B),
                Some('n') => bytes.push(b'\n'),
                Some('r') => bytes.push(b'\r'),
                Some('t') => bytes.push(b'\t'),
                Some('\\') => bytes.push(b'\\'),
                Some(other) => {
                    // Unknown escape — pass through literally.
                    bytes.push(b'\\');
                    let mut buf = [0u8; 4];
                    bytes.extend_from_slice(other.encode_utf8(&mut buf).as_bytes());
                }
                None => bytes.push(b'\\'),
            }
        } else {
            let mut buf = [0u8; 4];
            bytes.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
        }
    }

    bytes
}
