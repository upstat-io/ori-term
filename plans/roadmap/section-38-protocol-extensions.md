---
section: 38
title: Terminal Protocol Extensions
status: in-progress
reviewed: false
third_party_review:
  status: none
  updated: null
tier: 5
last_verified: "2026-03-29"
goal: Capability reporting, query/response sequences, extended SGR, window manipulation, DCS passthrough — the modern terminal protocol surface that applications rely on for progressive enhancement
sections:
  - id: "38.1"
    title: Device Attributes (DA1/DA2/DA3)
    status: in-progress
  - id: "38.2"
    title: Device Status Reports (DSR/DECXCPR)
    status: in-progress
  - id: "38.3"
    title: Mode Query (DECRQM)
    status: in-progress
  - id: "38.4"
    title: Terminfo Capability Query (XTGETTCAP)
    status: not-started
  - id: "38.5"
    title: Setting Query (DECRQSS)
    status: not-started
  - id: "38.6"
    title: Color Queries & Reports (OSC 4/10/11/12)
    status: in-progress
  - id: "38.7"
    title: Extended Underline Styles (SGR 4:x, SGR 58)
    status: in-progress
  - id: "38.8"
    title: Window Manipulation (CSI t)
    status: in-progress
  - id: "38.9"
    title: Additional Protocol Sequences
    status: in-progress
  - id: "38.10"
    title: DCS Passthrough
    status: not-started
  - id: "38.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "38.11"
    title: Section Completion
    status: not-started
---

# Section 38: Terminal Protocol Extensions

**Status:** In Progress (~50% complete across subsections)
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

> **Verification notes (2026-03-29):**
> - Section was marked not-started but is significantly implemented. Updated to in-progress.
> - **38.1** (~80%): DA1 and DA2 implemented and tested. DA3 not implemented. DA1 response is `\x1b[?6;4c` (ANSI color + sixel), plan says `\x1b[?62;4c` (VT220 conformance level 62). Needs decision on correct format.
> - **38.2** (~60%): DSR (`CSI 5 n`), CPR (`CSI 6 n`), CSI 14 t, CSI 18 t all implemented. DECXCPR not implemented. **BUG: CPR does not check `TermMode::ORIGIN` for scroll-region-relative reporting.**
> - **38.3** (~90%): DECRQM substantially implemented. 26+ private modes + 2 ANSI modes. Kitty keyboard mode query implemented. Unknown modes return 0.
> - **38.4** (0%): Not started. Needs VTE parser DCS dispatch for `DCS + q`.
> - **38.5** (0%): Not started. Needs VTE parser DCS dispatch for `DCS $ q`.
> - **38.6** (~95%): OSC 4/10/11/12 set, query, and reset all implemented and tested. Near-complete.
> - **38.7** (~90%): All underline styles (straight, double, curly, dotted, dashed) + underline color (SGR 58) fully implemented. Overline (SGR 53/55) missing.
> - **38.8** (~25%): Only CSI 14 t and CSI 18 t implemented. CSI 16 t (cell size), window state/position reports, and manipulation ops not implemented.
> - **38.9** (~15%): iTerm2 inline images implemented. Other sequences not started.
> - **38.10** (0%): Not started.

---

## 38.1 Device Attributes (DA1/DA2/DA3)

Respond to Device Attribute queries so applications can identify the terminal type and supported features. This is the primary capability discovery mechanism — every terminal application sends DA1 on startup.

**Files:** `oriterm_core/src/term/handler.rs` (CSI dispatch)

**Reference:** Alacritty `alacritty_terminal/src/term/mod.rs` (DA responses), xterm ctlseqs

- [x] **DA1 — Primary Device Attributes** (`CSI c` or `CSI 0 c`): (verified 2026-03-29)
  - [x] Response implemented: `\x1b[?6;4c` (ANSI color + sixel) in `oriterm_core/src/term/handler/status.rs:58-66` (verified 2026-03-29)
  - [ ] **NOTE**: Plan says `\x1b[?62;4c` (VT220 conformance level 62). Actual sends `\x1b[?6;4c`. Most modern terminals (xterm) use `62` or `65` (VT500). Needs decision on correct format.
  - [x] Response written to VTE response buffer (verified 2026-03-29)
