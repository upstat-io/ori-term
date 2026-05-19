#!/usr/bin/env python3
"""
prose-lint: scan authored .md and ori_term source for forbidden vocabulary.

Two pattern packs, dispatched by file extension:

  - Authored .md (.claude/skills, .claude/commands, .claude/rules) \u2014 prose
    creep: dated narrative, history keywords, rationale tails, paragraphs
    longer than the configured sentence threshold. Rule: global CLAUDE.md
    \u00a7"NO PROSE IN AUTHORED .md FILES" + .claude/skills/improve-tooling/SKILL.md
    \u00a7"No Prose in Authored .md Files".

  - ori_term source (.rs under term_repo) \u2014 internal-vocabulary leaks into
    the public OSS repo: bug IDs, methodology vocabulary, reviewer-tool
    names, internal-doc paths, and unattributed references to other
    terminal emulators (tmux, alacritty, wezterm, ghostty, kitty, rio,
    xterm, xterm.js, ratatui, etc.).
    Reference-implementation citations are allowed when paired with a
    verifiable file:line path. Rule: project CLAUDE.md \u00a7"Public Repo
    Never Leaks Private-Repo Identifiers" + \u00a7"Reference Repos".

Exit codes: 0 clean (or --exit-zero), 1 violations found, 2 usage error.
"""

import argparse
import json
import re
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Pattern packs \u2014 (regex, label, flags). flags=0 means case-sensitive.
# ---------------------------------------------------------------------------

# Authored .md prose-creep patterns (history-keywords, dated refs, etc).
KEYWORD_PATTERNS = [
    (r"\b(previously|originally|restoring|defeating)\b", "history-keyword", re.IGNORECASE),
    (r"\bas of 20\d{2}(-\d{2}(-\d{2})?)?\b", "dated-ref-as-of", re.IGNORECASE),
    (r"\bsince 20\d{2}(-\d{2}(-\d{2})?)?\b", "dated-ref-since", re.IGNORECASE),
    (r"\u2014 causes\b", "rationale-tail-em-dash-causes", re.IGNORECASE),
    (r"\bwas (originally|previously)\b", "history-phrase-was-originally", re.IGNORECASE),
]

# ori_term source internal-vocabulary leak patterns. Each catches a category
# of private-repo identifier that must not appear in the public terminal-
# emulator repo.
SOURCE_KEYWORD_PATTERNS = [
    # Bug-tracker IDs \u2014 internal scheme, never public.
    (r"\bBUG-\d{2}-\d{3}\b", "bug-id", 0),
    # Methodology vocabulary \u2014 review-loop and TDD-discipline names.
    (r"\bINVERTED-TDD\b", "methodology-inverted-tdd", 0),
    (r"\bPlan\s+TPR\s+Round\b", "methodology-plan-tpr", re.IGNORECASE),
    (r"\bTPR\s+Round\b", "methodology-tpr-round", re.IGNORECASE),
    (r"\b(?:semantic|negative)\s+pin\b", "methodology-pin-vocab", re.IGNORECASE),
    (r"\bTDD\s+matrix\b", "methodology-tdd-matrix", re.IGNORECASE),
    # Reviewer-tool names \u2014 case-sensitive lowercase only (the literal CLI
    # invocation form). Capitalized "Codex" or "Gemini" in product-name
    # context is uppercase + survives the regex. Bypass markers are banned
    # outright \u2014 see BYPASS_MARKER_PATTERNS.
    (r"\b(?:codex|gemini|opencode)\b", "reviewer-name", 0),
    # Reviewer-emphasis \u2014 uppercase-only "AGREEMENT" appears exclusively in
    # review-trail comments ("codex+opencode AGREEMENT"); regular prose uses
    # lowercase.
    (r"\bAGREEMENT\b", "reviewer-emphasis", 0),
    # Internal-doc paths \u2014 references to private rule files and CLAUDE.md.
    (r"\bCLAUDE\.md\b", "internal-doc-claude-md", 0),
    (r"\.claude/(?:rules|skills|commands|hooks)/", "internal-doc-claude-path", 0),
    (r"\bimpl-hygiene\.md\b", "internal-doc-impl-hygiene", re.IGNORECASE),
    (r"\baims-rules\.md\b", "internal-doc-aims-rules", re.IGNORECASE),
    (r"\bcodegen-rules\.md\b", "internal-doc-codegen-rules", re.IGNORECASE),
    (r"\btypeck\.md\b", "internal-doc-typeck", re.IGNORECASE),
    (r"\bcanon\.md\b", "internal-doc-canon", re.IGNORECASE),
]

