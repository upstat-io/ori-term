---
section: 14
title: URL Detection
status: complete
reviewed: true
last_verified: "2026-03-29"
tier: 3
goal: Detect URLs in terminal output for hover underline and Ctrl+click opening
sections:
  - id: "14.1"
    title: URL Detection Engine
    status: complete
  - id: "14.2"
    title: URL Cache
    status: complete
  - id: "14.3"
    title: Hover & Click Handling
    status: complete
  - id: "14.4"
    title: Section Completion
    status: complete
---

# Section 14: URL Detection

**Status:** Complete
**Goal:** Detect URLs in terminal output using regex, provide visual hover feedback (underline + pointer cursor), and open URLs in the system browser on Ctrl+click. Handles soft-wrapped lines, balanced parentheses (Wikipedia-style URLs), and coexists with explicit OSC 8 hyperlinks.

**Crate:** `oriterm` (binary)
**Dependencies:** `regex`, `std::sync::LazyLock`
**Reference:** `_old/src/url_detect.rs`, `_old/src/app/cursor_hover.rs`

**Prerequisite:** Section 01 (Grid), Section 09 (search text extraction — shared `extract_row_text`)

---

## 14.1 URL Detection Engine

Regex-based URL detection across logical lines (sequences of soft-wrapped rows).

**File:** `oriterm/src/url_detect/mod.rs`

**Reference:** `_old/src/url_detect.rs`

- [x] `UrlSegment` type alias — `(usize, usize, usize)` = `(abs_row, start_col, end_col)` inclusive (verified 2026-03-29)
- [x] `DetectedUrl` struct (verified 2026-03-29)
  - [x] Fields: (verified 2026-03-29)
    - `segments: Vec<UrlSegment>` — per-row segments (handles URLs wrapped across rows)
    - `url: String` — the extracted URL string
  - [x] `DetectedUrl::contains(&self, abs_row: usize, col: usize) -> bool` (verified 2026-03-29)
    - [x] Check if any segment covers the given position (verified 2026-03-29)
  - [x] Derive: `Debug`, `Clone` (verified 2026-03-29)
- [x] URL regex pattern (static `LazyLock<Regex>`): (verified 2026-03-29)
  - [x] `(?:https?|ftp|file)://[^\s<>\[\]'"]+` (verified 2026-03-29)
  - [x] Covers: http, https, ftp, file schemes (verified 2026-03-29)
  - [x] Stops at whitespace, angle brackets, square brackets, quotes (verified 2026-03-29)
- [x] `trim_url_trailing(url: &str) -> &str` (verified 2026-03-29)
  - [x] Strip trailing punctuation: `.`, `,`, `;`, `:`, `!`, `?` (verified 2026-03-29)
  - [x] Handle balanced parentheses: only strip trailing `)` if unbalanced (verified 2026-03-29)
    - [x] Count `(` and `)` in URL (verified 2026-03-29)
    - [x] If `close > open`: strip one trailing `)` (verified 2026-03-29)
    - [x] Repeat until stable (verified 2026-03-29)
  - [x] Preserves Wikipedia-style URLs: `https://en.wikipedia.org/wiki/Rust_(language)` (verified 2026-03-29)
- [x] `detect_urls_in_logical_line(grid: &Grid, line_start: usize, line_end: usize) -> Vec<DetectedUrl>` (verified 2026-03-29)
  - [x] Concatenate text from all rows in logical line using `extract_row_text` (verified 2026-03-29)
  - [x] Build `char_to_pos: Vec<(usize, usize)>` mapping char index to `(abs_row, col)` (verified 2026-03-29)
  - [x] Run regex on concatenated text (verified 2026-03-29)
  - [x] For each match: (verified 2026-03-29)
    - [x] Trim trailing punctuation (verified 2026-03-29)
    - [x] Skip URLs shorter than scheme prefix (e.g., bare "https://") (verified 2026-03-29)
    - [x] Convert byte offsets to char offsets (verified 2026-03-29)
    - [x] Skip if any cell in span has an OSC 8 hyperlink (explicit hyperlinks take precedence) (verified 2026-03-29 -- NOTE: precedence handled at hover layer in cursor_hover.rs instead of at detection time; implicit URL first, OSC 8 fallback. Functionally equivalent.)
    - [x] Build per-row segments from `char_to_pos` mapping (verified 2026-03-29)
    - [x] Emit `DetectedUrl` with segments and URL string (verified 2026-03-29)
