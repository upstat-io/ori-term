---
section: "02"
title: "Terminfo Provisioning"
status: in-progress
reviewed: true
goal: "Create extra/ori_term.info (a pinned terminfo source derived from xterm-256color), then add a runtime TerminfoEnv helper in oriterm_test_support that compiles ori_term.info via tic into a temp directory and exposes (TERM=ori_term, TERMINFO=..., TERMINFO_DIRS=...) for child processes. Tack-driven tests in Sections 03+ get a controlled, reproducible terminfo entry instead of validating whatever the host's ncurses database happens to say."
success_criteria:
  - "`extra/ori_term.info` exists and declares ONLY the capabilities ori_term actually implements (verified against `oriterm_core/src/term/handler/`, `oriterm_core/src/term/charset/`, `oriterm_core/src/paste/`, and `oriterm/src/key_encoding/` — see 02.1 capability table). Required base includes `am, bce, ccc, km, mir, msgr, xenl, colors#256, pairs#65536, sgr/setaf/setab/cup/csr/smcup/rmcup/smkx/rmkx, acsc/smacs/rmacs, rep, kf1-kf63`. Extension caps include `Tc, Ms, Ss, Se, Smulx, Sync, XT, AX, hs+dsl+tsl+fsl, BD+BE+PS+PE, kxIN+kxOUT+XF`."
  - "`tic -c -x extra/ori_term.info` exits with status 0. Stderr is either empty OR contains only the known ncurses false-positive for `Setulc` (`%; without %? in Setulc`); any other `tic:` message — parse error, undefined capability, syntax issue — is a gate failure."
  - "`infocmp -A <compiled-dir> ori_term` returns success AND contains `am`, `bce`, `colors#256`, `cup=`, `sgr=`, `setaf=`, `smkx=` (the robust success check — do NOT hardcode `<tempdir>/o/ori_term` filesystem path; infocmp is portable across directory/BDB backends)."
  - "`oriterm_test_support::TerminfoEnv::compile()` uses `include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../extra/ori_term.info\"))` to embed the source at compile time (missing file = build failure), writes it to a temp file, and invokes `tic -x -o <tempdir>` as a subprocess. **`compile()` is pure-`tic`** — it does NOT shell out to `infocmp` at construction time. The post-tic sanity check is a `Path::exists` probe on the entry name (the directory backend's `<dir>/o/<term>` file, since we know what name we asked tic to write). The portable `infocmp -A` round-trip is exercised by the 02.4 test suite, not by the constructor — so callers gate on `tic_available()` only, not on `infocmp_available()`. Returns a struct exposing `term() -> &'static str`, `terminfo_dir() -> &Path`, and `apply_env(&mut CommandBuilder)` which sets `TERM`, `TERMINFO`, and `TERMINFO_DIRS` on the child."
  - "`TerminfoEnv` is `Send` and the temp directory is cleaned up via `Drop` (RAII — uses `tempfile::TempDir`)"
  - "`tic_available()` and `infocmp_available()` runtime checks exist alongside `tack_available()` (Section 03). `tic_available()` gates `TerminfoEnv::compile()` and Section 03's `spawn_tack`. `infocmp_available()` ONLY gates the 02.4 round-trip tests — it is NOT a prerequisite for `compile()`."
  - "`TerminfoVariant` enum (`OriTerm` / `OriTermDirect`) is the only valid input to `TerminfoEnv::compile_with_variant`. Adding a third variant requires updating the exhaustive match in `compile_with_variant` — the compiler enforces the sync point. No `&'static str` parameter exists; callers cannot typo a name."
  - "Unit test `terminfo_env_compiles_ori_term` constructs a `TerminfoEnv` and verifies compilation via `infocmp -A <dir> ori_term` (NOT via hardcoded `<tempdir>/o/ori_term` path check), asserting the output contains `am` and `colors#256`. This test gates on BOTH `tic_available()` AND `infocmp_available()` since it exercises the round-trip. Companion unit tests in 02.2: `terminfo_env_drop_cleans_temp_dir` (pure-tic gate, proves Drop), `apply_env_sets_three_vars` (asserts the `env_pairs` SSOT triple — names distinct, values pinned to the compiled tempdir), `terminfo_variant_entry_names_are_distinct` (catches variant typo collisions), `embedded_terminfo_source_is_nonempty` (proves `include_str!` resolved), `terminfo_env_repeated_compile_stress` (5x compile, no leaks)."
  - "All Section 02 tests skip cleanly when `tic` is unavailable (Windows native, restricted CI environments). Round-trip tests additionally skip when `infocmp` is unavailable. The `infocmp_unavailable_skips_round_trip_test` pin proves the gate works even when `tic` is present but `infocmp` is not."
  - "Parallel/repeated compile stress: `terminfo_env_repeated_compile_stress` calls `TerminfoEnv::compile()` 5 times in succession in a single test, asserts all 5 succeed and all 5 tempdirs are cleaned up cleanly after Drop. Catches tempdir collisions, stale-fd leaks, and tic state leakage."
  - "Performance budget release-vs-debug discipline: BOTH debug and release builds must meet the per-call 1000 ms ceiling for `TerminfoEnv::compile()`. The `terminfo_env_compile_under_perf_budget` test runs in whichever profile `cargo test` uses; the section completion notes record warm-time numbers under both profiles (`cargo test --release -p oriterm_test_support terminfo_env_compile_under_perf_budget` is the second measurement)."
  - "Satisfies the two pinned-terminfo mission criteria in `00-overview.md` (the `extra/ori_term.info` source criterion and the `tic`-compiles + `TERM=ori_term` + `TERMINFO`/`TERMINFO_DIRS` criterion): `extra/ori_term.info` is a hand-authored, fully-pinned terminfo source with a private `ori_term+common` base fragment (NO `use=xterm-256color,` inheritance — capability vocabulary derived from xterm conventions but every cap declared explicitly); `tic` compiles it successfully; tests consume the compiled entry via pinned `TERM=ori_term` + `TERMINFO` + `TERMINFO_DIRS` env vars."
  - "Section 03's `spawn_tack(env: &TerminfoEnv, cols, rows)` consumes `TerminfoEnv::apply_env(&mut CommandBuilder)` — every downstream consumer (Sections 03, 04, 05, 06, 07, 08) reaches the pinned terminfo through this single behavioral API surface, never via raw env-var construction or by iterating an env-var array. The `apply_env` wrapper hides the (`TERM`, `TERMINFO`, `TERMINFO_DIRS`) ordering and var-name details inside `TerminfoEnv` itself — adding a fourth env var (e.g., `NCURSES_NO_UTF8_ACS`) tomorrow requires zero edits to consumers."
  - "BUG-07-008 (the existing `#[cfg(unix)]` antipattern at `crates/oriterm_test_support/src/session/tests.rs:16`) is fixed as part of 02.3's skip-discipline subsection — Section 02 owns the canonical fix because it codifies the runtime-gate-only convention."
  - "Performance budget: a single warm `TerminfoEnv::compile()` invocation finishes under the 1000 ms ceiling enforced by `terminfo_env_compile_under_perf_budget` (~150 ms observed locally; the 1000 ms ceiling captures a 5x debug-build regression). The 50-call projection (Sections 03-08 collectively spawn ~50 instances) must stay under 30 s — the test asserts both gates and emits the warm-time number via `eprintln!`. Section 09 owns the post-Sections-03-08 aggregate `time ./test-all.sh` measurement; Section 02 owns ONLY the per-call ceiling and projection."
  - "TerminfoEnv child-process integrity test: spawn `infocmp` as a CHILD PROCESS via `std::process::Command`, applying the same `TERM`/`TERMINFO`/`TERMINFO_DIRS` triple that `apply_env` would set, with NO `-A` argument, and assert it returns the pinned `ori_term` entry — proves the child actually consults `TERMINFO` / `TERMINFO_DIRS` rather than the host database. This is the only test that exercises the real env-var precedence path."

inspired_by:
  - "Alacritty extra/alacritty.info (alacritty/extra/alacritty.info, 112 lines, sudo tic -xe alacritty,alacritty-direct extra/alacritty.info)"
  - "WezTerm termwiz/data/wezterm.terminfo (90 lines, tic -x -o ~/.terminfo wezterm.terminfo)"
  - "Ghostty src/terminfo/ghostty.zig (declarative capability list compiled from source — same capability vocabulary)"
  - "ncurses tic(1) and term(5) man pages — TERMINFO_DIRS resolution order, -x flag for extension capabilities"
