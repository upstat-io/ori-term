---
reroute: true
name: "Threaded IO"
full_name: "Threaded Terminal IO"
status: active
order: 1
---

# Threaded Terminal IO — Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Terminal IO Thread Scaffold
**File:** `section-01-io-thread-scaffold.md` | **Status:** Complete

```
PaneIoThread, PaneIoCommand, io_thread, terminal_io
thread::spawn, JoinHandle, channel, crossbeam-channel, crossbeam_channel::select!
command enum, Resize, ScrollDisplay, SetTheme, Shutdown
pane/io_thread/mod.rs, pane/io_thread/commands.rs
message loop, select, drain commands, process bytes
oriterm_mux, pane lifecycle, thread ownership
```

---

### Section 02: VTE Parsing Migration
**File:** `section-02-vte-migration.md` | **Status:** Complete

```
VTE, vte::ansi::Processor, vte::Parser, parse_chunk
PtyEventLoop, pty/event_loop/mod.rs, reader thread
byte forwarding, raw bytes, READ_BUFFER_SIZE, MAX_LOCKED_PARSE
RawInterceptor, OSC 7, OSC 133, shell integration
FairMutex, lease, try_lock, lock_unfair, removal
Term, Handler, EventListener, MuxEventProxy
IoThreadEventProxy, suppress_metadata, grid_dirty, dual-Term
event_proxy.rs, event_proxy/mod.rs, event_proxy/tests.rs
```

---

### Section 03: Snapshot Production & Transfer
**File:** `section-03-snapshot-production.md` | **Status:** Complete

```
RenderableContent, renderable_content_into, snapshot
shared buffer, Mutex swap, ArcSwap, triple buffer
parking_lot::Mutex, Option<RenderableContent>
zero allocation, buffer reuse, maybe_shrink
wakeup, grid_dirty, wakeup_pending, MuxEventProxy
snapshot production, publish, swap_renderable_content
pane/io_thread/snapshot.rs
```

---

### Section 04: Render Pipeline Migration
**File:** `section-04-render-migration.md` | **Status:** Complete

```
refresh_pane_snapshot, build_snapshot_into, render path
EmbeddedMux, MuxBackend, swap_renderable_content
redraw/mod.rs, handle_redraw, extract phase
pane.terminal().lock(), remove lock, snapshot buffer
content_changed, snap_dirty, pane_changed
SnapshotCache, render_buf, renderable_cache
```

---

### Section 05: Resize Pipeline Migration
**File:** `section-05-resize-migration.md` | **Status:** Not Started

```
resize, handle_resize, sync_grid_layout, WindowEvent::Resized
resize_pane_grid, resize_grid, resize_pty, SIGWINCH
Grid::resize, reflow, reflow_cols, reflow_cells, finalize_resize
coalesce, debounce, latest size, batch resize
display_offset reset, cursor jump, text flash, BUG-06.2
PaneIoCommand::Resize, async resize, IO thread
chrome/resize.rs, resize_all_panes
```

---

### Section 06: Remaining State Operations
**File:** `section-06-remaining-ops.md` | **Status:** Not Started

```
scroll_display, scroll_to_bottom, display_offset
set_theme, set_cursor_shape, mark_all_dirty
set_image_config, image protocol, animation
extract_text, extract_html, clipboard, selection
search, open_search, close_search, set_query, next_match
oneshot, reply channel, command-response
mark_cursor, enter_mark_mode, exit_mark_mode
scroll_to_previous_prompt, scroll_to_next_prompt
server/dispatch, daemon dispatch, selection_dirty, command_output_selection
```

---

### Section 07: Pane Lifecycle & FairMutex Removal
**File:** `section-07-lifecycle-cleanup.md` | **Status:** Not Started

```
Pane, PaneParts, from_parts, Pane::new
Arc<FairMutex<Term<MuxEventProxy>>>, removal
terminal(), pub accessor, remove
PtyControl, move to IO thread
shutdown, join, graceful, Msg::Shutdown
cleanup_closed_pane, background drop
FairMutex, sync/mod.rs, deprecate
IoThreadEventProxy, suppress_metadata, metadata cutover
daemon mode, DaemonMux, snapshot path
```

---

### Section 08: Verification
**File:** `section-08-verification.md` | **Status:** Not Started

```
alloc_regression.rs, zero allocation, renderable_content_into
compute_control_flow, idle CPU, ControlFlow::Wait
stable RSS, scrollback bounded, image eviction
cross-platform, Windows, macOS, Linux, ConPTY
stress test, flood output, rapid resize, multi-pane
BUG-06.2, garbled text, key repeat, resize race
test-all.sh, build-all.sh, clippy-all.sh
performance invariants, regression
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Terminal IO Thread Scaffold | `section-01-io-thread-scaffold.md` |
| 02 | VTE Parsing Migration | `section-02-vte-migration.md` |
| 03 | Snapshot Production & Transfer | `section-03-snapshot-production.md` |
| 04 | Render Pipeline Migration | `section-04-render-migration.md` |
| 05 | Resize Pipeline Migration | `section-05-resize-migration.md` |
| 06 | Remaining State Operations | `section-06-remaining-ops.md` |
| 07 | Pane Lifecycle & FairMutex Removal | `section-07-lifecycle-cleanup.md` |
| 08 | Verification | `section-08-verification.md` |
