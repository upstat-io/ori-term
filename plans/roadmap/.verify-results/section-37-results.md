# Section 37: TUI Client -- Verification Results

**Verified:** 2026-03-29
**Status in plan:** not-started
**Actual status:** CONFIRMED NOT STARTED

---

## Codebase Search Evidence

### 37.1 `oriterm-tui` Binary + Crate

| Search | Result |
|--------|--------|
| `oriterm_tui` directory | **Does not exist** |
| `oriterm-tui` in workspace Cargo.toml | **Not found** -- workspace members are: `oriterm_core`, `oriterm`, `oriterm_ui`, `oriterm_ipc`, `oriterm_mux` |
| `crossterm` dependency | **Not found** in any Cargo.toml |
| `ratatui` dependency | **Not found** in any Cargo.toml |
| TUI-related code anywhere | `oriterm_ui/src/layout/tests.rs` mentions "TUI" in one comment, but this is the GUI widget layout tests, not a TUI client |

**Verdict:** Truly not started. No crate, no directory, no dependencies.

### 37.2 TUI Rendering Engine

| Search | Result |
|--------|--------|
| `TuiRenderer` / `BufWriter<Stdout>` | **Not found** |
| Synchronized output (Mode 2026) references | EXISTS in `oriterm_core` -- `TermMode::SYNC_UPDATE` and `NamedPrivateMode::SyncUpdate` are implemented for the *terminal emulation* side (handling sequences from hosted apps), but no code exists to *emit* sync output as a TUI client |
| Diff rendering for TUI | **Not found** |
| Box-drawing characters for splits | **Not found** in any TUI context |

**Verdict:** Truly not started.

### 37.3 Input Routing + Prefix Key

| Search | Result |
|--------|--------|
| Prefix key handling | **Not found** |
| `InputHandler` for TUI | **Not found** |
| tmux-style keybindings | **Not found** |

**Verdict:** Truly not started.

### 37.4 Attach / Detach / Session Management

| Search | Result |
|--------|--------|
| `TuiApp` / `oriterm-tui attach` | **Not found** |
| Detach flow / `TuiCleanup` | **Not found** |
| Session listing (`SESSION WINDOWS PANES`) | **Not found** |

**Verdict:** Truly not started.

---

## Infrastructure Available from Other Sections

The following already-built infrastructure would support the TUI client:

1. **MuxClient** (`oriterm_mux/src/backend/client/`) -- the IPC client is fully built. A TUI client would use `MuxClient` identically to how the GUI binary uses it. Connect, subscribe to panes, receive push notifications, send input.

2. **MuxBackend trait** -- the TUI client would operate through the same `MuxBackend` interface as the GUI.

3. **PaneSnapshot** (`oriterm_mux/src/protocol/snapshot.rs`) -- wire types for cell data, cursor state, search matches, selections. The TUI renderer would translate these to escape sequences.

4. **Push-based snapshot delivery** -- the TUI client would receive `NotifyPaneSnapshot` pushes just like the GUI, with `CAP_SNAPSHOT_PUSH` capability.

5. **ensure_daemon()** (`oriterm_mux/src/discovery/`) -- auto-start logic works for any client binary, not just the GUI.

6. **RenderableContent** (`oriterm_core/src/term/renderable/`) -- the cell-by-cell content model with resolved colors, flags, cursor state. The TUI renderer would iterate this and emit SGR sequences.

7. **Color resolution** -- `Palette::resolve()` already handles indexed/named/spec color resolution. The TUI renderer would need an additional layer to downgrade truecolor to 256/16 for the host terminal.

---

## Gap Analysis

### Plan Completeness

The plan is well-designed with 5 subsections. It correctly identifies all major components needed for a TUI multiplexer client.

### Issues Found

1. **Dependency on Section 36**: Plan says "Dependencies: Section 36 (remote attach + network transport), Section 34 (wire protocol)". For *local* TUI client functionality, Section 36 is not required. The TUI client can connect to a local daemon using the existing IPC infrastructure from Section 44. Remote attach is a bonus, not a prerequisite. The plan even acknowledges this: "Can also connect locally via Section 34."

2. **No `ratatui` decision is correct**: The plan says "No ratatui -- raw crossterm for maximum control." This is the right call for a terminal-in-terminal renderer that needs cell-by-cell control.

3. **Edition "2024" in example Cargo.toml**: The workspace uses `edition = "2024"` which is correct for the current Rust edition.

4. **Color downgrade logic**: The plan mentions truecolor -> 256 -> 16 mapping but doesn't detail the algorithm. `termenv` (Go) and `crossterm` have nearest-color algorithms that could be referenced.

5. **Copy mode marked as "stretch"**: This is realistic. Copy mode (vi-like scrollback navigation) is complex and can be deferred.

6. **Multiple clients on same session**: The plan correctly notes that the daemon already supports multiple subscribers. Each TUI client would be an independent MuxClient with its own viewport.

### Missing Items

1. **No mention of `oriterm_core` cell types**: The TUI renderer needs to map `CellFlags` (bold, italic, underline styles, etc.) to SGR escape sequences. This is a non-trivial mapping, especially for extended underline styles (curly, dotted, dashed) which not all host terminals support.

2. **No mention of image handling**: If a pane contains sixel/Kitty/iTerm2 images, the TUI client either needs to pass them through (if host terminal supports the same protocol) or skip them. This edge case should be documented.

3. **No mention of wide character handling**: CJK characters in pane content need correct width accounting in the TUI renderer to avoid column misalignment.

4. **Resize propagation**: When the host terminal resizes (SIGWINCH), the TUI client needs to resize all pane viewports *and* the tab bar/status bar layout. The plan mentions this but doesn't detail how the split tree geometry maps to the available terminal dimensions.

---

## Recommendation

1. Relax the Section 36 dependency -- local TUI client should only depend on Section 44 (already complete). Remote attach can be added incrementally.
2. Add a subsection or checklist item for SGR attribute mapping (CellFlags -> escape sequences).
3. Document the image passthrough/skip strategy.
4. This is a realistic section. The scope is appropriate for a single section because the heavy lifting (wire protocol, daemon, push delivery) is already done.
