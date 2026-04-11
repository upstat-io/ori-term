---
paths:
  - "**/tests/**"
  - "**/tests.rs"
  - "**/*_test*.rs"
  - "**test**"
---

# ori_term Testing Rules

**Tests are source of truth.** Test fails = code is wrong, not the test.
**Tests are MANDATORY.** There are zero scenarios where skipping tests is acceptable. Every code change — bug fix, feature, refactor, optimization — requires tests. No exceptions.

## What This File Is For

ori_term is a GPU-accelerated terminal emulator (Rust, wgpu, winit). Its test surface is **not** a language compiler — there is no Ori spec runner, no interpreter vs LLVM parity, no ARC/AOT cross-phase verification. Anything below is framed around the actual test layers this project runs:

- **Crate unit tests** via sibling `tests.rs` files (`cargo test -p <crate>` or `./test-all.sh`)
- **Integration tests** in each crate's `tests/` directory (e.g. `oriterm_core/tests/`, `oriterm/tests/`)
- **Widget harness tests** via `WidgetTestHarness` (`oriterm_ui/src/testing/`) — headless widget interaction, no GPU / display server required
- **GPU cached render path tests** in `oriterm/src/gpu/visual_regression/` — must use `render_frame_cached()` not `render_frame()` (see CLAUDE.md §GPU Render Path Testing)
- **Terminfo / terminal conformance** via `oriterm_core/tests/teseq/`, `oriterm_core/tests/tack/`, `oriterm_core/tests/vttest/` and their mirrors under `oriterm/src/gpu/visual_regression/`
- **Allocation regression** via `oriterm_core/tests/alloc_regression.rs` and `oriterm_core/tests/rss_regression.rs` — enforces the zero-allocation hot render path and stable RSS invariants
- **Control-flow / event-loop purity** via `oriterm/src/app/event_loop_helpers/tests.rs` — pure `compute_control_flow()` tests that enforce the zero-idle-CPU invariant
- **Cross-platform build** via `cargo build --target x86_64-pc-windows-gnu` — Windows cross-compile from WSL, plus native macOS/Linux in CI

## TDD for Bugs

The TDD *commitment* (you MUST do TDD, use `/fix-bug`) is in CLAUDE.md §Bug Discipline. This section details the *methodology*.

1. STOP — don't jump to fixing
2. Consult the reference repos for intended behavior when the bug touches a protocol or convention that tmux / alacritty / wezterm / ghostty / ratatui / ptyxis already solved
3. Write MATRIX tests — not just "multiple":
   - **Exact failing case**: the specific input (escape sequence, widget event, font config, GPU target size) that triggered the bug
   - **Edge cases**: empty input, single-cell grid, zero-sized window, CJK at column 0, empty selection, 1×1 surface, zero-width combining mark
   - **Cross-type matrix**: if the fix is scalar-type-dependent, test all relevant types through the same code path (`u8`, `u16`, `u32`, `usize`, `f32`, `Color`, `Cell`, `WidgetAction`, `KeymapAction`)
   - **Cross-pattern matrix**: if the fix is control-flow-dependent, test all relevant flow patterns (`for`, `while`, `loop { break }`, iterator chain, early return, short-circuit)
   - **Cross-feature matrix**: test interactions with other subsystems that flow through the same code path (see §Interaction Testing)
   - **Semantic pin**: at least one test that ONLY passes with the new behavior — the permanent regression guard
   - **Negative pin**: at least one assertion that REJECTS the old/broken behavior — proves the code actively prevents the regression, not just happens to avoid it
4. Verify tests FAIL (proves understanding)
5. Fix the code
6. Tests pass WITHOUT modification
7. Verify matrix completeness — missing cells are future regressions

## Matrix Testing Rule

**Every fix that touches a code path shared by multiple types, patterns, or platforms requires matrix coverage.** A fix to input dispatch that works for `KeyEvent::Enter` but isn't tested with `Tab`, `Shift+Tab`, `Ctrl+C`, `mouse_down`, and `scroll` is incomplete. Dimensions:

- **Type dimension**: all event / value / style types that flow through the fixed code path
- **Pattern dimension**: all control-flow / propagation patterns that exercise the fixed code path (widget tree recursion, capture vs bubble, focus ring traversal)
- **Feature dimension**: all subsystems that interact with the fixed code path (see §Interaction Testing)
- **Platform dimension**: debug + release builds, Linux (host) + Windows (cross-compile) + macOS (CI). Any `#[cfg(target_os = ...)]` branch must have a counterpart test per branch.
- **Surface dimension**: if the fix touches GPU rendering, verify both `render_frame_cached()` (production path) and the prepare/paint split — do NOT rely on `render_frame()` which skips content caching and hides real bugs

A fix is complete when the matrix is covered. Missing cells are potential regressions waiting to happen.

**Matrix squeeze principle**: Each matrix test narrows the gap between "works" and "crashes," triangulating the bug from multiple angles. When the matrix is dense, the correct fix surface becomes surgically obvious — all surrounding cases are pinned, so the fix must thread precisely between them.

**Self-verifying matrix completeness**: When writing matrix tests that iterate over types or patterns, include a count assertion that proves every cell was visited:
```rust
let mut count = 0;
for ty in ALL_INPUT_KINDS { for phase in ALL_PROPAGATION_PHASES { test(ty, phase); count += 1; } }
assert_eq!(count, ALL_INPUT_KINDS.len() * ALL_PROPAGATION_PHASES.len()); // proves no cells skipped
```

## Matrix Clamping — Pinning Correct Behavior from All Sides

Matrix clamping uses tests to **narrow the solution space** until only the correct fix survives. Each matrix cell is a clamp — a constraint that pins behavior from one angle.

- **Clamp from above and below**: for every "should work" cell, add a corresponding "should fail" cell at the boundary. If `Alt+Enter` produces one action, does `Alt+Shift+Enter` produce a *different* action (not a crash, not the same action)?
- **Clamp across type boundaries**: if a fix touches a code path shared by `KeyEvent`, `MouseEvent`, `WheelEvent`, and `TouchEvent`, pin all four.
- **Clamp across pattern boundaries**: if a scroll fix works for `direction: up`, clamp it with `down`, `at top`, `at bottom`, `during selection`, `during mark mode`.
- **Clamp across feature boundaries**: if the fix interacts with selection, mark mode, overlays, focus, or animation, add cells for each interaction.
- **The squeeze effect**: when all surrounding cells are clamped, the correct fix surface is surgically obvious.
- **Completeness test**: after writing the matrix, ask: "could a *different* fix also pass all these tests?" If yes, add a cell that distinguishes the correct fix.

**Fix completeness checklist** — a fix is NOT done until:
- [ ] Matrix tests cover every relevant type × pattern × feature × surface combination
- [ ] At least one semantic pin test would fail if the fix is reverted
- [ ] At least one negative pin rejects the broken behavior
- [ ] Positive + negative pairing: every "should work" test has a corresponding "should fail" counterpart
- [ ] Debug AND release builds pass
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green (workspace + Windows cross-compile)
- [ ] Teseq / tack / vttest suites green if the fix touches VT handling, terminfo, or visual rendering
- [ ] If the fix touches the GPU render path, visual-regression suite under `oriterm/src/gpu/visual_regression/` green on both cached and prepared paths

## Interaction Testing — Feature × Feature (MANDATORY)

**Every feature must be tested in combination with other features it can interact with.** Terminal emulators break at feature boundaries, not within features. A widget that works in isolation but fails under a resize, during animation, or inside an overlay is a real bug that users will hit.

**When implementing or fixing feature A, test A × B for every relevant B:**

| If A touches...                  | Also test with...                                                                                           |
|---|---|
| Grid / VTE parsing               | Reflow, scrollback, selection, mark mode, damage tracking, resize during output                             |
| Widget input                     | Focus changes, overlays, drag-in-progress, animation in flight, scale change, disabled state                |
| GPU rendering                    | Cached path (`render_frame_cached`), resize race (viewport vs surface mismatch), DPI change, device loss    |
| Color detection                  | `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`, `COLORTERM`, `TERM=dumb`, non-TTY stdout                          |
| Unicode width                    | CJK, emoji (base + variation selectors), combining marks, ZWJ sequences, RTL / BiDi                        |
| Selection                        | Copy, search, mark mode, reflow, scrollback expansion, linear vs rectangular vs semantic                    |
| Font pipeline                    | Multi-size registry, fallback fonts, live config reload, shaping cache invalidation                         |
| PTY / ConPTY                     | Write blocking, read buffering, resize mid-output, process death, backpressure                              |
| Session model (tabs/splits)      | Focus changes, directional nav, mux events, pane close-during-animation, layout recomputation               |
| Animation                        | Pause/resume on visibility, scale change, input during animation, multiple concurrent animations            |
| Config reload                    | Font swap, theme swap, keybind reload, pane resize, scrollback budget change                                |

