//! Windows COM implementation for Jump List submission.
//!
//! Uses `ICustomDestinationList` and `IShellLinkW` to register tasks
//! in the Windows taskbar right-click menu.

use std::path::{Path, PathBuf};

use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::Shell::Common::IObjectCollection;
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::Win32::UI::Shell::{ICustomDestinationList, IShellLinkW};
use windows::core::{GUID, Interface, PCWSTR, Result};

use super::JumpListTask;

/// `DestinationList` CLSID — the COM class that implements `ICustomDestinationList`.
const CLSID_DESTINATION_LIST: GUID = GUID::from_u128(0x77f10cf0_3db5_4966_b520_b7c54fd35ed6);

/// `ShellLink` CLSID — the COM class that implements `IShellLinkW`.
const CLSID_SHELL_LINK: GUID = GUID::from_u128(0x00021401_0000_0000_c000_000000000046);

/// `EnumerableObjectCollection` CLSID — the COM class that implements `IObjectCollection`.
const CLSID_ENUMERABLE_OBJECT_COLLECTION: GUID =
    GUID::from_u128(0x2d3468c1_36a7_43b6_ac24_d3f02fd9607a);

/// Submit jump list tasks to the Windows taskbar.
///
/// Creates a COM transaction that registers the given tasks as
/// "User Tasks" in the Jump List. Requires COM to be initialized
/// on the calling thread (`CoInitializeEx`).
#[allow(unsafe_code, reason = "COM FFI for Jump List construction")]
pub(crate) fn submit_jump_list(tasks: &[JumpListTask]) -> Result<()> {
    let exe = exe_path().map_err(|e| {
        windows::core::Error::new(
            windows::core::HRESULT(-1),
            format!("failed to resolve exe path: {e}"),
        )
    })?;

    let dest_list: ICustomDestinationList =
        unsafe { CoCreateInstance(&CLSID_DESTINATION_LIST, None, CLSCTX_INPROC_SERVER)? };

    let mut max_slots: u32 = 0;
    // BeginList returns the removed objects array (must be queried even if ignored).
    let _removed: windows::Win32::UI::Shell::Common::IObjectArray =
        unsafe { dest_list.BeginList(&raw mut max_slots)? };

    let collection: IObjectCollection = unsafe {
        CoCreateInstance(
            &CLSID_ENUMERABLE_OBJECT_COLLECTION,
            None,
            CLSCTX_INPROC_SERVER,
        )?
    };

    for task in tasks {
        let link = create_shell_link(&exe, task)?;
        unsafe { collection.AddObject(&link)? };
    }

    unsafe {
        dest_list.AddUserTasks(&collection)?;
        dest_list.CommitList()?;
    }

    Ok(())
}

/// Create a single `IShellLinkW` for a jump list task.
#[allow(unsafe_code, reason = "COM FFI for Jump List construction")]
fn create_shell_link(exe: &Path, task: &JumpListTask) -> Result<IShellLinkW> {
    let link: IShellLinkW =
        unsafe { CoCreateInstance(&CLSID_SHELL_LINK, None, CLSCTX_INPROC_SERVER)? };

    let exe_wide = to_wide(exe.to_string_lossy().as_ref());
    let args_wide = to_wide(&task.arguments);
    let desc_wide = to_wide(&task.description);

    unsafe {
        link.SetPath(PCWSTR(exe_wide.as_ptr()))?;
        link.SetArguments(PCWSTR(args_wide.as_ptr()))?;
        link.SetDescription(PCWSTR(desc_wide.as_ptr()))?;
    }

    set_link_title(&link, &task.label)?;

    Ok(link)
}

/// Set the display title on a shell link via `IPropertyStore`.
#[allow(unsafe_code, reason = "COM FFI for Jump List construction")]
fn set_link_title(link: &IShellLinkW, title: &str) -> Result<()> {
    use windows::Win32::Storage::EnhancedStorage::PKEY_Title;
    use windows::Win32::System::Com::StructuredStorage::{
        PROPVARIANT, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
    };
    use windows::Win32::System::Variant::VT_LPWSTR;

    let store: IPropertyStore = link.cast()?;
    let title_wide = to_wide(title);

    // Build a PROPVARIANT with VT_LPWSTR pointing to the title.
    let mut pv = PROPVARIANT::default();
    unsafe {
        pv.Anonymous.Anonymous = std::mem::ManuallyDrop::new(PROPVARIANT_0_0 {
            vt: VT_LPWSTR,
            wReserved1: 0,
            wReserved2: 0,
            wReserved3: 0,
            Anonymous: PROPVARIANT_0_0_0 {
                pwszVal: windows::core::PWSTR(title_wide.as_ptr().cast_mut()),
            },
        });

        store.SetValue(&PKEY_Title, &raw const pv)?;
        store.Commit()?;

        // Prevent double-free — the wide string is stack-owned, not COM-allocated.
        // Explicit deref required because ManuallyDrop doesn't auto-apply DerefMut
        // on union fields (would run the destructor for the old value).
        (*pv.Anonymous.Anonymous).Anonymous.pwszVal = windows::core::PWSTR::null();
    }

    Ok(())
}

/// Resolve the path to the running `oriterm` binary.
fn exe_path() -> std::io::Result<PathBuf> {
    std::env::current_exe()
}

/// Convert a `&str` to a null-terminated UTF-16 vector.
fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
