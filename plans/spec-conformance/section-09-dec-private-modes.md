---
section: "09"
title: "DEC Private Modes (full)"
status: in-progress
reviewed: true
goal: "Drive the subset of rows in `catalog/dec-private-modes.md` that Section 09 OWNS to `verified` status (state-rung + DECRQM-rung only): modes 9, 1000, 1002, 1003, 1005, 1006, 1015, 1047, 1048, 80, 8452 — these rows' verification chain tops out at the state/effect-mode-state rung, so flag-toggle + DECRQM fully verifies them. Plus the three implementation sub-deliverables: Mode 2031 (full implementation + verification), Mode 66/DECNKM (MISSING implementation), Mode 67/DECBKM (MISSING implementation with cross-crate key encoding). Mode 2026 gets core-layer plumbing verification (flag + DECRQM) only — its apex is Section 06's. Modes that have apexes beyond flag-state (1004 focus encoding, 1007 wheel-to-arrow, 1016 SGR-pixel, 9001 Win32 encoding, 1042 host-notification, 2 DECANM) stay at their current catalog status; Section 09 adds flag-toggle + DECRQM test coverage for those where the mode bit already exists but does NOT promote their catalog verification status — promotion waits for the owning section's apex work. Mode 1007 has no current apex owner and is filed as a deferred bug during 09.1 implementation."
success_criteria:
  - "Flag-toggle + DECRQM verification landed for EVERY Section 09-owned row AND every mode the section touches for flag coverage (includes: 9, 1000, 1002, 1003, 1004, 1005, 1006, 1015, 1042, 1047, 1048, 80, 8452, 9001, 2026). Rows promoted to `verified`: 9, 1000, 1002, 1003, 1005, 1006, 1015, 1047, 1048, 80, 8452 (state-rung apexes). Rows whose catalog status is NOT promoted by Section 09 (see goal and exclusion block): 1004 (apex is Section 16), 1007 (apex is deferred bug — no current owner), 1016 (Section 16), 1042 (host-notification deferred bug), 9001 (encoding is Section 17), 2 (Section 19), 2026 (apex is Section 06)."
  - "Mode 2026 core-layer plumbing verified: DECSET/DECRST toggles `TermMode::SYNC_UPDATE`, DECRQM returns correct set/reset value. NOTE: apex publication/commit/abort tests are owned by Section 06 (already `complete`) via mux-level harness at `oriterm_mux/src/pane/io_thread/tests.rs` — this section does NOT re-verify them and does NOT promote `catalog/mode-2026.md` rows."
  - "Mode 2031 (color scheme update notification) is `verified` — new `NamedPrivateMode::ColorSchemeUpdate` variant added, hooks into existing `Term::set_theme(Theme)` path, emits `CSI ? 997 ; Ps n` when mode 2031 is enabled and theme changes. Catalog row for mode 2031 added to `catalog/dec-private-modes.md`."
  - "Mode 66 (DECNKM) IMPLEMENTED — new NamedPrivateMode variant, reconciled with existing DECKPAM/DECKPNM ESC =/ESC > path, shares `TermMode::APP_KEYPAD` flag. Catalog row `DEC-DECNKM` promoted from `missing` to `verified`."
  - "Mode 67 (DECBKM) IMPLEMENTED — new NamedPrivateMode variant, new `TermMode::DECBKM` flag, cross-crate backspace encoding updated in `oriterm/src/key_encoding/legacy.rs`. Catalog row `DEC-DECBKM` promoted from `missing` to `verified`."
  - "Every mode touched by this section has DECRQM query/response verified via `status_report_private_mode()` at `oriterm_core/src/term/handler/status.rs:117`."
  - "All new `NamedPrivateMode` variants (ColorSchemeUpdate, DecNumericKeypad, DecBackarrowKey) added to the canonical `decset_decrst_flag_sync()` sync test (located via `rg -n 'fn decset_decrst_flag_sync' oriterm_core/src/term/handler/` — post-09.0-split the test lives under `oriterm_core/src/term/handler/tests/status_reports.rs` or a sibling submodule)."
  - "Cross-reference links land in this section's body: excluded rows link to their owning section (06 for mode-2026 apex; 16 for 1004 encoding / 1016 SGR-pixel; 17 for 9001 encoding; 19 for mode 2; deferred bug for 1007 wheel-to-arrow apex and for 1042 host-notification — both filed during 09.1 implementation)."
  - "**Bridge-cell verification** landed only for executable seams Section 09 owns. A bridge cell is an end-to-end test that proves a DECSET/DECRST sequence parsed by `oriterm_core` actually reaches the downstream consumer that the owning section will test at its apex. In-scope bridges (REQUIRED): (a) Mode 1004 focus encoder — parser → `TermMode::FOCUS_IN_OUT` → `focus_event_seq_for_mode()` at `oriterm/src/app/event_loop_helpers/focus_events/mod.rs:45-69`; (b) Mode 2026 mux snapshot gate — parser → `TermMode::SYNC_UPDATE` → `mode_cache` at `oriterm_mux/src/pane/io_thread/mod.rs:337-362`. Optional/best-effort bridge: (c) Mode 1007 alt-scroll Tier-2 gate — only if `should_translate_wheel_to_arrows(mode, shift_held)` can be extracted without destabilizing `oriterm/src/app/mouse_report/mod.rs`; otherwise file `/add-bug` for the refactor and SKIP per the 09.1 alt-scroll item (the apex itself is already deferred to a bug). Out-of-scope bridges (NOT Section 09's work): (d) Mode 9001 Win32 encoder seam — dispatch is not yet wired at `oriterm/src/key_encoding/mod.rs:111-115`, Section 17 (or a prerequisite within 17) owns that bridge once dispatch exists. Without the required bridges (a) + (b), Section 09 can ship clean core tests and still leave a dead consumer path."
  - "Mode 67 (DECBKM) verified end-to-end through the mux backend `pane_mode()` path (embedded + daemon), not just via synthetic `TermMode` in `oriterm/src/key_encoding/tests.rs`. The test must prove: parser `CSI ? 67 h` → io_thread `post_parse_housekeeping()` updates `mode_cache` → main-thread `pane_mode()` read → `encode_key_to_pty()` emits `0x08` for the next Backspace press."
  - "Mode 2031 explicit policy pins landed: `Theme::Unknown` emits NO notification (documented and tested); no notification on no-op `set_theme()` call when theme is unchanged; no notification emitted on `Term::new()` / construction even though a theme is set once during init; no notification back-fill when enabling mode 2031 after a prior theme change (enabling 2031 must NOT synthesize a stale notification); alt-screen / inactive-tab behavior is documented (policy: notification is per-pane via the existing broadcast path in `App::handle_theme_changed()` which already hits every live pane — no mode-2031 special-casing)."
  - "Mode 66 (DECNKM) reconciliation with ESC=/ESC> proven via four-cell matrix: (ESC= → DECRQM?66=set), (DECSET?66 → keypad encode uses APP_KEYPAD), (ESC= → DECRST?66 clears APP_KEYPAD), (DECSET?66 → ESC> clears APP_KEYPAD). Both mechanisms operate on the same `TermMode::APP_KEYPAD` bit — DECRQM reports STATE, not provenance."
  - "**09.0 test-file split (maintainability refactor — OPTIONAL).** Per `.claude/rules/code-hygiene.md` §File Size, `tests.rs` files are EXEMPT from the 500-line limit. 09.0 is a maintainability-driven split (not a rule-compliance prerequisite) that converts `oriterm_core/src/term/handler/tests.rs` (7015 lines) and `oriterm/src/key_encoding/tests.rs` (1801 lines) into directory-module form per `.claude/rules/test-organization.md` rule 3. The split is recommended so ~40 new private-modes cells land in topical submodules rather than appending to an unnavigable 7015-line file, but it MAY be SKIPPED if the final added test surface is small enough to land cleanly inside the existing file — this is a judgment call at implementation time, not a hard gate."
  - "Mode 1042 host-notification gap filed as a bug via `/add-bug` (Subsystem: Core Terminal / Effect boundary — `HostEffect::UrgencyHint` variant + host-adapter wiring)."
  - "All existing teseq mode tests pass without modification."
  - "`./build-all.sh` green (includes Windows cross-compile and both debug + release per its script contents); `./test-all.sh` green (debug workspace test sweep — the script runs `cargo test --workspace --features oriterm/gpu-tests`, no `--release`); `./clippy-all.sh` green. Additionally, an explicit release-mode test run (`cargo test --workspace --features oriterm/gpu-tests --release`, timeout-capped per `.claude/rules/tests.md`) must also pass — TDD discipline requires debug AND release verification for any change that touches hot paths or platform-conditional code."
  - "Section's mission criterion connection: contributes to **Verification chain complete per row** mission criterion for the Section 09-owned row subset only. Does NOT claim ownership of the **Mode 2026 fully wired** criterion — that's Section 06's contribution."
inspired_by:
  - "contour-terminal mode 2026 spec — sync output semantics"
  - "xterm `ctlseqs.html` — every numbered private mode and its semantics"
  - "kitty docs — mode 2031 color scheme notification reference"
depends_on: ["03", "06", "08"]
third_party_review:
  status: resolved
  updated: "2026-04-15"
  iteration_count: 5
  total_items: 19
  note: "19 TPR items total across 5 iterations (iter-1: 5 items / 3 codex + 2 gemini; iter-2: 2 codex-only items, gemini transport failed 3x; iter-3: 5 codex-only items, gemini API capacity failures across 3 attempts; iter-3b: 4 codex-only verification findings catching propagation regressions from iter-3 fixes; iter-3c: 3 codex-only findings catching consistency drift from iter-3b partial propagation). All 19 items [x] in 09.R. Codex-solo passes were best-effort clean per /tpr-review Transport Failure Handling Option 1 — retry dual-source when gemini API capacity recovers."
sections:
  - id: "09.0"
    title: "Test file split (optional maintainability refactor)"
    status: in-progress
  - id: "09.1"
    title: "Verify implemented DEC private mode flag toggles + DECRQM (with bridge cells for externally-owned rows)"
    status: not-started
  - id: "09.2"
    title: "Verify Mode 2026 core-layer plumbing (DECSET/DECRST + DECRQM + bridge to Section 06 apex)"
    status: not-started
  - id: "09.3"
    title: "Implement + verify Mode 2031 color scheme update notification"
    status: not-started
  - id: "09.4"
    title: "Implement Mode 66 (DECNKM) and Mode 67 (DECBKM) with cross-crate end-to-end verification"
    status: not-started
  - id: "09.5"
    title: "Cross-cutting DECRQM + mutual-exclusion + mode-replacement matrix"
    status: not-started
  - id: "09.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "09.N"
    title: "Completion Checklist"
    status: in-progress
---

# Section 09: DEC Private Modes (full)

**Status:** Not Started
**Goal:** Verify the Section 09-owned subset of DEC private mode catalog rows (core-layer DECSET/DECRST + DECRQM plumbing) and implement the two MISSING modes (66/DECNKM, 67/DECBKM) plus Mode 2031 (color scheme update notification). Apex tests for modes this section does NOT own (2026 publication/commit/abort, 1016 SGR-pixel, 9001 Win32 encoding, 1004 focus encoding) are owned by other sections and cross-linked here. Mode 1007 wheel-to-arrow apex has no current owner — Section 09 files it as a deferred bug (same treatment as 1042 host notification).

**Success Criteria:** see frontmatter.

