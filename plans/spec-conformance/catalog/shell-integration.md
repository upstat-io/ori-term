---
schema_version: "0.1-provisional"
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
| SHINT-OSC-9-NOTIFY | iTerm2 / Growl | `` `OSC 9 ; text BEL\|ST` `` | Desktop notification — cross-reference `osc.md::OSC-9` | See `osc.md::OSC-9` | effect-host-notification | — | missing | — | Cross-reference row. |
| SHINT-OSC-777-NOTIFY | urxvt | `` `OSC 777 ; notify ; title ; body BEL\|ST` `` | urxvt notification dispatch — cross-reference `osc.md::OSC-777` | See `osc.md::OSC-777` | effect-host-notification | — | missing | — | Cross-reference row. |
| SHINT-OSC-1337-SETMARK | iTerm2 proprietary | `` `OSC 1337 ; SetMark BEL\|ST` `` | Mark for command navigation — cross-reference `iterm2.md::ITERM2-1337-SETMARK` | See `iterm2.md::ITERM2-1337-SETMARK` | state-snapshot | — | missing | — | Cross-reference row. Used by shell integration to mark command boundaries. |
