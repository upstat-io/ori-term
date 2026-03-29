# Section 40: Vi Mode + Copy Mode -- Verification Results

**Verified:** 2026-03-29
**Status:** CONFIRMED NOT STARTED
**Reviewed:** false (unreviewed gate)

---

## 1. Code Search: Is Any Preliminary Code Present?

**No vi mode code exists.** Exhaustive search results:

- `vi.?mode|vi_mode|ViMode|copy.?mode|CopyMode` across all `*.rs` files: **zero matches**.
- `hjkl|yank|visual.?select` across all `*.rs` files: **zero matches**.
- No `oriterm/src/app/vi_mode.rs` or `oriterm/src/app/vi_mode/` directory exists (glob returned empty).

**Verdict:** Truly not started. No scaffold, no stubs, no dead code.

---

## 2. TODOs/FIXMEs Related to This Section's Domain

No TODOs or FIXMEs reference vi mode, copy mode, hjkl navigation, or yank operations anywhere in the codebase. The only TODOs in the entire codebase are:
- `oriterm/src/app/keyboard_input/overlay_dispatch.rs:246` -- "TODO: open About dialog" (unrelated)
- `oriterm/src/app/dialog_context/event_handling.rs:83` -- "TODO: re-rasterize UI fonts at new DPI" (unrelated)

---

## 3. Infrastructure That Partially Covers This Section

### 3a. Mark Mode (SIGNIFICANT OVERLAP)

**Location:** `oriterm/src/app/mark_mode/mod.rs` + `oriterm/src/app/mark_mode/motion.rs`

Mark mode is a Windows Terminal-style keyboard cursor navigation system that already implements the majority of the movement infrastructure vi mode needs:

**Already implemented (reusable):**
- `MarkCursor` struct (row/col, stable row index)
- Modal input interception (all keys routed to mark handler, not PTY)
- `Motion` enum with: Left, Right, Up, Down, PageUp, PageDown, LineStart, LineEnd, BufferStart, BufferEnd, WordLeft, WordRight
- Pure motion functions in `motion.rs`: `move_left`, `move_right`, `move_up`, `move_down`, `page_up`, `page_down`, `line_start`, `line_end`, `buffer_start`, `buffer_end`, `word_left`, `word_right`
- `GridBounds` and `AbsCursor` types for pure motion arithmetic
- `WordContext` with word boundary extraction
- `ensure_visible()` auto-scroll to keep cursor in viewport
- Selection integration via `extend_or_create_selection()`
- `select_all()` for Ctrl+A
- Enter to copy+exit, Escape to cancel+exit

**NOT implemented (vi mode needs):**
- `h`/`j`/`k`/`l` character key dispatch (mark mode uses arrow keys only)
- `w`/`b`/`e`/`W`/`B`/`E` word motions (mark mode has Ctrl+arrow word motion, but vi-style `w`=next-word-start, `b`=prev-word-start, `e`=end-of-word are different semantics)
- `^`/`$` first/last non-blank (mark mode has Home/End = column 0 / last column, not first/last non-blank)
- `H`/`M`/`L` viewport top/middle/bottom
- `gg`/`G` with multi-key sequence parsing
- `%` bracket matching
- `f`/`F`/`t`/`T` inline search
- `;`/`,` repeat inline search
- `*`/`#` word-under-cursor search
- `zz` center view
- `v`/`V`/Ctrl+V visual mode toggles (mark mode uses Shift+arrow, no visual mode state)
- `y`/`Y` yank commands
- `o` open URL under cursor
- `/`/`?` search entry from vi mode
- `n`/`N` search navigation
- Multi-key command parsing (e.g., `gg`, `f<char>`, `d<motion>`)

### 3b. Selection Model (oriterm_core)

**Location:** `oriterm_core/src/selection/`

Complete 3-point selection model with:
- `Selection` (anchor, pivot, end)
- `SelectionMode::Char`, `SelectionMode::Word`, `SelectionMode::Line`
- `SelectionPoint` with stable row index + column + side
- `word_boundaries()` with configurable delimiters
- `logical_line_start()` / `logical_line_end()`
- Text extraction from selection

