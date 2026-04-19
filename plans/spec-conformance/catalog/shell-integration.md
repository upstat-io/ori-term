---
schema_version: "1.0"
stack: shint
title: "Shell Integration Catalog"
owner_section: "01 (bootstrap), 10 (verification)"
---

# Shell Integration Catalog

Shell-integration OSC sequences (working directory, semantic prompt, command completion). Section 10 (OSC Suite) drives rows to `verified` — the handlers land alongside the `osc.md` additions.

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| SHINT-OSC-7-CWD | iTerm2 / VTE de-facto | `` `OSC 7 ; file://host/path BEL\|ST` `` | Current working directory — cross-reference `osc.md::OSC-7` | See `osc.md::OSC-7` | state-snapshot | — | stub | wezterm escape-sequences.md | Cross-reference row. Currently `stub` because `Term` does not override `Handler::set_working_directory`. |
| SHINT-OSC-133-PROMPT | Final Term (semantic prompt) | `` `OSC 133 ; A\|B\|C\|D ; params BEL\|ST` `` | Semantic prompt markers — cross-reference `osc.md::OSC-133` | See `osc.md::OSC-133` | state-snapshot | — | missing | wezterm escape-sequences.md | Section 10 lands the dispatcher + handler. |
| SHINT-OSC-633-VSCODE | VS Code | `` `OSC 633 ; Pt BEL\|ST` `` | VS Code shell integration — cross-reference `osc.md::OSC-633` | See `osc.md::OSC-633` | state-snapshot | — | missing | — | |
| SHINT-OSC-1337-REMOTEHOST | iTerm2 proprietary | `` `OSC 1337 ; RemoteHost=<...> BEL\|ST` `` | Remote host reporting — cross-reference `iterm2.md::ITERM2-1337-REMOTEHOST` | See `iterm2.md::ITERM2-1337-REMOTEHOST` | state-snapshot | — | missing | — | |
| SHINT-OSC-1337-CURRENTDIR | iTerm2 proprietary | `` `OSC 1337 ; CurrentDir=<path> BEL\|ST` `` | iTerm2 flavor of CWD reporting — cross-reference `iterm2.md::ITERM2-1337-CURRENTDIR` | See `iterm2.md::ITERM2-1337-CURRENTDIR` | state-snapshot | — | missing | — | |
| SHINT-OSC-9-NOTIFY | iTerm2 / Growl | `` `OSC 9 ; text BEL\|ST` `` | Desktop notification — cross-reference `osc.md::OSC-9` | See `osc.md::OSC-9` | effect-host-notification | parser:pass dispatch:pass effect:pass — `oriterm_mux/src/shell_integration/tests.rs::{osc9_simple_body_fires_notification, osc9_empty_body, osc9_and_osc99_use_different_sources, osc9_via_processor_without_mux_drops}` | verified | — | Cross-reference row. |
| SHINT-OSC-99-NOTIFY | kitty notification protocol | `` `OSC 99 ;; payload BEL\|ST` `` (no metadata) or `` `OSC 99 ; metadata ; payload BEL\|ST` `` | Kitty desktop notification — cross-reference `osc.md::OSC-99` | See `osc.md::OSC-99` | effect-host-notification | parser:pass dispatch:pass effect:pass — `oriterm_mux/src/shell_integration/tests.rs::{osc99_default_payload_routes_to_title, osc99_metadata_form_default_p_routes_payload_to_title, osc99_p_body_routes_payload_to_body, osc99_empty_payload_drops_notification, osc99_unsupported_payload_kind_drops_notification, osc9_and_osc99_use_different_sources, interceptor_osc99_kitty_notification}` | verified-with-deviation | kitty terminal docs | Cross-reference row. Honours `p=title` (default) / `p=body` for payload routing; drops `p=close|icon|?|alive|buttons` and empty notifications per spec; metadata keys other than `p` are recognised as opaque and discarded (full deviation in `osc.md::OSC-99`). |
| SHINT-OSC-777-NOTIFY | urxvt | `` `OSC 777 ; notify ; title ; body BEL\|ST` `` | urxvt notification dispatch — cross-reference `osc.md::OSC-777` | See `osc.md::OSC-777` | effect-host-notification | parser:pass dispatch:pass effect:pass — `oriterm_mux/src/shell_integration/tests.rs::{osc777_notify_title_body, osc777_non_notify_action_dropped, osc777_missing_title}` | verified | — | Cross-reference row. |
| SHINT-OSC-1337-SETMARK | iTerm2 proprietary | `` `OSC 1337 ; SetMark BEL\|ST` `` | Mark for command navigation — cross-reference `iterm2.md::ITERM2-1337-SETMARK` | See `iterm2.md::ITERM2-1337-SETMARK` | state-snapshot | — | missing | — | Cross-reference row. Used by shell integration to mark command boundaries. |
