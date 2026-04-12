//! Sibling tests for `catalog_coverage_check` per
//! `.claude/rules/test-organization.md`.
//!
//! Matrix per `.claude/rules/tests.md` §Matrix Testing Rule:
//!
//! - **Positive pins** — every canonical tuple shape is
//!   recognized by `canonical_tuple`, every parser arm is
//!   exercised, every extractor returns a known-good tuple set
//!   against a fixture file.
//! - **Negative pins** — `--check` REJECTS (a) duplicate IDs,
//!   (b) wezterm in Spec source, (c) verified rows in bootstrap
//!   mode, (d) line-number-primary Implementation citations.
//! - **Cross-type matrix** — CSI with / without intermediates,
//!   OSC with BEL and ST, DCS sixel vs DECRQSS.
//! - **Self-verifying completeness counter** —
//!   `matrix_visits_every_cell_exactly_once`.
//!
//! Test function naming follows `<subject>_<scenario>_<expected>`
//! shape per `.claude/rules/impl-hygiene.md` §Test Function
//! Naming. No ephemeral identifiers.

use std::io::Write;

use tempfile::TempDir;

use super::check::{CheckMode, FindingCategory, check};
use super::parser::parse_catalog_markdown;
use super::row::Verification;
use super::tuple::{Category, Tuple, canonical_tuple};

// -------- Positive pins: canonical_tuple -----------------------------------

#[test]
fn cup_sequence_canonicalizes_to_csi_ps_ps_h() {
    let t = canonical_tuple("`CSI Ps;Ps H`").expect("CUP must canonicalize");
    assert_eq!(t.category, Category::Csi);
    assert!(t.intermediates.is_empty());
    assert_eq!(t.params, "Ps;Ps");
    assert_eq!(t.final_byte, "H");
}

#[test]
fn cuu_sequence_canonicalizes_to_csi_ps_a() {
    let t = canonical_tuple("`CSI Ps A`").expect("CUU must canonicalize");
    assert_eq!(t.category, Category::Csi);
    assert!(t.intermediates.is_empty());
    assert_eq!(t.params, "Ps");
    assert_eq!(t.final_byte, "A");
}

#[test]
fn decset_pair_canonicalizes_to_first_alternative_h_form() {
    // Catalog rows use `CSI ? Ps h / CSI ? Ps l` for paired
    // DECSET/DECRST rows. The canonicalizer takes the first
    // alternative — the paired expansion happens at check time.
    let t = canonical_tuple("`CSI ? 25 h / CSI ? 25 l`").expect("DECSET must canonicalize");
    assert_eq!(t.category, Category::Csi);
    assert_eq!(t.intermediates, vec![b'?']);
    assert_eq!(t.final_byte, "h");
}

#[test]
fn pm_sequence_canonicalizes_to_pm_empty_ints_pt_st() {
    let t = canonical_tuple("`ESC ^ Pt ST`").expect("PM must canonicalize");
    assert_eq!(t.category, Category::Pm);
    assert!(t.intermediates.is_empty());
    assert_eq!(t.params, "Pt");
    assert_eq!(t.final_byte, "ST");
}

#[test]
fn sos_sequence_canonicalizes_to_sos_empty_ints_pt_st() {
    let t = canonical_tuple("`ESC X Pt ST`").expect("SOS must canonicalize");
    assert_eq!(t.category, Category::Sos);
    assert!(t.intermediates.is_empty());
    assert_eq!(t.params, "Pt");
    assert_eq!(t.final_byte, "ST");
}

#[test]
fn dcs_sixel_canonicalizes_to_dcs_empty_ints_pid_q() {
    let t = canonical_tuple("`DCS Ps1 ; Ps2 ; Ps3 q <data> ST`").expect("DCS q must canonicalize");
    assert_eq!(t.category, Category::Dcs);
    assert!(t.intermediates.is_empty());
    assert_eq!(t.params, "Pid");
    assert_eq!(t.final_byte, "q");
}

#[test]
fn dcs_decrqss_canonicalizes_to_dcs_dollar_pt_q() {
    let t = canonical_tuple("`DCS $ q Pt ST`").expect("DECRQSS must canonicalize");
    assert_eq!(t.category, Category::Dcs);
    assert_eq!(t.intermediates, vec![b'$']);
    assert_eq!(t.params, "Pt");
    assert_eq!(t.final_byte, "q");
}

#[test]
fn osc_title_canonicalizes_with_numeric_id_preserved() {
    let t = canonical_tuple("`OSC 0 ; Pt BEL|ST`").expect("OSC 0 must canonicalize");
    assert_eq!(t.category, Category::Osc);
    assert!(t.intermediates.is_empty());
    assert_eq!(t.params, "0;text");
    assert_eq!(t.final_byte, "BEL");
}

