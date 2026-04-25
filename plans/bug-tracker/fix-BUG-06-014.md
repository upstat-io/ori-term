---
bug: "BUG-06-014"
title: "SGR 53/73/74 flags stored on cells but not rendered — no visual effect for overline, superscript, subscript"
severity: "medium"
status: in-progress
goal: "OVERLINE/SUPERSCRIPT/SUBSCRIPT CellFlags reach the GPU decoration pipeline and the HTML export so a user feeding `\\x1b[53m`/`\\x1b[73m`/`\\x1b[74m` sees the corresponding visual effect on screen and in copied rich text."
success_criteria:
  - "Cell with `CellFlags::OVERLINE` emits a stroke-thickness rect at the cell's top edge (verified by new GPU prepare test)."
  - "Cell with `CellFlags::SUPERSCRIPT` shifts the shaped/built-in/unshaped glyph y-position upward by 25% of cell height; `CellFlags::SUBSCRIPT` shifts downward by the same amount (verified by new GPU prepare tests across all three glyph paths)."
  - "HTML export emits `text-decoration: overline` for OVERLINE, `vertical-align: super` + `font-size: 0.83em` for SUPERSCRIPT, `vertical-align: sub` + `font-size: 0.83em` for SUBSCRIPT, including correct combinations with underline/strikethrough."
  - "All existing decoration tests + new tests pass without modification (`cargo test -p oriterm` and `cargo test -p oriterm_core`)."
  - "Plan TPR (Phase 2.5) and Code TPR (Phase 5) clean; `/impl-hygiene-review` clean."
subsystem: "oriterm/src/gpu/prepare/{decorations,emit,mod,unshaped,dirty_skip/mod}.rs, oriterm_core/src/selection/html/mod.rs"
found: "2026-04-14"
source: "tpr-review (codex, spec-conformance §08 round 12)"
third_party_review:
  status: none
  updated: null
---

# Fix: BUG-06-014 — SGR 53/73/74 flags stored on cells but not rendered

**Status:** In Progress
**Severity:** medium
**Goal:** Wire the `OVERLINE`, `SUPERSCRIPT`, and `SUBSCRIPT` `CellFlags` (added in spec-conformance §08) into the two consumers that ignore them today: the GPU prepare pipeline (decorations + glyph emission) and the HTML clipboard export. After this fix, feeding `\x1b[53m`, `\x1b[73m`, or `\x1b[74m` produces the corresponding visual effect on screen AND survives a copy-paste into a rich-text editor.

**Success Criteria:**
- [ ] OVERLINE → stroke-thickness rect at cell top edge in `DecorationContext::draw`.
- [ ] SUPERSCRIPT → glyph y shifted up by 25% of cell height (wezterm-compatible offset).
- [ ] SUBSCRIPT → glyph y shifted down by 25% of cell height.
- [ ] HTML export maps OVERLINE/SUPERSCRIPT/SUBSCRIPT to the appropriate CSS in `CellStyle::write_css` and `CellStyle::is_default`.
- [ ] Plan TPR + Code TPR + impl-hygiene clean.

**Context:** spec-conformance §08 added the three flags + the SGR handlers + DECRQSS reporting + handler-level tests. The downstream rendering / export consumers were not updated in that section because they live in different crates (`oriterm` for GPU, `oriterm_core` for HTML) and `/tpr-review` round 12 (codex, TPR-08-004-codex-r12) caught the gap. The bug entry was filed 2026-04-14. This fix completes the SGR 53/73/74 surface end-to-end.

---

## 1. Root Cause Analysis

- **Symptom**: `printf '\x1b[53mFoo\x1b[0m'` and `printf '\x1b[73mFoo\x1b[0m'` produce no visible change. Selection HTML export omits the same attributes when copying SGR 53/73/74 cells.
- **Proximate cause**:
  - `oriterm/src/gpu/prepare/decorations.rs` — `DecorationContext::draw()` checks only `ALL_UNDERLINES | STRIKETHROUGH | hyperlink`. No reference to `OVERLINE`, `SUPERSCRIPT`, or `SUBSCRIPT`.
  - `oriterm/src/gpu/prepare/{mod.rs, dirty_skip/mod.rs, unshaped.rs}` — three glyph-emission paths compute `gy = y + baseline - bearing_y[ - sg.y_offset]` with no cell-flag-driven vertical adjustment.
  - `oriterm_core/src/selection/html/mod.rs` — `CellStyle::from_cell` and `CellStyle::write_css` consume only BOLD/DIM/ITALIC/INVERSE/UNDERLINE-variants/STRIKETHROUGH. OVERLINE/SUPERSCRIPT/SUBSCRIPT are silently dropped.
- **Root cause**: The two consumers were never extended when bits 16/17/18 were added to `CellFlags` in `oriterm_core/src/cell/mod.rs:40-42`. The handler side stores the flags faithfully (`oriterm_core/src/term/handler/sgr.rs:61-74`) but the renderers/exporters do not query them — a classic SSOT shadow-home: the bit is defined in `cell/mod.rs`, but downstream consumers do not exhaustively match against it.
- **Blast radius**: Three glyph-emit sites (shaped production, dirty-skip incremental production, unshaped test) plus one decoration site plus one HTML export site. No correctness ripple beyond these — the flags are already accepted/cleared cleanly, parser/handler/snapshot pipeline is sound.
- **Affected files**:
  - `oriterm/src/gpu/prepare/decorations.rs` — add OVERLINE rect emission in `DecorationContext::draw`.
  - `oriterm/src/gpu/prepare/mod.rs` — compute `super_sub_offset(flags, cell_height)` and apply to shaped + built-in glyph y; expose the helper for sibling modules.
  - `oriterm/src/gpu/prepare/dirty_skip/mod.rs` — apply the same offset in the incremental path.
  - `oriterm/src/gpu/prepare/unshaped.rs` — apply the offset in the test-only unshaped path so the matrix tests share one semantics.
  - `oriterm_core/src/selection/html/mod.rs` — extend `CellStyle` with `overline: bool` + `vertical_align: VerticalAlign` enum; emit CSS in `write_css`; include in `is_default`.

