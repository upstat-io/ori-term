---
section: 38
title: Terminal Protocol Extensions
status: not-started
reviewed: false
tier: 5
goal: Capability reporting, query/response sequences, extended SGR, window manipulation, DCS passthrough — the modern terminal protocol surface that applications rely on for progressive enhancement
sections:
  - id: "38.1"
    title: Device Attributes (DA1/DA2/DA3)
    status: not-started
  - id: "38.2"
    title: Device Status Reports (DSR/DECXCPR)
    status: not-started
  - id: "38.3"
    title: Mode Query (DECRQM)
    status: not-started
  - id: "38.4"
    title: Terminfo Capability Query (XTGETTCAP)
    status: not-started
  - id: "38.5"
    title: Setting Query (DECRQSS)
    status: not-started
  - id: "38.6"
    title: Color Queries & Reports (OSC 4/10/11/12)
    status: not-started
  - id: "38.7"
    title: Extended Underline Styles (SGR 4:x, SGR 58)
    status: not-started
  - id: "38.8"
    title: Window Manipulation (CSI t)
    status: not-started
  - id: "38.9"
    title: Additional Protocol Sequences
    status: not-started
  - id: "38.10"
    title: DCS Passthrough
    status: not-started
  - id: "38.11"
    title: Section Completion
    status: not-started
---

# Section 38: Terminal Protocol Extensions

**Status:** Not Started
**Goal:** Implement the modern terminal protocol surface that applications use for progressive enhancement and capability discovery. This section covers the query/response layer (DA, DSR, DECRQM, DECRQSS, XTGETTCAP), color reporting, extended SGR attributes, window manipulation sequences, and DCS passthrough for nested terminals. Without these, applications cannot detect what ori_term supports and must fall back to lowest-common-denominator behavior.

**Crate:** `oriterm_core` (response generation, SGR state) and `oriterm` (binary — window manipulation, passthrough)
**Dependencies:** `vte` (parser + handler), `oriterm_core` (TermMode, Palette, Grid)

**Prerequisite:**
- Section 02 complete (VTE parser handles CSI, DCS, OSC dispatch)
- Section 22 complete (terminal modes implemented — DECRQM queries mode state)
- Section 06 complete (font pipeline renders extended underline styles)

**Blocks:**
- Section 34 (IPC daemon must proxy query/response pairs correctly, namespace image IDs per pane)
- Section 37 (TUI client must translate or pass through capability queries to host terminal)

**Reference:**
- Ghostty: `src/terminal/Terminal.zig` (DA responses), `src/terminal/modes.zig` (DECRQM)
- Alacritty: `alacritty_terminal/src/term/mod.rs` (DA, DSR, DECRQSS)
- WezTerm: `term/src/terminalstate/performer.rs` (comprehensive query handling)
- xterm ctlseqs documentation (authoritative reference for all sequences)
- [XTGETTCAP](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Operating-System-Commands) — xterm terminfo query
- [Kitty keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) — for capability detection flow

**Why this matters:** Modern CLI tools (starship, zellij, helix, fish, nushell, delta, bat) probe the terminal via DA, DECRQM, XTGETTCAP, and color queries before enabling features. Without correct responses, these tools assume the terminal is dumb. This section is what makes ori_term a *discoverable* terminal — applications can ask "do you support X?" and get an answer, rather than guessing from `$TERM`.

---

## 38.1 Device Attributes (DA1/DA2/DA3)

Respond to Device Attribute queries so applications can identify the terminal type and supported features. This is the primary capability discovery mechanism — every terminal application sends DA1 on startup.

**Files:** `oriterm_core/src/term/handler.rs` (CSI dispatch)

**Reference:** Alacritty `alacritty_terminal/src/term/mod.rs` (DA responses), xterm ctlseqs

