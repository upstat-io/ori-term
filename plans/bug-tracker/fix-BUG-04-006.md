---
bug: "BUG-04-006"
title: "Subpixel/LCD glyph rendering shows heavy color fringing on regular text — looks \"too bold\""
severity: "high"
status: in-progress
goal: "Subpixel (LCD) text rendering matches DirectWrite/browser visual weight without saturated per-channel halos, at parity with the grayscale path's DirectWrite match."
success_criteria:
  - "Subpixel RGBA glyph bitmaps are emitted by `FontCollection::rasterize` / `rasterize_with_weight` WITHOUT a byte-wise gamma 1.8 LUT pass — per-channel coverage values are preserved as swash/zeno produced them."
  - "Grayscale (`GlyphFormat::Alpha`) bitmaps continue to receive the existing gamma 1.8 boost — no regression on the grayscale path that already matches DirectWrite."
  - "New tests in `oriterm/src/font/collection/tests.rs` semantically pin (a) subpixel bytes are untouched by `apply_alpha_correction`, (b) subpixel rasterization of 'H' does NOT boost intermediate values via the gamma LUT, (c) alpha path still applies the boost — each test fails against the current (buggy) code and passes after the fix."
  - "`timeout 150 ./test-all.sh`, `./clippy-all.sh`, `./build-all.sh` green. `cargo test -p oriterm` green. GPU visual regression suite under `oriterm/src/gpu/visual_regression/` green on the cached path."
subsystem: "oriterm/src/font/collection/rasterize.rs"
found: "2026-04-22"
source: "manual"
third_party_review:
  status: none
  updated: null
---

# Fix: BUG-04-006 — Subpixel/LCD glyph rendering shows heavy color fringing — looks "too bold"

**Status:** In Progress
**Severity:** high
**Goal:** Stop applying the grayscale gamma 1.8 LUT to subpixel RGBA coverage bytes so subpixel text is not over-bolded and per-channel fringes are not amplified, while preserving the gamma boost on the grayscale path that already matches DirectWrite.

**Success Criteria:**
- [ ] `FontCollection::rasterize` and `FontCollection::rasterize_with_weight` gate `apply_alpha_correction` to `GlyphFormat::Alpha` only.
- [ ] New tests semantically pin that subpixel bitmaps are NOT gamma-boosted and grayscale bitmaps still ARE.
- [ ] `./build-all.sh`, `./clippy-all.sh`, `timeout 150 ./test-all.sh` all green.
- [ ] GPU visual regression suite (`oriterm/src/gpu/visual_regression/`) green on the cached render path.

**Context:** User side-by-side compared oriterm vs Windows Terminal with the same font family/size/weight and saw oriterm "Final" looking heavier/bolder. Zoom reveals saturated red/blue/yellow LCD fringes on every glyph edge. User confirmed by bisection that switching Settings → Font AA from Subpixel to Grayscale makes the regression disappear — defect is fully contained in the subpixel/LCD AA path. Weight resolution is ruled out by the same bisection.

---

## 1. Root Cause Analysis

