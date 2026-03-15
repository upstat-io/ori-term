---
section: "01"
title: "Shader Compositing Fix"
status: complete
reviewed: true
goal: "Fix subpixel_fg.wgsl to handle zero-coverage pixels and fg.a dimming correctly"
inspired_by:
  - "Alacritty dual-source blending (alacritty/res/glsl3/text.f.glsl)"
  - "Ghostty embolden_strength formula"
depends_on: []
sections:
  - id: "01.1"
    title: "Zero-Coverage Guard"
    status: complete
  - id: "01.2"
    title: "fg.a Dim Factor Integration"
    status: complete
  - id: "01.3"
    title: "Rust-Side Blend Mirror Update"
    status: complete
  - id: "01.4"
    title: "Completion Checklist"
    status: complete
---

# Section 01: Shader Compositing Fix

**Status:** Not Started
**Goal:** The subpixel fragment shader correctly handles three cases: (a) known
background with nonzero coverage produces per-channel LCD compositing with fg.a
dimming applied, (b) known background with zero coverage outputs transparent
pass-through, (c) unknown background falls back to grayscale coverage. No
regressions in existing pipeline tests.

**Context:** The `subpixel_fg.wgsl` shader performs per-channel
`mix(bg, fg, mask)` compositing unconditionally (lines 81–83), then branches
on `bg.a > 0.001` (line 89) to decide what to return. The known-bg path has
two bugs:
1. When all mask channels are zero, the mix produces `(bg.r, bg.g, bg.b)` and
   the branch returns `vec4(bg.r, bg.g, bg.b, 1.0)`, which with premultiplied
   alpha blend (`src*1 + dst*(1-1.0)`) completely replaces the framebuffer
   pixel. This bakes the source cell's bg color into pixels that may belong to
   a neighboring cell (due to glyph bearing offsets extending beyond cell
   boundaries), creating visible rectangles.
2. It ignores `fg.a` (the dim factor from `fg_dim`). Cells in dimmed panes
   would render at full brightness instead of being dimmed.

Both bugs are latent because the known-bg path is currently never reached
(all terminal glyphs pass bg.a=0, hitting the unknown-bg fallback). They must
be fixed before Section 02 activates the known-bg path.

**Reference implementations:**
- **Alacritty** `alacritty/res/glsl3/text.f.glsl` (lines 67–72): Uses
  dual-source blending (GL extension `GL_EXT_blend_func_extended` on GLES2,
  `layout(location=0, index=1)` on GLSL3) for true per-channel alpha. For
  regular text glyphs: `FRAG_COLOR = vec4(fg.rgb, 1.0)` and
  `ALPHA_MASK = vec4(textColor, textColor.r)` where `textColor` is sampled
  from the mask texture. The blend equation achieves
  `result.rgb = fg.rgb * mask.rgb + dst.rgb * (1 - mask.rgb)` per-channel.
  We cannot use this approach because wgpu/WebGPU does not support dual-source
  blending. Instead we composite inside the shader and output the final result.

**Depends on:** None.

---

## 01.1 Zero-Coverage Guard

**File(s):** `oriterm/src/gpu/shaders/subpixel_fg.wgsl`

The known-bg path must discard (output transparent) when the mask has zero
coverage across all channels. This prevents the bg color from painting over
neighboring cells' content when glyph quads extend beyond cell boundaries.

