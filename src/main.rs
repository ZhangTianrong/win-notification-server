use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use windows::{
    core::*,
    Win32::System::Registry::*,
    Win32::Foundation::*,
    Win32::System::Com::*,
    Data::Xml::Dom::*,
    UI::Notifications::*,
    Foundation::{EventRegistrationToken, TypedEventHandler},
};

// Using a more standard AppUserModelID format
const APP_ID: &str = "TyroneCheung.NotificationServer";
const APP_DISPLAY_NAME: &str = "Notification Server";

#[derive(Debug, Serialize, Deserialize)]
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

struct NotificationManager {
    is_registered: bool,
    notifier: Option<ToastNotifier>,
    _com_initialized: bool,
}

impl NotificationManager {
    async fn new() -> Result<Self> {
        // Initialize COM for Windows Runtime
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok();
        }

        let mut manager = NotificationManager {
            is_registered: false,
            notifier: None,
            _com_initialized: true,
        };
        
        manager.ensure_registration()?;
        manager.initialize_notifier()?;
        manager.is_registered = true;
        Ok(manager)
    }

    fn initialize_notifier(&mut self) -> Result<()> {
        log::info!("Initializing toast notifier with APP_ID: {}", APP_ID);
        // Create the toast notifier with our app ID
        let aumid: HSTRING = APP_ID.into();
        let notifier = unsafe { ToastNotificationManager::CreateToastNotifierWithId(&aumid)? };
        self.notifier = Some(notifier);
        log::info!("Toast notifier initialized successfully");
        Ok(())
    }

    fn ensure_registration(&self) -> Result<()> {
        log::info!("Ensuring application registration...");
        // Create registry keys for notification settings
        self.register_app_id()?;
        self.register_notification_settings()?;
        self.register_aumid()?;
        log::info!("Application registration completed successfully");
        Ok(())
    }

    fn register_app_id(&self) -> Result<()> {
        log::info!("Registering application ID...");
        // Register application in Windows registry
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

            // Set the default value to the current executable path
            let exe_path = std::env::current_exe()?;
            let exe_path_str = exe_path.to_string_lossy().to_string();
            let mut exe_path_wide: Vec<u16> = exe_path_str.encode_utf16().collect();
            exe_path_wide.push(0); // Null terminate
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

            // Set display name
            let display_name = APP_DISPLAY_NAME.to_string();
            let mut display_name_wide: Vec<u16> = display_name.encode_utf16().collect();
            display_name_wide.push(0); // Null terminate
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

            // Set ShowInSettings
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

            // Enable notifications
            let enabled: u32 = 1;
            let status = RegSetValueExW(
                key,
                w!("Enabled"),
                0,
                REG_DWORD,
                Some(&enabled.to_ne_bytes()),
            );

            // Set sound enabled
            let status2 = RegSetValueExW(
                key,
                w!("Sound"),
                0,
                REG_DWORD,
                Some(&enabled.to_ne_bytes()),
            );

            // Set additional notification settings
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

    async fn send_notification(&self, request: NotificationRequest) -> Result<()> {
        log::info!("Sending notification: {:?}", request);
        
        if !self.is_registered {
            return Err(anyhow::anyhow!("Notification system not properly registered"));
        }

        if let Some(xml) = request.xml_payload {
            self.send_xml_notification(&xml).await?;
        } else {
            self.send_basic_notification(&request).await?;
        }
        Ok(())
    }

    async fn send_basic_notification(&self, request: &NotificationRequest) -> Result<()> {
        log::info!("Preparing basic notification");
        let toast_xml = format!(
            r#"<toast launch="action=mainContent" activationType="protocol" duration="long">
                <visual>
                    <binding template="ToastGeneric">
                        <text>{}</text>
                        <text>{}</text>
                        {}
                    </binding>
                </visual>
                {}
                <audio src="ms-winsoundevent:Notification.Default"/>
            </toast>"#,
            request.title,
            request.message,
            request.image_data.as_ref().map_or(String::new(), |img| {
                format!(r#"<image placement="appLogoOverride" src="data:image/png;base64,{}"/>"#, img)
            }),
            request.callback_command.as_ref().map_or(String::new(), |cmd| {
                format!(r#"<actions><action content="Run" arguments="cmd:{}"/></actions>"#, cmd)
            })
        );

        self.send_xml_notification(&toast_xml).await
    }

    async fn send_xml_notification(&self, xml: &str) -> Result<()> {
        log::info!("Sending XML notification: {}", xml);
        
        // Create XML document
        let xml_doc: XmlDocument = XmlDocument::new()?;
        let xml_string: HSTRING = xml.into();
        xml_doc.LoadXml(&xml_string)?;

        // Create toast notification
        let notification = ToastNotification::CreateToastNotification(&xml_doc)?;

        // Set up notification event handlers
        let _token: EventRegistrationToken = notification.Activated(&TypedEventHandler::<
            ToastNotification,
            IInspectable,
        >::new(|_, args| {
            if let Some(args) = args {
                if let Ok(args) = args.cast::<ToastActivatedEventArgs>() {
                    if let Ok(arguments) = args.Arguments() {
                        let args_str = arguments.to_string_lossy();
                        log::info!("Notification activated with arguments: {}", args_str);
                        if let Some(cmd) = args_str.strip_prefix("cmd:") {
                            if let Err(e) = std::process::Command::new("cmd")
                                .args(&["/C", cmd])
                                .spawn() {
                                log::error!("Failed to execute callback command: {}", e);
                            }
                        }
                    }
                }
            }
            Ok(())
        }))?;

        // Show notification
        if let Some(notifier) = &self.notifier {
            notifier.Show(&notification)?;
            log::info!("Notification sent successfully");
        } else {
            return Err(anyhow::anyhow!("Toast notifier not initialized"));
        }

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
    let manager = manager.lock().await;
    match manager.send_notification(request.0).await {
        Ok(_) => HttpResponse::Ok().body("Notification sent successfully"),
        Err(e) => {
            log::error!("Failed to send notification: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to send notification: {}", e))
        }
    }
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
    .run()
    .await?;

    Ok(())
}
