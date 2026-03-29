# ori_term UI Design System

Brutalist terminal aesthetic applied to GPU-rendered application chrome. Every pixel is drawn by our renderer — no OS controls, no platform widgets, no apologies. This is a machine that renders a terminal grid; its UI is an extension of that grid, not a skin borrowed from another design system.

Not ugly-brutalism. Intentional, high-craft brutalism with a terminal soul. The same design language as [ori-term.com](https://ori-term.com), adapted for application chrome rendered at 60+ FPS on the GPU.

---

## Design Principles

1. **Own it.** We render everything. No attempt to mimic native OS controls. A fully custom visual language that is self-evidently its own thing — like Blender, Ableton, or a game engine.
2. **Structural honesty.** Borders expose the grid. Surfaces are flat. Depth comes from color steps and border weight, never from shadows or gradients.
3. **Mechanical, not fluid.** State changes snap. Hover states appear instantly. Animations use `steps()`, never easing. This is a machine, not water.
4. **Monospace everywhere.** One font family for all chrome: IBM Plex Mono. The terminal grid uses the user's chosen font; every other surface uses Plex Mono.
5. **Every element earns its place.** No ornamental whitespace, no decorative flourishes, no rounded corners softening edges that should be sharp.

---

## Color Palette

### Backgrounds

Backgrounds step in defined increments. No in-between values.

```
bg-base       #0e0e12    Deepest background — terminal content, active tab bleed
bg-input      #12121a    Input field fills
bg-surface    #16161c    Primary chrome surface — tab bar, settings panel, footer
bg-raised     #1c1c24    Elevated containers — cards, color editor, stat cards
bg-hover      #24242e    Hover state for interactive elements
bg-active     #2a2a36    Active/pressed state, toggle track (off)
```

### Text

```
text-bright   #eeeeef    Page titles, active tab label, emphasis
text          #d4d4dc    Primary body text, setting names
text-muted    #9494a8    Descriptions, secondary info, inactive tab labels
text-faint    #8c8ca0    Section labels, // prefixes, tertiary info
```

### Accent & Semantic

```
accent        #6d9be0    Primary interactive color — links, focus, active indicators
accent-hover  #85ade8    Hover variant of accent
accent-bg     rgba(109,155,224, 0.08)    Subtle accent background
accent-bg+    rgba(109,155,224, 0.14)    Stronger accent background (active nav, selected card)

warning       #e0c454    Modified indicators, unsaved state, caution callouts
warning-bg    rgba(224,196,84, 0.08)     Warning callout background

danger        #c87878    Destructive actions, close hover, error states
danger-hover  #d89090    Danger hover variant
danger-bg     rgba(200,120,120, 0.08)    Danger callout background

success       #6bffb8    Build success, test pass, positive outcomes
```

### Borders

```
border-subtle  #1e1e28    Decorative separators — section lines, internal dividers
border         #2a2a36    Standard structural border — inputs, cards, tab dividers
border-strong  #3a3a48    Prominent structure — window frame, major grid lines
```

### Rules

- **No gradients.** Flat fills only.
- **No shadows.** Depth is communicated through background steps and border weight.
- **Desaturated semantics.** The accent blue, warning yellow, and danger red are all grayed off from pure saturation. They belong to the same machine, not borrowed from a candy-colored design system.
- **Accent-bg overlays** are computed by compositing the accent rgba over the surface background. In the GPU renderer, pre-multiply these.

---

## Typography

**Chrome font:** IBM Plex Mono — Regular (400), Medium (500), Bold (700).
**Terminal grid font:** User-configured (Cascadia Code, JetBrains Mono, etc.).

The chrome font is never the terminal font. They are independent.

### Hierarchy

| Element            | Size   | Weight | Case      | Letter-spacing |
|--------------------|--------|--------|-----------|----------------|
| Page title         | 18px   | 700    | UPPERCASE | 0.05em         |
| Section label      | 11px   | 500    | UPPERCASE | 0.15em         |
| Setting name       | 13px   | 400    | Normal    | Normal         |
| Setting desc       | 12px   | 400    | Normal    | Normal         |
| Tab title          | 12px   | 400    | Normal    | Normal         |
| Button text        | 12px   | 500    | UPPERCASE | 0.04em         |
| Tag / badge        | 9px    | 700    | UPPERCASE | 0.06em         |
| Status bar         | 11px   | 400    | Normal    | Normal         |
| Stat value         | 18px   | 600    | Normal    | Normal         |
| Stat label         | 10px   | 500    | UPPERCASE | 0.12em         |

### Rules

- All headings and section labels are UPPERCASE with letter-spacing.
- Section labels use `// LABEL` comment syntax: `// THEME`, `// GPU`, `// CLIPBOARD`.
- The `//` prefix is structural, not decorative — it reinforces the code-as-interface metaphor.
- Body text line-height: 1.5. Headings: 1.1.

---

## Borders & Corners

### Zero Radius

`border-radius: 0` on everything. No exceptions. No "just 2px to soften it." Zero.

Applies to: window frame, tabs, inputs, selects, toggles, buttons, cards, color cells, swatches, sliders, dialogs, callouts, badges, scrollbar thumbs, cursor options, scheme previews.

### Border Weight

| Context                      | Weight | Color          |
|------------------------------|--------|----------------|
| Window frame                 | 2px    | border-strong  |
| Sidebar divider              | 2px    | border         |
| Tab bar bottom               | 2px    | border         |
| Footer / status bar top      | 2px    | border         |
| Section title rule           | 2px    | border         |
| Input fields, selects        | 2px    | border         |
| Toggle track                 | 2px    | border         |
| Cards, color editor          | 2px    | border         |
| Scheme cards                 | 2px    | border         |
| Dialog overlay               | 2px    | border-strong  |
| Split pane divider           | 2px    | border         |
| Tab dividers (between tabs)  | 1px    | border         |
| Stepper internal divider     | 1px    | border         |
| Inner detail borders         | 1px    | border         |

### Rules

- Structural borders are always 2px — they define the grid.
- Internal detail borders (within a component) may be 1px.
- No `1px solid` on component boundaries that define interactive regions.
- Border color steps with the surface: subtle for decorative, standard for structural, strong for primary frame.

---

## Interaction States

All state changes are **instant**. No transitions, no easing, no animation duration on hover/focus/active.

### Hover

| Element           | Change                                           |
|-------------------|--------------------------------------------------|
| Setting row       | Background snaps to bg-raised                    |
| Nav item          | Background snaps to bg-hover, text to text       |
| Button (ghost)    | Background to bg-hover, text to text             |
| Button (primary)  | Background to accent-hover                       |
| Button (danger)   | Background to danger-bg, text/border to danger   |
| Tab               | Background to bg-hover, text to text             |
| Scheme card       | Background to bg-hover, border to border-strong  |
| Color cell        | Border snaps to text (white outline)             |
| Keybind row       | Background to bg-raised                          |
| Split divider     | Color snaps to accent                            |
| Window close btn  | Background to danger, text to text-bright        |

### Focus

- Input/select: border-color snaps to accent.
- Num-stepper wrapper: border-color snaps to accent (via `:focus-within`).
- Active pane: 2px accent outline (inset).

### Active / Selected

- Nav item: accent-bg-strong background + 3px left accent border.
- Scheme card: accent border.
- Cursor option: accent border + accent-bg fill.
- Active tab: accent top-bar (2px), background bleeds to bg-base.

---

## Components

### Tab Bar

- **Height:** 36px.
- **Background:** bg-surface.
- **Bottom border:** 2px solid border.
- **Tabs:** flat rectangles, 14px horizontal padding, max-width 200px.
- **Tab dividers:** 1px solid border between tabs.
- **Active tab:** 2px accent bar on top edge. Background changes to bg-base. Bottom border dissolves (active tab "opens into" content). The top accent bar is the primary visual indicator — no background highlight needed.
- **Tab close button:** 16x16px hit target. Hidden by default, visible on hover (opacity). Hover color: danger.
- **Modified indicator:** 6x6px square (not circle) in accent color.
- **New tab / split buttons:** 36px wide, centered 14px icon, 1px left border.
- **Window controls:** 46px wide each. Minimize/maximize: hover bg-hover. Close: hover danger background.

### Settings Panel

- **Window:** 860x620px, 2px border-strong frame. No shadow.
- **Sidebar:** 200px wide, bg-base, 2px right border.
- **Sidebar search:** 2px border, bg-surface fill, 12px monospace.
- **Sidebar nav items:** 3px transparent left border. Active: accent left border + accent-bg-strong + accent text.
- **Content area:** bg-surface. Header with UPPERCASE 18px title. Scrollable body.
- **Footer:** 2px top border, bg-surface. Save button (primary), Cancel/Reset (ghost). Buttons are UPPERCASE.

### Section Headers

Format: `// SECTION_NAME` in 11px, 500 weight, UPPERCASE, 0.15em letter-spacing, text-faint color. Followed by a 2px horizontal rule in border color that stretches to fill remaining width.

```
// ─── THEME ──────────────────────────────────
```

The `//` is rendered as part of the label text, not a separate element. The rule extends from the end of the text to the right edge.

### Toggle Switch

- **Track:** 38x20px rectangle. Off: bg-active fill, 2px border. On: accent-bg-strong fill, 2px accent border.
- **Thumb:** 12x12px square. Off: text-faint. On: accent. Positioned 3px from edges.
- **No rounded anything.** The track is a rectangle. The thumb is a square. It moves by translating 18px.

### Number Input (Stepper)

- **Wrapper:** single 2px border around the entire control.
- **Input field:** 56px wide, no right border, bg-input fill, centered text.
- **Button column:** 22px wide, 2px left border divider. Two buttons stacked vertically, separated by 1px border.
- **Buttons:** bg-active fill, text-faint arrows (&#9650;/&#9660;). Hover: bg-hover + text. Active: accent-bg-strong + accent.
- **Focus:** wrapper border snaps to accent via `:focus-within`.

### Select / Dropdown

- 2px border, bg-input fill, 12px monospace text.
- Custom dropdown arrow (SVG chevron, right-aligned).
- Hover: border-color to text-faint. Focus: border-color to accent.
- Min-width: 140px.

### Buttons

- **Primary:** accent background, bg-base text (dark on light), 2px accent border. 700 weight. UPPERCASE.
- **Ghost:** transparent background, 2px border, text-muted. Hover: bg-hover + text.
- **Danger ghost:** same as ghost, but hover: danger-bg + danger text + danger border.
- **All buttons:** 12px text, 500 weight, UPPERCASE, 0.04em letter-spacing, 6px vertical / 16px horizontal padding.

### Kbd (Keyboard Shortcut)

- 2px solid border, bg-input fill, 11px text.
- Flat — no bottom-shadow "3D key" effect. Square corners.

### Cards (Scheme Cards, Stat Cards)

- 2px border, bg-raised fill.
- Hover: bg-hover + border-strong.
- Active/selected: accent border + accent-bg.
- Badge: 9px UPPERCASE, 1px accent border + accent-bg-strong fill.

### Callouts (Info, Warning)

- 2px border with rgba tint of the callout color.
- 3px solid left border in the callout color (accent for info, warning for caution).
- Background: semantic -bg token.

### Dialogs (Keybind Editor, Color Picker)

- Centered overlay with rgba(0,0,0,0.6) backdrop.
- 2px border-strong frame, bg-surface fill.
- No shadow. No scale animation. Appears/disappears instantly.
- Title: 14px, 700 weight, UPPERCASE.

### Split Pane Dividers

- **Vertical:** 2px wide, border color. Hover: accent.
- **Horizontal:** 2px tall, border color. Hover: accent.
- Cursor: col-resize / row-resize.

### Status Bar

- **Height:** 22px.
- **Background:** bg-surface.
- **Top border:** 2px solid border.
- **Text:** 11px, text-faint. Accent-colored values for key info (shell name, TERM value).

### Scrollbar

- **Width:** 6px.
- **Track:** transparent.
- **Thumb:** border color. Square (no radius).

---

## Accessibility

Despite the raw aesthetic:

- **WCAG AA text contrast.** All text tokens pass 4.5:1 against their intended backgrounds:
  - text-faint (#8c8ca0): 5.14:1 on bg-raised, 5.47:1 on bg-surface, 5.85:1 on bg-base.
  - text-muted (#9494a8): 5.69:1 on bg-raised, 6.06:1 on bg-surface.
  - accent (#6d9be0): 5.88:1 on bg-raised, 6.26:1 on bg-surface.
  - warning (#e0c454): 7.80:1 on bg-surface.
  - danger (#c87878): 6.49:1 on bg-surface.
- **UI component contrast.** Interactive element borders (border-strong #3a3a48) achieve 3:1+ against primary surfaces per WCAG 1.4.11. Decorative borders (border-subtle) are exempt.
- **Keyboard navigable.** All controls reachable via Tab. Focus indicator: accent border snap.
- **No reliance on color alone.** Active states use structural indicators (top bar, left border, background change) in addition to color.

---

## Relationship to Website Design

The application UI and [ori-term.com](https://ori-term.com) share the same brutalist DNA but diverge where context demands:

| Property        | Website                          | Application UI                   |
|-----------------|----------------------------------|----------------------------------|
| Accent color    | Terminal green `#00ff41`         | Desaturated blue `#6d9be0`       |
| Background      | Neutral blacks (#0a, #11, #1a)   | Purple-tinted darks (#0e, #16)   |
| Font            | IBM Plex Mono                    | IBM Plex Mono (chrome)           |
| Border radius   | 0                                | 0                                |
| Shadows         | None                             | None                             |
| Transitions     | `steps()` or instant             | Instant (0s)                     |
| Section labels  | `// LABEL`                       | `// LABEL`                       |
| Borders         | 2px structural                   | 2px structural                   |
| Typography case | UPPERCASE headings               | UPPERCASE headings               |

The website uses terminal green because it is a marketing surface — the green is the brand signal. The application uses desaturated blue because it is a workspace — you stare at it for hours, and a muted accent reduces visual fatigue while maintaining identity.

---

## GPU Rendering Notes

Every value in this document maps to a GPU draw call:

- **Flat rectangles** → single-color quads. Background fills, borders, dividers, toggle tracks/thumbs.
- **Text** → glyph atlas lookups with IBM Plex Mono metrics. Color is a uniform, not per-glyph texture.
- **Borders** → four thin quads (top/right/bottom/left) at the specified weight and color. Or: one background quad + one inset quad (border = outer minus inner).
- **Accent-bg overlays** → pre-multiply `rgba(109,155,224, 0.08)` against the known surface color at theme-load time. No runtime alpha blending needed for static backgrounds.
- **Hover states** → swap the fill color uniform. No interpolation.
- **Active tab bleed** → draw the tab background quad in bg-base color, then draw the tab-bar bottom border with a gap at the active tab's x-range.

No texture sampling needed for chrome (except glyph atlas). No blend modes. No rounded-corner SDF. Every surface is an axis-aligned rectangle with a solid color.

---

## Reference Mockups

- `mockups/settings-brutal.html` — Full 8-page settings panel with all controls.
- `mockups/main-window-brutal.html` — Main window with tab bar, split panes, status bar.
- `mockups/settings.html` — Original (non-brutal) settings panel for comparison.