- **Symptom**: Subpixel-rendered terminal/UI text shows saturated per-channel color fringes (red/blue/yellow) on every glyph edge AND appears visibly heavier than Windows Terminal at the same font/size/weight. Switching to grayscale AA removes the effect.
- **Proximate cause**: `apply_alpha_correction` in `oriterm/src/font/collection/rasterize.rs:60` iterates over EVERY byte of the glyph bitmap through a gamma 1.8 LUT. For `GlyphFormat::SubpixelRgb` / `SubpixelBgr`, the bitmap is 4 bytes/pixel RGBA where R, G, B carry independent per-subpixel coverage (zeno `Format::Subpixel` rasterizes each channel at a different X sub-offset). The LUT is applied to each of R, G, B bytes independently.
- **Root cause**: The gamma boost is a compensation designed for SINGLE-CHANNEL alpha coverage — it raises low coverage values to thicken grayscale strokes against a linear-blended backdrop. Shader blending in oriterm IS already linear-space: `instance_writer::rgb_to_floats` (`oriterm/src/gpu/instance_writer/mod.rs:370-376`) passes fg/bg through `srgb_to_linear()` (`oriterm/src/gpu/mod.rs:48-67`) before upload; the subpixel atlas is `Rgba8Unorm` (linear, not sRGB — `oriterm/src/gpu/atlas/mod.rs:160-164`); the shader `mix()` (`subpixel_fg.wgsl:107-109`) interpolates linear values; the render target is sRGB via `add_srgb_suffix()` so wgpu auto-encodes linear-to-sRGB on write. Applying the concave gamma boost on top of a correctly-linear pipeline is the mistake only because it treats the 4-byte RGBA subpixel buffer as if every byte were an alpha scalar. Applying that same concave boost independently to three per-channel LCD coverage bytes produces two compounding errors:
  1. **Over-coverage per channel**: Each channel's stroke edge gets independently widened, so the perceived horizontal stroke width expands by up to ~3× the grayscale effect. Text reads as bolder than it should.
  2. **Amplified per-channel asymmetry**: Because the gamma curve is concave, LOW per-channel values get the largest relative boost. The LCD asymmetry that produces color fringes (R high, G medium, B low on a red-heavy edge) grows rather than softens when each channel is boosted in isolation. The visible "classic LCD halo" saturates into the reported red/blue/yellow fringes.

  The grayscale path has a single coverage value per pixel, so the boost simply thickens strokes uniformly — which is the intended DirectWrite match. There is no per-channel asymmetry to amplify, which is why the user's Grayscale AA workaround looks clean.
- **Blast radius**: Every subpixel terminal grid glyph and every subpixel UI-text glyph (tab titles, status bar, dialogs, overlays) whenever subpixel AA is active. `SubpixelMode::from_scale_factor` defaults to `SubpixelMode::Rgb` on any display with `scale_factor < 2.0`, so this is the DEFAULT path on every non-HiDPI monitor. No terminal state, grid data, or protocol handling is affected — purely a render quality regression.
- **Affected files**:
  - `oriterm/src/font/collection/rasterize.rs` — `rasterize` (terminal grid, line ~124) and `rasterize_with_weight` (UI text, line ~184) currently call `apply_alpha_correction` when `glyph.format != GlyphFormat::Color`. Change the guard to `glyph.format == GlyphFormat::Alpha` so only 8-bit grayscale coverage is boosted. The `apply_alpha_correction` doc-comment on line 59 must be updated to state it applies to `R8Unorm` coverage only (NOT subpixel); the module doc (lines 7–11) must reflect that the LCD/subpixel path is not gamma-boosted at rasterization.
  - `oriterm/src/font/collection/tests.rs` — add semantic + negative pin tests verifying that subpixel bitmaps are not gamma-boosted and the grayscale boost still applies.

**Reference implementations** (consulted):
- **WezTerm `wezterm-font/src/ftwrap.rs:1188`**: Uses FreeType's built-in LCD filter (`FT_LCD_FILTER_DEFAULT`) to smooth per-channel coverage before it ever reaches the app layer. No app-level gamma correction is applied to the bitmap; the shader does sRGB conversion at output.
- **Alacritty `alacritty/res/glsl3/text.f.glsl:69-72`**: Dual-source blending with per-channel `ALPHA_MASK = vec4(textColor.rgb, textColor.r)`; `FRAG_COLOR = vec4(fg.rgb, 1.0)`. No bitmap-level gamma boost — relies on FreeType LCD filter for color-fringe suppression.
- **zeno `zeno-0.3.3/src/mask.rs:266-278`**: `Format::Subpixel` rasterizes each of R, G, B at subpixel X offsets -0.3, 0.0, +0.3; A channel is never written (stays 0). No LCD filter is applied by zeno — the per-channel values are RAW rasterization coverage.
- **oriterm `subpixel_fg.wgsl:107-109` / `subpixel_fg_dual.wgsl:94-97`**: Both shaders mix per-channel: `mix(bg.r, fg.r, mask.r * dim)` etc. They operate on the mask as-is; the gamma LUT applied at rasterization time inflates the `mask.rgb` values before they reach the shader. Removing the LUT for subpixel aligns oriterm with the alacritty/wezterm model (raw per-channel coverage feeds the shader).

---

## 1.5 Fix Consensus (via /tp-help)

Independent dual-source design review of the proposed fix approach. Ran BEFORE tests or implementation to catch wrong-approach errors before they lock in. See `.claude/skills/fix-bug/SKILL.md` § Phase 1.75 for the calling contract.

