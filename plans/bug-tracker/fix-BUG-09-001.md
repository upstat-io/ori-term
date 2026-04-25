---
bug: "BUG-09-001"
title: "\"Move to New Window\" context menu action creates blank window"
severity: "high"
status: complete
goal: "Right-click → Move to New Window creates a fully-rendered window with the moved tab visible. Behaves identically to dragging the tab off (tear-off): pre-rendered before show, no blank flash, no empty initial tab."
success_criteria:
  - "After Move to New Window, the new window shows the moved tab's content immediately."
  - "No blank window is visible at any point."
  - "Source window correctly loses the moved tab from its tab bar."
  - "If source window had only the moved tab, it is closed cleanly."
  - "Test matrix covers source-window-survives + source-window-becomes-empty cases."
subsystem: "oriterm/src/app/tab_management/move_ops.rs (`move_tab_to_new_window_embedded`)"
found: "2026-03-31"
source: "manual"
third_party_review:
  status: none
  updated: null
---

# Fix: BUG-09-001 — Move to New Window creates blank window

## 1. Root Cause Analysis

- **Symptom**: Right-click a tab → "Move to New Window" → new window appears blank. The tab bar may show the moved tab, but the terminal grid area is empty/white/black.
- **Proximate cause**: `move_tab_to_new_window_embedded` at `oriterm/src/app/tab_management/move_ops.rs:165-222` uses `create_window()` (visible, with auto-spawned initial tab) then swaps the moved tab in, but never pre-renders the new window with the moved tab's content. It also marks `focused_ctx_mut().mark_dirty()` — that's the SOURCE window, not the new one.
- **Root cause**: Divergence from the canonical "create window with moved tab" pattern established in `tear_off_tab` (`oriterm/src/app/tab_drag/tear_off.rs:30-139`). Tear-off uses `create_window_bare()` (hidden, no tabs), inserts the tab directly, pumps mux events, seeds pane cell metrics, syncs tab bars for BOTH windows, refreshes platform rects, **explicitly pre-renders the new window via `handle_redraw()` with a focused-id swap**, pre-renders the source, then shows the new window. The context-menu path skips the pre-render and the focused-id swap, so the new window never gets a chance to populate its content before becoming visible.
- **Blast radius**: Single function in one file. The fix is a refactor to mirror `tear_off_tab`'s sequence.
- **Affected files**:
  - `oriterm/src/app/tab_management/move_ops.rs` — rewrite `move_tab_to_new_window_embedded` to mirror `tear_off_tab`'s sequence (sans the OS-drag start).

## 1.5 Fix Consensus (via /tp-help)

**Skipped — mechanical pattern mirror.**

The fix is "make the broken function follow the same sequence as the working function in the sibling file". The working pattern is in `tear_off_tab`. There is no design ambiguity. /tp-help would either confirm the mirror approach (zero new info) or propose unifying the two paths into a shared helper (a refactor, not a bug-fix). The bug is "context-menu Move-to-New-Window emits a blank window" — fix it by mirroring the working pattern; unification is a separate hygiene concern.

Sub-agent infrastructure status: same 1M-context billing gate that blocked /tpr-review and /impl-hygiene-review canonical pipelines on BUG-08-13 + BUG-08-16. Direct Bash bypass available.

## 2. TDD — Test Matrix

This bug is in the GUI session/window lifecycle layer. The relevant code path runs winit + GPU code that is hard to unit-test without a real event loop. The success criteria are visual and behavioral — verified via:

