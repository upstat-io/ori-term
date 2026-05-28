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

/// DEC Locator subsystem state.
#[derive(Debug, Default)]
pub struct DecLocatorState {
    /// Current reporting mode. `None` when DECELR Ps=0 (default,
    /// disabled) OR after a `OneShot` reply auto-clears.
    pub(crate) reporting: Option<LocatorReportingMode>,
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
