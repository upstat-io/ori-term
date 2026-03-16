//! Widget interaction sense flags.
//!
//! A `Sense` declares what interactions a widget cares about. Hit testing
//! skips widgets with `Sense::none()`, and the event routing layer uses
//! sense flags to decide which events to deliver.
//!
//! This is a leaf module with zero intra-crate imports, kept separate to
//! avoid circular dependencies (`layout` -> `interaction` -> `layout`).

/// Declares what interactions a widget cares about.
///
/// A bitflag set: widgets compose flags via `union()`. Hit testing
/// skips widgets with `Sense::none()`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sense(u8);

impl Sense {
    const HOVER_BIT: u8 = 0b0001;
    const CLICK_BIT: u8 = 0b0010;
    const DRAG_BIT: u8 = 0b0100;
    const FOCUS_BIT: u8 = 0b1000;

    /// No interactions — invisible to hit testing.
    pub const fn none() -> Self {
        Self(0)
    }

    /// Hover tracking only.
    pub const fn hover() -> Self {
        Self(Self::HOVER_BIT)
    }

    /// Click events (implies hover).
    pub const fn click() -> Self {
        Self(Self::HOVER_BIT | Self::CLICK_BIT)
    }

    /// Drag events (implies hover).
    pub const fn drag() -> Self {
        Self(Self::HOVER_BIT | Self::DRAG_BIT)
    }

    /// Click and drag (implies hover).
    pub const fn click_and_drag() -> Self {
        Self(Self::HOVER_BIT | Self::CLICK_BIT | Self::DRAG_BIT)
    }

    /// Keyboard focus only (no hover/click/drag).
    pub const fn focusable() -> Self {
        Self(Self::FOCUS_BIT)
    }

    /// All interactions.
    pub const fn all() -> Self {
        Self(Self::HOVER_BIT | Self::CLICK_BIT | Self::DRAG_BIT | Self::FOCUS_BIT)
    }

    /// Combines two sense sets (bitwise OR).
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether this sense set is empty (no interactions declared).
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }

    /// Whether hover tracking is enabled.
    pub const fn has_hover(self) -> bool {
        self.0 & Self::HOVER_BIT != 0
    }

    /// Whether click events are enabled.
    pub const fn has_click(self) -> bool {
        self.0 & Self::CLICK_BIT != 0
    }

    /// Whether drag events are enabled.
    pub const fn has_drag(self) -> bool {
        self.0 & Self::DRAG_BIT != 0
    }

    /// Whether keyboard focus is enabled.
    pub const fn has_focus(self) -> bool {
        self.0 & Self::FOCUS_BIT != 0
    }
}

impl Default for Sense {
    fn default() -> Self {
        Self::none()
    }
}

impl std::fmt::Debug for Sense {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_none() {
            return f.write_str("Sense(none)");
        }
        let mut parts = Vec::new();
        if self.has_hover() {
            parts.push("HOVER");
        }
        if self.has_click() {
            parts.push("CLICK");
        }
        if self.has_drag() {
            parts.push("DRAG");
        }
        if self.has_focus() {
            parts.push("FOCUS");
        }
        write!(f, "Sense({})", parts.join(" | "))
    }
}

#[cfg(test)]
mod tests;
