---
section: "08"
title: "Icon Fidelity Verification"
status: in-progress
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-25
goal: "All 8 settings sidebar icons are sourced from the mockup SVGs, lowered into renderer-compatible IconPath data, and verified by raster-fidelity tests at target logical sizes without regressing the shared icon system used by sidebar, tab bar, and window chrome"
inspired_by:
  - "SVG source-of-truth assets"
  - "Raster-based visual regression testing"
depends_on: []
sections:
  - id: "08.1"
    title: "Mockup Source of Truth"
    status: complete
  - id: "08.2"
    title: "Shared Icon Data and Generation"
    status: complete
  - id: "08.3"
    title: "Renderer and Consumer Integration"
    status: complete
  - id: "08.4"
    title: "Raster Fidelity Verification"
    status: complete
  - id: "08.5"
    title: "Tests"
    status: complete
  - id: "08.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "08.7"
    title: "Source-Faithful Stroke Width"
    status: in-progress
  - id: "08.6"
    title: "Build & Verify"
    status: complete
---

# Section 08: Icon Fidelity Verification

## Problem

The original draft correctly noticed that several settings sidebar icons do not match the mockup,
but it targeted the wrong implementation boundary and relied on a brittle review method.

What the tree actually has today:

- [mockups/settings-brutal.html](/home/eric/projects/ori_term/mockups/settings-brutal.html)
  contains the 8 authoritative sidebar SVGs inline at lines 1541-1572.
- [oriterm_ui/src/icons/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/icons/mod.rs)
  is the shared icon registry for the whole UI, not just the settings sidebar.
- [oriterm/src/gpu/icon_rasterizer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/icon_rasterizer/mod.rs)
  is the actual vector-to-alpha rasterization boundary. The draft's reference to
  `oriterm/src/gpu/icon_cache.rs` is stale.
- [oriterm/src/gpu/window_renderer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/mod.rs)
  pre-resolves all current icon atlas entries through `resolve_icons()` and the shared
  `ICON_SIZES` list.
- [oriterm/src/app/settings_overlay/form_builder/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/mod.rs)
  is the real consumer of the 8 sidebar icons.
- [oriterm_ui/src/widgets/tab_bar/widget/draw.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/tab_bar/widget/draw.rs)
  and [oriterm_ui/src/widgets/window_chrome/controls.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/window_chrome/controls.rs)
  confirm that icon changes live in a shared system and must not regress non-sidebar icons.
- [oriterm_ui/src/icons/tests.rs](/home/eric/projects/ori_term/oriterm_ui/src/icons/tests.rs)
  only checks structural invariants, and
  [oriterm/src/gpu/icon_rasterizer/tests.rs](/home/eric/projects/ori_term/oriterm/src/gpu/icon_rasterizer/tests.rs)
  only checks that rasterization is non-empty. There is no fidelity verification today.

The real current-state mismatch is not hypothetical. The sidebar icon set in
[oriterm_ui/src/icons/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/icons/mod.rs) is a mix
of near-matches and deliberate simplifications:

- `Sun` uses an octagon center instead of a real circle and different diagonal ray endpoints
- `Palette` is a circular/octagonal approximation, not the mockup's asymmetric palette silhouette
- `Type` omits the mockup's top bracket shape
- `Keyboard` drops keys from the first and second rows
- `Window` places the title-bar divider too high
- `Bell` is a simplified bell/body sketch, not a conversion of the mockup arcs
- `Activity` uses a different waveform with an extra bend and shifted peak positions

## Corrected Scope

Section 08 should keep the full goal intact: the 8 settings sidebar icons should match the mockup
artwork, not just "look close enough."

The feasible way to do that in this codebase is:

1. treat the mockup SVGs as the authoritative source data
2. lower those SVGs into runtime `IconPath` data at the shared icon boundary
3. verify the final raster output against the source SVGs at the sizes the UI actually renders

This avoids two failure modes in the old draft:

- hand-maintained markdown path transcriptions drifting from the actual code
- editing icon coordinates by eye without any renderer-side proof that the output matches

