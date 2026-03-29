---
section: 2
title: Terminal State Machine + VTE
status: complete
reviewed: true
last_verified: "2026-03-29"
tier: 0
goal: Build Term<T> and implement all ~50 VTE handler methods so escape sequences produce correct grid state
sections:
  - id: "2.1"
    title: Event System
    status: complete
  - id: "2.2"
    title: TermMode Flags
    status: complete
  - id: "2.3"
    title: CharsetState
    status: complete
  - id: "2.4"
    title: Color Palette
    status: complete
  - id: "2.5"
    title: "Term<T> Struct"
    status: complete
  - id: "2.6"
    title: "VTE Handler — Print + Execute"
    status: complete
  - id: "2.7"
    title: "VTE Handler — CSI Sequences"
    status: complete
  - id: "2.8"
    title: "VTE Handler — SGR (Select Graphic Rendition)"
    status: complete
  - id: "2.9"
    title: "VTE Handler — OSC Sequences"
    status: complete
  - id: "2.10"
    title: "VTE Handler — ESC Sequences"
    status: complete
  - id: "2.11"
    title: "VTE Handler — DCS + Misc"
    status: complete
  - id: "2.12"
    title: RenderableContent Snapshot
    status: complete
  - id: "2.13"
    title: FairMutex
    status: complete
  - id: "2.14"
    title: Damage Tracking Integration
    status: complete
  - id: "2.15"
    title: Section Completion
    status: complete
---

# Section 02: Terminal State Machine + VTE

**Status:** Complete
**Goal:** Build `Term<T: EventListener>` that implements `vte::ansi::Handler`. Feed escape sequences in, get correct grid state out. This is the core of terminal emulation.

**Crate:** `oriterm_core`
**Dependencies:** All from Section 01, plus `base64`, `parking_lot`, `log`, `unicode-width`, `regex`
**Reference:** Alacritty `alacritty_terminal/src/term/mod.rs` for `Term<T>` pattern; Ghostty `src/terminal/Terminal.zig` for terminal state + stream handler; old `_old/src/term_handler/` for VTE method implementations.

---

## 2.1 Event System

The bridge between terminal state changes and the UI layer. Terminal fires events; UI layer handles them.

**File:** `oriterm_core/src/event/mod.rs`

- [x] `Event` enum — terminal events that flow outward
  - [x] `Wakeup` — new content available, trigger redraw
  - [x] `Bell` — BEL character received
  - [x] `Title(String)` — window title changed (OSC 0/2)
  - [x] `ResetTitle` — title reset to default
  - [x] `IconName(String)` — icon name changed (OSC 0/1)
  - [x] `ResetIconName` — icon name reset to default
  - [x] `ClipboardStore(ClipboardType, String)` — OSC 52 clipboard store
  - [x] `ClipboardLoad(ClipboardType, Arc<dyn Fn(&str) -> String + Send + Sync>)` — OSC 52 clipboard load
  - [x] `ColorRequest(usize, Arc<dyn Fn(Rgb) -> String + Send + Sync>)` — OSC 4/10/11 color query
  - [x] `PtyWrite(String)` — response bytes to write back to PTY
  - [x] `CursorBlinkingChange` — cursor blink state toggled
  - [x] `Cwd(String)` — current working directory changed (OSC 7)
  - [x] `CommandComplete(Duration)` — command completed (OSC 133;D timing)
  - [x] `MouseCursorDirty` — mouse cursor shape may need update
  - [x] `ChildExit(i32)` — child process exited with status
- [x] `ClipboardType` enum — `Clipboard`, `Selection` (primary)
- [x] `Rgb` — re-exported from `vte::ansi::Rgb` (not defined in oriterm_core)
- [x] `EventListener` trait
  - [x] `fn send_event(&self, event: Event) {}` — default no-op
  - [x] Bound: `Send + 'static`
- [x] `VoidListener` struct — no-op implementation for testing
  - [x] `impl EventListener for VoidListener {}`
- [x] Re-export from `lib.rs`
- [x] **Tests** — 15 tests, all pass (verified 2026-03-29):
  - [x] `VoidListener` compiles and implements `EventListener`
  - [x] `VoidListener` is `Send + 'static`
  - [x] All `Event` variants can be constructed (including `Cwd`, `CommandComplete`)

---

## 2.2 TermMode Flags

Bitflags for terminal mode state (DECSET/DECRST, SM/RM).

**File:** `oriterm_core/src/term/mode/mod.rs`

- [x] `TermMode` — `bitflags! { struct TermMode: u32 { ... } }`
  - [x] `SHOW_CURSOR` — DECTCEM (cursor visible)
  - [x] `APP_CURSOR` — DECCKM (application cursor keys)
  - [x] `APP_KEYPAD` — DECKPAM/DECKPNM (application keypad)
  - [x] `MOUSE_REPORT_CLICK` — mode 1000
  - [x] `MOUSE_DRAG` — mode 1002
  - [x] `MOUSE_MOTION` — mode 1003
  - [x] `MOUSE_SGR` — mode 1006 (SGR mouse encoding)
  - [x] `MOUSE_UTF8` — mode 1005 (UTF8 mouse encoding)
  - [x] `ALT_SCREEN` — mode 1049 (alternate screen)
  - [x] `LINE_WRAP` — DECAWM (auto-wrap)
  - [x] `ORIGIN` — DECOM (origin mode)
  - [x] `INSERT` — IRM (insert mode)
  - [x] `FOCUS_IN_OUT` — mode 1004 (focus events)
  - [x] `BRACKETED_PASTE` — mode 2004
  - [x] `SYNC_UPDATE` — mode 2026 (synchronized output)
  - [x] `URGENCY_HINTS` — mode 1042
  - [x] `CURSOR_BLINKING` — ATT610
  - [x] `LINE_FEED_NEW_LINE` — LNM (LF acts as CR+LF)
  - [x] `ALTERNATE_SCROLL` — mode 1007 (wheel sends arrow keys in alt screen)
  - [x] `REVERSE_WRAP` — mode 45 (BS wraps to previous line)
  - [x] `MOUSE_URXVT` — mode 1015 (URXVT mouse encoding)
  - [x] `MOUSE_X10` — mode 9 (X10 mouse reporting)
  - [x] Kitty keyboard protocol flags: `DISAMBIGUATE_ESC_CODES`, `REPORT_EVENT_TYPES`, `REPORT_ALTERNATE_KEYS`, `REPORT_ALL_KEYS_AS_ESC`, `REPORT_ASSOCIATED_TEXT`
  - [x] `ANY_MOUSE` — computed: CLICK | DRAG | MOTION | X10
  - [x] `ANY_MOUSE_ENCODING` — computed: SGR | UTF8 | URXVT
  - [x] `KITTY_KEYBOARD_PROTOCOL` — computed: all five Kitty keyboard flags
  - [x] Default: `SHOW_CURSOR | LINE_WRAP | ALTERNATE_SCROLL`
  - [x] `From<vte::ansi::KeyboardModes>` conversion impl
