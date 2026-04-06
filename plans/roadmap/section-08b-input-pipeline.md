---
section: "08B"
title: "Input Event Normalization + Keyboard Encoding Pipeline"
status: not-started
reviewed: true
tier: 3
goal: "Normalized keyboard event pipeline that decouples platform capture, app routing, and PTY encoding — fixing numpad, Shift+letter, and key repeat bugs across legacy and Kitty modes."
success_criteria:
  - "Numpad digits 0-9, operators (+,-,*,/,.), Enter produce correct output in both APP_KEYPAD and normal mode"
  - "Shift+any letter produces uppercase in both legacy and Kitty modes, even when winit text is None"
  - "Holding Backspace (and any key) generates continuous output at OS repeat rate"
  - "App-level keybinding actions (CloseTab, SplitRight, etc.) do NOT auto-repeat on held keys"
  - "All existing key encoding tests pass (legacy unchanged; Kitty Shift+letter updated for spec-correct unshifted codepoints)"
  - "New tests cover each bug scenario with matrix dimensions"
  - "NormalizedKeyEvent fields are all Lua-serializable types (no opaque handles)"
  - "Dispatch chain has a documented extension point for Lua key event interception"
  - "Cross-platform: normalization produces identical NormalizedKeyEvent for the same logical key on all three platforms (macOS, Windows, Linux), with text resolution fallbacks covering platform-specific winit gaps"
  - "`./test-all.sh` green, `./build-all.sh` green, `./clippy-all.sh` green"
inspired_by:
  - "Ghostty src/input/key.zig (KeyEvent with consumed_mods, unshifted_codepoint, effectiveMods)"
  - "Ghostty src/input/key_encode.zig (encoding dispatches over normalized event)"
  - "Alacritty alacritty/src/input/keyboard.rs (priority-based SequenceBuilder try_build_* chain)"
  - "WezTerm termwiz/src/input.rs (normalize_shift_to_upper_case, KeyCode.encode)"
