//! Input event types for widget-level event routing.
//!
//! These are widget-oriented events, distinct from platform events (winit).
//! The application layer translates platform events into these types before
//! routing them through the widget tree.

use crate::geometry::Point;

/// Mouse button identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Primary (usually left) button.
    Left,
    /// Secondary (usually right) button.
    Right,
    /// Middle (wheel) button.
    Middle,
}

/// Keyboard modifier flags stored as a bitmask.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(u8);

impl Modifiers {
    /// No modifiers pressed.
    pub const NONE: Self = Self(0);

    const SHIFT: u8 = 1;
    const CTRL: u8 = 2;
    const ALT: u8 = 4;
    const LOGO: u8 = 8;

    /// Modifier with only Shift set.
    pub const SHIFT_ONLY: Self = Self(Self::SHIFT);

    /// Modifier with only Ctrl set.
    pub const CTRL_ONLY: Self = Self(Self::CTRL);

    /// Modifier with only Alt set.
    pub const ALT_ONLY: Self = Self(Self::ALT);

    /// Modifier with only Logo/Super set.
    pub const LOGO_ONLY: Self = Self(Self::LOGO);

    /// Combines this modifier set with another (bitwise OR).
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Returns `true` if Shift is held.
    pub const fn shift(self) -> bool {
        self.0 & Self::SHIFT != 0
    }

    /// Returns `true` if Ctrl is held.
    pub const fn ctrl(self) -> bool {
        self.0 & Self::CTRL != 0
    }

    /// Returns `true` if Alt/Option is held.
    pub const fn alt(self) -> bool {
        self.0 & Self::ALT != 0
    }

    /// Returns `true` if Super/Win/Cmd is held.
    pub const fn logo(self) -> bool {
        self.0 & Self::LOGO != 0
    }
}

impl std::fmt::Debug for Modifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Modifiers")
            .field("shift", &self.shift())
            .field("ctrl", &self.ctrl())
            .field("alt", &self.alt())
            .field("logo", &self.logo())
            .finish()
    }
}

/// Scroll delta in pixels or lines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollDelta {
    /// Delta in pixels (trackpad).
    Pixels { x: f32, y: f32 },
    /// Delta in lines (mouse wheel).
    Lines { x: f32, y: f32 },
}

/// What kind of mouse event occurred.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseEventKind {
    /// Button pressed.
    Down(MouseButton),
    /// Button released.
    Up(MouseButton),
    /// Cursor moved.
    Move,
    /// Scroll wheel or trackpad scroll.
    Scroll(ScrollDelta),
}

/// A mouse event in widget-local coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseEvent {
    /// What happened.
    pub kind: MouseEventKind,
    /// Cursor position in widget-local coordinates.
    pub pos: Point,
    /// Active keyboard modifiers.
    pub modifiers: Modifiers,
}

/// A keyboard key relevant to widget interaction.
///
/// Much simpler than winit's key model — only keys that widgets actually
/// handle. The app layer translates platform key events into this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Enter/Return.
    Enter,
    /// Space bar.
    Space,
    /// Backspace.
    Backspace,
    /// Delete (forward delete).
    Delete,
    /// Escape.
    Escape,
    /// Tab key (not to be confused with Tab cycling — that is handled by
    /// the focus manager before reaching widgets).
    Tab,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Up arrow.
    ArrowUp,
    /// Down arrow.
    ArrowDown,
    /// Left arrow.
    ArrowLeft,
    /// Right arrow.
    ArrowRight,
    /// Page Up.
    PageUp,
    /// Page Down.
    PageDown,
    /// A character key (after dead-key / IME composition).
    Character(char),
}

/// A keyboard event in the widget event model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    /// Which key was pressed.
    pub key: Key,
    /// Active keyboard modifiers.
    pub modifiers: Modifiers,
}

/// Unified input event for the two-phase propagation pipeline.
///
/// Replaces the separate `MouseEvent` / `KeyEvent` types for routing purposes.
/// The old types remain during the transition period (Sections 03–08) for
/// compatibility with existing `Widget::handle_mouse()` / `handle_key()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    /// Mouse button pressed.
    MouseDown {
        /// Cursor position in widget-local coordinates.
        pos: Point,
        /// Which button was pressed.
        button: MouseButton,
        /// Active keyboard modifiers.
        modifiers: Modifiers,
    },
    /// Mouse button released.
    MouseUp {
        /// Cursor position in widget-local coordinates.
        pos: Point,
        /// Which button was released.
        button: MouseButton,
        /// Active keyboard modifiers.
        modifiers: Modifiers,
    },
    /// Cursor moved.
    MouseMove {
        /// Cursor position in widget-local coordinates.
        pos: Point,
        /// Active keyboard modifiers.
        modifiers: Modifiers,
    },
    /// Scroll wheel or trackpad scroll.
    Scroll {
        /// Cursor position in widget-local coordinates.
        pos: Point,
        /// Scroll delta.
        delta: ScrollDelta,
        /// Active keyboard modifiers.
        modifiers: Modifiers,
    },
    /// Key pressed.
    KeyDown {
        /// Which key was pressed.
        key: Key,
        /// Active keyboard modifiers.
        modifiers: Modifiers,
    },
    /// Key released.
    KeyUp {
        /// Which key was released.
        key: Key,
        /// Active keyboard modifiers.
        modifiers: Modifiers,
    },
}

