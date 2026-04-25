---
section: "01"
title: "Per-cell emit helper extraction (F-03 → resolves F-04, F-05, F-06, F-08)"
status: complete
reviewed: true
goal: "Eliminate the ~120-line per-cell emit body duplicated across `fill_frame_shaped`, `fill_frame_incremental`, and the test-only unshaped `fill_frame` by extracting one canonical `emit_cell` helper that all three callers consume."
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-04-25
sections:
  - id: "01.1"
    title: "Identify the per-cell emit skeleton"
    status: complete
  - id: "01.2"
    title: "Extract emit_cell helper"
    status: complete
  - id: "01.3"
    title: "Migrate fill_frame_shaped"
    status: complete
  - id: "01.4"
    title: "Migrate fill_frame_incremental (dirty-row path)"
    status: complete
  - id: "01.5"
    title: "Migrate test-only unshaped fill_frame"
    status: complete
  - id: "01.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "01.N"
    title: "Build & Verify"
    status: complete
---

# Section 01: Per-cell emit helper extraction

**Goal:** Replace the three duplicated per-cell emit bodies in
`gpu/prepare/{mod.rs, dirty_skip/mod.rs, unshaped.rs}` with a single canonical
helper. After this section lands, a future change to per-cell emit logic
(e.g. a new SGR effect on `glyph_y`, a new background-quad rule, a new
decoration kind) lands at exactly one site.

**Production code path:** `GpuRenderer::draw_frame()` shaped pipeline →
`fill_frame_shaped` (cold path) and `fill_frame_incremental` (hot path) in
`oriterm/src/gpu/prepare/mod.rs` and `oriterm/src/gpu/prepare/dirty_skip/mod.rs`.
The test-only `fill_frame` in `oriterm/src/gpu/prepare/unshaped.rs` exercises
the same emit shape under unit-test conditions.

**Observable change:** None at runtime — this is a pure refactor. The
behavioral pin is that BUG-06-014's `glyph_y` shift, decoration anchoring,
and BLINK alpha continue to land identically in shaped, incremental, and
unshaped paths.

**Context:** BUG-06-014 (SGR 73/74 superscript / subscript) had to apply the
same `super_sub_glyph_offset(cell.flags, ch)` adjustment at three different
sync points:

- `oriterm/src/gpu/prepare/mod.rs:342` (`fill_frame_shaped`)
- `oriterm/src/gpu/prepare/dirty_skip/mod.rs:307` (`fill_frame_incremental`)
- `oriterm/src/gpu/prepare/unshaped.rs:158` (`fill_frame`, test-only)

The Phase 5 hygiene report flagged this as Critical LEAK F-03
(algorithmic-duplication, 3+ sites). Per `.claude/rules/impl-hygiene.md`
§Algorithmic DRY, three sync points hitting in one fix is the textbook
3-strike threshold for forced extraction. F-04 (BLOAT 260-line
`fill_frame_incremental`), F-05 (BLOAT 215-line `fill_frame_shaped`), F-06
(BLOAT 138-line test-only `fill_frame`), and F-08 (LEAK:scattered-knowledge
of `super_sub_glyph_offset` 3-site call) all dissolve as a side-effect of
this extraction.

**Reference implementations:**
- **WezTerm** `wezterm-gui/src/termwindow/render/mod.rs`: per-cell emit is
  centralized in `render_screen_line` with a single `paint_cell` step;
  shaped and unshaped fall-throughs route the same data shape into one
  emitter.
- **Alacritty** `alacritty/src/renderer/text.rs`: `RenderApi::render_cell`
  is the single per-cell emission point; the batch loop in
  `RenderApi::render_string` is the only caller.

**Depends on:** None. Section 01 is the structural foundation; subsequent
sections operate on the post-extraction shape.

---

## 01.1 Identify the per-cell emit skeleton

**File(s):** read-only audit of
`oriterm/src/gpu/prepare/mod.rs:217-432`,
`oriterm/src/gpu/prepare/dirty_skip/mod.rs:130-389`,
`oriterm/src/gpu/prepare/unshaped.rs:66-204`.

