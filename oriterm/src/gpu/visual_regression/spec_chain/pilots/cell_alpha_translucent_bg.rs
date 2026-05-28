//! §15.3 — cell-alpha compositing GPU-apex regression guard.
//!
//! Catalog row: DFCT-CELL-ALPHA (cell-level per-channel alpha, de-facto)
//! Apex: TextureRender (direct pixel-value assertion, no golden bless)
//!
//! Feeds an SGR mode-6 translucent background cell
//! (`ESC[48:6::r:g:b:128m`) over the deterministic golden-lane background,
//! drives it through the PRODUCTION cached render path
//! (`render_frame_cached()` via the visual rung chain), reads back the
//! cell-center pixel, and asserts it is a correct ~50% blend — an
//! intermediate value that is NOT too dark (the double-premultiply bug),
//! NOT fully opaque (the cell color at full strength), and NOT fully
//! transparent (the bg-only baseline).
//!
//! This guards the §15.3 `emit_cell` bg-alpha threading at the GPU apex:
//! it proves the blend MATH composites translucent cell backgrounds at the
//! SINGLE-premultiply brightness (the shaders own the one premultiply; see
//! `instance_writer::rgb_to_floats`). Final visual sign-off ("does it LOOK
//! right") is the operator's at §15 section close per the project's
//! visual-bug-verification rule.

use oriterm_test_support::spec_chain::ScenarioExpectations;

use super::super::visual_harness::VisualSpecHarness;
use crate::gpu::visual_regression::GoldenLaneConfig;

/// Sampled cell — row 3, col 4 — chosen away from the home-row cursor so
/// the cursor block never confounds the cell-center readback.
const SAMPLE_COL: usize = 4;
const SAMPLE_ROW: usize = 3;

/// Move to (row 3, col 5) via `CSI row;col H` (1-based) → 0-based (3,4),
/// set a bright-red translucent background at alpha=128 (≈0.502), emit a
/// space so the cell carries only the bg (no glyph at the sampled center),
/// reset SGR, then home the cursor so it sits at (0,0), far from the sample.
const TRANSLUCENT_BG_BYTES: &[u8] = b"\x1b[4;5H\x1b[48:6::220:30:30:128m \x1b[m\x1b[H";

/// Same placement at row 3, col 5, but a fully-OPAQUE bright-red bg —
/// the upper clamp for the blend assertion.
const OPAQUE_BG_BYTES: &[u8] = b"\x1b[4;5H\x1b[48:2::220:30:30m \x1b[m\x1b[H";

/// Move to (row 3, col 5), set an OPAQUE green foreground `(30,220,30)` over
/// an SGR mode-6 TRANSLUCENT red background `(220,30,30:128)`, emit a capital
/// `M`, reset SGR, then home the cursor away from the sample.
///
/// `M` is a dense FONT glyph (NOT a builtin block/box element — those route
/// through `push_glyph` on the mono pipeline; see `font::is_builtin` and
/// `emit_cell::emit_cell_glyphs`). As a `GlyphFormat::SubpixelRgb` glyph it
/// rasterizes into the SUBPIXEL atlas and routes through `push_glyph_with_bg`
/// (`prepare/emit.rs`: `AtlasKind::Subpixel => …push_glyph_with_bg(…)`),
/// exercising the known-bg branch of `subpixel_fg.wgsl` — the exact Porter-Duff
/// "glyph over translucent bg" path the §15.3 (F2) fix corrected. The harness
/// pins `GlyphFormat::SubpixelRgb` + `force_non_dual_subpixel` to reach this
/// branch (see the test body). The assertion samples the MAX-coverage glyph
/// pixel within the cell (a thick `M` stroke ≈ full coverage), not the
/// geometric centre, so glyph shape does not confound the readback.
const OPAQUE_GLYPH_TRANSLUCENT_BG_BYTES: &[u8] =
    b"\x1b[4;5H\x1b[38:2::30:220:30m\x1b[48:6::220:30:30:128mM\x1b[m\x1b[H";