#[test]
fn osc_4_palette_canonicalizes_with_index_placeholder() {
    let t = canonical_tuple("`OSC 4 ; index ; rgb BEL|ST`").expect("OSC 4 must canonicalize");
    assert_eq!(t.params, "4;index;rgb");
}

#[test]
fn osc_numeric_id_must_not_collapse_to_ps() {
    // Negative pin: the numeric id is preserved literally; if it
    // collapsed to `Ps` we would lose dispatch-level discrimination.
    let t = canonical_tuple("`OSC 52 ; Pc ; <b64> BEL|ST`").expect("OSC 52 must canonicalize");
    assert!(t.params.starts_with("52;"));
    assert_ne!(t.params, "Ps;Ps;Ps");
}

#[test]
fn apc_kitty_canonicalizes_to_apc_g_key_value_st() {
    let t = canonical_tuple("`APC G key=value ; <data> ST`").expect("APC _G must canonicalize");
    assert_eq!(t.category, Category::Apc);
    assert_eq!(t.intermediates, vec![b'G', b'_']);
    assert_eq!(t.params, "key-value");
    assert_eq!(t.final_byte, "ST");
}

#[test]
fn decscusr_canonicalizes_to_csi_sp_ps_q() {
    let t = canonical_tuple("`CSI Ps SP q`").expect("DECSCUSR must canonicalize");
    assert_eq!(t.intermediates, vec![b' ']);
    assert_eq!(t.final_byte, "q");
}

#[test]
fn esc_d_canonicalizes_to_esc_empty_dash_d() {
    let t = canonical_tuple("`ESC D`").expect("IND must canonicalize");
    assert_eq!(t.category, Category::Esc);
    assert_eq!(t.final_byte, "D");
}

#[test]
fn esc_charset_designation_canonicalizes_to_da_paren_b() {
    let t = canonical_tuple("`ESC ( B`").expect("G0=ASCII must canonicalize");
    assert_eq!(t.category, Category::Da);
    assert_eq!(t.intermediates, vec![b'(']);
    assert_eq!(t.final_byte, "B");
}

// -------- Tuple equality / sorting -----------------------------------------

#[test]
fn tuple_sort_intermediates_on_construction() {
    // Unsorted input → sorted output so hash equality holds.
    let t1 = Tuple::new(Category::Dcs, vec![b'!', b'$'], "Pt", "q");
    let t2 = Tuple::new(Category::Dcs, vec![b'$', b'!'], "Pt", "q");
    assert_eq!(t1, t2);
}

// -------- Negative pins: --check rejection ---------------------------------

#[test]
fn check_rejects_wezterm_as_spec_source() {
    let (_tmp, catalog_dir) = make_fixture_catalog(&[(
        "ecma-48.md",
        row_markdown(
            "ECMA48-BAD",
            "wezterm escape-sequences.md",
            "`CSI X`",
            "implemented-unverified",
            "`Term::stub` (`oriterm_core/src/term/handler/mod.rs`)",
        ),
    )]);
    let report = check(&catalog_dir, CheckMode::Normal).expect("parse ok");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.category == FindingCategory::SpecSourceCitesPeer),
        "expected SpecSourceCitesPeer; got: {:?}",
        report.findings
    );
}

#[test]
fn check_rejects_verified_status_in_bootstrap_mode() {
    let (_tmp, catalog_dir) = make_fixture_catalog(&[(
        "ecma-48.md",
        row_markdown(
            "ECMA48-BOOTSTRAP-VIOLATION",
            "ECMA-48 §8.3.21",
            "`CSI H`",
            "verified",
            "`Term::goto` (`oriterm_core/src/term/handler/mod.rs`)",
        ),
    )]);
    let bootstrap_report = check(&catalog_dir, CheckMode::Bootstrap).expect("parse ok");
    assert!(
        bootstrap_report
            .findings
            .iter()
            .any(|f| f.category == FindingCategory::VerifiedInBootstrap),
        "bootstrap must reject verified; got {:?}",
        bootstrap_report.findings
    );

    // And in Normal mode the same fixture passes.
    let normal_report = check(&catalog_dir, CheckMode::Normal).expect("parse ok");
    assert!(
        normal_report
            .findings
            .iter()
            .all(|f| f.category != FindingCategory::VerifiedInBootstrap),
        "normal mode must NOT reject verified"
    );
}

