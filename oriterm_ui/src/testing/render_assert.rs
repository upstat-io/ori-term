//! DrawList assertion helpers for rendering verification.
//!
//! Provides convenience functions for asserting properties of draw commands
//! produced by widget painting.

use crate::color::Color;
use crate::draw::{DrawCommand, DrawList};

/// Returns the number of draw commands.
pub fn command_count(draw_list: &DrawList) -> usize {
    draw_list.commands().len()
}

/// Returns all rect commands in the draw list.
pub fn rects(draw_list: &DrawList) -> Vec<&DrawCommand> {
    draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::Rect { .. }))
        .collect()
}

/// Returns all text commands in the draw list.
pub fn texts(draw_list: &DrawList) -> Vec<&DrawCommand> {
    draw_list
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::Text { .. }))
        .collect()
}

/// Asserts that the draw list contains a rect with the given fill color.
///
/// # Panics
///
/// Panics if no rect command has the specified fill color.
pub fn assert_has_rect_with_color(draw_list: &DrawList, color: Color) {
    let has = draw_list.commands().iter().any(|c| {
        if let DrawCommand::Rect { style, .. } = c {
            style.fill == Some(color)
        } else {
            false
        }
    });
    assert!(
        has,
        "expected a rect with fill color {color:?}, found none in {} commands",
        draw_list.commands().len()
    );
}

/// Asserts that the draw list contains at least one text command.
///
/// # Panics
///
/// Panics if no text draw command is present.
pub fn assert_has_text(draw_list: &DrawList) {
    let has = draw_list
        .commands()
        .iter()
        .any(|c| matches!(c, DrawCommand::Text { .. }));
    assert!(
        has,
        "expected at least one text command, found none in {} commands",
        draw_list.commands().len()
    );
}
