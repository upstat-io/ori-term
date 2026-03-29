# Section 24: Visual Polish -- Verification Results

**Verified:** 2026-03-29
**Auditor:** verify-roadmap agent (Claude Opus 4.6)
**Section status:** in-progress
**Reviewed gate:** false (not yet reviewed by /review-plan)

## Context Loaded

- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/CLAUDE.md` (full)
- `.claude/rules/code-hygiene.md`, `.claude/rules/impl-hygiene.md`, `.claude/rules/test-organization.md` (3 rule files; the 4th listed `crate-boundaries.md` was loaded via system reminder)
- `plans/roadmap/section-24-visual-polish.md` (full, 593 lines)

---

## 24.1 Cursor Blinking -- COMPLETE

**Plan status:** complete
**Actual status:** VERIFIED COMPLETE

### Files Verified

- `oriterm/src/app/cursor_blink/mod.rs` (90 lines) -- `CursorBlink` struct with `epoch`, `interval`, `last_visible`. Methods: `new()`, `is_visible()`, `set_interval()`, `reset()`, `update()`, `next_toggle()`. Visibility is a pure function of elapsed time (no drift). `#[cfg(test)] mod tests;` at bottom.
- `oriterm/src/app/cursor_blink/tests.rs` (143 lines) -- 8 tests covering all plan items.

### Tests Run

```
cargo test -p oriterm -- cursor_blink
13 passed (8 blink unit tests + 3 integration: settings toggle, config defaults, config TOML)
```

### Test-to-Plan Coverage

| Plan Test Item | Test Function | Status |
|---|---|---|
| Blink state reports change after interval | `update_after_interval_reports_change` | PASS |
| Keypress resets blink to visible | `reset_makes_visible` | PASS |
| Even DECSCUSR disables blinking | `decscusr_fires_cursor_blinking_change_event` (oriterm_core) | PASS |
| Odd DECSCUSR enables blinking | `decscusr_1_sets_blinking_block`, `decscusr_3_sets_blinking_underline`, `decscusr_5_sets_blinking_bar` | PASS |
| Focus loss freezes cursor visible | Integration only (event_loop.rs line 156: `cursor_blink.reset()`) -- no dedicated unit test | VERIFIED by code reading |
| Mouse click resets blink | Integration only (mouse_input.rs grid click path) -- no dedicated unit test | VERIFIED by code reading |
| Unfocused hollow cursor | `unfocused_window_renders_hollow_cursor`, `unfocused_window_bar_cursor_becomes_hollow`, `focused_window_renders_block_cursor` | PASS |

### Integration Wiring Verified

- `event_loop.rs` line 368: `cursor_blink.update()` drives blink timer in `about_to_wait`.
- `event_loop.rs` line 156: `cursor_blink.reset()` on `Focused(true/false)`.
- `event_loop.rs` line 449: `cursor_blink.next_toggle()` for `ControlFlow::WaitUntil`.
- `event_loop.rs` lines 138-140: `blinking_active` computed from `config.terminal.cursor_blink && terminal_mode`.

### Hygiene

- Module doc: present (`//! Cursor blink state machine.`)
- Test organization: sibling `tests.rs`, no inline tests, `super::` imports. Compliant.
- File size: 90 lines (well under 500).
- No `unwrap()` in production code.
- `DEFAULT_BLINK_INTERVAL` is `#[cfg(test)]` only -- not leaked to production.

---

## 24.2 Hide Cursor While Typing -- COMPLETE

**Plan status:** complete
**Actual status:** VERIFIED COMPLETE

### Files Verified

- `oriterm/src/app/cursor_hide/mod.rs` (60 lines) -- `HideContext` struct and `should_hide_cursor()` pure function. `is_modifier_only()` helper. `#[cfg(test)] mod tests;` at bottom.
- `oriterm/src/app/cursor_hide/tests.rs` (89 lines) -- 7 tests covering all plan items.
- `oriterm/src/config/behavior.rs` lines 71,88: `hide_mouse_when_typing: bool` (default: true).

### Tests Run

```
cargo test -p oriterm -- cursor_hide
7 passed
```

### Test-to-Plan Coverage