**Goal:** Produce a written diff-aligned skeleton listing every step that
must move into the helper, every input the helper consumes, and every
output it produces. This becomes the spec for the helper signature in 01.2.

- [x] Diff the three per-cell bodies side-by-side. Confirm the shared
      skeleton is:
  1. Spacer skip (`flags.intersects(WIDE_CHAR_SPACER | LEADING_WIDE_CHAR_SPACER)` → `continue`).
     **Note**: this predicate is the target of Section 02 (`is_spacer()`).
     Section 01 leaves the predicate inline; Section 02 then migrates the
     single canonical site.
  2. Column / x / y compute (cell-top y, not glyph-y). **Caller responsibility** — y-rounding with row transition differs per function.
  3. `resolve_cell_colors(cell, palette, …)` → fg, bg.
  4. Wide-char `bg_w` branch (double-width background quad).
  5. Background instance push.
  6. BLINK alpha (`cell_dim`, `deco_alpha`).
  7. `DecorationContext::draw(cell.flags, …)` — anchored to cell-top y.
  8. `super_sub_glyph_offset(cell.flags, ctx.cell_size.height)` — glyph y-shift only. Note: `ch` in the existing code is `cell_size.height` (f32), NOT a char — `cell.ch` is the character.
  9. Built-in branch (shaped/incremental only — unshaped uses `atlas.lookup` for all chars). Shared between production paths.
  10. Shaped glyph emission (`GlyphEmitter`) or unshaped atlas lookup (test-only).
- [x] Document inputs (the helper signature must accept):
      `cell: &RenderableCell`, `col: usize`, `row: usize` (for `is_hovered`),
      `x: f32`, `y: f32` (pre-computed at caller), `ctx: &mut EmitCtx<'_>`.
      `EmitCtx<'a>` bundles: `fg_dim`, `text_blink_opacity`, `subpixel_positioning`,
      `palette: &FramePalette`, `sel: Option<&FrameSelection>`, `search: Option<&FrameSearch>`,
      `cursor: RenderableCursor`, `cursor_opacity`, `hovered_cell`, `cell_size: &CellMetrics`,
      `atlas: &dyn AtlasLookup`, `size_q6: u32`, `frame: &mut PreparedFrame`,
      `glyph_mode: GlyphMode<'a>`. `GlyphMode::Shaped { shaped: &ShapedFrame, hinted: bool }`
      vs `GlyphMode::Unshaped`. `baseline` comes from `ctx.cell_size.baseline` (no duplication).
- [x] Document outputs (side-effects only — no return value):
      pushes into `frame.backgrounds` (bg quad), `frame.glyphs` / `frame.subpixel_glyphs` /
      `frame.color_glyphs` (glyph instances), and calls `DecorationContext::draw` which
      pushes decoration instances into those same writers. No return value needed — all
      callers proceed to post-loop bookkeeping regardless of per-cell results.
- [x] Confirm by grep that no fourth caller exists. Any future
      `fill_frame_*` variant must consume `emit_cell`, not duplicate the
      body. (Audit grep: `rg -n "resolve_cell_colors\(" oriterm/src/gpu/` confirms
      exactly 3 call sites: `mod.rs:286`, `dirty_skip/mod.rs:253`, `unshaped.rs:97`.)

---

## 01.2 Extract emit_cell helper

**File(s):** new module — recommended path
`oriterm/src/gpu/prepare/emit_cell.rs` (sibling of existing `emit.rs`),
or extend `oriterm/src/gpu/prepare/emit.rs` if the existing module is the
natural home and remains under the 500-line limit.

**Context:** The helper must compile in production (consumed by
`fill_frame_shaped` and `fill_frame_incremental`) AND in test mode
(consumed by the test-only unshaped `fill_frame`). It must NOT be
`#[cfg(test)]`-gated.

**Fix approach — 2 options:**

**(a) Free function `emit_cell(...)`** (recommended — simpler, no lifetime
acrobatics):

