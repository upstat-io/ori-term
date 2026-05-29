//! DEC Locator subsystem state.
//!
//! Mouse/locator extension protocol activated by DECELR
//! (`CSI Ps;Pu ' z`), independent of any DECSET mode. NOT mode 1001
//! (which is xterm's `SET_VT200_HIGHLIGHT_MOUSE` highlight tracking,
//! a separate protocol per the spec-conformance §16.1 F1 cure). State
//! mutated by 4 CSI sequences:
//!
//! - DECEFR (`' w`) — Enable Filter Rectangle; sets `filter_rect`.
//! - DECELR (`' z`) — Enable Locator Reporting; sets `reporting` +
//!   `pixel_unit`. Ps=0 disables (clears `reporting` to `None`);
//!   Ps=1 = `Some(Continuous)`; Ps=2 = `Some(OneShot)`.
//! - DECSLE (`' {`) — Select Locator Events; sets `event_mask`.
//! - DECRQLP (`' |`) — Request Locator Position; emits DECLRP reply.
//!   In `OneShot` mode, DECRQLP auto-clears `reporting` to `None`
//!   after the reply per xterm spec.

use bitflags::bitflags;

/// Locator reporting mode per DECELR Ps parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocatorReportingMode {
    /// DECELR Ps=1 — report every event matching `event_mask`.
    Continuous,
    /// DECELR Ps=2 — report next event then auto-clear `reporting`.
    OneShot,
}

bitflags! {
    /// Event-class bitmask per DECSLE Pm parameter.
    ///
    /// Each `Ps` value in the DECSLE `Pm` parameter list maps to a
    /// bit; the set of bits determines which event classes trigger a
    /// DECLRP reply. Default `EXPLICIT_ONLY` matches DECSLE Pm=0
    /// (only respond to DECRQLP).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LocatorEventMask: u8 {
        /// Pm=0 — only respond to explicit DECRQLP requests.
        const EXPLICIT_ONLY = 1 << 0;
        /// Pm=1 — report button-down events.
        const BUTTON_DOWN = 1 << 1;
        /// Pm=2 — do not report button-down events.
        const BUTTON_DOWN_OFF = 1 << 2;
        /// Pm=3 — report button-up events.
        const BUTTON_UP = 1 << 3;
        /// Pm=4 — do not report button-up events.
        const BUTTON_UP_OFF = 1 << 4;
    }
}

impl Default for LocatorEventMask {
    fn default() -> Self {
        Self::EXPLICIT_ONLY
    }
}

/// Filter rectangle per DECEFR Pt;Pl;Pb;Pr parameters.
///
/// Locator events outside the rectangle generate an
/// outside-rectangle event AND auto-clear the rectangle to one-shot
/// per xterm spec (DECEFR rectangles are inherently one-shot).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocatorRect {
    pub top: u16,
    pub left: u16,
    pub bottom: u16,
    pub right: u16,
}

/// Last-observed locator position for DECLRP reply composition.
///
/// `Unavailable` is the default (no observation yet) AND the
/// cursor-out-of-range state — both emit DECLRP Pe=0 with one
/// parameter per xterm `button.c:857-861`. `Known` carries cell
/// coords (Pu=0 reports `cell + 1`), DEVICE PHYSICAL pixel coords
/// (Pu=1 reports `pixel + 1` — distinct from SGR-Pixel logical px),
/// and the Pm button bitmask (`4*left + 2*middle + 1*right` per the
/// `button.c:944-948` Button1/Button3 swap).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocatorPosition {
    /// No observation yet OR cursor out of locator range → DECLRP Pe=0.
    #[default]
    Unavailable,
    /// Observed locator position; DECRQLP reads this for the reply.
    Known {
        /// `(col, line)` cell coords, 0-indexed; DECLRP Pu=0 reports `cell + 1`.
        cell: (u32, u32),
        /// `(px, py)` DEVICE physical pixels; DECLRP Pu=1 reports `pixel + 1`.
        pixel: (u32, u32),
        /// Pm button bitmask: `4*left + 2*middle + 1*right` (button.c:944-948 swap).
        buttons: u16,
    },
}

/// DEC Locator subsystem state.
#[derive(Debug, Default)]
pub struct DecLocatorState {
    /// Current reporting mode. `None` when DECELR Ps=0 (default,
    /// disabled) OR after a `OneShot` reply auto-clears.
    pub(crate) reporting: Option<LocatorReportingMode>,
    /// Last-observed locator position (cell + pixel coords + button
    /// mask). `Unavailable` until `handle_mouse_input` observes an
    /// event while reporting is active.
    pub(crate) position: LocatorPosition,
    /// Event-class bitmask set by DECSLE. Defaults to `EXPLICIT_ONLY`.
    pub(crate) event_mask: LocatorEventMask,
    /// Optional filter rectangle set by DECEFR. `None` when no
    /// rectangle is active (default OR after all-zeros DECEFR).
    pub(crate) filter_rect: Option<LocatorRect>,
    /// Coordinate unit per DECELR Pu: `true` = pixels (Pu=1),
    /// `false` = character cells (Pu=0 or Pu=2; default).
    pub(crate) pixel_unit: bool,
}

impl DecLocatorState {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Current reporting mode. `None` when disabled (default) OR after a
    /// `OneShot` reply auto-cleared. Read-only public accessor for tests
    /// and downstream emission gating.
    pub fn reporting(&self) -> Option<LocatorReportingMode> {
        self.reporting
    }