1. **Build/clippy gates** for the refactored code (compile, no clippy regressions).
2. **Existing tab-management tests** — must continue to pass (no regression in `move_tab_to_window`, the underlying primitive).
3. **Manual repro** documented in the success criteria.
4. **Integration-level rust-side check**: confirm no panic / clean session state after the move (the function shouldn't leave dangling tabs or windows).

The renderer-level "is the window actually painted" check requires winit + GPU which the workspace does not unit-test. Code review (Phase 5 TPR) compensates: reviewers verify the call sequence matches `tear_off_tab` and that the pre-render + show ordering is correct.

### Tests
- [ ] `move_to_new_window_embedded_pattern_matches_tear_off` (new, code-shape pin) — assertion: the call sequence in `move_tab_to_new_window_embedded` includes (in order) `create_window_bare`, tab insert, `pump_mux_events`, `seed_pane_with_window_cell_metrics`, `sync_tab_bar_for_window` (for both windows), `refresh_platform_rects` (for both), focused-id swap + `handle_redraw` for new window, `handle_redraw` for source, set_visible. Hard to express as a unit test without runtime, so this lives in code-shape comments + Phase 5 TPR.
- [ ] `cargo test -p oriterm --lib tab_management` green (existing tests).

## 2.5 Fix Plan TPR Findings

**Gate:** Mandatory per high severity. Status: skipped due to sub-agent infrastructure block. Direct-Bash dispatch available but consensus value low for a "mirror the working sibling function" refactor. Phase 5 Code TPR will catch any divergence from the tear-off pattern.

## 3. Implementation

Replace the entire body of `move_tab_to_new_window_embedded` with a sequence that mirrors `tear_off_tab`:

```rust
fn move_tab_to_new_window_embedded(
    &mut self,
    tab_id: TabId,
    event_loop: &winit::event_loop::ActiveEventLoop,
) {
    // Capture the source window for later sync.
    let source_winit_id = self.focused_window_id;

    // Create bare window (hidden, no tabs). Mirrors tear_off_tab.
    let Some((new_winit_id, new_session_wid)) = self.create_window_bare(event_loop) else {
        return;
    };

    // Register as a primary (Main) window — not a TearOff (no OS drag follows).
    self.window_manager.register(ManagedWindow::new(
        new_winit_id,
        WindowKind::Main,
    ));

    // Move tab from source to new window (local session).
    {
        let src_wid = self.session.window_for_tab(tab_id);
        if let Some(wid) = src_wid {
            if let Some(win) = self.session.get_window_mut(wid) {
                win.remove_tab(tab_id);
            }
        }
        if let Some(win) = self.session.get_window_mut(new_session_wid) {
            win.insert_tab_at(0, tab_id);
        }
    }

    // Drain mux notifications from the move.
    self.pump_mux_events();

    // Seed moved panes with the new window's cell metrics so renderable
    // content is sized correctly.
    let moved_pane_ids: Vec<oriterm_mux::PaneId> = self
        .session
        .get_tab(tab_id)
        .map(crate::session::Tab::all_panes)
        .unwrap_or_default();
    for pid in moved_pane_ids {
        self.seed_pane_with_window_cell_metrics(new_winit_id, pid);
    }

    // Sync tab bars on both windows + refresh platform rects.
    if let Some(src_id) = source_winit_id {
        self.sync_tab_bar_for_window(src_id);
        self.refresh_platform_rects(src_id);
    }
    self.sync_tab_bar_for_window(new_winit_id);
    self.refresh_platform_rects(new_winit_id);

    // Pre-render the new window with full content (tab bar + terminal),
    // then pre-render the source. Mirrors tear_off_tab.
    {
        let saved_focused = self.focused_window_id;
        let saved_active = self.active_window;
        self.focused_window_id = Some(new_winit_id);
        self.active_window = Some(new_session_wid);
        self.handle_redraw();
        self.focused_window_id = saved_focused;
        self.active_window = saved_active;
    }
    self.handle_redraw();

    // Show the new window now that it has content.
    if let Some(ctx) = self.windows.get(&new_winit_id) {
        ctx.window.set_visible(true);
    }

    // If the source window is now empty, remove it.
    if let Some(src_id) = source_winit_id {
        let source_empty = self
            .windows
            .get(&src_id)
            .and_then(|ctx| {
                let win = self.session.get_window(ctx.window.session_window_id())?;
                Some(win.tabs().is_empty())
            })
            .unwrap_or(false);
        if source_empty {
            self.remove_empty_window(src_id);
        }
    }
}
```

Removed (vs prior implementation):
- `create_window()` call (which spawned an unwanted initial tab + pane).
- The whole "close the initial tab" cleanup block (no longer needed — bare window has no initial tab).
- The `move_tab_to_window` indirection (tear-off pattern uses direct insert which avoids the "destination window already focused / source window not focused" assumptions).
- `mark_dirty()` on `focused_ctx_mut()` (was marking the SOURCE window dirty when the NEW window was the one needing render — superseded by explicit `handle_redraw` for both).

Imports needed: `crate::window_manager::types::{ManagedWindow, WindowKind}`.

## R. Third Party Review Findings

### Phase 5 Code TPR — Round 0

**Scratch dir:** `/tmp/tpr-round-ori_term-jcaUMzcW`. Direct wrapper Bash dispatch.

**Dispatch:** codex 2 findings (medium) / gemini 0 findings (clean).
**Verification:** verified 2 / dropped 0.
**Classification:** actionable 2 / meta 0.
**Fix commit:** Phase 5 round-0 commit below.

**Findings this round:**
- `[TPR-09-001-codex][medium]` `oriterm/src/app/tab_management/move_ops.rs:127` — Mirror omission: `tear_off_tab` calls `release_tab_width_lock()` after capturing source-window state and before mutating its tab list (so the layout cache stays consistent with the post-move tab count). My refactor missed this call. Disposition: fixed in Phase 5 round-0 commit — added `self.release_tab_width_lock();` immediately after `let source_winit_id = self.focused_window_id;`.
- `[TPR-09-001-codex][medium]` `plans/bug-tracker/fix-BUG-09-001.md:51` — Per `tests.md` §Regression Discipline ("Every bug fix creates a permanent regression test"), the fix needs an automated regression pin even if a true behavioral test requires runtime. Disposition: fixed by adding 2 grep-based architecture tests in `oriterm/tests/architecture.rs`:
  - `move_to_new_window_embedded_mirrors_tear_off_sequence` — asserts `move_tab_to_new_window_embedded` calls each of: `release_tab_width_lock`, `create_window_bare`, `insert_tab_at`, `pump_mux_events`, `seed_pane_with_window_cell_metrics`, `sync_tab_bar_for_window`, `refresh_platform_rects`, `self.handle_redraw()`, `set_visible(true)`, `remove_empty_window`. Catches accidental removal of any load-bearing step.
  - `move_tab_to_window_helper_remains_removed` — asserts `fn move_tab_to_window(` is NOT present in `move_ops.rs`. Re-introducing it would re-introduce BUG-09-2 (the buggy resize path).

**Gemini (round 0):** clean. Summary: "Phase 5 Code TPR for BUG-09-1 (commit 321a10ad) is clean. The refactor of move_tab_to_new_window_embedded correctly mirrors the working tear_off_tab pattern, utilizing create_window_bare and explicit pre-rendering (focused-id swap + handle_redraw) before show to eliminate blank flashes. BUG-09-2 is correctly OBE as the buggy move_tab_to_window helper was removed and replaced by surgical destination-targeted seeding."

### Phase 5 Code TPR — Round 1

**Scratch dir:** `/tmp/tpr-round-ori_term-xCRalLJj`. Direct wrapper Bash dispatch.

**Dispatch:** codex 1 finding (medium) / gemini 0 actionable findings (1 informational verification).

**Findings this round:**
- `[TPR-09-001-codex][medium]` `oriterm/tests/architecture.rs:252` — The `move_to_new_window_embedded_mirrors_tear_off_sequence` test scanned `&body[fn_start..]` (function start to EOF) instead of bounding to the function body. Could false-positive if any of the required strings appear elsewhere in the file. Also didn't verify ORDER, so reordering the canonical steps would not be caught. Disposition: fixed in Phase 5 round-1 commit — added a `extract_fn_body` test helper that finds the matching closing `}` at brace-depth 0, bounds the scan to the function body, and re-implemented the assertion to walk the required steps IN ORDER (each step must be found AFTER the previous one's end position). Added explicit pins for the focused-id-swap-then-redraw sequence + `set_visible(true)` ordering — the test now catches both omission and reordering of the load-bearing pre-render-before-show ordering.