depends_on: ["08", "13"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "08B.1"
    title: "Platform Key Event Diagnostic"
    status: not-started
  - id: "08B.2"
    title: "NormalizedKeyEvent Model + Text Resolution"
    status: not-started
  - id: "08B.3"
    title: "Encoding Pipeline Rewrite"
    status: not-started
  - id: "08B.4"
    title: "Dispatch Integration + Action Repeat Policy"
    status: not-started
  - id: "08B.5"
    title: "Alt+Non-ASCII Key Encoding"
    status: not-started
  - id: "08B.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "08B.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 08B: Input Event Normalization + Keyboard Encoding Pipeline

**Status:** Not Started
**Goal:** Build a normalized keyboard event pipeline that decouples platform event capture, app routing, binding lookup, and PTY encoding — fixing missing text, numpad behavior, and repeat handling across legacy and Kitty input modes on all three platforms.

**Success Criteria:**

- [ ] Numpad digits 0-9, operators, Enter produce output in both APP_KEYPAD and normal mode — satisfies mission criterion "numpad works"
- [ ] Shift+any letter produces uppercase in legacy and Kitty modes even when winit `text` is `None` — satisfies mission criterion "Shift works"
- [ ] Holding Backspace generates continuous output at OS repeat rate — satisfies mission criterion "repeat works"
- [ ] CloseTab, SplitRight, and other destructive keybinding actions do NOT fire on repeat events — satisfies mission criterion "actions don't auto-repeat"
- [ ] `NormalizedKeyEvent` fields map to Lua-compatible types (strings, numbers, booleans) — satisfies mission criterion "Lua-ready"
- [ ] Encoding pipeline is a standalone `encode_normalized()` function callable without dispatch state — satisfies mission criterion "Lua-callable encoding"
- [ ] All existing key encoding tests pass (legacy tests unchanged; Kitty Shift+letter tests updated for spec-correct unshifted codepoints)
- [ ] New tests cover each bug scenario with explicit matrix dimensions (key type x modifier x encoding path)
- [ ] Dispatch chain has a documented extension point for Lua key event interception (FUTURE comment in 08B.4)
- [ ] Cross-platform: text resolution fallbacks cover all three platforms — `key_without_modifiers()` may return the logical key unchanged on some platforms, so the fallback chain (lowercase derivation, US layout mapping) must not depend on it being accurate
- [ ] `./test-all.sh` green, `./build-all.sh` green, `./clippy-all.sh` green

**Context:** Section 08 (Keyboard Input) was marked complete but has three runtime bugs discovered during real usage: (1) numpad keys produce no output when APP_KEYPAD mode is not set, (2) Shift+letter produces nothing when winit doesn't provide the `text` field, and (3) holding Backspace only registers one keypress. All three bugs stem from the same root cause: the encoding pipeline depends too heavily on winit's `text` field with no fallback, and the event model lacks the richness needed for robust cross-platform input. The restructuring introduces a normalization layer (inspired by Ghostty's `KeyEvent`) that resolves text, tracks consumed modifiers, and feeds a priority-based encoding chain (inspired by Alacritty's `SequenceBuilder`). The architecture is designed to integrate seamlessly with the planned Lua runtime (see `project_lua_runtime_plan.md`) — `NormalizedKeyEvent` will become a Lua `UserData`, and the dispatch chain has a documented hook point for Lua key event interception.

**Reference implementations:**
- **Ghostty** `src/input/key.zig`: `KeyEvent` struct with `consumed_mods`, `unshifted_codepoint`, `effectiveMods()` method that subtracts consumed mods from active mods. The gold standard for event normalization.
- **Ghostty** `src/input/key_encode.zig`: Encoding dispatches over the normalized event — `if (kitty_flags) kitty() else legacy()`. Clean separation.
- **Alacritty** `alacritty/src/input/keyboard.rs`: `SequenceBuilder` with `try_build_numpad()`, `try_build_named_kitty()`, `try_build_named_normal()`, `try_build_control_char_or_mod()`, `try_build_textual()` priority chain. Each step returns `Option<SequenceBase>`, chained with `.or_else()`.
- **WezTerm** `termwiz/src/input.rs`: `normalize_shift_to_upper_case()` strips Shift from modifiers when the key is already uppercase, preventing double-shifting in encoding.

**Depends on:** Section 08 (existing encoding infrastructure, test suite), Section 13 (keybinding system — `Action` enum, `find_binding()`, `BindingKey` used in 08B.4).

**Downstream consumers:**
- **Section 49 (Advanced Keybinding System):** Key tables and key remapping (49.4) insert into the dispatch chain. Remapping occurs BEFORE normalization (transforms the raw winit event). Key table dispatch occurs AFTER normalization, between keybinding lookup and PTY encoding. The `NormalizedKeyEvent` model and dispatch chain architecture from 08B must accommodate these extension points — 08B establishes the pipeline, 49 extends it.
- **Section 28 (Extensibility / Lua Runtime):** The Lua key event callback sits between normalization and keybinding dispatch (documented in 08B.4). `NormalizedKeyEvent` becomes a Lua `UserData`. `encode_normalized()` is callable from Lua scripts for custom key sequences.

---

## 08B.1 Platform Key Event Diagnostic

**File(s):** `oriterm/src/app/keyboard_input/mod.rs` (temporary instrumentation, removed after diagnosis)

Before locking the design, prove all three bugs with real platform data. The hypothesis for bugs 1 and 2 (numpad, Shift) is strong — the code path is clear in `legacy.rs:247` (`text.map_or_else(Vec::new, ...)`). Bug 3 (repeat) needs platform verification — the code path appears correct if winit generates repeat events.

- [ ] Add temporary `log::info!` instrumentation to `encode_key_to_pty()` that logs for every key event:
  - `event.logical_key`, `event.physical_key`, `event.text`, `event.repeat`, `event.state`, `event.location`
  - `self.modifiers` (the tracked modifier state)
  - The `KeyInput` struct fields passed to `encode_key()`
  - The resulting byte vector (hex-formatted)
  - Use `log::info!` (not `log::debug!` — user doesn't set `RUST_LOG=debug`)
- [ ] Build and test on Windows (the primary development platform via WSL2 cross-compile):
  - **Numpad test**: Press numpad 0-9 with NumLock on. Record what winit reports for `logical_key`, `text`, `location`. Verify hypothesis: `text` is `None` OR `location` is not `Numpad`.
  - **Shift test**: Press Shift+A, Shift+Z, Shift+1 (should produce `!`). Record `logical_key`, `text`, modifier state. Verify hypothesis: `text` is `None` when Shift is held.
  - **Repeat test**: Hold Backspace for 3 seconds. Count how many `KeyboardInput` events winit fires. Check `event.repeat` values. This determines whether the bug is in winit (no repeat events) or in our dispatch (events generated but swallowed).
- [ ] Document findings in a comment block at the top of 08B.2 implementation (what winit actually sends per platform). Remove instrumentation after diagnosis.
- [ ] If bug 3 is winit-level (no repeat events generated), file a winit issue reference and implement a software repeat fallback. If bug 3 is dispatch-level, the fix goes in 08B.4.

**Validation:** Run `./build-all.sh` with instrumentation, launch binary, test all three scenarios, capture log output. Instrumentation is temporary — removed before 08B.2 begins.

---

## 08B.2 NormalizedKeyEvent Model + Text Resolution

**File(s):** `oriterm/src/key_encoding/normalized/mod.rs` (new — under 500 lines), `oriterm/src/key_encoding/normalized/tests.rs` (sibling tests.rs pattern), `oriterm/src/key_encoding/text_resolve.rs` (new — ~80 lines, no separate tests.rs; tested via `normalized/tests.rs` through the public `from_winit()` and `resolve_text()` API)

The core abstraction that fixes all three bugs. A `NormalizedKeyEvent` is constructed from a raw winit `KeyEvent` + tracked modifier state + terminal mode. It resolves text eagerly — printable keys ALWAYS have resolved text, never `None`. Both keybinding lookup and PTY encoding consume the same normalized event.

- [ ] **Add `mod` declarations** in `oriterm/src/key_encoding/mod.rs`: add `mod normalized;` and `mod text_resolve;` alongside existing `mod kitty;` / `mod legacy;`. Add `pub(crate) use normalized::NormalizedKeyEvent;` re-export.
- [ ] Define `NormalizedKeyEvent` struct:
  ```rust
  /// Normalized keyboard event — platform-independent, text-resolved.
  ///
  /// Constructed from winit's `KeyEvent` via `from_winit()`. The `text` field
  /// is ALWAYS populated for printable keys — if winit didn't provide it, the
  /// normalization layer derives it from the key + modifiers.
  ///
  /// All fields are Lua-serializable types (no opaque handles, no lifetimes
  /// that prevent conversion). When the Lua runtime lands, this becomes a
  /// `UserData` that Lua callbacks can inspect and consume.
  pub struct NormalizedKeyEvent {
      /// The logical key identifier (named or character).
      pub key: NormalizedKey,
      /// Physical key location (Standard, Left, Right, Numpad).
      pub location: KeyLocation,
      /// Press, repeat, or release.
      pub action: KeyAction,
      /// All modifier keys currently held.
      pub mods: Modifiers,
      /// Modifiers consumed by keyboard layout transformation.
      /// Shift is consumed when it produces an uppercase letter.
      /// AltGr is consumed when it produces a special character.
      pub consumed_mods: Modifiers,
      /// Resolved text — NEVER None for printable keys.
      /// For named keys (arrows, F-keys, etc.), this is None.
      /// `String` (not `SmolStr` or `Cow`) for Lua serialization simplicity.
      /// Allocation is per-key-event (~30/sec max), not per-cell — acceptable.
      pub text: Option<String>,
      /// Codepoint of the key without Shift applied.
      /// For 'A' with Shift held, this is 'a' (97).
      pub unshifted_codepoint: Option<u32>,
      /// US keyboard layout codepoint (for Kitty REPORT_ALTERNATE_KEYS).
      pub alternate_key: Option<u32>,
      /// Current terminal mode flags (for encoding decisions).
      pub mode: TermMode,
  }
  ```
- [ ] Define `NormalizedKey` enum (replacing raw winit `Key` dependency in encoding):
  ```rust
  /// Key identifier for encoding dispatch.
  ///
  /// Reuses `winit::keyboard::NamedKey` for the Named variant — this is a
  /// deliberate pragmatic choice. Defining a custom 50+ variant named key
  /// enum would be massive boilerplate for no near-term benefit. For Lua
  /// serialization, Named keys convert to their string name (e.g., "ArrowUp").
  pub enum NormalizedKey {
      /// Named functional key (arrow, F-key, Home, etc.).
      Named(NamedKey),
      /// Character key with its base (unshifted) codepoint.
      Character(u32),
      /// Dead key or unidentified — carries no semantic info.
      Unidentified,
  }
  ```
- [ ] Define `KeyAction` enum (replacing `KeyEventType` — clearer naming):
  ```rust
  pub enum KeyAction {
      Press,
      Repeat,
      Release,
  }
  ```
- [ ] Implement `NormalizedKeyEvent::from_winit()`:
  ```rust
  pub fn from_winit(
      event: &winit::event::KeyEvent,
      tracked_mods: winit::keyboard::ModifiersState,
      mode: TermMode,
  ) -> Self
  ```
  This is where text resolution happens:
  - [ ] **Text resolution priority chain** (call `resolve_text()`):
    1. If Ctrl is held and key is a letter → return `None` (Ctrl+letter produces C0 control codes, not text — encoding handles this in `try_encode_control()`)
    2. Use `event.text` if provided by winit (most common case)
    3. If `text` is None and key is `Character(s)`: derive from the character + modifiers
       - Shift+lowercase letter → uppercase (`'a'` + Shift → `"A"`)
       - Numpad digit → character value (`Character("5")` at `Numpad` location → `"5"`)
    4. If key is `Named` → text is `None` (named keys don't produce text)
  - [ ] **Consumed mods computation** (conservative, per Codex advice):
    - If Shift is held AND the resolved text differs from the unshifted key (e.g., `'a'` → `'A'`), Shift is consumed
    - If the key produces text that doesn't match any simple modifier transformation, all mods that could contribute are consumed
    - Don't block on perfect AltGr/dead-key inference — start conservative
  - [ ] **Unshifted codepoint**: extract from `event.key_without_modifiers()` (requires `use winit::platform::modifier_supplement::KeyEventExtModifierSupplement` extension trait). This trait is behind `#[cfg(any(windows_platform, macos_platform, x11_platform, wayland_platform, orbital_platform, docsrs))]` in winit 0.30 — all three ori_term target platforms are covered, so no additional `cfg` gates are needed in our code. On some platforms it may return the same value as `logical_key` (no modifier stripping). The fallback chain MUST NOT depend on `key_without_modifiers()` being accurate: if the result is `Key::Dead`, `Key::Unidentified`, or identical to the shifted logical key when Shift is held, fall back to lowercasing the character key. This ensures cross-platform correctness.
  - [ ] **Alternate key**: delegate to existing `physical_key_to_us_codepoint()`
  - [ ] **Action mapping**: `(ElementState::Pressed, repeat=false)` → `Press`, `(Pressed, true)` → `Repeat`, `(Released, _)` → `Release`
- [ ] Implement `effective_mods()` method (Ghostty pattern): `self.mods.difference(self.consumed_mods)` — returns only semantically active modifiers
- [ ] Add `resolve_text()` as a standalone function in `oriterm/src/key_encoding/text_resolve.rs` (new file, ~80 lines):
  ```rust
  /// Resolve the text a key event should produce.
  ///
  /// Priority: winit text → derived from key+mods → None (for named keys).
  /// Printable character keys ALWAYS return Some. Named keys ALWAYS return None.
  pub fn resolve_text(
      key: &winit::keyboard::Key,  // Raw winit Key — called BEFORE NormalizedKey construction.
      winit_text: Option<&str>,
      mods: Modifiers,
      location: KeyLocation,
  ) -> Option<String>
  ```
  - [ ] Shift+lowercase ASCII letter → uppercase: `(b'a'..=b'z')` → `to_ascii_uppercase()`
  - [ ] Shift+digit → symbol (US layout): `'1'`→`'!'`, `'2'`→`'@'`, etc. (fallback only when winit text is None)
  - [ ] Numpad Character keys always resolve to their digit/operator character
  - [ ] Non-ASCII characters: if winit text is None and we can't derive, return the character as-is (best effort)
  - [ ] Dead keys (`Key::Dead`): return `None` — dead key composition is handled by IME, not by text resolution
  - [ ] AltGr combinations: if winit provides text via AltGr composition, use it; otherwise return `None` and let the key pass through without text (AltGr produces platform-specific characters that cannot be reliably derived)
- [ ] **Tests FIRST** in `oriterm/src/key_encoding/normalized/tests.rs` (sibling `tests.rs` pattern). Write these immediately after defining the struct/enum skeletons and `from_winit()` signature (before implementing `from_winit()` body). They should fail initially, then pass as implementation proceeds:
  - [ ] `from_winit()` with Shift+A, winit text=Some("A") → text="A", consumed_mods=SHIFT, unshifted=97
  - [ ] `from_winit()` with Shift+A, winit text=None → text="A" (derived), consumed_mods=SHIFT
  - [ ] `from_winit()` with numpad "5", winit text=None → text="5" (derived from character key)
  - [ ] `from_winit()` with numpad "5", winit text=Some("5") → text="5" (winit provided)
  - [ ] `from_winit()` with Backspace → text=None (named key), action derived correctly
  - [ ] `from_winit()` with repeat=true → action=Repeat
  - [ ] `effective_mods()` with Shift consumed → returns mods without Shift
  - [ ] `resolve_text()` with Shift+"1" → "!" (derived when winit text missing)
  - [ ] `resolve_text()` with Ctrl+A → None (Ctrl+letter is a control code, not text — encoding handles it)
  - [ ] `from_winit()` with dead key → text=None, key=Unidentified (dead key composition deferred to IME)
  - [ ] `from_winit()` with AltGr+"e" (winit text=Some("€")) → text="€" (AltGr-produced text preserved)
  - [ ] `from_winit()` with AltGr+"e" (winit text=None) → text=None (cannot derive AltGr compositions)
  - [ ] `key_without_modifiers()` returning same as logical_key → fallback to lowercase derivation

**Matrix dimensions:**
- Key types: ASCII letter, digit, punctuation, numpad digit, numpad operator, named key (arrow, F-key, Backspace), dead key, unidentified
- Modifier combos: none, Shift, Ctrl, Alt, Ctrl+Shift, Ctrl+Alt, Shift+Alt, AltGr (Right Alt on international layouts)
- Text source: winit-provided, derived (Shift fallback), derived (numpad), None (named key), None (dead key)
- Action: Press, Repeat, Release

**Semantic pin:** Test that `NormalizedKeyEvent::from_winit()` with `text=None` + `Shift` + `Character("a")` produces `text=Some("A")`. This ONLY passes with the new text resolution — the old code would produce empty bytes.

- [ ] Text resolution tests exercise the fallback chain independently of `key_without_modifiers()` accuracy — tests construct events where `key_without_modifiers()` returns the same as `logical_key` and verify that lowercase derivation still produces correct `unshifted_codepoint`
- [ ] `./build-all.sh` green, `./clippy-all.sh` green
- [ ] `./test-all.sh` green (existing tests unaffected — new type is additive)

---

## 08B.3 Encoding Pipeline Rewrite

**File(s):** `oriterm/src/key_encoding/mod.rs` (rewrite `encode_key`), `oriterm/src/key_encoding/legacy.rs` (refactor into try_build functions), `oriterm/src/key_encoding/kitty.rs` (consume `NormalizedKeyEvent`), `oriterm/src/key_encoding/win32.rs` (migrate to `NormalizedKeyEvent`)

Replace the current `encode_key(&KeyInput)` with `encode_normalized(&NormalizedKeyEvent)` using a priority-based chain. The old `KeyInput` type and `encode_key()` function are removed — all callers migrate to `NormalizedKeyEvent`.

This subsection uses the `NormalizedKeyEvent` defined in Section 08B.2.

- [ ] Rewrite `encode_key()` → `encode_normalized()` in `key_encoding/mod.rs`:
  ```rust
  /// Encode a normalized key event into bytes for the PTY.
  ///
  /// Priority chain (first match wins):
  /// 1. Kitty protocol (when any KITTY_KEYBOARD_PROTOCOL flag set)
  /// 2. Release suppression (legacy mode drops releases)
  /// 3. Numpad (APP_KEYPAD SS3 sequences, OR normal mode character values)
  /// 4. Named keys — legacy letter/tilde terminators
  /// 5. Control characters (Ctrl+letter → C0 codes)
  /// 6. Textual (resolved text with Alt prefix if needed)
  ///
  /// Note: ConPTY win32 input mode (DECSET ? 9001) is parsed and tracked
  /// but not wired into this chain — reserved for future native Windows.
  pub fn encode_normalized(event: &NormalizedKeyEvent) -> Vec<u8>
  ```
- [ ] **Win32 input mode path**: refactor `win32::encode_win32()` to accept `&NormalizedKeyEvent`:
  - ConPTY win32 input mode (`DECSET ? 9001`) sends `KEY_EVENT_RECORD` structures instead of VT text
  - **Note:** win32 encoding is currently `#[allow(dead_code)]` and NOT wired into `encode_key()` — ConPTY's win32 input records don't work through the WSL bridge (Ctrl+C is silently dropped). Keep the `#[allow(dead_code)]` and do NOT wire it into `encode_normalized()` yet. The migration is type-signature only, preserving the module for future native Windows support.
  - Migrate `key_record_fields()` and `control_key_state()` to use `NormalizedKey` and `NormalizedKeyEvent` fields
  - The encoding format is unchanged — only the input type changes
- [ ] **Kitty path**: refactor `kitty::encode_kitty()` to accept `&NormalizedKeyEvent`:
  - The existing fallback to `super::legacy::encode_legacy()` (line 100) for unambiguous named keys in DISAMBIGUATE-only mode must change since `encode_legacy()` is removed. Replace with direct calls to `super::legacy::letter_key()` / `super::legacy::tilde_key()` / `super::legacy::encode_simple_named()` (now `pub(super)`), assembling the legacy sequence inline. Alternatively, add a `legacy::encode_named_legacy()` convenience function that wraps these three.
  - Uses `event.key` (NormalizedKey) instead of raw winit `Key`
  - Uses `event.mods` (raw modifiers, NOT `effective_mods()`) for modifier encoding — Kitty protocol requires the actual modifier state, not the layout-adjusted state. `consumed_mods` is for legacy encoding only where you don't want to double-report Shift.
  - Uses `event.text` (always resolved) for associated text reporting
  - Uses `event.unshifted_codepoint` for base key in CSI u sequences
  - Uses `event.alternate_key` for REPORT_ALTERNATE_KEYS
  - `NormalizedKey::Character(u32)` already stores the codepoint — remove `resolve_char_codepoint()` from kitty.rs (its job is now done by normalization)
  - No changes to the CSI u format or legacy terminator logic — the refactor is input-side only
- [ ] **Numpad handling** — add `try_encode_numpad()` in `key_encoding/mod.rs`:
  ```rust
  fn try_encode_numpad(event: &NormalizedKeyEvent) -> Option<Vec<u8>> {
      if event.location != KeyLocation::Numpad {
          return None;
      }
      if event.mode.contains(TermMode::APP_KEYPAD) {
          // SS3 sequences (ESC O {code}) — existing encode_numpad_app logic
          legacy::encode_numpad_app(&event.key)
      } else {
          // Normal mode: send the resolved text as character value.
          // This is the FIX for bug 1 — numpad "5" sends "5" even
          // without APP_KEYPAD, using the resolved text (never None).
          event.text.as_ref().map(|t| t.as_bytes().to_vec())
      }
  }
  ```
- [ ] **Named key handling** — add `try_encode_named()` in `key_encoding/mod.rs` (part of the `encode_normalized()` priority chain):
  ```rust
  fn try_encode_named(event: &NormalizedKeyEvent) -> Option<Vec<u8>>
  ```
  Delegates to `legacy::letter_key()`, `legacy::tilde_key()`, `legacy::encode_simple_named()` — these helpers must be promoted to `pub(super)` visibility so `mod.rs` can call them. Uses `event.effective_mods()` for modifier parameters. In legacy mode, `effective_mods()` strips consumed modifiers (e.g., Shift consumed by uppercase) — this is correct because legacy encoding should not double-report Shift in the modifier parameter when the key is already uppercase. Note: `KeyAction::Press` and `KeyAction::Repeat` produce identical encoding output in legacy mode.
- [ ] **Control character handling** — add `try_encode_control()` in `key_encoding/mod.rs`:
  ```rust
  fn try_encode_control(event: &NormalizedKeyEvent) -> Option<Vec<u8>>
  ```
  Ctrl+letter → C0 control codes. Delegates to `legacy::ctrl_key_byte()` (promote to `pub(super)`). Uses `event.unshifted_codepoint` or the character key to determine the C0 byte. Alt prefix applied if Alt is in `effective_mods()`.
- [ ] **Textual fallback** — add `try_encode_text()` in `key_encoding/mod.rs`:
  ```rust
  fn try_encode_text(event: &NormalizedKeyEvent) -> Option<Vec<u8>>
  ```
  Sends `event.text` as UTF-8 bytes. If Alt is in `effective_mods()`, prepend ESC. This is the FIX for bug 2 — `event.text` is ALWAYS resolved for printable keys (never None).
- [ ] **Remove `KeyInput` type and `encode_key()` function** — all callers (`encode_key_to_pty` in `keyboard_input/mod.rs`) migrate to `encode_normalized()`. Remove `KeyInput`, `KeyEventType` from `mod.rs` exports. Keep `Modifiers` (used by keybindings) and `physical_key_to_us_codepoint()` (used by normalization).
- [ ] **Preserve `encode_numpad_app()` in legacy.rs** — still used internally by `try_encode_numpad()` for APP_KEYPAD path. Refactor signature to accept `&NormalizedKey` instead of `&Key`. Note: `NormalizedKey::Character(u32)` stores a codepoint, not a string — the match arms change from `Key::Character("5")` to `NormalizedKey::Character(cp) if *cp == b'5' as u32` (or convert codepoint to char for matching).
- [ ] **Promote legacy.rs helpers to `pub(super)`**: `letter_key()`, `tilde_key()`, `encode_simple_named()`, `encode_space()`, `ctrl_key_byte()` are promoted from private to `pub(super)` so the try_encode functions in `mod.rs` can call them. The `LetterKey` and `TildeKey` structs must also be promoted to `pub(super)` since they are return types of `letter_key()` / `tilde_key()`. The old `encode_legacy()` function is removed — its dispatch logic moves into the `encode_normalized()` priority chain. `encode_character()` is also removed -- its logic splits between `try_encode_control()` and `try_encode_text()`.
- [ ] **Write new bug-scenario tests FIRST** in `oriterm/src/key_encoding/tests.rs` (these should fail against the old `encode_key`, proving the bugs exist, then pass after migration to `encode_normalized`):
  - [ ] Numpad "5" in normal mode (no APP_KEYPAD) → sends `b"5"` (was: empty)
  - [ ] Numpad "+" in normal mode → sends `b"+"` (was: empty)
  - [ ] Numpad Enter in normal mode → sends `b"\r"` (Enter is Named, handled by try_encode_named)
  - [ ] Shift+A with winit text=None → sends `b"A"` (was: empty)
  - [ ] Shift+Z with winit text=None → sends `b"Z"` (was: empty)
  - [ ] Repeat Backspace → sends `0x7f` (identical to initial press)
  - [ ] Repeat Ctrl+C → sends `0x03` (identical to initial press)
  - [ ] Kitty mode: Shift+A → `ESC[97;2u` (unshifted codepoint 97, Shift not consumed in Kitty)
  - [ ] Kitty mode: numpad "5" → correct CSI u with numpad flag
  - [ ] Win32 input mode: `encode_win32()` compiles with `&NormalizedKeyEvent` (type migration only — win32 is dead code, not wired into `encode_normalized()`)
- [ ] **Migrate existing tests** in `oriterm/src/key_encoding/tests.rs`:
  - All existing tests must be updated to construct `NormalizedKeyEvent` instead of `KeyInput`
  - The expected output bytes must NOT change for legacy encoding tests — this is a refactor, not a behavioral change for legacy paths
  - **Kitty Shift+letter tests**: existing `kitty_shift_a` uses `Key::Character("A")` (codepoint 65). With `NormalizedKeyEvent` using `unshifted_codepoint`, this changes to 97 — a spec-correctness fix (Kitty spec says base key is unshifted). Update the expected bytes from `\x1b[65;2u` to `\x1b[97;2u` and document the change. The `kitty_text_shift_a` test already uses codepoint 97, so `REPORT_ASSOCIATED_TEXT` mode is unaffected.
  - Helper function `make_event(...)` that constructs `NormalizedKeyEvent` with sensible defaults

**Matrix dimensions:**
- Encoding path: Kitty, legacy-numpad-APP_KEYPAD, legacy-numpad-normal, legacy-named-letter, legacy-named-tilde, legacy-named-simple, legacy-control, legacy-text (win32 is dead code — type migration only, no encoding path test needed)
- Key types: letter, digit, punctuation, numpad digit, numpad operator, arrow, F-key, Backspace/Enter/Tab/Escape/Space
- Modifier combos: none, Shift, Ctrl, Alt, Ctrl+Shift, Ctrl+Alt
- Event action: Press, Repeat (both should produce identical output for legacy)

**Semantic pin:** `encode_normalized()` with numpad "5" + no APP_KEYPAD + text resolved to "5" → `vec![b'5']`. This ONLY passes with the new pipeline — the old `encode_key()` produced `Vec::new()`.

- [ ] **TPR checkpoint** — `/tpr-review` covering 08B.2–08B.3 implementation work (core types + encoding pipeline)

- [ ] `./build-all.sh` green, `./clippy-all.sh` green
- [ ] `./test-all.sh` green — all existing tests pass with updated constructors, plus new bug-scenario tests

---

## 08B.4 Dispatch Integration + Action Repeat Policy

**File(s):** `oriterm/src/app/keyboard_input/mod.rs`, `oriterm/src/keybindings/mod.rs`

Wire `NormalizedKeyEvent` into the dispatch chain. Add action repeat policy so destructive app actions don't auto-repeat. Document the Lua hook point.

This subsection uses the `NormalizedKeyEvent` from 08B.2 and `encode_normalized()` from 08B.3.

- [ ] **Move normalization to top of `handle_keyboard_input()`** — construct `NormalizedKeyEvent` ONCE before any dispatch, then pass it to both keybinding lookup and encoding. This ensures the Lua hook point (documented below) and keybinding dispatch all operate on the same normalized event:
  ```rust
  pub(super) fn handle_keyboard_input(&mut self, event: &winit::event::KeyEvent) {
      // ... IME suppression, tab editing, overlay dispatch (these stay on raw winit event)
      
      // FUTURE: Key remapping (Section 49.4) transforms the raw winit event
      // BEFORE normalization. Remap inserts here, operating on &winit::event::KeyEvent.
      
      // Normalize ONCE before keybinding and PTY encoding.
      let Some(pane_id) = self.active_pane_id() else { return };
      let Some(mode) = self.pane_mode(pane_id) else { return };
      let normalized = NormalizedKeyEvent::from_winit(event, self.modifiers, mode);
      
      // FUTURE: Lua key event callback (see hook point below)
      
      // FUTURE: Key table dispatch (Section 49.1) checks active key table stack
      // AFTER normalization but using the same priority as keybinding lookup.
      
      // Keybinding dispatch uses normalized.action for repeat policy.
      // PTY encoding uses encode_normalized(&normalized).
  }
  ```
  Note: the early dispatch stages (IME, tab editing, overlay, search, mark mode) continue to use the raw winit `KeyEvent` since they don't need normalization. Normalization only needs to happen before keybinding lookup and PTY encoding.
- [ ] **Rewrite `encode_key_to_pty()`** to accept `&NormalizedKeyEvent` (already constructed above):
  ```rust
  fn encode_key_to_pty(&mut self, pane_id: PaneId, normalized: &NormalizedKeyEvent) {
      let bytes = key_encoding::encode_normalized(normalized);
      if !bytes.is_empty() {
          // ... existing scroll-to-bottom, write, SIGINT, blink reset, cursor hide logic
      }
  }
  ```
- [ ] **Action repeat policy** — add `is_repeatable()` to `Action` enum in `keybindings/mod.rs`:
  ```rust
  impl Action {
      /// Whether this action should fire on repeat key events.
      ///
      /// Safe actions (scroll, zoom, resize, paste) repeat.
      /// Destructive/toggle actions (close, split, new tab, mode toggles) don't.
      /// Future Lua-defined actions declare their own repeatability.
      pub(crate) fn is_repeatable(&self) -> bool {
          matches!(self,
              Self::ScrollPageUp | Self::ScrollPageDown
              | Self::ScrollToTop | Self::ScrollToBottom
              | Self::ZoomIn | Self::ZoomOut
              | Self::ResizePaneUp | Self::ResizePaneDown
              | Self::ResizePaneLeft | Self::ResizePaneRight
              | Self::Paste | Self::SmartPaste
              | Self::PreviousPrompt | Self::NextPrompt
              | Self::FocusPaneUp | Self::FocusPaneDown
              | Self::FocusPaneLeft | Self::FocusPaneRight
              | Self::NextPane | Self::PrevPane
              | Self::NextTab | Self::PrevTab
              | Self::SendText(_)
          )
      }
  }
  ```
- [ ] **Fix keybinding dispatch** to use `normalized.action` for repeat policy (`keyboard_input/mod.rs`):
  ```rust
  // Current (line 145): fires on ALL Pressed events including repeats
  if event.state == ElementState::Pressed {
  
  // Fixed: use normalized.action for repeat check
  if normalized.action != KeyAction::Release {
      let is_repeat = normalized.action == KeyAction::Repeat;
      let mods = normalized.effective_mods(); // or normalized.mods for binding lookup
      if let Some(binding_key) = keybindings::normalized_key_to_binding_key(&normalized.key) {
          if let Some(action) = keybindings::find_binding(&self.bindings, &binding_key, mods) {
              if is_repeat && !action.is_repeatable() {
                  return; // Suppress repeat for non-repeatable actions
              }
              let action = action.clone();
              if self.execute_action(&action) {
                  return;
              }
          }
      }
  }
  ```
  **This is the FIX for bug 3** (if the issue is dispatch-level): repeat events for unbound keys (like Backspace) fall through to `encode_key_to_pty()` and produce output. Repeat events for bound non-repeatable actions (like CloseTab) are suppressed.
- [ ] **Add `normalized_key_to_binding_key()`** in `oriterm/src/keybindings/mod.rs`: new `pub(crate)` function accepting `&NormalizedKey` instead of `&winit::keyboard::Key`. Matches `NormalizedKey::Named(n)` → `BindingKey::Named(n)`, `NormalizedKey::Character(cp)` → `BindingKey::Character(char::from_u32(cp).to_lowercase())`. The existing `key_to_binding_key()` is kept until all callers are migrated (it may still be used by non-PTY dispatch paths like overlay key handling).
- [ ] **Document Lua hook point** — add a `// FUTURE: Lua key event dispatch` comment block in the dispatch chain between normalization and keybinding lookup:
  ```rust
  // Normalize the raw winit event.
  let normalized = NormalizedKeyEvent::from_winit(event, self.modifiers, mode);
  
  // FUTURE: Lua key event callback (Section 28.5).
  // When the Lua runtime lands (see project_lua_runtime_plan.md), this is
  // where `oriterm.on_key_event(normalized)` dispatches. Lua can:
  //   - Inspect the NormalizedKeyEvent (all fields are Lua-serializable)
  //   - Consume the event (return true → stop propagation)
  //   - Let it fall through (return false → continue to keybindings/PTY)
  //   - Call encode_normalized() directly to send custom PTY sequences
  //   - Modify fields (remap key, change mods) before fallthrough
  // The NormalizedKeyEvent becomes a Lua UserData via mlua.
  // encode_normalized() is a standalone function (no &self, no App state)
  // so Lua can call it without access to the dispatch context.
  
  // Keybinding lookup with repeat policy...
  ```
- [ ] **Tests FIRST**. Split across two test files to respect crate boundaries:

  **`oriterm/src/keybindings/tests.rs`** (existing file — `is_repeatable()` lives on `Action` in `keybindings/mod.rs`):
  - [ ] `is_repeatable()` returns false for all destructive actions (CloseTab, ClosePane, SplitRight, SplitDown, NewTab, NewWindow, ToggleFullscreen, MoveTabToNewWindow, etc.)
  - [ ] `is_repeatable()` returns true for all navigation/scroll actions (ScrollPageUp, ScrollPageDown, NextTab, PrevTab, FocusPaneUp, FocusPaneDown, ZoomIn, ZoomOut, Paste, SmartPaste, SendText, etc.)
  - [ ] `normalized_key_to_binding_key()` maps `NormalizedKey::Named(ArrowUp)` → `BindingKey::Named(ArrowUp)`
  - [ ] `normalized_key_to_binding_key()` maps `NormalizedKey::Character(97)` → `BindingKey::Character("a")`

  **`oriterm/src/app/keyboard_input/tests.rs`** (existing file — dispatch integration tests):
  - [ ] Repeat Backspace: `encode_normalized()` with `action=Repeat, key=Named(Backspace)` produces `[0x7f]` (same as Press)
  - [ ] Repeat plain 'a': `encode_normalized()` with `action=Repeat, text=Some("a")` produces `b"a"` (same as Press)
  - [ ] Repeat CloseTab scenario: `find_binding(Ctrl+W)` returns `CloseTab`, and `CloseTab.is_repeatable()` returns false — so dispatch suppresses repeat. (Unit test using `find_binding` + `is_repeatable`, no App needed.)
  - [ ] Repeat ScrollPageUp scenario: `find_binding(Shift+PageUp)` returns `ScrollPageUp`, and `ScrollPageUp.is_repeatable()` returns true — so dispatch allows repeat.

**Matrix dimensions:**
- Action repeatability: repeatable action × repeat event → fires; non-repeatable × repeat → suppressed
- Unbound keys: repeat event → falls through to PTY encoding
- Event flow: Press, Repeat, Release × keybinding-matched, keybinding-unmatched

**Semantic pin:** Test that holding Ctrl+W (CloseTab) with `event.repeat=true` does NOT execute `Action::CloseTab`. This ONLY passes with the new repeat policy — the old code would execute on every repeat.

- [ ] `./build-all.sh` green, `./clippy-all.sh` green
- [ ] `./test-all.sh` green — all existing keyboard_input tests pass, plus new repeat policy tests

---

## 08B.5 Alt+Non-ASCII Key Encoding

<!-- Ghostty audit: #7110 (alt+é should send ESC é) -->

**Source:** Ghostty #7110 — On non-US keyboard layouts (AZERTY, etc.), pressing Alt+é should send ESC followed by the é character (U+00E9). Instead, Ghostty sends only ESC, and the following character gets a U+FFFD replacement prefix. Alacritty had the same bug (#4862).

**Problem:** The Alt key encoding path (`try_encode_alt`) typically sends ESC + the character's byte. But for non-ASCII characters (é, è, ç, à), the encoding may fail because the character doesn't fit in a single byte, or the platform reports the key event without the text field when Alt is held.

**Required work:**

- [ ] In the Alt encoding path of `NormalizedKeyEvent`: when Alt is held and the resolved text is a non-ASCII Unicode character, send ESC followed by the UTF-8 encoding of that character
- [ ] Ensure `resolve_text()` correctly resolves the base character even when Alt is held (platform may strip the text)
- [ ] Handle AZERTY, QWERTZ, and other non-US layouts where number keys produce accented characters
- [ ] Test: Alt+é → ESC + U+00E9 (2 bytes UTF-8); Alt+ç → ESC + U+00E7

**Priority:** Medium — affects all non-US keyboard layout users.

**Reference:** Alacritty #4862, xterm Alt encoding behavior.

---

## 08B.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers.
If unresolved findings exist here:
- section frontmatter `status` must be `in-progress`
- `third_party_review.status` must be `findings`

When all findings are triaged:
- accepted findings are integrated into the relevant implementation subsection(s)
- rejected findings are closed with rationale
- all items in this block are marked resolved
- `third_party_review.status` becomes `resolved` or `none`
-->

- None.

---

## 08B.N Completion Checklist

- [ ] All 08B.1–08B.4 items complete
- [ ] **Bug 1 verified**: numpad 0-9 produce digits in normal mode (no APP_KEYPAD) — test: `cargo test -p oriterm --target x86_64-pc-windows-gnu -- numpad_normal`
- [ ] **Bug 2 verified**: Shift+letter produces uppercase when winit text is None — test: `cargo test -p oriterm --target x86_64-pc-windows-gnu -- shift_letter_no_text`
- [ ] **Bug 3 verified**: key repeat produces output for unbound keys — test: `cargo test -p oriterm --target x86_64-pc-windows-gnu -- repeat_backspace`
- [ ] **Action repeat policy**: CloseTab does NOT repeat — test: `cargo test -p oriterm --target x86_64-pc-windows-gnu -- repeat_policy`
- [ ] All existing key encoding tests pass with `NormalizedKeyEvent` constructors (legacy unchanged; Kitty Shift+letter codepoints updated per spec)
- [ ] `NormalizedKeyEvent` fields are all primitives/strings (Lua-serializable, no opaque handles)
- [ ] `encode_normalized()` is callable without `App` or dispatch state (standalone function)
- [ ] Lua hook point documented in dispatch chain with integration notes
- [ ] Old `KeyInput` type and `encode_key()` function removed — no dead code
- [ ] Cross-platform: text resolution fallbacks tested independently of `key_without_modifiers()` accuracy
- [ ] No files over 500 lines (source, excluding tests)
- [ ] `./build-all.sh` green (cross-compile x86_64-pc-windows-gnu)
- [ ] `./clippy-all.sh` green (no warnings)
- [ ] `./test-all.sh` green (all tests pass)
- [ ] Plan annotation cleanup: all temporary scaffolding removed from `.rs` files
- [ ] All intermediate TPR checkpoint findings resolved
- [ ] **Plan sync** — update plan metadata:
  - [ ] This section's frontmatter `status` → `complete`, subsection statuses updated
  - [ ] `00-overview.md` Quick Reference table status updated for this section
  - [ ] `index.md` section status updated
  - [ ] Section 08 annotated: "runtime bugs resolved by Section 08B"
  - [ ] Section 49 annotated: "08B establishes the normalization/dispatch pipeline that 49 extends (remap before normalization, key tables after)"
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed — MUST run AFTER `/tpr-review` is clean

**Exit Criteria:** All three reported bugs are fixed and regression-tested. The encoding pipeline processes every key event through a normalized model with resolved text. Numpad keys produce output in normal mode, Shift+letter produces uppercase without winit text, held keys repeat at OS rate, and destructive app actions don't auto-repeat. All tests pass across `./test-all.sh`, `./build-all.sh`, and `./clippy-all.sh`. The architecture is Lua-ready with documented hook points and serializable event types.