# Reference-implementation attribution patterns. ori_term is a Rust
# terminal emulator using wgpu / winit / vte / portable-pty, and the
# project explicitly compares itself against established terminal
# emulators (tmux, alacritty, wezterm, ghostty, kitty, rio, xterm,
# xterm.js, ptyxis, notcurses) and Rust UI frameworks (ratatui,
# crossterm) plus Go TUI libs (bubbletea, lipgloss, termenv) — see
# CLAUDE.md §Reference Repos. Plain mentions (`tmux's grid stores
# extended cells`, citing a reference path verbatim) are fine. What
# gets flagged is *attribution form* — telling the reader the design
# was copied from another terminal emulator without a verifiable
# file:line citation. Allowed by structural exemption when the attribution
# carries a verifiable file:line cite (e.g. "WezTerm
# `term/src/terminalstate/performer.rs:473-478`") matched by the
# `reference-lang-source-cite` pattern below. The bypass-marker route
# was removed by CLAUDE.md §NEVER SILENCE LINTERS.
#
# `xterm.js` is matched as a single token (the `.js` suffix
# distinguishes it from bare `xterm`); both forms are caught.
_REFERENCE_LANGS = (
    r"(?:tmux|[Aa]lacritty|[Ww]ez[Tt]erm|[Gg]hostty|[Kk]itty|[Rr]io|"
    r"[Xx]term\.js|[Xx]term|[Pp]tyxis|[Nn]otcurses|[Rr]atatui|"
    r"[Cc]rossterm|[Bb]ubbletea|[Ll]ipgloss|[Tt]ermenv)"
)
SOURCE_KEYWORD_PATTERNS.extend([
    (
        rf"\b{_REFERENCE_LANGS}'s\s+"
        r"(?:pattern|approach|design|implementation|model|version|way|"
        r"equivalent|style|grid|reflow|selection|damage|cursor|"
        r"tracking|escape\s+handling|VT\s+parser|terminfo)\b",
        "reference-impl-possessive",
        0,
    ),
    (
        rf"\b(?:[Ff]ollowing|[Ii]nspired by|[Pp]atterned (?:on|after)|"
        rf"[Dd]erived from|[Mm]irrors|[Bb]ased on|[Aa]s in)\s+{_REFERENCE_LANGS}\b",
        "reference-impl-attribution-verb",
        0,
    ),
    (
        rf"\b{_REFERENCE_LANGS}-(?:derived|inspired|style|like|pattern|equivalent)\b",
        "reference-impl-hyphenated",
        0,
    ),
    (
        rf"^\s*//[/!]*\s*[-*]?\s*\*?\*?{_REFERENCE_LANGS}\*?\*?\s*[:`]",
        "reference-impl-bullet-header",
        re.MULTILINE,
    ),
])

# Exemption: a line carrying a verbatim file:line cite of the reference
# implementation (e.g. WezTerm `term/src/terminalstate/performer.rs:473-478`)
# is the canonical attribution form per CLAUDE.md §Reference Repos. Such a
# line is an explicit, verifiable citation — not an unattributed copy claim —
# so reference-impl-* patterns on the same line are suppressed.
# Common terminal-emulator source extensions: Rust, Go, C, C++, Zig,
# Python (kitty), JavaScript/TypeScript (xterm.js).
REFERENCE_IMPL_CITE_RE = re.compile(
    rf"\b{_REFERENCE_LANGS}\s+`[^`]*\.(?:rs|go|c|h|cc|cpp|zig|py|js|ts)`"
)

