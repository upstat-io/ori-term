---
reroute: true
name: "Spec Conformance"
full_name: "Spec Conformance: Universal Terminal Protocol Verification"
status: active
order: 2
---

# Spec Conformance Index

> **Maintenance Notice:** Update this index when adding/modifying sections.
> **Supersedes:** `plans/tack-conformance/` (mechanical absorption — see 00-overview.md "Tack Absorption Strategy")

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Catalog Bootstrap
**File:** `section-01-catalog-bootstrap.md` | **Status:** Complete

```
catalog, sequence inventory, row schema, row ID, schema_version 0.1-provisional
plans/spec-conformance/catalog, 10-column schema, stable-symbol primary
phase 0a catalog map of territory, bottom-up harvest, top-down spec walk
reconciliation pass, de-facto categorization, MISSING row category
wezterm escape-sequences.md as De-facto ref only (not Spec source)
alacritty escape_support.md, ghostty src/lib_vt.zig, stable-symbol citations
ori_term VTE dispatch tables, PM SOS parser states, State::SosPmApcString
crates/vte/src/ansi/dispatch/mod.rs, csi.rs, osc.rs, crates/vte/src/lib.rs
PM privacy message discard, SOS start of string discard, stub verification
committed deterministic real-app captures, plans/spec-conformance/captures
vim tmux htop btop less nvim notcurses-demo scripted flows
captures manifest.toml, sha256, idle-capture rejection, 20 unique tuple threshold
reconciliation-report.md committed audit trail
catalog_coverage_check Rust binary, crates/oriterm_test_support/src/bin/catalog_coverage_check.rs
shared library crates/oriterm_test_support/src/catalog/mod.rs (consumed by spec_coverage_report in Section 04.8)
sibling tests crates/oriterm_test_support/src/catalog/tests.rs per test-organization.md
positive pin negative pin cross type matrix test, tests.md matrix testing rule
missed tuple duplicate ID stale symbol line-number-primary verified bootstrap wezterm spec-source
spec corpus assembly, plans/spec-conformance/specs, manifest.toml, sha256, fetch script
manifest-fetch.sh verify mode, redistributable license gate
ecma-48.md, xterm-ctlseqs.md, dec-private-modes.md, osc.md, sixel.md
kitty-graphics.md, kitty-keyboard.md, iterm2.md, mode-2026.md
unicode-subcell.md, mouse.md, charsets.md, audio-print.md
shell-integration.md, historical.md, de-facto-behaviors.md
catalog/README.md stub owned by Section 01, extended by Section 04.7
_legacy-tack-mapping.md owned by Section 02.4 (NOT Section 01)
authority ladder, conformance reference, per-stack tiebreaker
verification status taxonomy, missing, stub, implemented-unverified
verified verified-partial verified-with-deviation FORBIDDEN in Section 01
audit memory verification, architecture_graphics_audit.md, MEMORY.md
stale claim discovery, broken window policy, HSL hue kitty q=1 image cache 320 MiB
provisional row schema, schema_version 0.1-provisional, freeze gate Section 04.7
bug-tracker filing oriterm_core/src/term/handler/image/kitty.rs 476 lines BLOAT
blocks Section 12 Sixel and Section 13 Kitty Graphics
csi.rs 390 lines image cache 436 lines NOTE informational only
subsections 01.1 01.2 01.3 01.4 01.5 01.6 01.7 01.8 01.9 01.10 01.11
chronological dependency fixed: corpus assembly before top-down walk
```

---

### Section 02: Tack-Conformance Absorption
**File:** `section-02-tack-absorption.md` | **Status:** Complete

```
tack absorption, plan hygiene, mechanical migration, no file moves
plans/tack-conformance/index.md, plans/tack-conformance/00-overview.md
supersede notice, status: resolved, reroute frontmatter
catalog/_legacy-tack-mapping.md, mapping table file
spec catalog row to tack section ID, citation stability
git history preservation, --follow, no rename churn
```

---

### Section 03: Effect Boundary Migration
**File:** `section-03-effect-boundary-migration.md` | **Status:** Complete