The current runtime path model already supports `MoveTo`, `LineTo`, `CubicTo`, and `Close`.
That is sufficient for these icons if circles, rounded corners, and SVG arcs are converted into
cubic segments before they reach the checked-in `IconPath` definitions. Section 08 should therefore
improve the source-to-path pipeline rather than widen the GPU/runtime contract prematurely.

---

## 08.1 Mockup Source of Truth

### Goal

Make the settings mockup icons authoritative in a way the code can verify and regenerate, instead
of relying on prose-normalized coordinates in a plan document.

### Files

- [mockups/settings-brutal.html](/home/eric/projects/ori_term/mockups/settings-brutal.html)
- new icon-source fixture module under
  [oriterm_ui/src/icons/](/home/eric/projects/ori_term/oriterm_ui/src/icons)
- optional dev utility under [tools/](/home/eric/projects/ori_term/tools)

### Required Source Fixture

Add a checked-in source fixture that maps the 8 sidebar `IconId`s to their exact SVG snippets from
the mockup:

```rust
pub struct SidebarIconSource {
    pub id: IconId,
    pub logical_size: u32,
    pub svg: &'static str,
}
```

The fixture should store the exact mockup markup for:

- `Sun`
- `Palette`
- `Type`
- `Terminal`
- `Keyboard`
- `Window`
- `Bell`
- `Activity`

This fixture must live outside the HTML mockup file so tests and codegen do not need to scrape
`settings-brutal.html` at runtime.

### Why This Is Needed

This makes the mockup a real source artifact instead of a screenshot reference. Once the SVGs are
checked in as fixture data, Section 08 can:

- generate or validate `IconPath` definitions from the same source every time
- render the source SVGs and the runtime icons side by side in tests
- update icons when the mockup changes without redoing a long markdown transcription

### Checklist

- [x] Add checked-in source fixtures for the 8 sidebar SVGs
- [x] Record their target logical size as `16`
- [x] Keep the fixture data independent from runtime HTML parsing

---

## 08.2 Shared Icon Data and Generation

### Goal

Replace the simplified sidebar icon definitions with mockup-derived runtime paths while keeping the
shared `IconId` / `IconPath` / `ResolvedIcons` contract intact for all existing consumers.

### Files

- [oriterm_ui/src/icons/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/icons/mod.rs)
- new sidebar-specific module under
  [oriterm_ui/src/icons/](/home/eric/projects/ori_term/oriterm_ui/src/icons)
- optional dev/codegen utility under [tools/](/home/eric/projects/ori_term/tools)

### Module Restructure

Split the current monolithic icon file so the source-derived sidebar icons can be reviewed and
maintained independently from tab-bar and window-chrome symbols.

Suggested layout:

- `mod.rs`: `IconId`, `IconPath`, `ResolvedIcon`, `ResolvedIcons`, re-exports
- `chrome.rs`: close/plus/chevron/window chrome icons
- `sidebar_nav.rs`: generated or source-derived settings icons
- `tests.rs`: shared icon tests

That keeps Section 08 focused on the icon set that actually comes from the mockup.

### Generation Boundary

> **MODERATE COMPLEXITY WARNING**: The SVG-to-PathCommand importer must correctly handle SVG arc commands (the `A`/`a` path command), which require non-trivial endpoint-to-center parameterization conversion plus arc-to-cubic-Bezier subdivision. Use a well-tested reference implementation (e.g., the algorithm from the SVG spec Appendix F) rather than writing arc conversion from scratch.

Do not keep editing normalized coordinates by hand. Add a small dev-side importer or codegen step
that converts the checked-in SVG fixtures into `PathCommand` arrays.

That importer must support the SVG subset already present in the mockup:

- `<path>`
- `<line>`
- `<polyline>`
- `<circle>`
- `<rect rx="...">`
- SVG arc commands inside path data

Lowering rules:

- line and polyline segments become `MoveTo` and `LineTo`
- circles become cubic Bezier loops
- rounded-rect corners become cubic Bezier corners
- SVG arc segments become cubic Bezier segments before they enter runtime `IconPath`
- stroke width remains controlled by `IconStyle::Stroke(NAV_STROKE)` at runtime

This keeps the runtime contract simple while still supporting the real geometry in the mockup.

### Runtime Outcome

All 8 sidebar icon definitions should be sourced from the importer/codegen output, not manually
maintained approximations.