- [x] **DA2 — Secondary Device Attributes** (`CSI > c` or `CSI > 0 c`): (verified 2026-03-29)
  - [x] Response: `\x1b[>0;{version};1c` with `crate_version_number()` in `status.rs:67-72` (verified 2026-03-29)
  - [x] Version encoding: `crate_version_number()` in `helpers.rs:85-95` converts semver to integer (verified 2026-03-29)
- [ ] **DA3 — Tertiary Device Attributes** (`CSI = c`):
  - [ ] Response: `DCS ! | {unit-id} ST`
    - [ ] Unit ID: hex-encoded terminal identifier string
  - [ ] ori_term response: `DCS ! | 6F726974 ST` (`"orit"` in hex — short unique identifier)
  - [ ] Not implemented -- `Some(c)` for other intermediates logs "Unsupported DA intermediate" and discards
- [x] All DA responses must be generated and queued immediately (applications block on the response) (verified 2026-03-29)
- [ ] **Tests** (`oriterm_core/src/term/tests.rs`):
  - [x] DA1 test exists in `oriterm_core/src/term/handler/tests.rs` (verified 2026-03-29)
  - [x] DA2 test exists in `oriterm_core/src/term/handler/tests.rs` (verified 2026-03-29)
  - [ ] DA3 produces `\x1bP!|6F726974\x1b\\`
  - [ ] Repeated DA queries produce identical responses (idempotent)

---

## 38.2 Device Status Reports (DSR/DECXCPR)

Report terminal status and cursor position. Applications use cursor position reports (CPR) extensively for layout detection, prompt rendering, and screen measurement.

**Files:** `oriterm_core/src/term/handler.rs` (CSI dispatch)

**Reference:** Alacritty `alacritty_terminal/src/term/mod.rs`, Ghostty `src/terminal/Terminal.zig`

- [x] **DSR — Device Status Report** (`CSI 5 n`): (verified 2026-03-29)
  - [x] Response: `\x1b[0n` in `status.rs:79-83` (verified 2026-03-29)
- [x] **CPR — Cursor Position Report** (`CSI 6 n`): (verified 2026-03-29)
  - [x] Response: `\x1b[{line};{col}R` (1-based) in `status.rs:84-89` (verified 2026-03-29)
  - [ ] **BUG**: Does not check `TermMode::ORIGIN` for scroll-region-relative reporting. Code uses `cursor().line() + 1` and `cursor().col().0 + 1` but ignores DECOM mode.
- [ ] **DECXCPR — Extended Cursor Position Report** (`CSI ? 6 n`):
  - [ ] Response: `CSI ? Pr ; Pc R` (same as CPR but with `?` prefix)
  - [ ] Not implemented -- no handler for the `?` variant
- [ ] **Cell size reporting** (`CSI 16 t`):
  - [ ] Response: `CSI 6 ; height ; width t` — cell dimensions in pixels
  - [ ] Not implemented
- [x] **Text area size in pixels** (`CSI 14 t`): (verified 2026-03-29)
  - [x] Response: `\x1b[4;{h};{w}t` in `dcs.rs:119-134` (verified 2026-03-29)
- [x] **Text area size in characters** (`CSI 18 t`): (verified 2026-03-29)
  - [x] Response: `\x1b[8;{lines};{cols}t` in `status.rs:96-101` (verified 2026-03-29)
  - [x] `text_area_size_chars_reports_dimensions` test exists (verified 2026-03-29)
- [ ] **Tests** (`oriterm_core/src/term/tests.rs`):
  - [x] DSR produces `\x1b[0n` (verified 2026-03-29)
  - [x] CPR test exists (verified 2026-03-29)
  - [ ] CPR with origin mode reports relative to scroll region (blocked by ORIGIN mode bug)
  - [ ] DECXCPR produces `\x1b[?5;10R`
  - [ ] Cell size report matches font metrics
  - [x] Text area size matches grid dimensions (verified 2026-03-29)

---

## 38.3 Mode Query (DECRQM)

Allow applications to query whether a specific terminal mode is set, reset, or unsupported. This is the canonical progressive enhancement mechanism — an application sends DECRQM for Kitty keyboard mode, and if the terminal responds "not recognized", it falls back to legacy encoding.

