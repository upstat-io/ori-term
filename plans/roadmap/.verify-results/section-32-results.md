# Section 32 Verification Results: Tab & Window Management (Mux-Aware)

**Verified:** 2026-03-29
**Status:** PASS
**Section status in plan:** complete, reviewed: true

## Context Loaded

- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/CLAUDE.md` (full)
- `.claude/rules/code-hygiene.md` (full)
- `.claude/rules/impl-hygiene.md` (full)
- `.claude/rules/test-organization.md` (full)
- `.claude/rules/crate-boundaries.md` (loaded via system-reminder)
- `plans/roadmap/section-32-tab-window-mux.md` (full)

---

## 32.1 Mux-Aware Tab Management

### Implementation Files

| File | Lines | Under 500 |
|------|-------|-----------|
| `oriterm/src/app/tab_management/mod.rs` | 449 | Yes |
| `oriterm/src/app/tab_management/move_ops.rs` | 240 | Yes |
| `oriterm/src/app/tab_management/tests.rs` | 138 | N/A (test) |

### Checklist Items Verified

- **New tab** (`new_tab_in_window`): Lines 25-82 of `mod.rs`. Inherits CWD via `self.active_pane_id().and_then(|id| self.mux.as_ref()?.pane_cwd(id))` (line 28-29). Builds `SpawnConfig` with shell, scrollback, cursor shape from config (lines 37-43). Calls `mux.spawn_pane(&config, theme)` (line 50). Creates local `Tab` via `session.alloc_tab_id()` + `session.add_tab()` (lines 63-68). Clears `tab_width_lock` (line 71). Marks dirty (line 81).

- **Close tab** (`close_tab`): Lines 91-152 of `mod.rs`. Collects pane IDs from local session (lines 99-103), closes each pane through mux (lines 107-110). Removes tab from session (line 114). If last tab, calls `self.exit_app()` (line 123) -- ConPTY-safe since `exit_app` calls `process::exit(0)` before dropping panes. If owning window now empty but not last, calls `close_empty_session_window` (line 136). Marks `tab_bar_dirty` (line 139).

- **Duplicate tab** (`duplicate_active_tab`): Lines 183-188 of `mod.rs`. Delegates to `new_tab_in_window(window_id)` which inherits CWD from active pane.

- **Cycle tabs** (`cycle_tab`): Lines 191-229 of `mod.rs`. Uses `rem_euclid` for wrapping arithmetic (line 204). Clears bell badge on newly active tab via `mux.clear_bell(id)` (line 218). Marks dirty.

- **Switch to specific tab** (`switch_to_tab`): Lines 232-259 of `mod.rs`. Finds window, sets active index, clears bell.

- **Reorder tabs** (`move_tab`): Lines 212-239 of `move_ops.rs`. Delegates to `session.get_window_mut(win_id).reorder_tab(from, to)`. Adjusts `active_tab` index via `Window::reorder_tab`.

- **Auto-close on PTY exit**: Handled in `mux_pump/mod.rs` lines 195-218: `handle_pane_closed` removes pane from session, and if tab becomes empty it removes the tab, if window becomes empty it calls `close_empty_session_window`.

### Test Coverage

**15 tests in `tab_management/tests.rs`** -- all pass:
- `wrap_forward_within_range`, `wrap_forward_wraps_around`, `wrap_backward_within_range`, `wrap_backward_wraps_around`, `wrap_forward_by_two`, `wrap_backward_by_two`, `wrap_single_tab`, `wrap_large_delta` -- pure `wrap_index` arithmetic tests.
- `create_three_tabs_unique_ids` -- verifies 3 tabs added to Window have unique IDs and window contains all 3.
- `close_middle_tab_preserves_order` -- removes tab at index 1, verifies remaining order is `[t0, t2]`, active_tab_idx stays 0.
- `close_active_tab_adjusts_index` -- removes active tab at end, active clamps to new last.
- `cycle_wrap_forward` -- tab 2 of 3 -> next -> tab 0.
- `cycle_wrap_backward` -- tab 0 of 3 -> prev -> tab 2.
- `reorder_tabs` -- moves tab from index 0 to 2, verifies new order.
- `closing_last_tab_leaves_empty` -- removes sole tab, verifies window is empty.

**Assessment:** Tests cover the pure-computation layer (wrap_index, Window CRUD). The full `App::new_tab_in_window`, `App::close_tab`, etc. require GPU+mux so they are tested via integration-level session model tests in `app/tests.rs` (see 32.3 below). This is the correct approach given CLAUDE.md's crate boundary rules -- App methods need winit/GPU and cannot run in headless unit tests.

---

## 32.2 Multi-Window + Shared GPU

### Implementation Files

| File | Lines | Under 500 |
|------|-------|-----------|
| `oriterm/src/window/mod.rs` | 318 | Yes |
| `oriterm/src/app/window_context.rs` | 134 | Yes |
| `oriterm/src/app/window_management.rs` | 435 | Yes |

### Checklist Items Verified

- **TermWindow struct**: Lines 43-63 of `window/mod.rs`. Contains: `window: Arc<Window>` (winit handle), `surface: wgpu::Surface<'static>`, `surface_config`, `size_px`, `scale_factor: ScaleFactor`, `is_maximized: bool`, `session_window_id: SessionWindowId` (link to session model).

