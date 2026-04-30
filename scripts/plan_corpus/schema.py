"""Schema definitions and validation for the seven plan file classes.

Contains FileClass, classify_file, the FILE_CLASS_META registry (pattern /
display-name / schema class / validator / status-enum), all _check_*
helpers, all _validate_* per-schema functions, and validate().

Dataclass schema shapes and status enums are homed in `.schemas` (the SSOT
for schema *shape* and constraint *values*). This module owns the *file
classification* + *validation dispatch* built on top of those shapes.
"""

from __future__ import annotations

import enum
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

from .types import (
    Finding,
    FindingCategory,
    FindingSubtype,
    Outcome,
    Severity,
    BUG_TRACKER_DIR,
    PLANS_DIR,
    ROADMAP_DIR,
    FIX_BUG_RE,
    SECTION_FILE_RE,
    OVERVIEW_RE,
    INTRA_PLAN_DEP_RE,
    CROSS_PLAN_DEP_RE,
    FULL_PATH_DEP_RE,
)
from .schemas import (
    PlanIndexSchema,
    PlanSectionSchema,
    RoadmapSectionSchema,
    OverviewSchema,
    BugTrackerSectionSchema,
    FixBugSchema,
    CompletedIndexSchema,
    AuditSchema,
    SubsectionEntry,
    TprInfo,
    PLAN_STATUSES,
    SECTION_STATUSES,
    OVERVIEW_STATUSES,
    FIX_STATUSES,
    TPR_STATUSES,
    SEVERITY_VALUES,
    COMPLETED_STATUSES,
    _schema_required_fields,
    _schema_allowed_fields,
)


# ---------------------------------------------------------------------------
# File classification
# ---------------------------------------------------------------------------


class FileClass(enum.Enum):
    """The eight schema classes for plan files."""
    PLAN_INDEX = "plan_index"
    PLAN_SECTION = "plan_section"
    ROADMAP_SECTION = "roadmap_section"
    OVERVIEW = "overview"
    BUG_TRACKER_SECTION = "bug_tracker_section"
    FIX_BUG = "fix_bug"
    COMPLETED_INDEX = "completed_index"
    AUDIT_SECTION = "audit_section"


def classify_file(path: Path) -> FileClass | None:
    """Classify a plan file into one of the seven schema classes.

    Anchors on the first `plans` OR `bug-tracker` segment in the resolved
    path. After the Phase C migration, bug-tracker/ lives at the wrapper
    root (sibling of plans/), so parts[0] == "bug-tracker" only works when
    we anchor on whichever segment appears first. Falls back to path.parts
    when neither segment exists (synthetic tmp_path corpora in tests).
    """
    path = path.resolve()
    if path.is_relative_to(BUG_TRACKER_DIR):
        # Files under <repo>/bug-tracker/*.md — prepend "bug-tracker"
        # so the len(parts) >= 2 and parts[0] == "bug-tracker" checks
        # below fire exactly as they did pre-migration.
        parts = ("bug-tracker",) + path.relative_to(BUG_TRACKER_DIR).parts
    elif path.is_relative_to(PLANS_DIR):
        parts = path.relative_to(PLANS_DIR).parts
    else:
        full_parts = path.parts
        # Scan for the first 'plans' or 'bug-tracker' segment and take
        # everything from there. 'bug-tracker' takes precedence because
        # it's now the outer directory under REPO_ROOT, so if both are
        # present in the parts list (shouldn't happen post-migration
        # but is defensible for tmp_path corpora) bug-tracker wins.
        anchor_idx: int | None = None
        for candidate in ("bug-tracker", "plans"):
            if candidate in full_parts:
                i = full_parts.index(candidate)
                if candidate == "bug-tracker":
                    # Keep the "bug-tracker" segment so parts[0] tests match.
                    parts = full_parts[i:]
                else:
                    # "plans/<name>/section-NN-*.md" → parts=("<name>", "...")
                    parts = full_parts[i + 1:]
                anchor_idx = i
                break
        if anchor_idx is None:
            parts = full_parts

    name = path.name

    # Completed plan indexes
    if len(parts) >= 2 and parts[0] == "completed" and name == "index.md":
        return FileClass.COMPLETED_INDEX

    # Bug-tracker fix files (legacy Route C — single-file fix-BUG-*.md)
    if len(parts) >= 2 and parts[0] == "bug-tracker" and FIX_BUG_RE.match(name):
        return FileClass.FIX_BUG

    # Bug-plan section files: bug-tracker/plans/[completed/]BUG-XX-NNN/section-NN-*.md.
    # These are full plan sections (PlanSectionSchema), not tracker-index entries.
    # Must be matched BEFORE the BUG_TRACKER_SECTION case below.
    if (len(parts) >= 4 and parts[0] == "bug-tracker" and parts[1] == "plans"
            and SECTION_FILE_RE.match(name)):
        return FileClass.PLAN_SECTION

    # Bug-tracker section files (tracker-index entries, top-level under bug-tracker/).
    if len(parts) >= 2 and parts[0] == "bug-tracker" and SECTION_FILE_RE.match(name):
        return FileClass.BUG_TRACKER_SECTION

    # Roadmap section files
    if len(parts) >= 2 and parts[0] == "roadmap" and SECTION_FILE_RE.match(name):
        return FileClass.ROADMAP_SECTION

    # Audit section files: plans/<plan>/audits/section-*.md.
    # `audits/` is the canonical subdir for top-down spec audits — sections that
    # walk a canonical spec source and inventory coverage. They have a different
    # required-field set than PlanSectionSchema (no status/goal/success_criteria)
    # plus audit-specific metadata (canonical_spec_sources, last_walked, walked_by,
    # audit_input). Match BEFORE the generic PLAN_SECTION fallback below.
    if "audits" in parts and SECTION_FILE_RE.match(name):
        return FileClass.AUDIT_SECTION

    # Overview files
    if OVERVIEW_RE.match(name):
        return FileClass.OVERVIEW

    # Plan index files
    if name == "index.md":
        return FileClass.PLAN_INDEX

    # Plan section files
    if SECTION_FILE_RE.match(name):
        return FileClass.PLAN_SECTION

    return None


