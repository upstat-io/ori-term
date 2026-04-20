---
section: "11"
title: "Unicode Subcell Glyphs (incl. octants)"
status: in-progress

reviewed: true
goal: "Drive every catalog row in `catalog/unicode-subcell.md` from `implemented-unverified` to `verified`, and ADD the missing octant implementation (U+1CD00–U+1CDE5, Unicode 16, inside the Symbols for Legacy Computing Supplement block U+1CC00–U+1CEBF) which is currently NOT implemented per Pass 1."
success_criteria:
  - "Top-down spec audit committed at `plans/spec-conformance/audits/section-11-top-down-inventory.md`. Every sequence in the canonical spec source(s) for this stack (Unicode 16 chart PDFs — U+2580–U+259F Block Elements, U+1FB00–U+1FBFF Symbols for Legacy Computing, U+1CC00–U+1CEBF Symbols for Legacy Computing Supplement with octants at U+1CD00–U+1CDE5, U+2800–U+28FF Braille Patterns) maps to a catalog row ID OR carries an explicit `not-targeted` decision with rationale. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file. This is enforced PER `plans/spec-conformance/audits/README.md` lint contract — added by Section 09A as the SSOT for top-down catalog coverage to prevent the bottom-up gap that hid DECRQCRA from the catalog."
  - "Octant block/range citations are normalized across the entire plan: section-11 frontmatter + body, `plans/spec-conformance/audits/section-11-top-down-inventory.md` frontmatter, `plans/spec-conformance/catalog/unicode-subcell.md` USC-LEGACY-OCTANT row, `plans/spec-conformance/index.md`, and `plans/spec-conformance/00-overview.md` all cite the SAME block name (Symbols for Legacy Computing Supplement) and the SAME octant range (U+1CD00–U+1CDE5, 230 codepoints). Drift gate lives in `plans/spec-conformance/audits/README.md §Drift Patterns` as a list of forbidden regex patterns; the section-11 file is excluded from the grep scope because it defines the gate's forbidden-pattern list (it is the authority that must NAME the incorrect spellings, so its own text must not match). The bare strings `1CC00` and `1CEBF` are acceptable inside the legitimate block citation `U+1CC00–U+1CEBF`; the gate only rejects the obsolete incorrect forms."
  - "Canonical octant bitmask-to-position mapping documented: a committed artifact (table or codegen input) defines the mapping of each bit in the 8-bit octant bitmask to its geometric position in the 2×4 sub-cell grid, grounded against the Unicode 16 chart PDF for U+1CD00–U+1CDE5 AND cross-checked against the WezTerm (`~/projects/reference_repos/console_repos/wezterm/wezterm-gui/src/customglyph.rs:317-559`) and Kitty (`~/projects/reference_repos/console_repos/kitty/kitty/decorations.c:979-1024`) octant tables. The implementer in §11.1 drives the codepoint→mask table from this artifact, not from the sextant arithmetic in `oriterm/src/gpu/builtin_glyphs/legacy_computing/mod.rs`."
  - "Every row in `catalog/unicode-subcell.md` is `verified` under Section 11's `owner_section: \"01 (bootstrap), 11 (verification)\"` contract: USC-BLOCKS (U+2580–U+259F, half blocks + quadrants), USC-BOX (U+2500–U+257F, box drawing), USC-BRAILLE (U+2800–U+28FF, braille patterns), USC-LEGACY-SEXTANT (U+1FB00–U+1FB3B, sextants), **USC-LEGACY-OCTANT (U+1CD00–U+1CDE5 — NEW)**. USC-BOX coverage is owned by Section 11 per the catalog row — it is NOT covered by any earlier section."
  - "Octants implemented: `oriterm/src/gpu/builtin_glyphs/legacy_computing/octants.rs` exists with the 8-bit bitmask Canvas implementation driven by the canonical table from §11.0; every U+1CD00–U+1CDE5 codepoint renders the correct shape per the Unicode 16 chart. Octants are wired into the built-in glyph dispatch in `oriterm/src/gpu/builtin_glyphs/mod.rs` AND into the font-shaper skip logic in `oriterm/src/font/mod.rs` so the built-in renderer is selected unconditionally — octants never fall through to font shaping."
  - "Spec_chain visual tests for every subcell glyph family live in `oriterm/src/gpu/visual_regression/spec_chain/glyphs/` (the `oriterm` crate owns rungs 5–8 per `plans/spec-conformance/section-05-golden-lane-determinism.md`; `oriterm_core` does not own GPU-path tests). Tests use `VisualSpecHarness` with `render_frame_cached()` — the production render path — per `.claude/rules/tests.md` and `.claude/rules/oriterm.md`. Goldens match with EXACT per-pixel equality (0-pixel diff); the earlier `exact-or-tiny tolerance` is replaced because any deviation signals a rendering drift that must be investigated, not accepted."
  - "Exhaustive semantic raster coverage: in addition to sparse golden pins (one committed PNG per family), a semantic raster test iterates every codepoint in each family and asserts the resulting bitmask matches the canonical table produced in §11.0 (for octants) or the corresponding canonical bit ordering (for the other families). Ranges exercised: U+2500..=U+257F (box drawing, 128 codepoints), U+2580..=U+259F (block elements — half blocks, eighths, shades, quadrants, 32 codepoints), U+1FB00..=U+1FB3B (sextants, 60 codepoints), U+1CD00..=U+1CDE5 (octants, 230 codepoints), U+2800..=U+28FF (braille, 256 codepoints). This closes the proof gap where a single representative golden can certify a family while a skipped or aliased codepoint is silently wrong."
  - "Internal-renderer-takes-precedence test: as a prerequisite, `VisualSpecHarness::with_config` / `headless_env_with_pinned_software_rasterizer()` (`oriterm/src/gpu/visual_regression/spec_chain/visual_harness.rs`) is extended to accept an injected test-only `FontSet` (see §11.2 for the harness-extension subsection). Given that capability, a spec_chain test configures a test-only font that advertises coverage for a known octant codepoint but renders an obviously-incorrect glyph for it; the test asserts that ori_term produces the correct canvas-rendered shape (i.e., the built-in renderer wins over the font). Same test applied to one representative codepoint per subcell family. Proves SSOT per `.claude/rules/impl-hygiene.md` — ori_term's built-in Canvas glyphs are the source of truth, never the configured system font. The harness extension lands in §11.2 as a prerequisite checkbox; the precedence test depends on it."
  - "Braille/octant adjacency test: a golden test renders a braille codepoint and an octant codepoint in the same row of cells and asserts the resulting image matches pixel-exact. Proves there is no visual or state interference between the two 2×4 renderers despite their geometric similarity."
  - "All existing visual_regression glyph tests pass without modification."
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release."
  - "Section's mission criterion connection: contributes to **Verification chain complete per row**."