- **Proposed approach (pre-consensus)**:
  - In `FontCollection::rasterize` and `FontCollection::rasterize_with_weight`, replace the current guard `if glyph.format != GlyphFormat::Color { apply_alpha_correction(...); }` with `if glyph.format == GlyphFormat::Alpha { apply_alpha_correction(...); }`.
  - Update `apply_alpha_correction`'s doc-comment to state it is for single-channel `R8` coverage only.
  - Update the module-level `//!` doc on `rasterize.rs` (lines 7-11) to reflect that the subpixel/LCD path is NOT gamma-boosted at rasterization — the gamma match for subpixel will be deferred to the shader/blend layer in a future plan.
  - Do NOT add an LCD filter (3-tap or FreeType-style) in this fix — that is a separate improvement.
  - Do NOT change shader blending (linear-space vs sRGB-space) in this fix — that is a separate improvement.
  - Do NOT change `SubpixelMode::from_scale_factor` defaults — the user's workaround (Grayscale) remains available but subpixel stays the default on non-HiDPI.
- **tp-help run scratch dir**: `/tmp/tpr-round-ori_term-PIsN9d1d`

### Round 1

- **Codex summary**: Agrees with the proposed guard change as the correct fix surface. Explicitly verifies that oriterm's shader path is already linear-space (instance colors decoded via `srgb_to_linear()` before upload, subpixel atlas is `Rgba8Unorm` not sRGB, render target is sRGB), so NO shader change should be bundled. Notes one real adjacent defect: no LCD filter in the swash/zeno subpixel path — but recommends landing the Alpha-only gate FIRST, verifying visually, and only then escalating to an LCD filter in a follow-on if raw zeno LCD separation is still unacceptable (causality of any post-fix regression stays clear). No production consumer of `RasterizedGlyph.bitmap` silently depends on the boosted subpixel bytes — atlas uploader writes verbatim, shaders consume whatever mask is uploaded with no compensating boost. Flags that the `subpixel_rgb` visual regression golden (`oriterm/src/gpu/visual_regression/reference_tests.rs:418-480`) WILL need re-baselining — not a silent dependency, it should fail loudly. Surfaces a NEW matrix-cell requirement: `embolden_strength()` in `oriterm/src/font/collection/face.rs:249-259` was explicitly tuned down (Ghostty's 2.0 → 1.5) to compensate for the gamma correction. Once subpixel stops getting that correction, synthetic-bold subpixel text may end up lighter than intended — add a test cell for synthetic-bold subpixel weight.
- **Gemini summary**: Agrees with the guard change. Independently verifies the linear-space-blending claim via `rgb_to_floats` + `srgb_to_linear` and the subpixel atlas format. Characterizes the gamma LUT as a "poor man's thickening" heuristic — fine for grayscale, mathematically destructive when applied per-channel to subpixel coverage because it expands R, G, B stroke widths independently and amplifies the very color halos that LCD AA is meant to balance. Agrees LCD filter is a follow-on quality enhancement, not part of this fix. Flags a subtle secondary effect: the "Unknown background" fallback path in `subpixel_fg.wgsl:128-132` collapses RGB coverage to a single grayscale `a` — after the fix, subpixel glyphs in that fallback state will look thinner than standard `Alpha` glyphs. Gemini classifies this as acceptable-for-Phase-1 because it's only the transparent/unknown-bg fallback; the proper answer is a whole-glyph thickening algorithm or weight-adjustment setting, not a per-channel LUT.
- **Agreement points**: (1) Gate `apply_alpha_correction` to `Alpha` only — both endorse it as the correct, minimal, architecturally-correct fix. (2) Do NOT bundle LCD filter in this fix. (3) Do NOT change the shader — shader blending is already linear-space. (4) Keep grayscale boost; do NOT drop it. (5) The `subpixel_rgb` visual regression golden will need re-baselining — expected, not a regression.
- **Disagreement points**: None. Both reviewers converged in round 1.
- **Independent code verification**:
  - Confirmed `rgb_to_floats` applies `srgb_to_linear()` per channel at `oriterm/src/gpu/instance_writer/mod.rs:370-376` (grep output: use at lines 14, 190, 220, 261, 262, 287 of that file). Confirmed `srgb_to_linear` decoder at `oriterm/src/gpu/mod.rs:48-67`. → linear-space-blending claim is TRUE; original §1 wording about "shader mixes in sRGB space" was wrong and has been corrected.
  - Confirmed subpixel atlas format gating at `oriterm/src/gpu/atlas/mod.rs:160-164` (`GlyphFormat::SubpixelRgb|Bgr => TextureFormat::Rgba8Unorm`; `Color => Rgba8UnormSrgb`). → "atlas is linear not sRGB" claim is TRUE.
  - Confirmed `embolden_strength()` scale factor at `oriterm/src/font/collection/face.rs:249-259` (reduced from Ghostty's 2.0 to 1.5 with a doc comment explicitly citing gamma-correction compensation). → synthetic-bold subpixel-thinning concern is TRUE; the TDD matrix needs a cell for it.
  - Confirmed `subpixel_fg.wgsl:128-132` grayscale fallback path (`max(mask.r, max(mask.g, mask.b))`). → Gemini's fallback-thinning concern is TRUE but scoped to the transparent/unknown-bg path only; the dominant terminal-grid path always has a known bg.
- **Outcome**: Agreement → proceed to Phase 2 with the original approach AND (a) correcting §1 re: linear-space blending, (b) adding a synthetic-bold subpixel matrix cell to §2, (c) explicitly recording the visual-regression golden re-baseline in §3.

### Final agreed approach

Gate `apply_alpha_correction` to `GlyphFormat::Alpha` only in both `FontCollection::rasterize` (line 124) and `FontCollection::rasterize_with_weight` (line 184) via `if glyph.format == GlyphFormat::Alpha`. Update `apply_alpha_correction`'s doc-comment (line 59) to narrow its contract to `R8Unorm` coverage only. Update the module `//!` doc (lines 7-11) to match. Do NOT change shader code (blending is already linear-space). Do NOT add an LCD filter in this fix (follow-on if post-fix visual check still shows unacceptable raw-zeno fringing). Do NOT change `SubpixelMode::from_scale_factor` defaults. Add the synthetic-bold subpixel matrix cell Codex surfaced. Re-baseline the `subpixel_rgb` visual-regression golden that is expected to shift (if the regression harness requires manual acceptance, use `cargo insta review` or the project's equivalent).

