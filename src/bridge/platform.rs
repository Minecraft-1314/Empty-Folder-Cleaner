use std::path::PathBuf;

use serde_json::{json, Value};

use crate::core::error::{AppError, AppResult};
use crate::core::localization;

pub fn choose_directory(language: &str) -> Value {
    let title = localization::resolve(language, localization::KEY_SELECT_FOLDER);
    let mut dialog = rfd::FileDialog::new().set_title(title);
    if let Ok(home) = std::env::var("USERPROFILE") {
        dialog = dialog.set_directory(PathBuf::from(home));
    }

    let path = dialog
        .pick_folder()
        .map(|value| value.to_string_lossy().to_string());
    json!({ "path": path })
}

pub fn open_folder(path: Option<&str>) -> AppResult<Value> {
    let Some(value) = path else {
        return Err(AppError::validation("not_found"));
    };
    if value.trim().is_empty() || !std::path::Path::new(value).is_dir() {
        return Err(AppError::validation("not_found"));
    }

    open_target(value)?;
    Ok(json!({ "opened": true }))
}

pub fn copy_path(path: Option<&str>) -> AppResult<Value> {
    let Some(value) = path else {
        return Err(AppError::validation("path_empty"));
    };
    if value.trim().is_empty() {
        return Err(AppError::validation("path_empty"));
    }

    set_clipboard_text(value)?;
    Ok(json!({ "copied": true }))
}

pub fn open_external(value: &str) -> AppResult<()> {
    let trimmed = value.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(AppError::validation("url_blocked"));
    }

    open_target(trimmed)
}

pub fn is_dark_system_theme() -> bool {
    use windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_READ, REG_DWORD,
    };

    const PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize";
    let subkey: Vec<u16> = PATH.encode_utf16().chain(std::iter::once(0)).collect();
    let mut handle = std::ptr::null_mut();
    let result =
        unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut handle) };
    if result != 0 {
        return result == ERROR_FILE_NOT_FOUND;
    }

    let name: Vec<u16> = "AppsUseLightTheme"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut kind: u32 = 0;
    let mut size: u32 = 4;
    let mut buffer = [0u8; 4];
    let result = unsafe {
        RegQueryValueExW(
            handle,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            buffer.as_mut_ptr(),
            &mut size,
        )
    };
    unsafe {
        RegCloseKey(handle);
    }

    if result != 0 || kind != REG_DWORD {
        return true;
    }

    u32::from_le_bytes(buffer) == 0
}

fn open_target(value: &str) -> AppResult<()> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let operation: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let target: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        return Err(AppError::general("operation_failed"));
    }

    Ok(())
}

fn set_clipboard_text(value: &str) -> AppResult<()> {
    use windows_sys::Win32::Foundation::{GlobalFree, HANDLE};
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
    };
    use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

    let units: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = units.len() * 2;

    unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if handle.is_null() {
            return Err(AppError::general("operation_failed"));
        }

        let target = GlobalLock(handle);
        if target.is_null() {
            GlobalFree(handle);
            return Err(AppError::general("operation_failed"));
        }
        std::ptr::copy_nonoverlapping(units.as_ptr() as *const u8, target as *mut u8, bytes);
        GlobalUnlock(handle);

        if OpenClipboard(std::ptr::null_mut()) == 0 {
            GlobalFree(handle);
            return Err(AppError::general("operation_failed"));
        }

        EmptyClipboard();
        let stored = SetClipboardData(CF_UNICODETEXT as u32, handle as HANDLE);
        CloseClipboard();
        if stored.is_null() {
            GlobalFree(handle);
            return Err(AppError::general("operation_failed"));
        }
    }

    Ok(())
}
