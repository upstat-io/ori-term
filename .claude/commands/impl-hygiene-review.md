---
name: impl-hygiene-review
description: Review implementation hygiene at module boundaries — plumbing quality and file organization.
allowed-tools: Read, Grep, Glob, Task, Bash, EnterPlanMode
---

# Implementation Hygiene Review

Review implementation hygiene against `.claude/rules/impl-hygiene.md` and generate a plan to fix violations.

**Implementation hygiene is NOT architecture** (design decisions are made). It covers the full plumbing layer — module boundaries, data flow, error propagation, rendering discipline, file organization, naming, comments, visibility, and lint discipline.

## Target

`$ARGUMENTS` specifies the boundary or scope to review. **If empty or blank, default to last commit mode** (equivalent to `/impl-hygiene-review last commit`). Otherwise, there are two modes:

### Path Mode (explicit file/directory targets)
- `/impl-hygiene-review src/grid src/term_handler.rs` — review grid<>VTE handler boundary
- `/impl-hygiene-review src/gpu/` — review GPU rendering internals
- `/impl-hygiene-review src/app.rs src/tab.rs` — review app<>tab boundary
- `/impl-hygiene-review src/` — review all module boundaries

### Commit Mode (use a commit as a scope selector)
- `/impl-hygiene-review last commit` — review files touched by the most recent commit
- `/impl-hygiene-review last 3 commits` — review files touched by the last N commits
- `/impl-hygiene-review <commit-hash>` — review files touched by a specific commit

**CRITICAL: Commits are scope selectors, NOT content filters.** The commit determines WHICH files and areas to review. Once the files are identified, review them completely — report ALL hygiene findings in those files, regardless of whether the finding is "related to" or "caused by" the commit. The commit is a lens to focus on a region of the codebase, nothing more. Do NOT annotate findings with whether they relate to the commit. Do NOT deprioritize or exclude findings because they predate the commit.

**Commit scoping procedure:**
1. Use `git diff --name-only HEAD~N..HEAD` (or appropriate range) to get the list of changed `.rs` files
2. Expand to include the full module(s) those files belong to (e.g., if `oriterm/src/gpu/atlas.rs` was touched, include all of `oriterm/src/gpu/`)
3. Proceed with the standard review process using those modules as the target

## Execution

### Step 1: Load Rules

Read `.claude/rules/impl-hygiene.md` to have the full rule set in context.

### Step 2: Load Plan Context

Gather context from active and recently-modified plan files so the review doesn't flag work that is already planned, in-progress, or intentionally deferred.

**Procedure:**
1. Run `git diff --name-only HEAD` and `git diff --name-only --cached` to find uncommitted modified files in `plans/`
2. Run `git diff --name-only HEAD~3..HEAD -- plans/` to find plan files changed in recent commits
3. Combine both lists (deduplicate) to get all recently-touched plan files
4. Read each discovered plan file (skip files > 1000 lines — read the `00-overview.md` or `index.md` instead)

**How to use plan context:**

Plan context does NOT suppress or deprioritize findings. Instead, it **annotates** them:

- If a finding falls within scope of an active plan, append `-> covered by plans/{plan}/` to the finding
- If a plan has an active reroute or suspension notice (e.g., "all work suspended until X"), note this in the review preamble so the user knows which areas are in flux
- If a plan explicitly describes a refactor that would resolve a finding, mark it as `[PLANNED]` instead of proposing a separate fix — but still list it so nothing falls through cracks
- Findings NOT covered by any plan are reported normally — these are the high-value discoveries

**Example annotation:**
```
3. **[DRIFT]** `oriterm/src/gpu/renderer.rs:142` — Missing atlas invalidation for new glyph variant
   -> covered by plans/gpu_refactor/ (Section 3: Atlas Overhaul)
```

This ensures the review adds value by distinguishing "known debt being addressed" from "unknown debt needing attention."

### Step 3: Map the Boundary

Identify the module boundary being reviewed:
1. What types cross the boundary? (cells, grid state, render params, events)
2. What functions form the interface? (entry points, draw calls, event handlers)
3. What data flows across? (grid cells, palette colors, font metrics, input events)

For each module in the target, read the key files to understand the public API surface.

### Step 4: Trace Data Flow

