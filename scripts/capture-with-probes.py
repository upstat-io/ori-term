#!/usr/bin/env python3
"""Probe-aware PTY capture driver for pixel-graphics-aware TUIs.

Sibling of `replay-capture-script.py`. Spawns a TUI under a PTY and
captures its byte stream, but also responds to common terminal-
capability probes — DA1/DA2/DA3, XTVERSION, XTSMGRAPHICS, kitty
graphics query, DECRQM for synchronized output, XTGETTCAP — with
affirmative replies advertising kitty graphics + sixel + truecolor.
Without these replies, kitty/sixel-aware apps (notcurses-demo,
chafa, sixel image viewers) silently fall back to ASCII/Unicode
blitters and the captured byte stream omits the pixel transport
payload that the consumer's regression pin depends on.

Output layout (next to the script, with `.script` stripped):

  <stem>.cap            byte stream
  <stem>.timing         per-chunk timing log (offset_sec  byte_count)

The `.timing` file matches `script(1)`'s `-t` format: lines of
`<offset_seconds_float> <chunk_size_bytes>`. Recovery of "what state
was the terminal in at 5s/10s/15s into the run" is done by summing
`byte_count` up to the row where `offset_seconds_float` crosses each
mark.

Linux-only — uses Python's stdlib `pty`. Wrapper-scope tooling per
`term_repo/CLAUDE.md §Crate Coding Guidelines (Exempt)`.

Usage:

    scripts/capture-with-probes.py SCRIPT_FILE [-o OUTPUT_CAP] \
        [--cols 80] [--rows 24] [--term xterm-256color]

Exit codes:
    0  capture written cleanly
    1  script parse error / runner failure
    2  child exited non-zero AND .cap is empty
"""

from __future__ import annotations

import argparse
import fcntl
import os
import pty
import re
import select
import shlex
import struct
import sys
import termios
import time
from dataclasses import dataclass
from pathlib import Path

# Default geometry — overridable via --cols/--rows. xray wants more
# than 80x24; the dual-thread NCBLIT_PIXEL video is sized to fit
# `notcurses_term_dim_yx()` minus a 1-row slider band.
DEFAULT_COLS = 100
DEFAULT_LINES = 30
DEFAULT_TERM = "xterm-256color"


# -----------------------------------------------------------------
# Probe → reply table
# -----------------------------------------------------------------
# Each entry: (regex matching one probe in the byte stream, reply
# bytes). Replies are written back to the PTY master, which appears
# to the child as keyboard / pseudo-terminal input.
#
# Affirmative posture: kitty graphics + sixel + truecolor + RGB +
# synchronized output supported. The regex set covers the probes
# notcurses-demo emits on startup (verified by inspecting the naive
# capture's first 200 bytes — DA1/CSR/DA3/XTVERSION/XTGETTCAP/DA2/
# query-color-N).

# DA1 (primary device attributes) — `\e[c` or `\e[0c`. Reply:
# VT420 + sixel + kitty graphics (62 = VT220, 4 = sixel, 22 =
# ANSI color, 28 = rectangular area ops).
DA1_REPLY = b"\x1b[?62;4;22;28c"

# DA2 (secondary device attributes) — `\e[>c` or `\e[>0c`. Reply:
# xterm 333.
DA2_REPLY = b"\x1b[>0;333;0c"

# DA3 (tertiary device attributes) — `\e[=c` or `\e[=0c`. Reply:
# DECRPTUI with a dummy unit id.
DA3_REPLY = b"\x1bP!|00000000\x1b\\"

# Cursor position report — `\e[6n`. Reply with row 1 col 1 (we
# do not maintain real cursor state).
CSR_REPLY = b"\x1b[1;1R"

# XTVERSION — `\e[>0q` or `\e[>q`. Reply with a kitty(0.32.2)
# identification so notcurses' apply_kitty_heuristics activates
# NCPIXEL_KITTY_ANIMATED (requires termversion >= 0.20.0 per
# notcurses/src/lib/termdesc.c:747).
XTVERSION_REPLY = b"\x1bP>|kitty(0.32.2)\x1b\\"

