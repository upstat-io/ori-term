//! Windows implementation of [`NativeWindowOps`].
//!
//! Uses Win32 APIs through `windows-sys` for HWND ownership, DWM shadow
//! extension, extended window styles, and modal window disabling.

#![allow(unsafe_code)]

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
use windows_sys::Win32::UI::Controls::MARGINS;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_TOOLWINDOW,
};

use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use super::NativeWindowOps;
use crate::window_manager::types::WindowKind;

/// `SetWindowLongPtrW` index for the owner window handle.
///
/// Not exported by `windows-sys` — the raw value (-8) is the standard
/// `GWLP_HWNDPARENT` constant used to set/get the owner HWND.
const GWLP_HWNDPARENT: i32 = -8;

pub(super) struct WindowsNativeOps;

impl NativeWindowOps for WindowsNativeOps {
    fn set_owner(&self, child: &Window, parent: &Window) {
        let Some(child_hwnd) = extract_hwnd(child) else {
            return;
        };
        let Some(parent_hwnd) = extract_hwnd(parent) else {
            return;
        };
        // Owner relationship: child always above owner in z-order,
        // hidden when owner is minimized.
        let prev = unsafe { SetWindowLongPtrW(child_hwnd, GWLP_HWNDPARENT, parent_hwnd as isize) };
        if prev == 0 {
            log::warn!("SetWindowLongPtrW(GWLP_HWNDPARENT) returned 0");
        }
    }

    fn clear_owner(&self, child: &Window) {
        let Some(child_hwnd) = extract_hwnd(child) else {
            return;
        };
        unsafe { SetWindowLongPtrW(child_hwnd, GWLP_HWNDPARENT, 0) };
    }

    fn enable_shadow(&self, window: &Window) {
        let Some(hwnd) = extract_hwnd(window) else {
            return;
        };
        // Dialog windows lack WS_THICKFRAME, so all four margins are needed
        // for DWM shadow. Main windows only need cyTopHeight:1 because
        // WS_THICKFRAME provides the other edges.
        let margins = MARGINS {
            cxLeftWidth: 1,
            cxRightWidth: 1,
            cyTopHeight: 1,
            cyBottomHeight: 1,
        };
        let hr = unsafe { DwmExtendFrameIntoClientArea(hwnd, &raw const margins) };
        if hr != 0 {
            log::warn!("DwmExtendFrameIntoClientArea failed: HRESULT 0x{hr:08X}");
        }
    }

    fn set_window_type(&self, window: &Window, kind: &WindowKind) {
        if !kind.is_dialog() {
            return;
        }
        let Some(hwnd) = extract_hwnd(window) else {
            return;
        };
        // WS_EX_TOOLWINDOW: no taskbar button, smaller title bar frame.
        unsafe {
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_TOOLWINDOW as isize);
        }
    }

    fn set_modal(&self, _dialog: &Window, owner: &Window) {
        let Some(owner_hwnd) = extract_hwnd(owner) else {
            return;
        };
        unsafe { EnableWindow(owner_hwnd, 0) }; // FALSE = disabled
    }

    fn clear_modal(&self, _dialog: &Window, owner: &Window) {
        let Some(owner_hwnd) = extract_hwnd(owner) else {
            return;
        };
        unsafe { EnableWindow(owner_hwnd, 1) }; // TRUE = enabled
    }
}

/// Extracts the raw HWND from a winit `Window`.
fn extract_hwnd(window: &Window) -> Option<HWND> {
    let handle = window.window_handle().ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(h) => Some(h.hwnd.get() as HWND),
        _ => None,
    }
}