```
Effect type, EffectSink, oriterm_core::effect, production interface
Effect::Pty, Effect::Host, Effect::HostRequest, Effect::Ui, Effect::Presentation
PtyEffect, HostEffect, UiEffect, PresentationEffect
ResponseToken, request/response, no closures, typed reply
LegacyEventSink, Event::ClipboardLoad migration, Event::ColorRequest migration
oriterm_core/src/event/mod.rs:46, oriterm_core/src/event/mod.rs:50
Term::pending_notifications, drain_notifications, shell_state.rs:218
EffectSink::drain_into, bulk drain, host adapter coalescing
desktop notifications, OSC 9, OSC 99, OSC 777, push_notification
fire-and-forget vs request/response, separate abstractions
```

---

### Section 04: Verification Chain Harness + Pilots + Coverage Report + Cataloging Safety Net
**File:** `section-04-verification-chain-harness.md` | **Status:** Complete

```
verification chain, test ladder, harness foundation, MVP framework
SpecHarness, SpecScenario, SpecRunner, per-rung test runner
effect transcript capture, snapshot_seqno tracking
presentation gate observation, SyncBegin, SyncCommit
TeseqHarness extension, visual_regression extension, render_frame_cached
sixel pilot, DA1 pilot, non-visual pilot, visual pilot
catalog row schema freeze, schema MVP, frozen template
spec-coverage-report, cargo run -p oriterm_test_support --bin
coverage tracking, per-stack absolute verified count, monotonic gating
citation scan, catalog row ID grep, false-verified detection
uncataloged citation detection, uncataloged-backlog.md
cataloging safety net, UncatalogedDetector, continuous delta detection
regression detection, absolute count not percentage
BLOAT split, gpu/prepare/mod.rs 504, gpu/prepare/dirty_skip/mod.rs 506
section 04 to 05 coupling, 04.4 04.5 04.7 blocked until 05.6 lands
```

---

### Section 05: Golden Lane Determinism
**File:** `section-05-golden-lane-determinism.md` | **Status:** Complete

```
GPU determinism, software rasterizer, llvmpipe, pinned adapter
oriterm/src/gpu/state/mod.rs:150, headless_env_with_pinned_adapter
HintingMode, grayscale alpha, oriterm/src/gpu/visual_regression/mod.rs:87
golden lane, exact tolerance, per-pixel ΔE, SSIM diagnostic only
PIXEL_TOLERANCE, MAX_MISMATCH_PERCENT, ORITERM_UPDATE_GOLDEN
pinned cell metrics, pinned font, pinned cell width/height
canonical lane Linux x86_64, real-GPU smoke non-gating
font bytes, glyph format, DPI, viewport pixels, opacity, filtering
animation clock, blink phase, locale, LANG
```

---

### Section 06: Terminal Mode Plumbing
**File:** `section-06-terminal-mode-plumbing.md` | **Status:** Complete

```
mode 2026, sync output, synchronized output, presentation gates
Processor::sync_timeout, Processor::stop_sync, timeout-abort
oriterm_mux/src/pane/io_thread/mod.rs, sync_bytes_count
publication suppression, snapshot_seqno
crossbeam_channel select! deadline-aware, default(timeout) arm
StdSyncHandler::sync_timeout, Option<Instant>, no duplicated state
post_parse_housekeeping extraction, shared normal + timeout path
PresentationEffect::Abort, SyncAbortReason::Timeout
Abort docstring flush not discard, stop_sync replays bytes
LegacyEventSink drops Presentation effects, silent drop fix
named_private_mode_number elimination, mode as u16, WASTE removal
NamedPrivateMode 6 sync points, compile-time exhaustive match guard
named_private_mode_flag retained, crate boundary, no registry in vte
resize during sync, alt-screen swap during sync, double-publish prevention
nested BSU, max-buffer-bytes, sync abort, SYNC_UPDATE_TIMEOUT 150ms
```

---

### Section 07: Image Lifecycle Correctness
**File:** `section-07-image-lifecycle-correctness.md` | **Status:** Complete

```
image lifecycle, image_cache resize, ImagePlacement, PlacementSizing
StableRowIndex, reflow remapping, ReflowMapping struct
oriterm_core/src/image/cache/mod.rs, cache/lifecycle.rs extraction
oriterm_core/src/grid/resize/mod.rs, reflow_cells first_output_row mapping
prune_scrollback, remove_placements_in_region, on_resize, remap_placements
update_cell_coverage, FixedPixels, CellCount, sizing modes
cell-metric plumbing, set_cell_dimensions, ImageConfig extension
app → mux → Term cell dimensions, sync_grid_layout, handle_dpi_change
alt-screen swap, alt_image_cache, alt cache existence condition
ED/EL erase, scrollback eviction, column out-of-bounds removal
reflow row split, reflow row merge, total_evicted adjustment
image+resize+reflow regression matrix, 3 protocols x 2 sizing x 7 mutations = 42
cache/tests.rs new test file, negative rendering pin
Kitty placeholder U+10EEEE cell-attachment, section 13 owns
notcurses keller scene, BUG-08-9 resolved
```

