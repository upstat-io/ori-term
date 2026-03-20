---
section: "06"
title: "Dialog Rendering Integration"
status: not-started
reviewed: true
goal: "Wire the incremental rendering pipeline into the dialog rendering path, replacing the current full-repaint compose_dialog_widgets with the new selective prepare → selective prepaint → retained scene → GPU composite pipeline"
depends_on: ["01", "02", "03", "04", "05"]
sections:
  - id: "06.1"
    title: "Dialog Render Pipeline Restructure"
    status: not-started
  - id: "06.2"
    title: "Event Handler Updates"
    status: not-started
  - id: "06.3"
    title: "Main Window Integration"
    status: not-started
  - id: "06.4"
    title: "Completion Checklist"
    status: not-started
---

# Section 06: Dialog Rendering Integration

**Status:** Not Started
**Goal:** The dialog rendering path uses the incremental pipeline: selective tree walks, viewport culling, retained scene, and GPU-side scroll. `compose_dialog_widgets()` no longer clears the scene and repaints everything; it patches dirty regions and composites.

**Context:** The dialog rendering pipeline (`App::compose_dialog_widgets` in `dialog_rendering.rs`) currently follows the immediate-mode pattern: `scene.clear()` -> phase gate on `max_dirty_kind` -> `prepare(chrome)` + `prepare(content)` -> `prepaint(chrome)` + `prepaint(content)` -> `chrome.paint()` + `content.paint()` -> `damage.compute_damage()` -> `append_ui_scene_with_text()`. This section replaces it with the incremental pipeline built in Sections 01-05.

The main terminal window rendering (`redraw/mod.rs`) will also benefit from these changes but is not the primary target — the dialog is the immediate pain point. Main window integration is included here as a secondary goal.

**Depends on:** All of Sections 01-05.

---

## 06.1 Dialog Render Pipeline Restructure

**File(s):** `oriterm/src/app/dialog_rendering.rs` (will need splitting -- currently 259 lines, the incremental pipeline + fallback will exceed 500 lines)

**WARNING: File size limit.** The current file is 259 lines. Adding the incremental pipeline alongside the existing immediate-mode fallback will push past 500 lines. Plan to split: `dialog_rendering/mod.rs` (render_dialog, render_dialog_overlays, shared helpers) + `dialog_rendering/incremental.rs` (compose_dialog_widgets_incremental, fragment assembly).

Replace `compose_dialog_widgets()` with the incremental pipeline.

- [ ] New pipeline structure:
  ```rust
  fn compose_dialog_widgets_incremental(
      ctx: &mut DialogWindowContext,
      ui_theme: &UiTheme,
      scale: f32,
      logical_w: f32,
      logical_h: f32,
      gpu: &GpuState,
  ) {
      // 1. Phase gating (existing pattern — reads max_dirty_kind + lifecycle events)
      let widget_dirty = ctx.root.invalidation().max_dirty_kind()
          .merge(/* lifecycle events, ui_stale */);

      // 2. Selective prepare (Section 02 — pass dirty_set)
      if widget_dirty >= DirtyKind::Prepaint {
          let (interaction, frame_requests) = ctx.root.interaction_mut_and_frame_requests();
          prepare_widget_tree(&mut ctx.chrome, interaction, ..., Some(dirty_set));
          prepare_widget_tree(ctx.content.content_widget_mut(), interaction, ..., Some(dirty_set));
      }

      // 3. Selective prepaint (Section 02 — pass dirty_set)
      if widget_dirty >= DirtyKind::Prepaint {
          prepaint_widget_tree(&mut ctx.chrome, ..., Some(dirty_set));
          prepaint_widget_tree(ctx.content.content_widget_mut(), ..., Some(dirty_set));
      }

      // 4. Retained scene assembly (Section 04)
      if widget_dirty >= DirtyKind::Paint {
          // Paint only dirty widgets into their fragments
          paint_dirty_widgets(content, &dirty_set, &mut fragment_cache);
          // Assemble scene from fragments (cached + fresh)
          assemble_scene(&mut ctx.scene, &fragment_cache, paint_order);
      }
      // Else: scene is unchanged from last frame — skip to GPU submit

      // 5. Damage tracking + GPU conversion
      ctx.root.damage_mut().compute_damage(&ctx.scene);
      let renderer = ctx.renderer.as_mut().expect("...");
      renderer.append_ui_scene_with_text(&ctx.scene, scale, 1.0, gpu);

      // 6. Clean up
      ctx.root.invalidation_mut().clear();
  }
  ```

- [ ] Keep the old `compose_dialog_widgets` as a fallback for the first frame or large structural changes (>50% widgets dirty)

- [ ] Ensure chrome rendering still works (chrome is typically static — always reuse cached fragment)

- [ ] **Chrome and content are separate widget trees.** `ctx.chrome` (WindowChromeWidget) and `ctx.content.content_widget()` (SettingsPanel or DialogWidget) are NOT rooted in `ctx.root.widget()`. `WindowRoot` owns a placeholder `LabelWidget("")` — it's used only for framework state (interaction, focus, overlays), not as the actual widget tree root. The incremental pipeline must handle chrome and content independently:
  - Chrome: always-cached fragment (title bar is static except on window resize)
  - Content: selective walk + fragment caching per the plan
  - They share the same `InvalidationTracker` (via `ctx.root`) and `InteractionManager`