**Reference implementations**:
- **wezterm** `wezterm-gui/src/glyphcache.rs:1312-1328` — overline drawn as a `metrics.underline_height`-thick line at the top of the cell rect, foreground color.
- **wezterm** `wezterm-gui/src/termwindow/render/screen_line.rs:437-445` — `valign_adjust` = `cell_size.height * -0.25` for SuperScript, `+ 0.25` for SubScript; offset applied to the per-glyph baseline. No font-size reduction.
- **wezterm** does NOT shrink the glyph for super/sub — vertical offset only. Matches the stated YAGNI scope of this fix; font-pipeline-driven size reduction is explicitly out-of-scope per the bug entry note ("may need font pipeline changes").

---

## 1.5 Fix Consensus (via /tp-help)

Independent dual-source design review of the proposed fix approach. Ran BEFORE tests or implementation per Phase 1.75.

- **Proposed approach (pre-consensus)**:
  1. **Overline** — extend `DecorationContext::draw` to emit a `ScreenRect { x, y, w: cell_width, h: stroke_size }` rect when `flags.contains(OVERLINE)`. Color = `fg`. Position = cell top (i.e. the `y` parameter that already represents cell top). No interaction with hyperlink or hover dispatch.
  2. **Superscript / Subscript** — add a private helper in `prepare/mod.rs`:
     ```rust
     pub(super) fn super_sub_glyph_offset(flags: CellFlags, cell_height: f32) -> f32 {
         const FACTOR: f32 = 0.25;
         if flags.contains(CellFlags::SUPERSCRIPT) {
             -cell_height * FACTOR
         } else if flags.contains(CellFlags::SUBSCRIPT) {
             cell_height * FACTOR
         } else {
             0.0
         }
     }
     ```
     Apply to the `y` passed to `GlyphEmitter::emit` and to the built-in glyph rect's y in `fill_frame_shaped`, `fill_frame_incremental`, and `unshaped::fill_frame`. Backgrounds and decorations keep the unmodified cell-top y.
  3. **HTML export** — replace `CellStyle::from_cell`'s separate booleans with two additions: `overline: bool` (independent flag) and `vertical_align: VerticalAlign` enum (`{None, Super, Sub}`) since SUPERSCRIPT/SUBSCRIPT are mutually exclusive (enforced in `sgr.rs:64-69`). Emit:
     - OVERLINE → CSS `text-decoration` value gets `overline` token (combined with existing underline/strikethrough tokens).
     - SUPERSCRIPT → `vertical-align: super; font-size: 0.83em;`.
     - SUBSCRIPT → `vertical-align: sub; font-size: 0.83em;`.
     `0.83em` matches the de-facto browser default for `<sup>`/`<sub>` (eta = 13/15.6 ≈ 0.83). Update `is_default` to require the new fields at default.
- **tp-help run scratch dir**: `/tmp/tpr-round-ori_term-N5SIabUE`

### Round 1
- **Codex summary**: Off-topic. Codex returned five fabricated findings about `oriterm/src/settings/preview_renderer.rs` and `oriterm/src/settings/font_picker.rs` — neither file exists; the actual settings code lives under `oriterm/src/app/settings_overlay/` and `oriterm_ui/src/widgets/settings_*`. Treated as a Tier-failed reviewer at verification: zero verifiable claims pertain to the question.
- **Gemini summary**: Substantive convergence with the proposal. Recommends (a) keeping the 25% offset factor BUT integer-rounding the result to preserve Y-pixel-snap; (b) moving the helper to `CellMetrics` as the SSOT for cell-pixel geometry; (c) confirms the three glyph-emit call-site list (shaped + dirty_skip + unshaped); (d) confirms CSS `vertical-align` over `<sup>`/`<sub>` tags (preserves span coalescing); (e) flags additional matrix gaps — Bar cursor on SUPERSCRIPT cell stays full cell height, selection background fills full cell height with shifted glyph.
- **Agreement points**:
  - 25% offset factor (wezterm-compatible).
  - Three glyph-emit call sites + built-in glyph branches at each (shaped, dirty_skip, unshaped).
  - HTML uses CSS `vertical-align` + `font-size:0.83em` (NOT semantic `<sup>`/`<sub>` tags).
  - Decorations (overline / underline / strikethrough) stay anchored to cell, NOT shifted by super/sub.
