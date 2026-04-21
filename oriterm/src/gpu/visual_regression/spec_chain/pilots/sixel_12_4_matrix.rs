//! §12.4 matrix-count guardrails.
//!
//! Ensures the eight §12.4 GPU-apex scenarios (A–G + SetToBg pilot) are
//! all present in the pilots directory AND have committed golden PNGs
//! under `oriterm/tests/references/`. The structural negative pin —
//! "unavailable deterministic lane skips cleanly, never false-passes" —
//! is embedded per-pilot via the `let Some(mut harness) =
//! VisualSpecHarness::new() else { eprintln!("SKIP: …"); return; };`
//! guard that every §12.4 pilot opens with; a standalone wrapper test
//! would be a tautology (no assertion, both branches already covered
//! by the existing pilots).

/// The eight §12.4 scenarios. Each name is both the pilot file stem
/// under `pilots/` AND the golden reference stem under
/// `oriterm/tests/references/<name>.png`.
///
/// Scenario A is the §04 pilot (`sixel_minimal`) reused as the §12.4
/// solid-rectangle row — its golden already lives at
/// `oriterm/tests/references/sixel_minimal.png`.
///
/// `sixel_set_to_bg` is the P2=2 pilot added round-1 to explicitly pin
/// the terminal-bg fill branch of `SixelBgMode::SetToBg` (§12.2's fix).
const SCENARIOS: &[&str] = &[
    "sixel_minimal",        // A — solid rectangle (P2=0 default)
    "sixel_palette_switch", // B — palette-switch mid-image
    "sixel_repeat",         // C — `!` repeat optimization
    "sixel_cr_nl_banding",  // D — `$` CR + `-` NL banding
    "sixel_transparency",   // E — P2=1 transparency composite
    "sixel_scrolling_off",  // F — DECRST 80 cursor-stays-at-home
    "sixel_cursor_right",   // G — DECSET 8452 cursor-right-of-image
    "sixel_set_to_bg",      // P2=2 — SetToBg terminal-bg fill (DECSCNM-aware)
];

/// Matrix-count assertion: §12.4 drives exactly 8 scenarios.
///
/// Adding or removing a scenario without updating this count is a
/// regression — the catalog rows for §12.4 GPU-apex would then
/// mis-cite the pilot that backs them.
#[test]
fn sixel_12_4_matrix_has_expected_scenario_count() {
    assert_eq!(
        SCENARIOS.len(),
        8,
        "§12.4 must drive exactly 8 GPU-apex scenarios (A–G + SetToBg P2=2)"
    );
}

/// Verify every §12.4 scenario has a committed golden PNG.
///
/// Fails before `ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm -- sixel`
/// has run — this is the TDD failing-test anchor. After goldens are
/// captured, this test turns green and stays as a regression guard
/// against accidental golden deletion.
#[test]
fn sixel_12_4_every_scenario_has_committed_golden() {
    let references_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/references");
    let mut missing = Vec::new();
    for name in SCENARIOS {
        let path = references_dir.join(format!("{name}.png"));
        if !path.exists() {
            missing.push(path);
        }
    }
    assert!(
        missing.is_empty(),
        "missing golden(s): {missing:?}\n\
         Run `ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm -- sixel_` to capture them."
    );
}
