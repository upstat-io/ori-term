# UI Framework Distillation: Cross-Framework Synthesis

Distilled from deep-dives into **egui**, **iced**, **GPUI (Zed)**, **druid**, **masonry**, and **makepad**. Each framework's individual research is in this directory. This document synthesizes the best patterns across all six and maps them to ori_term's current framework.

---

## Where Every Framework Agrees

These patterns appear in 4+ of the 6 frameworks. They're proven, not experimental.

### 1. Capability-Scoped Contexts

**Who does it:** druid, masonry, GPUI, iced
**Pattern:** Each rendering phase gets its own context type with restricted methods. `EventCtx` can `set_active()` and `request_paint()`. `PaintCtx` can only draw. `LayoutCtx` can only measure. Compile-time enforcement — you literally can't call the wrong method in the wrong phase.

**ori_term status:** We have `DrawCtx`, `EventCtx`, `LayoutCtx` (Section 08 of overhaul plan). But they may not be restrictive enough. Need to audit that `PaintCtx` truly can't trigger layout invalidation.

### 2. Framework-Owned Interaction State

**Who does it:** druid, masonry, GPUI, egui
**Pattern:** Hot/active/focus tracked by the framework, not by widgets. Widgets query `ctx.is_hot()` — they never track mouse position themselves. State computed from layout geometry + mouse position automatically.

**ori_term status:** `InteractionManager` does this. Good. But verify all widgets actually use it and don't have leftover `hovered: bool` fields.

### 3. Status Change Notifications

**Who does it:** druid (lifecycle), masonry (StatusChange), GPUI (element state)
**Pattern:** When hot/focus/active changes, the widget receives a notification **before** the event that caused it. Masonry separates these into `StatusChange::HotChanged(bool)` and `StatusChange::FocusChanged(bool)` — distinct from user events and structural lifecycle.

**ori_term status:** We have `LifecycleEvent` from Section 01. Verify it fires before the causing pointer event, not after.

### 4. Tree-Based Event Routing with Phases

**Who does it:** GPUI (DispatchTree), masonry (WidgetPod recursion), druid (WidgetPod)
**Pattern:** Events travel through a tree built during render. Capture phase (root→target) then bubble phase (target→root). Event consumption prevents further propagation.

**ori_term status:** Section 03 implements capture/bubble. Good alignment.

### 5. BoxConstraints / Limits Layout

**Who does it:** druid, masonry, iced, GPUI (via Taffy)
**Pattern:** Parent provides `Constraints { min, max }`. Child returns `Size` within those constraints. Parent positions child. Two-pass: sizes bottom-up, positions top-down.

**ori_term status:** Our layout system uses this model. Aligned.

### 6. Actions for Widget→App Communication

**Who does it:** masonry (Action), GPUI (actions), iced (Message), makepad (WidgetAction)
**Pattern:** Widgets don't call application code directly. They emit typed actions/messages that bubble up. App layer decides what to do. Decouples widgets from business logic.

**ori_term status:** We have `WidgetAction` enum. Aligned.

---

## Best-in-Class Patterns to Adopt

These are standout patterns from specific frameworks that would significantly improve ori_term.

### A. Entity System (from GPUI) — **HIGH IMPACT**

**Problem it solves:** `Arc<Mutex<T>>` scattered everywhere causes borrow checker friction, deadlock risk, and complex ownership graphs.

**Pattern:** Single `App` owns all state. `Entity<T>` handles are IDs + refcounts. Access always mediated through `Context<T>`:

```rust
entity.update(cx, |state, cx| { state.count += 1; cx.notify(); });
```

**Applicability:** ori_term's pane/tab/window state is currently wrapped in Arc. Moving to centralized entity ownership would eliminate deadlocks and simplify the borrow checker dance documented in our memory (`Borrow Checker Patterns`).

**Recommendation:** Evaluate for `oriterm_mux` pane registry. Would require significant refactoring but pays dividends in every future interaction.

### B. Three-Pass Rendering (from GPUI) — **HIGH IMPACT**

**Problem it solves:** Mixing layout computation with painting prevents caching. Can't cache layout if painting invalidates it.

**Pattern:**
1. `request_layout()` — Compute sizes via Taffy/flex
2. `prepaint()` — Commit hitboxes, prepare GPU resources
3. `paint()` — Render primitives to Scene

**Applicability:** Our current 2-phase (layout + paint) could be split. The prepaint phase would handle hitbox registration (currently mixed into paint), enabling layout caching independent of painting.

### C. Scene Abstraction (from GPUI) — **HIGH IMPACT**

**Problem it solves:** Direct GPU rendering during widget paint prevents damage tracking, z-order sorting, and batching.