---

## 2. TDD — Test Matrix

Tests separated into two classes per `.claude/rules/tests.md §TDD for Bugs`:

- **Red-first fail pins** — MUST fail against HEAD before the fix and pass after, without test modification. These are the regression-catching pins.
- **Invariant / regression guards** — may be green against HEAD; they pin behavior that holds either way so a future refactor cannot silently violate it.

### Test construction — gamma-swap technique

Both fail pins and invariant guards pair two `FontCollection` instances identical except for `gamma_lut`: one built at default gamma 1.8, one with gamma 1.0 (identity LUT) via `#[cfg(test)] pub(super) set_gamma_for_test(1.0)` on `FontCollection` (`oriterm/src/font/collection/mod.rs`). The helper installs a fresh identity LUT and clears the glyph cache + cache-bytes counter so subsequent `rasterize` calls re-rasterize from scratch.

This construction replaced the originally-planned "direct `rasterize_from_face` comparison" because reproducing the internal state `rasterize` passes to `rasterize_from_face` (face lookup, `face_variations`, `effective_synthetic`, `metrics.height`, `hinting.hint_flag()`, `scale_context`) would have required either widening many private fields to `pub(super)` or duplicating ~30 lines of `rasterize`'s body in the test helper. The gamma-swap is semantically equivalent (pre-fix: different bitmaps; post-fix: identical) and minimally invasive (one `#[cfg(test)]` accessor).

### Red-first fail pins (must fail on HEAD, pass after the fix)

- [ ] `subpixel_rgb_bitmap_not_gamma_boosted` (terminal grid, RGB): build fc_gamma and fc_raw as `SubpixelRgb` collections differing only in gamma (1.8 vs 1.0). Rasterize 'H' through both via `rasterize`. Assert `gamma_bitmap == raw_bitmap`. On HEAD: both apply `apply_alpha_correction`, so the two LUTs produce different bitmaps → fails. After fix: neither applies correction (guard narrowed to Alpha-only) → bitmaps identical → passes.
- [ ] `subpixel_bgr_bitmap_not_gamma_boosted`: same construction for `GlyphFormat::SubpixelBgr` — covers the BGR path via `Format::subpixel_bgra()`.
- [ ] `subpixel_rgb_ui_text_path_bitmap_not_gamma_boosted`: same construction for `FontCollection::rasterize_with_weight` (UI text weight-aware path; `requested_weight = 400` avoids the 500..700 medium-face substitution). The bug exists in both guarded sites; both need independent red-first coverage.

