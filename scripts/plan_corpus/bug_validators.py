"""bug_validators.py — Cross-check plan supersedes: vs bug-entry Superseded-by: markers.

This module implements Gate 1.6 of /continue-roadmap: bidirectional
supersede-drift detection.

Background (see fix-bug-design.md §4 2026-04-16 incident):
    When a plan declares `supersedes: [plans/bug-tracker/section-NN-*.md]`
    in its 00-overview.md, the bug entry in that section MUST carry a
    `Superseded by:` lifecycle marker.  Without the marker, /fix-bug Phase 0
    can't detect the routing and re-triggers the ~170k-token Phase -1
    rules-file re-read on every invocation.

    Conversely, when a bug entry has a `Superseded by:` marker but no plan
    claims it in `supersedes:`, the marker is orphaned — the plan may have
    been renamed, deleted, or the attribution was wrong.  Orphan markers
    are surfaced for manual review but are NOT auto-fixed.

Public API:
    find_supersede_drift(plans_dir: Path) -> DriftReport
    plan_auto_fixes(report: DriftReport) -> list[PlannedEdit]

Data model:
    DriftReport     — result of a full cross-check scan
    MissingMarker   — bug claimed by a plan but has no Superseded-by marker
    OrphanMarker    — bug has a Superseded-by marker but no plan claims it
    PlannedEdit     — concrete file-edit spec for inserting a missing marker

Wire format used by roadmap_scan.py Gate 1.6 payload:
    PlannedEdit.file_path       → "file" key
    PlannedEdit.bug_id          → "bug_id" key
    PlannedEdit.header_lineno   → "header_lineno" key
    PlannedEdit.insert_line     → "insert_line" key
    PlannedEdit.rationale       → "rationale" key
    OrphanMarker.bug_id                  → "bug_id" key
    OrphanMarker.bug_section_file        → filename component of "file" key
    OrphanMarker.bug_lineno              → "lineno" key
    OrphanMarker.declared_target         → "declared_target" key
    OrphanMarker.plan_overview_supersedes → "claiming_plans" key
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

try:
    import yaml
except ImportError:
    yaml = None  # type: ignore[assignment]

from scripts.plan_corpus.bug_markers import BUG_HEADER_RE, SUPERSEDED_RE, parse_bug_entries


# ---------------------------------------------------------------------------
# Regex: extract target from "Superseded by: <target>" line
# ---------------------------------------------------------------------------

# Captures everything after the colon (stripped).
_SUPERSEDED_BY_TARGET_RE = re.compile(
    r"(?im)^\s*superseded\s+by\s*:\s*(.+)$"
)


# ---------------------------------------------------------------------------
# Data model
# ---------------------------------------------------------------------------


@dataclass
class MissingMarker:
    """A plan claims a bug via supersedes: but the bug entry lacks the marker."""
    plan_name: str          # e.g. "spec-conformance"
    plan_dir: str           # e.g. "plans/spec-conformance"
    supersedes_target: str  # entry from plan supersedes: list, e.g. "plans/tack-conformance/"
    # The bug_id and bug-tracker context — may be None if the supersedes
    # target points to a plan directory (not a specific bug entry).
    # When the target is a plan directory, we skip auto-fix (ambiguous).
    bug_id: str | None = None
    bug_section_file: str | None = None
    bug_lineno: int | None = None


@dataclass
class OrphanMarker:
    """A bug entry has a Superseded-by marker but no plan claims it."""
    bug_id: str
    bug_section_file: str     # filename only, e.g. "section-04-fonts.md"
    bug_lineno: int           # 1-based line number of the bug header
    declared_target: str      # text after "Superseded by:"
    plan_overview_supersedes: list[str]  # plans that claim this bug (empty = none)


@dataclass
class DriftReport:
    """Result of a full bidirectional cross-check."""
    missing_markers: list[MissingMarker] = field(default_factory=list)
    orphan_markers: list[OrphanMarker] = field(default_factory=list)


@dataclass
class PlannedEdit:
    """A concrete file-edit spec for inserting a missing Superseded-by marker.

    The auto-fix inserts `insert_line` into the bug entry body immediately
    after the last body line (before the next blank line or next `- [`).
    The application protocol is in workflow.md Step 2c.
    """
    file_path: str    # absolute or repo-relative path to the section file
    bug_id: str       # "BUG-04-045"
    header_lineno: int   # 1-based line number of the `- [ ] BUG-...` header
    insert_line: str  # the full "  Superseded by: plans/foo/" line to insert
    rationale: str    # human-readable explanation


# ---------------------------------------------------------------------------
# YAML frontmatter reader
# ---------------------------------------------------------------------------


def _read_frontmatter(path: Path) -> dict[str, Any]:
    """Read YAML frontmatter from a Markdown file.

    Returns empty dict on any error (missing file, parse failure, no YAML).
    """
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return {}

    if not text.startswith("---"):
        return {}

    # Find closing `---`
    end = text.find("\n---", 3)
    if end == -1:
        return {}

    fm_text = text[3:end].strip()
    if yaml is None:
        return {}
    try:
        result = yaml.safe_load(fm_text)
        return result if isinstance(result, dict) else {}
    except yaml.YAMLError:
        return {}


# ---------------------------------------------------------------------------
# Plan scanner: build supersedes map
# ---------------------------------------------------------------------------


def _scan_plans_supersedes(plans_dir: Path) -> dict[str, list[str]]:
    """Scan every plan's 00-overview.md for `supersedes:` declarations.

    Returns a dict mapping supersede-target string → list of plan names that
    claim it.  The target strings are taken verbatim from the YAML list —
    they may be plan dirs ("plans/foo/") or specific files.

    Only `plans/*/00-overview.md` is scanned (not nested plans).
    """
    result: dict[str, list[str]] = {}
    for overview in sorted(plans_dir.glob("*/00-overview.md")):
        fm = _read_frontmatter(overview)
        supersedes_raw = fm.get("supersedes")
        if not supersedes_raw:
            continue
        if isinstance(supersedes_raw, str):
            targets = [supersedes_raw]
        elif isinstance(supersedes_raw, list):
            targets = [str(t) for t in supersedes_raw]
        else:
            continue
        plan_name = overview.parent.name
        for target in targets:
            t = target.strip()
            if t:
                result.setdefault(t, []).append(plan_name)
    return result


# ---------------------------------------------------------------------------
# Bug-tracker scanner: build markers map
# ---------------------------------------------------------------------------


def _scan_bug_tracker_superseded(plans_dir: Path) -> list[dict[str, Any]]:
    """Scan bug-tracker section files for entries with `Superseded by:` markers.

    Returns a list of dicts with keys:
        bug_id, section_file, header_lineno, declared_target, body_text
    """
    bug_tracker = plans_dir / "bug-tracker"
    if not bug_tracker.is_dir():
        return []

    results = []
    for section_file in sorted(bug_tracker.glob("section-*.md")):
        try:
            text = section_file.read_text(encoding="utf-8")
        except OSError:
            continue

        for entry in parse_bug_entries(text, section_file.name):
            if entry.status == "fixed":
                continue  # already resolved — skip
            body_text = "\n".join(entry.body_lines)
            m = _SUPERSEDED_BY_TARGET_RE.search(body_text)
            if m:
                target = m.group(1).strip()
                results.append({
                    "bug_id": entry.bug_id,
                    "section_file": section_file.name,
                    "header_lineno": entry.lineno,
                    "declared_target": target,
                    "body_lines": entry.body_lines,
                })
    return results


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def find_supersede_drift(plans_dir: Path) -> DriftReport:
    """Cross-check plan `supersedes:` declarations vs bug-entry `Superseded by:` markers.

    Args:
        plans_dir: Path to the `plans/` directory (repo root / "plans").

    Returns:
        DriftReport with:
          - missing_markers: plan claims a supersede target that corresponds to
            a bug entry, but the entry lacks the `Superseded by:` marker.
          - orphan_markers: bug entry has `Superseded by:` but no plan claims it
            via `supersedes:`.

    Note on scope:
        Plans whose `supersedes:` targets are plan directories (not specific
        bug-tracker section files) are recorded as MissingMarker entries with
        `bug_id=None` — the auto-fix pipeline skips these (ambiguous target;
        no single bug entry to annotate).  Only section-file targets or
        targets containing a bug-tracker path can map to a specific bug entry.
    """
    report = DriftReport()

    # Step 1 — build map: target_string → [plan_name, ...]
    plan_supersedes = _scan_plans_supersedes(plans_dir)

    # Step 2 — build map: bug_id → declared_target (for bugs with the marker)
    superseded_entries = _scan_bug_tracker_superseded(plans_dir)
    superseded_by_bug_id: dict[str, dict] = {
        e["bug_id"]: e for e in superseded_entries
    }

    # Step 3 — find orphan markers:
    # Bug has `Superseded by: <target>` but no plan claims it.
    for entry in superseded_entries:
        target = entry["declared_target"]
        claiming = plan_supersedes.get(target, [])
        if not claiming:
            report.orphan_markers.append(OrphanMarker(
                bug_id=entry["bug_id"],
                bug_section_file=entry["section_file"],
                bug_lineno=entry["header_lineno"],
                declared_target=target,
                plan_overview_supersedes=claiming,
            ))

    # Step 4 — find missing markers:
    # Plan claims a supersede target but the corresponding bug entry lacks marker.
    # We only attempt to map targets that look like bug-tracker section files
    # (e.g., "plans/bug-tracker/section-04-fonts.md") or plan directories.
    # For plan-directory targets (e.g., "plans/tack-conformance/"), we emit a
    # MissingMarker with bug_id=None — no auto-fix, just informational.
    bug_tracker_prefix = "plans/bug-tracker/"
    for target, claiming_plans in plan_supersedes.items():
        if bug_tracker_prefix in target and "section-" in target:
            # Target looks like a specific bug-tracker section file.
            # We can't pinpoint a single bug entry without more context,
            # so we leave bug_id=None for now.  The auto-fix pipeline will
            # skip entries with bug_id=None.
            section_file_name = Path(target.split(bug_tracker_prefix)[-1]).name
            report.missing_markers.append(MissingMarker(
                plan_name=claiming_plans[0] if claiming_plans else "unknown",
                plan_dir=f"plans/{claiming_plans[0]}" if claiming_plans else "",
                supersedes_target=target,
                bug_id=None,
                bug_section_file=section_file_name,
                bug_lineno=None,
            ))
        else:
            # Target is a plan directory — informational only.
            # (We don't know which specific bug entry to annotate.)
            report.missing_markers.append(MissingMarker(
                plan_name=claiming_plans[0] if claiming_plans else "unknown",
                plan_dir=f"plans/{claiming_plans[0]}" if claiming_plans else "",
                supersedes_target=target,
                bug_id=None,
                bug_section_file=None,
                bug_lineno=None,
            ))

    return report


def plan_auto_fixes(report: DriftReport) -> list[PlannedEdit]:
    """Convert a DriftReport into a list of auto-fixable PlannedEdit entries.

    Only MissingMarker entries where `bug_id` is not None are auto-fixable
    (those are entries where a plan claims a specific bug-tracker bug-entry file
    target and the bug entry was found but lacks the marker).

    Entries with `bug_id=None` (plan-directory targets, ambiguous section-file
    targets) are skipped — the auto-fix cannot pinpoint the edit location.

    Returns:
        List of PlannedEdit objects, one per fixable MissingMarker.
    """
    edits: list[PlannedEdit] = []
    for mm in report.missing_markers:
        if mm.bug_id is None or mm.bug_section_file is None or mm.bug_lineno is None:
            continue  # ambiguous target — skip auto-fix

        insert_line = f"  Superseded by: {mm.supersedes_target} (via plan `{mm.plan_name}` supersedes: field)"
        rationale = (
            f"Plan `{mm.plan_name}` declares `supersedes: [{mm.supersedes_target}]` "
            f"in its 00-overview.md, but bug entry {mm.bug_id} in "
            f"`plans/bug-tracker/{mm.bug_section_file}` lacks the "
            f"`Superseded by:` lifecycle marker. Without the marker, "
            f"/fix-bug Phase 0 cannot detect the routing and re-triggers "
            f"the ~170k-token Phase -1 waste on every invocation."
        )
        edits.append(PlannedEdit(
            file_path=f"plans/bug-tracker/{mm.bug_section_file}",
            bug_id=mm.bug_id,
            header_lineno=mm.bug_lineno,
            insert_line=insert_line,
            rationale=rationale,
        ))
    return edits
