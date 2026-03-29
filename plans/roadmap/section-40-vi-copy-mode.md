---
section: 40
title: Vi Mode + Copy Mode
status: not-started
reviewed: false
last_verified: "2026-03-29"
tier: 3
goal: "Keyboard-driven terminal navigation and text selection via vi-style motions, modeled after Alacritty's vi mode and WezTerm's copy mode."
sections:
  - id: "40.1"
    title: Vi Mode Core
    status: not-started
  - id: "40.2"
    title: Vi Motions
    status: not-started
  - id: "40.3"
    title: Vi Selection
    status: not-started
  - id: "40.4"
    title: Vi Search Integration
    status: not-started
  - id: "40.5"
    title: Section Completion
    status: not-started
---

# Section 40: Vi Mode + Copy Mode

**Status:** Not Started
**Goal:** Keyboard-driven terminal navigation and text selection without touching the mouse. Enter vi mode, navigate with hjkl and word/line motions, visually select text, yank to clipboard, and search within the scrollback. This is a must-have feature for power users — Alacritty, WezTerm, Ghostty, and Kitty all have it.

**Crate:** `oriterm` (modal input dispatch, cursor rendering)
**Dependencies:** Section 09 (Selection model), Section 11 (Search), Section 08 (Keyboard dispatch)
**Prerequisite:** Sections 08, 09, 11 complete.

**Reference:**
- Alacritty `alacritty/src/input/mod.rs` (vi mode dispatch, motions, inline search)
- WezTerm copy mode (`wezterm-gui/src/termwindow/keymap.rs`, CopyMode key table)
- Ghostty vi mode
- Kitty hints mode

**Why this matters:** Keyboard-only text selection is essential for SSH sessions, tiling WM users, and anyone who doesn't want to reach for the mouse. It's the #1 requested feature that separates a "serious" terminal from a toy.

> **Verification Notes (2026-03-29):** Confirmed not started -- no vi mode code exists. However, there is **major overlap with the existing mark mode** (`oriterm/src/app/mark_mode/`). Mark mode already implements: `MarkCursor` struct, modal input interception, `Motion` enum with Left/Right/Up/Down/PageUp/PageDown/LineStart/LineEnd/BufferStart/BufferEnd/WordLeft/WordRight, pure motion functions in `motion.rs`, `GridBounds`/`AbsCursor` types, `WordContext` with word boundary extraction, `ensure_visible()` auto-scroll, selection integration via `extend_or_create_selection()`, and Enter-to-copy/Escape-to-exit. The plan does not mention mark mode at all -- it proposes `vi_mode.rs` as a new file without addressing whether vi mode replaces, extends, or coexists with mark mode. The motion infrastructure in `mark_mode/motion.rs` is directly reusable. Additional gaps: (1) Block/rectangular selection (`Ctrl+V`) missing from `SelectionMode` enum (only Char/Word/Line exist), (2) multi-key command parser (gg, f<char>, zz) not specified, (3) no count prefix support (5j, 3w), (4) dependencies listed as "Sections 08, 09, 11 complete" but actual code for keyboard dispatch, selection, and search already exists in production. The search infrastructure (`oriterm_core/src/search/`) and `SnapshotGrid` for grid querying are also directly reusable.

---

## 40.1 Vi Mode Core

Modal input state machine. Vi mode intercepts all keyboard input and routes it through vi motion/selection logic instead of sending to the PTY.

**File:** `oriterm/src/app/vi_mode.rs`

- [ ] `ViMode` state:
  - [ ] `active: bool` — whether vi mode is engaged
  - [ ] `cursor: ViCursor` — vi cursor position (independent of terminal cursor)
  - [ ] `selection: Option<ViSelection>` — active vi selection (if any)
  - [ ] `search_direction: SearchDirection` — `Forward` or `Backward`
- [ ] `ViCursor` struct:
  - [ ] `row: usize` — absolute row index (scrollback + visible)
  - [ ] `col: usize` — column index
  - [ ] Rendered as a distinct cursor (e.g., filled block in vi mode vs normal cursor style)
- [ ] Enter/exit vi mode:
  - [ ] Toggle: configurable keybinding (default: `Ctrl+Shift+Space` or `Ctrl+Shift+X`)
  - [ ] Enter: vi cursor placed at terminal cursor position, PTY input suspended
  - [ ] Exit: clear vi selection, resume PTY input, scroll to bottom if needed
  - [ ] Also exit on: `Escape` (if no selection), `q`, `i`