- **Disagreement points (codex)**: None substantive — codex did not engage with the question. Its findings pertain to a fabricated codebase shape. Dropped at verification.
- **Disagreement points (gemini)**:
  - **Integer rounding** (persuaded divergence): gemini correct that `mod.rs:257` and `dirty_skip/mod.rs:250` use `(oy + row as f32 * ch).round()` to integer-snap Y. A fractional super/sub offset would defeat that snap and trigger bilinear filtering blur on fractional-DPI cells (e.g. `cell_height = 13.0` → `13 * 0.25 = 3.25`).
  - **Helper home `CellMetrics` vs `prepare/mod.rs`** (unpersuaded divergence): gemini argues `CellMetrics` is SSOT for cell-pixel geometry. Counter: `CellMetrics` (oriterm/src/font/mod.rs:32-103) currently has zero `CellFlags` coupling and zero SGR-policy coupling; its fields (width, height, baseline, underline_offset, stroke_size, strikeout_offset) are font-derived geometry. The 25% factor is **render-pipeline policy** — a visual convention chosen by ori_term, not a value computed from the font. Putting it on `CellMetrics` would require importing `oriterm_core::CellFlags` into the font module and would invert the SSOT (visual-policy logic does not belong in geometry SSOT). Render-pipeline policies (decoration ordering, super/sub offset factors, blink-alpha curves) live where decoration policies live: in the prepare module.
- **Independent code verification**:
  - `oriterm/src/gpu/prepare/mod.rs:257`: `let y = (oy + row as f32 * ch).round();` — confirms gemini's integer-snap claim.
  - `oriterm/src/gpu/prepare/dirty_skip/mod.rs:250`: same snap. Confirms.
  - `oriterm/src/font/mod.rs:32-103`: `CellMetrics` has no `CellFlags` import or SGR-related field. Refutes gemini's "this is just adding to existing geometry SSOT" framing.
  - Codex's `oriterm/src/settings/preview_renderer.rs` claim: `find` confirms no such file exists. Path verification fails — finding dropped.
  - Codex's `oriterm/src/settings/font_picker.rs` claim: `find` confirms no such file exists. Path verification fails — finding dropped.
- **Outcome**: persuaded divergence (integer rounding accepted) + unpersuaded divergence (helper home stays in `prepare/mod.rs`). One reviewer (codex) failed to engage; survivor mode with gemini's substantive feedback. Proceed to Phase 2 with revised approach below — no further rounds (additional /tp-help would not change the design since codex's transport behavior is not a substantive disagreement).

### Final agreed approach

(A) **OVERLINE — GPU rect emission.** In `DecorationContext::draw`, after the strikethrough block, emit:
```rust
if flags.contains(CellFlags::OVERLINE) {
    self.backgrounds.push_rect(
        ScreenRect { x, y, w: cell_width, h: t },
        fg,
        self.alpha,
    );
}
```
where `t = self.metrics.stroke_size`. Color = fg. Position = cell top (the `y` parameter already in scope). No interaction with hyperlink dispatch.

(B) **SUPERSCRIPT / SUBSCRIPT — glyph y offset, integer-rounded.** Helper in `oriterm/src/gpu/prepare/mod.rs`:
```rust
pub(super) fn super_sub_glyph_offset(flags: CellFlags, cell_height: f32) -> f32 {
    const FACTOR: f32 = 0.25;
    let raw = if flags.contains(CellFlags::SUPERSCRIPT) {
        -cell_height * FACTOR
    } else if flags.contains(CellFlags::SUBSCRIPT) {
        cell_height * FACTOR
    } else {
        0.0
    };
    raw.round()
}
```
The `.round()` call preserves the integer-Y-pixel-snap discipline at `mod.rs:257` and `dirty_skip/mod.rs:250`. Apply at three call sites:
- `fill_frame_shaped` (mod.rs) — both shaped GlyphEmitter call AND built-in glyph rect.
- `fill_frame_incremental` (dirty_skip/mod.rs) — both glyph paths.
- `unshaped::fill_frame` (test-only) — single glyph path.
Backgrounds, decorations, and cursor keep the unmodified cell-top y so they stay anchored to the cell.

(C) **HTML export.** Extend `CellStyle` with `overline: bool` and `vertical_align: VerticalAlign` enum `{None, Super, Sub}`. CSS:
- OVERLINE → `text-decoration` value gains `overline` token (combined with underline/line-through space-separated per CSS spec).
- SUPERSCRIPT → `vertical-align:super;font-size:0.83em;`.
- SUBSCRIPT → `vertical-align:sub;font-size:0.83em;`.

(D) **TDD matrix additions per gemini's matrix-gap call**: Bar cursor on SUPERSCRIPT cell remains full cell height (cursor uses unshifted y); selection background fills full cell height even when glyph is shifted. Added below in §2.

---

## 2. TDD — Test Matrix

Write ALL tests BEFORE the fix. Verify they fail against current code.

### Exact failing case
- [ ] **GPU**: cell with `OVERLINE` only → exactly 1 decoration rect emitted at `y = cell_top`, height = `stroke_size`, width = `cell_width`, color = fg.
- [ ] **GPU**: cell with `SUPERSCRIPT` only → glyph y shifted up by `cell_height * 0.25` relative to baseline cell.
- [ ] **GPU**: cell with `SUBSCRIPT` only → glyph y shifted down by `cell_height * 0.25` relative to baseline cell.
- [ ] **HTML**: cell with OVERLINE → `<span style="text-decoration:overline;">…</span>`.
- [ ] **HTML**: cell with SUPERSCRIPT → `<span style="vertical-align:super;font-size:0.83em;">…</span>`.
- [ ] **HTML**: cell with SUBSCRIPT → `<span style="vertical-align:sub;font-size:0.83em;">…</span>`.

