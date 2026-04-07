---
section: "02"
title: "Terminfo Provisioning"
status: not-started
reviewed: false
goal: "Create extra/ori_term.info (a pinned terminfo source derived from xterm-256color), then add a runtime TerminfoEnv helper in oriterm_test_support that compiles ori_term.info via tic into a temp directory and exposes (TERM=ori_term, TERMINFO_DIRS=...) for child processes. Tack-driven tests in Sections 03+ get a controlled, reproducible terminfo entry instead of validating whatever the host's ncurses database happens to say."
success_criteria:
  - "`extra/ori_term.info` exists and declares the capabilities ori_term actually implements (am, bce, ccc, km, mir, msgr, xenl, colors#256, pairs#65536, sgr/setaf/setab/cup/csr/smcup/rmcup/smkx/rmkx, kf1-kf63, etc.)"
  - "`tic -x extra/ori_term.info` compiles successfully against the host ncurses with zero errors and zero `tic: WARNING` lines (warnings are the gate)"
  - "`infocmp -A <compiled-dir> ori_term` round-trips back to a source file equivalent to the input"
  - "`oriterm_test_support::TerminfoEnv::compile()` invokes `tic -x -o <tempdir>` at runtime and returns a struct exposing `term_name() -> &str` and `terminfo_dirs() -> &Path`"
  - "`TerminfoEnv` is `Send` and the temp directory is cleaned up via `Drop` (RAII — uses `tempfile::TempDir`)"
  - "`tic_available()` and `infocmp_available()` runtime checks exist alongside `tack_available()` (Section 03)"
  - "Unit test `terminfo_env_compiles_ori_term` constructs a `TerminfoEnv`, asserts the compiled terminfo dir contains `o/ori_term`, and that `infocmp -A <dir> ori_term` produces output containing `am` and `colors#256`"
  - "All Section 02 tests skip cleanly when `tic` is unavailable (Windows native, restricted CI environments)"
  - "Satisfies mission criteria: 'extra/ori_term.info terminfo source exists', '`tic` compiles ori_term.info successfully', 'tests use pinned `TERM=ori_term` + `TERMINFO_DIRS`'"
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
    status: not-started
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

**Status:** Not Started
**Goal:** Stop letting tack tests validate whatever `xterm-256color` happens to declare on the host. Create a pinned `extra/ori_term.info` that documents EXACTLY which capabilities ori_term implements, then compile it at test runtime via `tic` into a temp directory, then expose `(TERM=ori_term, TERMINFO_DIRS=<tempdir>)` env-vars for `tack` (and any other terminfo-aware tool) to consume. After this section, every tack scenario in Sections 03-08 runs against ori_term's documented terminfo, not the host's xterm-256color guess.

**Success Criteria:**

- [ ] `extra/ori_term.info` exists, parseable by `tic -x` with zero warnings
- [ ] `extra/ori_term.info` derives from `xterm-256color` via `use=xterm-256color,` and overrides only the capabilities ori_term implements differently (or wishes to pin explicitly)
- [ ] `oriterm_test_support::terminfo::TerminfoEnv::compile()` produces a working terminfo directory consumable by `tack`, `infocmp`, and any ncurses-linked tool
- [ ] `TerminfoEnv` implements `Drop` cleanly via `tempfile::TempDir`
- [ ] `tic_available()` and `infocmp_available()` helper functions exist in `oriterm_test_support` next to `tack_available()` (Section 03)
- [ ] `cargo test -p oriterm_test_support terminfo` — internal unit tests pass on Linux/macOS, skip cleanly on Windows
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green
- [ ] Satisfies mission criteria #5, #6

**Context:** Without a controlled terminfo entry, tack reads `$TERM=xterm-256color` from the host system terminfo database (`/usr/share/terminfo/x/xterm-256color` on Linux). That entry was authored by someone else, drifts over time, and doesn't reflect ori_term's actual capabilities. Tests built on the host entry validate the host's idea of "xterm-256color", not ori_term's implementation. The result: ori_term might claim to implement a capability the host terminfo says exists, or fail a test because the host terminfo declares a feature ori_term doesn't yet support.