# Suppressions (false-positive guards)
COMPOUND_ADJ_RE = re.compile(
    r"\bpreviously-(failing|valid|completed|written|verified|seen|working|broken|projected|existing|resolved)\b",
    re.IGNORECASE,
)
STATE_LABEL_RE = re.compile(
    r"\*\*(CONFIRMED|REGRESSED|FIXED|PASSED|FAILED|NEW|RESOLVED)\*\*[^\n]*\bpreviously\b",
    re.IGNORECASE,
)

# Regression doc-comment exemption: `///` doc comments leading with
# "Regression: BUG-XX-NNN" or "See: bug-tracker/plans/BUG-XX-NNN/..." are
# the canonical pin format per .claude/rules/tests.md §Regression
# Discipline. The bug-id rule must not fire on these lines.
REGRESSION_DOC_COMMENT_RE = re.compile(
    r"^\s*//[/!]\s*(?:Regression\s*:|See\s*:\s*bug-tracker/)",
    re.IGNORECASE,
)

# Test-disposition attribute exemption: `#[ignore = "BUG-XX-NNN: ..."]`,
# `#[skip = "BUG-XX-NNN ..."]`, `#[fail = "..."]`, `#[compile_fail = "..."]`
# attributes REQUIRE a `BUG-XX-NNN` reference per CLAUDE.md §Test
# Disposition Discipline. The bug-id rule must not fire on bug IDs that
# fall at-or-after the attribute prefix on the same line.
DISPOSITION_PREFIX_RE = re.compile(
    r"(?:#\[ignore(?:\s*[(=])|#(?:skip|fail|compile_fail)\s*\()"
)

# Banned-escape-marker patterns. The linter's job is to surface authored-
# prose defects; silencing the report at the marker level was misused
# historically to bypass real violations (see CLAUDE.md §NEVER SILENCE
# LINTERS). The capability is removed: every match below is reported as its
# own meta-violation so the operator must STRIP the marker (and fix the
# underlying defect) rather than bypass the gate.
BYPASS_MARKER_PATTERNS = [
    (re.compile(r"<!--\s*prose-lint:\s*off\s*-->"), "bypass-marker:md-off"),
    (re.compile(r"<!--\s*prose-lint:\s*on\s*-->"), "bypass-marker:md-on"),
    (re.compile(r"<!--\s*prose-lint:\s*allow\s*-->"), "bypass-marker:md-allow"),
    (re.compile(r"//\s*prose-lint:\s*off\b"), "bypass-marker:src-off"),
    (re.compile(r"//\s*prose-lint:\s*on\b"), "bypass-marker:src-on"),
    (re.compile(r"//\s*prose-lint:\s*allow\b"), "bypass-marker:src-allow"),
]


# Design-log section headers where prose is allowed (see Exceptions table)
EXEMPT_HEADER_RE = re.compile(r"^##\s+\u00a7[46]\b")

# Markdown constructs (list/table/blockquote)
LIST_OR_TABLE_RE = re.compile(r"^\s*([-*+]|\d+\.|>|\|)")
INDENT_CONTINUATION_RE = re.compile(r"^\s{2,}\S")

DEFAULT_ROOTS = [".claude/skills", ".claude/commands", ".claude/rules"]
DEFAULT_MAX_SENTENCES = 2

# File-extension dispatch.
MD_EXT = ".md"
# ori_term is pure Rust — no `.ori` source files (that suffix belongs to the
# ori_lang compiler this script was originally written for; dropped here to
# stop scanning a non-existent file class on every invocation).
SOURCE_EXTS = {".rs"}
LINT_EXTENSIONS = {MD_EXT} | SOURCE_EXTS

# Directories never traversed during recursive scans.
EXCLUDE_DIRS = {"target", ".git", "node_modules", "build", "dist", "__pycache__", ".venv"}


