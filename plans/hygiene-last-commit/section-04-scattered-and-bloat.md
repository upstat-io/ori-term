---
section: "04"
title: "Scattered Knowledge and Bloat"
status: not-started
reviewed: false
goal: "Consolidate scattered theme application, fix blinking_active duplication, resolve BLOAT files and dead code"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-04-03
sections:
  - id: "04.1"
    title: "Consolidate Theme Application"
    status: not-started
  - id: "04.2"
    title: "Consolidate blinking_active Formula"
    status: not-started
  - id: "04.3"
    title: "BLOAT File Splits"
    status: not-started
  - id: "04.4"
    title: "Dead Code and Stale Attributes"
    status: not-started
  - id: "04.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "04.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: Scattered Knowledge and Bloat

**Status:** Not Started
**Goal:** Consolidate scattered theme application into one canonical method, unify the `blinking_active` computation, split BLOAT files, and clean up dead code.

**Context:** Theme application to chrome widgets (tab_bar + status_bar + cache invalidation) is copy-pasted at 3 sites. The `blinking_active` formula is computed identically at 2 sites via different data paths. Four source files exceed the 500-line limit.

---

## 04.1 Consolidate Theme Application

**File(s):** `oriterm/src/app/mod.rs`, `oriterm/src/app/keyboard_input/overlay_dispatch.rs`, `oriterm/src/app/config_reload/mod.rs`

Theme application (tab_bar.apply_theme + status_bar.apply_theme + cache invalidation) appears at 3 call sites:
- `mod.rs:372-380` (handle_theme_changed)
- `overlay_dispatch.rs:138-141` (theme toggle action)
- `config_reload/mod.rs:112-115` (config reload)

- [ ] Extract a method on `App`:
  ```rust
  /// Apply the current UI theme to all window chrome widgets and invalidate caches.
  fn apply_theme_to_all_windows(&mut self) {
      for ctx in self.windows.values_mut() {
          ctx.tab_bar.apply_theme(&self.ui_theme);
          ctx.status_bar.apply_theme(&self.ui_theme);
          ctx.pane_cache.invalidate_all();
          ctx.text_cache.clear();
          ctx.root.invalidation_mut().invalidate_all();
          ctx.root.damage_mut().reset();
          ctx.root.mark_dirty();
      }
  }
  ```
- [ ] Replace all 3 inline loops with `self.apply_theme_to_all_windows()`.
- [ ] Note: `handle_theme_changed` in mod.rs:372 does exactly this set of operations. The other two sites may have slight variations (e.g., config_reload may do additional work). Verify each site's exact loop body matches before extracting.

---

## 04.2 Consolidate blinking_active Formula

**File(s):** `oriterm/src/app/event_loop.rs`, `oriterm/src/app/redraw/post_render.rs`

The `blinking_active` derivation appears at 2 sites with the same formula:
- `event_loop.rs:140-143`: `self.blinking_active = self.config.terminal.cursor_blink && self.terminal_mode().is_some_and(|m| m.contains(TermMode::CURSOR_BLINKING))`
- `post_render.rs:49`: `self.blinking_active = self.config.terminal.cursor_blink && blinking_now` (where `blinking_now` is `frame.content.mode.contains(TermMode::CURSOR_BLINKING)`)

Same formula, different data access paths (live mode query vs. extracted frame mode).

- [ ] Evaluate whether these can be unified. The Focused(true) handler (event_loop.rs:140) needs to set blinking_active immediately (before any frame is rendered). The post_render path (post_render.rs:49) updates it from the freshest frame data. Both are needed for responsiveness.
- [ ] If unification isn't possible (different data sources at different times), add a cross-reference comment in both locations explaining that the formula is intentionally duplicated and WHY (focus handler uses live query, post_render uses frame snapshot).
- [ ] Consider extracting a named method:
  ```rust
  fn should_blink(&self, terminal_blinking: bool) -> bool {
      self.config.terminal.cursor_blink && terminal_blinking
  }
  ```
  And call it from both sites with the appropriate source of `terminal_blinking`.

---

## 04.3 BLOAT File Splits

**File(s):** Multiple files exceeding 500-line limit.

| File | Lines | Action |
|------|-------|--------|
| `config_reload/mod.rs` | 518 | Split — extract font/color/rendering config reload into submodules |
| `window_management.rs` | 503 | Split — extract window creation helpers into submodule |
| `tab_bar_input/mod.rs` | 502 | Split — extract tab drag initiation into submodule |
| `event_loop.rs` | 498 | Monitor — under limit but close. Chrome extraction in 01.1 may push it over; check after. |

- [ ] Split `config_reload/mod.rs`: it already has submodules (`color_config.rs`, `font_config.rs`). Move more helpers into these or create new ones to get under 500.
- [ ] Split `window_management.rs`: extract window creation / multi-window helpers.
- [ ] Split `tab_bar_input/mod.rs`: extract tab drag initiation.
- [ ] After Section 01 changes, re-check `event_loop.rs` line count.

---

## 04.4 Dead Code and Stale Attributes

**File(s):** Various.

1. **Stale `#[allow(dead_code)]` on `move_tab_to_new_window`** (`tab_management/move_ops.rs:66`): This function IS called from `event_loop.rs:345` (TermEvent::MoveTabToNewWindow). The `dead_code` allow with reason "superseded by tear_off_tab" is stale — the function is live code.
   - [ ] Remove the `#[allow(dead_code)]` attribute from `move_tab_to_new_window`.

2. **Pre-built dead fields** (`window_context.rs:84-92`): `render_strategy` and `damage` fields have `#[expect(dead_code)]` with reason "vocabulary for retained-ui plan". This is acceptable given the explicit plan reference — no action needed unless the retained-ui plan is abandoned.
   - [ ] No action (informational — tracked for awareness).

3. **`widget_pipeline` module dead_code allow** (`mod.rs:43-47`): The `widget_pipeline` module has a broad `#[allow(dead_code)]` with reason "incremental pipeline — delivery loop wired in OverlayManager migration".
   - [ ] Verify whether any code in `widget_pipeline/` is now used. If still entirely dead, leave as-is with the existing annotation. If partially used, remove the blanket allow and apply targeted allows.

---

## 04.R Third Party Review Findings

- [x] `[TPR-04-001][low]` `.claude/rules/code-hygiene.md:89-94`, `oriterm/src/gpu/frame_input/mod.rs:404-418`, `plans/hygiene-last-commit/section-01-redraw-pipeline.md:95-96` — Section 01 touches `oriterm/src/gpu/frame_input/mod.rs` to add `FrameInput::clear_transient_fields()`, but the file now measures 531 lines, exceeding the repo's hard 500-line limit for touched source files. Section 04's BLOAT inventory does not currently track this file, so the disposable plan understates the cleanup still required for the current worktree.

---

## 04.N Completion Checklist

- [ ] Theme application exists in exactly one method, called from all 3 sites
- [ ] `blinking_active` formula is either unified or cross-referenced
- [ ] All source files under 500 lines (test files exempt)
- [ ] Stale `dead_code` allow removed from `move_tab_to_new_window`
- [ ] `./test-all.sh` green
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** Theme application has one canonical home. No source file exceeds 500 lines. All dead_code attributes are accurate (no stale allows on live code).
