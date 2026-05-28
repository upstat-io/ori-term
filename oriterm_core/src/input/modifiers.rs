//! Canonical `Modifiers` bitflags type.
//!
//! Shift / Alt / Ctrl / Super bit layout matches xterm parameter
//! encoding (Shift=1, Alt=2, Ctrl=4, Super=8). Keyboard encoders
//! (§17) consume via `xterm_param()` returning `1 + bits` for CSI Pm;
//! mouse encoder (§16) consumes via `mouse_cb_modifier_bits()`
//! returning the additive `+4/+8/+16` form per xterm spec. The two
//! encoding functions are structurally different per spec; the cure
//! is to SHARE THE TYPE, not force one encoder on both.

use bitflags::bitflags;

bitflags! {
    /// Keyboard / mouse modifier state.
    ///
    /// Bit layout matches xterm modifier parameter encoding (Shift=1,
    /// Alt=2, Ctrl=4, Super=8).
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Modifiers: u8 {
        /// Shift key held.
        const SHIFT   = 0b0001;
        /// Alt (Meta/Option) key held.
        const ALT     = 0b0010;
        /// Control key held.
        const CONTROL = 0b0100;
        /// Super (Windows/Command) key held.
        const SUPER   = 0b1000;
    }
}

impl Modifiers {
    /// Encode as xterm modifier parameter (`1 + bitmask`).
    ///
    /// Returns 0 when no modifiers are active (caller omits the parameter).
    /// Consumed by §17 modifyOtherKeys keyboard encoder.
    pub fn xterm_param(self) -> u8 {
        if self.is_empty() { 0 } else { self.bits() + 1 }
    }

    /// Construct from individual shift/alt/ctrl bools (Super=false).
    /// Convenience for test fixtures + boundary-layer construction
    /// where the source value is a (bool, bool, bool) tuple per
    /// the pre-§16.3 `MouseModifiers` shape.
    pub fn from_shift_alt_ctrl(shift: bool, alt: bool, ctrl: bool) -> Self {
        let mut m = Self::empty();
        m.set(Self::SHIFT, shift);
        m.set(Self::ALT, alt);
        m.set(Self::CONTROL, ctrl);
        m
    }
}

#[cfg(test)]
mod tests;
