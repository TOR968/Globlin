use std::ffi::{c_char, CStr, OsStr};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mac_notification_sys::{Notification, NotificationResponse};
use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject};

use crate::Result;

const BUNDLE_ID: &str = "dev.globlin.app";
const NOT_INSTALLED_BY_SETUP: &str = "Globlin has no setup-managed install on macOS";
const DEFAULT_SHELL: &str = "/bin/zsh";
const STATUS_ENABLED: isize = 1;
const STATUS_REQUIRES_APPROVAL: isize = 2;
const KEY_AE_PROP_DATA: u32 = 0x7072_6474;
const KEY_AE_LAUNCHED_AS_LOGIN_ITEM: u32 = 0x6C67_6974;

#[link(name = "ServiceManagement", kind = "framework")]
extern "C" {}

pub fn prepare_environment() {
    super::unix::adopt_login_path(DEFAULT_SHELL);
}

pub fn claim_single_instance() -> bool {
    super::unix::claim_instance_lock(&super::data_dir())
}

pub const fn signal_running_instance() {}

pub fn on_show_request(_handler: impl Fn() + Send + 'static) {}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub(super) fn user_data_dir() -> PathBuf {
    home_dir()
        .map_or_else(std::env::temp_dir, |home| {
            home.join("Library").join("Application Support")
        })
        .join("globlin")
}

pub fn notify(title: &str, body: &str) -> Result<()> {
    let title = title.to_string();
    let body = body.to_string();
    std::thread::Builder::new()
        .name("notification".to_string())
        .spawn(move || {
            mac_notification_sys::set_application(BUNDLE_ID).ok();
            let mut options = Notification::new();
            options.wait_for_click(true);
            if let Ok(NotificationResponse::Click) =
                mac_notification_sys::send_notification(&title, None, &body, Some(&options))
            {
                super::notification_clicked();
            }
        })?;
    Ok(())
}

pub fn autostart_enabled() -> bool {
    let login_item = login_item_service().is_some_and(|service| counts_as_enabled(status(service)));
    login_item || launch_agent().is_some_and(|path| path.is_file())
}

pub fn set_autostart(enabled: bool) -> Result<()> {
    if enabled {
        enable_autostart()
    } else {
        disable_autostart()
    }
}

pub fn launched_at_login() -> bool {
    let Some(manager_class) = AnyClass::get(c"NSAppleEventManager") else {
        return false;
    };
    unsafe {
        let manager: *mut AnyObject = msg_send![manager_class, sharedAppleEventManager];
        if manager.is_null() {
            return false;
        }
        let event: *mut AnyObject = msg_send![manager, currentAppleEvent];
        if event.is_null() {
            return false;
        }
        let descriptor: *mut AnyObject =
            msg_send![event, paramDescriptorForKeyword: KEY_AE_PROP_DATA];
        if descriptor.is_null() {
            return false;
        }
        let code: u32 = msg_send![descriptor, enumCodeValue];
        code == KEY_AE_LAUNCHED_AS_LOGIN_ITEM
    }
}

fn enable_autostart() -> Result<()> {
    if register_login_item().is_ok() {
        remove_launch_agent()?;
        return Ok(());
    }
    let path = launch_agent().ok_or("HOME is not set")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, launch_agent_plist(&std::env::current_exe()?))?;
    Ok(())
}

fn disable_autostart() -> Result<()> {
    if let Some(service) = login_item_service() {
        if counts_as_enabled(status(service)) {
            unregister(service)?;
        }
    }
    remove_launch_agent()
}

fn remove_launch_agent() -> Result<()> {
    if let Some(path) = launch_agent().filter(|path| path.is_file()) {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn counts_as_enabled(status: isize) -> bool {
    status == STATUS_ENABLED || status == STATUS_REQUIRES_APPROVAL
}

fn login_item_service() -> Option<*mut AnyObject> {
    let class = AnyClass::get(c"SMAppService")?;
    let service: *mut AnyObject = unsafe { msg_send![class, mainAppService] };
    (!service.is_null()).then_some(service)
}

fn status(service: *mut AnyObject) -> isize {
    unsafe { msg_send![service, status] }
}

fn register_login_item() -> Result<()> {
    let service = login_item_service().ok_or("SMAppService is not available")?;
    if counts_as_enabled(status(service)) {
        return Ok(());
    }
    let mut error: *mut AnyObject = std::ptr::null_mut();
    let registered: bool = unsafe { msg_send![service, registerAndReturnError: &mut error] };
    if registered {
        Ok(())
    } else {
        Err(error_text(error).into())
    }
}

fn unregister(service: *mut AnyObject) -> Result<()> {
    let mut error: *mut AnyObject = std::ptr::null_mut();
    let unregistered: bool = unsafe { msg_send![service, unregisterAndReturnError: &mut error] };
    if unregistered {
        Ok(())
    } else {
        Err(error_text(error).into())
    }
}

fn error_text(error: *mut AnyObject) -> String {
    const UNKNOWN: &str = "unknown login item error";
    if error.is_null() {
        return UNKNOWN.to_string();
    }
    unsafe {
        let description: *mut AnyObject = msg_send![error, localizedDescription];
        if description.is_null() {
            return UNKNOWN.to_string();
        }
        let bytes: *const c_char = msg_send![description, UTF8String];
        if bytes.is_null() {
            return UNKNOWN.to_string();
        }
        CStr::from_ptr(bytes).to_string_lossy().into_owned()
    }
}

pub fn tray_is_light() -> bool {
    false
}

pub fn setup_install_location() -> Option<PathBuf> {
    None
}

pub fn record_installed_version(_version: &str) -> Result<()> {
    Err(NOT_INSTALLED_BY_SETUP.into())
}

pub fn open_in_shell(target: &OsStr) -> Result<()> {
    Command::new("open").arg(target).spawn()?;
    Ok(())
}

fn launch_agent() -> Option<PathBuf> {
    home_dir().map(|home| {
        home.join("Library")
            .join("LaunchAgents")
            .join(format!("{BUNDLE_ID}.plist"))
    })
}

fn launch_agent_plist(exe: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{BUNDLE_ID}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>
"#,
        xml_escape(&exe.to_string_lossy()),
        super::BACKGROUND_FLAG
    )
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests;