- [ ] **DA1 — Primary Device Attributes** (`CSI c` or `CSI 0 c`):
  - [ ] Response: `CSI ? 62 ; Ps ; ... c`
    - [ ] `62` = VT220 conformance level (standard for modern terminals)
    - [ ] Parameters advertise capabilities: `1` (132 cols), `4` (sixel), `22` (ANSI color)
  - [ ] ori_term response: `CSI ? 62 ; 4 c` (VT220 + sixel support)
    - [ ] Extend parameters as features are added (e.g., add `4` only after Section 22.7 ships sixel)
  - [ ] Response written to VTE response buffer, flushed by reader thread outside lock
- [ ] **DA2 — Secondary Device Attributes** (`CSI > c` or `CSI > 0 c`):
  - [ ] Response: `CSI > Pp ; Pv ; Pc c`
    - [ ] `Pp` = terminal type identifier (we use `1` for VT220-like, or a custom ID)
    - [ ] `Pv` = firmware/version number (encode `CARGO_PKG_VERSION` as integer, e.g., `0.1.0` → `100`)
    - [ ] `Pc` = 0 (ROM cartridge, always 0 for software terminals)
  - [ ] ori_term response: `CSI > 1 ; {version} ; 0 c`
- [ ] **DA3 — Tertiary Device Attributes** (`CSI = c`):
  - [ ] Response: `DCS ! | {unit-id} ST`
    - [ ] Unit ID: hex-encoded terminal identifier string
  - [ ] ori_term response: `DCS ! | 6F726974 ST` (`"orit"` in hex — short unique identifier)
- [ ] All DA responses must be generated and queued immediately (applications block on the response)
- [ ] **Tests** (`oriterm_core/src/term/tests.rs`):
  - [ ] DA1 produces `\x1b[?62;4c` (or correct parameter set)
  - [ ] DA2 produces `\x1b[>1;{version};0c`
  - [ ] DA3 produces `\x1bP!|6F726974\x1b\\`
  - [ ] Repeated DA queries produce identical responses (idempotent)

---

## 38.2 Device Status Reports (DSR/DECXCPR)

Report terminal status and cursor position. Applications use cursor position reports (CPR) extensively for layout detection, prompt rendering, and screen measurement.

**Files:** `oriterm_core/src/term/handler.rs` (CSI dispatch)

**Reference:** Alacritty `alacritty_terminal/src/term/mod.rs`, Ghostty `src/terminal/Terminal.zig`

- [ ] **DSR — Device Status Report** (`CSI 5 n`):
  - [ ] Response: `CSI 0 n` (terminal is OK, no malfunction)
  - [ ] Always responds with "OK" — no error states to report
- [ ] **CPR — Cursor Position Report** (`CSI 6 n`):
  - [ ] Response: `CSI Pr ; Pc R` where Pr = row (1-based), Pc = column (1-based)
  - [ ] When origin mode (DECOM) is set: report position relative to scroll region
  - [ ] When origin mode is reset: report absolute position
- [ ] **DECXCPR — Extended Cursor Position Report** (`CSI ? 6 n`):
  - [ ] Response: `CSI ? Pr ; Pc R` (same as CPR but with `?` prefix)
  - [ ] Some terminals add page parameter: `CSI ? Pr ; Pc ; 1 R`
  - [ ] ori_term: respond with `CSI ? Pr ; Pc R` (single-page terminal)
- [ ] **Cell size reporting** (`CSI 16 t`):
  - [ ] Response: `CSI 6 ; height ; width t` — cell dimensions in pixels
  - [ ] Read from current font metrics (`cell_width`, `cell_height`)
  - [ ] Applications use this for pixel-precise image placement
- [ ] **Text area size in pixels** (`CSI 14 t`):
  - [ ] Response: `CSI 4 ; height ; width t` — text area in pixels
  - [ ] Computed from grid dimensions * cell size
- [ ] **Text area size in characters** (`CSI 18 t`):
  - [ ] Response: `CSI 8 ; rows ; cols t` — grid dimensions
  - [ ] Read from current grid size
- [ ] **Tests** (`oriterm_core/src/term/tests.rs`):
  - [ ] DSR produces `\x1b[0n`
  - [ ] CPR at position (5, 10) produces `\x1b[5;10R` (1-based)
  - [ ] CPR with origin mode reports relative to scroll region
  - [ ] DECXCPR produces `\x1b[?5;10R`
  - [ ] Cell size report matches font metrics
  - [ ] Text area size matches grid * cell dimensions

