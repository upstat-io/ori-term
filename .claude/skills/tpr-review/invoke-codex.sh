#!/usr/bin/env bash
# Hardcoded codex CLI invocation for the tpr-review sub-agent transport.
# The sub-agent MUST NOT construct this command itself; it calls this script
# so the flags and arguments are not editable in its context.
#
# Usage: invoke-codex.sh <scratch_dir>
#
# Reads:  $RUN/prompt.md
# Writes: $RUN/codex-stdout.txt, $RUN/codex-stderr.txt
set -euo pipefail

RUN="${1:?usage: invoke-codex.sh SCRATCH_DIR}"
[ -f "$RUN/prompt.md" ] || { echo "ERROR: $RUN/prompt.md missing" >&2; exit 1; }

PROMPT="$(printf 'You are codex.\n\n'; cat "$RUN/prompt.md")"

codex exec --full-auto --json --ephemeral "$PROMPT" \
  2>"$RUN/codex-stderr.txt" | tee "$RUN/codex-stdout.txt"
