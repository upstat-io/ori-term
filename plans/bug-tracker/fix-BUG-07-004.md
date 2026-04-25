---
bug: "BUG-07-004"
title: "Windows PTY size propagation test removed"
severity: "medium"
status: complete
goal: "PTY size propagation is verified on Windows (ConPTY) as well as Unix — both branches assert that a PTY opened at 33×97 delivers 33 rows × 97 cols to the spawned child."
success_criteria:
  - "A `#[cfg(windows)]` test in `oriterm_core/tests/vttest/pty_size.rs` opens a ConPTY at 33×97 (and 50×40 as the negative pin), spawns `cmd /d /c mode con`, and asserts the child observed the requested dimensions."
  - "`cargo build --target x86_64-pc-windows-gnu --tests -p oriterm_core` builds the Windows test (compile gate)."
  - "`cargo test -p oriterm_core --test vttest pty_size_propagation` matches both the Unix and the Windows branches by substring; the suite reports a non-skipped pass on each platform's branch."
subsystem: "oriterm_core/tests/vttest/pty_size.rs"
found: "2026-04-02"
source: "tpr-review"
third_party_review:
  status: none
  updated: null
---

# Fix: BUG-07-004 — Windows PTY size propagation test removed

**Status:** Complete
**Severity:** medium
**Goal:** PTY size propagation is verified on every supported platform. The single Unix-gated test today asserts the invariant for Linux/macOS only; Windows ConPTY size handling — a distinct code path inside `portable-pty` — has zero regression coverage. After this fix, both code paths are pinned by a test that asserts the same observable invariant: a PTY opened at `(rows: 33, cols: 97)` delivers a 33×97 size to the child running inside it.

**Success Criteria:**
- [x] Windows-native test added under `oriterm_core/tests/vttest/pty_size.rs` (gated by `#[cfg(windows)]`) that uses `portable_pty::native_pty_system()` (returns ConPTY on Windows) to open a `(33, 97)` PTY (plus a `(50, 40)` negative-pin case), spawns `cmd /d /c mode con`, and asserts the printed size matches.
- [x] Both tests share a private helper (`assert_pty_reports_size(rows, cols, cmd, parse)`) so the Unix and Windows branches differ only in the command spawned and the output parser — the assertion logic stays single-sourced.
- [x] `cargo build --target x86_64-pc-windows-gnu --tests -p oriterm_core` succeeds (Windows-cross compile gate verifies the new code compiles even from the WSL dev loop).
- [x] `./test-all.sh` (Linux host) green; the Unix test still runs and the Windows test compiles + is excluded by `cfg(windows)`.
- [x] `cargo test -p oriterm_core --test vttest pty_size_propagation` matches both renamed tests by substring and reports passes on each platform's branch (filter must NOT use `pty_size_is_propagated` — that name is gone after the rename).

**Context:** The original `pty_size_is_propagated` test was gated to Unix in commit history when ConPTY support landed without a paired Windows test. ConPTY's size-handling code path (`ResizePseudoConsole` + `OpenPseudoConsole` size struct) is structurally distinct from the POSIX `openpty` + `TIOCSWINSZ` path — a regression in either branch is invisible to the other's test. CI on Windows would catch a ConPTY size break only if a downstream feature happened to read the size; right now nothing does, so a silent regression here ships uncaught. Filed by tpr-review on 2026-04-02 against `oriterm_core/tests/vttest.rs:226`.

---

## 1. Root Cause Analysis

- **Symptom**: `pty_size_is_propagated` carries `#[cfg(unix)]` (`oriterm_core/tests/vttest/pty_size.rs:4`), so on the Windows cross-compile target the test compiles to nothing. There is no companion `#[cfg(windows)]` test exercising ConPTY size propagation.
- **Proximate cause**: When the Unix test was originally written it shells out to `stty size` (a POSIX `coreutils` command). Rather than write a parallel Windows path that does not require `stty`, the test was simply gated to Unix.
- **Root cause**: The test couples its assertion to a Unix-only command. It conflates "verify portable-pty's size propagation" (a portable invariant) with "use `stty` to read the size" (a Unix-only mechanism). Splitting those concerns lets both platforms be tested.
- **Blast radius**: Windows-only ConPTY size regressions are invisible to local CI. Examples that would slip through today: a regression in `crates/portable-pty` (vendored) that drops `cols` when `cols < rows`; a Windows API binding upgrade that flips a `rows`/`cols` argument; a future ori_term change that bypasses `portable-pty` and calls `OpenPseudoConsole` directly without forwarding the requested size.
- **Affected files**:
  - `oriterm_core/tests/vttest/pty_size.rs` — single file change. Add a `#[cfg(windows)]` companion test that uses a Windows-native command to read the size the child observes; refactor the shared assertion logic into a small helper so both branches share the invariant check rather than duplicating it.