### Edge cases
- [ ] **GPU**: OVERLINE + UNDERLINE → both decorations emit (top + bottom rects).
- [ ] **GPU**: OVERLINE + STRIKETHROUGH → both decorations emit.
- [ ] **GPU**: OVERLINE + DOUBLE_UNDERLINE → 1 overline + 2 underline rects.
- [ ] **GPU**: OVERLINE + WIDE_CHAR → overline rect spans `2 * cell_width`.
- [ ] **GPU**: SUPERSCRIPT + UNDERLINE → glyph y shifted up; underline at unshifted y (tests that decoration y is NOT shifted).
- [ ] **GPU**: SUBSCRIPT + STRIKETHROUGH → glyph y shifted down; strikethrough at unshifted y.
- [ ] **GPU**: cell without any of OVERLINE/SUPERSCRIPT/SUBSCRIPT → zero extra decoration/glyph offset (existing tests still green).
- [ ] **HTML**: OVERLINE + UNDERLINE → CSS `text-decoration:underline overline;`.
- [ ] **HTML**: OVERLINE + STRIKETHROUGH → CSS `text-decoration:line-through overline;`.
- [ ] **HTML**: OVERLINE + UNDERLINE + STRIKETHROUGH → CSS `text-decoration:underline line-through overline;`.
- [ ] **HTML**: SUPERSCRIPT + UNDERLINE → CSS includes both `text-decoration:underline;` and `vertical-align:super;font-size:0.83em;`.

### Cross-pattern coverage
- [ ] **GPU shaped path** (`fill_frame_shaped` via `prepare_frame_shaped`) — assertions on `frame.glyphs` y-position via decoded instance bytes.
- [ ] **GPU unshaped path** (`fill_frame` via `prepare_frame`) — assertions on `frame.glyphs` y-position via decoded instance bytes.
- [ ] **GPU built-in glyph path** (a U+2500 box-drawing char with `SUPERSCRIPT`) — y shifted in built-in branch too.
- [ ] **GPU dirty-skip path** (`fill_frame_incremental`) — covered transitively by the helper since both shared paths call the same helper. Add at least one direct assertion.

### Cross-feature interactions
- [ ] **GPU** OVERLINE + hyperlink hover → overline rect emits AND hyperlink hover underline emits (no precedence collision).
- [ ] **GPU** SUPERSCRIPT + WIDE_CHAR → wide char's bg_w doubled; glyph y shifted; bg quad y NOT shifted.
- [ ] **GPU** OVERLINE + BLINK → overline rect inherits `deco_alpha` (text_blink_opacity).
- [ ] **GPU** Bar cursor on SUPERSCRIPT cell → cursor rect spans full cell height (uses unshifted y/ch), NOT shifted with glyph (per gemini matrix-gap call).
- [ ] **GPU** Block cursor on SUPERSCRIPT cell → cursor rect spans full cell height; cursor color rendering unaffected by glyph shift.
- [ ] **GPU** Selection background on SUBSCRIPT cell → background quad spans full cell height (cell rect, not glyph rect); only glyph y shifts.
- [ ] **GPU** Cell with SUPERSCRIPT + cell_height=13.0 → offset rounds to `-3.0` (not `-3.25`); preserves integer-Y-pixel-snap from `mod.rs:257`.
- [ ] **GPU** SUPERSCRIPT + INVERSE → INVERSE swaps fg/bg, background quad fills full cell rect (NOT shifted with glyph); glyph itself shifts up. Pinned via decoded-instance-bytes assertions on bg vs fg y-positions (TPR-06-014-codex F2).
- [ ] **GPU** SUPERSCRIPT + BLINK → glyph y shifts up AND glyph alpha follows `text_blink_opacity` curve; both effects compose, neither suppresses the other (TPR-06-014-codex F2).
- [ ] **GPU** SUBSCRIPT + BLINK → glyph y shifts down AND alpha follows blink curve; same composition as SUPERSCRIPT+BLINK (TPR-06-014-codex F2).
- [ ] **GPU** SUBSCRIPT cell at column 1 with non-shifted cells at columns 0 and 2 → glyph at column 1 shifts down by 4px; columns 0/2 unaffected; no horizontal-spacing artifacts (cell_width spacing preserved across the row) (TPR-06-014-codex F2).
- [ ] **GPU** OVERLINE + DOUBLE_UNDERLINE + STRIKETHROUGH on a single cell → 1 overline rect (top) + 2 underline rects (bottom + bottom-gap) + 1 strikethrough rect (middle). Total 4 decoration rects; each at the expected y; no interference (TPR-06-014-codex F2).
- [ ] **HTML** OVERLINE + UNDERLINE + STRIKETHROUGH on a single cell → CSS `text-decoration:underline line-through overline;` (single space-joined value via Vec join — pin coalescing) (TPR-06-014-gemini F2).

### Semantic pin
- [ ] **GPU**: `overline_emits_rect_at_cell_top_with_stroke_size_thickness` — the ONE test that fails iff overline support regresses.
- [ ] **GPU**: `superscript_shifts_glyph_y_up_by_quarter_cell_height` — semantic pin for SGR 73 visual offset.
- [ ] **GPU**: `subscript_shifts_glyph_y_down_by_quarter_cell_height` — semantic pin for SGR 74 visual offset.
- [ ] **HTML**: `overline_cell_emits_text_decoration_overline_css` — semantic pin for HTML overline mapping.

### Negative pin
- [ ] **GPU**: `overline_absent_emits_no_top_rect` — assert that without OVERLINE, decoration count is zero (rejects the false-positive that OVERLINE always emits).
- [ ] **GPU**: `decorations_y_unaffected_by_super_sub` — assert that SUPERSCRIPT/SUBSCRIPT do NOT change underline y or strikethrough y. This rejects the wrong-fix where the entire cell shifts (only the glyph should).
- [ ] **HTML**: `default_style_without_new_flags_unchanged` — `is_default()` returns true on a flag-empty cell (rejects regression where new fields default to non-default).

