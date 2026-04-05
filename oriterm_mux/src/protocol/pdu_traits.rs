//! PDU classification methods and wire helpers.
//!
//! Extracted from `messages.rs` to keep that file under the 500-line limit.
//! Contains the match-heavy `msg_type()`, `is_fire_and_forget()`, and
//! `is_notification()` methods plus the `theme_to_wire()` helper.

use oriterm_core::Theme;

use super::messages::MuxPdu;
use super::msg_type::MsgType;

impl MuxPdu {
    /// Message type ID for the wire header.
    pub(crate) fn msg_type(&self) -> MsgType {
        match self {
            Self::Hello { .. } => MsgType::Hello,
            Self::ClosePane { .. } => MsgType::ClosePane,
            Self::Input { .. } => MsgType::Input,
            Self::Resize { .. } => MsgType::Resize,
            Self::Subscribe { .. } => MsgType::Subscribe,
            Self::Unsubscribe { .. } => MsgType::Unsubscribe,
            Self::GetPaneSnapshot { .. } => MsgType::GetPaneSnapshot,
            Self::Ping => MsgType::Ping,
            Self::Shutdown => MsgType::Shutdown,
            Self::ScrollDisplay { .. } => MsgType::ScrollDisplay,
            Self::ScrollToBottom { .. } => MsgType::ScrollToBottom,
            Self::ScrollToPrompt { .. } => MsgType::ScrollToPrompt,
            Self::SetTheme { .. } => MsgType::SetTheme,
            Self::SetCursorShape { .. } => MsgType::SetCursorShape,
            Self::SetBoldIsBright { .. } => MsgType::SetBoldIsBright,
            Self::MarkAllDirty { .. } => MsgType::MarkAllDirty,
            Self::OpenSearch { .. } => MsgType::OpenSearch,
            Self::CloseSearch { .. } => MsgType::CloseSearch,
            Self::SearchSetQuery { .. } => MsgType::SearchSetQuery,
            Self::SearchNextMatch { .. } => MsgType::SearchNextMatch,
            Self::SearchPrevMatch { .. } => MsgType::SearchPrevMatch,
            Self::ExtractText { .. } => MsgType::ExtractText,
            Self::ExtractHtml { .. } => MsgType::ExtractHtml,
            Self::SetCapabilities { .. } => MsgType::SetCapabilities,
            Self::RequestNewTab => MsgType::RequestNewTab,
            Self::SetPanePriority { .. } => MsgType::SetPanePriority,
            Self::SpawnPane { .. } => MsgType::SpawnPane,
            Self::ListPanes => MsgType::ListPanes,
            Self::SetImageConfig { .. } => MsgType::SetImageConfig,
            Self::HelloAck { .. } => MsgType::HelloAck,
            Self::PaneClosedAck => MsgType::PaneClosedAck,
            Self::Subscribed { .. } => MsgType::Subscribed,
            Self::Unsubscribed => MsgType::Unsubscribed,
            Self::PaneSnapshotResp { .. } => MsgType::PaneSnapshotResp,
            Self::PingAck => MsgType::PingAck,
            Self::ShutdownAck => MsgType::ShutdownAck,
            Self::ScrollToPromptAck { .. } => MsgType::ScrollToPromptAck,
            Self::ExtractTextResp { .. } => MsgType::ExtractTextResp,
            Self::ExtractHtmlResp { .. } => MsgType::ExtractHtmlResp,
            Self::SpawnPaneResponse { .. } => MsgType::SpawnPaneResponse,
            Self::ListPanesResponse { .. } => MsgType::ListPanesResponse,
            Self::NewTabAck => MsgType::NewTabAck,
            Self::Error { .. } => MsgType::Error,
            Self::NotifyNewTab => MsgType::NotifyNewTab,
            Self::NotifyPaneOutput { .. } => MsgType::NotifyPaneOutput,
            Self::NotifyPaneExited { .. } => MsgType::NotifyPaneExited,
            Self::NotifyPaneMetadataChanged { .. } => MsgType::NotifyPaneMetadataChanged,
            Self::NotifyPaneBell { .. } => MsgType::NotifyPaneBell,
            Self::NotifyCommandComplete { .. } => MsgType::NotifyCommandComplete,
            Self::NotifyClipboardStore { .. } => MsgType::NotifyClipboardStore,
            Self::NotifyClipboardLoad { .. } => MsgType::NotifyClipboardLoad,
            Self::NotifyPaneSnapshot { .. } => MsgType::NotifyPaneSnapshot,
        }
    }

    /// Whether this PDU is a fire-and-forget message (no response expected).
    pub fn is_fire_and_forget(&self) -> bool {
        matches!(
            self,
            Self::Input { .. }
                | Self::Resize { .. }
                | Self::ScrollDisplay { .. }
                | Self::ScrollToBottom { .. }
                | Self::SetTheme { .. }
                | Self::SetCursorShape { .. }
                | Self::SetBoldIsBright { .. }
                | Self::MarkAllDirty { .. }
                | Self::OpenSearch { .. }
                | Self::CloseSearch { .. }
                | Self::SearchSetQuery { .. }
                | Self::SearchNextMatch { .. }
                | Self::SearchPrevMatch { .. }
                | Self::SetCapabilities { .. }
                | Self::SetImageConfig { .. }
                | Self::SetPanePriority { .. }
        )
    }

    /// Whether this PDU is a push notification from the daemon.
    pub fn is_notification(&self) -> bool {
        matches!(
            self,
            Self::NotifyPaneOutput { .. }
                | Self::NotifyPaneExited { .. }
                | Self::NotifyPaneMetadataChanged { .. }
                | Self::NotifyPaneBell { .. }
                | Self::NotifyCommandComplete { .. }
                | Self::NotifyClipboardStore { .. }
                | Self::NotifyClipboardLoad { .. }
                | Self::NotifyPaneSnapshot { .. }
                | Self::NotifyNewTab
        )
    }
}

/// Convert a [`Theme`] to its wire representation.
///
/// Returns `Some("dark")` or `Some("light")`, or `None` for
/// [`Theme::Unknown`] (server uses its default). Callers `.map(str::to_owned)`
/// at the serialization boundary when building PDU fields.
pub(crate) fn theme_to_wire(theme: Theme) -> Option<&'static str> {
    match theme {
        Theme::Dark => Some("dark"),
        Theme::Light => Some("light"),
        Theme::Unknown => None,
    }
}
