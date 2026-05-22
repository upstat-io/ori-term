#!/usr/bin/env python3
"""Generate release notes for oriterm releases.

Gathers commit log and PR descriptions, then generates structured release
notes using AI (Copilot SDK with Claude Sonnet 4.6) when available, falling
back to conventional-commit categorization.

Usage:
    # From git tag range (CI or local):
    ./scripts/generate-release-notes.py --tag v0.2.0-alpha.20260330

    # With explicit previous tag:
    ./scripts/generate-release-notes.py --tag v0.2.0-alpha.20260330 --prev v0.2.0-alpha.20260329

    # Output to file instead of stdout:
    ./scripts/generate-release-notes.py --tag v0.2.0-alpha.20260330 -o /tmp/notes.md

Environment:
    COPILOT_GITHUB_TOKEN  — enables AI generation (CI only)
    GH_TOKEN / GITHUB_TOKEN — needed for PR body fetching via `gh`
"""

import argparse
import os
import subprocess
import sys


def run(cmd, check=True):
    """Run a shell command and return stdout."""
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    if check and result.returncode != 0:
        return ""
    return result.stdout.strip()


def get_prev_tag(current_tag):
    """Find the most recent tag before current_tag."""
    tags = run("git tag --sort=-creatordate | grep '^v'")
    if not tags:
        return ""
    for tag in tags.split("\n"):
        tag = tag.strip()
        if tag and tag != current_tag:
            return tag
    return ""


def gather_commit_log(prev_tag, current_tag):
    """Get commit log between two tags. No truncation — every commit feeds the prompt."""
    if prev_tag:
        return run(f'git log "{prev_tag}..HEAD" --pretty=format:"- %s (%h)" --no-merges')
    return run('git log --pretty=format:"- %s (%h)" --no-merges -500')


def gather_pr_bodies(prev_tag):
    """Fetch merged PR descriptions via gh CLI. High limit so nothing is dropped silently."""
    if prev_tag:
        prev_date = run(f'git log -1 --format=%aI "{prev_tag}"')
        if prev_date:
            return run(
                f'gh pr list --state merged --base main --limit 500 '
                f'--json number,title,body,mergedAt '
                f'--jq \'[.[] | select(.mergedAt >= "{prev_date}")] | .[] | '
                f'"## PR #\\(.number): \\(.title)\\n\\(.body // "(no description)")\\n"\''
            )
    return run(
        'gh pr list --state merged --base main --limit 500 '
        '--json number,title,body '
        '--jq \'.[] | "## PR #\\(.number): \\(.title)\\n\\(.body // "(no description)")\\n"\''
    )


