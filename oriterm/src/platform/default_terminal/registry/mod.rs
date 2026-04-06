//! Windows Default Terminal registry helpers (Section 03.9 Phase 2).
//!
//! Reads and writes the registry keys that control which terminal
//! application receives new console sessions on Windows 11+.
//!
//! ## Two distinct registration steps
//!
//! 1. **Selector keys** under `HKCU\Console\%%Startup`:
//!    - `DelegationConsole` (`REG_SZ`): CLSID of the console host
//!      (we use Windows Terminal's `OpenConsole.exe` CLSID).
//!    - `DelegationTerminal` (`REG_SZ`): CLSID of the terminal application
//!      (our [`ORITERM_TERMINAL_CLSID`]).
//!
//!    Conhost reads these at startup to decide where to delegate.
//!
//! 2. **COM server registration** under
//!    `HKCU\Software\Classes\CLSID\{ORITERM_TERMINAL_CLSID}\LocalServer32`:
//!    - Default value (`REG_SZ`): full path to `oriterm.exe`.
//!
//!    Required so COM can `CoCreateInstance` our class out-of-process
//!    when the console host hands a session off (Phase 3 wires the COM
//!    server lifecycle that consumes this registration).
//!
//! Standard COM marshaling is used (no proxy/stub DLL) because
//! `ITerminalHandoff3` parameters are primitive `HANDLE`s and a single
//! struct pointer.
//!
//! ## Path parameterization
//!
//! Production code calls [`register_all`] / [`unregister_all`] /
//! [`is_registered`] which use [`RegistryPaths::production`]. Tests use
//! the `*_at` variants with a [`RegistryPaths`] scoped under
//! `HKCU\Software\Classes\oriterm_test_<random>` so they can run in
//! parallel without trampling on the user's real registry.

#![allow(
    unsafe_code,
    reason = "Win32 registry FFI: RegCreateKeyExW/RegSetValueExW/RegDeleteTreeW/RegGetValueW"
)]

use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ,
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegGetValueW, RegSetValueExW,
};

/// Stable CLSID for `ori_term`'s `ITerminalHandoff3` COM server.
///
/// Generated once via `uuidgen` for Section 03.9. Hardcoded as a string
/// constant so the registry helpers and the future COM server
/// implementation (Phase 3) reference the exact same identifier.
///
/// **Format**: `{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}` — braces required by
/// the Windows Registry CLSID convention.
pub(crate) const ORITERM_TERMINAL_CLSID: &str = "{86A2D6B1-7A4C-4F37-9C5E-9E0F0B7DBAE2}";

/// Windows Terminal's `OpenConsole.exe` CLSID.
///
/// Used as the `DelegationConsole` value: when `oriterm` is the default
/// terminal, conhost still delegates the *console host* role to
/// `OpenConsole` (which speaks `ConPTY` to `ori_term`). This is the same
/// pairing Windows Terminal uses, so installing `oriterm` does not
/// require shipping a separate console host.
///
/// Reference: Windows Terminal `src/cascadia/CascadiaPackage/Resources/`
/// `AppExtension` catalog.
const OPENCONSOLE_DELEGATION_CONSOLE_CLSID: &str = "{2EACA947-7F5F-4CF2-97EA-C9E8AED6FC68}";

/// Subkey under `HKCU` containing the conhost delegation selectors.
const PRODUCTION_STARTUP_SUBKEY: &str = r"Console\%%Startup";

/// Configurable registry paths used by [`register_all_at`] /
/// [`unregister_all_at`] / [`is_registered_at`].
///
/// Production callers use [`RegistryPaths::production`]. Tests build a
/// scoped instance pointing under `HKCU\Software\Classes\oriterm_test_*`
/// so they can mutate the registry without polluting the user's hive.
pub(crate) struct RegistryPaths {
    /// Subkey holding `DelegationConsole` and `DelegationTerminal` values.
    pub(crate) startup_subkey: String,
    /// Subkey holding the `LocalServer32` default value (the exe path).
    pub(crate) clsid_subkey: String,
}