| Plan Test Item | Test Function | Status |
|---|---|---|
| Keypress hides when enabled | `keypress_hides_when_enabled` | PASS |
| Already-hidden skips redundant hide | `already_hidden_skips` | PASS |
| ANY_MOUSE prevents hiding | `mouse_reporting_prevents_hiding` | PASS |
| Config disabled prevents hiding | `config_disabled_skips` | PASS |
| Modifier-only key does not trigger | `modifier_only_does_not_hide` | PASS |
| IME composition does not hide | `ime_active_prevents_hiding` | PASS |
| Named action keys (Enter, Space, Backspace) trigger hiding | `named_action_keys_hide` | PASS |

### Integration Wiring Verified

- `keyboard_input/mod.rs` line 253: `should_hide_cursor()` called, sets `mouse_cursor_hidden = true`, calls `set_cursor_visible(false)`.
- `event_loop.rs` lines 147, 164, 183: `restore_mouse_cursor()` on CursorMoved, CursorLeft, Focused(false).
- `app/mod.rs` lines 454-458: `restore_mouse_cursor()` helper with guard.
- `app/constructors.rs` line 130: `mouse_cursor_hidden: false` initial state.

### Hygiene

- Module doc: present.
- `#[allow(clippy::struct_excessive_bools, reason = "...")]` on `HideContext` -- justified.
- File size: 60 lines, tests 89 lines. Compliant.

---

## 24.3 Minimum Contrast -- NOT STARTED

**Plan status:** not-started
**Actual status:** VERIFIED NOT STARTED

### What Exists

- Config field `minimum_contrast: f32` in `color_config.rs` line 44 (default 1.0).
- `effective_minimum_contrast()` in `color_config.rs` line 86 with NaN/inf clamping.
- Config tests pass: `minimum_contrast_defaults_to_off`, `minimum_contrast_clamped`, `minimum_contrast_nan_defaults_to_one`, `minimum_contrast_inf_clamped_to_twenty_one` (4 tests, all pass).

### What Does NOT Exist

- No `gpu/contrast/` module (Rust reference WCAG implementation).
- No WGSL shader modifications (`_pad` still named `_pad` in all 7 shaders, not `extra`).
- `write_screen_size()` not renamed to `write_uniforms()`.
- No `min_contrast` on `PreparedFrame`.
- No HIDDEN cell prepare phase change.

### Plan Accuracy

The plan's config checkbox `[x]` is correct -- the config field and clamping exist. All unchecked items are genuinely not started. The plan's detailed step-by-step is accurate to the current codebase state.

---

## 24.4 HiDPI & Display Scaling -- IN PROGRESS

**Plan status:** in-progress
**Actual status:** VERIFIED IN PROGRESS

### What Exists (Checked Items Verified)

- `ScaleFactor` tracked per-window on `TermWindow` (window/mod.rs line 55).
- `update_scale_factor()` on `TermWindow` (window/mod.rs line 235) -- returns bool on change.
- `ScaleFactorChanged` handler in event_loop.rs lines 110-116: updates scale factor, calls `handle_dpi_change()`, updates resize increments.
- `handle_dpi_change()` in app/mod.rs line 301: re-rasterizes fonts, clears atlas, marks dirty.
- sRGB pipeline with `AlphaBlending::LinearCorrected` option exists in config.
- Additional DPI check in `chrome/resize.rs` line 125 for frameless windows on Windows (manual `get_current_dpi()` check).

### What Does NOT Exist (Unchecked Items Verified)

- **Font zoom** (`increase_font_size()`, `decrease_font_size()`, `reset_font_size()`) -- grep returns zero matches. NOT implemented.
- **Multi-monitor DPI transition confirmation** -- marked unchecked in plan. Code exists but verification is manual.
- **BUG: Aero Snap text shrink** -- documented, not fixed.
- **BUG: Settings dialog DPI inheritance** -- documented, not fixed.

### Tests

- Plan lists 3 unchecked test items: `handle_dpi_change()` unit test, grid dimensions recalculation, multi-monitor DPI transition. NONE exist as dedicated tests. No test file at `app/mod_tests.rs` or similar for `handle_dpi_change()`.

### Gap: Missing Tests

The `handle_dpi_change()` function is complex (font re-rasterization, atlas clear, mux resize) but has ZERO unit tests. This is a coverage gap -- the function takes `&mut self` on `App` which requires GPU, but the font-size calculation logic could be extracted and tested headlessly.

---

## 24.5 Vector Icon Pipeline -- COMPLETE

**Plan status:** complete
**Actual status:** VERIFIED COMPLETE (with 2 deferred GPU tests)

### Files Verified