```rust
// oriterm/src/gpu/prepare/emit_cell.rs

pub(super) fn emit_cell(
    cell: &Cell,
    ch: char,
    col: usize,
    row_y_top: f32,
    cell_w: f32,
    cell_h: f32,
    ctx: &mut EmitCtx<'_>,
) {
    // 1. (Spacer skip is the caller's responsibility — kept at the loop
    //    boundary so the caller can `continue` cleanly. Section 02 will
    //    migrate the predicate to CellFlags::is_spacer.)

    // 2-10. Per-cell emit body — single canonical implementation.
    let (fg, bg) = resolve_cell_colors(cell, ctx.palette, …);
    let bg_w = if cell.flags.contains(CellFlags::WIDE_CHAR) { 2.0 * cell_w } else { cell_w };
    ctx.bg.push(BgInstance::new(col, row_y_top, bg_w, cell_h, apply_blink_alpha(bg, cell)));
    ctx.decorations.draw(cell, col, row_y_top, cell_w, cell_h, fg);
    let glyph_y = row_y_top + super_sub_glyph_offset(cell.flags, ch);
    if let Some(builtin) = builtin_glyph(ch) {
        ctx.glyphs.push_builtin(builtin, col, glyph_y, cell_w, cell_h, fg);
    } else {
        ctx.glyphs.push_shaped(ch, cell.flags, col, glyph_y, cell_w, cell_h, fg);
    }
}

pub(super) struct EmitCtx<'a> {
    pub palette: &'a Palette,
    pub bg: &'a mut Vec<BgInstance>,
    pub glyphs: &'a mut GlyphInstances,
    pub decorations: &'a mut DecorationContext,
}
```

**Why this is best:** Free functions with explicit `EmitCtx<'_>` are
the lightest abstraction that makes the dependency graph obvious.
There is no inheritance, no method-resolution cleverness, and the helper
is easy to test in isolation.

**Trade-off:** `EmitCtx` adds a single struct definition. Worth it because
otherwise the helper takes 8+ positional parameters.

**(b) Inherent method on a `PerCellEmitter` struct** (alternative — more
state-heavy):

```rust
struct PerCellEmitter<'a> { palette: &'a Palette, bg: &'a mut Vec<BgInstance>, … }
impl<'a> PerCellEmitter<'a> {
    fn emit(&mut self, cell: &Cell, ch: char, col: usize, row_y_top: f32, cell_w: f32, cell_h: f32) { … }
}
```

**Downside:** The struct adds construction boilerplate at three call sites
without buying anything beyond what `EmitCtx` already does.

**Recommended path:** Option (a). Cite this in 01.R if codex/gemini
challenges the choice.

- [x] Create `oriterm/src/gpu/prepare/emit_cell.rs` (or extend existing
      `emit.rs` — pick whichever keeps the file under 500 lines).
      Decision: new directory module `emit_cell/mod.rs` + `emit_cell/tests.rs`.
- [x] Define `pub(super) struct EmitCtx<'a> { ... }` capturing every
      mutable reference the helper needs.
      Shape used: `shaped: Option<(&'a ShapedFrame, bool)>` is Copy; copied
      out of ctx before glyph-emit to avoid split-borrow conflicts.
- [x] Implement `pub(super) fn emit_cell(...)` containing the canonical
      per-cell body documented in 01.1.
      Final signature: `(cell: &RenderableCell, x: f32, y: f32, ctx: &mut EmitCtx)` —
      col/row removed (read from cell) to keep params ≤ 4.
- [x] Add `pub(super) mod emit_cell;` to `oriterm/src/gpu/prepare/mod.rs`
      (or re-export from `emit.rs`).
- [x] Add `oriterm/src/gpu/prepare/emit_cell/tests.rs` (per the
      sibling-tests rule in `.claude/rules/test-organization.md`) with
      direct unit tests for the helper:
  - [x] `emit_cell_pushes_bg_instance_with_correct_dims`
  - [x] `emit_cell_applies_super_sub_glyph_offset_to_glyph_only`
  - [x] `emit_cell_uses_bg_w_for_wide_char` (BUG-06-014 negative pin)
  - [x] `emit_cell_routes_builtin_glyph_via_builtin_branch`
  - [x] `emit_cell_anchors_decoration_to_cell_top_y` (negative pin: glyph_y
        must NOT bleed into decoration draw)
  - [x] `emit_cell_applies_blink_alpha_to_bg`

