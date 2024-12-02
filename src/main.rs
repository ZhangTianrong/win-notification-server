use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Instant;
use windows::{
    core::*,
    Win32::System::Registry::*,
    Win32::Foundation::*,
    Win32::System::Com::*,
    Win32::System::DataExchange::*,
    Win32::System::Memory::*,
    Data::Xml::Dom::*,
    UI::Notifications::*,
    Foundation::TypedEventHandler,
};

const APP_ID: &str = "TyroneCheung.NotificationServer";
const APP_DISPLAY_NAME: &str = "Notification Server";
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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct NotificationRequest {
    title: String,
    message: String,
    #[serde(default)]
    xml_payload: Option<String>,
    #[serde(default)]
    image_data: Option<String>,
    #[serde(default)]
    callback_command: Option<String>,
}

#[derive(Clone)]
struct NotificationData {
    callback_command: Option<String>,
    message: String,
}

struct NotificationManager {
    is_registered: bool,
    notifier: Option<ToastNotifier>,
    notifications: Arc<Mutex<HashMap<String, NotificationData>>>,
    _com_initialized: bool,
    xml_doc: Option<XmlDocument>,
}

impl NotificationManager {
    async fn new() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok();
        }

        let mut manager = NotificationManager {
            is_registered: false,
            notifier: None,
            notifications: Arc::new(Mutex::new(HashMap::new())),
            _com_initialized: true,
            xml_doc: Some(XmlDocument::new()?),
        };
        
        manager.ensure_registration()?;
        manager.initialize_notifier()?;
        manager.is_registered = true;
        Ok(manager)
    }

    fn set_clipboard_text(text: &str) -> Result<()> {
        log::info!("Attempting to copy text to clipboard: {}", text);
        
        unsafe {
            // Try to open clipboard once with a short timeout
            if !OpenClipboard(HWND(0)).as_bool() {
                std::thread::sleep(std::time::Duration::from_millis(50));
                if !OpenClipboard(HWND(0)).as_bool() {
                    log::error!("Failed to open clipboard");
                    return Ok(());
                }
            }

            // Clear existing content
            let _ = EmptyClipboard();

            // Convert to UTF-16 and add null terminator
            let mut text_utf16: Vec<u16> = text.encode_utf16().collect();
            text_utf16.push(0);
            let byte_len = text_utf16.len() * 2;

            // Allocate memory in one go
            let h_mem = GlobalAlloc(GMEM_MOVEABLE, byte_len)?;
            let p_mem = GlobalLock(h_mem);
            
            if !p_mem.is_null() {
                std::ptr::copy_nonoverlapping(
                    text_utf16.as_ptr() as *const u8,
                    p_mem as *mut u8,
                    byte_len
                );

                GlobalUnlock(h_mem);

                if SetClipboardData(13u32, HANDLE(h_mem.0)).is_ok() {
                    log::info!("Text successfully copied to clipboard");
                } else {
                    log::error!("Failed to set clipboard data");
                    let _ = GlobalFree(h_mem);
                }
            } else {
                log::error!("Failed to lock global memory");
                let _ = GlobalFree(h_mem);
            }

            CloseClipboard();
        }
        Ok(())
    }

    fn initialize_notifier(&mut self) -> Result<()> {
        log::info!("Initializing toast notifier with APP_ID: {}", APP_ID);
        let aumid: HSTRING = APP_ID.into();
        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&aumid)?;
        self.notifier = Some(notifier);
        log::info!("Toast notifier initialized successfully");
        Ok(())
    }

    fn ensure_registration(&self) -> Result<()> {
        log::info!("Ensuring application registration...");
        self.register_app_id()?;
        self.register_notification_settings()?;
        self.register_aumid()?;
        log::info!("Application registration completed successfully");
        Ok(())
    }

    fn register_app_id(&self) -> Result<()> {
        log::info!("Registering application ID...");
        let app_key_path = format!("SOFTWARE\\Classes\\AppUserModelId\\{}", APP_ID);
        let mut key = HKEY::default();
        
        unsafe {
            let status = RegCreateKeyW(
                HKEY_CURRENT_USER,
                &HSTRING::from(app_key_path),
                &mut key,
            );
            
            if status != ERROR_SUCCESS {
                return Err(anyhow::anyhow!("Failed to create app registry key: {:?}", status));
            }

            let exe_path = std::env::current_exe()?;
            let exe_path_str = exe_path.to_string_lossy().to_string();
            let mut exe_path_wide: Vec<u16> = exe_path_str.encode_utf16().collect();
            exe_path_wide.push(0);
            let bytes = std::slice::from_raw_parts(exe_path_wide.as_ptr() as *const u8, exe_path_wide.len() * 2);
            
            let status = RegSetValueExW(
                key,
                w!(""),
                0,
                REG_SZ,
                Some(bytes),
            );

            RegCloseKey(key);

            if status != ERROR_SUCCESS {
                return Err(anyhow::anyhow!("Failed to set app path: {:?}", status));
            }
        }

        log::info!("Application ID registered successfully");
        Ok(())
    }

    fn register_aumid(&self) -> Result<()> {
        log::info!("Registering AppUserModelID...");
        let key_path = format!("SOFTWARE\\Classes\\AppUserModelId\\{}", APP_ID);
        let mut key = HKEY::default();
        
        unsafe {
            let status = RegCreateKeyW(
                HKEY_CURRENT_USER,
                &HSTRING::from(key_path),
                &mut key,
            );
            
            if status != ERROR_SUCCESS {
                return Err(anyhow::anyhow!("Failed to create AUMID registry key: {:?}", status));
            }

            let display_name = APP_DISPLAY_NAME.to_string();
            let mut display_name_wide: Vec<u16> = display_name.encode_utf16().collect();
            display_name_wide.push(0);
            let bytes = std::slice::from_raw_parts(display_name_wide.as_ptr() as *const u8, display_name_wide.len() * 2);
            
            let status = RegSetValueExW(
                key,
                w!("DisplayName"),
                0,
                REG_SZ,
                Some(bytes),
            );

            if status != ERROR_SUCCESS {
                RegCloseKey(key);
                return Err(anyhow::anyhow!("Failed to set display name: {:?}", status));
            }

            let enabled: u32 = 1;
            let status = RegSetValueExW(
                key,
                w!("ShowInSettings"),
                0,
                REG_DWORD,
                Some(&enabled.to_ne_bytes()),
            );

            RegCloseKey(key);

            if status != ERROR_SUCCESS {
                return Err(anyhow::anyhow!("Failed to set ShowInSettings: {:?}", status));
            }
        }

        log::info!("AppUserModelID registered successfully");
        Ok(())
    }

    fn register_notification_settings(&self) -> Result<()> {
        log::info!("Registering notification settings...");
        let key_path = format!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Notifications\\Settings\\{}", APP_ID);
        let mut key = HKEY::default();
        
        unsafe {
            let status = RegCreateKeyW(
                HKEY_CURRENT_USER,
                &HSTRING::from(key_path),
                &mut key,
            );
            
            if status != ERROR_SUCCESS {
                return Err(anyhow::anyhow!("Failed to create notification settings key: {:?}", status));
            }

            let enabled: u32 = 1;
            let status = RegSetValueExW(
                key,
                w!("Enabled"),
                0,
                REG_DWORD,
                Some(&enabled.to_ne_bytes()),
            );

            let status2 = RegSetValueExW(
                key,
                w!("Sound"),
                0,
                REG_DWORD,
                Some(&enabled.to_ne_bytes()),
            );

            let status3 = RegSetValueExW(
                key,
                w!("ShowInActionCenter"),
                0,
                REG_DWORD,
                Some(&enabled.to_ne_bytes()),
            );

            RegCloseKey(key);

            if status != ERROR_SUCCESS || status2 != ERROR_SUCCESS || status3 != ERROR_SUCCESS {
                return Err(anyhow::anyhow!("Failed to set notification settings"));
            }
        }

        log::info!("Notification settings registered successfully");
        Ok(())
    }

    async fn send_notification(&mut self, request: NotificationRequest) -> Result<()> {
        let start = Instant::now();
        log::info!("Sending notification: {:?}", request);
        
        if !self.is_registered {
            return Err(anyhow::anyhow!("Notification system not properly registered"));
        }

        if let Some(xml) = request.xml_payload {
            self.send_xml_notification(&xml, request.callback_command.clone(), request.message.clone()).await?;
        } else {
            self.send_basic_notification(&request).await?;
        }
        log::info!("Notification processing completed in {:?}", start.elapsed());
        Ok(())
    }

    async fn send_basic_notification(&mut self, request: &NotificationRequest) -> Result<()> {
        let start = Instant::now();
        log::info!("Preparing basic notification");
        
        let tag = format!("notification_{}", uuid::Uuid::new_v4());
        
        let toast_xml = TOAST_TEMPLATE
            .replace("{tag}", &tag)
            .replace("{title}", &request.title.replace("&", "&amp;").replace("\"", "&quot;").replace("<", "&lt;").replace(">", "&gt;"))
            .replace("{message}", &request.message.replace("&", "&amp;").replace("\"", "&quot;").replace("<", "&lt;").replace(">", "&gt;"))
            .replace("{image}", &request.image_data.as_ref().map_or(String::new(), |img| {
                format!("<image placement=\"appLogoOverride\" src=\"data:image/png;base64,{}\"/>", img)
            }));

        let notification_data = NotificationData {
            callback_command: request.callback_command.clone(),
            message: request.message.clone(),
        };
        self.notifications.lock().unwrap().insert(tag.clone(), notification_data);

        log::info!("Basic notification prepared in {:?}", start.elapsed());
        self.send_xml_notification(&toast_xml, request.callback_command.clone(), request.message.clone()).await
    }

    async fn send_xml_notification(&mut self, xml: &str, callback: Option<String>, message: String) -> Result<()> {
        let start = Instant::now();
        log::info!("Sending XML notification: {}", xml);
        
        let xml_doc = self.xml_doc.as_ref().unwrap();
        let xml_string: HSTRING = xml.into();
        xml_doc.LoadXml(&xml_string)?;

        let notification = ToastNotification::CreateToastNotification(xml_doc)?;
        
        let tag = format!("notification_{}", uuid::Uuid::new_v4());
        notification.SetTag(&HSTRING::from(tag.clone()))?;

        let notification_data = NotificationData {
            callback_command: callback.clone(),
            message: message.clone(),
        };
        self.notifications.lock().unwrap().insert(tag.clone(), notification_data);

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
                        if let Err(e) = NotificationManager::set_clipboard_text(&data.message) {
                            log::error!("Failed to copy text to clipboard: {}", e);
                        }
                    }
                }
            }
            Ok(())
        }))?;

        let tag_clone = tag.clone();
        let notifications = Arc::clone(&self.notifications);
        let _token = notification.Dismissed(&TypedEventHandler::<ToastNotification, ToastDismissedEventArgs>::new(move |_: &Option<ToastNotification>, args: &Option<ToastDismissedEventArgs>| {
            if let Some(args) = args {
                if let Ok(reason) = args.Reason() {
                    match reason {
                        ToastDismissalReason::UserCanceled => {
                            log::info!("Notification dismissed from action center (UserCanceled)");
                            let tag = tag_clone.clone();
                            
                            if let Ok(notifications_guard) = notifications.lock() {
                                if let Some(data) = notifications_guard.get(&tag) {
                                    if let Some(cmd) = &data.callback_command {
                                        log::info!("Executing callback command for action center dismissal: {}", cmd);
                                        if let Err(e) = std::process::Command::new("cmd")
                                            .args(&["/C", cmd])
                                            .spawn() {
                                            log::error!("Failed to execute dismiss callback: {}", e);
                                        }
                                    } else {
                                        if let Err(e) = NotificationManager::set_clipboard_text(&data.message) {
                                            log::error!("Failed to copy text to clipboard: {}", e);
                                        }
                                    }
                                }
                            }
                        },
                        ToastDismissalReason::TimedOut => {
                            log::info!("Notification timed out (TimedOut)");
                        },
                        ToastDismissalReason::ApplicationHidden => {
                            log::info!("Notification hidden by application (ApplicationHidden)");
                        },
                        _ => {
                            log::info!("Notification dismissed with unknown reason: {:?}", reason);
                        }
                    }
                }
            }
            Ok(())
        }))?;

        let tag_clone = tag.clone();
        let _token = notification.Failed(&TypedEventHandler::<ToastNotification, ToastFailedEventArgs>::new(move |_: &Option<ToastNotification>, _: &Option<ToastFailedEventArgs>| {
            log::error!("Notification failed: {}", tag_clone);
            Ok(())
        }))?;

        if let Some(notifier) = &self.notifier {
            let show_start = Instant::now();
            notifier.Show(&notification)?;
            log::info!("Show notification call took {:?}", show_start.elapsed());
            log::info!("Notification sent successfully");
        } else {
            return Err(anyhow::anyhow!("Toast notifier not initialized"));
        }

        log::info!("XML notification processing completed in {:?}", start.elapsed());
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

async fn send_notification(
    manager: web::Data<Arc<Mutex<NotificationManager>>>,
    request: web::Json<NotificationRequest>,
) -> impl Responder {
    let start = Instant::now();
    log::info!("Received notification request at {:?}", start);
    
    let mut manager = manager.lock().unwrap();
    let result = match manager.send_notification(request.0).await {
        Ok(_) => {
            log::info!("Request completed successfully in {:?}", start.elapsed());
            HttpResponse::Ok().body("Notification sent successfully")
        },
        Err(e) => {
            log::error!("Failed to send notification: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to send notification: {}", e))
        }
    };
    
    log::info!("Total request handling time: {:?}", start.elapsed());
    result
}

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    log::info!("Initializing notification manager...");
    let manager = Arc::new(Mutex::new(
        NotificationManager::new().await.context("Failed to create notification manager")?
    ));
    log::info!("Notification manager initialized successfully");

    println!("Starting notification server on http://localhost:3000");
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(manager.clone()))
            .route("/notify", web::post().to(send_notification))
    })
    .bind("127.0.0.1:3000")?
    .workers(4) // Reduced from 16 to 4 workers
    .run()
    .await?;

    Ok(())
}