- `oriterm_ui/src/icons/mod.rs` (241 lines) -- `PathCommand`, `IconStyle`, `IconPath`, `IconId` (7 variants), `ResolvedIcon`, `ResolvedIcons`. All 7 static icon definitions with normalized coordinates. `#[cfg(test)] mod tests;`.
- `oriterm_ui/src/icons/tests.rs` (146 lines) -- 9 tests covering path validation, coordinate normalization, hashability, trait derivation.
- `oriterm/src/gpu/icon_rasterizer/mod.rs` (103 lines) -- `rasterize_icon()` function using `tiny_skia`. Scale-aware stroke width. `build_path()` helper. `#[cfg(test)] mod tests;`.
- `oriterm/src/gpu/icon_rasterizer/cache.rs` (121 lines) -- `IconCache` with `HashMap<CacheKey, AtlasEntry>`. `get_or_insert()` rasterizes on miss, uploads to mono atlas. `clear()` for DPI invalidation.
- `oriterm/src/gpu/icon_rasterizer/tests.rs` (131 lines) -- 9 tests covering rasterization sizes, anti-aliasing verification, cross-pattern check, scale-dependent stroke thickness.

### Tests Run

```
cargo test -p oriterm_ui -- icons       -> 9 passed
cargo test -p oriterm -- icon_rasterizer -> 9 passed
```

### Test-to-Plan Coverage

| Plan Test Item | Test Function | Status |
|---|---|---|
| Rasterize close at 16/24/32px with correct dimensions | `rasterize_close_at_multiple_sizes` | PASS |
| Different sizes produce different data | `different_sizes_produce_different_data` | PASS |
| Cache returns same AtlasRegion for same key | _requires GPU test harness_ | DEFERRED (plan acknowledges) |
| DPI change invalidates cache | _requires GPU test harness_ | DEFERRED (plan acknowledges) |
| Close icon at 2.0x has non-zero alpha on diagonal | `close_icon_has_antialiased_diagonals` | PASS |

### Widget Integration Verified

- `tab_bar/widget/draw.rs` line 347: `push_icon()` for tab close/plus/chevron.
- `tab_bar/widget/drag_draw.rs` line 74: `push_icon()` for dragged tab close.
- `window_chrome/controls.rs` lines 169-211: `push_icon()` for minimize, maximize, restore, window close.
- `dropdown/mod.rs` line 306: `push_icon()` for chevron down.
- `draw_list_convert/mod.rs` line 121: `DrawCommand::Icon` handled, routes to mono glyph writer.

### Phase 5 Cleanup Verified

- No `push_line()` fallback branches remain in `draw.rs`, `drag_draw.rs`, `controls.rs`, or `dropdown/mod.rs`.
- Remaining `push_line()` in `draw.rs` line 286 is for tab separators (not icons) -- correct.
- `ResolvedIcons` wired into `WindowRenderer` (mod.rs lines 162, 241, 348) and `ui_only.rs` line 109.

### Hygiene

- Module docs present on all files.
- Sibling `tests.rs` pattern followed.
- File sizes: all under 250 lines.
- `#[expect(clippy::too_many_arguments, reason = "...")]` on `get_or_insert()` -- justified.
- `#[allow(dead_code, reason = "...")]` on `len()` -- justified (test/diagnostics use).

---

## 24.6 Background Images -- NOT STARTED

**Plan status:** not-started
**Actual status:** VERIFIED NOT STARTED

- No `background_image` config field in `WindowConfig`.
- No `gpu/bg_image/` module.
- No `bg_image.wgsl` shader.
- No `bg_image_pipeline` in `GpuPipelines`.
- `image` crate is in `[build-dependencies]` and `[dev-dependencies]` only, not `[dependencies]`.

### File Size Concerns (Pre-existing)

- `render.rs` is 735 lines -- ALREADY exceeds the 500-line limit. The plan warns about this and recommends extraction before adding the background image pass. This is a pre-existing hygiene violation that must be addressed before 24.6 work begins.
- `pipeline/mod.rs` is exactly 500 lines -- at the hard limit. New pipeline creation functions must go in submodules as the plan states.

---

## 24.7 Background Gradients -- NOT STARTED

**Plan status:** not-started
**Actual status:** VERIFIED NOT STARTED

- No `GradientType` enum or gradient config fields.
- No `bg_gradient.wgsl` shader.
- No `bg_gradient_pipeline`.
- `oriterm_ui/src/draw/gradient.rs` exists (mentioned in plan as data structure reference) but is for UI-level gradients, not GPU-level background gradients.

