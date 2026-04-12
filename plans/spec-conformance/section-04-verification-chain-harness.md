---
section: "04"
title: "Verification Chain Harness + Pilots + Coverage Report"
status: not-started
reviewed: false
goal: "Build the SpecHarness API that drives a sequence through every applicable rung of the verification chain (parser → dispatch → state/effect → renderable → frame-input → gpu-instance → texture → golden), validate the API with one visual pilot (sixel) and one non-visual pilot (DA1), freeze the catalog row schema based on what the pilots needed, and deliver the spec-coverage-report binary."
success_criteria:
  - "`SpecHarness` API exists in `oriterm_test_support/src/spec_chain/mod.rs` with methods to drive a sequence through every applicable rung and observe the per-rung result"
  - "Sixel visual pilot test exists at `oriterm_core/tests/spec_chain/pilots/sixel_minimal.rs` (or equivalent path) — drives a minimal sixel raster fill scenario through every applicable rung from parser to golden image, all green"
  - "DA1 non-visual pilot test exists at `oriterm_core/tests/spec_chain/pilots/da1_query.rs` — drives a DA1 query through parser → dispatch → handler → effect transcript apex (PtyEffect::Write with PtyWriteKind::DeviceAttribute), all green"
  - "Catalog row schema is FROZEN: `plans/spec-conformance/catalog/README.md` documents the canonical row format, the column set, and the rung naming convention used by the harness"
  - "All catalog files from section 01 are migrated from the provisional schema to the frozen schema (all rows updated to the canonical format)"
  - "`cargo run -p oriterm_test_support --bin spec-coverage-report` exists, walks `plans/spec-conformance/catalog/*.md`, scans test directories (`oriterm_core/tests/`, `oriterm/tests/`, `oriterm_ui/tests/`, `oriterm_mux/tests/`, `crates/oriterm_test_support/`) for catalog row ID citations via both `// Catalog row: <ID>` comments AND `catalog_row_id: \"<ID>\"` const fields, and produces a per-stack absolute-verified-count table."
  - "Coverage report's gating metric is the ABSOLUTE count of `verified` rows per stack, NOT percentage. Reason: section 01 + the 04.9 continuous-delta detector keep adding rows as real captures surface uncataloged sequences, so the denominator grows over time. Absolute count is monotonic; percentage is not."
  - "Coverage report flags FALSE-VERIFIED rows (catalog says `verified` but no test cites the row ID) and UNCATALOGED citations (test cites a row ID that doesn't exist in any catalog file); `--check` mode fails CI on either."
  - "Cataloging safety net exists (section 04.9): `SpecHarness::feed()` is wrapped in `UncatalogedDetector` which records every distinct `(category, intermediates, final_byte)` tuple, compares against known catalog tuples, and appends misses to `plans/spec-conformance/uncataloged-backlog.md`. Non-empty backlog fails `--check` mode."
  - "BLOAT split: `oriterm/src/gpu/prepare/mod.rs` (504 lines) and `oriterm/src/gpu/prepare/dirty_skip/mod.rs` (506 lines) are split into submodules as the FIRST checkbox of any subsection that touches them — keeps each file under 500 lines"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row** mission criterion (delivers the harness; subsequent sections produce the rows) AND **Coverage report green** (delivers the generator; section 23 wires it into CI)"
inspired_by:
  - "ori_term `oriterm_core/tests/teseq/harness/runner.rs` — TeseqHarness pattern: `from_scenario(path)` + `run() -> ScenarioOutcome` with grid_text + cells + events"
  - "ori_term `oriterm/src/gpu/visual_regression/mod.rs` — visual_regression pattern: `headless_env_with_hinting()` + `render_to_pixels()` + `compare_with_reference()`"
  - "ori_term `crates/oriterm_test_support/src/tack_framework/spec.rs` — `ScenarioSpec` const-constructible pattern with function pointers (no closures)"
  - "wezterm `docs/escape-sequences.md` — per-row catalog format, used as the schema-freeze reference"
depends_on: ["03"]
# Structural note on 04 ↔ 05 coupling: subsections 04.4 (texture-render observer),
# 04.5 (sixel visual pilot, committing a golden), and 04.7 (schema freeze, which
# may key on golden-observer fields) are NOT reproducible until section 05 pins
# the software rasterizer, hinting mode, cell metrics, and tolerance. Options:
#   (a) land 04.1–04.3, 04.6 (non-visual pilot), 04.8 (coverage report walker)
#       BEFORE section 05, then land 04.4, 04.5, 04.7 (schema freeze) AFTER
#       section 05.
#   (b) land 04 end-to-end against the existing non-deterministic env with the
#       sixel pilot golden recaptured in 05.6 once the deterministic lane is in
#       place, accepting transient flakiness of 04.5 between 04 and 05 landing.
# Section 05 section-05.6 ("Migrate sixel_minimal pilot golden to the
# deterministic lane") is the apex point where the sixel pilot transitions
# from non-deterministic to deterministic. The catalog schema freeze in 04.7
# MUST NOT be finalized until 05.6 has landed — if 05.6 surfaces new required
# fields (e.g., `cell_metrics` pin, `pixel_tolerance_override` per row), the
# frozen schema has to include them. Section 04's completion checklist lists
# this dependency explicitly.
blocked_by_until_05_lands:
  - "04.5 (sixel visual pilot) runs on non-deterministic env only; may flake"
  - "04.7 (catalog row schema freeze) — do NOT finalize until 05.6 lands"
third_party_review:
  status: none
  updated: null
sections:
  - id: "04.1"
    title: "Design SpecHarness API + per-rung observers"
    status: not-started
  - id: "04.2"
    title: "Implement parser/dispatch/state observers"
    status: not-started
  - id: "04.3"
    title: "Implement renderable/frame-input/gpu-instance observers (BLOAT splits as touched)"
    status: not-started
  - id: "04.4"
    title: "Implement texture-render + golden-image observers (depends on 05's deterministic GPU env, but uses the existing non-deterministic env until 05 lands; section gates allow this)"
    status: not-started
  - id: "04.5"
    title: "Sixel visual pilot — drive minimal raster fill through every rung"
    status: not-started
  - id: "04.6"
    title: "DA1 non-visual pilot — drive query through effect transcript apex"
    status: not-started
  - id: "04.7"
    title: "Freeze catalog row schema + migrate section 01 catalog files"
    status: not-started
  - id: "04.8"
    title: "Coverage report generator binary (catalog walk + citation scan + monotonic absolute count)"
    status: not-started
  - id: "04.9"
    title: "Cataloging safety net — continuous delta detection for uncataloged sequences"
    status: not-started
  - id: "04.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "04.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement: 04.4 (after observer infrastructure — covers .1-.4),
# 04.6 (after both pilots run green — covers .5-.6), final in 04.N
---

# Section 04: Verification Chain Harness + Pilots + Coverage Report

