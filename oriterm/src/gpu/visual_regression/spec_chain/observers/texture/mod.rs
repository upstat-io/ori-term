//! Texture render observer (rung 7).
//!
//! Validates properties of the pixel buffer produced by `render_frame_cached()`
//! against the scenario's texture expectations. Receives pre-rendered pixel
//! data from the harness — does not perform GPU rendering itself.

use oriterm_test_support::spec_chain::{RungName, RungResult, TextureExpectation};

/// Rendered pixel data passed from the harness to the texture observer.
pub struct RenderedPixels<'a> {
    pub pixels: &'a [u8],
    pub width: u32,
    pub height: u32,
}

/// Observe the texture-render rung: does the rendered pixel buffer
/// match expected properties?
pub fn observe_texture_render(
    rendered: &RenderedPixels<'_>,
    expected: &TextureExpectation,
) -> RungResult {
    if let Some(w) = expected.width {
        if rendered.width != w {
            return RungResult::fail(
                RungName::TextureRender,
                format!("texture width: expected {w}, got {}", rendered.width),
            );
        }
    }

    if let Some(h) = expected.height {
        if rendered.height != h {
            return RungResult::fail(
                RungName::TextureRender,
                format!("texture height: expected {h}, got {}", rendered.height),
            );
        }
    }

    if let Some(min) = expected.min_non_zero_pixels {
        let count = rendered
            .pixels
            .chunks_exact(4)
            .filter(|rgba| rgba[0] != 0 || rgba[1] != 0 || rgba[2] != 0 || rgba[3] != 0)
            .count();
        if count < min {
            return RungResult::fail(
                RungName::TextureRender,
                format!(
                    "non-zero pixels: expected >= {min}, got {count} \
                     ({}×{} = {} total pixels)",
                    rendered.width,
                    rendered.height,
                    rendered.width as usize * rendered.height as usize,
                ),
            );
        }
    }

    RungResult::pass(RungName::TextureRender)
}

#[cfg(test)]
mod tests;
