//! `AppKit` / Core Graphics type definitions for `ObjC` FFI.
//!
//! Minimal `repr(C)` structs matching Apple's 64-bit ABI, with `Encode`
//! impls so `objc2::msg_send!` can pass and return them correctly.

/// `NSPoint` / `CGPoint` — two `f64` fields on 64-bit macOS.
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct NSPoint {
    pub x: f64,
    pub y: f64,
}

/// `NSRect` / `CGRect` — origin + size, four `f64` fields on 64-bit macOS.
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct NSRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

// SAFETY: These are plain C structs matching Apple's ABI for msg_send returns.
unsafe impl objc2::Encode for NSPoint {
    const ENCODING: objc2::Encoding = objc2::Encoding::Struct(
        "CGPoint",
        &[objc2::Encoding::Double, objc2::Encoding::Double],
    );
}

unsafe impl objc2::Encode for NSRect {
    const ENCODING: objc2::Encoding = objc2::Encoding::Struct(
        "CGRect",
        &[
            objc2::Encoding::Struct(
                "CGPoint",
                &[objc2::Encoding::Double, objc2::Encoding::Double],
            ),
            objc2::Encoding::Struct(
                "CGSize",
                &[objc2::Encoding::Double, objc2::Encoding::Double],
            ),
        ],
    );
}
