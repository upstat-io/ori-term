---
section: "05"
title: "Config Bugs"
status: in-progress
reviewed: true
goal: "Track and fix bugs in configuration loading, settings application, and config-to-runtime wiring"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "05.1"
    title: "Active Bugs"
    status: in-progress
  - id: "05.R"
    title: "Third Party Review Findings"
    status: not-started
---

# Section 05: Config Bugs

**Status:** In Progress
**Goal:** Track and fix bugs where configuration values are parsed/stored but not wired into runtime behavior.

**Note:** This section is never marked complete. New bugs are appended as discovered.

---

## 05.1 Active Bugs

- [ ] **BUG-05.1**: Default shell setting in config does nothing — value never passed to PTY spawn
  - **File(s)**: `oriterm/src/app/init/mod.rs`, `oriterm/src/app/tab_management/mod.rs`, `oriterm/src/app/pane_ops/mod.rs`, `oriterm/src/app/pane_ops/floating.rs`, `oriterm/src/app/window_management.rs`
  - **Root cause**: All 5 `SpawnConfig` construction sites set `scrollback` and `shell_integration` from `self.config.terminal` but omit the `shell` field — using `..SpawnConfig::default()` which defaults to `None`. The settings UI correctly writes to `config.terminal.shell` (action_handler line 173), and `spawn_pty()` correctly reads `config.shell` (spawn.rs line 225), but the value is never threaded through.
  - **Found**: 2026-03-29 — manual, user report
  - **Fix**: Add `shell: self.config.terminal.shell.clone()` to all 5 `SpawnConfig` construction sites. One-liner per site.

- [ ] **BUG-05.2**: No configurable cell letter-spacing for terminal grid — cursor sits too tight against prompt
  - **File(s)**: `oriterm/src/config/font_config.rs` (config), `oriterm_gpu/src/renderer.rs` (cell placement), `oriterm/src/font/shaper/` (glyph advance)
  - **Root cause**: The terminal grid cell width is derived purely from the font's advance width with no user-adjustable spacing offset. The UI text system has `letter_spacing` but the terminal grid does not. Other terminals (Windows Terminal, Alacritty via `offset.x`, WezTerm via `cell_width`) let users tune horizontal cell spacing. The default (0 extra spacing) makes the cursor feel cramped against the prompt text.
  - **Found**: 2026-03-29 — manual, user report. Gap varies wildly between terminals (Windows Terminal has a large one by default).
  - **Fix**: Add `font.cell_spacing` (or `font.offset.x`) config field — an additive pixel offset applied to cell width during grid layout. Default to a small positive value (1-2px at 96 DPI). Wire into `GlyphCache::cell_width` computation and the GPU cell placement loop. Also expose in the settings UI (Terminal or Font page).

---

## 05.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---