def generate_ai_notes(tag, prev_tag, commit_log, pr_bodies):
    """Try AI generation via Copilot SDK. Returns notes or None on failure."""
    if not os.environ.get("COPILOT_GITHUB_TOKEN"):
        return None

    prompt = f"""You are writing PUBLIC release notes for **oriterm** ({tag}), a GPU-accelerated terminal emulator built in Rust. oriterm is in alpha. The audience is end users and developers who use terminal emulators daily and care about performance, rendering correctness, graphics protocol support, and keyboard-driven workflows. These notes will be published on the GitHub release page and the oriterm.com changelog — they are read by people who have NEVER seen our internal plans, bug tracker, or codebase layout.

Take your time. Read EVERY pull request description and EVERY commit line in full before you write a single bullet. There is no time pressure. Accuracy, completeness, and clarity matter far more than brevity. If you are unsure whether to include a change, include it. Re-read the inputs before finalizing.

## Output format

Output clean markdown. Do NOT wrap the response in ```markdown fences. Do NOT include a top-level title — the GitHub release UI supplies one.

Structure the output in this order, omitting any empty sections:

1. **Summary** — one short paragraph (1-3 sentences) describing the theme of this release. Lead with the biggest user-facing wins.
2. **Highlights** — if 3+ standout changes exist, list 3-6 of them as one-sentence bullets at the top.
3. **Breaking changes** — if any config keys, keybindings, CLI flags, or default behaviors changed in a way that may affect existing users, list them here with the prefix `**Breaking:**` and an explicit migration sentence per bullet.
4. **Categorized changes** — every distinct user-facing change goes here, grouped by category. Use these section headers exactly; omit empty ones:
   - **Features** — new user-facing capabilities
   - **Rendering** — GPU pipeline, glyph atlas, shaders, color accuracy, font rendering, ligatures, emoji, cursor rendering
   - **Terminal Emulation** — VT parsing, grid, scrollback, reflow, escape sequences, sixel graphics, kitty graphics, iTerm2 graphics, OSC handling, mouse reporting modes, keyboard protocols
   - **UI** — widgets, window chrome, tab bar, status bar, dialogs, themes, scrollbar, command palette
   - **Input** — keyboard handling, mouse events, selection, clipboard, IME, keybindings, hyperlinks
   - **Multiplexer & PTY** — pane management, splits, PTY I/O, session model, process handling
   - **Performance** — latency, memory, CPU usage, startup time, allocation reduction (quantify with concrete numbers whenever the data is in the inputs)
   - **Configuration** — user-facing config additions, renames, removals, default changes
   - **Bug Fixes** — behavior that was broken in a previous release and now works correctly
   - **Platform** — Windows, macOS, Linux specific changes
   - **Documentation** — user-facing docs only (skip internal plan/design docs)
5. **Known issues** — only if any are evident in the inputs. Short bullets. Never fabricate.
6. **What's Next** — only if a clear, data-supported theme emerges from the inputs. 2-3 short bullets. Never invent roadmap items not present in the data.

## Bullet style

For each bullet:
- Lead with a **bold action clause** stating WHAT changed, then a sentence (or two) on WHY it matters to the user
- Past tense action verbs: Added, Fixed, Improved, Changed, Removed, Optimized, Renamed
- User-impact framing: describe the user-visible effect, never the implementation
- Quantify performance work whenever the inputs supply numbers (e.g. "30% faster startup", "12ms → 4ms input latency", "Reduced idle CPU from 8% to 1%")
- Plain English. No jargon, no internal vocabulary, no acronyms the user has not seen before in the product

## ABSOLUTE BAN — never expose internal references

The PR descriptions and commit messages contain heavy internal-process vocabulary. STRIP ALL OF IT. A reader outside the project must understand every bullet with zero context about how oriterm is organized internally.

NEVER include ANY of the following in any bullet, heading, summary, or sentence:

- **Plan references** — "Section 38", "§38.4", "Section NN", "subsection X.Y", "Phase 1", "Phase 1.75", "Tier 0/1/2/3/4/5/6/7/8", "Tier 4M", roadmap section numbers, plan directory names, "plans/<anything>"
- **Bug IDs** — `BUG-XX-NNN` patterns (e.g. `BUG-06-086`, `BUG-11-028`), `bug-tracker/...`, any bracketed identifier from the issue tracker. Refer to a bug by what it was, in plain English ("the kitty XTVERSION crash on Linux"), never by its tracker code.
- **PR numbers in bullet prose** — the GitHub release UI links these automatically. Do not bake `#123` or `PR #456` into the bullet text.
- **Internal crate names** — `oriterm_core`, `oriterm_ui`, `oriterm_mux`, `oriterm_ipc`, `oriterm_test_support`, `oriterm` (the crate). Say "the terminal", "the UI", "the multiplexer", "the test harness" instead.
- **Dependency names** — `wgpu`, `winit`, `rustybuzz`, `swash`, `vte`, `portable-pty`, `fontations`, `skrifa`, `read-fonts`. Say "GPU rendering", "windowing", "text shaping", "font rasterization", "VT parsing", "pseudo-terminal layer" instead.
- **Rust types** — `FairMutex`, `Arc<Grid>`, `Vec<Cell>`, `ProcessModel`, `WidgetTestHarness`, `SpecHarness`. Describe the behavior, not the type.
- **Pipeline / phase / rung vocabulary** — "Parser → Dispatch → State → Renderable → FrameInput → GpuInstance → TextureRender → GoldenImage", "rung 3", "Phase 2.5", "the spec chain", "the visual harness"
- **File paths** — `src/...`, `crates/...`, `term_repo/...`, `plans/...`, anything that looks like a path
- **Process artifacts** — "TPR findings", "TPR cycle", "impl-hygiene review", "RCA cycle", "matrix coverage", "Mikado leaf", "rerouted plan", "amended plan", "resume_pointer", "block-gate", "section close", "subsection close", "Phase N close", "reviewed: true/false"
- **Tracker / planning vocabulary** — "deferred", "blocked-by", "tracked for later", "in-progress section", "active subsection"

### Mapping examples — apply this discipline mechanically

| Internal phrasing (in the inputs) | Public bullet (in your output) |
|---|---|
| "fix(BUG-06-086): kitty XTVERSION crash cycle 3" | **Fixed kitty graphics XTVERSION crash on Linux** — querying terminal version no longer terminates the session under affected compositors. |
| "Section 12.3 — implement OSC 52 clipboard sync" | **Added OSC 52 clipboard sync** — applications running inside oriterm can now read from and write to the system clipboard using the standard OSC 52 sequence. |
| "Phase 4 TPR findings landed for §38 sixel" | **Improved sixel image rendering** — fixed color accuracy and palette handling for sixel graphics. |
| "BUG-11-028 close — rerouted to plans/clippy-gate-hardening §3" | (omit — internal hygiene, no user impact) |
| "feat(oriterm_ui): WidgetTestHarness for headless interaction tests" | (omit — internal test infrastructure) |

If a PR description is dominated by internal process language (TPR rounds, RCA cycles, plan resume pointers, hygiene findings), extract ONLY the user-visible deliverable from it and discard the rest. If after stripping internal process there is no user-visible deliverable left, omit the entry entirely.

## DO NOT truncate

**List every distinct user-facing change as its own bullet.** Do NOT consolidate, summarize, or curate down to a "tasteful" subset. If the release contains 40 distinct user-facing changes, the notes have 40 bullets. If it contains 100, the notes have 100. There is no upper limit. Length is not a problem; missing changes is.

The ONLY merge allowed: when two or more commits/PRs describe the *same* deliverable (e.g. an initial implementation plus follow-up fixes against the same feature, or a multi-commit feature broken up for review), merge them into ONE bullet that describes the final delivered behavior. "Related" does NOT mean "the same" — keep distinct features as distinct bullets and distinct bug fixes as distinct bullets.

If a single PR delivers multiple independent user-facing changes (e.g. "Added kitty graphics support, fixed selection wrapping, improved scrollback memory"), SPLIT it into multiple bullets — one per distinct change.

When in doubt about whether two items are "the same deliverable" or "two distinct deliverables", treat them as distinct and write two bullets.

## Omit only true noise

Skip ONLY these:

- Version-bump commits (`chore: release vX.Y.Z`, `chore(release):`, lockfile bumps)
- Nightly / CI automation PRs that change no production code
- Pure refactoring with ZERO user-visible effect. Be conservative: if a refactor improved performance, fixed a latent bug, or unblocked a feature, KEEP it and describe the user-visible effect.
- Internal tooling, plan-corpus scripts, diagnostics, hygiene scripts, hooks, prose linting — none of this ships to users
- Internal documentation, design docs, plan files, retrospectives, TPR/RCA artifacts
- Test additions, unless they represent a brand-new TESTING CAPABILITY exposed to users (extremely rare for an end-user terminal)

When in doubt, include rather than omit.

## Final self-check before emitting output

Before you finalize, scan your draft for:

1. Any string matching `BUG-`, `§`, `Section NN`, `Phase`, `Tier`, `Mikado`, `TPR`, `RCA`, `plans/`, `bug-tracker/`, `subsection`, `resume_pointer`, `reviewed:`, `oriterm_` (as a crate name), `wgpu`, `winit`, `vte`, `rustybuzz`, `swash`, `portable-pty`, `crates/`, `term_repo/` — if any are present, rewrite the bullet.
2. Any bullet whose meaning would be lost on a reader who has never seen the codebase — rewrite or omit.
3. Whether you have one bullet per distinct user-facing change. If you consolidated for taste rather than because they were the same deliverable, split them back apart.
4. Whether every performance claim has a number from the inputs (not invented).
5. Whether every "What's Next" item is supported by evidence in the inputs (not invented).

## Input

Pull request descriptions (primary source — read every one in full):

{pr_bodies}

Commit log ({prev_tag or 'beginning'}..{tag}):

{commit_log}

Now write the release notes. Take as long as you need."""

    try:
        import asyncio

        async def _generate():
            from copilot import CopilotClient
            from copilot.session import Kind, PermissionRequestResult

            def approve_all(_request, _metadata):
                return PermissionRequestResult(kind=Kind.APPROVED)

            client = CopilotClient()
            await client.start()
            try:
                session = await client.create_session(
                    model="claude-sonnet-4.6",
                    streaming=False,
                    on_permission_request=approve_all,
                )
                reply = await session.send_and_wait(prompt, timeout=900.0)
                if reply is None:
                    return None
                text = reply.data.content
                if not text or len(text) < 50:
                    print(f"AI returned suspiciously short response ({text!r}), using fallback", file=sys.stderr)
                    return None
                return text
            finally:
                await client.stop()

        return asyncio.run(_generate())
    except Exception as e:
        print(f"AI generation failed ({e}), using fallback", file=sys.stderr)
        return None