- [x] Add zero-coverage check to the known-bg path in `fs_main()`. The
  final shader structure (after combining with 01.2's dim fix) will have
  the guard inside the known-bg branch — see 01.2 for the complete code.
  Conceptually, the guard returns transparent when all mask channels are
  zero:
  ```wgsl
  if bg.a > 0.001 {
      // ... mix() calls ...
      let coverage = max(mask.r, max(mask.g, mask.b));
      if coverage < 0.001 {
          return vec4(0.0, 0.0, 0.0, 0.0);  // transparent pass-through
      }
      return vec4(r, g, b, 1.0);
  }
  ```

- [x] Verify that the 0.001 epsilon is appropriate for GPU float precision at
  glyph edges. swash rasterization produces exact 0.0 for pixels fully outside
  the glyph outline; the epsilon guards against floating-point sampling
  artifacts at the boundary.

- [x] Verify that `PREMUL_ALPHA_BLEND` (`src*1 + dst*(1-src_alpha)`) at
  `pipeline/mod.rs:105` works correctly for both shader output paths:
  - **Known-bg path** outputs `vec4(r, g, b, 1.0)` — since `src_alpha=1.0`,
    `dst*(1-1.0)=0`, so dst is completely replaced. This is correct: the
    shader has already composited fg over bg, so the result is final.
  - **Known-bg zero-coverage** outputs `vec4(0, 0, 0, 0)` — since
    `src_alpha=0.0`, `src*1=0` and `dst*(1-0)=dst`, so the framebuffer pixel
    is preserved. Correct transparent pass-through.
  - **Unknown-bg path** outputs premultiplied `vec4(fg.rgb*a, a)` — standard
    premultiplied alpha blending over whatever is in the framebuffer. Correct.
  No changes to `BlendState` are needed; document this invariant in the shader
  comments.

---

## 01.2 fg.a Dim Factor Integration

**File(s):** `oriterm/src/gpu/shaders/subpixel_fg.wgsl`

The dim factor is encoded in `fg_color.a` by `push_glyph` / `push_glyph_with_bg`
via `rgb_to_floats(fg, alpha)` where `alpha` is `fg_dim` (typically 1.0 for
normal panes, ~0.6 for dimmed unfocused panes). The known-bg path must apply
this to the compositing.

The correct approach: scale the effective coverage by `fg.a` before mixing.
When `fg.a < 1.0` (dimmed), the text appears lighter because coverage is
reduced, which is equivalent to the text being more transparent.

- [x] Apply `fg.a` as a coverage multiplier by modifying the unconditional
  mix calls (lines 81–83) to factor in dim when bg is known. The cleanest
  restructure: move the mix into the known-bg branch with dim applied, and
  keep the existing unconditional mix for the unknown-bg fallback:
  ```wgsl
  if bg.a > 0.001 {
      let dim = fg.a;
      let r = mix(bg.r, fg.r, mask.r * dim);
      let g = mix(bg.g, fg.g, mask.g * dim);
      let b = mix(bg.b, fg.b, mask.b * dim);
      let coverage = max(mask.r, max(mask.g, mask.b));
      if coverage < 0.001 {
          return vec4(0.0, 0.0, 0.0, 0.0);
      }
      return vec4(r, g, b, 1.0);
  }

  // Unknown background — existing logic (unchanged).
  let r = mix(bg.r, fg.r, mask.r);
  let g = mix(bg.g, fg.g, mask.g);
  let b = mix(bg.b, fg.b, mask.b);
  let coverage = max(mask.r, max(mask.g, mask.b));
  let a = coverage * fg.a;
  return vec4(fg.rgb * a, a);
  ```
  Note: this restructures the shader so the known-bg and unknown-bg paths
  each have their own `mix()` calls. The unknown-bg path's `mix()` results
  (`r`, `g`, `b`) are actually unused — the fallback outputs `fg.rgb * a`
  directly. The dead `mix()` calls should be removed for clarity:
  ```wgsl
  if bg.a > 0.001 {
      let dim = fg.a;
      let r = mix(bg.r, fg.r, mask.r * dim);
      let g = mix(bg.g, fg.g, mask.g * dim);
      let b = mix(bg.b, fg.b, mask.b * dim);
      let coverage = max(mask.r, max(mask.g, mask.b));
      if coverage < 0.001 {
          return vec4(0.0, 0.0, 0.0, 0.0);
      }
      return vec4(r, g, b, 1.0);
  }

  // Unknown background — grayscale alpha fallback.
  let coverage = max(mask.r, max(mask.g, mask.b));
  let a = coverage * fg.a;
  return vec4(fg.rgb * a, a);
  ```

- [x] Verify the unknown-bg fallback already handles `fg.a` correctly. Current
  code: `let a = coverage * fg.a; return vec4(fg.rgb * a, a);`. This correctly
  applies the dim factor as premultiplied alpha. No change needed.

- [x] Verify edge case: `fg.a = 0.0` (fully dimmed). In the known-bg path,
  `mask * dim = 0` for all channels, so `mix(bg, fg, 0) = bg` for all channels,
  and coverage = `max(mask) * dim = max(1,1,1) * 0 = 0 < 0.001`, so the
  zero-coverage guard returns transparent. Correct: fully dimmed text is
  invisible. Documented in the shader comment.

---

## 01.3 Rust-Side Blend Mirror Update

**File(s):** `oriterm/src/gpu/pipeline_tests.rs`

The `subpixel_blend()` function is a Rust-side mirror of the WGSL shader logic,
used by unit tests to verify the blend formula. It must be updated to match the
shader changes.

### Cleanup

- [x] **[STYLE]** `pipeline_tests.rs:12,146,216,389` -- Remove decorative banners (`// ── ... ──`). Replace with plain `// Section name` comments per code-hygiene.md.

- [x] Update `subpixel_blend()` to match the restructured shader. The current
  function (pipeline_tests.rs:18) does the mix unconditionally before
  branching. The updated version separates the paths:
  ```rust
  fn subpixel_blend(fg: [f32; 4], bg: [f32; 4], mask: [f32; 3]) -> [f32; 4] {
      fn mix(a: f32, b: f32, t: f32) -> f32 {
          a * (1.0 - t) + b * t
      }
      if bg[3] > 0.001 {
          let dim = fg[3];
          let r = mix(bg[0], fg[0], mask[0] * dim);
          let g = mix(bg[1], fg[1], mask[1] * dim);
          let b = mix(bg[2], fg[2], mask[2] * dim);
          let coverage = mask[0].max(mask[1]).max(mask[2]);
          if coverage < 0.001 {
              return [0.0, 0.0, 0.0, 0.0];
          }
          return [r, g, b, 1.0];
      }
      let coverage = mask[0].max(mask[1]).max(mask[2]);
      let a = coverage * fg[3];
      [fg[0] * a, fg[1] * a, fg[2] * a, a]
  }
  ```

- [x] Update `subpixel_blend_zero_mask_returns_bg` test (pipeline_tests.rs:46):
  zero mask with known bg should now return `[0, 0, 0, 0]` (transparent
  pass-through), not the bg color. Renamed to
  `subpixel_blend_zero_mask_known_bg_returns_transparent`.

- [x] Add new test: `subpixel_blend_known_bg_dim_reduces_coverage` — verify that
  `fg.a = 0.5` with known bg and full mask produces a result midway between bg
  and fg (dimmed text appears lighter).

- [x] Add new test: `subpixel_blend_known_bg_zero_coverage_transparent` — verify
  zero-mask with known bg returns `[0, 0, 0, 0]`.

- [x] Verify all existing pipeline tests pass with the updated formula. Tests
  that use `bg.a = 0` (unknown bg) should be unaffected by the shader changes.

- [x] Update `subpixel_blend_per_channel_independence` (pipeline_tests.rs:70):
  this test uses `bg.a = 1.0` (known bg) and expects `r=1.0, g=1.0, b=0.4,
  a=1.0`. The channel comments say "G: mask=0 → bg.g" — but with the updated
  shader, `mix(bg.g, fg.g, mask.g * dim)` where `dim=fg.a=1.0` still gives
  `mix(1.0, 0.5, 0.0) = 1.0`. Expected output is unchanged. Verified correct.

---

## 01.4 Completion Checklist

- [x] `subpixel_fg.wgsl` has zero-coverage guard in known-bg path
- [x] `subpixel_fg.wgsl` applies `fg.a` dim factor to coverage in known-bg path
- [x] `subpixel_fg.wgsl` has comments documenting BlendState interaction for all three output paths
- [x] `subpixel_blend()` in `pipeline_tests.rs` mirrors the updated shader exactly
- [x] All 6+ existing blend tests pass (including `per_channel_independence` which uses known bg)
- [x] 3 new blend tests cover zero-coverage, dim-factor, and full-dim cases
- [x] New test: `subpixel_blend_known_bg_full_dim_returns_transparent` — `fg.a=0.0` with known bg returns `[0,0,0,0]`
- [x] `./clippy-all.sh` green
- [x] `./build-all.sh` green
- [x] `./test-all.sh` green

**Exit Criteria:** The subpixel shader correctly handles known-bg compositing
with zero-coverage pass-through and dim factor, verified by Rust-side mirror
tests. The shader changes alone do not change any visible behavior because no
callers pass `bg.a > 0` yet (that comes in Section 02).
