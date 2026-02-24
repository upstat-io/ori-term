//! Synchronization primitives for terminal emulation.
//!
//! Provides [`FairMutex`], a mutex that prevents starvation between the PTY
//! reader thread and the render thread. Two threads competing for the same
//! lock will alternate access rather than allowing one to monopolize it.

use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::{Mutex, MutexGuard};

/// A fair mutex that prevents thread starvation.
///
/// Uses a two-lock protocol: a `next` lock for queuing and a `data` lock for
/// the protected value. Fair callers acquire `next` first (establishing FIFO
/// order), then `data`. Unfair callers bypass `next` entirely.
///
/// Both the PTY reader and render threads use [`lock`](FairMutex::lock) for
/// fair access. The reader calls [`unlock_fair`](FairMutexGuard::unlock_fair)
/// to guarantee the renderer gets the next turn when contending.
pub struct FairMutex<T> {
    /// The protected data.
    data: Mutex<T>,
    /// Fairness gate — establishes FIFO ordering among fair callers.
    next: Mutex<()>,
    /// Set when a `lock()` caller had to wait for the fairness gate.
    /// Cleared by `take_contended()`. The PTY reader checks this to
    /// decide whether to coalesce (sleep) — only when the renderer
    /// actually blocked does the reader yield.
    contended: AtomicBool,
}

/// RAII guard returned by [`FairMutex::lock`].
///
/// Holds both the fairness gate and data lock. Releasing this guard frees
/// both, allowing the next queued fair caller to proceed.
pub struct FairMutexGuard<'a, T> {
    /// Data lock — provides access to the protected value.
    data: MutexGuard<'a, T>,
    /// Fairness gate — held to prevent queue jumping.
    next: MutexGuard<'a, ()>,
}

/// RAII lease on the fairness gate, returned by [`FairMutex::lease`].
///
/// Test-only: reserves a position in the fair queue without locking the
/// data. Used in Pattern A comparison benchmarks.
#[cfg(test)]
pub(crate) struct FairMutexLease<'a> {
    /// Held for Drop — releasing this guard frees the fairness gate.
    _next: MutexGuard<'a, ()>,
}

impl<T> FairMutex<T> {
    /// Creates a new `FairMutex` protecting `data`.
    pub fn new(data: T) -> Self {
        Self {
            data: Mutex::new(data),
            next: Mutex::new(()),
            contended: AtomicBool::new(false),
        }
    }

    /// Acquires the mutex fairly.
    ///
    /// Blocks until both the fairness gate and data lock are available,
    /// guaranteeing FIFO ordering among fair callers. If the fairness
    /// gate is already held, sets the `contended` flag before blocking
    /// so the holder can detect contention via [`take_contended`].
    pub fn lock(&self) -> FairMutexGuard<'_, T> {
        let next = if let Some(guard) = self.next.try_lock() {
            guard
        } else {
            self.contended.store(true, Ordering::Release);
            self.next.lock()
        };
        let data = self.data.lock();
        FairMutexGuard { data, next }
    }

    /// Returns `true` if any `lock()` call blocked since the last check,
    /// and clears the flag.
    ///
    /// The PTY reader calls this after each processing cycle to decide
    /// whether to yield. When the renderer had to wait for the fairness
    /// gate, this returns `true` once, signaling the reader to coalesce.
    pub fn take_contended(&self) -> bool {
        self.contended.swap(false, Ordering::Acquire)
    }

    /// Acquires the mutex without fairness.
    ///
    /// Test-only: bypasses the fairness gate for Pattern A comparison
    /// benchmarks. Production code uses [`lock`](Self::lock).
    #[cfg(test)]
    pub(crate) fn lock_unfair(&self) -> MutexGuard<'_, T> {
        self.data.lock()
    }

    /// Attempts to acquire the mutex without fairness or blocking.
    ///
    /// Test-only: returns `None` if the data lock is currently held.
    /// Used in Pattern A comparison benchmarks.
    #[cfg(test)]
    pub(crate) fn try_lock_unfair(&self) -> Option<MutexGuard<'_, T>> {
        self.data.try_lock()
    }

    /// Reserves a position in the fair queue without locking the data.
    ///
    /// Test-only: the returned [`FairMutexLease`] holds the fairness
    /// gate for Pattern A comparison benchmarks.
    #[cfg(test)]
    pub(crate) fn lease(&self) -> FairMutexLease<'_> {
        FairMutexLease {
            _next: self.next.lock(),
        }
    }
}

impl<T> FairMutexGuard<'_, T> {
    /// Releases the guard using `parking_lot`'s fair unlock protocol.
    ///
    /// Unlike regular `drop()`, this hands the fairness gate directly to the
    /// next waiting thread (if any), preventing barging. The PTY reader
    /// should call this after each parse chunk so the render thread gets a
    /// guaranteed turn.
    ///
    /// When no thread is waiting, this behaves identically to `drop()`.
    pub fn unlock_fair(self) {
        let Self { data, next } = self;
        // Release data first so the next thread can acquire it immediately
        // after receiving the fairness gate handoff.
        drop(data);
        MutexGuard::unlock_fair(next);
    }
}

impl<T> Deref for FairMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data
    }
}

impl<T> DerefMut for FairMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

#[cfg(test)]
mod tests;
