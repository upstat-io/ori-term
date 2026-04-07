use std::io::Write;

use portable_pty::CommandBuilder;

use super::{ORI_TERM_INFO, TerminfoEnv, TerminfoVariant};
use crate::{infocmp_available, tic_available};

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
    if !tic_available() || !infocmp_available() {
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
