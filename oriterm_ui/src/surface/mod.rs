//! Surface abstractions for render strategy, damage tracking, and lifecycle.
//!
//! A *surface* is any drawable area — a terminal window, a dialog, a tooltip.
//! Each surface type declares its [`RenderStrategy`] and tracks pending
//! [`DamageKind`]s via a [`DamageSet`]. The [`SurfaceHost`] trait provides
//! a uniform contract for the event loop to query and consume damage.

/// How a surface renders its content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStrategy {
    /// Terminal grid content with cached base + transient overlays.
    ///
    /// Optimizes for streamed content: the terminal grid is rendered
    /// into a cached texture, overlays (tab bar, search bar, popups)
    /// are drawn on top each frame. Full rebuild only when PTY output
    /// changes the grid.
    TerminalCached,

    /// Retained UI scene with selective subtree rebuild.
    ///
    /// Optimizes for interaction latency: widget tree is cached per-subtree,
    /// only dirty widgets rebuild their draw commands. Used for dialogs,
    /// settings, and future standalone UI windows.
    UiRetained,

    /// Transient scene — rebuilt every frame.
    ///
    /// Used for tooltips, drag previews, and other short-lived visuals
    /// where caching overhead exceeds the cost of full rebuild.
    Transient,
}

/// What kind of change requires a render pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageKind {
    /// Layout changed (widget tree structure, sizes).
    Layout,
    /// Paint changed (colors, opacity, hover state).
    Paint,
    /// Overlay layer changed (popup open/close, tooltip).
    Overlay,
    /// Cursor blink or caret state changed.
    Cursor,
    /// Scroll position changed (transform-only update).
    ScrollTransform,
}

impl DamageKind {
    /// Returns the bit position for this variant.
    const fn bit(self) -> u8 {
        match self {
            Self::Layout => 0,
            Self::Paint => 1,
            Self::Overlay => 2,
            Self::Cursor => 3,
            Self::ScrollTransform => 4,
        }
    }
}

/// Pending damage as a compact bitflag set.
///
/// Since [`DamageKind`] has only 5 variants, a single `u8` represents
/// the full set with no per-frame heap allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DamageSet(u8);

impl DamageSet {
    /// Inserts a damage kind into the set.
    pub fn insert(&mut self, kind: DamageKind) {
        self.0 |= 1 << kind.bit();
    }

    /// Whether the set contains a specific damage kind.
    pub fn contains(self, kind: DamageKind) -> bool {
        self.0 & (1 << kind.bit()) != 0
    }

    /// Whether the set is empty (no pending damage).
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Clears all pending damage.
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Whether the damage requires urgent redraw (layout or paint).
    pub fn is_urgent(self) -> bool {
        self.contains(DamageKind::Layout) || self.contains(DamageKind::Paint)
    }
}

/// Lifecycle state for a secondary surface (dialog, tooltip, panel).
///
/// The framework drives transitions — hosts never skip states. Invalid
/// transitions panic in debug builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceLifecycle {
    /// OS window created, GPU surface configured, but not yet visible.
    /// Content is being built / first frame is being rendered.
    CreatedHidden,

    /// First frame rendered successfully. Ready to become visible.
    /// The framework will show the window on the next event loop tick.
    Primed,

    /// Window is visible and interactive.
    Visible,

    /// Close requested. Window is hidden, input suppressed.
    /// Cleanup (modal release, GPU teardown) is in progress.
    Closing,

    /// Fully destroyed. Context will be removed from the map.
    Destroyed,
}

impl SurfaceLifecycle {
    /// Transitions to a new state, asserting the transition is valid.
    ///
    /// Valid transitions:
    /// - `CreatedHidden → Primed` (first render succeeds)
    /// - `Primed → Visible` (framework shows window)
    /// - `Visible → Closing` (close requested)
    /// - `Closing → Destroyed` (cleanup complete)
    /// - `CreatedHidden → Destroyed` (creation failed, bail out)
    #[must_use]
    pub fn transition(self, to: Self) -> Self {
        debug_assert!(
            self.can_transition_to(to),
            "invalid lifecycle transition: {self:?} → {to:?}",
        );
        to
    }

    /// Whether transitioning from `self` to `to` is valid.
    pub fn can_transition_to(self, to: Self) -> bool {
        matches!(
            (self, to),
            (Self::CreatedHidden, Self::Primed | Self::Destroyed)
                | (Self::Primed, Self::Visible)
                | (Self::Visible, Self::Closing)
                | (Self::Closing, Self::Destroyed)
        )
    }
}

/// Shared contract for any drawable surface.
///
/// Both terminal windows and dialog windows implement this trait,
/// allowing the event loop to treat them uniformly for damage
/// tracking and render scheduling.
pub trait SurfaceHost {
    /// The rendering strategy this surface uses.
    fn render_strategy(&self) -> RenderStrategy;

    /// Record damage that needs rendering.
    fn record_damage(&mut self, damage: DamageKind);

    /// Whether this surface has any pending damage.
    fn has_damage(&self) -> bool;

    /// Consume and return the pending damage kinds, clearing the set.
    fn take_damage(&mut self) -> DamageSet;

    /// The current lifecycle state.
    fn lifecycle(&self) -> SurfaceLifecycle;
}

#[cfg(test)]
mod tests;
