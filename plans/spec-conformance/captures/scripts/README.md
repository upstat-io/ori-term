# Capture Script Format

Deterministic input scripts for `scripts/replay-capture-script.py`.
Each `.script` file is a line-oriented spec for a single PTY
interaction: which command to spawn, what keystrokes to send, when
to pause, when to terminate.

The runner (`scripts/replay-capture-script.py`) uses Python's
stdlib `pty` module — Linux-only. Section 01.5 commits the
`.cap` transcripts produced by the runner; Section 01.8's
reconciliation pass extracts tuples from the transcripts via
`catalog_coverage_check extract-capture-tuples`.

## Grammar

```
# Comments start with `#`
#
# COMMAND: <shell command> — invoked once at script start, pid
#     attached to a PTY.
#
# WAIT: <N>ms                — explicit delay in milliseconds
#                              before the next action. Minimum 10ms.
#
# KEY: <keyname>              — one keyboard event. Supported key
#                              names:
#                                 Escape / Enter / Tab / BS / Del
#                                 Space / Up / Down / Left / Right
#                                 F1 / F2 / ... / F12
#                                 PageUp / PageDown / Home / End
#                                 Ctrl+<char>    (e.g. Ctrl+C)
#                                 Alt+<char>     (e.g. Alt+F)
#                                 Shift+<key>    (e.g. Shift+Tab)
#
# TEXT: <literal>             — raw string typed as-is. No escape
#                              processing. Trailing whitespace is
#                              preserved.
#
# BLANK LINE                  — 100ms implicit delay. Blank lines
#                              are equivalent to `WAIT: 100ms`.
```

## Determinism

Scripts MUST produce byte-identical `.cap` output on repeated runs.
To achieve that:

- Pin the TERM environment variable (`xterm-256color`) via the
  `manifest.toml` entry — the runner reads `term_env` and exports
  it before spawning the command.
- Pin a fixed `LINES`/`COLUMNS` (80x24) via `TIOCSWINSZ` so the
  spawned app sees the same grid every run. The runner does this
  automatically.
- Use absolute paths for files the script opens (`/etc/passwd`,
  not `~/something`). Home directories differ per machine.
- Avoid time-of-day-dependent output (no `date` command, no
  `top`/`htop` scenes that don't scrub timestamps).
- Keep the script short (< 30 seconds) and use `WAIT:` to drain
  per-frame repainting before the next keystroke.

## Idle rejection

A capture that produces < 20 unique `(category, intermediates,
final_byte)` tuples in its first 30 seconds is REJECTED by
`verify-manifest.sh` as idle. This stops broken scripts from
silently landing empty `.cap` files that pass the sha256 check
but exercise nothing.

## Example

```
# vim-edit-passwd.script
# Starts vim on /etc/passwd, moves the cursor, enters insert mode,
# types a line, saves-as /tmp/passwd.copy, quits without touching
# the real /etc/passwd.
COMMAND: vim -u NONE /etc/passwd
WAIT: 500ms
KEY: G
WAIT: 100ms
KEY: g
KEY: g
WAIT: 100ms
KEY: i
TEXT: hello world
KEY: Escape

KEY: :
TEXT: w /tmp/passwd.copy
KEY: Enter
WAIT: 200ms

KEY: :
TEXT: q!
KEY: Enter
```

## NOT a verification mechanism

Real-app captures are **supplementary discovery**, not spec
verification. The spec-conformance plan's mission is spec-driven
per-row verification via the Section 04 `spec_chain` harness +
the existing `vttest` / `tack` / `teseq` deterministic suites.
Captures exist only to find de-facto sequences real apps emit
which the catalog has not yet rowed.

If a capture surfaces a tuple that is not in any catalog file,
Section 01.8's reconciliation pass routes it to one of three
buckets:

1. **Add a row to `catalog/de-facto-behaviors.md`** — the
   sequence is real de-facto behavior with no spec.
2. **Add a row to a primary catalog file with
   `Implementation: MISSING`** — the sequence is spec'd but
   ori_term does not dispatch it.
3. **File a `/add-bug` entry** — the sequence is broken /
   non-conformant / security-relevant in the capturing app,
   not something ori_term should catalog.
