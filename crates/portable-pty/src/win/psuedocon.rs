use super::WinChild;
use crate::cmdbuilder::CommandBuilder;
use crate::win::procthreadattr::ProcThreadAttributeList;
use anyhow::{bail, ensure, Error};
use filedescriptor::OwnedHandle;
use lazy_static::lazy_static;
use shared_library::shared_library;
use std::ffi::OsString;
use std::io::Error as IoError;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::{FromRawHandle, RawHandle};
use std::path::Path;
use std::sync::Mutex;
use std::{mem, ptr};
use winapi::shared::minwindef::DWORD;
use winapi::shared::winerror::{HRESULT, S_OK};
use winapi::um::handleapi::*;
use winapi::um::processthreadsapi::*;
use winapi::um::winbase::{
    CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT, STARTF_USESTDHANDLES, STARTUPINFOEXW,
};
use winapi::um::wincon::COORD;
use winapi::um::winnt::HANDLE;

pub type HPCON = HANDLE;

pub const PSUEDOCONSOLE_INHERIT_CURSOR: DWORD = 0x1;

shared_library!(ConPtyFuncs,
    pub fn CreatePseudoConsole(
        size: COORD,
        hInput: HANDLE,
        hOutput: HANDLE,
        flags: DWORD,
        hpc: *mut HPCON
    ) -> HRESULT,
    pub fn ResizePseudoConsole(hpc: HPCON, size: COORD) -> HRESULT,
    pub fn ClosePseudoConsole(hpc: HPCON),
);

fn load_conpty() -> ConPtyFuncs {
    // If the kernel doesn't export these functions then their system is
    // too old and we cannot run.
    let kernel = ConPtyFuncs::open(Path::new("kernel32.dll")).expect(
        "this system does not support conpty.  Windows 10 October 2018 or newer is required",
    );

    // We prefer to use a sideloaded conpty.dll and openconsole.exe deployed
    // alongside the application. Microsoft Terminal sideloads its own
    // OpenConsole.exe to get enhanced VT passthrough on the input pipe —
    // kernel32 conpty otherwise translates ESC bytes from host→child writes
    // into VK_ESCAPE INPUT_RECORDs that downstream consumers (wsl.exe, WSL
    // distros) drop on the floor, resulting in stripped-prefix text fragments
    // appearing as bash prompt input AFTER the child has exited.
    //
    // Look for `conpty.dll` in three locations, in order:
    //
    //  1. The directory containing the current executable (`std::env::current_exe`).
    //     This is where a release installer or the user dropping the
    //     Microsoft.Windows.Console.ConPTY NuGet payload would place the DLL.
    //  2. The current working directory (`Path::new("conpty.dll")` — matches
    //     wezterm's existing behavior for back-compat).
    //  3. Fall back to kernel32's built-in ConPTY (the no-sideload path —
    //     known-buggy for VT input passthrough on the wsl→linux chain).
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("conpty.dll")))
    {
        if let Ok(sideloaded) = ConPtyFuncs::open(&exe_dir) {
            return sideloaded;
        }
    }
    if let Ok(sideloaded) = ConPtyFuncs::open(Path::new("conpty.dll")) {
        sideloaded
    } else {
        kernel
    }
}

lazy_static! {
    static ref CONPTY: ConPtyFuncs = load_conpty();
}

pub struct PsuedoCon {
    con: HPCON,
}

unsafe impl Send for PsuedoCon {}
unsafe impl Sync for PsuedoCon {}

impl Drop for PsuedoCon {
    fn drop(&mut self) {
        unsafe { (CONPTY.ClosePseudoConsole)(self.con) };
    }
}