- [x] **Tests** — 12 tests, all pass (verified 2026-03-29):
  - [x] Default mode has `SHOW_CURSOR`, `LINE_WRAP`, and `ALTERNATE_SCROLL` set
  - [x] Can set/clear individual modes
  - [x] `ANY_MOUSE` is the union of all mouse modes (including X10)
  - [x] `ANY_MOUSE_ENCODING` is the union of encoding modes
  - [x] `KITTY_KEYBOARD_PROTOCOL` is the union of all Kitty flags
  - [x] `KeyboardModes` to `TermMode` conversion
  - [x] All individual flags are distinct (power-of-two bits)
  - [x] `TermMode` is 4 bytes (u32 regression guard)

---

## 2.3 CharsetState

Character set translation (G0-G3, single shifts). Needed for DEC special graphics and national character sets.

**File:** `oriterm_core/src/term/charset/mod.rs`

- [x] `StandardCharset` — re-exported from `vte::ansi::StandardCharset` (not a custom enum)
- [x] `CharsetIndex` — re-exported from `vte::ansi::CharsetIndex` (`G0`, `G1`, `G2`, `G3`)
- [x] `CharsetState` struct
  - [x] Fields:
    - `charsets: [StandardCharset; 4]` — G0-G3 (default: all `StandardCharset::Ascii`)
    - `active: CharsetIndex` — currently active charset (default: G0)
    - `single_shift: Option<CharsetIndex>` — SS2/SS3 single shift
  - [x] `translate(&mut self, ch: char) -> char` — apply charset mapping via `StandardCharset::map()`
    - [x] If single_shift is set, use that charset for one char, then clear
    - [x] DEC special graphics mapping provided by vte's `StandardCharset::SpecialCharacterAndLineDrawing`
  - [x] `set_charset(&mut self, index: CharsetIndex, charset: StandardCharset)`
  - [x] `set_active(&mut self, index: CharsetIndex)`
  - [x] `set_single_shift(&mut self, index: CharsetIndex)`
- [x] **Tests** — all pass (verified 2026-03-29):
  - [x] Default: all ASCII, no translation
  - [x] DEC special graphics: `'q'` (0x71) → `'─'` (U+2500)
  - [x] Full box-drawing character mapping (corners, lines, cross)
  - [x] Single shift: applies for one char then reverts
  - [x] G0/G1 switching
  - [x] Chars outside mapping range pass through unchanged
  - [x] Single shift overrides active charset for one character

---

## 2.4 Color Palette

270-entry color palette: 16 ANSI + 216 cube + 24 grayscale + named colors. Resolves `vte::ansi::Color` enum to `Rgb`.

**File:** `oriterm_core/src/color/palette/mod.rs`, `oriterm_core/src/color/mod.rs`
**Theme dependency:** `oriterm_core/src/theme/mod.rs` — `Theme` enum (`Dark`, `Light`, `Unknown`), `is_dark()`, `Default` → Dark. Tests: default is Dark, Dark/Unknown are dark, Light is not dark.

- [x] `NUM_COLORS: usize = 270` — total palette entries
- [x] Private helpers: `build_palette(theme)`, `fill_cube(colors)`, `fill_grayscale(colors)`
- [x] `Palette` struct
  - [x] Fields:
    - `colors: [Rgb; 270]` — live palette (0..=255 indexed, 256..269 semantic: foreground, background, cursor, dim variants, bright/dim foreground)
    - `defaults: [Rgb; 270]` — factory defaults for OSC 104 reset
    - `selection_fg: Option<Rgb>` — user-configured selection foreground override
    - `selection_bg: Option<Rgb>` — user-configured selection background override
  - [x] `Palette::default()` — delegates to `for_theme(Theme::default())` (dark theme)
  - [x] `Palette::for_theme(theme: Theme)` — theme-aware palette (dark/light semantic colors, shared indexed colors)
  - [x] `Palette::from_scheme_colors(ansi, fg, bg, cursor)` — build from explicit color scheme
  - [x] `resolve(&self, color: Color) -> Rgb` — resolve `vte::ansi::Color` enum to RGB (no `is_fg` param)
    - [x] `Color::Named(n)` → `self.colors[n as usize]`
    - [x] `Color::Spec(rgb)` → direct RGB passthrough
    - [x] `Color::Indexed(idx)` → `self.colors[idx as usize]`
  - [x] `set_indexed(&mut self, index: usize, color: Rgb)` — OSC 4
  - [x] `reset_indexed(&mut self, index: usize)` — OSC 104 (resets to `defaults`)
  - [x] `set_default(&mut self, index: usize, color: Rgb)` — config override (sets both live and default)
  - [x] `foreground(&self) -> Rgb` — default foreground
  - [x] `background(&self) -> Rgb` — default background
  - [x] `cursor_color(&self) -> Rgb` — cursor color
  - [x] `color(&self, index: usize) -> Rgb` — lookup by index (for OSC query responses)
  - [x] Selection color accessors: `selection_fg()`, `selection_bg()`, `set_selection_fg()`, `set_selection_bg()`, `selection_colors()`
- [x] `SelectionColors` struct — `{ fg: Option<Rgb>, bg: Option<Rgb> }`
- [x] `dim_rgb(c: Rgb) -> Rgb` — reduce to 2/3 brightness (used for dim ANSI variants); `pub(crate)` visibility
- [x] Palette convenience setters (each delegates to `set_default`):
  - [x] `set_foreground(&mut self, color: Rgb)` — override foreground
  - [x] `set_background(&mut self, color: Rgb)` — override background
  - [x] `set_cursor_color(&mut self, color: Rgb)` — override cursor color
- [x] `mod.rs`: re-export `Palette`, `Rgb`, `SelectionColors`; `dim_rgb` re-exported as `pub(crate)`
- [x] **Tests** — ~30 tests, all pass (verified 2026-03-29):
  - [x] Default palette: color 0 is black, color 7 is white (Tango), color 15 is bright white
  - [x] 256-color cube: indices 16–231 map correctly (formula verified)
  - [x] Grayscale ramp: indices 232–255 (all 24 steps verified)
  - [x] `resolve` handles Named, Spec, Indexed variants
  - [x] `set_indexed` / `reset_indexed` work (roundtrip verified)
  - [x] Dark/light theme palette differences (foreground, background, cursor)
  - [x] `from_scheme_colors` preserves ANSI, cube, grayscale, derives dim variants
  - [x] Selection colors default to None, roundtrip correctly, don't bleed into indexed palette
  - [x] `set_default` changes reset baseline (OSC 104 resets to config color, not xterm default)
  - [x] `dim_rgb` edge cases: black stays black, white produces 170

---

## 2.5 Term\<T\> Struct

The terminal state machine. Owns two grids (primary + alternate), mode flags, palette, charset, title, keyboard mode stack. Generic over `EventListener` for decoupling from UI.

**File:** `oriterm_core/src/term/mod.rs`, `oriterm_core/src/term/shell_state.rs`, `oriterm_core/src/term/alt_screen.rs`