# XTWINOPS cell/window size queries. notcurses needs cellpxy/cellpxx
# non-zero to consider pixel graphics supported (per
# notcurses_check_pixel_support at notcurses.c:1156). Reply with
# Cascadia-Mono-12pt-typical 10x22 cells, 100-cols x 30-rows grid
# matching the captured PTY geometry. Format follows xterm CSI t.
def xtwinops_reply(probe: bytes, cell_h: int, cell_w: int,
                   rows: int, cols: int) -> bytes:
    m = re.match(rb"\x1b\[(\d+)t", probe)
    if not m:
        return b""
    op = int(m.group(1))
    if op == 14:
        return f"\x1b[4;{rows * cell_h};{cols * cell_w}t".encode("ascii")
    if op == 16:
        return f"\x1b[6;{cell_h};{cell_w}t".encode("ascii")
    if op == 18:
        return f"\x1b[8;{rows};{cols}t".encode("ascii")
    if op == 19:
        return f"\x1b[9;{rows};{cols}t".encode("ascii")
    return b""

# Kitty keyboard protocol query — `\e[?u`. Reply: "we support
# flag 1" (disambiguate-escape).
KKBD_QUERY_REPLY = b"\x1b[?1u"

# Kitty graphics support probe — `\e_Gi=...,a=q,...\e\`. Reply OK
# for the queried id so notcurses considers kitty graphics
# supported. We extract `i=<id>` from the probe and echo it back.
def kitty_query_reply(probe: bytes) -> bytes:
    m = re.search(rb"i=(\d+)", probe)
    image_id = m.group(1) if m else b"1"
    return b"\x1b_Gi=" + image_id + b";OK\x1b\\"


# XTSMGRAPHICS (graphics attribute query) — `\e[?<Pi>;<Pa>;<Pv>S`
# with Pa=1 (read). Pi: 1=color regs, 2=sixel geom, 3=ReGIS geom.
# Reply: Ps=Pi, Ps=0 (success), Pv=<value>. Generous values:
# 1024 color regs, 1000x1000 sixel.
def xtsmgraphics_reply(probe: bytes) -> bytes:
    m = re.match(rb"\x1b\[\?(\d+);(\d+)(?:;(\d+))?S", probe)
    if not m:
        return b""
    pi = m.group(1)
    pa = m.group(2)
    if pa != b"1":
        return b""
    if pi == b"1":
        return b"\x1b[?1;0;1024S"  # 1024 color registers
    if pi == b"2":
        return b"\x1b[?2;0;1000;1000S"  # 1000x1000 sixel
    if pi == b"3":
        return b"\x1b[?3;0;1000;1000S"  # 1000x1000 regis
    return b"\x1b[?" + pi + b";3S"  # unsupported subitem


# DECRQM (request mode) — `\e[?<Ps>$p`. Reply DECRPM `\e[?<Ps>;<v>$y`
# where v=1 (set), v=2 (reset). Posture: report common modes as
# set so the TUI proceeds.
def decrqm_reply(probe: bytes) -> bytes:
    m = re.match(rb"\x1b\[\?(\d+)\$p", probe)
    if not m:
        return b""
    ps = m.group(1)
    # Mode 2026 = synchronized output; 2027 = grapheme cluster;
    # 1004 = focus tracking; etc. Report "permanently set" (v=3).
    return b"\x1b[?" + ps + b";3$y"


# XTGETTCAP (request terminfo capabilities) — `\eP+q<hex>;<hex>;...\e\`.
# Reply with `\eP1+r<hex_name>=<hex_value>;...\e\` (success) so
# notcurses' TERMINAL detection thinks we're kitty (TN=xterm-kitty)
# with RGB truecolor.
#
# notcurses' tcap_cb() parses `gettcap(s, &val, &key)` where `val`
# is the capability NAME (TN/RGB/hpa) and `key` is the VALUE. The
# format is `hex(name)=hex(value)`. Pre-computed common caps:
#   TN  -> "TN"   = 544e   -> value "xterm-kitty" = 7874657274656d2d6b69747479
#   RGB -> "RGB"  = 524742 -> value present, empty body is fine
#   hpa -> "hpa"  = 687061 -> value "\\033[%i%p1%dG" (xterm-256color hpa)
KITTY_TN_HEX = b"544e=" + b"xterm-kitty".hex().encode("ascii")
RGB_PRESENT  = b"524742="
HPA_TERMINFO = b"\x1b[%i%p1%dG"
HPA_HEX      = b"687061=" + HPA_TERMINFO.hex().encode("ascii")