inspired_by:
  - "ori_term existing `oriterm/src/gpu/builtin_glyphs/legacy_computing/mod.rs` (sextants, 2×3 grid, 6-bit bitmask) — relevant as a Canvas-dispatch shape reference, NOT as a codepoint→mask template. Octants require a distinct, canonical lookup table; the sextant module derives its mask by arithmetic over a compressed range and that approach is incorrect for the octant codepoint layout."
  - "WezTerm `customglyph.rs:317-559` and Kitty `decorations.c:979-1024` — de-facto cross-stack references for the 230-entry octant codepoint→8-bit-mask mapping. Bit ordering in both is row-major 0..7 over the 2×4 grid."
  - "Unicode chart PDFs — definitive shape reference for every glyph. The shape of every codepoint covered by this section has been stable since its introduction (half blocks / quadrants / braille have been stable for decades; sextants were introduced in Unicode 13; octants were introduced in Unicode 16, September 2024 — the earliest Unicode version this section can claim compatibility with). Canonical sources: `https://www.unicode.org/charts/PDF/U2580.pdf` (Block Elements), `https://www.unicode.org/charts/PDF/U1FB00.pdf` (Symbols for Legacy Computing), `https://www.unicode.org/charts/PDF/U1CC00.pdf` (Symbols for Legacy Computing Supplement — octants at U+1CD00–U+1CDE5), `https://www.unicode.org/charts/PDF/U2800.pdf` (Braille Patterns). These are UNVERSIONED URLs that always serve the current Unicode version — the manifest entry's `sha256` field is what pins a specific snapshot if strict version pinning is required; otherwise the shape-reference use is insensitive to Unicode version. Added as fetch-on-demand entries (`redistributable = false`) to `plans/spec-conformance/specs/manifest.toml` by §11.0 so the manifest-backed fetch flow (per `plans/spec-conformance/00-overview.md §Spec Corpus`) works identically to the other fetch-on-demand specs."