### Verify tests fail before fix
- [ ] All new tests fail against current code (confirming they test the right thing).

---

## 2.5 Fix Plan TPR Findings

**Gate:** Mandatory — complexity-elevated subsystem (GPU render pipeline) per `.claude/skills/fix-bug/SKILL.md` Phase 2.5 trigger gate.

- **TPR run**: 2026-04-25, scratch dir `/tmp/tpr-round-ori_term-EtuvoNWO`. Custom-objective mode, `--max-rounds=1`. Both reviewers returned `status: findings`. Total: 7 findings (3 codex + 4 gemini), all verified, all actionable (zero meta, zero dropped).
- **Key findings (each verified against actual code, all fixed inline in this plan before Phase 3):**
  - `[TPR-06-014-codex][high]` `oriterm/src/gpu/prepare/decorations.rs:71` — **Overline plan omits the decoration fast-path gate.** The `if !has_explicit_underline && !has_strikethrough && !has_hyperlink { return; }` early-return at line 71 would silently drop a cell that has only OVERLINE set. The plan said "after the strikethrough block, add..." but didn't update the predicate. Fixed in §3: introduce `let has_overline = flags.contains(CellFlags::OVERLINE);` and include it in the early-return check.
  - `[TPR-06-014-codex][medium]` `plans/bug-tracker/fix-BUG-06-014.md:182` — **Cross-feature matrix misses requested interactions.** Matrix lacks SUPERSCRIPT+INVERSE (full-cell background), super/sub+BLINK (alpha curve), SUBSCRIPT-adjacent-spacing (no horizontal artifacts on neighbour cells), and OVERLINE+DOUBLE_UNDERLINE+STRIKETHROUGH composition. Fixed in §2: added all four cells.
  - `[TPR-06-014-codex][low]` `plans/bug-tracker/fix-BUG-06-014.md:266-267` — **`cargo test` checklist entries lack mandatory `timeout 150`** per `.claude/rules/tests.md` §Running Tests. Fixed in §4 checklist.
  - `[TPR-06-014-gemini][high]` `plans/bug-tracker/fix-BUG-06-014.md:85` — **HTML export `font-size: 0.83em` breaks the monospace grid.** Within `<pre>`, character advance widths follow font-size; shrinking SUPERSCRIPT/SUBSCRIPT runs causes column-misalignment for adjacent cells. ALSO: GPU rendering applies vertical offset only (NO size reduction), so the HTML export and on-screen rendering would visually diverge. Fixed in §3: drop `font-size:0.83em`; emit only `vertical-align:super;` / `vertical-align:sub;`. Visual parity with GPU + grid integrity preserved.
  - `[TPR-06-014-gemini][medium]` `oriterm_core/src/selection/html/mod.rs:423-436` — **`text-decoration` CSS generation needs 3-way combination support.** Current 12-arm match becomes 24 arms when overline is added; structurally untestable. Fixed in §3: refactor to `let mut decs: Vec<&str> = Vec::new();` populated conditionally, then `decs.join(" ")` to produce the value. SSOT for decoration tokens, no n-arm explosion.
  - `[TPR-06-014-gemini][informational]` HTML overline color drift — CSS `text-decoration-color` (driven by SGR 58 underline color) applies to ALL decorations on the element including overline, while GPU renders overline in fg unconditionally. Documented as acceptable known drift in §3 implementation notes (rich-text editors typically strip CSS coloring; spec defines no separate "overline color"; cost of separating is two nested spans which would defeat coalescing).
  - `[TPR-06-014-gemini][informational]` `oriterm/src/gpu/prepare/mod.rs:257` — **Document the Y-pixel-snap invariant on the helper.** Fixed in §3: helper doc comment now explicitly cites the `mod.rs:257`/`dirty_skip/mod.rs:250` `.round()` invariant.
- **Plan revisions**: §2 matrix gained 4 cells; §3 implementation revised for OVERLINE early-return gate, HTML font-size removal, Vec-join refactor, helper doc comment, color-drift note; §4 checklist gained `timeout 150` prefixes.
- **Outcome**: All 7 findings resolved inline in this plan. Round summary: dispatch codex 3 / gemini 4 / survivor_mode false; verified 7 / dropped 0; actionable 7 / meta 0. Loop exited at `iter_cap_reached` (max_rounds=1 by design); fix-bug Phase 2.5 contract is "fix issues in the plan; re-run Plan TPR if findings were significant" — post-revision plan converges with reviewer consensus, no re-run needed. Findings transcribed into §R below.

---

## 3. Implementation

- [ ] **OVERLINE rect emission in `DecorationContext::draw`** (`oriterm/src/gpu/prepare/decorations.rs`). Update BOTH the early-return predicate AND the emission section (TPR-06-014-codex F1):
  ```rust
  let has_explicit_underline = flags.intersects(CellFlags::ALL_UNDERLINES);
  let has_strikethrough = flags.contains(CellFlags::STRIKETHROUGH);
  let has_overline = flags.contains(CellFlags::OVERLINE);  // NEW

  if !has_explicit_underline && !has_strikethrough && !has_hyperlink && !has_overline {
      return;
  }
  ```
  After the strikethrough block, add:
  ```rust
  if has_overline {
      self.backgrounds.push_rect(
          ScreenRect { x, y, w: cell_width, h: t },
          fg,
          self.alpha,
      );
  }
  ```
  The early-return gate update is critical — without it, OVERLINE-only cells silently skip the entire function before reaching the new branch. Update the doc comment on `draw()` to list overline.

