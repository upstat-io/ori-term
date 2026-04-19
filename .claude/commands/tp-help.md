---
name: tp-help
description: "Get third-party help from Codex + Gemini (dual-source concat mode). Use this proactively when stuck on a problem, need a second opinion, want help debugging, or want to verify reasoning. Returns BOTH reviewers' raw responses concatenated (not a synthesis). Full auto-trigger conditions and workflow live in the canonical skill file."
allowed-tools: Bash, Read, Grep, Glob, Skill
argument-hint: "[question or context]"
---

# /tp-help — Third-Party Help (Codex + Gemini, Dual-Source Concat Mode)

The canonical implementation of `/tp-help` lives in the skill file at
`.claude/skills/tp-help/SKILL.md`. When the `/tp-help` slash command
is invoked, load and follow that skill file exactly.

`/tp-help` is a **thin delegator** over `/tpr-review --help-mode
--max-rounds=1`. It gathers the question + context, then invokes
`/tpr-review` via the `Skill` tool with the help-mode flag set — which
dispatches codex + gemini in parallel through the canonical
`tp_agent_prompt.md` transport, skips finding verification / filing /
fix-and-commit (advice is not a bug), and returns both reviewers' raw
responses concatenated with HTML-comment sentinel attribution. This is
intentionally different from `/tpr-review`'s default review-mode
(iterative loop with finding verification) — help mode is a one-shot
consultation (raw perspectives from two models) rather than a review.

See `.claude/skills/tp-help/SKILL.md` for:
- Auto-trigger conditions (8 concrete triggers + negative examples).
- Context assembly into the composed objective.
- The `Skill({skill: "tpr-review", args: "--help-mode --max-rounds=1 ..."})`
  delegation pattern.
- Output pass-through rendering and the "apply the output" judgment
  guidance.

The underlying dispatch (parallel Agent sub-agents, codex/gemini CLI
invocation, scratch-dir management, sentinel extraction) lives in
`/tpr-review`'s canonical machinery — see
`.claude/skills/tpr-review/SKILL.md` §1 `--help-mode` flag and §5
"Help-mode branch," plus `.claude/skills/tpr-review/compose-round-prompt.md`
"Help-mode body."

This file is a thin pointer maintained to preserve the slash-command
dispatcher contract (`name`, `description`, `allowed-tools`,
`argument-hint`). All operational content lives in the skill file to
satisfy the single-source-of-truth rule (`.claude/rules/impl-hygiene.md
§SSOT`).
