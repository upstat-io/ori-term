---
section: 8
title: Keyboard Input
status: complete
reviewed: true
last_verified: "2026-04-01"
tier: 3
goal: Legacy + Kitty keyboard encoding, keyboard dispatch, IME support
third_party_review:
  status: none
  updated: null
sections:
  - id: "8.1"
    title: Legacy Key Encoding
    status: complete
  - id: "8.2"
    title: Kitty Keyboard Protocol
    status: complete
  - id: "8.3"
    title: Keyboard Input Dispatch
    status: complete
  - id: "8.4"
    title: Section Completion
    status: complete
---

# Section 08: Keyboard Input

**Status:** Complete (two Kitty protocol items reopened during verification 2026-03-29)
**Goal:** Encode all key events correctly for terminal applications using both legacy xterm/VT sequences and the Kitty keyboard protocol. Wire keyboard dispatch through keybindings to PTY output with IME support for CJK input.

**Crate:** `oriterm` (binary)
**Dependencies:** `winit` (key events), `oriterm_core` (TermMode)
**Reference:** `_old/src/key_encoding/legacy.rs`, `_old/src/key_encoding/kitty.rs`, `_old/src/app/input_keyboard.rs`

**Prerequisite:** Section 03 complete (PTY running, event loop accepting input). Section 02 complete (TermMode flags available for mode-dependent encoding).

---

## 8.1 Legacy Key Encoding (verified 2026-03-29 -- protocol-correct, byte-for-byte match with Alacritty/Kitty)

Correctly encode all key events for the terminal using legacy xterm/VT sequences. This is the baseline encoding used when Kitty protocol is not active.

**File:** `oriterm/src/key_encoding/legacy.rs`

**Reference:** `_old/src/key_encoding/legacy.rs` — carries forward the proven LetterKey/TildeKey dispatch tables.

- [x] `encode_legacy(key: &KeyEvent, mode: TermMode) -> Option<Vec<u8>>`
  - [x] Main entry point: inspects key, modifiers, and terminal mode to produce the byte sequence
  - [x] Returns `None` for keys that should not be sent (e.g., bare modifier keys)
- [x] **Regular text input**:
  - [x] Printable characters: send UTF-8 bytes directly
  - [x] Enter: send `\r` (or `\r\n` if LINEFEED_MODE active)
  - [x] Backspace: send `\x7f` (DEL) by default, `\x08` (BS) if DECBKM active
  - [x] Tab: send `\t`; Shift+Tab (backtab): send `ESC[Z`
  - [x] Escape: send `\x1b`
  - [x] Space: send `\x20`; Ctrl+Space: send `\x00` (NUL)
- [x] **Arrow keys** (mode-dependent DECCKM):
  - [x] Normal mode: `ESC[A` (Up), `ESC[B` (Down), `ESC[C` (Right), `ESC[D` (Left)
  - [x] Application cursor mode (DECCKM): `ESCOA`, `ESCOB`, `ESCOC`, `ESCOD`
  - [x] Modifiers override SS3 to CSI format: Ctrl+Up = `ESC[1;5A` (even in app mode)
- [x] **Home/End**:
  - [x] Normal: `ESC[H` / `ESC[F`
  - [x] Application cursor mode: `ESCOH` / `ESCOF`
  - [x] With modifiers: `ESC[1;{mod}H` / `ESC[1;{mod}F`
- [x] **PageUp/PageDown, Insert/Delete**:
  - [x] PageUp: `ESC[5~`, PageDown: `ESC[6~`
  - [x] Insert: `ESC[2~`, Delete: `ESC[3~`
  - [x] With modifiers: `ESC[5;{mod}~` etc.
- [x] **Function keys F1-F12**:
  - [x] F1-F4 use SS3: `ESCOP`, `ESCOQ`, `ESCOR`, `ESCOS`
  - [x] F5-F12 use tilde: `ESC[15~`, `ESC[17~`, `ESC[18~`, `ESC[19~`, `ESC[20~`, `ESC[21~`, `ESC[23~`, `ESC[24~`
  - [x] With modifiers: F1 = `ESC[1;{mod}P`, F5 = `ESC[15;{mod}~`