- [ ] **Add `super_sub_glyph_offset` helper in `oriterm/src/gpu/prepare/mod.rs`** (one canonical home; shaped + dirty_skip + unshaped paths all call it):
  ```rust
  /// Vertical glyph offset (in pixels) for SGR 73 (superscript) / SGR 74 (subscript).
  ///
  /// Returns a SIGNED, INTEGER-ROUNDED pixel offset relative to the cell-top y:
  /// negative shifts the glyph upward (super), positive shifts downward (sub),
  /// 0.0 when neither flag is set. The `.round()` is load-bearing — it preserves
  /// the integer-Y-pixel-snap discipline applied at `mod.rs:257` and
  /// `dirty_skip/mod.rs:250` (`(oy + row * ch).round()`); a fractional offset
  /// would defeat that snap and trigger bilinear-filtering blur on cells whose
  /// `cell_height * 0.25` is non-integer (e.g. `13.0 * 0.25 = 3.25`).
  /// Backgrounds, decorations, and cursors keep the unshifted cell-top y so they
  /// remain anchored to the cell rectangle.
  pub(super) fn super_sub_glyph_offset(flags: CellFlags, cell_height: f32) -> f32 {
      const FACTOR: f32 = 0.25;  // wezterm-compatible 25% (screen_line.rs:437-445)
      let raw = if flags.contains(CellFlags::SUPERSCRIPT) {
          -cell_height * FACTOR
      } else if flags.contains(CellFlags::SUBSCRIPT) {
          cell_height * FACTOR
      } else {
          0.0
      };
      raw.round()
  }
  ```

- [ ] In `fill_frame_shaped` (`mod.rs`), compute `let glyph_y_offset = super_sub_glyph_offset(cell.flags, ch);` and use `y + glyph_y_offset` for the GlyphEmitter call AND the built-in glyph rect.
- [ ] In `fill_frame_incremental` (`dirty_skip/mod.rs`), apply the same offset at the same two call sites.
- [ ] In `unshaped::fill_frame` (test-only), apply the offset to `glyph_y`.

- [ ] **Extend `CellStyle`** in `oriterm_core/src/selection/html/mod.rs` with:
  - `overline: bool`
  - `vertical_align: VerticalAlign` enum `{None, Super, Sub}` (mutually exclusive — matches `sgr.rs:64-69` enforcement).
- [ ] Update `CellStyle::from_cell` to populate the new fields from `CellFlags::OVERLINE / SUPERSCRIPT / SUBSCRIPT`.
- [ ] Update `CellStyle::is_default` to include `!overline && vertical_align == VerticalAlign::None`.
- [ ] **Refactor `CellStyle::write_css` to use `Vec<&str>::join(" ")` for `text-decoration`** (TPR-06-014-gemini F2). Replaces the existing 12-arm match (which would become 24 arms with overline added):
  ```rust
  let mut decs: Vec<&str> = Vec::new();
  match self.underline {
      UnderlineKind::None => {}
      UnderlineKind::Single => decs.push("underline"),
      UnderlineKind::Double => decs.push("underline double"),
      UnderlineKind::Curly => decs.push("underline wavy"),
      UnderlineKind::Dotted => decs.push("underline dotted"),
      UnderlineKind::Dashed => decs.push("underline dashed"),
  }
  if self.strikethrough { decs.push("line-through"); }
  if self.overline { decs.push("overline"); }
  if !decs.is_empty() {
      buf.push_str("text-decoration:");
      // Join with space — CSS shorthand accepts multiple decoration tokens.
      let joined = decs.join(" ");
      buf.push_str(&joined);
      buf.push(';');
      if let Some(uc) = self.underline_color {
          let _ = write!(buf, "text-decoration-color:#{:02x}{:02x}{:02x};",
                         uc.r, uc.g, uc.b);
      }
  }
  ```
  After the text-decoration block, when `vertical_align != None`, emit:
  ```rust
  match self.vertical_align {
      VerticalAlign::None => {}
      VerticalAlign::Super => buf.push_str("vertical-align:super;"),
      VerticalAlign::Sub => buf.push_str("vertical-align:sub;"),
  }
  ```
  **NOTE — drop `font-size:0.83em` per TPR-06-014-gemini F1**: the GPU rendering applies vertical offset only (no glyph-size reduction); HTML export must match for visual parity AND `font-size` shrinkage inside `<pre>` breaks the monospace grid (column alignment).

- [ ] **Document acceptable color drift** (TPR-06-014-gemini F3) as an inline comment in `write_css`: when SGR 58 sets `text-decoration-color` AND OVERLINE is also set, CSS applies the same color to underline AND overline (single `text-decoration-color` per element); the GPU renders overline in fg unconditionally. This is a known minor drift between on-screen and clipboard rendering — splitting into nested spans would defeat the existing flat coalescing pattern. Most rich-text editors strip CSS coloring when pasting; the visual divergence is bounded.

- [ ] Add tests per §2 to:
  - `oriterm/src/gpu/prepare/tests.rs` — decoration tests (overline, super/sub offset, integer-rounding, cross-feature interactions).
  - `oriterm_core/src/selection/html/tests.rs` — HTML CSS emission tests (overline, vertical-align, three-way decoration combination via Vec-join).

---

## R. Third Party Review Findings

