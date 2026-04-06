---
reroute: true
name: "Teseq Conformance"
full_name: "Teseq Conformance: Human-Readable Escape Sequence Test Framework"
status: active
order: 1
---

# Teseq Conformance Index

> **Maintenance Notice:** Update this index when adding/modifying sections.
> **Complements:** `plans/completed/vttest-conformance/` (VT protocol), handler unit tests (per-sequence)

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: TeseqHarness & Infrastructure
**File:** `section-01-infrastructure.md` | **Status:** Complete

```
TeseqHarness, ScenarioSpec, RecordedEvent, RecordedListener
reseq, reseq_available, subprocess, Command, Stdio
sidecar, TOML, toml, serde, Deserialize
scenario, loader, runner, assertions, harness
grid_text, grid_chars, cursor_position, events, pty_writes
insta, assert_snapshot, snapshots
feed, vte::ansi::Processor, Term, EventListener
oriterm_core/tests/teseq, main.rs, harness/
pre_feed, setup, SetupConfig, TerminalConfig, ExpectConfig
```

---

### Section 02: Basic Scenario Suite
**File:** `section-02-basic-scenarios.md` | **Status:** Complete

```
c0, control, CR, LF, BS, TAB, BEL, FF, VT, SO, SI
carriage_return, linefeed, backspace, tab, bell
CSI, cursor, CUP, CUU, CUD, CUF, CUB, VPA, HPA, CHA
erase, ED, EL, erase_display, erase_line
insert, delete, ICH, DCH, IL, DL
scenarios/c0/, scenarios/esc/, scenarios/csi/
basic_cursor, basic_erase, basic_insert_delete
```

---

### Section 03: Reports & Response Validation
**File:** `section-03-reports.md` | **Status:** Complete

```
DA, DA1, DA2, DA3, device_attributes, primary, secondary, tertiary
DSR, device_status_report, cursor_position_report
DECRQM, mode_report, DECRPM
PtyWrite, Event::PtyWrite, outbound, response
assert_pty_writes, assert_response_snapshot, analyze_response
teseq, analysis, response_analysis, debug_aid
scenarios/csi/reports/, da_handshake
RecordedEvent, pty_writes, take_responses
```

---

### Section 04: Mode Interaction Scenarios
**File:** `section-04-mode-interactions.md` | **Status:** Complete

```
DECOM, origin_mode, DECSTBM, scroll_region
DECCOLM, column_mode, 132, 80, deccolm_default_cols
DECAWM, auto_wrap, wrap, line_wrap, wide_char_margin
DECTCEM, cursor_visibility, show_cursor
IRM, insert_mode, insert_replace, insert_blank, wide_char
alt_screen, 1049, 1047, alternate, screen_swap, mode_leakage, reentry, swap_alt_clear
mode_combination, interaction, multi_mode, cross_cutting
TermMode, mode_flags, assert_mode_contains, assert_mode_not_contains
scrollback_integrity, scrollback_len, sub_region_scroll
negative_control, no_mode40, deccolm_no_resize
scenarios/csi/modes/, scenarios/workflows/
```

---

### Section 05: SGR & Color Scenarios
**File:** `section-05-sgr-colors.md` | **Status:** Complete

```
SGR, select_graphic_rendition, attributes
bold, dim, italic, underline, blink, inverse, hidden, strikethrough
ANSI, 16_color, 256_color, TrueColor, RGB
bold_as_bright, color_promotion, set_bold_is_bright
blink_fast, BlinkFast, BlinkSlow, SGR_6
underline_style, curly, dotted, dashed, double_underline, ALL_UNDERLINES, cancel_subparam, SGR_4_0
underline_color, SGR_58, SGR_59, CellExtra
dim_priority, dim_bold, DIM_wins, dim_rgb
selective_reset, SGR_21, SGR_22, SGR_23, SGR_24, SGR_25, SGR_27, SGR_28, SGR_29, SGR_39, SGR_49, SGR_59
DECSCNM, reverse_video, REVERSE_VIDEO, palette_swap, double_swap
inverse, apply_inverse, color_swap
resolve_fg, resolve_bg, color_resolution
reset, default, color_reset, empty_sgr, parameterless
contains, intersects, flags_assertion
cell_fg_at, cell_bg_at, cell_underline_color_at
assert_cell_flags_contain, assert_cell_flags_not_contain
scenarios/csi/sgr/
CellFlags, fg, bg, Rgb, RenderableCell
```

---

### Section 06: Complex Workflow Scenarios
**File:** `section-06-workflows.md` | **Status:** Complete

```
workflow, multi_sequence, interaction, combination
scroll_origin, scroll_region_with_origin_mode
alt_screen_roundtrip, 1049_enter_exit
deccolm_transition, 132_to_80
DECSC, DECRC, save_cursor_attrs, restore_cursor_attrs, SGR_save, charset_save, origin_flag_save
da_handshake, query_response
shell_startup, editor, multi_step
charset_switching, G0, G1, SCS, SO, SI, locking_shift, DEC_Special_Graphics, box_drawing
OSC, osc_title, osc_icon_name, osc_clipboard, osc_color_query
title, icon_name, clipboard, OSC_52, OSC_0, OSC_1, OSC_2, OSC_4, OSC_10, OSC_11
edge_case, malformed, partial, interleaved, erase_with_attrs, rapid_mode_toggle
zero_params, large_params, boundary, chunked_feed, split_sequence, adversarial
scenarios/workflows/, scenarios/osc/
```

---

### Section 07: Verification & CI Integration
**File:** `section-07-verification.md` | **Status:** Not Started

```
verification, test_matrix, coverage, gap_analysis
CI, continuous_integration, test-all.sh
reseq_available, platform, skip, graceful
cross_platform, Windows, macOS, Linux
documentation, CLAUDE.md, memory
scenario_count, pass_rate, coverage_report
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | TeseqHarness & Infrastructure | `section-01-infrastructure.md` |
| 02 | Basic Scenario Suite | `section-02-basic-scenarios.md` |
| 03 | Reports & Response Validation | `section-03-reports.md` |
| 04 | Mode Interaction Scenarios | `section-04-mode-interactions.md` |
| 05 | SGR & Color Scenarios | `section-05-sgr-colors.md` |
| 06 | Complex Workflow Scenarios | `section-06-workflows.md` |
| 07 | Verification & CI Integration | `section-07-verification.md` |
