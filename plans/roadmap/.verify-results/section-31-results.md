# Section 31: In-Process Mux + Multi-Pane Rendering — Verification Results

**Verified by:** Claude Opus 4.6 (1M context)
**Date:** 2026-03-29
**Branch:** dev
**Status:** PASS

## Context Loaded

- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/CLAUDE.md` (full)
- `.claude/rules/code-hygiene.md` (full)
- `.claude/rules/impl-hygiene.md` (full)
- `.claude/rules/test-organization.md` (full)
- `.claude/rules/crate-boundaries.md` (loaded via system reminder)
- `plans/roadmap/section-31-in-process-mux.md` (full)

---

## 31.1 InProcessMux

### Code Review

**Source:** `oriterm_mux/src/in_process/mod.rs` (164 lines), `oriterm_mux/src/in_process/event_pump.rs` (126 lines)

The `InProcessMux` struct at lines 39-58 owns:
- `pane_registry: PaneRegistry` — flat pane storage
- `local_domain: LocalDomain` — concrete domain (not trait-object; plan notes extension in Section 35)
- `domain_alloc: IdAllocator<DomainId>` — with `#[allow(dead_code, reason = "...")]` for Section 35
- `pane_alloc: IdAllocator<PaneId>` — pane ID allocation
- `event_tx/event_rx: mpsc` — event channels
- `notifications: Vec<MuxNotification>` — double-buffer pattern

**Pane operations verified:**
- `spawn_standalone_pane()` (lines 92-113): allocates PaneId, delegates to `LocalDomain::spawn_pane`, registers in PaneRegistry. Returns `(PaneId, Pane)`.
- `close_pane()` / `close_pane_with_exit_code()` (lines 120-139): unregisters from registry, pushes `MuxNotification::PaneClosed`. Returns `ClosePaneResult`.
- `get_pane_entry()` (lines 142-144): delegates to registry.

**Event pump verified** (`event_pump.rs`):
- `poll_events()` (lines 23-98): `while let Ok(event) = self.event_rx.try_recv()` handles all `MuxEvent` variants: `PaneOutput`, `PaneExited`, `PaneTitleChanged`, `PaneIconChanged`, `PaneCwdChanged`, `CommandComplete`, `PaneBell`, `PtyWrite`, `ClipboardStore`, `ClipboardLoad`.
- `drain_notifications()` (lines 105-108): uses `std::mem::swap` for double-buffer pattern, clearing caller's buffer first.
- `discard_notifications()` (line 111): direct clear.

**Plan deviation:** The plan lists `Tab` and `Window` operations (create_tab, split_pane, create_window, close_window) as part of InProcessMux, but in the actual implementation the session model (tabs, windows, split trees) lives in `oriterm/src/session/SessionRegistry`, not in the mux. The mux is a flat pane server per the architecture. This is the correct design (matches CLAUDE.md: "oriterm_mux is a pane-only server — no tabs, windows, sessions, or layouts"). The plan's checklist items for tab/window operations are satisfied by the `SessionRegistry` + `MuxBackend` trait pairing in `oriterm/src/app/`.

### Tests

**File:** `oriterm_mux/src/in_process/tests.rs` (661 lines)

31 tests, all passing. Test categories:

| Category | Tests | Evidence |
|----------|-------|----------|
| close_pane lifecycle | 4 | `close_pane_not_found`, `close_pane_emits_pane_closed`, `close_pane_twice_returns_not_found_on_second_call`, `close_pane_removes_from_registry` |
| Event pump dispatch | 9 | `poll_events_handles_title_change`, `poll_events_clipboard_store_emits_notification`, `poll_events_bell_emits_alert`, `poll_events_cwd_change_missing_pane_no_panic`, `poll_events_pty_write_missing_pane_no_panic`, `poll_events_command_complete_emits_notification`, `poll_events_command_complete_missing_pane_no_panic`, `poll_events_icon_changed_emits_title_notification`, `poll_events_icon_changed_missing_pane_no_panic` |
| Notification ordering | 3 | `drain_notifications_clears_queue`, `drain_notifications_preserves_insertion_order`, `drain_notifications_preserves_clipboard_data` |
| PaneExited lifecycle | 3 | `batch_pane_exits_emit_pane_closed_for_each`, `stale_pane_map_during_event_dispatch`, `pane_output_after_pane_closed_is_noop` |
| Edge cases | 8 | `poll_events_with_empty_channel_is_noop`, `poll_events_processes_multiple_events`, `pane_dirty_produced_for_absent_pane`, `clipboard_load_unknown_pane_produces_notification`, `empty_notification_buffer_short_circuits`, `drain_double_buffer_no_cross_cycle_accumulation`, `sender_dropped_during_poll_drains_remaining`, `pane_closed_notification_carries_correct_id` |
| Send trait bounds | 1 | `mux_types_are_send` |
| Domain allocator | 1 | `domain_alloc_persisted_in_struct` |
| Event tx | 1 | `event_tx_can_be_cloned_and_used` |

