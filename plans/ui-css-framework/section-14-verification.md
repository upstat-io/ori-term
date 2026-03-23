---
section: "14"
title: "Verification + Visual Regression"
status: not-started
reviewed: true
third_party_review:
  status: none
  updated: null
goal: "All CSS framework features are implemented, tested, and verified against the mockup — the settings dialog is visually indistinguishable from settings-brutal.html at 100% DPI"
depends_on: ["01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13"]
sections:
  - id: "14.1"
    title: "Test Matrix"
    status: not-started
  - id: "14.2"
    title: "Visual Comparison"
    status: not-started
  - id: "14.3"
    title: "Cross-Platform Build"
    status: not-started
  - id: "14.4"
    title: "Performance Validation"
    status: not-started
  - id: "14.5"
    title: "Completion Checklist"
    status: not-started
---

# Section 14: Verification + Visual Regression

**Status:** Not Started
**Goal:** Final verification pass. Every CSS framework feature from Sections 01-13 works correctly, passes tests, matches the mockup, and does not regress performance. This section does not implement new features — it verifies and documents.

**Production code paths:** All files modified in Sections 01-13. No new production code in this section.

**Observable change:** Clean builds, all tests green, visual match confirmed, performance validated. The plan is marked complete.

---

## 14.1 Test Matrix

Every CSS framework feature implemented in Sections 01-13 must have test coverage. This matrix catalogs the expected tests and their locations.

### Section 01 — Multi-Size Font Rendering

| Test | Location | What it verifies |
|------|----------|------------------|
| Title renders larger than body | `oriterm_ui` harness test | `TextStyle.size = 18` produces wider shaped text than `size = 13` |
| Glyph atlas accepts multiple sizes | `oriterm/src/font/` unit test | Atlas lookup with different `ppem` values returns distinct raster keys |
| Small text (10px) readable | Visual check | Section titles at 10px are legible, not blurred or aliased |

### Section 02 — Font Weight

| Test | Location | What it verifies |
|------|----------|------------------|
| Bold visibly heavier | `oriterm_ui` harness test | Shaped text with `FontWeight::Bold` has different glyph IDs or metrics than `Regular` |
| Medium weight (500) resolves | `oriterm_ui` or font unit test | `FontWeight::MEDIUM` selects the correct face (or synthesizes bold for 500+ from 400) |
| Weight fallback | Unit test | If font has no 500 face, nearest (400 or 700) is selected without panic |

### Section 03 — Text Transform + Letter Spacing

| Test | Location | What it verifies |
|------|----------|------------------|
| Uppercase applied | `oriterm_ui` harness test | `TextTransform::Uppercase` on "hello" produces "HELLO" in shaped output |
| Letter spacing widens text | `oriterm_ui` harness test | Shaped text with `letter_spacing = 2.0` has greater total width than `letter_spacing = 0.0` |
| Zero spacing is default | Unit test | `TextStyle::default().letter_spacing == 0.0` |

### Section 04 — Line Height

| Test | Location | What it verifies |
|------|----------|------------------|
| Line height multiplier | `oriterm_ui` harness test | `line_height = 1.5` at `size = 12` produces shaped text height ~18px (12 * 1.5) |
| Default line height | Unit test | `TextStyle::default().line_height` is `None` (use font metrics) |

### Section 05 — Per-Side Borders

| Test | Location | What it verifies |
|------|----------|------------------|
| Right-only border | `oriterm_ui` harness test | Widget with `border_right: 2px` draws exactly one 2px line on right edge |
| Top-only border | Harness test | Footer separator draws only on top edge |
| Left-only border | Harness test | Active nav item draws 3px border-left only |

### Section 06 — Opacity + Display

| Test | Location | What it verifies |
|------|----------|------------------|
| Opacity modulates alpha | `oriterm_ui` harness test | Widget with `opacity: 0.5` draws with half-alpha colors |
| Disabled controls dimmed | Harness test | `slider.with_disabled(true)` renders with disabled colors |

