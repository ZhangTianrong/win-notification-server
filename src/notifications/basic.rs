use anyhow::Result;
use windows::{
    core::*,
    UI::Notifications::*,
    Data::Xml::Dom::*,
};
use super::types::{NotificationType, NotificationData};

pub struct BasicNotification {
    pub title: String,
    pub message: String,
    pub image_data: Option<String>,
    pub callback_command: Option<String>,
}

const TOAST_TEMPLATE: &str = r#"<toast launch="action=mainContent&amp;tag={tag}" activationType="foreground" duration="long">
    <visual>
        <binding template="ToastGeneric">
            <text>{title}</text>
            <text>{message}</text>
            {image}
        </binding>
    </visual>
    <audio src="ms-winsoundevent:Notification.Default"/>
</toast>"#;

impl NotificationType for BasicNotification {
    fn prepare_xml(&self) -> Result<String> {
        let tag = format!("notification_{}", uuid::Uuid::new_v4());
        
        let toast_xml = TOAST_TEMPLATE
            .replace("{tag}", &tag)
            .replace("{title}", &escape_xml(&self.title))
            .replace("{message}", &escape_xml(&self.message))
            .replace("{image}", &self.image_data.as_ref().map_or(String::new(), |img| {
                format!("<image placement=\"appLogoOverride\" src=\"data:image/png;base64,{}\"/>", img)
            }));

        Ok(toast_xml)
    }

    fn create_notification(&self, xml: &str) -> Result<ToastNotification> {
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
        }
    }
}

impl From<super::types::NotificationRequest> for BasicNotification {
    fn from(request: super::types::NotificationRequest) -> Self {
        BasicNotification {
            title: request.title,
            message: request.message,
            image_data: request.image_data,
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
