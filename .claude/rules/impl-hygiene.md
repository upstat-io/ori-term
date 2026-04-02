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

### Interaction with SSOT

Algorithmic DRY is the complement of SSOT:
- **SSOT** asks: "where is this *fact* defined?" — answer must be one place
- **Algorithmic DRY** asks: "where is this *operation* defined?" — answer must be one place

When both apply (e.g., a dispatch table that encodes both facts and routing), fix the SSOT violation first (centralize the data), then the algorithmic violation (consolidate the routing logic that queries it). The data-driven dispatch pattern often fixes both at once.

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

### Crate Organization

- Each crate has a single documented purpose
- Module nesting max 4 levels (e.g., `oriterm_ui::widgets::button::style`). Deeper = missing abstraction.
- If a crate has >50 source files, consider splitting
- Shared utilities live in dedicated crates (`oriterm_core`, `oriterm_ipc`). No `utils` modules in application crates. If 3+ crates need the same utility, extract to a shared crate.

## Type Discipline

- **Newtypes for all IDs**: `TabId`, `PaneId`, `WidgetId` — not raw `u64`. Inner field private, construct via `new()`/`From`, `.0` access only inside defining module.
- **Metadata in sidecars**: metadata (debug info, diagnostic context) travels in sidecars or indexed maps, not inline in core data structures. Core types stay lean.
- **Pre-compute derivable metadata**: at construction time (e.g., flags, cached measurements). O(1) queries, never re-walk composite structures.
- **Option vs Result**: `Option` for absent/not found (lookup miss). `Result` for failure with diagnostic info. Never `Result<T, ()>` — use `Option`. Never `Option` when None should carry an error.
- **Type aliases**: for long generic types (e.g., `Result<T, E>` with fixed E). Never for simple types. Alias names add semantic meaning. Don't shadow std types without purpose.

### Dispatch Choice

- Static dispatch (generics) by default
- `dyn Trait` only for user-extensible plugin points or heterogeneous collections
- Cost: `&dyn` < `Box<dyn>` < `Arc<dyn>`. Never `Arc<dyn>` in hot paths.

## Clean Code Patterns

Micro-level code shape rules. These govern what happens *inside* a function body — complementing the macro-level architecture rules (SSOT, side logic, module boundaries) that govern cross-module structure.

### Parameter Hygiene

- **>4 parameters → config/options struct.** No exceptions for "they're all different types." A 5-param function signature is a design smell — the parameters are either related (group them) or the function does too much (split it).
- **>2 boolean parameters → flags enum or config struct.** Two bools in a signature means 4 implicit modes — name them. `draw_cell(cell, true, false, true)` is unreadable; `draw_cell(cell, &CellOptions { bold: true, ... })` is self-documenting.
- **Single bool parameter**: acceptable only when the call site reads naturally without checking the signature. Prefer a two-variant enum (`Mode::Skip` / `Mode::Include`) — it costs nothing and makes call sites unambiguous.
- **Audit trigger**: any function signature that wraps to 3+ lines in `rustfmt` output.
- **Dead parameters**: parameters that are unused or always passed the same constant value → remove. `#[allow(unused)]` on a parameter is a hygiene violation unless the parameter is required by a trait signature.

### Nesting Depth & Guard Clauses

Max 4 levels of nesting. These patterns keep the happy path at the left margin:

- **Invert and return early**: replace `if condition { ...long body... }` with `if !condition { return; }` followed by the body at top level.
- **`if let` chains 3+ deep → helper returning `Option`**: extract the chain into a function that uses `?` to short-circuit, then call it with a single `let Some(result) = helper() else { return; }`.
- **Nested `match` inside `match` → extract inner match**: if a match arm contains another multi-arm match, the inner match is a named function. Name it after what it decides.
- **`else` after diverging branch is noise**: `if condition { return x; } else { y }` → `if condition { return x; } y`. The `else` adds a nesting level for no reason.
- **Detection**: any `}` column indented past 16 spaces (4 levels × 4 spaces) in non-test code.

### Complex Conditionals → Named Predicates

Boolean expressions with 3+ clauses or any `&&`/`||` mix should be extracted to a named predicate method on the type being tested.

