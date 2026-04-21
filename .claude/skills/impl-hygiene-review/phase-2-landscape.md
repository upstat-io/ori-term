# Phase 2 — Map the Full Landscape

Read by a Sonnet sub-agent dispatched from `/impl-hygiene-review`. Not a registered skill. Uses _(intel-query not available in this project; use Grep/Glob)_ (blast radius, module inventory, cross-repo similarity) to build a landscape map of the review target before Phase 3 deep analysis.

Writes `{run_id}/phase-2.json` (the orchestrator-owned scratch dir passed in via the sub-agent prompt) with: call graph edges for in-scope symbols, file-symbol inventory per crate, cross-repo equivalents for architectural patterns.

---

### Phase 2: Map the Full Landscape

Before diving into findings, build a high-level map of the review scope. This is the "go wide" phase — understand the shape before probing the details.

#### 2a. Identify Review Targets

Determine the distinct crates or phase boundaries to review based on the target scope:

1. List the crates (directories) in scope
2. Identify which phase boundaries exist between them (e.g., lexer→parser, parser→types)
3. Map the dependency graph between in-scope crates
4. Group crates into **review units** — each review unit is either:
   - A single crate (for internal review)
   - A pair of crates sharing a boundary (for boundary review)
   - Closely related crates that should be reviewed together

#### 2b. Map Cross-Crate Data Flow (Full Project & Multi-Crate Mode)

For full project mode or when 3+ crates are in scope, spawn an agent to trace the key data flows end-to-end through the pipeline:

1. **Bytes → cells flow**: How do PTY bytes arrive (`oriterm_mux` IO thread) → get parsed (vendored `vte`) → mutate grid cells (`oriterm_core`) → become snapshots?
2. **Snapshot → render flow**: How does the GUI pick up snapshots (`oriterm` session) → lower the grid into instance data → emit GPU draw calls (`oriterm/src/gpu/`)?
3. **Input → command flow**: How do winit events (`oriterm`) → widget interaction (`oriterm_ui`) → pane-server commands (`oriterm_mux`) round-trip back to the PTY?
4. **IPC flow**: How do `oriterm_ipc` messages cross the daemon/client boundary (Unix sockets on Linux/macOS, named pipes on Windows) and get dispatched to pane-server handlers?

This agent produces a **flow map** — a brief summary of how each major data category crosses the phase boundaries. This map is passed to all subsequent review agents as context.