impl RegistryPaths {
    /// Production registry paths.
    pub(crate) fn production() -> Self {
        Self {
            startup_subkey: PRODUCTION_STARTUP_SUBKEY.to_string(),
            clsid_subkey: format!(r"Software\Classes\CLSID\{ORITERM_TERMINAL_CLSID}\LocalServer32"),
        }
    }
}

/// Register `ori_term` as the default terminal handler on Windows.
///
/// Writes both registration steps (selectors + CLSID/LocalServer32) under
/// the production registry paths.
#[allow(
    dead_code,
    reason = "consumed by --register-default CLI subcommand in Phase 4"
)]
pub(crate) fn register_all(exe_path: &Path) -> io::Result<()> {
    register_all_at(&RegistryPaths::production(), exe_path)
}

/// Remove `ori_term`'s default terminal registration.
///
/// Deletes both registration steps. Idempotent — missing keys are not
/// errors.
#[allow(
    dead_code,
    reason = "consumed by --unregister-default CLI subcommand in Phase 4"
)]
pub(crate) fn unregister_all() -> io::Result<()> {
    unregister_all_at(&RegistryPaths::production())
}

/// Whether `ori_term` is currently registered as the default terminal.
///
/// Returns `true` only if `DelegationTerminal` under the production
/// startup subkey matches [`ORITERM_TERMINAL_CLSID`] exactly.
#[allow(
    dead_code,
    reason = "consumed by Settings UI toggle and Phase 4 CLI status query"
)]
pub(crate) fn is_registered() -> bool {
    is_registered_at(&RegistryPaths::production())
}

/// Path-parameterized registration — see [`register_all`].
pub(crate) fn register_all_at(paths: &RegistryPaths, exe_path: &Path) -> io::Result<()> {
    write_delegation_value(
        paths,
        "DelegationConsole",
        OPENCONSOLE_DELEGATION_CONSOLE_CLSID,
    )?;
    write_delegation_value(paths, "DelegationTerminal", ORITERM_TERMINAL_CLSID)?;
    write_local_server32(paths, exe_path)?;
    Ok(())
}

/// Path-parameterized unregistration — see [`unregister_all`].
pub(crate) fn unregister_all_at(paths: &RegistryPaths) -> io::Result<()> {
    delete_subkey_tree(&paths.startup_subkey)?;
    delete_subkey_tree(&paths.clsid_subkey)?;
    Ok(())
}

/// Path-parameterized registration check — see [`is_registered`].
pub(crate) fn is_registered_at(paths: &RegistryPaths) -> bool {
    match read_delegation_value(paths, "DelegationTerminal") {
        Some(value) => value.eq_ignore_ascii_case(ORITERM_TERMINAL_CLSID),
        None => false,
    }
}

/// Write a single value under the startup subkey.
///
/// Public to the tests module so the corruption-detection test can write
/// a non-CLSID value into `DelegationTerminal` without going through the
/// full registration helper.
pub(crate) fn write_delegation_value(
    paths: &RegistryPaths,
    name: &str,
    value: &str,
) -> io::Result<()> {
    let key = create_subkey(&paths.startup_subkey)?;
    let result = set_string_value(key.handle(), name, value);
    drop(key);
    result
}

/// Write the `LocalServer32` default value (= the exe path).
fn write_local_server32(paths: &RegistryPaths, exe_path: &Path) -> io::Result<()> {
    let key = create_subkey(&paths.clsid_subkey)?;
    // Default value is named with the empty string.
    let exe_str = exe_path.to_string_lossy();
    let result = set_string_value(key.handle(), "", &exe_str);
    drop(key);
    result
}