_HEX_TO_NAME = {
    b"544e": b"TN",
    b"524742": b"RGB",
    b"687061": b"hpa",
}


def xtgettcap_reply(probe: bytes) -> bytes:
    m = re.match(rb"\x1bP\+q([0-9A-Fa-f;]+)\x1b\\", probe)
    if not m:
        return b""
    queried = m.group(1).lower().split(b";")
    parts: list[bytes] = []
    for hex_name in queried:
        name = _HEX_TO_NAME.get(hex_name)
        if name == b"TN":
            parts.append(KITTY_TN_HEX)
        elif name == b"RGB":
            parts.append(RGB_PRESENT)
        elif name == b"hpa":
            parts.append(HPA_HEX)
        else:
            # Unknown cap — reply with empty value (present).
            parts.append(hex_name + b"=")
    return b"\x1bP1+r" + b";".join(parts) + b"\x1b\\"


# Query color N — `\e]4;<n>;?\e\`. Reply with a deterministic
# black/white palette for now (notcurses uses these to compute
# truecolor distance — the exact value just needs to be valid).
def query_color_reply(probe: bytes) -> bytes:
    m = re.match(rb"\x1b\]4;(\d+);\?\x1b\\", probe)
    if not m:
        return b""
    n = m.group(1)
    return b"\x1b]4;" + n + b";rgb:8080/8080/8080\x1b\\"


# Foreground/background color query — `\e]10;?\e\` and `\e]11;?\e\`.
def query_fgbg_reply(probe: bytes) -> bytes:
    m = re.match(rb"\x1b\](1[01]);\?\x1b\\", probe)
    if not m:
        return b""
    n = m.group(1)
    if n == b"10":
        return b"\x1b]10;rgb:d3d3/d7d7/cfcf\x1b\\"
    return b"\x1b]11;rgb:0101/0101/0101\x1b\\"


# Ordered list of (regex, replier). The first match in a scan
# window wins. Replier is either a callable (probe-bytes -> reply)
# or a literal bytes object.
# XTWINOPS is parameterized by geometry — caller binds via lambda
# in run_capture() to apply the runtime cols/rows.
PROBE_TABLE: list[tuple[re.Pattern[bytes], object]] = [
    (re.compile(rb"\x1b\[6n"), CSR_REPLY),
    (re.compile(rb"\x1b\[>0?q"), XTVERSION_REPLY),
    (re.compile(rb"\x1b\[>u"), KKBD_QUERY_REPLY),
    (re.compile(rb"\x1b\[=0?c"), DA3_REPLY),
    (re.compile(rb"\x1b\[>0?c"), DA2_REPLY),
    (re.compile(rb"\x1b\[0?c"), DA1_REPLY),
    (re.compile(rb"\x1b\[\?\d+\$p"), decrqm_reply),
    (re.compile(rb"\x1b\[\?\d+;\d+(?:;\d+)?S"), xtsmgraphics_reply),
    (re.compile(rb"\x1bP\+q[0-9A-Fa-f;]+\x1b\\"), xtgettcap_reply),
    (re.compile(rb"\x1b_Gi=\d+,a=q[^\x1b]*\x1b\\"), kitty_query_reply),
    (re.compile(rb"\x1b\]4;\d+;\?\x1b\\"), query_color_reply),
    (re.compile(rb"\x1b\]1[01];\?\x1b\\"), query_fgbg_reply),
    # XTWINOPS (14/16/18/19) — bound at runtime with geometry.
    # Placeholder; replaced in run_capture() via PROBE_TABLE_GEOM.
]


def build_probe_table(cell_h: int, cell_w: int,
                      rows: int, cols: int) -> list[tuple[re.Pattern[bytes], object]]:
    """Return PROBE_TABLE with XTWINOPS bound to runtime geometry."""
    table = list(PROBE_TABLE)
    table.append((
        re.compile(rb"\x1b\[(?:14|16|18|19)t"),
        lambda probe: xtwinops_reply(probe, cell_h, cell_w, rows, cols),
    ))
    return table


