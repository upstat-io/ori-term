---
section: "04"
title: "Variable Font Metrics"
status: complete
reviewed: true
goal: "Fix glyph_metrics and metrics calls to pass variable font axis settings"
depends_on: []
sections:
  - id: "04.1"
    title: "Fix Advance Width Computation"
    status: complete
  - id: "04.2"
    title: "Fix Cell Metrics Computation"
    status: complete
  - id: "04.3"
    title: "Completion Checklist"
    status: complete
---

# Section 04: Variable Font Metrics

**Status:** Not Started
**Goal:** Both `glyph_metrics()` and `metrics()` in `face.rs` receive the
active variable font axis settings (e.g., `wght=400`) so that advance widths
and cell dimensions are correct for non-default weights on variable fonts.

**Context:** Found during the font weight investigation. Multiple call sites
pass empty variations `&[]` to swash metric functions:

In `oriterm/src/font/collection/face.rs`:
- Line 173: `fr.glyph_metrics(&[]).scale(size_px).advance_width(glyph_id)` —
  in `rasterize_from_face()`, advance width from default weight.
- Line 281: `fr.metrics(&[]).scale(size_px)` — in `compute_metrics()`, cell
  height, ascent, descent, underline offset, and strikeout metrics from
  default weight.
- Line 286: `fr.glyph_metrics(&[]).scale(size_px)` — in `compute_metrics()`,
  cell width from default weight.

In `oriterm/src/font/collection/mod.rs`:
- Line 266: `fr.glyph_metrics(&[]).scale(size).advance_width(gid)` — in
  `cmap_glyph()`, advance width for glyph lookup.

In `oriterm/src/font/collection/colr_v1/rasterize.rs`:
- Line 140: `fr.glyph_metrics(&[]).scale(size_px).advance_width(glyph_id)` —
  COLR v1 rasterization advance.
- Lines 159–161: `fr.metrics(&[])` and `fr.glyph_metrics(&[])` — COLR v1
  clip box estimation.

For fonts where the default weight matches the configured weight (e.g., a
static font or a variable font with `wght` default = 400), this is harmless.
But for variable fonts with a non-400 default weight, or when the user
configures a non-default weight (e.g., 300 Light, 500 Medium), metrics will
be wrong — potentially causing misaligned text, incorrect cell sizing, or
glyph clipping.

This is independent of the subpixel rendering fix and can be done in parallel.

**Reference implementations:**
- **swash** API: `FontRef::glyph_metrics(variations)` and
  `FontRef::metrics(variations)` both accept a `&[(&str, f32)]` slice of
  variation axis settings. The current code passes `&[]` (empty).

**Depends on:** None (independent of Sections 01-03).

---

## 04.1 Fix Advance Width Computation

**File(s):** `oriterm/src/font/collection/face.rs`

The `rasterize_from_face()` function already receives `variations` as a
parameter (line 164) but doesn't pass it to `glyph_metrics()` (line 173).

- [x] Change `glyph_metrics(&[])` to `glyph_metrics(variations)` at face.rs:173:
  ```rust
  let advance = fr.glyph_metrics(variations).scale(size_px).advance_width(glyph_id);
  ```

- [x] Verify the `variations` parameter type matches what `glyph_metrics()`
  expects. The `rasterize_from_face()` parameter is `&[(&str, f32)]` and swash's
  `glyph_metrics()` accepts the same type.

> **Complexity warning:** `cmap_glyph()` is called from the shaping fallback
> path. Threading variations requires computing `face_variations()` inside
> this method, which needs `self.weight` and the face's axes. The `self`
> borrow is already available, so this is feasible but verify borrow checker
> interactions with the face data access pattern.

