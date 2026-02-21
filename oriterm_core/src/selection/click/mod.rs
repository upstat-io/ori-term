//! Multi-click detection for selection mode selection.
//!
//! Tracks click timing and position to cycle through selection modes:
//! single click (Char), double click (Word), triple click (Line).

use std::time::{Duration, Instant};

/// Time window for consecutive clicks to count as multi-click.
const MULTI_CLICK_THRESHOLD: Duration = Duration::from_millis(500);

/// Tracks click state for multi-click detection.
///
/// Rapid clicks in the same cell position cycle through selection modes:
/// 1 (Char) → 2 (Word) → 3 (Line) → 1 (reset). Clicks at different
/// positions or outside the time window reset to 1.
#[derive(Debug)]
pub struct ClickDetector {
    /// Timestamp of the last click.
    last_time: Option<Instant>,
    /// Cell position (col, row) of the last click.
    last_pos: Option<(usize, usize)>,
    /// Current click count in the cycle (1, 2, or 3).
    count: u8,
}

impl ClickDetector {
    /// Create a new click detector with no prior state.
    pub fn new() -> Self {
        Self {
            last_time: None,
            last_pos: None,
            count: 0,
        }
    }

    /// Register a click and return the resulting click count (1, 2, or 3).
    ///
    /// Returns 1 for single click (Char selection), 2 for double click
    /// (Word selection), 3 for triple click (Line selection). The fourth
    /// rapid click resets to 1.
    pub fn click(&mut self, col: usize, row: usize) -> u8 {
        let now = Instant::now();
        let same_pos = self.last_pos == Some((col, row));
        let within_time = self
            .last_time
            .is_some_and(|t| now.duration_since(t) < MULTI_CLICK_THRESHOLD);

        let count = if same_pos && within_time {
            match self.count {
                1 => 2,
                2 => 3,
                _ => 1,
            }
        } else {
            1
        };

        self.last_time = Some(now);
        self.last_pos = Some((col, row));
        self.count = count;
        count
    }

    /// Reset all click state.
    ///
    /// Call when something external invalidates the click sequence (e.g.
    /// focus loss, selection cleared by output).
    pub fn reset(&mut self) {
        self.last_time = None;
        self.last_pos = None;
        self.count = 0;
    }
}

impl Default for ClickDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
