//! UI hint effects — signals that the UI layer should update visual state.

/// UI hints emitted by the terminal to the UI layer.
///
/// These are lightweight signals, not data payloads. The UI layer
/// queries the terminal state for the actual values.
#[derive(Debug, Clone, Copy)]
pub enum UiEffect {
    /// Cursor blink mode changed (DECSET/DECRST 12 or CSI q).
    CursorBlinkChanged { enabled: bool },
    /// Mouse cursor shape may need updating (e.g. after mode change).
    MouseCursorDirty,
}
