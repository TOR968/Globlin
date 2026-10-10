use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::*;

#[cfg(unix)]
mod unix;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;

pub const BACKGROUND_FLAG: &str = "--background";
const TEST_DATA_DIR: &str = "globlin-test-data";

static NOTIFICATION_CLICK: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

pub fn on_notification_click(handler: impl Fn() + Send + Sync + 'static) {
    NOTIFICATION_CLICK.set(Box::new(handler)).ok();
}

fn notification_clicked() {
    if let Some(handler) = NOTIFICATION_CLICK.get() {
        handler();
    }
}

pub fn data_dir() -> PathBuf {
    let dir = if cfg!(test) {
        std::env::temp_dir().join(TEST_DATA_DIR)
    } else {
        user_data_dir()
    };
    fs::create_dir_all(&dir).ok();
    dir
}
