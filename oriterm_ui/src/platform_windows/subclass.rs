//! `WndProc` subclass and Win32 message handlers.
//!
//! Contains the subclass callback installed by [`super::enable_snap()`] and
//! the individual message handlers it dispatches to.

#![allow(unsafe_code)]

use windows_sys::Win32::Foundation::{HWND, LRESULT, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::InvalidateRect;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTCLIENT,
    HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, IsZoomed, KillTimer, NCCALCSIZE_PARAMS,
    SM_CXFRAME, SM_CXPADDEDBORDER, SM_CYFRAME, SW_HIDE, SWP_NOACTIVATE, SWP_NOZORDER, SetTimer,
    SetWindowPos, ShowWindow, WM_DPICHANGED, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_MOVING,
    WM_NCCALCSIZE, WM_NCDESTROY, WM_NCHITTEST, WM_TIMER,
};

use std::sync::atomic::Ordering;

use crate::geometry::{Point, Size};
use crate::hit_test::{self, HitTestResult, ResizeDirection};

use super::{
    IN_MODAL_LOOP, MODAL_TIMER_ID, MODAL_TIMER_MS, OsDragResult, SUBCLASS_ID, SnapData, snap_ptrs,
};

fn get_x_lparam(lp: isize) -> i32 {
    i32::from((lp & 0xFFFF) as i16)
}

fn get_y_lparam(lp: isize) -> i32 {
    i32::from(((lp >> 16) & 0xFFFF) as i16)
}

/// Maps a [`HitTestResult`] to a Windows HT constant.
fn map_hit_result(result: HitTestResult) -> LRESULT {
    (match result {
        HitTestResult::Client => HTCLIENT,
        HitTestResult::Caption => HTCAPTION,
        HitTestResult::ResizeBorder(dir) => match dir {
            ResizeDirection::Top => HTTOP,
            ResizeDirection::Bottom => HTBOTTOM,
            ResizeDirection::Left => HTLEFT,
            ResizeDirection::Right => HTRIGHT,
            ResizeDirection::TopLeft => HTTOPLEFT,
            ResizeDirection::TopRight => HTTOPRIGHT,
            ResizeDirection::BottomLeft => HTBOTTOMLEFT,
            ResizeDirection::BottomRight => HTBOTTOMRIGHT,
        },
    }) as LRESULT
}

/// Handles `WM_NCHITTEST` by delegating to [`hit_test::hit_test()`].
///
/// Converts screen coordinates to client-relative physical pixels and
/// delegates to the pure hit test function. Resize borders are checked
/// before interactive rects so corners near window controls remain
/// resizable.
///
/// All coordinates are in physical pixels — `lparam` screen cursor,
/// visible frame bounds, and stored `interactive_rects` are all physical.
///
/// Uses `DWMWA_EXTENDED_FRAME_BOUNDS` (via [`super::visible_frame_bounds_hwnd`])
/// instead of `GetWindowRect` for coordinate conversion. On Windows 10/11,
/// `GetWindowRect` includes invisible DWM borders (~7px per side) for
/// windows with `WS_THICKFRAME`, but the client area starts at the visible
/// boundary. Using the DWM visible bounds ensures the screen-to-client
/// conversion matches winit's `CursorMoved` coordinates.
fn handle_nchittest(hwnd: HWND, lparam: isize, data: &SnapData) -> LRESULT {
    let cursor_x = get_x_lparam(lparam);
    let cursor_y = get_y_lparam(lparam);

    // Visible frame bounds in screen coordinates (physical pixels).
    // Uses DWMWA_EXTENDED_FRAME_BOUNDS to exclude the invisible DWM border
    // that GetWindowRect includes for WS_THICKFRAME windows.
    let rect = super::visible_frame_bounds_hwnd(hwnd);

    // Client-relative physical coordinates.
    let point = Point::new((cursor_x - rect.left) as f32, (cursor_y - rect.top) as f32);

    // Physical window size from the actual window rect.
    let window_size = Size::new(
        (rect.right - rect.left) as f32,
        (rect.bottom - rect.top) as f32,
    );

    let is_maximized = unsafe { IsZoomed(hwnd) != 0 };

    let border_width = data.border_width.lock().map(|g| *g).unwrap_or(0.0);
    let caption_height = data.caption_height.lock().map(|g| *g).unwrap_or(0.0);
    let rects_lock = data.interactive_rects.lock();
    let rects: &[crate::geometry::Rect] = rects_lock.as_ref().map(|g| g.as_slice()).unwrap_or(&[]);
    let chrome = hit_test::WindowChrome {
        window_size,
        border_width,
        caption_height,
        interactive_rects: rects,
        is_maximized,
    };
    let result = hit_test::hit_test(point, &chrome);
    let lresult = map_hit_result(result);

    log::trace!(
        "nchittest: screen=({cursor_x},{cursor_y}) vfb=({},{},{},{}) \
         client=({:.0},{:.0}) size=({:.0},{:.0}) max={is_maximized} \
         bw={border_width:.1} ch={caption_height:.1} rects={rects:?} \
         result={result:?} ht={lresult}",
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        point.x,
        point.y,
        window_size.width(),
        window_size.height(),
    );

    lresult
}

