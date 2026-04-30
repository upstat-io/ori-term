"""Plan-coherence lint.

Detects cross-artifact drift between the prose claims of a plan
(`00-overview.md`, `index.md`, dependency-graph + mission-success-criteria
blocks) and the executable section files those claims describe.

Distinct from `schema.py` / `dag.py` which validate YAML-derived structure
(`depends_on:` lists, `section:` numbers in frontmatter). Coherence covers
BODY-prose references the schema validator does not see — e.g. an overview
paragraph that says "§67 — history_sim" while the actual `section-67-*.md`
file's title/goal still describes the older `procgen` work, or a mission
success criterion citing "§73" with no `section-73-*.md` file present.

The module is pure: `lint_plan_dir(plan_dir, target_section=None)` returns
a `list[Finding]`. The `coherence` CLI subcommand in `__main__.py` is the
single consumer; `tpr-review/SKILL.md §3.5` invokes that CLI for plan-mode
reviews.

Three reference channels are extracted and verified:

1. Body-prose section references — `§NN`, `section-NN`, "section NN" tokens
   anywhere in overview / index / section bodies. Each ref must resolve
   to a `section-NN-*.md` file in the plan directory. Missing → finding.

2. Overview-label drift — when an overview/index reference carries a
   directly-adjacent identifier-shaped label (`§67 history_sim`,
   `§67 — history_sim`, `§67 (history_sim)`), compare that label against
   the section file's frontmatter `title:` / `goal:` and body H1.
   Materially-disjoint label → finding.

3. Frontmatter `section:` ↔ filename mismatch — `section-67-foo.md` whose
   YAML frontmatter declares `section: 73` is silent renumber drift.

The mission-success and dependency-graph block detectors are heading-anchored
(`## Mission Success Criteria` / `## Dependency Graph` / case-insensitive
variants) and emit `MS_CRITERION_NO_SECTION` / `DEPGRAPH_DEAD_EDGE` when
their references fail existence checks — same machinery as channel 1, with
the distinct subtypes preserving the structural-source signal so downstream
consumers can prioritize differently.
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable

from .parser import read_text_strict, split_frontmatter_strict
from .types import (
    CorpusParseError,
    Finding,
    FindingCategory,
    FindingSubtype,
    Outcome,
    Severity,
)


def _warn_parse_error(path: Path, exc: CorpusParseError) -> None:
    """Emit a stderr warning when the lint can't parse a plan file.

    Coherence is a partial-success-tolerant lint — a parse error in
    `00-overview.md` should not block linting of section files that DO
    parse. But silently returning `[]` hides degraded operation from
    operators (the `coherence` CLI exits 0 with a clean report when in
    fact the lint couldn't read the file). This warning makes the
    degradation visible without changing the exit code semantics.
    Schema-level parse errors are caught + filed as `PARSE_ERROR`
    findings by `python -m scripts.plan_corpus check` — that pipeline is
    the authoritative parse-error reporter; this warning is the lint's
    operator-facing degradation signal.
    """
    print(
        f"plan_corpus.coherence: WARNING: skipping {path} — {exc}",
        file=sys.stderr,
    )


# ────────────────────────────────────────────────────────────────────────────
# Reference extraction
# ────────────────────────────────────────────────────────────────────────────

# `§67`, `§ 67`, `§67.4` — allow a single optional `.subsection` suffix.
# Capture (number, optional dotted-suffix). Subsection refs (`§67.4`) still
# map to the parent section file `section-67-*.md` for existence checks.
_SECTION_SIGIL_RE = re.compile(r"§\s*(\d+)(?:\.(\d+))?")

# `section-67`, `section-67-foo` — filename-shaped reference. The trailing
# `-foo` slug is optional in prose. The negative lookbehind `(?<![/\w])`
# rejects the token when it appears inside a path (`plans/<name>/section-67-…`)
# — those are cross-plan references and don't belong to THIS plan's
# section-numbering. Without this guard, every `depends_on:` list entry
# pointing at another plan's section would fire a missing-section finding.
_SECTION_FILENAME_RE = re.compile(
    r"(?<![/\w])section-(\d+)(?:-[a-z0-9_-]*)?\b",
    re.IGNORECASE,
)

# `Section 67`, `section 67`, `Sect 67` — prose form. Bounded with word
# boundaries to avoid matching `intersection 67` etc.
_SECTION_PROSE_RE = re.compile(r"\b[Ss]ection\s+(\d+)\b")

# Identifier-shaped label that may follow a section reference. Strict by
# design on three axes:
#
# 1. A label MUST be wrapped in EXPLICIT delimiters: backticks
#    (`§67 \`history_sim\``), parens (`§67 (history_sim)`), or straight
#    double-quotes (`§67 "history_sim"`). The bare form (`§67 history_sim`
#    without delimiters) is REJECTED — it false-positives heavily on
#    connective prose where the em-dash separates clauses rather than
#    delimiting a label (e.g. `§03 is independent of §01/§02 — take-project
#    hardening doesn't touch enum layout`). Authors who want a labeled
#    section reference can adopt the explicit `§NN \`label\`` form, which
#    is also the conventional markdown shape.
#
# 2. The label content itself MUST be snake_case (contains `_`) or
#    kebab-case (contains `-`) — single-word delimited tokens like
#    `(harness)` are still ambiguous and skipped.
#
# 3. The reference MUST be a parent section (`§NN`), NOT a subsection
#    (`§NN.M`). The negative lookahead `(?!\.\d)` rejects subsection
#    refs because the adjacent label belongs to the subsection, not the
#    parent. Without this guard, `§5.5 tie-breaker` would wrongly attach
#    `tie_breaker` as a label of section 5.
_LABEL_AFTER_REF_RE = re.compile(
    r"§\s*(\d+)(?!\.\d)[\s:\-—–]+"
    r"(?:"
    r"`([a-z][a-z0-9_-]*[_-][a-z0-9_-]*)`"          # backticked snake/kebab
    r"|\(([a-z][a-z0-9_-]*[_-][a-z0-9_-]*)\)"        # parenthesized snake/kebab
    r"|\"([a-z][a-z0-9_-]*[_-][a-z0-9_-]*)\""        # double-quoted snake/kebab
    r")",
    re.IGNORECASE,
)

# Heading detectors (case-insensitive, allow trailing punctuation).
_MS_HEADING_RE = re.compile(
    r"^\s{0,3}#{1,6}\s+(?:mission\s+success(?:\s+criteria)?|success\s+criteria)\b",
    re.IGNORECASE | re.MULTILINE,
)
_DEPGRAPH_HEADING_RE = re.compile(
    r"^\s{0,3}#{1,6}\s+(?:dependency\s+graph|dependencies|implementation\s+sequence)\b",
    re.IGNORECASE | re.MULTILINE,
)
_NEXT_HEADING_RE = re.compile(r"^\s{0,3}#{1,6}\s+\S", re.MULTILINE)

# Plan section filename: `section-NN-...md` or `section-NN.md`. Captures NN.
_SECTION_FILENAME_PATTERN = re.compile(r"^section-(\d+)(?:-[a-z0-9_-]*)?\.md$")

# Plan-overview filename forms accepted: `00-overview.md` (canonical),
# `overview.md`, `00_overview.md` (legacy lenient match).
_OVERVIEW_FILENAMES = ("00-overview.md", "overview.md", "00_overview.md")
_INDEX_FILENAMES = ("index.md",)


@dataclass(frozen=True)
class _RefHit:
    """A single section-reference token extracted from a plan body."""

    section_num: int
    line: int
    column: int
    snippet: str  # ≤ 80 char window around the hit, for evidence
    kind: str = "sigil"  # one of: "sigil" (§NN), "filename" (section-NN-slug), "prose" (Section NN)


def _extract_section_refs(text: str) -> list[_RefHit]:
    """Extract every section-reference token from a plan body.

    Three channels are merged: `§NN` sigil, `section-NN[-slug]` filename,
    `[Ss]ection NN` prose. Each hit carries `kind` ∈ {"sigil", "filename",
    "prose"} so the existence-check can apply the right policy: filename
    hits are unambiguous plan-section refs (always flagged); sigil/prose
    hits in open prose require an adjacent strict label OR membership in
    a structured block to avoid mis-flagging external-artifact refs.
    """
    hits: list[_RefHit] = []
    for line_no, line in enumerate(text.split("\n"), start=1):
        for m in _SECTION_SIGIL_RE.finditer(line):
            hits.append(_make_hit(int(m.group(1)), line_no, m.start(), line, "sigil"))
        for m in _SECTION_FILENAME_RE.finditer(line):
            hits.append(_make_hit(int(m.group(1)), line_no, m.start(), line, "filename"))
        for m in _SECTION_PROSE_RE.finditer(line):
            hits.append(_make_hit(int(m.group(1)), line_no, m.start(), line, "prose"))
    return hits


def _make_hit(num: int, line: int, col: int, full_line: str, kind: str) -> _RefHit:
    snippet = full_line.strip()
    if len(snippet) > 80:
        # Center the window roughly on the column.
        start = max(0, col - 30)
        snippet = ("…" if start > 0 else "") + snippet[start : start + 80]
    return _RefHit(
        section_num=num, line=line, column=col + 1, snippet=snippet, kind=kind
    )


def _extract_labels(text: str) -> dict[int, set[str]]:
    """Collect identifier-shaped labels adjacent to each section ref.

    Returns `{section_num: {label1, label2, ...}}`. A section ref with no
    adjacent identifier-shaped label produces no entry — label-drift checks
    skip those refs (they are existence-only).

    The regex has three label-form alternatives — exactly one capture group
    fires per match. Read whichever is non-None.
    """
    labels: dict[int, set[str]] = {}
    for num, label, _line in _extract_labels_with_lines(text):
        labels.setdefault(num, set()).add(label)
    return labels


def _extract_labels_with_lines(
    text: str,
) -> list[tuple[int, str, int]]:
    """Same as `_extract_labels` but returns `(section_num, label, line_no)`
    tuples for accurate line attribution in label-drift findings.

    The line number is computed from the match's start offset so the
    drift finding points at the actual prose where the label was claimed,
    not at line 1.
    """
    out: list[tuple[int, str, int]] = []
    for m in _LABEL_AFTER_REF_RE.finditer(text):
        num = int(m.group(1))
        label_raw = m.group(2) or m.group(3) or m.group(4)
        if not label_raw:
            continue
        label = label_raw.lower().replace("-", "_")
        line_no = text.count("\n", 0, m.start()) + 1
        out.append((num, label, line_no))
    return out


def _refs_with_strict_labels(text: str) -> set[int]:
    """Section numbers that appear at least once with a strict identifier
    label adjacent. These are the refs the existence-check is willing to
    flag as plan-section refs in open prose; bare `§NN` prose with no
    label is skipped to avoid mis-flagging external-artifact references
    (e.g. `§5 round loop` discussing tpr-review/SKILL.md §5)."""
    return set(_extract_labels(text).keys())


@dataclass(frozen=True)
class _BlockRange:
    """A heading-anchored block region within a prose file.

    Single source of truth for "where do MS-criteria / dep-graph / other
    heading-anchored blocks live in this text" — used by both
    `_check_prose_file_existence` (skip-ranges to avoid double-counting
    refs that the more-specific block emitters will catch) and the MS /
    dep-graph orphan emitters (so they can attribute findings to the
    correct line). Replaces the previous `_extract_block_after_heading`
    helper (returned a 2-tuple recomputed in two places) — the new shape
    carries `start_offset` + `end_offset` + `base_line` precomputed once
    and passed down to all consumers.
    """

    name: str  # "ms" | "depgraph" — debug label only
    start_offset: int  # byte offset in `text` where block body begins
    end_offset: int  # byte offset where block body ends (exclusive)
    base_line: int  # 1-indexed line number of `start_offset` in `text`


def _find_block_ranges(text: str) -> list[_BlockRange]:
    """Locate every heading-anchored block (MS-criteria, dep-graph) in `text`.

    Returns the blocks in declaration order. Each block carries enough
    precomputed metadata for downstream callers to slice its content,
    skip its byte-range, OR attribute findings to the correct file line
    — without any caller re-running the heading regex or re-finding the
    block text inside the file.
    """
    out: list[_BlockRange] = []
    for name, heading_re in (("ms", _MS_HEADING_RE), ("depgraph", _DEPGRAPH_HEADING_RE)):
        # finditer (not search) so plans with two "## Mission Success Criteria"
        # blocks (or two "## Implementation Sequence" blocks — unusual but
        # possible after reformatting) get every block scanned, not just the
        # first. The previous `search`-only implementation silently skipped
        # all but the first match per heading regex.
        for m in heading_re.finditer(text):
            block_start = m.end()
            next_m = _NEXT_HEADING_RE.search(text, pos=block_start)
            block_end = next_m.start() if next_m else len(text)
            base_line = text.count("\n", 0, block_start) + 1
            out.append(
                _BlockRange(
                    name=name,
                    start_offset=block_start,
                    end_offset=block_end,
                    base_line=base_line,
                )
            )
    return out


# ────────────────────────────────────────────────────────────────────────────
# Plan-directory inventory
# ────────────────────────────────────────────────────────────────────────────


@dataclass
class _PlanInventory:
    """All files in a plan directory, classified."""

    plan_dir: Path
    overview: Path | None = None
    index: Path | None = None
    sections: dict[int, Path] = field(default_factory=dict)


def _inventory_plan_dir(plan_dir: Path) -> _PlanInventory:
    """Index every plan file under `plan_dir` (non-recursive).

    Section filenames must match `section-NN[-slug].md`. Subdirectories
    (e.g. nested completed/ trees) are NOT recursed — coherence is a
    single-plan concern.
    """
    inv = _PlanInventory(plan_dir=plan_dir)
    if not plan_dir.is_dir():
        return inv
    for child in sorted(plan_dir.iterdir()):
        if not child.is_file() or child.suffix != ".md":
            continue
        name = child.name
        if name in _OVERVIEW_FILENAMES and inv.overview is None:
            inv.overview = child
            continue
        if name in _INDEX_FILENAMES and inv.index is None:
            inv.index = child
            continue
        m = _SECTION_FILENAME_PATTERN.match(name)
        if m:
            num = int(m.group(1))
            # Duplicate section numbers (`section-67-a.md` + `section-67-b.md`)
            # are a separate schema-violation concern; coherence keeps the
            # first hit and proceeds.
            inv.sections.setdefault(num, child)
    return inv


_FRONTMATTER_SECTION_LINE_RE = re.compile(r"^section\s*:", re.MULTILINE)


def _section_metadata(section_path: Path) -> dict[str, object]:
    """Pull the cheap metadata coherence cares about out of a section file.

    Returns a dict with keys: `frontmatter_section` (int|None),
    `frontmatter_section_line` (int|None — 1-indexed line of the
    `section:` key in the frontmatter, used for finding-line attribution
    on FRONTMATTER_SECTION_FILENAME_MISMATCH), `title` (str|None),
    `goal` (str|None), `body_h1` (str|None), `parse_error`
    (CorpusParseError|None). Any parse failure is captured (not raised)
    so coherence can produce a partial report on a partially-broken
    plan.
    """
    out: dict[str, object] = {
        "frontmatter_section": None,
        "frontmatter_section_line": None,
        "frontmatter_section_evidence": None,
        "title": None,
        "goal": None,
        "body_h1": None,
        "parse_error": None,
    }
    try:
        text = read_text_strict(section_path)
        fm, body_offset = split_frontmatter_strict(text, section_path)
    except CorpusParseError as exc:
        out["parse_error"] = exc
        return out

    fm_section = fm.get("section")
    if isinstance(fm_section, int):
        out["frontmatter_section"] = fm_section
    elif isinstance(fm_section, str) and fm_section.isdigit():
        out["frontmatter_section"] = int(fm_section)

    # Track the 1-indexed line of `section:` in the frontmatter so the
    # mismatch finding can anchor at the actual key location (not a
    # synthesized `:1` placeholder). Frontmatter lives at lines 1..body_offset
    # (line 1 is `---`); scan that prefix for the key.
    fm_lines = text.split("\n")[:body_offset]
    for idx, line in enumerate(fm_lines, start=1):
        if _FRONTMATTER_SECTION_LINE_RE.match(line):
            out["frontmatter_section_line"] = idx
            # Capture verbatim line text for evidence (≤120 char trim per
            # Findings grounding policy "Quote ≤3 lines of the actual
            # code verbatim as evidence").
            raw = line.strip()
            out["frontmatter_section_evidence"] = (
                raw if len(raw) <= 120 else raw[:117] + "…"
            )
            break

    title = fm.get("title")
    if isinstance(title, str):
        out["title"] = title.strip()

    goal = fm.get("goal")
    if isinstance(goal, str):
        out["goal"] = goal.strip()

    # Find the first ATX H1 in the body (lines after body_offset).
    body_lines = text.split("\n")[body_offset:]
    for line in body_lines:
        s = line.strip()
        if s.startswith("# ") and not s.startswith("## "):
            out["body_h1"] = s[2:].strip()
            break

    return out


# ────────────────────────────────────────────────────────────────────────────
# Label normalization for drift comparison
# ────────────────────────────────────────────────────────────────────────────

_LABEL_NORMALIZE_RE = re.compile(r"[^a-z0-9]+")


def _normalize_label(s: str) -> set[str]:
    """Tokenize a free-form label for drift comparison.

    `"history_sim — chronological backbone"` → `{"history", "sim",
    "chronological", "backbone"}`. The drift check asks whether at least
    one prose-extracted label token appears in the section's tokenized
    title/goal/H1. Disjoint sets → drift.
    """
    if not s:
        return set()
    parts = _LABEL_NORMALIZE_RE.split(s.lower())
    return {p for p in parts if p and len(p) > 2}


# ────────────────────────────────────────────────────────────────────────────
# Finding constructors
# ────────────────────────────────────────────────────────────────────────────


def _emit_missing_section(
    source: Path, hit: _RefHit, plan_dir: Path
) -> Finding:
    return Finding(
        category=FindingCategory.PLAN_COHERENCE,
        subtype=FindingSubtype.MISSING_REFERENCED_SECTION,
        severity=Severity.HIGH,
        outcome=Outcome.ERROR,
        source=source,
        source_line=hit.line,
        source_column=hit.column,
        description=(
            f"section §{hit.section_num} is referenced but no "
            f"`section-{hit.section_num:02d}-*.md` (or `section-{hit.section_num}-*.md`) "
            f"file exists in {plan_dir}"
        ),
        recommended_fix=(
            f"Either create the missing section file under {plan_dir}, "
            f"renumber the reference to a section that exists, or remove "
            "the stale reference."
        ),
        evidence=(hit.snippet,),
    )


def _emit_frontmatter_filename_mismatch(
    section_path: Path, declared: int, filename_num: int,
    section_line: int | None = None,
    section_evidence: str | None = None,
) -> Finding:
    evidence_tuple: tuple[str, ...] = (
        (section_evidence,) if section_evidence else ()
    )
    return Finding(
        category=FindingCategory.PLAN_COHERENCE,
        subtype=FindingSubtype.FRONTMATTER_SECTION_FILENAME_MISMATCH,
        severity=Severity.CRITICAL,
        outcome=Outcome.ERROR,
        source=section_path,
        source_line=section_line,
        description=(
            f"frontmatter declares `section: {declared}` but the filename "
            f"is `{section_path.name}` (filename → §{filename_num})"
        ),
        recommended_fix=(
            f"Either update the frontmatter to `section: {filename_num}` "
            f"or rename the file to `section-{declared:02d}-*.md`."
        ),
        evidence=evidence_tuple,
    )


def _emit_label_drift(
    source: Path,
    hit: _RefHit,
    label_in_overview: str,
    section_path: Path,
    section_label_text: str,
) -> Finding:
    return Finding(
        category=FindingCategory.PLAN_COHERENCE,
        subtype=FindingSubtype.OVERVIEW_LABEL_DRIFT,
        severity=Severity.HIGH,
        outcome=Outcome.ERROR,
        source=source,
        source_line=hit.line,
        source_column=hit.column,
        target=section_path,
        description=(
            f"§{hit.section_num} is labeled `{label_in_overview}` in "
            f"{source.name} but the section file's title/goal/H1 carries "
            f"no overlapping token (section: `{section_label_text or '(empty)'}`)"
        ),
        recommended_fix=(
            f"Either update {source.name} to reflect the section's actual "
            f"deliverable, or update {section_path.name}'s title/goal/H1 "
            "to match the overview's claim — whichever is the lagging "
            "indicator."
        ),
        evidence=(hit.snippet,),
    )


def _emit_ms_orphan(source: Path, hit: _RefHit, plan_dir: Path) -> Finding:
    return Finding(
        category=FindingCategory.PLAN_COHERENCE,
        subtype=FindingSubtype.MS_CRITERION_NO_SECTION,
        severity=Severity.CRITICAL,
        outcome=Outcome.ERROR,
        source=source,
        source_line=hit.line,
        source_column=hit.column,
        description=(
            f"mission success criterion references §{hit.section_num} "
            f"but no `section-{hit.section_num}-*.md` file exists in "
            f"{plan_dir} — the criterion has no executable home"
        ),
        recommended_fix=(
            "Either author the missing section so the criterion can be "
            "delivered, or rewrite the criterion to point at a section "
            "that exists."
        ),
        evidence=(hit.snippet,),
    )


def _emit_depgraph_dead_edge(source: Path, hit: _RefHit, plan_dir: Path) -> Finding:
    return Finding(
        category=FindingCategory.PLAN_COHERENCE,
        subtype=FindingSubtype.DEPGRAPH_DEAD_EDGE,
        severity=Severity.HIGH,
        outcome=Outcome.ERROR,
        source=source,
        source_line=hit.line,
        source_column=hit.column,
        description=(
            f"dependency graph / implementation sequence references "
            f"§{hit.section_num} but no `section-{hit.section_num}-*.md` "
            f"file exists in {plan_dir}"
        ),
        recommended_fix=(
            "Either materialize the referenced section or remove the dead "
            "edge from the graph."
        ),
        evidence=(hit.snippet,),
    )


# ────────────────────────────────────────────────────────────────────────────
# Public lint entry points
# ────────────────────────────────────────────────────────────────────────────


def lint_plan_dir(
    plan_dir: Path, target_section: Path | None = None
) -> list[Finding]:
    """Run the full coherence lint over a plan directory.

    `target_section` (optional) does NOT scope the lint — coherence is a
    plan-wide concern by construction. The argument is accepted for parity
    with `tpr-review/SKILL.md §3.5` invocation shape (the gate passes it
    so the CLI's human output can mark which section triggered the gate),
    but every coherence finding fires regardless of which section is
    under review: drift in the overview affects every downstream section.
    """
    plan_dir = plan_dir.resolve()
    if not plan_dir.is_dir():
        return []

    inv = _inventory_plan_dir(plan_dir)
    findings: list[Finding] = []
    section_nums = set(inv.sections.keys())

    # ── Per-section frontmatter↔filename check ──────────────────────────
    for filename_num, section_path in inv.sections.items():
        meta = _section_metadata(section_path)
        declared = meta.get("frontmatter_section")
        if isinstance(declared, int) and declared != filename_num:
            section_line = meta.get("frontmatter_section_line")
            section_evidence = meta.get("frontmatter_section_evidence")
            findings.append(
                _emit_frontmatter_filename_mismatch(
                    section_path, declared, filename_num,
                    section_line if isinstance(section_line, int) else None,
                    section_evidence if isinstance(section_evidence, str) else None,
                )
            )

    # ── Overview + index prose: read each file ONCE per I32 read-once
    # discipline, then run all per-file checks against the cached text +
    # precomputed block ranges. The shared `_find_block_ranges` is the SSOT
    # for block boundaries; both the existence-check (skip-ranges) and the
    # block-orphan emitters consume the same ranges with no recomputation.
    for prose_file in (inv.overview, inv.index):
        if prose_file is None:
            continue
        try:
            text = read_text_strict(prose_file)
        except CorpusParseError as exc:
            _warn_parse_error(prose_file, exc)
            continue
        blocks = _find_block_ranges(text)

        # Existence check — skip refs inside MS / dep-graph blocks.
        findings.extend(
            _check_prose_file_existence(
                prose_file, text, blocks, section_nums, plan_dir,
                emit=_emit_missing_section,
            )
        )
        # Label-drift check.
        findings.extend(
            _check_label_drift(prose_file, text, inv.sections)
        )
        # MS-criteria block orphans.
        for block in blocks:
            if block.name == "ms":
                findings.extend(
                    _emit_block_orphans(
                        prose_file, text, block, section_nums,
                        plan_dir, _emit_ms_orphan,
                    )
                )
            elif block.name == "depgraph":
                findings.extend(
                    _emit_block_orphans(
                        prose_file, text, block, section_nums,
                        plan_dir, _emit_depgraph_dead_edge,
                    )
                )

    return findings


def lint_section(section_path: Path) -> list[Finding]:
    """Convenience wrapper: resolve the plan dir from a section path."""
    section_path = section_path.resolve()
    plan_dir = section_path.parent
    return lint_plan_dir(plan_dir, target_section=section_path)


# ────────────────────────────────────────────────────────────────────────────
# Internal helpers
# ────────────────────────────────────────────────────────────────────────────


def _check_prose_file_existence(
    prose_file: Path,
    text: str,
    blocks: list[_BlockRange],
    section_nums: set[int],
    plan_dir: Path,
    *,
    emit,
) -> Iterable[Finding]:
    """Emit one MISSING_REFERENCED_SECTION per unresolved §NN in the file
    that the policy below considers a plan-section reference.

    `text` and `blocks` are caller-precomputed (single read per file per
    `lint_plan_dir`'s read-once discipline); this function does NOT
    re-read the file or re-scan for block boundaries.

    Three filters apply (in order, fail-out semantics):

    1. Skip §0 / §00 — the overview file's own number is not a section.
    2. Skip hits inside MS-criteria / dep-graph blocks (`blocks`) — those
       are handled by the more-specific MS_CRITERION_NO_SECTION /
       DEPGRAPH_DEAD_EDGE emitters (prevents double-counting).
    3. For sigil (`§NN`) and prose (`Section NN`) hits in open prose:
       require an adjacent strict label OR membership in the plan's
       existing section-number range. Filename-form hits
       (`section-NN[-slug]`) are unambiguous plan-section refs — always
       checked regardless of label adjacency.

    The label-OR-range gate is the false-positive guard: bare `§5 round
    loop` discussing `tpr-review/SKILL.md §5` would otherwise flood the
    output of every plan whose overview discusses tooling internals.
    """
    skip_ranges = [(b.start_offset, b.end_offset) for b in blocks]

    # Pre-compute line→byte-offset map once for skip-range lookups.
    line_offsets = [0]
    for line in text.split("\n"):
        line_offsets.append(line_offsets[-1] + len(line) + 1)

    def _in_skip_range(line_no: int) -> bool:
        offset = line_offsets[line_no - 1] if 0 < line_no <= len(line_offsets) else 0
        return any(start <= offset < end for start, end in skip_ranges)

    # Section numbers that this file ALSO labels with an identifier-shape
    # token (the cinderfall `§67 history_sim` shape). These are presumed
    # plan-section refs even when bare `§NN` recurrences appear elsewhere
    # in the same file — once labeled, the number is in the plan vocabulary.
    labeled_nums = _refs_with_strict_labels(text)
    # Plan-numbering range: anything ≤ max(known) is presumed to refer to
    # an in-plan section (caught: missing renumber, gap in the sequence).
    # Numbers above max(known) require explicit signal (label / filename
    # form / structured block) to count as plan refs.
    in_range_max = max(section_nums) if section_nums else 0

    out: list[Finding] = []
    seen: set[tuple[int, int]] = set()  # (section_num, line) — dedupe
    for hit in _extract_section_refs(text):
        if hit.section_num == 0:
            continue  # §0 / §00 → overview file, not a section
        if hit.section_num in section_nums:
            continue
        if _in_skip_range(hit.line):
            continue
        # Apply the policy gate.
        is_filename_form = hit.kind == "filename"
        has_label = hit.section_num in labeled_nums
        is_in_plan_range = 1 <= hit.section_num <= in_range_max
        if not (is_filename_form or has_label or is_in_plan_range):
            continue
        key = (hit.section_num, hit.line)
        if key in seen:
            continue
        seen.add(key)
        out.append(emit(prose_file, hit, plan_dir))
    return out


def _emit_block_orphans(
    prose_file: Path,
    full_text: str,
    block: _BlockRange,
    section_nums: set[int],
    plan_dir: Path,
    emit,
) -> Iterable[Finding]:
    """Emit one orphan finding per unresolved §NN inside `block`.

    `block.base_line` is precomputed by the shared `_find_block_ranges`
    helper — this function does NOT recompute the line number by
    re-finding `block_text` inside `full_text` (the prior implementation
    did, which was a `LEAK:scattered-knowledge` since the same offset
    was already known to the caller).

    Same false-positive guard as `_check_prose_file_existence` applies:
    only count a §NN as a plan-section ref when it is filename-form,
    has a strict identifier label adjacent (anywhere in the file), or
    falls inside the plan's existing section-number range. Plans whose
    MS-criteria casually mention external artifact sections (e.g. a
    refactor plan that says "preserves §5 round loop" referring to
    `tpr-review/SKILL.md §5`) stay quiet under this gate.
    """
    block_text = full_text[block.start_offset:block.end_offset]
    base_line = block.base_line
    labeled_nums = _refs_with_strict_labels(full_text)
    in_range_max = max(section_nums) if section_nums else 0
    out: list[Finding] = []
    seen: set[tuple[int, int]] = set()
    for hit in _extract_section_refs(block_text):
        if hit.section_num == 0:
            continue
        if hit.section_num in section_nums:
            continue
        is_filename_form = hit.kind == "filename"
        has_label = hit.section_num in labeled_nums
        is_in_plan_range = 1 <= hit.section_num <= in_range_max
        if not (is_filename_form or has_label or is_in_plan_range):
            continue
        adjusted = _RefHit(
            section_num=hit.section_num,
            line=base_line + (hit.line - 1),
            column=hit.column,
            snippet=hit.snippet,
            kind=hit.kind,
        )
        key = (adjusted.section_num, adjusted.line)
        if key in seen:
            continue
        seen.add(key)
        out.append(emit(prose_file, adjusted, plan_dir))
    return out


def _check_label_drift(
    prose_file: Path,
    text: str,
    sections: dict[int, Path],
) -> Iterable[Finding]:
    """Emit OVERVIEW_LABEL_DRIFT when prose-label tokens are disjoint from
    section title/goal/H1 tokens.

    Iterates every (section_num, label, line_no) triple from the prose
    file so each finding anchors at the line where the drift claim was
    actually made, even when the same section is labeled multiple times.
    The evidence snippet is the actual file line (verbatim, ≤ 120 chars)
    rather than a synthesized `§N label` string — verifies-against-code
    per the Findings grounding policy.
    """
    out: list[Finding] = []
    section_meta_cache: dict[Path, dict[str, object]] = {}
    # De-dupe by (section_num, label) — multiple restatements of the same
    # drift are one logical finding, anchored at the FIRST occurrence.
    # Without this, an overview that says "§67 `history_sim`" three times
    # produces three identical drift findings.
    seen_pairs: set[tuple[int, str]] = set()
    # Cache of split lines for evidence-snippet extraction.
    text_lines = text.split("\n")

    for section_num, prose_label, line_no in _extract_labels_with_lines(text):
        section_path = sections.get(section_num)
        if section_path is None:
            # Existence handled by the missing-section emitter.
            continue
        if section_path not in section_meta_cache:
            section_meta_cache[section_path] = _section_metadata(section_path)
        meta = section_meta_cache[section_path]
        if meta.get("parse_error") is not None:
            continue

        section_label_text = " ".join(
            str(meta.get(k) or "")
            for k in ("title", "goal", "body_h1")
        ).strip()
        section_tokens = _normalize_label(section_label_text)
        prose_tokens = _normalize_label(prose_label)
        if not prose_tokens:
            continue
        if prose_tokens & section_tokens:
            continue

        pair = (section_num, prose_label)
        if pair in seen_pairs:
            continue
        seen_pairs.add(pair)

        # Verbatim file line as evidence (≤120 chars), per Findings
        # grounding policy: evidence MUST be a verbatim quote from the
        # cited file. Synthesizing `§N label` strings hides whether the
        # finding is actually grounded in the source text.
        if 0 < line_no <= len(text_lines):
            raw = text_lines[line_no - 1].strip()
            snippet = raw if len(raw) <= 120 else raw[:117] + "…"
        else:
            snippet = f"§{section_num} {prose_label}"

        hit = _RefHit(
            section_num=section_num,
            line=line_no,
            column=1,
            snippet=snippet,
        )
        out.append(
            _emit_label_drift(
                prose_file,
                hit,
                prose_label,
                section_path,
                section_label_text,
            )
        )
    return out
