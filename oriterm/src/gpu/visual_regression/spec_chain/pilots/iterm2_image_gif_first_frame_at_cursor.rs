//! iTerm2 inline image visual pilot — single-frame GIF fall-through.
//!
//! Catalog row: `ITERM2-1337-FILE-GIF`
//! Apex: `GoldenImage`
//!
//! Drives `OSC 1337 ; File=inline=1:<16x16-single-frame-gif-b64> ST`
//! through every visual rung. A single-frame GIF is not animatable, so
//! `decode_gif_frames` returns `None` and the handler routes the payload
//! through the static `decode_to_rgba` path. Pairs with the state-snapshot
//! pin `osc1337_file_single_frame_gif_takes_static_path` per the §14.4
//! dual-gate.

use super::iterm2_gif_pilot::run_gif_rung8_pilot;

/// `OSC 1337 ; File=inline=1:<b64> ST` with the b64 of a 16×16 single-frame
/// GIF produced by the `image` crate. Single-frame GIFs take the static
/// `decode_to_rgba` fall-through (the animated branch is skipped because
/// `decode_gif_frames` returns `None` for frame count ≤ 1). The
/// `expected_animated_frames: None` arg pins this fall-through against
/// drift via `assert_payload_decodes`.
const ITERM2_GIF_BYTES: &[u8] = b"\x1b]1337;File=inline=1:R0lGODlhEAAQAIAAAAAAAAAAACH/C05FVFNDQVBFMi4wAwEAAAAh+QQICgAAACwAAAAAEAAQAIcAAAAAAAsAABYAACEAACwAADcAAEIAAE0AAFgAAGMAAG4AAHkAAIQAAI8AAJoAAKUABwAABwsABxYAByEABywABzcAB0IAB00AB1gAB2MAB24AB3kAB4QAB48AB5oAB6UADgAADgsADhYADiEADiwADjcADkIADk0ADlgADmMADm4ADnkADoQADo8ADpoADqUAFQAAFQsAFRYAFSEAFSwAFTcAFUIAFU0AFVgAFWMAFW4AFXkAFYQAFY8AFZoAFaUAHAAAHAsAHBYAHCEAHCwAHDcAHEIAHE0AHFgAHGMAHG4AHHkAHIQAHI8AHJoAHKUAIwAAIwsAIxYAIyEAIywAIzcAI0IAI00AI1gAI2MAI24AI3kAI4QAI48AI5oAI6UAKgAAKgsAKhYAKiEAKiwAKjcAKkIAKk0AKlgAKmMAKm4AKnkAKoQAKo8AKpoAKqUAMQAAMQsAMRYAMSEAMSwAMTcAMUIAMU0AMVgAMWMAMW4AMXkAMYQAMY8AMZoAMaUAOAAAOAsAOBYAOCEAOCwAODcAOEIAOE0AOFgAOGMAOG4AOHkAOIQAOI8AOJoAOKUAPwAAPwsAPxYAPyEAPywAPzcAP0IAP00AP1gAP2MAP24AP3kAP4QAP48AP5oAP6UARgAARgsARhYARiEARiwARjcARkIARk0ARlgARmMARm4ARnkARoQARo8ARpoARqUATQAATQsATRYATSEATSwATTcATUIATU0ATVgATWMATW4ATXkATYQATY8ATZoATaUAVAAAVAsAVBYAVCEAVCwAVDcAVEIAVE0AVFgAVGMAVG4AVHkAVIQAVI8AVJoAVKUAWwAAWwsAWxYAWyEAWywAWzcAW0IAW00AW1gAW2MAW24AW3kAW4QAW48AW5oAW6UAYgAAYgsAYhYAYiEAYiwAYjcAYkIAYk0AYlgAYmMAYm4AYnkAYoQAYo8AYpoAYqUAaQAAaQsAaRYAaSEAaSwAaTcAaUIAaU0AaVgAaWMAaW4AaXkAaYQAaY8AaZoAaaUI/wABQAABAwgUMHAAQQIFCxg0cPACRAgRI0iUMHECRQoVK1i0cPEESBAhQ4gUMXIESRIlS5g0cfIGTBgxY8iUMXMGTRo1a9i0cfMIUCBBgwgVMnQIUSJFixg1cvQKVChRo0iVMnUKVSpVq1i1cvUMWDBhw4gVM3YMWTJly5g1c/YOXDhx48iVM3cOXTp169i1c/cQYECBAwkWNHgQYUKFCxk2dPgSZEiRI0mWNHkSZUqVK1m2dPkUaFChQ4kWNXoUaVKlS5k2dfoWbFixY8mWNXsWbVq1a9m2dfsYcGDBgwkXNnwYcWLFixk3dvwadGjRo0mXNn0adWrVq1m3dv0ceCRw4cOJFzd+HHly5cuZN3f+Hnx48ePJlzd/Hn169evZt3f/BAQAOw==\x1b\\";

/// Drives a single-frame GIF OSC payload through every visual rung and
/// captures the golden. `expected_animated_frames: None` pins (via the
/// shared driver's decode-guard) that the payload takes the static
/// `decode_to_rgba` fall-through. Catalog row `ITERM2-1337-FILE-GIF`.
#[test]
fn iterm2_image_gif_first_frame_at_cursor_drives_every_rung_green() {
    run_gif_rung8_pilot(
        "ITERM2-1337-FILE-GIF",
        ITERM2_GIF_BYTES,
        "iterm2_image_gif_first_frame_at_cursor",
        None,
    );
}