**Test quality:** Tests use `inject_test_pane()` helper to register test panes without spawning real PTYs. Covers missing-pane edge cases, stale event ordering, double-buffer semantics, and Send trait bounds. Good coverage.

```
cargo test -p oriterm_mux --lib -- in_process
test result: ok. 31 passed; 0 failed; 0 ignored
```

### Hygiene

- File sizes: `mod.rs` 164 lines, `event_pump.rs` 126 lines. Both well under 500.
- `#[cfg(test)] mod tests;` at bottom of `mod.rs` (line 163-164). Correct sibling pattern.
- Module docs (`//!`) present on both files.
- `#[allow(dead_code)]` on `domain_alloc` has a `reason` attribute.
- Import organization follows 3-group pattern.

**Verdict: PASS**

---

## 31.2 App Rewiring

### Code Review

**Source:** `oriterm/src/app/mod.rs` (477 lines), `oriterm/src/app/constructors.rs` (157 lines), `oriterm/src/app/mux_pump/mod.rs` (232 lines), `oriterm/src/app/pane_accessors.rs` (138 lines), `oriterm/src/app/init/mod.rs` (~361 lines)

**App struct fields verified** (mod.rs lines 119-242):
- `mux: Option<Box<dyn MuxBackend>>` — abstracts embedded vs daemon (correct; plan said `Option<InProcessMux>` which evolved to trait-object)
- `session: SessionRegistry` — local session model (tabs, windows, layouts)
- `active_window: Option<SessionWindowId>` — maps to focused TermWindow
- `notification_buf: Vec<MuxNotification>` — double-buffer for pump
- `pane_selections: HashMap<PaneId, Selection>` — client-side selection
- `mark_cursors: HashMap<PaneId, MarkCursor>` — client-side mark mode
- No `tab: Option<Tab>` field (verified with grep; fully removed)

**Pane accessors verified** (mod.rs + pane_accessors.rs):
- `active_pane_id()` (mod.rs lines 391-397): resolves through session model: `active_window` -> `get_window` -> `active_tab` -> `get_tab` -> `active_pane()`.
- `active_pane_id_for_window()` (mod.rs lines 381-388): window-specific variant.
- `pane_mode()` (mod.rs lines 403-408): delegates to `MuxBackend::pane_mode`.
- `write_pane_input()` (pane_accessors.rs lines 80-83): delegates to `MuxBackend::send_input`.
- `pane_selection()`, `set_pane_selection()`, `clear_pane_selection()`, `enter_mark_mode()`, `exit_mark_mode()` — all client-side state on `App`.

**Mux pump verified** (mux_pump/mod.rs lines 23-51):
- `pump_mux_events()`: checks daemon connectivity, skips if no pending wakeup, calls `mux.poll_events()`, drains notifications, handles each via `with_drained_notifications(Self::handle_mux_notification)`.
- `handle_mux_notification()` (lines 54-111): handles `PaneOutput` (clear selection, invalidate URL, mark dirty), `PaneClosed` (cleanup), `PaneMetadataChanged` (sync tab bar), `CommandComplete`, `PaneBell`, `ClipboardStore`, `ClipboardLoad`.
- Called from `about_to_wait()` (event_loop.rs line 365).

