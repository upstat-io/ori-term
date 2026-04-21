# `scripts/prose-lint.py` — Design Log

## Purpose + Context

`scripts/prose-lint.py` — lint authored `.md` files (skills, commands, rules, design logs) against the NO-PROSE rule.

- Global rule: `CLAUDE.md §NO PROSE IN AUTHORED .md FILES — ABSOLUTE`.
- Local rule: `.claude/skills/improve-tooling/SKILL.md §No Prose in Authored .md Files — ABSOLUTE`.
- Wraps the ban-phrase grep in that section with section-aware exemption logic + paragraph-length detection.

## §1 Core Design Philosophy

1. **Two detection modes: keyword + paragraph-length.** Keyword scan catches dated refs, history narrative, rationale tails. Paragraph-length catches prose-rot (multi-sentence rule bodies, rationale blocks).
2. **Section-aware exemption.** Design-log `## §4 Lessons` and `## §6 Improvement Log` allow prose — tool detects both by regex on `## §[46]\b` header.
3. **Per-file exemption for the rule-definition file itself.** `.claude/skills/improve-tooling/SKILL.md` contains banned-pattern literals as examples; tool excludes it wholesale.
4. **Opt-out directives.** `<!-- prose-lint: off -->` / `<!-- prose-lint: on -->` block markers; `<!-- prose-lint: allow -->` line marker.
5. **False-positive guards.** Hyphenated compound adjectives (`previously-failing`, `previously-completed`, etc.) suppressed via word list. State-label definitions (`**CONFIRMED** … previously …`) suppressed via adjacent-`previously` check.
6. **JSON output for programmatic consumers + human output for interactive.** Single script; no second binary.
7. **Default scope is `.claude/skills .claude/commands .claude/rules`** — covers all authored-.md surfaces without requiring the user to remember paths.

## §2 Load-Bearing Invariants

<!-- prose-lint: off -->

| Invariant | Failure mode it prevents |
|---|---|
| `SKILL.md` for `improve-tooling` is path-exempt. | Banned-pattern examples in the rule-definition file would generate permanent noise; tool becomes ignorable. |
| §4 Lessons and §6 Improvement Log in design logs are section-exempt. | Design logs are by contract "history book" files; stripping prose there deletes institutional memory. |
| Compound-adjective suppressor matches `previously-<past-participle>` from a known list. | Technical state-descriptor vocabulary (`previously-failing tests`, `previously-completed items`) is precise language, not prose history. |
| State-label suppressor matches `**LABEL**[^\n]*previously`. | CONFIRMED/REGRESSED/FIXED state-label tables literally define themselves using "previously X, now Y" — core vocabulary. |
| Keyword scan IS case-insensitive. | Sentence-start capitalization ("Previously" vs "previously") would slip past otherwise. |
| Fenced code blocks (` ``` `) are always exempt. | Code examples contain banned literal strings by necessity. |
| Exit code 1 on findings, 0 on clean. | CI/pre-commit integration needs a clean signal. |

<!-- prose-lint: on -->

## §3 File Inventory

| Path | Lines | Role |
|---|---|---|
| `scripts/prose-lint.py` | ~270 | The script: argparse, regexes, section-aware exemption, keyword scan, paragraph-length scan, JSON/human output. |
| `.claude/skills/improve-tooling/SKILL.md` §No Prose | ~60 | Rule definition + preferred invocation recipe. Canonical doc surface for the rule. |
| `CLAUDE.md` §Commands | 1 line | Discoverability entry. |
| This file (`script-prose-lint-design.md`) | ~150 | Design log. |

## §4 Lessons from Dogfood / Production Runs

### 2026-04-19 — Initial run surfaced 13 keyword hits + 357 paragraph hits

Dogfood on authored `.md` at creation time found 13 keyword violations and 357 paragraph-length violations across 69 of 100 scanned files. Of the 13 keyword hits:

- 10 were genuine violations → fixed (7 previously identified in pass-1 manual review + 3 new: `verify-roadmap.md:432`, `aims-rules.md:210`, `continue-roadmap-design.md:5`).
- 3 were false positives classified by the tool initially but belonged to the state-label definition pattern (`| **CONFIRMED** | Previously seen, still present |` in `code-journey/SCHEMA.md`). Tool regex was tightened in the same session — `STATE_LABEL_RE` changed from requiring em-dash separator to any-chars-same-line between the `**LABEL**` and `previously`. Re-run after refinement dropped the false positives correctly.

The 357 paragraph-too-long hits are a mix: some are genuine rationale-paragraph violations that should be converted to bullets; many are legitimate technical-spec prose in rules files (e.g., AIMS lattice dimension descriptions in `aims-rules.md`). The paragraph-length detector is most useful for catching rot in skills and commands files, less useful in rules files where multi-sentence rule bodies are often the intended form. No tuning has been done beyond the default `--max-paragraph-sentences=2` threshold; raising it to 3 or 4 would drop many rules-file hits but also miss real violations. Future work: emit findings tiered by confidence, perhaps based on proximity to bullet/table markers.

## §5 Regressions To Watch For

- [ ] `KEYWORD_PATTERNS` loses `re.IGNORECASE` flag — would miss sentence-start capitalization.
- [ ] `EXEMPT_HEADER_RE` narrowed to only one section (e.g., only §6 but not §4) — design-log §4 Lessons would start firing.
- [ ] `STATE_LABEL_RE` regressed to requiring em-dash — table-row state labels re-flag.
- [ ] `is_exempt_path()` drops the `improve-tooling/SKILL.md` special case — rule-definition file floods findings.
- [ ] Fence detector broken (e.g., counts `\`\`\`` inside a code span) — code examples re-flag.
- [ ] `COMPOUND_ADJ_RE` word list shrinks without replacement — technical state-descriptor vocabulary re-flags.
- [ ] `<!-- prose-lint: off|on -->` directive parsing broken — can't opt out of narrow sections.
- [ ] `--max-paragraph-sentences` default changed from 2 — rule threshold drift.