### Section 07 — Scrollbar

| Test | Location | What it verifies |
|------|----------|------------------|
| Scrollbar width 6px | Harness test or unit test | `ScrollbarStyle.width == 6.0` |
| Thumb hover state | Harness test | Scrollbar thumb changes color on hover |

### Section 08 — Icons

| Test | Location | What it verifies |
|------|----------|------------------|
| All 8 sidebar icons resolve | Unit test | `IconId::Sun` through `IconId::Activity` all return `Some(resolved)` from icon cache |
| Icon size 16px | Unit test | Resolved icon rect is 16x16 |

### Section 09 — Settings Content

| Test | Location | What it verifies |
|------|----------|------------------|
| Decorations section exists | `oriterm_ui` or integration test | Page has 3 sections (Theme, Window, Decorations) |
| New settings read from Config | Unit test | `build_page()` with Config containing `unfocused_opacity: 0.8` creates slider with value 80 |
| New settings save to Config | Integration test | Save handler writes new field values back to Config |

### Section 10 — Sidebar

| Test | Location | What it verifies |
|------|----------|------------------|
| Nav item padding | Harness test | Item icon starts at correct X offset (16px from sidebar edge) |
| Active state colors | Harness test | Active item has accent bg and accent text color |
| Footer border-top | Harness test | Footer area has 2px border at top |

### Section 11 — Content Typography

| Test | Location | What it verifies |
|------|----------|------------------|
| Page header at 18px | Harness test | Title text shaped at size 18 |
| Section divider extends | Harness test | Separator widget fills remaining width |
| Bottom padding 28px | Harness test | Content body has 28px bottom padding |
| Section gap 28px | Harness test | Container gap between sections is 28px |

### Section 12 — Footer + Buttons

| Test | Location | What it verifies |
|------|----------|------------------|
| Button font weight | Unit test | `ButtonStyle` has `font_weight: FontWeight::MEDIUM` for footer buttons |
| Button letter spacing | Unit test | `ButtonStyle` has `letter_spacing: 0.48` for footer buttons |
| Reset left, Save right | Harness test | Footer children order: reset, spacer, cancel, gap, save |
| UNSAVED indicator no overlap | Harness test | Indicator X > reset button right edge |

### Section 13 — Widget Controls

| Test | Location | What it verifies |
|------|----------|------------------|
| Toggle thumb 12x12 | Unit test | `thumb_size = height - 2 * thumb_padding = 20 - 8 = 12` |
| Toggle travel 18px | Unit test | `travel = width - 2 * thumb_padding - thumb_size = 38 - 8 - 12 = 18` |
| Slider track 120px | Unit test | `SliderStyle::from_theme().width == 120.0` |
| Row gap 0 | Unit test | `ROW_GAP == 0.0` |

### Checklist

- [ ] All unit tests listed above exist and pass.
- [ ] All harness tests listed above exist and pass.
- [ ] `cargo test -p oriterm_ui` green.
- [ ] `cargo test -p oriterm` green.
- [ ] `./test-all.sh` green.

---

## 14.2 Visual Comparison

Side-by-side screenshot comparison of the running settings dialog against `mockups/settings-brutal.html` at 100% DPI (no scaling).

### Comparison points

**Sidebar area:**

