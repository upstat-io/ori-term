---
section: "05"
title: "Golden Lane Determinism"
status: not-started
reviewed: false
goal: "Make the canonical golden image lane reproducible across runs and machines by pinning a software rasterizer (llvmpipe), pinning hinting mode to grayscale alpha for goldens, pinning cell metrics, and tightening tolerance to exact-or-tiny per-pixel matching with SSIM/ΔE relegated to diagnostic-only."
success_criteria:
  - "`oriterm/src/gpu/visual_regression/mod.rs::headless_env_with_pinned_software_rasterizer()` exists and returns a `(GpuState, GpuPipelines, WindowRenderer)` triple using llvmpipe (or equivalent software rasterizer) explicitly"
  - "`HintingMode::Full` is no longer hardcoded; spec-conformance golden tests default to `HintingMode::None` (grayscale alpha) for cross-machine reproducibility; per-test override hooks exist for tests that need explicit hinting"
  - "`oriterm/src/gpu/state/helpers.rs::pick_adapter` (current code: `enumerate_adapters` + first-discrete-then-fallback) no longer makes the headless selection uncontrollable. `GpuState::try_init_headless` (at `mod.rs:430`) and `GpuState::new_headless` (at `mod.rs:156`) are extended to accept an explicit adapter preference (software rasterizer / discrete / integrated), and the spec-conformance harness passes the software-rasterizer preference."
  - "Cell metrics (cell width, cell height in pixels) are pinned per golden test via a `GoldenLaneConfig` struct; default values match the existing visual_regression test font (12pt @ 96 DPI)"
  - "Per-pixel tolerance for spec-conformance goldens defaults to 0 (exact match); per-test override allows ΔE ≤ 1 with explicit comment justification"
  - "SSIM / ΔE diff metrics are relegated to diagnostic-only — they appear in failure messages but are NOT the gating metric"
  - "All existing visual_regression tests still pass (the existing tests use the old non-deterministic env via the un-renamed `headless_env()`; spec-conformance goldens use the new pinned env)"
  - "BLOAT check: `oriterm/src/gpu/state/mod.rs` (currently 493) does not exceed 500 lines after this section's edits — split if it does"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Deterministic golden environment** mission criterion"
inspired_by:
  - "wgpu-test patterns — explicit adapter selection via `wgpu::Backends::VULKAN | wgpu::Backends::GL` and `wgpu::PowerPreference::LowPower` for software fallback"
  - "Mesa llvmpipe — software rasterizer used by Linux CI environments for reproducible GPU output"
  - "ori_term existing `oriterm/src/gpu/visual_regression/mod.rs:65-112` — `headless_env_with_hinting()` is the entry point being extended"