def _validate_sections(data: dict, path: Path) -> list[Finding]:
    """Validate each entry in the sections[] list matches SubsectionEntry shape.

    Emits one finding per malformed entry (unknown field, missing required field,
    or enum violation on status). Silently returns [] if sections is absent or
    empty — the required-fields check at the top level handles absence.
    """
    findings: list[Finding] = []
    sections = data.get("sections")
    if sections is None:
        return findings
    if not isinstance(sections, list):
        findings.append(Finding(
            category=FindingCategory.SCHEMA_VIOLATION,
            subtype=FindingSubtype.WRONG_TYPE,
            severity=Severity.HIGH,
            source=path,
            description="sections must be a list",
            recommended_fix="Use sections: [{id: NN.N, title: ..., status: ...}, ...]",
        ))
        return findings
    for idx, entry in enumerate(sections):
        if not isinstance(entry, dict):
            findings.append(Finding(
                category=FindingCategory.SCHEMA_VIOLATION,
                subtype=FindingSubtype.WRONG_TYPE,
                severity=Severity.HIGH,
                source=path,
                description=f"sections[{idx}] must be a mapping, got {type(entry).__name__}",
                recommended_fix="Use a mapping with id/title/status keys",
            ))
            continue
        for required_field in _schema_required_fields(SubsectionEntry):
            if required_field not in entry:
                findings.append(Finding(
                    category=FindingCategory.SCHEMA_VIOLATION,
                    subtype=FindingSubtype.MISSING_REQUIRED_FIELD,
                    severity=Severity.HIGH,
                    source=path,
                    description=f"sections[{idx}] missing required field {required_field!r}",
                    recommended_fix=f"Add {required_field}: ... to the entry",
                    # Path-style target_key for nested fields so future auto-fix
                    # extensions can dispatch structurally instead of parsing
                    # the description prose (TPR-03-005-gemini informational).
                    target_key=f"sections[{idx}].{required_field}",
                ))
        for key in entry:
            if key not in _schema_allowed_fields(SubsectionEntry):
                findings.append(Finding(
                    category=FindingCategory.SCHEMA_VIOLATION,
                    subtype=FindingSubtype.UNKNOWN_FIELD,
                    severity=Severity.MEDIUM,
                    source=path,
                    description=f"sections[{idx}] has unknown field {key!r}",
                    recommended_fix=f"Remove {key!r} or add it to the SubsectionEntry schema",
                    target_key=f"sections[{idx}].{key}",
                ))
        status = entry.get("status")
        if status is not None and status not in SECTION_STATUSES:
            findings.append(Finding(
                category=FindingCategory.SCHEMA_VIOLATION,
                subtype=FindingSubtype.ENUM_OUT_OF_RANGE,
                severity=Severity.MEDIUM,
                source=path,
                description=(
                    f"sections[{idx}].status={status!r} not in {sorted(SECTION_STATUSES)}"
                ),
                recommended_fix=f"Use one of {sorted(SECTION_STATUSES)}",
                target_key=f"sections[{idx}].status",
            ))
    return findings


# ---------------------------------------------------------------------------
# Validation helpers
# ---------------------------------------------------------------------------


def _validate_tpr_info(raw: Any, path: Path) -> tuple[TprInfo | None, list[Finding]]:
    """Validate a third_party_review block."""
    findings: list[Finding] = []
    if raw is None:
        return None, findings
    if not isinstance(raw, dict):
        findings.append(Finding(
            category=FindingCategory.SCHEMA_VIOLATION,
            subtype=FindingSubtype.WRONG_TYPE,
            severity=Severity.HIGH,
            source=path,
            description="third_party_review must be a mapping",
            recommended_fix="Use third_party_review: {status: none, updated: null}",
        ))
        return None, findings
    status = raw.get("status")
    if status is not None:
        status = str(status)
    updated = raw.get("updated")
    if updated is not None:
        updated = str(updated)
    if status and status not in TPR_STATUSES:
        findings.append(Finding(
            category=FindingCategory.SCHEMA_VIOLATION,
            subtype=FindingSubtype.ENUM_OUT_OF_RANGE,
            severity=Severity.HIGH,
            source=path,
            description=f"third_party_review.status={status!r} not in {sorted(TPR_STATUSES)}",
            recommended_fix=f"Use one of: {', '.join(sorted(TPR_STATUSES))}",
        ))
    if status and status != "none" and updated is None:
        findings.append(Finding(
            category=FindingCategory.STATUS_CONTRADICTION,
            subtype=FindingSubtype.TPR_STATUS_WITHOUT_DATE,
            severity=Severity.MEDIUM,
            source=path,
            description=f"third_party_review.status={status!r} but updated is null",
            recommended_fix="Set third_party_review.updated to today's date",
        ))
    if status == "none" and updated is not None and updated != "null":
        findings.append(Finding(
            category=FindingCategory.STATUS_CONTRADICTION,
            subtype=FindingSubtype.TPR_STATUS_NONE_WITH_DATE,
            severity=Severity.LOW,
            source=path,
            description=f"third_party_review.status=none but updated={updated!r}",
            recommended_fix="Set updated to null when status is none",
        ))
    return TprInfo(status=status or "none", updated=updated), findings


def _check_required(
    data: dict, required: list[str], path: Path
) -> list[Finding]:
    findings = []
    for key in required:
        if key not in data:
            findings.append(Finding(
                category=FindingCategory.SCHEMA_VIOLATION,
                subtype=FindingSubtype.MISSING_REQUIRED_FIELD,
                severity=Severity.HIGH,
                source=path,
                description=f"missing required field: {key}",
                recommended_fix=f"Add '{key}:' to frontmatter",
                target_key=key,
            ))
    return findings


def _check_unknown_fields(
    data: dict, allowed: frozenset[str], path: Path
) -> list[Finding]:
    findings = []
    for key in data:
        if key not in allowed:
            findings.append(Finding(
                category=FindingCategory.SCHEMA_VIOLATION,
                subtype=FindingSubtype.UNKNOWN_FIELD,
                severity=Severity.MEDIUM,
                source=path,
                description=f"unknown field: {key!r}",
                recommended_fix=f"Remove '{key}' or check for typos. Allowed: {sorted(allowed)}",
                target_key=key,
            ))
    return findings


def _check_enum(
    data: dict, key: str, allowed: frozenset[str], path: Path
) -> list[Finding]:
    val = data.get(key)
    if val is None:
        return []
    val_str = str(val).strip()
    # Strip inline comments (e.g. "in-progress  # TPR done")
    if "#" in val_str:
        val_str = val_str.split("#")[0].strip()
    if val_str not in allowed:
        return [Finding(
            category=FindingCategory.SCHEMA_VIOLATION,
            subtype=FindingSubtype.ENUM_OUT_OF_RANGE,
            severity=Severity.HIGH,
            source=path,
            description=f"{key}={val!r} not in {sorted(allowed)}",
            recommended_fix=f"Use one of: {', '.join(sorted(allowed))}",
        )]
    return []