**Pattern:** Elements push primitives (quads, text, paths, sprites) into a `Scene` struct. Scene sorted by z-order at frame end. GPU backend consumes sorted scene.

**Applicability:** Would enable damage tracking (only repaint dirty regions) and automatic z-order computation (no manual layer indices).

### D. Instanced GPU Rendering (from makepad) — **HIGH IMPACT**

**Problem it solves:** Per-widget draw calls are expensive. Terminal grid has ~5000 cells.

**Pattern:** All widgets using the same shader append `#[repr(C)]` instance data to a shared buffer. GPU renders ALL instances in one draw call via instancing.

**Applicability:** Terminal grid cells are the perfect use case. Instead of per-cell rendering, batch all cells into one instanced draw call. This is a GPU rendering optimization, not a framework change.

**Note:** ori_term's GPU renderer may already do something like this. Verify `oriterm_gpu` renderer's batching strategy before adding this to the plan.

### E. Separated Event Types (from masonry) — **MEDIUM IMPACT**

**Problem it solves:** Single giant `Event` enum forces widgets to match on irrelevant variants.

**Pattern:**
```rust
fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent);
fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent);
fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange);
```

**Applicability:** Our event controllers already separate pointer from keyboard handling. This is more about the Widget trait interface. Low friction to adopt in Section 08's trait shape.

### F. Safety Rails (from masonry) — **MEDIUM IMPACT**

**Problem it solves:** Container widgets that forget to recurse to children. Widgets visited twice in a pass. Stashed widgets receiving events.

**Pattern:** Debug assertions that validate:
- All declared children visited in every pass
- `place_child()` called for every child in layout
- No double-visits
- Stashed widgets excluded

Uses Bloom filter (`Bloom<WidgetId>`) for cheap child tracking.

**Applicability:** We have 35+ widgets, many with children. Adding these safety rails would catch bugs early. Low cost, high value.

### G. Action/Keymap System (from GPUI) — **MEDIUM IMPACT**

**Problem it solves:** Keyboard shortcuts hard-coded in event handlers. Can't rebind at runtime. Can't replay for macros/testing.

**Pattern:** Actions declared via macro. Bound to keystrokes in a Keymap (data, not code). DispatchTree routes through `KeyContext` tags (e.g., "Editor", "Dialog"). Focused element's context wins.

**Applicability:** ori_term already needs keybinding support for the terminal. Separating actions from handlers enables runtime rebinding, macro recording, and accessibility.

### H. StyleRefinement (from GPUI) — **LOWER IMPACT**

**Problem it solves:** Builder pattern requires full style struct allocation at each step.

**Pattern:** `#[derive(Refineable)]` generates sparse `StyleRefinement` with `Option<T>` for each field. Builder methods set one field. Final style computed by merging.

**Applicability:** Nice ergonomic improvement but not critical. Our theme system works fine. Could adopt if we add CSS-like method chaining.

### I. Frame Cache (from egui) — **LOWER IMPACT**

**Problem it solves:** Expensive computations (text shaping, glyph rasterization) recomputed every frame.

**Pattern:** Cache by key hash, evict entries not used this frame. One-line API: `cache.get(key)`.

**Applicability:** We already have glyph caching in `oriterm_gpu`. Could generalize for layout computation caching.

### J. Arena Allocator (from GPUI) — **LOWER IMPACT**

**Problem it solves:** Per-widget allocation fragmentation. Thousands of elements allocated and freed per frame.

**Pattern:** Custom arena: bump-allocate per frame, clear all at once. Chunks reused.

**Applicability:** Only relevant if we find allocation is a bottleneck. Profile first.

---

## Patterns We Already Have That Are Best-in-Class

### Our Animation Engine > Everyone Else's

- **egui:** Linear interpolation only, no easing, no spring physics
- **iced:** No built-in animation
- **GPUI:** Minimal — closure-based easing, no state machine
- **druid/masonry:** Only `AnimFrame` lifecycle event
- **makepad:** Good declarative state machines, but tied to DSL

**ori_term has:** Spring physics, easing curves, AnimBehavior/AnimProperty, transactions for atomic updates, Visual State Manager with state groups. This is genuinely superior. Keep it.

### Our Controller Decomposition > Everyone Else's

- **egui:** Monolithic `Response` object
- **iced:** Everything in `update()` method
- **GPUI:** Everything in event handlers
- **druid/masonry:** Everything in `event()` / `on_pointer_event()`
- **makepad:** Everything in `handle_event()`

**ori_term has:** Separate HoverController, ClickController, DragController, ScrollController, FocusController, etc. Independently testable. Composable across widget types. This is the GTK4 pattern and it's strictly better than monolithic handlers.