---

## 38.3 Mode Query (DECRQM)

Allow applications to query whether a specific terminal mode is set, reset, or unsupported. This is the canonical progressive enhancement mechanism — an application sends DECRQM for Kitty keyboard mode, and if the terminal responds "not recognized", it falls back to legacy encoding.

**Files:** `oriterm_core/src/term/handler.rs` (CSI dispatch), `oriterm_core/src/term_mode.rs` (mode state)

**Reference:** Ghostty `src/terminal/modes.zig` (comptime mode table with DECRQM support), xterm ctlseqs

- [ ] **DECRQM for private modes** (`CSI ? Pm $ p`):
  - [ ] Response: `CSI ? Pm ; Ps $ y`
    - [ ] `Ps = 1`: mode is set
    - [ ] `Ps = 2`: mode is reset (known but not active)
    - [ ] `Ps = 0`: mode not recognized (unknown to this terminal)
    - [ ] `Ps = 3`: permanently set (always on, cannot be changed)
    - [ ] `Ps = 4`: permanently reset (always off, cannot be changed)
  - [ ] Query the `TermMode` bitflags for current state
  - [ ] Maintain a set of *recognized* mode numbers (from Section 22.6 mode table) to distinguish "reset" (2) from "not recognized" (0)
- [ ] **DECRQM for standard modes** (`CSI Pm $ p`):
  - [ ] Response: `CSI Pm ; Ps $ y` (no `?` prefix)
  - [ ] Same Ps values: 1 = set, 2 = reset, 0 = unknown
  - [ ] Covers: IRM (4), LNM (20)
- [ ] **Recognized mode registry**:
  - [ ] Build from Section 22.6 comprehensive mode table
  - [ ] All modes listed in 22.6 respond with 1 or 2
  - [ ] All other mode numbers respond with 0
  - [ ] As new modes are added in future, add to registry
- [ ] **Kitty keyboard mode query** (`CSI ? u`):
  - [ ] Not DECRQM but related — included here for completeness
  - [ ] Response: `CSI ? {flags} u` where flags = current keyboard mode bits
  - [ ] Already specified in Section 08.2 but ensure it works end-to-end here
- [ ] **Tests** (`oriterm_core/src/term/tests.rs`):
  - [ ] DECRQM for mode 2004 when set → `\x1b[?2004;1$y`
  - [ ] DECRQM for mode 2004 when reset → `\x1b[?2004;2$y`
  - [ ] DECRQM for mode 9999 (unknown) → `\x1b[?9999;0$y`
  - [ ] DECRQM for standard mode 4 (IRM) when set → `\x1b[4;1$y`
  - [ ] DECRQM for standard mode 4 (IRM) when reset → `\x1b[4;2$y`

---

## 38.4 Terminfo Capability Query (XTGETTCAP)

Allow applications to query specific terminfo capability values via DCS. This lets applications determine exact capability support without relying on `$TERM` or `infocmp`. Modern tools (fish, starship) increasingly use this.

**Files:** `oriterm_core/src/term/handler.rs` (DCS dispatch)

**Reference:** xterm ctlseqs (XTGETTCAP), Ghostty `src/terminal/Terminal.zig`, WezTerm `term/src/terminalstate/performer.rs`

- [ ] **XTGETTCAP** (`DCS + q Pt ST`):
  - [ ] `Pt` = hex-encoded terminfo capability name(s), separated by `;`
  - [ ] Response: `DCS 1 + r Pt = Pv ST` for each recognized capability
    - [ ] `Pt` = hex-encoded capability name (echo back)
    - [ ] `Pv` = hex-encoded capability value
  - [ ] For unrecognized capabilities: `DCS 0 + r Pt ST`
