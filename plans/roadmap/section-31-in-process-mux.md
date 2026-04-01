---
section: 31
title: In-Process Mux + Multi-Pane Rendering
status: in-progress
reviewed: true
last_verified: "2026-03-29"
tier: 4M
goal: Wire up InProcessMux, rewire App to use mux layer, render multiple panes per tab with correct viewport offsets and dividers
sections:
  - id: "31.1"
    title: InProcessMux
    status: complete
  - id: "31.2"
    title: App Rewiring
    status: complete
  - id: "31.3"
    title: Multi-Pane Rendering
    status: complete
  - id: "31.4"
    title: PaneRenderCache
    status: complete
  - id: "31.5"
    title: Section Completion
    status: complete
---

# Section 31: In-Process Mux + Multi-Pane Rendering

**Status:** Complete
**Goal:** Create the `InProcessMux` that runs all mux logic in the same process (no daemon). Rewire `App` to route all pane/tab/window operations through the mux. Implement multi-pane rendering with per-pane viewport offsets, dividers, and focus borders.

> **Architectural note (verified 2026-03-29):** Plan describes `InProcessMux` with integrated tab/window management (create_tab, split_pane, create_window, close_window). The implementation correctly separates concerns: `InProcessMux` = flat pane server (CRUD + event pump) in `oriterm_mux`; `SessionRegistry` = local session model (tabs, windows, split trees) in `oriterm/src/session/`; `MuxBackend` trait = unified API consumed by App. All functionality is present, just at different layers than described.

**Crate:** `oriterm_mux` (InProcessMux), `oriterm` (App rewiring, rendering)
**Dependencies:** Section 29 (layout engine), Section 30 (Pane, Domain, registries), Section 05 (GPU rendering)
**Prerequisite:** Sections 29 and 30 complete.

**Inspired by:**
- WezTerm: in-process `Mux` singleton with notification channels
- Ghostty: per-surface rendering with viewport offsets
- Alacritty: `prepare_frame_into` with offset parameters (already exists in our codebase)

**Key constraint:** After this section, the single-pane path must still work identically — a tab with one pane renders exactly as before. Multi-pane is additive.

---

## 31.1 InProcessMux

The in-process mux is the synchronous fast path — all mux operations happen on the main thread via direct method calls. No IPC, no serialization, no daemon. This is the default mode; the daemon (Section 34) layers on top later.

**Actual location:** `oriterm_mux/src/in_process/mod.rs` (164 lines), `oriterm_mux/src/in_process/event_pump.rs` (126 lines)

**Reference:** WezTerm `mux/src/lib.rs` (Mux struct, get/set pattern)

- [x] `InProcessMux` struct: (verified 2026-03-29)
  - [x] `pane_registry: PaneRegistry` (verified 2026-03-29)
  - [x] `local_domain: LocalDomain` (concrete; extended to domain registry in Section 35) (verified 2026-03-29)
  - [x] `domain_alloc: IdAllocator<DomainId>` — `#[allow(dead_code)]` with reason for Section 35 (verified 2026-03-29)
  - [x] `pane_alloc: IdAllocator<PaneId>` (verified 2026-03-29)
  - [x] `notifications: Vec<MuxNotification>` + `drain_notifications()` double-buffer pattern (verified 2026-03-29)
  - [x] `event_tx/event_rx: mpsc` — event channels (verified 2026-03-29)
  - NOTE: No `session: SessionRegistry`, `tab_alloc`, `window_alloc` — these are in `oriterm/src/session/` (GUI-owned)
- [x] Pane operations (on InProcessMux): (verified 2026-03-29)
  - [x] `spawn_standalone_pane()` — allocates PaneId, delegates to `LocalDomain::spawn_pane`, registers in PaneRegistry, returns `(PaneId, Pane)` (verified 2026-03-29)
  - [x] `close_pane()` / `close_pane_with_exit_code()` — unregisters from registry, pushes `MuxNotification::PaneClosed`, returns `ClosePaneResult` (verified 2026-03-29)
  - [x] `get_pane_entry(&self, pane_id) -> Option<&PaneEntry>` (verified 2026-03-29)
- [x] Tab/window operations (on SessionRegistry in `oriterm/src/session/`, NOT InProcessMux): (verified 2026-03-29)
  - [x] Tab creation via mux + session model pairing (verified 2026-03-29)
  - [x] Split pane via SplitTree immutable split_at + session Tab set_tree (verified 2026-03-29)
  - [x] Close tab/window via session registry (verified 2026-03-29)