| Element | Expected | Check |
|---------|----------|-------|
| Background color | `#0e0e12` (--bg-base) | [ ] |
| Right border | 2px `#2a2a36` | [ ] |
| Width | 200px | [ ] |
| Search field | 28px height, 2px border, `#16161c` bg | [ ] |
| Search placeholder | 12px, `#8c8ca0`, left offset 26px | [ ] |
| Section titles | 10px, Regular, uppercase, `// PREFIX`, `#8c8ca0` | [ ] |
| Section title letter spacing | 0.15em visible | [ ] |
| Nav item height | 32px | [ ] |
| Nav item padding | 7px 16px | [ ] |
| Nav normal text | 13px, `#9494a8` | [ ] |
| Nav hover bg | `#24242e` | [ ] |
| Nav hover text | `#d4d4dc` | [ ] |
| Nav active bg | rgba(0.14) accent tint | [ ] |
| Nav active text | `#6d9be0` | [ ] |
| Nav active left border | 3px `#6d9be0` | [ ] |
| Icons | 16px, 0.7 opacity (normal), 1.0 (active) | [ ] |
| Modified dot | 6px, `#e0c454` | [ ] |
| Footer border-top | 2px `#2a2a36` | [ ] |
| Version text | 11px, `#8c8ca0` | [ ] |
| Config path | 10px, `#8c8ca0` at 0.7 alpha | [ ] |
| Footer padding | 12px 28px | [ ] |

**Content area:**

| Element | Expected | Check |
|---------|----------|-------|
| Page title | 18px, Bold (700), uppercase, `#eeeeef`, 0.05em spacing | [ ] |
| Page subtitle | 12px, `#9494a8` | [ ] |
| Header padding | 24px 28px 20px | [ ] |
| Section title | 11px, Medium (500), uppercase, `#8c8ca0`, `//` prefix | [ ] |
| Section divider | 2px, `#2a2a36`, extends to right edge | [ ] |
| Section gap | 28px | [ ] |
| Setting name | 13px, `#d4d4dc` | [ ] |
| Setting desc | 11.5px, `#9494a8` | [ ] |
| Setting row padding | 10px 14px | [ ] |
| Setting row min-height | 44px | [ ] |
| Setting row hover bg | `#1c1c24` | [ ] |
| Row-to-row gap | 0px (flush) | [ ] |
| Content body padding | 0 28px 28px | [ ] |

**Controls:**

| Element | Expected | Check |
|---------|----------|-------|
| Slider track | 120px wide, 4px tall, `#2a2a36` | [ ] |
| Slider thumb | 12x14, `#6d9be0`, 2px border `#16161c` | [ ] |
| Slider value | 12px, `#9494a8`, right-aligned, 48px | [ ] |
| Toggle track | 38x20, 2px border `#2a2a36`, bg `#2a2a36` | [ ] |
| Toggle thumb | 12x12, `#8c8ca0` (off), `#6d9be0` (on) | [ ] |
| Toggle checked bg | rgba accent 0.14, border `#6d9be0` | [ ] |
| Dropdown min-width | 140px | [ ] |
| Dropdown padding | 6px 10px (left), 30px (right, arrow area) | [ ] |
| Dropdown border | 2px `#2a2a36` | [ ] |
| Dropdown bg | `#12121a` | [ ] |
| Dropdown arrow | right 10px, centered, `#8c8ca0` | [ ] |

**Footer:**

| Element | Expected | Check |
|---------|----------|-------|
| Height | 52px | [ ] |
| Border-top | 2px `#2a2a36` (content area only) | [ ] |
| Padding | 12px 28px | [ ] |
| Reset button | left-aligned, danger-ghost style | [ ] |
| Cancel button | right group, ghost style | [ ] |
| Save button | rightmost, primary style | [ ] |
| Button gap | 8px between Cancel and Save | [ ] |
| Button font | 12px, Medium (500), uppercase, 0.04em spacing | [ ] |
| UNSAVED indicator | 11px, `#e0c454`, warning icon, not overlapping Reset | [ ] |

---

## 14.3 Cross-Platform Build

All three build gates must pass with zero warnings.

### Commands

```bash
./build-all.sh     # cargo build for all targets
./clippy-all.sh    # clippy with deny(clippy::all) + nursery
./test-all.sh      # cargo test for all crates
```

### Platform targets

- `x86_64-pc-windows-gnu` (cross-compile from WSL)
- Host target (Linux WSL)

### Potential issues