**Init path verified** (init/mod.rs):
- `App::new()` (constructors.rs lines 73-93): creates `EmbeddedMux::new(wakeup)`, boxes as `dyn MuxBackend`.
- `App::new_daemon()` (constructors.rs lines 32-67): creates `MuxClient::connect()`, boxes as `dyn MuxBackend`.
- `try_init()` (init/mod.rs): creates window, spawns font discovery, inits GPU, allocates session window, calls `create_initial_tab()`.
- `create_initial_tab()` (init/mod.rs lines 308-347): calls `mux.spawn_pane()`, creates session Tab, adds to session Window.

**Old Tab removal verified:**
- No `tab: Option<Tab>` field in App (grep confirms).
- No `use crate::tab::Tab` in app/ (grep confirms).
- No `struct Tab` in oriterm/src/ (grep confirms — old Tab type fully removed).
- No `#[allow(dead_code)]` in app/mod.rs (grep confirms — cleanup completed beyond plan).

### Tests

**File:** `oriterm/src/app/mux_pump/tests.rs` (50 lines) — 6 tests for `format_duration_body()` helper.

```
cargo test -p oriterm -- mux_pump::tests
test result: ok. 6 passed; 0 failed; 0 ignored
```

The `handle_mux_notification()` logic is tested indirectly through the full test suite (3700+ tests passing). The pump is a thin dispatch layer that calls well-tested subsystems (selection, tab bar, clipboard). Direct unit testing of `handle_mux_notification` would require constructing a full App, which needs GPU/winit — correctly left as integration-level verification.

### Hygiene

- `mod.rs` is 477 lines (under 500 limit).
- `mux_pump/mod.rs` is 232 lines. Has `#[cfg(test)] mod tests;` at bottom.
- `pane_accessors.rs` is 138 lines.
- Import organization correct.
- `with_drained_notifications()` has `#[allow(clippy::iter_with_drain, reason = "...")]`.

**Verdict: PASS**

---

## 31.3 Multi-Pane Rendering

### Code Review

**Source:** `oriterm/src/app/redraw/multi_pane.rs` (505 lines), `oriterm/src/gpu/window_renderer/multi_pane.rs` (226 lines), `oriterm/src/gpu/prepare/mod.rs` (fg_dim threading)

**Rendering pipeline verified:**

1. **Single-pane detection** (redraw/mod.rs lines 41-44): `compute_pane_layouts()` returns `None` for single-pane tabs, triggering early return to fast path.

2. **`compute_pane_layouts()`** (redraw/multi_pane.rs lines 31-93): Walks session model to get `SplitTree` and `FloatingLayer`, delegates to `compute_all()` for layout computation. Handles zoomed pane special case (single full-area layout). Returns `None` when tree has 1 pane and no floating.

3. **`handle_redraw_multi_pane()`** (redraw/multi_pane.rs lines 106-483): Linear pipeline:
   - Copies pane selections and mark cursors to scratch buffers (lines 114-125).
   - Calls `renderer.begin_multi_pane_frame()` (line 168).
   - For each layout: dirty check, snapshot refresh, swap/extract, prepare_pane_into (lines 179-375).
   - Appends dividers via `renderer.append_dividers()` (lines 394-395).
   - Floating pane decorations (shadow + border) via `renderer.append_floating_decoration()` (lines 398-401).
   - Focus border when >1 pane via `renderer.append_focus_border()` (lines 403-408).
   - Chrome, tab bar, overlays, search bar (lines 411-459).
   - Render to surface (lines 469-471).

4. **`begin_multi_pane_frame()`** (gpu/window_renderer/multi_pane.rs lines 27-40): Resets atlas frame counters, clears PreparedFrame, sets viewport and clear color.

5. **`prepare_pane_into()`** (gpu/window_renderer/multi_pane.rs lines 52-107): Shapes rows, caches glyphs, fills into target PreparedFrame with origin offset. Inherits full window viewport for off-screen culling.

6. **`append_dividers()`** (lines 113-126): Pushes background rect instances for each DividerLayout. Color: configurable via `PaneConfig::effective_divider_color()`, default `Rgb(80, 80, 80)`.

