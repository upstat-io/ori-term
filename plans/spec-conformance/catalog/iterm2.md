---
schema_version: "0.1-provisional"
stack: iterm2
title: "iTerm2 Proprietary Sequences Catalog"
owner_section: "01 (bootstrap), 14 (verification)"
---

# iTerm2 Proprietary Sequences Catalog

iTerm2 introduced several proprietary sequences via OSC 1337 and other numeric OSCs. Section 14 (iTerm2 Inline Images) drives the image-transmit chain to `verified`.

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| ITERM2-1337-FILE | iTerm2 proprietary (File=) | `` `OSC 1337 ; File=<key=value,...>:<base64> BEL\|ST` `` | Inline image transmit (File= form) | `` `osc::dispatch` (`crates/vte/src/ansi/dispatch/osc.rs`) — `b"1337"` arm (`File=` prefix) → `Term::iterm2_file` → `Term::handle_iterm2_file` (`oriterm_core/src/term/handler/image/iterm2.rs`) `` | texture-render | parser:pending dispatch:pending snapshot:pending frame-input:pending texture:pending | implemented-unverified | — | Only the `File=` prefix is recognized; other OSC 1337 sub-ops fall through to `unhandled`. |
| ITERM2-1337-REMOTEHOST | iTerm2 proprietary (RemoteHost=) | `` `OSC 1337 ; RemoteHost=<user@host> BEL\|ST` `` | Report current remote host (shell integration) | MISSING — to be added by Section 14 (iTerm2 Inline Images) | state-snapshot | parser:pending dispatch:pending state:pending | missing | — | No dispatch arm. |
| ITERM2-1337-CURRENTDIR | iTerm2 proprietary (CurrentDir=) | `` `OSC 1337 ; CurrentDir=<path> BEL\|ST` `` | Report current working directory (iTerm2 flavor of OSC 7) | MISSING — to be added by Section 14 (iTerm2 Inline Images) | state-snapshot | parser:pending dispatch:pending state:pending | missing | — | |
| ITERM2-1337-COPY | iTerm2 proprietary (Copy=) | `` `OSC 1337 ; Copy=<b64> BEL\|ST` `` | Copy data to clipboard (iTerm2 flavor of OSC 52) | MISSING — to be added by Section 14 (iTerm2 Inline Images) | effect-clipboard | parser:pending dispatch:pending effect:pending | missing | — | |
| ITERM2-1337-SETMARK | iTerm2 proprietary (SetMark) | `` `OSC 1337 ; SetMark BEL\|ST` `` | Set a mark at the current cursor position (for Cmd+Shift+Up/Down navigation) | MISSING — to be added by Section 14 (iTerm2 Inline Images) | state-snapshot | parser:pending dispatch:pending state:pending | missing | — | No dispatch arm. Used by shell integration for command navigation. |
| ITERM2-1337-REPORTCELLSIZE | iTerm2 proprietary (ReportCellSize) | `` `OSC 1337 ; ReportCellSize BEL\|ST` `` | Report cell dimensions in pixels — response: `OSC 1337 ; ReportCellSize=H;W ST` | MISSING — to be added by Section 14 (iTerm2 Inline Images) | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | — | No dispatch arm. Used by image-display tools (imgcat, viu) to size images to cell boundaries. |
| ITERM2-1337-SETUSERVAR | iTerm2 proprietary (SetUserVar=) | `` `OSC 1337 ; SetUserVar=<name>=<b64> BEL\|ST` `` | Set a named user variable (shell integration context) | MISSING — to be added by Section 14 (iTerm2 Inline Images) | state-snapshot | parser:pending dispatch:pending state:pending | missing | — | No dispatch arm. Variables are accessible by iTerm2 scripting API. |