/// Drive the current harness state through the visual rung chain and return
/// `(pixels, width, height)` from the production cached render path.
fn drive_render(harness: &mut VisualSpecHarness, catalog_row: &str) -> (Vec<u8>, u32, u32) {
    // Pixel-value assertion only — no per-rung expectations; the rungs pass
    // through and TextureRender populates last_rendered_pixels.
    let expectations = ScenarioExpectations::default();
    let _results = harness.render_visual_rungs(catalog_row, &expectations);
    let (slice, w, h) = harness
        .last_rendered_pixels()
        .expect("render_visual_rungs must populate last_rendered_pixels");
    (slice.to_vec(), w, h)
}

/// Resolve cell (col, row) center to a pixel offset into the RGBA8 buffer.
fn cell_center_offset(harness: &VisualSpecHarness, col: usize, row: usize, w: u32) -> usize {
    let cell = harness.renderer().cell_metrics();
    let px_x = (cell.width * (col as f32 + 0.5)) as usize;
    let px_y = (cell.height * (row as f32 + 0.5)) as usize;
    (px_y * w as usize + px_x) * 4
}

/// §15.3 checkbox-7 — translucent-bg cell composites to an intermediate blend.
///
/// The deterministic golden-lane background is near-black `(1,1,1)`. The cell
/// background is bright red `(220,30,30)` at alpha=128. Under the corrected
/// SINGLE-premultiply compositing (shaders own the one premultiply; see
/// `instance_writer::rgb_to_floats`), the cell-center red channel must land
/// strictly between the bg-only baseline (~near-black) and the fully-opaque
/// cell red — i.e. a partial blend. The double-premultiply bug
/// (`rgb_to_floats` premultiplying on top of the shader) produced a too-dark
/// red (≈118 instead of the correct ≈162).
#[test]
fn translucent_bg_cell_composites_to_intermediate_blend() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    // Baseline render: empty grid, bg-only. Sample the (SAMPLE_COL, SAMPLE_ROW)
    // center red — far from the home-row cursor.
    let (base_pixels, base_w, _bh) = drive_render(&mut harness, "DFCT-CELL-ALPHA-BASELINE");
    let base_off = cell_center_offset(&harness, SAMPLE_COL, SAMPLE_ROW, base_w);
    let base_red = i32::from(base_pixels[base_off]);

    // Reference render: fully-OPAQUE bright-red cell bg at the sample cell.
    // This is the upper clamp — the translucent cell must be strictly dimmer.
    harness.core_mut().feed(OPAQUE_BG_BYTES);
    let (opaque_pixels, opaque_w, _oh) = drive_render(&mut harness, "DFCT-CELL-ALPHA-OPAQUE");
    let opaque_off = cell_center_offset(&harness, SAMPLE_COL, SAMPLE_ROW, opaque_w);
    let opaque_red = i32::from(opaque_pixels[opaque_off]);

    // Reset and render the translucent (alpha=128) cell at the same place.
    harness.core_mut().feed(b"\x1b[2J\x1b[H");
    harness.core_mut().feed(TRANSLUCENT_BG_BYTES);
    let (pixels, w, _h) = drive_render(&mut harness, "DFCT-CELL-ALPHA-TRANSLUCENT");
    let off = cell_center_offset(&harness, SAMPLE_COL, SAMPLE_ROW, w);
    let blend_red = i32::from(pixels[off]);

    // The opaque reference must actually be bright red (sanity clamp).
    assert!(
        opaque_red > 200,
        "opaque cell-bg red must be bright (got {opaque_red}); test harness \
         setup is wrong if this fails"
    );

    // Lower clamp (rejection guard for the DOUBLE-premultiply bug): the
    // correct SINGLE-premultiply composite at alpha=128 over near-black is
    // ≈162 (deterministic software rasterizer; derived via
    // `/tmp/check-pilot-blend.py`: lin(220)*a + lin(1)*(1-a), a=128/255,
    // then linear→sRGB). The double-premultiply bug (`rgb_to_floats`
    // premultiplying on top of the shader) produces ≈118 (too dark). The
    // lower bound 150 sits strictly above 118 — the correct value passes,
    // the double-dark value fails.
    assert!(
        blend_red > 150,
        "§15.3: translucent (alpha=128) red bg cell-center red must be the \
         correct single-premultiply composite (≈162), strictly above the \
         double-premultiply regression value (≈118); got {blend_red}. A value \
         ≤150 means rgb_to_floats is premultiplying on top of the shader's \
         premultiply (double-dark bug). It must also stay above the near-black \
         baseline (baseline={base_red}); if at baseline the bg quad's alpha \
         collapsed to ~0 (fully transparent)"
    );
    // Upper clamp: the translucent composite (≈162) must be strictly below
    // the fully-opaque cell red (≈220) — it is a partial blend, not the cell
    // color at full strength. 180 sits between the correct value and opaque.
    assert!(
        blend_red < 180,
        "§15.3: translucent (alpha=128) red bg cell-center red must be a \
         partial blend (≈162), strictly below the fully-opaque value \
         ({opaque_red}); got {blend_red}. A value ≥180 means the alpha did not \
         attenuate the cell color"
    );
}

