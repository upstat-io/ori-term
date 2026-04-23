---
section: "13"
title: "Kitty Graphics Protocol"
canonical_spec_sources:
  - "sw.kovidgoyal.net/kitty/graphics-protocol/ — primary protocol documentation (kitty source is the de facto spec for this protocol); snapshot at ~/projects/reference_repos/console_repos/kitty/docs/graphics-protocol.rst"
  - "kitty source kittens/icat/icat.py — cross-reference for client-side transmission patterns"
  - "wezterm term/src/terminalstate/kitty.rs — behavior reference for ambiguous arms"
last_walked: 2026-04-21
walked_by: "elucidsoft"
---

# Top-Down Spec Audit — Section 13: Kitty Graphics Protocol

## Canonical spec source(s)

The kitty graphics protocol documentation at sw.kovidgoyal.net/kitty/graphics-protocol/ (local snapshot at `~/projects/reference_repos/console_repos/kitty/docs/graphics-protocol.rst`) is the authoritative top-down enumerator for kitty graphics coverage. This is an APC-based protocol (`ESC _ G ... ESC \`) where kitty itself IS the spec — the public documentation and the kitty source are co-authoritative. Every key-value pair, action (`a=`), format (`f=`), transmission mode (`t=`), chunk flag (`m=`), placement flag (`U=`), response code, and delete specifier (`d=`) defined in the protocol maps to a catalog row below or carries an explicit `not-targeted` decision. `kittens/icat/icat.py` and wezterm's `term/src/terminalstate/kitty.rs` act as cross-references for ambiguous client-side transmission patterns and implementation tie-breaks.

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| `a=t` | kitty graphics-protocol.rst §Transmitting image data | `KG-ACTION-TRANSMIT` | mapped |
| `a=T` | kitty graphics-protocol.rst §Transmitting image data | `KG-ACTION-TRANSMIT-AND-PLACE` | mapped |
| `a=p` | kitty graphics-protocol.rst §Displaying an already transmitted image | `KG-ACTION-PLACE` | mapped |
| `a=d` | kitty graphics-protocol.rst §Deleting images | `KG-ACTION-DELETE` | mapped |
| `a=f` | kitty graphics-protocol.rst §Animation | `KG-ACTION-FRAME` | mapped |
| `a=a` | kitty graphics-protocol.rst §Animation control | `KG-ACTION-ANIMATE` | mapped |
| `a=q` | kitty graphics-protocol.rst §Querying for image support | `KG-ACTION-QUERY` | mapped |
| `a=c` | kitty graphics-protocol.rst §Animation frame composition | `KG-ACTION-COMPOSE` | mapped |
| `a=<unknown>` | kitty graphics-protocol.rst §Transmitting image data (behavioral pin) | `KG-ACTION-FALLBACK-TRANSMITANDPLACE` | mapped |
| `t=d` | kitty graphics-protocol.rst §Transmission mediums | `KG-TRANSMIT-DIRECT` | mapped |
| `t=f` | kitty graphics-protocol.rst §Transmission mediums | `KG-TRANSMIT-FILE` | mapped |
| `t=t` | kitty graphics-protocol.rst §Transmission mediums | `KG-TRANSMIT-TEMPFILE` | mapped |
| `t=s` | kitty graphics-protocol.rst §Transmission mediums | `KG-TRANSMIT-SHARED-MEM-REJECTED` | mapped |
| `t=<unknown>` | kitty graphics-protocol.rst §Transmission mediums (fallback) | `KG-TRANSMIT-DIRECT` | mapped |
| `f=24` | kitty graphics-protocol.rst §Pixel formats | `KG-TRANSMIT-FORMAT-24` | mapped |
| `f=32` | kitty graphics-protocol.rst §Pixel formats | `KG-TRANSMIT-FORMAT-32` | mapped |
| `f=100` | kitty graphics-protocol.rst §Pixel formats | `KG-TRANSMIT-FORMAT-100` | mapped |
| `f=<other>` | kitty graphics-protocol.rst §Pixel formats (validation) | `KG-FORMAT-UNSUPPORTED` | mapped |
| `o=z` | kitty graphics-protocol.rst §Compression | `KG-COMPRESSION-OZ-IGNORED` | mapped |
| `o=<other>` | kitty graphics-protocol.rst §Compression | — | not-targeted: kitty defines only `z` (zlib); no other compression codepoint documented and no real-world client uses one |
| `m=0` | kitty graphics-protocol.rst §Chunked data | `KG-TRANSMIT-CHUNKED-COALESCE` | mapped |
| `m=1` | kitty graphics-protocol.rst §Chunked data | `KG-TRANSMIT-CHUNKED-COALESCE` | mapped |
| chunked size-limit discard | kitty graphics-protocol.rst §Chunked data (implementation-defined cap) | `KG-TRANSMIT-CHUNKED-SIZE-LIMIT` | mapped |
| `KittyError::InvalidBase64` reply | kitty graphics-protocol.rst §Error handling | `KG-TRANSMIT-CHUNKED-MALFORMED-BASE64` | mapped |
| `i=<u32>` | kitty graphics-protocol.rst §Keys for image transmission | `KG-ACTION-TRANSMIT` | mapped |
| `I=<u32>` | kitty graphics-protocol.rst §Keys for image transmission | `KG-ACTION-TRANSMIT` | mapped |
| `p=<u32>` (placement id) | kitty graphics-protocol.rst §Keys for image transmission | `KG-ACTION-PLACE` | mapped |
| `q=0` | kitty graphics-protocol.rst §Suppressing responses | `KG-RESPONSE-QUIET` | mapped |
| `q=1` | kitty graphics-protocol.rst §Suppressing responses | `KG-RESPONSE-QUIET` | mapped |
| `q=2` | kitty graphics-protocol.rst §Suppressing responses | `KG-RESPONSE-QUIET` | mapped |
| `s=<u32>` (width) | kitty graphics-protocol.rst §Keys for image transmission | `KG-TRANSMIT-FORMAT-24` | mapped |
| `v=<u32>` (height) | kitty graphics-protocol.rst §Keys for image transmission | `KG-TRANSMIT-FORMAT-32` | mapped |
| `S=<u32>` (file-read size) | kitty graphics-protocol.rst §Keys for image transmission | — | not-targeted: ori_term reads the full file up to `max_single_image_bytes`; explicit size-slice read is a niche kitty extension with no observed consumer in the capture corpus |
| `O=<u32>` (file-read offset) | kitty graphics-protocol.rst §Keys for image transmission | — | not-targeted: pairs with `S=`; same rationale — unused by icat / wezterm / ghostty consumers |
| `x=<u32>` (src rect left, display) | kitty graphics-protocol.rst §Keys for image display | `KG-ACTION-PLACE` | mapped |
| `y=<u32>` (src rect top, display) | kitty graphics-protocol.rst §Keys for image display | `KG-ACTION-PLACE` | mapped |
| `w=<u32>` (src rect width, display) | kitty graphics-protocol.rst §Keys for image display | `KG-ACTION-PLACE` | mapped |
| `h=<u32>` (src rect height, display) | kitty graphics-protocol.rst §Keys for image display | `KG-ACTION-PLACE` | mapped |
| `X=<u32>` (cell x-offset, display) | kitty graphics-protocol.rst §Keys for image display | `KG-ACTION-PLACE` | mapped |
| `Y=<u32>` (cell y-offset, display) | kitty graphics-protocol.rst §Keys for image display | `KG-ACTION-PLACE` | mapped |
| `c=<u32>` (display cols) | kitty graphics-protocol.rst §Keys for image display | `KG-ACTION-PLACE` | mapped |
| `r=<u32>` (display rows) | kitty graphics-protocol.rst §Keys for image display | `KG-ACTION-PLACE` | mapped |
| `C=0` (cursor moves after image) | kitty graphics-protocol.rst §Keys for image display | `KG-ACTION-PLACE` | mapped |
| `C=1` (cursor stays) | kitty graphics-protocol.rst §Keys for image display | `KG-ACTION-PLACE` | mapped |
| `U=0` (normal placement) | kitty graphics-protocol.rst §Unicode placeholders | `KG-ACTION-PLACE` | mapped |
| `U=1` (unicode placeholder) | kitty graphics-protocol.rst §Unicode placeholders | `KG-UNICODE-PLACEHOLDER-TRANSMIT-U1` | mapped |
| `z=<i32>` (z-index) | kitty graphics-protocol.rst §Keys for image display | `KG-ACTION-PLACE` | mapped |
| `P=<u32>` (parent image id) | kitty graphics-protocol.rst §Relative placements | — | not-targeted: relative placement tree not implemented; no real-world consumer in ori_term's capture corpus (icat, chafa, matplotlib-kitty do not emit P=/Q=) |
| `Q=<u32>` (parent placement id) | kitty graphics-protocol.rst §Relative placements | — | not-targeted: pairs with `P=`; same rationale |
| `H=<i32>` (relative horizontal offset) | kitty graphics-protocol.rst §Relative placements | — | not-targeted: pairs with `P=`/`Q=`; relative placement family is uniformly excluded |
| `V=<i32>` (relative vertical offset) | kitty graphics-protocol.rst §Relative placements | — | not-targeted: pairs with `P=`/`Q=`; relative placement family is uniformly excluded |
| `a=f x=<u32>` (frame src rect left) | kitty graphics-protocol.rst §Animation frame loading | `KG-FRAME-TRANSMIT` | mapped |
| `a=f y=<u32>` (frame src rect top) | kitty graphics-protocol.rst §Animation frame loading | `KG-FRAME-TRANSMIT` | mapped |
| `a=f c=<u32>` (create/replace frame N) | kitty graphics-protocol.rst §Animation frame loading | `KG-FRAME-REPLACE` | mapped |
| `a=f r=<u32>` (edit frame N) | kitty graphics-protocol.rst §Animation frame loading | `KG-FRAME-EDIT` | mapped |
| `a=f z=<i32>` (frame gap ms) | kitty graphics-protocol.rst §Animation frame loading | `KG-FRAME-TRANSMIT` | mapped |
| `a=f X=0` (alpha-blend composition) | kitty graphics-protocol.rst §Animation frame loading | `KG-FRAME-COMPOSITE-ALPHABLEND` | mapped |
| `a=f X=1` (overwrite composition) | kitty graphics-protocol.rst §Animation frame loading | `KG-FRAME-COMPOSITE-OVERWRITE` | mapped |
| `a=f Y=<u32>` (frame bg color for undrawn pixels) | kitty graphics-protocol.rst §Animation frame loading | — | not-targeted: ori_term does not yet fill undrawn frame pixels with a background color; `ImageCache::add_animation_frame` treats alpha as canonical. Revisit when §13.3 ships composite tests and measures actual client usage |
| `a=c c=<u32>` (compose src frame) | kitty graphics-protocol.rst §Animation frame composition | `KG-ACTION-COMPOSE` | mapped |
| `a=c r=<u32>` (compose dest frame) | kitty graphics-protocol.rst §Animation frame composition | `KG-ACTION-COMPOSE` | mapped |
| `a=c w=<u32>` (compose rect width) | kitty graphics-protocol.rst §Animation frame composition | `KG-ACTION-COMPOSE` | mapped |
| `a=c h=<u32>` (compose rect height) | kitty graphics-protocol.rst §Animation frame composition | `KG-ACTION-COMPOSE` | mapped |
| `a=c x=<u32>` (compose dest rect left) | kitty graphics-protocol.rst §Animation frame composition | `KG-ACTION-COMPOSE` | mapped |
| `a=c y=<u32>` (compose dest rect top) | kitty graphics-protocol.rst §Animation frame composition | `KG-ACTION-COMPOSE` | mapped |
| `a=c X=<u32>` (compose src rect left) | kitty graphics-protocol.rst §Animation frame composition | `KG-ACTION-COMPOSE` | mapped |
| `a=c Y=<u32>` (compose src rect top) | kitty graphics-protocol.rst §Animation frame composition | `KG-ACTION-COMPOSE` | mapped |
| `a=c C=<u32>` (compose blend mode) | kitty graphics-protocol.rst §Animation frame composition | `KG-ACTION-COMPOSE` | mapped |
| `a=a s=1` (stop) | kitty graphics-protocol.rst §Animation control | `KG-ANIMATE-STOP` | mapped |
| `a=a s=2` (run and wait) | kitty graphics-protocol.rst §Animation control | `KG-ANIMATE-RUN-WAIT` | mapped |
| `a=a s=3` (run) | kitty graphics-protocol.rst §Animation control | `KG-ANIMATE-RUN` | mapped |
| `a=a v=<u32>` (loop count) | kitty graphics-protocol.rst §Animation control | `KG-ANIMATE-LOOP-COUNT` | mapped |
| `a=a r=<u32>` (set current frame) | kitty graphics-protocol.rst §Animation control | `KG-ANIMATE-SET-CURRENT-FRAME` | mapped |
| `a=a c=<u32>` (set displayed frame) | kitty graphics-protocol.rst §Animation control | `KG-ANIMATE-SET-CURRENT-FRAME` | mapped |
| `a=a z=<i32>` (set current frame gap) | kitty graphics-protocol.rst §Animation control | `KG-ANIMATE-SET-FRAME-GAP` | mapped |
| `a=d d=a` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-a` | mapped |
| `a=d d=A` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-A` | mapped |
| `a=d d=i` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-i` | mapped |
| `a=d d=I` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-I` | mapped |
| `a=d d=p` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-p` | mapped |
| `a=d d=P` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-P` | mapped |
| `a=d d=c` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-c` | mapped |
| `a=d d=C` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-C` | mapped |
| `a=d d=x` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-x` | mapped |
| `a=d d=X` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-X` | mapped |
| `a=d d=y` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-y` | mapped |
| `a=d d=Y` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-Y` | mapped |
| `a=d d=z` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-z` | mapped |
| `a=d d=Z` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-Z` | mapped |
| `a=d d=r` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-r` | mapped |
| `a=d d=R` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-R` | mapped |
| `a=d d=n` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-n` | mapped |
| `a=d d=N` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-N` | mapped |
| `a=d d=q` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-q` | mapped |
| `a=d d=Q` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-Q` | mapped |
| `a=d d=f` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-f` | mapped |
| `a=d d=F` | kitty graphics-protocol.rst §Deleting images | `KG-DELETE-F` | mapped |
| `a=d d=<unknown>` | kitty graphics-protocol.rst §Deleting images | — | not-targeted: spec enumerates exactly the 22 pairs above; any other value is a protocol violation and ori_term's `_ =>` catch-all logs-and-skips, matching wezterm behavior |
| Response `OK` | kitty graphics-protocol.rst §Error handling | `KG-RESPONSE-OK` | mapped |
| Response `EBADF:<msg>` | kitty graphics-protocol.rst §Error handling | `KG-RESPONSE-EBADF` | mapped |
| Response `EBIG:<msg>` | kitty graphics-protocol.rst §Error handling | `KG-RESPONSE-EBIG` | mapped |
| Response `EINVAL:<msg>` | kitty graphics-protocol.rst §Error handling | `KG-RESPONSE-EINVAL` | mapped |
| Response `ENOENT:<msg>` | kitty graphics-protocol.rst §Error handling | `KG-RESPONSE-ENOENT` | mapped |
| Response `ENOMEM:<msg>` | kitty graphics-protocol.rst §Error handling | `KG-RESPONSE-ENOMEM` | mapped |
| Response `EIO:<msg>` | kitty graphics-protocol.rst §Error handling | `KG-RESPONSE-EIO` | mapped |
| Response `ECYCLE:<msg>` | kitty graphics-protocol.rst §Relative placements (cycle rejection) | — | not-targeted: ECYCLE only applies to relative placement chains (P=/Q=); whole relative-placement family is not-targeted, so this response code is dead code for ori_term |
| Response `ETOODEEP:<msg>` | kitty graphics-protocol.rst §Relative placements (depth limit) | — | not-targeted: same rationale as ECYCLE — only fires on relative-placement chains |
| Response `ENOPARENT:<msg>` | kitty graphics-protocol.rst §Relative placements (missing parent) | — | not-targeted: same rationale — relative placement chains only |
| Response `EUNSUPPORTED:<msg>` | kitty kitty/graphics.c (client-side extension reply) | — | not-targeted: not in the public protocol documentation; a kitty-source-only reply used for unimplemented experimental kittens. ori_term uses EINVAL for the same conditions (matching wezterm) |
| Unicode placeholder `U+10EEEE + diacritic row/col` | kitty graphics-protocol.rst §Unicode placeholders (cell resolution) | `KG-UNICODE-PLACEHOLDER-CELL-RESOLVE` | mapped |
| Unicode placeholder cells move under reflow/scroll | kitty graphics-protocol.rst §Unicode placeholders (reflow semantics) | `KG-UNICODE-PLACEHOLDER-REFLOW` | mapped |
| Kitty placement mixed with sixel raster (z-order) | kitty graphics-protocol.rst §Z-index + sixel layering | `KG-CROSS-STACK-SIXEL-MIXED-Z-ORDER` | mapped |
| Kitty + sixel shared cache eviction | kitty graphics-protocol.rst §Memory budget | `KG-CROSS-STACK-SIXEL-MIXED-EVICTION` | mapped |
| U=1 placeholder cells coexisting with sixel raster | kitty graphics-protocol.rst §Unicode placeholders (cross-stack) | `KG-CROSS-STACK-SIXEL-PLACEHOLDER-COEXIST` | mapped |

## Decisions

Every `not-targeted` row above carries its rationale inline; this section collects them for cross-reference:

- `o=<other>` (non-zlib compression value) — kitty defines only `z`. No other compression codepoint is documented and no real-world client uses one; the fallback surfaces via silent no-op at `parse.rs` `apply_key_value` for key `o`.
- `S=<u32>` / `O=<u32>` (file-read size / offset) — niche kitty extension for reading a slice of a file rather than the whole file. No observed consumer (icat, chafa, matplotlib-kitty, wezterm's file-handling reference) exercises these keys; ori_term reads the full file up to the per-image byte cap. Revisit if a consumer surfaces.
- `P=` / `Q=` / `H=` / `V=` (relative placement family) — relative placement implementation touches a separate subsystem (placement tree + cycle detection + inheritance), no real-world consumer in ori_term's capture corpus emits these keys. The entire family is uniformly excluded; re-walk trigger: any consumer showing up in the capture corpus.
- `a=f Y=<u32>` (frame background color for undrawn pixels) — ori_term does not yet fill undrawn frame pixels with a background color; alpha is treated as canonical. §13.3 composite tests should measure client usage and upgrade this to a catalog row if clients rely on it.
- `a=d d=<unknown>` — spec enumerates exactly the 22 pairs (`a`/`A`/`i`/`I`/`n`/`N`/`c`/`C`/`f`/`F`/`p`/`P`/`q`/`Q`/`r`/`R`/`x`/`X`/`y`/`Y`/`z`/`Z`). Any other value is a protocol violation; `kitty_delete` `_ =>` catch-all logs-and-skips.
- `ECYCLE`, `ETOODEEP`, `ENOPARENT` — only emitted on relative-placement chains. Since the relative-placement family is `not-targeted`, these response codes are dead code for ori_term.
- `EUNSUPPORTED` — not in the public protocol documentation. Used by kitty source for experimental kitten replies; ori_term uses `EINVAL` for the same conditions, matching wezterm's convention.

All `not-targeted` decisions are **revisitable** — re-walk triggers: (1) a real-world client surfaces that depends on a currently excluded key, (2) kitty revises the protocol to move an excluded key into mandatory territory, or (3) a downstream section (sixel, graphics-cross-stack) discovers it needs the excluded arm to function.

## Verification

- [ ] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/kitty-graphics.md` with the cited row ID.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope.
- [ ] No row in the canonical spec source is missing from this table (top-down completeness).
- [ ] `last_walked` date is set; `walked_by` is set.
