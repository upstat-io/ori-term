# Spec Conformance — Research

**Status**: research snapshot (2026-04-07). **Superseded by `plans/spec-conformance/`** — see `plans/spec-conformance/00-overview.md` for the current plan tree (26 sections, active reroute). This document captured the foundation the plan was written against and is now historical context, NOT a source-of-truth for current project state. Claims below that disagree with the catalog under `plans/spec-conformance/catalog/` lose to the catalog.

**Date**: 2026-04-07 (research snapshot); superseded 2026-04-11 during §01.9 stale-claim corrections.

**Mandate (paraphrased from user)**: ori_term has many visible rendering issues running notcurses-demo. The strategy is to tackle the spec of each stack used (Kitty, Sixel, Unicode subcell glyphs, OSC, CSI/SGR, Mode 2026, etc.), aiming for 100% compliance per stack, methodically tested with golden images, snapshot pinning, and matrix testing. The test framework is a co-deliverable — built incrementally, driven by what each stack's tests demand, never speculatively.

**Status update (2026-04-11)**: the plan exists. `plans/spec-conformance/` is the active reroute. `plans/tack-conformance/` has been mechanically absorbed via `plans/spec-conformance/section-02-tack-absorption.md`; tack files stay in place for citation stability per the absorption covenant. Section 01 (Catalog Bootstrap) has delivered the catalog + bootstrap gate + bug filings; Sections 12 (Sixel) and 13 (Kitty Graphics) are blocked on `BUG-08-8` (kitty.rs BLOAT split).

---

## 1. Triggering observation

User running `notcurses-demo` against ori_term: many visible rendering issues across the board. Performance is good, no dropped frames, but visual fidelity is poor compared to WezTerm (which is the closest non-notcurses terminal to 99% accuracy on the demo, though even WezTerm hangs and drops frames in places).

Initial assumption was missing features. Audit overturned that — see §4.

## 2. Reference materials gathered

### 2a. WezTerm sixel + kitty graphics implementation (correctness reference)

Full end-to-end map saved to memory at `reference_wezterm_graphics.md`. Source:
`~/projects/reference_repos/console_repos/wezterm/`.

Key correctness details that ori_term implementations should be cross-checked against:

**Sixel — DCS `\x1bP..q\x1b\\`**
- Parser entry: `wezterm-escape-parser/src/parser/mod.rs:233-244` — DCS `dcs_hook(byte=q)` → `SixelBuilder::new(params)`. Bytes streamed via `dcs_put`, terminated via `dcs_unhook` → `Action::Sixel(Box)`.
- State machine: `wezterm-escape-parser/src/parser/sixel.rs:45-94`. Max 5 params, saturating arithmetic.
- Decoder: `term/src/terminalstate/sixel.rs:10-156` — `TerminalState::sixel(sixel)`. Walks `SixelData` enum (Data | Repeat | CR | NL | DefineColorMapRGB | SelectColorMapEntry). Each data byte = 6 vertical pixels in one column (bit N → y+N).
- **HSL hue rotation quirk**: `hue_angle - 120°` (sixel spec uses blue=0°, CSS uses red=0°). Lines 87-94. Common silent-bug source.
- Raster attrs P1/P2/P3/P4/P5: pan/pad aspect ratio + explicit pixel width/height. Lines 131-175.
- Repeat optimization: `SixelData::Repeat { repeat_count, data }` — critical for solid-fill scenes.
- Background transparency: `background_is_transparent` flag — unset pixels stay α=0 vs filled with `color_map[0]`.
- Cursor positioning: default → cursor moves left after image. DECSDM → cursor stays, image overlays.

**Kitty — APC `\x1b_G..;\x1b\\`**
- Parser entry: `wezterm-escape-parser/src/parser/mod.rs:225-230` — APC `apc_dispatch(data)` → data starts with `'G'` → `KittyImage::parse_apc(data)`.
- Chunked transmission: `m=1` → queue in accumulator; `m=0` → coalesce + decode. **Coalesce concatenates raw base64 first, then decodes** (NOT per-chunk decode). `term/src/terminalstate/kitty.rs:851-924`.
- Decoder: `term/src/terminalstate/kitty.rs:743-849` — `kitty_img_transmit_inner()`. Format `f=24` (RGB), `f=32` (RGBA, default), `f=100` (PNG via `image::load_from_memory`). Compression `o=z` (zlib via `miniz_oxide::inflate::decompress_to_vec_zlib`).
- Data sources: `Direct` (base64), `DirectBin` (raw), `File`, `TemporaryFile`, `SharedMem` (POSIX shm_open / Windows file mapping).
- Frame composition (`a=f` TransmitFrame, `a=c` ComposeFrame): `Overwrite` or `AlphaBlending` mode (`kitty.rs:959-979`). Stored as `ImageDataType::AnimRgba8 { frames, hashes, durations }`.