---

### Section 08: ECMA-48 Baseline (absorbs in-flight tack work)
**File:** `section-08-ecma-48-baseline.md` | **Status:** Not Started

```
ECMA-48, baseline, CSI, SGR, ED, EL, IL, DL, ICH, DCH, ECH, REP
CUP, CUU, CUD, CUF, CUB, SU, SD, DECSTBM
SM, RM, IRM, LNM, basic DEC modes
8-bit C1 controls, 0x9B CSI, 0x90 DCS, 0x9F APC
DECLRMM left/right margins, grid enforcement
catalog row subset ownership, baseline mode subset
catalog/_legacy-tack-mapping.md population
plans/tack-conformance section 01-06 references
oriterm_core/tests/teseq, csi_cursor, csi_erase, csi_insert_delete
ori_term.info terminfo entry, tic compilation
PtySession, ScenarioRunner, TOOLS_MENU_INVENTORY
```

---

### Section 09: DEC Private Modes (full)
**File:** `section-09-dec-private-modes.md` | **Status:** Complete

```
DEC private modes, DECSET, DECRST, ?Ps h, ?Ps l
mode 1003, 1004, 1005, 1006, 1007, 1015, 1016, 1034, 1036, 1042
mode 1047, 1048, 1049, 2004, 2026, 2031, 8452, 9001
DECCKM, DECCOLM, DECSCNM, DECOM, DECAWM, DECARM, DECTCEM
SIXEL_SCROLLING, SIXEL_CURSOR_RIGHT, REVERSE_VIDEO
ALTERNATE_SCROLL, REVERSE_WRAP, FOCUS_IN_OUT, BRACKETED_PASTE
SYNC_UPDATE mode 2026, color scheme update mode 2031
contour-terminal spec, oriterm_core/src/term/mode/mod.rs
```

---

### Section 10: OSC Suite (full)
**File:** `section-10-osc-suite.md` | **Status:** Complete

```
OSC, operating system command, OSC dispatch
OSC 0 (icon+title), OSC 1 (icon name), OSC 2 (window title)
OSC 4 (palette set/query), OSC 7 (CWD), OSC 8 (hyperlinks)
OSC 9 (notifications), OSC 10/11/12 (default colors), OSC 22 (cursor icon)
OSC 50 (cursor shape legacy), OSC 52 (clipboard), OSC 99 (notifications)
OSC 104 (palette reset), OSC 110/111/112 (default color reset)
OSC 133 (semantic prompt FTCS), OSC 633 (VS Code)
OSC 777 (kitty notifications), OSC 1337 (iTerm2 inline images minimal)
hyperlink, URI, gist:egmontkob, clipboard store/load, base64
oriterm_core/src/term/handler/osc.rs, crates/vte/src/ansi/dispatch/osc.rs
PromptMarker, PendingMarks, OSC 133;A/B/C/D, command_start, finish_command
```

---

### Section 09A: DEC Private CSI Extensions (rect ops + presentation + audits/ SSOT)
**File:** `section-09a-dec-csi-extensions.md` | **Status:** Complete

```
DEC private CSI extensions, DECRQCRA checksum, rectangular operations
DECCRA copy rectangle, DECFRA fill rectangle, DECERA erase rectangle
DECSERA selective erase, DECRARA reverse attributes, DECCARA change attributes
DECSACE select attribute change extent, XTCHECKSUM, XTREPORTSGR
DECIC insert column, DECDC delete column, DECBI back index, DECFI forward index
DECRQPSR presentation state report, DECRQUPSS user-preferred supplemental set
DECRQDE displayed extent, DECSCL conformance level, DECSCA character protection
DECSASD active status display, DECSSDT status line type
DECRQSS request status string (DCS), DECRSPS restore presentation status (DCS)
DCS Pid !~ checksum response, synchronous PtyEffect::Write from VTE handler
plans/spec-conformance/audits/, top-down spec audit SSOT, audit-files lint
spec-coverage-report --check audit-files, esctest as spec source
catalog/dec-rectangle-ops.md, catalog/dec-presentation.md
crates/vte/src/ansi/dispatch/csi.rs, oriterm_core/src/term/handler/status.rs
DECRECT prefix, DECPRES prefix, audits/section-NN-top-down-inventory.md
```