- [ ] Input dispatch:
  - [ ] When vi mode active: all key events routed to vi handler (not PTY)
  - [ ] Vi handler interprets keys as motions, selections, or actions
  - [ ] Unknown keys: ignored (not forwarded to PTY)
- [ ] Scrollback access:
  - [ ] Vi cursor can move into scrollback (above visible viewport)
  - [ ] Viewport auto-scrolls to keep vi cursor visible
  - [ ] `display_offset` adjusted as vi cursor moves through history
- [ ] Vi cursor rendering:
  - [ ] Distinct visual: filled block (always visible, no blink)
  - [ ] Color: configurable `vi_mode_cursor_color` (default: bright yellow or theme accent)
  - [ ] Replaces normal terminal cursor while vi mode is active
- [ ] **Tests:**
  - [ ] Enter vi mode places cursor at terminal cursor position
  - [ ] Exit vi mode resumes PTY input
  - [ ] Escape with no selection exits vi mode
  - [ ] Keys not forwarded to PTY while vi mode active

---

## 40.2 Vi Motions

Navigation motions that move the vi cursor without creating a selection.

**File:** `oriterm/src/app/vi_mode.rs` (continued)

**Reference:** Alacritty vi motions (`alacritty/src/input/mod.rs`, vi module)

- [ ] **Character motions:**
  - [ ] `h` — move left one cell
  - [ ] `j` — move down one row
  - [ ] `k` — move up one row
  - [ ] `l` — move right one cell
- [ ] **Word motions:**
  - [ ] `w` — move to start of next word
  - [ ] `b` — move to start of previous word
  - [ ] `e` — move to end of current/next word
  - [ ] `W` — move to start of next WORD (whitespace-delimited)
  - [ ] `B` — move to start of previous WORD
  - [ ] `E` — move to end of current/next WORD
  - [ ] Word boundary detection reuses Section 09.4 `char_class()` function
- [ ] **Line motions:**
  - [ ] `0` — move to column 0 (beginning of line)
  - [ ] `^` — move to first non-blank character
  - [ ] `$` — move to last non-blank character
  - [ ] `g0` — move to first column (same as `0` in terminal context)
  - [ ] `g$` — move to last column
- [ ] **Vertical motions:**
  - [ ] `H` — move to top of viewport
  - [ ] `M` — move to middle of viewport
  - [ ] `L` — move to bottom of viewport
  - [ ] `gg` — move to top of scrollback (first row)
  - [ ] `G` — move to bottom of buffer (last row)
  - [ ] `Ctrl+U` — half page up
  - [ ] `Ctrl+D` — half page down
  - [ ] `Ctrl+B` / `PageUp` — full page up
  - [ ] `Ctrl+F` / `PageDown` — full page down
- [ ] **Bracket matching:**
  - [ ] `%` — jump to matching bracket (`()`, `[]`, `{}`, `<>`)
  - [ ] Scan forward/backward for matching pair with nesting
- [ ] **Inline search:**
  - [ ] `f<char>` — move to next occurrence of `<char>` on current line
  - [ ] `F<char>` — move to previous occurrence of `<char>` on current line
  - [ ] `t<char>` — move to just before next occurrence
  - [ ] `T<char>` — move to just after previous occurrence
  - [ ] `;` — repeat last inline search (same direction)
  - [ ] `,` — repeat last inline search (opposite direction)
- [ ] **Semantic motions:**
  - [ ] `*` — search forward for word under vi cursor
  - [ ] `#` — search backward for word under vi cursor
- [ ] Auto-scroll: viewport follows vi cursor when it moves outside visible area
- [ ] Center view: `zz` — center viewport on vi cursor row
- [ ] **Tests:**
  - [ ] `h`/`j`/`k`/`l` move cursor by one cell in each direction
  - [ ] `w` jumps to next word start
  - [ ] `b` jumps to previous word start
  - [ ] `0` moves to column 0, `$` to last non-blank
  - [ ] `gg` moves to top of scrollback, `G` to bottom
  - [ ] `%` finds matching bracket
  - [ ] `f<char>` finds character on current line
  - [ ] Cursor clamps to grid boundaries (no out-of-bounds)

---

## 40.3 Vi Selection

Visual selection modes within vi mode. Selections created in vi mode use the same Selection model from Section 09.

**File:** `oriterm/src/app/vi_mode.rs` (continued)

