"""Migration 001 — owns_crates uniqueness across roadmap sections.

Schema bump rationale: the canonical strict-recon validator
`scripts/plan_corpus/schema.py::validate_roadmap_ownership_corpus` enforces
that every crate in any roadmap section's `owns_crates:` list is claimed by
EXACTLY ONE section. Consumers whose data was authored before this rule
landed will see strict-recon errors on their next post-sync `check`.

This migration's job is to surface those conflicts in an actionable
per-crate report and HALT (per §02 lock decision 6: failure mode is
halt-with-clear-error; pin is NOT advanced; consumer manually resolves and
re-runs migrate). The migration deliberately does NOT auto-pick an owner —
choosing which roadmap section should own a multi-claimed crate is a
semantic decision that requires human judgment about plan structure.

Operates on `consumer_root / "plans" / "roadmap" / "section-*.md"` — uses
the consumer's roadmap dir, NOT ori_lang's `ROADMAP_DIR` constant which
would point at the canonical authority's own roadmap.
"""

from __future__ import annotations

from pathlib import Path

from ..parser import read_text_strict, split_frontmatter_strict
from . import MigrationReport


VERSION: int = 1
DESCRIPTION: str = (
    "Detect multi-owner owns_crates state in plans/roadmap/section-*.md; "
    "halt with actionable per-crate report so consumer can manually "
    "resolve before re-running migrate."
)


def up(consumer_root: Path) -> MigrationReport:
    """Detect multi-owner owns_crates state across consumer's roadmap.

    Walks `consumer_root/plans/roadmap/section-*.md`, parses each file's
    frontmatter, builds a `crate -> [(path, idx)]` ownership map, and
    raises ValueError with a per-crate conflict report if any crate is
    claimed by 2+ sections. Returns a clean MigrationReport otherwise.

    Files that fail to parse or lack `owns_crates:` are skipped silently
    (those are schema-violation findings the strict-recon checker will
    surface separately — not this migration's concern).
    """
    roadmap_dir = consumer_root / "plans" / "roadmap"
    if not roadmap_dir.is_dir():
        return MigrationReport(
            version=VERSION,
            description=DESCRIPTION,
            notes=(
                f"no plans/roadmap/ directory at {roadmap_dir} — "
                f"nothing to migrate (consumer has no roadmap)",
            ),
        )

    inspected: list[Path] = []
    section_data: dict[Path, list[str]] = {}
    parse_failures: list[Path] = []

    for md in sorted(roadmap_dir.glob("section-*.md")):
        inspected.append(md)
        try:
            text = read_text_strict(md)
            data, _ = split_frontmatter_strict(text, md)
        except Exception:
            parse_failures.append(md)
            continue
        if not isinstance(data, dict):
            continue

        owns = data.get("owns_crates")
        if not isinstance(owns, list):
            continue

        crate_strings = [c for c in owns if isinstance(c, str)]
        if crate_strings:
            section_data[md] = crate_strings

    owners: dict[str, list[tuple[Path, int]]] = {}
    for path, crates in section_data.items():
        for idx, crate in enumerate(crates):
            owners.setdefault(crate, []).append((path, idx))

    conflicts = {
        crate: claims for crate, claims in owners.items() if len(claims) >= 2
    }

    if conflicts:
        report_lines: list[str] = [
            f"Migration 001 (owns_crates uniqueness) HALTED — "
            f"{len(conflicts)} crate(s) claimed by multiple roadmap sections:",
            "",
        ]
        for crate, claims in sorted(conflicts.items()):
            report_lines.append(f"  {crate!r}:")
            for path, idx in claims:
                rel = _relative(path, consumer_root)
                report_lines.append(f"    - {rel} (owns_crates[{idx}])")
            report_lines.append("")
        report_lines.append(
            "Resolve by editing each conflicting section's owns_crates: "
            "to claim the crate in EXACTLY ONE section. Remove the crate "
            "from the others. Re-run `python -m scripts.plan_corpus migrate` "
            "after editing."
        )
        report_lines.append("")
        report_lines.append(
            "Schema-version pin will NOT advance until all conflicts are "
            "resolved (forward-only, halt-on-failure semantics per "
            "scripts/plan_corpus/migrations/__init__.py)."
        )
        raise ValueError("\n".join(report_lines))

    notes: list[str] = [
        f"Inspected {len(inspected)} roadmap section(s); "
        f"{len(section_data)} declared owns_crates; "
        f"{sum(len(c) for c in section_data.values())} total crate claim(s); "
        f"0 conflicts. owns_crates uniqueness invariant holds.",
    ]
    if parse_failures:
        notes.append(
            f"Skipped {len(parse_failures)} file(s) with parse failures "
            f"(strict-recon check will surface as separate findings)."
        )

    return MigrationReport(
        version=VERSION,
        description=DESCRIPTION,
        files_changed=(),
        files_unchanged=tuple(inspected),
        notes=tuple(notes),
    )


def _relative(path: Path, root: Path) -> Path:
    """Render path relative to consumer_root when possible; absolute fallback."""
    try:
        return path.relative_to(root)
    except ValueError:
        return path