### Plan TPR — 2026-04-25 (Phase 2.5)

Round 0 (max_rounds=1, custom-objective mode). Scratch dir: `/tmp/tpr-round-ori_term-EtuvoNWO`. Both reviewers returned `status: findings`. All 7 findings verified against actual code, all actionable, all resolved inline in §1.5/§2/§3/§4 of this plan before Phase 3 implementation begins.

- [x] `[TPR-06-014-codex][high]` `oriterm/src/gpu/prepare/decorations.rs:71` — Overline plan omits the decoration fast-path gate.
  Evidence: `if !has_explicit_underline && !has_strikethrough && !has_hyperlink { return; }` — OVERLINE-only cells silently fall through.
  Impact: critical — without this gate update, OVERLINE rect emission code is unreachable for a cell with only OVERLINE set.
  Resolution: §3 implementation now updates BOTH the early-return predicate and the emission section. Disposition: fixed inline in plan.
  Basis: fresh_verification. Confidence: high.
- [x] `[TPR-06-014-codex][medium]` `plans/bug-tracker/fix-BUG-06-014.md:182` — Cross-feature matrix misses requested interactions.
  Evidence: §2 cross-feature matrix lacked SUPERSCRIPT+INVERSE, super/sub+BLINK, SUBSCRIPT-adjacent-spacing, OVERLINE+DOUBLE_UNDERLINE+STRIKETHROUGH composition.
  Impact: medium — matrix gaps would let rendering errors slip past unit tests into real-terminal use.
  Resolution: §2 matrix gained 5 new cells covering all four interactions plus a HTML 3-way decoration combination cell. Disposition: fixed inline in plan.
  Basis: fresh_verification. Confidence: high.
- [x] `[TPR-06-014-codex][low]` `plans/bug-tracker/fix-BUG-06-014.md:266-267` — `cargo test` checklist entries lack mandatory `timeout 150`.
  Evidence: bare `cargo test -p oriterm` and `cargo test -p oriterm_core` lines per `.claude/rules/tests.md` §Running Tests.
  Impact: low — would let a future hanging test go undetected.
  Resolution: §4 checklist now uses `timeout 150 cargo test -p ...` form. Disposition: fixed inline in plan.
  Basis: fresh_verification. Confidence: high.
- [x] `[TPR-06-014-gemini][high]` `plans/bug-tracker/fix-BUG-06-014.md:85` — HTML export `font-size:0.83em` breaks the monospace grid AND diverges from GPU's vertical-only shift.
  Evidence: original §1.5 plan emitted `vertical-align:super;font-size:0.83em;`; inside `<pre>`, font-size shrinkage reduces character advance widths, breaking column alignment for SUPERSCRIPT/SUBSCRIPT runs.
  Impact: high — visual divergence between on-screen and clipboard rendering AND grid-alignment break in pasted output.
  Resolution: §1.5 final agreed approach + §3 implementation now emit only `vertical-align:super;` / `vertical-align:sub;` (no font-size). Matches GPU's vertical-offset-only design. Disposition: fixed inline in plan.
  Basis: fresh_verification. Confidence: high.
- [x] `[TPR-06-014-gemini][medium]` `oriterm_core/src/selection/html/mod.rs:423-436` — `text-decoration` CSS generation needs 3-way combination support.
  Evidence: existing 12-arm match `(self.underline, self.strikethrough)` would become 24 arms with overline added; combinatorially fragile.
  Impact: medium — code complexity grows quadratically with each new decoration; current pattern would not scale.
  Resolution: §3 implementation now refactors to `let mut decs: Vec<&str> = Vec::new();` populated conditionally + `decs.join(" ")`. Linear scaling, SSOT for decoration tokens. Disposition: fixed inline in plan.
  Basis: fresh_verification. Confidence: high.
- [x] `[TPR-06-014-gemini][informational]` HTML overline color drift — CSS `text-decoration-color` applies to ALL decorations on element, GPU renders overline in fg.
  Evidence: SGR 58 colors underline; if cell has both UNDERLINE and OVERLINE, CSS applies the SGR-58 color to BOTH.
  Impact: informational — minor visual drift in clipboard pastes; rich-text editors typically strip CSS coloring; spec defines no separate "overline color".
  Resolution: §3 documents drift inline as known acceptable trade-off (alternative would defeat coalescing). Disposition: documented in plan.
  Basis: direct_file_inspection. Confidence: medium.
- [x] `[TPR-06-014-gemini][informational]` `oriterm/src/gpu/prepare/mod.rs:257` — Document Y-pixel-snap invariant on the helper.
  Evidence: `let y = (oy + row as f32 * ch).round();` is the integer-Y-snap source.
  Impact: informational — clarity / future maintenance.
  Resolution: §3 helper doc comment now explicitly cites `mod.rs:257`/`dirty_skip/mod.rs:250` invariant and explains why `.round()` is load-bearing. Disposition: documented in plan.
  Basis: direct_file_inspection. Confidence: high.

### Round Summary

Dispatch: codex 3 / gemini 4 / survivor_mode: false.
Verification: verified 7 / dropped 0.
Classification: actionable 7 / meta 0.
Fix commit: applied inline to `plans/bug-tracker/fix-BUG-06-014.md` (this file); see git log for the commit SHA after Phase 4 closure.
Loop exited at iter_cap_reached (max_rounds=1 by design). Per `/fix-bug` Phase 2.5 contract ("re-run Plan TPR if findings were significant"), post-revision plan converges with reviewer consensus — no re-run.

