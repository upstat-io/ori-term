//! Shared driver + drift-guard for the iTerm2 GIF GPU golden pilots.
//!
//! Both GIF pilots (`iterm2_image_gif_first_frame_at_cursor` and
//! `iterm2_image_animated_gif_first_frame`) embed a hand-pasted base64
//! GIF payload as a `&'static [u8]` OSC sequence (`SpecScenario::bytes`
//! requires `'static`, so a runtime-built fixture cannot be passed
//! directly). `assert_payload_decodes` pins those const bytes against
//! silent drift from the dynamic `animated_gif_bytes` fixture generator:
//! it decodes the embedded payload and asserts the frame structure the
//! pilot's catalog row depends on.

use base64::Engine as _;
use oriterm_core::image::{ImageFormat, decode_gif_frames, detect_format};
use oriterm_test_support::spec_chain::{
    ApexLayer, FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, RungName,
    ScenarioExpectations, SpecScenario, TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

const OSC_PREFIX: &[u8] = b"\x1b]1337;File=inline=1:";
const OSC_SUFFIX: &[u8] = b"\x1b\\";

/// Decode-guard: the embedded OSC payload's GIF must decode to the frame
/// structure the catalog row claims. `expected_animated_frames`:
/// `Some(n)` — the payload must promote to an `n`-frame animation;
/// `None` — single-frame GIF that must take the static fall-through
/// (`decode_gif_frames` returns `None`). Runs BEFORE the GPU SKIP gate so
/// the drift pin holds on every platform.
fn assert_payload_decodes(osc_bytes: &[u8], expected_animated_frames: Option<usize>) {
    let b64 = osc_bytes
        .strip_prefix(OSC_PREFIX)
        .and_then(|b| b.strip_suffix(OSC_SUFFIX))
        .expect("pilot bytes must be an OSC 1337 File=inline=1 sequence");
    let gif = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .expect("pilot payload must be valid base64");
    assert_eq!(
        detect_format(&gif),
        Some(ImageFormat::Gif),
        "pilot payload must be a GIF"
    );
    match expected_animated_frames {
        Some(n) => {
            let frames = decode_gif_frames(&gif, usize::MAX)
                .expect("animated pilot payload must decode as a multi-frame GIF");
            assert_eq!(
                frames.frames.len(),
                n,
                "animated pilot payload must carry {n} frames"
            );
        }
        None => assert!(
            decode_gif_frames(&gif, usize::MAX).is_none(),
            "single-frame pilot payload must take the static fall-through (decode_gif_frames None)"
        ),
    }
}

/// Drive a GIF OSC payload through every visual rung (1-8) and assert each
/// passes, capturing the golden at `oriterm/tests/references/<golden_name>.png`
/// via `ORITERM_UPDATE_GOLDEN=1`. Shared by both GIF pilots.
pub(crate) fn run_gif_rung8_pilot(
    catalog_row_id: &'static str,
    osc_bytes: &'static [u8],
    golden_name: &'static str,
    expected_animated_frames: Option<usize>,
) {
    assert_payload_decodes(osc_bytes, expected_animated_frames);

    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id,
        bytes: osc_bytes,
        apex_layer: ApexLayer::GoldenImage,
        setup: b"",
        expectations: ScenarioExpectations {
            frame_input: Some(FrameInputExpectation::default_grid()),
            gpu_instance: Some(GpuInstanceExpectation::at_least(1, 0).with_images(1)),
            // `min_non_zero_pixels` is a whole-texture smoke check: the
            // metric counts the full grid (non-zero background), so it
            // cannot pin the image footprint. The GoldenImage rung is the
            // exact pixel gate; `assert_payload_decodes` pins the source GIF.
            texture: Some(TextureExpectation {
                min_non_zero_pixels: Some(1),
                width: None,
                height: None,
            }),
            golden: Some(GoldenExpectation {
                golden_name: Some(golden_name),
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
