---
section: "15"
title: "Cell-Level Alpha + Transparency"
status: not-started
reviewed: false
goal: "Add cell-level alpha modeling to the Cell struct, propagate it through RenderableContent → FrameInput → GPU pipeline, and verify translucent overlays render correctly — architectural prerequisite for the notcurses `trans` scene in section 24."
success_criteria:
  - "`oriterm_core/src/cell/mod.rs` `Cell` struct has an alpha field (or `CellFlags::ALPHA` flag with associated alpha value); currently MISSING per Pass 1"
  - "`RenderableCell` propagates the alpha to the snapshot"
  - "`FrameInput` consumer (GPU pipeline) blends the cell with the underlying surface using the alpha — `oriterm/src/gpu/pipeline/image.rs` (or text pipeline if cells go through text) is updated"
  - "Spec_chain golden tests verify translucent cells composite correctly: a cell with alpha=128 renders as 50% blend of its color and the background"
  - "Multi-plane composition test: place an image at row 5 with alpha=64, place another image at row 5 with alpha=192, verify the final pixels match the expected blend (per the notcurses NCALPHA_BLEND semantics)"
  - "All existing teseq + visual_regression tests pass without modification"
  - "Alloc regression unchanged (alpha field is ~1 byte; no per-cell allocation)"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row** AND unblocks the notcurses `trans` scene for section 24"
inspired_by:
  - "notcurses `NCALPHA_OPAQUE`/`NCALPHA_TRANSPARENT`/`NCALPHA_BLEND` semantics — defines the per-cell alpha contract"
  - "wezterm cell color resolution — per-cell alpha is implicit via the color resolution path"