### Invariant / regression guards (green on HEAD, catch future refactors)

- [ ] `alpha_bitmap_still_gamma_boosted` (grayscale regression pin): same gamma-swap construction for `GlyphFormat::Alpha`. Assert `gamma_bitmap != raw_bitmap` AND `mean(gamma_bitmap) > mean(raw_bitmap)`. Green on HEAD and post-fix; fires if a future refactor widens the Alpha-only guard to exclude Alpha.
- [ ] `alpha_ui_text_path_bitmap_still_gamma_boosted`: same for `rasterize_with_weight` on `GlyphFormat::Alpha`. Pins the weight-aware Alpha guard independently so a refactor touching only `rasterize` cannot silently regress the UI path.
- [ ] `subpixel_rgb_bitmap_alpha_channel_is_zero`: every 4th byte of a `SubpixelRgb` bitmap equals exactly 0 (zeno never writes the A channel in subpixel mode; `LUT[0] == 0` regardless of gamma). Catches a regression where someone starts writing to the A channel or flips `LUT[0]`.

(No direct `apply_alpha_correction` white-box test — the function is private in `rasterize.rs` and widening the visibility is scope creep. Grayscale behavior is exercised indirectly through the two `alpha_*_still_gamma_boosted` invariant guards via the public `rasterize` / `rasterize_with_weight` entry points.)

### Cross-feature / synthetic-bold interaction

- [ ] `synthetic_bold_subpixel_rgb_heavier_than_regular`: rasterize 'H' as Synthetic-Bold vs Regular in `SubpixelRgb`. Pin `bold_mean > regular_mean`. Catches the `embolden_strength()` interaction from §1.5 (multiplier reduced from 2.0 → 1.5 at `oriterm/src/font/collection/face.rs:249-259` specifically to compensate for gamma correction subpixel will no longer receive). Pre-condition: asserts `resolved_bold.synthetic.contains(SyntheticFlags::BOLD)` so that if a native Bold face is ever added to `FontSet::embedded()` the test fails loudly rather than silently exercising native-bold.

### Visual regression (not a new test — goldens will shift)

The subpixel visual-regression cell lives in `oriterm/src/gpu/visual_regression/reference_tests.rs` (`subpixel_vs_grayscale` at line 424, renders text `"The quick brown fox jumps over the lazy dog 0123456789"`, compares via `compare_with_reference("subpixel_rgb", ...)` at line 479). The reference PNGs are in `oriterm/tests/references/` and are compared via `compare_with_reference` in `oriterm/src/gpu/visual_regression/compare.rs` — overwrite mode is `ORITERM_UPDATE_GOLDEN=1` (line 39, 147), guarded against CI at line 250-254.

The quantitative proof of the semantic change belongs to the red-first rasterizer pins above (`subpixel_rgb_bitmap_not_gamma_boosted`, `subpixel_bgr_bitmap_not_gamma_boosted`, `subpixel_rgb_ui_text_path_bitmap_not_gamma_boosted`) — they are deterministic, run without a GPU, and fail on HEAD. The visual regression's role here is QUALITATIVE drift detection + manual-review sign-off: the golden shift must be reviewed by eye to confirm the rendered text is the cleaner un-haloed form. A numeric pre-assertion on the rendered PNG was considered and rejected — it would (a) attach to the non-deterministic `headless_env()` / `headless_env_full()` GPU lane, (b) require fixture knowledge about character stroke positions (the current fixture has no capital 'H'), and (c) encode a mathematically-wrong asymmetry invariant (the concave gamma boost REDUCES relative per-channel asymmetry, so removing it would INCREASE the `|R-B|/max(R,B)` ratio even while absolute fringes soften).

- [ ] **Run + update commands** (test target verified via `cargo metadata`; `oriterm/tests/` contains only `architecture.rs`, so visual regression runs as part of the lib test suite):
  ```
  timeout 150 cargo test -p oriterm visual_regression              # expected: subpixel_rgb golden fails
  ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm visual_regression  # overwrite guarded against CI
  ```
