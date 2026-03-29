# Section 38: Terminal Protocol Extensions -- Verification Results

**Verified:** 2026-03-29
**Status in plan:** not-started
**Actual status:** PARTIALLY IMPLEMENTED -- several subsections have substantial code already

---

## Codebase Search Evidence

### 38.1 Device Attributes (DA1/DA2/DA3)

| Search | Result |
|--------|--------|
| DA1 response | **IMPLEMENTED** -- `status_identify_terminal(None)` responds `\x1b[?6;4c` (VT220 + ANSI color + sixel) | `oriterm_core/src/term/handler/status.rs:58-66` |
| DA2 response | **IMPLEMENTED** -- `status_identify_terminal(Some('>'))` responds `\x1b[>0;{version};1c` with `crate_version_number()` | `status.rs:67-72` |
| DA3 response | **NOT IMPLEMENTED** -- `Some(c)` for other intermediates logs "Unsupported DA intermediate" and discards. No `'='` handler for DA3. | `status.rs:73` |
| Version encoding | **IMPLEMENTED** -- `crate_version_number()` in `helpers.rs:85-95` converts semver to integer |
| Tests | DA1/DA2 tests exist in `oriterm_core/src/term/handler/tests.rs` |

**Verdict:** DA1 and DA2 are implemented and tested. DA3 (tertiary) is not implemented.

**Plan accuracy issue:** Plan says DA1 response should be `\x1b[?62;4c` (VT220 conformance level 62). Actual implementation responds `\x1b[?6;4c` (attribute 6 = ANSI color, attribute 4 = sixel). The "62" in the plan is the VT220 conformance level encoding; the actual "6" is just the ANSI color attribute. This may be intentional (different encoding style) or a bug in the implementation. Reference: xterm sends `\x1b[?62;...c` where 62 = VT220 level 2. Alacritty sends `\x1b[?6c`.

### 38.2 Device Status Reports (DSR/DECXCPR)

| Search | Result |
|--------|--------|
| DSR (CSI 5 n) | **IMPLEMENTED** -- `status_device_status(5)` responds `\x1b[0n` | `status.rs:79-83` |
| CPR (CSI 6 n) | **IMPLEMENTED** -- `status_device_status(6)` responds `\x1b[{line};{col}R` (1-based) | `status.rs:84-89` |
| DECXCPR (CSI ? 6 n) | **NOT IMPLEMENTED** -- no handler for the `?` variant |
| CSI 16 t (cell size) | **NOT IMPLEMENTED** -- not in `status.rs` (plan's 38.2 lists this but it's a CSI t report, properly in 38.8) |
| CSI 14 t (text area pixels) | **IMPLEMENTED** -- `dcs_text_area_size_pixels()` in `dcs.rs:119-134` responds `\x1b[4;{h};{w}t` |
| CSI 18 t (text area chars) | **IMPLEMENTED** -- `status_text_area_size_chars()` in `status.rs:96-101` responds `\x1b[8;{lines};{cols}t` |
| CPR origin mode | **NEEDS VERIFICATION** -- code uses `cursor().line() + 1` and `cursor().col().0 + 1` but doesn't check `TermMode::ORIGIN` for scroll region relative reporting |
| Tests | `text_area_size_chars_reports_dimensions` test exists |

**Verdict:** DSR and basic CPR implemented. DECXCPR (extended CPR with `?` prefix) not implemented. CPR may not correctly handle origin mode. CSI 14 t and CSI 18 t both implemented.

### 38.3 Mode Query (DECRQM)

| Search | Result |
|--------|--------|
| DECRQM for ANSI modes | **IMPLEMENTED** -- `status_report_mode()` handles `NamedMode::Insert` (4) and `LineFeedNewLine` (20), returns `0` for unknown | `status.rs:26-39` |
| DECRQM for private modes | **IMPLEMENTED** -- `status_report_private_mode()` handles all `NamedPrivateMode` variants, returns `0` for unknown | `status.rs:43-55` |
| Mode report values | **IMPLEMENTED** -- `mode_report_value()` returns 1 (set) or 2 (reset) | `helpers.rs:17-19` |
| Named private mode table | **IMPLEMENTED** -- 26 modes mapped in `named_private_mode_number()` covering cursor keys, origin, line wrap, cursor visibility, mouse modes, bracketed paste, sync update, sixel | `helpers.rs:22-49` |
| Named private mode flags | **IMPLEMENTED** -- `named_private_mode_flag()` maps to `TermMode` bitflags | `helpers.rs:53-79` |
| Kitty keyboard mode query | **IMPLEMENTED** -- `dcs_report_keyboard_mode()` responds `\x1b[?{bits}u` | `dcs.rs:87-96` |

**Verdict:** DECRQM is substantially implemented. The mode registry covers 26+ private modes and 2 ANSI modes. Unknown modes correctly return `0` (not recognized). This subsection is largely complete.

### 38.4 Terminfo Capability Query (XTGETTCAP)