7. **`append_focus_border()`** (lines 174-225): 4 cursor-layer rects (top, bottom, left, right), 2px border width. Color: configurable via `PaneConfig::effective_focus_border_color()`, default cornflower blue `Rgb(100, 149, 237)`.

8. **`append_floating_decoration()`** (lines 133-168): Drop shadow (background layer, 0.3 alpha) + accent-colored border (UI rects layer, 1px, 2px corner radius).

**fg_dim threading verified:**
- `FrameInput.fg_dim` field at `gpu/frame_input/mod.rs` line 335. Default 1.0 in `test_grid()`.
- Threaded through `GlyphEmitter` at `gpu/prepare/emit.rs` line 35.
- Applied in `fill_frame_shaped` at `gpu/prepare/mod.rs` lines 305, 418, 435.
- Set per pane in multi_pane.rs lines 347-350: `fg_dim = if focused || !dim_inactive { 1.0 } else { inactive_opacity }`.

**PaneConfig verified:**
- `config/mod.rs` lines 215-271: `PaneConfig { divider_px, min_cells, dim_inactive, inactive_opacity, divider_color, focus_border_color }`.
- Defaults: `divider_px: 1.0`, `min_cells: (10, 3)`, `dim_inactive: false`, `inactive_opacity: 0.7`.
- `effective_inactive_opacity()`: clamps to [0.0, 1.0], NaN defaults to 0.7.
- `effective_divider_color()`, `effective_focus_border_color()`: parse optional hex strings, fallback to defaults.

### Tests

**Multi-pane prepare tests** (`gpu/prepare/tests.rs`, lines 2936-3105):

| Test | What it verifies |
|------|-----------------|
| `fg_dim_default_alpha_is_one` | Default `fg_dim=1.0` produces alpha 1.0 in glyph instances |
| `fg_dim_reduces_glyph_alpha` | `fg_dim=0.7` produces alpha ~0.7 in glyph instances |
| `fill_frame_shaped_accumulates_without_clearing` | Two `fill_frame_shaped` calls accumulate (2+2=4 bg instances) |
| `two_panes_at_correct_offsets` | Pane A at x=0, Pane B at x=400 — correct pixel offsets |
| `cursor_only_in_focused_pane` | Focused pane emits 1 cursor; unfocused pane adds 0 more |
| `lower_pane_origin_is_not_culled_by_local_pane_height` | Lower split pane at y=200 renders correctly (not culled by pane-local viewport) |

**PaneConfig tests** (`config/tests.rs`, 8 tests):
- `pane_config_defaults`, `pane_config_roundtrip`, `pane_config_partial_toml` — serialization
- `pane_config_effective_opacity_clamps`, `pane_config_effective_opacity_nan_defaults` — validation
- `pane_config_color_defaults`, `pane_config_color_overrides`, `pane_config_invalid_color_falls_back` — color config

**Multi-pane scratch tests** (`app/redraw/multi_pane.rs`, inline, 3 tests):
- `reextracts_when_shared_scratch_belongs_to_another_pane`
- `skips_reextract_only_when_scratch_already_matches_clean_pane`
- `reextracts_when_content_changed_or_frame_missing`

All passing:
```
cargo test -p oriterm -- gpu::prepare::tests::fg_dim
test result: ok. 2 passed; 0 failed; 0 ignored

cargo test -p oriterm -- gpu::prepare::tests::cursor_only gpu::prepare::tests::fill_frame gpu::prepare::tests::two_panes gpu::prepare::tests::lower_pane
test result: ok. 4 passed; 0 failed; 0 ignored

cargo test -p oriterm -- config::tests::pane_config
test result: ok. 8 passed; 0 failed; 0 ignored

cargo test -p oriterm -- multi_pane
test result: ok. 3 passed; 0 failed; 0 ignored
```

### Hygiene

