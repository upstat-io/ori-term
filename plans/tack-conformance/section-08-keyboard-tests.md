---
section: "08"
title: "Keyboard / Function Key Tests"
status: complete
reviewed: true
needs_re_review_after: "05"
re_review_reason: "Section 05's Agent-1 / Agent-2 / Agent-3 review pass introduces a `cap_coverage_matrix` test (Section 05.5) that asserts every cap declared in `extra/ori_term.info` is exercised by at least one Section 05 / 06 / 08 scenario. Per Pivot 5 of /review-plan, the matrix uses an OWNER-PARTITIONED design: each consuming section owns its own `cap_coverage/section_NN.rs::CONTRIBUTION` with `covered` and `exempt` slices. Section 08 owns `cap_coverage/section_08.rs`. The keyboard-cap half of the matrix is split: kf1-kf63 (covered via `expand_kf_caps()` helper in `cap_coverage/mod.rs`) and the modified arrow / Home / End / editing key family kLFT/kRIT/kUP/kDN/kEND/kHOM/kIC/kDC/kNXT/kPRV with mod-param suffixes (covered via `expand_modified_key_caps()` helper) are exempted by the iterator-built expansion in `cap_coverage::exempt_caps()` — Section 08 MUST move those into `CONTRIBUTION.covered` (see subsection 08.6). The named cursor / editing keys (kcub1/kcud1/kcuf1/kcuu1, khome/kend/kpp/knp, kdch1/kich1, kbs) currently live in `cap_coverage/section_08.rs::CONTRIBUTION.exempt` — Section 08's subsection 08.6 MUST move them OUT of `exempt` and INTO `covered` once the keyboard tests land. EXCEPTION: `kmous` (mouse prefix \\E[M) does NOT go through key_encoding::encode_key and must stay in `exempt` with an updated reason — it is not a keyboard cap (cohesion review fix). Section 05.5's stale-exemption negative pin (caps appearing in BOTH any section's `covered` AND any section's `exempt`) fires loudly if a cap appears in both, forcing the cleanup."
goal: "Add keyboard capability tests in the `oriterm` crate that exercise the real `key_encoding::encode_key` pipeline against the function-key sequences declared in extra/ori_term.info. For each terminfo cap (kf1-kf63, kcub1, kcud1, kcuf1, kcuu1, khome, kend, kpp, knp, kdch1, kich1, kbs, AND the modified-key family kLFT/kRIT/kUP/kDN/kHOM/kEND/kIC/kDC/kNXT/kPRV with modifier suffixes 3-7 plus kind/kri), the test (a) reads the expected sequence from the compiled terminfo via infocmp, (b) constructs a KeyInput for the corresponding key+modifier combo, (c) calls encode_key, and (d) asserts the produced bytes match the terminfo declaration. This is the only section in oriterm rather than oriterm_core because it tests the application-layer key encoder."
success_criteria:
  - "`oriterm/src/key_encoding/terminfo_xcheck/` in-crate directory test module exists with `mod.rs` + `function_keys.rs` + `navigation.rs` + `modified_keys.rs` (preferred path, no visibility change). Fallback `oriterm/tests/keyboard_terminfo.rs` integration test target only if the preferred path is blocked and documented in 08.R."
  - "`infocmp_dump(env, term) -> Option<HashMap<String, String>>` and `infocmp_query(env, term, cap_name) -> Option<String>` helpers exist in oriterm_test_support — infocmp_dump parses one infocmp invocation into a HashMap for efficient multi-cap lookup; infocmp_query is a convenience wrapper"
  - "Test for each ori_term-declared function key (kf1-kf12 minimum, ideally up through kf63 with modifiers): expected sequence from infocmp matches encode_key output"
  - "Test for each cursor key (kcub1, kcud1, kcuf1, kcuu1) in normal mode (rmkx) AND application mode (smkx)"
  - "Test for kbs (backspace), khome, kend, kpp (pgup), knp (pgdn), kdch1 (delete), kich1 (insert)"
  - "Test for Home/End in normal mode emitting CSI sequences (parity with cursor_keys_normal_mode_emit_csi)"
  - "Test for all modified-key caps declared in extra/ori_term.info: kLFT/kRIT/kUP/kDN/kHOM/kEND/kIC/kDC/kNXT/kPRV (base + suffixes 3-7) plus kind/kri (62 caps total)"
  - "All tests skip cleanly when tic/infocmp are unavailable (Windows) — runtime gate, not cfg"
  - "All tests use the pinned ori_term terminfo via TerminfoEnv (not the host xterm-256color)"
  - "`timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck` passes on Linux (preferred in-crate path). Fallback command `cargo test -p oriterm --test keyboard_terminfo` only applies if the fallback path is taken (documented in 08.R)."
  - "All terminfo_xcheck tests run deterministically across 10 consecutive runs AND pass with both --test-threads=1 and --test-threads=4"
  - "cap_coverage/section_08.rs::CONTRIBUTION.covered includes all tested caps; kmous remains in exempt (mouse prefix, not key encoding) with updated reason"
  - "Satisfies mission criterion: 'Keyboard/function key capability tests exist in oriterm crate exercising real key encoding pipeline'"
inspired_by:
  - "ori_term key_encoding (oriterm/src/key_encoding/mod.rs:116 — encode_key entry point that this section validates)"
  - "ori_term key_encoding tests (oriterm/src/key_encoding/tests.rs — existing per-key encoding tests, expanded here to validate against terminfo)"
  - "Section 02 TerminfoEnv (plans/tack-conformance/section-02-terminfo-provisioning.md — provides the compiled ori_term terminfo to read sequences from)"
  - "ncurses infocmp(1) man page — capability extraction syntax"
depends_on: ["01", "02", "05"]
third_party_review:
  status: resolved
  updated: 2026-04-09
sections:
  - id: "08.1"
    title: "infocmp_dump/infocmp_query helpers in oriterm_test_support"
    status: complete
  - id: "08.2"
    title: "Function key tests (kf1-kf63)"
    status: complete
  - id: "08.3"
    title: "Cursor key tests (rmkx + smkx modes)"
    status: complete
  - id: "08.4"
    title: "Editing/navigation key tests (kbs, khome, kend, kpp, knp, kdch1, kich1)"
    status: complete
  - id: "08.5"
    title: "Modified key tests (kLFT, kRIT, kUP, kDN, kHOM, kEND, kIC, kDC, kNXT, kPRV with modifier suffixes)"
    status: complete
  - id: "08.6"
    title: "Cap-coverage extension (section_08.rs sync)"
    status: complete
  - id: "08.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "08.N"
    title: "Completion Checklist"
    status: complete
---

# Section 08: Keyboard / Function Key Tests

**Status:** Complete
**Goal:** Validate that ori_term's `key_encoding::encode_key` produces byte sequences that exactly match what `extra/ori_term.info` declares for each function/cursor/editing key. This catches the silent class of bugs where the terminfo claims `kf1=\EOP` but encode_key emits `\E[11~` (or vice versa) — the symptom is "F1 doesn't work in vim under ori_term", and the root cause is a divergence between the terminfo declaration and the application's actual key encoder. After this section, every key in the terminfo is mechanically verified to round-trip.

**Success Criteria:**