**Reference implementations:**
- **portable-pty upstream** (`crates/portable-pty/src/win/conpty.rs`): on Windows, `native_pty_system().openpty(PtySize { rows, cols, .. })` calls `CreatePseudoConsole` with the requested `COORD { X: cols, Y: rows }`. The child sees that size via `GetConsoleScreenBufferInfo`. The test must therefore exercise the same path: `pty_system.openpty(...)` → spawn a Windows command that calls `GetConsoleScreenBufferInfo` (or equivalent shell builtin) → parse the printed dimensions.
- **wezterm** (`~/projects/reference_repos/console_repos/wezterm/term/src/test/mod.rs` family): does not include a portable PTY-size acceptance test; their conformance is via `portable-pty`'s own crate-level tests. We are filling a gap upstream chose not to fill at the consumer layer.

**Windows command to use:** `cmd.exe /c mode con` is the most robust ConPTY-friendly choice — it is built into `cmd.exe` (always present on Windows, no install required), respects the active console's reported size, and prints rows/cols in plain text. PowerShell's `[Console]::WindowWidth/Height` is an alternative but adds a `pwsh`/`powershell` invocation cost and a parser dependency on PS quoting. `mode con` output sample at 33×97:

```
Status for device CON:
----------------------
    Lines:          33
    Columns:        97
    ...
```

The parser pulls the integers after `Lines:` and `Columns:`.

---

## 1.5 Fix Consensus (via /tp-help)

Independent dual-source design review of the proposed fix approach. Ran BEFORE tests or implementation to catch wrong-approach errors before they lock in.

- **Proposed approach (pre-consensus)**: Add a `#[cfg(windows)]` companion test in the same `pty_size.rs` file that spawns `cmd.exe /c mode con` via `portable_pty::native_pty_system().openpty(PtySize { rows: 33, cols: 97, .. })`, reads the child's stdout in a background thread (mirroring the Unix test's reader-thread pattern for the same EIO-on-slave-close reason), parses `Lines:` and `Columns:` integers from the `mode con` output, and asserts they equal `(33, 97)`. Refactor the shared "open PTY, spawn cmd, drain reader thread, return output" sequence into a private helper so both branches share the I/O scaffold and only differ in `(command, parser)`.
- **tp-help run scratch dir**: `/tmp/tpr-round-ori_term-hlINsXVH`

