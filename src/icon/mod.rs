use std::fs;
use std::path::Path;

use tray_icon::Icon;

use crate::Result;

#[cfg_attr(not(windows), allow(dead_code))]
mod ico;
#[cfg(test)]
mod png;
mod render;

pub use render::{IconState, BUSY_FRAMES};

const TRAY_SIZE: u32 = 32;
const INK_ON_DARK: [u8; 3] = [0xff, 0xff, 0xff];
const INK_ON_LIGHT: [u8; 3] = [0x1f, 0x1f, 0x1f];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Colour,
    Mono([u8; 3]),
}

pub const fn look(monochrome: bool, light_tray: bool) -> Look {
    match (monochrome, light_tray) {
        (false, _) => Look::Colour,
        (true, true) => Look::Mono(INK_ON_LIGHT),
        (true, false) => Look::Mono(INK_ON_DARK),
    }
}

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

pub fn tray(state: IconState, frame: u32, level: f32, look: Look) -> Result<Icon> {
    let pixels = match look {
        Look::Colour => render::rgba(state, frame, level, TRAY_SIZE),
        Look::Mono(ink) => render::mono(state, frame, level, TRAY_SIZE, ink),
    };
    Ok(Icon::from_rgba(pixels, TRAY_SIZE, TRAY_SIZE).map_err(|error| error.to_string())?)
}

pub fn image(state: IconState, size: u32) -> Vec<u8> {
    render::rgba(state, 0, 0.0, size)
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn write_app_icon(path: &Path) -> std::io::Result<()> {
    let images: Vec<(u32, Vec<u8>)> = ico::SIZES
        .iter()
        .map(|size| (*size, render::rgba(IconState::Idle, 0, 0.0, *size)))
        .collect();
    fs::write(path, ico::encode(&images))
}

#[cfg(test)]
mod tests;
