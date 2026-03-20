---
section: "04"
title: "Content Area Styling"
status: not-started
reviewed: false
goal: "Content area matches mockup: uppercase page titles in --text-bright, // prefixed section headers with horizontal rule, correct spacing"
depends_on: ["01"]
sections:
  - id: "04.1"
    title: "Page Header Styling"
    status: not-started
  - id: "04.2"
    title: "Section Title Styling"
    status: not-started
  - id: "04.3"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: Content Area Styling

**Status:** Not Started
**Goal:** Content area page headers and section titles match the mockup: uppercase bold title in `--text-bright`, description in `--text-muted`, section titles with `//` prefix + horizontal rule extending right.

**File(s):** `oriterm/src/app/settings_overlay/form_builder/*.rs` (page builders)

**Reference:** `mockups/settings-brutal.html` CSS classes `.content-header`, `.section-title`

---

## 04.1 Page Header Styling

Mockup: `h1` is 18px, 700 weight, `--text-bright`, uppercase with `letter-spacing: 0.05em`.

- [ ] Page title font size: 18px → update `TITLE_FONT_SIZE` in `appearance.rs`
- [ ] Page title color: `--text-bright` (`theme.fg_bright`) instead of `theme.fg_primary`
- [ ] Page title uppercase: `title.to_uppercase()` in each page builder
- [ ] Description color: `--text-muted` (`theme.fg_secondary`)
- [ ] Description font size: 12px

---

## 04.2 Section Title Styling

Mockup: section titles (THEME, WINDOW, etc.) have:
- `//` prefix
- Uppercase
- 11px font, 500 weight, `--text-faint` color
- Letter-spacing 0.15em
- Horizontal rule extending right: `::after { flex: 1; height: 2px; background: var(--border); }`

- [ ] Update `section_title()` helper in `appearance.rs` to add `"// "` prefix
- [ ] Section title color: `--text-faint` instead of accent
- [ ] Section title text: uppercase
- [ ] Add horizontal rule after section title text:
  - Draw a 2px-high line from the end of the title text to the right edge
  - Color: `--border` (`theme.border`)
  - Use `ctx.scene.push_quad()` for the line

---

## 04.3 Completion Checklist

- [ ] Page titles are uppercase, 18px, --text-bright
- [ ] Section titles show `// THEME`, `// WINDOW` format with --text-faint color
- [ ] Horizontal rule extends from section title to right edge
- [ ] Description text is --text-muted
- [ ] `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green

**Exit Criteria:** Content area page headers and section titles visually match mockup.
