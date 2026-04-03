---
section: "04"
title: "Scattered Knowledge and Bloat"
status: in-progress
reviewed: true
goal: "Consolidate scattered theme application, fix blinking_active duplication, resolve BLOAT files and dead code"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-04-03
sections:
  - id: "04.1"
    title: "Consolidate Theme Application"
    status: complete
  - id: "04.2"
    title: "Consolidate blinking_active Formula"
    status: complete
  - id: "04.3"
    title: "BLOAT File Splits"
    status: complete
  - id: "04.4"
    title: "Dead Code and Stale Attributes"
    status: complete
  - id: "04.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "04.N"
    title: "Completion Checklist"
    status: in-progress
---

# Section 04: Scattered Knowledge and Bloat

**Status:** Not Started
**Goal:** Consolidate scattered theme application into one canonical method, unify the `blinking_active` computation, split BLOAT files, and clean up dead code.

**Context:** Theme application to chrome widgets (tab_bar + status_bar + cache invalidation) is copy-pasted at 3 sites, and they have drifted — site 1 is the most complete while sites 2 and 3 are missing invalidation steps. The `blinking_active` formula is computed identically at 2 sites via different data paths. Three source files exceed the 500-line limit.

**Testing feasibility:** All methods modified in this section are on `App`, which requires GPU context, wgpu surfaces, and winit windows. Unit testing is infeasible (same constraint as section 03). Verification relies on: (1) `./build-all.sh` (including `--target x86_64-pc-windows-gnu`), (2) `./clippy-all.sh`, (3) `./test-all.sh` (existing tests pass, especially `widget_pipeline/tests.rs` after 04.4), (4) structural grep confirmation, and (5) `/tpr-review`. The `cursor_should_blink` helper in 04.2 is a trivial boolean AND (`config_flag && terminal_flag`) that does not warrant a dedicated test. No new unit tests are expected.

---

## 04.1 Consolidate Theme Application

**File(s):** `oriterm/src/app/event_loop_helpers/mod.rs` (new method), `oriterm/src/app/mod.rs`, `oriterm/src/app/keyboard_input/overlay_dispatch.rs`, `oriterm/src/app/config_reload/mod.rs`

Theme application (tab_bar.apply_theme + status_bar.apply_theme + cache invalidation) appears at 3 call sites, but **they have diverged** — the mod.rs site is the most complete while the other two are missing invalidation steps:

| Site | File | Conditional? | `text_cache.clear()` | `invalidation_mut().invalidate_all()` | `damage_mut().reset()` | `ui_stale = true` |
|------|------|-------------|---------------------|--------------------------------------|----------------------|-------------------|
| 1 | `mod.rs:378-388` (handle_theme_changed) | No | Yes | Yes | Yes | **No** |
| 2 | `overlay_dispatch.rs:134-148` (apply_settings_change) | Yes (`!= self.ui_theme`) | **No** | **No** | **No** | **No** |
| 3 | `config_reload/mod.rs:110-127` (apply_config_reload) | Yes (`!= self.ui_theme`) | **No** | Yes | Yes | Yes |

The missing steps in sites 2 and 3 are bugs — theme changes that don't clear the text shape cache or reset damage tracking can leave stale cached glyphs or skip repainting.

- [x] Extract a method on `App` in `event_loop_helpers/mod.rs` (currently 407 lines). Place it in the `impl App` block, in the "public operations" section per code-hygiene.md method ordering. The method MUST NOT go in `mod.rs` (currently 500 lines — at the hard limit):
  ```rust
  /// Apply the current UI theme to all window chrome widgets and invalidate caches.
  ///
  /// This is the canonical theme-application path. All sites that change
  /// `self.ui_theme` must call this afterwards instead of manually applying.
  pub(super) fn apply_theme_to_chrome(&mut self) {
      for ctx in self.windows.values_mut() {
          ctx.tab_bar.apply_theme(&self.ui_theme);
          ctx.status_bar.apply_theme(&self.ui_theme);
          ctx.pane_cache.invalidate_all();
          ctx.text_cache.clear();
          ctx.root.invalidation_mut().invalidate_all();
          ctx.root.damage_mut().reset();
          ctx.root.mark_dirty();
          ctx.ui_stale = true;
      }
  }
  ```
  This is a superset of all three sites. Site 1 was missing `ui_stale = true` — benign for site 1 because `mux.set_pane_theme()` triggers `content_dirty`, but the canonical helper must be safe for all callers. The `for ctx in self.windows.values_mut()` borrow of `self.windows` and `&self.ui_theme` borrow of `self.ui_theme` are disjoint fields; NLL permits this (verified: site 1 already compiles this pattern at mod.rs:380-388).