- [ ] **Supported capabilities** (minimum viable set):
  - [ ] `TN` (terminal name) → `oriterm`
  - [ ] `Co` / `colors` → `256` (or `16777216` for truecolor)
  - [ ] `RGB` → `8/8/8` (truecolor support indicator)
  - [ ] `Smulx` (extended underline) → `\x1b[4:%p1%dm` (styled underlines)
  - [ ] `Setulc` (underline color) → `\x1b[58:2::%p1%d:%p2%d:%p3%dm`
  - [ ] `Smol` (overline) → `\x1b[53m`
  - [ ] `Se` (cursor normal) → `\x1b[2 q`
  - [ ] `Ss` (cursor style) → `\x1b[%p1%d q`
  - [ ] `Ms` (clipboard via OSC 52) → `\x1b]52;%p1%s;%p2%s\x1b\\`
  - [ ] `title` variants → OSC 0/2 support
- [ ] **Hex encoding/decoding**:
  - [ ] Capability names are hex-encoded ASCII in the query
  - [ ] Decode query → lookup → hex-encode response
  - [ ] Example: query for `TN` arrives as `544E`, response value `oriterm` → `6F726974 65726D`
- [ ] **Multiple capabilities in one query**:
  - [ ] Capabilities separated by `;` in the query
  - [ ] Respond to each individually in order
  - [ ] Unknown capabilities get `DCS 0 + r ...` response
- [ ] **Tests** (`oriterm_core/src/term/tests.rs`):
  - [ ] Query `TN` returns hex-encoded `oriterm`
  - [ ] Query `colors` returns hex-encoded `256`
  - [ ] Query `RGB` returns hex-encoded `8/8/8`
  - [ ] Query unknown capability returns `DCS 0 + r` (not recognized)
  - [ ] Multi-capability query returns all results
  - [ ] Hex encode/decode round-trips correctly

---

## 38.5 Setting Query (DECRQSS)

Allow applications to query the current value of specific terminal settings (SGR attributes, scroll region, character set, etc.). Used for state synchronization and debugging.

**Files:** `oriterm_core/src/term/handler.rs` (DCS dispatch)

**Reference:** Alacritty `alacritty_terminal/src/term/mod.rs`, xterm ctlseqs

- [ ] **DECRQSS** (`DCS $ q Pt ST`):
  - [ ] `Pt` identifies the setting to query
  - [ ] Response: `DCS Ps $ r Pt ST`
    - [ ] `Ps = 1`: valid request, `Pt` = current setting value as escape sequence
    - [ ] `Ps = 0`: invalid/unknown request
- [ ] **Supported queries**:
  - [ ] `"m"` — current SGR attributes:
    - [ ] Reconstruct the SGR parameter string from current cell template
    - [ ] Example: bold + red foreground → `DCS 1 $ r 1;31 m ST`
    - [ ] Include extended attributes: underline style, underline color, overline
  - [ ] `"r"` — current scroll region (DECSTBM):
    - [ ] Response: `DCS 1 $ r {top};{bottom} r ST`
    - [ ] 1-based row numbers
  - [ ] `" q"` — current cursor style (DECSCUSR):
    - [ ] Response: `DCS 1 $ r {style} SP q ST`
    - [ ] Style: 0-6 per DECSCUSR
  - [ ] `"s"` — DECSLRM (left/right margins, if supported):
    - [ ] Response: `DCS 1 $ r {left};{right} s ST`
    - [ ] Or `DCS 0 $ r` if left/right margins not implemented
- [ ] **Tests** (`oriterm_core/src/term/tests.rs`):
  - [ ] Query `"m"` after `SGR 1;31` returns `DCS 1 $ r 1;31 m ST`
  - [ ] Query `"r"` after `DECSTBM 5;20` returns `DCS 1 $ r 5;20 r ST`
  - [ ] Query `" q"` after `DECSCUSR 4` returns `DCS 1 $ r 4 SP q ST`
  - [ ] Query unknown setting returns `DCS 0 $ r ST`

---

## 38.6 Color Queries & Reports (OSC 4/10/11/12)

Allow applications to query the terminal's current color palette and foreground/background/cursor colors. This is the primary mechanism for theme detection — applications read the background color luminance to determine if the terminal is using a light or dark theme.

