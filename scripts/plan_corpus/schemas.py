"""Dataclass SSOTs for the seven plan file schemas.

Each `@dataclass(frozen=True)` class is the sole source of truth for the
required / allowed field sets of one file class. Required fields have no
default; optional fields default to `None`.

Consumers (`schema.py`, `docgen.py`) derive `required` / `allowed` from
these dataclasses via `dataclasses.fields()` — do NOT maintain parallel
allowlist constants.

Status enum frozensets (`PLAN_STATUSES`, `SECTION_STATUSES`, …) are also
homed here: they are schema constraints — the allowed values for enum
fields — so they belong with the dataclass shapes they constrain.
"""

from __future__ import annotations

import dataclasses
from dataclasses import dataclass


# ---------------------------------------------------------------------------
# Status enums (corpus-derived)
# ---------------------------------------------------------------------------

PLAN_STATUSES = frozenset({"active", "queued", "resolved", "not-started", "research"})
# `partially-started` distinguishes "some subsections complete, others not-started" from
# in-progress (active work right now). `superseded` marks work absorbed into other sections.
SECTION_STATUSES = frozenset({"not-started", "in-progress", "complete", "superseded", "partially-started"})
OVERVIEW_STATUSES = frozenset({"not-started", "in-progress", "research", "complete", "superseded", "partially-started"})
FIX_STATUSES = frozenset({"not-started", "in-progress", "complete"})
TPR_STATUSES = frozenset({"none", "findings", "resolved", "clean"})
SEVERITY_VALUES = frozenset({"critical", "high", "medium", "low"})
COMPLETED_STATUSES = frozenset({"resolved"})


# ---------------------------------------------------------------------------
# Top-level file-class schemas
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class PlanIndexSchema:
    """Schema for `plans/*/index.md`."""
    name: str
    full_name: str
    status: str
    reviewed: bool | None = None
    reroute: bool | None = None
    parallel: bool | None = None
    order: int | None = None
    supersedes: list[str] | None = None
    references: list[str] | None = None
    inspired_by: list[str] | None = None
    # Free-text justification required when `reroute: true` (Fallback-R plan
    # per CLAUDE.md §Plan Routing — Mechanical). Forbidden when reroute is
    # false/None. Conditional-required validation is enforced by schema.py
    # rather than the dataclass (which stays permissive at the type level so
    # existing plans without the field keep parsing).
    #
    # NOTE: Bug plans under `bug-tracker/plans/BUG-XX-NNN/` MUST NOT declare
    # `reroute:` or `routing_justification:`. The directory location IS the
    # routing decision (categorically Route B / Fallback R per CLAUDE.md
    # §Bug Handling). schema.py emits SCHEMA_VIOLATION if either field is
    # present on a plan under bug-tracker/plans/.
    routing_justification: str | None = None


@dataclass(frozen=True)
class PlanSectionSchema:
    """Schema for `plans/*/section-*.md` (non-roadmap plan sections)."""
    section: str
    title: str
    status: str
    reviewed: bool
    goal: str
    success_criteria: list[str]
    sections: list[dict]
    third_party_review: dict
    depends_on: list[str] | None = None
    inspired_by: list[str] | None = None
    touches: list[str] | None = None
    # Pipeline state for /review-plan resume across /clear (SKILL.md §Step 1d).
    # Written by every step (precheck/audit/blind-spots/editor/tpr); removed on
    # clean Step 7+8 exit. Allows /continue-roadmap to skip re-running expensive
    # earlier steps (Step 4 /tp-help is ~20-45 min reviewer wall-clock).
    review_pipeline: dict | None = None
    # Optional review-invalidation audit block — written by `scripts/plan-invalidate.py`
    # when `reviewed:` is flipped from true → false. Omit when `reviewed: true` OR
    # when no prior review existed (new section). Schema:
    #   review:
    #     invalidated: YYYY-MM-DD          # date of invalidation
    #     by: ordinal-invalidation         # ordinal-invalidation | footprint-invalidation | manual
    #     reason: "<short string>"         # one-line explanation (≤200 chars)
    #     prior_reviewed_at: YYYY-MM-DD    # best-effort date of the now-stale review
    #     invalidated_by: ["04a", "04b"]   # ordinal-invalidation only — upstream sections that triggered it
    # Re-review path: when user re-runs TPR + flips `reviewed: true`, plan-invalidate.py
    # deletes this block. Block presence is the audit-trail signal "this section is
    # currently in an invalidated state and needs re-review before its reviewed flag
    # can flip back".
    review: dict | None = None


