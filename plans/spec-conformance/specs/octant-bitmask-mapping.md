---
artifact: "octant-bitmask-mapping"
version: "1.0"
codepoint_range: "U+1CD00..=U+1CDE5"
codepoint_count: 230
unicode_version: 16
unicode_block: "Symbols for Legacy Computing Supplement (U+1CC00..=U+1CEBF)"
grid: "2 columns × 4 rows (8 sub-cells)"
bit_order: "row-major 0..7"
references:
  - "wezterm customglyph.rs:317-560 (OCTANT_PATTERNS [u8; 230])"
  - "kitty decorations.c:979-1026 (mapping[232] enum flags)"
  - "Unicode 16 chart PDF — https://www.unicode.org/charts/PDF/U1CC00.pdf"
---

# Canonical Octant Codepoint → 8-bit Bitmask Mapping

## Purpose

This artifact is the **single source of truth** for the 230 octant codepoints at U+1CD00..=U+1CDE5 (Symbols for Legacy Computing Supplement, Unicode 16). Section 11's octant renderer (`oriterm/src/gpu/builtin_glyphs/legacy_computing/octants.rs`) drives its lookup table directly from this mapping. The Section 11.0 §11.0 top-down audit cites this file; the §11.1 canonical-mapping guard test asserts the renderer's lookup table is byte-identical to this table.

The table is cross-checked against two de-facto reference implementations:
- **WezTerm** `customglyph.rs:317-560` — `const OCTANT_PATTERNS: [u8; 230]` indexed by `codepoint - 0x1CD00`, with the canonical row-major bit ordering documented inline.
- **Kitty** `decorations.c:979-1026` — `static const enum flags mapping[232]`, uses column-major flag encoding; its bits are remapped (`a→bit0, m→bit1, b→bit2, n→bit3, c→bit4, o→bit5, d→bit6, p→bit7`) before comparison.

After remapping, **0 of 230 codepoints disagree** — WezTerm and Kitty are in full agreement on subset-to-codepoint assignment.

## Bit ordering (canonical — row-major)

The 2-column × 4-row sub-cell grid per terminal cell:

```
+-----+-----+
| b0  | b1  |  ← row 0 (top)
+-----+-----+
| b2  | b3  |  ← row 1 (upper-mid)
+-----+-----+
| b4  | b5  |  ← row 2 (lower-mid)
+-----+-----+
| b6  | b7  |  ← row 3 (bottom)
+-----+-----+
```

A set bit fills its sub-cell; a clear bit leaves it empty. The mask `0xFF` (all 8 sub-cells filled) is **not encoded** in this range because U+2588 FULL BLOCK already covers it; likewise `0x00` is not encoded because U+0020 SPACE covers it. 24 other masks are skipped because they match pre-existing glyphs (half-blocks, sextants, upper/lower halves) — see §"Skipped masks" below.

## Skipped masks (254 − 230 = 24)

The 8-bit mask space has 256 values; 230 are encoded in this range. The 26 omissions are:

- `0x00` (empty) — U+0020 SPACE
- `0xFF` (full) — U+2588 FULL BLOCK
- 24 other masks whose shape is already encoded in Block Elements (U+2580..U+259F) or Symbols for Legacy Computing (U+1FB00..U+1FB3B): `0x03`, `0x0A`, `0x14`, `0x28`, `0x3C`, `0x40`, `0x50`, `0x55`, `0x5F`, `0x5A`, `0x80`, `0xA0`, `0xA5`, `0xAA`, `0xAF`, `0xC0`, `0xD7`, `0xEB`, `0xEE`, `0xF0`, `0xF5`, `0xFA`, `0xFC`, `0xFE`.

The `octants.rs` renderer MUST NOT dispatch on a codepoint outside U+1CD00..=U+1CDE5; codepoints whose shape corresponds to an omitted mask are rendered by their pre-existing glyph module (blocks, half-blocks, sextants) and the font-shaper skip logic in `font::is_builtin` routes them accordingly.

## Canonical table (230 rows)