depends_on: ["04", "05", "08", "09A"]
third_party_review:
  status: resolved
  updated: 2026-04-19
  notes: "user-accepted at iter_cap_reached after 3 rounds; 16 findings fixed inline across commits bdaf54e0 + 0e74107d + eaa97a00; 0 outstanding findings — the plan state is fix-clean but a formal reviewer clean-return could not be produced within the 3-round cap"
sections:
  - id: "11.0"
    title: "Top-down spec audit (BLOCKING)"
    status: complete
  - id: "11.1"
    title: "Implement octants U+1CD00–U+1CDE5"
    status: complete
  - id: "11.2"
    title: "Spec_chain golden tests for every subcell glyph family"
    status: not-started
  - id: "11.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "11.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 11: Unicode Subcell Glyphs

**Status:** Not Started
**Goal:** Verify every subcell glyph catalog row + add the missing octant implementation. Octants are required by notcurses `keller`/`uniblock` blitter exhaustive tests (section 24 / 21 milestones).

**Success Criteria:** see frontmatter.

**Context:** Pass 1 confirmed half blocks, quadrants, sextants, braille all exist; octants NOT implemented. The existing `oriterm/src/gpu/builtin_glyphs/legacy_computing/mod.rs` implements sextants via a 2×3 grid + 6-bit bitmask Canvas pattern, but its codepoint→mask derivation is arithmetic over a compressed range (`bits = idx + idx / 0x14 + 1`) — this shape is NOT a generalizable template for octants. Octants require a canonical 230-entry lookup table keyed by U+1CD00..=U+1CDE5 (Unicode 16, inside the Symbols for Legacy Computing Supplement block U+1CC00–U+1CEBF). WezTerm and Kitty both carry explicit octant tables; ori_term follows that pattern rather than the sextant arithmetic.

**Reference implementations:**
- ori_term existing `oriterm/src/gpu/builtin_glyphs/legacy_computing/mod.rs` (sextants) — Canvas dispatch shape reference only; do NOT extrapolate its codepoint→mask arithmetic to octants.
- ori_term existing `oriterm/src/gpu/builtin_glyphs/blocks.rs` (half blocks + quadrants U+2500–U+259F).
- ori_term existing `oriterm/src/gpu/builtin_glyphs/braille.rs` (braille 2×4 dot grid, Unicode-dot-order bit numbering — distinct from octant bit order; see §11.1).
- WezTerm `~/projects/reference_repos/console_repos/wezterm/wezterm-gui/src/customglyph.rs:317-559` — canonical 230-entry octant codepoint→8-bit-mask table, row-major bit order.
- Kitty `~/projects/reference_repos/console_repos/kitty/kitty/decorations.c:979-1024` — octant index remap table (232 entries).
- Unicode 16 chart PDFs — fetch-on-demand via `plans/spec-conformance/specs/manifest.toml` (added by §11.0). Canonical URLs: `https://www.unicode.org/charts/PDF/U2580.pdf`, `https://www.unicode.org/charts/PDF/U1FB00.pdf`, `https://www.unicode.org/charts/PDF/U1CC00.pdf`, `https://www.unicode.org/charts/PDF/U2800.pdf`.