---

## 24.8 Window Backdrop Effects -- IN PROGRESS

**Plan status:** in-progress
**Actual status:** VERIFIED IN PROGRESS (partially implemented)

### What Exists (Checked Items Verified)

- `oriterm/src/gpu/transparency.rs` (66 lines) -- `apply_transparency()` function with platform-specific `apply_blur()`:
  - **Windows**: `window_vibrancy::apply_acrylic()` with RGBA tint color.
  - **macOS**: `window_vibrancy::apply_vibrancy()` with `UnderWindowBackground` material.
  - **Linux**: `window.set_blur(true)` via winit.
  - **Fallback**: `#[cfg(not(any(...)))]` for unsupported platforms.
- `window/mod.rs` line 91: `apply_transparency()` called on window creation when `config.transparent && config.blur`.
- `window/mod.rs` line 268: `set_transparency()` for config reload.
- Config: `opacity: f32` and `blur: bool` on `WindowConfig` (config/mod.rs line 171).

### What Does NOT Exist (Unchecked Items Verified)

- No `Backdrop` enum (`None`, `Blur`, `Acrylic`, `Mica`, `Auto`).
- No Mica support (Windows 11 DWM `DWMSBT_MAINWINDOW`).
- No `auto` platform detection logic.
- No material selection config for macOS.
- No compositor detection logging for Linux.
- `blur: bool` is still the primary API -- not deprecated in favor of `Backdrop`.
- No config-to-`apply_transparency()` mapping for the new enum variants.
- No tests for backdrop config parsing or behavior.

### Hygiene

- `transparency.rs` is a plain file (66 lines), not a directory module -- no tests. The plan doesn't list any existing tests for 24.8, so this is consistent. The file is clean: module doc, platform `#[cfg]` at function level (slightly violates the "cfg at module level" rule but is pragmatic for a 3-platform dispatch of a single function).

---

## 24.9 Scrollable Menus -- PARTIALLY IMPLEMENTED (Plan says not-started)

**Plan status:** not-started
**Actual status:** SUBSTANTIALLY IMPLEMENTED -- plan is STALE

### Discrepancy

The plan marks 24.9 as `not-started`, but the code shows substantial implementation:

### What Exists (implemented but not reflected in plan)

- `MenuStyle::max_height: Option<f32>` (mod.rs line 89, default: `None`).
- `MenuWidget::scroll_offset: f32` (mod.rs line 153).
- `scroll_by(delta)` method with clamping (mod.rs line 226).
- `ensure_visible(index)` for keyboard navigation auto-scroll (mod.rs line 249).
- `entry_at_y()` accounts for scroll offset (mod.rs line 274).
- `is_scrollable()` predicate (mod.rs line 221).
- `visible_height()` clamped by `max_height` (mod.rs line 210).
- `max_scroll()` calculation (mod.rs line 216).
- `entry_top_y()` for Y offset computation (mod.rs line 234).
- Mouse wheel scroll handling in `widget_impl.rs` lines 99-110: `ScrollDelta::Pixels` and `ScrollDelta::Lines` both handled.
- Draw clipping with `push_clip()` / `pop_clip()` when scrollable (widget_impl.rs lines 55-63, 68-71).
- Scrollbar drawing (`draw_scrollbar()` in widget_impl.rs lines 307-334) with proportional thumb.
- Keyboard ArrowDown/ArrowUp call `ensure_visible()` after `navigate()` (widget_impl.rs lines 135-152).
- `SCROLLBAR_WIDTH` and `SCROLLBAR_MIN_THUMB` constants (mod.rs lines 128-131).

### Pre-existing file split done differently than planned

- The plan says to extract drawing into `menu/draw.rs`. The actual split put drawing in `menu/widget_impl.rs` (336 lines). Same effect, different name. `mod.rs` is 376 lines (well under 500).

### What Does NOT Exist

- **PageUp/PageDown/Home/End keys** -- not in `handle_key()` (widget_impl.rs lines 129-169). Only ArrowDown, ArrowUp, Enter, Space, Escape handled.
- **Scroll position reset on menu rebuild** -- no explicit reset when entries change. `MenuWidget::new()` starts at `scroll_offset: 0.0`, so new menus start at top, but there's no `set_entries()` or rebuild method that resets scroll.

### ZERO scroll-related tests