```rust
// BAD — reader must reverse-engineer the intent
if !cell.flags.contains(CellFlags::WIDE_CHAR) && !cell.flags.contains(CellFlags::WIDE_SPACER) && cell.c != ' ' {

// GOOD — intent is the name
if cell.is_normal_visible() {
```

- **Rule**: 3+ boolean clauses → named method or local `let` binding with a descriptive name.
- **Rule**: negated compound conditions (`!(a && b)`) → named predicate with positive name.
- **Detection**: grep for `&&` and `||` in the same `if`/`while` condition.

### Mixed Abstraction Levels

Each function should operate at **one level of abstraction**. A function that mixes orchestration calls with low-level implementation details is doing two jobs.

```rust
// BAD — high-level orchestration mixed with low-level GPU ops
fn draw_frame(state: &AppState) -> ... {
    let layout = compute_layout(state);      // high-level
    encoder.begin_render_pass(&desc);        // low-level GPU
    update_cursor_blink(state);              // high-level again
}

// GOOD — orchestration only; low-level details in callees
fn draw_frame(state: &AppState) -> ... {
    let layout = compute_layout(state);
    render_grid(&layout, &mut encoder);
    update_cursor_blink(state);
}
```

- **Detection**: if a function body mixes "calls to domain functions" with "raw encoder/builder/pointer operations", the raw operations should be in their own named function.
- **Exemption**: leaf functions that ARE the low-level implementation don't need further splitting.

### Magic Numbers and Strings

- **No literal numbers** in non-test code except `0`, `1`, `-1`, and well-known mathematical/algorithmic constants (`2` in binary operations, powers of two for alignment). Everything else gets a named `const`.
- **Thresholds** — buffer sizes, capacity hints, animation durations, margin pixels — **must** be named constants, not literals scattered across call sites. Example: `const CURSOR_BLINK_INTERVAL_MS: u64 = 530;`, not bare `530` in a timer.
- **Detection**: any numeric literal > 1 in a comparison, capacity, or threshold context. Any string literal in a `match` arm or `==` comparison outside of parsing/config handling.

### Temporal Coupling & RAII Guards

When correctness depends on calling methods in a specific order, the ordering **must** be enforced structurally, not by convention.

```rust
// BAD — early return or ? skips the cleanup
encoder.begin_render_pass(&desc);
let result = draw_cells(&grid)?;
encoder.end_render_pass();

// GOOD — Drop ensures cleanup on all exit paths
let pass = encoder.begin_render_pass(&desc); // pass ends on Drop
let result = draw_cells(&grid, &mut pass)?;
```

- **Rule**: paired `begin_*/end_*`, `push_*/pop_*`, `enter_*/exit_*` operations → RAII guard whose `Drop` performs the cleanup half. The guard ensures cleanup on all exit paths including `?`, `return`, and panic.
- **Rule**: if two functions must be called in sequence with no valid interleaving, combine them into one function or use a typestate/builder pattern where the compiler enforces ordering.
- **Detection**: grep for paired method names (`begin_`/`end_`, `push_`/`pop_`, `enter_`/`exit_`) that aren't wrapped in a guard pattern.

### Return Type Complexity

- **Tuples of 3+ elements → named struct.** Even `(PaneId, bool)` deserves a struct if the `bool` isn't obvious from context at every call site. Named fields are free documentation.
- **`Option<Option<T>>` → rethink the API.** Two layers of optionality usually means two separate concerns that should be modeled explicitly (e.g., a `LookupResult` enum with `Found(T)` / `NotApplicable` / `NotFound`).
- **`Result<Option<T>, E>` is fine** — "might fail, might not exist" is a legitimate two-axis concern. But `Option<Result<T, E>>` is almost always wrong — prefer `Result<Option<T>, E>`.
- **Detection**: return types with `(A, B, C, ...)` or nested `Option`/`Result` wrappers.

### Stringly-Typed Internals

Using `&str` or `String` where an enum or newtype would catch errors at compile time.

- **Finite valid values → enum.** If a string parameter is matched against known alternatives, those alternatives should be enum variants. The compiler catches typos and missing arms; string matching doesn't.
- **Domain concepts → newtype.** Mode names, action names, config keys — anything that represents an internal concept should be a distinct type, not a bare `String`.
- **Detection**: `match string_value { "foo" => ..., "bar" => ... }` outside of parsing or config handling. Any `HashMap<String, ...>` where the key space is known at compile time.

