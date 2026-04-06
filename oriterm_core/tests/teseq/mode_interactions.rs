//! Mode interaction scenarios (DECOM, DECCOLM, alt screen, IRM, cross-cutting).

use std::path::Path;

use oriterm_core::TermMode;

use super::harness::{
    self, ScenarioOutcome, TeseqHarness, assert_grid_cols, assert_mode_contains,
    assert_mode_not_contains, assert_scrollback_empty, reseq_available,
};

/// Run a mode interaction scenario and apply spec assertions.
///
/// Returns the outcome for callers to perform additional mode/grid assertions.
/// Callers must guard with `if !reseq_available() { return; }` before calling.
fn run_scenario(name: &str) -> ScenarioOutcome {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/csi/modes")
        .join(format!("{name}.teseq"));
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), &format!("mode_interactions_{name}"));
    outcome
}

// Origin mode + scroll region

#[test]
fn origin_scroll_basic() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("origin_scroll_basic");
    assert_mode_contains(&outcome, TermMode::ORIGIN);
}

#[test]
fn origin_scroll_overflow() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("origin_scroll_overflow");
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    assert_scrollback_empty(&outcome);
}

#[test]
fn origin_cursor_save_restore() {
    if !reseq_available() {
        return;
    }
    run_scenario("origin_cursor_save_restore");
}

#[test]
fn il_in_scroll_region() {
    if !reseq_available() {
        return;
    }
    run_scenario("il_in_scroll_region");
}

#[test]
fn dl_in_scroll_region() {
    if !reseq_available() {
        return;
    }
    run_scenario("dl_in_scroll_region");
}

#[test]
fn origin_scroll_basic_97x33() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("origin_scroll_basic_97x33");
    assert_mode_contains(&outcome, TermMode::ORIGIN);
}

#[test]
fn origin_scroll_basic_120x40() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("origin_scroll_basic_120x40");
    assert_mode_contains(&outcome, TermMode::ORIGIN);
}

#[test]
fn origin_scroll_overflow_97x33() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("origin_scroll_overflow_97x33");
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    assert_scrollback_empty(&outcome);
}

#[test]
fn origin_scroll_overflow_120x40() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("origin_scroll_overflow_120x40");
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    assert_scrollback_empty(&outcome);
}

// DECCOLM

#[test]
fn deccolm_80_to_132() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("deccolm_80_to_132");
    assert_grid_cols(&outcome, 132);
}

#[test]
fn deccolm_132_to_80() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("deccolm_132_to_80");
    assert_grid_cols(&outcome, 80);
}

#[test]
fn deccolm_wrap_interaction() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("deccolm_wrap_interaction");
    assert_grid_cols(&outcome, 132);
}

#[test]
fn deccolm_resets_scroll_region() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("deccolm_resets_scroll_region");
    assert_grid_cols(&outcome, 132);
}

#[test]
fn deccolm_no_mode40() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("deccolm_no_mode40");
    assert_grid_cols(&outcome, 80);
}

#[test]
fn deccolm_132_to_80_120x40() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("deccolm_132_to_80_120x40");
    assert_grid_cols(&outcome, 120);
}

// Alt screen

#[test]
fn altscreen_roundtrip() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("altscreen_roundtrip");
    assert_mode_not_contains(&outcome, TermMode::ALT_SCREEN);
}

#[test]
fn altscreen_cursor() {
    if !reseq_available() {
        return;
    }
    run_scenario("altscreen_cursor");
}

#[test]
fn altscreen_content_isolation() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("altscreen_content_isolation");
    assert_mode_not_contains(&outcome, TermMode::ALT_SCREEN);
}

#[test]
fn altscreen_mode_leakage() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("altscreen_mode_leakage");
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    assert_mode_not_contains(&outcome, TermMode::LINE_WRAP);
    assert_mode_contains(&outcome, TermMode::INSERT);
}

#[test]
fn altscreen_reentry_1049() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("altscreen_reentry_1049");
    assert_mode_contains(&outcome, TermMode::ALT_SCREEN);
}

#[test]
fn altscreen_reentry_1047() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("altscreen_reentry_1047");
    assert_mode_contains(&outcome, TermMode::ALT_SCREEN);
}

#[test]
fn altscreen_roundtrip_97x33() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("altscreen_roundtrip_97x33");
    assert_mode_not_contains(&outcome, TermMode::ALT_SCREEN);
}

// Insert mode & wrap

#[test]
fn irm_insert() {
    if !reseq_available() {
        return;
    }
    run_scenario("irm_insert");
}

#[test]
fn irm_at_right_margin() {
    if !reseq_available() {
        return;
    }
    run_scenario("irm_at_right_margin");
}

#[test]
fn irm_wide_char() {
    if !reseq_available() {
        return;
    }
    run_scenario("irm_wide_char");
}

#[test]
fn wrap_at_margin() {
    if !reseq_available() {
        return;
    }
    run_scenario("wrap_at_margin");
}

#[test]
fn wrap_disabled() {
    if !reseq_available() {
        return;
    }
    run_scenario("wrap_disabled");
}

#[test]
fn wrap_disabled_wide_char() {
    if !reseq_available() {
        return;
    }
    run_scenario("wrap_disabled_wide_char");
}

#[test]
fn wrap_at_margin_120x40() {
    if !reseq_available() {
        return;
    }
    run_scenario("wrap_at_margin_120x40");
}

// Cross-cutting

#[test]
fn cross_deccolm_with_decom_decstbm() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("cross_deccolm_with_decom_decstbm");
    assert_grid_cols(&outcome, 132);
    assert_mode_contains(&outcome, TermMode::ORIGIN);
}

#[test]
fn cross_1049_with_decom_decstbm() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("cross_1049_with_decom_decstbm");
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    assert_mode_not_contains(&outcome, TermMode::ALT_SCREEN);
}

#[test]
fn cross_irm_at_margin_with_decawm() {
    if !reseq_available() {
        return;
    }
    run_scenario("cross_irm_at_margin_with_decawm");
}

#[test]
fn cross_scrollback_after_decstbm_overflow() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("cross_scrollback_after_decstbm_overflow");
    assert_scrollback_empty(&outcome);
}

#[test]
fn cross_scrollback_after_deccolm_clear() {
    if !reseq_available() {
        return;
    }
    let outcome = run_scenario("cross_scrollback_after_deccolm_clear");
    assert_scrollback_empty(&outcome);
}
