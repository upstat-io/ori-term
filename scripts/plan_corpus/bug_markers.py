"""bug_markers.py — SSOT for bug-tracker entry parsing and lifecycle classification.

This module is the canonical source for:
- Bug-entry header regex and parser (`parse_bug_entries`)
- Lifecycle marker patterns and classifier (`classify_bug_exclusion`)
- Body-field extractors (`extract_repro`, `extract_subsystem`)
- Severity normalizer (`normalize_severity`)

Consumers:
    - .claude/skills/continue-roadmap/roadmap_scan.py  (Gate 1.92, Gate 1.6)
    - .claude/skills/fix-next-bug/bug_queue_scan.py    (queue priority scan)

Extending the marker vocabulary:
    Edit ONLY this file. Both consumers pick up changes automatically via import.
    Never duplicate regex patterns in the consumer scripts — that is a
    `LEAK:algorithmic-duplication` per impl-hygiene.md §SSOT.

Lifecycle marker precedence (first match wins):
    Superseded  >  Escalated  >  Blocked  >  blocked-by
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Iterator


# ---------------------------------------------------------------------------
# Regex: bug-entry header line
# ---------------------------------------------------------------------------

# Matches the checkbox header line of a bug entry. Captures:
#   checked   : " " or "x" / "X"
#   section   : numeric section code ("04")
#   ordinal   : numeric ordinal ("074")
#   severity  : raw severity token (may include Unicode arrow "→")
#   title     : title text between the first `**` and the matching `**`
#
# We deliberately allow arbitrary trailing content after the closing
# `**` — entries frequently have attribution like `— found by tpr-review`
# following the bold title, and dropping those bugs (as an over-strict
# `\s*$` anchor did previously) would silently hide high-severity work.
BUG_HEADER_RE = re.compile(
    r"^- \[(?P<checked>[ xX])\] `\[BUG-(?P<section>\d+)-(?P<ordinal>\d+)\]"
    r"\[(?P<severity>[^\]]+)\]` \*\*(?P<title>.+?)\*\*"
)

# ---------------------------------------------------------------------------
# Regex: lifecycle marker patterns
# ---------------------------------------------------------------------------

# "Superseded by:" — bug is being fixed by a multi-section plan.
# Route via `/continue-roadmap`, NOT `/fix-bug`.
# The plan's 00-overview.md `supersedes:` frontmatter MUST point back.
# Matches: "Superseded by: plans/foo/" or "  Superseded by: plans/foo/"
SUPERSEDED_RE = re.compile(r"(?im)^\s*superseded\s+by\s*:")

# "Escalated to plan:" / "Escalated:" — waiting on plan creation.
ESCALATED_RE = re.compile(r"(?im)^\s*escalated(?:\s+to\s+plan)?\s*:")

# "Blocked:" / "**Blocked**:" / "**Blocked:**" — waiting on a dependency.
# NOTE: `**BLOCKER**:` (informational impact text) is NOT a lifecycle marker.
# Only `**Blocked**:` (with trailing colon, followed by reason text) is a
# lifecycle marker. The distinction is load-bearing — see fix-bug-design.md §4.
# Pattern anchors to line start and requires the word to be "Blocked" (past
# participle), not "BLOCKER" (noun/adjective). The negative lookahead on [R]
# ensures "BLOCKER" is excluded.
BLOCKED_RE = re.compile(r"(?im)^\s*\*{0,2}blocked(?!er)\*{0,2}\s*:")

# `<!-- blocked-by: ... -->` — cross-section blocker comment tag.
BLOCKED_BY_COMMENT_RE = re.compile(r"<!--\s*blocked-by:", re.IGNORECASE)

# ---------------------------------------------------------------------------
# Data model
# ---------------------------------------------------------------------------


@dataclass
class BugEntry:
    """A single bug-tracker entry parsed from a section-NN-*.md file.

    The `excluded_reason` field mirrors `bug_queue_scan.py`'s field of the
    same name: `None` = actionable; any string = excluded for that reason.

    Consumers that need only a subset of these fields (e.g., the roadmap
    scanner's local `Bug` shape) should cherry-pick from here rather than
    duplicating parsing logic.
    """
    bug_id: str           # "BUG-04-045"
    section: int          # 4
    ordinal: int          # 45
    severity: str         # normalized: critical | high | medium | low | unknown
    severity_raw: str     # raw token from the entry header
    status: str           # "open" | "fixed"
    title: str            # display title (stripped of `**` markers)
    lineno: int           # 1-based line number of the header in the source file
    source_file: str      # filename only, e.g. "section-04-fonts.md"
    body_lines: list[str] = field(default_factory=list)
    repro: str | None = None
    subsystem: str | None = None
    excluded_reason: str | None = None  # None = actionable


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def normalize_severity(raw: str) -> str:
    """Normalize a raw severity token to one of: critical high medium low.

    Handles reclassification notation like `critical→medium` (use the
    reclassified current severity for priority purposes) and trailing
    annotations like `critical (reclassified)`.
    """
    s = raw.strip().lower()
    if "→" in s:
        s = s.split("→")[-1].strip()
    # Strip trailing annotations like " (reclassified)" or " elevated"
    s = re.split(r"[\s(]", s, maxsplit=1)[0]
    return s


def classify_bug_exclusion(body_text: str) -> str | None:
    """Classify a bug entry's lifecycle exclusion reason from its body text.

    Returns a human-readable exclusion reason string, or `None` if the bug
    is actionable (should be routed to `/fix-bug`).

    Precedence (first match wins): Superseded > Escalated > Blocked > blocked-by.
    This ordering is LOAD-BEARING — see fix-bug-design.md §1 Invariant I2.
    """
    if SUPERSEDED_RE.search(body_text):
        return "Superseded by plan"
    if ESCALATED_RE.search(body_text):
        return "Escalated to plan"
    if BLOCKED_RE.search(body_text):
        return "Blocked"
    if BLOCKED_BY_COMMENT_RE.search(body_text):
        return "Blocked (cross-section blocker tag)"
    return None


def extract_repro(body_lines: list[str]) -> str | None:
    """Extract the first `Repro:` field value from body lines."""
    for line in body_lines:
        s = line.strip()
        if s.lower().startswith("repro:"):
            return s.split(":", 1)[1].strip() or None
    return None


def extract_subsystem(body_lines: list[str]) -> str | None:
    """Extract the first `Subsystem:` field value from body lines."""
    for line in body_lines:
        s = line.strip()
        if s.lower().startswith("subsystem:"):
            return s.split(":", 1)[1].strip() or None
    return None


def parse_bug_entries(text: str, source_file: str) -> Iterator[BugEntry]:
    """Parse all bug entries (open + fixed) from one section-NN-*.md file.

    Yields `BugEntry` objects in document order.  Callers that only want
    open bugs must filter: `entry.status == "open"`.

    Body collection: indented/continuation lines from the line after the
    header until a blank line or the next `- [` header.  This is the same
    collection rule as `bug_queue_scan.py::parse_section_file`.
    """
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        m = BUG_HEADER_RE.match(line)
        if not m:
            i += 1
            continue

        checked = m.group("checked").strip().lower() == "x"
        section = int(m.group("section"))
        ordinal = int(m.group("ordinal"))
        severity_raw = m.group("severity")
        title = m.group("title")
        lineno = i + 1  # 1-based

        # Collect body lines
        body_lines: list[str] = []
        j = i + 1
        while j < len(lines):
            bl = lines[j]
            if bl.strip() == "":
                break
            if bl.startswith("- ["):
                break
            body_lines.append(bl)
            j += 1

        body_text = "\n".join(body_lines)

        status = "fixed" if checked else "open"
        severity = normalize_severity(severity_raw)
        excluded_reason = classify_bug_exclusion(body_text) if not checked else None
        repro = extract_repro(body_lines)
        subsystem = extract_subsystem(body_lines)

        yield BugEntry(
            bug_id=f"BUG-{section:02d}-{ordinal:03d}",
            section=section,
            ordinal=ordinal,
            severity=severity,
            severity_raw=severity_raw,
            status=status,
            title=title,
            lineno=lineno,
            source_file=source_file,
            body_lines=body_lines,
            repro=repro,
            subsystem=subsystem,
            excluded_reason=excluded_reason,
        )
        i = j
