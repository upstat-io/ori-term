---
paths:
  - "**/src/**"
---

# Implementation Hygiene Rules

These rules cover **implementation quality**: module boundaries, data flow, architectural discipline, algorithmic DRY. Process rules (git workflow, CI) live in CLAUDE.md. No duplication between them.

**Implementation hygiene is NOT architecture** (design decisions are made) **and NOT code hygiene** (surface style). It's the full plumbing layer — whether the implementation faithfully and cleanly realizes the architecture. Tight joints, correct flow, no leaks.

## Finding Categories

- **LEAK** — Logic, data, or control living outside its canonical home. The most dangerous category — side logic is how clean architectures decay. Subcategories:
  - **Layer bleeding**: a module doing work that belongs to another module
  - **Backward reference**: downstream module calling back into upstream module
  - **Swallowed error**: error silently dropped instead of propagated
  - **Duplicated dispatch**: routing/matching logic duplicated outside the canonical dispatch point
  - **Scattered knowledge**: type/mode/event behavior encoded ad hoc instead of read from the canonical source
  - **Validation bypass**: validation rules implemented at consumption sites instead of at the canonical validation point
  - **Inline policy**: business logic (defaults, thresholds, formatting rules) hardcoded at call sites instead of centralized
  - **Algorithmic duplication**: two or more sites performing the same multi-step operation (even on different types) where the control-flow skeleton is identical — the algorithm has no canonical home
  Default: **Critical**. Every LEAK creates a second source of truth that WILL drift. Fix immediately — never defer.
- **DRIFT** — Registration data present in one location but missing from a parallel sync point. Default: **Major**.
- **GAP** — Feature supported in one module but blocked/missing in another. Default: **Major**.
- **WASTE** — Unnecessary allocation, clone, or transformation at boundary. Default: **Minor**.
- **EXPOSURE** — Internal state leaking through boundary types. Default: **Minor**.
- **BLOAT** — File exceeds limits, mixes responsibilities, lacks submodule structure. Default: **Minor**.
- **NOTE** — Observation, acceptable tradeoff, documented exception. Default: **Informational**.

5+ findings clustered in one module = design problem; escalate to architectural review, not individual fixes.
**LEAK escalation**: 3+ LEAKs in one module = systemic side logic; the module lacks a canonical dispatch/query point. Don't patch individual LEAKs — introduce the missing canonical home first.

## Paradigms

Two paradigms govern all hygiene rules. Every rule in this document is a specific application of one or both.

### Single Source of Truth (SSOT)

Every piece of knowledge in the codebase has exactly **one canonical home**. All other locations that need that knowledge **query** or **derive from** the canonical source — they never maintain independent copies. This is the foundation of global coherence.

**ori_term's architectural centers:**

| Knowledge Domain | Canonical Home | Consumers Query Via |
|---|---|---|
| Terminal emulation (grid, cursor, VTE) | `oriterm_core` | `Grid`, `Term`, VTE handler |
| Widget behavior (interaction, layout, paint) | `oriterm_ui` | Widget trait, InteractionManager |
| Pane lifecycle & I/O | `oriterm_mux` | PaneRegistry, InProcessMux |
| Session model (tabs, windows, layouts) | `oriterm/src/session/` | SessionRegistry, SplitTree |
| GPU rendering (atlas, pipelines, shaders) | `oriterm/src/gpu/` | GpuRenderer, GlyphCache |
| Platform IPC transport | `oriterm_ipc` | Transport trait |
| Color palette & theming | `oriterm_core::palette` | Palette, ColorSpec |
| Key → action mapping | `oriterm_ui::action::keymap` | KeymapAction dispatch |

**Three failure modes:**

1. **No home** — knowledge scattered with no canonical source. Fix: create the canonical home, migrate consumers to query it.
2. **Multiple homes** — two+ locations both claiming authority, no clear winner. Fix: designate one as canonical, derive the rest via queries or generation.
3. **Shadow home** — canonical source exists but consumers bypass it with local copies. This is a **LEAK** — fix by removing the local copy and wiring the consumer to the canonical source.

**Enforcement mechanisms (pick the strongest that applies):**

1. **Type-level** (strongest): knowledge can only be constructed in one place. Consumers receive opaque handles (`WidgetId`, `PaneId`, `TabId`).
2. **Compile-time**: exhaustive match on source-of-truth enum forces consumers to handle all cases.
3. **Test-time**: exhaustiveness tests iterate the canonical list and verify all consumers are in sync.
4. **Query pattern**: consumers call a function on the canonical owner rather than maintaining their own lookup table.

**The test**: if you can answer "where is X defined?" with exactly one file path, SSOT holds. If you hesitate or name two places, it doesn't.

### No Side Logic

The complement of SSOT. Side logic is any logic living outside its canonical home — the mechanism by which SSOT degrades. Every LEAK finding is an SSOT violation in action.

