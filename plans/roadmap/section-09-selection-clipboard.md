---
section: 9
title: Selection & Clipboard
status: in-progress
reviewed: true
tier: 3
goal: Windows Terminal-style 3-point selection, all selection modes, clipboard with paste filtering
sections:
  - id: "9.1"
    title: Selection Model & Anchoring
    status: complete
  - id: "9.2"
    title: Mouse Selection
    status: complete
  - id: "9.3"
    title: Keyboard Selection (Mark Mode)
    status: complete
  - id: "9.4"
    title: Word Delimiters & Boundaries
    status: complete
  - id: "9.5"
    title: Copy Operations
    status: in-progress
  - id: "9.6"
    title: Paste Operations
    status: complete
  - id: "9.7"
    title: Selection Rendering
    status: complete
  - id: "9.8"
    title: Section Completion
    status: in-progress
---

# Section 09: Selection & Clipboard

**Status:** In Progress
**Goal:** Implement text selection and clipboard modeled after Windows Terminal, which has the best selection/clipboard UX of any terminal emulator. 3-point selection with char/word/line/block modes, smart copy with formatting, paste filtering, and bracketed paste.

**Crate:** `oriterm_core` (selection model, boundaries, text extraction), `oriterm_mux` (pane-level selection ownership, invalidation), `oriterm` (mouse/keyboard integration, clipboard I/O, rendering)
**Dependencies:** `clipboard-win` (Windows clipboard), `oriterm_core` (Grid, Cell, CellFlags)
**Reference:** `_old/src/selection/`, `_old/src/app/mouse_selection.rs`, `_old/src/clipboard.rs`

**Modeled after:** Windows Terminal's selection and clipboard implementation. Key source files: `Selection.cpp`, `Clipboard.cpp`, `ControlInteractivity.cpp`, `textBuffer/TextBuffer.cpp`.

**Prerequisite:** Section 01 complete (Grid, Cell, Row data structures). Section 06 complete (keyboard input dispatch for keybinding wiring).

---

## 9.1 Selection Model & Anchoring

Windows Terminal uses a 3-point selection model: anchor, pivot, and endpoint. The pivot prevents losing the initially selected unit (word or line) during drag.

**Files:** `oriterm_core/src/selection/mod.rs`, `oriterm_core/src/selection/boundaries.rs`, `oriterm_core/src/selection/text.rs`, `oriterm_core/src/selection/html/mod.rs`, `oriterm_core/src/selection/click/mod.rs`

**Reference:** `_old/src/selection/mod.rs` — carries forward the proven 3-point model with `SelectionPoint`, `Selection`, `SelectionMode`.

- [x] `Side` enum — `Left`, `Right` (defined in `oriterm_core::index`, re-exported from `oriterm_core::lib`)
  - [x] Sub-cell precision for selection boundaries (which half of the cell was clicked)
  - [x] Derive: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`
- [x] `SelectionPoint` struct
  - [x] Fields:
    - `row: StableRowIndex` — row identity that survives scrollback eviction
    - `col: usize` — column index
    - `side: Side` — which half of the cell
  - [x] `effective_start_col(&self) -> usize` — when `side == Right`, selection starts at `col + 1`
  - [x] `effective_end_col(&self) -> usize` — when `side == Left && col > 0`, selection ends at `col - 1`
  - [x] `impl Ord` — compare by row, then col, then side (Left < Right)
  - [x] `impl PartialOrd` — delegate to `Ord`
  - [x] Derive: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`