- [ ] **Manual visual review before accepting the overwrite**: open the pre- and post-overwrite PNGs side by side; confirm (a) vertical-stroke edges in the post-overwrite image show less saturated per-channel fringing than HEAD, (b) overall glyph weight is lighter (matches Windows Terminal at the same point size per the repro), (c) no unrelated drift in spacing / baseline / background. Refuse the re-baseline if any of these fail — that indicates unrelated drift.
- [ ] Record every golden file that moves in the commit message body — one line per file, with byte-count delta.

### Verify tests fail before fix
- [ ] Before the `rasterize.rs` guard change, ALL red-first fail pins above fail against HEAD. Capture the output and cite the specific assertion lines in the fix commit.
- [ ] All invariant guards pass against HEAD (documents the invariant pre-fix so post-fix regressions cannot be attributed to "the test was always green anyway").

---

## 2.5 Fix Plan TPR Findings

Adversarial review of this fix PLAN (§1–§3) before implementation. Ran AFTER `/tp-help` consensus (§1.5) and plan finalization (§2) but BEFORE writing tests or code.

**Gate:** Mandatory — severity `high` AND complexity-elevated subsystem (font pipeline → GPU render path).

- **TPR run**: 2026-04-22, scratch `/tmp/tpr-round-ori_term-thRkFYdK`
- **Key findings (verified against code)**:
  1. **[AGREEMENT — codex F1 + gemini F-04]** (high) Red-first gate mixed fail-first pins with invariant guards. Cells like `subpixel_bitmap_alpha_channel_zero` and `apply_alpha_correction_direct_call` are always green — they pin invariants, not the bug. Resolution: split §2 into "red-first fail pins" (must fail on HEAD, pass after fix) and "invariant / regression guards" (green either way, catch future refactors). Every red-first pin now relies on a direct `rasterize_from_face` vs `FontCollection::rasterize` comparison that is demonstrably different on HEAD vs post-fix.
  2. **[AGREEMENT — codex F2 + gemini F-01]** (high) `subpixel_bitmap_bytes_not_in_lut_image` negative pin was mathematically unsound — the "b == LUT[x] for some x != b" construction produces false positives because zeno's raw coverage can incidentally equal a LUT image value. Resolution: replaced with `subpixel_bitmap_matches_raw_swash_output_not_lut_image`, a direct byte-for-byte comparison between `rasterize_from_face` output (raw) and `rasterize` output (under test). On HEAD the two differ (LUT applied); after the fix they match.
  3. **[codex F3]** (medium) `color_glyph_bitmap_unchanged` cell mirrored the existing skip-based color-emoji test at `oriterm/src/font/collection/tests.rs:666`, inheriting the same "no-emoji-font → silent pass" weakness. Resolution: removed the cell. Existing test at `tests.rs:666` already provides the defense; a second skip-based test adds no coverage.
  4. **[codex F4]** (medium) Visual-regression §3 step named `cargo test -p oriterm --test main_window` as the test binary and "insta review or equivalent" as the update mechanism — both wrong. `cargo metadata` confirms the only integration target in `oriterm/tests/` is `architecture`; visual regression runs under the lib tests via `cargo test -p oriterm visual_regression`. Project uses `ORITERM_UPDATE_GOLDEN=1` (grep-confirmed at `oriterm/src/gpu/visual_regression/compare.rs:39,147,250`). Resolution: rewrote the §2 "Visual regression" block with the correct target + `ORITERM_UPDATE_GOLDEN=1` mechanism + a mandatory semantic pre-assertion (R-channel mean coverage decrease on vertical strokes + per-channel asymmetry bound) so re-baseline cannot hide unrelated drift.
  5. **[gemini F-03]** (medium) No §2 cell pinned the `rasterize_with_weight` Alpha-path gamma guard independently — only the terminal-grid `rasterize` path had direct Alpha coverage. A refactor that flipped only one of the two guards would escape. Resolution: added `alpha_bitmap_ui_text_path_still_gamma_boosted` as an invariant guard.
