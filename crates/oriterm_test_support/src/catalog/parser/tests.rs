//! Tests for the `parser` module: catalog markdown table parsing.

use std::io::Write;

use tempfile::TempDir;

use crate::catalog::row::Verification;

use super::parse_catalog_markdown;

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