- [x] Replace site 1 (`mod.rs:378-388`, handle_theme_changed): replace the 8-line loop body with `self.apply_theme_to_chrome();`. The `self.ui_theme = resolve_ui_theme_with(...)` assignment stays at the call site — it requires the `system_theme` parameter from winit's `ThemeChanged` event. This reduces `mod.rs` by ~7 lines (500 to ~493), giving headroom. The helper now adds `ui_stale = true` which site 1 was missing — a correctness improvement.

- [x] Replace site 2 (`overlay_dispatch.rs:134-148`, apply_settings_change): replace BOTH the theme application loop (lines 138-141) AND the separate cache invalidation loop (lines 144-148) with a single call. The theme conditional still gates the `self.ui_theme` assignment, but the invalidation runs unconditionally because `apply_settings_change` is called after font/color/cursor/window/behavior config changes from the settings dialog — these changes affect rendering even if the theme itself didn't change:
  ```rust
  // Update UI theme.
  let new_theme = super::super::resolve_ui_theme(&self.config);
  if new_theme != self.ui_theme {
      self.ui_theme = new_theme;
  }
  // Always invalidate chrome — even if theme didn't change, settings
  // changes to fonts/colors/cursor affect cached text shapes and rendering.
  self.apply_theme_to_chrome();
  ```
  This fixes site 2's missing `text_cache.clear()`, `invalidation_mut().invalidate_all()`, `damage_mut().reset()`, and `ui_stale = true`.

- [x] Replace site 3 (`config_reload/mod.rs:110-127`, apply_config_reload): same pattern as site 2. Replace BOTH the theme application loop (lines 114-117) AND the separate cache invalidation loop (lines 121-127) with:
  ```rust
  // Update UI chrome theme if the config override changed.
  let new_theme = super::resolve_ui_theme(&self.config);
  if new_theme != self.ui_theme {
      self.ui_theme = new_theme;
  }
  self.apply_theme_to_chrome();
  ```
  This replaces the separate `for ctx in self.windows.values_mut()` loop that set `pane_cache.invalidate_all()`, `root.invalidation_mut().invalidate_all()`, `root.damage_mut().reset()`, `root.mark_dirty()`, and `ui_stale = true` — all of which are now in the helper. Net reduction in `config_reload/mod.rs` is ~12 lines (524 to ~512), which helps toward the BLOAT split in 04.3. This also fixes site 3's missing `text_cache.clear()`.

- [x] Verify that `resolve_ui_theme_with` (used by site 1 with system_theme param) and `resolve_ui_theme` (used by sites 2/3, detects system theme via platform call) are correctly handled. The helper only applies chrome — the `self.ui_theme` assignment and theme resolution remain at each call site.
- [x] Verify `event_loop_helpers/mod.rs` stays under 500 lines after adding the helper: 407 + ~16 = ~423 lines. Actual: 424.
- [x] Run `./build-all.sh`, `./clippy-all.sh`, and `./test-all.sh`.

---

## 04.2 Consolidate blinking_active Formula

**File(s):** `oriterm/src/app/event_loop.rs`, `oriterm/src/app/redraw/post_render.rs`, `oriterm/src/app/event_loop_helpers/mod.rs`

The `blinking_active` derivation appears at 2 sites with the same formula:
- `event_loop.rs:140-143`: `self.blinking_active = self.config.terminal.cursor_blink && self.terminal_mode().is_some_and(|m| m.contains(TermMode::CURSOR_BLINKING))`
- `post_render.rs:49`: `self.blinking_active = self.config.terminal.cursor_blink && blinking_now` (where `blinking_now` is `frame.content.mode.contains(TermMode::CURSOR_BLINKING)`)

