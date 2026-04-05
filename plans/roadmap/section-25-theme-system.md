---
section: 25
title: Theme System
status: in-progress
reviewed: true
third_party_review:
  status: none
  updated: null
last_verified: "2026-04-04"
tier: 6
goal: 100+ built-in themes, TOML theme files, discovery, live switching, light/dark auto-switch
sections:
  - id: "25.1"
    title: Theme Format & Loading
    status: complete
  - id: "25.2"
    title: Built-in Theme Library
    status: in-progress
  - id: "25.3"
    title: "Light/Dark Auto-Switch"
    status: in-progress
  - id: "25.4"
    title: Section Completion
    status: in-progress
---

# Section 25: Theme System

**Status:** In Progress
**Goal:** Ship 100+ built-in themes selectable by name, with automatic light/dark mode switching based on system preference.

**Crate:** `oriterm` (palette + config layer)
**Dependencies:** `serde` (TOML deserialization), existing `toml` crate, platform crates for dark mode detection

**Inspired by:** Ghostty (300+ built-in themes, light/dark auto-switch), iTerm2 (importable color schemes), base16 (standardized 16-color format), Kitty (theme kitten with preview)

---

## 25.1 Theme Format & Loading

**Status:** Complete (verified 2026-03-29)

**Files:** `oriterm/src/scheme/mod.rs`, `oriterm/src/scheme/loader/mod.rs`, `oriterm/src/scheme/builtin.rs`

- [x] TOML theme file format (`ansi = [16 hex strings]`, `foreground`, `background`, `cursor`, optional `selection_foreground`/`selection_background`)
- [x] `ThemeFile` struct with `Deserialize`, hex parsing (`#RRGGBB`) with validation
- [x] Load from: embedded builtins (53), user theme dir (`config_path().parent()/themes/*.toml`), config by name or absolute path
- [x] Discovery: `discover_count()` (production, count-only), `discover_all()` (test-only, full parse+merge), `resolve_scheme()` (on-demand lookup)
- [x] User themes override built-in names (case-insensitive)
- [x] Theme hot-reload via ConfigMonitor watching `themes/` subdirectory

**Tests (48 passing):** 15 in `loader/tests.rs` (TOML parsing, hex validation, format edge cases), 33 in `scheme/tests.rs` (builtin lookup, discovery, palette construction, conditional parsing, config overrides).

Missing coverage (2 tests):
- [ ] `discover_count_returns_builtin_count` — `discover_count()` is `pub(crate)` but untested
- [ ] `count_themes_nonexistent_dir` — `count_themes()` only tested indirectly via `discover_themes` equivalents

---

## 25.2 Built-in Theme Library

**File:** `oriterm/src/scheme/builtin.rs` (53 schemes, 689 lines)

### 25.2a Prerequisite: Split `builtin.rs`

`builtin.rs` at 689 lines exceeds the 500-line limit. Adding ~47 more schemes would push it to ~1400 lines. Split into a directory module before adding any new schemes.

**Target structure:**
```
oriterm/src/scheme/builtin/
  mod.rs           -- rgb() + ansi16() helpers, BUILTIN_SCHEMES array, re-exports
  catppuccin.rs    -- Catppuccin Mocha/Latte/Frappe/Macchiato
  popular.rs       -- Dracula, Nord, Monokai, Gruvbox, Solarized, One Dark/Light
  tokyo.rs         -- Tokyo Night variants
  nature.rs        -- Rose Pine, Everforest, Kanagawa, Ayu
  material.rs      -- Material, GitHub, Nightfox, Carbonfox, Dawnfox
  retro.rs         -- Zenburn, Tomorrow, Iceberg, PaperColor, Snazzy
  modern.rs        -- Night Owl, Palenight, Horizon, Poimandres, Vesper, etc.
  extended.rs      -- New ~47 schemes (split further if > 500 lines)
```

