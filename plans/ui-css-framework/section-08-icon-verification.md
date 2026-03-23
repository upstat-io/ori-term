---
section: "08"
title: "Icon Path Verification"
status: not-started
reviewed: true
third_party_review:
  status: none
  updated: null
goal: "All 8 sidebar nav icons match the mockup SVGs — verified by path-to-command conversion and render test"
inspired_by:
  - "SVG path data to PathCommand conversion"
depends_on: []
sections:
  - id: "08.1"
    title: "Mockup SVG Path Extraction"
    status: not-started
  - id: "08.2"
    title: "Current IconPath Comparison"
    status: not-started
  - id: "08.3"
    title: "Fix Mismatched Paths"
    status: not-started
  - id: "08.4"
    title: "Tests"
    status: not-started
  - id: "08.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "08.5"
    title: "Build & Verify"
    status: not-started
---

# Section 08: Icon Path Verification

**Goal:** Verify that all 8 sidebar navigation icons in `oriterm_ui/src/icons/mod.rs` faithfully reproduce the SVG paths from `mockups/settings-brutal.html`. Where they diverge, update the `PathCommand` sequences to match the mockup. All icons are stroked at `NAV_STROKE = 1.0` logical pixels.

**References:**
- `oriterm_ui/src/icons/mod.rs` — `IconId` enum, `PathCommand` enum, `ICON_SUN` through `ICON_ACTIVITY`
- `mockups/settings-brutal.html` lines 1540-1572 — inline SVGs in sidebar nav items
- `oriterm/src/gpu/icon_cache.rs` — `tiny_skia` rasterization from `PathCommand` sequences

---

## 08.1 Mockup SVG Path Extraction

All 8 sidebar nav icons from `mockups/settings-brutal.html`, with SVG attributes `viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"`:

### 1. Sun (Appearance) — Line 1541

```svg
<circle cx="12" cy="12" r="5"/>
<path d="M12 1v2 M12 21v2 M4.22 4.22l1.42 1.42 M18.36 18.36l1.42 1.42
         M1 12h2 M21 12h2 M4.22 19.78l1.42-1.42 M18.36 5.64l1.42-1.42"/>
```

**Normalized (divide by 24):**

Rays (8 lines, move+line pairs):
- Top: `(0.500, 0.042) -> (0.500, 0.125)`
- Bottom: `(0.500, 0.875) -> (0.500, 0.958)`
- Left: `(0.042, 0.500) -> (0.125, 0.500)`
- Right: `(0.875, 0.500) -> (0.958, 0.500)`
- Top-left: `(0.176, 0.176) -> (0.235, 0.235)`
- Bottom-right: `(0.765, 0.765) -> (0.824, 0.824)`
- Bottom-left: `(0.176, 0.824) -> (0.235, 0.765)`
- Top-right: `(0.765, 0.235) -> (0.824, 0.176)`

Wait — the SVG `d` attribute uses relative commands. Let's parse carefully:
- `M12 1v2` = Move to (12,1), vertical line +2 = line to (12,3) => normalized `(0.500, 0.042) -> (0.500, 0.125)`
- `M12 21v2` = Move to (12,21), line to (12,23) => `(0.500, 0.875) -> (0.500, 0.958)`
- `M4.22 4.22l1.42 1.42` = Move to (4.22, 4.22), relative line (1.42, 1.42) = line to (5.64, 5.64) => `(0.176, 0.176) -> (0.235, 0.235)`
- `M18.36 18.36l1.42 1.42` = Move to (18.36, 18.36), line to (19.78, 19.78) => `(0.765, 0.765) -> (0.824, 0.824)`
- `M1 12h2` = Move to (1, 12), horizontal line +2 = line to (3, 12) => `(0.042, 0.500) -> (0.125, 0.500)`
- `M21 12h2` = Move to (21, 12), line to (23, 12) => `(0.875, 0.500) -> (0.958, 0.500)`
- `M4.22 19.78l1.42-1.42` = Move to (4.22, 19.78), line to (5.64, 18.36) => `(0.176, 0.824) -> (0.235, 0.765)`
- `M18.36 5.64l1.42-1.42` = Move to (18.36, 5.64), line to (19.78, 4.22) => `(0.765, 0.235) -> (0.824, 0.176)`