**Status:** Not Started
**Goal:** Build the verification chain harness that section 08 onward will use to drive every catalog row to `verified` status. The harness extends the existing TeseqHarness + visual_regression patterns with per-rung observation: parser test, dispatch test, state test, renderable snapshot test, frame-input test, GPU instance test, texture render test, golden image test. Two pilot scenarios — one visual (sixel raster fill) and one non-visual (DA1 query) — exercise every applicable rung end-to-end and prove the harness works. The pilots' API requirements are then used to FREEZE the catalog row schema (which was provisional in section 01). The coverage report generator is the binary that walks the catalog files (via the shared `oriterm_test_support::catalog::parse_catalog_markdown` parser created by Section 01.3) and produces a per-stack absolute-verified-count table. **Gating metric is absolute count (monotonic), not percentage.** Percentage is advisory only — because section 01 and the continuous-discovery safety net (04.9) keep adding new rows, the denominator grows and percentages can drop while absolute counts stay flat or rise. CI gates on absolute counts per 04.8.

**Success Criteria:**
- [ ] `SpecHarness` API exists with per-rung observers
- [ ] Sixel visual pilot drives every visual rung (parser through golden) green
- [ ] DA1 non-visual pilot drives parser through effect apex green
- [ ] Catalog row schema frozen and section 01 catalogs migrated
- [ ] `spec-coverage-report` binary exists and produces correct per-stack absolute-verified-count table (monotonic gating metric — percentage is advisory only; see 04.8 for the full rationale)
- [ ] BLOAT splits applied as `gpu/prepare/{mod,dirty_skip/mod}.rs` are touched
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Connects to mission criteria: **Verification chain complete per row**, **Coverage report green**

**Context:** The harness is the load-bearing test infrastructure for the entire spec-conformance plan. Sections 08-20 each take a catalog file and grind every row from `implemented-unverified` to `verified` using this harness — without it, those sections have nothing to write tests against. Per Codex's "catalog breadth first, schema freeze after pilot" guidance, the catalog row format from section 01 is provisional; the pilots in this section discover what fields the harness actually needs to observe (e.g., does the row need an explicit `apex_layer` field or can it be inferred? Does the row need a `golden_path` field for visual sequences? What about per-platform variants?). Once the pilots run green, the schema is frozen and section 01's catalogs are migrated.

**Reference implementations:**
- **ori_term TeseqHarness** at `oriterm_core/tests/teseq/harness/runner.rs:39-124` — `TeseqHarness::from_scenario(path)` loads `.teseq` + `.toml` sidecar, constructs `Term<RecordedListener>`, applies `pre_feed`. `TeseqHarness::run() -> ScenarioOutcome` feeds bytes and captures grid_text, cells, cursor, events, mode. Pattern to extend with per-rung observation.
- **ori_term visual_regression** at `oriterm/src/gpu/visual_regression/mod.rs:69-141` — `headless_env_with_hinting()`, `render_to_pixels()`, `compare_with_reference()`. Provides the GPU rung infrastructure (texture render + golden compare). Section 05 makes this deterministic.
- **ori_term ScenarioSpec** at `crates/oriterm_test_support/src/tack_framework/spec.rs:74-132` — `const ScenarioSpec` with function pointers (parser, quit_path), no closures. Template for the catalog row → test scenario binding.

**Depends on:** Section 03 (Effect type exists for the effect-transcript observer).

**Section 04 ↔ Section 05 coupling (important):** Sections 04.4 (texture-render observer), 04.5 (sixel visual pilot committing a golden), and the FINALIZATION of 04.7 (catalog schema freeze) are NOT reproducible until Section 05 pins the software rasterizer, hinting mode, cell metrics, and tolerance. Ordering policy:

1. Land 04.1 (harness API), 04.2 (parser/dispatch/state observers), 04.3 (renderable/frame-input/gpu-instance observers + BLOAT splits), and 04.6 (DA1 non-visual pilot) BEFORE Section 05. These rungs never touch the GPU sample-accurate path.
2. Land 04.8 (coverage report walker + citation scanner) BEFORE Section 05 as well — it only reads markdown + test source files.
3. Land 04.4 (texture-render observer) and 04.5 (sixel pilot committing a golden) AFTER Section 05 lands its `headless_env_with_pinned_software_rasterizer()` and `GoldenLaneConfig`. Before 05 lands, 04.5 may be implemented against the existing non-deterministic env but its committed golden MUST be re-captured in 05.6 on the deterministic lane — the pre-05 capture is a throwaway.
4. The catalog row schema freeze in 04.7 MUST NOT be finalized until 05.6 has landed. If 05.6 surfaces new required fields (e.g. `cell_metrics`, `pixel_tolerance_override`, `hinting_mode_override`), the frozen schema has to include them. A provisional schema is acceptable during the 04.2–04.6 + 04.8 phase; the permanent `catalog/README.md` lock happens after 05.6.

This coupling is the reason the `depends_on` frontmatter lists only `03` for the first-phase work but includes a structured `blocked_by_until_05_lands` annotation.

---

## 04.1 Design SpecHarness API + per-rung observers

**File(s):** `crates/oriterm_test_support/src/spec_chain/mod.rs` (new), `crates/oriterm_test_support/src/spec_chain/api.rs` (new), `crates/oriterm_test_support/src/spec_chain/tests.rs` (new)

The `SpecHarness` is the main test entry point. It wraps `Term<EffectSink-aware>`, accepts a sequence (bytes or `.teseq` scenario), feeds it through every applicable rung, and exposes per-rung observation methods. Const-constructible scenario definitions following the tack ScenarioSpec pattern.

- [ ] Create `crates/oriterm_test_support/src/spec_chain/mod.rs` as the dispatch hub:
  ```rust
  //! Verification chain harness for spec conformance tests.
  //!
  //! Each catalog row gets a test that drives the sequence through every
  //! applicable rung (parser → dispatch → state → effect → renderable →
  //! frame-input → gpu-instance → texture → golden) and asserts the
  //! per-rung observation. A row is `verified` when every rung passes.
  //!
  //! See plans/spec-conformance/00-overview.md for the architecture.

  mod api;
  mod observers;
  mod scenario;

  pub use api::{SpecHarness, SpecOutcome, RungResult};
  pub use observers::*;
  pub use scenario::{SpecScenario, SpecScenarioBuilder, ApexLayer, RungName};

  #[cfg(test)]
  mod tests;
  ```