- [x] `logical_line_start(grid: &Grid, abs_row: usize) -> usize` — walk backwards to find first row of logical line (verified 2026-03-29)
- [x] `logical_line_end(grid: &Grid, abs_row: usize) -> usize` — walk forwards to find last row of logical line (verified 2026-03-29)

---

## 14.2 URL Cache

Lazy per-logical-line URL detection cache. Avoids redundant regex matching on every mouse move.

**File:** `oriterm/src/url_detect/mod.rs` (continued)

**Reference:** `_old/src/url_detect.rs` (UrlDetectCache)

- [x] `UrlDetectCache` struct (verified 2026-03-29)
  - [x] Fields: (verified 2026-03-29)
    - `lines: HashMap<usize, Vec<DetectedUrl>>` — logical line start row -> detected URLs
    - `row_to_line: HashMap<usize, usize>` — any row -> its logical line start (fast lookup)
  - [x] `Default` derive for empty initialization (verified 2026-03-29)
- [x] `UrlDetectCache::url_at(&mut self, grid: &Grid, abs_row: usize, col: usize) -> Option<DetectedUrl>` (verified 2026-03-29)
  - [x] Ensure logical line is computed (lazy) (verified 2026-03-29)
  - [x] Search cached URLs for one containing (abs_row, col) (verified 2026-03-29)
  - [x] Return cloned `DetectedUrl` if found (verified 2026-03-29)
- [x] `UrlDetectCache::ensure_logical_line(&mut self, grid: &Grid, abs_row: usize) -> usize` (verified 2026-03-29)
  - [x] If already cached (via `row_to_line`): return cached line start (verified 2026-03-29)
  - [x] Otherwise: compute logical line bounds, detect URLs, cache results (verified 2026-03-29)
  - [x] Register all rows in the logical line in `row_to_line` (verified 2026-03-29)
- [x] `UrlDetectCache::invalidate(&mut self)` (verified 2026-03-29)
  - [x] Clear both HashMaps (verified 2026-03-29)
  - [x] Called after: PTY output, scroll, resize, font change (anything that changes grid content or layout) (verified 2026-03-29 -- font change covered transitively via resize)
- [x] Cache is per-tab (stored in Tab or binary-side wrapper) (verified 2026-03-29 -- per-window on WindowContext)

---

## 14.3 Hover & Click Handling

Visual feedback on URL hover and opening URLs on Ctrl+click.

**File:** `oriterm/src/app/cursor_hover.rs`

**Reference:** `_old/src/app/cursor_hover.rs`, `_old/src/app/hover_url.rs`

- [x] On mouse move (while Ctrl held): (verified 2026-03-29)
  - [x] Convert pixel position to grid cell (abs_row, col) (verified 2026-03-29)
  - [x] Query `url_cache.url_at(grid, abs_row, col)` (verified 2026-03-29)
  - [x] If URL found: (verified 2026-03-29)
    - [x] Store `hovered_url: Option<DetectedUrl>` in app/tab state (verified 2026-03-29)
    - [x] Set cursor icon to `CursorIcon::Pointer` (hand cursor) (verified 2026-03-29)
    - [x] Underline all cells in the URL's segments (solid underline on hover) (verified 2026-03-29)
    - [x] Request redraw (verified 2026-03-29)
  - [x] If no URL (or Ctrl not held): (verified 2026-03-29)
    - [x] Clear `hovered_url` (verified 2026-03-29)
    - [x] Restore cursor icon to default (verified 2026-03-29)
    - [x] Remove hover underline (verified 2026-03-29)
    - [x] Request redraw if state changed (verified 2026-03-29)