**Files:** `oriterm_core/src/term/handler.rs` (OSC dispatch), `oriterm_core/src/color/palette.rs`

**Reference:** WezTerm `term/src/terminalstate/performer.rs`, termenv (Go library — canonical color detection), xterm ctlseqs

- [ ] **OSC 4 — Set/Query Indexed Colors** (`OSC 4 ; index ; spec ST`):
  - [ ] Set: `OSC 4 ; {index} ; {color-spec} ST` — set palette entry
  - [ ] Query: `OSC 4 ; {index} ; ? ST` — query palette entry
  - [ ] Response: `OSC 4 ; {index} ; rgb:{rr}/{gg}/{bb} ST`
    - [ ] `rr/gg/bb` are 4-digit hex (e.g., `rgb:ffff/0000/0000` for red)
    - [ ] xterm convention: 16-bit components, zero-extended from 8-bit
  - [ ] Support batch queries: multiple `index;?` pairs in one sequence
- [ ] **OSC 10 — Foreground Color** (`OSC 10 ; spec ST`):
  - [ ] Set: `OSC 10 ; {color-spec} ST` — set default foreground
  - [ ] Query: `OSC 10 ; ? ST` — query current foreground
  - [ ] Response: `OSC 10 ; rgb:{rr}/{gg}/{bb} ST`
- [ ] **OSC 11 — Background Color** (`OSC 11 ; spec ST`):
  - [ ] Set: `OSC 11 ; {color-spec} ST` — set default background
  - [ ] Query: `OSC 11 ; ? ST` — query current background
  - [ ] Response: `OSC 11 ; rgb:{rr}/{gg}/{bb} ST`
  - [ ] **Critical for theme detection**: applications query background luminance to choose light/dark output
- [ ] **OSC 12 — Cursor Color** (`OSC 12 ; spec ST`):
  - [ ] Set: `OSC 12 ; {color-spec} ST` — set cursor color
  - [ ] Query: `OSC 12 ; ? ST` — query current cursor color
  - [ ] Response: `OSC 12 ; rgb:{rr}/{gg}/{bb} ST`
- [ ] **OSC 104 — Reset Indexed Colors** (`OSC 104 ; index ST`):
  - [ ] Reset palette entry to default
  - [ ] `OSC 104 ST` (no index) resets all 256 entries to defaults
- [ ] **OSC 110/111/112 — Reset fg/bg/cursor colors**:
  - [ ] `OSC 110 ST` — reset foreground to default
  - [ ] `OSC 111 ST` — reset background to default
  - [ ] `OSC 112 ST` — reset cursor color to default (already in Section 22.2)
- [ ] **Color spec parsing** (shared across all OSC color operations):
  - [ ] `rgb:RR/GG/BB` or `rgb:RRRR/GGGG/BBBB` (1/2/4 hex digits per component)
  - [ ] `#RRGGBB` shorthand
  - [ ] Named colors (optional — `red`, `blue`, etc.)
  - [ ] `?` means query, not set
- [ ] **Tests** (`oriterm_core/src/term/tests.rs`):
  - [ ] OSC 11 query produces correct background color in `rgb:RRRR/GGGG/BBBB` format
  - [ ] OSC 10 query produces correct foreground color
  - [ ] OSC 4 set changes palette entry, query returns new value
  - [ ] OSC 104 resets palette entry to default
  - [ ] Color spec parsing handles `rgb:ff/00/ff`, `#ff00ff`, `rgb:ffff/0000/ffff`
  - [ ] Batch OSC 4 query returns all requested entries

---

## 38.7 Extended Underline Styles (SGR 4:x, SGR 58)

Support the Kitty-originated underline style extensions that are now widely adopted (foot, WezTerm, Ghostty, iTerm2, Contour). These enable rich text decoration in TUI applications — diagnostics underlines (curly for errors), semantic highlighting, and colored underlines.

**Files:** `oriterm_core/src/cell.rs` (CellFlags), `oriterm_core/src/term/handler.rs` (SGR dispatch), `oriterm/src/gpu/renderer.rs` (underline rendering is in Section 06 — this section handles the protocol/state side)