---

### Section 11: Unicode Subcell Glyphs (incl. octants)
**File:** `section-11-unicode-subcell-glyphs.md` | **Status:** Complete

```
unicode subcell, builtin glyphs, Canvas abstraction
half blocks U+2580 U+2584, blocks.rs
quadrants U+2596 U+2597 U+2598 U+2599 U+259A U+259B U+259C U+259D U+259E U+259F
sextants U+1FB00 U+1FB3B, oriterm/src/gpu/builtin_glyphs/legacy_computing
octants U+1CD00 U+1CDE5, Unicode 16, Symbols for Legacy Computing Supplement, U+1CC00 U+1CEBF
braille U+2800 U+28FF, braille.rs
powerline, nerd font, oriterm/src/gpu/builtin_glyphs/powerline.rs
2x3 grid, 2x4 grid, 4-bit bitmask
notcurses keller blitter exhaustive, all 7 blitters
hand-crafted goldens, Unicode chart PDFs
```

---

### Section 12: Sixel
**File:** `section-12-sixel.md` | **Status:** Not Started

```
sixel, DCS q, DEC STD 070, libsixel, saitoha
P1 pan, P2 pad, P5 width, P6 height, raster attrs
sixel data, color map, define color, select color, repeat
HLS rotation, hue 120 degrees, color/mod.rs hls_to_rgb
oriterm_core/src/image/sixel/mod.rs
oriterm_core/src/term/handler/image/sixel.rs
SIXEL_SCROLLING mode 80, SIXEL_CURSOR_RIGHT mode 8452, DECSDM
background transparency, palette[0] fill
crates/vte/src/ansi/dispatch/mod.rs:52
ImageCache, ImageTextureCache
oriterm/src/gpu/image_render/mod.rs
```

---

### Section 13: Kitty Graphics Protocol
**File:** `section-13-kitty-graphics.md` | **Status:** Not Started

```
kitty graphics, APC _G, kitty graphics protocol
key=value pairs, a, q, t, f, m, c, r, w, h, X, Y, z, C
i, I, p, d, U, q, e, S, P, V
chunked transmission, base64 accumulator, coalesce, decode
direct, direct binary, file, temporary file, shared memory
RGB f=24, RGBA f=32, PNG f=100, zlib o=z
delete actions, transmit, place, animate, virtual placement
animation, frame composition, transmit frame, compose frame
Overwrite, AlphaBlend, AnimRgba8
unicode placeholder protocol, virtual placements
oriterm_core/src/image/kitty/parse.rs
oriterm_core/src/term/handler/image/kitty.rs
oriterm_core/src/term/handler/image/kitty_animation.rs
```

---

### Section 14: iTerm2 Inline Images
**File:** `section-14-iterm2-images.md` | **Status:** Not Started

```
iTerm2, OSC 1337, File=, inline image
base64 image data, name, size, width, height
inline, download, doNotMoveCursor, type
GIF frame extraction, multi-frame
oriterm_core/src/term/handler/image/iterm2.rs
iTerm2 OSC suite, SetMark, RemoteHost, CurrentDir
```

---

### Section 15: Cell-Level Alpha + Transparency
**File:** `section-15-cell-level-alpha.md` | **Status:** Not Started

```
cell-level alpha, transparency, translucent overlays
notcurses trans scene, multi-plane composition
Cell struct alpha field, oriterm_core/src/cell/mod.rs
CellFlags ALPHA, premultiplied alpha, straight alpha
GPU pipeline, BlendState, oriterm/src/gpu/pipeline/image.rs
PREMULTIPLIED_ALPHA_BLENDING, opacity, fg_dim, bg_alpha
NCALPHA_OPAQUE, NCALPHA_TRANSPARENT, NCALPHA_BLEND
RenderableCell alpha extraction, FrameInput palette
plane stacking, multi-plane Z order
```

---

### Section 16: Mouse Protocols
**File:** `section-16-mouse-protocols.md` | **Status:** Not Started