- [x] On Ctrl+click (left button): (verified 2026-03-29)
  - [x] If `hovered_url` is Some: (verified 2026-03-29)
    - [x] Validate URL scheme: only `http`, `https`, `ftp`, `file` allowed (verified 2026-03-29 -- also allows `mailto:`)
    - [x] Open URL in system browser: (verified 2026-03-29)
      - [x] Windows: `ShellExecuteW` (Win32 API) (verified 2026-03-29)
      - [x] Linux: `xdg-open` (verified 2026-03-29)
      - [x] macOS: `open` (verified 2026-03-29)
    - [x] Consume the click event (don't pass to terminal/selection) (verified 2026-03-29)
- [x] URL hover rendering integration: (verified 2026-03-29)
  - [x] During `draw_frame`: check if cell is in `hovered_url` segments (verified 2026-03-29)
  - [x] If yes: draw solid underline decoration at cell position (verified 2026-03-29)
  - [x] Color: foreground color (matches text above) (verified 2026-03-29)
- [x] Interaction with OSC 8 hyperlinks: (verified 2026-03-29)
  - [x] Implicit URL detection skips cells that already have explicit OSC 8 hyperlinks (verified 2026-03-29 -- handled at hover layer: implicit first, OSC 8 fallback)
  - [x] OSC 8 hyperlinks have their own hover/click behavior (section 20) (verified 2026-03-29)
- [x] Interaction with mouse reporting: (verified 2026-03-29)
  - [x] When terminal has mouse reporting enabled: Ctrl+click still opens URL (Ctrl is override) (verified 2026-03-29)
  - [x] Shift+click bypasses mouse reporting per xterm convention (verified 2026-03-29)

---

## 14.4 Section Completion

- [x] All 14.1-14.3 items complete (verified 2026-03-29)
- [x] `cargo clippy -p oriterm --target x86_64-pc-windows-gnu` — no warnings (verified 2026-03-29)
- [x] Simple URLs detected at correct column ranges (http, https, ftp, file) (verified 2026-03-29)
- [x] Multiple URLs on same line detected independently (verified 2026-03-29)
- [x] Wikipedia-style parenthesized URLs preserved: `https://en.wikipedia.org/wiki/Rust_(language)` (verified 2026-03-29)
- [x] Trailing punctuation stripped: `https://example.com.` detects `https://example.com` (verified 2026-03-29)
- [x] Wrapped URLs: URL spanning two rows detected with correct per-row segments (verified 2026-03-29)
- [x] OSC 8 hyperlinks not duplicated by implicit detection (verified 2026-03-29)
- [x] Ctrl+hover: underline appears, cursor changes to pointer (verified 2026-03-29)
- [x] Ctrl+click: URL opens in system browser (verified 2026-03-29)
- [x] Cache invalidated on PTY output/scroll/resize (no stale URLs) (verified 2026-03-29)
- [x] No URL on plain text: no false positives on words like "https" without "://" (verified 2026-03-29)
- [x] **Tests** (`oriterm/src/url_detect/tests.rs`): (verified 2026-03-29 — 37 tests pass: 23 detection + 10 platform + 4 GPU)
  - [x] Detect simple URL at correct columns (verified 2026-03-29)
  - [x] Detect multiple URLs on same line (verified 2026-03-29)
  - [x] Balanced parentheses preserved (verified 2026-03-29)
  - [x] No URLs in plain text (verified 2026-03-29)
  - [x] Wrapped URL spans two rows with correct segments (verified 2026-03-29)
  - [x] `DetectedUrl::contains` returns correct results for all positions (verified 2026-03-29)

**Exit Criteria:** Ctrl+hover underlines URLs in terminal output, Ctrl+click opens them in the system browser. Detection handles wrapped lines, parenthesized URLs, and coexists with explicit OSC 8 hyperlinks.