---

## 01.3 Migrate fill_frame_shaped

**File(s):** `oriterm/src/gpu/prepare/mod.rs:217-432`.

**Context:** `fill_frame_shaped` is the cold (full-frame) shaped pipeline.
After this migration the function becomes:

```
setup → row_ranges loop {
    for col in 0..cols {
        if cell.is_spacer() { continue; }   // Section 02 will replace inline pred
        emit_cell(cell, ch, col, row_y_top, cell_w, cell_h, &mut ctx);
    }
} → final-row range record → overlays.
```

- [x] Replace the per-cell body (mod.rs:242-389) with a single
      `emit_cell(cell, ch, col, row_y_top, cell_w, cell_h, &mut ctx)` call.
- [x] Construct `EmitCtx` at the top of the row loop (or once per
      `fill_frame_shaped` if all references are stable across rows).
      EmitCtx constructed once before loop; `frame` moved into ctx.
- [x] Verify the `#[expect(clippy::too_many_lines, reason = "linear pipeline...")]`
      attribute at lines 213-216 is no longer needed and remove it. If clippy
      still flags the function, restructure further (do not silence with
      `#[expect]`).
      Removed. fill_frame_shaped is now ~90 lines, well under 100.
- [x] Run `cargo test -p oriterm --lib gpu::prepare` and confirm every
      shaped-path test in `gpu/prepare/tests.rs` is still green, including
      BUG-06-014 regressions:
  - `shaped_overline_emits_top_rect_*`
  - `shaped_superscript_shifts_glyph_y_*`
  - `shaped_subscript_shifts_glyph_y_*`
  - `shaped_overline_anchors_to_cell_top_*` (negative pin)

---

## 01.4 Migrate fill_frame_incremental (dirty-row path only)

**File(s):** `oriterm/src/gpu/prepare/dirty_skip/mod.rs:130-389`.

**Context:** `fill_frame_incremental` has two paths: dirty rows
(re-emit per-cell) and clean rows (replay `RowInstanceRanges` from the
saved tier). Only the dirty-row path duplicates the per-cell emit body.
The clean-row replay stays in this function.

- [x] Replace the per-cell body (dirty_skip/mod.rs:239-351) inside the
      dirty-row branch with a single `emit_cell(...)` call, matching the
      shape used in 01.3.
- [x] Leave the clean-row replay (saved_tier read + `frame.bg.extend` /
      `frame.glyphs.extend` from cached ranges) UNCHANGED.
      Clean-row replay is now in `replay_clean_row` helper.
- [x] Verify the `#[expect(clippy::too_many_lines, reason = "mirrors fill_frame_shaped structure")]`
      attribute is no longer needed and remove it. If clippy still flags
      the function, factor the dirty-row dispatch into a small named
      helper rather than silencing.
      Removed. Factored into `process_incremental_cells` + `replay_clean_row` +
      `push_dirty_row_range` helpers. fill_frame_incremental is now ~55 lines.
- [x] Run `cargo test -p oriterm --lib gpu::prepare::dirty_skip` and
      confirm every incremental-path test in `dirty_skip/tests.rs` is
      still green, including:
  - `incremental_dirty_row_with_superscript_shifts_glyph_y` (BUG-06-014 pin)
  - `incremental_clean_row_replays_overline_from_cache` (after F-17 lands
    inline; if this test name does not yet exist when Section 01 starts,
    coordinate with the inline F-17 fix landing in BUG-06-014's close-out
    commit so it lands first)

---

## 01.5 Migrate test-only unshaped fill_frame

**File(s):** `oriterm/src/gpu/prepare/unshaped.rs:66-204`.

**Context:** `unshaped::fill_frame` is gated `#[cfg(test)]` (verified at
`prepare/mod.rs:17-18`). It exists so unit tests can exercise the
prepare pipeline without invoking the shaped path. After 01.5 it consumes
the same `emit_cell` helper as production.

