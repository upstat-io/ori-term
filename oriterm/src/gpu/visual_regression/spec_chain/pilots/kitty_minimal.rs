//! Kitty graphics visual pilot — minimal RGBA upload through every rung.
//!
//! Apex: `GoldenImage`
//!
//! Drives the smallest possible kitty graphics transmit+place command
//! (1×1 solid red RGBA pixel via `f=32,s=1,v=1,a=T`) through every
//! visual rung. This is the kitty-graphics counterpart to
//! `sixel_minimal_drives_every_rung_green` and establishes that the
//! GPU pipeline can render a kitty image end-to-end on the platform's
//! native graphics backend.
//!
//! See: 
//!
//! **Why this matters for the wordmark gap.** The four prior Phase 1
//! layers (handler / Term snapshot / daemon fold / client extract) all
//! pass on Linux with real `notcurses-info` bytes. The remaining
//! untested layer is the GPU pipeline — `ensure_uploaded` → bind-group
//! creation → `draw_image_quads`. If this pilot fails (any rung), the
//! cure surface IS the GPU pipeline. If it passes, the GPU pipeline
//! works on Linux for kitty graphics and the Windows-only symptom
//! needs a platform-specific narrowing (DX12 backend, ConPTY reader,
//! or pane-exit timing race).

use oriterm_test_support::spec_chain::{
 ApexLayer, FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, RungName,
 ScenarioExpectations, SpecScenario, TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// Minimal kitty graphics transmit+place sequence.
/// Breakdown:
/// - `\x1b_G` — APC introducer for kitty graphics
/// - `f=32` — pixel format = 32-bit RGBA
/// - `s=1,v=1` — source image is 1 pixel wide, 1 pixel tall
/// - `a=T` — action = transmit-and-place
/// - `i=1` — image ID = 1
/// - `q=2` — suppress all responses (quiet mode)
/// - `;/wAA/w==` — base64 payload for one RGBA red pixel (FF 00 00 FF)
/// - `\x1b\\` — ST (String Terminator)
/// Payload provenance: `/wAA/w==` decodes via standard base64 to the
/// four bytes `0xFF 0x00 0x00 0xFF` — opaque red in RGBA8.
const KITTY_BYTES: &[u8] = b"\x1b_Gf=32,s=1,v=1,a=T,i=1,q=2;/wAA/w==\x1b\\";

/// Pilot test: drives the minimal kitty RGBA transmit+place through every rung.
/// Uses the deterministic golden lane (Section 05) via `VisualSpecHarness`.
/// The golden image is captured with `ORITERM_UPDATE_GOLDEN=1` and
/// committed under `oriterm/tests/references/`.
#[test]
fn kitty_minimal_drives_every_rung_green() {
 let Some(mut harness) = VisualSpecHarness::new() else {
 eprintln!("SKIP: software rasterizer unavailable");
 return;
 };

 let scenario = SpecScenario {
 catalog_row_id: "KG-ACTION-TRANSMIT-AND-PLACE",
 bytes: KITTY_BYTES,
 apex_layer: ApexLayer::GoldenImage,
 setup: b"",
 expectations: ScenarioExpectations {
 // Rungs 5 / 6 / 7 are the load-bearing assertions for the
 // wordmark cure surface: FrameInput must carry the placement,
 // GpuInstance must carry one image quad, texture buffer must
 // be non-empty. Parser-rung expectation omitted because there
 // is no APC-specific `ParserExpectation` constructor yet.
 frame_input: Some(FrameInputExpectation::default_grid()),
 gpu_instance: Some(GpuInstanceExpectation::at_least(1, 0).with_images(1)),
 texture: Some(TextureExpectation {
 min_non_zero_pixels: Some(1),
 width: None,
 height: None,
 }),
 golden: Some(GoldenExpectation {
 golden_name: Some("kitty_minimal"),
 }),
 ..ScenarioExpectations::default()
 },
 };

 let results = harness.run_visual_scenario(&scenario);

 for r in &results {
 assert!(
 r.passed,
 "rung {:?} failed: {}",
 r.rung_name,
 r.failure.as_deref().unwrap_or("(no message)")
 );
 }

 let rung_names: Vec<_> = results.iter().map(|r| r.rung_name).collect();
 assert_eq!(
 rung_names.len(),
 8,
 "GoldenImage apex should produce exactly 8 rung results, got: {rung_names:?}"
 );
 assert_eq!(
 *rung_names.last().unwrap(),
 RungName::GoldenImage,
 "last rung should be GoldenImage"
 );
}
