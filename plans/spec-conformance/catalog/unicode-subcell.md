---
schema_version: "1.0"
stack: unicode_subcell
title: "Unicode Subcell Glyphs Catalog"
owner_section: "01 (bootstrap), 11 (verification)"
---

# Unicode Subcell Glyphs Catalog

Subcell / block-drawing / quadrant / sextant / octant / braille glyphs from the Unicode Symbols for Legacy Computing block (U+1FB00–U+1FBFF) and adjacent ranges. Rendered by `oriterm/src/gpu/builtin_glyphs/` directly without going through the font pipeline.

Section 11 (Unicode Subcell Glyphs incl. octants) verifies that every supported glyph renders pixel-perfect.

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| USC-BLOCKS | Unicode block (Block Elements U+2580–U+259F) | Printable Unicode chars U+2580..U+259F | Upper/lower/left/right half blocks and light/medium/dark shading | `` `oriterm/src/gpu/builtin_glyphs/` — built-in raster atlas; dispatched by `Term::input` + grid's `put_char` `` | gpu-instance | parser:pending dispatch:pending state:pending frame-input:pending gpu:pending | implemented-unverified | — | Section 11 drives the GPU-instance rung to `verified`. |
| USC-BOX | Unicode block (Box Drawing U+2500–U+257F) | Printable Unicode chars U+2500..U+257F | Box drawing characters (light / heavy / double / round corners) | `` `oriterm/src/gpu/builtin_glyphs/` — built-in raster atlas `` | gpu-instance | parser:pending dispatch:pending state:pending frame-input:pending gpu:pending | implemented-unverified | — | |
| USC-BRAILLE | Unicode block (Braille Patterns U+2800–U+28FF) | Printable Unicode chars U+2800..U+28FF | 8-dot Braille patterns (used by notcurses, btop, etc.) | `` `oriterm/src/gpu/builtin_glyphs/` `` | gpu-instance | parser:pending dispatch:pending state:pending frame-input:pending gpu:pending | implemented-unverified | — | |
| USC-LEGACY-SEXTANT | Unicode block (Symbols for Legacy Computing U+1FB00–U+1FBFF) | Printable Unicode chars U+1FB00..U+1FB3B (sextants) | Sextant subcell blocks (2×3 grid, 6-bit bitmask) | `` `oriterm/src/gpu/builtin_glyphs/legacy_computing/mod.rs` — built-in raster atlas `` | gpu-instance | parser:pending dispatch:pending state:pending frame-input:pending gpu:pending | implemented-unverified | — | Renamed from USC-LEGACY-QUADRANT — the U+1FB00..U+1FB3B range holds sextants; quadrants (U+2596..U+259F) are covered by USC-BLOCKS. |
| USC-LEGACY-OCTANT | Unicode block (Symbols for Legacy Computing Supplement U+1CC00–U+1CEBF) | Printable Unicode chars U+1CD00..U+1CDE5 (octants, 230 codepoints) | Octant subcell blocks (Unicode 16, 2×4 grid, 8-bit bitmask) | MISSING — to be added by Section 11 (Unicode Subcell Glyphs) | gpu-instance | parser:pending dispatch:pending state:pending frame-input:pending gpu:pending | missing | — | Required for full notcurses-demo fidelity. Octants occupy the U+1CD00..U+1CDE5 subrange of the Symbols for Legacy Computing Supplement block. |