- [ ] **Selection modes:**
  - [ ] `v` — toggle character-wise visual selection
  - [ ] `V` — toggle line-wise visual selection
  - [ ] `Ctrl+V` — toggle block (rectangular) visual selection
- [ ] Selection mechanics:
  - [ ] First press: set anchor at vi cursor position, enter visual mode
  - [ ] Subsequent motions: extend selection from anchor to vi cursor
  - [ ] Second press of same key: cancel selection (return to normal vi mode)
  - [ ] Different visual key: switch selection mode (e.g., `v` then `V` switches to line-wise)
- [ ] Selection rendering:
  - [ ] Reuse Section 09.7 selection rendering (same visual highlight)
  - [ ] Selection updates on every vi cursor movement
  - [ ] Selection bridge: convert `ViSelection` to `Selection` for rendering
- [ ] **Yank (copy):**
  - [ ] `y` — yank (copy) current selection to clipboard, exit vi mode
  - [ ] `Y` — yank entire line(s) to clipboard, exit vi mode
  - [ ] After yank: clear selection, optionally exit vi mode (configurable)
- [ ] **Open action:**
  - [ ] `o` — open URL/hyperlink under vi cursor (same as Ctrl+click)
  - [ ] Checks OSC 8 hyperlink first, then implicit URL detection
- [ ] **Escape behavior:**
  - [ ] If selection active: `Escape` clears selection (stays in vi mode)
  - [ ] If no selection: `Escape` exits vi mode entirely
- [ ] **Tests:**
  - [ ] `v` starts character selection at cursor
  - [ ] Motion after `v` extends selection
  - [ ] `V` starts line selection
  - [ ] `Ctrl+V` starts block selection
  - [ ] `y` copies selection text and exits vi mode
  - [ ] `Escape` clears selection first, then exits vi mode

---

## 40.4 Vi Search Integration

Search from within vi mode using `/` and `?` motions.

**File:** `oriterm/src/app/vi_mode.rs` (continued)

**Reference:** Alacritty vi search (`alacritty/src/input/mod.rs`, SearchAction)

- [ ] **Enter search from vi mode:**
  - [ ] `/` — open search bar, search forward from vi cursor
  - [ ] `?` — open search bar, search backward from vi cursor
  - [ ] Reuses Section 11 search infrastructure (search bar, regex, highlighting)
- [ ] **Search result navigation:**
  - [ ] `n` — jump to next search match (same direction)
  - [ ] `N` — jump to previous search match (opposite direction)
  - [ ] Vi cursor moves to the match position
  - [ ] If selection is active: selection extends to include match
- [ ] **Search match as motion:**
  - [ ] Search match start/end positions are valid motion targets
  - [ ] `v/pattern<Enter>` — select from cursor to first match of pattern
- [ ] **Search confirmation/cancellation:**
  - [ ] `Enter` — confirm search, close search bar, vi cursor at match
  - [ ] `Escape` — cancel search, vi cursor returns to pre-search position
- [ ] **Search history:**
  - [ ] `Up`/`Down` in search bar: navigate previous searches
  - [ ] Search history shared with Section 11 search
- [ ] **Tests:**
  - [ ] `/` opens search bar in forward mode
  - [ ] `?` opens search bar in backward mode
  - [ ] `n`/`N` navigate between matches
  - [ ] `Escape` in search restores vi cursor position
  - [ ] Search with active selection extends selection to match

---

## 40.5 Section Completion

- [ ] All 40.1–40.4 items complete
- [ ] Vi mode toggles with configured keybinding
- [ ] All character, word, line, vertical, and bracket motions work
- [ ] Inline search (`f`/`F`/`t`/`T`) finds characters on current line
- [ ] Visual selection modes: character (`v`), line (`V`), block (`Ctrl+V`)
- [ ] Yank copies selection to clipboard
- [ ] `/` and `?` search integrates with Section 11 search
- [ ] `n`/`N` navigate search matches
- [ ] `*`/`#` search for word under cursor
- [ ] Vi cursor rendered with distinct style
- [ ] Auto-scroll follows vi cursor through scrollback
- [ ] `cargo clippy --target x86_64-pc-windows-gnu` — no warnings
- [ ] `cargo test` — all vi mode tests pass

**Exit Criteria:** Power users can navigate the entire scrollback, select arbitrary text, and yank to clipboard without touching the mouse. Vi mode feels natural to vim users — all standard motions work as expected.
