use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tauri_winrt_notification::{Duration as ToastDuration, Toast};
use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
use winreg::RegKey;

use crate::icon;
use crate::Result;

const APP_USER_MODEL_ID: &str = "Globlin.Tray";
const DISPLAY_NAME: &str = "Globlin";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "globlin";
const INSTANCE_MUTEX: &str = r"Local\globlin";
const SHOW_EVENT: &str = r"Local\globlin-show";
const SETUP_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{380BD341-81C1-4AC3-AA2E-BC70BE5CC8F7}_is1";

pub const fn prepare_environment() {}

pub fn claim_single_instance() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    let name = wide(INSTANCE_MUTEX);
    let handle = unsafe { CreateMutexW(std::ptr::null(), 1, name.as_ptr()) };
    if handle.is_null() {
        return true;
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe { CloseHandle(handle) };
        return false;
    }
    true
}

pub fn signal_running_instance() {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenEventW, SetEvent, EVENT_MODIFY_STATE};

    let name = wide(SHOW_EVENT);
    let handle = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, name.as_ptr()) };
    if handle.is_null() {
        return;
    }
    unsafe {
        SetEvent(handle);
        CloseHandle(handle);
    }
}

pub fn on_show_request(handler: impl Fn() + Send + 'static) {
    use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
    use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject, INFINITE};

    let name = wide(SHOW_EVENT);
    let handle = unsafe { CreateEventW(std::ptr::null(), 0, 0, name.as_ptr()) };
    if handle.is_null() {
        return;
    }
    let event = ShowEvent(handle);
    std::thread::Builder::new()
        .name("show-request".to_string())
        .spawn(move || {
            let event = event;
            while unsafe { WaitForSingleObject(event.0, INFINITE) } == WAIT_OBJECT_0 {
                handler();
            }
        })
        .ok();
}

struct ShowEvent(windows_sys::Win32::Foundation::HANDLE);

unsafe impl Send for ShowEvent {}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(PathBuf::from)
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map_or_else(std::env::temp_dir, PathBuf::from);
    let dir = base.join("globlin");
    fs::create_dir_all(&dir).ok();
    dir
}

pub fn notify(title: &str, body: &str) -> Result<()> {
    register_app_user_model_id().ok();
    match show_toast(APP_USER_MODEL_ID, title, body) {
        Ok(()) => Ok(()),
        Err(_) => show_toast(Toast::POWERSHELL_APP_ID, title, body),
    }
}

pub fn autostart_enabled() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .and_then(|key| key.get_value::<String, _>(RUN_VALUE))
        .is_ok()
}

pub fn set_autostart(enabled: bool) -> Result<()> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY)?;
    if enabled {
        let executable = std::env::current_exe()?;
        key.set_value(RUN_VALUE, &run_value(&executable))?;
    } else if key.get_value::<String, _>(RUN_VALUE).is_ok() {
        key.delete_value(RUN_VALUE)?;
    }
    Ok(())
}

fn run_value(executable: &Path) -> String {
    format!("\"{}\" {}", executable.display(), super::BACKGROUND_FLAG)
}

pub fn setup_install_location() -> Option<PathBuf> {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(SETUP_KEY)
        .and_then(|key| key.get_value::<String, _>("InstallLocation"))
        .ok()
        .map(PathBuf::from)
}

pub fn record_installed_version(version: &str) -> Result<()> {
    let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(SETUP_KEY, KEY_SET_VALUE)?;
    key.set_value("DisplayVersion", &version.to_string())?;
    Ok(())
}

pub fn open_in_shell(path: &Path) -> Result<()> {
    Command::new("explorer").arg(path).spawn()?;
    Ok(())
}

fn register_app_user_model_id() -> Result<()> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(format!(
        r"Software\Classes\AppUserModelId\{APP_USER_MODEL_ID}"
    ))?;
    key.set_value("DisplayName", &DISPLAY_NAME.to_string())?;

    let artwork = data_dir().join("app.ico");
    icon::write_app_icon(&artwork).ok();
    if artwork.is_file() {
        key.set_value("IconUri", &artwork.display().to_string())?;
    }
    Ok(())
}

fn show_toast(app_id: &str, title: &str, body: &str) -> Result<()> {
    Toast::new(app_id)
        .title(title)
        .text1(body)
        .duration(ToastDuration::Short)
        .on_activated(|_| {
            super::notification_clicked();
            Ok(())
        })
        .show()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests;
