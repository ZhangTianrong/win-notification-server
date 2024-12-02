use anyhow::Result;
use windows::{
    Win32::System::DataExchange::*,
    Win32::Foundation::*,
    Win32::System::Memory::*,
};

pub struct ClipboardService;

impl ClipboardService {
    pub fn set_text(text: &str) -> Result<()> {
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
}
