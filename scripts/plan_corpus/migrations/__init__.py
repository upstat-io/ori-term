"""Migration registry for plan_corpus schema evolution.

Each migration module in this directory declares three module-level
attributes:

    VERSION: int            # post-migration schema version
    DESCRIPTION: str        # one-line description of what changes
    def up(consumer_root: Path) -> MigrationReport: ...

Modules are discovered in numeric order by filename prefix (NNN_).
The migrate subcommand reads the consumer pin, computes the sequence
pin+1 → CURRENT_SCHEMA_VERSION, and applies migrations in order.

Migrations are FORWARD-ONLY. There is no `down()`. Consumers wanting to
undo a migration revert via git on the data files plus restore the
pre-migration pin; the registry does not assist downgrades.

Failure mode: if any migration's `up()` raises, the migrate subcommand
halts with a clear error AND does NOT advance the consumer pin. The
consumer addresses the issue (data fix, code revert, manual cleanup)
and re-runs migrate to retry from the failed migration onward.

Migration authors:
- Use the existing scripts.plan_corpus types (Finding / Severity /
  Outcome / FindingCategory / FindingSubtype) when reporting issues
  rather than inventing parallel taxonomies.
- Treat MigrationReport.notes as freeform operator-facing text — used
  for "skipped X because Y" or "transformed N files" summaries.
- Keep migrations idempotent where possible: re-running an already-
  applied migration against a partially-migrated tree should be a
  no-op rather than a double-application. The migrate subcommand
  guards against re-application across runs (via pin advancement);
  in-migration idempotency is a defense-in-depth concern.
"""

from __future__ import annotations

import importlib
import pkgutil
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable


@dataclass(frozen=True)
class MigrationReport:
    """Structured report of one migration's effect on a consumer tree."""

    version: int
    description: str
    files_changed: tuple[Path, ...] = field(default_factory=tuple)
    files_unchanged: tuple[Path, ...] = field(default_factory=tuple)
    notes: tuple[str, ...] = field(default_factory=tuple)


@dataclass(frozen=True)
class RegisteredMigration:
    """Internal record of one discovered migration module."""

    version: int
    description: str
    module_name: str
    up: Callable[[Path], MigrationReport]


def available_migrations() -> list[RegisteredMigration]:
    """Discover migration modules in this package, sorted by VERSION.

    Iterates submodules via `pkgutil.iter_modules`, skipping `_`-prefixed
    helper modules. Each non-skipped module MUST declare VERSION (int),
    DESCRIPTION (str), and `up(consumer_root: Path) -> MigrationReport`.

    Raises ValueError when:
    - A migration module is missing one of the required attributes.
    - Two migration modules declare the same VERSION (the registry must
      be unambiguous; numbering collisions are the most common drift
      mode and should fail loudly at discovery time).

    Discovery is cheap (one-time module import) and intentionally
    non-cached at this layer — callers (CLI, tests) call once per
    invocation. Tests may need to re-discover after `monkeypatch.syspath`
    edits, which a cache would defeat.
    """
    registry: dict[int, RegisteredMigration] = {}
    package_dir = Path(__file__).parent

    for info in pkgutil.iter_modules([str(package_dir)]):
        if info.name.startswith("_"):
            continue

        module = importlib.import_module(f"{__name__}.{info.name}")
        version = getattr(module, "VERSION", None)
        description = getattr(module, "DESCRIPTION", None)
        up_fn = getattr(module, "up", None)

        missing: list[str] = []
        if not isinstance(version, int):
            missing.append(f"VERSION (got {version!r})")
        if not isinstance(description, str):
            missing.append(f"DESCRIPTION (got {description!r})")
        if not callable(up_fn):
            missing.append(f"up() (got {up_fn!r})")
        if missing:
            raise ValueError(
                f"Migration module {info.name!r} missing required attribute(s): "
                f"{', '.join(missing)}"
            )

        if version in registry:
            other = registry[version]
            raise ValueError(
                f"Duplicate migration VERSION={version}: "
                f"{other.module_name!r} and {info.name!r}. "
                f"Each migration must declare a unique VERSION matching its "
                f"filename prefix (e.g. migrations/001_X.py declares VERSION=1)."
            )

        registry[version] = RegisteredMigration(
            version=version,
            description=description,
            module_name=info.name,
            up=up_fn,
        )

    return sorted(registry.values(), key=lambda m: m.version)


def latest_registered_version() -> int:
    """Return the highest VERSION across registered migrations, or 0 if none.

    Used by the migrate subcommand and by integrity tests that verify
    `CURRENT_SCHEMA_VERSION == latest_registered_version()` so the version
    pin and the migration registry never drift apart.
    """
    migrations = available_migrations()
    if not migrations:
        return 0
    return migrations[-1].version