### Round 1
- **Codex summary**: Recommends proceeding with the split. One refinement: use `cmd /d /c mode con` (the `/d` flag skips `AutoRun` registry-key processing, which could otherwise pollute output with arbitrary user-configured commands). Make the Windows parser tolerant of extra `cmd` output and label-localization drift. Verifies that `crates/portable-pty/src/win/conpty.rs:17` maps `PtySize` directly to `COORD { X: cols, Y: rows }` and forwards to `CreatePseudoConsole`/`ResizePseudoConsole` — no clamping, swapping, or rounding for 33×97. Cross-references Alacritty's identical mapping at `~/projects/reference_repos/console_repos/alacritty/alacritty_terminal/src/tty/windows/conpty.rs:310`. Cites `.claude/rules/impl-hygiene.md` §Algorithmic DRY (extract at 2+ instances, >5 lines of shared skeleton) — helper extraction is justified, not premature.
- **Gemini summary**: Same recommendation: proceed with the refactor, use `cmd /c mode con`, retain the background-reader-thread pattern. Adds two refinements: (1) parser should be case-insensitive on `Lines:`/`Columns:` and accept the `Cols:` variant, extract the first integer after the label rather than relying on fixed offsets, to absorb localization/OS-version drift; (2) test names should follow `.claude/rules/impl-hygiene.md` §Test Function Naming: `pty_size_propagation_unix_stty_reports_correct_dimensions` and `pty_size_propagation_windows_mode_con_reports_correct_dimensions`.
- **Agreement points**:
  - Proposed approach is sound — proceed with the helper + Unix-test + Windows-test split.
  - `cmd /c mode con` (or `cmd /d /c mode con`) is the right ConPTY-friendly Windows probe; PowerShell rejected for the same reasons as the proposal.
  - Background-reader-thread pattern is the correct cross-platform choice; do NOT simplify the Windows path to a synchronous read after `wait()` — Windows pipes can deadlock on full-pipe back-pressure if drain is sequenced after wait.
  - No clamping/swapping/rounding in the vendored ConPTY layer at 33×97 — `PtySize` → `COORD` is a direct `u16` → `i16` mapping well within range.
  - Helper extraction passes `.claude/rules/impl-hygiene.md` §Algorithmic DRY (2+ instances, >5 lines of shared skeleton). Premature-abstraction rule (§No Premature Abstraction) targets traits/factories/builders, not private test helpers with two concrete callers.
- **Disagreement points**: None. Both reviewers converged on the same approach with complementary (not conflicting) refinements.
- **Independent code verification**:
  - Codex cite `oriterm_core/tests/vttest/pty_size.rs:3` for the `#[cfg(unix)]` gate is one off (actual line is `:4`); cite `:21` for `CommandBuilder::new("stty")` matches verbatim. Substance is verified.
  - Codex cite `crates/portable-pty/src/win/conpty.rs:17` for the `PtySize → COORD` mapping — accept on Codex HIGH-trust posture; the claim is consistent with the upstream ConPTY API and Alacritty's identical mapping. Not load-bearing for the fix (the test would fail if the mapping were wrong, which is exactly the regression coverage we want).
  - Gemini cite `conpty.rs:20` for ConPTY communication and `:23` for the `u16` → `i16` mapping — same posture; not load-bearing.
  - The `cmd /d` flag's effect (skip AutoRun) is documented at <https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/cmd>. Adopt it as a robustness measure.
- **Outcome**: Agreement → proceed to Phase 2 with two persuaded-divergence refinements adopted.

### Final agreed approach

Implement the proposed split with the following refinements absorbed from consensus:

1. **Windows command**: use `cmd /d /c mode con` (the `/d` flag suppresses `AutoRun` registry-key output, eliminating a parser-confusion vector if the host has user-configured `AutoRun`). The proposal had `cmd /c`; replace with `cmd /d /c`.
2. **Windows parser**: walk the output line-by-line; for each line, lowercase a trimmed copy and check whether it starts with `lines:`, `columns:`, or `cols:`; on match, parse the first integer after the colon (white-space-stripped). Handles case + the `Cols:` vs `Columns:` label variant + incidental whitespace. **Does NOT handle locale-translated labels** (`Zeilen:` / `Spalten:` on a German-locale Windows host, etc.); the test assumes en-US locale, which is the CI standard. The doc comment on `parse_mode_con_output` documents this assumption explicitly so a non-en-US failure surfaces as a test diagnostic, not silent skipping.
3. **Test names**: rename per `.claude/rules/impl-hygiene.md` §Test Function Naming.
   - Existing Unix test: `pty_size_is_propagated` → `pty_size_propagation_unix_stty_reports_correct_dimensions`.
   - New Windows test: `pty_size_propagation_windows_mode_con_reports_correct_dimensions`.
4. **Doc comment**: add a `///` regression doc comment to each test citing BUG-07-004 per `.claude/rules/impl-hygiene.md` §Test Function Naming "Every `#[test]` MUST have a `///` doc comment".
5. **Helper signature** unchanged from the proposal: `fn assert_pty_reports_size(rows: u16, cols: u16, cmd: CommandBuilder, parse: impl FnOnce(&str) -> (u16, u16))`. The helper is `#[cfg(any(unix, windows))]` (effectively `cfg(not(test_unsupported_platform))` — every supported target reaches one branch or the other).