**Files:** `oriterm_core/src/term/handler.rs` (CSI dispatch), `oriterm_core/src/term_mode.rs` (mode state)

**Reference:** Ghostty `src/terminal/modes.zig` (comptime mode table with DECRQM support), xterm ctlseqs

- [x] **DECRQM for private modes** (`CSI ? Pm $ p`): (verified 2026-03-29)
  - [x] `status_report_private_mode()` handles all `NamedPrivateMode` variants, returns `0` for unknown in `status.rs:43-55` (verified 2026-03-29)
  - [x] `mode_report_value()` returns 1 (set) or 2 (reset) in `helpers.rs:17-19` (verified 2026-03-29)
  - [x] 26+ modes mapped in `named_private_mode_number()` in `helpers.rs:22-49` (verified 2026-03-29)
  - [x] `named_private_mode_flag()` maps to `TermMode` bitflags in `helpers.rs:53-79` (verified 2026-03-29)
- [x] **DECRQM for standard modes** (`CSI Pm $ p`): (verified 2026-03-29)
  - [x] `status_report_mode()` handles `NamedMode::Insert` (4) and `LineFeedNewLine` (20), returns `0` for unknown in `status.rs:26-39` (verified 2026-03-29)
- [x] **Recognized mode registry**: (verified 2026-03-29)
  - [x] 26+ private modes + 2 ANSI modes covered (verified 2026-03-29)
  - [x] Unknown modes correctly return `0` (not recognized) (verified 2026-03-29)
- [x] **Kitty keyboard mode query** (`CSI ? u`): (verified 2026-03-29)
  - [x] `dcs_report_keyboard_mode()` responds `\x1b[?{bits}u` in `dcs.rs:87-96` (verified 2026-03-29)
- [ ] **Tests** (`oriterm_core/src/term/tests.rs`):
  - [ ] DECRQM for mode 2004 when set -> `\x1b[?2004;1$y`
  - [ ] DECRQM for mode 2004 when reset -> `\x1b[?2004;2$y`
  - [ ] DECRQM for mode 9999 (unknown) -> `\x1b[?9999;0$y`
  - [ ] DECRQM for standard mode 4 (IRM) when set -> `\x1b[4;1$y`
  - [ ] DECRQM for standard mode 4 (IRM) when reset -> `\x1b[4;2$y`

---

## 38.4 Terminfo Capability Query (XTGETTCAP)

Allow applications to query specific terminfo capability values via DCS. This lets applications determine exact capability support without relying on `$TERM` or `infocmp`. Modern tools (fish, starship) increasingly use this.

**Files:** `oriterm_core/src/term/handler.rs` (DCS dispatch)

**Reference:** xterm ctlseqs (XTGETTCAP), Ghostty `src/terminal/Terminal.zig`, WezTerm `term/src/terminalstate/performer.rs`

- [ ] **XTGETTCAP** (`DCS + q Pt ST`): *(blocked: VTE parser has no DCS dispatch for `DCS + q`)*
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

- [ ] **DECRQSS** (`DCS $ q Pt ST`): *(blocked: VTE parser has no DCS dispatch for `DCS $ q`)*
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

- [x] **OSC 4 — Set/Query Indexed Colors** (`OSC 4 ; index ; spec ST`): (verified 2026-03-29)
  - [x] Set: `osc_set_color()` sets palette entries via `self.palette.set_indexed(index, color)` in `osc.rs:70-82` (verified 2026-03-29)
  - [x] Query: `osc_dynamic_color_sequence()` sends `Event::ColorRequest` with closure in `osc.rs:98-111` (verified 2026-03-29)
  - [x] Response format: `rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}` (4-digit hex per component) (verified 2026-03-29)
- [x] **OSC 10 — Foreground Color** (`OSC 10 ; spec ST`): (verified 2026-03-29)
  - [x] Set and query both implemented (verified 2026-03-29)
- [x] **OSC 11 — Background Color** (`OSC 11 ; spec ST`): (verified 2026-03-29)
  - [x] Set and query both implemented (verified 2026-03-29)
- [x] **OSC 12 — Cursor Color** (`OSC 12 ; spec ST`): (verified 2026-03-29)
  - [x] Set and query both implemented (verified 2026-03-29)
