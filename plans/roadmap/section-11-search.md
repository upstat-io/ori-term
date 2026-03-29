---
section: 11
title: Search
status: complete
reviewed: true
last_verified: "2026-03-29"
tier: 3
goal: Plain text and regex search across terminal grid with search UI overlay and match navigation
sections:
  - id: "11.1"
    title: Search State
    status: complete
  - id: "11.2"
    title: Search Algorithm
    status: complete
  - id: "11.3"
    title: Row Text Extraction
    status: complete
  - id: "11.4"
    title: Search UI
    status: complete
  - id: "11.5"
    title: Section Completion
    status: complete
---

# Section 11: Search

**Status:** Complete
**Goal:** Plain text and regex search across the terminal grid (viewport + scrollback) with a search bar overlay, match highlighting, and keyboard-driven navigation.

**Crate:** `oriterm_core` (search state, algorithm, text extraction), `oriterm` (search UI overlay)
**Dependencies:** `regex` (in `oriterm_core`)
**Reference:** `_old/src/search/` (mod.rs, find.rs, text.rs, tests.rs), `_old/src/app/search_ui.rs`

**Prerequisite:** Section 01 (Grid), Section 02 (VTE/Term — for StableRowIndex)

---

## 11.1 Search State (verified 2026-03-29)

Core search state: query, matches, focused index, navigation. Lives in `oriterm_core` so the library owns search logic independent of rendering.

**File:** `oriterm_core/src/search/mod.rs` (202 lines)

**Reference:** `_old/src/search/mod.rs`

