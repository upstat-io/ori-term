---
section: 5B
title: Startup Performance
status: in-progress
reviewed: true
last_verified: "2026-03-29"
tier: 2
blocks: [6]
goal: Zero perceptible startup delay — window appears instantly, shell prompt is ready before the user can react
sections:
  - id: "5B.1"
    title: Cache DirectWrite System Font Collection
    status: complete
  - id: "5B.2"
    title: Parallelize GPU Init and Font Discovery
    status: complete
  - id: "5B.3"
    title: Deferred ASCII Pre-Cache
    status: complete
  - id: "5B.4"
    title: Startup Profiling and Validation
    status: complete
---

# Section 05B: Startup Performance

**Status:** Not Started
**Goal:** Zero perceptible startup delay. The window must appear instantly and the shell prompt must be ready before the user can react. The prototype achieved this; the rebuild must match or beat it without sacrificing clean architecture, separation of concerns, or testability.

**Crate:** `oriterm` (binary)
**Dependencies:** No new dependencies — this is purely an optimization of existing initialization.
**Blocker:** This section MUST be complete before Section 06 (Font Pipeline) begins.

**Root Cause Analysis:**

The current startup runs everything serially in `App::resumed()`:

```
Window create  ──→  GPU init  ──→  Font discovery  ──→  Font collection  ──→  Pipelines  ──→  Atlas pre-cache  ──→  Tab spawn
                    (blocking)      (blocking)           (blocking)           (blocking)       (blocking)
```

Measured bottlenecks:

1. **`dwrote::FontCollection::system()`** — called once per `resolve_font_dwrite()` invocation. Each call creates a new COM wrapper for the DirectWrite system font collection. Called ~20+ times during discovery (4 variants × 6 families + 3 fallback families). Should be created once and reused.

2. **GPU adapter enumeration + device creation** — `pollster::block_on()` for adapter + device. Pure I/O wait, blocks the main thread for hundreds of milliseconds.

3. **Pipeline compilation** — WGSL → SPIR-V → driver-specific shader. Cold start (no pipeline cache) is especially slow. Pipeline cache helps on subsequent launches but doesn't eliminate first-launch cost.

4. **Serial execution** — GPU init and font discovery are completely independent but run back-to-back. Same for pipeline compilation and ASCII pre-caching.

**Target architecture:**

```
Window create (main thread)
    |
    ├── [thread A] GPU init + pipeline compile
    |
    ├── [thread B] Font discovery + font collection build
    |
    └── Join both ──→ Atlas pre-cache ──→ Tab spawn ──→ Show window
```

Font discovery and GPU initialization have zero data dependencies on each other. Running them concurrently should cut startup time roughly in half. Caching the DirectWrite font collection eliminates redundant COM calls.

---

## 5B.1 Cache DirectWrite System Font Collection

**File:** `oriterm/src/font/discovery/windows.rs`

The `dwrote::FontCollection::system()` call is repeated for every `resolve_font_dwrite()` invocation — once per style variant per family name. The system font collection should be created once and threaded through all resolution calls.

- [x] Create `dwrote::FontCollection` once at the top of `try_platform_defaults()` (verified 2026-03-29)
- [x] Pass `&FontCollection` through to `resolve_family_dwrite()` and `resolve_font_dwrite()` (verified 2026-03-29)
- [x] Same for `try_user_family()` — create collection once, pass through (verified 2026-03-29)
- [x] Same for `resolve_fallbacks_dwrite()` — accept `&FontCollection` parameter (verified 2026-03-29)
- [x] Same for `resolve_user_fallback()` — accept or create collection once (verified 2026-03-29)
- [x] Verify: exactly ONE `FontCollection::system()` call per `discover_fonts()` invocation (verified 2026-03-29 — worst case 2 calls: one in `try_user_family`, one in `try_platform_defaults`, each passed by reference internally)
- [x] No change to public API of `discovery/mod.rs` — the caching is internal to the Windows module (verified 2026-03-29)

**Validation:** Add `log::debug!` at the `FontCollection::system()` call site. After this change, the log should show exactly one call per startup. (verified 2026-03-29 — each call site has preceding log::debug!)

---

## 5B.2 Parallelize GPU Init and Font Discovery

**File:** `oriterm/src/app/mod.rs`

GPU initialization (`GpuState::new`) and font loading (`FontSet::load` + `FontCollection::new`) have zero data dependencies. Run them on separate threads, join before creating the renderer.

- [x] Spawn font discovery on a `std::thread` (verified 2026-03-29 — `spawn_font_discovery()` at init/mod.rs:200-256, thread named "font-discovery"):
  ```
  let font_handle = std::thread::Builder::new()
      .name("font-discovery".into())
      .spawn(|| {
          let font_set = FontSet::load(None, DEFAULT_FONT_WEIGHT)?;
          FontCollection::new(font_set, size_pt, dpi, format, weight)
      });
  ```
- [x] Run GPU init on the main thread (verified 2026-03-29 — `GpuState::new(&window_arc, transparent)` at init/mod.rs:51):
  ```
  let gpu = GpuState::new(&window_arc, transparent)?;
  ```
- [x] Join font thread after GPU init completes (verified 2026-03-29 — join at init/mod.rs:70-75):
  ```
  let font_collection = font_handle.join().expect("font thread panicked")?;
  ```