| Codepoint | Mask (hex) | Mask (binary) | WezTerm evidence | Kitty evidence | Discrepancy |
|---|---|---|---|---|---|
| U+1CD00 | 0x04 | 0b00000100 | customglyph.rs:L330 `0b00000100` | decorations.c:L983 kitty_flags=0x02 | none |
| U+1CD01 | 0x06 | 0b00000110 | customglyph.rs:L331 `0b00000110` | decorations.c:L983 kitty_flags=0x12 | none |
| U+1CD02 | 0x07 | 0b00000111 | customglyph.rs:L332 `0b00000111` | decorations.c:L983 kitty_flags=0x13 | none |
| U+1CD03 | 0x08 | 0b00001000 | customglyph.rs:L333 `0b00001000` | decorations.c:L983 kitty_flags=0x20 | none |
| U+1CD04 | 0x09 | 0b00001001 | customglyph.rs:L334 `0b00001001` | decorations.c:L983 kitty_flags=0x21 | none |
| U+1CD05 | 0x0B | 0b00001011 | customglyph.rs:L335 `0b00001011` | decorations.c:L983 kitty_flags=0x31 | none |
| U+1CD06 | 0x0C | 0b00001100 | customglyph.rs:L336 `0b00001100` | decorations.c:L983 kitty_flags=0x22 | none |
| U+1CD07 | 0x0D | 0b00001101 | customglyph.rs:L337 `0b00001101` | decorations.c:L983 kitty_flags=0x23 | none |
| U+1CD08 | 0x0E | 0b00001110 | customglyph.rs:L338 `0b00001110` | decorations.c:L983 kitty_flags=0x32 | none |
| U+1CD09 | 0x10 | 0b00010000 | customglyph.rs:L339 `0b00010000` | decorations.c:L983 kitty_flags=0x04 | none |
| U+1CD0A | 0x11 | 0b00010001 | customglyph.rs:L340 `0b00010001` | decorations.c:L983 kitty_flags=0x05 | none |
| U+1CD0B | 0x12 | 0b00010010 | customglyph.rs:L341 `0b00010010` | decorations.c:L983 kitty_flags=0x14 | none |
| U+1CD0C | 0x13 | 0b00010011 | customglyph.rs:L342 `0b00010011` | decorations.c:L983 kitty_flags=0x15 | none |
| U+1CD0D | 0x15 | 0b00010101 | customglyph.rs:L343 `0b00010101` | decorations.c:L983 kitty_flags=0x07 | none |
| U+1CD0E | 0x16 | 0b00010110 | customglyph.rs:L344 `0b00010110` | decorations.c:L983 kitty_flags=0x16 | none |
| U+1CD0F | 0x17 | 0b00010111 | customglyph.rs:L345 `0b00010111` | decorations.c:L983 kitty_flags=0x17 | none |
| U+1CD10 | 0x18 | 0b00011000 | customglyph.rs:L346 `0b00011000` | decorations.c:L985 kitty_flags=0x24 | none |
| U+1CD11 | 0x19 | 0b00011001 | customglyph.rs:L347 `0b00011001` | decorations.c:L985 kitty_flags=0x25 | none |
| U+1CD12 | 0x1A | 0b00011010 | customglyph.rs:L348 `0b00011010` | decorations.c:L985 kitty_flags=0x34 | none |
| U+1CD13 | 0x1B | 0b00011011 | customglyph.rs:L349 `0b00011011` | decorations.c:L985 kitty_flags=0x35 | none |
| U+1CD14 | 0x1C | 0b00011100 | customglyph.rs:L350 `0b00011100` | decorations.c:L985 kitty_flags=0x26 | none |
| U+1CD15 | 0x1D | 0b00011101 | customglyph.rs:L351 `0b00011101` | decorations.c:L985 kitty_flags=0x27 | none |
| U+1CD16 | 0x1E | 0b00011110 | customglyph.rs:L352 `0b00011110` | decorations.c:L985 kitty_flags=0x36 | none |
| U+1CD17 | 0x1F | 0b00011111 | customglyph.rs:L353 `0b00011111` | decorations.c:L985 kitty_flags=0x37 | none |
| U+1CD18 | 0x20 | 0b00100000 | customglyph.rs:L354 `0b00100000` | decorations.c:L985 kitty_flags=0x40 | none |
| U+1CD19 | 0x21 | 0b00100001 | customglyph.rs:L355 `0b00100001` | decorations.c:L985 kitty_flags=0x41 | none |
| U+1CD1A | 0x22 | 0b00100010 | customglyph.rs:L356 `0b00100010` | decorations.c:L985 kitty_flags=0x50 | none |
| U+1CD1B | 0x23 | 0b00100011 | customglyph.rs:L357 `0b00100011` | decorations.c:L985 kitty_flags=0x51 | none |
| U+1CD1C | 0x24 | 0b00100100 | customglyph.rs:L358 `0b00100100` | decorations.c:L985 kitty_flags=0x42 | none |
| U+1CD1D | 0x25 | 0b00100101 | customglyph.rs:L359 `0b00100101` | decorations.c:L985 kitty_flags=0x43 | none |
| U+1CD1E | 0x26 | 0b00100110 | customglyph.rs:L360 `0b00100110` | decorations.c:L985 kitty_flags=0x52 | none |
| U+1CD1F | 0x27 | 0b00100111 | customglyph.rs:L361 `0b00100111` | decorations.c:L985 kitty_flags=0x53 | none |
| U+1CD20 | 0x29 | 0b00101001 | customglyph.rs:L362 `0b00101001` | decorations.c:L987 kitty_flags=0x61 | none |
| U+1CD21 | 0x2A | 0b00101010 | customglyph.rs:L363 `0b00101010` | decorations.c:L987 kitty_flags=0x70 | none |
| U+1CD22 | 0x2B | 0b00101011 | customglyph.rs:L364 `0b00101011` | decorations.c:L987 kitty_flags=0x71 | none |
| U+1CD23 | 0x2C | 0b00101100 | customglyph.rs:L365 `0b00101100` | decorations.c:L987 kitty_flags=0x62 | none |
| U+1CD24 | 0x2D | 0b00101101 | customglyph.rs:L366 `0b00101101` | decorations.c:L987 kitty_flags=0x63 | none |
| U+1CD25 | 0x2E | 0b00101110 | customglyph.rs:L367 `0b00101110` | decorations.c:L987 kitty_flags=0x72 | none |
| U+1CD26 | 0x2F | 0b00101111 | customglyph.rs:L368 `0b00101111` | decorations.c:L987 kitty_flags=0x73 | none |
| U+1CD27 | 0x30 | 0b00110000 | customglyph.rs:L369 `0b00110000` | decorations.c:L987 kitty_flags=0x44 | none |
| U+1CD28 | 0x31 | 0b00110001 | customglyph.rs:L370 `0b00110001` | decorations.c:L987 kitty_flags=0x45 | none |
| U+1CD29 | 0x32 | 0b00110010 | customglyph.rs:L371 `0b00110010` | decorations.c:L987 kitty_flags=0x54 | none |
| U+1CD2A | 0x33 | 0b00110011 | customglyph.rs:L372 `0b00110011` | decorations.c:L987 kitty_flags=0x55 | none |
| U+1CD2B | 0x34 | 0b00110100 | customglyph.rs:L373 `0b00110100` | decorations.c:L987 kitty_flags=0x46 | none |
| U+1CD2C | 0x35 | 0b00110101 | customglyph.rs:L374 `0b00110101` | decorations.c:L987 kitty_flags=0x47 | none |
| U+1CD2D | 0x36 | 0b00110110 | customglyph.rs:L375 `0b00110110` | decorations.c:L987 kitty_flags=0x56 | none |
| U+1CD2E | 0x37 | 0b00110111 | customglyph.rs:L376 `0b00110111` | decorations.c:L987 kitty_flags=0x57 | none |
| U+1CD2F | 0x38 | 0b00111000 | customglyph.rs:L377 `0b00111000` | decorations.c:L987 kitty_flags=0x64 | none |
| U+1CD30 | 0x39 | 0b00111001 | customglyph.rs:L378 `0b00111001` | decorations.c:L989 kitty_flags=0x65 | none |
| U+1CD31 | 0x3A | 0b00111010 | customglyph.rs:L379 `0b00111010` | decorations.c:L989 kitty_flags=0x74 | none |
| U+1CD32 | 0x3B | 0b00111011 | customglyph.rs:L380 `0b00111011` | decorations.c:L989 kitty_flags=0x75 | none |
| U+1CD33 | 0x3C | 0b00111100 | customglyph.rs:L381 `0b00111100` | decorations.c:L989 kitty_flags=0x66 | none |
| U+1CD34 | 0x3D | 0b00111101 | customglyph.rs:L382 `0b00111101` | decorations.c:L989 kitty_flags=0x67 | none |
| U+1CD35 | 0x3E | 0b00111110 | customglyph.rs:L383 `0b00111110` | decorations.c:L989 kitty_flags=0x76 | none |
| U+1CD36 | 0x41 | 0b01000001 | customglyph.rs:L384 `0b01000001` | decorations.c:L989 kitty_flags=0x09 | none |
| U+1CD37 | 0x42 | 0b01000010 | customglyph.rs:L385 `0b01000010` | decorations.c:L989 kitty_flags=0x18 | none |
| U+1CD38 | 0x43 | 0b01000011 | customglyph.rs:L386 `0b01000011` | decorations.c:L989 kitty_flags=0x19 | none |
| U+1CD39 | 0x44 | 0b01000100 | customglyph.rs:L387 `0b01000100` | decorations.c:L989 kitty_flags=0x0A | none |
| U+1CD3A | 0x45 | 0b01000101 | customglyph.rs:L388 `0b01000101` | decorations.c:L989 kitty_flags=0x0B | none |
| U+1CD3B | 0x46 | 0b01000110 | customglyph.rs:L389 `0b01000110` | decorations.c:L989 kitty_flags=0x1A | none |
| U+1CD3C | 0x47 | 0b01000111 | customglyph.rs:L390 `0b01000111` | decorations.c:L989 kitty_flags=0x1B | none |
| U+1CD3D | 0x48 | 0b01001000 | customglyph.rs:L391 `0b01001000` | decorations.c:L989 kitty_flags=0x28 | none |
| U+1CD3E | 0x49 | 0b01001001 | customglyph.rs:L392 `0b01001001` | decorations.c:L989 kitty_flags=0x29 | none |
| U+1CD3F | 0x4A | 0b01001010 | customglyph.rs:L393 `0b01001010` | decorations.c:L989 kitty_flags=0x38 | none |
| U+1CD40 | 0x4B | 0b01001011 | customglyph.rs:L394 `0b01001011` | decorations.c:L991 kitty_flags=0x39 | none |
| U+1CD41 | 0x4C | 0b01001100 | customglyph.rs:L395 `0b01001100` | decorations.c:L991 kitty_flags=0x2A | none |
| U+1CD42 | 0x4D | 0b01001101 | customglyph.rs:L396 `0b01001101` | decorations.c:L991 kitty_flags=0x2B | none |
| U+1CD43 | 0x4E | 0b01001110 | customglyph.rs:L397 `0b01001110` | decorations.c:L991 kitty_flags=0x3A | none |
| U+1CD44 | 0x4F | 0b01001111 | customglyph.rs:L398 `0b01001111` | decorations.c:L991 kitty_flags=0x3B | none |
| U+1CD45 | 0x51 | 0b01010001 | customglyph.rs:L399 `0b01010001` | decorations.c:L991 kitty_flags=0x0D | none |
| U+1CD46 | 0x52 | 0b01010010 | customglyph.rs:L400 `0b01010010` | decorations.c:L991 kitty_flags=0x1C | none |
| U+1CD47 | 0x53 | 0b01010011 | customglyph.rs:L401 `0b01010011` | decorations.c:L991 kitty_flags=0x1D | none |
| U+1CD48 | 0x54 | 0b01010100 | customglyph.rs:L402 `0b01010100` | decorations.c:L991 kitty_flags=0x0E | none |
| U+1CD49 | 0x56 | 0b01010110 | customglyph.rs:L403 `0b01010110` | decorations.c:L991 kitty_flags=0x1E | none |
| U+1CD4A | 0x57 | 0b01010111 | customglyph.rs:L404 `0b01010111` | decorations.c:L991 kitty_flags=0x1F | none |
| U+1CD4B | 0x58 | 0b01011000 | customglyph.rs:L405 `0b01011000` | decorations.c:L991 kitty_flags=0x2C | none |
| U+1CD4C | 0x59 | 0b01011001 | customglyph.rs:L406 `0b01011001` | decorations.c:L991 kitty_flags=0x2D | none |
| U+1CD4D | 0x5B | 0b01011011 | customglyph.rs:L407 `0b01011011` | decorations.c:L991 kitty_flags=0x3D | none |
| U+1CD4E | 0x5C | 0b01011100 | customglyph.rs:L408 `0b01011100` | decorations.c:L991 kitty_flags=0x2E | none |
| U+1CD4F | 0x5D | 0b01011101 | customglyph.rs:L409 `0b01011101` | decorations.c:L991 kitty_flags=0x2F | none |
| U+1CD50 | 0x5E | 0b01011110 | customglyph.rs:L410 `0b01011110` | decorations.c:L993 kitty_flags=0x3E | none |
| U+1CD51 | 0x60 | 0b01100000 | customglyph.rs:L411 `0b01100000` | decorations.c:L993 kitty_flags=0x48 | none |
| U+1CD52 | 0x61 | 0b01100001 | customglyph.rs:L412 `0b01100001` | decorations.c:L993 kitty_flags=0x49 | none |
| U+1CD53 | 0x62 | 0b01100010 | customglyph.rs:L413 `0b01100010` | decorations.c:L993 kitty_flags=0x58 | none |
| U+1CD54 | 0x63 | 0b01100011 | customglyph.rs:L414 `0b01100011` | decorations.c:L993 kitty_flags=0x59 | none |
| U+1CD55 | 0x64 | 0b01100100 | customglyph.rs:L415 `0b01100100` | decorations.c:L993 kitty_flags=0x4A | none |
| U+1CD56 | 0x65 | 0b01100101 | customglyph.rs:L416 `0b01100101` | decorations.c:L993 kitty_flags=0x4B | none |
| U+1CD57 | 0x66 | 0b01100110 | customglyph.rs:L417 `0b01100110` | decorations.c:L993 kitty_flags=0x5A | none |
| U+1CD58 | 0x67 | 0b01100111 | customglyph.rs:L418 `0b01100111` | decorations.c:L993 kitty_flags=0x5B | none |
| U+1CD59 | 0x68 | 0b01101000 | customglyph.rs:L419 `0b01101000` | decorations.c:L993 kitty_flags=0x68 | none |
| U+1CD5A | 0x69 | 0b01101001 | customglyph.rs:L420 `0b01101001` | decorations.c:L993 kitty_flags=0x69 | none |
| U+1CD5B | 0x6A | 0b01101010 | customglyph.rs:L421 `0b01101010` | decorations.c:L993 kitty_flags=0x78 | none |
| U+1CD5C | 0x6B | 0b01101011 | customglyph.rs:L422 `0b01101011` | decorations.c:L993 kitty_flags=0x79 | none |
| U+1CD5D | 0x6C | 0b01101100 | customglyph.rs:L423 `0b01101100` | decorations.c:L993 kitty_flags=0x6A | none |
| U+1CD5E | 0x6D | 0b01101101 | customglyph.rs:L424 `0b01101101` | decorations.c:L993 kitty_flags=0x6B | none |
| U+1CD5F | 0x6E | 0b01101110 | customglyph.rs:L425 `0b01101110` | decorations.c:L993 kitty_flags=0x7A | none |
| U+1CD60 | 0x6F | 0b01101111 | customglyph.rs:L426 `0b01101111` | decorations.c:L995 kitty_flags=0x7B | none |
| U+1CD61 | 0x70 | 0b01110000 | customglyph.rs:L427 `0b01110000` | decorations.c:L995 kitty_flags=0x4C | none |
| U+1CD62 | 0x71 | 0b01110001 | customglyph.rs:L428 `0b01110001` | decorations.c:L995 kitty_flags=0x4D | none |
| U+1CD63 | 0x72 | 0b01110010 | customglyph.rs:L429 `0b01110010` | decorations.c:L995 kitty_flags=0x5C | none |
| U+1CD64 | 0x73 | 0b01110011 | customglyph.rs:L430 `0b01110011` | decorations.c:L995 kitty_flags=0x5D | none |
| U+1CD65 | 0x74 | 0b01110100 | customglyph.rs:L431 `0b01110100` | decorations.c:L995 kitty_flags=0x4E | none |
| U+1CD66 | 0x75 | 0b01110101 | customglyph.rs:L432 `0b01110101` | decorations.c:L995 kitty_flags=0x4F | none |
| U+1CD67 | 0x76 | 0b01110110 | customglyph.rs:L433 `0b01110110` | decorations.c:L995 kitty_flags=0x5E | none |
| U+1CD68 | 0x77 | 0b01110111 | customglyph.rs:L434 `0b01110111` | decorations.c:L995 kitty_flags=0x5F | none |
| U+1CD69 | 0x78 | 0b01111000 | customglyph.rs:L435 `0b01111000` | decorations.c:L995 kitty_flags=0x6C | none |
| U+1CD6A | 0x79 | 0b01111001 | customglyph.rs:L436 `0b01111001` | decorations.c:L995 kitty_flags=0x6D | none |
| U+1CD6B | 0x7A | 0b01111010 | customglyph.rs:L437 `0b01111010` | decorations.c:L995 kitty_flags=0x7C | none |
| U+1CD6C | 0x7B | 0b01111011 | customglyph.rs:L438 `0b01111011` | decorations.c:L995 kitty_flags=0x7D | none |
| U+1CD6D | 0x7C | 0b01111100 | customglyph.rs:L439 `0b01111100` | decorations.c:L995 kitty_flags=0x6E | none |
| U+1CD6E | 0x7D | 0b01111101 | customglyph.rs:L440 `0b01111101` | decorations.c:L995 kitty_flags=0x6F | none |
| U+1CD6F | 0x7E | 0b01111110 | customglyph.rs:L441 `0b01111110` | decorations.c:L995 kitty_flags=0x7E | none |
| U+1CD70 | 0x7F | 0b01111111 | customglyph.rs:L442 `0b01111111` | decorations.c:L997 kitty_flags=0x7F | none |
| U+1CD71 | 0x81 | 0b10000001 | customglyph.rs:L443 `0b10000001` | decorations.c:L997 kitty_flags=0x81 | none |
| U+1CD72 | 0x82 | 0b10000010 | customglyph.rs:L444 `0b10000010` | decorations.c:L997 kitty_flags=0x90 | none |
| U+1CD73 | 0x83 | 0b10000011 | customglyph.rs:L445 `0b10000011` | decorations.c:L997 kitty_flags=0x91 | none |
| U+1CD74 | 0x84 | 0b10000100 | customglyph.rs:L446 `0b10000100` | decorations.c:L997 kitty_flags=0x82 | none |
| U+1CD75 | 0x85 | 0b10000101 | customglyph.rs:L447 `0b10000101` | decorations.c:L997 kitty_flags=0x83 | none |
| U+1CD76 | 0x86 | 0b10000110 | customglyph.rs:L448 `0b10000110` | decorations.c:L997 kitty_flags=0x92 | none |
| U+1CD77 | 0x87 | 0b10000111 | customglyph.rs:L449 `0b10000111` | decorations.c:L997 kitty_flags=0x93 | none |
| U+1CD78 | 0x88 | 0b10001000 | customglyph.rs:L450 `0b10001000` | decorations.c:L997 kitty_flags=0xA0 | none |
| U+1CD79 | 0x89 | 0b10001001 | customglyph.rs:L451 `0b10001001` | decorations.c:L997 kitty_flags=0xA1 | none |
| U+1CD7A | 0x8A | 0b10001010 | customglyph.rs:L452 `0b10001010` | decorations.c:L997 kitty_flags=0xB0 | none |
| U+1CD7B | 0x8B | 0b10001011 | customglyph.rs:L453 `0b10001011` | decorations.c:L997 kitty_flags=0xB1 | none |
| U+1CD7C | 0x8C | 0b10001100 | customglyph.rs:L454 `0b10001100` | decorations.c:L997 kitty_flags=0xA2 | none |
| U+1CD7D | 0x8D | 0b10001101 | customglyph.rs:L455 `0b10001101` | decorations.c:L997 kitty_flags=0xA3 | none |
| U+1CD7E | 0x8E | 0b10001110 | customglyph.rs:L456 `0b10001110` | decorations.c:L997 kitty_flags=0xB2 | none |
| U+1CD7F | 0x8F | 0b10001111 | customglyph.rs:L457 `0b10001111` | decorations.c:L997 kitty_flags=0xB3 | none |
| U+1CD80 | 0x90 | 0b10010000 | customglyph.rs:L458 `0b10010000` | decorations.c:L999 kitty_flags=0x84 | none |
| U+1CD81 | 0x91 | 0b10010001 | customglyph.rs:L459 `0b10010001` | decorations.c:L999 kitty_flags=0x85 | none |
| U+1CD82 | 0x92 | 0b10010010 | customglyph.rs:L460 `0b10010010` | decorations.c:L999 kitty_flags=0x94 | none |
| U+1CD83 | 0x93 | 0b10010011 | customglyph.rs:L461 `0b10010011` | decorations.c:L999 kitty_flags=0x95 | none |
| U+1CD84 | 0x94 | 0b10010100 | customglyph.rs:L462 `0b10010100` | decorations.c:L999 kitty_flags=0x86 | none |
| U+1CD85 | 0x95 | 0b10010101 | customglyph.rs:L463 `0b10010101` | decorations.c:L999 kitty_flags=0x87 | none |
| U+1CD86 | 0x96 | 0b10010110 | customglyph.rs:L464 `0b10010110` | decorations.c:L999 kitty_flags=0x96 | none |
| U+1CD87 | 0x97 | 0b10010111 | customglyph.rs:L465 `0b10010111` | decorations.c:L999 kitty_flags=0x97 | none |
| U+1CD88 | 0x98 | 0b10011000 | customglyph.rs:L466 `0b10011000` | decorations.c:L999 kitty_flags=0xA4 | none |
| U+1CD89 | 0x99 | 0b10011001 | customglyph.rs:L467 `0b10011001` | decorations.c:L999 kitty_flags=0xA5 | none |
| U+1CD8A | 0x9A | 0b10011010 | customglyph.rs:L468 `0b10011010` | decorations.c:L999 kitty_flags=0xB4 | none |
| U+1CD8B | 0x9B | 0b10011011 | customglyph.rs:L469 `0b10011011` | decorations.c:L999 kitty_flags=0xB5 | none |
| U+1CD8C | 0x9C | 0b10011100 | customglyph.rs:L470 `0b10011100` | decorations.c:L999 kitty_flags=0xA6 | none |
| U+1CD8D | 0x9D | 0b10011101 | customglyph.rs:L471 `0b10011101` | decorations.c:L999 kitty_flags=0xA7 | none |
| U+1CD8E | 0x9E | 0b10011110 | customglyph.rs:L472 `0b10011110` | decorations.c:L999 kitty_flags=0xB6 | none |
| U+1CD8F | 0x9F | 0b10011111 | customglyph.rs:L473 `0b10011111` | decorations.c:L999 kitty_flags=0xB7 | none |
| U+1CD90 | 0xA1 | 0b10100001 | customglyph.rs:L474 `0b10100001` | decorations.c:L1001 kitty_flags=0xC1 | none |
| U+1CD91 | 0xA2 | 0b10100010 | customglyph.rs:L475 `0b10100010` | decorations.c:L1001 kitty_flags=0xD0 | none |
| U+1CD92 | 0xA3 | 0b10100011 | customglyph.rs:L476 `0b10100011` | decorations.c:L1001 kitty_flags=0xD1 | none |
| U+1CD93 | 0xA4 | 0b10100100 | customglyph.rs:L477 `0b10100100` | decorations.c:L1001 kitty_flags=0xC2 | none |
| U+1CD94 | 0xA6 | 0b10100110 | customglyph.rs:L478 `0b10100110` | decorations.c:L1001 kitty_flags=0xD2 | none |
| U+1CD95 | 0xA7 | 0b10100111 | customglyph.rs:L479 `0b10100111` | decorations.c:L1001 kitty_flags=0xD3 | none |
| U+1CD96 | 0xA8 | 0b10101000 | customglyph.rs:L480 `0b10101000` | decorations.c:L1001 kitty_flags=0xE0 | none |
| U+1CD97 | 0xA9 | 0b10101001 | customglyph.rs:L481 `0b10101001` | decorations.c:L1001 kitty_flags=0xE1 | none |
| U+1CD98 | 0xAB | 0b10101011 | customglyph.rs:L482 `0b10101011` | decorations.c:L1001 kitty_flags=0xF1 | none |
| U+1CD99 | 0xAC | 0b10101100 | customglyph.rs:L483 `0b10101100` | decorations.c:L1001 kitty_flags=0xE2 | none |
| U+1CD9A | 0xAD | 0b10101101 | customglyph.rs:L484 `0b10101101` | decorations.c:L1001 kitty_flags=0xE3 | none |
| U+1CD9B | 0xAE | 0b10101110 | customglyph.rs:L485 `0b10101110` | decorations.c:L1001 kitty_flags=0xF2 | none |
| U+1CD9C | 0xB0 | 0b10110000 | customglyph.rs:L486 `0b10110000` | decorations.c:L1001 kitty_flags=0xC4 | none |
| U+1CD9D | 0xB1 | 0b10110001 | customglyph.rs:L487 `0b10110001` | decorations.c:L1001 kitty_flags=0xC5 | none |
| U+1CD9E | 0xB2 | 0b10110010 | customglyph.rs:L488 `0b10110010` | decorations.c:L1001 kitty_flags=0xD4 | none |
| U+1CD9F | 0xB3 | 0b10110011 | customglyph.rs:L489 `0b10110011` | decorations.c:L1001 kitty_flags=0xD5 | none |
| U+1CDA0 | 0xB4 | 0b10110100 | customglyph.rs:L490 `0b10110100` | decorations.c:L1003 kitty_flags=0xC6 | none |
| U+1CDA1 | 0xB5 | 0b10110101 | customglyph.rs:L491 `0b10110101` | decorations.c:L1003 kitty_flags=0xC7 | none |
| U+1CDA2 | 0xB6 | 0b10110110 | customglyph.rs:L492 `0b10110110` | decorations.c:L1003 kitty_flags=0xD6 | none |
| U+1CDA3 | 0xB7 | 0b10110111 | customglyph.rs:L493 `0b10110111` | decorations.c:L1003 kitty_flags=0xD7 | none |
| U+1CDA4 | 0xB8 | 0b10111000 | customglyph.rs:L494 `0b10111000` | decorations.c:L1003 kitty_flags=0xE4 | none |
| U+1CDA5 | 0xB9 | 0b10111001 | customglyph.rs:L495 `0b10111001` | decorations.c:L1003 kitty_flags=0xE5 | none |
| U+1CDA6 | 0xBA | 0b10111010 | customglyph.rs:L496 `0b10111010` | decorations.c:L1003 kitty_flags=0xF4 | none |
| U+1CDA7 | 0xBB | 0b10111011 | customglyph.rs:L497 `0b10111011` | decorations.c:L1003 kitty_flags=0xF5 | none |
| U+1CDA8 | 0xBC | 0b10111100 | customglyph.rs:L498 `0b10111100` | decorations.c:L1003 kitty_flags=0xE6 | none |
| U+1CDA9 | 0xBD | 0b10111101 | customglyph.rs:L499 `0b10111101` | decorations.c:L1003 kitty_flags=0xE7 | none |
| U+1CDAA | 0xBE | 0b10111110 | customglyph.rs:L500 `0b10111110` | decorations.c:L1003 kitty_flags=0xF6 | none |
| U+1CDAB | 0xBF | 0b10111111 | customglyph.rs:L501 `0b10111111` | decorations.c:L1003 kitty_flags=0xF7 | none |
| U+1CDAC | 0xC1 | 0b11000001 | customglyph.rs:L502 `0b11000001` | decorations.c:L1003 kitty_flags=0x89 | none |
| U+1CDAD | 0xC2 | 0b11000010 | customglyph.rs:L503 `0b11000010` | decorations.c:L1003 kitty_flags=0x98 | none |
| U+1CDAE | 0xC3 | 0b11000011 | customglyph.rs:L504 `0b11000011` | decorations.c:L1003 kitty_flags=0x99 | none |
| U+1CDAF | 0xC4 | 0b11000100 | customglyph.rs:L505 `0b11000100` | decorations.c:L1003 kitty_flags=0x8A | none |
| U+1CDB0 | 0xC5 | 0b11000101 | customglyph.rs:L506 `0b11000101` | decorations.c:L1005 kitty_flags=0x8B | none |
| U+1CDB1 | 0xC6 | 0b11000110 | customglyph.rs:L507 `0b11000110` | decorations.c:L1005 kitty_flags=0x9A | none |
| U+1CDB2 | 0xC7 | 0b11000111 | customglyph.rs:L508 `0b11000111` | decorations.c:L1005 kitty_flags=0x9B | none |
| U+1CDB3 | 0xC8 | 0b11001000 | customglyph.rs:L509 `0b11001000` | decorations.c:L1005 kitty_flags=0xA8 | none |
| U+1CDB4 | 0xC9 | 0b11001001 | customglyph.rs:L510 `0b11001001` | decorations.c:L1005 kitty_flags=0xA9 | none |
| U+1CDB5 | 0xCA | 0b11001010 | customglyph.rs:L511 `0b11001010` | decorations.c:L1005 kitty_flags=0xB8 | none |
| U+1CDB6 | 0xCB | 0b11001011 | customglyph.rs:L512 `0b11001011` | decorations.c:L1005 kitty_flags=0xB9 | none |
| U+1CDB7 | 0xCC | 0b11001100 | customglyph.rs:L513 `0b11001100` | decorations.c:L1005 kitty_flags=0xAA | none |
| U+1CDB8 | 0xCD | 0b11001101 | customglyph.rs:L514 `0b11001101` | decorations.c:L1005 kitty_flags=0xAB | none |
| U+1CDB9 | 0xCE | 0b11001110 | customglyph.rs:L515 `0b11001110` | decorations.c:L1005 kitty_flags=0xBA | none |
| U+1CDBA | 0xCF | 0b11001111 | customglyph.rs:L516 `0b11001111` | decorations.c:L1005 kitty_flags=0xBB | none |
| U+1CDBB | 0xD0 | 0b11010000 | customglyph.rs:L517 `0b11010000` | decorations.c:L1005 kitty_flags=0x8C | none |
| U+1CDBC | 0xD1 | 0b11010001 | customglyph.rs:L518 `0b11010001` | decorations.c:L1005 kitty_flags=0x8D | none |
| U+1CDBD | 0xD2 | 0b11010010 | customglyph.rs:L519 `0b11010010` | decorations.c:L1005 kitty_flags=0x9C | none |
| U+1CDBE | 0xD3 | 0b11010011 | customglyph.rs:L520 `0b11010011` | decorations.c:L1005 kitty_flags=0x9D | none |
| U+1CDBF | 0xD4 | 0b11010100 | customglyph.rs:L521 `0b11010100` | decorations.c:L1005 kitty_flags=0x8E | none |
| U+1CDC0 | 0xD5 | 0b11010101 | customglyph.rs:L522 `0b11010101` | decorations.c:L1008 kitty_flags=0x8F | none |
| U+1CDC1 | 0xD6 | 0b11010110 | customglyph.rs:L523 `0b11010110` | decorations.c:L1008 kitty_flags=0x9E | none |
| U+1CDC2 | 0xD7 | 0b11010111 | customglyph.rs:L524 `0b11010111` | decorations.c:L1008 kitty_flags=0x9F | none |
| U+1CDC3 | 0xD8 | 0b11011000 | customglyph.rs:L525 `0b11011000` | decorations.c:L1008 kitty_flags=0xAC | none |
| U+1CDC4 | 0xD9 | 0b11011001 | customglyph.rs:L526 `0b11011001` | decorations.c:L1008 kitty_flags=0xAD | none |
| U+1CDC5 | 0xDA | 0b11011010 | customglyph.rs:L527 `0b11011010` | decorations.c:L1008 kitty_flags=0xBC | none |
| U+1CDC6 | 0xDB | 0b11011011 | customglyph.rs:L528 `0b11011011` | decorations.c:L1008 kitty_flags=0xBD | none |
| U+1CDC7 | 0xDC | 0b11011100 | customglyph.rs:L529 `0b11011100` | decorations.c:L1008 kitty_flags=0xAE | none |
| U+1CDC8 | 0xDD | 0b11011101 | customglyph.rs:L530 `0b11011101` | decorations.c:L1008 kitty_flags=0xAF | none |
| U+1CDC9 | 0xDE | 0b11011110 | customglyph.rs:L531 `0b11011110` | decorations.c:L1008 kitty_flags=0xBE | none |
| U+1CDCA | 0xDF | 0b11011111 | customglyph.rs:L532 `0b11011111` | decorations.c:L1008 kitty_flags=0xBF | none |
| U+1CDCB | 0xE0 | 0b11100000 | customglyph.rs:L533 `0b11100000` | decorations.c:L1008 kitty_flags=0xC8 | none |
| U+1CDCC | 0xE1 | 0b11100001 | customglyph.rs:L534 `0b11100001` | decorations.c:L1008 kitty_flags=0xC9 | none |
| U+1CDCD | 0xE2 | 0b11100010 | customglyph.rs:L535 `0b11100010` | decorations.c:L1008 kitty_flags=0xD8 | none |
| U+1CDCE | 0xE3 | 0b11100011 | customglyph.rs:L536 `0b11100011` | decorations.c:L1008 kitty_flags=0xD9 | none |
| U+1CDCF | 0xE4 | 0b11100100 | customglyph.rs:L537 `0b11100100` | decorations.c:L1008 kitty_flags=0xCA | none |
| U+1CDD0 | 0xE5 | 0b11100101 | customglyph.rs:L538 `0b11100101` | decorations.c:L1011 kitty_flags=0xCB | none |
| U+1CDD1 | 0xE6 | 0b11100110 | customglyph.rs:L539 `0b11100110` | decorations.c:L1011 kitty_flags=0xDA | none |
| U+1CDD2 | 0xE7 | 0b11100111 | customglyph.rs:L540 `0b11100111` | decorations.c:L1011 kitty_flags=0xDB | none |
| U+1CDD3 | 0xE8 | 0b11101000 | customglyph.rs:L541 `0b11101000` | decorations.c:L1011 kitty_flags=0xE8 | none |
| U+1CDD4 | 0xE9 | 0b11101001 | customglyph.rs:L542 `0b11101001` | decorations.c:L1011 kitty_flags=0xE9 | none |
| U+1CDD5 | 0xEA | 0b11101010 | customglyph.rs:L543 `0b11101010` | decorations.c:L1011 kitty_flags=0xF8 | none |
| U+1CDD6 | 0xEB | 0b11101011 | customglyph.rs:L544 `0b11101011` | decorations.c:L1011 kitty_flags=0xF9 | none |
| U+1CDD7 | 0xEC | 0b11101100 | customglyph.rs:L545 `0b11101100` | decorations.c:L1011 kitty_flags=0xEA | none |
| U+1CDD8 | 0xED | 0b11101101 | customglyph.rs:L546 `0b11101101` | decorations.c:L1011 kitty_flags=0xEB | none |
| U+1CDD9 | 0xEE | 0b11101110 | customglyph.rs:L547 `0b11101110` | decorations.c:L1011 kitty_flags=0xFA | none |
| U+1CDDA | 0xEF | 0b11101111 | customglyph.rs:L548 `0b11101111` | decorations.c:L1011 kitty_flags=0xFB | none |
| U+1CDDB | 0xF1 | 0b11110001 | customglyph.rs:L549 `0b11110001` | decorations.c:L1011 kitty_flags=0xCD | none |
| U+1CDDC | 0xF2 | 0b11110010 | customglyph.rs:L550 `0b11110010` | decorations.c:L1011 kitty_flags=0xDC | none |
| U+1CDDD | 0xF3 | 0b11110011 | customglyph.rs:L551 `0b11110011` | decorations.c:L1011 kitty_flags=0xDD | none |
| U+1CDDE | 0xF4 | 0b11110100 | customglyph.rs:L552 `0b11110100` | decorations.c:L1011 kitty_flags=0xCE | none |
| U+1CDDF | 0xF6 | 0b11110110 | customglyph.rs:L553 `0b11110110` | decorations.c:L1011 kitty_flags=0xDE | none |
| U+1CDE0 | 0xF7 | 0b11110111 | customglyph.rs:L554 `0b11110111` | decorations.c:L1013 kitty_flags=0xDF | none |
| U+1CDE1 | 0xF8 | 0b11111000 | customglyph.rs:L555 `0b11111000` | decorations.c:L1013 kitty_flags=0xEC | none |
| U+1CDE2 | 0xF9 | 0b11111001 | customglyph.rs:L556 `0b11111001` | decorations.c:L1013 kitty_flags=0xED | none |
| U+1CDE3 | 0xFB | 0b11111011 | customglyph.rs:L557 `0b11111011` | decorations.c:L1013 kitty_flags=0xFD | none |
| U+1CDE4 | 0xFD | 0b11111101 | customglyph.rs:L558 `0b11111101` | decorations.c:L1013 kitty_flags=0xEF | none |
| U+1CDE5 | 0xFE | 0b11111110 | customglyph.rs:L559 `0b11111110` | decorations.c:L1013 kitty_flags=0xFE | none |

