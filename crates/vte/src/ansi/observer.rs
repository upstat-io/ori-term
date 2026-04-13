//! Observer infrastructure for raw `Perform`-level tuple capture.
//!
//! `PerformObserver` is called by `Processor::advance_with_observer()` for
//! every `Perform` callback BEFORE the callback is dispatched to the `Handler`.
//! This captures the raw parser tuples needed by the spec-conformance harness
//! (rung 1) and the uncataloged-sequence detector (section 04.9).

extern crate alloc;

use alloc::vec::Vec;

/// A raw parser action captured at the `Perform` level.
///
/// Each variant corresponds to a `Perform` trait method and carries the
/// exact parameters the parser produced — before any semantic dispatch
/// by the `Handler`.
#[derive(Debug, Clone, PartialEq)]
pub enum PerformAction {
    /// `Perform::print(c)` — a printable character.
    Print { c: char },
    /// `Perform::execute(byte)` — a C0/C1 control byte.
    Execute { byte: u8 },
    /// `Perform::csi_dispatch(params, intermediates, ignore, action)`.
    CsiDispatch {
        params: Vec<Vec<u16>>,
        intermediates: Vec<u8>,
        ignore: bool,
        action: char,
    },
    /// `Perform::osc_dispatch(params, bell_terminated)`.
    OscDispatch {
        params: Vec<Vec<u8>>,
        bell_terminated: bool,
    },
    /// `Perform::esc_dispatch(intermediates, ignore, byte)`.
    EscDispatch {
        intermediates: Vec<u8>,
        ignore: bool,
        byte: u8,
    },
    /// `Perform::hook(params, intermediates, ignore, action)` — DCS start.
    Hook {
        params: Vec<Vec<u16>>,
        intermediates: Vec<u8>,
        ignore: bool,
        action: char,
    },
    /// `Perform::put(byte)` — DCS data byte.
    Put { byte: u8 },
    /// `Perform::unhook()` — DCS end.
    Unhook,
    /// `Perform::apc_start()`.
    ApcStart,
    /// `Perform::apc_put(byte)`.
    ApcPut { byte: u8 },
    /// `Perform::apc_end()`.
    ApcEnd,
}

/// Trait for observing raw `Perform` callbacks.
///
/// Implementors receive every parser action before it reaches the `Handler`.
/// The default implementations are no-ops so observers can pick which
/// actions they care about.
pub trait PerformObserver {
    /// Called for every raw parser action.
    fn observe(&mut self, _action: &PerformAction) {}
}

/// Collects `PerformAction` entries into a `Vec`.
///
/// The canonical observer for the spec-conformance harness — accumulates
/// all raw parser tuples so they can be inspected after feeding bytes.
#[derive(Debug, Default, Clone)]
pub struct PerformActionCollector {
    actions: Vec<PerformAction>,
}

impl PerformActionCollector {
    /// Create a new empty collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Drain all collected actions into the provided `Vec`.
    pub fn drain_into(&mut self, out: &mut Vec<PerformAction>) {
        out.extend(self.actions.drain(..));
    }

    /// Number of collected actions (for assertions).
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Whether the collector is empty.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

impl PerformObserver for PerformActionCollector {
    fn observe(&mut self, action: &PerformAction) {
        self.actions.push(action.clone());
    }
}

/// No-op observer — zero overhead when observation is not needed.
impl PerformObserver for () {
    fn observe(&mut self, _action: &PerformAction) {}
}
