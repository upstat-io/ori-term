# Section 01 Reconciliation Report

schema_version: "0.1-provisional"
generated: 2026-04-12

## Summary

- Bottom-up tuple count: 72
- Top-down tuple count: 87
- Capture tuple count: 43
- Reconciled (in spec + code/captures): 228
- De-facto (in code, not in spec): 17
- MISSING (in spec, not in code): 38
- Capture-only (in captures, not in code or spec): 6

## De-facto rows (in code/captures, no spec source)

| Signature | Reason |
|---|---|
| (APC, [], ST) | In dispatch but no primary spec row |
| (CSI, [], `) | In dispatch but no primary spec row |
| (CSI, [], a) | In dispatch but no primary spec row |
| (CSI, [], e) | In dispatch but no primary spec row |
| (CSI, [37], m) | In dispatch but no primary spec row |
| (CSI, [60], u) | In dispatch but no primary spec row |
| (CSI, [62], q) | In dispatch but no primary spec row |
| (CSI, [63], S) | In dispatch but no primary spec row |
| (DA, [40], 0) | In dispatch but no primary spec row |
| (DA, [41], 0) | In dispatch but no primary spec row |
| (DA, [41], B) | In dispatch but no primary spec row |
| (DA, [42], 0) | In dispatch but no primary spec row |
| (DA, [42], B) | In dispatch but no primary spec row |
| (DA, [43], 0) | In dispatch but no primary spec row |
| (DA, [43], B) | In dispatch but no primary spec row |
| (DCS, [], z) | In dispatch but no primary spec row |
| (DCS, [43], q) | In dispatch but no primary spec row |

## MISSING rows (in spec, not dispatched)

| Row ID | Catalog file | Spec source | Owner |
|---|---|---|---|
| PRINT-MC-ON | plans/spec-conformance/catalog/audio-print.md | ECMA-48 §8.3.82 | Section 20 |
| PRINT-MC-OFF | plans/spec-conformance/catalog/audio-print.md | ECMA-48 §8.3.82 | Section 20 |
| CHSET-LS2 | plans/spec-conformance/catalog/charsets.md | ECMA-48 §8.3.78 | Section 18 |
| CHSET-LS3 | plans/spec-conformance/catalog/charsets.md | ECMA-48 §8.3.79 | Section 18 |
| CHSET-LS1R | plans/spec-conformance/catalog/charsets.md | ECMA-48 §8.3.76 | Section 18 |
| CHSET-LS2R | plans/spec-conformance/catalog/charsets.md | ECMA-48 §8.3.77 | Section 18 |
| CHSET-LS3R | plans/spec-conformance/catalog/charsets.md | ECMA-48 §8.3.75 | Section 18 |
| ECMA48-CSI-DA1 | plans/spec-conformance/catalog/ecma-48.md | ECMA-48 §8.3.24 | Section — |
| ECMA48-CSI-DA2 | plans/spec-conformance/catalog/ecma-48.md | xterm ctlseqs (DA2) | Section — |
| ECMA48-CSI-DA3 | plans/spec-conformance/catalog/ecma-48.md | xterm ctlseqs (DA3) | Section — |
| ECMA48-CSI-DECSTR | plans/spec-conformance/catalog/ecma-48.md | DEC STD 070 §4.6.9 | Section 08 |
| ECMA48-CSI-DECSED | plans/spec-conformance/catalog/ecma-48.md | DEC STD 070 §4.6.3 | Section 08 |
| ECMA48-CSI-DECSEL | plans/spec-conformance/catalog/ecma-48.md | DEC STD 070 §4.6.3 | Section 08 |
| ECMA48-CSI-SL | plans/spec-conformance/catalog/ecma-48.md | ECMA-48 §8.3.121 | Section 08 |
| ECMA48-CSI-SR | plans/spec-conformance/catalog/ecma-48.md | ECMA-48 §8.3.122 | Section 08 |
| ECMA48-DCS-DECRQSS | plans/spec-conformance/catalog/ecma-48.md | DEC STD 070 §6.1.2 | Section — |
| ECMA48-DCS-DECRQSS-DECSLRM | plans/spec-conformance/catalog/ecma-48.md | DEC STD 070 §6.1.2 | Section 08 |
| ECMA48-PM-DISCARD | plans/spec-conformance/catalog/ecma-48.md | ECMA-48 §5.6 | Section — |
| ECMA48-SOS-DISCARD | plans/spec-conformance/catalog/ecma-48.md | ECMA-48 §5.6 | Section — |
| HIST-VT52-CURSOR | plans/spec-conformance/catalog/historical.md | DEC VT52 user manual | Section 19 |
| HIST-REGIS | plans/spec-conformance/catalog/historical.md | DEC STD 070 §6.4 (ReGIS) | Section 26 |
| HIST-TMUX-CONTROL | plans/spec-conformance/catalog/historical.md | tmux control mode | Section 19 |
| KG-QUERY | plans/spec-conformance/catalog/kitty-graphics.md | Kitty graphics-protocol | Section — |
| KG-TRANSMIT | plans/spec-conformance/catalog/kitty-graphics.md | Kitty graphics-protocol | Section — |
| KG-TRANSMIT-PLACE | plans/spec-conformance/catalog/kitty-graphics.md | Kitty graphics-protocol | Section — |
| KG-PLACE | plans/spec-conformance/catalog/kitty-graphics.md | Kitty graphics-protocol | Section — |
| KG-DELETE | plans/spec-conformance/catalog/kitty-graphics.md | Kitty graphics-protocol | Section — |
| KG-FRAME | plans/spec-conformance/catalog/kitty-graphics.md | Kitty graphics-protocol (animation) | Section — |
| KG-ANIMATE | plans/spec-conformance/catalog/kitty-graphics.md | Kitty graphics-protocol (animation) | Section — |
| KG-RESPONSE | plans/spec-conformance/catalog/kitty-graphics.md | Kitty graphics-protocol (response) | Section — |
| SIXEL-DCS-q | plans/spec-conformance/catalog/sixel.md | DEC STD 070 §6.3 + VT340 sixel | Section — |
| SIXEL-DCS-put | plans/spec-conformance/catalog/sixel.md | DEC STD 070 §6.3 | Section — |
| SIXEL-DCS-unhook | plans/spec-conformance/catalog/sixel.md | DEC STD 070 §6.3 | Section — |
| XT-PUSHCOLORS | plans/spec-conformance/catalog/xterm-ctlseqs.md | xterm ctlseqs XTPUSHCOLORS | Section 10 |
| XT-POPCOLORS | plans/spec-conformance/catalog/xterm-ctlseqs.md | xterm ctlseqs XTPOPCOLORS | Section 10 |
| XT-REPORTCOLORS | plans/spec-conformance/catalog/xterm-ctlseqs.md | xterm ctlseqs XTREPORTCOLORS | Section 10 |
| XT-PUSHSGR | plans/spec-conformance/catalog/xterm-ctlseqs.md | xterm ctlseqs XTPUSHSGR | Section 08 |
| XT-POPSGR | plans/spec-conformance/catalog/xterm-ctlseqs.md | xterm ctlseqs XTPOPSGR | Section 08 |

## Capture-only rows (in captures, not in code or spec)

| Signature | Reason |
|---|---|
| (APC, [], ST) | Seen in captures only |
| (CSI, [37], m) | Seen in captures only |
| (CSI, [62], q) | Seen in captures only |
| (CSI, [63], S) | Seen in captures only |
| (DCS, [], z) | Seen in captures only |
| (DCS, [43], q) | Seen in captures only |

