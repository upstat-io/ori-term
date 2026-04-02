---
name: impl-hygiene-review
description: Deep, wide implementation hygiene review — multi-pass analysis across modules and crate boundaries with third-party cross-checking.
allowed-tools: Read, Grep, Glob, Agent, Bash, Skill
---

# Implementation Hygiene Review

Deep, wide-angle review of implementation hygiene against `.claude/rules/impl-hygiene.md`. Multi-pass, multi-lens analysis that traces data flow end-to-end, detects algorithmic duplication, and cross-checks findings via third-party review.

**Implementation hygiene is NOT architecture** (design decisions are made). It covers the full plumbing layer — module boundaries, data flow, error propagation, abstraction discipline, algorithmic DRY, file organization, naming, comments, visibility, and lint discipline.

## Target

`$ARGUMENTS` specifies the boundary or scope to review. **If empty or blank, default to last commit mode** (equivalent to `/impl-hygiene-review last commit`). Otherwise, there are three modes:

### Path Mode (explicit crate/directory targets)
- `/impl-hygiene-review oriterm_core/src/grid oriterm_core/src/term_handler.rs` — review grid<>VTE handler boundary
- `/impl-hygiene-review oriterm/src/gpu/` — review GPU rendering internals
- `/impl-hygiene-review oriterm_ui/src/widgets/` — review widget implementations
- `/impl-hygiene-review oriterm_mux/src/pane/` — review pane I/O internals

### Commit Mode (use a commit as a scope selector)
- `/impl-hygiene-review last commit` — review files touched by the most recent commit
- `/impl-hygiene-review last 3 commits` — review files touched by the last N commits
- `/impl-hygiene-review <commit-hash>` — review files touched by a specific commit

### Full Project Mode (landscape survey)
- `/impl-hygiene-review full` — review the entire project across all crates and boundaries
- `/impl-hygiene-review full --focus=dry` — full review with emphasis on algorithmic duplication
- `/impl-hygiene-review full --focus=leaks` — full review with emphasis on side logic and SSOT

Full project mode is the widest sweep. It reviews every crate, every module boundary, all cross-crate interactions, and end-to-end pipeline flow. Use this when you want the complete landscape picture.

**CRITICAL: Commits are scope selectors, NOT content filters.** The commit determines WHICH files and areas to review. Once the files are identified, review them completely — report ALL hygiene findings in those files, regardless of whether the finding is "related to" or "caused by" the commit. The commit is a lens to focus on a region of the codebase, nothing more. Do NOT annotate findings with whether they relate to the commit. Do NOT deprioritize or exclude findings because they predate the commit.

**Commit scoping procedure:**
1. Use `git diff --name-only HEAD~N..HEAD` (or appropriate range) to get the list of changed `.rs` files
2. Expand to include the full crate(s) those files belong to (e.g., if `oriterm/src/gpu/atlas.rs` was touched, include all of `oriterm/src/gpu/`)
3. **Dependency expansion**: Also include crates that *consume* the changed crate's public types or functions. If `oriterm_core` was changed, also expand to `oriterm_ui`, `oriterm_mux`, `oriterm` (its consumers). This catches boundary violations that the changed crate creates for its downstream consumers.
4. Proceed with the standard review process using those crates as the target

**Dependency map for expansion:**
```
oriterm_ipc  → consumed by: oriterm_mux
oriterm_core → consumed by: oriterm_ui, oriterm_mux, oriterm
oriterm_ui   → consumed by: oriterm
oriterm_mux  → consumed by: oriterm
oriterm      → (application shell, no consumers)
```

## Execution

### Phase 1: Load Rules & Context

#### 1a. Load Rules

The full rule set is embedded below (source of truth files — do not maintain separate copies):

**Hygiene Rules** (`.claude/rules/impl-hygiene.md`):
@.claude/rules/impl-hygiene.md

**Code Hygiene Rules** (`.claude/rules/code-hygiene.md`):
@.claude/rules/code-hygiene.md

#### 1b. Load Plan Context

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

### Phase 2: Map the Full Landscape

Before diving into findings, build a high-level map of the review scope. This is the "go wide" phase — understand the shape before probing the details.

#### 2a. Identify Review Targets

Determine the distinct crates or module boundaries to review based on the target scope:

1. List the crates (directories) in scope
2. Identify which module boundaries exist between them (e.g., core<>ui, core<>mux, mux<>app, gpu<>app)
3. Map the dependency graph between in-scope crates
4. Group crates into **review units** — each review unit is either:
   - A single crate (for internal review)
   - A pair of crates sharing a boundary (for boundary review)
   - Closely related modules that should be reviewed together

#### 2b. Map Cross-Crate Data Flow (Full Project & Multi-Crate Mode)

For full project mode or when 3+ crates are in scope, spawn an agent to trace the key data flows end-to-end through the pipeline:

1. **Terminal data flow**: How do bytes from PTY -> VTE parser -> Grid -> Snapshot -> GPU renderer?
2. **Input flow**: How does a keystroke route from winit event -> keymap dispatch -> action handler -> PTY write?
3. **Resize flow**: How does a window resize propagate from winit -> session layout -> grid reflow -> GPU viewport?
4. **Session model flow**: How do pane lifecycle events (create, close, split) flow from mux -> session -> UI?

This agent produces a **flow map** — a brief summary of how each major data category crosses the module boundaries. This map is passed to all subsequent review agents as context.

### Phase 3: Deep Analysis (Multi-Pass, Multi-Lens)

This is the "go deep" phase. Each review unit gets **multiple analysis passes**, each with a different lens. This catches issues that a single-pass review misses because different violation types require different reading strategies.

For **each review unit** identified in Phase 2, spawn agents for the following passes. Passes within the same review unit run **sequentially** (each builds on the prior). Passes for **different review units** run in parallel.

#### Pass 1: LEAK & SSOT Scan (Structural Pass)

**Goal**: Find all side logic, scattered knowledge, duplicated dispatch, and SSOT violations.

This pass reads the code structurally — it's looking for *where* logic lives relative to where it *should* live.

