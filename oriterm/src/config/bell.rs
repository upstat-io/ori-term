//! Visual bell configuration.

use serde::{Deserialize, Serialize};

/// Visual bell animation curve.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BellAnimation {
    #[default]
    EaseOut,
    Linear,
    None,
}

/// Visual bell configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BellConfig {
    /// Visual bell animation curve.
    pub animation: BellAnimation,
    /// Duration in milliseconds (0 = disabled).
    pub duration_ms: u16,
    /// Flash color as "#RRGGBB" hex (default: white).
    pub color: Option<String>,
}

impl Default for BellConfig {
    fn default() -> Self {
        Self {
            animation: BellAnimation::default(),
            duration_ms: 150,
            color: None,
        }
    }
}

impl BellConfig {
    /// Returns true when the visual bell is enabled.
    #[allow(dead_code, reason = "used in bell rendering (Section 24)")]
    pub fn is_enabled(&self) -> bool {
        self.duration_ms > 0 && self.animation != BellAnimation::None
    }
}