depends_on: ["04"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "05.1"
    title: "Add explicit adapter preference parameter to GpuState::new_headless"
    status: not-started
  - id: "05.2"
    title: "Add headless_env_with_pinned_software_rasterizer() entry point"
    status: not-started
  - id: "05.3"
    title: "Decouple HintingMode default from spec-conformance goldens"
    status: not-started
  - id: "05.4"
    title: "Pin cell metrics via GoldenLaneConfig struct"
    status: not-started
  - id: "05.5"
    title: "Tighten tolerance: exact-or-tiny default, per-test override"
    status: not-started
  - id: "05.6"
    title: "Migrate sixel_minimal pilot golden to the deterministic lane"
    status: not-started
  - id: "05.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "05.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 05: Golden Lane Determinism

**Status:** Not Started
**Goal:** Make the spec-conformance golden image lane reproducible across runs and machines. Section 04's pilots may pass on the dev machine but flake on CI or another developer's GPU because the current `oriterm/src/gpu/visual_regression/mod.rs` picks `wgpu::PowerPreference::HighPerformance` adapters non-deterministically and uses `HintingMode::Full` which interacts with subpixel rasterization. This section pins everything: software rasterizer (llvmpipe or equivalent), grayscale alpha hinting, cell metrics, and exact-or-tiny per-pixel tolerance as the primary gate (with SSIM/ΔE only as diagnostic).

**Success Criteria:**
- [ ] `headless_env_with_pinned_software_rasterizer()` entry point exists in visual_regression
- [ ] `GpuState::new_headless()` accepts adapter preference parameter; spec-conformance harness passes software rasterizer
- [ ] `HintingMode::Full` no longer hardcoded; spec-conformance defaults to grayscale alpha (`HintingMode::None`)
- [ ] Cell metrics pinned per golden test via `GoldenLaneConfig`
- [ ] Tolerance defaults to exact (0); per-test override to ΔE ≤ 1 with comment
- [ ] SSIM/ΔE in diagnostic only, not gating
- [ ] Existing visual_regression tests still pass (back-compat preserved)
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Connects to mission criterion: **Deterministic golden environment**

**Context:** Pass 1 confirmed `oriterm/src/gpu/state/helpers.rs::pick_adapter` (called indirectly by `GpuState::new_headless` at `mod.rs:156` → `try_init_headless` at `mod.rs:430`) enumerates adapters via `instance.enumerate_adapters(backends)` and picks the first discrete GPU (or any fallback). There is NO `PowerPreference` pin, NO `force_fallback_adapter`, and NO software-rasterizer preference — headless selection is non-deterministic across machines. Pass 1 also confirmed `oriterm/src/gpu/visual_regression/mod.rs:87` defaults `headless_env_full()` to `HintingMode::Full`. Both produce variation across runs and machines: a CI runner on Mesa with llvmpipe will rasterize differently from a dev machine with NVIDIA, and `HintingMode::Full` interacts with subpixel positioning to produce slightly different glyph edges. Codex's Step 6B feedback stated explicitly: "deterministic GPU adapter selection belongs in foundation if visual verification is part of the locked model; otherwise every later golden/image section inherits avoidable flake."

**Reference implementations:**
- **wgpu-test** patterns — adapter selection via `wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::LowPower, force_fallback_adapter: true, .. }` forces a software fallback when available.
- **Mesa llvmpipe** — Linux software rasterizer used by CI environments for reproducible output. Available on most Linux distros via `mesa-utils` or similar.
- **ori_term existing** `oriterm/src/gpu/visual_regression/mod.rs:65-112` — `headless_env_with_hinting()` is the entry point being extended with the deterministic variant.

**Depends on:** Section 04 (the spec-conformance harness needs the deterministic env to capture the sixel_minimal pilot golden reproducibly).

---

## 05.1 Add explicit adapter preference parameter to GpuState::new_headless

**File(s):** `oriterm/src/gpu/state/mod.rs`, `oriterm/src/gpu/state/helpers.rs`, `oriterm/src/gpu/state/tests.rs`

`GpuState::new_headless()` (at `mod.rs:156`) currently calls `try_init_headless()` (at `mod.rs:430`) which in turn calls `pick_adapter()` in `helpers.rs:8`. The current `pick_adapter` uses `instance.enumerate_adapters(backends)` + "first discrete GPU or fallback" logic — no `PowerPreference`, no `force_fallback_adapter`, no software-rasterizer path. Add an overload that accepts an explicit adapter preference so the caller can pin the selection for reproducibility.

- [ ] Define an `AdapterPreference` enum in `oriterm/src/gpu/state/helpers.rs`:
  ```rust
  pub(crate) enum AdapterPreference {
      /// Current default — discrete GPU preferred, any fallback.
      DiscreteOrFallback,
      /// Software rasterizer (llvmpipe / WARP). Returns `None` if unavailable.
      SoftwareRasterizer,
  }
  ```
- [ ] Extend `pick_adapter()` (at `helpers.rs:8`) to take `AdapterPreference` and implement the software-rasterizer branch. For `SoftwareRasterizer`: filter `enumerate_adapters()` output for entries whose `adapter.get_info().device_type == wgpu::DeviceType::Cpu`, falling back to `force_fallback_adapter: true` via an explicit `instance.request_adapter()` call if enumeration finds nothing.
- [ ] Refactor `GpuState::try_init_headless()` (at `mod.rs:430`) to thread `AdapterPreference` from a new `GpuState::new_headless_with_preference(pref: AdapterPreference)` entry point. Preserve the existing `new_headless()` behavior (it calls `new_headless_with_preference(AdapterPreference::DiscreteOrFallback)`).
- [ ] Sibling tests in `oriterm/src/gpu/state/tests.rs`:
  - `new_headless_default_picks_discrete_or_fallback()` (existing behavior preserved)
  - `new_headless_with_software_rasterizer_picks_llvmpipe_or_warp_or_returns_none()`
  - `pick_adapter_software_rasterizer_prefers_cpu_device_type()`
- [ ] **Validation**: existing tests still pass; new entry point works on Linux (the canonical golden lane).
- [ ] **BLOAT check**: `wc -l oriterm/src/gpu/state/mod.rs` — currently at 493. If this section pushes it over 500, split immediately into `state/mod.rs` (dispatch) + `state/headless.rs` (the headless construction logic).

---

## 05.2 Add headless_env_with_pinned_software_rasterizer() entry point

**File(s):** `oriterm/src/gpu/visual_regression/mod.rs`, sibling tests

A new entry point in visual_regression that uses the pinned software rasterizer. The existing entry points (`headless_env`, `headless_env_with_config`, `headless_env_with_hinting`) remain unchanged for back-compat with the existing visual_regression test suite.

- [ ] Add `headless_env_with_pinned_software_rasterizer(config: GoldenLaneConfig) -> Option<(GpuState, GpuPipelines, WindowRenderer)>` to `oriterm/src/gpu/visual_regression/mod.rs`. Implementation calls `GpuState::new_headless_with_adapter(GpuState::software_rasterizer_options())`, then uses `config` for font/hinting/cell metrics.
- [ ] The function returns `None` if the software rasterizer is unavailable on the current platform. Tests using this entry point should `#[cfg_attr(not(target_os = "linux"), ignore)]` on macOS/Windows where the software rasterizer story is more nuanced (section 23's CI matrix configures the appropriate platforms).
- [ ] Sibling tests in `oriterm/src/gpu/visual_regression/tests.rs` (or wherever existing tests live):
  - `pinned_software_rasterizer_returns_env_on_linux()`
- [ ] **Validation**: new entry point works on Linux; gracefully `None`s on platforms without llvmpipe.

---

## 05.3 Decouple HintingMode default from spec-conformance goldens

**File(s):** `oriterm/src/gpu/visual_regression/mod.rs`, `crates/oriterm_test_support/src/spec_chain/observers/golden.rs`

The existing `headless_env_full()` defaults to `HintingMode::Full` at line 87. The existing tests rely on this default. Spec-conformance goldens use a NEW default (`HintingMode::None` for grayscale alpha). The decoupling: don't change the existing default, add a new spec-conformance-specific path with the new default.

- [ ] In `oriterm/src/gpu/visual_regression/mod.rs`, document that `headless_env_full()` defaults to `HintingMode::Full` for back-compat with existing tests.
- [ ] **[WASTE]** `oriterm/src/gpu/visual_regression/mod.rs:78,82,92` — add a doc comment block above `headless_env`, `headless_env_full`, and `headless_env_with_hinting` marking them as **legacy non-deterministic entry points**. New spec-conformance goldens MUST use `headless_env_with_pinned_software_rasterizer` instead. The legacy functions stay because the existing reference_tests.rs suite depends on them (see below).
- [ ] **[WASTE]** `oriterm/src/gpu/visual_regression/reference_tests.rs:15-16,354,424` — the reference_tests suite imports and uses `headless_env_full` and `headless_env_with_hinting` directly. Add a module-level `//!` doc comment at the top of `reference_tests.rs` stating: "This module uses the legacy non-deterministic visual_regression env. New golden tests for spec conformance MUST use `headless_env_with_pinned_software_rasterizer` (see section 05). These tests remain on the legacy env for historical back-compat only."
- [ ] In `headless_env_with_pinned_software_rasterizer()`, default to `HintingMode::None` (grayscale alpha) unless `GoldenLaneConfig` overrides.
- [ ] Update `crates/oriterm_test_support/src/spec_chain/observers/golden.rs` (from section 04) to use the new pinned entry point for spec-conformance goldens.
- [ ] Document why grayscale alpha is the default: hinting + subpixel positioning interactions produce variation across runs even on the same machine; grayscale alpha is reproducible because the rasterizer makes simpler decisions.
- [ ] **Validation**: spec-conformance goldens are reproducible across two consecutive runs of the same test on the same machine (manual sanity check).

---

## 05.4 Pin cell metrics via GoldenLaneConfig struct

**File(s):** `oriterm/src/gpu/visual_regression/mod.rs`, `crates/oriterm_test_support/src/spec_chain/golden_lane_config.rs` (new)

Cell metrics depend on font + hinting + GPU driver. Even with the software rasterizer, font metrics may shift if the font cache produces a slightly different glyph for the same character at the same size. Pinning cell metrics (cell width and height in pixels) at config time eliminates this drift.

- [ ] Create `GoldenLaneConfig` struct:
  ```rust
  pub struct GoldenLaneConfig {
      pub font_size_pt: f32,    // default 12.0
      pub dpi: f32,             // default 96.0
      pub glyph_format: GlyphFormat, // default Alpha
      pub hinting_mode: HintingMode, // default None for grayscale alpha
      pub cell_width_px: u32,   // pinned, default computed from font
      pub cell_height_px: u32,  // pinned, default computed from font
      pub viewport_cols: u32,
      pub viewport_rows: u32,
      pub pixel_tolerance: u8,  // default 0 (exact)
      pub max_diff_percent: f64, // default 0.0
  }

  impl GoldenLaneConfig {
      pub const SPEC_DEFAULT: Self = Self { /* canonical values */ };
  }
  ```
- [ ] Wire `GoldenLaneConfig` through `headless_env_with_pinned_software_rasterizer()` so the test caller can override defaults.
- [ ] **Validation**: same scenario rendered with same `GoldenLaneConfig::SPEC_DEFAULT` produces identical pixels on two consecutive runs.

---

## 05.5 Tighten tolerance: exact-or-tiny default, per-test override

**File(s):** `crates/oriterm_test_support/src/spec_chain/observers/golden.rs`, `oriterm/src/gpu/visual_regression/mod.rs`

Section 04's golden observer uses the existing `compare_with_reference()` with `PIXEL_TOLERANCE = 2` and `MAX_MISMATCH_PERCENT = 0.5`. Spec-conformance tightens this to exact-or-tiny per-pixel matching as the primary gate. SSIM/ΔE are diagnostic only.

- [ ] Add `compare_with_reference_strict(name, pixels, w, h, config: &GoldenLaneConfig) -> Result<(), String>` to visual_regression. Strict mode:
  - Default: pixel-exact match (every channel identical)
  - Per-config override: ΔE ≤ N for N up to 1 (any larger requires explicit comment in the test)
  - Failure message includes ΔE distribution histogram, max ΔE per channel, SSIM score, and saves `_actual.png` + `_diff.png` for inspection
- [ ] Update spec_chain golden observer to call the strict variant.
- [ ] Document the test-side override pattern: `#[golden_lane_override(pixel_tolerance = 1, reason = "anti-aliasing variation on glyph edges")]` (or similar — concrete syntax TBD by implementation).
- [ ] **Validation**: identical inputs produce 0-diff; intentional 1-pixel changes are detected and rejected.

---

## 05.6 Migrate sixel_minimal pilot golden to the deterministic lane

**File(s):** `oriterm_core/tests/spec_chain/pilots/sixel_minimal.rs` (modified), `crates/oriterm_test_support/tests/references/spec_chain/pilots/sixel_minimal.png` (re-captured)

Section 04 captured the sixel_minimal golden using the existing non-deterministic env. Now that the deterministic lane exists, re-capture the golden via the deterministic env and verify the test passes reproducibly.

- [ ] Update `oriterm_core/tests/spec_chain/pilots/sixel_minimal.rs` to use `headless_env_with_pinned_software_rasterizer(GoldenLaneConfig::SPEC_DEFAULT)` instead of the existing `headless_env_with_hinting()`.
- [ ] Re-capture the golden:
  ```bash
  ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm_core --test spec_chain pilots::sixel_minimal
  ```
- [ ] Verify reproducibility — run the test twice in succession and confirm the diff is exactly 0 pixels:
  ```bash
  cargo test -p oriterm_core --test spec_chain pilots::sixel_minimal
  cargo test -p oriterm_core --test spec_chain pilots::sixel_minimal
  ```
- [ ] **Validation**: pilot test passes; golden is the new deterministic-lane image; back-to-back runs produce 0 diffs.

---

## 05.R Third Party Review Findings

- None.

---

## 05.N Completion Checklist

- [ ] Failing test matrix written FIRST (TDD)
- [ ] **Matrix dimensions**: adapter type (high-perf vs software) × hinting mode × cell metric pinning × tolerance configuration
- [ ] **Semantic pin**: golden reproducibility test — same scenario rendered twice produces 0-pixel diff (this is the regression guard for the entire deterministic lane)
- [ ] `headless_env_with_pinned_software_rasterizer()` entry point works on Linux
- [ ] HintingMode default is grayscale alpha for spec-conformance goldens
- [ ] `GoldenLaneConfig` struct exists and pins cell metrics
- [ ] `compare_with_reference_strict()` defaults to 0 pixel tolerance
- [ ] Sixel pilot golden re-captured; back-to-back runs produce 0-pixel diff
- [ ] Existing visual_regression tests still pass (back-compat preserved via separate entry points)
- [ ] `oriterm/src/gpu/state/mod.rs` line count under 500
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 05 status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Spec-conformance golden tests run reproducibly on Linux/x86_64 with the pinned software rasterizer; sixel pilot golden produces 0-pixel diff on back-to-back runs; existing visual_regression tests still pass.
