use anyhow::Result;
use serde::{Deserialize, Serialize};
use windows::UI::Notifications::ToastNotification;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum NotificationKind {
    Basic,
    // Future notification types can be added here
}

impl Default for NotificationKind {
    fn default() -> Self {
        NotificationKind::Basic
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotificationRequest {
    pub title: String,
    pub message: String,
    #[serde(default)]
    pub notification_type: NotificationKind,
    #[serde(default)]
    pub image_path: Option<String>,
    #[serde(default)]
    pub file_paths: Option<Vec<String>>,
    #[serde(default)]
    pub callback_command: Option<String>,
}

#[derive(Clone)]
pub struct NotificationData {
    pub callback_command: Option<String>,
    pub message: String,
    pub image_path: Option<String>,
    pub file_paths: Option<Vec<String>>,
}

pub trait NotificationType {
    fn prepare_xml(&self) -> Result<String>;
    fn create_notification(&self, xml: &str) -> Result<ToastNotification>;
    fn get_callback_data(&self) -> NotificationData;
}
