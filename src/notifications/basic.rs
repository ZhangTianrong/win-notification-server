use anyhow::Result;
use windows::{
    core::*,
    UI::Notifications::*,
    Data::Xml::Dom::*,
};
use std::path::Path;
use super::types::{NotificationType, NotificationData};

pub struct BasicNotification {
    pub title: String,
    pub message: String,
    pub image_path: Option<String>,
    pub file_paths: Option<Vec<String>>,
    pub callback_command: Option<String>,
}

const TOAST_TEMPLATE: &str = r#"<toast launch="action=mainContent&amp;tag={tag}" activationType="foreground" duration="long">
    <visual>
        <binding template="ToastGeneric">
            {image}
            <text>{title}</text>
            <text>{message}</text>
        </binding>
    </visual>
    <audio src="ms-winsoundevent:Notification.Default"/>
</toast>"#;

impl NotificationType for BasicNotification {
    fn prepare_xml(&self) -> Result<String> {
        let tag = format!("notification_{}", uuid::Uuid::new_v4());
        
        let image_xml = if let Some(img_path) = &self.image_path {
            let path = Path::new(img_path);
            if !path.exists() {
                log::error!("Image file not found: {}", path.display());
                return Err(anyhow::anyhow!("Image file not found"));
            }

            format!("<image placement=\"hero\" src=\"{}\" />", 
                path.to_string_lossy())
        } else {
            String::new()
        };

        log::debug!("Generated image XML: {}", image_xml);

        let toast_xml = TOAST_TEMPLATE
            .replace("{tag}", &tag)
            .replace("{title}", &escape_xml(&self.title))
            .replace("{message}", &escape_xml(&self.message))
            .replace("{image}", &image_xml);

        log::debug!("Generated toast XML: {}", toast_xml);
        Ok(toast_xml)
    }

    fn create_notification(&self, xml: &str) -> Result<ToastNotification> {
        log::debug!("Creating notification with XML: {}", xml);
        let xml_doc = XmlDocument::new()?;
        let xml_string: HSTRING = xml.into();
        xml_doc.LoadXml(&xml_string)?;
        
        let notification = ToastNotification::CreateToastNotification(&xml_doc)?;
        let tag = format!("notification_{}", uuid::Uuid::new_v4());
        notification.SetTag(&HSTRING::from(tag))?;

        Ok(notification)
    }

    fn get_callback_data(&self) -> NotificationData {
        NotificationData {
            callback_command: self.callback_command.clone(),
            message: self.message.clone(),
            image_path: self.image_path.clone(),
            file_paths: self.file_paths.clone(),
        }
    }
}

impl From<super::types::NotificationRequest> for BasicNotification {
    fn from(request: super::types::NotificationRequest) -> Self {
        BasicNotification {
            title: request.title,
            message: request.message,
            image_path: request.image_path,
            file_paths: request.file_paths,
            callback_command: request.callback_command,
        }
    }
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
