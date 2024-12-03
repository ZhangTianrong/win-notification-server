use anyhow::Result;
use serde::{Deserialize, Serialize};
use windows::UI::Notifications::ToastNotification;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotificationRequest {
    pub title: String,
    pub message: String,
    #[serde(default)]
    pub xml_payload: Option<String>,
    #[serde(default)]
    pub image_path: Option<String>,
    #[serde(default)]
    pub callback_command: Option<String>,
}

#[derive(Clone)]
pub struct NotificationData {
    pub callback_command: Option<String>,
    pub message: String,
}

pub trait NotificationType {
    fn prepare_xml(&self) -> Result<String>;
    fn create_notification(&self, xml: &str) -> Result<ToastNotification>;
    fn get_callback_data(&self) -> NotificationData;
}
