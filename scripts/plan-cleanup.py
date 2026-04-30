#!/usr/bin/env python3
"""
Plan-doc cleanup — applies scanner-detected auto-fixes before /commit-push stages.

Called from /commit-push Step 4 (after cargo fmt --all, before git add -A).
Idempotent. Silent when clean. Always exits 0 (cleanup failures do not block commits).

Handles four gate payloads:
- stale_frontmatter.focus_plan_mismatches  — rewrite `status:` in section/subsection frontmatter (roadmap_scan.py --json)
- stale_plan_annotations.cleanup_plan      — invoke plan-annotations.sh --cleanup-only (roadmap_scan.py --json)
- bug_marker_drift.auto_fix_edits          — insert `Superseded by:` line into bug-tracker entry (roadmap_scan.py --json)
- schema_violations.autofixable            — rename `id:`→`section:`, insert default `sections: []`, `reviewed: false`, `third_party_review:` stub (plan_corpus.validate direct)

Usage:
  python3 scripts/plan-cleanup.py           # default: run cleanup, report if fixes applied
  python3 scripts/plan-cleanup.py --quiet   # suppress summary line even on fixes
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path


def repo_root() -> Path:
    return Path(
        subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"], text=True
        ).strip()
    )


def run_scanner(root: Path) -> dict | None:
    try:
        out = subprocess.check_output(
            ["python3", ".claude/skills/continue-roadmap/roadmap_scan.py", "--json"],
            cwd=root,
            text=True,
            stderr=subprocess.DEVNULL,
        )
        return json.loads(out)
    except Exception:
        return None


# ---- 2a: stale frontmatter ----


def decide_new_status(issue: str) -> str | None:
    if "complete but" in issue and "unchecked" in issue:
        return "in-progress"
    if "not-started but" in issue and "checked" in issue:
        return "in-progress"
    if "in-progress but all items checked" in issue:
        return "complete"
    if "in-progress but 0 items checked" in issue:
        return "not-started"
    return None


def rewrite_toplevel_status(path: Path, new_status: str) -> bool:
    text = path.read_text()
    lines = text.splitlines(keepends=True)
    if not lines or lines[0].rstrip() != "---":
        return False
    closing = None
    for i, line in enumerate(lines[1:], start=1):
        if line.rstrip() == "---":
            closing = i
            break
    if closing is None:
        return False
    status_re = re.compile(r"^(status:\s*)\S+(\s*)$")
    for i in range(1, closing):
        m = status_re.match(lines[i])
        if m:
            current = lines[i].split(":", 1)[1].strip()
            if current == new_status:
                return False
            lines[i] = f"{m.group(1)}{new_status}{m.group(2)}\n"
            path.write_text("".join(lines))
            return True
    return False


def rewrite_subsection_status(path: Path, subsection_id: str, new_status: str) -> bool:
    text = path.read_text()
    lines = text.splitlines(keepends=True)
    target_re = re.compile(rf'^\s*-\s*id:\s*"?{re.escape(subsection_id)}"?\s*$')
    status_re = re.compile(r"^(\s+status:\s*)\S+(\s*)$")
    for i, line in enumerate(lines):
        if target_re.match(line):
            for j in range(i + 1, min(i + 8, len(lines))):
                # End of this entry: next `- id:` or un-indented line
                if re.match(r"^\s*-\s*id:", lines[j]) and j > i:
                    break
                if lines[j] and not lines[j].startswith(" "):
                    break
                m = status_re.match(lines[j])
                if m:
                    current = lines[j].split(":", 1)[1].strip()
                    if current == new_status:
                        return False
                    lines[j] = f"{m.group(1)}{new_status}{m.group(2)}\n"
                    path.write_text("".join(lines))
                    return True
            break
    return False


def decide_tpr_status(issue: str) -> str | None:
    # Scanner emits these two forms (roadmap_scan.py Section.tpr_mismatch):
    #   "third_party_review.status=findings but all N findings resolved (should be resolved)"
    #   "third_party_review.status=findings but no TPR findings parsed (should be resolved or none)"
    #   "third_party_review.status=resolved but N finding(s) still open (should be findings)"
    if "third_party_review.status=findings" in issue and "should be resolved" in issue:
        return "resolved"
    if "third_party_review.status=resolved" in issue and "should be findings" in issue:
        return "findings"
    return None


def rewrite_tpr_status(path: Path, new_status: str) -> bool:
    text = path.read_text()
    lines = text.splitlines(keepends=True)
    if not lines or lines[0].rstrip() != "---":
        return False
    closing = None
    for i, line in enumerate(lines[1:], start=1):
        if line.rstrip() == "---":
            closing = i
            break
    if closing is None:
        return False
    in_tpr_block = False
    status_re = re.compile(r"^(\s+status:\s*)\S+(\s*)$")
    for i in range(1, closing):
        if lines[i].startswith("third_party_review:"):
            in_tpr_block = True
            continue
        if in_tpr_block:
            if lines[i] and not lines[i].startswith(" "):
                # left the third_party_review block without finding status
                return False
            m = status_re.match(lines[i])
            if m:
                current = lines[i].split(":", 1)[1].strip()
                if current == new_status:
                    return False
                lines[i] = f"{m.group(1)}{new_status}{m.group(2)}\n"
                path.write_text("".join(lines))
                return True
    return False


def fix_frontmatter(mismatch: dict, root: Path) -> bool:
    plan = mismatch.get("plan", "")
    location = mismatch.get("location", "")
    issue = mismatch.get("issue", "")
    if "#" in location:
        filename, sub_id = location.split("#", 1)
    else:
        filename, sub_id = location, None
    path = root / "plans" / plan / filename
    if not path.exists():
        return False
    # third_party_review.status drift — scoped to the file-level mismatch (no
    # subsection anchor); the field lives in the plan-section frontmatter.
    if sub_id is None:
        new_tpr_status = decide_tpr_status(issue)
        if new_tpr_status is not None:
            return rewrite_tpr_status(path, new_tpr_status)
    new_status = decide_new_status(issue)
    if new_status is None:
        return False
    if sub_id:
        return rewrite_subsection_status(path, sub_id, new_status)
    return rewrite_toplevel_status(path, new_status)


# ---- 2b: stale plan annotations ----


def run_plan_annotations_cleanup(plan_name: str, root: Path) -> bool:
    script = root / ".claude/skills/impl-hygiene-review/plan-annotations.sh"
    if not script.exists():
        return False
    result = subprocess.run(
        ["bash", str(script), "--cleanup-only", "--plan", plan_name],
        cwd=root,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return result.returncode == 0


# ---- 2c: bug marker drift ----


def apply_bug_marker_edit(edit: dict, root: Path) -> bool:
    file_path = root / edit.get("file", "")
    if not file_path.exists():
        return False
    header_lineno = edit.get("header_lineno", 0)  # 1-based
    insert_line = edit.get("insert_line", "").rstrip("\n") + "\n"
    if not insert_line.strip():
        return False

    lines = file_path.read_text().splitlines(keepends=True)
    if header_lineno < 1 or header_lineno > len(lines):
        return False

    end = header_lineno  # 0-based index of first line AFTER header
    for j in range(header_lineno, len(lines)):
        line = lines[j]
        if not line.strip():
            break
        if line.lstrip().startswith("- ["):
            break
        if "Superseded by:" in line:
            return False  # idempotent
        end = j + 1

    lines.insert(end, insert_line)
    file_path.write_text("".join(lines))
    return True


# ---- schema violation autofixes (id→section, missing required defaults) ----


_SECTION_FILE_RE = re.compile(r"section-\d{2}.*\.md$")


def _frontmatter_bounds(lines: list[str]) -> tuple[int, int] | None:
    """Return (open_idx, close_idx) — indices of the '---' fences, or None if missing."""
    if not lines or lines[0].rstrip() != "---":
        return None
    for i in range(1, len(lines)):
        if lines[i].rstrip() == "---":
            return (0, i)
    return None


def _has_key(lines: list[str], open_idx: int, close_idx: int, key: str) -> bool:
    """Check if a top-level key appears in the frontmatter block."""
    key_re = re.compile(rf"^{re.escape(key)}:\s*")
    for i in range(open_idx + 1, close_idx):
        if key_re.match(lines[i]):
            return True
    return False


def rename_id_to_section(path: Path) -> bool:
    """Rename the top-level `id:` key to `section:` when section: is absent.

    Safety gates: file name MUST match section-NN-*.md AND `section:` must be
    absent AND `id:` must be present as a top-level frontmatter key.
    """
    if not _SECTION_FILE_RE.search(path.name):
        return False
    text = path.read_text()
    lines = text.splitlines(keepends=True)
    bounds = _frontmatter_bounds(lines)
    if bounds is None:
        return False
    open_idx, close_idx = bounds
    if _has_key(lines, open_idx, close_idx, "section"):
        return False
    id_re = re.compile(r"^(id:)(\s.*)$")
    for i in range(open_idx + 1, close_idx):
        m = id_re.match(lines[i])
        if m:
            lines[i] = "section:" + m.group(2) + (
                "" if lines[i].endswith("\n") else "\n"
            )
            if not lines[i].endswith("\n"):
                lines[i] += "\n"
            path.write_text("".join(lines))
            return True
    return False


def insert_default_field(path: Path, key: str, default_lines: list[str]) -> bool:
    """Insert a missing required field with a safe default into frontmatter.

    Appends just before the closing `---` fence. Idempotent: if `key:` already
    present as a top-level frontmatter key, does nothing.

    default_lines: list of strings (each ending in newline) to insert as the
    field's lines. Example: ["sections: []\\n"] or
    ["third_party_review:\\n", "  status: none\\n", "  updated: null\\n"].
    """
    text = path.read_text()
    lines = text.splitlines(keepends=True)
    bounds = _frontmatter_bounds(lines)
    if bounds is None:
        return False
    open_idx, close_idx = bounds
    if _has_key(lines, open_idx, close_idx, key):
        return False
    # Ensure the line before close_idx ends with \n so our insert looks clean.
    if close_idx - 1 >= open_idx + 1 and not lines[close_idx - 1].endswith("\n"):
        lines[close_idx - 1] = lines[close_idx - 1] + "\n"
    for offset, insert_line in enumerate(default_lines):
        if not insert_line.endswith("\n"):
            insert_line = insert_line + "\n"
        lines.insert(close_idx + offset, insert_line)
    path.write_text("".join(lines))
    return True


_DEFAULT_THIRD_PARTY_REVIEW = [
    "third_party_review:\n",
    "  status: none\n",
    "  updated: null\n",
]


def _collect_schema_autofixes(root: Path) -> list[dict]:
    """Run plan_corpus validate over plans/ and return planned autofix edits.

    Each edit is a dict with keys: `file`, `kind`, `extra` (per-kind payload).
    Only schema-violation findings that map to a known autofix produce edits.
    """
    try:
        sys.path.insert(0, str(root))
        from scripts.plan_corpus.discovery import load_and_validate  # noqa: E402
    except Exception:
        return []
    plans_dir = root / "plans"
    if not plans_dir.exists():
        return []

    # Walk plans/ for all .md files. load_and_validate handles classification
    # + schema validation per file. Files outside the seven schema classes
    # return an unclassified gap finding; we ignore those here.
    per_file: dict[Path, dict] = {}
    for md_path in plans_dir.rglob("*.md"):
        if not md_path.is_file():
            continue
        try:
            result = load_and_validate(md_path)
        except Exception:
            continue
        vf = result.ok
        if vf is None:
            continue
        missing: set[str] = set()
        unknown: set[str] = set()
        for v in getattr(vf, "violations", []):
            cat = getattr(v.category, "value", str(v.category))
            if cat != "schema_violation":
                continue
            sub = getattr(v.subtype, "value", str(v.subtype))
            key = getattr(v, "target_key", None)
            if not key:
                continue
            if sub == "missing_required_field":
                missing.add(key)
            elif sub == "unknown_field":
                unknown.add(key)
        if missing or unknown:
            per_file[vf.path] = {
                "file_class": getattr(vf.file_class, "value", str(vf.file_class)),
                "missing": missing,
                "unknown": unknown,
            }

    edits: list[dict] = []
    section_classes = {"plan_section", "roadmap_section", "bug_tracker_section"}
    for path, info in per_file.items():
        fc = info["file_class"]
        missing = info["missing"]
        unknown = info["unknown"]
        # Rule 1: id→section rename when section missing + id unknown + section filename
        if (
            "section" in missing
            and "id" in unknown
            and _SECTION_FILE_RE.search(path.name)
        ):
            edits.append({"file": str(path), "kind": "rename_id_to_section"})
        # Rule 2: sections: [] default for section-like schemas missing sections
        if "sections" in missing and fc in section_classes:
            edits.append(
                {
                    "file": str(path),
                    "kind": "insert_default",
                    "key": "sections",
                    "lines": ["sections: []\n"],
                }
            )
        # Rule 3: reviewed: false default
        if "reviewed" in missing and fc in section_classes:
            edits.append(
                {
                    "file": str(path),
                    "kind": "insert_default",
                    "key": "reviewed",
                    "lines": ["reviewed: false\n"],
                }
            )
        # Rule 4: third_party_review: stub default
        if "third_party_review" in missing and fc == "plan_section":
            edits.append(
                {
                    "file": str(path),
                    "kind": "insert_default",
                    "key": "third_party_review",
                    "lines": _DEFAULT_THIRD_PARTY_REVIEW,
                }
            )
    return edits


def apply_schema_autofix(edit: dict) -> bool:
    """Apply a single schema autofix edit. Returns True if file changed."""
    path = Path(edit["file"])
    if not path.exists():
        return False
    kind = edit.get("kind")
    if kind == "rename_id_to_section":
        return rename_id_to_section(path)
    if kind == "insert_default":
        return insert_default_field(path, edit["key"], edit["lines"])
    return False


# ---- main ----


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--quiet", action="store_true", help="Suppress summary output.")
    args = ap.parse_args()

    try:
        root = repo_root()
    except Exception:
        return 0

    data = run_scanner(root)
    if data is None:
        return 0

    gates = data.get("gates", {})

    frontmatter_fixed = 0
    annotations_fixed = False
    markers_fixed = 0

    sf = gates.get("stale_frontmatter", {}).get("payload", {})
    # Prefer all_mismatches (every plan) so cleanup is scope-agnostic; fall
    # back to focus_plan_mismatches for older scanner versions.
    mismatches = sf.get("all_mismatches") or sf.get("focus_plan_mismatches", [])
    for m in mismatches:
        if fix_frontmatter(m, root):
            frontmatter_fixed += 1

    spa = gates.get("stale_plan_annotations", {}).get("payload", {})
    if spa.get("count", 0) > 0:
        plan_name = spa.get("cleanup_plan")
        if plan_name and run_plan_annotations_cleanup(plan_name, root):
            annotations_fixed = True

    bmd = gates.get("bug_marker_drift", {}).get("payload", {})
    for edit in bmd.get("auto_fix_edits", []):
        if apply_bug_marker_edit(edit, root):
            markers_fixed += 1

    schema_fixed = 0
    for edit in _collect_schema_autofixes(root):
        if apply_schema_autofix(edit):
            schema_fixed += 1

    total = (
        frontmatter_fixed
        + (1 if annotations_fixed else 0)
        + markers_fixed
        + schema_fixed
    )
    if total > 0 and not args.quiet:
        print(
            f"plan-cleanup: {frontmatter_fixed} frontmatter, "
            f"{1 if annotations_fixed else 0} annotations, "
            f"{markers_fixed} markers, "
            f"{schema_fixed} schema"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
