//! Config-driven window creation for frameless Chrome-style CSD.
//!
//! All platforms use frameless windows from day one. Windows are created
//! invisible so the first frame can be rendered before showing, preventing
//! a white flash.

use std::fmt;
use std::sync::Arc;

use winit::event_loop::ActiveEventLoop;
use winit::window::{Icon, Window, WindowAttributes};

use crate::geometry::{Point, Size};

/// Window decoration mode.
///
/// Controls whether the window uses OS-native decorations, custom frameless
/// CSD, or platform-specific variants like macOS transparent titlebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DecorationMode {
    /// Frameless window with custom CSD (default).
    #[default]
    Frameless,
    /// OS-native title bar and borders.
    Native,
    /// macOS: transparent titlebar with traffic lights visible.
    /// Other platforms: equivalent to `Frameless`.
    TransparentTitlebar,
    /// macOS: hide traffic light buttons.
    /// Other platforms: equivalent to `Frameless`.
    Buttonless,
}

impl DecorationMode {
    /// Returns whether two modes share the same macOS creation-time titlebar
    /// attributes (transparent, fullsize content view, hidden buttons).
    ///
    /// Transitions between modes in different groups require an app restart on
    /// macOS because winit cannot change these attributes at runtime.
    ///
    /// Groups:
    /// - `Native`: no titlebar transparency, no fullsize content view.
    /// - `Frameless` / `TransparentTitlebar`: transparent + fullsize.
    /// - `Buttonless`: transparent + fullsize + hidden buttons.
    pub fn macos_requires_restart(self, other: Self) -> bool {
        self.macos_titlebar_group() != other.macos_titlebar_group()
    }

    /// Classifies the mode by its macOS creation-time window attributes.
    fn macos_titlebar_group(self) -> u8 {
        match self {
            Self::Native => 0,
            Self::Frameless | Self::TransparentTitlebar => 1,
            Self::Buttonless => 2,
        }
    }
}

/// Configuration for creating a new window.
///
/// Scale factor is not included — it is a runtime property of the display,
/// not a configuration input. Query `window.scale_factor()` after creation.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Window title.
    pub title: String,
    /// Logical inner size in device-independent pixels.
    pub inner_size: Size,
    /// Enable transparent background (compositor alpha blending).
    pub transparent: bool,
    /// Enable background blur (macOS vibrancy, Windows Acrylic/Mica).
    pub blur: bool,
    /// Window opacity `[0.0, 1.0]`. Values >= 1.0 are fully opaque.
    pub opacity: f32,
    /// Initial window position, or `None` for OS default.
    pub position: Option<Point>,
    /// Whether the window is resizable. Defaults to `true`.
    pub resizable: bool,
    /// Window decoration mode (frameless CSD, native, or platform variant).
    pub decoration: DecorationMode,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: String::from("oriterm"),
            inner_size: Size::new(1024.0, 768.0),
            transparent: false,
            blur: false,
            opacity: 1.0,
            position: None,
            resizable: true,
            decoration: DecorationMode::default(),
        }
    }
}

/// Errors that can occur during window creation.
#[derive(Debug)]
pub enum WindowError {
    /// The windowing system refused to create the window.
    Creation(winit::error::OsError),
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Creation(e) => write!(f, "window creation failed: {e}"),
        }
    }
}

impl std::error::Error for WindowError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Creation(e) => Some(e),
        }
    }
}

impl From<winit::error::OsError> for WindowError {
    fn from(e: winit::error::OsError) -> Self {
        Self::Creation(e)
    }
}

/// Creates a new frameless window from the given configuration.
///
/// The window is created invisible. Call `window.set_visible(true)` after
/// rendering the first frame to avoid a white flash.
pub fn create_window(
    event_loop: &ActiveEventLoop,
    config: &WindowConfig,
) -> Result<Arc<Window>, WindowError> {
    let attrs = build_window_attributes(config);
    let window = event_loop.create_window(attrs)?;
    apply_post_creation_style(&window);
    Ok(Arc::new(window))
}

/// Resolve [`DecorationMode`] into the winit `with_decorations` boolean.
///
/// macOS: `Frameless` and `TransparentTitlebar` both enable winit decorations
/// (traffic lights visible; the frameless look comes from transparent titlebar +
/// fullsize content view). `Buttonless` also enables decorations but hides the
/// traffic lights in `apply_platform_attributes`. `Native` enables standard
/// decorations.
///
/// Other platforms: `Native` = decorated, everything else = frameless CSD.
pub fn resolve_winit_decorations(mode: DecorationMode) -> bool {
    #[cfg(target_os = "macos")]
    {
        // macOS needs winit decorations ON for all modes except Native
        // (where it's also on). The frameless look is achieved via
        // titlebar transparency + fullsize content view, not by
        // disabling decorations. Disabling decorations on macOS removes
        // the traffic lights AND breaks the titlebar area entirely.
        let _ = mode;
        true
    }
    #[cfg(not(target_os = "macos"))]
    {
        matches!(mode, DecorationMode::Native)
    }
}

