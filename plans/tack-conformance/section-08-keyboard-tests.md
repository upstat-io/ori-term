---
section: "08"
title: "Keyboard / Function Key Tests"
status: not-started
reviewed: false
goal: "Add keyboard capability tests in the `oriterm` crate that exercise the real `key_encoding::encode_key` pipeline against the function-key sequences declared in extra/ori_term.info. For each terminfo cap (kf1-kf63, kcub1, kcud1, kcuf1, kcuu1, khome, kend, kpp, knp, kdch1, kich1, kbs), the test (a) reads the expected sequence from the compiled terminfo via infocmp, (b) constructs a KeyInput for the corresponding key+modifier combo, (c) calls encode_key, and (d) asserts the produced bytes match the terminfo declaration. This is the only section in oriterm rather than oriterm_core because it tests the application-layer key encoder."
success_criteria:
  - "`oriterm/src/key_encoding/terminfo_xcheck.rs` in-crate sibling test module exists (preferred path, no visibility change). Fallback `oriterm/tests/keyboard_terminfo.rs` integration test target only if the preferred path is blocked and documented in 08.R."
  - "`infocmp_query(env, term, cap_name) -> Option<String>` helper exists in oriterm_test_support — invokes infocmp to extract a single capability's value"
  - "Test for each ori_term-declared function key (kf1-kf12 minimum, ideally up through kf63 with modifiers): expected sequence from infocmp matches encode_key output"
  - "Test for each cursor key (kcub1, kcud1, kcuf1, kcuu1) in normal mode (rmkx) AND application mode (smkx)"
  - "Test for kbs (backspace), khome, kend, kpp (pgup), knp (pgdn), kdch1 (delete), kich1 (insert)"
  - "All tests skip cleanly when tic/infocmp are unavailable (Windows) — runtime gate, not cfg"
  - "All tests use the pinned ori_term terminfo via TerminfoEnv (not the host xterm-256color)"
  - "`timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck` passes on Linux (preferred in-crate path). Fallback command `cargo test -p oriterm --test keyboard_terminfo` only applies if the fallback path is taken (documented in 08.R)."
  - "All terminfo_xcheck tests run deterministically across 10 consecutive runs"
  - "Satisfies mission criterion: 'Keyboard/function key capability tests exist in oriterm crate exercising real key encoding pipeline'"
inspired_by:
  - "ori_term key_encoding (oriterm/src/key_encoding/mod.rs:116 — encode_key entry point that this section validates)"
  - "ori_term key_encoding tests (oriterm/src/key_encoding/tests.rs — existing per-key encoding tests, expanded here to validate against terminfo)"
  - "Section 02 TerminfoEnv (plans/tack-conformance/section-02-terminfo-provisioning.md — provides the compiled ori_term terminfo to read sequences from)"
  - "ncurses infocmp(1) man page — capability extraction syntax"