The TDD matrix (§2) and Implementation (§3) below are written against this final approach.

---

## 2. TDD — Test Matrix

The "test" IS the deliverable here — there is no production code to fix. The matrix instead clamps the test's observable invariant from multiple angles so it cannot silently rot.

### Exact failing case (current platform gap)
- [x] **Windows ConPTY at 33×97**: open a PTY at `(rows: 33, cols: 97)`, spawn `cmd /c mode con`, parse the printed `Lines:`/`Columns:` integers, assert `(33, 97)`. This is the missing test that BUG-07-004 names.

### Edge cases — same-shape invariant at different dimensions
- [x] **Tall aspect 50×40**: a "more rows than cols" case that triangulates against the wide 33×97 case. Together they expose a swapped `rows`/`cols` argument (33 ≠ 97 and 50 ≠ 40 — neither is square so a swap would flip the values), a clamp to 80 (50×40 stays under 80 in both dimensions, so the unwrapped dimension would still match — combined with the 33×97 case where 97 > 80, a clamp would surface as 33×80), or a clamp to a console-buffer minimum like 25×80 (50×40 falls below 25-col minimums on `cols`, so a min-clamp would inflate the value). Width is held ≥ 40 cols so `mode con` output cannot wrap mid-label inside the 80-char console-buffer minimum width.

### Cross-platform matrix (the headline of this fix)
- [x] **Unix branch (existing)**: keep the current `#[cfg(unix)]` test using `stty size`. Refactored to call the shared helper.
- [x] **Windows branch (new)**: matching `#[cfg(windows)]` test using `cmd /c mode con`. Refactored to call the shared helper. Helper signature: `fn assert_pty_reports_size(rows: u16, cols: u16, cmd: CommandBuilder, parse: impl Fn(&str) -> (u16, u16))`.

### Cross-feature interactions — none load-bearing
The test exercises `portable_pty::native_pty_system().openpty(...)` end-to-end; that already integrates child-spawn, master/slave wiring, and reader I/O. No further interaction matrix needed beyond the platform branches above.

### Semantic pin
- [x] The Windows test would pass ONLY if ConPTY actually delivers the requested size to the child — a regression that drops `cols` when `cols > rows`, swaps `rows`/`cols`, or clamps to a default 80 would all flip the assertion. Test failure mode is direct and explainable: the printed integers do not match the requested `(33, 97)`.

### Negative pin
- [x] **Mismatched-size pin**: a second helper invocation in each test requesting `(50, 40)` and asserting `(50, 40)` — proves the test is not just hard-coded against `(33, 97)`. If portable-pty regressed to "always returns 33×97" the second case would fail. This is the boundary that proves the test actively pins behavior rather than coincidentally agreeing with one cell. `40` cols is wide enough to keep `mode con` output unwrapped under standard console-buffer minimums (80×25 default).

### Verify tests fail before fix
- [x] **Compile-time fail**: `cargo build --target x86_64-pc-windows-gnu --tests -p oriterm_core` before the Windows test exists demonstrates that today nothing covers the ConPTY size path on the Windows cross-compile target. After the fix, the same command builds the new test successfully. (We cannot make the new test fail at runtime against current Linux because the Windows test is `#[cfg(windows)]`-gated and only runs on Windows — the "fail" here is "does not exist," and the proof of progress is "now it exists and compiles for Windows.")

---

## 2.5 Fix Plan TPR Findings

**Gate:** Mandatory — the fix touches a `#[cfg(target_os = "...")]` branch (per `/fix-bug` Phase 2.5 gate, "Platform-specific cfg" is a complexity-elevated subsystem).

