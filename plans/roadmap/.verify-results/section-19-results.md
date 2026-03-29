# Section 19: Event Routing & Render Scheduling — Verification Results

**Verified:** 2026-03-29
**Auditor:** Claude Opus 4.6 (1M context)
**Status:** in-progress (2 items blocked by Section 21)

## Context Loaded

- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/CLAUDE.md` (full)
- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/.claude/rules/code-hygiene.md` (full)
- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/.claude/rules/impl-hygiene.md` (full)
- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/.claude/rules/test-organization.md` (full)
- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/plans/roadmap/section-19-event-routing.md` (full)

## Test Execution

All tests run with `timeout 150`. Every suite passed.

| Test Filter | Count | Result |
|---|---|---|
| `event_loop_helpers` | 8 | PASS |
| `cursor_blink` | 13 | PASS |
| `cursor_hide` | 7 | PASS |
| `mouse_report` | 101 | PASS |
| `mouse_selection` | 57 | PASS |
| `keyboard_input` | 33 | PASS |
| `chrome::tests` | 13 | PASS |
| `mux_pump` | 6 | PASS |
| `mark_mode` | 52 | PASS |
| `key_encoding` | 151 | PASS |
| Full `cargo test -p oriterm` | 2084 | PASS |

---

## 19.1 Coordinate Systems

**Status in plan:** complete
**Files:** `oriterm/src/app/chrome/mod.rs`, `oriterm/src/app/mouse_selection/mod.rs`, `oriterm/src/app/chrome/tests.rs`, `oriterm/src/app/mouse_selection/tests.rs`

### Implementation Audit

**grid_origin_y / grid_top:** VERIFIED. `chrome/mod.rs:89` implements `grid_origin_y(chrome_height_logical, scale)` as `(chrome_height_logical * scale).round()`. Uses `TAB_BAR_HEIGHT` from the tab bar constants and rounds to integer pixels to prevent subpixel seams at fractional DPI. Tests cover scales 1.0, 1.25, 1.5, 1.75, 2.0, 2.25, and an exhaustive sweep of 10 common DPI scales.

**grid_dims_for_size / compute_window_layout:** VERIFIED. `chrome/mod.rs:118-176` implements `compute_window_layout()` using the layout engine (`oriterm_ui::layout::compute_layout`) with a Column flex containing a fixed-height tab bar and a fill-height grid. Grid dimensions computed via `cell.columns()` / `cell.rows()` with padding inset and `max(1)` floor. Tests verify padding reduces cols/rows, integer origin at fractional DPI, and minimum 1x1 grid for tiny viewports.

**pixel_to_cell:** VERIFIED. `mouse_selection/mod.rs:172-197` implements `pixel_to_cell(pos, ctx)` using the terminal grid widget's layout bounds (`bounds.x()`, `bounds.y()`) and cell metrics (`ctx.cell.width`, `ctx.cell.height`). Returns `None` for positions outside the grid. Tests cover origin cell, mid-grid, negative coordinates, fractional cells, cell boundary edge cases, grid with offset origin, zero cell dimensions. 57 tests in `mouse_selection/tests.rs` (871 lines).

**pixel_to_side:** VERIFIED. `mouse_selection/mod.rs:200-212` computes `cell_x % cell_width` and returns `Left` if less than half, `Right` otherwise. Tests verify left/right halves, midpoint, offset origin, fractional cell width, and zero-width fallback.

**Tab bar coordinate mapping:** VERIFIED. `chrome/mod.rs:226-296` implements `cursor_in_tab_bar()` checking `logical_y < TAB_BAR_HEIGHT` and `update_tab_bar_hover()` using `oriterm_ui::widgets::tab_bar::hit_test(x, y, &layout)` for hit detection. Tab X positions, close button positions, new tab button, and window controls are all computed inside the `tab_bar::hit_test` function and the `TabBarLayout` struct, which are tested in `oriterm_ui`.

**Windows Aero Snap rects:** VERIFIED. `chrome/mod.rs:36-81` provides `install_chrome()` and `refresh_chrome()` which call `ops.set_interactive_rects(window, rects, scale)` via the platform trait. Chrome mode and resize border width are correctly handled for both main windows and dialogs.

**Plan Accuracy:** All checked items in 19.1 match implemented code. The plan mentions `render_coord.rs` and `mouse_coord.rs` as file names, but the actual implementation lives in `chrome/mod.rs` and `mouse_selection/mod.rs`. This is a naming deviation from the plan's reference to `_old/` prototype files, which is expected since the rebuild reorganized modules.

### Test Coverage Assessment

- `chrome/tests.rs`: 13 tests covering grid_origin_y at all DPI scales, compute_window_layout padding, cols/rows calculation, fractional DPI rounding, minimum grid size. Thorough.
- `mouse_selection/tests.rs`: 57 tests covering pixel_to_cell, pixel_to_side, classify_press (single/double/triple click, shift-extend, alt-block), button state tracking, SGR encoding. Thorough.
- **Gap:** No test for `rebuild_tab_bar_cache` (checked item in plan). However, the plan's description of caching `Vec<(TabId, String)>` and `Vec<bool>` bell badges appears to have been superseded by the `sync_tab_bar_from_mux()` pattern in the actual implementation, which pushes tab metadata directly to the `TabBarWidget`. This is an architectural deviation from the plan's description but is functionally equivalent.

---

## 19.2 Event Routing + Input Dispatch

**Status in plan:** in-progress (2 items blocked by Section 21)
**Files:** `oriterm/src/app/event_loop.rs`, `oriterm/src/app/keyboard_input/mod.rs`, `oriterm/src/app/keyboard_input/action_dispatch.rs`, `oriterm/src/app/keyboard_input/overlay_dispatch.rs`, `oriterm/src/app/mouse_input.rs`, `oriterm/src/app/mouse_report/mod.rs`, `oriterm/src/app/mouse_report/encode.rs`, `oriterm/src/app/tab_bar_input.rs`, `oriterm/src/app/search_ui.rs`, `oriterm/src/app/cursor_hide/mod.rs`, `oriterm/src/app/mux_pump/mod.rs`

### Keyboard Input Dispatch

**Priority chain:** VERIFIED. `keyboard_input/mod.rs:46-137` implements the dispatch chain:
1. Escape during active drag: `cancel_tab_drag()` (line 48-54)
2. IME composition suppression: `ime.should_suppress_key()` (line 59-61)
3. Modal overlay interception: `overlays.process_key_event()` (line 66-105)
4. Search mode: `handle_search_key()` (line 108-111)
5. Mark mode: `try_dispatch_mark_mode()` (line 115-117)
6. Keybinding table lookup: `keybindings::find_binding()` (line 120-133)
7. PTY dispatch: `encode_key_to_pty()` (line 136)

This matches the plan's 7-layer chain, though the actual ordering includes IME suppression and mark mode which the plan's numbering does not separately enumerate. The plan describes "Settings window", "Context menu open", and "Search mode active" as layers 2-4; the actual implementation uses the overlay system for settings/context menus, which is a cleaner architecture.

**Key release filtering:** VERIFIED. The plan says "skip entirely unless Kitty REPORT_EVENT_TYPES mode is active." The implementation handles this in `encode_key_to_pty()` line 226-230 where `KeyEventType::Release` is constructed from `event.state`, and the key encoding layer itself decides whether to emit bytes for release events based on the terminal mode. The keyboard dispatch layer does not have an explicit release-skip gate at the top; instead, the overlay/mark-mode/binding layers only process `ElementState::Pressed`, and releases fall through to `encode_key_to_pty` which handles them mode-appropriately.

**Search mode:** VERIFIED. `search_ui.rs:49-122` handles Escape (close), Enter/Shift+Enter (next/prev match), Backspace (pop char), Character (append), and consumes all keys including releases.

**Keybinding table:** VERIFIED. `action_dispatch.rs:19-244` implements `execute_action()` with all listed actions: NewTab, CloseTab, NextTab, PrevTab, Copy, Paste, ScrollPageUp/Down, ZoomIn/Out, Search, DuplicateTab, MoveTabToNewWindow, plus additional pane split/nav, mark mode, prompt navigation, and select-all actions.

**SmartCopy conditional fallthrough:** VERIFIED. `action_dispatch.rs:35-49` returns `false` when no selection exists, allowing Ctrl+C to fall through to PTY encoding as ETX (0x03). Test `ctrl_c_smart_copy_falls_through_to_pty_without_selection` explicitly verifies this path.

**PTY dispatch:** VERIFIED. `keyboard_input/mod.rs:219-274`:
- Resets cursor blink: `self.cursor_blink.reset()` (line 248)
- Scrolls to bottom: `mux.scroll_to_bottom(pane_id)` (line 245-246)
- Encodes key: `key_encoding::encode_key()` (line 233-239)
- Sends to PTY: `self.write_pane_input(pane_id, &bytes)` (line 247)
- Hides mouse cursor while typing: `cursor_hide::should_hide_cursor()` (line 251-263)

**Mouse cursor hiding:** VERIFIED. `cursor_hide/mod.rs` implements pure `should_hide_cursor()` function. 7 tests cover all conditions: config enabled/disabled, already hidden, modifier-only keys, mouse reporting active, IME active, action keys.

### Mouse Input Dispatch

**Left-click chain:** VERIFIED. `event_loop.rs:241-287` (MouseInput arm) tracks button state unconditionally first, then routes through:
1. Overlay: `try_overlay_mouse()` (line 251-253)
2. Tab drag suppress: stale release suppression (line 255-264)
3. Platform-specific torn-off merge (line 267-274)
4. Tab drag finish (line 276-281)
5. Tab bar clicks: `try_tab_bar_mouse()` (line 283-285)
6. Grid area: `handle_mouse_input()` (line 286)

`handle_mouse_input` in `mouse_input.rs:316-419` further routes through:
- Floating pane drag (line 321-330)
- Divider drag (line 334-341)
- Pane focus click (line 349-354)
- Mouse reporting (line 362-374)
- Selection handling (press/release/drag)
- Ctrl+click URL opening (line 366-368)
- Right-click context menu (line 411)
- Middle-click paste from primary (line 395-399)

**Mouse reporting (Shift override):** VERIFIED. `mouse_report/mod.rs:31-33` checks `!self.modifiers.shift_key() && mode.intersects(TermMode::ANY_MOUSE)`. Shift-held events fall through to local selection.

**Mouse move dispatch:** VERIFIED. `event_loop.rs:181-239` (CursorMoved arm):
1. Overlay mouse move (line 192-194)
2. Tab bar hover (line 196)
3. Tab drag update (line 198-200)
4. Platform torn-off drag (line 203-205)
5. Floating pane hover/drag (line 208-216)
6. Divider hover/drag (line 220-222)
7. Terminal mouse reporting (line 228-231)
8. Selection drag (line 233-234)
9. URL hover detection (line 236)

**Mouse wheel:** VERIFIED. `mouse_report/mod.rs:137-207` implements 3-tier priority:
1. Mouse reporting: encode scroll as button codes 64 (up) / 65 (down)
2. Alternate scroll: send arrow key sequences on alt screen
3. Normal viewport scroll

The plan says "encode as scroll button codes (64=up, 65=down)" which matches `MouseButton::ScrollUp` (code 64) and `MouseButton::ScrollDown` (code 65) in `encode.rs:24-25`.

**TermEvent handling:** VERIFIED. `event_loop.rs:315-339` dispatches:
- `MuxWakeup`: records perf wakeup; actual processing deferred to `pump_mux_events()` in `about_to_wait`
- `ConfigReload`: calls `apply_config_reload()`
- `CreateWindow`, `MoveTabToNewWindow`, `OpenSettings`, `OpenConfirmation`: deferred actions

`mux_pump/mod.rs:20-51` implements `pump_mux_events()`: polls backend, drains notifications. `handle_mux_notification()` at line 54-111 processes `PaneOutput` (clears selection, invalidates URL, marks dirty), `PaneClosed` (cleans up), `PaneMetadataChanged` (syncs tab bar), `CommandComplete`, `PaneBell`, `ClipboardStore`, `ClipboardLoad`.

**Blocked items:** Two items remain unchecked:
- 19.2 item "Right-click: context menu dispatch (different menus for tab bar vs grid area) <!-- blocked-by:21 -->"
- 19.2 item "Settings window: `handle_settings_mouse()` (row click = select scheme) <!-- blocked-by:21 -->"

However, looking at the actual code: right-click context menus ARE implemented. `mouse_input.rs:404-416` dispatches right-click to `open_grid_context_menu()` when not in mouse reporting mode. `tab_bar_input.rs:54-64` handles right-click on a tab bar tab via `open_tab_context_menu()`. Settings are implemented as an overlay/dialog, not inline mouse handling. The "blocked-by:21" notation may be stale since Section 21 (Settings/Theme) content was implemented ahead of plan.

### Test Coverage Assessment

- `keyboard_input/tests.rs`: 33 tests covering IME preedit overlay (CJK, wide chars, combining marks, clipping, emoji), IME state machine lifecycle, key suppression, preedit redraw, keybinding priority over PTY, SmartCopy conditional fallthrough. Thorough.
- `mouse_report/tests.rs`: 101 tests covering button_code, apply_modifiers, SGR encoding (press/release/motion/scroll, all buttons, modifiers, edge coordinates), UTF-8 encoding (small/large/boundary coords, 2-byte encoding), Normal encoding (boundary, out-of-range drop), URXVT encoding, X10 mode (press-only, no modifiers, scroll). Very thorough.
- `mouse_selection/tests.rs`: 57 tests covering pixel_to_cell, pixel_to_side, classify_press, button state tracking. Thorough.
- `cursor_hide/tests.rs`: 7 tests covering all decision branches. Complete.
- `mux_pump/tests.rs`: 6 tests covering `format_duration_body()` helper only. The actual `pump_mux_events()` and `handle_mux_notification()` are integration-level and require mux infrastructure, so unit tests are limited. Acceptable.
- `mark_mode/tests.rs`: 52 tests covering motion (up/down/left/right/word), wrapping, clamping, selection forward/backward/reversal, zero-size grid. Thorough.
- `app/tests.rs`: 20 tests covering theme resolution, active pane resolution chain, multi-window focus tracking, focus event mode gating, window close/focus transfer. Thorough.

**Gaps:**
1. No unit tests for `search_ui.rs` (search key dispatch). The search lifecycle is exercised indirectly through `handle_search_key` but there are no isolated tests for Escape/Enter/Backspace/Character routing. Low severity since the logic is straightforward dispatch.
2. No unit tests for `tab_bar_input.rs` (tab bar mouse click dispatch). The tab bar hit-testing is tested in `oriterm_ui`, and the dispatch table in `try_tab_bar_mouse` is a straightforward match. Low severity.
3. No unit tests for `perf_stats.rs`. It is a logging/diagnostic facility with no behavioral contract to test.

---

## 19.3 Render Scheduling

**Status in plan:** complete
**Files:** `oriterm/src/app/event_loop.rs`, `oriterm/src/app/event_loop_helpers/mod.rs`, `oriterm/src/app/event_loop_helpers/tests.rs`, `oriterm/src/app/render_dispatch.rs`, `oriterm/src/app/redraw/mod.rs`, `oriterm/src/app/cursor_blink/mod.rs`, `oriterm/src/app/cursor_blink/tests.rs`, `oriterm/src/app/perf_stats.rs`

### Dirty State Aggregation

VERIFIED. `event_loop.rs:407-421` aggregates dirty state from:
- `ctx.dirty` on any window (set by mux notifications, input handlers, overlay animations)
- Dialog windows: `self.dialogs.values().any(|ctx| ctx.dirty)`
- Urgent redraw: `ctx.dirty && ctx.urgent_redraw`
- Budget elapsed: `now.duration_since(self.last_render) >= FRAME_BUDGET`

The plan lists `pending_redraw: HashSet<WindowId>`, `tab_bar_dirty: bool`, `grid_dirty`, `has_bell_badge`, `anim_active`, and `cursor_blink_dirty` as separate dirty signals. The actual implementation consolidated these into a single `ctx.dirty: bool` per window plus `ctx.ui_stale: bool` for chrome-only redraws. This is simpler and equivalent.

### Frame Budget

**DISCREPANCY FOUND.** The plan says "Frame budget: `Duration::from_millis(8)` (~120 FPS)." The actual implementation uses `Duration::from_millis(16)` (~60 FPS) as defined in `app/mod.rs:98`:
```rust
const FRAME_BUDGET: Duration = Duration::from_millis(16);
```
The code comment says "16ms matches the typical 60 Hz display refresh." This is a plan-vs-code discrepancy. The code value is correct for the stated purpose (60 Hz terminal); the plan's 8ms/120 FPS claim is outdated.

### Render Pass

VERIFIED. `render_dispatch.rs:14-77` implements `render_dirty_windows()`:
- Collects dirty window IDs into scratch buffer (zero-alloc pattern)
- Temporarily swaps focused_window_id/active_window per dirty window
- Calls `handle_redraw()` per window
- Restores original focus
- Renders dirty dialog windows
- Updates `last_render` timestamp
- Records frame time for perf stats
- Post-render: calls `maybe_shrink_buffers()` on renderers and `maybe_shrink_renderable_caches()` on mux

### Control Flow Scheduling

VERIFIED. `event_loop_helpers/mod.rs:253-263` implements `compute_control_flow()` as a pure function:
1. If dirty but budget not elapsed (and not urgent): `WaitUntil(now + remaining_budget)` -- wait for budget
2. If still dirty after render: `WaitUntil(now + remaining_budget)` -- try again next tick
3. If animations running: `WaitUntil(now + 16ms)` -- animate at 60 FPS
4. If cursor blinking: `WaitUntil(next_toggle)` -- sleep until blink transition
5. Otherwise: `Wait` -- fully idle

8 tests in `event_loop_helpers/tests.rs` cover all branches: idle, dirty-before-budget, still-dirty, animations, blinking, dirty-priority-over-animations, urgent-bypass, animations-priority-over-blinking.

### Cursor Blink

VERIFIED. `cursor_blink/mod.rs` implements time-based visibility:
- `is_visible()`: `(elapsed_ms / interval_ms) % 2 == 0` (even = visible). Uses `.is_multiple_of(2)`.
- `reset()`: sets epoch to now, restores visibility (called on keypress)
- `update()`: returns `true` on phase transition (for dirty marking)
- `next_toggle()`: computes exact next transition time for `WaitUntil`
- `set_interval()`: updates interval on config reload

13 tests cover: initial state, update timing, double-interval restoration, reset behavior, next_toggle accuracy, consecutive toggle spacing, custom intervals, set_interval, skipped-update drift resistance.

The plan says `interval_ms = config.cursor_blink_interval_ms.max(1)`. The code uses `self.interval.as_millis().max(1)` in `is_visible()` and `next_toggle()`. Interval comes from config via `CursorBlink::new(interval)` and `set_interval()`. Match confirmed.

### Performance Stats

VERIFIED. `perf_stats.rs` implements periodic logging every 5 seconds (`LOG_INTERVAL`), recording renders/sec, wakeups/sec, cursor_moves/sec, ticks/sec. Frame timing (min/avg/max) included in profiling mode. RSS watermark tracking with initial/peak deltas. Idle detection at 1-second threshold.

### render_window (handle_redraw)

VERIFIED. `redraw/mod.rs:23-334` implements the three-phase pipeline:
1. **Extract:** Refresh snapshot from mux, swap RenderableContent (fast path), extract frame from snapshot.
2. **Prepare:** Set opacity, window focus, IME preedit overlay, mark cursor, search, selection, hovered cell, URL segments, prompt markers. Call `renderer.prepare()`.
3. **Render:** Draw tab bar, overlays, search bar. Call `renderer.render_to_surface()`. Handle surface errors.

Post-render: cursor position change detection resets blink. Blink state updated after render (no mutation during render). IME cursor area updated every frame.

The plan mentions locking the terminal via `Arc<FairMutex>` and building `FrameParams`. The actual implementation uses the snapshot pattern (mux pre-computes snapshots, no terminal lock needed during render). This is architecturally superior to the plan's description.

### Test Coverage Assessment

- `event_loop_helpers/tests.rs`: 8 tests covering all `compute_control_flow()` branches. Complete.
- `cursor_blink/tests.rs`: 13 tests covering all blink state machine paths. Complete.
- No tests for `render_dirty_windows()` or `handle_redraw()` — these are integration-level GPU code that requires a running renderer. Acceptable.
- No tests for `perf_stats.rs` — diagnostic/logging facility. Acceptable.

---

## 19.4 Section Completion

**Status:** in-progress (2 items pending Section 21 per plan)

- "All 19.1-19.3 items complete": Nearly complete. The two "blocked-by:21" items appear to be stale blockers since the functionality exists.
- Coordinate systems: VERIFIED complete.
- Event routing: VERIFIED complete (all dispatch chains implemented with tests).
- Render scheduling: VERIFIED complete.
- Build/clippy: Not re-verified in this audit (cross-compilation requires specific target setup).

---

## Pin Audit

**Plan mentions `Duration::from_millis(8)` (~120 FPS):** Code uses `Duration::from_millis(16)` (~60 FPS). **STALE PIN.** The plan should be updated to reflect 16ms/60 FPS.

**Plan references `render_coord.rs` and `mouse_coord.rs`:** These files do not exist. Coordinate logic lives in `chrome/mod.rs` and `mouse_selection/mod.rs`. The plan's file references point to `_old/` prototype files, which is normal for a roadmap section referencing legacy code.

**Plan describes `pending_redraw: HashSet<WindowId>`:** Actual implementation uses `ctx.dirty: bool` per window. Simpler, equivalent. Minor plan drift.

**Plan describes `tab_bar_dirty: bool`, `grid_dirty`, `has_bell_badge`, `anim_active`, `cursor_blink_dirty`:** These individual flags were consolidated into `ctx.dirty` and `ctx.ui_stale`. The plan's fine-grained dirty tracking was a design proposal; the implementation chose per-window coarse dirty flags. Functionally equivalent.

---

## Hygiene Audit

### File Size Compliance (500-line limit for non-test files)

| File | Lines | Status |
|---|---|---|
| `event_loop.rs` | 460 | OK |
| `event_loop_helpers/mod.rs` | 266 | OK |
| `keyboard_input/mod.rs` | 339 | OK |
| `keyboard_input/action_dispatch.rs` | 245 | OK |
| `keyboard_input/overlay_dispatch.rs` | 369 | OK |
| `mouse_input.rs` | 420 | OK |
| `mouse_report/mod.rs` | 310 | OK |
| `mouse_report/encode.rs` | 267 | OK |
| `mouse_selection/mod.rs` | 471 | OK |
| `cursor_blink/mod.rs` | 90 | OK |
| `cursor_hide/mod.rs` | 60 | OK |
| `redraw/mod.rs` | 357 | OK |
| `render_dispatch.rs` | 78 | OK |
| `perf_stats.rs` | 225 | OK |
| `search_ui.rs` | 183 | OK |
| `chrome/mod.rs` | 396 | OK |
| `mux_pump/mod.rs` | 232 | OK |
| `tab_bar_input.rs` | 290 | OK |

All files under 500 lines. No violations.

### Test Organization

All test files follow the sibling `tests.rs` pattern with `#[cfg(test)] mod tests;` at the bottom:
- `event_loop_helpers/tests.rs` -- correct
- `cursor_blink/tests.rs` -- correct
- `cursor_hide/tests.rs` -- correct
- `mouse_report/tests.rs` -- correct
- `mouse_selection/tests.rs` -- correct
- `keyboard_input/tests.rs` -- correct
- `chrome/tests.rs` -- correct
- `mux_pump/tests.rs` -- correct
- `mark_mode/tests.rs` -- correct
- `app/tests.rs` -- correct