> **NOTE (resolved):** `term/mod.rs` was previously 505 lines. As of 2026-03-29 verification it is **461 lines** (under 500-line limit). Shell types remain inline since the file is under the limit. No extraction needed.

**Theme dependency:** `oriterm_core/src/theme/mod.rs` — `Theme` enum (`Dark`, `Light`, `Unknown`) with `is_dark()` and `Default` (Dark). Used by `Palette::for_theme()` and `Term::new()`.

- [x] `Term<T: EventListener>` struct
  - [x] Fields:
    - `grid: Grid` — primary grid (active when not in alt screen)
    - `alt_grid: Option<Grid>` — alternate grid, lazy-allocated (active during alt screen; no scrollback). Implementation uses `Option<Grid>` to save ~28 KB per terminal that never uses alt screen.
    - `mode: TermMode` — terminal mode flags (active grid determined by `ALT_SCREEN` flag, no separate bool)
    - `palette: Palette` — color palette (270 entries)
    - `theme: Theme` — active color theme (dark/light)
    - `charset: CharsetState` — character set translation state (G0-G3)
    - `title: String` — window title (set by OSC 0/2)
    - `icon_name: String` — icon name (set by OSC 0/1)
    - `cwd: Option<String>` — current working directory (OSC 7)
    - `title_stack: VecDeque<String>` — pushed titles (xterm extension, capped at 4096)
    - `cursor_shape: CursorShape` — cursor shape for rendering
    - `keyboard_mode_stack: VecDeque<KeyboardModes>` — Kitty keyboard protocol stack (capped at 4096)
    - `inactive_keyboard_mode_stack: VecDeque<KeyboardModes>` — stack for inactive screen
    - `event_listener: T` — event sink
    - `selection_dirty: bool` — content-modifying operations set this for selection invalidation
    - `prompt_state: PromptState` — OSC 133 shell integration lifecycle
    - `pending_marks: PendingMarks` — deferred OSC 133 row marking
    - `prompt_markers: Vec<PromptMarker>` — prompt lifecycle markers for navigation
    - `pending_notifications: Vec<Notification>` — OSC 9/99/777 desktop notifications
    - `command_start: Option<Instant>` — OSC 133;C timing
    - `last_command_duration: Option<Duration>` — last completed command duration
    - `has_explicit_title: bool` — whether title was explicitly set via OSC
    - `title_dirty: bool` — title changed since last check
    - `saved_private_modes: HashMap<u16, bool>` — XTSAVE/XTRESTORE state
  - [x] `Term::new(lines, cols, scrollback, theme: Theme, listener: T) -> Self`
    - [x] Create primary grid with scrollback
    - [x] Create alt grid (no scrollback)
    - [x] Theme-aware palette via `Palette::for_theme(theme)`
    - [x] Default mode, charset, empty title
  - [x] `grid(&self) -> &Grid` — active grid (checks `ALT_SCREEN` mode flag)
  - [x] `grid_mut(&mut self) -> &mut Grid` — active grid (mutable)
  - [x] `mode(&self) -> TermMode`
  - [x] `palette(&self) -> &Palette`, `palette_mut(&mut self) -> &mut Palette`
  - [x] `title(&self) -> &str`, `icon_name(&self) -> &str`, `cwd(&self) -> Option<&str>`
  - [x] `cursor_shape(&self) -> CursorShape`, `set_cursor_shape(&mut self, shape)`
  - [x] `theme(&self) -> Theme`, `set_theme(&mut self, theme)` — rebuilds palette, marks dirty
  - [x] `is_selection_dirty()`, `clear_selection_dirty()` — selection invalidation
  - [x] `resize(&mut self, lines, cols)` — resizes both grids (primary with reflow, alt without)
  - [x] Alt screen swap — three variants in `alt_screen.rs`:
    - [x] `swap_alt()` — mode 1049: save/restore cursor, toggle `ALT_SCREEN`, swap keyboard stacks, mark dirty
    - [x] `swap_alt_no_cursor()` — mode 47: no cursor save/restore
    - [x] `swap_alt_clear()` — mode 1047: clears alt grid before entering
- [x] Shell integration types (in `term/mod.rs`):
  - [x] `PromptState` enum — `None`, `PromptStart`, `CommandStart`, `OutputStart` (OSC 133 lifecycle)
  - [x] `PromptMarker` struct — `prompt: usize`, `command: Option<usize>`, `output: Option<usize>` (absolute row indices)
  - [x] `Notification` struct — `title: String`, `body: String` (OSC 9/99/777)
  - [x] `PendingMarks` bitflags — `PROMPT`, `COMMAND_START`, `OUTPUT_START` (deferred OSC 133 marking)
- [x] Shell state methods (`shell_state.rs`):
  - [x] Prompt state: `prompt_state()`, `set_prompt_state()`, `prompt_mark_pending()`, `set_prompt_mark_pending()`
  - [x] Prompt row marking: `mark_prompt_row()`, `mark_command_start_row()`, `mark_output_start_row()`
  - [x] Prompt navigation: `scroll_to_previous_prompt()`, `scroll_to_next_prompt()`, `scroll_to_absolute_row()`
  - [x] Prompt markers: `prompt_markers()`, `prune_prompt_markers(evicted)`, `command_output_range(near_row)`, `command_input_range(near_row)`
  - [x] Command timing: `set_command_start(Instant)`, `finish_command() -> Option<Duration>`, `last_command_duration()`
  - [x] Notifications: `drain_notifications()`, `push_notification()`
  - [x] Title state: `has_explicit_title()`, `set_has_explicit_title()`, `is_title_dirty()`, `clear_title_dirty()`, `mark_title_dirty()`, `set_cwd()`, `effective_title()`
- [x] `cwd_short_path(cwd: &str) -> &str` — free function, extracts last path component for tab display
- [x] **Tests** (`term/tests.rs`) — 50+ tests, all pass (verified 2026-03-29):
  - [x] `Term::new(24, 80, 0, Theme::default(), VoidListener)` creates a working terminal
  - [x] `grid()` returns primary grid by default
  - [x] `swap_alt()` switches to alt grid and back
  - [x] Mode defaults include SHOW_CURSOR and LINE_WRAP
  - [x] Alt grid has no scrollback, primary grid has scrollback
  - [x] Swap alt preserves keyboard mode stacks
  - [x] Dark/light theme palette integration
  - [x] Selection dirty tracking for content-modifying operations
  - [x] Resize changes both grids, marks dirty
  - [x] `mark_prompt_row` creates marker with prompt row only
  - [x] `mark_command_start` fills last marker
  - [x] `mark_output_start` fills last marker
  - [x] `mark_prompt_row` avoids duplicate entries at same row
  - [x] `prune_prompt_markers` removes evicted, adjusts remaining row indices
  - [x] `command_output_range` returns correct bounds, bounded by next prompt
  - [x] `command_input_range` returns correct bounds
  - [x] Range methods return None when no markers, no output_start, no command_start
  - [x] `scroll_to_previous_prompt` / `scroll_to_next_prompt` scroll viewport
  - [x] RIS clears prompt state, CWD, title state, command timing, pending notifications
  - [x] `drain_notifications` returns empty on second call
  - [x] Multiple prompt starts without completion create separate markers
  - [x] Prompt markers survive subsequent output and scrolling
  - [x] `cwd_short_path`: last component, root, trailing slash, single dir, triple slash
  - [x] Scroll region preserves scrollback content
  - [x] Scrollback survives region scroll_down
  - [x] Resize with VTE-wrapped content

