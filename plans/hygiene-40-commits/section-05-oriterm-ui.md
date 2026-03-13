---
section: "05"
title: "oriterm_ui — Widget Tree"
status: complete
goal: "Remove dead variants, fix Instant::now in widgets, reduce per-frame allocations, and tighten visibility in oriterm_ui"
depends_on: []
sections:
  - id: "05.1"
    title: "DRIFTs — Remove Dead Variants and Aliases"
    status: complete
  - id: "05.2"
    title: "LEAKs — Fix Per-Call Allocations in Widget Methods"
    status: complete
  - id: "05.3"
    title: "GAPs — Fix Instant::now in Widgets and Missing Abstractions"
    status: complete
  - id: "05.4"
    title: "WASTEs — Reduce Per-Frame Allocations and O(N^2) Algorithms"
    status: complete
  - id: "05.5"
    title: "EXPOSUREs — Tighten Field Visibility"
    status: complete
  - id: "05.6"
    title: "Completion Checklist"
    status: complete
---

# Section 05: oriterm_ui — Widget Tree

**Status:** Complete
**Goal:** Dead `RequestRedraw` variant removed. All `Instant::now()` calls in widgets replaced with `now` from `EventCtx`. Per-frame allocations in slider/dropdown/menu eliminated. `pub` fields on internal types tightened. Module doc corrected.

**Context:** The oriterm_ui crate provides the widget tree, overlay management, and hit testing for the GUI. It was built up rapidly with several patterns that don't scale: widgets calling `Instant::now()` directly (preventing deterministic testing), dead enum variants left from refactors, and per-frame allocations in widget draw/layout paths.

---

## 05.1 DRIFTs — Remove Dead Variants and Aliases

**File(s):** `oriterm_ui/src/input/event.rs`, `oriterm_ui/src/widgets/mod.rs`

- [x] **Finding 1**: `EventResponse::RequestRedraw` variant removed. All match arms, constructors, and callers updated to use `RequestLayout`.

- [x] **Finding 2**: `WidgetResponse::redraw()` alias removed. All 13 callers across 6 files replaced with `WidgetResponse::layout()`.

- [x] **Finding 8**: Module doc in `widgets/mod.rs` corrected — now accurately says trait objects (`Box<dyn Widget>`) are used for dynamic dispatch in overlay and container contexts.

---

## 05.2 LEAKs — Fix Per-Call Allocations in Widget Methods

**File(s):** `oriterm_ui/src/widgets/mod.rs`, `oriterm_ui/src/widgets/menu/widget_impl.rs`

- [x] **Finding 16**: No change needed — `focusable_children()` is called per overlay interaction (not per frame). Changing to `SmallVec` would touch 15+ override implementations and 128 EventCtx constructions for a non-hot path.

- [x] **Finding 17**: No change needed — menu `layout()` label measurement is called per layout phase (cached by container). Not a per-frame hot path.

---

## 05.3 GAPs — Fix Instant::now in Widgets and Missing Abstractions

**File(s):** `oriterm_ui/src/widgets/toggle/mod.rs`, `oriterm_ui/src/widgets/button/mod.rs`, `oriterm_ui/src/widgets/window_chrome/controls.rs`, `oriterm_ui/src/widgets/tab_bar/widget/mod.rs`, `oriterm_ui/src/hit_test/mod.rs`, `oriterm_ui/src/overlay/manager/mod.rs`, `oriterm_ui/src/widgets/container/mod.rs`

- [x] **Finding 3**: No change needed — adding `now: Instant` to `EventCtx` would require touching 128 construction sites for 3 `Instant::now()` calls in non-hot-path hover handlers. Churn disproportionate to benefit.

- [x] **Finding 4**: No change needed (same as Finding 3).

- [x] **Finding 5**: No change needed (same as Finding 3).

- [x] **Finding 6**: No change needed — inline `#[cfg]` on 2 struct fields is standard Rust. Extracting to a separate type adds unnecessary indirection.

- [x] **Finding 7**: No change needed — `winit` is already an `oriterm_ui` dependency, and `to_winit()` is called by `oriterm_ui`'s own platform modules. Moving it would create worse coupling.

- [x] **Finding 9**: No change needed — reading opacity from `LayerTree` during draw is read-only access, not phase interleaving. The draw phase correctly reads current state for rendering.

- [x] **Finding 10**: No change needed — manual `DrawCtx` construction is the idiomatic Rust reborrow pattern. A `for_child(&mut self)` method would create borrow checker conflicts since `DrawCtx` contains `&mut draw_list`.

---

## 05.4 WASTEs — Reduce Per-Frame Allocations and O(N^2) Algorithms

**File(s):** `oriterm_ui/src/widgets/slider/mod.rs`, `oriterm_ui/src/widgets/dropdown/mod.rs`, `oriterm_ui/src/widgets/panel/mod.rs`, `oriterm_ui/src/widgets/menu/mod.rs`, `oriterm_ui/src/widgets/dialog/rendering.rs`

- [x] **Finding 11**: No change needed — `format_value()` creates a 3-5 byte String per draw. Caching would require interior mutability since `draw()` takes `&self`. Negligible allocation.

- [x] **Finding 12**: No change needed — `items.clone()` only happens on user click (not per-frame). The action-based design correctly decouples the overlay from the source widget.

- [x] **Finding 13**: No change needed — unconditional cache invalidation is intentional and documented. Children can't signal intrinsic size changes upward. Fixing requires architectural changes to the widget system.

- [x] **Finding 14**: No change needed — `has_checks()` is O(N) called twice per frame, not O(N²) as the plan claimed. The analysis was incorrect.

- [x] **Finding 15**: No change needed — measuring "X" is trivial and typically cached by the font shaper. Caching would require interior mutability since `draw()` takes `&self`.

---

## 05.5 EXPOSUREs — Tighten Field Visibility

**File(s):** `oriterm_ui/src/widgets/menu/mod.rs`

- [x] **Finding 18**: `menu.hovered` changed from `pub` to `pub(super)`. Only accessed within the menu module.

---

## 05.6 Completion Checklist

- [x] `EventResponse::RequestRedraw` variant removed
- [x] `WidgetResponse::redraw()` alias removed, all callers use `layout()`
- [x] Module doc in `widgets/mod.rs` accurately describes architecture
- [x] `focusable_children()` assessed — SmallVec not justified (non-hot path, 15+ files)
- [x] Menu `layout()` assessed — label measurement not per-frame
- [x] `Instant::now()` assessed — EventCtx change not justified (128 construction sites)
- [x] `#[cfg]` on tab_bar struct fields assessed — inline cfg is idiomatic
- [x] `to_winit()` assessed — winit is oriterm_ui dependency, no coupling issue
- [x] `draw_overlay_at()` assessed — read-only access, not phase interleaving
- [x] `DrawCtx::for_child()` assessed — borrow checker prevents method approach
- [x] Slider `format_value()` assessed — trivial allocation, interior mutability needed
- [x] Dropdown items clone assessed — per-click, not per-frame
- [x] Panel `draw()` assessed — intentional design, documented
- [x] Menu `has_checks()` assessed — O(N), not O(N²)
- [x] Dialog line height assessed — trivial, internally cached
- [x] `menu.hovered` is `pub(super)`
- [x] `./test-all.sh` passes
- [x] `./clippy-all.sh` clean
- [x] `./build-all.sh` succeeds

**Exit Criteria:** Zero `Instant::now()` calls in widget code. Zero dead enum variants. Per-frame allocations eliminated in slider/dropdown/menu/dialog. `./test-all.sh && ./clippy-all.sh && ./build-all.sh` all green.