- [x] **Ctrl+letter** — send C0 control codes:
  - [x] Ctrl+A = `\x01`, Ctrl+B = `\x02`, ..., Ctrl+Z = `\x1A`
  - [x] Ctrl+[ = `\x1b` (ESC), Ctrl+] = `\x1d`, Ctrl+\ = `\x1c`
  - [x] Ctrl+/ = `\x1f`, Ctrl+@ = `\x00`
- [x] **Alt+key** — ESC prefix:
  - [x] Alt+a = `\x1b a`, Alt+A = `\x1b A`
  - [x] Alt+Backspace = `\x1b \x7f`
  - [x] Alt+Ctrl combinations: ESC prefix + C0 byte (Alt+Ctrl+A = `\x1b \x01`)
- [x] **Modifier parameter encoding** (for named keys with modifiers):
  - [x] Parameter = `1 + modifier_bits` where Shift=1, Alt=2, Ctrl=4, Super=8
  - [x] Example: Ctrl+Shift+Up = `ESC[1;6A` (1 + 1 + 4 = 6)
- [x] **Application keypad mode** (DECKPAM/DECKPNM):
  - [x] Normal: numpad keys send their character values
  - [x] Application: numpad sends `ESCOp` through `ESCOy`, Enter = `ESCOM`, operators `ESCOk/m/j/n`
- [x] Helper structs:
  - [x] `LetterKey { term: u8, needs_app_cursor: bool }` — named key with letter terminator
  - [x] `TildeKey { num: u8 }` — named key with tilde terminator
  - [x] `fn letter_key(key: NamedKey) -> Option<LetterKey>` — lookup table
  - [x] `fn tilde_key(key: NamedKey) -> Option<TildeKey>` — lookup table
  - [x] `fn ctrl_key_byte(key: &Key) -> Option<u8>` — Ctrl+key to C0 byte
- [x] **Tests** (`oriterm/src/key_encoding/tests.rs`):
  - [x] Arrow Up in normal mode produces `ESC[A`
  - [x] Arrow Up in application cursor mode produces `ESCOA`
  - [x] Ctrl+Up produces `ESC[1;5A` (modifier parameter)
  - [x] Ctrl+C produces `\x03`
  - [x] Ctrl+A produces `\x01`
  - [x] Alt+A produces `\x1b A`
  - [x] Alt+Backspace produces `\x1b \x7f`
  - [x] Shift+Tab produces `ESC[Z`
  - [x] Shift+F5 produces `ESC[15;2~`
  - [x] Home in normal mode produces `ESC[H`
  - [x] Home in application cursor mode produces `ESCOH`
  - [x] F1 produces `ESCOP`, F5 produces `ESC[15~`
  - [x] Enter produces `\r`
  - [x] Numpad in application keypad mode sends ESC O sequences

---

## 8.2 Kitty Keyboard Protocol (verified 2026-03-29 -- conditional pass, two protocol issues noted below)

Progressive enhancement keyboard protocol for modern terminal applications. Encodes keys in CSI u format with mode-dependent behavior.

**File:** `oriterm/src/key_encoding/kitty.rs`

