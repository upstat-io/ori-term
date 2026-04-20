#!/usr/bin/env python3
"""Generate subcell-precedence-test.ttf — a minimal TrueType font used by the
§11.2 font-precedence tests in oriterm.

The font advertises coverage of one representative codepoint from each of the
six subcell-glyph families covered by §11.2:

    U+256C — BOX DRAWINGS DOUBLE VERTICAL AND HORIZONTAL (box drawing)
    U+2588 — FULL BLOCK                                   (block elements)
    U+259F — QUADRANT UPPER RIGHT AND LOWER LEFT AND LOWER RIGHT (quadrants)
    U+1FB3B — LOWER LEFT AND LOWER RIGHT BLOCK (inverse of sextant 63 — sextant all-6-bits)
    U+1CDE5 — RIGHT BLOCK OCTANT-6 (octant bit pattern 0xFE — all but one)
    U+28FF — BRAILLE PATTERN DOTS-12345678 (all 8 dots)

For each codepoint the font draws a SOLID FILLED EM-SQUARE. This shape is
maximally distinct from the correct Canvas-rendered subcell glyph, so a test
that compares against the canonical Canvas golden will fail unless ori_term
selects the built-in renderer instead of the shaper.

Usage:
    python3 generate_subcell_precedence_test.py

The output path is `subcell-precedence-test.ttf` next to this script.

Deterministic: identical input -> identical output bytes (modulo fontTools
version). Pin the fontTools version when regenerating to preserve byte equality.
"""

from __future__ import annotations

import pathlib
import sys

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen


# Em-square / units-per-em. Matched to JetBrains Mono so the precedence test
# produces the SAME cell metrics as the sparse-golden tests (both fonts hash
# to identical advance width + ascent + descent → identical cell size).
# If these don't match, the precedence test's rendered viewport size will
# differ from the sparse-golden viewport and the shared reference PNGs will
# fail the size-match precheck before the pixel comparison even runs.
UPEM = 1000
# JetBrains Mono metrics (inspected via fontTools on
# oriterm/fonts/JetBrainsMono-Regular.ttf):
#   hhea/OS-2 typo ascent = 1020, descent = -300, lineGap = 0
#   OS/2 winAscent = 1165, winDescent = 400
#   hmtx advance = 600 for every glyph
JBM_ASCENT = 1020
JBM_DESCENT = -300
JBM_WIN_ASCENT = 1165
JBM_WIN_DESCENT = 400
JBM_ADVANCE = 600

# Codepoints the font advertises coverage for — one per subcell-glyph family.
# The glyph names are arbitrary but must be stable across regenerations.
SUBCELL_CODEPOINTS = [
    (0x256C, "uni256C"),    # box drawing
    (0x2588, "uni2588"),    # full block
    (0x259F, "uni259F"),    # quadrant
    (0x1FB3B, "u1FB3B"),    # sextant family
    (0x1CDE5, "u1CDE5"),    # octant family
    (0x28FF, "uni28FF"),    # braille
]


def build_solid_em_square_glyph() -> object:
    """Return a TTGlyph whose outline is a solid filled rectangle matching
    the ink area JetBrains Mono uses (advance width × [descent..ascent]).

    The rectangle fills every pixel that a sane glyph might occupy, so the
    built-in-wins assertion is maximally strong: if the shaper wins, every
    pixel in the glyph cell is alpha=255, which is trivially distinguishable
    from the pattern the built-in Canvas renderer produces."""
    pen = TTGlyphPen(None)
    pen.moveTo((0, JBM_DESCENT))
    pen.lineTo((JBM_ADVANCE, JBM_DESCENT))
    pen.lineTo((JBM_ADVANCE, JBM_ASCENT))
    pen.lineTo((0, JBM_ASCENT))
    pen.closePath()
    return pen.glyph()


def build_empty_glyph() -> object:
    """Return a TTGlyph with no outline (used for .notdef / .null / CR)."""
    pen = TTGlyphPen(None)
    return pen.glyph()


def main(output_path: pathlib.Path) -> None:
    # FontBuilder requires .notdef, .null, and CR as the first three glyphs.
    glyph_order = [".notdef", ".null", "CR", "space"]
    glyphs: dict[str, object] = {
        ".notdef": build_solid_em_square_glyph(),
        ".null": build_empty_glyph(),
        "CR": build_empty_glyph(),
        "space": build_empty_glyph(),
    }

    # Advance widths: match JetBrains Mono's 600/1000 so the harness reports
    # identical cell dimensions when this font is swapped in.
    advance_widths = {
        ".notdef": JBM_ADVANCE,
        ".null": 0,
        "CR": 0,
        "space": JBM_ADVANCE,
    }

    # Character map: codepoint -> glyph name.
    cmap = {
        0x000D: "CR",
        0x0020: "space",
    }

    for codepoint, name in SUBCELL_CODEPOINTS:
        glyph_order.append(name)
        glyphs[name] = build_solid_em_square_glyph()
        advance_widths[name] = JBM_ADVANCE
        cmap[codepoint] = name

    fb = FontBuilder(UPEM, isTTF=True)
    fb.setupGlyphOrder(glyph_order)
    fb.setupCharacterMap(cmap)
    fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics({n: (w, 0) for n, w in advance_widths.items()})
    fb.setupHorizontalHeader(ascent=JBM_ASCENT, descent=JBM_DESCENT, lineGap=0)
    fb.setupOS2(
        sTypoAscender=JBM_ASCENT,
        sTypoDescender=JBM_DESCENT,
        sTypoLineGap=0,
        usWinAscent=JBM_WIN_ASCENT,
        usWinDescent=JBM_WIN_DESCENT,
    )
    fb.setupNameTable({
        "familyName": "Subcell Precedence Test",
        "styleName": "Regular",
        "psName": "SubcellPrecedenceTest-Regular",
    })
    fb.setupPost()

    fb.save(str(output_path))
    print(f"wrote {output_path} ({output_path.stat().st_size} bytes)")


if __name__ == "__main__":
    here = pathlib.Path(__file__).resolve().parent
    out = here / "subcell-precedence-test.ttf"
    main(out)
    sys.exit(0)
