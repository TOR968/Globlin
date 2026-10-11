use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use tauri_winrt_notification::{Duration as ToastDuration, Toast};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_SET_VALUE};
use winreg::{RegKey, HKEY};

use crate::icon;
use crate::Result;

const APP_USER_MODEL_ID: &str = "Globlin.Tray";
const DISPLAY_NAME: &str = "Globlin";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "globlin";
const PERSONALIZE_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const LIGHT_TASKBAR_VALUE: &str = "SystemUsesLightTheme";
const USER_ENVIRONMENT: &str = "Environment";
const SYSTEM_ENVIRONMENT: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
const VOLATILE_ENVIRONMENT: &str = "Volatile Environment";
const PROFILE_VARIABLES: [&str; 5] = [
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "HOMEDRIVE",
    "HOMEPATH",
];
const INSTANCE_MUTEX: &str = r"Local\globlin";
const SHOW_EVENT: &str = r"Local\globlin-show";
const SETUP_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{380BD341-81C1-4AC3-AA2E-BC70BE5CC8F7}_is1";

pub fn prepare_environment() {
    restore_profile_variables();
    let registered: Vec<String> = [
        registry_value(HKEY_LOCAL_MACHINE, SYSTEM_ENVIRONMENT, "Path"),
        registry_value(HKEY_CURRENT_USER, USER_ENVIRONMENT, "Path"),
    ]
    .into_iter()
    .flatten()
    .map(|raw| expand_variables(&raw, |name| std::env::var(name).ok()))
    .collect();
    let current = std::env::var("PATH").unwrap_or_default();
    std::env::set_var(
        "PATH",
        append_missing_paths(&current, &registered.join(";")),
    );
}

fn restore_profile_variables() {
    let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(VOLATILE_ENVIRONMENT) else {
        return;
    };
    for name in PROFILE_VARIABLES {
        if std::env::var_os(name).is_some() {
            continue;
        }
        if let Ok(value) = key.get_value::<String, _>(name) {
            std::env::set_var(name, value);
        }
    }
}

fn registry_value(root: HKEY, path: &str, name: &str) -> Option<String> {
    RegKey::predef(root)
        .open_subkey(path)
        .ok()?
        .get_value::<String, _>(name)
        .ok()
}

fn expand_variables(raw: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    let mut expanded = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find('%') {
        expanded.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            expanded.push('%');
            rest = after;
            break;
        };
        let name = &after[..end];
        if let Some(value) = lookup(name).filter(|_| !name.is_empty()) {
            expanded.push_str(&value);
        } else {
            expanded.push('%');
            expanded.push_str(name);
            expanded.push('%');
        }
        rest = &after[end + 1..];
    }
    expanded.push_str(rest);
    expanded
}

fn append_missing_paths(current: &str, registered: &str) -> String {
    let mut merged: Vec<&str> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for entry in current.split(';').chain(registered.split(';')) {
        let entry = entry.trim();
        let key = entry.trim_end_matches(['\\', '/']).to_ascii_lowercase();
        if entry.is_empty() || seen.contains(&key) {
            continue;
        }
        seen.push(key);
        merged.push(entry);
    }
    merged.join(";")
}

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

pub(super) fn user_data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("globlin")
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

pub fn tray_is_light() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(PERSONALIZE_KEY)
        .and_then(|key| key.get_value::<u32, _>(LIGHT_TASKBAR_VALUE))
        .is_ok_and(|light| light != 0)
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

pub fn open_in_shell(target: &OsStr) -> Result<()> {
    Command::new("explorer").arg(target).spawn()?;
    Ok(())
}

fn register_app_user_model_id() -> Result<()> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(format!(
        r"Software\Classes\AppUserModelId\{APP_USER_MODEL_ID}"
    ))?;
    key.set_value("DisplayName", &DISPLAY_NAME.to_string())?;

    let artwork = super::data_dir().join("app.ico");
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