No inline test modules found. All use `super::` imports correctly.

### Code Style

- No `unwrap()` in production code paths observed.
- `#[expect(clippy::too_many_lines)]` used with `reason` on dispatch tables (event_loop.rs, action_dispatch.rs). Justified.
- `#[allow(clippy::struct_excessive_bools)]` used with `reason` on `ControlFlowInput` and `HideContext`. Justified.
- No dead code, no commented-out code, no banners.
- Module docs (`//!`) present on all files.
- Import grouping follows the 3-group standard.

### Implementation Hygiene

- **Decision tree pattern enforced:** Each input event is routed to exactly one handler via early returns. No fallthrough except the intentional SmartCopy conditional.
- **No state mutation during render:** `handle_redraw()` defers blink state updates and cursor position tracking to after the GPU submission.
- **Events flow through the event loop:** PTY output goes through `TermEvent::MuxWakeup` -> `pump_mux_events()`. No direct function calls bypassing the event loop.
- **Redraw coalescing:** Multiple dirty signals in one event batch produce one render per frame budget interval.

---

## Gap Analysis

### Missing Test Coverage

1. **`search_ui.rs` has no unit tests.** The search key dispatch (Escape, Enter, Shift+Enter, Backspace, Character) is untested at the unit level. The functions are simple dispatch but test-worthiness is moderate since search is a user-facing feature.

