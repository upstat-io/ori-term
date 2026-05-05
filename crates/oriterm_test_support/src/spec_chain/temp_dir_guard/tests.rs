//! Tests for `TempDirGuard`.
//!
//! The property is `temp_dir_guard_cleans_up_on_panic_unwinding` —
//! proves the RAII contract holds when an assertion inside the guard's
//! scope panics during unwind, which is the exact failure mode the
//! manual `fs::remove_*` tails did NOT cover.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::TempDirGuard;

/// Pins: the guard's `path()` accessor resolves to an existing directory
/// while the guard is alive AND the human-readable prefix survives to
/// the final on-disk dir name. Anchor: `HYG-13.1-005` §13.1 close-out.
#[test]
fn temp_dir_guard_path_exists_while_guard_alive() {
    let g = TempDirGuard::new("exists");
    assert!(g.path().is_dir(), "path must exist once constructed");
    assert!(
        g.path()
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|s| s.starts_with("oriterm_spec_exists_")),
        "prefix must survive to the final dir name — got {:?}",
        g.path(),
    );
}

/// Pins: the RAII `Drop` chain removes the directory when the guard
/// exits normal scope. Paired positive case for the panic-unwinding
/// property below. Anchor: `HYG-13.1-005` §13.1 close-out,
/// §Temporal Coupling & RAII Guards.
#[test]
fn temp_dir_guard_cleans_up_on_normal_drop() {
    let path: PathBuf = {
        let g = TempDirGuard::new("normal_drop");
        let captured = g.path().to_path_buf();
        assert!(captured.is_dir());
        captured
    };
    assert!(!path.exists(), "dir must be removed on guard drop");
}

/// Verifies: the RAII `Drop` chain runs during panic unwinding —
/// the exact path manual `fs::remove_*` tails cannot cover. The guard
/// is constructed INSIDE the `catch_unwind` closure so the panic
/// actually unwinds past its scope, triggering `Drop`; a side-channel
/// `Arc<Mutex<Option<PathBuf>>>` records the path before the panic so
/// the outer scope can assert on it. Anchor: `HYG-13.1-005` §13.1
/// close-out, §Temporal Coupling &
/// RAII Guards.
#[test]
fn temp_dir_guard_cleans_up_on_panic_unwinding() {
    let captured: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
    let captured_inner = Arc::clone(&captured);
    let result = catch_unwind(AssertUnwindSafe(move || {
        let g = TempDirGuard::new("panic_path");
        let path = g.path().to_path_buf();
        assert!(path.is_dir(), "precondition: dir exists before panic");
        *captured_inner.lock().expect("capture lock") = Some(path);
        panic!("simulated assertion failure");
    }));
    assert!(
        result.is_err(),
        "inner block MUST panic — if not, the test is not exercising the unwinding path",
    );
    let path = captured
        .lock()
        .expect("capture lock")
        .take()
        .expect("closure must have recorded its path before panicking");
    assert!(
        !path.exists(),
        "dir MUST be removed by the guard's drop during panic unwinding — manual cleanup tails skip this path, RAII must not",
    );
}

/// Pins: the cleanup removes non-empty directory contents (nested files
/// and subdirectories), not just an empty dir. Anchor: `HYG-13.1-005`
/// §13.1 close-out.
#[test]
fn temp_dir_guard_cleans_up_with_nested_files() {
    let path: PathBuf = {
        let g = TempDirGuard::new("nested");
        let file = g.path().join("payload.raw");
        std::fs::write(&file, b"hello").expect("write fixture");
        let subdir = g.path().join("inner");
        std::fs::create_dir(&subdir).expect("mkdir inner");
        std::fs::write(subdir.join("deep.bin"), b"more").expect("write deep");
        g.path().to_path_buf()
    };
    assert!(
        !path.exists(),
        "non-empty dir contents MUST be removed along with the dir",
    );
}