Missing for vi mode: Block/rectangular selection mode (`Ctrl+V`). The `SelectionMode` enum currently has `Char`, `Word`, `Line` -- no `Block`/`Rect` variant.

### 3c. Search Infrastructure (oriterm_core)

**Location:** `oriterm_core/src/search/`

- `SearchState` with query, matches, focused index, case sensitivity, regex mode
- `SearchMatch` with stable row coordinates
- `find` module for match computation

This is directly reusable for vi mode's `/` and `?` search integration (Section 40.4).

### 3d. SnapshotGrid

**Location:** `oriterm/src/app/snapshot_grid/`

Provides grid querying interface without terminal lock:
- `stable_to_absolute()` / `absolute_to_stable()`
- `word_boundaries()`
- `total_rows()`, `cols()`, `lines()`
- `first_visible_absolute()`

---

## 4. Gap Analysis

### Plan Strengths
- Thorough vi keybinding coverage (character, word, line, vertical, bracket, inline search motions)
- Correct reference to Alacritty's vi mode implementation
- Proper integration with existing Selection model (Section 09)
- Search integration via `/` and `?` (Section 40.4)
- Configurable keybinding for enter/exit
- Distinct vi cursor rendering

### Plan Gaps and Issues

**G1: Mark Mode Overlap Not Addressed.**
The plan proposes `oriterm/src/app/vi_mode.rs` as a new file. It does not mention the existing mark mode at all. This is a significant planning gap. The question is: should vi mode replace mark mode, extend it, or coexist? The motion infrastructure in `mark_mode/motion.rs` is directly reusable. The plan should specify whether mark mode is deprecated, or whether vi mode is a superset that subsumes it.

**G2: Block/Rectangular Selection Missing from Selection Model.**
Section 40.3 specifies `Ctrl+V` block selection. The existing `SelectionMode` enum has `Char`, `Word`, `Line` but no `Block`/`Rect`. This either needs to be added in Section 09 (Selection) as a prerequisite, or Section 40 needs to own adding the block selection variant. The plan doesn't address this dependency.

**G3: Multi-Key Command Parser Not Specified.**
Vi mode requires multi-key sequences: `gg`, `f<char>`, `zz`, etc. The plan lists these motions but doesn't specify how the multi-key state machine works. This needs a pending-key buffer and timeout logic (e.g., after typing `g`, wait briefly for second `g` before treating as unknown).

**G4: Crate Placement.**
The plan says "Crate: `oriterm`" for everything. The pure motion functions (like those in `mark_mode/motion.rs`) could arguably belong in `oriterm_ui` since they're testable without GPU/platform. However, since vi mode needs terminal state (scrollback grid content), `oriterm` is the correct crate. This is fine.

**G5: Dependencies Listed as "Sections 08, 09, 11 complete" -- All Not Started.**
Section 08 (Keyboard Input), Section 09 (Selection & Clipboard), and Section 11 (Search) are all listed as "Not Started" in the roadmap index. Vi mode cannot be implemented until all three are complete. However, looking at the actual codebase, keyboard input dispatch, selection, and search are already substantially implemented in production code -- the roadmap sections are "not started" in terms of the roadmap's tracking, but the code exists. The plan should clarify whether it depends on the *roadmap sections* or the *actual infrastructure*.

**G6: No Count Prefix.**
Standard vi supports count prefixes (e.g., `5j` moves down 5 lines, `3w` moves 3 words). The plan doesn't mention count support at all. This is a feature gap that power users will notice.

---

## 5. Dependency Status

| Dependency | Roadmap Status | Actual Code Status |
|---|---|---|
| Section 08 (Keyboard Input) | Not Started | Keyboard dispatch exists, keybinding table exists, action system exists |
| Section 09 (Selection & Clipboard) | Not Started | Selection model complete, clipboard ops complete, mark mode complete |
| Section 11 (Search) | Not Started | SearchState, find module, search UI exist |

The code dependencies are largely met even though the roadmap sections show "Not Started" -- this is because the roadmap tracks formal completion milestones, not code presence.
