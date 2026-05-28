//! Protocol-level encoders that consume `TermMode` state.
//!
//! Owns the encoder bodies for mouse events (§16), focus events
//! (forthcoming under §16.7), and keyboard events (forthcoming under
//! §17 per Decision 10 precedent). App layer constructs semantic
//! events + dispatches into core via `Term::handle_*` methods; core
//! reads `TermMode`, encodes per protocol selection, and emits via
//! `Effect::Pty(PtyEffect::Write { kind, bytes })`.

pub mod mouse;