**Minimum interaction coverage**: for any new feature or fix, test at least 3 cross-feature interactions. For features touching the hot render path (prepare/paint), test at least 5 (cached path, resize race, scale change, damage invalidation, scrollback scroll).

## Cross-Platform Verification (MANDATORY)

**Every `#[cfg(target_os = "...")]` branch must have tests.** ori_term targets Linux, macOS, and Windows. The build/test matrix:

1. **Native Linux** — `./test-all.sh` on the host (WSL or Linux)
2. **Windows cross-compile** — `cargo build --target x86_64-pc-windows-gnu` must succeed (CI also runs native Windows)
3. **macOS** — CI-only; locally verify via code inspection against `tmux`, `alacritty`, `ghostty` macOS code paths
4. **Teseq scenarios** — Linux-only (`sudo apt install teseq`); tests must skip gracefully on macOS/Windows (no panic, no false failure)
5. **Tack / vttest** — conditional on tool availability; must skip cleanly when the tool is missing (see §Graceful Skip Protocol below)
6. **Architecture tests** — `cargo test -p oriterm --test architecture` enforces crate-boundary rules

If a feature cannot be implemented on a platform, it must degrade gracefully with a compile-time `cfg` gate, not a runtime panic. Every `#[cfg(target_os = "...")]` block must have counterparts for all supported targets — no platform left behind.

## Graceful Skip Protocol

Tests that depend on an external tool (`teseq`, `tack`, `tic`, `infocmp`, GPU adapter, display server) **must** detect the tool's absence and skip without failing.

- Detect via a helper (`PtySession::ensure_tack_available`, `reseq::which`, `wgpu::Instance::request_adapter`) at the top of the test
- On absence: log a `SKIP: <reason>` message and `return` early — do NOT `panic!`, do NOT `assert!(false)`
- The skip MUST be visible in test output so a missing tool is not silently hidden; use `eprintln!("SKIP: tack binary unavailable")`
- Every skip path MUST have at least one "also runs" companion test that is NOT gated on the tool, so the matrix is never fully empty on a platform

Drift-gate tests — `cap_coverage_matrix`, `begin_testing_inventory`, `tools_menu_inventory`, `status_reports_inventory` — run unconditionally on ALL platforms and enforce that gate regressions are impossible to ship.

## Performance Invariants (MANDATORY for hot-path changes)

These are enforced by regression tests that must NEVER be disabled. See CLAUDE.md §Performance Invariants for the full contract.

1. **Zero idle CPU beyond cursor blink** — enforced by `compute_control_flow()` pure-function tests in `oriterm/src/app/event_loop_helpers/tests.rs`. When idle, the event loop sleeps via `ControlFlow::Wait`; the only wakeup source is the cursor blink timer. Any change that touches event-loop scheduling MUST re-run these tests.
2. **Zero allocations in hot render path** — enforced by `oriterm_core/tests/alloc_regression.rs`. The IO thread calls `renderable_content_into()` into a reusable buffer, then `SnapshotDoubleBuffer::flip_swap()` exchanges it with the front buffer via `std::mem::swap()`. All `Vec` buffers are reused via `.clear()` + capacity retention. Any change that allocates per-cell or per-frame MUST fail these tests.
3. **Stable RSS under sustained output** — enforced by `rss_stability_under_sustained_output` in `oriterm_core/tests/rss_regression.rs`. Scrollback is bounded, image caches evict, GPU textures drop. Any change that adds an unbounded growth vector MUST fail these tests.
4. **Buffer shrink discipline** — grow-only `Vec` buffers (instance writers, shaping scratch, notification buffer, `RenderableContent` fields) apply `maybe_shrink()` post-render. No shrinking during `draw_frame()` (pure computation, no side effects).

