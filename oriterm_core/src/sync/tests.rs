use std::sync::Arc;
use std::thread;

use super::{FairMutex, FairMutexGuard};

#[test]
fn basic_lock_unlock() {
    let mutex = FairMutex::new(42);

    {
        let mut guard = mutex.lock();
        assert_eq!(*guard, 42);
        *guard = 100;
    }

    let guard = mutex.lock();
    assert_eq!(*guard, 100);
}

#[test]
fn two_threads_take_turns() {
    let mutex = Arc::new(FairMutex::new(Vec::new()));
    let iterations = 100;

    let m1 = Arc::clone(&mutex);
    let t1 = thread::spawn(move || {
        for i in 0..iterations {
            let mut guard = m1.lock();
            guard.push(('A', i));
        }
    });

    let m2 = Arc::clone(&mutex);
    let t2 = thread::spawn(move || {
        for i in 0..iterations {
            let mut guard = m2.lock();
            guard.push(('B', i));
        }
    });

    t1.join().unwrap();
    t2.join().unwrap();

    let guard = mutex.lock();
    // Both threads contributed all their entries.
    assert_eq!(guard.len(), iterations * 2);

    let a_count = guard.iter().filter(|(c, _)| *c == 'A').count();
    let b_count = guard.iter().filter(|(c, _)| *c == 'B').count();
    assert_eq!(a_count, iterations);
    assert_eq!(b_count, iterations);
}

#[test]
fn try_lock_unfair_returns_none_when_locked() {
    let mutex = FairMutex::new(());
    let _guard = mutex.lock_unfair();

    assert!(mutex.try_lock_unfair().is_none());
}

#[test]
fn try_lock_unfair_succeeds_when_unlocked() {
    let mutex = FairMutex::new(7);
    let guard = mutex.try_lock_unfair();
    assert!(guard.is_some());
    assert_eq!(*guard.unwrap(), 7);
}

#[test]
fn lease_blocks_fair_lock() {
    let mutex = Arc::new(FairMutex::new(0));

    // Take a lease — this holds the `next` lock.
    let _lease = mutex.lease();

    // Unfair lock should still succeed (bypasses `next`).
    {
        let mut guard = mutex.lock_unfair();
        *guard = 1;
    }

    // Fair lock from another thread should block because the lease holds `next`.
    let m = Arc::clone(&mutex);
    let handle = thread::spawn(move || {
        // This will block until the lease is dropped.
        let guard = m.lock();
        *guard
    });

    // Give the spawned thread time to attempt the lock.
    thread::sleep(std::time::Duration::from_millis(50));

    // The thread should still be running (blocked on `next`).
    assert!(!handle.is_finished());

    // Drop the lease — the spawned thread should now proceed.
    drop(_lease);

    let val = handle.join().unwrap();
    assert_eq!(val, 1);
}

#[test]
fn lock_unfair_bypasses_fairness() {
    let mutex = FairMutex::new(42);

    // Lock unfair should give direct access to data.
    let guard = mutex.lock_unfair();
    assert_eq!(*guard, 42);
    drop(guard);

    // Fair lock should also work after unfair lock is released.
    let guard = mutex.lock();
    assert_eq!(*guard, 42);
}

#[test]
fn guard_deref_mut() {
    let mutex = FairMutex::new(String::from("hello"));
    let mut guard: FairMutexGuard<'_, String> = mutex.lock();
    guard.push_str(" world");
    assert_eq!(&*guard, "hello world");
}