- [x] Create `builtin/mod.rs` with helpers (`rgb`, `ansi16` as `pub(super) const fn`), `BUILTIN_SCHEMES` array, 7 submodule declarations (completed 2026-04-04)
- [x] Move existing 53 scheme constants into category submodules (`pub(super) const`): catppuccin (4), popular (9), tokyo (4), nature (10), material (8), retro (8), modern (10) (completed 2026-04-04)
- [x] Verify no submodule exceeds 500 lines — largest is `nature.rs` at 130 lines (verified 2026-04-04)
- [x] All existing tests pass unchanged (no public API change) — 69 scheme tests pass (verified 2026-04-04)

### 25.2b Add Remaining Schemes (depends on 25.2a)

**53 built-in schemes implemented** (verified 2026-04-04, exceeds interim 50+ target):
- [x] Catppuccin (4), One Dark/Light (2), Solarized (2), Dracula (1), Tokyo Night (3), WezTerm (1)
- [x] Gruvbox (2), Nord (1), Rose Pine (3), Everforest (2), Kanagawa (2), Ayu (3)
- [x] Material (2), Monokai (1), Nightfox/Dawnfox/Carbonfox (3), GitHub (3)
- [x] Snazzy (1), Tomorrow (2), Zenburn (1), Iceberg (2), Night Owl (1), Palenight (1)
- [x] Horizon (1), Poimandres (1), Vesper (1), Sonokai (1), OneDark Pro (1), Moonfly (1)
- [x] PaperColor (2), Oxocarbon (1), Andromeda (1)

**Remaining ~47 schemes to reach 100+:**
- [ ] Batch 1 (15 schemes): Monokai Pro, Monokai Soda, Atom One Dark/Light, Nightfly, Srcery, Cobalt2, Jellybeans, Molokai, Wombat, Afterglow, Spacegray, Tender, Flatland, Twilight
- [ ] Batch 2 (18 schemes, dark/light pairs): Modus Vivendi/Operandi, Lucius, Pencil, Seoul256, Xcode, Tango, Vim, Nvim, Zenbones
- [ ] Batch 3 (9 schemes): Everblush, Fairy Floss, Shades of Purple, Synthwave 84, Rosebox, Sakura, Spaceduck, Quiet Light, Rigel
- [ ] Batch 4 (5 schemes, base16): Base16 Default Dark/Light, Base16 Monokai, Base16 Ocean, Base16 Eighties
- [ ] Each batch: verify against canonical source, separate commit, run `all_builtins_produce_valid_palettes` after each

**Conversion tools** (complete): `tools/convert-iterm2.py`, `tools/convert-ghostty.py`, `tools/convert-base16.py`

**Tests:**
- [x] 6 passing: `builtin_schemes_have_valid_rgb`, `builtin_names_unique`, `builtin_names_not_empty` (asserts >= 50), `find_builtin_*`, `all_builtins_produce_valid_palettes`, `discover_all_roundtrip_resolve`
- [ ] Update `builtin_names_not_empty` to assert `>= 100` after reaching target
- [ ] After all batches: verify no submodule exceeds 500 lines

---

## 25.3 Light/Dark Auto-Switch

**Files:** `oriterm/src/scheme/mod.rs` (`parse_conditional`, `resolve_scheme_name`), `oriterm/src/platform/theme/mod.rs` (detection), `oriterm/src/app/config_reload/color_config.rs` (`build_palette_from_config`)

- [x] Config syntax: `scheme = "dark:Tokyo Night, light:Tokyo Night Light"`
- [x] Parse conditional `dark:`/`light:` prefixes; plain names pass through
- [x] System dark/light mode detection (D-Bus, gsettings, KDE, GTK fallback, macOS, Windows registry)
- [x] On system theme change: swap palette via `build_palette_from_config()`, mark all lines dirty
- [ ] Settings dropdown enhancements (dropdown exists via 21.3; these are UX improvements within this section):
  - [ ] Group themes by light/dark/universal
  - [ ] Show "(dark)" / "(light)" label next to theme names

