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

- [ ] `[BUG-08-7][high]` **Kitty delete dispatch has 4 wrong specifier mappings** — found by tpr-review.
  Repro: Send Kitty graphics delete commands with d=a, d=c, d=p, d=r specifiers. Behavior diverges from protocol spec.
  Detail: Verified against official Kitty protocol docs (sw.kovidgoyal.net/kitty/graphics-protocol/) and WezTerm reference. Errors: (1) d=a clears ALL images+placements (spec: visible placements only); (2) d=c uses cursor column (spec: cursor position — cell intersection); (3) d=p does placement-ID deletion (spec: cell position x,y intersection); (4) d=r does cursor-position deletion (spec: image-ID range x≤id≤y). Also missing: d=q/Q (cell+z-index deletion), d=f/F (animation frame deletion).
  Subsystem: `oriterm_core/src/term/handler/image/kitty.rs`
  Found: 2026-04-05 | Source: tpr-review (TPR iteration 4)
  Note: Active work in roadmap section 39 (Image Protocols) covers Kitty image support.

- [ ] `[BUG-08-8][high]` **`kitty.rs` is 476 lines — BLOAT-adjacent; must split before Sections 12 / 13 implementation** — found by continue-roadmap.
  Repro: `wc -l oriterm_core/src/term/handler/image/kitty.rs` prints `476`.
  Detail: `oriterm_core/src/term/handler/image/kitty.rs` sits 24 lines below the 500-line hard limit defined in `.claude/rules/code-hygiene.md` §File Size (also ~26 lines above the ~450-line proactive-split threshold). Sections 12 (Sixel) and 13 (Kitty Graphics) in `plans/spec-conformance/` are explicitly blocked on splitting this file — starting their implementation work on a 476-line file guarantees a 500+ overflow as soon as new per-action code lands, which would violate the hard limit AND force a mid-section refactor at the worst time (feature work mixed with mechanical moves). This bug is orthogonal to `BUG-08-7`: that bug is about protocol-spec correctness of the delete-specifier arms (semantic); this bug is about file size / structural BLOAT (plumbing).
  Proposed fix: extract per-action handlers into submodules at `oriterm_core/src/term/handler/image/kitty/` — `transmit.rs`, `place.rs`, `delete.rs`, `animate.rs`, `query.rs`, `frame_compose.rs`. Keep `kitty/mod.rs` as the dispatch entry point that reads `KittyCommand::action` and routes. Follow the sibling `tests.rs` pattern per `.claude/rules/test-organization.md`. Aim for every split file ≤ 200 lines.
  Subsystem: `oriterm_core/src/term/handler/image/kitty.rs`
  Found: 2026-04-11 | Source: continue-roadmap
  Blocking consumers: `plans/spec-conformance/section-12-sixel.md` and `plans/spec-conformance/section-13-kitty-graphics.md`. Neither section's frontmatter `depends_on:` is edited (that field uses section-number grammar); the linkage lives in each section's body text as a `**Blocker note:**` paragraph AND as a completion-checklist entry in each section's own `## 12.N` / `## 13.N` block.
  Reference rules: `.claude/rules/code-hygiene.md` §File Size.
  Note: Active work in `plans/spec-conformance/` Section 01 (Catalog Bootstrap) discovered and filed this bug while harvesting the Kitty APC `_G` dispatch arms (Section 01.11). Sections 12 (Sixel) and 13 (Kitty Graphics) will consume the fix.

- [ ] `[BUG-08-13][high]` **Numpad keys produce no output — Enter, digits, operators all dead**
  Repro: With numpad active (NumLock on), press numpad Enter / digits / +,-,*,/ / ".". Nothing is sent to the PTY; shell sees no keystrokes.
  Detail: Winit reports numpad keys with `location = KeyLocation::Numpad` and in many cases `Key::Named(...)` (Enter) or `Key::Character("+")`/etc. The dispatcher in `oriterm/src/key_encoding/mod.rs:116` only special-cases numpad when `APP_KEYPAD` is set (line 128 → `legacy::encode_numpad_app`). Outside APP_KEYPAD it falls through to `legacy::encode_legacy`, which is expected to handle normal numpad output (NumpadEnter → CR, digits → their ASCII chars, operators → their ASCII chars). Suspect: `legacy::encode_legacy` does not cover the numpad `Key::Named` / `Key::Character` variants that winit emits with `KeyLocation::Numpad`, or `input.text` is `None` on these events and no fallback fires. Needs verification of which winit variant is delivered for each numpad key and a matching arm in legacy encoding (plus Kitty path at `key_encoding/kitty.rs` — per the dispatch priority in `mod.rs:106-109`, Kitty mode is checked FIRST, so the same gap likely exists there).
  Subsystem: `oriterm/src/key_encoding/legacy.rs`, `oriterm/src/key_encoding/kitty.rs`, `oriterm/src/key_encoding/mod.rs` (dispatch), `oriterm/src/app/keyboard_input/` (upstream KeyInput construction)
  Found: 2026-04-14 | Source: manual
  Note: Related to BUG-08-12 (keyboard-mode / encoding routing) but orthogonal — that bug is about Kitty mode persisting after program exit; this bug is about numpad keys producing no bytes in ANY mode.

