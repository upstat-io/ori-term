"""CLI entry point for the plan corpus library.

Invoke via `python -m scripts.plan_corpus`.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

from collections import defaultdict

from .types import Finding, FindingCategory, FindingSubtype, Outcome, PLANS_DIR, ROADMAP_DIR, REPO_ROOT
from .discovery import discover_corpus, load_and_validate
from .docgen import generate_schema_reference
from .migrate import run_migrate, format_migrate_result
from .schema import FileClass, classify_file, validate_roadmap_ownership_corpus


def main() -> int:
    import argparse

    parser = argparse.ArgumentParser(
        description="Ori plan corpus parser and validator"
    )
    sub = parser.add_subparsers(dest="command")

    check_p = sub.add_parser(
        "check",
        help=(
            "Validate a file or directory. Exits 1 only on findings with "
            "Outcome.ERROR; WARNING findings are printed but non-gating."
        ),
    )
    check_p.add_argument("paths", nargs="+", type=Path)
    check_p.add_argument("--json", action="store_true")
    check_p.add_argument(
        "--strict-recon",
        action="store_true",
        help=(
            "Escalate `status: not-started` PLAN_SECTION missing/stub "
            "Intelligence Reconnaissance findings from Outcome.WARNING to "
            "Outcome.ERROR (gates CI). `status: in-progress` findings are "
            "NOT escalated; graph-unavailable documentation is NOT "
            "escalated. See §06 in plans/query-intel-adoption for the "
            "full enforcement contract."
        ),
    )

    sub.add_parser("discover", help="Discover and report corpus")

    docgen_p = sub.add_parser("docgen", help="Generate schema reference")
    docgen_p.add_argument("--check", action="store_true",
                          help="Compare against committed file, exit non-zero on diff")

    migrate_p = sub.add_parser(
        "migrate",
        help=(
            "Apply pending plan_corpus schema migrations against a consumer "
            "project. Reads the consumer pin at "
            ".claude/.sync-ref/plan-corpus-version.json, computes the "
            "migration sequence from pin+1 through CURRENT_SCHEMA_VERSION, "
            "and applies each in order. Halt-on-failure with pin advanced "
            "through every successfully-applied migration."
        ),
    )
    migrate_p.add_argument(
        "--consumer-root", type=Path, default=Path.cwd(),
        help="Consumer project root (default: current working directory).",
    )
    migrate_p.add_argument(
        "--dry-run", action="store_true",
        help="Print the planned migration sequence without applying anything.",
    )
    migrate_p.add_argument(
        "--from", type=int, default=None, dest="from_version",
        help="Override starting version (default: consumer pin or 0).",
    )
    migrate_p.add_argument(
        "--to", type=int, default=None, dest="to_version",
        help="Override target version (default: CURRENT_SCHEMA_VERSION).",
    )

    coherence_p = sub.add_parser(
        "coherence",
        help=(
            "Run the plan-coherence lint over a plan directory or single "
            "section. Detects body-prose drift between overview / index / "
            "MS-criteria / dep-graph and the actual section files (missing "
            "section, frontmatter↔filename mismatch, overview-label drift, "
            "MS-criterion → no section, dep-graph dead edge). Exit 1 on "
            "any finding. See scripts/plan_corpus/coherence.py for the "
            "full reference-extraction protocol."
        ),
    )
    coherence_p.add_argument(
        "path",
        type=Path,
        help=(
            "Plan directory (with 00-overview.md / index.md / "
            "section-NN-*.md) OR a single section file (the lint resolves "
            "the plan dir from the parent directory)."
        ),
    )
    coherence_p.add_argument(
        "--target",
        type=Path,
        default=None,
        help=(
            "Optional target section path. Recorded in human output to "
            "mark which section triggered the gate; does NOT scope the "
            "lint (coherence is plan-wide by construction)."
        ),
    )
    coherence_p.add_argument("--json", action="store_true")

    export_p = sub.add_parser(
        "export",
        help=(
            "Export corpus + DAG as a deterministic Neo4j-flavored JSON "
            "envelope (to stdout or --output file)."
        ),
    )
    export_p.add_argument(
        "--output", type=Path, default=None,
        help="Write to file instead of stdout",
    )
    export_p.add_argument(
        "--no-references", action="store_true",
        help=(
            "Omit reference-only relationships (PROSE_VERB, YAML_COMMENT, "
            "HTML_COMMENT_CONVENTION, EXPLICIT_REFERENCES). Edge-backed "
            "relationships and structural edges still emit."
        ),
    )

    args = parser.parse_args()

    if args.command == "check":
        all_findings: list[Finding] = []
        touched_roadmap = False
        for p in args.paths:
            if p.is_dir():
                for md in sorted(p.rglob("*.md")):
                    result = load_and_validate(md, strict_recon=args.strict_recon)
                    if result.err:
                        all_findings.append(result.err)
                    elif result.ok:
                        all_findings.extend(result.ok.violations)
                    if classify_file(md) == FileClass.ROADMAP_SECTION:
                        touched_roadmap = True
            else:
                result = load_and_validate(p, strict_recon=args.strict_recon)
                if result.err:
                    all_findings.append(result.err)
                elif result.ok:
                    all_findings.extend(result.ok.violations)
                if classify_file(p) == FileClass.ROADMAP_SECTION:
                    touched_roadmap = True

        # Corpus-level roadmap `owns_crates` validation — fires whenever ANY
        # roadmap section was in scope. Loads the entire roadmap directory so
        # exclusivity/orphan findings reflect the full picture regardless of
        # which single section the user asked to check.
        if touched_roadmap and ROADMAP_DIR.is_dir():
            sections_data: dict[Path, dict] = {}
            for md in sorted(ROADMAP_DIR.glob("section-*.md")):
                r = load_and_validate(md, strict_recon=args.strict_recon)
                if r.ok is not None:
                    sections_data[md] = r.ok.data
            all_findings.extend(validate_roadmap_ownership_corpus(sections_data))

        if args.json:
            print(json.dumps([f.to_json() for f in all_findings], indent=2))
        else:
            for f in sorted(all_findings, key=lambda f: (-f.severity.value, str(f.source))):
                print(f.to_markdown())

        # Exit-code policy (§06.2 Design Decision 4): gate on Outcome.ERROR
        # only. WARNING findings are printed for visibility but do not fail
        # the check. This keeps the warnings-as-signal channel useful
        # without breaking CI on every `not-started` section that hasn't
        # yet done its recon.
        errors = [f for f in all_findings if f.outcome == Outcome.ERROR]
        return 1 if errors else 0

    elif args.command == "coherence":
        from .coherence import lint_plan_dir, lint_section

        path = args.path
        if path.is_file() and path.suffix == ".md":
            findings = lint_section(path)
            scope_label = f"section {path}"
        elif path.is_dir():
            findings = lint_plan_dir(path, target_section=args.target)
            scope_label = f"plan {path}"
            if args.target is not None:
                scope_label += f" (gate triggered by {args.target})"
        else:
            print(
                f"ERROR: coherence path must be a plan directory or a "
                f".md section file: {path}",
                file=sys.stderr,
            )
            return 2

        if args.json:
            print(json.dumps([f.to_json() for f in findings], indent=2))
        else:
            print(f"# Plan-coherence lint — {scope_label}")
            if not findings:
                print("\nNo findings — overview / index / sections agree.")
            else:
                print(f"\nFound {len(findings)} coherence finding(s):\n")
                for f in sorted(
                    findings,
                    key=lambda f: (-f.severity.value, str(f.source), f.source_line or 0),
                ):
                    print(f.to_markdown())
                    print()

        # Coherence findings are always Outcome.ERROR (gate-blocking by
        # construction — drift is the failure mode the lint exists to
        # catch). Exit policy mirrors `check`: return 1 on any finding.
        errors = [f for f in findings if f.outcome == Outcome.ERROR]
        return 1 if errors else 0

    elif args.command == "discover":
        corpus = discover_corpus()
        print(f"Plan indexes: {len(corpus.indexes)}")
        print(f"Completed indexes: {len(corpus.completed_indexes)}")
        print(f"Plan sections: {len(corpus.plan_sections)}")
        print(f"Roadmap sections: {len(corpus.roadmap_sections)}")
        print(f"Overviews: {len(corpus.overviews)}")
        print(f"Bug sections: {len(corpus.bug_sections)}")
        print(f"Fix-BUG files: {len(corpus.fix_bug_files)}")
        print(f"Name index: {len(corpus.name_index)} plans")
        print(f"Gaps: {len(corpus.gaps)}")
        for gap in corpus.gaps:
            print(f"  {gap.to_markdown()}")
        _print_recon_coverage(corpus)
        return 0

    elif args.command == "export":
        from .dag import build_dag
        from .export_json import export_neo4j_json
        corpus = discover_corpus()
        # Gate: refuse to export if any plan-admission file
        # (`plans/<dir>/index.md` or `plans/<dir>/00-overview.md`, excluding
        # plans/completed/, plans/roadmap/, bug-tracker/) produced a
        # parse_error finding. A parse failure on these files means the
        # corresponding plan is silently dropped from the Neo4j graph —
        # exit 1 is the correct signal so the sync pipeline
        # (`sync_plan_bug_graph.py run_full()`) fails loudly instead of
        # importing a partial corpus.
        plans_root = PLANS_DIR.resolve()
        admission_parse_errors = []
        for f in corpus.gaps:
            if f.category != FindingCategory.PARSE_ERROR:
                continue
            try:
                rel = Path(f.source).resolve().relative_to(plans_root)
            except ValueError:
                continue
            if rel.parts[0] in {"completed", "roadmap", "bug-tracker"}:
                continue
            if Path(f.source).name in {"index.md", "00-overview.md"}:
                admission_parse_errors.append(f)
        if admission_parse_errors:
            print(
                "ERROR: plan_corpus export refuses to emit — parse_error on "
                f"{len(admission_parse_errors)} plan-admission file(s) would "
                "silently drop plans from the Neo4j graph:",
                file=sys.stderr,
            )
            for f in admission_parse_errors:
                print(f"  - {f.source}: {f.description}", file=sys.stderr)
            print(
                "Fix the listed file(s) (add YAML frontmatter, fix YAML "
                "syntax) and re-run.",
                file=sys.stderr,
            )
            return 1
        dag = build_dag(corpus)
        envelope = export_neo4j_json(
            corpus, dag, include_references=not args.no_references
        )
        rendered = json.dumps(envelope, indent=2, sort_keys=True)
        if args.output:
            args.output.write_text(rendered, encoding="utf-8")
            print(
                f"Wrote {len(envelope['nodes'])} nodes, "
                f"{len(envelope['relationships'])} relationships to {args.output}"
            )
        else:
            print(rendered)
        return 0

    elif args.command == "migrate":
        try:
            result = run_migrate(
                args.consumer_root,
                dry_run=args.dry_run,
                from_version=args.from_version,
                to_version=args.to_version,
            )
        except ValueError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 1
        print(format_migrate_result(result))
        if result.error is not None:
            return 1
        return 0

    elif args.command == "docgen":
        ref = generate_schema_reference()
        target = REPO_ROOT / "docs" / "internal" / "plan-schema-reference.md"
        if args.check:
            if target.exists():
                committed = target.read_text().replace("\r\n", "\n")
                if committed == ref:
                    print("Schema reference is up to date.")
                    return 0
                else:
                    print("Schema reference is OUT OF DATE. Regenerate with:")
                    print(f"  python -m scripts.plan_corpus docgen > {target}")
                    return 1
            else:
                print(f"Schema reference not found at {target}. Generate with:")
                print(f"  python -m scripts.plan_corpus docgen > {target}")
                return 1
        else:
            # `end=""` so shell redirect (`docgen > file.md`) produces bytes
            # identical to `generate_schema_reference()` output — otherwise
            # `print(ref)` appends a second `\n` and the next `docgen --check`
            # run flags the file as drifted against its own generator.
            print(ref, end="")
            return 0

    else:
        parser.print_help()
        return 0


def _print_recon_coverage(corpus) -> None:
    """Print a per-plan Intelligence Reconnaissance coverage table.

    Reads each PLAN_SECTION's `ValidatedFile.violations` (already populated
    by `body_validator` during `load_and_validate`) to classify coverage:

    * Missing   — any `MISSING_RECON_BLOCK` violation present
    * Present   — no missing violation (stub / graph-unavailable / complete
                  all count as PRESENT; quality issues surface in `check`)

    Grouped by plan directory and section `status`. §09 consumes this
    table to measure retrofit completeness against `not-started` sections.
    """
    # plan_dir -> status -> (present, total)
    buckets: dict[Path, dict[str, list[int]]] = defaultdict(
        lambda: {
            "not-started": [0, 0],
            "in-progress": [0, 0],
            "complete":    [0, 0],
        }
    )
    for path in sorted(corpus.plan_sections.keys()):
        result = load_and_validate(path)
        if not result.is_ok or result.ok is None:
            continue
        vf = result.ok
        status = str(vf.data.get("status", "")).strip()
        if status not in ("not-started", "in-progress", "complete"):
            # Unknown statuses are excluded — schema validation surfaces
            # these separately via ENUM_OUT_OF_RANGE.
            continue
        plan_dir = path.parent
        buckets[plan_dir][status][1] += 1
        if status == "complete":
            # `complete` sections are exempt — present-count is pinned
            # equal to total so the table reads as "N/N exempt".
            buckets[plan_dir][status][0] += 1
            continue
        # Present = no MISSING_RECON_BLOCK violation. Stub / graph-unavailable
        # still count as present (the block exists; quality is a `check`
        # concern).
        is_missing = any(
            f.subtype == FindingSubtype.MISSING_RECON_BLOCK
            for f in vf.violations
        )
        if not is_missing:
            buckets[plan_dir][status][0] += 1

    if not buckets:
        print("\nPer-plan recon coverage: (no plan sections found)")
        return

    print("\nPer-plan recon coverage:")
    # Format plan_dir as a relative path under `plans/` so output is stable
    # across checkout locations.
    from .types import PLANS_DIR
    width = max(
        len(_rel_plan_dir(d, PLANS_DIR)) for d in buckets.keys()
    )
    for plan_dir in sorted(buckets.keys()):
        counts = buckets[plan_dir]
        ns = counts["not-started"]
        ip = counts["in-progress"]
        co = counts["complete"]
        label = _rel_plan_dir(plan_dir, PLANS_DIR).ljust(width)
        ns_cell = f"not-started: {ns[0]}/{ns[1]} PRESENCE"
        ip_cell = f"in-progress: {ip[0]}/{ip[1]} PRESENCE"
        co_cell = f"complete: {co[0]}/{co[1]} exempt"
        print(f"  {label} — {ns_cell}   {ip_cell}   {co_cell}")


def _rel_plan_dir(plan_dir: Path, plans_dir: Path) -> str:
    """Render plan_dir as `plans/<plan-name>/` (stable across checkouts)."""
    try:
        rel = plan_dir.resolve().relative_to(plans_dir.parent.resolve())
        return str(rel) + "/"
    except ValueError:
        return str(plan_dir) + "/"


if __name__ == "__main__":
    sys.exit(main())
