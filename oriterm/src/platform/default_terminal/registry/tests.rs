//! Tests for the Windows default-terminal registry helpers.
//!
//! All tests are gated `#[cfg(windows)]` because they touch the real
//! `HKCU` hive — there is no cross-platform stub for this. To avoid
//! polluting the user's registry or interfering with concurrent runs,
//! every test uses a unique scoped subkey under
//! `HKCU\Software\Classes\oriterm_test_<random>` and cleans up via
//! `RegistryTestScope::Drop`.
//!
//! On non-Windows targets the file compiles to an empty module so
//! `cargo test` on Linux/macOS does not error.

#![cfg(windows)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{
    RegistryPaths, is_registered_at, register_all_at, unregister_all_at, write_delegation_value,
};

/// Monotonic counter so concurrent tests get distinct test scopes even
/// when the random seed collides.
static SCOPE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// RAII guard owning a unique scoped registry subtree.
///
/// On `Drop`, recursively deletes the test scope so the next run starts
/// from a clean slate even if the test panicked.
struct RegistryTestScope {
    /// The randomized suffix used to build all subkeys.
    suffix: String,
    /// `RegistryPaths` configured to write under this scope only.
    paths: RegistryPaths,
}

impl RegistryTestScope {
    fn new() -> Self {
        let pid = std::process::id();
        let counter = SCOPE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let suffix = format!("oriterm_test_{pid}_{counter}_{nanos}");
        // clsid_subkey is the PARENT CLSID key — register_all_at will
        // create a `LocalServer32` child under it, and unregister_all_at
        // deletes the entire parent tree (which removes the child).
        let paths = RegistryPaths {
            startup_subkey: format!(r"Software\Classes\{suffix}\Startup"),
            clsid_subkey: format!(r"Software\Classes\{suffix}\CLSID"),
        };
        Self { suffix, paths }
    }

    fn paths(&self) -> &RegistryPaths {
        &self.paths
    }

    fn fake_exe(&self) -> PathBuf {
        // Doesn't have to exist on disk — register_all_at only writes
        // the path string into the LocalServer32 default value.
        PathBuf::from(format!(r"C:\test\{}\oriterm.exe", self.suffix))
    }
}

impl Drop for RegistryTestScope {
    fn drop(&mut self) {
        // Best-effort cleanup. RegDeleteTreeW recursively removes the
        // entire test root, including the Startup and CLSID children.
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RegDeleteTreeW};

        let root = format!(r"Software\Classes\{}", self.suffix);
        let root_w: Vec<u16> = OsStr::new(&root).encode_wide().chain(Some(0)).collect();
        // SAFETY: HKEY_CURRENT_USER is a constant predefined hive handle;
        // root_w is a valid null-terminated UTF-16 string. RegDeleteTreeW
        // accepts NULL/missing keys gracefully (returns ERROR_FILE_NOT_FOUND).
        unsafe {
            let _ = RegDeleteTreeW(HKEY_CURRENT_USER, root_w.as_ptr());
        }
    }
}

#[test]
fn register_then_is_registered_returns_true() {
    let scope = RegistryTestScope::new();
    register_all_at(scope.paths(), &scope.fake_exe())
        .expect("register_all_at must succeed in a fresh scope");
    assert!(
        is_registered_at(scope.paths()),
        "after register_all_at, is_registered_at must return true",
    );
}

#[test]
fn unregister_clears_is_registered() {
    let scope = RegistryTestScope::new();
    register_all_at(scope.paths(), &scope.fake_exe()).expect("register_all_at");
    unregister_all_at(scope.paths()).expect("unregister_all_at");
    assert!(
        !is_registered_at(scope.paths()),
        "after unregister_all_at, is_registered_at must return false",
    );
}

#[test]
fn register_is_idempotent() {
    let scope = RegistryTestScope::new();
    register_all_at(scope.paths(), &scope.fake_exe()).expect("first register_all_at");
    register_all_at(scope.paths(), &scope.fake_exe())
        .expect("second register_all_at must not error");
    assert!(is_registered_at(scope.paths()));
}

#[test]
fn unregister_without_register_is_no_error() {
    let scope = RegistryTestScope::new();
    // No prior register_all_at — keys do not exist.
    unregister_all_at(scope.paths()).expect("unregister of non-existent keys must succeed");
    assert!(!is_registered_at(scope.paths()));
}

#[test]
fn is_registered_with_corrupted_guid_returns_false() {
    let scope = RegistryTestScope::new();
    // Write a non-CLSID value into DelegationTerminal — `is_registered_at`
    // must reject anything that doesn't match `ORITERM_TERMINAL_CLSID`.
    write_delegation_value(scope.paths(), "DelegationTerminal", "not-a-real-guid")
        .expect("write_delegation_value");
    assert!(
        !is_registered_at(scope.paths()),
        "DelegationTerminal with the wrong CLSID must not count as registered",
    );
}

#[test]
fn is_registered_with_missing_startup_subkey_returns_false() {
    let scope = RegistryTestScope::new();
    // Never created — `is_registered_at` must handle missing keys.
    assert!(!is_registered_at(scope.paths()));
}
