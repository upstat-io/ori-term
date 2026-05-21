//! Spec_chain conversion of the tack `enq_ack` scenario family.
//!
//! Drives `ECMA48-C0-ENQ` (`0x05`, ECMA-48 §8.3.40) end-to-end through
//! parser → dispatch → effect emission, plus reset/alt-screen interaction
//! cells pinning the terminal-global-config invariant for `Term<S>::answerback`.
//!
//! # Catalog rows verified
//!
//! - `ECMA48-C0-ENQ` — Enquiry / Answerback control character (`0x05`) →
//! `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::Answerback, .. })`
//! when answerback is configured non-empty; no effect when empty (default).
//!
//! Replaces the load-bearing regression guard
//! `ecma48_c0_enq_catalog_row_still_missing` that pinned the catalog status
//! until the answerback dispatch chain landed.
//! See: 

use oriterm_core::effect::{Effect, PtyWriteKind};
use oriterm_test_support::spec_chain::{
 ApexLayer, DispatchExpectation, EffectExpectation, ParserExpectation, ScenarioExpectations,
 SpecHarness, SpecScenario, effect_filters::pty_writes_of_kind,
};

/// Snapshot visible-row cell chars across the entire grid for
/// renderable-state clamp comparisons (ENQ-no-mutation pin helper).
fn snapshot_visible_chars(harness: &SpecHarness) -> Vec<char> {
 let grid = harness.term().grid();
 let lines = grid.lines();
 let cols = grid.cols();
 let mut buf = Vec::with_capacity(lines * cols);
 for line in 0..lines {
 let row = &grid[oriterm_core::index::Line(line as i32)];
 for col in 0..cols {
 buf.push(row[oriterm_core::index::Column(col)].ch);
 }
 }
 buf
}

/// ENQ drives parser → dispatch → effect-emit when answerback configured.
/// drives `ECMA48-C0-ENQ` end-to-end and pins
/// byte-exact answerback emission with `PtyWriteKind::Answerback` kind.
/// Side-effect clamps assert ENQ does NOT emit Host effects or mutate
/// renderable state.
#[test]
fn ecma48_c0_enq_drives_to_pty_write_apex() {
 let scenario = SpecScenario {
 catalog_row_id: "ECMA48-C0-ENQ",
 bytes: b"\x05",
 apex_layer: ApexLayer::EffectPtyWrite,
 setup: b"",
 expectations: ScenarioExpectations {
 parser: Some(ParserExpectation {
 action: '\x05',
 params: &[],
 intermediates: &[],
 osc_command: None,
 }),
 dispatch: Some(DispatchExpectation::method("enquiry")),
 effect: Some(EffectExpectation::pty("Answerback")),
 ..ScenarioExpectations::default()
 },
 };

 let mut harness = SpecHarness::new();
 harness
 .term_mut()
 .set_answerback(b"oriterm-answer".to_vec());

 // Snapshot pre-feed grid state for renderable-clamp (cursor + visible-row cells).
 let pre_cursor = (
 harness.term().grid().cursor().line(),
 harness.term().grid().cursor().col(),
 );
 let pre_grid_chars = snapshot_visible_chars(&harness);

 let results = harness.run_scenario(&scenario);
 for r in &results {
 assert!(
 r.passed,
 "rung {:?} failed: {}",
 r.rung_name,
 r.failure.as_deref().unwrap_or("(no message)")
 );
 }

 // Byte-exact post-assertion — apex pins kind, NOT bytes; this assertion pins the bytes.
 let answerback_writes: Vec<_> =
 pty_writes_of_kind(&harness, PtyWriteKind::Answerback).collect();
 assert_eq!(
 answerback_writes.len(),
 1,
 "ENQ must emit exactly one Answerback write"
 );
 assert_eq!(
 answerback_writes[0], b"oriterm-answer",
 "Answerback bytes must match configured string verbatim"
 );

 // Side-effect clamps — ENQ must not emit any non-PTY effects nor mutate renderable state.
 let host_effect_count = harness
 .outcome()
 .effects_emitted
 .iter()
 .filter(|e| matches!(e, Effect::Host(_)))
 .count();
 assert_eq!(host_effect_count, 0, "ENQ must NOT emit any Effect::Host");

 let post_cursor = (
 harness.term().grid().cursor().line(),
 harness.term().grid().cursor().col(),
 );
 assert_eq!(
 pre_cursor, post_cursor,
 "ENQ must NOT mutate cursor (renderable state)"
 );
 let post_grid_chars = snapshot_visible_chars(&harness);
 assert_eq!(
 pre_grid_chars, post_grid_chars,
 "ENQ must NOT mutate visible grid cells (renderable state)"
 );
}