```
mouse protocols, mouse encoding, mouse reporting
X10 mouse (DECSET 9), normal mouse (1000), button-event (1002)
any-event (1003), focus events (1004), UTF-8 mouse (1005)
SGR mouse (1006), URXVT mouse (1015), SGR pixels (1016)
locator mode (1001)
press, release, motion, drag, button code, modifiers
shift +4, alt +8, ctrl +16, wheel
oriterm/src/app/mouse_report/encode.rs
CSI < button ; col ; row M/m, CSI M Cb Cx Cy
```

---

### Section 17: Kitty Keyboard Protocol
**File:** `section-17-kitty-keyboard.md` | **Status:** Not Started

```
kitty keyboard, CSI > u, CSI = u, CSI < u, CSI ? u
disambiguate escape codes, report event types, report alternate keys
report all keys as escape, report associated text
mode flags, push, pop, query, set
keyboard mode stack, KeyboardModes
modifier reporting, base layout key, shifted key
event type, press, release, repeat
modifyOtherKeys mode 1, mode 2, xterm
Win32 Input mode 9001, ConPTY input
key encoding, KeyEncoder, oriterm/src/key_encoding
sw.kovidgoyal.net/kitty/keyboard-protocol/
```

---

### Section 18: Charsets + UAX Policy
**File:** `section-18-charsets-and-uax-policy.md` | **Status:** Not Started

```
character sets, charset designation, ISO 2022, ECMA-35
G0, G1, G2, G3 designation, GL active, GR active
locking shift, single shift, SS2, SS3, LS2, LS3
ESC ( B, ESC ) B, ESC * B, ESC + B, ESC ( 0, ESC ) 0
DEC special graphics, DEC line drawing, DEC technical, DEC supplemental
DEC dingbats, StandardCharset, attr.rs:204
NRCS variants, national replacement character sets
ANSI X3.4, BS, DE, FI, FR, FR_CA, IT, NL, NO, PT, SE, SP, SU, CH
JIS Roman, JIS Kana, KOR, ARA, GREEK, HEB, RUS, TUR
ISO 8859 family, ISO 2022 multibyte, JIS X 0208, GB 2312, KSC 5601
UTF-8 decoding, error recovery
UAX #11 East Asian Width, CJK width 2
UAX #29 grapheme clustering, ZWJ, extended cluster
UAX #9 bidi, embedding levels
variation selectors VS15 VS16, emoji ZWJ sequences
unicode-width, unicode-segmentation
oriterm_core/src/term/charset/mod.rs
```

---

### Section 19: Historical LEGACY CONTROL Stacks
**File:** `section-19-historical-stacks.md` | **Status:** Not Started

```
historical legacy control stacks, DEC heritage
no deferral forks, every stack implemented not verified-with-deviation
VT52, ESC A B C D F H I J K Y Z
VT100, VT102, VT220 8-bit + downloadable
VT320 rectangular editing + page memory
VT420 left/right margins, VT520 525 color
DEC LK201 keyboard protocol, LK201 scan codes, LK201 DA2 identification
Wyse 50, Wyse 60, attribute byte, protected mode, status line, key programming
ADM-3A, dumb terminal, ESC = row col cursor addressing
IBM PC ANSI.SYS, MS-DOS ANSI extensions, keyboard reassignment CSI p
Microsoft Console Virtual Terminal Sequences
(ReGIS and Tek 4014 and vector_raster helper are Section 26, not Section 19)
```

---

### Section 20: Audio + Print
**File:** `section-20-audio-and-print.md` | **Status:** Not Started

```
audio, BEL, bell character, 0x07
ANSI music, CSI M, music notation, MML
DECPS, DEC play sound, ESC [ Vol Note Tones p
visual bell DECVB, screen flash
print, print screen CSI i, auto print mode
print form, print extent
file transfer detection, Zmodem, Kermit, passthrough
```

---

### Section 21: notcurses-demo Harness + Scene Matrix + qrcode smoke
**File:** `section-21-notcurses-demo-harness.md` | **Status:** Not Started

```
notcurses-demo harness, PTY recording, replay infrastructure
scene matrix, per-scene golden capture
qrcode scene smoke test, simplest scene
notcurses-demo binary /usr/bin/notcurses-demo
incremental scene gates, partial pass tracking
~/projects/reference_repos/console_repos/notcurses
```