| Search | Result |
|--------|--------|
| `xtgettcap` / `XTGETTCAP` | **Not found** in any source file |
| `DCS + q` handler | **Not found** -- no DCS dispatch for XTGETTCAP in the VTE handler |
| `fn xtgettcap` | **Not found** |
| VTE handler trait | No `xtgettcap` method in `crates/vte/src/ansi/handler.rs` |

**Verdict:** Truly not started. The VTE handler trait doesn't even have a method for XTGETTCAP, so implementing this requires both VTE-level parser support and Term-level response generation.

### 38.5 Setting Query (DECRQSS)

| Search | Result |
|--------|--------|
| `DECRQSS` / `decrqss` / `report_setting` | **Not found** |
| `DCS $ q` handler | **Not found** |

**Verdict:** Truly not started. Like XTGETTCAP, requires VTE parser support.

### 38.6 Color Queries & Reports (OSC 4/10/11/12)

| Search | Result |
|--------|--------|
| OSC 4/10/11/12 set | **IMPLEMENTED** -- `osc_set_color()` sets palette entries via `self.palette.set_indexed(index, color)` | `osc.rs:70-82` |
| OSC 4/10/11/12 query | **IMPLEMENTED** -- `osc_dynamic_color_sequence()` sends `Event::ColorRequest` with closure that formats `rgb:RRRR/GGGG/BBBB` | `osc.rs:98-111` |
| OSC 104/110/111/112 reset | **IMPLEMENTED** -- `osc_reset_color()` resets palette entries to defaults | `osc.rs:85-91` |
| Color spec format | **IMPLEMENTED** -- response uses `rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}` format (4-digit hex per component via zero-extension) | `osc.rs:106` |
| Event::ColorRequest | **IMPLEMENTED** -- carries index + formatting closure | `event/mod.rs:50` |
| Palette resolution | **IMPLEMENTED** -- `palette.rs` has full palette management |
| Tests | Color query/set tests exist in `oriterm_core/src/term/handler/tests.rs` |

**Verdict:** Color queries and reports are substantially implemented. Set, query, and reset all work for OSC 4/10/11/12/104/110/111/112. This subsection appears complete or near-complete.

### 38.7 Extended Underline Styles (SGR 4:x, SGR 58)

| Search | Result |
|--------|--------|
| Underline styles in CellFlags | **IMPLEMENTED** -- `UNDERLINE`, `DOUBLE_UNDERLINE`, `CURLY_UNDERLINE`, `DOTTED_UNDERLINE`, `DASHED_UNDERLINE`, `ALL_UNDERLINES` mask | `cell/mod.rs:22-46` |
| SGR dispatch for underline styles | **IMPLEMENTED** -- `Attr::Underline`, `Attr::DoubleUnderline`, `Attr::Undercurl`, `Attr::DottedUnderline`, `Attr::DashedUnderline` all handled in `sgr::apply()` with mutual exclusion via `remove(ALL_UNDERLINES)` | `sgr.rs:26-45` |
| Underline color (SGR 58) | **IMPLEMENTED** -- `Attr::UnderlineColor(color)` handled, stored in `CellExtra::underline_color` | `sgr.rs:60`, `cell/mod.rs:63` |
| SGR 59 (reset underline color) | **IMPLEMENTED** -- `Attr::Reset` clears underline color | `sgr.rs:21` |
| CellExtra for underline color | **IMPLEMENTED** -- `set_underline_color()` allocates `CellExtra` lazily | `cell/mod.rs:196-226` |
| Overline (SGR 53/55) | **NOT IMPLEMENTED** -- no `OVERLINE` flag in `CellFlags`, no `Attr::Overline` in VTE parser |
| Tests | Extensive: `sgr_curly_underline`, `sgr_underline_color_truecolor`, `sgr_59_clears_underline_color`, `sgr_dotted_underline`, `sgr_dashed_underline`, `sgr_underline_color_survives_underline_type_change`, `sgr_underline_color_256`, `underline_styles_are_mutually_exclusive` | `handler/tests.rs` |
| HTML export | Underline styles map correctly in HTML export | `selection/html/mod.rs:425-428` |
| Snapshot | `underline_color` resolved through palette in snapshot path | `term/snapshot.rs:105-121` |

**Verdict:** Extended underline styles (SGR 4:x) and underline color (SGR 58) are FULLY IMPLEMENTED and well-tested. Only overline (SGR 53/55) is missing. This subsection is 90%+ complete.

### 38.8 Window Manipulation (CSI t)

| Search | Result |
|--------|--------|
| CSI 14 t (text area pixels) | **IMPLEMENTED** | `dcs.rs:119-134` |
| CSI 18 t (text area chars) | **IMPLEMENTED** | `status.rs:96-101` |
| CSI 16 t (cell size pixels) | **NOT IMPLEMENTED** |
| CSI 11/13/19 t (reports) | **NOT IMPLEMENTED** |
| CSI 1-10 t (manipulation) | **NOT IMPLEMENTED** |
| `allow_window_ops` config | **NOT IMPLEMENTED** |
| Window state query (iconified/not) | **NOT IMPLEMENTED** |

