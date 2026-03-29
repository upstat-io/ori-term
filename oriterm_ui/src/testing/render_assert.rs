//! Scene assertion helpers for rendering verification.
//!
//! Provides convenience functions for asserting properties of draw primitives
//! produced by widget painting.

use crate::color::Color;
use crate::draw::Scene;

/// Returns the total number of primitives.
pub fn command_count(scene: &Scene) -> usize {
    scene.len()
}

/// Returns all quad primitives in the scene.
pub fn rects(scene: &Scene) -> Vec<&crate::draw::Quad> {
    scene.quads().iter().collect()
}

/// Returns all text run primitives in the scene.
pub fn texts(scene: &Scene) -> Vec<&crate::draw::TextRun> {
    scene.text_runs().iter().collect()
}

/// Asserts that the scene contains a quad with the given fill color.
///
/// # Panics
///
/// Panics if no quad has the specified fill color.
pub fn assert_has_rect_with_color(scene: &Scene, color: Color) {
    let has = scene.quads().iter().any(|q| q.style.fill == Some(color));
    assert!(
        has,
        "expected a quad with fill color {color:?}, found none in {} primitives",
        scene.len()
    );
}

/// Asserts that the scene contains at least one text run.
///
/// # Panics
///
/// Panics if no text run is present.
pub fn assert_has_text(scene: &Scene) {
    assert!(
        !scene.text_runs().is_empty(),
        "expected at least one text run, found none in {} primitives",
        scene.len()
    );
}