---

### Section 22: Real-App E2E Harness
**File:** `section-22-real-app-harness.md` | **Status:** Not Started

```
real-app harness, PTY recording, replay infrastructure
captured PTY trace, snapshot capture pipeline
recorded daily-driver scenario, snapshot golden, diff
first app smoke test, vim simple session
script utility, ttyrec, asciinema
```

---

### Section 23: Cross-Stack Regression Sweep + Coverage CI
**File:** `section-23-cross-stack-regression-sweep.md` | **Status:** Not Started

```
cross-stack regression, regression sweep, every PR
coverage report, spec-coverage-report, automated count
verified row count, per-stack percentage
build failure on regression, verified to lower drop
per-stack test binary, 150-second test cap
GitHub Actions, .github/workflows/spec-conformance.yml
per-platform apex matrix, OS-dependent apices
clipboard, audio, focus, kitty file/shm transports
title, shell integration, platform-specific verification
legacy test removal, teseq removal, tack removal, vttest removal
external tool dependency elimination, zero SKIP messages
self-contained tests, no platform-specific binaries
```

---

### Section 24: notcurses-demo FULL-PASS Milestone
**File:** `section-24-notcurses-demo-full-pass.md` | **Status:** Not Started

```
notcurses-demo full-pass, all 28 scenes, integration milestone
ixetunchdmbkywjgarvlsfqzo default order
intro xray eagle trans uniblock normal chunli highcon dragon mojibake
box keller yield whiteout jungle grid animate reel view sliders
fission zoo qrcode outro
keller all 7 blitters, trans multi-plane alpha, uniblock blitter mode
mojibake unicode catalog
per-scene correctness criterion, glitch bisection
PTY capture replay, byte stream minimization
fallback chain kitty sixel octant sextant quadrant half-block braille ASCII
```

---

### Section 25: Real-App FULL-PASS Milestone
**File:** `section-25-real-app-full-pass.md` | **Status:** Not Started

```
real-app full-pass, daily-driver applications
vim, neovim, helix, htop, btop, tmux, aerc, ncmpcpp, less, nvim
recorded session, captured byte stream
snapshot golden, regression matrix
treesitter highlighting, git log -p, ripgrep colored
```

---

### Section 26: Historical VECTOR Stacks (vector_raster + ReGIS + Tek 4010/4014)
**File:** `section-26-historical-vector-stacks.md` | **Status:** Not Started

```
historical vector stacks, vector graphics, rasterizer
shared vector_raster helper, oriterm_core/src/vector_raster
VectorCanvas, Bresenham line, midpoint circle, midpoint arc, Catmull-Rom curve
even-odd fill polygon, stroke text, to_image_placement
DEC ReGIS, ReGIS graphics, ReGIS command interpreter
ReGIS parser, ReGIS interpreter, DCS p introducer
Tektronix 4010, Tektronix 4014, Tek byte-pair coordinate decoder
Tek alpha vs graphics mode, Tek rasterizer, GS US ESC FF
depends on section 05 (deterministic lane for rasterizer goldens)
depends on section 07 (image lifecycle, ImageCache::on_resize)
depends on section 08 (baseline parser/dispatch)
```

---

## Catalog Files