- [x] Event pump (`event_pump.rs`): (verified 2026-03-29)
  - [x] `poll_events()` — `while let Ok(event) = self.event_rx.try_recv()` handles all 10 `MuxEvent` variants exhaustively (verified 2026-03-29)
    - [x] `PaneOutput`, `PaneExited`, `PaneTitleChanged`, `PaneIconChanged`, `PaneCwdChanged`, `CommandComplete`, `PaneBell`, `PtyWrite`, `ClipboardStore`, `ClipboardLoad` (verified 2026-03-29)
  - [x] `drain_notifications()` — `std::mem::swap` for double-buffer pattern (verified 2026-03-29)
  - [x] `discard_notifications()` — direct clear (verified 2026-03-29)
  - [x] Called from `App::about_to_wait()` on every event loop iteration (wired in 31.2) (verified 2026-03-29)

**Tests:** `oriterm_mux/src/in_process/tests.rs` (661 lines) — 31 tests, ALL PASS (verified 2026-03-29)
- [x] close_pane lifecycle: not_found, emits_pane_closed, twice_returns_not_found, removes_from_registry (verified 2026-03-29)
- [x] Event pump dispatch: title_change, clipboard_store, bell, cwd_missing_pane, pty_write_missing_pane, command_complete, icon_changed (verified 2026-03-29)
- [x] Notification ordering: clears_queue, preserves_insertion_order, preserves_clipboard_data (verified 2026-03-29)
- [x] PaneExited lifecycle: batch_exits, stale_pane_map, output_after_closed (verified 2026-03-29)
- [x] Edge cases: empty_channel_noop, multiple_events, absent_pane, unknown_pane_clipboard, empty_buffer, double_buffer, sender_dropped, correct_id (verified 2026-03-29)
- [x] Send trait bounds (verified 2026-03-29)
- [x] Domain allocator persisted (verified 2026-03-29)
- [x] Event tx clone and use (verified 2026-03-29)

---

## 31.2 App Rewiring

Rewire the `App` struct to use `InProcessMux` as the source of truth for all pane/tab/window state. The App becomes a thin GUI shell that forwards input and renders output.

**File:** `oriterm/src/app/mod.rs`

