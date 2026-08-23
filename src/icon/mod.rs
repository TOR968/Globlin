use std::fs;
use std::path::Path;

use tray_icon::Icon;

use crate::Result;

mod ico;
#[cfg(test)]
mod png;
mod render;

pub use render::{IconState, BUSY_FRAMES};

const TRAY_SIZE: u32 = 32;

pub const fn state_for(busy: bool, failed: bool, outdated: usize) -> IconState {
    if busy {
        IconState::Busy
    } else if failed {
        IconState::Error
    } else if outdated == 0 {
        IconState::Idle
    } else {
        IconState::Updates
    }
}

pub fn tray(state: IconState, frame: u32, level: f32) -> Result<Icon> {
    Ok(Icon::from_rgba(
        render::rgba(state, frame, level, TRAY_SIZE),
        TRAY_SIZE,
        TRAY_SIZE,
    )
    .map_err(|error| error.to_string())?)
}

pub fn write_app_icon(path: &Path) -> std::io::Result<()> {
    let images: Vec<(u32, Vec<u8>)> = ico::SIZES
        .iter()
        .map(|size| (*size, render::rgba(IconState::Idle, 0, 0.0, *size)))
        .collect();
    fs::write(path, ico::encode(&images))
}

#[cfg(test)]
mod tests;
