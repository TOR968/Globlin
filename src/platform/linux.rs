use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::Command;

use notify_rust::Notification;

use crate::Result;

const NOT_INSTALLED_BY_SETUP: &str = "Globlin has no setup-managed install on Linux";
const DEFAULT_SHELL: &str = "/bin/sh";
const SHOW_SOCKET: &str = "globlin.sock";
const OWNER_ONLY: u32 = 0o600;
const AUTOSTART_FILE: &str = "globlin.desktop";
const ICON_NAME: &str = "globlin";
const DEFAULT_ACTION: &str = "default";

pub fn prepare_environment() {
    super::unix::adopt_login_path(DEFAULT_SHELL);
}

pub fn claim_single_instance() -> bool {
    super::unix::claim_instance_lock(&super::data_dir())
}

pub fn signal_running_instance() {
    if let Ok(mut stream) = UnixStream::connect(super::data_dir().join(SHOW_SOCKET)) {
        stream.write_all(b"show").ok();
    }
}

pub fn on_show_request(handler: impl Fn() + Send + 'static) {
    let path = super::data_dir().join(SHOW_SOCKET);
    fs::remove_file(&path).ok();
    let Ok(listener) = UnixListener::bind(&path) else {
        return;
    };
    fs::set_permissions(&path, fs::Permissions::from_mode(OWNER_ONLY)).ok();
    std::thread::Builder::new()
        .name("show-request".to_string())
        .spawn(move || {
            for _ in listener.incoming().flatten() {
                handler();
            }
        })
        .ok();
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub(super) fn user_data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".local").join("share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("globlin")
}

pub fn notify(title: &str, body: &str) -> Result<()> {
    let handle = Notification::new()
        .appname("Globlin")
        .summary(title)
        .body(body)
        .icon(ICON_NAME)
        .action(DEFAULT_ACTION, "Open Globlin")
        .show()?;
    std::thread::Builder::new()
        .name("notification".to_string())
        .spawn(move || {
            handle.wait_for_action(|action| {
                if action == DEFAULT_ACTION {
                    super::notification_clicked();
                }
            });
        })?;
    Ok(())
}

pub fn autostart_enabled() -> bool {
    autostart_entry().is_some_and(|path| path.is_file())
}

pub fn set_autostart(enabled: bool) -> Result<()> {
    let path = autostart_entry().ok_or("neither XDG_CONFIG_HOME nor HOME is set")?;
    if enabled {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, autostart_desktop_entry(&std::env::current_exe()?))?;
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
    Command::new("xdg-open").arg(path).spawn()?;
    Ok(())
}

fn config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".config")))
}

fn autostart_entry() -> Option<PathBuf> {
    config_home().map(|config| config.join("autostart").join(AUTOSTART_FILE))
}

fn autostart_desktop_entry(exe: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Globlin\nExec={} {}\nIcon={ICON_NAME}\nTerminal=false\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n",
        desktop_quote(&exe.to_string_lossy()),
        super::BACKGROUND_FLAG
    )
}

fn desktop_quote(argument: &str) -> String {
    let mut quoted = String::from('"');
    for character in argument.chars() {
        if matches!(character, '"' | '`' | '$' | '\\') {
            quoted.push('\\');
        }
        quoted.push(character);
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests;