    /// Coordinate unit per DECELR Pu: `true` = pixels, `false` = cells.
    pub fn pixel_unit(&self) -> bool {
        self.pixel_unit
    }

    /// Event-class bitmask set by DECSLE.
    pub fn event_mask(&self) -> LocatorEventMask {
        self.event_mask
    }

    /// Optional filter rectangle set by DECEFR.
    pub fn filter_rect(&self) -> Option<LocatorRect> {
        self.filter_rect
    }

    /// Last-observed locator position. `Unavailable` until an event is
    /// observed while reporting is active.
    pub fn position(&self) -> LocatorPosition {
        self.position
    }

    /// Pm button bitmask of the last-observed position (0 when `Unavailable`).
    pub fn buttons(&self) -> u16 {
        match self.position {
            LocatorPosition::Known { buttons, .. } => buttons,
            LocatorPosition::Unavailable => 0,
        }
    }

    /// Record an observed locator position — cell coords, physical pixel
    /// coords, and the Pm button mask. Called by
    /// `Term::handle_mouse_input` Step A when `reporting().is_some()`.
    ///
    /// Stored as `Unavailable` (→ DECRQLP Pe=0) when EITHER:
    /// - `cell` is `None` — cursor out of grid range, OR
    /// - `pixel_unit` (DECELR Pu=1) is active AND `pixel` is `None` —
    ///   fabricating `(0, 0)` would misreport a valid origin coordinate
    ///   per xterm `button.c:857-861` (locator-unavailable, not origin).
    ///
    /// In Pu=0 (cell) mode an absent `pixel` is harmless — the pixel
    /// field is never read by `compose_declrp_reply` — so it stores
    /// `Known` with a `(0, 0)` placeholder pixel.
    pub(crate) fn observe(
        &mut self,
        cell: Option<(u32, u32)>,
        pixel: Option<(u32, u32)>,
        buttons: u16,
    ) {
        self.position = match (cell, self.pixel_unit, pixel) {
            (None, _, _) | (_, true, None) => LocatorPosition::Unavailable,
            (Some(cell), _, pixel) => LocatorPosition::Known {
                cell,
                pixel: pixel.unwrap_or((0, 0)),
                buttons,
            },
        };
    }

    /// Apply DECELR — Enable Locator Reporting.
    ///
    /// Ps=0 → disabled (clears `reporting` to `None`). Ps=1 → Continuous.
    /// Ps=2 → `OneShot`. Other values silently default to disabled per
    /// xterm spec.
    pub(crate) fn apply_decelr(&mut self, ps: u16, pu: u16) {
        self.reporting = match ps {
            1 => Some(LocatorReportingMode::Continuous),
            2 => Some(LocatorReportingMode::OneShot),
            _ => None,
        };
        self.pixel_unit = pu == 1;
        // Reset the observed position on any DECELR: re-enabling after a
        // disable, or switching coordinate unit, invalidates the prior
        // observation (its coords were captured under the old reporting /
        // unit state). A DECRQLP before a fresh event then emits Pe=0
        // rather than reporting a stale position.
        self.position = LocatorPosition::Unavailable;
    }

    /// Apply DECEFR — Enable Filter Rectangle.
    ///
    /// All-zeros (or any all-default parameters) clears the
    /// rectangle. Per xterm spec, omitted parameters default to the
    /// current locator position; the parser passes 0 for omitted
    /// params and the handler interprets this as "clear rectangle"
    /// in the absence of a known locator position.
    pub(crate) fn apply_decefr(&mut self, pt: u16, pl: u16, pb: u16, pr: u16) {
        if pt == 0 && pl == 0 && pb == 0 && pr == 0 {
            self.filter_rect = None;
        } else {
            self.filter_rect = Some(LocatorRect {
                top: pt,
                left: pl,
                bottom: pb,
                right: pr,
            });
        }
    }

    /// Apply DECSLE — Select Locator Events.
    ///
    /// Maps each Ps value in the Pm list to its `LocatorEventMask`
    /// bit. An empty list defaults to `EXPLICIT_ONLY` per xterm spec.
    pub(crate) fn apply_decsle(&mut self, events: &[u16]) {
        if events.is_empty() {
            self.event_mask = LocatorEventMask::EXPLICIT_ONLY;
            return;
        }
        let mut mask = LocatorEventMask::empty();
        for &ps in events {
            match ps {
                0 => mask |= LocatorEventMask::EXPLICIT_ONLY,
                1 => mask |= LocatorEventMask::BUTTON_DOWN,
                2 => mask |= LocatorEventMask::BUTTON_DOWN_OFF,
                3 => mask |= LocatorEventMask::BUTTON_UP,
                4 => mask |= LocatorEventMask::BUTTON_UP_OFF,
                _ => {} // unknown event class — silently ignore.
            }
        }
        self.event_mask = mask;
    }

    /// Acknowledge DECRQLP — auto-clear `reporting` if `OneShot`.
    ///
    /// Per xterm spec, `OneShot` reporting clears after the response
    /// fires. Called by the handler AFTER it emits the DECLRP reply
    /// (or — until §16.1.C lands — after the reply WOULD have been
    /// emitted; the state transition is the verifiable half).
    pub(crate) fn on_decrqlp_acknowledged(&mut self) {
        if self.reporting == Some(LocatorReportingMode::OneShot) {
            self.reporting = None;
        }
    }
}

#[cfg(test)]
mod tests;