- [x] Fix `cmap_glyph()` in `collection/mod.rs:266`: this function doesn't
  currently receive variations. Either thread variations through (from the
  `FontCollection`'s stored settings) or accept the default-weight advance
  as intentional for glyph lookup purposes. Document the decision.

- [x] Fix COLR v1 call sites in `colr_v1/rasterize.rs:140` and `:159–161`:
  the COLR rasterization path should receive and pass variations for correct
  advance width and clip box estimation on variable fonts.

---

## 04.2 Fix Cell Metrics Computation

**File(s):** `oriterm/src/font/collection/face.rs`

The `compute_metrics()` function does not receive variations at all. It needs
to accept them and pass through to `metrics()` and `glyph_metrics()`.

- [x] Add a `variations: &[(&str, f32)]` parameter to `compute_metrics()`:
  ```rust
  pub(super) fn compute_metrics(
      bytes: &[u8],
      face_index: u32,
      size_px: f32,
      variations: &[(&str, f32)],
  ) -> Option<FontMetrics> {
      let fr = FontRef::from_index(bytes, face_index as usize)?;
      let metrics = fr.metrics(variations).scale(size_px);
      // ...
      let cell_width = fr
          .glyph_metrics(variations)
          .scale(size_px)
          .advance_width(gid)
          .ceil();
      // ...
  }
  ```

- [x] Update all callers of `compute_metrics()` to pass the appropriate
  variations. There are 4 call sites (excluding tests):
  1. `collection/mod.rs:120` — `FontCollection::new()`, primary Regular face.
     Has access to `weight` and `font_set.regular` axes. Compute variations
     via `face_variations(FaceIdx::REGULAR, SyntheticFlags::NONE, weight, &axes)`.
  2. `collection/mod.rs:144` — `FontCollection::new()`, fallback faces.
     Pass `&[]` — `face_variations()` returns empty for fallback faces by design.
  3. `collection/mod.rs:338` — `set_size()`, primary Regular face.
     Same as caller 1: needs variations from stored weight and axes.
     `self.weight` is already stored at `collection/mod.rs:84` (confirmed).
     Primary face axes accessible via `self.primary[0].as_ref().unwrap().axes`.
     Use `face_variations(FaceIdx::REGULAR, SyntheticFlags::NONE, self.weight, &axes)`
     to compute variations, then pass `.settings.as_slice()` to `compute_metrics()`.
  4. `collection/mod.rs:344` — `set_size()`, fallback faces.
     Pass `&[]` — same as caller 2.

- [x] For fallback fonts, `compute_metrics` is called for cap-height
  normalization. Fallback fonts currently get empty variations (by design —
  `face_variations()` returns empty for fallback faces). Verify this is correct
  and pass `&[]` for fallback metric computation.

- [x] Update existing tests in `collection/tests.rs` that call
  `compute_metrics()` with 3 arguments (lines 386, 397, 398, 409, 421).
  All 5 test calls must be updated to pass `&[]` as the 4th argument
  (the test font is not a variable font, so empty variations is correct).

### Cleanup

- [x] **[STYLE]** `collection/tests.rs:382,393` -- Remove decorative banners (`// ── cap_height ──`, `// ── compute_metrics ──`). Replace with plain `// Section name` comments per code-hygiene.md. Fix at least the banners near the `compute_metrics` tests this section modifies.

---

## 04.3 Completion Checklist

- [x] `rasterize_from_face()` passes `variations` to `glyph_metrics()`
- [x] `compute_metrics()` accepts and passes `variations` to both `metrics()`
  and `glyph_metrics()`
- [x] `cmap_glyph()` in `collection/mod.rs` uses variations or documents
  why default-weight advance is acceptable
- [x] COLR v1 rasterization (`colr_v1/rasterize.rs`) passes variations
- [x] All 4 non-test callers of `compute_metrics()` updated
- [x] `set_size()` at `collection/mod.rs:338` computes variations from `self.weight` and primary face axes
- [x] All 5 existing `compute_metrics` tests updated for 4-arg signature
- [x] Fallback font metrics still use `&[]` (intentional)
- [x] No behavioral change for fonts where default weight equals configured weight
- [x] `./clippy-all.sh` green
- [x] `./build-all.sh` green
- [x] `./test-all.sh` green

**Exit Criteria:** `compute_metrics()` and `rasterize_from_face()` produce
correct metrics for variable fonts at non-default weights. A variable font
configured at weight 300 (Light) produces narrower advance widths and
potentially different cell dimensions than the same font at weight 700 (Bold).
Verified by unit test comparing metrics with and without variations.