def _validate_dep_id(dep: Any, path: Path) -> list[Finding]:
    """Validate a single depends_on entry."""
    if not isinstance(dep, str):
        return [Finding(
            category=FindingCategory.SCHEMA_VIOLATION,
            subtype=FindingSubtype.DEP_ID_MALFORMED,
            severity=Severity.HIGH,
            source=path,
            description=f"depends_on entry must be a string, got {type(dep).__name__}",
            recommended_fix="Use depends_on: [\"01\", \"02\"]",
        )]
    if FULL_PATH_DEP_RE.match(dep):
        return [Finding(
            category=FindingCategory.SCHEMA_VIOLATION,
            subtype=FindingSubtype.DEP_ID_FULL_PATH,
            severity=Severity.HIGH,
            source=path,
            description=f"depends_on uses full path: {dep!r}",
            recommended_fix="Use logical ID (\"01\") or cross-plan (\"plan-name#01\")",
        )]
    if not INTRA_PLAN_DEP_RE.match(dep) and not CROSS_PLAN_DEP_RE.match(dep):
        return [Finding(
            category=FindingCategory.SCHEMA_VIOLATION,
            subtype=FindingSubtype.DEP_ID_MALFORMED,
            severity=Severity.HIGH,
            source=path,
            description=f"malformed depends_on ID: {dep!r}",
            recommended_fix="Use \"NN\" (intra-plan) or \"plan-name#NN\" (cross-plan)",
        )]
    return []


def _validate_depends_on(data: dict, path: Path) -> list[Finding]:
    """Validate the depends_on field if present."""
    raw = data.get("depends_on")
    if raw is None:
        return []
    if isinstance(raw, str):
        return [Finding(
            category=FindingCategory.SCHEMA_VIOLATION,
            subtype=FindingSubtype.DEP_ID_MALFORMED,
            severity=Severity.HIGH,
            source=path,
            description="depends_on must be a list, not a bare string (would iterate chars)",
            recommended_fix=f'Use depends_on: ["{raw}"]',
        )]
    if not isinstance(raw, list):
        return [Finding(
            category=FindingCategory.SCHEMA_VIOLATION,
            subtype=FindingSubtype.WRONG_TYPE,
            severity=Severity.HIGH,
            source=path,
            description=f"depends_on must be a list, got {type(raw).__name__}",
            recommended_fix="Use depends_on: [\"01\"]",
        )]
    findings = []
    for dep in raw:
        findings.extend(_validate_dep_id(dep, path))
    return findings


# ---------------------------------------------------------------------------
# Per-schema validators — required/allowed derived from dataclasses above
# ---------------------------------------------------------------------------


def _is_bug_plan_path(path: Path) -> bool:
    """Return True for files under bug-tracker/plans/[completed/]BUG-XX-NNN/.

    Bug plans use a relaxed schema variant — they do NOT carry name/full_name
    (the directory IS the routing decision per CLAUDE.md §Bug Handling), do
    NOT carry sections: arrays on section files (the 7-section roster is
    fixed), do NOT carry reviewed: on section files (single-section pre-
    implementation review does not apply to the bug-plan workflow), and DO
    carry a top-level `bug:` block on 00-overview.md.

    Canonical template: .claude/skills/fix-bug/fix-section-template.md
    """
    return "bug-tracker/plans/" in str(path)


def _validate_plan_index(data: dict, path: Path) -> list[Finding]:
    findings = []
    if _is_bug_plan_path(path):
        # Bug-plan index.md only requires `status`. The directory IS the
        # routing decision (CLAUDE.md §Bug Handling) so name/full_name do
        # not apply to the new bug-plan template, but legacy bug-plans
        # carry them — accept either shape during transition.
        bug_required = ["status"]
        bug_allowed = frozenset({
            "status", "name", "full_name", "order", "parallel",
        })
        findings.extend(_check_required(data, bug_required, path))
        findings.extend(_check_unknown_fields(data, bug_allowed, path))
    else:
        findings.extend(_check_required(data, _schema_required_fields(PlanIndexSchema), path))
        findings.extend(_check_unknown_fields(data, _schema_allowed_fields(PlanIndexSchema), path))
    findings.extend(_check_enum(data, "status", PLAN_STATUSES, path))
    if data.get("reroute") is False:
        findings.append(Finding(
            category=FindingCategory.SCHEMA_VIOLATION,
            subtype=FindingSubtype.CROSS_FIELD_INVARIANT,
            severity=Severity.MEDIUM,
            source=path,
            description="reroute: false is invalid; omit the field instead",
            recommended_fix="Remove the reroute field entirely",
            target_key="reroute",
        ))
    # Conditional-required: routing_justification: MUST be present when
    # reroute: true (Fallback-R plan per CLAUDE.md §Plan Routing — Mechanical)
    # and MUST be absent otherwise. Enforces PLAN_ROUTING_DRIFT at corpus-check
    # time rather than relying on reviewers to catch it.
    if data.get("reroute") is True and not data.get("routing_justification"):
        findings.append(Finding(
            category=FindingCategory.SCHEMA_VIOLATION,
            subtype=FindingSubtype.CROSS_FIELD_INVARIANT,
            severity=Severity.HIGH,
            source=path,
            description=(
                "reroute: true requires routing_justification: to document "
                "why the plan lands as a Fallback-R peer instead of a "
                "roadmap subsection (CLAUDE.md §Plan Routing — Mechanical)"
            ),
            recommended_fix=(
                "Add routing_justification: \"<reason>\" explaining the tie, "
                "partial-ownership, or zero-owner routing outcome. "
                "Run `scripts/plan_routing.py --scope ...` to derive."
            ),
            target_key="routing_justification",
        ))
    if data.get("routing_justification") and not data.get("reroute"):
        findings.append(Finding(
            category=FindingCategory.SCHEMA_VIOLATION,
            subtype=FindingSubtype.CROSS_FIELD_INVARIANT,
            severity=Severity.MEDIUM,
            source=path,
            description=(
                "routing_justification: is only valid when reroute: true; "
                "subsection-routable plans do not carry a justification"
            ),
            recommended_fix=(
                "Either add reroute: true (if this plan is a Fallback-R peer) "
                "or remove routing_justification: (if this plan is a "
                "subsection target)"
            ),
            target_key="routing_justification",
        ))
    # Bug plans under bug-tracker/plans/BUG-XX-NNN/ MUST NOT declare reroute:
    # or routing_justification: — the directory IS the routing decision.
    # CLAUDE.md §Bug Handling: every bug is a plan directory regardless of
    # crate ownership; there is no subsection-vs-Fallback-R choice for bugs.
    path_str = str(path)
    if "bug-tracker/plans/" in path_str:
        if "reroute" in data:
            findings.append(Finding(
                category=FindingCategory.SCHEMA_VIOLATION,
                subtype=FindingSubtype.CROSS_FIELD_INVARIANT,
                severity=Severity.HIGH,
                source=path,
                description=(
                    "bug plans MUST NOT declare reroute: — the directory "
                    "bug-tracker/plans/ IS the routing decision. Every bug "
                    "is categorically a plan directory (CLAUDE.md §Bug Handling)"
                ),
                recommended_fix="Remove the reroute field from this bug plan's index.md",
                target_key="reroute",
            ))
        if "routing_justification" in data:
            findings.append(Finding(
                category=FindingCategory.SCHEMA_VIOLATION,
                subtype=FindingSubtype.CROSS_FIELD_INVARIANT,
                severity=Severity.HIGH,
                source=path,
                description=(
                    "bug plans MUST NOT declare routing_justification: — the "
                    "directory bug-tracker/plans/ IS the routing decision"
                ),
                recommended_fix="Remove the routing_justification field from this bug plan's index.md",
                target_key="routing_justification",
            ))
    return findings


