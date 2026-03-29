---
section: 47
title: Semantic Prompt State Management
status: not-started
reviewed: false
last_verified: "2026-03-29"
tier: 5
goal: Cell-level and row-level semantic content tracking (OSC 133), prompt-aware resize, prompt iteration — the deep grid-level infrastructure that makes prompt navigation, smart selection, and command output extraction robust and fast
sections:
  - id: "47.1"
    title: Cell Semantic Content Type
    status: not-started
  - id: "47.2"
    title: Row Prompt Flags
    status: not-started
  - id: "47.3"
    title: Prompt Iterator
    status: not-started
  - id: "47.4"
    title: Semantic Content Highlighting
    status: not-started
  - id: "47.5"
    title: Prompt-Aware Resize
    status: not-started
  - id: "47.6"
    title: Screen Optimization Flag
    status: not-started
  - id: "47.7"
    title: Click to Move Cursor in Prompt
    status: not-started
  - id: "47.8"
    title: Upgrade Existing Consumers
    status: not-started
  - id: "47.9"
    title: Section Completion
    status: not-started
---

# Section 47: Semantic Prompt State Management

**Status:** Not Started
**Goal:** Rewrite the semantic prompt tracking from a simple marker list (`Vec<PromptMarker>`) to cell-level and row-level semantic content flags in the grid, matching the architecture Ghostty adopted in [ghostty-org/ghostty#10455](https://github.com/ghostty-org/ghostty/pull/10455). This makes prompt navigation, smart selection, command output extraction, and resize all more correct and robust.

**Crate:** `oriterm_core` (grid, cell, term handler) and `oriterm` (consumers)
**Dependencies:** Section 01 (Cell + Grid), Section 02 (Term + VTE), Section 20 (Shell Integration — current marker-based approach)

**Blocks:**
- Improves Section 40 (Vi Mode) — semantic motions (`[[`, `]]` to jump between prompts) become trivial with row flags
- Improves Section 41 (Hints) — command output region detection becomes exact
- Enables future features: per-command scrollback folding, command history extraction

> **Verification Notes (2026-03-29):** Confirmed not started -- no `SemanticContentType`, `RowPromptFlag`, or `semantic_range` types exist. Existing infrastructure to build upon: `PromptState` enum (None/PromptStart/CommandStart/OutputStart), `PromptMarker` struct, `prompt_markers: Vec<PromptMarker>`, `shell_state.rs` (353 lines of prompt navigation/marker management), OSC 133 interception in `oriterm_mux/src/shell_integration/interceptor.rs`, shell integration scripts for bash/zsh/fish/powershell, and `PendingMarks` bitflags for deferred marking.
>
> **CRITICAL BLOCKER: CellFlags is full (16/16 bits used).** All 16 bits of the `u16` CellFlags are occupied (bits 0-7: SGR attributes, bits 8-10: wide char/spacer/wrap, bits 11-14: underline variants, bit 15: leading wide char spacer). The plan says "use 2 bits from the existing bitflags" but there are none available. Options: (1) expand to `u32` (risks increasing Cell size beyond the 24-byte compile-time assertion), (2) use a separate `u8` field in Cell (also increases size), (3) use the 2-byte padding slot between `CellFlags(2)` and `Option<Arc>(8)` in the current Cell layout (`char(4) + Color(4) + Color(4) + CellFlags(2) + pad(2) + Option<Arc>(8)` = 24 bytes). Option 3 fits without increasing Cell size. The plan must explicitly address this constraint.
>
> Additional gaps: plan does not mention the `PendingMarks` bitflags mechanism (must be preserved or adapted), does not address VTE parsing of OSC 133 parameters (`k=s`, `k=c`, `redraw=0`), and does not note that scrollback eviction naturally handles flag cleanup (simplification over current `prune_prompt_markers()`).

**Reference:**
- Ghostty PR #10455: "Rewritten semantic prompt state management (OSC 133)" — merged
  - `terminal.Page`: 2-bit `SemanticContentType` per Cell, 2-bit prompt flag per Row
  - `terminal.PageList`: `promptIterator`, `highlightSemanticContent`
  - `terminal.Screen`: `has_semantic_prompts` fast-path flag
  - `terminal.Terminal`: `semanticPrompt` handler for all OSC 133 operations
  - Resize clears prompt lines when shell can redraw (Kitty `redraw=0` extension for shells that cannot)
- Ghostty issue #5932 (original tracking issue)
- [Semantic Prompts spec](https://gitlab.freedesktop.org/Per_Bothner/specifications/blob/master/proposals/semantic-prompts.md)

**Why this matters:** The current `Vec<PromptMarker>` approach tracks *positions* but not *content types*. It can't tell you which cells are prompt vs. input vs. output — only where transitions happened. This means smart selection must guess boundaries, resize can't clear prompt lines properly, and prompt scanning requires linear search through markers. Cell-level flags make every grid operation prompt-aware at zero marginal cost per cell (2 bits packed into existing flags).

---

## 47.1 Cell Semantic Content Type

Add a 2-bit semantic content type to each cell, tracking whether the cell contains prompt, user input, or command output.

**Files:** `oriterm_core/src/cell.rs`

**Reference:** Ghostty `terminal.Page` — `SemanticContentType` enum on Cell

- [ ] `SemanticContentType` enum (2-bit, repr(u8)):
  - [ ] `Output` = 0 (zero value / default — command output)
  - [ ] `Input` = 1 (user-typed input at prompt)
  - [ ] `Prompt` = 2 (prompt itself, e.g., `user@host $`)
- [ ] Pack into `CellFlags` — use 2 bits from the existing bitflags (or add 2 bits)
  - [ ] Zero-cost: `Output` is the zero value, so all existing cells default to output
  - [ ] No `CellExtra` needed — fits in flags
- [ ] `Cell::semantic_type(&self) -> SemanticContentType` getter
- [ ] `Cell::set_semantic_type(&mut self, ty: SemanticContentType)` setter
- [ ] When the terminal writes a cell (via `put_char` / handler), stamp the current semantic type from the terminal's prompt state machine
- [ ] `SGR 0` (reset) does NOT reset semantic type — it persists across attribute changes
- [ ] **Tests:**
  - [ ] Default cell has `Output` type
  - [ ] Setting and getting semantic type round-trips
  - [ ] `CellFlags` size unchanged (2 bits fit in existing padding or extend minimally)

---

## 47.2 Row Prompt Flags

Add a 2-bit prompt flag to each row, tracking whether the row contains a prompt start, prompt continuation, or no prompt. This enables O(1) prompt detection per row (skip rows with `no_prompt`) and preserves the distinction between primary and continuation prompts.

**Files:** `oriterm_core/src/grid.rs` (Row struct)

**Reference:** Ghostty `terminal.Page` — Row prompt flags

- [ ] `RowPromptFlag` enum (2-bit, repr(u8)):
  - [ ] `NoPrompt` = 0 (zero value — row has no prompt cells)
  - [ ] `Prompt` = 1 (row contains prompt cells and is the START of a prompt — triggered by OSC 133;A)
  - [ ] `PromptContinuation` = 2 (row contains prompt cells but is a continuation of a prior prompt — OSC 133;A with `k=s` or `k=c`)
- [ ] Store in `Row` struct (2-bit field, packed into existing flags or a new `u8` field)
- [ ] `Row::prompt_flag(&self) -> RowPromptFlag` getter
- [ ] `Row::set_prompt_flag(&mut self, flag: RowPromptFlag)` setter
- [ ] Set `Prompt` on the cursor row when OSC 133;A is processed (deferred, after both parsers finish)
- [ ] Set `PromptContinuation` when OSC 133;A arrives with `k=s` (secondary) or `k=c` (continuation)
- [ ] Row flags serve two purposes:
  - [ ] **Optimization**: prompt scanning skips rows with `NoPrompt`
  - [ ] **Data**: only place where primary vs. continuation prompt is recorded (cells only store `Prompt`)
- [ ] **Tests:**
  - [ ] Default row has `NoPrompt`
  - [ ] Setting `Prompt` flag on row, reading it back
  - [ ] Setting `PromptContinuation` flag on row, reading it back

---

## 47.3 Prompt Iterator

An iterator over grid rows that yields only rows marked as prompt starts. This replaces scanning the `Vec<PromptMarker>` — the iterator walks the grid's row flags directly.

**Files:** `oriterm_core/src/grid.rs` or `oriterm_core/src/grid/prompt_iter.rs`

**Reference:** Ghostty `terminal.PageList.promptIterator`

- [ ] `PromptIterator` struct:
  - [ ] Iterates over rows (visible + scrollback) in order
  - [ ] Yields `(absolute_row_index, &Row)` for rows where `prompt_flag == Prompt`
  - [ ] Skips `NoPrompt` and `PromptContinuation` rows
- [ ] Bidirectional: can iterate forward or backward from a given position
- [ ] `Grid::prompt_iter(&self) -> PromptIterator` — forward from top of scrollback
- [ ] `Grid::prompt_iter_from(&self, row: usize, direction: Direction) -> PromptIterator` — from a given row
- [ ] Performance: O(n) worst case over rows, but fast in practice since most rows are `NoPrompt` (check is a 2-bit comparison)
- [ ] **Tests:**
  - [ ] Grid with no prompts: iterator yields nothing
  - [ ] Grid with 3 prompts: iterator yields 3 rows in order
  - [ ] Backward iteration from bottom yields prompts in reverse order
  - [ ] `PromptContinuation` rows are NOT yielded (only `Prompt` starts)

---

## 47.4 Semantic Content Highlighting

Given a prompt row, extract the range of rows/cells belonging to a specific semantic content type (prompt, input, output). This enables "select command output" and "select command input" to be exact rather than heuristic.

**Files:** `oriterm_core/src/grid.rs` or `oriterm_core/src/grid/semantic_highlight.rs`

**Reference:** Ghostty `terminal.PageList.highlightSemanticContent`

- [ ] `Grid::semantic_range(&self, prompt_row: usize, content_type: SemanticContentType) -> Option<Range<Point>>`:
  - [ ] From a prompt start row, scan forward to find the extent of the requested content type
  - [ ] `Prompt` → all cells from OSC 133;A to OSC 133;B (prompt region)
  - [ ] `Input` → all cells from OSC 133;B to OSC 133;C (user input region)
  - [ ] `Output` → all cells from OSC 133;C to next OSC 133;A (command output)
- [ ] Returns a `Point` range (start row/col to end row/col) suitable for creating a `Selection`
- [ ] Handles multi-line prompts (prompt spans multiple rows)
- [ ] Handles missing transitions gracefully (e.g., no OSC 133;C → input extends to end of visible area)
- [ ] **Tests:**
  - [ ] Prompt region: returns rows from `A` to `B`
  - [ ] Input region: returns rows from `B` to `C`
  - [ ] Output region: returns rows from `C` to next `A`
  - [ ] Multi-line prompt: region spans multiple rows
  - [ ] Missing `C` marker: input extends to end

---

## 47.5 Prompt-Aware Resize

When the terminal resizes, clear prompt lines so the shell can redraw them. This prevents garbled prompt display after resize. Respect the Kitty `redraw=0` extension for shells that cannot redraw.

**Files:** `oriterm_core/src/grid.rs` (resize logic), `oriterm_core/src/term_handler.rs`

**Reference:** Ghostty `terminal.Screen` — "clearing the prompt lines is now built-in to resize"

- [ ] On resize: if the grid has semantic prompts, find the current prompt region (from last OSC 133;A to cursor) and clear those rows
  - [ ] Clear means: erase cell contents, reset to default attributes, but preserve row structure
  - [ ] The shell will re-emit the prompt after receiving SIGWINCH
- [ ] **Kitty `redraw=0` extension**:
  - [ ] If OSC 133;A was received with `redraw=0`, the shell CANNOT redraw the prompt
  - [ ] In this case, do NOT clear prompt lines on resize — preserve them as-is
  - [ ] Track `can_redraw_prompt: bool` per prompt state (default: true)
- [ ] Only clear the CURRENT prompt (the one at/near the cursor), not historical prompts in scrollback
- [ ] **Tests:**
  - [ ] Resize with active prompt: prompt rows are cleared
  - [ ] Resize with `redraw=0` prompt: prompt rows are preserved
  - [ ] Resize with no semantic prompts: no clearing (no regression)
  - [ ] Historical prompts in scrollback: not cleared by resize

---

## 47.6 Screen Optimization Flag

A boolean flag on the screen/terminal that tracks whether ANY row has ever had a semantic prompt. This provides a fast path to skip all prompt-related work when shell integration is not active.

**Files:** `oriterm_core/src/term_handler.rs` or `oriterm_core/src/grid.rs`

**Reference:** Ghostty `terminal.Screen` — "flag that tracks whether we've seen any row trigger a semantic prompt"

- [ ] `has_semantic_prompts: bool` on `Grid` or `Term`
  - [ ] Default: `false`
  - [ ] Set to `true` when ANY OSC 133;A is processed
  - [ ] Never reset to `false` (once seen, always assume possible)
- [ ] Use as fast-path guard:
  - [ ] `prompt_iter()` returns empty immediately if `!has_semantic_prompts`
  - [ ] Resize prompt clearing skipped if `!has_semantic_prompts`
  - [ ] Prompt navigation actions are no-ops if `!has_semantic_prompts`
- [ ] **Tests:**
  - [ ] Fresh terminal: `has_semantic_prompts == false`
  - [ ] After OSC 133;A: `has_semantic_prompts == true`
  - [ ] Prompt iterator returns empty when flag is false

---

## 47.7 Click to Move Cursor in Prompt

When the user clicks within the current prompt's input region, reposition the shell cursor to that position by synthesizing arrow key sequences. This is a quality-of-life feature enabled by cell-level semantic content tracking — we know exactly which cells are `Input` and can safely send cursor movement keys.

**Files:** `oriterm/src/app/mouse_input.rs`, `oriterm_core/src/grid.rs`

**Reference:** Ghostty (enabled by semantic prompt state), iTerm2 (click-to-move in prompt), WezTerm `SemanticZone`

- [ ] On left-click within the current prompt's `Input` region:
  - [ ] Determine click column and current cursor column
  - [ ] Compute delta (positive = right, negative = left)
  - [ ] Synthesize arrow key sequences: `\x1b[C` (right) × N or `\x1b[D` (left) × N
  - [ ] Write synthesized keys to PTY
- [ ] **Guard conditions** (all must be true):
  - [ ] `has_semantic_prompts == true`
  - [ ] Click is on a row within the current prompt's `Input` region (between OSC 133;B and OSC 133;C)
  - [ ] Terminal is in the `CommandStart` prompt state (user is typing at the prompt)
  - [ ] Mouse reporting is NOT enabled (application mode overrides this)
  - [ ] Not in alt screen (alt screen apps handle their own cursor)
- [ ] Handle wide characters correctly: a CJK character at the target position may need 2 arrow keys to traverse
- [ ] Config: `behavior.click_to_move_cursor = true | false` (default: true)
- [ ] **Tests:**
  - [ ] Click 5 columns right of cursor: 5 right-arrow sequences sent
  - [ ] Click 3 columns left of cursor: 3 left-arrow sequences sent
  - [ ] Click outside input region: no arrow keys sent (normal click behavior)
  - [ ] Click while mouse reporting enabled: no arrow keys sent
  - [ ] Click in alt screen: no arrow keys sent

---

## 47.8 Upgrade Existing Consumers

Migrate all existing prompt consumers from `Vec<PromptMarker>` to the new cell/row-based system. Remove the old marker list.

**Files:** `oriterm/src/app/prompt_nav.rs`, `oriterm/src/shell_integration.rs`, `oriterm_core/src/term_handler.rs`

- [ ] **Prompt navigation** (Section 20.12):
  - [ ] `PreviousPrompt` / `NextPrompt`: use `Grid::prompt_iter_from()` instead of scanning `prompt_markers`
  - [ ] Should be simpler and more robust (no marker pruning needed)
- [ ] **Select command output** (Section 20.12):
  - [ ] Use `Grid::semantic_range(row, SemanticContentType::Output)` instead of manual marker math
- [ ] **Select command input** (Section 20.12):
  - [ ] Use `Grid::semantic_range(row, SemanticContentType::Input)`
- [ ] **Command completion notifications** (Section 20.13):
  - [ ] Continue using prompt state machine transitions (OSC 133;C/D timing) — no change needed
- [ ] **Remove `prompt_markers: Vec<PromptMarker>`**:
  - [ ] Delete the old marker list and all pruning/maintenance code
  - [ ] The row flags are the single source of truth now
- [ ] **Stamp cells during write**:
  - [ ] In the VTE handler's `input()` / `put_char()` path: set `Cell::semantic_type` based on current `PromptState`
  - [ ] `PromptState::PromptStart` → `SemanticContentType::Prompt`
  - [ ] `PromptState::CommandStart` → `SemanticContentType::Input`
  - [ ] `PromptState::OutputStart` → `SemanticContentType::Output`
  - [ ] `PromptState::None` → `SemanticContentType::Output` (default)
- [ ] **Click-to-move integration** (Section 47.7):
  - [ ] Wire mouse click handler to check semantic content type before synthesizing arrow keys
- [ ] **Tests:**
  - [ ] Prompt navigation produces same results as before
  - [ ] Select command output produces same results as before
  - [ ] `prompt_markers` field is gone (compile-time verification)

---

## 47.9 Section Completion

- [ ] All 47.1–47.8 items complete
- [ ] Cell semantic content type stored in `CellFlags` (2 bits, zero-cost default)
- [ ] Row prompt flags stored per row (2 bits)
- [ ] Prompt iterator scans row flags directly (no separate marker list)
- [ ] `semantic_range()` extracts exact prompt/input/output regions
- [ ] Resize clears active prompt lines (respects `redraw=0`)
- [ ] `has_semantic_prompts` flag gates all prompt work when shell integration is absent
- [ ] Old `Vec<PromptMarker>` removed — row/cell flags are the single source of truth
- [ ] All existing prompt navigation and selection features work identically
- [ ] `cargo build --target x86_64-pc-windows-gnu` — clean build
- [ ] `cargo clippy --target x86_64-pc-windows-gnu` — no warnings
- [ ] `cargo test` — all tests pass

**Exit Criteria:** Semantic prompt state is tracked at the cell and row level in the grid, making all prompt operations (navigation, selection, resize) exact and efficient. The old marker-based approach is fully replaced. Shell integration users see more correct prompt behavior especially around resize. Non-shell-integration users see zero overhead.
