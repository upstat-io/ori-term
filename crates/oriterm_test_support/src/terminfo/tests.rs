use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

use portable_pty::CommandBuilder;

use super::{
    ORI_TERM_INFO, TerminfoEnv, TerminfoVariant, decode_terminfo_string, infocmp_dump,
    infocmp_query, infocmp_respects_terminfo_env,
};
use crate::{infocmp_available, tic_available};

/// SSOT for the 02.4 round-trip-suite skip gate.
///
/// Every round-trip test — `ori_term_terminfo_round_trips_via_infocmp`,
/// `ori_term_direct_declares_truecolor`,
/// `child_process_with_apply_env_reads_pinned_terminfo`, and
/// `terminfo_env_compiles_ori_term` — short-circuits via `if
/// round_trip_gate_closed() { return; }`. The gate is closed when
/// either `tic` or `infocmp` is missing: the suite needs both.
/// Having one helper means a cap regression touches one line, not
/// four sprinkled `if !tic_available() || !infocmp_available()`
/// copies.
fn round_trip_gate_closed() -> bool {
    round_trip_gate_closed_for(tic_available(), infocmp_available())
}

/// Pure form of [`round_trip_gate_closed`] — exposed so
/// `round_trip_gate_semantics_pinned` can pin the 2x2 truth table
/// without depending on whether the real tools are installed on
/// the host.
fn round_trip_gate_closed_for(tic_ok: bool, infocmp_ok: bool) -> bool {
    !tic_ok || !infocmp_ok
}

#[test]
fn embedded_terminfo_source_is_nonempty() {
    // The committed extra/ori_term.info is embedded at compile time
    // via include_str!. If the file is missing the build fails, so
    // this test simply pins the expectation that the source is
    // substantive.
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
    assert_eq!(
        TerminfoVariant::OriTermDirect.entry_name(),
        "ori_term-direct"
    );
    assert_ne!(
        TerminfoVariant::OriTerm.entry_name(),
        TerminfoVariant::OriTermDirect.entry_name()
    );
}

#[test]
fn terminfo_env_compiles_ori_term() {
    // This test gates on BOTH tic AND infocmp because it exercises
    // the round-trip in addition to the compile. The bare compile
    // path is exercised by `terminfo_env_drop_cleans_temp_dir`
    // below, which gates only on tic.
    if round_trip_gate_closed() {
        return;
    }
    let env = TerminfoEnv::compile();
    assert_eq!(env.term(), "ori_term");
    assert_eq!(env.variant(), TerminfoVariant::OriTerm);

    // Portable success check — use infocmp, not hardcoded filesystem
    // layout. This works across ncurses directory and hashed-db
    // backends; asserting `<tempdir>/o/ori_term` would only work on
    // the directory backend.
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
    assert!(
        out.contains("am"),
        "expected 'am' boolean in infocmp output:\n{out}"
    );
    assert!(
        out.contains("colors#256") || out.contains("colors#0x100"),
        "expected colors#256 in infocmp output:\n{out}"
    );
}