- [ ] **Consider unifying the widget tree.** The current architecture has chrome and content as independent trees, which complicates every pipeline step (two `prepare_widget_tree` calls, two `prepaint_widget_tree` calls, two `paint` calls). Consider making `WindowRoot.widget` the actual root of a tree that contains both chrome and content. This is a larger refactor but would eliminate duplication and let `WindowRoot.compute_layout()`, `WindowRoot.prepare()`, and `WindowRoot.paint()` handle everything. **Recommendation: out of scope for this plan, but document as future improvement.**

---

## 06.1b Overlay Rendering Path

**File(s):** `oriterm/src/app/dialog_rendering.rs` (`render_dialog_overlays`)

The overlay rendering path (`render_dialog_overlays`) clears the scene for each overlay, paints it, and appends with its own opacity. This path must remain independent from the incremental content pipeline:

- [ ] Overlays continue to use immediate-mode rendering (scene.clear() + paint per overlay). Overlays are transient, small (one dropdown list), and rarely visible — caching their fragments provides negligible benefit.
- [ ] Ensure overlay rendering does NOT interfere with the retained content scene. The current code already uses a separate scene (it clears `ctx.scene` per overlay, which is the scratch scene on `DialogWindowContext`). If the retained scene is also stored on `DialogWindowContext`, ensure the overlay path uses a separate scratch scene, not the retained one.
- [ ] Add a `scratch_scene: Scene` field to `DialogWindowContext` if the main `scene` field becomes the retained scene. Overlays use the scratch scene; content uses the retained scene.

---

## 06.2 Event Handler Updates

**File(s):** `oriterm/src/app/dialog_context/event_handling/mouse.rs`, `oriterm/src/app/dialog_context/event_handling/mod.rs`, `oriterm/src/app/dialog_context/content_actions.rs`

Update event handlers to use granular invalidation instead of global dirty marking.

- [ ] `handle_dialog_scroll`: Mark only the ScrollWidget as dirty (Paint level), not the entire dialog
- [ ] `handle_dialog_cursor_move` / `dispatch_dialog_content_move`: Mark only the hovered widget as dirty (Prepaint level). **Critical:** preserve the existing action routing and `apply_dispatch_requests` — `dispatch_dialog_content_move` now routes `result.actions` (e.g., slider `DragUpdate` → `ValueChanged`) through `handle_dialog_content_action` and applies `SET_ACTIVE`/`CLEAR_ACTIVE` requests for drag capture. The incremental pipeline must not regress this.
- [ ] `dispatch_dialog_settings_action`: Mark the affected widget subtree as dirty (Layout level for page switch)
- [ ] `handle_dialog_keyboard`: Mark the focused widget as dirty (Prepaint level)

- [ ] Remove `invalidation_mut().invalidate_all()` calls — replace with targeted `mark_widget_dirty(id, level)`

**Invariant:** All three dialog event dispatch paths (click, move, scroll) must:
1. Dispatch the event via `deliver_event_to_tree` with the correct `active` widget
2. Call `apply_dispatch_requests` to update interaction state (active, focus)
3. Route `result.actions` to `handle_dialog_content_action` (not drop them)
4. Request urgent redraw when PAINT requested

This was a bug (MouseMove dropped actions, breaking slider drag) and must not regress during the incremental pipeline integration.

---

## 06.3 Main Window Integration

**File(s):** `oriterm/src/app/redraw/mod.rs`

Apply the same incremental pipeline to the main terminal window's UI rendering (tab bar, search overlay, etc.). The terminal grid rendering already uses damage tracking; this extends it to the UI widgets.

- [ ] Tab bar: retained fragment, repaint only on tab add/remove/rename/reorder
- [ ] Search overlay: retained fragment, repaint only on query change or result navigation
- [ ] Settings overlay (non-dialog mode): same incremental pipeline as dialog

---

## 06.4 Completion Checklist

- [ ] `compose_dialog_widgets` uses selective prepare/prepaint
- [ ] Scene is assembled from retained fragments, not rebuilt from scratch
- [ ] Event handlers use targeted invalidation
- [ ] Chrome is rendered once and cached (static content)
- [ ] Chrome and content handled as separate widget trees (not unified through WindowRoot)
- [ ] Scroll triggers only viewport culling + strip paint (no full repaint)
- [ ] Overlay rendering path preserved (immediate-mode, independent from retained scene)
- [ ] Scratch scene separated from retained scene for overlay rendering
- [ ] `prepaint_bounds` populated from layout tree (Section 03.3b bug fix applied)
- [ ] Main window UI uses retained fragments for tab bar and overlays
- [ ] No regressions — `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green

**Exit Criteria:** Opening the settings dialog and scrolling the Colors page produces <4ms frame time. Hovering a setting row produces <2ms frame time. Idle dialog with no interaction produces 0ms frame time (no GPU work).