## §6 Improvement Log

### Open items

- [ ] [p2] Confidence tiering for paragraph-too-long findings: down-weight multi-sentence rule bodies in rules files; up-weight prose in skills/commands. Today's flat "paragraph > N sentences" surfaces too many legitimate technical descriptions.
- [ ] [p2] Table-cell scanner: a single line `| cell1 | cell2 that has three whole sentences. Like this. And this. |` currently evades paragraph-length detection (treated as a table row and skipped). Add a per-cell sentence count.
- [ ] [p3] Colorize human output (follow `diagnostics/_common.sh` convention — respect `--no-color` for CI).
- [ ] [p3] Add `--diff-mode` that only scans files changed in `git diff HEAD` — useful as pre-commit hook.
- [ ] [p3] Integrate into `/commit-push` Step 4 alongside `fmt-all.sh` + `plan-cleanup.py`.

### Recently closed

- [x] 2026-04-19 — **Initial implementation.** Script at `scripts/prose-lint.py`. Default scope: `.claude/skills .claude/commands .claude/rules`. Keyword patterns cover dated refs (`as of 20XX`, `since 20XX`), history keywords (`previously|originally|restoring|defeating`), rationale tails (`— causes`), and `was (originally|previously)` phrasing. Paragraph-length detection scans non-list/non-table/non-fence paragraphs and flags > 2 sentences by default. Exemptions: per-file (improve-tooling/SKILL.md, CHANGELOG.md, HISTORY.md), per-section (design-log §4/§6), per-region (lint off/on blocks), per-line (lint allow comments), per-pattern (hyphenated compound adjectives, state-label definitions). `--json` + `--human` output modes. Exit codes: 0 clean, 1 findings, 2 usage. `--exit-zero` for advisory runs. Initial dogfood: 13 keyword violations surfaced, 10 fixed; STATE_LABEL_RE tightened to catch table-row state labels; verified clean (0 keyword hits) post-fix. Commit: pending.
- [x] 2026-04-19 — **STATE_LABEL_RE generalization.** Original regex required `**LABEL** — previously` with em-dash separator; this missed table-row forms `| **LABEL** | Previously ... |`. Tightened to `\*\*(LABEL)\*\*[^\n]*\bpreviously\b` — matches any chars between the label and "previously" on the same line. Drops 3 false positives in `code-journey/SCHEMA.md`. Commit: pending.

## §7 How To Use This File In Future Sessions

Open this file before editing `scripts/prose-lint.py` or `.claude/skills/improve-tooling/SKILL.md §No Prose`. Check §2 invariants before loosening any suppressor (false negatives are worse than false positives — a violation that slips past is silent rot, a false positive is a 10-second triage). Check §5 regressions before releasing a change. Add a `- [x]` entry to §6 Recently closed on every modification with today's date + one-line description + commit sha.
