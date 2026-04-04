---
section: 25
title: Theme System
status: in-progress
reviewed: false
last_verified: "2026-03-29"
tier: 6
goal: 100+ built-in themes, TOML theme files, discovery, live switching, light/dark auto-switch
sections:
  - id: "25.1"
    title: Theme Format & Loading
    status: complete
  - id: "25.2"
    title: Built-in Theme Library
    status: complete
  - id: "25.3"
    title: "Light/Dark Auto-Switch"
    status: in-progress
  - id: "25.4"
    title: Section Completion
    status: in-progress
---

# Section 25: Theme System

**Status:** In Progress
**Goal:** Ship 100+ built-in themes selectable by name, with automatic light/dark mode switching based on system preference. Theme richness is a strong first impression signal -- users want to personalize their terminal immediately.

**Crate:** `oriterm` (palette + config layer)
**Dependencies:** `serde` (TOML deserialization), existing `toml` crate, platform crates for dark mode detection

**Inspired by:**
- Ghostty: 300+ built-in themes, single-line config, light/dark auto-switch
- iTerm2: hundreds of importable color schemes
- base16: standardized 16-color scheme format used across editors/terminals
- Kitty: theme kitten with preview and selection

---

## 25.1 Theme Format & Loading

Define a theme format and support loading from files.

**File:** `oriterm/src/scheme/mod.rs` (ColorScheme), `oriterm/src/scheme/loader/mod.rs` (TOML loading), `oriterm/src/scheme/builtin.rs` (built-in schemes)

- [x] TOML theme file format (flat format: `ansi = [16 hex strings]`, `foreground`, `background`, `cursor`, optional `selection_foreground`/`selection_background`) (verified 2026-03-29, 48 loader+scheme tests pass)
- [x] `ThemeFile` struct with `Deserialize`
- [x] Parse hex color strings (`#RRGGBB`) to `Rgb`
  - [x] Validate format, return error for malformed strings
- [x] Load themes from:
  - [x] Embedded in binary (54 `const BuiltinScheme` definitions) (verified 2026-03-29)
  - [x] User theme directory: `config_dir/themes/*.toml`
  - [x] Config: `colors.scheme = "nord"` (by name, case-insensitive)
  - [x] Config: `colors.scheme = "/path/to/mytheme.toml"` (by absolute path)
- [x] Theme discovery at startup: (verified 2026-03-29)
  - [x] Scan `config_dir/themes/` for `*.toml` files
  - [x] Parse each, build `Vec<ColorScheme>` of user themes
  - [x] Merge with built-in schemes (user themes can override built-in names)
- [x] Theme hot-reload: (verified 2026-03-29)
  - [x] ConfigMonitor already watches config dir
  - [x] Extend to watch `themes/` subdirectory
  - [x] On theme file change: re-parse and apply if it's the active theme

**Tests:**
- [x] Parse valid TOML theme file to `ColorScheme` (verified 2026-03-29)
- [x] Reject malformed hex colors with descriptive error (verified 2026-03-29, 3 tests: malformed, 3-digit, 0x-prefix)
- [x] Case-insensitive name lookup finds built-in themes (verified 2026-03-29)
- [x] User theme overrides built-in theme with same name (verified 2026-03-29)
- [x] Absolute path loading works for custom theme file (verified 2026-03-29)
- [x] Missing theme file returns error, does not crash (verified 2026-03-29)

---

## 25.2 Built-in Theme Library

Port popular color schemes as embedded themes. Target 50+ built-in.

**File:** `oriterm/src/scheme/builtin.rs` (54 scheme constants, 689 lines -- exceeds 500-line limit but is pure const data)

