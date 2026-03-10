//! Vocabulary types for the window management system.
//!
//! Every OS window in the application is represented as a [`ManagedWindow`]
//! with a [`WindowKind`] discriminant. Context menus and dropdowns remain as
//! in-window overlays via `OverlayManager` — only heavyweight windows (main
//! terminals, dialogs, tear-offs) become real OS windows tracked here.

use winit::window::WindowId;

/// Discriminates the role and behavior of a managed window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowKind {
    /// Primary terminal window with tab bar, grid, chrome.
    Main,
    /// Dialog window (settings, confirmation, about).
    /// Owned by a parent main window. Has UI-only rendering.
    /// Destroyed when parent closes.
    Dialog(DialogKind),
    /// Tear-off window created by dragging a tab out of an existing window.
    /// Behaviorally identical to `Main` after creation; the kind tracks
    /// origin for merge detection.
    #[allow(
        dead_code,
        reason = "window manager API — wired during main window migration"
    )]
    TearOff,
}

impl WindowKind {
    /// Returns `true` for primary terminal windows.
    pub fn is_main(&self) -> bool {
        matches!(self, Self::Main)
    }

    /// Returns `true` for dialog windows of any subkind.
    pub fn is_dialog(&self) -> bool {
        matches!(self, Self::Dialog(_))
    }

    /// Returns `true` for tear-off windows.
    #[allow(
        dead_code,
        reason = "window manager API — wired during main window migration"
    )]
    pub fn is_tear_off(&self) -> bool {
        matches!(self, Self::TearOff)
    }

    /// Returns `true` for windows that count as "primary" for exit-on-close
    /// logic (main or tear-off).
    pub fn is_primary(&self) -> bool {
        matches!(self, Self::Main | Self::TearOff)
    }
}

/// Specific dialog types with their own content and behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKind {
    /// Settings / preferences dialog.
    Settings,
    /// Confirmation prompt (e.g. close with running processes).
    Confirmation,
    /// About dialog (version info, credits).
    #[allow(
        dead_code,
        reason = "dialog kind — wired when About dialog is implemented"
    )]
    About,
}

impl DialogKind {
    /// Whether this dialog kind is modal (blocks parent input).
    pub fn is_modal(self) -> bool {
        matches!(self, Self::Confirmation)
    }

    /// Whether this dialog kind supports user resizing.
    pub fn is_resizable(self) -> bool {
        matches!(self, Self::Settings)
    }

    /// Default window title for this dialog kind.
    pub fn title(self) -> &'static str {
        match self {
            Self::Settings => "Settings",
            Self::Confirmation => "Confirm",
            Self::About => "About",
        }
    }

    /// Default logical inner size `(width, height)` for this dialog kind.
    pub fn default_size(self) -> (u32, u32) {
        match self {
            Self::Settings => (720, 560),
            Self::Confirmation => (440, 240),
            Self::About => (400, 300),
        }
    }
}

/// A tracked OS window in the window manager.
#[derive(Debug, Clone)]
pub struct ManagedWindow {
    /// Winit window ID (for event routing from the OS).
    pub winit_id: WindowId,
    /// Window kind (determines behavior and rendering pipeline).
    pub kind: WindowKind,
    /// Parent window (for dialogs and initially for tear-offs).
    /// `None` for root-level main windows.
    pub parent: Option<WindowId>,
    /// Child windows owned by this window.
    /// Destroyed when this window closes.
    pub children: Vec<WindowId>,
    /// Whether the window is currently visible.
    #[allow(
        dead_code,
        reason = "window manager API — read during main window migration"
    )]
    pub visible: bool,
}

/// Request to create a new OS window through the window manager.
///
/// The caller fills this out and passes it to
/// [`WindowManager::prepare_create`](super::WindowManager::prepare_create)
/// to get the corresponding `WindowAttributes`.
#[allow(
    dead_code,
    reason = "window manager API — used when prepare_create is wired"
)]
#[derive(Debug, Clone)]
pub struct WindowRequest {
    /// What kind of window to create.
    pub kind: WindowKind,
    /// Parent window (for dialogs and tear-offs).
    pub parent: Option<WindowId>,
    /// Window title.
    pub title: String,
    /// Initial inner size in logical pixels.
    pub size: Option<(u32, u32)>,
    /// Initial position in logical pixels.
    pub position: Option<(i32, i32)>,
    /// Whether the window starts visible.
    pub visible: bool,
    /// Whether the window has OS decorations.
    pub decorations: bool,
}

impl ManagedWindow {
    /// Create a new managed window with no children.
    pub fn new(winit_id: WindowId, kind: WindowKind) -> Self {
        Self {
            winit_id,
            kind,
            parent: None,
            children: Vec::new(),
            visible: true,
        }
    }

    /// Create a new managed window owned by `parent`.
    pub fn with_parent(winit_id: WindowId, kind: WindowKind, parent: WindowId) -> Self {
        Self {
            winit_id,
            kind,
            parent: Some(parent),
            children: Vec::new(),
            visible: true,
        }
    }
}