/// §15.3 (F2) — an OPAQUE glyph over a translucent bg must OCCLUDE.
///
/// The existing `translucent_bg_cell_composites_to_intermediate_blend` pilot
/// feeds a SPACE (coverage 0), so it never exercises the glyph path — it only
/// proves the bg quad composites. This rung is the regression guard the F2 bug
/// needed: it feeds an OPAQUE green `M` glyph over an SGR-mode-6 TRANSLUCENT red
/// bg of a different colour and proves end-to-end, through the production
/// `render_frame_cached()` path, that the opaque glyph OCCLUDES where it covers
/// — the translucent red plane beneath does NOT bleed through.
///
/// Arithmetic (deterministic software rasterizer, NON-dual subpixel path;
/// derived via `/tmp/check-nondual.py`; sRGB→linear, the bg quad composited
/// over the near-black golden lane, the glyph composited over THAT, then
/// linear→sRGB encode by the surface):
/// - fg green `(30,220,30)` opaque, bg red `(220,30,30)` at a=128/255≈0.502.
///   At a fully-inked glyph pixel (a thick `M` stroke) subpixel coverage≈1.0.
/// - Corrected Porter-Duff "over": premul out_g = fg.g_lin (bg term vanishes at
///   cov=1), out_a = 1.0 → the glyph fully occludes. Final pixel: R≈30, G≈220.
/// - Pre-fix `vec4((bg*(1-cov)+fg*cov)*bg.a, bg.a)` capped out_a at bg.a≈0.502,
///   so PREMUL_ALPHA_BLEND let the translucent-red framebuffer bleed through the
///   alpha hole → final R≈119, G≈162. Occlusion guard (green): max-coverage
///   green ≤200 means the opaque glyph was wrongly capped at bg.a (the F2 bug).
///   Bleed-through guard (red): red ≥60 at that pixel means the translucent red
///   bg bled through the opaque glyph (≈119 in the bug vs ≈30 when the glyph
///   correctly occludes).
#[test]
fn opaque_glyph_over_translucent_bg_occludes() {
    // The F2 fix lives in the NON-dual `subpixel_fg.wgsl` known-bg branch.
    // Reaching it requires BOTH (a) the glyph rasterized into the SUBPIXEL
    // atlas (GlyphFormat::SubpixelRgb) so it routes through `push_glyph_with_bg`,
    // and (b) the non-dual subpixel pipeline (`force_non_dual_subpixel`) so the
    // shader's explicit bg.a compositing runs — the dual-source path blends in
    // hardware over the framebuffer and never reaches the branch the fix
    // touched. SPEC_DEFAULT (Alpha glyphs, dual-source when available) would
    // route through the mono `fg.wgsl` and could not detect a regression here.
    let Some(mut harness) = VisualSpecHarness::with_config(GoldenLaneConfig::NON_DUAL_DEFAULT)
    else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    harness.core_mut().feed(OPAQUE_GLYPH_TRANSLUCENT_BG_BYTES);
    let (pixels, w, _h) = drive_render(&mut harness, "DFCT-CELL-ALPHA-OCCLUDE");

    // Scan the sample cell's pixel rect. Two independent occlusion signals,
    // each robust to glyph shape (a thick `M` stroke gives pixels at ≈full
    // coverage; the geometric centre lands in the glyph's interior gap):
    //   * max_green — highest green over the cell (most glyph ink).
    //   * min_red_at_high_green — the LOWEST red among near-fully-covered pixels
    //     (green > 195), i.e. the cleanest-occluding ink. Coverage attenuates
    //     red, so the most-covered pixel is where occlusion is most visible.
    let (max_green, min_red_at_high_green) = occlusion_signals(&harness, &pixels, w);

    // Occlusion guard: at fully-inked glyph pixels the opaque green occludes →
    // high green (≈220). The pre-fix bg.a-cap form dims green to ≤162 at EVERY
    // coverage level (it scales the premul colour by bg.a≈0.502 and lets the
    // framebuffer bleed through), so 195 cleanly separates correct from bug.
    assert!(
        max_green > 195,
        "§15.3 (F2): opaque green `M` over translucent red bg must OCCLUDE — \
         the max-coverage glyph pixel's green must be high (≈220); got \
         {max_green}. A value ≤195 means out_a was capped at bg.a (the pre-fix \
         `vec4(r*bg.a,…,bg.a)` bug) and the translucent-red framebuffer bled \
         through the alpha hole, dimming the green to ≈162"
    );
    // Bleed-through guard: the translucent red bg must NOT bleed through the
    // opaque glyph where it fully covers → low red (≈30) at the fullest ink.
    // The buggy form leaves red ≈119 even at cov=1 (the framebuffer's red shows
    // through the alpha hole); 60 sits strictly between correct (≈30) and bug
    // (≈119). Among green>195 pixels, the bug's red never drops below ≈119.
    assert!(
        min_red_at_high_green < 60,
        "§15.3 (F2): opaque green `M` must occlude the translucent red bg — the \
         lowest red among fully-covered glyph pixels must be low (≈30); got \
         {min_red_at_high_green}. A high red (≈119) means the translucent red bg \
         bled through the opaque glyph (the occlusion bug)"
    );
}

