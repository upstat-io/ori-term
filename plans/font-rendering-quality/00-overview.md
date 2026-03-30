---
plan: "font-rendering-quality"
title: "Font Rendering Quality: Exhaustive Implementation Plan"
status: not-started
references:
  - "plans/bug-tracker/section-04-fonts.md"
---

# Font Rendering Quality: Exhaustive Implementation Plan

## Mission

Fix three font rendering bugs that cause blurriness and visual artifacts, then expose font rendering settings as user-configurable options in the Settings dialog so users can tune hinting, subpixel rendering, subpixel positioning, and atlas filtering to their display and preferences.

## Architecture

```
TOML Config (font_config.rs)
  ├── hinting: Option<String>           ← already exists
  ├── subpixel_mode: Option<String>     ← already exists
  ├── subpixel_positioning: Option<bool>  ← CHANGE type from bool (None=auto)
  └── atlas_filtering: Option<String>   ← NEW

Config Reload (config_reload/font_config.rs)
  ├── resolve_hinting()                 ← already exists
  ├── resolve_subpixel_mode()           ← already exists
  ├── resolve_subpixel_positioning()    ← NEW (reads config, auto-detect fallback)
  └── resolve_atlas_filtering()         ← NEW (reads config, auto-detect fallback)

Config Reload (config_reload/mod.rs)
  └── apply_font_changes()             ← FIX: add change detection for new fields

GPU Renderer
  ├── WindowRenderer.set_hinting_and_format()  ← FIX: stop overwriting UI font settings
  ├── atlas/texture.rs upload_glyph()          ← FIX: zero gutter texels on upload
  ├── prepare/mod.rs fill_frame_shaped()       ← FIX: round per-row grid Y positions
  ├── prepare/dirty_skip/mod.rs                ← FIX: round per-row grid Y (incremental path)
  ├── prepare/emit.rs all Y computations       ← FIX: round Y in cursor/markers/underlines
  ├── bind_groups/mod.rs AtlasBindGroup        ← EXTEND: configurable sampler FilterMode + AtlasFiltering enum
  ├── prepare/emit.rs subpx_bin()              ← EXTEND: honor subpixel_positioning=false
  ├── scene_convert/mod.rs TextContext         ← EXTEND: add subpixel_positioning field
  ├── scene_convert/text.rs subpx_bin()        ← EXTEND: honor subpixel_positioning=false (UI text)
  └── window_renderer/helpers.rs raster keys   ← EXTEND: honor subpixel_positioning=false (pre-caching)

Settings Dialog (Font page)
  └── form_builder/font.rs
      └── build_advanced_section()      ← NEW: 4 dropdowns with Auto defaults
          ├── Hinting:         Auto (Full) / Full / None
          ├── Subpixel AA:     Auto (RGB)  / RGB / BGR / None (Grayscale)
          ├── Subpixel pos.:   Auto (Quarter-pixel) / Quarter-pixel / None
          └── Atlas filtering: Auto (Linear) / Linear / Nearest

  build_settings_dialog()               ← CHANGE: add scale_factor + opacity params
```

## Design Principles

**Auto-detect with override.** Every setting has an `Auto` default that uses the current scale-factor-based detection logic. Users who never touch Advanced settings get the same quality as today (plus bug fixes). Power users get full control.

**Fix bugs first, then expose settings.** Section 01 fixes the three TPR bugs independently of the settings UI. Section 02 adds user-configurable controls. This way bug fixes ship even if settings work is deferred.

## Section Dependency Graph

```
Section 01 (Bug Fixes) ──┐
                          ├──→ Section 03 (Verification)
Section 02 (Settings)  ───┘
```

- Sections 01 and 02 are **independent** — neither requires the other.
- Section 03 requires both 01 and 02.

## Implementation Sequence

```
Phase 1 - Bug Fixes
  └─ Section 01: Fix TPR-04-006, TPR-04-008, TPR-04-010
  Gate: ./build-all.sh + ./clippy-all.sh + ./test-all.sh green

Phase 2 - Settings UI
  └─ Section 02: Config wiring + Font page Advanced section + action handler
  Gate: All 4 settings functional in dialog, config persists, renderer responds

Phase 3 - Verification
  └─ Section 03: Test matrix, build verification, bug tracker updates
  Gate: Full test suite green, all TPR items resolved or tracked
```

