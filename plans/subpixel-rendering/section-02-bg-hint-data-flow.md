---
section: "02"
title: "Background Hint Data Flow"
status: complete
reviewed: true
goal: "Pass real cell bg color to subpixel glyphs through the full render pipeline"
depends_on: ["01", "03"]
sections:
  - id: "02.1"
    title: "GlyphEmitter bg Plumbing"
    status: complete
  - id: "02.2"
    title: "Incremental Path Support"
    status: complete
  - id: "02.3"
    title: "Completion Checklist"
    status: complete
---

# Section 02: Background Hint Data Flow

**Status:** Not Started
**Goal:** Terminal grid subpixel glyphs receive the cell's real background color
via `push_glyph_with_bg()`, activating the shader's known-bg per-channel LCD
compositing path. Weight 400 text on Windows renders at the correct visual weight.
Both the full-rebuild and incremental render paths pass bg hints consistently.

**Context:** Currently all terminal grid glyphs — mono, subpixel, and color —
are emitted via `push_glyph()` which writes `bg_color = [0,0,0,0]`. The subpixel
shader's `bg.a > 0.001` check always fails, forcing the `max(r,g,b)` grayscale
fallback that overestimates coverage and makes text appear too bold.

The cell's background color is already computed in `fill_frame_shaped()` via
`resolve_cell_colors()` — it just isn't propagated to `GlyphEmitter::emit()`.
The fix is to thread the bg color through the emitter and use
`push_glyph_with_bg()` for subpixel atlas entries.

**Note:** `push_glyph_with_bg()` is already used in the **UI text** rendering
path (`oriterm/src/gpu/draw_list_convert/text.rs:129`) where the `bg_hint`
is provided by the UI widget layer. This section only modifies the **terminal
grid** emission path (`emit.rs` / `fill_frame_shaped`). The UI path is
unaffected.

**Depends on:** Section 01 (shader must have zero-coverage guard and fg.a
handling before bg hints are activated). Section 03 (opacity-aware disable must
be wired so transparent windows don't get subpixel with wrong bg assumptions).

---

## 02.1 GlyphEmitter bg Plumbing

**File(s):** `oriterm/src/gpu/prepare/emit.rs`, `oriterm/src/gpu/prepare/mod.rs`

### emit.rs Changes

The `GlyphEmitter::emit()` method needs a `bg: Rgb` parameter so it can route
subpixel glyphs to `push_glyph_with_bg()`.

- [x] Add `bg: Rgb` parameter to `GlyphEmitter::emit()`:
  ```rust
  pub fn emit(
      &mut self,
      row_glyphs: &[ShapedGlyph],
      col_starts: &[usize],
      start_idx: usize,
      col: usize,
      x: f32,
      y: f32,
      fg: Rgb,
      bg: Rgb,  // NEW
  ) {
  ```

- [x] Route `AtlasKind::Subpixel` entries to `push_glyph_with_bg()`. The
  current code at emit.rs:109–114 uses `let writer = match { ... }` with a
  shared `writer.push_glyph(...)` call. Change the subpixel arm to call
  `push_glyph_with_bg` directly and `continue` past the shared call:
  ```rust
  let writer = match entry.kind {
      AtlasKind::Color => &mut self.frame.color_glyphs,
      AtlasKind::Subpixel => {
          self.frame.subpixel_glyphs.push_glyph_with_bg(
              rect, uv, fg, bg, self.fg_dim, entry.page,
          );
          continue;  // Skip the shared push_glyph below.
      }
      AtlasKind::Mono => &mut self.frame.glyphs,
  };
  writer.push_glyph(rect, uv, fg, self.fg_dim, entry.page);
  ```

- [x] Update the comment block at emit.rs:98–108. Remove the obsolete
  "Subpixel glyphs always use the no-bg-hint path" comment. Replace with
  documentation explaining that subpixel glyphs now receive the cell bg for
  proper per-channel LCD compositing, and that the shader's zero-coverage guard
  prevents cross-cell bleeding.

### mod.rs Changes

The call site in `fill_frame_shaped()` must pass `bg` to `GlyphEmitter::emit()`.

- [x] Update the `GlyphEmitter::emit()` call in `fill_frame_shaped()` to pass `bg`:
  ```rust
  GlyphEmitter {
      baseline,
      size_q6: shaped.size_q6(),
      hinted: shaped.hinted(),
      fg_dim,
      atlas,
      frame,
  }
  .emit(row_glyphs, row_col_starts, start_idx, col, x, y, fg, bg);
  ```

- [x] Verify that `bg` is available at the call site. It is: `let (fg, bg) =
  resolve_cell_colors(...)` is called at prepare/mod.rs:357, and `bg` is
  already in scope at the `.emit()` call at prepare/mod.rs:439.

---

## 02.2 Incremental Path Support

> **Complexity warning:** The incremental path copies raw instance bytes from
> the previous frame for clean rows. These bytes now encode `bg_color` —
> if a theme change or selection change alters a cell's background without
> marking the row dirty, the cached `bg_color` in the subpixel glyph
> instances will be stale. Verify that `content.all_dirty` is set on theme
> changes and that selection damage tracking correctly marks affected rows.

**File(s):** `oriterm/src/gpu/prepare/dirty_skip/mod.rs`

The incremental render path (`fill_frame_incremental`) reuses cached row
instances for clean rows and only regenerates dirty rows. Dirty rows go through
the same `GlyphEmitter` codepath, so they automatically get the bg hint from
02.1. But we need to verify:

- [x] Confirm that `fill_frame_incremental` calls the same `GlyphEmitter::emit()`
  path as `fill_frame_shaped()` for dirty rows (dirty_skip/mod.rs:446–454).
  It does — the bg parameter added in 02.1 must also be passed here. The
  `bg` variable is in scope from `resolve_cell_colors()` at
  dirty_skip/mod.rs:377.

- [x] Verify cached (clean) rows: instances saved from the previous frame already
  have the correct bg_color baked in from the previous `push_glyph_with_bg()`
  call. When the row is clean (content unchanged), the cached instances are valid.

- [x] Check row invalidation: when a cell's background color changes (e.g., user
  changes theme), the row must be marked dirty. Verify that the damage tracking
  system correctly marks rows dirty when bg colors change. The existing
  `content.all_dirty` flag handles theme changes (full rebuild), but verify
  per-row damage also catches per-cell bg changes from terminal output (SGR
  background color changes).

