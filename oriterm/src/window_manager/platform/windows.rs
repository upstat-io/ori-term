//! Windows implementation of [`NativeWindowOps`] and [`NativeChromeOps`].
//!
//! Uses Win32 APIs through `windows-sys` for HWND ownership, DWM shadow
//! extension, extended window styles, modal window disabling, and frameless
//! chrome subclass installation.

#![allow(unsafe_code, reason = "Win32 FFI via windows-sys")]

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
use windows_sys::Win32::UI::Controls::MARGINS;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_TOOLWINDOW,
};

use winit::window::Window;

use oriterm_ui::geometry::Rect;

use super::{ChromeMode, NativeChromeOps, NativeWindowOps};
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

impl NativeChromeOps for WindowsNativeOps {
    fn install_chrome(
        &self,
        window: &Window,
        mode: ChromeMode,
        border_width: f32,
        caption_height: f32,
    ) {
        match mode {
            ChromeMode::Main => {
                oriterm_ui::platform_windows::enable_snap(window, border_width, caption_height);
                // Main windows get edge shadows from WS_THICKFRAME.
                // DWM margins {0,0,1,0} are set by install_chrome_subclass.
            }
            ChromeMode::Dialog { resizable } => {
                let bw = if resizable { border_width } else { 0.0 };
                oriterm_ui::platform_windows::enable_dialog_chrome(window, bw, caption_height);

                // Dialog windows lack WS_THICKFRAME, so install_chrome_subclass's
                // {0,0,1,0} margins only give a top-edge shadow. Override with
                // full margins for shadow on all four edges.
                if let Some(hwnd) = extract_hwnd(window) {
                    let margins = MARGINS {
                        cxLeftWidth: 1,
                        cxRightWidth: 1,
                        cyTopHeight: 1,
                        cyBottomHeight: 1,
                    };
                    unsafe { DwmExtendFrameIntoClientArea(hwnd, &raw const margins) };
                }
            }
        }
    }

    fn set_interactive_rects(&self, window: &Window, rects: &[Rect], scale: f32) {
        let scaled: Vec<Rect> = rects.iter().map(|r| scale_rect(*r, scale)).collect();
        oriterm_ui::platform_windows::set_client_rects(window, &scaled);
    }

    fn set_chrome_metrics(&self, window: &Window, border_width: f32, caption_height: f32) {
        oriterm_ui::platform_windows::set_chrome_metrics(window, border_width, caption_height);
    }
}

/// Scale a logical-pixel rect to physical pixels.
fn scale_rect(r: Rect, scale: f32) -> Rect {
    Rect::new(
        r.x() * scale,
        r.y() * scale,
        r.width() * scale,
        r.height() * scale,
    )
}

/// Extracts the raw HWND from a winit `Window`.
fn extract_hwnd(window: &Window) -> Option<HWND> {
    oriterm_ui::platform_windows::hwnd_from_window(window)
}