depends_on: ["01", "02"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "08.1"
    title: "infocmp_query helper in oriterm_test_support"
    status: not-started
  - id: "08.2"
    title: "Function key tests (kf1-kf63)"
    status: not-started
  - id: "08.3"
    title: "Cursor key tests (rmkx + smkx modes)"
    status: not-started
  - id: "08.4"
    title: "Editing/navigation key tests (kbs, khome, kend, kpp, knp, kdch1, kich1)"
    status: not-started
  - id: "08.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "08.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 08: Keyboard / Function Key Tests

**Status:** Not Started
**Goal:** Validate that ori_term's `key_encoding::encode_key` produces byte sequences that exactly match what `extra/ori_term.info` declares for each function/cursor/editing key. This catches the silent class of bugs where the terminfo claims `kf1=\EOP` but encode_key emits `\E[11~` (or vice versa) — the symptom is "F1 doesn't work in vim under ori_term", and the root cause is a divergence between the terminfo declaration and the application's actual key encoder. After this section, every key in the terminfo is mechanically verified to round-trip.

**Success Criteria:**

- [ ] **Preferred in-crate sibling path:** `oriterm/src/key_encoding/terminfo_xcheck.rs` exists as a `#[cfg(test)] mod terminfo_xcheck;` sibling submodule of `oriterm/src/key_encoding/mod.rs`, leaving `pub(crate) mod key_encoding;` unchanged in `oriterm/src/lib.rs:17`. Fallback `oriterm/tests/keyboard_terminfo.rs` integration test target is only created if the in-crate path is blocked (documented in 08.R).
- [ ] `oriterm_test_support::infocmp_query(env: &TerminfoEnv, term: &str, cap: &str) -> Option<String>` exists and returns the raw cap value (or None if unset)
- [ ] All function keys F1-F12 in normal mode (no modifiers) tested
- [ ] All function keys F1-F12 in shift, control, and alt modifier combinations tested where ori_term encodes them
- [ ] All cursor keys (Up/Down/Left/Right) tested in both rmkx (normal) and smkx (application) modes
- [ ] All editing keys (Backspace, Delete, Insert, Home, End, PgUp, PgDn) tested
- [ ] All tests skip cleanly when `tic`/`infocmp` are unavailable
- [ ] `timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck` passes (preferred in-crate path)
- [ ] 10 consecutive runs of the `terminfo_xcheck` test group all pass (determinism gate)
- [ ] Satisfies mission criterion #11

**Context:** Function key encoding is a notorious source of "works on the keyboard, breaks in vim/less/htop" bugs. The chain is:
1. User presses F1
2. winit reports `KeyCode::F1` to ori_term
3. `key_encoding::encode_key` translates to bytes (e.g., `\EOP`)
4. Bytes flow through the PTY into the foreground process (vim, etc.)
5. Vim looks up `\EOP` in terminfo, finds it matches `kf1` for the current TERM
6. Vim invokes the F1 binding

If step 3 produces `\E[11~` but step 5's terminfo says `kf1=\EOP`, vim sees an unknown sequence and the binding doesn't fire. The terminfo claims one thing, the encoder does another, the user gets broken keys.

The fix is mechanical: for every key cap in the terminfo, assert that encode_key produces the same bytes the terminfo declares. This section adds those tests. The terminfo is the source of truth (Section 02 pinned it), and encode_key must conform.

**Reference implementations:**
- **ori_term** `oriterm/src/key_encoding/mod.rs:116`: `pub fn encode_key(input: &KeyInput<'_>) -> Vec<u8>` — the function under test.
- **ori_term** `oriterm/src/key_encoding/tests.rs`: existing per-key encoding tests. This section adds a NEW in-crate sibling test submodule (`terminfo_xcheck.rs`) that validates the same encoder against the terminfo as the source of truth — complementary to the existing unit tests, not a replacement.
- **Section 02** `plans/tack-conformance/section-02-terminfo-provisioning.md`: `TerminfoEnv::compile()` is the source of the pinned terminfo we read from.
- **ncurses infocmp(1) man page**: capability extraction syntax.

**Depends on:** Section 01 (PtySession framework — actually only `oriterm_test_support` is needed; PtySession itself is not used here), Section 02 (TerminfoEnv).

**PREFERRED APPROACH — In-crate sibling tests (no visibility change):** `oriterm/src/key_encoding/tests.rs` already exists as a 1801-line sibling test module with full access to the module's items via `super::`. New keyboard-terminfo cross-check tests belong inside that existing sibling file (or a new submodule under it), NOT in a separate `tests/` integration target. Benefits:

1. **Zero public API surface change.** `key_encoding` stays `pub(crate) mod key_encoding` — encoder internals remain hidden from downstream users.
2. **`super::` imports work directly.** No visibility gymnastics needed — the test module already has `use super::{encode_key, KeyInput, Modifiers, KeyEventType};`-style access to every name in `mod.rs`, `legacy.rs`, and `kitty.rs`.
3. **Dev-deps already reach sibling tests.** `oriterm_test_support` is added to `oriterm`'s `[dev-dependencies]` in Section 01.4, which makes it visible to BOTH integration tests and sibling `#[cfg(test)]` modules. There is no dependency advantage to the integration test target.
4. **Respects the `.claude/rules/test-organization.md` sibling-tests convention.** This is how every other test in `oriterm` is organized.

**File layout (preferred):** create `oriterm/src/key_encoding/terminfo_xcheck.rs` as a new submodule containing the cross-check test fn bodies and the `CapMapping` table. Add `#[cfg(test)] mod terminfo_xcheck;` at the bottom of `oriterm/src/key_encoding/mod.rs`. The `terminfo_xcheck.rs` file is itself a test-only module (gated by the `#[cfg(test)]` declaration) — its `#[test] fn`s appear in `cargo test -p oriterm` output prefixed with `key_encoding::terminfo_xcheck::...`.

With this layout, the visibility of `key_encoding` does NOT change — `pub(crate) mod key_encoding;` stays as-is.

**FALLBACK — Integration test target (only if the in-crate approach is infeasible):** if, for some reason, the in-crate sibling approach does not work (e.g., cyclical dev-dep that forces the test target to live elsewhere), fall back to `oriterm/tests/keyboard_terminfo.rs` as an integration test target. This path requires changing `oriterm/src/lib.rs:17` from `pub(crate) mod key_encoding;` to `pub mod key_encoding;` so the integration test can import `encode_key`, `KeyInput`, `KeyEventType`, and `Modifiers`. This widens the public API surface — every `encode_key` caller in the stable API becomes something downstream crates could theoretically invoke. Only take this path if the preferred in-crate approach is blocked.

The plan picks the PREFERRED path (in-crate sibling). All subsection checklists below use `oriterm/src/key_encoding/terminfo_xcheck.rs` as the file target, with `oriterm/tests/keyboard_terminfo.rs` listed as the fallback file for reference.

**Cross-platform skip discipline:** every test in `terminfo_xcheck.rs` MUST begin with `if !tic_available() || !infocmp_available() { return; }` so Windows native (no ncurses) compiles AND runs cleanly. Use runtime gating via `tic_available()` / `infocmp_available()` from `oriterm_test_support`, NOT `#[cfg(unix)]` — per CLAUDE.md "every `#[cfg(target_os = "...")]` block must have counterparts for all supported targets. If a feature cannot be implemented on a platform, it must degrade gracefully with a compile-time `cfg` gate, not a runtime panic." In this case, the TEST must compile on every platform and runtime-skip on Windows — never `#[cfg(unix)]`.

**CRITICAL — `KeyInput` struct shape:** `oriterm/src/key_encoding/mod.rs:80-99` defines `KeyInput<'a>` with borrow fields. The actual fields are:
- `key: &'a Key` (not `logical_key`)
- `mods: Modifiers` (not `modifiers`)
- `mode: TermMode` (not `app_keypad`)
- `text: Option<&'a str>`
- `location: KeyLocation` (not `physical_key`)
- `event_type: KeyEventType` (not `state_pressed`/`repeat`)
- `alternate_key: Option<u32>`

`KeyInput` does **NOT** implement `Default`, and it holds borrows (`&'a Key`, `&'a str`), so it is not `const`-constructible and cannot be stored in a `static` table. Test bodies must:
1. Construct owning `Key::Named(NamedKey::F1)` values into `let` bindings in the test function body, then take references.
2. Explicitly provide every field — no `..Default::default()`.
3. Use `TermMode::APP_KEYPAD | TermMode::APP_CURSOR` to represent application-mode keys (not an `app_keypad: bool` field).
4. Use `KeyEventType::Press` for press events.

The table-driven pattern the original draft used (`const F_KEYS_BASE: &[CapMapping]`) still works if `CapMapping` stores only owned/plain data (cap name, `NamedKey` variant, `Modifiers` bitflags, `bool` for app-mode), and the test loop constructs the `KeyInput` per-iteration from the owning data.

---

## 08.1 infocmp_query helper in oriterm_test_support

**File(s):** `crates/oriterm_test_support/src/terminfo/mod.rs` (extend Section 02's module)

`infocmp_query` is a small subprocess wrapper that extracts a single capability's value from a compiled terminfo entry. We use it to read the canonical "what should the F1 key emit?" from `extra/ori_term.info` (compiled at runtime via `TerminfoEnv`).

- [ ] Extend `crates/oriterm_test_support/src/terminfo/mod.rs`:
  ```rust
  /// Extract a single capability's value from a compiled terminfo entry.
  ///
  /// Returns:
  ///   - `Some(value)` if the cap is declared (string caps return their
  ///     value with terminfo escape syntax decoded back to raw bytes)
  ///   - `None` if the cap is not declared
  ///
  /// Implementation: invokes `infocmp -A <env_dir> -1 <term>` to get
  /// one cap per line, then parses the output looking for `<cap>=...,`
  /// in the matching entry.
  ///
  /// Returns `None` if `infocmp` is not available — callers must
  /// gate on `infocmp_available()` first.
  #[must_use]
  pub fn infocmp_query(env: &TerminfoEnv, term: &str, cap: &str) -> Option<String> {
      use std::process::Command;
      let output = Command::new("infocmp")
          .arg("-A")
          .arg(env.terminfo_dir())
          .arg("-1")
          .arg(term)
          .output()
          .ok()?;
      if !output.status.success() {
          return None;
      }
      let text = String::from_utf8_lossy(&output.stdout);
      // Each cap is on its own line: `\t<name>=<value>,`
      // We match `<name>=` at the start (after leading whitespace).
      let prefix = format!("{cap}=");
      for line in text.lines() {
          let trimmed = line.trim_start();
          if let Some(rest) = trimmed.strip_prefix(&prefix) {
              // Strip trailing comma (terminfo entry separator).
              let value = rest.trim_end_matches(',');
              return Some(value.to_string());
          }
      }
      None
  }

  /// Decode terminfo escape syntax to raw bytes.
  ///
  /// Terminfo encodes ESC as `\E`, CR as `\r`, NL as `\n`, etc. The
  /// `infocmp_query` helper returns these encoded forms — this
  /// function converts them back to the raw byte sequences that
  /// `encode_key` would actually produce.
  ///
  /// Handles:
  ///   `\E` → 0x1b (ESC)
  ///   `\r` → 0x0d (CR)
  ///   `\n` → 0x0a (LF)
  ///   `\t` → 0x09 (TAB)
  ///   `\b` → 0x08 (BS)
  ///   `\f` → 0x0c (FF)
  ///   `\\` → `\`
  ///   `^X` → control char (X is the letter)
  ///   `\NNN` → octal
  ///   `\xNN` → hex (rare in terminfo)
  ///
  /// Does NOT handle parameterized strings (`%p1%d` etc.) — those are
  /// for cup/csr/setaf etc., not for keyboard caps. Keyboard caps are
  /// always plain escape sequences.
  #[must_use]
  pub fn decode_terminfo_string(s: &str) -> Vec<u8> {
      let mut out = Vec::with_capacity(s.len());
      let mut chars = s.chars().peekable();
      while let Some(c) = chars.next() {
          if c == '\\' {
              match chars.next() {
                  Some('E') | Some('e') => out.push(0x1b),
                  Some('r') => out.push(b'\r'),
                  Some('n') => out.push(b'\n'),
                  Some('t') => out.push(b'\t'),
                  Some('b') => out.push(0x08),
                  Some('f') => out.push(0x0c),
                  Some('\\') => out.push(b'\\'),
                  Some('s') => out.push(b' '),
                  Some(',') => out.push(b','),
                  Some(':') => out.push(b':'),
                  Some('0') => out.push(0),
                  Some(other) if other.is_ascii_digit() => {
                      // Octal (up to 3 digits).
                      let mut n = (other as u8) - b'0';
                      for _ in 0..2 {
                          if let Some(d) = chars.peek().filter(|c| c.is_ascii_digit()) {
                              n = n * 8 + (*d as u8 - b'0');
                              chars.next();
                          } else {
                              break;
                          }
                      }
                      out.push(n);
                  }
                  Some(other) => {
                      // Unknown escape — push the backslash and the char
                      // verbatim so the test can fail with a clear diff.
                      out.push(b'\\');
                      out.extend(other.to_string().bytes());
                  }
                  None => out.push(b'\\'),
              }
          } else if c == '^' {
              // Control char: ^X means Ctrl-X.
              if let Some(letter) = chars.next() {
                  let upper = letter.to_ascii_uppercase();
                  if ('@'..='_').contains(&upper) {
                      out.push((upper as u8) - b'@');
                  } else if upper == '?' {
                      out.push(0x7f); // DEL
                  } else {
                      out.push(b'^');
                      out.extend(letter.to_string().bytes());
                  }
              } else {
                  out.push(b'^');
              }
          } else {
              // Regular char (ASCII expected for keyboard caps).
              out.extend(c.to_string().bytes());
          }
      }
      out
  }
  ```

- [ ] Add sibling tests at `crates/oriterm_test_support/src/terminfo/tests.rs`:
  ```rust
  use super::{infocmp_query, decode_terminfo_string};

  #[test]
  fn decode_terminfo_string_handles_esc() {
      assert_eq!(decode_terminfo_string("\\E[OP"), b"\x1b[OP");
  }

  #[test]
  fn decode_terminfo_string_handles_control_caret() {
      // ^? = DEL = 0x7f
      assert_eq!(decode_terminfo_string("^?"), &[0x7f]);
      // ^H = BS = 0x08
      assert_eq!(decode_terminfo_string("^H"), &[0x08]);
      // ^A = SOH = 0x01
      assert_eq!(decode_terminfo_string("^A"), &[0x01]);
  }

  #[test]
  fn decode_terminfo_string_handles_octal() {
      // \033 = octal for 27 = ESC
      assert_eq!(decode_terminfo_string("\\033"), &[0x1b]);
  }

  #[test]
  fn infocmp_query_returns_none_for_missing_cap() {
      use crate::tic_available;
      if !tic_available() { return; }
      let env = super::TerminfoEnv::compile();
      // Pick a cap that ori_term will not declare — use a made-up
      // name. infocmp will not find it.
      assert_eq!(infocmp_query(&env, "ori_term", "completely_made_up_cap_xyz"), None);
  }

  #[test]
  fn infocmp_query_extracts_kf1() {
      use crate::{tic_available, infocmp_available};
      if !tic_available() || !infocmp_available() { return; }
      let env = super::TerminfoEnv::compile();
      let kf1 = infocmp_query(&env, "ori_term", "kf1");
      assert!(kf1.is_some(), "ori_term must declare kf1");
      let raw = decode_terminfo_string(&kf1.unwrap());
      // kf1 should start with ESC.
      assert_eq!(raw[0], 0x1b);
      // It should be 3 or more bytes (the parser doesn't constrain
      // the exact value here — that's the job of Section 08.2's
      // encode_key vs terminfo cross-check).
      assert!(raw.len() >= 2);
  }
  ```

- [ ] Re-export from `lib.rs`:
  ```rust
  pub use terminfo::{decode_terminfo_string, infocmp_query, TerminfoEnv};
  ```

---

## 08.2 Function key tests (kf1-kf63)

**File(s) — PREFERRED:** `oriterm/src/key_encoding/terminfo_xcheck.rs` (NEW submodule, in-crate sibling test), `oriterm/src/key_encoding/mod.rs` (add `#[cfg(test)] mod terminfo_xcheck;` at bottom)

**File(s) — FALLBACK only if in-crate blocked:** `oriterm/tests/keyboard_terminfo.rs` (integration test target) + `oriterm/src/lib.rs:17` visibility change.

The new submodule reads each `kfN` cap from the pinned terminfo, constructs a `KeyInput` for the corresponding F-key + modifier combo, calls `encode_key`, and asserts the produced bytes match the terminfo declaration.

- [ ] **Add submodule declaration.** At the bottom of `oriterm/src/key_encoding/mod.rs`, after the existing production code and before any other `#[cfg(test)]` declaration, add:
  ```rust
  #[cfg(test)]
  mod terminfo_xcheck;
  ```
  Verify `pub(crate) mod key_encoding;` in `oriterm/src/lib.rs:17` stays UNCHANGED — the in-crate approach does not require the visibility promotion.

- [ ] **Super-import discipline.** The new file uses `super::{encode_key, KeyInput, KeyEventType, Modifiers};` to reach sibling items. No `use oriterm::key_encoding::...` — the file IS inside `key_encoding`, so `super::` is the correct path. For items from `oriterm_core` (`TermMode`), use the full crate path `use oriterm_core::TermMode;`. For items from `oriterm_test_support` (`TerminfoEnv`, `infocmp_query`, `decode_terminfo_string`, `tic_available`, `infocmp_available`), use `use oriterm_test_support::{...};` — the dev-dependency added in Section 01.4 makes the crate visible inside `#[cfg(test)]` modules anywhere in `oriterm`.

- [ ] Create `oriterm/src/key_encoding/terminfo_xcheck.rs`:
  ```rust
  //! Cross-check ori_term's key_encoding pipeline against the pinned
  //! terminfo entry. For each function/cursor/editing cap declared in
  //! `extra/ori_term.info`, this test asserts that the bytes
  //! `super::encode_key` would emit exactly match what the terminfo
  //! says they should be.
  //!
  //! The point is to catch silent divergences where the terminfo claims
  //! `kf1=\EOP` but the encoder emits `\E[11~` (or vice versa). Vim,
  //! less, htop and every ncurses-aware tool look up key sequences in
  //! terminfo — if the encoder doesn't match, F1 silently breaks for
  //! the user.
  //!
  //! Skips when `tic` or `infocmp` are unavailable (Windows native) —
  //! runtime gate, never a `cfg(unix)` block (per CLAUDE.md
  //! cross-platform rule: compile everywhere, runtime skip).
  //!
  //! This file is the preferred in-crate sibling location (see
  //! `plans/tack-conformance/section-08-keyboard-tests.md` for why it
  //! lives here instead of `oriterm/tests/keyboard_terminfo.rs`).

  use super::{encode_key, KeyEventType, KeyInput, Modifiers};
  use oriterm_core::TermMode;
  use oriterm_test_support::{
      decode_terminfo_string, infocmp_available, infocmp_query, tic_available, TerminfoEnv,
  };
  use winit::keyboard::{Key, KeyLocation, NamedKey};

  /// One terminfo cap → key mapping. Fields are plain data so the
  /// table can be `static`; the test loop constructs the borrowing
  /// `KeyInput` per-iteration from owning `Key::Named(...)` locals.
  #[derive(Copy, Clone)]
  struct CapMapping {
      /// Terminfo cap name (e.g., `kf1`, `kcub1`).
      cap: &'static str,
      /// Named key (F1-F24, ArrowLeft, Home, etc.) — owned, not a reference.
      named: NamedKey,
      /// Modifier bitset.
      mods: Modifiers,
      /// `true` when the test simulates application-keypad/cursor mode.
      app_mode: bool,
  }

  /// Function key mapping table — F1-F12, no modifiers, normal mode.
  static F_KEYS_BASE: &[CapMapping] = &[
      CapMapping { cap: "kf1",  named: NamedKey::F1,  mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kf2",  named: NamedKey::F2,  mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kf3",  named: NamedKey::F3,  mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kf4",  named: NamedKey::F4,  mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kf5",  named: NamedKey::F5,  mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kf6",  named: NamedKey::F6,  mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kf7",  named: NamedKey::F7,  mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kf8",  named: NamedKey::F8,  mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kf9",  named: NamedKey::F9,  mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kf10", named: NamedKey::F10, mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kf11", named: NamedKey::F11, mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kf12", named: NamedKey::F12, mods: Modifiers::empty(), app_mode: false },
  ];

  fn assert_encoded_matches_terminfo(env: &TerminfoEnv, mapping: &CapMapping) {
      let cap_str = match infocmp_query(env, "ori_term", mapping.cap) {
          Some(s) => s,
          None => {
              eprintln!("skip: {} not declared in terminfo", mapping.cap);
              return;
          }
      };
      let expected = decode_terminfo_string(&cap_str);

      // Own the Key on the stack so the &'a Key borrow in KeyInput is valid.
      let key = Key::Named(mapping.named);

      // Build TermMode. Application keypad mode is a bitflag on TermMode,
      // not a separate KeyInput field. Cursor keys in app mode also set
      // APP_CURSOR — verify against oriterm_core::TermMode::APP_CURSOR.
      let mut mode = TermMode::empty();
      if mapping.app_mode {
          mode |= TermMode::APP_KEYPAD | TermMode::APP_CURSOR;
      }

      // Construct KeyInput explicitly — no Default impl; KeyInput<'a>
      // holds borrows so every field must be provided.
      let input = KeyInput {
          key: &key,
          mods: mapping.mods,
          mode,
          text: None,
          location: KeyLocation::Standard,
          event_type: KeyEventType::Press,
          alternate_key: None,
      };
      let actual = encode_key(&input);

      assert_eq!(
          actual, expected,
          "key_encoding::encode_key for cap {} produced {:?} but terminfo says {:?}",
          mapping.cap, actual, expected,
      );
  }

  #[test]
  fn function_keys_match_terminfo() {
      if !tic_available() || !infocmp_available() {
          eprintln!("tic or infocmp not installed, skipping function_keys_match_terminfo");
          return;
      }
      let env = TerminfoEnv::compile();
      for mapping in F_KEYS_BASE {
          assert_encoded_matches_terminfo(&env, mapping);
      }
  }
  ```


  **Note on `KeyInput` field set:** the exact fields and their defaults may evolve. The implementer must verify against `oriterm/src/key_encoding/mod.rs:80` at the time of writing. `KeyInput` does NOT derive `Default` — every field must be provided explicitly. `KeyInput<'a>` holds borrows, so the test must own `Key::Named(...)` values on the stack and take `&key` references. Application keypad/cursor mode is a `TermMode` bitflag, not a separate `KeyInput` field.

  **Note on `NamedKey::F13`-`F24`:** if ori_term encodes keys above F12, add those mappings too. Inspect `oriterm/src/key_encoding/legacy.rs` and `kitty.rs` for the supported range. The terminfo declares kf1-kf63 historically — many of those (kf13-kf63) are modified F1-F12 sequences (`kf13 = Shift+F1`, etc.), not separate physical keys.

- [ ] Add modified F-key tests (Shift+F1, Ctrl+F1, etc.). Each table is fully enumerated — NO `// ...` elisions. The ranges match xterm's terminfo entry (`infocmp xterm-256color | grep '^[ \t]*kf' | sort`) and must be reproduced verbatim:
  ```rust
  // kf13..kf24 = Shift+F1..F12 (xterm convention)
  static F_KEYS_SHIFTED: &[CapMapping] = &[
      CapMapping { cap: "kf13", named: NamedKey::F1,  mods: Modifiers::SHIFT, app_mode: false },
      CapMapping { cap: "kf14", named: NamedKey::F2,  mods: Modifiers::SHIFT, app_mode: false },
      CapMapping { cap: "kf15", named: NamedKey::F3,  mods: Modifiers::SHIFT, app_mode: false },
      CapMapping { cap: "kf16", named: NamedKey::F4,  mods: Modifiers::SHIFT, app_mode: false },
      CapMapping { cap: "kf17", named: NamedKey::F5,  mods: Modifiers::SHIFT, app_mode: false },
      CapMapping { cap: "kf18", named: NamedKey::F6,  mods: Modifiers::SHIFT, app_mode: false },
      CapMapping { cap: "kf19", named: NamedKey::F7,  mods: Modifiers::SHIFT, app_mode: false },
      CapMapping { cap: "kf20", named: NamedKey::F8,  mods: Modifiers::SHIFT, app_mode: false },
      CapMapping { cap: "kf21", named: NamedKey::F9,  mods: Modifiers::SHIFT, app_mode: false },
      CapMapping { cap: "kf22", named: NamedKey::F10, mods: Modifiers::SHIFT, app_mode: false },
      CapMapping { cap: "kf23", named: NamedKey::F11, mods: Modifiers::SHIFT, app_mode: false },
      CapMapping { cap: "kf24", named: NamedKey::F12, mods: Modifiers::SHIFT, app_mode: false },
  ];

  // kf25..kf36 = Ctrl+F1..F12 (xterm convention)
  static F_KEYS_CTRL: &[CapMapping] = &[
      CapMapping { cap: "kf25", named: NamedKey::F1,  mods: Modifiers::CONTROL, app_mode: false },
      CapMapping { cap: "kf26", named: NamedKey::F2,  mods: Modifiers::CONTROL, app_mode: false },
      CapMapping { cap: "kf27", named: NamedKey::F3,  mods: Modifiers::CONTROL, app_mode: false },
      CapMapping { cap: "kf28", named: NamedKey::F4,  mods: Modifiers::CONTROL, app_mode: false },
      CapMapping { cap: "kf29", named: NamedKey::F5,  mods: Modifiers::CONTROL, app_mode: false },
      CapMapping { cap: "kf30", named: NamedKey::F6,  mods: Modifiers::CONTROL, app_mode: false },
      CapMapping { cap: "kf31", named: NamedKey::F7,  mods: Modifiers::CONTROL, app_mode: false },
      CapMapping { cap: "kf32", named: NamedKey::F8,  mods: Modifiers::CONTROL, app_mode: false },
      CapMapping { cap: "kf33", named: NamedKey::F9,  mods: Modifiers::CONTROL, app_mode: false },
      CapMapping { cap: "kf34", named: NamedKey::F10, mods: Modifiers::CONTROL, app_mode: false },
      CapMapping { cap: "kf35", named: NamedKey::F11, mods: Modifiers::CONTROL, app_mode: false },
      CapMapping { cap: "kf36", named: NamedKey::F12, mods: Modifiers::CONTROL, app_mode: false },
  ];

  // kf37..kf48 = Ctrl+Shift+F1..F12 (xterm convention).
  // `Modifiers::CONTROL.union(Modifiers::SHIFT)` is const, so the static
  // initializer is valid at compile time.
  static F_KEYS_CTRL_SHIFT: &[CapMapping] = &[
      CapMapping { cap: "kf37", named: NamedKey::F1,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), app_mode: false },
      CapMapping { cap: "kf38", named: NamedKey::F2,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), app_mode: false },
      CapMapping { cap: "kf39", named: NamedKey::F3,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), app_mode: false },
      CapMapping { cap: "kf40", named: NamedKey::F4,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), app_mode: false },
      CapMapping { cap: "kf41", named: NamedKey::F5,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), app_mode: false },
      CapMapping { cap: "kf42", named: NamedKey::F6,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), app_mode: false },
      CapMapping { cap: "kf43", named: NamedKey::F7,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), app_mode: false },
      CapMapping { cap: "kf44", named: NamedKey::F8,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), app_mode: false },
      CapMapping { cap: "kf45", named: NamedKey::F9,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), app_mode: false },
      CapMapping { cap: "kf46", named: NamedKey::F10, mods: Modifiers::CONTROL.union(Modifiers::SHIFT), app_mode: false },
      CapMapping { cap: "kf47", named: NamedKey::F11, mods: Modifiers::CONTROL.union(Modifiers::SHIFT), app_mode: false },
      CapMapping { cap: "kf48", named: NamedKey::F12, mods: Modifiers::CONTROL.union(Modifiers::SHIFT), app_mode: false },
  ];

  // kf49..kf60 = Alt+F1..F12 (xterm convention)
  static F_KEYS_ALT: &[CapMapping] = &[
      CapMapping { cap: "kf49", named: NamedKey::F1,  mods: Modifiers::ALT, app_mode: false },
      CapMapping { cap: "kf50", named: NamedKey::F2,  mods: Modifiers::ALT, app_mode: false },
      CapMapping { cap: "kf51", named: NamedKey::F3,  mods: Modifiers::ALT, app_mode: false },
      CapMapping { cap: "kf52", named: NamedKey::F4,  mods: Modifiers::ALT, app_mode: false },
      CapMapping { cap: "kf53", named: NamedKey::F5,  mods: Modifiers::ALT, app_mode: false },
      CapMapping { cap: "kf54", named: NamedKey::F6,  mods: Modifiers::ALT, app_mode: false },
      CapMapping { cap: "kf55", named: NamedKey::F7,  mods: Modifiers::ALT, app_mode: false },
      CapMapping { cap: "kf56", named: NamedKey::F8,  mods: Modifiers::ALT, app_mode: false },
      CapMapping { cap: "kf57", named: NamedKey::F9,  mods: Modifiers::ALT, app_mode: false },
      CapMapping { cap: "kf58", named: NamedKey::F10, mods: Modifiers::ALT, app_mode: false },
      CapMapping { cap: "kf59", named: NamedKey::F11, mods: Modifiers::ALT, app_mode: false },
      CapMapping { cap: "kf60", named: NamedKey::F12, mods: Modifiers::ALT, app_mode: false },
  ];

  // kf61..kf63 = Alt+Shift+F1..F3 (xterm convention; ncurses terminfo
  // kfN namespace truncates at kf63, so Alt+Shift coverage stops at F3).
  // Any additional Alt+Shift F-keys would need a non-terminfo cross-check.
  static F_KEYS_ALT_SHIFT: &[CapMapping] = &[
      CapMapping { cap: "kf61", named: NamedKey::F1, mods: Modifiers::ALT.union(Modifiers::SHIFT), app_mode: false },
      CapMapping { cap: "kf62", named: NamedKey::F2, mods: Modifiers::ALT.union(Modifiers::SHIFT), app_mode: false },
      CapMapping { cap: "kf63", named: NamedKey::F3, mods: Modifiers::ALT.union(Modifiers::SHIFT), app_mode: false },
  ];

  // GAP discipline: ori_term MUST encode every combination this table
  // exercises. If encode_key returns a different byte string from the
  // terminfo declaration for any `kfN` in the tables above, the
  // encoder is authoritative for behavioral truth (see 08.4 backspace
  // caveat for the drift-resolution rule). File via `/add-bug`
  // immediately if the drift exists and treat as blocker per CLAUDE.md
  // broken-window policy.

  #[test]
  fn function_keys_ctrl_shift_match_terminfo() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      for mapping in F_KEYS_CTRL_SHIFT {
          assert_encoded_matches_terminfo(&env, mapping);
      }
  }

  #[test]
  fn function_keys_alt_match_terminfo() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      for mapping in F_KEYS_ALT {
          assert_encoded_matches_terminfo(&env, mapping);
      }
  }

  #[test]
  fn function_keys_alt_shift_match_terminfo() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      for mapping in F_KEYS_ALT_SHIFT {
          assert_encoded_matches_terminfo(&env, mapping);
      }
  }




  // The exact xterm-style modifier-to-kfN mapping is documented in
  // xterm's terminfo entry (infocmp xterm-256color | grep kf). Mirror
  // it here.

  #[test]
  fn function_keys_shift_match_terminfo() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      for mapping in F_KEYS_SHIFTED {
          assert_encoded_matches_terminfo(&env, mapping);
      }
  }

  #[test]
  fn function_keys_ctrl_match_terminfo() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      for mapping in F_KEYS_CTRL {
          assert_encoded_matches_terminfo(&env, mapping);
      }
  }
  ```

  Update the cap → modifier mapping table once the implementer has consulted `infocmp xterm-256color` to confirm the canonical xterm mapping.

---

## 08.3 Cursor key tests (rmkx + smkx modes)

**File(s):** `oriterm/src/key_encoding/terminfo_xcheck.rs` (extend the PREFERRED in-crate sibling file)

Cursor keys differ between normal mode (rmkx, sequences like `\E[A`) and application mode (smkx, sequences like `\EOA`). Test BOTH modes.

- [ ] Add to `terminfo_xcheck.rs`:
  ```rust
  /// Cursor keys in normal (rmkx) mode.
  /// Note: terminfo doesn't have separate caps for normal vs app mode
  /// for cursor keys — `kcub1` is the canonical "cursor back" cap.
  /// The trick is: the cap value matches what the terminal emits in
  /// APPLICATION mode (because that's how curses uses it). Normal-mode
  /// emission is the standard CSI form (`\E[A` etc.) and isn't a
  /// terminfo cap at all. So this test ALWAYS sets app_mode: true.
  static CURSOR_KEYS_APP: &[CapMapping] = &[
      CapMapping { cap: "kcub1", named: NamedKey::ArrowLeft,  mods: Modifiers::empty(), app_mode: true },
      CapMapping { cap: "kcud1", named: NamedKey::ArrowDown,  mods: Modifiers::empty(), app_mode: true },
      CapMapping { cap: "kcuf1", named: NamedKey::ArrowRight, mods: Modifiers::empty(), app_mode: true },
      CapMapping { cap: "kcuu1", named: NamedKey::ArrowUp,    mods: Modifiers::empty(), app_mode: true },
  ];


  #[test]
  fn cursor_keys_app_mode_match_terminfo() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      for mapping in CURSOR_KEYS_APP {
          assert_encoded_matches_terminfo(&env, mapping);
      }
  }

  /// Cursor keys in normal (rmkx) mode — verify encode_key produces
  /// the standard CSI sequence directly, not the application form.
  /// These are not validated against terminfo (terminfo only declares
  /// the app form) — instead they're hard-coded against the standard.
  #[test]
  fn cursor_keys_normal_mode_emit_csi() {
      let pairs: &[(NamedKey, &[u8])] = &[
          (NamedKey::ArrowUp,    b"\x1b[A"),
          (NamedKey::ArrowDown,  b"\x1b[B"),
          (NamedKey::ArrowRight, b"\x1b[C"),
          (NamedKey::ArrowLeft,  b"\x1b[D"),
      ];
      for (named, expected) in pairs {
          let key = Key::Named(*named);
          let input = KeyInput {
              key: &key,
              mods: Modifiers::empty(),
              mode: TermMode::empty(), // normal cursor mode, not APP_CURSOR
              text: None,
              location: KeyLocation::Standard,
              event_type: KeyEventType::Press,
              alternate_key: None,
          };
          let actual = encode_key(&input);
          assert_eq!(
              actual, *expected,
              "{:?} in normal mode produced {:?}, expected {:?}",
              named, actual, expected
          );
      }
  }
  ```


---

## 08.4 Editing/navigation key tests (kbs, khome, kend, kpp, knp, kdch1, kich1)

**File(s):** `oriterm/src/key_encoding/terminfo_xcheck.rs` (extend)

- [ ] Add to `terminfo_xcheck.rs`:
  ```rust
  static EDITING_KEYS: &[CapMapping] = &[
      CapMapping { cap: "kbs",   named: NamedKey::Backspace, mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "khome", named: NamedKey::Home,      mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kend",  named: NamedKey::End,       mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kpp",   named: NamedKey::PageUp,    mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "knp",   named: NamedKey::PageDown,  mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kdch1", named: NamedKey::Delete,    mods: Modifiers::empty(), app_mode: false },
      CapMapping { cap: "kich1", named: NamedKey::Insert,    mods: Modifiers::empty(), app_mode: false },
  ];


  #[test]
  fn editing_keys_match_terminfo() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      for mapping in EDITING_KEYS {
          assert_encoded_matches_terminfo(&env, mapping);
      }
  }
  ```

  **Backspace caveat:** `kbs` is special — it can be `^H` (0x08) or `^?` (0x7f) depending on terminal convention. ori_term's encoder must match whichever one the terminfo declares. If the test fails because the encoder emits `^H` but the terminfo says `^?`, the FIX is in the terminfo (Section 02), not the encoder — pick the one ori_term actually emits and document it. CLAUDE.md is clear that the canonical answer for "what does ori_term advertise" is the terminfo file, but the encoder is the source of behavioral truth. Resolve drift in favor of consistency: pick one, document it, fix the other side.

- [ ] **TPR checkpoint** — `/tpr-review` covering 08.1–08.4. Catches: `infocmp_query` parsing bugs (multi-line cap values), `decode_terminfo_string` edge cases, `KeyInput` field mismatches with the current `key_encoding::mod.rs`, modifier-to-kfN mapping wrong against xterm convention.

- [ ] Run all keyboard tests. The preferred in-crate sibling location puts these tests in the `oriterm` crate's unit-test target, so the `--test` flag selects by test-function-name prefix, not by integration target name:
  ```
  timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck
  ```
  Iterate until they all pass. Each failing test points to a divergence between encoder and terminfo — fix one or the other based on which represents the correct behavior.

  **Fallback command** (only if the integration-test fallback was taken): `timeout 150 cargo test -p oriterm --test keyboard_terminfo`.

- [ ] **Negative pin — encoder must refuse undeclared key sequences.** Add a test that constructs a `KeyInput` for a key whose terminfo cap is NOT declared in `extra/ori_term.info`, and asserts that either (a) `encode_key` returns an empty `Vec<u8>` (no encoding), or (b) `infocmp_query` returns `None` so the loop in `assert_encoded_matches_terminfo` short-circuits. This ensures the test harness does not silently "pass" when the cap table drifts out of sync with the terminfo source:
  ```rust
  #[test]
  fn infocmp_query_returns_none_for_cap_not_in_ori_term() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      // `kf64` is outside the xterm kfN namespace and MUST NOT be
      // declared by extra/ori_term.info. If this starts returning
      // Some(_), something added an unexpected cap — investigate.
      assert!(infocmp_query(&env, "ori_term", "kf64").is_none());
  }
  ```
  This is the semantic-pin test that ONLY passes when Section 08's design intent holds. If the encoder starts producing bytes for `kf64` without a terminfo declaration, this test must fail.

- [ ] **Determinism gate (10 reruns).** Key encoding is deterministic by construction, but the infocmp subprocess call and the TerminfoEnv temp-dir compile path are shared with Sections 02-07. Run the full `terminfo_xcheck` test group 10 times in a row:
  ```
  for i in $(seq 1 10); do
      timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck || break
  done
  ```
  All 10 must pass. Any failure → file `/add-bug` immediately and treat as a blocker.

- [ ] **Both `--test-threads` modes.** Run with `--test-threads=1` and `--test-threads=4`:
  ```
  timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck -- --test-threads=1
  timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck -- --test-threads=4
  ```
  Both must pass. Parallel runs surface `TerminfoEnv` temp-dir collision bugs (each call must use its own `tempfile::TempDir`).

---

## 08.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 08.N Completion Checklist

- [ ] **PREFERRED path taken:** `oriterm/src/lib.rs:17` stays as `pub(crate) mod key_encoding;` (NO visibility promotion). `#[cfg(test)] mod terminfo_xcheck;` declaration added at the bottom of `oriterm/src/key_encoding/mod.rs`.
- [ ] `oriterm/src/key_encoding/terminfo_xcheck.rs` exists as the new in-crate sibling test module, with `super::` imports for `encode_key`, `KeyInput`, `KeyEventType`, `Modifiers`
- [ ] (Fallback only — document only if taken:) If the preferred path is blocked, `oriterm/src/lib.rs:17` is promoted to `pub mod key_encoding;` and `oriterm/tests/keyboard_terminfo.rs` integration test target is created instead. This fallback is NOT taken unless explicitly documented with rationale in 08.R.
- [ ] `infocmp_query(env, term, cap) -> Option<String>` implemented in `oriterm_test_support`
- [ ] `decode_terminfo_string(s) -> Vec<u8>` implemented and unit-tested
- [ ] `function_keys_match_terminfo` test covers F1-F12 unmodified (kf1-kf12)
- [ ] `function_keys_shift_match_terminfo` test covers Shift+F1-F12 (kf13-kf24)
- [ ] `function_keys_ctrl_match_terminfo` test covers Ctrl+F1-F12 (kf25-kf36)
- [ ] `function_keys_ctrl_shift_match_terminfo` test covers Ctrl+Shift+F1-F12 (kf37-kf48)
- [ ] `function_keys_alt_match_terminfo` test covers Alt+F1-F12 (kf49-kf60)
- [ ] `function_keys_alt_shift_match_terminfo` test covers Alt+Shift+F1-F3 (kf61-kf63)
- [ ] `cursor_keys_app_mode_match_terminfo` test covers kcub1/kcud1/kcuf1/kcuu1
- [ ] `cursor_keys_normal_mode_emit_csi` test verifies normal-mode CSI encoding
- [ ] `editing_keys_match_terminfo` test covers kbs, khome, kend, kpp, knp, kdch1, kich1
- [ ] `infocmp_query_returns_none_for_cap_not_in_ori_term` negative pin test passes — asserts that querying an undeclared cap (`kf64`) returns `None`
- [ ] All tests pass deterministically (10 consecutive runs of `cargo test -p oriterm key_encoding::terminfo_xcheck`)
- [ ] Both `--test-threads=1` and `--test-threads=4` runs pass (surfaces `TerminfoEnv` tempdir collision bugs)
- [ ] All tests skip cleanly when `tic`/`infocmp` are unavailable
- [ ] Cross-compile for `x86_64-pc-windows-gnu` succeeds
- [ ] Any divergences between encoder output and terminfo declarations resolved (in favor of the encoder OR the terminfo, whichever is correct — document the choice)
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `timeout 150 ./test-all.sh` green
- [ ] Plan annotation cleanup
- [ ] All TPR checkpoint findings resolved (see `08.R`)
- [ ] **Plan sync**:
  - [ ] Section frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference table updated
  - [ ] `00-overview.md` Mission Success Criteria #11 ticked
  - [ ] `index.md` Section 08 updated
- [ ] `/tpr-review` final pass clean
- [ ] `/impl-hygiene-review last commit` final pass clean (after TPR)

**Exit Criteria:** `timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck` passes (preferred in-crate sibling path). Every function key across the full `kf1`-`kf63` terminfo namespace (F1-F12 base + Shift = kf13-kf24 + Ctrl = kf25-kf36 + Ctrl+Shift = kf37-kf48 + Alt = kf49-kf60 + Alt+Shift = kf61-kf63), every cursor key in app mode (kcub1/kcud1/kcuf1/kcuu1) AND normal-mode CSI encoding, and every editing key (kbs, khome, kend, kpp, knp, kdch1, kich1) round-trips between `key_encoding::encode_key` and the pinned terminfo. Any divergence found during section work has been resolved with documented rationale. 10 consecutive runs of the test set all pass (determinism gate). Cross-compile for `x86_64-pc-windows-gnu` succeeds with the test body runtime-skipping on Windows. The terminfo and the encoder agree byte-for-byte.