impl PsuedoCon {
    /// Creates a new pseudo-console attached to the given input/output
    /// handles. Caller controls the transport: a duplex pipe may pass
    /// the SAME raw handle for both `input` and `output` (matches WT
    /// `ConptyConnection.cpp:406-407`'s `pipe.client.get(), pipe.client.get()`);
    /// a split pipe arrangement passes two distinct handles.
    ///
    /// Takes raw handles (NOT `FileDescriptor`/`OwnedHandle`) so the
    /// caller owns the lifetime: ConPTY duplicates these handles into
    /// the host process internally via `CreatePseudoConsole`, so the
    /// parent-side handles are consumed-but-not-owned by `PsuedoCon`.
    pub fn new(size: COORD, input: RawHandle, output: RawHandle) -> Result<Self, Error> {
        let mut con: HPCON = INVALID_HANDLE_VALUE;
        // Pass PSEUDOCONSOLE_INHERIT_CURSOR only — match Microsoft Terminal's
        // ConPTY flag set (terminal/src/cascadia/TerminalConnection/ConptyConnection.cpp:262
        // starts with `_flags = 0` and only conditionally adds INHERIT_CURSOR
        // + GLYPH_WIDTH_*). PSEUDOCONSOLE_RESIZE_QUIRK (0x2) and
        // PSEUDOCONSOLE_WIN32_INPUT_MODE (0x4) are absent from Microsoft
        // Terminal's public ConPTY headers (terminal/src/winconpty/winconpty.h
        // declares only INHERIT_CURSOR + GLYPH_WIDTH_*).
        let result = unsafe {
            (CONPTY.CreatePseudoConsole)(
                size,
                input as _,
                output as _,
                PSUEDOCONSOLE_INHERIT_CURSOR,
                &mut con,
            )
        };
        ensure!(
            result == S_OK,
            "failed to create psuedo console: HRESULT {}",
            result
        );
        Ok(Self { con })
    }

    pub fn resize(&self, size: COORD) -> Result<(), Error> {
        let result = unsafe { (CONPTY.ResizePseudoConsole)(self.con, size) };
        ensure!(
            result == S_OK,
            "failed to resize console to {}x{}: HRESULT: {}",
            size.X,
            size.Y,
            result
        );
        Ok(())
    }

    pub fn spawn_command(&self, cmd: CommandBuilder) -> anyhow::Result<WinChild> {
        let mut si: STARTUPINFOEXW = unsafe { mem::zeroed() };
        si.StartupInfo.cb = mem::size_of::<STARTUPINFOEXW>() as u32;
        // Explicitly set the stdio handles as invalid handles otherwise
        // we can end up with a weird state where the spawned process can
        // inherit the explicitly redirected output handles from its parent.
        // For example, when daemonizing wezterm-mux-server, the stdio handles
        // are redirected to a log file and the spawned process would end up
        // writing its output there instead of to the pty we just created.
        si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        si.StartupInfo.hStdInput = INVALID_HANDLE_VALUE;
        si.StartupInfo.hStdOutput = INVALID_HANDLE_VALUE;
        si.StartupInfo.hStdError = INVALID_HANDLE_VALUE;

        let mut attrs = ProcThreadAttributeList::with_capacity(1)?;
        attrs.set_pty(self.con)?;
        si.lpAttributeList = attrs.as_mut_ptr();

        let mut pi: PROCESS_INFORMATION = unsafe { mem::zeroed() };

        let (mut exe, mut cmdline) = cmd.cmdline()?;
        let cmd_os = OsString::from_wide(&cmdline);

        let cwd = cmd.current_directory();

        let res = unsafe {
            CreateProcessW(
                exe.as_mut_slice().as_mut_ptr(),
                cmdline.as_mut_slice().as_mut_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
                cmd.environment_block().as_mut_slice().as_mut_ptr() as *mut _,
                cwd.as_ref()
                    .map(|c| c.as_slice().as_ptr())
                    .unwrap_or(ptr::null()),
                &mut si.StartupInfo,
                &mut pi,
            )
        };
        if res == 0 {
            let err = IoError::last_os_error();
            let msg = format!(
                "CreateProcessW `{:?}` in cwd `{:?}` failed: {}",
                cmd_os,
                cwd.as_ref().map(|c| OsString::from_wide(c)),
                err
            );
            log::error!("{}", msg);
            bail!("{}", msg);
        }

        // Make sure we close out the thread handle so we don't leak it;
        // we do this simply by making it owned
        let _main_thread = unsafe { OwnedHandle::from_raw_handle(pi.hThread as _) };
        let proc = unsafe { OwnedHandle::from_raw_handle(pi.hProcess as _) };

        Ok(WinChild {
            proc: Mutex::new(proc),
        })
    }
}
