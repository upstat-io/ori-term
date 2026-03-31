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
    """Get commit log between two tags."""
    if prev_tag:
        return run(f'git log "{prev_tag}..HEAD" --pretty=format:"- %s (%h)" --no-merges')
    return run('git log --pretty=format:"- %s (%h)" --no-merges -20')


def gather_pr_bodies(prev_tag):
    """Fetch merged PR descriptions via gh CLI."""
    if prev_tag:
        prev_date = run(f'git log -1 --format=%aI "{prev_tag}"')
        if prev_date:
            return run(
                f'gh pr list --state merged --base main --limit 20 '
                f'--json number,title,body,mergedAt '
                f'--jq \'[.[] | select(.mergedAt >= "{prev_date}")] | .[] | '
                f'"## PR #\\(.number): \\(.title)\\n\\(.body // "(no description)")\\n"\''
            )
    return run(
        'gh pr list --state merged --base main --limit 5 '
        '--json number,title,body '
        '--jq \'.[] | "## PR #\\(.number): \\(.title)\\n\\(.body // "(no description)")\\n"\''
    )


def generate_ai_notes(tag, prev_tag, commit_log, pr_bodies):
    """Try AI generation via Copilot SDK. Returns notes or None on failure."""
    if not os.environ.get("COPILOT_GITHUB_TOKEN"):
        return None

    prompt = f"""You are writing release notes for **oriterm** ({tag}), a GPU-accelerated terminal emulator built in Rust with wgpu. It is in alpha stage. The audience is developers who use terminal emulators daily and care about performance, rendering correctness, and keyboard-driven workflows.

Write clear, curated release notes in markdown. Do NOT wrap in ```markdown fences.

## Format

Start with a 1-3 sentence summary describing the theme of this release.

Then group changes into sections (omit empty ones):
- **Rendering** — GPU pipeline, glyph atlas, shaders, text rendering, color accuracy
- **Terminal Emulation** — VTE handling, grid, scrollback, reflow, escape sequence support
- **UI** — widgets, window chrome, tab bar, status bar, dialogs, keyboard shortcuts
- **Input** — keyboard handling, mouse events, selection, clipboard, IME
- **Multiplexer** — pane management, PTY I/O, splits, session persistence
- **Performance** — latency, memory, CPU usage, allocation reduction
- **Bug Fixes** — corrected behavior that was broken in a previous release
- **Platform** — Windows, macOS, Linux-specific changes, cross-compilation

End with a brief **What's Next** section — 2-3 bullet points on near-term focus areas, derived from the commit log and PR descriptions.

For each bullet:
- **Bold title** followed by 1-2 sentences explaining what changed and why it matters to users
- Use past tense ("Added", "Fixed", "Improved")
- Frame everything through user impact — if a change has no user-visible effect, omit it entirely

## Core Principle: Deliverables, Not Process

Release notes describe what was DELIVERED, not the process of delivery. Review feedback, iterative refinements, and internal cleanup are part of delivering a feature correctly — not separate items.

Apply this to ALL process artifacts:
- **Internal plan references** — strip section numbers, plan IDs, and internal tracking codes. Use plain English.
- **Iterative refinements** — hardening, edge case fixes, and polish done during development are part of the feature, not separate items.
- **Test additions** — only mention if they represent a new testing capability (e.g., headless widget testing), not routine test coverage.

**The test**: would an outside contributor who has never seen our plans or internal tracking understand every bullet? If not, rewrite it.

## Rules
- The PR descriptions are your PRIMARY source — they contain human-written summaries of what changed and why
- The commit log is supplementary — use it to catch anything the PRs missed
- Curate ruthlessly — 4 meaningful bullets beats 12 granular ones. Combine related work into single entries.
- Never dump the git log — every entry must be written for humans
- Never say "Internal improvements and maintenance" — if it matters, describe the impact; if it doesn't, omit it
- Skip version-bump commits and nightly automation PRs
- Do not reproduce test plan checklists
- Quantify performance improvements when data is available ("2.3x faster", "12ms to 4ms")
- If a change has no user-visible effect (pure refactoring, internal code movement), omit it entirely

## Input

Pull request descriptions (primary source):
{pr_bodies}

Commit log ({prev_tag or 'beginning'}..{tag}):
{commit_log}"""

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
                reply = await session.send_and_wait(prompt, timeout=120.0)
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