def _validate_plan_section(data: dict, path: Path) -> list[Finding]:
    findings = []
    if _is_bug_plan_path(path):
        # Bug-plan section files have a fixed 7-section roster — there is
        # no nested `sections:` tree. Single-section pre-implementation
        # review (`reviewed:` flip) does not apply to the bug-plan workflow
        # (which has its own /tp-help + /tpr-review gates at Phases 1.75
        # and 5). Required is narrow; allowed unions the new bug-plan
        # template shape with the legacy regular-plan shape (`reviewed`,
        # `sections`, `inspired_by`) so plans authored before the
        # bug-plan template diverged still validate.
        bug_required = ["section", "title", "status", "goal", "success_criteria", "third_party_review"]
        bug_allowed = frozenset({
            "section", "title", "status", "goal", "success_criteria",
            "third_party_review", "depends_on", "review_pipeline",
            "reviewed", "sections", "inspired_by", "touches", "review",
        })
        findings.extend(_check_required(data, bug_required, path))
        findings.extend(_check_unknown_fields(data, bug_allowed, path))
    else:
        findings.extend(_check_required(data, _schema_required_fields(PlanSectionSchema), path))
        findings.extend(_check_unknown_fields(data, _schema_allowed_fields(PlanSectionSchema), path))
    findings.extend(_check_enum(data, "status", SECTION_STATUSES, path))
    findings.extend(_validate_depends_on(data, path))
    if not _is_bug_plan_path(path):
        # Bug-plan sections do not carry a `sections:` tree.
        findings.extend(_validate_sections(data, path))
    _, tpr_findings = _validate_tpr_info(data.get("third_party_review"), path)
    findings.extend(tpr_findings)
    return findings


_COMPILER_CRATES_DIR = (
    Path(__file__).resolve().parent.parent.parent / "compiler_repo" / "compiler"
)


def _real_crate_names() -> frozenset[str]:
    """Lazy-cached listing of real crate directories under compiler/.

    Cached at module scope on first call. The listing changes only between
    process lifetimes (a new crate added to the workspace = a rebuild), so
    caching within a single `plan_corpus` invocation is safe. Returns an
    empty frozenset when the directory is unavailable (synthetic tmp_path
    tests); validation callers degrade gracefully.
    """
    cache = _real_crate_names.__dict__.get("_cache")
    if cache is not None:
        return cache
    if not _COMPILER_CRATES_DIR.is_dir():
        _real_crate_names._cache = frozenset()  # type: ignore[attr-defined]
        return _real_crate_names._cache  # type: ignore[attr-defined,return-value]
    _real_crate_names._cache = frozenset(  # type: ignore[attr-defined]
        p.name for p in _COMPILER_CRATES_DIR.iterdir()
        if p.is_dir() and not p.name.startswith(".")
    )
    return _real_crate_names._cache  # type: ignore[attr-defined,return-value]


def _validate_owns_crates(data: dict, path: Path) -> list[Finding]:
    """Per-file validation for `owns_crates: list[str]`.

    - Must be a list (not a scalar or mapping).
    - Each element must be a string.
    - Each element must name a real crate directory under
      `compiler/<crate>/`. Empty list `[]` is explicit
      "this section owns no crate" and is valid.
    Cross-section exclusivity + orphan-crate detection live in
    `validate_roadmap_ownership_corpus` (corpus-level).
    """
    findings: list[Finding] = []
    raw = data.get("owns_crates")
    if raw is None:
        return findings  # Missing-required-field finding is emitted by _check_required.
    if not isinstance(raw, list):
        findings.append(Finding(
            category=FindingCategory.SCHEMA_VIOLATION,
            subtype=FindingSubtype.WRONG_TYPE,
            severity=Severity.HIGH,
            source=path,
            description=f"owns_crates must be a list, got {type(raw).__name__}",
            recommended_fix='Use owns_crates: [] (empty) or owns_crates: [ori_parse, ori_lexer]',
            target_key="owns_crates",
        ))
        return findings
    real_crates = _real_crate_names()
    for idx, crate in enumerate(raw):
        if not isinstance(crate, str):
            findings.append(Finding(
                category=FindingCategory.SCHEMA_VIOLATION,
                subtype=FindingSubtype.WRONG_TYPE,
                severity=Severity.HIGH,
                source=path,
                description=f"owns_crates[{idx}] must be a string, got {type(crate).__name__}",
                recommended_fix="Use a bare crate name like 'ori_parse'",
                target_key=f"owns_crates[{idx}]",
            ))
            continue
        # If the compiler/ directory is unavailable (e.g., tests running in
        # a synthetic tmp_path), skip existence enforcement — the type check
        # above still runs.
        if real_crates and crate not in real_crates:
            findings.append(Finding(
                category=FindingCategory.SCHEMA_VIOLATION,
                subtype=FindingSubtype.ENUM_OUT_OF_RANGE,
                severity=Severity.HIGH,
                source=path,
                description=(
                    f"owns_crates[{idx}]={crate!r} is not a real crate directory "
                    f"under compiler/"
                ),
                recommended_fix=(
                    f"Use one of: {', '.join(sorted(real_crates))}"
                ),
                target_key=f"owns_crates[{idx}]",
                target_value=crate,
            ))
    return findings