- [ ] Create `crates/oriterm_test_support/src/spec_chain/api.rs`:
  ```rust
  use oriterm_core::{Term, effect::*};
  use std::sync::Arc;

  pub struct SpecHarness {
      term: Term<crate::tests_support::CapturingEventListener>,
      processor: vte::ansi::Processor,
      effect_sink: Arc<oriterm_core::effect::QueueingEffectSink>,
      observed: SpecOutcome,
  }

  #[derive(Debug, Default, Clone)]
  pub struct SpecOutcome {
      pub parsed_actions: Vec<ParsedAction>,        // rung 1: parser tokenization
      pub dispatched_calls: Vec<DispatchCall>,      // rung 2: handler invocation
      pub final_grid_state: Option<GridSnapshot>,   // rung 3: state mutation
      pub effects_emitted: Vec<Effect>,             // rung 3 alt: effect transcript
      pub renderable_snapshot: Option<RenderableSnapshot>, // rung 4
      pub frame_input: Option<FrameInputSnapshot>,  // rung 5
      pub gpu_instances: Option<GpuInstanceSnapshot>, // rung 6
      pub texture_pixels: Option<Vec<u8>>,          // rung 7
      pub golden_match: Option<GoldenComparisonResult>, // rung 8
  }

  pub struct RungResult {
      pub rung_name: RungName,
      pub passed: bool,
      pub failure: Option<String>,
  }

  impl SpecHarness {
      pub fn new() -> Self { /* construct Term + sink */ }

      /// Feed bytes through the parser and dispatch.
      pub fn feed(&mut self, bytes: &[u8]) {
          self.processor.advance(&mut self.term, bytes);
          self.observed.effects_emitted.extend(self.effect_sink.take_pending());
      }

      /// Run a scenario through every rung up to its apex.
      pub fn run_scenario(&mut self, scenario: &SpecScenario) -> Vec<RungResult> {
          let mut results = Vec::new();
          for rung in scenario.applicable_rungs() {
              let result = self.run_rung(rung, scenario);
              let failed = !result.passed;
              results.push(result);
              if failed { break; }
          }
          results
      }

      // Per-rung observer methods (one per rung)
      pub fn observe_parser_rung(&self, expected: &ParserExpectation) -> RungResult { ... }
      pub fn observe_dispatch_rung(&self, expected: &DispatchExpectation) -> RungResult { ... }
      pub fn observe_state_rung(&self, expected: &StateExpectation) -> RungResult { ... }
      pub fn observe_effect_rung(&self, expected: &EffectExpectation) -> RungResult { ... }
      pub fn observe_renderable_rung(&self, expected: &RenderableExpectation) -> RungResult { ... }
      pub fn observe_frame_input_rung(&self, expected: &FrameInputExpectation) -> RungResult { ... }
      pub fn observe_gpu_instance_rung(&self, expected: &GpuInstanceExpectation) -> RungResult { ... }
      pub fn observe_texture_render_rung(&self, expected: &TextureExpectation) -> RungResult { ... }
      pub fn observe_golden_image_rung(&self, expected: &GoldenExpectation) -> RungResult { ... }
  }
  ```
- [ ] Create `crates/oriterm_test_support/src/spec_chain/scenario.rs`:
  ```rust
  use super::*;

  /// Const-constructible scenario definition (no closures, function pointers only).
  #[derive(Copy, Clone, Debug)]
  pub struct SpecScenario {
      pub catalog_row_id: &'static str,
      pub bytes: &'static [u8],
      pub apex_layer: ApexLayer,
      pub setup: &'static [u8], // optional pre-feed
      pub expectations: ScenarioExpectations,
  }

  #[derive(Copy, Clone, Debug, PartialEq, Eq)]
  pub enum ApexLayer {
      // Visual chain
      ParserOnly,
      Dispatch,
      State,
      Renderable,
      FrameInput,
      GpuInstance,
      TextureRender,
      GoldenImage,
      // Non-visual chain
      EffectPtyWrite,
      EffectClipboard,
      EffectHostTitle,
      EffectModeState,
      EffectPresentationCommit,
      // De-facto
      EffectAudio,
      EffectNotification,
  }

  #[derive(Copy, Clone, Debug, PartialEq, Eq)]
  pub enum RungName {
      Parser, Dispatch, State, Effect, Renderable, FrameInput,
      GpuInstance, TextureRender, GoldenImage,
  }
  ```
- [ ] Sibling tests in `crates/oriterm_test_support/src/spec_chain/tests.rs`:
  - `harness_constructs()`
  - `feed_advances_parser_and_captures_effects()`
  - `run_scenario_stops_at_first_failed_rung()`
  - `apex_layer_determines_applicable_rungs()`
- [ ] **Validation**: `cargo test -p oriterm_test_support --lib spec_chain::tests` passes; harness constructs without panic.

### Canonical `SpecScenario` recipe (for test authors writing new rows)

