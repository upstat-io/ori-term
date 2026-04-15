#!/usr/bin/env python3
"""
Verify that plan subsection titles are in sync across:

  1. YAML frontmatter `sections:` array — each `{id, title, status}` entry
  2. The body `## {id} {title}` heading

This lint exists because subsection titles are duplicated across both
surfaces in every plan section file. When a title changes (e.g. a renamed
scope), it is easy to update one copy and miss the other — as happened
during the 2026-04-14 spec-conformance §08.5 round-11 TPR run, where the
08.5 subsection was renamed and the frontmatter entry, the `## 08.5`
heading, and the 08.N-checklist bullet drifted from each other across
three iterations before a reviewer noticed.

This script parses every `plans/**/section-*.md` file, extracts the
`sections:` frontmatter entries, finds each `## {id}` heading in the
body, and flags any cases where the frontmatter `title` does not match
(verbatim) the heading text that follows the id.

Usage:
  scripts/check-plan-subsection-sync.py                   # check all plans
  scripts/check-plan-subsection-sync.py plans/foo/ bar/   # check explicit paths
  scripts/check-plan-subsection-sync.py --help            # show help

Exit codes:
  0  every frontmatter title matches its body heading
  1  at least one drift detected
  2  usage error
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# ----- YAML frontmatter extraction (deliberately minimal — no PyYAML dep) -----

_FRONTMATTER_RE = re.compile(r"\A---\n(.*?)\n---\n", re.DOTALL)
_SECTION_ENTRY_RE = re.compile(
    r"""
    ^\s*-\s+id:\s*"?(?P<id>[^"\n]+?)"?\s*$     # - id: "08.5"
    \n\s*title:\s*"?(?P<title>.*?)"?\s*$       # title: "DECLRMM extended ..."
    """,
    re.MULTILINE | re.VERBOSE,
)


def extract_section_entries(text: str) -> list[tuple[str, str]]:
    """Return list of (id, title) tuples from the plan file's frontmatter."""
    m = _FRONTMATTER_RE.match(text)
    if not m:
        return []
    frontmatter = m.group(1)
    return [(m.group("id"), m.group("title")) for m in _SECTION_ENTRY_RE.finditer(frontmatter)]


# ----- Body heading extraction -----


def find_heading_title(text: str, section_id: str) -> str | None:
    """Find the `## {id} ...` heading in the body and return its trailing text."""
    pattern = rf"^##\s+{re.escape(section_id)}[:\s]\s*(.+?)\s*$"
    m = re.search(pattern, text, re.MULTILINE)
    return m.group(1) if m else None


# ----- Drift detection -----


def _normalize(s: str) -> str:
    """Normalize for loose comparison — strip backticks, collapse whitespace.

    The body heading and the frontmatter title commonly differ in ways that
    are cosmetic, not semantic:
      - body heading wraps identifiers in backticks (`foo`) while the
        frontmatter title uses bare words
      - body heading appends parentheticals with counts / impact qualifiers
        (e.g., "(Major × 3)", "(old behavior)")
      - whitespace collapses differently across markdown renderers

    Normalization strips backticks and collapses inner whitespace so the
    substring check below catches meaningful divergence (a renamed scope)
    while accepting cosmetic variation.
    """
    return re.sub(r"\s+", " ", s.replace("`", "")).strip().casefold()


def check_file(path: Path) -> list[str]:
    """Return a list of human-readable drift messages for this file."""
    text = path.read_text(encoding="utf-8")
    entries = extract_section_entries(text)
    drifts: list[str] = []

    for section_id, title in entries:
        # Skip meta subsections that never get a `## {id}` body heading.
        # `.R` (Third Party Review Findings) and `.N` (Completion Checklist)
        # use their own heading forms in existing plans.
        if section_id.endswith((".R", ".N")):
            continue

        heading = find_heading_title(text, section_id)
        if heading is None:
            drifts.append(
                f"{path}: frontmatter declares id={section_id!r} with title "
                f"{title!r}, but no `## {section_id} ...` heading found in body"
            )
            continue

        norm_title = _normalize(title)
        norm_heading = _normalize(heading)
        # Drift is flagged only when neither form contains the other — i.e.,
        # the texts share no common containment relationship. A body heading
        # that simply adds a parenthetical ("Polish (Major × 3)") still
        # contains the frontmatter title verbatim after normalization.
        if norm_title not in norm_heading and norm_heading not in norm_title:
            drifts.append(
                f"{path}:\n"
                f"  id:         {section_id}\n"
                f"  frontmatter title:  {title!r}\n"
                f"  body heading:       {heading!r}\n"
                f"  fix: make the body heading match the frontmatter title "
                f"(or update both if the rename is intentional)"
            )

    return drifts


# ----- CLI -----


def discover_plan_files(roots: list[Path]) -> list[Path]:
    """Find all `section-*.md` and `00-overview.md` files under the given roots."""
    files: list[Path] = []
    for root in roots:
        if root.is_file() and root.suffix == ".md":
            files.append(root)
            continue
        if not root.is_dir():
            continue
        files.extend(sorted(root.rglob("section-*.md")))
        files.extend(sorted(root.rglob("00-overview.md")))
    # Deduplicate while preserving order.
    seen: set[Path] = set()
    unique: list[Path] = []
    for f in files:
        if f not in seen:
            seen.add(f)
            unique.append(f)
    return unique


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "paths",
        nargs="*",
        type=Path,
        help="Plan directories or files to check. Default: plans/",
    )
    args = parser.parse_args()

    roots = args.paths or [Path("plans")]
    for root in roots:
        if not root.exists():
            print(f"error: path does not exist: {root}", file=sys.stderr)
            return 2

    files = discover_plan_files(roots)
    if not files:
        print(f"error: no plan files found under {roots}", file=sys.stderr)
        return 2

    all_drifts: list[str] = []
    for path in files:
        all_drifts.extend(check_file(path))

    if all_drifts:
        print(f"plan subsection title drift detected ({len(all_drifts)} issue(s)):\n")
        for drift in all_drifts:
            print(drift)
            print()
        print(
            "fix: make the body `## {id} ...` heading match the frontmatter "
            "title verbatim, or update both if the rename is intentional."
        )
        return 1

    checked = len(files)
    print(f"plan subsection title sync: OK ({checked} plan file(s) checked)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
