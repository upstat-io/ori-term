---
bug: "BUG-08-013"
title: "Numpad keys produce no output — Enter, digits, operators all dead"
severity: "high"
status: in-progress
goal: "Numpad keys (digits 0-9, `+`, `-`, `*`, `/`, `.`, Enter) produce the correct bytes on the PTY in every terminal mode (normal, Kitty, APP_KEYPAD, LINE_FEED_NEW_LINE) regardless of whether winit populates `KeyEvent::text`."
success_criteria:
  - "Matrix tests cover numpad digits / operators / decimal / Enter, with and without `text` populated, in both legacy and Kitty dispatch paths."
  - "Negative pin: the exact `enc_numpad(Key::Character(N), empty, no_mode(), text=None)` input that today returns empty bytes instead returns `b\"N\"`."
  - "`./test-all.sh`, `./build-all.sh`, `./clippy-all.sh` green on both host and Windows cross-compile."
subsystem: "oriterm/src/key_encoding/legacy.rs, oriterm/src/key_encoding/kitty.rs, oriterm/src/key_encoding/mod.rs"
found: "2026-04-14"
source: "manual"
third_party_review:
  status: none
  updated: null
---

# Fix: BUG-08-013 — Numpad keys produce no output

**Status:** In Progress
**Severity:** high
**Goal:** Numpad keys produce the correct PTY bytes regardless of whether winit populates `KeyEvent::text`. Current encoder silently drops numpad Character keys when `text` is `None`, which is the platform-observed symptom.

**Success Criteria:**
- [ ] `enc_numpad(Key::Character("1".into()), empty, no_mode(), text=None)` returns `b"1"` (fallback on the logical-key character).
- [ ] `enc_numpad(Key::Character("+".into()), empty, no_mode(), text=None)` returns `b"+"`.
- [ ] `enc_numpad(Key::Named(Enter), empty, no_mode(), text=None)` returns `b"\r"` (already works via `encode_simple_named`, regression-pin it).
- [ ] Same matrix with `text=Some(...)` populated — prefers `text` over `Key::Character(ch)`.
- [ ] Kitty dispatch path: `send-as-text` branch falls back on `Key::Character(ch)` when `text` is `None`.
- [ ] APP_KEYPAD mode unaffected (already works — pin it to catch regressions).

**Context:** Pressing any numpad key with NumLock on (digits, operators, decimal, Enter) produces no bytes on the PTY in practice. Investigation reveals `legacy::encode_character` and `kitty::encode_kitty` both fall back to `input.text` as the final byte source for unmodified Character keys; when `text` is `None` — which can happen on some winit backends / Ctrl-with-numpad / future platform quirks — the encoder returns `Vec::new()` and nothing reaches the shell. `NumpadEnter` maps to `Key::Named(NamedKey::Enter)` and is safe because `encode_simple_named` returns `\r` unconditionally, so the bug is scoped to numpad Character keys.

---

## 1. Root Cause Analysis

- **Symptom**: Numpad digits, `+`, `-`, `*`, `/`, `.` send no bytes to the PTY. Shell sees nothing.
- **Proximate cause**: Both legacy and Kitty encoders treat `input.text` as the sole byte source for single-char Character keys without modifiers:
  - `legacy::encode_character` (oriterm/src/key_encoding/legacy.rs:247): `text.map_or_else(Vec::new, |t| t.as_bytes().to_vec())` — returns empty if `text` is None.
  - `kitty::encode_kitty` send-as-text branch (oriterm/src/key_encoding/kitty.rs:113-116): same pattern — returns empty if `text` is None.
- **Root cause**: The encoder assumes winit always populates `KeyEvent::text` for numpad character keys. This holds on most platforms with NumLock on, but is not guaranteed: some backends, certain modifier combos (Ctrl-with-numpad on Windows), and some WSL / remote-display configurations deliver `text: None` with `logical_key: Key::Character("1")`, `location: Numpad`. The encoder has no fallback, so the character in `Key::Character(ch)` — which is always present — is ignored.
- **Blast radius**: All numpad character keys when `text` is unpopulated for any reason. The same fallback gap exists in the Kitty protocol path when `DISAMBIGUATE_ESC_CODES` is active and a numpad digit falls into the `should_send_as_text` branch. `NumpadEnter` is unaffected because it routes through `encode_simple_named(Enter)` which returns `b"\r"` without consulting `text`.
- **Affected files**:
  - `oriterm/src/key_encoding/legacy.rs` — add `Key::Character(ch)` fallback in `encode_character` plain-text branch.
  - `oriterm/src/key_encoding/kitty.rs` — add the same fallback in the `send-as-text` branch and in the multi-char early-return.
  - `oriterm/src/key_encoding/tests/application_keypad.rs` — invert `numpad_5_no_app_keypad` (currently pins the broken behavior) and add the full numpad matrix with both `text=None` and `text=Some(...)`.
  - `oriterm/src/key_encoding/tests/mod.rs` — extend `enc_numpad` helper shape (or add a sibling) so tests can drive the `text` parameter.