**Tests (39 platform + 12 conditional parsing = 51 passing):**
- [x] Conditional parsing: 7 tests (`parse_conditional_*` covering dark/light, reversed, plain, single-prefix, whitespace, duplicate, space-before-colon)
- [x] Scheme name resolution: 5 tests (`resolve_scheme_name_*` covering plain, conditional dark/light, unknown-defaults-dark, toggle simulation)
- [x] Platform detection: `system_theme_*` (2), `parse_dbus_*` (5), `gtk_*` (6), `classify_*` (12), `gsettings_color_scheme_*` (5), `kdeglobals_*` (7)
- [x] Config integration: `config_foreground_overrides_scheme`

Missing coverage (2 tests):
- [ ] `build_palette_fallback_when_scheme_missing` — fallback to `Palette::for_theme()` when scheme not found (only the found-path is tested)
- [ ] `build_palette_conditional_dark` — conditional scheme + dark theme end-to-end integration

---

## 25.4 Section Completion

- [x] 25.2a `builtin.rs` split complete (prerequisite for 100+ schemes) (completed 2026-04-04)
- [ ] 100+ themes available by name (53 as of 2026-04-04, needs ~47 more)
- [ ] Settings dropdown lists themes with light/dark grouping
- [ ] 4 missing tests written (`discover_count_returns_builtin_count`, `count_themes_nonexistent_dir`, `build_palette_fallback_when_scheme_missing`, `build_palette_conditional_dark`)
- [ ] `builtin_names_not_empty` assertion updated to `>= 100`
- [x] Custom themes loadable from TOML files
- [x] Light/dark auto-switching works
- [x] Theme hot-reload works
- [x] User themes discovered automatically
- [x] Theme conversion scripts for iTerm2/Ghostty/base16

**Exit Criteria:** `colors.scheme = "nord"` in config produces Nord palette. System dark/light mode change auto-switches. 100+ schemes available by name at startup.

**Verification (2026-04-04):** 87 tests pass (33 `scheme/tests.rs` + 15 `loader/tests.rs` + 39 `platform/theme/tests.rs`). 53 built-in schemes verified by counting `BUILTIN_SCHEMES` array entries. `builtin.rs` at 689 lines must split before adding remaining schemes.

---

## Third Party Review

**Agent 3 (2026-04-04):** Verified all facts: 53 schemes, 87 tests, 689-line builtin.rs. Crate boundary correct per crate-boundaries.md. Edits: extracted 25.2a prerequisite subsection for mandatory builtin.rs split, expanded test inventories with specific function names, identified 4 missing tests, restructured remaining schemes into 4 batches with regression gates, added "100+ at startup" to exit criteria.

**Agent 4 (2026-04-04):** Final skeptic review. Mission fulfillment: all 5 goals (100+ themes, TOML files, discovery, live switching, auto-switch) traced to delivering subsections. 3 of 5 fully delivered; 100+ themes blocked on 25.2a/25.2b work; settings dropdown grouping is a UX polish item. Corrections: (1) Removed incorrect `blocked-by:21.5/21.6` on settings dropdown -- the dropdown exists via 21.3, grouping by light/dark is section 25 work that doesn't depend on taskbar jump lists. (2) Condensed completed test inventories from 35+ individual bullets to grouped summaries -- individual function names for completed tests are audit noise, not actionable guidance. Incomplete items kept with full detail. (3) Removed all `<!-- reviewed: ... -->` inline comments from prior agents. (4) Restructured `third_party_review` frontmatter to proper structured format. Sequential cohesion verified: 25.1 (done) -> 25.2a (split, no API change) -> 25.2b (add schemes in batches) -> 25.3 dropdown polish -> 25.4 gates. Codebase stays buildable at every step. No scope gaps found. Plan is honest about difficulty -- adding 47 schemes is tedious but mechanical; the split prerequisite is the only real engineering work remaining.