#[test]
fn terminfo_env_drop_cleans_temp_dir() {
    // Pure-tic gate — no infocmp dependency. Proves Drop on the
    // bare compile path works without dragging infocmp into the
    // gate.
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
    // SSOT semantic pin: `env_pairs()` is the canonical (name,
    // value) triple that BOTH `apply_env(&mut CommandBuilder)` AND
    // the 02.4 child-process integrity test consume. We assert
    // here that:
    //   1. Exactly three env vars are set (TERM, TERMINFO,
    //      TERMINFO_DIRS).
    //   2. TERM matches the pinned variant entry name.
    //   3. TERMINFO and TERMINFO_DIRS BOTH point at the compiled
    //      tempdir (some ncurses consumers honor only one of the
    //      two).
    //   4. The three names are distinct (catches a copy-paste bug
    //      where `TERMINFO_DIRS` accidentally became `TERMINFO`).
    //
    // We cannot read env back from `CommandBuilder` (portable-pty
    // does not expose accessors), so the unit-test scope is the
    // SSOT itself. The end-to-end behavioral pin — proving the env
    // triple actually steers a real child — lives in 02.4's
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
    // TERMINFO" copy-paste regression that no integration test
    // would catch (the host inheritance would silently re-engage).
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 3, "env var names must be distinct: {names:?}");

    // Value pins.
    let term = pairs
        .iter()
        .find(|(n, _)| *n == "TERM")
        .map(|(_, v)| v.as_str());
    let terminfo = pairs
        .iter()
        .find(|(n, _)| *n == "TERMINFO")
        .map(|(_, v)| v.as_str());
    let terminfo_dirs = pairs
        .iter()
        .find(|(n, _)| *n == "TERMINFO_DIRS")
        .map(|(_, v)| v.as_str());
    assert_eq!(term, Some("ori_term"));
    assert_eq!(
        terminfo,
        Some(env.terminfo_dir().to_string_lossy().as_ref())
    );
    assert_eq!(
        terminfo_dirs,
        Some(env.terminfo_dir().to_string_lossy().as_ref())
    );

    // Smoke-test the public wrapper too — proves `apply_env` does
    // not panic and returns cleanly when given a real
    // CommandBuilder.
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
        // env drops at end of loop iteration → tempdir cleaned up
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

// Negative pin — ensures TerminfoEnv would fail loudly if the
// committed source were ever corrupted. The typo-safe
// `TerminfoVariant` enum eliminates the "unknown term name" negative
// pin (it would not even compile).