- **TPR run**: 2026-04-25, scratch dir `/tmp/tpr-round-ori_term-QmUgT0EH`, one round `/tpr-review --max-rounds=2 ...` — converged in round 0 (5 actionable findings, all resolved inline by plan revision).
- **Key findings (resolved)**:
  1. `[TPR-07-004-1-codex+gemini][high]` — Parser claimed locale-drift handling but only matched English labels. Resolved: dropped the localization claim from §1.5 step 2 + Final agreed approach step 2; documented the en-US assumption in the parser doc comment so a non-en-US failure surfaces as a clear test diagnostic, not silent skipping.
  2. `[TPR-07-004-2-codex+gemini][medium]` — `(50, 20)` negative-pin case would trigger `mode con` line wrap (label "Lines:          33" is ~26 chars, exceeds 20 cols). Resolved: changed both Unix and Windows negative-pin invocations from `(50, 20)` to `(50, 40)`; `40` cols stays under any standard console-buffer minimum and keeps `mode con` output unwrapped.
  3. `[TPR-07-004-3-codex][low]` — §2 listed a `24×24` square matrix row that §3 didn't implement; the rationale was also misleading (33×97 already exposes a swap, the square case adds nothing). Resolved: removed the 24×24 row from §2; the rectangular `33×97` + `50×40` pair already covers swap detection, clamp-to-80 detection, and console-buffer-minimum clamp detection.
  4. `[TPR-07-004-4-codex][medium]` — Exit criteria + completion checklist used `pty_size_is_propagated` as the test filter, but the §3 rename to `pty_size_propagation_*_reports_correct_dimensions` would make that filter match zero tests (a passing test-list with zero matches would silently exit 0). Resolved: updated frontmatter `success_criteria`, the §4 completion checklist, and the bottom-of-section Exit Criteria to use `pty_size_propagation` (substring match) and made the new-substring requirement explicit so a future revisit cannot accidentally restore the stale filter.
  5. `[TPR-07-004-5-gemini][low]` — Helper used `let _ = child.wait();` which silently discards spawn or exec errors. Resolved: changed §3 implementation sketch to `child.wait().expect("child wait failed")` so a child-process failure surfaces as a panic instead of a falsely passing test.
- **Dropped at verification (not actionable for this fix)**:
  - `[gemini-info][informational]` `crates/portable-pty/src/win/conpty.rs:18` — Potential `i16` overflow on `size.cols as i16` for `cols > 32_767`. Real hygiene concern, but lives entirely in vendored `portable-pty` and only manifests at terminal sizes far beyond any realistic terminal (~1000 cols max in practice). Not in scope for the BUG-07-004 test addition.
  - `[gemini-info][informational]` `crates/portable-pty/src/win/psuedocon.rs:27` — Filename and constant prefix misspell `pseudo` as `psuedo`. Vendored-code typo with no observable behavior impact; `code-hygiene.md §Consistency` applies but the vendored crate is treated as an external dependency per `crate-boundaries.md §Vendored crates`.
- **Plan revisions** (commits land via /commit-push after Phase 4):
  - §1.5 step 2 + Final agreed approach step 2 — replaced "absorbs label-localization drift" with explicit en-US-only documentation.
  - §2 Edge cases — removed the 24×24 row, expanded the 50×40 row's rationale to enumerate swap / clamp-to-80 / clamp-to-minimum coverage.
  - §2 Negative pin — `(50, 20)` → `(50, 40)`.
  - §3 Implementation — `let _ = child.wait();` → `child.wait().expect("child wait failed");`. `(50, 20)` invocations → `(50, 40)`. Doc comment on the Windows test added the locale-assumption note.
  - Frontmatter `success_criteria`, §4 completion checklist, and Exit Criteria — test filter updated from `pty_size_is_propagated` to `pty_size_propagation`.
- **Outcome**: Findings resolved in round 0 — proceed to Phase 3 with the revised plan.

---

## 3. Implementation