Same formula, different data access paths. They cannot be unified to a single call site because they serve different timing needs: the focus handler (event_loop.rs) must set `blinking_active` immediately so the first `drive_blink_timers()` call after focus gain is correct; the post-render path updates from the freshest frame data after each render. Both are necessary.

- [x] Extract a named helper on `App` in `event_loop_helpers/mod.rs`:
  ```rust
  /// Whether cursor blinking is enabled: config allows it AND the terminal
  /// has set the `CURSOR_BLINKING` mode via DECSCUSR.
  pub(super) fn cursor_should_blink(&self, terminal_blinking: bool) -> bool {
      self.config.terminal.cursor_blink && terminal_blinking
  }
  ```

- [x] Update `event_loop.rs:140-143` to use `cursor_should_blink()`.

- [x] Update `post_render.rs:49` to use `cursor_should_blink()`.

- [x] Run `./build-all.sh` and `./clippy-all.sh`.

---

## 04.3 BLOAT File Splits

**File(s):** Multiple files exceeding 500-line limit.

| File | Actual Lines | Status | Action |
|------|-------------|--------|--------|
| `config_reload/mod.rs` | 524 | Over limit | Split — extract `apply_window_changes` to submodule |
| `window_management.rs` | 503 | Over limit | Split — extract window creation helpers into submodule |
| `tab_bar_input/mod.rs` | 502 | Over limit | Split — extract tab editing methods into submodule |
| `event_loop.rs` | 481 | Under limit | Monitor — safe after sections 01-03 |
| `mod.rs` | 500 | At limit | 04.1 theme extraction reduces by ~7 lines to ~493 |
| `frame_input/mod.rs` | 391 | Under limit | Already fixed by section 01 (was 531) |

**Implementation order:** Complete 04.1 (theme consolidation) before 04.3 (BLOAT splits). The 04.1 changes reduce `config_reload/mod.rs` by ~12 lines and `mod.rs` by ~7 lines; the 04.3 target line counts assume those reductions are in place.

### config_reload/mod.rs (524 -> target <500)

The file already has submodules `color_config.rs` and `font_config.rs`. The largest remaining method is `apply_window_changes` (lines 330-427, ~98 lines).

- [x] Extract `apply_window_changes` (lines 330-427, ~98 lines) to a new `config_reload/window_config.rs` submodule. This method handles opacity, blur, decorations, and tab bar position changes. The new file needs:
  - Module doc comment (`//! Window transparency, blur, decoration, and tab bar config changes.`)
  - Imports: `use crate::config::Config;` and `use super::super::App;` at minimum. The method also references `super::init::decoration_to_mode`, `super::init::metrics_from_style`, `oriterm_ui::window::resolve_winit_decorations`, `crate::config::TabBarPosition`. Check all references and add necessary imports. From inside `window_config.rs`, `super` is `config_reload`, so `super::super` is `app`. References to `super::init::*` become `super::super::init::*`.
  - `impl App { ... }` block wrapping the method.
  - The method's visibility stays `pub(in crate::app)`.
  - `#[cfg(target_os = "macos")]` blocks within the method (lines 378-384 and 403-408) must be preserved as-is.
- [x] Add `mod window_config;` to `config_reload/mod.rs`.
- [x] Verify `config_reload/mod.rs` drops below 500 lines: ~512 (after 04.1) - 98 = ~414 lines. Actual: 410.
- [x] Run `./build-all.sh` and `./clippy-all.sh`.

### window_management.rs (503 -> target <500)

The file has two groups: window creation (`create_window` at line 27, ~93 lines; `create_window_bare` at line 130, ~84 lines; `create_window_renderer` at line 217, ~72 lines = ~249 lines total) and window closing/cleanup (`close_window` at line 295, rest of file).

- [x] Convert `window_management.rs` to a directory module:
  1. `mkdir oriterm/src/app/window_management`
  2. `git mv oriterm/src/app/window_management.rs oriterm/src/app/window_management/mod.rs`
  3. Extract window creation methods (lines 19-288: the `impl App` block opening through `create_window_renderer`) to `window_management/create.rs`.

  The parent `mod.rs` declaration `mod window_management;` resolves to either `window_management.rs` or `window_management/mod.rs` in Rust 2018+, so no change to `app/mod.rs` is needed.