**Never disable a regression test to ship a fix.** If a regression test fails, the fix is wrong — investigate, fix the fix, re-run. Disabling the regression turns the invariant into a ghost — still documented, no longer enforced.

## Negative Testing Protocol (MANDATORY)

**Every test suite must include negative tests.** A test suite with only positive tests ("this works") provides no protection against the code becoming too permissive.

1. **Must-reject tests pin exact rejections**: when adding input validation (e.g. "malformed escape sequence must be dropped without crashing"), write a test that feeds the malformed input and asserts the expected rejection behavior. `should_panic` tests are allowed only when the code path is genuinely unreachable from safe inputs; prefer `Result::Err` assertions.

2. **Must-not-render tests for damage tracking**: when adding a damage-tracking optimization, write a companion test where the optimization must NOT fire (e.g. cell marked dirty → MUST be redrawn). A one-sided test (only checks "it re-renders") misses the regression where the optimization fails to mark a real change dirty.

3. **Must-not-allocate tests for hot path**: when claiming an allocation-free path, wrap the inner loop in `oriterm_core::tests::alloc_counter::measure!` and assert `0` allocations. A test that only checks output correctness misses silent allocation regressions.

4. **Must-not-leak tests for GPU resources**: texture / buffer / bind-group drops must be verified — not just by running in debug builds, but by explicit leak detection tests. A test that drops a texture and immediately reuses the `wgpu::Device` without checking `poll(Wait)` may miss resource-retention bugs.

5. **Forbid-output pins**: when a fix changes behavior (e.g. a warning is no longer emitted, or a color is no longer applied), add a test that asserts the OLD behavior does NOT appear. This is a stronger guarantee than just checking the new behavior is present.

6. **Idempotency tests**: when fixing a layout pass, resize handler, or damage invalidator, verify that running it twice produces the same result as running it once. A non-idempotent pass is a bug.

## Regression Discipline

**Every bug fix creates a permanent regression test.** The test carries a doc comment linking to the bug or plan item that motivated it:

```rust
/// Regression: BUG-11-1 — cursor blink kept frame budget gate open
/// See: plans/bug-tracker/fix-BUG-11-1.md
#[test]
fn blink_animating_does_not_bypass_frame_budget() { ... }
```

**Regression test naming**: `<subject>_<scenario>_<expected>` shape. No ephemeral identifiers (plan names, section numbers, bug IDs) in function names — provenance in `///` doc comments only. Full naming convention and banned-descriptor list: `impl-hygiene.md` §Test Function Naming.

**Crash regression tests**: if any component ever panics on valid input, that input becomes a permanent test — even before the fix is identified. Add it immediately as an `#[ignore]` test with a tracking doc comment if it can't be fixed yet, but it MUST be recorded.

## Test Hygiene

1. **No orphan tests**: every test file must contain at least one assertion (`assert_eq`, `assert`, `assert_matches`, `insta::assert_snapshot!`, or similar). A test that runs code but never asserts anything proves nothing and provides false confidence. The assertion IS the test.

2. **No trivial assertions**: `assert_eq!(true, true)` or `assert_eq!(1, 1)` are not tests — they're tautologies. The assertion must test a value that was computed by the code under test.

3. **`#[ignore]` budget**: a file with more than 3 `#[ignore]` annotations is a red flag. Either the feature is not implemented (and the tests should be in a plan, not committed ignored) or there are bugs to fix. Each `#[ignore]` must have a doc comment tracking its resolution.

4. **Stale snapshot detection**: if an `insta` snapshot is pending (`*.snap.new`), the test has produced unreviewed output. Worktree cleanliness is a completion criterion — `git status --porcelain -- '*.snap.new' '*.png'` must be empty at green build.

5. **Test file layout**: Rust tests live in sibling `tests.rs` files per `.claude/rules/test-organization.md`. `#[cfg(test)] mod tests;` at the bottom of the source file, body in `tests.rs`. No inline `#[cfg(test)] mod tests { ... }` blocks.

## Flaky Tests Are Bugs

Per CLAUDE.md §Bug Discipline, a test that passes sometimes and fails sometimes is a bug — not noise. Do NOT retry and move on. Research the root cause:

- Race condition (IO thread vs main thread, GPU command submission ordering)
- Timing dependency (frame-counter assumptions, animation easing curves, blink timers)
- Temp file collision (parallel test runs sharing a path)
- State leakage (global statics, `static mut`, `OnceLock` not reset between tests)
- Non-deterministic ordering (`HashMap` iteration, `std::fs::read_dir` order)
- GPU device-loss timing (lost adapter during test)
- Surface reconfiguration races (resize during render)

Fix it so the test is deterministic. File via `/add-bug` if discovered during a different fix. **Flake-proofing gate for conformance suites**: the tack + vttest suite must pass 5 consecutive runs at both `--test-threads=1` and `--test-threads=4` before a release is cut.

## Widget Harness Testing

`WidgetTestHarness` (`oriterm_ui/src/testing/`) enables headless widget interaction testing without GPU, display server, or platform dependencies. It wraps `WindowRoot` and is the primary test vehicle for any widget in `oriterm_ui`.

**Writing a new harness test**:
```rust
let mut h = WidgetTestHarness::new(ButtonWidget::new("OK"));
h.mouse_move_to(button_center);
assert!(h.is_hot(button_id));
h.click(button_center);
let scene = h.render(); // paint capture — a Scene, no GPU required
```

Key APIs: `mouse_move()`, `mouse_down()`, `mouse_up()`, `click()`, `key_press()`, `tab()`, `shift_tab()`, `scroll()`, `drag()`, `type_text()`, `advance_time()`, `resize()`, `render()`, `is_hot()`, `is_active()`, `is_focused()`, `interaction_state()`, `get_widget()`, `all_widget_ids()`, `widgets_with_sense()`, `push_popup()`, `has_overlays()`, `dismiss_overlays()`.

**Rule**: every widget with input senses (hover / click / drag / keyboard / focus) MUST have at least one harness test covering each sense. A widget that owns a sense and has no harness test for that sense is untested — fix it, don't ship it.

## GPU Cached Render Path Testing

The production render path is **content-cached**: content is rendered to an offscreen cache texture, then copied to the surface via `copy_texture_to_texture`. Test-only `render_frame()` skips this entirely — bugs in the cached path are invisible to `render_frame()`. See CLAUDE.md §GPU Render Path Testing.

**Use `render_frame_cached()`** when testing GPU rendering under resize or any condition where viewport and surface dimensions may diverge:
```rust
renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
renderer.render_frame_cached(&gpu, &pipelines, target_w, target_h, true);
```

Use `gpu.create_copy_dst_target()` when manually creating destination targets (adds `COPY_DST` usage to simulate a surface texture).

## Terminal Conformance Suites

- **teseq** (`cargo test -p oriterm_core --test teseq`) — 176 tests across 10 protocol families. Requires `reseq` (Linux only; `sudo apt install teseq`). Update snapshots with `INSTA_UPDATE=1`.
- **tack** (`cargo test -p oriterm_core --test tack`) — 27 PTY scenarios + 51 direct-VTE cap xcheck in `tack_cap_xcheck`. Tack-dependent tests skip on Windows; cap xcheck runs on all platforms.
- **vttest** (`cargo test -p oriterm_core --test vttest`) — vttest menu structural markers for DA/DSR responses.
- **GPU visual regression** (`cargo test -p oriterm --test main_window` etc.) — golden-image regression for the cached render path.

When adding a new escape sequence handler or terminfo capability, the matching conformance test MUST be added in the same commit as the handler. No handler merges without a test.

## Anti-patterns (NEVER)

- Remove a test "because it's flaky" — investigate WHY, fix the determinism
- Change expected to match actual without proving the new actual is correct — fix the code, not the test
- Assume the reference implementation is wrong without consulting the reference repos (`~/projects/reference_repos/console_repos/`)
- Delete "redundant" tests — they may cover different platforms, different widget states, or different event orderings
- Mark `#[ignore]` without a doc comment and tracking issue
- Test only one type when the code path handles multiple types — matrix coverage required
- Test only the happy path when drop/resize/error/cancel are possible — pattern coverage required
- Write only positive tests — negative pins are equally required
- Write a test without an assertion — running code is not testing it
- Test a widget in isolation without harness interaction tests — widgets break at propagation boundaries
- Ship a fix that disables a regression test — the invariant IS the product
- Use `render_frame()` to "verify" a cached-path fix — bugs in the cached path are invisible to `render_frame()`
- Accept "it works on my machine" — debug AND release, Linux AND Windows (cross), macOS (CI), all must pass