# ---------------------------------------------------------------------------
# Per-file exemptions
# ---------------------------------------------------------------------------

def is_design_log(path: Path) -> bool:
    return path.name.endswith("-design.md")


def is_exempt_path(path: Path) -> bool:
    s = str(path)
    # Rule-definition file itself (self-reference)
    if path.name == "SKILL.md" and "improve-tooling" in s:
        return True
    # Project CLAUDE.md — the rule-definition file that enumerates banned
    # phrases inline as examples of what NOT to do. The file would otherwise
    # fire its own rules on itself.
    if path.name == "CLAUDE.md":
        return True
    # CHANGELOG / HISTORY / README — documentation, not prescriptive Claude artifacts.
    # README.md files describe a crate's purpose and need explanatory prose.
    if path.name in ("CHANGELOG.md", "HISTORY.md", "README.md"):
        return True
    # Error-code documentation files (e.g., E1015.md, E2007.md, E4001.md) —
    # these explain compiler errors to users with prose, not prescriptive
    # rules. Match `errors/EXXXX.md` shape.
    if "errors" in path.parts and re.match(r"^E\d{4}\.md$", path.name):
        return True
    # Rosetta-style task descriptions in test fixtures — external problem
    # statements copied from public task corpora (Rosetta Code etc.), not
    # authored compiler docs.
    if "_tasks" in path.parts and "rosetta" in path.parts:
        return True
    return False


# ---------------------------------------------------------------------------
# Region exemption - fences, design-log sections, lint-off blocks
# ---------------------------------------------------------------------------

def compute_exempt_lines(lines, path, kind):
    """Return the set of 1-indexed line numbers that the keyword scan must skip.

    Lint-off / lint-on / lint-allow directives are NO LONGER honored — see
    BYPASS_MARKER_PATTERNS. Only structural exemptions remain: fenced code
    blocks in .md and design-log §4 / §6 sections.
    """
    exempt = set()
    in_fence = False
    in_design_exempt = False
    design = is_design_log(path) if kind == "md" else False
    for idx, line in enumerate(lines, 1):
        stripped = line.strip()
        if kind == "md" and stripped.startswith("```"):
            in_fence = not in_fence
            exempt.add(idx)
            continue
        if in_fence:
            exempt.add(idx)
            continue
        if design and line.startswith("## "):
            in_design_exempt = bool(EXEMPT_HEADER_RE.match(line))
        if in_design_exempt:
            exempt.add(idx)
    return exempt


# ---------------------------------------------------------------------------
# Sentence counting (rough but good enough)
# ---------------------------------------------------------------------------

SENTENCE_END_RE = re.compile(r"[.!?](?:\s|$)")
ABBREVIATIONS = ["e.g.", "i.e.", "etc.", "vs.", "Dr.", "Mr.", "Mrs.", "Ms.", "St.", "Ft."]


def count_sentences(text):
    t = re.sub(r"`[^`]*`", " ", text)
    t = re.sub(r"https?://\S+", " ", t)
    for abbr in ABBREVIATIONS:
        t = t.replace(abbr, abbr.replace(".", ""))
    count = len(SENTENCE_END_RE.findall(t))
    stripped = t.rstrip()
    if stripped and stripped[-1] not in ".!?":
        count += 1
    return count


# ---------------------------------------------------------------------------
# Paragraph detection
# ---------------------------------------------------------------------------

