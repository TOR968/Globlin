use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tauri_winrt_notification::{Duration as ToastDuration, Toast};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

use crate::Result;

const APP_USER_MODEL_ID: &str = "NpmGlobals.Tray";
const DISPLAY_NAME: &str = "npm globals";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "npm-globals-tray";

pub fn claim_single_instance() -> bool {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    let name: Vec<u16> = r"Local\npm-globals-tray"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let handle = unsafe { CreateMutexW(std::ptr::null(), 1, name.as_ptr()) };
    if handle.is_null() {
        return true;
    }
    unsafe { GetLastError() != ERROR_ALREADY_EXISTS }
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(PathBuf::from)
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("npm-globals-tray");
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
        key.set_value(RUN_VALUE, &format!("\"{}\"", executable.display()))?;
    } else if key.get_value::<String, _>(RUN_VALUE).is_ok() {
        key.delete_value(RUN_VALUE)?;
    }
    Ok(())
}

pub fn open_in_shell(path: &Path) -> Result<()> {
    Command::new("explorer").arg(path).spawn()?;
    Ok(())
}

fn register_app_user_model_id() -> Result<()> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(format!(r"Software\Classes\AppUserModelId\{APP_USER_MODEL_ID}"))?;
    key.set_value("DisplayName", &DISPLAY_NAME.to_string())?;
    Ok(())
}

fn show_toast(app_id: &str, title: &str, body: &str) -> Result<()> {
    Toast::new(app_id)
        .title(title)
        .text1(body)
        .duration(ToastDuration::Short)
        .show()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "writes to the HKCU Run key: cargo test -- --ignored --exact platform::windows::tests::autostart_round_trips_through_the_run_key"]
    fn autostart_round_trips_through_the_run_key() {
        let was_enabled = autostart_enabled();

        set_autostart(true).unwrap();
        assert!(autostart_enabled());

        let value: String = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(RUN_KEY)
            .unwrap()
            .get_value(RUN_VALUE)
            .unwrap();
        assert!(value.contains("npm-globals-tray"), "unexpected value: {value}");

        set_autostart(false).unwrap();
        assert!(!autostart_enabled());

        set_autostart(was_enabled).unwrap();
        assert_eq!(autostart_enabled(), was_enabled);
    }

    #[test]
    fn disabling_autostart_twice_is_not_an_error() {
        if autostart_enabled() {
            return;
        }
        assert!(set_autostart(false).is_ok());
    }

    #[test]
    fn the_data_dir_exists_after_it_is_requested() {
        assert!(data_dir().is_dir());
    }
}