def _validate_roadmap_section(data: dict, path: Path) -> list[Finding]:
    findings = []
    findings.extend(_check_required(data, _schema_required_fields(RoadmapSectionSchema), path))
    findings.extend(_check_unknown_fields(data, _schema_allowed_fields(RoadmapSectionSchema), path))
    findings.extend(_check_enum(data, "status", SECTION_STATUSES, path))
    findings.extend(_validate_depends_on(data, path))
    findings.extend(_validate_sections(data, path))
    findings.extend(_validate_owns_crates(data, path))
    if "third_party_review" in data:
        _, tpr_findings = _validate_tpr_info(data.get("third_party_review"), path)
        findings.extend(tpr_findings)
    return findings


def _validate_audit_section(data: dict, path: Path) -> list[Finding]:
    findings = []
    findings.extend(_check_required(data, _schema_required_fields(AuditSchema), path))
    findings.extend(_check_unknown_fields(data, _schema_allowed_fields(AuditSchema), path))
    findings.extend(_check_enum(data, "status", SECTION_STATUSES, path))
    findings.extend(_validate_depends_on(data, path))
    findings.extend(_validate_sections(data, path))
    if "third_party_review" in data:
        _, tpr_findings = _validate_tpr_info(data.get("third_party_review"), path)
        findings.extend(tpr_findings)
    return findings


def validate_roadmap_ownership_corpus(
    sections: dict[Path, dict],
) -> list[Finding]:
    """Corpus-level validation of roadmap `owns_crates` across ALL sections.

    Emits two kinds of findings:
    - `CROSS_FIELD_INVARIANT` (ERROR) when a crate is claimed by two sections.
    - `ENUM_OUT_OF_RANGE` (WARNING) when a real crate is orphaned — claimed by
      no section. Orphan is a WARNING because it's the Fallback R signal, not
      a failure: orphans fall through to `bug-tracker/plans/` resolution-plans.

    Callers pass a `{roadmap_section_path -> parsed_frontmatter_dict}` mapping
    for EVERY roadmap section in scope. Partial input yields partial coverage
    — the caller is responsible for collecting the full set.
    """
    findings: list[Finding] = []
    # owner[crate] = list of (path, list-index) where the crate was claimed.
    owner: dict[str, list[tuple[Path, int]]] = {}
    for path, data in sections.items():
        raw = data.get("owns_crates")
        if not isinstance(raw, list):
            continue
        for idx, crate in enumerate(raw):
            if not isinstance(crate, str):
                continue
            owner.setdefault(crate, []).append((path, idx))

    # (a) Exclusive ownership — no crate in more than one section.
    for crate, claims in sorted(owner.items()):
        if len(claims) <= 1:
            continue
        for claim_path, claim_idx in claims:
            other_claims = [
                f"{str(p)}[{i}]" for (p, i) in claims if p != claim_path
            ]
            findings.append(Finding(
                category=FindingCategory.SCHEMA_VIOLATION,
                subtype=FindingSubtype.CROSS_FIELD_INVARIANT,
                severity=Severity.HIGH,
                source=claim_path,
                description=(
                    f"crate {crate!r} is claimed by multiple roadmap sections; "
                    f"also claimed at: {', '.join(other_claims)}"
                ),
                recommended_fix=(
                    f"Remove {crate!r} from all but ONE section's owns_crates "
                    f"— crate ownership is exclusive (one section per crate)."
                ),
                target_key=f"owns_crates[{claim_idx}]",
                target_value=crate,
            ))

    # (b) Orphan-crate warning — a real crate directory with no owner.
    real_crates = _real_crate_names()
    if real_crates:
        orphans = sorted(real_crates - set(owner.keys()))
        for crate in orphans:
            findings.append(Finding(
                category=FindingCategory.GAP,
                subtype=FindingSubtype.UNCLASSIFIED_DIRECTORY,
                severity=Severity.LOW,
                source=_COMPILER_CRATES_DIR / crate,
                description=(
                    f"crate {crate!r} is not claimed by any roadmap "
                    f"section's owns_crates — bugs in this crate will "
                    f"fall through to Fallback R (bug-tracker/plans/)"
                ),
                recommended_fix=(
                    f"Add {crate!r} to the appropriate roadmap section's "
                    f"owns_crates, OR accept the Fallback R carve-out."
                ),
                outcome=Outcome.WARNING,
            ))
    return findings


# Bug-plan 00-overview.md: required fields are the union members PRESENT in
# both conventions — `plan/title/status` are universal; `references` exists
# in both shapes; `bug:` is REQUIRED only on the new template (legacy
# bug-plans omit it), so it stays in allowed-but-not-required.
_BUG_OVERVIEW_REQUIRED = ["plan", "title", "status"]
_BUG_OVERVIEW_ALLOWED = frozenset({
    "plan", "title", "status", "bug", "references", "supersedes",
    "reviewed",
})


def _validate_overview(data: dict, path: Path) -> list[Finding]:
    findings = []
    if _is_bug_plan_path(path):
        findings.extend(_check_required(data, _BUG_OVERVIEW_REQUIRED, path))
        findings.extend(_check_unknown_fields(data, _BUG_OVERVIEW_ALLOWED, path))
    else:
        findings.extend(_check_required(data, _schema_required_fields(OverviewSchema), path))
        findings.extend(_check_unknown_fields(data, _schema_allowed_fields(OverviewSchema), path))
    findings.extend(_check_enum(data, "status", OVERVIEW_STATUSES, path))
    return findings


def _validate_bug_section(data: dict, path: Path) -> list[Finding]:
    findings = []
    findings.extend(_check_required(data, _schema_required_fields(BugTrackerSectionSchema), path))
    findings.extend(_check_unknown_fields(data, _schema_allowed_fields(BugTrackerSectionSchema), path))
    findings.extend(_check_enum(data, "status", SECTION_STATUSES, path))
    return findings


