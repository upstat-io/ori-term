---
section: "09"
title: "DEC Private Modes (full)"
status: not-started
reviewed: false
goal: "Drive every catalog row in `catalog/dec-private-modes.md` and `catalog/mode-2026.md` beyond the baseline subset (covered by section 08) from `implemented-unverified` to `verified`, including the obscure modes (1003 motion, 1004 focus events, 1015 URXVT, 1016 SGR pixels, 2026 sync timeout, 2031 color scheme update)."
success_criteria:
  - "Every row in `catalog/dec-private-modes.md` not covered by section 08's baseline subset is `verified` via spec_chain tests"
  - "Every row in `catalog/mode-2026.md` is `verified`, including the Begin/Commit/Abort apex tests that depend on section 06's timeout-abort wiring"
  - "Mode 2026 timeout-abort path tested end-to-end via spec_chain harness — feeds BSU + writes, waits >150ms (real wall-clock via StdSyncHandler), asserts `PresentationEffect::Abort { reason: SyncAbortReason::Timeout }` effect emitted and snapshot_seqno advances by exactly 1"
  - "Mode 2031 (color scheme update notification) is `verified` — terminal emits the notification when the user/host changes color scheme"
  - "All existing teseq mode tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row** AND **Mode 2026 fully wired** mission criteria"
inspired_by:
  - "contour-terminal mode 2026 spec — `docs/vt-extensions.md` reference for sync output semantics"
  - "xterm `ctlseqs.html` — every numbered private mode and its semantics"
  - "kitty docs — mode 2031 color scheme notification reference"