**Reference:** Kitty underline extension spec, Section 06 (rendering), Ghostty `src/terminal/sgr.zig`

- [ ] **Extended underline styles** (colon-separated SGR sub-parameters):
  - [ ] `SGR 4:0` — no underline (reset)
  - [ ] `SGR 4:1` — straight underline (same as `SGR 4`)
  - [ ] `SGR 4:2` — double underline
  - [ ] `SGR 4:3` — curly/wavy underline (used for spelling errors, diagnostics)
  - [ ] `SGR 4:4` — dotted underline
  - [ ] `SGR 4:5` — dashed underline
  - [ ] `SGR 24` — underline off (existing, unchanged)
- [ ] **Underline color**:
  - [ ] `SGR 58:2::{r}:{g}:{b}` — set underline color (24-bit RGB, colon sub-params)
  - [ ] `SGR 58:5:{index}` — set underline color (indexed, 256-color palette)
  - [ ] `SGR 59` — reset underline color to default (follows foreground color)
  - [ ] Underline color stored separately from foreground — requires `CellExtra` or dedicated field
- [ ] **Overline**:
  - [ ] `SGR 53` — enable overline (horizontal line above text)
  - [ ] `SGR 55` — disable overline
  - [ ] Rendered as 1px line at top of cell (rendering handled in Section 06)
- [ ] **Cell storage**:
  - [ ] `UnderlineStyle` enum: `None`, `Straight`, `Double`, `Curly`, `Dotted`, `Dashed`
  - [ ] Store in `CellFlags` (3 bits for style) or as a separate `u8` field
  - [ ] Underline color: `Option<Color>` in `CellExtra` (only allocated when non-default)
  - [ ] Overline flag in `CellFlags`
- [ ] **SGR colon sub-parameter parsing**:
  - [ ] VTE parser already handles colon-separated sub-parameters (`4:3` arrives as sub-params)
  - [ ] Verify `vte` crate dispatches sub-parameters correctly
  - [ ] Handle both colon-separated (correct: `4:3`) and semicolon-separated (legacy: `4;3` treated as two separate SGR params — `SGR 4` then `SGR 3` which is italic, NOT curly underline)
  - [ ] This distinction is critical: `\x1b[4:3m` = curly underline, `\x1b[4;3m` = underline + italic
- [ ] **Tests** (`oriterm_core/src/cell.rs` `#[cfg(test)]`, `oriterm_core/src/term/tests.rs`):
  - [ ] `SGR 4:3` sets curly underline style
  - [ ] `SGR 4;3` sets straight underline + italic (NOT curly) — critical distinction
  - [ ] `SGR 58:2::255:0:0` sets red underline color
  - [ ] `SGR 58:5:196` sets indexed underline color
  - [ ] `SGR 59` resets underline color to default
  - [ ] `SGR 53` enables overline flag
  - [ ] `SGR 55` disables overline flag
  - [ ] `SGR 24` clears underline style entirely
  - [ ] `SGR 0` resets all attributes including underline style and color

---

## 38.8 Window Manipulation (CSI t)

Handle xterm window manipulation sequences. These are security-sensitive — some operations (resize, move, iconify) should be gated behind configuration. Report-only operations are safe and should always respond.

**Files:** `oriterm/src/app/event_loop.rs` (window operations), `oriterm_core/src/term/handler.rs` (CSI dispatch)

**Reference:** xterm ctlseqs (Window manipulation), Alacritty (allows reports, blocks manipulation), WezTerm (configurable)

- [ ] **Report operations** (always enabled — read-only, no security concern):
  - [ ] `CSI 11 t` — report window state:
    - [ ] Response: `CSI 1 t` (not iconified) or `CSI 2 t` (iconified)
  - [ ] `CSI 13 t` — report window position:
    - [ ] Response: `CSI 3 ; x ; y t` (pixel position on screen)
  - [ ] `CSI 14 t` — report text area size in pixels:
    - [ ] Response: `CSI 4 ; height ; width t`
  - [ ] `CSI 16 t` — report cell size in pixels:
    - [ ] Response: `CSI 6 ; height ; width t`
  - [ ] `CSI 18 t` — report text area size in characters:
    - [ ] Response: `CSI 8 ; rows ; cols t`
  - [ ] `CSI 19 t` — report screen size in characters:
    - [ ] Response: `CSI 9 ; rows ; cols t` (full screen, not just terminal)