def _validate_fix_bug(data: dict, path: Path) -> list[Finding]:
    findings = []
    findings.extend(_check_required(data, _schema_required_fields(FixBugSchema), path))
    findings.extend(_check_unknown_fields(data, _schema_allowed_fields(FixBugSchema), path))
    findings.extend(_check_enum(data, "status", FIX_STATUSES, path))
    findings.extend(_check_enum(data, "severity", SEVERITY_VALUES, path))
    _, tpr_findings = _validate_tpr_info(data.get("third_party_review"), path)
    findings.extend(tpr_findings)
    return findings


def _validate_completed_index(data: dict, path: Path) -> list[Finding]:
    findings = []
    findings.extend(_check_required(data, _schema_required_fields(CompletedIndexSchema), path))
    findings.extend(_check_unknown_fields(data, _schema_allowed_fields(CompletedIndexSchema), path))
    findings.extend(_check_enum(data, "status", COMPLETED_STATUSES, path))
    return findings


# ---------------------------------------------------------------------------
# §06.2 — `## Intelligence Reconnaissance` body-level validator
# ---------------------------------------------------------------------------
#
# Detection rules (from `plans/query-intel-adoption/section-06-plan-schema-recon.md`
# §06.2 implementation item 6):
#
# * **Missing block** — no `## Intelligence Reconnaissance` header in body.
#   status=not-started → Severity.HIGH + Outcome.WARNING (ERROR under --strict-recon).
#   status=in-progress → Severity.MEDIUM + Outcome.WARNING (unaffected by --strict-recon).
#   status=complete → exempt (0 findings).
#
# * **Graph-unavailable documentation** (header + ISO date + an unavailability
#   phrase) — Severity.LOW + Outcome.WARNING, distinct `RECON_GRAPH_UNAVAILABLE`
#   subtype. PASSES the anti-stub check (not a performative ritual — author
#   ran the availability check and recorded the result).
#
# * **Stub / performative-ritual block** (header present but fails concrete-
#   content checks) — `VALIDATION_BYPASS` subtype, same severity/outcome
#   mapping as missing-block. Triggers on: empty/whitespace, placeholder-
#   only, missing query line, missing date, missing citation, citation
#   marker followed by placeholder within 20 chars ("mixed-placeholder-
#   after-citation"), or a bare `@`-include without a summary paragraph.
#
# Format-coupling contract (§03/§06/§07): ≤500-char bound +
# `[ori]` / `[repo#N]` / `[repo:path]` citation vocabulary +
# `.claude/skills/query-intel/compose-intel-summary.md` SSOT. Exact line-level
# formatting may vary per consumer rendering context; anti-stub detection
# enforces the citation-grammar half.

_RECON_HEADER_RE = re.compile(
    r"^## Intelligence Reconnaissance\s*$", re.MULTILINE,
)
_NEXT_SECTION_RE = re.compile(r"^## ", re.MULTILINE)
_HTML_COMMENT_RE = re.compile(r"<!--.*?-->", re.DOTALL)

_RECON_DATE_RE = re.compile(r"\b\d{4}-\d{2}-\d{2}\b")
_RECON_QUERY_RE = re.compile(r"\bscripts/intel-query\.sh\b")
_RECON_ORI_CITATION_RE = re.compile(r"\[ori\]")
# Step D grammar (compose-intel-summary.md):
#   `[repo#N]` — issue/PR shorthand, N is a POSITIVE INTEGER
#   `[repo:path]` — symbol path shorthand, path is non-empty
# Split the two forms so the validator rejects malformed near-misses
# (e.g. `[rust#abc]` is not a valid issue citation; `[rust:]` is not a
# valid symbol citation). Repo name is `[a-z][a-z0-9-]*` — matches any
# lowercase repo identifier without hardcoding the repo list.
_RECON_REPO_ISSUE_CITATION_RE = re.compile(r"\[[a-z][a-z0-9-]*#\d+\]")
_RECON_REPO_PATH_CITATION_RE = re.compile(r"\[[a-z][a-z0-9-]*:[^\]\s][^\]]*\]")
_RECON_AT_INCLUDE_RE = re.compile(
    r"@\.claude/skills/query-intel/compose-intel-summary\.md",
)

# Placeholder tokens (case-insensitive, whole-token). Used for the
# "only-placeholders" whole-body check and the mixed-placeholder-after-
# citation scan — BUT the two uses have different token lists.
#
# `_RECON_WHOLE_BODY_PLACEHOLDERS` — ALL placeholder tokens, including
# natural-English words like `none` / `n/a`. When a block's ENTIRE body
# consists of these tokens (after strip), it is a performative stub
# regardless of which word the author used.
#
# `_RECON_MIXED_PLACEHOLDERS` — STRICT stub tokens only. Excludes `NONE`
# and `N/A` because those words appear in natural prose (e.g.
# `[ori] None of the callers of validate live outside the package.`
# is a valid summary). Including them in the mixed-placeholder scan
# would false-positive legitimate summaries that happen to start with
# "None" — a high-severity correctness bug (TPR-06-001-codex).
_RECON_WHOLE_BODY_PLACEHOLDER_PATTERN = (
    r"\bTBD\b|\bTODO\b|\bNONE\b|\bN/?A\b|\bXXX\b|\(empty\)|\.{3}|…"
)
_RECON_MIXED_PLACEHOLDER_PATTERN = (
    r"\bTBD\b|\bTODO\b|\bXXX\b|\(empty\)|\.{3}|…"
)
_RECON_PLACEHOLDER_RE = re.compile(
    _RECON_WHOLE_BODY_PLACEHOLDER_PATTERN, re.IGNORECASE,
)
# `[ori]` OR `[repo#` / `[repo:` — start of any citation marker, followed
# by optional punctuation/whitespace, then a STRICT stub token. The tight
# `[^\n]{0,8}?` gap ensures only `[ori] TBD` / `[ori]: TODO` / `[repo#N] …`
# shapes match; sentences like `[ori] Summary of what we found.` do not.
_RECON_MIXED_CITATION_RE = re.compile(
    r"(\[ori\]|\[[a-z][a-z0-9-]*[#:])"
    r"[\s:;—–\-]{0,4}"
    r"(?:" + _RECON_MIXED_PLACEHOLDER_PATTERN + r")",
    re.IGNORECASE,
)

_RECON_GRAPH_UNAVAIL_PHRASES = (
    "graph was unavailable",
    "intelligence graph unavailable",
    "graph unavailable",
)