def find_long_paragraphs(lines, exempt, max_sentences):
    findings = []
    n = len(lines)
    i = 0
    while i < n:
        lineno = i + 1
        line = lines[i]
        stripped = line.strip()
        if (
            not stripped
            or stripped.startswith("#")
            or stripped.startswith("```")
            or LIST_OR_TABLE_RE.match(line)
            or INDENT_CONTINUATION_RE.match(line)
            or lineno in exempt
        ):
            i += 1
            continue
        start = lineno
        buf = [stripped]
        j = i + 1
        while j < n:
            nxt = lines[j]
            ns = nxt.strip()
            if (
                not ns
                or ns.startswith("#")
                or ns.startswith("```")
                or LIST_OR_TABLE_RE.match(nxt)
                or (j + 1) in exempt
            ):
                break
            buf.append(ns)
            j += 1
        text = " ".join(buf)
        count = count_sentences(text)
        if count > max_sentences:
            excerpt = (text[:180] + "\u2026") if len(text) > 180 else text
            findings.append({
                "line": start,
                "end_line": j,
                "type": "paragraph-too-long",
                "sentence_count": count,
                "excerpt": excerpt,
            })
        i = j if j > i else i + 1
    return findings


# ---------------------------------------------------------------------------
# Keyword scan
# ---------------------------------------------------------------------------

def keyword_scan(lines, exempt, patterns, apply_md_suppressors):
    """Scan lines for forbidden keywords/regexes.

    patterns: list of (regex, label, flags) tuples.
    apply_md_suppressors: when True, applies STATE_LABEL_RE (suppress
        **CONFIRMED|REGRESSED|FIXED ... previously ...** lines) and
        COMPOUND_ADJ_RE masking ("previously-failing" etc) — both are
        prose-pack false-positive guards, irrelevant for source scans.

    Bypass markers (`// prose-lint: allow` / `off` / `on`, and the
    HTML-comment forms) are NOT honored — each match is reported as its own
    finding so the operator strips the marker and fixes the underlying defect.
    """
    findings = []
    for idx, line in enumerate(lines, 1):
        # Bypass-marker meta-detection runs BEFORE structural exemptions —
        # we still want to surface markers inside fenced code blocks if any
        # production code somehow grew them.
        for bypass_re, bypass_label in BYPASS_MARKER_PATTERNS:
            m = bypass_re.search(line)
            if m:
                findings.append({
                    "line": idx,
                    "type": bypass_label,
                    "message": (
                        "banned escape marker — strip and fix the underlying defect "
                        "(see CLAUDE.md §NEVER SILENCE LINTERS)"
                    ),
                    "match": m.group(0),
                    "snippet": line.rstrip()[:200],
                })
        if idx in exempt:
            continue
        if apply_md_suppressors and STATE_LABEL_RE.search(line):
            continue
        # Regression doc comments are provenance metadata per
        # .claude/rules/tests.md §Regression Discipline — they legitimately
        # cite bug IDs (BUG-XX-NNN) and TDD vocabulary (semantic pin,
        # negative pin, TDD matrix, INVERTED-TDD, TPR Round). Other
        # keyword rules (reference-lang attribution, internal-doc paths,
        # reviewer names like codex/gemini, dated-ref) MUST still apply
        # even on regression doc-comment lines — those are real leaks
        # regardless of doc-comment context.
        regression_doc_line = REGRESSION_DOC_COMMENT_RE.search(line) is not None
        regression_exempt_labels = {
            "bug-id",
            "methodology-inverted-tdd",
            "methodology-plan-tpr",
            "methodology-tpr-round",
            "methodology-pin-vocab",
            "methodology-tdd-matrix",
        }
        # Lines carrying a verbatim file:line cite of a reference impl are
        # exempt from all reference-impl-* attribution patterns — the cite IS
        # the attribution.
        cite_line = REFERENCE_IMPL_CITE_RE.search(line) is not None
        masked = COMPOUND_ADJ_RE.sub("<compound>", line) if apply_md_suppressors else line
        # Test-disposition attribute: BUG-XX-NNN matches at-or-after a
        # `#[ignore = "..."]` / `#[skip(...)]` / `#[fail(...)]` /
        # `#[compile_fail(...)]` prefix are required by CLAUDE.md §Test
        # Disposition Discipline and MUST be exempt from the bug-id rule.
        disp_m = DISPOSITION_PREFIX_RE.search(line)
        disp_start = disp_m.end() if disp_m else None
        for regex, label, flags in patterns:
            if regression_doc_line and label in regression_exempt_labels:
                continue
            if cite_line and label.startswith("reference-impl-"):
                continue
            m = re.search(regex, masked, flags)
            if m:
                if (
                    label == "bug-id"
                    and disp_start is not None
                    and m.start() >= disp_start
                ):
                    continue
                findings.append({
                    "line": idx,
                    "type": "keyword",
                    "pattern": label,
                    "match": m.group(0),
                    "excerpt": line.strip()[:180],
                })
                break
    return findings


