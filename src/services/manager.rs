use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use windows::{
    core::*,
    Win32::System::Com::*,
    UI::Notifications::*,
    Data::Xml::Dom::*,
    Foundation::TypedEventHandler,
};

use crate::notifications::{NotificationRequest, NotificationData, NotificationType, BasicNotification};
use super::registry::RegistryService;
use super::clipboard::ClipboardService;

pub struct NotificationManager {
    is_registered: bool,
    notifier: Option<ToastNotifier>,
    notifications: Arc<Mutex<HashMap<String, NotificationData>>>,
    _com_initialized: bool,
    registry_service: RegistryService,
}

impl NotificationManager {
    pub async fn new(app_id: &str, display_name: &str) -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok();
        }

        let registry_service = RegistryService::new(app_id, display_name);
        
        let mut manager = NotificationManager {
            is_registered: false,
            notifier: None,
            notifications: Arc::new(Mutex::new(HashMap::new())),
            _com_initialized: true,
            registry_service,
        };
        
        manager.ensure_registration()?;
        manager.initialize_notifier(app_id)?;
        manager.is_registered = true;
        Ok(manager)
    }

    fn initialize_notifier(&mut self, app_id: &str) -> Result<()> {
        log::info!("Initializing toast notifier with APP_ID: {}", app_id);
        let aumid: HSTRING = app_id.into();
        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&aumid)?;
        self.notifier = Some(notifier);
        log::info!("Toast notifier initialized successfully");
        Ok(())
    }

    fn ensure_registration(&self) -> Result<()> {
        log::info!("Ensuring application registration...");
        self.registry_service.ensure_registration()?;
        log::info!("Application registration completed successfully");
        Ok(())
    }

    pub async fn send_notification(&mut self, request: NotificationRequest) -> Result<()> {
        if !self.is_registered {
            return Err(anyhow::anyhow!("Notification system not properly registered"));
        }

        if let Some(xml) = request.xml_payload.clone() {
            self.send_xml_notification(&xml, request.callback_command.clone(), request.message.clone()).await?;
        } else {
            let notification = BasicNotification::from(request);
            self.send_typed_notification(&notification).await?;
        }
        
        Ok(())
    }

    async fn send_typed_notification<T: NotificationType>(&mut self, notification_type: &T) -> Result<()> {
        let xml = notification_type.prepare_xml()?;
        let toast = notification_type.create_notification(&xml)?;
        let notification_data = notification_type.get_callback_data();
        
        let tag = format!("notification_{}", uuid::Uuid::new_v4());
        toast.SetTag(&HSTRING::from(tag.clone()))?;

        self.notifications.lock().unwrap().insert(tag.clone(), notification_data.clone());
        self.setup_notification_handlers(&toast, tag)?;

        if let Some(notifier) = &self.notifier {
            notifier.Show(&toast)?;
            log::info!("Notification sent successfully");
        } else {
            return Err(anyhow::anyhow!("Toast notifier not initialized"));
        }

        Ok(())
    }

    async fn send_xml_notification(&mut self, xml: &str, callback: Option<String>, message: String) -> Result<()> {
        let xml_doc = XmlDocument::new()?;
        let xml_string: HSTRING = xml.into();
        xml_doc.LoadXml(&xml_string)?;

        let notification = ToastNotification::CreateToastNotification(&xml_doc)?;
        let tag = format!("notification_{}", uuid::Uuid::new_v4());
        notification.SetTag(&HSTRING::from(tag.clone()))?;

        let notification_data = NotificationData {
            callback_command: callback,
            message,
        };
        
        self.notifications.lock().unwrap().insert(tag.clone(), notification_data);
        self.setup_notification_handlers(&notification, tag)?;

        if let Some(notifier) = &self.notifier {
            notifier.Show(&notification)?;
            log::info!("Notification sent successfully");
        } else {
            return Err(anyhow::anyhow!("Toast notifier not initialized"));
        }

        Ok(())
    }

    fn setup_notification_handlers(&self, notification: &ToastNotification, tag: String) -> Result<()> {
        let notifications = Arc::clone(&self.notifications);

        let tag_clone = tag.clone();
        let _token = notification.Activated(&TypedEventHandler::<ToastNotification, IInspectable>::new(move |_: &Option<ToastNotification>, _: &Option<IInspectable>| {
            log::info!("Notification clicked (Activated event)");
            let tag = tag_clone.clone();
            
            if let Ok(notifications_guard) = notifications.lock() {
                if let Some(data) = notifications_guard.get(&tag) {
                    if let Some(cmd) = &data.callback_command {
                        log::info!("Executing callback command for click: {}", cmd);
                        if let Err(e) = std::process::Command::new("cmd")
                            .args(&["/C", cmd])
                            .spawn() {
                            log::error!("Failed to execute click callback: {}", e);
                        }
                    } else {
                        if let Err(e) = ClipboardService::set_text(&data.message) {
                            log::error!("Failed to copy text to clipboard: {}", e);
                        }
                    }
                }
            }
            Ok(())
        }))?;

        let _token = notification.Dismissed(&TypedEventHandler::<ToastNotification, ToastDismissedEventArgs>::new(move |_: &Option<ToastNotification>, args: &Option<ToastDismissedEventArgs>| {
            if let Some(args) = args {
                if let Ok(reason) = args.Reason() {
                    match reason {
                        ToastDismissalReason::UserCanceled => {
                            log::info!("Notification dismissed by user - no action taken");
                        },
                        ToastDismissalReason::TimedOut => {
                            log::info!("Notification timed out");
                        },
                        ToastDismissalReason::ApplicationHidden => {
                            log::info!("Notification hidden by application");
                        },
                        _ => {
                            log::info!("Notification dismissed with unknown reason: {:?}", reason);
                        }
                    }
                }
            }
            Ok(())
        }))?;

        let tag_clone = tag;
        let _token = notification.Failed(&TypedEventHandler::<ToastNotification, ToastFailedEventArgs>::new(move |_: &Option<ToastNotification>, _: &Option<ToastFailedEventArgs>| {
            log::error!("Notification failed: {}", tag_clone);
            Ok(())
        }))?;

        Ok(())
    }
}

impl Drop for NotificationManager {
    fn drop(&mut self) {
        if self._com_initialized {
            unsafe {
                CoUninitialize();
            }
        }
    }
}