### Our Sense Declarations = egui's (Both Good)

Both use bitflags to declare widget interaction capabilities. Framework uses Sense to optimize hit testing and event routing.

### Our InteractionManager = druid/masonry's WidgetPod State (Both Good)

Framework-managed hot/active/focus. Widgets query, don't track.

---

## What We're Missing (Gap Analysis)

| Gap | Impact | Source Framework | Current State |
|-----|--------|-----------------|---------------|
| Scene abstraction (damage tracking) | High | GPUI | Direct GPU rendering in paint |
| 3-pass rendering (layout/prepaint/paint) | High | GPUI | 2-pass (layout + paint) |
| Safety rails (debug assertions on tree traversal) | Medium | masonry | None |
| Keybinding/action dispatch system | Medium | GPUI | Direct key handling |
| Separated pointer/text/status events on Widget trait | Medium | masonry | Events mixed in controller dispatch |
| WidgetRef for read-only introspection | Low | masonry | No formal read-only access pattern |
| Test harness (headless widget testing) | Medium | masonry, egui, iced | No headless test infrastructure |
| Frame cache for expensive computations | Low | egui | Ad-hoc caching |
| Global pattern for shared config | Low | GPUI | Parameter passing |

---

## Prioritized Recommendations

### Tier 1: Fix Before Shipping (High Impact, Architectural)

1. **Scene abstraction** — Collect paint primitives, sort by z-order, enable damage tracking
2. **Safety rails** — Debug assertions on child visitation, double-visit prevention
3. **Test harness** — Headless widget testing with input simulation and state inspection

### Tier 2: Significant Quality Improvement (Medium Impact)

4. **3-pass rendering** — Split prepaint from paint for caching
5. **Action/keymap system** — Separate keybindings from handlers
6. **Separated event types** — `on_pointer_event()` / `on_text_event()` / `on_status_change()` on Widget trait

### Tier 3: Polish and Optimization (Lower Impact)

7. **Entity system** — Centralized state ownership (big refactor, evaluate ROI)
8. **Instanced grid rendering** — Verify GPU renderer batching, optimize if needed
9. **StyleRefinement** — Ergonomic builder pattern for styles
10. **Frame cache** — Generalized computation caching

### Tier 4: Nice-to-Have

11. Arena allocator (profile first)
12. Bloom filter child tracking
13. WidgetRef introspection
14. Global pattern for shared config

---

## Framework Comparison Matrix

| Aspect | egui | iced | GPUI | druid | masonry | makepad | **ori_term** |
|--------|------|------|------|-------|---------|---------|-------------|
| **Mode** | Immediate | Retained (Elm) | Retained (Entity) | Retained (Generic) | Retained | GPU-native | **Retained** |
| **State** | IdTypeMap | Tree+Tag | Entity system | Widget struct | WidgetPod | UID+LiveId | **Widget struct** |
| **Interaction** | Sense+Response | Shell+Status | Element state | WidgetPod auto | StatusChange | Area+Hit | **InteractionMgr** |
| **Event routing** | Hit test | Tree walk | DispatchTree | WidgetPod | WidgetPod | Area hits | **Capture/Bubble** |
| **Animation** | Linear only | None built-in | Minimal easing | AnimFrame only | AnimFrame only | State machine | **Spring+Easing+VSM** |
| **Layout** | Placer (1-pass) | Limits | Taffy (flex/grid) | BoxConstraints | BoxConstraints | Turtle | **Flex+Grid** |
| **Controllers** | None (Sense) | None (update) | None (handlers) | None (event) | None (handlers) | None (handlers) | **6 controllers** |
| **GPU** | CPU tessellate | Pluggable | Scene+backend | Piet (2D) | Vello | Instanced shaders | **wgpu direct** |
| **Testing** | Snapshot | Simulator | TestAppContext | Harness | TestHarness | None | **None (gap)** |
| **Safety** | ID clash warn | Type panic | Weak ref check | Hot/active auto | Bloom+assertions | None | **None (gap)** |

---

## Relationship to Current Plan

The existing `plans/ui-framework-overhaul/` has 11 sections, most complete. This distillation identifies:

1. **Gaps not covered by any section** — Scene abstraction, safety rails, test harness, keybinding system
2. **Sections that could be strengthened** — Section 08 (Widget Trait) could adopt masonry's separated event types; Section 03 (Event Propagation) could add GPUI's DispatchTree pattern
3. **Validation of existing design** — Our animation engine, controller decomposition, Sense declarations, and InteractionManager are confirmed best-in-class across all 6 frameworks

The next step is to create a **focused improvement plan** that addresses the gaps without disrupting the working framework. The biggest wins are: Scene abstraction, safety rails, and test harness.