- **Dropped**:
  - **[gemini F-02]** (medium) Claimed the plan "references a missing §5" — the fix section has no §5 reference anywhere. Grep-verified zero occurrences of `§5`, `§ 5`, `section 5`, or `see §` in `plans/bug-tracker/fix-BUG-04-006.md`. Hallucination, dropped.
- **Plan revisions**: §2 TDD Matrix rewritten (red-first vs invariant guards split; negative pin replaced with direct `rasterize_from_face` comparison; color-glyph cell removed; UI-text Alpha cell added; synthetic-bold cell hardened with font-selection pre-condition; visual regression mechanism corrected with semantic pre-assertion). §3 already carries the visual-regression note; the §2 rewrite is the authoritative mechanism spec.
- **Round 1 verification** (after applying round 0 resolutions):
  6. **[AGREEMENT — codex F1 + gemini F1]** (medium) `apply_alpha_correction` is private at `oriterm/src/font/collection/rasterize.rs:60` — sibling `tests.rs` cannot call it directly without a visibility widening. Bundling the widening into the TDD matrix is scope creep. Resolution: REMOVED the `apply_alpha_correction_direct_call` cell. Grayscale boost is already exercised indirectly through `alpha_bitmap_still_gamma_boosted` and `alpha_bitmap_ui_text_path_still_gamma_boosted` via the public `rasterize` / `rasterize_with_weight` entry points; a direct-call pin is redundant.
  7. **[codex F2 + gemini F2 + gemini F3]** (medium, three-angle agreement) The visual-regression semantic pre-assertion had three independent defects: (a) attached to the non-deterministic `headless_env()`/`headless_env_full()` GPU lane (codex F2); (b) cited 'H' stroke columns but the fixture `"The quick brown fox..."` at `reference_tests.rs:440` has no capital H (gemini F2); (c) the asymmetry invariant `|mean(R) - mean(B)| / max` was mathematically backwards — the concave gamma boost REDUCES relative per-channel asymmetry (verified with R=50,B=200 → rel=0.75 vs LUT(50)≈104, LUT(200)≈227 → rel≈0.54), so removing the boost would INCREASE the ratio even when absolute fringes soften (gemini F3). Resolution: REMOVED the semantic pre-assertion entirely. The red-first rasterizer pins are the deterministic semantic proof (`subpixel_bitmap_matches_raw_swash_output_not_lut_image*`, `subpixel_mean_coverage_lower_than_gamma_boosted`); the visual-regression lane's role is now explicitly qualitative drift detection + manual-review sign-off. The §2 "Visual regression" block documents both the rejected pre-assertion and the rationale so a future reviewer cannot re-introduce the unsound invariant.
  8. **[gemini F4]** (informational) Test import list needs `rasterize_from_face` added. No plan change — captured implicitly by Phase 3 implementation.
- **Outcome**: Clean after two rounds. §2 is now a reliable red-first / invariant-guard split with deterministic rasterizer-level semantic proof. §3 visual regression is an explicitly qualitative confirmation step. Proceed to Phase 3 (write tests).

---

## 3. Implementation

- [ ] Gate `apply_alpha_correction` to `GlyphFormat::Alpha` in both `FontCollection::rasterize` and `FontCollection::rasterize_with_weight`:
  ```rust
  // Boost glyph coverage to match DirectWrite/browser visual weight.
  // Gamma compensates for sRGB-space compositing with a single-channel
  // coverage mask — per-byte boosting is wrong for subpixel masks where
  // each byte is an independent per-channel LCD coverage value.
  if glyph.format == GlyphFormat::Alpha {
      apply_alpha_correction(&mut glyph, &self.gamma_lut);
  }
  ```
- [ ] Update `apply_alpha_correction` doc comment to narrow the contract:
  ```rust
  /// Apply gamma-aware alpha correction to 8-bit monochrome glyph coverage.
  ///
  /// Transforms each byte through the pre-built LUT: `byte = lut[byte]`.
  /// Applied to `GlyphFormat::Alpha` (R8Unorm) bitmaps only. Must NOT be
  /// applied to `SubpixelRgb`/`SubpixelBgr` (per-channel LCD coverage — the
  /// concave gamma boost magnifies channel asymmetry into saturated color
  /// fringes) nor to `Color` (premultiplied RGBA color data).
  ```