# ---------------------------------------------------------------------------
# File + path handling
# ---------------------------------------------------------------------------

def scan_file(path: Path, max_sentences: int):
    if is_exempt_path(path):
        return []
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as e:
        return [{"file": str(path), "line": 0, "type": "read-error", "message": str(e)}]

    suffix = path.suffix.lower()
    if suffix == MD_EXT:
        kind = "md"
        patterns = KEYWORD_PATTERNS
    elif suffix in SOURCE_EXTS:
        kind = "source"
        patterns = SOURCE_KEYWORD_PATTERNS
    else:
        return []

    lines = text.splitlines()
    exempt = compute_exempt_lines(lines, path, kind)
    items = keyword_scan(
        lines, exempt,
        patterns=patterns,
        apply_md_suppressors=(kind == "md"),
    )
    if kind == "md":
        items.extend(find_long_paragraphs(lines, exempt, max_sentences))
    return [{"file": str(path), **f} for f in items]


def collect_paths(roots):
    out = []
    for r in roots:
        p = Path(r)
        if p.is_file() and p.suffix in LINT_EXTENSIONS:
            out.append(p)
        elif p.is_dir():
            for fp in p.rglob("*"):
                if not fp.is_file() or fp.suffix not in LINT_EXTENSIONS:
                    continue
                if any(part in EXCLUDE_DIRS for part in fp.parts):
                    continue
                out.append(fp)
    return sorted(set(out))


# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

def format_human(findings, scanned):
    if not findings:
        return f"prose-lint: clean - {scanned} file(s) scanned, 0 violations.\n"
    by_file = {}
    for f in findings:
        by_file.setdefault(f["file"], []).append(f)
    out = [f"prose-lint: {len(findings)} violation(s) across {len(by_file)} file(s) (of {scanned} scanned)"]
    has_bug_id = any(f.get("pattern") == "bug-id" for f in findings)
    has_internal_doc = any(
        isinstance(f.get("pattern"), str) and f["pattern"].startswith("internal-doc-")
        for f in findings
    )
    has_wrapper_path = any(
        isinstance(f.get("pattern"), str) and f["pattern"].startswith("wrapper-")
        for f in findings
    )
    for file, fs in sorted(by_file.items()):
        out.append("")
        out.append(file)
        for f in sorted(fs, key=lambda x: x["line"]):
            t = f["type"]
            if t == "keyword":
                match = f.get("match", "")
                out.append(f"  :{f['line']}  [{f['pattern']}] match={match!r}")
                out.append(f"      {f['excerpt']}")
            elif t == "paragraph-too-long":
                out.append(
                    f"  :{f['line']}-{f['end_line']}  [paragraph: {f['sentence_count']} sentences > threshold]"
                )
                out.append(f"      {f['excerpt']}")
            else:
                out.append(f"  :{f['line']}  [{t}] {f.get('message','')}")
    if has_bug_id or has_internal_doc or has_wrapper_path:
        out.append("")
        out.append("Hints (per .claude/rules/tests.md §Regression Discipline + impl-hygiene.md):")
        if has_bug_id:
            out.append("  bug-id violations exempt these line prefixes:")
            out.append("    /// Regression: BUG-XX-NNN ...        (canonical regression-anchor doc comment)")
            out.append("    /// See: bug-tracker/plans/...        (canonical plan-pointer doc comment)")
            out.append("    Bypass markers (// prose-lint: allow / off / on) are BANNED — see CLAUDE.md §NEVER SILENCE LINTERS.")
        if has_internal_doc:
            out.append("  internal-doc-* matches reference wrapper-private rule files (e.g., impl-hygiene.md).")
            out.append("    Public source MUST NOT cite wrapper docs — refactor the comment to describe")
            out.append("    intent rather than cite the wrapper rule by name.")
        if has_wrapper_path:
            out.append("  wrapper-* matches reference wrapper-only paths (e.g., bug-tracker/plans/).")
            out.append("    These are exempt only on canonical `/// See: bug-tracker/plans/...` regression")
            out.append("    doc-comment lines; otherwise refactor or use the prose-lint allow marker.")
    out.append("")
    return "\n".join(out)