### Defensive Code for Impossible States

Trust internal invariants. If the surrounding code guarantees a condition, don't add runtime error handling for its negation.

```rust
// BAD — map was populated 3 lines above, key is guaranteed present
let Some(value) = map.get(&key) else {
    return Err(Error::new("unexpected missing key")); // dead code that hides real bugs
};

// GOOD — assert the invariant, don't handle the "failure"
let value = map[&key]; // panics if invariant violated — which IS the correct behavior
// or: debug_assert!(map.contains_key(&key));
```

- **Rule**: no error handling for conditions that the surrounding code path guarantees. Use `debug_assert!` — it documents the assumption AND catches violations in debug builds. A runtime fallback (`unwrap_or_default`, `else { return }`) hides the bug instead of surfacing it.
- **Rule**: `"this should never happen"` / `"just in case"` in a comment is a code smell. If it can't happen, `unreachable!()` with context. If it *can* happen, handle it properly.
- **Detection**: `else` branches or `match` arms with comments containing "should never", "just in case", "unexpected", "shouldn't happen".

### No Premature Abstraction

Abstractions earn their existence by having multiple consumers. A trait with one implementor, a factory that builds one type, or a wrapper that adds nothing is indirection without value.

- **Single-implementor traits → delete the trait.** Use a concrete type. Add the trait later when a second implementor actually appears. Exception: traits required by external interfaces or documented extension points with a stated design reason.
- **Builder for <3 fields → direct construction.** `Foo { a, b }` is clearer than `Foo::builder().a(a).b(b).build()`. Builders earn their keep at 4+ fields, or when some fields have defaults/validation.
- **Factory that constructs one product → `new()`.** A `FooFactory` that only ever produces `Foo` is a naming ceremony, not a pattern.
- **Observer/listener with one subscriber → direct call.** Event systems earn their keep with 2+ subscribers or runtime registration.
- **Detection**: `trait T` with exactly one `impl T for X` and no documented extension intent. Any `*Factory`, `*Builder`, `*Manager`, `*Handler` type that wraps a single operation.

### No Gratuitous Intermediates

- **`let result = expr; result` → just `expr`.** If a binding's only purpose is to be immediately returned, inline it.
- **`let x = foo(); bar(x)` where `x` is used once → `bar(foo())`**, unless the intermediate name adds genuine clarity or the expression is complex enough that a name aids debugging.
- **Exception**: intermediates that aid debugger breakpoints or that name a non-obvious value are fine.
- **Detection**: any `let` binding that is used exactly once, on the immediately following line, with no meaningful name (e.g., `result`, `value`, `output`, `ret`, `tmp`).

### No Symmetry for Its Own Sake

Methods exist because callers need them. An API is not a checklist to "complete."

- **No unused getters/setters.** A `set_name()` that nothing calls is dead code. Add it when a caller needs it, not when `get_name()` exists.
- **No speculative conversion methods.** `to_foo()` / `from_foo()` pairs are not mandatory. Implement the direction that's actually used.
- **Detection**: any `pub` method with zero call sites outside its own module and tests. `#[allow(dead_code)]` on a method is a hygiene violation — either the method is needed or it isn't.

### No Cargo-Culted Design Patterns

Design patterns are solutions to *specific problems*. Name the problem before applying the pattern. If you can't articulate the problem, you don't need the pattern.

- **Builder**: justified at 4+ fields, optional fields with defaults, or construction validation. Unjustified for simple structs.
- **Observer**: justified at 2+ runtime-registered subscribers. Unjustified for a single static call.
- **Strategy**: justified when the algorithm varies at runtime or is user-extensible. Unjustified when a simple `match` on an enum covers all cases.
- **Wrapper/Decorator**: justified when adding cross-cutting behavior (logging, caching, retry). Unjustified when the wrapper delegates every method with no added logic.
- **Detection**: any pattern where removing the abstraction layer and inlining the logic makes the code shorter AND clearer.

### No Narrating Comments

Comments that describe control flow as it happens are noise. They restate the code in English without adding information.