---

## 2.6 VTE Handler — Print + Execute

`impl vte::ansi::Handler for Term<T>`. The `input` method (print) and control character execution.

**File:** `oriterm_core/src/term/handler/mod.rs`, `oriterm_core/src/term/handler/helpers.rs`

**Handler helpers** (`helpers.rs`): `try_reverse_wrap()` (backspace reverse wrap), `goto_origin_aware(line, col)` (ORIGIN-mode CUP/VPA), `crate_version_number()` (DA2 version encoding), `mode_report_value(bool)` (DECRPM), `named_private_mode_number(mode)` (mode-to-CSI-number map), `named_private_mode_flag(mode)` (mode-to-TermMode map).

- [x] `impl<T: EventListener> Handler for Term<T>` (implements `vte::ansi::Handler`)
- [x] `fn input(&mut self, c: char)`
  - [x] Sets `selection_dirty = true`
  - [x] Translate through charset (`self.charset.translate(c)`)
  - [x] If `UnicodeWidthChar::width(c) == Some(0)`: append to previous cell's zerowidth list via `push_zerowidth()` (combining marks, variation selectors)
  - [x] If `width == None` (control chars): return early
  - [x] If INSERT mode active: shift content right before writing
  - [x] Call `self.grid_mut().put_char(c)` (grid handles auto-wrap internally)
- [x] Control characters (VTE calls named trait methods, not a single `execute`):
  - [x] `fn bell()` — `self.event_listener.send_event(Event::Bell)`
  - [x] `fn backspace()` — move cursor left (with reverse wrap support for mode 45)
  - [x] `fn put_tab(count)` — tab forward
  - [x] `fn linefeed()` — LF (with LNM mode: CR+LF)
  - [x] `fn carriage_return()` — CR
  - [x] `fn substitute()` — treated as space per ECMA-48
  - [x] `fn set_active_charset(index)` — SO/SI charset switching
  - [x] `fn configure_charset(index, charset)` — ESC ( / ) / * / +
  - [x] `fn set_single_shift(index)` — SS2/SS3
- [x] **Tests** (feed bytes through `vte::ansi::Processor`) — all pass (verified 2026-03-29):
  - [x] `"hello"` → cells 0..5 contain h,e,l,l,o; cursor at col 5
  - [x] `"hello\nworld"` → "hello" on line 0, "world" on line 1
  - [x] `"hello\rworld"` → "world" on line 0 (overwrites "hello")
  - [x] `"\t"` → cursor advances to column 8
  - [x] `"\x08"` → cursor moves left
  - [x] BEL triggers Event::Bell on a recording listener
  - [x] `"e\u{0301}"` → cell 0 has `ch='e'`, `zerowidth=['\u{0301}']`, cursor at col 1
  - [x] Multiple combining marks append to same cell's zerowidth list
  - [x] Zero-width char at column 0 (no previous cell) is discarded gracefully
  - [x] Combining mark on wide char appends to wide cell
  - [x] Combining mark at wrap-pending appends to last cell of previous line
  - [x] ZWJ, ZWNBSP (word joiner), variation selectors (VS15, VS16) all store as zerowidth
  - [x] ZWJ emoji sequence stores each base emoji separately
  - [x] Mixed zerowidth types on same cell accumulate
  - [x] Combining mark after line wrap appends to correct cell
  - [x] Combining mark on wide char after wrap works correctly
  - [x] Dirty tracking fires for combining mark and zerowidth writes

---

## 2.7 VTE Handler — CSI Sequences

Cursor movement, erase, scroll, insert/delete, device status, mode setting.

**File:** `oriterm_core/src/term/handler/mod.rs` (continued), `oriterm_core/src/term/handler/modes.rs`

- [x] Cursor movement CSIs:
  - [x] `CUU` (CSI n A) — `move_up(n)`
  - [x] `CUD` (CSI n B) — `move_down(n)`
  - [x] `CUF` (CSI n C) — `move_forward(n)`
  - [x] `CUB` (CSI n D) — `move_backward(n)`
  - [x] `CNL` (CSI n E) — move down n, column 0
  - [x] `CPL` (CSI n F) — move up n, column 0
  - [x] `CHA` (CSI n G) — `move_to_column(n-1)` (1-based)
  - [x] `CUP` (CSI n;m H) — `move_to(n-1, m-1)` (1-based)
  - [x] `VPA` (CSI n d) — `move_to_line(n-1)` (1-based)
  - [x] `HVP` (CSI n;m f) — same as CUP
- [x] Erase CSIs:
  - [x] `ED` (CSI n J) — `erase_display(mode)`
  - [x] `EL` (CSI n K) — `erase_line(mode)`
  - [x] `ECH` (CSI n X) — `erase_chars(n)`
- [x] Insert/Delete CSIs:
  - [x] `ICH` (CSI n @) — `insert_blank(n)`
  - [x] `DCH` (CSI n P) — `delete_chars(n)`
  - [x] `IL` (CSI n L) — `insert_lines(n)`
  - [x] `DL` (CSI n M) — `delete_lines(n)`
- [x] Scroll CSIs:
  - [x] `SU` (CSI n S) — `scroll_up(n)`
  - [x] `SD` (CSI n T) — `scroll_down(n)`
- [x] Tab CSIs:
  - [x] `CHT` (CSI n I) — tab forward n times
  - [x] `CBT` (CSI n Z) — tab backward n times
  - [x] `TBC` (CSI n g) — clear tab stops
- [x] Mode CSIs:
  - [x] `SM` (CSI n h) — set ANSI mode
  - [x] `RM` (CSI n l) — reset ANSI mode
  - [x] `DECSET` (CSI ? n h) — set DEC private mode
  - [x] `DECRST` (CSI ? n l) — reset DEC private mode
  - [x] Supported DECSET/DECRST modes: 1 (DECCKM), 6 (DECOM), 7 (DECAWM), 9 (X10 mouse), 12 (cursor blinking), 25 (DECTCEM), 45 (reverse wraparound), 47/1047/1048/1049 (alt screen variants + save cursor), 1000/1002/1003 (mouse tracking), 1004 (focus), 1005/1006/1015 (mouse encoding), 1007 (alternate scroll), 1042 (urgency hints), 2004 (bracketed paste), 2026 (sync output)
  - [x] Mouse modes are mutually exclusive (setting one clears others)
  - [x] Mouse encoding modes are mutually exclusive
  - [x] `XTSAVE` (CSI ? s) / `XTRESTORE` (CSI ? r) — save/restore private mode values
