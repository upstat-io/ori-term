//! Animation metadata that travels with state changes.
//!
//! SwiftUI-inspired: wrap state mutations in [`with_transaction()`] to override
//! the animation curve used by all [`AnimProperty::set()`](super::property::AnimProperty::set)
//! calls within the block.

use std::cell::Cell;

use super::behavior::AnimBehavior;

/// Animation metadata attached to a state change.
///
/// When a `Transaction` is active, all `AnimProperty::set()` calls within
/// it use the transaction's animation curve instead of the property's
/// default behavior.
#[derive(Debug, Clone, Copy)]
pub struct Transaction {
    /// Override animation. `None` means instant (no animation regardless
    /// of property behavior).
    pub animation: Option<AnimBehavior>,
}

impl Transaction {
    /// No animation — changes are instant regardless of property behavior.
    pub fn instant() -> Self {
        Self { animation: None }
    }

    /// Override with a specific animation curve.
    pub fn animated(behavior: AnimBehavior) -> Self {
        Self {
            animation: Some(behavior),
        }
    }
}

thread_local! {
    static CURRENT_TRANSACTION: Cell<Option<Transaction>> = const { Cell::new(None) };
}

/// Drop guard that restores the previous transaction on scope exit.
struct RestoreGuard<'a> {
    cell: &'a Cell<Option<Transaction>>,
    prev: Option<Transaction>,
}

impl Drop for RestoreGuard<'_> {
    fn drop(&mut self) {
        self.cell.set(self.prev);
    }
}

/// Execute `f` with the given transaction active.
///
/// All `AnimProperty::set()` calls within `f` use the transaction's
/// animation curve instead of each property's default behavior.
///
/// Transactions nest: the inner transaction overrides the outer one.
/// The previous transaction is restored after `f` returns (including
/// on panic, via the `RestoreGuard` drop).
pub fn with_transaction<F, R>(tx: Transaction, f: F) -> R
where
    F: FnOnce() -> R,
{
    CURRENT_TRANSACTION.with(|cell| {
        let prev = cell.get();
        cell.set(Some(tx));

        let guard = RestoreGuard { cell, prev };
        let result = f();
        drop(guard);
        result
    })
}

/// Read the current transaction (called by `AnimProperty::set()`).
pub(crate) fn current_transaction() -> Option<Transaction> {
    CURRENT_TRANSACTION.with(Cell::get)
}

#[cfg(test)]
mod tests;