**Why this order:**
- Phase 1 fixes visible bugs with no UI changes — pure correctness.
- Phase 2 builds on the fixed rendering pipeline to expose correct controls.
- Phase 3 verifies the complete system.

## Metrics (Current State)

| File | Lines | Role |
|------|-------|------|
| `config/font_config.rs` | 140 | Config struct + TOML parsing |
| `config_reload/font_config.rs` | 183 | Config resolution + application |
| `config_reload/mod.rs` | 447 | Config hot-reload dispatch (change detection) |
| `font/mod.rs` | 493 | HintingMode, SubpixelMode, GlyphFormat, subpx_bin enums |
| `gpu/window_renderer/font_config.rs` | 164 | Font size/hinting/format setters |
| `gpu/window_renderer/mod.rs` | 488 | WindowRenderer struct + bind group rebuild |
| `gpu/atlas/mod.rs` | 579 | Atlas packing, clear, eviction |
| `gpu/atlas/texture.rs` | 82 | Texture upload |
| `gpu/bind_groups/mod.rs` | 191 | Atlas bind group + sampler |
| `gpu/prepare/mod.rs` | 487 | Frame preparation (grid Y, shaped path) |
| `gpu/prepare/emit.rs` | 280 | Glyph emission, cursor, markers, URL underlines |
| `gpu/prepare/unshaped.rs` | 194 | Test-only unshaped path |
| `gpu/prepare/dirty_skip/mod.rs` | 497 | Incremental frame preparation (also has grid Y) |
| `gpu/scene_convert/mod.rs` | 366 | Scene conversion dispatch + TextContext struct |
| `gpu/scene_convert/text.rs` | 194 | UI text scene conversion (has Y rounding) |
| `gpu/window_renderer/helpers.rs` | 467 | Grid/UI raster key generation (subpx_bin) |
| `gpu/window_renderer/scene_append.rs` | 195 | Scene append (TextContext construction) |
| `settings_overlay/form_builder/font.rs` | 177 | Font page builder |
| `settings_overlay/form_builder/rendering.rs` | 101 | Rendering page builder |
| `settings_overlay/form_builder/mod.rs` | 235 | SettingsIds + page routing |
| `settings_overlay/action_handler/mod.rs` | 264 | Settings action dispatch |

## Estimated Effort

| Section | Est. Lines Changed | Est. Test Lines | Complexity | Depends On |
|---------|-------------------|----------------|------------|------------|
| 01 Bug Fixes | ~90 | ~180 | Low-Medium | — |
| 02 Advanced Settings | ~420 | ~400 | Medium | — |
| 03 Verification | ~20 | — | Low | 01, 02 |
| **Total** | **~530** | **~580** | | |

## Known Bugs (Pre-existing)

| Bug | Root Cause | Fix Location | Status |
|-----|-----------|-------------|--------|
| TPR-04-006: UI font wrong after DPI change | `set_hinting_and_format()` overwrites UI font's Alpha/None with terminal settings | Section 01 | Not Started |
| TPR-04-008: Stale atlas gutter texels | `clear()`/`evict_page()` reset packer but don't zero texture memory | Section 01 | Not Started |
| TPR-04-010: Grid text Y not rounded | `oy + row * ch` is fractional on non-integer scale factors | Section 01 | Not Started |
| TPR-04-007: `subpixel_positioning` dead config | Parsed but never consumed by renderer | Section 02 | Not Started |
| TPR-04-011: Atlas sampler hardcoded Linear | Users cannot choose Nearest for pixel-perfect rendering | Section 02 (atlas_filtering setting) | Not Started |
| BUG-04-002: Tab bar blur after DPI change | Root cause is TPR-04-006 | Section 01 | Not Started |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Bug Fixes | `section-01-bug-fixes.md` | Not Started |
| 02 | Advanced Font Rendering Settings | `section-02-advanced-settings.md` | Not Started |
| 03 | Verification | `section-03-verification.md` | Not Started |