def main():
    parser = argparse.ArgumentParser(
        prog="prose-lint",
        description="Scan authored .md and compiler source for forbidden vocabulary.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""\
Scope (default): .claude/skills .claude/commands .claude/rules

Pattern packs (dispatched by file extension):

  Authored .md:
    history-keyword          previously, originally, restoring, defeating
    dated-ref-as-of/since    "as of YYYY", "since YYYY"
    rationale-tail           "\u2014 causes \u2026"
    history-phrase           "was originally|previously"
    paragraph-too-long       paragraph exceeds --max-paragraph-sentences

  Compiler source (.rs, .ori):
    bug-id                   BUG-XX-NNN
    methodology-*            INVERTED-TDD, Plan TPR Round, TPR Round,
                             semantic/negative pin, TDD matrix
    reviewer-name            codex, gemini, opencode (case-sensitive)
    reviewer-emphasis        AGREEMENT (uppercase only)
    internal-doc-*           CLAUDE.md, .claude/{rules,skills,commands,hooks}/,
                             impl-hygiene.md, aims-rules.md, codegen-rules.md,
                             typeck.md, canon.md

Exempts:
  - Fenced code blocks (.md only).
  - Design-log sections: ## \u00a74 Lessons and ## \u00a76 Improvement Log (.md only).
  - Rule-definition self-reference: .claude/skills/improve-tooling/SKILL.md.
  - CHANGELOG.md / HISTORY.md.
  - State-label definitions (**CONFIRMED|REGRESSED|FIXED** ... previously ...) \u2014 .md only.
  - Hyphenated compound adjectives (previously-failing/valid/...) \u2014 .md only.
  - target/, .git/, node_modules/, build/, dist/, __pycache__/, .venv/.

Bypass markers BANNED \u2014 see CLAUDE.md \u00a7NEVER SILENCE LINTERS:
  `// prose-lint: allow / off / on` and the `<!-- prose-lint: ... -->` forms
  are NOT honored. Every occurrence is reported as its own meta-violation
  (`bypass-marker:*`). Operators MUST strip the marker AND fix the underlying
  defect, not silence the gate.

Exit codes: 0 clean, 1 violations found, 2 usage error.
""",
    )
    parser.add_argument(
        "paths", nargs="*",
        help=f"Files or dirs to scan (default: {' '.join(DEFAULT_ROOTS)})",
    )
    parser.add_argument("--json", action="store_true", help="Emit JSON findings")
    parser.add_argument(
        "--max-paragraph-sentences", type=int, default=DEFAULT_MAX_SENTENCES,
        help=f"Paragraph length threshold (default: {DEFAULT_MAX_SENTENCES})",
    )
    parser.add_argument(
        "--exit-zero", action="store_true",
        help="Always exit 0 (even with findings)",
    )
    args = parser.parse_args()

    roots = args.paths or DEFAULT_ROOTS
    paths = collect_paths(roots)
    findings = []
    for p in paths:
        findings.extend(scan_file(p, args.max_paragraph_sentences))

    if args.json:
        print(json.dumps(
            {
                "scanned_files": len(paths),
                "finding_count": len(findings),
                "findings": findings,
            },
            indent=2,
        ))
    else:
        sys.stdout.write(format_human(findings, len(paths)))

    sys.exit(0 if (not findings or args.exit_zero) else 1)


if __name__ == "__main__":
    main()