Circle: center (12/24, 12/24) = (0.500, 0.500), radius 5/24 = 0.208.
Must be approximated as path commands since `PathCommand` has no `Circle` variant.
An 8-point circle approximation using cubic Bezier curves (4 arcs, standard kappa = 0.5522847):

```
r = 0.208, k = 0.208 * 0.5523 = 0.1149
Center = (0.500, 0.500)

MoveTo(0.500, 0.292)  // top of circle
CubicTo(0.615, 0.292, 0.708, 0.385, 0.708, 0.500)  // top-right arc
CubicTo(0.708, 0.615, 0.615, 0.708, 0.500, 0.708)  // bottom-right arc
CubicTo(0.385, 0.708, 0.292, 0.615, 0.292, 0.500)  // bottom-left arc
CubicTo(0.292, 0.385, 0.385, 0.292, 0.500, 0.292)  // top-left arc (close)
Close
```

### 2. Palette (Colors) — Line 1546

```svg
<path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10c1.1 0 2-.9 2-2
         0-.5-.2-1-.5-1.3-.3-.4-.5-.8-.5-1.3 0-1.1.9-2 2-2h2.4
         c3.1 0 5.6-2.5 5.6-5.6C23 5.8 18.1 2 12 2z"/>
<circle cx="6.5" cy="11.5" r="1.5"/>
<circle cx="9.5" cy="7.5" r="1.5"/>
<circle cx="14.5" cy="7.5" r="1.5"/>
<circle cx="17.5" cy="11.5" r="1.5"/>
```

This is a complex path with multiple cubic bezier segments (the `C` and `s` commands) forming a paint palette shape, plus 4 small circles for color dots. The main path uses shorthand smooth cubic (`s`) which references the previous control point.

**Normalized key points:**
- Outer shape: starts at (0.500, 0.083), sweeps through an oval-ish palette shape
- 4 dots at normalized positions:
  - (0.271, 0.479) r=0.0625
  - (0.396, 0.313) r=0.0625
  - (0.604, 0.313) r=0.0625
  - (0.729, 0.479) r=0.0625

The full cubic path conversion is complex. The current `ICON_PALETTE` uses an octagon approximation for the outer circle and short line segments for dots. This is a reasonable simplification for a 16px icon — at that size, an octagon is visually indistinguishable from a circle.

However, the mockup's palette icon is not a circle — it is an asymmetric blob (the palette handle curves inward at the bottom-right). At 16px, this detail is mostly lost. The octagon is acceptable but does not match the exact mockup silhouette.

### 3. Type (Font) — Line 1550

```svg
<path d="M4 7V4h16v3 M9 20h6 M12 4v16"/>
```

**Normalized:**
- Top crossbar: Move to (4/24, 7/24) = (0.167, 0.292), vertical line to (4/24, 4/24) = (0.167, 0.167), horizontal line to (20/24, 4/24) = (0.833, 0.167), vertical line to (20/24, 7/24) = (0.833, 0.292)

Wait — parsing `M4 7V4h16v3`: Move to (4, 7), Vertical line to y=4, Horizontal line +16 to x=20, Vertical line +3 to y=7.
So the path is: (4,7) -> (4,4) -> (20,4) -> (20,7). This draws the top serif bracket of the T.

- Bottom serif: `M9 20h6` = Move to (9, 20), horizontal +6 to (15, 20) => `(0.375, 0.833) -> (0.625, 0.833)`
- Vertical stem: `M12 4v16` = Move to (12, 4), vertical +16 to (12, 20) => `(0.500, 0.167) -> (0.500, 0.833)`

**Current `ICON_TYPE`:**
```rust
// Top crossbar: (0.17, 0.17) -> (0.83, 0.17)
// Vertical stem: (0.50, 0.17) -> (0.50, 0.83)
// Bottom serif: (0.33, 0.83) -> (0.67, 0.83)
```

