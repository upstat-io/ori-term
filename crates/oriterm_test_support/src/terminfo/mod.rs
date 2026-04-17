//! Pinned terminfo provisioning for conformance test sessions.
//!
//! Compiles `extra/ori_term.info` at test runtime via `tic -x -o <tempdir>`
//! and applies the resulting (`TERM`, `TERMINFO`, `TERMINFO_DIRS`) env
//! triple to a `CommandBuilder` so child processes (`tack`, `infocmp`,
//! anything ncurses-linked) read `ori_term`'s pinned terminfo entry
//! instead of the host's `xterm-256color`.
//!
//! The terminfo source is embedded at compile time via `include_str!`.
//! If `extra/ori_term.info` is missing or unreadable, this crate fails
//! to compile — the dependency between the test-support crate and the
//! committed source file is enforced at build time, not runtime. There
//! is no filesystem discovery, no repo-root walk, no env-var lookup.
//!
//! [`TerminfoEnv::compile`] requires `tic` and never forces callers
//! to gate on [`crate::infocmp_available`] — the constructor still
//! runs when only `tic` is present. The post-tic sanity check is a
//! best-effort `infocmp -A <tempdir> <entry>` probe that is skipped
//! when `infocmp` is not installed, so the hard dependency stays
//! tic-only. `infocmp` (via ncurses) is the only portable probe that
//! works across both the directory and hashed-db backends; a
//! `Path::exists` probe on `<tempdir>/<bucket>/<entry>` would panic
//! spuriously on hosts whose ncurses was built with `--with-hashed-db`
//! (e.g. macOS's default `tic`). The full `infocmp -A` round-trip
//! with content assertions lives in the 02.4 test suite — this
//! probe only asks "can infocmp see the entry we asked tic to
//! write?" and leaves structural assertions to 02.4.

use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};

use portable_pty::CommandBuilder;
use tempfile::TempDir;

/// Embedded `extra/ori_term.info` source, captured at compile time.
///
/// The path is relative to `CARGO_MANIFEST_DIR` — i.e. the
/// `oriterm_test_support` crate root, which sits at
/// `crates/oriterm_test_support/`. Two levels up is the workspace
/// root, where `extra/ori_term.info` lives.
pub(crate) const ORI_TERM_INFO: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../extra/ori_term.info"
));

/// Which terminfo entry to pin a [`TerminfoEnv`] to.
///
/// Compile-time exhaustivity: adding a third variant requires
/// updating the `match` in [`TerminfoEnv::compile_with_variant`] AND
/// the [`Self::entry_name`] mapping below — the compiler enforces
/// both sync points. This is the impl-hygiene "Stringly-typed
/// internals" rule (finite valid values → enum); a `&'static str`
/// parameter would only constrain lifetime, not value.
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