- [x] Device status:
  - [x] `DSR` (CSI 6 n) — report cursor position (CPR response)
  - [x] `DA` (CSI c) — primary device attributes response
  - [x] `DA2` (CSI > c) — secondary device attributes response
- [x] Scroll region:
  - [x] `DECSTBM` (CSI n;m r) — `set_scroll_region(n-1, m)`
- [x] `DECSC` (CSI s when not in alt screen) — save cursor
- [x] `DECRC` (CSI u when not in alt screen) — restore cursor
- [x] `DECRPM` (CSI ? n $ p) — report mode (respond if mode is set/reset)
- [x] **Tests** (feed CSI sequences through `vte::ansi::Processor` — `handler/tests.rs`) — all pass, protocol verified against Alacritty byte-for-byte (verified 2026-03-29):
  - [x] `ESC[5A` moves cursor up 5
  - [x] `ESC[10;20H` moves cursor to line 9, column 19 (0-based)
  - [x] `ESC[2J` clears screen
  - [x] `ESC[K` clears to end of line
  - [x] `ESC[5@` inserts 5 blanks
  - [x] `ESC[3P` deletes 3 chars
  - [x] `ESC[2L` inserts 2 lines
  - [x] `ESC[3M` deletes 3 lines
  - [x] `ESC[?25l` hides cursor (DECTCEM), `ESC[?25h` shows cursor
  - [x] `ESC[?1049h` switches to alt screen, `ESC[?1049l` switches back
  - [x] `ESC[3;20r` sets scroll region
  - [x] `ESC[6n` produces cursor position report
  - [x] Origin mode: CUP/VPA relative to scroll region, clamps within region
  - [x] IRM insert mode shifts content, replace mode overwrites
  - [x] LNM mode: LF acts as CR+LF
  - [x] CHA default/overflow, CNL/CPL move + column 0
  - [x] DSR code 5 (OK), DA1, DA2 (version-encoded)
  - [x] DECRPM reports set/reset for private and ANSI modes
  - [x] ECH overflow clamps, SU/SD scroll content
  - [x] RI at top of scroll region, in middle, outside region
  - [x] DECSC/DECRC save/restore cursor
  - [x] CHT/CBT forward/backward tab by count, TBC clear tab stops
  - [x] NEL performs CR+LF
  - [x] Alt screen preserves/restores cursor, mode 47/1047/1048 variants
  - [x] Mouse mode mutual exclusion (1000/1002/1003/9), encoding mutual exclusion (1005/1006/1015)
  - [x] Reverse wraparound (mode 45), XTSAVE/XTRESTORE
  - [x] Wide char rendering (two cells, spacer, wrap at boundary, single-column grid)
  - [x] Unknown DECSET/DECRST modes silently ignored
  - [x] DECSTBM edge cases: top > bottom ignored, top == bottom ignored, no-params resets to full screen
  - [x] SU/SD count exceeding region size handled correctly
  - [x] Insert/delete lines outside scroll region is noop
  - [x] CHT count zero treated as one, count three advances three stops
  - [x] CBT at column past end goes to last stop
  - [x] Scroll up in scroll region preserves content outside region
  - [x] DECRST mouse tracking does not reactivate previous mode
  - [x] DECRST encoding reverts to no encoding (all three: SGR, UTF8, URXVT)
  - [x] DECRST single mode preserves unrelated active modes
  - [x] Double-enter alt screen (mode 47) is noop
  - [x] Mode 1049 enter then mode 47 exit interaction
  - [x] Alt screen with scroll region interaction
  - [x] Wrap pending cleared by cursor movement
  - [x] Print past last column wraps to next line

---

## 2.8 VTE Handler — SGR (Select Graphic Rendition)

Cell attribute setting: bold, italic, underline, colors. The most complex CSI.

**File:** `oriterm_core/src/term/handler/sgr.rs`

- [x] `fn terminal_attribute(&mut self, attr: Attr)` dispatches to `sgr::apply(template, &attr)`. VTE parses SGR params into `Attr` variants:
  - [x] `0` — reset all attributes (clear template flags and colors)
  - [x] `1` — bold
  - [x] `2` — dim
  - [x] `3` — italic
  - [x] `4` — underline (with sub-params: `4:0` none, `4:1` single, `4:3` curly, `4:4` dotted, `4:5` dashed)
  - [x] `5` — blink
  - [x] `7` — inverse
  - [x] `8` — hidden
  - [x] `9` — strikethrough
  - [x] `21` — double underline
  - [x] `22` — neither bold nor dim
  - [x] `23` — not italic
  - [x] `24` — not underline
  - [x] `25` — not blink
  - [x] `27` — not inverse
  - [x] `28` — not hidden
  - [x] `29` — not strikethrough
  - [x] `30..=37` — set foreground (ANSI 0–7)
  - [x] `38` — set foreground (extended): `38;5;n` (256-color) or `38;2;r;g;b` (truecolor)
  - [x] `39` — default foreground
  - [x] `40..=47` — set background (ANSI 0–7)
  - [x] `48` — set background (extended)
  - [x] `49` — default background
  - [x] `58` — set underline color (extended): `58;5;n` or `58;2;r;g;b`
  - [x] `59` — default underline color
  - [x] `90..=97` — set bright foreground (ANSI 8–15)
  - [x] `100..=107` — set bright background (ANSI 8–15)
- [x] **Tests** — all pass, protocol verified against Alacritty (verified 2026-03-29):
  - [x] `ESC[1m` sets bold; `ESC[2m` dim; `ESC[3m` italic; `ESC[5m` blink; `ESC[7m` inverse; `ESC[8m` hidden; `ESC[9m` strikethrough
  - [x] Cancel variants: `ESC[22m` (bold+dim), `ESC[23m` (italic), `ESC[24m` (all underlines), `ESC[25m` (blink), `ESC[27m` (inverse), `ESC[28m` (hidden), `ESC[29m` (strikethrough)
  - [x] `ESC[31m` sets fg to red (ANSI 1)
  - [x] `ESC[38;5;196m` sets fg to 256-color index 196; `ESC[48;5;n` background
  - [x] `ESC[38;2;255;128;0m` sets fg to truecolor RGB; `ESC[48;2;r;g;b` background
  - [x] `ESC[0m` resets all attributes
  - [x] `ESC[1;31;42m` compound: bold + red fg + green bg
  - [x] `ESC[4:3m` curly underline; `ESC[4:4m` dotted; `ESC[4:5m` dashed; `ESC[21m` double underline
  - [x] Underline types are mutually exclusive (setting one clears others)
  - [x] `ESC[58;2;255;0;0m` sets underline color (truecolor); `ESC[58;5;n` (indexed)
  - [x] `ESC[59m` clears underline color; underline color survives underline type change
  - [x] `ESC[39m` resets fg only; `ESC[49m` resets bg only
  - [x] `ESC[90..97m` bright foreground; `ESC[100..107m` bright background
  - [x] Printed chars inherit SGR attributes from cursor template
  - [x] SGR persists across cursor movement, stacks across separate sequences
  - [x] Fast blink (5) uses same BLINK flag as slow blink
  - [x] Colon separator equivalent: `38:5:n` and `38:2:r:g:b` (both fg and bg)
  - [x] `ESC[22m` cancels both bold and dim independently
  - [x] Cancel one attribute preserves others (e.g., cancel underline preserves bold; cancel bold preserves italic+color)
  - [x] Underline type replaces previous type (single replaces curly, double replaces single)
  - [x] Empty SGR params reset all attributes
  - [x] Last color in compound SGR wins
  - [x] SGR reset between chars gives different attributes on different cells
  - [x] SGR reset (`ESC[0m`) clears underline color