**Comparison:** The current icon draws a simple horizontal line for the top crossbar, not the bracket shape (which includes vertical drops from the crossbar ends to y=7/24=0.292). The mockup's `V4h16v3` draws a U-shaped bracket at the top. The bottom serif widths also differ: current is 0.33-0.67 (width 0.34), mockup is 0.375-0.625 (width 0.25).

**Mismatch:** The top bracket shape is missing — the current icon has a simple horizontal line at y=0.17, while the mockup has a bracket `(0.167, 0.292) -> (0.167, 0.167) -> (0.833, 0.167) -> (0.833, 0.292)`.

### 4. Terminal — Line 1554

```svg
<polyline points="4 17 10 11 4 5"/>
<line x1="12" y1="19" x2="20" y2="19"/>
```

**Normalized:**
- Chevron: `(0.167, 0.708) -> (0.417, 0.458) -> (0.167, 0.208)`
- Input line: `(0.500, 0.792) -> (0.833, 0.792)`

**Current `ICON_TERMINAL`:**
```rust
// Chevron: (0.17, 0.21) -> (0.42, 0.46) -> (0.17, 0.71)
// Input line: (0.50, 0.79) -> (0.83, 0.79)
```

**Comparison:** The chevron points are very close (0.17 vs 0.167, 0.42 vs 0.417, 0.21 vs 0.208, etc.). These are rounding differences from dividing by 24. The input line matches (0.79 vs 0.792). **Essentially matching** — within rounding tolerance at 16px.

### 5. Keyboard (Keybindings) — Line 1558

```svg
<rect x="2" y="4" width="20" height="16" rx="2"/>
<path d="M6 8h.01 M10 8h.01 M14 8h.01 M18 8h.01
         M8 12h.01 M12 12h.01 M16 12h.01 M8 16h8"/>
```

**Normalized:**
- Outer frame: rect at (0.083, 0.167) with size (0.833, 0.667), rx=0.083 (rounded corners)
  As path: `(0.083, 0.167) -> (0.917, 0.167) -> (0.917, 0.833) -> (0.083, 0.833) -> Close`
  (Corner radius is a rendering concern, not a path concern — the rect is drawn with `Close`)

- Top row key dots at y=8/24=0.333: x positions 6, 10, 14, 18 => 0.250, 0.417, 0.583, 0.750
  Each is `h.01` — a 0.01-unit horizontal line (essentially a dot)

- Middle row key dots at y=12/24=0.500: x positions 8, 12, 16 => 0.333, 0.500, 0.667

- Space bar at y=16/24=0.667: `M8 16h8` = (8/24, 16/24) horizontal +8/24 => `(0.333, 0.667) -> (0.667, 0.667)`

**Current `ICON_KEYBOARD`:**
```rust
// Frame: (0.08, 0.17) -> (0.92, 0.17) -> (0.92, 0.83) -> (0.08, 0.83) -> Close
// Top row: 3 dots at (0.25, 0.33), (0.50, 0.33), (0.71, 0.33)
// Middle row: 2 dots at (0.33, 0.50), (0.63, 0.50)
// Space bar: (0.33, 0.67) -> (0.67, 0.67)
```

**Mismatches:**
1. **Top row count:** Mockup has 4 key dots (x: 6, 10, 14, 18). Current has 3 dots (x: 0.25, 0.50, 0.71). Missing the 4th dot at x=18/24=0.750.
2. **Middle row count:** Mockup has 3 key dots (x: 8, 12, 16). Current has 2 dots (x: 0.33, 0.63). Missing the 3rd dot at x=16/24=0.667.
3. **Top row positions:** Current third dot is at 0.71 but should be at 0.583. Fourth dot at 0.750 is missing entirely.
4. **Middle row positions:** Current second dot is at 0.63 but should be at 0.500. Third dot at 0.667 is missing.

### 6. Window — Line 1562

```svg
<rect x="3" y="3" width="18" height="18" rx="2"/>
<path d="M3 9h18"/>
```