- [x] **OSC 104 — Reset Indexed Colors** (`OSC 104 ; index ST`): (verified 2026-03-29)
  - [x] `osc_reset_color()` resets palette entries to defaults in `osc.rs:85-91` (verified 2026-03-29)
- [x] **OSC 110/111/112 — Reset fg/bg/cursor colors**: (verified 2026-03-29)
  - [x] All three implemented (verified 2026-03-29)
- [x] **Color spec parsing** (shared across all OSC color operations): (verified 2026-03-29)
  - [x] `Event::ColorRequest` carries index + formatting closure in `event/mod.rs:50` (verified 2026-03-29)
  - [x] Full palette management in `palette.rs` (verified 2026-03-29)
- [x] **Tests** (`oriterm_core/src/term/handler/tests.rs`): (verified 2026-03-29)
  - [x] Color query/set tests exist (verified 2026-03-29)

---

## 38.7 Extended Underline Styles (SGR 4:x, SGR 58)

Support the Kitty-originated underline style extensions that are now widely adopted (foot, WezTerm, Ghostty, iTerm2, Contour). These enable rich text decoration in TUI applications — diagnostics underlines (curly for errors), semantic highlighting, and colored underlines.

**Files:** `oriterm_core/src/cell.rs` (CellFlags), `oriterm_core/src/term/handler.rs` (SGR dispatch), `oriterm/src/gpu/renderer.rs` (underline rendering is in Section 06 — this section handles the protocol/state side)

**Reference:** Kitty underline extension spec, Section 06 (rendering), Ghostty `src/terminal/sgr.zig`

- [x] **Extended underline styles** (colon-separated SGR sub-parameters): (verified 2026-03-29)
  - [x] `CellFlags`: `UNDERLINE`, `DOUBLE_UNDERLINE`, `CURLY_UNDERLINE`, `DOTTED_UNDERLINE`, `DASHED_UNDERLINE`, `ALL_UNDERLINES` mask in `cell/mod.rs:22-46` (verified 2026-03-29)
  - [x] SGR dispatch: `Attr::Underline`, `DoubleUnderline`, `Undercurl`, `DottedUnderline`, `DashedUnderline` all handled in `sgr.rs:26-45` with mutual exclusion via `remove(ALL_UNDERLINES)` (verified 2026-03-29)
- [x] **Underline color**: (verified 2026-03-29)
  - [x] `SGR 58`: `Attr::UnderlineColor(color)` handled, stored in `CellExtra::underline_color` in `sgr.rs:60`, `cell/mod.rs:63` (verified 2026-03-29)
  - [x] `SGR 59`: `Attr::Reset` clears underline color in `sgr.rs:21` (verified 2026-03-29)
  - [x] `set_underline_color()` allocates `CellExtra` lazily in `cell/mod.rs:196-226` (verified 2026-03-29)
- [ ] **Overline**:
  - [ ] `SGR 53` — enable overline -- NOT IMPLEMENTED (no `OVERLINE` flag in `CellFlags`, no `Attr::Overline` in VTE parser)
  - [ ] `SGR 55` — disable overline -- NOT IMPLEMENTED
  - [ ] Requires cross-cutting change: VTE `Attr` enum, VTE SGR dispatch, `CellFlags`, SGR apply, GPU rendering, HTML export
- [x] **Cell storage**: (verified 2026-03-29)
  - [x] Underline styles stored via `CellFlags` bit flags (verified 2026-03-29)
  - [x] Underline color in `CellExtra` (lazy allocation) (verified 2026-03-29)
  - [ ] Overline flag in `CellFlags` -- NOT IMPLEMENTED
- [x] **SGR colon sub-parameter parsing**: (verified 2026-03-29)
  - [x] VTE parser dispatches sub-parameters correctly (verified 2026-03-29)
