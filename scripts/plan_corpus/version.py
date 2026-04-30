"""Schema version pin for the plan_corpus canonical schema.

SSOT for "what version of plan_corpus is the canonical schema authority on,
right now." Downstream projects synced via /sync-from-orilang record the
version their data was last migrated to in:

    <consumer_root>/.claude/.sync-ref/plan-corpus-version.json

The `migrate` subcommand compares the consumer pin against
CURRENT_SCHEMA_VERSION below and applies any pending migrations registered
under scripts/plan_corpus/migrations/.

Bump policy: increment CURRENT_SCHEMA_VERSION whenever a validator,
schema rule, or required-field set tightens in a way that would reject
data authored against the prior shape. Add a corresponding migration
module under scripts/plan_corpus/migrations/<NNN>_<slug>.py BEFORE
landing the bump (without the migration, downstream consumers break on
their next /sync-from-orilang).
"""

from __future__ import annotations

import json
from pathlib import Path

# Bump this when a new migration ships. Migration files are numbered to
# match: scripts/plan_corpus/migrations/<NNN>_<slug>.py declares
# VERSION = NNN. The invariant `CURRENT_SCHEMA_VERSION ==
# latest_registered_version()` is enforced by the migrate subcommand
# and by integrity tests so the pin and the registry never drift apart.
# Version 0 means "no migrations authored yet" (initial-state default
# for fresh consumer pins).
CURRENT_SCHEMA_VERSION: int = 1

# Stable consumer-side pin location. Lives under .claude/.sync-ref/ which
# already exists in synced consumers as the rules sidecar dir
# (`~/.claude/skills/sync-from-orilang/SKILL.md` line 96). Reusing it
# keeps all sync-related metadata under one root.
CONSUMER_PIN_RELATIVE_PATH: Path = Path(".claude/.sync-ref/plan-corpus-version.json")


def consumer_pin_path(consumer_root: Path) -> Path:
    """Resolve the absolute pin file path for a consumer root."""
    return consumer_root / CONSUMER_PIN_RELATIVE_PATH


def read_consumer_pin(consumer_root: Path) -> int | None:
    """Return the consumer's recorded schema version, or None if unset.

    Returns None when the pin file does not exist (treated as version 0
    by the migrate subcommand — apply all migrations).

    Raises ValueError if the pin file exists but is malformed or carries
    a non-integer schema_version. The migrate subcommand surfaces these
    as halt-with-clear-error cases per §02 consensus failure-mode
    decision.
    """
    pin_path = consumer_pin_path(consumer_root)
    if not pin_path.is_file():
        return None
    try:
        payload = json.loads(pin_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise ValueError(
            f"consumer pin file {pin_path} is not valid JSON: {exc}"
        ) from exc
    if not isinstance(payload, dict):
        raise ValueError(
            f"consumer pin file {pin_path} must be a JSON object, "
            f"got {type(payload).__name__}"
        )
    version = payload.get("schema_version")
    if not isinstance(version, int):
        raise ValueError(
            f"consumer pin file {pin_path} has schema_version="
            f"{version!r}, must be an integer"
        )
    return version


def write_consumer_pin(
    consumer_root: Path, version: int, *, last_migrated_at: str
) -> Path:
    """Write the consumer pin to its canonical location.

    Creates parent directories if missing (the .claude/.sync-ref/ dir
    may not exist on a fresh consumer). Returns the resolved path
    written.
    """
    pin_path = consumer_pin_path(consumer_root)
    pin_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "schema_version": version,
        "last_migrated_at": last_migrated_at,
    }
    pin_path.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return pin_path
