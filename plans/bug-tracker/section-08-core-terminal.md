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

- [ ] `[BUG-08-10][high]` **`Term::image_cache()` / `image_cache_mut()` return the inactive cache in `ALT_SCREEN` mode — alt-mode placements leak into primary after `swap_alt` back** — found by continue-roadmap.
  Repro: Place image in primary mode, `term.swap_alt()`, call `term.image_cache().placement_count()` — returns 1 (should be 0 for the empty alt cache). Or: place image while in alt mode, `term.swap_alt()` back to primary, `image_cache().placement_count()` returns both primary's original image + the alt-mode image (should return primary only).
  Detail: `toggle_alt_common` (`oriterm_core/src/term/alt_screen.rs:86-89`) swaps the contents of the `image_cache` and `alt_image_cache` fields so that `image_cache` always holds the **currently active** cache. But `image_cache()` / `image_cache_mut()` (`oriterm_core/src/term/mod.rs:336-364`) in `ALT_SCREEN` mode return `self.alt_image_cache.as_ref()` (the inactive cache after swap) instead of `&self.image_cache` (the active cache). Both accessors should simply return `&self.image_cache` unconditionally (the swap ensures it is always the active cache). Verified by a temporary sanity-check test: `assert_eq!(primary_count, 1)` fails with `left: 1, right: 0` after entering alt mode then reading `image_cache().placement_count()`.
  Impact: Any VTE handler that routes through `image_cache_mut` while in alt mode (all sixel / kitty / iTerm2 image writes) writes to the wrong cache. Images placed while in alt-screen apps (vim image viewers, fullscreen graphics TUIs) leak into primary cache on exit and vice versa.
  Subsystem: `oriterm_core/src/term/mod.rs` (accessors), `oriterm_core/src/term/alt_screen.rs` (swap invariant)
  Found: 2026-04-13 | Source: continue-roadmap
  Note: Discovered during `plans/spec-conformance/` Section 07 (Image Lifecycle Correctness) implementation while writing `term_resize_updates_alt_cache_when_alt_exists` test. Section 07's `Term::resize` already operates on both `self.image_cache` and `self.alt_image_cache` fields directly, so it correctly resizes both caches regardless of this accessor bug — but the bug still affects all protocol-handler write paths.

- [ ] `[BUG-08-9][medium]` **`Term::set_cell_dimensions` has no production caller — FixedPixels image placements never get updated cell coverage at runtime** — found by tpr-review.
  Repro: Change font size at runtime with a sixel FixedPixels image on screen. The image's `cols`/`rows` coverage is never recalculated because `set_cell_dimensions` (at `image_config.rs:17`) is only called in tests. `PaneIoCommand` has no cell-dimension variant; `App::handle_dpi_change` and `sync_grid_layout` do not forward cell metrics to the IO-thread `Term`.
  Detail: Cross-crate plumbing gap: `oriterm/src/app/` must detect cell-metric changes (font size, DPI), `oriterm_mux/` needs a new `PaneIoCommand` variant to transport `(cell_w, cell_h)`, and the IO thread handler must call `term.set_cell_dimensions(w, h)`. Without this, `ImageCache::update_cell_coverage` is dead code in production.
  Subsystem: `oriterm_core/src/term/image_config.rs`, `oriterm_mux/src/pane/io_thread/`, `oriterm/src/app/`
  Found: 2026-04-13 | Source: tpr-review | Reviewer: codex (TPR-07-001-codex during spec-conformance Section 07 review)
  Note: Active work in `plans/spec-conformance/` Section 07 (Image Lifecycle Correctness) explicitly scopes this out; section 07 handles grid-dimension-only resizes where cell metrics are unchanged.

- [x] `[BUG-08-3][low]` **vttest.rs exceeds 500-line file size limit (956 lines)** — found by tpr-review.
  Found: 2026-04-03 | Source: tpr-review
  Fixed: 2026-04-03 — Split into `tests/vttest/` directory with per-menu modules (main.rs, session.rs, pty_size.rs, menu1-8.rs). Largest file is 239 lines. All 29 tests pass. 207 snapshots regenerated under new module paths.

- [x] `[BUG-08-2][high]` **Selection highlight cannot be dismissed — sticks after selecting text** — found by manual.
  Found: 2026-03-30 | Source: manual
  Root cause: Every left-click created a `PressAction::New(Selection)` — even single clicks without drag. `handle_release()` only cleared button flags, never the selection. No Escape handling existed.
  Fixed: 2026-03-30 — Two changes: (1) `clear_click_selection()` on mouse-up without drag clears `Char` mode selections (single click), preserving Word/Line selections from double/triple click. (2) Escape key dismisses active selection before falling through to PTY encoding.
