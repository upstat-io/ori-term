---
section: "01"
title: "Theme Token Overhaul"
status: not-started
reviewed: true
goal: "UiTheme::dark() produces colors matching the brutal mockup CSS variables exactly"
depends_on: []
sections:
  - id: "01.1"
    title: "Map CSS Variables to UiTheme Fields"
    status: not-started
  - id: "01.2"
    title: "Completion Checklist"
    status: not-started
---

# Section 01: Theme Token Overhaul

**Status:** Not Started
**Goal:** `UiTheme::dark()` returns colors that match the mockup's CSS `:root` variables.

**File(s):** `oriterm_ui/src/theme/mod.rs`

---

## 01.1 Map CSS Variables to UiTheme Fields

Update `UiTheme::dark()` const to use the mockup's color palette:

- [ ] `bg_primary` → `#16161c` (--bg-surface, content area)
- [ ] `bg_secondary` → `#0e0e12` (--bg-base, sidebar)
- [ ] `bg_card` / `bg_raised` → `#1c1c24` (--bg-raised)
- [ ] `bg_hover` → `#24242e` (--bg-hover)
- [ ] `bg_active` → `#2a2a36` (--bg-active)
- [ ] `bg_input` → `#12121a` (--bg-input)
- [ ] `border` → `#2a2a36` (--border)
- [ ] `border_strong` → `#3a3a48` (--border-strong) — add if missing
- [ ] `fg_primary` → `#d4d4dc` (--text)
- [ ] `fg_secondary` → `#9494a8` (--text-muted)
- [ ] `fg_faint` → `#8c8ca0` (--text-faint)
- [ ] `fg_bright` → `#eeeeef` (--text-bright) — add if missing
- [ ] `accent` → `#6d9be0` (--accent)
- [ ] `accent_hover` → `#85ade8` (--accent-hover) — add if missing
- [ ] `accent_bg` → `rgba(109,155,224,0.08)` (--accent-bg)
- [ ] `accent_bg_strong` → `rgba(109,155,224,0.14)` (--accent-bg-strong)
- [ ] `shadow` → fully transparent or remove (--shadow: none)
- [ ] Verify `UiTheme::dark()` is `const` — if any new fields break const-ness, fix

---

## 01.2 Completion Checklist

- [ ] All mockup CSS variables mapped to UiTheme fields
- [ ] `UiTheme::dark()` compiles and returns correct colors
- [ ] New fields (if any) added with `///` doc comments
- [ ] `./test-all.sh` green (existing tests use `UiTheme::dark()`)
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green

**Exit Criteria:** `UiTheme::dark()` fields match mockup CSS variables when compared side-by-side.