- [x] Verify selection/search color interaction: when a cell is selected or
  search-matched, `resolve_cell_colors()` returns the selection/search bg
  color (not the cell's original bg). This selection bg must be the `bg` passed
  to `push_glyph_with_bg()` so the shader composites against the correct
  visible background. Confirm that `resolve_cell_colors()` returns the
  final visible `(fg, bg)` pair including selection/search overrides.

---

## 02.2b Other push_glyph Call Sites

Several other code paths call `push_glyph()` directly (not through `GlyphEmitter`).
These must be verified to ensure they either don't need bg hints or are updated.

**Built-in geometric glyphs** (`prepare/mod.rs:418`, `dirty_skip/mod.rs:435`):
- [x] Verify built-in glyphs (box-drawing, block elements) are always rendered
  as `AtlasKind::Mono`. They use `push_glyph()` with the mono writer, which
  is correct — mono glyphs use the `fg.wgsl` shader that ignores `bg_color`.
  No change needed.

**Unshaped path** (`prepare/unshaped.rs:156`):
- [x] The unshaped path is test-only (`prepare_frame()` / `fill_frame()`) and
  always routes through `frame.glyphs.push_glyph()` (the mono writer). It does
  not distinguish `AtlasKind` — all glyphs go to the mono writer. This is
  acceptable for test-only code, but document that this path does not support
  subpixel rendering. Add a comment at `unshaped.rs:143–156` noting this
  limitation.
- [x] If the unshaped path's `AtlasLookup` returns subpixel entries, they would
  be routed to the mono writer incorrectly. Verify that the test atlas always
  returns `AtlasKind::Mono` entries, or add a kind check.

**UI text draw_list_convert** (`draw_list_convert/text.rs:124-138`):
- [x] Verify the UI text path is unaffected. It already correctly handles
  subpixel with/without `bg_hint` via `subpixel_bg: Option<Rgb>`. No changes
  needed. Confirm no regressions after Section 01 shader changes.

---

### Cleanup

- [x] **[BLOAT]** `prepare/dirty_skip/mod.rs` — Stayed at 495 lines. The `bg` plumbing only appended `, bg` to an existing call — no new lines needed. No extraction required.
- [x] **[BLOAT]** `prepare/mod.rs` — Stayed at 485 lines. Same: only appended `, bg` to existing `.emit()` call. No extraction required.

---

## 02.3 Completion Checklist

- [x] `GlyphEmitter::emit()` accepts `bg: Rgb` parameter
- [x] `AtlasKind::Subpixel` entries use `push_glyph_with_bg(fg, bg, fg_dim, ...)`
- [x] `AtlasKind::Mono` and `AtlasKind::Color` entries unchanged (use `push_glyph`)
- [x] `fill_frame_shaped()` passes resolved `bg` to emitter
- [x] Incremental path (`fill_frame_incremental`) passes `bg` for dirty rows
- [x] Cached clean rows retain correct bg_color from previous frame
- [x] Obsolete "no-bg-hint" comment removed, replaced with accurate docs
- [x] Selection/search bg colors are correctly passed as bg hints
- [x] Built-in geometric glyphs verified as Mono-only (no bg hint needed)
- [x] Unshaped test path documented as mono-only (no subpixel support)
- [x] UI text path (`draw_list_convert/text.rs`) verified unaffected
- [x] Weight 400 text on Windows renders at correct visual weight (manual test)
- [x] `./clippy-all.sh` green
- [x] `./build-all.sh` green
- [x] `./test-all.sh` green

**Exit Criteria:** Running ori_term on Windows at 1x DPI with weight 400 font
produces text that visually matches the font's intended regular weight, not a
bold/semibold appearance. Both full-rebuild and incremental render paths produce
identical output. No visual artifacts at cell boundaries (no rectangles, no
halos, no color fringing on opaque backgrounds).