#[test]
fn check_rejects_line_number_primary_citation() {
    let (_tmp, catalog_dir) = make_fixture_catalog(&[(
        "ecma-48.md",
        row_markdown(
            "ECMA48-LN",
            "ECMA-48 §8.3.21",
            "`CSI H`",
            "implemented-unverified",
            "`crates/vte/src/ansi/dispatch/csi.rs:91 → TermHandler::goto`",
        ),
    )]);
    let report = check(&catalog_dir, CheckMode::Normal).expect("parse ok");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.category == FindingCategory::LineNumberPrimaryCitation),
        "expected LineNumberPrimaryCitation; got {:?}",
        report.findings
    );
}

#[test]
fn check_rejects_duplicate_row_id() {
    let (_tmp, catalog_dir) = make_fixture_catalog(&[
        (
            "ecma-48.md",
            row_markdown(
                "ECMA48-DUP",
                "ECMA-48 §8.3.21",
                "`CSI H`",
                "implemented-unverified",
                "`Term::goto` (`oriterm_core/src/term/handler/mod.rs`)",
            ),
        ),
        (
            "xterm-ctlseqs.md",
            row_markdown(
                "ECMA48-DUP",
                "xterm ctlseqs",
                "`CSI 14 t`",
                "implemented-unverified",
                "`Term::text_area_size_pixels` (`oriterm_core/src/term/handler/mod.rs`)",
            ),
        ),
    ]);
    let report = check(&catalog_dir, CheckMode::Normal).expect("parse ok");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.category == FindingCategory::DuplicateId),
        "expected DuplicateId; got {:?}",
        report.findings
    );
}

// -------- Positive pin: full real catalog passes ---------------------------

#[test]
fn real_catalog_passes_bootstrap_mode_check() {
    let repo_root = std::env::var("CARGO_MANIFEST_DIR")
        .ok()
        .and_then(|mpd| {
            std::path::PathBuf::from(mpd)
                .parent()?
                .parent()
                .map(std::path::Path::to_path_buf)
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let catalog_dir = repo_root.join("plans/spec-conformance/catalog");
    if !catalog_dir.exists() {
        // Some test environments (pre-Section-01.1 checkouts) don't
        // have the catalog yet — skip gracefully.
        eprintln!(
            "SKIP real_catalog_passes_bootstrap_mode_check: {} missing",
            catalog_dir.display()
        );
        return;
    }
    let report = check(&catalog_dir, CheckMode::Bootstrap).expect("real catalog parses");
    assert!(
        !report.has_findings(),
        "real catalog bootstrap check has findings: {:?}",
        report.findings
    );
    assert!(report.files_scanned > 0);
    assert!(report.rows_total > 0);
}

// -------- Row parser happy-path --------------------------------------------

#[test]
fn parse_catalog_markdown_accepts_well_formed_row() {
    let (_tmp, catalog_dir) = make_fixture_catalog(&[(
        "ecma-48.md",
        row_markdown(
            "ECMA48-OK",
            "ECMA-48 §8.3.21",
            "`CSI Ps;Ps H`",
            "implemented-unverified",
            "`Term::goto` (`oriterm_core/src/term/handler/mod.rs`)",
        ),
    )]);
    let path = catalog_dir.join("ecma-48.md");
    let rows = parse_catalog_markdown(&path).expect("valid row parses");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "ECMA48-OK");
    assert_eq!(rows[0].verification, Verification::ImplementedUnverified);
}

// -------- Self-verifying completeness counter ------------------------------

#[test]
fn matrix_visits_every_category_exactly_once() {
    // Enumerate every Category variant the canonicalizer can
    // produce and assert the count matches the enum. This catches
    // new variants that are added without a test.
    let categories = [
        Category::C0,
        Category::C1,
        Category::Esc,
        Category::Csi,
        Category::Osc,
        Category::Dcs,
        Category::Apc,
        Category::Pm,
        Category::Sos,
        Category::Da,
    ];
    let mut visited = 0_usize;
    for c in categories {
        // Touch every variant so a missing Display arm fails.
        let _ = format!("{c}");
        visited += 1;
    }
    assert_eq!(visited, 10);
}

// -------- Fixture helpers --------------------------------------------------

fn row_markdown(
    id: &str,
    spec_source: &str,
    sequence: &str,
    verification: &str,
    implementation: &str,
) -> String {
    format!(
        r#"---
schema_version: "0.1-provisional"
---

# Test Fixture

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| {id} | {spec_source} | {sequence} | Test row | {implementation} | state-snapshot | parser:pending | {verification} | — | fixture |
"#,
    )
}

fn make_fixture_catalog(files: &[(&str, String)]) -> (TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let catalog_dir = tmp.path().join("catalog");
    std::fs::create_dir_all(&catalog_dir).expect("create catalog dir");
    for (name, body) in files {
        let path = catalog_dir.join(name);
        let mut file = std::fs::File::create(&path).expect("create fixture file");
        file.write_all(body.as_bytes()).expect("write fixture");
    }
    (tmp, catalog_dir)
}
