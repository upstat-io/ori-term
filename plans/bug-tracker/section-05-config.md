---
section: "05"
title: "Config Bugs"
status: in-progress
reviewed: true
goal: "Track and fix bugs in configuration loading, settings application, and config-to-runtime wiring"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-03-29
sections:
  - id: "05.1"
    title: "Active Bugs"
    status: in-progress
  - id: "05.R"
    title: "Third Party Review Findings"
    status: complete
---

# Section 05: Config Bugs

**Status:** In Progress
**Goal:** Track and fix bugs where configuration values are parsed/stored but not wired into runtime behavior.

**Note:** This section is never marked complete. New bugs are appended as discovered.

---

## 05.1 Active Bugs

- [x] **BUG-05.1**: Default shell setting in config does nothing — value never passed to PTY spawn
  - **File(s)**: `oriterm/src/app/init/mod.rs`, `tab_management/mod.rs`, `pane_ops/mod.rs`, `pane_ops/floating.rs`, `window_management.rs`
  - **Root cause**: All 5 `SpawnConfig` construction sites omitted the `shell` field, defaulting to `None`.
  - **Found**: 2026-03-29 — manual, user report
  - **Fixed**: 2026-03-29 — Added `shell: self.config.terminal.shell.clone()` to all 5 `SpawnConfig` construction sites.

- [ ] **BUG-05.2**: No configurable cell letter-spacing for terminal grid — cursor sits too tight against prompt
  - **File(s)**: `oriterm/src/config/font_config.rs` (config), `oriterm_gpu/src/renderer.rs` (cell placement), `oriterm/src/font/shaper/` (glyph advance)
  - **Root cause**: The terminal grid cell width is derived purely from the font's advance width with no user-adjustable spacing offset. The UI text system has `letter_spacing` but the terminal grid does not. Other terminals (Windows Terminal, Alacritty via `offset.x`, WezTerm via `cell_width`) let users tune horizontal cell spacing. The default (0 extra spacing) makes the cursor feel cramped against the prompt text.
  - **Found**: 2026-03-29 — manual, user report. Gap varies wildly between terminals (Windows Terminal has a large one by default).
  - **Fix**: Add `font.cell_spacing` (or `font.offset.x`) config field — an additive pixel offset applied to cell width during grid layout. Default to a small positive value (1-2px at 96 DPI). Wire into `GlyphCache::cell_width` computation and the GPU cell placement loop. Also expose in the settings UI (Terminal or Font page).

- [x] **BUG-05.4**: "Restore previous session" toggle in settings does nothing — no session save/restore logic exists
  - **File(s)**: `oriterm/src/app/settings_overlay/form_builder/window.rs`
  - **Root cause**: Toggle was interactive but session persistence feature doesn't exist yet.
  - **Found**: 2026-03-29 — manual, user report
  - **Fixed**: 2026-03-30 — Disabled the toggle with `.with_disabled(true)` and changed description to "Not yet implemented — coming in a future release". Config field preserved for forward compatibility.

- [x] **BUG-05.5**: Settings dialog save path (`apply_settings_change`) is incomplete — many settings have no effect
  - **File(s)**: `oriterm/src/app/keyboard_input/overlay_dispatch.rs`, `oriterm/src/app/config_reload/mod.rs`
  - **Root cause**: `apply_settings_change()` called only 4 of 7 apply methods, missing `apply_behavior_changes`, `apply_image_changes`, `apply_keybinding_changes`.
  - **Found**: 2026-03-29 — manual, user report
  - **Fixed**: 2026-03-29 — Added the 3 missing apply calls. Widened visibility of the 3 methods from private to `pub(in crate::app)` so both call sites can use them.

- [x] **BUG-05.3**: Zoom font size actions (ZoomIn/ZoomOut/ZoomReset) have no test coverage
  - **File(s)**: `oriterm/src/app/config_reload/mod.rs:413` (`zoom_font_size`, `reset_font_size`), `oriterm/src/app/keyboard_input/action_dispatch.rs:237` (dispatch arms)
  - **Root cause**: The zoom helpers and their dispatch wiring landed without matching tests. Existing keyboard dispatch tests in `keyboard_input/tests.rs` stop at copy/PTY cases. This violates the repo rule that behavior changes ship with tests.
  - **Found**: 2026-03-29 — tpr-review (Codex)
  - **Fix**: Add unit tests for `zoom_font_size()` (clamp behavior, no-op on boundary) and `reset_font_size()` (reads config file default, no-op when already at default). Also test dispatch wiring.

---

## 05.R Third Party Review Findings

- [x] `[TPR-05-001][medium]` `oriterm/src/app/config_reload/tests.rs:1`, `oriterm/src/app/config_reload/mod.rs:452`, `oriterm/src/app/keyboard_input/action_dispatch.rs:237` — `BUG-05.3` was closed before the current tests covered the new reset path or the zoom action wiring.
  Resolved: Added `compute_reset_size()` pure function with 4 tests (noop, different, after zoom in/out). Dispatch arms are thin wrappers calling `zoom_font_size(delta)` and `reset_font_size()` — they require full App with GPU/platform state and cannot be unit-tested per crate boundary rules. The pure computation logic is now fully covered. Resolved 2026-03-29.

---