- [x] **Preferred in-crate directory path:** `oriterm/src/key_encoding/terminfo_xcheck/` directory module exists with `mod.rs`, `function_keys.rs`, `navigation.rs`, `modified_keys.rs` as a `#[cfg(test)] mod terminfo_xcheck;` submodule of `oriterm/src/key_encoding/mod.rs`, leaving `pub(crate) mod key_encoding;` unchanged in `oriterm/src/lib.rs:17`. Fallback `oriterm/tests/keyboard_terminfo.rs` integration test target is only created if the in-crate path is blocked (documented in 08.R).
- [x] `oriterm_test_support::infocmp_query(env: &TerminfoEnv, term: &str, cap: &str) -> Option<String>` exists and returns the raw cap value (or None if unset)
- [x] All function keys F1-F12 in normal mode (no modifiers) tested
- [x] All function keys F1-F12 in shift, control, and alt modifier combinations tested where ori_term encodes them
- [x] All cursor keys (Up/Down/Left/Right) tested in both rmkx (normal) and smkx (application) modes
- [x] All editing keys (Backspace, Delete, Insert, Home, End, PgUp, PgDn) tested
- [x] Home/End normal mode CSI encoding verified (parity with cursor_keys_normal_mode_emit_csi)
- [x] All 62 modified-key caps tested (kLFT/kRIT/kUP/kDN/kHOM/kEND/kIC/kDC/kNXT/kPRV base + suffixes 3-7, plus kind/kri)
- [x] All infocmp-dependent tests skip cleanly when `tic`/`infocmp` are unavailable; pure encoder tests run unconditionally on all platforms
- [x] `timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck` passes (preferred in-crate path)
- [x] 10 consecutive runs of the `terminfo_xcheck` test group all pass (determinism gate)
- [x] cap_coverage/section_08.rs CONTRIBUTION updated: all tested caps in `covered`, only `kmous` in `exempt`
- [x] Satisfies mission criterion #13 (keyboard tests)

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
- **ori_term** `oriterm/src/key_encoding/tests.rs`: existing per-key encoding tests. This section adds a NEW in-crate directory test submodule (`terminfo_xcheck/`) that validates the same encoder against the terminfo as the source of truth — complementary to the existing unit tests, not a replacement.
- **Section 02** `plans/tack-conformance/section-02-terminfo-provisioning.md`: `TerminfoEnv::compile()` is the source of the pinned terminfo we read from.
- **ncurses infocmp(1) man page**: capability extraction syntax.

**Depends on:** Section 01 (PtySession framework — actually only `oriterm_test_support` is needed; PtySession itself is not used here), Section 02 (TerminfoEnv), **Section 05** (cap_coverage_matrix extension contract + `expand_kf_caps` / `expand_modified_key_caps` SSOT helpers — see Section 05.5b for the contract details).

**PREFERRED APPROACH — In-crate sibling tests (no visibility change):** `oriterm/src/key_encoding/tests.rs` already exists as a 1801-line sibling test module with full access to the module's items via `super::`. New keyboard-terminfo cross-check tests belong inside that existing sibling file (or a new submodule under it), NOT in a separate `tests/` integration target. Benefits:

1. **Zero public API surface change.** `key_encoding` stays `pub(crate) mod key_encoding` — encoder internals remain hidden from downstream users.
2. **`super::` imports work directly.** No visibility gymnastics needed — the test module already has `use super::{encode_key, KeyInput, Modifiers, KeyEventType};`-style access to every name in `mod.rs`, `legacy.rs`, and `kitty.rs`.
3. **Dev-deps already reach sibling tests.** `oriterm_test_support` is added to `oriterm`'s `[dev-dependencies]` in Section 01.4, which makes it visible to BOTH integration tests and sibling `#[cfg(test)]` modules. There is no dependency advantage to the integration test target.
4. **Respects the `.claude/rules/test-organization.md` sibling-tests convention.** This is how every other test in `oriterm` is organized.

**File layout (preferred — directory module):** The combined test content (6 F-key tables totaling 63 entries, 7 editing keys, 4 cursor keys, ~62 modified key caps in loops, `CapMapping` struct, helpers, ~12 test functions, doc comments) will exceed the 500-line source file limit from CLAUDE.md. Therefore, `terminfo_xcheck` MUST be a directory module from the start, not a single file:

```
oriterm/src/key_encoding/
    mod.rs                          (existing — add #[cfg(test)] mod terminfo_xcheck;)
    tests.rs                        (existing sibling tests)
    terminfo_xcheck/
        mod.rs                      (CapMapping struct, assert_encoded_matches_terminfo helper,
                                     suffix_to_mods, modified_base_to_named, shared imports)
        function_keys.rs            (F_KEYS_BASE/SHIFTED/CTRL/CTRL_SHIFT/ALT/ALT_SHIFT tables,
                                     function_keys_*_match_terminfo tests)
        navigation.rs               (CURSOR_KEYS_APP, EDITING_KEYS tables,
                                     cursor_keys_app_mode_match_terminfo,
                                     cursor_keys_normal_mode_emit_csi,
                                     editing_keys_match_terminfo,
                                     editing_keys_normal_mode_emit_csi tests)
        modified_keys.rs            (modified_keys_match_terminfo test,
                                     infocmp_query_returns_none_for_cap_not_in_ori_term negative pin)
```

Add `#[cfg(test)] mod terminfo_xcheck;` at the bottom of `oriterm/src/key_encoding/mod.rs`. Each submodule file uses `use super::super::{encode_key, KeyInput, KeyEventType, Modifiers};` to reach the parent `key_encoding` module (two levels up from `terminfo_xcheck/function_keys.rs` to `key_encoding/mod.rs`). The `terminfo_xcheck/mod.rs` re-exports the shared helpers and declares `mod function_keys; mod navigation; mod modified_keys;`.

With this layout, the visibility of `key_encoding` does NOT change — `pub(crate) mod key_encoding;` stays as-is.

**FALLBACK — Integration test target (only if the in-crate approach is infeasible):** if, for some reason, the in-crate sibling approach does not work (e.g., cyclical dev-dep that forces the test target to live elsewhere), fall back to `oriterm/tests/keyboard_terminfo.rs` as an integration test target. This path requires changing `oriterm/src/lib.rs:17` from `pub(crate) mod key_encoding;` to `pub mod key_encoding;` so the integration test can import `encode_key`, `KeyInput`, `KeyEventType`, and `Modifiers`. This widens the public API surface — every `encode_key` caller in the stable API becomes something downstream crates could theoretically invoke. Only take this path if the preferred in-crate approach is blocked.

The plan picks the PREFERRED path (in-crate directory module). All subsection checklists below use `oriterm/src/key_encoding/terminfo_xcheck/` as the file target, with `oriterm/tests/keyboard_terminfo.rs` listed as the fallback file for reference.

**Cross-platform skip discipline:** Tests that invoke `infocmp_dump`, `infocmp_query`, or `TerminfoEnv::compile()` MUST begin with `if !tic_available() || !infocmp_available() { return; }` so Windows native (no ncurses) compiles AND runs cleanly. Use runtime gating via `tic_available()` / `infocmp_available()` from `oriterm_test_support`, NOT `#[cfg(unix)]` — per CLAUDE.md "every `#[cfg(target_os = "...")]` block must have counterparts for all supported targets. If a feature cannot be implemented on a platform, it must degrade gracefully with a compile-time `cfg` gate, not a runtime panic." In this case, the TEST must compile on every platform and runtime-skip on Windows — never `#[cfg(unix)]`.

**EXCEPTION — pure encoder tests:** Tests that do NOT call any external tool (infocmp, tic) and instead hard-code expected byte sequences — specifically `cursor_keys_normal_mode_emit_csi` and `editing_keys_normal_mode_emit_csi` — MUST NOT have a tic/infocmp gate. These tests exercise `encode_key` against known-correct byte literals and MUST run on Windows too. They validate the encoder itself, not the terminfo round-trip.

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

The table-driven pattern the original draft used (`const F_KEYS_BASE: &[CapMapping]`) still works if `CapMapping` stores only owned/plain data (cap name, `NamedKey` variant, `Modifiers` bitflags, `TermMode` for terminal mode bits), and the test loop constructs the `KeyInput` per-iteration from the owning data.

