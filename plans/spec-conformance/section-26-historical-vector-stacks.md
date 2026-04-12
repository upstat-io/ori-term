---
section: "26"
title: "Historical Vector Stacks (ReGIS + Tek 4010/4014 + shared vector_raster helper)"
status: not-started
reviewed: false
goal: "Drive every VECTOR-GRAPHICS catalog row in `catalog/historical.md` to `verified` by IMPLEMENTING the ReGIS command language interpreter, the Tektronix 4010/4014 vector mode interpreter, and a shared `vector_raster` helper module that both interpreters rasterize through. Split out from Section 19 because the vector stacks depend on Section 05 (deterministic golden lane for rasterizer goldens) and Section 07 (image lifecycle for the `ImageCache` placements these interpreters emit) in addition to Section 08 — the legacy-control stacks in Section 19 depend only on 08."
success_criteria:
  - "**Shared `vector_raster` helper IMPLEMENTED** at `oriterm_core/src/vector_raster/`: Bresenham line, midpoint circle, midpoint arc, Catmull-Rom curve, even-odd fill polygon, stroke text. Public API: `VectorCanvas::{new, clear, move_to, line_to, draw_circle, draw_arc, draw_curve, fill_polygon, draw_text, to_image_placement}`. Both ReGIS and Tek interpreters rasterize through this helper."
  - "**ReGIS** IMPLEMENTED: `oriterm_core/src/regis/` (new module) parses ReGIS command language (screen commands, position commands, write commands, arc/circle/curve commands, text commands, macro definitions), rasterizes vector output into the existing `ImageCache` placement model via `vector_raster::VectorCanvas`, and renders via the existing GPU image pipeline. The interpreter is minimal (2D vector primitives + simple raster) but COMPLETE for the documented DEC ReGIS reference manual sequences."
  - "**Tek 4010/4014 vector mode** IMPLEMENTED: `oriterm_core/src/tektronix/` (new module) parses Tek 4014 byte-pair coordinate addressing, draw/move modes, alpha (character) mode vs graphics mode switching, and status-reply sequences. The interpreter rasterizes the vector output into an `ImageCache` placement through the shared `vector_raster::VectorCanvas`."
  - "Vector rasterizer goldens (line/circle/arc/curve/fill) reproducible on the Section 05 deterministic lane — exact-or-tiny per-pixel match"
  - "ReGIS + Tek rasterized placements survive grid resize/reflow/scrollback eviction because they go through `ImageCache` and inherit Section 07's `on_resize` handler"
  - "Every ReGIS and Tek 4014 row in `catalog/historical.md` is `verified` (NOT `verified-with-deviation` — the deferral fork is gone)"
  - "All existing tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row**"
inspired_by:
  - "xterm `graphics/regis.c` — partial ReGIS implementation, used as a reference for the xterm-observable subset"
  - "xterm `Tekproc.c` — Tektronix 4014 emulation reference, byte-pair coordinate decoder"
  - "DEC ReGIS technical manual — canonical spec for the ReGIS command language"
  - "Tektronix 4014 Programmer's Reference Manual — canonical spec for the byte-pair coordinate format and graphics/alpha mode switching"
depends_on: ["05", "07", "08"]
# vector stacks depend on 05 (deterministic
# golden lane for rasterizer goldens), 07 (image lifecycle — ReGIS/Tek rasterize to
# ImageCache so they inherit on_resize), and 08 (baseline solid).
third_party_review:
  status: none
  updated: null
