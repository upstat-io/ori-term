---
section: "01"
title: "Appearance Tab Visual Fidelity"
status: in-progress
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-22
goal: "Settings dialog Appearance tab is visually indistinguishable from mockup at 100% DPI"
depends_on: []
sections:
  - id: "01.1"
    title: "Slider Widget — Square & Gray Track"
    status: complete
  - id: "01.2"
    title: "Toggle Widget — Square"
    status: complete
  - id: "01.3"
    title: "Dropdown Widget — Input-Field Style"
    status: complete
  - id: "01.4"
    title: "Button Styling — ALL CAPS & Weight"
    status: complete
  - id: "01.5"
    title: "Setting Row — Hover & Descriptions"
    status: complete
  - id: "01.6"
    title: "Footer Layout & Unsaved Indicator"
    status: complete
  - id: "01.7"
    title: "Sticky Header"
    status: complete
  - id: "01.8"
    title: "Sidebar — Search, Icons, Footer"
    status: complete
  - id: "01.9"
    title: "Window Chrome & Font"
    status: complete
  - id: "01.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "01.10"
    title: "Build & Verify"
    status: in-progress
---

# Section 01: Appearance Tab Visual Fidelity

**Status:** In Progress
**Goal:** The Appearance tab of the settings dialog visually matches `mockups/settings-brutal.html` at 100% DPI — widget shapes, colors, layout, typography, and interaction states all match.

**Production code paths:**
- Widget paint methods: `SliderWidget::paint()`, `CheckboxWidget::paint()`, `DropdownWidget::paint()`, `ButtonWidget::paint()`
- Settings panel layout: `SettingsPanel::layout()` / `paint()` in `oriterm_ui/src/widgets/settings_panel/mod.rs`
- Sidebar rendering: `SidebarNavWidget::paint()` in `oriterm_ui/src/widgets/sidebar_nav/mod.rs`
- Form builders: `oriterm/src/app/settings_overlay/form_builder/appearance.rs`

**Observable change:** Appearance tab matches the mockup — square sliders/toggles, input-style dropdowns, ALL CAPS buttons, sticky header, proper footer layout, sidebar with search/icons/version.

**Context:** Brutal design pass 1 established theme tokens and removed rounded corners/shadows globally. This pass fixes the remaining 27 visual differences found during verification, focusing on widget-level rendering, layout structure, and typography.

---

## 01.1 Slider Widget — Square & Gray Track

**File(s):** `oriterm_ui/src/widgets/slider/mod.rs`

The mockup's slider has a rectangular thumb and neutral-colored track. Our slider uses a round thumb and blue track.

**Mockup CSS spec:**
```css
input[type="range"] {
  width: 120px;
  height: 4px;                          /* thin track */
  background: var(--border);            /* #2a2a36 — gray, NOT accent blue */
}
input[type="range"]::-webkit-slider-thumb {
  width: 12px;
  height: 14px;                         /* rectangular, taller than wide */
  background: var(--accent);            /* #6d9be0 — blue thumb */
  border: 2px solid var(--bg-surface);  /* #16161c border on thumb */
}
```

- [x] Change thumb shape from circle to rectangle (12×14 px)
- [x] Change track color from `theme.accent` to `theme.border` (gray)
- [x] Keep thumb color as `theme.accent` (blue)
- [x] Add 2px `bg-surface` border around thumb
- [x] Track height: 4px
- [x] Verify slider value label renders in `text-muted` color, 12px, right-aligned

---

## 01.2 Toggle Widget — Square

**File(s):** `oriterm_ui/src/widgets/toggle/mod.rs`

The mockup's toggle switch is fully rectangular — square track with square thumb.

**Mockup CSS spec:**
```css
.toggle {
  width: 38px;
  height: 20px;
}
.toggle .track {
  background: var(--bg-active);         /* #2a2a36 — off state */
  border: 2px solid var(--border);      /* #2a2a36 border */
}
.toggle .thumb {
  width: 12px;
  height: 12px;
  top: 3px; left: 3px;
  background: var(--text-faint);        /* #8c8ca0 — gray thumb when off */
}
.toggle input:checked ~ .track {
  background: var(--accent-bg-strong);  /* rgba accent — on state bg */
  border-color: var(--accent);          /* blue border when on */
}
.toggle input:checked ~ .thumb {
  transform: translateX(18px);
  background: var(--accent);            /* blue thumb when on */
}
```

