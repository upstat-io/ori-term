---
section: "17"
title: "Kitty Keyboard Protocol"
status: not-started
reviewed: false
goal: "Drive every catalog row in `catalog/kitty-keyboard.md` from `implemented-unverified` to `verified`, and IMPLEMENT the missing PTY encoding side — Pass 1 confirmed kitty keyboard modes are PARSED but no PTY encoding exists. Also fix modifyOtherKeys (CSI > 4 m) and Win32 Input (mode 9001) which are stubs."
success_criteria:
  - "Every row in `catalog/kitty-keyboard.md` is `verified`"
  - "Kitty keyboard ENCODING implemented: a key press with the appropriate disambiguation modes set produces the expected byte sequence emitted via `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::KeyboardEvent })`"
  - "All 5 disambiguation modes verified: DISAMBIGUATE_ESC_CODES, REPORT_EVENT_TYPES, REPORT_ALTERNATE_KEYS, REPORT_ALL_KEYS_AS_ESC, REPORT_ASSOCIATED_TEXT"
  - "Mode push/pop stack verified: pushing modes onto the stack and popping them restores the previous state correctly (Pass 1 confirmed the stack exists, just no encoding)"
  - "modifyOtherKeys (CSI > 4 m) ENCODING implemented: mode 1 and mode 2 each produce the expected key encoding format (currently STUB per Pass 1, always reports disabled)"
  - "Win32 Input mode 9001 ENCODING implemented: terminal produces the Windows ConPTY input format when mode is enabled (currently STUB per Pass 1)"
  - "Cross-platform: kitty keyboard encoding works the same on macOS / Linux / Windows (the encoder is pure data; only the input pipeline differs)"
  - "All existing keyboard tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row**"
inspired_by:
  - "kitty source itself — `~/projects/reference_repos/console_repos/kitty/kitty/keys.py` for the encoding logic"
  - "sw.kovidgoyal.net/kitty/keyboard-protocol/ — public protocol documentation"
  - "ori_term existing `oriterm_core/src/term/handler/dcs.rs` (or wherever push_keyboard_mode lives) — current parsing surface"
depends_on: ["03", "16"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "17.1"
    title: "Implement kitty keyboard encoder (oriterm/src/key_encoding/kitty.rs)"
    status: not-started
  - id: "17.2"
    title: "Verify all 5 disambiguation modes via spec_chain"
    status: not-started
  - id: "17.3"
    title: "Verify mode push/pop stack semantics"
    status: not-started
  - id: "17.4"
    title: "Implement modifyOtherKeys (CSI > 4 m) encoding (mode 1 and mode 2)"
    status: not-started
  - id: "17.5"
    title: "Implement Win32 Input mode 9001 encoding"
    status: not-started
  - id: "17.6"
    title: "Cross-platform verification (encoder is pure data)"
    status: not-started
  - id: "17.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "17.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement: 17.3 (after kitty encoder + modes — covers .1-.3),
# 17.5 (after modifyOtherKeys + Win32 — covers .4-.5), final in 17.N
---

# Section 17: Kitty Keyboard Protocol

**Status:** Not Started
**Goal:** Verify every kitty keyboard catalog row + implement the missing encoding side.

**Success Criteria:** see frontmatter.

**Context:** Pass 1 confirmed kitty keyboard modes are PARSED (CSI > u, push/pop stack, all 5 disambiguation modes) but there is NO encoding side — the parsed modes set flags on the term, but no key press is encoded to PTY using those flags. This section implements the encoder. modifyOtherKeys and Win32 Input are similar stubs.

**Reference implementations:** see frontmatter.

**Depends on:** Section 16 (mouse encoding pattern established; kitty keyboard reuses similar Effect::Pty / encoder scaffolding).

---

## 17.1 Implement kitty keyboard encoder

**File(s):** `oriterm/src/key_encoding/kitty.rs` (new), sibling tests

- [ ] Create `oriterm/src/key_encoding/kitty.rs` with `encode_kitty_key(key, modifiers, event_type, mode_flags) -> Vec<u8>` function
- [ ] Implementation reads the active disambiguation modes from term state and produces the encoded bytes per kitty protocol spec
- [ ] Reference: `~/projects/reference_repos/console_repos/kitty/kitty/keys.py` `encode_key()` function
- [ ] Sibling tests: encode common keys (Enter, Tab, function keys, modified keys) under each mode combination
- [ ] **Validation**: encoder tests pass; bytes match kitty's reference output

---

## 17.2 Verify all 5 disambiguation modes via spec_chain

- [ ] Spec_chain test for each mode: enable mode, simulate key press, assert `Effect::Pty(PtyEffect::Write { kind: KeyboardEvent, bytes: ... })` matches expected
- [ ] Update catalog rows to `verified`

---

## 17.3 Verify mode push/pop stack semantics

- [ ] Spec_chain test: push mode A, push mode B, encode key (uses mode B), pop mode (back to A), encode key (uses A), pop mode (back to default)
- [ ] Stack overflow handling: push more modes than the stack supports, verify the documented behavior (truncate or error)
- [ ] Update catalog rows to `verified`
- [ ] **TPR checkpoint** — `/tpr-review` covering 17.1-17.3

---

## 17.4 Implement modifyOtherKeys encoding (mode 1 and mode 2)

**File(s):** `oriterm/src/key_encoding/modify_other_keys.rs` (new)

- [ ] Reference: xterm `ctlseqs.html` modifyOtherKeys section
- [ ] Implementation: when modifyOtherKeys mode 1 or 2 is enabled, encode modified keys (e.g., `Ctrl+A`, `Alt+Shift+B`) using the documented CSI format
- [ ] Spec_chain tests for both modes
- [ ] Update catalog rows for modifyOtherKeys to `verified`

---

## 17.5 Implement Win32 Input mode 9001 encoding

**File(s):** `oriterm/src/key_encoding/win32_input.rs` (new — guarded behind cross-platform consideration)

- [ ] Reference: Microsoft Console Virtual Terminal Sequences spec for mode 9001
- [ ] Implementation: when mode 9001 is enabled, encode key events in the Windows ConPTY input format
- [ ] Spec_chain tests
- [ ] Update catalog rows to `verified`
- [ ] **TPR checkpoint** — `/tpr-review` covering 17.4-17.5

---

## 17.6 Cross-platform verification

- [ ] The kitty/modifyOtherKeys/Win32 encoders are pure data — they take a key event + mode flags + return bytes. The cross-platform concern is the input pipeline (winit on each platform), not the encoder. Verify the encoder produces identical bytes on macOS/Linux/Windows for the same input.
- [ ] Run the encoder tests on each platform via `cargo test` (or trust the cross-compile gate in `./build-all.sh`)
- [ ] **Validation**: encoder tests pass cross-platform.

---

## 17.R Third Party Review Findings

- None.

---

## 17.N Completion Checklist

- [ ] Failing test matrix written FIRST
- [ ] **Matrix dimensions**: protocol (kitty/modifyOtherKeys/Win32) × disambiguation mode × key type × modifier combination
- [ ] **Semantic pin**: kitty encoder tests are the regression guard for the new implementation
- [ ] Kitty keyboard encoder implemented; all 5 modes verified
- [ ] Mode push/pop stack verified
- [ ] modifyOtherKeys mode 1 and 2 implemented
- [ ] Win32 Input mode 9001 implemented
- [ ] Cross-platform encoder tests pass
- [ ] All existing keyboard tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` + `index.md` updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Kitty keyboard, modifyOtherKeys, and Win32 Input encoders all exist and verified; every catalog row `verified`.