| File | Stacks Covered |
|---|---|
| `catalog/_legacy-tack-mapping.md` | mapping table from spec catalog row IDs to legacy tack section IDs (created empty by 02; populated by 08) |
| `catalog/ecma-48.md` | ECMA-48 CSI/SGR/modes baseline |
| `catalog/xterm-ctlseqs.md` | xterm extensions: window, focus, bracketed paste, DECRQM |
| `catalog/dec-private-modes.md` | every DECSET/DECRST private mode |
| `catalog/dec-rectangle-ops.md` | DEC private CSI rectangular-area ops: DECRQCRA, DECCRA, DECFRA, DECERA, DECSERA, DECRARA, DECCARA, DECSACE, XTCHECKSUM, XTREPORTSGR (added by Section 09A) |
| `catalog/dec-presentation.md` | DEC private CSI presentation/column ops + DCS-path presentation queries: DECIC, DECDC, DECBI, DECFI, DECRQPSR, DECRQUPSS, DECRQDE, DECSCL, DECSCA, DECSASD, DECSSDT, DECRQSS, DECRSPS (added by Section 09A) |
| `catalog/osc.md` | OSC registry: 0, 1, 2, 4, 7, 8, 9, 10, 11, 12, 22, 50, 52, 99, 104, 110-112, 133, 633, 777, 1337 |
| `catalog/sixel.md` | DCS q + raster attrs + transparency + DECSDM |
| `catalog/kitty-graphics.md` | APC _G + every key + chunked + animation + virtual placements + unicode placeholders |
| `catalog/kitty-keyboard.md` | CSI > u + 5 disambiguation modes |
| `catalog/iterm2.md` | OSC 1337 + iTerm2 OSC suite |
| `catalog/mode-2026.md` | sync output + presentation gates + timeout-abort |
| `catalog/unicode-subcell.md` | half-blocks, quadrants, sextants, octants, braille, legacy computing |
| `catalog/mouse.md` | every numbered mouse protocol + locator |
| `catalog/charsets.md` | DEC charsets, NRCS, ISO 2022, ISO 8859, UAX policies |
| `catalog/audio-print.md` | BEL, ANSI music, DECPS, visual bell, print sequences |
| `catalog/shell-integration.md` | OSC 7, OSC 9/99/777, OSC 133, OSC 633, command timing |
| `catalog/historical.md` | VT52, VT100/102/220/320/420/520, DEC LK201 keyboard, Wyse 50/60, ADM-3A, IBM PC ANSI.SYS, Microsoft Console VT (legacy control — Section 19); ReGIS, Tek 4014 (vector — Section 26) |
| `catalog/de-facto-behaviors.md` | sequences with no spec, reference impl tiebreakers cited per row |

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Catalog Bootstrap | `section-01-catalog-bootstrap.md` |
| 02 | Tack-Conformance Absorption (Phase 0b) | `section-02-tack-absorption.md` |
| 03 | Effect Boundary Migration | `section-03-effect-boundary-migration.md` |
| 04 | Verification Chain Harness + Pilots + Coverage Report | `section-04-verification-chain-harness.md` |
| 05 | Golden Lane Determinism | `section-05-golden-lane-determinism.md` |
| 06 | Terminal Mode Plumbing | `section-06-terminal-mode-plumbing.md` |
| 07 | Image Lifecycle Correctness | `section-07-image-lifecycle-correctness.md` |
| 08 | ECMA-48 Baseline | `section-08-ecma-48-baseline.md` |
| 09 | DEC Private Modes (full) | `section-09-dec-private-modes.md` |
| 09A | DEC Private CSI Extensions (rect ops + presentation + audits/ SSOT) | `section-09a-dec-csi-extensions.md` |
| 10 | OSC Suite (full) | `section-10-osc-suite.md` |
| 11 | Unicode Subcell Glyphs (incl. octants) | `section-11-unicode-subcell-glyphs.md` |
| 12 | Sixel | `section-12-sixel.md` |
| 13 | Kitty Graphics Protocol | `section-13-kitty-graphics.md` |
| 14 | iTerm2 Inline Images | `section-14-iterm2-images.md` |
| 15 | Cell-Level Alpha + Transparency | `section-15-cell-level-alpha.md` |
| 16 | Mouse Protocols | `section-16-mouse-protocols.md` |
| 17 | Kitty Keyboard Protocol | `section-17-kitty-keyboard.md` |
| 18 | Charsets + UAX Policy | `section-18-charsets-and-uax-policy.md` |
| 19 | Historical LEGACY CONTROL Stacks (VT52, LK201, Wyse, ADM-3A, IBM PC, MS Console) | `section-19-historical-stacks.md` |
| 20 | Audio + Print | `section-20-audio-and-print.md` |
| 21 | notcurses-demo Harness + Scene Matrix + qrcode smoke | `section-21-notcurses-demo-harness.md` |
| 22 | Real-App E2E Harness | `section-22-real-app-harness.md` |
| 23 | Cross-Stack Regression Sweep + Coverage CI | `section-23-cross-stack-regression-sweep.md` |
| 24 | notcurses-demo FULL-PASS Milestone | `section-24-notcurses-demo-full-pass.md` |
| 25 | Real-App FULL-PASS Milestone | `section-25-real-app-full-pass.md` |
| 26 | Historical VECTOR Stacks (vector_raster + ReGIS + Tek 4010/4014) | `section-26-historical-vector-stacks.md` |