**Depends on:**
- Section 04 (delivers `CoreSpecHarness` + `VisualSpecHarness` at `oriterm/src/gpu/visual_regression/spec_chain/visual_harness.rs` + the frozen catalog row schema; §11.2's spec_chain tests call `VisualSpecHarness` directly and rely on the schema freeze for their catalog updates).
- Section 05 (deterministic GPU env for goldens — `headless_env_with_pinned_software_rasterizer` + `GoldenLaneConfig::SPEC_DEFAULT`).
- Section 08 (baseline correct so glyph rendering tests aren't fighting through baseline bugs).
- Section 09A (introduces the `plans/spec-conformance/audits/` SSOT + `spec-coverage-report --check audit-files` tooling that §11.0 requires; §11.0 CANNOT start until 09A lands).

---

## 11.0 Top-down spec audit (BLOCKING — precedes all other subsections)

**Goal:** Walk the canonical spec source(s) for this stack TOP-DOWN. Every sequence the spec defines gets a row in this section's audit file at `plans/spec-conformance/audits/section-11-top-down-inventory.md`, mapped to either an existing catalog row ID or an explicit `not-targeted` decision with rationale.

**Why this exists:** Section 09A introduced the `audits/` SSOT to close the bottom-up catalog construction gap that hid DECRQCRA (and the entire DEC private rectangular-ops family) from the catalog. The original Section 01 catalog bootstrap was bottom-up (audit existing dispatch + add tack/teseq-discovered items), which is incomplete by construction — sequences absent from both the catalog AND the test corpus are invisible. The per-section audit file makes top-down coverage mechanically lintable: `spec-coverage-report --check audit-files` fails CI if any audit-file mapping does not resolve to a real catalog row.

**Canonical spec source(s):** Unicode chart PDFs (shape-reference use; Unicode version tracked by the manifest entry's `sha256` for strict pinning, otherwise insensitive) — U+2580–U+259F Block Elements (half-blocks, quadrants), U+1FB00–U+1FBFF Symbols for Legacy Computing (sextants at U+1FB00–U+1FB3B + additional subcell glyphs), U+1CC00–U+1CEBF Symbols for Legacy Computing Supplement (octants at U+1CD00–U+1CDE5, introduced in Unicode 16, September 2024), U+2800–U+28FF Braille Patterns.

**Files touched:**
- `plans/spec-conformance/audits/section-11-top-down-inventory.md` (NEW — stub created by Section 09A's §09A.10; populated by this subsection).
- `plans/spec-conformance/catalog/unicode-subcell.md` (open new rows for any sequences that should be `mapped` but aren't catalogued yet — use the canonical schema per `plans/spec-conformance/00-overview.md §Catalog Row Schema`).
- `plans/spec-conformance/specs/manifest.toml` (add Unicode 16 chart PDF entries — see manifest-update completion criterion below).
- `plans/spec-conformance/00-overview.md §Spec Corpus` (update the directory-listing block to name the new manifest-backed chart entries rather than any obsolete local-PDF reference — see overview-sync completion criterion below).
- Any file in `plans/spec-conformance/` whose octant block/range citation drifts from the canonical supplement block name and U+1CD00–U+1CDE5 range (see drift-normalization completion criterion below).

**Completion criteria:**

- [x] Audit file `plans/spec-conformance/audits/section-11-top-down-inventory.md` is populated with every sequence in the canonical spec source(s).
- [x] Every row in the audit-file table has a `Decision` of `mapped` (cites a catalog row ID) or `not-targeted` (with one-line rationale).
- [x] Every `mapped` row resolves to a real catalog row that exists in `plans/spec-conformance/catalog/`.
- [x] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file.
- [x] Audit file `last_walked` frontmatter is set to today's date and `walked_by` to the implementer's handle.
- [x] Any new catalog rows opened in this subsection use the canonical 10-column schema from `plans/spec-conformance/00-overview.md §Catalog Row Schema`. (No new rows opened — USC-LEGACY-OCTANT already committed by pre-review pass; `Implementation` column will be updated by §11.1 when `octants.rs` lands.)
- [x] **Drift normalization across plan artifacts:** drift-gate grep per `plans/spec-conformance/audits/README.md §Drift Patterns` returns zero matches across `plans/spec-conformance/**` with the README + section-11 exclusions in place.
- [x] **Canonical octant bitmask-to-position mapping artifact committed:** `plans/spec-conformance/specs/octant-bitmask-mapping.md` committed with all 230 rows; cross-checked against WezTerm `customglyph.rs:317-560` and Kitty `decorations.c:979-1026` — 0 discrepancies after normalizing Kitty's column-major encoding to row-major canonical. §11.1 drives `octants.rs` from this artifact.
- [x] **Manifest update:** four Unicode 16 chart PDF entries (`unicode_chart_u2580`, `unicode_chart_u1fb00`, `unicode_chart_u1cc00`, `unicode_chart_u2800`) added to `plans/spec-conformance/specs/manifest.toml` with `redistributable = false` and the conventional fields.
- [x] **Overview sync:** `plans/spec-conformance/00-overview.md §Spec Corpus` directory listing already carries the four `unicode_chart_*.pdf` entries; no reference to the obsolete `unicode-symbols-legacy.pdf` path exists in the overview (verified by grep).

**No other subsection in this section can begin work until §11.0 is complete.** This is a hard gate.

---

## 11.1 Implement octants U+1CD00–U+1CDE5

**File(s):** `oriterm/src/gpu/builtin_glyphs/legacy_computing/octants.rs` (new — submodule of `legacy_computing`), `oriterm/src/gpu/builtin_glyphs/legacy_computing/mod.rs` (extend the existing `pub(in crate::gpu::builtin_glyphs) fn draw(canvas: &mut Canvas, ch: char) -> bool` match to cover the octant range via a new sub-match arm), `oriterm/src/gpu/builtin_glyphs/mod.rs` (extend the existing `pub(crate) fn rasterize(ch: char, cell_w: u32, cell_h: u32) -> Option<RasterizedGlyph>` match arm list to include `'\u{1CD00}'..='\u{1CDE5}' => legacy_computing::draw(&mut canvas, ch)`), `oriterm/src/font/mod.rs` (extend the existing `pub(crate) fn is_builtin(ch: char) -> bool` range match to include `'\u{1CD00}'..='\u{1CDE5}'` alongside the existing `'\u{1FB00}'..='\u{1FB9F}'`), sibling `oriterm/src/gpu/builtin_glyphs/legacy_computing/tests.rs` (create if absent; add `#[cfg(test)] mod tests;` at the bottom of `legacy_computing/mod.rs` per `.claude/rules/test-organization.md §Sibling tests.rs Pattern`).

**API surface note (corrects an earlier plan-draft miscitation):** the built-in-glyph dispatch surface does NOT have separate `is_builtin_glyph` + `render_builtin_glyph` entrypoints. The real API is: (a) `oriterm/src/font/mod.rs::is_builtin(ch) -> bool` is the predicate the font shaper consults before deciding whether to skip font shaping; (b) `oriterm/src/gpu/builtin_glyphs/mod.rs::rasterize(ch, cell_w, cell_h) -> Option<RasterizedGlyph>` dispatches to the correct submodule and returns `Some(..)` when handled; (c) each submodule (like `legacy_computing`) exposes a single `pub(in crate::gpu::builtin_glyphs) fn draw(canvas: &mut Canvas, ch: char) -> bool` entry. §11.1 extends existing match arms in those three sites — it does NOT introduce new entrypoint names.

- [x] Create `oriterm/src/gpu/builtin_glyphs/legacy_computing/octants.rs` driven by the canonical bitmask table artifact produced in §11.0. Table is a `const [u8; 230]` indexed by `ch - OCTANT_START`; the `draw()` function applies the row-major bit decoding over a 2×4 Canvas grid.
- [x] Added `#[cfg(test)] mod tests;` (semicolon form) at the bottom of `legacy_computing/mod.rs`; tests live in `legacy_computing/tests.rs` per `.claude/rules/test-organization.md`.
- [x] Canonical-mapping guard test (`octants_table_matches_canonical_artifact`): parses the canonical artifact markdown at test time and asserts every one of the 230 table entries is byte-identical.
- [x] Braille-vs-octant rendering-model check (`braille_and_octant_rendering_models_are_distinct`): asserts the two renderers produce different pixel buffers for the same nominal bit value — proving the bit-order semantics are distinct, which is the invariant that blocks inadvertent Canvas-helper DRY between the two 2×4 modules.
- [x] Extended `legacy_computing::draw` match arm with `octants::OCTANT_START..=octants::OCTANT_END => octants::draw(canvas, ch)`. Additive with the existing `U+1FB00..=U+1FB9F` arms.
- [x] Extended the `rasterize` match in `oriterm/src/gpu/builtin_glyphs/mod.rs` with `'\u{1CD00}'..='\u{1CDE5}'` joined into the existing `legacy_computing::draw` arm.
- [x] Extended `is_builtin` in `oriterm/src/font/mod.rs` with `'\u{1CD00}'..='\u{1CDE5}'` alongside the existing sextant range — the shaper never handles octants.
- [x] Sibling tests: representative render checks (`octant_u1cd00_renders_upper_mid_left_cell`, `octant_u1cde5_renders_all_but_top_left`) verify specific pixels match the canonical mask semantics; full-coverage test (`every_octant_codepoint_is_builtin_and_rasterizes`) iterates all 230 codepoints.
- [x] **Validation**: all 9 octant tests pass (`cargo test -p oriterm --lib legacy_computing`); full workspace test suite green (`cargo test --workspace`); clippy + build + test all green.

---

## 11.2 Spec_chain visual tests for every subcell glyph family

**File(s):** `oriterm/src/gpu/visual_regression/spec_chain/glyphs/subcell.rs` (new — visual rungs 5–8 live in the `oriterm` crate per Section 05's crate-boundary rule; `oriterm_core` does not own GPU-path tests), goldens in `oriterm/src/gpu/visual_regression/references/spec_chain/glyphs/` (or the path `VisualSpecHarness` resolves to — follow the convention established in `oriterm/src/gpu/visual_regression/spec_chain/visual_harness.rs` and the existing `pilots/sixel_minimal.rs`); harness extensions in `oriterm/src/gpu/visual_regression/spec_chain/visual_harness.rs` (`VisualSpecHarness::with_config`) and `oriterm/src/gpu/visual_regression/mod.rs` (`headless_env_with_pinned_software_rasterizer`).

**Why here and not in `oriterm_core`:** `VisualSpecHarness` owns rungs 5–8 (FrameInput → GPU instance → texture render → golden image) and requires a GPU context — it cannot run inside `oriterm_core`. Placing the test in `oriterm_core/tests/spec_chain/` was a GAP in the earlier plan draft; crate-boundary rules in `.claude/rules/crate-boundaries.md` and `.claude/rules/oriterm.md` pin the test site to `oriterm`.

- [ ] **Prerequisite — harness font-injection extension:** extend `VisualSpecHarness::with_config` (and/or the underlying `headless_env_with_pinned_software_rasterizer` helper) to accept an optional test-only `FontSet` override. Today the harness only exposes `new() / with_size(lines, cols) / with_config(GoldenLaneConfig)` and has no hook for injecting a specific font — the font-precedence test cannot run without this capability. The extension should add a new field to `GoldenLaneConfig` (or a dedicated `HarnessFontOverride` struct) plus a builder method, wired into the existing deterministic-lane font setup in §05. This work is IN SCOPE for §11.2 — the precedence test below is its first consumer.
- [ ] **Prerequisite — test font asset:** generate and commit a test-only TrueType font at `crates/oriterm_test_support/src/fixtures/fonts/subcell-precedence-test.ttf` (or the equivalent fixture path established by Section 05). The font must advertise coverage of at least one codepoint from each subcell family (box drawing, half blocks, quadrants, sextants, octants, braille) and render an obviously-incorrect glyph for each one (e.g., a filled square, an offset cross, a mirrored shape). Without this asset the font-precedence test below has nothing to inject into the harness. Generation can use `fonttools` (Python) with a tiny glyph-table definition, or a hand-crafted TTF stored as a binary fixture — the implementer picks whichever is deterministic and reproducible.
- [ ] **Sparse golden pins (one per family):** for each glyph family (box drawing, half blocks, quadrants, sextants, octants, braille), pick one representative codepoint that exercises a non-trivial bitmask and commit an exact golden PNG at the canonical 12pt @ 96 DPI cell metrics. Representative examples: box `╬` U+256C (BOX DRAWINGS DOUBLE VERTICAL AND HORIZONTAL), full block `█` U+2588 (all 4 quadrant bits = 0x0F), quadrant `▟` U+259F (QUADRANT UPPER RIGHT AND LOWER LEFT AND LOWER RIGHT — 3 of 4 bits set, not 4), sextant with all 6 bits set, octant with all 8 bits set (U+1CDE5 end of range), braille `⣿` U+28FF (all 8 dots).
- [ ] **Render path:** the test invokes `render_frame_cached()` via `VisualSpecHarness` — the production render path — per `.claude/rules/tests.md §GPU cached render testing` and `.claude/rules/oriterm.md §GPU Render Path Testing`. Never use a simplified test-only render path; it would mask production bugs.
- [ ] **Exact pixel tolerance:** goldens are compared with 0-pixel diff (`assert_eq!(actual, expected)` on the pixel buffer). The earlier "exact-or-tiny tolerance" phrasing is removed — any deviation signals a rendering drift that must be investigated, not accepted. The deterministic golden lane from Section 05 (pinned llvmpipe, grayscale alpha hinting, pinned cell metrics) makes exact-match feasible.
- [ ] **Exhaustive semantic raster coverage:** a companion test iterates every codepoint in each family and asserts the rendered Canvas bitmask matches the canonical mapping from §11.0 (for octants) or the established Unicode bit order (for braille) or the existing `blocks.rs` / `box_drawing.rs` / `legacy_computing/mod.rs` table (for half blocks / quadrants / box drawing / sextants). Runs every codepoint in: U+2500..=U+257F (box drawing, 128 codepoints), U+2580..=U+259F (block elements including half blocks + eighths + shades + quadrants, 32 codepoints), U+1FB00..=U+1FB3B (sextants, 60 codepoints), U+1CD00..=U+1CDE5 (octants, 230 codepoints), U+2800..=U+28FF (braille, 256 codepoints). Proves coverage beyond the sparse golden pins.
- [ ] **Internal-renderer-takes-precedence test** (depends on the harness font-injection extension above): configures a test-only font that advertises coverage of a known octant codepoint (e.g., U+1CD00) but renders an obviously-incorrect glyph for it (a filled square, say); asserts the resulting frame matches the correct canvas-rendered golden. Repeat for one representative codepoint per subcell family (box drawing, half blocks, quadrants, sextants, octants, braille). Proves `ori_term`'s built-in Canvas glyphs win over the configured system font — the SSOT requirement from `.claude/rules/impl-hygiene.md`.
- [ ] **Braille/octant adjacency test:** render a braille codepoint and an octant codepoint in the same row of cells (e.g., `U+28FF U+1CD7F`) and compare against an exact golden. Proves there is no visual or stateful interference between the two 2×4 renderers despite their geometric similarity.
- [ ] Update catalog rows in `catalog/unicode-subcell.md` to `verified`: USC-BLOCKS, USC-BOX, USC-BRAILLE, USC-LEGACY-SEXTANT, USC-LEGACY-OCTANT. USC-BOX is owned by Section 11 per the catalog's `owner_section: "01 (bootstrap), 11 (verification)"` contract — this section owns the verification rung for box drawing as well.
- [ ] **Validation**: every family's sparse golden test passes with 0-pixel diff; the exhaustive raster sweep passes for every codepoint in scope (including all 128 box-drawing codepoints); the font-precedence test passes for every family (including box drawing); the braille/octant adjacency test passes; back-to-back runs produce 0-pixel diff.

---

## 11.R Third Party Review Findings

/tpr-review ran 3 rounds on this section (2026-04-19). 16 verified findings across all rounds were fixed and committed inline; zero remain outstanding. The review exited at `iter_cap_reached` with `user-accepted` disposition per §5 of `.claude/skills/tpr-review/SKILL.md`.

| Round | Codex findings | Gemini findings | Agreements | Actionable | Fix commit |
|---|---|---|---|---|---|
| 0 | 6 | 4 | 3 | 7 unique | `bdaf54e0` |
| 1 | 4 | 3 | 1 | 6 unique | `0e74107d` |
| 2 | 3 | 2 (1 dropped — hallucinated evidence) | 1 | 3 unique | `eaa97a00` |

**Outstanding findings:** None. All 16 unique verified findings were fixed and committed in the round they surfaced.

**Notable fixes applied:**

- Range / block-name drift normalized across section-11, catalog, audit stub, index.md, and 00-overview.md (octant range corrected to U+1CD00–U+1CDE5 inside the Symbols for Legacy Computing Supplement block U+1CC00–U+1CEBF; obsolete "Znamenny Musical Notation" and "U+1CC00..U+1CEFF" citations removed; "USC-LEGACY-QUADRANT" renamed to "USC-LEGACY-SEXTANT").
- Dispatch-API citations corrected to match real ori_term source (`font::is_builtin`, `builtin_glyphs::rasterize`, `legacy_computing::draw`).
- Test-site location corrected from `oriterm_core` to `oriterm` per crate-boundary rules.
- `depends_on` expanded to `["04", "05", "08", "09A"]`.
- Canonical octant bitmask-to-position mapping artifact added as §11.0 deliverable.
- Harness font-injection extension added as §11.2 prerequisite.
- Test font asset creation added as §11.2 prerequisite.
- Unicode chart PDFs switched to manifest-backed fetch-on-demand (no local PDF reference).
- Drift-gate forbidden-pattern list moved to `plans/spec-conformance/audits/README.md §Drift Patterns` (SSOT); grep scope excludes both authority files.
- Exhaustive semantic raster coverage per-codepoint added alongside sparse golden pins.
- Font-precedence test added.
- Braille/octant adjacency test added.
- USC-BOX ownership acknowledged (Section 11 verifies box drawing per the catalog's `owner_section` contract).

**Dropped at verification:** one round-2 gemini finding (claimed a stale line-number citation for `geometric_shapes::draw()` / `U+25A0..=U+25FF` that does not appear in the section file — hallucinated per /tpr-review §4 LOWER-trust verification).

---

## 11.N Completion Checklist

- [ ] §11.0 audit populated + octant block/range drift normalized across `plans/spec-conformance/` + canonical octant bitmask mapping artifact committed + Unicode chart PDF entries added to `plans/spec-conformance/specs/manifest.toml` (see §11.0 + §11.2 prerequisites).
- [ ] Failing test matrix written FIRST.
- [ ] **Matrix dimensions**: glyph family (box / half / quad / sext / oct / braille) × {sparse-golden, exhaustive-raster, font-precedence, adjacency} × apex (visual golden for golden-pin / semantic-bitmask for raster / visual for font-precedence + adjacency).
- [ ] **Semantic pins**: octant canonical-mapping guard test + braille-vs-octant rendering-model check + font-precedence test (per family, including box drawing) + braille/octant adjacency test are each a distinct regression guard; removing any one of them must cause a test failure in the crate it lives in.
- [ ] `VisualSpecHarness` font-injection extension landed in `oriterm/src/gpu/visual_regression/spec_chain/visual_harness.rs` — the font-precedence test would be unimplementable without it.
- [ ] Octants U+1CD00–U+1CDE5 implemented in `oriterm/src/gpu/builtin_glyphs/legacy_computing/octants.rs` driven by the canonical §11.0 artifact.
- [ ] Octants wired into the `builtin_glyphs::rasterize` match arm list (`oriterm/src/gpu/builtin_glyphs/mod.rs:60-69`) AND `font::is_builtin` range match (`oriterm/src/font/mod.rs:485-496`) — built-in renderer is unconditionally selected for U+1CD00..=U+1CDE5; font shaper never handles those codepoints.
- [ ] Every subcell glyph family has: (a) a sparse golden pin, (b) an exhaustive semantic raster sweep over every codepoint, (c) a font-precedence test, (d) the braille/octant adjacency test covers both 2×4 families.
- [ ] All goldens assert exact 0-pixel diff (no "tiny tolerance" fallback).
- [ ] Visual tests live in `oriterm/src/gpu/visual_regression/spec_chain/glyphs/` (the `oriterm` crate), not in `oriterm_core` — crate-boundary correctness per Section 05 + `.claude/rules/crate-boundaries.md`.
- [ ] Visual tests use `render_frame_cached()` via `VisualSpecHarness` — the production render path.
- [ ] Tests in `legacy_computing/tests.rs` are wired from `legacy_computing/mod.rs` via `#[cfg(test)] mod tests;` (semicolon form, no inline body) per `.claude/rules/test-organization.md §Sibling tests.rs Pattern`.
- [ ] Catalog rows in `catalog/unicode-subcell.md` all `verified`: USC-BLOCKS, USC-BOX, USC-BRAILLE, USC-LEGACY-SEXTANT, USC-LEGACY-OCTANT.
- [ ] All existing visual_regression glyph tests pass without modification.
- [ ] Alloc regression unchanged.
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release.
- [ ] Plan annotation cleanup.
- [ ] Section frontmatter `status` → `complete`.
- [ ] `00-overview.md` Quick Reference + mission criteria updated.
- [ ] `index.md` section 11 status updated.
- [ ] `/tpr-review` passed.
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean).

**Exit Criteria:** Every Unicode subcell glyph catalog row is `verified` (including USC-BOX); octants implemented and exhaustively tested (sparse goldens + per-codepoint semantic raster + font-precedence + braille/octant adjacency); octant block/range citations are normalized across every plan-file; `VisualSpecHarness` supports font injection; Unicode chart PDFs are registered in `specs/manifest.toml`; the built-in renderer is canonical and the font shaper never handles U+1CD00..=U+1CDE5.