- [x] Replace the per-cell body (unshaped.rs:85-204) with a single
      `emit_cell(...)` call, matching the shape used in 01.3 / 01.4.
      Uses `ctx.shaped = None` (unshaped mode in emit_cell).
- [x] Confirm the test-only function still produces byte-identical
      `frame.bg` / `frame.glyphs` output for the unit-test workloads in
      `gpu/prepare/tests.rs` that drive the unshaped path.
      ./test-all.sh green.
- [x] Remove `unshaped.rs`'s `#[expect(clippy::too_many_lines, ...)]` if
      present (it should not be needed post-migration).
      No such attribute existed in unshaped.rs.

---

## 01.R Third Party Review Findings

Track findings from `/tpr-review` runs against Section 01 here. Leave the
block in place even when empty so tooling has a stable anchor.

- None.

When findings exist, use this format:

```
- [ ] `[TPR-01-NNN][high]` `path/to/file.rs:123` — Concrete finding summary.
  Validation: {How the reviewer proved it.}

- [x] `[TPR-01-NNN][medium]` `path/to/file.rs:456` — Concrete finding summary.
  Resolved: Accepted and integrated into 01.X on YYYY-MM-DD.
```

Rules:
- Only reject findings that are factually incorrect.
- Do not delete historical findings; mark them resolved with rationale.
- If unchecked findings exist, set `third_party_review.status: findings`.
- If all findings are resolved, set `third_party_review.status: resolved`.
- If the block contains only `- None.`, set `third_party_review.status: none`.

---

## 01.N Build & Verify

### TDD Matrix

| Test name (in `prepare/emit_cell/tests.rs` or existing `prepare/tests.rs`) | Pin type | Lock-in target |
|---|---|---|
| `emit_cell_pushes_bg_instance_with_correct_dims` | semantic | bg quad geometry |
| `emit_cell_uses_bg_w_for_wide_char` | semantic | wide-char bg width invariant |
| `emit_cell_applies_blink_alpha_to_bg` | semantic | BLINK alpha on bg |
| `emit_cell_applies_super_sub_glyph_offset_to_glyph_only` | semantic + negative | glyph_y receives offset |
| `emit_cell_anchors_decoration_to_cell_top_y` | **negative** | decoration must NOT receive glyph_y offset (BUG-06-014 invariant) |
| `emit_cell_routes_builtin_glyph_via_builtin_branch` | semantic | builtin path takes precedence |
| Existing `shaped_*` tests in `prepare/tests.rs` | regression | shaped path still green |
| Existing `incremental_*` tests in `dirty_skip/tests.rs` | regression | incremental path still green |
| Existing `unshaped_*` tests in `prepare/tests.rs` | regression | test-only path still green |

### Completion Checklist

- [x] `./build-all.sh` passes (debug + release cross-compile)
- [x] `./clippy-all.sh` passes — no `#[expect(clippy::too_many_lines)]` on
      `fill_frame_shaped` or `fill_frame_incremental`
- [x] `./test-all.sh` passes
- [x] New `emit_cell` helper has direct unit tests covering every TDD
      matrix row marked semantic / negative
- [x] No `#[allow(dead_code)]` on `emit_cell` or `EmitCtx`
- [x] All three callers (`fill_frame_shaped`, `fill_frame_incremental`
      dirty-row path, test-only unshaped `fill_frame`) consume `emit_cell`
- [x] Repo grep confirms zero remaining duplicates of the per-cell skeleton
      (`rg -n "super_sub_glyph_offset\(" oriterm/src/gpu/` returns exactly
      one production call site — inside `emit_cell`)
- [x] `/tpr-review` against this section returns clean (or all findings
      `[x]` resolved in 01.R)
- [x] BUG-06-014 regression tests still green (overline/superscript/subscript)

**Exit Criteria:** A `cargo test -p oriterm --lib gpu::prepare` invocation
runs the full suite green AND `rg -n "resolve_cell_colors\(" oriterm/src/gpu/`
returns exactly one production call (inside `emit_cell`). Section 01 is
complete.