## Investigation Order

When a test fails and you're unsure whether it's a test bug or a code bug:

1. Reference repo: does tmux / alacritty / wezterm / ghostty / ratatui / ptyxis handle this case? Read their code.
2. Protocol spec: for VT sequences, check vt100.net, XTerm ctlseqs, ECMA-48. For terminfo, check `man 5 terminfo`.
3. VTE parser: is the sequence getting parsed at all? Add a trace, re-run.
4. Term handler: does `term_handler.rs` have a handler for this sequence?
5. Grid mutation: is the grid actually being updated correctly?
6. Snapshot / render: is the change flowing through `renderable_content_into()` to the GPU?
7. ONLY THEN consider the test is wrong.

## Running Tests

- `./test-all.sh` — full workspace suite (use this — it includes clippy + fmt verification)
- `./build-all.sh` — workspace build including Windows cross-compile
- `./clippy-all.sh` — zero-warnings clippy across all crates + targets
- `./fmt-all.sh` — format all Rust sources
- `cargo test -p <crate>` — single crate
- `cargo test -p oriterm_core --test teseq` — teseq suite (Linux only)
- `cargo test -p oriterm_core --test tack` — tack PTY + cap xcheck
- `cargo test -p oriterm_core --test vttest` — vttest structural markers
- `cargo test -p oriterm_ui` — widget harness + animation + layout
- `cargo test -p oriterm --test architecture` — architecture / boundary tests
- `cargo test -p oriterm --test main_window` — GPU visual regression (main window scene)
- `INSTA_UPDATE=1 cargo test -p oriterm_core --test teseq` — update insta snapshots

**Mandatory timeout**: every test command MUST use a 150-second timeout (see CLAUDE.md §MANDATORY TEST TIMEOUTS). `timeout 150 cargo test ...` or `Bash.timeout: 150000`. If tests exceed the timeout, you introduced a hanging test — fix it, don't extend the timeout.

## Property-Based Testing

`proptest` is available in the workspace for invariants that benefit from randomized input:

- **Roundtrip**: `parse(render(grid)) == grid` for VT output round-trip, `format(parse(config)) == config` for config file round-trip
- **Determinism**: `render(snapshot) == render(snapshot)` — the same input must produce the same output
- **Layout invariants**: any widget tree laid out at size W must produce rects that stay inside W and respect z-order
- **Fuzz-to-crash**: parser must not panic on any input, ever: `proptest! { |bytes: Vec<u8>| { let _ = vte_parser.advance(&mut handler, &bytes); } }`
- **Observational equivalence**: for damage tracking, `render_full(scene) == render_damaged(scene)` when the damage is trivially everything

Property tests live in the same sibling `tests.rs` file as unit tests, using `proptest!` blocks.

## Prior Art Reference

These rules are derived from production terminal-emulator and UI-framework testing practices:

- **tmux**: `grid.c` cell storage with extended-cell fuzzing, `input.c` 83k-line VT parser test suite, `window-copy.c` selection tests
- **alacritty**: `vte`-crate parser tests, `damage` unit tests, strict clippy as regression surface
- **wezterm**: `termwiz` parser corpus, `portable-pty` cross-platform tests, color profile detection tests
- **ghostty**: comptime parser state-machine tests, Valgrind integration, Metal/OpenGL/WebGL matrix testing
- **ratatui**: `TestBackend` buffer-based widget tests, Unicode-width matrix (CJK, emoji, combining, ZWJ), `unsafe_code = "forbid"`
- **crossterm**: `io::Result<T>` everywhere, `queue!`/`execute!` macro test corpus, platform-specific test gating
- **ptyxis**: libvte consumer tests, container-detection matrix, GPU rendering under GTK4
- **termenv**: color profile detection tests for every env var combination (NO_COLOR / CLICOLOR / CLICOLOR_FORCE / COLORTERM / TERM)

When a bug or test failure touches an area covered by one of these reference repos, read their code before writing the fix. They have already seen this bug.