- `redraw/multi_pane.rs` is 505 lines. Production code ends at line 484; lines 486-505 are an inline `#[cfg(test)] mod tests { }` block. **Minor hygiene violation:** inline tests should be in a sibling `tests.rs` file per test-organization.md. However, the production code is 484 lines (under 500). The 3 inline tests are trivial boolean-logic assertions. Not a blocking issue.
- `gpu/window_renderer/multi_pane.rs` is 226 lines. Clean.
- `#[expect(clippy::too_many_lines, reason = "...")]` on `handle_redraw_multi_pane` and `#[expect(clippy::too_many_arguments, reason = "...")]` on `prepare_pane_into` — both with documented reasons.
- `redraw/mod.rs` is a directory module with `draw_helpers.rs`, `multi_pane.rs`, `preedit.rs`, `search_bar.rs` submodules. Good split.

**Verdict: PASS** (minor: inline tests in multi_pane.rs)

---

## 31.4 PaneRenderCache

### Code Review

**Source:** `oriterm/src/gpu/pane_cache/mod.rs` (132 lines)

`PaneRenderCache` (lines 28-129):
- `entries: HashMap<PaneId, CachedPaneFrame>` — one entry per pane.
- `CachedPaneFrame { prepared: PreparedFrame, layout: PaneLayout }` — stores layout for invalidation.
- `get_or_prepare()` (lines 48-85): If `!dirty && layout matches` -> return cached. Otherwise clear, call `prepare_fn`, update layout. Handles both `Occupied` and `Vacant` entries.
- `is_cached()` (lines 88-92): Layout-aware cache check.
- `get_cached()` (lines 98-99): Read-only access without layout check.
- `invalidate()` (lines 107-109): Single-pane invalidation. `#[allow(dead_code)]` with reason.
- `remove()` (lines 112-114): Frees memory for closed pane.
- `retain_only()` (lines 120-123): Batch prune. `#[allow(dead_code)]` with reason.
- `invalidate_all()` (lines 126-128): Global invalidation (atlas rebuild, font change).

**Integration verified:**
- `handle_redraw_multi_pane()` calls `ctx.pane_cache.is_cached()` and `ctx.pane_cache.get_or_prepare()` for dirty panes, `ctx.pane_cache.get_cached()` for clean panes.
- `handle_pane_closed()` calls `ctx.pane_cache.remove(id)`.
- `handle_dpi_change()` calls `ctx.pane_cache.invalidate_all()`.
- `handle_theme_changed()` calls `ctx.pane_cache.invalidate_all()`.

### Tests

**File:** `oriterm/src/gpu/pane_cache/tests.rs` (370 lines), 17 tests.

| Test | What it verifies |
|------|-----------------|
| `clean_pane_returns_cached_frame` | Cache hit: `prepare_fn` NOT called, frame preserved |
| `dirty_pane_calls_prepare_fn` | Cache miss: `prepare_fn` called, old instances replaced |
| `layout_change_triggers_reprepare` | Same pane, different layout -> re-prepare |
| `invalidate_all_forces_reprepare` | Both panes re-prepare after `invalidate_all` |
| `remove_frees_entry` | Removed pane forces fresh prepare |
| `extend_from_merges_cached_frames` | Two cached frames merge into main (2+1=3 bg instances) |
| `position_change_same_size_triggers_reprepare` | Same size, different x -> re-prepare |
| `selective_dirty_only_reprepares_dirty_pane` | Only dirty pane re-prepared; clean pane cached |
| `is_cached_true_after_prepare` | `is_cached` returns true post-prepare |
| `is_cached_false_after_remove` | `is_cached` returns false post-remove |
| `is_cached_false_after_invalidate_all` | `is_cached` returns false post-invalidate |
| `is_cached_false_when_layout_mismatches` | Different layout -> cache miss |
| `get_cached_returns_some_after_prepare` | Read-only access works |
| `get_cached_returns_none_for_unknown_pane` | Unknown pane -> None |
| `get_cached_returns_none_after_remove` | Removed pane -> None |
| `invalidate_single_pane_triggers_reprepare` | Single-pane invalidation; other pane stays cached |
| `retain_only_removes_stale_entries` | Batch prune preserves active, removes stale |

All passing:
```
cargo test -p oriterm -- pane_cache
test result: ok. 17 passed; 0 failed; 0 ignored
```

### Hygiene