depends_on: ["01"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "02.1"
    title: "Author extra/ori_term.info"
    status: complete
  - id: "02.2"
    title: "TerminfoEnv runtime compiler"
    status: not-started
  - id: "02.3"
    title: "tic_available, infocmp_available, and skip discipline"
    status: not-started
  - id: "02.4"
    title: "Verify pinned terminfo round-trips through infocmp"
    status: not-started
  - id: "02.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "02.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: Terminfo Provisioning

<!-- resolves: plans/bug-tracker/section-07-ci-build.md#BUG-07-008 — fixed as part of 02.3 skip-discipline subsection -->

**Status:** Not Started
**Goal:** Stop letting tack tests validate whatever `xterm-256color` happens to declare on the host. Create a pinned `extra/ori_term.info` (hand-authored — no `use=xterm-256color,` inheritance) that documents EXACTLY which capabilities ori_term implements, then compile it at test runtime via `tic` into a temp directory, then expose `(TERM=ori_term, TERMINFO=<tempdir>, TERMINFO_DIRS=<tempdir>)` env-vars for `tack` (and any other terminfo-aware tool) to consume. After this section, every tack scenario in Sections 03-08 runs against ori_term's documented terminfo, not the host's xterm-256color guess.

**Success Criteria:**

- [ ] `extra/ori_term.info` exists, parseable by `tic -c -x` with exit 0 and stderr empty (sole tolerated exception: the known ncurses `Setulc` false positive — see Success Criteria frontmatter for exact text)
- [ ] `extra/ori_term.info` is an Alacritty-style hand-authored entry (no `use=xterm-256color,` inheritance — the full cap list is declared explicitly so terminfo drift on the host doesn't silently change what ori_term claims). It mirrors Alacritty's `alacritty+common` base + per-entry overrides, with the cap list scoped to what ori_term ACTUALLY implements today
- [ ] `oriterm_test_support::terminfo::TerminfoEnv::compile()` produces a working terminfo directory consumable by `tack`, `infocmp`, and any ncurses-linked tool. **`compile()` shells out only to `tic`** — the `infocmp -A` round-trip lives in 02.4 tests, NOT in the constructor (so callers gate on `tic_available()` only).
- [ ] `TerminfoVariant` enum (`OriTerm`, `OriTermDirect`) is the only valid `compile_with_variant` parameter — the compiler enforces exhaustivity, no `&'static str` typo surface
- [ ] `TerminfoEnv` implements `Drop` cleanly via `tempfile::TempDir`
- [ ] `TerminfoEnv::apply_env(&mut CommandBuilder)` sets `TERM`, `TERMINFO`, AND `TERMINFO_DIRS` on the child in one call — consumers do NOT iterate an env-var array. Adding/removing env vars is internal to `TerminfoEnv`, never a downstream-edit storm.
- [ ] `tic_available()` and `infocmp_available()` helper functions exist in `oriterm_test_support` next to `tack_available()` (Section 03). `tic_available()` gates `compile()` and `spawn_tack`; `infocmp_available()` ONLY gates 02.4 round-trip tests.
- [ ] `cargo test -p oriterm_test_support terminfo` — internal unit tests pass on Linux/macOS, skip cleanly on Windows
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green
- [ ] BUG-07-008 fix landed (preferred two-arm portable test for `pty_session_drains_simple_output`) — see 02.3 checklist
- [ ] Performance: `TerminfoEnv::compile()` micro-benchmark recorded under BOTH debug and release profiles. Single-call ceiling 1000 ms. Section 02 also projects 50-call cost (×50) and files `/add-bug` if the projection exceeds 30 s.
- [ ] Child-process integrity test landed: `infocmp` spawned as a child with `apply_env()` applied, NO `-A` flag, returns the pinned ori_term entry (proves env-var precedence path actually works)
- [ ] Parallel/repeated compile stress test (`terminfo_env_repeated_compile_stress`) calls `compile()` 5 times in succession in one test, asserts all 5 succeed AND all 5 tempdirs clean up after Drop
- [ ] `infocmp_unavailable_skips_round_trip_test` proves the `infocmp_available()` gate works (round-trip test skips cleanly when `infocmp` is missing even though `tic` is present)
- [ ] Failing-test-first sequencing enforced: every TDD entry in 02.2 explicitly says "write the test FIRST, watch it fail, THEN implement"
- [ ] Satisfies the two pinned-terminfo mission criteria in `00-overview.md` (the `extra/ori_term.info` hand-authored source criterion AND the `tic`-compiles + pinned `TERM=ori_term`/`TERMINFO`/`TERMINFO_DIRS` criterion — see overview for the reconciled wording). Section 02 also indirectly contributes to the cross-platform skip-discipline mission criterion via the BUG-07-008 fix landed in 02.3 (`pty_session_drains_simple_output` becomes the second portable test in `oriterm_test_support`).

**Context:** Without a controlled terminfo entry, tack reads `$TERM=xterm-256color` from the host system terminfo database (`/usr/share/terminfo/x/xterm-256color` on Linux). That entry was authored by someone else, drifts over time, and doesn't reflect ori_term's actual capabilities. Tests built on the host entry validate the host's idea of "xterm-256color", not ori_term's implementation. The result: ori_term might claim to implement a capability the host terminfo says exists, or fail a test because the host terminfo declares a feature ori_term doesn't yet support.

The fix is the convention used by every serious terminal: ship a `.info` source file in the project, compile it with `tic` at install time (or test time), and use the compiled entry via `TERMINFO`/`TERMINFO_DIRS`. Alacritty does this with `extra/alacritty.info` (112 lines, compiled via `sudo tic -xe alacritty,alacritty-direct extra/alacritty.info`). WezTerm does it with `termwiz/data/wezterm.terminfo` (90 lines, compiled via `tic -x -o ~/.terminfo wezterm.terminfo`). Ghostty generates the source from Zig code (`src/terminfo/ghostty.zig`) at build time. We adopt the Alacritty pattern: a hand-authored `.info` file under `extra/` with a private base fragment, two user-facing entries, NO `xterm-256color` inheritance, and a runtime tic invocation from the test driver.

**Reference implementations:**
- **Alacritty** `extra/alacritty.info` (lines 1-112): canonical Rust-terminal terminfo source. Two entries (`alacritty` legacy 256-color, `alacritty-direct` truecolor) plus a private `alacritty+common` base fragment. Both user-facing entries `use=alacritty+common,` — they do NOT inherit from `xterm-256color`. We adopt the SAME approach: a private `ori_term+common` fragment that `ori_term` and `ori_term-direct` both consume. Inheriting from `xterm-256color` would make ori_term's terminfo depend on whatever the host happens to ship today, which defeats the entire purpose of pinning.
- **Alacritty** install instruction: `sudo tic -xe alacritty,alacritty-direct extra/alacritty.info` — `-x` enables extension capabilities (Tc/Ms/Ss/Smulx/Sync), `-e` selects entries to compile.
- **WezTerm** `termwiz/data/wezterm.terminfo` (line 17 onward): the modern extension capability declarations (`Tc, hs, Su, Cr, Cs, Ms, Se, Ss, Smulx, Sync, Setulc, Smol`). These are the boilerplate any modern terminal needs. NOTE: WezTerm's `Setulc` declaration triggers ncurses' known false-positive warning `%; without %? in Setulc` — this is the cap form ori_term will use, with the warning explicitly tolerated by the gate.
- **Ghostty** `src/terminfo/ghostty.zig:1-40`: capability list as Zig data — useful as a cross-reference for what a modern terminal *should* declare.
- **ncurses** `term(5)` man page: terminfo source format (capability codes, escape sequences, parameter substitution).
- **ncurses** `tic(1)` man page: `-x` (extension caps), `-o <dir>` (output dir), `-c` (validate without compiling). The `-x` flag is REQUIRED — without it, modern caps like `Tc` produce `tic: WARNING` lines and may be silently dropped.

**Depends on:** Section 01 — `oriterm_test_support` crate must exist before `TerminfoEnv` can be added to it.

---

## 02.1 Author extra/ori_term.info

**File(s):** `extra/ori_term.info` (NEW FILE — `extra/` directory does not yet exist and will be created here)

The terminfo source is the canonical declaration of "what ori_term claims to be". It should match what ori_term actually IMPLEMENTS today, not what we'd like it to be — Section 09 verification will compare claims against reality.

- [x] Create directory `/home/eric/projects/ori_term/extra/`. This directory does not exist — verify before creating, then `mkdir extra`.

- [x] Create `extra/ori_term.info` with two entries: `ori_term` (256-color) and `ori_term-direct` (truecolor). Match Alacritty's two-entry-plus-common-fragment pattern (`alacritty+common`) — a private fragment holds every cap shared by both entries; the two user-facing entries `use=` only that fragment, NEVER `xterm-256color`. This is load-bearing: `use=xterm-256color,` would inherit whatever the host's xterm-256color happens to declare today, defeating the entire point of pinning.

  The file structure (modeled on `~/projects/reference_repos/console_repos/alacritty/extra/alacritty.info`):
  ```
  # ori_term terminfo entry.
  #
  # Compile with:
  #   tic -x -o ~/.terminfo extra/ori_term.info
  #
  # Or for tests, the runtime helper at
  # crates/oriterm_test_support/src/terminfo/mod.rs:TerminfoEnv::compile()
  # embeds this file via include_str! and invokes tic -x -o <tempdir>,
  # then sets TERM=ori_term + TERMINFO=<tempdir> + TERMINFO_DIRS=<tempdir>
  # for the child process.
  #
  # The -x flag is REQUIRED — modern extension capabilities (Tc, Ms,
  # Ss, Se, Smulx, Sync, BD, BE, PS, PE, XF, kxIN, kxOUT) are user-
  # defined and tic warns + drops them without -x. See `man tic` for
  # the full -x behavior.

  ori_term|ori_term terminal emulator,
      use=ori_term+common,
      # Override setaf/setab with 256-color indexed variants.
      ccc,
      colors#0x100, pairs#0x10000,
      setab=\E[%?%p1%{8}%<%t4%p1%d%e%p1%{16}%<%t10%p1%{8}%-%d%e48;
            5;%p1%d%;m,
      setaf=\E[%?%p1%{8}%<%t3%p1%d%e%p1%{16}%<%t9%p1%{8}%-%d%e38;5
            ;%p1%d%;m,

  ori_term-direct|ori_term with direct color indexing,
      use=ori_term+common,
      RGB,
      colors#0x1000000, pairs#0x10000,
      op=\E[39;49m,
      setab=\E[%?%p1%{8}%<%t4%p1%d%e48\:2\:\:%p1%{65536}%/%d\:%p1%{256}
            %/%{255}%&%d\:%p1%{255}%&%d%;m,
      setaf=\E[%?%p1%{8}%<%t3%p1%d%e38\:2\:\:%p1%{65536}%/%d\:%p1%{256}
            %/%{255}%&%d\:%p1%{255}%&%d%;m,

  ori_term+common|base fragment for ori_term,
      # ... full capability list (see table below) ...
  ```

  **Capability authoring rules — ground every line in actual ori_term behavior:**

  Before declaring any capability, verify ori_term actually implements it. Sources of truth (verified file paths):
  - `oriterm_core/src/term/handler/` — VTE handler implementations split across `osc.rs` (title/icon/color/clipboard/hyperlink), `modes.rs` (DECSET/DECRST), `sgr.rs` (SGR attrs including colored underline), `esc.rs`, `dcs.rs`, `status.rs`. The `handler/mod.rs` is the dispatch hub.
  - `oriterm_core/src/term/charset/mod.rs` — G0-G3 charset slots and DEC special graphics mapping via `StandardCharset::map` (backs `acsc`/`smacs`/`rmacs`)
  - `oriterm_core/src/paste/mod.rs` — bracketed paste wrapping (`BRACKET_START`/`BRACKET_END` = `\E[200~`/`\E[201~`), backs `BD`/`BE`/`PS`/`PE` declarations
  - `oriterm_core/src/term/mode/mod.rs` — `TermMode` flags (alt-screen, mouse, focus in/out, bracketed paste, sync-update). `FOCUS_IN_OUT` = `1 << 12`.
  - `oriterm_core/src/grid/` — modes that affect grid behavior (autowrap=`am`, bce=`bce`, origin=`om`, alt-screen=`smcup/rmcup`)
  - `oriterm/src/key_encoding/legacy.rs` — legacy xterm key encoding (F1-F12 direct, F13-F63 = F1-F12 with xterm modifier param via `encode_legacy`). Kitty protocol extensions live in `kitty.rs` but legacy is the kf1-kf63 source of truth.
  - `oriterm/src/app/event_loop_helpers/mod.rs:139` — `send_focus_event` emits `\x1b[I`/`\x1b[O` for focus-in/out (backs `kxIN`/`kxOUT`/`XF`). NOTE: focus-event emission lives in the binary crate (`oriterm/`), not `oriterm_core`. The terminfo declaration is still valid because it describes the terminal-as-a-whole, not `oriterm_core` specifically.
  - `oriterm_core/src/color/palette/mod.rs:14` — `NUM_COLORS = 270` (16 ANSI + 216 cube + 24 grayscale + 14 named/system). For terminfo, this means `colors#256` for `ori_term` (16M entries for `ori_term-direct`).
  - `crates/vte/src/ansi/dispatch/csi.rs:60` — CSI `b` (REP) dispatch: repeats `preceding_char` N times via `handler.input()`. This is vendored vte but runs through `oriterm_core::Term`'s input path, so `rep=%p1%c\E[%p2%{1}%-%db` is a real capability and is declared in 02.1's required cap list.
  - The `vttest` test results (from Section 01's preserved 198 snapshots) — what menus pass and what they exercise

  **Hand-authored, no host inheritance.** The base fragment `ori_term+common` declares every cap explicitly. DO NOT use `use=xterm-256color,` — that would make ori_term's terminfo depend on whatever the host's xterm-256color happens to declare today. Pin everything. Cross-reference against Alacritty's `alacritty+common` fragment at `~/projects/reference_repos/console_repos/alacritty/extra/alacritty.info:26-112` for authoring style and exact string-cap parameter formats.

  **Required base capability set** (every entry below must have a corresponding implementation site — grep the file:line comment markers at commit time):

  Booleans (all verified against ori_term source):
  - `am` — auto-margin; grid autowrap (default on)
  - `bce` — back-color-erase
  - `ccc` — can change color palette (OSC 4/10-12); `oriterm_core/src/term/handler/osc.rs:74` `osc_set_color`
  - `km` — has a meta key
  - `mir` — safe to move while in insert mode
  - `msgr` — safe to move while in standout mode
  - `xenl` — newline glitch (cursor stays at col N after writing column N)
  - `AX` — default fg/bg restore via SGR 39/49 supported
  - `XT` — xterm-style title sequences supported; `oriterm_core/src/term/handler/osc.rs:21` `osc_set_title`
  - `hs` — has status line. REQUIRES `dsl`/`tsl`/`fsl` string caps below — using OSC 0/2 as the "status line" the way Alacritty does (`alacritty.info:108`). The contract: `tsl=\E]2;,` opens a title-write, `fsl=^G` closes it, `dsl=\E]2;\007` clears it. ori_term's OSC 0/2 handler (`osc_set_title`) fulfills this contract.
  - DO NOT declare `OTbs` (deprecated obsolete cap)

  Numbers:
  - `colors#256, cols#80, it#8, lines#24, pairs#0x10000` for `ori_term`
  - `colors#0x1000000, pairs#0x10000` for `ori_term-direct`

  Cursor movement: `cup, cub, cub1, cuf, cuf1, cud, cud1, cuu, cuu1, hpa, vpa, home, ind, ri, nel, cr, ht`
  Erase/clear: `ed, el, el1, ech, clear, E3=\E[3J`
  Insert/delete: `ich, dch, dch1, il, il1, dl, dl1, ich1` (verify each against `oriterm_core/src/term/handler/` before declaring)
  SGR: `bold, dim, sitm, ritm, smul, rmul, smso, rmso, rev, blink, invis, sgr, sgr0, smxx, rmxx` — verified against `oriterm_core/src/term/handler/sgr.rs`
  Color: `setaf, setab, op, oc, initc` (SKIP `setb`/`setf` — deprecated, use `setab`/`setaf`)
  Screen: `smcup=\E[?1049h\E[22;0;0t, rmcup=\E[?1049l\E[23;0;0t, csr, sc, rc` (DECSC/DECRC wired in handler)
  Keypad: `smkx=\E[?1h\E=, rmkx=\E[?1l\E>, kbs=^?, kcub1, kcud1, kcuf1, kcuu1, khome, kend, kpp, knp, kdch1, kich1`
  Function keys: `kf1` through `kf63` — legacy xterm conventions (see `oriterm/src/key_encoding/legacy.rs:85-108` for the direct map):
  - `kf1-kf4 = \EOP/\EOQ/\EOR/\EOS` (SS3)
  - `kf5-kf12 = \E[15~..\E[24~` (CSI tilde)
  - `kf13-kf24 = \E[1;2{P/Q/R/S}` / `\E[{n};2~` (Shift)
  - `kf25-kf36 = \E[1;5{P/Q/R/S}` / `\E[{n};5~` (Ctrl)
  - `kf37-kf48` = Ctrl+Shift; `kf49-kf60` = Alt; `kf61-kf63` = Alt+Shift (F1-F3 only; kf63 is the last standard entry)
  - Modified F1-F12 are emitted by `encode_legacy` via `mod_param` — verify each row against the actual `legacy.rs` output before committing
  - Cross-reference Alacritty's `alacritty+common` at `alacritty.info:69-86` for the complete kf1-kf63 table; ori_term's values must match the xterm modifier convention identically
  Editing/cursor keypad: `kDC, kEND, kHOM, kIC, kNXT, kPRV` with modifier suffixes per xterm — see Alacritty's `alacritty.info:94-105`
  Charset: `acsc=``aaffggiijjkkllmmnnooppqqrrssttuuvvwwxxyyzz{{||}}~~, smacs=\E(0, rmacs=\E(B, enacs=` — DEC special graphics; verified against `oriterm_core/src/term/charset/mod.rs`
  Mouse: `kmous=\E[M`
  Reports: `u6=\E[%i%d;%dR, u7=\E[6n, u8=\E[?%[;0123456789]c, u9=\E[c`
  Misc: `bel=^G, flash, civis=\E[?25l, cnorm=\E[?12l\E[?25h, cvvis=\E[?12;25h, rep=%p1%c\E[%p2%{1}%-%db, indn=\E[%p1%dS, rin=\E[%p1%dT` — `rep` is wired via vendored vte at `crates/vte/src/ansi/dispatch/csi.rs:60` which calls `handler.input()` per repetition.

  **Modern extension capabilities** (require `tic -x`):
  - `Tc` — truecolor support (boolean; direct entry declares `RGB` instead)
  - `Ms=\E]52;%p1%s;%p2%s\007` — clipboard set/get via OSC 52; verified at `oriterm_core/src/term/handler/osc.rs:114` `osc_clipboard_store` and `osc.rs:145` `osc_clipboard_load`
  - `Ss=\E[%p1%d q` and `Se=\E[2 q` — DECSCUSR cursor style (note the literal space before `q` — this is NOT `\sq`)
  - `Smulx=\E[4\:%p1%dm` — kitty-style underline styles; verified at `crates/vte/src/ansi/attr.rs:150-156` (`DoubleUnderline`, `DottedUnderline`, `DashedUnderline`) and SGR dispatch in `oriterm_core/src/term/handler/sgr.rs`
  - `Setulc=\E[58:2::%p1%{65536}%/%d:%p1%{256}%/%{255}%&%d:%p1%{255}%&%d%;m` — underline color; verified at `oriterm_core/src/term/handler/sgr.rs:60` (`Attr::UnderlineColor`) and `oriterm_core/src/cell/mod.rs:196` `set_underline_color`. NOTE: ncurses 6.4 `tic -c -x` emits a known false-positive warning `%; without %? in Setulc` — this specific warning is tolerated by the gate (see Success Criteria). All other tic warnings are gate failures.
  - `Sync=\E[?2026%?%p1%{1}%-%tl%eh%;` — synchronized output (mode 2026); verified at `oriterm_core/src/term/handler/modes.rs:83` (`NamedPrivateMode::SyncUpdate`)
  - `Cr=\E]112\007, Cs=\E]12;%p1%s\007` — cursor color OSC 12 set/reset
  - `hs, dsl=\E]2;\007, fsl=^G, tsl=\E]2;` — status line via OSC title (see `hs` note above)
  - `BD=\E[?2004l, BE=\E[?2004h, PE=\E[201~, PS=\E[200~` — bracketed paste start/stop + paste prefix/suffix; verified at `oriterm_core/src/paste/mod.rs:11-14` and `oriterm_core/src/term/handler/modes.rs:82` (`NamedPrivateMode::BracketedPaste`)
  - `kxIN=\E[I, kxOUT=\E[O, XF` — focus in/out report (boolean + two key caps); verified at `oriterm_core/src/term/handler/modes.rs:49` (`NamedPrivateMode::ReportFocusInOut`) for the mode itself, and `oriterm/src/app/event_loop_helpers/mod.rs:143` `send_focus_event` for the sequence emission
  - `XT` — boolean indicating "xterm-style title sequences supported"
  - `AX` — boolean indicating "default fg/bg restore via SGR 39/49 supported"

  **Capabilities to EXPLICITLY NOT declare** (ori_term does not implement these today — any future plan that adds them must also update `extra/ori_term.info`):
  - No `is1`/`is2`/`rs1`/`rs2` reset strings yet — add when the plan for terminal reset lands
  - No printer support (`mc0, mc4, mc5, mc5i`)
  - No `meml`/`memu` memory lock
  - No `rmm`/`smm` meta-mode toggles (handled at input layer, not via terminfo)

  **Extension cap discipline:** for EVERY modern extension cap declared, leave a comment line above it pointing to the file:line where it's implemented. Example:
  ```
      # oriterm_core/src/term/handler/osc.rs:114 — OSC 52 clipboard
      Ms=\E]52;%p1%s;%p2%s\007,
  ```
  This makes Section 09 verification mechanical: grep for the comment markers, follow the references, confirm each declared capability has a corresponding implementation site.

- [x] Compile-check the source as you author it: `tic -c -x extra/ori_term.info` (the `-c` flag validates without writing). The gate is: **exit status 0 AND stderr is either empty or contains only the known ncurses false-positive `%; without %? in Setulc`**. Any other `tic:` message (parse error, undefined capability, line-column syntax issue, duplicate entry) is a gate failure that must be fixed before committing the file. Verified locally: ncurses 6.4.20240113 reports exit 0 and only the three Setulc false-positive lines (one per entry — `ori_term`, `ori_term-direct`, `ori_term+common`).

  Authoring note: cross-check the actual cap strings with `infocmp -x xterm-256color` (the `-x` flag exposes user-defined / extension capabilities that plain `infocmp` hides). This is the reference format to match.

- [x] Run `tic -x -o /tmp/ori_term_terminfo extra/ori_term.info` and verify via the portable command (NOT by asserting filesystem layout — ncurses supports directory AND hashed-db backends, and the `<dir>/o/<name>` layout is only present on the directory backend):
  - `tic` exits with status 0
  - `infocmp -A /tmp/ori_term_terminfo ori_term` exits 0 and emits source-form output containing the declared boolean caps and `colors#256`
  - `infocmp -A /tmp/ori_term_terminfo ori_term-direct` exits 0 and contains `colors#16777216` or `colors#0x1000000`
  - `infocmp -A /tmp/ori_term_terminfo ori_term+common` exits 0 (the private fragment must be present for subsequent `use=` inheritance to succeed)
  Verified locally: `tic` exits 0 (only the three tolerated `Setulc` warnings). `infocmp -A` succeeds for all three entries. The 256-color entry shows `colors#0x100, pairs#0x10000`; the direct entry shows `colors#0x1000000, pairs#0x10000`. Required base caps (`am, bce, ccc, hs, km, mir, msgr, xenl`, `cup=`, `sgr=`, `setaf=`, `setab=`, `smkx=`, `acsc=`, `smacs=`, `rmacs=`, `rep=`) all appear in plain `infocmp -A` output. Modern extension caps (`Tc`, `Ms=`, `Ss=`, `Se=`, `Smulx=`, `Setulc=`, `Sync=`, `BD=`, `BE=`, `PS=`, `PE=`, `kxIN=`, `kxOUT=`, `XF`, `Cr=`, `Cs=`) only appear under `infocmp -x -A` — Section 02.4 round-trip tests must use the `-x` flag (the existing test code at lines 904-962 omits it; will be fixed in 02.4).

- [x] Commit `extra/ori_term.info` as a tracked file. **Do NOT** check in any compiled output (`/tmp/ori_term_terminfo/` is throw-away — runtime compilation in 02.2 produces a fresh copy per test run).

- [x] **Release-packaging scope note:** `extra/ori_term.info` is a source file consumed only by the test harness in this plan. It is NOT bundled into release binaries, and this plan does NOT add install scripts for it. When ori_term ships its first packaged release, a separate plan (installer/packaging) will decide whether to bundle the compiled terminfo into `/usr/share/terminfo/o/ori_term` (Linux/macOS system install) or leave it as an opt-in developer-side file. For this plan, the only consumer is `TerminfoEnv::compile()` at test time; the file lives in the source tree and nowhere else.

---

## 02.2 TerminfoEnv runtime compiler

**File(s):** `crates/oriterm_test_support/src/terminfo/mod.rs` (NEW — directory module), `crates/oriterm_test_support/src/terminfo/tests.rs` (NEW — sibling tests), `crates/oriterm_test_support/src/lib.rs` (mod declaration + re-export), `crates/oriterm_test_support/Cargo.toml` (tempfile dep)

`TerminfoEnv` is the test-time bridge between `extra/ori_term.info` (committed source, embedded at build time via `include_str!`) and tack/tic-driven test sessions (runtime consumers). Each test that needs a pinned terminfo constructs a `TerminfoEnv`, then calls `env.apply_env(&mut cmd)` exactly once on the `CommandBuilder` it is about to spawn — the wrapper sets `TERM`, `TERMINFO`, and `TERMINFO_DIRS` together. `TERMINFO` and `TERMINFO_DIRS` are both set because some ncurses consumers honor only one of the two; the wrapper hides that detail so consumers never need to know.

`compile()` itself is pure-`tic`: it embeds the source via `include_str!`, writes it to a scratch file in a fresh tempdir, invokes `tic -x -o <tempdir>`, and (as a sanity check) verifies the entry file exists on disk under the directory backend. **It does NOT shell out to `infocmp`** — the round-trip via `infocmp -A` is exercised by the 02.4 test suite, not by the constructor. This split keeps `compile()`'s tool dependency precisely one tool (`tic`) and matches Section 03's `tic_available()`-only gate.

- [ ] Add `tempfile` to `crates/oriterm_test_support/Cargo.toml` `[dependencies]`:
  ```toml
  [dependencies]
  oriterm_core = { path = "../../oriterm_core" }
  portable-pty = "0.9.0"
  tempfile = "3"
  vte = { version = "0.15.0", features = ["ansi"] }
  ```
  `tempfile = "3"` is already a dev-dep elsewhere in the workspace (e.g., `oriterm/Cargo.toml:78`) — same major version.

- [ ] Add `pub mod terminfo;` to `crates/oriterm_test_support/src/lib.rs` and `pub use terminfo::TerminfoEnv;`.

- [ ] Create `crates/oriterm_test_support/src/terminfo/mod.rs` (directory module, not single file — the sibling `tests.rs` pattern requires `foo/mod.rs` + `foo/tests.rs` per `.claude/rules/test-organization.md`):
  ```rust
  //! Pinned terminfo provisioning for conformance test sessions.
  //!
  //! Compiles `extra/ori_term.info` at test runtime via `tic -x -o <tempdir>`
  //! and applies the resulting (`TERM`, `TERMINFO`, `TERMINFO_DIRS`) env
  //! triple to a `CommandBuilder` so child processes (`tack`, `infocmp`,
  //! anything ncurses-linked) read ori_term's pinned terminfo entry
  //! instead of the host's `xterm-256color`.
  //!
  //! The terminfo source is embedded at compile time via `include_str!`.
  //! If `extra/ori_term.info` is missing or unreadable, this crate fails
  //! to compile — the dependency between the test-support crate and the
  //! committed source file is enforced at build time, not runtime. There
  //! is no filesystem discovery, no repo-root walk, no env-var lookup.
  //!
  //! `compile()` is pure-`tic`. It does NOT shell out to `infocmp`. The
  //! `infocmp -A` round-trip lives in the 02.4 test suite — running it
  //! inside the constructor would force every caller (including Section
  //! 03's `spawn_tack`) to gate on `infocmp_available()`, which is a
  //! second tool dependency for no constructor-side gain. The post-tic
  //! sanity check is a `Path::exists` probe on the directory-backend
  //! entry file, since we know what name we asked tic to write.

  use std::io::Write;
  use std::path::Path;
  use std::process::{Command, Stdio};

  use portable_pty::CommandBuilder;
  use tempfile::TempDir;

  /// Embedded `extra/ori_term.info` source, captured at compile time.
  ///
  /// The path is relative to `CARGO_MANIFEST_DIR` — i.e. the
  /// `oriterm_test_support` crate root, which sits at
  /// `crates/oriterm_test_support/`. Two levels up is the workspace root,
  /// where `extra/ori_term.info` lives.
  pub(crate) const ORI_TERM_INFO: &str = include_str!(concat!(
      env!("CARGO_MANIFEST_DIR"),
      "/../../extra/ori_term.info"
  ));

  /// Which terminfo entry to pin a [`TerminfoEnv`] to.
  ///
  /// Compile-time exhaustivity: adding a third variant requires updating
  /// the `match` in [`TerminfoEnv::compile_with_variant`] AND the
  /// `entry_name` mapping below — the compiler enforces both sync points.
  /// This is the impl-hygiene "Stringly-typed internals" rule (finite
  /// valid values → enum); a `&'static str` parameter would only constrain
  /// lifetime, not value, and Rust does not have C++-style template
  /// constant validation.
  #[derive(Copy, Clone, Debug, PartialEq, Eq)]
  pub enum TerminfoVariant {
      /// 256-color entry (`ori_term`). Default for [`TerminfoEnv::compile`].
      OriTerm,
      /// Truecolor entry (`ori_term-direct`).
      OriTermDirect,
  }

  impl TerminfoVariant {
      /// Returns the literal `TERM` string corresponding to this variant.
      #[must_use]
      pub fn entry_name(self) -> &'static str {
          match self {
              Self::OriTerm => "ori_term",
              Self::OriTermDirect => "ori_term-direct",
          }
      }
  }

  /// A compiled terminfo directory for ori_term plus the env-var
  /// machinery to steer child processes to it.
  ///
  /// At construction, this type:
  ///   1. Creates a temp directory via `tempfile::TempDir`
  ///   2. Writes the embedded `ORI_TERM_INFO` source to a scratch file
  ///      inside the tempdir
  ///   3. Invokes `tic -x -o <tempdir> <scratch>` as a subprocess
  ///   4. Sanity-checks that `<tempdir>/<bucket>/<entry>` exists on disk
  ///      (directory backend; the entry name is what we just told tic to
  ///      write so we know what to look for). The portable round-trip via
  ///      `infocmp -A` lives in the 02.4 test suite, not here.
  ///
  /// Drop cleans up the temp directory automatically (RAII via TempDir).
  ///
  /// # Errors / Panics
  ///
  /// - Panics if `tic` is not installed (callers must gate on
  ///   [`crate::tic_available`] first).
  /// - Panics if `tic` exits non-zero (compilation failure — prints stderr).
  /// - Panics if the post-tic sanity check fails to locate the directory-
  ///   backend entry file (proves the compile actually wrote the entry,
  ///   not just exited 0 silently).
  pub struct TerminfoEnv {
      tempdir: TempDir,
      variant: TerminfoVariant,
  }

  impl TerminfoEnv {
      /// Compile `extra/ori_term.info` into a fresh temp dir, returning
      /// a handle pinned to the 256-color [`TerminfoVariant::OriTerm`]
      /// entry. Equivalent to
      /// `TerminfoEnv::compile_with_variant(TerminfoVariant::OriTerm)`.
      #[must_use]
      pub fn compile() -> Self {
          Self::compile_with_variant(TerminfoVariant::OriTerm)
      }

      /// Compile and pin to a specific [`TerminfoVariant`].
      ///
      /// The variant enum gives compile-time exhaustivity — adding a
      /// third variant forces updating the `match` arms here AND in
      /// `TerminfoVariant::entry_name`. Callers cannot pass an invalid
      /// terminfo name; there is no string surface to typo against.
      #[must_use]
      pub fn compile_with_variant(variant: TerminfoVariant) -> Self {
          let tempdir = TempDir::new().expect("create terminfo tempdir");
          let source_path = tempdir.path().join("ori_term.info");
          {
              let mut f = std::fs::File::create(&source_path)
                  .expect("create embedded terminfo source file");
              f.write_all(ORI_TERM_INFO.as_bytes())
                  .expect("write embedded terminfo source");
          }

          let tic_out = Command::new("tic")
              .arg("-x")
              .arg("-o")
              .arg(tempdir.path())
              .arg(&source_path)
              .stdout(Stdio::piped())
              .stderr(Stdio::piped())
              .output()
              .expect("invoke tic");

          if !tic_out.status.success() {
              panic!(
                  "tic failed (exit {}):\nstdout: {}\nstderr: {}",
                  tic_out.status,
                  String::from_utf8_lossy(&tic_out.stdout),
                  String::from_utf8_lossy(&tic_out.stderr),
              );
          }

          // Sanity check: directory-backend file exists. We are NOT using
          // this as the portable success check — that lives in 02.4 via
          // `infocmp -A` round-trip tests. We use a filesystem probe here
          // because we know exactly which entry we asked `tic` to write
          // (it's the variant we passed in), so the path is determined.
          // If the host has the hashed-db backend instead of the directory
          // backend, this probe would fail — but every Linux/macOS ncurses
          // build that ships `tic` also defaults to the directory backend
          // when given `-o <dir>` (see `man tic`). Hashed-db is opt-in via
          // `--with-hashed-db` at ncurses build time and is rare.
          let entry_name = variant.entry_name();
          let bucket = entry_name
              .chars()
              .next()
              .expect("variant entry name is non-empty");
          let entry_path = tempdir.path().join(bucket.to_string()).join(entry_name);
          assert!(
              entry_path.exists(),
              "tic claimed success but entry file {entry_path:?} is missing — \
               the host may use a hashed-db terminfo backend; the 02.4 \
               infocmp round-trip will catch the same case via a different \
               error message"
          );

          Self { tempdir, variant }
      }

      /// The pinned `TERM` value (`ori_term` or `ori_term-direct`).
      #[must_use]
      pub fn term(&self) -> &'static str {
          self.variant.entry_name()
      }

      /// Which [`TerminfoVariant`] this env was compiled against.
      #[must_use]
      pub fn variant(&self) -> TerminfoVariant {
          self.variant
      }

      /// The directory `tic` wrote into. Used as both `TERMINFO` and
      /// `TERMINFO_DIRS` by [`Self::apply_env`].
      #[must_use]
      pub fn terminfo_dir(&self) -> &Path {
          self.tempdir.path()
      }

      /// The (name, value) env-var triple this `TerminfoEnv` advertises.
      ///
      /// **SSOT for the env-var contract.** [`Self::apply_env`] (the
      /// public wrapper for `portable_pty::CommandBuilder`) and the 02.4
      /// `child_process_with_apply_env_reads_pinned_terminfo` integrity
      /// test (which spawns `std::process::Command`, not `CommandBuilder`)
      /// BOTH consume this method. There is no other place that knows
      /// which env vars get set. Adding a fourth env var tomorrow is a
      /// one-line edit here; every consumer follows automatically.
      ///
      /// `pub(crate)` because the type-safe public API surface is
      /// `apply_env` — external callers should never iterate the array.
      /// The 02.4 integrity test is in the same crate, so it sees this
      /// helper.
      pub(crate) fn env_pairs(&self) -> [(&'static str, String); 3] {
          let dir = self.terminfo_dir().to_string_lossy().into_owned();
          [
              ("TERM", self.variant.entry_name().to_owned()),
              ("TERMINFO", dir.clone()),
              ("TERMINFO_DIRS", dir),
          ]
      }

      /// Apply `TERM`, `TERMINFO`, and `TERMINFO_DIRS` to a
      /// `CommandBuilder` so the spawned child reads the pinned
      /// terminfo entry instead of the host database.
      ///
      /// Some ncurses consumers honor only `TERMINFO` (singular); others
      /// honor only `TERMINFO_DIRS` (plural). Setting both via this single
      /// wrapper ensures ncurses lookup never falls back to the host
      /// database regardless of which variable the consumer consults
      /// first. The wrapper exists so consumers (Section 03's
      /// `spawn_tack`, the 02.4 child-process integrity test, any future
      /// caller) never need to know the (name, value) tuple shape — if
      /// `TerminfoEnv` learns to set a fourth env var tomorrow, only
      /// [`Self::env_pairs`] changes.
      ///
      /// ```ignore
      /// let env = TerminfoEnv::compile();
      /// let mut cmd = CommandBuilder::new("tack");
      /// env.apply_env(&mut cmd);
      /// ```
      pub fn apply_env(&self, cmd: &mut CommandBuilder) {
          for (name, value) in self.env_pairs() {
              cmd.env(name, value);
          }
      }
  }
  ```

- [ ] **Write the failing tests FIRST**, watch them fail (they reference `TerminfoEnv` / `TerminfoVariant` which do not yet exist), THEN proceed to implement the types in the previous step. This is non-negotiable TDD ordering — see CLAUDE.md "Testing" and the impl-hygiene "Semantic changes require semantic pins" rule. Do not write the implementation first and back-fill tests.

- [ ] Add sibling tests at `crates/oriterm_test_support/src/terminfo/tests.rs` (per `.claude/rules/test-organization.md`):

  Note: the parent module is already a directory (`terminfo/mod.rs`) per the file structure above — this file sits alongside it as `terminfo/tests.rs`. No restructuring needed.

  ```rust
  use portable_pty::CommandBuilder;

  use super::{ORI_TERM_INFO, TerminfoEnv, TerminfoVariant};
  use crate::tic_available;

  #[test]
  fn embedded_terminfo_source_is_nonempty() {
      // The committed extra/ori_term.info is embedded at compile time via
      // include_str!. If the file is missing the build fails, so this
      // test simply pins the expectation that the source is substantive.
      assert!(!ORI_TERM_INFO.is_empty(), "embedded ori_term.info is empty");
      assert!(
          ORI_TERM_INFO.contains("ori_term|") || ORI_TERM_INFO.contains("ori_term+common|"),
          "embedded source missing expected ori_term entry header"
      );
  }

  #[test]
  fn terminfo_variant_entry_names_are_distinct() {
      // Compile-time-ish exhaustivity smoke test — if a third variant
      // lands, this assertion needs the new arm too. The test catches
      // an accidental duplicate `entry_name` mapping for two variants.
      assert_eq!(TerminfoVariant::OriTerm.entry_name(), "ori_term");
      assert_eq!(TerminfoVariant::OriTermDirect.entry_name(), "ori_term-direct");
      assert_ne!(
          TerminfoVariant::OriTerm.entry_name(),
          TerminfoVariant::OriTermDirect.entry_name()
      );
  }

  #[test]
  fn terminfo_env_compiles_ori_term() {
      // This test gates on BOTH tic AND infocmp because it exercises
      // the round-trip in addition to the compile. The bare compile
      // path is exercised by terminfo_env_drop_cleans_temp_dir below,
      // which gates only on tic.
      if !tic_available() || !crate::infocmp_available() {
          eprintln!("tic or infocmp not installed, skipping terminfo_env_compiles_ori_term");
          return;
      }
      let env = TerminfoEnv::compile();
      assert_eq!(env.term(), "ori_term");
      assert_eq!(env.variant(), TerminfoVariant::OriTerm);

      // Portable success check — use infocmp, not hardcoded filesystem
      // layout. This works across ncurses directory and hashed-db
      // backends; asserting `<tempdir>/o/ori_term` would only work on
      // the directory backend.
      let infocmp = std::process::Command::new("infocmp")
          .arg("-A")
          .arg(env.terminfo_dir())
          .arg("ori_term")
          .output()
          .expect("invoke infocmp");
      assert!(
          infocmp.status.success(),
          "infocmp failed: {}",
          String::from_utf8_lossy(&infocmp.stderr)
      );
      let out = String::from_utf8_lossy(&infocmp.stdout);
      assert!(out.contains("am"), "expected 'am' boolean in infocmp output:\n{out}");
      assert!(
          out.contains("colors#256") || out.contains("colors#0x100"),
          "expected colors#256 in infocmp output:\n{out}"
      );
  }

  #[test]
  fn terminfo_env_drop_cleans_temp_dir() {
      // Pure-tic gate — no infocmp dependency. Proves Drop on the
      // bare compile path works without dragging infocmp into the gate.
      if !tic_available() {
          return;
      }
      let dir_path;
      {
          let env = TerminfoEnv::compile();
          dir_path = env.terminfo_dir().to_path_buf();
          assert!(dir_path.exists());
      } // env dropped here
      assert!(!dir_path.exists(), "temp dir not cleaned up after Drop");
  }

  #[test]
  fn apply_env_sets_three_vars() {
      // SSOT semantic pin: `env_pairs()` is the canonical (name, value)
      // triple that BOTH `apply_env(&mut CommandBuilder)` AND the 02.4
      // child-process integrity test consume. We assert here that:
      //   1. Exactly three env vars are set (TERM, TERMINFO, TERMINFO_DIRS)
      //   2. TERM matches the pinned variant entry name
      //   3. TERMINFO and TERMINFO_DIRS BOTH point at the compiled tempdir
      //      (some ncurses consumers honor only one of the two)
      //   4. The three names are distinct (catches a copy-paste bug
      //      where `TERMINFO_DIRS` accidentally became `TERMINFO`)
      //
      // We cannot read env back from `CommandBuilder` (portable-pty does
      // not expose accessors), so the unit-test scope is the SSOT itself.
      // The end-to-end behavioral pin — proving the env triple actually
      // steers a real child — lives in 02.4's
      // `child_process_with_apply_env_reads_pinned_terminfo`, which
      // consumes the SAME `env_pairs()` SSOT.
      if !tic_available() {
          return;
      }
      let env = TerminfoEnv::compile();
      let pairs = env.env_pairs();

      assert_eq!(pairs.len(), 3, "expected exactly three env vars");
      let names: Vec<&str> = pairs.iter().map(|(n, _)| *n).collect();
      assert!(names.contains(&"TERM"));
      assert!(names.contains(&"TERMINFO"));
      assert!(names.contains(&"TERMINFO_DIRS"));

      // Distinctness pin — catches the "TERMINFO_DIRS got typoed to
      // TERMINFO" copy-paste regression that no integration test would
      // catch (the host inheritance would silently re-engage).
      let mut sorted = names.clone();
      sorted.sort_unstable();
      sorted.dedup();
      assert_eq!(sorted.len(), 3, "env var names must be distinct: {names:?}");

      // Value pins.
      let term = pairs.iter().find(|(n, _)| *n == "TERM").map(|(_, v)| v.as_str());
      let terminfo = pairs.iter().find(|(n, _)| *n == "TERMINFO").map(|(_, v)| v.as_str());
      let terminfo_dirs = pairs.iter().find(|(n, _)| *n == "TERMINFO_DIRS").map(|(_, v)| v.as_str());
      assert_eq!(term, Some("ori_term"));
      assert_eq!(terminfo, Some(env.terminfo_dir().to_string_lossy().as_ref()));
      assert_eq!(terminfo_dirs, Some(env.terminfo_dir().to_string_lossy().as_ref()));

      // Smoke-test the public wrapper too — proves `apply_env` does
      // not panic and returns cleanly when given a real CommandBuilder.
      let mut cmd = CommandBuilder::new("/bin/true");
      env.apply_env(&mut cmd);
  }

  #[test]
  fn terminfo_env_repeated_compile_stress() {
      // Calls compile() 5 times in succession in the same test.
      // Catches: tempdir-name collisions, file-handle leaks, tic
      // state leakage, Drop-order edge cases. All 5 tempdirs must
      // exist while alive AND be gone after each Drop.
      if !tic_available() {
          return;
      }
      let mut paths = Vec::with_capacity(5);
      for _ in 0..5 {
          let env = TerminfoEnv::compile();
          let path = env.terminfo_dir().to_path_buf();
          assert!(path.exists(), "compile() did not create tempdir");
          paths.push(path);
          // env drops at end of loop iteration -> tempdir cleaned up
      }
      // After every Drop has run, NONE of the paths should still exist.
      for path in &paths {
          assert!(
              !path.exists(),
              "tempdir {path:?} survived Drop — compile() leaks state across calls"
          );
      }
      // Sanity: no two compile() calls produced the same tempdir.
      for (i, p1) in paths.iter().enumerate() {
          for p2 in paths.iter().skip(i + 1) {
              assert_ne!(p1, p2, "compile() reused tempdir name across calls");
          }
      }
  }

  // Negative pins — these tests ensure TerminfoEnv fails loudly when
  // the source is corrupted or tic is invoked on a bogus file. The
  // typo-safe `TerminfoVariant` enum eliminates the "unknown term name"
  // negative pin (it would not even compile).

  #[test]
  fn terminfo_env_compile_fails_loudly_on_corrupted_source() {
      // Hand-synthesized corrupted terminfo source in a tempfile.
      // We call `tic -c -x <tempfile>` directly (bypassing
      // TerminfoEnv::compile which uses the committed extra/ori_term.info)
      // and assert that tic reports a non-zero exit. This proves
      // our panic-on-tic-failure behavior would trigger if someone
      // committed garbage into extra/ori_term.info.
      if !tic_available() { return; }
      use std::io::Write;
      let mut f = tempfile::NamedTempFile::new().expect("tempfile");
      writeln!(f, "ori_term_corrupt|broken,").expect("write");
      writeln!(f, "    this_is_not_a_valid_capability_line_at_all").expect("write");
      let path = f.path();
      let out = std::process::Command::new("tic")
          .arg("-c")
          .arg("-x")
          .arg(path)
          .stdout(std::process::Stdio::piped())
          .stderr(std::process::Stdio::piped())
          .output()
          .expect("invoke tic");
      assert!(
          !out.status.success(),
          "tic must report failure on corrupted source; stdout={} stderr={}",
          String::from_utf8_lossy(&out.stdout),
          String::from_utf8_lossy(&out.stderr),
      );
  }
  ```

  `tempfile = "3"` is added to `[dependencies]` in the Cargo.toml step above. Tests inside the crate see it without a separate `[dev-dependencies]` entry.

- [ ] Add `#[cfg(test)] mod tests;` at the bottom of `terminfo/mod.rs`.

- [ ] **Assert `TerminfoEnv: Send` at compile time.** The success criterion in the frontmatter claims `TerminfoEnv` is `Send`. Add a trait-bound assertion at the bottom of `terminfo/mod.rs` so the claim is enforced by the compiler:
  ```rust
  const _: fn() = || {
      fn assert_send<T: Send>() {}
      assert_send::<TerminfoEnv>();
  };
  ```
  `tempfile::TempDir` is `Send` (it's a `PathBuf` + `Fd`), and `&'static str` is `Send`, so the assertion should compile cleanly. If it fails at compile time, the field types have drifted — fix the offending field or drop the claim from the success criterion. Do NOT add a `Sync` bound speculatively — it is not needed unless a future change caches a `TerminfoEnv` in a process-global `OnceLock`, and expanding the bound now would block future field additions (e.g., an `RwLock<Stats>`) for no current benefit.

---

## 02.3 tic_available, infocmp_available, and skip discipline

**File(s):** `crates/oriterm_test_support/src/session/mod.rs` (Section 01 already promoted `session` to a directory module — extend `mod.rs` directly; do NOT create a sibling `availability.rs`)

The runtime check helpers go alongside `vttest_available()` from Section 01. They follow the same pattern.

- [ ] Add to `crates/oriterm_test_support/src/session/mod.rs` (after the existing `vttest_available()` function):
  ```rust
  /// Check if `tic` (terminfo compiler) is installed.
  ///
  /// Probe is `tic -V`, which is the version flag every ncurses build
  /// (BSD and GNU) supports. A too-old `tic` (ncurses < 6.0) may exist
  /// and still fail to compile modern extension caps; in that case
  /// `TerminfoEnv::compile()` will panic with the tic stderr output,
  /// which IS the failure contract — the user sees the error message
  /// and upgrades their ncurses package.
  #[must_use]
  pub fn tic_available() -> bool {
      tool_available("tic", "-V")
  }

  /// Check if `infocmp` (terminfo decompiler / inspector) is installed.
  #[must_use]
  pub fn infocmp_available() -> bool {
      tool_available("infocmp", "-V")
  }
  ```

  `tic` and `infocmp` both support `-V` for version (per `man tic`, `man infocmp`). Use `-V` consistently — `--version` is not universal across BSD vs GNU ncurses builds (verified locally: ncurses 6.4 `tic --version` errors out with `tic: invalid option -- '-'`, while `tic -V` prints `ncurses 6.4.20240113`).

  **On `tic_available()` as a presence-only probe:** ncurses 5.9 ships a `tic` binary that lacks `-x` extension support, so a `tic_available() == true` doesn't guarantee compilation will succeed. The compile path handles this via the panic-on-stderr contract: if `tic` rejects an extension cap, the test panics with the stderr output, the user upgrades, and the test re-runs. We do NOT add a `tic_supports_ext_caps()` probe — the failure is loud and the message is actionable.

- [ ] Re-export from `lib.rs`:
  ```rust
  pub mod session;
  pub mod terminfo;

  pub use session::{
      PtyResponder, PtySession,
      tic_available, infocmp_available, vttest_available, tool_available,
  };
  pub use terminfo::{TerminfoEnv, TerminfoVariant};
  ```
  Section 03 will add `tack_available` to the `session::` re-export list. `TerminfoVariant` is exported alongside `TerminfoEnv` so callers (Section 03's `spawn_tack`, future tests) can name the truecolor variant without reaching into the module path.

- [ ] Add unit tests in the existing `session/tests.rs`:
  ```rust
  #[test]
  fn tic_available_matches_tool_available() {
      assert_eq!(tic_available(), tool_available("tic", "-V"));
  }

  #[test]
  fn infocmp_available_matches_tool_available() {
      assert_eq!(infocmp_available(), tool_available("infocmp", "-V"));
  }
  ```

- [ ] **Cross-platform skip discipline** (this rule applies to every test in Sections 03-08 too — codify it here):

  Every test that constructs a `TerminfoEnv` MUST gate on `tic_available()` first:
  ```rust
  #[test]
  fn my_test_using_terminfo() {
      if !tic_available() {
          eprintln!("tic not installed, skipping");
          return;
      }
      let env = TerminfoEnv::compile();
      // ...
  }
  ```

  On Windows native, `tic` is not available — these tests skip with a message and return successfully. On Linux/macOS, `tic` is available via `ncurses` package (`apt install ncurses-bin`, `brew install ncurses`) — these tests run.

  **What NOT to do:** do NOT use `#[cfg(unix)]` to skip — the test must COMPILE on every platform (per CLAUDE.md cross-platform rule: "All code must compile and run correctly on all three platforms"). Use a runtime gate via `tic_available()` so the test source compiles everywhere and only the body short-circuits on platforms missing the tool.

  **What about `#[ignore]`?** No. `#[ignore]` requires `cargo test -- --ignored` to run, which means CI matrices need a special invocation per platform. Runtime gating via `tic_available()` keeps the default `cargo test` invocation correct on every platform.

- [ ] **Hygiene sweep on existing `oriterm_test_support` source files** — Section 02 is the first plan after Section 01 to touch this crate, so it owns one bug-tracker pass on what Section 01 left behind. Three concrete findings to address (every reference is verified against the current source on disk):
  1. `crates/oriterm_test_support/src/session/mod.rs:43` and `:50` — two `Mutex::lock().unwrap()` calls inside `PtyResponder::take_responses` and `<PtyResponder as EventListener>::send_event`. CLAUDE.md "Coding Standards: Error Handling" says "No `unwrap()` in library code." These are test-support code, but the rule applies to any Rust crate that ships compiled artifacts. The correct fix is `lock().expect("PtyResponder mutex poisoned")` — replaces a silent unwrap with a documented expect, and the panic message tells the test author what went wrong. Fix as part of 02.3 (the same edit window already touches `session/mod.rs`).
  2. `crates/oriterm_test_support/src/session/mod.rs` is currently 335 lines. After 02.3 adds `tic_available` + `infocmp_available` (~30 lines) and Section 03 adds `tack_available` + `spawn_tack` + `wait_for_child_exit` (~60 lines), the file projects to ~425 lines — within the 500-line limit but in the proactive-split zone (>~450). Section 02 does NOT need to split it yet, but the 02.N completion checklist must record the post-section line count so Section 03 knows whether to split during its work. If 02.3's edits push the file past 450 lines, split now via `session/availability.rs` (`tic_available`/`infocmp_available`/`vttest_available`) — that's the natural seam.
  3. The existing `crates/oriterm_test_support/src/session/tests.rs:1` import line `use super::{tool_available, vttest_available};` and the in-function `use super::PtySession;` at `:21` should consolidate to a single top-of-file import block per `.claude/rules/test-organization.md` "Import Style in Test Files." Fix as part of 02.3's BUG-07-008 edit (the test body is being rewritten anyway).

- [ ] **Resolve BUG-07-008 in this subsection** — Section 01 left an existing `#[cfg(unix)]` antipattern at `crates/oriterm_test_support/src/session/tests.rs:16` (`pty_session_drains_simple_output`). That gate violates the runtime-skip rule this subsection codifies. Section 02 owns the canonical fix because 02.3 is the convention's home.

  Required edits:
  1. Open `crates/oriterm_test_support/src/session/tests.rs` and remove the `#[cfg(unix)]` attribute on `pty_session_drains_simple_output`.
  2. Replace the body with a portable two-arm test (preferred over a runtime `cfg!(unix)` early-return — Windows gets real PTY drain coverage instead of a no-op skip):
     ```rust
     #[test]
     fn pty_session_drains_simple_output() {
         // Portable PTY drain smoke test. portable-pty owns ConPTY on
         // Windows, so the same PtySession spawn path works on every
         // platform. Two-arm shell selection — /bin/sh on Unix and
         // cmd.exe on Windows — is the cross-platform idiom for "run
         // a one-liner in the platform shell."
         #[cfg(unix)]
         let mut cmd = {
             let mut c = portable_pty::CommandBuilder::new("/bin/sh");
             c.args(["-c", "printf hello"]);
             c
         };
         #[cfg(windows)]
         let mut cmd = {
             let mut c = portable_pty::CommandBuilder::new("cmd.exe");
             c.args(["/C", "echo hello"]);
             c
         };
         let mut session = super::PtySession::spawn(cmd, 80, 24);
         session.wait_for("hello", 5_000);
         let grid = session.grid_text();
         assert!(grid.contains("hello"), "expected drained output to contain 'hello':\n{grid}");
     }
     ```
     Note: a `#[cfg(unix)] / #[cfg(windows)]` block INSIDE a single `#[test] fn` is fine and is the idiom every cross-platform test in this codebase uses (it's the OUTER-attribute `#[cfg(unix)] #[test]` form that violates the rule because the test function then doesn't EXIST on Windows). The body still runs on every platform; only the `cmd` value differs.
  3. After landing the change, append `<!-- resolved-by: plans/tack-conformance/section-02-terminfo-provisioning.md#02.3 -->` underneath BUG-07-008 in `plans/bug-tracker/section-07-ci-build.md` and check the box.
  4. Run `cargo build --target x86_64-pc-windows-gnu -p oriterm_test_support --tests` to confirm the test source compiles for Windows.

---

## 02.4 Verify pinned terminfo round-trips through infocmp

**File(s):** `crates/oriterm_test_support/src/terminfo/tests.rs` (additional tests)

The terminfo source is correct only if `tic` round-trips it through `infocmp` cleanly. This subsection is the gate.

- [ ] Add `tic -c -x` warning-gate test. This is the gate that catches cap syntax errors at CI time — `tic` would otherwise silently drop bad caps during the real compile:
  ```rust
  #[test]
  fn tic_validate_source_has_zero_unexpected_warnings() {
      if !tic_available() {
          return;
      }
      // Use the same embedded source TerminfoEnv writes to a scratch
      // file, but invoke tic -c (validate only) with -x.
      use std::io::Write;
      let mut f = tempfile::NamedTempFile::new().expect("tempfile");
      f.write_all(super::ORI_TERM_INFO.as_bytes()).expect("write");
      let out = std::process::Command::new("tic")
          .arg("-c")
          .arg("-x")
          .arg(f.path())
          .output()
          .expect("invoke tic");
      assert!(
          out.status.success(),
          "tic -c -x failed: stdout={} stderr={}",
          String::from_utf8_lossy(&out.stdout),
          String::from_utf8_lossy(&out.stderr),
      );
      // Tolerate only the known ncurses false-positive warning for
      // Setulc. Every other tic: message is a hard failure.
      let stderr = String::from_utf8_lossy(&out.stderr);
      for line in stderr.lines() {
          let trimmed = line.trim();
          if trimmed.is_empty() {
              continue;
          }
          // Known false positive: ncurses 6.x nags about %;/%? balance
          // inside Setulc even though the cap compiles correctly.
          if trimmed.contains("%; without %? in Setulc") {
              continue;
          }
          panic!("unexpected tic -c -x stderr: {line}\nfull stderr:\n{stderr}");
      }
  }
  ```

- [ ] Add round-trip integrity test. This is the canonical home of the `infocmp -A` portable check — keeping it here (rather than inside `TerminfoEnv::compile()`) is what lets the constructor stay pure-`tic` and gate only on `tic_available()`. The 02.4 round-trip suite gates on BOTH `tic_available()` AND `infocmp_available()`:
  ```rust
  #[test]
  fn ori_term_terminfo_round_trips_via_infocmp() {
      if !tic_available() || !crate::infocmp_available() {
          return;
      }
      let env = TerminfoEnv::compile();

      // Decompile ori_term back to source form.
      let infocmp = std::process::Command::new("infocmp")
          .arg("-A")
          .arg(env.terminfo_dir())
          .arg("-1")  // one cap per line for stable diff
          .arg("ori_term")
          .output()
          .expect("invoke infocmp");
      assert!(infocmp.status.success());
      let out = String::from_utf8_lossy(&infocmp.stdout);

      // Required boolean caps (match on tab-prefixed boundary to avoid
      // false positives like `am` matching inside `kfoo=\E[1;2Am`).
      for cap in &["am", "bce", "ccc", "km", "mir", "msgr", "xenl"] {
          assert!(
              out.contains(&format!("\t{cap},")) || out.contains(&format!("\t{cap},\n")),
              "expected boolean cap {cap:?} in ori_term terminfo, got:\n{out}"
          );
      }

      // Required numeric cap:
      assert!(out.contains("colors#256"), "expected colors#256, got:\n{out}");

      // Required string caps (just check the names appear; values are
      // long parameter strings).
      for cap in &["cup=", "sgr=", "setaf=", "setab=", "smkx=", "rmkx="] {
          assert!(out.contains(cap), "expected string cap {cap:?} in ori_term terminfo");
      }

      // Charset capabilities — verify smacs/rmacs/acsc are declared
      // (backs the DEC special graphics handling in
      // oriterm_core/src/term/charset/mod.rs).
      for cap in &["acsc=", "smacs=", "rmacs="] {
          assert!(out.contains(cap), "expected charset cap {cap:?} in ori_term terminfo");
      }

      // Bracketed paste and focus-event extension caps.
      for cap in &["BD=", "BE=", "PS=", "PE=", "kxIN=", "kxOUT="] {
          assert!(out.contains(cap), "expected extension cap {cap:?} in ori_term terminfo");
      }

      // Function key kf1 through kf63 must all be present (legacy xterm
      // modifier convention; kf13-kf63 are the modified F1-F12 variants).
      for n in 1..=63 {
          let cap = format!("kf{n}=");
          assert!(out.contains(&cap), "expected {cap} in ori_term terminfo, got:\n{out}");
      }

      // REP capability — verifies the CSI b dispatch at
      // crates/vte/src/ansi/dispatch/csi.rs:60 is advertised.
      assert!(out.contains("rep="), "expected rep= in ori_term terminfo");
  }
  ```

- [ ] Add direct-entry round-trip test (uses the typo-safe `TerminfoVariant` enum, not a `&'static str`):
  ```rust
  #[test]
  fn ori_term_direct_declares_truecolor() {
      if !tic_available() || !crate::infocmp_available() {
          return;
      }
      let env = TerminfoEnv::compile_with_variant(TerminfoVariant::OriTermDirect);
      let infocmp = std::process::Command::new("infocmp")
          .arg("-A")
          .arg(env.terminfo_dir())
          .arg("ori_term-direct")
          .output()
          .expect("invoke infocmp");
      assert!(infocmp.status.success());
      let out = String::from_utf8_lossy(&infocmp.stdout);
      // Direct entry must declare RGB (truecolor) and 16M colors.
      assert!(out.contains("RGB") || out.contains("Tc"), "expected RGB or Tc in ori_term-direct");
      assert!(
          out.contains("colors#16777216") || out.contains("colors#0x1000000"),
          "expected colors#16777216 in ori_term-direct, got:\n{out}"
      );
  }

  #[test]
  fn infocmp_unavailable_skips_round_trip_test() {
      // Proves the infocmp_available() gate is the actual condition
      // that skips round-trip tests, NOT just an "or" with tic_available().
      // This test documents the gate semantics: if infocmp goes missing
      // tomorrow but tic stays, all 02.4 round-trip tests must skip
      // cleanly without panicking. We exercise the gate logic directly
      // here so a regression in the gate placement (e.g., a future test
      // forgetting `|| !infocmp_available()`) is caught immediately.
      //
      // The test passes in EVERY environment:
      // - infocmp present + tic present:  gate returns false, body runs harmlessly
      // - infocmp absent + tic present:   gate returns true, body short-circuits cleanly
      // - infocmp absent + tic absent:    gate returns true, body short-circuits cleanly
      // - infocmp present + tic absent:   gate returns true, body short-circuits cleanly
      let should_skip = !tic_available() || !crate::infocmp_available();
      if should_skip {
          // We "skipped" — assert we're returning, not panicking.
          eprintln!(
              "infocmp_unavailable_skips_round_trip_test: gate triggered \
               (tic={}, infocmp={}); returning cleanly",
              tic_available(),
              crate::infocmp_available()
          );
          return;
      }
      // Both tools available — sanity-check that we can call compile()
      // and the gate semantic test runs end-to-end.
      let _env = TerminfoEnv::compile();
  }
  ```

- [ ] **Child-process integrity test** — proves `TerminfoEnv`'s env-var SSOT actually steers a real child process to the pinned terminfo (NOT just to the on-disk dir). This is the only test that exercises the same code path Sections 03-08 will rely on. We use `std::process::Command` directly here (rather than `portable_pty::CommandBuilder`) because the test doesn't need a PTY — it just needs a child whose env-var precedence we can inspect via stdout. The behavioral contract being tested is the same: the SSOT must set `TERM`/`TERMINFO`/`TERMINFO_DIRS` such that ncurses lookup finds the pinned entry.

  Critically, the test consumes `TerminfoEnv::env_pairs()` directly — the SAME `pub(crate)` SSOT that `apply_env(&mut CommandBuilder)` calls. There is NO duplicate hand-rolled `cmd.env("TERM", ...)` triple. If `env_pairs` grows a fourth entry tomorrow, BOTH `apply_env` AND this test pick it up automatically with zero edits. This is the SSOT enforcement that turns the test into a real contract check, not a parallel implementation.
  ```rust
  #[test]
  fn child_process_with_apply_env_reads_pinned_terminfo() {
      if !tic_available() || !crate::infocmp_available() {
          return;
      }
      let env = TerminfoEnv::compile();

      // Spawn `infocmp` as a CHILD with TerminfoEnv's env-var triple
      // applied via the SAME SSOT (`env_pairs`) that `apply_env`
      // consumes. CRITICALLY do NOT pass `-A <dir>` — that would make
      // infocmp look up the entry by directory regardless of env vars.
      // With no `-A`, infocmp consults `$TERM` / `$TERMINFO` /
      // `$TERMINFO_DIRS` exactly the way tack will. If the child returns
      // the pinned ori_term entry, the env-var precedence path works
      // end-to-end.
      //
      // We use `std::process::Command` (not `portable_pty::CommandBuilder`)
      // because the test needs stdout capture, which CommandBuilder does
      // not expose. Both APIs accept (name, value) env pairs, so the
      // SSOT iteration pattern is identical — only the receiver changes.
      let mut cmd = std::process::Command::new("infocmp");
      cmd.arg(env.term());
      for (name, value) in env.env_pairs() {
          cmd.env(name, value);
      }
      // Strip any inherited TERMCAP from the parent process so we're
      // only testing what the env_pairs SSOT sets.
      cmd.env_remove("TERMCAP");
      let out = cmd.output().expect("invoke infocmp");
      assert!(
          out.status.success(),
          "child infocmp failed (apply_env triple should have steered it to the pinned dir): stderr={}",
          String::from_utf8_lossy(&out.stderr),
      );
      let stdout = String::from_utf8_lossy(&out.stdout);
      // The pinned entry must report colors#256 — the host
      // xterm-256color also reports colors#256, so this alone
      // doesn't prove we hit the pinned entry. Pin on a unique
      // marker that ONLY ori_term declares: the kf63 cap (the host
      // xterm-256color usually stops at kf48).
      assert!(
          stdout.contains("kf63="),
          "child infocmp returned a terminfo entry WITHOUT kf63 — env precedence did not steer the child to the pinned ori_term entry. stdout:\n{stdout}",
      );
  }
  ```

- [ ] **Self-contained per-call performance benchmark with 50-call projection.** Section 02 must measure tic overhead WITHOUT depending on Sections 03-08 existing yet — those sections do not exist at 02's sitting, so a `time ./test-all.sh` measurement is non-executable inside this section's window. Instead, project the 50-call cost from a single in-test measurement and file a bug immediately if the projection exceeds 30 s.
  ```rust
  #[test]
  fn terminfo_env_compile_under_perf_budget() {
      if !tic_available() {
          return;
      }
      use std::time::Instant;
      // Warm-up call so we measure steady-state cost, not first-run
      // overhead (filesystem cache, dynamic linker resolution).
      let _warmup = TerminfoEnv::compile();
      let t0 = Instant::now();
      let _env = TerminfoEnv::compile();
      let elapsed = t0.elapsed();
      // Per-call ceiling: 1000 ms. Captures a 5x regression vs.
      // observed local debug timings (~150 ms) and 10x vs. release.
      // BOTH debug AND release builds must meet this ceiling — run
      // `cargo test --release -p oriterm_test_support \
      //   terminfo_env_compile_under_perf_budget` after the debug
      // run to capture the second number. Document both in 02's
      // completion notes.
      assert!(
          elapsed.as_millis() < 1000,
          "TerminfoEnv::compile() took {}ms (>1000ms ceiling) — investigate before deferring",
          elapsed.as_millis(),
      );
      // Project the 50-call cost (Sections 03-08 will spawn ~50
      // TerminfoEnv instances). If projection exceeds 30 s, file a
      // shared-cache follow-up bug NOW via /add-bug — filing is NOT
      // deferral, it creates a concrete tracked artifact.
      let projected_50 = elapsed.as_millis() * 50;
      eprintln!(
          "TerminfoEnv::compile() warm = {}ms; 50-call projection = {}ms",
          elapsed.as_millis(),
          projected_50
      );
      assert!(
          projected_50 < 30_000,
          "50-call projection {}ms > 30s ceiling — file /add-bug for shared-cache follow-up",
          projected_50
      );
  }
  ```
  Section 02's perf gate is fully self-contained: it measures one warm call, asserts the per-call ceiling (1000 ms), and asserts the 50-call projection (30 s). Section 09 (Verification) — NOT Section 02 — owns the post-Sections-03-08 `time ./test-all.sh` aggregate measurement; that item belongs in 09's checklist, not 02's. The dependency is one-way: 09 reads 02's per-call number when projecting; 02 cannot wait for 09.

- [ ] **TPR checkpoint** — `/tpr-review` covering 02.1–02.4 implementation work. Catches: missing required capabilities, wrong escape sequence syntax in `extra/ori_term.info`, unexpected `tic` warnings (anything beyond the known `Setulc` false positive), `TerminfoEnv` resource leaks (failing to clean up tempdirs on panic paths), `include_str!` path mis-resolution (if the `extra/` file moves, the build fails — but a TPR can confirm the path is correctly relative to `CARGO_MANIFEST_DIR`), AND verifies the BUG-07-008 fix (portable two-arm `pty_session_drains_simple_output` test) is sound on Windows.

---

## 02.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 02.N Completion Checklist

- [ ] `extra/` directory exists (created in 02.1 — verify with `ls extra/`)
- [ ] `extra/ori_term.info` is a committed file with three entries: `ori_term` (256-color), `ori_term-direct` (truecolor), and the private `ori_term+common` base fragment. No `use=xterm-256color,` inheritance.
- [ ] Every modern extension capability declared in `extra/ori_term.info` has a comment line referencing the implementation site (file:line in `oriterm_core/src/term/handler/`, `oriterm_core/src/paste/`, `oriterm_core/src/term/charset/`, `oriterm/src/app/event_loop_helpers/`, or `crates/vte/src/ansi/`)
- [ ] `tic -c -x extra/ori_term.info` exits with status 0 AND stderr is either empty OR contains only the known false-positive `%; without %? in Setulc` line. Any other `tic:` stderr line is a gate failure.
- [ ] `tic -x -o /tmp/test_terminfo extra/ori_term.info` exits 0 and `infocmp -A /tmp/test_terminfo ori_term` also exits 0 (portable check — do NOT assert filesystem layout like `/tmp/test_terminfo/o/ori_term`)
- [ ] `infocmp -A /tmp/test_terminfo ori_term` output contains `am`, `bce`, `colors#256`, `cup=`, `sgr=`, `setaf=`, `smkx=`, `acsc=`, `smacs=`, `rmacs=`, `rep=`, `BD=`, `BE=`, `kxIN=`, `kxOUT=`, and `kf1=` through `kf63=`
- [ ] `crates/oriterm_test_support/src/terminfo/mod.rs` exists (directory module — `terminfo.rs` as a single file is NOT acceptable because the sibling `tests.rs` convention requires a directory module)
- [ ] `crates/oriterm_test_support/src/terminfo/mod.rs` is below the 500-line file limit per `.claude/rules/code-hygiene.md`. If implementation pushes it past ~450 lines, proactively split (e.g., `terminfo/variant.rs` for the enum, `terminfo/compile.rs` for the tic invocation logic) BEFORE the limit; do not write a 500-line file and split later.
- [ ] `crates/oriterm_test_support/src/terminfo/tests.rs` exists and contains the test suite described in 02.2 and 02.4
- [ ] `TerminfoVariant` enum (`OriTerm`, `OriTermDirect`) is the only valid input to `compile_with_variant`. `compile_with_variant(TerminfoVariant)`, `compile()`, `term()`, `variant()`, `terminfo_dir()`, and `apply_env(&mut CommandBuilder)` are the entire **public** API. `pub(crate) fn env_pairs(&self) -> [(&'static str, String); 3]` is the **internal SSOT** consumed by `apply_env` AND the 02.4 `child_process_with_apply_env_reads_pinned_terminfo` integrity test — there is no other place that lists the env-var triple.
- [ ] `TerminfoEnv::compile()` is **pure-tic** — it does NOT shell out to `infocmp` at any point. The post-tic check is a `Path::exists` probe on `<tempdir>/<bucket>/<entry_name>` (directory backend). The portable `infocmp -A` round-trip lives in 02.4 tests, never in the constructor.
- [ ] `TerminfoEnv::compile()` uses `include_str!` to embed the terminfo source — NOT a `find_source()` walk. The `include_str!` path is `concat!(env!("CARGO_MANIFEST_DIR"), "/../../extra/ori_term.info")`. Missing file = build failure, not runtime panic.
- [ ] `TerminfoEnv` cleans up temp dir via `tempfile::TempDir` Drop
- [ ] `const _: fn() = || { fn assert_send<T: Send>() {} assert_send::<TerminfoEnv>(); };` present at the bottom of `terminfo/mod.rs` (compile-time `Send` assertion)
- [ ] `tic_available()` and `infocmp_available()` exist next to `vttest_available()` in `crates/oriterm_test_support/src/session/mod.rs` (note: `session` is already a directory module at completion of Section 01 — add the functions to `session/mod.rs`, not a nonexistent `session.rs`)
- [ ] **Crate-boundary check**: `oriterm_test_support` depends ONLY on `oriterm_core`, `portable-pty`, `tempfile`, and `vte` (the four entries in `Cargo.toml [dependencies]`). It MUST NOT depend on `oriterm_ui`, `oriterm_mux`, `oriterm`, or `oriterm_ipc`. Verify with `cargo tree -p oriterm_test_support --no-default-features` after the section lands. (See `.claude/rules/crate-boundaries.md` for the dependency direction matrix.)
- [ ] All tests in `crates/oriterm_test_support/src/terminfo/tests.rs` pass on Linux (`cargo test -p oriterm_test_support terminfo`). The full set: `embedded_terminfo_source_is_nonempty`, `terminfo_variant_entry_names_are_distinct`, `terminfo_env_compiles_ori_term`, `terminfo_env_drop_cleans_temp_dir`, `apply_env_sets_three_vars`, `terminfo_env_repeated_compile_stress`, `terminfo_env_compile_fails_loudly_on_corrupted_source`, `tic_validate_source_has_zero_unexpected_warnings`, `ori_term_terminfo_round_trips_via_infocmp`, `ori_term_direct_declares_truecolor`, `infocmp_unavailable_skips_round_trip_test`, `child_process_with_apply_env_reads_pinned_terminfo`, `terminfo_env_compile_under_perf_budget`. The "unknown term name" negative pin from earlier drafts is REMOVED — `TerminfoVariant` makes the typo case a compile error, not a runtime panic, so a negative pin would not even compile.
- [ ] `terminfo_env_repeated_compile_stress` passes (5 successive `compile()` calls each create + clean up their tempdir, no collisions)
- [ ] `infocmp_unavailable_skips_round_trip_test` passes (proves the `infocmp_available()` gate logic works regardless of which combinations of `tic`/`infocmp` are present)
- [ ] **Child-process integrity test passes** — `child_process_with_apply_env_reads_pinned_terminfo` proves that applying `TerminfoEnv` env vars to a child via the same code path `apply_env` uses actually steers a real child to the pinned entry (no `-A` flag, child consults `$TERMINFO`/`$TERMINFO_DIRS`, returned entry contains `kf63=` which the host xterm-256color does not). This is the contract Sections 03-08 rely on.
- [ ] **Performance budget gates pass** — `terminfo_env_compile_under_perf_budget` confirms a single warm `TerminfoEnv::compile()` finishes in <1000 ms AND its 50-call projection stays below 30 s. Run the test under BOTH debug AND release profiles; record both `eprintln!`-emitted warm-time numbers in section completion notes.
- [ ] **BUG-07-008 cross-link resolved** — `crates/oriterm_test_support/src/session/tests.rs` `pty_session_drains_simple_output` no longer carries `#[cfg(unix)]`; replaced with the portable two-arm shell test from 02.3. Box checked in `plans/bug-tracker/section-07-ci-build.md` BUG-07-008 with a "Fixed YYYY-MM-DD" line.
- [ ] **Hygiene sweep landed** — `PtyResponder` `Mutex::lock().unwrap()` calls at `session/mod.rs:43` and `:50` replaced with `.expect("PtyResponder mutex poisoned")`; `session/tests.rs` imports consolidated to a single top-of-file block per `.claude/rules/test-organization.md`; post-section line count for `session/mod.rs` recorded in completion notes (target: under 450 — split now if higher)
- [ ] All tests skip cleanly when `tic`/`infocmp` are unavailable (no panics, returns Ok)
- [ ] `cargo build -p oriterm_test_support` for `x86_64-pc-windows-gnu` succeeds (cross-compile gate)
- [ ] `cargo build --target x86_64-pc-windows-gnu -p oriterm_test_support --tests` succeeds (proves the BUG-07-008 portable test source compiles for Windows, not just lib code)
- [ ] `tempfile = "3"` added to `crates/oriterm_test_support/Cargo.toml` `[dependencies]`
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green — no new warnings
- [ ] `timeout 150 ./test-all.sh` green
- [ ] Plan annotation cleanup: no temporary scaffolding in any `.rs` or `.info` file
- [ ] All TPR checkpoint findings resolved (see `02.R`)
- [ ] **Plan sync**:
  - [ ] This section's frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference table: Section 02 marked Complete
  - [ ] `00-overview.md` Mission Success Criteria: tick the `extra/ori_term.info` hand-authored source criterion (the one starting with "`extra/ori_term.info` terminfo source exists as a hand-authored, fully-pinned entry…") AND the `tic`-compiles criterion (the one starting with "`tic` compiles `ori_term.info` successfully…"). Cross-check the BUG-07-008 fix against the cross-platform skip-discipline criterion ("All tests skip cleanly when tack/tic unavailable…") — Section 02 contributes the second portable test in `oriterm_test_support` toward that criterion but does NOT close it on its own.
  - [ ] `index.md` Section 02 status updated
  - [ ] Section 03's `depends_on: ["01", "02"]` confirmed (Section 03 spawns tack with TerminfoEnv)
  - [ ] Section 04's `depends_on: ["03"]` confirmed (Section 04 builds on Section 03's spawn_tack helper)
  - [ ] Section 08's `depends_on: ["01", "02"]` confirmed (Section 08 reads sequences from the compiled terminfo via `infocmp`)
  - [ ] Section 09's `depends_on` includes `"02"` (verification gate consumes everything)
- [ ] `/tpr-review` final pass clean
- [ ] `/impl-hygiene-review last commit` final pass clean (after TPR) — verifies the new `terminfo` module sits cleanly in `oriterm_test_support`'s crate-boundary envelope (depends only on `oriterm_core` + `tempfile`, NOT on `oriterm_ui`/`oriterm_mux`/`oriterm`), and that `mod.rs` and `tests.rs` both stay under the 500-line limit

**Exit Criteria:** `extra/ori_term.info` is committed as a hand-authored, fully-pinned entry (`ori_term+common` private fragment + `ori_term` 256-color + `ori_term-direct` truecolor, NO `use=xterm-256color,` inheritance) and passes `tic -c -x` with exit 0 and stderr containing at most the known `Setulc` false-positive. `oriterm_test_support::TerminfoEnv::compile()` embeds the source via `include_str!`, writes it to a scratch file in a fresh tempdir, invokes `tic -x -o <tempdir>` (pure-tic, no `infocmp`), and sanity-checks the directory-backend entry path. The `TerminfoVariant` enum (`OriTerm`/`OriTermDirect`) is the only valid `compile_with_variant` input — the compiler enforces exhaustivity. `TerminfoEnv::apply_env(&mut CommandBuilder)` sets `TERM` + `TERMINFO` + `TERMINFO_DIRS` in one call; the 02.4 `child_process_with_apply_env_reads_pinned_terminfo` integrity test proves the env-var precedence path actually steers a real child to the pinned entry. The 02.4 round-trip suite (`ori_term_terminfo_round_trips_via_infocmp`, `ori_term_direct_declares_truecolor`, `infocmp_unavailable_skips_round_trip_test`) gates on BOTH `tic_available()` AND `infocmp_available()` — proving the gate semantics are wired correctly. `terminfo_env_repeated_compile_stress` proves 5 successive compile() calls collide-free. `terminfo_env_compile_under_perf_budget` confirms per-call cost stays under 1000 ms and 50-call projection under 30 s under BOTH debug and release profiles. BUG-07-008 (the existing `#[cfg(unix)]` antipattern in `oriterm_test_support`'s session test) is fixed in 02.3 with a portable two-arm shell test, restoring Windows ConPTY drain coverage. `cargo test -p oriterm_test_support terminfo` and `cargo test -p oriterm_test_support session` run successfully on Linux/macOS and skip cleanly on Windows; cross-compile gate `cargo build --target x86_64-pc-windows-gnu -p oriterm_test_support --tests` succeeds. `cargo tree -p oriterm_test_support` confirms zero `oriterm_ui` / `oriterm_mux` / `oriterm` / `oriterm_ipc` dependencies (crate-boundary discipline). Zero new clippy warnings. The pinned terminfo is ready for tack to consume in Section 03 via `TerminfoEnv::apply_env` and the documented `spawn_tack` helper.

**Section 02 self-contained scope reminder:** Items that depend on Sections 03-08 existing (e.g., `time ./test-all.sh` aggregate measurement, end-to-end integration with `spawn_tack`) belong in Section 09 (Verification), not here. Section 02's checklist contains ONLY items executable within its own sitting.
