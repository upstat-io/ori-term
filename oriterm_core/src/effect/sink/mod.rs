//! Effect sink trait and concrete implementations.
//!
//! The `EffectSink` is the production-side interface that the VTE handler
//! emits to. `Term<S: EffectSink>` is statically dispatched (monomorphized)
//! — zero vtable overhead on the handler hot path.

pub mod legacy;

use super::Effect;

pub use legacy::LegacyEventSink;

/// Receives terminal effects from the VTE handler.
///
/// # Contract
///
/// - `push()` accepts an effect. Implementations may either queue it for
///   later retrieval via `drain_into()`, or forward it immediately to a
///   downstream consumer (as `LegacyEventSink` does).
/// - `drain_into()` appends effects that have NOT already been forwarded.
///   For queuing sinks, this drains the queue into the provided Vec.
///   For immediate-forward sinks, this is a no-op — the effects were
///   already delivered via `push()`.
/// - Consumers that need guaranteed deferred-then-bulk-drain semantics
///   MUST use `QueueingEffectSink` (or a type that documents those semantics).
///   Do NOT assume all `EffectSink` impls queue.
///
/// # Ordering
///
/// Effects pushed via `push()` are ordered relative to each other:
/// if A is pushed before B, A appears before B in `drain_into()`.
/// Effects are NOT ordered relative to state changes — an effect
/// pushed during VTE handling may be drained before or after the
/// next snapshot publication. Consumers that need to correlate
/// effects with state must use `PresentationEffect::Commit`
/// which carries the `snapshot_seqno` at the time of commit.
/// This is the ONLY synchronization point between the effect
/// stream and the state stream.
///
/// # Thread safety
///
/// `Send + Sync` is required because the IO thread pushes effects and the
/// main thread may drain them.
pub trait EffectSink: Send + Sync {
    /// Push an effect onto the sink.
    fn push(&self, effect: Effect);

    /// Drain all pending effects that have not been forwarded.
    ///
    /// Callers should reuse a `Vec<Effect>` across calls via
    /// `drain_into()` to avoid per-drain allocation.
    fn drain_into(&self, out: &mut Vec<Effect>);
}

/// Default thread-safe queue-backed sink.
///
/// Effects accumulate in an internal queue and are retrieved in bulk
/// via `drain_into()`. The queue retains capacity across drain cycles
/// to avoid repeated allocation.
#[derive(Debug, Default)]
pub struct QueueingEffectSink {
    queue: parking_lot::Mutex<Vec<Effect>>,
}

impl QueueingEffectSink {
    /// Create a new empty queuing sink.
    pub fn new() -> Self {
        Self::default()
    }
}

impl EffectSink for QueueingEffectSink {
    fn push(&self, effect: Effect) {
        self.queue.lock().push(effect);
    }

    fn drain_into(&self, out: &mut Vec<Effect>) {
        let mut q = self.queue.lock();
        out.extend(q.drain(..));
        // Capacity stays in `q` for reuse on next push cycle.
    }
}

/// No-op sink used for tests that don't observe effects.
#[derive(Debug, Default, Clone, Copy)]
pub struct VoidEffectSink;

impl EffectSink for VoidEffectSink {
    fn push(&self, _effect: Effect) {}
    fn drain_into(&self, _out: &mut Vec<Effect>) {}
}

#[cfg(test)]
mod tests;