- **Window ID mapping**: `App::windows: HashMap<WindowId, WindowContext>` (line 136 of `app/mod.rs`). `focused_window_id: Option<WindowId>` (line 142). Each `WindowContext` contains a `TermWindow` which stores `session_window_id`.

- **TermWindow::resize_surface**: Lines 205-217 of `window/mod.rs`. Reconfigures surface dimensions. Defers actual `DXGI ResizeBuffers` call via `surface_stale` flag to minimize DWM stale content stretch.

- **Shared GPU resources**: `create_window` (lines 27-104 of `window_management.rs`) reuses `self.gpu` (GpuState). Each window creates its own renderer via `create_window_renderer` (lines 178-225), but they share the GpuState's device and queue. FontSet is cloned per window via `self.font_set.as_ref()?.clone()` (line 120).

- **Focus tracking**: `event_loop.rs` lines 119-159. `Focused(true)` sets `self.focused_window_id`, updates `self.active_window`, sends focus-in escape if FOCUS_IN_OUT mode active. `Focused(false)` defers focus-out to `about_to_wait` to suppress when focus moves to child dialog.

### Test Coverage

**Multi-window session tests** in `app/tests.rs`:
- `multi_window_focus_switch_resolves_different_panes` (lines 207-234): Two windows with distinct panes. Verifies focusing window 1 resolves pane A, window 2 resolves pane B, switching back resolves A again.
- `multi_window_stale_window_returns_none` (lines 236-251): Focus on non-existent window returns None.
- Focus event mode gating: `focus_in_out_mode_bit_pattern`, `focus_in_out_not_set_by_default`, `focus_in_out_combined_with_other_modes` verify TermMode::FOCUS_IN_OUT bit pattern correctness.

**Assessment:** Architecture tests verify the session model supports multi-window. Actual GPU sharing is verified by architecture (single `self.gpu` used by all `create_window` calls). Focus event gating is thoroughly tested via mode bitmask tests.

---

## 32.3 Window Lifecycle

### Implementation Files

| File | Lines | Under 500 |
|------|-------|-----------|
| `oriterm/src/app/window_management.rs` | 435 | Yes |
| `oriterm/src/app/chrome/resize.rs` | 210 | Yes |

### Checklist Items Verified

- **create_window**: Lines 27-104 of `window_management.rs`. Calls `create_window_bare` (which creates invisible frameless window, attaches GPU surface, builds renderer, chrome, tab bar, grid widget). Computes grid dims from renderer cell metrics. Spawns initial pane via `mux.spawn_pane`. Clears surface before showing (`gpu.clear_surface` at line 85 -- prevents gray/white flash). Shows window (line 89). Registers with window manager (line 93).

- **create_window_bare**: Lines 114-175. Calculates `WindowConfig` with transparency/blur from config. Creates `TermWindow::new(event_loop, &window_config, gpu, session_wid)`. Creates per-window renderer with font collection. Window starts hidden.

- **handle_resize**: Lines 103-189 of `chrome/resize.rs`. Releases `tab_width_lock`. On Windows: detects DPI changes from `WM_DPICHANGED` (lines 122-145). Resizes GPU surface. Updates tab bar width. Recomputes grid dimensions via `sync_grid_layout`. Marks dirty.

- **close_window**: Lines 232-289 of `window_management.rs`. If last window, calls `exit_app()` (line 243) -- ConPTY-safe. Collects pane IDs, closes each via mux (lines 248-253). Removes session state. Cleans up pane resources on background threads via `mux.cleanup_closed_pane(id)` (lines 260-263). Transfers focus to remaining window.

- **exit_app**: Lines 369-375. Saves GPU pipeline cache async, then `std::process::exit(0)`. Return type is `-> !` (diverging). ConPTY-safe: `process::exit()` runs before any pane destructors.

- **Fullscreen toggle**: In `action_dispatch.rs` lines 78-84. Queries `ctx.window.is_fullscreen()`, toggles via `set_fullscreen(!is_fs)`. `TermWindow::set_fullscreen` (window/mod.rs lines 256-263) uses `Fullscreen::Borderless(None)` vs `None`. Wired to `Action::ToggleFullscreen` keybinding (Alt+Enter on Windows/Linux, Cmd+Ctrl+F on macOS).