That means:

- `Terminal` may still end up looking similar to its current definition, but the committed data
  should come from the source pipeline anyway
- `Sun`, `Palette`, `Type`, `Keyboard`, `Window`, `Bell`, and `Activity` must be replaced with
  source-derived command sequences

### Checklist

- [x] Split sidebar icons into a dedicated module
- [x] Add a source-to-`PathCommand` importer or codegen tool
- [x] Lower circles, rounded rects, and arcs into cubic segments
- [x] Replace all 8 sidebar icon definitions with source-derived data
- [x] Keep `IconId` and `ResolvedIcons` compatible with existing widget code

---

## 08.3 Renderer and Consumer Integration

### Goal

Keep the fidelity work aligned with the real render path and verify it against the actual consumers
that use these icons today.

### Files

- [oriterm/src/gpu/icon_rasterizer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/icon_rasterizer/mod.rs)
- [oriterm/src/gpu/icon_rasterizer/cache.rs](/home/eric/projects/ori_term/oriterm/src/gpu/icon_rasterizer/cache.rs)
- [oriterm/src/gpu/window_renderer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/mod.rs)
- [oriterm/src/app/settings_overlay/form_builder/mod.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/mod.rs)
- [oriterm_ui/src/widgets/sidebar_nav/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/sidebar_nav/mod.rs)

### What Should Change

The main implementation work stays in `oriterm_ui/src/icons`, not in the rasterizer. The renderer
already has the right high-level contract:

- `rasterize_icon()` converts `IconPath` to alpha data
- `IconCache` caches by `(IconId, size_px)`
- `resolve_icons()` pre-rasterizes the shared icon set per frame

Section 08 should keep that boundary, but it should verify these integration details explicitly:

- the sidebar icons still rasterize cleanly at their real logical size (`16`)
- the generated geometry fits within the icon box and is not clipped by antialiasing at 1x or HiDPI
- `WindowRenderer::ICON_SIZES` remains in sync with the actual widget consumers

### Consumer Guardrails

The fidelity rewrite must not silently turn into a sidebar-only fork. Keep the shared registry and
add regression coverage for the currently known consumers:

- settings sidebar: 8 icons at `16px`
- tab bar: close/plus/chevron at `10px`
- window chrome: minimize/maximize/restore/window-close at `10px`

If Section 08 discovers a size sync problem, fix it by moving the authoritative icon-size metadata
closer to the icon registry, not by adding more ad hoc lists.

### Checklist

- [x] Keep fidelity work at the shared icon definition boundary
- [x] Verify sidebar icons remain unclipped at `16px`
- [x] Add a guard against drift between widget consumers and `ICON_SIZES`
- [x] Preserve existing non-sidebar icon consumers unchanged

---

## 08.4 Raster Fidelity Verification

### Goal

Prove that the runtime icon output matches the mockup source at the pixel level, instead of only
comparing command lists or eyeballing screenshots.

### Files

- [oriterm/src/gpu/icon_rasterizer/tests.rs](/home/eric/projects/ori_term/oriterm/src/gpu/icon_rasterizer/tests.rs)
- [oriterm_ui/src/icons/tests.rs](/home/eric/projects/ori_term/oriterm_ui/src/icons/tests.rs)
- source fixtures from Section 08.1

### Verification Strategy

Add a reference rasterization path for the sidebar SVG fixtures, then compare it against the actual
output from `rasterize_icon(id.path(), size_px, scale)`.

Recommended test layers:

1. definition/source equivalence
   - ensure each sidebar `IconId` maps to an SVG fixture
   - ensure each runtime icon definition is generated from or validated against that fixture

2. raster fidelity
   - rasterize the SVG fixture at `16px` logical / `1.0x`
   - rasterize the runtime `IconPath` through the existing `tiny_skia` path
   - compare alpha masks with a strict tolerance

3. HiDPI fidelity
   - repeat at `32px` physical / `2.0x`
   - ensure the thicker physical stroke still matches the source behavior

The comparison should be strong enough to catch the current mismatches, especially:

- missing keyboard keys
- wrong title-bar divider height in `Window`
- incorrect top bracket shape in `Type`
- silhouette drift in `Palette` and `Bell`