/// §15.3 (F2, double-bg-apply) — at a glyph EDGE (partial coverage) over a
/// translucent bg, the bg must be applied EXACTLY ONCE.
///
/// The other pilots sample full-coverage ink or the bg-only cell center; neither
/// exercises partial-coverage glyph EDGES, where the double-bg-apply bug lives.
/// The cell bg is drawn first as a separate bg quad; the buggy Porter-Duff
/// known-bg branch (`fg*cov + bg*bg.a*(1-cov)`, out_a `cov + bg.a*(1-cov)`)
/// re-composites that same bg a SECOND time at glyph edges. The corrected
/// glyph-only translucent path (`out_rgb = fg*cov`, out_a `coverage`) applies
/// the bg once (from the quad already in the framebuffer).
///
/// Arithmetic (deterministic software rasterizer, NON-dual subpixel path;
/// derived via `/tmp/check-edge-range.py`; green fg `(30,220,30)` opaque over
/// SGR-mode-6 translucent red bg `(220,30,30:128)` over the near-black golden
/// lane; sRGB→linear, bg quad over dest, glyph over THAT, linear→sRGB encode):
/// - At a partial-coverage EDGE pixel the RED channel discriminates. For glyph
///   green in `[100, 180]` (coverage ≈ 0.2–0.6) the CORRECT glyph-only red stays
///   ≤ ~147, while the double-applying form lifts red to ≥ ~155 (the translucent
///   red bg is composited twice). 160 separates correct (≤147) from bug (≥155).
/// - Green is NOT a discriminant here (the red bg has little green): both forms
///   give nearly identical green at every coverage. Red is the load-bearing
///   channel — the double-apply re-adds the red bg.
#[test]
fn glyph_edge_over_translucent_bg_applies_bg_once() {
    // Same seam as the occlusion pilot: SubpixelRgb glyph + force_non_dual so
    // the shader's explicit bg.a compositing runs (the dual-source path blends
    // per-channel in hardware and never reaches the corrected branch).
    let Some(mut harness) = VisualSpecHarness::with_config(GoldenLaneConfig::NON_DUAL_DEFAULT)
    else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    harness.core_mut().feed(OPAQUE_GLYPH_TRANSLUCENT_BG_BYTES);
    let (pixels, w, _h) = drive_render(&mut harness, "DFCT-CELL-ALPHA-EDGE-BG-ONCE");

    // Among partial-coverage edge pixels (glyph green in [100,180], excluding the
    // bg-only exterior ≈19 and the fully-covered interior ≈220), the MAX red is
    // the worst-case double-apply signal: the buggy form re-composites the red bg
    // and lifts edge red to ≥155.
    let (max_red_at_edge, edge_samples) = edge_red_signal(&harness, &pixels, w);

    // Guard against a vacuous pass: the scan MUST actually sample partial-coverage
    // glyph-edge pixels. Without this, `max_red_at_edge` stays -1 when no pixel
    // matches and the `< 160` check below would pass without testing anything (a
    // vacuous test that can never fail).
    assert!(
        edge_samples > 0,
        "§15.3 (F2): the glyph-edge scan sampled NO partial-coverage pixels \
         (green in [100,180]); the `< 160` check would pass vacuously. The glyph \
         must produce AA edges over the translucent bg for this guard to be \
         load-bearing"
    );

    assert!(
        max_red_at_edge < 160,
        "§15.3 (F2): glyph EDGE over translucent red bg must apply the bg ONCE — \
         the max red among partial-coverage edge pixels must be ≤ ~147 \
         (glyph-only over the bg quad already in the framebuffer); got \
         {max_red_at_edge}. A value ≥160 means the known-bg branch reverted to the \
         double-applying Porter-Duff form (`fg*cov + bg*bg.a*(1-cov)`), \
         re-compositing the translucent red bg a second time at glyph edges"
    );
}