Every catalog row that reaches `verified` status is backed by a test that declares a `const SpecScenario` and drives it through the harness. This recipe is the canonical template — copy it when adding a new scenario. Place the test in `oriterm_core/tests/spec_chain/<stack>/<row_id_kebab>.rs` (or the equivalent path under the relevant crate's `tests/`).

```rust
// oriterm_core/tests/spec_chain/ecma_48/ecma48_cup.rs

//! Catalog row: ECMA48-CUP (ECMA-48 §8.3.21)
//! Apex: state-snapshot
//!
//! Cursor Position (CSI Ps ; Ps H). Moves the cursor to (row, col),
//! 1-based. Interacts with DECOM (origin mode) — origin mode test
//! lives in teseq/csi_cursor.rs as documented in the catalog row notes.

use oriterm_test_support::spec_chain::*;

/// Canonical declaration: `const SCENARIO` with
///   - `catalog_row_id`:    stable ID that citation scan + coverage report cross-check
///   - `bytes`:              the raw PTY input to feed
///   - `apex_layer`:         the highest rung the test drives
///   - `setup`:              optional pre-feed bytes (e.g. put term in origin mode first;
///                            empty slice `b""` when no setup needed)
///   - `expectations`:       per-rung ScenarioExpectations struct (state, effect, etc.)
const SCENARIO: SpecScenario = SpecScenario {
    catalog_row_id: "ECMA48-CUP",
    bytes: b"\x1b[5;10H",
    apex_layer: ApexLayer::State,
    setup: b"",
    expectations: ScenarioExpectations {
        // Parser rung: tokenizer sees one CSI 'H' with params [5, 10]
        parser: Some(ParserExpectation::csi_with_params('H', &[5, 10])),
        // Dispatch rung: TermHandler::goto called with (line=4, col=9)
        // (converted from 1-based to 0-based by dispatch)
        dispatch: Some(DispatchExpectation::method("goto")),
        // State apex: cursor position asserted on grid snapshot
        state: Some(StateExpectation::cursor_at(/*line*/ 4, /*col*/ 9)),
        // Non-state rungs not applicable for this apex
        effect: None,
        renderable: None,
        frame_input: None,
        gpu_instance: None,
        texture: None,
        golden: None,
    },
};

#[test]
fn ecma48_cup_basic() {
    let mut harness = SpecHarness::new();
    // Optional: apply setup bytes (empty here)
    if !SCENARIO.setup.is_empty() {
        harness.feed(SCENARIO.setup);
    }
    // Run the scenario — harness drives through every applicable rung
    // up to the declared apex layer and stops at the first failure.
    let results = harness.run_scenario(&SCENARIO);
    // Standard rung assertion: every rung in the apex chain must pass.
    for result in &results {
        assert!(
            result.passed,
            "rung {:?} failed for {}: {:?}",
            result.rung_name, SCENARIO.catalog_row_id, result.failure
        );
    }
    // Apex pin: the last rung run must match the declared apex layer.
    assert_eq!(
        results.last().map(|r| r.rung_name),
        Some(RungName::from_apex(SCENARIO.apex_layer)),
        "apex layer did not match last rung executed",
    );
}
```

**Recipe rules:**

1. **`const SCENARIO`** is required — it enables the citation scanner (04.8) to find `catalog_row_id: "…"` via a literal grep. Do not build scenarios dynamically inside `#[test]` functions; the scanner will miss them and the row will be flagged false-verified.
2. **Apex alignment**: `scenario.apex_layer` MUST match the `Apex layer` column in the catalog row. The coverage report walker cross-checks this.
3. **Setup is a slice** — `b""` for no setup, otherwise a byte literal. No closures (const-friendly).
4. **Every rung assertion uses `result.passed`** — do not reach into `.failure` for partial success; partial success is a test bug.
5. **The apex pin** at the end (`assert_eq!(results.last(), ...)`) guards against the harness silently stopping early on an unreported rung failure.
6. For visual-apex scenarios (`ApexLayer::GoldenImage`), the golden PNG path MUST live under `crates/oriterm_test_support/tests/references/spec_chain/<stack>/<row_id>.png` — the coverage report scans this directory for orphan goldens.

This recipe is the contract between the harness and the rest of sections 08-20 + 26. Do not deviate without updating `catalog/README.md` + the harness tests.

---

## 04.2 Implement parser/dispatch/state observers

**File(s):** `crates/oriterm_test_support/src/spec_chain/observers/{parser,dispatch,state,effect}.rs` (new)

Each observer takes the captured `SpecOutcome` and an `Expectation` struct, and returns `RungResult`. Observers are pure functions of the outcome — no side effects, no Term access — which makes them composable and easy to test in isolation.

- [ ] `observers/parser.rs`: `observe_parser(outcome, expected) -> RungResult` — assert that the parser tokenized the expected sequence and extracted the expected parameters. Compare against `outcome.parsed_actions`.
- [ ] `observers/dispatch.rs`: `observe_dispatch(outcome, expected) -> RungResult` — assert that the expected handler method was called with the expected arguments. Compare against `outcome.dispatched_calls` (which the harness records via a wrapper around the actual handler).
- [ ] `observers/state.rs`: `observe_state(outcome, expected) -> RungResult` — assert that the final terminal state matches expected (cells, cursor, modes, palette, etc.). Compare against `outcome.final_grid_state`.
- [ ] `observers/effect.rs`: `observe_effect(outcome, expected) -> RungResult` — assert that the expected Effect was emitted (or that NO effect was emitted, depending on the expectation). Compare against `outcome.effects_emitted`.
- [ ] Sibling tests for each observer.
- [ ] **Validation**: each observer's tests pass in isolation. Observers correctly distinguish "expected matched" vs "expected absent" vs "expected missing".

---

## 04.3 Implement renderable/frame-input/gpu-instance observers (BLOAT splits as touched)

**File(s):** `crates/oriterm_test_support/src/spec_chain/observers/{renderable,frame_input,gpu_instance}.rs` (new), `oriterm/src/gpu/prepare/mod.rs` (split), `oriterm/src/gpu/prepare/dirty_skip/mod.rs` (split)

These observers operate on the post-state pipeline: `RenderableContent` (rung 4), `FrameInput` (rung 5), and the GPU instance buffers (rung 6). The harness captures these by exposing observation hooks in `gpu/prepare/mod.rs` and `gpu/prepare/dirty_skip/mod.rs` — both files are at the BLOAT limit (504 and 506 lines), so they MUST be split before we add the hooks.

- [ ] **FIRST CHECKBOX (BLOAT split prerequisite)**: Split `oriterm/src/gpu/prepare/mod.rs` (504 lines) into submodules. Identify natural seams (e.g., separate `cell_emit`, `cursor_emit`, `image_emit` into individual files). Each new file under 500 lines. Verify no behavior change with `./test-all.sh`.
- [ ] **FIRST CHECKBOX (BLOAT split prerequisite)**: Split `oriterm/src/gpu/prepare/dirty_skip/mod.rs` (506 lines) similarly. Identify the natural seams and extract submodules.
- [ ] After splits, add observation hooks: `gpu::prepare::observe_renderable(content) -> RenderableSnapshot` and similar for FrameInput and GPU instances. These are debug-only paths gated behind `#[cfg(any(test, debug_assertions))]` to avoid hot-path overhead in release builds.
- [ ] `observers/renderable.rs`: `observe_renderable(outcome, expected) -> RungResult` — asserts cells, palette, image placements, hyperlinks, cursor, mode bits all match expected.
- [ ] `observers/frame_input.rs`: `observe_frame_input(outcome, expected) -> RungResult` — asserts FrameInput composition (viewport, cell metrics, hovered cell, prompt markers, etc.) matches expected.
- [ ] `observers/gpu_instance.rs`: `observe_gpu_instance(outcome, expected) -> RungResult` — asserts the GPU instance buffer contents match expected (vertex count, UV coords, colors, z-order). Use the existing `oriterm/src/gpu/instance_writer/` infrastructure.
- [ ] Sibling tests for each observer.
- [ ] **Validation**: BLOAT files now under 500 lines; observer tests pass.
- [ ] **TPR checkpoint** — `/tpr-review` covering 04.1–04.4 (harness API + observer infrastructure). Catches design issues before pilots are written against them.

---

## 04.4 Implement texture-render + golden-image observers (LAND AFTER Section 05)

**File(s):** `crates/oriterm_test_support/src/spec_chain/observers/{texture,golden}.rs` (new)

**Ordering gate:** This subsection MUST land AFTER Section 05's deterministic golden lane is in place (`headless_env_with_pinned_software_rasterizer()` + `GoldenLaneConfig`). The texture-render observer reads back GPU pixels; the golden observer compares against a committed PNG. Without 05's adapter pin, hinting pin, cell metrics pin, and tolerance pin, any golden committed here will flake on CI or another developer's machine. Section 04's first-phase work (04.1–04.3, 04.6, 04.8) does not depend on this subsection; this subsection is the bridge from the pilot-era harness to the verified-apex-era harness and should be interleaved with 05.6.

- [ ] `observers/texture.rs`: `observe_texture_render(outcome, expected) -> RungResult` — uses `render_frame_cached()` to render the FrameInput onto an offscreen target, reads back pixels, asserts pixel buffer matches expected. Must be invoked via `headless_env_with_pinned_software_rasterizer()` from Section 05.
- [ ] `observers/golden.rs`: `observe_golden_image(outcome, expected_path) -> RungResult` — calls `compare_with_reference_strict(name, pixels, w, h, config)` from Section 05.5. Returns `RungResult::pass()` on exact match, `failure(diff_summary)` on any mismatch.
- [ ] Sibling tests (in the `crates/oriterm_test_support/src/spec_chain/observers/tests.rs` file): use Section 05's pinned env; do NOT use the legacy `headless_env_full()` entry point.
- [ ] **Validation**: texture render observer produces deterministic pixel readback for a known input across TWO consecutive runs on the same machine. Golden observer correctly matches identical inputs and rejects single-pixel changes.

---

## 04.5 Sixel visual pilot — drive minimal raster fill through every rung (LAND AFTER Section 05)

**File(s):** `oriterm_core/tests/spec_chain/pilots/sixel_minimal.rs` (new), `oriterm_core/tests/spec_chain/pilots/mod.rs` (new), `oriterm_core/tests/spec_chain/main.rs` (new), `crates/oriterm_test_support/tests/references/spec_chain/pilots/sixel_minimal.png` (golden, captured via `ORITERM_UPDATE_GOLDEN=1`)

**Ordering gate:** This subsection lands AFTER Section 05's deterministic lane. The committed `sixel_minimal.png` golden is captured via `headless_env_with_pinned_software_rasterizer(GoldenLaneConfig::SPEC_DEFAULT)` — NOT the legacy non-deterministic env. Section 05.6 ("Migrate sixel_minimal pilot golden to the deterministic lane") is the apex coordination point: it re-captures the golden on the deterministic lane and verifies the test passes on back-to-back runs with 0-pixel diff. If 04.5 is implemented before 05 for sequencing reasons, its committed golden is considered THROWAWAY and replaced by 05.6.

The sixel visual pilot is the canonical visual chain test. It feeds a minimal sixel raster fill (a few sixel bytes that paint a small solid rectangle) and asserts every rung from parser to golden image passes. This proves the harness can drive a visual sequence end-to-end.

- [ ] Create the test main.rs:
  ```rust
  // oriterm_core/tests/spec_chain/main.rs
  mod pilots;
  ```
- [ ] Create `oriterm_core/tests/spec_chain/pilots/mod.rs`:
  ```rust
  pub mod sixel_minimal;
  pub mod da1_query;
  ```
- [ ] Create `oriterm_core/tests/spec_chain/pilots/sixel_minimal.rs`:
  ```rust
  use oriterm_test_support::spec_chain::*;

  /// Pilot scenario: minimal sixel raster fill.
  ///
  /// Catalog row: SIXEL-DCS-Q-MINIMAL
  /// Apex: GoldenImage
  ///
  /// Drives a minimal sixel sequence (DCS q ... ST that paints a solid
  /// rectangle) through every visual rung and asserts each rung passes.
  /// This pilot establishes the harness MVP for visual sequences.
  const SCENARIO: SpecScenario = SpecScenario {
      catalog_row_id: "SIXEL-DCS-Q-MINIMAL",
      bytes: b"\x1bPq#0;2;100;100;100#0!10~-#0!10~\x1b\\",
      apex_layer: ApexLayer::GoldenImage,
      setup: b"",
      expectations: /* per-rung expectations */,
  };

  #[test]
  fn sixel_minimal_drives_every_rung_green() {
      let mut harness = SpecHarness::new();
      let results = harness.run_scenario(&SCENARIO);

      // Every rung must pass.
      for result in &results {
          assert!(result.passed, "rung {:?} failed: {:?}", result.rung_name, result.failure);
      }

      // Apex (golden image) must be the last rung run.
      assert_eq!(results.last().map(|r| r.rung_name), Some(RungName::GoldenImage));
  }
  ```
- [ ] Capture the golden:
  ```bash
  ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm_core --test spec_chain pilots::sixel_minimal::sixel_minimal_drives_every_rung_green
  ```
- [ ] Verify the test passes when run again without `ORITERM_UPDATE_GOLDEN`:
  ```bash
  cargo test -p oriterm_core --test spec_chain pilots::sixel_minimal
  ```
- [ ] **Validation**: pilot test passes; every rung observed; golden image captured under `crates/oriterm_test_support/tests/references/spec_chain/pilots/sixel_minimal.png` (or wherever the canonical golden directory ends up).

---

## 04.6 DA1 non-visual pilot — drive query through effect transcript apex

**File(s):** `oriterm_core/tests/spec_chain/pilots/da1_query.rs` (new)

The DA1 (Device Attributes Primary) non-visual pilot proves the effect transcript apex works. DA1 is `CSI c` — the terminal responds with a `CSI ? ... c` reply that identifies its capabilities. The reply is a `PtyEffect::Write { kind: PtyWriteKind::DeviceAttribute }` — the apex of the non-visual chain.

- [ ] Create `oriterm_core/tests/spec_chain/pilots/da1_query.rs`:
  ```rust
  use oriterm_test_support::spec_chain::*;
  use oriterm_core::effect::*;

  /// Pilot scenario: DA1 device attribute query.
  ///
  /// Catalog row: ECMA48-DA1
  /// Apex: EffectPtyWrite (PtyWriteKind::DeviceAttribute)
  ///
  /// Drives a CSI c query and asserts the harness observes the
  /// expected PtyEffect::Write with DeviceAttribute kind. This pilot
  /// establishes the harness MVP for non-visual sequences with PTY-reply apex.
  #[test]
  fn da1_query_drives_to_effect_apex() {
      let mut harness = SpecHarness::new();
      harness.feed(b"\x1b[c");

      // Parser rung: assert CSI c was parsed
      let parsed = harness.observe_parser_rung(&ParserExpectation::csi('c'));
      assert!(parsed.passed);

      // Dispatch rung: assert identify_terminal handler was invoked
      let dispatched = harness.observe_dispatch_rung(&DispatchExpectation::method("identify_terminal"));
      assert!(dispatched.passed);

      // Effect apex: PtyEffect::Write { kind: PtyWriteKind::DeviceAttribute, ... }
      // The reply bytes should be the VT420 + sixel attribute string per
      // oriterm_core/src/term/handler/status.rs.
      let effect_apex = harness.observe_effect_rung(&EffectExpectation::pty_write(
          PtyWriteKind::DeviceAttribute,
          // expected reply prefix (the exact bytes are parameter-dependent)
          b"\x1b[?64",
      ));
      assert!(effect_apex.passed, "DA1 reply effect not observed: {:?}", effect_apex.failure);
  }
  ```
- [ ] **Validation**: pilot test passes; effect transcript correctly captures the reply; PtyWriteKind discriminator is observable.
- [ ] **TPR checkpoint** — `/tpr-review` covering 04.5–04.6 (both pilots green). Validates the harness API works end-to-end before the schema freeze locks it in.

---

## 04.7 Freeze catalog row schema + migrate section 01 catalog files (LAND AFTER Section 05.6)

**File(s):** `plans/spec-conformance/catalog/README.md` (extended — stub was created by Section 01.10), `plans/spec-conformance/catalog/*.md` (migrated)

**Ownership note (Phase 4 TPR fix for section 01):** Section 01.10 creates a STUB `catalog/README.md` (~60 lines) documenting the catalog directory structure, authority-ladder index, and schema version. This subsection EXTENDS that stub with the frozen schema reference — it does NOT create a new file and MUST NOT overwrite the existing stub's ownership table, authority-ladder pointer, or `schema_version` front-matter. Per the Section 01 / Section 04 ownership split documented in both sections' bodies: Section 01 owns the stub; Section 04.7 owns the frozen schema extension; no file is re-created.

**Ordering gate:** Schema freeze MUST wait until Section 05.6 lands. Reason: the deterministic golden lane may surface new required fields (e.g. `cell_metrics_pin`, `hinting_mode_override`, `pixel_tolerance_override` per row, or a `golden_env_config` column referencing `GoldenLaneConfig` presets). Freezing the schema before 05.6 risks immediate invalidation. During the pre-05 phase, section 01's provisional schema REMAINS provisional and catalog files stay in their section-01 form; this subsection performs the final freeze in a single pass once 05.6 has landed.

After both pilots run green AND Section 05.6 has migrated the sixel pilot to the deterministic lane, the harness API is stable and the catalog row schema can be frozen. Extend `catalog/README.md` (the stub created by Section 01.10) with the canonical row format reference. Then migrate every catalog file from section 01's provisional schema to the frozen one.

- [ ] Extend `plans/spec-conformance/catalog/README.md` BELOW the existing stub's "Schema evolution" section. Do NOT rewrite or replace the Section 01 stub content above that boundary. Add:
  - The canonical catalog row table format (markdown table with explicit column order)
  - The required columns: `ID`, `Spec source`, `Sequence`, `Description`, `Implementation`, `Apex layer`, `Test chain`, `Verification`, `De-facto ref`, `Notes` (the column name is `De-facto ref` to match the SSOT in `plans/spec-conformance/00-overview.md` and Section 01.1.e; do NOT use the longer form `De-facto reference` — that would create a column-name DRIFT)
  - The `ApexLayer` enum values (matching `spec_chain::ApexLayer`)
  - The `RungName` enum values
  - The verification status enum values (`missing` / `stub` / `implemented-unverified` / `verified-partial` / `verified` / `verified-with-deviation`)
  - How a row is added (workflow for new sequences)
  - How a row is migrated to `verified` (test chain requirements)
  - Update the stub's front-matter `schema_version: "0.1-provisional"` to `schema_version: "1.0"` as the very last edit of this subsection (the migration is atomic — all catalog files flip to 1.0 in the same commit).
- [ ] Walk every catalog file from section 01 (`ecma-48.md`, `xterm-ctlseqs.md`, `dec-private-modes.md`, `osc.md`, `sixel.md`, `kitty-graphics.md`, `kitty-keyboard.md`, `iterm2.md`, `mode-2026.md`, `unicode-subcell.md`, `mouse.md`, `charsets.md`, `audio-print.md`, `shell-integration.md`, `historical.md`, `de-facto-behaviors.md`). Migrate each row to the frozen schema. Every row's front-matter `schema_version` flips from `0.1-provisional` to `1.0` in this subsection. The `Verification` column may need refinement based on what the pilots discovered.
- [ ] **Validation**: every catalog file has `schema_version: "1.0"`; `catalog/README.md` is the single source of truth for the format AND still contains the Section 01.10 stub content above the extension boundary (authority-ladder index, files table, ownership note).

---

## 04.8 Coverage report generator binary (catalog walk + citation scan + monotonic absolute count)

**File(s):** `crates/oriterm_test_support/src/bin/spec_coverage_report.rs` (new), `crates/oriterm_test_support/src/spec_chain/coverage/mod.rs` (new), `crates/oriterm_test_support/src/spec_chain/coverage/scan.rs` (new), `crates/oriterm_test_support/src/spec_chain/coverage/tests.rs` (new). **Note (Phase 4 section-01 review iteration-8 TPR-01-001-codex fix):** this subsection does NOT create its own `walk.rs` or its own catalog markdown parser. Catalog parsing is owned by `crates/oriterm_test_support/src/catalog/mod.rs`, which is created by Section 01.3 (Mechanical `catalog_coverage_check`) and is the SSOT for markdown-table parsing and tuple canonicalization per `.claude/rules/impl-hygiene.md` §SSOT / §Algorithmic DRY. `spec_coverage_report` imports `oriterm_test_support::catalog::{parse_catalog_markdown, Row}` (the public API exposed by 01.3) and consumes it as a library — no duplicated parser logic. 04.8's own code is limited to: (1) aggregation — grouping rows by stack, counting `Verification` statuses per stack, computing the per-stack absolute count; (2) citation scan — walking the test directories for catalog-row-ID references; (3) cross-check — verifying that `verified` rows in the catalog have at least one test citation AND that test citations resolve to real catalog rows.

The coverage report has TWO responsibilities:

1. **Catalog walk**: parse every `plans/spec-conformance/catalog/*.md`, extract the row ID + `Verification` column, count by status.
2. **Citation scan**: walk every test file under `oriterm_core/tests/`, `oriterm/tests/`, `oriterm_mux/tests/`, `oriterm_ui/tests/`, and `crates/oriterm_test_support/src/` looking for catalog row IDs cited in test comments (e.g. `// Catalog row: ECMA48-CUP`) and in `SpecScenario::catalog_row_id` const fields. Cross-check: a row marked `verified` in the catalog MUST have at least one test that cites it by ID. If the catalog says `verified` but no test cites the row, the report flags the row as a **false verified** and fails CI. If a test cites a row ID that doesn't exist in any catalog file, the report flags the row as **uncataloged** and fails CI (this is the cataloging safety net from section 04.9, below).

**Monotonicity semantic: absolute verified count, NOT percentage.** Because section 01 + the continuous-discovery safety net (04.9) keep adding new rows as real captures surface uncataloged sequences, the denominator grows over time — so percentage can DROP even when no row regresses. The CI-gating metric is the **absolute count of `verified` rows per stack**, which is strictly monotonic increasing (except during intentional demotions documented in the PR description). The percentage is still printed for human readability but is advisory, not gating.

- [ ] Create `crates/oriterm_test_support/src/spec_chain/coverage/mod.rs`:
  ```rust
  //! Coverage report generator library.
  //!
  //! Consumes the shared catalog parser at `crate::catalog::parse_catalog_markdown`
  //! (created by Section 01.3) and scans test directories for catalog row ID
  //! citations. Produces a `CoverageReport` with per-stack absolute-count metrics.
  //!
  //! Catalog parsing is delegated — this module does NOT own markdown-table
  //! parsing. See `crates/oriterm_test_support/src/catalog/mod.rs` for the SSOT
  //! parser (single source of truth for both `catalog_coverage_check` and this
  //! binary, per impl-hygiene.md §SSOT / §Algorithmic DRY).
  use crate::catalog::{parse_catalog_markdown, Row};

  mod scan;
  pub use scan::{scan_test_citations, Citation};

  pub struct CoverageReport {
      pub stacks: Vec<StackSummary>,
      pub false_verified: Vec<String>,   // verified in catalog but not cited
      pub uncataloged: Vec<String>,      // cited in tests but not in catalog
      pub per_stack_absolute_verified: std::collections::BTreeMap<String, u32>,
  }

  pub struct StackSummary {
      pub stack: String,
      pub verified: u32,
      pub implemented_unverified: u32,
      pub stub: u32,
      pub missing: u32,
  }

  impl CoverageReport {
      /// Build a coverage report by walking `catalog_dir` via the shared
      /// `parse_catalog_markdown` parser and scanning `test_dirs` for citations.
      ///
      /// Returns `Err` if the catalog directory cannot be read OR if any catalog
      /// markdown file fails schema validation. **Error propagation is load-bearing**
      /// (Section 01 iteration-9 TPR-01-001-gemini + iteration-10 TPR-01-001-codex
      /// fix): `.unwrap_or_default()` is explicitly banned because it would silently
      /// drop parser errors and let catalog schema drift in.
      pub fn build(
          catalog_dir: &std::path::Path,
          test_dirs: &[std::path::PathBuf],
      ) -> Result<Self, anyhow::Error> {
          // 1. Walk plans/spec-conformance/catalog/*.md via the shared parser.
          //    Propagate every parser error via `?` — never swallow with
          //    `.unwrap_or_default()`.
          let mut rows: Vec<Row> = Vec::new();
          for entry in std::fs::read_dir(catalog_dir)? {
              let entry = entry?;
              if entry.path().extension().map_or(false, |ext| ext == "md") {
                  let file_rows = parse_catalog_markdown(&entry.path())
                      .map_err(|e| anyhow::anyhow!(
                          "catalog parse failed for {}: {e}",
                          entry.path().display()
                      ))?;
                  rows.extend(file_rows);
              }
          }
          // 2. Scan test directories for citations
          let citations = scan_test_citations(test_dirs)?;
          // 3. Aggregate, cross-check, return
          Ok(Self::aggregate(rows, citations))
      }

      fn aggregate(rows: Vec<Row>, citations: Vec<Citation>) -> Self { /* ... */ }
      pub fn has_regression(&self, baseline: &CoverageBaseline) -> bool { /* ... */ }
      pub fn print_table(&self) { /* ... */ }
  }

  #[cfg(test)]
  mod tests;
  ```
- [ ] Do NOT create `crates/oriterm_test_support/src/spec_chain/coverage/walk.rs`. The catalog-walk logic is owned by `crates/oriterm_test_support/src/catalog/mod.rs` (created by Section 01.3) and consumed by both binaries (`catalog_coverage_check` from 01.3 and `spec_coverage_report` from 04.8). A separate `walk.rs` under `spec_chain/coverage/` would violate SSOT — two markdown-table parsers for the same file set.
- [ ] Create `crates/oriterm_test_support/src/spec_chain/coverage/scan.rs` — walks the test directories via `walkdir`, greps every `.rs` file for `// Catalog row: ([A-Z0-9-]+)` AND `catalog_row_id: "([A-Z0-9-]+)"`, produces `Vec<Citation>` with `{ catalog_row_id, test_file_path }`.
- [ ] Create `crates/oriterm_test_support/src/bin/spec_coverage_report.rs`:
  ```rust
  //! Walks plans/spec-conformance/catalog/*.md AND scans
  //! oriterm_core/tests/, oriterm/tests/, oriterm_ui/tests/,
  //! oriterm_mux/tests/, and crates/oriterm_test_support/ for catalog row
  //! ID citations. Prints a per-stack absolute-verified-count table +
  //! flags false-verified rows + flags uncataloged citations.
  //!
  //! Gating metric: absolute count of `verified` rows (monotonic). A row
  //! dropping from `verified` to any lower status FAILS CI. The percentage
  //! is advisory only.
  //!
  //! Run: `cargo run -p oriterm_test_support --bin spec-coverage-report`
  //! Check: `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check`
  //!
  //! Catalog parsing is owned by `oriterm_test_support::catalog::parse_catalog_markdown`
  //! (created by Section 01.3). This binary does NOT re-implement markdown-table
  //! parsing; it only does aggregation, citation scanning, and cross-check.

  use std::path::PathBuf;
  use oriterm_test_support::spec_chain::coverage::{CoverageReport, CoverageBaseline};

  fn main() -> anyhow::Result<()> {
      let workspace_root: PathBuf = /* find via CARGO_MANIFEST_DIR ancestors */;
      let catalog_dir = workspace_root.join("plans/spec-conformance/catalog");
      let test_roots: Vec<PathBuf> = vec![
          workspace_root.join("oriterm_core/tests"),
          workspace_root.join("oriterm/tests"),
          workspace_root.join("oriterm_ui/tests"),
          workspace_root.join("oriterm_mux/tests"),
          workspace_root.join("crates/oriterm_test_support/src"),
          workspace_root.join("crates/oriterm_test_support/tests"),
      ];
      // CoverageReport::build() calls the SSOT parser at
      // oriterm_test_support::catalog::parse_catalog_markdown internally; errors
      // from the parser propagate here via ? rather than being swallowed (iter-9
      // TPR-01-001-gemini error-propagation fix — no .unwrap_or_default()).
      let report = CoverageReport::build(&catalog_dir, &test_roots)?;
      report.print_table();

      if std::env::args().any(|a| a == "--check") {
          let baseline = CoverageBaseline::load(
              &workspace_root.join("plans/spec-conformance/coverage-baseline.toml"),
          )?;
          let mut fail = false;
          if !report.false_verified.is_empty() {
              eprintln!("FALSE VERIFIED (catalog says verified but no test cites):");
              for row in &report.false_verified { eprintln!("  {row}"); }
              fail = true;
          }
          if !report.uncataloged.is_empty() {
              eprintln!("UNCATALOGED CITATIONS (test cites row ID not in catalog):");
              for row in &report.uncataloged { eprintln!("  {row}"); }
              fail = true;
          }
          if report.has_regression(&baseline) {
              eprintln!("REGRESSION: absolute verified count dropped for one or more stacks");
              fail = true;
          }
          if fail { std::process::exit(1); }
      }
      Ok(())
  }
  ```
- [ ] **Error propagation (Phase 4 section-01 iteration-9 TPR-01-001-gemini fix — no swallowed errors):** `CoverageReport::build()` MUST propagate parser errors via `Result<Self, anyhow::Error>` rather than silently swallowing them with `.unwrap_or_default()`. If a catalog markdown file fails to parse (bad 10-column schema, unrecognized column name, malformed row), the build fails loudly with the file path and the parser's error message. The earlier `.flat_map(|e| parse_catalog_markdown(&e.path()).unwrap_or_default())` pattern was a LEAK (swallowed error) and is explicitly forbidden — use a fold that accumulates errors and returns the first failure. Section 01.3.a's `parse_catalog_markdown` is documented to return `Result<Vec<Row>, Error>` and this binary respects that signature; consumers must NEVER silently drop errors from the shared parser.
- [ ] Sibling tests in `crates/oriterm_test_support/src/spec_chain/coverage/tests.rs`:
  - `coverage_report_build_invokes_shared_catalog_parser()` — assert the shared `oriterm_test_support::catalog::parse_catalog_markdown` is the only code path used for markdown parsing (no inline parsing, no duplicated regex)
  - `coverage_report_build_propagates_parser_errors()` — feed a malformed catalog file, assert `CoverageReport::build()` returns `Err(_)` with the file path in the error message (iter-9 TPR-01-001-gemini error-propagation pin)
  - `scan_test_citations_finds_comment_citation()` (`// Catalog row: ECMA48-CUP`)
  - `scan_test_citations_finds_const_field_citation()` (`catalog_row_id: "ECMA48-CUP"`)
  - `false_verified_flagged_when_catalog_verified_but_no_test_cites()`
  - `uncataloged_flagged_when_test_cites_but_catalog_missing()`
  - `has_regression_fails_when_absolute_verified_drops()`
  - `has_regression_passes_when_absolute_verified_holds_steady_despite_new_rows_added()` (the monotonicity semantic)
- [ ] Add Cargo binary entry to `crates/oriterm_test_support/Cargo.toml`:
  ```toml
  [[bin]]
  name = "spec-coverage-report"
  path = "src/bin/spec_coverage_report.rs"
  ```
- [ ] **Validation**: `cargo run -p oriterm_test_support --bin spec-coverage-report` produces the expected per-stack table reflecting current catalog state. `--check` mode fails on (a) manually-injected regression in the absolute-verified count, (b) a fabricated `verified` row with no test citation, AND (c) a fabricated test citation to a nonexistent row. Passes on a clean run.

---

## 04.9 Cataloging safety net — continuous delta detection

**File(s):** `crates/oriterm_test_support/src/spec_chain/uncataloged/mod.rs` (new), `crates/oriterm_test_support/src/spec_chain/uncataloged/tests.rs` (new), `plans/spec-conformance/uncataloged-backlog.md` (new — starts empty)

The catalog is bootstrapped in section 01 via a one-time bottom-up scan + top-down spec walk. If a real-world sequence is missed by both passes, it never gets a row and the coverage report never sees it. The continuous-delta detector catches this:

- **At harness replay time**: every `SpecHarness::feed()` call feeds bytes through a wrapped parser that records every distinct `(category, intermediates, final_byte, ...)` tuple. After the scenario completes, the harness compares the observed tuples against the catalog's known tuples. Any unknown tuple is an **uncataloged sequence** and is written to `plans/spec-conformance/uncataloged-backlog.md` as a TODO for section 01 follow-up.
- **At committed-capture scan time**: the notcurses-demo harness (section 21) and real-app harness (section 22) replay committed PTY captures. Every replay runs the same uncataloged-sequence detector and any hit is flagged.
- **At CI time**: section 23's `spec-coverage-report --check` mode fails CI if `uncataloged-backlog.md` is non-empty. This prevents "forgetting" to add a new catalog row when a real capture surfaces a new sequence.

- [ ] Define `SequenceTuple` — a canonicalized form of the `(category, intermediates, final_byte, param_hash?)` that uniquely identifies a catalog row.
- [ ] Build a hashset of known tuples by walking `plans/spec-conformance/catalog/*.md` and parsing the `Sequence` column.
- [ ] Wrap `SpecHarness::feed()` in a `UncatalogedDetector` that records every distinct tuple observed during the scenario.
- [ ] After `run_scenario()`, the detector compares observed tuples against the known set and appends misses to `plans/spec-conformance/uncataloged-backlog.md` (with context: scenario ID, capture file, first-seen byte offset).
- [ ] CI gate: `spec-coverage-report --check` fails if `uncataloged-backlog.md` has any rows that weren't explicitly acknowledged by an accompanying catalog-update PR.
- [ ] Sibling tests:
  - `known_tuple_is_not_flagged()`
  - `unknown_tuple_is_appended_to_backlog()`
  - `backlog_with_rows_fails_check_mode()`
- [ ] **Validation**: feed a fabricated "unknown" CSI sequence through the harness; verify it lands in the backlog file; verify `--check` mode fails; remove the row from the backlog; verify `--check` mode passes.

---

## 04.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 04.N Completion Checklist

- [ ] Failing test matrix written FIRST (TDD): pilot tests in 04.5 + 04.6 written before observer wiring; observer tests in 04.2/04.3/04.4 written before observer implementation
- [ ] **Matrix dimensions**: rung × scenario type (visual/non-visual) × apex layer × verification status — pilots cover both visual chain (8 rungs to GoldenImage apex) and non-visual chain (3-4 rungs to EffectPtyWrite apex)
- [ ] **Semantic pin**: pilots are the permanent regression guard — `sixel_minimal_drives_every_rung_green` and `da1_query_drives_to_effect_apex` must continue passing for the lifetime of the plan. They're the first tests that prove the harness works; they're also the canary if a future change breaks rung observation.
- [ ] SpecHarness API exists with all 9 observer methods
- [ ] All 9 observer implementations exist with sibling tests
- [ ] BLOAT splits applied: `oriterm/src/gpu/prepare/mod.rs` and `oriterm/src/gpu/prepare/dirty_skip/mod.rs` are now under 500 lines (verified by `wc -l`)
- [ ] **Section 04 ↔ 05 coupling respected**: 04.1–04.3, 04.6, 04.8, 04.9 land in Phase 1a (before 05); 04.4, 04.5, 04.7-finalize land in Phase 1b (after 05.6)
- [ ] Sixel visual pilot test passes on the deterministic lane (after 05.6); golden captured under `tests/references/spec_chain/pilots/sixel_minimal.png` via `headless_env_with_pinned_software_rasterizer`
- [ ] DA1 non-visual pilot test passes
- [ ] `plans/spec-conformance/catalog/README.md` exists with the frozen schema documentation (frozen AFTER 05.6)
- [ ] All catalog files migrated to the frozen schema
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report` produces a sane per-stack table with ABSOLUTE verified counts (not just percentages)
- [ ] Coverage report walks BOTH catalog files AND test source files (grep for `// Catalog row: <ID>` comments + `catalog_row_id: "<ID>"` const fields)
- [ ] `--check` mode of the report binary correctly detects ALL FOUR gates: (a) absolute-verified-count regression, (b) false-verified (no citation), (c) uncataloged citation (no catalog row), (d) non-empty uncataloged-backlog without paired catalog-update PR
- [ ] Cataloging safety net (04.9) lands: `UncatalogedDetector` wraps `SpecHarness::feed()`, appends misses to `plans/spec-conformance/uncataloged-backlog.md`, and the CI gate in section 23 fails on non-empty backlog
- [ ] Observation hooks in `gpu/prepare/` are gated behind `#[cfg(any(test, debug_assertions))]` so release builds have zero overhead
- [ ] Alloc regression unchanged: `cargo test -p oriterm_core --test alloc_regression` passes
- [ ] `./build-all.sh` green (cross-compile too)
- [ ] `./test-all.sh` green debug + release
- [ ] `./clippy-all.sh` green
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 04 status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** SpecHarness drives both pilots through every applicable rung green; catalog row schema frozen and section 01 catalogs migrated; coverage report binary works; BLOAT files split under 500 lines; full test suite green debug + release; alloc regression unchanged. Sections 08-20 can now be written against a stable harness API and a frozen catalog schema.
