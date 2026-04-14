---
schema_version: "1.0"
stack: mouse
title: "Mouse Protocol Catalog"
owner_section: "01 (bootstrap), 16 (verification)"
---

# Mouse Protocol Catalog

DEC private mode enables (rows live in `dec-private-modes.md`) combined with the encoding-side sequences ori_term emits to the PTY when the user moves / clicks / scrolls the mouse. Section 16 (Mouse Protocols) drives both sides to `verified`.

Enable-side rows are cross-references; the encoding-side implementation lives in `oriterm/src/pane_input/` and `oriterm_ui`, outside `oriterm_core`.

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| MOUSE-X10 | xterm ctlseqs | `` `CSI ? 9 h/l` `` enable + `` `CSI M CbCxCy` `` encoding | X10 press-only mouse reporting | See `dec-private-modes.md::DEC-X10-MOUSE` (enable); encoding MISSING — Section 16 | effect-pty-write | parser:pending dispatch:pending effect:pending | stub | — | Enable arm tracked; ori_term does not currently emit X10-format mouse events. |
| MOUSE-VT200 | xterm ctlseqs (1000) | `` `CSI ? 1000 h/l` `` enable + `` `CSI M CbCxCy` `` encoding | VT200 mouse reporting (press + release) | See `dec-private-modes.md::DEC-MOUSE-CLICKS` | effect-pty-write | parser:pending dispatch:pending effect:pending | stub | — | |
| MOUSE-BTN-EVENT | xterm ctlseqs (1002) | `` `CSI ? 1002 h/l` `` enable | Cell-motion reporting while a button is held | See `dec-private-modes.md::DEC-MOUSE-DRAG` | effect-pty-write | parser:pending dispatch:pending effect:pending | stub | — | |
| MOUSE-ANY-EVENT | xterm ctlseqs (1003) | `` `CSI ? 1003 h/l` `` enable | Cell-motion reporting at all times | See `dec-private-modes.md::DEC-MOUSE-MOTION` | effect-pty-write | parser:pending dispatch:pending effect:pending | stub | — | |
| MOUSE-FOCUS | xterm ctlseqs (1004) | `` `CSI ? 1004 h/l` `` enable + `` `CSI I` `` / `` `CSI O` `` emit | Focus-in / focus-out events | See `dec-private-modes.md::DEC-FOCUS-IN-OUT` | effect-pty-write | parser:pending dispatch:pending effect:pending | stub | — | |
| MOUSE-UTF8 | xterm ctlseqs (1005) | `` `CSI ? 1005 h/l` `` enable + UTF-8 encoded coordinate bytes | UTF-8 mouse coordinate encoding | See `dec-private-modes.md::DEC-UTF8-MOUSE` | effect-pty-write | parser:pending dispatch:pending effect:pending | stub | — | |
| MOUSE-SGR | xterm ctlseqs (1006) | `` `CSI ? 1006 h/l` `` enable + `` `CSI < Cb ; Cx ; Cy M\|m` `` encoding | SGR mouse encoding | See `dec-private-modes.md::DEC-SGR-MOUSE` | effect-pty-write | parser:pending dispatch:pending effect:pending | stub | — | |
| MOUSE-URXVT | urxvt | `` `CSI ? 1015 h/l` `` enable + `` `CSI Cb ; Cx ; Cy M` `` encoding | URXVT mouse encoding | See `dec-private-modes.md::DEC-URXVT-MOUSE` | effect-pty-write | parser:pending dispatch:pending effect:pending | stub | — | |
| MOUSE-SGR-PIXEL | xterm ctlseqs (1016) | `` `CSI ? 1016 h/l` `` enable + `` `CSI < Cb ; Px ; Py M\|m` `` encoding | SGR-Pixel mouse — pixel coordinates (not cell) in SGR format | See `dec-private-modes.md::DEC-SGR-PIXEL-MOUSE` (enable); encoding MISSING — Section 16 | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | — | Modern extension: reports pixel-level mouse coordinates. Supported by kitty, wezterm, foot. Enable mode not yet in `NamedPrivateMode`. |
