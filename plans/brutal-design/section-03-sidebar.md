---
section: "03"
title: "Sidebar Enhancements"
status: not-started
reviewed: false
goal: "Sidebar matches mockup: // prefixed section titles, 3px left border active indicator, icon support, version footer"
depends_on: ["01"]
sections:
  - id: "03.1"
    title: "Section Title Styling"
    status: not-started
  - id: "03.2"
    title: "Active Indicator"
    status: not-started
  - id: "03.3"
    title: "Version Footer"
    status: not-started
  - id: "03.4"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: Sidebar Enhancements

**Status:** Not Started
**Goal:** Sidebar matches mockup: `//` prefixed section titles in `--text-faint`, 3px left border on active item with `--accent-bg-strong` background, nav item hover with `--bg-hover`, version label at bottom.

**File(s):** `oriterm_ui/src/widgets/sidebar_nav/mod.rs`

**Reference:** `mockups/settings-brutal.html` CSS classes `.sidebar-title`, `.nav-item`, `.nav-item.active`

---

## 03.1 Section Title Styling

Mockup: section titles have `//` prefix, uppercase, letter-spacing 0.15em, `--text-faint` color.

- [ ] Prefix section titles with `"// "` in `paint()`:
  ```rust
  let title_text = format!("// {}", section.title.to_uppercase());
  ```
- [ ] Use `--text-faint` color (`theme.fg_faint`) for section titles
- [ ] Font size: 10px, weight: 400 (regular, not bold)
- [ ] Letter spacing: 0.15em equivalent (if text shaping supports it, else skip)

---

## 03.2 Active Indicator

Mockup: active item has 3px left border in `--accent`, `--accent-bg-strong` background.

- [ ] Draw 3px-wide accent-colored rect on the left edge of the active item:
  ```rust
  let indicator = Rect::new(x, y, 3.0, ITEM_HEIGHT);
  ctx.scene.push_quad(indicator, RectStyle::filled(self.style.active_fg));
  ```
- [ ] Active item background: `--accent-bg-strong` (already `self.style.active_bg`)
- [ ] Active item text color: `--accent` (already `self.style.active_fg`)
- [ ] Non-active items: `--text-muted` color, transparent background
- [ ] Hover items: `--bg-hover` background, `--text` (primary) color

---

## 03.3 Version Footer

Mockup: version label at bottom of sidebar, font-size 11px, `--text-faint` color.

- [ ] Version label already exists (`paint_version_label`) — verify styling matches:
  - Font size: 10-11px
  - Color: `--text-faint`
  - Position: bottom of sidebar with 8px padding

- [ ] Sidebar search bar (from mockup) — defer to future work (not critical for design match)

---

## 03.4 Completion Checklist

- [ ] Section titles show `// GENERAL`, `// ADVANCED` format
- [ ] Active item has 3px left accent border
- [ ] Active item background is accent-bg-strong
- [ ] Hover items show bg-hover background
- [ ] Version label renders at bottom
- [ ] `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green

**Exit Criteria:** Sidebar visually matches mockup's `.sidebar` section at 100% DPI.
