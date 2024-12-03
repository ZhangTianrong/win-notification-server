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
use std::path::Path;

use crate::notifications::{NotificationRequest, NotificationData, NotificationType, BasicNotification, NotificationKind};
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

        match request.notification_type {
            NotificationKind::Basic => {
                let notification = BasicNotification::from(request);
                self.send_typed_notification(&notification).await?;
            }
            // Add future notification types here
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
            Ok(())
        } else {
            Err(anyhow::anyhow!("Toast notifier not initialized"))
        }
    }

    fn setup_notification_handlers(&self, notification: &ToastNotification, tag: String) -> Result<()> {
        let notifications = Arc::clone(&self.notifications);

        let tag_clone = tag.clone();
        let _token = notification.Activated(&TypedEventHandler::<ToastNotification, IInspectable>::new(move |_: &Option<ToastNotification>, _: &Option<IInspectable>| {
            log::info!("Notification clicked (Activated event)");
            let tag = tag_clone.clone();
            
            if let Ok(notifications_guard) = notifications.lock() {
                if let Some(data) = notifications_guard.get(&tag) {
                    // Handle callback command if present
                    if let Some(cmd) = &data.callback_command {
                        log::info!("Executing callback command for click: {}", cmd);
                        if let Err(e) = std::process::Command::new("cmd")
                            .args(&["/C", cmd])
                            .spawn() {
                            log::error!("Failed to execute click callback: {}", e);
                        }
                    } else {
                        // Copy message to clipboard if no callback command
                        if let Err(e) = ClipboardService::set_text(&data.message) {
                            log::error!("Failed to copy text to clipboard: {}", e);
                        }
                    }

                    // Determine which directory to open
                    let directory_to_open = if let Some(image_path) = &data.image_path {
                        // If there's an image, use its directory
                        Path::new(image_path).parent().map(|p| p.to_path_buf())
                    } else if let Some(file_paths) = &data.file_paths {
                        if !file_paths.is_empty() {
                            // If there are files but no image, use the directory of the first file
                            Path::new(&file_paths[0]).parent().map(|p| p.to_path_buf())
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // Open the directory if one was found
                    if let Some(dir) = directory_to_open {
                        log::info!("Opening directory: {}", dir.display());
                        if let Err(e) = std::process::Command::new("explorer")
                            .arg(dir.to_str().unwrap_or(""))
                            .spawn() {
                            log::error!("Failed to open directory: {}", e);
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
