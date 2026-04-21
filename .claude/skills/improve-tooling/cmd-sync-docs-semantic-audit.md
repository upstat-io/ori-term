# Sync-Docs Semantic Audit Ledger

<!-- prose-lint: off -->

## 2026-04-21 — Focus: codegen / LLVM (index 3)

- **Mission thesis**: The codegen layer faithfully translates realized ARC IR to LLVM IR using deterministic type resolution, ABI classification, narrowed storage boundaries, trampolined closures, and a C-ABI runtime contract — where every invariant is either always-on or opt-in-gated, never assumed.
- **Realized-state summary**: RT-1 (may-unwind set of 8 functions), RT-5 (MAX_ELEM_SIZE = 256), TM-1 (four trampoline variants), canon.md §1 cross-crate note (`ori_canon::patterns` delegating to `ori_arc::decision_tree`) all check out verbatim against current source; documented gaps (AT-5 RL-29/30/31 target-system rules, IT-2 BUG-04-076 flat_map stride) are explicitly flagged as target-only or bug-tracked.
- **Gap identified**: None this rotation — the rule file's factual claims are accurate and its acknowledged gaps are already tracked.
- **Path chosen**: (d) no-op
- **Artifact**: none — audit entry only
- **Reasoning**: Tier-A fallback (missions.md absent from snapshot) to codegen-rules.md / llvm.md / aot.md / repr.md. Every spot-checked mandatory claim (`SHALL`, §10 interface) matches code. AT-5 RL-29/30/31 are explicitly target-only ("not yet shipped"). IT-2 flat_map stride is BUG-04-076 (already in bug tracker). Forcing a path (a/b/c) output tonight would produce a non-tiny change; correct behaviour per Phase 0.7 contract is to log the no-op and wait for the next codegen rotation (12 days).
