#!/usr/bin/env python3
"""Deterministic PTY replay driver for spec-conformance captures.

Reads a `.script` file describing a scripted interactive flow
(COMMAND / WAIT / KEY / TEXT), spawns the command under a PTY with
a pinned 80x24 window and `TERM=xterm-256color`, replays the
events, and records the PTY output to a `.cap` file.

Linux-only — uses Python's stdlib `pty` module. Section 01.4
explicitly scopes this script to Linux; Windows-native capture
replay is Section 22's concern.

Usage:

    scripts/replay-capture-script.py SCRIPT_FILE [-o OUTPUT_CAP]

If `-o` is omitted, output goes next to the script as
`<stem>.cap` (stripping `.script`).

Exit codes:

    0 — capture written cleanly
    1 — script parse error / missing COMMAND / runner failure
    2 — child process exited non-zero AND the `.cap` file is
        empty (total failure). A non-zero exit with non-empty
        output is OK — many TUIs exit with 1 on clean quit.
"""

from __future__ import annotations

import argparse
import fcntl
import os
import pty
import select
import shlex
import struct
import sys
import termios
import time
from dataclasses import dataclass
from pathlib import Path

# -----------------------------------------------------------------
# Pinned terminal geometry — every capture sees the same grid.
# -----------------------------------------------------------------
COLS = 80
LINES = 24
TERM_ENV = "xterm-256color"

# -----------------------------------------------------------------
# Key-name → byte sequence table.
# -----------------------------------------------------------------
KEYS: dict[str, bytes] = {
    "Enter": b"\r",
    "Return": b"\r",
    "Escape": b"\x1b",
    "Esc": b"\x1b",
    "Tab": b"\t",
    "BS": b"\x7f",
    "Backspace": b"\x7f",
    "Del": b"\x1b[3~",
    "Delete": b"\x1b[3~",
    "Space": b" ",
    "Up": b"\x1b[A",
    "Down": b"\x1b[B",
    "Right": b"\x1b[C",
    "Left": b"\x1b[D",
    "Home": b"\x1b[H",
    "End": b"\x1b[F",
    "PageUp": b"\x1b[5~",
    "PageDown": b"\x1b[6~",
    "F1": b"\x1bOP",
    "F2": b"\x1bOQ",
    "F3": b"\x1bOR",
    "F4": b"\x1bOS",
    "F5": b"\x1b[15~",
    "F6": b"\x1b[17~",
    "F7": b"\x1b[18~",
    "F8": b"\x1b[19~",
    "F9": b"\x1b[20~",
    "F10": b"\x1b[21~",
    "F11": b"\x1b[23~",
    "F12": b"\x1b[24~",
}


def ctrl(ch: str) -> bytes:
    """Encode `Ctrl+<letter>` / `Ctrl+<digit>` as the legacy ASCII
    control byte (e.g., Ctrl+C → 0x03, Ctrl+[ → 0x1b)."""
    if len(ch) != 1:
        raise ValueError(f"Ctrl+ expects a single character, got {ch!r}")
    c = ch.upper()
    if c.isalpha():
        return bytes([ord(c) - ord("A") + 1])
    if c == "[":
        return b"\x1b"
    if c == "]":
        return b"\x1d"
    if c == "\\":
        return b"\x1c"
    if c == "^":
        return b"\x1e"
    if c == "_":
        return b"\x1f"
    if c == " " or c == "@":
        return b"\x00"
    raise ValueError(f"unsupported Ctrl+{ch}")


def alt(ch: str) -> bytes:
    """Encode `Alt+<char>` as ESC-prefixed (xterm convention)."""
    if len(ch) != 1:
        raise ValueError(f"Alt+ expects a single character, got {ch!r}")
    return b"\x1b" + ch.encode("utf-8")


def keyname_to_bytes(name: str) -> bytes:
    """Resolve a key name to the byte sequence a real terminal
    would send when the user presses it under `TERM=xterm-
    256color`."""
    if name.startswith("Ctrl+"):
        return ctrl(name[len("Ctrl+"):])
    if name.startswith("Alt+"):
        return alt(name[len("Alt+"):])
    if name.startswith("Shift+"):
        # Shift is handled by the base key encoding for ASCII;
        # for function keys and arrows, xterm-256color does not
        # expose a distinct Shift+Up sequence by default, so we
        # fall through to the base key. Scripts that need
        # Shift-modified function keys should use the full
        # CSI sequence via `TEXT:` instead.
        return keyname_to_bytes(name[len("Shift+"):])
    if name in KEYS:
        return KEYS[name]
    if len(name) == 1:
        return name.encode("utf-8")
    raise ValueError(f"unknown key name: {name!r}")


@dataclass
class Event:
    """One parsed script event."""
    kind: str     # "command" | "wait" | "key" | "text"
    payload: str  # command string / delay-ms str / key name / literal text


class ScriptParseError(Exception):
    """Parse failure surfaces as a non-zero exit code."""