### Reference Renderer

Use a dedicated reference renderer for the source SVGs in tests or dev tooling rather than treating
the generated `PathCommand` output as its own oracle. The important property is independence: the
source SVG should be rasterized from source data, and the runtime icon should be rasterized from
the committed `IconPath`.

If exact byte-for-byte equality is not stable because of stroke join/cap implementation details,
use a small documented alpha-diff tolerance and fail on meaningful shape drift.

**Recommended tolerance methodology**: Compare per-pixel alpha values and compute the mean absolute
difference (MAD) across all pixels. A MAD threshold of `2.0` (out of 255) catches shape drift while
tolerating minor antialiasing differences. Additionally, require that no single pixel differs by more
than `15` alpha units, to catch localized geometry errors that MAD might average away. Document the
chosen thresholds in a comment next to the comparison function.

### Checklist

- [x] Add source-SVG rasterization in tests or dev tooling
- [x] Compare source and runtime alpha masks at `16px`
- [x] Repeat comparison at HiDPI (`32px` physical / `2.0x`)
- [x] Set and document a strict diff tolerance
- [x] Cover every sidebar icon, not just one smoke-test icon

---

## 08.5 Tests

### Goal

Turn icon fidelity from an informal review step into repeatable regression coverage that matches the
repository's existing test layout.

### Required Test Coverage

In [oriterm_ui/src/icons/tests.rs](/home/eric/projects/ori_term/oriterm_ui/src/icons/tests.rs):

- keep the existing structural invariants
- add sidebar-fixture completeness tests
- add tests that source-derived sidebar definitions are present for all 8 mockup icons

In [oriterm/src/gpu/icon_rasterizer/tests.rs](/home/eric/projects/ori_term/oriterm/src/gpu/icon_rasterizer/tests.rs):

- keep generic rasterizer tests
- add per-icon raster-fidelity comparisons for the 8 sidebar icons
- add size coverage for `16px @ 1.0x` and `32px @ 2.0x`
- add clipping/bounds assertions so strokes do not get cut off at the icon box edges

Add one integration guard that verifies the currently used `(IconId, logical_size)` pairs stay in
sync with the shared consumer set.

### Explicit Non-Goals

Section 08 does not need to add a generic application-wide screenshot harness. The missing proof is
icon-source fidelity at the icon rasterization boundary, and targeted tests there are enough.

### Checklist

- [x] Add source-to-runtime equivalence tests (compare `icon.path().commands` vs `svg_to_commands(fixture.svg)` for all 8 sidebar icons)
- [x] Expand `oriterm_ui` icon tests with fixture completeness checks
- [x] Expand `oriterm` rasterizer tests with per-icon fidelity checks
- [x] Add HiDPI coverage
- [x] Add clipping/bounds assertions
- [x] Add a consumer-size sync regression test

---

## 08.R Third Party Review Findings

### Open Findings

- [x] `[TPR-08-009][medium]` `oriterm/src/gpu/icon_rasterizer/tests.rs:291` — The Section 08 “source SVG” fidelity path is self-referential and also collapses the mockup’s `stroke-width=”2”` down to the same `1.0px` runtime stroke, so it neither verifies nor preserves the source artwork’s real stroke weight.
  Evidence: The checked-in fixtures record `stroke-width=”2”`, but `svg_to_commands()` only preserves geometry, `rasterize_fixture_svg()` re-rasterizes those commands with hardcoded `SIDEBAR_STROKE = 1.0`, and the runtime icons use the same `NAV_STROKE = 1.0`. At the target `16px` size, the mockup stroke scales to about `1.33px`, so the current tests can pass while both the importer and runtime remain systematically too thin.
  Resolved 2026-03-25: accepted. Concrete implementation tasks added as §08.7 “Source-Faithful Stroke Width.” The mockup SVGs use `stroke-width=”2”` in a 24×24 viewBox; at 16px target the correct stroke is `2.0 × 16/24 = 1.333px`. Both `NAV_STROKE` and `SIDEBAR_STROKE` must be derived from the source spec, and the fidelity test reference path must rasterize at the source stroke width to provide an independent comparison.