**Checklist:**
- [ ] **No duplicated dispatch**: match/if-chain on TermEvent, KeyAction, Mode, or widget type exists ONLY at the canonical dispatch point? Any parallel match elsewhere is a LEAK — even if it produces correct results today.
- [ ] **No scattered knowledge**: mode behavior, key bindings, color resolution read from the canonical source, never hardcoded? Any `if mode == X { special_behavior }` outside the canonical dispatcher is a LEAK.
- [ ] **No re-derived facts**: information computed by another module is queried, not recomputed? Recomputing what's already stored creates a shadow source of truth.
- [ ] **No inline policy**: defaults, thresholds, format strings, validation rules defined at their canonical home, not at consumption sites? If changing a default requires grep-and-replace across files, it's a LEAK.
- [ ] **No validation at consumption**: invariants enforced at construction/entry, not checked at every use site? (parse-don't-validate)
- [ ] **No format logic outside formatters**: Display/Debug/diagnostic strings built in their formatting impls, not inline at error sites?
- [ ] **"Where would I look?" passes**: for every behavioral decision in this code, can you point to exactly ONE canonical location that defines it?
- [ ] **"What if it changes?" passes**: if the behavior changed, would exactly ONE file need updating (plus tests/docs)? If N > 1, it's a LEAK.
- [ ] **Canonical home exists**: for every behavioral decision, event routing rule, or mode behavior in this code — is there exactly ONE file that defines it? If the knowledge has no home (scattered everywhere), that's a structural SSOT violation.
- [ ] **No parallel authority**: are there two locations both claiming to define the same knowledge? Designate one as canonical, derive the other.
- [ ] **Consumers query, don't cache**: do consumers of shared knowledge call a function/query on the canonical owner, or do they maintain a local lookup table? Local tables are shadow homes.
- [ ] **Enforcement exists**: for every canonical source, is there a compile-time (exhaustive match) or test-time (exhaustiveness test) mechanism that catches consumers falling out of sync?
- [ ] **Architectural centers respected**: does this code correctly query from: InteractionManager (interaction state), KeymapAction (key bindings), Grid (terminal state), Palette (colors)? Or does it re-derive what these centers already know?

#### Pass 2: Algorithmic DRY Scan (Pattern Pass)

**Goal**: Find duplicated algorithms — functions with identical control-flow skeletons that differ only in types, operations, or field names.

This pass reads the code *comparatively* — it's looking for structural similarity between function bodies, match arms, and dispatch tables. This is the hardest pass because it requires comparing code across files.

**Checklist:**
- [ ] **"Diff the bodies" test**: Read pairs of functions that handle similar cases (e.g., two widget paint methods, two event dispatch functions). Do their bodies differ only in type names, field names, or closure bodies while sharing the same control-flow skeleton?
- [ ] **"Count the steps" test**: Are there 3+ call sites that perform the same sequence of 2+ operations (even with different arguments)? Example: validate input -> extract state -> perform operation -> request redraw.
- [ ] **"Cross-crate mirror" test**: Do different crates maintain parallel dispatch tables, match arms, or routing logic with the same structure? Trace a concept (e.g., widget type, event kind) through multiple crates — does each maintain its own routing independently?
- [ ] **"Match arm count" test**: Is the same enum/tag matched in N files with similar arm structure? If N > 2, N-1 of those are candidates for consolidation.
- [ ] **Threshold check**: 2 instances with >5 shared skeleton lines = extract. 3+ instances any size = extract. Cross-crate = always extract to shared crate or shared metadata source.
- [ ] **Remediation check**: For each algorithmic duplication found, identify the correct extraction: generic fn, higher-order fn, trait + blanket impl, data-driven dispatch, or (last resort) macro?

**How to execute this pass:**
1. For each crate, identify the major dispatch/routing functions (event dispatch, widget paint, input handling, mode switching)
2. Group structurally similar functions — same parameters shape, same loop/match/if structure
3. Read them side by side. Count the lines that are structurally identical vs. lines that differ.
4. If >60% structural overlap across 5+ lines: flag as `LEAK:algorithmic-duplication`
5. For cross-crate patterns, read both sides of the boundary — trace both paths

#### Pass 3: Boundary & Flow Scan (Plumbing Pass)

**Goal**: Find boundary violations, data flow issues, error handling gaps, and type discipline problems.

This pass reads the code *across boundaries* — it's looking at how data crosses module lines.

**Checklist:**

**Module Boundary Discipline:**
- [ ] Data flows one way? (no callbacks to earlier layer, no reaching back)
- [ ] No circular imports between modules?
- [ ] Boundary types are minimal? (only what's needed crosses)
- [ ] Clean ownership transfer? (borrow for rendering, move for ownership changes)
- [ ] No layer bleeding? (grid doesn't render, renderer doesn't parse VTE)

**Data Flow:**
- [ ] Zero-copy where possible? (cell references, not cell copies)
- [ ] No allocation in hot paths? (render loop, VTE input, key encoding)
- [ ] Newtypes for IDs? (`TabId`, `PaneId`, not bare integers)
- [ ] Instance buffers reused across frames?
- [ ] Glyph cache avoids redundant rasterization?
- [ ] Snapshot transfer is zero-alloc? (swap, not copy)

**Error Handling at Boundaries:**
- [ ] No panics on user input? (bad escapes, invalid UTF-8, unexpected keys)
- [ ] PTY errors recoverable? (close pane, don't crash app)
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

#### Pass 4: Surface Hygiene Scan (Polish Pass)

**Goal**: Find file organization violations, naming issues, comment problems, visibility leaks, and style violations.

This pass reads the code *locally* — each file on its own terms.

**Checklist:**

**File Organization:**
- [ ] All production source files under 500 lines? (test files exempt)
- [ ] Each file has a single clear responsibility? (not mixing input handling, rendering, state management)
- [ ] Logical groups of 200+ lines within a file extracted to submodules?
- [ ] File names describe what the file does? (not just `mod.rs` holding everything)
- [ ] Directory structure mirrors the logical module structure?
- [ ] Files touched by these commits that were already over 500 lines — were they split?

**Naming, Comments, Visibility, Style:**
- [ ] Verb-prefix conventions used? (render_, draw_, handle_, encode_)
- [ ] No decorative banners, no commented-out code, no bare TODOs?
- [ ] Functions < 100 lines? Nesting depth <= 4?
- [ ] pub(crate)/pub(super) used appropriately? No dead pub items?

### Phase 4: Third-Party Cross-Check

**MANDATORY for full project mode. Recommended for all other modes.**

After Phase 3 agents return their findings, use `/tp-help` to cross-check the work. This creates a two-brain review: you found the patterns, now Codex validates them and looks for what you missed.

#### 4a. Validate Findings

Invoke `/tp-help` with a focused question. Pass a summary of 5-10 of the most significant findings (not all — pick the ones that are most ambiguous or architecturally significant) and ask Codex to validate:

```
/tp-help I'm running a hygiene review of [scope]. Here are my top findings — validate whether these are real violations or false positives, and tell me if I'm missing anything obvious in these areas:

[List of 5-10 findings with file:line and brief description]

Key files involved: [list the main files]
```

**What to do with Codex's response:**
- If Codex confirms a finding: increase confidence, keep it
- If Codex challenges a finding: re-read the code, check if you misunderstood the pattern. Update or drop the finding if Codex is right.
- If Codex surfaces NEW findings you missed: add them to the findings list. These are high-value discoveries — a fresh pair of eyes saw what you didn't.

#### 4b. Probe Blind Spots

After validating findings, use `/tp-help` again to probe areas you might have under-examined:

```
/tp-help I reviewed [scope] and found [N] findings, but I'm worried I may have missed algorithmic duplication in [specific area]. Can you compare [file A] and [file B] structurally and tell me if their control-flow skeletons are duplicated?
```

**When to probe:**
- Any crate that yielded zero findings (suspiciously clean — likely under-examined)
- Cross-crate code — hardest to catch because it requires reading two codebases in parallel
- Large match/dispatch functions — easy to skim past structural duplication when arms look "different enough"
- Code paths you traced superficially (read the entry point but not the helpers)

#### 4c. Integrate Cross-Check Results

Merge Codex's validated and new findings back into the main findings list. Tag findings that Codex confirmed with `[TP-CONFIRMED]` and findings Codex surfaced with `[TP-SURFACED]` — this helps the plan prioritize high-confidence issues.

### Phase 5: Compile & Present Findings

Collect the findings from all passes across all review units. This is synthesis, not just concatenation.

#### 5a. Deduplicate

Same violation caught by multiple passes -> keep the deepest analysis, drop the others.

#### 5b. Cross-Reference

Look for patterns across findings:
- **Cluster analysis**: 5+ findings in one module = design problem (escalate to architectural review)
- **3+ LEAKs in one module** = systemic side logic; the module lacks a canonical dispatch/query point
- **Same algorithm duplicated across N files** = missing abstraction (report as a single finding, not N findings)
- **Cross-crate findings** = highest priority; these drift silently

#### 5c. Severity Calibration

Apply default severities from the finding categories, then adjust:
- **LEAK:algorithmic-duplication** across 3+ sites -> Critical (blast radius of protocol change is proportional to copy count)
- **Cross-crate LEAKs** -> always Critical (boundary drift is a correctness risk, not just maintainability)
- Findings tagged `[TP-CONFIRMED]` -> keep severity. Findings tagged `[TP-SURFACED]` -> bump severity one level (Codex caught what you missed, which means it's less obvious and more likely to be missed again)

#### 5d. Present to User

Present findings organized by category and severity, with a summary preamble:

```
## Hygiene Review: [scope]

**Scope**: [crates/boundaries reviewed]
**Passes**: LEAK/SSOT, Algorithmic DRY, Boundary/Flow, Surface Hygiene
**Third-party cross-check**: [Yes/No] — [N confirmed, M surfaced]
**Finding counts**: [N LEAK, N DRIFT, N GAP, N WASTE, N EXPOSURE, N BLOAT]

### Active Plan Context
[Plans read and their relevance]

### Critical Findings (LEAKs)
...

### Major Findings (DRIFT, GAP)
...

### Minor Findings (WASTE, EXPOSURE, BLOAT)
...
```

### Phase 6: Generate Plan (Separate Agent)

Spawn a **separate Agent** to generate the fix plan. This agent should use `/create-plan` (via the **Skill** tool). Pass it:

1. **All compiled findings** from Phase 5
2. **The plan name**: `hygiene-{target-short-name}` (e.g., `hygiene-gpu`, `hygiene-grid-vte`, `hygiene-last-commit`, `hygiene-full`)

The agent should create a plan that:

1. Lists every LEAK, DRIFT, GAP, WASTE, EXPOSURE, and BLOAT finding with `file:line` references
2. Groups by boundary (e.g., "core<>ui", "mux<>app", "gpu internals") or by violation type for full-project mode
3. Estimates scope: "N boundaries, ~M findings"
4. Orders: **LEAKs first and separately** (side logic is the root of all evil — every LEAK is a ticking architectural bomb), then drift (sync), then gaps (feature coverage), then bloat (file organization), then waste (perf), then exposure (encapsulation). LEAKs must NEVER be deferred — they cascade.
5. **Algorithmic duplication findings get their own section** — these often require coordinated multi-file refactoring (extracting a shared helper, adding a generic function, creating a data-driven dispatch table). Group by the algorithm being duplicated, not by where the copies live.

The **final section** of the plan must be a cleanup step:

```markdown
## Cleanup

- [ ] Run `./test-all.sh` to verify no behavior changes
- [ ] Run `./clippy-all.sh` to verify no regressions
- [ ] Run `./build-all.sh` to verify cross-compilation
- [ ] Delete this plan directory: `rm -rf plans/hygiene-{name}/`
```

Hygiene fix plans are disposable — they exist to track the fixes, then get deleted when complete.

### Plan Section Format

Each section groups findings by boundary or violation cluster:

```
## {Boundary: Module A <> Module B}

**Interface types:** {list types crossing this boundary}
**Entry points:** {list key functions}

### Active Plan Context

{List each plan file read and its relevance. If a plan has a reroute/suspension, note it here.}
- `plans/gpu_refactor/` — Active reroute: renderer architecture overhaul
- (none) — if no plan files were found

### Findings

1. **[LEAK:duplicated-dispatch]** `file:line` — {description} — **canonical home**: `{canonical_file:line}`
2. **[LEAK:algorithmic-duplication]** `file_a:line` <> `file_b:line` — {description of shared skeleton} — **extraction**: {generic fn / HOF / trait / data-driven / macro}
3. **[LEAK:scattered-knowledge]** `file:line` — {description} — **canonical home**: `{canonical_file:line}`
4. **[DRIFT]** `file:line` — {description}
   -> covered by plans/{plan}/ ({section name})
5. **[DRIFT] [PLANNED]** `file:line` — {description}
   -> fix described in plans/{plan}/{section}.md
6. **[GAP]** `file:line` — {description}
7. **[WASTE]** `file:line` — {description}
8. **[EXPOSURE]** `file:line` — {description}
```

## Important Rules

1. **No architecture changes** — Don't propose new modules, new crates, or restructured dependency graphs
2. **Full scope** — Module boundaries, data flow, naming, comments, visibility, file organization, lint discipline, algorithmic DRY, and code fixes are all in scope. Only new modules, crates, or dependency graph restructures are out of scope (that's architecture).
3. **Trace, don't grep** — Follow actual data flow through the code, don't just search for patterns
4. **Read both sides** — Always read both the producer and consumer of a boundary
5. **Compare function bodies** — For algorithmic DRY, you must read pairs/groups of structurally similar functions side by side. Grepping for names is not enough — you need to compare control-flow skeletons.
6. **Understand before flagging** — Some apparent violations are intentional (e.g., `app.rs` coordinating between tabs and windows is orchestration, not layer bleeding)
7. **Be specific** — Every finding must have `file:line`, the boundary it violates, and a concrete fix
8. **Compare to reference terminals** — When in doubt, check how Alacritty/WezTerm/Ghostty handle the same boundary at `~/projects/reference_repos/console_repos/`
9. **Cross-check with /tp-help** — For full project mode: MANDATORY. For other modes: RECOMMENDED. Always validate ambiguous findings and probe blind spots. A hygiene review that doesn't question its own completeness is incomplete.
10. **Follow the algorithm, not the name** — Two functions named differently but with identical control-flow skeletons are duplicated. Two functions named similarly but with genuinely different logic are not. Read bodies, not signatures.

## Finding Targets

Finding targets scale with scope. These are **minimums** — dig deep, read broadly, trace more paths. Do NOT fabricate, exaggerate, or inflate findings to hit the target — every finding must be real and verifiable. If the target area genuinely has fewer issues, report what you find honestly and note the shortfall.

| Mode | Minimum Findings | Expected Range |
|------|-----------------|----------------|
| Single boundary or single crate | 20 | 20-35 |
| Multi-crate or last N commits spanning multiple crates | 40 | 40-60 |
| Full project | 80 | 80-120 |
| Full project with --focus | 60 (focused category) + 30 (other categories) | 60-100 focused + 30-50 other |

**Algorithmic DRY findings count as high-value** — a single algorithmic duplication finding that spans 5 files is worth more than 5 individual surface hygiene findings. Quality over quantity, but quantity matters too because thoroughness is the point.
