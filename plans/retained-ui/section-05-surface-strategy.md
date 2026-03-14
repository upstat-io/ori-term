---
section: "05"
title: "Surface Strategy Abstraction"
status: complete
goal: "A shared surface host contract distinguishes terminal surfaces, UI-only surfaces, and transient overlays — each with its own render strategy, damage tracking, and invalidation policy."
inspired_by:
  - "Chromium aura::Window + WindowTargeter strategy pattern"
  - "Ghostty Surface abstraction (src/Surface.zig)"
depends_on: []
sections:
  - id: "05.1"
    title: "RenderStrategy Enum"
    status: complete
  - id: "05.2"
    title: "DamageKind Enum"
    status: complete
  - id: "05.3"
    title: "SurfaceHost Trait"
    status: complete
  - id: "05.4"
    title: "Integration with WindowContext and DialogWindowContext"
    status: complete
  - id: "05.5"
    title: "Completion Checklist"
    status: complete
reviewed: true
---

# Section 05: Surface Strategy Abstraction

**Status:** Not Started
**Goal:** Terminal windows, dialog windows, and overlay layers share one host contract but differ by render strategy and invalidation policy. The framework decides dirtying, redraw urgency, and scene rebuilding — hosts stop manually poking ad-hoc flags.

**Context:** Today `WindowContext` (window_context.rs) and `DialogWindowContext` (`dialog_context/mod.rs`) are independent structs with duplicated fields: both have `dirty`, `urgent_redraw`, `renderer`, `overlays`, `layer_tree`. Their render paths are separate: `handle_redraw()` for terminal windows, `render_dialog()` for dialogs. The dirty/stale semantics differ subtly: terminal windows have `ui_stale` for chrome (and `chrome_draw_list` for tab bar/overlay draw output), while dialogs use a single `draw_list` and don't distinguish chrome from content.

This duplication means every new window type (about dialog, tooltip window, settings) must reimplement the same dirty tracking, overlay management, and render decision logic. The fix is a shared contract that each surface type implements, so the event loop can treat them uniformly.

**Reference implementations:**
- **Chromium** `ui/aura/window.cc`: All windows share `Window` base class with event targeting, bounds, transforms. `WindowDelegate` customizes behavior per-window-type.
- **Ghostty** `src/Surface.zig`: Abstracted surface handles input, rendering, and lifecycle uniformly.

**Depends on:** Nothing — pure abstraction addition. Does not change existing rendering behavior, just provides the vocabulary for Sections 02-04 to use.

---

## 05.1 RenderStrategy Enum

**File(s):** new `oriterm_ui/src/surface/mod.rs`

Define the rendering strategies that different surface types use.

- [x] Define `RenderStrategy`:
  ```rust
  /// How a surface renders its content.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum RenderStrategy {
      /// Terminal grid content with cached base + transient overlays.
      ///
      /// Optimizes for streamed content: the terminal grid is rendered
      /// into a cached texture, overlays (tab bar, search bar, popups)
      /// are drawn on top each frame. Full rebuild only when PTY output
      /// changes the grid.
      TerminalCached,

      /// Retained UI scene with selective subtree rebuild.
      ///
      /// Optimizes for interaction latency: widget tree is cached per-subtree,
      /// only dirty widgets rebuild their draw commands. Used for dialogs,
      /// settings, and future standalone UI windows.
      UiRetained,

      /// Transient scene — rebuilt every frame.
      ///
      /// Used for tooltips, drag previews, and other short-lived visuals
      /// where caching overhead exceeds the cost of full rebuild.
      Transient,
  }
  ```

---

## 05.2 DamageKind Enum

**File(s):** `oriterm_ui/src/surface/mod.rs`

Define damage categories that drive render decisions.

- [x] Define `DamageKind`:
  ```rust
  /// What kind of change requires a render pass.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum DamageKind {
      /// Layout changed (widget tree structure, sizes).
      Layout,
      /// Paint changed (colors, opacity, hover state).
      Paint,
      /// Overlay layer changed (popup open/close, tooltip).
      Overlay,
      /// Cursor blink or caret state changed.
      Cursor,
      /// Scroll position changed (transform-only update).
      ScrollTransform,
  }
  ```