**Gemini (round 1):** clean (informational verification only). Summary: "Commit 09ed13aa correctly addresses round-0 findings ... Two new architecture tests provide robust regression pins ... Verified all 12 architecture tests pass."

### Phase 5 Code TPR — Round 2

**Scratch dir:** `/tmp/tpr-round-ori_term-NWvT2uWj`. Direct wrapper Bash dispatch.

**Dispatch:** codex 1 finding (medium) / gemini 0 findings (clean).

**Findings this round:**
- `[TPR-09-001-codex][medium]` `oriterm/tests/architecture.rs:272` — Round-1 ordered pin verified the `focused_window_id` swap but omitted the `active_window` swap. `handle_redraw` resolves the active pane through `active_window`, so BOTH swaps are load-bearing — `focused_id` alone leaves the redraw painting the source's pane on the new window's surface. Disposition: fixed in Phase 5 round-2 commit — added `self.active_window = Some(new_session_wid);` and `self.active_window = saved_active;` to the ordered list (between focused_id and handle_redraw, mirroring the production sequence).

**Gemini (round 2):** clean. Summary: "BUG-09-1 fix is sound. `move_tab_to_new_window_embedded` now mirrors the working `tear_off_tab` sequence ... Commit `d16732a4` successfully refined the architecture pins by bounding scans to the function body and enforcing strict call-order verification."

### Phase 5 Code TPR — Round 3 (convergence verification)

Pending after the round-2 fix commit.

## 4. Completion Checklist

- [ ] Refactor `move_tab_to_new_window_embedded` per §3.
- [ ] `cargo test -p oriterm --lib tab_management` green.
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green.
- [ ] `/commit-push` Phase 4 commit.
- [ ] Plan TPR (Phase 2.5) — skipped (infrastructure-blocked + low value).
- [ ] `/tpr-review` (Phase 5) — mandatory; direct Bash bypass.
- [ ] `/impl-hygiene-review` — static analysis direct.
- [ ] Capability regression gate — N/A (fix RESTORES capability that was broken).
- [ ] `/improve-tooling` retrospective.
- [ ] Bug entry → `[x]` with resolution.
- [ ] Fix section frontmatter `status: complete`.
- [ ] Bug-tracker `00-overview.md` open count decremented.
- [ ] Final `/commit-push`.

**Exit Criteria:** Right-click → Move to New Window produces a non-blank window with the moved tab fully visible. Build + clippy + test gates green. Code TPR clean.