- [x] Add mux field to `App`: (verified 2026-03-29)
  - [x] `mux: Option<Box<dyn MuxBackend>>` — abstracts embedded vs daemon (evolved from plan's `Option<InProcessMux>`) (verified 2026-03-29)
  - [x] `session: SessionRegistry` — local session model (verified 2026-03-29)
  - [x] Remove direct `tab: Option<Tab>` field — fully removed, no stale type (verified 2026-03-29)
  - [x] `pane_selections: HashMap<PaneId, Selection>` — client-side selection (verified 2026-03-29)
  - [x] `mark_cursors: HashMap<PaneId, MarkCursor>` — client-side mark mode (verified 2026-03-29)
  - [x] `active_window: Option<SessionWindowId>` — maps to focused TermWindow (verified 2026-03-29)
  - [x] `notification_buf: Vec<MuxNotification>` — double-buffer for pump (verified 2026-03-29)
  - [x] `active_pane_id() -> Option<PaneId>` — resolves through session model chain (verified 2026-03-29)
  - [x] `active_pane_id_for_window()` — window-specific variant (verified 2026-03-29)
- [x] Rewire `about_to_wait`: (verified 2026-03-29)
  - [x] `pump_mux_events()` in `app/mux_pump/mod.rs` — called from `about_to_wait()` (event_loop.rs line 365) (verified 2026-03-29)
  - [x] Checks daemon connectivity, skips if no pending wakeup (verified 2026-03-29)
  - [x] `mux.poll_events()` then `drain_notifications()` via `with_drained_notifications(Self::handle_mux_notification)` (verified 2026-03-29)
  - [x] `handle_mux_notification()` handles: (verified 2026-03-29)
    - [x] `PaneOutput` → clear selection, invalidate URL, mark dirty (verified 2026-03-29)
    - [x] `PaneClosed` → cleanup (verified 2026-03-29)
    - [x] `PaneMetadataChanged` → sync tab bar (verified 2026-03-29)
    - [x] `CommandComplete` (verified 2026-03-29)
    - [x] `PaneBell` (verified 2026-03-29)
    - [x] `ClipboardStore/Load` → forward to clipboard system (verified 2026-03-29)
- [x] Rewire all `self.tab` references to `self.active_pane()` / `self.active_pane_mut()`:
  - [x] `app/mod.rs` — handle_dpi_change, handle_theme_changed, terminal_mode, sync_tab_bar_titles, handle_terminal_event
  - [x] `app/search_ui.rs` — all search operations
  - [x] `app/redraw.rs` — frame extraction
  - [x] `app/keyboard_input/mod.rs` — key dispatch, mark mode
  - [x] `app/mouse_report/mod.rs` — PTY mouse reporting
  - [x] `app/chrome/mod.rs` — chrome hit testing
  - [x] `app/config_reload.rs` — palette/resize
  - [x] `app/clipboard_ops/mod.rs` — copy/paste
  - [x] `app/mouse_selection/mod.rs` — selection lifecycle
  - [x] `app/mouse_input.rs` — press/drag/release (split-borrow pattern)
  - [x] `app/cursor_hover.rs` — URL hover detection
  - [x] `app/mark_mode/mod.rs` — Tab→Pane parameter types
- [x] Remove old Tab infrastructure: (verified 2026-03-29)
  - [x] Removed `tab: Option<Tab>` field from App — grep confirms no `tab: Option<Tab>` (verified 2026-03-29)
  - [x] No `use crate::tab::Tab` in app/ — grep confirms (verified 2026-03-29)
  - [x] No `struct Tab` in oriterm/src/ — old Tab type fully removed (verified 2026-03-29)
  - [x] No `#[allow(dead_code)]` in app/mod.rs — cleanup completed beyond plan (verified 2026-03-29)
- [x] Single-pane compatibility:
  - [x] A tab with one pane renders identically — `active_pane()` resolves through mux session model
  - [x] No layout overhead when `SplitTree` is `Leaf` — single pane path is unchanged

**Implementation notes:**
- Split-borrow pattern: `active_pane_id()` returns `Option<PaneId>` (Copy), avoids borrowing all of `self`
- `App::new()` creates `EmbeddedMux::new(wakeup)`, boxed as `dyn MuxBackend`
- `App::new_daemon()` creates `MuxClient::connect()`, boxed as `dyn MuxBackend`
- `try_init()` creates window, spawns font discovery, inits GPU, creates initial tab via mux

**Tests:** 6 mux_pump tests + full suite, ALL PASS (verified 2026-03-29)
- [x] All existing tests pass (3700+) (verified 2026-03-29)
- [x] `format_duration_body()` helper: 6 tests (verified 2026-03-29)
- [x] `handle_mux_notification()` tested indirectly through full suite — pump is thin dispatch layer (verified 2026-03-29)
- [x] App::mod.rs at 477 lines (under 500 limit) (verified 2026-03-29)

---

## 31.3 Multi-Pane Rendering

Render multiple panes per tab, each with its own viewport offset. The key change: `prepare_pane_into()` takes an origin offset so instances are positioned correctly within the overall frame.

**Actual locations:** `oriterm/src/app/redraw/multi_pane.rs` (505 lines), `oriterm/src/gpu/window_renderer/multi_pane.rs` (226 lines), `oriterm/src/gpu/prepare/mod.rs` (fg_dim threading)

**Reference:** Existing `prepare_frame_into` (already takes `FrameInput` with viewport)

- [x] `fill_frame_shaped` made `pub(crate)` for multi-pane direct calls (verified 2026-03-29)
- [x] `fg_dim: f32` field added to `FrameInput` for inactive pane dimming (verified 2026-03-29)
  - [x] Threaded through `GlyphEmitter` and all glyph push calls (shaped + unshaped paths) (verified 2026-03-29)
  - [x] Default 1.0 in extract and test_grid constructors (verified 2026-03-29)
- [x] `prepare_pane_into()` — shapes, caches, and fills one pane (appends to PreparedFrame) (verified 2026-03-29)
  - [x] Each pane's instances offset by origin `(pixel_rect.x, pixel_rect.y)` (verified 2026-03-29)
  - [x] Inherits full window viewport for off-screen culling (verified 2026-03-29)
- [x] Multi-pane frame loop in `handle_redraw_multi_pane()`: (verified 2026-03-29)
  - [x] `compute_pane_layouts()` → `compute_all(tree, floating, focused, desc)` (verified 2026-03-29)
  - [x] `begin_multi_pane_frame()` → clear PreparedFrame, set viewport (verified 2026-03-29)
  - [x] For each `PaneLayout`: dirty check, snapshot refresh, prepare_pane_into (verified 2026-03-29)
  - [x] After all panes: append dividers, floating decorations, focus border (verified 2026-03-29)
  - [x] Single pane optimization: `compute_pane_layouts()` returns `None` for single-pane tabs (verified 2026-03-29)
- [x] Divider rendering: (verified 2026-03-29)
  - [x] `append_dividers()` pushes background rect instances for each `DividerLayout` (verified 2026-03-29)
  - [x] Divider color: configurable via `PaneConfig::effective_divider_color()`, default `Rgb(80, 80, 80)` (verified 2026-03-29)
- [x] Focus border: (verified 2026-03-29)
  - [x] `append_focus_border()` — 2px border (4 cursor-layer rects) around focused pane (verified 2026-03-29)
  - [x] Color: configurable via `PaneConfig::effective_focus_border_color()`, default cornflower blue `Rgb(100, 149, 237)` (verified 2026-03-29)
  - [x] Only shown when `layouts.len() > 1` (verified 2026-03-29)
- [x] Floating pane decorations: (verified 2026-03-29)
  - [x] `append_floating_decoration()` — drop shadow (0.3 alpha) + accent border (1px, 2px radius) (verified 2026-03-29)
- [x] Inactive pane dimming (config-controlled): (verified 2026-03-29)
  - [x] `PaneConfig` struct: `dim_inactive`, `inactive_opacity`, `divider_px`, `min_cells`, `divider_color`, `focus_border_color` (verified 2026-03-29)
  - [x] `fg_dim` set to `inactive_opacity` for unfocused panes when `dim_inactive` enabled (verified 2026-03-29)
  - [x] `effective_inactive_opacity()`: clamps to [0.0, 1.0], NaN defaults to 0.7 (verified 2026-03-29)
- [x] `app/redraw.rs` → `app/redraw/mod.rs` directory module (verified 2026-03-29)
  - [x] `app/redraw/multi_pane.rs` — `compute_pane_layouts()` + `handle_redraw_multi_pane()` (verified 2026-03-29)
  - [x] Branching in `handle_redraw()`: multi-pane path dispatches via early return (verified 2026-03-29)

**Tests:** 6 prepare + 8 config + 3 scratch = 17 tests, ALL PASS (verified 2026-03-29)
- [x] `fg_dim_default_alpha_is_one` — default 1.0 produces alpha 1.0 (verified 2026-03-29)
- [x] `fg_dim_reduces_glyph_alpha` — fg_dim=0.7 produces alpha ~0.7 (verified 2026-03-29)
- [x] `fill_frame_shaped_accumulates_without_clearing` — two fills accumulate (2+2=4 bg instances) (verified 2026-03-29)
- [x] `two_panes_at_correct_offsets` — pane A at x=0, pane B at x=400 (verified 2026-03-29)
- [x] `cursor_only_in_focused_pane` — focused pane emits cursor; unfocused adds none (verified 2026-03-29)
- [x] `lower_pane_origin_is_not_culled_by_local_pane_height` — lower split pane renders correctly (verified 2026-03-29)
- [x] `pane_config_defaults` / `pane_config_roundtrip` / `pane_config_partial_toml` — config serialization (verified 2026-03-29)
- [x] `pane_config_effective_opacity_clamps` / `pane_config_effective_opacity_nan_defaults` — opacity validation (verified 2026-03-29)
- [x] `pane_config_color_defaults` / `pane_config_color_overrides` / `pane_config_invalid_color_falls_back` — color config (verified 2026-03-29)
- [x] 3 multi_pane scratch tests: reextract, skip, reextract on content change (verified 2026-03-29)

**Hygiene finding (verified 2026-03-29):**
- [ ] `redraw/multi_pane.rs` has inline `#[cfg(test)] mod tests { }` (3 trivial tests) — should be sibling `tests.rs` per test-organization.md
- [ ] `redraw/multi_pane.rs` is 505 lines total (484 production + 21 inline test) — extracting tests to sibling file would bring it under 500

---

## 31.4 PaneRenderCache

Per-pane `PreparedFrame` caching to avoid re-preparing unchanged panes on every frame. Only dirty panes get re-prepared; clean panes reuse their cached instances.

**Actual location:** `oriterm/src/gpu/pane_cache/mod.rs` (132 lines)

- [x] `PaneRenderCache`: (verified 2026-03-29)
  - [x] `entries: HashMap<PaneId, CachedPaneFrame>` (verified 2026-03-29)
  - [x] `CachedPaneFrame`: (verified 2026-03-29)
    - [x] `prepared: PreparedFrame` — cached GPU-ready instances (verified 2026-03-29)
    - [x] `layout: PaneLayout` — layout at time of preparation (for invalidation on resize) (verified 2026-03-29)
    - NOTE: No `generation: u64` field — layout comparison used for staleness instead
  - [x] `get_or_prepare(pane_id, layout, dirty, prepare_fn) -> &PreparedFrame` (verified 2026-03-29)
    - [x] If `!dirty && layout matches` -> return cached; otherwise clear, call prepare_fn, update (verified 2026-03-29)
    - [x] Handles both Occupied and Vacant entries (verified 2026-03-29)
  - [x] `is_cached()` — layout-aware cache check (verified 2026-03-29)
  - [x] `get_cached()` — read-only access without layout check (verified 2026-03-29)
  - [x] `invalidate(pane_id: PaneId)` — single-pane invalidation, `#[allow(dead_code)]` with reason (verified 2026-03-29)
  - [x] `remove(pane_id: PaneId)` — pane closed, free memory (verified 2026-03-29)
  - [x] `retain_only()` — batch prune, `#[allow(dead_code)]` with reason (verified 2026-03-29)
  - [x] `invalidate_all()` — atlas rebuild, font change, etc. (verified 2026-03-29)
- [x] Integration with frame loop: (verified 2026-03-29)
  - [x] `handle_redraw_multi_pane()` calls `pane_cache.is_cached()` and `get_or_prepare()` for dirty, `get_cached()` for clean (verified 2026-03-29)
  - [x] `handle_pane_closed()` calls `pane_cache.remove(id)` (verified 2026-03-29)
  - [x] `handle_dpi_change()` and `handle_theme_changed()` call `pane_cache.invalidate_all()` (verified 2026-03-29)

**Tests:** `oriterm/src/gpu/pane_cache/tests.rs` (370 lines) — 17 tests, ALL PASS (verified 2026-03-29)
- [x] Clean pane: `get_or_prepare` returns cached frame, `prepare_fn` NOT called (verified 2026-03-29)
- [x] Dirty pane: `get_or_prepare` calls `prepare_fn`, updates cache (verified 2026-03-29)
- [x] Layout change: triggers re-prepare even if not dirty (verified 2026-03-29)
- [x] `invalidate_all`: forces all panes to re-prepare (verified 2026-03-29)
- [x] `remove`: frees memory for closed pane (verified 2026-03-29)
- [x] `extend_from_merges_cached_frames`: two cached frames merge into main (verified 2026-03-29)
- [x] Position change same size triggers re-prepare (verified 2026-03-29)
- [x] Selective dirty: only dirty pane re-prepared, clean cached (verified 2026-03-29)
- [x] `is_cached` true/false after prepare/remove/invalidate/layout mismatch (verified 2026-03-29)
- [x] `get_cached` returns Some/None correctly (verified 2026-03-29)
- [x] `invalidate` single pane (verified 2026-03-29)
- [x] `retain_only` removes stale entries (verified 2026-03-29)

---

## 31.5 Section Completion

- [x] All 31.1–31.4 items complete (verified 2026-03-29)
- [x] `InProcessMux` handles pane CRUD with correct notification flow (verified 2026-03-29)
- [x] App rewired: `MuxBackend` trait abstracts embedded vs daemon, session model in `oriterm/src/session/` (verified 2026-03-29)
- [x] Multi-pane rendering: each pane at correct offset, dividers between, focus border on active, floating decorations (verified 2026-03-29)
- [x] `PaneRenderCache`: only dirty panes re-prepared, clean panes cached (verified 2026-03-29)
- [x] Single-pane fast path: `compute_pane_layouts()` returns `None`, zero overhead (verified 2026-03-29)
- [x] `./build-all.sh` — compiles (verified 2026-03-29)
- [x] `./clippy-all.sh` — no warnings (verified 2026-03-29)
- [x] `./test-all.sh` — all tests pass, no regressions (verified 2026-03-29)
- [x] No `unwrap()` in library code, no dead code, no unsafe (verified 2026-03-29)
- [x] Crate boundaries respected: InProcessMux in oriterm_mux, session in oriterm, rendering in oriterm/gpu (verified 2026-03-29)

**Total section-31-related tests:** 71 (31 in_process + 6 mux_pump + 17 pane_cache + 6 prepare + 8 pane_config + 3 scratch) (verified 2026-03-29)

**Hygiene findings (verified 2026-03-29):**
- [ ] `redraw/multi_pane.rs`: inline `mod tests { }` should be sibling `tests.rs` (3 trivial tests)
- [ ] `redraw/multi_pane.rs`: 505 lines (484 prod + 21 test) — extracting tests to sibling would fix both issues

**Exit Criteria:** The mux layer is fully wired into the App. Multiple panes render correctly with proper offsets, dividers, and focus borders. Cached rendering prevents unnecessary GPU work. The single-pane case has zero overhead. All existing functionality works unchanged.