/// Empty-answerback no-emit policy holds even with concurrent unrelated PTY writes.
/// kind-specific invariant check proves no false-positive
/// when DA1 reply (PtyWriteKind::DeviceAttribute) is emitted in the same transcript.
#[test]
fn enq_without_configured_answerback_emits_no_answerback_effect_in_mixed_transcript() {
 let mut harness = SpecHarness::new();
 // Default empty answerback (no set_answerback call).
 harness.feed(b"\x1b[c\x05"); // DA1, then ENQ

 // DA1 reply present (proves unrelated PTY writes flow normally).
 let da_writes: Vec<_> = pty_writes_of_kind(&harness, PtyWriteKind::DeviceAttribute).collect();
 assert_eq!(
 da_writes.len(),
 1,
 "DA1 reply must be present (proves PTY writes flow), got: {da_writes:?}"
 );

 // No Answerback writes (proves empty-answerback policy holds in mixed transcript).
 let answerback_writes: Vec<_> =
 pty_writes_of_kind(&harness, PtyWriteKind::Answerback).collect();
 assert!(
 answerback_writes.is_empty(),
 "ENQ with empty answerback must NOT emit Answerback effect, got: {answerback_writes:?}"
 );
}

/// Configured answerback survives RIS (ESC c).
/// `answerback` is terminal-global config (alongside
/// `bold_is_bright` / `image_protocol_enabled`). `esc_reset_state` does
/// field-by-field resets that explicitly omit `answerback`.
#[test]
fn enq_after_ris_preserves_configured_answerback() {
 let mut harness = SpecHarness::new();
 harness.term_mut().set_answerback(b"X".to_vec());
 harness.feed(b"\x1bc\x05"); // RIS, then ENQ

 let writes: Vec<_> = pty_writes_of_kind(&harness, PtyWriteKind::Answerback).collect();
 assert_eq!(
 writes.len(),
 1,
 "Answerback must survive RIS — `esc_reset_state` excludes terminal-global config"
 );
 assert_eq!(writes[0], b"X");
}

/// Configured answerback survives DECSTR (CSI ! p).
/// `soft_reset` is narrower than RIS and likewise
/// excludes terminal-global config.
#[test]
fn enq_after_decstr_preserves_configured_answerback() {
 let mut harness = SpecHarness::new();
 harness.term_mut().set_answerback(b"X".to_vec());
 harness.feed(b"\x1b[!p\x05"); // DECSTR, then ENQ

 let writes: Vec<_> = pty_writes_of_kind(&harness, PtyWriteKind::Answerback).collect();
 assert_eq!(
 writes.len(),
 1,
 "Answerback must survive DECSTR — `soft_reset` excludes terminal-global config"
 );
 assert_eq!(writes[0], b"X");
}

/// Configured answerback emits in both primary and alt screens (3 DECSET modes).
/// alt-screen toggle swaps grid + charset state +
/// origin-mode + keyboard-mode stacks, NOT terminal-global config. Tests all
/// 3 distinct alt-screen DECSET modes (47 / 1047 / 1049).
#[test]
fn enq_during_alt_screen_emits_terminal_global_answerback() {
 for (mode_set, mode_reset) in [
 (b"\x1b[?47h".as_slice(), b"\x1b[?47l".as_slice()),
 (b"\x1b[?1047h".as_slice(), b"\x1b[?1047l".as_slice()),
 (b"\x1b[?1049h".as_slice(), b"\x1b[?1049l".as_slice()),
 ] {
 let mut harness = SpecHarness::new();
 harness.term_mut().set_answerback(b"X".to_vec());

 // ENQ in alt-screen
 harness.feed(mode_set);
 harness.feed(b"\x05");
 // ENQ in primary (after alt-screen exit)
 harness.feed(mode_reset);
 harness.feed(b"\x05");

 let writes: Vec<_> = pty_writes_of_kind(&harness, PtyWriteKind::Answerback).collect();
 assert_eq!(
 writes.len(),
 2,
 "Two ENQs (one per screen) must emit two Answerback writes for mode_set={:?}, mode_reset={:?}",
 std::str::from_utf8(mode_set).unwrap_or("<bytes>"),
 std::str::from_utf8(mode_reset).unwrap_or("<bytes>")
 );
 assert!(writes.iter().all(|w| *w == b"X"));
 }
}

/// DECSC/DECRC does NOT clobber answerback.
/// DECSC saves cursor + active charset + origin-mode
/// only; DECRC restores those fields. Terminal-global config (including
/// answerback) is unaffected.
#[test]
fn decsc_decrc_does_not_clobber_answerback() {
 let mut harness = SpecHarness::new();
 harness.term_mut().set_answerback(b"X".to_vec());
 harness.feed(b"\x1b7"); // DECSC
 harness.term_mut().set_answerback(b"Y".to_vec());
 harness.feed(b"\x1b8\x05"); // DECRC, then ENQ

 let writes: Vec<_> = pty_writes_of_kind(&harness, PtyWriteKind::Answerback).collect();
 assert_eq!(
 writes.len(),
 1,
 "ENQ must emit one Answerback write after DECRC"
 );
 assert_eq!(
 writes[0], b"Y",
 "DECRC must NOT restore answerback to its DECSC-time value — answerback is terminal-global, not saved by DECSC"
 );
}
