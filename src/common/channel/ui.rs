//! Non-React UI commands sent from the JS runtime to the GPUI app thread.

use crate::common::{ids::NativeObjectId, mount::NodeValue};

#[derive(Debug, Clone, PartialEq)]
pub enum UiCommand {
    ShowNotification(NotificationCommandPayload),
    DismissNotification {
        id: String,
    },
    ClearNotifications,
    ChartAppendData {
        node_id: NativeObjectId,
        rows: Vec<NodeValue>,
    },
    ChartReplaceData {
        node_id: NativeObjectId,
        rows: Vec<NodeValue>,
    },
    ChartClearData {
        node_id: NativeObjectId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationCommandPayload {
    pub id: Option<String>,
    pub type_: NotificationType,
    pub title: Option<String>,
    pub message: String,
    pub autohide: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationType {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}