- [x] **Tests** (extensive, verified 2026-03-29):
  - [x] `sgr_curly_underline`, `sgr_underline_color_truecolor`, `sgr_59_clears_underline_color`, `sgr_dotted_underline`, `sgr_dashed_underline`, `sgr_underline_color_survives_underline_type_change`, `sgr_underline_color_256`, `underline_styles_are_mutually_exclusive` (verified 2026-03-29)
  - [x] HTML export: underline styles map correctly in `selection/html/mod.rs:425-428` (verified 2026-03-29)
  - [x] Snapshot: `underline_color` resolved through palette in `term/snapshot.rs:105-121` (verified 2026-03-29)
  - [ ] `SGR 53` enables overline flag -- NOT IMPLEMENTED
  - [ ] `SGR 55` disables overline flag -- NOT IMPLEMENTED

---

## 38.8 Window Manipulation (CSI t)

Handle xterm window manipulation sequences. These are security-sensitive — some operations (resize, move, iconify) should be gated behind configuration. Report-only operations are safe and should always respond.

**Files:** `oriterm/src/app/event_loop.rs` (window operations), `oriterm_core/src/term/handler.rs` (CSI dispatch)

**Reference:** xterm ctlseqs (Window manipulation), Alacritty (allows reports, blocks manipulation), WezTerm (configurable)

- [ ] **Report operations** (always enabled — read-only, no security concern):
  - [ ] `CSI 11 t` — report window state -- NOT IMPLEMENTED
  - [ ] `CSI 13 t` — report window position -- NOT IMPLEMENTED
  - [x] `CSI 14 t` — report text area size in pixels: `\x1b[4;{h};{w}t` in `dcs.rs:119-134` (verified 2026-03-29)
  - [ ] `CSI 16 t` — report cell size in pixels -- NOT IMPLEMENTED
  - [x] `CSI 18 t` — report text area size in characters: `\x1b[8;{lines};{cols}t` in `status.rs:96-101` (verified 2026-03-29)
  - [ ] `CSI 19 t` — report screen size in characters -- NOT IMPLEMENTED
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
- [x] **iTerm2 OSC 1337** (`OSC 1337 ; ... ST`): (partially verified 2026-03-29)
  - [x] `File=...` — inline image protocol implemented via `handle_iterm2_file()` in `handler/image/iterm2.rs` (verified 2026-03-29)
  - [ ] `SetUserVar=...` — user-defined variables -- NOT IMPLEMENTED
  - [x] Parse without crashing for known sub-commands (verified 2026-03-29)
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

## 38.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 38.11 Section Completion

**Already complete (verified 2026-03-29):**
- [x] DA1/DA2 responses are correct and fast (verified 2026-03-29)
- [x] DECRQM reports correct state for 26+ private modes + 2 ANSI modes (verified 2026-03-29)
- [x] Color queries return correct palette, fg, bg, cursor colors via OSC 4/10/11/12 (verified 2026-03-29)
- [x] Extended underline styles render correctly (curly, dotted, dashed, double) (verified 2026-03-29)
- [x] Underline colors work independently of foreground color (verified 2026-03-29)
- [x] CSI 14 t and CSI 18 t window size reports accurate (verified 2026-03-29)
- [x] iTerm2 inline images via OSC 1337 (verified 2026-03-29)

**Remaining for full completion:**
- [ ] All 38.1-38.10 items complete
- [ ] DA3 response implemented
- [ ] DA1 response format decision (current `?6;4c` vs plan `?62;4c`)
- [ ] CPR origin mode bug fixed
- [ ] DECXCPR implemented
- [ ] CSI 16 t (cell size) implemented
- [ ] XTGETTCAP responds to all supported capability queries (blocked by VTE parser work)
- [ ] DECRQSS responds to setting queries (blocked by VTE parser work)
- [ ] Overline (SGR 53/55) implemented (cross-cutting VTE change)
- [ ] Window manipulation ops gated behind `allow_window_ops` config
- [ ] DCS passthrough works for tmux running inside ori_term
- [ ] `cargo test` — all protocol extension tests pass
- [ ] `cargo clippy --target x86_64-pc-windows-gnu` — no warnings

- [ ] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)

**Exit Criteria:** ori_term is a fully discoverable terminal. Applications can query capabilities via DA, DECRQM, XTGETTCAP, and color queries and receive correct answers. Extended SGR attributes (underline styles, underline color, overline) are stored and rendered. The terminal responds correctly to all standard query/report sequences. Modern CLI tools (fish, starship, helix, delta, bat, nushell) auto-detect ori_term's capabilities without relying on `$TERM` heuristics.