**Reference:** `_old/src/key_encoding/kitty.rs`, Kitty keyboard protocol specification (https://sw.kovidgoyal.net/kitty/keyboard-protocol/), Ghostty `src/input/key_encode.zig` (Kitty + legacy encoding), Alacritty `alacritty_terminal/src/term/mod.rs` (key input handling)

- [x] `encode_kitty(input: &KeyInput) -> Vec<u8>`
  - [x] Main entry point: encodes key events using CSI u format
  - [x] Reads mode flags from `TermMode` bitflags
  - [x] Returns empty `Vec` for plain printable chars without modifiers when `REPORT_ALL_KEYS` inactive
- [x] **Mode flags** (5 progressive enhancement levels):
  - [x] Bit 0 — `DISAMBIGUATE_ESC_CODES` (1): use CSI u for ambiguous keys
  - [x] Bit 1 — `REPORT_EVENT_TYPES` (2): report press/repeat/release
  - [x] Bit 2 — `REPORT_ALTERNATE_KEYS` (4): report shifted/base key variants *(implemented in 8aac6e5: `encode_kitty()` reads the flag, resolves `alternate_key` from `physical_key_to_us_codepoint()`, and includes it in CSI u output as `base::alternate`. 3 dedicated tests.)*
  - [x] Bit 3 — `REPORT_ALL_KEYS_AS_ESC` (8): encode all keys including plain text as CSI u
  - [x] Bit 4 — `REPORT_ASSOCIATED_TEXT` (16): report text generated by key
- [x] **CSI u encoding format**: `ESC [ keycode ; modifiers u`
  - [x] Extended: `ESC [ keycode ; modifiers : event_type u`
  - [x] With text: `ESC [ keycode ; modifiers : event_type ; text u`
- [x] **Keycode mapping** (`fn kitty_codepoint(key: NamedKey) -> Option<u32>`):
  - [x] Escape=27, Enter=13, Tab=9, Backspace=127
  - [x] Insert=57348, Delete=57349, Left=57350, Right=57351, Up=57352, Down=57353
  - [x] PageUp=57354, PageDown=57355, Home=57356, End=57357
  - [x] F1=57364 through F35=57398
  - [x] CapsLock=57358, ScrollLock=57359, NumLock=57360
  - [x] Character keys: use Unicode codepoint directly
- [x] **Modifier encoding**:
  - [x] Modifier parameter = `1 + bits` where Shift=1, Alt=2, Ctrl=4, Super=8
  - [x] Omit modifier parameter if value is 1 (no modifiers) and no event type needed
- [x] **Event types** (when `REPORT_EVENT_TYPES` active):
  - [x] 1 = press (omitted as default when `REPORT_EVENT_TYPES` not active)
  - [x] 2 = repeat
  - [x] 3 = release
  - [x] Format: `ESC [ keycode ; modifiers : event_type u`
  - [x] Key release events pass through app shortcuts to PTY when `REPORT_EVENT_TYPES` active
- [x] **Mode stack management** (wired through VTE Handler trait):
  - [x] `push_keyboard_mode(mode)` — push onto stack, apply
  - [x] `pop_keyboard_modes(n)` — pop n entries, apply top or clear
  - [x] `set_keyboard_mode(mode, behavior)` — Replace/Union/Difference on top
  - [x] `report_keyboard_mode()` — respond `ESC[?{bits}u`
  - [x] Stack save/restore on alt screen switch
  - [x] Stack clear on terminal reset
- [x] **Protocol divergence: universal `u` terminator** *(added 2026-03-29, fixed 2026-04-01)*: Keys with legacy CSI sequences now use their traditional terminators in Kitty mode (e.g., ArrowUp → `ESC[1A`, F1 → `ESC[1P`, Insert → `ESC[2~`), matching Kitty and Alacritty output. Added `legacy_csi_info()` lookup and refactored `build_csi_sequence()` to select the correct terminator. 6 existing tests updated, 3 new tilde-terminator tests added.

- [x] **Tests** (`oriterm/src/key_encoding/tests.rs`): (verified 2026-03-29 -- 151 tests pass)
  - [x] `'a'` with mode 1 (disambiguate): plain `a` (no encoding needed, not ambiguous)
  - [x] Ctrl+A with mode 1: `ESC[97;5u` (codepoint 97, modifier 5)
  - [x] Enter with mode 1: `ESC[13u` (disambiguated from legacy)
  - [x] Escape with mode 1: `ESC[27u`
  - [x] Key release with mode 2: `ESC[97;1:3u` (event type 3)
  - [x] Key repeat with mode 2: `ESC[97;1:2u`
  - [x] `'a'` with mode 8 (report all): `ESC[97u`
  - [x] F1 with mode 1: `ESC[57364u`
  - [x] Shift+A with mode 1: `ESC[65;2u`

---

## 8.3 Keyboard Input Dispatch (verified 2026-03-29 -- 33 tests pass, clean decision tree)

Route keyboard events through keybindings, then through key encoding, then to the PTY. Single decision tree: each input event handled by exactly one handler.

**File:** `oriterm/src/app/mod.rs` (keyboard dispatch in `handle_keyboard_input`)

**Reference:** `_old/src/app/input_keyboard.rs`

- [x] `handle_keyboard_input(&mut self, event: &KeyEvent)`
  - [x] Main entry point called from the winit event loop on `WindowEvent::KeyboardInput`
- [x] **Dispatch priority** (first match wins):
  1. [x] Check keybindings table: if key+modifiers match a bound action, execute the action and return
  2. [x] Check Kitty keyboard mode on active tab:
     - [x] Read `keyboard_mode_stack` from active tab's terminal state
     - [x] If Kitty mode active: call `encode_kitty()`, send result to PTY
     - [x] If REPORT_EVENT_TYPES active: also send release events
  3. [x] Fall through to legacy encoding:
     - [x] Call `encode_legacy()`, send result to PTY
  4. [x] If encoding returns None: key not handled (bare modifier press, etc.)
- [x] **Cursor blink reset**:
  - [x] On any keypress that sends to PTY: reset cursor blink timer (cursor becomes visible)
- [x] **Scroll to bottom on input**:
  - [x] If display_offset > 0 (viewing scrollback): scroll to live position on keypress
- [x] **Smart Ctrl+C**:
  - [x] If selection exists and Ctrl+C pressed: copy selection to clipboard, do NOT send SIGINT
  - [x] If no selection and Ctrl+C pressed: send `\x03` to PTY
- [x] **IME handling** (`WindowEvent::Ime`):
  - [x] `Ime::Commit(text)`: send committed text bytes to PTY
  - [x] `Ime::Preedit(text, cursor)`: display composition text at cursor position (overlay rendering)
  - [x] `Ime::Enabled` / `Ime::Disabled`: track IME state, suppress raw key events during composition
  - [x] Position IME candidate window near terminal cursor (call `window.set_ime_cursor_area()`)
  - [x] Don't send raw key events to PTY during active IME preedit
- [x] **Tests** (`oriterm/src/app/keyboard_input/tests.rs`):
  - [x] Keybinding takes priority over PTY send
  - [x] Kitty mode takes priority over legacy encoding
  - [x] Ctrl+C with selection copies, without selection sends `\x03`
  - [x] IME commit sends text to PTY
  - [x] Preedit overlay replaces cells at cursor with underline
  - [x] Preedit hides terminal cursor
  - [x] Wide (CJK) preedit chars set WIDE_CHAR flags
  - [x] Preedit clips at grid edge

---

## 8.4 Section Completion (verified 2026-03-29 -- 196 total tests pass)

- [x] All 8.1-8.3 items complete *(reopened 2026-03-29, completed 2026-04-01)*
- [x] `cargo test -p oriterm --target x86_64-pc-windows-gnu` — key encoding tests pass (verified 2026-03-29 -- 151 key_encoding + 33 keyboard_input + 12 keyboard_mode = 196 tests)
- [x] `cargo clippy -p oriterm --target x86_64-pc-windows-gnu` — no warnings (verified 2026-03-29)
- [x] All printable characters encoded correctly (verified 2026-03-29)
- [x] Arrow keys work in both normal and application cursor modes (verified 2026-03-29)
- [x] F1-F12 function keys produce correct sequences (verified 2026-03-29)
- [x] Ctrl+letter sends correct C0 control codes (verified 2026-03-29)
- [x] Alt+key sends ESC prefix correctly (verified 2026-03-29)
- [x] Modifier combinations on special keys produce correct parameter encoding (verified 2026-03-29)
- [x] Numpad keys work in both normal and application keypad modes (verified 2026-03-29)
- [x] Kitty keyboard protocol level 1+ supported (all 5 mode flags) *(reopened 2026-03-29, completed 2026-04-01: REPORT_ALTERNATE_KEYS now wired — `physical_key_to_us_codepoint()` maps physical keys to US layout, included as `base::alternate` in CSI u sequences)*
- [x] Key release/repeat events reported when REPORT_EVENT_TYPES active (verified 2026-03-29)
- [x] Keybinding dispatch has priority over PTY encoding (verified 2026-03-29)
- [x] IME commit text reaches PTY (verified 2026-03-29)
- [x] Smart Ctrl+C works (copy if selection, SIGINT if not) (verified 2026-03-29)

> **Verification summary (2026-03-29):** Section is functionally complete for all practical use. Two Kitty protocol issues identified:
> 1. **Universal `u` terminator (LOW RISK):** All keys use `u` terminator instead of legacy terminators where spec prefers them (e.g., ArrowUp sends `ESC[57352u` instead of `ESC[A`). Both valid per spec.
> 2. **REPORT_ALTERNATE_KEYS not implemented (MEDIUM RISK):** Mode flag 4 is stored/pushed/popped correctly but `encode_kitty()` never reads it. Apps like neovim with kitty protocol won't receive alternate key info.

**Exit Criteria:** All standard terminal applications receive correct key input. vim, tmux, htop, and other apps work with correct modifier handling. Kitty protocol apps (e.g., kitty-based tools) receive properly encoded events.