- **DPI change**: `handle_dpi_change` in `app/mod.rs` lines 301-334. Re-rasterizes fonts at `physical_dpi = DEFAULT_DPI * scale`. Updates hinting and subpixel mode. Clears glyph cache. Marks all grid lines dirty.

### Test Coverage

**Window lifecycle tests** in `app/tests.rs`:
- `close_window_focus_transfers_to_remaining` (lines 284-295): Removes window 1 from two-window session. Verifies window count drops to 1, window 2 still resolves, window 1 returns None.
- `close_window_cleans_up_tabs` (lines 297-316): Closes window 1, verifies its tabs are removed but window 2's tabs survive.
- `close_all_windows_leaves_empty_session` (lines 318-334): Both windows closed, session empty.
- `multi_window_close_preserves_other_window_tabs` (lines 337-368): Three windows, close middle, verify windows 1 and 3 unaffected.

**Event loop helpers tests** (8 tests, all pass):
- `idle_returns_wait`, `blinking_returns_next_toggle`, `dirty_before_budget_returns_wait_until_remaining`, `still_dirty_after_render_returns_wait_until`, `animations_return_16ms_wait`, `animations_take_priority_over_blinking`, `dirty_takes_priority_over_animations`, `urgent_dirty_bypasses_budget_wait`.

**Assessment:** No-flash window creation (clear surface before show) and ConPTY-safe exit ordering (`exit_app` calls `process::exit(0)` via `-> !` before any destructors) are structurally correct. DPI change and resize tests are structural (the full chain requires a display server). Session-level lifecycle is well tested.

---

## 32.4 Cross-Window Operations

### Implementation Files

| File | Lines | Under 500 |
|------|-------|-----------|
| `oriterm/src/app/tab_management/move_ops.rs` | 240 | Yes |
| `oriterm/src/app/tab_drag/tear_off.rs` | 476 | Yes |
| `oriterm/src/app/tab_drag/merge_core.rs` | 117 | Yes |

### Checklist Items Verified

- **move_tab_to_window**: Lines 15-47 of `move_ops.rs`. Removes tab from source window in session (`win.remove_tab`), adds to destination (`win.add_tab`). Releases tab width lock. Syncs tab bar. Resizes panes in moved tab to fit destination. Marks dirty.

- **move_tab_to_new_window**: Lines 67-86 of `move_ops.rs`. Refuses if last tab in session (line 74). Supports daemon mode (spawns new process via `move_tab_to_new_window_daemon`) and embedded mode (creates in-process window via `move_tab_to_new_window_embedded`).

- **Tab tear-off** (`tear_off_tab`): Lines 30-125 of `tear_off.rs`. Extracts drag state, refuses last tab, creates bare window, moves tab via session, positions under cursor, pre-renders both windows, shows torn-off window, starts OS drag. Platform-specific implementations for Windows (`drag_window` with `WM_MOVING`/merge rects), macOS (`TornOffPending` tracking), and Linux (X11 manual tracking, Wayland `drag_window()`). All three platforms have their own `#[cfg]` blocks.

- **Tab merge** (`execute_tab_merge`): Lines 36-94 of `merge_core.rs`. Moves tab from source to target window in session, drains mux events, removes empty source window, focuses target, syncs tab bars, resizes panes.

- **Multi-pane tab preservation**: `move_tab_to_window` operates on `TabId` only, which references a `Tab` containing a `SplitTree`. The tree is not modified during the move -- the tab retains all its panes and layout structure. Panes are resized to fit the target window via `resize_all_panes()`.

### Test Coverage

**36 tab_drag tests** -- all pass. Key cross-window tests:
- `compute_drag_visual_x` tests (4): verify cursor-to-visual mapping with clamping.
- `compute_insertion_index` tests (7): verify drop-index computation for merge targets.
- `exceeds_tear_off` tests (7): verify directional thresholds (upward more sensitive than downward).
- `TabDragState` construction and cancel verification (5 tests).
- `sequential_drag_right/left_increments/decrements_index`: verify monotonic index progression during drag.
- `two_tab_reorder_swap_at_center`, `two_tab_reorder_just_before_center_stays`: boundary cases.

**Session-level cross-window tests** (from `app/tests.rs`):
- The two-window session tests exercise the session registry operations that underlie cross-window tab movement.

**Assessment:** Cross-window tab movement is thoroughly tested at the computation level (drag visual, insertion index, tear-off threshold, merge). The full end-to-end tear-off requires an OS event loop and is verified structurally. Platform coverage is complete: Windows, macOS, and Linux each have their own `#[cfg]` blocks in `tear_off.rs` with platform-appropriate drag mechanisms.

---

## 32.5 Section Completion

### Verification

