//! OSC scenarios (title, icon name, clipboard, color query).

use std::path::Path;

use super::harness::{self, RecordedEvent, ScenarioOutcome, TeseqHarness, reseq_available};

/// Run an OSC scenario and apply spec assertions.
///
/// Returns `None` when `reseq` is unavailable (graceful skip with visible message).
/// Returns the outcome for callers to perform additional event assertions.
fn run_scenario(name: &str) -> Option<ScenarioOutcome> {
    if !reseq_available() {
        eprintln!("reseq not installed, skipping");
        return None;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/osc")
        .join(format!("{name}.teseq"));
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), &format!("osc_{name}"));
    Some(outcome)
}

#[test]
fn osc_title() {
    let Some(outcome) = run_scenario("osc_title") else {
        return;
    };
    // OSC 0 sets both title AND icon name.
    let titles: Vec<_> = outcome
        .events
        .iter()
        .filter_map(|e| match e {
            RecordedEvent::Title(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        titles,
        &["My Terminal Title", "Window Title Only"],
        "expected 2 Title events"
    );
    let icons: Vec<_> = outcome
        .events
        .iter()
        .filter_map(|e| match e {
            RecordedEvent::IconName(n) => Some(n.as_str()),
            _ => None,
        })
        .collect();
    // OSC 0 emits IconName; OSC 2 does not.
    assert_eq!(icons, &["My Terminal Title"], "expected 1 IconName event");
}

#[test]
fn osc_icon_name() {
    let Some(outcome) = run_scenario("osc_icon_name") else {
        return;
    };
    let icons: Vec<_> = outcome
        .events
        .iter()
        .filter_map(|e| match e {
            RecordedEvent::IconName(n) => Some(n.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(icons, &["My Icon Name"]);
}

#[test]
fn osc_clipboard() {
    let Some(outcome) = run_scenario("osc_clipboard") else {
        return;
    };
    let clips: Vec<_> = outcome
        .events
        .iter()
        .filter_map(|e| match e {
            RecordedEvent::ClipboardStore(_, text) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(clips, &["Hello"], "expected base64-decoded 'Hello'");
}

#[test]
fn osc_color_query() {
    let Some(outcome) = run_scenario("osc_color_query") else {
        return;
    };
    let requests: Vec<_> = outcome
        .events
        .iter()
        .filter_map(|e| match e {
            RecordedEvent::ColorRequest(idx) => Some(*idx),
            _ => None,
        })
        .collect();
    // OSC 4;1;? → palette index 1 (red)
    // OSC 10;? → NamedColor::Foreground = 256
    // OSC 11;? → NamedColor::Background = 257
    assert_eq!(requests, &[1, 256, 257], "expected color query indices");
}