**Context:** Section 08 verifies the basic mode subset (1, 5, 6, 7, 12, 25, 45, 47, 69/DECLRMM, 1049, 2004 — the modes tack covers plus DECLRMM) including full grid enforcement, CSI s / DECSLRM ambiguity, and save/restore/reset paths. This section drives the remainder: X10 mouse (9), mouse clicks (1000), cell motion (1002), all motion (1003), focus events (1004 — flag only; encoding is Section 16), UTF-8 mouse (1005), SGR mouse (1006), URXVT mouse (1015), alt screen variants (1047, 1048), sixel modes (80, 8452), Win32 input (9001 — flag only; encoding is Section 17), core-layer Mode 2026 plumbing (DECSET/DECRQM; apex tests are Section 06), and implements three missing modes: Mode 2031 (ColorSchemeUpdate), Mode 66 (DECNKM), Mode 67 (DECBKM). (Note: DECLRMM was moved from Section 09 to Section 08 because it's a baseline correctness prerequisite for Phase 3 stacks.)

---

**Row ownership cross-reference (what Section 09 does NOT own):**

| Row | Apex owner | Reason |
|---|---|---|
| `catalog/mode-2026.md` (Begin/Commit/Abort apex) | **Section 06** (`status: complete`) | The publication-suppression / commit / timeout-abort mechanics (`snapshot_seqno`, `handle_sync_timeout()`, `maybe_produce_snapshot()`) live in `oriterm_mux/src/pane/io_thread/mod.rs`. Section 06 already owns + verifies these via `oriterm_mux/src/pane/io_thread/tests.rs`. Section 09 only verifies the core-layer DECSET/DECRST + DECRQM plumbing for mode 2026 — NOT the apex tests. |
| Mode 1007 (alt scroll) wheel-to-arrow apex | **Deferred — filed as bug (no current owning section)** | The wheel-to-arrow translation lives in `oriterm/src/app/mouse_report/mod.rs:181` — app-shell code, not oriterm_core. No roadmap section currently claims ownership of the app-shell apex for alt-scroll. Section 09 verifies only the flag toggle + DECRQM at the core layer. Catalog row `DEC-ALT-SCROLL` stays `stub`. File the app-shell integration apex as a bug via `/add-bug` during 09.1 implementation (subsystem: Core Terminal / mouse report app-shell wiring). |
| Mode 1016 (SGR-pixel mouse) | **Section 16** (Mouse Protocols) | Catalog row `DEC-SGR-PIXEL-MOUSE` is `missing` — the mode needs a new `NamedPrivateMode` variant AND the SGR-pixel encoder. Both belong to Section 16 per Section 16's goal. Section 09 excludes mode 1016 from its scope. |
| Mode 9001 (Win32 input) encoding | **Section 17** (Kitty Keyboard Protocol) | Section 17 owns the ConPTY Win32-input encoding implementation (currently stub per Pass 1). Section 09 verifies only the flag toggle + DECRQM for mode 9001; the ENCODING apex test is Section 17's scope. |
| Mode 1004 (focus events) encoding | **Section 16** (Mouse Protocols) | Section 16's goal explicitly includes focus/1004, and subsection 16.5 verifies the mouse + focus event interaction via the shared SGR encoder pipeline. Section 09 verifies only the flag toggle + DECRQM. |
| Mode 1042 (urgency hints) host notification | **Deferred — filed as bug** | Pass 1 found mode 1042 as `stub`: flag is tracked but BEL-to-window-manager-hint wiring is missing. Section 09 verifies the flag toggle + DECRQM only — catalog row stays `stub` until the host-effect wiring lands in a future section/bug. |
| Mode 2 (DECANM / VT52 switch) | **Section 19** (Historical Legacy Control Stacks) | VT52 mode requires an entirely different parser dispatch table. Section 19 owns VT52 implementation. Catalog row `DEC-DECANM` stays `missing`. |

**Boundary with Section 16 (Mouse Protocols):** Section 09 verifies **mode flag toggles**, **DECRQM query/response**, and **mode-gated state changes** (e.g., mutual exclusion of mouse tracking modes). Section 16 owns the **mouse encoding wire format** — the byte sequences emitted by `oriterm/src/app/mouse_report/encode.rs` for each protocol (X10, UTF-8, SGR, URXVT, SGR pixels) and the focus event encoding (`ESC [I` / `ESC [O`). The mode 1007 alt-scroll app-shell apex has no current owning section — see the row-ownership table for the deferred-bug handling.

**Boundary with Section 06 (Terminal Mode Plumbing):** Section 06 owns the mode 2026 apex tests — publication suppression, atomic commit, timeout-abort — exercised via the mux-level harness at `oriterm_mux/src/pane/io_thread/tests.rs`. Section 06 is `status: complete` as of this writing. Section 09 verifies ONLY the core-layer plumbing for mode 2026: DECSET/DECRST toggles `TermMode::SYNC_UPDATE`, and DECRQM reports the flag correctly.

<!-- blocked-by:19 — DECANM (mode 2) -->
<!-- blocked-by:16 — DEC-SGR-PIXEL-MOUSE (mode 1016) -->
<!-- blocked-by:17 — DEC-WIN32-INPUT (mode 9001) encoding apex -->
<!-- blocked-by:06 — DEC-SYNC-UPDATE (mode 2026) apex tests -->
**Excluded from Section 09's `verified` claims:** DECANM (mode 2 — Section 19), SGR-pixel mouse (mode 1016 — Section 16), alt scroll wheel-to-arrow apex (mode 1007 — deferred bug, no current owner), Win32 input encoding (mode 9001 — Section 17), mode 2026 Begin/Commit/Abort apex (Section 06), mode 1004 focus encoding (Section 16), mode 1042 host notification (deferred bug).

**Reference implementations:**
- **xterm** `ctlseqs.html` — definitive numbered-mode reference
- **contour-terminal** — Mode 2026 spec (sync output semantics)
- **kitty** docs — Mode 2031 color scheme notification
- **Alacritty** `oriterm/src/app/mouse_report/mod.rs` pattern — alternate scroll tier 2

**Depends on:** Section 06 (`status: complete` — mode 2026 timeout-abort and mux-level apex already verified there; Section 09 only needs the core-layer plumbing); Section 08 (baseline modes verified so this section's per-mode tests have a solid baseline).

**Coupling with Section 06 (cross-reference)**: Mode 2026's publication/commit/abort apex tests are Section 06's deliverable (already complete at `oriterm_mux/src/pane/io_thread/tests.rs`). Section 09 does NOT re-verify them. Section 09's subsection 09.2 verifies ONLY the DECSET/DECRST + DECRQM plumbing at the core layer — the flag is toggled correctly and DECRQM reports the right value.

---

## 09.0 Test file split (optional maintainability refactor)

**OPTIONAL — not an ordering gate.** 09.0 is a maintainability-driven refactor that converts the two oversized `tests.rs` files into directory-modules so new private-modes content lands in topical submodules. **This is not a rule-compliance prerequisite.** Per `.claude/rules/code-hygiene.md` §File Size, `tests.rs` files are EXEMPT from the 500-line limit — the limit applies only to source files excluding `tests.rs`. The split exists because:

1. Adding ~40 new private-modes test cells to a 7015-line file would make the file unnavigable and hide interactions between adjacent modes.
2. The `.claude/rules/test-organization.md` sibling-tests.rs pattern supports directory-module conversion (rule 3: "When a module has tests, it must be a directory module"). Converting `handler/tests.rs` to `handler/tests/` with topical submodules follows the same pattern used for source-file submodule extraction.
3. Topical organization (mouse, screen, sync, theme, keyboard, status_reports) makes it obvious where new content belongs and surfaces coverage gaps by category.

**Execution ordering is flexible.** 09.0 may run before, during, or alongside 09.1–09.5 — the rule is only that if 09.0 is NOT executed, the added test content must still be navigable. If the added test surface stays small enough to live cleanly inside the existing `tests.rs` files (judgment call at implementation time), 09.0 may be SKIPPED entirely and 09.N's BLOAT block marked N/A with a note explaining the judgment.

**Target file 1:** `oriterm_core/src/term/handler/tests.rs` (7015 lines). Convert the file-module into a directory-module (`oriterm_core/src/term/handler/tests/` — legal because parent `handler/mod.rs` carries `#[cfg(test)] mod tests;` per test-organization rule 3). Move existing test bodies into topical submodules:

- [x] Create `oriterm_core/src/term/handler/tests/mod.rs` as the test module root (declares submodules only; no test bodies — mirrors the `lib.rs` index role from code-hygiene.md §Module Roles). *(done: commit 408a0d8a)*
- [x] Create `oriterm_core/src/term/handler/tests/private_modes_mouse.rs` — mouse-tracking-mode tests (modes 9, 1000, 1002, 1003) and mouse-encoding-mode tests (1005, 1006, 1015). Private-modes content for 09.1 mouse cells lands here. *(done: commit 408a0d8a)*
- [x] Create `oriterm_core/src/term/handler/tests/private_modes_screen.rs` — alt-screen + save-cursor + reverse-video + origin/wrap tests (modes 5, 6, 7, 45, 47, 1047, 1048). 09.1 screen-related cells land here. *(done: commit 408a0d8a)*
- [x] Create `oriterm_core/src/term/handler/tests/private_modes_sync.rs` — mode 2026 core-layer plumbing tests. 09.2 cells land here. *(done: commit 408a0d8a)*
- [ ] Create `oriterm_core/src/term/handler/tests/private_modes_theme.rs` — mode 2031 color-scheme-update tests. 09.3 cells land here. *(deferred: will be created by 09.3 when it adds content)*
- [ ] Create `oriterm_core/src/term/handler/tests/private_modes_keyboard.rs` — DECNKM (66) and DECBKM (67) core-layer flag tests. 09.4 core cells land here. *(deferred: will be created by 09.4 when it adds content)*
- [x] Create `oriterm_core/src/term/handler/tests/status_reports.rs` — DECRQM + DSR + DA tests. 09.5 DECRQM cross-cutting cells land here. *(done: commit 408a0d8a)*
- [x] Distribute the existing 7015 lines of `tests.rs` into the correct submodules by topical concern. This is mechanical re-homing — no test body change. Every existing test keeps the same name and semantics; only its module path changes. *(done: 12 topical submodules — core, dcs, esc, image, modes, osc, sgr + the 4 plan-named files above + mod.rs)*
- [x] After distribution, the empty `tests.rs` file is DELETED (directory module replaces it). *(done: commit 408a0d8a)*

**Target file 2:** `oriterm/src/key_encoding/tests.rs` (1801 lines). Same maintainability-driven split pattern:

- [x] Create `oriterm/src/key_encoding/tests/mod.rs`. *(done: shared helpers + re-exports + 6 submodule declarations)*
- [x] Create `oriterm/src/key_encoding/tests/legacy_backspace.rs` — Backspace / modified Backspace tests. 09.4b cells land here. *(done: 2 tests)*
- [x] Create `oriterm/src/key_encoding/tests/application_keypad.rs` — DECKPAM / DECKPNM / mode 66 integration cells (09.4a integration). *(done: 12 tests)*
- [x] Create `oriterm/src/key_encoding/tests/kitty_precedence.rs` — kitty-mode precedence tests (existing content). *(done: 86 tests)*
- [x] Create `oriterm/src/key_encoding/tests/modifier_matrix.rs` — Alt/Ctrl/Shift/Meta modifier-encoding tests. 09.4b modifier cells land here. *(done: 38 tests)*
- [x] Create `oriterm/src/key_encoding/tests/win32.rs` — Win32 input-mode encoding tests (keeps parity with `oriterm/src/key_encoding/win32.rs`). *(done: 2 tests)*
- [x] Distribute the existing 1801 lines into the correct submodules. Mechanical re-homing only. *(done: 167 tests total, identical to pre-split baseline; added legacy_core.rs (27 tests) beyond the 5 planned files)*

**Validation for 09.0:**

- [x] `./build-all.sh` green after each split step (catch compile errors early).
- [x] `./test-all.sh` green after the split — identical pass count as before (`diff` of `cargo test -p oriterm_core 2>&1 | grep "test result"` before/after).
- [x] Every new `tests/*.rs` file is ≤ the test-file sanity ceiling (tests.rs is exempt from the 500-line limit per code-hygiene.md, but we target ≤ 1500 per split file so future growth has headroom). *(largest: kitty_precedence.rs ~750 lines)*
- [x] `./clippy-all.sh` green — no new warnings from the move.
- [x] No inline `#[cfg(test)] mod tests { ... }` introduced (per test-organization.md rule 1).
- [x] Import style inside split files follows test-organization.md §Import Style: `super::` for parent-module items, `crate::` for cross-module items, grouped stdlib → external → internal.

---

## 09.1 Verify implemented DEC private mode flag toggles + DECRQM (with bridge cells for externally-owned rows)

**File(s):**
- Spec-chain integration tests: `oriterm_core/tests/spec_chain/private_modes/*.rs` (new — one file per mode family). These exercise the full VTE → Term → Effect pipeline end-to-end.
- Core handler unit tests: the split submodules under `oriterm_core/src/term/handler/tests/` created by 09.0 (`private_modes_mouse.rs`, `private_modes_screen.rs`, `status_reports.rs`). These exercise the Term handler directly.
- Bridge cells (externally-owned rows — see below): colocated with the consumer they bridge. In-scope bridges for Section 09: focus events (`oriterm/src/app/event_loop_helpers/focus_events/tests.rs`) and alt-scroll Tier-2 gate (`oriterm/src/app/mouse_report/tests.rs`; optional, see 09.1 alt-scroll item). NOT in scope: the Win32 9001 bridge — Section 17 owns that seam because dispatch is not yet wired (see 09.1 Win32 item).

**Scope:** For every already-implemented mode in `catalog/dec-private-modes.md` not already verified by Section 08, this subsection:
1. Writes a spec_chain test that toggles the mode via DECSET/DECRST
2. Verifies the correct `TermMode` flag is set/cleared
3. Verifies mutual exclusion behavior (mouse tracking modes clear `ANY_MOUSE` before setting their specific bit; mouse encoding modes clear `ANY_MOUSE_ENCODING`)
4. Verifies DECRQM (`CSI ? Ps $ p`) returns `1` (set) or `2` (reset) per `status_report_private_mode()` at `oriterm_core/src/term/handler/status.rs:117`
5. **Bridge cells (NEW)** — for every externally-owned row (apex elsewhere), writes an end-to-end bridge test that proves the parser's bit flip actually reaches the consumer site the owning section will test. A bridge cell is the SSOT contract between parser and consumer. Section 09 does NOT do the owning section's apex work, but it MUST prove the seam is live — otherwise a working parser + dead consumer ships.

**Test file organization:** `oriterm_core/tests/spec_chain/main.rs` currently declares `mod baseline;` and `mod pilots;`. Adding `private_modes/*.rs` requires:
1. Create `oriterm_core/tests/spec_chain/private_modes/mod.rs` declaring submodules
2. Add `mod private_modes;` to `oriterm_core/tests/spec_chain/main.rs`
3. One test file per mode family: `mouse_modes.rs`, `focus_mode.rs`, `alt_screen_modes.rs`, `encoding_modes.rs`, `misc_modes.rs`, `mode_2026.rs`, `mode_2031.rs`, `decnkm_decbkm.rs`

**Checklist:**

- [ ] **Mouse tracking modes (9, 1000, 1002, 1003) — mutual exclusion matrix:** For each of the 4 tracking modes:
  - [ ] DECSET sets the correct `TermMode` flag and clears other `ANY_MOUSE` bits. DECRST clears the flag. DECRQM returns correct set/reset value.
  - [ ] **Replacement cells (sibling-reset):** `DECSET ?9` then `DECSET ?1002` — the 9 bit is cleared, only 1002 is set; `DECSET ?1003` then `DECRST ?1000` — the 1000 bit was never set so 1003 stays set (not cleared by `DECRST` on the other bit); `DECSET ?1002` then `DECSET ?1003` — only 1003 remains; `DECSET ?1003` then `DECSET ?9` — only 9 remains. Assert DECRQM reports the correct single "set" bit at every step. These cells catch the class of bug where switching one mouse tracking mode silently leaves an old bit set.

- [ ] **Mouse encoding modes (1005, 1006, 1015) — mutual exclusion + replacement matrix:** DECSET sets the correct encoding flag (`MOUSE_UTF8`, `MOUSE_SGR`, `MOUSE_URXVT`) and clears `ANY_MOUSE_ENCODING`. DECRST clears the flag. DECRQM works.
  - [ ] **Parallel sync point risk** (per `.claude/rules/impl-hygiene.md` §Registration Sync Points): `DECSET ?1005` → `DECSET ?1006` should clear 1005; `DECSET ?1006` → `DECSET ?1015` should clear 1006; round-robin all three. Verify tracking-mode state is untouched — switching encoding must NOT clear `MOUSE_REPORT_CLICK` / `MOUSE_DRAG` / `MOUSE_MOTION`.

- [ ] **Tracking × encoding interaction cells:** `DECSET ?1000` + `DECSET ?1006` — both tracking AND encoding bits set simultaneously (the common case). Then `DECSET ?1002` — expect 1000 cleared (tracking replaced), 1006 preserved (encoding orthogonal). Then `DECSET ?1005` — expect 1006 cleared (encoding replaced), 1002 preserved.

- [ ] **Focus events (1004) — flag toggle + bridge cell to Section 16 consumer:**
  - Core-layer: DECSET sets `TermMode::FOCUS_IN_OUT`. DECRST clears it. DECRQM works.
  - **Bridge cell (NEW):** parser `CSI ? 1004 h` → `mode.contains(TermMode::FOCUS_IN_OUT)` returns true → `focus_event_seq_for_mode(mode, true)` at `oriterm/src/app/event_loop_helpers/focus_events/mod.rs:45-69` returns `Some(FOCUS_IN_SEQ)`. Colocated test in `oriterm/src/app/event_loop_helpers/focus_events/tests.rs` — takes a `TermMode` built by feeding `\x1b[?1004h` through a real `Term`, asserts `focus_event_seq_for_mode(term.mode(), true) == Some(b"\x1b[I")` and `(term.mode(), false) == Some(b"\x1b[O")`. This proves the parser → consumer SSOT contract is live. Section 16's 16.5 owns the full `App::send_focus_event()` integration apex; Section 09 owns the bridge.
  - Note: the apex catalog row `DEC-FOCUS-IN-OUT` stays `implemented-unverified` until Section 16 lands the `send_focus_event()` app-shell integration test. Section 09 does NOT update this catalog row's verification status.

- [ ] **Alternate scroll (1007) — flag toggle + core-layer bridge cell:**
  - Core-layer: DECSET/DECRST toggles `TermMode::ALTERNATE_SCROLL` correctly. DECRQM returns correct value.
  - **Bridge cell (NEW — optional, best-effort):** parser `CSI ? 1007 h` + `CSI ? 1049 h` (alt screen) → the `handle_mouse_wheel()` Tier-2 condition at `oriterm/src/app/mouse_report/mod.rs:181` reads `mode.contains(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL)` when checked against the post-parse mode. If the Tier-2 gate can be extracted into a pure helper (`should_translate_wheel_to_arrows(mode, shift_held) -> bool`) without destabilizing `mouse_report/mod.rs`, add a colocated test proving `true` for `ALT_SCREEN | ALTERNATE_SCROLL` with no shift, `false` when tracking mode is also set (tracking beats alt-scroll per Tier-1 short-circuit at `mouse_report/mod.rs:150`). If extraction is non-trivial or destabilizes the module, file `/add-bug` for the refactor and SKIP the bridge cell in Section 09 — the app-shell apex is already filed as a deferred bug (see row-ownership table), so Section 09 does NOT block on this bridge.
  - **Interaction cell (core-layer, always required):** `DECSET ?1000` + `DECSET ?1007` (both tracking and alt-scroll enabled) — assert both `TermMode::MOUSE_REPORT_CLICK` and `TermMode::ALTERNATE_SCROLL` bits coexist after parsing. This pins the mutual-state invariant at the core level without depending on the app-shell consumer.
  - Note: **catalog row `DEC-ALT-SCROLL` stays `stub`** — verification requires the app-shell apex that Section 09 filed as a deferred bug. Section 09 does NOT update this catalog row's verification status.

- [ ] **Urgency hints (1042) flag-only verification:** DECSET/DECRST toggles `TermMode::URGENCY_HINTS`. DECRQM works. Note: **catalog row `DEC-URGENCY-HINTS` stays `stub`** — the BEL-to-window-manager-hint path requires a `HostEffect::UrgencyHint` variant and host-adapter wiring which is NOT in Section 09's scope. Section 09 does NOT update this catalog row's verification status. File the missing wiring as a bug via `/add-bug` during 09.1 implementation if not already tracked. (There is NO bridge cell here because the consumer — host adapter wiring — does not exist yet; the gap itself is the bug to file.)

- [ ] **Alt screen variants (47, 1047, 1048):** Verify mode flag set/clear, DECRQM, and downstream behavior (screen swap for 47/1047, cursor save/restore for 1048). 1049 is already verified by Section 08 — skip.

- [ ] **Sixel modes (80, 8452):** DECSET/DECRST toggles `TermMode::SIXEL_SCROLLING` and `TermMode::SIXEL_CURSOR_RIGHT`. DECRQM works.

- [ ] **Win32 input (9001) — flag toggle + DECRQM only:**
  - Core-layer: DECSET/DECRST toggles `TermMode::WIN32_INPUT`. DECRQM returns correct set/reset value.
  - **Bridge cell NOT in Section 09 scope:** Per `oriterm/src/key_encoding/mod.rs:111-115`, Win32 input dispatch is "not yet wired here" — the encoder body exists in `win32.rs` but no dispatch path reads `TermMode::WIN32_INPUT` at keypress time. A bridge cell that tries to observe the Win32 branch being selected is not executable against current code. Section 17 (Kitty Keyboard Protocol) owns BOTH the dispatch wiring AND the encoding apex; Section 17 (or a prerequisite subsection inside Section 17) is responsible for landing the bridge once the dispatch seam exists. Section 09 limits itself to the core-layer flag/DECRQM contract.
  - Note: **catalog row `DEC-WIN32-INPUT` stays `stub`** — dispatch seam + ConPTY encoding apex are Section 17's scope. Section 09 does NOT update this catalog row's verification status.

- [ ] **Column mode gate (40) and column mode (3):** Verify EnableMode3 flag and DECCOLM side effects (screen clear, margin reset, cursor home). These were partially tested in Section 08 — verify anything not yet covered.

- [ ] **Reverse video (5):** Verify `TermMode::REVERSE_VIDEO` toggle + DECRQM. (May already be covered by Section 08.)

- [ ] **DECRQM cross-cutting validation:** For every mode with a `NamedPrivateMode` variant, assert that `CSI ? Ps $ p` returns `\x1b[?Ps;1$y` when set and `\x1b[?Ps;2$y` when reset. For modes without a `TermMode` flag mapping (`SaveCursor`, `ColumnMode`), `named_private_mode_flag` returns `None` and DECRQM returns `0` (not recognized) — document this deviation if xterm reports these differently.

- [ ] **Catalog update — rows promoted to `verified` by Section 09:** `DEC-X10-MOUSE`, `DEC-MOUSE-CLICKS`, `DEC-MOUSE-DRAG`, `DEC-MOUSE-MOTION`, `DEC-UTF8-MOUSE`, `DEC-SGR-MOUSE`, `DEC-URXVT-MOUSE`, `DEC-ALT-SCREEN-47`, `DEC-ALT-SCREEN-1047`, `DEC-SAVE-CURSOR-1048`, `DEC-SIXEL-SCROLLING`, `DEC-SIXEL-CURSOR-RIGHT`, `DEC-DECNRCM`, `DEC-DECSCNM` (if not covered by 08), `DEC-BRACKETED-PASTE` (if not covered by 08). These rows have apex `effect-mode-state` — flag-toggle + DECRQM fully verifies them.
- [ ] **Catalog: rows NOT promoted by Section 09** (flag coverage added but apex is owned elsewhere or deferred, so catalog status stays unchanged): `DEC-FOCUS-IN-OUT` (stays `implemented-unverified` — focus encoding apex is Section 16), `DEC-ALT-SCROLL` (stays `stub` — wheel-to-arrow apex is a deferred bug, no current section owner), `DEC-URGENCY-HINTS` (stays `stub` — host-notification deferred to bug), `DEC-WIN32-INPUT` (stays `stub` — ConPTY encoding apex is Section 17), `DEC-DECANM` (stays `missing` — VT52 is Section 19), `DEC-SGR-PIXEL-MOUSE` (stays `missing` — 1016 is Section 16). `DEC-SYNC-UPDATE`/`catalog/mode-2026.md` rows stay at their current status (owned by Section 06 apex). This section adds DECSET/DECRST + DECRQM test coverage for these rows without changing their catalog verification status.

- [ ] **Validation:** all tests pass; no existing tests regressed.

---

## 09.2 Verify Mode 2026 core-layer plumbing (DECSET/DECRST + DECRQM + bridge to Section 06 apex)

**File(s):** `oriterm_core/tests/spec_chain/private_modes/mode_2026.rs` (new — core-layer tests); `oriterm_mux/src/pane/io_thread/tests.rs` or new sibling (bridge cell).

**Scope:** Section 09 verifies ONLY the core-layer DECSET/DECRST + DECRQM plumbing for mode 2026. The Begin/Commit/Abort apex tests (`snapshot_seqno` advancement, `PresentationEffect::Begin|Commit|Abort`, publication suppression, timeout-abort) are owned by **Section 06** (`status: complete`) and live at `oriterm_mux/src/pane/io_thread/tests.rs`. Section 09 does NOT duplicate them. See the row ownership cross-reference block above.

- [ ] `mode_2026_decset_toggles_flag`: `CSI ? 2026 h` sets `TermMode::SYNC_UPDATE`; `CSI ? 2026 l` clears it. This is core-layer only — it does NOT exercise the mux sync buffer.
- [ ] `mode_2026_decrqm`: DECRQM query (`CSI ? 2026 $ p`) returns `\x1b[?2026;1$y` when set and `\x1b[?2026;2$y` when reset. Verify via `status_report_private_mode()` delegation to `named_private_mode_flag()`.
- [ ] **Bridge cell (NEW) to Section 06 consumer:** parser `CSI ? 2026 h` → `post_parse_housekeeping()` at `oriterm_mux/src/pane/io_thread/mod.rs:337-362` publishes the updated `TermMode::SYNC_UPDATE` bit into `mode_cache` (`AtomicU64`). The mux-level consumer at `maybe_produce_snapshot()` reads this bit to gate publication. Colocated test in `oriterm_mux/src/pane/io_thread/tests.rs` (existing file — owned by Section 06, but the bridge cell is a Section 09 responsibility): feed `\x1b[?2026h` through the IO thread, assert `mode_cache.load(Ordering::Acquire) & TermMode::SYNC_UPDATE.bits() != 0` AFTER `post_parse_housekeeping()` returns. This proves the parser → mode_cache → publication-gate SSOT contract is live. Section 06's apex tests (publication suppression + commit + timeout-abort) continue to be Section 06's scope; Section 09 owns only the bridge that proves the same bit Section 06's consumer reads is the bit Section 09's parser writes. If the test file is too crowded to add this cell, extract a new `oriterm_mux/src/pane/io_thread/tests_mode_2026_bridge.rs` sibling per the test-organization rule.
- [ ] Catalog note: `catalog/mode-2026.md` rows are owned by Section 06 for the apex verification; Section 09 does not update row statuses there. If the catalog has a core-layer plumbing row separate from the apex rows (check before editing), update it; otherwise leave all mode-2026 catalog rows for Section 06 to manage.
- [ ] **Validation**: flag toggle, DECRQM, and bridge-cell tests pass; Section 06's apex tests continue to pass without modification.

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
- `PaneIoCommand::SetTheme(Theme, Box<Palette>)` → IO thread (`oriterm_mux/src/pane/io_thread/handler.rs:103-107`) → `Term::set_theme(Theme)` at `oriterm_core/src/term/mod.rs:384-391`
- `Theme` is `{Dark, Light, Unknown}` at `oriterm_core/src/theme/mod.rs:7-16`
- Host side: `App::handle_theme_changed()` at `oriterm/src/app/mod.rs:414-432` already broadcasts `mux.set_pane_theme(pane_id, theme, palette)` to every live pane — this is the natural fan-out for mode 2031 notifications; no new fan-out logic is needed.

Do NOT invent a new `ColorScheme` type. Instead:
- [ ] Modify `Term::set_theme()` to check if `TermMode::COLOR_SCHEME_UPDATE` is set. If the theme actually changes AND mode 2031 is enabled, emit `Effect::Pty(PtyEffect::Write { bytes: notification_bytes, kind: PtyWriteKind::Other })` where:
  - `Theme::Dark` → `CSI ? 997 ; 1 n` (`\x1b[?997;1n`)
  - `Theme::Light` → `CSI ? 997 ; 2 n` (`\x1b[?997;2n`)
  - `Theme::Unknown` → **NO notification** (documented policy pin: kitty recognizes only the two-value notification per `~/projects/reference_repos/console_repos/kitty/kitty/tools/tui/loop/run.go:142-157`; inventing a third notification value (e.g. `?997;0n`) would diverge from the de-facto standard — do NOT do this)
- [ ] The notification emits ONLY when the theme actually changes (the existing `if self.theme == theme { return; }` guard at `term/mod.rs:384-390` handles this — the early-return is now load-bearing for the no-op pin)

### Part C.1: Explicit policy pins (NEW — codex finding on 2031 semantic pins)

Each pin below MUST have a test that ONLY passes with the new behavior AND a negative-pin test that ASSERTS the bad alternative does not happen. These are behavioral invariants, not implementation details.

- [ ] **Theme::Unknown policy pin (positive + negative pair):**
  - Positive: with mode 2031 enabled, `Term::set_theme(Theme::Unknown)` from a non-Unknown baseline produces exactly zero PTY effects.
  - Negative: assert the effect sink was NOT pushed to at all during the Unknown transition (not merely "no `?997;Xn` bytes"). This forbids the implementer from later adding a third notification variant and claiming compliance.
- [ ] **No-backfill on enable pin (positive + negative pair):**
  - Positive: `Term::new()` → `set_theme(Light)` (with mode 2031 DISABLED throughout) → `DECSET ?2031` (enables mode). Assert NO `?997;Xn` emission during the DECSET step.
  - Negative: assert the effect sink received no `Pty::Write` with `?997` prefix at the enable step. Policy: enabling 2031 is event-driven going forward, not a replay of prior state. This matches kitty's model (`~/projects/reference_repos/console_repos/kitty/kitty/tools/tui/loop/api.go:227-232`, `run.go:154-157`).
- [ ] **No-notification-on-no-op pin (positive + negative pair):**
  - Positive: `Term::set_theme(Dark)` when theme is already Dark, with mode 2031 enabled, produces zero effects.
  - Negative: the `if self.theme == theme { return; }` early-return at `term/mod.rs:385-387` is the load-bearing guard — add a regression test that would fail if someone refactors the guard out (e.g. asserts notification count == 0 over 100 consecutive same-theme calls).
- [ ] **No-notification-on-construction pin (positive + negative pair):**
  - Positive: `Term::new(config)` — even if construction sets an initial theme, mode 2031 is OFF by default, so no notification is emitted. Assert effect sink is empty after construction.
  - Negative: repeat with mode 2031 enabled via a mode-preconfigured constructor if one exists; if not, document that the only way to enable 2031 is via DECSET (no boot-time enable), and add a test proving that: `Term::new()` + `DECSET ?2031` + assertion that the DECSET did NOT flush a construction-time notification (covered by the no-backfill pin above).
- [ ] **Alt-screen / inactive-tab policy pin:**
  - Document policy in the subsection body: mode 2031 notification is per-pane and emitted on every live pane's `Term` when `App::handle_theme_changed()` broadcasts `set_pane_theme()`. Alt screen vs primary screen is NOT differentiated — the notification fires on the active `Term` regardless of which screen is active (consistent with the theme change being a "global pane fact"). Inactive (background) panes that still have mode 2031 enabled also receive the notification — that is intended because the application inside may need to react even when not focused.
  - Test: enable mode 2031 on two panes, trigger `handle_theme_changed()`, assert both panes' effect sinks received the notification.
  - Test: enable mode 2031 on a pane, switch to alt screen, trigger theme change, assert the notification still fires (alt screen is not a suppression axis).

### Part D: Sync points
- [ ] Add `NamedPrivateMode::ColorSchemeUpdate` to the canonical `decset_decrst_flag_sync()` test (locate via `rg -n 'fn decset_decrst_flag_sync' oriterm_core/src/term/handler/` — post-09.0-split the file module moves into `oriterm_core/src/term/handler/tests/`)
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

## 09.4 Implement Mode 66 (DECNKM) and Mode 67 (DECBKM) with cross-crate end-to-end verification

**File(s):** `crates/vte/src/ansi/types.rs` (new enum variants), `oriterm_core/src/term/handler/modes.rs`, `oriterm_core/src/term/handler/helpers.rs`, `oriterm_core/src/term/handler/status.rs`, `oriterm_core/src/term/mode/mod.rs`, `oriterm/src/key_encoding/legacy.rs` (DECBKM cross-crate impact), `oriterm_core/tests/spec_chain/private_modes/decnkm_decbkm.rs` (new)

These are two MISSING modes found in the catalog (`DEC-DECNKM`, `DEC-DECBKM`). Both require changes across the VTE crate, oriterm_core mode flags, AND the key encoding layer in the oriterm app shell.

### 09.4a Mode 66 (DECNKM) — Numeric/Application keypad via DECSET/DECRST

**Reconciliation with DECKPAM/DECKPNM:** `ESC =` (DECKPAM) and `ESC >` (DECKPNM) already toggle `TermMode::APP_KEYPAD` at `oriterm_core/src/term/handler/mod.rs:316-320`. Mode 66 is the DECSET/DECRST equivalent per DEC STD 070. Both mechanisms MUST manipulate the SAME `TermMode::APP_KEYPAD` flag — NOT a separate flag. This prevents SSOT drift between the two paths.

- [ ] Add `DecNumericKeypad = 66` variant to `NamedPrivateMode` enum in `crates/vte/src/ansi/types.rs`
- [ ] Add `66 => Self::Named(NamedPrivateMode::DecNumericKeypad)` to `PrivateMode::new()`
- [ ] Add `NamedPrivateMode::DecNumericKeypad => self.mode.insert(TermMode::APP_KEYPAD)` to `apply_decset()`
- [ ] Add `NamedPrivateMode::DecNumericKeypad => self.mode.remove(TermMode::APP_KEYPAD)` to `apply_decrst()`
- [ ] Add `NamedPrivateMode::DecNumericKeypad => Some(TermMode::APP_KEYPAD)` to `named_private_mode_flag()`
- [ ] Add to the canonical `decset_decrst_flag_sync()` test — located via `rg -n 'fn decset_decrst_flag_sync' oriterm_core/src/term/handler/`. Post-09.0-split the test lives under `oriterm_core/src/term/handler/tests/` (likely `status_reports.rs` or `private_modes_*.rs`). The canonical test is the SSOT sync point for every `NamedPrivateMode` variant; its line number is incidental and MUST NOT be referenced in this plan — always locate via grep.

- [ ] **DECRQM-reports-state-not-provenance pin:** DECRQM `?66` returns `1` (set) whenever `TermMode::APP_KEYPAD` is set, regardless of whether the bit was set by `ESC =` or by `CSI ? 66 h`. Per xterm `input.c:1004` and `ctlseqs.txt:962,1108`, DECRQM reports the STATE, not how it was reached. Do NOT add a separate "provenance" flag.

- [ ] **Reconciliation matrix with DECKPAM/DECKPNM (codex finding — 4 cells):** Both mechanisms operate on `TermMode::APP_KEYPAD` — these cells pin the SSOT invariant across the two entry points:
  - [ ] **Cell 1 — ESC= → DECRQM?66 reports set:** Feed `\x1b=`, then DECRQM `CSI ? 66 $ p`. Response MUST be `\x1b[?66;1$y`. This proves DECRQM queries the shared state bit, not the entry-point.
  - [ ] **Cell 2 — DECSET?66 → keypad encoding:** Feed `\x1b[?66h`, then simulate the numeric keypad "1" key press through the encoder. Assert the encoder emits the SS3 application-mode byte sequence (`\x1bOq`) that `ESC =` would produce, not the normal-mode digit. Proves the encoder reads the shared flag.
  - [ ] **Cell 3 — ESC= → DECRST?66 clears:** Feed `\x1b=`, then `\x1b[?66l`. Assert `TermMode::APP_KEYPAD` is cleared; encoder now emits normal-mode bytes.
  - [ ] **Cell 4 — DECSET?66 → ESC> clears:** Feed `\x1b[?66h`, then `\x1b>`. Assert `TermMode::APP_KEYPAD` is cleared. Pins that `ESC >` is not special-cased to only affect flags set by `ESC =`.
  - [ ] **Negative-pin completeness check:** after each of the four cells, DECRQM ?66 must report the CORRECT state (1 after set, 2 after reset) — this proves the two mechanisms are not drifting to separate internal flags.
- [ ] Update catalog row `DEC-DECNKM` from `missing` to `verified`

### 09.4b Mode 67 (DECBKM) — Backarrow key sends BS or DEL

**Cross-crate impact:** Mode 67 changes backspace key encoding. The existing backspace encoding lives in `oriterm/src/key_encoding/legacy.rs` (the `legacy.rs` file at line 179 per the Phase 2 finding, though the exact line may have shifted). When DECBKM is set, Backspace sends BS (`0x08`); when reset (default), Backspace sends DEL (`0x7F`). The key encoding in the app shell reads `TermMode` from the terminal snapshot to decide which byte to emit.

- [ ] Add `DecBackarrowKey = 67` variant to `NamedPrivateMode` enum in `crates/vte/src/ansi/types.rs`
- [ ] Add `67 => Self::Named(NamedPrivateMode::DecBackarrowKey)` to `PrivateMode::new()`
- [ ] Add `DECBKM` flag to `TermMode` in `oriterm_core/src/term/mode/mod.rs` (new flag needed — this is NOT the same as any existing flag)
- [ ] Add `NamedPrivateMode::DecBackarrowKey => self.mode.insert(TermMode::DECBKM)` to `apply_decset()`
- [ ] Add matching `self.mode.remove(TermMode::DECBKM)` to `apply_decrst()`
- [ ] Add `NamedPrivateMode::DecBackarrowKey => Some(TermMode::DECBKM)` to `named_private_mode_flag()`
- [ ] **Cross-crate: key encoding update** — In `oriterm/src/key_encoding/legacy.rs:179-186`, the Backspace match arm currently hardcodes `0x7f` (DEL) for the plain case and `0x08` (BS) for Ctrl+Backspace. The behavior after fix:
  - DECBKM RESET (default): plain Backspace → `0x7f`, Ctrl+Backspace → `0x08` (unchanged — Ctrl-branch is invariant)
  - DECBKM SET: plain Backspace → `0x08` (inverted), Ctrl+Backspace → `0x7f` (inverted — Ctrl swaps the polarity per xterm semantics)
  - Alt+Backspace prefix (`\x1b` ESC-prefix) is applied AFTER the byte is chosen, so the prefix discipline is preserved across both polarities.
  - Note: `TermMode` is already threaded into the encoder path via `KeyInput.mode` (see `encode_key_to_pty()` at `oriterm/src/app/keyboard_input/mod.rs:248` which reads `self.pane_mode(pane_id)`).
- [ ] Add to `decset_decrst_flag_sync()` test (post-split: verify the path — see 09.4a note).

- [ ] Spec_chain tests (core — flag toggle only):
  - `decbkm_set_activates_flag()` — `CSI ? 67 h` sets `TermMode::DECBKM`
  - `decbkm_reset_clears_flag()` — `CSI ? 67 l` clears `TermMode::DECBKM`
  - `decbkm_decrqm()` — DECRQM query returns correct value

- [ ] **Key encoding modifier matrix (app shell — codex finding #5):** Tests land in the split-submodules `oriterm/src/key_encoding/tests/legacy_backspace.rs` + `oriterm/src/key_encoding/tests/modifier_matrix.rs`. Every cell must be present; each is a positive OR negative pin for the fix.
  - [ ] `backspace_sends_del_when_decbkm_reset()` — semantic pin (default behavior)
  - [ ] `backspace_sends_bs_when_decbkm_set()` — semantic pin (new behavior)
  - [ ] `ctrl_backspace_sends_bs_when_decbkm_reset()` — regression pin (existing Ctrl-branch unchanged)
  - [ ] `ctrl_backspace_sends_del_when_decbkm_set()` — semantic pin (Ctrl swaps polarity under DECBKM)
  - [ ] `alt_backspace_esc_prefix_preserved_under_both_modes()` — regression pin (Alt prefix applied after byte choice, both polarities)
  - [ ] `alt_ctrl_backspace_under_both_modes()` — 4-cell sub-matrix (Alt × Ctrl × {DECBKM set, reset}) — must not drop prefix OR flip the wrong byte
  - [ ] `shift_backspace_unchanged_by_decbkm()` — negative pin (Shift does NOT participate in the polarity; if future spec changes this, the test forces a review)
  - [ ] Matrix completeness assertion: include a `let mut count = 0; for mode in [DECBKM_SET, DECBKM_RESET] { for mods in [plain, alt, ctrl, alt|ctrl, shift] { ... count += 1 } } assert_eq!(count, 10);` per `.claude/rules/tests.md` §Matrix Testing self-verifying completeness.

- [ ] **End-to-end bridge test through `pane_mode()` (codex finding #2 — high severity):** Unit-level tests with a synthetic `TermMode` only prove the encoder reads the bit. They do NOT prove that a DECSET seen by the parser actually reaches the encoder at the next keypress through the mux state path. The end-to-end seam is:
  - parser `CSI ? 67 h` → `apply_decset()` → `TermMode::DECBKM` bit set on `self.mode`
  - `post_parse_housekeeping()` at `oriterm_mux/src/pane/io_thread/mod.rs:355-357` stores `self.terminal.mode().bits()` into `mode_cache: AtomicU64`
  - Main-thread keypress handler calls `encode_key_to_pty()` at `oriterm/src/app/keyboard_input/mod.rs:244-268`
  - `encode_key_to_pty()` reads `self.pane_mode(pane_id)` at `oriterm/src/app/mod.rs:471-476` which delegates to `mux.pane_mode(pane_id)` and truncates bits into `TermMode`
  - Encoder sees the updated `mode.contains(TermMode::DECBKM)` and emits `0x08`

  Required test — `decbkm_end_to_end_through_pane_mode_embedded_backend()`:
  - [ ] Use the `oriterm_test_support` fixtures (or equivalent IO-thread harness) to drive the embedded backend end-to-end headlessly. Send `\x1b[?67h` through the PTY mock, allow `post_parse_housekeeping()` to run, then invoke `mux.pane_mode(pane_id)` from the "main thread" side and assert the returned `TermMode` contains `DECBKM`.
  - [ ] Then call the encoder with a `KeyInput` whose `mode` is the mux-returned mode; assert the emitted bytes are `b"\x08"`.
  - [ ] Required test — `decbkm_end_to_end_through_pane_mode_daemon_backend()`: same scenario but through the daemon backend's cached-snapshot path (the backend layer differs between embedded and daemon per `oriterm_mux/src/backend/mod.rs:151-155`). Both backends MUST agree — this pins the SSOT invariant across the two backends.
  - [ ] If constructing a full IO-thread harness is disproportionately heavy for one test, extract the bridge-cell scaffolding into `crates/oriterm_test_support` (per the crate-boundaries rule that test helpers belong there, not in consumer crates) and use it for all future end-to-end mode tests. Do NOT skip the test because "it's hard to set up" — that is exactly the kind of bridge-cell gap codex flagged as high severity.

- [ ] Update catalog row `DEC-DECBKM` from `missing` to `verified`

---

## 09.5 Cross-cutting DECRQM + mutual-exclusion + mode-replacement matrix

**Scope:** Section 09-level matrix tests that span multiple modes and pin the parallel-sync-point invariants per `.claude/rules/impl-hygiene.md` §Registration Sync Points. These tests catch the "DRIFT" class of bug where a new `NamedPrivateMode` variant is added to one place but missing from the DECRQM path or the mutual-exclusion group.

**File(s):** `oriterm_core/src/term/handler/tests/status_reports.rs` (post-09.0-split) and `oriterm_core/tests/spec_chain/private_modes/matrix.rs` (new).

- [ ] **DECRQM exhaustive matrix:** Iterate every `NamedPrivateMode` variant (`NamedPrivateMode::*` — the compile-time enum enumerates the full set), and for each variant:
  - [ ] DECSET the mode, then DECRQM — expect `1` (set) when `named_private_mode_flag()` returns `Some`; expect `0` (not recognized) when it returns `None` (`SaveCursor`, `ColumnMode` — modes without a TermMode flag mapping).
  - [ ] DECRST the mode, then DECRQM — expect `2` (reset) when flag-mapped; `0` otherwise.
  - [ ] The matrix iteration uses a closure over the variant list; the test asserts the iteration count equals the compile-time variant count (per `.claude/rules/tests.md` §Matrix Testing self-verifying completeness). A new variant without a matching DECRQM response fails the count assertion.

- [ ] **Mutual-exclusion matrix — mouse tracking modes:** 4×4 matrix (9, 1000, 1002, 1003) × {enable, then enable the other} — for each pair (A, B) where A≠B:
  - [ ] DECSET A, then DECSET B — assert only B's bit is set; A's bit is cleared (mutual exclusion via `mode.remove(TermMode::ANY_MOUSE)` before `mode.insert(specific)` at `modes.rs::apply_decset()`).
  - [ ] DECSET A, then DECRST B — assert A's bit is UNCHANGED (DECRST on a different mouse-mode bit must not affect A).
  - [ ] 12 cells total (4×3 pairs for exclusion + 4 same-mode for idempotence — DECSET A + DECSET A = A remains set).

- [ ] **Mutual-exclusion matrix — mouse encoding modes:** 3×3 (1005, 1006, 1015) — same shape as above. Assert encoding modes are mutually exclusive among themselves AND orthogonal to tracking modes.

- [ ] **Orthogonality pin — tracking × encoding × focus × alt-scroll:** DECSET `?1000` + `?1006` + `?1004` + `?1007` all on simultaneously. Assert every bit is present in `mode`. Then DECSET `?1002` — assert 1000 cleared, 1006 preserved, 1004 preserved, 1007 preserved. This is a regression pin against the class of bug where "clearing tracking" accidentally clears related bits.

- [ ] **SaveCursor / ColumnMode DECRQM deviation pin:** Document (and pin) that `SaveCursor` (1048) and `ColumnMode` (3) have NO `TermMode` flag mapping — they trigger side effects (cursor save/restore, column mode change) without a persistent state bit. `named_private_mode_flag()` returns `None` for these; DECRQM returns `0` (not recognized). Cross-check against xterm: xterm reports these as `3` or `4` (permanently set / permanently reset) — document the deviation in `catalog/dec-private-modes.md` Notes column OR promote a bug via `/add-bug` if the deviation is unintentional. Decision required at implementation time.

- [ ] **Validation:** every variant in `NamedPrivateMode` is covered by at least one cell; the self-verifying count assertion matches the variant count.

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

**Iteration 2 (2026-04-15 — post-fix re-review; gemini transport failed 3x, codex verified iter-1 resolutions + surfaced 2 new findings):**

- [x] `[TPR-09-001-codex-i2][high]` `plans/spec-conformance/section-09-dec-private-modes.md:6` — Reconcile ownership/status contradictions across Section 09 row lists.
  Evidence: Goal at line 6 and success criteria at line 8 still listed modes 1004 and 9001 as Section 09-owned "verified" rows, while exclusion block at line 80 said focus encoding is owned by 16 and 9001 encoding is owned by 17. 09.1 catalog-update list at line 136 and completion checklist at line 297 still included DEC-FOCUS-IN-OUT as a Section 09-owned verified row. Catalog status language also mismatched — catalog has DEC-ALT-SCROLL as `stub` (not `implemented-unverified`).
  Impact: Section 09 had no single executable ownership story. Implementer could overclaim `verified` status for externally-owned apexes.
  Required plan update: Make goal, success criteria, 09.1 catalog-update bullets, 09.N checklist, and exit criteria all match the row-ownership table and current catalog.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. Split the catalog-update into two items: "rows promoted to `verified` by Section 09" (only those with apex `effect-mode-state`) and "rows NOT promoted by Section 09" (DEC-FOCUS-IN-OUT, DEC-ALT-SCROLL, DEC-URGENCY-HINTS, DEC-WIN32-INPUT, DEC-DECANM, DEC-SGR-PIXEL-MOUSE). Goal rewritten to distinguish the Section 09-promoted row subset from rows where Section 09 adds flag coverage but does NOT promote the catalog status (because apex is owned elsewhere). Completion checklist updated similarly. Catalog status language now matches catalog file verbatim (`stub` stays `stub`, `implemented-unverified` stays `implemented-unverified`, `missing` stays `missing`).

- [x] `[TPR-09-002-codex-i2][medium]` `plans/spec-conformance/section-09-dec-private-modes.md:32` — Retitle subsection 09.2 to match its narrowed core-layer-only scope.
  Evidence: Frontmatter `sections:` entry for 09.2 at line 32 still said "Verify Mode 2026 sync output (suppression + commit + abort)", but the subsection body at line 142 and scope block at 146-151 explicitly limit 09.2 to DECSET/DECRST + DECRQM and defer Begin/Commit/Abort to Section 06.
  Impact: Section inventory advertised the exact mux-level work iteration 1 removed.
  Required plan update: Rename 09.2 entry in frontmatter `sections:` list to match the body.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. Frontmatter 09.2 entry updated to "Verify Mode 2026 core-layer plumbing (DECSET/DECRST + DECRQM)".

**Iteration 2 gemini transport:** failed all 3 attempts (rc=1; attempt 1 stuck at `[init]+[msg.user]` 491s; attempt 2 tool-result error 264s; attempt 3 tool-result error 303s). Codex succeeded on all 3 attempts and its diagnostics verified iteration 1 resolutions against actual code (`oriterm_core/src/term/handler/tests.rs:5190-5245`, `status.rs:100-145`, `handler/mod.rs:300-330`, `crates/vte/src/ansi/types.rs:160-315`, `oriterm/src/app/mouse_report/mod.rs:150-220`, `oriterm/src/key_encoding/legacy.rs:150-240`).

**Iteration 3 (2026-04-15 — post-Opus-expansion re-review; gemini transport blocked by sustained API-capacity failures across 3 attempts; codex solo produced 5 new findings against the expanded plan):**

- [x] `[TPR-09-001-codex-i3][high]` `plans/spec-conformance/section-09-dec-private-modes.md:76` — Resolve GAP for mode 1007 apex ownership.
  Evidence: Section 09 assigned the mode 1007 wheel-to-arrow apex to Section 16, but Section 16's goal/success criteria (`section-16-mouse-protocols.md:6-15`) only claim 9/1000/1001/1002/1003/1004/1005/1006/1015/1016 — no 1007. Section 16's subsections (16.1-16.5) contain no 1007 work item. Catalog also marks `DEC-ALT-SCROLL` as `stub`, not `implemented-unverified`.
  Impact: Section 09 pointed 1007 at an apex owner that doesn't actually own the work; the row would remain orphaned.
  Required plan update: Update Section 09's 1007 ownership and status language.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. Changed 1007 row-owner from "Section 16" to "Deferred — filed as bug (no current owner)", same treatment as 1042. Updated goal, boundary paragraph with Section 16, exclusion block, blocked-by tags, and 09.1 alt-scroll bridge item to reflect that the app-shell apex has no owning section and will be filed as a bug during 09.1 implementation. Catalog status language ("stays `stub`") was already correct.

- [x] `[TPR-09-002-codex-i3][high]` `plans/spec-conformance/section-09-dec-private-modes.md:190` — Resolve GAP in the 9001 bridge requirement.
  Evidence: Section 09 required a bridge cell proving the Win32 encoder dispatch branch is selected for mode 9001, but the encoder entrypoint at `oriterm/src/key_encoding/mod.rs:111-115` explicitly says "Win32 input mode is parsed and tracked as a terminal mode, and the encoder exists in win32.rs, but dispatch is not yet wired here." No live consumer branch exists for the proposed bridge to observe.
  Impact: The checklist was not executable. Satisfying it would force Section 09 to wire the missing 9001 consumer seam — collapsing the Section 17 boundary.
  Required plan update: Move the 9001 bridge cell into Section 17 (or a prerequisite inside 17) and limit Section 09 to flag/DECRQM verification.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. Removed the unexecutable 9001 bridge cell from 09.1. Reframed the Win32 item as "flag toggle + DECRQM only" with an explicit note that Section 17 (or a prerequisite within 17) owns the bridge once the dispatch seam exists. Updated the 09.1 File(s) block to drop the Win32 bridge colocation reference.

- [x] `[TPR-09-003-codex-i3][medium]` `plans/spec-conformance/section-09-dec-private-modes.md:14` — Replace the DRIFT-prone deleted-file anchor.
  Evidence: Frontmatter success criterion hard-coded `decset_decrst_flag_sync()` at `oriterm_core/src/term/handler/tests.rs:5213`, but 09.0 deletes that file (splits into a directory module). The 09.N checklist repeated the same deletion as a completion gate. Body already said "verify via grep" but the frontmatter didn't.
  Impact: Success criterion became stale as soon as 09.0 executed its own prerequisite.
  Required plan update: Use a symbol-based requirement or remove the hard-coded line number.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. Replaced all three hard-coded `tests.rs:5213` references (frontmatter success_criteria, 09.3 checklist, 09.4a checklist) with symbol-based language that locates the test via `rg -n 'fn decset_decrst_flag_sync' oriterm_core/src/term/handler/` and names the post-split home (`oriterm_core/src/term/handler/tests/status_reports.rs` or sibling). The line number is now declared incidental and banned from this plan — always locate via grep.

- [x] `[TPR-09-004-codex-i3][medium]` `plans/spec-conformance/section-09-dec-private-modes.md:108` — Reframe the BLOAT prerequisite to match repo rules.
  Evidence: 09.0 claimed the split was mandatory because the two target `tests.rs` files exceed the 500-line budget and touching them unsplit is "explicitly forbidden." But `.claude/rules/code-hygiene.md` §File Size says: "Source files (excluding `tests.rs`) must not exceed 500 lines. Test files (`tests.rs`) are exempt from the 500-line limit."
  Impact: The plan turned a maintainability refactor into a rule-blocking prerequisite on a false basis, misstating why 09.0 exists and potentially delaying the actual conformance work.
  Required plan update: Reframe 09.0 as a maintainability-driven refactor, or make it optional.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. Rewrote 09.0's opening paragraph: removed the false "500-line rule forbids touching tests.rs" claim, cited the actual code-hygiene.md rule that exempts tests.rs, and reframed the split as a maintainability decision driven by (1) navigability of ~40 new private-modes cells in a 7015-line file, (2) directory-module pattern from test-organization.md rule 3, and (3) topical organization surfacing coverage gaps. Added an explicit "may be SKIPPED if not needed" clause so the split is a judgment call, not a hard prerequisite. Dropped "over budget" language throughout 09.0.

- [x] `[TPR-09-005-codex-i3][medium]` `plans/spec-conformance/section-09-dec-private-modes.md:516` — Fix DRIFT in the release-test gate.
  Evidence: Section 09 claimed both frontmatter and final checklist are satisfied by "`./test-all.sh` green debug + release", but `test-all.sh:8-9` only runs `cargo test --workspace --features oriterm/gpu-tests` with no `--release` flag. `build-all.sh` is the script that carries the debug+release burden.
  Impact: The completion gate overstated what the repo's test script proves; a green checklist could still miss release-mode regressions.
  Required plan update: Either add an explicit release test command, or describe test-all.sh accurately as the debug test sweep.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. Updated frontmatter success_criteria and 09.N completion checklist to describe `./test-all.sh` accurately (debug-only workspace sweep) and `./build-all.sh` as carrying the debug+release burden. Added an explicit release-mode test run as a separate checklist item: `timeout 150 cargo test --workspace --features oriterm/gpu-tests --release` — required because release-mode failures (optimizer-induced alloc regressions, `#[cfg(debug_assertions)]`-gated code divergence) are invisible to test-all.sh.

**Iteration 3 transport note:** gemini API returned `No capacity available for model gemini-3.1-pro-preview` on 3 consecutive attempts (rc=1 each, walltimes 332s/245s/296s), matching the iter-2 gemini failure pattern. Codex was run solo as a best-effort clean pass per `/tpr-review` Transport Failure Handling Option 1 (informed override). Codex's envelope verified scope_actually_reviewed.files_read=46 / rules_consulted=10 including all grounding files (CLAUDE.md, impl-hygiene.md, tests.md, test-organization.md, code-hygiene.md, crate-boundaries.md, oriterm.md, oriterm_core.md, oriterm_mux.md) plus the target plan + catalog + 4 cross-referenced sections (03, 06, 08, 16, 17, 19). All 5 findings independently verified against current code before being marked resolved. Gemini capacity is structural (API-side); retry when capacity recovers.

**Iteration 3b (2026-04-15 — codex verification re-review of iter-3 fixes; caught 4 regressions from inconsistent propagation):**

- [x] `[TPR-09-006-codex-i3b][high]` `plans/spec-conformance/section-09-dec-private-modes.md:8` — Align mode 1007 ownership references with the deferred-bug boundary.
  Evidence: iter-3 fixed the row-ownership table, boundary paragraph, and 09.1 alt-scroll item to say "deferred bug / no current owner" but left stale `Section 16` ownership claims for 1007 in frontmatter success_criteria (line 8: `1007 (apex is Section 16)`, line 15: `16 for ... 1007 wheel-to-arrow`), the 09.1 catalog exception (line 210), and the 09.N catalog checklist (`DEC-ALT-SCROLL ... apex → 16`).
  Impact: Top-level gates still route 1007 to a section that doesn't own it — an implementer reading the frontmatter would target Section 16 and find no matching work.
  Required plan update: Replace every remaining non-TPR Section 16 reference for mode 1007 with the deferred-bug/no-current-owner wording.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. Updated frontmatter goal (mode 1007 line added), success_criteria line 8 ("1007 (apex is deferred bug — no current owner)") and line 15 (removed 1007 from Section 16 list, added to deferred-bug list). Updated 09.1 catalog exception (`DEC-ALT-SCROLL ... apex → deferred bug, no current section owner`). Updated 09.N catalog checklist similarly (`apex → deferred bug, no current owner`). Verified: no remaining non-TPR "Section 16" or "→ 16" references for mode 1007 in the body.

- [x] `[TPR-09-007-codex-i3b][high]` `plans/spec-conformance/section-09-dec-private-modes.md:16` — Limit bridge-cell completion gates to executable seams.
  Evidence: iter-3 removed the 9001 bridge cell from 09.1 and made the 1007 bridge optional in 09.1, but the frontmatter global bridge-cell success criterion at line 16 still required bridges for "1004 focus encoder, 1007 wheel-to-arrow, 9001 Win32 encoder seam, 2026 mux snapshot gate" with no optionality. The 09.N Bridge cells block still listed `Bridge 9001 (Win32 input)` as an unconditional `[ ]` checkbox.
  Impact: Top-level gates forced Section 09 to produce a 9001 bridge that is not executable against current code — reintroducing the exact problem iter-3 Finding 2 resolved.
  Required plan update: Rewrite the global bridge-cell success criterion to list only executable bridges Section 09 owns; remove Bridge 9001 from 09.N; mark 1007 bridge as optional.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. Rewrote the frontmatter bridge-cell success criterion to distinguish REQUIRED bridges (1004 focus, 2026 mux snapshot), OPTIONAL/best-effort bridges (1007 alt-scroll with fallback to `/add-bug`), and OUT-OF-SCOPE bridges (9001 Win32 — owned by Section 17 once dispatch exists). Rewrote the 09.N Bridge-cells block to mirror the frontmatter: REQUIRED (1004, 2026, 67), OPTIONAL (1007 with fallback), OUT-OF-SCOPE (9001 — intentionally no `[ ]` task, documented exclusion so drift is visible).

- [x] `[TPR-09-008-codex-i3b][medium]` `plans/spec-conformance/section-09-dec-private-modes.md:20` — Propagate the tests.rs exemption and optional split semantics to top-level gates.
  Evidence: iter-3 fixed 09.0's body to cite the code-hygiene.md tests.rs exemption and mark the split as a maintainability-driven optional refactor, but the frontmatter BLOAT success criterion (line 20: `Section 09 must not append tests to files already over the 500-line budget`) and the 09.N BLOAT block (line 548: `09.0 test-file split done BEFORE any new test content added`) still claimed it was a mandatory prerequisite. Those top-level gates contradicted the corrected 09.0 text.
  Impact: Implementer reading the frontmatter would still treat 09.0 as a hard prerequisite and either (a) waste effort splitting when not needed or (b) be blocked from landing small additions.
  Required plan update: Match the frontmatter + 09.N gates to the corrected 09.0 semantics.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. Rewrote frontmatter success_criteria 09.0 block to explicitly cite the tests.rs exemption and declare the split OPTIONAL + maintainability-driven + skippable. Rewrote 09.N BLOAT section to "Test file organization (09.0 — OPTIONAL maintainability refactor)" with conditional ("if 09.0 executed") phrasing and an N/A escape hatch. Kept the "no inline `#[cfg(test)] mod tests { ... }`" rule as an unconditional test-organization invariant (applies regardless of 09.0).

- [x] `[TPR-09-009-codex-i3b][low]` `plans/spec-conformance/section-09-dec-private-modes.md:31` — Make third-party review status auditable against the recorded item count.
  Evidence: Frontmatter `third_party_review` note said "5 findings surfaced and resolved" (iter-3 only) without accounting for iter-1 (5 items) and iter-2 (2 items), making the `status: resolved` claim not directly auditable against the 09.R block contents (which had 12 checked items total).
  Impact: Audit mismatch — external tooling or future review couldn't easily verify that every recorded TPR item is resolved by comparing the note to the block.
  Required plan update: Either add missing context to the note or structure it to match the block's actual contents.
  Basis: direct_file_inspection. Confidence: low.
  Resolved: Fixed on 2026-04-15. Expanded `third_party_review` frontmatter with explicit `iteration_count: 3`, `total_items: 12`, and a note breaking down per-iteration counts (iter-1: 5 items / 3 codex + 2 gemini; iter-2: 2 codex-only; iter-3: 5 codex-only) that match the checked items in 09.R. Status `resolved` now has a concrete audit invariant: 12 `[x]` entries in the block + 0 `[ ]` entries.

**Iteration 3b transport note:** codex-solo verification pass only (gemini still unavailable — same API capacity issue). Codex read 15 files including the revised plan, all grounding rules, and the 3 cross-referenced code anchors (`oriterm/src/key_encoding/mod.rs`, `test-all.sh`, `plans/spec-conformance/section-16-mouse-protocols.md`) for finding verification.

**Iteration 3c (2026-04-15 — second codex verification pass; caught 3 remaining consistency bugs from iter-3b):**

- [x] `[TPR-09-010-codex-i3c][high]` `plans/spec-conformance/section-09-dec-private-modes.md:559` — Remove the out-of-scope Bridge 9001 checkbox.
  Evidence: iter-3b's rewrite of 09.N's Bridge-cells block left a self-contradicting line that both created a `- [ ]` task AND declared "This line intentionally has no `[ ]` task". The `- [ ]` marker made it appear as an unchecked gate; a build script counting open boxes would flag the section as incomplete forever.
  Impact: Self-contradicting checklist item — tooling counting checkboxes gets the wrong answer, implementer is confused.
  Required plan update: Remove the `- [ ]` marker entirely; render the 9001 paragraph as plain documentation text.
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. Removed the `- [ ]` marker from the Bridge 9001 line in 09.N. The paragraph now reads as a bold header + explanatory prose without any checkbox syntax, so tooling and humans both see it as documentation only.

- [x] `[TPR-09-011-codex-i3c][medium]` `plans/spec-conformance/section-09-dec-private-modes.md:38` — Strip residual prerequisite wording from 09.0.
  Evidence: Frontmatter sections list titled 09.0 as `Test file split prerequisite (BLOAT remediation)`; body header repeated `prerequisite`; body opened with `RUNS FIRST. Before any new test content is added for 09.1–09.5...`. These still signaled 09.0 as an ordering gate, contradicting the "may be SKIPPED" language introduced by iter-3.
  Impact: Implementer reading the frontmatter title or the body header would still treat 09.0 as a hard prerequisite.
  Required plan update: Rename 09.0 to drop "prerequisite"; rewrite the body's opening to drop "RUNS FIRST" / "Before any new test content is added".
  Basis: direct_file_inspection. Confidence: high.
  Resolved: Fixed on 2026-04-15. Renamed 09.0 to "Test file split (optional maintainability refactor)" in both frontmatter `sections:` list and body header. Rewrote the body's opening paragraph: dropped "RUNS FIRST"; replaced with "OPTIONAL — not an ordering gate" and added an explicit "Execution ordering is flexible" paragraph stating 09.0 may run before/during/alongside 09.1–09.5, or be skipped entirely.

- [x] `[TPR-09-012-codex-i3c][low]` `plans/spec-conformance/section-09-dec-private-modes.md:6` — Erase remaining Section 16 mentions from mode 1007 prose.
  Evidence: Even though iter-3 and iter-3b removed ownership claims, non-TPR prose still paired mode 1007 with "Section 16" in denial form ("Section 16's scope does not cover 1007", "Section 16's stated scope ... does NOT own mode 1007", "Section 16 does NOT cover mode 1007 alt-scroll"). A grep-clean invariant for `1007.*16` still fails outside 09.R.
  Impact: Any grep-based drift detector looking for `1007.*16` co-occurrences would still flag these as stale ownership claims even though they're denials.
  Required plan update: Rephrase the remaining non-TPR 1007 prose with neutral "no current owner" / "deferred bug" language.
  Basis: direct_file_inspection. Confidence: low.
  Resolved: Fixed on 2026-04-15. Rewrote goal clause ("Mode 1007 has no current apex owner and is filed as a deferred bug"), row-ownership table entry ("no current owning section"), and Section-16 boundary paragraph ("The mode 1007 alt-scroll app-shell apex has no current owning section — see the row-ownership table"). No non-TPR 1007+Section 16 co-occurrences remain outside the `09.R` TPR block.

**Iteration 3c transport note:** second codex-solo verification pass (gemini still unavailable). Codex read 23 files for verification. All 3 iter-3c findings were consistency drift from iter-3b's partial propagation — no new structural concerns. The plan is now verified internally consistent across frontmatter / body / 09.N / 09.R with all iter-3 and iter-3b fixes properly propagated.

---

## 09.N Completion Checklist

### TDD Discipline (MUST be FIRST item — TDD for bugs per `.claude/rules/tests.md`)

- [ ] **Failing test matrix written FIRST** — all 09.0 splits done, then all 09.1–09.5 tests written and VERIFIED FAIL, then implementation begins. Missing this step invalidates the TDD contract.
- [ ] **Ordering gate:** tests for 09.0 split ride along the mechanical re-homing (no new test logic). Tests for 09.1 bridge cells + 09.2 bridge + 09.3 semantic pins + 09.4 end-to-end + 09.5 matrix are all RED before any implementation lands.

### Crate-order & Matrix

- [ ] **Crate ordering:** changes land in this order (`.claude/rules/crate-boundaries.md` allowed direction): `crates/vte` (new `NamedPrivateMode` variants) → `oriterm_core` (TermMode flags + mode dispatch + test split part 1 + status_report) → `oriterm` (key_encoding update + test split part 2 + bridge consumers) → `oriterm_mux` (mode_cache bridge verification). Building in the wrong order (e.g. updating `oriterm_core` before the VTE variants are added) produces cycles the build will reject.
- [ ] **Matrix dimensions**: every Section 09-owned DEC private mode × set/reset/query(DECRQM) × downstream state change (flag toggle, mutual exclusion, consumer bridge). Apex tests (full encoding wire format, sync commit/abort, host notifications) are owned by other sections.
- [ ] **Self-verifying matrix completeness:** DECRQM exhaustive matrix (09.5) and DECBKM modifier matrix (09.4b) both include `assert_eq!(count, expected)` completeness pins.

### Semantic pins (every pin has a POSITIVE test that ONLY passes with new behavior AND a NEGATIVE test that REJECTS the old/broken behavior — per `.claude/rules/tests.md` §Negative Testing Protocol)

- [ ] Mode 2026 DECSET/DECRQM plumbing — regression guard for core-layer plumbing (apex pins are Section 06's)
- [ ] Mode 2026 bridge cell — parser → `mode_cache` → Section 06 publication gate
- [ ] Mode 2031 notification-on-theme-change — regression guard for color scheme reporting
- [ ] Mode 2031 Theme::Unknown — no-notification policy pin (positive + negative)
- [ ] Mode 2031 no-backfill-on-enable pin (positive + negative)
- [ ] Mode 2031 no-notification-on-no-op pin (positive + negative — load-bearing `theme == theme` early-return guard at `term/mod.rs:385-387`)
- [ ] Mode 2031 no-notification-on-construction pin (positive + negative)
- [ ] Mode 2031 alt-screen / inactive-pane policy pin (documented + tested)
- [ ] Mode 66 reconciliation with DECKPAM/DECKPNM — 4-cell matrix, regression guard for keypad mode SSOT; DECRQM reports state not provenance
- [ ] Mode 67 backspace encoding switch — cross-crate key encoding modifier matrix (10 cells); Ctrl swaps polarity under DECBKM
- [ ] Mode 67 end-to-end bridge through `pane_mode()` — embedded backend AND daemon backend; both MUST agree

### Bridge cells (executable seams Section 09 owns — SSOT contract between parser and consumer)

- [ ] **Bridge 1004 (focus)** — REQUIRED: parser → `focus_event_seq_for_mode()` returns `Some(seq)` → colocated test in `oriterm/src/app/event_loop_helpers/focus_events/tests.rs`
- [ ] **Bridge 2026 (sync update)** — REQUIRED: parser → `mode_cache` atomic → colocated test in `oriterm_mux/src/pane/io_thread/tests.rs` or new sibling; proves parser → mode_cache → publication-gate SSOT contract is live for Section 06's apex consumer
- [ ] **Bridge 67 (DECBKM)** — REQUIRED: embedded + daemon `pane_mode()` paths both tested (is also a semantic pin above — double-duty)
- [ ] **Bridge 1007 (alt scroll)** — OPTIONAL/best-effort: only if `should_translate_wheel_to_arrows()` helper can be extracted from `oriterm/src/app/mouse_report/mod.rs` without destabilizing the module; otherwise file `/add-bug` for the refactor and SKIP (the 1007 app-shell apex is itself a deferred bug — see row-ownership table)
**Bridge 9001 (Win32 input) — OUT OF SCOPE for Section 09 (documentation, not a task):** dispatch is not yet wired at `oriterm/src/key_encoding/mod.rs:111-115`. Section 17 (or a prerequisite inside 17) owns the bridge once the dispatch seam exists. Section 09's coverage for 9001 is flag + DECRQM only. This paragraph is intentionally NOT a `- [ ]` task — it documents the exclusion so the drift is visible if the ownership rule changes.

### DECRQM & sync-point hygiene

- [ ] **DECRQM cross-cutting**: every mode verified or implemented by this section has its DECRQM query/response tested; 09.5 DECRQM exhaustive matrix is the drift-gate
- [ ] **Sync point**: all new `NamedPrivateMode` variants (ColorSchemeUpdate, DecNumericKeypad, DecBackarrowKey) added to `decset_decrst_flag_sync()` — post-09.0-split path confirmed via `grep -rn 'fn decset_decrst_flag_sync' oriterm_core/src/term/handler/`
- [ ] **Sync point**: all new `NamedPrivateMode` variants handled in `status_report_private_mode()` at `oriterm_core/src/term/handler/status.rs:117` (automatic via `named_private_mode_flag()` delegation — but verify with a dedicated exhaustive iteration test in 09.5)
- [ ] **Sync point**: all new `NamedPrivateMode` variants have a matching `PrivateMode::new()` number → variant arm in `crates/vte/src/ansi/types.rs` (post-09.0 — no split needed here, but verify each new variant has exactly one `NN => Self::Named(...)` arm)

### Catalog updates

- [ ] **Section 09-promoted rows** (reach `verified` via flag + DECRQM; apex = `effect-mode-state`): `DEC-X10-MOUSE`, `DEC-MOUSE-CLICKS`, `DEC-MOUSE-DRAG`, `DEC-MOUSE-MOTION`, `DEC-UTF8-MOUSE`, `DEC-SGR-MOUSE`, `DEC-URXVT-MOUSE`, `DEC-ALT-SCREEN-47`, `DEC-ALT-SCREEN-1047`, `DEC-SAVE-CURSOR-1048`, `DEC-SIXEL-SCROLLING`, `DEC-SIXEL-CURSOR-RIGHT`, `DEC-DECNRCM`.
- [ ] **Section 09-implemented + verified rows** (new catalog rows or promoted from `missing`): `DEC-DECNKM` (mode 66), `DEC-DECBKM` (mode 67), new mode 2031 row (ColorSchemeUpdate).
- [ ] **Rows NOT promoted by Section 09** (flag coverage added but catalog status stays unchanged — apex owned elsewhere or deferred): `DEC-FOCUS-IN-OUT` (stays `implemented-unverified`; apex → Section 16), `DEC-ALT-SCROLL` (stays `stub`; apex → deferred bug, no current owner), `DEC-URGENCY-HINTS` (stays `stub`; → deferred bug), `DEC-WIN32-INPUT` (stays `stub`; encoding → Section 17), `DEC-DECANM` (stays `missing`; → Section 19), `DEC-SGR-PIXEL-MOUSE` (stays `missing`; → Section 16), `catalog/mode-2026.md` rows (stay at current status; apex → Section 06).
- [ ] Mode 2026 core-layer plumbing verified (flag + DECRQM + bridge to Section 06's consumer — apex remains Section 06's)
- [ ] Mode 2031 color scheme update verified (using existing `Theme` type, NOT a new `ColorScheme` type); Theme::Unknown policy pin documented in catalog Notes column
- [ ] Mode 66 (DECNKM) implemented and reconciled with DECKPAM/DECKPNM (4-cell matrix passes)
- [ ] Mode 67 (DECBKM) implemented with cross-crate key encoding update AND end-to-end bridge
- [ ] Catalog row for mode 2031 added to `catalog/dec-private-modes.md` (no existing row — must be added)
- [ ] Mode 1042 host-notification gap filed as bug via `/add-bug` (subsystem: Core Terminal / Effect boundary)

### Test file organization (09.0 — OPTIONAL maintainability refactor)

Per `.claude/rules/code-hygiene.md` §File Size, `tests.rs` files are EXEMPT from the 500-line limit — 09.0 is a maintainability-driven refactor, not a rule-compliance prerequisite. The checklist below applies ONLY if 09.0 is executed. If 09.0 is skipped (because the added test surface is small enough to land cleanly in the existing files), mark this whole block as N/A with a brief note explaining the judgment.

- [x] (If 09.0 executed) `oriterm_core/src/term/handler/tests.rs` is DELETED; directory module `oriterm_core/src/term/handler/tests/` is in place *(done: commit 408a0d8a — 12 submodules)*
- [x] (If 09.0 executed) `oriterm/src/key_encoding/tests.rs` is DELETED; directory module `oriterm/src/key_encoding/tests/` is in place *(done: 7 submodules, 167 tests)*
- [x] No new inline `#[cfg(test)] mod tests { ... }` blocks introduced anywhere in the section's changes (test-organization.md rule 1 — applies regardless of 09.0)
- [x] No new source file (non-`tests.rs`) exceeds the 500-line limit; `tests.rs` files remain exempt per code-hygiene.md §File Size, but post-09.0 submodule files are kept under ~1500 lines as a maintainability target (not a hard rule) *(largest split file: kitty_precedence.rs ~750 lines)*

### Final verification

- [ ] All existing teseq tests pass (`timeout 150 cargo test -p oriterm_core --test teseq`)
- [ ] All existing tack tests pass (`timeout 150 cargo test -p oriterm_core --test tack`)
- [ ] Alloc regression unchanged (`timeout 150 cargo test -p oriterm_core --test alloc_regression`)
- [ ] RSS regression unchanged (`timeout 150 cargo test -p oriterm_core --test rss_regression`)
- [ ] `./build-all.sh` green (the script runs both debug and release plus Windows cross-compile from WSL: `cargo build --target x86_64-pc-windows-gnu` — per `.claude/rules/tests.md` §Cross-Platform Verification)
- [ ] `./test-all.sh` green (debug workspace test sweep — `cargo test --workspace --features oriterm/gpu-tests`, no `--release`; timeout-capped per `.claude/rules/tests.md`)
- [ ] Explicit release-mode test run: `timeout 150 cargo test --workspace --features oriterm/gpu-tests --release` green — required because `./test-all.sh` only covers debug; release-mode failures (optimizer-induced alloc regressions, panic-unwind differences, `#[cfg(debug_assertions)]`-gated code divergence) are invisible to `./test-all.sh`
- [ ] `./clippy-all.sh` green — no new warnings
- [ ] Plan annotation cleanup (remove `<!-- blocked-by:... -->` tags if the bridge cells satisfy the block)
- [ ] Section frontmatter `status` → `complete`; `09.0`–`09.5` sub-entries all `complete`
- [ ] `00-overview.md` Quick Reference updated (do NOT tick the "Mode 2026 fully wired" mission criterion — that belongs to Section 06; DO tick "Verification chain complete per row" for the Section-09-promoted rows)
- [ ] `index.md` section 09 status updated
- [ ] `/tpr-review` passed (dual-source: codex + gemini both clean)
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Section 09-owned DEC private mode catalog rows are `verified` (flag + DECRQM); excluded rows (DECANM, 1016, 1007 apex, 9001 encoding, 1042 host notification, mode-2026 apex) stay at their current status with cross-references pointing at the owning section; Mode 2031 implemented + verified; Modes 66 and 67 implemented + verified; all DECRQM queries return correct responses; `decset_decrst_flag_sync()` updated for new modes; mode 1042 gap filed as a bug.