- [ ] `[BUG-08-12][high]` **Kitty keyboard mode persists after program exit — shell renders raw CSI u sequences instead of typed characters**
  Repro: Run `notcurses-demo` (or any program that pushes Kitty keyboard protocol modes). After it exits and you return to the shell prompt, typed characters display as raw CSI u escape fragments like `0;1;100u7;1;97u` instead of the actual letters. Terminal is effectively unusable until `reset` is typed blind or the pane is killed. Other terminals (WezTerm, Ghostty) do not exhibit this.
  Detail: The `keyboard_mode_stack` in `Term` retains pushed Kitty keyboard protocol flags after the child program exits. When the shell regains control, `TermMode::KITTY_KEYBOARD_PROTOCOL` is still set, so `key_encoding/mod.rs:118` routes all keypresses through the Kitty CSI u encoder (`key_encoding/kitty.rs`). The shell (bash/zsh) doesn't understand CSI u encoding and displays the raw parameter bytes. Root cause: no mechanism resets the keyboard mode stack when a subprocess terminates without sending `CSI < u` (pop). Programs may crash, be killed via SIGKILL, or simply forget to clean up. RIS (`ESC c`) does clear the stack (`esc.rs:50-51`), but nothing triggers RIS automatically on subprocess exit. Possible fixes: (1) shell integration resets keyboard mode stack on prompt detection, (2) detect when the direct child shell re-emits its prompt and auto-pop any modes the shell didn't push, (3) app-layer heuristic that pops keyboard modes when the pane's foreground process group changes.
  Subsystem: `oriterm_core/src/term/handler/dcs.rs` (push/pop/set), `oriterm_core/src/term/mod.rs` (keyboard_mode_stack), `oriterm/src/key_encoding/mod.rs` (mode check at line 118)
  Found: 2026-04-14 | Source: manual
  Note: Active work in spec-conformance section-17 (Kitty Keyboard) and roadmap section-08 (Keyboard Input) touch this area.

- [ ] `[BUG-08-11][medium]` **`term/tests.rs` combines tests for multiple submodules (2500+ lines) — violates test-organization.md**
  Repro: `wc -l oriterm_core/src/term/tests.rs` prints ~2570. Contains tests for `mod.rs`, `alt_screen.rs`, `image_config.rs`, `snapshot.rs`, and `resize.rs`.
  Subsystem: `oriterm_core/src/term/tests.rs`
  Found: 2026-04-14 | Source: tpr-review
  Reviewer: gemini (TPR-07-001-gemini round 15 during spec-conformance Section 07 close-out)
  Proposed fix: Convert `alt_screen.rs`, `image_config.rs`, `snapshot.rs`, and `resize.rs` to directory modules. Extract their tests from `term/tests.rs` into per-module `tests.rs` files. Verify with `./test-all.sh`.

- [ ] `[BUG-08-14][medium]` **Mode 1042 (urgency hints): flag toggle works but BEL-to-window-manager-hint path is missing**
  Repro: `printf '\x1b[?1042h'` then `printf '\a'` — expected: window manager urgency hint; actual: no effect beyond existing visual bell.
  Detail: `TermMode::URGENCY_HINTS` flag toggles correctly via DECSET/DECRST and DECRQM reports correctly. But no `HostEffect::UrgencyHint` variant exists, and the BEL handler (`oriterm_core/src/term/handler/mod.rs`) does not check mode 1042 to conditionally emit a host-adapter urgency hint. Need: (1) new Effect variant for urgency hint, (2) mode-1042-gated emission in the BEL handler, (3) app-layer host-adapter wiring to platform window-manager urgency API.
  Subsystem: `oriterm_core/src/term/handler/mod.rs` (BEL handler), `oriterm_core/src/effect.rs`, `oriterm/src/app/mux_pump/mod.rs`
  Found: 2026-04-15 | Source: continue-roadmap
  Note: Active work in spec-conformance Section 09 touches mode 1042 flag verification.