- [x] `SelectionMode` enum
  - [x] `Char` — character-by-character (single click + drag)
  - [x] `Word` — word selection (double-click, subsequent drag expands by words)
  - [x] `Line` — full logical line selection (triple-click, follows WRAP)
  - [x] `Block` — rectangular block selection (Alt+click+drag)
  - [x] Derive: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`
- [x] `Selection` struct
  - [x] Fields:
    - `mode: SelectionMode`
    - `anchor: SelectionPoint` — initial click position (fixed)
    - `pivot: SelectionPoint` — other end of initial unit (word end, line end); prevents losing selected word during drag
    - `end: SelectionPoint` — current drag endpoint (moves with mouse)
  - [x] `Selection::new_char(row: StableRowIndex, col: usize, side: Side) -> Self` — anchor = pivot = end
  - [x] `Selection::new_word(anchor: SelectionPoint, pivot: SelectionPoint) -> Self` — anchor/pivot set to word boundaries
  - [x] `Selection::new_line(anchor: SelectionPoint, pivot: SelectionPoint) -> Self` — anchor/pivot set to line boundaries
  - [x] `ordered(&self) -> (SelectionPoint, SelectionPoint)` — normalize: sort anchor, pivot, end and return (min, max)
  - [x] `contains(&self, stable_row: StableRowIndex, col: usize) -> bool` — test if cell is within selection
    - [x] Block mode: rectangular bounds (min_col..max_col within row range)
    - [x] Other modes: use effective_start_col/effective_end_col at boundary rows, full rows in between
  - [x] `bounds(&self) -> SelectionBounds` — precompute bounds for batch containment testing (avoids recomputing `ordered()` per cell during rendering)
  - [x] `is_empty(&self) -> bool` — true if Char mode and anchor == end (zero area)
- [x] `SelectionBounds` struct — precomputed start/end with mode, used by `FrameSelection` for per-cell testing during rendering. `contains(stable_row, col) -> bool` method handles Block vs linear modes.
- [x] Selection across scrollback: points use `StableRowIndex` (absolute row positions that survive scrollback eviction)
- [x] Selection invalidation: clear on output that affects selected region
  - [x] `selection_dirty` flag on `Term<T>` set by content-modifying VTE handler operations
  - [x] `Pane::check_selection_invalidation()` (in `oriterm_mux/src/pane/selection.rs`) checks flag and clears selection on terminal wakeup
  - [x] Selection ownership is split across two layers:
    - `Pane` owns `selection: Option<Selection>` in the mux layer, used for terminal-output-driven invalidation via `check_selection_invalidation()`
    - `App::pane_selections: HashMap<PaneId, Selection>` is authoritative for rendering and GUI operations (set/clear/update_end via `pane_accessors.rs`)
    - The GUI side (`App`) is the source of truth for frame building; the mux side (`Pane`) handles invalidation
- [x] Multi-click detection:
  - [x] Track last click position and timestamp
  - [x] Use 500ms window for multi-click detection
  - [x] Click counter cycles: 1 -> 2 -> 3 -> 1 (single -> double -> triple -> reset)
  - [x] Clicks must be in same cell position to count as multi-click
  - [x] `ClickDetector` struct in `oriterm_core::selection::click` with `click()`, `click_at()`, and `reset()` (tests in `oriterm_core/src/selection/click/tests.rs`)
- [x] Re-export `Selection`, `SelectionPoint`, `SelectionMode`, `SelectionBounds`, `Side`, `ClickDetector`, `DEFAULT_WORD_DELIMITERS`, `logical_line_start`, `logical_line_end` from `oriterm_core/src/lib.rs`. Note: `SelectionColors` is re-exported from `color`, not `selection` (`pub use color::{Palette, Rgb, SelectionColors}`).
- [x] **Tests** (`oriterm_core/src/selection/tests.rs`):
  - [x] `new_char` creates selection with anchor == pivot == end
  - [x] `new_word` creates selection with distinct anchor and pivot
  - [x] `ordered()` returns min/max regardless of anchor/end order
  - [x] `contains()` returns true for cells inside selection, false outside
  - [x] `contains()` respects Side precision at boundary cells
  - [x] Block mode `contains()` uses rectangular bounds
  - [x] `is_empty()` returns true for zero-area Char selection
  - [x] SelectionPoint ordering: row takes priority, then col, then side

---

## 9.2 Mouse Selection

Windows Terminal-style mouse selection with drag threshold, multi-click modes, and auto-scroll.

**Files:** `oriterm/src/app/mouse_selection/mod.rs`, `oriterm/src/app/mouse_selection/helpers.rs`, `oriterm/src/app/mouse_input.rs`

**Reference:** `_old/src/app/mouse_selection.rs` — carries forward click counting, word/line selection creation, drag endpoint updates.

- [x] **Click count detection** (via `ClickDetector` from `oriterm_core::selection::click`, see 9.1):
  - [x] Same position + within 500ms: increment count (1 -> 2 -> 3 -> 1)
  - [x] Different position or expired window: reset to 1
- [x] **Drag threshold**: Selection only starts after cursor moves >= 1/4 cell width from initial click position
  - [x] Track touchdown position separately from selection anchor
  - [x] Only initiate selection once threshold exceeded (prevents accidental selection)
- [x] **Single click + drag** — Character selection:
  - [x] Convert pixel position to cell coordinates (account for display_offset, tab bar offset)
  - [x] Determine Side (Left/Right) from pixel sub-cell position
  - [x] Clear any existing selection
  - [x] Set anchor at click position with `Selection::new_char()`
  - [x] Drag extends endpoint via `update_selection_end()`
- [x] **Double-click** — Word selection:
  - [x] Compute word boundaries around click position (see 9.4)
  - [x] Create selection with `Selection::new_word(start_boundary, end_boundary)`
  - [x] Pivot set to expanded word boundaries
  - [x] Subsequent drag expands by words: compare drag position to anchor, snap to word boundary via `helpers::compute_drag_endpoint()`
- [x] **Triple-click** — Line selection:
  - [x] Select entire logical line (follows wrapped lines via WRAP flag)
  - [x] Walk backwards through `logical_line_start()` to find first row of logical line
  - [x] Walk forwards through `logical_line_end()` to find last row
  - [x] Start at (first_row, col 0, Side::Left), end at (last_row, last_col, Side::Right)
  - [x] Create selection with `Selection::new_line()`
- [x] **Alt+click+drag** — Toggle block/character mode:
  - [x] If current mode is Char or Line: switch to `SelectionMode::Block`
  - [x] If current mode is Block: switch to `SelectionMode::Char`
- [x] **Shift+click** — Extend existing selection:
  - [x] If selection exists: update endpoint to clicked position
  - [x] If click is beyond anchor: include clicked cell
  - [x] If click is before anchor: start from clicked position
  - [x] Respect double-wide character boundaries
- [x] **Ctrl+click** — Open hyperlink URL: <!-- blocked-by:14 -->
  - [x] Check OSC 8 hyperlink on clicked cell (takes priority)
  - [x] Fall through to implicit URL detection
  - [x] If URL found: open in default browser, consume click
- [x] **Auto-scroll during drag** (mouse above/below viewport):
  - [x] When dragging above grid top: scroll viewport up into history (1 line per event)
  - [x] When dragging below grid bottom: scroll viewport down toward live (if display_offset > 0)
  - [x] Continue extending selection into scrollback during auto-scroll
  - [x] Post-scroll endpoint computation via `helpers::compute_auto_scroll_endpoint()`: constructs a fresh `SnapshotGrid` from the post-scroll snapshot and computes the endpoint at the visible edge row
- [x] **SnapshotGrid helpers** (in `oriterm/src/app/snapshot_grid/mod.rs`):
  - [x] `viewport_to_stable_row()` — maps viewport line to `StableRowIndex`
  - [x] `redirect_spacer()` — redirects clicks on WIDE_CHAR_SPACER to base cell
  - [x] `word_boundaries()` — delegates to `oriterm_core::selection::word_boundaries` via snapshot cells
  - [x] `logical_line_start()`/`logical_line_end()` — walk WRAP flag within viewport bounds
- [x] **Double-wide character handling**:
  - [x] Selection never splits a double-wide character
  - [x] If click lands on WIDE_CHAR_SPACER: redirect to base cell (col - 1)
  - [x] Automatically adjust selection endpoint to cell boundary
- [x] **Tests** (`oriterm/src/app/mouse_selection/tests.rs`):
  - [x] Click count detection: rapid clicks cycle 1 -> 2 -> 3 -> 1 (in `oriterm_core::selection::click::tests`)
  - [x] Click at different position resets to 1
  - [x] Expired click window resets to 1
  - [x] Double-click creates Word selection with correct boundaries
  - [x] Triple-click creates Line selection spanning wrapped lines
  - [x] Alt+click toggles block mode
  - [x] Shift+click extends existing selection

---

## 9.3 Keyboard Selection (Mark Mode)

Keyboard-driven selection for accessibility and power users, modeled after Windows Terminal's mark mode.

**Files:** `oriterm/src/app/mark_mode/mod.rs`, `oriterm/src/app/mark_mode/motion.rs` (mark mode logic and cursor motion), `oriterm/src/app/keyboard_input/action_dispatch.rs` (mark mode entry/exit via keybinding)

- [x] **Enter mark mode**: Ctrl+Shift+M
  - [x] Insert `MarkCursor` into `App::mark_cursors: HashMap<PaneId, MarkCursor>` for the active pane
  - [x] Show visual cursor at current terminal cursor position
  - [x] Arrow keys move selection cursor (not terminal cursor, not sent to PTY)
- [x] **Shift+Arrow keys** — Extend selection by one cell:
  - [x] Shift+Left/Right: extend by one column
  - [x] Shift+Up/Down: extend by one row
- [x] **Ctrl+Shift+Arrow keys** — Extend selection by word:
  - [x] Ctrl+Shift+Left: extend to previous word boundary
  - [x] Ctrl+Shift+Right: extend to next word boundary
- [x] **Shift+Page Up/Down** — Extend by one screen:
  - [x] Selection extends by `grid.lines` rows
- [x] **Shift+Home/End** — Extend to line boundaries:
  - [x] Shift+Home: extend to start of current line (column 0)
  - [x] Shift+End: extend to end of current line (last non-empty column)
- [x] **Ctrl+Shift+Home/End** — Extend to buffer boundaries:
  - [x] Ctrl+Shift+Home: extend to top of scrollback
  - [x] Ctrl+Shift+End: extend to bottom of buffer
- [x] **Ctrl+A** — Select all:
  - [x] In mark mode: `select_all(grid)` selects entire buffer (visible + scrollback)
  - [x] Outside mark mode: `Action::SelectAll` -> `select_all_in_pane()` tries shell input zone first (OSC 133), falls back to entire buffer
- [x] **Escape** — Cancel selection:
  - [x] Clear selection
  - [x] Exit mark mode
- [x] **Enter** — Copy and exit:
  - [x] Copy current selection to clipboard
  - [x] Exit mark mode
- [x] **Viewport scroll-follow** (`ensure_visible()`): when the mark cursor moves outside the visible viewport, computes a scroll delta so the caller can scroll the viewport to keep the cursor visible
- [x] **Pure motion functions** (in `mark_mode/motion.rs`): all motion functions (`move_left`, `move_right`, `move_up`, `move_down`, `page_up`, `page_down`, `line_start`, `line_end`, `buffer_start`, `buffer_end`, `word_left`, `word_right`) are pure — no locks, no grid access, no side effects. Grid bounds and word context are extracted under lock before calling.
- [x] **`MarkModeResult` return type**: dispatch returns `MarkModeResult { action, new_cursor, new_selection }` so the caller (App) applies state mutations. Decouples mark mode logic from App state.
- [x] **Tests** (`oriterm/src/app/mark_mode/tests.rs`):
  - [x] Enter mark mode sets flag, exit clears it
  - [x] Shift+Right extends selection by one column
  - [x] Ctrl+A selects entire buffer
  - [x] Escape clears selection and exits mark mode

---

## 9.4 Word Delimiters & Boundaries

Configurable word boundary detection for double-click selection and Ctrl+arrow word movement.

**File:** `oriterm_core/src/selection/boundaries.rs`

**Reference:** `_old/src/selection/boundaries.rs` — carries forward the delimiter_class + scan approach.

- [x] **Default word delimiters**: `` ,│`|:\"' ()[]{}<>\t `` (defined in `DEFAULT_WORD_DELIMITERS`)
- [x] **Character classification** (`fn delimiter_class(c: char, word_delimiters: &str) -> u8`):
  - [x] Class 0: Word characters (anything NOT in `word_delimiters` and not whitespace/null)
  - [x] Class 1: Whitespace (space, `\0`, tab)
  - [x] Class 2: Non-whitespace delimiters (characters in `word_delimiters` that aren't whitespace)
  - [x] Separating whitespace (class 1) from delimiters (class 2) allows word navigation to treat them differently (e.g., stop at punctuation but skip whitespace)
- [x] `is_word_delimiter` — test-only helper in `tests.rs`, not a public function (`delimiter_class(c, DEFAULT_WORD_DELIMITERS) != 0`)
- [x] `word_boundaries(grid: &Grid, abs_row: usize, col: usize, word_delimiters: &str) -> (usize, usize)`
  - [x] Returns (start_col, end_col) inclusive
  - [x] If clicked on WIDE_CHAR_SPACER: redirect to base cell (col - 1)
  - [x] Classify the clicked character
  - [x] Scan left: move while `delimiter_class(cell.ch, word_delimiters) == click_class`, skipping WIDE_CHAR_SPACER
  - [x] Scan right: move while `delimiter_class(cell.ch, word_delimiters) == click_class`, including WIDE_CHAR_SPACER that follows a wide char
  - [x] Returns (start, end) of contiguous same-class region
- [x] `logical_line_start(grid: &Grid, abs_row: usize) -> usize`
  - [x] Walk backwards through rows connected by WRAP flag
  - [x] Returns absolute row index of first row in logical line
- [x] `logical_line_end(grid: &Grid, abs_row: usize) -> usize`
  - [x] Walk forwards through rows connected by WRAP flag
  - [x] Returns absolute row index of last row in logical line
- [x] Configurable delimiters via settings (future: wired through config in Section 13)
- [x] **Tests** (`oriterm_core/src/selection/tests.rs`):
  - [x] `delimiter_class('a', DEFAULT_WORD_DELIMITERS)` returns 0 (word)
  - [x] `delimiter_class(' ', DEFAULT_WORD_DELIMITERS)` returns 1 (whitespace)
  - [x] `delimiter_class('(', DEFAULT_WORD_DELIMITERS)` returns 2 (non-whitespace delimiter)
  - [x] `word_boundaries` on "hello world" at col 2 returns (0, 4)
  - [x] `word_boundaries` on "hello world" at col 5 returns (5, 5) (space is its own unit)
  - [x] `word_boundaries` on wide char spacer redirects to base cell
  - [x] `logical_line_start` walks back through WRAP rows
  - [x] `logical_line_end` walks forward through WRAP rows

---

## 9.5 Copy Operations <!-- unblocks:8.3 -->

Windows Terminal copies multiple clipboard formats simultaneously. Smart copy behavior adapts to context.

**Files:** `oriterm/src/app/clipboard_ops/mod.rs` (clipboard I/O), `oriterm_core/src/selection/text.rs` (text extraction), `oriterm_core/src/selection/html/mod.rs` (HTML extraction)

**Reference:** `_old/src/selection/text.rs` — carries forward text extraction with wrap handling, spacer skipping, grapheme cluster support.

- [x] **Copy triggers**:
  - [x] Ctrl+Shift+C — copy selection
  - [x] Ctrl+C — smart: copy if selection exists, send SIGINT (`\x03`) if not
  - [x] Ctrl+Insert — copy selection
  - [x] Enter — copy selection (in mark mode, then exit mark mode)
  - [x] CopyOnSelect setting: auto-copy on mouse release after selection (does NOT clear selection)
  - [x] Right-click: copy if selection exists (when context menu disabled)
- [x] **Text extraction** (`extract_text(grid: &Grid, selection: &Selection) -> String`):
  - [x] Convert StableRowIndex to absolute row for iteration
  - [x] Walk selected cells, concatenate characters
  - [x] Skip WIDE_CHAR_SPACER cells (include the wide char cell, not its spacer)
  - [x] Skip LEADING_WIDE_CHAR_SPACER cells
  - [x] Replace `\0` (null) with space
  - [x] Append zero-width characters (combining marks) from `cell.extra.zerowidth` (via `CellExtra`)
  - [x] Handle wrapped lines: rows connected by WRAP flag join without newline
  - [x] Unwrapped lines: trim trailing spaces, add newline between rows
  - [x] Block selection: add newlines between rows, trim trailing spaces per row, use min_col..max_col bounds
  - [x] Handle grapheme clusters: base char + all zerowidth chars from CellExtra
  - [x] Skip Kitty image placeholder characters (`KITTY_PLACEHOLDER`) — virtual cells for image rendering should not appear in copied text
- [x] **CopyOnSelect copies to primary selection, not clipboard**: `copy_selection_to_primary()` stores to `ClipboardType::Selection` (X11/Wayland primary selection). On Windows/macOS the clipboard module silently ignores `Selection` stores.
- [x] **Clipboard formats** (placed on clipboard simultaneously):
  - [x] Plain text (always; `CF_UNICODETEXT` on Windows, UTF-8 string on macOS/Linux)
  - [x] `HTML Format` — HTML with inline styles (if CopyFormatting enabled)
    - [x] Per-cell foreground/background colors as inline CSS
    - [x] Font name and size (embedded in `<pre>` style attribute)
    - [x] Bold rendering for BOLD cells (`font-weight:bold`)
    - [x] Italic rendering for ITALIC cells (`font-style:italic`)
    - [x] Underline styles (single, double, curly/wavy, dotted, dashed via `text-decoration`)
    - [x] Strikethrough rendering (`text-decoration:line-through`)
    - [x] Dim/faint rendering (`opacity:0.5`)
    - [ ] Underline color (`text-decoration-color`) <!-- blocked-by:38 — requires colored underline support from Section 38 -->
    - [x] HIDDEN cells (SGR 8) skipped in HTML output
    - [x] HTML entity escaping (`&`, `<`, `>`, `"`, `'`) via `push_html_escaped()`
    - [x] INVERSE (SGR 7) cells: fg/bg swapped before style resolution
    - [x] Style coalescing: adjacent cells with identical formatting share a single `<span>`
  - [x] `extract_html_with_text()` — single-pass dual extraction returns `(html, text)`, avoiding double iteration over selected cells
  - [x] ~~`Rich Text Format`~~ — skipped: no reference terminal implements RTF (WezTerm/Ghostty use HTML)
- [x] **Copy modifiers**:
  - [x] Shift held during copy: collapse multi-line selection to single line (join with spaces)
  - [x] Alt held during copy: force HTML formatting regardless of CopyFormatting setting
- [x] Selection NOT cleared after copy (user must press Escape or click elsewhere)
- [x] **OSC 52 clipboard integration**:
  - [x] Application can set clipboard via `ESC]52;c;{base64_data}ST`
  - [x] Application can request clipboard (if permitted by config)
- [x] **Tests** (`oriterm_core/src/selection/tests.rs`, `oriterm_core/src/selection/html/tests.rs`):
  - [x] Extract text from single row: correct characters
  - [x] Extract text skips WIDE_CHAR_SPACER
  - [x] Extract text includes zero-width chars (combining marks)
  - [x] Wrapped lines joined without newline
  - [x] Unwrapped lines separated by newline
  - [x] Trailing spaces trimmed per row
  - [x] Block selection extracts rectangular region
  - [x] Null chars replaced with spaces

---

## 9.6 Paste Operations

Windows Terminal-style paste with character filtering, line ending normalization, and bracketed paste support.

**Files:** `oriterm/src/app/clipboard_ops/mod.rs` (paste dispatch), `oriterm_core/src/paste/mod.rs` (paste processing pipeline)

**Reference:** `_old/src/clipboard.rs`

- [x] **Paste triggers**:
  - [x] Ctrl+Shift+V — paste from clipboard
  - [x] Ctrl+V — paste (when no VT conflict)
  - [x] Shift+Insert — paste
  - [x] Right-click — paste (when no selection and context menu disabled)
  - [x] Middle-click — paste from primary selection (`paste_from_primary()` loads `ClipboardType::Selection`; on Windows/macOS primary selection is typically empty, making this a no-op)
- [x] **Character filtering on paste** (configurable `FilterOnPaste` setting):
  | Character | Behavior |
  |-----------|----------|
  | Tab (`\t`) | Strip (prevents tab expansion issues) |
  | Non-breaking space (U+00A0, U+202F) | Convert to regular space |
  | Smart quotes (U+201C, U+201D) | Convert to straight double quotes (`"`) |
  | Smart single quotes (U+2018, U+2019) | Convert to straight single quotes (`'`) |
  | Em-dash (U+2014) | Convert to double hyphen (`--`) |
  | En-dash (U+2013) | Convert to hyphen (`-`) |
- [x] **Line ending handling** (via `normalize_line_endings()`):
  - [x] Convert Windows CRLF (`\r\n`) to CR (`\r`) for terminal
  - [x] Convert standalone LF (`\n`) to CR (`\r`) — terminals expect CR for newline input
  - [x] Standalone CR passes through unchanged
- [x] **Bracketed paste** (XTERM DECSET 2004):
  - [x] Check TermMode::BRACKETED_PASTE flag on active pane
  - [x] When enabled: wrap paste in `\x1b[200~` ... `\x1b[201~`
  - [x] Allows applications to differentiate pasted text from typed text
  - [x] Strip ESC (`\x1b`) characters from pasted content within brackets (via `strip_escape_chars()`)
- [x] **Paste processing pipeline** (`prepare_paste(text, bracketed, filter) -> Vec<u8>`):
  - [x] Step 1: character filtering (if `filter` is true)
  - [x] Step 2: line ending normalization (CRLF/LF to CR)
  - [x] Step 3: ESC stripping (if `bracketed` is true)
  - [x] Step 4: bracketed paste wrapping (if `bracketed` is true)
  - [x] Returns raw bytes ready for PTY write
- [x] **Injection defense**: bracketed paste strips the ESC character from `\x1b[201~` end markers, preventing paste-escape injection attacks. Tested explicitly.
- [x] **Multi-line paste warning** (configurable):
  - [x] Detect newlines in pasted content
  - [x] Optionally warn user before sending multi-line paste to shell
  - [x] Configurable: always warn, never warn, warn if > N lines
- [x] **File drag-and-drop paste**:
  - [x] Handle `WindowEvent::DroppedFile` events
  - [x] Extract file path(s)
  - [x] Auto-quote paths containing spaces: `"C:\path with spaces\file.txt"`
  - [x] Write path(s) to PTY as if typed
  - [x] Multiple files: space-separated
- [x] **Tests** (`oriterm_core/src/paste/tests.rs`):
  - [x] FilterOnPaste strips tabs
  - [x] FilterOnPaste converts smart quotes to straight quotes
  - [x] FilterOnPaste converts em-dash to double hyphen
  - [x] CRLF converted to CR
  - [x] Bracketed paste wraps content in ESC[200~ / ESC[201~
  - [x] ESC chars stripped within bracketed paste
  - [x] Bracketed paste neutralizes `\x1b[201~` end marker injection
  - [x] Plain mode preserves ESC sequences (only bracketed mode strips)
  - [x] OSC/CSI injection defense in bracketed paste (title set, SGR sequences stripped)
  - [x] File path with spaces gets quoted
  - [x] Multiple file paths are space-separated
  - [x] Empty path list produces empty string
  - [x] `collapse_lines()` tests in `clipboard_ops/tests.rs`: single line unchanged, multi-line joined with spaces, CRLF handling, preserves internal spaces

---

## 9.7 Selection Rendering

Visual highlighting of selected text during GPU rendering.

**Files:** `oriterm/src/gpu/prepare/mod.rs` (selection color resolution during cell processing), `oriterm/src/gpu/frame_input/mod.rs` (`FrameSelection` struct, selection bounds for rendering), `oriterm/src/gpu/extract/from_snapshot/mod.rs` (selection extraction into frame input)

**Reference:** `_old/src/gpu/render_grid.rs` (selection check in cell loop)

- [x] **Selection colors**: configurable selection foreground and background
  - [x] Default: inverted colors (swap fg/bg of selected cells)
  - [x] Alternative: user-configured selection_fg / selection_bg from palette
  - [x] Colors stored in palette semantic slots
- [x] **Render approach** (during cell rendering loop): <!-- unblocks:5.13 --><!-- unblocks:6.5 --><!-- unblocks:6.16 -->
  - [x] For each visible cell: check `selection.contains(stable_row, col)`
  - [x] If selected: override fg/bg with selection colors
  - [x] Convert viewport row to StableRowIndex for comparison
  - [x] Selection check must be efficient (called per-cell per-frame)
- [x] **Double-wide character handling**:
  - [x] If WIDE_CHAR cell is selected: highlight both the wide char cell and its spacer
  - [x] If only the spacer col is in selection bounds: still highlight both cells
  - [x] Never render half of a double-wide character as selected
- [x] **Selection across wrapped lines**:
  - [x] Highlight continues seamlessly across wrap boundaries
  - [x] No gap between wrapped rows in the selection highlight
- [x] **Block selection rendering**:
  - [x] Only highlight cells within rectangular bounds (min_col..max_col, min_row..max_row)
  - [x] Rows between start and end use same column bounds
- [x] **Include selection in FrameInput** (via `FrameSelection`):
  - [x] Pass current selection (if any) to the render function via `FrameInput::selection: Option<FrameSelection>`
  - [x] `FrameSelection` precomputes `SelectionBounds` and stores `base_stable` for viewport-to-stable mapping
  - [x] `FrameSelection::contains(viewport_line, col) -> bool` — per-cell test with stable row conversion
  - [x] `FrameSelection::viewport_line_range(num_rows) -> Option<(start, end)>` — clamped viewport line range for damage tracking
- [x] **Selection damage tracking** (incremental redraw on selection change):
  - [x] `mark_selection_damage(dirty, old, new)` in `gpu/prepare/dirty_skip/mod.rs` marks symmetric difference lines dirty
  - [x] New selection: damage all newly-selected lines
  - [x] Clear selection: damage all previously-selected lines
  - [x] Extend selection: damage newly covered lines + boundary lines
  - [x] Shrink selection: damage uncovered lines + boundary lines
  - [x] Same selection: no damage (early return)
  - [x] `prev_selection_range` on `PreparedFrame` persists across frames for tracking
  - [x] Selection range clamped to viewport bounds (never indexes out of bounds)
  - [x] Tests: `new_selection_damages_selected_lines`, `clear_selection_damages_previously_selected_lines`, `extend_selection_damages_new_lines_and_boundary`, `shrink_selection_damages_uncovered_lines`, `same_selection_no_damage`, `selection_damage_integrated_with_build_dirty_set`, `selection_damage_clamped_to_viewport`
- [x] **Edge case handling**:
  - [x] Block cursor exclusion: skip selection inversion at cursor position
  - [x] INVERSE (SGR 7) cells: use palette defaults instead of double-swap
  - [x] fg==bg fallback: prevent invisible text by using palette defaults
  - [x] HIDDEN (SGR 8) guard: intentionally hidden text stays invisible under selection
  - [x] Non-block cursors (underline, beam): do not block selection inversion
- [x] **Tests** (`oriterm/src/gpu/prepare/tests.rs`, `oriterm/src/gpu/frame_input/tests.rs`):
  - [x] Selection highlight inverts colors for selected cells
  - [x] Wide character selected as complete unit
  - [x] Block selection renders rectangular highlight
  - [x] Selection across wrapped lines has no visual gap
  - [x] Block cursor at selected cell skips inversion
  - [x] INVERSE flag cell uses palette defaults when selected
  - [x] fg==bg cell falls back to palette defaults when selected
  - [x] HIDDEN cell stays invisible when selected
  - [x] Underline cursor does not block selection inversion
  - [x] Explicit `selection_fg`/`selection_bg` colors override fg/bg swap when both are set

---

## 9.8 Section Completion

- [ ] All 9.1-9.7 items complete *(blocked: 9.5 has one item pending Section 38 — underline color in HTML copy)*
- [x] `cargo test -p oriterm_core --target x86_64-pc-windows-gnu` — selection model tests pass
- [x] `cargo test -p oriterm --target x86_64-pc-windows-gnu` — clipboard + mouse selection tests pass
- [x] `cargo clippy --workspace --target x86_64-pc-windows-gnu` — no warnings
- [x] Single click + drag selects text character-by-character
- [x] Drag threshold prevents accidental selection on slight mouse movement
- [x] Double-click selects words (configurable delimiters)
- [x] Triple-click selects full logical lines (follows wraps)
- [x] Alt+drag does block/rectangular selection
- [x] Shift+click extends existing selection
- [x] Keyboard selection with Shift+arrows, Ctrl+Shift+arrows
- [x] Ctrl+A selects all
- [x] Ctrl+Shift+C copies selection
- [x] Ctrl+C smart behavior (copy if selection, SIGINT if not)
- [x] CopyOnSelect option (auto-copy on mouse release)
- [x] Ctrl+Shift+V pastes from clipboard
- [x] Bracketed paste mode wraps pasted text in ESC[200~ / ESC[201~
- [x] FilterOnPaste strips/converts special characters
- [x] File drag-and-drop auto-quotes paths with spaces
- [x] Selection visually highlighted with configurable colors
- [x] Wide characters selected as complete units
- [x] Soft-wrapped lines joined correctly in copied text
- [x] Selection across scrollback works (StableRowIndex survives eviction)
- [x] OSC 52 clipboard integration works
- [x] Middle-click pastes from primary selection (X11/Wayland)
- [x] Multi-line paste warning dialog with configurable threshold
- [x] Selection damage tracking: incremental redraw on selection create/extend/clear
- [x] HTML copy with styled spans (colors, bold, italic, underline, strikethrough)
- [x] Mark mode: Ctrl+Shift+M toggles, arrow keys move cursor, Shift+arrows extend selection

**Exit Criteria:** Selection and clipboard works identically to Windows Terminal. Users coming from Windows Terminal should feel completely at home with the selection, copy, and paste behavior.
