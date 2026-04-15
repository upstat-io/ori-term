---
section: "09"
title: "DEC Private Modes (full)"
status: not-started
reviewed: false
goal: "Drive the subset of rows in `catalog/dec-private-modes.md` that Section 09 OWNS from `implemented-unverified` to `verified`: flag-toggle + DECRQM verification for modes 9, 1000, 1002, 1003, 1004, 1005, 1006, 1015, 1047, 1048, 2004, 80, 8452, 9001 (flag-only; 9001 encoding apex is Section 17's scope), plus the two MISSING modes (66/DECNKM, 67/DECBKM) implementation. Mode 2031 (color scheme update notification) is implemented here. Mode 2026 core-layer DECSET/DECRST + DECRQM plumbing is verified here; its publication/commit/abort apex tests belong to Section 06. Mode 1016 is blocked by Section 16 (MISSING catalog row, SGR-pixel encoding is mouse-protocol work). Mode 1007 and 1042 stay at `implemented-unverified` — 1007's wheel-to-arrow apex test belongs to Section 16; 1042's host-notification wiring is not in this section's scope. Mode 2 (DECANM/VT52) is blocked by Section 19."
success_criteria:
  - "Flag-toggle + DECRQM verification landed for every Section 09-owned row: modes 9, 1000, 1002, 1003, 1004, 1005, 1006, 1015, 1047, 1048, 2004, 80, 8452, 9001 (flag only). Each row reaches `verified` for the DECSET/DECRST state-rung and the DECRQM reporting rung."
  - "Mode 2026 core-layer plumbing verified: DECSET/DECRST toggles `TermMode::SYNC_UPDATE`, DECRQM returns correct set/reset value. NOTE: apex publication/commit/abort tests are owned by Section 06 (already `complete`) via mux-level harness at `oriterm_mux/src/pane/io_thread/tests.rs` — this section does NOT re-verify them."
  - "Mode 2031 (color scheme update notification) is `verified` — new `NamedPrivateMode::ColorSchemeUpdate` variant added, hooks into existing `Term::set_theme(Theme)` path, emits `CSI ? 997 ; Ps n` when mode 2031 is enabled and theme changes."
  - "Mode 66 (DECNKM) IMPLEMENTED — new NamedPrivateMode variant, reconciled with existing DECKPAM/DECKPNM ESC =/ESC > path, shares `TermMode::APP_KEYPAD` flag."
  - "Mode 67 (DECBKM) IMPLEMENTED — new NamedPrivateMode variant, new `TermMode::DECBKM` flag, cross-crate backspace encoding updated in `oriterm/src/key_encoding/legacy.rs`."
  - "Every mode touched by this section has DECRQM query/response verified via `status_report_private_mode()` at `oriterm_core/src/term/handler/status.rs:117`."
  - "All new `NamedPrivateMode` variants (ColorSchemeUpdate, DecNumericKeypad, DecBackarrowKey) added to the `decset_decrst_flag_sync()` sync test at `oriterm_core/src/term/handler/tests.rs:5213`."
  - "Cross-reference links land: the rows Section 09 does NOT own (mode 2026 apex → Section 06; mode 1007 wheel-to-arrow apex → Section 16; mode 1016 → Section 16; mode 9001 encoding apex → Section 17; mode 2 → Section 19; mode 1042 host-notification → explicitly deferred to a future section filed via /add-bug) are cross-linked from this section's body."
  - "All existing teseq mode tests pass without modification."
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release."
  - "Section's mission criterion connection: contributes to **Verification chain complete per row** mission criterion for the Section 09-owned row subset only. Does NOT claim ownership of the **Mode 2026 fully wired** criterion — that's Section 06's contribution."
inspired_by:
  - "contour-terminal mode 2026 spec — sync output semantics"
  - "xterm `ctlseqs.html` — every numbered private mode and its semantics"
  - "kitty docs — mode 2031 color scheme notification reference"
depends_on: ["03", "06", "08"]
third_party_review:
  status: findings
  updated: "2026-04-15"
sections:
  - id: "09.1"
    title: "Verify implemented DEC private mode flag toggles + DECRQM"
    status: not-started
  - id: "09.2"
    title: "Verify Mode 2026 sync output (suppression + commit + abort)"
    status: not-started
  - id: "09.3"
    title: "Implement + verify Mode 2031 color scheme update notification"
    status: not-started
  - id: "09.4"
    title: "Implement Mode 66 (DECNKM) and Mode 67 (DECBKM)"
    status: not-started
  - id: "09.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "09.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 09: DEC Private Modes (full)

**Status:** Not Started
**Goal:** Verify the Section 09-owned subset of DEC private mode catalog rows (core-layer DECSET/DECRST + DECRQM plumbing) and implement the two MISSING modes (66/DECNKM, 67/DECBKM) plus Mode 2031 (color scheme update notification). Apex tests for modes this section does NOT own (2026 publication/commit/abort, 1007 wheel-to-arrow, 1016 SGR-pixel, 9001 Win32 encoding, 1004 focus encoding) are owned by other sections and cross-linked here.