def generate_fallback_notes(tag, commit_log):
    """Categorize commits by conventional commit prefix."""
    sections = {
        "Rendering": [],
        "Terminal Emulation": [],
        "UI": [],
        "Performance": [],
        "Bug Fixes": [],
        "Other": [],
    }

    for line in commit_log.strip().split("\n"):
        line = line.strip()
        if not line or not line.startswith("- "):
            continue
        subject = line[2:]  # strip "- " prefix
        # Skip version bumps and automation
        if subject.startswith("chore: release") or subject.startswith("chore(release)"):
            continue
        if subject.startswith("feat"):
            # Route by parenthetical scope if present
            if "(gpu)" in subject or "(render)" in subject or "(font)" in subject:
                sections["Rendering"].append(line)
            elif "(ui)" in subject or "(widget)" in subject:
                sections["UI"].append(line)
            elif "(term)" in subject or "(grid)" in subject or "(vte)" in subject:
                sections["Terminal Emulation"].append(line)
            else:
                sections["UI"].append(line)
        elif subject.startswith("fix"):
            sections["Bug Fixes"].append(line)
        elif subject.startswith("perf"):
            sections["Performance"].append(line)
        elif subject.startswith(("refactor", "chore", "build", "ci", "test", "style", "docs")):
            continue  # omit housekeeping in fallback mode
        else:
            sections["Other"].append(line)

    body = ""
    for section, items in sections.items():
        if items:
            body += f"## {section}\n\n" + "\n".join(items) + "\n\n"

    if not body.strip():
        body = f"## Changes\n\n{commit_log}"

    return body.strip()


def main():
    parser = argparse.ArgumentParser(description="Generate oriterm release notes")
    parser.add_argument("--tag", required=True, help="Release tag (e.g., v0.2.0-alpha.20260330)")
    parser.add_argument("--prev", default=None, help="Previous tag (auto-detected if omitted)")
    parser.add_argument("-o", "--output", default=None, help="Output file (stdout if omitted)")
    args = parser.parse_args()

    prev_tag = args.prev or get_prev_tag(args.tag)
    commit_log = gather_commit_log(prev_tag, args.tag)
    pr_bodies = gather_pr_bodies(prev_tag)

    # Try AI, fall back to structured categorization
    notes = generate_ai_notes(args.tag, prev_tag, commit_log, pr_bodies)
    if not notes:
        notes = generate_fallback_notes(args.tag, commit_log)

    if args.output:
        with open(args.output, "w") as f:
            f.write(notes)
        print(f"Release notes written to {args.output}", file=sys.stderr)
    else:
        print(notes)


if __name__ == "__main__":
    main()