def scan_for_probes(buf: bytes,
                    table: list[tuple[re.Pattern[bytes], object]]) -> list[tuple[int, int, bytes]]:
    """Scan `buf` for known probes; return [(start, end, reply)]
    sorted by start offset. Non-overlapping — first match wins
    in tie cases."""
    hits: list[tuple[int, int, bytes]] = []
    occupied: list[tuple[int, int]] = []
    for pattern, replier in table:
        for m in pattern.finditer(buf):
            start, end = m.start(), m.end()
            if any(s < end and start < e for s, e in occupied):
                continue
            probe_bytes = buf[start:end]
            if callable(replier):
                reply = replier(probe_bytes)
            else:
                reply = replier
            if not reply:
                continue
            hits.append((start, end, reply))
            occupied.append((start, end))
    hits.sort(key=lambda h: h[0])
    return hits


# -----------------------------------------------------------------
# Script parsing — reuses the COMMAND/WAIT/KEY/TEXT grammar from
# replay-capture-script.py
# -----------------------------------------------------------------
KEYS: dict[str, bytes] = {
    "Enter": b"\r",
    "Return": b"\r",
    "Escape": b"\x1b",
    "Esc": b"\x1b",
    "Tab": b"\t",
    "BS": b"\x7f",
    "Backspace": b"\x7f",
    "Space": b" ",
    "Up": b"\x1b[A",
    "Down": b"\x1b[B",
    "Right": b"\x1b[C",
    "Left": b"\x1b[D",
    "Home": b"\x1b[H",
    "End": b"\x1b[F",
}


def keyname_to_bytes(name: str) -> bytes:
    if name.startswith("Ctrl+") and len(name) == 6:
        c = name[5].upper()
        if c.isalpha():
            return bytes([ord(c) - ord("A") + 1])
    if name in KEYS:
        return KEYS[name]
    if len(name) == 1:
        return name.encode("utf-8")
    raise ValueError(f"unknown key name: {name!r}")


@dataclass
class Event:
    kind: str
    payload: str


def parse_script(path: Path) -> tuple[list[str], list[Event]]:
    command_line: str | None = None
    events: list[Event] = []
    lineno = 0
    with path.open("r", encoding="utf-8") as f:
        for raw in f:
            lineno += 1
            line = raw.rstrip("\n")
            if not line.strip():
                events.append(Event("wait", "100"))
                continue
            s = line.strip()
            if s.startswith("#"):
                continue
            if ": " in s:
                head, rest = s.split(": ", 1)
            elif s.endswith(":"):
                head, rest = s[:-1], ""
            else:
                raise ValueError(f"{path}:{lineno}: malformed line {line!r}")
            head = head.strip().upper()
            if head == "COMMAND":
                command_line = rest
            elif head == "WAIT":
                if not rest.endswith("ms"):
                    raise ValueError(f"{path}:{lineno}: WAIT expects <N>ms")
                events.append(Event("wait", rest[:-2]))
            elif head == "KEY":
                events.append(Event("key", rest))
            elif head == "TEXT":
                events.append(Event("text", rest))
            else:
                raise ValueError(f"{path}:{lineno}: unknown directive {head!r}")
    if command_line is None:
        raise ValueError(f"{path}: missing COMMAND")
    return shlex.split(command_line), events


def set_winsize(fd: int, rows: int, cols: int) -> None:
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))


# -----------------------------------------------------------------
# Capture loop
# -----------------------------------------------------------------
@dataclass
class TimingEntry:
    offset_sec: float
    nbytes: int