- All 32.1-32.4 items verified as complete with implementation evidence.
- Tab management: create, close, duplicate, cycle, reorder -- all through mux.
- Multi-window: shared GPU (single `GpuState`), per-window font collection + renderer, correct lifecycle.
- No-flash window startup: `gpu.clear_surface()` before `set_visible(true)`.
- ConPTY-safe shutdown: `exit_app() -> !` calls `process::exit(0)` before any pane destructors.
- Cross-window tab movement preserves `SplitTree` (pane state and layout).

### Build Verification

Tests were run from the worktree at `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/`.

### Test Results

| Test Suite | Tests | Result |
|-----------|-------|--------|
| `oriterm -- tab_management` | 15 | All pass |
| `oriterm -- session::window` | 16 | All pass |
| `oriterm -- session::registry` | 16 | All pass |
| `oriterm -- session::tab` | 11 | All pass |
| `oriterm -- app::tests` | 20 | All pass |
| `oriterm -- tab_drag` | 36 | All pass |
| `oriterm -- mux_pump` | 6 | All pass |
| `oriterm -- event_loop_helpers` | 8 | All pass |
| Full `oriterm` crate | 2084 | All pass |
| Full `oriterm_mux` crate | 23 | All pass |

### File Size Compliance

All source files (excluding tests) are under 500 lines:
- `tab_management/mod.rs`: 449
- `tab_management/move_ops.rs`: 240
- `window_management.rs`: 435
- `window/mod.rs`: 318
- `session/window/mod.rs`: 155
- `session/registry/mod.rs`: 149
- `session/tab/mod.rs`: 180
- `window_context.rs`: 134
- `chrome/resize.rs`: 210
- `tab_drag/tear_off.rs`: 476

### Code Hygiene

- **Module docs**: All files have `//!` module docs.
- **Public item docs**: All `pub` functions have `///` docs.
- **Test organization**: All tests follow the sibling `tests.rs` pattern (separate file, no inline test modules, `super::` imports).
- **Import organization**: Three-group pattern (std, external, crate) observed across all files.
- **Error handling**: No `unwrap()` in production code. `exit_app` uses `process::exit(0)`, not panic. MuxBackend errors logged with `log::error!`.
- **Newtypes for IDs**: `TabId`, `WindowId`, `SessionWindowId`, `PaneId` -- all newtypes, no bare `u64`.
- **Platform coverage**: Cross-platform `#[cfg]` blocks for Windows, macOS, and Linux in tear_off.rs (positions, show, OS drag).
- **`#[allow]` annotations**: All have `reason = "..."` as required.

### Coverage Gaps

1. **No headless integration test for the full `close_tab -> exit_app` path.** The `exit_app() -> !` signature prevents unit testing the call itself (it terminates the process). The test for "closing last tab in last window triggers shutdown" is structural: `closing_last_tab_leaves_empty` verifies the Window becomes empty, and the production code at line 122 calls `self.exit_app()` when `is_last` is true. This is acceptable -- testing `process::exit` would require process-level integration tests.

2. **No headless test for pane background-thread drop.** The pattern (`mux.cleanup_closed_pane(id)` calls `std::thread::spawn(move || drop(pane))`) is in the mux layer. The mux tests verify pane lifecycle, but the specific background-drop behavior is structural. Acceptable.

3. **CWD inheritance test is structural.** The plan item "CWD inheritance: new tab starts in active pane's directory (via CWD in SpawnConfig)" is verified by reading the code: `new_tab_in_window` at line 28 calls `self.mux.as_ref()?.pane_cwd(id)` and passes it to `SpawnConfig.cwd`. No unit test mocks the mux to verify this end-to-end. This would require a test fixture with a fake MuxBackend that tracks `pane_cwd` calls. Acceptable given the straightforward delegation pattern.

### Pins and Dependencies

- **Section 31 dependency**: Section 32 depends on InProcessMux being wired into App (Section 31). The presence of `self.mux` as `Option<Box<dyn MuxBackend>>` throughout all Section 32 code confirms this dependency is satisfied.
- **Section 44 supersession**: Section 32.4 header correctly notes "SUPERSEDED by Section 44 (Multi-Process Window Architecture)" for the primary multi-window model. The in-process cross-window operations remain as fallback/embedded mode.
- **Section 17 wiring**: Tab drag infrastructure referenced in 32.1 (reorder) and 32.4 (tear-off) is present in `app/tab_drag/`.

---

## VERDICT: PASS

All checklist items in Section 32 are implemented with working code, passing tests, and proper hygiene. The implementation correctly separates concerns: mux owns pane lifecycle, session registry owns tab/window model, App orchestrates GUI. ConPTY-safe shutdown ordering is structurally correct. Cross-platform coverage is complete. No issues found.