**Success Criteria:** see frontmatter.

**Context:** Section 08 verifies the basic mode subset (1, 5, 6, 7, 12, 25, 45, 47, 69/DECLRMM, 1049, 2004 — the modes tack covers plus DECLRMM) including full grid enforcement, CSI s / DECSLRM ambiguity, and save/restore/reset paths. This section drives the remainder: X10 mouse (9), mouse clicks (1000), cell motion (1002), all motion (1003), focus events (1004 — flag only; encoding is Section 16), UTF-8 mouse (1005), SGR mouse (1006), URXVT mouse (1015), alt screen variants (1047, 1048), sixel modes (80, 8452), Win32 input (9001 — flag only; encoding is Section 17), core-layer Mode 2026 plumbing (DECSET/DECRQM; apex tests are Section 06), and implements three missing modes: Mode 2031 (ColorSchemeUpdate), Mode 66 (DECNKM), Mode 67 (DECBKM). (Note: DECLRMM was moved from Section 09 to Section 08 because it's a baseline correctness prerequisite for Phase 3 stacks.)

---

**Row ownership cross-reference (what Section 09 does NOT own):**

| Row | Apex owner | Reason |
|---|---|---|
| `catalog/mode-2026.md` (Begin/Commit/Abort apex) | **Section 06** (`status: complete`) | The publication-suppression / commit / timeout-abort mechanics (`snapshot_seqno`, `handle_sync_timeout()`, `maybe_produce_snapshot()`) live in `oriterm_mux/src/pane/io_thread/mod.rs`. Section 06 already owns + verifies these via `oriterm_mux/src/pane/io_thread/tests.rs`. Section 09 only verifies the core-layer DECSET/DECRST + DECRQM plumbing for mode 2026 — NOT the apex tests. |
| Mode 1007 (alt scroll) wheel-to-arrow apex | **Section 16** (Mouse Protocols) | The wheel-to-arrow translation lives in `oriterm/src/app/mouse_report/mod.rs:181` — app-shell code, not oriterm_core. Full apex test requires an app-shell integration harness owned by Section 16's encoding-verification pass. Section 09 verifies only the flag toggle and DECRQM; catalog row `DEC-ALT-SCROLL` stays `implemented-unverified` until Section 16 lands the app-shell apex test. |
| Mode 1016 (SGR-pixel mouse) | **Section 16** (Mouse Protocols) | Catalog row `DEC-SGR-PIXEL-MOUSE` is `missing` — the mode needs a new `NamedPrivateMode` variant AND the SGR-pixel encoder. Both belong to Section 16 per Section 16's goal. Section 09 excludes mode 1016 from its scope. |
| Mode 9001 (Win32 input) encoding | **Section 17** (Kitty Keyboard Protocol) | Section 17 owns the ConPTY Win32-input encoding implementation (currently stub per Pass 1). Section 09 verifies only the flag toggle + DECRQM for mode 9001; the ENCODING apex test is Section 17's scope. |
| Mode 1004 (focus events) encoding | **Section 16** (Mouse Protocols) | Section 16's goal explicitly includes focus/1004, and subsection 16.5 verifies the mouse + focus event interaction via the shared SGR encoder pipeline. Section 09 verifies only the flag toggle + DECRQM. |
| Mode 1042 (urgency hints) host notification | **Deferred — filed as bug** | Pass 1 found mode 1042 as `stub`: flag is tracked but BEL-to-window-manager-hint wiring is missing. Section 09 verifies the flag toggle + DECRQM only — catalog row stays `stub` until the host-effect wiring lands in a future section/bug. |
| Mode 2 (DECANM / VT52 switch) | **Section 19** (Historical Legacy Control Stacks) | VT52 mode requires an entirely different parser dispatch table. Section 19 owns VT52 implementation. Catalog row `DEC-DECANM` stays `missing`. |

**Boundary with Section 16 (Mouse Protocols):** Section 09 verifies **mode flag toggles**, **DECRQM query/response**, and **mode-gated state changes** (e.g., mutual exclusion of mouse tracking modes). Section 16 owns the **mouse encoding wire format** — the byte sequences emitted by `oriterm/src/app/mouse_report/encode.rs` for each protocol (X10, UTF-8, SGR, URXVT, SGR pixels), the focus event encoding (`ESC [I` / `ESC [O`), AND the mode 1007 wheel-to-arrow app-shell apex test.

**Boundary with Section 06 (Terminal Mode Plumbing):** Section 06 owns the mode 2026 apex tests — publication suppression, atomic commit, timeout-abort — exercised via the mux-level harness at `oriterm_mux/src/pane/io_thread/tests.rs`. Section 06 is `status: complete` as of this writing. Section 09 verifies ONLY the core-layer plumbing for mode 2026: DECSET/DECRST toggles `TermMode::SYNC_UPDATE`, and DECRQM reports the flag correctly.

<!-- blocked-by:19 — DECANM (mode 2) -->
<!-- blocked-by:16 — DEC-SGR-PIXEL-MOUSE (mode 1016) -->
<!-- blocked-by:16 — DEC-ALT-SCROLL (mode 1007) apex test -->
<!-- blocked-by:17 — DEC-WIN32-INPUT (mode 9001) encoding apex -->
<!-- blocked-by:06 — DEC-SYNC-UPDATE (mode 2026) apex tests -->
**Excluded from Section 09's `verified` claims:** DECANM (mode 2 — Section 19), SGR-pixel mouse (mode 1016 — Section 16), alt scroll wheel-to-arrow apex (mode 1007 — Section 16), Win32 input encoding (mode 9001 — Section 17), mode 2026 Begin/Commit/Abort apex (Section 06), mode 1004 focus encoding (Section 16), mode 1042 host notification (deferred).

**Reference implementations:**
- **xterm** `ctlseqs.html` — definitive numbered-mode reference
- **contour-terminal** — Mode 2026 spec (sync output semantics)
- **kitty** docs — Mode 2031 color scheme notification
- **Alacritty** `oriterm/src/app/mouse_report/mod.rs` pattern — alternate scroll tier 2

**Depends on:** Section 06 (`status: complete` — mode 2026 timeout-abort and mux-level apex already verified there; Section 09 only needs the core-layer plumbing); Section 08 (baseline modes verified so this section's per-mode tests have a solid baseline).

**Coupling with Section 06 (cross-reference)**: Mode 2026's publication/commit/abort apex tests are Section 06's deliverable (already complete at `oriterm_mux/src/pane/io_thread/tests.rs`). Section 09 does NOT re-verify them. Section 09's subsection 09.2 verifies ONLY the DECSET/DECRST + DECRQM plumbing at the core layer — the flag is toggled correctly and DECRQM reports the right value.

---

## 09.1 Verify implemented DEC private mode flag toggles + DECRQM

**File(s):** `oriterm_core/tests/spec_chain/private_modes/*.rs` (new — one file per mode family)

**Scope:** For every already-implemented mode in `catalog/dec-private-modes.md` not already verified by Section 08, this subsection:
1. Writes a spec_chain test that toggles the mode via DECSET/DECRST
2. Verifies the correct `TermMode` flag is set/cleared
3. Verifies mutual exclusion behavior (mouse tracking modes clear `ANY_MOUSE` before setting their specific bit; mouse encoding modes clear `ANY_MOUSE_ENCODING`)
4. Verifies DECRQM (`CSI ? Ps $ p`) returns `1` (set) or `2` (reset) per `status_report_private_mode()` at `oriterm_core/src/term/handler/status.rs:117`

**Test file organization:** `oriterm_core/tests/spec_chain/main.rs` currently declares `mod baseline;` and `mod pilots;`. Adding `private_modes/*.rs` requires:
1. Create `oriterm_core/tests/spec_chain/private_modes/mod.rs` declaring submodules
2. Add `mod private_modes;` to `oriterm_core/tests/spec_chain/main.rs`
3. One test file per mode family: `mouse_modes.rs`, `focus_mode.rs`, `alt_screen_modes.rs`, `encoding_modes.rs`, `misc_modes.rs`, `mode_2026.rs`, `mode_2031.rs`, `decnkm_decbkm.rs`

**Checklist:**

- [ ] **Mouse tracking modes (9, 1000, 1002, 1003):** For each, DECSET sets the correct `TermMode` flag and clears other `ANY_MOUSE` bits. DECRST clears the flag. DECRQM returns correct set/reset value. Verify that enabling one mode disables the others (mutual exclusion via `mode.remove(TermMode::ANY_MOUSE)` before `mode.insert(specific)`).

- [ ] **Mouse encoding modes (1005, 1006, 1015):** DECSET sets the correct encoding flag (`MOUSE_UTF8`, `MOUSE_SGR`, `MOUSE_URXVT`) and clears `ANY_MOUSE_ENCODING`. DECRST clears the flag. DECRQM works. Mutual exclusion verified.

- [ ] **Focus events (1004):** DECSET sets `TermMode::FOCUS_IN_OUT`. DECRST clears it. DECRQM works. Note: the actual focus-in/focus-out encoding (`ESC [I` / `ESC [O`) lives in `oriterm/src/app/event_loop_helpers/focus_events/mod.rs` — this is an app-shell concern, not core. Section 09 only verifies the mode flag toggle. Section 16 verifies the full focus-event encoding pipeline.

- [ ] **Alternate scroll (1007) flag-only verification:** The wheel-to-arrow translation lives at `oriterm/src/app/mouse_report/mod.rs:181` (`handle_mouse_wheel()` Tier 2 checks `TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL` and emits `\x1bOA`/`\x1bOB`). Section 09 verifies ONLY:
  - DECSET/DECRST toggles `TermMode::ALTERNATE_SCROLL` correctly
  - DECRQM returns correct value
  - Note: **catalog row `DEC-ALT-SCROLL` stays `implemented-unverified`** — it only becomes `verified` after Section 16 lands the app-shell apex test that asserts wheel-up produces `\x1bOA` bytes on the PTY. Section 09 does NOT update this catalog row's verification status.

- [ ] **Urgency hints (1042) flag-only verification:** DECSET/DECRST toggles `TermMode::URGENCY_HINTS`. DECRQM works. Note: **catalog row `DEC-URGENCY-HINTS` stays `stub`** — the BEL-to-window-manager-hint path requires a `HostEffect::UrgencyHint` variant and host-adapter wiring which is NOT in Section 09's scope. Section 09 does NOT update this catalog row's verification status. File the missing wiring as a bug via `/add-bug` during 09.1 implementation if not already tracked.

- [ ] **Alt screen variants (47, 1047, 1048):** Verify mode flag set/clear, DECRQM, and downstream behavior (screen swap for 47/1047, cursor save/restore for 1048). 1049 is already verified by Section 08 — skip.

- [ ] **Sixel modes (80, 8452):** DECSET/DECRST toggles `TermMode::SIXEL_SCROLLING` and `TermMode::SIXEL_CURSOR_RIGHT`. DECRQM works.

- [ ] **Win32 input (9001) flag-only verification:** DECSET/DECRST toggles `TermMode::WIN32_INPUT`. DECRQM works. Note: **catalog row `DEC-WIN32-INPUT` stays `stub`** — the ConPTY encoding apex is Section 17's scope. Section 09 does NOT update this catalog row's verification status.

- [ ] **Column mode gate (40) and column mode (3):** Verify EnableMode3 flag and DECCOLM side effects (screen clear, margin reset, cursor home). These were partially tested in Section 08 — verify anything not yet covered.

- [ ] **Reverse video (5):** Verify `TermMode::REVERSE_VIDEO` toggle + DECRQM. (May already be covered by Section 08.)

- [ ] **DECRQM cross-cutting validation:** For every mode with a `NamedPrivateMode` variant, assert that `CSI ? Ps $ p` returns `\x1b[?Ps;1$y` when set and `\x1b[?Ps;2$y` when reset. For modes without a `TermMode` flag mapping (`SaveCursor`, `ColumnMode`), `named_private_mode_flag` returns `None` and DECRQM returns `0` (not recognized) — document this deviation if xterm reports these differently.

- [ ] **Catalog update:** Update catalog rows that reach `verified` via this section's work: `DEC-X10-MOUSE`, `DEC-MOUSE-CLICKS`, `DEC-MOUSE-DRAG`, `DEC-MOUSE-MOTION`, `DEC-FOCUS-IN-OUT` (flag only), `DEC-UTF8-MOUSE`, `DEC-SGR-MOUSE`, `DEC-URXVT-MOUSE`, `DEC-ALT-SCREEN-47`, `DEC-ALT-SCREEN-1047`, `DEC-SAVE-CURSOR-1048`, `DEC-SIXEL-SCROLLING`, `DEC-SIXEL-CURSOR-RIGHT`, `DEC-DECNRCM`, `DEC-DECSCNM` (if not covered by 08), `DEC-BRACKETED-PASTE` (if not covered by 08). Rows that stay unchanged per row ownership cross-reference: `DEC-ALT-SCROLL` (→16), `DEC-URGENCY-HINTS` (→deferred bug), `DEC-WIN32-INPUT` (→17), `DEC-DECANM` (→19), `DEC-SGR-PIXEL-MOUSE` (→16).

- [ ] **Validation:** all tests pass; no existing tests regressed.

---

## 09.2 Verify Mode 2026 core-layer plumbing (DECSET/DECRST + DECRQM)

**File(s):** `oriterm_core/tests/spec_chain/private_modes/mode_2026.rs` (new)

**Scope:** Section 09 verifies ONLY the core-layer DECSET/DECRST + DECRQM plumbing for mode 2026. The Begin/Commit/Abort apex tests (`snapshot_seqno` advancement, `PresentationEffect::Begin|Commit|Abort`, publication suppression, timeout-abort) are owned by **Section 06** (`status: complete`) and live at `oriterm_mux/src/pane/io_thread/tests.rs`. Section 09 does NOT duplicate them. See the row ownership cross-reference block above.

- [ ] `mode_2026_decset_toggles_flag`: `CSI ? 2026 h` sets `TermMode::SYNC_UPDATE`; `CSI ? 2026 l` clears it. This is core-layer only — it does NOT exercise the mux sync buffer.
- [ ] `mode_2026_decrqm`: DECRQM query (`CSI ? 2026 $ p`) returns `\x1b[?2026;1$y` when set and `\x1b[?2026;2$y` when reset. Verify via `status_report_private_mode()` delegation to `named_private_mode_flag()`.
- [ ] Catalog note: `catalog/mode-2026.md` rows are owned by Section 06 for the apex verification; Section 09 does not update row statuses there. If the catalog has a core-layer plumbing row separate from the apex rows (check before editing), update it; otherwise leave all mode-2026 catalog rows for Section 06 to manage.
- [ ] **Validation**: flag toggle and DECRQM tests pass; Section 06's apex tests continue to pass without modification.

---

## 09.3 Implement + verify Mode 2031 color scheme update notification

**File(s):** `crates/vte/src/ansi/types.rs` (new `NamedPrivateMode` variant), `oriterm_core/src/term/handler/modes.rs` (DECSET/DECRST dispatch), `oriterm_core/src/term/handler/helpers.rs` (flag mapping), `oriterm_core/src/term/handler/status.rs` (DECRQM), `oriterm_core/src/term/mode/mod.rs` (new `TermMode` bitflag), `oriterm_core/src/term/mod.rs` (hook into existing `set_theme` path), `oriterm_core/tests/spec_chain/private_modes/mode_2031.rs` (new)

Mode 2031 is full implementation work. The work has five parts:

### Part A: VTE layer — parse mode 2031
- [ ] Add `ColorSchemeUpdate = 2031` variant to `NamedPrivateMode` enum in `crates/vte/src/ansi/types.rs`
- [ ] Add `2031 => Self::Named(NamedPrivateMode::ColorSchemeUpdate)` to `PrivateMode::new()` in `crates/vte/src/ansi/types.rs`

### Part B: Core layer — mode flag + DECSET/DECRST + DECRQM
- [ ] Add `COLOR_SCHEME_UPDATE` flag to `TermMode` in `oriterm_core/src/term/mode/mod.rs`
- [ ] Add `NamedPrivateMode::ColorSchemeUpdate => self.mode.insert(TermMode::COLOR_SCHEME_UPDATE)` to `apply_decset()` in `oriterm_core/src/term/handler/modes.rs`
- [ ] Add matching `self.mode.remove(TermMode::COLOR_SCHEME_UPDATE)` to `apply_decrst()`
- [ ] Add `NamedPrivateMode::ColorSchemeUpdate => Some(TermMode::COLOR_SCHEME_UPDATE)` to `named_private_mode_flag()` in `oriterm_core/src/term/handler/helpers.rs`
- [ ] DECRQM automatically works via `status_report_private_mode()` because it delegates to `named_private_mode_flag()` — verify with a test

### Part C: Notification hook — use existing `set_theme(Theme)` path
**IMPORTANT:** There is NO `ColorScheme` type in oriterm_core. The existing host-to-terminal theme path is:
- `PaneIoCommand::SetTheme(Theme, Box<Palette>)` → IO thread → `Term::set_theme(Theme)` at `oriterm_core/src/term/mod.rs:384`
- `Theme` is `{Dark, Light, Unknown}` at `oriterm_core/src/theme/mod.rs:8`

Do NOT invent a new `ColorScheme` type. Instead:
- [ ] Modify `Term::set_theme()` to check if `TermMode::COLOR_SCHEME_UPDATE` is set. If the theme actually changes AND mode 2031 is enabled, emit `Effect::Pty(PtyEffect::Write { bytes: notification_bytes, kind: PtyWriteKind::Other })` where:
  - Dark → `CSI ? 997 ; 1 n` (`\x1b[?997;1n`)
  - Light → `CSI ? 997 ; 2 n` (`\x1b[?997;2n`)
  - Unknown → no notification (same as Dark per kitty convention)
- [ ] The notification emits ONLY when the theme actually changes (the existing `if self.theme == theme { return; }` guard handles this)

### Part D: Sync points
- [ ] Add `NamedPrivateMode::ColorSchemeUpdate` to the `decset_decrst_flag_sync()` test at `oriterm_core/src/term/handler/tests.rs:5213`
- [ ] Verify DECRQM reports correctly for mode 2031

### Part E: Spec_chain tests
- [ ] `mode_2031_disabled_no_notification_on_scheme_change()` — scheme changes via `set_theme(Light)`, no PTY write emitted
- [ ] `mode_2031_dark_scheme_emits_997_1_notification()` — enable mode 2031, call `set_theme(Dark)` on a terminal that was Light, assert `\x1b[?997;1n` emitted
- [ ] `mode_2031_light_scheme_emits_997_2_notification()` — enable mode 2031, call `set_theme(Light)` on a terminal that was Dark, assert `\x1b[?997;2n` emitted
- [ ] `mode_2031_mode_toggle_does_not_emit_notification_by_itself()` — toggling mode 2031 on/off does not emit notification; only real `set_theme()` calls do
- [ ] `mode_2031_same_theme_no_notification()` — calling `set_theme(Dark)` when already Dark is a no-op, no notification even with mode enabled
- [ ] `mode_2031_decrqm_reports_correctly()` — DECRQM returns 1 when set, 2 when reset
- [ ] Update `catalog/dec-private-modes.md` — add a row for mode 2031 (currently missing from catalog)
- [ ] **Validation**: tests pass; mode 2031 implementation is NOT a stub.

---

## 09.4 Implement Mode 66 (DECNKM) and Mode 67 (DECBKM)

**File(s):** `crates/vte/src/ansi/types.rs` (new enum variants), `oriterm_core/src/term/handler/modes.rs`, `oriterm_core/src/term/handler/helpers.rs`, `oriterm_core/src/term/handler/status.rs`, `oriterm_core/src/term/mode/mod.rs`, `oriterm/src/key_encoding/legacy.rs` (DECBKM cross-crate impact), `oriterm_core/tests/spec_chain/private_modes/decnkm_decbkm.rs` (new)

These are two MISSING modes found in the catalog (`DEC-DECNKM`, `DEC-DECBKM`). Both require changes across the VTE crate, oriterm_core mode flags, AND the key encoding layer in the oriterm app shell.

### 09.4a Mode 66 (DECNKM) — Numeric/Application keypad via DECSET/DECRST

**Reconciliation with DECKPAM/DECKPNM:** `ESC =` (DECKPAM) and `ESC >` (DECKPNM) already toggle `TermMode::APP_KEYPAD` at `oriterm_core/src/term/handler/mod.rs:316-320`. Mode 66 is the DECSET/DECRST equivalent per DEC STD 070. Both mechanisms MUST manipulate the SAME `TermMode::APP_KEYPAD` flag — NOT a separate flag. This prevents SSOT drift between the two paths.

- [ ] Add `DecNumericKeypad = 66` variant to `NamedPrivateMode` enum in `crates/vte/src/ansi/types.rs`
- [ ] Add `66 => Self::Named(NamedPrivateMode::DecNumericKeypad)` to `PrivateMode::new()`
- [ ] Add `NamedPrivateMode::DecNumericKeypad => self.mode.insert(TermMode::APP_KEYPAD)` to `apply_decset()`
- [ ] Add `NamedPrivateMode::DecNumericKeypad => self.mode.remove(TermMode::APP_KEYPAD)` to `apply_decrst()`
- [ ] Add `NamedPrivateMode::DecNumericKeypad => Some(TermMode::APP_KEYPAD)` to `named_private_mode_flag()`
- [ ] Add to `decset_decrst_flag_sync()` test
- [ ] Spec_chain tests:
  - `decnkm_set_activates_app_keypad()` — `CSI ? 66 h` sets `APP_KEYPAD`
  - `decnkm_reset_deactivates_app_keypad()` — `CSI ? 66 l` clears `APP_KEYPAD`
  - `decnkm_agrees_with_deckpam()` — `ESC =` then `CSI ? 66 l` clears the flag; `CSI ? 66 h` then `ESC >` clears it. Both paths operate on the same flag.
  - `decnkm_decrqm()` — DECRQM query returns correct value
- [ ] Update catalog row `DEC-DECNKM` from `missing` to `verified`

### 09.4b Mode 67 (DECBKM) — Backarrow key sends BS or DEL

**Cross-crate impact:** Mode 67 changes backspace key encoding. The existing backspace encoding lives in `oriterm/src/key_encoding/legacy.rs` (the `legacy.rs` file at line 179 per the Phase 2 finding, though the exact line may have shifted). When DECBKM is set, Backspace sends BS (`0x08`); when reset (default), Backspace sends DEL (`0x7F`). The key encoding in the app shell reads `TermMode` from the terminal snapshot to decide which byte to emit.

- [ ] Add `DecBackarrowKey = 67` variant to `NamedPrivateMode` enum in `crates/vte/src/ansi/types.rs`
- [ ] Add `67 => Self::Named(NamedPrivateMode::DecBackarrowKey)` to `PrivateMode::new()`
- [ ] Add `DECBKM` flag to `TermMode` in `oriterm_core/src/term/mode/mod.rs` (new flag needed — this is NOT the same as any existing flag)
- [ ] Add `NamedPrivateMode::DecBackarrowKey => self.mode.insert(TermMode::DECBKM)` to `apply_decset()`
- [ ] Add matching `self.mode.remove(TermMode::DECBKM)` to `apply_decrst()`
- [ ] Add `NamedPrivateMode::DecBackarrowKey => Some(TermMode::DECBKM)` to `named_private_mode_flag()`
- [ ] **Cross-crate: key encoding update** — In `oriterm/src/key_encoding/legacy.rs`, the backspace encoding currently sends `\x7f` (DEL). When `TermMode::DECBKM` is set, it must send `\x08` (BS) instead. Verify the existing code path at `oriterm/src/key_encoding/legacy.rs` and update the Backspace match arm to check the mode flag. (Note: `TermMode` is already available in the key encoding path via `KeyInput.term_mode`.)
- [ ] Add to `decset_decrst_flag_sync()` test
- [ ] Spec_chain tests (core — flag toggle only):
  - `decbkm_set_activates_flag()` — `CSI ? 67 h` sets `TermMode::DECBKM`
  - `decbkm_reset_clears_flag()` — `CSI ? 67 l` clears `TermMode::DECBKM`
  - `decbkm_decrqm()` — DECRQM query returns correct value
- [ ] Key encoding tests (app shell — in `oriterm/src/key_encoding/tests.rs`):
  - `backspace_sends_del_by_default()` — mode reset (default), Backspace → `\x7f`
  - `backspace_sends_bs_when_decbkm_set()` — mode set, Backspace → `\x08`
  - `backspace_with_alt_and_decbkm()` — verify Alt+Backspace encoding respects DECBKM
- [ ] Update catalog row `DEC-DECBKM` from `missing` to `verified`

---

## 09.R Third Party Review Findings

**Iteration 1 (2026-04-15 — /review-plan pre-implementation review):**

- [x] `[TPR-09-001-codex][high]` `plans/spec-conformance/section-09-dec-private-modes.md:126` — Fix LEAK by moving Mode 2026 apex verification into oriterm_mux.
  Evidence: Section 09.2 placed `mode_2026.rs` under `oriterm_core/tests/spec_chain`, but the assertions (snapshot_seqno, publication suppression, handle_sync_timeout, maybe_produce_snapshot) live in `oriterm_mux/src/pane/io_thread/mod.rs`. Section 06 already owns + verifies these via `oriterm_mux/src/pane/io_thread/tests.rs`. `catalog/mode-2026.md` owner is "01 (bootstrap), 06 (verification)" — not 09.
  Impact: Crate-boundary LEAK; implementer would either duplicate mux snapshot machinery into core or silently drop real coverage.
  Required plan update: Retarget 09.2 to core-local DECSET/DECRQM only; cross-reference Section 06 for the apex tests.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. 09.2 rewritten to verify only core-layer flag toggle + DECRQM. Added "Row ownership cross-reference" table to section body documenting Section 06 as the apex owner. Updated goal, success criteria, and exit criteria to not claim mode 2026 apex ownership.

- [x] `[TPR-09-002-codex][high]` `plans/spec-conformance/section-09-dec-private-modes.md:6` — Resolve DRIFT in Section 09 row ownership and exit criteria.
  Evidence: Section 09 claimed to close every non-baseline row except DECANM. But `catalog/mode-2026.md` names Section 06 as owner; `catalog/dec-private-modes.md:50` assigns mode 1016 to Section 16; overview + section 17 assign Win32 mode 9001 encoding to Section 17.
  Impact: Section 09 cannot be completed honestly — duplicates other sections' work OR marks externally-owned rows `verified` without the owning section landing.
  Required plan update: Rewrite goal / success / exit criteria to own only actual row subset; cross-reference Section 06 for mode-2026, Section 16 for 1016, Section 17 for 9001 encoding.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. Goal rewritten to list the Section 09-owned row subset explicitly. Added "Row ownership cross-reference" table with per-row owners (06, 16, 17, 19, deferred). Success criteria and exit criteria updated to distinguish Section 09-owned rows from excluded/cross-referenced rows. Added `<!-- blocked-by:X -->` tags for each excluded row.

- [x] `[TPR-09-003-codex][medium]` `plans/spec-conformance/section-09-dec-private-modes.md:98` — Close the GAP between flag-only tests and verified status for modes 1007 and 1042.
  Evidence: 1007 bullet said real wheel-to-arrow behavior is app-shell-only, then still proposed updating catalog row from `stub` to `verified` from core-side flag/DECRQM work. 1042 bullet said BEL-to-window-manager-hint wiring is TBD, catalog row is `stub` with apex `effect-host-notification`. Core tests cannot reach either row's catalog apex.
  Impact: False `verified` status from partial coverage. Breaks verification-chain contract.
  Required plan update: Keep 1007 at `implemented-unverified` until app-shell integration test; explicitly exclude 1042 OR add concrete host-effect subsection.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. 09.1's mode 1007 bullet rewritten: catalog row stays `implemented-unverified` until Section 16 lands the app-shell apex test. Mode 1042 bullet rewritten: catalog row stays `stub` — flag toggle + DECRQM verified only; host-notification gap filed as a bug via `/add-bug` during implementation (added as 09.N checklist item). Row ownership cross-reference table documents both.

- [x] `[TPR-09-001-gemini][medium]` `plans/spec-conformance/section-09-dec-private-modes.md:96` — Fix cross-section reference for focus events.
  Evidence: Gemini claimed Section 16 is "strictly Mouse Protocols" and does not cover focus events (Mode 1004).
  Resolved: Rejected after verification on 2026-04-15. Section 16's goal explicitly includes `focus/1004` in its scope — "every numbered mouse protocol (X10/9, normal/1000, locator/1001, button-event/1002, any-event/1003, focus/1004, UTF-8/1005, SGR/1006, URXVT/1015, SGR pixels/1016)". Section 16 success criteria item 5: "Mouse + focus event interaction verified: enabling 1004 alongside 1006 produces both focus events AND mouse events through the same SGR encoder pipeline." Subsection 16.5 is titled "Verify mouse + focus event interaction" and covers the focus encoding apex. The gemini claim is factually incorrect — Section 16 DOES cover focus event encoding via its own goal and 16.5. The cross-reference from 09.1 to Section 16 for focus encoding is correct.

- [x] `[TPR-09-002-gemini][low]` `plans/spec-conformance/section-09-dec-private-modes.md:250` — Add Mode 1016 to the completion checklist exceptions.
  Evidence: Completion checklist said "All non-baseline DEC private mode catalog rows are `verified` (except DECANM/mode 2 — blocked by Section 19)". But catalog lists Mode 1016 as MISSING — to be added by Section 16 — and Section 09 excludes it.
  Impact: Implementer would be confused by Mode 1016's presence in catalog without an explicit exception.
  Required plan update: Add Mode 1016 to the exception list.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. Completion checklist now has an "Excluded rows stay at current status" item that explicitly lists DECANM, 1016, 1007 apex, 9001 encoding, 1042, and mode-2026 apex rows with their owning sections. Exit criteria updated similarly. Also added `<!-- blocked-by:16 -->` tag in the section body for mode 1016.

---

## 09.N Completion Checklist

- [ ] Failing test matrix written FIRST (TDD)
- [ ] **Matrix dimensions**: every Section 09-owned DEC private mode × set/reset/query(DECRQM) × downstream state change (flag toggle, mutual exclusion). Apex tests (encoding wire format, sync commit/abort, host notifications) are owned by other sections.
- [ ] **Semantic pins**:
  - Mode 2026 DECSET/DECRQM plumbing — regression guard for core-layer plumbing (apex pins are Section 06's)
  - Mode 2031 notification-on-theme-change — regression guard for color scheme reporting
  - Mode 66 reconciliation with DECKPAM/DECKPNM — regression guard for keypad mode SSOT
  - Mode 67 backspace encoding switch — regression guard for cross-crate key encoding
- [ ] **DECRQM cross-cutting**: every mode verified or implemented by this section has its DECRQM query/response tested
- [ ] **Sync point**: all new `NamedPrivateMode` variants (ColorSchemeUpdate, DecNumericKeypad, DecBackarrowKey) added to `decset_decrst_flag_sync()` at `oriterm_core/src/term/handler/tests.rs:5213`
- [ ] **Sync point**: all new `NamedPrivateMode` variants handled in `status_report_private_mode()` (automatic via `named_private_mode_flag()` delegation — but verify)
- [ ] **Section 09-owned rows verified** (see 09.1 "Catalog update" item for the complete list): `DEC-X10-MOUSE`, `DEC-MOUSE-CLICKS`, `DEC-MOUSE-DRAG`, `DEC-MOUSE-MOTION`, `DEC-FOCUS-IN-OUT` (flag only), `DEC-UTF8-MOUSE`, `DEC-SGR-MOUSE`, `DEC-URXVT-MOUSE`, `DEC-ALT-SCREEN-47`, `DEC-ALT-SCREEN-1047`, `DEC-SAVE-CURSOR-1048`, `DEC-SIXEL-SCROLLING`, `DEC-SIXEL-CURSOR-RIGHT`, `DEC-DECNRCM`. Rows NOT owned by Section 09 stay at their current status.
- [ ] **Excluded rows stay at current status** — Section 09 does NOT mark the following as `verified`: `DEC-DECANM` (→ Section 19), `DEC-SGR-PIXEL-MOUSE` (→ Section 16), `DEC-ALT-SCROLL` (apex → Section 16), `DEC-WIN32-INPUT` (encoding → Section 17), `DEC-URGENCY-HINTS` (host notification → deferred bug), `catalog/mode-2026.md` rows (apex → Section 06)
- [ ] Mode 2026 core-layer plumbing verified (flag + DECRQM only — apex remains Section 06's)
- [ ] Mode 2031 color scheme update verified (using existing `Theme` type, NOT a new `ColorScheme` type)
- [ ] Mode 66 (DECNKM) implemented and reconciled with DECKPAM/DECKPNM
- [ ] Mode 67 (DECBKM) implemented with cross-crate key encoding update
- [ ] Catalog row for mode 2031 added to `catalog/dec-private-modes.md`
- [ ] Mode 1042 host-notification gap filed as bug via `/add-bug` (subsystem: Core Terminal / Effect boundary)
- [ ] All existing teseq tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference updated (do NOT tick the "Mode 2026 fully wired" mission criterion — that belongs to Section 06)
- [ ] `index.md` section 09 status updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Section 09-owned DEC private mode catalog rows are `verified` (flag + DECRQM); excluded rows (DECANM, 1016, 1007 apex, 9001 encoding, 1042 host notification, mode-2026 apex) stay at their current status with cross-references pointing at the owning section; Mode 2031 implemented + verified; Modes 66 and 67 implemented + verified; all DECRQM queries return correct responses; `decset_decrst_flag_sync()` updated for new modes; mode 1042 gap filed as a bug.