def run_capture(
    script: Path,
    cap_out: Path,
    timing_out: Path,
    cols: int,
    rows: int,
    term: str,
) -> int:
    argv, events = parse_script(script)

    env = os.environ.copy()
    env["TERM"] = term
    env["LINES"] = str(rows)
    env["COLUMNS"] = str(cols)
    env.setdefault("LC_ALL", "C.UTF-8")
    scratch = Path("/tmp/ori_capture_scratch")
    scratch.mkdir(exist_ok=True)
    env["XDG_STATE_HOME"] = str(scratch / "state")
    env["XDG_CACHE_HOME"] = str(scratch / "cache")
    env["XDG_CONFIG_HOME"] = str(scratch / "config")

    sink = bytearray()
    timing: list[TimingEntry] = []
    pending_probe_scan_from = 0

    # Bind XTWINOPS replies to runtime geometry (Cascadia-Mono-12pt
    # cell metrics — 10 wide × 22 tall — match ori_term's typical
    # font config so notcurses sizes its pixel-blit to the right
    # grid).
    probe_table = build_probe_table(cell_h=22, cell_w=10,
                                    rows=rows, cols=cols)

    pid, master_fd = pty.fork()
    if pid == 0:
        try:
            os.execvpe(argv[0], argv, env)
        except FileNotFoundError:
            sys.stderr.write(f"capture: command not found: {argv[0]}\n")
            os._exit(127)
        except OSError as e:
            sys.stderr.write(f"capture: exec failed: {e}\n")
            os._exit(126)

    try:
        set_winsize(master_fd, rows, cols)
    except OSError as e:
        sys.stderr.write(f"capture: TIOCSWINSZ failed: {e}\n")

    start_ts = time.monotonic()

    def pump(timeout_s: float) -> bool:
        nonlocal pending_probe_scan_from
        deadline = time.monotonic() + timeout_s
        alive = True
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                break
            rlist, _, _ = select.select([master_fd], [], [], remaining)
            if not rlist:
                break
            try:
                chunk = os.read(master_fd, 8192)
            except OSError:
                alive = False
                break
            if not chunk:
                alive = False
                break
            offset = time.monotonic() - start_ts
            sink.extend(chunk)
            timing.append(TimingEntry(offset, len(chunk)))
            # Scan the newly-added range plus a 64-byte rewind to
            # handle probes split across chunk boundaries.
            scan_from = max(pending_probe_scan_from - 64, 0)
            scan_buf = bytes(sink[scan_from:])
            hits = scan_for_probes(scan_buf, probe_table)
            for start, end, reply in hits:
                try:
                    os.write(master_fd, reply)
                except OSError:
                    pass
            pending_probe_scan_from = len(sink)
        return alive

    # Initial banner drain — TUIs paint their opening scene first.
    pump(0.5)

    for event in events:
        if event.kind == "wait":
            ms = int(event.payload)
            pump(ms / 1000.0)
        elif event.kind == "key":
            try:
                payload = keyname_to_bytes(event.payload)
            except ValueError as e:
                sys.stderr.write(f"capture: {e}\n")
                os.close(master_fd)
                return 1
            try:
                os.write(master_fd, payload)
            except OSError:
                break
            pump(0.05)
        elif event.kind == "text":
            try:
                os.write(master_fd, event.payload.encode("utf-8"))
            except OSError:
                break
            pump(0.05)

    # Final drain — exit banners, alt-screen-leave.
    pump(2.0)

    try:
        os.close(master_fd)
    except OSError:
        pass

    try:
        os.waitpid(pid, os.WNOHANG)
    except OSError:
        pass

    cap_out.write_bytes(bytes(sink))
    with timing_out.open("w", encoding="utf-8") as f:
        for entry in timing:
            f.write(f"{entry.offset_sec:.6f} {entry.nbytes}\n")

    if not sink:
        sys.stderr.write("capture: empty .cap\n")
        return 2
    return 0


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("script", type=Path, help="capture .script file")
    p.add_argument("-o", "--output", type=Path, default=None,
                   help="output .cap path (default: alongside .script)")
    p.add_argument("--cols", type=int, default=DEFAULT_COLS)
    p.add_argument("--rows", type=int, default=DEFAULT_LINES)
    p.add_argument("--term", default=DEFAULT_TERM)
    args = p.parse_args()

    if args.output is None:
        cap_out = args.script.with_suffix(".cap")
    else:
        cap_out = args.output
    timing_out = cap_out.with_suffix(cap_out.suffix + ".timing")

    try:
        return run_capture(
            args.script, cap_out, timing_out,
            args.cols, args.rows, args.term,
        )
    except ValueError as e:
        sys.stderr.write(f"capture: {e}\n")
        return 1


if __name__ == "__main__":
    sys.exit(main())