#[test]
fn terminfo_env_compile_fails_loudly_on_corrupted_source() {
    // Hand-synthesized fatal terminfo source — caps written without
    // a preceding entry header. ncurses tic reports
    // "Separator inconsistent with syntax" and exits non-zero (this
    // is the actual fatal-error path; unknown caps and bad headers
    // only emit warnings under ncurses 6.4 but still exit 0). We
    // call `tic -c -x <tempfile>` directly (bypassing
    // TerminfoEnv::compile which uses the committed
    // extra/ori_term.info) and assert that tic reports a non-zero
    // exit. This proves the panic-on-tic-failure path inside
    // TerminfoEnv::compile would trigger if someone committed
    // garbage into extra/ori_term.info.
    if !tic_available() {
        return;
    }
    let mut f = tempfile::NamedTempFile::new().expect("tempfile");
    // Entry body with no header — guaranteed fatal under tic.
    writeln!(f, "    am, bce,").expect("write");
    let path = f.path();
    let out = Command::new("tic")
        .arg("-c")
        .arg("-x")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("invoke tic");
    assert!(
        !out.status.success(),
        "tic must report failure on corrupted source; stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

// 02.4 round-trip suite — gates on BOTH tic AND infocmp.
//
// These tests exercise the infocmp round-trip over the committed
// `extra/ori_term.info`, proving that what `tic` compiles round-
// trips cleanly back through `infocmp -A`. Keeping the round-trip
// here (not inside `TerminfoEnv::compile`) is what lets the
// constructor stay pure-`tic` and gate only on `tic_available()`.

#[test]
fn tic_validate_source_has_zero_unexpected_warnings() {
    // tic -c -x gate: compile-check the embedded source without
    // writing output and assert stderr is either empty OR contains
    // only the known ncurses 6.x `%; without %? in Setulc` false
    // positive (one line per entry — ori_term, ori_term-direct,
    // ori_term+common). Any other `tic:` message is a gate failure.
    if !tic_available() {
        return;
    }
    let mut f = tempfile::NamedTempFile::new().expect("tempfile");
    f.write_all(ORI_TERM_INFO.as_bytes()).expect("write");
    let out = Command::new("tic")
        .arg("-c")
        .arg("-x")
        .arg(f.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("invoke tic");
    assert!(
        out.status.success(),
        "tic -c -x failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    for line in stderr.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Known ncurses 6.x false positive: the %;/%? balance
        // checker nags about `Setulc` even though the cap compiles
        // correctly. Tolerate this specific line only.
        if trimmed.contains("%; without %? in Setulc") {
            continue;
        }
        panic!("unexpected tic -c -x stderr line: {line}\nfull stderr:\n{stderr}");
    }
}

#[test]
fn ori_term_terminfo_round_trips_via_infocmp() {
    // Canonical round-trip: compile via TerminfoEnv::compile, then
    // decompile via `infocmp -A <dir> -1 ori_term` and assert every
    // capability class ori_term is supposed to declare is present.
    // Using `-1` (one cap per line) gives stable output regardless
    // of terminal width.
    if round_trip_gate_closed() {
        return;
    }
    let env = TerminfoEnv::compile();

    // `-x` is REQUIRED here — extension caps (`Tc`, `Smulx`,
    // `Setulc`, `Sync`, `BD`, `BE`, `PS`, `PE`, `kxIN`, `kxOUT`,
    // `Cr`, `Cs`, `Ms`, `Ss`, `Se`) only appear under
    // `infocmp -x`. Without `-x`, the assertions below for
    // `BD=`, `BE=`, `PS=`, `PE=`, `kxIN=`, `kxOUT=` would fail
    // even though the caps are correctly compiled into the entry.
    let infocmp = Command::new("infocmp")
        .arg("-x")
        .arg("-A")
        .arg(env.terminfo_dir())
        .arg("-1")
        .arg("ori_term")
        .output()
        .expect("invoke infocmp");
    assert!(
        infocmp.status.success(),
        "infocmp -x -A failed: {}",
        String::from_utf8_lossy(&infocmp.stderr)
    );
    let out = String::from_utf8_lossy(&infocmp.stdout);

    // Required boolean caps — `-1` output places each boolean on
    // its own line as `\t<name>,`. Match on the tab-prefixed form
    // to avoid false positives like `am` matching inside a string
    // cap parameter.
    for cap in ["am", "bce", "ccc", "km", "mir", "msgr", "xenl"] {
        let needle = format!("\t{cap},");
        assert!(
            out.contains(&needle),
            "expected boolean cap {cap:?} in ori_term terminfo, got:\n{out}"
        );
    }

    // Required numeric cap.
    assert!(
        out.contains("colors#256") || out.contains("colors#0x100"),
        "expected colors#256 in ori_term terminfo, got:\n{out}"
    );

    // Required string caps (just check the names appear; values
    // are long parameterized strings).
    for cap in ["cup=", "sgr=", "setaf=", "setab=", "smkx=", "rmkx="] {
        assert!(
            out.contains(cap),
            "expected string cap {cap:?} in ori_term terminfo"
        );
    }

    // Charset capabilities — verify smacs/rmacs/acsc are declared
    // (backs the DEC special graphics handling in
    // oriterm_core/src/term/charset/mod.rs).
    for cap in ["acsc=", "smacs=", "rmacs="] {
        assert!(
            out.contains(cap),
            "expected charset cap {cap:?} in ori_term terminfo"
        );
    }

    // Bracketed paste and focus-event extension caps.
    for cap in ["BD=", "BE=", "PS=", "PE=", "kxIN=", "kxOUT="] {
        assert!(
            out.contains(cap),
            "expected extension cap {cap:?} in ori_term terminfo"
        );
    }

    // Function keys kf1 through kf63 — legacy xterm modifier
    // convention; kf13-kf63 are the modified F1-F12 variants.
    for n in 1..=63 {
        let cap = format!("kf{n}=");
        assert!(
            out.contains(&cap),
            "expected {cap} in ori_term terminfo, got:\n{out}"
        );
    }

    // REP capability — verifies the CSI b dispatch at
    // crates/vte/src/ansi/dispatch/csi.rs:60 is advertised.
    assert!(out.contains("rep="), "expected rep= in ori_term terminfo");
}

#[test]
fn ori_term_direct_declares_truecolor() {
    // Direct-entry round-trip: the truecolor variant must declare
    // either `RGB` or `Tc` (these are the actual capability markers
    // ncurses consumers query at runtime to decide whether to emit
    // 24-bit SGR sequences) and a colors count consistent with the
    // host ncurses backend.
    if round_trip_gate_closed() {
        return;
    }
    let env = TerminfoEnv::compile_with_variant(TerminfoVariant::OriTermDirect);
    // `-x` is REQUIRED — `RGB` and `Tc` are both extension caps
    // and only appear under `infocmp -x`.
    let infocmp = Command::new("infocmp")
        .arg("-x")
        .arg("-A")
        .arg(env.terminfo_dir())
        .arg("ori_term-direct")
        .output()
        .expect("invoke infocmp");
    assert!(
        infocmp.status.success(),
        "infocmp -x -A on ori_term-direct failed: {}",
        String::from_utf8_lossy(&infocmp.stderr)
    );
    let out = String::from_utf8_lossy(&infocmp.stdout);
    // Capability marker is the truecolor contract: ncurses
    // applications check `RGB`/`Tc`, not the colors count, to
    // decide whether to emit 24-bit SGR. This assertion is the
    // load-bearing one — colors# is informational.
    assert!(
        out.contains("RGB") || out.contains("Tc"),
        "expected RGB or Tc in ori_term-direct, got:\n{out}"
    );
    // Modern ncurses (≥6.x) stores colors as int and reconstructs
    // `colors#16777216` (or `colors#0x1000000`). Legacy ncurses 5.x —
    // notably Apple's `/usr/bin/infocmp` on the macOS CI runner — uses
    // a 16-bit signed `short` for colors and clamps the reconstructed
    // value to `32767` (`SHRT_MAX`). The compiled entry IS still a
    // truecolor entry (RGB/Tc above are present); the count is purely
    // a backend-storage artifact, not a semantic difference. Accept
    // both representations so the test runs cleanly across the
    // ncurses-version split between Linux/Windows (modern) and
    // macOS's bundled `tic`/`infocmp` (legacy).
    assert!(
        out.contains("colors#16777216")
            || out.contains("colors#0x1000000")
            || out.contains("colors#32767"),
        "expected truecolor count (modern: colors#16777216 / colors#0x1000000; \
         legacy ncurses 5.x clamp: colors#32767) in ori_term-direct, got:\n{out}"
    );
}

#[test]
fn round_trip_gate_semantics_pinned() {
    // Pin the 2x2 truth table of `round_trip_gate_closed_for` so
    // that a future edit cannot silently invert, relax, or drop a
    // branch of the round-trip gate — the suite's contract is
    // that BOTH `tic` AND `infocmp` must be present for any
    // round-trip test to execute. If either is missing, every
    // round-trip test must short-circuit cleanly.
    //
    // The four round-trip/compile tests that need to skip when
    // tools are missing —
    // `terminfo_env_compiles_ori_term`,
    // `ori_term_terminfo_round_trips_via_infocmp`,
    // `ori_term_direct_declares_truecolor`, and
    // `child_process_with_apply_env_reads_pinned_terminfo` —
    // route their skip decision through `round_trip_gate_closed()`,
    // which in turn delegates to the pure
    // `round_trip_gate_closed_for(tic_ok, infocmp_ok)` form pinned
    // below. This is the SSOT for the gate; pinning the pure form
    // here is how a TPR-02-002-style "the gate test tests nothing"
    // regression gets caught — if someone flips
    // `!tic_ok || !infocmp_ok` to `!tic_ok && !infocmp_ok` (so
    // round-trip tests would run when only ONE tool is present),
    // one of the asserts below fails immediately.
    //
    // This test itself does NOT skip through `round_trip_gate_closed()`
    // — a gate test that short-circuits via its own gate would be
    // self-defeating. It runs in every environment (with or
    // without `tic`/`infocmp` installed) and asserts the gate's
    // pure form is correct, then asserts the live helper
    // delegates to that pure form.
    assert!(
        !round_trip_gate_closed_for(true, true),
        "gate must be OPEN when both tic and infocmp are present"
    );
    assert!(
        round_trip_gate_closed_for(true, false),
        "gate must be CLOSED when infocmp is missing even if tic is present"
    );
    assert!(
        round_trip_gate_closed_for(false, true),
        "gate must be CLOSED when tic is missing even if infocmp is present"
    );
    assert!(
        round_trip_gate_closed_for(false, false),
        "gate must be CLOSED when both tic and infocmp are missing"
    );

    // Also assert the live helper agrees with the pure form — if
    // someone forgets to route `round_trip_gate_closed` through
    // `round_trip_gate_closed_for`, this check fires.
    assert_eq!(
        round_trip_gate_closed(),
        round_trip_gate_closed_for(tic_available(), infocmp_available()),
        "round_trip_gate_closed must delegate to round_trip_gate_closed_for"
    );
}

#[test]
fn child_process_with_apply_env_reads_pinned_terminfo() {
    // Env-var precedence contract check — the only test in 02.4
    // that exercises the code path Sections 03-08 will rely on.
    //
    // Spawn `infocmp` as a child with TerminfoEnv's env-var triple
    // applied via the SAME SSOT (`env_pairs`) that `apply_env`
    // consumes. CRITICALLY do NOT pass `-A <dir>` — that would
    // make infocmp look up the entry by directory regardless of
    // env vars. With no `-A`, infocmp consults `$TERM` /
    // `$TERMINFO` / `$TERMINFO_DIRS` exactly the way tack will.
    //
    // Behavioral proof #1: `status.success()`. If env precedence
    // fails, the child falls back to `$HOME/.terminfo`,
    // `/etc/terminfo`, `/usr/share/terminfo` — and if none of
    // them ship `ori_term`, infocmp exits non-zero.
    //
    // Behavioral proof #2 (primary pin): `infocmp` prints
    // `# Reconstructed via infocmp from file: <path>` as its
    // first output line, where `<path>` is the actual file
    // ncurses opened. If env-precedence steered the child to our
    // tempdir, `<path>` will live INSIDE `env.terminfo_dir()`. A
    // system-installed `ori_term` (e.g., `/usr/share/terminfo/o/
    // ori_term` from a future packaging release) would show a
    // different path — this pin catches a regression in
    // `env_pairs()` / `apply_env()` even after ori_term gets
    // added to the default terminfo database. This addresses
    // TPR-02-001.
    //
    // Historical deviation from 02.4 plan text: the plan
    // originally proposed matching on `kf63=` as a uniqueness
    // marker on the assumption that host xterm-256color "usually
    // stops at kf48"; that is false on ncurses 6.x (which ships
    // kf1-kf63). An interim fix used `hs,` as a content marker,
    // but that was still host-state dependent (any future
    // system-installed `ori_term` could also declare `hs`). The
    // final fix pins on the actual file path infocmp loaded
    // from, which is immune to content collisions.
    //
    // We use `std::process::Command` (not
    // `portable_pty::CommandBuilder`) because the test needs
    // stdout capture, which CommandBuilder does not expose. Both
    // APIs accept (name, value) env pairs, so the SSOT iteration
    // pattern is identical — only the receiver changes.
    if round_trip_gate_closed() {
        return;
    }
    if !infocmp_respects_terminfo_env() {
        // Some hosts (notably MSYS infocmp on Windows CI runners)
        // ignore $TERMINFO / $TERMINFO_DIRS entirely and consult
        // only a hardcoded terminfo location. The end-to-end pin
        // for env precedence cannot run on those hosts because
        // the dependency is not present. The pure-form SSOT pin
        // (`apply_env_sets_three_vars`) runs on every platform
        // and catches `env_pairs()` regressions independently of
        // host infocmp behavior.
        eprintln!(
            "host infocmp does not honor TERMINFO/TERMINFO_DIRS env precedence \
             (likely MSYS infocmp on Windows) — skipping. The SSOT pin lives \
             in `apply_env_sets_three_vars` which runs on every platform."
        );
        return;
    }
    let env = TerminfoEnv::compile();
    let mut cmd = Command::new("infocmp");
    cmd.arg(env.term());
    for (name, value) in env.env_pairs() {
        cmd.env(name, value);
    }
    // Strip any inherited TERMCAP from the parent process so
    // we're only testing what the env_pairs SSOT sets.
    cmd.env_remove("TERMCAP");
    let out = cmd.output().expect("invoke infocmp");
    assert!(
        out.status.success(),
        "child infocmp failed (apply_env triple should have steered it to the pinned dir): stderr={}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let header = stdout
        .lines()
        .next()
        .expect("infocmp produced empty stdout");
    // Match on the unique tempdir basename rather than the full path.
    // Some `infocmp` builds (notably MSYS infocmp on Windows) report
    // the path in normalized form: drive letter stripped, mixed
    // slashes — e.g. `\Users\…\.tmpXYZ/6f/ori_term` instead of the
    // canonical `C:\Users\…\.tmpXYZ`. The tempdir basename
    // (`.tmpXYZ`) is the unique part assigned by `tempfile::TempDir`
    // and is sufficient to prove env precedence steered the child to
    // OUR tempdir — no other tempdir on the host shares that
    // randomly-generated name.
    let tempdir_basename = env
        .terminfo_dir()
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .expect("tempdir has a unicode basename");
    let tempdir_str = env.terminfo_dir().to_string_lossy();
    assert!(
        header.contains("Reconstructed") && header.contains(tempdir_basename),
        "infocmp reconstruction-source header `{header}` does not contain the \
         pinned tempdir basename `{tempdir_basename}` (full tempdir path: \
         `{tempdir_str}`) — env precedence did not steer the child to OUR \
         compiled `ori_term` entry. A system-installed ori_term entry can \
         mask a regression in env_pairs() / apply_env() without this pin. \
         full stdout:\n{stdout}",
    );
}

#[test]
fn terminfo_env_compile_under_perf_budget() {
    // Self-contained per-call performance benchmark. Section 02
    // owns ONLY the per-call ceiling and the 50-call projection;
    // Section 09 owns the post-Sections-03-08 aggregate
    // measurement. This test runs under whatever profile
    // `cargo test` uses; the completion notes record both debug
    // and release numbers via the `eprintln!` below.
    if !tic_available() {
        return;
    }
    // Warm-up call so we measure steady-state cost, not first-run
    // overhead (filesystem cache, dynamic linker resolution).
    let _warmup = TerminfoEnv::compile();
    let t0 = Instant::now();
    let _env = TerminfoEnv::compile();
    let elapsed = t0.elapsed();
    // Per-call ceiling: 1000 ms. Captures a ~5x regression vs.
    // observed local debug timings (~150 ms) and ~10x vs. release.
    // BOTH debug AND release must meet this ceiling.
    assert!(
        elapsed.as_millis() < 1000,
        "TerminfoEnv::compile() took {}ms (>1000ms ceiling) — investigate before deferring",
        elapsed.as_millis(),
    );
    // 50-call projection (Sections 03-08 will spawn ~50 TerminfoEnv
    // instances). If the projection exceeds 30 s, file a shared-
    // cache follow-up bug via /add-bug — filing is NOT deferral.
    let projected_50 = elapsed.as_millis() * 50;
    eprintln!(
        "TerminfoEnv::compile() warm = {}ms; 50-call projection = {}ms",
        elapsed.as_millis(),
        projected_50
    );
    assert!(
        projected_50 < 30_000,
        "50-call projection {projected_50}ms > 30s ceiling — file /add-bug for shared-cache follow-up",
    );
}

#[test]
fn infocmp_env_precedence_probe_is_deterministic() {
    // The probe must be deterministic across calls. A regression
    // that accidentally modifies process state (leaving env vars
    // set, leaking tempdirs across calls, mutating CWD) would
    // make the second call diverge from the first.
    let first = infocmp_respects_terminfo_env();
    let second = infocmp_respects_terminfo_env();
    assert_eq!(
        first, second,
        "probe must be deterministic across calls; first={first}, second={second}"
    );
}

#[test]
fn infocmp_env_precedence_probe_returns_false_when_tools_missing() {
    // When `tic` or `infocmp` is missing, the probe MUST return
    // false rather than panic. This pin proves the early-return
    // gate at the top of the helper. We can only assert this
    // directly when the tools ARE missing — otherwise we assert
    // the looser invariant that "tools-present implies the probe
    // does not panic" via the determinism test above.
    if !tic_available() || !infocmp_available() {
        assert!(
            !infocmp_respects_terminfo_env(),
            "probe must return false when tic or infocmp is missing"
        );
    }
}

// Section 08.1 — decode_terminfo_string tests

#[test]
fn decode_terminfo_string_handles_esc() {
    assert_eq!(decode_terminfo_string("\\EOP"), b"\x1bOP");
}

#[test]
fn decode_terminfo_string_handles_esc_bracket() {
    assert_eq!(decode_terminfo_string("\\E[A"), b"\x1b[A");
}

#[test]
fn decode_terminfo_string_handles_control_caret() {
    assert_eq!(decode_terminfo_string("^?"), &[0x7f]); // DEL
    assert_eq!(decode_terminfo_string("^H"), &[0x08]); // BS
    assert_eq!(decode_terminfo_string("^A"), &[0x01]); // SOH
}

#[test]
fn decode_terminfo_string_handles_octal() {
    assert_eq!(decode_terminfo_string("\\033"), &[0x1b]); // ESC
}

#[test]
fn decode_terminfo_string_handles_special_escapes() {
    assert_eq!(decode_terminfo_string("\\r"), b"\r");
    assert_eq!(decode_terminfo_string("\\n"), b"\n");
    assert_eq!(decode_terminfo_string("\\t"), b"\t");
    assert_eq!(decode_terminfo_string("\\\\"), b"\\");
    assert_eq!(decode_terminfo_string("\\s"), b" ");
}

#[test]
fn decode_terminfo_string_handles_plain_ascii() {
    assert_eq!(decode_terminfo_string("abc"), b"abc");
}

#[test]
fn decode_terminfo_string_handles_combined_sequences() {
    // \E[15~ is a typical function key sequence
    assert_eq!(decode_terminfo_string("\\E[15~"), b"\x1b[15~");
    // \EOA is application-mode cursor up
    assert_eq!(decode_terminfo_string("\\EOA"), b"\x1bOA");
}

// Section 08.1 — infocmp_query / infocmp_dump tests

#[test]
fn infocmp_query_returns_none_for_missing_cap() {
    if !tic_available() || !infocmp_available() {
        return;
    }
    let env = TerminfoEnv::compile();
    assert_eq!(
        infocmp_query(&env, "ori_term", "completely_made_up_cap_xyz"),
        None
    );
}

#[test]
fn infocmp_query_extracts_kf1() {
    if !tic_available() || !infocmp_available() {
        return;
    }
    let env = TerminfoEnv::compile();
    let kf1 = infocmp_query(&env, "ori_term", "kf1");
    assert!(kf1.is_some(), "ori_term must declare kf1");
    let raw = decode_terminfo_string(&kf1.unwrap());
    assert_eq!(raw[0], 0x1b, "kf1 should start with ESC");
    assert!(raw.len() >= 2, "kf1 should be at least 2 bytes");
}

#[test]
fn infocmp_dump_returns_populated_map() {
    if !tic_available() || !infocmp_available() {
        return;
    }
    let env = TerminfoEnv::compile();
    let caps = infocmp_dump(&env, "ori_term").expect("infocmp dump should succeed for ori_term");
    assert!(caps.contains_key("kf1"), "dump should contain kf1");
    assert!(
        caps.contains_key("am"),
        "dump should contain boolean cap am"
    );
    assert!(
        caps.contains_key("colors"),
        "dump should contain numeric cap colors"
    );
    assert!(!caps.contains_key("completely_made_up_xyz"));
}