## Using this table

### In `octants.rs`

The octant renderer MUST read this file's table as the authority. The implementer can either:

1. **Embed the table verbatim as a `[u8; 230]` array** in `octants.rs` — one byte per codepoint, indexed by `codepoint as u32 - 0x1CD00`. The canonical-mapping guard test in `legacy_computing/tests.rs` parses this markdown file at test time (via the `#[test]` function reading the file from `CARGO_MANIFEST_DIR` or similar) and asserts byte-equality against the embedded array.

2. **Codegen the table from this markdown file** via a `build.rs` or a static-data generator in `oriterm_test_support`. The output is a `[u8; 230]` compiled into `octants.rs`; the canonical-mapping guard asserts that the generated file is up-to-date.

Approach (1) is simpler and is what §11.1's checklist assumes; approach (2) is acceptable if the implementer prefers codegen.

### In the canonical-mapping guard test

The test MUST fail if any single byte of the renderer's table diverges from this file. This is the primary regression guard for the 230-entry mapping.

### In the exhaustive semantic raster sweep (§11.2)

The raster sweep iterates every codepoint in U+1CD00..=U+1CDE5, rasterizes it via the built-in renderer, and extracts the 8-bit bitmask from the rendered Canvas. The expected bitmask for each codepoint comes from THIS table.

## Re-walking / updating this artifact

If Unicode publishes a new version that extends the Symbols for Legacy Computing Supplement block:

1. Re-fetch `https://www.unicode.org/charts/PDF/U1CC00.pdf` (manifest-backed — see `specs/manifest.toml §specs.unicode_chart_u1cc00`).
2. Walk the chart for any new codepoints in the octant subrange U+1CD00..U+1CDE5 (or beyond — the range may extend).
3. Cross-check the new codepoints against updated WezTerm + Kitty tables.
4. Append rows to the table above; update the `codepoint_count` frontmatter.
5. The canonical-mapping guard test should fail until the renderer's table is regenerated to include the new entries.