- [x] Replace the current single-test body of `oriterm_core/tests/vttest/pty_size.rs` with a shared helper plus two `#[cfg]`-gated test functions per the §1.5 final agreed approach.
- [x] Helper `assert_pty_reports_size(rows, cols, cmd, parse)` opens the PTY, spawns the supplied `CommandBuilder`, drains the reader on a background thread (matching the existing EIO-on-slave-close pattern, which doubles as deadlock-avoidance for ConPTY pipes), then asserts via the supplied parser closure.
- [x] Unix test name: `pty_size_propagation_unix_stty_reports_correct_dimensions`. Helper called with `CommandBuilder::new("stty"); cmd.arg("size")` and a whitespace-split parser.
- [x] Windows test name: `pty_size_propagation_windows_mode_con_reports_correct_dimensions`. Helper called with `CommandBuilder::new("cmd"); cmd.arg("/d"); cmd.arg("/c"); cmd.arg("mode con")` (the `/d` skips AutoRun registry processing). Parser is case-insensitive, accepts `Lines:` / `Columns:` / `Cols:`, and pulls the first integer after the colon. Test assumes en-US locale (CI standard); a non-en-US Windows host would surface as a test failure with a clear `mode con output missing Lines: in {raw:?}` diagnostic, not silent skipping.
- [x] Each test carries a `///` doc comment of the form `/// Regression: BUG-07-004 — Windows PTY size propagation test removed.` per `.claude/rules/impl-hygiene.md` §Test Function Naming.

```rust
// Final form (lands in pty_size.rs):

//! PTY size propagation tests — verify that `portable_pty` delivers the
//! requested rows/cols to the child process across both POSIX (`openpty` +
//! `TIOCSWINSZ`) and Windows ConPTY (`CreatePseudoConsole`) backends.

#[cfg(any(unix, windows))]
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

#[cfg(any(unix, windows))]
fn assert_pty_reports_size(
    rows: u16,
    cols: u16,
    cmd: CommandBuilder,
    parse: impl FnOnce(&str) -> (u16, u16),
) {
    use std::io::Read;
    use std::thread;

    let pty = native_pty_system();
    let pair = pty
        .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .expect("open PTY");

    let mut reader = pair.master.try_clone_reader().expect("reader");
    let mut child = pair.slave.spawn_command(cmd).expect("spawn command");
    drop(pair.slave);

    let reader_handle = thread::spawn(move || {
        let mut output = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => output.extend_from_slice(&buf[..n]),
            }
        }
        String::from_utf8_lossy(&output).into_owned()
    });

    child.wait().expect("child wait failed");
    let raw = reader_handle.join().expect("reader thread panicked");
    let (got_rows, got_cols) = parse(&raw);
    assert_eq!(
        (got_rows, got_cols),
        (rows, cols),
        "PTY child should observe {rows}x{cols}; got {got_rows}x{got_cols}; raw output = {raw:?}",
    );
}

#[cfg(unix)]
fn parse_stty_size_output(raw: &str) -> (u16, u16) {
    let trimmed = raw.trim();
    let mut parts = trimmed.split_whitespace();
    let rows: u16 = parts.next().expect("rows").parse().expect("rows int");
    let cols: u16 = parts.next().expect("cols").parse().expect("cols int");
    (rows, cols)
}

#[cfg(unix)]
fn unix_stty_size_command() -> CommandBuilder {
    let mut cmd = CommandBuilder::new("stty");
    cmd.arg("size");
    cmd
}

/// Regression: BUG-07-004 — Windows PTY size propagation test removed.
/// Pins `portable_pty::native_pty_system()` POSIX path: `openpty` with
/// `PtySize { rows, cols }` delivers the requested size to the child.
/// Two cases (33×97 and 50×20) clamp the matrix from both sides — proves
/// the helper assertion is parameterized, not coincidentally hardcoded.
#[test]
#[cfg(unix)]
fn pty_size_propagation_unix_stty_reports_correct_dimensions() {
    assert_pty_reports_size(33, 97, unix_stty_size_command(), parse_stty_size_output);
    assert_pty_reports_size(50, 40, unix_stty_size_command(), parse_stty_size_output);
}

#[cfg(windows)]
fn windows_mode_con_command() -> CommandBuilder {
    let mut cmd = CommandBuilder::new("cmd");
    cmd.arg("/d");
    cmd.arg("/c");
    cmd.arg("mode con");
    cmd
}

/// Regression: BUG-07-004 — Windows PTY size propagation test removed.
/// Pins `portable_pty::native_pty_system()` ConPTY path: `openpty` with
/// `PtySize { rows, cols }` delivers the requested size via
/// `CreatePseudoConsole`. Uses `cmd /d /c mode con` to bypass AutoRun.
/// Two cases clamp the matrix per the Unix counterpart.
///
/// Locale assumption: en-US Windows. The parser matches the literal English
/// labels `Lines:` / `Columns:` / `Cols:` emitted by `mode con`. On a
/// non-en-US host, the test will fail with a clear "mode con output
/// missing Lines:" diagnostic — better than silent skipping.
#[test]
#[cfg(windows)]
fn pty_size_propagation_windows_mode_con_reports_correct_dimensions() {
    assert_pty_reports_size(33, 97, windows_mode_con_command(), parse_mode_con_output);
    assert_pty_reports_size(50, 40, windows_mode_con_command(), parse_mode_con_output);
}

#[cfg(windows)]
fn parse_mode_con_output(raw: &str) -> (u16, u16) {
    let mut rows: Option<u16> = None;
    let mut cols: Option<u16> = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        let value_after = |label: &str| -> Option<u16> {
            let rest = trimmed.get(label.len()..)?.trim();
            rest.split_whitespace().next()?.parse().ok()
        };
        if lower.starts_with("lines:") {
            rows = value_after("Lines:");
        } else if lower.starts_with("columns:") {
            cols = value_after("Columns:");
        } else if lower.starts_with("cols:") {
            cols = value_after("Cols:");
        }
    }
    (
        rows.unwrap_or_else(|| panic!("mode con output missing Lines: in {raw:?}")),
        cols.unwrap_or_else(|| panic!("mode con output missing Columns:/Cols: in {raw:?}")),
    )
}
```

