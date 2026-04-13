---
section: "04"
title: "Verification Chain Harness + Pilots + Coverage Report"
status: in-progress
reviewed: true
goal: "Build the SpecHarness API that drives a sequence through every applicable rung of the verification chain (parser → dispatch → state/effect → renderable → frame-input → gpu-instance → texture → golden), validate the API with one visual pilot (sixel) and one non-visual pilot (DA1), freeze the catalog row schema based on what the pilots needed, and deliver the spec-coverage-report binary."
success_criteria:
  - "`CoreSpecHarness` (headless, rungs 1-4) API exists in `oriterm_test_support/src/spec_chain/mod.rs` with `RecordingHandler` for dispatch capture and vendored VTE `PerformObserver` for raw tuple capture; `VisualSpecHarness` (rungs 5-8) exists in `oriterm/src/gpu/visual_regression/spec_chain/` wrapping the core harness with GPU observation"
  - "Sixel visual pilot test exists at `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_minimal.rs` (lives under `visual_regression/spec_chain/` for `pub(super)` access to GPU helpers) — drives a minimal sixel raster fill scenario through every applicable rung from parser to golden image, all green"
  - "DA1 non-visual pilot test exists at `oriterm_core/tests/spec_chain/pilots/da1_query.rs` — drives a DA1 query through parser → dispatch → handler → effect transcript apex (PtyEffect::Write with PtyWriteKind::DeviceAttribute), all green"
  - "Catalog row schema is FROZEN: `plans/spec-conformance/catalog/README.md` documents the canonical row format, the column set, and the rung naming convention used by the harness"
  - "All catalog files from section 01 are migrated from the provisional schema to the frozen schema (all rows updated to the canonical format)"
  - "`cargo run -p oriterm_test_support --bin spec-coverage-report` exists, walks `plans/spec-conformance/catalog/*.md`, scans test directories (`oriterm_core/tests/`, `oriterm/tests/`, `oriterm_ui/tests/`, `oriterm_mux/tests/`, `crates/oriterm_test_support/`) for catalog row ID citations via both `// Catalog row: <ID>` comments AND `catalog_row_id: \"<ID>\"` const fields, and produces a per-stack absolute-verified-count table."
  - "Coverage report's gating metric is the ABSOLUTE count of `verified` rows per stack, NOT percentage. Reason: section 01 + the 04.9 continuous-delta detector keep adding rows as real captures surface uncataloged sequences, so the denominator grows over time. Absolute count is monotonic; percentage is not."
  - "Coverage report flags FALSE-VERIFIED rows (catalog says `verified` but no test cites the row ID) and UNCATALOGED citations (test cites a row ID that doesn't exist in any catalog file); `--check` mode fails CI on either."
  - "Cataloging safety net exists (section 04.9): `SpecHarness::feed()` accumulates observed sequence tuples in-memory via `UncatalogedDetector` (plain `HashSet<TupleSig>` — each harness is single-threaded, no `Arc`/`Mutex`). On drop, tuples are serialized to a uniquely-named per-instance temp file (using atomic counter or nanosecond timestamp to avoid overwriting). `spec-coverage-report --check` materializes `plans/spec-conformance/uncataloged-backlog.md` in a single serial post-test step and fails CI on unknown tuples. No file I/O during parallel test execution (flaky-test discipline)."
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
  status: resolved
  updated: 2026-04-12
sections:
  - id: "04.1"
    title: "Design SpecHarness API + per-rung observers"
    status: complete
  - id: "04.2"
    title: "Implement parser/dispatch/state observers"
    status: complete
  - id: "04.3"
    title: "Implement renderable observer + BLOAT splits (headless — oriterm_test_support)"
    status: complete
  - id: "04.3b"
    title: "Implement frame-input/gpu-instance observers (visual — oriterm)"
    status: complete
  - id: "04.4"
    title: "Implement texture-render + golden-image observers (depends on 05's deterministic GPU env, but uses the existing non-deterministic env until 05 lands; section gates allow this)"
    status: not-started
  - id: "04.5"
    title: "Sixel visual pilot — drive minimal raster fill through every rung"
    status: not-started
  - id: "04.6"
    title: "DA1 non-visual pilot — drive query through effect transcript apex"
    status: complete
  - id: "04.7"
    title: "Freeze catalog row schema + migrate section 01 catalog files"
    status: not-started
  - id: "04.8"
    title: "Coverage report generator binary (catalog walk + citation scan + monotonic absolute count)"
    status: complete
  - id: "04.9"
    title: "Cataloging safety net — continuous delta detection for uncataloged sequences"
    status: complete
  - id: "04.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "04.N"
    title: "Completion Checklist"
    status: in-progress
# TPR Checkpoint Placement: 04.3 (after headless observer infrastructure — covers .1-.3),
# 04.6 (after both pilots run green — covers .3b-.6), final in 04.N
---

# Section 04: Verification Chain Harness + Pilots + Coverage Report

**Status:** In Progress
**Goal:** Build the verification chain harness that section 08 onward will use to drive every catalog row to `verified` status. The harness is split into two layers by crate boundary: `CoreSpecHarness` (headless, rungs 1-4: parser/dispatch/state/effect/renderable) lives in `oriterm_test_support`, and `VisualSpecHarness` (GPU, rungs 5-8: frame-input/gpu-instance/texture/golden) lives in `oriterm/src/gpu/visual_regression/spec_chain/` (under `#[cfg(test)]`). Raw parser tuples are captured via a vendored VTE `Processor::advance_with_observer()` shim with `PerformObserver` trait. Semantic dispatch calls are captured via `RecordingHandler`, a wrapper that implements `vte::ansi::Handler` and records each method call before delegating to `Term<QueueingEffectSink>`. Two pilot scenarios — one visual (sixel raster fill) and one non-visual (DA1 query) — exercise every applicable rung end-to-end and prove the harness works. The pilots' API requirements are then used to FREEZE the catalog row schema (which was provisional in section 01). The coverage report generator is the binary that walks the catalog files (via the shared `oriterm_test_support::catalog::parse_catalog_markdown` parser and `walk_catalog_files()` created by Section 01.3) and produces a per-stack absolute-verified-count table. **Gating metric is absolute count (monotonic), not percentage.** Percentage is advisory only — because section 01 and the continuous-discovery safety net (04.9) keep adding new rows, the denominator grows and percentages can drop while absolute counts stay flat or rise. CI gates on absolute counts per 04.8.

**Success Criteria:**
- [x] `CoreSpecHarness` (headless) + `VisualSpecHarness` (GPU, at `visual_regression/spec_chain/`) APIs exist with per-rung observers; vendored VTE `PerformObserver` captures raw parser tuples; `RecordingHandler` captures semantic dispatch calls
- [ ] Sixel visual pilot drives every visual rung (parser through golden) green <!-- blocked-by:05 -->
- [x] DA1 non-visual pilot drives parser through effect apex green
- [ ] Catalog row schema frozen and section 01 catalogs migrated <!-- blocked-by:05 -->
- [x] `spec-coverage-report` binary exists and produces correct per-stack absolute-verified-count table (monotonic gating metric — percentage is advisory only; see 04.8 for the full rationale)
- [x] BLOAT splits applied as `gpu/prepare/{mod,dirty_skip/mod}.rs` are touched
- [x] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Connects to mission criteria: **Verification chain complete per row**, **Coverage report green**

**Context:** The harness is the load-bearing test infrastructure for the entire spec-conformance plan. Sections 08-20 each take a catalog file and grind every row from `implemented-unverified` to `verified` using this harness — without it, those sections have nothing to write tests against. Per Codex's "catalog breadth first, schema freeze after pilot" guidance, the catalog row format from section 01 is provisional; the pilots in this section discover what fields the harness actually needs to observe (e.g., does the row need an explicit `apex_layer` field or can it be inferred? Does the row need a `golden_path` field for visual sequences? What about per-platform variants?). Once the pilots run green, the schema is frozen and section 01's catalogs are migrated.