## Module Boundary Discipline

- **One-way data flow**: Data flows downstream only. Rendering never calls back into VTE parsing. Input handling doesn't reach into GPU internals.
- **No circular imports**: Module dependencies must be acyclic. `gpu/` never imports `tab_bar.rs` internals. `grid/` never imports `app.rs`.
- **Minimal boundary types**: Only pass what the next layer needs. Render params: `(&Grid, &Palette, &FontSet)`, not the entire `Tab`.
- **Clean ownership transfer**: Move at boundaries, borrow within modules. No unnecessary `.clone()` at layer transitions.
- **No layer bleeding**: Grid doesn't render, renderer doesn't parse VTE, input handler doesn't mutate grid directly.
- **Crate-level boundaries**: Pure UI logic (testable without GPU/platform/terminal) belongs in `oriterm_ui`, not `oriterm`. See `.claude/rules/crate-boundaries.md` for full ownership rules and allowed dependency directions.
- **Module purity**: output depends only on input; no global mutable state, no side channels between modules.

### Module-Specific Responsibilities

- **oriterm_core**: Terminal emulation only — grid, VTE handling, selection, search. No rendering, no I/O, no platform deps.
- **oriterm_ui**: Widget framework — interaction, layout, paint, animation, testing. No GPU, no platform, no terminal I/O.
- **oriterm_mux**: Pane server — PTY I/O, pane lifecycle, snapshots. No rendering, no session model.
- **oriterm/src/gpu/**: GPU rendering only — atlas, pipelines, draw_frame. No terminal logic, no session model.
- **oriterm/src/session/**: Session orchestration — tabs, windows, splits, navigation. No rendering, no terminal emulation.
- **oriterm/src/app/**: Application shell — event loop, window management, GPU init. Wiring layer that composes everything.

## Side Logic — Root of Architectural Decay

Side logic is any logic that lives outside its canonical home. It is the primary mechanism by which clean architectures degrade into historical drift. Each instance creates a second source of truth that can diverge from the canonical one.

**The cascade**: one side-logic shortcut invites another. Within months, the canonical source becomes "one of several places" that defines behavior, and eventually no single location is authoritative. This is irreversible without major refactoring.

### Detection Heuristics

1. **The "where would I look?" test**: If someone asks "where is X's behavior defined?" and the answer isn't a single location, there's a LEAK.
2. **The "what if it changes?" test**: If changing behavior X requires edits in N locations and N > 1 (excluding tests and docs), there's a LEAK.
3. **The "copy-paste smell" test**: If a match arm, if-chain, or lookup table mirrors structure from another file, one of them is side logic. If the duplication is *algorithmic* (same control-flow skeleton, different types/operations), see also Algorithmic DRY.
4. **The "special case" test**: If a function has `if mode == SomeSpecificMode { ... }` outside the canonical dispatch point for that mode, it's side logic.

### Common Side Logic Patterns (All Are LEAK)

- **Ad hoc mode knowledge**: Checking `is_alternate_screen()` or `is_bracketed_paste()` to apply special behavior outside the canonical mode handler. The mode system defines behavior — consumers query it.
- **Duplicated dispatch tables**: A match on `TermEvent` or `KeyAction` that parallels an existing canonical match elsewhere. Add a case to the canonical dispatcher instead.
- **Inline defaults**: Hardcoding a default value, threshold, or policy at a call site instead of defining it in the type/config that owns it.
- **Re-derived facts**: Computing something that another module already computed and stored. Query the stored result.
- **Format logic outside formatters**: Building display strings for terminal state outside `Display`/`Debug`/diagnostic formatters.
- **Validation at consumption**: Checking invariants at every use site instead of enforcing them at construction (parse-don't-validate pattern).

### Remediation

The fix for side logic is always the same: **move the logic to its canonical home and have the consumption site query/call it**. Never "fix" a LEAK by adding a comment explaining why the duplication exists. If the canonical home doesn't exist yet, create it — that's the real fix.

## Data Flow

- **Zero-copy where possible**: Grid cells referenced by position, not by owned copies. Borrow `&Row`, don't clone rows for rendering.
- **No allocation in hot paths**: Render loop, VTE handler input path, and key encoding are hot. No `String::from()`, no `Vec::new()`, no `Box::new()` per cell/frame.
- **Newtypes for IDs**: `TabId(u64)`, `PaneId(u64)`, `WidgetId` — not bare integers. Prevents cross-boundary ID confusion.
- **Instance buffers reused**: GPU instance buffers should grow but never shrink per frame. Reuse allocations across frames.
- **Hash lookups over linear scans**: for collections > ~8 items; small fixed-size lists may use linear scan.
- **Snapshot transfer is zero-alloc**: IO thread → main thread snapshot exchange uses `std::mem::swap()` on pre-allocated buffers, never allocation.

### Hot Paths

Hot: render loop (draw_frame), VTE parser input, key encoding, grid cell iteration, glyph shaping, instance buffer filling. Cold: config parsing, font loading, error formatting, startup, test setup. When unsure, profile.

## Error Handling at Boundaries

- **No panics on user input**: Malformed escape sequences, invalid UTF-8 from PTY, unexpected key events — all must be handled gracefully, never `panic!` or `unwrap()`.
- **PTY errors are recoverable**: Reader thread errors close the pane, don't crash the app.
- **GPU errors surfaced**: Surface lost, device lost — recover or report, don't silently fail.
- **Config errors fall back to defaults**: Invalid TOML, missing fields, bad values — log a warning, use defaults.
- **Errors carry context**: not bare `unwrap()` or swallowed `Result`. Use `.context()` or `.map_err()` at boundaries.
- **Never swallow errors silently**: `match err { Err(_) => Ok(default) }` and `if let Ok(x) = fallible` (silently drops error) are anti-patterns.

## Rendering Discipline

- **Frame building is pure computation**: `draw_frame()` reads state, builds instance buffers. No side effects on Grid, Tab, or App state.
- **No state mutation during render**: Rendering borrows immutably. If render needs to change state (e.g., cursor blink toggle), send a message/event instead.
- **Opacity and color are resolved once**: Per-cell color resolution (bold-bright, dim, inverse) happens once per frame, not per-pipeline-pass.
- **Atlas misses are deferred**: If a glyph isn't cached, rasterize and cache it, but don't block the frame. Pre-cache ASCII at load time.

## Event Flow Discipline

- **Events flow through the event loop**: PTY output -> `TermEvent` -> `user_event` handler. No direct function calls bypassing the event loop.
- **Input dispatch is a decision tree, not a cascade**: Each input event is handled by exactly one handler. No fallthrough to multiple handlers.
- **State transitions are explicit**: Drag state machine (`Pending -> DraggingInBar -> TornOff`) uses enum variants, not boolean flags.
- **Redraw requests are coalesced**: Multiple state changes in one event batch should produce one redraw, not N redraws.

## Platform & External Resource Abstraction

- **`#[cfg()]` at module level, not inline**: Platform differences go in dedicated files (`clipboard.rs` with `#[cfg(windows)]`/`#[cfg(not(windows))]`), not scattered `#[cfg()]` blocks inside functions.
- **Shared interface, platform implementation**: Common trait or function signature, platform-specific body.
- **No `cfg` in business logic**: Grid, VTE handler, selection, search — these must be platform-independent.
- **No concrete external-resource types in logic layers**: Structs that perform logic (event routing, state management, command dispatch) must not embed concrete types that require runtime resources — display servers (`EventLoopProxy`, `Window`), GPU contexts (`wgpu::Device`), file handles, network sockets, etc. Accept callbacks (`Arc<dyn Fn() + Send + Sync>`), traits, or channels instead. The litmus test: if a type can't be constructed in a headless `#[test]` without `#[ignore]`, `OnceLock<EventLoop>`, or platform `#[cfg]` gymnastics, the boundary is wrong. The concrete resource type belongs at the wiring layer (e.g., `App::new`), not in the logic layer it's injected into.

## Registration Sync Points

Application of the SSOT paradigm to enum variants, lookup tables, and parallel data structures.

- **Canonical source drives all consumers**: one location is the source of truth — others derive from it or are validated against it. Never maintain independent parallel lists.
- **No manual mirroring**: centralize via `from_str()`, `all()`, iterator — not parallel lists. If you must have parallel structure, generate or validate it from the canonical source.
- **Compile-time or test-time enforcement**: add test iterating source-of-truth list. Prefer compile-time (exhaustive match) over test-time where possible.
- **Flag drift as finding**: new variant in one location but missing from parallel = **DRIFT**
- **Flag duplication as finding**: parallel lookup table that could query the canonical source instead = **LEAK:scattered-knowledge**
- **Common sync risks**: new `TermEvent` variant -> all match arms in event loop; new `Mode` flag -> VTE handler + grid + renderer; new widget type -> interaction manager + layout + paint; new key action -> keymap + dispatch + help text.

## Algorithmic DRY — No Duplicated Algorithms

SSOT ensures every piece of **knowledge** has one canonical home. This section ensures every **algorithm** — a multi-step operation with a recognizable control-flow skeleton — also has one home. Duplicated algorithms are `LEAK:algorithmic-duplication` findings.

An algorithm is duplicated when two or more sites share the same control-flow skeleton (loop structure, branch conditions, error handling shape) and differ only in:
- **Types** — same traversal over different type parameters
- **Operations** — same loop harness with different per-element callbacks
- **Field names** — same structural access pattern on different structs
- **Module context** — same validation/dispatch logic in different modules

Knowledge duplication drifts when facts change. Algorithmic duplication drifts when the *protocol* changes — a new step is added to one copy but not the others. Both are equally dangerous.

### Detection Heuristics

1. **The "diff the bodies" test**: If two function bodies differ only in type names, field names, or closure bodies but share the same control-flow skeleton (loops, branches, error paths), the skeleton is an extractable algorithm.
2. **The "count the steps" test**: If 3+ call sites perform the same sequence of 2+ operations (even with different arguments), extract a higher-order function.
3. **The "inline lambda" test**: If you could copy-paste a block and only change the closures/callbacks, the surrounding scaffold is the algorithm to extract.
4. **The "match arm count" test**: If the same enum/tag is matched in N files with similar arm structure, N-1 of those matches are candidates for consolidation into a canonical dispatcher.

### Thresholds

- **2 instances, >5 lines of shared skeleton**: extract immediately. Two non-trivial copies is already one too many.
- **3+ instances, any size**: always extract. No exceptions. This is the "missing abstraction" threshold.
- **Cross-crate duplication**: even 2 instances = extract to a shared crate or shared type. Cross-crate copy-paste is the most dangerous because drift is invisible — different test suites, different maintainers, different change cadences.

### Remediation Hierarchy

When algorithmic duplication is found, select the **first** approach that fits:

1. **Generic function** (`<T>` / trait bounds) — steps identical, only types differ.
2. **Higher-order function** (closure parameters) — skeleton identical, per-element operations differ.
3. **Trait + blanket impl** — pattern crosses type families with shared interface.
4. **Data-driven dispatch** (registry table) — routing structure identical, entries differ.
5. **Macro** — last resort, when duplication is syntactic (identical token structure) rather than semantic. Prefer any of the above when the shared structure is semantic.

### What This Is NOT

- **Not "never repeat a line"** — three similar `map.insert()` calls aren't duplication. The bar is *structural*: same multi-step algorithm with the same control-flow shape.
- **Not speculative generalization** — extract only when you have 2+ concrete instances. Never for a hypothetical future need.
- **Not "every helper is good"** — a helper called once that just relocates code is noise, not DRY. The test: does the extraction eliminate a *second copy*, or does it just move a single copy?
- **Not a license to over-abstract** — the extraction should be simpler than the duplication it replaces. If the abstraction is harder to understand than the copies, the copies are better.

## Gap Detection

- **Cross-module capability mismatch = GAP**: one module supports a feature, another blocks it
- **Never silently work around a gap**: flag immediately
- **Audit across modules**: when adding capability, verify full pipeline: input -> event -> handler -> grid -> render

## Cascading Fix Detection

- **Whack-a-mole = architectural issue**: fix at one callsite moves failure to next -> STOP
- **Three-strike rule**: same fix at 3+ callsites = missing abstraction; fix at boundary
- **More heuristics**: >3-4 params -> config struct. Same enum matched in 3+ files -> centralize dispatch. Same error string in 3+ places -> error factory function.
- **Present options**: (1) architectural issue, (2) why per-site patches won't scale, (3) 2-3 options

## Invariant Explicitness

- **Implicit invariants are invisible regressions.** If correctness depends on a property (buffer capacity maintained, snapshot double-buffer in valid state, event loop control flow correct, atlas cache consistent), it MUST be either:
  - A `debug_assert!` at the point where the invariant is relied upon, OR
  - A test that would fail if the invariant is violated
- **Semantic changes require semantic pins.** When a fix changes observable behavior (render output, event routing, resize behavior), add a regression test that ONLY passes with the new semantics.

## File Organization

- **500-line limit**: source files (excluding tests); exceeding = **BLOAT** finding
- **Proactive split**: split at ~450 lines if you know more code is coming. Don't wait until over the limit.
- **Single responsibility per file**: one logical operation or one type family. Anti-pattern: `utils.rs`, `helpers.rs`, `misc.rs`. Every file name describes its domain.
- **Submodule extraction**: logical group exceeding ~200 lines -> sibling submodule; parent `mod.rs` = dispatch hub
- **Split when touching**: touching a file over 500 lines without splitting = finding
- **Tests in sibling `tests.rs`**: `#[cfg(test)] mod tests;` declaration only — body in sibling file
- **Section markers**: plain `// Section name` on its own line, preceded by blank line. No decorative characters.
- **Banner removal**: if you touch a file with decorative banners (`// ===`, `// ---`), remove them.

### Module Roles

- `lib.rs` is an **index**: `//!` doc, `mod` declarations, `pub use` re-exports — no function bodies. Strict, no exceptions.
- `mod.rs` **dispatches**: routes to submodules, holds shared private items
- Leaf files **implement**: actual logic lives here