- [x] `[TPR-08-008][low]` `plans/ui-css-framework/index.md:112` — The plan index still reports Section 08 as "08.1–08.2 complete, 08.3–08.6 remaining" even though Section 08.3 is already marked complete in the section file.
  Resolved 2026-03-24: accepted and fixed. Updated index.md to "08.1–08.3 complete, 08.4–08.6 remaining".

- [x] `[TPR-08-006][medium]` — Accepted 2026-03-24. Frontmatter updated: 08.1 and 08.2 marked complete, section status set to in-progress.
- [x] `[TPR-08-007][medium]` — Accepted 2026-03-24. Added source-to-runtime equivalence test requirement to 08.5 checklist.

### Resolved Findings

- `TPR-08-001` The draft referenced `oriterm/src/gpu/icon_cache.rs`, but the real implementation
  boundary is [icon_rasterizer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/icon_rasterizer/mod.rs),
  [icon_rasterizer/cache.rs](/home/eric/projects/ori_term/oriterm/src/gpu/icon_rasterizer/cache.rs),
  and [window_renderer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/mod.rs).
- `TPR-08-002` The draft scoped the work too narrowly to sidebar icon coordinates even though
  [icons/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/icons/mod.rs) is a shared icon system
  consumed by the settings sidebar, tab bar, and window chrome.
- `TPR-08-003` The draft's manual markdown SVG transcription is not maintainable as an engineering
  artifact. Section 08 needs checked-in source fixtures plus code/data verification, not prose.
- `TPR-08-004` The draft did not account for the current `PathCommand` boundary. Circles, rounded
  rects, and SVG arcs must be lowered into runtime-supported cubic/line segments before fidelity can
  be verified.
- `TPR-08-005` The existing test suite has no source-fidelity coverage. Structural checks and
  non-empty rasterization are insufficient for a feature whose goal is visual equivalence.

---

## 08.7 Source-Faithful Stroke Width

### Goal

Make the icon fidelity test independent by rasterizing the reference path with the source-specified
stroke width, and derive the runtime `NAV_STROKE` from the mockup spec instead of hardcoding `1.0`.

### Problem

The mockup SVGs specify `stroke-width="2"` in a 24×24 viewBox. At 16px logical size, the correct
stroke width is `2.0 × (16.0 / 24.0) = 1.333px`. Both the runtime icons (`NAV_STROKE = 1.0`) and
the fidelity test reference path (`SIDEBAR_STROKE = 1.0`) use `1.0px`, making the test
self-referential and the shipped icons systematically thinner than the mockup.

### Files

- `oriterm_ui/src/icons/sidebar_nav.rs` — `NAV_STROKE` constant
- `oriterm/src/gpu/icon_rasterizer/tests.rs` — `SIDEBAR_STROKE` constant, `rasterize_fixture_svg()`
- `oriterm_ui/src/icons/sidebar_fixtures.rs` — source metadata (viewBox, stroke-width)

### Checklist

- [x] Add `stroke_width` and `viewbox_size` metadata to `SidebarIconSource` fixtures
- [x] Derive `NAV_STROKE` from source: `source_stroke_width × (target_size / viewbox_size)` = `2.0 × 16.0 / 24.0` ≈ `1.333`
- [x] Update `SIDEBAR_STROKE` in tests — removed entirely; reference path now uses `fixture.scaled_stroke()`
- [x] Fidelity reference path rasterizes with the derived stroke, not a hardcoded value
- [x] All 8 sidebar fidelity tests pass with the updated stroke width
- [x] Chrome icons (close/plus/chevron at 10px) are unaffected (different stroke constant)
- [ ] Visual verification: sidebar icons at correct thickness

---

## 08.6 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Focused Verification

- run the targeted icon tests in `oriterm_ui`
- run the targeted icon rasterizer tests in `oriterm`
- verify the settings sidebar renders the 8 mockup icons at `16px` without clipping
- verify existing tab-bar and window-chrome icons still resolve and render at `10px`

Suggested commands:

```bash
cargo test -p oriterm_ui icons::tests
cargo test -p oriterm icon_rasterizer::tests
```

If Section 08 introduces a dev/codegen tool for the sidebar icon fixtures, include one documented
command in the final implementation notes for regenerating the checked-in icon data.