**Image storage & cache**
- Hash-deduped LRU: `LruCache<[u8; 32], Arc<ImageData>>` — 16 entries. Key = SHA256 of decoded RGBA. `term/src/terminalstate/mod.rs:358,569` and `image.rs:256-270`.
- Kitty state: `accumulator: Vec<KittyImage>` (chunked queue), `max_image_id: u32`, `number_to_id: HashMap<u32, u32>` (I= → i= map), `id_to_data: HashMap<u32, Arc<ImageData>>`, `placements: HashMap<(u32, Option<u32>), PlacementInfo>`, `used_memory: usize`.
- Kitty memory budget: hardcoded 320 MB. `prune_unreferenced()` evicts unreferenced images oldest-first. `kitty.rs:46-68`.
- Image number (`I=`) vs ID (`i=`): `I=` = short-lived session number, `i=` = persistent ID. ID 0 reserved for temporary, reused per number.

**Cell-grid integration**
- Sixel = positional placement (cartesian, scrolls with text): `cell.attrs_mut().set_image(ImageCell)` — **replaces** glyph. Each cell stores normalized texture coords. Scrolls into scrollback.
- Kitty = virtual placement (z-ordered, doesn't scroll): `cell.attrs_mut().attach_image(ImageCell)` — **layered**, z-sorted insertion. Multiple images per cell allowed. Doesn't scroll naturally.
- Shared `ImageCell` struct (`wezterm-cell/src/lib.rs:365-404`): top_left, bottom_right (texture coords), data (Arc<ImageData>), z_index, padding_left/top/right/bottom, image_id, placement_id.

**GPU render path**
- `wezterm-gui/src/termwindow/render/mod.rs:441-521` — `populate_image_quad()`.
- `glyph_cache.cached_image(image.image_data(), padding)` returns Sprite (TextureRect + GpuTexture). **Image cached as sprite in same texture atlas as glyphs.**
- Compute physical texture coords from ImageCell's normalized `top_left`/`bottom_right`.
- `layers.allocate(layer_num)` → 4-vertex quad. **Same shader path as glyphs**, switched by `has_color` flag (IS_COLOR_EMOJI / IS_BG_IMAGE).
- Z-order: `z_index < 0` → background layer, `z_index ≥ 0` → foreground layer. Per-cell z-list sorted via binary insertion.

**Lifecycle summary**

| Aspect | Sixel | Kitty |
|---|---|---|
| Cell occupancy | Glyph replaced | Glyph coexists, z-sorted |
| Scrolling | Scrolls with text | Virtual — doesn't scroll |
| Deletion | Via scrollback / erase | Explicit `a=d` |
| Memory | LRU 16 entries | 320 MB budget |
| Animation | N/A | Frame composition |
| Cursor impact | Moves (DECSDM toggles) | None |
| Aspect ratio | P1 pan parameter | Implicit in cols/rows |

**Existing tests in WezTerm**
- Sixel parser: `wezterm-escape-parser/src/parser/sixel.rs:188-305` — Wikipedia "HI" image, color/repeat/newline coverage.
- Kitty parser: `wezterm-escape-parser/src/apc.rs:1209-1269` — RGB, PNG, deletion, frame composition payloads.
- No notcurses-specific integration tests.

### 2b. notcurses-demo scene → protocol matrix

Full matrix saved to memory at `reference_notcurses_demo.md`. Source:
`~/projects/reference_repos/console_repos/notcurses/src/demo/`. Binary: `/usr/bin/notcurses-demo`.

**Default scene order** (`demo.c:23`): `ixetunchdmbkywjgarvlsfqzo` (28 scenes)
→ intro → xray → eagle → trans → uniblock → normal → chunli → highcon → dragon → mojibake → box → keller → yield → whiteout → jungle → grid → animate → reel → view → sliders → fission → zoo → qrcode → outro

**Scene matrix (one row each)**

| Letter | File | Subsystems exercised | Media | Complexity |
|---|---|---|---|---|
| **a** | animate.c | quadrants, rgb, scrolling, multi-plane, unicode-boxes | none | medium |
| **b** | box.c | unicode-boxes, transparency, multi-plane, media, pixel-blit | spaceship.png | medium |
| **c** | chunli.c | media, pixel-blit | chunli{0..99}.png + bmp | medium |
| **d** | dragon.c | pixel-blit, rgb | none (generated) | high |
| **e** | eagle.c | unicode-boxes, multi-plane, media | eagles.png | medium |
| **f** | fission.c | multi-plane, scrolling, media, greyscale | lamepatents.jpg | high |
| **g** | grid.c | rgb, unicode-boxes, text-attributes | none | high |
| **h** | highcon.c | rgb, text-attributes | none | medium |
| **i** | intro.c | pixel-blit, rgb, fades, sextants, wide-chars, boxes | natasha-blur.png | medium |
| **j** | jungle.c | media, pixel-blit, audio | embedded 1.3MB | high |
| **k** | keller.c | media, **all 7 blitters** | covid19.jpg, atma.png, fonts.jpg, aidsrobots.jpeg | high |
| **l** | luigi.c | media, pixel-blit, multi-plane, transparency, wide-chars | megaman2.bmp, warmech.bmp | high |
| **m** | mojibake.c | wide-chars, unicode-boxes, rgb, scrolling | none | high |
| **n** | normal.c | pixel-blit, rgb, multi-plane, text-attributes, media | normal.png | high |
| **o** | outro.c | media, video, fades, multi-plane | changes.jpg, samoa.avi | high |
| **q** | qrcode.c | qrcode, rgb, text-attributes | none | low |
| **r** | reel.c | multi-plane, scrolling, text-attributes | none | high |
| **s** | sliders.c | multi-plane, rgb, text-attributes, wide-chars, boxes | none | high |
| **t** | trans.c | transparency, rgb, fades, multi-plane | none | high |
| **u** | uniblock.c | wide-chars, unicode-boxes, rgb, **all 7 blitters** | none | high |
| **v** | view.c | media, video, pixel-blit, multi-plane, transparency | dsscaw-purp.png, PurpleDrank.jpg, fm6.mov | high |
| **w** | whiteout.c | unicode-boxes, multi-plane, rgb, text-attributes | none | high |
| **x** | xray.c | media, video, scrolling, multi-plane, text-attributes | notcurses.avi | high |
| **y** | yield.c | media, pixel-blit, threading | worldmap.png | high |
| **z** | zoo.c | media, multi-plane, scrolling, rgb, text-attributes | changes.jpg | high |

**Pixel-graphics dependence**

- Hard-required (skip if no images): chunli, eagle, jungle, keller, luigi, view, yield, zoo
- Degrade gracefully (work without media): box, dragon, fission, normal, whiteout, xray
- No pixel graphics needed at all: animate, grid, highcon, intro, mojibake, qrcode, reel, sliders, trans, uniblock

**Capability detection** (`notcurses/src/lib/termdesc.c`)
- `TERM` env var → initial classification
- terminfo db → smcup/DA2 capabilities
- DA2 query → kitty responds with `\x1b[?...;...\x1bP<kitty-marker>...\x1b\\`; xterm with version
- Sixel probe → checks for sixel response
- Kitty probe → `\x1b_Gi=0\x1b\\`
- Env: `KITTY_WINDOW_ID` presence → kitty
- `setup_kitty_bitmaps()` (termdesc.c:120) and `setup_sixel_bitmaps()` (termdesc.c:87) gated on detection
- **Fallback chain**: kitty → sixel → octant → sextant → quadrant → half-block → braille → ASCII

**Best test targets** (from simplest to hardest)
1. **qrcode (q)** — simplest, ~40 lines, no media, deterministic
2. **highcon (h)** — gradient iteration, no animation
3. **grid (g)** — static color gradients + box drawing, deterministic
4. **animate (a)** — medium, no media; quadrants + plane stacking
5. **box (b)** — medium, optional image; box drawing + transparency
6. **trans (t)** — alpha blending across 6 planes — best transparency stress test
7. **uniblock (u)** — exhaustive 7-blitter rendering — best blitter A/B test
8. **keller (k)** — same image rendered through every blitter — gold for blitter correctness
9. **xray (x)**, **yield (y)** — hardest, video + threading

### 2c. ori_term graphics protocol audit (THE SURPRISE)

Full audit saved to memory at `architecture_graphics_audit.md`. Audit date 2026-04-07, verify against current code before relying.

**Key finding**: ori_term has sixel, kitty graphics, iTerm2 inline images, half-blocks, quadrants, sextants, and braille all implemented. The user's "many rendering issues" cannot be missing features. **The gap is correctness inside existing implementations.** This changes the strategy from "build features" to "bisect bugs in existing code."

**Implementation map**

Image / pixel graphics protocols:
- Sixel parser: `crates/vte/src/ansi/dispatch/mod.rs:52-57` — DCS `q` → `sixel_start()`
- Sixel decoder: `oriterm_core/src/image/sixel/mod.rs:1-440` — palette (256 RGB), repeat ops, raster attrs (P1/P2), HLS rotation via `hls_to_rgb()`, background modes
- Sixel grid integration: `oriterm_core/src/term/handler/image/sixel.rs:64-139` — `sixel_create_placement()`, `ImageCache`, cell coverage, orphan cleanup, cursor positioning per SIXEL_SCROLLING/SIXEL_CURSOR_RIGHT modes
- Sixel GPU rendering: `oriterm/src/gpu/image_render/mod.rs:1-127` — `ImageTextureCache` LRU GPU textures
- Kitty parser: `oriterm_core/src/image/kitty/parse.rs:141-291` — APC `G` key=value, base64 decode, chunked transmission, all standard keys
- Kitty decoder: `oriterm_core/src/term/handler/image/kitty.rs:313-344` — RGB/RGBA/PNG via `image` crate (feature `image-protocol`), file transmission with path traversal protection
- Kitty placement: `oriterm_core/src/term/handler/image/kitty.rs:148-164, 401-462` — image_id, placement_id, z-index, virtual placements (U=1 with unicode placeholders), delete actions
- Kitty animation: `oriterm_core/src/term/handler/image/kitty_animation.rs` — frame composition, base frames (verify alpha modes)
- iTerm2 OSC 1337: `oriterm_core/src/term/handler/image/iterm2.rs:1-232` — base64, inline/download, GIF frame extraction
- Image cache: `oriterm_core/src/image/cache/mod.rs:1-436` — LRU, hash-deduped. Default memory cap is **320 MiB** via `DEFAULT_MEMORY_LIMIT: usize = 320 * 1024 * 1024` (`cache/mod.rs:15`, used as `memory_limit` on line 64). Ghostty parity. (Corrected 2026-04-11 during §01.9 audit — the pre-§01.9 assertion of the cap disagreed with the source constant.)
- GPU image pipeline: `oriterm/src/gpu/pipeline/image.rs:1-127` — 36-byte instance, premultiplied alpha blend, lazy upload

Pixel-blit glyphs (rendered via `Canvas` abstraction in `oriterm/src/gpu/builtin_glyphs/mod.rs:60-90` with fractional pixel precision — no chunky misalignment per audit):
- Half blocks (U+2580/U+2584): `oriterm/src/gpu/builtin_glyphs/blocks.rs:13-48`
- Quadrants (U+2596–U+259F): `oriterm/src/gpu/builtin_glyphs/blocks.rs:44-89`
- Sextants (U+1FB00–U+1FB3B): `oriterm/src/gpu/builtin_glyphs/legacy_computing/mod.rs:35-67`
- Braille (U+2800–U+28FF): `oriterm/src/gpu/builtin_glyphs/braille.rs:26-47`
- Powerline / Nerd Font: `oriterm/src/gpu/builtin_glyphs/powerline.rs` + glyph cache fallback
- **Octants (U+1CD00–U+1CDE5): NOT IMPLEMENTED** (Unicode 16, narrow demo impact)

Color:
- RGB / 256-color: `crates/vte/src/ansi/attr.rs:130-134`; `oriterm_core/src/term/handler/sgr.rs`
- Palette: `oriterm_core/src/color/palette/mod.rs:1-400` — 270 entries, OSC 4/10/11/12 supported
- **Cell-level alpha: NOT MODELED.** Image quads support opacity; cells do not. Likely matters for `trans` scene.

Modes / cursor / scroll regions:
- DECSTBM: implemented
- DECLRMM (left/right margins): STUB — VTE recognizes, grid doesn't enforce
- DECOM, IRM, DECCKM, keypad: implemented
- DECSDM (Sixel scrolling, mode 80): `oriterm_core/src/term/mode/mod.rs:72`, default ON

Detection:
- DA1: `CSI?64;6;4c` (VT420 + ANSI color + sixel param 4)
- DA2/DA3: implemented
- **Kitty graphics query (`q=1`): IMPLEMENTED** (corrected 2026-04-11 during §01.9 audit — the pre-01.9 note said "NOT IMPLEMENTED" which was wrong). `parse_kitty_command` (re-exported from `oriterm_core/src/image/kitty/mod.rs:9`) parses `a=q` into `KittyAction::Query`; `Term::handle_kitty_graphics` → `Term::kitty_query` at `oriterm_core/src/term/handler/image/kitty.rs:53, 64` dispatches it and responds via `Term::kitty_respond` (line 465). Current response is a hardcoded `OK` — capability-richness is Section 13 scope, but dispatch is live.

Other:
- OSC 8 hyperlinks: `crates/vte/src/ansi/dispatch/osc.rs:120-142`
- OSC 52 clipboard: `crates/vte/src/ansi/dispatch/osc.rs:206-215`
- Sync output (mode 2026): `oriterm_core/src/term/mode/mod.rs:44`
- Bracketed paste (mode 2004): implemented
- Mouse: X10/click/drag/motion/SGR/UTF-8/URXVT/focus all supported

**Bisection priorities (where to look for bugs first)**

1. ~~HSL color rotation in sixel decoder~~ — **VERIFIED CORRECT 2026-04-11 during §01.9 audit.** `hls_to_rgb` at `oriterm_core/src/image/sixel/color.rs:30` does `let hf = hue as f64 - 120.0;` on line 41 — matches WezTerm. Test pin at `oriterm_core/src/image/sixel/tests.rs:83` (sixel hue 120 → standard hue 0 = red).
2. Kitty animation frame blending — partial; alpha modes need cross-check (Section 13 owns)
3. Premultiplied vs straight alpha in image GPU pipeline — easy to get wrong
4. Cell-level alpha gap — scenes with overlapping translucent planes (trans.c) likely render wrong (Section 15 owns)
5. Sixel cursor positioning vs DECSDM toggle
6. Synchronized output flushing (mode 2026) — must actually defer GPU draws (Section 06 owns)
7. ~~Kitty `q=1` query response~~ — **DISPATCH IMPLEMENTED 2026-04-11.** See the "Kitty graphics query" entry under Detection above; residual work (richer capability report) tracked by Section 13.

## 3. Initial strategy candidate (rejected) — notcurses-demo replay harness

**The idea**: capture notcurses-demo's byte stream once via PTY tee, feed it through ori_term's VTE pipeline, snapshot frames at known boundaries (Mode 2026 sync ends), image-diff against WezTerm-rendered references.

**Why we moved past this**: WezTerm itself is not 100% accurate on notcurses-demo (drops frames, has its own rendering issues). Using WezTerm-rendered references would bake WezTerm's bugs into our targets. Comparing against a reference implementation only validates "match this implementation," not "match the spec." For "many rendering issues across the board" we need ground truth, not another implementation.

**The pivot**: spec-by-stack conformance with the spec itself as ground truth. WezTerm becomes one of several reference implementations we cross-check, not the target.

## 4. Strategy direction (agreed in discussion, not yet planned)

**Spec-by-stack conformance**, taking each protocol stack ori_term implements and driving it to 100% spec compliance with golden images, snapshot pinning, and matrix testing. One stack at a time, finished completely before starting the next.

**Test framework as co-deliverable**: not built upfront. Built incrementally, driven by what each stack's tests demand. The first stack's tests dictate the framework's MVP — its scenario format, golden-image storage, diff tolerance, spec-citation convention, failure reporting. Each subsequent stack adds capabilities only when forced to.

**Why this works**:
- Spec is the only source of truth that doesn't drift.
- Golden images catch silent visual regressions that grid-state snapshots (existing teseq tests) miss.
- Matrix testing forces confronting feature interactions (kitty image + z-index + virtual placement + animation frame).
- Finishing one stack to 100% before starting the next prevents the "everything is half-done" failure mode.
- Coverage tracking (which spec sections have tests, which don't) makes "100% conformance" falsifiable.

**Design tensions** (real, need decisions before plan):

1. **Specs are not as clean as they sound.** Implementation-defined behaviors, ambiguous "should" vs "must," variant interpretations across DEC docs / libsixel / xterm. "100% compliance" requires picking a conformance reference per stack to break ties.
2. **Reference implementations disagree.** Kitty / WezTerm / xterm / libsixel render the same edge cases differently. Decision: per stack, pick one reference to follow.
3. **Golden image fragility.** Pixel-exact diff fails on font version, GPU driver, antialiasing variation. Needs per-pixel ΔE + structural similarity (SSIM) tolerance scheme.
4. **Matrix explosion.** Kitty alone has ~40 keys × ~6 actions × ~5 formats × placement permutations × animation frames. Cartesian product is millions. Smart matrix design = independent axes tested independently + a small set of multi-axis stress cases for known interaction points.
5. **Existing infrastructure exists** — `oriterm_core/tests/teseq/` already does "byte stream → parser → snapshot" via insta. 176 tests across 10 protocol families. Extend it: add "byte stream → parser → grid → GPU render → golden image." Same scenario format, second axis of verification. Don't build new infrastructure — extend what works.
6. **Audio + video aren't our problem.** notcurses uses ffmpeg for video. From our perspective video is just frames blitted via the chosen blitter — covered transitively by getting the blitter right. Audio we ignore.

**Suggested conformance references** (not yet decided):

| Stack | Conformance reference |
|---|---|
| Kitty graphics | kitty itself |
| Kitty keyboard | kitty itself |
| Sixel | libsixel + DEC STD 070 |
| ECMA-48 / VT220 / VT420 | xterm via ctlseqs |
| OSC 52 / OSC 8 | xterm |
| Mode 2026 | contour terminal's spec |
| Unicode subcell glyphs | Unicode chart PDFs (abstract glyph shapes) |

**Stack ordering candidates** (not yet decided):

(a) **Leverage on notcurses-demo** (visible wins fast):
1. Unicode subcell glyphs — unblocks ~50% of demo scenes that fall back to glyph blitters
2. Sixel — fallback path most legacy apps use
3. Kitty graphics — modern path notcurses prefers when available
4. CSI/SGR/Mode 2026 audit — backstop for everything else
5. OSC suite (palette, hyperlinks, clipboard, iTerm images)

(b) **Dependency order** (rigorous, slower to visible wins):
1. ECMA-48 / CSI / SGR baseline
2. Mode 2026 sync output
3. Unicode subcell glyphs
4. OSC suite
5. Sixel
6. Kitty graphics
7. Kitty keyboard

**Refined order suggestion** (Unicode-glyphs-first as framework MVP):
1. Unicode subcell glyphs — establishes visual axis + tolerance + golden storage + spec citation. No scenarios.
2. Sixel — adds parser, scenarios, grid integration. Reuses visual axis from stack 1.
3. Kitty graphics — adds chunking, animation, virtual placements, clock abstraction, matrix runner.
4. OSC suite — adds OSC parsing axis. Smaller stacks.
5. CSI/SGR/Mode 2026 audit — fills baseline gaps the first four uncovered.
6. Kitty keyboard — separate input axis, deferred until output is solid.

Rationale: the first stack picks the framework MVP. Unicode glyphs is the simplest possible visual axis, with no parser noise muddying the picture. Stack 2 (sixel) can rely on the visual axis being known-good and only adds parser/scenario complexity. From there, each stack adds one new dimension to the framework.

## 5. Framework growth curve (informational, not committed)

**Stack 1 forces the framework to have**:
- Scenario format with spec citation (each test points back to "Kitty spec §3.2 example 4")
- Input → grid state snapshot (insta-style — exists in teseq)
- Input → rendered frame → golden image diff with tolerance (new — visual axis)
- Spec coverage report ("which sections tested, which not")
- Per-test-binary scoping that fits the 150s runtime cap

**Stack 2 adds (depending on stack)**:
- Multi-frame capture if animation or sync output
- Reference-implementation invocation if goldens come from libsixel/kitty/xterm
- Matrix runner if parameter space large enough to need cartesian generation

**Stack 3 adds**:
- Virtual clock if animation timing matters
- Cross-stack regression sweeps so fixing kitty doesn't silently break sixel
- Coverage diff against previous stack's coverage report (no regressions)

**By stack N**: complete spec-conformance harness with citations, golden images, matrix testing, virtual time, multi-frame capture, coverage tracking, per-stack runtime budgeting. Built incrementally, every feature has a justification because some real test demanded it.

## 6. Specs to pull down (not yet done)

| Stack | Source | Format |
|---|---|---|
| Kitty graphics | sw.kovidgoyal.net/kitty/graphics-protocol/ | HTML → markdown |
| Kitty keyboard | sw.kovidgoyal.net/kitty/keyboard-protocol/ | HTML → markdown |
| Sixel (primary) | vt100.net DEC STD 070 / VT382 manual | PDF |
| Sixel (modern) | "All About SIXELs" (saitoha) | text |
| Sixel test corpus | github.com/saitoha/libsixel test images | binary |
| xterm ctlseqs | invisible-island.net/xterm/ctlseqs/ctlseqs.html | text |
| ECMA-48 | ecma-international.org | PDF |
| Mode 2026 | contour-terminal spec doc | markdown |
| Terminal Unicode Core | github.com/contour-terminal/terminal-unicode-core | text |
| Unicode 16 Symbols for Legacy Computing | unicode.org U+1FB00 / U+1CD00 charts | PDF |
| Notcurses' own emit notes | already at `reference_repos/console_repos/notcurses/TERMINALS.md` `USAGE.md` | local |
| WezTerm's compiled escape sequence list | already at `reference_repos/console_repos/wezterm/docs/escape-sequences.md` | local |

These should live in `~/projects/reference_repos/specs/` (out-of-tree, shared) or `~/projects/ori_term/specs/` (in-tree, versioned with code) — decision pending.

## 7. Open decisions blocking plan creation

These are the questions a plan author needs answered before writing the plan. They should be revisited after `tack-conformance` lands, since some may be answered by precedent.

1. **Conformance references** — agree with the suggested table in §4? Or pick differently per stack? This decision determines what "correct" means.
2. **Stack ordering** — leverage-first (a), dependency-first (b), or refined Unicode-first?
3. **Where do specs live** — `~/projects/reference_repos/specs/` (out-of-tree, shared) or `~/projects/ori_term/specs/` (in-tree, versioned)?
4. **Golden image generation strategy** — three options:
   - Render through reference implementation headlessly (need kitty + libsixel + xterm/Xvfb installed; reproducible only if pinned versions in a Docker image)
   - Hand-craft from spec text per test (slow, immune to reference drift)
   - Capture once from a manually-verified ori_term run, then pin (fast, tautological — only catches regressions)
5. **Tolerance policy for golden diffs** — exact match, ΔE per-pixel threshold, SSIM, or some combination?
6. **Test runtime budget** — 150-second cap is the rule. Matrix-heavy stacks won't fit one binary. OK with per-stack test binaries (`cargo test -p oriterm_core --test kitty_graphics`)?
7. **First stack = framework MVP** — Unicode subcell glyphs first, or override (e.g., sixel first for visible wins faster)?

## 8. Relationship to tack-conformance plan

`plans/tack-conformance/` is an existing in-progress plan (`section-01-shared-pty-session.md`, `section-02-terminfo-provisioning.md`) covering tack (the S-Lang terminfo testing tool) compliance. tack tests verify that our terminal correctly implements the capabilities we advertise via terminfo — the ECMA-48 / VT100/220 / xterm baseline.

**Why spec-conformance waits for tack-conformance**:
- tack-conformance covers the baseline our spec-conformance work depends on. If the baseline has bugs, spec-conformance kitty/sixel tests would have to fight through them.
- tack-conformance may establish testing patterns (PTY session, snapshot format, reference invocation) that we should inherit not reinvent.
- May resolve open decision 7 (first stack pick) by making "ECMA-48 baseline" effectively done before spec-conformance starts, letting us start at sixel or unicode glyphs without baseline interference.
- May answer open decisions 4 (golden generation) and 5 (tolerance) by precedent if tack-conformance has to make those calls for terminfo testing.

After tack-conformance lands, this research document should be re-read, updated where it has drifted, and used as the input to a `00-overview.md` + section files. The seven open decisions become input to the plan author.

## 9. Memory references

- `reference_wezterm_graphics.md` — full WezTerm sixel + kitty parser → GPU trace
- `reference_notcurses_demo.md` — 28-scene matrix with subsystem mapping
- `architecture_graphics_audit.md` — ori_term's current graphics protocol implementation status with bisection priorities
- This document — `plans/spec-conformance/research.md`