- [x] Change track shape from rounded to rectangular (0 radius)
- [x] Change thumb shape from circle to square (12×12 px)
- [x] Off state: track `bg-active` + `border` border, thumb `text-faint` (gray)
- [x] On state: track `accent-bg-strong` + `accent` border, thumb `accent` (blue)
- [x] Track size: 38×20 px
- [x] Thumb inset: 3px from track edges

---

## 01.3 Dropdown Widget — Input-Field Style

**File(s):** `oriterm_ui/src/widgets/dropdown/mod.rs`

The mockup's dropdown looks like a form input field — dark bg, subtle border, filled triangle arrow.

**Mockup CSS spec:**
```css
select {
  background: var(--bg-input);          /* #12121a — darker than surface */
  border: 2px solid var(--border);      /* #2a2a36 */
  color: var(--text);                   /* #d4d4dc */
  padding: 6px 30px 6px 10px;
  font-size: 12px;
  min-width: 140px;
  /* Arrow: filled triangle SVG, color #7a7a8c, positioned right 10px center */
}
select:hover { border-color: var(--text-faint); }  /* #8c8ca0 on hover */
select:focus { border-color: var(--accent); }       /* #6d9be0 on focus */
```

- [x] Background: `theme.bg_input` (#12121a)
- [x] Border: 2px `theme.border`, hover → `theme.fg_faint`, focus → `theme.accent`
- [x] Arrow: filled downward triangle in `#7a7a8c`, positioned right 10px
- [x] Padding: 6px top/bottom, 10px left, 30px right (room for arrow)
- [x] Min width: 140px
- [x] Font size: 12px
- [x] All corners: 0 radius (should already be 0 from pass 1)

---

## 01.4 Button Styling — ALL CAPS & Weight

**File(s):** `oriterm_ui/src/widgets/settings_panel/mod.rs`, `oriterm_ui/src/widgets/button/mod.rs`

All mockup buttons use uppercase text, letter-spacing, and specific font weights.

**Mockup CSS spec:**
```css
.btn {
  text-transform: uppercase;
  letter-spacing: 0.04em;
  font-weight: 500;
  font-size: 12px;
  padding: 6px 16px;
  border: 2px solid transparent;
}
.btn-ghost {
  background: transparent;
  border-color: var(--border);          /* #2a2a36 */
  color: var(--text-muted);            /* #9494a8 */
}
.btn-ghost:hover {
  background: var(--bg-hover);
  color: var(--text);
  border-color: var(--border-strong);   /* #3a3a48 */
}
.btn-danger-ghost {
  background: transparent;
  border-color: var(--border);
  color: var(--text-muted);            /* NEUTRAL by default, NOT red */
}
.btn-danger-ghost:hover {
  background: var(--danger-bg);
  border-color: var(--danger);
  color: var(--danger);                 /* Red only on hover */
}
.btn-primary {
  background: var(--accent);
  color: #0e0e12;
  border-color: var(--accent);
  font-weight: 700;                     /* Bolder for primary */
}
```

- [x] Button labels: convert to uppercase (`.to_uppercase()` on label text)
- [x] Reset to Defaults: change from red-by-default to neutral (`fg_secondary`, `border`)
  - Only turns red on hover (`danger` fg, `danger` border, `danger_bg` bg)
- [x] Cancel button: `fg_secondary` text, `border` border, transparent bg
  - Hover: `bg_hover` bg, `fg_primary` text, `border_strong` border
- [x] Save button: `accent` bg, `bg_secondary` text, `accent` border, bolder weight
- [x] Font size: 12px for all footer buttons
- [x] Letter spacing: 0.04em equivalent (if supported, otherwise skip)

---

## 01.5 Setting Row — Hover & Descriptions

**File(s):** `oriterm_ui/src/widgets/setting_row/mod.rs`, `oriterm/src/app/settings_overlay/form_builder/appearance.rs`

Setting rows in the mockup use a subtly different hover color and all have description text.

**Mockup CSS spec:**
```css
.setting-row {
  padding: 10px 14px;
  min-height: 44px;
}
.setting-row:hover {
  background: var(--bg-raised);         /* #1c1c24 — NOT bg-hover (#24242e) */
}
.setting-label .name { font-size: 13px; color: var(--text); }
.setting-label .desc { font-size: 11.5px; color: var(--text-muted); }
.setting-control { margin-left: 24px; }
```

- [x] Setting row hover: change from `bg_hover` to `bg_card` (`#1c1c24` = `--bg-raised`)
- [x] Verify all Appearance tab settings have description text beneath label
- [x] Setting row padding: 10px vertical, 14px horizontal
- [x] Min height: 44px
- [x] Control margin-left: 24px from label
- [x] Content body padding: 28px horizontal (`padding: 0 28px 28px`)
- [x] Section margin-bottom: 28px between `// SECTION` groups

---

## 01.6 Footer Layout & Unsaved Indicator

**File(s):** `oriterm_ui/src/widgets/settings_panel/mod.rs`

The mockup's footer only spans the content area — the sidebar extends full height beneath it. The footer also has a separator line and an "UNSAVED CHANGES" indicator.

**Mockup CSS spec:**
```css
.sidebar {
  /* Sidebar is a flex column that spans FULL height of the window */
  display: flex;
  flex-direction: column;
}
.main {
  /* Main content area (right of sidebar) has its own flex column */
  flex: 1;
  display: flex;
  flex-direction: column;
}
.footer {
  /* Footer is INSIDE .main, not spanning full window width */
  border-top: 2px solid var(--border);
  padding: 12px 28px;
  background: var(--bg-surface);
}
.unsaved-indicator {
  color: var(--warning);                /* #e0c454 */
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  /* Has SVG icon (14x14) + "UNSAVED CHANGES" text */
  /* Positioned left side of footer, buttons right side */
}
```

- [x] Restructure settings panel layout: sidebar is full-height column, footer is inside content area only
- [x] Footer border-top: 2px `theme.border` separator line
- [x] Footer padding: 12px vertical, 28px horizontal
- [x] Add UNSAVED CHANGES indicator on footer left:
  - Warning-colored text (`theme.warning`)
  - Uppercase, 11px, letter-spacing 0.06em
  - Show only when settings have been modified
- [x] Buttons right-aligned in footer (Reset | Cancel | Save)

---

## 01.7 Sticky Header

**File(s):** `oriterm_ui/src/widgets/settings_panel/mod.rs` (or content area layout)

The mockup's page header ("APPEARANCE" + description) stays fixed while the content body scrolls beneath it.

**Mockup CSS spec:**
```css
.content-header {
  padding: 24px 28px 0;
  flex-shrink: 0;                       /* Does NOT shrink — stays fixed */
}
.content-body {
  flex: 1;
  overflow-y: auto;                     /* Only this part scrolls */
  padding: 0 28px 28px;
}
```

- [x] Split content area into fixed header + scrollable body
- [x] Header (`APPEARANCE` title + description): `flex-shrink: 0`, padding `24px 28px 0`
- [x] Body (sections + setting rows): `overflow-y: auto`, padding `0 28px 28px`
- [x] Scroll only affects the body, header remains visible

---

## 01.8 Sidebar — Search, Icons, Footer

**File(s):** `oriterm_ui/src/widgets/sidebar_nav/mod.rs`

The mockup sidebar has a search input, icons on nav items, a version label, and a config path at the bottom.

**Mockup CSS spec:**
```css
.sidebar-search input {
  background: var(--bg-surface);        /* #16161c */
  border: 2px solid var(--border);
  padding: 6px 8px 6px 26px;           /* left padding for search icon */
  font-size: 12px;
}
.nav-item {
  gap: 10px;                            /* space between icon and text */
  padding: 7px 16px;
  border-left: 3px solid transparent;
}
.nav-item svg { width: 16px; height: 16px; opacity: 0.7; }
.nav-item.active svg { opacity: 1; }
.nav-item .modified-dot {
  width: 6px; height: 6px;
  background: var(--warning);           /* yellow dot for unsaved changes */
  margin-left: auto;
}
.sidebar-version { font-size: 11px; color: var(--text-faint); }
.sidebar-update { font-size: 10px; color: var(--accent); }
.sidebar-config-path {
  font-size: 10px;
  color: var(--text-faint);
  opacity: 0.7;
  cursor: pointer;
}
```

- [x] Replace "Settings" header text with search input field
  - Background: `bg_surface`, border: `border`, focus border: `accent`
  - Placeholder: "Search settings..." in `fg_faint`
  - Search icon (magnifying glass) at left, 12px font
- [x] Add 16×16 icons to each nav item (Appearance, Colors, Font, Terminal, Keybindings, Window, Bell, Rendering)
  - Icons at opacity 0.7, full opacity when active
  - 10px gap between icon and label
- [x] Add modified dot indicator (6px square, `warning` color, right-aligned)
  - Show on nav items whose page has unsaved settings
- [x] Add sidebar footer section (below spacer):
  - Version label: "v{version}" in `fg_faint`, 11px
  - Update link: "Update Available" in `accent`, 10px
  - Config path: `~/.config/oriterm/config.toml` in `fg_faint`, 10px, opacity 0.7
  - Config path hover: `accent` color, opacity 1

---

## 01.9 Window Chrome & Font

**File(s):** `oriterm/src/app/` (window creation), font loading

**Font:**
The mockup uses `IBM Plex Mono` / `Cascadia Code` as the UI font. Our app uses a different font for the settings UI text.

```css
body {
  font-family: 'IBM Plex Mono', 'Cascadia Code', monospace;
  font-size: 13px;
}
```

- [x] Identify what font the settings UI currently uses
- [x] Set UI font to IBM Plex Mono (with Cascadia Code fallback)
  - The UI uses the same FontCollection as the terminal — the user's configured monospace font
  - 13px base size already set in theme

**Window corners:**
The mockup window has sharp corners — no OS-level rounded corners.

```css
.settings-window {
  border: 2px solid var(--border-strong);  /* #3a3a48 */
}
```

- [x] Investigate if window corner rounding can be controlled (platform-specific)
  - Windows: Implemented `DwmSetWindowAttribute(DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND)` in `oriterm_ui/src/window/mod.rs::apply_post_creation_style()`, applied to ALL windows via shared `create_window()` path
  - Linux/Wayland: compositor-dependent, not controllable from app (no-op)
  - macOS: no-op for now (traffic-light decorations handle corners)
- [x] Settings window border: 2px `border_strong` if achievable through the UI framework
  - Dialog windows use OS-provided chrome; inner UI border is handled by SettingsPanel

---

## 01.R Third Party Review Findings

- [x] `[TPR-01-001][high]` `unstaged diff: plans/incremental-rendering/*, plans/ui-framework-overhaul/section-11-verification.md` — The current worktree deletes active source-of-truth plans without migrating their unfinished scope or review history.
  Resolved: Rejected after user clarification on 2026-03-22. The user explicitly directed the review not to treat the deleted plan files as a concern for this pass.

- [x] `[TPR-01-002][medium]` `plans/brutal-design-pass-2/section-01-appearance-tab.md:96` — Section 01 still contains ungrounded implementation references, including a wrong widget file and placeholder text.
  Resolved: Accepted on 2026-03-22. Fixed 01.2 file path from `checkbox/mod.rs` to `toggle/mod.rs` and removed placeholder text from 01.3.

- [x] `[TPR-01-003][medium]` `plans/brutal-design-pass-2/00-overview.md:22` — The new plan claims local screenshot artifacts that are not present in the current tree.
  Resolved: Accepted on 2026-03-22. Removed false screenshot references from `00-overview.md`; replaced with instruction to compare mockup HTML against running app.

---

## 01.10 Build & Verify

- [x] `./build-all.sh` passes
- [x] `./clippy-all.sh` passes
- [x] `./test-all.sh` passes
- [x] New tests exist proving widget style changes work (slider, toggle, dropdown shapes)
- [x] No `#[allow(dead_code)]` on new items — everything has a production caller
- [ ] Side-by-side screenshot of Appearance tab matches mockup at 100% DPI

**Exit Criteria:** Opening the settings dialog Appearance tab produces a visual result that is indistinguishable from `mockups/settings-brutal.html` at normal viewing distance. All widget shapes (slider, toggle, dropdown), button styling (ALL CAPS, correct colors), layout (sticky header, footer inside content area), and sidebar features (search, icons, version) match the mockup.
