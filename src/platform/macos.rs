use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mac_notification_sys::{Notification, NotificationResponse};

use crate::Result;

const BUNDLE_ID: &str = "dev.globlin.app";
const NOT_INSTALLED_BY_SETUP: &str = "Globlin has no setup-managed install on macOS";
const DEFAULT_SHELL: &str = "/bin/zsh";

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
    launch_agent().is_some_and(|path| path.is_file())
}

pub fn set_autostart(enabled: bool) -> Result<()> {
    let path = launch_agent().ok_or("HOME is not set")?;
    if enabled {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, launch_agent_plist(&std::env::current_exe()?))?;
    } else if path.is_file() {
        fs::remove_file(&path)?;
    }
    Ok(())
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

pub fn open_in_shell(path: &Path) -> Result<()> {
    Command::new("open").arg(path).spawn()?;
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