/// Read a `REG_SZ` value from the startup subkey.
///
/// Returns `None` if the key or value does not exist (the
/// `is_registered` path treats both as "not registered").
fn read_delegation_value(paths: &RegistryPaths, name: &str) -> Option<String> {
    let subkey_w = wide(&paths.startup_subkey);
    let name_w = wide(name);

    // First call: query the buffer size with a null data pointer.
    let mut size: u32 = 0;
    // SAFETY: HKEY_CURRENT_USER is a constant predefined hive handle;
    // subkey_w and name_w are valid null-terminated UTF-16 strings;
    // size is a valid out-pointer; data pointer is null which RegGetValueW
    // accepts to return the required buffer size in `size`.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey_w.as_ptr(),
            name_w.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &raw mut size,
        )
    };
    if status != ERROR_SUCCESS || size == 0 {
        return None;
    }

    // Allocate exactly enough u16 slots (size is in bytes including the
    // terminating NUL).
    let len_u16 = (size as usize).div_ceil(2);
    let mut buf = vec![0u16; len_u16];
    let mut size_inout = size;
    // SAFETY: same invariants as above. The data pointer now refers to
    // a buffer of `size_inout` bytes, matching the value RegGetValueW
    // requested. RegGetValueW writes the UTF-16 string with a NUL
    // terminator into `buf`.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey_w.as_ptr(),
            name_w.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &raw mut size_inout,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }

    // Trim the trailing NUL(s) before decoding.
    let trimmed_end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..trimmed_end]))
}

/// Recursively delete a subkey under `HKCU`. Idempotent.
fn delete_subkey_tree(subkey: &str) -> io::Result<()> {
    let subkey_w = wide(subkey);
    // SAFETY: HKEY_CURRENT_USER is a constant; subkey_w is a valid
    // null-terminated UTF-16 string. RegDeleteTreeW returns
    // ERROR_FILE_NOT_FOUND for missing keys, which we treat as success
    // for idempotency.
    let status = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, subkey_w.as_ptr()) };
    if status == ERROR_SUCCESS || status == windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}

// Low-level helpers.

/// Convert a Rust string to a null-terminated UTF-16 buffer for Win32 APIs.
fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

/// RAII guard owning a Win32 `HKEY` opened via `RegCreateKeyExW`.
///
/// `Drop` calls `RegCloseKey` so the handle is released even on early
/// returns from set/delete operations.
struct OpenedKey(HKEY);

impl OpenedKey {
    fn handle(&self) -> HKEY {
        self.0
    }
}

impl Drop for OpenedKey {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: self.0 is a valid HKEY produced by RegCreateKeyExW
            // and not yet closed (Drop runs at most once).
            unsafe {
                let _ = RegCloseKey(self.0);
            }
        }
    }
}

/// Create or open a subkey under `HKCU` with read+write access.
fn create_subkey(subkey: &str) -> io::Result<OpenedKey> {
    let subkey_w = wide(subkey);
    let mut key: HKEY = std::ptr::null_mut();
    let mut disposition: u32 = 0;
    // SAFETY: HKEY_CURRENT_USER is a constant; subkey_w is a valid
    // null-terminated UTF-16 string; key/disposition are valid out-
    // pointers. RegCreateKeyExW either opens an existing key or creates
    // a new one with REG_OPTION_NON_VOLATILE storage.
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey_w.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_READ | KEY_WRITE,
            std::ptr::null(),
            &raw mut key,
            &raw mut disposition,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    Ok(OpenedKey(key))
}

/// `cold` factory for the "value too large for `REG_SZ`" error so the
/// hot path stays free of `map_err(|_|)` wildcards.
#[cold]
fn value_too_large(_err: std::num::TryFromIntError) -> io::Error {
    io::Error::other("registry value too large for REG_SZ")
}

/// Set a `REG_SZ` value on an open key.
fn set_string_value(key: HKEY, name: &str, value: &str) -> io::Result<()> {
    let name_w = wide(name);
    let value_w = wide(value);
    let byte_len = u32::try_from(value_w.len() * 2).map_err(value_too_large)?;
    // SAFETY: key is a valid HKEY held by an OpenedKey RAII guard;
    // name_w/value_w are valid null-terminated UTF-16 strings; byte_len
    // matches the buffer length in bytes. RegSetValueExW writes the
    // string atomically.
    let status = unsafe {
        RegSetValueExW(
            key,
            name_w.as_ptr(),
            0,
            REG_SZ,
            value_w.as_ptr().cast(),
            byte_len,
        )
    };
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}

#[cfg(test)]
mod tests;
