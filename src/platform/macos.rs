use std::fs::{self, File, TryLockError};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use mac_notification_sys::{Notification, NotificationResponse};

use crate::Result;

const BUNDLE_ID: &str = "dev.globlin.app";
const NOT_INSTALLED_BY_SETUP: &str = "Globlin has no setup-managed install on macOS";
const DEFAULT_SHELL: &str = "/bin/zsh";
const PATH_MARKER: &str = "__GLOBLIN_PATH__";
const LOGIN_SHELL_TIMEOUT: Duration = Duration::from_secs(5);
const LOGIN_SHELL_POLL: Duration = Duration::from_millis(20);

static INSTANCE_LOCK: OnceLock<File> = OnceLock::new();

pub fn prepare_environment() {
    let Some(login) = login_path() else {
        return;
    };
    let current = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", merge_paths(&login, &current));
}

pub fn claim_single_instance() -> bool {
    let Ok(file) = File::options()
        .create(true)
        .write(true)
        .truncate(false)
        .open(data_dir().join("globlin.lock"))
    else {
        return true;
    };
    match file.try_lock() {
        Ok(()) => {
            INSTANCE_LOCK.set(file).ok();
            true
        }
        Err(TryLockError::WouldBlock) => false,
        Err(TryLockError::Error(_)) => true,
    }
}

pub const fn signal_running_instance() {}

pub fn on_show_request(_handler: impl Fn() + Send + 'static) {}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn data_dir() -> PathBuf {
    let base = home_dir().map_or_else(std::env::temp_dir, |home| {
        home.join("Library").join("Application Support")
    });
    let dir = base.join("globlin");
    fs::create_dir_all(&dir).ok();
    dir
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

fn login_path() -> Option<String> {
    let shell = std::env::var_os("SHELL").unwrap_or_else(|| DEFAULT_SHELL.into());
    let script = format!("printf '\\n{PATH_MARKER}%s{PATH_MARKER}\\n' \"$PATH\"");
    let mut child = Command::new(shell)
        .args(["-i", "-l", "-c", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + LOGIN_SHELL_TIMEOUT;
    while child.try_wait().ok()?.is_none() {
        if Instant::now() > deadline {
            child.kill().ok();
            child.wait().ok();
            return None;
        }
        std::thread::sleep(LOGIN_SHELL_POLL);
    }
    let mut output = String::new();
    child.stdout.take()?.read_to_string(&mut output).ok()?;
    extract_path(&output)
}

fn extract_path(output: &str) -> Option<String> {
    let start = output.find(PATH_MARKER)? + PATH_MARKER.len();
    let length = output[start..].find(PATH_MARKER)?;
    let path = output[start..start + length].trim();
    (!path.is_empty()).then(|| path.to_string())
}

fn merge_paths(login: &str, current: &str) -> String {
    let mut merged: Vec<&str> = Vec::new();
    for entry in login.split(':').chain(current.split(':')) {
        if !entry.is_empty() && !merged.contains(&entry) {
            merged.push(entry);
        }
    }
    merged.join(":")
}

#[cfg(test)]
mod tests;