**54 built-in schemes implemented:** (verified 2026-03-29, exceeds 50+ target)
- [x] Catppuccin Mocha, Latte, Frappe, Macchiato
- [x] One Dark, One Light
- [x] Solarized Dark, Solarized Light
- [x] Dracula
- [x] Tokyo Night, Tokyo Night Storm, Tokyo Night Light
- [x] WezTerm Default
- [x] Gruvbox Dark, Gruvbox Light
- [x] Nord
- [x] Rose Pine, Rose Pine Moon, Rose Pine Dawn
- [x] Everforest Dark, Everforest Light
- [x] Kanagawa, Kanagawa Light
- [x] Ayu Dark, Ayu Light, Ayu Mirage
- [x] Material Dark, Material Light
- [x] Monokai
- [x] Nightfox, Dawnfox, Carbonfox
- [x] GitHub Dark, GitHub Light, GitHub Dimmed
- [x] Snazzy, Tomorrow Night, Tomorrow Light
- [x] Zenburn, Iceberg Dark, Iceberg Light
- [x] Night Owl, Palenight, Horizon, Poimandres, Vesper
- [x] Sonokai, OneDark Pro, Moonfly
- [x] PaperColor Dark, PaperColor Light
- [x] Oxocarbon, Andromeda

**Conversion tools:**
- [x] Script to convert iTerm2 `.itermcolors` XML to TOML format
- [x] Script to convert Ghostty theme format (key=value) to TOML format
- [x] Script to convert base16 YAML to TOML format

**Tests:**
- [x] All built-in schemes have valid RGB values (no out-of-range) (verified 2026-03-29)
- [x] All built-in schemes have unique names (verified 2026-03-29)
- [x] `BUILTIN_SCHEMES` array contains 50+ defined schemes (verified 2026-03-29, asserts >= 50, actual 54)
- [x] `find_builtin()` returns correct scheme for each name (verified 2026-03-29)

---

## 25.3 Light/Dark Auto-Switch

Automatically switch theme based on system appearance.

**File:** `oriterm/src/scheme/mod.rs` (parsing), `oriterm/src/app/mod.rs` (detection + switching), `oriterm/src/app/config_reload.rs` (palette building)

- [x] Config syntax: `scheme = "dark:Tokyo Night, light:Tokyo Night Light"` (verified 2026-03-29, 12 conditional + 39 platform tests pass)
- [x] Parse `scheme` value:
  - [x] If contains `dark:` / `light:` prefixes: conditional theme
  - [x] Otherwise: static theme
- [x] System dark/light mode detection (existing `platform::theme` module) (verified 2026-03-29, 39 platform tests: dbus, gsettings, KDE, GTK)
- [x] On system theme change:
  - [x] Swap palette to the appropriate scheme via `build_palette_from_config()`
  - [x] Mark all grid lines dirty for redraw
- [ ] Settings dropdown improvements: <!-- blocked-by:21 -->
  - [ ] Group themes by light/dark/universal
  - [ ] Show "(dark)" / "(light)" label next to theme names

**Tests:**
- [x] Parse `"dark:X, light:Y"` config syntax correctly (verified 2026-03-29)
- [x] Parse plain `"X"` config syntax as static theme (verified 2026-03-29)
- [x] Reversed order `"light:Y, dark:X"` parses correctly (verified 2026-03-29)
- [x] Extra whitespace handled (verified 2026-03-29)
- [x] Single prefix (e.g. `"dark:X"` without light) returns None (verified 2026-03-29)

---

## 25.4 Section Completion

- [ ] All 25.1-25.3 items complete *(blocked: 25.3 has settings dropdown items pending Section 7)*
- [x] 50+ themes available by name in config (verified 2026-03-29, 54 built-in)
- [x] Custom themes loadable from TOML files in theme directory (verified 2026-03-29)
- [x] Light/dark auto-switching works (verified 2026-03-29)
- [ ] Settings dropdown lists all available themes (built-in + user) <!-- blocked-by:21 -->
- [x] Theme hot-reload works (edit theme file, see change) (verified 2026-03-29)
- [x] User themes in theme directory discovered automatically (verified 2026-03-29)
- [x] Theme conversion scripts for iTerm2/Ghostty/base16 formats (verified 2026-03-29)

**Exit Criteria:** User can type `colors.scheme = "nord"` in config and get the Nord color scheme. System dark/light mode change auto-switches themes.

**Verification Notes (2026-03-29):** 105 tests pass (66 scheme + 39 platform). All checked items verified with passing tests. `builtin.rs` at 689 lines technically exceeds 500-line limit but is pure const data with zero logic -- low-severity hygiene note. Section is accurately tracked; blocked items properly documented.