_RECON_SSOT_HINT = (
    "See '.claude/skills/create-plan/plan-schema.md' Section File Template and "
    "'.claude/skills/query-intel/compose-intel-summary.md' Step D for the query "
    "protocol and summary format ([ori] / [repo#N] / [repo:path] citations, "
    "≤500 chars). If the graph is unavailable, record the unavailability as "
    "freeform prose with an ISO date — do NOT omit the block."
)


def _extract_recon_block(body: str) -> str | None:
    """Extract the `## Intelligence Reconnaissance` block body.

    Returns None if the header is absent. Otherwise returns the text from
    the line AFTER the header to the next `## ` section header or EOF,
    with HTML comments stripped (they are metadata, not content).
    """
    header = _RECON_HEADER_RE.search(body)
    if not header:
        return None
    start = header.end()
    next_section = _NEXT_SECTION_RE.search(body, start)
    end = next_section.start() if next_section else len(body)
    block = body[start:end]
    return _HTML_COMMENT_RE.sub("", block)


def _check_intel_recon_block(
    data: dict, body: str, path: Path, strict_recon: bool = False,
) -> list[Finding]:
    """Body-level validator for PLAN_SECTION recon-block mandate.

    See the `§06.2` header comment for the full detection rules. Returns
    a list with AT MOST ONE finding per invocation — the first failing
    check wins, so consumers see the primary violation rather than a
    cascade.
    """
    status = data.get("status", "")
    # `complete` sections are totally exempt — the retrofit / enforcement
    # surface is `not-started` and `in-progress` only.
    if status == "complete":
        return []
    # Bug-plan sections are exempt — the bug-plan workflow has its own
    # intel-graph step at /fix-bug Phase 1.5a (populated when §01 status
    # moves past not-started). The validator does not duplicate that gate.
    if _is_bug_plan_path(path):
        return []

    # Severity + Outcome for missing/stub findings, gated on status.
    if status == "in-progress":
        base_sev = Severity.MEDIUM
        base_outcome = Outcome.WARNING  # --strict-recon does NOT escalate in-progress
    else:
        # not-started (and any unknown status — treat as not-started
        # defensively; the schema validator will emit its own ENUM_OUT_OF_RANGE).
        base_sev = Severity.HIGH
        base_outcome = Outcome.ERROR if strict_recon else Outcome.WARNING

    block = _extract_recon_block(body)
    if block is None:
        return [Finding(
            category=FindingCategory.GAP,
            subtype=FindingSubtype.MISSING_RECON_BLOCK,
            severity=base_sev,
            source=path,
            description=(
                "Section lacks a '## Intelligence Reconnaissance' block. "
                "Every PLAN_SECTION author must record graph queries run + "
                "≤500-char summary + ISO date before implementation."
            ),
            recommended_fix=_RECON_SSOT_HINT,
            outcome=base_outcome,
        )]

    stripped = block.strip()
    has_date = bool(_RECON_DATE_RE.search(block))
    lower = block.lower()

    # Graph-unavailable takes precedence over stub checks, but ONLY when the
    # block is genuinely substituting for full recon — not when the phrase
    # happens to appear inside an otherwise complete block. Requires:
    #   1. A date marker (so authors can't pass with a bare "unavailable" note)
    #   2. One of the unavailability phrases
    #   3. NO concrete query line (block is NOT running real queries)
    #
    # Rule 3 is what distinguishes a dedicated graph-unavailable note from
    # a complete block that mentions the phrase in prose. Without it, a
    # valid recon block containing e.g. "This section discusses how graph
    # unavailable notes should be handled [ori]." would be misclassified
    # as graph-unavailable, shadowing its real completeness (TPR-06-003-codex).
    if (
        has_date
        and any(p in lower for p in _RECON_GRAPH_UNAVAIL_PHRASES)
        and not _RECON_QUERY_RE.search(block)
    ):
        return [Finding(
            category=FindingCategory.GAP,
            subtype=FindingSubtype.RECON_GRAPH_UNAVAILABLE,
            severity=Severity.LOW,
            source=path,
            description=(
                "Section records graph-unavailable state. This is a distinct, "
                "non-gating finding — the author ran the availability check "
                "and recorded that the graph was down. Fill in full queries "
                "if/when the graph becomes available."
            ),
            recommended_fix=(
                "When `scripts/intel-query.sh status` returns ok, run the "
                "canonical recon queries and update this block with the "
                "results summary and [ori] / [repo#N] citations."
            ),
            outcome=Outcome.WARNING,
        )]

    # Empty / whitespace-only after the header.
    if not stripped:
        return [Finding(
            category=FindingCategory.GAP,
            subtype=FindingSubtype.VALIDATION_BYPASS,
            severity=base_sev,
            source=path,
            description=(
                "'## Intelligence Reconnaissance' header present but body is "
                "empty (performative-ritual stub)."
            ),
            recommended_fix=_RECON_SSOT_HINT,
            outcome=base_outcome,
        )]

    # Only-placeholder tokens: strip all placeholder tokens + whitespace;
    # if nothing remains, the body is a ritual stub.
    placeholder_free = _RECON_PLACEHOLDER_RE.sub("", stripped).strip()
    if not placeholder_free:
        return [Finding(
            category=FindingCategory.GAP,
            subtype=FindingSubtype.VALIDATION_BYPASS,
            severity=base_sev,
            source=path,
            description=(
                "'## Intelligence Reconnaissance' block contains only "
                "placeholder tokens (TBD / TODO / n/a / none / (empty) / ...). "
                "This is a performative-ritual stub — placeholder tokens do "
                "not record real reconnaissance."
            ),
            recommended_fix=_RECON_SSOT_HINT,
            outcome=base_outcome,
        )]

    # Bare `@`-include with no surrounding summary. The @-include is a
    # SOURCE for Claude's prompt, NOT a substitute for a plan-resident
    # snapshot. Strip the include text and see if anything substantive
    # remains.
    if _RECON_AT_INCLUDE_RE.search(block):
        without_at = _RECON_AT_INCLUDE_RE.sub("", stripped).strip()
        if not without_at:
            return [Finding(
                category=FindingCategory.GAP,
                subtype=FindingSubtype.VALIDATION_BYPASS,
                severity=base_sev,
                source=path,
                description=(
                    "'## Intelligence Reconnaissance' block contains only the "
                    "literal `@.claude/skills/query-intel/compose-intel-summary.md` "
                    "SSOT reference. The @-include is a SOURCE for Claude's "
                    "prompt — NOT a substitute for the plan-resident snapshot."
                ),
                recommended_fix=(
                    "Add a condensed ≤500-char summary paragraph alongside the "
                    "@-include, with the literal scripts/intel-query.sh commands "
                    "run, an ISO YYYY-MM-DD date, and at least one "
                    "[ori] / [repo#N] / [repo:path] citation marker."
                ),
                outcome=base_outcome,
            )]

    # Concrete-content checks: query line, ISO date, citation marker. The
    # citation check accepts three Step D forms: `[ori]`, `[repo#N]` where
    # N is a positive integer (issue/PR shorthand), and `[repo:path]` where
    # path is non-empty (symbol shorthand). Malformed near-misses like
    # `[rust#abc]` or `[rust:]` do NOT satisfy the citation check — they are
    # performative-ritual stubs per the Step D grammar (TPR-06-002-codex).
    has_query = bool(_RECON_QUERY_RE.search(block))
    has_citation = bool(
        _RECON_ORI_CITATION_RE.search(block)
        or _RECON_REPO_ISSUE_CITATION_RE.search(block)
        or _RECON_REPO_PATH_CITATION_RE.search(block)
    )
    missing: list[str] = []
    if not has_query:
        missing.append("scripts/intel-query.sh command line")
    if not has_date:
        missing.append("ISO date marker (YYYY-MM-DD)")
    if not has_citation:
        missing.append("citation marker ([ori] / [repo#N] / [repo:path])")
    if missing:
        return [Finding(
            category=FindingCategory.GAP,
            subtype=FindingSubtype.VALIDATION_BYPASS,
            severity=base_sev,
            source=path,
            description=(
                f"'## Intelligence Reconnaissance' block is missing: "
                f"{', '.join(missing)}. Performative-ritual stub."
            ),
            recommended_fix=_RECON_SSOT_HINT,
            outcome=base_outcome,
        )]

    # Mixed-placeholder-after-citation — the citation marker provides false
    # cover for a TBD/TODO content line within 20 chars. Example: `[ori] TBD`.
    if _RECON_MIXED_CITATION_RE.search(block):
        return [Finding(
            category=FindingCategory.GAP,
            subtype=FindingSubtype.VALIDATION_BYPASS,
            severity=base_sev,
            source=path,
            description=(
                "'## Intelligence Reconnaissance' has a citation marker "
                "followed by a placeholder token within 20 characters "
                "(mixed-placeholder-after-citation). The citation marker "
                "provides false cover — the content is not real TBD."
            ),
            recommended_fix=(
                "Replace the placeholder after the citation marker with the "
                "actual summary substance. Example: replace `[ori] TBD` with "
                "`[ori] <what the graph query actually returned>`."
            ),
            outcome=base_outcome,
        )]

    # All checks passed — block is complete.
    return []


