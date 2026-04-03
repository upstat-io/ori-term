---
section: 8
title: "Core Terminal"
domain: "VTE handler, terminal emulation, bell, escape sequences"
status: in-progress
---

# Section 08: Core Terminal

Terminal emulation behavior — VTE handler, bell, escape sequences, terminal modes.

## Open Bugs

- [ ] `[BUG-08-1][medium]` **Audible bell not implemented — `printf '\a'` produces no sound** — found by manual.
  Repro: Run `printf '\a'` in the terminal. Expected: audible beep or system sound. Actual: silence.
  Detail: VTE handler emits `Event::Bell` correctly (`oriterm_core/src/term/handler/mod.rs:112`). App handler in `oriterm/src/app/mux_pump/mod.rs:103` triggers visual tab bar pulse via `ring_bell()` but plays no system sound. `BellConfig` (`oriterm/src/config/bell.rs`) only covers visual bell (animation, duration, color). No audible bell or OS notification exists. Roadmap section 27 plans `behavior.bell = "none" | "visual" | "notification"` but is not yet implemented.
  Subsystem: `oriterm/src/app/mux_pump/mod.rs`, `oriterm/src/config/bell.rs`
  Found: 2026-03-29 | Source: manual
  Note: Active work in roadmap section 27 (command palette) plans bell notification modes.

- [ ] `[BUG-08-4][low]` **vttest LNM key encoding not testable in headless mode** — found by vttest conformance audit.
  Repro: vttest menu 6 sub-item 2 (LineFeed/NewLine mode). vttest sets LNM, presses RETURN, expects CR+LF. Gets bare CR.
  Detail: LNM IS correctly implemented in both VTE handler (`handler/mod.rs:117`) and key encoding (`key_encoding/legacy.rs:165`). The failure is in test infrastructure: `VtTestSession` sends raw `\r` bytes to PTY, bypassing the key encoding layer. Would need VtTestSession to route through key encoding when simulating keypresses -- significant infrastructure change.
  Subsystem: `oriterm_core/tests/vttest/session.rs`
  Found: 2026-04-03 | Source: vttest conformance audit

- [ ] `[BUG-08-5][low]` **DA3 qualifier test fails in vttest menu 6** — found by vttest conformance audit.
  Repro: vttest menu 6 sub-item 6, screen 2. Shows `<13> failed` for a second DA3-related query.
  Detail: DA3 (tertiary device attributes) basic response implemented (`status.rs` responds to `CSI = c` with `DCS ! | 00000000 ST`). Screen 1 now passes. Screen 2 tests a DA3 qualifier/variant that we don't handle. Would require vttest source analysis to identify the specific query.
  Subsystem: `oriterm_core/src/term/handler/status.rs`
  Found: 2026-04-03 | Source: vttest conformance audit

- [ ] `[BUG-08-6][low]` **ENQ/Answerback not implemented** — found by vttest conformance audit.
  Repro: vttest menu 6 sub-item 1 (answerback test). No response displayed.
  Detail: ENQ (0x05) control code not handled in VTE C0 dispatcher. WezTerm implements it (defaults to empty string), Alacritty does not. Would need: (1) add ENQ to VTE C0 dispatch, (2) add handler method to Handler trait, (3) implement in Term. Low priority -- most terminals don't support configurable answerback.
  Subsystem: `crates/vte/src/ansi/dispatch/mod.rs`, `oriterm_core/src/term/handler/mod.rs`
  Found: 2026-04-03 | Source: vttest conformance audit

- [x] `[BUG-08-3][low]` **vttest.rs exceeds 500-line file size limit (956 lines)** — found by tpr-review.
  Found: 2026-04-03 | Source: tpr-review
  Fixed: 2026-04-03 — Split into `tests/vttest/` directory with per-menu modules (main.rs, session.rs, pty_size.rs, menu1-8.rs). Largest file is 239 lines. All 29 tests pass. 207 snapshots regenerated under new module paths.

- [x] `[BUG-08-2][high]` **Selection highlight cannot be dismissed — sticks after selecting text** — found by manual.
  Found: 2026-03-30 | Source: manual
  Root cause: Every left-click created a `PressAction::New(Selection)` — even single clicks without drag. `handle_release()` only cleared button flags, never the selection. No Escape handling existed.
  Fixed: 2026-03-30 — Two changes: (1) `clear_click_selection()` on mouse-up without drag clears `Char` mode selections (single click), preserving Word/Line selections from double/triple click. (2) Escape key dismisses active selection before falling through to PTY encoding.