The fix is the convention used by every serious terminal: ship a `.info` source file in the project, compile it with `tic` at install time (or test time), and use the compiled entry via `TERMINFO_DIRS`. Alacritty does this with `extra/alacritty.info` (112 lines, compiled via `sudo tic -xe alacritty,alacritty-direct extra/alacritty.info`). WezTerm does it with `termwiz/data/wezterm.terminfo` (90 lines, compiled via `tic -x -o ~/.terminfo wezterm.terminfo`). Ghostty generates the source from Zig code (`src/terminfo/ghostty.zig`) at build time. We adopt the Alacritty pattern: a hand-authored `.info` file under `extra/` with a runtime tic invocation from the test driver.

**Reference implementations:**
- **Alacritty** `extra/alacritty.info` (lines 1-112): canonical Rust-terminal terminfo source. Two entries (`alacritty` legacy 256-color, `alacritty-direct` truecolor). Builds on a `alacritty+common` fragment via `use=`. We adopt the same `use=xterm-256color,` parent-and-overrides pattern.
- **Alacritty** install instruction: `sudo tic -xe alacritty,alacritty-direct extra/alacritty.info` — `-x` enables extension capabilities (Tc/Ms/Ss/Smulx/Sync), `-e` selects entries to compile.
- **WezTerm** `termwiz/data/wezterm.terminfo` (line 17 onward): the modern extension capability declarations (`Tc, hs, Su, Cr, Cs, Ms, Se, Ss, Smulx, Sync, Setulc, Smol`). These are the boilerplate any modern terminal needs.
- **Ghostty** `src/terminfo/ghostty.zig:1-40`: capability list as Zig data — useful as a cross-reference for what a modern terminal *should* declare.
- **ncurses** `term(5)` man page: terminfo source format (capability codes, escape sequences, parameter substitution).
- **ncurses** `tic(1)` man page: `-x` (extension caps), `-o <dir>` (output dir), `-c` (validate without compiling). The `-x` flag is REQUIRED — without it, modern caps like `Tc` produce `tic: WARNING` lines and may be silently dropped.

**Depends on:** Section 01 — `oriterm_test_support` crate must exist before `TerminfoEnv` can be added to it.

---

## 02.1 Author extra/ori_term.info

**File(s):** `extra/ori_term.info` (NEW FILE — `extra/` directory does not yet exist and will be created here)

The terminfo source is the canonical declaration of "what ori_term claims to be". It should match what ori_term actually IMPLEMENTS today, not what we'd like it to be — Section 09 verification will compare claims against reality.

- [ ] Create directory `/home/eric/projects/ori_term/extra/`. This directory does not exist — verify before creating, then `mkdir extra`.