---

## 2.9 VTE Handler — OSC Sequences

Operating System Commands: title, palette, clipboard.

**File:** `oriterm_core/src/term/handler/osc.rs`

- [x] `OSC 0` — set icon name + window title (VTE calls both `set_title` and `set_icon_name`)
  - [x] Sets title, fires `Event::Title(...)`, sets `has_explicit_title = true`
- [x] `OSC 1` — set icon name (VTE calls `set_icon_name`)
  - [x] Sets icon_name, fires `Event::IconName(...)`
- [x] `OSC 2` — set window title (VTE calls `set_title`)
- [x] `OSC 4` — set/query indexed color
  - [x] `set_color(index, rgb)` → `palette.set_indexed(index, color)`, marks grid dirty
  - [x] `dynamic_color_sequence(prefix, index, terminator)` → query: sends `Event::ColorRequest`
- [x] `OSC 7` — set working directory (handled via raw OSC interceptor, stored as `Term.cwd`)
- [x] `OSC 8` — hyperlink
  - [x] `set_hyperlink(Some(link))` → set hyperlink on cursor template via `Hyperlink::from`
  - [x] `set_hyperlink(None)` → clear hyperlink
- [x] `OSC 10` — set/query default foreground color
- [x] `OSC 11` — set/query default background color
- [x] `OSC 12` — set/query cursor color (cursor color changes don't mark grid dirty)
- [x] `OSC 52` — clipboard operations (base64 encoded via `base64` crate)
  - [x] `clipboard_store(clipboard, base64)` → decode, validate UTF-8, send `Event::ClipboardStore`
  - [x] `clipboard_load(clipboard, terminator)` → send `Event::ClipboardLoad` with response closure
  - [x] Supports selectors: `c` (clipboard), `p`/`s` (primary selection)
- [x] `OSC 104` — reset indexed color to default
- [x] `OSC 110` — reset foreground color
- [x] `OSC 111` — reset background color
- [x] `OSC 112` — reset cursor color
- [x] Title stack: `push_title()` / `pop_title()` (xterm extension, capped at 4096)
- [x] **Tests** — all pass, OSC 52 clipboard response format byte-for-byte matches Alacritty (verified 2026-03-29):
  - [x] OSC 0/1/2: title setting, icon name setting, ST terminator, semicolons in title
  - [x] Push/pop title stack (interleaved, cap at 4096, pop empty is noop)
  - [x] OSC 4: set color, query with ColorRequest event, multiple colors, out-of-range ignored, set/reset roundtrip
  - [x] OSC 10/11/12: set/query foreground, background, cursor color (set+query roundtrip)
  - [x] OSC 104/110/111/112: reset indexed, foreground, background, cursor
  - [x] OSC 52: clipboard store (base64 decode), load, primary selection (`p`/`s`), invalid base64/UTF-8 ignored, empty payload, multiline, CRLF, large payload, no-padding/double-padding base64
  - [x] OSC 8: set hyperlink, with id parameter, clear, survives SGR reset, written to cells, URI with semicolons
  - [x] OSC set color marks grid dirty, cursor color does not
  - [x] OSC title None resets, empty string, UTF-8 multibyte
  - [x] OSC 104 no-params resets all indexed colors
  - [x] OSC 4 set-then-query roundtrip, OSC 10/11/12 set-then-query roundtrips
  - [x] OSC 4 set-reset-then-verify roundtrip
  - [x] OSC 1 sets icon name only (not title), OSC 0 sets both, OSC 2 does not set icon name
  - [x] OSC 52 multi-selector uses first, missing data param ignored
  - [x] OSC 52 truncated base64 handled gracefully
  - [x] OSC 52 load response formatting (BEL vs ST terminator, valid base64 output)
  - [x] OSC set_icon_name None resets and fires ResetIconName event

---

## 2.10 VTE Handler — ESC Sequences

Escape sequences (non-CSI): charset designation, cursor save/restore, index/reverse index, full reset.

**File:** `oriterm_core/src/term/handler/mod.rs`, `oriterm_core/src/term/handler/esc.rs` (RIS only)

- [x] `ESC 7` / `DECSC` — save cursor position + attributes
- [x] `ESC 8` / `DECRC` — restore cursor position + attributes
- [x] `ESC D` / `IND` — index (linefeed without CR)
- [x] `ESC E` / `NEL` — next line (CR + LF)
- [x] `ESC H` / `HTS` — horizontal tab set
- [x] `ESC M` / `RI` — reverse index
- [x] `ESC c` / `RIS` — full reset (reset all state to initial)
- [x] `ESC (` / `ESC )` / `ESC *` / `ESC +` — designate G0/G1/G2/G3 charset
  - [x] `B` → ASCII, `0` → DEC Special Graphics
- [x] `ESC =` / `DECKPAM` — application keypad mode
- [x] `ESC >` / `DECKPNM` — normal keypad mode
- [x] `ESC N` / `SS2` — single shift G2
- [x] `ESC O` / `SS3` — single shift G3
- [x] **Tests** — all pass (verified 2026-03-29):
  - [x] `ESC7` + `ESC8` saves/restores cursor position
  - [x] `ESC7` + `ESC8` preserves SGR attributes and wrap-pending state
  - [x] `ESCD` (IND) at bottom line scrolls up
  - [x] `ESCM` (RI) at top of scroll region scrolls down, in middle just moves up
  - [x] `ESCc` (RIS) resets all state: mode, pen, palette, cursor shape and blinking, alt screen, keyboard modes and flags, mouse modes, hyperlink, origin mode, grid content, saved cursor, prompt state, CWD, title
  - [x] `ESC(0` activates DEC special graphics, `ESC(B` restores ASCII
  - [x] Full DEC special graphics mapping tested
  - [x] G1 configuration doesn't affect G0, SO/SI charset switching
  - [x] Single shift applies to one character only
  - [x] DEC special charset ignores non-ASCII input
  - [x] `ESC7`/`ESC8` preserves hyperlink on cursor

---

## 2.11 VTE Handler — DCS + Misc

Device Control Strings and remaining handler methods.

**File:** `oriterm_core/src/term/handler/dcs.rs`, `oriterm_core/src/term/handler/status.rs`

- [x] Kitty keyboard protocol (`dcs.rs`):
  - [x] `push_keyboard_mode(mode)` — push onto `VecDeque<KeyboardModes>` (capped at 4096)
  - [x] `pop_keyboard_modes(to_pop)` — pop from stack, reload active mode
  - [x] `set_keyboard_mode(mode, apply)` — apply with Replace/Union/Difference behavior
  - [x] `report_keyboard_mode()` — respond with `CSI ? <bits> u`
- [x] DECSCUSR: `set_cursor_style(Option<CursorStyle>)` (`dcs.rs`)
  - [x] Sets `cursor_shape` from `CursorStyle.shape`, updates `CURSOR_BLINKING` mode flag
  - [x] `None` resets to default block cursor
  - [x] Also: `set_cursor_shape(shape)` — shape only, no blinking change
- [x] `modifyOtherKeys`: stub impl (logs and ignores)
- [x] `text_area_size_pixels()`: stub (reports 0x0 until wired to GUI)
- [x] Device status (`status.rs`):
  - [x] `identify_terminal(intermediate)` — DA1 (`ESC[?6c`) and DA2 (version-encoded)
  - [x] `device_status(arg)` — DSR code 5 (OK) and code 6 (cursor position report)
  - [x] `text_area_size_chars()` — CSI 18 t response
  - [x] `report_mode(mode)` — DECRQM for ANSI modes
  - [x] `report_private_mode(mode)` — DECRQM for DEC private modes
- [x] XTSAVE/XTRESTORE implementation in `modes.rs`: `save_private_mode_values(modes)`, `restore_private_mode_values(modes)` (dispatched from CSI in 2.7)
- [x] Unhandled sequences:
  - [x] Log at `debug!` level, do not panic or error
  - [x] Return gracefully from handler methods
- [x] **Tests** — all pass (verified 2026-03-29):
  - [x] All DECSCUSR values 0-6 set correct shape and blinking (e.g., `ESC[1 q` blinking block, `ESC[5 q` blinking bar)
  - [x] DECSCUSR fires `CursorBlinkingChange` event
  - [x] DECSCUSR same shape twice is idempotent
  - [x] `ESC[>1u` pushes keyboard mode
  - [x] `ESC[<u` pops keyboard mode
  - [x] Query keyboard mode responds with bitmask
  - [x] Pop from empty stack is noop
  - [x] Unknown sequences don't panic (CSI, OSC, ESC)
  - [x] Push keyboard mode 1 and mode 3 independently
  - [x] Query keyboard mode reports bitmask from stack top
  - [x] Query keyboard mode with empty stack reports zero
  - [x] Pop more than stack depth clamps to empty
  - [x] Keyboard mode stack survives alt screen swap
  - [x] RIS clears keyboard mode stack, resets flags, resets cursor shape, clears blinking

---

## 2.12 RenderableContent Snapshot

A lightweight struct that captures everything the renderer needs from `Term`, extracted under lock and used without lock.

**File:** `oriterm_core/src/term/renderable/mod.rs`, `oriterm_core/src/term/mod.rs`

- [x] `RenderableContent` struct
  - [x] Fields:
    - `cells: Vec<RenderableCell>` — flattened visible cells (row-by-row)
    - `cursor: RenderableCursor` — cursor position, shape, visibility
    - `display_offset: usize` — scrollback offset
    - `stable_row_base: u64` — stable row index of viewport line 0 (for `StableRowIndex` conversion)
    - `mode: TermMode` — terminal mode flags snapshot
    - `all_dirty: bool` — full redraw signal
    - `damage: Vec<DamageLine>` — which lines changed (empty when `all_dirty`)
  - [x] `Default` impl and `clear(&mut self)` — reuses allocated capacity
  - [x] `Term::renderable_content(&self) -> RenderableContent` — allocating convenience wrapper
  - [x] `Term::renderable_content_into(&self, out: &mut RenderableContent)` — reusable buffer path (hot-path renderer)
    - [x] Iterate visible rows (scrollback lines from display_offset, then grid lines)
    - [x] Resolve all colors via palette (fg, bg, underline_color)
    - [x] Apply bold-as-bright, dim, inverse
    - [x] Include cursor info (visible only when SHOW_CURSOR set and at live view)
    - [x] Collect damage info via `collect_damage()`
    - [x] Pure read — does NOT clear dirty state (caller must drain separately)
- [x] `RenderableCell` struct
  - [x] `line: usize`, `column: Column`, `ch: char`, `fg: Rgb`, `bg: Rgb`, `flags: CellFlags`
  - [x] `underline_color: Option<Rgb>`, `has_hyperlink: bool`, `zerowidth: Vec<char>`
  - [x] Colors are **fully resolved** (palette lookup, bold-as-bright, dim, inverse all applied)
- [x] `RenderableCursor` struct
  - [x] `line: usize`, `column: Column`, `shape: CursorShape`, `visible: bool`
- [x] `DamageLine` struct
  - [x] `line: usize`, `left: Column`, `right: Column`
- [x] `TermDamage` iterator — drains dirty lines from grid, yields `DamageLine`s
- [x] Color resolution helpers: `resolve_fg()`, `resolve_bg()`, `apply_inverse()`
  - [x] `resolve_fg`: DIM takes priority over BOLD (no bright promotion when dim)
  - [x] Bold-as-bright: ANSI 0-7 promoted to 8-15 for `Named` and `Indexed` colors
  - [x] Dim: Named uses `to_dim()`, Indexed/Spec uses `dim_rgb()` (2/3 brightness)
- [x] **Tests** (`renderable/tests.rs`) — 1575 lines, all pass (verified 2026-03-29):
  - [x] Written chars appear in cells, cursor position matches
  - [x] Default colors resolve to palette defaults
  - [x] SGR named/indexed/truecolor colors resolve correctly
  - [x] Bold-as-bright promotes ANSI 0-7 (not 8-15, not truecolor)
  - [x] Inverse swaps fg/bg
  - [x] Dim reduces brightness (Named, Indexed, truecolor)
  - [x] Bold+dim: dim takes priority (consistent across Named/Indexed)
  - [x] Underline color (truecolor, indexed) resolves, None when absent
  - [x] Wide chars produce two cells, combining marks propagate to zerowidth
  - [x] Alt screen snapshot reads alt grid
  - [x] Cursor shape variants (block, underline, bar) in snapshot
  - [x] Scrollback content visible when scrolled, preserves colors/flags
  - [x] All SGR flags preserved (bold, italic, dim, blink, inverse, hidden, strikethrough)
  - [x] Hyperlink presence tracked via `has_hyperlink`
  - [x] Damage tracking integration (write marks dirty, drain clears, scroll marks all)
  - [x] Empty term produces space cells with default colors
  - [x] Cell ordering is row-major
  - [x] Cursor hidden when shape is `Hidden`
  - [x] `resolve_fg` Spec passthrough, Indexed bold promotion (not above 7), dim Spec reduces
  - [x] `resolve_bg` passthrough
  - [x] `apply_inverse` noop without INVERSE flag
  - [x] Mode flags captured in snapshot
  - [x] Display offset zero in live view
  - [x] Fresh term reports all dirty
  - [x] Multiple writes same line produce single damage entry, different lines produce separate entries
  - [x] All SGR attributes combined in one cell
  - [x] Bold+inverse with named colors, dim+inverse with default colors
  - [x] Indexed+truecolor mixed in one line
  - [x] Empty cells have default everything
  - [x] ZWJ emoji sequence renderable cells, VS16 on ASCII, wide char with combining mark
  - [x] Wrap flag set at end of line
  - [x] Scrollback preserves bold flag
  - [x] Leaving alt screen restores primary grid content
  - [x] SGR reset in middle, `ESC[39m` / `ESC[49m` reset fg/bg independently, `ESC[39m` preserves bg

---

## 2.13 FairMutex

Prevents starvation between PTY reader thread and render thread. Ported from Alacritty.

**File:** `oriterm_core/src/sync/mod.rs`

**Reference:** Alacritty `alacritty_terminal/src/sync.rs` (FairMutex); Ghostty `src/Surface.zig` (3-thread model with mailboxes — different approach worth studying)

- [x] `FairMutex<T>` struct
  - [x] Fields:
    - `data: parking_lot::Mutex<T>` — the actual data
    - `next: parking_lot::Mutex<()>` — fairness gate (FIFO ordering)
    - `contended: AtomicBool` — set when `lock()` blocks on fairness gate
  - [x] `FairMutex::new(data: T) -> Self`
  - [x] `lock(&self) -> FairMutexGuard<'_, T>` — fair lock: acquire `next`, then `data`; sets `contended` if gate was held
  - [x] `lock_unfair(&self) -> MutexGuard<'_, T>` — bypass fairness gate (for PTY thread)
  - [x] `try_lock(&self) -> Option<MutexGuard<'_, T>>` — non-blocking try (bypasses fairness gate)
  - [x] `lease(&self) -> FairMutexLease<'_>` — reserve `next` lock (PTY reader holds during read+parse cycle)
  - [x] `take_contended(&self) -> bool` — check and clear contention flag (PTY reader uses to decide whether to yield)
- [x] `FairMutexGuard<'_, T>` — RAII guard holding both locks
  - [x] `unlock_fair(self)` — fair unlock via `MutexGuard::unlock_fair` (hands off directly to next waiter)
  - [x] `Deref` + `DerefMut` impls
- [x] `FairMutexLease<'_>` — RAII guard for the `next` lock only
- [x] **Tests** (`sync/tests.rs`) — 563 lines, all pass (verified 2026-03-29):
  - [x] Basic lock/unlock works
  - [x] Two threads can take turns locking
  - [x] `try_lock` returns None when data lock held, succeeds when unlocked
  - [x] Lease blocks fair lock
  - [x] `lock_unfair` bypasses fairness gate
  - [x] Guard `Deref`/`DerefMut` work
  - [x] `unlock_fair` releases data and hands off to waiter
  - [x] `unlock_fair` prevents starvation
  - [x] `take_contended` initially false, cleared after read, set on blocked lock
  - [x] `take_contended` not set on unblocked lock
  - [x] `take_contended` resets per contention event (second blocked lock re-sets flag)
  - [x] `unlock_fair` uncontested throughput benchmark
  - [x] Contention benchmarks comparing locking strategies

---

## 2.14 Damage Tracking Integration

Wire dirty tracking from Grid into the RenderableContent snapshot.

**File:** `oriterm_core/src/term/mod.rs`, `oriterm_core/src/term/renderable/mod.rs`

- [x] `Term::damage(&mut self) -> TermDamage<'_>`
  - [x] Returns dirty lines from active grid's DirtyTracker
  - [x] After reading damage, marks are cleared (drain semantics)
  - [x] `TermDamage::is_all_dirty()` — check before iterating
- [x] `Term::reset_damage(&mut self)` — discard pending damage without reading (drains and drops)
- [x] `RenderableContent` includes damage info via `collect_damage()`
  - [x] If `all_dirty`, damage list is empty (signals full redraw)
  - [x] Otherwise, damage list contains only changed lines
  - [x] Fast paths: explicit all-dirty flag, nothing-dirty shortcut
- [x] **Tests** (extensive coverage in `term/tests.rs`) — ~30 damage tests, all pass (verified 2026-03-29):
  - [x] Write char → line is damaged
  - [x] Drain damage → no longer damaged
  - [x] scroll_up → all lines dirty
  - [x] No changes → no damage
  - [x] Cursor movement (goto, forward, backward, up, down) marks lines dirty
  - [x] Carriage return, linefeed, backspace, tab — damage tracking
  - [x] Wrap to next line damages both lines
  - [x] Reverse index with scroll damages all lines
  - [x] Erase chars, delete chars, insert blank — damage tracking
  - [x] Clear line (all, right, left) — damage tracking
  - [x] Clear screen (below, above, all) — damage tracking
  - [x] Scroll up/down CSI — damage tracking
  - [x] Insert/delete lines — damage tracking
  - [x] Swap alt screen marks all dirty
  - [x] Palette set/reset color marks dirty (cursor color exempt)
  - [x] Resize marks all dirty
  - [x] Scroll display marks all dirty
  - [x] Wide char and combining mark writes mark line dirty

---

## 2.15 Section Completion

- [x] All 2.1–2.14 items complete (verified 2026-03-29)
- [x] `cargo test -p oriterm_core` — all tests pass (verified 2026-03-29 — 1429 passed, 0 failed, 2 ignored)
- [x] `cargo clippy -p oriterm_core --target x86_64-pc-windows-gnu` — no warnings
- [x] Feed `echo "hello world"` through Term<VoidListener> → correct grid state (verified 2026-03-29)
- [x] Feed CSI sequences (cursor move, erase, SGR) → correct results (verified 2026-03-29)
- [x] Feed OSC sequences (title, palette) → correct events fired (verified 2026-03-29)
- [x] Alt screen switch works correctly (verified 2026-03-29)
- [x] RenderableContent snapshot extracts correct data (verified 2026-03-29)
- [x] FairMutex compiles and basic tests pass (verified 2026-03-29)
- [x] No GPU, no PTY, no window — purely in-memory terminal emulation (verified 2026-03-29)

**Verification notes (2026-03-29):** All handler tests (5294 lines) verified. Protocol responses (DA1, DA2, DSR, DECRPM, CPR, clipboard, color query, text area size) match Alacritty byte-for-byte. Mouse mode mutual exclusion is more correct than Alacritty (clears all encoding modes when setting a new one). Zero TODOs, FIXMEs, or ignored tests. All files under 500 lines. Over 10,000 lines of tests across the section's modules.

**Exit Criteria:** Full VTE processing works in-memory. `Term<VoidListener>` can process any escape sequence and produce correct grid state. `RenderableContent` snapshots work.