**Verdict:** Only CSI 14 t and CSI 18 t are implemented. The rest (cell size report, window position, screen size, manipulation operations, security gating) are not implemented.

### 38.9 Additional Protocol Sequences

| Search | Result |
|--------|--------|
| iTerm2 `iterm2_file` handler | **IMPLEMENTED** -- `handle_iterm2_file()` exists for inline images | `handler/mod.rs:432-434`, `handler/image/iterm2.rs` |
| OSC 1337 (general) | Inline image part is implemented; user vars and other sub-commands are not |
| Kitty OSC 66 (text sizing) | **NOT IMPLEMENTED** |
| Kitty OSC 5522 (clipboard) | **NOT IMPLEMENTED** |
| ConEmu OSC 9 subcommands | Unknown -- would need deeper search |
| CSI S scrollback preservation | Unknown |

**Verdict:** Partially implemented (iTerm2 inline images). Most additional sequences are not implemented.

### 38.10 DCS Passthrough

| Search | Result |
|--------|--------|
| `tmux passthrough` / `DCS tmux` | **Not found** |
| Generic DCS dispatch | VTE parser handles DCS sequences and dispatches to `Term` -- no generic passthrough mechanism exists |

**Verdict:** Not implemented.

---

## Summary of Implementation Status

| Subsection | Plan Status | Actual Status |
|------------|------------|---------------|
| 38.1 DA1/DA2/DA3 | not-started | **~80% done** (DA1, DA2 implemented; DA3 missing) |
| 38.2 DSR/DECXCPR | not-started | **~60% done** (DSR, CPR, CSI 14t, CSI 18t done; DECXCPR, CSI 16t missing) |
| 38.3 DECRQM | not-started | **~90% done** (26+ private modes + 2 ANSI modes; Kitty keyboard mode query done) |
| 38.4 XTGETTCAP | not-started | **Not started** (needs VTE parser support) |
| 38.5 DECRQSS | not-started | **Not started** (needs VTE parser support) |
| 38.6 OSC 4/10/11/12 | not-started | **~95% done** (set, query, reset all implemented) |
| 38.7 Extended Underlines | not-started | **~90% done** (all underline styles + colors; overline missing) |
| 38.8 CSI t Window Manip | not-started | **~25% done** (CSI 14t + CSI 18t only) |
| 38.9 Additional Sequences | not-started | **~15% done** (iTerm2 images only) |
| 38.10 DCS Passthrough | not-started | **Not started** |

---

## Gap Analysis

### Plan Accuracy

The plan marks everything as "not-started" but significant portions are already implemented. This is the most inaccurate status of the five sections audited.

### Issues Found

1. **DA1 response format**: Plan says `\x1b[?62;4c` (VT220 conformance level). Implementation sends `\x1b[?6;4c`. These are different -- `62` in xterm means "VT220 conformance level 2", while `6` means "ANSI color attribute." Need to decide which is correct. Most modern terminals use `62` or `65` (VT500 level).

2. **CPR with ORIGIN mode**: The current `status_device_status(6)` does not check origin mode. The plan correctly requires origin-relative reporting when DECOM is set. This is a bug in the current implementation.

3. **XTGETTCAP and DECRQSS need VTE parser changes**: The vendored VTE crate (`crates/vte/`) does not dispatch XTGETTCAP (`DCS + q ...`) or DECRQSS (`DCS $ q ...`). Implementing these requires modifying the VTE DCS dispatch code. This is significant work because it touches the parser layer, not just the handler.

4. **Overline**: The VTE `Attr` enum doesn't include `Overline` / `CancelOverline`. Adding overline requires changes in: (a) VTE crate `Attr` enum, (b) VTE SGR dispatch, (c) `CellFlags`, (d) SGR apply, (e) GPU rendering, (f) HTML export. This is a cross-cutting change.

5. **CSI 16 t (cell size in pixels)**: The plan lists this under 38.2 (DSR) but it's a CSI t sequence. It's in the same family as CSI 14 t (implemented) and CSI 18 t (implemented). Adding it is straightforward -- read `cell_pixel_width`/`cell_pixel_height` and format the response.

---

## Recommendation

1. **Update status**: Mark 38.1, 38.2, 38.3, 38.6, 38.7 as in-progress or partially complete. They are NOT not-started.
2. **Fix DA1 response**: Decide between `?6;4c` (current) and `?62;4c` (plan). Check what Alacritty, Ghostty, and WezTerm send.
3. **Fix CPR origin mode bug**: `status_device_status(6)` should check `TermMode::ORIGIN` and report relative to scroll region when set.
4. **Estimate VTE parser work**: XTGETTCAP and DECRQSS require modifying the vendored VTE crate's DCS dispatch. This is the main blocker for 38.4 and 38.5.
5. **Add overline to VTE**: Requires a cross-cutting change spanning VTE, oriterm_core, and GPU rendering.
6. **Split the section**: Given the mixed completion status, consider splitting into "Protocol Extensions (complete remaining)" and "Protocol Extensions (new features)" to avoid re-implementing things that already work.
