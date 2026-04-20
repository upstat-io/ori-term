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

### Upstreaming

These patches are oriterm-specific protocol coverage and are NOT upstreamable
— the upstream maintainers intentionally keep the `Handler` trait minimal and
the non-image OSC 1337 sub-ops are not in scope for upstream. Expect rebase
work on every upstream `vte` sync; the `VENDORED PATCH (oriterm)` breadcrumbs
identify every oriterm-owned divergence.