- [x] The new `create.rs` file needs:
  - Module doc comment (`//! Window creation helpers.`)
  - Its own `impl App { ... }` block wrapping the three methods.
  - Imports: copy the relevant subset from `window_management/mod.rs`. At minimum: `use winit::event_loop::ActiveEventLoop;`, `use winit::window::WindowId;`, `use crate::session::WindowId as SessionWindowId;`, `use oriterm_ui::window::WindowConfig;`, `use super::super::App;`, `use super::super::window_context::WindowContext;`, plus crate imports used by `create_window_renderer` (`crate::font::*`, `crate::gpu::*`, `crate::window::TermWindow`).
  - The method visibilities (`pub(super)`, private) are preserved.
- [x] Add `mod create;` to the new `window_management/mod.rs`. Remove the creation methods from `mod.rs`, keeping the closing/cleanup methods and the existing imports needed for those.
- [x] Verify line counts: `window_management/mod.rs` drops to ~254 lines. `window_management/create.rs` is ~294 lines. Actual: mod.rs=230, create.rs=294.
- [x] Run `./build-all.sh` and `./clippy-all.sh`.

### tab_bar_input/mod.rs (502 -> target <500)

The file has tab bar mouse handling, context menus, and tab editing. The tab editing block is a natural extraction point: `commit_tab_edit` (line 356-380), `cancel_tab_edit` (line 382-389), `handle_tab_editing_key` (line 391-470) — 115 lines total including doc comments.

- [x] Extract tab editing methods to `tab_bar_input/editing.rs`. The new file needs:
  - Module doc comment (`//! Tab title inline editing.`)
  - Imports: `use winit::event::ElementState;`, `use super::super::App;`, `use super::tab_edit_key_action;`, `use super::TabEditAction;` at minimum. The `tab_edit_key_action` free function and `TabEditAction` enum are private items in `tab_bar_input/mod.rs` — in Rust, private items are visible to child modules, so `editing.rs` can access them via `super::` without any visibility change.
  - `impl App { ... }` block wrapping the three methods.
  - Method visibilities (`pub(super)`) are preserved.
- [x] Add `mod editing;` to `tab_bar_input/mod.rs`.
- [x] Verify `tab_bar_input/mod.rs` drops below 500 lines: 502 - 115 = ~387 lines (plus `mod editing;` declaration). Actual: 388.
- [x] Run `./build-all.sh` and `./clippy-all.sh`.

### Post-split verification

- [x] After all splits, run `find oriterm/src/app/ -name '*.rs' ! -name 'tests.rs' -exec wc -l {} + | awk '$1 > 500'` and verify zero results.
- [x] Run `./test-all.sh` to confirm no regressions from file splits (existing `tab_bar_input/tests.rs` and `config_reload/tests.rs` still pass).

---

## 04.4 Dead Code and Stale Attributes

**File(s):** Various.

1. **Stale `#[allow(dead_code)]` on `move_tab_to_new_window`** (`tab_management/move_ops.rs:66`): This function IS called from `event_loop.rs:345` (`TermEvent::MoveTabToNewWindow(tab_id) => self.move_tab_to_new_window(tab_id, event_loop)`). The `dead_code` allow with reason "superseded by tear_off_tab in Section 17.2" is stale — the function is live code.
   - [x] Remove the `#[allow(dead_code, reason = "superseded by tear_off_tab in Section 17.2")]` attribute from `move_tab_to_new_window` at line 66 of `tab_management/move_ops.rs`.
   - [x] Run `./build-all.sh` to confirm no dead_code error.

2. **Pre-built dead fields** (`window_context.rs:83-92`): `render_strategy` and `damage` fields have `#[expect(dead_code)]` with reason "vocabulary for retained-ui plan". This is acceptable given the explicit plan reference — no action needed.
   - [x] No action (informational — tracked for awareness).