impl InputEvent {
    /// Returns the cursor position for mouse events, `None` for keyboard events.
    pub fn pos(&self) -> Option<Point> {
        match self {
            Self::MouseDown { pos, .. }
            | Self::MouseUp { pos, .. }
            | Self::MouseMove { pos, .. }
            | Self::Scroll { pos, .. } => Some(*pos),
            Self::KeyDown { .. } | Self::KeyUp { .. } => None,
        }
    }

    /// Returns `true` for mouse-related events.
    pub fn is_mouse(&self) -> bool {
        matches!(
            self,
            Self::MouseDown { .. }
                | Self::MouseUp { .. }
                | Self::MouseMove { .. }
                | Self::Scroll { .. }
        )
    }

    /// Returns `true` for keyboard events.
    pub fn is_keyboard(&self) -> bool {
        matches!(self, Self::KeyDown { .. } | Self::KeyUp { .. })
    }

    /// Converts to a legacy `MouseEvent` for the transition bridge.
    ///
    /// Returns `None` for keyboard events.
    pub fn to_mouse_event(self) -> Option<MouseEvent> {
        match self {
            Self::MouseDown {
                pos,
                button,
                modifiers,
            } => Some(MouseEvent {
                kind: MouseEventKind::Down(button),
                pos,
                modifiers,
            }),
            Self::MouseUp {
                pos,
                button,
                modifiers,
            } => Some(MouseEvent {
                kind: MouseEventKind::Up(button),
                pos,
                modifiers,
            }),
            Self::MouseMove { pos, modifiers } => Some(MouseEvent {
                kind: MouseEventKind::Move,
                pos,
                modifiers,
            }),
            Self::Scroll {
                pos,
                delta,
                modifiers,
            } => Some(MouseEvent {
                kind: MouseEventKind::Scroll(delta),
                pos,
                modifiers,
            }),
            Self::KeyDown { .. } | Self::KeyUp { .. } => None,
        }
    }

    /// Converts from a legacy `MouseEvent` for the transition bridge.
    ///
    /// Inverse of `to_mouse_event()`. Used by the overlay manager to
    /// convert legacy mouse events into `InputEvent` for controller dispatch.
    pub fn from_mouse_event(event: &MouseEvent) -> Self {
        match event.kind {
            MouseEventKind::Down(button) => Self::MouseDown {
                pos: event.pos,
                button,
                modifiers: event.modifiers,
            },
            MouseEventKind::Up(button) => Self::MouseUp {
                pos: event.pos,
                button,
                modifiers: event.modifiers,
            },
            MouseEventKind::Move => Self::MouseMove {
                pos: event.pos,
                modifiers: event.modifiers,
            },
            MouseEventKind::Scroll(delta) => Self::Scroll {
                pos: event.pos,
                delta,
                modifiers: event.modifiers,
            },
        }
    }

    /// Converts from a legacy `KeyEvent` for the transition bridge.
    ///
    /// Produces `KeyDown` — overlays only handle key presses, not releases.
    pub fn from_key_event(event: KeyEvent) -> Self {
        Self::KeyDown {
            key: event.key,
            modifiers: event.modifiers,
        }
    }

    /// Converts to a legacy `KeyEvent` for the transition bridge.
    ///
    /// Returns `None` for mouse events.
    pub fn to_key_event(self) -> Option<KeyEvent> {
        match self {
            Self::KeyDown { key, modifiers } | Self::KeyUp { key, modifiers } => {
                Some(KeyEvent { key, modifiers })
            }
            _ => None,
        }
    }
}

/// Phase of event propagation through the widget tree.
///
/// Events flow in two phases: Capture (root to target) then Bubble (target
/// to root). The Target phase marks the widget the event is addressed to.
/// Inspired by WPF's Preview/Bubble paired propagation and GTK4's
/// three-phase controller model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventPhase {
    /// Root to target. Parents see the event first and can intercept.
    Capture,
    /// Event reaches the target widget.
    Target,
    /// Target to root. Standard handling — children handle first.
    Bubble,
}