- [ ] **Manipulation operations** (gated behind `allow_window_ops` config):
  - [ ] `CSI 1 t` — de-iconify (restore/unminimize window)
  - [ ] `CSI 2 t` — iconify (minimize window)
  - [ ] `CSI 3 ; x ; y t` — move window to (x, y) pixel position
  - [ ] `CSI 4 ; height ; width t` — resize text area to height x width pixels
  - [ ] `CSI 5 t` — raise window to front
  - [ ] `CSI 6 t` — lower window behind others
  - [ ] `CSI 7 t` — refresh window
  - [ ] `CSI 8 ; rows ; cols t` — resize text area to rows x cols characters
  - [ ] `CSI 9 ; 0 t` — un-maximize
  - [ ] `CSI 9 ; 1 t` — maximize
  - [ ] `CSI 10 ; 0 t` — un-fullscreen
  - [ ] `CSI 10 ; 1 t` — fullscreen
  - [ ] `CSI 10 ; 2 t` — toggle fullscreen
  - [ ] Default: manipulation operations **disabled** (security — a malicious escape sequence in `cat`-ed file should not move/resize the window)
  - [ ] Config: `[security] allow_window_ops = false` (opt-in)
- [ ] **Tests** (`oriterm/src/app/tests.rs`):
  - [ ] `CSI 18 t` reports correct grid dimensions
  - [ ] `CSI 16 t` reports correct cell dimensions
  - [ ] `CSI 14 t` reports correct text area pixel dimensions
  - [ ] Window manipulation ops are no-ops when `allow_window_ops = false`
  - [ ] Window manipulation ops work when `allow_window_ops = true`
  - [ ] Unknown `CSI t` parameters are silently ignored

---

## 38.9 Additional Protocol Sequences

Parse and handle additional terminal protocol sequences used by modern applications.

**Files:** `oriterm_core/src/term/handler.rs`

**Reference:** Ghostty 1.3.0 release notes, Kitty protocol docs

- [ ] **Kitty OSC 66 — Text Sizing** (`OSC 66 ; ... ST`):
  - [ ] Parse Kitty text sizing protocol (font size adjustments from within the terminal)
  - [ ] Implementation: adjust font size or store as state for reporting
  - [ ] If not implementing full support, parse and discard gracefully (no error)
- [ ] **Kitty OSC 5522 — Extended Clipboard Protocol** (`OSC 5522 ; ... ST`):
  - [ ] Kitty's extended clipboard protocol (superset of OSC 52)
  - [ ] Supports MIME types, multiple clipboard targets, metadata
  - [ ] Parse and route to clipboard system (extend OSC 52 handling)
- [ ] **iTerm2 OSC 1337** (`OSC 1337 ; ... ST`):
  - [ ] Parse iTerm2 proprietary extensions (primarily inline images)
  - [ ] `File=...` — inline image protocol (alternative to Kitty graphics and sixel)
  - [ ] `SetUserVar=...` — user-defined variables
  - [ ] At minimum: parse without crashing, log unhandled sub-commands at debug level
  - [ ] Full inline image support deferred to Section 39 (Image Protocols)
- [ ] **ConEmu OSC 9 Full Subcommands** (`OSC 9;N;... ST`):
  - [ ] Subcommands 1-12 (currently only OSC 9;4 progress in Section 27.4)
  - [ ] `OSC 9;1` — Palette set
  - [ ] `OSC 9;2` — Palette query
  - [ ] `OSC 9;5` — Task notification
  - [ ] `OSC 9;6` — Process manipulation
  - [ ] Parse all, implement where useful, discard rest gracefully