Despite substantial scroll implementation, the test file (`menu/tests.rs`, 27 tests) has ZERO tests for:
- `max_height` clamping layout height
- Mouse wheel scroll
- `scroll_offset` clamping
- Keyboard navigation auto-scroll
- `entry_at_y()` with scroll offset
- Scrollbar visibility
- `ensure_visible()` behavior

This is a significant coverage gap -- the plan's test list (8 items) is entirely unaddressed.

---

## 24.10 Section Completion -- NOT STARTED

All exit criteria depend on completing subsections 24.3, 24.4, 24.6, 24.7, 24.8, and 24.9.

---

## Summary

| Subsection | Plan Status | Verified Status | Tests | Notes |
|---|---|---|---|---|
| 24.1 Cursor Blinking | complete | VERIFIED COMPLETE | 13 pass | Solid coverage |
| 24.2 Hide Cursor While Typing | complete | VERIFIED COMPLETE | 7 pass | Solid coverage |
| 24.3 Minimum Contrast | not-started | VERIFIED NOT STARTED | 4 (config only) | Config exists, shader/Rust impl not started |
| 24.4 HiDPI & Display Scaling | in-progress | VERIFIED IN PROGRESS | 0 dedicated | Font zoom not implemented; 2 bugs documented |
| 24.5 Vector Icons | complete | VERIFIED COMPLETE | 18 pass | 2 GPU tests deferred (plan-acknowledged) |
| 24.6 Background Images | not-started | VERIFIED NOT STARTED | 0 | render.rs at 735 lines (pre-existing hygiene issue) |
| 24.7 Background Gradients | not-started | VERIFIED NOT STARTED | 0 | Depends on 24.6 extraction |
| 24.8 Window Backdrop Effects | in-progress | VERIFIED IN PROGRESS | 0 | Basic blur works; Backdrop enum not created |
| 24.9 Scrollable Menus | not-started | **STALE -- SUBSTANTIALLY IMPLEMENTED** | 0 scroll tests | Code exists but plan not updated; missing PageUp/Down/Home/End and all tests |
| 24.10 Section Completion | not-started | VERIFIED NOT STARTED | -- | Depends on all above |

---

## Issues Found

### STALE-001: Section 24.9 plan status is wrong

**Severity:** Plan accuracy
**Details:** The plan marks 24.9 as `not-started` but the core scrollable menu functionality (scroll_offset, max_height, scroll_by, ensure_visible, mouse wheel, draw clipping, scrollbar) is implemented. The plan's prerequisite (file split) was also done (as `widget_impl.rs` instead of `draw.rs`). Only PageUp/PageDown/Home/End keys and tests are missing.
**Action:** Update plan to `in-progress`, check implemented items, leave unchecked items for the remaining work.

### GAP-001: Zero scroll tests for menu widget

**Severity:** High
**Details:** 24.9 lists 8 test items, none of which exist. The scroll implementation has no test coverage despite being the most complex part of MenuWidget. Core behaviors (max_height clamping, scroll_offset bounds, entry_at_y with offset, ensure_visible) are untested.
**Affected file:** `oriterm_ui/src/widgets/menu/tests.rs` (27 tests, 0 scroll-related)

### GAP-002: Zero tests for handle_dpi_change (24.4)

**Severity:** Medium
**Details:** `handle_dpi_change()` re-rasterizes fonts and clears atlases but has no dedicated unit test. The function requires `&mut App` (GPU-dependent), but the font size calculation logic could be extracted and tested headlessly.
**Affected file:** `oriterm/src/app/mod.rs` line 301

### GAP-003: Zero tests for backdrop/transparency (24.8)

**Severity:** Low
**Details:** `transparency.rs` has no tests. The function is a thin platform dispatch layer, so the risk is low, but config parsing tests for the future `Backdrop` enum are needed before it's implemented.

### HYGIENE-001: render.rs exceeds 500-line limit

**Severity:** Medium (pre-existing, blocks 24.6/24.7)
**Details:** `oriterm/src/gpu/window_renderer/render.rs` is 735 lines, exceeding the hard 500-line limit from CLAUDE.md. The plan explicitly warns about this and recommends extraction before 24.6 work. This must be addressed before adding background image or gradient render passes.

### HYGIENE-002: pipeline/mod.rs at exactly 500 lines

**Severity:** Low (at limit, not over)
**Details:** `oriterm/src/gpu/pipeline/mod.rs` is exactly 500 lines. Any new pipeline creation code must go in submodules (plan correctly notes this).