def parse_script(path: Path) -> tuple[list[str], list[Event]]:
    """Parse a `.script` file. Returns (command_argv, events)."""
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
            stripped = line.strip()
            if stripped.startswith("#"):
                continue
            if ": " in stripped:
                head, rest = stripped.split(": ", 1)
            elif stripped.endswith(":"):
                head, rest = stripped[:-1], ""
            else:
                raise ScriptParseError(
                    f"{path}:{lineno}: malformed line {line!r}"
                )
            head = head.strip().upper()
            if head == "COMMAND":
                if command_line is not None:
                    raise ScriptParseError(
                        f"{path}:{lineno}: duplicate COMMAND"
                    )
                command_line = rest
            elif head == "WAIT":
                if not rest.endswith("ms"):
                    raise ScriptParseError(
                        f"{path}:{lineno}: WAIT expects <N>ms, got {rest!r}"
                    )
                try:
                    _ = int(rest[:-2])
                except ValueError as e:
                    raise ScriptParseError(
                        f"{path}:{lineno}: WAIT parse error: {e}"
                    ) from e
                events.append(Event("wait", rest[:-2]))
            elif head == "KEY":
                events.append(Event("key", rest))
            elif head == "TEXT":
                events.append(Event("text", rest))
            else:
                raise ScriptParseError(
                    f"{path}:{lineno}: unknown directive {head!r}"
                )
    if command_line is None:
        raise ScriptParseError(f"{path}: missing COMMAND directive")
    argv = shlex.split(command_line)
    return argv, events


def set_winsize(fd: int, rows: int, cols: int) -> None:
    """Pin the PTY window size via TIOCSWINSZ."""
    winsize = struct.pack("HHHH", rows, cols, 0, 0)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, winsize)


def drain_reads(master_fd: int, sink: bytearray, timeout_s: float) -> bool:
    """Pump bytes from the PTY master into `sink` for up to
    `timeout_s` seconds. Returns True if the child is still
    readable, False on EOF/HUP."""
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
        sink.extend(chunk)
    return alive


def run_script(script: Path, cap_out: Path) -> int:
    argv, events = parse_script(script)

    env = os.environ.copy()
    env["TERM"] = TERM_ENV
    env["LINES"] = str(LINES)
    env["COLUMNS"] = str(COLS)

    # Neutralize locale-dependent output where possible; leave
    # LANG alone so UTF-8 handling stays consistent with CI.
    env.setdefault("LC_ALL", "C.UTF-8")
    # Some TUIs read XDG_STATE_HOME / XDG_CACHE_HOME and emit
    # version banners that include timestamps — pin them to a
    # scratch dir under /tmp so they do not pollute the user's
    # home during replay.
    scratch = Path("/tmp/ori_capture_scratch")
    scratch.mkdir(exist_ok=True)
    env["XDG_STATE_HOME"] = str(scratch / "state")
    env["XDG_CACHE_HOME"] = str(scratch / "cache")
    env["XDG_CONFIG_HOME"] = str(scratch / "config")

    sink = bytearray()
    pid, master_fd = pty.fork()
    if pid == 0:
        # Child — exec the target command.
        try:
            os.execvpe(argv[0], argv, env)
        except FileNotFoundError:
            sys.stderr.write(f"replay: command not found: {argv[0]}\n")
            os._exit(127)
        except OSError as e:
            sys.stderr.write(f"replay: exec failed: {e}\n")
            os._exit(126)

    # Parent — pin the window, drain an initial banner, then
    # replay events.
    try:
        set_winsize(master_fd, LINES, COLS)
    except OSError as e:
        sys.stderr.write(f"replay: TIOCSWINSZ failed: {e}\n")

    # Initial banner drain: 300ms to let the TUI paint its
    # opening scene before we start pressing keys.
    drain_reads(master_fd, sink, 0.3)

    for event in events:
        if event.kind == "wait":
            ms = int(event.payload)
            drain_reads(master_fd, sink, ms / 1000.0)
        elif event.kind == "key":
            try:
                payload = keyname_to_bytes(event.payload)
            except ValueError as e:
                sys.stderr.write(f"replay: {e}\n")
                os.close(master_fd)
                return 1
            try:
                os.write(master_fd, payload)
            except OSError as e:
                sys.stderr.write(f"replay: write failed: {e}\n")
                break
            drain_reads(master_fd, sink, 0.05)
        elif event.kind == "text":
            try:
                os.write(master_fd, event.payload.encode("utf-8"))
            except OSError as e:
                sys.stderr.write(f"replay: write failed: {e}\n")
                break
            drain_reads(master_fd, sink, 0.05)

    # Final drain: up to 2 seconds to capture any exit banner
    # / cursor-show / reset / alt-screen-leave sequences.
    drain_reads(master_fd, sink, 2.0)

    try:
        os.close(master_fd)
    except OSError:
        pass

    try:
        reaped_pid, _status = os.waitpid(pid, os.WNOHANG)
    except ChildProcessError:
        reaped_pid = pid  # Already reaped by someone else.
    if reaped_pid == 0:
        # Child still running — terminate it cleanly.
        try:
            os.kill(pid, 15)
        except ProcessLookupError:
            pass
        time.sleep(0.1)
        try:
            os.waitpid(pid, os.WNOHANG)
        except ChildProcessError:
            pass

    cap_out.parent.mkdir(parents=True, exist_ok=True)
    cap_out.write_bytes(bytes(sink))

    if not sink:
        sys.stderr.write(
            f"replay: captured zero bytes — {script} is broken\n"
        )
        return 2
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Deterministic PTY replay driver for spec-"
                    "conformance captures."
    )
    parser.add_argument("script", type=Path, help=".script file to replay")
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=None,
        help="output .cap file (default: <script-stem>.cap)",
    )
    args = parser.parse_args()

    script_path: Path = args.script.resolve()
    if not script_path.exists():
        sys.stderr.write(f"replay: no such script: {script_path}\n")
        return 1

    if args.output is None:
        cap_out = script_path.with_suffix(".cap")
    else:
        cap_out = args.output.resolve()

    try:
        return run_script(script_path, cap_out)
    except ScriptParseError as e:
        sys.stderr.write(f"replay: {e}\n")
        return 1


if __name__ == "__main__":
    sys.exit(main())