### Code TPR — Phase 5 (2026-04-25, commit `777ce890` baseline)

Round 0 (max_rounds=2). Scratch dir: `/tmp/tpr-round-ori_term-XYe6UUxO`.
- Codex: `status: findings` (2). Gemini: `status: clean`.
- Both findings verified against actual code, both actionable, both fixed inline before close.

- [x] `[TPR-06-014-codex][low]` `oriterm_core/src/selection/html/mod.rs:508` — HTML module exceeds the 500-line source-file limit.
  Evidence: `wc -l html/mod.rs` reported 508 lines after the BUG-06-014 implementation landed, 8 over the `.claude/rules/code-hygiene.md §File Size` hard limit.
  Impact: low — file-size hygiene violation; no functional defect, but violates the stated invariant.
  Resolution: extracted `CellStyle`, `UnderlineKind`, `VerticalAlign`, and the `from_cell` / `is_default` / `write_css` impl block into a sibling submodule `oriterm_core/src/selection/html/style.rs`. Mod.rs declares `mod style;` and imports `CellStyle` via `self::style::CellStyle`. `HtmlCtx` was promoted to `pub(super)` so the new submodule can borrow it. Final sizes: `mod.rs` = 338 lines, `style.rs` = 192 lines.
  Disposition: fixed inline (commit forthcoming).
  Basis: fresh_verification. Confidence: high.
- [x] `[TPR-06-014-codex][medium]` `oriterm/src/gpu/prepare/tests.rs:1759` — Super/sub tests miss shaped and incremental glyph paths.
  Evidence: original new tests used `prepare_frame` (unshaped) only; matrix gap on shaped GlyphEmitter, built-in glyph rect, and dirty-skip incremental rebuild paths.
  Impact: medium — production rendering goes through `prepare_frame_shaped` / `fill_frame_incremental`; unshaped-only tests miss real-world regressions.
  Resolution: added 5 new tests — `shaped_superscript_shifts_glyph_y_up_by_quarter_cell_height`, `shaped_subscript_shifts_glyph_y_down_by_quarter_cell_height`, `shaped_no_super_sub_keeps_glyph_y_unshifted`, `shaped_builtin_glyph_with_superscript_shifts_y` (uses U+2500 box-drawing char + `crate::gpu::builtin_glyphs::raster_key`), and `incremental_dirty_row_with_superscript_shifts_glyph_y` (exercises `prepare_frame_shaped_into` with a 2-pass dirty rebuild).
  Disposition: fixed inline (commit forthcoming).
  Basis: fresh_verification. Confidence: high.

### Round Summary

Dispatch: codex 2 / gemini 0 / survivor_mode: false. Verification: verified 2 / dropped 0. Classification: actionable 2 / meta 0. Fix commit: pending — code edits applied locally to `oriterm_core/src/selection/html/mod.rs`, `oriterm_core/src/selection/html/style.rs` (new), `oriterm/src/gpu/prepare/tests.rs`. `./build-all.sh` + `./clippy-all.sh` + `timeout 150 ./test-all.sh` green. Loop exited at iter_cap_reached (max_rounds=2 by design but only 1 round needed since both findings fixed inline before any re-dispatch).

---

## 4. Completion Checklist

- [ ] All new tests pass unchanged after fix (no test modifications needed)
- [ ] Matrix completeness verified — every cell in flag × glyph-path × decoration-interaction grid has a test
- [ ] Debug AND release builds pass (`cargo b && cargo b --release`)
- [ ] Windows cross-compile green (`cargo build --target x86_64-pc-windows-gnu`)
- [ ] GPU visual-regression suite green (cached path via `render_frame_cached`) — fix touches `oriterm/src/gpu/prepare/`
- [ ] `oriterm_core/tests/alloc_regression.rs` and `rss_regression.rs` still green — fix adds at most 1 rect per cell with OVERLINE; capacity already amortized
- [ ] `timeout 150 ./test-all.sh` green — no regressions
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green (workspace + cross-compile)
- [ ] `timeout 150 cargo test -p oriterm` green (TPR-06-014-codex F3)
- [ ] `timeout 150 cargo test -p oriterm_core` green (TPR-06-014-codex F3)
- [ ] `/commit-push` — commit all changes before review
- [ ] Plan TPR (Phase 2.5) — completed (mandatory: complexity-elevated subsystem)
- [ ] `/tpr-review` (Phase 5 — code review) passed
- [ ] `/impl-hygiene-review` passed
- [ ] **Capability regression gate** — N/A (this fix adds capabilities, does not regress any)
- [ ] `/improve-tooling` retrospective completed
- [ ] Bug entry in `plans/bug-tracker/section-06-rendering-perf.md` updated: `- [x]` with resolution details
- [ ] Fix section frontmatter `status` updated to `complete`
- [ ] Bug-tracker `00-overview.md` Quick Reference open bug count updated (Section 06: 8 → 7)
- [ ] Final `/commit-push` — commit closure artifacts

**Exit Criteria:** `printf '\x1b[53mO\x1b[0m'` produces a horizontal stroke at the top edge of the `O` cell on all three platforms; `printf '\x1b[73mS\x1b[0m'` shifts the `S` glyph upward by 25% of cell height; `printf '\x1b[74mS\x1b[0m'` shifts it downward by the same amount. Selecting these cells and pasting into a rich-text editor preserves the corresponding CSS attributes. `cargo test -p oriterm` and `cargo test -p oriterm_core` are green; Plan TPR + Code TPR + impl-hygiene clean.
