"""Migration runner — applies pending migrations against a consumer root.

Reads the consumer's schema-version pin, computes the migration sequence
from `pin+1` through `CURRENT_SCHEMA_VERSION`, applies each in order, and
advances the pin one step at a time as each migration succeeds.

Per §02 lock decision 6 (halt-with-clear-error):
- If a migration raises, the runner halts immediately. The pin is
  advanced through every migration that successfully completed BEFORE
  the failing one (Alembic semantics — re-running picks up at the
  failed version).
- The runner does NOT roll back successful migrations on a later
  failure — consumer data lives in git, not a database.
- Per §02 lock decision 7 (idempotency): when consumer pin equals
  CURRENT_SCHEMA_VERSION, the runner is a no-op and exits 0 without
  touching the pin file.
- Downgrades (pin > CURRENT_SCHEMA_VERSION) are unsupported — the
  runner exits with a clear error and pin untouched.

Both the CLI (`python -m scripts.plan_corpus migrate ...`) and the
sync-from-orilang skill's post-sync phase invoke `run_migrate()` —
keeping migration execution behind one entry point ensures CLI flags
and library calls share identical semantics.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path

from .migrations import (
    MigrationReport,
    RegisteredMigration,
    available_migrations,
)
from .version import (
    CURRENT_SCHEMA_VERSION,
    read_consumer_pin,
    write_consumer_pin,
)


@dataclass(frozen=True)
class MigrateResult:
    """Structured result of one `run_migrate` invocation."""

    starting_version: int
    target_version: int
    final_version: int
    applied: tuple[MigrationReport, ...] = field(default_factory=tuple)
    skipped_no_op: bool = False
    error: str | None = None
    failing_migration_version: int | None = None
    dry_run: bool = False
    planned: tuple[RegisteredMigration, ...] = field(default_factory=tuple)


def run_migrate(
    consumer_root: Path,
    *,
    dry_run: bool = False,
    from_version: int | None = None,
    to_version: int | None = None,
) -> MigrateResult:
    """Apply pending migrations against `consumer_root`.

    Without overrides, runs from `read_consumer_pin()` (or 0 if no pin)
    through `CURRENT_SCHEMA_VERSION`. `from_version` / `to_version`
    overrides are for testing and debugging — production calls leave
    both at None.

    `dry_run=True` returns the planned sequence without invoking any
    migration's `up()` and without touching the pin file.

    Raises ValueError on configuration errors (downgrade requested,
    target above current). Migration failures are reported via
    `MigrateResult.error` rather than exceptions — the caller decides
    how to surface them.
    """
    pin_value = read_consumer_pin(consumer_root)
    starting = from_version if from_version is not None else (pin_value or 0)

    target = to_version if to_version is not None else CURRENT_SCHEMA_VERSION

    if target > CURRENT_SCHEMA_VERSION:
        raise ValueError(
            f"--to {target} exceeds CURRENT_SCHEMA_VERSION "
            f"({CURRENT_SCHEMA_VERSION}) — refusing to migrate beyond the "
            f"latest authored migration."
        )
    if starting > target:
        raise ValueError(
            f"Cannot migrate from version {starting} to {target} — "
            f"downgrades are not supported (forward-only registry per "
            f"scripts/plan_corpus/migrations/__init__.py). Consumer can "
            f"revert via git on data files + manually edit the pin file "
            f"at {consumer_root}/.claude/.sync-ref/plan-corpus-version.json."
        )

    if starting == target:
        return MigrateResult(
            starting_version=starting,
            target_version=target,
            final_version=starting,
            skipped_no_op=True,
            dry_run=dry_run,
        )

    sequence = [
        m for m in available_migrations()
        if starting < m.version <= target
    ]

    if dry_run:
        return MigrateResult(
            starting_version=starting,
            target_version=target,
            final_version=starting,
            dry_run=True,
            planned=tuple(sequence),
        )

    applied: list[MigrationReport] = []
    current = starting

    for migration in sequence:
        try:
            report = migration.up(consumer_root)
        except Exception as exc:
            return MigrateResult(
                starting_version=starting,
                target_version=target,
                final_version=current,
                applied=tuple(applied),
                error=str(exc),
                failing_migration_version=migration.version,
                dry_run=False,
            )
        applied.append(report)
        current = migration.version
        write_consumer_pin(
            consumer_root,
            current,
            last_migrated_at=_now_iso(),
        )

    return MigrateResult(
        starting_version=starting,
        target_version=target,
        final_version=current,
        applied=tuple(applied),
        dry_run=False,
    )


def format_migrate_result(result: MigrateResult) -> str:
    """Render a MigrateResult as operator-facing text for the CLI."""
    lines: list[str] = []

    if result.skipped_no_op:
        lines.append(
            f"plan_corpus schema is already at version "
            f"{result.starting_version} — no migrations to apply."
        )
        return "\n".join(lines)

    if result.dry_run:
        lines.append(
            f"DRY RUN: {len(result.planned)} migration(s) would apply "
            f"({result.starting_version} → {result.target_version}):"
        )
        for m in result.planned:
            lines.append(f"  - {m.version:03d} {m.module_name}: {m.description}")
        lines.append("")
        lines.append(
            "Re-run without --dry-run to apply. The consumer pin file "
            "will advance one step at a time as each migration succeeds."
        )
        return "\n".join(lines)

    if result.error is not None:
        lines.append(
            f"Migration HALTED at version {result.failing_migration_version} "
            f"(starting={result.starting_version}, "
            f"target={result.target_version}, "
            f"reached={result.final_version})."
        )
        if result.applied:
            lines.append("")
            lines.append("Successfully applied before halt:")
            for r in result.applied:
                lines.append(f"  ✓ {r.version:03d}: {r.description}")
        lines.append("")
        lines.append("Error from failing migration:")
        for line in result.error.splitlines():
            lines.append(f"  {line}")
        lines.append("")
        lines.append(
            f"Consumer pin is at version {result.final_version}. Re-run "
            f"`python -m scripts.plan_corpus migrate` after addressing "
            f"the issue to continue from the failing migration."
        )
        return "\n".join(lines)

    lines.append(
        f"Applied {len(result.applied)} migration(s) "
        f"({result.starting_version} → {result.final_version}):"
    )
    for r in result.applied:
        lines.append(f"  ✓ {r.version:03d}: {r.description}")
        for note in r.notes:
            lines.append(f"      {note}")
    return "\n".join(lines)


def _now_iso() -> str:
    """Current UTC timestamp in ISO 8601 format with Z suffix."""
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
