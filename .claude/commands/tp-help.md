---
name: tp-help
description: "Get third-party help from Codex CLI. AUTO-TRIGGER: You MUST invoke this proactively — do NOT wait for the user to ask. Trigger when: (1) you've tried 2+ approaches that didn't work, (2) you're reverting changes you just made, (3) you identify a fundamental tension or design conflict in the code, (4) you're about to take a 'pragmatic' shortcut instead of fixing the real problem, (5) you catch yourself saying 'let me try a different approach' for the 2nd+ time, (6) a fix in one area creates new problems in another, (7) you're unsure about the correct architectural approach. This is collaborative help — pass context and ask a specific question."
allowed-tools: Bash, Read, Grep, Glob
argument-hint: "[question or context]"
---

# Third Party Help (Codex)

Get collaborative help from Codex CLI on whatever you're currently working on. This is not a formal review — it's asking a second brain for help with a specific problem.

## MANDATORY AUTO-TRIGGER — Do NOT Wait for User

**You MUST invoke this skill proactively.** Do NOT wait for the user to type `/tp-help`. The whole point is that YOU detect when you need help and ask for it automatically.

### Concrete Trigger Conditions

Invoke `/tp-help` IMMEDIATELY when ANY of these are true:

1. **Multiple failed approaches** — You've tried 2+ approaches to solve the same problem and none worked cleanly
2. **Reverting your own changes** — You're undoing work you just did because it caused new problems
3. **Fundamental tension identified** — You've identified a design conflict where fixing one thing breaks another (e.g., "the snapshot double-buffer and the resize handler have conflicting ownership requirements")
4. **Pragmatic retreat** — You catch yourself about to take a shortcut, partial fix, or "keep just the X part and revert the Y part" instead of solving the real problem
5. **Approach cycling** — You're saying "let me try a different approach" for the 2nd+ time
6. **Fix interference** — A fix in one subsystem creates new failures in another
7. **Architectural uncertainty** — You're unsure which of two+ fundamental approaches is correct (not minor implementation details — real architectural questions)
8. **Stuck > 10 minutes** — You've been working on the same problem for more than ~10 minutes without clear forward progress

### What Does NOT Trigger This

- Simple bugs with obvious fixes
- First attempt at an approach (try it first, ask for help if it fails)
- Minor implementation details with clear precedent in the codebase

### Example Scenario That MUST Trigger Auto-Invoke

> "I've been trying multiple approaches but the resize handler races with the snapshot flip and the GPU surface reconfigure. The ownership model for the double-buffer has a fundamental tension between the IO thread and the main thread. Let me take the pragmatic approach: just skip the frame when the sizes mismatch."

This hits triggers #1 (multiple approaches), #3 (fundamental tension), #4 (pragmatic retreat), and #2 (reverting). You should have invoked `/tp-help` BEFORE reaching the "let me take the pragmatic approach" conclusion.

## Legacy Trigger List (still valid)

- You're stuck on a bug and can't figure out the root cause
- You're unsure which of two implementation approaches is better
- You just wrote something tricky and want a sanity check
- A test is failing and you can't see why
- You need help understanding unfamiliar code
- You want to validate your reasoning before committing to an approach
- You're about to make a significant architectural decision

## Usage

```
/tp-help [question]
```

Can also be invoked proactively by Claude when it determines outside help would be valuable.

## Workflow

### Step 1: Build Context Package

Gather the relevant context for the question. Be specific — Codex works best with concrete context, not vague requests.

**Always include:**
- The specific question or problem
- The file(s) involved (read them and include key sections)

**Include when relevant:**
- The error message or test failure output
- What you've already tried
- The two approaches you're deciding between
- Recent git diff showing what you changed

### Step 2: Format the Prompt

Build a prompt that gives Codex everything it needs in one shot:

```
You are helping with ori_term, a GPU-accelerated terminal emulator in Rust (wgpu, winit, cross-platform: macOS/Windows/Linux).

## Question
{The specific question or problem}

## Context
{Key file contents, error messages, diffs — whatever is relevant}

## What I've Tried
{If applicable — what approaches were attempted and why they didn't work}

## Constraints
{Any rules from CLAUDE.md or .claude/rules/ that apply — e.g., "no workarounds, must be architecturally correct", "crate boundary: this must live in oriterm_ui not oriterm"}
```

### Step 3: Call Codex via Bash in background

Run codex directly via the Bash tool with `run_in_background: true`. The
2-minute foreground default cap does not apply to background tasks. You
will receive a completion notification when codex finishes (typically
5-15 minutes).

Write a prompt file first so heredocs/quoting don't fight shell escaping:

```
Write '/tmp/tp-help-prompt.md' with the full question + context package.
```

Then launch codex in the background:

```
Bash (run_in_background: true):
  rm -f /tmp/tp-help.jsonl /tmp/tp-help.done
  codex exec "$(cat /tmp/tp-help-prompt.md)" --full-auto --json 2>/dev/null > /tmp/tp-help.jsonl
  ec=$?
  touch /tmp/tp-help.done
  echo "exit=$ec"
```

Continue working or wait idle. When the completion notification arrives,
parse the JSONL output for `agent_message` items:

```
Bash:
  python3 -c "
  import json
  with open('/tmp/tp-help.jsonl') as f:
      for line in f:
          line = line.strip()
          if not line:
              continue
          try:
              obj = json.loads(line)
              if obj.get('type') == 'item.completed' and obj.get('item', {}).get('type') == 'agent_message':
                  print(obj['item']['text'])
                  print()
          except json.JSONDecodeError:
              pass
  "
```

**DO NOT:**
- Run `codex exec` in the Bash foreground (will hit the 2-minute default
  timeout or get auto-backgrounded; either way output may be truncated).
- Wrap codex in an Agent subagent — the Agent adds no value over direct
  background Bash, costs an extra process, and the Agent cannot be
  `run_in_background: true` so it can't wait longer than the harness cap.
- Set a `timeout:` parameter on the Bash call (backgrounding is the
  preferred path; foreground timeouts will still hit the harness cap).
- Inline the full prompt in the Bash command — shell escaping of multi-
  line markdown is fragile; write to a file and `cat` it instead.

### Step 4: Apply the Answer

- Evaluate Codex's response against CLAUDE.md rules before applying
- You have full project context that Codex doesn't — use your judgment to filter
- If Codex disagrees with your approach, present both perspectives to the user

### Step 5: Brief the User

Tell the user:
- What you asked Codex
- What Codex said (brief summary)
- How you're applying it (or why you're not)