/// Handles `WM_MOVING`: position correction + cursor-based merge detection.
///
/// Modifies the proposed rect via `lparam` for position correction.
/// If a merge is detected, hides the window and releases capture.
/// Caller always calls `DefSubclassProc` afterward.
fn handle_moving(hwnd: HWND, lparam: isize, data: &SnapData) {
    let Ok(mut lock) = data.os_drag.lock() else {
        return;
    };
    let Some(state) = lock.as_mut() else {
        return;
    };

    let proposed = unsafe { &mut *(lparam as *mut RECT) };
    let w = proposed.right - proposed.left;
    let h = proposed.bottom - proposed.top;

    // Always correct position: window origin = cursor - grab_offset.
    let mut pt = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&raw mut pt) };
    let (gx, gy) = state.grab_offset;
    proposed.left = pt.x - gx;
    proposed.top = pt.y - gy;
    proposed.right = proposed.left + w;
    proposed.bottom = proposed.top + h;

    // Skip merge check during cooldown (position still corrected).
    if state.skip_remaining > 0 {
        state.skip_remaining -= 1;
        return;
    }

    // Cursor-based merge detection (Chrome's DoesTabStripContain pattern).
    for &[cl, ct, cr, ctb] in &state.merge_rects {
        if pt.x >= cl && pt.x < cr && pt.y >= ct && pt.y < ctb {
            state.result = Some(OsDragResult::MergeDetected {
                cursor: (pt.x, pt.y),
            });
            // Hide window + release capture to end the move loop.
            unsafe {
                ShowWindow(hwnd, SW_HIDE);
                ReleaseCapture();
            }
            return;
        }
    }
}

/// `WndProc` subclass callback installed by [`super::enable_snap()`].
///
/// `ref_data` is a valid `*const SnapData` allocated in `enable_snap` and
/// freed in the `WM_NCDESTROY` handler.
pub(super) unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
    _uid: usize,
    ref_data: usize,
) -> LRESULT {
    unsafe {
        let data = &*(ref_data as *const SnapData);

        match msg {
            // Return 0 so the entire window is client area (no OS frame).
            // When maximized, inset by frame thickness to prevent
            // adjacent-monitor bleed (Chrome's GetClientAreaInsets pattern).
            WM_NCCALCSIZE if wparam == 1 => {
                if IsZoomed(hwnd) != 0 {
                    let params = &mut *(lparam as *mut NCCALCSIZE_PARAMS);
                    let fx = GetSystemMetrics(SM_CXFRAME) + GetSystemMetrics(SM_CXPADDEDBORDER);
                    let fy = GetSystemMetrics(SM_CYFRAME) + GetSystemMetrics(SM_CXPADDEDBORDER);
                    params.rgrc[0].left += fx;
                    params.rgrc[0].top += fy;
                    params.rgrc[0].right -= fx;
                    params.rgrc[0].bottom -= fy;
                }
                0
            }

            WM_NCHITTEST => handle_nchittest(hwnd, lparam, data),

            WM_DPICHANGED => {
                // HIWORD(wParam) = new Y-axis DPI.
                let new_dpi = ((wparam >> 16) & 0xFFFF) as u32;
                data.last_dpi.store(new_dpi, Ordering::Relaxed);

                // Apply OS-suggested rect to prevent DPI oscillation.
                let suggested = &*(lparam as *const RECT);
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                0
            }

            WM_ENTERSIZEMOVE => {
                IN_MODAL_LOOP.store(true, Ordering::Relaxed);
                SetTimer(hwnd, MODAL_TIMER_ID, MODAL_TIMER_MS, None);
                DefSubclassProc(hwnd, msg, wparam, lparam)
            }

            WM_TIMER if wparam == MODAL_TIMER_ID => {
                // Invalidate all windows so the modal message pump
                // generates WM_PAINT → RedrawRequested for each.
                if let Ok(map) = snap_ptrs().lock() {
                    for &hwnd_key in map.keys() {
                        InvalidateRect(hwnd_key as HWND, std::ptr::null(), 0);
                    }
                }
                0
            }

            WM_MOVING => {
                handle_moving(hwnd, lparam, data);
                DefSubclassProc(hwnd, msg, wparam, lparam)
            }

            WM_EXITSIZEMOVE => {
                KillTimer(hwnd, MODAL_TIMER_ID);
                IN_MODAL_LOOP.store(false, Ordering::Relaxed);
                if let Ok(mut lock) = data.os_drag.lock() {
                    if let Some(state) = lock.as_mut() {
                        if state.result.is_none() {
                            let mut pt = POINT { x: 0, y: 0 };
                            GetCursorPos(&raw mut pt);
                            state.result = Some(OsDragResult::DragEnded {
                                cursor: (pt.x, pt.y),
                            });
                        }
                    }
                }
                DefSubclassProc(hwnd, msg, wparam, lparam)
            }

            WM_NCDESTROY => {
                RemoveWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID);
                if let Ok(mut map) = snap_ptrs().lock() {
                    map.remove(&(hwnd as usize));
                }
                drop(Box::from_raw(ref_data as *mut SnapData));
                DefSubclassProc(hwnd, msg, wparam, lparam)
            }

            _ => DefSubclassProc(hwnd, msg, wparam, lparam),
        }
    }
}