**Reference implementations**:
- **Alacritty** (`alacritty/src/input/keyboard.rs:83-93`): when `should_build_sequence` is false, alacritty uses `key.text_with_all_modifiers()` directly and sends the bytes. It does NOT fall back on `Key::Character(ch)` for legacy mode — which means alacritty has the same latent gap. But in DISAMBIGUATE mode (`try_build_numpad`, lines 430-468) it maps numpad character keys explicitly to Kitty codepoints 57399-57413, bypassing `text` entirely. So alacritty's defense is mode-dependent; ours needs to work in both modes.
- **WezTerm**: uses its own `KeyCode::Numpad(n)` enum rather than winit's Character+location, so not directly comparable.

---

## 1.5 Fix Consensus (via /tp-help)

- **Proposed approach (pre-consensus)**: Two-site fallback: in both `legacy::encode_character` and `kitty::encode_kitty`'s send-as-text branch, when `text` is `None` for a single-char `Key::Character(ch)`, fall back to `ch.as_bytes()`. Invert the existing `numpad_5_no_app_keypad` test. Add numpad × {digits, operators, decimal, Enter} × {text=None, text=Some} × {legacy, Kitty, APP_KEYPAD} matrix.
- **tp-help run scratch dir**: `/tmp/tpr-round-ori_term-XawWv6PG`