/// Scan the sample cell's pixel rect and return `(max_green,
/// min_red_among_high_green)`: the highest green channel over the cell, and the
/// lowest red among pixels whose green exceeds 195 (the cleanest-occluding,
/// near-fully-covered glyph ink). Locates fully-covered glyph pixels without
/// depending on glyph shape.
fn occlusion_signals(harness: &VisualSpecHarness, pixels: &[u8], w: u32) -> (i32, i32) {
    let cell = harness.renderer().cell_metrics();
    let x0 = (cell.width * SAMPLE_COL as f32) as usize;
    let y0 = (cell.height * SAMPLE_ROW as f32) as usize;
    let x1 = (cell.width * (SAMPLE_COL as f32 + 1.0)) as usize;
    let y1 = (cell.height * (SAMPLE_ROW as f32 + 1.0)) as usize;
    let mut max_green = -1;
    let mut min_red_at_high_green = i32::MAX;
    for py in y0..y1 {
        for px in x0..x1 {
            let off = (py * w as usize + px) * 4;
            let green = i32::from(pixels[off + 1]);
            let red = i32::from(pixels[off]);
            max_green = max_green.max(green);
            if green > 195 {
                min_red_at_high_green = min_red_at_high_green.min(red);
            }
        }
    }
    (max_green, min_red_at_high_green)
}

/// Scan the sample cell and return `(max_red, edge_samples)` among partial-coverage
/// glyph EDGE pixels — those with green in `[100, 180]`, which excludes the bg-only
/// exterior (green ≈ 19) and the fully-covered glyph interior (green ≈ 220). At
/// these edges the red channel discriminates the double-bg-apply bug: glyph-only
/// keeps red ≤ ~147, the double-applying form lifts it to ≥ ~155. `edge_samples`
/// lets the caller reject a vacuous pass (no edge pixels matched).
fn edge_red_signal(harness: &VisualSpecHarness, pixels: &[u8], w: u32) -> (i32, usize) {
    let cell = harness.renderer().cell_metrics();
    let x0 = (cell.width * SAMPLE_COL as f32) as usize;
    let y0 = (cell.height * SAMPLE_ROW as f32) as usize;
    let x1 = (cell.width * (SAMPLE_COL as f32 + 1.0)) as usize;
    let y1 = (cell.height * (SAMPLE_ROW as f32 + 1.0)) as usize;
    let mut max_red_at_edge = -1;
    let mut edge_samples = 0usize;
    for py in y0..y1 {
        for px in x0..x1 {
            let off = (py * w as usize + px) * 4;
            let green = i32::from(pixels[off + 1]);
            let red = i32::from(pixels[off]);
            if (100..=180).contains(&green) {
                max_red_at_edge = max_red_at_edge.max(red);
                edge_samples += 1;
            }
        }
    }
    (max_red_at_edge, edge_samples)
}