@dataclass(frozen=True)
class RoadmapSectionSchema:
    """Schema for `plans/roadmap/section-*.md`."""
    section: str
    title: str
    status: str
    reviewed: bool
    goal: str
    sections: list[dict]
    # Crate → section ownership map; bug triage inverts this list at
    # `/add-bug` / `/fix-bug` time to pick the owning roadmap section. Use
    # `[]` (never omitted) when a section owns no crates — explicit empty is
    # the contract; silent absence is not allowed.
    owns_crates: list[str]
    tier: str | None = None
    last_verified: str | None = None
    spec: str | None = None
    depends_on: list[str] | None = None
    # Inverse of `depends_on` — list of section IDs that this section blocks. Useful when
    # a foundational section gates downstream work (e.g., parser changes block typeck).
    blocks: list | None = None
    third_party_review: dict | None = None
    tpr_findings: list[dict] | None = None
    verification_summary: str | None = None
    # Optional metadata fields — also valid on PlanSectionSchema; mirrored here so
    # roadmap sections can carry the same author-intent fields without forcing every
    # section to declare them.
    success_criteria: list[str] | None = None
    inspired_by: list[str] | None = None
    # Supersession metadata. `status: superseded` (SECTION_STATUSES) marks a section
    # whose work has been absorbed into other sections. `superseded_by` is the list of
    # section IDs that absorbed the work; `superseded_reason` is a one-line explanation.
    superseded_by: list | None = None
    superseded_reason: str | None = None
    # Optional review-invalidation audit block — see PlanSectionSchema.review docstring.
    # Same shape, same invalidation rules. Written by `scripts/plan-invalidate.py`.
    review: dict | None = None


@dataclass(frozen=True)
class OverviewSchema:
    """Schema for `plans/*/00-overview.md`."""
    plan: str
    title: str
    status: str
    reviewed: bool | None = None
    supersedes: list[str] | None = None
    references: list[str] | None = None


@dataclass(frozen=True)
class BugTrackerSectionSchema:
    """Schema for `bug-tracker/section-*.md`."""
    section: str
    title: str
    status: str
    goal: str
    sections: list[dict] | None = None


@dataclass(frozen=True)
class FixBugSchema:
    """Schema for `bug-tracker/fix-BUG-*.md`."""
    bug: str
    title: str
    severity: str
    status: str
    goal: str
    success_criteria: list[str]
    subsystem: str
    found: str
    source: str
    third_party_review: dict
    sections: list[dict] | None = None
    depends_on: list[str] | None = None
    touches: list[str] | None = None


@dataclass(frozen=True)
class AuditSchema:
    """Schema for `plans/*/audits/section-*.md` (top-down spec audits).

    Audit sections are a separate file class from PlanSectionSchema because their
    workflow is "walk a canonical spec source, inventory coverage" rather than
    "implement a deliverable". Required fields are minimal (section/title/reviewed
    + sections list); the audit-specific metadata (canonical_spec_sources,
    last_walked, walked_by, audit_input) is optional so audits authored before the
    full pattern landed still parse.
    """
    section: str
    title: str
    reviewed: bool
    sections: list[dict]
    # Audit-specific metadata.
    canonical_spec_sources: list[str] | None = None
    last_walked: str | None = None  # YYYY-MM-DD
    walked_by: str | None = None
    audit_input: str | None = None  # path to canonical input artifact
    # Carry-over plan-section optional fields so audits can use the same review /
    # dependency machinery as regular plan sections.
    status: str | None = None
    goal: str | None = None
    success_criteria: list[str] | None = None
    third_party_review: dict | None = None
    depends_on: list[str] | None = None
    review: dict | None = None


@dataclass(frozen=True)
class CompletedIndexSchema:
    """Schema for `plans/completed/*/index.md`."""
    name: str
    full_name: str
    status: str
    reroute: bool | None = None
    parallel: bool | None = None
    order: int | None = None


# ---------------------------------------------------------------------------
# Shape helpers
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class SubsectionEntry:
    """Schema for each entry in a section's `sections: [...]` list.

    Used by `_validate_sections` in schema.py — its required/allowed field
    sets are derived from this dataclass via the same introspection helpers
    as the top-level file schemas, so there is ONE SSOT for the shape.
    """
    id: str
    title: str
    status: str


@dataclass(frozen=True)
class TprInfo:
    """Parsed shape of a `third_party_review` block."""
    status: str
    updated: str | None


def _schema_required_fields(cls) -> list[str]:
    """Fields without defaults are required.

    Preserves declaration order, so the resulting list is stable and
    matches the order used in generated documentation.
    """
    return [
        f.name for f in dataclasses.fields(cls)
        if f.default is dataclasses.MISSING
        and f.default_factory is dataclasses.MISSING
    ]


def _schema_allowed_fields(cls) -> frozenset[str]:
    """All dataclass field names are allowed frontmatter keys."""
    return frozenset(f.name for f in dataclasses.fields(cls))