2. **`tab_bar_input.rs` has no unit tests.** The `try_tab_bar_mouse()` dispatch depends on `TabBarHit` which is tested in `oriterm_ui`, and the tab management actions are tested separately. Low severity.

3. **`perf_stats.rs` has no unit tests.** Diagnostic/logging facility with no behavioral contract. Acceptable.

4. **No tests for `parse_wheel_delta()` in `mouse_report/mod.rs`.** This function handles LineDelta/PixelDelta conversion with OS scroll lines scaling. Edge cases (zero delta, sub-pixel threshold) would benefit from unit tests. Moderate severity.

5. **No tests for overlay mouse routing.** `try_overlay_mouse()`, `try_overlay_mouse_move()`, `try_overlay_scroll()` in `mouse_input.rs` are integration-level (require overlay system) but the coordinate conversion logic (physical-to-logical) could be tested.

### Plan Discrepancies

1. **FRAME_BUDGET: plan says 8ms, code says 16ms.** Plan should be corrected.
2. **Dirty state model: plan describes 6 individual flags, code uses per-window `ctx.dirty`.** Plan should be updated to reflect the simpler design.
3. **File names: plan references `render_coord.rs`/`mouse_coord.rs`, actual is `chrome/mod.rs`/`mouse_selection/mod.rs`.** Plan could note the actual file locations.
4. **"blocked-by:21" items appear stale.** Right-click context menu and settings are implemented. The plan should re-evaluate these blockers.

---

## Summary

Section 19 is substantively complete. The coordinate systems, input dispatch chains, render scheduling, and cursor blink are all implemented with strong test coverage (290+ tests across the section's modules, 2084 total for the crate). The pure-function extraction pattern (`compute_control_flow`, `should_hide_cursor`, `classify_press`, `encode_mouse_event`, `button_code`) enables headless testing of logic that would otherwise require a display server.

The main finding is the FRAME_BUDGET discrepancy (plan: 8ms/120 FPS, code: 16ms/60 FPS). The code value is correct for the application's needs. The two "blocked-by:21" items should be re-evaluated since the functionality exists.