# ---------------------------------------------------------------------------
# FileClass metadata registry (SSOT for per-class display / pattern / schema /
# validator — queried by docgen.py, validate(), and any other consumer)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class FileClassMeta:
    """Per-FileClass metadata used by validation, docgen, and discovery.

    `display_name` and `pattern` drive human-readable documentation; `schema_cls`
    is the dataclass SSOT for fields; `validator` performs per-class semantic
    checks on parsed frontmatter; `body_validator` (optional) performs
    body-level checks that need the raw post-frontmatter text, threaded with
    the `strict_recon` enforcement flag.

    The body validator is None for classes that don't need body-level checks.
    Adding a body validator is explicit opt-in — ROADMAP_SECTION /
    BUG_TRACKER_SECTION / FIX_BUG / overview / index classes remain None so
    §06.2's recon-block mandate is confined to PLAN_SECTION.

    Rationale for shape (a) over a parallel body-check phase (shape b):
    one dispatch mechanism avoids duplicate plumbing and keeps the
    class-keyed registry that docgen relies on. See §06.2 Design Decision 5.
    """
    display_name: str
    pattern: str
    schema_cls: type
    validator: Callable[[dict, Path], list[Finding]]
    body_validator: Callable[[dict, str, Path, bool], list[Finding]] | None = None


FILE_CLASS_META: dict[FileClass, FileClassMeta] = {
    FileClass.PLAN_INDEX: FileClassMeta(
        "Plan Index", "plans/*/index.md", PlanIndexSchema, _validate_plan_index),
    FileClass.PLAN_SECTION: FileClassMeta(
        "Plan Section", "plans/*/section-*.md", PlanSectionSchema, _validate_plan_section,
        body_validator=_check_intel_recon_block),
    FileClass.ROADMAP_SECTION: FileClassMeta(
        "Roadmap Section", "plans/roadmap/section-*.md", RoadmapSectionSchema,
        _validate_roadmap_section),
    FileClass.OVERVIEW: FileClassMeta(
        "Overview", "plans/*/00-overview.md", OverviewSchema, _validate_overview),
    FileClass.BUG_TRACKER_SECTION: FileClassMeta(
        "Bug Tracker Section", "bug-tracker/section-*.md",
        BugTrackerSectionSchema, _validate_bug_section),
    FileClass.FIX_BUG: FileClassMeta(
        "Fix Bug", "bug-tracker/fix-BUG-*.md", FixBugSchema, _validate_fix_bug),
    FileClass.COMPLETED_INDEX: FileClassMeta(
        "Completed Index", "plans/completed/*/index.md", CompletedIndexSchema,
        _validate_completed_index),
    FileClass.AUDIT_SECTION: FileClassMeta(
        "Audit Section", "plans/*/audits/section-*.md", AuditSchema,
        _validate_audit_section),
}


def validate(
    file_class: FileClass,
    data: dict,
    path: Path,
    *,
    body: str = "",
    strict_recon: bool = False,
) -> list[Finding]:
    """Validate parsed frontmatter AND body text against the file-class schema.

    Runs the frontmatter `validator` first, then (if present) the
    `body_validator`, concatenating findings in that order. `body` and
    `strict_recon` are keyword-only with defaults — the 3-positional form
    `validate(fc, data, path)` remains the stable public contract for
    frontmatter-only callers (tests, ad-hoc invocations). The body-aware
    path is opt-in via keyword.

    Callers: `discovery.load_and_validate` — the single production call
    site — passes both keywords. Frontmatter-only tests skip them.
    """
    meta = FILE_CLASS_META.get(file_class)
    if meta is None:
        return []
    findings = list(meta.validator(data, path))
    if meta.body_validator is not None:
        findings.extend(meta.body_validator(data, body, path, strict_recon))
    return findings
