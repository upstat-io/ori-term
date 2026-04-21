---
title: "xterm reference cheatsheet — feature → file:line"
purpose: "Quick lookup for xterm source locations when implementing spec-conformance sections"
source_repo: "~/projects/reference_repos/console_repos/xterm/"
maintained: "append-only; grep for the feature or drop the row when the reference moves"
---

# xterm Reference Cheatsheet

Shortcuts to the exact file:line in the xterm source tree that implement each VT/DEC feature ori_term is trying to match. Populated on demand during spec-conformance work — add a row whenever you spend >5 minutes grep-hunting for the canonical xterm location of a behavior.

All paths are relative to `~/projects/reference_repos/console_repos/xterm/` unless noted.

## DECRQCRA + rectangular-area ops (§09A)

| Feature | File | Lines | Notes |
|---|---|---|---|
| `xtermCheckRect()` — DECRQCRA checksum core | `screen.c` | 3136-3265 | Sum-then-negate; xterm's `trimmed`/`total` split + end-of-row reset |
| CHARDRAWN skip path | `screen.c` | 3178-3180 | `if (!(ld->attribs[col] & CHARDRAWN)) { if (!(mode & (csNOTRIM | csDRAWN))) continue; }` |
| DRAWX_MASK trim gate | `screen.c` | 3236-3241 | `if (first || (ch != ' ') || (ld->attribs[col] & DRAWX_MASK))` — drawn structural cells survive trim |
| Attribute folding constants | `screen.c` | 3221-3234 | PROTECTED=0x04, INVISIBLE=0x08, UNDERLINE=0x10, INVERSE=0x20, BLINK=0x40, BOLD=0x80 |
| Combining-mark fold (wide chars) | `screen.c` | 3243-3251 | `if (!(mode & csBYTE)) { for_each_combData(off, ld) total += combData[off][col]; }` |
| `first`/`embedded` trim state hoist | `screen.c` | 3166-3167 | Declared ONCE outside row loop; reset at `3254-3257` under `!csNOTRIM` |
| `DRAWX_MASK` macro | `ptyx.h` | 3778 | `#define DRAWX_MASK (ATTRIBUTES | CHARDRAWN)` |
| csPOSITIVE / csATTRIBS / csNOTRIM / csDRAWN / csBYTE constants | `screen.c` | 3149 | Bit layout for XTCHECKSUM flags |
| `validRect` — coordinate clamping | `screen.c` | 3162 | "clamped to physical buffer" semantics |
| `DECALN` screen-alignment fill | `screen.c` | (grep for `"DECALN"`) | Fills every cell with 'E' + sets CHARDRAWN |

## Sixel / Kitty graphics (§13)

| Feature | File | Lines | Notes |
|---|---|---|---|
| Sixel parser entry | `graphics_sixel.c` | 396-405 | Does NOT set CHARDRAWN on image-occupied cells — xterm graphics are grid-transparent |
| Sixel cell coverage | `graphics.c` | 694-699 | Image placement tracks pixel regions, not cell state |
| `drawXtermText()` — CHARDRAWN setter | `util.c` | (grep for `CHARDRAWN`) | ONLY path that sets the bit; image paths never do |

## Mouse protocols (§16)

| Feature | File | Lines | Notes |
|---|---|---|---|
| Mouse locator response (DECEFR/DECELR/DECSLE/DECRQLP) | `button.c` | (search `DECRQLP`) | Legacy locator — we mark gate-only in catalog/mouse.md |

## DEC private modes (§09)

| Feature | File | Lines | Notes |
|---|---|---|---|
| `do_dec_rqm()` — DECRQM dispatch | `charproc.c` | (grep for `do_dec_rqm`) | Mode-state report encoding |

## Pattern for adding rows

When you spend significant time finding an xterm source location, add a row here with:
1. **Feature** — the VT/DEC sequence or behavior (prefix with the catalog ID if applicable, e.g., `DECRECT-DECRQCRA`)
2. **File** — path relative to `~/projects/reference_repos/console_repos/xterm/`
3. **Lines** — specific line numbers (range for multi-line constructs, single line for constants)
4. **Notes** — one-sentence description of what lives there; flag any deviations ori_term makes from xterm

## Sources reviewers can verify against

For `/tpr-review` and `/tp-help` invocations that need xterm cross-check, cite the full path form:
```
~/projects/reference_repos/console_repos/xterm/screen.c:3136
```

This lets the reviewer `Read` the file directly. Relative paths like `screen.c:3136` are shorter but require the reviewer to already know the xterm root.

## Related references

- `~/projects/reference_repos/console_repos/xterm/ctlseqs.txt` — canonical spec source for DEC private CSI intermediates (used by `catalog/dec-rectangle-ops.md`, `catalog/dec-presentation.md`)
- `plans/spec-conformance/research.md` — higher-level architectural notes on xterm/wezterm/alacritty patterns
- `.claude/memory/reference_wezterm_graphics.md` — WezTerm sixel + kitty graphics map
