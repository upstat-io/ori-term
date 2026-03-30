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

- [ ] `[BUG-08-2][high]` **Selection highlight cannot be dismissed — sticks after selecting text** — found by manual.
  Repro: Select text in the terminal grid (click-drag or double-click). Try to dismiss the highlight by clicking elsewhere, pressing Escape, or any other action. The selection highlight persists until keyboard input is typed into the PTY.
  Detail: `clear_pane_selection()` is only called from two paths: (1) keyboard input to PTY (`keyboard_input/mod.rs:195`) and (2) terminal output dirty flag (`mux_pump/mod.rs:66-67`). There is no dismissal on: single left-click without drag, Escape key, or any explicit "deselect" action. Every left-click in `handle_press()` creates a new `PressAction::New(Selection)` — even a single click creates a zero-width char selection rather than clearing to `None`. The fix should: (a) treat single-click-then-release-without-drag as "clear selection" (replace the zero-width selection with `None`), and (b) add Escape key binding to dismiss selection.
  Subsystem: `oriterm/src/app/mouse_input.rs`, `oriterm/src/app/mouse_selection/mod.rs`, `oriterm/src/app/keyboard_input/mod.rs`
  Found: 2026-03-30 | Source: manual
  Note: Active work in roadmap section 09 (Selection & Clipboard) touches this area.