sections:
  - id: "26.1"
    title: "Implement shared vector-to-raster helper (VectorCanvas + primitives)"
    status: not-started
  - id: "26.2"
    title: "Implement ReGIS command language interpreter + rasterizer"
    status: not-started
  - id: "26.3"
    title: "Implement Tek 4010/4014 vector mode interpreter + rasterizer"
    status: not-started
  - id: "26.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "26.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement: 26.1 (after shared helper lands, before interpreters consume
# it), 26.3 (after both interpreters are integrated — covers .2-.3), final in 26.N
---

# Section 26: Historical Vector Stacks

**Status:** Not Started
**Goal:** Implement and verify every vector-graphics historical stack (ReGIS, Tek 4010/4014) plus the shared `vector_raster` helper they both rasterize through. Split out from Section 19 because vector stacks have a wider dependency footprint (05 + 07 + 08) than the legacy control stacks (08 only) and because the shared helper makes them a coherent unit.

**Success Criteria:** see frontmatter.

**Context:** ReGIS and Tek 4014 both interpret vector-drawing commands and produce raster output for the GPU image pipeline. Sharing the rasterizer between the two interpreters keeps each one small and avoids duplicating line/curve/arc rasterization code. Both interpreters push rasterized canvases into `ImageCache` as image placements, so they inherit Section 07's lifecycle handling (resize/reflow/scrollback). Their committed rasterizer goldens require Section 05's deterministic golden lane — without it, the line/circle/arc/curve primitives will rasterize with tiny differences across GPU drivers and CI runs will flake.

**No verified-with-deviation escape hatch.** Per the cohesion pass, this section does not contain a decision fork between implement and defer. Both vector stacks get implemented.

**Reference implementations:** see frontmatter.

**Depends on:**
- **Section 08** (baseline solid — parser/dispatch infrastructure in place for DCS/CSI routing)
- **Section 07** (image lifecycle — `ImageCache::on_resize` exists so rasterized placements survive grid mutations)
- **Section 05** (deterministic golden lane — `headless_env_with_pinned_software_rasterizer` exists so rasterizer goldens are reproducible cross-machine)

---

## 26.1 Implement shared vector-to-raster helper (VectorCanvas + primitives)

**File(s):** `oriterm_core/src/vector_raster/mod.rs` (new), `oriterm_core/src/vector_raster/shapes.rs` (new), `oriterm_core/src/vector_raster/tests.rs` (new)

ReGIS and Tek 4014 both interpret vector-drawing commands and produce raster output for the GPU image pipeline. Sharing the rasterizer between the two interpreters keeps each one small and avoids duplicating line/curve/arc rasterization code.

- [ ] Create `oriterm_core/src/vector_raster/mod.rs` as the public module root (re-exports `VectorCanvas`, internal `shapes` private). Include `#[cfg(test)] mod tests;` (sibling tests.rs per `.claude/rules/test-organization.md`).
- [ ] Define a `VectorCanvas` struct that exposes:
  ```rust
  pub struct VectorCanvas {
      pub width: u32,
      pub height: u32,
      pub pixels: Vec<u32>, // RGBA8, origin top-left
      pub pen_pos: (i32, i32),
      pub pen_color: u32,
      pub bg_color: u32,
  }

  impl VectorCanvas {
      pub fn new(width: u32, height: u32) -> Self;
      pub fn clear(&mut self);
      pub fn move_to(&mut self, x: i32, y: i32);
      pub fn line_to(&mut self, x: i32, y: i32); // Bresenham
      pub fn draw_circle(&mut self, cx: i32, cy: i32, radius: u32); // midpoint
      pub fn draw_arc(&mut self, cx: i32, cy: i32, radius: u32, start: f32, end: f32);
      pub fn draw_curve(&mut self, pts: &[(i32, i32)]); // Catmull-Rom or similar
      pub fn fill_polygon(&mut self, pts: &[(i32, i32)]);
      pub fn draw_text(&mut self, text: &str, x: i32, y: i32); // stroke-based text
      pub fn to_image_placement(&self) -> ImageData; // feed into ImageCache
  }
  ```
- [ ] Implement Bresenham line, midpoint circle, midpoint arc, Catmull-Rom curve, even-odd fill polygon. Each primitive is a free function in `shapes.rs` that the `VectorCanvas` methods delegate to (keeps `mod.rs` under 500 lines per `impl-hygiene.md`).
- [ ] Sibling tests in `oriterm_core/src/vector_raster/tests.rs` (per test-organization.md — `#[cfg(test)] mod tests;` at bottom of `mod.rs`, tests in sibling file, `super::` imports):
  - `bresenham_line_horizontal_renders_expected_pixels()`
  - `bresenham_line_diagonal_matches_reference_golden()` — compare against committed small PNG under `oriterm_core/tests/references/vector_raster/line_diag.png`
  - `midpoint_circle_radius_10_matches_reference()`
  - `midpoint_arc_quarter_matches_reference()`
  - `catmull_rom_curve_4_points_matches_reference()`
  - `even_odd_fill_polygon_triangle_matches_reference()`
  - `to_image_placement_produces_imagedata_with_correct_dimensions()`
- [ ] BLOAT check: if `mod.rs` approaches 500 lines, extract more submodules proactively (e.g. `curves.rs`, `fills.rs`).
- [ ] Alloc regression: `VectorCanvas` allocates `pixels: Vec<u32>` ONCE in `new()`; no per-primitive allocation.
- [ ] **Validation**: vector primitives match reference goldens on the deterministic lane (Section 05); canvas converts to `ImageData` cleanly.
- [ ] **TPR checkpoint** — `/tpr-review` covering 26.1 (shared helper) before either interpreter consumes it.

---

## 26.2 Implement ReGIS command language interpreter + rasterizer

**File(s):** `oriterm_core/src/regis/mod.rs` (new), `oriterm_core/src/regis/parser.rs` (new), `oriterm_core/src/regis/interpreter.rs` (new), `oriterm_core/src/regis/tests.rs` (new), catalog row updates, `oriterm_core/tests/spec_chain/historical/regis.rs` (new)

ReGIS is a DEC vector graphics command language. Commands are ASCII characters (e.g. `P` for position, `V` for vector, `C` for circle, `A` for arc, `T` for text, `S` for screen commands, `W` for write state), with parameters in parentheses. ReGIS enters via `DCS p ... ST` and exits via `ST`. The interpreter parses commands, dispatches to the `VectorCanvas` helper, and on `ST` finalizes the raster into an `ImageCache` placement.

- [ ] Create `oriterm_core/src/regis/parser.rs`:
  - Tokenize ReGIS command bytes into `RegisCommand { op: char, args: Vec<RegisArg> }`
  - Handle parentheses/brackets for parameter groups
  - Handle macros (ReGIS `@` macro expansion)
- [ ] Create `oriterm_core/src/regis/interpreter.rs`:
  - State machine: current pen position, current pen color, current text size, active screen region
  - Dispatch commands: P (position), V (vector), C (circle), A (arc), T (text), S (screen setup), W (write state), L (load color), F (fill), R (rubber banding — no-op acceptable)
  - On `ST`, finalize the canvas into an `ImageData` and push it into `ImageCache` via an `ImagePlacement` at the current cursor row/col
- [ ] Hook the DCS p introducer in `crates/vte/src/ansi/dispatch/mod.rs` — route `DCS p ... ST` to the ReGIS interpreter
- [ ] Sibling tests in `oriterm_core/src/regis/tests.rs` (per test-organization.md: `#[cfg(test)] mod tests;` in `mod.rs`, sibling file with `super::` imports, no inline `mod tests { ... }` wrapper):
  - `parser_tokenizes_position_and_vector()` — `P(100,100) V(200,200)`
  - `interpreter_draws_10x10_square()` — `P(0,0) V(10,0) V(10,10) V(0,10) V(0,0)`
  - `interpreter_draws_circle_via_C50()` — `C(50)` at current pen position, radius 50
  - `interpreter_draws_arc_via_A_start_end_radius()`
  - `interpreter_applies_macro_at_expansion()`
- [ ] Spec_chain tests in `oriterm_core/tests/spec_chain/historical/regis.rs`:
  - Feed a ReGIS program that draws a simple vector shape; assert the `ImageCache` contains a placement; assert the rasterized output matches a committed golden PNG via the Section 05 deterministic lane
  - Image placement survives `Grid::resize(new_cols, new_rows)` — ReGIS placement goes through `ImageCache::on_resize` (Section 07)
- [ ] Update `catalog/historical.md` ReGIS rows to `verified`
- [ ] BLOAT check: keep `parser.rs`, `interpreter.rs` each under 500 lines — split further (`interpreter/state.rs`, `interpreter/dispatch.rs`) if needed.
- [ ] **Validation**: ReGIS primitives rasterize correctly; images integrate with image cache lifecycle (Section 07) so resize/scrollback work.

---

## 26.3 Implement Tek 4010/4014 vector mode interpreter + rasterizer

**File(s):** `oriterm_core/src/tektronix/mod.rs` (new), `oriterm_core/src/tektronix/parser.rs` (new), `oriterm_core/src/tektronix/interpreter.rs` (new), `oriterm_core/src/tektronix/tests.rs` (new), catalog row updates, `oriterm_core/tests/spec_chain/historical/tek_4014.rs` (new)

Tek 4014 uses byte-pair coordinate addressing: high-Y byte, low-Y byte, high-X byte, low-X byte encode a 12-bit coordinate. Mode is switched via `GS` (graphics), `US` (alpha/text), `ESC FF` (clear screen). The terminal transitions between alpha (character) mode and graphics (vector) mode.

- [ ] Create `oriterm_core/src/tektronix/parser.rs`:
  - State machine that recognizes mode-switching bytes: `ESC FF` (clear), `GS` (graphics mode), `US` (alpha mode), `ESC ETB` (end of text)
  - In graphics mode, decode byte-pair coordinates — each coordinate is up to 4 bytes
- [ ] Create `oriterm_core/src/tektronix/interpreter.rs`:
  - Track current mode, current pen position, current pen state (dark/light/vector/point/incremental)
  - Dispatch draw commands to the shared `VectorCanvas`
  - On mode switch or explicit finalization, push the raster into `ImageCache`
- [ ] Hook byte-level Tek entry: when ori_term sees the Tek entry sequence (or is in Tek compatibility mode via a DEC mode toggle), the byte stream routes through the Tek parser
- [ ] Sibling tests in `oriterm_core/src/tektronix/tests.rs` (test-organization.md compliant):
  - `byte_pair_decoder_0x20_0x20_0x40_0x40_decodes_to_expected_xy()`
  - `mode_switch_gs_enters_graphics_and_us_returns_to_alpha()`
  - `draw_vector_through_shared_vector_canvas_emits_imagedata()`
- [ ] Spec_chain tests in `oriterm_core/tests/spec_chain/historical/tek_4014.rs`:
  - Feed a Tek 4014 program that draws a shape; assert the `ImageCache` contains the rasterized placement
  - Tek rasterized placement survives `Grid::resize` (Section 07 handler)
  - Tek vector golden matches committed PNG on the Section 05 deterministic lane
- [ ] Update `catalog/historical.md` Tek 4010/4014 rows to `verified`
- [ ] BLOAT check: keep `parser.rs`, `interpreter.rs` each under 500 lines.
- [ ] **Validation**: Tek byte-pair decoder is exact; vector drawing rasterizes; existing tests still pass.
- [ ] **TPR checkpoint** — `/tpr-review` covering 26.2-26.3 (both interpreters integrated with the shared helper).

---

## 26.R Third Party Review Findings

- None.

---

## 26.N Completion Checklist

- [ ] Failing test matrix written FIRST (TDD): rasterizer primitives, ReGIS interpreter, Tek interpreter spec_chain tests are written and failing before implementation lands
- [ ] **Matrix dimensions**: primitive (line/circle/arc/curve/fill) × interpreter (ReGIS/Tek) × lifecycle event (fresh-rasterize, resize-survive, scrollback-prune) × rung (parser/dispatch/interpreter state/ImageCache placement/GPU golden)
- [ ] **Semantic pin**: each vector primitive has a committed golden on the Section 05 deterministic lane; ReGIS and Tek spec_chain tests are the regression guards for both interpreters
- [ ] Shared `vector_raster` helper exists at `oriterm_core/src/vector_raster/` and is used by both ReGIS and Tek
- [ ] ReGIS command interpreter implemented and verified (via spec_chain + golden)
- [ ] Tek 4010/4014 vector mode interpreter implemented and verified
- [ ] No catalog rows marked `verified-with-deviation` for implementation-skip reasons
- [ ] BLOAT check: none of the new modules exceed 500 lines (`oriterm_core/src/regis/`, `oriterm_core/src/tektronix/`, `oriterm_core/src/vector_raster/` all split as needed per `impl-hygiene.md`)
- [ ] Vector rasterizer goldens reproducible on the deterministic lane (Section 05)
- [ ] Rasterized placements survive `Grid::resize` / reflow / scrollback eviction via `ImageCache::on_resize` (Section 07)
- [ ] All existing tests pass
- [ ] Alloc regression unchanged (vector rasterizer allocates once per raster — must not allocate on hot paths)
- [ ] Cross-platform: rasterizer compiles and runs on macOS / Linux / Windows (no platform-specific code; pure CPU rasterization)
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` + `index.md` updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** `vector_raster` helper exists; ReGIS and Tek 4010/4014 interpreters both use it; every ReGIS and Tek row in `catalog/historical.md` is `verified`; rasterized placements survive grid mutations via `ImageCache::on_resize`; rasterizer goldens reproducible on the deterministic lane.