/// A compiled terminfo directory for `ori_term` plus the env-var
/// machinery to steer child processes to it.
///
/// At construction, this type:
///   1. Creates a temp directory via [`tempfile::TempDir`].
///   2. Writes the embedded [`ORI_TERM_INFO`] source to a scratch
///      file inside the tempdir.
///   3. Invokes `tic -x -o <tempdir> <scratch>` as a subprocess.
///   4. Best-effort sanity check via `infocmp -A <tempdir> <entry>`
///      that the entry we asked `tic` to write is visible through
///      ncurses. Skipped when `infocmp` is not installed so the
///      hard dependency remains `tic` only. This replaces the
///      previous `Path::exists` probe on `<tempdir>/<bucket>/<entry>`,
///      which only worked on the directory backend and panicked on
///      hashed-db hosts (e.g. macOS). The full `infocmp -A` content
///      round-trip lives in the 02.4 test suite, not here.
///
/// Drop cleans up the temp directory automatically (RAII via
/// [`TempDir`]).
///
/// # Errors / Panics
///
/// - Panics if `tic` is not installed (callers must gate on
///   [`crate::tic_available`] first).
/// - Panics if `tic` exits non-zero (compilation failure — prints
///   stderr).
/// - Panics if `infocmp` is installed AND the post-tic sanity probe
///   fails to locate the entry (proves the compile actually wrote
///   a readable entry, not just exited 0 silently). If `infocmp` is
///   not installed the probe is skipped and we trust `tic`'s exit
///   code alone.
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
    /// [`TerminfoVariant::entry_name`]. Callers cannot pass an
    /// invalid terminfo name; there is no string surface to typo
    /// against.
    #[must_use]
    pub fn compile_with_variant(variant: TerminfoVariant) -> Self {
        let tempdir = TempDir::new().expect("create terminfo tempdir");
        let source_path = write_embedded_source(tempdir.path());
        run_tic(&source_path, tempdir.path());
        assert_entry_written(tempdir.path(), variant);
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
    pub fn terminfo_dir(&self) -> &std::path::Path {
        self.tempdir.path()
    }

    /// The (name, value) env-var triple this `TerminfoEnv` advertises.
    ///
    /// **SSOT for the env-var contract.** [`Self::apply_env`] (the
    /// public wrapper for [`portable_pty::CommandBuilder`]) and the
    /// 02.4 `child_process_with_apply_env_reads_pinned_terminfo`
    /// integrity test (which spawns [`std::process::Command`], not
    /// `CommandBuilder`) BOTH consume this method. There is no
    /// other place that knows which env vars get set. Adding a
    /// fourth env var tomorrow is a one-line edit here; every
    /// consumer follows automatically.
    ///
    /// `pub(crate)` because the type-safe public API surface is
    /// [`Self::apply_env`] — external callers should never iterate
    /// the array. The 02.4 integrity test lives in the same crate,
    /// so it sees this helper.
    pub(crate) fn env_pairs(&self) -> [(&'static str, String); 3] {
        let dir = self.terminfo_dir().to_string_lossy().into_owned();
        [
            ("TERM", self.variant.entry_name().to_owned()),
            ("TERMINFO", dir.clone()),
            ("TERMINFO_DIRS", dir),
        ]
    }

    /// Apply `TERM`, `TERMINFO`, and `TERMINFO_DIRS` to a
    /// [`CommandBuilder`] so the spawned child reads the pinned
    /// terminfo entry instead of the host database.
    ///
    /// Some ncurses consumers honor only `TERMINFO` (singular);
    /// others honor only `TERMINFO_DIRS` (plural). Setting both via
    /// this single wrapper ensures ncurses lookup never falls back
    /// to the host database regardless of which variable the
    /// consumer consults first. The wrapper exists so consumers
    /// (Section 03's `spawn_tack`, the 02.4 child-process integrity
    /// test, any future caller) never need to know the (name,
    /// value) tuple shape — if `TerminfoEnv` learns to set a fourth
    /// env var tomorrow, only [`Self::env_pairs`] changes.
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

/// Runtime probe: does the host's `infocmp` honor the
/// `$TERMINFO` / `$TERMINFO_DIRS` env-var precedence ncurses
/// applications rely on?
///
/// On Linux/macOS with native ncurses, the answer is yes. On
/// Windows where `infocmp` is shipped by MSYS2 / Cygwin / WSL,
/// the answer depends on which `infocmp` the test runner picked
/// up — some MSYS variants ignore the env vars entirely and
/// consult only a hardcoded terminfo location.
///
/// The probe compiles a fresh [`TerminfoEnv`] (which writes our
/// `ori_term` entry into a tempdir), sets the env-var triple via
/// `apply_env`, runs `infocmp ori_term` with no `-A`, and reports
/// whether the child exited successfully. Successful exit means
/// the child resolved `ori_term` via env-var precedence and read
/// our tempdir; non-zero exit means the child fell back to the
/// system terminfo location and could not find `ori_term` there.
///
/// Used by `child_process_with_apply_env_reads_pinned_terminfo`
/// to skip cleanly on hosts whose `infocmp` lacks env-var
/// precedence support, instead of failing with an opaque
/// assertion error.
///
/// Returns `false` if `tic` or `infocmp` is missing on the host
/// (either tool is required for the probe).
#[must_use]
pub fn infocmp_respects_terminfo_env() -> bool {
    if !crate::tic_available() || !crate::infocmp_available() {
        return false;
    }
    let env = TerminfoEnv::compile();
    let mut cmd = Command::new("infocmp");
    cmd.arg(env.term());
    for (name, value) in env.env_pairs() {
        cmd.env(name, value);
    }
    cmd.env_remove("TERMCAP");
    cmd.output().is_ok_and(|out| out.status.success())
}

/// Write the embedded terminfo source to a scratch file inside
/// `tempdir` and return its path. Panics on I/O failure (test-only
/// code; the surrounding `compile_with_variant` is documented to
/// panic on every error path).
fn write_embedded_source(tempdir: &std::path::Path) -> std::path::PathBuf {
    let source_path = tempdir.join("ori_term.info");
    let mut f = std::fs::File::create(&source_path).expect("create embedded terminfo source file");
    f.write_all(ORI_TERM_INFO.as_bytes())
        .expect("write embedded terminfo source");
    source_path
}

/// Invoke `tic -x -o <dest> <source>` and panic on non-zero exit,
/// emitting captured stdout and stderr in the panic message so the
/// failing test points directly at the broken terminfo source.
fn run_tic(source: &std::path::Path, dest: &std::path::Path) {
    let tic_out = Command::new("tic")
        .arg("-x")
        .arg("-o")
        .arg(dest)
        .arg(source)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("invoke tic");
    assert!(
        tic_out.status.success(),
        "tic failed (exit {}):\nstdout: {}\nstderr: {}",
        tic_out.status,
        String::from_utf8_lossy(&tic_out.stdout),
        String::from_utf8_lossy(&tic_out.stderr),
    );
}

/// Sanity-check that `tic` actually wrote the variant's entry to
/// `tempdir` by asking ncurses via `infocmp -A <tempdir> <entry>`.
///
/// This replaces the previous directory-backend `Path::exists` probe
/// on `<tempdir>/<first-letter>/<entry-name>`, which panicked on
/// hosts whose ncurses was built with `--with-hashed-db` (macOS's
/// default `tic` in CI). `infocmp -A` consults the database through
/// ncurses, which handles both the directory and hashed-db backends
/// transparently — it is the only portable "did tic actually write
/// the entry?" probe.
///
/// The probe is gated on [`crate::infocmp_available`] and silently
/// returns when `infocmp` is not installed, so the constructor's
/// hard tool dependency stays `tic` only (callers still gate on
/// [`crate::tic_available`], not on `infocmp_available`). In that
/// degraded case we trust `tic`'s exit code alone — downstream
/// tests that actually consume the compiled entry (Section 02.4
/// and later) will catch a silent-success bug through their own
/// assertions.
///
/// The full structural round-trip (asserting specific caps, colors,
/// extension caps) lives in the 02.4 test suite, not here. This
/// probe only asks "is the entry readable?", not "is its content
/// correct?".
fn assert_entry_written(tempdir: &std::path::Path, variant: TerminfoVariant) {
    if !crate::infocmp_available() {
        return;
    }
    let entry_name = variant.entry_name();
    let out = Command::new("infocmp")
        .arg("-A")
        .arg(tempdir)
        .arg(entry_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("invoke infocmp");
    assert!(
        out.status.success(),
        "tic claimed success but infocmp -A {} {} cannot locate the entry — \
         tic may have silently produced an empty database or written to an \
         unexpected location:\nstdout: {}\nstderr: {}",
        tempdir.display(),
        entry_name,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

// Compile-time `Send` assertion. The success criterion in the
// section frontmatter claims `TerminfoEnv: Send`. `tempfile::TempDir`
// is `Send` and `TerminfoVariant` is `Copy`, so the assertion should
// compile cleanly. If it ever fails, a field type drifted — fix the
// offending field or drop the claim from the success criterion. We
// do NOT add a `Sync` bound speculatively; expanding it now would
// block future field additions (e.g., an `RwLock<Stats>`) for no
// current benefit.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<TerminfoEnv>();
};

/// Parse a full `infocmp` dump into a cap-name → raw-value map.
///
/// Invokes `infocmp -x -A <env_dir> -1 <term>` once and parses every
/// cap declaration into a [`HashMap`]. Boolean caps (e.g., `am,`) map
/// to an empty string. Numeric caps (e.g., `colors#256,`) map to the
/// numeric string. String caps (e.g., `kf1=\EOP,`) map to the encoded
/// value string (still terminfo-encoded — call [`decode_terminfo_string`]
/// on the values as needed).
///
/// Returns `None` if `infocmp` is not available or exits non-zero.
/// Callers must gate on [`crate::infocmp_available`] first.
///
/// This is the preferred entry point for tests that validate multiple
/// caps — call once, look up many.
#[must_use]
pub fn infocmp_dump(env: &TerminfoEnv, term: &str) -> Option<HashMap<String, String>> {
    let output = Command::new("infocmp")
        .arg("-x")
        .arg("-A")
        .arg(env.terminfo_dir())
        .arg("-1")
        .arg(term)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut map = HashMap::new();
    for line in text.lines() {
        // Skip entry header lines (they start at column 0).
        if !line.starts_with(|c: char| c.is_whitespace()) {
            continue;
        }
        let trimmed = line.trim_start();
        // Strip trailing comma (terminfo entry separator).
        let trimmed = trimmed.trim_end_matches(',');
        if trimmed.is_empty() {
            continue;
        }
        // String cap: name=value
        if let Some((name, value)) = trimmed.split_once('=') {
            map.insert(name.to_string(), value.to_string());
        // Numeric cap: name#value
        } else if let Some((name, value)) = trimmed.split_once('#') {
            map.insert(name.to_string(), value.to_string());
        // Boolean cap: just the name
        } else {
            map.insert(trimmed.to_string(), String::new());
        }
    }
    Some(map)
}

/// Extract a single capability's value from a compiled terminfo entry.
///
/// Convenience wrapper around [`infocmp_dump`] for callers that only
/// need one cap. For tests that validate many caps, prefer calling
/// `infocmp_dump` once and using `.get()` on the returned map.
///
/// Returns `None` if the cap is not declared or `infocmp` is unavailable.
#[must_use]
pub fn infocmp_query(env: &TerminfoEnv, term: &str, cap: &str) -> Option<String> {
    infocmp_dump(env, term)?.get(cap).cloned()
}

/// Decode terminfo escape syntax to raw bytes.
///
/// Terminfo encodes ESC as `\E`, CR as `\r`, etc. [`infocmp_dump`]
/// returns these encoded forms — this function converts them back to
/// the raw byte sequences that `encode_key` would actually produce.
///
/// Handles: `\E` (ESC), `\e` (ESC), `\r` (CR), `\n` (LF), `\t` (TAB),
/// `\b` (BS), `\f` (FF), `\\` (backslash), `\s` (space), `\,` (comma),
/// `\:` (colon), `\0` (NUL), `\NNN` (octal), `^X` (control char).
///
/// Does NOT handle parameterized strings (`%p1%d` etc.) — those are for
/// `cup`/`csr`/`setaf`, not keyboard caps.
#[must_use]
pub fn decode_terminfo_string(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('E' | 'e') => out.push(0x1b),
                Some('r') => out.push(b'\r'),
                Some('n') => out.push(b'\n'),
                Some('t') => out.push(b'\t'),
                Some('b') => out.push(0x08),
                Some('f') => out.push(0x0c),
                Some('\\') | None => out.push(b'\\'),
                Some('s') => out.push(b' '),
                Some(',') => out.push(b','),
                Some(':') => out.push(b':'),
                Some(other) if other.is_ascii_digit() => {
                    // Octal (up to 3 digits).
                    let mut n = (other as u8) - b'0';
                    for _ in 0..2 {
                        if let Some(d) = chars.peek().filter(|c| c.is_ascii_digit()) {
                            n = n * 8 + (*d as u8 - b'0');
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    out.push(n);
                }
                Some(other) => {
                    // Unknown escape — push verbatim so test diffs are clear.
                    out.push(b'\\');
                    out.extend(other.to_string().bytes());
                }
            }
        } else if c == '^' {
            // Control char: ^X means Ctrl-X.
            if let Some(letter) = chars.next() {
                let upper = letter.to_ascii_uppercase();
                if ('@'..='_').contains(&upper) {
                    out.push((upper as u8) - b'@');
                } else if upper == '?' {
                    out.push(0x7f); // DEL
                } else {
                    out.push(b'^');
                    out.extend(letter.to_string().bytes());
                }
            } else {
                out.push(b'^');
            }
        } else {
            // Regular char (ASCII expected for keyboard caps).
            out.extend(c.to_string().bytes());
        }
    }
    out
}

#[cfg(test)]
mod tests;
