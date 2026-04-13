//! Frame input observer (rung 5).
//!
//! Asserts that the `FrameInput` assembled from terminal state matches
//! the scenario's expectations for grid dimensions, cursor visibility,
//! and mode flags.

use oriterm_test_support::spec_chain::RungResult;
use oriterm_test_support::spec_chain::{FrameInputExpectation, RungName};

use crate::gpu::frame_input::FrameInput;

/// Observe the frame-input rung: does `FrameInput` match expected?
pub fn observe_frame_input(input: &FrameInput, expected: &FrameInputExpectation) -> RungResult {
    if let Some(cols) = expected.cols {
        if input.columns() != cols {
            return RungResult::fail(
                RungName::FrameInput,
                format!(
                    "FrameInput columns: expected {cols}, got {}",
                    input.columns()
                ),
            );
        }
    }

    if let Some(rows) = expected.rows {
        if input.rows() != rows {
            return RungResult::fail(
                RungName::FrameInput,
                format!("FrameInput rows: expected {rows}, got {}", input.rows()),
            );
        }
    }

    if let Some(cursor_visible) = expected.cursor_visible {
        if input.content.cursor.visible != cursor_visible {
            return RungResult::fail(
                RungName::FrameInput,
                format!(
                    "FrameInput cursor_visible: expected {cursor_visible}, got {}",
                    input.content.cursor.visible
                ),
            );
        }
    }

    if let Some(reverse_video) = expected.reverse_video {
        if input.reverse_video != reverse_video {
            return RungResult::fail(
                RungName::FrameInput,
                format!(
                    "FrameInput reverse_video: expected {reverse_video}, got {}",
                    input.reverse_video
                ),
            );
        }
    }

    RungResult::pass(RungName::FrameInput)
}