- [x] `MatchType` enum — `None`, `Match`, `FocusedMatch` (verified 2026-03-29)
  - [x] Derive: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`
  - [x] Used by rendering to decide per-cell highlight color
- [x] `SearchMatch` struct — a single match span in stable grid coordinates (verified 2026-03-29)
  - [x] Fields:
    - `start_row: StableRowIndex` — stable row of match start
    - `start_col: usize` — column of match start
    - `end_row: StableRowIndex` — stable row of match end (same row for single-line matches)
    - `end_col: usize` — column of match end (inclusive)
  - [x] Derive: `Debug`, `Clone`
- [x] `SearchState` struct (verified 2026-03-29)
  - [x] Fields:
    - `query: String` — current search query text
    - `matches: Vec<SearchMatch>` — all matches, sorted by position (earliest first)
    - `focused: usize` — index of currently focused match
    - `case_sensitive: bool` — case sensitivity toggle
    - `use_regex: bool` — regex mode toggle
  - [x] `SearchState::new() -> Self` — empty defaults
  - [x] `SearchState::next_match(&mut self)` — advance to next match, wrapping around
  - [x] `SearchState::prev_match(&mut self)` — go to previous match, wrapping around
  - [x] `SearchState::update_query(&mut self, grid: &Grid)` — re-run search with current query settings
    - [x] Empty query: clear matches, reset focused to 0
    - [x] Non-empty: call `find::find_matches()`, clamp focused index
  - [x] `SearchState::focused_match(&self) -> Option<&SearchMatch>` — currently focused match
  - [x] `SearchState::cell_match_type(&self, stable_row: StableRowIndex, col: usize) -> MatchType` (verified 2026-03-29)
    - [x] Binary search via `partition_point` for O(log n) lookup
    - [x] Check small window around found index (handles edge cases)
    - [x] Returns `FocusedMatch` for the focused match, `Match` for others, `None` outside matches
- [x] `cell_in_match(m: &SearchMatch, stable_row: StableRowIndex, col: usize) -> bool` (verified 2026-03-29)
  - [x] Private helper: checks if (stable_row, col) falls within a match span
  - [x] Handles single-row matches, multi-row matches, and middle rows
- [x] Re-export `SearchState`, `SearchMatch`, `MatchType` from `lib.rs` (verified 2026-03-29 -- also re-exports `extract_row_text`)

---

## 11.2 Search Algorithm (verified 2026-03-29)

Find all matches in the grid for a given query, supporting both plain text and regex.

**File:** `oriterm_core/src/search/find.rs` (137 lines)

**Reference:** `_old/src/search/find.rs`

- [x] `find_matches(grid: &Grid, query: &str, case_sensitive: bool, use_regex: bool) -> Vec<SearchMatch>` (verified 2026-03-29)
  - [x] Returns matches sorted by position (earliest first)
- [x] Regex path: (verified 2026-03-29)
  - [x] Build `regex::RegexBuilder` with case_insensitive flag
  - [x] Invalid regex: return empty vec (no crash)
  - [x] Search row by row (multi-row regex deferred)
  - [x] For each row: extract text, get `StableRowIndex`, run `find_iter`, map byte spans to columns
- [x] Plain text path: (verified 2026-03-29)
  - [x] Case-insensitive: lowercase both query and haystack
  - [x] Sliding search: `haystack[start..].find(query)`, advance past each match
  - [x] Map byte spans to column spans via `byte_span_to_cols`
- [x] Iteration scope: all rows = `scrollback.len() + grid.lines` (full history + viewport)
- [x] Each match: `SearchMatch { start_row, start_col, end_row, end_col }`
- [x] **Tests** (`oriterm_core/src/search/tests.rs`): (verified 2026-03-29 -- 7 algorithm tests, all pass)
  - [x] Plain text: "hello" found at correct columns in two rows
  - [x] Case insensitive: "Hello", "HELLO", "hello" all found
  - [x] Case sensitive: only exact case match found
  - [x] Regex `\d+`: digits found at correct positions
  - [x] Invalid regex: empty result, no panic
  - [x] Empty query: empty result

---

## 11.3 Row Text Extraction (verified 2026-03-29)

Extract text from grid rows for search and URL detection, mapping between byte positions and column indices.

**File:** `oriterm_core/src/search/text.rs` (97 lines)

**Reference:** `_old/src/search/text.rs`

- [x] `extract_row_text(row: &Row) -> (String, Vec<usize>)` (verified 2026-03-29)
  - [x] Iterate cells, skip `WIDE_CHAR_SPACER` and `LEADING_WIDE_CHAR_SPACER` cells
  - [x] Replace `'\0'` with `' '` (null cells render as space)
  - [x] Push base char to text, record column in col_map
  - [x] Append zero-width characters (combining marks) from cell
  - [x] Returns: extracted text + column map (char index -> grid column)
- [x] `byte_span_to_cols(text: &str, col_map: &[usize], byte_start: usize, byte_end: usize) -> Option<(usize, usize)>` (verified 2026-03-29)
  - [x] Convert byte span in extracted text to `(start_col, end_col)` inclusive
  - [x] Returns `None` if span is empty or indices out of range
  - [x] Uses `char_index_at_byte` and `char_index_containing_byte` helpers
- [x] `char_index_at_byte(text: &str, byte_offset: usize) -> usize` — private
  - [x] First character starting at or after byte_offset
- [x] `char_index_containing_byte(text: &str, byte_offset: usize) -> usize` — private
  - [x] Character whose encoding contains byte_offset
- [x] `pub` visibility for `extract_row_text` — shared with URL detection, re-exported at lib.rs level (verified 2026-03-29 -- plan said `pub(crate)` but actual is `pub` for lib.rs re-export; intentional)
- [x] **Tests**: (verified 2026-03-29 -- 7 text extraction tests, all pass)
  - [x] ASCII row: text matches, col_map is identity
  - [x] Wide char row: spacer cells skipped, col_map jumps by 2
  - [x] Null cells: replaced with spaces
  - [x] Combining marks share base char column
  - [x] Byte span to cols: correct mapping for multi-byte UTF-8
  - [x] Empty byte span returns None

---

## 11.4 Search UI (verified 2026-03-29)

Search bar overlay rendered on top of the terminal grid, with text input, match count, and keyboard navigation.

**File:** `oriterm/src/app/search_ui.rs` (183 lines)

**Reference:** `_old/src/app/search_ui.rs`

- [x] `App::open_search(&mut self)` (verified 2026-03-29)
  - [x] Activate search for the active tab
  - [x] Request redraw
- [x] `App::close_search(&mut self)` (verified 2026-03-29)
  - [x] Close search for the active tab, clear search state
  - [x] Request redraw
- [x] `App::handle_search_key(&mut self, event: &KeyEvent)` (verified 2026-03-29)
  - [x] `Escape` — close search
  - [x] `Enter` — next match (Shift+Enter = prev match)
  - [x] `Backspace` — delete last character from query
  - [x] `Character(c)` — append to query, re-run search
  - [x] Each change calls `scroll_to_search_match` then requests redraw
- [x] `App::scroll_to_search_match(&self)` (verified 2026-03-29)
  - [x] Convert focused match's StableRowIndex to absolute row
  - [x] If match outside current viewport: scroll display_offset to center it
- [x] Search bar rendering: <!-- unblocks:6.13 -->
  - [x] Position: top-right of grid area (configurable in future)
  - [x] Content: query text, match count ("N of M"), up/down navigation indicators
  - [x] Rendered as GPU instances on foreground layer
  - [x] Match highlight: all match cells get distinct background color
  - [x] Focused match: different (brighter) background from non-focused matches
- [x] Key flow: `Ctrl+Shift+F` -> open, type -> update+highlight, `Enter`/`Shift+Enter` -> next/prev, `Escape` -> close+clear (verified 2026-03-29 -- keybinding confirmed in defaults.rs; macOS uses Cmd+F)
- [x] Search state is per-pane (stored on Pane in oriterm_mux, not per-tab) (verified 2026-03-29 -- plan said "per-tab" but actual is per-pane, which is more correct)

---

## 11.5 Section Completion (verified 2026-03-29 -- 34 tests across 3 crates, all pass)

- [x] All 11.1-11.4 items complete (verified 2026-03-29)
- [x] `cargo test -p oriterm_core` — search tests pass (verified 2026-03-29 -- 24 tests)
- [x] `cargo test -p oriterm` — search UI tests pass (verified 2026-03-29 -- 6 tests)
- [x] `cargo test -p oriterm_mux` — contract + e2e search tests pass (verified 2026-03-29 -- 4 tests)
- [x] `cargo clippy -p oriterm_core --target x86_64-pc-windows-gnu` — no warnings (verified 2026-03-29)
- [x] `cargo clippy -p oriterm --target x86_64-pc-windows-gnu` — no warnings (verified 2026-03-29)
- [x] Search finds plain text across viewport and scrollback
- [x] Regex search works (including edge cases like `\d+`, `[a-z]+`)
- [x] Invalid regex handled gracefully (no crash, empty results)
- [x] Match cycling wraps around (next from last -> first, prev from first -> last)
- [x] `cell_match_type` is O(log n) via binary search
- [x] Search UI opens/closes cleanly, keyboard input captured during search
- [x] Viewport scrolls to center focused match when outside view
- [x] Wide characters and combining marks handled correctly in text extraction (verified 2026-03-29)
- [ ] **Hygiene: decorative banners in tests.rs** — `oriterm_core/src/search/tests.rs` uses 9 `// ── ... ──` decorative banner comments (lines 13, 42, 91, 118, 153, 181, 227, 269, 299). Code-hygiene.md bans these; should be plain `// Section name` labels. (found 2026-03-29)

**Exit Criteria:** Ctrl+Shift+F opens search, typing highlights matches in the grid, Enter/Shift+Enter navigates between matches, Escape closes. Plain text and regex modes both functional.

**Note:** All functional items complete. One minor code hygiene finding (decorative banners in test file) remains. Search bar visual overlay renders as a `StatusBadge` in the top-right of the grid area, showing query text and "N of M" match count.