- [ ] `[BUG-08-15][medium]` **Mode 1007 (alt-scroll): wheel-to-arrow app-shell apex has no owning roadmap section**
  Repro: `printf '\x1b[?1049h\x1b[?1007h'` then scroll mouse wheel — expected: arrow key sequences sent to PTY; actual: works (Tier-2 gate at `mouse_report/mod.rs:196`), but no spec-conformance section owns end-to-end verification.
  Detail: The `should_translate_wheel_to_arrows(mode, shift_held)` function (extracted by Section 09.1) correctly gates wheel-to-arrow translation. Bridge tests verify the parser→flag→decision path. But catalog row `DEC-ALT-SCROLL` stays `stub` because no roadmap section owns the full app-shell integration apex (actual mouse wheel event → pane write). Section 09 verified the core-layer flag toggle + DECRQM + the bridge cell; the remaining gap is the app-shell integration test that proves a real mouse wheel event flows through `handle_mouse_wheel()` Tier 2 into the PTY.
  Subsystem: `oriterm/src/app/mouse_report/mod.rs` (Tier-2 wheel-to-arrow), catalog `DEC-ALT-SCROLL`
  Found: 2026-04-15 | Source: continue-roadmap
  Note: Section 09.1 bridge cell covers the pure-function decision path; this bug tracks the integration apex.

- [x] `[BUG-08-10][high]` **`Term::image_cache()` / `image_cache_mut()` return the inactive cache in `ALT_SCREEN` mode — alt-mode placements leak into primary after `swap_alt` back** — found by continue-roadmap.
  Found: 2026-04-13 | Source: continue-roadmap
  Fixed: 2026-04-13 — Resolved as part of spec-conformance Section 07 round-6 TPR triage (TPR-07-001 codex+gemini agreement). The deeper issue was that `toggle_alt_common` swapped `image_cache`/`alt_image_cache` field contents but did NOT swap `grid`/`alt_grid`, leaving `Term::resize` pairing the primary grid with whichever cache happened to be in the `image_cache` field regardless of semantic ownership. Root-cause fix: remove the image-cache field swap from `toggle_alt_common` entirely. The fields now carry their semantic contents at all times — `image_cache` = primary, `alt_image_cache` = alt. `image_cache()` / `image_cache_mut()` route by `ALT_SCREEN` mode (mirroring `grid()` / `grid_mut()`) to return the active screen's cache without touching field contents. Regression test `term_resize_routes_each_grid_through_its_own_image_cache` reads the fields directly to prevent future routing inversions; `alt_image_cache_isolation_check` verifies primary/alt placements no longer leak across swaps.

- [x] `[BUG-08-9][medium]` **`Term::set_cell_dimensions` has no production caller — FixedPixels image placements never get updated cell coverage at runtime** — found by tpr-review.
  Found: 2026-04-13 | Source: tpr-review | Reviewer: codex (TPR-07-001-codex during spec-conformance Section 07 review)
  Fixed: 2026-04-13 — Resolved by spec-conformance Section 07.6. Added `PaneIoCommand::SetCellDimensions { width, height }` variant, `MuxPdu::SetCellDimensions`, `MsgType::SetCellDimensions = 0x012B`, `MuxBackend::set_cell_dimensions` trait method with EmbeddedMux + MuxClient + daemon server dispatch implementations. App-layer: 6 pane creation sites now seed metrics at pane spawn (init/create_initial_tab, init/create_handoff_tab, tab_management/new_tab_in_window, pane_ops/split_pane, pane_ops/floating/toggle_floating_pane, window_management/create/create_window), plus `sync_grid_layout` and `handle_dpi_change` broadcast to every pane in the affected window via `App::broadcast_cell_metrics_to_window`. IO-thread handler calls `Term::set_cell_dimensions` and marks grid dirty. Tests: `test_set_cell_dimensions_command_marks_dirty` (mux plumbing), `set_cell_dimensions_missing_pane_is_noop` (EmbeddedMux safety), `roundtrip_set_cell_dimensions` + zero-metrics wire roundtrip (protocol).

- [x] `[BUG-08-3][low]` **vttest.rs exceeds 500-line file size limit (956 lines)** — found by tpr-review.
  Found: 2026-04-03 | Source: tpr-review
  Fixed: 2026-04-03 — Split into `tests/vttest/` directory with per-menu modules (main.rs, session.rs, pty_size.rs, menu1-8.rs). Largest file is 239 lines. All 29 tests pass. 207 snapshots regenerated under new module paths.

- [x] `[BUG-08-2][high]` **Selection highlight cannot be dismissed — sticks after selecting text** — found by manual.
  Found: 2026-03-30 | Source: manual
  Root cause: Every left-click created a `PressAction::New(Selection)` — even single clicks without drag. `handle_release()` only cleared button flags, never the selection. No Escape handling existed.
  Fixed: 2026-03-30 — Two changes: (1) `clear_click_selection()` on mouse-up without drag clears `Char` mode selections (single click), preserving Word/Line selections from double/triple click. (2) Escape key dismisses active selection before falling through to PTY encoding.