```rust
// BAD — every comment restates the next line
// Check if the grid is empty
if grid.is_empty() {
    // Return early since there's nothing to render
    return;
}
// Iterate over each row
for row in grid.visible_rows() {
    // Process the current row
    render_row(row);
}

// GOOD — no comments needed; the code is the documentation
if grid.is_empty() {
    return;
}
for row in grid.visible_rows() {
    render_row(row);
}
```

- **Rule**: delete any comment that can be derived by reading the next 1-3 lines of code. Comments earn their existence by explaining *why*, not *what*.
- **Narration keywords that signal noise**: "now we", "next we", "first we", "then we", "handle the case where", "check if", "iterate over", "return the result".
- **"Obvious decision" comments are noise**: `// We use HashMap for O(1) lookups` — nobody was going to use a Vec. Comment the *surprising* choice, not the obvious one.

## Clone Discipline

- Clone acceptable on cold/error paths and test setup
- Prefer `&str`/`Cow` over `String` at boundaries
- `Arc` only for shared ownership across threads/tasks (IO thread ↔ main thread)
- No `.clone()` in hot paths without a comment justifying it

## Performance Annotations

- `#[cold]` on error factory functions and unlikely branches
- `#[inline]`: 1-5 lines freely. 6-20 lines only if profiling shows benefit or cross-crate hot path. >20 lines never.
- **Size assertions** on types in per-cell arrays or passed by value in hot loops. Add when size exceeds 2 machine words (16 bytes on 64-bit): `const _: () = assert!(size_of::<T>() == N);`
- `#[must_use]` on all pub functions returning `Result`/`Option`, builder methods returning `Self`, and pure functions where ignoring the return is always a bug.

## Unsafe & FFI

- Every `unsafe` block requires a `// SAFETY:` comment explaining the invariant
- Minimize unsafe scope — extract safe logic outside the unsafe block
- `unsafe` justified only for: platform FFI (winit, wgpu interop), raw pointer operations (GPU buffer mapping), performance-critical hot paths where safe alternatives measurably regress
- **Platform-specific code**: isolate behind `#[cfg(target_os)]` blocks with implementations for all three platforms (macOS, Windows, Linux). Abstract platform differences behind shared interfaces.

## Panic & Assertion

- **Never panic on user input**: bad escape sequences, invalid PTY output, unexpected config values — all must be handled gracefully
- **`.unwrap()`**: only with comment proving infallibility, or in tests. Production code: `.expect("reason")` or propagate with `?`.
- **`assert!()`**: for invariants whose violation would cause unsound behavior or safety issues. Always include a message.
- **`debug_assert!()`**: for expensive invariant checks (O(n) or worse)
- **`unreachable!()`**: for impossible code paths. Include context message. Never `panic!()` for impossible states.

## Narrow the Front

- **Complete one fix fully before starting another.** Rendering + input + state interactions multiply failure surfaces. Concurrent changes across these domains compound risk.
- **"Fully" means**: fix + tests + plan update. A fix without tests is incomplete. A fix with tests but without plan update (when cross-section) is incomplete.
- **Prefer depth over breadth.** Fix one widget type across all interaction states before fixing a second widget type. Fix one pipeline pass completely before the other. This reduces the number of concurrent moving parts and makes failures narrow and explainable.

## Lifetime Annotations

- Prefer elision when possible
- Descriptive names for long-lived borrows: `'grid`, `'frame`, `'ctx`
- Single-letter (`'a`) only for local/obvious cases
- Avoid >2 lifetime parameters per function

## API Stability

- Pub items in `lib.rs` are the stable API surface
- Breaking changes to pub crate APIs must update all downstream consumers in the same commit
- When replacing a code path, remove the old code in the same commit. No deprecation for internal code.

## Dependencies

- Prefer `std` over external crates
- New external deps require justification
- Features are additive only (never remove functionality). Each feature documented in `Cargo.toml`.

## Commit Hygiene

- One logical change per commit; conventional commit format (`feat`/`fix`/`refactor`)
- Cross-crate changes that must be in sync go in a single commit
- Large refactors broken into phases: (1) add new API alongside old, (2) migrate consumers, (3) remove old API. Never break the build between phases.

## Technical Debt

- Fix when you find it. If it can't be fixed in the current change, add an entry to the active plan or create a roadmap item. No untracked debt.
- Experimental/prototype code lives in feature branches, never in dev/master.