**Normalized:**
- Frame: (3/24, 3/24) = (0.125, 0.125), size (0.750, 0.750)
  Path: `(0.125, 0.125) -> (0.875, 0.125) -> (0.875, 0.875) -> (0.125, 0.875) -> Close`
- Title bar line: `(0.125, 0.375) -> (0.875, 0.375)`

**Current `ICON_WINDOW`:**
```rust
// Frame: (0.12, 0.12) -> (0.88, 0.12) -> (0.88, 0.88) -> (0.12, 0.88) -> Close
// Title bar: (0.12, 0.33) -> (0.88, 0.33)
```

**Comparison:** Frame matches within rounding (0.12 vs 0.125, 0.88 vs 0.875). Title bar line: current y=0.33 vs mockup y=0.375 (9/24). This is a noticeable difference at 16px — the title bar line should be lower (at ~6px from top instead of ~5.3px). **Minor mismatch.**

### 7. Bell — Line 1568

```svg
<path d="M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9"/>
<path d="M13.73 21a2 2 0 01-3.46 0"/>
```

**Normalized key points:**

The bell path uses SVG arc commands (`A`) and smooth cubic (`s`):
- Start at (18/24, 8/24) = (0.750, 0.333)
- Arc (radius 6, large-arc=0, sweep=0) to (6/24, 8/24) = (0.250, 0.333) — top of bell
- `c0 7-3 9-3 9` = cubic Bezier: relative control points (0,7), (-3,9), endpoint (-3,9) from (0.250, 0.333):
  Control 1: (0.250, 0.625), Control 2: (0.125, 0.708), End: (0.125, 0.708)
  Wait — `c0 7-3 9-3 9` means: `c dx1 dy1 dx2 dy2 dx dy` = `c 0 7 -3 9 -3 9`.
  From (6, 8): c1=(6,15), c2=(3,17), end=(3,17) => normalized c1=(0.250, 0.625), c2=(0.125, 0.708), end=(0.125, 0.708)
- `h18` = horizontal +18 to x=21 => (0.875, 0.708)
- `s-3-2-3-9` = smooth cubic: from (21, 17), cp2=(-3,-2)=(18,15), end=(-3,-9)=(18,8) => back to (0.750, 0.333)

Clapper: `M13.73 21a2 2 0 01-3.46 0` — arc from (13.73/24, 21/24) to (10.27/24, 21/24)

**Current `ICON_BELL`:** Uses a simplified construction with a top stem, cubic-curved bell sides, a horizontal brim, and a clapper line. The general shape is correct but the exact path differs from the SVG.

**Assessment:** The bell shape involves arcs (which `PathCommand` does not support natively — only cubic Beziers). The current approximation with `CubicTo` curves is the correct approach. The key question is whether the control points accurately approximate the SVG arcs. A visual comparison at 16px is needed. The overall silhouette (bell body widening downward with a flat brim and clapper) matches.

### 8. Activity (Rendering) — Line 1572

```svg
<path d="M22 12h-4l-3 9L9 3l-3 9H2"/>
```

**Normalized (divide by 24):**
- Move to (22/24, 12/24) = (0.917, 0.500)
- h-4: horizontal to (18/24, 12/24) = (0.750, 0.500)
- l-3 9: relative line to (15/24, 21/24) = (0.625, 0.875)
- L9 3: absolute line to (9/24, 3/24) = (0.375, 0.125)
- l-3 9: relative line to (6/24, 12/24) = (0.250, 0.500)
- H2: horizontal to (2/24, 12/24) = (0.083, 0.500)

**Current `ICON_ACTIVITY`:**
```rust
MoveTo(0.04, 0.50),   // should be (0.083, 0.50) — but drawn left-to-right
LineTo(0.25, 0.50),   // matches (0.250, 0.500)
LineTo(0.38, 0.12),   // should be (0.375, 0.125)
LineTo(0.50, 0.88),   // should be (0.625, 0.875) — WRONG x-coordinate
LineTo(0.62, 0.25),   // should be (0.750, 0.500) — both x and y WRONG
LineTo(0.75, 0.50),   // this should be the end at (0.917, 0.500)
LineTo(0.96, 0.50),
```