3. **Stale blanket `#[allow(dead_code)]` on `widget_pipeline` module** (`mod.rs:43-47`): The `widget_pipeline` module has a broad `#[allow(dead_code)]` with reason "incremental pipeline -- delivery loop wired in OverlayManager migration". This is stale: the module is heavily used in production code (verified 7+ production call sites across `dialog_rendering.rs`, `dialog_context/`, `redraw/draw_helpers.rs`). Within the module, two items are dead in production:
   - `apply_requests` (function at lines 14-26): zero callers in production AND zero callers in tests. Completely dead code.
   - `DispatchResult` (re-export at line 8): used only in `widget_pipeline/tests.rs`, never in production.

   - [x] Remove the blanket `#[allow(dead_code, reason = "...")]` from the `mod widget_pipeline;` declaration in `mod.rs:43-47`. The declaration becomes simply `mod widget_pipeline;`.
   - [x] In `widget_pipeline/mod.rs`:
     - **Delete** the `apply_requests` function entirely (lines 14-26, including the doc comment). It has zero callers anywhere.
     - Move `DispatchResult` from the production re-export (line 8) into the existing `#[cfg(test)]` re-export (line 12). Result:
       ```rust
       pub(crate) use oriterm_ui::pipeline::{
           apply_dispatch_requests, collect_all_widget_ids, collect_focusable_ids,
           deregister_widget_tree, prepaint_widget_tree, prepare_widget_tree,
           register_widget_tree,
       };
       #[cfg(test)]
       pub(crate) use oriterm_ui::pipeline::{
           DispatchResult, dispatch_step, prepare_widget_frame,
       };
       ```
   - [x] Run `./build-all.sh` to confirm no compile errors. Blanket allow removal produced no additional dead_code warnings.
   - [x] Run `./clippy-all.sh` to verify no new clippy warnings.
   - [x] Run `./test-all.sh` to confirm `widget_pipeline/tests.rs` still passes (tests import `DispatchResult` via `super::DispatchResult` which is now `#[cfg(test)]` — tests compile with `cfg(test)` so this works).

---

## 04.R Third Party Review Findings

- [x] `[TPR-04-001][low]` `.claude/rules/code-hygiene.md:89-94`, `oriterm/src/gpu/frame_input/mod.rs:404-418`, `plans/hygiene-last-commit/section-01-redraw-pipeline.md:95-96` — Section 01 touches `oriterm/src/gpu/frame_input/mod.rs` to add `FrameInput::clear_transient_fields()`, but the file now measures 531 lines, exceeding the repo's hard 500-line limit for touched source files. **Resolved**: Section 01 completion (TPR-01-001) already split `frame_input/mod.rs` (531 to 391 lines) by extracting `FrameSearch` to `search.rs`. The file is now well under the limit.

---

## 04.N Completion Checklist

- [x] Theme application exists in exactly one method (`apply_theme_to_chrome`), called from all 3 sites
- [x] All 3 sites perform the same complete invalidation (no more missing `text_cache.clear()`, `damage_mut().reset()`, or `ui_stale = true`)
- [x] `mod.rs` is under 500 lines (492)
- [x] `blinking_active` formula is defined in one helper (`cursor_should_blink`), called from both sites with cross-reference comments
- [x] All source files under 500 lines (test files exempt)
  - [x] `config_reload/mod.rs` under 500 (410)
  - [x] `window_management/mod.rs` under 500 (230)
  - [x] `tab_bar_input/mod.rs` under 500 (388)
- [x] Stale `dead_code` allow removed from `move_tab_to_new_window`
- [x] Stale blanket `dead_code` allow removed from `widget_pipeline` module; `apply_requests` deleted (zero callers); `DispatchResult` re-export moved to `#[cfg(test)]` block
- [x] `widget_pipeline/tests.rs` still passes (DispatchResult accessible via `#[cfg(test)]` re-export)
- [x] No new unit tests required (GPU/platform methods — see testing feasibility note)
- [x] `./test-all.sh` green
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** Theme application has one canonical home with complete invalidation. No source file in `oriterm/src/app/` exceeds 500 lines. All dead_code attributes are accurate (no stale allows on live code, dead items properly gated or removed).