- [x] `DamageKind` maps to render work:
  - `Layout` → relayout + repaint all affected subtrees + GPU upload
  - `Paint` → repaint dirty subtrees only + GPU upload
  - `Overlay` → rebuild overlay layer only
  - `Cursor` → cursor blink toggle only (cheapest — no content rebuild)
  - `ScrollTransform` → update transform uniform/commands only

---

## 05.3 SurfaceHost Trait

**File(s):** `oriterm_ui/src/surface/mod.rs`

A trait that both `WindowContext` and `DialogWindowContext` can implement.

- [x] Define `SurfaceHost`:
  ```rust
  /// Shared contract for any drawable surface.
  pub trait SurfaceHost {
      /// The rendering strategy this surface uses.
      fn render_strategy(&self) -> RenderStrategy;

      /// Record damage that needs rendering.
      fn record_damage(&mut self, damage: DamageKind);

      /// Whether this surface has any pending damage.
      fn has_damage(&self) -> bool;

      /// Consume and return the pending damage kinds, clearing the set.
      fn take_damage(&mut self) -> DamageSet;

      /// The current lifecycle state.
      fn lifecycle(&self) -> SurfaceLifecycle;
  }
  ```

- [x] This trait is NOT implemented in this section — just defined. The implementations come when Sections 02-06 are all in place. This section is about establishing the vocabulary.

- [x] **Compile dependency:** The `lifecycle()` method returns `SurfaceLifecycle`, which is defined in Section 06 (same file: `surface/mod.rs`). To compile, `SurfaceLifecycle` must be defined first. Either implement Section 06.1 before this subsection, or stub `SurfaceLifecycle` as an empty enum and fill in the variants during Section 06.

---

## 05.4 Integration with WindowContext and DialogWindowContext

**File(s):** `oriterm/src/app/window_context.rs`, `oriterm/src/app/dialog_context/mod.rs`

Add `RenderStrategy` to existing context structs, replacing ad-hoc dirty management over time.

- [x] `WindowContext` gets `render_strategy: RenderStrategy` field, set to `TerminalCached` at construction.

- [x] `DialogWindowContext` gets `render_strategy: RenderStrategy` field, set to `UiRetained` at construction.

- [x] Add `damage: DamageSet` to both contexts. Initially coexists with `dirty: bool` -- `dirty` is set when `!damage.is_empty()`. Future sections remove `dirty` once all render paths consume `damage` directly.

- [x] `urgent_redraw` maps to: the damage set contains `Layout` or `Paint`. Non-urgent damage (e.g. `Cursor`) does not bypass frame budget.

- [x] **`DamageSet` bitflag struct** (defined in `surface/mod.rs` alongside the trait): Since `DamageKind` has only 5 variants, represent the pending damage set as a compact bitflag to avoid per-frame heap allocation:
  ```rust
  /// Pending damage as a compact bitflag set.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub struct DamageSet(u8);

  impl DamageSet {
      pub fn insert(&mut self, kind: DamageKind) { ... }
      pub fn contains(self, kind: DamageKind) -> bool { ... }
      pub fn is_empty(self) -> bool { self.0 == 0 }
      pub fn clear(&mut self) { self.0 = 0; }
  }
  ```

- [x] **Module registration:** Add `pub mod surface;` to `oriterm_ui/src/lib.rs` (after `pub mod text;`). Without this, the new module won't compile.

---

## 05.5 Completion Checklist

**Tests:** If `DamageSet` needs unit tests (bitflag correctness, insert/contains/clear), use the sibling `tests.rs` pattern: `surface/mod.rs` + `surface/tests.rs`. Add `#[cfg(test)] mod tests;` at the bottom of `surface/mod.rs`.

- [x] `RenderStrategy`, `DamageKind`, `DamageSet`, `SurfaceHost` are defined and exported from `oriterm_ui::surface`
- [x] `WindowContext` carries `render_strategy: TerminalCached`
- [x] `DialogWindowContext` carries `render_strategy: UiRetained`
- [x] Damage tracking coexists with existing `dirty` flags (no behavioral change yet)
- [x] New module `oriterm_ui/src/surface/mod.rs` is ≤500 lines
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green

**Exit Criteria:** The abstractions compile, are exported, and are assigned to both context types. No behavioral change — this section only adds vocabulary. Verified by `./test-all.sh` green with zero test changes.