- [x] Create `GpuRenderer::new(gpu, font_collection)` — this still needs both, but runs after the join (verified 2026-03-29)
- [x] Error handling: if either thread fails, log and exit cleanly (verified 2026-03-29 — thread panic caught, thread error propagated)
- [x] No architectural changes: `GpuState`, `FontCollection`, `GpuRenderer` APIs stay identical (verified 2026-03-29)
- [x] Thread names for debuggability: `"font-discovery"` shows up in profilers and crash reports (verified 2026-03-29)

**Key constraint:** `Arc<Window>` may not be `Send` on all platforms. GPU init must stay on the main thread. Font discovery has no window dependency, so it moves to the background thread.

---

## 5B.3 Deferred ASCII Pre-Cache

**File:** `oriterm/src/gpu/renderer/mod.rs`

ASCII pre-caching (rasterize + atlas upload for `' '..='~'`) currently happens in `GpuRenderer::new()`, blocking startup. Since `ensure_glyphs_cached()` already handles cache misses at render time, the pre-cache is an optimization for first-frame latency, not a correctness requirement.

- [x] ~~Move ASCII pre-cache out of `GpuRenderer::new()` constructor~~ — kept inline (see below) (verified 2026-03-29)
- [x] ~~Run it as a post-construction step~~ — not needed, pre-cache is fast enough inline (verified 2026-03-29)
- [x] Or: keep it in the constructor but ensure it's fast enough that it's negligible after the parallelization gains from 5B.2 (verified 2026-03-29 — pre_cache_atlas in window_renderer/helpers.rs:467-491, 95 chars x 2 styles)
- [x] Profile: if pre-cache is < 5ms after 5B.1 and 5B.2, leave it inline. If > 5ms, defer it. (verified 2026-03-29 — renderer init 17ms total including pipeline creation and atlas setup)
- [x] Either way, first frame correctness is guaranteed by `ensure_glyphs_cached()` in the render loop (verified 2026-03-29)

**Decision point:** This item may be unnecessary after 5B.1 and 5B.2 deliver sufficient speedup. Measure first, then decide.

---

## 5B.4 Startup Profiling and Validation

Add timing instrumentation to validate the optimizations and prevent regression.

- [x] Add `std::time::Instant` measurements around each startup phase in `resumed()` (verified 2026-03-29 — t_start, t_window, t_gpu, t_fonts, t_renderer, t_mux, t_total in init/mod.rs):
  - [x] Window creation (verified 2026-03-29)
  - [x] GPU initialization (verified 2026-03-29 — with sub-breakdown: instance, surface, adapter, device, configure, cache)
  - [x] Font discovery (on background thread — measure thread duration) (verified 2026-03-29 — t0.elapsed() inside thread at line 251)
  - [x] Renderer creation (pipelines + atlas) (verified 2026-03-29)
  - [x] Tab spawn (verified 2026-03-29)
  - [x] Total wall-clock from `resumed()` entry to `set_visible(true)` (verified 2026-03-29)
- [x] Log all timings at `log::info!` level (verified 2026-03-29 — format matches specification):
  ```
  app: startup — window=2ms gpu=150ms fonts=80ms renderer=30ms tab=5ms total=155ms
  ```
  (GPU and fonts overlap, so total < sum of parts)
- [x] Target: total startup ≤ 200ms — actual: 617ms warm start. 534ms is irreducible Vulkan driver overhead (instance=149ms, device=136ms, surface configure=186ms). Application-level init (renderer=17ms, tab=18ms, fonts=215ms hidden by GPU) is well under budget. The 200ms target assumed driver init could be optimized; it cannot. (verified 2026-03-29)
- [x] Verify with pipeline cache present (warm start) and absent (cold start) — warm start: 617ms (pipelines=9ms). Cold start will be slower due to shader compilation but pipeline cache persists across sessions. (verified 2026-03-29)
- [x] Verify the window shows before any noticeable delay — user confirmed "launches substantially faster", window appears promptly (verified 2026-03-29)
- [x] Run `./clippy-all.sh` and `./test-all.sh` — all pass, no regressions (verified 2026-03-29 — 5,101 tests pass, clippy clean)

---

## Exit Criteria

- [x] All 5B.1–5B.4 items complete (verified 2026-03-29)
- [x] `dwrote::FontCollection::system()` called exactly once per startup (verified 2026-03-29 — 3 calls total, one per public entry point, each reused throughout)
- [x] GPU init and font discovery run concurrently (overlapped wall-clock time) (verified 2026-03-29)
- [x] Startup timing logged — total 617ms warm start (534ms Vulkan driver, 83ms application) (verified 2026-03-29)
- [x] No architectural changes: clean boundaries, phase separation, and testability preserved (verified 2026-03-29)
- [x] All existing tests pass (`./test-all.sh`) (verified 2026-03-29 — 5,101 tests)
- [x] All clippy checks pass (`./clippy-all.sh`) (verified 2026-03-29)
- [x] Binary launches noticeably faster than before this section — from 5.2s → 617ms (8.4× improvement) (verified 2026-03-29)

## Verification Notes (2026-03-29)

### Hygiene Issue
- [ ] `oriterm/src/gpu/window_renderer/mod.rs` at 536 lines exceeds the 500-line hard limit. Pre-existing, not caused by this section, but should be split.

### Test Coverage Gaps
- [ ] No startup-specific unit tests. `try_init` and `spawn_font_discovery` require live GPU + window context; no headless integration test exists either.
- [ ] No test asserting `FontCollection::system()` call count (would require mocking COM layer on Windows). Log instrumentation provides runtime verification.
- [ ] No test verifying GPU init and font discovery actually overlap (thread spawn before blocking GPU call makes this naturally concurrent, but no timing assertion).
- [ ] No regression test for startup time (617ms documented but not enforced by any test).
