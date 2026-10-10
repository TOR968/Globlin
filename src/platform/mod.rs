use std::sync::OnceLock;

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::*;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(any(windows, target_os = "macos")))]
mod linux;

#[cfg(not(any(windows, target_os = "macos")))]
pub use linux::*;

pub const BACKGROUND_FLAG: &str = "--background";

static NOTIFICATION_CLICK: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

pub fn on_notification_click(handler: impl Fn() + Send + Sync + 'static) {
    NOTIFICATION_CLICK.set(Box::new(handler)).ok();
}

#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
fn notification_clicked() {
    if let Some(handler) = NOTIFICATION_CLICK.get() {
        handler();
    }
}