1. **New `FontWeight` variants**: If Section 02 adds `FontWeight::MEDIUM`, ensure all match arms in all crates handle it.
2. **New `ButtonStyle` fields**: Adding `font_weight` and `letter_spacing` fields requires updating all existing `ButtonStyle` construction sites (there may be others beyond footer buttons).
3. **New Config fields**: `unfocused_opacity`, `tab_bar_style` must have serde defaults so existing config files remain valid.
4. **Icon resources**: If Section 08 adds new icon assets, ensure they are embedded (not external files) and available on all platforms.

### Checklist

- [ ] `./build-all.sh` exits 0.
- [ ] `./clippy-all.sh` exits 0.
- [ ] `./test-all.sh` exits 0.
- [ ] No new warnings introduced.
- [ ] Cross-compile target builds cleanly.

---

## 14.4 Performance Validation

Opening and interacting with the settings dialog must not introduce jank or regressions.

### Metrics to verify

1. **Dialog open**: Settings dialog appears within one frame (no visible pop-in or layout shift). First frame may have atlas misses for new font sizes (Section 01), but they should be rasterized and cached on first open, not per-frame.

2. **Glyph atlas**: Multiple font sizes (10, 11, 11.5, 12, 13, 18) should not cause atlas thrashing. The atlas should grow to accommodate all sizes and then stabilize. No atlas rebuild per frame.

3. **Hover interaction**: Moving the mouse over setting rows should produce smooth highlight transitions without dropped frames. The `VisualStateAnimator` should drive transitions at frame rate.

4. **Scroll performance**: Scrolling the content area should not allocate (all `Vec` buffers reused). Damage tracking should limit GPU work to visible rows.

5. **Idle CPU**: With the settings dialog open and no interaction, CPU should be near zero (only cursor blink timer wakeups). No animation frames scheduled when all transitions are complete.

### Test procedure

1. Build release: `cargo build --target x86_64-pc-windows-gnu --release`.
2. Launch `oriterm.exe`.
3. Open settings dialog.
4. Observe: no flicker on first open.
5. Hover over multiple setting rows: smooth transitions.
6. Scroll content area: no jank.
7. Stop interacting: CPU drops to idle baseline.

### Checklist

- [ ] Dialog opens without visible atlas miss flicker.
- [ ] Hover transitions are smooth (no frame drops).
- [ ] Scroll is smooth.
- [ ] Idle CPU matches baseline (cursor blink only).
- [ ] No new allocations in hot render path (verify with existing allocation regression tests).

---

## 14.5 Completion Checklist

All 13 prior sections must be complete before this section can be marked complete.

| Section | Title | Status | Verified |
|---------|-------|--------|----------|
| 01 | Multi-Size Font Rendering | [ ] | [ ] |
| 02 | Numeric Font Weight System | [ ] | [ ] |
| 03 | Text Transform + Letter Spacing | [ ] | [ ] |
| 04 | Line Height Control | [ ] | [ ] |
| 05 | Per-Side Borders | [ ] | [ ] |
| 06 | Opacity + Display Control | [ ] | [ ] |
| 07 | Scrollbar Styling | [ ] | [ ] |
| 08 | Icon Path Verification | [ ] | [ ] |
| 09 | Settings Content Completeness | [ ] | [ ] |
| 10 | Visual Fidelity: Sidebar + Nav | [ ] | [ ] |
| 11 | Visual Fidelity: Content + Typography | [ ] | [ ] |
| 12 | Visual Fidelity: Footer + Buttons | [ ] | [ ] |
| 13 | Visual Fidelity: Widget Controls | [ ] | [ ] |

### Final sign-off criteria

1. All 13 sections marked `status: complete` in their YAML frontmatter.
2. `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` all pass.
3. Visual comparison (14.2) shows no differences from mockup.
4. Performance validation (14.4) shows no regressions.
5. `index.md` updated with all sections marked complete.
6. Plan status in `index.md` changed from `queued` to `complete`.

### Checklist

- [ ] All 13 prior sections complete.
- [ ] All build gates pass.
- [ ] Visual match confirmed.
- [ ] Performance validated.
- [ ] `index.md` updated.
- [ ] Plan marked complete.
