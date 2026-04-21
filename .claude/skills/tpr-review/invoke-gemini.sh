#!/usr/bin/env bash
# Hardcoded gemini CLI invocation for the tpr-review sub-agent transport.
# The sub-agent MUST NOT construct this command itself; it calls this script
# so the model and flags are not editable in its context.
#
# Model is PINNED. Do not change without updating tpr-review-design.md §2
# (Load-Bearing Invariants) and running `/tpr-review` dogfood.
#
# 429 retry: gemini's API frequently returns 429 (rate-limit / overcapacity)
# under normal load. The wrapper retries up to MAX_ATTEMPTS with exponential
# backoff starting at INITIAL_BACKOFF seconds, doubling each retry. Default:
# 5s → 10s → 20s → 40s (75s max cumulative wait across the 5-attempt window).
# The sub-agent sees only the final attempt's stdout. All attempts' stderr
# are concatenated into $RUN/gemini-stderr.txt with per-attempt markers for
# postmortem.
#
# Usage: invoke-gemini.sh <scratch_dir>
#
# Reads:  $RUN/prompt.md, optionally $RUN/prompt-gemini-depth.md
# Writes: $RUN/gemini-stdout.txt  (final attempt's stdout only)
#         $RUN/gemini-stderr.txt  (all attempts' stderr, concatenated)
# Exit:   the final attempt's exit code.
set -uo pipefail

RUN="${1:?usage: invoke-gemini.sh SCRATCH_DIR}"
[ -f "$RUN/prompt.md" ] || { echo "ERROR: $RUN/prompt.md missing" >&2; exit 1; }

GEMINI_MODEL="gemini-3-flash-preview"
MAX_ATTEMPTS=5
INITIAL_BACKOFF=5   # seconds; doubled per retry: 5, 10, 20, 40 (75s max cumulative)

if [ -f "$RUN/prompt-gemini-depth.md" ]; then
  PROMPT="$(printf 'You are gemini.\n\n'; cat "$RUN/prompt.md"; printf '\n\n'; cat "$RUN/prompt-gemini-depth.md")"
else
  PROMPT="$(printf 'You are gemini.\n\n'; cat "$RUN/prompt.md")"
fi

RC=0
attempt=0
BACKOFF="$INITIAL_BACKOFF"
for attempt in $(seq 1 "$MAX_ATTEMPTS"); do
  ATTEMPT_STDERR="$RUN/gemini-stderr.attempt-$attempt.txt"

  gemini -m "$GEMINI_MODEL" --approval-mode yolo --output-format stream-json \
    -p "$PROMPT" 2>"$ATTEMPT_STDERR" | tee "$RUN/gemini-stdout.txt"
  RC=${PIPESTATUS[0]}

  if [ "$RC" -eq 0 ]; then
    break
  fi

  # Retry only on 429-class failures (rate limit / overcapacity).
  # Match: JSON "status": 429, HTTP/N 429, "Too Many Requests", "Rate limit".
  if grep -qE '"status":[[:space:]]*429|HTTP/[0-9.]+ 429|Too Many Requests|Rate limit|429 [A-Za-z]' \
       "$ATTEMPT_STDERR"; then
    if [ "$attempt" -lt "$MAX_ATTEMPTS" ]; then
      printf '=== invoke-gemini.sh attempt %s hit 429; sleeping %ss before retry ===\n' \
        "$attempt" "$BACKOFF" >> "$ATTEMPT_STDERR"
      sleep "$BACKOFF"
      BACKOFF=$((BACKOFF * 2))
      continue
    fi
  fi

  break
done

# Concatenate all attempts' stderr into the canonical stderr file with
# per-attempt markers. Remove the per-attempt files after merge.
: > "$RUN/gemini-stderr.txt"
for a in $(seq 1 "$attempt"); do
  AF="$RUN/gemini-stderr.attempt-$a.txt"
  if [ -f "$AF" ]; then
    printf '=== invoke-gemini.sh attempt %s of %s ===\n' "$a" "$MAX_ATTEMPTS" >> "$RUN/gemini-stderr.txt"
    cat "$AF" >> "$RUN/gemini-stderr.txt"
    printf '\n' >> "$RUN/gemini-stderr.txt"
    rm -f "$AF"
  fi
done

exit "$RC"
