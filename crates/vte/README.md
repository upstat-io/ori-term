vte (vendored by ori_term)
==========================

[![Build Status](https://travis-ci.org/alacritty/vte.svg?branch=master)](https://travis-ci.org/alacritty/vte)
[![Crates.io Version](https://img.shields.io/crates/v/vte.svg)](https://crates.io/crates/vte/)

Parser for implementing virtual terminal emulators in Rust.

The parser is implemented according to [Paul Williams' ANSI parser state
machine]. The state machine doesn't assign meaning to the parsed data and is
thus not itself sufficient for writing a terminal emulator. Instead, it is
expected that an implementation of the `Perform` trait which does something
useful with the parsed data. The `Parser` handles the book keeping, and the
`Perform` gets to simply handle actions.

See the [docs] for more info.

[Paul Williams' ANSI parser state machine]: https://vt100.net/emu/dec_ansi_parser
[docs]: https://docs.rs/crate/vte/

## Vendored patches (ori_term)

This crate is a vendored fork of upstream `vte`, patched for oriterm-specific
protocol coverage. Patches are marked in source with a `// VENDORED PATCH
(oriterm): ...` breadcrumb that names the owning roadmap section.

### Section 10 — OSC Suite (2026-04)

Added `Handler` trait methods and dispatcher arms for OSC sub-ops the upstream
`Handler` trait does not expose:

- **OSC 1337 non-image sub-ops** (Section 10.0): refactored `b"1337"` arm into
  `dispatch_iterm2_osc1337` sub-dispatcher that forwards `File=` to the
  existing `Handler::iterm2_file` and routes the non-image sub-ops to
  dedicated trait methods — `iterm2_set_mark`, `iterm2_remote_host`,
  `iterm2_current_dir`, `iterm2_copy`, `iterm2_report_cell_size`,
  `iterm2_set_user_var`, `iterm2_shell_integration_version`.
- **OSC 3 / 5 / 6 / 13 / 14 / 17 / 19 / 113 / 114 / 117 / 119 / L / l**
  (Section 10.9): new dispatcher arms + Handler methods — `set_x11_property`,
  `set_special_color`, `query_special_color`, `set_tab_title_color`,
  `set_mouse_fg_color`, `set_mouse_bg_color`, `set_highlight_bg_color`,
  `set_highlight_fg_color`, `query_mouse_fg_color`, `query_mouse_bg_color`,
  `query_highlight_bg_color`, `query_highlight_fg_color`,
  `reset_mouse_fg_color`, `reset_mouse_bg_color`, `reset_highlight_bg_color`,
  `reset_highlight_fg_color`. OSC L and OSC l reuse `set_icon_name` /
  `set_title` via the shared `join_title_payload` helper.

### Section 12 — Sixel DCS Abort Plumbing (2026-04)

Added a two-byte-lookahead state plus a `Perform`-level abort callback plus a
dispatch-level abort flag so the DCS consumer can distinguish a normal `ST`
(`ESC \` or `0x9C`) finish from a CAN / SUB / ESC-to-new-sequence abort per
DEC STD 070 §6.4. Without this, `sixel_end()` fires the same way in both
cases and the handler cannot tell whether to commit or discard the in-flight
image — a real bug surfaced by the §12.1 conformance tests (`dcs_abort_*`
scenarios in `oriterm_core/tests/spec_chain/sixel/state_machine.rs`).

- **Parser state machine** (`src/lib.rs`): new `State::DcsEscape` variant +
  `advance_dcs_escape()` handler. In `advance_dcs_passthrough`, ESC
  (`0x1B`) mid-DCS no longer calls `unhook()` immediately — it transitions
  to `DcsEscape` so the NEXT byte decides: `0x5C` (`\`) completes the
  2-byte ST (normal `unhook`), anything else calls `notify_dcs_abort` +
  `unhook` + re-dispatches through `advance_esc` so the new sequence
  begins cleanly. CAN (`0x18`) and SUB (`0x1A`) unchanged except they now
  call `notify_dcs_abort()` before `unhook()`.
- **`Perform` trait** (`src/lib.rs`): new `notify_dcs_abort()` default-empty
  callback. Implementors that don't care about abort distinction ignore
  it; the dispatch-layer `Performer` uses it to flip
  `ProcessorState::dcs_aborted = true`.
- **`ProcessorState`** (`src/ansi/processor.rs`): new `dcs_aborted: bool`
  field. Set by `Perform::notify_dcs_abort`, read by `dispatch_unhook`,
  reset to `false` after every unhook so the next DCS starts clean.
- **`Handler` trait** (`src/ansi/handler/core_methods.rs`): `sixel_end()`
  signature changed to `sixel_end(&mut self, aborted: bool)`. The
  dispatch layer reads `state.dcs_aborted` and passes it in. DECRQSS /
  DECRSPS were left unchanged — they are query/response sequences and
  their current handler stubs log + ignore, so abort semantics for
  those are functionally equivalent today; when a real DECRSPS restore
  lands it must check `aborted` before applying.
- **`ObservedPerformer`** (`src/ansi/dispatch/observed.rs`): mirrors the
  `notify_dcs_abort` impl on `Performer` so the observation-enabled
  dispatch path sees the same abort signal.

### Upstreaming

DCS abort plumbing is a general terminal-emulator concern (not
oriterm-specific) — the `Perform::notify_dcs_abort` + `DcsEscape` state
pattern is a candidate for upstreaming if the maintainers want to pin the
DEC STD 070 §6.4 abort contract. In the interim this is a vendored patch.

### Section 09A — DEC Private CSI Extensions (2026-04)

Added `Handler` trait methods and dispatch arms for the DEC private
rectangular-area and presentation ops that the upstream `Handler` trait does
not expose. Also added the DCS-path DECRQSS / DECRSPS reply stubs.

- **DCS-path queries**: `decrqss` Pt branches extended for `q` (DECSCUSR) and
  `"q` (DECSCA); new `decrsps(ps, pt)` default wired through
  `dispatch/mod.rs::dispatch_hook`/`dispatch_unhook` with a Ps-preserving
  `DcsState::Decrsps { ps }`.
- **ESC-path index ops**: `decbi` (ESC 6) and `decfi` (ESC 9) dispatcher arms
  + handler defaults.
- **CSI-path column ops**: `decic` (CSI Ps ' }) and `decdc` (CSI Ps ' ~).
- **CSI-path rectangular-area ops**: `decsace`, `deccara`, `decrara`,
  `deccra`, `decfra`, `xtchecksum`, `decrqcra`, `decera`, `decsera`,
  `xtreportsgr`.
- **CSI-path presentation ops**: `decrqpsr`, `decrqupss`, `decrqde`, `decscl`,
  `decsca`, `decsasd`, `decssdt`.
- **`Handler` trait split**: `ansi/handler.rs` converted to directory module
  `ansi/handler/` (`mod.rs` + `core_methods.rs` + `vendored_osc_methods.rs` +
  `dec_private_methods.rs`). The trait body is assembled by three
  `macro_rules!` items-level macros so each source file stays under the
  500-line hygiene cap; consumers still implement exactly one `Handler`
  trait (no new super-traits, no API change).

### Upstreaming

These patches are oriterm-specific protocol coverage and are NOT upstreamable
— the upstream maintainers intentionally keep the `Handler` trait minimal and
the non-image OSC 1337 sub-ops are not in scope for upstream. Expect rebase
work on every upstream `vte` sync; the `VENDORED PATCH (oriterm)` breadcrumbs
identify every oriterm-owned divergence.