depends_on: ["12", "13", "14"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "15.1"
    title: "Add alpha field/flag to Cell struct"
    status: not-started
  - id: "15.2"
    title: "Propagate alpha through RenderableContent + FrameInput"
    status: not-started
  - id: "15.3"
    title: "Update GPU pipeline to blend with cell alpha"
    status: not-started
  - id: "15.4"
    title: "Spec_chain golden tests for translucent cell rendering"
    status: not-started
  - id: "15.5"
    title: "Multi-plane composition test (notcurses trans scene preview)"
    status: not-started
  - id: "15.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "15.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 15: Cell-Level Alpha + Transparency

**Status:** Not Started
**Goal:** Add cell-level alpha to the Cell struct and verify translucent rendering. This is the architectural prerequisite for notcurses `trans` scene in section 24, which uses 6 planes with different alpha-blend modes — without cell-level alpha, the scene's plane composition can't be modeled.

**Success Criteria:** see frontmatter.

**Context:** Pass 1 confirmed `oriterm_core/src/cell/mod.rs` has no alpha field. Image quads support opacity (via `oriterm/src/gpu/pipeline/image.rs` blend state, which Pass 1 confirmed is premultiplied alpha), but cells don't. The notcurses `trans` scene stacks 6 planes with different `NCALPHA_*` modes; rendering it correctly requires per-cell alpha.

**Reference implementations:** see frontmatter.

**Depends on:** Sections 12, 13, 14 (image stacks landed; this section is the architectural change that lets section 24 use the image stacks under translucent overlay scenes).

---

## 15.1 Add alpha field/flag to Cell struct

**File(s):** `oriterm_core/src/cell/mod.rs`, sibling tests

- [ ] Add `alpha: u8` field to `Cell` (default 255 = fully opaque). Or add a `CellFlags::TRANSLUCENT` flag with a separate `alpha_value` storage if size constraints require.
- [ ] If size matters: check the existing Cell struct size and the `size_assertions` in `oriterm_core/src/cell/mod.rs`. If adding 1 byte pushes the cell past a cache-line boundary, document the trade-off and either accept it or use the flag+sidecar approach.
- [ ] Update Cell construction sites to default alpha to 255.
- [ ] Sibling test: cell created without alpha defaults to 255; cell created with alpha preserves the value.
- [ ] **Validation**: `cargo test -p oriterm_core --lib cell::tests` passes; existing cell tests still pass.

---

## 15.2 Propagate alpha through RenderableContent + FrameInput

**File(s):** `oriterm_core/src/term/snapshot.rs`, `oriterm_core/src/term/renderable/mod.rs`, `oriterm/src/gpu/frame_input/mod.rs`

- [ ] Add `alpha: u8` to `RenderableCell` in `oriterm_core/src/term/renderable/mod.rs`.
- [ ] In `oriterm_core/src/term/snapshot.rs::renderable_content_into()`, copy the cell alpha into the renderable cell.
- [ ] If `FrameInput` needs to expose the per-cell alpha to the GPU pipeline (via instance buffer fields), add the field to the relevant instance buffer struct.
- [ ] Sibling tests verify alpha is preserved through the snapshot extraction.

---

## 15.3 Update GPU pipeline to blend with cell alpha

**File(s):** `oriterm/src/gpu/pipeline/text.rs` (or wherever the text pipeline lives), `oriterm/src/gpu/pipeline/image.rs`

- [ ] Update the text pipeline to read the per-cell alpha from the instance buffer and use it in the blend.
- [ ] Verify the text pipeline's BlendState supports the blend (likely premultiplied alpha already).
- [ ] If the cells need to blend against an underlying image quad (multi-plane composition), the rendering order matters: image quads first, then text on top with cell alpha controlling the blend.
- [ ] Sibling tests in the visual_regression test suite (or section 04 spec_chain harness): render a cell with alpha=128 over a known background, verify the final pixel is the 50% blend.

---

## 15.4 Spec_chain golden tests for translucent cell rendering

**File(s):** `oriterm_core/tests/spec_chain/cell_alpha/translucent.rs` (new), goldens

- [ ] Spec_chain test: write a single character at alpha=128 over a known background color, capture golden, verify pixel matches expected blend
- [ ] Test alpha=0 (fully transparent — character invisible)
- [ ] Test alpha=255 (fully opaque — same as no alpha)
- [ ] Update catalog rows in `catalog/de-facto-behaviors.md` (since cell-level alpha is a notcurses-driven de facto, no spec) to `verified` with the de-facto reference cited.
- [ ] **Validation**: golden tests pass.

---

## 15.5 Multi-plane composition test (notcurses trans scene preview)

**File(s):** `oriterm_core/tests/spec_chain/cell_alpha/multi_plane.rs` (new)

This test is the preview of the notcurses `trans` scene's composition. Place multiple "planes" (overlapping image placements with different alpha values) and verify the final composition matches the expected blend.

- [ ] Spec_chain test: place an opaque kitty image at row 5; on top of it, place a translucent text overlay; verify the final golden matches the expected NCALPHA_BLEND result.
- [ ] Test multiple translucent layers: 6 planes with NCALPHA_BLEND, NCALPHA_HIGHCONTRAST, NCALPHA_TRANSPARENT modes — verify the final pixel matches notcurses' reference output.
- [ ] **Validation**: multi-plane composition test passes; this unblocks section 24's `trans` scene verification.

---

## 15.R Third Party Review Findings

- None.

---

## 15.N Completion Checklist

- [ ] Failing test matrix written FIRST
- [ ] **Matrix dimensions**: alpha value (0/64/128/192/255) × cell content type (text/image/empty) × layer depth (1/2/6 planes)
- [ ] **Semantic pin**: multi-plane composition test (the notcurses trans scene preview)
- [ ] `Cell` has alpha field/flag
- [ ] Alpha propagates through RenderableContent + FrameInput
- [ ] GPU pipeline blends correctly
- [ ] Spec_chain golden tests pass for translucent cells
- [ ] Multi-plane composition test passes
- [ ] All existing teseq + visual_regression tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` + `index.md` updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Cell-level alpha modeled and rendered; translucent overlays composite correctly; notcurses `trans` scene unblocked for section 24.
