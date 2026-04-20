<!-- prose-lint: off -->

# Sync-Docs Semantic Audit Ledger

Append-only record of Phase 0.7 runs. One entry per nightly run of `/sync-docs`. Ledger is exempt from `scripts/prose-lint.py` per §"The Semantic Audit Ledger is Sync-Internal State" (design log I18).

Schema per entry (see `cmd-sync-docs-design.md` §Audit Ledger for the canonical definition):

```
## YYYY-MM-DD — Focus: <focus-name> (index <N>)

- **Mission thesis**: <one sentence>
- **Realized-state summary**: <one sentence naming what works + what doesn't>
- **Gap identified**: <one sentence naming the smallest single step toward realization>
- **Path chosen**: (a) factual-edit | (b) add-bug | (c) proposal | (d) no-op
- **Artifact**: <commit sha | BUG-XX-NNN | proposal path | "none — audit entry only">
- **Reasoning (≤3 sentences)**: <why this path, why this scope>
```

Path meanings: (a) factual correction to a rules file; (b) `/add-bug` on code where mission is realizable but unrealized; (c) `/create-draft-proposal` where the mission/spec/grammar itself needs change; (d) no-op audit-only entry (mission realized OR no single-night tiny change identifiable).

Intent: weekly/monthly review of this ledger surfaces the cumulative trajectory of mission realization across all focus areas. Each entry is a single tiny step; the ledger is the record of the walk.

---

<!-- Phase 0.7 entries appended below this marker, newest-first -->

## 2026-04-19 — Focus: typeck / inference (index 1)

- **Mission thesis**: `ori_types` proves before the next phase that every expression has one resolved type and every trait dispatch a known target, with primitive dispatch routed through the registry — never hardcoded — to preserve the unified trait/capability model.
- **Realized-state summary**: Dispatch lookup through the registry is active (`crates/types/src/infer/expr/operators.rs:16` `infer_binary`), but the structural rewrite `a + b` → `Add::add(a, b)` is NOT shipped — `crates/canon/src/lower/expr.rs:108-111` preserves `CanExpr::Binary { op, left, right }` and the AST retains `Binary` nodes into canonicalization.
- **Gap identified**: `canon.md §2` row 2 labels "Binary operator trait dispatch" as `Shipped`, but both `missions.md §ori_types` and the code reality say the structural rewrite is not yet shipped. Smallest single change: flip row 2's Status cell from `Shipped` to `**Target-only**` to match mission + code.
- **Path chosen**: (a) factual-edit
- **Artifact**: commit on worktree branch `worktree-sync-docs-2026-04-19` (82c2426f)
- **Reasoning**: Rows 3-4 (index/field assignment) share the same class of drift but are separate mission bullets not named in `§ori_types`; scoping tonight's output to row 2 keeps the "ONE output" rule honest, and Batch 1 (canon) caught rows 3-4 within this same sync run.
<!-- prose-lint: on -->