/// Builds platform-aware [`WindowAttributes`] from a [`WindowConfig`].
///
/// All platforms share a frameless, initially-invisible window. Per-platform
/// `#[cfg]` blocks add OS-specific attributes.
fn build_window_attributes(config: &WindowConfig) -> WindowAttributes {
    let decorations = resolve_winit_decorations(config.decoration);

    let mut attrs = WindowAttributes::default()
        .with_title(&config.title)
        .with_inner_size(winit::dpi::LogicalSize::new(
            config.inner_size.width(),
            config.inner_size.height(),
        ))
        .with_decorations(decorations)
        .with_visible(false)
        .with_resizable(config.resizable)
        .with_transparent(config.transparent);

    if let Some(pos) = config.position {
        attrs = attrs.with_position(winit::dpi::LogicalPosition::new(pos.x, pos.y));
    }

    if let Some(icon) = load_icon() {
        attrs = attrs.with_window_icon(Some(icon));
    }

    attrs = apply_platform_attributes(attrs, config);
    attrs
}

/// Loads the embedded application icon (256x256 RGBA, decoded at build time).
///
/// Returns `None` if the icon data is malformed.
fn load_icon() -> Option<Icon> {
    static ICON_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon_rgba.bin"));

    if ICON_DATA.len() < 8 {
        return None;
    }

    let w = u32::from_le_bytes([ICON_DATA[0], ICON_DATA[1], ICON_DATA[2], ICON_DATA[3]]);
    let h = u32::from_le_bytes([ICON_DATA[4], ICON_DATA[5], ICON_DATA[6], ICON_DATA[7]]);
    let rgba = &ICON_DATA[8..];

    let expected_len = (w as usize) * (h as usize) * 4;
    if rgba.len() != expected_len {
        log::warn!(
            "icon RGBA data length mismatch: expected {expected_len}, got {}",
            rgba.len()
        );
        return None;
    }

    Icon::from_rgba(rgba.to_vec(), w, h).ok()
}

/// Applies platform-specific window attributes.
#[cfg(target_os = "windows")]
fn apply_platform_attributes(attrs: WindowAttributes, config: &WindowConfig) -> WindowAttributes {
    use winit::platform::windows::WindowAttributesExtWindows;

    let mut attrs = attrs;
    if config.transparent {
        // DirectComposition requires no redirection bitmap for alpha blending.
        attrs = attrs.with_no_redirection_bitmap(true);
    }
    attrs
}

/// Applies platform-specific window attributes.
///
/// macOS decoration modes:
/// - `Frameless` / `TransparentTitlebar`: transparent titlebar + fullsize content view
///   (traffic lights visible, content extends behind titlebar).
/// - `Buttonless`: same as above but hides traffic light buttons.
/// - `Native`: standard macOS titlebar (no transparency, no fullsize content).
#[cfg(target_os = "macos")]
fn apply_platform_attributes(attrs: WindowAttributes, config: &WindowConfig) -> WindowAttributes {
    use winit::platform::macos::{OptionAsAlt, WindowAttributesExtMacOS};

    let attrs = attrs.with_option_as_alt(OptionAsAlt::Both);

    match config.decoration {
        DecorationMode::Native => attrs,
        DecorationMode::Buttonless => attrs
            .with_titlebar_transparent(true)
            .with_title_hidden(true)
            .with_fullsize_content_view(true)
            .with_titlebar_buttons_hidden(true),
        // Frameless and TransparentTitlebar: traffic lights visible, content behind titlebar.
        _ => attrs
            .with_titlebar_transparent(true)
            .with_title_hidden(true)
            .with_fullsize_content_view(true),
    }
}

/// Applies platform-specific window attributes.
#[cfg(target_os = "linux")]
fn apply_platform_attributes(attrs: WindowAttributes, _config: &WindowConfig) -> WindowAttributes {
    use winit::platform::x11::WindowAttributesExtX11;

    // WM_CLASS for X11 window managers (used for taskbar grouping, rules).
    attrs.with_name("oriterm", "oriterm")
}

/// Applies platform-specific window attributes (fallback for other platforms).
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn apply_platform_attributes(attrs: WindowAttributes, _config: &WindowConfig) -> WindowAttributes {
    attrs
}

/// Applies post-creation window style (sharp corners on Windows 11).
#[cfg(target_os = "windows")]
#[allow(
    unsafe_code,
    reason = "Win32 FFI: DwmSetWindowAttribute for sharp corners"
)]
fn apply_post_creation_style(window: &Window) {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };
    let hwnd = win32.hwnd.get() as windows_sys::Win32::Foundation::HWND;

    // DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWCP_DONOTROUND = 1.
    let preference: i32 = windows_sys::Win32::Graphics::Dwm::DWMWCP_DONOTROUND;
    let attr = windows_sys::Win32::Graphics::Dwm::DWMWA_WINDOW_CORNER_PREFERENCE;
    // SAFETY: `hwnd` is a valid window handle from winit. The attribute and
    // value are well-typed DWM constants. This is standard Win32 FFI.
    unsafe {
        windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(
            hwnd,
            attr as u32,
            std::ptr::addr_of!(preference).cast(),
            size_of::<i32>() as u32,
        );
    }
}

/// Post-creation style is a no-op on non-Windows platforms.
#[cfg(not(target_os = "windows"))]
fn apply_post_creation_style(_window: &Window) {}

#[cfg(test)]
mod tests;