### Round 1
- **Codex summary**: Agrees fallback is the right primary fix (not explicit normal-mode numpad dispatch — that would duplicate `encode_numpad_app`'s digit/operator knowledge and still leave non-numpad `Character(ch) + text:None` broken). But flags three load-bearing refinements: (a) the Kitty send-as-text branch at `kitty.rs:113-116` runs BEFORE `resolve_event_suffix` suppresses releases — today it accidentally returns empty because `text` is absent on release, but the fallback would start sending character bytes on release. Need release suppression moved ahead of the text-return OR a negative pin that enforces empty on `Release + DISAMBIGUATE`. (b) `legacy::encode_character`'s Alt prefix branch (`legacy.rs:236-244`) only prepends ESC when `text` is populated — a naive end-of-function fallback emits `b"5"` instead of `ESC 5` for Alt+Numpad5 with `text:None`. The fallback must also feed the Alt-prefix branch. (c) Kitty 57399-57413 codepoints are a real protocol parity GAP but out of scope for BUG-08-013; file separately. On NumpadEnter: codex says the root cause for "Enter dead" likely ISN'T this bug — named Enter routes through `encode_simple_named` → `b"\r"` without consulting text; user may be seeing an upstream interceptor (IME suppression, overlays, keybindings) OR the backend is reporting NumpadEnter as `Key::Character("\r")` not `Key::Named(Enter)`.
- **Gemini summary**: Agrees fallback is correct and sufficient for legacy + Kitty `should_send_as_text`. Gives concrete NumpadEnter diagnosis: "likely caused by it being reported as `Key::Character(\"\\r\")` on the user's platform rather than `NamedKey::Enter`. If reported as a character, it hits the same `text: None` gap in `legacy.rs:247`." Recommends ADDITIONALLY bundling Kitty 57399-57413 disambiguation into this bug ("Do it properly, not just simply"). Confirms `Key::Dead(c)` is distinct from `Key::Character` so fallback is dead-key-safe. Recommends test matrix cover `location: Numpad` vs `location: Standard`.
- **Agreement points**: (1) Fallback approach is correct; (2) Explicit normal-mode numpad dispatch is rejected; (3) `Key::Dead` is distinct — no composition risk; (4) NumpadEnter in the repro is likely `Key::Character("\r")` on the user's backend hitting the same gap (or an upstream interceptor — test both hypotheses).
- **Disagreement points**: (1) Should Kitty 57399-57413 disambiguation ship in this bug? Codex says no (scope), gemini says yes (do-it-properly). (2) Codex raises two refinements gemini missed entirely (Kitty release suppression + legacy Alt prefix).
- **Independent code verification**:
  - `legacy.rs:223` `encode_character(s: &str, mods, text)` — `s` IS the `Key::Character(ch)` content already, not a separate field. No threading required — just use `s` directly in the fallback. Verified.
  - `legacy.rs:236-244` Alt branch — confirmed: `if let Some(t) = text { ESC + t }` falls through to the trailing fallback when text is None. Fix must also fold `s` into this branch.
  - `kitty.rs:113-116` send-as-text branch at `encode_kitty` — confirmed: runs BEFORE `resolve_event_suffix(report_events, event_type)` at `kitty.rs:196`. On `event_type == Release` without `REPORT_EVENT_TYPES`, resolve_event_suffix returns `None` and the whole encoder returns empty. A text-fallback that runs before this suppression would leak release bytes. Codex's finding verified.
  - `kitty.rs:87` reads `report_events` — the early release-suppression check could reuse this. Alternative: add `input.event_type != KeyEventType::Release` to the `should_send_as_text` predicate.
  - `winit-0.30.12/src/keyboard.rs:1588` — `Key::to_text()` returns `Some(s)` for `Character(s)` variant; confirms `s` is the correct fallback source.
  - `winit-0.30.12/src/event.rs:594` — `KeyEvent::text` doc confirms `None` is valid for various cases. No "None means drop" semantic.
  - `encode_simple_named` at `legacy.rs:164` returns `b"\r"` unconditionally for `NamedKey::Enter` — codex's diagnosis that true-Named-Enter isn't explained by this bug is verified.
- **Outcome**: persuaded divergence — proceed with fallback approach but EXPANDED to cover codex's refinements (release suppression, Alt prefix) and keep Kitty 57399-57413 as a separately-filed follow-up bug.

### Final agreed approach

**Three-point fix (expanded from 2-point after codex refinements):**

1. **Legacy `encode_character` (`oriterm/src/key_encoding/legacy.rs:223`)**: use `s` (the `Key::Character(ch)` content) as the fallback source. Apply to BOTH the Alt-prefix branch (line 237) AND the trailing fallback (line 247). Concretely:
   - Compute `bytes: &[u8] = text.map(str::as_bytes).unwrap_or_else(|| s.as_bytes())` at the top of the function.
   - The Alt branch sends `ESC + bytes`; the final fallback returns `bytes.to_vec()`.
   - Ctrl+letter path is unchanged (uses `ctrl_key_byte(s)` which already reads `s` directly).

2. **Kitty `encode_kitty` send-as-text branch (`oriterm/src/key_encoding/kitty.rs:110-117`)**: tighten `should_send_as_text` to also require `event_type == Press OR REPORT_EVENT_TYPES is active` — i.e., never enter the send-as-text early-return on a release event when REPORT_EVENT_TYPES is off. Within the (now-release-safe) send-as-text return, fall back to `ch.as_str().as_bytes()` when `input.text` is None. Add a negative pin: `encode_key(Key::Character("a"), empty, DISAMBIGUATE_ESC_CODES, text=None, event_type=Release)` returns empty.

3. **File BUG-08-26** (via `/add-bug`) for Kitty numpad codepoint disambiguation (57399-57413 per alacritty's `try_build_numpad`). Out of scope for BUG-08-013 which is about missing PTY bytes, not protocol-parity feature addition. Codex-recommended, gemini-acknowledged.

**Test matrix additions** (on top of the original matrix):
- Negative pin: `Key::Character("a") + Release + DISAMBIGUATE` returns empty (verifies fallback doesn't leak release bytes into Kitty path).
- `Key::Named(Enter) + Numpad location + text=None` returns `b"\r"` — regression pin for NumpadEnter (codex's primary hypothesis).
- `Key::Character("\r") + Numpad location + text=None` returns `b"\r"` — gemini's hypothesis that some backends report NumpadEnter as a character.
- Alt+Numpad5 with `text=None`: returns `ESC 5` (not `b"5"`).
- Ctrl+Numpad5: routes through Ctrl path, unchanged.

---

## 2. TDD — Test Matrix

Write ALL tests BEFORE the fix. Verify they fail against current code.

### Exact failing case
- [ ] `enc_numpad(Key::Character("1".into()), empty, no_mode(), text=None)` returns `b"1"` (currently returns empty).

### Edge cases — all digits and operators (text=None)
- [ ] `Key::Character("0".into())` → `b"0"`
- [ ] `Key::Character("2".into())` → `b"2"`
- [ ] `Key::Character("5".into())` → `b"5"`
- [ ] `Key::Character("9".into())` → `b"9"`
- [ ] `Key::Character("+".into())` → `b"+"`
- [ ] `Key::Character("-".into())` → `b"-"`
- [ ] `Key::Character("*".into())` → `b"*"`
- [ ] `Key::Character("/".into())` → `b"/"`
- [ ] `Key::Character(".".into())` → `b"."`

### Cross-mode coverage
- [ ] Legacy no_mode × `text=None` × numpad digit/operator: fallback to `Key::Character(ch)`.
- [ ] Legacy no_mode × `text=Some("5")` × numpad digit: prefer text (pins "text wins over ch").
- [ ] APP_KEYPAD × `text=None` × numpad digit: SS3 sequence (`\x1bOu` for "5", etc.) — unchanged (`encode_numpad_app` path).
- [ ] Kitty DISAMBIGUATE_ESC_CODES × `text=None` × Numpad Character("5") × Press: returns `b"5"` (send-as-text with fallback).
- [ ] Kitty DISAMBIGUATE_ESC_CODES × `text=Some("5")` × Numpad Character("5") × Press: returns `b"5"` (prefer text).
- [ ] Kitty REPORT_ALL_KEYS_AS_ESC × Numpad Character: routes through CSI u path (not send-as-text), unaffected by this fix.
- [ ] `LINE_FEED_NEW_LINE` + Numpad `Key::Named(Enter)` + `text=None`: returns `b"\r\n"`.

### NumpadEnter regression pins (both winit-backend shapes)
- [ ] `Key::Named(Enter)` + Numpad location + `text=None` + no_mode → `b"\r"` (already-works regression pin; `encode_simple_named` path).
- [ ] `Key::Named(Enter)` + Numpad location + `text=None` + APP_KEYPAD → `b"\x1bOM"`.
- [ ] `Key::Character("\r".into())` + Numpad location + `text=None` + no_mode → `b"\r"` (gemini's hypothesis: some backends emit NumpadEnter as Character, not Named).
- [ ] `Key::Character("\r".into())` + Numpad location + `text=Some("\r")` + no_mode → `b"\r"`.

### Cross-feature (modifier) matrix
- [ ] Ctrl+Numpad5 with `text=None` → routes through Ctrl path (`ctrl_key_byte("5") = None` for digits that aren't in the Ctrl+digit shortcut table; falls back to text/ch fallback). Actually: `ctrl_key_byte("5") = Some(0x1d)` per existing table (`b'5'` maps to GS). So Ctrl+Numpad5 returns `[0x1d]`. Pin both `text=None` and `text=Some("5")`.
- [ ] Ctrl+Numpad1 with `text=None` → `ctrl_key_byte("1") = None` (digit 1 not in Ctrl-shortcut table) → falls through. With this fix's `s`-fallback, returns `b"1"` (no Ctrl modification since no C0 code). Current behavior returns empty. Pin new behavior.
- [ ] Alt+Numpad5 with `text=None` → `b"\x1b5"` (ESC + `5`, via the Alt-prefix branch now using `s` fallback). Without this fix returns empty (Alt branch requires text).
- [ ] Alt+Numpad5 with `text=Some("5")` → `b"\x1b5"` (prefer text within the Alt-prefix branch).
- [ ] Alt+Ctrl+Numpad5 with `text=None` → `[0x1b, 0x1d]` (Ctrl+Alt path — no change; Ctrl path emits ESC + 0x1d).

### Kitty release-suppression negative pin (codex's refinement)
- [ ] `Key::Character("a".into()) + Release + DISAMBIGUATE_ESC_CODES (Kitty flag) + text=None` → returns empty bytes. Pins the fix: `should_send_as_text` must exclude Release events when REPORT_EVENT_TYPES is off, so the fallback doesn't leak release bytes.
- [ ] `Key::Character("5".into()) + Release + DISAMBIGUATE_ESC_CODES + text=None` at Numpad → returns empty (release on numpad still suppressed).
- [ ] Regression pin: `Key::Character("a") + Press + DISAMBIGUATE_ESC_CODES + text=None` → `b"a"` (still works — send-as-text path with fallback).

### Semantic pin
- [ ] `encode_key(Key::Character("5".into()), empty, no_mode, text=None, Numpad, Press)` → `b"5"`. This test is the permanent regression guard — only passes with the fallback.

### Negative pins
- [ ] Same input but `text=Some("5")` → still `b"5"` (verifies `text` is preferred over `Key::Character(ch)`; rejects a buggy fallback that ALWAYS uses `ch` and ignores `text`).
- [ ] `Key::Character("a")` + Kitty DISAMBIGUATE + Release + `text=None` → empty (codex's Kitty-release-suppression pin).
- [ ] `Key::Dead(Some('a'))` path: unchanged (fallback only fires on `Key::Character`, not `Key::Dead`).

### Matrix completeness assertion
Add a count-based self-verification test per `tests.md §Self-verifying matrix completeness`:
```rust
let modes = [no_mode(), app_keypad_mode(), kitty_disambiguate_mode()];
let keys = ["0","1","2","3","4","5","6","7","8","9","+","-","*","/","."];
let mut count = 0;
for mode in modes { for k in keys {
    let r = enc_numpad(Key::Character(k.into()), Modifiers::empty(), mode);
    assert!(!r.is_empty(), "{k} in {mode:?} produced empty bytes");
    count += 1;
}}
assert_eq!(count, modes.len() * keys.len());
```

### Verify tests fail before fix
- [ ] All new tests fail against current code. Existing `numpad_5_no_app_keypad` assertion is inverted: `assert_eq!(r, b"5")` instead of `assert!(r.is_empty())`.

### Helper update
- `enc_numpad` helper in `tests/mod.rs` currently accepts only `(Key, Modifiers, TermMode)` and hardcodes `text: None`. Add a sibling `enc_numpad_text(Key, Modifiers, TermMode, &str)` or restructure to accept an `Option<&str>` parameter so tests can drive both the `text=None` and `text=Some` arms. Do NOT change the existing signature (other tests depend on it).

---

## 2.5 Fix Plan TPR Findings

**Gate:** Mandatory — high severity (per Phase 2.5 gate criteria in `/fix-bug` SKILL.md).

**Status:** Infrastructure-blocked — `/tpr-review` dispatched at 2026-04-24 during `/fix-next-bug` autopilot, but BOTH codex and gemini Sonnet sub-agents returned `API Error: Extra usage is required for 1M context · run /extra-usage to enable, or /model to switch to standard context`. The reviewer CLIs never reached the grounding step; this is a session-level billing gate on Sonnet sub-agents, not a signal about fix-plan quality. In `/fix-bug --autopilot` mode, `AskUserQuestion` is banned and there is no interactive path to enable the billing flag; the autopilot contract is "continue through every phase until fix is complete or a hard blocker is surfaced."

**Decision:** proceed to Phase 3 (TDD) on the strength of the §1.5 `/tp-help` consensus run, which DID complete successfully (codex + gemini both returned structured advice at `/tmp/tpr-round-ori_term-XawWv6PG`). The consensus surfaced three load-bearing refinements (release-suppression guard, Alt-prefix threading, separate BUG-08-26 filing for Kitty 57399-57414 parity), all of which are folded into §2 TDD matrix and §3 Implementation. Plan TPR would have pressure-tested the SAME plan, most likely re-raising the same refinements that consensus already caught.

**Transport failure evidence:**
- Scratch dir: `/tmp/tpr-round-ori_term-aeVSJpaF`.
- Sub-agent Agent() call returned: `API Error: Extra usage is required for 1M context · run /extra-usage to enable, or /model to switch to standard context` (both codex and gemini dispatches — same error, same billing gate).
- No stdout, no stderr, no scratch artifacts from the reviewer CLIs (they never ran).

**Follow-up owned by Phase 5:** the Phase 5 Code `/tpr-review` is subject to the same infrastructure limitation. If it also cannot run, the implementation must NOT close until either (a) the 1M-context billing gate is resolved and Phase 5 TPR runs clean, OR (b) the user explicitly accepts an infrastructure-blocked closure via an interactive session. Autopilot MUST NOT unilaterally close this fix if Phase 5 TPR cannot run — that would ship an unreviewed high-severity change. This escalation is recorded here so the end-of-autopilot session report surfaces it.

---

## 3. Implementation

### 3.1 `oriterm/src/key_encoding/legacy.rs` — `encode_character`

The function already receives `s: &str` which IS the `Key::Character(ch)` content (no threading required — verified during `/tp-help` round 1 code verification). Refactor to compute the textual byte source once, then use it in BOTH the Alt-prefix branch AND the final fallback:

```rust
fn encode_character(s: &str, mods: Modifiers, text: Option<&str>) -> Vec<u8> {
    // Ctrl+letter → C0 control code. (unchanged — uses `s` directly.)
    if mods.contains(Modifiers::CONTROL) {
        if let Some(c0) = ctrl_key_byte(s) {
            let mut v = Vec::with_capacity(2);
            if mods.contains(Modifiers::ALT) {
                v.push(0x1b);
            }
            v.push(c0);
            return v;
        }
    }

    // Textual byte source: prefer `text` (locale-aware, may reflect IME composition),
    // fall back to the logical character `s`. Covers numpad keys when winit's backend
    // does not populate `text` (BUG-08-013).
    let bytes: &[u8] = text.map_or_else(|| s.as_bytes(), str::as_bytes);

    // Alt prefix for character keys (without Ctrl).
    if mods.contains(Modifiers::ALT) && !mods.contains(Modifiers::CONTROL) {
        let mut v = Vec::with_capacity(1 + bytes.len());
        v.push(0x1b);
        v.extend_from_slice(bytes);
        return v;
    }

    // Plain: the textual bytes.
    bytes.to_vec()
}
```

Covers codex refinement (b): Alt+Numpad5 with `text=None` now emits `ESC 5` instead of empty.

### 3.2 `oriterm/src/key_encoding/kitty.rs` — `encode_kitty` + `should_send_as_text`

Two changes, coupled:

**3.2.a** Tighten `should_send_as_text` so it cannot fire on Release events when `REPORT_EVENT_TYPES` is off. Today the send-as-text branch runs BEFORE `resolve_event_suffix`'s release suppression; the current encoder returns empty on release only because `text` is accidentally absent. With the fallback added, a release without suppression would leak character bytes.

```rust
fn should_send_as_text(
    cp: u32,
    mods: Modifiers,
    report_all: bool,
    report_events: bool,
    event_type: KeyEventType,
) -> bool {
    // Release events must go through the CSI u path so release suppression
    // (resolve_event_suffix at kitty.rs:196) can drop them when
    // REPORT_EVENT_TYPES is off. Send-as-text bypasses that suppression.
    if event_type == KeyEventType::Release {
        return false;
    }
    let needs_event_type = report_events && event_type != KeyEventType::Press;
    !report_all && !needs_event_type && mods.is_empty() && cp >= 32 && cp != 127
}
```

**3.2.b** Inside the send-as-text branch, fall back on `ch.as_str()` when `input.text` is None:

```rust
// In encode_kitty, the Character arm at kitty.rs:110-117:
Key::Character(ch) => match resolve_char_codepoint(ch.as_str()) {
    Some(cp) => {
        if should_send_as_text(cp, input.mods, report_all, report_events, input.event_type)
            && !report_text
        {
            // Prefer text; fall back to the logical character.
            let bytes: &[u8] = input.text.map_or_else(|| ch.as_str().as_bytes(), str::as_bytes);
            return bytes.to_vec();
        }
        cp
    }
    None => {
        // Multi-char (dead-key compositions etc.) — send text as-is if available,
        // else the logical character.
        let bytes: &[u8] = input.text.map_or_else(|| ch.as_str().as_bytes(), str::as_bytes);
        return bytes.to_vec();
    }
},
```

Covers codex refinement (a): release suppression is preserved by the `should_send_as_text` guard, so the fallback cannot leak release bytes into the PTY.

### 3.3 Test helpers — `oriterm/src/key_encoding/tests/mod.rs`

Add a sibling helper that accepts text:

```rust
/// Encode a key press at numpad location with explicit text override.
pub(super) fn enc_numpad_text(
    key: Key,
    mods: Modifiers,
    mode: TermMode,
    text: Option<&str>,
) -> Vec<u8> {
    encode_key(&KeyInput {
        key: &key,
        mods,
        mode,
        text,
        location: KeyLocation::Numpad,
        event_type: KeyEventType::Press,
        alternate_key: None,
    })
}

/// Encode a key event at numpad location with full control over event type.
pub(super) fn enc_numpad_full(
    key: Key,
    mods: Modifiers,
    mode: TermMode,
    text: Option<&str>,
    event_type: KeyEventType,
) -> Vec<u8> {
    encode_key(&KeyInput { key: &key, mods, mode, text,
        location: KeyLocation::Numpad, event_type, alternate_key: None })
}

/// Kitty DISAMBIGUATE_ESC_CODES mode (for release-suppression pins).
pub(super) fn kitty_disambiguate_mode() -> TermMode {
    TermMode::default() | TermMode::DISAMBIGUATE_ESC_CODES
}
```

The existing `enc_numpad(key, mods, mode)` is kept as a thin wrapper forwarding `text: None, event_type: Press` — no test depending on it changes.

### 3.4 Tests

- `application_keypad.rs`: invert `numpad_5_no_app_keypad` to expect `b"5"`; add full numpad digit/operator matrix with `text=None` (§2 edge cases); add Alt+Numpad cases.
- `application_keypad.rs`: add `numpad_enter_named_legacy_text_none`, `numpad_enter_character_fallback`, `numpad_enter_app_keypad_text_none` per §2 NumpadEnter pins.
- `kitty_precedence.rs`: add `kitty_disambiguate_numpad_digit_fallback`, `kitty_disambiguate_character_release_suppressed_no_report_events`, `kitty_disambiguate_character_press_with_fallback`.
- `modifier_matrix.rs`: add Alt+Numpad5 with both `text=None` and `text=Some` per §2 modifier matrix.

### 3.5 Order of operations

1. Write ALL tests first (TDD). Run `timeout 150 cargo test -p oriterm --lib key_encoding` and verify every new test FAILS.
2. Apply the `encode_character` change in `legacy.rs`.
3. Apply the `should_send_as_text` tightening + send-as-text fallback in `kitty.rs`.
4. Re-run `timeout 150 cargo test -p oriterm --lib key_encoding` — all new tests pass, no regressions.
5. Run `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` — all green.
6. `/commit-push` before Phase 5 reviews.

---

## R. Third Party Review Findings

Initially empty — populated by the executor during Phase 5 completion checklist.

---

## 4. Completion Checklist

- [ ] All new tests pass unchanged after fix (no test modifications needed).
- [ ] Matrix completeness verified — numpad × {digits, operators, decimal, Enter} × {text=None, text=Some} × {legacy, APP_KEYPAD, Kitty}.
- [ ] Debug AND release builds pass (`cargo b && cargo b --release`).
- [ ] Windows cross-compile green (`cargo build --target x86_64-pc-windows-gnu`).
- [ ] Hot-render-path regressions (`oriterm_core/tests/alloc_regression.rs`, `rss_regression.rs`) still green — key encoding is NOT a hot render path, but keep the invariant check.
- [ ] `timeout 150 ./test-all.sh` green — no regressions.
- [ ] `./clippy-all.sh` green.
- [ ] `./build-all.sh` green (workspace + cross-compile).
- [ ] `cargo test -p oriterm --lib key_encoding` green.
- [ ] `/commit-push` — commit all changes before review.
- [ ] Plan TPR (Phase 2.5) — completed.
- [ ] `/tpr-review` (Phase 5 — code review) passed.
- [ ] `/impl-hygiene-review` passed.
- [ ] Capability regression gate — N/A (fix adds capability, doesn't remove any).
- [ ] `/improve-tooling` retrospective completed.
- [ ] Bug entry in `plans/bug-tracker/section-08-core-terminal.md` updated to `- [x]` with resolution.
- [ ] Fix section frontmatter `status` updated to `complete`.
- [ ] Bug-tracker `00-overview.md` Quick Reference open bug count updated.
- [ ] Final `/commit-push` — commit closure artifacts.

**Exit Criteria:** `cargo test -p oriterm --lib key_encoding` passes with the new numpad-text-None matrix included. The user-visible behavior: pressing numpad digits/operators/decimal in a real shell (with NumLock on) produces the expected characters regardless of which winit backend / platform / Ctrl-state-combination happens to populate or suppress `KeyEvent::text`. `./test-all.sh`, `./build-all.sh`, `./clippy-all.sh` all green. `/tpr-review` and `/impl-hygiene-review` clean.