**Note:** The current icon draws the path left-to-right (flipped from the SVG which starts at the right). This is fine for stroke rendering — the visual result is the same. Let's compare the normalized vertices in left-to-right order:

SVG path right-to-left: (0.917, 0.500) -> (0.750, 0.500) -> (0.625, 0.875) -> (0.375, 0.125) -> (0.250, 0.500) -> (0.083, 0.500)

Reversed (left-to-right): (0.083, 0.500) -> (0.250, 0.500) -> (0.375, 0.125) -> (0.625, 0.875) -> (0.750, 0.500) -> (0.917, 0.500)

Current: (0.04, 0.50) -> (0.25, 0.50) -> (0.38, 0.12) -> (0.50, 0.88) -> (0.62, 0.25) -> (0.75, 0.50) -> (0.96, 0.50)

**Mismatches:**
1. Start x: 0.04 vs 0.083
2. Point 3: x=0.50, y=0.88 vs x=0.625, y=0.875 — **x is wrong** (0.50 vs 0.625)
3. Point 4: x=0.62, y=0.25 vs x=0.750, y=0.500 — **both x and y wrong**
4. Point 5: x=0.75 (should be part of the baseline) and point 6 extends to 0.96 vs 0.917
5. The current icon has **7 vertices** but the mockup SVG has **6 vertices**

The current icon draws a different waveform shape than the mockup. The mockup's EKG line has: flat baseline, diagonal down-spike, sharp up-spike, return to baseline, flat baseline. The current icon has an extra vertex creating a different wave pattern.

---

## 08.2 Current IconPath Comparison

### Summary Table

| Icon | Mockup Shape | Current Shape | Match | Issues |
|------|-------------|---------------|-------|--------|
| **Sun** | Circle + 8 rays | Octagon + 8 rays | Close | Octagon vs circle center body; ray endpoints differ by rounding |
| **Palette** | Asymmetric blob + 4 circles | Octagon + 4 dot lines | Partial | Palette shape simplified to circle/octagon; dot representation differs |
| **Type** | T with bracket serifs | T with simple crossbar | Partial | Missing top bracket shape (V drops); bottom serif width differs |
| **Terminal** | Chevron + input line | Chevron + input line | Match | Within rounding tolerance |
| **Keyboard** | Rect + 4+3 dots + spacebar | Rect + 3+2 dots + spacebar | Partial | Missing key dots (4th top row, 3rd middle row); positions shifted |
| **Window** | Rect + title bar line | Rect + title bar line | Close | Title bar line y-offset differs (0.375 vs 0.333) |
| **Bell** | Arc bell + clapper arc | Cubic bell + clapper line | Close | Cubic approximation of arcs; general silhouette matches |
| **Activity** | 6-vertex EKG line | 7-vertex wave line | Wrong | Different vertex positions, extra vertex, wrong wave shape |

### Priority Fix Order

1. **Activity** — vertex positions are significantly wrong, producing a different wave shape
2. **Keyboard** — missing key dots change the icon's recognizability
3. **Type** — missing bracket makes the T look like a simple cross
4. **Window** — title bar line position is off
5. **Sun** — octagon-to-circle upgrade for fidelity (low priority, looks fine at 16px)
6. **Palette** — complex shape, simplified version is acceptable at 16px
7. **Bell** — cubic approximation is acceptable

---

## 08.3 Fix Mismatched Paths

### Activity Icon (Critical)

Replace the current `ICON_ACTIVITY` with corrected path matching the mockup SVG:

```rust
static ICON_ACTIVITY: IconPath = IconPath {
    commands: &[
        PathCommand::MoveTo(0.083, 0.500),
        PathCommand::LineTo(0.250, 0.500),
        PathCommand::LineTo(0.375, 0.125),
        PathCommand::LineTo(0.625, 0.875),
        PathCommand::LineTo(0.750, 0.500),
        PathCommand::LineTo(0.917, 0.500),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};
```

### Keyboard Icon (Important)

Add the missing 4th top-row dot and 3rd middle-row dot, and correct positions:

```rust
static ICON_KEYBOARD: IconPath = IconPath {
    commands: &[
        // Outer frame.
        PathCommand::MoveTo(0.083, 0.167),
        PathCommand::LineTo(0.917, 0.167),
        PathCommand::LineTo(0.917, 0.833),
        PathCommand::LineTo(0.083, 0.833),
        PathCommand::Close,
        // Top row keys (4 dots).
        PathCommand::MoveTo(0.250, 0.333),
        PathCommand::LineTo(0.254, 0.333),
        PathCommand::MoveTo(0.417, 0.333),
        PathCommand::LineTo(0.421, 0.333),
        PathCommand::MoveTo(0.583, 0.333),
        PathCommand::LineTo(0.587, 0.333),
        PathCommand::MoveTo(0.750, 0.333),
        PathCommand::LineTo(0.754, 0.333),
        // Middle row keys (3 dots).
        PathCommand::MoveTo(0.333, 0.500),
        PathCommand::LineTo(0.337, 0.500),
        PathCommand::MoveTo(0.500, 0.500),
        PathCommand::LineTo(0.504, 0.500),
        PathCommand::MoveTo(0.667, 0.500),
        PathCommand::LineTo(0.671, 0.500),
        // Space bar.
        PathCommand::MoveTo(0.333, 0.667),
        PathCommand::LineTo(0.667, 0.667),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};
```

**Note on dot rendering:** The mockup uses `h.01` (0.01-unit horizontal lines) for key dots. At 16px icon size, 0.01 * 16 = 0.16px — sub-pixel. The `tiny_skia` rasterizer with 1.0px stroke width will render these as small dots of approximately 1x1 pixel, which is the intended appearance. The current approach of using 0.04-unit lines (0.04 * 16 = 0.64px) is also fine.

The dot line length does not need to exactly match the mockup's `h.01` — what matters is that the dot positions are correct and they render as visible dots at the target size.

### Type Icon (Important)

Replace the simple crossbar with the bracket-shaped top:

```rust
static ICON_TYPE: IconPath = IconPath {
    commands: &[
        // Top bracket (left drop, crossbar, right drop).
        PathCommand::MoveTo(0.167, 0.292),
        PathCommand::LineTo(0.167, 0.167),
        PathCommand::LineTo(0.833, 0.167),
        PathCommand::LineTo(0.833, 0.292),
        // Vertical stem.
        PathCommand::MoveTo(0.500, 0.167),
        PathCommand::LineTo(0.500, 0.833),
        // Bottom serif.
        PathCommand::MoveTo(0.375, 0.833),
        PathCommand::LineTo(0.625, 0.833),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};
```

### Window Icon (Minor)

Adjust title bar line y-position from 0.33 to 0.375:

```rust
static ICON_WINDOW: IconPath = IconPath {
    commands: &[
        PathCommand::MoveTo(0.125, 0.125),
        PathCommand::LineTo(0.875, 0.125),
        PathCommand::LineTo(0.875, 0.875),
        PathCommand::LineTo(0.125, 0.875),
        PathCommand::Close,
        PathCommand::MoveTo(0.125, 0.375),
        PathCommand::LineTo(0.875, 0.375),
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};
```

### Sun Icon (Low Priority)

Replace the octagon center with a 4-arc cubic Bezier circle:

```rust
static ICON_SUN: IconPath = IconPath {
    commands: &[
        // 8 rays (matching mockup positions).
        PathCommand::MoveTo(0.500, 0.042),
        PathCommand::LineTo(0.500, 0.125),
        PathCommand::MoveTo(0.500, 0.875),
        PathCommand::LineTo(0.500, 0.958),
        PathCommand::MoveTo(0.042, 0.500),
        PathCommand::LineTo(0.125, 0.500),
        PathCommand::MoveTo(0.875, 0.500),
        PathCommand::LineTo(0.958, 0.500),
        PathCommand::MoveTo(0.176, 0.176),
        PathCommand::LineTo(0.235, 0.235),
        PathCommand::MoveTo(0.765, 0.765),
        PathCommand::LineTo(0.824, 0.824),
        PathCommand::MoveTo(0.176, 0.824),
        PathCommand::LineTo(0.235, 0.765),
        PathCommand::MoveTo(0.765, 0.235),
        PathCommand::LineTo(0.824, 0.176),
        // Center circle (4-arc Bezier approximation, r=5/24=0.208).
        PathCommand::MoveTo(0.500, 0.292),
        PathCommand::CubicTo(0.615, 0.292, 0.708, 0.385, 0.708, 0.500),
        PathCommand::CubicTo(0.708, 0.615, 0.615, 0.708, 0.500, 0.708),
        PathCommand::CubicTo(0.385, 0.708, 0.292, 0.615, 0.292, 0.500),
        PathCommand::CubicTo(0.292, 0.385, 0.385, 0.292, 0.500, 0.292),
        PathCommand::Close,
    ],
    style: IconStyle::Stroke(NAV_STROKE),
};
```

### Palette and Bell Icons (Defer)

The Palette icon's asymmetric blob shape and the Bell icon's arc-based curves are acceptable approximations at 16px. These can be refined later if visual comparison shows noticeable differences. Do not block the section on these.

---

## 08.4 Tests

### Render Smoke Tests

**File:** `oriterm_ui/src/icons/tests.rs`

Verify that each icon's `PathCommand` sequence can be iterated without panics and that the path stays within the 0.0-1.0 normalized bounds:

```rust
#[test]
fn all_nav_icons_within_bounds() {
    let nav_icons = [
        IconId::Sun,
        IconId::Palette,
        IconId::Type,
        IconId::Terminal,
        IconId::Keyboard,
        IconId::Window,
        IconId::Bell,
        IconId::Activity,
    ];

    for id in nav_icons {
        let path = id.path();
        for cmd in path.commands {
            match cmd {
                PathCommand::MoveTo(x, y) | PathCommand::LineTo(x, y) => {
                    assert!(
                        *x >= -0.01 && *x <= 1.01 && *y >= -0.01 && *y <= 1.01,
                        "{id:?} has out-of-bounds point ({x}, {y})"
                    );
                }
                PathCommand::CubicTo(cx1, cy1, cx2, cy2, x, y) => {
                    // Control points can slightly exceed 0-1 for curve overshoot.
                    assert!(
                        *x >= -0.05 && *x <= 1.05 && *y >= -0.05 && *y <= 1.05,
                        "{id:?} has out-of-bounds cubic endpoint ({x}, {y})"
                    );
                    let _ = (cx1, cy1, cx2, cy2); // Control points allowed wider range.
                }
                PathCommand::Close => {}
            }
        }
    }
}
```

### Vertex Count Tests

Verify each icon has the expected number of vertices (catches accidental deletions/additions):

```rust
#[test]
fn activity_icon_has_6_vertices() {
    let path = IconId::Activity.path();
    let vertex_count = path.commands.iter().filter(|c| !matches!(c, PathCommand::Close)).count();
    assert_eq!(vertex_count, 6, "Activity icon should have 6 path vertices (not 7)");
}

#[test]
fn keyboard_icon_has_4_top_row_dots() {
    let path = IconId::Keyboard.path();
    // Count MoveTo commands at approximately y=0.333 (top row).
    let top_row = path.commands.iter().filter(|c| {
        matches!(c, PathCommand::MoveTo(_, y) if (*y - 0.333).abs() < 0.01)
    }).count();
    assert_eq!(top_row, 4, "Keyboard icon should have 4 top-row key dots");
}
```

---

## 08.R Third Party Review Findings

Reserved for findings from `/review-plan` or external review. Not actionable until populated.

---

## 08.5 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Checklist

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] All 8 nav icons render without panics at 16px
- [ ] Activity icon renders correct EKG waveform (visual check)
- [ ] Keyboard icon shows 4+3+1 key layout (visual check)
- [ ] Type icon shows bracket-serif T shape (visual check)
- [ ] No regressions in tab bar icons (Close, Plus, ChevronDown) or window chrome icons