- `mod.rs` is 132 lines. Clean.
- `#[cfg(test)] mod tests;` at bottom (line 131-132). Correct sibling pattern.
- Module docs present. All pub items documented.
- `#[allow(dead_code, reason = "...")]` on `invalidate()` and `retain_only()` with clear reasons.

**Verdict: PASS**

---

## 31.5 Section Completion

### Full Test Suite

```
./test-all.sh
All tests passed.
```

All workspace tests pass. No regressions introduced.

### File Size Audit

| File | Lines | Status |
|------|-------|--------|
| `oriterm_mux/src/in_process/mod.rs` | 164 | OK |
| `oriterm_mux/src/in_process/event_pump.rs` | 126 | OK |
| `oriterm/src/app/mod.rs` | 477 | OK |
| `oriterm/src/app/mux_pump/mod.rs` | 232 | OK |
| `oriterm/src/app/pane_accessors.rs` | 138 | OK |
| `oriterm/src/app/redraw/mod.rs` | 357 | OK |
| `oriterm/src/app/redraw/multi_pane.rs` | 505 | Minor (484 prod + 21 inline test) |
| `oriterm/src/gpu/pane_cache/mod.rs` | 132 | OK |
| `oriterm/src/gpu/window_renderer/multi_pane.rs` | 226 | OK |
| `oriterm/src/gpu/frame_input/mod.rs` | 468 | OK |

### Architecture Compliance

- **Crate boundaries respected:** `InProcessMux` lives in `oriterm_mux` (pane lifecycle). Session model lives in `oriterm/src/session/`. GUI rendering lives in `oriterm/src/gpu/`. No cross-crate boundary violations.
- **No concrete external-resource types in logic layers:** `InProcessMux` uses `mpsc::Sender<MuxEvent>` (not `EventLoopProxy`). App receives wakeups through `Arc<dyn Fn() + Send + Sync>` callback.
- **MuxBackend trait:** Clean abstraction over embedded vs daemon. 48 methods covering event pump, pane CRUD, grid ops, search, clipboard, snapshots.
- **Single-pane fast path preserved:** `compute_pane_layouts()` returns `None` for single-pane tabs, falling through to the original `handle_redraw()` path with zero overhead.
- **No `unwrap()` in library code:** All mux operations return `Result` or use `Option` chains.
- **No dead code:** Old `Tab` type fully removed from `oriterm/src`. No stale `#[allow(dead_code)]` on actively-used types.

### Plan Accuracy

The plan's 31.1 section describes `InProcessMux` with integrated tab/window management (create_tab, split_pane, close_tab, create_window, close_window). The actual implementation correctly separates concerns:
- `InProcessMux` = flat pane server (CRUD + event pump)
- `SessionRegistry` = local session model (tabs, windows, split trees)
- `MuxBackend` trait = unified API consumed by App

This is the right architecture (matches CLAUDE.md) even though the plan's checklist items are satisfied at a different layer than described. All functionality is present.

---

## Summary

| Subsection | Tests | Status | Notes |
|------------|-------|--------|-------|
| 31.1 InProcessMux | 31 | PASS | Clean pane lifecycle, event pump, notifications |
| 31.2 App Rewiring | 6 + full suite | PASS | Old Tab removed, MuxBackend wired, session model in use |
| 31.3 Multi-Pane Rendering | 17 | PASS | fg_dim, offsets, cursor, dividers, focus border, config |
| 31.4 PaneRenderCache | 17 | PASS | Cache hit/miss, invalidation, layout change, batch prune |
| 31.5 Completion | full suite | PASS | All tests pass, no regressions |

**Total section-31-related tests:** 71 (31 in_process + 6 mux_pump + 17 pane_cache + 6 prepare/multi-pane + 8 pane_config + 3 multi_pane scratch)

**Minor findings (non-blocking):**
1. `redraw/multi_pane.rs` has inline `mod tests { }` (should be sibling `tests.rs` per test-organization.md). 3 trivial tests.
2. `redraw/multi_pane.rs` is 505 lines total (484 production + 21 inline test). Extracting tests to sibling file would bring it under 500.

**Overall: PASS**