**TDD ordering discipline:** For each subsection, write the test functions FIRST (they will fail because the helpers/tables don't exist yet), then implement the helpers/tables to make them pass. Specifically: 08.1 writes `decode_terminfo_string` unit tests first, then the implementation. 08.2-08.5 write the test functions referencing the tables before populating the tables. This ensures every test function is a genuine failing-test-first TDD artifact, not a post-hoc rubber stamp.

---

## 08.1 infocmp_dump/infocmp_query helpers in oriterm_test_support

**File(s):** `crates/oriterm_test_support/src/terminfo/mod.rs` (extend Section 02's module)

`infocmp_dump` parses a single `infocmp -A <dir> -1 <term>` invocation into a complete `HashMap<String, String>` of cap name to raw terminfo-encoded value. `infocmp_query` is a convenience wrapper that spawns a dump and extracts one cap. Test bodies that need multiple lookups should call `infocmp_dump` once and use `.get()` on the map.

- [x] Extend `crates/oriterm_test_support/src/terminfo/mod.rs`:
  ```rust
  use std::collections::HashMap;

  /// Parse a full infocmp dump into a cap name → raw value map.
  ///
  /// Invokes `infocmp -A <env_dir> -1 <term>` ONCE and parses every
  /// cap declaration into a `HashMap`. Boolean caps (e.g., `am,`) map
  /// to an empty string. Numeric caps (e.g., `colors#256,`) map to the
  /// numeric string. String caps (e.g., `kf1=\EOP,`) map to the
  /// encoded value string (still terminfo-encoded — call
  /// `decode_terminfo_string` on the values as needed).
  ///
  /// Returns `None` if `infocmp` is not available or exits non-zero.
  /// Callers must gate on `infocmp_available()` first.
  ///
  /// This is the preferred entry point for tests that validate
  /// multiple caps — call once, look up many.
  #[must_use]
  pub fn infocmp_dump(env: &TerminfoEnv, term: &str) -> Option<HashMap<String, String>> {
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
      let mut map = HashMap::new();
      for line in text.lines() {
          let trimmed = line.trim_start();
          // Skip entry header lines (start at column 0, contain `|`).
          if !line.starts_with(|c: char| c.is_whitespace()) {
              continue;
          }
          // Strip trailing comma.
          let trimmed = trimmed.trim_end_matches(',');
          if trimmed.is_empty() {
              continue;
          }
          // String cap: name=value
          if let Some((name, value)) = trimmed.split_once('=') {
              map.insert(name.to_string(), value.to_string());
          // Numeric cap: name#value
          } else if let Some((name, value)) = trimmed.split_once('#') {
              map.insert(name.to_string(), value.to_string());
          // Boolean cap: just the name
          } else {
              map.insert(trimmed.to_string(), String::new());
          }
      }
      Some(map)
  }

  /// Extract a single capability's value from a compiled terminfo entry.
  ///
  /// Convenience wrapper around [`infocmp_dump`] for callers that only
  /// need one cap. For tests that validate many caps, prefer calling
  /// `infocmp_dump` once and using `.get()` on the returned map.
  ///
  /// Returns `None` if the cap is not declared or `infocmp` is
  /// unavailable.
  #[must_use]
  pub fn infocmp_query(env: &TerminfoEnv, term: &str, cap: &str) -> Option<String> {
      infocmp_dump(env, term)?.get(cap).cloned()
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

- [x] Add sibling tests at `crates/oriterm_test_support/src/terminfo/tests.rs`:
  ```rust
  use super::{infocmp_dump, infocmp_query, decode_terminfo_string};

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

  #[test]
  fn infocmp_dump_returns_populated_map() {
      use crate::{tic_available, infocmp_available};
      if !tic_available() || !infocmp_available() { return; }
      let env = super::TerminfoEnv::compile();
      let caps = infocmp_dump(&env, "ori_term")
          .expect("infocmp dump should succeed for ori_term");
      // The map should contain known caps from extra/ori_term.info.
      assert!(caps.contains_key("kf1"), "dump should contain kf1");
      assert!(caps.contains_key("am"), "dump should contain boolean cap am");
      assert!(caps.contains_key("colors"), "dump should contain numeric cap colors");
      // Verify it does NOT contain made-up caps.
      assert!(!caps.contains_key("completely_made_up_xyz"));
  }
  ```

- [x] Re-export from `lib.rs`:
  ```rust
  pub use terminfo::{decode_terminfo_string, infocmp_dump, infocmp_query, TerminfoEnv};
  ```

---

## 08.2 Function key tests (kf1-kf63)

**File(s) — PREFERRED:** `oriterm/src/key_encoding/terminfo_xcheck/mod.rs` + `function_keys.rs` (NEW directory submodule, in-crate sibling test), `oriterm/src/key_encoding/mod.rs` (add `#[cfg(test)] mod terminfo_xcheck;` at bottom)

**File(s) — FALLBACK only if in-crate blocked:** `oriterm/tests/keyboard_terminfo.rs` (integration test target) + `oriterm/src/lib.rs:17` visibility change.

The new submodule reads each `kfN` cap from the pinned terminfo, constructs a `KeyInput` for the corresponding F-key + modifier combo, calls `encode_key`, and asserts the produced bytes match the terminfo declaration.

- [x] **Add submodule declaration.** At the bottom of `oriterm/src/key_encoding/mod.rs`, alongside the existing `#[cfg(test)] mod tests;` declaration (line 228), add:
  ```rust
  #[cfg(test)]
  mod terminfo_xcheck;
  ```
  The file already has `#[cfg(test)] mod tests;` -- two `#[cfg(test)] mod` declarations in the same file is valid Rust. Place the new declaration immediately after the existing one. Verify `pub(crate) mod key_encoding;` in `oriterm/src/lib.rs:17` stays UNCHANGED -- the in-crate approach does not require the visibility promotion.

- [x] **Create directory module structure.** Per the file layout above (Finding 5 — 500-line split), create `oriterm/src/key_encoding/terminfo_xcheck/` as a directory with `mod.rs`, `function_keys.rs`, `navigation.rs`, `modified_keys.rs`. The `mod.rs` declares `mod function_keys; mod navigation; mod modified_keys;` and contains the shared `CapMapping` struct, `assert_encoded_matches_terminfo` helper, and shared imports. Each leaf file contains its domain-specific tables and test functions.

- [x] **Super-import discipline.** With the directory module layout:
  - `terminfo_xcheck/mod.rs` uses `use super::{encode_key, KeyInput, KeyEventType, Modifiers};` — one `super::` reaches `key_encoding/mod.rs`.
  - Leaf files (`function_keys.rs`, `navigation.rs`, `modified_keys.rs`) use `use super::{CapMapping, assert_encoded_matches_terminfo};` to reach shared items in `terminfo_xcheck/mod.rs`, and `use super::super::{encode_key, KeyInput, KeyEventType, Modifiers};` to reach `key_encoding/mod.rs` items (or re-export them from `terminfo_xcheck/mod.rs` with `pub(super) use super::{...};` so leaf files just use `super::`).
  - No `use oriterm::key_encoding::...` — the files ARE inside `key_encoding`, so `super::` is the correct path.
  - For items from `oriterm_core` (`TermMode`), use the full crate path `use oriterm_core::TermMode;`.
  - For items from `oriterm_test_support` (`TerminfoEnv`, `infocmp_dump`, `decode_terminfo_string`, `tic_available`, `infocmp_available`), use `use oriterm_test_support::{...};` — the dev-dependency added in Section 01.4 makes the crate visible inside `#[cfg(test)]` modules anywhere in `oriterm`.

- [x] Create `oriterm/src/key_encoding/terminfo_xcheck/mod.rs`:
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

  // Submodules — each contains domain-specific tables and test functions.
  mod function_keys;
  mod modified_keys;
  mod navigation;

  use std::collections::HashMap;

  use super::{encode_key, KeyEventType, KeyInput, Modifiers};
  use oriterm_core::TermMode;
  use oriterm_test_support::{
      decode_terminfo_string, infocmp_available, infocmp_dump,
      tic_available, TerminfoEnv,
  };
  use winit::keyboard::{Key, KeyLocation, NamedKey};

  // Re-export key_encoding items for leaf submodules to use via `super::`.
  pub(super) use super::{encode_key, KeyEventType, KeyInput, Modifiers};

  /// One terminfo cap → key mapping. Fields are plain data so the
  /// table can be `static`; the test loop constructs the borrowing
  /// `KeyInput` per-iteration from owning `Key::Named(...)` locals.
  #[derive(Copy, Clone)]
  pub(super) struct CapMapping {
      /// Terminfo cap name (e.g., `kf1`, `kcub1`).
      cap: &'static str,
      /// Named key (F1-F24, ArrowLeft, Home, etc.) — owned, not a reference.
      named: NamedKey,
      /// Modifier bitset.
      mods: Modifiers,
      /// Terminal mode bits to set when encoding this key. Use
      /// `TermMode::APP_CURSOR` for cursor/Home/End keys in application
      /// mode, `TermMode::APP_KEYPAD` for numpad keys, or
      /// `TermMode::empty()` for normal mode. This replaces the
      /// previous `app_mode: bool` which fused both flags.
      term_mode: TermMode,
  }

  /// Assert that encode_key output matches the terminfo declaration.
  ///
  /// Accepts a pre-parsed infocmp dump (`HashMap`) so each test
  /// function invokes infocmp exactly ONCE rather than once per cap.
  /// Panics on missing cap — for a pinned SSOT terminfo, a missing
  /// cap is a test infrastructure bug, not a skip reason.
  pub(super) fn assert_encoded_matches_terminfo(
      caps: &HashMap<String, String>,
      mapping: &CapMapping,
  ) {
      let cap_str = caps
          .get(mapping.cap)
          .unwrap_or_else(|| panic!(
              "SSOT violation: cap '{}' not found in infocmp dump of pinned ori_term \
               terminfo. Every cap in the test table MUST be declared in \
               extra/ori_term.info. If this cap was intentionally removed, \
               remove it from the test table too.",
              mapping.cap,
          ));
      let expected = decode_terminfo_string(cap_str);

      // Own the Key on the stack so the &'a Key borrow in KeyInput is valid.
      let key = Key::Named(mapping.named);

      // Construct KeyInput explicitly — no Default impl; KeyInput<'a>
      // holds borrows so every field must be provided. The TermMode
      // bits are carried directly from the mapping — no bool → bitflag
      // translation layer.
      let input = KeyInput {
          key: &key,
          mods: mapping.mods,
          mode: mapping.term_mode,
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
  ```

  **The remaining code in this subsection goes in `function_keys.rs`**, not `mod.rs`. The `mod.rs` contains only the shared types (`CapMapping`), helpers (`assert_encoded_matches_terminfo`, `suffix_to_mods`, `modified_base_to_named`), re-exports, and submodule declarations. All F-key tables and test functions belong in `function_keys.rs`.

  Create `oriterm/src/key_encoding/terminfo_xcheck/function_keys.rs`:
  ```rust
  //! Function key terminfo cross-check tests (kf1-kf63).

  use oriterm_core::TermMode;
  use oriterm_test_support::{
      infocmp_available, infocmp_dump, tic_available, TerminfoEnv,
  };
  use winit::keyboard::NamedKey;

  use super::{assert_encoded_matches_terminfo, CapMapping, Modifiers};

  /// Function key mapping table — F1-F12, no modifiers, normal mode.
  static F_KEYS_BASE: &[CapMapping] = &[
      CapMapping { cap: "kf1",  named: NamedKey::F1,  mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "kf2",  named: NamedKey::F2,  mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "kf3",  named: NamedKey::F3,  mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "kf4",  named: NamedKey::F4,  mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "kf5",  named: NamedKey::F5,  mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "kf6",  named: NamedKey::F6,  mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "kf7",  named: NamedKey::F7,  mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "kf8",  named: NamedKey::F8,  mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "kf9",  named: NamedKey::F9,  mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "kf10", named: NamedKey::F10, mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "kf11", named: NamedKey::F11, mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "kf12", named: NamedKey::F12, mods: Modifiers::empty(), term_mode: TermMode::empty() },
  ];

  #[test]
  fn function_keys_match_terminfo() {
      if !tic_available() || !infocmp_available() {
          eprintln!("tic or infocmp not installed, skipping function_keys_match_terminfo");
          return;
      }
      let env = TerminfoEnv::compile();
      let caps = infocmp_dump(&env, "ori_term").expect("infocmp dump");
      let mut tested = 0usize;
      for mapping in F_KEYS_BASE {
          assert_encoded_matches_terminfo(&caps, mapping);
          tested += 1;
      }
      assert_eq!(
          tested,
          F_KEYS_BASE.len(),
          "count pin: expected to test {} caps, only tested {}",
          F_KEYS_BASE.len(),
          tested,
      );
  }
  ```


  **Note on `KeyInput` field set:** the exact fields and their defaults may evolve. The implementer must verify against `oriterm/src/key_encoding/mod.rs:80` at the time of writing. `KeyInput` does NOT derive `Default` — every field must be provided explicitly. `KeyInput<'a>` holds borrows, so the test must own `Key::Named(...)` values on the stack and take `&key` references. Application keypad/cursor mode is a `TermMode` bitflag, not a separate `KeyInput` field.

  **Note on `NamedKey::F13`-`F24`:** if ori_term encodes keys above F12, add those mappings too. Inspect `oriterm/src/key_encoding/legacy.rs` and `kitty.rs` for the supported range. The terminfo declares kf1-kf63 historically — many of those (kf13-kf63) are modified F1-F12 sequences (`kf13 = Shift+F1`, etc.), not separate physical keys.

- [x] Add modified F-key tests (Shift+F1, Ctrl+F1, etc.). Each table is fully enumerated — NO `// ...` elisions. The ranges match xterm's terminfo entry (`infocmp xterm-256color | grep '^[ \t]*kf' | sort`) and must be reproduced verbatim:
  ```rust
  // kf13..kf24 = Shift+F1..F12 (xterm convention)
  static F_KEYS_SHIFTED: &[CapMapping] = &[
      CapMapping { cap: "kf13", named: NamedKey::F1,  mods: Modifiers::SHIFT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf14", named: NamedKey::F2,  mods: Modifiers::SHIFT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf15", named: NamedKey::F3,  mods: Modifiers::SHIFT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf16", named: NamedKey::F4,  mods: Modifiers::SHIFT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf17", named: NamedKey::F5,  mods: Modifiers::SHIFT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf18", named: NamedKey::F6,  mods: Modifiers::SHIFT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf19", named: NamedKey::F7,  mods: Modifiers::SHIFT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf20", named: NamedKey::F8,  mods: Modifiers::SHIFT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf21", named: NamedKey::F9,  mods: Modifiers::SHIFT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf22", named: NamedKey::F10, mods: Modifiers::SHIFT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf23", named: NamedKey::F11, mods: Modifiers::SHIFT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf24", named: NamedKey::F12, mods: Modifiers::SHIFT, term_mode: TermMode::empty() },
  ];

  // kf25..kf36 = Ctrl+F1..F12 (xterm convention)
  static F_KEYS_CTRL: &[CapMapping] = &[
      CapMapping { cap: "kf25", named: NamedKey::F1,  mods: Modifiers::CONTROL, term_mode: TermMode::empty() },
      CapMapping { cap: "kf26", named: NamedKey::F2,  mods: Modifiers::CONTROL, term_mode: TermMode::empty() },
      CapMapping { cap: "kf27", named: NamedKey::F3,  mods: Modifiers::CONTROL, term_mode: TermMode::empty() },
      CapMapping { cap: "kf28", named: NamedKey::F4,  mods: Modifiers::CONTROL, term_mode: TermMode::empty() },
      CapMapping { cap: "kf29", named: NamedKey::F5,  mods: Modifiers::CONTROL, term_mode: TermMode::empty() },
      CapMapping { cap: "kf30", named: NamedKey::F6,  mods: Modifiers::CONTROL, term_mode: TermMode::empty() },
      CapMapping { cap: "kf31", named: NamedKey::F7,  mods: Modifiers::CONTROL, term_mode: TermMode::empty() },
      CapMapping { cap: "kf32", named: NamedKey::F8,  mods: Modifiers::CONTROL, term_mode: TermMode::empty() },
      CapMapping { cap: "kf33", named: NamedKey::F9,  mods: Modifiers::CONTROL, term_mode: TermMode::empty() },
      CapMapping { cap: "kf34", named: NamedKey::F10, mods: Modifiers::CONTROL, term_mode: TermMode::empty() },
      CapMapping { cap: "kf35", named: NamedKey::F11, mods: Modifiers::CONTROL, term_mode: TermMode::empty() },
      CapMapping { cap: "kf36", named: NamedKey::F12, mods: Modifiers::CONTROL, term_mode: TermMode::empty() },
  ];

  // kf37..kf48 = Ctrl+Shift+F1..F12 (xterm convention).
  // `Modifiers::CONTROL.union(Modifiers::SHIFT)` is const, so the static
  // initializer is valid at compile time.
  static F_KEYS_CTRL_SHIFT: &[CapMapping] = &[
      CapMapping { cap: "kf37", named: NamedKey::F1,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
      CapMapping { cap: "kf38", named: NamedKey::F2,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
      CapMapping { cap: "kf39", named: NamedKey::F3,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
      CapMapping { cap: "kf40", named: NamedKey::F4,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
      CapMapping { cap: "kf41", named: NamedKey::F5,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
      CapMapping { cap: "kf42", named: NamedKey::F6,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
      CapMapping { cap: "kf43", named: NamedKey::F7,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
      CapMapping { cap: "kf44", named: NamedKey::F8,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
      CapMapping { cap: "kf45", named: NamedKey::F9,  mods: Modifiers::CONTROL.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
      CapMapping { cap: "kf46", named: NamedKey::F10, mods: Modifiers::CONTROL.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
      CapMapping { cap: "kf47", named: NamedKey::F11, mods: Modifiers::CONTROL.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
      CapMapping { cap: "kf48", named: NamedKey::F12, mods: Modifiers::CONTROL.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
  ];

  // kf49..kf60 = Alt+F1..F12 (xterm convention)
  static F_KEYS_ALT: &[CapMapping] = &[
      CapMapping { cap: "kf49", named: NamedKey::F1,  mods: Modifiers::ALT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf50", named: NamedKey::F2,  mods: Modifiers::ALT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf51", named: NamedKey::F3,  mods: Modifiers::ALT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf52", named: NamedKey::F4,  mods: Modifiers::ALT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf53", named: NamedKey::F5,  mods: Modifiers::ALT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf54", named: NamedKey::F6,  mods: Modifiers::ALT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf55", named: NamedKey::F7,  mods: Modifiers::ALT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf56", named: NamedKey::F8,  mods: Modifiers::ALT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf57", named: NamedKey::F9,  mods: Modifiers::ALT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf58", named: NamedKey::F10, mods: Modifiers::ALT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf59", named: NamedKey::F11, mods: Modifiers::ALT, term_mode: TermMode::empty() },
      CapMapping { cap: "kf60", named: NamedKey::F12, mods: Modifiers::ALT, term_mode: TermMode::empty() },
  ];

  // kf61..kf63 = Alt+Shift+F1..F3 (xterm convention; ncurses terminfo
  // kfN namespace truncates at kf63, so Alt+Shift coverage stops at F3).
  // Any additional Alt+Shift F-keys would need a non-terminfo cross-check.
  static F_KEYS_ALT_SHIFT: &[CapMapping] = &[
      CapMapping { cap: "kf61", named: NamedKey::F1, mods: Modifiers::ALT.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
      CapMapping { cap: "kf62", named: NamedKey::F2, mods: Modifiers::ALT.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
      CapMapping { cap: "kf63", named: NamedKey::F3, mods: Modifiers::ALT.union(Modifiers::SHIFT), term_mode: TermMode::empty() },
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
      let caps = infocmp_dump(&env, "ori_term").expect("infocmp dump");
      let mut tested = 0usize;
      for mapping in F_KEYS_CTRL_SHIFT {
          assert_encoded_matches_terminfo(&caps, mapping);
          tested += 1;
      }
      assert_eq!(tested, F_KEYS_CTRL_SHIFT.len());
  }

  #[test]
  fn function_keys_alt_match_terminfo() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      let caps = infocmp_dump(&env, "ori_term").expect("infocmp dump");
      let mut tested = 0usize;
      for mapping in F_KEYS_ALT {
          assert_encoded_matches_terminfo(&caps, mapping);
          tested += 1;
      }
      assert_eq!(tested, F_KEYS_ALT.len());
  }

  #[test]
  fn function_keys_alt_shift_match_terminfo() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      let caps = infocmp_dump(&env, "ori_term").expect("infocmp dump");
      let mut tested = 0usize;
      for mapping in F_KEYS_ALT_SHIFT {
          assert_encoded_matches_terminfo(&caps, mapping);
          tested += 1;
      }
      assert_eq!(tested, F_KEYS_ALT_SHIFT.len());
  }

  #[test]
  fn function_keys_shift_match_terminfo() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      let caps = infocmp_dump(&env, "ori_term").expect("infocmp dump");
      let mut tested = 0usize;
      for mapping in F_KEYS_SHIFTED {
          assert_encoded_matches_terminfo(&caps, mapping);
          tested += 1;
      }
      assert_eq!(tested, F_KEYS_SHIFTED.len());
  }

  #[test]
  fn function_keys_ctrl_match_terminfo() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      let caps = infocmp_dump(&env, "ori_term").expect("infocmp dump");
      let mut tested = 0usize;
      for mapping in F_KEYS_CTRL {
          assert_encoded_matches_terminfo(&caps, mapping);
          tested += 1;
      }
      assert_eq!(tested, F_KEYS_CTRL.len());
  }
  ```

  Update the cap → modifier mapping table once the implementer has consulted `infocmp xterm-256color` to confirm the canonical xterm mapping.

---

## 08.3 Cursor key tests (rmkx + smkx modes)

**File(s):** `oriterm/src/key_encoding/terminfo_xcheck/navigation.rs` (extend the PREFERRED in-crate sibling file)

Cursor keys differ between normal mode (rmkx, sequences like `\E[A`) and application mode (smkx, sequences like `\EOA`). Test BOTH modes.

- [x] Add to `terminfo_xcheck/navigation.rs`:
  ```rust
  /// Cursor keys in normal (rmkx) mode.
  /// Note: terminfo doesn't have separate caps for normal vs app mode
  /// for cursor keys — `kcub1` is the canonical "cursor back" cap.
  /// The trick is: the cap value matches what the terminal emits in
  /// APPLICATION mode (because that's how curses uses it). Normal-mode
  /// emission is the standard CSI form (`\E[A` etc.) and isn't a
  /// terminfo cap at all. So this test ALWAYS sets term_mode: TermMode::APP_CURSOR.
  static CURSOR_KEYS_APP: &[CapMapping] = &[
      CapMapping { cap: "kcub1", named: NamedKey::ArrowLeft,  mods: Modifiers::empty(), term_mode: TermMode::APP_CURSOR },
      CapMapping { cap: "kcud1", named: NamedKey::ArrowDown,  mods: Modifiers::empty(), term_mode: TermMode::APP_CURSOR },
      CapMapping { cap: "kcuf1", named: NamedKey::ArrowRight, mods: Modifiers::empty(), term_mode: TermMode::APP_CURSOR },
      CapMapping { cap: "kcuu1", named: NamedKey::ArrowUp,    mods: Modifiers::empty(), term_mode: TermMode::APP_CURSOR },
  ];


  #[test]
  fn cursor_keys_app_mode_match_terminfo() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      let caps = infocmp_dump(&env, "ori_term").expect("infocmp dump");
      let mut tested = 0usize;
      for mapping in CURSOR_KEYS_APP {
          assert_encoded_matches_terminfo(&caps, mapping);
          tested += 1;
      }
      assert_eq!(tested, CURSOR_KEYS_APP.len());
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

**File(s):** `oriterm/src/key_encoding/terminfo_xcheck/navigation.rs` (extend)

- [x] Add to `terminfo_xcheck/navigation.rs`:
  ```rust
  static EDITING_KEYS: &[CapMapping] = &[
      CapMapping { cap: "kbs",   named: NamedKey::Backspace, mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "khome", named: NamedKey::Home,      mods: Modifiers::empty(), term_mode: TermMode::APP_CURSOR },
      CapMapping { cap: "kend",  named: NamedKey::End,       mods: Modifiers::empty(), term_mode: TermMode::APP_CURSOR },
      CapMapping { cap: "kpp",   named: NamedKey::PageUp,    mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "knp",   named: NamedKey::PageDown,  mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "kdch1", named: NamedKey::Delete,    mods: Modifiers::empty(), term_mode: TermMode::empty() },
      CapMapping { cap: "kich1", named: NamedKey::Insert,    mods: Modifiers::empty(), term_mode: TermMode::empty() },
  ];


  #[test]
  fn editing_keys_match_terminfo() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      let caps = infocmp_dump(&env, "ori_term").expect("infocmp dump");
      let mut tested = 0usize;
      for mapping in EDITING_KEYS {
          assert_encoded_matches_terminfo(&caps, mapping);
          tested += 1;
      }
      assert_eq!(tested, EDITING_KEYS.len());
  }
  ```

  **Backspace caveat:** `kbs` is special — it can be `^H` (0x08) or `^?` (0x7f) depending on terminal convention. ori_term's encoder must match whichever one the terminfo declares. If the test fails because the encoder emits `^H` but the terminfo says `^?`, the FIX is in the terminfo (Section 02), not the encoder — pick the one ori_term actually emits and document it. CLAUDE.md is clear that the canonical answer for "what does ori_term advertise" is the terminfo file, but the encoder is the source of behavioral truth. Resolve drift in favor of consistency: pick one, document it, fix the other side.

  **Normal-mode Home/End parity (REQUIRED):** Like cursor keys (08.3), Home and End have different encodings in normal mode (`\E[H` / `\E[F`) vs application mode (`\EOH` / `\EOF`). The terminfo declares the application-mode form (`khome`/`kend`), so the `EDITING_KEYS` table correctly uses `TermMode::APP_CURSOR`. Add the following test to `terminfo_xcheck/navigation.rs`:

- [x] Add `editing_keys_normal_mode_emit_csi` test:
  ```rust
  /// Home/End in normal (non-APP_CURSOR) mode — verify encode_key
  /// produces the standard CSI sequence, not the application SS3 form.
  /// Pure encoder test — no infocmp/tic dependency, runs on all platforms.
  #[test]
  fn editing_keys_normal_mode_emit_csi() {
      let pairs: &[(NamedKey, &[u8])] = &[
          (NamedKey::Home, b"\x1b[H"),
          (NamedKey::End,  b"\x1b[F"),
      ];
      for (named, expected) in pairs {
          let key = Key::Named(*named);
          let input = KeyInput {
              key: &key,
              mods: Modifiers::empty(),
              mode: TermMode::empty(), // normal mode, not APP_CURSOR
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

## 08.5 Modified key tests (kLFT, kRIT, kUP, kDN, kHOM, kEND, kIC, kDC, kNXT, kPRV with modifier suffixes)

**File(s):** `oriterm/src/key_encoding/terminfo_xcheck/modified_keys.rs` (extend)

`extra/ori_term.info` lines 159-181 declare modified arrow, Home, End, and editing key caps with modifier suffixes. These use the CSI letter or tilde form with an xterm modifier parameter (e.g., `kLFT5=\E[1;5D` = Ctrl+Left, `kDC3=\E[3;3~` = Alt+Delete). The encoder produces these via the `mod_param > 0` branch in `legacy.rs:127-145`. This subsection validates that the encoder matches the terminfo for the full modified-key family.

The modifier suffix → `Modifiers` mapping follows xterm convention:
- Suffix 2 = Shift (no suffix on base cap name, e.g., `kLFT` = Shift+Left)
- Suffix 3 = Alt
- Suffix 4 = Alt+Shift
- Suffix 5 = Ctrl
- Suffix 6 = Ctrl+Shift
- Suffix 7 = Ctrl+Alt

The two special caps `kind` and `kri` are ncurses aliases for Shift+Down and Shift+Up (equivalent to `kDN` and `kUP` respectively).

- [x] Add modifier mapping helper to `terminfo_xcheck/mod.rs` (shared across leaf files):
  ```rust
  /// Map xterm modifier suffix to Modifiers bitset.
  /// The base caps (kLFT, kRIT, etc.) without a digit suffix use
  /// modifier param 2 = Shift.
  pub(super) fn suffix_to_mods(suffix: Option<u8>) -> Modifiers {
      match suffix {
          None | Some(2) => Modifiers::SHIFT,
          Some(3) => Modifiers::ALT,
          Some(4) => Modifiers::ALT.union(Modifiers::SHIFT),
          Some(5) => Modifiers::CONTROL,
          Some(6) => Modifiers::CONTROL.union(Modifiers::SHIFT),
          Some(7) => Modifiers::CONTROL.union(Modifiers::ALT),
          _ => unreachable!("invalid xterm modifier suffix"),
      }
  }

  /// Map a modified-key base name to its NamedKey.
  pub(super) fn modified_base_to_named(base: &str) -> NamedKey {
      match base {
          "kLFT" => NamedKey::ArrowLeft,
          "kRIT" => NamedKey::ArrowRight,
          "kUP"  => NamedKey::ArrowUp,
          "kDN"  => NamedKey::ArrowDown,
          "kHOM" => NamedKey::Home,
          "kEND" => NamedKey::End,
          "kIC"  => NamedKey::Insert,
          "kDC"  => NamedKey::Delete,
          "kNXT" => NamedKey::PageDown,
          "kPRV" => NamedKey::PageUp,
          _ => unreachable!("unknown modified-key base: {base}"),
      }
  }
  ```

- [x] Add modified key cross-check test to `terminfo_xcheck/modified_keys.rs`:
  ```rust
  #[test]
  fn modified_keys_match_terminfo() {
      if !tic_available() || !infocmp_available() { return; }
      let env = TerminfoEnv::compile();
      let caps = infocmp_dump(&env, "ori_term").expect("infocmp dump");

      let bases = [
          "kLFT", "kRIT", "kUP", "kDN", "kHOM", "kEND",
          "kIC", "kDC", "kNXT", "kPRV",
      ];

      let mut tested = 0usize;
      for base in bases {
          // Base cap (e.g., kLFT) = Shift variant (modifier param 2).
          let val = caps.get(base).unwrap_or_else(|| panic!(
              "SSOT violation: modified-key base cap '{}' not found in infocmp dump",
              base,
          ));
          let expected = decode_terminfo_string(val);
          let key = Key::Named(modified_base_to_named(base));
          let input = KeyInput {
              key: &key,
              mods: suffix_to_mods(None),
              mode: TermMode::empty(),
              text: None,
              location: KeyLocation::Standard,
              event_type: KeyEventType::Press,
              alternate_key: None,
          };
          let actual = encode_key(&input);
          assert_eq!(
              actual, expected,
              "encode_key for modified cap {base} produced {:?} but terminfo says {:?}",
              actual, expected,
          );
          tested += 1;

          // Suffixed caps (e.g., kLFT3, kLFT4, ..., kLFT7).
          for suffix in 3..=7u8 {
              let cap_name = format!("{base}{suffix}");
              let val = caps.get(cap_name.as_str()).unwrap_or_else(|| panic!(
                  "SSOT violation: modified-key cap '{}' not found in infocmp dump",
                  cap_name,
              ));
              let expected = decode_terminfo_string(val);
              let key = Key::Named(modified_base_to_named(base));
              let input = KeyInput {
                  key: &key,
                  mods: suffix_to_mods(Some(suffix)),
                  mode: TermMode::empty(),
                  text: None,
                  location: KeyLocation::Standard,
                  event_type: KeyEventType::Press,
                  alternate_key: None,
              };
              let actual = encode_key(&input);
              assert_eq!(
                  actual, expected,
                  "encode_key for modified cap {cap_name} produced {:?} but terminfo says {:?}",
                  actual, expected,
              );
              tested += 1;
          }
      }

      // Special ncurses aliases: kind = Shift+Down, kri = Shift+Up.
      for (cap, named) in [("kind", NamedKey::ArrowDown), ("kri", NamedKey::ArrowUp)] {
          let val = caps.get(cap).unwrap_or_else(|| panic!(
              "SSOT violation: ncurses alias cap '{}' not found in infocmp dump",
              cap,
          ));
          let expected = decode_terminfo_string(val);
          let key = Key::Named(named);
          let input = KeyInput {
              key: &key,
              mods: Modifiers::SHIFT,
              mode: TermMode::empty(),
              text: None,
              location: KeyLocation::Standard,
              event_type: KeyEventType::Press,
              alternate_key: None,
          };
          let actual = encode_key(&input);
          assert_eq!(
              actual, expected,
              "encode_key for modified cap {cap} produced {:?} but terminfo says {:?}",
              actual, expected,
          );
          tested += 1;
      }

      // Count pin: 10 bases * (1 base + 5 suffixes) + 2 aliases = 62.
      assert_eq!(
          tested, 62,
          "count pin: expected to test 62 modified-key caps, only tested {}",
          tested,
      );
  }
  ```

  **Note:** Modified arrow/Home/End caps all use the CSI form with modifier parameter (`\E[1;{mod}{term}` for letter keys, `\E[{num};{mod}~` for tilde keys). The encoder produces this via the `mod_param > 0` branches at `legacy.rs:127-129` (letter) and `legacy.rs:141-142` (tilde). `TermMode` is `empty()` for all modified keys because the modifier parameter forces CSI form regardless of `APP_CURSOR` state (when `mod_param > 0`, letter keys always use CSI per `legacy.rs:127`).

**Post-08.5 verification gate (runs after ALL test subsections 08.1-08.5 land):**

- [x] **TPR checkpoint** — `/tpr-review` covering 08.1-08.5. Catches: `infocmp_dump` parsing bugs (multi-line cap values, boolean vs string vs numeric disambiguation), `decode_terminfo_string` edge cases, `KeyInput` field mismatches with the current `key_encoding::mod.rs`, modifier-to-kfN mapping wrong against xterm convention, modified-key suffix-to-Modifiers mapping errors.

- [x] **Run all keyboard tests.** The preferred in-crate sibling location puts these tests in the `oriterm` crate's unit-test target, so the `--test` flag selects by test-function-name prefix, not by integration target name:
  ```
  timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck
  ```
  Iterate until they all pass. Each failing test points to a divergence between encoder and terminfo -- fix one or the other based on which represents the correct behavior.

  **Fallback command** (only if the integration-test fallback was taken): `timeout 150 cargo test -p oriterm --test keyboard_terminfo`.

- [x] **Negative pin -- infocmp_query returns None for undeclared caps.** Add a test that asserts `infocmp_query` returns `None` for a cap name that `extra/ori_term.info` does NOT declare. This ensures the infocmp parsing helper itself is correct (it doesn't hallucinate caps) and that the `assert_encoded_matches_terminfo` loop's short-circuit path fires for absent caps. Note: this does NOT assert that `encode_key` refuses to produce bytes for keys without terminfo cap names -- `encode_key` validly produces sequences for key combinations beyond the `kf1`-`kf63` terminfo namespace (e.g., modifier combos that xterm encodes but terminfo cannot name):
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
  This is the semantic-pin test that ONLY passes when the terminfo source does not declare `kf64`. It validates the infocmp parsing path, not the encoder's key coverage.

- [x] **Determinism gate (10 reruns).** Key encoding is deterministic by construction, but the infocmp subprocess call and the TerminfoEnv temp-dir compile path are shared with Sections 02-07. Run the full `terminfo_xcheck` test group 10 times in a row:
  ```
  for i in $(seq 1 10); do
      timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck || break
  done
  ```
  All 10 must pass. Any failure -> file `/add-bug` immediately and treat as a blocker.

- [x] **Both `--test-threads` modes.** Run with `--test-threads=1` and `--test-threads=4`:
  ```
  timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck -- --test-threads=1
  timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck -- --test-threads=4
  ```
  Both must pass. Parallel runs surface `TerminfoEnv` temp-dir collision bugs (each call must use its own `tempfile::TempDir`).

- [x] **Debug + release parity.** Run the test group in release mode to catch any optimizer-sensitive divergence:
  ```
  timeout 150 cargo test -p oriterm --release key_encoding::terminfo_xcheck
  ```
  Must produce the same results as the debug run. Key encoding is pure computation with no debug_assert-only paths, so this is a safety net, not an expected divergence source.

---

## 08.6 Cap-coverage extension (section_08.rs sync)

**File(s):** `crates/oriterm_test_support/src/tack_framework/cap_coverage/section_08.rs`

After subsections 08.1-08.5 land, the cap-coverage matrix must be updated to reflect that Section 08's tests now cover the keyboard caps. This subsection is the lockstep sync point between landing the tests and updating the coverage bookkeeping.

**CRITICAL — `kmous` disposition:** `kmous=\E[M` is the mouse encoding prefix. It does NOT go through `key_encoding::encode_key` and cannot be cross-checked by the keyboard terminfo_xcheck tests. Section 08 is NOT the right place to test `kmous` -- mouse encoding lives in a different subsystem (mouse reporting / `oriterm_core` or `oriterm_mux` input path). `kmous` must be moved from `section_08.rs::CONTRIBUTION.exempt` to whichever section actually tests mouse input encoding, OR it must remain in `exempt` with an updated reason pointing to the correct future owner. Do NOT move it to `covered` without an actual test -- that would create a false coverage claim.

- [x] **Move named cursor + editing keys to `covered`.** Update `section_08.rs::CONTRIBUTION`:
  - Move `kcub1`, `kcud1`, `kcuf1`, `kcuu1`, `khome`, `kend`, `kpp`, `knp`, `kdch1`, `kich1`, `kbs` from `exempt` to `covered`. These are all tested by 08.3 (cursor), 08.4 (editing), and 08.5 (modified keys via their base forms).
  - Do NOT move `kmous` to `covered` -- update its `exempt` reason to: `"mouse encoding prefix \\E[M — not testable via key_encoding::encode_key; belongs to mouse input subsystem (future roadmap section)"`.

- [x] **Move kf1-kf63 + modified-key family to `covered`.** Remove the iterator-built exemptions from `cap_coverage::exempt_caps()` in `crates/oriterm_test_support/src/tack_framework/cap_coverage/mod.rs` and move coverage tracking to `section_08.rs` using a programmatic approach:
  - Add a `pub fn covered_caps_08() -> Vec<String>` method to `section_08.rs` that returns the union of the static `CONTRIBUTION.covered` entries (cursor + editing keys from the first bullet) with `expand_kf_caps()` and `expand_modified_key_caps()` results. This avoids a 136-entry static slice.
  - Update `cap_coverage::covered_caps()` in `mod.rs` to call `section_08::covered_caps_08()` and insert those entries alongside the static `CONTRIBUTION.covered` entries from other sections. The current `for contrib in ALL_CONTRIBUTIONS { for cap in contrib.covered { ... } }` loop stays for sections 05/06; section 08's programmatic extension is added after it.
  - Remove the `expand_kf_caps()` and `expand_modified_key_caps()` calls from `exempt_caps()` — they are no longer exemptions.
  - Verify the stale-exemption negative pin does not fire (no cap in both covered and exempt across any section).

- [x] **Run `tack_cap_coverage_matrix` and confirm zero stale exemptions.** The negative pin fires on any cap in both `covered` and `exempt` across all sections:
  ```
  timeout 150 cargo test -p oriterm_core --test tack tack_cap_coverage_matrix
  ```

- [x] **Update `section_08.rs` doc comment** to reflect that Section 08 has landed and `covered` is populated.

---

## 08.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-08-001][low]` `oriterm/src/key_encoding/terminfo_xcheck/navigation.rs:11`, `oriterm/src/key_encoding/terminfo_xcheck/navigation.rs:69` — The new Section 08 navigation test module uses decorative banner comments (`// ── ... ──`), which violates `.claude/rules/code-hygiene.md`'s "Never: Decorative banners" rule.
  Resolved: Fixed on 2026-04-09. Replaced decorative `// ── ... ──` banners with plain `// Section name` section labels.

---

## 08.N Completion Checklist

- [x] **PREFERRED path taken:** `oriterm/src/lib.rs:17` stays as `pub(crate) mod key_encoding;` (NO visibility promotion). `#[cfg(test)] mod terminfo_xcheck;` declaration added at the bottom of `oriterm/src/key_encoding/mod.rs`.
- [x] `oriterm/src/key_encoding/terminfo_xcheck/` directory module exists with `mod.rs` (shared types + helpers), `function_keys.rs`, `navigation.rs`, `modified_keys.rs`. Each file under 500 lines. `super::` imports for `encode_key`, `KeyInput`, `KeyEventType`, `Modifiers`.
- [x] **Count pins in every table-driven test** — each test asserts `tested == TABLE.len()` (or a known constant) after the loop. No silent skips. `assert_encoded_matches_terminfo` panics on missing caps (not returns early).
- [x] (Fallback NOT taken — preferred in-crate path works.) `oriterm/src/lib.rs:17` unchanged.
- [x] `infocmp_dump(env, term) -> Option<HashMap<String, String>>` implemented in `oriterm_test_support` (parse-once approach)
- [x] `infocmp_query(env, term, cap) -> Option<String>` implemented as convenience wrapper around `infocmp_dump`
- [x] `decode_terminfo_string(s) -> Vec<u8>` implemented and unit-tested
- [x] `function_keys_match_terminfo` test covers F1-F12 unmodified (kf1-kf12)
- [x] `function_keys_shift_match_terminfo` test covers Shift+F1-F12 (kf13-kf24)
- [x] `function_keys_ctrl_match_terminfo` test covers Ctrl+F1-F12 (kf25-kf36)
- [x] `function_keys_ctrl_shift_match_terminfo` test covers Ctrl+Shift+F1-F12 (kf37-kf48)
- [x] `function_keys_alt_match_terminfo` test covers Alt+F1-F12 (kf49-kf60)
- [x] `function_keys_alt_shift_match_terminfo` test covers Alt+Shift+F1-F3 (kf61-kf63)
- [x] `cursor_keys_app_mode_match_terminfo` test covers kcub1/kcud1/kcuf1/kcuu1
- [x] `cursor_keys_normal_mode_emit_csi` test verifies normal-mode CSI encoding
- [x] `editing_keys_match_terminfo` test covers kbs, khome (app mode), kend (app mode), kpp, knp, kdch1, kich1
- [x] `editing_keys_normal_mode_emit_csi` test verifies Home emits `\E[H` and End emits `\E[F` in normal mode (parity with `cursor_keys_normal_mode_emit_csi`)
- [x] `modified_keys_match_terminfo` test covers all 62 modified-key caps (kLFT/kRIT/kUP/kDN/kHOM/kEND/kIC/kDC/kNXT/kPRV base + suffixes 3-7, plus kind/kri)
- [x] `infocmp_query_returns_none_for_cap_not_in_ori_term` negative pin test passes — asserts that querying an undeclared cap (`kf64`) returns `None`
- [x] **Cap-coverage extension (subsection 08.6).** All items in 08.6 complete: named cursor/editing keys moved to `covered`, kf1-kf63 + modified-key family moved to `covered`, `kmous` remains in `exempt` with updated reason, `tack_cap_coverage_matrix` green, `section_08.rs` doc comment updated.
- [x] All tests pass deterministically (10 consecutive runs of `cargo test -p oriterm key_encoding::terminfo_xcheck`)
- [x] Both `--test-threads=1` and `--test-threads=4` runs pass (surfaces `TerminfoEnv` tempdir collision bugs)
- [x] Debug + release parity: `cargo test -p oriterm --release key_encoding::terminfo_xcheck` produces same results as debug
- [x] Infocmp-dependent tests skip cleanly when `tic`/`infocmp` are unavailable (runtime gate, not `#[cfg(unix)]`)
- [x] Pure encoder tests (`cursor_keys_normal_mode_emit_csi`, `editing_keys_normal_mode_emit_csi`) run unconditionally on all platforms including Windows — no tic/infocmp gate
- [x] Cross-compile for `x86_64-pc-windows-gnu` succeeds
- [x] No divergences found — encoder output and terminfo declarations agree byte-for-byte on all caps
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `timeout 150 ./test-all.sh` green
- [x] Plan annotation cleanup
- [x] All TPR checkpoint findings resolved (see `08.R` — TPR-08-001 fixed)
- [x] **Plan sync**:
  - [x] Section frontmatter `status` → `complete`
  - [x] `00-overview.md` Quick Reference table updated
  - [x] `00-overview.md` Mission Success Criteria #13 (keyboard tests) ticked
  - [x] `index.md` Section 08 status updated
  - [x] `section-09-verification.md` depends_on includes `"08"` (already present — verified)
- [x] `/tpr-review` final pass clean (1 low finding — TPR-08-001 decorative banners — fixed and resolved)
- [x] `/impl-hygiene-review last commit` final pass clean (after TPR). Findings: 3 algorithmic duplication (extracted `setup_terminfo_env`, `run_cap_mapping_test`, `encode_named_key` helpers), 1 BLOAT (function_keys.rs 513→439 lines), 1 decorative banner fixed (gpu/prepare/tests.rs). All resolved.

**Exit Criteria:** `timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck` passes (preferred in-crate sibling path). Every function key across the full `kf1`-`kf63` terminfo namespace (F1-F12 base + Shift = kf13-kf24 + Ctrl = kf25-kf36 + Ctrl+Shift = kf37-kf48 + Alt = kf49-kf60 + Alt+Shift = kf61-kf63), every cursor key in app mode (kcub1/kcud1/kcuf1/kcuu1) AND normal-mode CSI encoding, every editing key (kbs, khome, kend, kpp, knp, kdch1, kich1), and every modified key (kLFT/kRIT/kUP/kDN/kHOM/kEND/kIC/kDC/kNXT/kPRV with modifier suffixes 3-7, plus kind/kri) round-trips between `key_encoding::encode_key` and the pinned terminfo. Any divergence found during section work has been resolved with documented rationale. 10 consecutive runs of the test set all pass (determinism gate), both `--test-threads=1` and `--test-threads=4` pass (parallelism gate). Cross-compile for `x86_64-pc-windows-gnu` succeeds with the test body runtime-skipping on Windows. The terminfo and the encoder agree byte-for-byte. Cap-coverage matrix (`tack_cap_coverage_matrix`) passes with all Section 08 keyboard caps in `covered` and only `kmous` remaining in `exempt` (mouse prefix, not key encoding).