**Reference implementations:**
- **ori_term TeseqHarness** at `oriterm_core/tests/teseq/harness/runner.rs:39-124` — `TeseqHarness::from_scenario(path)` loads `.teseq` + `.toml` sidecar, constructs `Term<LegacyEventSink<RecordedListener>>`, applies `pre_feed`. `TeseqHarness::run() -> ScenarioOutcome` feeds bytes via `proc.advance(&mut self.term, &bytes)` and captures grid_text, cells, cursor, events, mode. Pattern to extend with per-rung observation. NOTE: TeseqHarness uses the `LegacyEventSink` adapter; SpecHarness uses `QueueingEffectSink` directly (per Section 03 contract).
- **ori_term visual_regression** at `oriterm/src/gpu/visual_regression/mod.rs:69-141` — `headless_env_with_hinting()`, `render_to_pixels()`, `compare_with_reference()`. Provides the GPU rung infrastructure (texture render + golden compare). Section 05 makes this deterministic.
- **ori_term ScenarioSpec** at `crates/oriterm_test_support/src/tack_framework/spec.rs:74-132` — `const ScenarioSpec` with function pointers (parser, quit_path), no closures. Template for the catalog row → test scenario binding.

**Depends on:** Section 03 (Effect type exists for the effect-transcript observer). Section 05 depends on the Phase 1a subset of Section 04 (04.1-04.3, 04.6, 04.8, 04.9 — the headless harness infrastructure), NOT on all of Section 04. This avoids a circular dependency: Section 05's `depends_on: ["04"]` should be read as "depends on 04 Phase 1a" since 04.4/04.5/04.7 themselves depend on Section 05. The `/continue-roadmap` scanner treats this as: Phase 1a lands → Section 05 lands → Phase 1b (04.4, 04.5, 04.7) lands.

**Section 04 ↔ Section 05 coupling (important — ACYCLIC dependency model):** Sections 04.4 (texture-render observer), 04.5 (sixel visual pilot committing a golden), and the FINALIZATION of 04.7 (catalog schema freeze) are NOT reproducible until Section 05 pins the software rasterizer, hinting mode, cell metrics, and tolerance. Ordering policy:

1. **Phase 1a (BEFORE Section 05):** Land 04.1 (harness API + VTE shim), 04.2 (parser/dispatch/state observers), 04.3 (renderable observer + BLOAT splits), 04.3b (VisualSpecHarness + frame-input/gpu-instance observers), 04.6 (DA1 non-visual pilot), 04.8 (coverage report), 04.9 (uncataloged detector). 04.3b is visual (uses GPU) but does NOT depend on the deterministic golden lane — it observes FrameInput and instance buffers, not pixel-level golden comparison.
2. **Phase 1b (AFTER Section 05):** Land 04.4 (texture-render + golden-image observers) and 04.5 (sixel pilot with deterministic golden) AFTER Section 05's `headless_env_with_pinned_software_rasterizer()` and `GoldenLaneConfig` are in place. The sixel pilot's golden is captured directly on the deterministic lane — there is no pre-05 throwaway.
3. **Schema freeze (AFTER Section 05):** The catalog row schema freeze in 04.7 MUST NOT be finalized until Section 05 has landed. If 05 surfaces new required fields (e.g. `cell_metrics`, `pixel_tolerance_override`, `hinting_mode_override`), the frozen schema has to include them.

This coupling is the reason the `depends_on` frontmatter lists only `03` for the first-phase work but includes a structured `blocked_by_until_05_lands` annotation.

---

## 04.1 Design SpecHarness API + per-rung observers

**File(s):** `crates/oriterm_test_support/src/spec_chain/mod.rs` (new), `crates/oriterm_test_support/src/spec_chain/api.rs` (new), `crates/oriterm_test_support/src/spec_chain/recording_handler.rs` (new), `crates/oriterm_test_support/src/spec_chain/tests.rs` (new)

The `SpecHarness` is the main test entry point. It wraps `Term<EffectSink-aware>`, accepts a sequence (bytes or `.teseq` scenario), feeds it through every applicable rung, and exposes per-rung observation methods. Const-constructible scenario definitions following the tack ScenarioSpec pattern.