- [ ] Create `extra/ori_term.info` with two entries: `ori_term` (256-color) and `ori_term-direct` (truecolor). Match Alacritty's two-entry pattern. The file structure:
  ```
  # ori_term terminfo entry.
  #
  # Compile with:
  #   tic -x -o ~/.terminfo extra/ori_term.info
  #
  # Or for tests, the runtime helper at
  # crates/oriterm_test_support/src/terminfo.rs:TerminfoEnv::compile()
  # invokes tic -x -o <tempdir> against this file, then sets
  # TERM=ori_term + TERMINFO_DIRS=<tempdir> for the child process.
  #
  # The -x flag is REQUIRED — modern extension capabilities (Tc, Ms,
  # Ss, Se, Smulx, Sync) are user-defined and tic warns + drops them
  # without -x. See `man tic` for the full -x behavior.

  ori_term|ori_term terminal emulator,
      use=xterm-256color,
      # Overrides and extension caps below override the parent.
      Tc,
      Ms=\E]52;%p1%s;%p2%s\007,
      # ... see capability table below ...

  ori_term-direct|ori_term with direct color indexing,
      use=ori_term,
      RGB,
      colors#0x1000000, pairs#0x10000,
      # ... direct-color setaf/setab from alacritty-direct as reference ...
  ```

  **Capability authoring rules — ground every line in actual ori_term behavior:**

  Before declaring any capability, verify ori_term actually implements it. Sources of truth:
  - `oriterm_core/src/term_handler.rs` — VTE handler implementations (which CSI/OSC/DCS sequences are wired)
  - `oriterm_core/src/grid/` — modes that affect grid behavior (autowrap=`am`, bce=`bce`, origin=`om`, alt-screen=`smcup/rmcup`, mouse=`kmous`)
  - `oriterm/src/key_encoding/` — key sequence encoding (`kf1`-`kf63`, `kcub1`-`kcuu1`, `khome`, `kend`, `kpp`, `knp`, `kdch1`, `kich1`)
  - `oriterm_core/src/palette.rs` — color count (`colors#256` or `colors#0x1000000` for direct-color entry)
  - The `vttest` test results (from Section 01's preserved 198 snapshots) — what menus pass and what they exercise

  When in doubt about a capability, consult `xterm-256color` from the host (`infocmp xterm-256color`) and inherit it via `use=xterm-256color,`. This means ori_term's terminfo starts as an alias of xterm-256color and adds explicit declarations only for the capabilities we want to PIN (so the entry doesn't drift if the host xterm-256color changes) or DENY (capabilities ori_term explicitly does not support — declared with `cap@`).

  **Required base capability set** (drawn from the Alacritty/WezTerm/Ghostty intersection — every modern emulator declares these):
  - Booleans: `am, bce, ccc, km, mir, msgr, xenl, AX, XT, hs` (skip `OTbs` — it's a deprecated obsolete cap)
  - Numbers: `colors#256, cols#80, it#8, lines#24, pairs#0x10000`
  - Cursor movement: `cup, cub, cub1, cuf, cuf1, cud, cud1, cuu, cuu1, hpa, vpa, home, ind, ri, nel, cr, ht`
  - Erase/clear: `ed, el, el1, ech, clear`
  - Insert/delete: `ich, dch, dch1, il, il1, dl, dl1, ich1` (only those ori_term implements)
  - SGR: `bold, dim, sitm, ritm, smul, rmul, smso, rmso, rev, blink, invis, sgr, sgr0, smxx, rmxx`
  - Color: `setaf, setab, op, oc, initc` (skip `setb`/`setf` — deprecated, use `setab`/`setaf`)
  - Screen: `smcup, rmcup, csr, sc, rc, decsc, decrc`
  - Keypad: `smkx, rmkx, kbs, kcub1, kcud1, kcuf1, kcuu1, khome, kend, kpp, knp, kdch1, kich1`
  - Function keys: `kf1` through `kf63` (xterm modifier conventions: F1-F4 = `\EOP/\EOQ/\EOR/\EOS`, F5-F12 = `\E[15~..\E[24~`, modified F1-F12 = `\E[1;{mod}P` etc.)
  - Mouse: `kmous=\E[M`
  - Reports: `u6=\E[%i%d;%dR, u7=\E[6n, u8=\E[?%[;0123456789]c, u9=\E[c`
  - Misc: `bel=^G, flash, civis, cnorm, cvvis, rep, indn, rin`

  **Modern extension capabilities** (require `tic -x`):
  - `Tc` — truecolor support (boolean)
  - `Ms=\E]52;%p1%s;%p2%s\007` — clipboard set/get via OSC 52 (only declare if ori_term implements OSC 52; check `oriterm_core/src/term_handler.rs` for the OSC 52 dispatch — if it's wired, declare `Ms`; if not, omit)
  - `Ss=\E[%p1%d\sq` and `Se=\E[2\sq` — DECSCUSR cursor style
  - `Smulx=\E[4:%p1%dm` — kitty-style underline styles (only if ori_term parses CSI 4:N m; check the SGR handler)
  - `Setulc=\E[58:2::%p1%{65536}%/%d:%p1%{256}%/%{255}%&%d:%p1%{255}%&%d%;m` — underline color (only if ori_term implements colored underlines)
  - `Sync=\E[?2026%?%p1%{1}%-%tl%eh%;` — synchronized output (mode 2026; only if implemented)
  - `XT` — boolean indicating "xterm-style title sequences supported"
  - `AX` — boolean indicating "default fg/bg restore via SGR 39/49 supported"

  **Extension cap discipline:** for EVERY modern extension cap declared, leave a comment line above it pointing to the file:line in oriterm_core where it's implemented. Example:
  ```
      # oriterm_core/src/term/handler/clipboard.rs:42 — OSC 52 clipboard
      Ms=\E]52;%p1%s;%p2%s\007,
  ```
  This makes Section 09 verification mechanical: grep for the comment markers, follow the references, confirm each declared capability has a corresponding implementation site.

- [ ] Compile-check the source as you author it: `tic -c -x extra/ori_term.info` (the `-c` flag validates without writing). The output should be empty — any `tic: WARNING:` line is a gate failure that must be fixed before committing the file.

- [ ] Run `tic -x -o /tmp/ori_term_terminfo extra/ori_term.info` and verify:
  - `/tmp/ori_term_terminfo/o/ori_term` exists
  - `/tmp/ori_term_terminfo/o/ori_term-direct` exists (or symlink to `ori_term`)
  - `infocmp -A /tmp/ori_term_terminfo ori_term` round-trips to a source-form output containing the declared boolean caps and `colors#256`
  - `infocmp -A /tmp/ori_term_terminfo ori_term-direct` round-trips with `colors#0x1000000`

- [ ] Commit `extra/ori_term.info` as a tracked file. **Do NOT** check in any compiled output (`/tmp/ori_term_terminfo/` is throw-away — runtime compilation in 02.2 produces a fresh copy per test run).

- [ ] **Release-packaging scope note:** `extra/ori_term.info` is a source file consumed only by the test harness in this plan. It is NOT bundled into release binaries, and this plan does NOT add install scripts for it. When ori_term ships its first packaged release, a separate plan (installer/packaging) will decide whether to bundle the compiled terminfo into `/usr/share/terminfo/o/ori_term` (Linux/macOS system install) or leave it as an opt-in developer-side file. For this plan, the only consumer is `TerminfoEnv::compile()` at test time; the file lives in the source tree and nowhere else.

---

## 02.2 TerminfoEnv runtime compiler

**File(s):** `crates/oriterm_test_support/src/terminfo.rs` (NEW), `crates/oriterm_test_support/src/lib.rs` (mod declaration), `crates/oriterm_test_support/Cargo.toml` (tempfile dep)

`TerminfoEnv` is the test-time bridge between `extra/ori_term.info` (committed source) and tack/tic-driven test sessions (runtime consumers). Each test that needs a pinned terminfo constructs a `TerminfoEnv`, gets back `TERM` and `TERMINFO_DIRS` strings, and passes those to `PtySession::spawn(...)` via `CommandBuilder::env(...)`.

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

- [ ] Create `crates/oriterm_test_support/src/terminfo.rs`:
  ```rust
  //! Pinned terminfo provisioning for conformance test sessions.
  //!
  //! Compiles `extra/ori_term.info` at test runtime via `tic -x -o <tempdir>`
  //! and exposes the resulting (`TERM`, `TERMINFO_DIRS`) env pair so child
  //! processes (`tack`, `infocmp`, anything ncurses-linked) read ori_term's
  //! pinned terminfo entry instead of the host's `xterm-256color`.

  use std::path::{Path, PathBuf};
  use std::process::{Command, Stdio};

  use tempfile::TempDir;

  /// A compiled terminfo directory for ori_term, plus the env vars to
  /// reach it.
  ///
  /// The terminfo source (`extra/ori_term.info`) is committed in the
  /// repository. At construction, this type:
  ///   1. Locates the `.info` source via `find_source()`
  ///   2. Creates a temp directory via `tempfile::TempDir`
  ///   3. Invokes `tic -x -o <tempdir> extra/ori_term.info` as a subprocess
  ///   4. Verifies the compiled entry exists at `<tempdir>/o/ori_term`
  ///
  /// Drop cleans up the temp directory automatically (RAII via TempDir).
  ///
  /// # Errors / Panics
  ///
  /// - Panics if `tic` is not installed (callers must gate on
  ///   `tic_available()` first).
  /// - Panics if `extra/ori_term.info` cannot be found (search order:
  ///   `CARGO_MANIFEST_DIR`, then walking up to find `extra/`).
  /// - Panics on `tic` non-zero exit (compilation failure — print stderr).
  pub struct TerminfoEnv {
      tempdir: TempDir,
      term: &'static str,
  }

  impl TerminfoEnv {
      /// Compile `extra/ori_term.info` into a fresh temp dir, returning
      /// a handle that exposes (`TERM`, `TERMINFO_DIRS`) for child
      /// processes. Term is `"ori_term"` (the 256-color entry).
      #[must_use]
      pub fn compile() -> Self {
          Self::compile_with_term("ori_term")
      }

      /// Compile and pin to a specific entry name (e.g., `"ori_term-direct"`).
      ///
      /// `term` must be `"ori_term"` or `"ori_term-direct"` — callers
      /// pass a `&'static str` to make typos a compile error.
      #[must_use]
      pub fn compile_with_term(term: &'static str) -> Self {
          assert!(
              term == "ori_term" || term == "ori_term-direct",
              "TerminfoEnv: unknown term {term:?}; expected ori_term or ori_term-direct"
          );
          let source = find_source();
          let tempdir = TempDir::new().expect("create terminfo tempdir");
          let status = Command::new("tic")
              .arg("-x")
              .arg("-o")
              .arg(tempdir.path())
              .arg(&source)
              .stdout(Stdio::piped())
              .stderr(Stdio::piped())
              .output()
              .expect("invoke tic");

          if !status.status.success() {
              panic!(
                  "tic failed (exit {}) compiling {source:?}:\nstdout: {}\nstderr: {}",
                  status.status,
                  String::from_utf8_lossy(&status.stdout),
                  String::from_utf8_lossy(&status.stderr),
              );
          }

          // Verify the compiled entry materialized at <tempdir>/o/<term>.
          // tic stores entries in a single-letter subdir keyed on the
          // first letter of the term name (see `man term`).
          let entry_path = tempdir.path().join("o").join(term);
          assert!(
              entry_path.exists(),
              "tic claimed success but {entry_path:?} does not exist"
          );

          Self { tempdir, term }
      }

      /// The pinned `TERM` value (`ori_term` or `ori_term-direct`).
      #[must_use]
      pub fn term(&self) -> &'static str {
          self.term
      }

      /// The directory `tic` wrote into. Use this as `TERMINFO_DIRS`.
      #[must_use]
      pub fn terminfo_dir(&self) -> &Path {
          self.tempdir.path()
      }

      /// Convenience: returns `(TERM, TERMINFO_DIRS)` ready to feed
      /// `CommandBuilder::env`.
      ///
      /// ```ignore
      /// let env = TerminfoEnv::compile();
      /// let (term, dirs) = env.env_vars();
      /// let mut cmd = CommandBuilder::new("tack");
      /// cmd.env("TERM", term);
      /// cmd.env("TERMINFO_DIRS", dirs);
      /// ```
      #[must_use]
      pub fn env_vars(&self) -> (&'static str, String) {
          (self.term, self.terminfo_dir().to_string_lossy().into_owned())
      }
  }

  /// Locate `extra/ori_term.info` from a workspace member crate.
  ///
  /// Walks up from `CARGO_MANIFEST_DIR` until it finds an `extra/`
  /// directory containing `ori_term.info`. This handles both the
  /// `oriterm_core/tests/...` case (manifest dir is `oriterm_core`)
  /// and the `oriterm/src/...` case (manifest dir is `oriterm`).
  fn find_source() -> PathBuf {
      let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
      let mut cur = manifest.as_path();
      loop {
          let candidate = cur.join("extra").join("ori_term.info");
          if candidate.exists() {
              return candidate;
          }
          let Some(parent) = cur.parent() else {
              panic!(
                  "could not find extra/ori_term.info from CARGO_MANIFEST_DIR={manifest:?} \
                   walking upward — is the file committed?"
              );
          };
          cur = parent;
      }
  }
  ```

- [ ] Add sibling tests at `crates/oriterm_test_support/src/terminfo/tests.rs` (per `.claude/rules/test-organization.md`):

  Note: when adding `terminfo/tests.rs`, restructure `terminfo.rs` into `terminfo/mod.rs` to satisfy the sibling-tests-file convention. The split is cheap — `terminfo/mod.rs` holds everything currently in `terminfo.rs`, and `terminfo/tests.rs` holds the tests.

  ```rust
  use std::process::Command;

  use super::{TerminfoEnv, find_source};
  use crate::tic_available;

  #[test]
  fn find_source_locates_committed_terminfo() {
      // The committed extra/ori_term.info must always be findable from
      // any workspace crate's manifest dir.
      let path = find_source();
      assert!(path.exists(), "find_source returned non-existent path: {path:?}");
      assert!(path.ends_with("extra/ori_term.info"));
  }

  #[test]
  fn terminfo_env_compiles_ori_term() {
      if !tic_available() {
          eprintln!("tic not installed, skipping terminfo_env_compiles_ori_term");
          return;
      }
      let env = TerminfoEnv::compile();
      assert_eq!(env.term(), "ori_term");

      let entry = env.terminfo_dir().join("o").join("ori_term");
      assert!(entry.exists(), "compiled entry missing at {entry:?}");

      // Round-trip via infocmp: -A points to the compiled dir, then we
      // ask infocmp for the ori_term entry and verify a known capability
      // is present in the output.
      let infocmp = Command::new("infocmp")
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
  fn terminfo_env_env_vars_are_consumable_strings() {
      if !tic_available() {
          return;
      }
      let env = TerminfoEnv::compile();
      let (term, dirs) = env.env_vars();
      assert_eq!(term, "ori_term");
      assert!(!dirs.is_empty());
      assert!(std::path::Path::new(&dirs).exists());
  }

  // Negative pins — these tests ensure TerminfoEnv fails loudly when
  // the source is corrupted, tic is invoked on a bogus file, or the
  // caller asks for a non-existent entry. Without these, a silent
  // failure mode would let a corrupted terminfo propagate to every
  // downstream tack test in Sections 03-08.

  #[test]
  #[should_panic(expected = "unknown term")]
  fn terminfo_env_rejects_unknown_term_name() {
      if !tic_available() { return; }
      // Deliberately pass a `&'static str` that is not one of the two
      // declared terms. The compile_with_term assertion guards against
      // typos at the call site; this test pins that guard.
      let _ = TerminfoEnv::compile_with_term("ori_term_bogus");
  }

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

  Add `tempfile = "3"` to `crates/oriterm_test_support/Cargo.toml` `[dev-dependencies]` (it is already in `[dependencies]` from the main module — the dev-deps entry keeps it explicit for the sibling tests). Alternatively, re-use the existing `[dependencies]` entry — no separate dev-dep is strictly required since `tempfile` is already visible.

- [ ] Add `#[cfg(test)] mod tests;` at the bottom of `terminfo/mod.rs`.

- [ ] **Assert `TerminfoEnv: Send` at compile time.** The success criterion in the frontmatter claims `TerminfoEnv` is `Send`. Add a trait-bound assertion at the bottom of `terminfo/mod.rs` so the claim is enforced by the compiler:
  ```rust
  const _: fn() = || {
      fn assert_send<T: Send>() {}
      assert_send::<TerminfoEnv>();
  };
  ```
  `tempfile::TempDir` is `Send` (it's a `PathBuf` + `Fd`), and `&'static str` is `Send`, so the assertion should compile cleanly. If it fails at compile time, the field types have drifted — fix the offending field or drop the claim from the success criterion.

---

## 02.3 tic_available, infocmp_available, and skip discipline

**File(s):** `crates/oriterm_test_support/src/session.rs` (or `availability.rs`)

The runtime check helpers go alongside `vttest_available()` from Section 01. They follow the same pattern.

- [ ] Add to `crates/oriterm_test_support/src/session.rs`:
  ```rust
  /// Check if `tic` (terminfo compiler) is installed.
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

  `tic` and `infocmp` both support `-V` for version (per `man tic`, `man infocmp`). Use `-V` consistently — `--version` is not universal across BSD vs GNU ncurses builds.

- [ ] Re-export from `lib.rs`:
  ```rust
  pub use session::{
      PtyResponder, PtySession,
      tic_available, infocmp_available, vttest_available, tool_available,
  };
  ```
  Section 03 will add `tack_available` to this same list.

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

---

## 02.4 Verify pinned terminfo round-trips through infocmp

**File(s):** `crates/oriterm_test_support/src/terminfo/tests.rs` (additional tests)

The terminfo source is correct only if `tic` round-trips it through `infocmp` cleanly. This subsection is the gate.

- [ ] Add round-trip integrity test:
  ```rust
  #[test]
  fn ori_term_terminfo_round_trips_via_infocmp() {
      if !tic_available() || !infocmp_available() {
          return;
      }
      let env = TerminfoEnv::compile();

      // Decompile ori_term back to source form.
      let infocmp = Command::new("infocmp")
          .arg("-A")
          .arg(env.terminfo_dir())
          .arg("-1")  // one cap per line for stable diff
          .arg("ori_term")
          .output()
          .expect("invoke infocmp");
      assert!(infocmp.status.success());
      let out = String::from_utf8_lossy(&infocmp.stdout);

      // Required boolean caps:
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
  }

  #[test]
  fn ori_term_direct_declares_truecolor() {
      if !tic_available() || !infocmp_available() {
          return;
      }
      let env = TerminfoEnv::compile_with_term("ori_term-direct");
      let infocmp = Command::new("infocmp")
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
  ```

- [ ] **TPR checkpoint** — `/tpr-review` covering 02.1–02.4 implementation work. Catches: missing required capabilities, wrong escape sequence syntax in `extra/ori_term.info`, `tic` warnings that slipped through, `TerminfoEnv` resource leaks (failing to clean up tempdirs on panic paths), platform-specific path bugs (Windows drive letters in `find_source`).

---

## 02.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 02.N Completion Checklist

- [ ] `extra/` directory exists (created in 02.1 — verify with `ls extra/`)
- [ ] `extra/ori_term.info` is a committed file with two entries: `ori_term` and `ori_term-direct`
- [ ] Every modern extension capability declared in `extra/ori_term.info` has a comment line referencing the implementation site (file:line in `oriterm_core/src/term/handler/`)
- [ ] `tic -c -x extra/ori_term.info` produces zero `WARNING` lines (validate-only check passes)
- [ ] `tic -x -o /tmp/test_terminfo extra/ori_term.info` succeeds and creates `/tmp/test_terminfo/o/ori_term`
- [ ] `infocmp -A /tmp/test_terminfo ori_term` round-trips to source and contains `am`, `bce`, `colors#256`, `cup=`, `sgr=`, `setaf=`, `smkx=`
- [ ] `crates/oriterm_test_support/src/terminfo/mod.rs` (or `terminfo.rs` if no submodule needed) exists
- [ ] `TerminfoEnv::compile()`, `TerminfoEnv::compile_with_term()`, `TerminfoEnv::term()`, `TerminfoEnv::terminfo_dir()`, `TerminfoEnv::env_vars()` implemented
- [ ] `TerminfoEnv` cleans up temp dir via `tempfile::TempDir` Drop
- [ ] `tic_available()` and `infocmp_available()` exist next to `vttest_available()` in `crates/oriterm_test_support/src/session.rs`
- [ ] All tests in `crates/oriterm_test_support/src/terminfo/tests.rs` pass on Linux (`cargo test -p oriterm_test_support terminfo`) — includes happy-path tests, round-trip tests, AND negative pins (`terminfo_env_rejects_unknown_term_name`, `terminfo_env_compile_fails_loudly_on_corrupted_source`)
- [ ] All tests skip cleanly when `tic`/`infocmp` are unavailable (no panics, returns Ok)
- [ ] `cargo build -p oriterm_test_support` for `x86_64-pc-windows-gnu` succeeds (cross-compile gate)
- [ ] `tempfile = "3"` added to `crates/oriterm_test_support/Cargo.toml` `[dependencies]`
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green — no new warnings
- [ ] `timeout 150 ./test-all.sh` green
- [ ] Plan annotation cleanup: no temporary scaffolding in any `.rs` or `.info` file
- [ ] All TPR checkpoint findings resolved (see `02.R`)
- [ ] **Plan sync**:
  - [ ] This section's frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference table: Section 02 marked Complete
  - [ ] `00-overview.md` Mission Success Criteria #5, #6 ticked
  - [ ] `index.md` Section 02 status updated
  - [ ] Section 03's `depends_on: ["02"]` confirmed (Section 03 spawns tack with TerminfoEnv)
- [ ] `/tpr-review` final pass clean
- [ ] `/impl-hygiene-review last commit` final pass clean (after TPR)

**Exit Criteria:** `extra/ori_term.info` is committed and parseable by `tic -x` with zero warnings. `oriterm_test_support::TerminfoEnv::compile()` produces a working temp directory containing `o/ori_term` and `o/ori_term-direct`. `infocmp -A <tempdir> ori_term` round-trips to source form containing all required base capabilities. `cargo test -p oriterm_test_support terminfo` runs all tests successfully on Linux/macOS and skips cleanly on Windows. Zero new clippy warnings. The pinned terminfo is ready for tack to consume in Section 03.