depends_on: ["03", "06", "08"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "09.1"
    title: "Verify obscure DEC private modes (mouse 1003-1016, focus 1004, etc.)"
    status: not-started
  - id: "09.2"
    title: "Verify Mode 2026 sync output (suppression + commit + abort)"
    status: not-started
  - id: "09.3"
    title: "Verify Mode 2031 color scheme update notification"
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
**Goal:** Verify every DEC private mode catalog row beyond the baseline subset, including the obscure modes, Mode 2026 (with the timeout-abort path now wired by section 06), and Mode 2031.

**Success Criteria:** see frontmatter.

**Context:** Section 08 verifies the basic mode subset (1, 5, 6, 7, 12, 25, 47, 1049, 2004 — the modes tack covers) AND DECLRMM (mode 69) including full grid enforcement, CSI s / DECSLRM ambiguity, and save/restore/reset paths. This section drives the remainder: 1000-1003 mouse, 1004 focus, 1005 UTF-8 mouse, 1006 SGR mouse, 1007 alternate scroll, 1015 URXVT mouse, 1016 SGR pixels, 1042 urgency hints, 1047 alt screen, 1048 save cursor only, 2026 sync, 2031 color scheme update, 8452 sixel cursor right, 9001 Win32 input, plus any obscure modes section 01 cataloged. (Note: DECLRMM was moved from Section 09 to Section 08 because it's a baseline correctness prerequisite for Phase 3 stacks.)

**Reference implementations:**
- **xterm** `ctlseqs.html` — definitive numbered-mode reference
- **contour-terminal** `docs/vt-extensions.md` — Mode 2026 spec
- **kitty** docs — Mode 2031 color scheme notification

**Depends on:** Section 06 (timeout-abort wired so Mode 2026 tests can verify the abort apex), Section 08 (baseline modes verified so this section's Phase 3 tests have a solid baseline).

**Hard coupling with Section 06 (non-negotiable)**: the Mode 2026 timeout-abort tests in 09.2 are **non-executable** until section 06.1 and 06.3 land. Section 06 adds the `Processor::sync_timeout` / `Processor::stop_sync` call sites in `oriterm_mux/src/pane/io_thread/mod.rs` AND emits `Effect::Presentation(PresentationEffect::Abort { reason: SyncAbortReason::Timeout })` on timeout. Without those call sites, the tests in 09.2 will deadlock or time out. Section 09 MUST NOT start 09.2 until section 06 is `status: complete`. Section 06's mode metadata consolidation (eliminating `named_private_mode_number()`) does not block 09.1 — the exhaustive matches in `named_private_mode_flag()` serve as the canonical enumeration mechanism.

---

## 09.1 Verify obscure DEC private modes

**File(s):** `oriterm_core/tests/spec_chain/private_modes/*.rs` (new — one file per mode family)

- [ ] For each catalog row in `catalog/dec-private-modes.md` not covered by section 08, write a spec_chain test that toggles the mode, verifies the mode bit is set, and (where applicable) verifies the downstream behavior change.
- [ ] Mouse modes (1000, 1001, 1002, 1003): toggling on causes mouse events to encode + emit through `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::MouseEvent })`. Test the encoding for each mode's wire format.
- [ ] Focus events (1004): toggling on causes focus gain/loss to emit. Test in section 16 (mouse) since focus encoding shares the input encoder.
- [ ] **Alt scroll (1007) IMPLEMENTED (no defer)**: when mode 1007 is enabled AND the alternate screen is active, the terminal translates mouse wheel events into arrow key presses (scroll-up = Up arrow, scroll-down = Down arrow) so that apps like `less` on the alt screen respond to wheel events as expected. Pass 1 found the parse exists but the behavior is a no-op stub. This subsection implements the behavior: wheel events check the mode flag + alt-screen state, and when both are set, emit `Effect::Pty(PtyEffect::Write { kind: KeyboardEvent, bytes: arrow_key_bytes(scroll_dir) })` instead of the mouse encoding. Spec_chain test enables alt screen + mode 1007, simulates wheel-up, asserts the Up arrow bytes are emitted on the PTY. Catalog row marked `verified` (NOT `verified-with-deviation`).
- [ ] **Validation**: every catalog row in `catalog/dec-private-modes.md` reaches `verified`. `verified-with-deviation` is reserved for intentional-deviation rows with citation — not for implementation-skip rows.

---

## 09.2 Verify Mode 2026 sync output (suppression + commit + abort)

**File(s):** `oriterm_core/tests/spec_chain/private_modes/mode_2026.rs` (new)

The Mode 2026 verification chain has three apex tests: publication suppression during sync, atomic commit on sync end, and timeout-abort. Section 06 wires the timeout-abort; this section verifies all three apex behaviors.

- [ ] `mode_2026_suppression_test`: feed BSU + writes, snapshot_seqno does not advance, observed snapshot reflects pre-BSU state
- [ ] `mode_2026_commit_test`: feed BSU + writes + ESU, snapshot_seqno advances by exactly 1, observed snapshot reflects all writes atomically, `Effect::Presentation(PresentationEffect::Begin)` and `Effect::Presentation(PresentationEffect::Commit { snapshot_seqno: N+1 })` both observed in transcript
- [ ] `mode_2026_timeout_abort_test`: feed BSU + writes, wait >150ms (real wall-clock via StdSyncHandler timeout), `Effect::Presentation(PresentationEffect::Abort { reason: SyncAbortReason::Timeout })` observed, snapshot_seqno advances by exactly 1, observed snapshot reflects all buffered writes
- [ ] `mode_2026_nested_bsu_handled`: feed BSU + BSU + writes + ESU, only the outer ESU triggers commit (or document the actual behavior per spec)
- [ ] Update `catalog/mode-2026.md` rows to `verified`.
- [ ] **Validation**: all three apex tests pass; existing mode tests still pass.

---

## 09.3 Implement + verify Mode 2031 color scheme update notification

**File(s):** `oriterm_core/src/term/handler/modes.rs` (mode 2031 flag), `oriterm_core/src/term/mod.rs` (scheme change hook), `oriterm_core/tests/spec_chain/private_modes/mode_2031.rs` (new)

Mode 2031 is full implementation work, not just verification. Pass 1 did not explicitly confirm the mode flag exists, so this subsection assumes it must be added. The work has three parts: (1) parse/recognize mode 2031 in the DECSET/DECRST dispatch, (2) add a scheme-change hook that the host app calls when the user flips dark/light mode, (3) emit the notification PTY bytes when the hook fires AND mode 2031 is enabled.

- [ ] Read kitty docs / contour-terminal spec for Mode 2031 semantics — the notification wire format is `CSI ? 997 ; 1 n` (dark) or `CSI ? 997 ; 2 n` (light)
- [ ] Verify mode 2031 has a `NamedPrivateMode` variant and entries in `named_private_mode_flag()` and `apply_decset()`/`apply_decrst()` (section 06's sync-point structure)
- [ ] Add `Term::on_color_scheme_change(scheme: ColorScheme)` method — called by the host (oriterm app shell) when the user or host platform reports a scheme change
- [ ] When `on_color_scheme_change` fires AND mode 2031 is enabled, emit `Effect::Pty(PtyEffect::Write { bytes: scheme_notification_bytes, kind: PtyWriteKind::Other })`
- [ ] Spec_chain tests:
  - `mode_2031_disabled_no_notification_on_scheme_change()` — scheme changes, no PTY write
  - `mode_2031_dark_scheme_emits_997_1_notification()` — mode enabled + dark scheme
  - `mode_2031_light_scheme_emits_997_2_notification()` — mode enabled + light scheme
  - `mode_2031_mode_toggle_does_not_emit_notification_by_itself()` — only real scheme changes trigger
- [ ] Update `catalog/dec-private-modes.md` row for mode 2031 to `verified`
- [ ] **Validation**: tests pass; mode 2031 implementation is NOT a stub.

---

## 09.R Third Party Review Findings

- None.

---

## 09.N Completion Checklist

- [ ] Failing test matrix written FIRST
- [ ] **Matrix dimensions**: every numbered DEC private mode × set/unset/query × downstream behavior (where applicable)
- [ ] **Semantic pin**: Mode 2026 three-apex tests (suppression, commit, abort) — these are the regression guards for the entire sync output semantics
- [ ] All non-baseline DEC private mode catalog rows are `verified`
- [ ] Mode 2026 sync output verified (suppression + commit + abort)
- [ ] Mode 2031 color scheme update verified
- [ ] All existing teseq tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated (Mode 2026 fully wired now fully checked off)
- [ ] `index.md` section 09 status updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** All DEC private mode catalog rows verified; Mode 2026 three-apex tests green; Mode 2031 verified.