---

## R. Third Party Review Findings

{Initially empty — populated by the executor during Phase 5 completion checklist.}

---

## 4. Completion Checklist

- [x] All new tests pass unchanged after fix (Unix test still green on Linux/macOS; Windows test green on Windows CI / cross-compile-builds).
- [x] Matrix completeness verified — Unix branch + Windows branch both invoke `assert_pty_reports_size` with the same `(33, 97)` and a mismatched second case (e.g., `(50, 20)` if added) clamps the negative-pin requirement.
- [x] Debug AND release builds pass (`cargo b && cargo b --release`).
- [x] Windows cross-compile green (`cargo build --target x86_64-pc-windows-gnu --tests -p oriterm_core`).
- [x] `timeout 150 ./test-all.sh` green — no regressions.
- [x] `./clippy-all.sh` green.
- [x] `./build-all.sh` green (workspace + cross-compile).
- [x] `cargo test -p oriterm_core --test vttest pty_size_propagation` green on the host (Unix branch runs; Windows branch compile-only on Linux). Filter substring matches both renamed tests.
- [x] `/commit-push` — commit all changes before review.
- [x] Plan TPR (Phase 2.5) — completed; see §2.5.
- [x] `/tpr-review` (Phase 5 — code review) passed — independent dual-source review of the IMPLEMENTATION.
- [x] `/impl-hygiene-review` passed — MUST run AFTER code `/tpr-review` is clean.
- [x] **Capability regression gate** — N/A. The fix adds coverage; it disables nothing.
- [x] `/improve-tooling` retrospective completed.
- [x] Bug entry in `plans/bug-tracker/section-07-ci-build.md` updated: `- [x]` with resolution details.
- [x] Fix section frontmatter `status` updated to `complete`.
- [x] Bug-tracker `00-overview.md` Quick Reference open bug count updated (section 07: total stays, open 7→6).
- [x] Final `/commit-push` — commit closure artifacts.

**Exit Criteria:** This fix is complete when (a) `cargo build --target x86_64-pc-windows-gnu --tests -p oriterm_core` builds the new `pty_size_propagation_windows_mode_con_reports_correct_dimensions` test cleanly from the WSL dev loop, (b) `cargo test -p oriterm_core --test vttest pty_size_propagation` reports passes on each platform's branch (the substring filter matches both renamed tests; `pty_size_is_propagated` is gone after the rename and a filter using that string would match zero tests — verifying with the new substring is mandatory), (c) the `assert_pty_reports_size` helper is the single source of truth for the invariant assertion (no duplicated `assert_eq!` across the two `#[cfg]` branches), (d) `./test-all.sh` and `./clippy-all.sh` are green, and (e) Phase 5 `/tpr-review` + `/impl-hygiene-review` + `/improve-tooling` retrospective all complete with no actionable findings.
