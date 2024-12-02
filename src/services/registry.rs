use anyhow::{Result, anyhow};
use windows::{
    core::*,
    Win32::System::Registry::*,
    Win32::Foundation::*,
};

pub struct RegistryService {
    app_id: String,
    display_name: String,
}

impl RegistryService {
    pub fn new(app_id: &str, display_name: &str) -> Self {
        Self {
            app_id: app_id.to_string(),
            display_name: display_name.to_string(),
        }
    }

    pub fn ensure_registration(&self) -> Result<()> {
        self.register_app_id()?;
        self.register_notification_settings()?;
        self.register_aumid()?;
        Ok(())
    }

    fn register_app_id(&self) -> Result<()> {
        let app_key_path = format!("SOFTWARE\\Classes\\AppUserModelId\\{}", self.app_id);
        let mut key = HKEY::default();
        
        unsafe {
            let status = RegCreateKeyW(
                HKEY_CURRENT_USER,
                &HSTRING::from(app_key_path),
                &mut key,
            );
            
            if status != ERROR_SUCCESS {
                return Err(anyhow!("Failed to create app registry key: {:?}", status));
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
                return Err(anyhow!("Failed to set app path: {:?}", status));
            }
        }

        Ok(())
    }

    fn register_aumid(&self) -> Result<()> {
        let key_path = format!("SOFTWARE\\Classes\\AppUserModelId\\{}", self.app_id);
        let mut key = HKEY::default();
        
        unsafe {
            let status = RegCreateKeyW(
                HKEY_CURRENT_USER,
                &HSTRING::from(key_path),
                &mut key,
            );
            
            if status != ERROR_SUCCESS {
                return Err(anyhow!("Failed to create AUMID registry key: {:?}", status));
            }

            let mut display_name_wide: Vec<u16> = self.display_name.encode_utf16().collect();
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
                return Err(anyhow!("Failed to set display name: {:?}", status));
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
                return Err(anyhow!("Failed to set ShowInSettings: {:?}", status));
            }
        }

        Ok(())
    }

    fn register_notification_settings(&self) -> Result<()> {
        let key_path = format!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Notifications\\Settings\\{}", self.app_id);
        let mut key = HKEY::default();
        
        unsafe {
            let status = RegCreateKeyW(
                HKEY_CURRENT_USER,
                &HSTRING::from(key_path),
                &mut key,
            );
            
            if status != ERROR_SUCCESS {
                return Err(anyhow!("Failed to create notification settings key: {:?}", status));
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
                return Err(anyhow!("Failed to set notification settings"));
            }
        }

        Ok(())
    }
}