- [ ] **CSI Scroll Up preserving scrollback** (`CSI n S`):
  - [ ] When CSI Scroll Up is executed, preserve scrolled-off lines in scrollback buffer instead of erasing them
  - [ ] Reference: Ghostty 1.3.0 — "CSI Scroll Up now preserves scrolled-off lines in scrollback buffer"
  - [ ] This matches user expectation: lines that scroll off the top should be accessible in scrollback
- [ ] **Tests:**
  - [ ] OSC 66 parsed without crash
  - [ ] OSC 5522 routes to clipboard system
  - [ ] OSC 1337 parsed without crash, unhandled sub-commands logged
  - [ ] ConEmu OSC 9 subcommands parsed
  - [ ] CSI S preserves lines in scrollback

---

## 38.10 DCS Passthrough

Support DCS passthrough for applications running inside nested terminals or multiplexers. When ori_term is the inner terminal, the outer terminal (tmux, screen, or another terminal emulator) needs to forward escape sequences. When ori_term is the outer terminal running a multiplexer, it must handle passthrough from inner sessions.

**Files:** `oriterm_core/src/term/handler.rs` (DCS dispatch)

**Reference:** tmux passthrough (`DCS tmux; ... ST`), WezTerm passthrough handling

- [ ] **tmux passthrough** (`DCS tmux; <escaped-sequence> ST`):
  - [ ] When ori_term hosts tmux: tmux wraps sequences for the outer terminal in `DCS tmux;`
  - [ ] Unescape the inner sequence (replace `\x1b\x1b` → `\x1b`) and process it
  - [ ] This allows tmux to set the outer terminal's title, clipboard, etc.
- [ ] **Kitty DCS passthrough** (for mux/daemon scenarios):
  - [ ] When `oriterm_mux` daemon receives image data, passthrough DCS to the rendering client
  - [ ] Passthrough must preserve binary data integrity (no re-encoding)
- [ ] **Generic DCS dispatch**:
  - [ ] Route unrecognized DCS sequences to a configurable handler
  - [ ] Log unrecognized DCS at debug level (do not drop silently — aids debugging)
- [ ] **Security**: DCS passthrough is only processed when the sequence comes from a running shell/application, never from pasted content (bracketed paste prevents this)
- [ ] **Tests** (`oriterm_core/src/term/tests.rs`):
  - [ ] tmux passthrough `DCS tmux; \x1b]2;title\x1b\\ ST` sets window title
  - [ ] Nested escape unescaping is correct (`\x1b\x1b` → `\x1b`)
  - [ ] Unrecognized DCS logged, not panicked

---

## 38.11 Section Completion

- [ ] All 38.1-38.10 items complete
- [ ] DA1/DA2/DA3 responses are correct and fast (no blocking)
- [ ] `fish` shell correctly detects ori_term capabilities via DA + XTGETTCAP
- [ ] `starship` prompt correctly detects color support via OSC 10/11 query
- [ ] `helix` editor correctly detects Kitty keyboard support via DECRQM
- [ ] DECRQM reports correct state for all modes in Section 22.6 table
- [ ] XTGETTCAP responds to all supported capability queries
- [ ] Color queries return correct palette, fg, bg, cursor colors
- [ ] Extended underline styles render correctly (curly, dotted, dashed, double)
- [ ] Underline colors work independently of foreground color
- [ ] Window size reports are accurate
- [ ] Window manipulation is disabled by default (security)
- [ ] DCS passthrough works for tmux running inside ori_term
- [ ] `cargo test` — all protocol extension tests pass
- [ ] `cargo clippy --target x86_64-pc-windows-gnu` — no warnings

**Exit Criteria:** ori_term is a fully discoverable terminal. Applications can query capabilities via DA, DECRQM, XTGETTCAP, and color queries and receive correct answers. Extended SGR attributes (underline styles, underline color, overline) are stored and rendered. The terminal responds correctly to all standard query/report sequences. Modern CLI tools (fish, starship, helix, delta, bat, nushell) auto-detect ori_term's capabilities without relying on `$TERM` heuristics.
