//! OSC 3 / 5 / 6 / 13 / 14 / 17 / 19 terminal-level color + property state.
//!
//! Holds the state-carrying fields added by Section 10.9 of the
//! spec-conformance plan:
//!
//! - `x11_properties`: OSC 3 X11 property map (`prop=value` sets, bare
//!   `prop` deletes). Bounded by `X11_PROPERTIES_MAX_ENTRIES`.
//! - `tab_title_color`: OSC 6 (iTerm2 tab title color).
//! - `special_colors`: OSC 5 special color slots (0..=4 per xterm).
//! - `mouse_fg_color` / `mouse_bg_color`: OSC 13 / 14.
//! - `highlight_bg_color` / `highlight_fg_color`: OSC 17 / 19.
//!
//! Reset paths (OSC 113 / 114 / 117 / 119) clear the matching
//! `Option<Rgb>` back to `None`.

use std::collections::BTreeMap;

use crate::color::Rgb;
use crate::effect::sink::EffectSink;

use super::Term;

/// Maximum number of X11 properties retained for OSC 3.
///
/// Prevents unbounded RSS growth from adversarial PTY output emitting
/// distinct `OSC 3 ; KEY=value` sequences. Re-inserting an existing key
/// replaces the value in place; new keys at capacity evict the
/// lexicographically first entry (deterministic, easy to reason about).
pub(crate) const X11_PROPERTIES_MAX_ENTRIES: usize = 64;

/// Number of OSC 5 special-color slots.
///
/// Per xterm ctlseqs, Ps ∈ {0: bold, 1: underline, 2: blink, 3: reverse,
/// 4: italics}. Indices ≥ 5 are dropped by the set-handler.
pub(crate) const OSC5_SPECIAL_COLOR_SLOTS: usize = 5;

/// OSC 3 / 5 / 6 / 13 / 14 / 17 / 19 terminal-level state.
#[derive(Debug)]
pub(crate) struct TermColorsState {
    /// OSC 3 X11 window properties (`prop=value` sets, bare `prop` deletes).
    /// Bounded by [`X11_PROPERTIES_MAX_ENTRIES`].
    pub(crate) x11_properties: BTreeMap<String, String>,
    /// OSC 6 tab title color (iTerm2 interpretation).
    pub(crate) tab_title_color: Option<Rgb>,
    /// OSC 5 special-color slots (bold/underline/blink/reverse/italics).
    pub(crate) special_colors: [Option<Rgb>; OSC5_SPECIAL_COLOR_SLOTS],
    /// OSC 13 mouse foreground color.
    pub(crate) mouse_fg_color: Option<Rgb>,
    /// OSC 14 mouse background color.
    pub(crate) mouse_bg_color: Option<Rgb>,
    /// OSC 17 highlight (selection) background color.
    pub(crate) highlight_bg_color: Option<Rgb>,
    /// OSC 19 highlight (selection) foreground color.
    pub(crate) highlight_fg_color: Option<Rgb>,
}

impl TermColorsState {
    pub(crate) fn new() -> Self {
        Self {
            x11_properties: BTreeMap::new(),
            tab_title_color: None,
            special_colors: [None; OSC5_SPECIAL_COLOR_SLOTS],
            mouse_fg_color: None,
            mouse_bg_color: None,
            highlight_bg_color: None,
            highlight_fg_color: None,
        }
    }

    /// Record a `prop=value` pair, enforcing the capacity cap.
    ///
    /// Re-inserting an existing key updates the value in place. New keys
    /// at capacity evict the lexicographically first entry (`BTreeMap`
    /// first key).
    pub(crate) fn set_x11_property(&mut self, name: String, value: String) {
        if !self.x11_properties.contains_key(&name)
            && self.x11_properties.len() >= X11_PROPERTIES_MAX_ENTRIES
        {
            if let Some((first, _)) = self
                .x11_properties
                .iter()
                .next()
                .map(|(k, v)| (k.clone(), v.clone()))
            {
                self.x11_properties.remove(&first);
            }
        }
        self.x11_properties.insert(name, value);
    }

    /// Delete an X11 property (bare `prop` form).
    pub(crate) fn delete_x11_property(&mut self, name: &str) {
        self.x11_properties.remove(name);
    }
}

impl<S: EffectSink> Term<S> {
    /// Current tab-title color (OSC 6). `None` until set.
    pub fn tab_title_color(&self) -> Option<Rgb> {
        self.colors_state.tab_title_color
    }

    /// Current mouse-cursor foreground color (OSC 13). `None` until set.
    pub fn mouse_fg_color(&self) -> Option<Rgb> {
        self.colors_state.mouse_fg_color
    }

    /// Current mouse-cursor background color (OSC 14). `None` until set.
    pub fn mouse_bg_color(&self) -> Option<Rgb> {
        self.colors_state.mouse_bg_color
    }

    /// Current highlight (selection) background color (OSC 17). `None` until set.
    pub fn highlight_bg_color(&self) -> Option<Rgb> {
        self.colors_state.highlight_bg_color
    }

    /// Current highlight (selection) foreground color (OSC 19). `None` until set.
    pub fn highlight_fg_color(&self) -> Option<Rgb> {
        self.colors_state.highlight_fg_color
    }

    /// OSC 5 special-color slot (bold / underline / blink / reverse /
    /// italics). Returns `None` when the slot index is out of range or
    /// no color has been set.
    pub fn special_color(&self, index: usize) -> Option<Rgb> {
        self.colors_state
            .special_colors
            .get(index)
            .copied()
            .flatten()
    }

    /// Value of an OSC 3 X11 property. `None` if the property was never
    /// set or was deleted by the bare-`prop` form.
    pub fn x11_property(&self, name: &str) -> Option<&str> {
        self.colors_state
            .x11_properties
            .get(name)
            .map(String::as_str)
    }

    /// Number of X11 properties currently retained.
    pub fn x11_properties_len(&self) -> usize {
        self.colors_state.x11_properties.len()
    }
}