Follow the data from producer to consumer:
1. **Read the producer's output types** — What does the upstream module emit?
2. **Read the consumer's input handling** — How does the downstream module receive and process it?
3. **Check the boundary types** — Are they minimal? Do they carry unnecessary baggage?
4. **Check ownership** — Is data moved, borrowed, or cloned? Are clones necessary?

### Step 5: Audit Each Rule Category

**Module Boundary Discipline:**
- [ ] Data flows one way? (no callbacks to earlier layer, no reaching back)
- [ ] No circular imports between modules?
- [ ] Boundary types are minimal? (only what's needed crosses)
- [ ] Clean ownership transfer? (borrow for rendering, move for ownership changes)
- [ ] No layer bleeding? (grid doesn't render, renderer doesn't parse VTE)

**Data Flow:**
- [ ] Zero-copy where possible? (cell references, not cell copies)
- [ ] No allocation in hot paths? (render loop, VTE input, key encoding)
- [ ] Newtypes for IDs? (`TabId`, not bare `u64`)
- [ ] Instance buffers reused across frames?
- [ ] Glyph cache avoids redundant rasterization?

**Error Handling at Boundaries:**
- [ ] No panics on user input? (bad escapes, invalid UTF-8, unexpected keys)
- [ ] PTY errors recoverable? (close tab, don't crash app)
- [ ] GPU errors surfaced? (surface lost, device lost -> recover or report)
- [ ] Config errors fall back to defaults?
- [ ] Errors carry context? (not bare `unwrap()` or swallowed `Result`)

**Rendering Discipline:**
- [ ] Frame building is pure computation? (no side effects on Grid/Tab/App)
- [ ] No state mutation during render?
- [ ] Color resolution happens once per frame, not per pass?
- [ ] Atlas misses are handled without blocking the frame?

**Event Flow:**
- [ ] Events flow through the event loop? (no bypassing `TermEvent`)
- [ ] Input dispatch is a decision tree? (one handler per event, no fallthrough)
- [ ] State transitions use enums, not booleans?
- [ ] Redraw requests coalesced?

**Platform & External Resource Abstraction:**
- [ ] `#[cfg()]` at module level, not scattered inline?
- [ ] Grid, VTE handler, selection, search are platform-independent?
- [ ] Logic-layer structs free of concrete external-resource types? (no `EventLoopProxy`, `Window`, `wgpu::Device`, etc. embedded in types that do routing/dispatch/state — use callbacks, traits, or channels instead)
- [ ] Concrete resource types wired only at the composition root? (`App::new`, entry point — not threaded through intermediate layers)

**Registration Sync Points:**
- [ ] Any enum/variant that must appear in multiple locations has a single source of truth?
- [ ] Parallel lists (match arms, arrays, maps) that must cover the same variants are derived from a shared source rather than manually mirrored?
- [ ] New variants added in one location are present in all parallel locations? (e.g., new `TermEvent` variant -> all match arms in event loop, new `Mode` flag -> VTE handler + grid + renderer)
- [ ] When centralization isn't feasible, is there a test enforcing completeness?
- [ ] Key -> action mappings, mode -> behavior mappings, event -> handler mappings — are these centralized or at risk of drift?

**Gap Detection:**
- [ ] Features supported in downstream modules also supported in upstream modules?
- [ ] No silent workarounds for missing capabilities? (e.g., hardcoded fallback because config doesn't expose a setting)
- [ ] Full pipeline works end-to-end for each feature? (input -> event -> handler -> grid -> render)

**File Organization:**
- [ ] All production source files under 500 lines? (test files exempt)
- [ ] Each file has a single clear responsibility? (not mixing input handling, rendering, state management)
- [ ] Logical groups of 200+ lines within a file extracted to submodules?
- [ ] File names describe what the file does? (not just `mod.rs` holding everything)
- [ ] Directory structure mirrors the logical module structure?
- [ ] Files touched by these commits that were already over 500 lines — were they split?

### Step 6: Compile Findings

Organize findings by boundary/interface, categorized as:

- **LEAK** — Data or control flow crossing a boundary it shouldn't (layer bleeding, backward reference, panic on user input)
- **DRIFT** — Registration data present in one location but missing from a parallel location that must stay in sync (e.g., enum variant added but match arm / handler / mapping not updated)
- **GAP** — Feature supported in one module but blocked or missing in another, breaking end-to-end functionality (e.g., VTE handler parses an escape but grid ignores it)
- **BLOAT** — File exceeds 500-line production limit, mixes multiple responsibilities, or lacks submodule structure. Bloated files obscure internal boundaries and make drift/leak detection harder. Include: current line count, identified responsibilities, and concrete extraction targets.
- **WASTE** — Unnecessary allocation, clone, or transformation at boundary (extra copy, redundant resolution, per-frame allocation)
- **EXPOSURE** — Internal state leaking through boundary types (app state in render params, grid internals in input handler)
- **NOTE** — Observation, not actionable (acceptable tradeoff, documented exception)

### Step 7: Generate Plan

Use **EnterPlanMode** to create a fix plan. The plan should:

1. List every LEAK, DRIFT, GAP, BLOAT, WASTE, and EXPOSURE finding with `file:line` references
2. Group by boundary (e.g., "app<>tab", "tab<>gpu", "grid<>term_handler")
3. Estimate scope: "N boundaries, ~M findings"
4. Order: leaks first (layer bleeding), then drift (sync), then gaps (feature coverage), then bloat (file organization), then waste (perf), then exposure (encapsulation)

### Plan Format

```
## Implementation Hygiene Review: {target}

**Scope:** N boundaries reviewed, ~M findings (X leak, Y drift, Z gap, W bloat, V waste, U exposure)

### Active Plan Context

{List each plan file read and its relevance. If a plan has a reroute/suspension, note it here.}
- `plans/gpu_refactor/` — Active: renderer architecture overhaul
- `plans/roadmap/section-12-mux.md` — Recently modified, covers mux boundary changes
- (none) — if no plan files were found

### {Boundary: Module A -> Module B}

**Interface types:** {list types crossing this boundary}
**Entry points:** {list key functions}

1. **[LEAK]** `file:line` — {description}
2. **[DRIFT]** `file:line` — {description}
   -> covered by plans/{plan}/ ({section name})
3. **[DRIFT] [PLANNED]** `file:line` — {description}
   -> fix described in plans/{plan}/{section}.md
4. **[GAP]** `file:line` — {description}
5. **[BLOAT]** `file:line` — {description}
6. **[WASTE]** `file:line` — {description}
7. **[EXPOSURE]** `file:line` — {description}
...

### {Next Boundary}
...

### Execution Order

1. Layer bleeding fixes (may require interface changes)
2. Registration drift fixes (add missing mappings, centralize parallel lists)
3. Gap fixes (unblock end-to-end feature paths)
4. File organization fixes (split bloated files into submodules — pure refactor, no logic changes)
5. Error handling fixes (may add error variants)
6. Ownership/allocation fixes (perf, no API change)
7. Encapsulation fixes (minimize boundary types)
8. Run `./test-all.sh` to verify no behavior changes
9. Run `./clippy-all.sh` to verify no regressions
```

## Important Rules

1. **No architecture changes** — Don't propose new modules, new crates, or restructured dependency graphs
2. **Full scope** — Module boundaries, data flow, naming, comments, visibility, file organization, lint discipline, and code fixes are all in scope. Only new modules, crates, or dependency graph restructures are out of scope (that's architecture).
3. **Trace, don't grep** — Follow actual data flow through the code, don't just search for patterns
4. **Read both sides** — Always read both the producer and consumer of a boundary
5. **Understand before flagging** — Some apparent violations are intentional (e.g., `app.rs` coordinating between tabs and windows is orchestration, not layer bleeding)
6. **Be specific** — Every finding must have `file:line`, the boundary it violates, and a concrete fix
7. **Compare to reference terminals** — When in doubt, check how Alacritty/WezTerm/Ghostty handle the same boundary at `~/projects/reference_repos/console_repos/`
8. **Minimum 20 findings** — Do your best to find at least 20 genuine issues. Dig deep, read broadly, trace more paths. Do NOT fabricate, exaggerate, or inflate findings to hit this number — every finding must be real and verifiable. If the target area genuinely has fewer than 20 issues, report what you find honestly and note the shortfall.