- [ ] Update the module-level `//!` doc (lines 7-11) to reflect the narrowed contract.
- [ ] Add the tests from §2 to `oriterm/src/font/collection/tests.rs` following the sibling `tests.rs` pattern (tests already live there; no new file needed).
- [ ] Verify tests FAIL on HEAD before the guard change, PASS after.
- [ ] Run the GPU visual regression suite under the lib tests (the only integration target in `oriterm/tests/` is `architecture`; visual regression lives under `oriterm/src/gpu/visual_regression/`):
  ```
  timeout 150 cargo test -p oriterm visual_regression              # subpixel_rgb golden will diff
  ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm visual_regression  # overwrite; CI-guarded per compare.rs:250-254
  ```
  Before accepting the overwrite, do the §2 "Manual visual review" checklist (vertical-stroke fringing softened, overall glyph weight lighter, no spacing / baseline / background drift). The red-first rasterizer pins in §2 are the quantitative semantic proof; the golden lane is qualitative drift detection. List every moved golden in the commit body with its byte-count delta so the re-baseline is auditable.
- [ ] If the `synthetic_bold_subpixel_vs_regular_weight` test from §2 surfaces genuinely-too-light synthetic-bold subpixel output, implement a format-aware `embolden_strength()` in `oriterm/src/font/collection/face.rs` — Alpha keeps `1.5`, subpixel formats use a larger multiplier (starting point: `2.0`, matching Ghostty). Do NOT restore the per-channel gamma boost as the remediation.

---

## R. Third Party Review Findings

{Initially empty — populated by the executor during Phase 5 completion checklist.}

---

## 4. Completion Checklist

Reviews MUST complete before bug closure — a bug marked resolved before TPR/hygiene is a premature closure.

- [ ] All new tests pass unchanged after fix (no test modifications needed)
- [ ] Matrix completeness verified — RGB + BGR subpixel paths (`subpixel_rgb_bitmap_not_gamma_boosted`, `subpixel_bgr_bitmap_not_gamma_boosted`), terminal + UI-text rasterizers (`subpixel_rgb_ui_text_path_bitmap_not_gamma_boosted` + `alpha_ui_text_path_bitmap_still_gamma_boosted`), grayscale regression pin (`alpha_bitmap_still_gamma_boosted`), A-channel invariant (`subpixel_rgb_bitmap_alpha_channel_is_zero`), synthetic-bold interaction (`synthetic_bold_subpixel_rgb_heavier_than_regular`). Color-emoji cell intentionally NOT included — the existing `rasterize_emoji_as_color_format` test (tests.rs:666) is the Color-path guard; a second skip-based cell adds no coverage.
- [ ] Debug AND release builds pass (`cargo b && cargo b --release`)
- [ ] Windows cross-compile green (`cargo build --target x86_64-pc-windows-gnu`)
- [ ] GPU visual-regression suite under `oriterm/src/gpu/visual_regression/` green (cached path via `render_frame_cached`)
- [ ] `oriterm_core/tests/alloc_regression.rs` and `rss_regression.rs` still green
- [ ] `timeout 150 ./test-all.sh` green — no regressions
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green (workspace + cross-compile)
- [ ] `cargo test -p oriterm` green
- [ ] `/commit-push` — commit all changes before review
- [ ] Plan TPR (Phase 2.5) — mandatory (high severity, complexity-elevated subsystem). See §2.5 above.
- [ ] `/tpr-review` (Phase 5 — code review) passed
- [ ] `/impl-hygiene-review` passed
- [ ] **Capability regression gate** — not applicable: the fix narrows an incorrectly-applied correction; no capability is disabled.
- [ ] `/improve-tooling` retrospective completed
- [ ] Bug entry in `plans/bug-tracker/section-04-fonts.md` updated: `- [x]` with resolution details
- [ ] Fix section frontmatter `status` updated to `complete`
- [ ] Bug-tracker `00-overview.md` Quick Reference open bug count updated
- [ ] Final `/commit-push` — commit closure artifacts

**Exit Criteria:** `cargo test -p oriterm` green, `./build-all.sh` + `./clippy-all.sh` + `timeout 150 ./test-all.sh` green, the new §2 tests pass without modification after the `rasterize.rs` guard change and fail against HEAD before it. Visual verification on Windows: user launches oriterm at 1x scale with default (Subpixel) Font AA and reports that text weight matches Windows Terminal at the same font/size/weight with no saturated color halos on glyph edges.