- [x] Create `crates/oriterm_test_support/src/spec_chain/mod.rs` as the dispatch hub:
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
  mod recording_handler;
  mod scenario;

  pub use api::{SpecHarness, SpecOutcome, RungResult, PerformActionCollector};
  pub use observers::*;
  pub use recording_handler::{RecordingHandler, DispatchCall, DispatchArgs};
  // PerformAction and PerformObserver live in the vendored VTE crate
  // (crates/vte/src/ansi/mod.rs) — re-export for convenience
  pub use vte::ansi::{PerformAction, PerformObserver};
  pub use scenario::{SpecScenario, SpecScenarioBuilder, ApexLayer, RungName};

  #[cfg(test)]
  mod tests;
  ```
- [x] Create `crates/oriterm_test_support/src/spec_chain/api.rs`:

  **CRITICAL ARCHITECTURAL DECISION — parser/dispatch rung capture mechanism:**

  `Processor::advance()` (`crates/vte/src/ansi/processor.rs:96`) creates a `Performer` that directly calls `Handler` methods (via `crate::Perform` impl at `crates/vte/src/ansi/dispatch/mod.rs:22`). There is NO intermediate "parsed actions" or "dispatched calls" data structure that `advance()` returns. The parser feeds the `Performer`, the `Performer` calls `Handler` methods on `Term`. This means:

  - **Rung 1 (parser observation) — raw `Perform`-level tuple recording:** The parser (`crate::Parser`) calls `Perform` trait methods (`print`, `execute`, `csi_dispatch`, `osc_dispatch`, `hook`, `put`, `unhook`, `esc_dispatch`) on the `Performer`. These callbacks carry raw `(params, intermediates, action_byte)` data — the exact tuples that `catalog::TupleSig` and the 04.9 `UncatalogedDetector` need. The `Handler` trait does NOT expose this raw data (it has semantic methods like `goto`, `identify_terminal`). Therefore, rung 1 MUST capture at the `Perform` level, not the `Handler` level.

    **Implementation — vendored VTE shim (MANDATORY):** The VTE crate is vendored at `crates/vte/`. The `Performer` struct and dispatch functions (`csi`, `osc`, `esc_dispatch`) are private internals of `crates/vte/src/ansi/dispatch/mod.rs`. Attempting to bypass `Processor::advance()` with a manual `vte::Parser::advance()` + custom `Perform` impl would require duplicating the entire dispatch implementation — a `LEAK:algorithmic-duplication` violation.

    **The correct fix:** Add `Processor::advance_with_observer<H, O>(&mut self, handler: &mut H, observer: &mut O, byte: u8)` to the vendored VTE crate at `crates/vte/src/ansi/processor.rs`. This method:
    - Creates the internal `Performer` as usual
    - Wraps it in a `RecordingPerformer<O>` that records raw `PerformAction` entries into the observer before delegating to the real `Performer`
    - Uses the canonical dispatch path (no duplication)
    - `O: PerformObserver` is a new trait in `crates/vte/src/ansi/mod.rs` with methods `on_csi_dispatch(params, intermediates, action)`, `on_osc_dispatch(params)`, `on_esc_dispatch(intermediates, byte)`, `on_execute(byte)`, `on_print(c)`

    The `RecordingPerformer` lives INSIDE the vendored VTE crate (not in `oriterm_test_support`) because it wraps the private `Performer`. `oriterm_test_support` only sees the `PerformObserver` trait and calls `Processor::advance_with_observer()`.

    **File changes to vendored VTE:**
    - `crates/vte/src/ansi/processor.rs` — add `advance_with_observer()` method
    - `crates/vte/src/ansi/mod.rs` — add `PerformObserver` trait + `PerformAction` type
    - `crates/vte/src/ansi/dispatch/mod.rs` — add `RecordingPerformer` wrapper (internal)

    This is a minimal, focused patch to the vendored crate that preserves SSOT for dispatch logic.

  - **Rung 2 (dispatch observation) — semantic `Handler`-level call recording:** The `RecordingHandler` wrapper implements `Handler` by recording each method call as a `DispatchCall { method: &'static str, args: DispatchArgs }` and delegating to the inner `Term`. This captures "was the right handler method called with the right arguments?" The wrapper lives in `crates/oriterm_test_support/src/spec_chain/recording_handler.rs`. This is a DIFFERENT layer from rung 1 — rung 1 captures raw parser tuples; rung 2 captures semantic dispatch calls. Both layers operate in a single pass.

  - **Implementation approach:** The harness composes `RecordingPerformer` (rung 1 capture) wrapping `RecordingHandler` (rung 2 capture) wrapping `Term<QueueingEffectSink>` (state/effect capture). The harness calls `vte::Parser::advance()` with `&mut recording_performer`, which records the raw tuple, then calls the dispatch function which invokes the `RecordingHandler`, which records the semantic call and delegates to `Term`. One pass through the byte stream populates all three capture layers.

  ```rust
  use oriterm_core::{Term, effect::*};
  use oriterm_core::effect::sink::QueueingEffectSink;

  use super::recording_handler::RecordingHandler;

  /// Headless verification chain harness for spec conformance tests.
  ///
  /// Wraps `Term<QueueingEffectSink>` (per Section 03 contract — Section 03.N
  /// closeout explicitly requires Section 04 to use `Term<QueueingEffectSink>`,
  /// NOT the old `Term<T: EventListener>` model).
  ///
  /// Two recording layers operate in a single pass:
  /// - `RecordingPerformer` (rung 1): captures raw `Perform` callbacks
  ///   (`csi_dispatch`, `osc_dispatch`, `esc_dispatch` with params/intermediates/byte)
  /// - `RecordingHandler` (rung 2): captures semantic `Handler` method calls
  ///
  /// Effects are drained from the `Term`'s owned `QueueingEffectSink` via
  /// `self.handler.term().effect_sink().drain_into()` — no separate `Arc`
  /// needed because `Term::new()` takes the sink by value and there is no
  /// `EffectSink for Arc<T>` blanket impl.
  pub struct SpecHarness {
      handler: RecordingHandler<QueueingEffectSink>,
      processor: vte::ansi::Processor,
      perform_observer: PerformActionCollector, // impl PerformObserver
      observed: SpecOutcome,
  }

  #[derive(Debug, Default, Clone)]
  pub struct SpecOutcome {
      /// Rung 1: raw parser actions recorded by RecordingPerformer.
      /// Each entry captures the `Perform` callback type + raw params/
      /// intermediates/final_byte — needed for UncatalogedDetector (04.9).
      pub perform_actions: Vec<PerformAction>,
      /// Rung 2: semantic handler calls recorded by RecordingHandler.
      pub dispatched_calls: Vec<DispatchCall>,
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
      pub fn new() -> Self {
          let sink = QueueingEffectSink::new();
          let term = Term::new(24, 80, 1000, Theme::default(), sink);
          let handler = RecordingHandler::new(term);
          let processor = vte::ansi::Processor::new();
          let perform_observer = PerformActionCollector::new();
          Self {
              handler,
              processor,
              perform_observer,
              observed: SpecOutcome::default(),
          }
      }

      /// Feed bytes through the parser and dispatch.
      ///
      /// Uses `Processor::advance_with_observer()` (vendored VTE shim) that:
      /// 1. Records raw `Perform` callbacks via `PerformObserver` (rung 1)
      /// 2. Delegates to the canonical `Performer` which calls `Handler`
      ///    methods on `RecordingHandler` (rung 2: semantic handler calls)
      /// 3. `RecordingHandler` delegates to `Term` (rung 3: state/effects)
      ///
      /// Effects are drained via `handler.term().effect_sink().drain_into()`
      /// — `QueueingEffectSink::drain_into` takes `&self` (interior Mutex).
      pub fn feed(&mut self, bytes: &[u8]) {
          self.processor.advance_with_observer(
              &mut self.handler,
              &mut self.perform_observer,
              bytes,
          );
          // Drain rung 1 recordings (raw Perform actions)
          self.perform_observer.drain_into(&mut self.observed.perform_actions);
          // Drain rung 2 recordings (semantic dispatch calls)
          self.handler.drain_calls_into(&mut self.observed.dispatched_calls);
          // Drain effects from Term's owned QueueingEffectSink
          self.handler.term().effect_sink().drain_into(&mut self.observed.effects_emitted);
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
- [x] Create `crates/oriterm_test_support/src/spec_chain/scenario.rs`:
  ```rust
  use super::*;

  /// Const-constructible scenario definition (no closures, function pointers only).
  ///
  /// **Const-constructibility contract:** Every field type must be `const`-
  /// constructible. Slices use `&'static [u16]` / `&'static [u8]` (e.g.,
  /// `&[5, 10]` works as a const `&'static [u16]`). Expectation constructors
  /// (`ParserExpectation::csi_with_params`, `StateExpectation::cursor_at`,
  /// etc.) MUST be declared `const fn` returning `&'static` slices where
  /// applicable. `Option` wrapping is fine in const context. This is what
  /// enables the `const SCENARIO: SpecScenario = ...` pattern that the
  /// citation scanner depends on.
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
      EffectHostNotification,
  }

  #[derive(Copy, Clone, Debug, PartialEq, Eq)]
  pub enum RungName {
      Parser, Dispatch, State, Effect, Renderable, FrameInput,
      GpuInstance, TextureRender, GoldenImage,
  }
  ```
- [x] Create `crates/oriterm_test_support/src/spec_chain/recording_handler.rs`:
  ```rust
  //! Recording handler wrapper for parser/dispatch observation.
  //!
  //! Wraps `Term<S: EffectSink>` and implements `vte::ansi::Handler`.
  //! Every handler method records a `DispatchCall` (method name + typed
  //! arguments) and delegates to the inner `Term`. This is how the
  //! SpecHarness captures rungs 1 (parser tokenization) and 2 (dispatch
  //! routing) — `Processor::advance()` takes `&mut impl Handler`, so
  //! passing `&mut RecordingHandler` intercepts all dispatch calls.

  use oriterm_core::effect::sink::EffectSink;
  use oriterm_core::Term;

  /// A single recorded handler dispatch call.
  #[derive(Debug, Clone)]
  pub struct DispatchCall {
      pub method: &'static str,
      pub args: DispatchArgs,
  }

  /// Typed argument capture for each handler method family.
  #[derive(Debug, Clone)]
  pub enum DispatchArgs {
      Input(char),
      Goto { line: i64, col: usize },
      CsiDispatch { params: Vec<Vec<u16>>, intermediates: Vec<u8>, action: char },
      OscDispatch { params: Vec<Vec<u8>> },
      EscDispatch { intermediates: Vec<u8>, byte: u8 },
      Bell,
      // ... one variant per Handler method family.
      // Exhaustive coverage built incrementally as pilots exercise methods.
      Other { method: &'static str },
  }

  pub struct RecordingHandler<S: EffectSink> {
      term: Term<S>,
      calls: Vec<DispatchCall>,
  }

  impl<S: EffectSink> RecordingHandler<S> {
      pub fn new(term: Term<S>) -> Self {
          Self { term, calls: Vec::new() }
      }
      pub fn term(&self) -> &Term<S> { &self.term }
      pub fn term_mut(&mut self) -> &mut Term<S> { &mut self.term }
      pub fn drain_calls_into(&mut self, out: &mut Vec<DispatchCall>) {
          out.extend(self.calls.drain(..));
      }
  }

  // impl Handler for RecordingHandler<S> — records each call, delegates to self.term.
  // Each method: self.calls.push(DispatchCall { method: "goto", args: ... }); self.term.goto(line, col);
  ```
- [x] Sibling tests in `crates/oriterm_test_support/src/spec_chain/tests.rs`:
  - `harness_constructs()`
  - `feed_advances_parser_and_captures_effects()`
  - `feed_records_dispatch_calls()` — feed `\x1b[5;10H` and assert `dispatched_calls` contains a `goto` entry
  - `run_scenario_stops_at_first_failed_rung()`
  - `apex_layer_determines_applicable_rungs()`
- [x] **Validation**: `cargo test -p oriterm_test_support --lib spec_chain::tests` passes; harness constructs without panic.

### Canonical `SpecScenario` recipe (for test authors writing new rows)

Every catalog row that reaches `verified` status is backed by a test that declares a `const SpecScenario` and drives it through the harness. This recipe is the canonical template — copy it when adding a new scenario.

**Placement rules (crate boundary):**
- **Non-visual scenarios** (apex is `State`, `EffectPtyWrite`, etc. — rungs 1-4 only): place in `oriterm_core/tests/spec_chain/<stack>/<row_id_kebab>.rs`. These use `CoreSpecHarness` from `oriterm_test_support` and run headlessly.
- **Visual scenarios** (apex is `FrameInput`, `GpuInstance`, `TextureRender`, `GoldenImage` — rungs 5-8): place in `oriterm/src/gpu/visual_regression/spec_chain/<stack>/<row_id_kebab>.rs`. These use `VisualSpecHarness` (wraps `CoreSpecHarness` + GPU env) and require `pub(super)` access to GPU helpers in `visual_regression/mod.rs`.

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
6. For visual-apex scenarios (`ApexLayer::GoldenImage`), the golden PNG path MUST live under `oriterm/tests/references/spec_chain/<stack>/<row_id>.png` — the `reference_dir()` helper at `visual_regression/mod.rs:64` resolves to `oriterm/tests/references/`. The coverage report scans this directory for orphan goldens.

This recipe is the contract between the harness and the rest of sections 08-20 + 26. Do not deviate without updating `catalog/README.md` + the harness tests.

---

## 04.2 Implement parser/dispatch/state observers

**File(s):** `crates/oriterm_test_support/src/spec_chain/observers/{parser,dispatch,state,effect}.rs` (new)

Each observer takes the captured `SpecOutcome` and an `Expectation` struct, and returns `RungResult`. Observers are pure functions of the outcome — no side effects, no Term access — which makes them composable and easy to test in isolation.

**Parser/dispatch observation architecture (resolved in 04.1):** `Processor::advance()` calls `Handler` methods directly via the `Performer` — there is no intermediate "parsed actions" data structure. The `RecordingHandler` (created in 04.1) intercepts every `Handler` method call, records a `DispatchCall` entry (method name + typed `DispatchArgs`), and delegates to the inner `Term`. Both the parser observer (rung 1) and dispatch observer (rung 2) operate on `outcome.dispatched_calls`:

- **Parser observer** asserts the raw sequence was recognized correctly by checking that the expected method was called with the expected parameter values (e.g., CSI `H` with params `[5, 10]`).
- **Dispatch observer** asserts the correct semantic handler method was invoked (e.g., `goto` rather than some other method).

These are conceptually distinct observations on the same data — the parser observer checks "did the parser extract the right parameters?" while the dispatch observer checks "did the dispatch route to the right handler method?"

- [x] `observers/parser.rs`: `observe_parser(outcome, expected) -> RungResult` — assert that the parser tokenized the expected sequence by checking `outcome.perform_actions` (raw `Perform`-level callbacks) for an entry matching the expected category, intermediates, and final byte. Example: for CSI `H` with params `[5, 10]`, assert there is a `PerformAction::CsiDispatch { params: [5, 10], intermediates: [], action: 'H', .. }`. This operates on the raw parser layer — distinct from rung 2 (dispatch) which checks semantic `Handler` method calls.
- [x] `observers/dispatch.rs`: `observe_dispatch(outcome, expected) -> RungResult` — assert that the expected handler method was called by checking `outcome.dispatched_calls` for an entry with the expected method name. Example: for `DispatchExpectation::method("goto")`, assert there is a `DispatchCall { method: "goto", .. }`.
- [x] `observers/state.rs`: `observe_state(outcome, expected) -> RungResult` — assert that the final terminal state matches expected (cells, cursor, modes, palette, etc.). State observer takes `&Term` directly for cursor/grid access.
- [x] `observers/effect.rs`: `observe_effect(outcome, expected) -> RungResult` — assert that the expected Effect was emitted (or that NO effect was emitted, depending on the expectation). Compare against `outcome.effects_emitted`.
- [x] Sibling tests for each observer.
- [x] **Validation**: each observer's tests pass in isolation. Observers correctly distinguish "expected matched" vs "expected absent" vs "expected missing".

---

## 04.3 Implement renderable observer + BLOAT splits (headless — `oriterm_test_support`)

**File(s):** `crates/oriterm_test_support/src/spec_chain/observers/renderable.rs` (new), `oriterm/src/gpu/prepare/mod.rs` (split), `oriterm/src/gpu/prepare/dirty_skip/mod.rs` (split)

**Crate boundary decision (resolves GAP/BLOAT from /tp-help):** Rungs 1-4 (parser, dispatch, state, effect, renderable) are headless and live in `oriterm_test_support`. Rungs 5-8 (frame-input, gpu-instance, texture, golden) require `oriterm`'s GPU types (`FrameInput`, `GpuPipelines`, `WindowRenderer`, `GpuState`). Putting rungs 5-8 in `oriterm_test_support` would create a circular dev-dependency: `oriterm` dev-depends on `oriterm_test_support`, and if `oriterm_test_support` depends on `oriterm` for GPU types, `oriterm_core`'s dev-dep on `oriterm_test_support` would transitively pull wgpu/winit into headless core tests = massive BLOAT.

**Split:** `CoreSpecHarness` (rungs 1-4) in `oriterm_test_support`. `VisualSpecHarness` (rungs 5-8) in `oriterm/src/gpu/visual_regression/spec_chain/` (under `#[cfg(test)]`), wrapping the core harness and adding GPU observation. The `SpecHarness` from 04.1 IS the `CoreSpecHarness`. `VisualSpecHarness` imports it as a field and extends it with GPU rung methods. This split is load-bearing for the entire plan — rungs 5-8 tests live under `oriterm` where GPU types are available; rungs 1-4 tests live under `oriterm_core/tests/` or `oriterm_test_support/` where they run headlessly.

The renderable observer (rung 4) stays in `oriterm_test_support` because `RenderableContent` lives in `oriterm_core` and requires no GPU types. The BLOAT splits in `gpu/prepare/` are prerequisite for 04.3b (visual observers) which lands later.

- [x] **FIRST CHECKBOX (BLOAT split prerequisite)**: Split `oriterm/src/gpu/prepare/mod.rs` (504 lines) into submodules. Extracted `resolve.rs` (color constants + `resolve_cursor` + `resolve_cell_colors`) — mod.rs now 395 lines, resolve.rs 121 lines.
- [x] **FIRST CHECKBOX (BLOAT split prerequisite)**: Split `oriterm/src/gpu/prepare/dirty_skip/mod.rs` (506 lines). Extracted `selection_damage.rs` (`build_dirty_set` + `mark_selection_damage` + `is_block_mode`) — mod.rs now 378 lines, selection_damage.rs 138 lines.
- [x] `observers/renderable.rs`: `observe_renderable(outcome, expected) -> RungResult` — stub that always passes until pilots define concrete expectations. Lives in `oriterm_test_support` (no GPU types needed).
- [x] Sibling tests for the renderable observer. (Stub observer has no behavior to test independently — covered by harness integration tests.)
- [x] **Validation**: BLOAT files now under 500 lines; renderable observer compiles headlessly; all tests pass.
- [x] **TPR checkpoint** — `/tpr-review` covering 04.1–04.3. 5 iterations, 16 findings fixed, clean pass. Key improvements: run_scenario wired to observers, EffectExpectation sub_variant, parser DCS/OSC/APC support, unit struct stubs.

## 04.3b Implement frame-input/gpu-instance observers (visual — `oriterm`)

**File(s):** `oriterm/src/gpu/visual_regression/spec_chain/mod.rs` (new), `oriterm/src/gpu/visual_regression/spec_chain/visual_harness.rs` (new), `oriterm/src/gpu/visual_regression/spec_chain/observers/{frame_input,gpu_instance}.rs` (new), `oriterm/src/gpu/visual_regression/spec_chain/tests.rs` (new)

**Crate boundary:** These observers live under `oriterm/src/gpu/visual_regression/` (not `oriterm/tests/` and not `oriterm_test_support`) because they depend on `FrameInput`, `GpuPipelines`, `WindowRenderer`, `GpuState`, and critically `headless_env_with_hinting()`, `render_to_pixels()`, `compare_with_reference()` — all of which are `pub(super)` in `oriterm/src/gpu/visual_regression/mod.rs`. Integration tests under `oriterm/tests/` compile as an external crate and CANNOT access `pub(super)` or `pub(crate)` items — only unit-test modules within the same crate can. Therefore the `VisualSpecHarness` MUST live as a submodule of `visual_regression`, not as an integration test.

**Why not promote helpers to `pub`?** Making `headless_env_with_hinting`, `render_to_pixels`, and `compare_with_reference` public would EXPOSE internal GPU test infrastructure as part of `oriterm`'s public API — a violation of `.claude/rules/code-hygiene.md` §"Public API discipline". The `pub(super)` visibility is correct; the harness must live where it can access them.

- [x] Create `oriterm/src/gpu/visual_regression/spec_chain/mod.rs` as the visual harness hub (add `mod spec_chain;` to `visual_regression/mod.rs` under `#[cfg(test)]`).
- [x] Create `oriterm/src/gpu/visual_regression/spec_chain/visual_harness.rs`: `VisualSpecHarness` wraps `SpecHarness` (core, from `oriterm_test_support`), holds `GpuState` + `GpuPipelines` + `WindowRenderer`. Builds `FrameInput` from `Term` state using the same palette constants as `frame_input_helper` (SSOT). Added `prepare_scenario()` and `observe_rung()` to core `SpecHarness` for clean rung delegation without algorithmic duplication.
- [x] `observers/frame_input.rs`: `observe_frame_input(&FrameInput, &FrameInputExpectation) -> RungResult` — asserts grid dimensions (cols, rows), cursor visibility, and reverse video mode. `FrameInputExpectation` expanded with `cols`, `rows`, `cursor_visible`, `reverse_video` fields.
- [x] `observers/gpu_instance.rs`: `observe_gpu_instance(&PreparedFrame, &GpuInstanceExpectation) -> RungResult` — asserts background count, total glyph count (mono + subpixel + color), and cursor presence. `GpuInstanceExpectation` expanded with `min_backgrounds`, `min_glyphs`, `has_cursor` fields.
- [x] Observation hooks: `PreparedFrame` fields are `pub(crate)`, giving `VisualSpecHarness` direct access to instance buffers without needing separate debug-gated hooks. No hot-path instrumentation needed — the observer reads post-prepare state.
- [x] Sibling tests in `spec_chain/tests.rs` for each observer. 10 tests: 2 harness construction, 4 frame_input observer (pass/fail cols/rows/none), 2 gpu_instance observer (pass/fail threshold), 2 visual scenario end-to-end (full rung chain, early-stop on failure).
- [x] **Validation**: observer tests pass under `cargo test -p oriterm --features gpu-tests -- spec_chain` (10/10 pass). `build-all.sh`, `clippy-all.sh`, `test-all.sh` all green.

---

## 04.4 Implement texture-render + golden-image observers (LAND AFTER Section 05)

**File(s):** `oriterm/src/gpu/visual_regression/spec_chain/observers/{texture,golden}.rs` (new — lives under `visual_regression/spec_chain/` for `pub(super)` access to GPU helpers, same placement logic as 04.3b)

**Crate boundary:** These observers live under `oriterm/src/gpu/visual_regression/spec_chain/` (not `oriterm/tests/` and not `oriterm_test_support`) because they depend on `render_frame_cached()`, `headless_env_with_pinned_software_rasterizer()`, and `compare_with_reference_strict()` — all `pub(super)` in `visual_regression/mod.rs`. Integration tests under `oriterm/tests/` compile as an external crate and cannot access `pub(super)` items (see 04.3b crate boundary decision).

**Ordering gate:** This subsection MUST land AFTER Section 05's deterministic golden lane is in place (`headless_env_with_pinned_software_rasterizer()` + `GoldenLaneConfig`). The texture-render observer reads back GPU pixels; the golden observer compares against a committed PNG. Without 05's adapter pin, hinting pin, cell metrics pin, and tolerance pin, any golden committed here will flake on CI or another developer's machine. Section 04's first-phase work (04.1–04.3, 04.6, 04.8) does not depend on this subsection; this subsection is the bridge from the pilot-era harness to the verified-apex-era harness and should be interleaved with 05.6.

- [ ] `observers/texture.rs`: `observe_texture_render(outcome, expected) -> RungResult` — uses `render_frame_cached()` (NOT `render_frame()` — per `.claude/rules/tests.md` §GPU Cached Render Path Testing) to render the FrameInput onto an offscreen target, reads back pixels, asserts pixel buffer matches expected. Must be invoked via `headless_env_with_pinned_software_rasterizer()` from Section 05.
- [ ] `observers/golden.rs`: `observe_golden_image(outcome, expected_path) -> RungResult` — calls `compare_with_reference_strict(name, pixels, w, h, config)` from Section 05.5. Returns `RungResult::pass()` on exact match, `failure(diff_summary)` on any mismatch.
- [ ] Sibling tests in `oriterm/src/gpu/visual_regression/spec_chain/observers/tests.rs`: use Section 05's pinned env; do NOT use the legacy `headless_env_full()` entry point.
- [ ] **Validation**: texture render observer produces deterministic pixel readback for a known input across TWO consecutive runs on the same machine. Golden observer correctly matches identical inputs and rejects single-pixel changes.

---

## 04.5 Sixel visual pilot — drive minimal raster fill through every rung (LAND AFTER Section 05)

**File(s):** `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_minimal.rs` (new — lives under `visual_regression/spec_chain/` because it drives GPU rungs 5-8 which need `pub(super)` access to `headless_env_with_hinting`, `render_to_pixels`, `compare_with_reference`), `oriterm/src/gpu/visual_regression/spec_chain/pilots/mod.rs` (new), `oriterm/tests/references/spec_chain/pilots/sixel_minimal.png` (golden, captured via `ORITERM_UPDATE_GOLDEN=1` — stored in `oriterm/tests/references/` which is where the existing `reference_dir()` at `visual_regression/mod.rs:64` resolves to)

**Ordering gate (Phase 1b — strictly AFTER Section 05):** This subsection lands AFTER Section 05's deterministic lane is fully in place. The committed `sixel_minimal.png` golden is captured directly via `headless_env_with_pinned_software_rasterizer(GoldenLaneConfig::SPEC_DEFAULT)` — using the deterministic env natively, not a legacy throwaway. Section 05.6 does NOT need to "migrate" this pilot because it never exists in a non-deterministic form. The dependency is one-directional: 04.5 depends on 05 being complete, 05 does not depend on 04.5.

The sixel visual pilot is the canonical visual chain test. It feeds a minimal sixel raster fill (a few sixel bytes that paint a small solid rectangle) and asserts every rung from parser to golden image passes. This proves the harness can drive a visual sequence end-to-end.

- [ ] Create `oriterm/src/gpu/visual_regression/spec_chain/pilots/mod.rs`:
  ```rust
  pub mod sixel_minimal;
  // DA1 pilot lives under oriterm_core (non-visual, no GPU)
  ```
- [ ] Create `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_minimal.rs`:
  ```rust
  use oriterm_test_support::spec_chain::*;
  use super::super::visual_harness::VisualSpecHarness;

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
      // Visual pilot uses VisualSpecHarness (wraps CoreSpecHarness + GPU env)
      // because it drives through rungs 5-8 which require wgpu types.
      // Located under visual_regression/spec_chain/ where pub(super) GPU helpers
      // are accessible.
      let mut harness = VisualSpecHarness::new();
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
  ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm -- visual_regression::spec_chain::pilots::sixel_minimal::sixel_minimal_drives_every_rung_green
  ```
- [ ] Verify the test passes when run again without `ORITERM_UPDATE_GOLDEN`:
  ```bash
  cargo test -p oriterm -- visual_regression::spec_chain::pilots::sixel_minimal
  ```
- [ ] **Validation**: pilot test passes; every rung observed; golden image captured under `oriterm/tests/references/spec_chain/pilots/sixel_minimal.png` (where `reference_dir()` at `visual_regression/mod.rs:64` resolves to).

---

## 04.6 DA1 non-visual pilot — drive query through effect transcript apex

**File(s):** `oriterm_core/tests/spec_chain/pilots/da1_query.rs` (new)

The DA1 (Device Attributes Primary) non-visual pilot proves the effect transcript apex works. DA1 is `CSI c` — the terminal responds with a `CSI ? ... c` reply that identifies its capabilities. The reply is a `PtyEffect::Write { kind: PtyWriteKind::DeviceAttribute }` — the apex of the non-visual chain.

- [x] Create `oriterm_core/tests/spec_chain/pilots/da1_query.rs`: Integration test with 3 tests — `da1_query_drives_to_effect_apex` (full scenario with parser + dispatch + effect rungs via `run_scenario`), `da1_reply_bytes_match_vt420_attributes` (verifies exact reply bytes `\x1b[?64;6;4c`), `da1_skips_parser_rung_when_no_expectation` (proves None expectations pass unconditionally). Note: CSI c default param is `[0]` per VTE parser convention.
- [x] **Validation**: all 3 pilot tests pass; effect transcript correctly captures `PtyEffect::Write { kind: DeviceAttribute, bytes: ESC[?64;6;4c }`; PtyWriteKind discriminator observable via `EffectExpectation::pty("DeviceAttribute")`.
- [x] **TPR checkpoint** — `/tpr-review` covering 04.3b–04.9 (Phase 1a). 2 semantic iterations, 6 codex findings found and fixed: (1) UncatalogedDetector wired into harness lifecycle, (2) citation scanner false-positive fix, (3) catalog row ID mismatch fix, (4) TextureRender/GoldenImage stub observer rejection, (5) backlog gate subtracts catalog signatures, (6) repo-local spool directory. Gemini unavailable (API capacity) across both iterations — codex-only clean pass accepted. (2026-04-13)

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

- [x] Create `crates/oriterm_test_support/src/spec_chain/coverage/mod.rs`:
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
          // 1. Walk plans/spec-conformance/catalog/*.md via the SSOT
          //    `walk_catalog_files()` function (crates/oriterm_test_support/
          //    src/catalog/mod.rs:61). Do NOT use raw `std::fs::read_dir()` —
          //    that would duplicate the file-enumeration policy (README.md
          //    exclusion, `_`-prefixed file exclusion, sort order) and create
          //    DRIFT vs the canonical walker.
          //    Propagate every parser error via `?` — never swallow with
          //    `.unwrap_or_default()`.
          let mut rows: Vec<Row> = Vec::new();
          let catalog_files = crate::catalog::walk_catalog_files(catalog_dir)?;
          for path in catalog_files {
              let file_rows = parse_catalog_markdown(&path)
                  .map_err(|e| anyhow::anyhow!(
                      "catalog parse failed for {}: {e}",
                      path.display()
                  ))?;
              rows.extend(file_rows);
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
- [x] Do NOT create `crates/oriterm_test_support/src/spec_chain/coverage/walk.rs`. The catalog-walk logic is owned by `crates/oriterm_test_support/src/catalog/mod.rs` (created by Section 01.3) and consumed by both binaries (`catalog_coverage_check` from 01.3 and `spec_coverage_report` from 04.8). A separate `walk.rs` under `spec_chain/coverage/` would violate SSOT — two markdown-table parsers for the same file set.
- [x] Create `crates/oriterm_test_support/src/spec_chain/coverage/scan.rs` — walks the test directories via `walkdir`, greps every `.rs` file for ALL citation forms: `// Catalog row: ([A-Z0-9-]+)`, `//! Catalog row: ([A-Z0-9-]+)`, `/// Catalog row: ([A-Z0-9-]+)` (doc comments used in canonical recipe), AND `catalog_row_id: "([A-Z0-9-]+)"` (const field pattern). Also walks `src/` directories (not just `tests/`) because visual spec_chain tests live under `oriterm/src/gpu/visual_regression/spec_chain/` as unit tests — the scanner must include those source roots. Produces `Vec<Citation>` with `{ catalog_row_id, test_file_path }`.
- [x] Create `crates/oriterm_test_support/src/bin/spec_coverage_report.rs`:
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
          workspace_root.join("oriterm_core/src"),    // sibling tests.rs files
          workspace_root.join("oriterm/tests"),
          workspace_root.join("oriterm/src"),          // visual_regression/spec_chain/ unit tests
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
- [x] **Error propagation (Phase 4 section-01 iteration-9 TPR-01-001-gemini fix — no swallowed errors):** `CoverageReport::build()` MUST propagate parser errors via `Result<Self, anyhow::Error>` rather than silently swallowing them with `.unwrap_or_default()`. If a catalog markdown file fails to parse (bad 10-column schema, unrecognized column name, malformed row), the build fails loudly with the file path and the parser's error message. The earlier `.flat_map(|e| parse_catalog_markdown(&e.path()).unwrap_or_default())` pattern was a LEAK (swallowed error) and is explicitly forbidden — use a fold that accumulates errors and returns the first failure. Section 01.3.a's `parse_catalog_markdown` is documented to return `Result<Vec<Row>, Error>` and this binary respects that signature; consumers must NEVER silently drop errors from the shared parser.
- [x] Sibling tests in `crates/oriterm_test_support/src/spec_chain/coverage/tests.rs`:
  - `coverage_report_build_invokes_shared_catalog_parser()` — assert the shared `oriterm_test_support::catalog::parse_catalog_markdown` is the only code path used for markdown parsing (no inline parsing, no duplicated regex)
  - `coverage_report_build_propagates_parser_errors()` — feed a malformed catalog file, assert `CoverageReport::build()` returns `Err(_)` with the file path in the error message (iter-9 TPR-01-001-gemini error-propagation pin)
  - `scan_test_citations_finds_comment_citation()` (`// Catalog row: ECMA48-CUP`)
  - `scan_test_citations_finds_const_field_citation()` (`catalog_row_id: "ECMA48-CUP"`)
  - `false_verified_flagged_when_catalog_verified_but_no_test_cites()`
  - `uncataloged_flagged_when_test_cites_but_catalog_missing()`
  - `has_regression_fails_when_absolute_verified_drops()`
  - `has_regression_passes_when_absolute_verified_holds_steady_despite_new_rows_added()` (the monotonicity semantic)
- [x] Add Cargo binary entry to `crates/oriterm_test_support/Cargo.toml`:
  ```toml
  [[bin]]
  name = "spec-coverage-report"
  path = "src/bin/spec_coverage_report.rs"
  ```
- [x] Create `plans/spec-conformance/coverage-baseline.toml` — the initial baseline file that `--check` mode reads. Format: TOML table with per-stack `verified` count. Example:
  ```toml
  # Coverage baseline — absolute verified row counts per stack.
  # Updated by spec-coverage-report --update-baseline.
  # CI fails if any stack's verified count drops below these values.
  [stacks]
  ecma-48 = 0
  xterm-ctlseqs = 0
  dec-private-modes = 0
  osc = 0
  sixel = 0
  kitty-graphics = 0
  kitty-keyboard = 0
  iterm2 = 0
  mode-2026 = 0
  unicode-subcell = 0
  mouse = 0
  charsets = 0
  audio-print = 0
  shell-integration = 0
  historical = 0
  de-facto-behaviors = 0
  ```
  Initial values are all 0 (no rows verified yet). As sections 08-20 verify rows, the baseline is updated via `spec-coverage-report --update-baseline`. The `CoverageBaseline` type lives in `crates/oriterm_test_support/src/spec_chain/coverage/mod.rs` alongside `CoverageReport`.
- [x] **Validation**: `cargo run -p oriterm_test_support --bin spec-coverage-report` produces the expected per-stack table (16 stacks, 315 total rows, 0 verified). `--check` mode correctly fails on uncataloged test citations. All 12 unit tests pass (citation scanning, error propagation, false-verified detection, regression detection, baseline parsing).

---

## 04.9 Cataloging safety net — continuous delta detection

**File(s):** `crates/oriterm_test_support/src/spec_chain/uncataloged/mod.rs` (new), `crates/oriterm_test_support/src/spec_chain/uncataloged/tests.rs` (new), `plans/spec-conformance/uncataloged-backlog.md` (new — starts empty)

The catalog is bootstrapped in section 01 via a one-time bottom-up scan + top-down spec walk. If a real-world sequence is missed by both passes, it never gets a row and the coverage report never sees it. The continuous-delta detector catches this:

- **At harness replay time**: every `SpecHarness::feed()` call feeds bytes through the `RecordingPerformer` which records every raw `Perform` callback as a `PerformAction`. The `UncatalogedDetector` converts each `PerformAction` into a `TupleSig` (the canonical `(category, intermediates, final_byte)` form) and accumulates distinct tuples in memory. After all scenarios complete, the test runner collects the in-memory tuples and compares against the catalog's known tuples. Unknown tuples are **uncataloged sequences**.
- **At committed-capture scan time**: the notcurses-demo harness (section 21) and real-app harness (section 22) replay committed PTY captures. Every replay runs the same uncataloged-sequence detector and any hit is flagged.
- **At CI time (serial post-test step)**: section 23's `spec-coverage-report --check` mode reads all tuples emitted by test runs (written as a single serialized file by a dedicated serial post-test step — see below), compares against the catalog, and fails CI on unknown tuples. This prevents "forgetting" to add a new catalog row when a real capture surfaces a new sequence.

**Flaky-test discipline (per `.claude/rules/tests.md`):** The detector does NOT write to `plans/spec-conformance/uncataloged-backlog.md` during test execution. Parallel test threads writing to a shared file is a race condition and violates flaky-test discipline. Instead:

1. **During test execution**: `UncatalogedDetector` accumulates tuples in a thread-safe in-memory set (`Arc<Mutex<HashSet<SequenceTuple>>>`). The harness's `Drop` impl serializes the tuples to a per-process temp file under `target/spec-chain-uncataloged/`.
2. **Serial post-test step**: `spec-coverage-report --check` reads all temp files from `target/spec-chain-uncataloged/`, deduplicates, compares against the catalog's known tuples (via `crate::catalog::walk_catalog_files()` + tuple extraction), and materializes `plans/spec-conformance/uncataloged-backlog.md` from the merged result. This single-writer approach eliminates the file I/O race.
3. **CI gate**: `spec-coverage-report --check` fails if uncataloged tuples exist without an accompanying catalog-update PR.

- [x] Define `SequenceTuple` — a canonicalized form of the `(category, intermediates, final_byte, param_hash?)` that uniquely identifies a catalog row. Reuse `crate::catalog::TupleSig` from `crates/oriterm_test_support/src/catalog/tuple.rs` as the canonical tuple type (SSOT — do NOT define a parallel tuple type).
- [x] Build a hashset of known tuples by walking `plans/spec-conformance/catalog/*.md` via `crate::catalog::walk_catalog_files()` (NOT raw `std::fs::read_dir()`) and extracting tuples from the `Sequence` column.
- [x] Implement `UncatalogedDetector` with an `HashSet<TupleSig>` for in-memory accumulation (each `SpecHarness` is single-threaded — no `Arc`/`Mutex` needed). The detector is a field on `SpecHarness`; each `feed()` call extracts tuples from the `RecordingPerformer`'s `PerformAction` entries (raw `csi_dispatch`, `osc_dispatch`, `esc_dispatch` callbacks with category/intermediates/final_byte — NOT from the semantic `Handler` calls, which lose the raw tuple data).
- [x] On `SpecHarness::drop()`, serialize the accumulated tuples to a uniquely-named temp file under `target/spec-chain-uncataloged/<pid>-<atomic-counter>-<nanos>.jsonl` (atomic counter + nanosecond timestamp ensures no overwriting even for sequential tests on the same thread). No file I/O during test execution proper.
- [x] In `spec-coverage-report --check`, add a step that reads all files from `target/spec-chain-uncataloged/`, deduplicates tuples, compares against known catalog tuples, and materializes `plans/spec-conformance/uncataloged-backlog.md`. Fail CI if uncataloged tuples exist.
- [x] Sibling tests: 7 tests — `known_tuple_is_not_double_counted`, `unknown_tuple_is_recorded_in_memory`, `print_and_put_are_not_catalogable`, `osc_dispatch_extracts_command_number`, `esc_dispatch_converts_byte_to_char`, `serialize_and_read_round_trip`, `read_accumulated_tuples_handles_missing_dir`.
- [x] **Validation**: `serialize_and_read_round_trip` feeds fabricated CSI and C0 sequences, serializes to temp dir, reads back and verifies all tuples round-trip. `unknown_tuple_is_recorded_in_memory` verifies a fabricated CSI `?z` sequence is accumulated. `read_accumulated_tuples_handles_missing_dir` verifies graceful handling of nonexistent dir. All 7 tests pass.

---

## 04.R Third Party Review Findings

- [x] `[TPR-04-001-codex][high]` `plans/spec-conformance/section-04-verification-chain-harness.md:515` — Close the GAP between oriterm/tests spec_chain and the GPU test APIs.
  Resolved: Fixed on 2026-04-12. Moved VisualSpecHarness from `oriterm/tests/spec_chain/` to `oriterm/src/gpu/visual_regression/spec_chain/` where `pub(super)` GPU helpers are accessible. Updated all file paths, test commands, and golden locations.
- [x] `[TPR-04-002-codex][high]` `plans/spec-conformance/section-04-verification-chain-harness.md:161` — Fix the LEAK where handler-level recording stands in for parser tuples.
  Resolved: Fixed on 2026-04-12. Added `RecordingPerformer` (implements `vte::Perform`) for rung 1 raw tuple capture alongside `RecordingHandler` for rung 2 semantic dispatch. Two distinct layers: `PerformAction` for parser tuples, `DispatchCall` for handler calls. UncatalogedDetector now uses `PerformAction` tuples.
- [x] `[TPR-04-003-codex][medium]` `plans/spec-conformance/section-04-verification-chain-harness.md:186` — Fix the GAP between the SpecHarness sketch and Term sink ownership.
  Resolved: Fixed on 2026-04-12. Removed `Arc<QueueingEffectSink>` from SpecHarness. Effects are now drained via `handler.term().effect_sink().drain_into()` — the sink is owned by Term, no shared Arc needed.
- [x] `[TPR-04-004-codex][medium]` `plans/spec-conformance/section-04-verification-chain-harness.md:541` — Remove the DRIFT in the sixel pilot file and golden locations.
  Resolved: Fixed on 2026-04-12. Sixel pilot permanently lives at `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_minimal.rs`. Golden stored in `oriterm/tests/references/spec_chain/pilots/sixel_minimal.png` (where `reference_dir()` resolves to). Test commands updated.
- [x] `[TPR-04-005-codex][medium]` `plans/spec-conformance/section-04-verification-chain-harness.md:769` — Close the DRIFT between the citation scanner and the canonical test recipe.
  Resolved: Fixed on 2026-04-12. Expanded scanner contract to accept `//!`, `///`, and `//` comment forms. Added `src/` directories to scan roots since visual spec_chain tests live under `oriterm/src/gpu/visual_regression/spec_chain/`.
- [x] `[TPR-04-006-codex][medium]` `plans/spec-conformance/section-04-verification-chain-harness.md:114` — Break the GAP in the 04 and 05 execution order.
  Resolved: Fixed on 2026-04-12. Documented acyclic dependency model: Section 05 depends on 04 Phase 1a only (04.1-04.3, 04.6, 04.8, 04.9). Phase 1b (04.4, 04.5, 04.7) depends on Section 05. No circular dependency.
- [x] `[TPR-04-001-gemini][high]` `plans/spec-conformance/section-04-verification-chain-harness.md:150` — Fix compilation error from Arc<QueueingEffectSink> not implementing EffectSink.
  Resolved: Fixed on 2026-04-12. Same fix as [TPR-04-003-codex] — removed Arc, drain through Term's owned sink.
- [x] `[TPR-04-002-gemini][medium]` `plans/spec-conformance/section-04-verification-chain-harness.md:550` — Remove syntax error in CoverageReport::build code sketch.
  Resolved: Fixed on 2026-04-12. Removed extra closing brace from the for loop.
- [x] `[TPR-04-001-codex-r2][high]` `section-04:112 + section-05:22` — Make the 04↔05 phase split machine-readable.
  Resolved: Fixed on 2026-04-12. Changed Section 05's `depends_on` from `["04"]` to `["03"]` with comment documenting the acyclic graph: 03 → {04-Phase1a, 05} → 04-Phase1b.
- [x] `[TPR-04-002-codex-r2][medium]` `section-04:8-9,420,532,966` — Remove remaining pre-fix path references.
  Resolved: Fixed on 2026-04-12. Updated all success criteria, placement rules, split summary, and completion checklist to use `oriterm/src/gpu/visual_regression/spec_chain/` (not `oriterm/tests/spec_chain/`).
- [x] `[TPR-04-003-codex-r2][medium]` `section-04:165` — Specify the VTE shim that makes RecordingPerformer viable.
  Resolved: Fixed on 2026-04-12. Mandated `Processor::advance_with_observer()` in vendored VTE crate with `PerformObserver` trait. RecordingPerformer lives inside VTE to avoid dispatch duplication. Removed manual composition alternative.
- [x] `[TPR-04-004-codex-r2][low]` `section-05:96,112` — Unify pinned headless constructor names. NOTE: Out of scope for Section 04 review — flagged for Section 05's review gate.
- [x] `[TPR-04-001-gemini-r2][high]` `section-04:171` — Mandate patching vendored VTE to avoid duplicated dispatch.
  Resolved: Fixed on 2026-04-12. Same fix as [TPR-04-003-codex-r2] — vendored VTE gets `advance_with_observer()`.
- [x] `[TPR-04-002-gemini-r2][high]` `section-04:550` — Move texture/golden observers to src for pub(super) access.
  Resolved: Fixed on 2026-04-12. Updated 04.4 file paths to `oriterm/src/gpu/visual_regression/spec_chain/observers/`.
- [x] `[TPR-04-003-gemini-r2][high]` `section-04:567` — Clarify sixel pilot sequencing against section 05.
  Resolved: Fixed on 2026-04-12. Made 04.5 strictly Phase 1b — lands AFTER Section 05 with deterministic golden natively. Removed throwaway/migration paradox.
- [x] `[TPR-04-004-gemini-r2][medium]` `section-04:261` — Fix effect sink drain method.
  Resolved: Rejected — `QueueingEffectSink::drain_into` takes `&self` (interior Mutex via `parking_lot::Mutex`), verified at `oriterm_core/src/effect/sink/mod.rs:77`. The sketch is correct.
- [x] `[TPR-04-005-gemini-r2][low]` `section-04:915` — Remove Arc<Mutex> from per-harness UncatalogedDetector.
  Resolved: Fixed on 2026-04-12. Changed to plain `HashSet<TupleSig>` — each `SpecHarness` is single-threaded.

- [x] `[TPR-04-001-codex-r3][high]` `crates/oriterm_test_support/src/spec_chain/api.rs:133` — Wire scenario expectations into `run_scenario` via observer functions.
  Resolved: Fixed on 2026-04-12. `run_scenario()` now calls observer functions (observe_parser, observe_dispatch, observe_state, observe_effect) for each rung with `Some` expectation. Stops at first failure.
- [x] `[TPR-04-002-codex-r3][medium]` `crates/oriterm_test_support/src/spec_chain/observers/effect.rs:13` — Expand `EffectExpectation` to match on sub-variant (e.g. `PtyWriteKind`).
  Resolved: Fixed on 2026-04-12. Added `sub_variant: Option<&'static str>` to `EffectExpectation` with `pty()`, `host()`, `family()` const constructors. Observer matches on `PtyWriteKind` name.
- [x] `[TPR-04-003-codex-r3][medium]` `crates/oriterm_test_support/src/spec_chain/observers/parser.rs:14` — Add DCS/OSC/APC matching to parser observer.
  Resolved: Fixed on 2026-04-12. Added `Hook` (merged with `CsiDispatch` arm), `OscDispatch`, `ApcStart`/`ApcEnd` matching.
- [x] `[TPR-04-004-codex-r3][medium]` `crates/oriterm_test_support/src/spec_chain/scenario.rs:278` — Add public const constructors for stub expectation types.
  Resolved: Fixed on 2026-04-12. Changed stub types from `struct Foo { _private: () }` to unit structs `struct Foo;` with `#[derive(Default)]`. Fully const-constructible.

---

## 04.N Completion Checklist

- [ ] Failing test matrix written FIRST (TDD): pilot tests in 04.5 + 04.6 written before observer wiring; observer tests in 04.2/04.3/04.4 written before observer implementation
- [ ] **Matrix dimensions**: rung × scenario type (visual/non-visual) × apex layer × verification status — pilots cover both visual chain (8 rungs to GoldenImage apex) and non-visual chain (3-4 rungs to EffectPtyWrite apex)
- [ ] **Semantic pin**: pilots are the permanent regression guard — `sixel_minimal_drives_every_rung_green` and `da1_query_drives_to_effect_apex` must continue passing for the lifetime of the plan. They're the first tests that prove the harness works; they're also the canary if a future change breaks rung observation.
- [x] `CoreSpecHarness` (rungs 1-4) exists in `oriterm_test_support` with `RecordingHandler` for parser/dispatch capture and renderable observer
- [x] `VisualSpecHarness` (rungs 5-8) exists in `oriterm/src/gpu/visual_regression/spec_chain/` wrapping `CoreSpecHarness` with GPU observation (frame-input, gpu-instance, texture, golden)
- [ ] All observer implementations exist with sibling tests (headless observers under `oriterm_test_support`, visual observers under `oriterm`)
- [x] BLOAT splits applied: `oriterm/src/gpu/prepare/mod.rs` (395 lines) and `oriterm/src/gpu/prepare/dirty_skip/mod.rs` (378 lines) are now under 500 lines (verified by `wc -l` on 2026-04-13)
- [x] **Section 04 ↔ 05 coupling respected**: 04.1–04.3, 04.6, 04.8, 04.9 land in Phase 1a (before 05); 04.3b, 04.4, 04.5, 04.7-finalize land in Phase 1b (after 05.6)
- [x] **Harness split respected**: headless rungs 1-4 in `oriterm_test_support`, visual rungs 5-8 in `oriterm` — no circular dev-dependencies
- [x] `plans/spec-conformance/coverage-baseline.toml` committed with initial all-zero counts
- [ ] Sixel visual pilot test passes on the deterministic lane (after 05.6); golden captured under `tests/references/spec_chain/pilots/sixel_minimal.png` via `headless_env_with_pinned_software_rasterizer`
- [x] DA1 non-visual pilot test passes (3/3 green: drives_to_effect_apex, reply_bytes_match, skips_parser_rung — verified 2026-04-13)
- [ ] `plans/spec-conformance/catalog/README.md` exists with the frozen schema documentation (frozen AFTER 05.6)
- [ ] All catalog files migrated to the frozen schema
- [x] `cargo run -p oriterm_test_support --bin spec-coverage-report` produces a sane per-stack table with ABSOLUTE verified counts (16 stacks, 315 total rows — verified 2026-04-13)
- [x] Coverage report walks BOTH catalog files AND test source files (scans `catalog/*.md` via shared parser + 8 test root dirs for `// Catalog row:` and `catalog_row_id:` citations — verified 2026-04-13)
- [x] `--check` mode of the report binary correctly detects ALL FOUR gates: (a) absolute-verified-count regression, (b) false-verified (no citation), (c) uncataloged citation (no catalog row), (d) non-empty uncataloged-backlog without paired catalog-update PR — all four gates verified, exit code 1 on uncataloged backlog (2026-04-13)
- [x] Cataloging safety net (04.9) lands: `UncatalogedDetector` records tuples in-memory during test execution (`HashSet<TupleSig>`), serializes to temp files on drop, and `spec-coverage-report --check` materializes the backlog in a single serial post-test step. No file I/O during parallel test execution (flaky-test discipline per `.claude/rules/tests.md`). 6 unit tests green.
- [x] Observation hooks in `gpu/prepare/` are gated behind `#[cfg(test)]` (more restrictive than `#[cfg(any(test, debug_assertions))]`) so release builds have zero overhead — verified 2026-04-13
- [x] Alloc regression unchanged: `cargo test -p oriterm_core --test alloc_regression` passes (5/5 green — verified 2026-04-13)
- [x] `./build-all.sh` green (debug + release cross-compile — verified 2026-04-13)
- [x] `./test-all.sh` green debug + release (verified 2026-04-13)
- [x] `./clippy-all.sh` green (host + Windows cross-compile — verified 2026-04-13)
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 04 status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** `CoreSpecHarness` (headless, rungs 1-4) + `VisualSpecHarness` (GPU, rungs 5-8) drive both pilots through every applicable rung green; `RecordingHandler` captures parser/dispatch observations; catalog row schema frozen and section 01 catalogs migrated; `coverage-baseline.toml` committed with initial counts; coverage report binary works with all four CI gates; `UncatalogedDetector` accumulates tuples in-memory (no file I/O during parallel tests); BLOAT files split under 500 lines; full test suite green debug + release; alloc regression unchanged. Sections 08-20 can now be written against a stable harness API and a frozen catalog schema.
